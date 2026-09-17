# EXP-PLAT-013: Windows NTFS reparse objects, attributes, symlink extraction and pack refusal behaviour (non-admin)

> **Design pre-registration draft** (Phase C-design, platform domain). Index status: **READY**. Structure: decision-method §12.2 record fields plus the Phase C-design protocol fields. Shared provisions PCP-1…PCP-8 live in `research/experiments/_index/platform-common-provisions.md` and are part of this pre-registration. This file becomes the §12 `record.md` only after the gates are met.

## Pre-registration

- **decision_ids:** DEC-PLT-056, DEC-PLT-058, DEC-PLT-030, DEC-PLT-038, DEC-PLT-055, DEC-PLT-008, DEC-PLT-033, DEC-MOD-044, DEC-PLT-042, DEC-PLT-057, DEC-LEG-055, DEC-LEG-091
- **Program sections:** 15, 17, 18, 21
- **Decision type (proposed, not assigned):** EMPIRICAL (behaviour, settability, refusal frequency); threat analysis per tag is analytical input. Assignment is made by a session not involved in the candidates (decision-method §1.2).
- **od_ids / hc_ids (proposed for the schema-v2 assignment, §14 item 12):** OD-03 (M03.1, M03.2), OD-18, OD-25 (M25.4); HC-07, HC-05, HC-01 (reparse authority bits), HC-17.
- **Method:** `research/decision-method.md` and `research/methods/thresholds.json` at pre-registration commit `14b977c` (status `pre-registered`).
- **Pre-registration commit:** the checkpoint commit adding this file (design draft); the §12 record commit follows G-A; every run's raw `meta` git SHA must descend from it with `dirty = false` (§5.8 d).
- **Author, independence and exposure:** Phase C-design platform-domain designer (workflow `wf_47347bc4-611`); no candidate was constructed by this session (ledger R0 sets reproduced mechanically). Held-out identity exposure (event L7) is disclosed in **PCP-1**.
- **Public data sources used for tuning (§5.4 consequence 2):** upstream sources in `research/corpus/manifest.json` `provenance` for the tuning/validation items named below, plus the pinned tool downloads named under Commands.
- **Gates:** G-A to G-D as in **PCP-2**.

## Question and hypotheses

- **Question:** On the Windows host without elevation, which reparse objects (junctions, AF_UNIX sockets, WSL LX symlinks, AppExecLinks), attribute bits, compression, hardlinks and security descriptors can be created, observed and restored, what does status-quo `ebound pack`/`unpack` do with each (abort, skip, capture, report), how often do real developer trees contain them, and how do Windows-native tools behave?
- **H1:** Junctions, AF_UNIX sockets created through WSL and LX symlinks are creatable non-admin and each aborts status-quo pack (InvalidReparsePoint); package-manager developer trees (pnpm, npm link) contain junctions, so abort-on-reparse refuses a measurable share of them; symlink extraction without privilege fails and aborts the whole extraction under the status quo; READONLY, HIDDEN, SYSTEM, ARCHIVE, TEMPORARY and NOT_CONTENT_INDEXED are settable non-admin while PINNED/UNPINNED are not on a non-cloud directory; status quo captures lone NTFS hardlinks as independent files without declaration.
- **H0 / equivalence:** No reparse object is creatable non-admin; developer trees contain none; symlink extraction failures are per-entry.
- **Falsification of H1:** Status-quo pack accepts junctions, or no developer tree contains a reparse object, or symlink extraction failure is per-entry with a report.

## Decisions informed

| Decision | Release relevance | Ledger question (abridged; full text in the ledger) | What this experiment contributes | Evidence still needed elsewhere |
|---|---|---|---|---|
| DEC-PLT-056 | V1_BLOCKING | When packing on Windows, should reparse sources (symlinks, junctions, cloud placeholders, dedup stubs, AppExecLinks) abort the whole pack (status quo), be skipped with... | Refusal frequency of abort-pack on real developer trees materialized on D: (pnpm/npm workspaces, Python venvs, git checkouts, WSL-created trees); per-tag pack behaviour. | OneDrive and profile scans (owner consent, EXP-PLAT-023); feasibility of exact capture (EXP-PLAT-016). |
| DEC-PLT-058 | V1_IMPORTANT | On Windows extraction, how are symlink type (file versus directory), missing SeCreateSymbolicLinkPrivilege, dangling or forward targets, non-UTF-8 (POSIX_BYTES) target... | Extraction of file, directory, dangling, chained and non-UTF-8-target symlinks without SeCreateSymbolicLinkPrivilege by ebound, tar.exe, 7-Zip and Python; junction-fallback simulation for in-root directory targets. | Developer Mode on and privileged cases (EXP-PLAT-023). |
| DEC-PLT-030 | V1_BLOCKING | Can exact opaque reparse data be captured (FSCTL_GET_REPARSE_POINT with FILE_OPEN_REPARSE_POINT) and restored (FSCTL_SET_REPARSE_POINT on an empty object) without foll... | Which tag classes can be created and queried non-admin; exact payload bytes via `fsutil reparsepoint query` as the observer for prototype tests. | Capture/restore prototypes and unsafe audit (EXP-PLAT-016); cloud/dedup/symlink tags (EXP-PLAT-023). |
| DEC-PLT-038 | V1_IMPORTANT | Which reparse tags are 'recognized' (mapped to Symlink entries) versus opaque ReparsePoints, is a recognized tag encoded as ReparsePoint noncanonical, does tag-2 data... | Real payload layouts of junction, AF_UNIX, LX symlink and AppExecLink tags on this host. | Tag enumeration from primary documentation (EXP-PLAT-021); acceptance tests of symlink-tagged ReparsePoint entries (EXP-PLAT-019); third-party filter tags (EXP-PLAT-023). |
| DEC-PLT-055 | V1_IMPORTANT | Which accepted windows.file-attributes bits and windows.creation-time are restored under platform-metadata Restore, ignored or reported, including ENCRYPTED/COMPRESSED... | Per-bit settability and status-quo restore; prototype restore through std and `SetFileInformationByHandle` (EXP-PLAT-016 probe); creation time via std `FileTimes`; READONLY ordering. | exFAT/ReFS/OneDrive (EXP-PLAT-023); EFS raw APIs post-v1 (owner consent for EFS certificate creation). |
| DEC-PLT-008 | V1_IMPORTANT | How are transparently compressed files (macOS decmpfs with UF_COMPRESSED, NTFS compression) captured and restored: decompressed content with the attribute dropped and... | NTFS compressed-file capture: content, attribute, restore declaration. | macOS decmpfs (EXP-PLAT-022). |
| DEC-PLT-033 | V1_IMPORTANT | How should capture treat a multiply-linked file whose other aliases lie outside the packed root (silent ordinary File, entry-scoped declared loss, or refusal), and sho... | Status-quo capture of NTFS hardlink pairs and lone aliases; declaration check. | Safe file-ID API (EXP-PLAT-016). |
| DEC-MOD-044 | V1_IMPORTANT | Can ReparsePoint entries have descendant Entries or carry content (directory junctions, cloud placeholders, dedup files), and is Entry-v3 tag 9 forbidden on non-Repars... | Semantics of junctions with children, mount-point junctions and data-bearing reparse objects observable on this host. | Cloud/dedup data-bearing tags (EXP-PLAT-023); EntrySet validation tests (EXP-PLAT-019). |
| DEC-PLT-042 | V1_IMPORTANT | Must the CLI restrict the ACLs of generated secret key and identity files on Windows (and check them on load), and through which safe API? | Default inherited ACLs on D:\eb-research and (read-only observation) on user-profile folders; principals able to read a newly written key file. | Safe DACL APIs (EXP-PLAT-016); key-load behaviour (EXP-PLAT-020). |
| DEC-PLT-057 | V1_IMPORTANT | Is Windows security descriptor (owner, group, DACL, SACL) capture and restoration required for v1, and if so through which path under unsafe_code=forbid? | Observed owner/group/DACL on D: files, robocopy `/SEC` and 7-Zip `-sni` behaviour without elevation. | SACL (EXP-PLAT-023); descriptor APIs (EXP-PLAT-016); security review of restoring attacker-supplied descriptors (external). |
| DEC-LEG-055 | V1_IMPORTANT | Must the 7-Zip FILE_ATTRIBUTE_UNIX_EXTENSION flag (0x8000) gate interpretation of high-word POSIX mode bits in 7z attributes? | 7-Zip for Windows archives of files with each attribute bit: attribute high word and 0x8000 flag as written. | Import interpretation (legacy domain). |
| DEC-LEG-091 | POST_V1 | Should a new ZIP profile write Info-ZIP Unix extras (0x7875 ids), NTFS times (0x000a), symlinks via Unix mode, xattr/ACL extras, or keep declaring these losses? | Windows-native extractors (Explorer shell, tar.exe, Expand-Archive, 7-Zip) and their use of ZIP extras 0x000a, 0x5455, 0x7875 on extraction. | Info-ZIP, Python, Java and macOS Archive Utility rows (legacy domain / EXP-PLAT-022). |

## Candidates

Candidate sets are the ledger's R0 sets; completeness (status quo, SPEC design, named alternatives, strongest incumbent, defer, critic candidate, corrective candidate) is checked by the R0 completeness critic, not by this design. Where a set lacks a strongest-incumbent reference, the incumbent systems measured below act as REF-INC for the metric (objective §3.0).

- **Reference candidate:** Status quo production `ebound.exe` (`abort-pack-status-quo`, `file-symlink-always-status-quo`, `refuse-status-quo`, `undefined-status-quo`, `status-quo-report-unavailable`, `status-quo-decompressed-read-flag-recorded`, `status-quo-silent`, `leaf-no-content-status-quo`, `status-quo-no-op-on-windows`, `status-quo-declared-unavailable`, `status-quo-ungated`, `status-quo-extended-timestamp-only`).
- **W and S:** determined on tuning (R3/R4) and declared in the §12 record before any validation look (§4.12).

Descriptions of every candidate are in the ledger row (`candidate_set`); they are not repeated here.

- **DEC-PLT-056** — candidates: `abort-pack-status-quo`, `skip-with-declared-loss`, `capture-symlinks-refuse-others`, `capture-all-exact`, `hydrate-or-follow-option` (status quo: `abort-pack-status-quo`). Treatment: Status quo measured directly; alternatives scored by feasibility on this host and by the benign cost measured here (`[derived]` where no implementation exists).
  - excluded before execution (ledger): `coerce-to-file-or-directory` — Repo docs forbid coercing unknown reparse objects to File or Directory (violates docs/security-metadata-v1.md L33-34 (repo freeze)); L0/R1(b) counterexample and reviewer sign-off still required (R1).
- **DEC-PLT-058** — candidates: `file-symlink-always-status-quo`, `infer-type-from-archive-tree`, `record-type-at-capture`, `junction-fallback-for-directories`, `skip-with-report-no-privilege`, `copy-target-content-fallback` (status quo: `file-symlink-always-status-quo`). Treatment: Status quo measured directly; alternatives scored by feasibility on this host and by the benign cost measured here (`[derived]` where no implementation exists).
  - `junction-fallback-for-directories`: Simulated with `mklink /J` for in-root directory targets; resolution equivalence checked with `Resolve-Path` and file IDs.
- **DEC-PLT-030** — candidates: `refuse-status-quo`, `windows-rs-safe-wrappers`, `audited-unsafe-deviceiocontrol`, `symlink-junction-only-via-std`, `capture-only-no-restore`, `external-helper-process` (status quo: `refuse-status-quo`). Treatment: Status quo measured directly; alternatives scored by feasibility on this host and by the benign cost measured here (`[derived]` where no implementation exists).
- **DEC-PLT-038** — candidates: `undefined-status-quo`, `symlink-only-recognized`, `symlink-and-junction-recognized`, `registry-of-tags`, `remove-known-safe-option`, `guid-included-in-data`, `guid-excluded-separate-field` (status quo: `undefined-status-quo`). Treatment: Status quo measured directly; alternatives scored by feasibility on this host and by the benign cost measured here (`[derived]` where no implementation exists).
- **DEC-PLT-055** — candidates: `status-quo-report-unavailable`, `restore-basic-bits-and-creation-time`, `restore-compression-via-fsctl`, `never-restore-cloud-bits`, `efs-raw-apis-post-v1` (status quo: `status-quo-report-unavailable`). Treatment: Status quo measured directly; alternatives scored by feasibility on this host and by the benign cost measured here (`[derived]` where no implementation exists).
- **DEC-PLT-008** — candidates: `status-quo-decompressed-read-flag-recorded`, `opaque-capture-only-blob`, `drop-with-declared-loss`, `recompress-on-restore` (status quo: `status-quo-decompressed-read-flag-recorded`). Treatment: Status quo measured directly; alternatives scored by feasibility on this host and by the benign cost measured here (`[derived]` where no implementation exists).
  - excluded before execution (ledger): `synthesize-decmpfs` — SPEC forbids synthesising the undocumented macOS compression attribute. (violates SPEC §10.7 L1296); L0/R1(b) counterexample and reviewer sign-off still required (R1).
- **DEC-PLT-033** — candidates: `status-quo-silent`, `declare-partial-link-loss`, `refuse-pack-policy`, `windows-file-id-detection`, `windows-links-declared-unavailable` (status quo: `status-quo-silent`). Treatment: Status quo measured directly; alternatives scored by feasibility on this host and by the benign cost measured here (`[derived]` where no implementation exists).
- **DEC-MOD-044** — candidates: `leaf-no-content-status-quo`, `directory-reparse-with-children`, `data-bearing-reparse`, `map-to-directory-plus-metadata` (status quo: `leaf-no-content-status-quo`). Treatment: Status quo measured directly; alternatives scored by feasibility on this host and by the benign cost measured here (`[derived]` where no implementation exists).
- **DEC-PLT-042** — candidates: `status-quo-no-op-on-windows`, `owner-only-dacl-safe-wrapper`, `warn-on-inherited-access`, `helper-process-icacls`, `document-risk-only` (status quo: `status-quo-no-op-on-windows`). Treatment: Status quo measured directly; alternatives scored by feasibility on this host and by the benign cost measured here (`[derived]` where no implementation exists).
- **DEC-PLT-057** — candidates: `status-quo-declared-unavailable`, `audited-safe-wrapper-crate`, `isolated-ffi-crate`, `backup-read-stream`, `helper-process` (status quo: `status-quo-declared-unavailable`). Treatment: Status quo measured directly; alternatives scored by feasibility on this host and by the benign cost measured here (`[derived]` where no implementation exists).
  - excluded before execution (ledger): `sddl-text-capture` — SDDL does not round-trip conditional ACEs and resource attributes; binary self-relative form is frozen. (violates SPEC §10.6 L1290; docs/security-metadata-v1.md type 42); L0/R1(b) counterexample and reviewer sign-off still required (R1).
  - excluded before execution (ledger): `lossy-acl-view` — Repo explicitly forbids substituting a lossy ACL view for unavailable descriptor capture. (violates docs/platform-fidelity-v1.md L27-28); L0/R1(b) counterexample and reviewer sign-off still required (R1).
- **DEC-LEG-055** — candidates: `status-quo-ungated`, `gate-on-0x8000`, `record-both-refuse-conflict` (status quo: `status-quo-ungated`). Treatment: Status quo measured directly; alternatives scored by feasibility on this host and by the benign cost measured here (`[derived]` where no implementation exists).
- **DEC-LEG-091** — candidates: `status-quo-extended-timestamp-only`, `add-unix-ids-and-symlinks`, `add-ntfs-times`, `add-xattr-extras`, `declare-loss-only` (status quo: `status-quo-extended-timestamp-only`). Treatment: Status quo measured directly; alternatives scored by feasibility on this host and by the benign cost measured here (`[derived]` where no implementation exists).

## Corpus, fixtures and case sets

- **Fixtures (per seed; tuning 20260917 `plat013-fixture-tuning`, validation 20260918 `plat013-fixture-validation`):** junctions to in-root and out-of-root directories and to a volume GUID path; AF_UNIX socket files and LX symlinks created by WSL on `/mnt/d/eb-research/scratch/platform/EXP-PLAT-013/` (drvfs); read-only observation of AppExecLinks under `%LOCALAPPDATA%\Microsoft\WindowsApps` (no writes on C:); files with each settable attribute bit; `compact /c` compressed files; hardlink pairs and lone aliases; archives (created on WSL) with file, directory, dangling, chained and non-UTF-8-target symlinks.
- **Developer trees (dev-tree census):** materialized on D: from pinned manifests: a pnpm workspace (junction-based `node_modules`), an npm project with `npm link`, a Python venv, a git checkout of a repository containing symlinks (`core.symlinks=false` default), and a WSL-created tree on `/mnt/d` with symlinks. Personal folders are never scanned without owner consent.
- **Legacy fixtures:** 7-Zip `a` archives of attribute-bit files; ZIPs with 0x000a/0x5455/0x7875 extras (Python-written) extracted by Explorer shell, tar.exe, Expand-Archive and 7-Zip.
- **Splits:** tuning and validation only; validation looks are registered in `research/decisions/validation-looks.jsonl` (§5.2, at most two per decision). Generated fixtures are assigned to splits by seed as stated above. No spec selects held-out (Phase D placeholder below).

## Environment and platform requirements

- **env_id:** `windows-host` (non-admin; SeCreateSymbolicLinkPrivilege absent and Developer Mode off per EXP-PLAT-001), with WSL as a helper for drvfs-created reparse objects. `TEMP`/`TMP` on D:. No timing.

| Requirement | Needed? |
|---|---|
| Windows 11 host (non-admin, NTFS D:) | yes |
| Linux (WSL2 helper) | yes |
| macOS | no |
| x86-64 | yes |
| ARM64 | no |
| Network | pinned Node.js/pnpm and package manifests for dev trees (approved downloads) |
| Privileged | no (elevated cases in EXP-PLAT-023) |
| Large storage | no (about 20 GB on D:) |
| External review | no |

- **Storage discipline:** **PCP-6**.

## Commands

- **Production CLI (Windows):** `git clone D:/Projects/entrybound/entrybound D:/eb-research/src/entrybound-plat; git -C D:/eb-research/src/entrybound-plat checkout <record-commit>; $env:CARGO_TARGET_DIR='D:/eb-research/target/production-win'; & "$env:USERPROFILE/.cargo/bin/cargo.exe" +1.98.1 build --release -p entrybound-cli --bin ebound` → `D:/eb-research/target/production-win/release/ebound.exe`.
- **Pre-flight:** `C:/Python313/python.exe research/orchestration/disk_guard.py` must pass; no other workflow may hold the loop devices or mount points named here.
- **ebr (Windows host, PowerShell 7):** `$env:PYTHONPATH='research/tools'; $env:EB_DATA_ROOT='D:/eb-research'; $env:TEMP='D:/eb-research/tmp'; $env:TMP=$env:TEMP; C:/Python313/python.exe -m ebr run --spec research/experiments/EXP-PLAT-013/spec-windows.yaml` followed by `verify-raw`, `normalize`, `report` with the same spec (for corpus items the Windows runs set `EB_DATA_ROOT=\\wsl.localhost\Ubuntu\root\eb-research` and copy items to D: inside the command).
- **Scripts:** `scripts/windows_arms.ps1 -Arm <a> -FixtureFrom <item_id> -Scratch <dir> -DetailDir <dir>` dispatches to `reparse_observe.ps1` (`fsutil reparsepoint query`, tag and payload hex), `attr_perbit.ps1` (`[IO.File]::SetAttributes`, `attrib`, readback; `ebound.exe pack` and `inspect --json`; `ebound.exe unpack --platform-metadata restore`; prototype restore via `ebr-platform-probe set-attributes`), `devtree_materialize.ps1` (pinned `pnpm install --frozen-lockfile`, `npm link`, `python -m venv`, `git clone`), and `wsl_reparse_helper.sh` (`python3 -c 'import socket; socket.socket(socket.AF_UNIX).bind(p)'`, `ln -s` on drvfs). Every arm prints one JSON line.

## Seeds, warmups and replication

- **PCP-3** of `research/experiments/_index/platform-common-provisions.md` applies verbatim.

## Measurement method and metrics

- **Creatability and observation** per object class.
- **Pack behaviour:** abort (reason code), skip with report, capture as Symlink/ReparsePoint.
- **Refusal frequency:** fraction of dev trees whose pack aborts; entries affected.
- **Symlink extraction:** per link, created (type), junction fallback, skipped with report, or whole extraction aborted.
- **Attributes:** per bit: settable, captured, restored by status quo, restored by prototype.
- **Hardlinks:** captured as group, as independent files with declaration, or without declaration.
- **Descriptors:** principals granted read on new files per directory.

| Metric | OD | Role here | ebr family | Unit | Direction | Threshold / oracle reference | Source |
|---|---|---|---|---|---|---|---|
| `capture_fidelity_fraction` | OD-03 | primary per object class and dev tree (pack aborts count as 0) | other | fraction | maximize | T-20 (`metric_thresholds.capture_fidelity_fraction`; A.2 role `banded`) | script |
| `restore_fidelity_fraction` | OD-03 | primary per attribute bit, link class and policy | other | fraction | maximize | T-20 (`metric_thresholds.restore_fidelity_fraction`; A.2 role `banded`) | script |
| `hc07_mvt07a_undeclared_differences` | OD-03 | HC oracle output (objective §2.0: binary; any non-zero value is a committed violation artifact and an R1 matter; not an Appendix A.2 metric, never enters §6 utility) | correctness | count | equal_zero | HC-07 MVT-07(a) | script |
| `unsafe_defaults` | OD-25 | binary screen (for example a default that follows a junction during extraction) | correctness | count | equal_zero | HC-05 (`metric_thresholds.unsafe_defaults`; A.2 role `binary`) | script |
| `platform_coverage_count` | OD-18 | descriptive | other | count | report | - (`metric_thresholds.platform_coverage_count`; A.2 role `descriptive`) | analysis |

## Normalization

Outcome vocabulary per arm; bits by name; tags by hex value. Reference: status quo.

## Statistical analysis

- **PCP-4** of `research/experiments/_index/platform-common-provisions.md` applies verbatim.

## Practical-significance thresholds

Referenced, never restated: each banded metric uses its `research/methods/thresholds.json` `metric_thresholds.<name>` entry (decision-method §3.2 ID in the metrics table; §3.1 semantics; §7.3 minimum-gain caps). Binary metrics and HC oracle outputs have no band (§3.3; objective §2.0 rule 1).

## Sensitivity analysis

- **PCP-5** of `research/experiments/_index/platform-common-provisions.md` applies verbatim.
- **Dev-tree arm:** leave-one-ecosystem-out for refusal frequency.

## Held-out evaluation (Phase D placeholder)

**PCP-7**.

## Cost accounting inputs (decision-method §7)

**PCP-8**.

## Expected negative results worth recording

- Status quo abort on any reparse source is expected to refuse pnpm-style trees wholesale; a measured capture loss.
- Symlink extraction without privilege is expected to abort the whole extraction; that is a defect relative to per-entry reporting candidates.
- AppExecLinks can be observed but not safely recreated; any capture-all-exact candidate faces restore-time hazards recorded for the threat analysis.

## Threats to validity

- Non-admin session hides symlink creation and SACL behaviour.
- AF_UNIX and LX symlink objects are created by WSL's drvfs, not by native Windows APIs; tag layouts are still native NTFS reparse data.
- Dev-tree composition is a small pinned sample, not a survey of Windows users.

## Estimated cost and effort

- **Compute:** about 6 machine-hours; **disk:** about 20 GB peak (transient scratch unless stated).
- **Timing required:** no.
- **Tooling to build:** research/experiments/EXP-PLAT-013/scripts/{windows_arms.ps1,reparse_observe.ps1,attr_perbit.ps1,devtree_materialize.ps1,wsl_reparse_helper.sh} (mechanical); pinned downloads: Node.js LTS portable and pnpm (dev-tree arm), 7-Zip for Windows portable; per-bit prototype restore and file-ID capture come from EXP-PLAT-016 (`ebr-platform-probe`); archives with ReparsePoint entries from the EXP-PLAT-019 research encoder
- **Agent effort:** execution none (scripted); tooling scripted; judgment for reparse tag threat notes and for the dev-tree census interpretation.

## Deviations (append-only; UTC time, author, reason, results-visible yes/no)

_None._

## Outcome (after analysis)

- classification (`outcome.json`, §10.1), verdict table reference, confirmation-bias triggers evaluated (§10.2): _pending_
