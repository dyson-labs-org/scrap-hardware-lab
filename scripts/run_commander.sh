#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$REPO_ROOT"

# Ensure repo root is on sys.path
export PYTHONPATH="$REPO_ROOT"

exec /usr/bin/python3 -m src.node.commander "$@"
