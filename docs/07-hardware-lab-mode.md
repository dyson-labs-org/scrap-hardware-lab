# Hardware Lab Mode Deployment Plan

This plan applies to the switch-era lab (192.168.50.0/24) described in
`docs/06-bringup.md`.

## Roles by Node (default)

- Laptop (192.168.50.1): operator + commander + orchestration scripts.
- Jetson-A (192.168.50.10): executor (UDP/7227).
- Pi-A (192.168.50.11): ingress/observer only (no routing).
- BBB-01/02 (192.168.50.31/.32): optional commanders for multi-node demos.

## Two-Node Run Plan (Laptop -> Jetson-A)

1) Configure env on the laptop:

   - `copy demo\config\demo.env.template demo\config\demo.env`
   - Set `EXECUTOR_HOST`, `EXECUTOR_USER`, `EXECUTOR_NODE_ID` for Jetson-A.

2) Push code to Jetson-A:

   - `powershell -ExecutionPolicy Bypass -File scripts\push_code.ps1`

3) Run the smoke loop from the laptop:

   - `powershell -ExecutionPolicy Bypass -File scripts\smoke.ps1`

4) Validate output:

   - Laptop logs: `demo/runtime/JETSON-A/executor.log`
   - Token artifacts: `demo/runtime/JETSON-A/tokens/*.bin`
   - Replay cache: `demo/runtime/JETSON-A/replay_cache.json`

## Multi-Node Extension (optional)

- Push code to BBB-01/02 using the same `scripts\push_code.ps1` with
  `-TargetHost` / `-TargetUser` parameters.
- Start the executor on Jetson-A (same as above).
- Run the commander locally on the laptop or via SSH on a BBB.

## Notes

- Signatures are mocked (no external crypto dependencies).
- Default smoke run uses simulated payment; pay-gated demo uses BTCPay via `scripts/demo_pay_gate.sh`.
- All runtime data is stored under `demo/runtime/<node_id>/` on the laptop.
