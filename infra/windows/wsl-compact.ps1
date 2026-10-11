# Reclaims the space a WSL distro's ext4.vhdx holds but no longer uses.
#
# The virtual disk grows to its peak and never shrinks: on 12 October 2026 it
# held 803 GB for a distro using 272 GB. Sparse mode would fix that, but WSL
# disables it upstream for data-corruption risk, so this trims the distro,
# stops WslService (anything touching \\wsl.localhost restarts a plain
# `wsl --shutdown` within seconds), compacts with diskpart and restores the
# service. See docs/BUILD_STORAGE.md.
#
# Run from an elevated shell:
#   .\wsl-compact.ps1             compact now if the slack is worth it
#   .\wsl-compact.ps1 -Install    copy to ProgramData and register a weekly task
#   .\wsl-compact.ps1 -Uninstall  remove the task and the copy
param(
  [string]$Distro = 'Ubuntu',
  [int]$MinSlackGB = 50,
  [switch]$Install,
  [switch]$Uninstall
)
$ErrorActionPreference = 'Stop'
Set-Location C:\
$TaskName = 'Restless WSL compact'
$Home_ = Join-Path $env:ProgramData 'Restless'
$LogFile = Join-Path $Home_ 'wsl-compact.log'

$admin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $admin) { throw 'Run from an elevated shell: diskpart and WslService need administrator rights.' }

function Log($m) {
  $line = "$(Get-Date -Format 'yyyy-MM-dd HH:mm:ss') $m"
  Write-Output $line
  New-Item -ItemType Directory -Force $Home_ | Out-Null
  Add-Content -LiteralPath $LogFile $line
}

if ($Uninstall) {
  Unregister-ScheduledTask -TaskName $TaskName -Confirm:$false -ErrorAction SilentlyContinue
  Remove-Item -LiteralPath (Join-Path $Home_ 'wsl-compact.ps1') -ErrorAction SilentlyContinue
  Write-Output "removed $TaskName"
  return
}

if ($Install) {
  # The task runs elevated, so the copy it runs must not be writable by the
  # unelevated user: administrators and SYSTEM own it, users may only read.
  New-Item -ItemType Directory -Force $Home_ | Out-Null
  $copy = Join-Path $Home_ 'wsl-compact.ps1'
  Copy-Item -LiteralPath $PSCommandPath $copy -Force
  $acl = New-Object Security.AccessControl.DirectorySecurity
  $acl.SetAccessRuleProtection($true, $false)
  foreach ($who in 'BUILTIN\Administrators', 'NT AUTHORITY\SYSTEM') {
    $acl.AddAccessRule((New-Object Security.AccessControl.FileSystemAccessRule($who, 'FullControl', 'ContainerInherit,ObjectInherit', 'None', 'Allow')))
  }
  $acl.AddAccessRule((New-Object Security.AccessControl.FileSystemAccessRule('BUILTIN\Users', 'ReadAndExecute', 'ContainerInherit,ObjectInherit', 'None', 'Allow')))
  Set-Acl -LiteralPath $Home_ $acl
  $action = New-ScheduledTaskAction -Execute 'powershell.exe' -Argument "-NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File `"$copy`" -Distro $Distro -MinSlackGB $MinSlackGB"
  $trigger = New-ScheduledTaskTrigger -Weekly -DaysOfWeek Sunday -At 4am
  $user = "$env:USERDOMAIN\$env:USERNAME"
  # WSL distros are registered per user, so the task runs as this user, elevated,
  # while they are signed in. No password is stored.
  $principal = New-ScheduledTaskPrincipal -UserId $user -LogonType Interactive -RunLevel Highest
  $settings = New-ScheduledTaskSettingsSet -StartWhenAvailable -ExecutionTimeLimit (New-TimeSpan -Hours 2) -AllowStartIfOnBatteries:$false
  Register-ScheduledTask -TaskName $TaskName -Action $action -Trigger $trigger -Principal $principal -Settings $settings -Force | Out-Null
  Write-Output "registered '$TaskName': Sundays 04:00 as $user; runs $copy; logs to $LogFile"
  return
}

function InDistro([string]$cmd) {
  ((wsl.exe -d $Distro -u root -- sh -c $cmd 2>$null) | Out-String).Trim()
}

$key = Get-ChildItem 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Lxss' |
  Where-Object { (Get-ItemProperty $_.PSPath).DistributionName -eq $Distro } | Select-Object -First 1
if (-not $key) { throw "No WSL distro named $Distro" }
$vhd = Join-Path ((Get-ItemProperty $key.PSPath).BasePath -replace '^\\\\\?\\', '') 'ext4.vhdx'
$fileGB = [math]::Round((Get-Item -LiteralPath $vhd).Length / 1GB)
$usedGB = [math]::Round([double](InDistro 'df -B1 --output=used / | tail -n 1') / 1GB)
# An unreadable distro reports 0 used, which would read as all slack.
if ($usedGB -le 0) { Log "skip: could not measure usage inside $Distro"; return }
$slackGB = $fileGB - $usedGB
if ($slackGB -lt $MinSlackGB) { Log "skip: $vhd is $fileGB GB, distro uses $usedGB GB, slack $slackGB GB < $MinSlackGB"; return }

# Stopping WSL kills everything in it. Skip a week rather than cut off a CI job
# or a Rust build; the slack is still there next time.
$busy = InDistro 'pgrep -f Runner.Worker >/dev/null && echo ci-job; pgrep -x cargo >/dev/null && echo cargo; true'
if ($busy) { Log "skip: busy ($($busy -replace '\s+', ', ')); slack $slackGB GB"; return }

Log "start: $vhd is $fileGB GB, distro uses $usedGB GB"
InDistro 'fstrim / >/dev/null' | Out-Null
$start = (Get-Service WslService).StartType
$script = Join-Path $env:TEMP 'restless-wsl-compact.txt'
"select vdisk file=`"$vhd`"`r`nattach vdisk readonly`r`ncompact vdisk`r`ndetach vdisk" | Out-File $script -Encoding ascii
try {
  wsl.exe --shutdown
  Set-Service WslService -StartupType Disabled
  Stop-Service WslService -Force
  $out = diskpart /s $script 2>&1 | Out-String
  if ($out -notmatch 'successfully compacted') { Log "diskpart did not compact: $(($out -split "`n" | Where-Object { $_ -match 'error' }) -join ' ')" }
} finally {
  Set-Service WslService -StartupType $start
  Start-Service WslService
  Remove-Item -LiteralPath $script -ErrorAction SilentlyContinue
}
# Boot the distro so its systemd user services (the plane, the CI runners) return.
wsl.exe -d $Distro -- true
Log ("done: {0} GB -> {1} GB" -f $fileGB, [math]::Round((Get-Item -LiteralPath $vhd).Length / 1GB))
