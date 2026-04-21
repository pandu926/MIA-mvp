#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import os

import psycopg


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Backfill realized 1h labels for ml_alpha_predictions.")
    parser.add_argument("--database-url", default=os.getenv("DATABASE_URL"), required=os.getenv("DATABASE_URL") is None)
    parser.add_argument("--max-age-hours", type=int, default=336)
    parser.add_argument("--hit-threshold", type=float, default=65.0)
    parser.add_argument(
        "--recompute-existing",
        action="store_true",
        help="Recompute realized labels even when existing values are present.",
    )
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    with psycopg.connect(args.database_url) as conn:
        with conn.cursor() as cur:
            cur.execute(
                """
                WITH candidates AS (
                    SELECT p.id, p.window_end, p.token_address
                    FROM ml_alpha_predictions p
                    WHERE (%s OR p.realized_hit_1h IS NULL)
                      AND p.window_end <= NOW() - INTERVAL '1 hour'
                      AND p.window_end >= NOW() - (%s || ' hours')::interval
                ),
                agg AS (
                    SELECT
                        c.id,
                        GREATEST(COALESCE(base.volume, 0)::double precision, 0.05) AS baseline_volume_1h,
                        COALESCE(f1.volume, 0)::double precision AS future_volume_1h,
                        COALESCE(f1.buys, 0)::double precision AS future_buy_count_1h,
                        COALESCE(f1.sells, 0)::double precision AS future_sell_count_1h
                    FROM candidates c
                    LEFT JOIN LATERAL (
                        SELECT SUM(tt.amount_bnb) AS volume
                        FROM token_transactions tt
                        WHERE tt.token_address = c.token_address
                          AND tt.created_at > c.window_end - INTERVAL '1 hour'
                          AND tt.created_at <= c.window_end
                    ) base ON TRUE
                    LEFT JOIN LATERAL (
                        SELECT
                            SUM(tt.amount_bnb) AS volume,
                            COUNT(*) FILTER (WHERE tt.tx_type = 'buy') AS buys,
                            COUNT(*) FILTER (WHERE tt.tx_type = 'sell') AS sells
                        FROM token_transactions tt
                        WHERE tt.token_address = c.token_address
                          AND tt.created_at > c.window_end
                          AND tt.created_at <= c.window_end + INTERVAL '1 hour'
                    ) f1 ON TRUE
                ),
                scored AS (
                    SELECT
                        a.id,
                        (
                            (
                                LEAST(
                                    GREATEST(a.future_volume_1h / a.baseline_volume_1h, 0.0),
                                    3.0
                                ) / 3.0
                            ) * 55.0
                            +
                            (
                                CASE
                                    WHEN a.future_buy_count_1h + a.future_sell_count_1h > 0
                                        THEN (a.future_buy_count_1h / (a.future_buy_count_1h + a.future_sell_count_1h))
                                    ELSE 0.5
                                END
                            ) * 45.0
                        ) *
                        (
                            CASE
                                WHEN a.future_buy_count_1h + a.future_sell_count_1h < 3 THEN 0.75
                                ELSE 1.0
                            END
                        ) AS score_1h
                    FROM agg a
                )
                UPDATE ml_alpha_predictions p
                SET realized_score_1h = ROUND(s.score_1h::numeric, 2),
                    realized_hit_1h = (s.score_1h >= %s),
                    realized_at = NOW()
                FROM scored s
                WHERE p.id = s.id
                RETURNING p.id
                """,
                (args.recompute_existing, args.max_age_hours, args.hit_threshold),
            )
            updated = cur.fetchall()
        conn.commit()

    print(
        json.dumps(
            {
                "updated_rows": len(updated),
                "hit_threshold": args.hit_threshold,
                "recompute_existing": args.recompute_existing,
            },
            indent=2,
        )
    )


if __name__ == "__main__":
    main()
