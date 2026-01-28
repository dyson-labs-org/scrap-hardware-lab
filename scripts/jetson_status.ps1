param(
  [string]$JetsonHost = "192.168.50.10",
  [int]$Port = 7227
)

$ErrorActionPreference = "Stop"
$JETSON = "jetson@$JetsonHost"

ssh $JETSON "echo '--- pgrep'; pgrep -af 'src.node.executor' || true; echo '--- ss'; ss -lunp | grep ':$Port' || true; echo '--- log'; tail -n 50 ~/scrap-demo/demo/runtime/executor.log 2>/dev/null || true"
