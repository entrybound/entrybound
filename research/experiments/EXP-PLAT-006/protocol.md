# EXP-PLAT-006: Timestamp granularity, range and out-of-range capture and restore per filesystem

> **Design pre-registration draft** (Phase C-design, platform domain). Index status: **READY**. Structure: decision-method §12.2 record fields plus the Phase C-design protocol fields. Shared provisions PCP-1…PCP-8 live in `research/experiments/_index/platform-common-provisions.md` and are part of this pre-registration. This file becomes the §12 `record.md` only after the gates are met.

## Pre-registration

- **decision_ids:** DEC-PLT-050, DEC-PLT-051, DEC-PLT-049, DEC-MOD-050, DEC-PLT-003, DEC-LEG-088
- **Program sections:** 15, 21, 25
- **Decision type (proposed, not assigned):** EMPIRICAL (granularity, range and restore behaviour); FORMAL for the time-scale definition part of DEC-PLT-050 (vectors from EXP-PLAT-021 primary sources). Assignment is made by a session not involved in the candidates (decision-method §1.2).
- **od_ids / hc_ids (proposed for the schema-v2 assignment, §14 item 12):** OD-03 (M03.1, M03.2), OD-18; HC-07 (silent clamping), HC-13 (precision determinism), HC-01.
- **Method:** `research/decision-method.md` and `research/methods/thresholds.json` at pre-registration commit `14b977c` (status `pre-registered`).
- **Pre-registration commit:** the checkpoint commit adding this file (design draft); the §12 record commit follows G-A; every run's raw `meta` git SHA must descend from it with `dirty = false` (§5.8 d).
- **Author, independence and exposure:** Phase C-design platform-domain designer (workflow `wf_47347bc4-611`); no candidate was constructed by this session (ledger R0 sets reproduced mechanically). Held-out identity exposure (event L7) is disclosed in **PCP-1**.
- **Public data sources used for tuning (§5.4 consequence 2):** upstream sources in `research/corpus/manifest.json` `provenance` for the tuning/validation items named below, plus the pinned tool downloads named under Commands.
- **Gates:** G-A to G-D as in **PCP-2**.

## Question and hypotheses

- **Question:** What granularity, representable range and out-of-range behaviour does each filesystem and restore API exhibit for mtime, atime, ctime and birth/creation time, which precision classes and time-scale rules capture every source exactly without recording an unobservable precision, and does restoration ever clamp or round silently?
- **H1:** Granularities are ext4 1 ns (256-byte inodes) and 1 s with a 2038 upper bound (128-byte inodes), XFS 1 ns with bigtime range beyond 2446 and without it 2038, btrfs 1 ns, tmpfs 1 ns, vfat 2 s mtime in local or UTC per mount option, NTFS 100 ns; at least one Linux target clamps an out-of-range restore silently (smoke observation on 128-byte inodes), so a report-only out-of-range policy that relies on the kernel's error returns misses clamps (HC-07 violation) unless the restorer reads back and compares; the five-class precision registry cannot represent the 2-second and 10-millisecond sources exactly.
- **H0 / equivalence:** Every filesystem returns an error for unrepresentable values (no silent clamping), and the status-quo registry represents every measured granularity exactly.
- **Falsification of H1:** No silent clamp on any target across the value ladder, or every measured granularity maps exactly to one of the five existing classes.

## Decisions informed

| Decision | Release relevance | Ledger question (abridged; full text in the ledger) | What this experiment contributes | Evidence still needed elsewhere |
|---|---|---|---|---|
| DEC-PLT-050 | V1_IMPORTANT | What is the normative time scale of Timestamp (Unix epoch, POSIX seconds without leap seconds, UTC), how are zone-less local times, leap seconds and values outside a t... | Platform restore behaviour for negative, pre-1601, post-2038, post-2446 and far-future values on ext4, XFS, btrfs, vfat, ntfs-3g and NTFS; leap-second representability. | Independent-reader ambiguity log (conformance domain); normative vectors from primary sources (EXP-PLAT-021). |
| DEC-PLT-051 | V1_IMPORTANT | Which source-precision classes does the timestamp registry include (add two-second, millisecond, hour, unknown?), how do legacy imports treat granularities outside it,... | Measured granularity of every reachable source (filesystems, ZIP DOS/UT/NTFS extras, pax fractional digits per writer) against each precision-class candidate. | HFS+ (EXP-PLAT-022), exFAT native (EXP-PLAT-023); diff false-change rates (EXP-PLAT-005). |
| DEC-PLT-049 | V1_IMPORTANT | Which timestamps beyond mtime are captured, declared or restored in v1 (atime, ctime, Linux statx btime capture-only, Windows creation-time restore, macOS birthtime re... | statx btime availability per Linux filesystem; creation-time set via .NET and std `FileTimes` on NTFS; atime/ctime observability. | AUX churn (EXP-PLAT-005); macOS birthtime (EXP-PLAT-022). |
| DEC-MOD-050 | V1_IMPORTANT | Should timestamp precision recorded from the capturing filesystem be canonicalized (or excluded) so identical instants yield identical AUX across hosts? | Restoration fidelity implications per platform of recording host precision versus canonical precision. | AUX stability (EXP-PLAT-005). |
| DEC-PLT-003 | V1_IMPORTANT | Should capture detect and record source filesystem type and per-filesystem timestamp granularity and range (FidelityReport.filesystem, per-filesystem precision tags),... | Accuracy of observed-granularity inference from captured values versus measured granularity. | Detection APIs (EXP-PLAT-001/016). |
| DEC-LEG-088 | V1_IMPORTANT | Should ZIP import keep dropping DOS local timestamps (status quo), import them under an explicit recorded timezone assumption, store them as zone-less AUX metadata, an... | Fidelity tests of UT (0x5455) and NTFS (0x000a) extras: which of atime/ctime/mtime are captured, declared unavailable, or silently dropped by ZIP import. | Real-ZIP prevalence (EXP-PLAT-003); usability of lost mtimes (HUMAN-FACING). |

## Candidates

Candidate sets are the ledger's R0 sets; completeness (status quo, SPEC design, named alternatives, strongest incumbent, defer, critic candidate, corrective candidate) is checked by the R0 completeness critic, not by this design. Where a set lacks a strongest-incumbent reference, the incumbent systems measured below act as REF-INC for the metric (objective §3.0).

- **Reference candidate:** Status quo production behaviour (`status-quo-implicit-unix-utc`, `status-quo-five-classes`, `status-quo-mtime-plus-platform-birth`, `status-quo-host-precision`, `status-quo-os-derived-precision`, `status-quo-drop-dos`).
- **W and S:** determined on tuning (R3/R4) and declared in the §12 record before any validation look (§4.12).

Descriptions of every candidate are in the ledger row (`candidate_set`); they are not repeated here.

- **DEC-PLT-050** — candidates: `status-quo-implicit-unix-utc`, `explicit-posix-time-no-leap`, `tai-based-scale`, `drop-timestamp-restorable-flag`, `validate-flag-against-registry`, `refuse-out-of-range-restore` (status quo: `status-quo-implicit-unix-utc`). Treatment: Scored against the measured ground truth of this experiment.
- **DEC-PLT-051** — candidates: `status-quo-five-classes`, `add-two-second-and-millisecond`, `add-unknown-precision`, `exponent-encoded-precision`, `refuse-out-of-registry-precision`, `truncate-with-declared-loss` (status quo: `status-quo-five-classes`). Treatment: Scored against the measured ground truth of this experiment.
- **DEC-PLT-049** — candidates: `status-quo-mtime-plus-platform-birth`, `declare-unavailable-only`, `add-linux-btime-capture-only`, `add-atime-optional`, `add-ctime-capture-only`, `exclude-creation-time-by-default`, `restore-creation-time-and-birthtime` (status quo: `status-quo-mtime-plus-platform-birth`). Treatment: Scored against the measured ground truth of this experiment.
- **DEC-MOD-050** — candidates: `status-quo-host-precision`, `canonical-precision`, `precision-out-of-aux` (status quo: `status-quo-host-precision`). Treatment: Scored against the measured ground truth of this experiment.
- **DEC-PLT-003** — candidates: `status-quo-os-derived-precision`, `detect-fs-type-and-map`, `record-fs-type-only`, `unknown-precision-when-undetected`, `observed-granularity-inference` (status quo: `status-quo-os-derived-precision`). Treatment: Scored against the measured ground truth of this experiment.
- **DEC-LEG-088** — candidates: `status-quo-drop-dos`, `assume-timezone-flag`, `assume-utc-default`, `zoneless-aux-metadata`, `report-extra-times`, `refinement-per-spec` (status quo: `status-quo-drop-dos`). Treatment: Scored against the measured ground truth of this experiment.
  - `status-quo-drop-dos`: Measured directly through `ebound convert --strict`.
  - `refinement-per-spec`: Scored by whether the most precise compatible value is recoverable from each fixture.
  - excluded before execution (ledger): `silent-atime-ctime-drop` — Discarding extra-field times without a typed record is silent metadata loss (violates I13); L0/R1(b) counterexample and reviewer sign-off still required (R1).

## Corpus, fixtures and case sets

- **Value ladder** (committed as `ladder.json`): seconds {−62135596800, −11644473601, −11644473600, −2147483649, −2147483648, −1, 0, 1, 315532799, 315532800, 2147483647, 2147483648, 4294967296, 15032385535, 15032385536, 16725225600, 253402300799, 910692730085} × nanoseconds {0, 1, 99, 100, 500000000, 999999999}; tuning and validation use the same ladder with different seeded draws of an additional 64 random values each (seeds 20260917 / 20260918; triggers `plat006-ladder-tuning`, `plat006-ladder-validation`).
- **Time-zone arm:** vfat mounted with `tz=UTC` and with the default local-time interpretation under `TZ` values {UTC, America/Los_Angeles, Asia/Kolkata, Pacific/Kiritimati}.
- **Legacy fixtures** (`scripts/legacy_time_fixtures.py`): ZIPs with DOS-only times, with UT extras (mtime only; mtime+atime+ctime in the local header), with NTFS extras (three times at 100 ns) written by Python `zipfile` (raw extra fields), Info-ZIP `zip` (default and `-X`) and `7z`; pax archives from GNU tar 1.35, bsdtar 3.7.2 and Python 3.12 `tarfile` from the ladder tree, parsed for fractional digits.
- **Splits:** tuning and validation only; validation looks are registered in `research/decisions/validation-looks.jsonl` (§5.2, at most two per decision). Generated fixtures are assigned to splits by seed as stated above. No spec selects held-out (Phase D placeholder below).

## Environment and platform requirements

- **Linux targets:** `ext4`, `ext4-i128`, `xfs-bigtime`, `xfs-nobigtime`, `btrfs`, `tmpfs`, `vfat-utc`, `vfat-localtime`, `ntfs3g`, `exfat-fuse`, `drvfs-control` (via `EXP-PLAT-001/scripts/fsimg.sh`; `exfat-fuse` optional).
- **Windows targets:** `ntfs-d-dotnet` ([IO.File]::SetLastWriteTimeUtc/SetCreationTimeUtc) and `ntfs-d-std-filetimes` (EXP-PLAT-016 probe).
- **env_ids:** `wsl-ubuntu`, `windows-host`.

| Requirement | Needed? |
|---|---|
| Linux (WSL2 Ubuntu 24.04, kernel 5.15.167.4, root) | yes |
| Windows 11 host (non-admin, NTFS D:) | yes |
| macOS | no (macOS counterpart is EXP-PLAT-022, BLOCKED) |
| x86-64 | yes |
| ARM64 | no (native ARM64 parity is EXP-PLAT-025, BLOCKED) |
| Network | no (downloads of pinned tools only, approved) |
| Privileged | WSL root for loop mounts; Windows non-admin only |
| Large storage | no |
| External review | no |

- **Storage discipline:** **PCP-6**.

## Commands

- **Production CLI (Linux):** `rm -rf /root/eb-research/src/entrybound-plat && git clone /mnt/d/Projects/entrybound/entrybound /root/eb-research/src/entrybound-plat && git -C /root/eb-research/src/entrybound-plat checkout <record-commit> && cd /root/eb-research/src/entrybound-plat && CARGO_TARGET_DIR=/root/eb-research/target/production /root/.cargo/bin/cargo +1.98.1 build --release -p entrybound-cli --bin ebound` → `/root/eb-research/target/production/release/ebound` (production workspace, no `research-internals`; harness review R1-06).
- **Production CLI (Windows):** `git clone D:/Projects/entrybound/entrybound D:/eb-research/src/entrybound-plat; git -C D:/eb-research/src/entrybound-plat checkout <record-commit>; $env:CARGO_TARGET_DIR='D:/eb-research/target/production-win'; & "$env:USERPROFILE/.cargo/bin/cargo.exe" +1.98.1 build --release -p entrybound-cli --bin ebound` → `D:/eb-research/target/production-win/release/ebound.exe`.
- **Pre-flight:** `C:/Python313/python.exe research/orchestration/disk_guard.py` must pass; no other workflow may hold the loop devices or mount points named here.
- **ebr (WSL):** `wsl.exe -d Ubuntu -u root -- bash -lc 'cd /mnt/d/Projects/entrybound/entrybound && export PYTHONPATH=research/tools && PY=/root/eb-research/venv/bin/python && $PY -m ebr validate --spec research/experiments/EXP-PLAT-006/spec.yaml && $PY -m ebr run --spec research/experiments/EXP-PLAT-006/spec.yaml && $PY -m ebr verify-raw --spec research/experiments/EXP-PLAT-006/spec.yaml --refingerprint && $PY -m ebr normalize --spec research/experiments/EXP-PLAT-006/spec.yaml && $PY -m ebr report --spec research/experiments/EXP-PLAT-006/spec.yaml'`
- **ebr (Windows host, PowerShell 7):** `$env:PYTHONPATH='research/tools'; $env:EB_DATA_ROOT='D:/eb-research'; $env:TEMP='D:/eb-research/tmp'; $env:TMP=$env:TEMP; C:/Python313/python.exe -m ebr run --spec research/experiments/EXP-PLAT-006/spec-windows.yaml` followed by `verify-raw`, `normalize`, `report` with the same spec (for corpus items the Windows runs set `EB_DATA_ROOT=\\wsl.localhost\Ubuntu\root\eb-research` and copy items to D: inside the command).
- **Scripts:** `scripts/timestamp_probe.py --target <t> --ladder-from <item_id> --scratch <dir>`: set each value with `os.utime(ns=...)` (and `touch -d @s.ns` as a second API), read back with `os.stat`/statx, classify {exact, truncated to g, rounded, clamped-silently, error}; record `dmesg` deltas; then pack the ladder tree with `ebound pack`, read values and precision tags via `ebr-inspect-meta`, unpack onto every target and observe again. `scripts/timestamp_probe.ps1 -Mode dotnet|std` for NTFS. `scripts/precision_models.py` scores each DEC-PLT-051 candidate registry and the DEC-PLT-003 inference candidate against measured granularities.

## Seeds, warmups and replication

- **PCP-3** of `research/experiments/_index/platform-common-provisions.md` applies verbatim.

## Measurement method and metrics

- **Granularity:** for each target, the smallest g such that every written nanosecond value reads back as ⌊v/g⌋·g (or the rounding rule observed).
- **Range:** minimum and maximum exactly representable seconds; behaviour outside (error code, silent clamp value, wrap).
- **Capture:** recorded value and precision class versus source observation (exact = value equal and precision class equal to the measured granularity, or to the candidate's rule).
- **Restore:** destination observation versus archived value; a destination value that differs without an ExtractionReport entry is an undeclared difference.
- **btime/creation:** readable (statx `STATX_BTIME` mask, `%W`), settable (NTFS .NET/std; Linux none).
- **Legacy extras:** per fixture, which times are captured, declared unavailable, or silently dropped by `ebound convert --strict`.

| Metric | OD | Role here | ebr family | Unit | Direction | Threshold / oracle reference | Source |
|---|---|---|---|---|---|---|---|
| `capture_fidelity_fraction` | OD-03 | primary per (target, timestamp class) over the ladder | other | fraction | maximize | T-20 (`metric_thresholds.capture_fidelity_fraction`; A.2 role `banded`) | probe + `ebr-inspect-meta` |
| `restore_fidelity_fraction` | OD-03 | primary per (source, target, timestamp class) | other | fraction | maximize | T-20 (`metric_thresholds.restore_fidelity_fraction`; A.2 role `banded`) | probe |
| `hc07_mvt07a_undeclared_differences` | OD-03 | HC oracle output (objective §2.0: binary; any non-zero value is a committed violation artifact and an R1 matter; not an Appendix A.2 metric, never enters §6 utility) | correctness | count | equal_zero | HC-07 MVT-07(a) | probe (silent clamps without report) |
| `platform_coverage_count` | OD-18 | descriptive | other | count | report | - (`metric_thresholds.platform_coverage_count`; A.2 role `descriptive`) | analysis |

## Normalization

Values are compared as (seconds, nanoseconds) integers; precision classes by identifier. Reference: status quo.

## Statistical analysis

- **PCP-4** of `research/experiments/_index/platform-common-provisions.md` applies verbatim.

## Practical-significance thresholds

Referenced, never restated: each banded metric uses its `research/methods/thresholds.json` `metric_thresholds.<name>` entry (decision-method §3.2 ID in the metrics table; §3.1 semantics; §7.3 minimum-gain caps). Binary metrics and HC oracle outputs have no band (§3.3; objective §2.0 rule 1).

## Sensitivity analysis

- **PCP-5** of `research/experiments/_index/platform-common-provisions.md` applies verbatim.
- **Ladder arm:** leave-one-decade-out over the value ladder.

## Held-out evaluation (Phase D placeholder)

**PCP-7**.

## Cost accounting inputs (decision-method §7)

**PCP-8**.

## Expected negative results worth recording

- Silent clamping on ext4 with 128-byte inodes (smoke) is a negative result for 'report out-of-range' candidates that trust syscall errors.
- vfat local-time interpretation makes the same stored instant differ by the host time zone: a negative result for any rule that treats FAT/DOS times as UTC without a declaration.
- Linux has no API to set btime; `restore-creation-time-and-birthtime` is Windows-only here.

## Threats to validity

- Kernel behaviour for out-of-range values depends on the kernel version; results are scoped to 5.15.167.4.
- ntfs-3g exposes NTFS times through FUSE with its own conversion; native NTFS runs on D:.
- exFAT is only reachable through FUSE here; native exFAT needs elevation (EXP-PLAT-023).

## Estimated cost and effort

- **Compute:** about 3 machine-hours; **disk:** about 3 GB peak (transient scratch unless stated).
- **Timing required:** no.
- **Tooling to build:** research/experiments/EXP-PLAT-006/scripts/{timestamp_probe.py,timestamp_probe.ps1,legacy_time_fixtures.py,precision_models.py} (mechanical); optional apt exfat-fuse for the exFAT arm; Windows std `FileTimes` probe from EXP-PLAT-016
- **Agent effort:** execution none (scripted); tooling scripted; judgment only for classifying silent clamping versus documented behaviour.

## Feasibility smoke (NON-DECISION-GRADE)

See EXP-PLAT-001: the design smoke observed a far-future mtime set on a 128-byte-inode ext4 image read back silently clamped (qualitative feasibility note used only to justify the out-of-range arm; not a result and not cited as a number).

## Deviations (append-only; UTC time, author, reason, results-visible yes/no)

_None._

## Outcome (after analysis)

- classification (`outcome.json`, §10.1), verdict table reference, confirmation-bias triggers evaluated (§10.2): _pending_
