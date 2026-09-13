#requires -Version 7
<#
.SYNOPSIS
  Capture all Entrybound research environment fingerprints (Windows host, WSL Ubuntu,
  pinned ubuntu:24.04 Docker container) and verify the store.

.DESCRIPTION
  Order matters: the WSL and Docker fingerprints embed the Windows host platform_id as
  stable.platform.parent, so windows-host is captured first. Every step is idempotent:
  an unchanged fingerprint is not rewritten, and any hash mismatch aborts the run.

  Run from a normal PowerShell 7 session (non-admin). Tool resolution follows the PATH of
  the invoking shell, so capture from the same kind of shell the research runner uses.
#>
$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path (Join-Path $PSScriptRoot '../../..')).Path
$py = 'C:/Python313/python.exe'
$tool = Join-Path $PSScriptRoot 'capture_env.py'
$out = Join-Path $repo 'research/environment'
$wslEnvDir = '/mnt/d/Projects/entrybound/entrybound/research/tools/env'

& $py $tool capture --name windows-host --repo $repo --work-dir $repo --out-dir $out
if ($LASTEXITCODE -ne 0) { throw "windows-host capture failed ($LASTEXITCODE)" }

wsl.exe -d Ubuntu -- bash "$wslEnvDir/capture_wsl.sh"
if ($LASTEXITCODE -ne 0) { throw "wsl-ubuntu capture failed ($LASTEXITCODE)" }

wsl.exe -d Ubuntu -- bash "$wslEnvDir/capture_docker.sh"
if ($LASTEXITCODE -ne 0) { throw "docker-ubuntu-24.04 capture failed ($LASTEXITCODE)" }

& $py $tool verify --out-dir $out
if ($LASTEXITCODE -ne 0) { throw "fingerprint verification failed ($LASTEXITCODE)" }
