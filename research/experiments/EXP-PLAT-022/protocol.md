# EXP-PLAT-022: macOS APFS/HFS+ fidelity, names, forks, flags, ACLs, quarantine and confinement suite (BLOCKED)

> **Design pre-registration draft** (Phase C-design, platform domain). Index status: **BLOCKED** — PLATFORM_BLOCKED: requires a macOS host (Apple silicon and Intel, current and previous major release) with APFS case-insensitive and case-sensitive volumes, an HFS+ disk image, SIP enabled, and an admin account for chflags SF_* and hdiutil arms; the program's standing decision declines hosted runners (PROGRESS 2026-09-12). Structure: decision-method §12.2 record fields plus the Phase C-design protocol fields. Shared provisions PCP-1…PCP-8 live in `research/experiments/_index/platform-common-provisions.md` and are part of this pre-registration. This file becomes the §12 `record.md` only after the gates are met.

## Pre-registration

- **decision_ids:** DEC-PLT-008, DEC-PLT-016, DEC-PLT-027, DEC-PLT-029, DEC-PLT-031, DEC-PLT-049, DEC-PLT-005, DEC-PLT-017, DEC-PLT-041, DEC-PLT-048, DEC-PLT-028, DEC-PLT-035, DEC-PLT-024, DEC-PLT-040, DEC-PLT-045, DEC-PLT-047, DEC-PLT-062, DEC-LEG-076, DEC-PLT-007, DEC-PLT-061, DEC-INT-034
- **Program sections:** 15, 16, 17, 20, 21
- **Decision type (proposed, not assigned):** EMPIRICAL (mirrors EXP-PLAT-002/004/009/012/014 on macOS). Assignment is made by a session not involved in the candidates (decision-method §1.2).
- **od_ids / hc_ids (proposed for the schema-v2 assignment, §14 item 12):** OD-03, OD-18, OD-25; HC-05, HC-07, HC-08, HC-17.
- **Method:** `research/decision-method.md` and `research/methods/thresholds.json` at pre-registration commit `14b977c` (status `pre-registered`).
- **Pre-registration commit:** the checkpoint commit adding this file (design draft); the §12 record commit follows G-A; every run's raw `meta` git SHA must descend from it with `dirty = false` (§5.8 d).
- **Author, independence and exposure:** Phase C-design platform-domain designer (workflow `wf_47347bc4-611`); no candidate was constructed by this session (ledger R0 sets reproduced mechanically). Held-out identity exposure (event L7) is disclosed in **PCP-1**.
- **Public data sources used for tuning (§5.4 consequence 2):** upstream sources in `research/corpus/manifest.json` `provenance` for the tuning/validation items named below, plus the pinned tool downloads named under Commands.
- **Gates:** G-A to G-D as in **PCP-2**.

## Question and hypotheses

- **Question:** What do APFS and HFS+ do with each hostile name class and collision (normalization-insensitive, case-insensitive and case-sensitive volumes); what fraction of FinderInfo, resource forks, decmpfs/UF_COMPRESSED state, BSD flags, birthtime, macOS ACLs and com.apple.quarantine/provenance xattrs do Entrybound, Apple Archive, ditto and bsdtar capture and restore exactly; how do Archive Utility, ditto and bsdtar propagate quarantine; and do O_NOFOLLOW_ANY/O_RESOLVE_BENEATH-based containment candidates stay inside the root under the EXP-PLAT-014 scenarios?
- **H1:** APFS normalization-insensitive lookup aliases NFC/NFD pairs that ext4 keeps distinct; decmpfs content is readable decompressed while the xattr is hidden from listxattr; unprivileged users cannot set SF_* flags; Archive Utility propagates quarantine to extracted members while bsdtar does not; status-quo macOS capture declares ACLs unavailable.
- **H0 / equivalence:** APFS behaves like ext4 for every case and every class restores exactly.
- **Falsification of H1:** APFS distinguishes NFC/NFD pairs on the default volume, or every class restores exactly under the status quo.

## Decisions informed

| Decision | Release relevance | Ledger question (abridged; full text in the ledger) | What this experiment contributes | Evidence still needed elsewhere |
|---|---|---|---|---|
| DEC-PLT-008 | V1_IMPORTANT | How are transparently compressed files (macOS decmpfs with UF_COMPRESSED, NTFS compression) captured and restored: decompressed content with the attribute dropped and... | macOS/APFS/HFS+ column of this decision's evidence (designed, not runnable here). | All non-macOS evidence is in EXP-PLAT-001–021. |
| DEC-PLT-016 | V1_IMPORTANT | Which file flags are captured and restored: a restorable subset of macOS UF/SF flags, exclusion of kernel-managed bits (SF_DATALESS, UF_COMPRESSED, SF_FIRMLINK), and a... | macOS/APFS/HFS+ column of this decision's evidence (designed, not runnable here). | All non-macOS evidence is in EXP-PLAT-001–021. |
| DEC-PLT-027 | V1_BLOCKING | How are NTFS alternate data streams (including Zone.Identifier), macOS resource forks, FinderInfo, AppleDouble sidecars and xattr analogues classified: LAI-participati... | macOS/APFS/HFS+ column of this decision's evidence (designed, not runnable here). | All non-macOS evidence is in EXP-PLAT-001–021. |
| DEC-PLT-029 | V1_IMPORTANT | Should the NFS4 ACL dialect remain frozen without any producing or consuming implementation, and which non-Linux ACL capture targets (macOS extended ACLs, FreeBSD/ZFS... | macOS/APFS/HFS+ column of this decision's evidence (designed, not runnable here). | All non-macOS evidence is in EXP-PLAT-001–021. |
| DEC-PLT-031 | V1_IMPORTANT | How does Entrybound handle origin/provenance markers: preserve raw member markers (Zone.Identifier, com.apple.quarantine/provenance) only, define a canonical cross-pla... | macOS/APFS/HFS+ column of this decision's evidence (designed, not runnable here). | All non-macOS evidence is in EXP-PLAT-001–021. |
| DEC-PLT-049 | V1_IMPORTANT | Which timestamps beyond mtime are captured, declared or restored in v1 (atime, ctime, Linux statx btime capture-only, Windows creation-time restore, macOS birthtime re... | macOS/APFS/HFS+ column of this decision's evidence (designed, not runnable here). | All non-macOS evidence is in EXP-PLAT-001–021. |
| DEC-PLT-005 | V1_BLOCKING | Which containment primitive does Entrybound extraction (and capture) use on Linux, macOS and Windows, given cap-std 4.0.3 uses openat2 RESOLVE_BENEATH only on Linux (w... | macOS/APFS/HFS+ column of this decision's evidence (designed, not runnable here). | All non-macOS evidence is in EXP-PLAT-001–021. |
| DEC-PLT-017 | V1_BLOCKING | Which handle-relative, no-follow operations must capture and extraction use for directory, file, symlink and special-file creation and metadata (mode, owner, xattrs, t... | macOS/APFS/HFS+ column of this decision's evidence (designed, not runnable here). | All non-macOS evidence is in EXP-PLAT-001–021. |
| DEC-PLT-041 | V1_BLOCKING | Should the Safe symlink policy remain a per-link lexical check, or resolve each target against the planned extracted tree including other archived symlinks (and case-i... | macOS/APFS/HFS+ column of this decision's evidence (designed, not runnable here). | All non-macOS evidence is in EXP-PLAT-001–021. |
| DEC-PLT-048 | V1_BLOCKING | Which collision classes must extraction detect (case-fold, NFC/NFD normalization, Windows reserved aliases, trailing dot/space, 8.3 short names, confusables), against... | macOS/APFS/HFS+ column of this decision's evidence (designed, not runnable here). | All non-macOS evidence is in EXP-PLAT-001–021. |
| DEC-PLT-028 | V1_BLOCKING | What exact rules map LogicalPath components to native names on each target (ext4, XFS, btrfs, APFS, NTFS, ReFS), and which components are refused or renamed: invalid U... | macOS/APFS/HFS+ column of this decision's evidence (designed, not runnable here). | All non-macOS evidence is in EXP-PLAT-001–021. |
| DEC-PLT-035 | V1_BLOCKING | Which metadata classes does stable v1 guarantee to capture and restore on each supported OS/filesystem (Linux ext4/XFS/btrfs/tmpfs, Windows NTFS/exFAT/ReFS, macOS APFS... | macOS/APFS/HFS+ column of this decision's evidence (designed, not runnable here). | All non-macOS evidence is in EXP-PLAT-001–021. |
| DEC-PLT-024 | V1_IMPORTANT | Which incumbent-matrix metadata fidelity claims (Entrybound 'full' POSIX/Windows/macOS fidelity, tar parity, sole declared-loss reporting) survive measurement, and how... | macOS/APFS/HFS+ column of this decision's evidence (designed, not runnable here). | All non-macOS evidence is in EXP-PLAT-001–021. |
| DEC-PLT-040 | V1_IMPORTANT | Under workspace unsafe_code=forbid, how does Entrybound obtain Windows security descriptors, ADS enumeration, macOS ACLs, birthtime restoration, resource forks and no-... | macOS/APFS/HFS+ column of this decision's evidence (designed, not runnable here). | All non-macOS evidence is in EXP-PLAT-001–021. |
| DEC-PLT-045 | V1_IMPORTANT | For v1 sparse handling: is the 1,000,000-extent bound justified, should dense files carry a sparse map, should non-Linux capture (macOS SEEK_HOLE, Windows allocated-ra... | macOS/APFS/HFS+ column of this decision's evidence (designed, not runnable here). | All non-macOS evidence is in EXP-PLAT-001–021. |
| DEC-PLT-047 | V1_IMPORTANT | What minimum Linux kernel, macOS and Windows versions does v1 support for metadata capture and kernel-enforced confinement, and what fallback or report applies below t... | macOS/APFS/HFS+ column of this decision's evidence (designed, not runnable here). | All non-macOS evidence is in EXP-PLAT-001–021. |
| DEC-PLT-062 | V1_BLOCKING | How does stable v1 disposition capabilities that cannot be evaluated locally (macOS/APFS metadata, native ARM64 performance, ReFS, admin-only Windows operations): keep... | macOS/APFS/HFS+ column of this decision's evidence (designed, not runnable here). | All non-macOS evidence is in EXP-PLAT-001–021. |
| DEC-LEG-076 | POST_V1 | Should ZIP import reconcile __MACOSX/ AppleDouble members (ditto --sequesterRsrc) into native resource forks/xattrs, keep them as ordinary entries (status quo), or rep... | macOS/APFS/HFS+ column of this decision's evidence (designed, not runnable here). | All non-macOS evidence is in EXP-PLAT-001–021. |
| DEC-PLT-007 | V1_BLOCKING | What is the v1 default extraction policy and the closed set of named grants for dangerous restorations: setuid/setgid/sticky and umask, privilege-bearing xattr namespa... | macOS/APFS/HFS+ column of this decision's evidence (designed, not runnable here). | All non-macOS evidence is in EXP-PLAT-001–021. |
| DEC-PLT-061 | V1_IMPORTANT | Should xattr restoration filter namespaces (security.capability, security.selinux, security.ima, trusted.*, system.*, com.apple.quarantine), relabel rather than restor... | macOS/APFS/HFS+ column of this decision's evidence (designed, not runnable here). | All non-macOS evidence is in EXP-PLAT-001–021. |
| DEC-INT-034 | V1_IMPORTANT | Which in-band unverified marker is used per destination type for salvage output (report only, filename suffix, xattr/ADS, marker manifest, quarantine layout, new archi... | macOS/APFS/HFS+ column of this decision's evidence (designed, not runnable here). | All non-macOS evidence is in EXP-PLAT-001–021. |

## Candidates

Candidate sets are the ledger's R0 sets; completeness (status quo, SPEC design, named alternatives, strongest incumbent, defer, critic candidate, corrective candidate) is checked by the R0 completeness critic, not by this design. Where a set lacks a strongest-incumbent reference, the incumbent systems measured below act as REF-INC for the metric (objective §3.0).

- **Reference candidate:** Status quo production `ebound` built for aarch64-apple-darwin and x86_64-apple-darwin at the record commit; REF-INC: Apple Archive (`aa`), `ditto`, bsdtar (system libarchive).
- **W and S:** determined on tuning (R3/R4) and declared in the §12 record before any validation look (§4.12).

Descriptions of every candidate are in the ledger row (`candidate_set`); they are not repeated here.

- **DEC-PLT-008** — candidates: `status-quo-decompressed-read-flag-recorded`, `opaque-capture-only-blob`, `drop-with-declared-loss`, `recompress-on-restore` (status quo: `status-quo-decompressed-read-flag-recorded`). Treatment: Measured on macOS when unblocked, with the same treatment as the Linux/Windows counterpart experiment.
  - excluded before execution (ledger): `synthesize-decmpfs` — SPEC forbids synthesising the undocumented macOS compression attribute. (violates SPEC §10.7 L1296); L0/R1(b) counterexample and reviewer sign-off still required (R1).
- **DEC-PLT-016** — candidates: `status-quo`, `macos-user-flag-subset-restore`, `declare-linux-flags-unavailable`, `add-linux-flags-name`, `immutable-last-privileged-opt-in` (status quo: `status-quo`). Treatment: Measured on macOS when unblocked, with the same treatment as the Linux/Windows counterpart experiment.
- **DEC-PLT-027** — candidates: `status-quo`, `declare-ads-unavailable`, `ads-as-xattr-analogue`, `named-content-streams-in-lai`, `per-class-split`, `appledouble-materialize-on-export`, `refuse-files-with-streams` (status quo: `status-quo`). Treatment: Measured on macOS when unblocked, with the same treatment as the Linux/Windows counterpart experiment.
- **DEC-PLT-029** — candidates: `status-quo-frozen-unvalidated`, `validate-via-linux-nfs-client`, `macos-acl-adapter-v1`, `separate-macos-acl-dialect`, `defer-nfs4-post-v1`, `freebsd-zfs-adapter` (status quo: `status-quo-frozen-unvalidated`). Treatment: Measured on macOS when unblocked, with the same treatment as the Linux/Windows counterpart experiment.
- **DEC-PLT-031** — candidates: `status-quo-none`, `preserve-raw-only`, `propagate-archive-marker-default`, `canonical-origin-metadata`, `caller-policy-flag`, `propagate-when-source-marked` (status quo: `status-quo-none`). Treatment: Measured on macOS when unblocked, with the same treatment as the Linux/Windows counterpart experiment.
- **DEC-PLT-049** — candidates: `status-quo-mtime-plus-platform-birth`, `declare-unavailable-only`, `add-linux-btime-capture-only`, `add-atime-optional`, `add-ctime-capture-only`, `exclude-creation-time-by-default`, `restore-creation-time-and-birthtime` (status quo: `status-quo-mtime-plus-platform-birth`). Treatment: Measured on macOS when unblocked, with the same treatment as the Linux/Windows counterpart experiment.
- **DEC-PLT-005** — candidates: `cap-std-4-status-quo`, `rustix-openat2-in-root-no-symlinks`, `manual-openat-nofollow-walk`, `macos-o-resolve-beneath`, `windows-ntcreatefile-reparse-walk`, `mount-namespace-or-chroot-sandbox`, `landlock-seccomp-complement`, `refuse-unsupported-platforms` (status quo: `cap-std-4-status-quo`). Treatment: Measured on macOS when unblocked, with the same treatment as the Linux/Windows counterpart experiment.
  - excluded before execution (ledger): `string-prefix-validation` — SPEC §11.6 Pattern 1 forbids string validation followed by ordinary file operations; R1 and R2 classify prefix checking as a defect class (violates SPEC §11.6 L1413 frozen extraction doctrine); L0/R1(b) counterexample and reviewer sign-off still required (R1).
- **DEC-PLT-017** — candidates: `status-quo-mixed`, `rustix-at-nofollow`, `o-path-proc-self-fd-bridge`, `cap-std-ext-apis`, `audited-unsafe-ffi-boundary`, `decline-and-report-only`, `status-quo`, `handle-based-capture`, `kernel-resolve-beneath-macos`, `landlock-sandboxed-extraction`, `declare-concurrent-mutation-out-of-scope`, `private-mount-destination` (status quo: `status-quo-mixed`, `status-quo`). Treatment: Measured on macOS when unblocked, with the same treatment as the Linux/Windows counterpart experiment.
  - excluded before execution (ledger): `path-based-with-checks` — SPEC §11.6 Pattern 2 forbids setting metadata by path; silently doing so is a conformance violation (violates SPEC §11.6 L1424-1426 frozen extraction pattern); L0/R1(b) counterexample and reviewer sign-off still required (R1).
- **DEC-PLT-041** — candidates: `lexical-per-link-status-quo`, `planned-tree-resolution`, `no-symlink-components-in-targets`, `post-extraction-physical-verify`, `case-insensitive-aware-resolution`, `refuse-all-symlinks-default` (status quo: `lexical-per-link-status-quo`). Treatment: Measured on macOS when unblocked, with the same treatment as the Linux/Windows counterpart experiment.
- **DEC-PLT-048** — candidates: `no-detection-status-quo`, `exclusive-create-only`, `pinned-unicode-full-casefold-nfc`, `probed-target-rules`, `filesystem-specific-tables`, `conservative-superset-plus-probe`, `confusables-advisory`, `confusables-refuse`, `enable-per-directory-case-sensitivity` (status quo: `no-detection-status-quo`). Treatment: Measured on macOS when unblocked, with the same treatment as the Linux/Windows counterpart experiment.
- **DEC-PLT-028** — candidates: `status-quo-utf8-only-no-target-checks`, `refuse-per-target-hazard-list`, `refuse-portable-superset`, `declared-escape-renaming`, `native-bytes-on-posix`, `wtf8-surrogate-roundtrip`, `verbatim-long-path-prefix` (status quo: `status-quo-utf8-only-no-target-checks`). Treatment: Measured on macOS when unblocked, with the same treatment as the Linux/Windows counterpart experiment.
  - excluded before execution (ledger): `silent-sanitize` — Silent mangling makes faithful and relocated extraction indistinguishable; SPEC P8 requires refusal or a declared, reported renaming policy (violates SPEC §3.4 P8 L224 (frozen)); L0/R1(b) counterexample and reviewer sign-off still required (R1).
- **DEC-PLT-035** — candidates: `status-quo-bootstrap-matrix`, `status-quo-plus-declared-gaps`, `tar-pax-parity-target`, `apple-archive-parity-macos`, `wim-robocopy-parity-windows`, `minimal-portable-guarantee` (status quo: `status-quo-bootstrap-matrix`, `status-quo-plus-declared-gaps`). Treatment: Measured on macOS when unblocked, with the same treatment as the Linux/Windows counterpart experiment.
- **DEC-PLT-024** — candidates: `keep-claims`, `restate-per-measured-matrix`, `remove-full-until-implemented`, `publish-per-class-matrix`. Treatment: Measured on macOS when unblocked, with the same treatment as the Linux/Windows counterpart experiment.
- **DEC-PLT-040** — candidates: `status-quo-declare-unavailable`, `third-party-safe-wrappers`, `workspace-isolated-ffi-crate`, `helper-process-os-tools`, `upstream-std-cap-std-extensions`, `std-only-subset` (status quo: `status-quo-declare-unavailable`). Treatment: Measured on macOS when unblocked, with the same treatment as the Linux/Windows counterpart experiment.
- **DEC-PLT-045** — candidates: `status-quo`, `omit-map-for-dense-files`, `lower-or-budgeted-extent-bound`, `add-windows-allocated-ranges`, `add-macos-seek-hole`, `restore-sparse-by-default`, `drop-sparse-v1` (status quo: `status-quo`). Treatment: Measured on macOS when unblocked, with the same treatment as the Linux/Windows counterpart experiment.
- **DEC-PLT-047** — candidates: `status-quo-unstated`, `linux-5-6-openat2-minimum`, `linux-lts-5-10-minimum`, `macos-11-minimum`, `macos-latest-two`, `tiered-support-weaker-reported` (status quo: `status-quo-unstated`). Treatment: Measured on macOS when unblocked, with the same treatment as the Linux/Windows counterpart experiment.
- **DEC-PLT-062** — candidates: `status-quo-compiled-untested-plus-unavailable-declarations`, `exclude-blocked-platform-claims`, `experimental-tier-per-platform`, `declare-unavailable-until-tested`, `emulated-correctness-evidence-only`, `documentation-and-third-party-evidence`, `community-contributor-runs`, `acquire-hardware-or-admin-session` (status quo: `status-quo-compiled-untested-plus-unavailable-declarations`). Treatment: Measured on macOS when unblocked, with the same treatment as the Linux/Windows counterpart experiment.
  - excluded before execution (ledger): `hosted-ci-runners` — The program owner declined hosted runners; platform experiments are local and Docker only (violates PROGRESS standing decision 2026-09-12 (no GitHub Actions or hosted runners)); L0/R1(b) counterexample and reviewer sign-off still required (R1).
- **DEC-LEG-076** — candidates: `status-quo-plain-entries`, `fold-into-forks-xattrs`, `opt-in-fold`, `report-and-drop`, `aux-opaque` (status quo: `status-quo-plain-entries`). Treatment: Measured on macOS when unblocked, with the same treatment as the Linux/Windows counterpart experiment.
  - excluded before execution (ledger): `silent-drop` — Dropping __MACOSX members without a record is silent loss (violates I13); L0/R1(b) counterexample and reviewer sign-off still required (R1).
- **DEC-PLT-007** — candidates: `status-quo-per-class-enums`, `python-data-filter-parity`, `bsdtar-style-p-flag`, `named-grants-closed-set`, `xattr-namespace-allow-list`, `root-aware-defaults`, `per-platform-xattr-defaults`, `status-quo-per-class-flags`, `spec-allow-list`, `named-presets`, `per-class-flags-plus-allow-for-privileged`, `python-filter-model` (status quo: `status-quo-per-class-enums`, `status-quo-per-class-flags`). Treatment: Measured on macOS when unblocked, with the same treatment as the Linux/Windows counterpart experiment.
  - excluded before execution (ledger): `archive-requested-restoration` — Extraction policy is caller-owned and cannot be constructed or widened by archive content (violates I16); L0/R1(b) counterexample and reviewer sign-off still required (R1).
- **DEC-PLT-061** — candidates: `status-quo-all-or-nothing`, `user-namespace-only-default`, `separate-security-namespace-flag`, `capability-as-dangerous-class`, `selinux-relabel`, `security-namespaces-capture-only` (status quo: `status-quo-all-or-nothing`). Treatment: Measured on macOS when unblocked, with the same treatment as the Linux/Windows counterpart experiment.
- **DEC-INT-034** — candidates: `report-only`, `filename-suffix`, `xattr-or-ads-marker`, `marker-manifest-file`, `quarantine-directory`, `salvage-to-new-archive`. Treatment: Measured on macOS when unblocked, with the same treatment as the Linux/Windows counterpart experiment.
  - excluded before execution (ledger): `no-marking` — Salvage artifacts must be marked unverified (violates I25); L0/R1(b) counterexample and reviewer sign-off still required (R1).

## Corpus, fixtures and case sets

- Generated fixtures with the same seeds as EXP-PLAT-002/004/009/012/014 (tuning 20260917, validation 20260918), plus Finder-created and `ditto -c -k --sequesterRsrc` ZIPs (for DEC-LEG-076), and a quarantined download simulated with `xattr -w com.apple.quarantine`.
- Corpus items are copied from tuning/validation only.
- **Splits:** tuning and validation only; validation looks are registered in `research/decisions/validation-looks.jsonl` (§5.2, at most two per decision). Generated fixtures are assigned to splits by seed as stated above. No spec selects held-out (Phase D placeholder below).

## Environment and platform requirements

- **env_id:** new `macos-<arch>` captures. Volumes: APFS default (case-insensitive), APFS case-sensitive (created with `diskutil apfs addVolume`), HFS+ image (`hdiutil create -fs HFS+`).

| Requirement | Needed? |
|---|---|
| Linux | no |
| Windows | no |
| macOS | yes (BLOCKED) |
| x86-64 | yes (Intel Mac) |
| ARM64 | yes (Apple silicon, native) |
| Network | no |
| Privileged | admin for SF_* flags and volume creation arms |
| Large storage | about 50 GB |
| External review | no |

- **Storage discipline:** **PCP-6**.

## Commands

- **Build:** `cargo +1.98.1 build --release -p entrybound-cli --bin ebound` on each Mac at the record commit.
- **Run:** `PYTHONPATH=research/tools python3 -m ebr run --spec research/experiments/EXP-PLAT-022/spec.yaml` then `verify-raw`, `normalize`, `report`; raw artifacts returned with the environment capture.
- **Scripts:** `scripts/macos/suite.sh --arm <paths|fidelity|special|streams|confinement> --fixture-from <item_id> --scratch <dir>`.

## Seeds, warmups and replication

- **PCP-3** of `research/experiments/_index/platform-common-provisions.md` applies verbatim.

## Measurement method and metrics

As in the counterpart experiments, with macOS observers (`stat -f '%Sf %B'`, `xattr -lx`, `ls -le`, `GetFileInfo`, `mdls`, `fs_usage` for write monitoring).

| Metric | OD | Role here | ebr family | Unit | Direction | Threshold / oracle reference | Source |
|---|---|---|---|---|---|---|---|
| `capture_fidelity_fraction` | OD-03 | primary | other | fraction | maximize | T-20 (`metric_thresholds.capture_fidelity_fraction`; A.2 role `banded`) | observers |
| `restore_fidelity_fraction` | OD-03 | primary | other | fraction | maximize | T-20 (`metric_thresholds.restore_fidelity_fraction`; A.2 role `banded`) | observers |
| `hostile_name_handling_fraction` | OD-18 | binary R1 screen | correctness | fraction | equal_one | HC-08 (`metric_thresholds.hostile_name_handling_fraction`; A.2 role `binary`) | path matrix |
| `hc07_mvt07a_undeclared_differences` | OD-03 | HC oracle output (objective §2.0: binary; any non-zero value is a committed violation artifact and an R1 matter; not an Appendix A.2 metric, never enters §6 utility) | correctness | count | equal_zero | HC-07 MVT-07(a) | oracle |
| `hc05_mvt05_escapes` | OD-25 | HC oracle output (objective §2.0: binary; any non-zero value is a committed violation artifact and an R1 matter; not an Appendix A.2 metric, never enters §6 utility) | correctness | count | equal_zero | HC-05 MVT-05(a)/(b) | confinement suite |
| `unsafe_defaults` | OD-25 | binary screen | correctness | count | equal_zero | HC-05 (`metric_thresholds.unsafe_defaults`; A.2 role `binary`) | quarantine propagation |

## Normalization

As in EXP-PLAT-002/004/014.

## Statistical analysis

- **PCP-4** of `research/experiments/_index/platform-common-provisions.md` applies verbatim.
- **Scope rule:** until this experiment runs, every decision whose scope includes macOS stays `INSUFFICIENT_EVIDENCE` (§4.1, §8.4); decisions may explicitly exclude macOS from `decision_scope` instead.

## Practical-significance thresholds

Referenced, never restated: each banded metric uses its `research/methods/thresholds.json` `metric_thresholds.<name>` entry (decision-method §3.2 ID in the metrics table; §3.1 semantics; §7.3 minimum-gain caps). Binary metrics and HC oracle outputs have no band (§3.3; objective §2.0 rule 1).

## Sensitivity analysis

- **PCP-5** of `research/experiments/_index/platform-common-provisions.md` applies verbatim.

## Held-out evaluation (Phase D placeholder)

Same Phase D placeholder as the counterpart experiments, executed on the macOS host after unlock.

## Cost accounting inputs (decision-method §7)

**PCP-8**.

## Expected negative results worth recording

- Expected blocked-platform negative: no macOS evidence exists in this program; all macOS fidelity and confinement claims remain unsupported.

## Threats to validity

- Contributor-run results come from hardware outside the program's environment control; they carry the contributor's environment capture and are not paired with local results (§4.1).

## Estimated cost and effort

- **Compute:** about 20 machine-hours; **disk:** about 50 GB peak (transient scratch unless stated).
- **Timing required:** no.
- **Tooling to build:** macOS variants of the Linux scripts under research/experiments/EXP-PLAT-022/scripts/macos/ (path matrix, fidelity observers using `stat -f`, `xattr -lx`, `ls -le`, `GetFileInfo`, `ls -lO`, `mdls`), confinement suite port using O_NOFOLLOW_ANY/O_RESOLVE_BENEATH prototypes, Apple Archive (`aa`) and `ditto` system arms; a contributor test kit (the spec below plus pinned build instructions) so a maintainer with the hardware can run it and return raw artifacts with an environment capture
- **Agent effort:** blocked; when unblocked: scripted execution (about 20 machine-hours), judgment for macOS-specific oracle rules.

## Deviations (append-only; UTC time, author, reason, results-visible yes/no)

_None._

## Outcome (after analysis)

- classification (`outcome.json`, §10.1), verdict table reference, confirmation-bias triggers evaluated (§10.2): _pending_
