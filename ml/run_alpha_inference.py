#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
from typing import Sequence

import joblib
import pandas as pd
import psycopg

PUBLIC_MARKET_FEATURE_COLUMNS = [
    "btc_ret_1h",
    "btc_range_1h",
    "btc_vol_z24",
    "eth_ret_1h",
    "eth_range_1h",
    "eth_vol_z24",
    "btc_eth_corr_24",
    "market_stress_1h",
]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Run ML alpha inference for latest ranking window.")
    parser.add_argument("--database-url", default=os.getenv("DATABASE_URL"), required=os.getenv("DATABASE_URL") is None)
    parser.add_argument("--model-version", default=None, help="Model version to use; default active model in registry.")
    parser.add_argument("--artifacts-dir", default=str(Path(__file__).resolve().parents[1] / "backend" / "ml_models"))
    parser.add_argument(
        "--public-market-csv",
        default=str(Path(__file__).resolve().parent / "data" / "public_market_hourly.csv"),
        help="Optional external market-regime features CSV.",
    )
    parser.add_argument(
        "--backfill-hours",
        type=int,
        default=0,
        help="If >0, score all ranking windows in the last N hours (not only latest window).",
    )
    parser.add_argument(
        "--window-limit",
        type=int,
        default=0,
        help="Optional max number of windows to process (latest first). 0 means no limit.",
    )
    parser.add_argument(
        "--only-missing",
        action="store_true",
        help="For backfill mode, process only windows that do not yet have score_source='ml' rows.",
    )
    parser.add_argument("--dry-run", action="store_true")
    return parser.parse_args()


def resolve_model_version(conn: psycopg.Connection, explicit: str | None) -> str:
    if explicit:
        return explicit
    with conn.cursor() as cur:
        cur.execute("SELECT model_version FROM ml_model_registry WHERE is_active = true ORDER BY updated_at DESC LIMIT 1")
        row = cur.fetchone()
    if not row:
        raise RuntimeError("No active model in ml_model_registry; provide --model-version.")
    return str(row[0])


def load_window_features(conn: psycopg.Connection, window_end) -> pd.DataFrame:
    sql = """
    SELECT
        ar.window_end,
        ar.token_address,
        COALESCE(rs.composite_score, 50)::double precision AS risk_score,
        COALESCE(ar.alpha_score::double precision, 50.0) AS legacy_alpha_score,
        COALESCE(ar.rank, 50)::double precision AS legacy_rank,
        COALESCE(base.volume, 0)::double precision AS baseline_volume_1h,
        COALESCE(base.buys, 0)::double precision AS baseline_buys_1h,
        COALESCE(base.sells, 0)::double precision AS baseline_sells_1h,
        CASE
            WHEN COALESCE(base.buys, 0) + COALESCE(base.sells, 0) > 0
                THEN COALESCE(base.buys, 0)::double precision /
                     (COALESCE(base.buys, 0) + COALESCE(base.sells, 0))
            ELSE 0.5
        END AS buy_share_1h
    FROM alpha_rankings ar
    LEFT JOIN risk_scores rs
      ON rs.token_address = ar.token_address
    LEFT JOIN LATERAL (
        SELECT
            SUM(tt.amount_bnb) AS volume,
            COUNT(*) FILTER (WHERE tt.tx_type = 'buy') AS buys,
            COUNT(*) FILTER (WHERE tt.tx_type = 'sell') AS sells
        FROM token_transactions tt
        WHERE tt.token_address = ar.token_address
          AND tt.created_at > ar.window_end - INTERVAL '1 hour'
          AND tt.created_at <= ar.window_end
    ) base ON TRUE
    WHERE ar.window_end = %s
    ORDER BY ar.rank ASC
    """
    return pd.read_sql_query(sql, conn, params=(window_end,))


def attach_public_market_features(frame: pd.DataFrame, window_end, csv_path: str) -> pd.DataFrame:
    if frame.empty:
        return frame

    path = Path(csv_path)
    if not path.exists():
        out = frame.copy()
        for col in PUBLIC_MARKET_FEATURE_COLUMNS:
            if col not in out.columns:
                out[col] = 0.0
        return out

    market_df = pd.read_csv(path, parse_dates=["window_hour_utc"])
    if market_df.empty:
        out = frame.copy()
        for col in PUBLIC_MARKET_FEATURE_COLUMNS:
            if col not in out.columns:
                out[col] = 0.0
        return out

    ts = pd.Timestamp(window_end)
    wh = (ts.tz_localize("UTC") if ts.tzinfo is None else ts.tz_convert("UTC")).floor("h")
    market_df["window_hour_utc"] = pd.to_datetime(market_df["window_hour_utc"], utc=True)
    row = market_df.loc[market_df["window_hour_utc"] == wh]

    out = frame.copy()
    if row.empty:
        for col in PUBLIC_MARKET_FEATURE_COLUMNS:
            if col not in out.columns:
                out[col] = 0.0
        return out

    first = row.iloc[0]
    for col in PUBLIC_MARKET_FEATURE_COLUMNS:
        out[col] = float(first[col]) if col in first and pd.notna(first[col]) else 0.0
    return out


def positive_proba(model, x: pd.DataFrame) -> pd.Series:
    probs = model.predict_proba(x)
    if probs.ndim != 2:
        return pd.Series(probs, dtype=float)
    if probs.shape[1] == 1:
        classes = getattr(model, "classes_", [0])
        cls = int(classes[0]) if len(classes) > 0 else 0
        value = 1.0 if cls == 1 else 0.0
        return pd.Series([value] * len(x), dtype=float)

    classes = list(getattr(model, "classes_", [0, 1]))
    if 1 in classes:
        idx = classes.index(1)
        return pd.Series(probs[:, idx], dtype=float)
    return pd.Series(probs[:, -1], dtype=float)


def resolve_probs(artifact: dict, x: pd.DataFrame) -> pd.Series:
    model = artifact["model"]
    calibrator = artifact.get("calibrator")
    logreg = artifact.get("logreg_baseline")
    scorer = str(artifact.get("scorer", "lightgbm_calibrated"))
    scorer_params = artifact.get("scorer_params", {}) or {}

    if scorer == "blend_logreg_lgbm" and logreg is not None:
        lw = float(scorer_params.get("logreg_weight", 0.7))
        mw = float(scorer_params.get("lgbm_weight", 0.3))
        lgbm_probs = positive_proba(model, x)
        log_probs = positive_proba(logreg, x)
        blend = (lw * log_probs) + (mw * lgbm_probs)
        return blend.clip(0.0, 1.0)
    if scorer == "logreg_baseline" and logreg is not None:
        return positive_proba(logreg, x)
    if scorer == "lightgbm" or calibrator is None:
        return positive_proba(model, x)
    return positive_proba(calibrator, x)


def resolve_target_windows(
    conn: psycopg.Connection, backfill_hours: int, window_limit: int, only_missing: bool
) -> list:
    with conn.cursor() as cur:
        if backfill_hours <= 0:
            cur.execute("SELECT MAX(window_end) FROM alpha_rankings")
            row = cur.fetchone()
            return [row[0]] if row and row[0] is not None else []

        sql = """
        SELECT DISTINCT ar.window_end
        FROM alpha_rankings ar
        WHERE ar.window_end >= NOW() - (%s || ' hours')::interval
        """
        params = [backfill_hours]
        if only_missing:
            sql += """
              AND NOT EXISTS (
                  SELECT 1
                  FROM ml_alpha_predictions p
                  WHERE p.window_end = ar.window_end
                    AND p.score_source = 'ml'
              )
            """
        sql += " ORDER BY ar.window_end DESC"
        if window_limit > 0:
            sql += " LIMIT %s"
            params.append(window_limit)

        cur.execute(sql, tuple(params))
        rows = cur.fetchall()
        return [r[0] for r in rows]


def upsert_predictions(
    conn: psycopg.Connection,
    frame: pd.DataFrame,
    model_version: str,
) -> None:
    if frame.empty:
        return
    with conn.cursor() as cur:
        cur.execute(
            """
            SELECT COALESCE(
                (SELECT rollout_mode FROM ml_model_registry WHERE model_version = %s),
                'shadow'
            )
            """,
            (model_version,),
        )
        rollout_mode = str(cur.fetchone()[0])

        rows = [
            (
                r["window_end"],
                r["token_address"],
                "ml",
                model_version,
                rollout_mode,
                float(r["score"]),
                float(r["confidence"]),
            )
            for _, r in frame.iterrows()
        ]
        cur.executemany(
            """
            INSERT INTO ml_alpha_predictions
                (window_end, token_address, score_source, model_version, rollout_mode, score, confidence)
            VALUES (%s, %s, %s, %s, %s, %s, %s)
            ON CONFLICT (window_end, token_address, score_source) DO UPDATE SET
                model_version = EXCLUDED.model_version,
                rollout_mode = EXCLUDED.rollout_mode,
                score = EXCLUDED.score,
                confidence = EXCLUDED.confidence
            """,
            rows,
        )
    conn.commit()


def main() -> None:
    args = parse_args()
    artifacts_dir = Path(args.artifacts_dir)

    with psycopg.connect(args.database_url) as conn:
        model_version = resolve_model_version(conn, args.model_version)
        artifact_path = artifacts_dir / f"{model_version}.joblib"
        if not artifact_path.exists():
            raise RuntimeError(f"Model artifact not found: {artifact_path}")

        artifact = joblib.load(artifact_path)
        feature_columns: Sequence[str] = artifact["feature_columns"]
        target_windows = resolve_target_windows(
            conn,
            backfill_hours=args.backfill_hours,
            window_limit=args.window_limit,
            only_missing=args.only_missing,
        )
        if not target_windows:
            print("No windows found to score.")
            return

        total_rows = 0
        processed = []
        for window_end in target_windows:
            frame = load_window_features(conn, window_end)
            if frame.empty:
                continue
            frame = attach_public_market_features(frame, window_end, args.public_market_csv)

            for feature in feature_columns:
                if feature not in frame.columns:
                    frame[feature] = 0.0

            probs = resolve_probs(artifact, frame[list(feature_columns)])
            frame = frame.copy()
            frame["score"] = (probs * 100.0).clip(0.0, 100.0)
            frame["confidence"] = probs.apply(lambda p: max(float(p), 1.0 - float(p))).clip(0.0, 1.0)

            if args.dry_run:
                preview = (
                    frame[["token_address", "score", "confidence"]]
                    .sort_values("score", ascending=False)
                    .head(10)
                )
                print(f"=== {window_end} ===")
                print(preview.to_string(index=False))
                continue

            upsert_predictions(conn, frame, model_version)
            row_count = int(len(frame))
            total_rows += row_count
            processed.append({"window_end": str(window_end), "rows": row_count})

        if args.dry_run:
            return

        print(
            json.dumps(
                {
                    "model_version": model_version,
                    "windows_processed": len(processed),
                    "predictions_written": total_rows,
                    "latest_window": processed[0]["window_end"] if processed else None,
                },
                indent=2,
            )
        )


if __name__ == "__main__":
    main()
