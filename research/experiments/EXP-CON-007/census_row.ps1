# EXP-CON-007 census row, Windows host arm (pwsh 7). Mirrors census_row.sh: packs one staged NTFS copy with
# one (profile, layout, encryption) cell and prints ONE JSON line for the ebr stdout_json metric source.
# Pre-registered design artifact. Plain cells use the production CLI; encrypted cells use the harness
# ebr-pack.exe --encrypt true (the production CLI reads passwords only from the console).
param(
    [Parameter(Mandatory = $true)][string]$ItemPath,
    [Parameter(Mandatory = $true)][string]$Out,
    [string]$Profile = 'balanced',
    [string]$Layout = 'indexed',
    [string]$Encrypt = 'none',
    [Parameter(Mandatory = $true)][string]$Ebound,
    [Parameter(Mandatory = $true)][string]$InspectBytes,
    [string]$EbrPack = 'D:\eb-research\target\harness\release\ebr-pack.exe',
    [string]$EnvName = 'windows-host'
)
$ErrorActionPreference = 'Stop'
$dir = Split-Path -Parent $Out
$packErr = Join-Path $dir 'pack.stderr'
function Emit($obj) { $obj | ConvertTo-Json -Compress -Depth 4 | Write-Output }

if ($Encrypt -eq 'none') {
    & $Ebound pack $ItemPath $Out --profile $Profile --layout $Layout 1> (Join-Path $dir 'pack.stdout') 2> $packErr
} else {
    & $EbrPack --item-path $ItemPath --experiment-id EXP-CON-007 --env-name $EnvName --profile $Profile `
        --layout indexed --encrypt true --archive-out $Out 1> (Join-Path $dir 'pack.stdout') 2> $packErr
}
if ($LASTEXITCODE -ne 0) {
    $code = (Select-String -Path $packErr -Pattern 'EB_[A-Z0-9_]+' -AllMatches | Select-Object -First 1).Matches.Value
    if (-not $code) { $code = 'PACK_FAILED' }
    Emit ([ordered]@{ pack_outcome = $code })
    exit 0
}

$inspect = $null
try { $inspect = (& $Ebound inspect $Out --json 2> (Join-Path $dir 'inspect.stderr')) | Out-String | ConvertFrom-Json } catch { $inspect = $null }
$ibArgs = @('--archive-path', $Out, '--experiment-id', 'EXP-CON-007', '--env-name', $EnvName, '--layout', $Layout)
if ($Encrypt -ne 'none') {
    $ibArgs = @('--archive-path', $Out, '--experiment-id', 'EXP-CON-007', '--env-name', $EnvName, '--layout', 'indexed',
                '--encrypted', 'true', '--unlock-password-file', "$Out.testpassword")
}
$ib = $null
try { $ib = ((& $InspectBytes @ibArgs 2> (Join-Path $dir 'ib.stderr')) | Select-Object -Last 1) | ConvertFrom-Json } catch { $ib = $null }

$fs = [System.IO.File]::OpenRead($Out)
try { $buf = New-Object byte[] 24; [void]$fs.Seek(16, 'Begin'); [void]$fs.Read($buf, 0, 24) } finally { $fs.Dispose() }
$preHex = -join ($buf | ForEach-Object { $_.ToString('x2') })

function Get-Prop($o, [string[]]$path) {
    foreach ($p in $path) { if ($null -eq $o -or -not ($o.PSObject.Properties.Name -contains $p)) { return $null }; $o = $o.$p }
    return $o
}
$audits = Get-Prop $inspect @('reconstruction_audits')
Emit ([ordered]@{
    pack_outcome               = 'OK'
    incompat                   = Get-Prop $inspect @('archive', 'features', 'incompat')
    read_only_compat           = Get-Prop $inspect @('archive', 'features', 'read_only_compat')
    compat                     = Get-Prop $inspect @('archive', 'features', 'compat')
    preamble_feature_words_hex = $preHex
    reconstruction_audit_count = if ($null -eq $audits) { $null } else { @($audits).Count }
    entry_count                = Get-Prop $inspect @('resources', 'entry_count')
    chunk_count                = Get-Prop $inspect @('resources', 'chunk_count')
    planner_id                 = Get-Prop $ib @('payload', 'counts', 'planner_id')
    artifact_bytes             = Get-Prop $ib @('payload', 'artifact_bytes')
    reconstruction_bytes       = Get-Prop $ib @('payload', 'byte_accounting', 'reconstruction_bytes')
    manifest_bytes             = Get-Prop $ib @('payload', 'byte_accounting', 'manifest_bytes')
    index_bytes                = Get-Prop $ib @('payload', 'byte_accounting', 'index_bytes')
})
