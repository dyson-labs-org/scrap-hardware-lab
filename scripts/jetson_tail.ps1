param(
  [string]$JetsonHost = "192.168.50.10",
  [int]$Lines = 200
)

$ErrorActionPreference = "Stop"
$JETSON = "jetson@$JetsonHost"

ssh $JETSON "tail -n $Lines ~/scrap-demo/demo/runtime/executor.log 2>/dev/null || true"
