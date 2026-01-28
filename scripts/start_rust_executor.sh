#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

BIN_PATH="${RUST_EXECUTOR_BIN:-$REPO_ROOT/rust/target/release/scrap-executor}"
BIND_IP="${SCRAP_BIND:-0.0.0.0}"
PORT="${SCRAP_PORT:-7227}"
POLICY_PATH="${SCRAP_POLICY:-$REPO_ROOT/demo/config/policy.json}"
KEYS_PATH="${SCRAP_KEYS:-$REPO_ROOT/demo/config/keys.json}"
ALLOW_MOCK="${ALLOW_MOCK_SIGNATURES:-1}"

if [[ ! -x "$BIN_PATH" ]]; then
  echo "Missing executor binary: $BIN_PATH" >&2
  exit 2
fi

if [[ ! -f "$POLICY_PATH" ]]; then
  echo "Missing policy.json: $POLICY_PATH" >&2
  exit 2
fi

if [[ ! -f "$KEYS_PATH" ]]; then
  echo "Missing keys.json: $KEYS_PATH" >&2
  exit 2
fi

ARGS=("--bind" "$BIND_IP" "--port" "$PORT" "--policy" "$POLICY_PATH" "--keys" "$KEYS_PATH")
if [[ "$ALLOW_MOCK" == "1" ]]; then
  ARGS+=("--allow-mock-signatures")
fi

exec "$BIN_PATH" "${ARGS[@]}"
