# Run the SCRAP Hardware Demo

This demo uses the **switch-era lab topology** (192.168.50.0/24) and runs
SCRAP task requests over UDP (port 7227) between real nodes. It is a **demo
mapping** of SCRAP message fields to JSON for observability; the SCRAP spec
remains authoritative for semantics.

## Prerequisites

- This repository is cloned on the laptop (orchestrator), Jetson-A, and BBB-01/02.
- The laptop can SSH to each node over the switch.
- Python 3 is available on all nodes.

## Configure Keys and Policy

Default demo configs live in:
- `demo/config/keys.json`
- `demo/config/policy.json`

By default, signatures are **mocked** to avoid cryptographic dependencies.
To enable real Schnorr signatures:
1. Install `coincurve` on nodes.
2. Generate keys:
   `python3 -m src.controller.operator_stub gen-keys --out demo/config/keys.json`
3. Set `allow_mock_signatures` to `false` in `demo/config/policy.json`.

## Scenarios

Run from the laptop (orchestrator):

- Authorized request:
  `demo/run_demo.sh 01_authorized`
- Unauthorized (subject mismatch):
  `demo/run_demo.sh 02_unauthorized`
- Revoked token:
  `demo/run_demo.sh 03_revoked`
- Replay detection:
  `demo/run_demo.sh 04_replay`

Each scenario uses:
- Jetson-A as executor
- BBB-01 or BBB-02 as commander

Executor logs on Jetson-A:
`demo/runtime/executor.log`

## Demo Assumptions (explicit)

- Task request/accept/proof messages are JSON over UDP for lab visibility.
- `task_hash` and `in_reply_to` use canonical JSON hashing (documented in code).
- Lightning HTLC steps are **simulated** (payment hash is real; settlement is not).

These are intentional demo simplifications; see `spec/SCRAP.md` for protocol
semantics and authoritative definitions.
