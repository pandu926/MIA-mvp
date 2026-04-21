#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
ROOT_DIR="$REPO_DIR/ml"
LOG_DIR="$ROOT_DIR/logs"
LOG_FILE="$LOG_DIR/auto_promotion.log"
LOCK_FILE="/tmp/mia-ml-promo.lock"
BASE_URL="${MIA_BACKEND_URL:-http://127.0.0.1:8080}"
VENV_ACTIVATE="${VENV_ACTIVATE:-$ROOT_DIR/.venv/bin/activate}"

mkdir -p "$LOG_DIR"

{
  echo "[$(date -u +'%Y-%m-%dT%H:%M:%SZ')] start auto-promotion check"
  /usr/bin/flock -n "$LOCK_FILE" /bin/bash -lc "
    cd '$ROOT_DIR' &&
    . '$VENV_ACTIVATE' &&
    python auto_promote_rollout.py --base-url '$BASE_URL' --hours 336
  "
  echo "[$(date -u +'%Y-%m-%dT%H:%M:%SZ')] done auto-promotion check"
} >>"$LOG_FILE" 2>&1
