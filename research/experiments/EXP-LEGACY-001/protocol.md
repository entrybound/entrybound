# EXP-LEGACY-001: ZIP runtime differential outcome matrix (primitive cases × pinned runtimes × Windows/WSL, with Entrybound strict, compat and preserve columns)

| Field | Value |
|---|---|
| Status | **NEEDS_TOOLING** (case generator `zip_cases.py` C0-C14; reader drivers for 14 runtime families; runtime acquisition and Linux reproduction builds; `ebound_import.py`; `matrix.py`) |
| Domain / program sections | legacy / §25 (feeds §29 conformance expectation classes and §30 adversarial fixtures) |
| decision_ids | DEC-ECO-047, DEC-LEG-001, DEC-LEG-012, DEC-LEG-031, DEC-LEG-032, DEC-LEG-070, DEC-LEG-077, DEC-LEG-078, DEC-LEG-079, DEC-LEG-080, DEC-LEG-081, DEC-LEG-082, DEC-LEG-083, DEC-LEG-084, DEC-LEG-085, DEC-LEG-086, DEC-LEG-087, DEC-LEG-089, DEC-LEG-090, DEC-LEG-092, DEC-LEG-094, DEC-LEG-095, DEC-LEG-102, DEC-LEG-103, DEC-LEG-104, DEC-LEG-105, DEC-INT-022, DEC-PLT-010 |
| Decision types (proposed, §1.2) | EMPIRICAL (observed runtime behaviour, agreement fractions) with FORMAL parts (normative citation per case; triage rule) |
| OD / HC (proposed, G-A) | OD-19 (M19.2, M19.3), OD-04 (M04.2), OD-17 (M17.4 for C14); HC-07, HC-17, HC-05, HC-06 (C14), HC-15 |
| Shared rules | `research/experiments/_legacy/conventions.md` (C§1-C§13), runtime registry `_legacy/runtime-registry.json` |
| Author / L7 | See C§ header: legacy-domain designer session with a recorded L7 identity exposure; analysis plan needs adoption by a non-exposed session before G-A |
| Gates | G-A before `record.md` and any run; G-B before decision-grade analysis (C§2) |
| Timing | `timing: false` (correctness and coverage only) |

## 1. Question

For every pre-registered ZIP primitive construction and real ZIP-family archive of the tuning and validation splits,
what does each pinned ZIP runtime (listing, content selection, extraction, error) and each Entrybound import mode
(strict, the four frozen compat profiles, compat+preserve) do; where do they diverge; do the frozen compat rule tables
agree with the runtimes they claim to model; and which additional runtimes add distinguishing behaviour?

## 2. Hypotheses

- **H1 (reproduction).** The Windows anchors (`python-zipfile-3.13.5-win`, `java-zipfile-21.0.12.1-win`,
  `java-zipinputstream-21.0.12.1-win`, `libarchive-3.8.8-win`) reproduce `tools/zip-compat/observed-outcomes-v1.json`
  on all 14 C0 cases (listing, selected content digests, error classes), and the same versions built or installed on
  WSL reproduce the Windows observations on every C0 case. *Falsified by* one differing cell.
- **H2 (profile fidelity).** Each frozen profile agrees (C§7) with its anchor on 100% of C0-C13 tuning cases.
  *Falsified by* one disagreement that the triage rule does not attribute to a runtime bug.
- **H3 (strict safety floor).** `ebound convert --strict` refuses or records as a typed LOM conflict every case that is
  ambiguous under C§7 (`ambiguity_refusal_fraction` = 1). *Falsified by* one ambiguous case imported with no recorded
  conflict.
- **H4 (marginal runtime value, DEC-LEG-082).** Each of Go `archive/zip`, .NET `ZipArchive` and 7-Zip 23.01 differs
  from all four frozen-profile anchors on at least 3 tuning cases; Info-ZIP `unzip`, zip2, yauzl and Commons Compress
  on at least 1 each `[INFERRED prediction]`. *Falsified* per runtime by fewer distinguishing cases.
- **H5 (version drift, DEC-LEG-080).** At least one adjacent-version pair (CPython 3.12.3/3.13.5/3.14, JDK 17/21/25,
  libarchive 3.7.2/3.8.8) differs on at least one case. *Falsified by* zero drift across all pairs (which would
  support `deprecate-never-remove`/`version-range-metadata` candidates over frequent re-pinning).
- **H6 (compat revalidation, DEC-LEG-083).** Under the status quo, at least one C14 case (local size spanning the next
  member under `java-zipinputstream` or `libarchive`; declared-1-KiB DEFLATE bomb under `java-zipfile`) is projected
  without the overlap/extent or actual-ratio check. *Falsified by* all C14 cases refused with a typed code.
- **H0.** No divergence beyond those already in `observed-outcomes-v1.json`; profiles are exact models.

## 3. Decisions informed

| Decision | Supplied here | Remaining elsewhere |
|---|---|---|
| DEC-ECO-047 | Pinned multi-runtime matrix; regeneration cost and runtime; strict/broad divergence recount; positive controls per detector; Linux-vs-Windows reproduction | ZipDiff harness trial on local hardware is arm C2-Z below; prevalence claims in EXP-LEGACY-010 |
| DEC-LEG-001 | Effort (hours, lines) to extend rule tables to C1-C14; unmatched conflict patterns among generated cases | Fuzzed generalisation EXP-LEGACY-005; tail frequency EXP-LEGACY-010 |
| DEC-LEG-012, DEC-LEG-089 | Runtime and Entrybound behaviour on ZipCrypto, AE-1/AE-2 and partially encrypted archives (C12) | Prevalence EXP-LEGACY-010; crypto review external (§8.3) |
| DEC-LEG-031, DEC-LEG-090, DEC-LEG-103 | Outcomes for SFX (absolute/relative offsets), gaps, hidden local entries (EB-Z007/Z011 class), multiple EOF-aligned EOCD, decoy EOCD in comment, concatenated and zip-in-zip, multi-disk, non-EOF-aligned EOCD (C7) | Prevalence EXP-LEGACY-010; APK Signing Block round trip EXP-LEGACY-050 |
| DEC-LEG-032, DEC-LEG-084, DEC-PLT-010 | Symlink (0o120777), FIFO/device (0o010644, 0o020644) and directory-with-content (EB-Z025 class) outcomes per runtime and per profile (C9) | tar/7z analogues EXP-LEGACY-002/003 |
| DEC-LEG-070 | DOS-2 s versus UT/NTFS time windows and local/central flag differences per runtime (C10) | Real-corpus distribution EXP-LEGACY-010 |
| DEC-LEG-077, DEC-LEG-094, DEC-LEG-104 | Backslash and colon names, CP437 all-byte table agreement, UTF-8 flag with invalid UTF-8, Unicode Path extra variants (C8) | Prevalence by producer EXP-LEGACY-010; identity participation EXP-LEGACY-051 |
| DEC-LEG-078, DEC-INT-022 | CRC-collision size cases (STORE, DEFLATE; shorter and longer declared sizes) through every runtime and profile (C11) | Security review of candidates (independent session) |
| DEC-LEG-079 | Duplicates with reordered central directory, case-fold and NFC/NFD collisions, extraction-to-disk results (C13) | Platform extraction collision rules (platform domain) |
| DEC-LEG-080, DEC-LEG-082 | Drift across adjacent versions; distinguishing cases per candidate runtime; Windows/Linux reproduction | Pipeline gatekeeping research (external citations in EXP-LEGACY-021) |
| DEC-LEG-081, DEC-LEG-087 | JSON replay of the whole matrix against each profile; traceability count of rule-table cells to ≥ 2 distinguishing cases; the unified versioned matrix itself | Differential fuzzing EXP-LEGACY-005 |
| DEC-LEG-083 | C14 fixtures under every profile; false refusals on real descriptor archives under compat | Peak memory/time under candidates EXP-LEGACY-041 |
| DEC-LEG-085 | Runtime support for Deflate64, bzip2, LZMA, zstd 93/20, XZ, PPMd (C12) | Method distribution EXP-LEGACY-010; bombs EXP-LEGACY-041; dependency audit EXP-LEGACY-021 |
| DEC-LEG-086 | Descriptor width variants from real writers and hand-built mismatches (C6) | none |
| DEC-LEG-092 | Runtime interpretation of external attributes and extras (0x7875, 0x756e, 0x5455, 0x000a, 0xCAFE, 0xd935, 0x9901) (C5, C9) | Prevalence EXP-LEGACY-010; fidelity classes EXP-LEGACY-051 |
| DEC-LEG-095 | Per case: whether the strict refusal carries LOM evidence (dry-run provenance present) | Share over real refusals EXP-LEGACY-010; exhaustion fuzzing EXP-LEGACY-041 |
| DEC-LEG-102, DEC-LEG-105 | Differential materiality per anomaly class; writer-variant outputs (surplus/zero local ZIP64 extras, unconditional ZIP64 EOCD, CD digital signature record, reserved flags) per runtime (C5) | Refusal rates on real corpora EXP-LEGACY-010 |

## 4. Candidates

Candidate ids are ledger ids (C§11). The columns of this experiment are *instruments*, not candidates; candidates are
evaluated by applying their rule to the observed matrix.

| Decision | Candidates evaluated from this matrix | Not evaluable here (reason) |
|---|---|---|
| DEC-ECO-047 | status-quo-per-adapter-matrices; dockerized-pinned-incumbent-matrix (realised as the WSL+Windows pinned matrix; Docker unavailable, recorded deviation); native-readers-only-matrix (subset analysis); per-release-regeneration and frozen-dated-snapshot (cost measured: wall time and bytes of a full regeneration); strict-and-broad-divergence-reporting; reuse-zipdiff-dockerized-harness (arm C2-Z) | no-matrix: proposed L0 exclusion (contradicts SPEC §19.3 matrix-per-case freeze); run anyway as the null column |
| DEC-LEG-001 | status-quo-hand-rule-tables; matrix-lookup-refuse-unmatched; narrow-runtime-set | rule-model-fuzz-validated, runtime-in-the-loop, source-derived-models (EXP-LEGACY-005) |
| DEC-LEG-077 | status-quo-refuse; normalise-as-separator; compat-profile-only; literal-component; harmonise-refuse-both; refuse-colon-components (each applied as a projection rule to C8 observations with collision analysis) | — |
| DEC-LEG-078 | status-quo-mixed; faithful-truncation; refuse-all-size-disagreement; import-with-conflict-report; dual-projection-preserve-only; per-profile-reviewed-override | user expectation research (HUMAN-FACING, external) |
| DEC-LEG-079 | status-quo-last-central-order; per-profile-order; refuse-duplicates-in-compat; first-member-profiles; collision-byte-equality-only; collision-casefold-nfc | — |
| DEC-LEG-080 | status-quo-no-policy; add-on-behaviour-change; version-range-metadata; scheduled-reprobe; deprecate-never-remove; remove-after-window (evaluated by drift frequency) | — |
| DEC-LEG-081 | status-quo-hand-written-tests; json-replay-test; expanded-generated-corpus; real-archive-replay; extraction-level-probes; contradiction-as-safety-override; contradiction-new-profile-id | differential-fuzzing (EXP-LEGACY-005) |
| DEC-LEG-082 | status-quo-four-profiles; add-infozip-unzip; add-go-archive-zip; add-dotnet-ziparchive; add-7zip; add-windows-explorer; add-rust-zip2-node-commons; cross-platform-before-freeze; demand-driven-profiles | add-macos-archive-utility: BLOCKED (EXP-LEGACY-080) |
| DEC-LEG-083 | status-quo-central-extents; revalidate-selected-extents; intersection-extent; actual-ratio-cap; refuse-extent-disagreement | streaming-decoder-budget memory/time (EXP-LEGACY-041) |
| DEC-LEG-084 | status-quo-libarchive-only; refuse-in-all-profiles; project-as-runtime-does-with-warning | — |
| DEC-LEG-086 | status-quo-central-sentinels; local-zip64-extra; try-both-conflict; require-agreement | — |
| DEC-LEG-087 | status-quo-separate-evidence; unified-versioned-matrix; native-conformance-integration; real-archive-replay-tier; fuzz-derived-regressions (tier filled by EXP-LEGACY-005) | — |
| DEC-LEG-090 | status-quo-highest-aligned; majority-reader-selection; record-candidates-conflict; refuse-any-extra-signature; profile-dependent-selection | — |
| DEC-LEG-094 | status-quo-cp437; explicit-codepage-override; heuristic-detection; utf8-if-valid-else-cp437; opaque-raw-bytes; refuse-unflagged-non-ascii (cross-implementation CP437 table agreement measured for 0x00-0xFF) | locale corpus study EXP-LEGACY-010 |
| DEC-LEG-102 | status-quo-entrybound-strict; pypi-rule-parity (warehouse column); r3-classification; zipdiff-derived; strict-plus-lint-levels | measured-rejection-threshold (EXP-LEGACY-010) |
| DEC-LEG-103 | status-quo-silent-tolerance; record-gap-observations; scan-hidden-local-headers; recognise-apk-signing-block; refuse-all-gaps-strict; sfx-offset-delta | — |
| DEC-LEG-104 | status-quo-refuse-invalid; ignore-stale-as-omission; ignore-stale-warn; refuse-any-differing-extra; profile-specific-stale-handling | — |
| DEC-LEG-105 | status-quo-refuse; accept-as-refinement; accept-signature-record-as-opaque; pin-appnote-6.3.3; pin-latest-appnote | — |
| DEC-INT-022 | crc-safety-all-profiles; crc-plus-size-invariant; status-quo-present-checks-only (ZIP part) | zstd/xz parts EXP-LEGACY-004 |
| DEC-PLT-010 | status-quo-zip-refuse-divergence; normalise-with-conflict-report; import-as-file; policy-selectable | silent-normalise: proposed L0 exclusion (HC-07 MVT-07(b)); still observed as a projection rule |
| DEC-LEG-012, DEC-LEG-089, DEC-LEG-031, DEC-LEG-032, DEC-LEG-070, DEC-LEG-085, DEC-LEG-092, DEC-LEG-095 | All ledger candidates are evaluated only for their runtime-behaviour premise (what readers do); their selection needs EXP-LEGACY-010/041/051 evidence | — |

Candidates that need Entrybound code changes (for example `revalidate-selected-extents`, `record-then-refuse`) are
evaluated here only as **decision rules applied to observations** (what they would output), not as implementations;
implementations are measured in EXP-LEGACY-041/005.

## 5. Corpus selectors

**Generated case sets** (C§5; `research/tools/legacy/cases/zip_cases.py`; family F20 unless noted; one independence
group per case family; tuning seed `S_T = 2509170101`, validation seed `S_V = 2509170102`):

| Set | Content (primitive classes) | Approx. cases per split |
|---|---|---|
| C0 | the 14 `tools/zip-compat/generate_corpus.py` cases, byte-identical (reproduction; identical in both splits) | 14 |
| C1 | Research III EB-Z001..EB-Z025 regenerated from primitive descriptions | 25 |
| C2 | ZipDiff ambiguity types (USENIX Security 2025 artifact at a pinned commit), regenerated with the artifact's own generator where its licence permits; arm C2-Z runs the artifact harness itself on WSL without Docker if feasible (RAM, storage, runtime and parser coverage recorded) | ~60 |
| C3 | R3 content-substitution and EOCD classes (short_usize, short_usize_zip64, cd_comment, 8-bit comment, duplicate EOCD, zip-in-zip) regenerated from APPNOTE primitives; no third-party fixture file is copied | 20 |
| C4 | Package-ecosystem advisory classes (pip/uv/warehouse wheel parsing GHSA advisories, cited by GHSA id) | 15 |
| C5 | Writer variants: surplus and zero-filled local ZIP64 extras, unconditional ZIP64 EOCD, CD digital-signature record (0x05054b50), reserved/rare flag bits, alignment extra 0xd935, 0xCAFE; plus real writer outputs from Java `ZipOutputStream` (forced ZIP64), Go, 7-Zip `-tzip`, Commons Compress, .NET, Info-ZIP `zip -fz`, PowerShell 5.1 `Compress-Archive` | 40 |
| C6 | Descriptor width: ZIP64 descriptors from Java/Go/7-Zip/Commons/.NET streaming writes to a pipe; hand-built 32/64-bit width mismatches, both-parse ambiguity | 20 |
| C7 | Prefix/suffix/EOCD: shell and PE SFX with absolute and relative offsets (`zip -A`), gaps, hidden local entries, multiple EOF-aligned EOCD, decoy EOCD in comment, concatenated archives, zip-in-zip, `zip -s` multi-disk, non-EOF-aligned complete EOCD | 40 |
| C8 | Names: backslash separators (PowerShell 5.1 producer and hand-built), colon components, every CP437 byte 0x00-0xFF in unflagged names, UTF-8 flag with invalid UTF-8, Unicode Path extra 0x7075 variants (stale CRC, wrong version, flag set plus extra, CD-only, LFH-only, multiple) | 120 |
| C9 | Entry types via external attributes: symlink 0o120777, FIFO 0o010644, char device 0o020644, directory flag with content (EB-Z025 class), trailing-slash directories without attributes, implicit ancestors | 20 |
| C10 | Timestamps: DOS-local versus 0x5455 UT offsets {0, 1, 2, 3600, 36000 s}, NTFS 0x000a, local/central flag differences | 30 |
| C11 | CRC-collision content substitution: STORE and DEFLATE, declared size shorter and longer, forged CRC-32 collision | 16 |
| C12 | Methods and encryption: STORE, DEFLATE, Deflate64, bzip2 (12), LZMA (14), zstd (93) and legacy 20, XZ (95), PPMd (98); ZipCrypto, WinZip AE-1/AE-2, mixed methods, partially encrypted archives | 40 |
| C13 | Duplicates with reordered central directory, case-fold pairs, NFC/NFD pairs | 20 |
| C14 | Compat revalidation adversaries: local size spanning next member, declared-1-KiB DEFLATE bomb (4 GiB actual), overlapping extents selected by descriptor | 10 |

**Real ZIP-family tiers (tuning/validation corpus items only):** `f20-tuning-generated-malicious-structures`,
`f20-tuning-real-libarchive-testsuite` (zip subset), `f11-pypi-binary-wheels-cp312`, `f14-gen-nested-archives-py`,
`f14-jenkins-25683-war`, `f14-legacy-writer-matrix-tuning` (zip members) — tuning;
`f20-validation-generated-malicious-structures`, `f20-validation-real-cpython-archivetestdata` (zip subset),
`f20-validation-real-commons-compress-testdata` (zip subset), `f11-maven-central-jvm-jars`,
`f14-gen-office-documents`, `f14-legacy-writer-matrix-validation` (zip members) — validation.

**Held-out:** placeholder only (C§5): sealed post-freeze seed for C1-C14 and the post-freeze real sample of
EXP-LEGACY-010; one run after Commit C (`decision-method.md` §5.5).

## 6. Environment

`wsl-ubuntu` (ext4 under `/root/eb-research`) for Linux runtimes and the ebr runner; `windows-host` runtimes through
WSL interop with NTFS sandboxes (C§4, C§12). No timing. Docker not used (unavailable; not required).

## 7. Commands and tooling

Tooling to build (`tooling_to_build`): `research/tools/legacy/cases/zip_cases.py`; `research/tools/legacy/materialize_cases.py`;
drivers `research/tools/legacy/readers/{py_zip.py, ZipReaders.java, CommonsReaders.java, bsdtar.py, unzip.py,
sevenzip.py, goread/main.go, DotnetRead/, expand_archive.ps1, explorer_copyhere.ps1, noderead.mjs,
warehouse_check.py}` and harness binary `ebr-legacy-readers` (zip2); `research/tools/legacy/run_matrix.py`
(dispatch, sandboxing, observation writing); `research/tools/legacy/ebound_import.py` (modes: `strict`,
`compat:<profile>`, `preserve:<profile>`; runs `ebound convert <case> <out.eb> [--strict|--compat=P] [--preserve --compat=P] --dry-run`
then without `--dry-run`, and `ebound inspect <out.eb> --json --provenance`); `research/tools/legacy/matrix.py`
(C§7 classification, positive controls C§9, profile agreement, projection-rule evaluation for §4 candidates,
distinguishing-case counts, traceability of rule-table cells, JSON replay); `acquire_runtimes.py`.

```sh
# WSL, repository root /mnt/d/Projects/entrybound/entrybound
/mnt/c/Python313/python.exe research/orchestration/disk_guard.py   # Windows Python (winreg checks) through WSL interop
/root/eb-research/venv/bin/python research/tools/legacy/acquire_runtimes.py --registry research/experiments/_legacy/runtime-registry.json --lock research/experiments/_legacy/runtime-lock.json --only-formats zip
/root/eb-research/venv/bin/python research/tools/legacy/materialize_cases.py --experiment EXP-LEGACY-001 --generator zip_cases --sets C0-C14 --split tuning --seed 2509170101
/root/eb-research/venv/bin/python research/tools/legacy/materialize_cases.py --experiment EXP-LEGACY-001 --generator zip_cases --sets C0-C14 --split validation --seed 2509170102 --append-corpus-items research/experiments/EXP-LEGACY-001/real-items.txt
export PYTHONPATH=research/tools
/root/eb-research/venv/bin/python -m ebr validate --spec research/experiments/EXP-LEGACY-001/spec.yaml
/root/eb-research/venv/bin/python -m ebr run --spec research/experiments/EXP-LEGACY-001/spec.yaml
/root/eb-research/venv/bin/python -m ebr verify-raw --spec research/experiments/EXP-LEGACY-001/spec.yaml --refingerprint
/root/eb-research/venv/bin/python -m ebr normalize --spec research/experiments/EXP-LEGACY-001/spec.yaml
/root/eb-research/venv/bin/python research/tools/legacy/matrix.py --experiment EXP-LEGACY-001 --format zip --reference-set zip --controls research/experiments/EXP-LEGACY-001/cases-manifest.jsonl --out research/normalized/EXP-LEGACY-001/matrix/
```

`real-items.txt` lists the §5 corpus item ids. The tuning split runs first; the validation split runs only after the
tuning matrix, the rule tables under test and the projection rules are frozen in the record (§5.2 look registry).
The frozen observed-outcome file `tools/zip-compat/observed-outcomes-v1.json` is **read, never regenerated in place**
(PROGRESS open item: agents must not run regeneration tools against tracked files); C0 reproduction writes to
`research/normalized/EXP-LEGACY-001/matrix/c0-reproduction.json` and diffs.

## 8. Seeds

`seeds.order = 2509170111`, `seeds.bootstrap = 2509170112`, `seeds.command = 2509170113`; case seeds §5.

## 9. Warmup, replication, adaptive rule

No warmup (no timing). 2 repetitions per (runtime, case set) in different rounds and processes (§4.6 deterministic
metrics) plus per-case repeat-hash of the observation tuple (C§10). No adaptive extension; a nondeterministic cell is
a finding.

## 10. Measurement and metrics

| Metric | Canonical / role | OD | Threshold | Source |
|---|---|---|---|---|
| `import_agreement_fraction` (per frozen profile vs anchor, per platform) | canonical T-20, banded | OD-19 | T-20 | `matrix.py` over observations |
| `import_majority_agreement_fraction` | canonical, descriptive | OD-19 | none | `matrix.py` |
| `ambiguity_refusal_fraction` (strict, ambiguous cases) | canonical, binary | OD-19 | §3.3 | `matrix.py` |
| `adversarial_true_refusal_fraction` (C14 under each profile) | canonical, binary | OD-17 | §3.3 | `matrix.py` |
| `reason_code_specificity_fraction` (Entrybound refusals naming the distinguishing construct) | canonical T-20 | OD-04 | T-20 | `matrix.py` with case `labels` |
| strict/broad divergence counts per runtime pair; distinguishing-case count per candidate runtime; drift count per version pair; rule-table cells traceable to ≥ 2 cases; LOM-evidence-present share of strict refusals; regeneration wall time and bytes | non-canonical descriptive (C§8, secondary only) | — | none | `matrix.py`, ebr `wall_s` |
| detector precision/recall on controls | non-canonical (C§9 gate) | — | must equal 1 | `matrix.py` |

## 11. Normalization

Names compared as raw bytes (`name_b64`); content by SHA-256 and length; extraction by tree fingerprint (`research/corpus/tools/fingerprint.py` `logical_tree_sha256`, cold read). Platform path translation (Windows `\` rendering) is normalised only for the *extraction* tuple and recorded.

## 12. Statistical analysis

Exact counts per evaluation condition (format zip × platform × runtime/profile), AGG-S worst condition headline
(C§8, C§10). Candidate projection rules (§4) are applied deterministically to the frozen observations and scored with
the canonical metrics above; R1 eliminations for binary failures record the case bytes as the counterexample. Real
tiers aggregate L1 item → L2 group → L3 family with §4.9 support check (real ZIP-family tiers here cover F11, F14,
F20 only, so corpus-level verdicts from real tiers alone are not reachable; recorded as a scope limitation and
combined with EXP-LEGACY-010).

## 13. Practical significance

T-20 fractions: `a = 0.5/n_cases` (`decision-method.md` §3.2 T-20); binary metrics §3.3. Descriptive statistics carry no band.

## 14. Sensitivity analysis

C§13: leave-one-case-family-out and leave-one-generator-group-out on every agreement fraction and projection-rule
ranking; platform arm (Windows anchors vs Linux reproductions, same-direction support); version arm (drift pairs);
threshold arm on T-20 `a` (×0.5, ×2); divergence-definition arm (strict vs broad).

## 15. Expected negative results worth recording

Frozen profile disagreeing with its anchor (a frozen-rule defect → new profile id, not in-place change, HC-16);
Linux builds of the anchor versions not reproducing Windows observations (platform-dependent runtime behaviour, which
would make `cross-platform-before-freeze` mandatory); candidate runtimes adding no distinguishing case (argues
against new profiles); zero drift across versions; ZipDiff harness infeasible without Docker; strict refusals that
leave no LOM evidence.

## 16. Threats to validity

Generated cases over-represent constructions the designers thought of (mitigated by EXP-LEGACY-005 fuzzing and the
EXP-LEGACY-010 real tail); WSL interop may alter Windows runtime behaviour (path translation, `\\wsl.localhost` 9P
reads) — mitigated by staging each case onto NTFS before invoking a Windows runtime; Explorer COM automation may not
represent interactive Explorer behaviour; runtime drivers could mis-report (mitigated by positive controls and a
second independent driver for the four anchors: the existing `tools/zip-compat` probes for C0); R3 regeneration may
not match R3 bytes (cases are cited by class, and differences recorded).

## 17. Platform requirements

Linux x86-64 (WSL2) and Windows x86-64 (non-admin, via interop); no network except runtime acquisition downloads;
no privileged host operations (bubblewrap inside WSL as root); macOS columns PLATFORM_BLOCKED (EXP-LEGACY-080);
storage ~20 GB.

## 18. Estimates

Compute ≈ 30 CPU-hours (≈ 1,000 generated cases per split pair plus ≈ 2,500 real archives × 28 columns × 2
repetitions; JVM and .NET columns batch a whole case set per process); disk ≈ 20 GB (runtimes ≈ 8 GB, observations
and sandboxes ≈ 12 GB transient). Agent effort: tooling scripted (Sonnet); execution none; triage of disagreements
and projection-rule analysis judgment (independent session for triage, C§7).

## 19. Expected cost tiers of candidates `[INFERRED]` (computed later, C§11)

New frozen compat profile id: T2. New reason code or LOM conflict class: T2. `record-candidates-conflict` style LOM
changes that alter preserved wire records 30-36: T3 (new record semantics) or excluded if in-place (HC-16).
`opaque-raw-bytes`/`literal-component` name representations touching identity participation: T4. Status-quo rules: T0.
