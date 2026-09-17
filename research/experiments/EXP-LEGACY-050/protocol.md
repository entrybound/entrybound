# EXP-LEGACY-050: legacy preservation — size overhead, exact-source recovery, completeness, tar/7z recomposition, re-projection, composition with encryption and signing, and signed legacy containers

| Field | Value |
|---|---|
| Status | **NEEDS_TOOLING** (research accessors for preserved-source recovery and `observe()` dumps; prototypes `planner-compressed-source`, `dedup-against-decoded`, `strict-preserve`, `reproject-from-lom`, `detached-encrypted-provenance`, `cli-recover-command` behind `research-internals` with identity proof; recomposition sweep `recompose.py`; signed-container fixtures with `jarsigner`, Ubuntu `apksigner` and `zipalign` packages) |
| Domain / program sections | legacy / §25 (with §22.5 composition, §23 sidecars, §27 export of signed containers) |
| decision_ids | DEC-LEG-096, DEC-LEG-097, DEC-LEG-098, DEC-LEG-060, DEC-LEG-025, DEC-LEG-044, DEC-CRY-024, DEC-CON-010, DEC-CRY-091, DEC-LEG-103 |
| Decision types (proposed) | EMPIRICAL (bytes, recovery, match rates) with FORMAL parts (identity participation of preservation records per `docs/legacy-preservation-v1.md`); crypto composition designs EXTERNAL |
| OD / HC (proposed) | OD-05 (M05.1 preservation cost), OD-02 (exact recovery), OD-03, OD-20 (M20.1), OD-15 (signature state); HC-02, HC-07, HC-10, HC-14 (composition), HC-16 |
| Shared rules / L7 / gates | C§ header, C§2 |
| Timing | `timing: false` (decision-grade preserve time overhead is the `preserve-*` arm of EXP-LEGACY-040) |

## 1. Question

What does preservation (exact source plus structured LOM evidence, feature 0x4000) cost in bytes per storage design,
does exact-source recovery hold across layouts, profiles, repacks and signing, is the preserved evidence complete, how
much would tar, compressed-tar and 7z preservation cost (including whether compressor bytes can be recomposed rather
than stored), can preserved evidence be re-projected under another profile, how does preservation compose with
encryption, how often do archives need the 0x2000/0x4000 feature bits, and what happens to signed JAR/APK/ZIP
containers through import, export and recovery?

## 2. Hypotheses

- **H1 (cost, DEC-LEG-096).** Status-quo raw preservation roughly doubles `artifact_bytes` for DEFLATE-dominated ZIPs
  (ratio ≥ 1.8) and `dedup-against-decoded` reduces the overhead below 5% only for STORE-dominated ZIPs `[INFERRED]`.
- **H2 (recovery).** Recovered source bytes equal the original SHA-256 for every preserved archive in INDEXED and STREAM
  layouts, all four profiles, after `repack` and after `ebound sign` (`roundtrip_mismatches` = 0). *Falsified by* one
  mismatch.
- **H3 (completeness).** Every field emitted by `observe()` for a preserved ZIP appears in the preserved LOM records.
  *Falsified by* one missing field class.
- **H4 (recomposition, DEC-LEG-025/060).** Fewer than 50% of real `.tar.gz` and fewer than 20% of `.tar.xz`/`.tar.zst`
  compressed streams are byte-reproducible by any pinned encoder configuration `[INFERRED]`, so tar-split side data
  alone cannot give exact recovery for most compressed tars.
- **H5 (strict+preserve, DEC-LEG-097).** A strict+preserve prototype yields LAI and PCR equal to strict import and an AUX
  that differs only by preservation records. *Falsified by* any LAI/PCR difference.
- **H6 (re-projection, DEC-LEG-044).** Re-projection from preserved LOM under another ZIP profile equals direct import
  under that profile on every EXP-LEGACY-001 case. *Falsified by* one disagreement.
- **H7 (signed containers, DEC-CRY-091).** Every jarsigner-signed JAR and v2/v3-signed APK fails signature verification
  after strict import and `zip/portable-v1` export, while exact recovery from preservation verifies; the status quo
  declares no signature loss in the receipt.
- **H0.** Preservation is cheap, complete and exact everywhere; signed containers survive export.

## 3. Decisions informed

| Decision | Supplied here |
|---|---|
| DEC-LEG-096 | Size overhead per storage design; exact recovery rate in INDEXED and STREAM; completeness audit; regenerated-ZIP equivalence rate (for `claim-regenerated-roundtrip`) |
| DEC-LEG-097 | Identity test of strict+preserve; wire impact (whether a mode field is needed, C1 census) |
| DEC-LEG-098 | Recovery through the ZIP-module function versus a format-neutral API and CLI prototype with source-digest verification (success and failure-mode matrix) |
| DEC-LEG-060 | Preserved-size overhead estimates for tar, compressed tar and 7z; composition with encryption, signing and repack |
| DEC-LEG-025 | tar-split side-data size and recomposition exactness including compressor bytes |
| DEC-LEG-044 | Re-projection agreement with direct import; adapter-version drift on preserved observations |
| DEC-CRY-024 | Behaviour and residual evidence for encrypting archives carrying 0x2000/0x4000 under each composition candidate |
| DEC-CON-010 | Share of imported archives requiring 0x2000 and 0x4000 per mode; record bytes per object |
| DEC-CRY-091 | Signed JAR/APK/ZIP round trips, detection heuristic precision/recall, preservation-based exact reconstruction |
| DEC-LEG-103 | APK Signing Block and zipalign padding handling through import, export and recovery |

## 4. Candidates

| Decision | Candidates | Realisation |
|---|---|---|
| DEC-LEG-096 | status-quo-raw-record; planner-compressed-source; dedup-against-decoded; external-sidecar-source; claim-exact-recovery-only; claim-regenerated-roundtrip | prototypes (storage); claims evaluated by recovery and regeneration rates |
| DEC-LEG-097 | status-quo-compat-coupled; strict-preserve; evidence-only-sidecar; any-mode-preserve | prototypes |
| DEC-LEG-098 | status-quo-zip-module-fn; format-neutral-api; cli-recover-command; export-target; inspect-only | prototypes |
| DEC-LEG-060 | status-quo-zip-only; strict-plus-preserve; compat-then-preserve; tar-and-streams-only; sidecar-instead; exact-source-without-lom | size estimates and composition tests |
| DEC-LEG-025 | status-quo-zip-snapshot-only; full-snapshot-all-formats; tar-split-side-data; external-sidecar-only; not-offered | size and recomposition estimates |
| DEC-LEG-044 | no-reprojection; reproject-from-source-bytes; reproject-from-lom; chained-provenance | prototypes |
| DEC-CRY-024 | status-quo-refuse; private-records-new-feature (C1 census only); two-step-convert-then-encrypt; detached-encrypted-provenance; silently-drop-provenance | `silently-drop-provenance` is a proposed L0 exclusion (ledger constraint: features that cannot compose are refused, never silently dropped); run only to record what would be lost |
| DEC-CON-010 | status-quo-always-set-cumulative; minimal-activation; auxiliary-as-ro-compat-or-compat (share-based premise evidence only) | census |
| DEC-CRY-091 | status-quo-no-detection; detect-and-warn; preserve-exact-reconstruction; record-declared-loss; refuse-normalizing-profile | rules over round-trip observations; detection prototype |
| DEC-LEG-103 | status-quo-silent-tolerance; recognise-apk-signing-block; record-gap-observations | APK arm |

## 5. Corpus selectors

- **ZIP-family (tuning):** `f11-pypi-binary-wheels-cp312`, `f14-gen-nested-archives-py`, `f14-jenkins-25683-war`,
  `f14-legacy-writer-matrix-tuning` (zip members), EXP-LEGACY-001 C0-C14 tuning cases; **(validation):**
  `f11-maven-central-jvm-jars`, `f14-gen-office-documents`, `f14-legacy-writer-matrix-validation` (zip members),
  EXP-LEGACY-001 validation cases.
- **tar/compressed tar/7z (tuning):** `f14-apache-maven-3916-bin-tarball`, `f11-alpine-324-apks`, `f11-ubuntu-noble-debs`
  (data tars), EXP-LEGACY-010 tuning T-strata artifacts (≤ 2,000 sampled by seed `2509175001`), EXP-LEGACY-003 S9
  tuning 7z outputs; **(validation):** `f14-pyspark-420-sdist`, `f11-fedora-44-rpms` (payload streams), EXP-LEGACY-010
  validation T-strata (≤ 1,000, seed `2509175002`), EXP-LEGACY-003 S9 validation outputs.
- **Signed containers** (`research/tools/legacy/cases/signed_containers.py`; `S_T = 2509175003`, `S_V = 2509175004`):
  JARs signed with `jarsigner` (RSA and EC test keys generated for the experiment), APKs built from a minimal
  generated package, aligned with Ubuntu `zipalign` and signed with Ubuntu `apksigner` (v1, v2, v3 schemes), tampered
  variants; real signed candidates detected in the ZIP-family items above.
- Held-out: placeholder (C§5).

## 6. Environment

`wsl-ubuntu`; `ebound-research` build with prototypes off proven identical to `ebound-head`; experiment-only keys
generated locally (never user credentials).

## 7. Commands and tooling

Tooling: `research/tools/legacy/preserve_run.py` (arms P1-P9), `recompose.py` (decode each compressed stream and
re-encode with GNU gzip 1.12 `-n -1..-9`, pigz, Python `gzip` `mtime=0`, zstd 1.5.5/1.5.7 levels 1-19 with and without
checksum and `-T0`, xz 5.4.5/5.8 presets 0-9 and `-e` single/multi-thread, bzip2 1-9; 7-Zip 23.01/26 solid settings for
7z folders; record first byte-identical configuration), `tar_split_size.py` (exact header/padding/trailer bytes from the
census), `observe_dump` accessor, `signed_verify.py` (`jarsigner -verify -strict`, `apksigner verify --verbose`),
`feature_bits_census.py`.

```sh
/mnt/c/Python313/python.exe research/orchestration/disk_guard.py   # Windows Python (winreg checks) through WSL interop
bash research/harness/internals-identity-check.sh
export PYTHONPATH=research/tools
/root/eb-research/venv/bin/python research/tools/legacy/materialize_cases.py --experiment EXP-LEGACY-050 --generator signed_containers --split tuning --seed 2509175003 --from-experiments EXP-LEGACY-001,EXP-LEGACY-003,EXP-LEGACY-010 --append-corpus-items research/experiments/EXP-LEGACY-050/real-items-tuning.txt
/root/eb-research/venv/bin/python research/tools/legacy/materialize_cases.py --experiment EXP-LEGACY-050 --generator signed_containers --split validation --seed 2509175004 --from-experiments EXP-LEGACY-001,EXP-LEGACY-003,EXP-LEGACY-010 --append-corpus-items research/experiments/EXP-LEGACY-050/real-items-validation.txt
/root/eb-research/venv/bin/python -m ebr run --spec research/experiments/EXP-LEGACY-050/spec.yaml
/root/eb-research/venv/bin/python -m ebr verify-raw --spec research/experiments/EXP-LEGACY-050/spec.yaml --refingerprint
/root/eb-research/venv/bin/python -m ebr normalize --spec research/experiments/EXP-LEGACY-050/spec.yaml
/root/eb-research/venv/bin/python research/tools/legacy/preserve_run.py --analyze --experiment EXP-LEGACY-050 --out research/normalized/EXP-LEGACY-050/
```

## 8. Seeds

`order 2509175011`, `bootstrap 2509175012`, `command 2509175013`; sampling and fixture seeds §5.

## 9. Replication

2 repetitions with repeat-hash of all bytes and identities; recomposition sweeps run once per stream per configuration
(deterministic encoders; any encoder whose repeated output differs is recorded as nondeterministic and excluded from
match claims).

## 10. Metrics

| Metric | Canonical / role | OD | Threshold |
|---|---|---|---|
| `artifact_bytes` (preserve versus compat, per storage design) | banded T-01 (primary for DEC-LEG-096 storage) | OD-05 | T-01 |
| `side_data_bytes` (preservation and provenance records) | banded T-02 (primary where the decision is about that component) | OD-05 | T-02 |
| `roundtrip_mismatches` (exact source recovery) | binary | OD-02 | §3.3 |
| `migration_identity_conformance_fraction` (strict+preserve AUX-only change; repack/sign identity rules) | binary | OD-20 | §3.3 |
| `import_agreement_fraction` (re-projection versus direct import) | banded T-20 | OD-19 | T-20 |
| `export_acceptance_fraction` (signature verifiers accept after export or recovery) | banded T-20 | OD-19 | T-20 |
| `receipt_completeness_fraction` (signature loss declared) | binary where declaration is claimed | OD-20 | §3.3 |
| recomposition match rate, tar-split side-data share, feature-bit shares, detection precision/recall, completeness-audit missing classes | non-canonical descriptive | — | none |

## 11-12. Normalization and analysis

Byte costs per item exact (§4.13); banded comparisons across storage designs per §4.9-§4.12 on ZIP-family real items
(groups from the manifest; the ZIP-family set spans F11/F14 only, so corpus-level support is not met and verdicts are
scoped to those families, recorded limitation). Binary claims per case. Recomposition and feature-bit statistics are
descriptive with exact intervals.

## 13. Practical significance

T-01, T-02, T-20 per `decision-method.md` §3.2; binary §3.3.

## 14. Sensitivity analysis

Layout arm (INDEXED/STREAM); profile arm (four profiles); encoder-set arm for recomposition (CLI-only versus CLI plus
library encoders); stratum arm for tar samples; signing-scheme arm (v1/v2/v3).

## 15. Expected negative results worth recording

Recovery failing after repack or signing (defect); completeness gaps in preserved LOM; recomposition rates too low for
tar-split preservation; re-projection disagreeing with direct import; signed JARs verifying after export (would weaken
refuse-normalizing-profile).

## 16. Threats to validity

Recomposition success depends on the pinned encoder set (unlisted historical encoders would raise match rates);
generated signed containers use test keys and minimal content; real signed-JAR prevalence depends on EXP-LEGACY-010
frames.

## 17. Platform requirements

Linux x86-64 WSL2; apt packages `apksigner`, `zipalign` (universe, no licence click-through); no privileged host
operations; storage ~80 GB.

## 18. Estimates

Compute ≈ 80 CPU-hours (recomposition sweeps dominate); disk ≈ 80 GB. Agent effort: prototype judgment; execution none.

## 19. Expected cost tiers `[INFERRED]`

New preserved-source storage encodings: T3 (record semantics; never in place for frozen types 30-36, HC-16);
strict-preserve with a mode field: T3; recovery CLI command: T2; tar/7z preservation: T3-T4 (new record types, identity
participation in AUX); encrypted private records: T4 (cryptographic construction, EXTERNAL review).
