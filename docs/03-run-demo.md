# Run the SCRAP Hardware Demo

This demo uses the switch-era lab topology (192.168.50.0/24) and runs SCRAP
TaskRequest/Accept/Proof messages over UDP (port 7227) between real nodes.

The demo maps SCRAP fields to JSON for observability; protocol semantics remain
per spec.

## Prerequisites

- Repo cloned on the laptop and the executor node.
- Laptop can SSH to the executor over the switch.
- Python 3 available on laptop + executor.

## Configure (one-time)

1) Copy the template env:
   `copy demo\config\demo.env.template demo\config\demo.env`

2) Edit demo/config/demo.env with the executor host/user and node_id.

## Smoke Run (PowerShell, laptop)

`powershell -ExecutionPolicy Bypass -File scripts\smoke.ps1`

## Notes

- Signatures are mocked (no external crypto dependencies).
- The payment layer is simulated (payment_hash is real; settlement is not).
- Logs are written under demo/runtime/<node_id>/ on the laptop.
