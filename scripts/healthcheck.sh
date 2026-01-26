#!/usr/bin/env bash
set -euo pipefail

# SCRAP Hardware Lab — Healthcheck (Switch-First)
#
# Usage examples:
#   ./healthcheck.sh
#   ./healthcheck.sh --batch
#   JETSON_HOST=192.168.50.10 JETSON_USER=jetson ./healthcheck.sh
#
# Notes:
# - Requires: bash, ssh, ping. Optional: nc (netcat) for TCP/22 probe.
# - If --batch is enabled, ssh will fail fast (no password prompts).

# -----------------------------
# Defaults (edit or override via env)
# -----------------------------
LAPTOP_IP="${LAPTOP_IP:-192.168.50.1}"

PI_HOST="${PI_HOST:-192.168.50.11}"
PI_USER="${PI_USER:-pi}"

JETSON_HOST="${JETSON_HOST:-192.168.50.10}"
JETSON_USER="${JETSON_USER:-jetson}"

BBB01_HOST="${BBB01_HOST:-192.168.50.31}"
BBB01_USER="${BBB01_USER:-debian}"

BBB02_HOST="${BBB02_HOST:-192.168.50.32}"
BBB02_USER="${BBB02_USER:-debian}"

BATCH_MODE=0
TIMEOUT_SEC=6

# -----------------------------
# Args
# -----------------------------
for arg in "${@:-}"; do
  case "$arg" in
    --batch) BATCH_MODE=1 ;;
    --timeout=*) TIMEOUT_SEC="${arg#*=}" ;;
    *) ;;
  esac
done

# -----------------------------
# Helpers
# -----------------------------
have() { command -v "$1" >/dev/null 2>&1; }

ssh_common_args=(
  -o "ConnectTimeout=${TIMEOUT_SEC}"
  -o "ServerAliveInterval=3"
  -o "ServerAliveCountMax=2"
  -o "StrictHostKeyChecking=accept-new"
)
if [[ "$BATCH_MODE" -eq 1 ]]; then
  ssh_common_args+=(-o "BatchMode=yes")
fi

tcp22_open() {
  local host="$1"
  if have nc; then
    nc -z -w "${TIMEOUT_SEC}" "$host" 22 >/dev/null 2>&1
  else
    # Fallback: best-effort check using bash /dev/tcp (may not exist on all shells)
    (echo >/dev/tcp/"$host"/22) >/dev/null 2>&1
  fi
}

ping_ok() {
  local host="$1"
  # macOS uses different ping flags; this works on both:
  ping -c 1 -W 1 "$host" >/dev/null 2>&1 || ping -c 1 -t 1 "$host" >/dev/null 2>&1
}

run_ssh() {
  local user="$1" host="$2" cmd="$3"
  ssh "${ssh_common_args[@]}" "${user}@${host}" "$cmd"
}

ts() { date "+%Y-%m-%d %H:%M:%S"; }

# -----------------------------
# Commands to run on nodes
# -----------------------------
CMD_BASIC='echo OK; hostname; whoami; uptime; (ip -br a || true); (ss -lntp | grep ":22" || true)'

# BBB-friendly extra (lsusb can be slow on some images; keep minimal)
CMD_BBB='echo OK; hostname; whoami; uptime; (ip -br a || true); (ss -lntp | grep ":22" || true)'

# -----------------------------
# Report table
# -----------------------------
printf "\nSCRAP Hardware Lab — Healthcheck (switch-first)\n" 
printf "Time: %s\n\n" "$(ts)"

printf "%-10s %-20s %-15s %-6s %-6s %-s\n" "TARGET" "PATH" "HOST" "PING" "TCP22" "SSH"
printf "%-10s %-20s %-15s %-6s %-6s %-s\n" "------" "----" "----" "----" "-----" "---"

fail=0

check_target() {
  local name="$1" host="$2" user="$3" cmd="$4"

  local path="laptop -> switch"
  local ping_res="no"
  local tcp_res="no"
  local ssh_res="no"
  local notes=""

  if ping_ok "$host"; then
    ping_res="yes"
  else
    notes="ping failed"
  fi

  if tcp22_open "$host"; then
    tcp_res="yes"
  else
    notes="${notes:+$notes; }tcp/22 closed/unreachable"
  fi

  if [[ "$ping_res" == "yes" && "$tcp_res" == "yes" ]]; then
    if out="$(run_ssh "$user" "$host" "$cmd" 2>&1)"; then
      ssh_res="yes"
      notes="ok"
      # Store details in temp vars for later print
      DETAILS["$name"]="$out"
    else
      ssh_res="no"
      notes="ssh failed: ${out##*$'\n'}"
      DETAILS["$name"]="$out"
      fail=1
    fi
  else
    ssh_res="no"
    DETAILS["$name"]="$notes"
    fail=1
  fi

  printf "%-10s %-20s %-15s %-6s %-6s %-s\n" "$name" "$path" "$host" "$ping_res" "$tcp_res" "$ssh_res"
}

declare -A DETAILS

check_target "pi-a"     "$PI_HOST"    "$PI_USER"    "$CMD_BASIC"
check_target "jetson-a" "$JETSON_HOST" "$JETSON_USER" "$CMD_BASIC"
check_target "bbb-01"   "$BBB01_HOST" "$BBB01_USER" "$CMD_BBB"
check_target "bbb-02"   "$BBB02_HOST" "$BBB02_USER" "$CMD_BBB"

printf "\nDetails:\n"
for k in "pi-a" "jetson-a" "bbb-01" "bbb-02"; do
  printf "\n[%s]\n%s\n" "$k" "${DETAILS[$k]}"
done

if [[ "$fail" -ne 0 ]]; then
  printf "\nHEALTHCHECK: FAIL\n" >&2
  exit 1
else
  printf "\nHEALTHCHECK: PASS\n"
  exit 0
fi
#!/usr/bin/env bash
set -euo pipefail

# SCRAP Hardware Lab — Healthcheck (Switch-First)
#
# Usage examples:
#   ./healthcheck.sh
#   ./healthcheck.sh --batch
#   JETSON_HOST=192.168.50.10 JETSON_USER=jetson ./healthcheck.sh
#
# Notes:
# - Requires: bash, ssh, ping. Optional: nc (netcat) for TCP/22 probe.
# - If --batch is enabled, ssh will fail fast (no password prompts).

# -----------------------------
# Defaults (edit or override via env)
# -----------------------------
LAPTOP_IP="${LAPTOP_IP:-192.168.50.1}"

PI_HOST="${PI_HOST:-192.168.50.11}"
PI_USER="${PI_USER:-pi}"

JETSON_HOST="${JETSON_HOST:-192.168.50.10}"
JETSON_USER="${JETSON_USER:-jetson}"

BBB01_HOST="${BBB01_HOST:-192.168.50.31}"
BBB01_USER="${BBB01_USER:-debian}"

BBB02_HOST="${BBB02_HOST:-192.168.50.32}"
BBB02_USER="${BBB02_USER:-debian}"

BATCH_MODE=0
TIMEOUT_SEC=6

# -----------------------------
# Args
# -----------------------------
for arg in "${@:-}"; do
  case "$arg" in
    --batch) BATCH_MODE=1 ;;
    --timeout=*) TIMEOUT_SEC="${arg#*=}" ;;
    *) ;;
  esac
done

# -----------------------------
# Helpers
# -----------------------------
have() { command -v "$1" >/dev/null 2>&1; }

ssh_common_args=(
  -o "ConnectTimeout=${TIMEOUT_SEC}"
  -o "ServerAliveInterval=3"
  -o "ServerAliveCountMax=2"
  -o "StrictHostKeyChecking=accept-new"
)
if [[ "$BATCH_MODE" -eq 1 ]]; then
  ssh_common_args+=(-o "BatchMode=yes")
fi

tcp22_open() {
  local host="$1"
  if have nc; then
    nc -z -w "${TIMEOUT_SEC}" "$host" 22 >/dev/null 2>&1
  else
    # Fallback: best-effort check using bash /dev/tcp (may not exist on all shells)
    (echo >/dev/tcp/"$host"/22) >/dev/null 2>&1
  fi
}

ping_ok() {
  local host="$1"
  # macOS uses different ping flags; this works on both:
  ping -c 1 -W 1 "$host" >/dev/null 2>&1 || ping -c 1 -t 1 "$host" >/dev/null 2>&1
}

run_ssh() {
  local user="$1" host="$2" cmd="$3"
  ssh "${ssh_common_args[@]}" "${user}@${host}" "$cmd"
}

ts() { date "+%Y-%m-%d %H:%M:%S"; }

# -----------------------------
# Commands to run on nodes
# -----------------------------
CMD_BASIC='echo OK; hostname; whoami; uptime; (ip -br a || true); (ss -lntp | grep ":22" || true)'

# BBB-friendly extra (lsusb can be slow on some images; keep minimal)
CMD_BBB='echo OK; hostname; whoami; uptime; (ip -br a || true); (ss -lntp | grep ":22" || true)'

# -----------------------------
# Report table
# -----------------------------
printf "\nSCRAP Hardware Lab — Healthcheck (switch-first)\n" 
printf "Time: %s\n\n" "$(ts)"

printf "%-10s %-20s %-15s %-6s %-6s %-s\n" "TARGET" "PATH" "HOST" "PING" "TCP22" "SSH"
printf "%-10s %-20s %-15s %-6s %-6s %-s\n" "------" "----" "----" "----" "-----" "---"

fail=0

check_target() {
  local name="$1" host="$2" user="$3" cmd="$4"

  local path="laptop -> switch"
  local ping_res="no"
  local tcp_res="no"
  local ssh_res="no"
  local notes=""

  if ping_ok "$host"; then
    ping_res="yes"
  else
    notes="ping failed"
  fi

  if tcp22_open "$host"; then
    tcp_res="yes"
  else
    notes="${notes:+$notes; }tcp/22 closed/unreachable"
  fi

  if [[ "$ping_res" == "yes" && "$tcp_res" == "yes" ]]; then
    if out="$(run_ssh "$user" "$host" "$cmd" 2>&1)"; then
      ssh_res="yes"
      notes="ok"
      # Store details in temp vars for later print
      DETAILS["$name"]="$out"
    else
      ssh_res="no"
      notes="ssh failed: ${out##*$'\n'}"
      DETAILS["$name"]="$out"
      fail=1
    fi
  else
    ssh_res="no"
    DETAILS["$name"]="$notes"
    fail=1
  fi

  printf "%-10s %-20s %-15s %-6s %-6s %-s\n" "$name" "$path" "$host" "$ping_res" "$tcp_res" "$ssh_res"
}

declare -A DETAILS

check_target "pi-a"     "$PI_HOST"    "$PI_USER"    "$CMD_BASIC"
check_target "jetson-a" "$JETSON_HOST" "$JETSON_USER" "$CMD_BASIC"
check_target "bbb-01"   "$BBB01_HOST" "$BBB01_USER" "$CMD_BBB"
check_target "bbb-02"   "$BBB02_HOST" "$BBB02_USER" "$CMD_BBB"

printf "\nDetails:\n"
for k in "pi-a" "jetson-a" "bbb-01" "bbb-02"; do
  printf "\n[%s]\n%s\n" "$k" "${DETAILS[$k]}"
done

if [[ "$fail" -ne 0 ]]; then
  printf "\nHEALTHCHECK: FAIL\n" >&2
  exit 1
else
  printf "\nHEALTHCHECK: PASS\n"
  exit 0
fi
