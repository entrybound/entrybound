# EXP-DET-SMOKE: cross-host determinism smoke

Status: **smoke, non-decision-grade** (no timing recorded; this checks byte and
identity determinism, not performance). Not attached to any decision id.

## Method

1. **Fixed input tree.** The same checked-in, seeded generator already used by
   `research/harness/internals-identity-check.sh`/`.ps1`
   (`research/harness/internals_identity_tools.py tree <dest>`) materializes a
   44-file, 3,104,966-byte tree with fixed mtimes
   (`2023-11-14T22:13:20Z`) and deterministic xorshift/counter-based content.
   Both hosts produced a byte-identical tree: tree digest
   `2c1ffcf617f9506e27330d4ff85eacc1471be456e51ac787628d616790ec86b0`
   (`<file count> <total bytes> <sha256 of sorted path/length/sha256 tuples>`,
   see `wsl-x86_64/tree-digest.txt` and `windows-x86_64/tree-digest.txt`).
2. **Pack.** `research/harness/crates/ebr-pack`'s `ebr-pack` binary (release
   build, pinned toolchain 1.98.1, `CARGO_TARGET_DIR=.../target/harness`)
   packed that tree under all 4 profiles (`fast`/`balanced`/`dense`/`extreme`)
   x both layouts (`indexed`/`stream`) = 8 cells, on:
   - WSL Ubuntu (x86_64): `ebr-pack --item-path <tree> --experiment-id
     EXP-DET-SMOKE --env-name wsl-ubuntu --profile <p> --layout <l>
     [--stream-window 100000] --archive-out <out>.eb`
   - Windows host (x86_64): the same command via
     `D:\eb-research\target\harness\release\ebr-pack.exe`, `--env-name
     windows-host`.
   - **`--stream-window`**: `ebr-pack`'s STREAM path only exposes a fixed
     `StreamWindow::Ceiling(n)` (no `Auto`, unlike the production CLI's `pack
     --stream-window auto`); this tree's minimum required ceiling differs by
     profile (33 for `fast`, at least 46 for others), so all STREAM cells here
     use a generously large fixed ceiling (`100000`) instead. See
     `research/harness/README.md`'s known limitations.
   - **linux/arm64 (emulated via Docker)**: **DEFERRED**. Docker Desktop is
     unavailable on this host (stale AF_UNIX socket / Win32 error 1920 at
     startup; see `PROGRESS.md`'s 2026-09-16 storage incident). Per the task's
     standing instructions, Docker was not started, restarted, stopped, or
     reset to check this.
3. **Native identities.** For every produced archive, the production CLI
   (`entrybound-cli`, binary `ebound`, release build from
   `crates/entrybound-cli`, **not** the harness's `research-internals`
   feature) ran `ebound inspect <archive> --json` and the script extracted
   `identities.{lai,aux,pcr,pci}` from the fixed-order inspection JSON
   (`entrybound::archive::inspection_json`).
4. Every cell's artifact SHA-256, byte count, and the four identities are in
   `<host>/cells.csv` (columns: `profile,layout,bytes,sha256,lai,aux,pcr,pci`),
   the raw `ebr-pack` JSON rows are in `<host>/pack-rows.jsonl`, and the full
   `ebound inspect --json` output for each cell is in `<host>/inspect/`.

## Result

See `research/normalized/EXP-DET-SMOKE/summary.csv` for the full cross-host
comparison. Summary:

- **PCR (physical content root) is byte-identical across WSL and Windows for
  all 8 cells.** Content-defined chunking and codec/dictionary/group
  selection are platform-independent for this tree, at every profile and
  layout.
- **Artifact bytes, SHA-256, LAI, and AUX all differ between WSL and Windows,
  for every cell.** This is an expected, already-documented finding, not a
  new defect:
  - The two hosts' `ebound inspect --json` output has
    `archive.features.incompat` = `32771` (WSL) vs `98307` (Windows); the
    difference is exactly `65536` (`0x10000`), matching `PROGRESS.md`'s
    Finding F-0001 ("host-dependent required feature bits (Windows always
    requires 0x10000 platform-security feature)").
  - Per-entry metadata is genuinely different native data, not a
    determinism bug: on WSL every entry's `posix.{mode,uid,gid}` is populated
    and `platform_security.windows_*` fields are `null`; on Windows
    `posix.{mode,uid,gid}` is `null` and `platform_security.
    windows_file_attributes`/`windows_creation_time` are populated instead.
    Since LAI and AUX are computed over exactly this per-entry metadata (see
    `crates/entrybound/src/identity.rs`), and the content generator only
    fixes mtimes (not permission bits or Windows attributes, which do not
    exist symmetrically on both platforms), LAI/AUX are expected to differ
    cross-host for any tree materialized natively on each OS.
  - The artifact byte-count difference (e.g. `fast`/`indexed`: 2,211,070 WSL
    vs 2,198,820 Windows, 12,250 bytes smaller) follows from the same
    per-entry metadata shape difference plus the feature-bit change; PCR
    (content bytes only) is unaffected.

This does not call production correctness or the identity model into
question: LAI/AUX are *designed* to capture exactly this metadata, and doing
so correctly means they must differ when the same logical tree is captured
natively on two operating systems with different metadata models. It does
mean **any cross-host reproducibility claim in this program must be scoped to
PCR/content-addressing, or must normalize/exclude platform metadata**, if it
is to hold for LAI/AUX/whole-container-byte comparisons. No production code
was changed to investigate or report this.
