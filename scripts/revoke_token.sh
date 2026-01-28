#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
source "$SCRIPT_DIR/lab_env.sh"

TOKEN_NAME="authorized"
while [[ $# -gt 0 ]]; do
  case "$1" in
    --token-name) TOKEN_NAME="$2"; shift 2;;
    *) echo "Unknown arg: $1"; exit 2;;
  esac
 done

"$SCRIPT_DIR/init_runtime.sh" >/dev/null

RUNTIME_DIR="$REPO_ROOT/demo/runtime/$EXECUTOR_NODE_ID"
META_JSON="$RUNTIME_DIR/tokens/${TOKEN_NAME}.meta.json"
REVOKE_LIST="$RUNTIME_DIR/revoked.json"

if [[ ! -f "$META_JSON" ]]; then
  echo "Missing token meta: $META_JSON" >&2
  exit 2
fi

TOKEN_ID=$(META_JSON="$META_JSON" $PYTHON - <<'PY'
import json
import os
meta_path = os.environ["META_JSON"]
with open(meta_path, "r", encoding="utf-8") as h:
    print(json.load(h)["token_id"])
PY
)

$PYTHON -m src.controller.operator_stub revoke \
  --revocation-list "$REVOKE_LIST" \
  --token-id "$TOKEN_ID"

echo "[revoke] token_id=$TOKEN_ID"
