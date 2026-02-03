#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

BIN="${SCRAP_COMMANDER_BIN:-$REPO_ROOT/rust/target/release/scrap-commander}"

if [[ ! -x "$BIN" ]]; then
  echo "Missing scrap-commander binary: $BIN" >&2
  echo "Build it with: (cd rust && cargo build --release -p scrap-commander)" >&2
  exit 2
fi

exec "$BIN" "$@"
