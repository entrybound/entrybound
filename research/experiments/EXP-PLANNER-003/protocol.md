# EXP-PLANNER-003: Search strategies and deterministic work budgets

| Field | Value |
|---|---|
| Domain | planner (program §9, §10) |
| Index status | NEEDS_TOOLING (TP-01 sampled-cost option, TP-02, TP-03 `strategies`/`budgets`/`fit-tree`/`fit-bounds`, TP-07, TP-08, runner additions and calibration for the timing arm) |
| Document state | Phase C design; `record.md` after gate G-A |
| Shared conventions | `research/planner/README.md` |
| Runner specs | `spec.yaml` (deterministic materialization of every strategy cell, tuning); `spec-timing.yaml` (in-process encode timing template for planner look 1; the record replaces its candidate list with W ∪ S per profile using `research/planner/tools/make_timing_spec.py` before the look) |

## 1. Pre-registration header

- **decision_ids:** DEC-CMP-032 (search strategy), DEC-CMP-033 (deterministic work budget and sampled evaluation).
- **Decision type (proposed):** EMPIRICAL. The integer-only and no-environment-input property of every strategy is FORMAL: a written check of the strategy definition plus a counterexample search, namely the repeat-hash of every materialized cell across two processes and, in EXP-PLANNER-009, across hosts.
- **Proposed `od_ids`:** OD-05 (superiority), OD-06, OD-07, OD-08, OD-10 (guards). **Proposed `hc_ids`:** HC-02, HC-06, HC-11, HC-13, HC-16.
- **Frame:** profile scope, constrained form (§2.0, Appendix B.1; README §6). Scopes: fast, balanced, dense, extreme.
- **Author / L7:** README §10.

## 2. Question and hypotheses

- **Question.** Within each profile's declared bounds, which deterministic search strategy (and which work budget) attains the smallest `artifact_bytes` without breaking the encode-time, decode-time, decode-memory and access-granularity guards? Is any strategy more complex than the status-quo table search justified by lower regret against the exhaustive oracle?
- **H1.**
  1. In dense and extreme, successive halving (k = 2) is `NON_INFERIOR` to the status quo on `artifact_bytes` at at most half of its candidate evaluations.
  2. No strategy is `DISTINGUISHABLE_BETTER` than the status quo on `artifact_bytes` in fast or balanced.
  3. The learned integer selector keeps corpus-level regret against the bounded oracle within 1 band unit at at most 25% of status-quo evaluations.
  4. Per profile there is a finite evaluation-count budget that is `NON_INFERIOR` to the unbudgeted table on `artifact_bytes` on tuning.
- **H0.** Every alternative is `EQUIVALENT` to the status quo on `artifact_bytes`. The status quo then wins by R10 (key 4) in every scope.
- **Falsification.** H1.1 fails if successive halving is not `NON_INFERIOR` at validation, or needs more than 50% of evaluations. H1.2 fails on any validation `DISTINGUISHABLE_BETTER` verdict in fast or balanced. H1.3 fails if the corpus-level regret interval upper bound exceeds 1 band unit, or evaluations exceed 25%. H1.4 fails if no grid budget is `NON_INFERIOR` on tuning.

## 3. Decisions informed

| Decision | Supplied here | Elsewhere |
|---|---|---|
| DEC-CMP-032 | Regret distributions of every strategy; R3-R10 per profile scope on `artifact_bytes` with guards; work units; in-process encode time | Status-quo regret and oracle tables: EXP-PLANNER-002; cross-platform determinism: 009; held-out: 012 |
| DEC-CMP-033 | Budget response curves (`artifact_bytes` against evaluation and bytes-examined budgets per profile); the tightest non-inferior budget per profile (TNB rule, README-consistent), consumed as `compress_budget` values by EXP-PLANNER-006; budget-exhaustion behaviour on F20 | `compress_budget` fixing: EXP-PLANNER-006 (DEC-CMP-035); pack throughput on multi-GB: EXP-PLANNER-007 |

## 4. Candidates (per profile scope)

Strategies operate on the profile's own candidate universe: the v6 table, or the bounded oracle universe where stated. Cohort and region stages stay status-quo unless stated, so strategies differ only in the Chunk stage (and the plan-sharing and grouping DP).

| candidate_id | Definition | Expected tier |
|---|---|---|
| sq-table (reference) | Status quo: full encode of every table candidate per Chunk, probe-gated transforms, v4-v6 margin rule | status quo |
| fixed-ladder | No search: fast Zstd 1, balanced Zstd 3, dense Zstd 9, extreme Zstd 19; STORE when not smaller by the status-quo margin | T1 |
| oracle-bounded | EXP-PLANNER-002 bounded oracle, as a strategy | T1 |
| sh-k1, sh-k2, sh-k3 | Successive halving. Stage 1 costs every table candidate on a deterministic sample of the Chunk: up to 16 contiguous 4,096-byte windows at offsets `floor(i·(len−4096)/15)`, i = 0..15, or the whole Chunk if at most 65,536 bytes. Candidates are ranked by sampled complete cost scaled to Chunk length with integer arithmetic. Stage 2 fully encodes the top k and applies the status-quo rule | T1 |
| dp-plan-sharing | Dynamic program charging each distinct TransformPlan record once per archive when choosing Chunk plans. In dense and extreme, also `select_group_lookback` partitions within the profile's lookback bound (balanced keeps lookback 0 by contract) | T1 |
| bb-prune | Cheap-first order (`candidate_shapes` order). Candidate c is skipped when an integer upper bound on its improvement over the incumbent cannot clear the margin. Bounds are integer basis points per (predecessor shape, successor shape, ratio bucket of the predecessor in 16 buckets), fitted on tuning as the maximum observed improvement plus one bucket (no statistics, no floating point) | T1 |
| tree-d6 | Integer decision tree, depth at most 6, over `distinct_symbols`, `maximum_symbol_frequency_ppm`, `adjacent_repeat_ppm`, `likely_compressible`, `logical_len` bucket, TP-05 category id, and the Zstd-1 size ratio in basis points. Each leaf holds a shortlist of at most 2 shapes that are fully encoded. Fitted on tuning with grouped cross-validation by independence group (seed 260917993, 5 folds); the split criterion is integer total cost | T1 |
| lut-cat-probe | Lookup table keyed by (TP-05 category, probe bucket), holding the shortlist of at most 2 shapes with the lowest summed tuning cost | T1 |
| greedy-p1, -p2, -p3, -p5 | `select_greedy` with patience 1, 2, 3, 5 over the profile table | T1 |
| evalbudget-c{1,2,4,8,16} | Status-quo order truncated after c full-Chunk evaluations per Chunk, plus a per-archive cap of 4·c evaluations per MiB. When exhausted, the incumbent stands | T1 |
| bytesbudget-m{1,2,4,8,16} | Plaintext bytes examined by candidate encoders capped at m × Chunk length per Chunk (per-archive cap m × logical bytes) | T1 |
| sampled-bal-full-dense | sh-k1 for fast and balanced; sq-table for dense and extreme | T1 |
| critic-proposed-n | Reserved slot for candidates the Phase C adversarial design review adds (R0 item 6) | – |

- **Reference candidate:** sq-table per profile.
- **Excluded before execution:**
  - `float-heuristic-selector` (DEC-CMP-032): R1 by HC-13 and SPEC §5.7 (floating-point evaluation in deterministic mode; `docs/planner-v1.md` determinism clause). Recorded in the ledger's `candidate_exclusion_reasons`. The R1(b) counterexample is the SPEC clause; sign-off by a session not involved is pending.
  - `wall-clock-budget` (DEC-CMP-033): R1 by HC-13 and SPEC §5.7 L605 (machine-dependent stopping point). Same sign-off requirement.
- **Defer (R0 item 5):** equivalent to sq-table (no strategy change for v1). Add-later cost C7: a new planner ID at any later time.
- **Tiers:** every strategy is expected to be T1, as a new planner ID with a permanent-vector obligation (§7.2). The computed tier comes from the §14 item 7 scripts plus the C4-C7 assessor before look 1.

## 5. Corpus

- **Deterministic arm:** PSUB-256 sub-items of every tuning item (`research/planner/derived/psub-256.json`); validation sub-items in planner look 1. All 20 families.
- **Timing arm (look 1 only):** TS-1-V. Fast and balanced use whole items; dense and extreme use PSUB-64 sub-items (README §4).
- **Fitting data** (bb-prune bounds, tree-d6, lut-cat-probe): tuning traces only. The public tuning sources used for fitting are listed in the record for the §5.4 item 2 upstream-lineage check.

## 6. Environment and platform

- Deterministic arm: `wsl-ubuntu` x86-64.
- Timing arm: `wsl-ubuntu` with `st-v` affinity {2} (§4.3), and `windows-host` with `st-p` {2} (§4.2) for same-direction support (§4.1). The strategies are single-threaded, like production.
- Requires runner additions (§14 item 3), calibration `EXP-CAL-*-wsl-ubuntu` and `EXP-CAL-*-windows-host` for `time_wall` and `time_cpu`, and the TP-08 counting allocator for `alloc_peak_bytes`.

## 7. Commands

```sh
python3 research/planner/tools/replay.py fit-bounds --traces /root/eb-research/raw-side/EXP-PLANNER-002/traces --split tuning --out research/normalized/EXP-PLANNER-003/replay/bb-bounds.json
python3 research/planner/tools/replay.py fit-tree --traces /root/eb-research/raw-side/EXP-PLANNER-002/traces --split tuning --depth 6 --folds 5 --seed 260917993 --out research/normalized/EXP-PLANNER-003/replay/tree-d6.json
python3 research/planner/tools/replay.py strategies --traces /root/eb-research/raw-side/EXP-PLANNER-002/traces --split tuning \
  --bounds research/normalized/EXP-PLANNER-003/replay/bb-bounds.json --tree research/normalized/EXP-PLANNER-003/replay/tree-d6.json \
  --assignments-out /root/eb-research/raw-side/EXP-PLANNER-003/assignments --out research/normalized/EXP-PLANNER-003/replay/
python3 research/planner/tools/replay.py budgets --traces /root/eb-research/raw-side/EXP-PLANNER-002/traces --split tuning \
  --assignments-out /root/eb-research/raw-side/EXP-PLANNER-003/assignments --out research/normalized/EXP-PLANNER-003/replay/
PYTHONPATH=research/tools /root/eb-research/venv/bin/python -m ebr run --spec research/experiments/EXP-PLANNER-003/spec.yaml
PYTHONPATH=research/tools /root/eb-research/venv/bin/python -m ebr verify-raw --spec research/experiments/EXP-PLANNER-003/spec.yaml --refingerprint
PYTHONPATH=research/tools /root/eb-research/venv/bin/python -m ebr normalize --spec research/experiments/EXP-PLANNER-003/spec.yaml
# look 1 (after W and S are frozen)
python3 research/planner/tools/make_timing_spec.py --template research/experiments/EXP-PLANNER-003/spec-timing.yaml \
  --record research/experiments/EXP-PLANNER-003/record.md --out research/experiments/EXP-PLANNER-003/spec-timing-look1.yaml
PYTHONPATH=research/tools /root/eb-research/venv/bin/python -m ebr run --timing --spec research/experiments/EXP-PLANNER-003/spec-timing-look1.yaml
# decision analysis (§14 item 2 tooling) -> research/normalized/EXP-PLANNER-003/decision/
```

## 8. Seeds, warmup, replication

- Seeds: order 260917031, bootstrap 260917032, command 260917033; tree folds 260917993; selection bootstrap 260917994.
- **Deterministic arm:** warmups 0, 2 repetitions; `artifact_sha256` repeat-hash (§4.13). The strategies claim determinism, so a mismatch is an R1 elimination with the artifacts committed.
- **Timing arm:** warmups 2 for small and medium, 1 for large (§4.5). Rounds follow §4.6 by the **realized** input tier of each measured (sub-)item: minimum `protocol.timing_rounds.<tier>.min`, with the adaptive step and cap from the same keys. The minimum is raised to n_min for the comparison's confidence (§4.6). The adaptive rule is precision-based only (§4.6 items 1-4). Order: randomized blocks, paired within rounds.

## 9. Metrics

| Metric | OD | Role | Family | Unit | Dir | Threshold | Source |
|---|---|---|---|---|---|---|---|
| `artifact_bytes` | OD-05 | primary, the only superiority metric (B.1) | size | B | min | T-01 | TP-02 `artifact_bytes` (materialized, every cell) |
| `encode_wall_s` (in-process: selection + encode) | OD-06 | primary guard at the profile band (`metric_thresholds.encode_wall_s.relative_by_profile`) | time_wall | s | min | T-03 | TP-07 |
| `decode_wall_s` | OD-07 | primary guard | time_wall | s | min | T-04 | `ebr-unpack` on materialized W and S archives, TS-1-V |
| `alloc_peak_bytes` (decode) | OD-08 | primary guard | memory_peak | B | min | T-06 | TP-08 |
| `access_granularity_bytes` | OD-10 | primary guard (changes only for dp-plan-sharing grouping) | size | B | min | T-09 | declared access cost of the materialized archive |
| `encode_cpu_s` | OD-06 | secondary | time_cpu | s | min | T-05 | TP-07 |
| regret against oracle-bounded (log ratio in band units) | OD-05 | secondary (reporting) | size | – | min | T-01 | TP-03 |
| work units (`candidate_evaluations`, `bytes_examined`) | – | descriptive; R2 budget screen input | other | count; B | – | none | TP-03 and TP-07 |
| `roundtrip_mismatches`, repeat-hash | OD-02, HC-13 | binary | correctness | – | = 0 | §3.3 | TP-02 |
| `bounded_memory_growth_bytes` of planning (F20) | HC-06 | binary claim check when a budget claims boundedness | correctness | B | at most bound | objective M08.3 | scale-domain probe, informational here |

At most one primary metric per OD (R3).

## 10. Normalization and statistical analysis

1. **R1/R2.** Round-trip, repeat-hash and determinism failures eliminate at R1. The R2 compress-budget screen uses per-profile `compress_budget` values from DEC-CMP-035 (EXP-PLANNER-006) in deterministic work units: any tuning or validation sub-item whose work exceeds the budget eliminates the (candidate, profile) pair. **Until DEC-CMP-035 fixes values, budgets are reported, not screened, and neither decision can be `DECIDED` (R2 last bullet).** An F20 item whose work is not bounded by the candidate's declared budget eliminates that candidate at R2.
2. **R3 tuning.** Per profile, W = the smallest corpus-level `artifact_bytes` candidate meeting the bounds and guards. Guards on tuning use deterministic proxies (work units for encode; declared decode requirements) and are exploratory. S = candidates not eliminated by R4 on tuning. The MDE gate (§4.12) comes from tuning group variance with the validation structure.
3. **Look 1 (validation, confirmatory).** For each profile scope, W against each s in S on `artifact_bytes`, with superiority metrics declared per pair from tuning (k_sup = 1). Superiority confidence follows `bootstrap.superiority_confidence_rule`; non-inferiority follows `bootstrap.noninferiority_confidence`; the family harm guard follows `bootstrap.family_harm_guard_confidence` and `statistics.family_guard_max_point_bands` (R4 condition 3), per family, tier and profile. Guard metrics use the non-inferiority confidence at corpus level. Intervals: exact order statistic at item level for timing, Welch-Satterthwaite at corpus level, pooled variance for families (§4.10).
4. **R6.** Computed tiers. Every strategy is expected at T1 (k = 2 against the status quo's T0), so an alternative must be `DISTINGUISHABLE_BETTER` against the `cost_tiers.multipliers.T1` minimum-gain band of T-01 to displace the status quo.
5. **R8/R9.** A universal claim holds only if the same candidate is selected in all four profile scopes. Otherwise the per-profile winners stand at R9 partition 1, whose expressibility is given because planner IDs are per profile. The partition cost (R9 ii) is T1.
6. **R10.** Tie keys in order: tier, normative words (the checked-in counter over each strategy's planner document), writer LoC, status quo, id.
7. **Regret deliverable.** For every candidate: median, p90 and maximum regret in band units per profile × family, tier, WM-* mix and TP-05 category, with n per cell.
8. **DEC-CMP-033 budget curves.** Per profile and budget kind, corpus-level `artifact_bytes` ratio against sq-table as a function of the budget. The TNB value is the tightest grid budget whose tuning point estimate is within 1 band unit, confirmed as `NON_INFERIOR` at look 1. It is exported to EXP-PLANNER-006 as the `compress_budget` candidate value.

Practical significance: T-01, T-03 (per profile), T-04, T-06 and T-09 as transcribed in `research/methods/thresholds.json`.

## 11. Sensitivity analysis

B.1 frame (no weights):

- §6.4 threshold perturbation (global ×0.5 and ×2, single-metric), with the resolvability condition for timing bands from A/A half-widths.
- §6.5 arms: equal weights, leave-one-family-out, leave-three-families-out, family Dirichlet, WM-* mixes, tier-stratified, byte-weighted, environment arm (WSL against Windows).
- §6.7 selection-stability bootstrap (seed 260917994). Bars per §6.6.

Specific arms:

- **Learned-selector overfit arm:** tree-d6 and lut-cat-probe refitted with each family held out in turn. The decision uses the in-family fit; the held-out-family regret is reported as a generalization limitation.
- **Sampling-window arm:** sh-k2 with 8 and 32 windows (secondary).

## 12. Expected negative results worth recording

- No strategy beats sq-table beyond the T1 minimum gain in any profile: complex search is not justified (DEC-CMP-032 outcome "status quo by R10").
- tree-d6 regret above 1 band unit on families absent from its fit.
- Budgets that save most work are not `NON_INFERIOR` on F07, F08 or F09, whose winners appear late in cheap-first order.
- fixed-ladder is `NON_INFERIOR` in fast, meaning fast's LZ4 and STORE search does not pay.

## 13. Threats to validity

- Strategies are simulated through replay over traces and then materialized. Encode-time comparisons come from TP-07 in-process timing of a research build (the `research-internals` codegen caveat, harness review R1-06). Timings are paired within one build, and the status quo is timed through the same TP-07 path.
- WSL timing is `VIRTUALIZED` (soft cap MEDIUM for Linux-native claims); Windows timing is subject to Defender and VBS (§4.2).
- The learned selectors are fitted on the same tuning split where W is chosen. Validation protects confirmation, but W selection is optimistic.
- Oracle and strategy universes exclude reconstructive DEFLATE and JPEG steps in the Chunk stage (harness `in_scope`).
- Cross-domain relatedness: codec table changes (EXP-PLANNER-004) alter strategy universes. Their W and S must freeze together (README §5).
- Designer L7 exposure.

## 14. Cost and effort

- **Compute:** replay and fitting about 10 CPU-hours. Materialization of every cell: 25 strategy and budget candidates × 4 profiles on PSUB-256 tuning, 2 repetitions, about 150 CPU-hours (encoding a fixed assignment, no search). Validation materialization of W ∪ S about 40 CPU-hours.
- **Timing (look 1):** for W ∪ S (estimated at most 4 per profile) on TS-1-V, about 60 hours of exclusive wall time per environment, about 120 hours for both.
- **Disk:** assignments about 3 GB; transient scratch about 15 GB; raw about 1 GB.
- **Agent effort:** tooling and execution scripted; analysis, fits review and the R4-R10 write-up are judgment.

## 15. Gates and status

- **Tooling:** TP-01 (with `--sampled-costs 16x4096`), TP-02, TP-03, TP-05, TP-07, TP-08, `make_timing_spec.py`.
- **Gates:** G-A; G-B; runner additions and calibration for the timing arm; §14 item 7 cost scripts and item 9 rule simulation before look 1; DEC-CMP-035 values (EXP-PLANNER-006) before any `DECIDED` status.
- **Held-out:** EXP-PLANNER-012.
