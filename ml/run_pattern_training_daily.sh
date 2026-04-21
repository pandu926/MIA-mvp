#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
ROOT_DIR="$REPO_DIR/ml"
LOG_DIR="$ROOT_DIR/logs"
LOG_FILE="$LOG_DIR/pattern-training.log"
LOCK_FILE="/tmp/mia-pattern-train.lock"
DB_URL="${DATABASE_URL:-postgres://mia:mia_password@localhost:5432/mia_db}"
VENV_ACTIVATE="${VENV_ACTIVATE:-$ROOT_DIR/.venv/bin/activate}"

mkdir -p "$LOG_DIR"

{
  echo "[$(date -u +'%Y-%m-%dT%H:%M:%SZ')] start pattern-engine daily train"
  /usr/bin/flock -n "$LOCK_FILE" /bin/bash -lc "
    cd '$ROOT_DIR' &&
    . '$VENV_ACTIVATE' &&
    python train_pattern_engine.py --database-url '$DB_URL' --activate &&
    python run_pattern_inference.py --database-url '$DB_URL' --backfill-hours 24 --only-missing
  "
  echo "[$(date -u +'%Y-%m-%dT%H:%M:%SZ')] done pattern-engine daily train"
} >>"$LOG_FILE" 2>&1
