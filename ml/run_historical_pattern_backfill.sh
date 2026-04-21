#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DB_URL="${LOCAL_DATABASE_URL:-${DATABASE_URL:-postgres://mia:mia_password@localhost:5432/mia_db}}"
OUTPUT_DIR="${OUTPUT_DIR:-$ROOT_DIR/ml/exports/pattern_engine}"
START_AT="${START_AT:-2025-04-01T00:00:00Z}"
END_AT="${END_AT:-2026-05-01T00:00:00Z}"
MIN_TOTAL_TX="${MIN_TOTAL_TX:-50}"
HORIZONS="${HORIZONS:-1,6,24}"
SUMMARY_ONLY="${SUMMARY_ONLY:-0}"

VENV_PYTHON="${VENV_PYTHON:-$ROOT_DIR/ml/.venv/bin/python}"

CMD=(
  "$VENV_PYTHON"
  "$ROOT_DIR/ml/export_pattern_training_data.py"
  --database-url "$DB_URL"
  --start-at "$START_AT"
  --end-at "$END_AT"
  --min-total-tx "$MIN_TOTAL_TX"
  --horizons "$HORIZONS"
  --output-dir "$OUTPUT_DIR"
)

if [ "$SUMMARY_ONLY" = "1" ]; then
  CMD+=(--summary-only)
fi

"${CMD[@]}"
