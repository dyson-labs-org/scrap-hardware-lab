#!/usr/bin/env bash
set -euo pipefail
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib.sh"

ok=0; fail=0

section(){ echo -e "\n\033[36m== $* ==\033[0m"; }
pass(){ echo -e "\033[32mPASS\033[0m $*"; ok=$((ok+1)); }
bad(){  echo -e "\033[31mFAIL\033[0m $*"; fail=$((fail+1)); }

# -----------------------------------------------------------------------------
# SWITCH-ERA HEALTHCHECK (SCRAP hardware lab)
#
# Assumptions:
# - All nodes are on the same L2 segment via the switch (or direct Ethernet).
# - Orchestrator (your laptop) has an IP on 192.168.50.0/24.
# - No USB networking, no jump hosts.
#
# Override anything via env vars:
#   PI_A_HOST=192.168.50.11 PI_A_USER=pi ./healthcheck.sh
#   JETSON_A_HOST=192.168.50.21 JETSON_A_USER=ubuntu ./healthcheck.sh
# -----------------------------------------------------------------------------

# --- Node config (defaults) ---
: "${PI_A_HOST:=192.168.50.11}"
: "${PI_A_USER:=pi}"

# Add these as you bring them online; leave blank to skip.
: "${JETSON_A_HOST:=}"   # e.g. 192.168.50.21
: "${JETSON_A_USER:=ubuntu}"

: "${BBB01_HOST:=}"      # e.g. 192.168.50.31
: "${BBB01_USER:=debian}"

: "${BBB02_HOST:=}"      # e.g. 192.168.50.32
: "${BBB02_USER:=debian}"

# Optional: set the orchestrator interface explicitly (recommended on laptops)
: "${ORCH_IFACE:=}"      # e.g. eth0, enp0s31f6

# Ping/SSH timeouts (tune if needed)
: "${PING_TIMEOUT_SEC:=1}"
: "${SSH_TIMEOUT_SEC:=6}"

# --- Helpers ---
run_node() {
  local label="$1" user="$2" host="$3" cmd="$4"
  if out="$(ssh_run "${user}@${host}" "$cmd" 2>&1)"; then
    pass "$label (${user}@${host})"
    echo "$out"
  else
    bad "$label (${user}@${host})"
    echo "$out"
  fi
}

ping_check() {
  local label="$1" host="$2"
  if ping -c 1 -W "${PING_TIMEOUT_SEC}" "$host" >/dev/null 2>&1; then
    pass "ping $label ($host)"
  else
    bad "ping $label ($host)"
  fi
}

detect_iface() {
  if [[ -n "${ORCH_IFACE}" ]]; then
    echo "${ORCH_IFACE}"
    return
  fi

  # Heuristic: prefer an interface that has a 192.168.50.* address
  local iface
  iface="$(ip -o -4 addr show 2>/dev/null | awk '$4 ~ /^192\.168\.50\./ {print $2; exit}' || true)"
  if [[ -n "$iface" ]]; then
    echo "$iface"
    return
  fi

  # Fallback: default route interface
  iface="$(ip route get 1.1.1.1 2>/dev/null | awk '{for(i=1;i<=NF;i++) if($i=="dev"){print $(i+1); exit}}' || true)"
  echo "$iface"
}

orch_on_lab_subnet() {
  local iface="$1"
  ip -o -4 addr show dev "$iface" 2>/dev/null | grep -qE 'inet 192\.168\.50\.' || return 1
}

# Build node list (skip blanks)
declare -a NODE_LABELS=()
declare -a NODE_USERS=()
declare -a NODE_HOSTS=()

add_node() {
  local label="$1" user="$2" host="$3"
  [[ -n "$host" ]] || return 0
  NODE_LABELS+=("$label")
  NODE_USERS+=("$user")
  NODE_HOSTS+=("$host")
}

add_node "pi-a"    "$PI_A_USER"    "$PI_A_HOST"
add_node "jetson-a" "$JETSON_A_USER" "$JETSON_A_HOST"
add_node "bbb-01"  "$BBB01_USER"   "$BBB01_HOST"
add_node "bbb-02"  "$BBB02_USER"   "$BBB02_HOST"

# --- Orchestrator checks ---
section "Orchestrator (laptop) network"
iface="$(detect_iface)"
if [[ -z "$iface" ]]; then
  bad "could not detect active interface; set ORCH_IFACE=eth0 (or similar)"
else
  ip -br addr show dev "$iface" || true
  if orch_on_lab_subnet "$iface"; then
    pass "orchestrator iface=$iface on 192.168.50.0/24"
  else
    bad "orchestrator iface=$iface is NOT on 192.168.50.0/24 (expected for switch-era lab)"
    echo "Hint: set static IP like 192.168.50.1/24 on the interface used to reach the switch."
  fi
fi

section "LAN neighbor view (sanity)"
ip neigh show nud reachable,stale,delay,probe 2>/dev/null | head -n 40 || true

# --- Reachability ---
section "Reachability (ping)"
if [[ "${#NODE_HOSTS[@]}" -eq 0 ]]; then
  bad "no nodes configured (set PI_A_HOST / JETSON_A_HOST / BBB01_HOST / BBB02_HOST)"
else
  for i in "${!NODE_HOSTS[@]}"; do
    ping_check "${NODE_LABELS[$i]}" "${NODE_HOSTS[$i]}"
  done
fi

# --- SSH identity ---
section "SSH + basic identity"
for i in "${!NODE_HOSTS[@]}"; do
  run_node "${NODE_LABELS[$i]}" "${NODE_USERS[$i]}" "${NODE_HOSTS[$i]}" \
    'echo OK; hostname; hostname -I; uptime'
done

# --- Device-specific checks ---
section "Device-specific checks"
for i in "${!NODE_HOSTS[@]}"; do
  label="${NODE_LABELS[$i]}"
  user="${NODE_USERS[$i]}"
  host="${NODE_HOSTS[$i]}"

  case "$label" in
    pi-a|pi-*)
      # Lightweight checks that don't assume packages
      run_node "$label os"  "$user" "$host" 'uname -a; cat /etc/os-release | head -n 5'
      run_node "$label eth" "$user" "$host" 'ip -br link show eth0; ip -br addr show eth0'
      ;;
    bbb-01|bbb-02|bbb-*|*bbb*)
      run_node "$label usb" "$user" "$host" 'lsusb | head -n 8'
      run_node "$label bcf" "$user" "$host" 'lsusb | grep -i -E "BeagleConnect|Texas Instruments" || true'
      ;;
    jetson-a|jetson-*|*jetson*)
      run_node "$label sys" "$user" "$host" 'uname -a; cat /etc/nv_tegra_release 2>/dev/null || true'
      run_node "$label pwr" "$user" "$host" 'nvpmodel -q 2>/dev/null || true'
      ;;
    *)
      run_node "$label sys" "$user" "$host" 'uname -a'
      ;;
  esac
done

section "Summary"
echo "OK: $ok  FAIL: $fail"
if [[ "$fail" -gt 0 ]]; then
  echo -e "\n\033[31mHEALTHCHECK: FAIL\033[0m"
  exit 1
else
  echo -e "\n\033[32mHEALTHCHECK: PASS\033[0m"
  exit 0
fi
