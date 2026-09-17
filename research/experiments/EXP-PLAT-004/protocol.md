# EXP-PLAT-004: Metadata capture and restore fidelity matrix per class and platform pair

> **Design pre-registration draft** (Phase C-design, platform domain). Index status: **NEEDS_TOOLING**. Structure: decision-method §12.2 record fields plus the Phase C-design protocol fields. Shared provisions PCP-1…PCP-8 live in `research/experiments/_index/platform-common-provisions.md` and are part of this pre-registration. This file becomes the §12 `record.md` only after the gates are met.

## Pre-registration

- **decision_ids:** DEC-PLT-035, DEC-PLT-024, DEC-PLT-049, DEC-PLT-016, DEC-PLT-008, DEC-PLT-055, DEC-PLT-022, DEC-PLT-025, DEC-MOD-009, DEC-MOD-035, DEC-LEG-068
- **Program sections:** 15, 17, 19, 21
- **Decision type (proposed, not assigned):** EMPIRICAL (fidelity fractions and HC-07 oracle); the claim-restatement part of DEC-PLT-024 is EMPIRICAL wording checked mechanically against the measured matrix. Assignment is made by a session not involved in the candidates (decision-method §1.2).
- **od_ids / hc_ids (proposed for the schema-v2 assignment, §14 item 12):** OD-03 (M03.1, M03.2, M03.3), OD-18 (M18.2), OD-05 diagnostic metadata bytes for DEC-PLT-022; HC-07, HC-02 (content gating), HC-05 (default restore policy), HC-08.
- **Method:** `research/decision-method.md` and `research/methods/thresholds.json` at pre-registration commit `14b977c` (status `pre-registered`).
- **Pre-registration commit:** the checkpoint commit adding this file (design draft); the §12 record commit follows G-A; every run's raw `meta` git SHA must descend from it with `dirty = false` (§5.8 d).
- **Author, independence and exposure:** Phase C-design platform-domain designer (workflow `wf_47347bc4-611`); no candidate was constructed by this session (ledger R0 sets reproduced mechanically). Held-out identity exposure (event L7) is disclosed in **PCP-1**.
- **Public data sources used for tuning (§5.4 consequence 2):** upstream sources in `research/corpus/manifest.json` `provenance` for the tuning/validation items named below, plus the pinned tool downloads named under Commands.
- **Gates:** G-A to G-D as in **PCP-2**.

## Question and hypotheses

- **Question:** Per metadata class and (source platform, target platform, privilege) cell, what fraction of independently observed source metadata does Entrybound capture exactly and restore exactly, is every observed difference declared in the FidelityReport or ExtractionReport (HC-07 MVT-07(a)), and how does that compare with capability-matched incumbents (GNU tar pax, bsdtar, 7-Zip, wimlib, squashfs-tools, dar; tar.exe, 7-Zip and robocopy on Windows)?
- **H1:** The status quo restores POSIX classes exactly on Linux same-filesystem pairs under explicit grants, leaves Windows security descriptors, ADS, reparse and inode flags at capture fraction 0 on their native platforms, and has at least one undeclared difference (HC-07 violation) in the cells the ledger marks CONTRADICTED (NTFS ADS neither captured nor declared; lone hardlink aliases; atime/ctime/Linux btime silently absent); GNU tar pax is NON_INFERIOR on every POSIX class and wimlib is DISTINGUISHABLE_BETTER on NTFS classes captured through ntfs-3g.
- **H0 / equivalence:** Entrybound status quo and each incumbent are EQUIVALENT per class on every in-scope pair, and no undeclared difference exists.
- **Falsification of H1:** Zero undeclared differences for the status quo across all cells (then REQ-PLT-0079's CONTRADICTED state is itself contradicted), or GNU tar pax DISTINGUISHABLE_WORSE than Entrybound on some POSIX class on a Linux pair.

## Decisions informed

| Decision | Release relevance | Ledger question (abridged; full text in the ledger) | What this experiment contributes | Evidence still needed elsewhere |
|---|---|---|---|---|
| DEC-PLT-035 | V1_BLOCKING | Which metadata classes does stable v1 guarantee to capture and restore on each supported OS/filesystem (Linux ext4/XFS/btrfs/tmpfs, Windows NTFS/exFAT/ReFS, macOS APFS... | Per class × OS/filesystem capture and restore fractions and undeclared-loss audit, the basis for which classes a stable-v1 guarantee may name. | macOS/APFS/HFS+, ReFS, exFAT, SACL and Docker userns rows (EXP-PLAT-022/023/024). |
| DEC-PLT-024 | V1_IMPORTANT | Which incumbent-matrix metadata fidelity claims (Entrybound 'full' POSIX/Windows/macOS fidelity, tar parity, sole declared-loss reporting) survive measurement, and how... | Measured per-class matrix against GNU tar 1.35, bsdtar 3.7.2/3.8.8, 7-Zip 23.01 and squashfs-tools, and whether incumbent warnings constitute loss reports. | Apple Archive (EXP-PLAT-022); DwarFS xattr recheck is a descriptive incumbent row added here if the pinned dwarfs binary supports xattrs. |
| DEC-PLT-049 | V1_IMPORTANT | Which timestamps beyond mtime are captured, declared or restored in v1 (atime, ctime, Linux statx btime capture-only, Windows creation-time restore, macOS birthtime re... | Which of atime, ctime, btime (statx), Windows creation time are observed, captured, declared or restored per platform. | Out-of-range and precision behaviour (EXP-PLAT-006); AUX churn (EXP-PLAT-005); restore API feasibility (EXP-PLAT-016); macOS birthtime (EXP-PLAT-022). |
| DEC-PLT-016 | V1_IMPORTANT | Which file flags are captured and restored: a restorable subset of macOS UF/SF flags, exclusion of kernel-managed bits (SF_DATALESS, UF_COMPRESSED, SF_FIRMLINK), and a... | Linux inode flags (immutable, append, nodump, noatime, nocow, btrfs compression) capture/declaration status. | Restore order with immutable flags (EXP-PLAT-008); FS_IOC API (EXP-PLAT-016); macOS chflags (EXP-PLAT-022). |
| DEC-PLT-008 | V1_IMPORTANT | How are transparently compressed files (macOS decmpfs with UF_COMPRESSED, NTFS compression) captured and restored: decompressed content with the attribute dropped and... | NTFS `FILE_ATTRIBUTE_COMPRESSED` capture (content decompressed? attribute recorded? loss declared on restore?). | macOS decmpfs (EXP-PLAT-022). |
| DEC-PLT-055 | V1_IMPORTANT | Which accepted windows.file-attributes bits and windows.creation-time are restored under platform-metadata Restore, ignored or reported, including ENCRYPTED/COMPRESSED... | Status-quo restore of Windows attribute bits and creation time (reported unavailable) versus incumbents on NTFS D:. | Per-bit settability and prototype restore (EXP-PLAT-013 arm C, EXP-PLAT-016); exFAT/ReFS/OneDrive (EXP-PLAT-023). |
| DEC-PLT-022 | V1_IMPORTANT | Should pack and unpack expose --metadata <class> selection, what are the default captured and restored class sets, and what are the stable CLI class names? | Metadata byte overhead per class (fixture twins with and without the class) and default capture set behaviour. | AUX churn per class (EXP-PLAT-005); CLI class-name usability (HUMAN-FACING). |
| DEC-PLT-025 | V1_BLOCKING | Does v1 keep a closed numeric metadata-name registry (15 names) or add SPEC's open reverse-DNS x- namespace with unknown-item carriage, criticality and safe-to-copy ru... | Measured size of closed-registry gaps: classes observed on real platforms that the registry cannot carry, compared with tar/pax, WIM and dar coverage. | Interop/security cost of opaque x- items and fail-closed tests (EXP-PLAT-019). |
| DEC-MOD-009 | V1_IMPORTANT | How is metadata criticality exercised in v1 (which names are Critical, how readers treat unknown Critical items) and is restorability a per-name constant validated by... | Per platform, which classes' restoration failure changes security or access semantics (deny ACEs, immutable/append flags, encrypted/compressed attributes), identifying candidates for Critical names. | Fail-closed tests with constructed unknown Critical items (EXP-PLAT-019). |
| DEC-MOD-035 | V1_BLOCKING | Which metadata classes and model objects participate in which identity root (LAI, AUX, PCR, PCI or none), including pending classes such as ownership names, NTFS ADS/f... | Per-class fidelity and cross-platform restorability inputs for identity-root assignment. | LAI/AUX stability per class (EXP-PLAT-005); forks/markers (EXP-PLAT-012); security analysis of AUX-only security metadata (external review). |
| DEC-LEG-068 | V1_IMPORTANT | If a tar/pax-v2 profile is added, which semantics does it carry and how: symlinks, hardlink records versus copies, uid/gid and names (octal, base-256 or pax), xattrs (... | Round-trip fidelity per metadata class for existing pax key families (SCHILY.xattr, LIBARCHIVE.xattr, SCHILY.acl, birthtime) as written and read by GNU tar and bsdtar. | Reader matrix across busybox, star, Go, 7-Zip, Windows tar.exe and large-id encodings (legacy domain). |

## Candidates

Candidate sets are the ledger's R0 sets; completeness (status quo, SPEC design, named alternatives, strongest incumbent, defer, critic candidate, corrective candidate) is checked by the R0 completeness critic, not by this design. Where a set lacks a strongest-incumbent reference, the incumbent systems measured below act as REF-INC for the metric (objective §3.0).

- **Reference candidate:** Status quo production `ebound` (REF-PROD at the record commit) with default extraction policy; REF-INC per class: GNU tar 1.35 pax (POSIX classes), bsdtar 3.8.8 (Windows host) / 3.7.2 (Linux), wimlib (NTFS classes via ntfs-3g), robocopy `/COPY:DAT /DCOPY:DAT` plus `/SEC` on Windows (copy-tool reference for DACL and attributes).
- **W and S:** determined on tuning (R3/R4) and declared in the §12 record before any validation look (§4.12).

Descriptions of every candidate are in the ledger row (`candidate_set`); they are not repeated here.

- **DEC-PLT-035** — candidates: `status-quo-bootstrap-matrix`, `status-quo-plus-declared-gaps`, `tar-pax-parity-target`, `apple-archive-parity-macos`, `wim-robocopy-parity-windows`, `minimal-portable-guarantee` (status quo: `status-quo-bootstrap-matrix`, `status-quo-plus-declared-gaps`). Treatment: A guarantee-set candidate is falsified when it names a class whose measured restore fraction is below 1 on an in-scope pair or whose loss is undeclared; the measured matrix is the test.
- **DEC-PLT-024** — candidates: `keep-claims`, `restate-per-measured-matrix`, `remove-full-until-implemented`, `publish-per-class-matrix`. Treatment: Each claim label is checked against the measured per-class matrix; a DISTINGUISHABLE shortfall marks the claim CONTRADICTED (§9.1).
- **DEC-PLT-049** — candidates: `status-quo-mtime-plus-platform-birth`, `declare-unavailable-only`, `add-linux-btime-capture-only`, `add-atime-optional`, `add-ctime-capture-only`, `exclude-creation-time-by-default`, `restore-creation-time-and-birthtime` (status quo: `status-quo-mtime-plus-platform-birth`). Treatment: Status quo measured; alternatives scored by which classes are observable/restorable per platform (feasibility), with AUX-churn cost from EXP-PLAT-005.
- **DEC-PLT-016** — candidates: `status-quo`, `macos-user-flag-subset-restore`, `declare-linux-flags-unavailable`, `add-linux-flags-name`, `immutable-last-privileged-opt-in` (status quo: `status-quo`). Treatment: Status quo measured; declaration candidates scored on undeclared-loss count; capture/restore candidates need EXP-PLAT-016 prototypes.
- **DEC-PLT-008** — candidates: `status-quo-decompressed-read-flag-recorded`, `opaque-capture-only-blob`, `drop-with-declared-loss`, `recompress-on-restore` (status quo: `status-quo-decompressed-read-flag-recorded`). Treatment: Status quo measured on NTFS; macOS candidates blocked.
  - excluded before execution (ledger): `synthesize-decmpfs` — SPEC forbids synthesising the undocumented macOS compression attribute. (violates SPEC §10.7 L1296); L0/R1(b) counterexample and reviewer sign-off still required (R1).
- **DEC-PLT-055** — candidates: `status-quo-report-unavailable`, `restore-basic-bits-and-creation-time`, `restore-compression-via-fsctl`, `never-restore-cloud-bits`, `efs-raw-apis-post-v1` (status quo: `status-quo-report-unavailable`). Treatment: Status quo and incumbents measured here; per-bit prototype restore measured in EXP-PLAT-013.
- **DEC-PLT-022** — candidates: `status-quo-capture-all-restore-per-flag`, `spec-metadata-class-flags`, `profile-based-classes`, `capture-all-restore-selection-only`, `exclude-volatile-classes-by-default` (status quo: `status-quo-capture-all-restore-per-flag`). Treatment: Overhead per class measured; CLI-surface candidates are not measurable here.
- **DEC-PLT-025** — candidates: `closed-registry-status-quo`, `spec-x-namespace-reverse-dns`, `x-namespace-plus-safe-to-copy`, `registry-only-growth`, `central-iana-style-registry`, `fill-spec-v1-membership` (status quo: `closed-registry-status-quo`). Treatment: Registry-gap size measured; namespace mechanisms evaluated in EXP-PLAT-019.
- **DEC-MOD-009** — candidates: `all-optional-per-name-status-quo`, `registry-defined-critical-names`, `per-item-override`, `drop-per-item-fields`, `restorability-runtime-capability` (status quo: `all-optional-per-name-status-quo`). Treatment: Security-relevance classification of classes derived from the matrix; per-item mechanisms evaluated in EXP-PLAT-019.
- **DEC-MOD-035** — candidates: `status-quo-assignment`, `named-content-streams-in-lai`, `forks-in-aux`, `origin-markers-none`, `origin-markers-aux`, `refuse-unassigned` (status quo: `status-quo-assignment`). Treatment: Contributes fidelity inputs only; selection happens with EXP-PLAT-005 identity results.
  - excluded before execution (ledger): `unauthenticated-hints` — Leaves asserted archive bytes outside LAI/AUX, allowing silent alteration in signed archives (violates SPEC §8.0 L885 metadata coverage; I13); L0/R1(b) counterexample and reviewer sign-off still required (R1).
- **DEC-LEG-068** — candidates: `status-quo-no-v2`, `v2-links-and-ownership`, `v2-gnu-compatible`, `v2-libarchive-compatible`, `v2-densify-sparse`, `v2-oci-windows-keys` (status quo: `status-quo-no-v2`). Treatment: Incumbent key families measured as proxies for each v2 candidate's semantics.

## Corpus, fixtures and case sets

- **Fixtures (primary case lists):** `scripts/gen_fidelity_fixture.py --seed <seed> --profile posix` (Linux, root) and `scripts/gen_fidelity_fixture.ps1 -Seed <seed> -Profile windows` (non-admin), one exemplar entry per (class, variant) with a twin fixture that omits the class (for byte overhead). Tuning seed 20260917 (`plat004-fixture-tuning`), validation seed 20260918 (`plat004-fixture-validation`), held-out seed sealed.
- **Class list (fixed before results; OD-03 rule):** timestamps (mtime; atime/ctime observation-only; btime/creation time) with precision; mode bits including setuid/setgid/sticky; numeric uid/gid; user/group names; xattrs by namespace (user, trusted, security.selinux, security.capability); POSIX1E access and default ACLs; Windows owner/group/DACL; Windows attribute bits; NTFS ADS including Zone.Identifier; junction reparse points; symlinks (relative, absolute, dangling, non-UTF-8 target); hardlink topology including lone aliases; sparse layout; empty directories; Linux inode flags; NTFS compressed attribute; FIFOs/devices/sockets. A class enters a pair's list only if EXP-PLAT-001 shows the observers can read and write it on both platforms of the pair.
- **Corpus items (tuning/validation F19):** `f19-tuning-zoo`, `f19-tuning-real-ext4-acl-selinux`, `f19-tuning-real-ntfs-metadata`, `f19-tuning-rsnapshot-a`, `f19-tuning-real-home-snapshot`, `f19-validation-deep`, `f19-validation-real-xfs-acl-selinux`, `f19-validation-real-ntfs-metadata`, `f19-validation-rsnapshot-b`, `f19-validation-real-home-snapshot`. Filesystem-image items are loop-mounted read-only as capture sources (NTFS images via ntfs-3g `streams_interface=xattr`, so ADS surface as `user.*` xattrs and DOS attributes as `system.ntfs_attrib`).
- **Platform pairs (evaluation conditions):** same-filesystem ext4→ext4, xfs→xfs, btrfs→btrfs, tmpfs→tmpfs, vfat→vfat, ntfs3g→ntfs3g, ntfsD→ntfsD; cross-filesystem xfs→ext4 (oversize xattrs), btrfs→ext4, ext4→vfat, ext4→ntfs3g; cross-OS wsl-ext4→ntfsD and ntfsD→wsl-ext4. Restore privilege contexts: Linux root and non-root (`setpriv --reuid=65534`); Windows non-admin.
- **Splits:** tuning and validation only; validation looks are registered in `research/decisions/validation-looks.jsonl` (§5.2, at most two per decision). Generated fixtures are assigned to splits by seed as stated above. No spec selects held-out (Phase D placeholder below).

## Environment and platform requirements

- **env_ids:** `wsl-ubuntu`, `windows-host`; cross-OS pairs are driven from the Windows host (`cross_os_driver.ps1` calls WSL through `wsl.exe -d Ubuntu -u root --`). No timing.

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
- **Harness (only where named):** `wsl.exe -d Ubuntu -- bash /mnt/d/Projects/entrybound/entrybound/research/harness/build.sh build --release` (→ `/root/eb-research/target/harness/release/`), after `C:/Python313/python.exe research/harness/lock_parity.py` reports `RESULT PASS` (harness review R1-06).
- **Pre-flight:** `C:/Python313/python.exe research/orchestration/disk_guard.py` must pass; no other workflow may hold the loop devices or mount points named here.
- **ebr (WSL):** `wsl.exe -d Ubuntu -u root -- bash -lc 'cd /mnt/d/Projects/entrybound/entrybound && export PYTHONPATH=research/tools && PY=/root/eb-research/venv/bin/python && $PY -m ebr validate --spec research/experiments/EXP-PLAT-004/spec.yaml && $PY -m ebr run --spec research/experiments/EXP-PLAT-004/spec.yaml && $PY -m ebr verify-raw --spec research/experiments/EXP-PLAT-004/spec.yaml --refingerprint && $PY -m ebr normalize --spec research/experiments/EXP-PLAT-004/spec.yaml && $PY -m ebr report --spec research/experiments/EXP-PLAT-004/spec.yaml'`
- **ebr (Windows host, PowerShell 7):** `$env:PYTHONPATH='research/tools'; $env:EB_DATA_ROOT='D:/eb-research'; $env:TEMP='D:/eb-research/tmp'; $env:TMP=$env:TEMP; C:/Python313/python.exe -m ebr run --spec research/experiments/EXP-PLAT-004/spec-windows.yaml` followed by `verify-raw`, `normalize`, `report` with the same spec (for corpus items the Windows runs set `EB_DATA_ROOT=\\wsl.localhost\Ubuntu\root\eb-research` and copy items to D: inside the command).
- **ebr (Windows host, PowerShell 7):** `$env:PYTHONPATH='research/tools'; $env:EB_DATA_ROOT='D:/eb-research'; $env:TEMP='D:/eb-research/tmp'; $env:TMP=$env:TEMP; C:/Python313/python.exe -m ebr run --spec research/experiments/EXP-PLAT-004/spec-crossos.yaml` followed by `verify-raw`, `normalize`, `report` with the same spec (for corpus items the Windows runs set `EB_DATA_ROOT=\\wsl.localhost\Ubuntu\root\eb-research` and copy items to D: inside the command).
- **Systems (exact invocations, in `scripts/run_system.sh`):**
  - `eb-default`: `ebound pack <src> <scratch>/a.eb` then `ebound unpack <scratch>/a.eb <dst>`.
  - `eb-restore-all`: unpack with `--symlinks all --restore-owner --xattrs restore --sparse restore --acls restore --windows-security restore --reparse all --platform-metadata restore`.
  - `gnutar`: `tar --format=pax --xattrs --xattrs-include='*' --acls --selinux --numeric-owner -S -cf a.tar -C <src> .` / `tar -xpf a.tar --xattrs --xattrs-include='*' --acls --selinux --same-owner -C <dst>`.
  - `bsdtar`: `bsdtar --xattrs --acls --fflags -cf a.tar -C <src> .` / `bsdtar -xpf a.tar --xattrs --acls --fflags --same-owner -C <dst>`.
  - `sevenzip`: `7z a -snl -snh a.7z <src>/.` / `7z x -snl -snh -o<dst> a.7z`; `wimlib`: `wimcapture <src> a.wim --unix-data` (for NTFS images: `wimcapture <ntfs.img> a.wim`) / `wimapply a.wim <dst> --unix-data`; `squashfs`: `mksquashfs <src> a.sqfs -xattrs` / `unsquashfs -xattrs -d <dst> a.sqfs`; `dar`: `dar -c a -R <src> -am` / `dar -x a -R <dst>`.
  - Windows (`scripts/run_system.ps1`): `ebound.exe pack/unpack` (same two policies), `tar.exe -cf/-xpf`, 7-Zip `a -snl -sni -sns -snh` / `x -snl -sni -sns -snh`, `robocopy <src> <dst> /E /COPY:DAT /DCOPY:DAT /SL` and a second run with `/SEC`.
- **Observers:** `scripts/observe_linux.py <root>` (stat including `%W`, `getfattr -h -d -m - -e hex`, `getfacl -n -P`, `lsattr -d`, SEEK_DATA extents after `posix_fadvise(DONTNEED)`, `readlink` bytes, `(st_dev, st_ino)` groups) and `scripts/observe_windows.ps1 <root>` (`Get-Item` attributes and ticks, `Get-Item -Stream *` sizes and SHA-256, `Get-Acl` with `RawSecurityDescriptor.GetBinaryForm`, `fsutil reparsepoint query`, `fsutil file queryfileid`, `fsutil hardlink list`, `compact /q`).
- **Oracle:** `scripts/fidelity_oracle.py --source <obs.json> --dest <obs.json> --archive-meta <ebr-inspect-meta.json> --extraction-report <unpack.log> --classes <pair-class-list.json>` → per class captured_exact, restored_exact, declared, undeclared; for incumbents, capture is judged from archive listings where the format exposes them (`tar --list -v --xattrs --acls`, `bsdtar -tvv`, `7z l -slt`, `wiminfo`, `wimdir --detailed`) and otherwise restore-only.

## Seeds, warmups and replication

- **PCP-3** of `research/experiments/_index/platform-common-provisions.md` applies verbatim.

## Measurement method and metrics

- **Capture:** source observation versus the archive's recorded metadata (`ebr-inspect-meta` for Entrybound; listings for incumbents). Exact means equal value and, for timestamps, equal precision class where the format records one.
- **Restore:** source observation versus destination observation after extraction, per class, per privilege context.
- **Declaration audit (HC-07 MVT-07(a)):** the symmetric difference of source and destination observations, minus items covered by a typed FidelityReport or ExtractionReport entry naming that class and entry scope, must be empty; the residue is `hc07_mvt07a_undeclared_differences`. Incumbent warnings on stderr are recorded verbatim and classified (does the warning name the class and entries?) for DEC-PLT-024.
- **Content gating:** every sample checks content equality (`roundtrip_mismatches`); a sample with a content mismatch is excluded from OD analysis and reported as an HC-02 violation (objective §3.0).
- **Overhead (DEC-PLT-022):** `metadata_bytes` of the fixture pack minus the class-free twin, from `ebr-pack` exact section accounting.
- **Oracle validation (before first use):** 200 seeded injected undeclared differences (one per class, strip or perturb in the destination) must all be flagged, and 200 declared-only differences must all pass; failure blocks the experiment.

| Metric | OD | Role here | ebr family | Unit | Direction | Threshold / oracle reference | Source |
|---|---|---|---|---|---|---|---|
| `capture_fidelity_fraction` | OD-03 | primary (M03.1) per (class, source platform) | other | fraction | maximize | T-20 (`metric_thresholds.capture_fidelity_fraction`; A.2 role `banded`) | oracle |
| `restore_fidelity_fraction` | OD-03 | primary (M03.2) per (class, pair, privilege) | other | fraction | maximize | T-20 (`metric_thresholds.restore_fidelity_fraction`; A.2 role `banded`) | oracle |
| `cross_platform_restore_fidelity_fraction` | OD-18 | primary (M18.2) for pairs with different source and target platforms | other | fraction | maximize | T-20 (`metric_thresholds.cross_platform_restore_fidelity_fraction`; A.2 role `banded`) | oracle |
| `declared_gap_count` | OD-03 | descriptive (M03.3) | other | count | report | - (`metric_thresholds.declared_gap_count`; A.2 role `descriptive`) | oracle |
| `hc07_mvt07a_undeclared_differences` | OD-03 | HC oracle output (objective §2.0: binary; any non-zero value is a committed violation artifact and an R1 matter; not an Appendix A.2 metric, never enters §6 utility) | correctness | count | equal_zero | HC-07 MVT-07(a) | oracle |
| `roundtrip_mismatches` | OD-02 | binary gating (HC-02) | correctness | count | equal_zero | HC-02 (`metric_thresholds.roundtrip_mismatches`; A.2 role `binary`) | oracle |
| `metadata_bytes` | OD-05 | diagnostic; primary only for DEC-PLT-022 (the decision is about encoding the class) | size | B | minimize | T-02 (`metric_thresholds.metadata_bytes`; A.2 role `diagnostic`) | `ebr-pack` section accounting |
| `platform_coverage_count` | OD-18 | descriptive (M18.4) | other | count | report | - (`metric_thresholds.platform_coverage_count`; A.2 role `descriptive`) | analysis |

## Normalization

Per (system, fixture/item, class, pair, privilege) the oracle emits counts of observed source items, exact captures, exact restores, declared differences and undeclared differences; fractions are exact counts over observed source items of the class. The reference system for relative statements is `eb-default`; incumbents are REF-INC per class. The full matrix is written to `research/raw/EXP-PLAT-004/detail/` and summarized in `research/normalized/EXP-PLAT-004/decision/fidelity-matrix.csv`.

## Statistical analysis

- **PCP-4** of `research/experiments/_index/platform-common-provisions.md` applies verbatim.
- **Headline per pair:** minimum over the pair's listed classes (a class a candidate does not restore counts 0 whether or not declared; objective OD-03).
- **Claims (DEC-PLT-024/035):** every published label or guarantee candidate is evaluated per pair; a label that exceeds the measured matrix is recorded as `CONTRADICTED`; declared-non-restorable classes are reported against GNU tar 1.35 pax and bsdtar 3.8.8 capability.
- **Incumbent comparisons** are capability-matched per `research/baselines/README.md` fair-comparison rules; a system lacking a class is compared only on classes both carry, and the omission is stated.

## Practical-significance thresholds

Referenced, never restated: each banded metric uses its `research/methods/thresholds.json` `metric_thresholds.<name>` entry (decision-method §3.2 ID in the metrics table; §3.1 semantics; §7.3 minimum-gain caps). Binary metrics and HC oracle outputs have no band (§3.3; objective §2.0 rule 1).

## Sensitivity analysis

- **PCP-5** of `research/experiments/_index/platform-common-provisions.md` applies verbatim.
- **Privilege arm:** root versus non-root restore reported separately; no pooling.

## Held-out evaluation (Phase D placeholder)

**PCP-7**.

## Cost accounting inputs (decision-method §7)

**PCP-8**.

## Expected negative results worth recording

- Status quo is expected to show capture fraction 0 for Windows security descriptors (declared unavailable by design) and for ADS (not captured and — per ledger — not declared, which would be an HC-07 violation).
- The baseline probe's whole-tree refusal for ACL mask/mode disagreement makes every class of an affected fixture score 0 captured; that cost is recorded.
- wimlib reading NTFS images may out-capture Entrybound on Windows classes; GNU tar pax may equal it on POSIX classes; both are negative results for SPEC §22.2 'full fidelity' labels if confirmed.

## Threats to validity

- Observers use the same kernel and libc views as the tools under test; they are independent of Entrybound's capture code but not of the OS.
- ntfs-3g translates Windows metadata into POSIX views; NTFS-image pairs measure the ntfs-3g transport, not the Windows driver; native NTFS pairs run on D:.
- Non-admin Windows cannot restore owners other than the current user or any SACL; those cells are HC_UNVERIFIED (EXP-PLAT-023), not failures.
- SELinux is not enforcing in WSL; `security.selinux` is only an xattr here.
- Defender may touch atime on Windows; atime is observation-only.

## Estimated cost and effort

- **Compute:** about 14 machine-hours; **disk:** about 45 GB peak (transient scratch unless stated).
- **Timing required:** no.
- **Tooling to build:** research harness binary `ebr-inspect-meta` (new, in crates/ebr-pack): dumps full EAM metadata per entry (xattr values, ACL entries, descriptor bytes, attribute masks, timestamps with precision, sparse maps, FidelityReport issues) through entrybound's public API; research/experiments/EXP-PLAT-004/scripts/{gen_fidelity_fixture.py,gen_fidelity_fixture.ps1,observe_linux.py,observe_windows.ps1,fidelity_oracle.py,run_system.sh,run_system.ps1,cross_os_driver.ps1}; oracle self-validation: injected undeclared-loss and declared-only mutations must be flagged 100% / 0% before first use; apt: dar (approved download); 7-Zip for Windows portable (pinned)
- **Agent effort:** execution none (scripted); tooling scripted plus one judgment review of the oracle's declaration-matching rules; analysis judgment for claim restatement (DEC-PLT-024/035).

## Deviations (append-only; UTC time, author, reason, results-visible yes/no)

_None._

## Outcome (after analysis)

- classification (`outcome.json`, §10.1), verdict table reference, confirmation-bias triggers evaluated (§10.2): _pending_
