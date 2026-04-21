from __future__ import annotations

import json
from typing import Iterable

import pandas as pd
import psycopg

from .common import FEATURE_COLUMNS


TRAINING_SQL = """
WITH base_windows AS (
    SELECT
        ar.window_end,
        ar.token_address,
        COALESCE(rs.composite_score, 50)::double precision AS risk_score,
        COALESCE(ar.alpha_score::double precision, 50.0) AS legacy_alpha_score,
        COALESCE(ar.rank, 50)::double precision AS legacy_rank,
        GREATEST(EXTRACT(EPOCH FROM (ar.window_end - t.deployed_at)) / 60.0, 0)::double precision AS token_age_minutes,
        COALESCE(t.initial_liquidity_bnb::double precision, 0.0) AS initial_liquidity_bnb,
        COALESCE(base.volume, 0)::double precision AS baseline_volume_1h,
        COALESCE(base.buys, 0)::double precision AS baseline_buys_1h,
        COALESCE(base.sells, 0)::double precision AS baseline_sells_1h,
        CASE
            WHEN COALESCE(base.buys, 0) + COALESCE(base.sells, 0) > 0
                THEN COALESCE(base.buys, 0)::double precision /
                     (COALESCE(base.buys, 0) + COALESCE(base.sells, 0))
            ELSE 0.5
        END AS buy_share_1h,
        COALESCE(base.active_wallets, 0)::double precision AS active_wallets_1h,
        COALESCE(base.tx_count, 0)::double precision AS tx_count_1h,
        COALESCE(wh.watch_alerts, 0)::double precision AS whale_watch_24h,
        COALESCE(wh.critical_alerts, 0)::double precision AS whale_critical_24h,
        COALESCE(hist.deployer_total_tokens, 0)::double precision AS deployer_total_tokens,
        COALESCE(hist.deployer_rug_count, 0)::double precision AS deployer_rug_count,
        COALESCE(hist.deployer_graduated_count, 0)::double precision AS deployer_graduated_count,
        COALESCE(cluster.probable_cluster_wallets, 0)::double precision AS probable_cluster_wallets,
        COALESCE(cluster.potential_cluster_wallets, 0)::double precision AS potential_cluster_wallets
    FROM alpha_rankings ar
    JOIN tokens t
      ON LOWER(t.contract_address) = LOWER(ar.token_address)
    LEFT JOIN risk_scores rs
      ON LOWER(rs.token_address) = LOWER(ar.token_address)
    LEFT JOIN LATERAL (
        SELECT
            SUM(tt.amount_bnb) AS volume,
            COUNT(*) FILTER (WHERE tt.tx_type = 'buy') AS buys,
            COUNT(*) FILTER (WHERE tt.tx_type = 'sell') AS sells,
            COUNT(*) AS tx_count,
            COUNT(DISTINCT tt.wallet_address) AS active_wallets
        FROM token_transactions tt
        WHERE LOWER(tt.token_address) = LOWER(ar.token_address)
          AND tt.created_at > ar.window_end - INTERVAL '1 hour'
          AND tt.created_at <= ar.window_end
    ) base ON TRUE
    LEFT JOIN LATERAL (
        SELECT
            COUNT(*) FILTER (WHERE wa.alert_level = 'watch') AS watch_alerts,
            COUNT(*) FILTER (WHERE wa.alert_level = 'critical') AS critical_alerts
        FROM whale_alerts wa
        WHERE LOWER(wa.token_address) = LOWER(ar.token_address)
          AND wa.created_at > ar.window_end - INTERVAL '24 hours'
          AND wa.created_at <= ar.window_end
    ) wh ON TRUE
    LEFT JOIN LATERAL (
        SELECT
            COUNT(*) AS deployer_total_tokens,
            COUNT(*) FILTER (WHERE history.is_rug) AS deployer_rug_count,
            COUNT(*) FILTER (WHERE history.graduated) AS deployer_graduated_count
        FROM tokens history
        WHERE LOWER(history.deployer_address) = LOWER(t.deployer_address)
          AND history.deployed_at <= ar.window_end
    ) hist ON TRUE
    LEFT JOIN LATERAL (
        SELECT
            COUNT(*) FILTER (WHERE confidence = 'probable') AS probable_cluster_wallets,
            COUNT(*) FILTER (WHERE confidence = 'potential') AS potential_cluster_wallets
        FROM wallet_clusters wc
        WHERE LOWER(wc.token_address) = LOWER(ar.token_address)
    ) cluster ON TRUE
    WHERE ar.window_end >= NOW() - (%s || ' hours')::interval
),
future_window AS (
    SELECT
        bw.window_end,
        bw.token_address,
        COALESCE(fut.volume, 0)::double precision AS future_volume,
        COALESCE(fut.buys, 0)::double precision AS future_buys,
        COALESCE(fut.sells, 0)::double precision AS future_sells,
        COALESCE(fut.tx_count, 0)::double precision AS future_tx_count
    FROM base_windows bw
    LEFT JOIN LATERAL (
        SELECT
            SUM(tt.amount_bnb) AS volume,
            COUNT(*) FILTER (WHERE tt.tx_type = 'buy') AS buys,
            COUNT(*) FILTER (WHERE tt.tx_type = 'sell') AS sells,
            COUNT(*) AS tx_count
        FROM token_transactions tt
        WHERE LOWER(tt.token_address) = LOWER(bw.token_address)
          AND tt.created_at > bw.window_end
          AND tt.created_at <= bw.window_end + (%s || ' hours')::interval
    ) fut ON TRUE
)
SELECT bw.*, fw.future_volume, fw.future_buys, fw.future_sells, fw.future_tx_count
FROM base_windows bw
JOIN future_window fw
  ON fw.window_end = bw.window_end
 AND LOWER(fw.token_address) = LOWER(bw.token_address)
ORDER BY bw.window_end ASC, bw.token_address ASC
"""

HISTORICAL_TRAINING_SQL = """
WITH token_totals AS (
    SELECT
        LOWER(tt.token_address) AS token_address,
        COUNT(*)::bigint AS total_tx_count
    FROM token_transactions tt
    GROUP BY LOWER(tt.token_address)
),
eligible_tokens AS (
    SELECT
        LOWER(t.contract_address) AS token_address,
        t.deployed_at,
        COALESCE(t.initial_liquidity_bnb::double precision, 0.0) AS initial_liquidity_bnb,
        LOWER(t.deployer_address) AS deployer_address,
        COALESCE(tot.total_tx_count, 0)::double precision AS total_tx_count
    FROM tokens t
    LEFT JOIN token_totals tot
      ON tot.token_address = LOWER(t.contract_address)
    WHERE t.deployed_at >= %s
      AND t.deployed_at < %s
      AND COALESCE(tot.total_tx_count, 0) >= %s
),
base_windows AS (
    SELECT
        et.deployed_at + INTERVAL '1 hour' AS window_end,
        et.token_address,
        COALESCE(rs.composite_score, 50)::double precision AS risk_score,
        (
            ((100 - COALESCE(rs.composite_score, 50))::double precision * 0.50) +
            (LEAST(COALESCE(base.buys, 0), 300)::double precision * 0.30) +
            (LEAST(COALESCE(base.volume, 0), 50)::double precision * 0.20)
        ) AS legacy_alpha_score,
        50.0::double precision AS legacy_rank,
        60.0::double precision AS token_age_minutes,
        et.initial_liquidity_bnb,
        COALESCE(base.volume, 0)::double precision AS baseline_volume_1h,
        COALESCE(base.buys, 0)::double precision AS baseline_buys_1h,
        COALESCE(base.sells, 0)::double precision AS baseline_sells_1h,
        CASE
            WHEN COALESCE(base.buys, 0) + COALESCE(base.sells, 0) > 0
                THEN COALESCE(base.buys, 0)::double precision /
                     (COALESCE(base.buys, 0) + COALESCE(base.sells, 0))
            ELSE 0.5
        END AS buy_share_1h,
        COALESCE(base.active_wallets, 0)::double precision AS active_wallets_1h,
        COALESCE(base.tx_count, 0)::double precision AS tx_count_1h,
        COALESCE(wh.watch_alerts, 0)::double precision AS whale_watch_24h,
        COALESCE(wh.critical_alerts, 0)::double precision AS whale_critical_24h,
        COALESCE(hist.deployer_total_tokens, 0)::double precision AS deployer_total_tokens,
        COALESCE(hist.deployer_rug_count, 0)::double precision AS deployer_rug_count,
        COALESCE(hist.deployer_graduated_count, 0)::double precision AS deployer_graduated_count,
        COALESCE(cluster.probable_cluster_wallets, 0)::double precision AS probable_cluster_wallets,
        COALESCE(cluster.potential_cluster_wallets, 0)::double precision AS potential_cluster_wallets
    FROM eligible_tokens et
    LEFT JOIN risk_scores rs
      ON LOWER(rs.token_address) = et.token_address
    LEFT JOIN LATERAL (
        SELECT
            SUM(tt.amount_bnb) AS volume,
            COUNT(*) FILTER (WHERE tt.tx_type = 'buy') AS buys,
            COUNT(*) FILTER (WHERE tt.tx_type = 'sell') AS sells,
            COUNT(*) AS tx_count,
            COUNT(DISTINCT tt.wallet_address) AS active_wallets
        FROM token_transactions tt
        WHERE LOWER(tt.token_address) = et.token_address
          AND tt.created_at >= et.deployed_at
          AND tt.created_at <= et.deployed_at + INTERVAL '1 hour'
    ) base ON TRUE
    LEFT JOIN LATERAL (
        SELECT
            COUNT(*) FILTER (WHERE wa.alert_level = 'watch') AS watch_alerts,
            COUNT(*) FILTER (WHERE wa.alert_level = 'critical') AS critical_alerts
        FROM whale_alerts wa
        WHERE LOWER(wa.token_address) = et.token_address
          AND wa.created_at >= et.deployed_at
          AND wa.created_at <= et.deployed_at + INTERVAL '24 hours'
    ) wh ON TRUE
    LEFT JOIN LATERAL (
        SELECT
            COUNT(*) AS deployer_total_tokens,
            COUNT(*) FILTER (WHERE history.is_rug) AS deployer_rug_count,
            COUNT(*) FILTER (WHERE history.graduated) AS deployer_graduated_count
        FROM tokens history
        WHERE LOWER(history.deployer_address) = et.deployer_address
          AND history.deployed_at < et.deployed_at
    ) hist ON TRUE
    LEFT JOIN LATERAL (
        SELECT
            COUNT(*) FILTER (WHERE confidence = 'probable') AS probable_cluster_wallets,
            COUNT(*) FILTER (WHERE confidence = 'potential') AS potential_cluster_wallets
        FROM wallet_clusters wc
        WHERE LOWER(wc.token_address) = et.token_address
    ) cluster ON TRUE
),
future_window AS (
    SELECT
        bw.window_end,
        bw.token_address,
        COALESCE(fut.volume, 0)::double precision AS future_volume,
        COALESCE(fut.buys, 0)::double precision AS future_buys,
        COALESCE(fut.sells, 0)::double precision AS future_sells,
        COALESCE(fut.tx_count, 0)::double precision AS future_tx_count
    FROM base_windows bw
    LEFT JOIN LATERAL (
        SELECT
            SUM(tt.amount_bnb) AS volume,
            COUNT(*) FILTER (WHERE tt.tx_type = 'buy') AS buys,
            COUNT(*) FILTER (WHERE tt.tx_type = 'sell') AS sells,
            COUNT(*) AS tx_count
        FROM token_transactions tt
        WHERE LOWER(tt.token_address) = bw.token_address
          AND tt.created_at > bw.window_end
          AND tt.created_at <= bw.window_end + (%s || ' hours')::interval
    ) fut ON TRUE
)
SELECT bw.*, fw.future_volume, fw.future_buys, fw.future_sells, fw.future_tx_count
FROM base_windows bw
JOIN future_window fw
  ON fw.window_end = bw.window_end
 AND LOWER(fw.token_address) = LOWER(bw.token_address)
ORDER BY bw.window_end ASC, bw.token_address ASC
"""

INFERENCE_SQL = """
SELECT
    ar.window_end,
    ar.token_address,
    COALESCE(rs.composite_score, 50)::double precision AS risk_score,
    COALESCE(ar.alpha_score::double precision, 50.0) AS legacy_alpha_score,
    COALESCE(ar.rank, 50)::double precision AS legacy_rank,
    GREATEST(EXTRACT(EPOCH FROM (ar.window_end - t.deployed_at)) / 60.0, 0)::double precision AS token_age_minutes,
    COALESCE(t.initial_liquidity_bnb::double precision, 0.0) AS initial_liquidity_bnb,
    COALESCE(base.volume, 0)::double precision AS baseline_volume_1h,
    COALESCE(base.buys, 0)::double precision AS baseline_buys_1h,
    COALESCE(base.sells, 0)::double precision AS baseline_sells_1h,
    CASE
        WHEN COALESCE(base.buys, 0) + COALESCE(base.sells, 0) > 0
            THEN COALESCE(base.buys, 0)::double precision /
                 (COALESCE(base.buys, 0) + COALESCE(base.sells, 0))
        ELSE 0.5
    END AS buy_share_1h,
    COALESCE(base.active_wallets, 0)::double precision AS active_wallets_1h,
    COALESCE(base.tx_count, 0)::double precision AS tx_count_1h,
    COALESCE(wh.watch_alerts, 0)::double precision AS whale_watch_24h,
    COALESCE(wh.critical_alerts, 0)::double precision AS whale_critical_24h,
    COALESCE(hist.deployer_total_tokens, 0)::double precision AS deployer_total_tokens,
    COALESCE(hist.deployer_rug_count, 0)::double precision AS deployer_rug_count,
    COALESCE(hist.deployer_graduated_count, 0)::double precision AS deployer_graduated_count,
    COALESCE(cluster.probable_cluster_wallets, 0)::double precision AS probable_cluster_wallets,
    COALESCE(cluster.potential_cluster_wallets, 0)::double precision AS potential_cluster_wallets
FROM alpha_rankings ar
JOIN tokens t
  ON LOWER(t.contract_address) = LOWER(ar.token_address)
LEFT JOIN risk_scores rs
  ON LOWER(rs.token_address) = LOWER(ar.token_address)
LEFT JOIN LATERAL (
    SELECT
        SUM(tt.amount_bnb) AS volume,
        COUNT(*) FILTER (WHERE tt.tx_type = 'buy') AS buys,
        COUNT(*) FILTER (WHERE tt.tx_type = 'sell') AS sells,
        COUNT(*) AS tx_count,
        COUNT(DISTINCT tt.wallet_address) AS active_wallets
    FROM token_transactions tt
    WHERE LOWER(tt.token_address) = LOWER(ar.token_address)
      AND tt.created_at > ar.window_end - INTERVAL '1 hour'
      AND tt.created_at <= ar.window_end
) base ON TRUE
LEFT JOIN LATERAL (
    SELECT
        COUNT(*) FILTER (WHERE wa.alert_level = 'watch') AS watch_alerts,
        COUNT(*) FILTER (WHERE wa.alert_level = 'critical') AS critical_alerts
    FROM whale_alerts wa
    WHERE LOWER(wa.token_address) = LOWER(ar.token_address)
      AND wa.created_at > ar.window_end - INTERVAL '24 hours'
      AND wa.created_at <= ar.window_end
) wh ON TRUE
LEFT JOIN LATERAL (
    SELECT
        COUNT(*) AS deployer_total_tokens,
        COUNT(*) FILTER (WHERE history.is_rug) AS deployer_rug_count,
        COUNT(*) FILTER (WHERE history.graduated) AS deployer_graduated_count
    FROM tokens history
    WHERE LOWER(history.deployer_address) = LOWER(t.deployer_address)
      AND history.deployed_at <= ar.window_end
) hist ON TRUE
LEFT JOIN LATERAL (
    SELECT
        COUNT(*) FILTER (WHERE confidence = 'probable') AS probable_cluster_wallets,
        COUNT(*) FILTER (WHERE confidence = 'potential') AS potential_cluster_wallets
    FROM wallet_clusters wc
    WHERE LOWER(wc.token_address) = LOWER(ar.token_address)
) cluster ON TRUE
WHERE ar.window_end = %s
ORDER BY ar.rank ASC
"""


def _read_frame(conn: psycopg.Connection, sql: str, params: tuple) -> pd.DataFrame:
    with conn.cursor() as cur:
        cur.execute(sql, params)
        rows = cur.fetchall()
        columns = [item.name for item in cur.description]
    return pd.DataFrame(rows, columns=columns)


def load_training_frame(
    conn: psycopg.Connection,
    lookback_hours: int,
    horizon_hours: int,
) -> pd.DataFrame:
    return _read_frame(conn, TRAINING_SQL, (lookback_hours, horizon_hours))


def load_historical_training_frame(
    conn: psycopg.Connection,
    start_at,
    end_at,
    min_total_tx: int,
    horizon_hours: int,
) -> pd.DataFrame:
    return _read_frame(
        conn,
        HISTORICAL_TRAINING_SQL,
        (start_at, end_at, min_total_tx, horizon_hours),
    )


def load_training_frame_from_csv(input_dir: str, horizon_hours: int) -> pd.DataFrame:
    path = (
        pd.io.common.stringify_path(input_dir).rstrip("/")
        + f"/pattern_training_{horizon_hours}h.csv"
    )
    return pd.read_csv(path, parse_dates=["window_end"])


def load_inference_frame(conn: psycopg.Connection, window_end) -> pd.DataFrame:
    return _read_frame(conn, INFERENCE_SQL, (window_end,))


def resolve_target_windows(
    conn: psycopg.Connection,
    backfill_hours: int,
    window_limit: int,
    only_missing: bool,
    model_version: str,
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
                  FROM ml_pattern_predictions p
                  WHERE p.window_end = ar.window_end
                    AND p.model_version = %s
              )
            """
            params.append(model_version)
        sql += " ORDER BY ar.window_end DESC"
        if window_limit > 0:
            sql += " LIMIT %s"
            params.append(window_limit)
        cur.execute(sql, tuple(params))
        return [row[0] for row in cur.fetchall()]


def ensure_feature_frame(frame: pd.DataFrame) -> pd.DataFrame:
    normalized = frame.copy()
    for column in FEATURE_COLUMNS:
        if column not in normalized.columns:
            normalized[column] = 0.0
        normalized[column] = normalized[column].fillna(0.0).astype(float)
    return normalized


def upsert_model_registry(
    conn: psycopg.Connection,
    model_version: str,
    metadata: dict,
    activate: bool,
) -> None:
    with conn.cursor() as cur:
        if activate:
            cur.execute("UPDATE ml_pattern_model_registry SET is_active = false WHERE is_active = true")
        cur.execute(
            """
            INSERT INTO ml_pattern_model_registry (model_version, model_family, is_active, metadata, updated_at)
            VALUES (%s, 'lightgbm_similarity_iforest', %s, %s::jsonb, NOW())
            ON CONFLICT (model_version) DO UPDATE SET
                is_active = EXCLUDED.is_active,
                metadata = EXCLUDED.metadata,
                updated_at = NOW()
            """,
            (model_version, activate, json.dumps(metadata)),
        )
    conn.commit()


def resolve_active_model_version(conn: psycopg.Connection, explicit: str | None) -> str:
    if explicit:
        return explicit
    with conn.cursor() as cur:
        cur.execute(
            "SELECT model_version FROM ml_pattern_model_registry WHERE is_active = true ORDER BY updated_at DESC LIMIT 1"
        )
        row = cur.fetchone()
    if not row:
        raise RuntimeError("No active pattern-engine model in ml_pattern_model_registry.")
    return str(row[0])


def upsert_predictions(
    conn: psycopg.Connection,
    rows: Iterable[tuple],
) -> int:
    rows = list(rows)
    if not rows:
        return 0
    with conn.cursor() as cur:
        cur.executemany(
            """
            INSERT INTO ml_pattern_predictions (
                window_end,
                token_address,
                horizon_hours,
                model_version,
                match_label,
                outcome_class,
                score,
                confidence,
                anomaly_score,
                expected_path_summary,
                rationale,
                analogs,
                feature_snapshot
            )
            VALUES (%s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s::jsonb, %s::jsonb)
            ON CONFLICT (window_end, token_address, horizon_hours, model_version) DO UPDATE SET
                match_label = EXCLUDED.match_label,
                outcome_class = EXCLUDED.outcome_class,
                score = EXCLUDED.score,
                confidence = EXCLUDED.confidence,
                anomaly_score = EXCLUDED.anomaly_score,
                expected_path_summary = EXCLUDED.expected_path_summary,
                rationale = EXCLUDED.rationale,
                analogs = EXCLUDED.analogs,
                feature_snapshot = EXCLUDED.feature_snapshot
            """,
            rows,
        )
    conn.commit()
    return len(rows)
