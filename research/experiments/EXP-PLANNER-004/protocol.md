# EXP-PLANNER-004: Candidate tables, gating probe, gain margins, dictionary acceptance and stage conflicts

| Field | Value |
|---|---|
| Domain | planner (program §9, §10) |
| Index status | NEEDS_TOOLING (TP-01, TP-02, TP-03 `tables`/`gating`/`margins`/`dictionary-rule`/`fit-tables`, TP-05, TP-06 `stage_order`, TP-08; look-1 decode timing needs runner additions and calibration) |
| Document state | Phase C design; `record.md` after gate G-A |
| Shared conventions | `research/planner/README.md` |
| Runner spec | `spec.yaml` (deterministic materialization and re-plan cells, tuning). Look-1 decode timing of W ∪ S runs inside EXP-PLANNER-007's windows (`spec-decode-look1.yaml` generated there by `make_timing_spec.py`) |

## 1. Pre-registration header

- **decision_ids:** DEC-CMP-034 (arm A), DEC-CMP-012 (arm B), DEC-CMP-030 (arm C), DEC-CMP-020 (arm D, acceptance-rule part only), DEC-CMP-039 (arm E), DEC-CMP-009 (arm F, win frequency and modelled minimum-gain screen).
- **Decision type (proposed):** EMPIRICAL. Arm F contributes only a modelled R6 screen; the §5.8 criteria are codecs-domain, EXTERNAL where licences need interpretation.
- **Proposed `od_ids`:** OD-05 (superiority), OD-06, OD-07, OD-08, OD-10 (guards); OD-17 for arm B's adversarial work. **Proposed `hc_ids`:** HC-01 (arm E: one physical representation per Chunk), HC-02, HC-06, HC-09 (dictionary dependencies), HC-11, HC-13, HC-16.
- **Frame:** profile scope, constrained form (Appendix B.1; README §6).
- **Author / L7:** README §10.

## 2. Question and hypotheses

- **Question.** Holding the search strategy at the status quo, which candidate table, transform gate, gain-margin rule, dictionary acceptance rule and region/cohort conflict rule gives the smallest `artifact_bytes` per profile within the guards? Which frozen candidates never pay, and could any density codec clear the registered-extended minimum gain even in its best modelled case?
- **H1.**
  - A1: `pruned-tables` is `NON_INFERIOR` to the v6 tables in every profile, with at least one third fewer evaluations in dense and extreme.
  - B1: `always-try` is not `DISTINGUISHABLE_BETTER` than the status-quo probe in any profile. `retuned-probe` is `NON_INFERIOR` at fewer evaluations.
  - C1: `zero-margin-smallest-wins` is not `DISTINGUISHABLE_BETTER` than the status-quo margins on `artifact_bytes` and fails the `decode_wall_s` guard in at least one family.
  - D1: charging decoder and access cost (`decoder-and-access-cost-charged`) is `NON_INFERIOR` on `artifact_bytes`.
  - E1: `joint-complete-cost` is `DISTINGUISHABLE_BETTER` than cohorts-first on F12 photo-library items with duplicates, and identical elsewhere.
  - F1: no external codec's modelled upper-bound gain clears the T2 minimum-gain band in dense or extreme.
- **H0.** Every variant is `EQUIVALENT` to the status quo. The status quo is then selected by R10 in every arm and profile.
- **Falsification.** A1 fails if pruned tables are not non-inferior at look 1. B1 fails on an `always-try` superiority verdict. C1 fails if zero margin is distinguishably better, or passes every decode guard. D1 fails if the charged rule is not non-inferior. E1 fails if joint costing is not distinguishably better on F12, or differs anywhere outside JPEG-bearing items. F1 fails if a modelled upper bound clears the T2 band.

## 3. Decisions informed

| Decision | Supplied here | Elsewhere |
|---|---|---|
| DEC-CMP-034 | R3-R10 over table variants; per-candidate win frequency and marginal size (from EXP-PLANNER-002 traces); decode speed and memory of selected plans (look-1 timing) | Status-quo usage census: EXP-PLANNER-001; held-out: 012 |
| DEC-CMP-012 | Probe false-negative and false-positive rates per group; work saved by gating; size regret against always-try; adversarial mis-categorisation cost on F20 | – |
| DEC-CMP-030 | Size against decode-cost trade-off curves over margins; permutation invariance and orphan plan bytes; perturbation stability | – |
| DEC-CMP-020 | Acceptance-rule counterfactuals charging decoder memory and access fetch bytes; break-even reuse count | Sample count and size, trainer, supplied and per-category dictionaries, remote fetch cost: crossfile (§8.4) and remote (§12) domains |
| DEC-CMP-039 | Region-suppression frequency by reason; archive-size delta for region-first, cohorts-first and joint; audit schema bytes | JPEG reconstruction payoff and bounds: reconstruction domain (§8.8) |
| DEC-CMP-009 | Win frequency of LZMA2 and external codecs under exhaustive search in dense and extreme; modelled R6 screen | §5.8 criteria, decoder footprint, portability: codecs domain (§8.6) |

## 4. Candidates

### Arm A (DEC-CMP-034), per profile

| candidate_id | Definition |
|---|---|
| status-quo-v6-tables (reference) | Frozen v4-v6 tables (`docs/codec-transform-v1.md`) |
| pruned-tables | Backward elimination on tuning: repeatedly remove the candidate whose removal raises corpus-level modelled `artifact_bytes` least. Stop before the cumulative increase exceeds 0.25 T-01 band units. STORE is never removed |
| spec-literal | fast: {STORE, Zstd 1}, no transforms. balanced: {Zstd 1, 3, 5} plus the structural transforms, evaluated with sh-k1 sampling (EXP-PLANNER-003 definition). dense: the bounded-oracle universe on full data. extreme: the unbounded-oracle universe (EXP-PLANNER-002) |
| zstd-only-levels | STORE plus Zstd {1} / {1, 3, 5} / {5, 9, 15, 19} / {9, 15, 19, 22}; transforms paired only with Zstd |
| lz4-fast-lzma2-extreme-only | v6 tables with LZ4 removed except in fast, and LZMA2 removed except in extreme |
| corpus-fitted-tables | Forward selection from {STORE} over the bounded-oracle universe on tuning. Add the shape with the largest corpus-level size reduction per added `bytes_examined`. Stop when the reduction is below 0.25 band units or the table's work units exceed the status quo's |

### Arm B (DEC-CMP-012), balanced, dense and extreme (fast never searches transforms)

| candidate_id | Definition |
|---|---|
| status-quo-sampling-probe (reference) | 4,096-sample probe; thresholds 192 symbols, 12,000 ppm, 20,000 ppm |
| retuned-probe | Same probe with a truly adjacent-byte repeat measure. Thresholds are chosen on the tuning grid distinct ≤ {128, 160, 192, 224} × max-frequency ≥ {8k, 12k, 16k, 20k} ppm × adjacent ≥ {10k, 20k, 30k, 40k} ppm, minimizing modelled `artifact_bytes`; ties go to fewer evaluations |
| signature-categoriser | TP-05 `signature-v0` categories select transform families (structured-numeric and model-weights: delta8 and byte-shuffle-2/4/8; others none) |
| entropy-estimate-gate | Integer order-0 entropy of the 4,096-byte sample, in milli-bits per byte via an integer log2 table. Transforms are tried when entropy is at or below a threshold chosen on the tuning grid {5000, 6000, 7000, 7500} mbit; STORE-only at or above 7,900 mbit |
| always-try | No gate |
| per-category-scope | TP-05 category restricts both codecs and transforms (incompressible: STORE; text: Zstd and LZMA2; numeric: transforms plus Zstd and LZMA2; media: STORE and Zstd 1) |

- **Excluded:** `extension-based-categoriser`. R1: SPEC §5.2 ("categorise by bytes, never extension") and `docs/planner-v1.md` determinism lines 23-24. This is the ledger's existing exclusion; independent sign-off is pending (R1(b)).

### Arm C (DEC-CMP-030), all profiles

| candidate_id | Definition |
|---|---|
| status-quo-margins (reference) | Frozen v1-v6 margins, charges and tie rules |
| amortized-plan-cost | Shared TransformPlan records charged once per archive when comparing candidates |
| decode-cost-weighted-margin | Margin = status-quo margin × integer decode-cost class of the candidate (STORE 0, LZ4 1, Zstd 2, LZMA2 8; reconstruction 8) divided by 2 (Zstd equals status quo) |
| zero-margin-smallest-wins | Pure smallest complete cost; STORE wins exact ties only |
| corpus-fitted-margins | Absolute {0, 16, 32, 64, 128, 256} B × relative {0, 50, 100, 200} bp, chosen on tuning to minimize modelled `artifact_bytes` subject to the modelled `decode_wall_s` staying within 1 band unit of the status quo. Modelled decode time comes from the EXP-PLANNER-008 reference table on tuning chunks |
| order-independent-region-costing | Region section header charged after selection, proportional to selected regions; orphan TransformPlans pruned |

### Arm D (DEC-CMP-020 acceptance rule), balanced, dense and extreme

| candidate_id | Definition |
|---|---|
| status-quo-absolute-128 (reference) | Complete cost must beat independent by more than 128 B; independent wins ties |
| relative-plus-absolute-margin-1 | Margin max(128 B, 1%) |
| relative-plus-absolute-margin-2 | Margin max(256 B, 2%) |
| decoder-and-access-cost-charged | Complete cost plus `dictionary_bytes × min(members, 4) / 4`, charging the expected re-fetch for independently accessed members, plus the decoder dictionary working set rounded up to 64 KiB (counted as bytes) |
| minimum-reuse-count-n (n ∈ {4, 8, 16}) | Accept only when at least n Chunks reference the dictionary |
| no-dictionaries | Never accept |

### Arm E (DEC-CMP-039), profiles with region candidates

| candidate_id | Definition |
|---|---|
| status-quo-cohorts-first-conservative (reference) | Frozen v6 |
| region-first | Regions selected before similarity cohorts; region members excluded from cohorts (TP-06 `--set stage_order=region-first`) |
| joint-complete-cost | Region savings compared with the displaced cohort and dedup savings; the lower total wins (`--set stage_order=joint`) |
| split-audit-reasons | Status-quo selection with separate audit reasons for dedup sharing and for dictionary or group conflicts (T2: new reason codes); measured for audit bytes and explain accuracy only |

- **Excluded:** `duplicate-representation-for-shared-chunk`. R1 by HC-01 (I1: one frame per Chunk digest and single region membership; `ecf/container.rs:1856-1858`, `eam/validate.rs:419-481`). Ledger exclusion; independent sign-off pending.

### Arm F (DEC-CMP-009)

Candidates are the codecs domain's (status-quo LZMA2-only, none, lzma2-xz, brotli, bzip2, ppmd, and others). This arm evaluates only those implemented in `ebr-codec` (brotli, bzip2, PPMd, LZMA2) as modelled additions to the dense and extreme tables.

- **Reference candidate** in every arm: the status-quo rule of the same profile.
- **R0 item 6:** a critic slot `critic-proposed-n` exists per arm.
- **Expected tiers:** A-D T1 (new planner ID); E T1, and `split-audit-reasons` T2; F T2 (registered extended codec). Tiers are computed by the §14 item 7 scripts.

## 5. Corpus

- **Arms A-D and F:** PSUB-256 tuning sub-items (replay over EXP-PLANNER-002 traces, then materialization of every cell). Validation in look 1.
- **Arm E:** re-plan on PSUB-256 tuning sub-items of every family. Items without JPEG objects must produce byte-identical archives to the status quo (a binary check that proves isolation). The applicable families for size verdicts are the families containing TP-05 `jpeg` objects (at least F12, F13, F17, F18).
- **Arm B adversarial:** every F20 tuning item; work units per plaintext byte against the candidate's declared bound.
- **Minimum support:** arm E's applicable scope may fall below 10 groups across 5 families (§4.9). If so, E cannot reach a corpus-level verdict and is scoped to the applicable families, with `single-group` labels where relevant. This is a declared limitation.

## 6. Environment and platform

`wsl-ubuntu` x86-64 for the deterministic arms. The look-1 decode guard is timed on WSL `st-v` and Windows `st-p` inside EXP-PLANNER-007 windows. `alloc_peak_bytes` needs TP-08. No network, privileges or external review, except arm F's licence and specification criteria (codecs domain, possibly EXTERNAL).

## 7. Commands

```sh
python3 research/planner/tools/replay.py fit-tables --traces /root/eb-research/raw-side/EXP-PLANNER-002/traces --split tuning --out research/normalized/EXP-PLANNER-004/replay/tables.json
for arm in tables gating margins dictionary-rule; do
  python3 research/planner/tools/replay.py $arm --traces /root/eb-research/raw-side/EXP-PLANNER-002/traces --split tuning \
    --decode-cost-table research/normalized/EXP-PLANNER-008/decision/reference-table.json \
    --assignments-out /root/eb-research/raw-side/EXP-PLANNER-004/assignments --out research/normalized/EXP-PLANNER-004/replay/
done
python3 research/planner/tools/replay.py external-screen --traces /root/eb-research/raw-side/EXP-PLANNER-002/traces --split tuning --profiles dense,extreme \
  --out research/normalized/EXP-PLANNER-004/replay/external-screen.csv
PYTHONPATH=research/tools /root/eb-research/venv/bin/python -m ebr run --spec research/experiments/EXP-PLANNER-004/spec.yaml
PYTHONPATH=research/tools /root/eb-research/venv/bin/python -m ebr verify-raw --spec research/experiments/EXP-PLANNER-004/spec.yaml --refingerprint
PYTHONPATH=research/tools /root/eb-research/venv/bin/python -m ebr normalize --spec research/experiments/EXP-PLANNER-004/spec.yaml
```

The arm C corpus-fitted margins depend on EXP-PLANNER-008's tuning-only reference table. If that table does not exist when arm C is fitted, the fit uses the codec-class integer tiers and records the substitution as a deviation before any arm C result.

## 8. Seeds, warmup, replication

Seeds: order 260917041, bootstrap 260917042, command 260917043; selection bootstrap 260917994. Deterministic arms use 0 warmups, 2 repetitions and a repeat-hash check. The permutation-invariance check for arm C runs 3 seeded input-order permutations of cohort and region processing (seeds 260917044-046) on every tuning sub-item; it applies to `order-independent-region-costing` against the status quo. Look-1 timing follows §4.5-§4.6 (see EXP-PLANNER-007).

## 9. Metrics

| Metric | OD | Role | Family | Threshold | Source |
|---|---|---|---|---|---|
| `artifact_bytes` | OD-05 | primary superiority (all arms) | size | T-01 | TP-02/TP-06 materialized |
| `encode_wall_s` | OD-06 | guard (arms A, B); deterministic work units on tuning, timed at look 1 | time_wall | T-03 per profile | TP-07 |
| `decode_wall_s` | OD-07 | guard (A, C, E) | time_wall | T-04 | `ebr-unpack`, look 1 |
| `alloc_peak_bytes` | OD-08 | guard (A, D, E) | memory_peak | T-06 | TP-08 |
| `access_granularity_bytes` | OD-10 | guard (D, E) | size | T-09 | declared access cost |
| `dictionary_bytes`, `side_data_bytes`, `index_bytes` | OD-05 | diagnostic | size | T-02 | byte accounting |
| win share, marginal contribution, gate false-negative and false-positive rates, suppression frequency by reason, permutation churn count, orphan plan bytes | – | descriptive | other | none | TP-03 |
| work units per plaintext byte on F20 | OD-17 | descriptive; R2 boundedness input | other | none | TP-03 |
| `roundtrip_mismatches`, repeat-hash, isolation equality (arm E) | OD-02, HC-13 | binary | correctness | §3.3 | TP-02/TP-06 |

## 10. Normalization and statistical analysis

- **Per arm and per profile scope:** R1 (correctness, determinism, isolation equality), R2 (bounds from DEC-CMP-035 once fixed; until then reported, not screened), R3 tuning W and S, look-1 confirmatory R4 with confidences from `bootstrap.*` keys (§4.12), R6 with computed tiers, R7 sensitivity, R8/R9 per profile, R10.
- **Arm F:** for each external codec c and profile p, the modelled upper bound of the corpus-level `artifact_bytes` gain from adding c to p's table is the oracle-unbounded-plus-external selection restricted to that table ∪ {c}. If its interval does not lie beyond the `decision_rules.minimum_gain.new_registered_extended_codec` band (T2 multiplier from `cost_tiers.multipliers`), c is eliminated for p at R6 as a modelled upper bound (README §7). A passing upper bound selects nothing; the codecs domain decides.
- **Descriptive deliverables:**
  - A: win share by Chunk count and stored bytes per candidate, profile, family and tier; "never wins" list.
  - B: confusion matrix of the gate against always-try; the "transform would have won" oracle per group.
  - C: size against modelled decode-cost curves over the margin grid.
  - D: break-even reuse count distribution.
  - E: suppression counts by reason.
- **Aggregation:** §4.9 L1-L4 with `workload-mix.json` weights.
- **Practical significance:** thresholds.json keys only.

## 11. Sensitivity analysis

- B.1 frame: §6.4 threshold perturbation, §6.5 family and mix arms, tier-stratified and byte-weighted arms, §6.7 bootstrap. Bars per §6.6.
- Arm A: pruning stop rule at 0.1 and 0.5 band units (secondary).
- Arm B: fitted thresholds re-chosen with each family held out (overfit check).
- Arm C: permutation churn reported with every selection.
- Arm D: the charged-cost weights ×0.5 and ×2 (secondary).

## 12. Expected negative results worth recording

- Many frozen table candidates never win (for example LZ4 in balanced, or some LZMA2 pairs in dense). This is a negative for the status-quo tables but not necessarily a change, because R10 may keep them.
- `always-try` gains nothing measurable, so the probe is adequate. Alternatively the probe silently skips winning transforms on F07 and F08.
- Zero margins buy under one band of size while slowing decode.
- Dictionaries rejected by every charged rule on F04 small-file trees.
- The region/cohort conflict matters on few items.
- External codecs fail the T2 screen.

## 13. Threats to validity

- Replays assume the other stages are unchanged. Interactions between arms (a table change altering cohort baselines) are measured only at each arm's own W, not jointly. The joint W of arms A-E per profile is materialized once at look 1 as a secondary interaction check (CB-3 label, not confirmatory).
- The arm C decode-cost model depends on EXP-PLANNER-008.
- Arm E's applicable-family support is small.
- Arm F's plan-record bytes for external codecs are assumed.
- PSUB-256 truncation affects dictionary and cohort arms on large items.
- Designer L7 exposure.

## 14. Cost and effort

- **Compute:** replays about 20 CPU-hours; materialization of 90 (variant, profile) replay cells × PSUB-256 tuning × 2 repetitions, about 150 CPU-hours; arm E re-plans (16 cells) about 30 CPU-hours; look-1 validation materialization of W ∪ S about 50 CPU-hours.
- **Look-1 decode timing:** about 30 hours per environment, shared with EXP-PLANNER-007 windows.
- **Disk:** about 5 GB of assignments and raw data; about 20 GB transient.
- **Agent effort:** scripted tooling and execution; judgment for fits review and the per-arm R4-R10 write-up.

## 15. Gates and status

- **Tooling:** TP-01, TP-02, TP-03, TP-05, TP-06 (`stage_order`), TP-07, TP-08, `make_timing_spec.py`.
- **Gates:** G-A, G-B, cost scripts and rule simulation before look 1, runner additions and calibration for the timing guards, DEC-CMP-035 bounds before `DECIDED`.
- **Held-out:** EXP-PLANNER-012.
