#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
source "$REPO_ROOT/scripts/lab_env.sh"

"$REPO_ROOT/scripts/issue_token.sh" --token-name authorized
"$REPO_ROOT/scripts/start_executor.sh"
"$REPO_ROOT/scripts/run_commander.sh" --token-name authorized --task-id "$TASK_ID"
"$REPO_ROOT/scripts/collect_logs.sh"
