# EXP-PLAT-016: Safe-Rust platform API feasibility: capability inventory, crate survey and exactness/no-follow prototypes

> **Design pre-registration draft** (Phase C-design, platform domain). Index status: **NEEDS_TOOLING**. Structure: decision-method §12.2 record fields plus the Phase C-design protocol fields. Shared provisions PCP-1…PCP-8 live in `research/experiments/_index/platform-common-provisions.md` and are part of this pre-registration. This file becomes the §12 `record.md` only after the gates are met.

## Pre-registration

- **decision_ids:** DEC-PLT-040, DEC-PLT-030, DEC-PLT-057, DEC-PLT-017, DEC-PLT-029, DEC-PLT-042, DEC-CRY-108, DEC-PLT-033, DEC-PLT-049, DEC-PLT-055, DEC-PLT-003, DEC-PLT-014, DEC-PLT-016, DEC-PLT-031, DEC-PLT-044, DEC-PLT-045, DEC-PLT-037, DEC-INT-003, DEC-ACC-026, DEC-CMP-028
- **Program sections:** 15, 17, 19, 20, 21
- **Decision type (proposed, not assigned):** EMPIRICAL (exactness and no-follow behaviour of each API on real objects; census counts) with EXTERNAL components: security review of any unsafe boundary and licence interpretation beyond SPDX lists (§7.5, §8.3). Assignment is made by a session not involved in the candidates (decision-method §1.2).
- **od_ids / hc_ids (proposed for the schema-v2 assignment, §14 item 12):** OD-22 (M22.1–M22.4 cost inputs), OD-24 (M24.1, M24.3), OD-03 (feasibility of capture/restore classes); HC-05 (no-follow, handle-relative), HC-07 (no lossy substitute), F-23 (`unsafe_code = forbid` in workspace crates).
- **Method:** `research/decision-method.md` and `research/methods/thresholds.json` at pre-registration commit `14b977c` (status `pre-registered`).
- **Pre-registration commit:** the checkpoint commit adding this file (design draft); the §12 record commit follows G-A; every run's raw `meta` git SHA must descend from it with `dirty = false` (§5.8 d).
- **Author, independence and exposure:** Phase C-design platform-domain designer (workflow `wf_47347bc4-611`); no candidate was constructed by this session (ledger R0 sets reproduced mechanically). Held-out identity exposure (event L7) is disclosed in **PCP-1**.
- **Public data sources used for tuning (§5.4 consequence 2):** upstream sources in `research/corpus/manifest.json` `provenance` for the tuning/validation items named below, plus the pinned tool downloads named under Commands.
- **Gates:** G-A to G-D as in **PCP-2**.

## Question and hypotheses

- **Question:** For each platform capability that platform decisions need, does an API exist that is exact, handle-relative/no-follow where required, and callable from safe Rust (std, cap-std 4.0.3, rustix 1.1.4, xattr 1.6.1, or an audited third-party crate), and where only unsafe FFI exists, what are the unsafe surface, dependency, licence and maintenance costs of an audited boundary?
- **H1:** Safe exact APIs exist for Linux statx btime, SEEK_DATA/HOLE, renameat2 NOREPLACE, openat2, O_TMPFILE+linkat, FICLONE and fd-based xattrs, and on Windows for creation-time set (std FileTimes) and read-only attribute set; no audited safe crate exposes exact self-relative security-descriptor get/set, exact reparse get/set with FILE_OPEN_REPARSE_POINT, ADS enumeration, allocated-range queries or file-ID capture without unsafe inside a dependency that fails at least one §7.2 maintenance criterion; no-follow symlink metadata on Linux (lchmod-equivalent) is impossible by kernel design, so decline-and-report is the only exact option for that row.
- **H0 / equivalence:** Every capability row has a safe, exact, no-follow API in a maintained crate.
- **Falsification of H1:** A maintained crate exposing exact descriptor and reparse get/set through safe APIs that pass the exactness and no-follow tests, or a Linux API that changes symlink mode without following.

## Decisions informed

| Decision | Release relevance | Ledger question (abridged; full text in the ledger) | What this experiment contributes | Evidence still needed elsewhere |
|---|---|---|---|---|
| DEC-PLT-040 | V1_IMPORTANT | Under workspace unsafe_code=forbid, how does Entrybound obtain Windows security descriptors, ADS enumeration, macOS ACLs, birthtime restoration, resource forks and no-... | Per-capability API inventory (crate, unsafe surface, maintenance, licence) and prototype results for every candidate route (safe wrappers, isolated FFI crate, helper processes, std-only subset). | Admission policy (ecosystem domain); security review of any unsafe boundary (external); macOS rows (EXP-PLAT-022). |
| DEC-PLT-030 | V1_BLOCKING | Can exact opaque reparse data be captured (FSCTL_GET_REPARSE_POINT with FILE_OPEN_REPARSE_POINT) and restored (FSCTL_SET_REPARSE_POINT on an empty object) without foll... | Crate survey naming each wrapper examined; safe-versus-FFI prototypes of FSCTL_GET/SET_REPARSE_POINT on junction, AF_UNIX and LX symlink objects with exact-byte and no-follow checks. | Symlink/cloud tags need privilege (EXP-PLAT-023); threat model per tag (analytical). |
| DEC-PLT-057 | V1_IMPORTANT | Is Windows security descriptor (owner, group, DACL, SACL) capture and restoration required for v1, and if so through which path under unsafe_code=forbid? | Exact self-relative descriptor read (and restore where non-admin permits on own files) through each route; SDDL/lossy views excluded. | SACL (EXP-PLAT-023); security review (external). |
| DEC-PLT-017 | V1_BLOCKING | Which handle-relative, no-follow operations must capture and extraction use for directory, file, symlink and special-file creation and metadata (mode, owner, xattrs, t... | API coverage table per platform and crate for handle-relative, no-follow metadata operations; behaviour without /proc; cap-primitives 4.0.3 source audit of Linux fallback and macOS paths. | Race tests (EXP-PLAT-014); macOS (EXP-PLAT-022). |
| DEC-PLT-029 | V1_IMPORTANT | Should the NFS4 ACL dialect remain frozen without any producing or consuming implementation, and which non-Linux ACL capture targets (macOS extended ACLs, FreeBSD/ZFS... | Safe-Rust ACL crate survey (exacl, posix-acl and alternatives) with unsafe audit. | NFS4 producers (EXP-PLAT-011); macOS (EXP-PLAT-022). |
| DEC-PLT-042 | V1_IMPORTANT | Must the CLI restrict the ACLs of generated secret key and identity files on Windows (and check them on load), and through which safe API? | Safe options to set an owner-only protected DACL and to inspect DACLs. | Default ACL census (EXP-PLAT-013/020). |
| DEC-CRY-108 | V1_IMPORTANT | When the CLI loads an X-Wing identity or Ed25519 signing-key file readable beyond its owner, must it refuse (frozen crypto-suite-v1 behaviour), warn (current code) or... | Safe-Rust feasibility of Windows owner-only DACL inspection under `unsafe_code = forbid`. | Incumbent survey and workflow census (EXP-PLAT-020); crypto-doc reconciliation (crypto domain). |
| DEC-PLT-033 | V1_IMPORTANT | How should capture treat a multiply-linked file whose other aliases lie outside the packed root (silent ordinary File, entry-scoped declared loss, or refusal), and sho... | Safe Windows file-ID API availability (std unstable `windows_by_handle`, same-file, file-id, cap-std). | Capture behaviour (EXP-PLAT-009/013). |
| DEC-PLT-049 | V1_IMPORTANT | Which timestamps beyond mtime are captured, declared or restored in v1 (atime, ctime, Linux statx btime capture-only, Windows creation-time restore, macOS birthtime re... | Safe restore APIs for creation time (std FileTimes::set_created) and verification that Linux has no btime-setting API as of the pinned kernel/headers. | macOS setattrlist (EXP-PLAT-022). |
| DEC-PLT-055 | V1_IMPORTANT | Which accepted windows.file-attributes bits and windows.creation-time are restored under platform-metadata Restore, ignored or reported, including ENCRYPTED/COMPRESSED... | Safe routes to set each Windows attribute bit (std readonly only; `SetFileInformationByHandle` via crates) and their exactness. | Per-bit behaviour (EXP-PLAT-013). |
| DEC-PLT-003 | V1_IMPORTANT | Should capture detect and record source filesystem type and per-filesystem timestamp granularity and range (FidelityReport.filesystem, per-filesystem precision tags),... | Safe filesystem-type detection (rustix fstatfs, GetVolumeInformationByHandleW wrappers). | Accuracy (EXP-PLAT-001). |
| DEC-PLT-014 | V1_IMPORTANT | How does extraction learn target filesystem properties needed for planning (per-directory case sensitivity, normalization behaviour, xattr size limits, symlink and har... | Availability of non-writing capability queries in safe Rust per OS. | Predictor accuracy (EXP-PLAT-001). |
| DEC-PLT-016 | V1_IMPORTANT | Which file flags are captured and restored: a restorable subset of macOS UF/SF flags, exclusion of kernel-managed bits (SF_DATALESS, UF_COMPRESSED, SF_FIRMLINK), and a... | FS_IOC_GETFLAGS/SETFLAGS and statx attributes through rustix on WSL ext4/XFS/btrfs. | Declarations and ordering (EXP-PLAT-004/008); macOS chflags (EXP-PLAT-022). |
| DEC-PLT-031 | V1_IMPORTANT | How does Entrybound handle origin/provenance markers: preserve raw member markers (Zone.Identifier, com.apple.quarantine/provenance) only, define a canonical cross-pla... | Safe ADS write/read (`name:Zone.Identifier` opens through std and cap-std) and exactness. | Propagation behaviour (EXP-PLAT-012). |
| DEC-PLT-044 | POST_V1 | Which platform snapshot mechanisms (VSS, APFS, btrfs, ZFS, LVM) does pack --snapshot support in v1, and how is snapshot-consistent versus best-effort capture recorded? | Snapshot API feasibility: btrfs subvolume snapshot ioctl (root, loop fs) safe or FFI; VSS requires elevation (EXP-PLAT-023); APFS (EXP-PLAT-022). | Torn-capture frequency (EXP-PLAT-018). |
| DEC-PLT-045 | V1_IMPORTANT | For v1 sparse handling: is the 1,000,000-extent bound justified, should dense files carry a sparse map, should non-Linux capture (macOS SEEK_HOLE, Windows allocated-ra... | Safe sparse-layout APIs on Windows (FSCTL_QUERY_ALLOCATED_RANGES) and Linux. | Accuracy (EXP-PLAT-010); macOS SEEK_HOLE (EXP-PLAT-022). |
| DEC-PLT-037 | V1_IMPORTANT | What commit strategy applies where hard links are unavailable (FAT32/exFAT, some SMB/NFS shares, cloud-sync folders): refuse with a clear reason, no-replace rename, ex... | Safe no-replace rename on Linux (renameat2) and Windows (MoveFileExW without REPLACE_EXISTING / FileRenameInfoEx), hardlink creation, directory fsync. | Per-target behaviour and crash consistency (EXP-PLAT-017). |
| DEC-INT-003 | V1_IMPORTANT | Should every CLI output (pack INDEXED, export, publish, repack, sidecar, in-place mutations) use one crash-safe transaction pattern (temp+sync+link or rename-over, pla... | Platform API feasibility of atomic replace in safe Rust (ReplaceFileW, renameat2 RENAME_EXCHANGE, POSIX-semantics rename). | Crash injection (EXP-PLAT-017; integrity domain). |
| DEC-ACC-026 | POST_V1 | What milestone counts as "random-access model proven", and when, by whom and under what correctness/security obligations does an ecosystem mount (FUSE/WinFsp/macFUSE)... | Mount API feasibility: FUSE (fuser crate, unsafe inside) in WSL; WinFsp and macFUSE require driver installation (blocked). | Proof milestone definition and mount latency versus EROFS/squashfs/DwarFS (access domain). |
| DEC-CMP-028 | V1_IMPORTANT | How should pack handle sources that change during capture: retry count (default 2), stability predicate (Unix full stat set versus Windows length and mtime only), and... | Snapshot API availability without unsafe code or admin rights (btrfs, VSS, APFS, ZFS). | Fault injection and retry behaviour (EXP-PLAT-018). |

## Candidates

Candidate sets are the ledger's R0 sets; completeness (status quo, SPEC design, named alternatives, strongest incumbent, defer, critic candidate, corrective candidate) is checked by the R0 completeness critic, not by this design. Where a set lacks a strongest-incumbent reference, the incumbent systems measured below act as REF-INC for the metric (objective §3.0).

- **Reference candidate:** Status quo production dependency set (std, cap-std 4.0.3, rustix 1.1.4, xattr 1.6.1; no Windows descriptor/reparse/ADS capture).
- **W and S:** determined on tuning (R3/R4) and declared in the §12 record before any validation look (§4.12).

Descriptions of every candidate are in the ledger row (`candidate_set`); they are not repeated here.

- **DEC-PLT-040** — candidates: `status-quo-declare-unavailable`, `third-party-safe-wrappers`, `workspace-isolated-ffi-crate`, `helper-process-os-tools`, `upstream-std-cap-std-extensions`, `std-only-subset` (status quo: `status-quo-declare-unavailable`). Treatment: Each route is prototyped as a capability row (safe crate, FFI, helper process, std-only) and scored for exactness, no-follow behaviour, unsafe surface, licence and maintenance.
- **DEC-PLT-030** — candidates: `refuse-status-quo`, `windows-rs-safe-wrappers`, `audited-unsafe-deviceiocontrol`, `symlink-junction-only-via-std`, `capture-only-no-restore`, `external-helper-process` (status quo: `refuse-status-quo`). Treatment: Each route is prototyped as a capability row (safe crate, FFI, helper process, std-only) and scored for exactness, no-follow behaviour, unsafe surface, licence and maintenance.
- **DEC-PLT-057** — candidates: `status-quo-declared-unavailable`, `audited-safe-wrapper-crate`, `isolated-ffi-crate`, `backup-read-stream`, `helper-process` (status quo: `status-quo-declared-unavailable`). Treatment: Each route is prototyped as a capability row (safe crate, FFI, helper process, std-only) and scored for exactness, no-follow behaviour, unsafe surface, licence and maintenance.
  - excluded before execution (ledger): `sddl-text-capture` — SDDL does not round-trip conditional ACEs and resource attributes; binary self-relative form is frozen. (violates SPEC §10.6 L1290; docs/security-metadata-v1.md type 42); L0/R1(b) counterexample and reviewer sign-off still required (R1).
  - excluded before execution (ledger): `lossy-acl-view` — Repo explicitly forbids substituting a lossy ACL view for unavailable descriptor capture. (violates docs/platform-fidelity-v1.md L27-28); L0/R1(b) counterexample and reviewer sign-off still required (R1).
- **DEC-PLT-017** — candidates: `status-quo-mixed`, `rustix-at-nofollow`, `o-path-proc-self-fd-bridge`, `cap-std-ext-apis`, `audited-unsafe-ffi-boundary`, `decline-and-report-only`, `status-quo`, `handle-based-capture`, `kernel-resolve-beneath-macos`, `landlock-sandboxed-extraction`, `declare-concurrent-mutation-out-of-scope`, `private-mount-destination` (status quo: `status-quo-mixed`, `status-quo`). Treatment: Each route is prototyped as a capability row (safe crate, FFI, helper process, std-only) and scored for exactness, no-follow behaviour, unsafe surface, licence and maintenance.
  - excluded before execution (ledger): `path-based-with-checks` — SPEC §11.6 Pattern 2 forbids setting metadata by path; silently doing so is a conformance violation (violates SPEC §11.6 L1424-1426 frozen extraction pattern); L0/R1(b) counterexample and reviewer sign-off still required (R1).
- **DEC-PLT-029** — candidates: `status-quo-frozen-unvalidated`, `validate-via-linux-nfs-client`, `macos-acl-adapter-v1`, `separate-macos-acl-dialect`, `defer-nfs4-post-v1`, `freebsd-zfs-adapter` (status quo: `status-quo-frozen-unvalidated`). Treatment: Each route is prototyped as a capability row (safe crate, FFI, helper process, std-only) and scored for exactness, no-follow behaviour, unsafe surface, licence and maintenance.
- **DEC-PLT-042** — candidates: `status-quo-no-op-on-windows`, `owner-only-dacl-safe-wrapper`, `warn-on-inherited-access`, `helper-process-icacls`, `document-risk-only` (status quo: `status-quo-no-op-on-windows`). Treatment: Each route is prototyped as a capability row (safe crate, FFI, helper process, std-only) and scored for exactness, no-follow behaviour, unsafe surface, licence and maintenance.
- **DEC-CRY-108** — candidates: `status-quo-warn-unix-only`, `refuse-unix-default-with-override`, `refuse-without-override`, `warn-all-platforms-with-dacl-check`, `policy-knob-default-warn`, `no-check-caller-responsibility` (status quo: `status-quo-warn-unix-only`). Treatment: Each route is prototyped as a capability row (safe crate, FFI, helper process, std-only) and scored for exactness, no-follow behaviour, unsafe surface, licence and maintenance.
- **DEC-PLT-033** — candidates: `status-quo-silent`, `declare-partial-link-loss`, `refuse-pack-policy`, `windows-file-id-detection`, `windows-links-declared-unavailable` (status quo: `status-quo-silent`). Treatment: Each route is prototyped as a capability row (safe crate, FFI, helper process, std-only) and scored for exactness, no-follow behaviour, unsafe surface, licence and maintenance.
- **DEC-PLT-049** — candidates: `status-quo-mtime-plus-platform-birth`, `declare-unavailable-only`, `add-linux-btime-capture-only`, `add-atime-optional`, `add-ctime-capture-only`, `exclude-creation-time-by-default`, `restore-creation-time-and-birthtime` (status quo: `status-quo-mtime-plus-platform-birth`). Treatment: Each route is prototyped as a capability row (safe crate, FFI, helper process, std-only) and scored for exactness, no-follow behaviour, unsafe surface, licence and maintenance.
- **DEC-PLT-055** — candidates: `status-quo-report-unavailable`, `restore-basic-bits-and-creation-time`, `restore-compression-via-fsctl`, `never-restore-cloud-bits`, `efs-raw-apis-post-v1` (status quo: `status-quo-report-unavailable`). Treatment: Each route is prototyped as a capability row (safe crate, FFI, helper process, std-only) and scored for exactness, no-follow behaviour, unsafe surface, licence and maintenance.
- **DEC-PLT-003** — candidates: `status-quo-os-derived-precision`, `detect-fs-type-and-map`, `record-fs-type-only`, `unknown-precision-when-undetected`, `observed-granularity-inference` (status quo: `status-quo-os-derived-precision`). Treatment: Each route is prototyped as a capability row (safe crate, FFI, helper process, std-only) and scored for exactness, no-follow behaviour, unsafe surface, licence and maintenance.
- **DEC-PLT-014** — candidates: `learn-at-write-status-quo`, `probe-destination-scratch-objects`, `query-filesystem-apis`, `static-per-os-assumptions`, `caller-declared-target-profile` (status quo: `learn-at-write-status-quo`). Treatment: Each route is prototyped as a capability row (safe crate, FFI, helper process, std-only) and scored for exactness, no-follow behaviour, unsafe surface, licence and maintenance.
- **DEC-PLT-016** — candidates: `status-quo`, `macos-user-flag-subset-restore`, `declare-linux-flags-unavailable`, `add-linux-flags-name`, `immutable-last-privileged-opt-in` (status quo: `status-quo`). Treatment: Each route is prototyped as a capability row (safe crate, FFI, helper process, std-only) and scored for exactness, no-follow behaviour, unsafe surface, licence and maintenance.
- **DEC-PLT-031** — candidates: `status-quo-none`, `preserve-raw-only`, `propagate-archive-marker-default`, `canonical-origin-metadata`, `caller-policy-flag`, `propagate-when-source-marked` (status quo: `status-quo-none`). Treatment: Each route is prototyped as a capability row (safe crate, FFI, helper process, std-only) and scored for exactness, no-follow behaviour, unsafe surface, licence and maintenance.
- **DEC-PLT-044** — candidates: `status-quo-best-effort`, `btrfs-zfs-helpers`, `vss-helper`, `record-best-effort-flag-only`, `defer-post-v1` (status quo: `status-quo-best-effort`). Treatment: Each route is prototyped as a capability row (safe crate, FFI, helper process, std-only) and scored for exactness, no-follow behaviour, unsafe surface, licence and maintenance.
- **DEC-PLT-045** — candidates: `status-quo`, `omit-map-for-dense-files`, `lower-or-budgeted-extent-bound`, `add-windows-allocated-ranges`, `add-macos-seek-hole`, `restore-sparse-by-default`, `drop-sparse-v1` (status quo: `status-quo`). Treatment: Each route is prototyped as a capability row (safe crate, FFI, helper process, std-only) and scored for exactness, no-follow behaviour, unsafe surface, licence and maintenance.
- **DEC-PLT-037** — candidates: `status-quo-hardlink-only`, `noreplace-rename-fallback`, `exclusive-create-direct-write`, `refuse-with-reason`, `add-directory-fsync` (status quo: `status-quo-hardlink-only`). Treatment: Each route is prototyped as a capability row (safe crate, FFI, helper process, std-only) and scored for exactness, no-follow behaviour, unsafe surface, licence and maintenance.
  - excluded before execution (ledger): `rename-replace` — Replaces an existing destination, violating the no-overwrite publication rule. (violates docs/repack-v1.md L34-36); L0/R1(b) counterexample and reviewer sign-off still required (R1).
- **DEC-INT-003** — candidates: `status-quo-mixed`, `temp-sync-hardlink-everywhere`, `temp-rename-over`, `platform-atomic-replace`, `leftover-cleanup-on-start`, `journal-file-recovery` (status quo: `status-quo-mixed`). Treatment: Each route is prototyped as a capability row (safe crate, FFI, helper process, std-only) and scored for exactness, no-follow behaviour, unsafe surface, licence and maintenance.
- **DEC-ACC-026** — candidates: `status-quo-deferred`, `reference-fuse-prototype`, `third-party-only`, `never` (status quo: `status-quo-deferred`). Treatment: Each route is prototyped as a capability row (safe crate, FFI, helper process, std-only) and scored for exactness, no-follow behaviour, unsafe surface, licence and maintenance.
- **DEC-CMP-028** — candidates: `status-quo-abort`, `exclude-with-report`, `include-with-record`, `snapshot-integration`, `stronger-predicate` (status quo: `status-quo-abort`). Treatment: Each route is prototyped as a capability row (safe crate, FFI, helper process, std-only) and scored for exactness, no-follow behaviour, unsafe surface, licence and maintenance.
  - excluded before execution (ledger): `silent-include` — Unstable content included silently (violates SPEC §5.7a L621-625); L0/R1(b) counterexample and reviewer sign-off still required (R1).

## Corpus, fixtures and case sets

- **Capability rows** (`capabilities.yaml`, committed with the record): Windows — exact self-relative descriptor get/set by handle; owner-only DACL check/set; BackupRead/BackupWrite streams; ADS enumerate/read/write; reparse get/set with FILE_OPEN_REPARSE_POINT; attribute set by handle; creation-time set; file ID; allocated ranges; sparse flag; NTFS compression; ReplaceFileW; MoveFileExW without replace; POSIX-semantics rename; NtCreateFile-relative opens; per-directory case-sensitivity query; volume information by handle. Linux — openat2 flags; fchmodat/fchownat/utimensat with AT_SYMLINK_NOFOLLOW; fd-based xattrs; O_PATH plus /proc/self/fd and AT_EMPTY_PATH; statx btime/attributes; FS_IOC_GETFLAGS/SETFLAGS; SEEK_DATA/HOLE; renameat2 NOREPLACE/EXCHANGE; linkat AT_EMPTY_PATH with O_TMPFILE; directory fsync; FICLONE; btrfs snapshot ioctl; fstatfs; pathconf; mount_setattr idmap; Landlock; seccomp; FUSE. macOS rows are listed and marked BLOCKED.
- **Test objects:** fixtures from EXP-PLAT-004 and EXP-PLAT-013 on D: and WSL filesystems; swap objects for no-follow checks.
- **Crate list:** std, cap-std, cap-primitives, cap-fs-ext, rustix, xattr, nix, libc, windows, windows-sys, filetime, same-file, file-id, junction, uds_windows, exacl, posix-acl, landlock, fuser, reflink-copy, plus any crate the survey's crates.io keyword queries surface (queries committed before the survey runs).
- **Splits:** tuning and validation only; validation looks are registered in `research/decisions/validation-looks.jsonl` (§5.2, at most two per decision). Generated fixtures are assigned to splits by seed as stated above. No spec selects held-out (Phase D placeholder below).

## Environment and platform requirements

- **env_ids:** `wsl-ubuntu`, `windows-host`. Builds with the pinned toolchain 1.98.1; downloads go to D: or WSL ext4 only. No timing.

| Requirement | Needed? |
|---|---|
| Linux (WSL2, root) | yes |
| Windows 11 host (non-admin) | yes |
| macOS | rows BLOCKED (EXP-PLAT-022) |
| x86-64 | yes |
| ARM64 | no |
| Network | crates.io and advisory-db downloads (approved) |
| Privileged | WSL root for loop/btrfs rows; Windows non-admin |
| Large storage | no |
| External review | yes for any unsafe boundary admitted as a candidate (security review) and non-SPDX licence questions |

- **Storage discipline:** **PCP-6**.

## Commands

- **Pre-flight:** `C:/Python313/python.exe research/orchestration/disk_guard.py` must pass; no other workflow may hold the loop devices or mount points named here.
- **Build safe probe:** `research/harness/build.sh build --release -p ebr-platform-probe` (WSL) and `research/harness/build.ps1 build --release -p ebr-platform-probe` with `CARGO_TARGET_DIR=D:/eb-research/target/harness-win` (Windows).
- **Build FFI probe:** `cd research/prototypes/platform-probe-ffi && CARGO_HOME=D:/eb-research/cargo-home CARGO_TARGET_DIR=D:/eb-research/target/prototypes-ffi cargo +1.98.1 build --release` (and the WSL equivalent under `/root/eb-research/target/prototypes-ffi`).
- **ebr (WSL):** `wsl.exe -d Ubuntu -u root -- bash -lc 'cd /mnt/d/Projects/entrybound/entrybound && export PYTHONPATH=research/tools && PY=/root/eb-research/venv/bin/python && $PY -m ebr validate --spec research/experiments/EXP-PLAT-016/spec.yaml && $PY -m ebr run --spec research/experiments/EXP-PLAT-016/spec.yaml && $PY -m ebr verify-raw --spec research/experiments/EXP-PLAT-016/spec.yaml --refingerprint && $PY -m ebr normalize --spec research/experiments/EXP-PLAT-016/spec.yaml && $PY -m ebr report --spec research/experiments/EXP-PLAT-016/spec.yaml'`
- **ebr (Windows host, PowerShell 7):** `$env:PYTHONPATH='research/tools'; $env:EB_DATA_ROOT='D:/eb-research'; $env:TEMP='D:/eb-research/tmp'; $env:TMP=$env:TEMP; C:/Python313/python.exe -m ebr run --spec research/experiments/EXP-PLAT-016/spec-windows.yaml` followed by `verify-raw`, `normalize`, `report` with the same spec (for corpus items the Windows runs set `EB_DATA_ROOT=\\wsl.localhost\Ubuntu\root\eb-research` and copy items to D: inside the command).
- **Rows:** `scripts/api_rows.sh --route <safe|ffi> --scratch <dir>` and `scripts/api_rows.ps1 -Route <safe|ffi> -Scratch <dir>` run every row: exactness check against the independent observer (for example descriptor bytes versus `RawSecurityDescriptor.GetBinaryForm`, reparse payload versus `fsutil` hex, xattr bytes versus `getfattr`), no-follow check (target replaced by a symlink or junction immediately before the call), and error classification.
- **Survey:** `scripts/crate_survey.py --crates capabilities.yaml --advisory-db <snapshot> --out research/normalized/EXP-PLAT-016/decision/crate-survey.csv` (licence SPDX, last release, owners count, RUSTSEC advisories, MSRV, `unsafe` counts from `scripts/unsafe_census.py` over vendored sources including transitive dependencies from `cargo tree -e normal --target all`).

## Seeds, warmups and replication

- **PCP-3** of `research/experiments/_index/platform-common-provisions.md` applies verbatim.

## Measurement method and metrics

- **Per row and route:** available (API exists), exact (bytes/values equal to the observer), no-follow (operation acted on the link object or refused, never on its target), safe (no `unsafe` in the calling crate; unsafe count in dependencies recorded), privilege needed.
- **Per crate:** cost inputs of §7.1 C3/C4.

| Metric | OD | Role here | ebr family | Unit | Direction | Threshold / oracle reference | Source |
|---|---|---|---|---|---|---|---|
| `decode_path_native_unsafe_count` | OD-22 | cost_input (M22.2) | other | count | minimize | C3/C4 (`metric_thresholds.decode_path_native_unsafe_count`; A.2 role `cost_input`) | `unsafe_census.py` |
| `decode_path_dependency_count` | OD-22 | cost_input (M22.1) | other | count | minimize | C3 (`metric_thresholds.decode_path_dependency_count`; A.2 role `cost_input`) | `cargo tree` |
| `rustsec_advisory_count` | OD-22 | cost_input (M22.3) | other | count | minimize | C3 (`metric_thresholds.rustsec_advisory_count`; A.2 role `cost_input`) | cargo-audit snapshot |
| `maintainer_count` | OD-22 | cost_input (M22.4) | other | count | maximize | C3 (`metric_thresholds.maintainer_count`; A.2 role `cost_input`) | crates.io snapshot |
| `msrv_gap` | OD-22 | cost_input (M22.4) | other | versions | minimize | C3 (`metric_thresholds.msrv_gap`; A.2 role `cost_input`) | crate metadata |
| `release_age_years` | OD-22 | descriptive (M22.4) | other | years | report | - (`metric_thresholds.release_age_years`; A.2 role `descriptive`) | crates.io snapshot |
| `license_compatibility` | OD-22 | binary R2 screen (§7.5) | correctness | binary | equal_one | R2 (`metric_thresholds.license_compatibility`; A.2 role `binary`) | SPDX expression check |
| `untrusted_input_loc` | OD-24 | cost_input (M24.1) for parsers of descriptor/reparse bytes added by a candidate | other | LoC | minimize | C4 (`metric_thresholds.untrusted_input_loc`; A.2 role `cost_input`) | LoC counter |
| `capture_fidelity_fraction` | OD-03 | primary for route feasibility: rows where the route reads exact values | other | fraction | maximize | T-20 (`metric_thresholds.capture_fidelity_fraction`; A.2 role `banded`) | rows |
| `restore_fidelity_fraction` | OD-03 | primary for route feasibility: rows where the route writes exact values | other | fraction | maximize | T-20 (`metric_thresholds.restore_fidelity_fraction`; A.2 role `banded`) | rows |
| `hc05_mvt05_escapes` | OD-24 | HC oracle output (objective §2.0: binary; any non-zero value is a committed violation artifact and an R1 matter; not an Appendix A.2 metric, never enters §6 utility) | correctness | count | equal_zero | HC-05 MVT-05(a)/(b) | no-follow checks (a followed link is an escape) |

## Normalization

Rows normalized to {exact-safe, exact-unsafe-dependency, exact-ffi-only, inexact, unavailable, privilege-blocked}. Reference: status quo dependency set.

## Statistical analysis

- **Screens:** a route that follows links (HC-05) or substitutes a lossy view (HC-07; docs/platform-fidelity-v1.md L27-28) is eliminated at R1; unsafe code in workspace crates is eliminated by F-23 (§7.5); copyleft production dependencies are eliminated at R2.
- **Cost:** surviving routes carry computed tiers (§7.2) with the maintenance penalty; any dependency containing unsafe or native code is at least T3.
- **Selection:** per capability row, the lowest-tier exact route; a row with no admissible route yields `declared unavailable` as the only surviving candidate (R6 exemption does not apply to rows where a lower tier exists).
- **External review:** any admitted unsafe boundary is routed to `EXTERNAL_REVIEW_REQUIRED` with a dossier (§8.3) before a decision depending on it can be `DECIDED`.

## Practical-significance thresholds

Cost-input metrics have no band (Appendix A.2 role `cost_input`; they feed §7 tiers). Fidelity fractions use T-20 (`metric_thresholds.capture_fidelity_fraction`, `metric_thresholds.restore_fidelity_fraction`). Binary screens have no band (§3.3).

## Sensitivity analysis

- Maintenance-penalty arm: tiers recomputed with each §7.2 maintenance criterion toggled; selections that change are flagged.
- Snapshot-date arm: crates.io and advisory data re-captured at freeze; changes recorded as reopen triggers (§11 item 2).

## Held-out evaluation (Phase D placeholder)

Not applicable: API feasibility is a property of platforms and crates, not of corpus content (FORMAL/EMPIRICAL-census parts skip R5 with this reason recorded). The survey is re-captured at Commit A.

## Cost accounting inputs (decision-method §7)

**PCP-8**.

## Expected negative results worth recording

- Expected: no safe exact descriptor/reparse route — a negative result that supports `declare unavailable` or an audited FFI boundary with its full cost.
- Linux cannot change symlink permissions without following; any candidate claiming symlink mode restoration is falsified by the API itself.

## Threats to validity

- Crate ecosystem changes quickly; results are dated.
- Prototype correctness depends on test objects; rows are exercised on a limited object set (EXP-PLAT-004/013 fixtures).
- Unsafe counts by token census over-approximate audit burden; they are cost inputs, not risk measures.

## Estimated cost and effort

- **Compute:** about 6 machine-hours; **disk:** about 15 GB peak (transient scratch unless stated).
- **Timing required:** no.
- **Tooling to build:** `research/harness/crates/ebr-platform-probe` (new harness crate, `unsafe_code = "forbid"`): one subcommand per capability row using only safe APIs (std 1.98.1, cap-std/cap-primitives/cap-fs-ext 4.0.3, rustix 1.1.4, xattr 1.6.1, and surveyed safe wrapper crates); JSON output per row; `research/prototypes/platform-probe-ffi/` (separate Cargo workspace, RESEARCH-ONLY, never a dependency of production or the harness, unsafe allowed and counted): Win32/NT and Linux ioctl rows that have no safe API, to size the audited-boundary candidates (C3/C4 inputs); research/experiments/EXP-PLAT-016/{capabilities.yaml,scripts/crate_survey.py,scripts/unsafe_census.py,scripts/api_rows.sh,scripts/api_rows.ps1}; pinned RustSec advisory-db snapshot and `cargo-audit`; crates.io metadata snapshot (owners, release dates) captured once with date; Windows builds use `CARGO_HOME=D:/eb-research/cargo-home` and `CARGO_TARGET_DIR=D:/eb-research/target/prototypes-ffi` so no crate download lands on C:
- **Agent effort:** tooling judgment-heavy (prototype rows and survey classification need review), execution scripted, analysis judgment; the survey classification is signed off by a session not involved in the candidates.

## Deviations (append-only; UTC time, author, reason, results-visible yes/no)

_None._

## Outcome (after analysis)

- classification (`outcome.json`, §10.1), verdict table reference, confirmation-bias triggers evaluated (§10.2): _pending_
