#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
source "$SCRIPT_DIR/lab_env.sh"

TOKEN_NAME="authorized"
TASK_ID="$TASK_ID"
CAPABILITY="$TASK_CAPABILITY"
KEYS_JSON=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --token-name) TOKEN_NAME="$2"; shift 2;;
    --task-id) TASK_ID="$2"; shift 2;;
    --capability) CAPABILITY="$2"; shift 2;;
    --keys) KEYS_JSON="$2"; shift 2;;
    *) echo "Unknown arg: $1"; exit 2;;
  esac
 done

RUNTIME_DIR="$REPO_ROOT/demo/runtime/$EXECUTOR_NODE_ID"
TOKEN_BIN="$RUNTIME_DIR/tokens/${TOKEN_NAME}.bin"

if [[ -z "$KEYS_JSON" ]]; then
  KEYS_JSON="$RUNTIME_DIR/keys.json"
fi

if [[ ! -f "$TOKEN_BIN" ]]; then
  echo "Missing token: $TOKEN_BIN" >&2
  exit 2
fi

$PYTHON -m src.node.commander \
  --target-host "$EXECUTOR_HOST" \
  --target-port "$EXECUTOR_PORT" \
  --token "$TOKEN_BIN" \
  --keys "$KEYS_JSON" \
  --task-id "$TASK_ID" \
  --requested-capability "$CAPABILITY" \
  --task-type "$TASK_TYPE" \
  --max-amount-sats "$MAX_AMOUNT_SATS" \
  --allow-mock-signatures
