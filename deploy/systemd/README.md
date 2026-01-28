# SCRAP node systemd (Jetson + BBB)

This service runs the Rust node binary using a config file:
`/usr/local/bin/scrap-node --config /etc/scrap/node.json`

For demo stability, the unit runs as `root`. This avoids permission issues when
writing the replay cache in the lab environment.

## Install

```bash
sudo install -m 0755 scrap-node /usr/local/bin/scrap-node
sudo install -m 0644 deploy/systemd/scrap-node.service /etc/systemd/system/scrap-node.service
sudo install -d /etc/scrap
sudo install -m 0644 /path/to/node.json /etc/scrap/node.json

sudo systemctl daemon-reload
sudo systemctl enable --now scrap-node
sudo systemctl status scrap-node --no-pager
```

## Example config

`/etc/scrap/node.json`:
```json
{
  "node_id": "JETSON-A",
  "bind": "0.0.0.0",
  "port": 7227,
  "routes_path": "/opt/scrap-hardware-lab/inventory/routes.json",
  "replay_cache_path": "/opt/scrap-hardware-lab/demo/runtime/replay_cache.json",
  "revoked_path": "/opt/scrap-hardware-lab/demo/config/revoked.json",
  "commander_pubkey": "DEV-COMMANDER",
  "allow_mock_signatures": true
}
```
