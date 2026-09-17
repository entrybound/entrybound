# EXP-PLAT-005: Cross-host identity stability, feature-bit census and unobservable-semantics re-projection (F-0001/F-0002 follow-up)

> **Design pre-registration draft** (Phase C-design, platform domain). Index status: **NEEDS_TOOLING**. Structure: decision-method §12.2 record fields plus the Phase C-design protocol fields. Shared provisions PCP-1…PCP-8 live in `research/experiments/_index/platform-common-provisions.md` and are part of this pre-registration. This file becomes the §12 `record.md` only after the gates are met.

## Pre-registration

- **decision_ids:** DEC-MOD-049, DEC-MOD-050, DEC-CON-030, DEC-MOD-002, DEC-MOD-022, DEC-MOD-030, DEC-MOD-035, DEC-MOD-028, DEC-MOD-025, DEC-MOD-019, DEC-CON-010, DEC-CRY-008, DEC-PLT-003, DEC-PLT-022, DEC-PLT-049, DEC-PLT-062
- **Program sections:** 15, 16, 17, 19, 20, 21
- **Decision type (proposed, not assigned):** EMPIRICAL (cross-host agreement, feature-bit shares, AUX churn) plus FORMAL parts (each candidate's participation model is a derivation from its written rule; model validation against production digests is the empirical check). Assignment is made by a session not involved in the candidates (decision-method §1.2).
- **od_ids / hc_ids (proposed for the schema-v2 assignment, §14 item 12):** OD-18 (M18.1), OD-04, OD-21 (M21.6 `reader_fragmentation_count` as cost input), OD-05 diagnostic metadata bytes, OD-16 for public-bit exposure (descriptive); HC-08, HC-10, HC-13, HC-04, HC-01, HC-07.
- **Method:** `research/decision-method.md` and `research/methods/thresholds.json` at pre-registration commit `14b977c` (status `pre-registered`).
- **Pre-registration commit:** the checkpoint commit adding this file (design draft); the §12 record commit follows G-A; every run's raw `meta` git SHA must descend from it with `dirty = false` (§5.8 d).
- **Author, independence and exposure:** Phase C-design platform-domain designer (workflow `wf_47347bc4-611`); no candidate was constructed by this session (ledger R0 sets reproduced mechanically). Held-out identity exposure (event L7) is disclosed in **PCP-1**.
- **Public data sources used for tuning (§5.4 consequence 2):** upstream sources in `research/corpus/manifest.json` `provenance` for the tuning/validation items named below, plus the pinned tool downloads named under Commands.
- **Gates:** G-A to G-D as in **PCP-2**.

## Question and hypotheses

- **Question:** When the same logical tree is packed on different hosts, filesystems and architectures, and after common copy and checkout operations, which identity roots (LAI, AUX, PCR, PCI) and required feature bits differ, which recorded facts cause each difference, and which candidate representation removes every unexplained cross-host difference without inventing facts: tri-state or caller-supplied executable state, exclusion from LAI, canonical or out-of-AUX timestamp precision, deterministic-mode normalisation of host-dependent AUX, and a compatible tier, per-record criticality, default-off capture or profile split for AUX-only platform metadata?
- **H1:** Under the status quo, LAI differs between NTFS and every POSIX filesystem for every tree containing an executable file (F-0002), AUX differs across every host pair, and every Windows-packed archive requires feature 0x10000 (F-0001), making all of them unreadable by a reader without platform-security-metadata-v1 (`reader_fragmentation_count` ≥ 1). The tri-state executable and exclude-from-LAI candidates both yield LAI agreement on all POSIX↔NTFS pairs, canonical precision yields AUX agreement for identical instants, and only the compat-tier and profile-split candidates reduce the minimal-reader refusal share to 0 while keeping HC-04 fail-closed tests passing.
- **H0 / equivalence:** All candidates within a decision are EQUIVALENT on cross-host agreement (the status quo already agrees), or no candidate achieves agreement on some pair.
- **Falsification of H1:** LAI agrees across NTFS and ext4 under the status quo for trees with executables (contradicting F-0002), or the re-projection model fails its validation against production digests (then candidate results are not decision-grade), or the aarch64 QEMU control disagrees with native x86_64 (then the EMULATED arm is invalid).

## Decisions informed

| Decision | Release relevance | Ledger question (abridged; full text in the ledger) | What this experiment contributes | Evidence still needed elsewhere |
|---|---|---|---|---|
| DEC-MOD-049 | V1_BLOCKING | How should identity-participating semantics that a capture platform cannot observe (e.g. POSIX executable state on NTFS) be represented so that LAI is canonical across... | Inventory of LAI-participating fields and their observability per platform; cross-host LAI agreement per candidate over real trees. | Wire/identity cost (C1 scripts); extraction semantics of unknown executable state (analytical plus EXP-PLAT-008). |
| DEC-MOD-050 | V1_IMPORTANT | Should timestamp precision recorded from the capturing filesystem be canonicalized (or excluded) so identical instants yield identical AUX across hosts? | AUX agreement across NTFS, ext4, XFS, btrfs, tmpfs, vfat captures of identical instants per precision candidate. | Restore implications (EXP-PLAT-006). |
| DEC-CON-030 | V1_BLOCKING | Which feature-tier mechanism should gate AUX-only platform metadata (e.g. platform-security-metadata-v1) so that Windows-created archives are not unreadable to readers... | Required feature bits for default capture per host; share of Windows-packed archives unreadable by a reader without platform-security-metadata-v1 per tier candidate; HC-04 checks for each candidate's critical items. | Minimal/independent reader cost (conformance domain); fail-closed semantics analysis (EXP-PLAT-019); macOS column (EXP-PLAT-022); the F-0001 test-portability fix is a production work item, not an experiment. |
| DEC-MOD-002 | V1_IMPORTANT | Should AUX avoid host-dependent inputs (sparse maps from allocation state, fidelity platform/filesystem strings, Windows creation times) in deterministic mode, and sho... | Count of AUX differences attributable to each host-dependent field (sparse maps from allocation state, fidelity platform strings, creation times) after ext4/XFS/btrfs/tmpfs/NTFS captures and cp/rsync/robocopy variants. | Privacy review of fidelity fields (external/analytical); duplicate-declaration rejection tests (EXP-PLAT-019). |
| DEC-MOD-022 | V1_BLOCKING | How is core.executable derived from POSIX mode (owner vs any x bit) and from non-POSIX sources (Windows, DOS/NTFS ZIP, 7z), and must it always be present on File entri... | LAI comparison of the same tree packed on Linux and Windows and via ZIP/7z/tar import; executable derivation outcomes per candidate. | Writer/adapter emission audit (legacy/conformance domains). |
| DEC-MOD-030 | V1_BLOCKING | Which metadata participate in LAI under identity/v1: only core.executable (status quo), none, a NAR-like set, or mode/ownership/ACLs? | LAI stability across hosts and filesystems for each participation set (none, NAR-like, executable-only, mode bits, mode plus ACLs). | Consumer mapping and permission-tampering threat model (analytical/external). |
| DEC-MOD-035 | V1_BLOCKING | Which metadata classes and model objects participate in which identity root (LAI, AUX, PCR, PCI or none), including pending classes such as ownership names, NTFS ADS/f... | LAI/AUX stability when each metadata class is captured on different hosts. | Security analysis of AUX-only security metadata under content-binding signatures (external review). |
| DEC-MOD-028 | V1_IMPORTANT | What must identity/strict-v1 contain (mode bits, ownership names vs numeric ids vs SIDs, mtime precision and normalisation, hardlink grouping, xattrs, ACLs, platform m... | Cross-host/filesystem stability of mode bits, numeric ids, names, SIDs and mtime precision for each strict-profile candidate (with EXP-PLAT-007 namespace scenarios). | Consumer requirement survey (HUMAN-FACING, blocked); implementation cost. |
| DEC-MOD-025 | POST_V1 | Is the frozen path-derived hardlink group identity (hash of content digest and sorted member paths) the right construction, given AUX churn on alias renames? | AUX changes and diff report size when one hardlink alias is renamed, added or removed, per group-identity construction; determinism after copying trees across filesystems. | I1 analysis per representation (analytical). |
| DEC-MOD-019 | V1_IMPORTANT | Should declared path encodings participate in entry identity and LAI, and how must legacy adapters choose declarations deterministically? | Legacy archives where identical name bytes receive different encoding declarations across adapters and modes, and the resulting LAI differences. | Security analysis of declaration exclusion (analytical, EXP-PLAT-014 arm H). |
| DEC-CON-010 | V1_BLOCKING | For container, compression and auxiliary features (0x1, 0x2, 0x4, 0x8, 0x10, 0x2000, 0x4000) and their interplay with metadata bits, should writers set bits only when... | Share of archives per profile and host requiring each feature bit; determinism of active feature sets across runs, hosts and planner ids. | Audit/fallback record bytes and reader refusals avoided (container domain). |
| DEC-CRY-008 | V1_IMPORTANT | Should encrypted archives expose content-dependent incompatibility bits (0x8000 POSIX/symlinks, 0x10000 platform security) in the public preamble, always set them, or... | Enumeration of content-dependent public required-feature bits in encrypted archives packed on each host. | Reader early-failure impact and wire cost of private declaration (crypto domain). |
| DEC-PLT-003 | V1_IMPORTANT | Should capture detect and record source filesystem type and per-filesystem timestamp granularity and range (FidelityReport.filesystem, per-filesystem precision tags),... | AUX host dependence caused by recording filesystem type or precision tags, per candidate. | Detection accuracy (EXP-PLAT-001); safe APIs (EXP-PLAT-016). |
| DEC-PLT-022 | V1_IMPORTANT | Should pack and unpack expose --metadata <class> selection, what are the default captured and restored class sets, and what are the stable CLI class names? | AUX churn per metadata class under copy variants (the default-capture-set question). | Byte overhead per class (EXP-PLAT-004). |
| DEC-PLT-049 | V1_IMPORTANT | Which timestamps beyond mtime are captured, declared or restored in v1 (atime, ctime, Linux statx btime capture-only, Windows creation-time restore, macOS birthtime re... | AUX churn caused by creation/birth times across copies. | Observability and restore (EXP-PLAT-004/006). |
| DEC-PLT-062 | V1_BLOCKING | How does stable v1 disposition capabilities that cannot be evaluated locally (macOS/APFS metadata, native ARM64 performance, ReFS, admin-only Windows operations): keep... | Emulation validity check for ARM64 determinism: QEMU-user aarch64 versus native x86_64, with a QEMU-user x86_64 control. | Native ARM64 (EXP-PLAT-025, BLOCKED). |

## Candidates

Candidate sets are the ledger's R0 sets; completeness (status quo, SPEC design, named alternatives, strongest incumbent, defer, critic candidate, corrective candidate) is checked by the R0 completeness critic, not by this design. Where a set lacks a strongest-incumbent reference, the incumbent systems measured below act as REF-INC for the metric (objective §3.0).

- **Reference candidate:** Status quo production `ebound` at the record commit on each host (`status-quo-false`, `status-quo-host-precision`, `status-quo-incompat`, `status-quo-capture-all`, ...).
- **W and S:** determined on tuning (R3/R4) and declared in the §12 record before any validation look (§4.12).

Descriptions of every candidate are in the ledger row (`candidate_set`); they are not repeated here.

- **DEC-MOD-049** — candidates: `status-quo-false`, `tri-state-unknown`, `caller-policy-default`, `exclude-from-lai`, `source-metadata-import` (status quo: `status-quo-false`). Treatment: Evaluated through the re-projection model on measured per-entry facts (status quo additionally through real production digests).
- **DEC-MOD-050** — candidates: `status-quo-host-precision`, `canonical-precision`, `precision-out-of-aux` (status quo: `status-quo-host-precision`). Treatment: Evaluated through the re-projection model on measured per-entry facts (status quo additionally through real production digests).
- **DEC-CON-030** — candidates: `status-quo-incompat`, `compat-tier`, `per-record-criticality`, `capture-policy-default-off`, `profile-split` (status quo: `status-quo-incompat`). Treatment: Evaluated through the re-projection model on measured per-entry facts (status quo additionally through real production digests).
  - `status-quo-incompat`: Measured directly (production feature masks).
  - `capture-policy-default-off`: Measured by re-projection (platform_security dropped) and by real Windows packs with the platform metadata absent where the status quo already omits it.
- **DEC-MOD-002** — candidates: `status-quo-capture-all`, `deterministic-mode-normalise`, `sparse-maps-excluded`, `structured-fidelity-vocabulary`, `reject-duplicates` (status quo: `status-quo-capture-all`). Treatment: Evaluated through the re-projection model on measured per-entry facts (status quo additionally through real production digests).
- **DEC-MOD-022** — candidates: `any-x-bit-optional-status-quo`, `always-present-on-files`, `absent-means-false-canonical`, `owner-x-bit`, `non-posix-false` (status quo: `any-x-bit-optional-status-quo`). Treatment: Evaluated through the re-projection model on measured per-entry facts (status quo additionally through real production digests).
  - excluded before execution (ledger): `extension-heuristic` — Heuristic derivation makes identity depend on writer choice rather than recorded facts (violates SPEC §8.3 L928 (participation never a writer choice) / I15); L0/R1(b) counterexample and reviewer sign-off still required (R1).
- **DEC-MOD-030** — candidates: `executable-only-status-quo`, `no-metadata`, `nar-like`, `mode-bits`, `mode-and-acls` (status quo: `executable-only-status-quo`). Treatment: Evaluated through the re-projection model on measured per-entry facts (status quo additionally through real production digests).
  - excluded before execution (ledger): `keyword-selectable` — Per-archive writer-chosen participation contradicts registry-determined participation (violates SPEC §8.3 L928 participation depends only on registered name and profile); L0/R1(b) counterexample and reviewer sign-off still required (R1).
- **DEC-MOD-035** — candidates: `status-quo-assignment`, `named-content-streams-in-lai`, `forks-in-aux`, `origin-markers-none`, `origin-markers-aux`, `refuse-unassigned` (status quo: `status-quo-assignment`). Treatment: Evaluated through the re-projection model on measured per-entry facts (status quo additionally through real production digests).
  - excluded before execution (ledger): `unauthenticated-hints` — Leaves asserted archive bytes outside LAI/AUX, allowing silent alteration in signed archives (violates SPEC §8.0 L885 metadata coverage; I13); L0/R1(b) counterexample and reviewer sign-off still required (R1).
- **DEC-MOD-028** — candidates: `spec-set`, `numeric-ids`, `names-and-ids`, `tagged-union-ownership`, `plus-xattrs-acls`, `mtime-second-precision`, `not-in-v1`, `multiple-strict-profiles`. Treatment: Evaluated through the re-projection model on measured per-entry facts (status quo additionally through real production digests).
- **DEC-MOD-025** — candidates: `path-derived-status-quo`, `ordinal-in-canonical-order`, `first-member-path-reference`, `content-plus-first-path`, `no-group-id-structural-set` (status quo: `path-derived-status-quo`). Treatment: Evaluated through the re-projection model on measured per-entry facts (status quo additionally through real production digests).
  - `path-derived-status-quo`: Measured directly (production AUX) and modelled.
- **DEC-MOD-019** — candidates: `participates-status-quo`, `bytes-only-identity`, `canonical-declaration-rule`, `declaration-in-conversion-record` (status quo: `participates-status-quo`). Treatment: Evaluated through the re-projection model on measured per-entry facts (status quo additionally through real production digests).
  - `participates-status-quo`: Measured directly.
- **DEC-CON-010** — candidates: `status-quo-always-set-cumulative`, `minimal-activation`, `independent-composable-bits`, `auxiliary-as-ro-compat-or-compat`, `derived-supported-mask`, `exact-presence-with-documented-implication` (status quo: `status-quo-always-set-cumulative`). Treatment: Evaluated through the re-projection model on measured per-entry facts (status quo additionally through real production digests).
  - `status-quo-always-set-cumulative`: Measured directly.
- **DEC-CRY-008** — candidates: `status-quo-content-dependent-bits`, `always-set-under-encryption`, `private-declaration`, `document-as-public-leak` (status quo: `status-quo-content-dependent-bits`). Treatment: Evaluated through the re-projection model on measured per-entry facts (status quo additionally through real production digests).
  - `status-quo-content-dependent-bits`: Measured directly from encrypted packs.
  - `always-set-under-encryption`: Derived: public bits become constant; exposure count 0 by construction; reader early-failure impact left to the crypto domain.
  - `private-declaration`: Not measurable without a wire prototype; exposure count derived.
  - `document-as-public-leak`: Same measurements as status quo.
- **DEC-PLT-003** — candidates: `status-quo-os-derived-precision`, `detect-fs-type-and-map`, `record-fs-type-only`, `unknown-precision-when-undetected`, `observed-granularity-inference` (status quo: `status-quo-os-derived-precision`). Treatment: Evaluated through the re-projection model on measured per-entry facts (status quo additionally through real production digests).
- **DEC-PLT-022** — candidates: `status-quo-capture-all-restore-per-flag`, `spec-metadata-class-flags`, `profile-based-classes`, `capture-all-restore-selection-only`, `exclude-volatile-classes-by-default` (status quo: `status-quo-capture-all-restore-per-flag`). Treatment: Evaluated through the re-projection model on measured per-entry facts (status quo additionally through real production digests).
- **DEC-PLT-049** — candidates: `status-quo-mtime-plus-platform-birth`, `declare-unavailable-only`, `add-linux-btime-capture-only`, `add-atime-optional`, `add-ctime-capture-only`, `exclude-creation-time-by-default`, `restore-creation-time-and-birthtime` (status quo: `status-quo-mtime-plus-platform-birth`). Treatment: Evaluated through the re-projection model on measured per-entry facts (status quo additionally through real production digests).
- **DEC-PLT-062** — candidates: `status-quo-compiled-untested-plus-unavailable-declarations`, `exclude-blocked-platform-claims`, `experimental-tier-per-platform`, `declare-unavailable-until-tested`, `emulated-correctness-evidence-only`, `documentation-and-third-party-evidence`, `community-contributor-runs`, `acquire-hardware-or-admin-session` (status quo: `status-quo-compiled-untested-plus-unavailable-declarations`). Treatment: Evaluated through the re-projection model on measured per-entry facts (status quo additionally through real production digests).
  - `emulated-correctness-evidence-only`: Directly tested by the validity control.
  - excluded before execution (ledger): `hosted-ci-runners` — The program owner declined hosted runners; platform experiments are local and Docker only (violates PROGRESS standing decision 2026-09-12 (no GitHub Actions or hosted runners)); L0/R1(b) counterexample and reviewer sign-off still required (R1).

## Corpus, fixtures and case sets

- **Trees:** (a) the EXP-DET-SMOKE generator tree (`research/harness/internals_identity_tools.py tree`, continuity with F-0002), generated per seed (`plat005-detsmoke-tuning` 20260917, `plat005-detsmoke-validation` 20260918); (b) tuning/validation corpus items with executables, symlinks, hardlinks and varied timestamps: `f01-tuning-ripgrep-14-1-1-git`, `f02-tuning-lz4-intree-make-build`, `f03-tuning-fzf-go-mod-vendor`, `f04-tuning-sourcelike`, `f17-tuning-duptree-mixed`, `f18-tuning-lz4-releases-1-9-to-1-10`, `f19-tuning-rsnapshot-a`, `f19-tuning-real-home-snapshot`, `f03-validation-yq-go-mod-vendor`, `f04-validation-objstore`, `f17-validation-duptree-vendor`, `f19-validation-rsnapshot-b`, `f19-validation-real-home-snapshot`, `f02-validation-cjson-cmake-build`, `f18-validation-cjson-releases`.
- **Transfers:** Linux filesystems receive `cp -a --sparse=auto` copies (plus the copy-variant arm); Windows receives `robocopy \\wsl.localhost\Ubuntu\root\eb-research\corpus\<split>\<item> D:\eb-research\scratch\platform\EXP-PLAT-005\<item> /E /COPY:DT /DCOPY:DT` copies; for F01/F02/F18 items with a recorded upstream git commit, an additional `git clone` + checkout on both hosts (Windows `core.fileMode=false`, `core.autocrlf=false`, `core.symlinks=false` defaults recorded).
- **Copy-variant arm (AUX churn):** `cp --sparse=always`, `cp --sparse=never`, `cp -r` (no -p), GNU tar round trip, `rsync -aHAX`, robocopy with and without `/COPY:T`, `Copy-Item -Recurse`.
- **Alias-edit arm:** on `f19-tuning-rsnapshot-a` and `f19-validation-rsnapshot-b`, seeded choice of one hardlink group: rename one alias, add one alias, remove one alias.
- **Legacy-declaration arm:** ZIP/tar/7z archives found in `f14-legacy-writer-matrix-tuning`, `f14-legacy-writer-matrix-validation`, `f20-tuning-real-libarchive-testsuite`, `f20-validation-real-cpython-archivetestdata` converted with `ebound convert --strict`, `--compat=<each versioned profile>`, and `--preserve --compat=<profile>`.
- **Encrypted arm:** every tree above packed encrypted on each host.
- **Splits:** tuning and validation only; validation looks are registered in `research/decisions/validation-looks.jsonl` (§5.2, at most two per decision). Generated fixtures are assigned to splits by seed as stated above. No spec selects held-out (Phase D placeholder below).

## Environment and platform requirements

- **Contexts:** `wsl-ext4`, `loop-xfs`, `loop-btrfs`, `tmpfs`, `loop-vfat`, `ntfs3g`, `qemu-aarch64-user`, `qemu-x86_64-user-control`, `windows-ntfs-robocopy`, `windows-ntfs-git-checkout`. `qemu-aarch64-user` runs the aarch64 production binary under `qemu-aarch64-static` on WSL ext4 (qualifier EMULATED; correctness and identity only); `qemu-x86_64-user-control` runs the native x86_64 binary under `qemu-x86_64-static` as the emulation-validity control.
- **env_ids:** `wsl-ubuntu`, `windows-host`. No timing.

| Requirement | Needed? |
|---|---|
| Linux (WSL2, root) | yes |
| Windows 11 host (non-admin) | yes |
| macOS | no (column blocked: EXP-PLAT-022) |
| x86-64 | yes |
| ARM64 | emulated only (QEMU user mode); native in EXP-PLAT-025 (BLOCKED) |
| Network | git clone of recorded upstream commits and pinned toolchain downloads (approved) |
| Privileged | WSL root for loop mounts |
| Large storage | about 80 GB transient on WSL ext4 and D: |
| External review | no (security analyses routed elsewhere) |

- **Storage discipline:** **PCP-6**.

## Commands

- **Production CLI (Linux):** `rm -rf /root/eb-research/src/entrybound-plat && git clone /mnt/d/Projects/entrybound/entrybound /root/eb-research/src/entrybound-plat && git -C /root/eb-research/src/entrybound-plat checkout <record-commit> && cd /root/eb-research/src/entrybound-plat && CARGO_TARGET_DIR=/root/eb-research/target/production /root/.cargo/bin/cargo +1.98.1 build --release -p entrybound-cli --bin ebound` → `/root/eb-research/target/production/release/ebound` (production workspace, no `research-internals`; harness review R1-06).
- **Production CLI (Windows):** `git clone D:/Projects/entrybound/entrybound D:/eb-research/src/entrybound-plat; git -C D:/eb-research/src/entrybound-plat checkout <record-commit>; $env:CARGO_TARGET_DIR='D:/eb-research/target/production-win'; & "$env:USERPROFILE/.cargo/bin/cargo.exe" +1.98.1 build --release -p entrybound-cli --bin ebound` → `D:/eb-research/target/production-win/release/ebound.exe`.
- **Harness (only where named):** `wsl.exe -d Ubuntu -- bash /mnt/d/Projects/entrybound/entrybound/research/harness/build.sh build --release` (→ `/root/eb-research/target/harness/release/`), after `C:/Python313/python.exe research/harness/lock_parity.py` reports `RESULT PASS` (harness review R1-06).
- **Pre-flight:** `C:/Python313/python.exe research/orchestration/disk_guard.py` must pass; no other workflow may hold the loop devices or mount points named here.
- **aarch64 build:** `CARGO_TARGET_DIR=/root/eb-research/target/production-aarch64 CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc /root/.cargo/bin/cargo +1.98.1 build --release -p entrybound-cli --bin ebound --target aarch64-unknown-linux-gnu` (clean clone at the record commit).
- **ebr (WSL):** `wsl.exe -d Ubuntu -u root -- bash -lc 'cd /mnt/d/Projects/entrybound/entrybound && export PYTHONPATH=research/tools && PY=/root/eb-research/venv/bin/python && $PY -m ebr validate --spec research/experiments/EXP-PLAT-005/spec.yaml && $PY -m ebr run --spec research/experiments/EXP-PLAT-005/spec.yaml && $PY -m ebr verify-raw --spec research/experiments/EXP-PLAT-005/spec.yaml --refingerprint && $PY -m ebr normalize --spec research/experiments/EXP-PLAT-005/spec.yaml && $PY -m ebr report --spec research/experiments/EXP-PLAT-005/spec.yaml'`
- **ebr (Windows host, PowerShell 7):** `$env:PYTHONPATH='research/tools'; $env:EB_DATA_ROOT='D:/eb-research'; $env:TEMP='D:/eb-research/tmp'; $env:TMP=$env:TEMP; C:/Python313/python.exe -m ebr run --spec research/experiments/EXP-PLAT-005/spec-windows.yaml` followed by `verify-raw`, `normalize`, `report` with the same spec (for corpus items the Windows runs set `EB_DATA_ROOT=\\wsl.localhost\Ubuntu\root\eb-research` and copy items to D: inside the command).
- **ebr (WSL):** `wsl.exe -d Ubuntu -u root -- bash -lc 'cd /mnt/d/Projects/entrybound/entrybound && export PYTHONPATH=research/tools && PY=/root/eb-research/venv/bin/python && $PY -m ebr validate --spec research/experiments/EXP-PLAT-005/spec-arms.yaml && $PY -m ebr run --spec research/experiments/EXP-PLAT-005/spec-arms.yaml && $PY -m ebr verify-raw --spec research/experiments/EXP-PLAT-005/spec-arms.yaml --refingerprint && $PY -m ebr normalize --spec research/experiments/EXP-PLAT-005/spec-arms.yaml && $PY -m ebr report --spec research/experiments/EXP-PLAT-005/spec-arms.yaml'`
- **Scripts to be written:**
  - `scripts/identity_capture.sh --context <c> --item <path> --item-id <id> --scratch <dir> --detail-dir <dir>` — materializes the tree on the context's filesystem, runs `ebound pack` (balanced INDEXED, balanced STREAM, encrypted INDEXED), `ebound inspect --json`, `ebr-inspect-meta`; prints `lai`, `aux`, `pcr`, `pci` (strings), `incompat_features`, `ro_compat_features`, `compat_features`, `public_required_bits_encrypted`, `entries_executable_true`, `observation_sha256`.
  - `scripts/identity_capture.ps1 -Mode robocopy|git -Item <unc path> -ItemId <id> -Scratch <dir>` — Windows counterpart.
  - `scripts/identity_compare.py --raw research/raw/EXP-PLAT-005 --out research/normalized/EXP-PLAT-005/decision/` — joins contexts per (item, layout), classifies each LAI/AUX difference by field (executable, precision, posix, platform_security, sparse_map, fidelity strings, hardlink group, name declaration), and checks MVT-13(a): every cross-host AUX difference must be explained item by item by FidelityReport entries for classes the host does not capture.
  - `scripts/identity_reproject.py --facts <ebr-inspect-meta jsonl> --candidates candidates.yaml` — computes per candidate the canonical LAI and AUX participation fact sets (SHA-256 over canonical JSON). **Validation gate:** for the status quo, fact-set agreement must equal production digest agreement on every (item, pair); any mismatch means the model misses a participating fact and blocks every modelled result.
  - `scripts/alias_edits.py`, `scripts/legacy_declarations.py`, `scripts/copy_variants.sh` for the arms (spec-arms.yaml).

## Seeds, warmups and replication

- **PCP-3** of `research/experiments/_index/platform-common-provisions.md` applies verbatim.
- **Determinism claims:** where a candidate claims cross-host identity (HC-13 logical determinism), the MVT-13(a) cells use 3 repetitions per (context, item) plus one 20-repetition stress cell per host at the host's maximum thread count with seeded background load (objective MVT-13(a); decision-method Appendix C).

## Measurement method and metrics

- **Direct measurements:** production identities and feature masks per (context, item, layout, encryption); per-entry facts from `ebr-inspect-meta`.
- **Agreement:** `cross_host_identity_agreement` = 1 for an (item, context pair, candidate) iff LAI agrees (and AUX agrees where the candidate claims AUX agreement) after removing only differences the candidate's own rule declares as host-scoped; computed on production digests for the status quo and on validated fact sets for other candidates.
- **Feature tiers:** for each tier candidate, the placement of every active feature bit is derived from the candidate's rule; a reader lacking platform-security-metadata-v1 refuses iff an incompat bit it does not know is set; the share of archives refused is reported per host, and each candidate's critical items are checked to still fail closed (HC-04) using the EXP-PLAT-019 injected-critical cases.
- **AUX churn:** per copy variant and per alias edit, count of entries whose AUX-participating facts change, attributed to the class responsible; diff report bytes from `ebound diff --json`.
- **Public bits:** in encrypted archives, the public required-feature bitmap as read without keys; each content-dependent bit that varies with content is one exposed bit (descriptive).
- **Emulation validity:** identities from `qemu-x86_64-user-control` must equal native x86_64 in every cell; only then are `qemu-aarch64-user` results used (EMULATED qualifier).

| Metric | OD | Role here | ebr family | Unit | Direction | Threshold / oracle reference | Source |
|---|---|---|---|---|---|---|---|
| `cross_host_identity_agreement` | OD-18 | binary R1 screen per candidate and context pair (M18.1) | correctness | binary | equal_one | HC-08 (`metric_thresholds.cross_host_identity_agreement`; A.2 role `binary`) | production digests / validated re-projection |
| `hc13_mvt13a_unexplained_aux_differences` | OD-04 | HC oracle output (objective §2.0: binary; any non-zero value is a committed violation artifact and an R1 matter; not an Appendix A.2 metric, never enters §6 utility) | correctness | count | equal_zero | HC-13 MVT-13(a) | `identity_compare.py` |
| `reader_fragmentation_count` | OD-21 | cost_input (M21.6): features that make Windows-packed archives unreadable by a minimal reader, per tier candidate | other | count | minimize | C5 (`metric_thresholds.reader_fragmentation_count`; A.2 role `cost_input`) | derived from feature masks |
| `hc04_fail_open_outcomes` | OD-04 | HC oracle output (objective §2.0: binary; any non-zero value is a committed violation artifact and an R1 matter; not an Appendix A.2 metric, never enters §6 utility) | correctness | count | equal_zero | HC-04 | EXP-PLAT-019 injected critical cases applied to each tier candidate |
| `metadata_bytes` | OD-05 | diagnostic; primary only for DEC-MOD-049/050 encodings (e.g. an always-present tri-state item on every file) | size | B | minimize | T-02 (`metric_thresholds.metadata_bytes`; A.2 role `diagnostic`) | `ebr-pack` accounting plus wire-formula for modelled candidates |
| `artifact_bytes` | OD-05 | guard (T-01) for candidates that add per-entry items | size | B | minimize | T-01 (`metric_thresholds.artifact_bytes`; A.2 role `banded`) | path size |
| `wire_items_added` | OD-23 | cost_input (M23.2) | other | count | minimize | C1 (`metric_thresholds.wire_items_added`; A.2 role `cost_input`) | candidate census |

## Normalization

Identities are compared as exact strings; differences are attributed to fields by per-entry diff. Byte metrics are exact. Reference: status quo on `wsl-ext4` for Linux pairs and on `windows-ntfs-robocopy` for Windows pairs.

## Statistical analysis

- **PCP-4** of `research/experiments/_index/platform-common-provisions.md` applies verbatim.
- **Screens:** a candidate that leaves any unexplained LAI difference on an in-scope pair fails HC-08 MVT-08(c) and HC-10 for that scope (R1); a candidate that records a fact the platform cannot observe as a definite value is excluded at L0 under objective HC-07/HC-10 ("without inventing facts") with the committed counterexample (for DEC-MOD-049 the status quo `executable=false` on NTFS is exactly such an artifact: F-0002 raw inspect JSON).
- **Survivors** are compared on `metadata_bytes`/`artifact_bytes` guards and `reader_fragmentation_count`, then by computed tier: tri-state executable or exclude-from-LAI are identity participation assignments (T4) and need either the R6 exemption (all lower-tier candidates eliminated at R1) or the k=10 minimum gain; caller policy flags are T2.
- **Scopes (R9):** platform-pair scoped winners are admissible only if the product can express them (a writer knows its capture platform; a reader does not choose the capture host).

## Practical-significance thresholds

Referenced, never restated: each banded metric uses its `research/methods/thresholds.json` `metric_thresholds.<name>` entry (decision-method §3.2 ID in the metrics table; §3.1 semantics; §7.3 minimum-gain caps). Binary metrics and HC oracle outputs have no band (§3.3; objective §2.0 rule 1).

## Sensitivity analysis

- **PCP-5** of `research/experiments/_index/platform-common-provisions.md` applies verbatim.
- **Item-set arm:** leave-one-family-out over the corpus trees (§6.5).
- **Checkout arm:** robocopy copies versus git checkouts reported separately; a selection that flips between them is recorded as scope-limited.

## Held-out evaluation (Phase D placeholder)

**PCP-7**.

## Cost accounting inputs (decision-method §7)

**PCP-8**.

## Expected negative results worth recording

- F-0001 and F-0002 are expected to reproduce on every corpus tree with executables; their reproduction is the status quo's counterevidence.
- Canonical precision cannot make AUX agree when the capturing filesystem truly truncated the instant (vfat 2-second mtime); that residue is a declared-loss case, not a model failure.
- Compat-tier candidates may still fail a minimal reader if other incompat bits (0x8000 POSIX/symlinks) are content-dependent on Linux; the census reports that as a limit of any tier-only fix.
- Encrypted archives may expose 0x8000/0x10000 as content-dependent public bits (DEC-CRY-008).

## Threats to validity

- The re-projection model is a derivation, not an implementation; its validation covers only the status quo's participation rules. Candidate results carry `[FORMALLY_DERIVED; model]` and need production prototypes before any T4 selection becomes `DECIDED`.
- robocopy and git checkouts are two of many ways trees reach Windows; others (WSL cp to /mnt/d, zip extraction) are not covered here.
- QEMU user mode emulates the CPU, not the kernel: filesystem behaviour is the host kernel's, so the aarch64 arm tests only architecture-dependent code paths.
- ntfs-3g on Linux is not the Windows capture path; Windows-native captures run on D:.

## Estimated cost and effort

- **Compute:** about 24 machine-hours; **disk:** about 80 GB peak (transient scratch unless stated).
- **Timing required:** no.
- **Tooling to build:** research/experiments/EXP-PLAT-005/scripts/identity_reproject.py: candidate participation models over `ebr-inspect-meta` facts, validated against production LAI/AUX agreement before use; research/experiments/EXP-PLAT-005/scripts/{identity_capture.sh,identity_capture.ps1,materialize_windows_tree.ps1,identity_compare.py,alias_edits.py,legacy_declarations.py,copy_variants.sh}; aarch64 cross build of production ebound: `rustup +1.98.1 target add aarch64-unknown-linux-gnu`, apt `gcc-aarch64-linux-gnu qemu-user-static` (approved downloads); encrypted arm through `ebr-pack --encrypt` (research build, byte-identical to production per internals identity check) unless a non-interactive production recipient path is confirmed; `ebr-inspect-meta` from EXP-PLAT-004
- **Agent effort:** execution none (scripted); tooling: re-projection model needs one judgment review of each candidate's participation rules against its ledger description; analysis judgment.

## Deviations (append-only; UTC time, author, reason, results-visible yes/no)

_None._

## Outcome (after analysis)

- classification (`outcome.json`, §10.1), verdict table reference, confirmation-bias triggers evaluated (§10.2): _pending_
