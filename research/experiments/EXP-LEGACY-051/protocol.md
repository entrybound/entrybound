# EXP-LEGACY-051: conversion provenance and Legacy Observation Model evidence — emitted-value census, outcome-class consistency, identity stability across releases, name-encoding declarations, executable derivation, 7z evidence exactness, LOM generality, implicit directories and timestamp precision

| Field | Value |
|---|---|
| Status | **NEEDS_TOOLING** (`provenance_census.py`; historical-build script over legacy-touching commits; name-declaration and executable-derivation case builders; `observe()` dump accessor; LOM mapping prototypes for cpio and OCI layers; implicit-directory and missing-MTime prototypes behind `research-internals`) |
| Domain / program sections | legacy / §25 (with §15/§16 model mapping, §28 independent reader, §34 v1 freeze) |
| decision_ids | DEC-LEG-003, DEC-LEG-004, DEC-LEG-023, DEC-LEG-033, DEC-LEG-034, DEC-LEG-049, DEC-LEG-053, DEC-LEG-088, DEC-LEG-092, DEC-MOD-019, DEC-MOD-022, DEC-MOD-046, DEC-PLT-051 |
| Decision types (proposed) | EMPIRICAL (census, identity observations) with FORMAL parts (code audit tables, identity-collision constructions); HUMAN-FACING part of DEC-LEG-088 not addressed |
| OD / HC (proposed) | OD-04, OD-03, OD-20 (M20.1), OD-18 (M18.1), OD-23 (C1 inputs), OD-21; HC-10, HC-08, HC-07, HC-16, HC-01 |
| Shared rules / L7 / gates | C§ header, C§2 |
| Timing | `timing: false` |

## 1. Question

What values do the legacy adapters actually write into ConversionProvenance and LOM evidence, are outcome classes for
the same structural fault consistent across adapters, are import identities stable across importer commits, do name
declarations and executable derivation give one identity for one logical tree regardless of source format and host,
how exact is 7z evidence compared with ZIP, can the LOM represent cpio and OCI-layer constructs without schema change,
and how do implicit directories, missing 7z MTime, ZIP time extras and mixed timestamp precisions affect evidence,
fidelity reports and diffs?

## 2. Hypotheses

- **H1 (census, DEC-LEG-004/023).** At least one conversion-record string field takes a value outside any documented
  vocabulary, and at least one structural fault class (truncation, bad CRC/checksum, malformed header) maps to different
  outcome classes in the ZIP and 7z adapters. *Falsified by* closed, consistent vocabularies.
- **H2 (release stability).** No importer commit between `9e44608` and the recorded HEAD changes LAI for any legacy
  corpus input unless the commit introduces a new documented adapter version. *Falsified by* one undocumented LAI
  change (an HC-16/HC-10 finding).
- **H3 (declarations, DEC-MOD-019).** The same name bytes receive different declared encodings through ZIP, tar and 7z
  adapters for at least one byte class, giving different LAIs for byte-identical names.
- **H4 (executable, DEC-MOD-022).** One logical tree yields at least three distinct `core.executable` patterns across
  Linux pack, Windows pack, Info-ZIP ZIP import, PowerShell ZIP import, p7zip 7z import and GNU tar import (extends
  finding F-0002).
- **H5 (strict vs compat identity, DEC-MOD-046).** On ambiguous EXP-LEGACY-001 cases, at least one compat profile's LAI
  differs from strict's (where strict imports with a recorded resolution).
- **H6 (7z exactness, DEC-LEG-053).** Fewer 7z header field classes than ZIP field classes carry exact source extents and
  raw bytes in LOM observations.
- **H7 (LOM generality, DEC-LEG-034).** cpio newc and OCI layer constructs (hardlink inode groups, whiteouts, opaque
  directories) map onto existing LOM types without schema change for all but whiteout semantics.
- **H8 (precision, DEC-PLT-051).** `ebound diff` reports a change for at least one pair of archives encoding the same
  instant at different source precisions. *Falsified by* no false change.
- **H0.** Evidence is closed and consistent; identities are host- and format-independent.

## 3. Decisions informed

| Decision | Supplied here |
|---|---|
| DEC-LEG-003 | Field-by-field gap table SPEC §16.1 versus ConversionProvenance (`crates/entrybound/src/eam/model.rs` lines), with provisional C1 counts per candidate field set (sizes in EXP-LEGACY-040) |
| DEC-LEG-004 | Enumeration of emitted values per adapter; byte-observation rendering; LAI stability across importer releases |
| DEC-LEG-023 | Outcome class per structural fault per adapter; adapter-id strings as written |
| DEC-LEG-033 | Whether AUX/provenance distinguishes synthesised directories; export and diff effects of candidates |
| DEC-LEG-034 | cpio/OCI mapping prototypes; inventory of adapter deviations (7z coarse evidence, refinement-only conflicts); change history of LOM types and records 28-36 |
| DEC-LEG-049 | LAI effect and FidelityReport truthfulness for 7z entries without MTime under candidates |
| DEC-LEG-053 | Evidence-exactness audit 7z versus ZIP; LOM size impact estimate |
| DEC-LEG-088 | Captured/unavailable classes asserted for UT (0x5455) and NTFS (0x000a) times |
| DEC-LEG-092 | Which extras and attributes currently reach EAM versus evidence-only, per fixture |
| DEC-MOD-019 | Cross-adapter declarations for identical name bytes; constructed identity-collision pairs under `bytes-only-identity` |
| DEC-MOD-022 | Executable derivation across pack hosts and import formats; LAI comparison |
| DEC-MOD-046 | Strict versus compat LAI divergence on ambiguous ZIPs |
| DEC-PLT-051 | Diff false-change rate across precision mixes |

## 4. Candidates

| Decision | Candidates (ledger ids) | Treatment |
|---|---|---|
| DEC-LEG-003 | status-quo-provenance; spec-field-set; minimal-additions; external-receipt-json; preserve-only-detail | gap table and C1 counts |
| DEC-LEG-004 | status-quo-free-form; closed-registries-v2; documented-string-registry; exact-byte-observations; record-tool-version-and-runtime; unverified-producer-metadata | census evaluated against each |
| DEC-LEG-023 | status-quo-per-adapter; uniform-corrupt; uniform-nonconforming; namespace-format-mode-version; namespace-entrybound-prefix | census |
| DEC-LEG-033 | status-quo-report-list / status-quo-empty-metadata; aux-implicit-marker; inherit-child-mtime / inherit-from-child; no-metadata-typed-issue; policy-mode-0755; fidelity-report-flag | prototypes for marker and inheritance |
| DEC-LEG-034 | status-quo-experimental; freeze-wire-only; freeze-full-contract; explicit-layer-tree; per-adapter-models; universal-entry-facade | mapping prototypes and inventories |
| DEC-LEG-049 | status-quo-placeholder-captured; omit-and-report-unavailable; placeholder-reported-unavailable; refuse-missing-mtime | prototypes |
| DEC-LEG-053 | status-quo-coarse; exact-all-fields; exact-conflict-fields-only; decoded-header-layer; conflicts-in-lom | audit and size estimates |
| DEC-LEG-088 | status-quo-drop-dos; assume-timezone-flag; assume-utc-default; zoneless-aux-metadata; report-extra-times; refinement-per-spec | mechanical capture classes only |
| DEC-LEG-092 | status-quo-minimal; import-full-posix-mode; import-uid-gid; import-symlinks; report-only-plus-typed-comments; ecosystem-extras-opaque-aux | rules over fixtures |
| DEC-MOD-019 | participates-status-quo; bytes-only-identity; canonical-declaration-rule; declaration-in-conversion-record | collision constructions |
| DEC-MOD-022 | any-x-bit-optional-status-quo; always-present-on-files; absent-means-false-canonical; owner-x-bit; non-posix-false; extension-heuristic | `extension-heuristic` proposed L0 exclusion (HC-08: content categorisation never uses extensions); others evaluated as derivation rules |
| DEC-MOD-046 | record-mode-and-profile; strict-only; unverified-claim; not-in-v1 | divergence frequency |
| DEC-PLT-051 | status-quo-five-classes; add-two-second-and-millisecond; add-unknown-precision; exponent-encoded-precision; refuse-out-of-registry-precision; truncate-with-declared-loss | diff false-change evaluation per rule |

## 5. Corpus selectors

- EXP-LEGACY-001..004 tuning and validation case sets and real tiers; EXP-LEGACY-010 tuning/validation samples (≤ 5,000
  artifacts per split drawn with seeds `2509175101`/`2509175102`) for the census and release-stability arms.
- Generated (`research/tools/legacy/cases/identity_cases.py`; F19/F20; `S_T = 2509175103`, `S_V = 2509175104`): name
  byte classes (ASCII, UTF-8 NFC/NFD, CP437 high bytes, Latin-1, Shift-JIS, invalid UTF-8) written as ZIP (flagged,
  unflagged, Unicode Path extra), tar (raw ustar, pax `path`, `hdrcharset=BINARY`), 7z (UTF-16LE incl. unpaired
  surrogates); an executable-mode tree (0755, 0744, 0711, 0700, 0710, 0701, 0644, 0600; directories) packed on WSL ext4
  and Windows NTFS and archived by Info-ZIP `zip`, PowerShell 5.1 `Compress-Archive`, 7-Zip Windows `-tzip`, p7zip 16.02,
  7-Zip Windows `-t7z`, GNU tar; timestamp-precision pairs (ZIP DOS 2 s, UT 1 s, NTFS 100 ns, pax ns, 7z 100 ns, ustar
  octal 1 s) for equal instants; 7z archives without MTime; ZIPs with UT/NTFS/0x7875/0x756e extras and implicit
  ancestors.
- Held-out: placeholder (C§5).

## 6. Environment

`wsl-ubuntu`; Windows `ebound.exe` native build (EXP-LEGACY-031 arm B build) via interop for Windows packing; historical
builds under `/root/eb-research/target/legacy-history/<sha>`.

## 7. Commands and tooling

Tooling: `research/tools/legacy/provenance_census.py` (parses `ebound inspect --json --provenance --entries`),
`build_history.sh` (`git log --format=%H 9e44608e5a3ed44fb8f2c4cc5423a6f69b0d0155..<record HEAD> -- crates/entrybound/src/legacy crates/entrybound/src/eam`
limited to ≤ 15 commits evenly spaced plus both ends; clone and build each), `cases/identity_cases.py`,
`lom_mapping_prototype.py` (cpio/OCI → LOM records validated by the production LOM validator through research
accessors), `lom_history.py` (git history of `crates/entrybound/src/legacy/lom.rs` and `docs/format-v0.md` records
28-36), `precision_diff.py` (`ebound diff --json` over precision pairs), code audit tables
`research/experiments/EXP-LEGACY-051/audits/{provenance-gap.md, sevenz-exactness.md, executable-emitters.md}`.

```sh
/mnt/c/Python313/python.exe research/orchestration/disk_guard.py   # Windows Python (winreg checks) through WSL interop
bash research/tools/legacy/build_history.sh --from 9e44608e5a3ed44fb8f2c4cc5423a6f69b0d0155 --to <record HEAD> --max 15
export PYTHONPATH=research/tools
/root/eb-research/venv/bin/python research/tools/legacy/materialize_cases.py --experiment EXP-LEGACY-051 --generator identity_cases --split tuning --seed 2509175103 --from-experiments EXP-LEGACY-001,EXP-LEGACY-002,EXP-LEGACY-003,EXP-LEGACY-004,EXP-LEGACY-010
/root/eb-research/venv/bin/python research/tools/legacy/materialize_cases.py --experiment EXP-LEGACY-051 --generator identity_cases --split validation --seed 2509175104 --from-experiments EXP-LEGACY-001,EXP-LEGACY-002,EXP-LEGACY-003,EXP-LEGACY-004,EXP-LEGACY-010
/root/eb-research/venv/bin/python -m ebr run --spec research/experiments/EXP-LEGACY-051/spec.yaml
/root/eb-research/venv/bin/python -m ebr verify-raw --spec research/experiments/EXP-LEGACY-051/spec.yaml --refingerprint
/root/eb-research/venv/bin/python -m ebr normalize --spec research/experiments/EXP-LEGACY-051/spec.yaml
/root/eb-research/venv/bin/python research/tools/legacy/provenance_census.py --analyze --experiment EXP-LEGACY-051 --out research/normalized/EXP-LEGACY-051/
```

## 8. Seeds

`order 2509175111`, `bootstrap 2509175112`, `command 2509175113`; sampling and case seeds §5.

## 9. Replication

2 repetitions with repeat-hash of every census and identity output; historical builds are built once, their binaries
hashed and re-verified before each use.

## 10. Metrics

| Metric | Canonical / role | OD | Threshold |
|---|---|---|---|
| `migration_identity_conformance_fraction` (release stability; strict+preserve and candidate identity rules) | binary | OD-20 | §3.3 |
| `cross_host_identity_agreement` (Windows vs WSL pack of the executable tree, after declared FidelityReport differences) | binary | OD-18 | §3.3 |
| `capture_fidelity_fraction` (extras, times, 7z MTime) | T-20 | OD-03 | T-20 |
| `reason_code_specificity_fraction` (outcome-class consistency per fault class) | T-20 | OD-04 | T-20 |
| `side_data_bytes`, `metadata_bytes_per_entry` (exact-evidence size estimates) | T-02 / T-02b | OD-05 | T-02, T-02b |
| `normative_words_added`, `wire_items_added` (provisional C1 per candidate field set or registry) | cost_input | OD-23 | §7 |
| `declared_gap_count` | descriptive | OD-03 | none |
| distinct emitted values per field; outcome-class matrix; declaration differences; executable patterns; constructed collision pairs; diff false changes; LOM change counts | non-canonical descriptive | — | none |

## 11-12. Normalization and analysis

Census values compared as exact strings/bytes; identities as hex digests; FidelityReport entries parsed into (class,
state). Release stability: every LAI change is matched against the commit's documentation diff (a change without a new
adapter version identifier is a finding). Collision constructions are FORMAL counterexamples when found. Candidate rules
are evaluated deterministically over observations; the audit tables are reviewed by a session not involved in writing
them (§1.2 FORMAL route).

## 13. Practical significance

T-02, T-02b, T-20 per `decision-method.md` §3.2; binary §3.3.

## 14. Sensitivity analysis

Commit-sampling arm (≤ 15 evenly spaced versus all legacy-touching commits if ≤ 40); host arm (Windows vs WSL); writer
arm for executable derivation (drop each writer).

## 15. Expected negative results worth recording

Undocumented identity changes across importer commits (a frozen-identifier finding); consistent vocabularies already
(weakens registry candidates); LOM needing schema changes for cpio/OCI (argues against freezing the full contract);
precision mixes producing no false diffs.

## 16. Threats to validity

Historical builds may not compile with the pinned toolchain (recorded, skipped); PowerShell 5.1 and 7-Zip Windows
writers run through interop; the gap analysis depends on the SPEC reading of one session (independent review required).

## 17. Platform requirements

Linux x86-64 WSL2; Windows x86-64 non-admin for Windows packing and writers; macOS writers **PLATFORM_BLOCKED**
(EXP-LEGACY-080); storage ~40 GB (historical targets).

## 18. Estimates

Compute ≈ 30 CPU-hours plus ≈ 3 build-hours; disk ≈ 40 GB. Agent effort: audit tables and mapping prototypes are judgment;
census scripted.

## 19. Expected cost tiers `[INFERRED]`

Closed registries and new record fields: T2-T3; exact 7z evidence changing preserved record semantics: T3 (new version);
declaration participation or executable presence changes: T4 (identity participation), never in place for Entry-v1 bytes
(HC-16); implicit-directory AUX marker: T4 if it participates in AUX identity, T2 as a FidelityReport flag.
