#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
BACKUP_ROOT="${BACKUP_ROOT:-$REPO_DIR/backups/ml}"
ARTIFACT_ROOT="${ARTIFACT_ROOT:-$REPO_DIR/backend/ml_models}"
DB_URL="${DATABASE_URL:-postgres://mia:mia_password@localhost:5432/mia_db}"
VENV_PYTHON="${VENV_PYTHON:-$REPO_DIR/ml/.venv/bin/python}"
TS="$(date -u +'%Y%m%dT%H%M%SZ')"
DEST_DIR="$BACKUP_ROOT/$TS"

mkdir -p "$DEST_DIR"

MODEL_VERSION="$("$VENV_PYTHON" - <<PY
import psycopg
conn = psycopg.connect("${DB_URL}")
with conn, conn.cursor() as cur:
    cur.execute("SELECT model_version FROM ml_model_registry WHERE is_active = true ORDER BY updated_at DESC LIMIT 1")
    row = cur.fetchone()
print(row[0] if row else "")
PY
)"

if [ -z "$MODEL_VERSION" ]; then
  echo "no active model; skip backup"
  exit 0
fi

JOBLIB="$ARTIFACT_ROOT/$MODEL_VERSION.joblib"
META="$ARTIFACT_ROOT/$MODEL_VERSION.metadata.json"

if [ ! -f "$JOBLIB" ]; then
  echo "artifact missing: $JOBLIB"
  exit 1
fi

cp "$JOBLIB" "$DEST_DIR/"
[ -f "$META" ] && cp "$META" "$DEST_DIR/"
echo "$MODEL_VERSION" > "$DEST_DIR/model_version.txt"

# Keep last 14 daily snapshots.
find "$BACKUP_ROOT" -mindepth 1 -maxdepth 1 -type d -mtime +14 -exec rm -rf {} +

echo "backup complete: $DEST_DIR"
