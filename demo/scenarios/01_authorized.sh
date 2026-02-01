#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
source "$REPO_ROOT/scripts/lab_env.sh"

# --- Phase 4 demo hardening: allow headless execution ---
: "${EXECUTOR_NODE_ID:=EXECUTOR}"
: "${TASK_ID:=demo-task-001}"
: "${TASK_CAPABILITY:=demo:authorized}"

if [[ -z "${EXECUTOR_NODE_ID}" ]]; then
  echo "ERROR: EXECUTOR_NODE_ID not set (lab_env.sh incomplete)" >&2
  exit 2
fi

# Issue token + ensure executor is up
"$REPO_ROOT/scripts/issue_token_rust.sh" --token-name authorized
"$REPO_ROOT/scripts/ensure_rust_executor.sh"

TOKEN_JSON="$REPO_ROOT/demo/runtime/$EXECUTOR_NODE_ID/tokens/authorized.json"
[[ -f "$TOKEN_JSON" ]] || { echo "ERROR: missing token: $TOKEN_JSON" >&2; exit 2; }

# Phase 4 hardening: keep a single source of truth for commander identity.
# The running executor is launched with demo/config/keys.json (see demo/runtime/executor.log).
# Allow override via SCRAP_KEYS if you want to test alternate key material.
KEYS_JSON="${SCRAP_KEYS:-$REPO_ROOT/demo/config/keys.json}"
[[ -f "$KEYS_JSON" ]] || { echo "ERROR: Missing keys.json: $KEYS_JSON" >&2; exit 2; }

OUT_DIR="$REPO_ROOT/demo/runtime/$EXECUTOR_NODE_ID"
mkdir -p "$OUT_DIR"
OUT_LOG="$OUT_DIR/scenario_01_authorized.jsonl"
: > "$OUT_LOG"

echo "[scenario] writing: $OUT_LOG"

"$REPO_ROOT/scripts/run_commander_rust.sh" \
  --target-host 127.0.0.1 \
  --target-port 7227 \
  --token "$TOKEN_JSON" \
  --keys "$KEYS_JSON" \
  --task-id "$TASK_ID" \
  --requested-capability "$TASK_CAPABILITY" \
  --allow-mock-signatures 2>&1 | tee -a "$OUT_LOG"

# --- Assertions: scenario must ACCEPT and produce PROOF, and must NOT REJECT ---
if grep -q '"event":"task_rejected"' "$OUT_LOG"; then
  echo "FAIL: task_rejected detected" >&2
  exit 1
fi
if ! grep -q '"event":"task_accepted"' "$OUT_LOG"; then
  echo "FAIL: missing task_accepted" >&2
  exit 1
fi
if ! grep -q '"event":"proof_received"' "$OUT_LOG"; then
  echo "FAIL: missing proof_received" >&2
  exit 1
fi

echo "PASS: authorized task accepted and proof received"

"$REPO_ROOT/scripts/collect_logs.sh"
