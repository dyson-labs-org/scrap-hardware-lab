#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
source "$REPO_ROOT/scripts/lab_env.sh"

TOKEN_NAME="revoked"
while [[ $# -gt 0 ]]; do
  case "$1" in
    --token-name) TOKEN_NAME="$2"; shift 2;;
    *) echo "Unknown arg: $1" >&2; exit 2;;
  esac
done

RUNTIME_DIR="$REPO_ROOT/demo/runtime/$EXECUTOR_NODE_ID"
TOKEN_JSON="$RUNTIME_DIR/tokens/${TOKEN_NAME}.json"
REVOCATIONS="$RUNTIME_DIR/revoked.json"
mkdir -p "$RUNTIME_DIR"

[[ -f "$TOKEN_JSON" ]] || { echo "Missing token json: $TOKEN_JSON" >&2; exit 2; }
[[ -f "$REVOCATIONS" ]] || echo "[]" > "$REVOCATIONS"

python3 - "$TOKEN_JSON" "$REVOCATIONS" <<'PY'
import json,sys,hashlib
tok_path, rev_path = sys.argv[1], sys.argv[2]
t=json.load(open(tok_path,"r",encoding="utf-8"))

tid = t.get("token_id","")
if not tid:
    raise SystemExit("token missing token_id")

# Add the string token id (always)
to_add = {tid}

# OPTIONAL: add a deterministic hash-of-string fallback (useful if some implementations derive bytes from it)
to_add.add(hashlib.sha256(tid.encode("utf-8")).hexdigest())

rev=json.load(open(rev_path,"r",encoding="utf-8"))
if not isinstance(rev,list):
    raise SystemExit("revoked.json must be a list")

changed = False
for x in sorted(to_add):
    if x not in rev:
        rev.append(x)
        changed = True

open(rev_path,"w",encoding="utf-8").write(json.dumps(rev,indent=2,sort_keys=True)+"\n")

print("[revoke] added entries:" if changed else "[revoke] already present:", ", ".join(sorted(to_add)))
PY
