# Extracts the external, unpublished research inputs (which are NOT committed to this
# repository) into a stable local directory so research agents and tools can read them.
#
# Inputs are verified against the SHA-256 values recorded in research/source-inventory.md
# and research/PROGRESS.md before extraction. A mismatch aborts.
#
# Usage (PowerShell 7):
#   pwsh research/tools/sources/extract_external_sources.ps1 [-Root D:\Projects\entrybound] [-Dest D:\eb-research\sources]

param(
    [string]$Root = 'D:\Projects\entrybound',
    [string]$Dest = 'D:\eb-research\sources'
)
$ErrorActionPreference = 'Stop'

$expected = [ordered]@{
    'design\2026-08-29-entrybound-product-architecture.md'                           = '1f881c7b13f193b41353b90066d33ca6abcb7d4c3dcd4f58e22834801f0fe29c'
    'design\research-appendix\entrybound-architecture-research-appendix.tgz'         = '47a3240a685300206353c84c348ac504266b2e3e335bc05c441369a9e7d4db99'
    'research\2026-08-29-archive-category-research.md'                               = 'bc524da26e85e20bce515b81a9e84ba1a412599ca66b08bdda7ab3005c4b8cfe'
    'research\2026-08-29-entrybound-opportunity-validation.md'                       = '8c3575a463e492d99298400c2bf77f0f8b6fa260368671b23398e00c9c1bd301'
    'research\2026-08-29-entrybound-research-iii-final-gate.md'                      = 'bb209e0c74bf3aac4067efdf54aeb70efd0cee232945903e6b11d1bdfee495c3'
    'research\entrybound-research-iii-instrumentation.tgz'                           = 'd221f434dd92641156003fe4b1fa9d50ba7fa10d4c848374cb37cd14017810c8'
}

foreach ($rel in $expected.Keys) {
    $path = Join-Path $Root $rel
    if (-not (Test-Path $path)) { throw "missing external source: $path" }
    $actual = (Get-FileHash $path -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actual -ne $expected[$rel]) { throw "hash mismatch for ${path}: expected $($expected[$rel]) got $actual" }
    Write-Output "ok  $actual  $rel"
}

New-Item -ItemType Directory -Force (Join-Path $Dest 'appendix'), (Join-Path $Dest 'r3-instrumentation') | Out-Null
tar -xzf (Join-Path $Root 'design\research-appendix\entrybound-architecture-research-appendix.tgz') -C (Join-Path $Dest 'appendix')
tar -xzf (Join-Path $Root 'research\entrybound-research-iii-instrumentation.tgz') -C (Join-Path $Dest 'r3-instrumentation')
Write-Output "extracted to $Dest"
