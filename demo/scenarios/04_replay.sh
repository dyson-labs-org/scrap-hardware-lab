#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
source "$REPO_ROOT/scripts/lab_env.sh"

: "${EXECUTOR_NODE_ID:=EXECUTOR}"
: "${TASK_ID:=demo-task-001}"
: "${TASK_CAPABILITY:=demo:authorized}"

"$REPO_ROOT/scripts/issue_token_rust.sh" --token-name replay
"$REPO_ROOT/scripts/ensure_rust_executor.sh"

OUT_DIR="$REPO_ROOT/demo/runtime/$EXECUTOR_NODE_ID"
TOKEN_JSON="$OUT_DIR/tokens/replay.json"
KEYS_JSON="${SCRAP_KEYS:-$REPO_ROOT/demo/config/keys.json}"
OUT_LOG="$OUT_DIR/scenario_04_replay.jsonl"
: > "$OUT_LOG"

echo "[scenario] writing: $OUT_LOG"

# First attempt (should accept)
"$REPO_ROOT/scripts/run_commander_rust.sh" \
  --target-host 127.0.0.1 \
  --target-port 7227 \
  --token "$TOKEN_JSON" \
  --keys "$KEYS_JSON" \
  --task-id "${TASK_ID}-REPLAY-A" \
  --requested-capability "$TASK_CAPABILITY" \
  --allow-mock-signatures 2>&1 | tee -a "$OUT_LOG"

# Second attempt with SAME token (should reject if replay cache is working)
"$REPO_ROOT/scripts/run_commander_rust.sh" \
  --target-host 127.0.0.1 \
  --target-port 7227 \
  --token "$TOKEN_JSON" \
  --keys "$KEYS_JSON" \
  --task-id "${TASK_ID}-REPLAY-B" \
  --requested-capability "$TASK_CAPABILITY" \
  --allow-mock-signatures 2>&1 | tee -a "$OUT_LOG"

# Assertions: must see at least one accept and at least one reject
if ! grep -q '"event":"task_accepted"' "$OUT_LOG"; then
  echo "FAIL: expected a task_accepted (first run), but did not see it" >&2
  exit 1
fi
if ! grep -q '"event":"task_rejected"' "$OUT_LOG"; then
  echo "FAIL: expected a task_rejected (replay), but did not see it" >&2
  exit 1
fi

echo "PASS: replay behavior produced reject on second submission"
"$REPO_ROOT/scripts/collect_logs.sh"
