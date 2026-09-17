# EXP-LEGACY-041: legacy import limit defaults — benign false refusals from real requirement vectors, bomb-corpus refusal correctness and cost, compat revalidation and record-then-refuse prototypes

| Field | Value |
|---|---|
| Status | **NEEDS_TOOLING** (policy files per candidate default set; requirement-vector extractor; bomb generators `bombs.py`; `ebr-legacy-import` with policy overrides, counting allocator and in-process refusal timers; compat revalidation and record-then-refuse prototypes behind `research-internals` with identity proof; depends on EXP-LEGACY-010 requirement vectors and EXP-LEGACY-004 decoder multipliers) |
| Domain / program sections | legacy / §25 (with §30 adversarial inputs, §13 limits, §35) |
| decision_ids | DEC-LEG-022, DEC-LEG-093, DEC-LEG-007, DEC-LEG-083, DEC-LEG-059, DEC-LEG-085, DEC-LEG-095, DEC-CON-006, DEC-CON-001 |
| Decision types (proposed) | EMPIRICAL (false refusals, refusal cost, bound conformance) with FORMAL parts (declared-bound formulas; incumbent default citations) |
| OD / HC (proposed) | OD-17 (M17.1-M17.4, M17.6), OD-08 (M08.1, M08.3); HC-06 (MVT-06(a) exact allocator oracle), HC-05, HC-07 |
| Shared rules / L7 / gates | C§ header, C§2 (in-process `refusal_cpu_s` needs calibration of `time_cpu` in-process timers; allocator metrics are exact) |
| Timing | `timing: true` for arm B/E/F/G refusal costs (in-process timers); arm A is an offline deterministic computation |

## 1. Question

Which default values for the adapter-specific import limits (ZIP, tar, compressed streams, 7z, preservation, conversion
evidence) refuse the fewest benign real artifacts while refusing every pre-registered bomb before exceeding the declared
memory, how cheaply (bytes read, CPU, allocation) does each default refuse, does it admit legitimate ultra presets, and
do compat profiles need re-validation of profile-selected extents and actual expansion ratios?

## 2. Hypotheses

- **H1 (status-quo false refusals).** The status-quo constants refuse more than 0.5% of unambiguous artifacts in at least
  one EXP-LEGACY-010 stratum, concentrated in one or two limits `[INFERRED]`. *Falsified by* ≤ 0.5% everywhere.
- **H2 (bomb floor).** Every candidate default set except `unlimited-reference` refuses every bomb
  (`adversarial_true_refusal_fraction` = 1) with allocator peak within its declared memory; at least one candidate fails
  on at least one bomb class (predicted: declared-size DEFLATE or overlapping-extent bombs under compat profiles).
- **H3 (ultra presets, DEC-LEG-022 / SPEC §25.1 #23).** The status-quo constants refuse legitimate `7z -mx9 -md=1536m`
  archives and `zstd --ultra -22 --long=31` tars; `ultra-admitting` admits them within a 2 GiB declared envelope.
- **H4 (header precheck, DEC-LEG-007).** Candidates that check dictionary/window requirements from headers refuse
  over-policy streams with `refusal_bytes_read` below 64 KiB and `refusal_alloc_peak_bytes` below 1 MiB; the status quo
  reads more before refusing. *Falsified by* the status quo being non-inferior on both.
- **H5 (compat revalidation, DEC-LEG-083).** `revalidate-selected-extents` plus `actual-ratio-cap` refuse every C14 case
  and refuse no more than 0.1% of real descriptor-bearing ZIPs under compat (EXP-LEGACY-010 tuning ZIP strata).
- **H6 (record-then-refuse, DEC-LEG-095).** `record-then-refuse` keeps `bounded_memory_growth_bytes` within the evidence
  budget on evidence bombs. *Falsified by* one exceedance.
- **H0.** All candidate default sets are equivalent in false refusals and bomb handling.

## 3. Decisions informed

| Decision | Supplied here |
|---|---|
| DEC-LEG-022 | Corpus distributions per limit; peak RSS and time at candidate values; bomb refusal and resource use; sensitivity of refusal rate to each default |
| DEC-LEG-093 | ZipImportPolicy defaults: false refusals, bomb handling (Fifield overlap, 42.zip class, declared-size bombs), expansion-ratio unit harmonisation |
| DEC-LEG-007 | Outcome class, reason code and pre-decode refusal cost for over-policy decoder memory; measured multipliers (from EXP-LEGACY-004) as a candidate |
| DEC-LEG-083 | Compat revalidation candidates on C14 fixtures and on real descriptor ZIPs |
| DEC-LEG-059 | Adversarial provenance-size growth under candidate caps |
| DEC-LEG-085 | Bomb behaviour per ZIP method decoder (Deflate64, bzip2, LZMA, zstd, XZ, PPMd) under each default set |
| DEC-LEG-095 | Resource-exhaustion behaviour of record-then-refuse prototypes |
| DEC-CON-006 | Import-budget part: corpus distributions, hostile-suite cost, false-refusal rates, memory and time at limits, incumbent default comparison |
| DEC-CON-001 | Preserved-legacy-source-bytes and nested bombs against ResourceBudget accounting; false refusals of legitimate preservation archives under the default metadata budget |

## 4. Candidates (policy files `research/experiments/EXP-LEGACY-041/policies/<id>.json`)

| Decision | Candidates |
|---|---|
| DEC-LEG-022 | status-quo-constants / status-quo-defaults (values read from code with `path:line`); corpus-percentile (tuning p99.9 × 2 per limit, rounded up to a power of two, frozen before validation); memory-envelope (256 MiB and 1 GiB variants); incumbent-aligned (libarchive 3.8.8, 7-Zip 26, Go `archive/zip`, CPython 3.13 `tarfile` data filter and `zipfile`, Info-ZIP, Commons Compress defaults, each cited at pinned source lines); tiered-presets (conservative/default/permissive); ultra-admitting; absolute-decoded-budgets; per-format-ratio; caller-declared-budgets |
| DEC-LEG-093 | status-quo-experimental; measured-percentile-defaults (= corpus-percentile); memory-derived-defaults (= memory-envelope); harmonised-ratio-unit (per-entry, per-archive, per-folder units evaluated); conservative-embedder-defaults; cli-limit-flags (surface only; flag census in EXP-LEGACY-070) |
| DEC-CON-006 | bootstrap-generous-status-quo; go-extract-style-conservative (defaults of the go-extract project, cited); corpus-percentile-derived; context-presets; host-resource-derived; declared-with-hard-ceiling; no-defaults-caller-supplied; archive-declaration-raises-limits |
| DEC-LEG-007 | status-quo-integrity-mismatch; header-precheck-policy-refused; dedicated-reason-code; policy-out-of-evidence; measured-multipliers |
| DEC-LEG-083 | status-quo-central-extents; revalidate-selected-extents; intersection-extent; actual-ratio-cap; streaming-decoder-budget; refuse-extent-disagreement |
| DEC-LEG-059 | status-quo-independent-caps; add-provenance-byte-bound; derive-observation-cap; count-shared-observations; aggregate-resolutions |
| DEC-LEG-095 | status-quo-fail-fast; record-then-refuse; partial-lom-on-error; dry-run-only-completeness; hybrid-safety-first |
| DEC-CON-001 | status-quo-eight-fields; add-declared-fields; caller-policy-only-new-limits; fixed-format-bounds-registry; hole-logical-len-bound (import-side accounting only) |

Proposed L0 exclusions (C§11): `archive-declaration-raises-limits` (SPEC §11.5: the archive controls neither budget;
`docs/random-access-v1.md` L38-39: declarations only narrow caller limits); `unlimited-when-unset` of DEC-LEG-059
(`docs/legacy-preservation-v1.md` L86-90). Both are cheap and are run as references. `host-resource-derived` is
measured; its host dependence of refusal outcomes is reported (HC-08 relevance for CLI defaults noted for review).

## 5. Corpus selectors

- **Arm A (benign requirement vectors):** EXP-LEGACY-010 tuning and validation artifacts (per-artifact requirement
  vectors), plus tuning items `f11-pypi-binary-wheels-cp312`, `f11-ubuntu-noble-debs`, `f14-jenkins-25683-war`,
  `f14-legacy-writer-matrix-tuning`, `f20-tuning-real-libarchive-testsuite`; validation items `f11-maven-central-jvm-jars`,
  `f11-fedora-44-rpms`, `f14-pyspark-420-sdist`, `f14-legacy-writer-matrix-validation`,
  `f20-validation-real-commons-compress-testdata`. Legitimate ultra presets built from `f05-tuning-loghub-hdfs-v1`
  (tuning) and `f03-validation-bat-cargo-vendor` (validation): `7z -mx9 -md=1536m`, `zstd --ultra -22 --long=31` tars.
- **Arm B (bombs, `research/tools/legacy/cases/bombs.py`, F20, `S_T = 2509174101`, `S_V = 2509174102`):** Fifield
  overlapping-file ZIP bomb (Fifield, "A better zip bomb", USENIX WOOT 2019) with and without extra-field quoting;
  42.zip-class nesting (depth 5, fan-out 16); declared-size DEFLATE bomb (1 KiB declared, 4 GiB actual); C14 overlap
  cases; 1,000,000 zero-byte entries; 65,535 extra bytes per entry; evidence bomb (maximal LFH/CD contradictions per
  entry × 100k entries); gzip of 16 GiB zeros; triple-nested gzip; xz with 1536 MiB dictionary; zstd windowLog 31;
  bzip2 -9 under a 1 MiB policy; 4 GiB skippable frame; tar with a 1 TiB GNU sparse member, 10M pax records in one
  header, pax size 2^63, 1M unresolved forward hardlinks; 7z with a 1536 MiB LZMA dictionary on tiny data, declared
  NumFolders 2^31, inflated unpack sizes, 1M empty files; per-method ZIP bombs for Deflate64/bzip2/LZMA/zstd/XZ/PPMd;
  preservation bombs (4 GiB preserved source; observation bomb under `--preserve`). Corpus items
  `f20-tuning-bombs-dense`, `f20-tuning-generated-malicious-structures` (tuning) and `f20-validation-bombs-compressed`,
  `f20-validation-generated-malicious-structures` (validation).
- **Arm E (compat revalidation):** EXP-LEGACY-001 C14 cases; EXP-LEGACY-010 tuning ZIP artifacts with data descriptors.
- Held-out: placeholder (C§5).

## 6. Environment

`wsl-ubuntu`; bombs run inside bubblewrap with a cgroup memory ceiling of 8 GiB and a 600 s wall limit per sample
(ceiling hits are recorded as bound violations of the candidate, not as infrastructure faults); in-process timers for
refusal costs; `st-v` = {2}.

## 7. Commands and tooling

Tooling: `research/tools/legacy/requirements.py` (requirement vectors from census output and `ebr-legacy-import
--dry-run --report-requirements`); `research/tools/legacy/policy_eval.py` (arm A offline evaluation, exact counts per
limit and candidate); `cases/bombs.py`; `ebr-legacy-import --policy-file <json> --compat-revalidation <candidate>
--refusal-strategy <candidate> --report-refusal-cost`; policy files and `research/experiments/EXP-LEGACY-041/incumbent-defaults.md`
(citations).

```sh
/mnt/c/Python313/python.exe research/orchestration/disk_guard.py   # Windows Python (winreg checks) through WSL interop
export PYTHONPATH=research/tools
/root/eb-research/venv/bin/python research/tools/legacy/requirements.py --from EXP-LEGACY-010 --split tuning --items research/experiments/EXP-LEGACY-041/real-items-tuning.txt --out research/normalized/EXP-LEGACY-041/requirements-tuning.parquet
/root/eb-research/venv/bin/python research/tools/legacy/policy_eval.py --requirements research/normalized/EXP-LEGACY-041/requirements-tuning.parquet --policies research/experiments/EXP-LEGACY-041/policies --derive corpus-percentile --out research/normalized/EXP-LEGACY-041/arm-a-tuning/
# freeze policies (commit), register the validation look, then:
/root/eb-research/venv/bin/python research/tools/legacy/requirements.py --from EXP-LEGACY-010 --split validation --items research/experiments/EXP-LEGACY-041/real-items-validation.txt --out research/normalized/EXP-LEGACY-041/requirements-validation.parquet
/root/eb-research/venv/bin/python research/tools/legacy/policy_eval.py --requirements research/normalized/EXP-LEGACY-041/requirements-validation.parquet --policies research/experiments/EXP-LEGACY-041/policies --out research/normalized/EXP-LEGACY-041/arm-a-validation/
/root/eb-research/venv/bin/python research/tools/legacy/materialize_cases.py --experiment EXP-LEGACY-041 --generator bombs --split tuning --seed 2509174101
/root/eb-research/venv/bin/python research/tools/legacy/materialize_cases.py --experiment EXP-LEGACY-041 --generator bombs --split validation --seed 2509174102 --append-corpus-items research/experiments/EXP-LEGACY-041/bomb-items.txt
/root/eb-research/venv/bin/python -m ebr run --spec research/experiments/EXP-LEGACY-041/spec.yaml --timing
/root/eb-research/venv/bin/python -m ebr verify-raw --spec research/experiments/EXP-LEGACY-041/spec.yaml --refingerprint
/root/eb-research/venv/bin/python -m ebr normalize --spec research/experiments/EXP-LEGACY-041/spec.yaml
```

Arm C (peak RSS and time at candidate limits on legitimate large inputs) runs inside EXP-LEGACY-040 with these policy
files as its policy arm.

## 8. Seeds

`order 2509174111`, `bootstrap 2509174112`, `command 2509174113`; bomb seeds §5.

## 9. Warmup, replication, adaptive rule

Arm A: deterministic, run twice, byte-identical outputs required. Bomb arms: warmups 2 (small inputs), §4.6 small/medium
tier rounds (10, step 10, cap 30/20) with the precision rule for `refusal_cpu_s`; allocator peaks and refusal bytes are
exact (repeat-hash); any bound exceedance is a counterexample on first occurrence.

## 10. Metrics

| Metric | Canonical / role | OD | Threshold |
|---|---|---|---|
| `benign_false_refusal_fraction` (per limit, per stratum, overall) | banded T-20 (primary) | OD-17 | T-20 |
| `adversarial_true_refusal_fraction` | binary | OD-17 | §3.3 |
| `bounded_memory_growth_bytes` (allocator peak ≤ declared policy memory) | binary | OD-08 | §3.3 |
| `refusal_bytes_read` | banded T-02 | OD-17 | T-02 |
| `refusal_cpu_s` (in-process) | banded T-05 | OD-17 | T-05 |
| `refusal_alloc_peak_bytes` | banded T-06 | OD-17 | T-06 |
| `declared_tightness_ratio` (declared requirement / measured allocator peak on admitted legitimate inputs) | banded T-23 | OD-17 | T-23 |
| `reason_code_specificity_fraction` | banded T-20 | OD-04 | T-20 |
| `scale_limits` | descriptive | OD-17 | none |

## 11. Normalization

Arm A counts are exact per artifact; each candidate's refusal of an artifact is attributed to every limit it exceeds and
to the first limit in the production check order. Refusal costs are paired per (bomb, candidate) within rounds.

## 12. Statistical analysis

Arm A: T-20 differences between candidates per stratum and pooled (strata never averaged into bands; §4.9 applies with
EXP-LEGACY-010 groups and families). Bomb arms: binary floor first (R1/R2), then banded refusal costs with §4.10 item
intervals; R4 dominance among survivors with confirmatory comparisons declared after tuning; R9 policy partition by
context preset (`context-presets`, `tiered-presets`) subject to expressibility; R10 simplicity tie-breaks with the power
requirement.

## 13. Practical significance

T-02, T-05, T-06, T-20, T-23 per `decision-method.md` §3.2; binary §3.3.

## 14. Sensitivity analysis

§6.4 threshold perturbation; percentile arm (p99, p99.9, p99.99 and margins ×1.5/×2/×4 for `corpus-percentile`);
stratum-weighting arm; leave-one-stratum-out; per-format arm; memory-ceiling arm (8 GiB versus 4 GiB cgroup).

## 15. Expected negative results worth recording

No candidate both admits ultra presets and keeps a small declared envelope (a real trade-off to document); corpus-
percentile defaults dominated by one ecosystem; compat revalidation refusing many legitimate Java descriptor archives;
record-then-refuse exceeding evidence budgets.

## 16. Threats to validity

Public-ecosystem bias of requirement vectors (EXP-LEGACY-010 §16); bombs are known classes (novel bombs come from
fuzzing in EXP-LEGACY-005 and the security domain); cgroup ceilings inside WSL may kill before the allocator records the
peak (the ceiling hit itself is recorded).

## 17. Platform requirements

Linux x86-64 WSL2 with cgroup v2 memory control inside the VM; ≥ 60 GB free; no host privileged operations.

## 18. Estimates

Compute ≈ 30 CPU-hours (bombs × 20 candidate policies × modes × rounds) plus ≈ 2 CPU-hours arm A; disk ≈ 30 GB. Agent
effort: incumbent-default citation judgment; policy freezing judgment; execution none.

## 19. Expected cost tiers `[INFERRED]`

Changing default values within existing policy fields: T1; new policy fields (provenance byte bound, per-folder ratio
unit): T2; new reason codes (header-precheck refusal): T2; new ResourceBudget fields in the frozen descriptor: excluded in
place (HC-16), T3/T4 as a new structure.
