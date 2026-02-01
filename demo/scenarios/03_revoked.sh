#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
source "$REPO_ROOT/scripts/lab_env.sh"

: "${EXECUTOR_NODE_ID:=EXECUTOR}"
: "${TASK_ID:=IMG-001}"
: "${TASK_CAPABILITY:=cmd:imaging:msi}"

# Issue a token JSON for the demo
"$REPO_ROOT/scripts/issue_token_rust.sh" --token-name revoked

# Ensure the Rust executor is running (systemd/binary)
"$REPO_ROOT/scripts/ensure_rust_executor.sh"

# Single source of truth: policy dictates where revocations live
POLICY_PATH="${SCRAP_POLICY:-$REPO_ROOT/demo/config/policy.json}"
[[ -f "$POLICY_PATH" ]] || { echo "Missing policy.json: $POLICY_PATH" >&2; exit 2; }

REVOCATIONS="$(python3 - "$POLICY_PATH" <<'PY'
import json,sys
p=json.load(open(sys.argv[1],"r",encoding="utf-8"))
path=p.get("revocation_list_path","")
if not path:
  raise SystemExit("policy missing revocation_list_path")
print(path)
PY
)"
mkdir -p "$(dirname "$REVOCATIONS")"
[[ -f "$REVOCATIONS" ]] || echo "[]" > "$REVOCATIONS"

# Token is written under runtime/<EXECUTOR_NODE_ID>/tokens by issue_token_rust.sh
TOKEN_JSON="$REPO_ROOT/demo/runtime/$EXECUTOR_NODE_ID/tokens/revoked.json"
[[ -f "$TOKEN_JSON" ]] || { echo "Missing token json: $TOKEN_JSON" >&2; exit 2; }

# Revoke the token id (string form) in the policy-defined revocations file
python3 - "$TOKEN_JSON" "$REVOCATIONS" <<'PY'
import json,sys
tok_path, rev_path = sys.argv[1], sys.argv[2]
t=json.load(open(tok_path,"r",encoding="utf-8"))
tid=t.get("token_id","")
if not tid:
    raise SystemExit("token missing token_id")
rev=json.load(open(rev_path,"r",encoding="utf-8"))
if not isinstance(rev,list):
    raise SystemExit("revoked.json must be a list")
if tid not in rev:
    rev.append(tid)
open(rev_path,"w",encoding="utf-8").write(json.dumps(rev,indent=2,sort_keys=True)+"\n")
print("[revoke] wrote:", rev_path)
print("[revoke] added token_id:", tid)
PY

# Keys used by commander side should match what the running executor expects
KEYS_JSON="${SCRAP_KEYS:-$REPO_ROOT/demo/config/keys.json}"
[[ -f "$KEYS_JSON" ]] || { echo "Missing keys.json: $KEYS_JSON" >&2; exit 2; }

OUT_DIR="$REPO_ROOT/demo/runtime/$EXECUTOR_NODE_ID"
mkdir -p "$OUT_DIR"
OUT_LOG="$OUT_DIR/scenario_03_revoked.jsonl"
: > "$OUT_LOG"

echo "[scenario] writing: $OUT_LOG"

"$REPO_ROOT/scripts/run_commander_rust.sh" \
  --target-host 127.0.0.1 \
  --target-port 7227 \
  --token "$TOKEN_JSON" \
  --keys "$KEYS_JSON" \
  --task-id "${TASK_ID}-REVOKED" \
  --requested-capability "$TASK_CAPABILITY" \
  --allow-mock-signatures 2>&1 | tee -a "$OUT_LOG"

# Assertions: must reject and include 'token revoked'
if ! grep -q '"event":"task_rejected"' "$OUT_LOG"; then
  echo "FAIL: expected task_rejected, but did not see it" >&2
  exit 1
fi
if ! grep -q 'token revoked' "$OUT_LOG"; then
  echo "FAIL: expected 'token revoked' detail, but did not see it" >&2
  exit 1
fi

echo "PASS: revoked token was rejected"
"$REPO_ROOT/scripts/collect_logs.sh"
