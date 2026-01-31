#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

# Ensure repo root is on sys.path
export PYTHONPATH="$REPO_ROOT"

# Optional debug: SCRAP_DEBUG=1 scripts/run_commander.sh ...
if [[ "${SCRAP_DEBUG:-0}" == "1" ]]; then
  echo "=== run_commander debug ==="
  echo "whoami: $(whoami)"
  echo "pwd: $(pwd)"
  echo "python3: $(command -v python3 || true)"
  python3 - <<'PY'
import os, sys, importlib
print("sys.executable:", sys.executable)
print("cwd:", os.getcwd())
m = importlib.import_module("src.node.commander")
print("commander module file:", m.__file__)
print("sys.path[0:6]:", sys.path[:6])
PY
  echo "=== end debug ==="
fi

PYTHON_BIN="${PYTHON:-/usr/bin/python3}"
exec "$PYTHON_BIN" -m src.node.commander "$@"
