# EXP-PLAT-023: Windows elevated-session, ReFS, exFAT, 8.3, VSS, symlink-privilege and cloud-placeholder suite (BLOCKED)

> **Design pre-registration draft** (Phase C-design, platform domain). Index status: **BLOCKED** — Requires an elevated Windows session (SeSecurityPrivilege for SACL capture, VHD/VHDX creation and mount for ReFS/exFAT/8.3-enabled NTFS volumes, VSS snapshot creation, Developer Mode or SeCreateSymbolicLinkPrivilege for symlink creation — agents never change security settings), a ReFS or Dev Drive volume, an OneDrive-synced folder with explicit owner consent, and Windows Server Data Deduplication for dedup reparse tags; this session is non-admin. Structure: decision-method §12.2 record fields plus the Phase C-design protocol fields. Shared provisions PCP-1…PCP-8 live in `research/experiments/_index/platform-common-provisions.md` and are part of this pre-registration. This file becomes the §12 `record.md` only after the gates are met.

## Pre-registration

- **decision_ids:** DEC-PLT-057, DEC-PLT-044, DEC-PLT-058, DEC-PLT-048, DEC-PLT-028, DEC-PLT-035, DEC-PLT-030, DEC-PLT-038, DEC-PLT-055, DEC-PLT-037, DEC-PLT-018, DEC-PLT-056, DEC-CMP-028, DEC-PLT-062
- **Program sections:** 15, 16, 19, 21
- **Decision type (proposed, not assigned):** EMPIRICAL. Assignment is made by a session not involved in the candidates (decision-method §1.2).
- **od_ids / hc_ids (proposed for the schema-v2 assignment, §14 item 12):** OD-03, OD-18, OD-25; HC-05, HC-07, HC-08, HC-18.
- **Method:** `research/decision-method.md` and `research/methods/thresholds.json` at pre-registration commit `14b977c` (status `pre-registered`).
- **Pre-registration commit:** the checkpoint commit adding this file (design draft); the §12 record commit follows G-A; every run's raw `meta` git SHA must descend from it with `dirty = false` (§5.8 d).
- **Author, independence and exposure:** Phase C-design platform-domain designer (workflow `wf_47347bc4-611`); no candidate was constructed by this session (ledger R0 sets reproduced mechanically). Held-out identity exposure (event L7) is disclosed in **PCP-1**.
- **Public data sources used for tuning (§5.4 consequence 2):** upstream sources in `research/corpus/manifest.json` `provenance` for the tuning/validation items named below, plus the pinned tool downloads named under Commands.
- **Gates:** G-A to G-D as in **PCP-2**.

## Question and hypotheses

- **Question:** With elevation and the missing volume types, what do SACL capture/restore, VSS snapshots, symlink creation (file versus directory type), 8.3 short-name collisions, per-directory case sensitivity, ReFS and exFAT path/metadata behaviour, cloud placeholder and dedup reparse tags, and OneDrive side effects look like for Entrybound and Windows incumbents?
- **H1:** SACL capture requires SeSecurityPrivilege and silently yields an owner/DACL-only descriptor otherwise; 8.3 aliases collide with long names on 8.3-enabled volumes; symlink type must be chosen from the target kind; exFAT lacks hardlinks and ADS; ReFS lacks short names and some attributes; OneDrive placeholders hydrate on read.
- **H0 / equivalence:** Elevation and volume type do not change any measured outcome relative to the non-admin NTFS D: results.
- **Falsification of H1:** No outcome differs from EXP-PLAT-002/004/013/017/018 non-admin results.

## Decisions informed

| Decision | Release relevance | Ledger question (abridged; full text in the ledger) | What this experiment contributes | Evidence still needed elsewhere |
|---|---|---|---|---|
| DEC-PLT-057 | V1_IMPORTANT | Is Windows security descriptor (owner, group, DACL, SACL) capture and restoration required for v1, and if so through which path under unsafe_code=forbid? | Elevated/ReFS/exFAT/8.3/VSS/placeholder column of this decision's evidence (designed, not runnable here). | Non-admin NTFS evidence is in EXP-PLAT-002/004/013/016/017/018. |
| DEC-PLT-044 | POST_V1 | Which platform snapshot mechanisms (VSS, APFS, btrfs, ZFS, LVM) does pack --snapshot support in v1, and how is snapshot-consistent versus best-effort capture recorded? | Elevated/ReFS/exFAT/8.3/VSS/placeholder column of this decision's evidence (designed, not runnable here). | Non-admin NTFS evidence is in EXP-PLAT-002/004/013/016/017/018. |
| DEC-PLT-058 | V1_IMPORTANT | On Windows extraction, how are symlink type (file versus directory), missing SeCreateSymbolicLinkPrivilege, dangling or forward targets, non-UTF-8 (POSIX_BYTES) target... | Elevated/ReFS/exFAT/8.3/VSS/placeholder column of this decision's evidence (designed, not runnable here). | Non-admin NTFS evidence is in EXP-PLAT-002/004/013/016/017/018. |
| DEC-PLT-048 | V1_BLOCKING | Which collision classes must extraction detect (case-fold, NFC/NFD normalization, Windows reserved aliases, trailing dot/space, 8.3 short names, confusables), against... | Elevated/ReFS/exFAT/8.3/VSS/placeholder column of this decision's evidence (designed, not runnable here). | Non-admin NTFS evidence is in EXP-PLAT-002/004/013/016/017/018. |
| DEC-PLT-028 | V1_BLOCKING | What exact rules map LogicalPath components to native names on each target (ext4, XFS, btrfs, APFS, NTFS, ReFS), and which components are refused or renamed: invalid U... | Elevated/ReFS/exFAT/8.3/VSS/placeholder column of this decision's evidence (designed, not runnable here). | Non-admin NTFS evidence is in EXP-PLAT-002/004/013/016/017/018. |
| DEC-PLT-035 | V1_BLOCKING | Which metadata classes does stable v1 guarantee to capture and restore on each supported OS/filesystem (Linux ext4/XFS/btrfs/tmpfs, Windows NTFS/exFAT/ReFS, macOS APFS... | Elevated/ReFS/exFAT/8.3/VSS/placeholder column of this decision's evidence (designed, not runnable here). | Non-admin NTFS evidence is in EXP-PLAT-002/004/013/016/017/018. |
| DEC-PLT-030 | V1_BLOCKING | Can exact opaque reparse data be captured (FSCTL_GET_REPARSE_POINT with FILE_OPEN_REPARSE_POINT) and restored (FSCTL_SET_REPARSE_POINT on an empty object) without foll... | Elevated/ReFS/exFAT/8.3/VSS/placeholder column of this decision's evidence (designed, not runnable here). | Non-admin NTFS evidence is in EXP-PLAT-002/004/013/016/017/018. |
| DEC-PLT-038 | V1_IMPORTANT | Which reparse tags are 'recognized' (mapped to Symlink entries) versus opaque ReparsePoints, is a recognized tag encoded as ReparsePoint noncanonical, does tag-2 data... | Elevated/ReFS/exFAT/8.3/VSS/placeholder column of this decision's evidence (designed, not runnable here). | Non-admin NTFS evidence is in EXP-PLAT-002/004/013/016/017/018. |
| DEC-PLT-055 | V1_IMPORTANT | Which accepted windows.file-attributes bits and windows.creation-time are restored under platform-metadata Restore, ignored or reported, including ENCRYPTED/COMPRESSED... | Elevated/ReFS/exFAT/8.3/VSS/placeholder column of this decision's evidence (designed, not runnable here). | Non-admin NTFS evidence is in EXP-PLAT-002/004/013/016/017/018. |
| DEC-PLT-037 | V1_IMPORTANT | What commit strategy applies where hard links are unavailable (FAT32/exFAT, some SMB/NFS shares, cloud-sync folders): refuse with a clear reason, no-replace rename, ex... | Elevated/ReFS/exFAT/8.3/VSS/placeholder column of this decision's evidence (designed, not runnable here). | Non-admin NTFS evidence is in EXP-PLAT-002/004/013/016/017/018. |
| DEC-PLT-018 | V1_IMPORTANT | When a hardlink alias cannot be created (FAT/exFAT, some SMB/FUSE/cloud filesystems, cross-device), should extraction abort (status quo), materialize an independent ve... | Elevated/ReFS/exFAT/8.3/VSS/placeholder column of this decision's evidence (designed, not runnable here). | Non-admin NTFS evidence is in EXP-PLAT-002/004/013/016/017/018. |
| DEC-PLT-056 | V1_BLOCKING | When packing on Windows, should reparse sources (symlinks, junctions, cloud placeholders, dedup stubs, AppExecLinks) abort the whole pack (status quo), be skipped with... | Elevated/ReFS/exFAT/8.3/VSS/placeholder column of this decision's evidence (designed, not runnable here). | Non-admin NTFS evidence is in EXP-PLAT-002/004/013/016/017/018. |
| DEC-CMP-028 | V1_IMPORTANT | How should pack handle sources that change during capture: retry count (default 2), stability predicate (Unix full stat set versus Windows length and mtime only), and... | Elevated/ReFS/exFAT/8.3/VSS/placeholder column of this decision's evidence (designed, not runnable here). | Non-admin NTFS evidence is in EXP-PLAT-002/004/013/016/017/018. |
| DEC-PLT-062 | V1_BLOCKING | How does stable v1 disposition capabilities that cannot be evaluated locally (macOS/APFS metadata, native ARM64 performance, ReFS, admin-only Windows operations): keep... | Elevated/ReFS/exFAT/8.3/VSS/placeholder column of this decision's evidence (designed, not runnable here). | Non-admin NTFS evidence is in EXP-PLAT-002/004/013/016/017/018. |

## Candidates

Candidate sets are the ledger's R0 sets; completeness (status quo, SPEC design, named alternatives, strongest incumbent, defer, critic candidate, corrective candidate) is checked by the R0 completeness critic, not by this design. Where a set lacks a strongest-incumbent reference, the incumbent systems measured below act as REF-INC for the metric (objective §3.0).

- **Reference candidate:** Status quo production `ebound.exe` at the record commit; REF-INC: robocopy `/COPYALL /SL`, 7-Zip `-sni -sns -snl`, tar.exe, WIM (`dism /Capture-Image` or wimlib on Windows).
- **W and S:** determined on tuning (R3/R4) and declared in the §12 record before any validation look (§4.12).

Descriptions of every candidate are in the ledger row (`candidate_set`); they are not repeated here.

- **DEC-PLT-057** — candidates: `status-quo-declared-unavailable`, `audited-safe-wrapper-crate`, `isolated-ffi-crate`, `backup-read-stream`, `helper-process` (status quo: `status-quo-declared-unavailable`). Treatment: Measured when unblocked with the treatment of the non-admin counterpart experiment.
  - excluded before execution (ledger): `sddl-text-capture` — SDDL does not round-trip conditional ACEs and resource attributes; binary self-relative form is frozen. (violates SPEC §10.6 L1290; docs/security-metadata-v1.md type 42); L0/R1(b) counterexample and reviewer sign-off still required (R1).
  - excluded before execution (ledger): `lossy-acl-view` — Repo explicitly forbids substituting a lossy ACL view for unavailable descriptor capture. (violates docs/platform-fidelity-v1.md L27-28); L0/R1(b) counterexample and reviewer sign-off still required (R1).
- **DEC-PLT-044** — candidates: `status-quo-best-effort`, `btrfs-zfs-helpers`, `vss-helper`, `record-best-effort-flag-only`, `defer-post-v1` (status quo: `status-quo-best-effort`). Treatment: Measured when unblocked with the treatment of the non-admin counterpart experiment.
- **DEC-PLT-058** — candidates: `file-symlink-always-status-quo`, `infer-type-from-archive-tree`, `record-type-at-capture`, `junction-fallback-for-directories`, `skip-with-report-no-privilege`, `copy-target-content-fallback` (status quo: `file-symlink-always-status-quo`). Treatment: Measured when unblocked with the treatment of the non-admin counterpart experiment.
- **DEC-PLT-048** — candidates: `no-detection-status-quo`, `exclusive-create-only`, `pinned-unicode-full-casefold-nfc`, `probed-target-rules`, `filesystem-specific-tables`, `conservative-superset-plus-probe`, `confusables-advisory`, `confusables-refuse`, `enable-per-directory-case-sensitivity` (status quo: `no-detection-status-quo`). Treatment: Measured when unblocked with the treatment of the non-admin counterpart experiment.
- **DEC-PLT-028** — candidates: `status-quo-utf8-only-no-target-checks`, `refuse-per-target-hazard-list`, `refuse-portable-superset`, `declared-escape-renaming`, `native-bytes-on-posix`, `wtf8-surrogate-roundtrip`, `verbatim-long-path-prefix` (status quo: `status-quo-utf8-only-no-target-checks`). Treatment: Measured when unblocked with the treatment of the non-admin counterpart experiment.
  - excluded before execution (ledger): `silent-sanitize` — Silent mangling makes faithful and relocated extraction indistinguishable; SPEC P8 requires refusal or a declared, reported renaming policy (violates SPEC §3.4 P8 L224 (frozen)); L0/R1(b) counterexample and reviewer sign-off still required (R1).
- **DEC-PLT-035** — candidates: `status-quo-bootstrap-matrix`, `status-quo-plus-declared-gaps`, `tar-pax-parity-target`, `apple-archive-parity-macos`, `wim-robocopy-parity-windows`, `minimal-portable-guarantee` (status quo: `status-quo-bootstrap-matrix`, `status-quo-plus-declared-gaps`). Treatment: Measured when unblocked with the treatment of the non-admin counterpart experiment.
- **DEC-PLT-030** — candidates: `refuse-status-quo`, `windows-rs-safe-wrappers`, `audited-unsafe-deviceiocontrol`, `symlink-junction-only-via-std`, `capture-only-no-restore`, `external-helper-process` (status quo: `refuse-status-quo`). Treatment: Measured when unblocked with the treatment of the non-admin counterpart experiment.
- **DEC-PLT-038** — candidates: `undefined-status-quo`, `symlink-only-recognized`, `symlink-and-junction-recognized`, `registry-of-tags`, `remove-known-safe-option`, `guid-included-in-data`, `guid-excluded-separate-field` (status quo: `undefined-status-quo`). Treatment: Measured when unblocked with the treatment of the non-admin counterpart experiment.
- **DEC-PLT-055** — candidates: `status-quo-report-unavailable`, `restore-basic-bits-and-creation-time`, `restore-compression-via-fsctl`, `never-restore-cloud-bits`, `efs-raw-apis-post-v1` (status quo: `status-quo-report-unavailable`). Treatment: Measured when unblocked with the treatment of the non-admin counterpart experiment.
- **DEC-PLT-037** — candidates: `status-quo-hardlink-only`, `noreplace-rename-fallback`, `exclusive-create-direct-write`, `refuse-with-reason`, `add-directory-fsync` (status quo: `status-quo-hardlink-only`). Treatment: Measured when unblocked with the treatment of the non-admin counterpart experiment.
  - excluded before execution (ledger): `rename-replace` — Replaces an existing destination, violating the no-overwrite publication rule. (violates docs/repack-v1.md L34-36); L0/R1(b) counterexample and reviewer sign-off still required (R1).
- **DEC-PLT-018** — candidates: `abort-status-quo`, `copy-fallback-with-policy-and-report`, `reflink-then-copy`, `skip-alias-with-report`, `caller-grant-for-fallback` (status quo: `abort-status-quo`). Treatment: Measured when unblocked with the treatment of the non-admin counterpart experiment.
  - excluded before execution (ledger): `unchecked-copy-fallback` — SPEC Pattern 6 and incumbent CVEs (Python gh-151987, LinkFallbackError) require policy re-application on link emulation (violates SPEC §11.6 L1432 frozen extraction pattern); L0/R1(b) counterexample and reviewer sign-off still required (R1).
- **DEC-PLT-056** — candidates: `abort-pack-status-quo`, `skip-with-declared-loss`, `capture-symlinks-refuse-others`, `capture-all-exact`, `hydrate-or-follow-option` (status quo: `abort-pack-status-quo`). Treatment: Measured when unblocked with the treatment of the non-admin counterpart experiment.
  - excluded before execution (ledger): `coerce-to-file-or-directory` — Repo docs forbid coercing unknown reparse objects to File or Directory (violates docs/security-metadata-v1.md L33-34 (repo freeze)); L0/R1(b) counterexample and reviewer sign-off still required (R1).
- **DEC-CMP-028** — candidates: `status-quo-abort`, `exclude-with-report`, `include-with-record`, `snapshot-integration`, `stronger-predicate` (status quo: `status-quo-abort`). Treatment: Measured when unblocked with the treatment of the non-admin counterpart experiment.
  - excluded before execution (ledger): `silent-include` — Unstable content included silently (violates SPEC §5.7a L621-625); L0/R1(b) counterexample and reviewer sign-off still required (R1).
- **DEC-PLT-062** — candidates: `status-quo-compiled-untested-plus-unavailable-declarations`, `exclude-blocked-platform-claims`, `experimental-tier-per-platform`, `declare-unavailable-until-tested`, `emulated-correctness-evidence-only`, `documentation-and-third-party-evidence`, `community-contributor-runs`, `acquire-hardware-or-admin-session` (status quo: `status-quo-compiled-untested-plus-unavailable-declarations`). Treatment: Measured when unblocked with the treatment of the non-admin counterpart experiment.
  - excluded before execution (ledger): `hosted-ci-runners` — The program owner declined hosted runners; platform experiments are local and Docker only (violates PROGRESS standing decision 2026-09-12 (no GitHub Actions or hosted runners)); L0/R1(b) counterexample and reviewer sign-off still required (R1).

## Corpus, fixtures and case sets

- Fixtures with the counterpart seeds (tuning 20260917, validation 20260918); VHDX volumes created under `D:\eb-research\vhd\`; no C: usage.
- **Splits:** tuning and validation only; validation looks are registered in `research/decisions/validation-looks.jsonl` (§5.2, at most two per decision). Generated fixtures are assigned to splits by seed as stated above. No spec selects held-out (Phase D placeholder below).

## Environment and platform requirements

- **env_id:** new `windows-host-elevated` capture (same hardware, elevated token, recorded privileges).

| Requirement | Needed? |
|---|---|
| Linux | no |
| Windows 11 host | yes (elevated, BLOCKED) |
| macOS | no |
| x86-64 | yes |
| ARM64 | no |
| Network | OneDrive sync with owner consent only |
| Privileged | yes (administrator) |
| Large storage | about 40 GB of VHDX on D: |
| External review | no |

- **Storage discipline:** **PCP-6**.

## Commands

- **Run (elevated PowerShell 7 on the host):** `$env:PYTHONPATH='research/tools'; C:/Python313/python.exe -m ebr run --spec research/experiments/EXP-PLAT-023/spec.yaml` then `verify-raw`, `normalize`, `report`.
- **Scripts:** `scripts/elevated/suite.ps1 -Arm <sacl|vss|symlinks|8dot3|casesensitive|refs|exfat|placeholders|dedup> -FixtureFrom <item_id> -Scratch <dir>`; each script refuses to run without an elevated token and an owner-consent file for consent-gated arms.

## Seeds, warmups and replication

- **PCP-3** of `research/experiments/_index/platform-common-provisions.md` applies verbatim.

## Measurement method and metrics

As in the non-admin counterparts, plus privilege-dependent observations (SACL presence, VSS snapshot path, symlink type flags, `dir /x` short names, `fsutil file queryCaseSensitiveInfo`, placeholder hydration state).

| Metric | OD | Role here | ebr family | Unit | Direction | Threshold / oracle reference | Source |
|---|---|---|---|---|---|---|---|
| `capture_fidelity_fraction` | OD-03 | primary | other | fraction | maximize | T-20 (`metric_thresholds.capture_fidelity_fraction`; A.2 role `banded`) | observers |
| `restore_fidelity_fraction` | OD-03 | primary | other | fraction | maximize | T-20 (`metric_thresholds.restore_fidelity_fraction`; A.2 role `banded`) | observers |
| `hostile_name_handling_fraction` | OD-18 | binary R1 screen | correctness | fraction | equal_one | HC-08 (`metric_thresholds.hostile_name_handling_fraction`; A.2 role `binary`) | path matrix |
| `hc07_mvt07a_undeclared_differences` | OD-03 | HC oracle output (objective §2.0: binary; any non-zero value is a committed violation artifact and an R1 matter; not an Appendix A.2 metric, never enters §6 utility) | correctness | count | equal_zero | HC-07 MVT-07(a) | oracle |
| `hc07_mvt07d_mixed_version_entries` | OD-03 | HC oracle output (objective §2.0: binary; any non-zero value is a committed violation artifact and an R1 matter; not an Appendix A.2 metric, never enters §6 utility) | correctness | count | equal_zero | HC-07 MVT-07(d) | VSS arm |
| `hc18_mvt18a_crash_violations` | OD-25 | HC oracle output (objective §2.0: binary; any non-zero value is a committed violation artifact and an R1 matter; not an Appendix A.2 metric, never enters §6 utility) | correctness | count | equal_zero | HC-18 MVT-18(a) | exFAT/placeholder commit arm |

## Normalization

As in the counterparts.

## Statistical analysis

- **PCP-4** of `research/experiments/_index/platform-common-provisions.md` applies verbatim.
- **Scope rule:** until run, decisions must exclude elevated-only classes (SACL, VSS, symlink creation, ReFS, exFAT, 8.3) from `decision_scope` or remain `INSUFFICIENT_EVIDENCE` (§8.4 HC_UNVERIFIED for a platform in scope).

## Practical-significance thresholds

Referenced, never restated: each banded metric uses its `research/methods/thresholds.json` `metric_thresholds.<name>` entry (decision-method §3.2 ID in the metrics table; §3.1 semantics; §7.3 minimum-gain caps). Binary metrics and HC oracle outputs have no band (§3.3; objective §2.0 rule 1).

## Sensitivity analysis

- **PCP-5** of `research/experiments/_index/platform-common-provisions.md` applies verbatim.

## Held-out evaluation (Phase D placeholder)

Same Phase D placeholder as the counterparts, executed elevated after unlock.

## Cost accounting inputs (decision-method §7)

**PCP-8**.

## Expected negative results worth recording

- Expected blocked-platform negative: no SACL, VSS, symlink-creation, ReFS, exFAT, 8.3 or placeholder evidence exists in this program.

## Threats to validity

- Elevated runs may alter behaviour through privilege use that ordinary users lack; results are labelled by token privileges.

## Estimated cost and effort

- **Compute:** about 12 machine-hours; **disk:** about 40 GB peak (transient scratch unless stated).
- **Timing required:** no.
- **Tooling to build:** elevated variants of EXP-PLAT-002/004/009/013/014/016/017/018 scripts (`scripts/elevated/*.ps1`) plus VHDX provisioning (`New-VHD`, `Mount-VHD`, `Format-Volume -FileSystem ReFS|exFAT|NTFS` with `fsutil 8dot3name set` per volume); owner-consent record for any scan of personal or OneDrive folders (counts only; no names or contents leave the host)
- **Agent effort:** blocked; when unblocked: scripted execution with owner present for elevation prompts; judgment for SACL and placeholder side effects.

## Deviations (append-only; UTC time, author, reason, results-visible yes/no)

_None._

## Outcome (after analysis)

- classification (`outcome.json`, §10.1), verdict table reference, confirmation-bias triggers evaluated (§10.2): _pending_
