#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
source "$SCRIPT_DIR/lab_env.sh"

TARGET_HOST="$EXECUTOR_HOST"
TARGET_USER="$EXECUTOR_USER"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --host) TARGET_HOST="$2"; shift 2;;
    --user) TARGET_USER="$2"; shift 2;;
    *) echo "Unknown arg: $1"; exit 2;;
  esac
 done

if [[ -z "${TARGET_HOST}" || -z "${TARGET_USER}" ]]; then
  echo "Missing target host/user" >&2
  exit 2
fi

ssh "${TARGET_USER}@${TARGET_HOST}" "mkdir -p ${REPO_DIR}"
scp -r "$REPO_ROOT/src" "$REPO_ROOT/demo" "$REPO_ROOT/docs" "$REPO_ROOT/scripts" \
  "${TARGET_USER}@${TARGET_HOST}:${REPO_DIR}/"

echo "[push] ${TARGET_USER}@${TARGET_HOST}:${REPO_DIR}"
