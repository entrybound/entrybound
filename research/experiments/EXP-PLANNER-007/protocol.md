# EXP-PLANNER-007: Profile set, default profile and profile timing campaign

| Field | Value |
|---|---|
| Domain | planner (program §9, §10) |
| Index status | NEEDS_TOOLING (runner additions of decision-method §14 item 3; calibration experiments of §4.7 for `wsl-ubuntu` and `windows-host`; TP-06 fitted profiles; TP-07; TP-08; TP-16 TS-1; TP-19 Windows item staging; `make_timing_spec.py`) |
| Document state | Phase C design; `record.md` after gate G-A |
| Shared conventions | `research/planner/README.md` |
| Runner specs | `spec-prepack.yaml` (untimed deterministic pre-pack of decode inputs); `spec-alloc.yaml` (untimed counting-allocator decode peaks of the pre-packed archives); `spec.yaml` (WSL timing, TS-1 whole items: fast and balanced encode, all-profile decode); `spec-psub64.yaml` (WSL timing, TS-1 PSUB-64: dense and extreme encode); `spec-windows.yaml` (Windows host template for look 1). Look-1 validation specs are generated from these by `make_timing_spec.py` with `splits: [validation]` on TS-1-V |

## 1. Pre-registration header

- **decision_ids:** DEC-CMP-036 (primary: the four-profile set and the balanced default). Timing guards for DEC-CMP-035 (fitted profile definitions from EXP-PLANNER-006) are measured in the same windows. DEC-CMP-032, -033 and -034 timing guards run in their own look-1 specs inside these windows (EXP-PLANNER-003, -004).
- **Decision type (proposed):** EMPIRICAL for the profile-set and default measurements; HUMAN-FACING for any claim that users pick or understand profiles. Such claims are routed to the ecosystem domain's simulated first-use evaluation (heuristic only, §4.16) and to EXTERNAL_REVIEW_REQUIRED (§8.3). The guidance census is a mechanical EMPIRICAL check.
- **Proposed `od_ids`:** OD-05, OD-06, OD-07, OD-08, OD-10 (the utility OD set of part (c)); OD-25 (C6 flag and concept counts per menu, cost input). **Proposed `hc_ids`:** HC-05 (unsafe defaults: no default may disable verification or safety), HC-06, HC-13, HC-16.
- **Frames:** parts (a) and (d) apply §7.3 and §7.4 to the profile set. Part (c), the default, is **universal scope** with band-unit utility (§6.2) and equal OD base weights (§6.3). Guards of DEC-CMP-035 use Appendix B.1.
- **Author / L7:** README §10.

## 2. Question and hypotheses

- **Question.** Measured on size, encode time, decode time, decode memory and access granularity, does each of the four profiles earn its place as if it were new (§7.3 "New profile", §7.4 "no sunk-cost credit"), does any smaller or differently shaped menu cover the measured trade-off space within the pre-registered regret bars, and is `balanced` the defensible default?
- **H1.**
  1. fast, balanced and dense each pass the §7.3 new-profile rule. extreme fails it against dense (gain below the T2 band in fewer than `decision_rules.minimum_gain.new_profile.min_families` families).
  2. The menu {fast, balanced, max = extreme's assignment} meets the coverage bars of part (d).
  3. Under equal OD weights, balanced is the measured band-unit utility winner, with weight-grid and Dirichlet FPW ≥ `sensitivity.min_fraction_preserving_winner`.
  4. The guidance census fails for the status quo: `ebound pack` help states neither use case nor bounds per profile.
- **H0.** All four profiles pass. No smaller menu meets the bars. No default is a measured winner (R10 or R9(iii) decides).
- **Falsification.** H1.1 fails if extreme passes, or if any of fast, balanced or dense fails, at look 1. H1.2 fails if that menu breaks a bar. H1.3 fails if another default wins U, or if FPW is below the bar. H1.4 fails if the help text states use case and bounds for every profile.

## 3. Decisions informed

| Decision | Supplied here | Elsewhere |
|---|---|---|
| DEC-CMP-036 | Pareto placement of each profile per family and tier on size, encode, decode, memory and access granularity (with EXP-PLANNER-001 sizes); adjacent-profile distinctness; §7.3 profile justification; default selection with §6 sensitivity; menu coverage for 3-profile, 2-profile, single-plus-knobs and level-scale alternatives; guidance census | Simulated first-use profile choice (heuristic, labelled SIMULATED): ecosystem §33; human claims: EXTERNAL review; held-out: EXP-PLANNER-012 |
| DEC-CMP-035 | Timing and memory guards of the fitted profile definitions against the frozen tables; harness-against-`ebound` timing parity | Size side: EXP-PLANNER-006 |

## 4. Candidates

### 4.1 Timed configurations Φ

| config id | Encoder | Decoder |
|---|---|---|
| sq-fast, sq-balanced, sq-dense, sq-extreme | production `ebound pack --profile P` (no `research-internals`; harness use constraint 2) | production `ebound unpack` |
| fit-fast, fit-balanced, fit-dense, fit-extreme | EXP-PLANNER-006 DEC-CMP-035 provisional W per profile via `ebr-planner param-plan --fitted-profiles` (omitted where W = status quo) | production `ebound unpack` (plans are data; decoding is profile-independent, SPEC §6.0) |
| par-fast, par-balanced, par-dense, par-extreme (parity arm) | status-quo profiles via `ebr-planner param-plan --base-profile P` with no parameter sets and `--no-verify true`: the same harness encoder path as fit-P, producing archives byte-identical to sq-P (checked in `spec-prepack.yaml`) | – |

**Parity rule.** If par-P against sq-P is not `EQUIVALENT` on `encode_wall_s` at corpus level (§4.11, `bootstrap.noninferiority_confidence`), fit-P encode timings are informational only (harness review R1-06 codegen caveat).

### 4.2 DEC-CMP-036 candidates

| candidate_id | Menu | Default |
|---|---|---|
| status-quo-four-balanced-default (reference) | fast, balanced, dense, extreme | balanced |
| three-profiles-merge-dense-extreme-A | fast, balanced, max := dense | balanced |
| three-profiles-merge-dense-extreme-B | fast, balanced, max := extreme | balanced |
| two-profiles-A / -B | fast, small := dense / extreme | best of the two under part (c) |
| single-profile-plus-knobs | balanced plus the thirteen EXP-PLANNER-006 parameters exposed as flags | balanced |
| level-scale | 9 levels. The EXP-PLANNER-006 tuning Pareto set of measured configurations (size against work units) is sorted by work units, and level k is the point at the k/9 quantile of log work units | level 5 |
| default-dense | four profiles | dense |
| default-fast | four profiles | fast |
| default-extreme | four profiles | extreme (added by this designer for completeness of the default question; R0 item 6 still needs a critic's candidate) |
| critic-proposed-n | reserved | – |

- **Expected tiers:** every profile counts as a new profile at T2 (§7.2), with no sunk-cost credit (§7.4). Menus of equal tier differ in C1 words and C6 flags (R10 key 2, cost input `flags_per_task`). single-profile-plus-knobs and level-scale add user-visible flags or modes: T2 plus C6.
- **Excluded:** none.

## 5. Corpus

- **Tuning (exploratory, WSL only):** TS-1 (README §4). fast and balanced encode on whole items plus the TS-1 large items. dense, extreme and fit-dense/extreme encode on the PSUB-64 sub-items of TS-1. Decode of every configuration on the same inputs its encode used.
- **Look 1 (confirmatory):** TS-1-V on WSL and Windows.
- **Families:** every family present in TS-1; minimum support per §4.9 is checked on TS-1-V before the look (the MDE gate).
- **Deterministic inputs** (`artifact_bytes`, `access_granularity_bytes`): from EXP-PLANNER-001 and 006 on the full tuning and validation splits, joined per item.
- **Weights:** `workload-mix.json`; equal weights and WM-* as §6.5 arms.

## 6. Environment and platform

| Environment | Affinity | Guard (spec tightening, §4.4) | Notes |
|---|---|---|---|
| `wsl-ubuntu` | `st-v` {2} | `max_loadavg_1m` 2.0; `max_other_cpu_percent` 3.0; `max_wait_s` 300 | `VIRTUALIZED`; scratch on ext4 under `/root/eb-research/scratch`; cache state `hot` (§4.5) |
| `windows-host` | `st-p` {2} (sibling 3 idle) | as §4.4 with per-core-class accounting (§14 item 3) | NTFS scratch outside the repo (`D:/eb-research/scratch-win`); Defender and VBS recorded (§4.2); items staged by TP-19 |

Exclusive timing windows only (§4.2): no workflow agents, builds or provisioning. Calibration `EXP-CAL-AA-`, `-AAP-`, `-KE-` and `-BG-` for both environments and families `time_wall`, `time_cpu` and `memory_peak` must pass first (§4.7). Platforms: Linux x86-64 (virtualized) and Windows x86-64. macOS and native ARM64 are PLATFORM_BLOCKED, so the decision scope explicitly excludes them (§4.1), and EXP-PLANNER-011 carries the replication.

## 7. Commands

```sh
python3 research/planner/tools/make_ts1.py --manifest research/corpus/manifest.json --out research/planner/derived/ts1.json
python3 research/planner/tools/make_psub.py --k-mib 64 --manifest research/planner/derived/ts1.json --out-manifest research/planner/derived/ts1-psub-64.json \
  --out-root /root/eb-research/derived/planner/ts1-psub-64
PYTHONPATH=research/tools /root/eb-research/venv/bin/python -m ebr run --spec research/experiments/EXP-PLANNER-007/spec-prepack.yaml
PYTHONPATH=research/tools /root/eb-research/venv/bin/python -m ebr run --timing --spec research/experiments/EXP-PLANNER-007/spec.yaml
PYTHONPATH=research/tools /root/eb-research/venv/bin/python -m ebr run --timing --spec research/experiments/EXP-PLANNER-007/spec-psub64.yaml
# look 1
python3 research/planner/tools/make_timing_spec.py --template research/experiments/EXP-PLANNER-007/spec.yaml --split validation --subset research/planner/derived/ts1-validation.json --out research/experiments/EXP-PLANNER-007/spec-look1-wsl.yaml
python3 research/planner/tools/make_timing_spec.py --template research/experiments/EXP-PLANNER-007/spec-windows.yaml --split validation --subset research/planner/derived/ts1-validation-windows.json --out research/experiments/EXP-PLANNER-007/spec-look1-windows.yaml
# guidance census (mechanical)
/root/eb-research/target/planner-cli/release/ebound pack --help > research/raw/EXP-PLANNER-007/guidance/pack-help.txt
python3 research/planner/tools/guidance_census.py research/raw/EXP-PLANNER-007/guidance/pack-help.txt --out research/normalized/EXP-PLANNER-007/guidance.csv
```

Every run is followed by `ebr verify-raw` and `ebr normalize`; decision analysis uses the §14 item 2 tooling.

## 8. Seeds, warmup, replication

- Seeds: order 260917071, bootstrap 260917072, command 260917073; selection bootstrap 260917994; weight Dirichlet and SMAA seeded from `bootstrap`.
- Warmups: 2 for small and medium, 1 for large (§4.5), recorded. The warmup-adequacy check (Kendall τ) comes from calibration.
- Rounds: the minimum for the realized input tier from `protocol.timing_rounds`, raised to n_min for the comparison confidence. Adaptive step and cap as in §4.6: precision-based, never direction-based.
- Order: randomized blocks paired within rounds. Cooldown, drift canary, frequency covariate and throttling-onset probe per §4.2 (Windows) and §4.3 (WSL).

## 9. Metrics

| Metric | OD | Role | Family | Threshold | Source |
|---|---|---|---|---|---|
| `artifact_bytes` | OD-05 | primary (utility and §7.3 superiority) | size | T-01 | EXP-PLANNER-001 / 006 |
| `encode_wall_s` | OD-06 | primary (utility); guard for DEC-CMP-035 at the profile band | time_wall | T-03 (`relative_by_profile`; the small-tier in-process rule applies via TP-07 for fit-* and par-*, process samples informational on small items) | runner `resource.wall_s` of the pack sample |
| `decode_wall_s` | OD-07 | primary | time_wall | T-04 | runner `resource.wall_s` of the unpack sample |
| `alloc_peak_bytes` (decode) | OD-08 | primary | memory_peak | T-06 | TP-08 via `ebr-unpack --alloc-peak true` (`spec-alloc.yaml`, untimed, 2 repetitions: the counting allocator is exact) |
| `access_granularity_bytes` | OD-10 | primary | size | T-09 | EXP-PLANNER-001 / 006 declared access cost |
| `encode_cpu_s`, `decode_cpu_s` | OD-06, OD-07 | secondary | time_cpu | T-05 | runner `resource.cpu_s` |
| `peak_rss_bytes` (Linux, GNU time only), `peak_commit_bytes` (Windows) | OD-08 | secondary | memory_peak | T-06 | runner (R1-16 constraint) |
| `flags_per_task` | OD-25 | cost input (C6) | other | – | guidance census and menu definition |
| guidance census (use case and bounds per profile) | – | binary condition of the §7.3 new-profile rule | correctness | – | `guidance_census.py` |

## 10. Normalization and statistical analysis

**(a) Profile justification (§7.3 "New profile", applied as if new under §7.4).** For each profile p, the bounds are the DEC-CMP-035 provisional W values from EXP-PLANNER-006. Before those exist, the status-quo implied bounds (EXP-PLANNER-002 §4) are used and labelled provisional. The other profiles q that satisfy p's bounds are identified at R2. p passes iff all of the following hold:
1. p's `artifact_bytes` is `DISTINGUISHABLE_BETTER` than every such q against the T2 minimum-gain band (`cost_tiers.multipliers.T2` on T-01) in at least `decision_rules.minimum_gain.new_profile.min_families` families;
2. the guards hold (R4 condition 3);
3. no T1 parameter change to an existing profile from the EXP-PLANNER-006 grid achieves at least `decision_rules.minimum_gain.new_profile.t1_alternative_max_log_gain_share` of p's log gain;
4. the guidance census passes.

If no other profile satisfies p's bounds, the R6 exemption applies (every lower-tier alternative eliminated at R2) and is recorded.

**(b) Distinctness.** Adjacent-profile gaps in continuous band units on each primary metric, per family and tier (descriptive).

**(c) Default (universal scope).**
- Candidates: the four profiles as default.
- OD set {OD-05, OD-06, OD-07, OD-08, OD-10} with equal base weights (§6.3). U(c) per §6.2 at corpus level with `workload-mix.json` weights.
- R4 among defaults at look 1: W = the tuning U-maximizer; S = the rest. Confirmatory pairs W against each s, with superiority metrics declared from tuning (k_sup per pair) and confidences per `bootstrap.superiority_confidence_rule` and `noninferiority_confidence`.
- R8: measured winner plus the R4 guard plus §6.6 bars (weight grid over 5 ODs, Dirichlet, selection bootstrap, family arms, WM-* mixes) plus R5 later.
- If R8 fails, R9 partitions 1-4 are **not expressible** for a default (R9 (i): pack cannot know the profile, policy, mix or family intent), so R9 (iii) applies. The compromise minimizes the maximum weighted regret across WM-* parts, accepted only within `sensitivity.max_compromise_regret_band_units` and `sensitivity.max_compromise_regret_per_metric_band_units`, soft-capped MEDIUM. Otherwise `INSUFFICIENT_EVIDENCE` with the frontier published.
- SMAA whole-simplex acceptability and single-OD-vertex winners are reported (§6.3).

**(d) Menu coverage (method extension; needs round-2 method review acceptance, otherwise descriptive).** For menu M and part π (each WM-* mix, and each single-OD vertex), the menu regret is `max_{c∈Φ} U_π(c) − max_{m∈M} U_π(m)` in band units, computed over the timed configurations Φ. M meets coverage iff its maximum menu regret over parts is within `sensitivity.max_compromise_regret_band_units` and, per primary metric, within `sensitivity.max_compromise_regret_per_metric_band_units`. These are the R9 (iii) values, applied by analogy and flagged as such.

**Decision logic for the profile set** (pre-registered order):
1. Eliminate menus containing a profile that fails (a) (R6 minimum gain).
2. Eliminate menus failing (d).
3. R10 among the rest: tier, then C1 words plus C6 flags, then LoC, then status quo.
4. The default of the surviving menu comes from (c) restricted to its members.

**Aggregation and intervals:** §4.9 and §4.10. Item level uses the exact order-statistic interval of paired log ratios for timing, and exact values for deterministic metrics.

**Invalid-sample balance:** §4.4 rules; a comparison with more than 20% of items excluded is not decision-grade.

## 11. Sensitivity analysis

- **Default (c):** full §6.3 weight grid (5 ODs, grid factors from `sensitivity.grid_factors`), Dirichlet with `sensitivity.dirichlet_samples` and `sensitivity.dirichlet_concentration`, SMAA flat Dirichlet report, §6.4 threshold perturbation with the resolvability condition from calibration half-widths, all §6.5 arms including the environment arm (WSL against Windows), and the §6.7 bootstrap. Bars per §6.6.
- **Profile justification (a):** threshold perturbation of T-01 ×0.5 and ×2 (reported as `band_dependent` if outcomes flip); bound-source arm (status-quo implied bounds against fitted bounds).
- **Menus (d):** Φ extended with EXP-PLANNER-006 deterministic-only configurations, whose time is proxied by work units (secondary, labelled).

## 12. Expected negative results worth recording

- extreme not justified against dense (a merge candidate).
- balanced not the equal-weight utility winner (fast wins when time dominates).
- The guidance census fails for every profile.
- level-scale levels fail the per-level minimum gain.
- WSL and Windows disagree in direction for encode-time comparisons of dense against extreme (Defender scan-on-close effects).
- Parity failure between harness and `ebound` timings.

## 13. Threats to validity

- There is a single host CPU model. P-core and E-core classes are not both measured here (`st-p` only on Windows).
- WSL is virtualized; Windows timing is affected by Defender and VBS.
- dense and extreme encode timing on PSUB-64 sub-items does not cover large-tier encode, which is reported as a limitation of the tier guard.
- Φ is small (at most 8 timed configurations), so menu regret against an unmeasured configuration is unknown.
- Equal OD base weights are `INFERRED` (§6.6 rationale).
- The human-facing part is not measurable here.
- Designer L7 exposure.

## 14. Cost and effort

- **Timing, exclusive wall (estimates):** WSL tuning about 80 hours; look-1 WSL about 80 hours; look-1 Windows about 90 hours. Total about 250 hours, overnight-scheduled windows.
- **Deterministic pre-pack:** about 20 CPU-hours.
- **Disk:** pre-packed archives about 15 GB; staged Windows items about 10 GB on `D:/eb-research`; raw about 1 GB.
- **Agent effort:** scripted execution; judgment for parts (a)-(d) and the sensitivity write-up.

## 15. Gates and status

- **Tooling and prerequisites:** runner additions (§14 item 3); calibration (§4.7) in both environments; TP-06 fitted profiles (after EXP-PLANNER-006 tuning); TP-07; TP-08; TP-16; TP-19 `research/planner/tools/stage_windows_items.py`; `research/planner/tools/guidance_census.py`; `make_timing_spec.py`.
- **Gates:** G-A; G-B; the round-2 method review for part (d)'s analogy.
- **Held-out:** EXP-PLANNER-012. Cross-architecture replication: EXP-PLANNER-011 (BLOCKED).
