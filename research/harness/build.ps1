<#
.SYNOPSIS
    Builds (and by default tests) the research harness workspace on Windows,
    with the pinned research Rust toolchain (1.98.1) and its own target
    directory, kept out of both git and the production workspace's target/.

.DESCRIPTION
    With no arguments, runs `cargo +1.98.1 test --workspace`. Any arguments
    given replace that default entirely and are passed to cargo as-is.

    $env:CARGO_TARGET_DIR defaults to D:/eb-research/target/harness: a
    shared, stable location for this one workspace, not the per-agent
    D:/eb-research/target/b2-<label> convention other Phase B2 agents use
    while several agents build concurrently. Set $env:CARGO_TARGET_DIR
    yourself before calling this script if you are building alongside
    another agent and want to avoid lock contention on the shared default.

    $env:CARGO_BIN defaults to %USERPROFILE%\.cargo\bin\cargo.exe (the
    rustup proxy; plain `cargo`/`rustc` are not on the default PATH on this
    host -- see research/PROGRESS.md).

.EXAMPLE
    research/harness/build.ps1
.EXAMPLE
    research/harness/build.ps1 build --release
.EXAMPLE
    research/harness/build.ps1 -p ebr-common
.EXAMPLE
    research/harness/build.ps1 clippy --workspace --all-targets

.NOTES
    Deliberately has no declared `param()` block: every argument (including
    ones that look like PowerShell-style flags, e.g. `-p ebr-common`) must
    reach cargo untouched, in `$args`, rather than risk PowerShell's
    named-parameter binder matching a cargo flag against a common parameter
    (`-p` is ambiguous with `-ProgressAction`) or otherwise consuming it.
#>

$ErrorActionPreference = "Stop"

$HarnessDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$CargoBin = if ($env:CARGO_BIN) { $env:CARGO_BIN } else { Join-Path $env:USERPROFILE ".cargo\bin\cargo.exe" }
if (-not $env:CARGO_TARGET_DIR) {
    $env:CARGO_TARGET_DIR = "D:/eb-research/target/harness"
}
$CargoArgs = if ($args.Count -eq 0) { @("test", "--workspace") } else { $args }

Write-Host "harness dir:      $HarnessDir"
Write-Host "CARGO_TARGET_DIR: $($env:CARGO_TARGET_DIR)"
Write-Host "toolchain:        1.98.1 ($CargoBin +1.98.1)"
Write-Host "command:          cargo +1.98.1 $($CargoArgs -join ' ')"

Push-Location $HarnessDir
try {
    & $CargoBin "+1.98.1" @CargoArgs
    if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
    }
}
finally {
    Pop-Location
}
