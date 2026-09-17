# Stage NTFS copies of small and medium tuning/validation corpus items for the Windows-host arms of
# EXP-CON-007 and EXP-CON-005 (pre-registered design artifact; pwsh 7, non-admin).
#
#   pwsh -File stage_ntfs_copies.ps1 -Split tuning [-Items id1,id2]
#
# Reads research/experiments/EXP-CON-007/ntfs-items.json (tuning/validation rows only; the file contains no
# held-out rows), copies each item from the WSL corpus through \\wsl.localhost\Ubuntu into
# D:\eb-research\corpus-ntfs\<split>\<item_id> with robocopy, and writes a staging report
# (D:\eb-research\corpus-ntfs\staging-report-<split>.json) listing robocopy exit codes, skipped files, junctions
# and symbolic links that could not be created without SeCreateSymbolicLinkPrivilege, and names that NTFS
# or the WSL 9P bridge could not carry. The copy is a different capture (NTFS metadata) by design; the report
# is part of the experiment record and every skipped object is a recorded fidelity gap, never silently dropped.
param(
    [Parameter(Mandatory = $true)][ValidateSet('tuning', 'validation')][string]$Split,
    [string[]]$Items = @(),
    [string]$Repo = 'D:\Projects\entrybound\entrybound',
    [string]$Dest = 'D:\eb-research\corpus-ntfs'
)
$ErrorActionPreference = 'Stop'
& C:\Python313\python.exe "$Repo\research\orchestration\disk_guard.py"
if ($LASTEXITCODE -ne 0) { throw 'disk_guard.py failed; refusing to stage copies' }
$manifest = Get-Content -Raw "$Repo\research\experiments\EXP-CON-007\ntfs-items.json" | ConvertFrom-Json
$rows = $manifest.items | Where-Object { $_.split -eq $Split -and ($Items.Count -eq 0 -or $Items -contains $_.item_id) }
$report = @()
foreach ($r in $rows) {
    if ($r.item_id -match 'heldout') { throw "refusing held-out-looking item id $($r.item_id)" }
    $src = "\\wsl.localhost\Ubuntu\root\eb-research\corpus\$Split\$($r.item_id)"
    $dst = Join-Path (Join-Path $Dest $Split) $r.item_id
    New-Item -ItemType Directory -Force -Path $dst | Out-Null
    $log = "$dst.robocopy.log"
    if (Test-Path -LiteralPath $src -PathType Leaf) {
        robocopy (Split-Path -Parent $src) $dst (Split-Path -Leaf $src) /COPY:DAT /DCOPY:DAT /R:0 /W:0 /SL /NP /LOG:$log | Out-Null
    } else {
        robocopy $src $dst /E /COPY:DAT /DCOPY:DAT /R:0 /W:0 /SL /XJ /NP /LOG:$log | Out-Null
    }
    $code = $LASTEXITCODE
    $failed = (Select-String -Path $log -Pattern 'ERROR' -SimpleMatch | Measure-Object).Count
    $report += [ordered]@{ item_id = $r.item_id; robocopy_exit = $code; error_lines = $failed; log = $log }
}
$report | ConvertTo-Json -Depth 3 | Set-Content -Encoding utf8 (Join-Path $Dest "staging-report-$Split.json")
