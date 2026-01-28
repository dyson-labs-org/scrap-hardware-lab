param(
  [string]$TokenName = "authorized",
  [string]$Capability,
  [int]$ExpiresIn = 3600,
  [string]$ConfigPath
)

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$repoRoot = Resolve-Path (Join-Path $scriptDir "..")
$cfg = & (Join-Path $scriptDir "lab_env.ps1") -ConfigPath $ConfigPath

if (-not $Capability) { $Capability = $cfg["TASK_CAPABILITY"] }
$python = $cfg["PYTHON"]

& (Join-Path $scriptDir "init_runtime.ps1") -ConfigPath $ConfigPath | Out-Null

$nodeId = $cfg["EXECUTOR_NODE_ID"]
$runtimeDir = Join-Path $repoRoot "demo\runtime\$nodeId"
$tokensDir = Join-Path $runtimeDir "tokens"
New-Item -ItemType Directory -Force $tokensDir | Out-Null

$keysPath = Join-Path $runtimeDir "keys.json"
$tokenBin = Join-Path $tokensDir "$TokenName.bin"
$tokenMeta = Join-Path $tokensDir "$TokenName.meta.json"

$keys = Get-Content $keysPath | ConvertFrom-Json
$subject = $keys.commander_pubkey

Push-Location $repoRoot
try {
  & $python -m src.controller.operator_stub issue-token \
    --keys $keysPath \
    --out $tokenBin \
    --meta-out $tokenMeta \
    --subject $subject \
    --audience $nodeId \
    --capability $Capability \
    --expires-in $ExpiresIn \
    --allow-mock-signature
} finally {
  Pop-Location
}

Write-Output "[token] $tokenBin"
