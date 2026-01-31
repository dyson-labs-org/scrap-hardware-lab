#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

if [[ -f "$SCRIPT_DIR/lab_env.sh" ]]; then
  # shellcheck disable=SC1090
  source "$SCRIPT_DIR/lab_env.sh"
fi

MODE="fake"
ARGS=()

usage() {
  cat <<'USAGE' >&2
Usage: scripts/demo_pay_gate.sh (--fake|--real) --usd <amount> [options]

Examples:
  scripts/demo_pay_gate.sh --fake --usd 5 --target-host 127.0.0.1
  scripts/demo_pay_gate.sh --real --usd 25 --btcpay-url https://btcpay.example \
    --btcpay-store-id STORE_ID --btcpay-api-key API_KEY --target-host 192.168.50.10
USAGE
}

has_arg() {
  local key="$1"
  for item in "${ARGS[@]}"; do
    if [[ "$item" == "$key" ]]; then
      return 0
    fi
  done
  return 1
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --real) MODE="real"; shift ;;
    --fake) MODE="fake"; shift ;;
    -h|--help) usage; exit 0 ;;
    *) ARGS+=("$1"); shift ;;
  esac
done

if ! has_arg "--usd"; then
  usage
  exit 2
fi

if ! has_arg "--target-host"; then
  ARGS+=(--target-host "${EXECUTOR_HOST:-127.0.0.1}")
fi
if ! has_arg "--target-port"; then
  ARGS+=(--target-port "${EXECUTOR_PORT:-7227}")
fi
if ! has_arg "--token"; then
  ARGS+=(--token "${DEMO_TOKEN_PATH:-demo/config/token.json}")
fi
if ! has_arg "--keys"; then
  ARGS+=(--keys "${DEMO_KEYS_PATH:-demo/config/keys.json}")
fi
if ! has_arg "--requested-capability"; then
  ARGS+=(--requested-capability "${TASK_CAPABILITY:-telemetry.read}")
fi
if ! has_arg "--task-id"; then
  ARGS+=(--task-id "${TASK_ID:-DEMO-TASK-01}")
fi

export PYTHONPATH="$REPO_ROOT"
PYTHON_BIN="${PYTHON:-/usr/bin/python3}"

exec "$PYTHON_BIN" -m src.controller.settlement_bridge "--${MODE}" "${ARGS[@]}"
