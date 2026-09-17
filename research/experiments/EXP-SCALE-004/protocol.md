# EXP-SCALE-004: Staging, extraction failure, crash residue and non-seekable I/O behaviour (status quo)

| Field | Value |
|---|---|
| Status | **READY** (design draft until gate G-A; gate and L7 disclosure as EXP-SCALE-001) |
| Domain | scale (program §13; interfaces with integrity §21/§24 and platform §15) |
| Platforms | Linux x86-64 (WSL2, root, loop-mounted ext4/XFS/btrfs, `setpriv` to drop root); Windows 11 host NTFS non-admin (arm `EXP-SCALE-004-WIN`) |
| Requires timing | false (time-to-first-file is descriptive) |
| Compute estimate | about 20 machine-hours (Linux about 17 h, Windows about 3 h) |
| Disk estimate | about 60 GB peak (b16g STREAM unpack with spill; single-entry boundary 16 GiB) |
| Agent effort | scripted |
| Tooling to build | `research/tools/scale/fault_cell.py` (scenario driver, contract below); `apt-get install inotify-tools` (approved download, records version); reuses `memcap_run.sh`, `synth_tree.py`, EXP-SCALE-001 cell shapes, harness `ebr-inspect-bytes` (frame offsets for corruption) |

## Pre-registration

- **decision_ids:** DEC-ACC-006, DEC-ACC-053, DEC-ACC-055, DEC-INT-039, DEC-PLT-011, DEC-MOD-032, DEC-ACC-022, DEC-ACC-052, DEC-ACC-013.
- **Decision type:** EMPIRICAL for costs and failure outcomes; the security parts of the same decisions (spill location review, progressive-mode security analysis) are EXTERNAL and are not addressed here.
- **od_ids / hc_ids:** PENDING-G-A. Proposal: OD-08 (scratch), OD-07 M07.4 and OD-09 M09.3 (time to first output, descriptive here), OD-04 M04.2 (`reason_code_specificity_fraction` for I/O faults), OD-25 (error actionability census input); HC-05 (MVT-05(d) staging), HC-15, HC-18 (MVT-18(a) regression evidence), HC-06.
- **Method, gate, author, L7:** as EXP-SCALE-001 (generated inputs only).

## Question

What does the status-quo staging design (resident cap plus FIFO spill to `TMPDIR`, stage everything until full verification) cost in disk, memory and time to first output file relative to incumbent pipelines; how does extraction, packing, repacking, export and publish behave under disk exhaustion, uncatchable kills, late corruption, pre-existing destinations, I/O errors, bootstrap limit boundaries and non-seekable inputs and outputs; what residue do those failures leave; and what does a script-level "stage into a sibling, then rename" emulation cost on ext4, XFS, btrfs and NTFS?

## Hypotheses (status quo, from `crates/entrybound/src/ecf/staging.rs`, `stream.rs`, `filesystem.rs`, CLI; `[INFERRED]`)

- **H1a:** STREAM unpack creates no destination object before full verification, both with late corruption and under kill (F-11, MVT-05(d)); time to first output file therefore grows linearly with archive size, while `zstd -dc | tar -x` emits its first file within seconds regardless of size.
- **H1b:** spill files use `std::env::temp_dir()`, default umask permissions, and are removed on normal exit and on error but **left behind after SIGKILL**.
- **H1c:** no `pack`, `repack` or `export` output interrupted by SIGKILL at any sampled point verifies as `OK`, and no pre-existing destination object is modified; interrupted multi-target `publish` can leave a strict subset of targets (partial publish, MVT-18(a) violation) unless it commits all-or-nothing (F-22).
- **H1d:** disk exhaustion in `TMPDIR` or destination yields a typed refusal, but I/O failures map to `POLICY_REFUSED EB_IO` without distinguishing ENOSPC, EACCES, EROFS, EPIPE and EMFILE (DEC-MOD-032 status-quo mapping).
- **H1e:** the entry-count cap (1,000,000), single-entry cap (16 GiB) and total-logical-bytes cap (64 GiB) of the bootstrap policy are enforced only after full capture, so above-cap inputs consume plaintext-proportional memory (and are OOM-killed under a cap) before any typed refusal.
- **H1f:** a sibling-stage-then-rename emulation costs no extra data bytes on one filesystem, fails with EXDEV across filesystems (copy fallback doubles disk), and on NTFS changes inherited ACLs of the final directory relative to direct extraction.
- **H0 / falsification:** each H1 part is falsified by one reproducible counterexample recorded as an artifact (for example a destination object created before a late corruption is detected, a spill file left after a normal error exit, a typed refusal issued before capture completes).

## Decisions informed

| Decision | Addressed here | Routed elsewhere |
|---|---|---|
| DEC-ACC-006 | Disk and memory overhead, time to completion and failure behaviour (disk full, kill) of the status-quo design on multi-GiB archives; cross-filesystem rename behaviour on NTFS/ext4/XFS/btrfs | alternative staging designs: EXP-SCALE-008; security review of spill location: external review (not measurable) |
| DEC-ACC-053 | Kill/crash cleanup tests; spill permissions under umask 022 and 077 and Windows TEMP ACLs | ephemeral-key encryption, unlink-on-create and concurrent accounting prototypes and the >64 GiB cumulative false-refusal case: EXP-SCALE-008; security review: external |
| DEC-ACC-055, DEC-INT-039 | Staging disk/memory and time to first file vs `zstd | tar` on large archives; crash and late-corruption behaviour of the status quo | quarantine and per-entry verified designs: EXP-SCALE-008; security analysis of progressive modes: external |
| DEC-PLT-011 | Cost of staging via sibling-and-rename (disk doubling, EXDEV, directory fsync, Windows rename, ACL/label divergence) | share of real failures preventable by preflight, and rollback safety analysis: analytical (platform domain) |
| DEC-MOD-032 | Fault injection (disk full, permission denied, read-only destination, pipe close, descriptor exhaustion) recording reported class and code | network and range-server faults: remote domain; diagnostic usability review: human participants |
| DEC-ACC-022 | Non-seekable behaviour census: every command with `-` input or output, INDEXED from stdin, `--layout indexed` to stdout, zero bytes emitted before refusal | first-use study and frequency survey: human participants / ecosystem domain |
| DEC-ACC-052 | Behaviour at and one unit above each bootstrap policy cap (entries, single entry, total logical bytes) | adversarial undeclared-budget archives: EXP-SCALE-006 |
| DEC-ACC-013 | Atomic multi-target publish under kill | — |

## Candidates

Status-quo scenarios (ebr candidates), each run by `fault_cell.py --scenario <name>`; the production `ebound` binary is EXP-SCALE-001's. Incumbent references are included where a comparison is part of the question.

| candidate | Procedure (all commands under `memcap_run.sh --cap 25769803776` unless stated) | Oracle / recorded outputs |
|---|---|---|
| `ttff-unpack-stream-stdin` | `inotifywait -m -r -e create --timefmt %s%N` on the destination parent, then `ebound unpack - DEST` with stdin = balanced STREAM archive | first create event time − start (TTFF), total wall, `peak_rss_bytes`, `scratch_write_bytes` (tmp loop mount), content round trip |
| `ttff-unpack-indexed` | same with `ebound unpack A.eb DEST` | same |
| `ttff-inc-zstd-tar` | same with `zstd -dc A.tar.zst | tar -C DEST -xf -` | same (REF-INC) |
| `diskfull-tmp` | STREAM unpack with `TMPDIR` on a 512 MiB ext4 loop image | outcome, reason code, destination objects present (must be none), leftover spill files and bytes |
| `diskfull-dst` | INDEXED and STREAM unpack into a destination loop image of 50% of the logical size | outcome, code, objects left, whether any left object is byte-identical to its source |
| `diskfull-pack` | `ebound pack IN OUT.eb` with OUT on a loop image of 25% of the expected archive size | outcome, code, partial OUT present, `ebound verify` of any partial OUT (must not be OK) |
| `kill-<op>` for op in {`unpack-stream`, `unpack-indexed`, `pack-indexed`, `pack-stream-stdout`, `repack`, `export-zip`, `publish`} | a calibration run records the uncapped wall W; then for each of 25 kill fractions f drawn from `random.Random(seed)` uniform on [0.02, 0.98], rerun and send SIGKILL to the process group at f × W | per kill: destination objects present; any output that `ebound verify` (or the incumbent reader for exports) reports OK and complete (violation); publish target subset present (violation if strict non-empty subset); spill files left; destination objects created before verification (violation for unpack) |
| `corrupt-last-chunk-stream`, `corrupt-last-chunk-indexed` | flip one byte in the payload of the last ChunkData frame located with `ebr-inspect-bytes --frames` (flag to add if absent: then use the stored length from `inspect --json --chunks` and the section table); unpack while `inotifywait` watches DEST | outcome CORRUPT with stable code; zero create events under DEST at any time (MVT-05(d)) |
| `exclusive-create` | pre-create a sentinel at the output path (pack, export, publish base name) and a non-empty DEST for unpack | refusal, sentinel SHA-256 and mtime unchanged |
| `boundary-entries` | pack `entries` trees with 1,000,000 and 1,000,001 files of 64 B | outcome and code per size; `peak_rss_bytes`; wall to refusal |
| `boundary-single-entry` | pack one file of 2^34 and 2^34 + 1 bytes (half content) under cap24 and under cap2 | same |
| `boundary-logical` | pack 64 files totalling 2^36 + 2^20 bytes under cap24 | same (H1e expects OOM kill before refusal) |
| `sibling-rename-<fs>` for fs in {ext4, xfs, btrfs} | on a fresh loop image of that filesystem: parent directory with default ACL `setfacl -d -m u:nobody:r` and (ext4) `security.selinux` xattr set; (a) `ebound unpack A.eb P/final`; (b) `ebound unpack A.eb P/.stage-<seed>` then `mv -T P/.stage-<seed> P/final2`; (c) stage on another filesystem then `mv` (EXDEV fallback) | extra device write bytes of (b) and (c) vs (a); rename wall (descriptive); `getfacl -R` and `getfattr -d -m -` diff between `final` and `final2`; EXDEV observed |
| `spill-permissions` | STREAM unpack of `b4g` under umask 022 and 077; `stat -c %a %U` of every `entrybound-stage-*` file sampled every 50 ms | modes observed; world- or group-readable spill (recorded, not judged here) |
| `io-fault-<kind>` for kind in {`eacces`, `erofs`, `epipe`, `emfile`, `enospc`} | EACCES: destination directory mode 0555 with the command run via `setpriv --reuid=65534 --regid=65534 --clear-groups`; EROFS: destination mounted `-o ro`; EPIPE: `ebound pack IN - --layout stream | head -c 1024 > /dev/null`; EMFILE: `prlimit --nofile=16:16`; ENOSPC: as `diskfull-dst` | outcome class, reason code, stderr text; the case list is fixed here (n = 5 per operation × {pack, unpack, convert export}) |
| `nonseekable-census` | for each command in {list, inspect, explain, verify, read, unpack, diff} with input `-`; for each in {pack, repack, convert import, convert export, publish} with output `-`; INDEXED archive on stdin; `pack --layout indexed -`; STREAM archive on stdin to `read` | outcome, code, bytes written to stdout before refusal (must be 0 for refusals), `peak_rss_bytes` |

**Windows arm (`EXP-SCALE-004-WIN`, `spec-windows.yaml`)**, host NTFS, non-admin, `ebound.exe` built from the same commit with `%USERPROFILE%\.cargo\bin\cargo.exe +1.98.1 build --release -p entrybound-cli --locked` into `D:/eb-research/target/scale-prod-win`: `sibling-rename-ntfs` (stage under `D:\eb-research\scale\P\.stage-<seed>`, `Move-Item`, `icacls` diff against direct extraction, retries on sharing violations counted), `spill-permissions-windows` (`Get-Acl` of `%TEMP%\entrybound-stage-*`), `kill-unpack-stream` (job termination via the runner's job object, `taskkill /F /T`), `exclusive-create`.

- **Excluded:** alternative staging designs (not implemented; EXP-SCALE-008); kills at research-build write-phase boundaries (MVT-18(a) requires phase reporting in a research build; seeded interior points here are regression evidence only, noted in the HC-18 status); EIO injection (needs device-mapper `dm-error`, absent in the WSL kernel; documented gap).

## Corpus selectors (generated)

EXP-SCALE-001 shapes and seeds, reused byte for byte: `b1g`, `b4g`, `b16g`, `e4g`, `n1e5`; plus boundary shapes `entries` with 1,000,000 and 1,000,001 files of 64 B (seeds 1431, 1432), single files of 2^34 and 2^34 + 1 bytes (seeds 1433, 1434), `files` with 64 files and 2^36 + 2^20 total (seed 1435). Manifest `research/experiments/EXP-SCALE-004/cells.jsonl` (generated by `make_cells.py`); applicability of scenarios to cells is in `cells.json`. **Held-out placeholder:** none (generated inputs).

## Environment

As EXP-SCALE-001, plus: loop images for XFS (`mkfs.xfs -q`) and btrfs (`mkfs.btrfs -q`); `inotify-tools` from Ubuntu 24.04 apt (version recorded); dropping root with `setpriv` for permission faults (root bypasses DAC). Windows arm: `env_id` `windows-host`, Defender on (recorded), scratch under `D:\eb-research\scale` (outside the repository), non-admin.

## Commands

```sh
cd /mnt/d/Projects/entrybound/entrybound; export PYTHONPATH=research/tools; PY=/root/eb-research/venv/bin/python
$PY research/experiments/EXP-SCALE-004/make_cells.py
$PY -m ebr validate --spec research/experiments/EXP-SCALE-004/spec.yaml
$PY -m ebr run --spec research/experiments/EXP-SCALE-004/spec.yaml && $PY -m ebr verify-raw --spec research/experiments/EXP-SCALE-004/spec.yaml
$PY -m ebr normalize --spec research/experiments/EXP-SCALE-004/spec.yaml
```
Windows (PowerShell 7, repository root): `$env:PYTHONPATH='research/tools'; C:/Python313/python.exe -m ebr run --spec research/experiments/EXP-SCALE-004/spec-windows.yaml` (ebr's Windows job-object path; the research venv's `psutil` must be importable by that interpreter), then `verify-raw` and `normalize` likewise.

Driver contract: `fault_cell.py --experiment EXP-SCALE-004 --cells <cells.json> --cell {item_id} --scenario {candidate} --rep {rep} --seed {seed} --scratch {scratch}` prints one `ebr.scale.fault.v1` line; per-kill details (25 rows per kill scenario) go to `{scratch}/kills.jsonl`, whose SHA-256 is a payload field and whose file is copied to `research/raw/EXP-SCALE-004/kills/<run>/<sample>.jsonl` by the driver (raw-adjacent artifact, hash-bound through the row).

## Seeds

`seeds.order` 1304001, `seeds.bootstrap` 1304002, `seeds.command` 1304003 (kill fractions and stage-directory suffixes derive from the per-sample `{seed}`).

## Warmup

0. TTFF scenarios pre-read the archive (hot cache, §4.5).

## Measurement method and metrics

| Payload key | Canonical name | Role | Family |
|---|---|---|---|
| `outcome`, `reason_code`, `stderr_class_text` | none | outcome | correctness |
| `dest_objects_before_verify` | none; any value > 0 for unpack is an HC-05/MVT-05(d) violation | binary | correctness |
| `verified_ok_partial_outputs` | none; > 0 is an HC-18 MVT-18(a) violation | binary | correctness |
| `partial_publish_sets` | none; > 0 is an HC-18 violation | binary | correctness |
| `sentinel_modified` | none; > 0 is an HC-18 violation | binary | correctness |
| `spill_leftover_files`, `spill_leftover_bytes` | none | secondary | scratch_peak |
| `scratch_write_bytes` | `scratch_write_bytes` | secondary here (not a claim test) | correctness |
| `peak_rss_bytes` | `peak_rss_bytes` (T-06) | secondary | memory_peak |
| `ttff_s`, `op_wall_s` | none (descriptive; M07.4/M09.3 decision-grade values need in-process timers and timing windows) | descriptive | time_wall |
| `extra_write_bytes_vs_direct`, `exdev_observed`, `acl_diff_count`, `xattr_diff_count`, `rename_retries` | none | secondary | io, other |
| `spill_mode_octal` | none | descriptive | other |
| `reason_code_specific` | input to `reason_code_specificity_fraction` (T-20) over the fixed I/O fault case list | primary for DEC-MOD-032 status-quo row | other |
| `stdout_bytes_before_refusal` | none; > 0 on a refused non-seekable case violates "refuse before output" | binary | correctness |

## Replication

3 repetitions per (scenario, cell); kill scenarios run 25 seeded kill points per repetition (75 per operation per cell). Binary outcomes: one violation suffices (objective §2.0 rule 2). Detection statement: the 75 interior kill points are **regression evidence** (MVT-18(a) requires exhaustive research-build phase boundaries for a bound).

## Normalization and statistical analysis

- Violation tables per HC (HC-05, HC-15, HC-18) with committed artifacts for every violation (the killed state is preserved by copying the destination listing, partial outputs up to 1 GiB, and spill listing into `research/raw/EXP-SCALE-004/artifacts/`).
- TTFF and staging overhead: per cell, medians over repetitions, ratio `ttff(ebound STREAM) / ttff(zstd|tar)` reported descriptively with absolute values; no verdict (timing false).
- Sibling-rename cost: exact device write deltas; ACL/xattr diffs as counts with examples.
- `reason_code_specificity_fraction` over the n = 15 pre-registered I/O fault cases (5 kinds × 3 operations), T-20 band a = 0.5/15; status-quo value only.
- Boundary table: outcome, reason code, `peak_rss_bytes`, wall to outcome for each cap at the boundary and one unit above.

## Practical-significance thresholds

Referenced: T-20 for `reason_code_specificity_fraction`; binary HC oracles have no band (§3.3). No threshold restated.

## Sensitivity analysis

- Archive size arm for TTFF (`b1g`, `b4g`, `b16g`) and entry-count arm (`n1e5`).
- umask arm (022, 077) for spill permissions.
- Kill-point distribution arm: repeat `kill-unpack-stream` with fractions concentrated on the last 10% of W (seeded) to sample the commit window densely.

## Expected negative results worth recording

Spill files left after kills; time to first file tens to hundreds of times the incumbent pipeline's on multi-GiB archives; partial publish; boundary refusals issued only after plaintext-proportional capture (or OOM kills instead of refusals); identical `EB_IO` codes for distinct I/O faults; NTFS ACL divergence after rename.

## Threats to validity

1. Kill timing by wall-clock fraction is coarse; the phase-boundary oracle needs a research build (HC-18 remains HC_UNVERIFIED for exhaustive coverage).
2. Loop-mounted filesystems approximate real devices; XFS/btrfs behaviour under ENOSPC may differ on real disks.
3. `setpriv` drops root but keeps the same namespace; SELinux is not enforcing in WSL, so label behaviour is metadata-only.
4. Windows: Defender scanning can cause rename sharing violations; counted as `rename_retries`, never retried silently more than the pre-registered 5 times.
5. Synthetic trees (EXP-SCALE-001 threat 5).

## Platform requirements

Linux x86-64 WSL2 root (loops, cgroups, `setpriv`); Windows 11 NTFS non-admin; about 60 GB free; no network; no external review for the parts addressed.

## Estimated compute and effort

About 20 h (kill scenarios dominate: 7 operations × 75 kills × up to 4 GiB inputs; boundary cells about 2 h). Scripted.

## Deviations (append-only)

None.
