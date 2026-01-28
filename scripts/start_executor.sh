#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
source "$SCRIPT_DIR/lab_env.sh"

"$SCRIPT_DIR/init_runtime.sh" >/dev/null

RUNTIME_DIR="$REPO_ROOT/demo/runtime/$EXECUTOR_NODE_ID"
KEYS_JSON="$RUNTIME_DIR/keys.json"
POLICY_JSON="$RUNTIME_DIR/policy.json"
REVOKED_JSON="$RUNTIME_DIR/revoked.json"
REMOTE_RUNTIME="$REPO_DIR/demo/runtime/$EXECUTOR_NODE_ID"
LOG_FILE="$REMOTE_RUNTIME/executor.log"

if [[ "$EXECUTOR_HOST" == "127.0.0.1" || "$EXECUTOR_HOST" == "localhost" ]]; then
  mkdir -p "$RUNTIME_DIR"
  nohup $PYTHON -m src.node.executor \
    --bind "$EXECUTOR_BIND" \
    --port "$EXECUTOR_PORT" \
    --keys "$KEYS_JSON" \
    --policy "$POLICY_JSON" \
    > "$RUNTIME_DIR/executor.log" 2>&1 &
  echo "[executor] started locally (log: $RUNTIME_DIR/executor.log)"
  exit 0
fi

ssh "${EXECUTOR_USER}@${EXECUTOR_HOST}" "mkdir -p ${REMOTE_RUNTIME}"
scp "$KEYS_JSON" "$POLICY_JSON" "$REVOKED_JSON" "$EXECUTOR_USER@$EXECUTOR_HOST:$REMOTE_RUNTIME/"

ssh "${EXECUTOR_USER}@${EXECUTOR_HOST}" "cd $REPO_DIR && nohup $PYTHON -m src.node.executor \
  --bind $EXECUTOR_BIND \
  --port $EXECUTOR_PORT \
  --keys $REMOTE_RUNTIME/keys.json \
  --policy $REMOTE_RUNTIME/policy.json \
  > $LOG_FILE 2>&1 &"

echo "[executor] started on $EXECUTOR_HOST (log: $LOG_FILE)"
