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
TOKEN_BIN="$TOKEN_DIR/replay.bin"

SUBJECT=$($PYTHON - <<'PY'
import json
with open("demo/config/keys.json", "r", encoding="utf-8") as h:
    print(json.load(h)["commander_pubkey"])
PY
)

$PYTHON -m src.controller.operator_stub issue-token \
  --keys demo/config/keys.json \
  --out "$TOKEN_BIN" \
  --subject "$SUBJECT" \
  --audience "$NODE_ID" \
  --capability "cmd:imaging:msi" \
  --allow-mock-signature

scp "${SSH_OPTS[@]}" demo/config/keys.json demo/config/policy.json "$TOKEN_BIN" \
  "$JETSON_USER@$JETSON_HOST:$REPO_DIR/demo/config/"
ssh_run "$JETSON_USER@$JETSON_HOST" "mkdir -p $REPO_DIR/demo/config/tokens $REPO_DIR/demo/runtime"
scp "${SSH_OPTS[@]}" "$TOKEN_BIN" "$BBB01_USER@$BBB01_HOST:$REPO_DIR/demo/config/tokens/replay.bin"
scp "${SSH_OPTS[@]}" demo/config/keys.json "$BBB01_USER@$BBB01_HOST:$REPO_DIR/demo/config/"

ssh_run "$JETSON_USER@$JETSON_HOST" "cd $REPO_DIR && nohup $PYTHON -m src.node.executor --keys demo/config/keys.json --policy demo/config/policy.json > demo/runtime/executor.log 2>&1 &"

sleep 1

echo "[run] First request (should accept)"
ssh_run "$BBB01_USER@$BBB01_HOST" "cd $REPO_DIR && $PYTHON -m src.node.commander \
  --target-host $JETSON_HOST \
  --token demo/config/tokens/replay.bin \
  --keys demo/config/keys.json \
  --task-id IMG-004 \
  --requested-capability cmd:imaging:msi \
  --allow-mock-signatures"

sleep 1

echo "[run] Second request with same token (should reject as replay)"
ssh_run "$BBB01_USER@$BBB01_HOST" "cd $REPO_DIR && $PYTHON -m src.node.commander \
  --target-host $JETSON_HOST \
  --token demo/config/tokens/replay.bin \
  --keys demo/config/keys.json \
  --task-id IMG-004B \
  --requested-capability cmd:imaging:msi \
  --allow-mock-signatures"

echo "[log] Executor log on Jetson: $REPO_DIR/demo/runtime/executor.log"
