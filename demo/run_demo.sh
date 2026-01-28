#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

if [[ $# -lt 1 ]]; then
  echo "Usage: $0 <scenario>" >&2
  echo "Scenarios: 01_authorized 02_unauthorized 03_revoked 04_replay" >&2
  exit 2
fi

case "$1" in
  01_authorized) exec "$SCRIPT_DIR/scenarios/01_authorized.sh" ;;
  02_unauthorized) exec "$SCRIPT_DIR/scenarios/02_unauthorized.sh" ;;
  03_revoked) exec "$SCRIPT_DIR/scenarios/03_revoked.sh" ;;
  04_replay) exec "$SCRIPT_DIR/scenarios/04_replay.sh" ;;
  *) echo "Unknown scenario: $1" >&2; exit 2 ;;
 esac
