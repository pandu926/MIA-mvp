#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/ml/remote_pattern_common.sh"

MODEL_VERSION="${MODEL_VERSION:-}"
LOCAL_ARTIFACT_DIR="${LOCAL_ARTIFACT_DIR:-$ROOT_DIR/backend/ml_models/pattern_engine}"

if [ -z "$MODEL_VERSION" ]; then
  echo "MODEL_VERSION is required." >&2
  exit 1
fi

mkdir -p "$LOCAL_ARTIFACT_DIR"
scp_from_remote \
  "$REMOTE_SSH_TARGET:$REMOTE_ARTIFACT_DIR/$MODEL_VERSION.joblib" \
  "$REMOTE_SSH_TARGET:$REMOTE_ARTIFACT_DIR/$MODEL_VERSION.metadata.json" \
  "$LOCAL_ARTIFACT_DIR/"

echo "pulled_model_version=$MODEL_VERSION"
