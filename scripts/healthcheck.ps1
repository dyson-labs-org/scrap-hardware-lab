param(
  [string]$PiIP       = "192.168.50.11",
  [string]$PiUser     = "pi",

  [string]$JetsonIP   = "192.168.50.10",
  [string]$JetsonUser = "jetson",

  [string]$BBB01IP    = "192.168.50.31",
  [string]$BBB01User  = "debian",

  [string]$BBB02IP    = "192.168.50.32",
  [string]$BBB02User  = "debian",

  [bool]$BatchMode = $false
)

function Test-Tcp22([string]$IP) {
  try {
    (Test-NetConnection -ComputerName $IP -Port 22 -WarningAction SilentlyContinue).TcpTestSucceeded
  } catch { $false }
}

function Get-CommonSshArgs([bool]$Batch) {
  $args = @(
    "-o","ConnectTimeout=5",
    "-o","ServerAliveInterval=3",
    "-o","ServerAliveCountMax=2",
    "-o","StrictHostKeyChecking=accept-new"
  )
  if ($Batch) { $args += @("-o","BatchMode=yes") }
  return $args
}

function Invoke-SshInteractive([string[]]$Argv) {
  & ssh @Argv
  return $LASTEXITCODE
}

function Invoke-SshBatch([string[]]$Argv) {
  $out = & ssh @Argv 2>&1
  return @{ Code=$LASTEXITCODE; Out=($out | Out-String).Trim() }
}

Write-Host ""
Write-Host "SCRAP Hardware Lab - Switch Healthcheck" -ForegroundColor Cyan
Write-Host ("Time: " + (Get-Date).ToString("yyyy-MM-dd HH:mm:ss"))
Write-Host "Subnet: 192.168.50.0/24"
Write-Host ""

$common = Get-CommonSshArgs -Batch $BatchMode

# IMPORTANT: remote command must be ONE argument
$remoteCmd = 'echo OK; hostname; uptime; ip -br a'

$nodes = @(
  @{ Name="pi-a";     IP=$PiIP;     User=$PiUser     },
  @{ Name="jetson-a"; IP=$JetsonIP; User=$JetsonUser },
  @{ Name="bbb-01";   IP=$BBB01IP;  User=$BBB01User  },
  @{ Name="bbb-02";   IP=$BBB02IP;  User=$BBB02User  }
)

$results = @()

foreach ($n in $nodes) {
  $tcp = Test-Tcp22 $n.IP
  $sshOk = $false
  $notes = "ok"

  if (-not $tcp) {
    $notes = "TCP/22 unreachable"
  } else {
    $dest = "$($n.User)@$($n.IP)"

    # Build ONE flat argv list
    $argv = @()
    $argv += $common
    $argv += $dest
    $argv += $remoteCmd

    if ($BatchMode) {
      $r = Invoke-SshBatch $argv
      $sshOk = ($r.Code -eq 0)
      if (-not $sshOk) { $notes = "ssh exit $($r.Code): $($r.Out.Split([Environment]::NewLine)[0])" }
    } else {
      Write-Host "`n--- $($n.Name) ($dest) ---" -ForegroundColor Cyan
      Write-Host "(If prompted, enter password)" -ForegroundColor DarkGray
      $code = Invoke-SshInteractive $argv
      $sshOk = ($code -eq 0)
      if (-not $sshOk) { $notes = "ssh exit $code" }
    }
  }

  $results += [pscustomobject]@{
    Target = $n.Name
    IP     = $n.IP
    Tcp22  = $tcp
    SshOk  = $sshOk
    Notes  = $notes
  }
}

Write-Host ""
$results | Format-Table -AutoSize

$failed = $results | Where-Object { $_.SshOk -ne $true }
if ($failed.Count -gt 0) {
  Write-Host "`nHEALTHCHECK: FAIL" -ForegroundColor Red
  exit 1
} else {
  Write-Host "`nHEALTHCHECK: PASS" -ForegroundColor Green
  exit 0
}
