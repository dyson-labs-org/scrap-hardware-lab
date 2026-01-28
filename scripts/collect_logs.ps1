param(
  [string]$ConfigPath
)

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$repoRoot = Resolve-Path (Join-Path $scriptDir "..")
$cfg = & (Join-Path $scriptDir "lab_env.ps1") -ConfigPath $ConfigPath

$nodeId = $cfg["EXECUTOR_NODE_ID"]
$runtimeDir = Join-Path $repoRoot "demo\runtime\$nodeId"
New-Item -ItemType Directory -Force $runtimeDir | Out-Null

$executorHost = $cfg["EXECUTOR_HOST"]
if ($executorHost -eq "127.0.0.1" -or $executorHost -eq "localhost") {
  Write-Output "[logs] local: $runtimeDir"
  exit 0
}

$executorUser = $cfg["EXECUTOR_USER"]
$repoDir = $cfg["REPO_DIR"]
$remoteRuntime = "$repoDir/demo/runtime/$nodeId"

& scp "$executorUser@$executorHost:$remoteRuntime/executor.log" $runtimeDir 2>$null
& scp "$executorUser@$executorHost:$remoteRuntime/replay_cache.json" $runtimeDir 2>$null

Write-Output "[logs] collected to $runtimeDir"
