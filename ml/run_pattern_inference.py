#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import os
from pathlib import Path

import psycopg

from pattern_engine.common import HORIZONS, artifact_path
from pattern_engine.dataset import (
    ensure_feature_frame,
    load_inference_frame,
    resolve_active_model_version,
    resolve_target_windows,
    upsert_predictions,
)
from pattern_engine.modeling import load_artifact_bundle, predict_horizon


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Run inference for the Deep Research Pattern Match Engine.")
    parser.add_argument("--database-url", default=os.getenv("DATABASE_URL"), required=os.getenv("DATABASE_URL") is None)
    parser.add_argument("--model-version", default=None)
    parser.add_argument(
        "--artifacts-dir",
        default=str(Path(__file__).resolve().parents[1] / "backend" / "ml_models" / "pattern_engine"),
    )
    parser.add_argument("--backfill-hours", type=int, default=0)
    parser.add_argument("--window-limit", type=int, default=0)
    parser.add_argument("--only-missing", action="store_true")
    parser.add_argument("--dry-run", action="store_true")
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    with psycopg.connect(args.database_url) as conn:
        model_version = resolve_active_model_version(conn, args.model_version)
        bundle = load_artifact_bundle(artifact_path(args.artifacts_dir, model_version))
        windows = resolve_target_windows(
            conn,
            args.backfill_hours,
            args.window_limit,
            args.only_missing,
            model_version,
        )

        all_rows = []
        for window_end in windows:
            frame = ensure_feature_frame(load_inference_frame(conn, window_end))
            if frame.empty:
                continue
            for horizon_hours in HORIZONS:
                artifact = bundle["horizons"][str(horizon_hours)]
                for item in predict_horizon(artifact, frame, horizon_hours):
                    all_rows.append(
                        (
                            item["window_end"],
                            item["token_address"],
                            horizon_hours,
                            model_version,
                            item["match_label"],
                            item["outcome_class"],
                            item["score"],
                            item["confidence"],
                            item["anomaly_score"],
                            item["expected_path_summary"],
                            item["rationale"],
                            json.dumps(item["analogs"]),
                            json.dumps(item["feature_snapshot"]),
                        )
                    )

        written = 0 if args.dry_run else upsert_predictions(conn, all_rows)

    print(
        json.dumps(
            {
                "model_version": model_version,
                "window_count": len(windows),
                "prediction_rows": len(all_rows),
                "written_rows": written,
                "dry_run": args.dry_run,
            },
            indent=2,
        )
    )


if __name__ == "__main__":
    main()

