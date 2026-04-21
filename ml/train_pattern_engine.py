#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import os
from pathlib import Path

import psycopg

from pattern_engine.common import HORIZONS, artifact_path, default_model_version, metadata_path
from pattern_engine.dataset import (
    ensure_feature_frame,
    load_training_frame,
    load_training_frame_from_csv,
    upsert_model_registry,
)
from pattern_engine.modeling import fit_horizon_model, save_artifact_bundle, write_metadata


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Train the Deep Research Pattern Match Engine.")
    parser.add_argument("--database-url", default=os.getenv("DATABASE_URL"))
    parser.add_argument("--lookback-hours", type=int, default=24 * 14)
    parser.add_argument(
        "--input-dir",
        default=None,
        help="Optional directory containing exported pattern_training_<h>h.csv files.",
    )
    parser.add_argument("--model-version", default=default_model_version())
    parser.add_argument(
        "--output-dir",
        default=str(Path(__file__).resolve().parents[1] / "backend" / "ml_models" / "pattern_engine"),
    )
    parser.add_argument("--activate", action="store_true")
    args = parser.parse_args()
    if not args.input_dir and not args.database_url:
        parser.error("--database-url is required unless --input-dir is provided.")
    return args


def main() -> None:
    args = parse_args()
    results = {}
    artifact_bundle = {
        "model_family": "lightgbm_similarity_iforest",
        "model_version": args.model_version,
        "horizons": {},
    }

    if args.input_dir:
        conn = None
    else:
        conn = psycopg.connect(args.database_url)

    try:
        for horizon_hours in HORIZONS:
            if args.input_dir:
                frame = ensure_feature_frame(
                    load_training_frame_from_csv(args.input_dir, horizon_hours)
                )
            else:
                frame = ensure_feature_frame(
                    load_training_frame(conn, args.lookback_hours, horizon_hours)
                )
            print(
                json.dumps(
                    {
                        "stage": "load_training_frame",
                        "horizon_hours": horizon_hours,
                        "rows": len(frame),
                        "source": "csv" if args.input_dir else "database",
                    }
                ),
                flush=True,
            )
            result = fit_horizon_model(frame, horizon_hours)
            artifact_bundle["horizons"][str(horizon_hours)] = result.artifact
            results[str(horizon_hours)] = {
                "sample_count": result.sample_count,
                "label_distribution": result.label_distribution,
                "metrics": result.metrics,
            }
            print(
                json.dumps(
                    {
                        "stage": "fit_complete",
                        "horizon_hours": horizon_hours,
                        "sample_count": result.sample_count,
                        "metrics": result.metrics,
                    }
                ),
                flush=True,
            )

        save_artifact_bundle(artifact_path(args.output_dir, args.model_version), artifact_bundle)
        metadata = {
            "model_version": args.model_version,
            "model_family": artifact_bundle["model_family"],
            "lookback_hours": args.lookback_hours,
            "horizons": results,
        }
        write_metadata(metadata_path(args.output_dir, args.model_version), metadata)
        if conn is not None:
            upsert_model_registry(conn, args.model_version, metadata, args.activate)
    finally:
        if conn is not None:
            conn.close()

    print(json.dumps(metadata, indent=2))


if __name__ == "__main__":
    main()
