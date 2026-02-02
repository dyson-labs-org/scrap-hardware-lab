# SCRAP Hardware Lab

This repository contains the **hardware bring-up, networking, and health-check tooling**
for the SCRAP (Secure Capability Routing and Authorization Protocol) hardware lab.

The lab is intentionally designed around **boring, deterministic infrastructure** so
protocol behavior is never confused with transport issues.

## Quickstart: Run the Live SCRAP Demo (2–3 minutes)

This demo lets you trigger real SCRAP authorization flows against **real hardware executors**
from a shared demo environment.

You do **not** need hardware, keys, or a local build.

### What you are doing

- You SSH into a demo VPS operated by Dyson Labs
- You run preconfigured demo scenarios as a **demo commander**
- Those scenarios authorize (or reject) execution on real SBC hardware
- Results are observable via emitted JSON event logs

This is a **correctness and security semantics demo**, not a production deployment.

### Step 1: SSH into the demo VPS

```bash
ssh demo@170.75.161.148
```
---
Once connected, enter the demo environment
```bash

```
---

### Step 2: Once connected, enter the demo environment (creates an invoice)

```bash
enter_lab
```
---
Once settled, the session become **READY**

### Step 3: Run an authorized execution

```bash
demo/scenarios/01_authorized.sh
```
---

### Step 4: Try a failure case

```bash
demo/scenarios/02_unauthorized.sh
```
---

### Step 5: Observe the results

```bash
ls demo/runtime/JETSON-A

tail -n 50 demo/runtime/JETSON-A/scenario_01_authorized.jsonl
tail -n 50 demo/runtime/JETSON-A/scenario_02_unauthorized.jsonl
```
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

### Payments & Settlement (BTCPay demo)

**Flow boundaries**
- Operator-side (online): runs the settlement bridge, holds BTCPay credentials, creates invoices, and decides when to release execution.
- Executor-side (offline/limited): receives `task_request`, waits for `payment_lock`, and never talks to BTCPay.
  - The pay-gated demo uses the Rust JSON UDP path (task_request + payment_lock).

**Lifecycle mapping (Requested -> LockedAcked -> Claimed)**
- Requested: BTCPay invoice created (task request issued).
- LockedAcked: invoice status is paid enough for the demo (rule: `Paid` is sufficient).
- Claimed: proof received and verified against the deterministic `proof_hash`.

**Demo flow**
1) Task request sent to executor (no execution yet).
2) Operator prints invoice URL.
3) Customer pays.
4) Operator sends `payment_lock` after BTCPay is paid.
5) Executor executes and sends proof.
6) Operator verifies proof, records claim, and prints `DEMO SUCCESS`.

**Trust assumptions**
- Operator is online and trusted to map BTCPay status to the lock.
- Executors are offline/limited and never hold BTCPay credentials.

**CLI examples**
- Fake (offline dev):
  `./scripts/demo_pay_gate.sh --fake --usd 5 --target-host 127.0.0.1`
- Real (VPS demo):
  `./scripts/demo_pay_gate.sh --real --usd 25 --btcpay-url https://btcpay.example --btcpay-store-id STORE_ID --btcpay-api-key API_KEY --target-host 192.168.50.10`
- Real config can also be provided via env (`BTCPAY_URL`, `BTCPAY_STORE_ID`, `BTCPAY_API_KEY`) or `--btcpay-config demo/config/btcpay.json` (gitignored).
- Operator settlement state persists locally in `demo/runtime/settlement.json`.

**What this demo proves**
- Execution is gated on real payment confirmation.
- Proof is cryptographically bound to the paid task via deterministic hashes.

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

## Roadmap / To-Do (Hardware Demo)

This repository is a hands-on execution and validation lab for SCRAP across real hardware
(Jetson, Raspberry Pi, BeagleBone + BCFs, Bitaxe). The goal is not feature completeness,
but to demonstrate **authorization → execution → proof → settlement** across heterogeneous
devices.

---

### 🧠 SCRAP Core (Protocol Correctness)

**Goal:** Ensure the demo faithfully represents the SCRAP specification when run in `--mode spec`.

- [ ] Verify `--mode spec` enforces spec-level validation (token structure, ordering, required fields)
- [ ] Clearly document behavioral differences between `demo` and `spec` modes
- [ ] Add at least one negative test (invalid token → executor must reject)
- [ ] Ensure Proof-of-Execution messages match spec fields exactly
- [ ] Confirm spec mode works across Jetson, Pi, and BBB (no arch-specific shortcuts)

---

### 💸 Payments & Settlement (BTCPay Server)

**Goal:** Demonstrate that SCRAP can gate execution on real economic settlement.

- [ ] Define payment flow boundaries (on-device vs operator-side)
- [ ] Integrate BTCPay Server as an external operator service
- [ ] Implement payment lifecycle mapping:
  - Payment requested
  - Payment locked / acknowledged
  - Payment claimed after proof
- [ ] Map BTCPay events → SCRAP `SettlementState`
- [ ] Demo flow: task request → payment → execution → proof → settlement
- [ ] Document trust assumptions (online operator, offline executors)
- [X] Added outbound reverse SSH tunnel to allow external access to lab services (prerequisite for BTCPay integration).
---

### 🔌 BCF Modules (Hardware Attestation)

**Goal:** Turn BCFs into cryptographic executors with verifiable proof-of-action.

- [ ] Inventory current BCF capabilities (MCU, interfaces, storage)
- [ ] Define BCF role within SCRAP execution
- [ ] Implement SCRAP-lite on BCFs (BIP-340 Schnorr only, no token parsing)
- [ ] Design BCF attestation format:
  - Task ID / nonce
  - Command hash
  - Optional monotonic counter or hash chain
- [ ] Have BBB verify BCF proof and embed it in Proof-of-Execution
- [ ] Demo: BCF executes a physical action and signs a receipt

---

### ⚡ Bitaxe Integration (Real Work Payload)

**Goal:** Show SCRAP gating real compute and energy usage.

- [ ] Define Bitaxe control surface (start/stop hashing, throttle)
- [ ] Decide control path (direct from BBB or via BCF)
- [ ] Gate Bitaxe activity on valid SCRAP tasks
- [ ] Optionally expose metrics (hashrate snapshot, power draw)
- [ ] Demo: paid task causes real hash work for a bounded interval
- [ ] Document what Bitaxe execution proves (economic load, not trustless mining)

---

### 🧪 Demo Infrastructure & Ops

**Goal:** Make the demo repeatable and safe for external users.

- [ ] Single-command startup per device
- [ ] Consistent config layout (`~/scrap-demo-config`)
- [ ] Time-handling policy documented (what requires real time vs logical time)
- [ ] Optional SSH tunnel instructions (via VM)
- [ ] Network topology diagram (even ASCII)
- [ ] Structured JSON logging with correlation IDs

---

### 📖 Documentation & Narrative

**Goal:** Make the demo understandable to engineers, reviewers, and partners.

- [ ] One-page explanation: “What this demo proves”
- [ ] Diagram: Operator ↔ Executor ↔ BCF ↔ Payload
- [ ] Explicit callouts for:
  - What is real
  - What is mocked
  - What is intentionally out of scope









