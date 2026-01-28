#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
source "$SCRIPT_DIR/lab_env.sh"

TOKEN_NAME="authorized"
CAPABILITY="$TASK_CAPABILITY"
EXPIRES_IN=3600

while [[ $# -gt 0 ]]; do
  case "$1" in
    --token-name) TOKEN_NAME="$2"; shift 2;;
    --capability) CAPABILITY="$2"; shift 2;;
    --expires-in) EXPIRES_IN="$2"; shift 2;;
    *) echo "Unknown arg: $1"; exit 2;;
  esac
 done

"$SCRIPT_DIR/init_runtime.sh"

RUNTIME_DIR="$REPO_ROOT/demo/runtime/$EXECUTOR_NODE_ID"
TOKENS_DIR="$RUNTIME_DIR/tokens"
KEYS_JSON="$RUNTIME_DIR/keys.json"
TOKEN_BIN="$TOKENS_DIR/${TOKEN_NAME}.bin"
TOKEN_META="$TOKENS_DIR/${TOKEN_NAME}.meta.json"

SUBJECT=$(EXECUTOR_NODE_ID="$EXECUTOR_NODE_ID" $PYTHON - <<'PY'
import json
import os
node_id = os.environ["EXECUTOR_NODE_ID"]
with open(f"demo/runtime/{node_id}/keys.json", "r", encoding="utf-8") as h:
    print(json.load(h)["commander_pubkey"])
PY
)

$PYTHON -m src.controller.operator_stub issue-token \
  --keys "$KEYS_JSON" \
  --out "$TOKEN_BIN" \
  --meta-out "$TOKEN_META" \
  --subject "$SUBJECT" \
  --audience "$EXECUTOR_NODE_ID" \
  --capability "$CAPABILITY" \
  --expires-in "$EXPIRES_IN" \
  --allow-mock-signature

echo "[token] $TOKEN_BIN"
