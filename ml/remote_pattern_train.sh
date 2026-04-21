#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/ml/remote_pattern_common.sh"

MODEL_VERSION="${MODEL_VERSION:-pattern-engine-remote-$(date -u +%Y%m%d-%H%M%S)}"
LOOKBACK_HOURS="${LOOKBACK_HOURS:-168}"
LOCAL_DATABASE_URL="${LOCAL_DATABASE_URL:-${DATABASE_URL:-postgres://mia:mia_password@localhost:5432/mia_db}}"
HISTORICAL_START_AT="${HISTORICAL_START_AT:-}"
HISTORICAL_END_AT="${HISTORICAL_END_AT:-}"
MIN_TOTAL_TX="${MIN_TOTAL_TX:-0}"
VENV_PYTHON="${VENV_PYTHON:-$ROOT_DIR/ml/.venv/bin/python}"

EXPORT_ARGS=()
if [ -n "$HISTORICAL_START_AT" ] || [ -n "$HISTORICAL_END_AT" ]; then
  if [ -z "$HISTORICAL_START_AT" ] || [ -z "$HISTORICAL_END_AT" ]; then
    echo "HISTORICAL_START_AT and HISTORICAL_END_AT must be set together." >&2
    exit 1
  fi
  EXPORT_ARGS+=(
    --start-at "$HISTORICAL_START_AT"
    --end-at "$HISTORICAL_END_AT"
    --min-total-tx "$MIN_TOTAL_TX"
  )
else
  EXPORT_ARGS+=(--lookback-hours "$LOOKBACK_HOURS")
fi

for horizon in 1 6 24; do
  "$VENV_PYTHON" "$ROOT_DIR/ml/export_pattern_training_data.py" \
    --database-url "$LOCAL_DATABASE_URL" \
    --horizons "$horizon" \
    "${EXPORT_ARGS[@]}"
done

"$ROOT_DIR/ml/remote_pattern_sync.sh"

REMOTE_CMD="
set -euo pipefail
cd '$REMOTE_PROJECT_DIR/ml'
python3 -m venv .venv
. .venv/bin/activate
pip install -r requirements.txt >/tmp/mia-pattern-pip.log 2>&1
mkdir -p '$REMOTE_ARTIFACT_DIR'
python -u train_pattern_engine.py \
  --input-dir '$REMOTE_PROJECT_DIR/ml/exports/pattern_engine' \
  --lookback-hours '$LOOKBACK_HOURS' \
  --model-version '$MODEL_VERSION' \
  --output-dir '$REMOTE_ARTIFACT_DIR'
"

ssh_remote "$REMOTE_CMD"

echo "remote_model_version=$MODEL_VERSION"
