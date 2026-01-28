#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
source "$SCRIPT_DIR/lab_env.sh"

RUNTIME_DIR="$REPO_ROOT/demo/runtime/$EXECUTOR_NODE_ID"
TOKENS_DIR="$RUNTIME_DIR/tokens"
mkdir -p "$TOKENS_DIR"

$PYTHON - <<PY
import json
import os

runtime_dir = "$RUNTIME_DIR"
node_id = "$EXECUTOR_NODE_ID"

keys_path = os.path.join(runtime_dir, "keys.json")
policy_path = os.path.join(runtime_dir, "policy.json")
revoked_path = os.path.join(runtime_dir, "revoked.json")

os.makedirs(runtime_dir, exist_ok=True)

if not os.path.exists(keys_path):
    keys = {
        "operator_privkey": "11" * 32,
        "operator_pubkey": "02" + ("11" * 32),
        "commander_privkey": "22" * 32,
        "commander_pubkey": "02" + ("22" * 32),
        "executor_privkey": "33" * 32,
        "executor_pubkey": "02" + ("33" * 32),
    }
    with open(keys_path, "w", encoding="utf-8") as handle:
        json.dump(keys, handle, indent=2, sort_keys=True)

policy = {
    "node_id": node_id,
    "allow_mock_signatures": True,
    "require_commander_sig": False,
    "replay_cache_path": f"demo/runtime/{node_id}/replay_cache.json",
    "revocation_list_path": f"demo/runtime/{node_id}/revoked.json",
    "execute_delay_sec": 2,
}
with open(policy_path, "w", encoding="utf-8") as handle:
    json.dump(policy, handle, indent=2, sort_keys=True)

if not os.path.exists(revoked_path):
    with open(revoked_path, "w", encoding="utf-8") as handle:
        json.dump([], handle, indent=2)
PY

echo "[init] runtime_dir=$RUNTIME_DIR"
