#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
source "$SCRIPT_DIR/lab_env.sh"

RUNTIME_DIR="$REPO_ROOT/demo/runtime/$EXECUTOR_NODE_ID"
REMOTE_RUNTIME="$REPO_DIR/demo/runtime/$EXECUTOR_NODE_ID"

mkdir -p "$RUNTIME_DIR"

if [[ "$EXECUTOR_HOST" == "127.0.0.1" || "$EXECUTOR_HOST" == "localhost" ]]; then
  echo "[logs] local: $RUNTIME_DIR"
  exit 0
fi

scp "$EXECUTOR_USER@$EXECUTOR_HOST:$REMOTE_RUNTIME/executor.log" "$RUNTIME_DIR/" || true
scp "$EXECUTOR_USER@$EXECUTOR_HOST:$REMOTE_RUNTIME/replay_cache.json" "$RUNTIME_DIR/" || true

echo "[logs] collected to $RUNTIME_DIR"
