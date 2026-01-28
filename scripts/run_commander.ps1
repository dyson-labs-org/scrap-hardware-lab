param(
  [string]$TokenName = "authorized",
  [string]$TaskId,
  [string]$Capability,
  [string]$KeysPath,
  [string]$ConfigPath
)

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$repoRoot = Resolve-Path (Join-Path $scriptDir "..")
$cfg = & (Join-Path $scriptDir "lab_env.ps1") -ConfigPath $ConfigPath

if (-not $TaskId) { $TaskId = $cfg["TASK_ID"] }
if (-not $Capability) { $Capability = $cfg["TASK_CAPABILITY"] }

$nodeId = $cfg["EXECUTOR_NODE_ID"]
$runtimeDir = Join-Path $repoRoot "demo\runtime\$nodeId"
if (-not $KeysPath) { $KeysPath = Join-Path $runtimeDir "keys.json" }
$tokenBin = Join-Path $runtimeDir "tokens\$TokenName.bin"

if (-not (Test-Path $tokenBin)) {
  throw "Missing token: $tokenBin"
}

$python = $cfg["PYTHON"]
$executorHost = $cfg["EXECUTOR_HOST"]
$executorPort = $cfg["EXECUTOR_PORT"]
$taskType = $cfg["TASK_TYPE"]
$maxAmount = $cfg["MAX_AMOUNT_SATS"]

Push-Location $repoRoot
try {
  & $python -m src.node.commander \
    --target-host $executorHost \
    --target-port $executorPort \
    --token $tokenBin \
    --keys $KeysPath \
    --task-id $TaskId \
    --requested-capability $Capability \
    --task-type $taskType \
    --max-amount-sats $maxAmount \
    --allow-mock-signatures
} finally {
  Pop-Location
}
