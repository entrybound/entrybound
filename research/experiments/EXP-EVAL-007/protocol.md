# EXP-EVAL-007: Corpus composition and aggregation-weighting sensitivity, incumbent tuning parity, and held-out power feasibility (incumbent-only analysis)

| Field | Value |
|---|---|
| Domain | evaluation (program sections 6, 7, 34, 35) |
| Status | NEEDS_TOOLING (analysis script; inputs EXP-BASE-SIZE and EXP-EVAL-001 complete; `research/methods/workload-mix.json` committed first, §14 item 13) |
| Kind | Analysis-only experiment on **incumbent-only** outputs, which are not decision-relevant under §0.1. It informs program-method decisions (corpus, weighting, tranche sizing) without touching any product candidate. No runner spec: it consumes normalized outputs. |
| Ordering rule (pre-registered) | This analysis runs only **after** `workload-mix.json` is committed. The workload-mix author must not have seen this experiment's outputs (Appendix B.3), and this experiment can never change base weights. It reports sensitivity and feasibility only. |
| Method | `research/decision-method.md` §4.9-§4.12, §5.3-§5.5, §6.5, Appendix B, Appendix D.1/D.7 at `14b977c` |

## Author, independence and exposure

Designed by the Phase C-design evaluation-domain session. Held-out information used here is limited to the published group counts per family ({1:1, 2:4, 3:10, 4:4, 6:1}, `decision-method.md` §4.9), which §5.4 permits. The analysis session must be one without a recorded L7 exposure, because the sealed-tranche sizing it produces constrains EXP-EVAL-011. The recorded L7 exposure of this designer is in the EXP-EVAL-001 protocol.

## Question

1. **(Q1, DEC-ECO-077)** Do conclusions drawn on the ebrc corpus depend on how it is composed and weighted? The test is whether the ranking of incumbent configurations by corpus-level `artifact_bytes` changes across these views: equal family weights, the cited workload mix, byte weighting, public-benchmark items only, real items only, grouped cross-validation folds, leave-one-family-out, WM-* mixes and scale tiers.
2. **(Q2, DEC-ECO-031)** Tuning parity for incumbents: when the best incumbent configuration per family and capability class is selected on tuning, does it stay within band of the best on validation? How large is the gap between defaults and tuned configurations?
3. **(Q3, DEC-ECO-076 and DEC-ECO-077)** Power feasibility: what corpus-level MDE, in band units, do realistic effect variances give on the validation structure, the existing held-out structure and candidate sealed-tranche structures? What is the smallest sealed tranche that meets R5's held-out MDE limit of 2 band units and the §4.9 minimum support?

## Hypotheses and falsification

- **H1.** The ranking under the cited workload mix agrees with equal weights and with the WM-* mixes (Kendall tau-b ≥ 0.8; top-1 preserved), but not with public-benchmark-only items (tau-b < 0.7).
  - Falsified if public-benchmark-only rankings agree (tau-b ≥ 0.8), which would weaken the case against the `public-standard-benchmarks` candidate.
  - Also falsified if the equal-weight or WM-* views disagree (tau-b < 0.8), which would show that weighting drives conclusions. Every decision must then report its equal-weight and mix arms as scope limits, not footnotes.
- **H2.** Grouped 5-fold cross-validation by independence group reproduces the single tuning/validation split's top-3 incumbent configuration per family in at least 80% of folds.
  - Falsified if it does not. The single split is then fragile, which is evidence for the `grouped-k-fold-cross-validation` candidate.
- **H3.** The tuning-selected best incumbent per (family, capability class) is within 1 band (T-01) of the validation-best in at least 90% of (family, class) cells, and defaults are 1 or more bands worse than tuned in most families.
  - Falsified if under 90% of cells hold. Tuning parity then needs per-family re-selection on validation-equivalent data for incumbents too, and Entrybound claims against "tuned incumbents" must use the validation-best incumbent.
- **H4.** On the existing held-out structure, the held-out MDE for T-01 corpus comparisons is at most 2 band units for effects whose validation group-level standard deviation is at most the pooled median of the incumbent pairs. A sealed tranche of at least 2 groups in at least 10 families meets the same limit.
  - Falsified if the minimal tranche needs 3 or more groups per family, or more than 15 families. EXP-EVAL-011's size then rises accordingly (sizing rule below), or the sealed route is recorded as infeasible within budget.

## Decisions informed

`DEC-ECO-077`, `DEC-ECO-031`, `DEC-ECO-076`.

## Candidates

These are views and structures, not product candidates.

- **Q1 aggregation views** (candidates of DEC-ECO-077 made measurable):
  - V1 cited workload mix (status quo base weights, B.3);
  - V0 equal family weights;
  - V2 byte-weighted ratio of totals (`total_stored_bytes`);
  - V3 public-benchmark-tagged items only (Silesia, enwik8, Kodak, Blender open movies; manifest `tags` or `pipeline`);
  - V4 excluding public-benchmark items;
  - V5 real items only (`real_or_generated = real`);
  - V6 leave-one-family-out (20 views);
  - V7 grouped 5-fold cross-validation over the pooled tuning and validation groups (seeded fold assignment, whole groups only; within each fold, rank on 4 folds and evaluate on the held-back fold);
  - V8 WM-DIST, WM-CI, WM-BACKUP, WM-DATA, WM-MEDIA (Appendix B.2: 0.8/0.2 rule);
  - V9 per scale tier;
  - V10 excluding the cross-split covariate items flagged by corpus critique round 2 (F03 shared packages, F17 kernel headers).
- **Q2 capability classes** (from EXP-EVAL-001 `best_incumbent_per_item.csv`):
  - all configurations;
  - random-access-capable (`7z-*-nosolid`, `zip-*`, `squashfs-*`, `dwarfs-*`, `tarxz-pixz`);
  - streaming tar pipelines;
  - dedup-capable (`borg`, `restic`, `dwarfs`, `tarzstd-22-ultra-long27-t1`).
  - Defaults: `targz-gzip-6`, `zip-6`, `7z-mx5-solid`, `tarzstd-3-t1`, `tarxz-6`.
- **Q3 incumbent effect pairs**, fixed now and spanning small to large effects:
  - `tarzstd-19-t1`/`tarzstd-3-t1`
  - `7z-mx9-solid`/`tarxz-6`
  - `7z-mx9-nosolid`/`7z-mx9-solid`
  - `zip-9`/`zip-6`
  - `tarzstd-22-ultra-long27-t1`/`tarzstd-19-t1`
  - `dwarfs-l7`/`squashfs-zstd`
  - `tarbrotli-q11`/`tarxz-9e`
- **Q3 structures:**
  - (a) validation: {1:1, 2:9, 3:9, 4:1};
  - (b) existing held-out: {1:1, 2:4, 3:10, 4:4, 6:1};
  - (c) sealed tranches: k groups per family, k in {1, 2, 3}, over F families, F in {5, 10, 15, 20};
  - (d) combined (b) plus (c), reported with the tranche-stratified caveat of EXP-EVAL-012.

## Corpus selectors

Tuning and validation items only, via `research/normalized/EXP-BASE-SIZE/` and `research/normalized/EXP-EVAL-001*/` (merged by `research/baselines/merge_size_pass.py`). Samples with `roundtrip_ok = 0` are excluded from ranking (correctness gating) and reported. Cells capped by EXP-EVAL-001 are handled per configuration: rankings on medium-high and large items use only configurations run on every item of that stratum.

## Environment

Any host (pure analysis, stdlib plus the research venv). Windows `C:/Python313/python.exe` or WSL venv.

## Commands

```sh
# TO BE WRITTEN: research/experiments/EXP-EVAL-007/analyze.py
C:/Python313/python.exe research/experiments/EXP-EVAL-007/analyze.py \
  --size-pass research/normalized/EXP-EVAL-001/merged/item_config.csv \
  --manifest research/corpus/manifest.json \
  --workload-mix research/methods/workload-mix.json \
  --heldout-group-structure "1:1,2:4,3:10,4:4,6:1" \
  --seed 20260932 --out research/normalized/EXP-EVAL-007/
# outputs: q1_rank_views.csv (view, config, rank, geomean ratio), q1_tau.csv (view vs V1: tau_b, top1_preserved, flip_fraction),
#          q2_parity.csv (family, class, tuning_best, validation_best, regret_bands, default_gap_bands),
#          q3_mde.csv (pair, metric, structure, se_log, mde_bands), q3_min_tranche.json, provenance headers (§12.3)
```

The script reads no held-out row. It takes the held-out structure only as the literal counts argument above, and it refuses a manifest row whose `split` is `heldout` (it filters before any other processing).

## Seeds

- Grouped fold assignment: 20260932.
- Dirichlet family-weight draws (the Q1 robustness check, 10,000 draws at concentration 4.0): 20260933.

## Warmup

Not applicable.

## Measurement method and metrics

- Input metric: `artifact_bytes` (T-01), as the item-level log ratio to input bytes, and its pairwise log ratio between configurations.
- **Q1:** Kendall tau-b (`stats.kendall_tau`), top-1 preservation, pairwise flip fraction.
- **Q2:** selection regret in continuous band units, `q = |ln(v/v_best)| / ln(1.01)` (§6.2), per (family, class).
- **Q3:** SE and MDE in log units and band units, computed exactly as §4.10 (Welch-Satterthwaite, family weights) and §4.12 (`MDE = (t_{df,1-α} + t_{df,0.80}) x SE`; held-out α one-sided 0.05). The Q3 group-level variance comes from the validation split of each incumbent pair, and it is also inflated by factors 1.5 and 2 as a heteroscedasticity sensitivity.
- **Timing extension.** When EXP-EVAL-005 incumbent-only tuning rows exist, the same Q3 computation for `decode_wall_s` (T-04) is appended. Until then, T-04 uses EXP-EVAL-004 A/A item half-widths as a lower-bound variance and is labelled so.

## Replication and adaptive rule

None. The inputs are deterministic byte metrics, already repeat-hash checked. The bootstraps and folds are seeded.

## Normalization

§4.9 levels L1-L4 with each view's weights. Absolute floors do not apply (ratios to input bytes and between configurations).

## Statistical analysis

Descriptive robustness statistics plus the analytic MDE formula. No verdict about any product decision is produced.

**Pre-registered sizing rule for EXP-EVAL-011.** Choose the smallest (k, F) from `q3_min_tranche.json` such that:

- F ≥ 10 and k ≥ 2 (§4.9 minimum support, plus the family guard's 2 groups);
- for at least 90% of the incumbent pairs at median variance, the T-01 held-out MDE is at most 2 band units on the sealed tranche alone;
- the same holds at 1.5x variance for at least 75% of pairs.

If no (k, F) with F ≤ 20 and k ≤ 3 meets this, EXP-EVAL-011 uses k = 3 and F = all feasible families, and records the sealed-route MDE limitation.

## Practical-significance thresholds

T-01 (`artifact_bytes`, r = 0.01) for band units. The held-out MDE limit is 2 band units (R5, `thresholds.json` `statistics.mde_max_bands`). Kendall tau cut-offs (0.8, 0.7) and the 80% and 90% fractions are pre-registered descriptive criteria for this method-evidence experiment, not decision bands.

## Sensitivity analysis

- Family Dirichlet at concentration 4.0 around V1: fraction of draws preserving the top-1 configuration.
- Variance inflation 1.5x and 2x for Q3.
- With and without covariate-flagged items (V10).

## Expected negative results worth recording

- Byte-weighted rankings dominated by the few large F05, F07 and F15 items (V2 disagreeing with V1).
- Public-benchmark-only views ranking `zpaq` and `7z` differently.
- Tuned-best incumbents not transferring to validation in F11-F13 (already-compressed media), where differences sit inside the band.
- A sealed tranche needing 3 groups per family.

## Threats to validity

- **Incumbent effects as proxies.** Incumbent pairs may have different variance structures than Entrybound candidate comparisons. Q3 is feasibility evidence, and every decision recomputes its own MDE (§4.12).
- **Cross-split covariates** (F03 packages, F17 kernel headers) can inflate apparent transfer in Q2. The V10 view reports them.
- **Workload-mix dependence.** If `workload-mix.json` is absent at execution time, V1 is omitted and V0 becomes the reference. The output is then labelled `partial` and cannot inform DEC-ECO-077's weighting choice.

## Platform requirements

Any (analysis only); no network; no privileges.

## Estimates

- Machine time: under 1 h.
- Disk: under 100 MB.
- Agent effort: scripted analysis; judgment for the interpretation, by an unexposed session.
