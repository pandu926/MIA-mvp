#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/ml/remote_pattern_common.sh"

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

mkdir -p "$TMP_DIR/ml/pattern_engine" "$TMP_DIR/backend/ml_models/pattern_engine"
if [ -d "$ROOT_DIR/ml/exports/pattern_engine" ]; then
  mkdir -p "$TMP_DIR/ml/exports/pattern_engine"
  cp -R "$ROOT_DIR/ml/exports/pattern_engine/." "$TMP_DIR/ml/exports/pattern_engine/"
fi
cp "$ROOT_DIR/ml/requirements.txt" "$TMP_DIR/ml/requirements.txt"
cp "$ROOT_DIR/ml/export_pattern_training_data.py" "$TMP_DIR/ml/export_pattern_training_data.py"
cp "$ROOT_DIR/ml/train_pattern_engine.py" "$TMP_DIR/ml/train_pattern_engine.py"
cp "$ROOT_DIR/ml/run_pattern_inference.py" "$TMP_DIR/ml/run_pattern_inference.py"
cp "$ROOT_DIR/ml/run_pattern_training_daily.sh" "$TMP_DIR/ml/run_pattern_training_daily.sh"
cp "$ROOT_DIR/ml/run_pattern_inference_hourly.sh" "$TMP_DIR/ml/run_pattern_inference_hourly.sh"
cp "$ROOT_DIR/ml/run_historical_pattern_backfill.sh" "$TMP_DIR/ml/run_historical_pattern_backfill.sh"
cp "$ROOT_DIR/ml/remote_pattern_common.sh" "$TMP_DIR/ml/remote_pattern_common.sh"
cp "$ROOT_DIR/ml/remote_pattern_sync.sh" "$TMP_DIR/ml/remote_pattern_sync.sh"
cp "$ROOT_DIR/ml/remote_pattern_train.sh" "$TMP_DIR/ml/remote_pattern_train.sh"
cp "$ROOT_DIR/ml/remote_pattern_pull.sh" "$TMP_DIR/ml/remote_pattern_pull.sh"
cp -R "$ROOT_DIR/ml/pattern_engine/." "$TMP_DIR/ml/pattern_engine/"

ssh_remote "mkdir -p '$REMOTE_PROJECT_DIR' '$REMOTE_ARTIFACT_DIR'"
scp_to_remote -r "$TMP_DIR/ml" "$REMOTE_SSH_TARGET:$REMOTE_PROJECT_DIR/"

echo "synced_ml_workspace=$REMOTE_PROJECT_DIR"
