# EXP-CONF-011 — Legacy adapter conformance tier: expectation classes, asserted outcomes, versioned observed-outcome matrix, and compatibility-model generalisation fuzzing

| Field | Value |
|---|---|
| Status | `NEEDS_TOOLING` (CC§2): adapter case manifest `ebcc-adapters-v1`, Entrybound replay driver over strict and every compatibility profile, matrix binder, structure-aware ZIP/tar variant generator, batch runtime probes invoked through WSL interop; reuses the legacy domain's runtime registry, case generators, observation schema and sandbox (CC§10) |
| Domain / program sections | conformance / §29 (adapter corpora, observed-outcome matrix), §30 (differential fuzzing of compatibility models); legacy §25-§27 own runtime-matrix evidence |
| Decision IDs informed | DEC-ECO-013, DEC-ECO-021, DEC-ECO-046, DEC-ECO-047, DEC-CON-023, DEC-LEG-001, DEC-LEG-007, DEC-LEG-020, DEC-LEG-021, DEC-LEG-022, DEC-LEG-026, DEC-LEG-031, DEC-LEG-050, DEC-LEG-059, DEC-LEG-061, DEC-LEG-062, DEC-LEG-063, DEC-LEG-064, DEC-LEG-066, DEC-LEG-072, DEC-LEG-081, DEC-LEG-082, DEC-LEG-083, DEC-LEG-084, DEC-LEG-087, DEC-LEG-090, DEC-LEG-095, DEC-LEG-102, DEC-LEG-103, DEC-LEG-105, DEC-LEG-107, DEC-PLT-010, DEC-PLT-034 |
| Requirements | REQ-ECO-0078, REQ-ECO-0136, REQ-ECO-0134 (incumbent differential) |
| HC oracles | HC-07 (M19.3 `ambiguity_refusal_fraction`), HC-17 MVT-17(c), HC-06 (`adversarial_true_refusal_fraction` on bombs), HC-05 (compatibility never bypasses safety); proposal for the assigner: `hc_ids` HC-05, HC-06, HC-07, HC-17; `od_ids` OD-19, OD-17 |
| Decision type | not assigned (gate G-A) |
| Method / pre-registration | `decision-method.md` @ `14b977c`; design pre-registration (CC§3) |
| Author / exposure | CC§1 (F11, F14 and F20 real items are used; analyses need a non-exposed session) |
| Depends on | legacy-domain case generators and runtime lock (`research/experiments/_legacy/conventions.md` C4-C5); EXP-CONF-003 (reason-code census for adapter codes) |

## 1. Question

For every legacy adapter case (ZIP, tar family, 7z, compressed-stream wrappers, and native polyglots seen by legacy
readers), what expectation class does the written adapter specification assign to Entrybound's strict import and to each
frozen compatibility profile; does Entrybound return that outcome with a stable reason code and within resource limits;
how are the pinned runtimes' observed outcomes bound to cases as a versioned matrix with positive controls; and do the
frozen compatibility-profile rule tables generalise to structurally varied inputs the way the modelled runtimes behave?

## 2. Hypotheses and falsification

- **H1 (asserted outcomes).** For every case with class `REQUIRED` or `FORBIDDEN` for strict import, Entrybound returns the
  expected outcome class and, where `docs/*-import-v1.md` defines it, the reason code. *Falsified* by one mismatch.
- **H2 (M19.3, HC-07).** Strict import refuses or declares every Research III ambiguity class instantiated in the case set
  (`ambiguity_refusal_fraction` = 1). *Falsified* by one silent resolution.
- **H3 (compatibility never bypasses safety; F-20, DEC-LEG-083, DEC-LEG-084).** Under every compatibility profile, path,
  extent, collision, special-file and resource-safety cases are refused with the documented safety-override code.
  *Falsified* by one projection that admits a hazard.
- **H4 (bombs; HC-06).** Every bomb and decoder-memory-over-policy case is refused under the default adapter limits
  (`adversarial_true_refusal_fraction` = 1). *Falsified* by one acceptance.
- **H5 (model generalisation; DEC-LEG-001, DEC-LEG-081).** For each frozen ZIP profile P modelled on runtime R, on the
  pre-registered structure-aware variant set, P's projection agrees with R's observed outcome or P refuses with a
  documented safety-override code (legacy C7 profile agreement). *Falsified* per profile by one disagreement; the
  agreement fraction and the unmatched conflict patterns are reported.

## 3. Case sets and inputs

- **Generated adapter cases** (legacy conventions C5; tuning instantiation seed `S_T` of the legacy domain, validation
  instantiation `S_V`): ZIP, tar, 7z and wrapper primitive classes including Research III EB-Z001..EB-Z025 and
  EB-T001..EB-T014 regenerated from primitive descriptions; GNU sparse pax 0.0/0.1/1.0 (DEC-LEG-020); v7, old-GNU and GNU
  incremental, star/xstar, Solaris `X`, volume labels (DEC-LEG-064); forward, chained and data-bearing hardlinks
  (DEC-LEG-066); a gzip member above 4 GiB generated as a sparse stream (DEC-LEG-021); Zstandard skippable, seekable-index,
  legacy and size-less frames (DEC-LEG-107); magic collisions (tar names beginning `BZh1`-`BZh9`, `PK\x03\x04`) and nested
  transports (DEC-LEG-072); decoder-memory-over-policy fixtures (`zstd --long=31`, xz with a 1.5 GiB dictionary,
  `bzip2 -9` under small policies; DEC-LEG-007); SFX prefixes (shell and PE), inter-entry gaps, trailing data,
  concatenated archives, multi-disk ZIP and multi-volume 7z (DEC-LEG-031, DEC-LEG-103); APK Signing Block and zipalign
  padding constructed from the published APK signature scheme v2/v3 structure (DEC-LEG-103); multiple EOF-aligned EOCD
  records, decoy EOCD in a comment, zip-in-zip (DEC-LEG-090); ZIP64 surplus or zero-filled extras, unconditional ZIP64
  end records, central-directory digital-signature records, reserved flag bits (DEC-LEG-105); non-regular Unix entry
  types `0o120777` and `0o010644` (DEC-LEG-084); declared-1-KiB DEFLATE bomb and local sizes spanning the next member
  (DEC-LEG-083); directory entries carrying content in ZIP, tar and 7z (DEC-PLT-010); path, link and reparse hazards
  (DEC-PLT-034); 7z malformed and anti items, backslash names, CRC-less streams (DEC-LEG-050); tar authority conflicts
  (pax versus ustar, GNU long name versus ustar, local versus global pax; DEC-LEG-062); non-path adversarial tar cases
  (DEC-LEG-061); 500,000- and 1,000,000-entry tars and a 100,000-file 7z with unknown FilesInfo properties
  (DEC-LEG-059); native `.eb` polyglots of EXP-CONF-004 F-TRUNC opened by legacy readers (DEC-CON-023).
- **Scratch regenerations** of `tools/zip-compat` (14 cases) and `tools/tar-strict` (6 cases) (CC§7).
- **Real items** (strict rejection rates per rule; DEC-LEG-102, DEC-LEG-105, DEC-LEG-026): tuning
  `f20-tuning-real-libarchive-testsuite`, `f14-legacy-writer-matrix-tuning`, `f14-jenkins-25683-war`,
  `f14-apache-maven-3916-bin-tarball`, `f14-gen-nested-archives-py`, `f11-alpine-324-apks`,
  `f20-tuning-generated-malicious-structures`; validation look: `f20-validation-real-commons-compress-testdata`,
  `f20-validation-real-cpython-archivetestdata`, `f14-legacy-writer-matrix-validation`, `f14-gen-office-documents`,
  `f14-pyspark-420-sdist`, `f20-validation-generated-malicious-structures`, `f20-validation-bombs-compressed`.
- **Exclusions** (CC§7): no malo/fastzip fixtures, Go standard-library archive testdata or D. Fifield bomb constructions
  before Phase D. For DEC-ECO-021 only the overlap census between the Research III case descriptions and Entrybound's
  adapter rules is computed; any comparison with malo contents is deferred to Phase D and to the owner-approved
  maintainer contact (listed uncovered for that part).

## 4. Candidates

- **Implementations** (columns): Entrybound `ebound-head` strict import for each format; each frozen ZIP compatibility
  profile (`zip/python-zipfile@3.13.5`, `zip/java-zipfile@21.0.12.1`, `zip/java-zipinputstream@21.0.12.1`,
  `zip/libarchive-bsdtar@3.8.8`); `ebound-baseline` strict import (status quo at `9e44608`). Runtime columns (pinned per the
  legacy runtime lock, CC§10): observations produced by `EXP-LEGACY-*` runs are **reused by SHA-256**; missing observations
  are produced with the legacy drivers (`ebr-legacy-readers`) for the Windows-host frozen versions (CPython 3.13.5,
  JDK 21.0.12.1, bsdtar 3.8.8) and the WSL runtimes (GNU tar 1.35, bsdtar 3.7.2, 7-Zip 23.01, Info-ZIP UnZip 6.00,
  Python 3.12.3, Go 1.22.2 `archive/zip` and `archive/tar` probes, py7zr pinned).
- **Design candidates** of the informed decisions are the ledger's (CC§1: none added); per-case expectations record
  per-candidate outcomes where candidates differ (for example DEC-LEG-020 `refuse-gnu-sparse-keys` versus
  `project-pax-1.0-only`), as in EXP-CONF-004 §5.2.

## 5. Expectation and matrix discipline

- Expectations for Entrybound strict and each profile are written from `docs/zip-import-v1.md`, `docs/tar-import-v1.md`,
  `docs/7z-import-v1.md`, `docs/compressed-stream-import-v1.md`, `docs/zip-compatibility-profiles-v1.md` and
  `docs/legacy-observation-model-v1.md` by an expectation author and checked by a reviewer before the replay (EXP-CONF-004
  §5.3 discipline). `UNRESOLVED` is allowed for adapter cases (SPEC §19.3) and is reported, not hidden.
- **Matrix binding** (`bind_matrix.py`): case manifest `research/conformance/corpus/ebcc-adapters-v1/manifest.json`
  (schema `ebcc.case.v1` with `format`, `runtime_observation_refs`), observed-outcome matrix
  `research/conformance/corpus/ebcc-adapters-v1/observed-outcomes-v1.json` keyed by (case SHA-256, runtime lock SHA-256),
  divergence per legacy C7 (strict and broad), and the positive and negative controls of legacy C9 for every detector
  used. Matrix version = SHA-256 of the manifest plus runtime lock; regeneration reports drift per runtime version.
- **Test binding** (DEC-LEG-087): `emit_regression_tests.py` generates a data-only test list (case id, expected class and
  code per implementation) that a production test can consume; this experiment measures completeness (every case bound)
  and does not modify production tests.

## 6. Model-generalisation fuzzing (H5)

`zipvariants.py` (seed `2809170011` tuning, `2809170012` validation) produces 20,000 ZIP variants per seed from the
generated ZIP cases by structure-aware operators: conflicting local/central sizes and names, descriptor presence and
ambiguity, extra-field duplication and ordering, EOCD placement and multiplicity, comment content, ZIP64 record
combinations, method and flag bits. Each variant is projected by every frozen profile (Entrybound) and observed with the
modelled runtime's exact version on the Windows host (batch probes invoked through WSL interop: one Python process, one
JVM, one bsdtar per batch of 500 inputs). Agreement per legacy C7. A tar variant set (5,000 variants, same operators for
pax/ustar/GNU authorities) is observed with GNU tar 1.35 and bsdtar 3.7.2 as evidence for DEC-LEG-063 (no frozen tar
profile exists; agreement is descriptive).

## 7. Environment and commands

WSL2 Ubuntu 24.04 x86-64 (drivers, generators, WSL runtimes, extraction sandbox per legacy C12); Windows host runtimes
through WSL interop for correctness observations only. No timing (resource use for H4 from GNU time `max_rss_bytes`,
informational; decision-grade import resource measurement belongs to the legacy domain).

```sh
python3 research/tools/legacy/materialize_cases.py --experiment EXP-CONF-011 --split tuning
python3 research/conformance/adapters/build_manifest.py --out research/conformance/corpus/ebcc-adapters-v1
cd /mnt/d/Projects/entrybound/entrybound && export PYTHONPATH=research/tools
/root/eb-research/venv/bin/python -m ebr validate --spec research/experiments/EXP-CONF-011/spec.yaml
/root/eb-research/venv/bin/python -m ebr run --spec research/experiments/EXP-CONF-011/spec.yaml              # replay (raw id EXP-CONF-011)
/root/eb-research/venv/bin/python -m ebr run --spec research/experiments/EXP-CONF-011/spec-model-fuzz.yaml   # H5 (raw id EXP-CONF-011-MF)
python3 research/conformance/adapters/bind_matrix.py --exp EXP-CONF-011 --reuse-legacy-observations research/raw
python3 research/conformance/adapters/emit_regression_tests.py --manifest research/conformance/corpus/ebcc-adapters-v1/manifest.json
```

## 8. Seeds, warmup, replication

Seeds per §3 and §6; ebr seeds CC§12. No warmup. Replay: 2 repetitions + repeat-hash (legacy C10); a runtime whose
repeated observation differs is reclassified as nondeterministic for that case. Model fuzzing: one pass per seed; every
disagreement re-observed twice.

## 9. Metrics (CC§9; legacy C8)

| Metric | Role | Unit | Definition |
|---|---|---|---|
| `ambiguity_refusal_fraction` | binary (HC-07) | fraction | H2, strict import, per format |
| `adversarial_true_refusal_fraction` | binary (HC-06) | fraction | H4, bomb and over-policy cases, default adapter limits |
| `import_acceptance_fraction` | banded (T-20) | fraction | per format and profile, over the pre-registered generated case list and over real items |
| `import_agreement_fraction` | banded (T-20) | fraction | per profile: agreement with the modelled runtime over the variant set (H5) and over unambiguous generated cases |
| `import_majority_agreement_fraction` | descriptive | fraction | agreement with the reference-reader majority |
| `benign_false_refusal_fraction` | banded (T-20) | fraction | strict refusals of real items per format (rejection rate per rule reported beside) |
| H1 mismatches; H3 safety bypasses; native `UNRESOLVED` versus adapter `UNRESOLVED` counts; cases unbound to tests; matrix drift | binary (H1, H3 as HC-17/HC-05 oracle counts) / descriptive | count | §5 tools |

## 10. Normalization and statistical analysis

Deterministic counts. T-20 fractions over fixed case lists compare implementations and candidate expectations with band
`a = 0.5 / n_cases` per evaluation condition (legacy format × profile; objective AGG-S, never averaged across formats).
Real-item rejection rates are per item with family-level aggregation only after the §4.9 minimum-support check; unmet
support narrows the claim to the family and is recorded. The analysis plan needs adoption by a non-exposed session
(CC§1).

## 11. Practical-significance thresholds

§3.3 for binary metrics; T-20 (`thresholds.json` `metric_thresholds.import_acceptance_fraction`,
`…import_agreement_fraction`, `…benign_false_refusal_fraction`) for fractions; descriptive counts have no band.

## 12. Sensitivity analysis

Legacy C13: leave-one-case-family-out and leave-one-generator-group-out (fraction preserving ≥ 0.90, §6.6), runtime-
version arm (Windows frozen versions versus WSL versions of Python and bsdtar), platform arm (Windows versus WSL),
threshold arm (T-20 `a` × 0.5 and × 2). Model fuzzing: agreement with and without variants whose operator touches
compression method or encryption flags.

## 13. Split discipline and held-out placeholder

Tuning instantiation and tuning real items for exploration and expectation refinement; one registered validation look
(validation instantiation `S_V` and validation real items) with frozen expectations and drivers, defined by
`spec-validation.yaml` (raw id `EXP-CONF-011-VAL`) and registered in `research/decisions/validation-looks.jsonl` first. Held-out placeholder: the legacy domain's sealed tranche (legacy C5),
replayed once after Commit C with the frozen expectations; no held-out item is named here.

## 14. Expected negative results worth recording

- Profiles that generalise differently from their runtimes on unmatched conflict patterns (DEC-LEG-001).
- Strict refusal of benign writer variants (ZIP64 surplus extras, signing blocks) at non-trivial real-item rates.
- Adapter reason codes collapsing distinct refusals (for example decoder memory over policy reported as an integrity
  mismatch; DEC-LEG-007 status quo).
- Bombs or observation growth passing default adapter limits.
- Research III classifications contradicted by replay (DEC-LEG-061 T007, T012).

## 15. Threats to validity

- Runtime observations through WSL interop run on NTFS scratch; path semantics differ from Linux runs (platform arm).
- Generated variants cover structure-aware operators, not the full input space; H5 agreement is regression evidence.
- Expectation authoring from adapter documents may inherit their gaps; `UNRESOLVED` is reported rather than decided.
- Reuse of legacy-domain observations depends on identical case bytes and runtime locks; mismatches trigger a re-run.
- Shared base model across expectation author, reviewer and triage (CC§11).

## 16. Cost estimates

| Item | Estimate |
|---|---|
| Compute | 70 CPU-hours: replay ≈ 25 h (cases × Entrybound columns × 2 repetitions, plus missing runtime observations); model fuzzing ≈ 35 h (40,000 ZIP variants × 4 runtimes in batches through interop; 5,000 tar variants × 2 runtimes); >4 GiB and 1.5 GiB-dictionary fixtures ≈ 10 h |
| Disk | 60 GB (large fixtures, real items' scratch copies, observation artifacts) |
| Agent effort | **judgment, medium** (expectation author and reviewer sessions per format; divergence triage); replay and fuzzing scripted |
| Platform | Linux x86-64 (WSL), Windows x86-64 host runtimes via interop; sandboxed extraction; no timing; network once for pinned runtimes (legacy C4) |

## 17. Outcome obligations

`outcome.json` (§10.1); `ebcc-adapters-v1` manifest, matrix and test list; every H1/H3/H4 violation is a defect work item;
`UNRESOLVED` adapter cases are published as such.
