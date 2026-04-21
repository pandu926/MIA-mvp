#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
ROOT_DIR="$REPO_DIR/ml"
LOG_DIR="$ROOT_DIR/logs"
LOG_FILE="$LOG_DIR/realized_labels.log"
LOCK_FILE="/tmp/mia-ml-label.lock"
DB_URL="${DATABASE_URL:-postgres://mia:mia_password@localhost:5432/mia_db}"
VENV_ACTIVATE="${VENV_ACTIVATE:-$ROOT_DIR/.venv/bin/activate}"

mkdir -p "$LOG_DIR"

{
  echo "[$(date -u +'%Y-%m-%dT%H:%M:%SZ')] start realized label update"
  /usr/bin/flock -n "$LOCK_FILE" /bin/bash -lc "
    cd '$ROOT_DIR' &&
    . '$VENV_ACTIVATE' &&
    python update_realized_labels.py --database-url '$DB_URL' --max-age-hours 336
  "
  echo "[$(date -u +'%Y-%m-%dT%H:%M:%SZ')] done realized label update"
} >>"$LOG_FILE" 2>&1
