# EXP-PLAT-003: Corpus census of path hazards, link topology and metadata-structure distributions

> **Design pre-registration draft** (Phase C-design, platform domain). Index status: **READY**. Structure: decision-method §12.2 record fields plus the Phase C-design protocol fields. Shared provisions PCP-1…PCP-8 live in `research/experiments/_index/platform-common-provisions.md` and are part of this pre-registration. This file becomes the §12 `record.md` only after the gates are met.

## Pre-registration

- **decision_ids:** DEC-PLT-059, DEC-PLT-028, DEC-PLT-048, DEC-MOD-038, DEC-MOD-010, DEC-MOD-036, DEC-PLT-060, DEC-PLT-002, DEC-PLT-045, DEC-PLT-046, DEC-PLT-001, DEC-PLT-041, DEC-PLT-018, DEC-PLT-033, DEC-CON-001, DEC-LEG-032, DEC-LEG-033, DEC-LEG-065, DEC-LEG-067, DEC-LEG-070, DEC-LEG-076, DEC-LEG-088, DEC-LEG-092, DEC-LEG-094
- **Program sections:** 15, 16, 17, 18
- **Decision type (proposed, not assigned):** EMPIRICAL (prevalence and candidate compatibility cost over real and generated trees); descriptive for pure distributions. Assignment is made by a session not involved in the candidates (decision-method §1.2).
- **od_ids / hc_ids (proposed for the schema-v2 assignment, §14 item 12):** OD-03 (M03.1/M03.2 as exact capture/restore losses a refusal rule would cause), OD-17 (M17.3 `benign_false_refusal_fraction` for declared bounds only), OD-18; HC-07, HC-08, HC-17 (census never screens by itself).
- **Method:** `research/decision-method.md` and `research/methods/thresholds.json` at pre-registration commit `14b977c` (status `pre-registered`).
- **Pre-registration commit:** the checkpoint commit adding this file (design draft); the §12 record commit follows G-A; every run's raw `meta` git SHA must descend from it with `dirty = false` (§5.8 d).
- **Author, independence and exposure:** Phase C-design platform-domain designer (workflow `wf_47347bc4-611`); no candidate was constructed by this session (ledger R0 sets reproduced mechanically). Held-out identity exposure (event L7) is disclosed in **PCP-1**.
- **Public data sources used for tuning (§5.4 consequence 2):** upstream sources in `research/corpus/manifest.json` `provenance` for the tuning/validation items named below, plus the pinned tool downloads named under Commands.
- **Gates:** G-A to G-D as in **PCP-2**.

## Question and hypotheses

- **Question:** How often do platform hazards and structure-stressing metadata properties occur in the tuning and validation trees and in the legacy archives they contain, and what exact-capture, exact-restore or bound-refusal cost would each candidate refusal rule or bound impose on benign items?
- **H1:** Hazards that only some targets cannot represent (Windows-reserved stems, trailing dots, case-fold collisions) occur in a small but non-zero share of real benign items outside F19/F20, so a portable-superset refusal rule loses exact restores on at least two families, while structural bounds (3 ACLs, 65,536 ACEs, 4,096 xattrs, 16 MiB xattr values, 1,000,000 extents, 1 MiB symlink targets) are never reached by any real benign item.
- **H0 / equivalence:** No benign item outside F19/F20 contains any hazard class or bound-stressing value, so every refusal candidate is EQUIVALENT to the status quo on benign items.
- **Falsification of H1:** Zero hazard occurrences outside F19/F20 on both splits (then refusal-rule cost is unmeasurable here and the decisions rest on EXP-PLAT-002 cases only), or any real benign item exceeding a frozen bound (then R2 bound candidates face a benign false refusal).

## Decisions informed

| Decision | Release relevance | Ledger question (abridged; full text in the ledger) | What this experiment contributes | Evidence still needed elsewhere |
|---|---|---|---|---|
| DEC-PLT-059 | V1_IMPORTANT | Should pack and strict legacy import warn about, flag in the archive, or refuse names that collide or are unrepresentable on mainstream targets (SPEC: writers SHOULD w... | Hazard-class prevalence per family and the warning volume a pack-time diagnostic would emit. | Usability of warnings (HUMAN-FACING); target behaviour (EXP-PLAT-002). |
| DEC-PLT-028 | V1_BLOCKING | What exact rules map LogicalPath components to native names on each target (ext4, XFS, btrfs, APFS, NTFS, ReFS), and which components are refused or renamed: invalid U... | Prevalence of each hazard class in real trees and legacy archives. | Target behaviour (EXP-PLAT-002). |
| DEC-PLT-048 | V1_BLOCKING | Which collision classes must extraction detect (case-fold, NFC/NFD normalization, Windows reserved aliases, trailing dot/space, 8.3 short names, confusables), against... | False-positive/false-negative inputs: collision groups under each pinned rule set on real names. | Ground truth (EXP-PLAT-002); cost (EXP-PLAT-015). |
| DEC-MOD-038 | V1_BLOCKING | Do non-UTF-8 / non-Unicode native names (raw POSIX bytes, unpaired UTF-16 surrogates, legacy code pages) need LogicalPath representation in v1, and if so with which en... | Frequency of invalid UTF-8 names and legacy code pages in trees and archive members. | Representation behaviour (EXP-PLAT-002); identity (EXP-PLAT-005). |
| DEC-MOD-010 | V1_IMPORTANT | Should the ArchiveDescriptor carry path hostility and collision-class summaries, and if so are they advisory caches or validated authorities, with which enumerated cla... | Collision/hostility prevalence and operation counts for reader-side computation. | Timing (EXP-PLAT-015). |
| DEC-MOD-036 | V1_IMPORTANT | Which structural bounds belong in the model versus budgets and extraction policy: LogicalPath component length and depth, symlink target size (currently 1 MiB), and ca... | Distributions of component lengths, depths, symlink target sizes and entry counts. | Platform limits (EXP-PLAT-002/021); adversarial allocation tests (container/conformance domains). |
| DEC-PLT-060 | V1_IMPORTANT | Are the frozen xattr bounds (at most 4096 attributes per entry, values at most 16 MiB, names 1-255 bytes) justified, or should they be derived from filesystem limits,... | xattr count and value-size distributions by namespace (F19 images, OCI layer pax headers, archive members). | macOS resource forks (EXP-PLAT-022); per-filesystem maxima (EXP-PLAT-011/021); restore of oversize values (EXP-PLAT-012). |
| DEC-PLT-002 | V1_IMPORTANT | Are the frozen ACL bounds (at most 3 ACLs per Entry, 65,536 ACEs per ACL) justified, and may POSIX1E and NFS4 ACLs coexist on one Entry, and if so with what precedence... | ACE count distribution and co-occurrence of access/default ACLs. | Filesystem maxima and dialect coexistence (EXP-PLAT-011). |
| DEC-PLT-045 | V1_IMPORTANT | For v1 sparse handling: is the 1,000,000-extent bound justified, should dense files carry a sparse map, should non-Linux capture (macOS SEEK_HOLE, Windows allocated-ra... | Extent counts of fragmented VM and database images versus the 1,000,000-extent bound; share of dense files that carry maps. | Accuracy and overhead (EXP-PLAT-010). |
| DEC-PLT-046 | V1_BLOCKING | How are character/block devices, FIFOs and sockets handled in v1: first-class EntryKinds (SPEC §3.3), auxiliary snapshot metadata, FidelityReport-only omission with en... | Prevalence of FIFOs, sockets, devices and whiteouts in rootfs, container-layer and tar content. | Restoration feasibility (EXP-PLAT-009) and incumbent matrix (EXP-PLAT-008). |
| DEC-PLT-001 | V1_BLOCKING | For symlink members with absolute or escaping targets, should extraction refuse by default (status quo Safe, Python data filter), rewrite or resolve in-root (openat2 R... | Frequency of absolute and escaping symlinks in container layers and system trees. | Incumbent outcomes and threat analysis (EXP-PLAT-008/014). |
| DEC-PLT-041 | V1_BLOCKING | Should the Safe symlink policy remain a per-link lexical check, or resolve each target against the planned extracted tree including other archived symlinks (and case-i... | Share of intra-archive links whose planned-tree resolution differs from the lexical check (links through links, chains, loops). | Adversarial resolution (EXP-PLAT-014), cost (EXP-PLAT-015). |
| DEC-PLT-018 | V1_IMPORTANT | When a hardlink alias cannot be created (FAT/exFAT, some SMB/FUSE/cloud filesystems, cross-device), should extraction abort (status quo), materialize an independent ve... | Frequency and size of hardlink groups (package caches, git objects, snapshot trees) and copy-fallback byte expansion. | Restore onto targets without hardlinks (EXP-PLAT-009). |
| DEC-PLT-033 | V1_IMPORTANT | How should capture treat a multiply-linked file whose other aliases lie outside the packed root (silent ordinary File, entry-scoped declared loss, or refusal), and sho... | Prevalence of partial hardlink groups when packing first-level subtrees of snapshot-style trees. | Capture behaviour (EXP-PLAT-009). |
| DEC-CON-001 | V1_IMPORTANT | Do the eight ResourceBudget fields bound every attacker-controllable allocation (dictionaries, Merkle or outboard data, recipients, signatures, preserved legacy source... | Real distributions for xattr counts, ADS counts, hole lengths and preserved legacy bytes that the budget axes would bound. | Allocation-path audit and adversarial archives (container domain). |
| DEC-LEG-032 | V1_IMPORTANT | Should ZIP and 7z adapters project Unix symlinks (and reparse points) into EAM Symlink entries, matching tar import, or keep refusing special files? | Symlink prevalence in ZIP and 7z archives present in the corpus. | Safety review and adapter semantics (legacy domain). |
| DEC-LEG-033 | V1_IMPORTANT | What metadata do ZIP-synthesised ancestor directories receive, and must provenance/AUX distinguish synthesised from explicit directories so exports and diffs can repro... | Share of ZIP/tar archives in the corpus that need ancestor synthesis. | Export round-trip requirements (legacy domain). |
| DEC-LEG-065 | V1_BLOCKING | Should strict or a compatibility profile canonicalise a leading "./" member prefix and the "." root entry (and V7 trailing-slash directories) instead of refusing them? | Prevalence of `./`-prefixed members, `.` root entries and V7 trailing-slash directories in corpus tarballs. | GNU tar and bsdtar behaviour (legacy domain). |
| DEC-LEG-067 | V1_IMPORTANT | Should strict tar import refuse non-UTF-8 member names (hdrcharset=BINARY, legacy filesystems) or offer an explicit policy mapping them to declared encodings? | Non-UTF-8 tar member names and `hdrcharset=BINARY` prevalence. | Adapter policy (legacy domain). |
| DEC-LEG-070 | V1_BLOCKING | What rule classifies two timestamp (and flag) assertions as compatible Refinement versus Divergence (DOS local 2 s time versus UTC extended timestamps), should compat... | Distribution of DOS versus extended mtime differences and local/central flag differences in corpus ZIPs. | Runtime-version behaviour (legacy domain). |
| DEC-LEG-076 | POST_V1 | Should ZIP import reconcile __MACOSX/ AppleDouble members (ditto --sequesterRsrc) into native resource forks/xattrs, keep them as ordinary entries (status quo), or rep... | Prevalence of `__MACOSX/` AppleDouble members in corpus ZIPs (published samples only). | Finder/ditto creation (EXP-PLAT-022, BLOCKED). |
| DEC-LEG-088 | V1_IMPORTANT | Should ZIP import keep dropping DOS local timestamps (status quo), import them under an explicit recorded timezone assumption, store them as zone-less AUX metadata, an... | Fraction of corpus ZIPs lacking UT/NTFS timestamp extras, per ecosystem. | Fidelity tests of extras (EXP-PLAT-006); usability (HUMAN-FACING). |
| DEC-LEG-092 | V1_IMPORTANT | Which ZIP extras and attributes must v1 interpret into EAM metadata (Unix mode from external attributes, 0x7875 UID/GID, 0x756e ASi Unix, symlink attributes, 0xCAFE, 0... | Prevalence of Unix external attributes and extras (0x7875, 0x756e, 0xCAFE, 0xd935, 0x9901) in JAR/APK/wheel/Office archives. | Interpretation policy (legacy domain). |
| DEC-LEG-094 | V1_IMPORTANT | For ZIP names without the UTF-8 flag or a valid Unicode Path extra, should import keep deterministic CP437 decoding (status quo), accept an explicit caller code-page o... | Unflagged non-ASCII ZIP names: UTF-8-valid versus not, by producer field. | Encoding policy (legacy domain). |

## Candidates

Candidate sets are the ledger's R0 sets; completeness (status quo, SPEC design, named alternatives, strongest incumbent, defer, critic candidate, corrective candidate) is checked by the R0 completeness critic, not by this design. Where a set lacks a strongest-incumbent reference, the incumbent systems measured below act as REF-INC for the metric (objective §3.0).

- **Reference candidate:** The status quo of each decision; costs are reported as differences from it.
- **W and S:** determined on tuning (R3/R4) and declared in the §12 record before any validation look (§4.12).

Descriptions of every candidate are in the ledger row (`candidate_set`); they are not repeated here.

- **DEC-PLT-059** — candidates: `silent-status-quo`, `warn-on-stderr-and-report`, `record-hazard-summary`, `strict-import-refuse-option` (status quo: `silent-status-quo`). Treatment: Not measured directly: the census computes this candidate's benign cost (refusals, exact-capture/restore losses or bound hits) from the recorded distributions with the rule in `rules.json`.
  - excluded before execution (ledger): `default-refuse-hazardous-names` — Target-hostile and colliding names are legal archive content so POSIX trees archive faithfully; default refusal at write would contradict the frozen P5/P8 model (violates SPEC §3.4 P5/P8 (frozen); I18); L0/R1(b) counterexample and reviewer sign-off still required (R1).
- **DEC-PLT-028** — candidates: `status-quo-utf8-only-no-target-checks`, `refuse-per-target-hazard-list`, `refuse-portable-superset`, `declared-escape-renaming`, `native-bytes-on-posix`, `wtf8-surrogate-roundtrip`, `verbatim-long-path-prefix` (status quo: `status-quo-utf8-only-no-target-checks`). Treatment: Not measured directly: the census computes this candidate's benign cost (refusals, exact-capture/restore losses or bound hits) from the recorded distributions with the rule in `rules.json`.
  - excluded before execution (ledger): `silent-sanitize` — Silent mangling makes faithful and relocated extraction indistinguishable; SPEC P8 requires refusal or a declared, reported renaming policy (violates SPEC §3.4 P8 L224 (frozen)); L0/R1(b) counterexample and reviewer sign-off still required (R1).
- **DEC-PLT-048** — candidates: `no-detection-status-quo`, `exclusive-create-only`, `pinned-unicode-full-casefold-nfc`, `probed-target-rules`, `filesystem-specific-tables`, `conservative-superset-plus-probe`, `confusables-advisory`, `confusables-refuse`, `enable-per-directory-case-sensitivity` (status quo: `no-detection-status-quo`). Treatment: Not measured directly: the census computes this candidate's benign cost (refusals, exact-capture/restore losses or bound hits) from the recorded distributions with the rule in `rules.json`.
- **DEC-MOD-038** — candidates: `utf8-only-refuse-status-quo`, `opaque-bytes-declaration`, `utf16le-code-units`, `wtf8`, `legacy-charset-registry`, `transcode-on-import`, `escape-mapping`, `refuse-with-fidelity-report` (status quo: `utf8-only-refuse-status-quo`). Treatment: Not measured directly: the census computes this candidate's benign cost (refusals, exact-capture/restore losses or bound hits) from the recorded distributions with the rule in `rules.json`.
- **DEC-MOD-010** — candidates: `not-implemented-status-quo`, `validated-summary`, `advisory-cache`, `reader-computed-only`, `extractor-target-specific` (status quo: `not-implemented-status-quo`). Treatment: Not measured directly: the census computes this candidate's benign cost (refusals, exact-capture/restore losses or bound hits) from the recorded distributions with the rule in `rules.json`.
- **DEC-MOD-036** — candidates: `status-quo-bounds`, `platform-derived-bounds`, `budget-declared-only`, `format-limit-registry` (status quo: `status-quo-bounds`). Treatment: Not measured directly: the census computes this candidate's benign cost (refusals, exact-capture/restore losses or bound hits) from the recorded distributions with the rule in `rules.json`.
- **DEC-PLT-060** — candidates: `status-quo-4096-16mib`, `linux-vfs-limits`, `budget-declared-only`, `per-platform-limit-table`, `status-quo-16mib-4096`, `linux-vfs-64kib`, `per-platform-maximum`, `large-values-as-content-objects` (status quo: `status-quo-4096-16mib`, `status-quo-16mib-4096`). Treatment: Not measured directly: the census computes this candidate's benign cost (refusals, exact-capture/restore losses or bound hits) from the recorded distributions with the rule in `rules.json`.
- **DEC-PLT-002** — candidates: `status-quo-bounds-coexistence-allowed`, `forbid-mixed-dialects`, `platform-derived-ace-bounds`, `budget-only-bounds`, `keep-bounds-define-precedence` (status quo: `status-quo-bounds-coexistence-allowed`). Treatment: Not measured directly: the census computes this candidate's benign cost (refusals, exact-capture/restore losses or bound hits) from the recorded distributions with the rule in `rules.json`.
- **DEC-PLT-045** — candidates: `status-quo`, `omit-map-for-dense-files`, `lower-or-budgeted-extent-bound`, `add-windows-allocated-ranges`, `add-macos-seek-hole`, `restore-sparse-by-default`, `drop-sparse-v1` (status quo: `status-quo`). Treatment: Not measured directly: the census computes this candidate's benign cost (refusals, exact-capture/restore losses or bound hits) from the recorded distributions with the rule in `rules.json`.
- **DEC-PLT-046** — candidates: `status-quo-refuse-pack`, `omit-and-report-entry-scoped`, `first-class-kinds-spec`, `kinds-without-socket`, `aux-snapshot-metadata`, `policy-selectable-refuse-or-omit`, `post-v1-feature-bit` (status quo: `status-quo-refuse-pack`). Treatment: Not measured directly: the census computes this candidate's benign cost (refusals, exact-capture/restore losses or bound hits) from the recorded distributions with the rule in `rules.json`.
- **DEC-PLT-001** — candidates: `refuse-by-default-all-opt-in-status-quo`, `write-as-is-never-follow`, `rewrite-relative-in-root`, `skip-with-report`, `separate-grants-absolute-vs-escaping` (status quo: `refuse-by-default-all-opt-in-status-quo`). Treatment: Not measured directly: the census computes this candidate's benign cost (refusals, exact-capture/restore losses or bound hits) from the recorded distributions with the rule in `rules.json`.
- **DEC-PLT-041** — candidates: `lexical-per-link-status-quo`, `planned-tree-resolution`, `no-symlink-components-in-targets`, `post-extraction-physical-verify`, `case-insensitive-aware-resolution`, `refuse-all-symlinks-default` (status quo: `lexical-per-link-status-quo`). Treatment: Not measured directly: the census computes this candidate's benign cost (refusals, exact-capture/restore losses or bound hits) from the recorded distributions with the rule in `rules.json`.
- **DEC-PLT-018** — candidates: `abort-status-quo`, `copy-fallback-with-policy-and-report`, `reflink-then-copy`, `skip-alias-with-report`, `caller-grant-for-fallback` (status quo: `abort-status-quo`). Treatment: Not measured directly: the census computes this candidate's benign cost (refusals, exact-capture/restore losses or bound hits) from the recorded distributions with the rule in `rules.json`.
  - excluded before execution (ledger): `unchecked-copy-fallback` — SPEC Pattern 6 and incumbent CVEs (Python gh-151987, LinkFallbackError) require policy re-application on link emulation (violates SPEC §11.6 L1432 frozen extraction pattern); L0/R1(b) counterexample and reviewer sign-off still required (R1).
- **DEC-PLT-033** — candidates: `status-quo-silent`, `declare-partial-link-loss`, `refuse-pack-policy`, `windows-file-id-detection`, `windows-links-declared-unavailable` (status quo: `status-quo-silent`). Treatment: Not measured directly: the census computes this candidate's benign cost (refusals, exact-capture/restore losses or bound hits) from the recorded distributions with the rule in `rules.json`.
- **DEC-CON-001** — candidates: `status-quo-eight-fields`, `add-declared-fields`, `caller-policy-only-new-limits`, `fixed-format-bounds-registry`, `hole-logical-len-bound` (status quo: `status-quo-eight-fields`). Treatment: Not measured directly: the census computes this candidate's benign cost (refusals, exact-capture/restore losses or bound hits) from the recorded distributions with the rule in `rules.json`.
- **DEC-LEG-032** — candidates: `status-quo-refuse-zip-7z`, `zip-strict-v2-symlinks`, `symlinks-and-reparse`, `refuse-links-everywhere` (status quo: `status-quo-refuse-zip-7z`). Treatment: Not measured directly: the census computes this candidate's benign cost (refusals, exact-capture/restore losses or bound hits) from the recorded distributions with the rule in `rules.json`.
- **DEC-LEG-033** — candidates: `status-quo-report-list`, `aux-implicit-marker`, `inherit-child-mtime`, `no-metadata-typed-issue`, `status-quo-empty-metadata`, `policy-mode-0755`, `inherit-from-child`, `fidelity-report-flag` (status quo: `status-quo-report-list`, `status-quo-empty-metadata`). Treatment: Not measured directly: the census computes this candidate's benign cost (refusals, exact-capture/restore losses or bound hits) from the recorded distributions with the rule in `rules.json`.
- **DEC-LEG-065** — candidates: `status-quo-refuse`, `strip-leading-dot-slash-strict`, `compat-profile-only`, `v7-dirs-as-directories` (status quo: `status-quo-refuse`). Treatment: Not measured directly: the census computes this candidate's benign cost (refusals, exact-capture/restore losses or bound hits) from the recorded distributions with the rule in `rules.json`.
- **DEC-LEG-067** — candidates: `status-quo-refuse`, `opaque-bytes-declaration`, `charset-policy` (status quo: `status-quo-refuse`). Treatment: Not measured directly: the census computes this candidate's benign cost (refusals, exact-capture/restore losses or bound hits) from the recorded distributions with the rule in `rules.json`.
- **DEC-LEG-070** — candidates: `status-quo-exact-plus-unmodeled-refusal`, `precision-window-rule`, `zone-offset-tolerant-rule`, `prefer-extended-timestamp`, `model-flag-divergence`, `runtime-version-minor`, `runtime-version-patch` (status quo: `status-quo-exact-plus-unmodeled-refusal`). Treatment: Not measured directly: the census computes this candidate's benign cost (refusals, exact-capture/restore losses or bound hits) from the recorded distributions with the rule in `rules.json`.
- **DEC-LEG-076** — candidates: `status-quo-plain-entries`, `fold-into-forks-xattrs`, `opt-in-fold`, `report-and-drop`, `aux-opaque` (status quo: `status-quo-plain-entries`). Treatment: Not measured directly: the census computes this candidate's benign cost (refusals, exact-capture/restore losses or bound hits) from the recorded distributions with the rule in `rules.json`.
  - excluded before execution (ledger): `silent-drop` — Dropping __MACOSX members without a record is silent loss (violates I13); L0/R1(b) counterexample and reviewer sign-off still required (R1).
- **DEC-LEG-088** — candidates: `status-quo-drop-dos`, `assume-timezone-flag`, `assume-utc-default`, `zoneless-aux-metadata`, `report-extra-times`, `refinement-per-spec` (status quo: `status-quo-drop-dos`). Treatment: Not measured directly: the census computes this candidate's benign cost (refusals, exact-capture/restore losses or bound hits) from the recorded distributions with the rule in `rules.json`.
  - excluded before execution (ledger): `silent-atime-ctime-drop` — Discarding extra-field times without a typed record is silent metadata loss (violates I13); L0/R1(b) counterexample and reviewer sign-off still required (R1).
- **DEC-LEG-092** — candidates: `status-quo-minimal`, `import-full-posix-mode`, `import-uid-gid`, `import-symlinks`, `report-only-plus-typed-comments`, `ecosystem-extras-opaque-aux` (status quo: `status-quo-minimal`). Treatment: Not measured directly: the census computes this candidate's benign cost (refusals, exact-capture/restore losses or bound hits) from the recorded distributions with the rule in `rules.json`.
- **DEC-LEG-094** — candidates: `status-quo-cp437`, `explicit-codepage-override`, `heuristic-detection`, `utf8-if-valid-else-cp437`, `opaque-raw-bytes`, `refuse-unflagged-non-ascii` (status quo: `status-quo-cp437`). Treatment: Not measured directly: the census computes this candidate's benign cost (refusals, exact-capture/restore losses or bound hits) from the recorded distributions with the rule in `rules.json`.
  - excluded before execution (ledger): `nondeterministic-locale` — Decoding with the host OEM/ANSI locale makes interpretation host-dependent (violates I15); L0/R1(b) counterexample and reviewer sign-off still required (R1).

## Corpus, fixtures and case sets

- **Items:** every tuning and validation item in `research/corpus/manifest.json` (`corpus.splits: [tuning, validation]`, no family filter; 203 items). Held-out rows are never read (ebr guard plus `corpuslib.assert_not_heldout`).
- **Walk:** bytes-level `os.scandir` without following links; for F19/F16 filesystem-image items the image is loop-mounted read-only (`mount -o ro,loop[,norecovery]`; ntfs-3g `ro,streams_interface=xattr`) and walked; raw disk images are scanned for SEEK_DATA extents without mounting.
- **Archive members:** containers are detected by magic bytes (never by extension, HC-08) and listed without extraction: tar/pax/GNU (`tarfile` in stream mode), ZIP (`zipfile` plus raw extra-field parsing), 7z (`7z l -slt`), cpio (`bsdtar -tv`), gzip/xz/zstd-wrapped tars.
- **Provenance caveat:** mode, owner and xattrs of materialized directory trees come from provisioning, not from upstream; those distributions are reported only for filesystem-image items, archive members and generated F19 items whose generators set metadata deliberately. Name bytes, link targets, link topology and sparse extents are upstream-faithful.
- **Benign population:** all items outside F20; F19 generated items are reported separately because they are deliberately hostile; real versus generated versus derived items are stratified.
- **Splits:** tuning and validation only; validation looks are registered in `research/decisions/validation-looks.jsonl` (§5.2, at most two per decision). Generated fixtures are assigned to splits by seed as stated above. No spec selects held-out (Phase D placeholder below).

## Environment and platform requirements

- **env_id:** `wsl-ubuntu` (root, for read-only loop mounts). No timing.

| Requirement | Needed? |
|---|---|
| Linux (WSL2, root) | yes |
| Windows host | no |
| macOS | no |
| x86-64 | yes |
| ARM64 | no |
| Network | no |
| Privileged | root for read-only loop mounts |
| Large storage | no (reads corpus in place) |
| External review | no |

- **Storage discipline:** **PCP-6**.

## Commands

- **Pre-flight:** `C:/Python313/python.exe research/orchestration/disk_guard.py` must pass; no other workflow may hold the loop devices or mount points named here.
- **ebr (WSL):** `wsl.exe -d Ubuntu -u root -- bash -lc 'cd /mnt/d/Projects/entrybound/entrybound && export PYTHONPATH=research/tools && PY=/root/eb-research/venv/bin/python && $PY -m ebr validate --spec research/experiments/EXP-PLAT-003/spec.yaml && $PY -m ebr run --spec research/experiments/EXP-PLAT-003/spec.yaml && $PY -m ebr verify-raw --spec research/experiments/EXP-PLAT-003/spec.yaml --refingerprint && $PY -m ebr normalize --spec research/experiments/EXP-PLAT-003/spec.yaml && $PY -m ebr report --spec research/experiments/EXP-PLAT-003/spec.yaml'`
- **Scripts to be written (mechanical):**
  - `scripts/platform_census.py --item <path> --item-id <id> --family <F> --rules rules.json --detail-dir research/raw/EXP-PLAT-003/detail` — prints one JSON line with the counters in the spec and writes the per-item detail (name bytes digests and hazard hits, link table, hardlink groups, xattr/ACL/extent histograms) for the analysis; never prints item names to stdout.
  - `scripts/archive_members.py` — member-level parser used by the census.
  - `scripts/candidate_costs.py --detail research/raw/EXP-PLAT-003/detail --rules rules.json --out research/normalized/EXP-PLAT-003/decision/` — computes, per candidate rule, item-level losses and refusals, and exports `names.parquet` (hashed names plus hazard flags) for EXP-PLAT-002 Arm D.

## Seeds, warmups and replication

- **PCP-3** of `research/experiments/_index/platform-common-provisions.md` applies verbatim.

## Measurement method and metrics

- **Name hazards:** per component: non-UTF-8, each portable-superset class (reserved stems with and without extensions, trailing dot/space, control bytes, `\`, `:`, stream syntax), component bytes, depth, path bytes; collision groups under ASCII folding, Unicode 15.0 full folding, full folding plus NFC, NTFS upcase snapshot, NFC≠NFD pairs.
- **Links:** symlink class (relative in-root, relative escaping lexically, absolute, dangling, resolving through another archived link, loop), target byte length, planned-tree versus lexical disagreement; hardlink groups (count, size, bytes duplicated if copied), partial groups relative to the item root and to each first-level subdirectory.
- **Special objects:** FIFOs, sockets, char/block devices, OCI whiteout markers (`.wh.*`, `.wh..wh..opq`), overlay char 0/0.
- **Metadata structure:** xattr count and value sizes by namespace, POSIX ACL entry counts (decoded from `system.posix_acl_*`), ACL mask/mode disagreement (the ebound refusal case), sparse extents per file and dense files with maps, entry counts per directory and per item.
- **Legacy members:** the per-decision counters listed in Decisions informed.
- **Candidate costs:** for each candidate rule or bound, the item-level exact-capture loss (pack aborts or omitted entries), exact-restore loss (refused names on a stated target) or bound hit.

| Metric | OD | Role here | ebr family | Unit | Direction | Threshold / oracle reference | Source |
|---|---|---|---|---|---|---|---|
| `restore_fidelity_fraction` | OD-03 | primary for name-refusal candidates (entries a rule would refuse on the stated target / benign entries) | other | fraction | maximize | T-20 (`metric_thresholds.restore_fidelity_fraction`; A.2 role `banded`) | `candidate_costs.py` |
| `capture_fidelity_fraction` | OD-03 | primary for pack-abort candidates (special files, reparse sources, ACL mask mismatch, non-UTF-8 names) | other | fraction | maximize | T-20 (`metric_thresholds.capture_fidelity_fraction`; A.2 role `banded`) | `candidate_costs.py` |
| `benign_false_refusal_fraction` | OD-17 | primary only for declared-bound candidates (ACL/ACE, xattr, extent, symlink-target and budget bounds) | other | fraction | minimize | T-20 (`metric_thresholds.benign_false_refusal_fraction`; A.2 role `banded`) | `candidate_costs.py` |
| `scale_limits` | OD-17 | descriptive (M17.6): observed maxima | other | count; B | report | - (`metric_thresholds.scale_limits`; A.2 role `descriptive`) | stdout_json counters |
| `declared_gap_count` | OD-03 | descriptive (M03.3) | other | count | report | - (`metric_thresholds.declared_gap_count`; A.2 role `descriptive`) | analysis |

## Normalization

Counts are exact per item; fractions use benign entries (or benign items for pack-abort rules) of the item as denominator; distributions are summarized per item as max, p95 and p50 (type-7, §4.8). Reference: status quo rule (zero cost by definition where it does not refuse).

## Statistical analysis

- **PCP-4** of `research/experiments/_index/platform-common-provisions.md` applies verbatim.
- **Corpus-level:** tuning has 20 families and validation 20; L4 intervals (§4.10) are computed where §4.9 support holds, with `research/methods/workload-mix.json` base weights and the equal-weight arm.
- **Use by other experiments:** candidate costs from this census are the benign-cost inputs for R4 in EXP-PLAT-002, 008, 009, 010, 011, 013 and 014; they are not confirmatory on their own.

## Practical-significance thresholds

Referenced, never restated: each banded metric uses its `research/methods/thresholds.json` `metric_thresholds.<name>` entry (decision-method §3.2 ID in the metrics table; §3.1 semantics; §7.3 minimum-gain caps). Binary metrics and HC oracle outputs have no band (§3.3; objective §2.0 rule 1).

## Sensitivity analysis

- **PCP-5** of `research/experiments/_index/platform-common-provisions.md` applies verbatim.
- **Byte-weighted arm (§6.5):** costs weighted by logical bytes rather than entries.
- **Real-only arm:** recomputation excluding generated and derived items.

## Held-out evaluation (Phase D placeholder)

After unlock the census is re-run once on the held-out split for the same families so that benign-cost verdicts supporting any selection are replicated under R5 (c)/(d); no re-selection (§5.6).

## Cost accounting inputs (decision-method §7)

**PCP-8**.

## Expected negative results worth recording

- Frozen bounds that are never approached by any benign item are a negative result for 'tighter bound' candidates' benefit and for 'budget-only' candidates' necessity alike; both are recorded.
- If ACL mask/mode disagreement occurs on real Linux images (F19 construction mirrors real default-ACL inheritance), the status quo's whole-pack refusal is a measured capture loss.
- Absence of `__MACOSX/` members in the corpus leaves DEC-LEG-076's prevalence unmeasured, not zero.

## Threats to validity

- Materialized trees lost upstream ownership and modes; distributions of those fields are restricted as stated.
- Corpus composition (not a random sample of the world) limits prevalence claims to this corpus; the workload mix weights are an input, not a correction.
- F19 generated items are hostile by construction; they are excluded from benign populations.
- Unicode normalization of names by upstream tooling (git on macOS, npm) may have removed some hazards before provisioning.

## Estimated cost and effort

- **Compute:** about 4 machine-hours; **disk:** about 2 GB peak (transient scratch unless stated).
- **Timing required:** no.
- **Tooling to build:** research/experiments/EXP-PLAT-003/scripts/{platform_census.py,archive_members.py,candidate_costs.py} and rules.json (mechanical)
- **Agent effort:** execution none (scripted, about 4 machine-hours); tooling scripted; judgment only when interpreting candidate cost tables in the decision analyses.

## Deviations (append-only; UTC time, author, reason, results-visible yes/no)

_None._

## Outcome (after analysis)

- classification (`outcome.json`, §10.1), verdict table reference, confirmation-bias triggers evaluated (§10.2): _pending_
