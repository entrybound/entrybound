# EXP-PLANNER-006: Profile parameter response surface and fitted thirteen-parameter profiles

| Field | Value |
|---|---|
| Domain | planner (program §9, §10) |
| Index status | NEEDS_TOOLING (TP-06 `param-plan` with the thirteen SPEC §6.0a parameters and arm parameters, TP-02, TP-03 `params`/`fit-tnb`, TP-08, TP-15 including G1-PSUB-64, TP-18 cgroup decode; EXP-PLANNER-008's tuning decode-cost table for the `min_decode_rate` arm) |
| Document state | Phase C design; `record.md` after gate G-A |
| Shared conventions | `research/planner/README.md` |
| Runner specs | `spec.yaml` (fast and balanced on PSUB-256 tuning sub-items); `spec-dense-extreme.yaml` (dense and extreme on G1-PSUB-64); `spec-lowmem.yaml` (decode under cgroup v2 memory ceilings) |

## 1. Pre-registration header

- **decision_ids:**
  - DEC-CMP-035 (primary): profile definition form, numeric values, stability.
  - DEC-CMP-045: Zstandard window per profile.
  - DEC-CMP-016: profile-scaled decoder ceilings; planner eligibility mirroring validation bounds.
  - DEC-CMP-006: `clustering_mode` per profile.
  - DEC-CMP-013: demonstrated value of creation overrides.
  - DEC-CMP-019: size gain available from environment-adaptive planning; recorded resource model bytes.
  - DEC-CRY-028: ratio loss of each cross-file restriction.
  - DEC-ACC-003: ratio impact of file-locality ordering.
  - DEC-ACC-029: ratio loss of windowed planning.
- **Decision type (proposed):** EMPIRICAL. The human-facing parts of DEC-CMP-013, the leakage parts of DEC-CRY-028 and the latency parts of DEC-ACC-003 are routed elsewhere (§3).
- **Proposed `od_ids`:** OD-05 (superiority), OD-06, OD-07, OD-08, OD-10 (guards); OD-17 (`declared_tightness_ratio`, low-memory arm); OD-25 (C6 flag counts, override arm). **Proposed `hc_ids`:** HC-02, HC-06, HC-09, HC-11, HC-13, HC-14 (arm R only as context), HC-16.
- **Frame:** profile scope, constrained form (Appendix B.1). This experiment is where DEC-CMP-035 **fixes the numeric profile bounds** that R2 then screens in every profile-scope decision (R2 last bullet).
- **Author / L7:** README §10.

## 2. Question and hypotheses

- **Question.** For each profile, how does `artifact_bytes` respond to each SPEC §6.0a parameter and to the arm parameters? Which numeric assignment is the tightest that is non-inferior in size? Does a profile defined by that explicit assignment match or beat the frozen candidate tables within the guards, and are the fitted values stable under the method's perturbations?
- **H1.**
  1. For every profile there is a fitted thirteen-parameter assignment that is `NON_INFERIOR` to the frozen tables on `artifact_bytes` with every guard intact.
  2. The fitted bounds respect the SPEC §6 ordinal order (fast tightest … extreme loosest) for `max_decode_memory`, `max_access_granularity`, `max_lookback` and `compress_budget`.
  3. Every fitted value is preserved in at least 90% of selection-stability bootstrap replicates (`sensitivity.min_fraction_preserving_winner`).
  4. Windows above 1 MiB (modelled-only) gain less than the T2 minimum gain in every profile.
  5. Overrides' best-per-item upper bound clears the T2 minimum gain in at most one WM-* mix.
  6. Host-adaptive planning gains less than the T2 minimum gain in every profile.
  7. Disabling cross-file context costs more than 1 band unit on F04, F17 and F18 in dense and extreme.
  8. Windowed planning at 64 MiB costs less than 1 band unit at corpus level.
  9. Path (file-locality) ordering is `NON_INFERIOR` to digest order in balanced.
  10. Declared decode memory is never below the measured decode peak (`declared_tightness_ratio` ≥ 1), and no archive whose declaration fits the cgroup ceiling is OOM-killed.
- **H0.** Every parameter change is `EQUIVALENT` to the status quo, which leaves no basis for numeric values except the status quo's implied ones.
- **Falsification.** Each H1.k has its complementary verdict at look 1, or, for modelled-only screens, the modelled interval. H1.2 fails on any ordinal inversion, which is recorded as a negative result against the SPEC ordering. H1.10 fails on a single counterexample: an HC-06 violation artifact, committed.

## 3. Decisions informed

| Decision | Supplied here | Elsewhere |
|---|---|---|
| DEC-CMP-035 | Fitted values per profile from corpus measurements; R3-R10 for status-quo tables against thirteen-parameter, reduced, weighted and lexicographic forms; selection churn under §6.3 grid factors for the weighted form's internal weights (the ledger's "±25%" is covered by grid factors 0.75 and 1.25); §6.7 stability of fitted values; explainability census (C1 words, parameter-to-behaviour coverage) | Timing guards: EXP-PLANNER-007; held-out: 012 |
| DEC-CMP-045 | `artifact_bytes` against window log per profile (modelled-only above 1 MiB); declared decoder window and working set | Measured decoder peak memory across libzstd and pure-Rust decoders, tool refusal of large windows: codecs domain (§8.6) |
| DEC-CMP-016 | Size effect of profile-scaled ceilings; pack-failure fraction on benign items (`benign_false_refusal_fraction`) under status quo against `planner-mirrors-validation`; cgroup low-memory decode outcomes and `declared_tightness_ratio` | preflate/jxl peak memory against input size and megapixels: reconstruction domain (§8.8); native low-memory devices: EXP-PLANNER-011 (BLOCKED) |
| DEC-CMP-006 | Ratio effect of `clustering_mode` and base order per profile; planning work units; declared STREAM window per ordering | Extraction locality and remote range counts: remote domain (§12); similarity method itself: crossfile (§8.3) |
| DEC-CMP-013 | Upper bound of the size value of `--lookback N`, a supplied dictionary and a chunk-size override per profile and WM-* mix; R6 screen at T2 | First-use evaluation, dictionary untrusted-input handling, adoption demand: ecosystem (§32-§33), crossfile (§8.4) |
| DEC-CMP-019 | Upper bound of the gain from host-adaptive planning; bytes of a recorded resource model | Reproducibility matrix across platforms and threads: EXP-PLANNER-009 and scale (§14); `verify --reproducible`: integrity (§24) |
| DEC-CRY-028 | `artifact_bytes` loss of no-cross-file, same-owner-only, mandatory-compression and profile-opt-in restrictions (planning measured on plaintext, padding held fixed) | Chosen-plaintext co-packing leakage, padding effect: crypto (§22.3, §22.5), EXTERNAL review |
| DEC-ACC-003 | Ratio impact of file-locality (path) ordering | Hot-set and trace-derived ordering, time to first useful read: remote (§12) |
| DEC-ACC-029 | Ratio loss of windowed similarity, cohort and dictionary planning (global exact dedup kept) | Peak RSS against input size, streaming CDC with spill, external dedup index: scale (§13) |

## 4. Candidates

### 4.1 Parameter grid (one-at-a-time around each profile's status quo)

Each point sets one parameter via `ebr-planner param-plan --base-profile P --set …`. **R** marks a replay cell (from EXP-PLANNER-002 traces, then materialized by TP-02). **P** marks a re-plan cell (TP-06).

| Point id | Parameter setting | Kind | Profiles |
|---|---|---|---|
| sq (reference) | none (frozen v6) | P | all |
| codecs-zstd-only / codecs-no-lzma2 | `codec_candidates` subsets | R | all / dense, extreme |
| eval-sampled | `evaluation_mode=sampled` (sh-k1 of EXP-PLANNER-003) | R | all |
| budget-tnb | `compress_budget` = EXP-PLANNER-003 TNB value | R | all |
| transforms-none / -structural / -all | `transform_classes` | R | all |
| winmax-1m / -4m / -8m | `max_decode_window` (filters LZMA2 dictionary sizes) | R | dense, extreme |
| zwin-22 / zwin-25 / zwin-27 / zwin-long | Zstandard window log 22/25/27, or long-distance matching at log 27 (DEC-CMP-045) | R, **modelled-only** (ebr-codec) | all |
| decrate-t1 / -t2 / -t3 | `min_decode_rate` floor at the EXP-PLANNER-008 model's tier boundaries | R | all |
| dict-none / dict-supplied | `dictionary_policy` (supplied = `train_dictionary` output from a disjoint tuning item of the same family, at most 112 KiB, file `/root/eb-research/derived/planner/supplied-dicts/<family>.dict`; families with one tuning group get none) | P | balanced, dense, extreme |
| lookback-0 / -1 / -2 / -4 / -8 | `max_lookback` | P | balanced (1-8 are the override arm), dense, extreme |
| gran-256k / -1m / -4m / -16m | `max_access_granularity` | P | all |
| cluster-none / -cheap / -full | `clustering_mode` (cheap = type-then-path ordering within directories and TP-05 categories) | P | all |
| order-path / order-type-path | base physical order (DEC-CMP-006 `path-order-base`, DEC-ACC-003 `file-locality-order`) | P | all |
| xfile-none / xfile-owner | `cross_file_scope` (owner = first path component) | P | balanced, dense, extreme |
| no-store | STORE forbidden (DEC-CRY-028 `mandatory-compression-under-encryption`) | P | all |
| pwin-16m / pwin-64m | `planning_window_bytes` (similarity, cohorts and dictionaries within deterministic byte windows; exact dedup global) | P | all |
| adaptive-host | `adaptive=host`: compress budget unbounded, ceilings up to 50% of host memory, no planning window | P | all |
| ceil-16m / -64m / -128m / -256m / -1g | `max_decode_memory` (reconstruction working sets included) | P | all |
| mirror-validation | planner eligibility checks every decoder validation bound and falls back instead of failing | P | balanced, dense, extreme |
| chunk-* | `chunk_size_target/min/max`, `chunking_strategy_search`: reused from the EXP-PLANNER-005 matrix, not re-run | – | all |

### 4.2 DEC-CMP-035 candidates

| candidate_id | Definition | Expected tier |
|---|---|---|
| status-quo-candidate-tables (reference) | frozen v6 profile enums and tables | status quo |
| spec-thirteen-parameters | All thirteen parameters set. Bound parameters (`max_decode_memory`, `max_decode_window`, `max_access_granularity`, `max_lookback`, `compress_budget`, `min_decode_rate`) take the **TNB value**: the tightest grid value whose tuning corpus-level `artifact_bytes` point estimate is within 1 T-01 band unit of the profile's loosest grid value. Categorical parameters take the smallest-size level under the fitted bounds, ties to the SPEC §6.1-6.4 literal level, then to the status quo | T1 (planner IDs) plus T2 if any parameter becomes recorded or user-visible |
| reduced-parameter-set | Only `codec_candidates`, `transform_classes`, `max_decode_memory` and `max_access_granularity` are fitted (TNB and categorical rule); the rest stay at status quo | T1 |
| weighted-objective | The planner minimizes `stored_bytes + a·modelled_decode_ns + b·work_units` with integer (a, b) from the grid {0, 1, 4, 16} × {0, 1, 4, 16} per MiB, fitted on tuning to the smallest size subject to modelled decode and work within 1 band unit of the status quo | T1 |
| lexicographic-constraints | Filter the bounded-oracle universe by the fitted decode-memory and access-granularity bounds, then minimize size (SPEC §5.4 literally, other parameters status quo) | T1 |
| critic-proposed-n | R0 item 6 slot | – |

### 4.3 Arm candidates (DEC-CMP-045, -016, -006, -013, -019, CRY-028, ACC-003, ACC-029)

These map onto grid points:

| Decision | Candidate → point(s) |
|---|---|
| DEC-CMP-045 | status-quo-log20 → sq; per-profile-windows → zwin-22 (balanced), zwin-25 (dense), zwin-27 (extreme); long-mode-dense-extreme → zwin-long; window-sized-to-chunk → zwin at ceil(log2(max Chunk)); rely-on-cross-file-dictionaries → sq |
| DEC-CMP-016 | status-quo-limits → sq; profile-scaled-ceilings → ceil-16m (fast), ceil-64m (balanced), ceil-256m (dense), ceil-1g (extreme); lower-default-ceiling → ceil-128m (balanced default); planner-mirrors-validation → mirror-validation; measured-limits → the TNB fit of `max_decode_memory` |
| DEC-CMP-006 | status-quo-digest-order-with-cohorts → sq; spec-clustering-modes → cluster-none (fast), cluster-cheap (balanced), cluster-full (dense, extreme); path-order-base → order-path; type-then-path-ordering → order-type-path (balanced); cohort-only-dense-extreme → cluster-none (balanced) |
| DEC-CMP-013 | status-quo-profile-only → sq; spec-flags-bounded and lookback-override-only → best of lookback-1…8 per item (upper bound) plus dict-supplied; expert-policy-file → best grid point per item (upper bound); research-only-feature-flag → sq for users (no product value measured) |
| DEC-CMP-019 | status-quo-always-deterministic → sq; explicit-flag-adaptive-default → adaptive-host; always-deterministic-recorded-model → sq plus modelled descriptor bytes of the resource model (C1 counter); always-deterministic-unrecorded → sq |
| DEC-CRY-028 (ratio arm) | status-quo-same-planner → sq; no-cross-file-context-when-encrypted → xfile-none; same-owner-cross-file-only → xfile-owner; mandatory-compression → no-store; profile-opt-in → xfile-none in fast and balanced, sq in dense and extreme; require-maximum-padding → not measured here (crypto domain) |
| DEC-ACC-003 (ratio arm) | status-quo-cohort-digest-order → sq; file-locality-order → order-path; hot-set-prefix and trace-derived-order → not measured here (remote domain) |
| DEC-ACC-029 (ratio arm) | status-quo-in-memory → sq; windowed-planner → pwin-16m, pwin-64m; other architectures → scale domain |

- **Excluded:** none at this stage. `runtime-measurement` for decode rate is excluded in EXP-PLANNER-008 (R1, SPEC §5.7). Parameters that TP-06 cannot express in a cell are reported in `unsupported_parameters` and make that cell non-decision-grade.

## 5. Corpus

- **Fast and balanced:** PSUB-256 tuning sub-items (all 20 families).
- **Dense and extreme:** **G1-PSUB-64**, meaning the PSUB-64 sub-item of the first tuning item of each independence group in `SHA-256("G1" ‖ item_id)` order (defined in `research/planner/README.md` §3). All 20 families, with every tuning group represented once.
- **Low-memory arm:** G1-PSUB-64 archives of all four profiles.
- **Windowed-planning supplement:** full medium and large tuning items for balanced, `pwin-256m` and `pwin-1g` (secondary: tier-limited).
- **Look 1:** fitted candidates and W ∪ S on the validation counterparts.
- **Weights:** `workload-mix.json`. Minimum support per §4.9.

## 6. Environment and platform

- `wsl-ubuntu` x86-64. The low-memory arm needs WSL root to create a cgroup v2 subtree (`/sys/fs/cgroup/ebr-planner-lowmem`) and set `memory.max`. These are privileged operations inside the WSL VM only; no host setting changes.
- The look-1 timing guards for fitted profiles run in EXP-PLANNER-007.
- No network and no external review, except the crypto routing of DEC-CRY-028.

## 7. Commands

```sh
python3 research/planner/tools/make_psub.py --k-mib 64 --splits tuning --one-per-group --group-key G1 --manifest research/corpus/manifest.json \
  --out-root /root/eb-research/derived/planner/g1-psub-64 --out-manifest research/planner/derived/g1-psub-64.json
python3 research/planner/tools/replay.py params --traces /root/eb-research/raw-side/EXP-PLANNER-002/traces --split tuning \
  --decode-cost-table research/normalized/EXP-PLANNER-008/decision/reference-table.json \
  --assignments-out /root/eb-research/raw-side/EXP-PLANNER-006/assignments --out research/normalized/EXP-PLANNER-006/replay/
for s in spec.yaml spec-dense-extreme.yaml spec-lowmem.yaml; do
  PYTHONPATH=research/tools /root/eb-research/venv/bin/python -m ebr run --spec research/experiments/EXP-PLANNER-006/$s
  PYTHONPATH=research/tools /root/eb-research/venv/bin/python -m ebr verify-raw --spec research/experiments/EXP-PLANNER-006/$s --refingerprint
  PYTHONPATH=research/tools /root/eb-research/venv/bin/python -m ebr normalize --spec research/experiments/EXP-PLANNER-006/$s
done
python3 research/planner/tools/replay.py fit-tnb --normalized research/normalized/EXP-PLANNER-006 --out research/normalized/EXP-PLANNER-006/replay/fitted-profiles.json
# the fitted-* candidates (spec-thirteen, reduced, weighted, lexicographic) are then materialized by the fitted-* cells of both specs
```

**Ordering constraint.** The fitted-* cells in both specs read `fitted-profiles.json`, which is committed **before** those cells run. It is a tuning artifact.

## 8. Seeds, warmup, replication

Seeds: order 260917061, bootstrap 260917062, command 260917063; selection bootstrap 260917994. Deterministic: 0 warmups, 2 repetitions, repeat-hash. Low-memory arm: 3 runs per (archive, ceiling) (§4.6 deterministic, plus one extra because OOM behaviour can be scheduling-sensitive). Any OOM kill at a ceiling at or above the declared requirement is committed as an HC-06 violation artifact without rerun (§4.13 rule applied to resource claims).

## 9. Metrics

| Metric | OD | Role | Family | Threshold | Source |
|---|---|---|---|---|---|
| `artifact_bytes` | OD-05 | primary superiority | size | T-01 | TP-06 / TP-02 |
| `access_granularity_bytes` | OD-10 | guard | size | T-09 | TP-06 |
| `encode_wall_s` | OD-06 | guard (work units on tuning; timed in EXP-PLANNER-007 look 1) | time_wall | T-03 per profile | EXP-PLANNER-007 |
| `decode_wall_s` | OD-07 | guard | time_wall | T-04 | EXP-PLANNER-007 |
| `alloc_peak_bytes` | OD-08 | guard | memory_peak | T-06 | TP-08 |
| `declared_tightness_ratio` (declared decode memory / measured `alloc_peak_bytes`) | OD-17 | primary for the DEC-CMP-016 low-memory claim; values below 1 are HC-06 violations | other | T-23 | spec-lowmem |
| `benign_false_refusal_fraction` (tuning items whose pack fails with a planner-caused error) | OD-17 | primary for DEC-CMP-016 `mirror-validation` | other | T-20 | TP-06 outcome |
| `dictionary_bytes`, `index_bytes`, `side_data_bytes`, `metadata_bytes` | OD-05 | diagnostic | size | T-02 | TP-06 byte accounting |
| `bounded_memory_growth_bytes` | HC-06 | binary (OOM at or above declaration) | correctness | objective M08.3 | spec-lowmem |
| declared window, declared STREAM window, work units, `unsupported_parameters` | – | descriptive; R2 bound inputs | other | none | TP-06 |
| `roundtrip_mismatches`, repeat-hash | OD-02, HC-13 | binary | correctness | §3.3 | TP-06 |

## 10. Normalization and statistical analysis

1. **Response curves (tuning, exploratory).** Per profile and parameter: the corpus-level `artifact_bytes` ratio against sq with §4.10 intervals, per family and parent tier, plus guard deltas.
2. **TNB fits.** As defined in §4.2, frozen and committed before any fitted-* cell runs. Ordinal consistency (H1.2) is checked mechanically.
3. **DEC-CMP-035 look 1.** For each profile, W = the smallest-size DEC-CMP-035 candidate meeting guards on tuning; S = the non-eliminated rest. Confirmatory R4 on validation with confidences from `bootstrap.*`, followed by R6 (computed tiers), R7, R8/R9 per profile, and R10. After this decision's provisional selection is frozen, the fitted bounds of W become the **R2 profile bounds** for every other profile-scope decision (README §6), applied at their own look-1 analyses.
4. **Modelled-only screens** (zwin-*, the override and adaptive upper bounds): R6 elimination only (README §7). Overrides and adaptive mode are expected T2 (new user-visible flag or mode). The band is `cost_tiers.multipliers.T2` on T-01. The scope for overrides is each WM-* mix (Appendix B.2), because a flag serves chosen workloads.
5. **Ratio arms** (DEC-CRY-028, DEC-ACC-003, DEC-ACC-029, DEC-CMP-006): per profile, the corpus and family ratios against sq with verdict classes (§4.11). These are supplied as evidence lines to the owning domains. Within this domain they select only for DEC-CMP-006, under B.1 with R3-R10.
6. **Low-memory arm:** R1 or R2 screening only, per decision-method §3.3 (binary), with `declared_tightness_ratio` graded above 1 (T-23 band).
7. **Explainability census for DEC-CMP-035:** normative words describing each profile form (C1 counter), and the fraction of observed behavioural differences between profiles attributable to a parameter value. Descriptive; enters R10 key 2 via C1.

## 11. Sensitivity analysis

- B.1 frame (no weights on the decision): §6.4 threshold perturbation; §6.5 arms (equal weights, leave-one-family-out, leave-three-families-out, family Dirichlet, WM-* mixes, tier-stratified, byte-weighted); §6.7 selection-stability bootstrap applied both to the DEC-CMP-035 selection and to **each fitted TNB value** (H1.3). Bars per §6.6.
- The weighted-objective candidate's internal weights are perturbed on the §6.3 grid (`sensitivity.grid_factors`). Selection churn is reported (share of Chunks changing plan, and the size change in band units) as the ledger's "selection churn under perturbation".
- **TNB tolerance arm:** fits repeated at 0.5 and 2 band units (secondary, labelled).

## 12. Expected negative results worth recording

- Fitted bounds invert the SPEC ordering (for example dense needing no more decode memory than balanced).
- The thirteen-parameter form reproduces the tables' sizes but adds normative words without measured gain, so the status quo wins by R10.
- Larger Zstandard windows buy nothing at Entrybound Chunk sizes.
- Overrides and adaptive planning fail the T2 screen.
- Disabling cross-file context under encryption is cheap except on F04 and F17, which bounds the cost of the safest crypto option.
- Windowed planning is nearly free.
- Planner-caused pack failures on crafted JPEGs under the status quo (DEC-CMP-016 defect evidence).

## 13. Threats to validity

- One-at-a-time grids miss interactions. The fitted joint assignment is confirmed at look 1, but untested interactions away from it remain.
- G1-PSUB-64 for dense and extreme reduces within-group replication and truncates large items.
- The replay decode-rate filters depend on EXP-PLANNER-008's model.
- Zstandard window cells are modelled-only.
- cgroup memory accounting includes the page cache (it measures VM memory, not a device ceiling); native low-memory devices are BLOCKED (EXP-PLANNER-011).
- `unsupported_parameters` may exclude cells.
- Designer L7 exposure.

## 14. Cost and effort

- **Compute:**
  - `spec.yaml`: 81 (point, profile) cells for fast and balanced on PSUB-256 × 2 repetitions, about 100 CPU-hours.
  - `spec-dense-extreme.yaml`: 100 cells for dense and extreme on G1-PSUB-64 (about 3 GiB) × 2 repetitions, about 400 CPU-hours.
  - `spec-lowmem.yaml` (16 cells × 3 runs): about 10 CPU-hours.
  - Windowed supplement: about 20 CPU-hours.
  - Look 1 W ∪ S: about 60 CPU-hours.
  - With TP-17 at 16 workers, about 35 hours of wall time in total.
- **Disk:** G1-PSUB-64 about 4 GB; raw about 3 GB; transient about 20 GB.
- **Agent effort:** scripted tooling and execution; judgment for fits review, ordinal-consistency interpretation and the R4-R10 write-up.

## 15. Gates and status

- **Tooling:** TP-02, TP-03, TP-06, TP-08, TP-15 (with `--one-per-group`), TP-18; EXP-PLANNER-008's tuning table, or the codec-class tier substitute recorded as a deviation.
- **Gates:** G-A, G-B, cost scripts and rule simulation before look 1, EXP-PLANNER-007 timing guards for `DECIDED`.
- **Held-out:** EXP-PLANNER-012.
