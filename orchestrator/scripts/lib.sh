#!/usr/bin/env bash
set -euo pipefail

# Load config
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
CONFIG_FILE="${CONFIG_FILE:-$REPO_ROOT/orchestrator/config/demo.env}"

if [[ -f "$CONFIG_FILE" ]]; then
  # shellcheck disable=SC1090
  source "$CONFIG_FILE"
else
  echo "Missing config: $CONFIG_FILE" >&2
  exit 2
fi

SSH_OPTS=(
  -o ConnectTimeout=5
  -o ServerAliveInterval=3
  -o ServerAliveCountMax=2
  -o StrictHostKeyChecking=accept-new
)

if [[ "${BATCHMODE:-0}" == "1" ]]; then
  SSH_OPTS+=(-o BatchMode=yes)
fi

ssh_run() {
  local target="$1"; shift
  ssh "${SSH_OPTS[@]}" "$target" "$@"
}

ssh_jump() {
  local jump="$1"; shift
  local target="$1"; shift
  ssh "${SSH_OPTS[@]}" -J "$jump" "$target" "$@"
}
