# EXP-PLAT-009: Special files and hardlink topology: capture, restore feasibility, fallbacks and inode-scoped metadata

> **Design pre-registration draft** (Phase C-design, platform domain). Index status: **READY**. Structure: decision-method §12.2 record fields plus the Phase C-design protocol fields. Shared provisions PCP-1…PCP-8 live in `research/experiments/_index/platform-common-provisions.md` and are part of this pre-registration. This file becomes the §12 `record.md` only after the gates are met.

## Pre-registration

- **decision_ids:** DEC-PLT-046, DEC-MOD-021, DEC-PLT-018, DEC-PLT-019, DEC-PLT-033
- **Program sections:** 15, 18, 21
- **Decision type (proposed, not assigned):** EMPIRICAL. Assignment is made by a session not involved in the candidates (decision-method §1.2).
- **od_ids / hc_ids (proposed for the schema-v2 assignment, §14 item 12):** OD-03 (M03.1, M03.2), OD-17 is not used; HC-07, HC-17 (MVT-17(d) hardlink inference), HC-05.
- **Method:** `research/decision-method.md` and `research/methods/thresholds.json` at pre-registration commit `14b977c` (status `pre-registered`).
- **Pre-registration commit:** the checkpoint commit adding this file (design draft); the §12 record commit follows G-A; every run's raw `meta` git SHA must descend from it with `dirty = false` (§5.8 d).
- **Author, independence and exposure:** Phase C-design platform-domain designer (workflow `wf_47347bc4-611`); no candidate was constructed by this session (ledger R0 sets reproduced mechanically). Held-out identity exposure (event L7) is disclosed in **PCP-1**.
- **Public data sources used for tuning (§5.4 consequence 2):** upstream sources in `research/corpus/manifest.json` `provenance` for the tuning/validation items named below, plus the pinned tool downloads named under Commands.
- **Gates:** G-A to G-D as in **PCP-2**.

## Question and hypotheses

- **Question:** Which special-file kinds and hardlink topologies can be captured and restored on each filesystem and privilege context, what happens when hardlinks cannot be created at the destination (vfat, cross-device, cifs), which metadata is inode-scoped versus per-link on ext4, XFS, btrfs, tmpfs, ntfs-3g and NTFS, and is a lone alias of a partially packed group declared?
- **H1:** Devices restore only as root outside user namespaces, FIFOs restore everywhere except vfat and NTFS D:, sockets carry no restorable content; status quo aborts pack on any special file and aborts extraction on any failed hardlink; mode, owner, xattrs, ACLs and timestamps are inode-scoped on every Linux filesystem, while NTFS directory-entry timestamps can differ per link; the status quo captures lone aliases and Windows hardlinks as independent files without a declaration (HC-07 violation).
- **H0 / equivalence:** Every kind and topology restores (or is refused with a report) identically across filesystems, and no undeclared topology loss occurs.
- **Falsification of H1:** Lone aliases or Windows hardlink pairs produce a FidelityReport declaration under the status quo, or per-link metadata differences are absent on NTFS.

## Decisions informed

| Decision | Release relevance | Ledger question (abridged; full text in the ledger) | What this experiment contributes | Evidence still needed elsewhere |
|---|---|---|---|---|
| DEC-PLT-046 | V1_BLOCKING | How are character/block devices, FIFOs and sockets handled in v1: first-class EntryKinds (SPEC §3.3), auxiliary snapshot metadata, FidelityReport-only omission with en... | Capture/restore feasibility per kind and context; dev_t major/minor round trip through ustar, pax and GNU base-256 encodings; overlay whiteout capture on a real overlayfs mount; incumbent capture behaviour. | Prevalence (EXP-PLAT-003); extraction policy (EXP-PLAT-008); FreeBSD/macOS dev_t split (EXP-PLAT-021 primary sources); socket semantics per platform (analytical). |
| DEC-MOD-021 | V1_BLOCKING | What is the v1 EntryKind set (implemented Directory/File/Symlink/ReparsePoint vs SPEC Fifo/Socket/BlockDevice/CharDevice, whiteouts, junctions) and how are unknown kin... | Restoration feasibility of each SPEC kind per platform and context. | Security review and SPEC amendment (analytical/external). |
| DEC-PLT-018 | V1_IMPORTANT | When a hardlink alias cannot be created (FAT/exFAT, some SMB/FUSE/cloud filesystems, cross-device), should extraction abort (status quo), materialize an independent ve... | Status quo and incumbent behaviour when hardlinks cannot be created (vfat, cross-device mount inside the destination, cifs loopback); byte expansion of a copy fallback on F17/F19 trees; reflink feasibility on btrfs and XFS. | exFAT and SMB-on-Windows targets (EXP-PLAT-023). |
| DEC-PLT-019 | V1_IMPORTANT | Which metadata names are inode-scoped and must agree across hardlink aliases (mode, uid, gid, mtime, xattrs, ACLs, sparse map, executable, platform flags, Windows attr... | Per-filesystem table of which metadata changes through one alias are visible through another (inode-scoped versus per-link), including NTFS. | APFS (EXP-PLAT-022); negative vectors (conformance domain). |
| DEC-PLT-033 | V1_IMPORTANT | How should capture treat a multiply-linked file whose other aliases lie outside the packed root (silent ordinary File, entry-scoped declared loss, or refusal), and sho... | Subtree packs with lone aliases on ext4 and NTFS, Windows hardlink capture, and prevalence input from EXP-PLAT-003. | Safe Windows file-ID API (EXP-PLAT-016). |

## Candidates

Candidate sets are the ledger's R0 sets; completeness (status quo, SPEC design, named alternatives, strongest incumbent, defer, critic candidate, corrective candidate) is checked by the R0 completeness critic, not by this design. Where a set lacks a strongest-incumbent reference, the incumbent systems measured below act as REF-INC for the metric (objective §3.0).

- **Reference candidate:** Status quo production behaviour (`status-quo-refuse-pack`, `four-kinds-status-quo`, `abort-status-quo`, `status-quo-all-non-group-metadata`, `status-quo-silent`); REF-INC: GNU tar 1.35, bsdtar 3.7.2, cpio.
- **W and S:** determined on tuning (R3/R4) and declared in the §12 record before any validation look (§4.12).

Descriptions of every candidate are in the ledger row (`candidate_set`); they are not repeated here.

- **DEC-PLT-046** — candidates: `status-quo-refuse-pack`, `omit-and-report-entry-scoped`, `first-class-kinds-spec`, `kinds-without-socket`, `aux-snapshot-metadata`, `policy-selectable-refuse-or-omit`, `post-v1-feature-bit` (status quo: `status-quo-refuse-pack`). Treatment: Status quo measured directly; alternative dispositions are scored by applying their written rule to the measured feasibility table and benign prevalence (`[derived]`), since no production implementation exists.
- **DEC-MOD-021** — candidates: `four-kinds-status-quo`, `spec-seven-kinds`, `special-files-as-metadata`, `whiteout-kind`, `unknown-kind-fail-closed` (status quo: `four-kinds-status-quo`). Treatment: Status quo measured directly; alternative dispositions are scored by applying their written rule to the measured feasibility table and benign prevalence (`[derived]`), since no production implementation exists.
- **DEC-PLT-018** — candidates: `abort-status-quo`, `copy-fallback-with-policy-and-report`, `reflink-then-copy`, `skip-alias-with-report`, `caller-grant-for-fallback` (status quo: `abort-status-quo`). Treatment: Status quo measured directly; alternative dispositions are scored by applying their written rule to the measured feasibility table and benign prevalence (`[derived]`), since no production implementation exists.
  - `reflink-then-copy`: Feasibility of FICLONE measured on btrfs and XFS(reflink=1).
  - excluded before execution (ledger): `unchecked-copy-fallback` — SPEC Pattern 6 and incumbent CVEs (Python gh-151987, LinkFallbackError) require policy re-application on link emulation (violates SPEC §11.6 L1432 frozen extraction pattern); L0/R1(b) counterexample and reviewer sign-off still required (R1).
- **DEC-PLT-019** — candidates: `status-quo-all-non-group-metadata`, `enumerate-current-set`, `exclude-per-link-items`, `per-name-scope-flag` (status quo: `status-quo-all-non-group-metadata`). Treatment: Status quo measured directly; alternative dispositions are scored by applying their written rule to the measured feasibility table and benign prevalence (`[derived]`), since no production implementation exists.
- **DEC-PLT-033** — candidates: `status-quo-silent`, `declare-partial-link-loss`, `refuse-pack-policy`, `windows-file-id-detection`, `windows-links-declared-unavailable` (status quo: `status-quo-silent`). Treatment: Status quo measured directly; alternative dispositions are scored by applying their written rule to the measured feasibility table and benign prevalence (`[derived]`), since no production implementation exists.

## Corpus, fixtures and case sets

- **Fixtures:** special-file and hardlink trees generated per seed (tuning 20260917 `plat009-fixture-tuning`, validation 20260918 `plat009-fixture-validation`): FIFO, socket, char/block devices with large major/minor (4095/1048575), 2-, 3- and 1,000-member hardlink groups, groups spanning directories, groups with differing ACLs attempted, a subtree containing one alias of an external group.
- **Corpus:** `f19-tuning-rsnapshot-a`, `f19-validation-rsnapshot-b` (hardlink farms), `f17-tuning-duptree-mixed`, `f17-validation-duptree-vendor` (duplicate content on distinct inodes, MVT-17(d) negative control), `f19-tuning-zoo` (devices, FIFOs, sockets), `f16-oci-ubuntu-noble-multiarch-index`, `f16-oci-fedora-44-zstd-layers` (layer tars with whiteout markers, unpacked into an overlayfs lower/upper pair).
- **Destinations:** ext4, XFS (reflink=1), btrfs, tmpfs, vfat, ntfs-3g, a cross-device bind mount inside the destination, optional cifs loopback; Windows NTFS D: for the inode-scope and hardlink-capture arms.
- **Splits:** tuning and validation only; validation looks are registered in `research/decisions/validation-looks.jsonl` (§5.2, at most two per decision). Generated fixtures are assigned to splits by seed as stated above. No spec selects held-out (Phase D placeholder below).

## Environment and platform requirements

- **env_ids:** `wsl-ubuntu` (root, non-root, user namespace), `windows-host` (non-admin). No timing.

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
- **ebr (WSL):** `wsl.exe -d Ubuntu -u root -- bash -lc 'cd /mnt/d/Projects/entrybound/entrybound && export PYTHONPATH=research/tools && PY=/root/eb-research/venv/bin/python && $PY -m ebr validate --spec research/experiments/EXP-PLAT-009/spec.yaml && $PY -m ebr run --spec research/experiments/EXP-PLAT-009/spec.yaml && $PY -m ebr verify-raw --spec research/experiments/EXP-PLAT-009/spec.yaml --refingerprint && $PY -m ebr normalize --spec research/experiments/EXP-PLAT-009/spec.yaml && $PY -m ebr report --spec research/experiments/EXP-PLAT-009/spec.yaml'`
- **ebr (Windows host, PowerShell 7):** `$env:PYTHONPATH='research/tools'; $env:EB_DATA_ROOT='D:/eb-research'; $env:TEMP='D:/eb-research/tmp'; $env:TMP=$env:TEMP; C:/Python313/python.exe -m ebr run --spec research/experiments/EXP-PLAT-009/spec-windows.yaml` followed by `verify-raw`, `normalize`, `report` with the same spec (for corpus items the Windows runs set `EB_DATA_ROOT=\\wsl.localhost\Ubuntu\root\eb-research` and copy items to D: inside the command).
- **Scripts:** `scripts/special_hardlink_arms.sh --arm <a> --system <s> --item <path> --item-id <id> --scratch <dir>` (pack/capture with `ebound pack`, `tar --format=pax -cf`, `bsdtar -cf`, `find . | cpio -o -H newc`, Python `tarfile`; restore with each system across destinations and contexts; observe with `stat -c '%F %t %T %h %i'` and `(st_dev, st_ino)` groups); `scripts/devt_formats.py` (ustar octal limits, pax `SCHILY.devmajor/devminor`, GNU base-256 round trips); `scripts/overlay_whiteouts.sh` (`mount -t overlay` with lower/upper/work dirs on ext4, apply layer deletions, capture the upper dir with `ebound pack` and GNU tar); `scripts/inode_scope_probe.py` and `.ps1` (change mode, owner, xattr, ACL, times, flags, NTFS attributes, ADS and DACL through one alias and observe through the other); `scripts/lone_alias.sh` (pack a subtree and inspect `hardlink_group` and FidelityReport via `ebr-inspect-meta` or `ebound inspect --json --provenance`).

## Seeds, warmups and replication

- **PCP-3** of `research/experiments/_index/platform-common-provisions.md` applies verbatim.

## Measurement method and metrics

- **Kinds:** capture outcome (captured, pack aborted, omitted with report, omitted silently) and restore outcome per destination and context.
- **Hardlinks:** restore outcome where links are impossible (abort, copy, skip, report), bytes duplicated by copy fallback, reflink availability.
- **Inode scope:** per (filesystem, metadata item): shared, per-link, or not applicable.
- **Lone aliases:** whether topology loss is declared; MVT-17(d): equal content on distinct inodes never becomes a group.

| Metric | OD | Role here | ebr family | Unit | Direction | Threshold / oracle reference | Source |
|---|---|---|---|---|---|---|---|
| `capture_fidelity_fraction` | OD-03 | primary per (kind or topology, source filesystem) | other | fraction | maximize | T-20 (`metric_thresholds.capture_fidelity_fraction`; A.2 role `banded`) | script |
| `restore_fidelity_fraction` | OD-03 | primary per (kind or topology, destination, context) | other | fraction | maximize | T-20 (`metric_thresholds.restore_fidelity_fraction`; A.2 role `banded`) | script |
| `hc07_mvt07a_undeclared_differences` | OD-03 | HC oracle output (objective §2.0: binary; any non-zero value is a committed violation artifact and an R1 matter; not an Appendix A.2 metric, never enters §6 utility) | correctness | count | equal_zero | HC-07 MVT-07(a) | script |
| `hc17_encoded_hazards` | OD-03 | HC oracle output (objective §2.0: binary; any non-zero value is a committed violation artifact and an R1 matter; not an Appendix A.2 metric, never enters §6 utility) | correctness | count | equal_zero | HC-17 MVT-17 | MVT-17(d) negative control |
| `roundtrip_mismatches` | OD-02 | binary gating | correctness | count | equal_zero | HC-02 (`metric_thresholds.roundtrip_mismatches`; A.2 role `binary`) | script |

## Normalization

Outcome vocabulary as above; bytes duplicated reported as a secondary quantity (`copy_fallback_extra_bytes`, no A.2 band). Reference: status quo.

## Statistical analysis

- **PCP-4** of `research/experiments/_index/platform-common-provisions.md` applies verbatim.

## Practical-significance thresholds

Referenced, never restated: each banded metric uses its `research/methods/thresholds.json` `metric_thresholds.<name>` entry (decision-method §3.2 ID in the metrics table; §3.1 semantics; §7.3 minimum-gain caps). Binary metrics and HC oracle outputs have no band (§3.3; objective §2.0 rule 1).

## Sensitivity analysis

- **PCP-5** of `research/experiments/_index/platform-common-provisions.md` applies verbatim.

## Held-out evaluation (Phase D placeholder)

**PCP-7**.

## Cost accounting inputs (decision-method §7)

**PCP-8**.

## Expected negative results worth recording

- Status quo whole-pack refusal of special files will score capture 0 on rootfs-like trees; this is recorded against the omit-and-report candidates' benign benefit from EXP-PLAT-003.
- Sockets have no restorable content; any 'first-class socket' candidate gains nothing measurable here.
- NTFS per-link timestamp visibility may be an observer artifact of directory-entry caching; both `Get-Item` and handle-based queries are recorded.

## Threats to validity

- Overlayfs in WSL uses the WSL kernel's overlay implementation; container runtimes may add opaque-directory xattrs requiring CAP_SYS_ADMIN semantics that differ (EXP-PLAT-024).
- Device numbers are kernel-specific; FreeBSD and macOS splits come from primary sources only.
- cifs loopback depends on Samba configuration (unix extensions), recorded in the environment capture.

## Estimated cost and effort

- **Compute:** about 4 machine-hours; **disk:** about 20 GB peak (transient scratch unless stated).
- **Timing required:** no.
- **Tooling to build:** research/experiments/EXP-PLAT-009/scripts/{special_hardlink_arms.sh,inode_scope_probe.py,inode_scope_probe.ps1,overlay_whiteouts.sh,devt_formats.py,lone_alias.sh} (mechanical)
- **Agent effort:** execution none (scripted); tooling scripted; judgment only for interpreting per-link versus per-inode tables.

## Deviations (append-only; UTC time, author, reason, results-visible yes/no)

_None._

## Outcome (after analysis)

- classification (`outcome.json`, §10.1), verdict table reference, confirmation-bias triggers evaluated (§10.2): _pending_
