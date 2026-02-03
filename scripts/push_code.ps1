param(
  [string]$TargetHost,
  [string]$TargetUser,
  [string]$ConfigPath
)

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$repoRoot = Resolve-Path (Join-Path $scriptDir "..")
$cfg = & (Join-Path $scriptDir "lab_env.ps1") -ConfigPath $ConfigPath

if (-not $TargetHost) { $TargetHost = $cfg["EXECUTOR_HOST"] }
if (-not $TargetUser) { $TargetUser = $cfg["EXECUTOR_USER"] }
$repoDir = $cfg["REPO_DIR"]

if (-not $TargetHost -or -not $TargetUser) {
  throw "Missing target host/user"
}

Push-Location $repoRoot
try {
  & ssh "$TargetUser@$TargetHost" "mkdir -p $repoDir"
  & scp -r src demo docs scripts "$TargetUser@$TargetHost:$repoDir/"
} finally {
  Pop-Location
}

Write-Output "[push] $TargetUser@$TargetHost:$repoDir"
