#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
from datetime import datetime

import psycopg

from pattern_engine.common import HORIZONS
from pattern_engine.dataset import load_historical_training_frame, load_training_frame


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Export pattern-engine training datasets to CSV.")
    parser.add_argument("--database-url", default=os.getenv("DATABASE_URL"), required=os.getenv("DATABASE_URL") is None)
    parser.add_argument("--lookback-hours", type=int, default=24 * 14)
    parser.add_argument(
        "--start-at",
        default=None,
        help="Historical export start timestamp, for example 2025-04-01T00:00:00Z.",
    )
    parser.add_argument(
        "--end-at",
        default=None,
        help="Historical export end timestamp, for example 2026-05-01T00:00:00Z.",
    )
    parser.add_argument(
        "--min-total-tx",
        type=int,
        default=0,
        help="Historical export filter. Only include tokens whose lifetime tx count is at least this threshold.",
    )
    parser.add_argument(
        "--horizons",
        default="1,6,24",
        help="Comma-separated horizon list. Example: 24 or 1,6,24",
    )
    parser.add_argument(
        "--output-dir",
        default=str(Path(__file__).resolve().parent / "exports" / "pattern_engine"),
    )
    parser.add_argument(
        "--summary-only",
        action="store_true",
        help="Print export counts without writing CSV files.",
    )
    args = parser.parse_args()
    if (args.start_at is None) != (args.end_at is None):
        parser.error("--start-at and --end-at must be provided together.")
    return args


def parse_horizons(raw: str) -> list[int]:
    values = []
    for item in raw.split(","):
        item = item.strip()
        if not item:
            continue
        horizon = int(item)
        if horizon not in HORIZONS:
            raise ValueError(f"Unsupported horizon: {horizon}")
        values.append(horizon)
    if not values:
        raise ValueError("At least one horizon is required.")
    return values


def parse_timestamp(raw: str | None) -> datetime | None:
    if raw is None:
        return None
    normalized = raw.strip()
    if normalized.endswith("Z"):
        normalized = normalized[:-1] + "+00:00"
    return datetime.fromisoformat(normalized)


def main() -> None:
    args = parse_args()
    output_dir = Path(args.output_dir).resolve()
    horizons = parse_horizons(args.horizons)
    start_at = parse_timestamp(args.start_at)
    end_at = parse_timestamp(args.end_at)
    export_mode = "historical" if start_at and end_at else "rolling"

    if not args.summary_only:
        output_dir.mkdir(parents=True, exist_ok=True)

    summary = {
        "mode": export_mode,
        "lookback_hours": args.lookback_hours,
        "start_at": start_at.isoformat() if start_at else None,
        "end_at": end_at.isoformat() if end_at else None,
        "min_total_tx": args.min_total_tx,
        "horizons": horizons,
        "files": [],
    }
    with psycopg.connect(args.database_url) as conn:
        for horizon_hours in horizons:
            if start_at and end_at:
                frame = load_historical_training_frame(
                    conn,
                    start_at,
                    end_at,
                    args.min_total_tx,
                    horizon_hours,
                )
            else:
                frame = load_training_frame(conn, args.lookback_hours, horizon_hours)
            path = output_dir / f"pattern_training_{horizon_hours}h.csv"
            item = {
                "horizon_hours": horizon_hours,
                "rows": len(frame),
                "path": None if args.summary_only else str(path),
            }
            if not args.summary_only:
                frame.to_csv(path, index=False)
            summary["files"].append(item)

    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    main()
