#Requires -Version 7
<#
.SYNOPSIS
Byte-identity proof for the default-off `research-internals` cargo feature (Windows host).

.DESCRIPTION
Windows counterpart of internals-identity-check.sh (WSL is the primary host).

1. Additive-only audit: every production change is a removable
   #[cfg(feature = "research-internals")] block (internals_identity_tools.py).
2. Builds the CLI with and without --features entrybound/research-internals,
   packs one fixed generated tree under all four profiles in INDEXED and STREAM
   layouts, and requires identical SHA-256 for every output.
3. cargo test -p entrybound -p entrybound-cli without the feature, and the same
   packages with --features entrybound/research-internals (a superset of
   cargo test -p entrybound --features research-internals).
4. cargo clippy --workspace --all-targets (lints reported, and -D warnings)
   both ways, and cargo fmt --all --check.

Gates report PASS, FAIL, or PRE-EXISTING (fails identically without the feature,
on source the additive audit proves equal to the base commit).
5. Rewrites the Windows section of internals-identity-check.md.

No timings are recorded (not decision-grade). Build targets and scratch output
must live on D: (never on C:).

.EXAMPLE
pwsh -NoProfile -File D:\Projects\entrybound\entrybound\research\harness\internals-identity-check.ps1
#>
[CmdletBinding()]
param(
    [string]$TargetDir = 'D:\eb-research\target\b2-internals',
    [string]$WorkDir = 'D:\eb-research\scratch\b2-internals-identity',
    [string]$BaseRev = '',
    [switch]$SkipDiskGuard
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $false

$Repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$Toolchain = '1.98.1'
$CargoBin = Join-Path $env:USERPROFILE '.cargo\bin'
$CargoExe = Join-Path $CargoBin 'cargo.exe'
$Cargo = @($CargoExe, "+$Toolchain")
$Py = @('C:\Python313\python.exe', '-B')
$Tools = Join-Path $Repo 'research\harness\internals_identity_tools.py'
$Report = Join-Path $Repo 'research\harness\internals-identity-check.md'
$Profiles = @('fast', 'balanced', 'dense', 'extreme')
$Layouts = @('indexed', 'stream')

foreach ($path in @($TargetDir, $WorkDir)) {
    if ($path -notmatch '^[Dd]:[\\/]') {
        throw "refusing ${path}: research build output must stay on D:"
    }
}

$env:CARGO_TARGET_DIR = $TargetDir
$env:RUSTUP_TOOLCHAIN = $Toolchain
$env:PATH = "$CargoBin;$env:PATH"
$env:GIT_OPTIONAL_LOCKS = '0'

Set-Location -LiteralPath $Repo
if (Test-Path -LiteralPath $WorkDir) {
    Remove-Item -LiteralPath $WorkDir -Recurse -Force -Confirm:$false
}
$Logs = Join-Path $WorkDir 'logs'
$Bin = Join-Path $WorkDir 'bin'
foreach ($dir in @($Logs, $Bin, (Join-Path $WorkDir 'out\default'), (Join-Path $WorkDir 'out\research'))) {
    New-Item -ItemType Directory -Force -Path $dir | Out-Null
}
$Section = [System.Collections.Generic.List[string]]::new()
$Gates = [System.Collections.Generic.List[string]]::new()
$script:Failed = 0
$script:PreExisting = 0

function Add-Md([string[]]$Lines) { foreach ($line in $Lines) { $Section.Add($line) } }
function Add-Gate([string]$Name, [string]$Status, [string]$Detail) {
    $Gates.Add("| $Name | $Status | $Detail |")
    if ($Status -eq 'PRE-EXISTING') { $script:PreExisting = 1 } elseif ($Status -ne 'PASS') { $script:Failed = 1 }
}
# Command lines are recorded with the user profile path redacted (the report is published).
function Format-Command([string[]]$Command) {
    $text = ($Command | ForEach-Object { if ($_ -match '[\s"]') { '"' + ($_ -replace '"', '\"') + '"' } else { $_ } }) -join ' '
    $text.Replace($env:USERPROFILE, '%USERPROFILE%')
}
function Invoke-Logged([string]$Log, [string[]]$Command) {
    $ErrorActionPreference = 'Continue'
    "`$ $(Format-Command $Command)" | Out-File -LiteralPath $Log -Encoding utf8NoBOM
    [string[]]$rest = @($Command | Select-Object -Skip 1)
    & $Command[0] @rest 2>&1 | ForEach-Object { "$_" } | Out-File -LiteralPath $Log -Append -Encoding utf8NoBOM
    return $LASTEXITCODE
}
function Add-Fence([string]$Path) {
    Add-Md @('```text')
    Add-Md @(Get-Content -LiteralPath $Path)
    Add-Md @('```')
}
function Get-Sha([string]$Path) {
    if (Test-Path -LiteralPath $Path) { (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash.ToLowerInvariant() } else { 'n/a' }
}
function Hide-Paths([string]$Text) {
    $Text.Replace($Repo, '$REPO').Replace($WorkDir, '$WORK')
}

# ---------------------------------------------------------------------------
# Disk guard
# ---------------------------------------------------------------------------
if ($SkipDiskGuard) {
    $guardStatus = 'skipped (-SkipDiskGuard; caller ran disk_guard.py)'
} else {
    $guardLog = Join-Path $Logs 'disk-guard.log'
    $rc = Invoke-Logged $guardLog @($Py[0], (Join-Path $Repo 'research\orchestration\disk_guard.py'))
    if ($rc -ne 0) {
        Get-Content -LiteralPath $guardLog | Write-Host
        Write-Host 'disk guard failed; not building'
        exit 2
    }
    $guardStatus = 'PASS'
}

$headSha = (git -c safe.directory=* -C $Repo rev-parse HEAD).Trim()
$worktree = @(git -c safe.directory=* -C $Repo status --porcelain -- crates research/harness)
$rustcVersion = (& rustc "+$Toolchain" --version).Trim()
$llvm = ((& rustc "+$Toolchain" -vV) | Where-Object { $_ -like 'LLVM version:*' }) -replace 'LLVM version: ', 'LLVM '

Add-Md @('## Windows', '')
Add-Md @("- Generated: $([DateTime]::UtcNow.ToString('yyyy-MM-ddTHH:mm:ssZ')) on $([System.Environment]::OSVersion.VersionString) (host name redacted)")
Add-Md @("- Repository HEAD: ``$headSha``")
Add-Md @('- Worktree changes under `crates/` and `research/harness/` at run time:', '')
if ($worktree.Count -gt 0) { Add-Md @('```text'); Add-Md ($worktree | ForEach-Object { "    $_" }); Add-Md @('```') } else { Add-Md @('    (clean: the feature is committed)') }
Add-Md @('', "- Toolchain: ``$((& $CargoExe "+$Toolchain" --version).Trim())``, ``$rustcVersion`` ($llvm)")
Add-Md @("- ``CARGO_TARGET_DIR=$TargetDir``, work directory ``$WorkDir`` (both on D:, outside the repository)")
Add-Md @("- Disk guard (``research/orchestration/disk_guard.py``): $guardStatus")
Add-Md @('- Timing: none recorded (not decision-grade).', '')

# ---------------------------------------------------------------------------
# 1. Additive-only source audit
# ---------------------------------------------------------------------------
$auditCmd = @($Py + @($Tools, 'additive-check', $Repo))
if ($BaseRev) { $auditCmd += @('--base', $BaseRev) }
$auditLog = Join-Path $Logs 'additive-check.log'
$audit = if ((Invoke-Logged $auditLog $auditCmd) -eq 0) { 'PASS' } else { 'FAIL' }
Add-Gate 'Additive-only production diff' $audit 'every changed production line is inside a removable research-internals block'

# ---------------------------------------------------------------------------
# 2. Builds and output byte identity
# ---------------------------------------------------------------------------
$buildDefault = @($Cargo + @('build', '--release', '--locked', '-p', 'entrybound-cli'))
$buildResearch = @($Cargo + @('build', '--release', '--locked', '-p', 'entrybound-cli', '--features', 'entrybound/research-internals'))
$buildsOk = 'PASS'
$released = Join-Path $TargetDir 'release\ebound.exe'
if ((Invoke-Logged (Join-Path $Logs 'build-default.log') $buildDefault) -eq 0) {
    Copy-Item -LiteralPath $released -Destination (Join-Path $Bin 'ebound-default.exe')
} else { $buildsOk = 'FAIL' }
if ((Invoke-Logged (Join-Path $Logs 'build-research.log') $buildResearch) -eq 0) {
    Copy-Item -LiteralPath $released -Destination (Join-Path $Bin 'ebound-research.exe')
} else { $buildsOk = 'FAIL' }
Add-Gate 'CLI builds (default and research-internals)' $buildsOk 'release, --locked'

$tree = Join-Path $WorkDir 'tree'
$treeCmd = @($Py + @($Tools, 'tree', $tree))
[string[]]$treeArgs = @($treeCmd | Select-Object -Skip 1)
$treeLine = (& $treeCmd[0] @treeArgs).Trim()
$treeFiles, $treeBytes, $treeDigest = $treeLine -split ' '

$packRows = [System.Collections.Generic.List[string]]::new()
$identical = 0
$packTotal = 0
$packOk = 'PASS'
if ($buildsOk -eq 'PASS') {
    foreach ($profile in $Profiles) {
        foreach ($layout in $Layouts) {
            $hashes = @{}
            foreach ($build in @('default', 'research')) {
                $out = Join-Path $WorkDir "out\$build\$profile-$layout.eb"
                # Dense/Extreme lookback groups need a STREAM dedup window; the CLI
                # default ceiling is 0, so STREAM packs declare an automatic window.
                $window = if ($layout -eq 'stream') { @('--stream-window', 'auto') } else { @() }
                $cmd = @((Join-Path $Bin "ebound-$build.exe"), 'pack', $tree, $out, '--layout', $layout) + $window + @('--profile', $profile)
                if ((Invoke-Logged (Join-Path $Logs "pack-$build-$profile-$layout.log") $cmd) -eq 0) {
                    $hashes[$build] = Get-Sha $out
                } else {
                    $hashes[$build] = 'PACK-FAILED'
                }
            }
            $packTotal++
            $defaultOut = Join-Path $WorkDir "out\default\$profile-$layout.eb"
            $size = if (Test-Path -LiteralPath $defaultOut) { (Get-Item -LiteralPath $defaultOut).Length } else { '?' }
            if ($hashes['default'] -eq $hashes['research'] -and $hashes['default'] -ne 'PACK-FAILED') {
                $same = 'yes'; $identical++
            } else {
                $same = 'NO'; $packOk = 'FAIL'
            }
            $packRows.Add("| $profile | $layout | $size | ``$($hashes['default'])`` | ``$($hashes['research'])`` | $same |")
        }
    }
    foreach ($profile in $Profiles) {
        & (Join-Path $Bin 'ebound-default.exe') inspect (Join-Path $WorkDir "out\default\$profile-indexed.eb") 2>&1 |
            ForEach-Object { "$_" } | Out-File -LiteralPath (Join-Path $Logs "inspect-$profile.log") -Encoding utf8NoBOM
    }
} else {
    $packOk = 'FAIL'
}
Add-Gate 'Pack output SHA-256 identical with and without the feature' $packOk "$identical/$packTotal profile x layout outputs identical"

# ---------------------------------------------------------------------------
# 3. Tests. --no-fail-fast so every test binary runs even when an unrelated
# environment-dependent test fails. The research run covers the same packages
# plus research_internals (a superset of cargo test -p entrybound --features
# research-internals), so failing-test sets are directly comparable.
# ---------------------------------------------------------------------------
$testDefault = @($Cargo + @('test', '--release', '--locked', '--no-fail-fast', '-p', 'entrybound', '-p', 'entrybound-cli'))
$testResearch = @($Cargo + @('test', '--release', '--locked', '--no-fail-fast', '-p', 'entrybound', '-p', 'entrybound-cli', '--features', 'entrybound/research-internals'))
$testDefaultRc = Invoke-Logged (Join-Path $Logs 'test-default.log') $testDefault
$testResearchRc = Invoke-Logged (Join-Path $Logs 'test-research.log') $testResearch
$summaries = @{}
$failures = @{}
foreach ($name in @('test-default', 'test-research')) {
    $summaryPath = Join-Path $Logs "$name.summary"
    & $Py[0] $Py[1] $Tools test-summary (Join-Path $Logs "$name.log") | Out-File -LiteralPath $summaryPath -Encoding utf8NoBOM
    $summaries[$name] = @(Get-Content -LiteralPath $summaryPath)
    $failures[$name] = (@($summaries[$name] | Where-Object { $_ -like 'FAILED *' }) -join "`n")
}
$defaultCompiled = [bool]($summaries['test-default'] -contains 'compile errors: no')
$researchCompiled = [bool]($summaries['test-research'] -contains 'compile errors: no')
$researchPassed = [bool]($summaries['test-research'] | Where-Object { $_ -match '^research_internals .*: ok, [1-9][0-9]* passed, 0 failed' })
$tDefault = if ($testDefaultRc -eq 0) {
    'PASS'
} elseif ($audit -eq 'PASS' -and $defaultCompiled -and $failures['test-default']) {
    'PRE-EXISTING'
} else {
    'FAIL'
}
$tResearch = if ($researchCompiled -and $researchPassed -and $failures['test-default'] -eq $failures['test-research'] -and ($testResearchRc -eq 0 -or $testDefaultRc -ne 0)) { 'PASS' } else { 'FAIL' }
$defaultFailing = @($summaries['test-default'] | Where-Object { $_ -like 'FAILED *' }).Count
Add-Gate 'cargo test without the feature' $tDefault "exit $testDefaultRc; $($summaries['test-default'][0]); $defaultFailing failing"
Add-Gate 'cargo test with research-internals: research_internals passes, failing tests identical to the run without the feature' $tResearch "exit $testResearchRc; $($summaries['test-research'][0])"

# ---------------------------------------------------------------------------
# 4. Clippy and rustfmt. The warning run (lints reported, not denied) checks
# every target, so its findings are the complete comparison; the -D warnings
# run stops at the first failing crate.
# ---------------------------------------------------------------------------
$clippyWarnDefault = @($Cargo + @('clippy', '--locked', '--workspace', '--all-targets'))
$clippyWarnResearch = @($Cargo + @('clippy', '--locked', '--workspace', '--all-targets', '--features', 'entrybound/research-internals'))
$clippyDefault = @($clippyWarnDefault + @('--', '-D', 'warnings'))
$clippyResearch = @($clippyWarnResearch + @('--', '-D', 'warnings'))
$warnDefaultRc = Invoke-Logged (Join-Path $Logs 'clippy-warn-default.log') $clippyWarnDefault
$warnResearchRc = Invoke-Logged (Join-Path $Logs 'clippy-warn-research.log') $clippyWarnResearch
$strictDefaultRc = Invoke-Logged (Join-Path $Logs 'clippy-strict-default.log') $clippyDefault
$strictResearchRc = Invoke-Logged (Join-Path $Logs 'clippy-strict-research.log') $clippyResearch
$clippy = @{}
foreach ($name in @('clippy-warn-default', 'clippy-warn-research', 'clippy-strict-default', 'clippy-strict-research')) {
    $summaryPath = Join-Path $Logs "$name.summary"
    & $Py[0] $Py[1] $Tools clippy-summary (Join-Path $Logs "$name.log") | Out-File -LiteralPath $summaryPath -Encoding utf8NoBOM
    $clippy[$name] = (Get-Content -Raw -LiteralPath $summaryPath)
}
$cSame = if ($warnDefaultRc -eq 0 -and $warnResearchRc -eq 0 -and $clippy['clippy-warn-default'] -eq $clippy['clippy-warn-research']) { 'PASS' } else { 'FAIL' }
$warnCount = @(Get-Content -LiteralPath (Join-Path $Logs 'clippy-warn-default.summary') | Where-Object { $_ -ne '(no diagnostics)' }).Count
Add-Gate 'clippy findings identical with and without the feature (all targets, lints reported)' $cSame "exit $warnDefaultRc/$warnResearchRc; $warnCount findings without the feature"
$cStrict = if ($strictDefaultRc -eq 0 -and $strictResearchRc -eq 0) {
    'PASS'
} elseif ($audit -eq 'PASS' -and $strictDefaultRc -eq $strictResearchRc -and $clippy['clippy-strict-default'] -eq $clippy['clippy-strict-research']) {
    'PRE-EXISTING'
} else {
    'FAIL'
}
Add-Gate 'clippy --all-targets -D warnings, both ways' $cStrict "exit $strictDefaultRc/$strictResearchRc"
$fmtCmd = @($Cargo + @('fmt', '--all', '--check'))
$fmt = if ((Invoke-Logged (Join-Path $Logs 'fmt.log') $fmtCmd) -eq 0) { 'PASS' } else { 'FAIL' }
Add-Gate 'cargo fmt --all --check' $fmt 'exit 0 required'

# ---------------------------------------------------------------------------
# 5. Report section
# ---------------------------------------------------------------------------
$verdict = if ($script:Failed -ne 0) {
    'FAIL'
} elseif ($script:PreExisting -ne 0) {
    'PASS (pre-existing failures recorded; identical with and without the feature)'
} else {
    'PASS'
}
Add-Md @("### Verdict: $verdict", '', '| Gate | Result | Detail |', '|---|---|---|')
Add-Md @($Gates)
Add-Md @('', 'PRE-EXISTING: the command also fails without the feature, and its results are identical with the feature. The additive-only audit proves that the source compiled without the feature is byte-identical to the base commit, so these failures exist at the base commit in this environment and are not caused by research-internals.')
Add-Md @('', '### Additive-only production diff', '', "``$(Hide-Paths (Format-Command $auditCmd))``", '')
Add-Fence $auditLog
Add-Md @('', '### Builds', '')
Add-Md @("- ``$(Format-Command $buildDefault)`` -> ``ebound-default.exe`` SHA-256 ``$(Get-Sha (Join-Path $Bin 'ebound-default.exe'))``")
Add-Md @("- ``$(Format-Command $buildResearch)`` -> ``ebound-research.exe`` SHA-256 ``$(Get-Sha (Join-Path $Bin 'ebound-research.exe'))``")
Add-Md @('- Binary hashes are informational; the gate is the output identity below.', '')
Add-Md @('### Fixed input tree', '')
Add-Md @("- ``$(Hide-Paths (Format-Command $treeCmd))``")
Add-Md @("- $treeFiles files, $treeBytes bytes, tree digest ``$treeDigest`` (sorted path, length and SHA-256 of every file; mtimes fixed at 2023-11-14T22:13:20Z)")
Add-Md @('- Contents: log text, Markdown, an empty file and directory, counters, zeros, SHA-256 counter-mode random bytes, a gzip DEFLATE stream, a baseline JPEG, 12 similar text records, 24 shared-noise samples.', '')
Add-Md @('### Pack outputs', '')
Add-Md @('Command per cell: `$WORK\bin\ebound-<build>.exe pack $WORK\tree $WORK\out\<build>\<profile>-<layout>.eb --layout <layout> [--stream-window auto for stream] --profile <profile>`', '')
Add-Md @('| Profile | Layout | Bytes | SHA-256 without feature | SHA-256 with research-internals | Identical |', '|---|---|---|---|---|---|')
Add-Md @($packRows)
Add-Md @('', "Coverage of the default build's INDEXED archives (``ebound inspect``, selected lines):", '')
$inspectSummary = Join-Path $Logs 'inspect.summary'
$inspectLines = foreach ($profile in $Profiles) {
    "[$profile]"
    $inspectLog = Join-Path $Logs "inspect-$profile.log"
    if (Test-Path -LiteralPath $inspectLog) {
        Select-String -LiteralPath $inspectLog -Pattern '^(planner|cross-file compression|chunk groups|reconstruction|whole-object reconstruction):' | ForEach-Object { $_.Line }
    }
}
$inspectLines | Out-File -LiteralPath $inspectSummary -Encoding utf8NoBOM
Add-Fence $inspectSummary
Add-Md @('', '### Tests', '')
Add-Md @("- ``$(Format-Command $testDefault)``: $tDefault")
Add-Fence (Join-Path $Logs 'test-default.summary')
Add-Md @("- ``$(Format-Command $testResearch)``: $tResearch")
Add-Fence (Join-Path $Logs 'test-research.summary')
Add-Md @('', '### Clippy and rustfmt', '')
Add-Md @("- ``$(Format-Command $clippyWarnDefault)``: exit $warnDefaultRc")
Add-Fence (Join-Path $Logs 'clippy-warn-default.summary')
Add-Md @("- ``$(Format-Command $clippyWarnResearch)``: exit $warnResearchRc")
Add-Fence (Join-Path $Logs 'clippy-warn-research.summary')
Add-Md @("- ``$(Format-Command $clippyDefault)``: exit $strictDefaultRc")
Add-Fence (Join-Path $Logs 'clippy-strict-default.summary')
Add-Md @("- ``$(Format-Command $clippyResearch)``: exit $strictResearchRc")
Add-Fence (Join-Path $Logs 'clippy-strict-research.summary')
Add-Md @("- ``$(Format-Command $fmtCmd)``: $fmt")
Add-Md @('', "Full logs were written to ``$Logs`` (outside the repository).")

$sectionFile = Join-Path $WorkDir 'section.md'
$Section | Out-File -LiteralPath $sectionFile -Encoding utf8NoBOM
& $Py[0] $Py[1] $Tools write-section $Report windows $sectionFile
Write-Host "research-internals identity check (Windows): $verdict"
$Gates | ForEach-Object { Write-Host $_ }
exit $script:Failed
