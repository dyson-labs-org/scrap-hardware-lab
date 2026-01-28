param(
  [string]$TokenName = "authorized",
  [string]$ConfigPath
)

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$repoRoot = Resolve-Path (Join-Path $scriptDir "..")
$cfg = & (Join-Path $scriptDir "lab_env.ps1") -ConfigPath $ConfigPath

& (Join-Path $scriptDir "init_runtime.ps1") -ConfigPath $ConfigPath | Out-Null

$nodeId = $cfg["EXECUTOR_NODE_ID"]
$runtimeDir = Join-Path $repoRoot "demo\runtime\$nodeId"
$metaPath = Join-Path $runtimeDir "tokens\$TokenName.meta.json"
$revokedPath = Join-Path $runtimeDir "revoked.json"

if (-not (Test-Path $metaPath)) {
  throw "Missing token meta: $metaPath"
}

$meta = Get-Content $metaPath | ConvertFrom-Json
$tokenId = $meta.token_id

$python = $cfg["PYTHON"]
Push-Location $repoRoot
try {
  & $python -m src.controller.operator_stub revoke \
    --revocation-list $revokedPath \
    --token-id $tokenId
} finally {
  Pop-Location
}

Write-Output "[revoke] token_id=$tokenId"
