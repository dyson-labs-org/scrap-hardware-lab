#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
source "$REPO_ROOT/orchestrator/scripts/lib.sh"

PYTHON=${PYTHON:-python3}
REPO_DIR=${REPO_DIR:-~/scrap-hardware-lab}
NODE_ID=${NODE_ID:-JETSON-A}

TOKEN_DIR="$REPO_ROOT/demo/config/tokens"
mkdir -p "$TOKEN_DIR"
TOKEN_BIN="$TOKEN_DIR/revoked.bin"
TOKEN_META="$TOKEN_DIR/revoked.json"
REVOKE_LIST="$REPO_ROOT/demo/config/revoked.json"

SUBJECT=$($PYTHON - <<'PY'
import json
with open("demo/config/keys.json", "r", encoding="utf-8") as h:
    print(json.load(h)["commander_pubkey"])
PY
)

# Issue token
$PYTHON -m src.controller.operator_stub issue-token \
  --keys demo/config/keys.json \
  --out "$TOKEN_BIN" \
  --meta-out "$TOKEN_META" \
  --subject "$SUBJECT" \
  --audience "$NODE_ID" \
  --capability "cmd:imaging:msi" \
  --allow-mock-signature

TOKEN_ID=$($PYTHON - <<'PY'
import json
with open("demo/config/tokens/revoked.json", "r", encoding="utf-8") as h:
    print(json.load(h)["token_id"])
PY
)

$PYTHON -m src.controller.operator_stub revoke --revocation-list "$REVOKE_LIST" --token-id "$TOKEN_ID"

echo "[stage] Copying config + token + revocation list to Jetson and BBB-01"
scp "${SSH_OPTS[@]}" demo/config/keys.json demo/config/policy.json "$REVOKE_LIST" \
  "$JETSON_USER@$JETSON_HOST:$REPO_DIR/demo/config/"
ssh_run "$JETSON_USER@$JETSON_HOST" "mkdir -p $REPO_DIR/demo/config/tokens $REPO_DIR/demo/runtime"
scp "${SSH_OPTS[@]}" "$TOKEN_BIN" "$BBB01_USER@$BBB01_HOST:$REPO_DIR/demo/config/tokens/revoked.bin"
scp "${SSH_OPTS[@]}" demo/config/keys.json "$BBB01_USER@$BBB01_HOST:$REPO_DIR/demo/config/"

# Start executor in background
ssh_run "$JETSON_USER@$JETSON_HOST" "cd $REPO_DIR && nohup $PYTHON -m src.node.executor --keys demo/config/keys.json --policy demo/config/policy.json > demo/runtime/executor.log 2>&1 &"

sleep 1

echo "[run] Sending revoked task request from BBB-01"
ssh_run "$BBB01_USER@$BBB01_HOST" "cd $REPO_DIR && $PYTHON -m src.node.commander \
  --target-host $JETSON_HOST \
  --token demo/config/tokens/revoked.bin \
  --keys demo/config/keys.json \
  --task-id IMG-003 \
  --requested-capability cmd:imaging:msi \
  --allow-mock-signatures"

echo "[log] Executor log on Jetson: $REPO_DIR/demo/runtime/executor.log"
