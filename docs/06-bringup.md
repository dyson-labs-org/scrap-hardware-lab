# SCRAP Hardware Lab – Bring-Up Notes

## Known-good facts
- Jetson user: `jetson`
- Jetson hostname: `ubuntu`
- Jetson USB gadget IP: `192.168.55.1`
- Laptop USB gadget IP: `192.168.55.100`
- BBB USB gadget IPs: `192.168.7.2`, `192.168.6.2` (depending on port)

## Observed issues
- USB-C Ethernet link to Jetson is unstable under some conditions
- SSH interactive works; command-mode SSH sometimes resets
- System is Ubuntu 22.04 LTS (minimized image)

## Mitigations to apply next session
- Disable Windows USB Ethernet power saving
- Force MTU = 1500 on Jetson USB bridge
- Add non-interactive guard to ~/.bashrc
- Install SSH keys everywhere; set BATCHMODE=1

## Next steps
- Stabilize Jetson USB networking
- Finalize demo.env
- Run healthcheck.sh end-to-end
