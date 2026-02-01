#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
source "$REPO_ROOT/scripts/lab_env.sh"

: "${EXECUTOR_NODE_ID:=EXECUTOR}"
: "${TASK_ID:=IMG-001}"
: "${TASK_CAPABILITY:=cmd:imaging:msi}"

"$REPO_ROOT/scripts/issue_token_rust.sh" --token-name authorized
"$REPO_ROOT/scripts/ensure_rust_executor.sh"

OUT_DIR="$REPO_ROOT/demo/runtime/$EXECUTOR_NODE_ID"
mkdir -p "$OUT_DIR"

TOKEN_JSON="$OUT_DIR/tokens/authorized.json"
[[ -f "$TOKEN_JSON" ]] || { echo "ERROR: missing token: $TOKEN_JSON" >&2; exit 2; }

CANON_KEYS="${SCRAP_KEYS:-$REPO_ROOT/demo/config/keys.json}"
[[ -f "$CANON_KEYS" ]] || { echo "ERROR: Missing keys.json: $CANON_KEYS" >&2; exit 2; }

ATTACKER_KEYS="$OUT_DIR/keys_attacker.json"
OUT_LOG="$OUT_DIR/scenario_02_unauthorized.jsonl"
: > "$OUT_LOG"

python3 - "$CANON_KEYS" "$ATTACKER_KEYS" <<'PY' | tee -a "$OUT_LOG"
import json,sys
src=sys.argv[1]; dst=sys.argv[2]
k=json.load(open(src,"r",encoding="utf-8"))
k["commander_pubkey"]="DEAD"*16
open(dst,"w",encoding="utf-8").write(json.dumps(k,indent=2,sort_keys=True)+"\n")
print("[scenario] wrote attacker keys:", dst)
PY

echo "[scenario] writing: $OUT_LOG"

"$REPO_ROOT/scripts/run_commander_rust.sh" \
  --target-host 127.0.0.1 \
  --target-port 7227 \
  --token "$TOKEN_JSON" \
  --keys "$ATTACKER_KEYS" \
  --task-id "${TASK_ID}-UNAUTH" \
  --requested-capability "$TASK_CAPABILITY" \
  --allow-mock-signatures 2>&1 | tee -a "$OUT_LOG"

# Assertions: must reject (and ideally for the right reason)
if ! grep -q '"event":"task_rejected"' "$OUT_LOG"; then
  echo "FAIL: expected task_rejected, but did not see it" >&2
  exit 1
fi
if ! grep -q 'token subject does not match commander_pubkey' "$OUT_LOG"; then
  echo "FAIL: expected commander pubkey mismatch reason, but did not see it" >&2
  exit 1
fi

echo "PASS: unauthorized commander identity was rejected"
"$REPO_ROOT/scripts/collect_logs.sh"
