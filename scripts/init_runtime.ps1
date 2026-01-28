param(
  [string]$ConfigPath
)

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$repoRoot = Resolve-Path (Join-Path $scriptDir "..")
$cfg = & (Join-Path $scriptDir "lab_env.ps1") -ConfigPath $ConfigPath

$nodeId = $cfg["EXECUTOR_NODE_ID"]
$runtimeDir = Join-Path $repoRoot "demo\runtime\$nodeId"
$tokensDir = Join-Path $runtimeDir "tokens"

New-Item -ItemType Directory -Force $tokensDir | Out-Null

$keysPath = Join-Path $runtimeDir "keys.json"
$policyPath = Join-Path $runtimeDir "policy.json"
$revokedPath = Join-Path $runtimeDir "revoked.json"

if (-not (Test-Path $keysPath)) {
  $keys = @{ 
    operator_privkey = ("11" * 32)
    operator_pubkey = ("02" + ("11" * 32))
    commander_privkey = ("22" * 32)
    commander_pubkey = ("02" + ("22" * 32))
    executor_privkey = ("33" * 32)
    executor_pubkey = ("02" + ("33" * 32))
  }
  $keys | ConvertTo-Json -Depth 2 | Set-Content -Path $keysPath
}

$policy = @{
  node_id = $nodeId
  allow_mock_signatures = $true
  require_commander_sig = $false
  replay_cache_path = "demo/runtime/$nodeId/replay_cache.json"
  revocation_list_path = "demo/runtime/$nodeId/revoked.json"
  execute_delay_sec = 2
}
$policy | ConvertTo-Json -Depth 3 | Set-Content -Path $policyPath

if (-not (Test-Path $revokedPath)) {
  "[]" | Set-Content -Path $revokedPath
}

Write-Output "[init] runtime_dir=$runtimeDir"
