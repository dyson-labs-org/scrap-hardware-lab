# SCRAP Hardware Lab – Bring-Up Notes (Switch-First Topology)

## Current Topology (Authoritative)
- All SBCs connect via a **dumb Ethernet switch**
- Single Layer-2 subnet: **`192.168.50.0/24`**
- **Ethernet is the authoritative control/data plane**
- USB networking is **recovery-only**
- Wi-Fi may be present temporarily for provisioning, but is **not relied upon**
- Laptop acts as **orchestrator / control plane**, not a router

---

## Deterministic Addressing (Switch Fabric)

| Node | Role | Interface | IP |
|----|----|----|----|
| Laptop | Orchestrator | Ethernet | `192.168.50.1` |
| **Pi-A** | Controller | `eth0` | `192.168.50.11` |
| **Jetson-A** | Compute | `enP8p1s0` | `192.168.50.10` |
| **BBB-01** | Constrained node | `eth0` | `192.168.50.31` |
| **BBB-02** | Constrained node | `eth0` | `192.168.50.32` |

> IPs are **static and explicit**. No DHCP is required for SCRAP operation.

---

## Known-Good Facts (Current)

### Pi-A
- **Hostname:** `pi-a`
- **User:** `pi`
- **Ethernet IP:** `192.168.50.11/24` (static)
- Reachable via switch:
  - `ping 192.168.50.11`
  - `ssh pi@192.168.50.11`
- SSH enabled and stable over Ethernet
- Wi-Fi disabled (or scheduled to be disabled) to prevent ambiguity

---

### Jetson-A
- **Hostname:** `ubuntu`
- **User:** `jetson`
- **Ethernet interface:** `enP8p1s0`
- **Ethernet IP:** `192.168.50.10/24` (static, NetworkManager-managed)
- NetworkManager connection:
  - Name: `scrap-switch`
  - `ipv4.method manual`
  - `ipv4.never-default yes`
- Reachable via switch:
  - `ping 192.168.50.10`
  - `ssh jetson@192.168.50.10`
- SSH daemon confirmed listening on `0.0.0.0:22` and `::22`
- Wi-Fi (`wlP1p1s0`) may remain enabled **temporarily** for:
  - package installation
  - updates
  - recovery
- USB device mode (`l4tbr0`, `usb0`, `usb1`) preserved **for recovery only**

---

### BBB-01
- **Hostname:** `bbb-01`
- **User:** `debian`
- **Ethernet interface:** `eth0`
- **Ethernet IP:** `192.168.50.31/24` (static)
- Persistent config via `/etc/network/interfaces`:
  ```ini
  auto eth0
  iface eth0 inet static
      address 192.168.50.31
      netmask 255.255.255.0

  source /etc/network/interfaces.d/*
