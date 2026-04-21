#!/usr/bin/env bash
set -euo pipefail

REMOTE_HOST="${MIA_REMOTE_HOST:-}"
REMOTE_USER="${MIA_REMOTE_USER:-root}"
REMOTE_BASE_DIR="${MIA_REMOTE_BASE_DIR:-/opt/mia-ml-trainer}"
REMOTE_ARTIFACT_DIR="$REMOTE_BASE_DIR/backend/ml_models/pattern_engine"
REMOTE_PROJECT_DIR="$REMOTE_BASE_DIR/project"
REMOTE_SSH_TARGET="${REMOTE_USER}@${REMOTE_HOST}"

require_remote_password() {
  if [ -z "$REMOTE_HOST" ]; then
    echo "MIA_REMOTE_HOST is required." >&2
    exit 1
  fi
  if [ -z "${MIA_REMOTE_PASSWORD:-}" ]; then
    echo "MIA_REMOTE_PASSWORD is required." >&2
    exit 1
  fi
}

ssh_remote() {
  require_remote_password
  sshpass -p "$MIA_REMOTE_PASSWORD" ssh -o StrictHostKeyChecking=no "$REMOTE_SSH_TARGET" "$@"
}

scp_to_remote() {
  require_remote_password
  sshpass -p "$MIA_REMOTE_PASSWORD" scp -o StrictHostKeyChecking=no "$@"
}

scp_from_remote() {
  require_remote_password
  sshpass -p "$MIA_REMOTE_PASSWORD" scp -o StrictHostKeyChecking=no "$@"
}
