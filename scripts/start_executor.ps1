param(
  [string]$ConfigPath
)

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$repoRoot = Resolve-Path (Join-Path $scriptDir "..")
$cfg = & (Join-Path $scriptDir "lab_env.ps1") -ConfigPath $ConfigPath

& (Join-Path $scriptDir "init_runtime.ps1") -ConfigPath $ConfigPath | Out-Null

$nodeId = $cfg["EXECUTOR_NODE_ID"]
$python = $cfg["PYTHON"]
$runtimeDir = Join-Path $repoRoot "demo\runtime\$nodeId"
$keysPath = Join-Path $runtimeDir "keys.json"
$policyPath = Join-Path $runtimeDir "policy.json"
$revokedPath = Join-Path $runtimeDir "revoked.json"

$executorHost = $cfg["EXECUTOR_HOST"]
$executorUser = $cfg["EXECUTOR_USER"]
$executorPort = $cfg["EXECUTOR_PORT"]
$executorBind = $cfg["EXECUTOR_BIND"]
$repoDir = $cfg["REPO_DIR"]

if ($executorHost -eq "127.0.0.1" -or $executorHost -eq "localhost") {
  $logPath = Join-Path $runtimeDir "executor.log"
  Push-Location $repoRoot
  try {
    Start-Process $python "-m src.node.executor --bind $executorBind --port $executorPort --keys $keysPath --policy $policyPath" \
      -NoNewWindow -RedirectStandardOutput $logPath -RedirectStandardError $logPath
  } finally {
    Pop-Location
  }
  Write-Output "[executor] started locally (log: $logPath)"
  exit 0
}

if (-not $executorUser) { throw "EXECUTOR_USER missing" }

$remoteRuntime = "$repoDir/demo/runtime/$nodeId"
$remoteLog = "$remoteRuntime/executor.log"

& ssh "$executorUser@$executorHost" "mkdir -p $remoteRuntime"
& scp $keysPath $policyPath $revokedPath "$executorUser@$executorHost:$remoteRuntime/"
& ssh "$executorUser@$executorHost" "cd $repoDir && nohup $python -m src.node.executor --bind $executorBind --port $executorPort --keys $remoteRuntime/keys.json --policy $remoteRuntime/policy.json > $remoteLog 2>&1 &"

Write-Output "[executor] started on $executorHost (log: $remoteLog)"
