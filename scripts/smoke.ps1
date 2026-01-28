param(
  [string]$ConfigPath
)

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path

& (Join-Path $scriptDir "push_code.ps1") -ConfigPath $ConfigPath
& (Join-Path $scriptDir "issue_token.ps1") -ConfigPath $ConfigPath
& (Join-Path $scriptDir "start_executor.ps1") -ConfigPath $ConfigPath
Start-Sleep -Seconds 1
& (Join-Path $scriptDir "run_commander.ps1") -ConfigPath $ConfigPath
& (Join-Path $scriptDir "collect_logs.ps1") -ConfigPath $ConfigPath
