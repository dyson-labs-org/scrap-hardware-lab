#!/usr/bin/env bash
set -euo pipefail

# Ensure rust executor is listening on localhost:7227 (or override via env)
HOST="${SCRAP_BIND:-127.0.0.1}"
PORT="${SCRAP_PORT:-7227}"

if ss -lun | grep -q ":$PORT"; then
  exit 0
fi

echo "[SCRAP demo] Executor not listening on UDP :$PORT (expected rust systemd service scrap-demo.service)." >&2
echo "[SCRAP demo] Ask the operator/menu to restart: systemctl restart scrap-demo.service" >&2
exit 1
