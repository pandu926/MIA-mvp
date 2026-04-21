#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
ROOT_DIR="$REPO_DIR/ml"
LOG_DIR="$ROOT_DIR/logs"
LOG_FILE="$LOG_DIR/inference.log"
LOCK_FILE="/tmp/mia-ml-infer.lock"
DB_URL="${DATABASE_URL:-postgres://mia:mia_password@localhost:5432/mia_db}"
MODEL_VERSION="${ML_MODEL_VERSION:-lightgbm-shadow-v0}"
VENV_ACTIVATE="${VENV_ACTIVATE:-$ROOT_DIR/.venv/bin/activate}"

mkdir -p "$LOG_DIR"

{
  echo "[$(date -u +'%Y-%m-%dT%H:%M:%SZ')] start inference"
  /usr/bin/flock -n "$LOCK_FILE" /bin/bash -lc "
    cd '$ROOT_DIR' &&
    . '$VENV_ACTIVATE' &&
    python run_alpha_inference.py --database-url '$DB_URL' --model-version '$MODEL_VERSION'
  "
  echo "[$(date -u +'%Y-%m-%dT%H:%M:%SZ')] done inference"
} >>"$LOG_FILE" 2>&1
