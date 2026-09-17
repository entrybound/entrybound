# EXP-PLAT-001: Platform capability and independent-observer census

> **Design pre-registration draft** (Phase C-design, platform domain). Index status: **READY**. Structure: decision-method §12.2 record fields plus the Phase C-design protocol fields. Shared provisions PCP-1…PCP-8 live in `research/experiments/_index/platform-common-provisions.md` and are part of this pre-registration. This file becomes the §12 `record.md` only after the gates are met.

## Pre-registration

- **decision_ids:** DEC-PLT-062, DEC-PLT-014, DEC-PLT-003, DEC-PLT-047
- **Program sections:** 15, 16, 21
- **Decision type (proposed, not assigned):** EMPIRICAL (capability availability and predictor accuracy) with a FORMAL part (the mechanical derivation of which MVT cells are HC_UNVERIFIED or PLATFORM_BLOCKED from the table). Assignment is made by a session not involved in the candidates (decision-method §1.2).
- **od_ids / hc_ids (proposed for the schema-v2 assignment, §14 item 12):** OD-18 (M18.4 `platform_coverage_count`), OD-03 (precision-tag capture exactness for DEC-PLT-003), OD-17 is not used; HC-07 and HC-08 (this census decides which MVT-07(a)/MVT-08 cells can run).
- **Method:** `research/decision-method.md` and `research/methods/thresholds.json` at pre-registration commit `14b977c` (status `pre-registered`).
- **Pre-registration commit:** the checkpoint commit adding this file (design draft); the §12 record commit follows G-A; every run's raw `meta` git SHA must descend from it with `dirty = false` (§5.8 d).
- **Author, independence and exposure:** Phase C-design platform-domain designer (workflow `wf_47347bc4-611`); no candidate was constructed by this session (ledger R0 sets reproduced mechanically). Held-out identity exposure (event L7) is disclosed in **PCP-1**.
- **Public data sources used for tuning (§5.4 consequence 2):** upstream sources in `research/corpus/manifest.json` `provenance` for the tuning/validation items named below, plus the pinned tool downloads named under Commands.
- **Gates:** G-A to G-D as in **PCP-2**.

## Question and hypotheses

- **Question:** Which metadata classes, name behaviours, kernel and OS mechanisms, and independent observer tools can actually be created, observed and restored in each locally reachable environment and privilege context (WSL root, WSL non-root, WSL user namespace, Windows non-admin), which are blocked, and how accurately do non-writing capability queries predict the write-probe ground truth?
- **H1:** Capability availability is a deterministic function of (env_id, target filesystem, privilege context): both repetitions produce identical capability vectors. The query-based predictor (`query-filesystem-apis`) agrees with write probes on every case whose capability the OS exposes through a documented query, and the static per-OS table (`static-per-os-assumptions`) disagrees on at least one case per OS (for example trailing-dot handling on vfat versus ext4 under the same Linux kernel).
- **H0 / equivalence:** The static table and the query predictor are EQUIVALENT (identical per-case predictions) against write-probe ground truth on every target.
- **Falsification of H1:** Any run-to-run disagreement of a capability vector (then capability is not a deterministic function of the context and every downstream HC_UNVERIFIED derivation must be re-examined), or zero static-table disagreements across all targets.

## Decisions informed

| Decision | Release relevance | Ledger question (abridged; full text in the ledger) | What this experiment contributes | Evidence still needed elsewhere |
|---|---|---|---|---|
| DEC-PLT-062 | V1_BLOCKING | How does stable v1 disposition capabilities that cannot be evaluated locally (macOS/APFS metadata, native ARM64 performance, ReFS, admin-only Windows operations): keep... | Mechanical inventory of which classes/mechanisms are testable here versus blocked (macOS, native ARM64, ReFS, elevated Windows, Docker), feeding the disposition table and remaining-unknowns. | Cost of hardware or contributor runs and the risk assessment of shipping untested code are analytical (owner decision); emulation validity is EXP-PLAT-005; blocked suites are EXP-PLAT-022/023/025. |
| DEC-PLT-014 | V1_IMPORTANT | How does extraction learn target filesystem properties needed for planning (per-directory case sensitivity, normalization behaviour, xattr size limits, symlink and har... | Accuracy of non-writing capability queries and static assumptions against write probes, per target; probe side effects recorded. | Planning-time cost (EXP-PLAT-015); TOCTOU exposure of in-destination probes (EXP-PLAT-014). |
| DEC-PLT-003 | V1_IMPORTANT | Should capture detect and record source filesystem type and per-filesystem timestamp granularity and range (FidelityReport.filesystem, per-filesystem precision tags),... | Feasibility and accuracy of filesystem-type detection (statfs f_type, `GetVolumeInformationByHandleW`) and mapping to measured timestamp granularity from EXP-PLAT-006. | Safe-Rust API availability (EXP-PLAT-016); AUX host-dependence impact (EXP-PLAT-005). |
| DEC-PLT-047 | V1_IMPORTANT | What minimum Linux kernel, macOS and Windows versions does v1 support for metadata capture and kernel-enforced confinement, and what fallback or report applies below t... | Presence of openat2 RESOLVE_* flags, Landlock ABI, idmapped mounts, O_TMPFILE, statx btime on this kernel and of Windows primitives on this build. | Minimum-version timeline from primary release notes (EXP-PLAT-021); fallback behaviour below thresholds (EXP-PLAT-014 arm E); macOS (EXP-PLAT-022). |

## Candidates

Candidate sets are the ledger's R0 sets; completeness (status quo, SPEC design, named alternatives, strongest incumbent, defer, critic candidate, corrective candidate) is checked by the R0 completeness critic, not by this design. Where a set lacks a strongest-incumbent reference, the incumbent systems measured below act as REF-INC for the metric (objective §3.0).

- **Reference candidate:** `learn-at-write-status-quo` (DEC-PLT-014) and `status-quo-os-derived-precision` (DEC-PLT-003); DEC-PLT-062/047 have no measured reference (inventory only).
- **W and S:** determined on tuning (R3/R4) and declared in the §12 record before any validation look (§4.12).

Descriptions of every candidate are in the ledger row (`candidate_set`); they are not repeated here.

- **DEC-PLT-062** — candidates: `status-quo-compiled-untested-plus-unavailable-declarations`, `exclude-blocked-platform-claims`, `experimental-tier-per-platform`, `declare-unavailable-until-tested`, `emulated-correctness-evidence-only`, `documentation-and-third-party-evidence`, `community-contributor-runs`, `acquire-hardware-or-admin-session` (status quo: `status-quo-compiled-untested-plus-unavailable-declarations`). Treatment: Not selected by measurement: the census supplies the inventory each disposition candidate needs; candidates are compared at synthesis (Phase E) on the blocked inventory.
  - excluded before execution (ledger): `hosted-ci-runners` — The program owner declined hosted runners; platform experiments are local and Docker only (violates PROGRESS standing decision 2026-09-12 (no GitHub Actions or hosted runners)); L0/R1(b) counterexample and reviewer sign-off still required (R1).
- **DEC-PLT-014** — candidates: `learn-at-write-status-quo`, `probe-destination-scratch-objects`, `query-filesystem-apis`, `static-per-os-assumptions`, `caller-declared-target-profile` (status quo: `learn-at-write-status-quo`). Treatment: Evaluated as a predictor of write-probe ground truth over the capability case list.
  - `learn-at-write-status-quo`: Status quo reference: ground truth equals attempt outcome; scored on whether a failure would surface only mid-extraction (planning miss).
  - `caller-declared-target-profile`: Scored with the profile table a caller would pass (`windows-ntfs`, `linux-ext4`, ...) built from primary documentation (EXP-PLAT-021), not from probes.
- **DEC-PLT-003** — candidates: `status-quo-os-derived-precision`, `detect-fs-type-and-map`, `record-fs-type-only`, `unknown-precision-when-undetected`, `observed-granularity-inference` (status quo: `status-quo-os-derived-precision`). Treatment: Evaluated as a predictor of the measured granularity (EXP-PLAT-006 ground truth) on each target.
- **DEC-PLT-047** — candidates: `status-quo-unstated`, `linux-5-6-openat2-minimum`, `linux-lts-5-10-minimum`, `macos-11-minimum`, `macos-latest-two`, `tiered-support-weaker-reported` (status quo: `status-quo-unstated`). Treatment: Mechanism presence recorded for this host only; version thresholds come from EXP-PLAT-021.

## Corpus, fixtures and case sets

- **Corpus items:** none. Each sample is triggered by an ebr generated item (`plat001-trigger`, generator `zeros-v1`, 1 byte) and builds its own target.
- **Capability case list** `research/experiments/EXP-PLAT-001/cases.json` (committed with the §12 record): for every target, the cells: symlink, hardlink, junction (NTFS), alternate data stream, xattr namespaces (user, trusted, security.selinux, security.capability, system.posix_acl_*), NFS4 ACL xattr, case-sensitive lookup, per-directory case sensitivity set/query, normalization-insensitive lookup, 8.3 short-name generation, sparse hole creation and SEEK_HOLE reporting, NTFS allocated-range query, reflink (FICLONE), O_TMPFILE, renameat2 NOREPLACE/EXCHANGE, statx btime, FS_IOC_GETFLAGS/SETFLAGS (immutable, append, nodump, noatime, casefold, compression), mknod char/block and mkfifo and AF_UNIX node (root, non-root, user namespace), chown to another uid, setuid retention after chown and after write, timestamp minimum/maximum set probes (ladder from EXP-PLAT-006), maximum component bytes/UTF-16 units, maximum path length, reserved device names, trailing dot/space, invalid UTF-8 names, unpaired-surrogate names (Windows), idmapped bind mounts, unprivileged user namespaces, openat2 with RESOLVE_BENEATH/IN_ROOT/NO_SYMLINKS/NO_XDEV, Landlock ABI version, seccomp user notification, `/proc/self/fd` presence, SeCreateSymbolicLinkPrivilege and Developer Mode (read-only registry query; never changed), SeBackup/SeRestore/SeSecurity privileges, NTFS compression (`compact`), NTFS sparse flag, EFS availability (query only, no certificate creation), reparse creation classes (junction, AF_UNIX via WSL on drvfs, LX symlink via WSL), `fsutil file queryfileid`, dm targets (`dmsetup targets`), nfsd/cifs modules, `/dev/kvm`, qemu-user binaries.
- **Linux targets:** `wsl-ext4`, `loop-ext4-i128`, `loop-xfs-bigtime`, `loop-xfs-nobigtime`, `loop-btrfs`, `tmpfs`, `loop-vfat`, `ntfs3g-windows-names`, `ntfs3g-xattr-streams`, `loop-ext4-casefold`, `drvfs-mnt-d-control`, `cifs-loopback-samba`, `nfs4-loopback-knfsd`. Privilege contexts per target: root; non-root via `setpriv --reuid=65534 --regid=65534 --clear-groups`; user namespace via `unshare --user --map-root-user`.
- **Windows targets:** `ntfs-d-win32` (normal paths under `D:\eb-research\scratch\platform\EXP-PLAT-001`), `ntfs-d-verbatim` (`\\?\` paths).
- **Predictor inputs:** `query_predictors.py` records only non-writing queries (`stat -f -c %T`, `os.pathconf`, `lsattr -d`, `findmnt -no OPTIONS`, `fsutil fsinfo volumeinfo`, `fsutil file queryCaseSensitiveInfo`); the static table is `static_tables.json` transcribed from EXP-PLAT-021 primary sources before any run.
- **Splits:** the case list is a fixed design object; there is no split-dependent content. Repetition 2 re-derives every cell from scratch.
- **Splits:** tuning and validation only; validation looks are registered in `research/decisions/validation-looks.jsonl` (§5.2, at most two per decision). Generated fixtures are assigned to splits by seed as stated above. No spec selects held-out (Phase D placeholder below).

## Environment and platform requirements

- **env_ids:** `wsl-ubuntu` and `windows-host` (fingerprints in `research/environment/`). Loop-device images are created on WSL ext4; `drvfs-mnt-d-control` is a negative control only (mounted without `metadata`).
- **Qualifiers:** none needed (no timing). FUSE (ntfs-3g) and loopback SMB/NFS are labelled as emulations of the respective native stacks in every result table.

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
- **ebr (WSL):** `wsl.exe -d Ubuntu -u root -- bash -lc 'cd /mnt/d/Projects/entrybound/entrybound && export PYTHONPATH=research/tools && PY=/root/eb-research/venv/bin/python && $PY -m ebr validate --spec research/experiments/EXP-PLAT-001/spec.yaml && $PY -m ebr run --spec research/experiments/EXP-PLAT-001/spec.yaml && $PY -m ebr verify-raw --spec research/experiments/EXP-PLAT-001/spec.yaml --refingerprint && $PY -m ebr normalize --spec research/experiments/EXP-PLAT-001/spec.yaml && $PY -m ebr report --spec research/experiments/EXP-PLAT-001/spec.yaml'`
- **ebr (Windows host, PowerShell 7):** `$env:PYTHONPATH='research/tools'; $env:EB_DATA_ROOT='D:/eb-research'; $env:TEMP='D:/eb-research/tmp'; $env:TMP=$env:TEMP; C:/Python313/python.exe -m ebr run --spec research/experiments/EXP-PLAT-001/spec-windows.yaml` followed by `verify-raw`, `normalize`, `report` with the same spec (for corpus items the Windows runs set `EB_DATA_ROOT=\\wsl.localhost\Ubuntu\root\eb-research` and copy items to D: inside the command).
- **Scripts to be written (mechanical):**
  - `research/experiments/EXP-PLAT-001/scripts/fsimg.sh {create|mount|umount|destroy} <target> <workdir>` — shared helper for all EXP-PLAT experiments: creates sparse image files under `/root/eb-research/scratch/platform/<EXP>/img/`, runs `mkfs.ext4 -F [-I 128] [-O casefold -E encoding=utf8]`, `mkfs.xfs -f -m bigtime=0|1`, `mkfs.btrfs -f`, `mkfs.vfat`, `mkntfs -F -f`, mounts with `mount -o loop` (never `losetup -D`), ntfs-3g with `windows_names` or `streams_interface=xattr`, tmpfs with `size=512m`; prints the mount point; refuses any path under a held-out root.
  - `research/experiments/EXP-PLAT-001/scripts/linux_capability_probe.sh --target <name> --scratch <dir>` — for each case in `cases.json` performs the write probe in each privilege context and prints one JSON line `{schema: ebr.platform.probe.v1, experiment_id, payload: {capability_cells_total, capability_cells_available, capability_cells_blocked, query_predictor_miss_count, static_predictor_miss_count, observer_tools_missing, platform_coverage_count, observation_sha256, cells: [...]}}`; always exits 0 unless the probe itself crashed.
  - `research/experiments/EXP-PLAT-001/scripts/query_predictors.py --target-mount <dir>` — non-writing predictions for every case.
  - `research/experiments/EXP-PLAT-001/scripts/windows_capability_probe.ps1 -Mode win32|verbatim -Scratch <dir>` — PowerShell 7, non-admin; never changes system, power or security settings; creates and removes objects only under the scratch directory.
- **Analysis:** `research/experiments/EXP-PLAT-001/scripts/capability_table.py --raw research/raw/EXP-PLAT-001 --out research/normalized/EXP-PLAT-001/decision/` writes `capability-table.csv` (target × context × case → available/blocked/unobservable), `hc-unverified-cells.csv` (derived MVT cells), and `predictor-accuracy.csv`.

## Seeds, warmups and replication

- **PCP-3** of `research/experiments/_index/platform-common-provisions.md` applies verbatim.

## Measurement method and metrics

- **Arm A (ground truth):** for each case, attempt the operation and verify it with an independent observer (`stat`, `getfattr -h -d -m - -e hex`, `getfacl -n`, `lsattr -d`, `filefrag -v`, `ls -li`; PowerShell `Get-Item -Stream *`, `Get-Acl`, `fsutil reparsepoint query`, `fsutil file queryfileid`). A capability counts as available only when the observer confirms the effect (for example a hole reported by SEEK_HOLE and by `filefrag`).
- **Arm B (predictors):** before Arm A writes anything, run each predictor candidate; a *miss* is a case the predictor declares supported that the probe shows unsupported or silently transformed (planning miss), a *false alarm* is the reverse.
- **Arm C (observer inventory):** tool presence and versions of every observer the MVT-07(a) oracle needs, recorded in the environment capture.
- **Derived output:** `hc-unverified-cells.csv` lists, for every MVT-07(a)/MVT-08/MVT-05 cell used by EXP-PLAT-002…020, whether it can run here; those experiments read this file instead of re-deciding applicability.

| Metric | OD | Role here | ebr family | Unit | Direction | Threshold / oracle reference | Source |
|---|---|---|---|---|---|---|---|
| `platform_coverage_count` | OD-18 | descriptive (M18.4) | other | count | report | - (`metric_thresholds.platform_coverage_count`; A.2 role `descriptive`) | stdout_json `payload.platform_coverage_count` |
| `capture_fidelity_fraction` | OD-03 | primary for DEC-PLT-003 only: fraction of (target, timestamp class) cases whose predicted precision tag equals the EXP-PLAT-006 measured granularity | other | fraction | maximize | T-20 (`metric_thresholds.capture_fidelity_fraction`; A.2 role `banded`) | analysis join of `predictor-accuracy.csv` with EXP-PLAT-006 raw |
| `restore_fidelity_fraction` | OD-03 | primary for DEC-PLT-014: fraction of capability cases for which the predictor's planning decision leads to the exact restore outcome (a planning miss that surfaces mid-extraction counts as not restored at plan time) | other | fraction | maximize | T-20 (`metric_thresholds.restore_fidelity_fraction`; A.2 role `banded`) | analysis over `predictor-accuracy.csv` |
| `query_predictor_miss_count` | OD-18 | secondary only (no Appendix A.2 band; decision-method §0.2 item 4 — reported, never eliminates or selects) | other | see definition | report | none | stdout_json |

## Normalization

Every cell is normalized to one of `available`, `unavailable-refused` (operation fails with an error), `unavailable-transformed` (operation succeeds but the observer shows a different effect, for example a stripped trailing dot), `unobservable` (no independent observer) and `blocked-privilege`. Predictor outputs use the same vocabulary. Reference: the status-quo predictor.

## Statistical analysis

- **PCP-4** of `research/experiments/_index/platform-common-provisions.md` applies verbatim.
- **Specific to this census:** predictor comparisons are case-list fractions per target (evaluation condition); DEC-PLT-062 and DEC-PLT-047 receive inventories, not selections.

## Practical-significance thresholds

Referenced, never restated: each banded metric uses its `research/methods/thresholds.json` `metric_thresholds.<name>` entry (decision-method §3.2 ID in the metrics table; §3.1 semantics; §7.3 minimum-gain caps). Binary metrics and HC oracle outputs have no band (§3.3; objective §2.0 rule 1).

## Sensitivity analysis

- **PCP-5** of `research/experiments/_index/platform-common-provisions.md` applies verbatim.

## Held-out evaluation (Phase D placeholder)

Not applicable: the census has no corpus content. The capability table is re-captured whenever an `env_id` changes (§4.7) and immediately before Phase D so that held-out runs use the same HC_UNVERIFIED cells.

## Cost accounting inputs (decision-method §7)

**PCP-8**.

## Expected negative results worth recording

- ext4 casefold cannot be mounted on this WSL kernel (feasibility smoke), so ext4 casefold behaviour is HC_UNVERIFIED here and must be excluded from any decision scope or obtained elsewhere.
- Windows symlink creation is unavailable to this non-admin session without Developer Mode (smoke); every NTFS symlink restoration cell is HC_UNVERIFIED (MVT-05(b) note) and routed to EXP-PLAT-023.
- `fsutil file setCaseSensitiveInfo` returned success while the subsequent query reported the attribute disabled (smoke); the census must record the observed effect, not the exit code.
- The static per-OS table is expected to mispredict at least vfat-on-Linux and ntfs-3g behaviours; that negative result bounds DEC-PLT-014's static candidate.

## Threats to validity

- WSL2 runs a Microsoft-configured 5.15 kernel; capability absence (casefold, dm targets) may be configuration-specific, not Linux-general. Results are scoped to this `env_id`.
- ntfs-3g and loopback SMB/NFS emulate, not reproduce, the Windows NTFS driver and remote servers.
- Non-admin Windows hides privilege-dependent behaviour (SACL, symlinks, 8.3 control); these cells are blocked, not negative.
- Loop images live on ext4 inside a VHDX on NTFS; allocation-related cells (holes, reflink) reflect the image filesystem, which is the intent, but host caching can affect SEEK_HOLE (see the fingerprint page-cache incident in PROGRESS); probes drop caches with `posix_fadvise(DONTNEED)` before extent queries.

## Estimated cost and effort

- **Compute:** about 1 machine-hours; **disk:** about 3 GB peak (transient scratch unless stated).
- **Timing required:** no.
- **Tooling to build:** research/experiments/EXP-PLAT-001/scripts/{fsimg.sh,linux_capability_probe.sh,query_predictors.py,windows_capability_probe.ps1} (mechanical); research/experiments/EXP-PLAT-001/cases.json capability case list (committed with the §12 record); optional arms: apt packages samba cifs-utils nfs-kernel-server nfs4-acl-tools exfat-fuse (approved downloads)
- **Agent effort:** execution none (scripted, about 1 machine-hour); tooling scripted (mechanical model); judgment only for case-list sign-off and for reading the generated capability table.

## Feasibility smoke (NON-DECISION-GRADE)

A ≤2-minute smoke on tiny images (2026-09-17, this design session) was run only to decide READY/BLOCKED labels; its outputs are not results and are not cited as numbers. Observed qualitatively: ext4, ext4 with 128-byte inodes, XFS (bigtime on/off), btrfs and vfat loop mounts work; an ext4 casefold image did not mount; an unprivileged user namespace works while `mknod` inside it is refused; idmapped bind mounts work; ntfs-3g with `windows_names` refuses `CON`; the openat2 syscall is present; `/dev/kvm` exists; `cifs`, `nfsd` and `dm_mod` appear under `/sys/module`. On D: (non-admin): ADS, junction and hardlink creation work, symlink creation does not, `compact /c` and `fsutil sparse setflag` exit 0, CPython on Windows cannot bind AF_UNIX sockets, and a verbatim-path trailing-dot file can be created. All scratch objects were removed.

## Deviations (append-only; UTC time, author, reason, results-visible yes/no)

_None._

## Outcome (after analysis)

- classification (`outcome.json`, §10.1), verdict table reference, confirmation-bias triggers evaluated (§10.2): _pending_
