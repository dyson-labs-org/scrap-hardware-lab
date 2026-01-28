#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
source "$REPO_ROOT/scripts/lab_env.sh"

"$REPO_ROOT/scripts/issue_token.sh" --token-name authorized
"$REPO_ROOT/scripts/start_executor.sh"

RUNTIME_DIR="$REPO_ROOT/demo/runtime/$EXECUTOR_NODE_ID"
ATTACKER_KEYS="$RUNTIME_DIR/keys_attacker.json"

RUNTIME_DIR="$RUNTIME_DIR" $PYTHON - <<'PY'
import json
import os
runtime = os.environ["RUNTIME_DIR"]
keys_path = os.path.join(runtime, "keys.json")
attacker_path = os.path.join(runtime, "keys_attacker.json")
with open(keys_path, "r", encoding="utf-8") as h:
    keys = json.load(h)
keys["commander_pubkey"] = "DEAD" * 16
with open(attacker_path, "w", encoding="utf-8") as h:
    json.dump(keys, h, indent=2, sort_keys=True)
PY

"$REPO_ROOT/scripts/run_commander.sh" --token-name authorized --task-id "${TASK_ID}-UNAUTH" --keys "$ATTACKER_KEYS"
"$REPO_ROOT/scripts/collect_logs.sh"
