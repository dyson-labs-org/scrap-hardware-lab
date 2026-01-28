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
TOKEN_BIN="$TOKEN_DIR/authorized.bin"

SUBJECT=$($PYTHON - <<'PY'
import json
with open("demo/config/keys.json", "r", encoding="utf-8") as h:
    print(json.load(h)["commander_pubkey"])
PY
)

# Issue a valid token for the real commander
$PYTHON -m src.controller.operator_stub issue-token \
  --keys demo/config/keys.json \
  --out "$TOKEN_BIN" \
  --subject "$SUBJECT" \
  --audience "$NODE_ID" \
  --capability "cmd:imaging:msi" \
  --allow-mock-signature

# Create attacker keys (mismatched commander_pubkey)
ATTACKER_KEYS="$REPO_ROOT/demo/config/keys_attacker.json"
$PYTHON - <<'PY'
import json
# Minimal keys file for commander CLI
attacker = {"commander_pubkey": "DEAD" * 16}
with open("demo/config/keys_attacker.json", "w", encoding="utf-8") as h:
    json.dump(attacker, h, indent=2)
PY

# Copy configs and token
scp "${SSH_OPTS[@]}" demo/config/keys.json demo/config/policy.json "$TOKEN_BIN" \
  "$JETSON_USER@$JETSON_HOST:$REPO_DIR/demo/config/"
ssh_run "$JETSON_USER@$JETSON_HOST" "mkdir -p $REPO_DIR/demo/config/tokens $REPO_DIR/demo/runtime"
scp "${SSH_OPTS[@]}" "$TOKEN_BIN" "$BBB02_USER@$BBB02_HOST:$REPO_DIR/demo/config/tokens/authorized.bin"
scp "${SSH_OPTS[@]}" demo/config/keys_attacker.json "$BBB02_USER@$BBB02_HOST:$REPO_DIR/demo/config/keys.json"

# Start executor in background
ssh_run "$JETSON_USER@$JETSON_HOST" "cd $REPO_DIR && nohup $PYTHON -m src.node.executor --keys demo/config/keys.json --policy demo/config/policy.json > demo/runtime/executor.log 2>&1 &"

sleep 1

echo "[run] Sending unauthorized task request from BBB-02 (subject mismatch)"
ssh_run "$BBB02_USER@$BBB02_HOST" "cd $REPO_DIR && $PYTHON -m src.node.commander \
  --target-host $JETSON_HOST \
  --token demo/config/tokens/authorized.bin \
  --keys demo/config/keys.json \
  --task-id IMG-002 \
  --requested-capability cmd:imaging:msi \
  --allow-mock-signatures"

echo "[log] Executor log on Jetson: $REPO_DIR/demo/runtime/executor.log"
