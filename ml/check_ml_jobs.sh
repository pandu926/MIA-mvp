#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
ROOT_DIR="$REPO_DIR/ml"
LOG_DIR="$ROOT_DIR/logs"
LOG_FILE="$LOG_DIR/monitor.log"
DB_URL="${DATABASE_URL:-postgres://mia:mia_password@localhost:5432/mia_db}"
MAX_PRED_AGE_MIN=120
MAX_LABEL_AGE_MIN=180
VENV_PYTHON="${VENV_PYTHON:-$ROOT_DIR/.venv/bin/python}"

mkdir -p "$LOG_DIR"

RESULT="$(
  "$VENV_PYTHON" - <<PY
import json
import psycopg

conn = psycopg.connect("${DB_URL}")
with conn, conn.cursor() as cur:
    cur.execute("""
        SELECT
          EXTRACT(EPOCH FROM (NOW() - COALESCE(MAX(created_at), NOW()))) / 60.0 AS pred_age_min,
          EXTRACT(EPOCH FROM (NOW() - COALESCE(MAX(realized_at), NOW()))) / 60.0 AS label_age_min
        FROM ml_alpha_predictions
    """)
    row = cur.fetchone()
print(json.dumps({"pred_age_min": float(row[0]), "label_age_min": float(row[1])}))
PY
)"

PRED_AGE="$(echo "$RESULT" | python3 -c 'import json,sys; print(int(json.load(sys.stdin)["pred_age_min"]))')"
LABEL_AGE="$(echo "$RESULT" | python3 -c 'import json,sys; print(int(json.load(sys.stdin)["label_age_min"]))')"

STAMP="[$(date -u +'%Y-%m-%dT%H:%M:%SZ')]"
if [ "$PRED_AGE" -gt "$MAX_PRED_AGE_MIN" ] || [ "$LABEL_AGE" -gt "$MAX_LABEL_AGE_MIN" ]; then
  MSG="$STAMP ALERT ml pipeline stale: pred_age_min=$PRED_AGE label_age_min=$LABEL_AGE"
  echo "$MSG" | tee -a "$LOG_FILE"
  logger -t mia-ml-monitor "$MSG"
  exit 1
fi

echo "$STAMP OK ml pipeline fresh: pred_age_min=$PRED_AGE label_age_min=$LABEL_AGE" >>"$LOG_FILE"
