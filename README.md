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
```

## Rust demo (no Python)

This demo uses **Rust-only JSON formats** for tokens and UDP messages. These
formats are stable for the demo but **not compatible** with the Python TLV
implementation.

### Build (WSL2 / Linux)

```bash
cd rust
cargo build --release
```

### Spec mode audience migration

Spec mode now treats `token.audience` as the executor's public key (compressed hex or
32-byte x-only). A key-id derived from the executor pubkey (sha256 of the x-only pubkey)
is also accepted for audience matching. The executor reads `executor_pubkey` from
`demo/config/policy.json` (node_id is now logging/routing only). Re-issue spec tokens
with `--audience <executor_pubkey>` or the derived key-id.

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

### UDP transport debug (echo / ping)

Use this to prove raw UDP round-trip between Jetson and BBB-02 without SCRAP parsing.

On BBB-02 (echo server):
```bash
./rust/target/release/scrap-executor \
  --bind 0.0.0.0 --port 7227 \
  --policy demo/config/policy.json \
  --keys demo/config/keys.json \
  --allow-mock-signatures \
  --debug-echo
```

On Jetson (ping client):
```bash
./rust/target/release/scrap-commander \
  --target-host 192.168.50.32 --target-port 7227 \
  --ping --timeout 5
```

If you need a fixed reply port (firewall rules), add `--bind-port 7331` on the commander.

### Start executor on Jetson (manual)

```bash
./scripts/start_rust_executor.sh
```


## SCRAP-lite no_std demo (CBOR)

This tiered workspace provides a **no_std-ready** edge runtime with a thin Linux
UDP shim. The core protocol logic lives in `crates/scrap-core-lite` and
`crates/scrap-edge` and **does not depend on std**. The orchestrator and Linux
shim are std-only.

### Workspace layout

```
crates/
  scrap-core-lite   # no_std + alloc, CBOR envelope + types
  scrap-edge        # no_std + alloc, routing + token verification
  scrap-linux-udp   # std, UDP IO + route loading + replay cache
bins/
  scrap-node         # std thin wrapper for Jetson/BBB
  scrap-orchestrator # std orchestrator on laptop
```

### CBOR message envelope (map keys)

Envelope (CBOR map):
- `0` version
- `1` msg_type
- `2` trace_id (bytes, 16)
- `3` src (text)
- `4` dst (text)
- `5` hop_limit (u8)
- `6` payload (map)

Payload types:
- TaskRequest (`msg_type=1`):
  - `0` token (map)
  - `1` command (text)
  - `2` args (text)
  - `3` reply_to (text)
  - `4` commander_pubkey (text)
- TaskResult (`msg_type=2`):
  - `0` status (u8)
  - `1` output_digest (bytes)
  - `2` telemetry (map)
- TaskRejected (`msg_type=3`):
  - `0` reason (text)
  - `1` details (array text)

Token map:
- `0` token_id (bytes, 16)
- `1` subject (text)
- `2` audience (text)
- `3` capability (text)
- `4` issued_at (u64)
- `5` expires_at (u64)

Telemetry map:
- `0` duration_ms (u32)
- `1` node_id (text)

### Route table format

`inventory/routes.json` (static next-hop map):

```json
{
  "nodes": {
    "ORCH": {
      "routes": {
        "JETSON-A": "192.168.50.10:7227",
        "BBB-01": "192.168.50.10:7227"
      }
    },
    "JETSON-A": {
      "routes": {
        "BBB-01": "192.168.50.31:7227",
        "ORCH": "192.168.50.1:7331"
      }
    },
    "BBB-01": {
      "routes": {
        "ORCH": "192.168.50.1:7331"
      }
    }
  }
}
```

### Build (WSL2 / Linux)

```bash
# full workspace
cargo build --release

# prove no_std for core
cargo build -p scrap-core-lite --target thumbv7em-none-eabi
cargo build -p scrap-edge --target thumbv7em-none-eabi
```

### Cross-compile (from WSL2)

```bash
rustup target add aarch64-unknown-linux-gnu armv7-unknown-linux-gnueabihf

cargo build -p scrap-node --release --target aarch64-unknown-linux-gnu
cargo build -p scrap-node --release --target armv7-unknown-linux-gnueabihf
```

> You may need `aarch64-linux-gnu-gcc` and `arm-linux-gnueabihf-gcc` on WSL2
> for linking.

### Run scrap-node (Jetson / BBB)

```bash
./scrap-node \
  --node-id JETSON-A \
  --bind 0.0.0.0 \
  --port 7227 \
  --routes inventory/routes.json \
  --replay-cache demo/runtime/replay_cache.json \
  --revoked demo/config/revoked.json \
  --commander-pubkey <hex> \
  --allow-mock-signatures
```

### Run orchestrator (Laptop)

```bash
./scrap-orchestrator \
  --node-id ORCH \
  --bind 0.0.0.0 \
  --port 7331 \
  --routes inventory/routes.json \
  --target BBB-01 \
  --keys demo/config/keys.json \
  --command demo.hash \
  --args 123 \
  --timeout 10
```

### Smoke test

```bash
./tests/smoke/run.sh
```

### Keys file (dev)

Create `demo/config/keys.json` from the template and set `commander_pubkey`:

```json
{
  "commander_pubkey": "DEV-COMMANDER"
}
```

Other fields in the template are ignored by the no_std demo.

### Running as a service

See `deploy/systemd/` for unit files and install steps for Jetson and BBB.
Copy the unit + config, then enable/start:

```bash
sudo install -m 0755 scrap-node /usr/local/bin/scrap-node
sudo install -m 0644 deploy/systemd/scrap-node.service /etc/systemd/system/scrap-node.service

sudo systemctl daemon-reload
sudo systemctl enable --now scrap-node
sudo systemctl status scrap-node --no-pager
```
## Running as a service (systemd)

See `deploy/systemd/README.md` for installation instructions and an example `/etc/scrap/node.json`.
