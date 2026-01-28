#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

CONFIG_FILE="${CONFIG_FILE:-$REPO_ROOT/demo/config/demo.env}"
if [[ ! -f "$CONFIG_FILE" ]]; then
  CONFIG_FILE="$REPO_ROOT/demo/config/demo.env.template"
fi

# shellcheck disable=SC1090
source "$CONFIG_FILE"

EXECUTOR_HOST=${EXECUTOR_HOST:-127.0.0.1}
EXECUTOR_PORT=${EXECUTOR_PORT:-7227}
EXECUTOR_NODE_ID=${EXECUTOR_NODE_ID:-EXECUTOR}
EXECUTOR_BIND=${EXECUTOR_BIND:-$EXECUTOR_HOST}
PYTHON=${PYTHON:-python3}
REPO_DIR=${REPO_DIR:-~/scrap-hardware-lab}

TASK_ID=${TASK_ID:-IMG-001}
TASK_CAPABILITY=${TASK_CAPABILITY:-cmd:imaging:msi}
TASK_TYPE=${TASK_TYPE:-imaging}
MAX_AMOUNT_SATS=${MAX_AMOUNT_SATS:-22000}
