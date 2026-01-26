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
