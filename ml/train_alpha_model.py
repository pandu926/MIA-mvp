#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import os
from dataclasses import dataclass, asdict
from datetime import datetime, timezone
from pathlib import Path
from typing import Sequence

import joblib
import numpy as np
import pandas as pd
import psycopg
from lightgbm import LGBMClassifier
from sklearn.calibration import CalibratedClassifierCV
from sklearn.dummy import DummyClassifier
from sklearn.linear_model import LogisticRegression
from sklearn.metrics import average_precision_score, log_loss, roc_auc_score

BASE_FEATURE_COLUMNS = [
    "risk_score",
    "legacy_alpha_score",
    "legacy_rank",
    "baseline_volume_1h",
    "baseline_buys_1h",
    "baseline_sells_1h",
    "buy_share_1h",
]

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


@dataclass
class EvalMetrics:
    train_rows: int
    test_rows: int
    auc: float
    logloss: float
    hit_rate_at_10: float
    positive_rate_test: float
    pr_auc: float


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Train LightGBM alpha ranking model with walk-forward + calibration.")
    parser.add_argument("--database-url", default=os.getenv("DATABASE_URL"))
    parser.add_argument("--input-csv", default=None, help="Path to pre-exported training CSV")
    parser.add_argument("--lookback-hours", type=int, default=24 * 14)
    parser.add_argument("--model-version", default=f"lightgbm-{datetime.now(timezone.utc).strftime('%Y%m%d-%H%M%S')}")
    parser.add_argument("--output-dir", default=str(Path(__file__).resolve().parents[1] / "backend" / "ml_models"))
    parser.add_argument(
        "--public-market-csv",
        default=str(Path(__file__).resolve().parent / "data" / "public_market_hourly.csv"),
        help="Optional external market-regime features CSV.",
    )
    parser.add_argument(
        "--disable-public-features",
        action="store_true",
        help="Disable external public market features even if CSV exists.",
    )
    parser.add_argument("--activate", action="store_true", help="Set trained model as active in ml_model_registry")
    parser.add_argument("--rollout-mode", default="shadow", choices=["legacy", "shadow", "ml", "hybrid"])
    parser.add_argument("--walk-forward-folds", type=int, default=4)
    return parser.parse_args()


def load_training_data(conn: psycopg.Connection, lookback_hours: int) -> pd.DataFrame:
    sql = """
    WITH labels AS (
        SELECT
            p.window_end,
            p.token_address,
            MAX(p.realized_hit_1h::int) AS label
        FROM ml_alpha_predictions p
        WHERE p.score_source = 'legacy'
          AND p.realized_hit_1h IS NOT NULL
          AND p.window_end >= NOW() - (%s || ' hours')::interval
        GROUP BY p.window_end, p.token_address
    )
    SELECT
        l.window_end,
        l.token_address,
        l.label,
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
    FROM labels l
    LEFT JOIN alpha_rankings ar
      ON ar.window_end = l.window_end
     AND ar.token_address = l.token_address
    LEFT JOIN risk_scores rs
      ON rs.token_address = l.token_address
    LEFT JOIN LATERAL (
        SELECT
            SUM(tt.amount_bnb) AS volume,
            COUNT(*) FILTER (WHERE tt.tx_type = 'buy') AS buys,
            COUNT(*) FILTER (WHERE tt.tx_type = 'sell') AS sells
        FROM token_transactions tt
        WHERE tt.token_address = l.token_address
          AND tt.created_at > l.window_end - INTERVAL '1 hour'
          AND tt.created_at <= l.window_end
    ) base ON TRUE
    ORDER BY l.window_end ASC, l.token_address ASC
    """
    return pd.read_sql_query(sql, conn, params=(lookback_hours,))


def add_public_market_features(df: pd.DataFrame, csv_path: str, enabled: bool) -> tuple[pd.DataFrame, list[str], dict]:
    if not enabled:
        return df, [], {"enabled": False, "reason": "disabled_by_flag"}

    path = Path(csv_path)
    if not path.exists():
        return df, [], {"enabled": False, "reason": "csv_not_found", "path": str(path)}

    market_df = pd.read_csv(path, parse_dates=["window_hour_utc"])
    if market_df.empty:
        return df, [], {"enabled": False, "reason": "csv_empty", "path": str(path)}

    base = df.copy()
    base["window_hour_utc"] = pd.to_datetime(base["window_end"], utc=True).dt.floor("h")
    market_df["window_hour_utc"] = pd.to_datetime(market_df["window_hour_utc"], utc=True)
    merged = base.merge(market_df, on="window_hour_utc", how="left")

    usable_cols: list[str] = []
    for col in PUBLIC_MARKET_FEATURE_COLUMNS:
        if col not in merged.columns:
            continue
        missing_ratio = float(merged[col].isna().mean())
        if missing_ratio > 0.9:
            continue
        merged[col] = merged[col].fillna(0.0)
        usable_cols.append(col)

    merged = merged.drop(columns=["window_hour_utc"])
    info = {
        "enabled": True,
        "path": str(path),
        "usable_feature_count": len(usable_cols),
        "usable_features": usable_cols,
    }
    return merged, usable_cols, info


def make_lgbm(scale_pos_weight: float) -> LGBMClassifier:
    return LGBMClassifier(
        n_estimators=220,
        learning_rate=0.03,
        max_depth=-1,
        num_leaves=63,
        min_child_samples=8,
        subsample=0.85,
        colsample_bytree=0.85,
        random_state=42,
        objective="binary",
        scale_pos_weight=scale_pos_weight,
        reg_alpha=0.2,
        reg_lambda=1.0,
        verbose=-1,
    )


def temporal_split(df: pd.DataFrame) -> tuple[pd.DataFrame, pd.DataFrame]:
    windows = sorted(df["window_end"].unique())
    if len(windows) < 4:
        raise RuntimeError("Need at least 4 windows for train/test split.")
    split_idx = int(len(windows) * 0.8)
    split_idx = min(max(split_idx, 1), len(windows) - 1)
    cutoff = windows[split_idx]
    train_df = df[df["window_end"] < cutoff].copy()
    test_df = df[df["window_end"] >= cutoff].copy()
    if train_df.empty or test_df.empty:
        raise RuntimeError("Temporal split produced empty set.")
    return train_df, test_df


def hit_rate_at_k(test_df: pd.DataFrame, probs: np.ndarray, k: int = 10) -> float:
    scored = test_df[["window_end", "label"]].copy()
    scored["proba"] = probs
    hits = []
    for _, group in scored.groupby("window_end"):
        top = group.sort_values("proba", ascending=False).head(k)
        if len(top) == 0:
            continue
        hits.append(float(top["label"].mean()))
    return float(np.mean(hits) * 100.0) if hits else 0.0


def evaluate_probs(train_df: pd.DataFrame, test_df: pd.DataFrame, probs: np.ndarray) -> EvalMetrics:
    y_test = test_df["label"].astype(int)
    try:
        auc = float(roc_auc_score(y_test, probs))
    except ValueError:
        auc = 0.5
    try:
        ll = float(log_loss(y_test, probs, labels=[0, 1]))
    except ValueError:
        ll = 0.0
    try:
        pr_auc = float(average_precision_score(y_test, probs))
    except ValueError:
        pr_auc = 0.0
    return EvalMetrics(
        train_rows=len(train_df),
        test_rows=len(test_df),
        auc=auc,
        logloss=ll,
        hit_rate_at_10=hit_rate_at_k(test_df, probs, k=10),
        positive_rate_test=float(y_test.mean() * 100.0),
        pr_auc=pr_auc,
    )


def positive_proba(model, x: pd.DataFrame) -> np.ndarray:
    probs = model.predict_proba(x)
    if probs.ndim != 2:
        return np.asarray(probs, dtype=float).reshape(-1)
    if probs.shape[1] == 1:
        classes = getattr(model, "classes_", np.array([0]))
        cls = int(classes[0]) if len(classes) > 0 else 0
        return np.ones(len(x), dtype=float) if cls == 1 else np.zeros(len(x), dtype=float)

    classes = getattr(model, "classes_", np.array([0, 1]))
    if 1 in list(classes):
        idx = int(np.where(classes == 1)[0][0])
        return probs[:, idx]
    return probs[:, -1]


def walk_forward_eval(df: pd.DataFrame, folds: int, feature_columns: Sequence[str]) -> dict:
    windows = sorted(df["window_end"].unique())
    if len(windows) < max(6, folds + 2):
        return {"folds": [], "summary": {"auc": None, "logloss": None, "hit_rate_at_10": None}}

    segments = np.array_split(np.array(windows), folds + 1)
    fold_metrics = []
    for i in range(1, len(segments)):
        train_windows = np.concatenate(segments[:i])
        test_windows = segments[i]
        train_df = df[df["window_end"].isin(train_windows)].copy()
        test_df = df[df["window_end"].isin(test_windows)].copy()
        if train_df.empty or test_df.empty:
            continue
        x_train = train_df[list(feature_columns)]
        y_train = train_df["label"].astype(int)
        x_test = test_df[list(feature_columns)]

        pos = int(y_train.sum())
        neg = int(len(y_train) - pos)
        scale_pos_weight = float(max(1.0, neg / max(1, pos)))
        model = make_lgbm(scale_pos_weight)
        model.fit(x_train, y_train)
        probs = positive_proba(model, x_test)
        m = evaluate_probs(train_df, test_df, probs)
        fold_metrics.append(
            {
                "fold": i,
                "train_rows": m.train_rows,
                "test_rows": m.test_rows,
                "auc": round(m.auc, 6),
                "pr_auc": round(m.pr_auc, 6),
                "logloss": round(m.logloss, 6),
                "hit_rate_at_10": round(m.hit_rate_at_10, 4),
            }
        )

    if not fold_metrics:
        return {"folds": [], "summary": {"auc": None, "logloss": None, "hit_rate_at_10": None}}

    return {
        "folds": fold_metrics,
        "summary": {
            "auc": round(float(np.mean([f["auc"] for f in fold_metrics])), 6),
            "pr_auc": round(float(np.mean([f["pr_auc"] for f in fold_metrics])), 6),
            "logloss": round(float(np.mean([f["logloss"] for f in fold_metrics])), 6),
            "hit_rate_at_10": round(float(np.mean([f["hit_rate_at_10"] for f in fold_metrics])), 4),
        },
    }


def fit_main_models(train_df: pd.DataFrame, feature_columns: Sequence[str]):
    x_train = train_df[list(feature_columns)]
    y_train = train_df["label"].astype(int)

    if y_train.nunique() < 2:
        constant = int(y_train.iloc[0]) if len(y_train) > 0 else 0
        dummy = DummyClassifier(strategy="constant", constant=constant)
        dummy.fit(x_train, y_train)
        # Keep interfaces compatible: model + baseline both implement predict_proba.
        return dummy, dummy, None, {"fallback": "single_class", "constant_class": constant}

    pos = int(y_train.sum())
    neg = int(len(y_train) - pos)
    scale_pos_weight = float(max(1.0, neg / max(1, pos)))

    lgbm = make_lgbm(scale_pos_weight)
    lgbm.fit(x_train, y_train)

    logreg = LogisticRegression(max_iter=2000, class_weight="balanced")
    logreg.fit(x_train, y_train)

    calibrator = CalibratedClassifierCV(estimator=lgbm, method="isotonic", cv="prefit")
    calibrator.fit(x_train, y_train)

    return lgbm, logreg, calibrator, {"fallback": None, "scale_pos_weight": scale_pos_weight}


def choose_scorer(
    lgbm_metrics: EvalMetrics, cal_metrics: EvalMetrics, logreg_metrics: EvalMetrics
) -> tuple[str, dict]:
    candidates = {
        "lightgbm": lgbm_metrics,
        "lightgbm_calibrated": cal_metrics,
        "logreg_baseline": logreg_metrics,
    }
    # Prioritize top-k hit-rate, then PR-AUC for rare-event ranking quality.
    best_name = max(
        candidates.keys(),
        key=lambda name: (
            candidates[name].hit_rate_at_10,
            candidates[name].pr_auc,
            candidates[name].auc,
            -candidates[name].logloss,
        ),
    )

    # Ensemble if logreg is materially stronger than lgbm on PR-AUC.
    if (
        logreg_metrics.pr_auc > lgbm_metrics.pr_auc + 0.05
        and logreg_metrics.hit_rate_at_10 >= lgbm_metrics.hit_rate_at_10
    ):
        return "blend_logreg_lgbm", {"logreg_weight": 0.7, "lgbm_weight": 0.3}
    return best_name, {}


def upsert_registry(conn: psycopg.Connection, model_version: str, rollout_mode: str, activate: bool, metadata: dict) -> None:
    with conn.cursor() as cur:
        if activate:
            cur.execute("UPDATE ml_model_registry SET is_active = false WHERE is_active = true")
        cur.execute(
            """
            INSERT INTO ml_model_registry (model_version, rollout_mode, is_active, metadata, updated_at)
            VALUES (%s, %s, %s, %s::jsonb, NOW())
            ON CONFLICT (model_version) DO UPDATE SET
                rollout_mode = EXCLUDED.rollout_mode,
                is_active = EXCLUDED.is_active,
                metadata = EXCLUDED.metadata,
                updated_at = NOW()
            """,
            (model_version, rollout_mode, activate, json.dumps(metadata)),
        )
    conn.commit()


def main() -> None:
    args = parse_args()
    output_dir = Path(args.output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)

    if args.input_csv:
        df = pd.read_csv(args.input_csv, parse_dates=["window_end"])
    else:
        if not args.database_url:
            raise RuntimeError("Provide --database-url or --input-csv.")
        with psycopg.connect(args.database_url) as conn:
            df = load_training_data(conn, args.lookback_hours)
    if df.empty:
        raise RuntimeError("No training data found.")

    df, public_cols, public_info = add_public_market_features(
        df, args.public_market_csv, enabled=(not args.disable_public_features)
    )
    feature_columns = list(BASE_FEATURE_COLUMNS) + public_cols

    train_df, test_df = temporal_split(df)
    walk_forward = walk_forward_eval(df, args.walk_forward_folds, feature_columns)
    lgbm, logreg, calibrator, fit_info = fit_main_models(train_df, feature_columns)

    x_test = test_df[feature_columns]
    lgbm_probs = positive_proba(lgbm, x_test)
    lgbm_cal_probs = positive_proba(calibrator, x_test) if calibrator is not None else lgbm_probs
    logreg_probs = positive_proba(logreg, x_test)

    lgbm_metrics = evaluate_probs(train_df, test_df, lgbm_probs)
    cal_metrics = evaluate_probs(train_df, test_df, lgbm_cal_probs)
    logreg_metrics = evaluate_probs(train_df, test_df, logreg_probs)
    scorer, scorer_params = choose_scorer(lgbm_metrics, cal_metrics, logreg_metrics)

    artifact_path = output_dir / f"{args.model_version}.joblib"
    metadata_path = output_dir / f"{args.model_version}.metadata.json"

    artifact = {
        "model_type": "lightgbm_binary_classifier",
        "model_version": args.model_version,
        "feature_columns": feature_columns,
        "trained_at_utc": datetime.now(timezone.utc).isoformat(),
        "lookback_hours": args.lookback_hours,
        "model": lgbm,
        "calibrator": calibrator,
        "logreg_baseline": logreg,
        "scorer": scorer,
        "scorer_params": scorer_params,
    }
    joblib.dump(artifact, artifact_path)

    metadata = {
        "model_version": args.model_version,
        "artifact_path": str(artifact_path),
        "feature_columns": feature_columns,
        "lookback_hours": args.lookback_hours,
        "public_market_features": public_info,
        "fit_info": fit_info,
        "selected_scorer": scorer,
        "scorer_params": scorer_params,
        "walk_forward": walk_forward,
        "metrics": {
            "lightgbm": asdict(lgbm_metrics),
            "lightgbm_calibrated": asdict(cal_metrics),
            "logreg_baseline": asdict(logreg_metrics),
        },
    }
    metadata_path.write_text(json.dumps(metadata, indent=2), encoding="utf-8")

    if args.activate:
        if not args.database_url:
            raise RuntimeError("--activate requires --database-url")
        with psycopg.connect(args.database_url) as conn:
            upsert_registry(conn, args.model_version, args.rollout_mode, args.activate, metadata)

    print(f"Model trained: {args.model_version}")
    print(f"Artifact: {artifact_path}")
    print(f"Metadata: {metadata_path}")
    print(
        "Metrics: "
        f"lgbm_auc={lgbm_metrics.auc:.4f}, "
        f"cal_auc={cal_metrics.auc:.4f}, "
        f"logreg_auc={logreg_metrics.auc:.4f}, "
        f"cal_hit@10={cal_metrics.hit_rate_at_10:.2f}%, "
        f"selected={scorer}"
    )


if __name__ == "__main__":
    main()
