#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
source "$REPO_ROOT/scripts/lab_env.sh"

TOKEN_NAME="authorized"
TTL_SECS="${TOKEN_TTL_SECS:-600}"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --token-name) TOKEN_NAME="$2"; shift 2;;
    --ttl-secs) TTL_SECS="$2"; shift 2;;
    *) echo "Unknown arg: $1" >&2; exit 2;;
  esac
done

RUNTIME_DIR="$REPO_ROOT/demo/runtime/$EXECUTOR_NODE_ID"
TOK_DIR="$RUNTIME_DIR/tokens"
mkdir -p "$TOK_DIR"

OUT="$TOK_DIR/${TOKEN_NAME}.json"

NOW="$(date +%s)"
EXP="$((NOW + TTL_SECS))"

CAPABILITY="${TASK_CAPABILITY:-imaging}"
TOKEN_ID="${TOKEN_NAME}-${TASK_ID}-${NOW}"
SIG="mock"

KEYS_PATH="${SCRAP_KEYS:-$REPO_ROOT/demo/config/keys.json}"
[[ -f "$KEYS_PATH" ]] || { echo "Missing keys.json: $KEYS_PATH" >&2; exit 2; }

POLICY_PATH="$RUNTIME_DIR/policy.json"
[[ -f "$POLICY_PATH" ]] || POLICY_PATH="$REPO_ROOT/demo/config/policy.json"

read -r COMMANDER_PUB AUDIENCE_NODE < <(python3 - "$KEYS_PATH" "$POLICY_PATH" <<'PY2'
import json,sys
k=json.load(open(sys.argv[1],"r",encoding="utf-8"))
p=json.load(open(sys.argv[2],"r",encoding="utf-8"))
# Emit both values on ONE line; bash `read` consumes a single line.
print(f"{k.get('commander_pubkey','')} {p.get('node_id','')}")
PY2
)

if [[ -z "$COMMANDER_PUB" ]]; then
  echo "keys missing commander_pubkey: $KEYS_PATH" >&2
  exit 2
fi
if [[ -z "$AUDIENCE_NODE" ]]; then
  echo "policy missing node_id: $POLICY_PATH" >&2
  exit 2
fi

cat > "$OUT" <<JSON
{
  "version": 1,
  "token_id": "$TOKEN_ID",
  "subject": "$COMMANDER_PUB",
  "audience": "$AUDIENCE_NODE",
  "capability": "$CAPABILITY",
  "issued_at": $NOW,
  "expires_at": $EXP,
  "signature": "$SIG"
}
JSON

echo "[token] $OUT"
