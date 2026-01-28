#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

"$SCRIPT_DIR/push_code.sh"
"$SCRIPT_DIR/issue_token.sh"
"$SCRIPT_DIR/start_executor.sh"
sleep 1
"$SCRIPT_DIR/run_commander.sh"
"$SCRIPT_DIR/collect_logs.sh"
