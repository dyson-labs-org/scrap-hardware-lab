#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

JETSON_HOST="${JETSON_HOST:-192.168.50.10}"
JETSON_USER="${JETSON_USER:-jetson}"
JETSON_REPO_DIR="${JETSON_REPO_DIR:-~/scrap-hardware-lab}"
EXECUTOR_NODE_ID="${EXECUTOR_NODE_ID:-JETSON-A}"
EXECUTOR_PORT="${EXECUTOR_PORT:-7227}"

TOKEN_PATH="$REPO_ROOT/demo/config/token.json"
TOKEN_META="$REPO_ROOT/demo/config/token.meta.json"
KEYS_PATH="$REPO_ROOT/demo/config/keys.json"
POLICY_PATH="$REPO_ROOT/demo/config/policy.json"
REVOKED_PATH="$REPO_ROOT/demo/config/revoked.json"

mkdir -p "$REPO_ROOT/demo/config"

if [[ ! -f "$KEYS_PATH" ]]; then
  cat > "$KEYS_PATH" <<'JSON'
{
  "operator_privkey": "11",
  "operator_pubkey": "11",
  "commander_privkey": "22",
  "commander_pubkey": "22",
  "executor_privkey": "33",
  "executor_pubkey": "33"
}
JSON
fi

if [[ ! -f "$POLICY_PATH" ]]; then
  cat > "$POLICY_PATH" <<JSON
{
  "node_id": "$EXECUTOR_NODE_ID",
  "replay_cache_path": "demo/runtime/replay_cache.json",
  "revocation_list_path": "demo/config/revoked.json"
}
JSON
fi

if [[ ! -f "$REVOKED_PATH" ]]; then
  echo "[]" > "$REVOKED_PATH"
fi

pushd "$REPO_ROOT/rust" >/dev/null
cargo build --release
popd >/dev/null

scp "$REPO_ROOT/rust/target/release/scrap-executor" "$JETSON_USER@$JETSON_HOST:$JETSON_REPO_DIR/rust/target/release/scrap-executor"
scp "$REPO_ROOT/demo/config/keys.json" "$REPO_ROOT/demo/config/policy.json" "$REPO_ROOT/demo/config/revoked.json" \
  "$JETSON_USER@$JETSON_HOST:$JETSON_REPO_DIR/demo/config/"
scp "$REPO_ROOT/scripts/start_rust_executor.sh" "$JETSON_USER@$JETSON_HOST:$JETSON_REPO_DIR/scripts/"

ssh "$JETSON_USER@$JETSON_HOST" "chmod +x $JETSON_REPO_DIR/scripts/start_rust_executor.sh"
ssh "$JETSON_USER@$JETSON_HOST" "mkdir -p $JETSON_REPO_DIR/demo/runtime"
ssh "$JETSON_USER@$JETSON_HOST" "nohup $JETSON_REPO_DIR/scripts/start_rust_executor.sh > $JETSON_REPO_DIR/demo/runtime/executor.log 2>&1 &"

"$REPO_ROOT/rust/target/release/scrap-operator" issue-token \
  --keys "$KEYS_PATH" \
  --out "$TOKEN_PATH" \
  --meta-out "$TOKEN_META" \
  --subject "22" \
  --audience "$EXECUTOR_NODE_ID" \
  --capability "telemetry.read" \
  --expires-in 600 \
  --token-id "aabbccddeeff00112233445566778899" \
  --allow-mock-signature

"$REPO_ROOT/rust/target/release/scrap-commander" \
  --target-host "$JETSON_HOST" \
  --target-port "$EXECUTOR_PORT" \
  --token "$TOKEN_PATH" \
  --keys "$KEYS_PATH" \
  --task-id "00112233445566778899aabbccddeeff" \
  --requested-capability "telemetry.read" \
  --allow-mock-signatures \
  --timeout 10

scp "$JETSON_USER@$JETSON_HOST:$JETSON_REPO_DIR/demo/runtime/executor.log" "$REPO_ROOT/demo/runtime/executor.log" || true

