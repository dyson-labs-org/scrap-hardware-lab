#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

POLICY_PATH="${1:-$REPO_ROOT/demo/config/policy.json}"
KEYS_PATH="${2:-$REPO_ROOT/demo/config/keys.json}"
BIND_ADDR="${EXECUTOR_BIND:-0.0.0.0}"
PORT="${EXECUTOR_PORT:-7227}"
PYTHON_BIN="${PYTHON_BIN:-python3}"

NODE_ID="$("$PYTHON_BIN" - "$POLICY_PATH" <<'PY'
import json
import sys
path = sys.argv[1]
with open(path, "r", encoding="utf-8") as handle:
    data = json.load(handle)
node_id = data.get("node_id")
if not node_id:
    raise SystemExit("policy.json missing node_id")
print(node_id)
PY
)"

RUNTIME_DIR="$REPO_ROOT/demo/runtime/$NODE_ID"
mkdir -p "$RUNTIME_DIR"

if [[ ! -f "$RUNTIME_DIR/replay_cache.json" ]]; then
  echo "{}" > "$RUNTIME_DIR/replay_cache.json"
fi
if [[ ! -f "$RUNTIME_DIR/revoked.json" ]]; then
  echo "[]" > "$RUNTIME_DIR/revoked.json"
fi

LOG_FILE="$RUNTIME_DIR/executor.log"

pkill -f "src.node.executor" >/dev/null 2>&1 || true

cd "$REPO_ROOT"
nohup env PYTHONPATH=. "$PYTHON_BIN" -u -m src.node.executor \
  --bind "$BIND_ADDR" \
  --port "$PORT" \
  --keys "$KEYS_PATH" \
  --policy "$POLICY_PATH" \
  --allow-mock-signatures \
  > "$LOG_FILE" 2>&1 &

sleep 0.3
PID="$(pgrep -f "src.node.executor" | head -n 1 || true)"
echo "[executor] node_id=$NODE_ID pid=$PID log=$LOG_FILE"
ss -lunp | grep ":$PORT" || true
tail -n 20 "$LOG_FILE" || true
