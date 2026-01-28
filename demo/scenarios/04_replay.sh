#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
source "$REPO_ROOT/scripts/lab_env.sh"

"$REPO_ROOT/scripts/issue_token.sh" --token-name replay
"$REPO_ROOT/scripts/start_executor.sh"

"$REPO_ROOT/scripts/run_commander.sh" --token-name replay --task-id "${TASK_ID}-A"
"$REPO_ROOT/scripts/run_commander.sh" --token-name replay --task-id "${TASK_ID}-B"

"$REPO_ROOT/scripts/collect_logs.sh"
