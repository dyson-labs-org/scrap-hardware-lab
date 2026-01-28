# SCRAP Hardware Lab

This repository contains the **hardware bring-up, networking, and health-check tooling**
for the SCRAP (Secure Capability Routing and Authorization Protocol) hardware lab.

The lab is intentionally designed around **boring, deterministic infrastructure** so
protocol behavior is never confused with transport issues.

---

## Topology: Switch-First (Authoritative)

- All SBCs connect via a **dumb Ethernet switch**
- Single Layer-2 subnet: **`192.168.50.0/24`**
- **Ethernet is the authoritative control & data plane**
- USB networking is **recovery/debug only**
- Wi-Fi may be temporarily enabled for provisioning, but is **never required**
- Laptop acts as **orchestrator / control plane**, not a router

This topology is intentional and permanent.

---

## Deterministic Addressing

| Node | Role | Interface | IP |
|----|----|----|----|
| Laptop | Orchestrator | Ethernet | `192.168.50.1` |
| Pi-A | Controller | `eth0` | `192.168.50.11` |
| Jetson-A | Compute | `enP8p1s0` | `192.168.50.10` |
| BBB-01 | Constrained node | `eth0` | `192.168.50.31` |
| BBB-02 | Constrained node | `eth0` | `192.168.50.32` |

- IPs are **static and explicit**
- No DHCP is required
- Nodes must remain reachable across reboot

---

## Node Configuration Summary

### Pi-A
- Hostname: `pi-a`
- User: `pi`
- Static Ethernet IP
- SSH enabled and stable
- Wi-Fi disabled (or scheduled to be disabled)

### Jetson-A
- Hostname: `ubuntu`
- User: `jetson`
- Static Ethernet IP via **NetworkManager**
- Connection name: `scrap-switch`
- USB device mode preserved **for recovery only**
- Wi-Fi optional during provisioning

### BeagleBone Blacks (BBB-01 / BBB-02)
- OS: Debian Bookworm
- User: `debian`
- Static Ethernet IP via `/etc/network/interfaces`
- USB gadget networking disabled after Ethernet validation

---

## Health Check

`scripts/healthcheck.ps1` verifies:

- TCP/22 reachability
- SSH login to each node
- Basic runtime state:
  - hostname
  - uptime
  - interface status

### Run (PowerShell)

```powershell
.\scripts\healthcheck.ps1

## Rust demo (no Python)

This demo uses **Rust-only JSON formats** for tokens and UDP messages. These
formats are stable for the demo but **not compatible** with the Python TLV
implementation.

### Build (WSL2 / Linux)

```bash
cd rust
cargo build --release
```

### Token JSON format (Rust-only)

```json
{
  "version": 1,
  "token_id": "32hex",
  "subject": "hex string",
  "audience": "JETSON-A",
  "capability": "telemetry.read",
  "issued_at": 1710000000,
  "expires_at": 1710000600,
  "signature": "mock"
}
```

### UDP message JSON format (Rust-only)

Commander -> Executor:
```json
{
  "version": 1,
  "type": "task_request",
  "task_id": "32hex",
  "requested_capability": "telemetry.read",
  "token": { "...": "token json" },
  "commander_pubkey": "hex string",
  "commander_signature": "mock"
}
```

Executor -> Commander (success):
```json
{
  "version": 1,
  "type": "task_accepted",
  "task_id": "32hex",
  "payment_hash": "64hex"
}
```

Executor -> Commander (proof):
```json
{
  "version": 1,
  "type": "proof",
  "task_id": "32hex",
  "proof_hash": "64hex"
}
```

Executor -> Commander (reject):
```json
{
  "version": 1,
  "type": "task_rejected",
  "task_id": "32hex",
  "details": ["..."],
  "notes": ["signature verification skipped (mock mode)"]
}
```

### Deterministic hashes

- `payment_hash = sha256(task_id || token_id || "payment")`
- `proof_hash   = sha256(task_id || payment_hash || "proof")`

### Smoke run (Laptop/WSL2 -> Jetson)

```bash
./scripts/rust_smoke.sh
```

This script will:
- Build Rust binaries
- Copy `scrap-executor` + config JSON to Jetson
- Start executor on Jetson
- Issue a token with `scrap-operator`
- Send a request with `scrap-commander`
- Pull back `demo/runtime/executor.log`

### Start executor on Jetson (manual)

```bash
./scripts/start_rust_executor.sh
```

