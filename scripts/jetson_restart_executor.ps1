param(
  [string]$JetsonHost = "192.168.50.10",
  [int]$Port = 7227
)

$ErrorActionPreference = "Stop"
$JETSON = "jetson@$JetsonHost"

# Kill any existing executor
ssh $JETSON "pkill -f 'src.node.executor' || true"

# Start clean, unbuffered, logging to file
ssh $JETSON "cd ~/scrap-demo && mkdir -p demo/runtime && : > demo/runtime/executor.log && nohup env PYTHONPATH=. python3 -u -m src.node.executor --bind 0.0.0.0 --port $Port --policy demo/config/policy.json --keys demo/config/keys.json --allow-mock-signatures >> demo/runtime/executor.log 2>&1 &"

Start-Sleep -Milliseconds 300

# Show status
ssh $JETSON "pgrep -af 'src.node.executor' || true; ss -lunp | grep ':$Port' || true; tail -n 50 ~/scrap-demo/demo/runtime/executor.log || true"
