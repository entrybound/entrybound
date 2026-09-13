# Run the Windows-applicable ebr tests on the Windows host (job-object/psutil measurement path).
# Creates (idempotently) a throwaway venv outside the repo, installs the hash-verified psutil wheel
# recorded in windows-test-downloads.json, and runs pytest on the stdlib/psutil-only test modules.
# Usage: pwsh -File research/tools/run_tests_windows.ps1 [-VenvDir <dir>]
param([string]$VenvDir = (Join-Path $env:TEMP 'eb-research-ebr-winvenv'))
$ErrorActionPreference = 'Stop'
$tools = $PSScriptRoot
$python = 'C:/Python313/python.exe'
$manifest = Get-Content (Join-Path $tools 'windows-test-downloads.json') -Raw | ConvertFrom-Json
$wheelDir = Join-Path $VenvDir 'wheels'

if (-not (Test-Path (Join-Path $VenvDir 'Scripts/python.exe'))) {
    & $python -m venv --system-site-packages $VenvDir
    if ($LASTEXITCODE -ne 0) { throw 'venv creation failed' }
}
New-Item -ItemType Directory -Force $wheelDir | Out-Null
foreach ($f in $manifest.files) {
    $dest = Join-Path $wheelDir $f.filename
    if (-not (Test-Path $dest)) { Invoke-WebRequest -Uri $f.url -OutFile $dest }
    $h = (Get-FileHash -Algorithm SHA256 $dest).Hash.ToLower()
    $size = (Get-Item $dest).Length
    if ($h -ne $f.sha256 -or $size -ne $f.size_bytes) { throw "HASH/SIZE MISMATCH for $($f.filename): $h $size" }
}
$vpy = Join-Path $VenvDir 'Scripts/python.exe'
& $vpy -m pip install --disable-pip-version-check --no-index --no-deps --find-links $wheelDir 'psutil==7.2.2' | Out-Null
if ($LASTEXITCODE -ne 0) { throw 'psutil install failed' }

$env:PYTHONPATH = $tools
$env:PYTHONDONTWRITEBYTECODE = '1'
Push-Location $tools
try {
    & $vpy -m pytest -p no:cacheprovider -q tests/test_windows_exec.py tests/test_timev.py
    exit $LASTEXITCODE
} finally {
    Pop-Location
}
