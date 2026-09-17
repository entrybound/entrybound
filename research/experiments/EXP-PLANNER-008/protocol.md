# EXP-PLANNER-008: Deterministic decode-cost model for `min_decode_rate`

| Field | Value |
|---|---|
| Domain | planner (program §9, §10) |
| Index status | NEEDS_TOOLING (TP-09 `ebr-codec decode-matrix`, `research/planner/tools/make_decode_bundles.py`, TP-19 Windows staging, runner additions and calibration including the `st-e` core class; cross-architecture parts in EXP-PLANNER-011, BLOCKED) |
| Document state | Phase C design; `record.md` after gate G-A |
| Shared conventions | `research/planner/README.md` |
| Runner specs | `spec.yaml` (WSL `st-v`); `spec-windows-p.yaml` (Windows `st-p`, P-core, the declared reference machine class); `spec-windows-e.yaml` (Windows `st-e`, E-core) |

## 1. Pre-registration header

- **decision_ids:** DEC-CMP-015.
- **Decision type (proposed):** EMPIRICAL. That no model reads the environment at pack time is FORMAL (definition review plus the EXP-PLANNER-009 repeat-hash across hosts).
- **Proposed `od_ids`:** OD-07 (decode performance: the quantity the model predicts), OD-05 (size under a floor), OD-17 (`declared_tightness_ratio` of the declared rate). **Proposed `hc_ids`:** HC-11 (decoded output independent of CPU features), HC-13 (the model is declared data, so planning is deterministic), HC-16.
- **Frame:** profile scope, constrained form (Appendix B.1). The floor is a profile bound. A model that lets a declared floor be missed on an in-scope machine class is eliminated at R2 (declared bound not honoured, by analogy with R2's "measured values never exceed the declared bound").
- **Author / L7:** README §10.

## 2. Question and hypotheses

- **Question.** Which deterministic decode-cost model should the planner use to enforce `min_decode_rate` (logical bytes per second per core on a declared reference machine)? The model must rank candidate plans the way real hardware does, keep declared floors honest on other core classes, and cost the least size.
- **H1.**
  1. `reference-machine-measured-table` (measured on `st-p`) has Kendall τ-b ≥ 0.9 against measured decode time per byte on `st-e` and on WSL `st-v`.
  2. `codec-class-tiers` has τ-b ≥ 0.8.
  3. Under every EXP-PLANNER-006 floor tier, plans selected with the measured table satisfy the floor (`declared_tightness_ratio` ≥ 1) on `st-p` and `st-v`, but miss it on `st-e` for LZMA2-heavy plans unless the floor is declared per core class.
  4. `static-cost-table` is `NON_INFERIOR` in size to the measured table at every floor.
- **H0.** Every model yields the same selections at every floor tier (`EQUIVALENT` sizes, no violations). `status-quo-implicit` or `drop-decode-rate-constraint` then wins by R10.
- **Falsification.** H1.1 or H1.2 fail if τ-b falls below its bound on any in-scope machine class. H1.3 fails on any floor violation on `st-p` or `st-v`, or if no `st-e` violation occurs for LZMA2-heavy plans. H1.4 fails if the static table is not non-inferior at look 1.

## 3. Decisions informed

| Decision | Supplied here | Elsewhere |
|---|---|---|
| DEC-CMP-015 | Rank correlation of modelled against measured decode throughput across two microarchitectures (Raptor Cove P-core, Gracemont E-core) and one virtualized vCPU class; selection differences between models; sensitivity of profile outcomes to model error | Other CPUs and architectures (ARM64 native, AMD x86-64): EXP-PLANNER-011 (BLOCKED); floor values: EXP-PLANNER-006 (DEC-CMP-035) |

## 4. Candidates

| candidate_id | Definition | Expected tier |
|---|---|---|
| status-quo-implicit (reference) | No model; the candidate tables implicitly bound decode speed | status quo |
| static-cost-table | A priori integer cost per MiB per configuration, fixed at design without measurement: STORE 1; LZ4 2; Zstandard any level 3; LZMA2 4+1 MiB 12, 6+4 MiB 14, 9+8 MiB 16; +1 per structural transform; DEFLATE reconstruction 25; JPEG reconstruction 60. These are inferred design constants, not evidence | T1 (planner parameter) |
| codec-class-tiers | Ordinal classes only: {STORE} < {LZ4} < {Zstandard} < {LZMA2} < {reconstruction}. A floor tier admits the classes up to a bound | T1 |
| reference-machine-measured-table | Integer nanoseconds per MiB per (configuration, TP-05 category), measured on **tuning** bundles on the declared reference class `windows-host st-p`, rounded to 2 significant digits, frozen per planner ID | T1 (plus C7 permanent re-measurement obligation per planner ID) |
| drop-decode-rate-constraint | `min_decode_rate` removed from the parameter set (a SPEC §6.0a deviation, recorded in C1 as a specification change) | T1, plus a SPEC revision outside the program's authority (flagged) |
| critic-proposed-n | R0 item 6 slot | – |

- **Excluded:** `runtime-measurement`. R1 by HC-13 and SPEC §5.7 L596-605 (the planner may not read the machine). This is the ledger's exclusion; independent sign-off is pending.

## 5. Corpus

- **Chunk bundles:** from EXP-PLANNER-002 tuning traces, `research/planner/tools/make_decode_bundles.py` selects up to 64 unique Chunks per (family, TP-05 category) in SHA-256 order and writes one bundle item per (family, category). The manifest is `research/planner/derived/decode-bundles.json`, with the parent item's independence group recorded per bundle as the majority group. Bundles mixing groups are flagged, and their L2 aggregation uses Chunk-level group assignment.
- **Configurations:** every codec and transform configuration in the v4-v6 tables, plus DEFLATE and JPEG reconstruction decode where the category applies (encoded untimed in the bundle setup).
- **Look 1:** validation bundles from validation traces.
- **Selection-effect arm:** the EXP-PLANNER-006 `decrate-t1/t2/t3` materialized archives (tuning; W ∪ S at look 1) are decoded with `ebound unpack` inside EXP-PLANNER-007 windows on each machine class.

## 6. Environment and platform

| env / class | Affinity | Qualifier |
|---|---|---|
| `windows-host` `st-p` {2} | P-core (declared reference machine class) | Defender/VBS covariates (§4.2) |
| `windows-host` `st-e` {18} | E-core | as above; speedups across classes never compared (§4.2) |
| `wsl-ubuntu` `st-v` {2} | vCPU | `VIRTUALIZED` |

Timing windows are exclusive. Calibration (§4.7) for `time_wall` and `throughput` in all three classes is required, and `st-e` needs its own A/A. The Windows runner additions (power-throttling opt-out, frequency covariate, per-core-class guard) are required (§14 item 3). No network or external review. Cross-architecture scope: **x86-64 hybrid only**. The decision scope explicitly excludes ARM64 and other vendors until EXP-PLANNER-011 runs (§4.1, §8.4).

## 7. Commands

```sh
python3 research/planner/tools/make_decode_bundles.py --traces /root/eb-research/raw-side/EXP-PLANNER-002/traces --split tuning --per-cell 64 \
  --out-root /root/eb-research/derived/planner/decode-bundles --out-manifest research/planner/derived/decode-bundles.json
python3 research/planner/tools/stage_windows_items.py --manifest research/planner/derived/decode-bundles.json --dest D:/eb-research/derived/planner/decode-bundles-win \
  --out-manifest research/planner/derived/decode-bundles-windows.json
PYTHONPATH=research/tools /root/eb-research/venv/bin/python -m ebr run --timing --spec research/experiments/EXP-PLANNER-008/spec.yaml
# Windows host (PowerShell 7, research venv for Windows per research/tools/run_tests_windows.ps1)
python -m ebr run --timing --spec research/experiments/EXP-PLANNER-008/spec-windows-p.yaml
python -m ebr run --timing --spec research/experiments/EXP-PLANNER-008/spec-windows-e.yaml
python3 research/planner/tools/decode_cost_model.py fit --normalized research/normalized/EXP-PLANNER-008 --reference windows-st-p --split tuning \
  --out research/normalized/EXP-PLANNER-008/decision/reference-table.json
python3 research/planner/tools/decode_cost_model.py evaluate --tables research/normalized/EXP-PLANNER-008/decision/reference-table.json \
  --static research/experiments/EXP-PLANNER-008/protocol.md#4 --out research/normalized/EXP-PLANNER-008/decision/
```

`reference-table.json` is produced from tuning only and is consumed as an input by EXP-PLANNER-004 (arm C) and 006 (`decrate-*`) before their looks.

## 8. Seeds, warmup, replication

Seeds: order 260917081, bootstrap 260917082, command 260917083. Each sample is an in-process batch of at least 1,000 decode operations or at least 50 ms (§3.2 T-08 batch rule applied to per-Chunk decode). Warmups 2 (§4.5). Rounds follow `protocol.timing_rounds.small` (bundles are small-tier inputs): minimum 10, raised to n_min for the confidence in use, with the adaptive step and cap of §4.6. Randomized blocks, paired within rounds across configurations.

## 9. Metrics

| Metric | OD | Role | Family | Threshold | Source |
|---|---|---|---|---|---|
| `decode_throughput_bps` per (configuration, category, class) | OD-07 | primary for the model-accuracy measurement | throughput | T-05b | TP-09 |
| `decode_wall_s` (in-process batch per operation) | OD-07 | secondary | time_wall | T-04 (in-process floor) | TP-09 |
| `declared_tightness_ratio` = (1 / declared floor) ÷ (measured seconds per byte) for selected plans | OD-17 | primary R2 screen (< 1 is a violation) and graded guard (T-23) | other | T-23 | selection-effect arm |
| `artifact_bytes` under each model and floor | OD-05 | primary superiority | size | T-01 | EXP-PLANNER-006 `decrate-*` cells re-selected per model by TP-03 |
| Kendall τ-b of model cost against measured time per byte | – | descriptive (`stats.kendall_tau`) | other | none | `decode_cost_model.py` |

## 10. Normalization and statistical analysis

1. **Item level:** per (bundle, configuration, class), the exact order-statistic interval of the median per-round throughput (§4.10). Group and family levels per §4.9.
2. **Model accuracy:** τ-b per machine class between each model's cost and the measured median seconds per byte across (configuration, category) cells, with a percentile bootstrap over bundles (informational, §4.10 bootstrap rule).
3. **R2 screen:** for each model, floor tier and in-scope class, selected plans with `declared_tightness_ratio` < 1 at the item level (the exact interval's upper bound below 1) eliminate the model for that profile scope. Classes are in scope per the decision's declared scope; a class-specific floor is a separate candidate variant added by the record before look 1 if H1.3's `st-e` pattern appears on tuning (logged addition, R0).
4. **Among survivors (B.1 per profile):** superiority on `artifact_bytes` at the profile's floor; guard `declared_tightness_ratio` against the T-23 band; R3-R10 with look 1; confidences from `bootstrap.*`.
5. **Model-error sensitivity:** selections recomputed with measured costs perturbed by ±1 band of T-05b on each class. The change in selected plans and size is reported ("sensitivity of profile outcomes to model error").

## 11. Sensitivity analysis

§6.4 threshold perturbation; §6.5 family and mix arms; environment arm (P-core against E-core against WSL, same-direction requirement §4.1); §6.7 bootstrap. Table rounding arm: 1 against 3 significant digits.

## 12. Expected negative results worth recording

- Measured-table ranks on the P-core disagree with the E-core for reconstruction and LZMA2 configurations, so a single reference machine cannot promise floors on hybrid CPUs.
- Coarse tiers suffice (τ-b high), and the measured table buys no size.
- `drop-decode-rate-constraint` wins by R10 because no floor ever binds on tuning.

## 13. Threats to validity

- One host, two x86 microarchitectures and one virtualized class: no ARM64, AMD or older Intel (EXP-PLANNER-011).
- In-process research build: the codec paths are production functions via `research-internals` (codegen caveat R1-06).
- The bundle sample is at most 64 Chunks per cell.
- Thermal and power-limit effects on the E-cores (`power_limit_confound` flag, §4.2).
- The static table constants are designer-chosen (flagged `INFERRED`).
- Designer L7 exposure.

## 14. Cost and effort

- **Timing:** about 25 configurations × about 40 bundles × at least 12 rounds × 3 classes; about 30 hours of exclusive wall time, plus the selection-effect decodes inside EXP-PLANNER-007 windows (about 10 hours).
- **Disk:** about 3 GB of bundles per host.
- **Agent effort:** scripted execution; judgment for the model evaluation write-up.

## 15. Gates and status

- **Tooling:** TP-09, `make_decode_bundles.py`, `decode_cost_model.py`, TP-19, runner additions for Windows and WSL, `st-e` calibration.
- **Gates:** G-A, G-B, EXP-PLANNER-006 floor values before `DECIDED`.
- **Blocked parts:** cross-architecture scope → EXP-PLANNER-011.
- **Held-out:** EXP-PLANNER-012.
