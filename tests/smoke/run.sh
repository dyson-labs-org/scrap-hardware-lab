#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

BIN="$REPO_ROOT/target/release/scrap-orchestrator"
if [[ ! -x "$BIN" ]]; then
  (cd "$REPO_ROOT" && cargo build --release -p scrap-orchestrator)
fi

NODE_ID="${SCRAP_ORCH_NODE_ID:-ORCH}"
TARGET="${SCRAP_TARGET_NODE:-BBB-01}"
ROUTES="${SCRAP_ROUTES:-inventory/routes.json}"
KEYS="${SCRAP_KEYS:-demo/config/keys.json}"
COMMAND="${SCRAP_COMMAND:-demo.hash}"
ARGS="${SCRAP_ARGS:-123}"
TIMEOUT="${SCRAP_TIMEOUT:-10}"
BIND="${SCRAP_BIND:-0.0.0.0}"
PORT="${SCRAP_PORT:-7331}"

run_orch() {
  set +e
  OUTPUT=$(
    "$BIN" \
      --node-id "$NODE_ID" \
      --bind "$BIND" \
      --port "$PORT" \
      --routes "$ROUTES" \
      --target "$TARGET" \
      --keys "$KEYS" \
      --command "$COMMAND" \
      --args "$ARGS" \
      --timeout "$TIMEOUT" \
      "$@"
  )
  STATUS=$?
  set -e
}

extract_trace() {
  echo "$OUTPUT" | grep -E "\"trace_id\"" | head -n 1 | sed -E 's/.*"trace_id":"([a-f0-9]+)".*/\1/'
}

run_orch
TRACE_ID=$(extract_trace)
if [[ "$STATUS" -ne 0 ]]; then
  echo "FAIL happy trace_id=$TRACE_ID"
  echo "$OUTPUT"
  exit 1
fi
echo "PASS happy trace_id=$TRACE_ID"

run_orch --token-audience "INVALID-AUDIENCE"
TRACE_ID=$(extract_trace)
if [[ "$STATUS" -eq 1 ]]; then
  echo "PASS reject trace_id=$TRACE_ID"
  exit 0
fi
if [[ "$STATUS" -eq 3 ]]; then
  echo "PASS trace_mismatch trace_id=$TRACE_ID"
  exit 0
fi

echo "FAIL negative trace_id=$TRACE_ID status=$STATUS"
echo "$OUTPUT"
exit 1
