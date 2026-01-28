param(
  [string]$ConfigPath
)

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$repoRoot = Resolve-Path (Join-Path $scriptDir "..")

if (-not $ConfigPath) {
  $ConfigPath = Join-Path $repoRoot "demo\config\demo.env"
}
if (-not (Test-Path $ConfigPath)) {
  $ConfigPath = Join-Path $repoRoot "demo\config\demo.env.template"
}

$values = @{}
Get-Content $ConfigPath | ForEach-Object {
  $line = $_.Trim()
  if ($line -eq "" -or $line.StartsWith("#")) { return }
  $parts = $line -split "=", 2
  if ($parts.Length -eq 2) {
    $values[$parts[0]] = $parts[1]
  }
}

# Defaults
if (-not $values.ContainsKey("EXECUTOR_HOST")) { $values["EXECUTOR_HOST"] = "127.0.0.1" }
if (-not $values.ContainsKey("EXECUTOR_PORT")) { $values["EXECUTOR_PORT"] = "7227" }
if (-not $values.ContainsKey("EXECUTOR_NODE_ID")) { $values["EXECUTOR_NODE_ID"] = "EXECUTOR" }
if (-not $values.ContainsKey("EXECUTOR_BIND")) { $values["EXECUTOR_BIND"] = $values["EXECUTOR_HOST"] }
if (-not $values.ContainsKey("PYTHON")) { $values["PYTHON"] = "python3" }
if (-not $values.ContainsKey("REPO_DIR")) { $values["REPO_DIR"] = "~/scrap-hardware-lab" }
if (-not $values.ContainsKey("TASK_ID")) { $values["TASK_ID"] = "IMG-001" }
if (-not $values.ContainsKey("TASK_CAPABILITY")) { $values["TASK_CAPABILITY"] = "cmd:imaging:msi" }
if (-not $values.ContainsKey("TASK_TYPE")) { $values["TASK_TYPE"] = "imaging" }
if (-not $values.ContainsKey("MAX_AMOUNT_SATS")) { $values["MAX_AMOUNT_SATS"] = "22000" }

return $values
