param([Parameter(Mandatory = $true)][string]$Code)
# Lend this computer to your Restless plane (Settings > Computers > Add a computer).
#
# It starts one container, "restless-machine", in Docker Desktop that opens a single outbound
# connection to your plane. Nothing listens on this computer and no port is opened. Restless can
# then run its own sealed containers here, and nothing else: it cannot read your files or touch
# your other containers. Remove the computer in Settings > Computers to cut it off at once.
$ErrorActionPreference = 'Stop'
$Plane = '__RESTLESS_PLANE__'
$Helper = 'restless-machine'

if (-not (Get-Command docker -ErrorAction SilentlyContinue)) {
  Write-Host 'Restless runs on Docker, which is not installed here.'
  Write-Host 'Install Docker Desktop (https://www.docker.com/products/docker-desktop/), start it, then run this again.'
  return
}
docker info *> $null
if ($LASTEXITCODE -ne 0) {
  Write-Host 'Docker Desktop is installed but not running. Start it, wait for it to say Running, then run this again.'
  return
}

Write-Host 'Fetching the Restless computer image from your plane. This is a few GB, once.'
$Archive = Join-Path $env:TEMP ('restless-machine-' + [guid]::NewGuid().ToString() + '.tar')
try {
  $ProgressPreference = 'SilentlyContinue'
  Invoke-WebRequest -Uri "$Plane/machines/helper-image" -Headers @{ 'X-Restless-Pairing' = $Code } -OutFile $Archive -UseBasicParsing
  $Loaded = docker load -q -i $Archive
} catch {
  Write-Host 'Could not fetch the image. A code lasts 10 minutes and works once; make a new one and try again.'
  return
} finally {
  Remove-Item -Force -ErrorAction SilentlyContinue $Archive
}
$Image = ($Loaded | Select-String -Pattern '^Loaded image ID: (.+)$' | Select-Object -Last 1).Matches.Groups[1].Value
if (-not $Image) {
  Write-Host 'The image did not load. Try again with a new code.'
  return
}

$Existing = docker inspect -f '{{index .Config.Labels "io.restless.helper"}}' $Helper 2> $null
if ($LASTEXITCODE -eq 0) {
  if ($Existing -ne '1') {
    Write-Host "A container named $Helper already exists and is not Restless's. Rename or remove it, then run this again."
    return
  }
  docker rm -f $Helper *> $null
}
docker volume create restless-machine-state *> $null
docker run -d --name $Helper --restart unless-stopped `
  --label io.restless.helper=1 `
  --user 0 `
  -v /var/run/docker.sock:/var/run/docker.sock `
  -v restless-machine-state:/var/lib/restless-machine `
  -e "RESTLESS_MACHINE_NAME=$env:COMPUTERNAME" `
  --entrypoint /usr/local/bin/restless-machine `
  $Image connect $Plane $Code *> $null

Write-Host 'Waiting for your plane to confirm...'
for ($i = 0; $i -lt 45; $i++) {
  if ((docker logs $Helper 2>&1 | Out-String) -match 'linked to the plane') {
    Write-Host 'Connected. This computer now appears in Settings > Computers. You can close this window.'
    return
  }
  if ((docker inspect -f '{{.State.Running}}' $Helper) -ne 'true') {
    Write-Host 'The helper stopped:'
    docker logs $Helper 2>&1 | Select-Object -Last 5
    return
  }
  Start-Sleep -Seconds 2
}
Write-Host 'Still connecting. Check Settings > Computers; the helper keeps trying on its own.'
