# Platform-domain common provisions (PCP) for EXP-PLAT-001 … EXP-PLAT-025

Part of the Phase C-design pre-registration of the platform domain (program sections 15–21). Every EXP-PLAT protocol cites these provisions by tag; a provision applies verbatim unless the protocol states an addition. Changing a provision after any decision-relevant result follows decision-method §0.2.

## PCP-1 — Author, independence and held-out identity exposure (L7)

This design session (Phase C-design platform-domain designer, workflow `wf_47347bc4-611`) read `research/corpus/coverage.md` as the Phase C-design rules required; that file lists item identifiers and independence-group names of every split, including held-out items of families the EXP-PLAT experiments use. No held-out content, listing, statistic, fingerprint or path under the held-out data roots was opened, and no held-out identifier appears in any EXP-PLAT protocol, spec or index fragment. This is an identity exposure (event L7, decision-method §5.7) to be entered in the L7 record (§5.8 h). Under §5.4 consequence 3 the gate audit decides whether the analysis plans of EXP-PLAT protocols must be re-derived or countersigned by an unexposed session for decisions whose applicable families include exposed items. This session did not construct any candidate: candidate sets are the ledger's R0 sets, reproduced mechanically from `research/decision-ledger.jsonl`.

## PCP-2 — Gates

| Gate (decision-method §0.4, §14) | Items this experiment depends on | Consequence while unmet |
|---|---|---|
| G-A | §14 item 10 constraint crosswalk and sign-off; item 12 ledger schema v2 (`hc_ids`, `od_ids`, `decision_type` assigned by an independent session); item 14 validation-look registry | Each EXP-PLAT protocol stays a design draft: the §12.2 `record.md` (pre-registration record proper) is committed only after G-A; nothing run before it is decision-grade. |
| G-B | item 1 thresholds test update; item 2 decision-analysis tooling; item 5 held-out lock frozen; item 13 workload mix (only where corpus-level L4 aggregation is used) | Runs may execute for instrument validation, but no result is decision-relevant. |
| G-C | item 15 held-out lineage/overlap audit, transcript audit, held-out materialization removal | Required before the Phase D held-out run placeholder below. |
| G-D | item 6 ledger validator; gate audit §8.6 | Required before any informed decision can become `DECIDED`. |

## PCP-3 — Seeds, warmups and replication for deterministic metrics

- **Seeds:** ebr `seeds` (order, bootstrap, command) = 20260917 in every spec; fixture/case generator seeds as stated under Corpus (tuning seed 20260917, validation seed 20260918, held-out seed sealed).
- **Warmups:** 0 — every metric here is deterministic (§4.5 warmups apply to timing and memory only).
- **Replication:** 2 repetitions per (candidate, item) in different rounds and processes (§4.6, `order: randomized_blocks`), plus the §4.13 repeat-hash check over the canonical observation document (`observation_sha256` string metric). All valid samples of an (env, candidate, item) must share one hash.
- **On repeat-hash mismatch:** where the candidate claims determinism the two differing outputs are committed as the violation artifact without a confirming rerun (objective §2.0 rule 2; §4.13) unless an infrastructure fault is proven; otherwise the metric is reclassified as stochastic and re-run with timing-style rounds (§4.6 small-tier minimum 10, step 10, cap 30) under the precision-only adaptive rule, which never looks at effect direction.
- **Volatile fields:** atime and ctime of restored objects are set by the restoring process itself; they are excluded from repeat-hash and from fidelity case lists by pre-registered rule (as in `research/baselines/README.md`).

## PCP-4 — Statistical analysis for coverage fractions, binary screens and exact byte metrics

- **Item level (L1):** exact values (deterministic, §4.13). T-20 fractions are computed over the pre-registered case list whose `n_cases` is written into the §12 record before any run; a verdict between two candidates on the same case list is exact: a difference beyond the metric's absolute rule is `DISTINGUISHABLE`, otherwise `EQUIVALENT` (§3.1, §4.11).
- **Aggregation:** AGG-S per evaluation condition (platform pair or target filesystem; conditions are never averaged, §1.1), headline = minimum over the class list fixed before results (objective OD-03 rule, computed mechanically from EXP-PLAT-001 observer capability); AGG-W (sum) for HC oracle counts; AGG-C for coverage by family where corpus items are used.
- **Corpus-item metrics:** L2 independence-group means, L3 family means, L4 weighted by `research/methods/workload-mix.json` with the Welch–Satterthwaite interval (§4.10) only when the §4.9 minimum support (≥10 groups across ≥5 applicable families in the split) holds; otherwise verdicts are family- or condition-scoped and labelled `single-group` where applicable.
- **Rule order:** R1 (HC oracle outputs and binary metrics: one reproducible violation eliminates the candidate for the scope) → R2 (declared bounds, specifiability) → R3/R4 on the primary metrics with the family/tier/condition guard → R6 minimum gain at the computed tier → R7 → R8/R9 (platform- or policy-scoped winners must be expressible, R9 (i)) → R10.
- **Confirmatory set (§4.12):** on validation only W versus each s ∈ S on the declared superiority metrics (confidence 1 − 0.01/k_sup), non-inferiority at 0.99, family/condition guard at 0.90; the MDE gate is computed before the look from tuning variance (for exact case-list fractions the MDE equals the case granularity and is recorded as such).
- **Production defects:** a violation found in the status quo creates a defect and ledger work item; it never weakens the HC (objective §2.0 rule 5).

## PCP-5 — Sensitivity analysis for coverage decisions

- **Threshold perturbation (§6.4):** each primary metric's band ×0.5 and ×2 (for exact case-list fractions the resolvability condition holds trivially); a changed selection sets `band_dependent`.
- **Case-list arms (pre-registered, reported with §6.5 'other arms'):** leave-one-class-out and leave-one-condition-out recomputation of every headline minimum; the fraction preserving the selection is held to the §6.6 bars (≥ 0.90 pass, [0.80, 0.90) MEDIUM).
- **Weights (§6.3):** equal base weights over the decision's OD set unless the §12 record declares otherwise; grid and Dirichlet FPW; SMAA whole-simplex report.
- **Environment arm (§6.5):** Windows host versus WSL where both are in scope; a claim covering both needs same-direction support in both (§4.1).
- **Selection-stability bootstrap (§6.7):** 2,000 replicates over independence groups wherever corpus items (not only fixtures) carry the metric.

## PCP-6 — Storage discipline

- Linux scratch and images under `/root/eb-research/scratch/platform/<EXP-ID>/` and mount points under `/root/eb-research/mnt/<EXP-ID>/` (WSL ext4 inside the D:-hosted distro). Loop devices are attached only through `mount -o loop` (auto-clear); `losetup -D` is never used because other workflows may hold loop devices.
- Windows scratch under `D:\eb-research\scratch\platform\<EXP-ID>\`, with `TEMP`/`TMP` redirected to `D:\eb-research\tmp` and `CARGO_HOME=D:\eb-research\cargo-home` for any new crate download, so nothing is written to C: (standing decision 2026-09-16). Read-only observation of C: locations (for example default ACLs of profile folders) is allowed; writes are not.
- `research/orchestration/disk_guard.py` must pass before each campaign; the orchestrator's disk alarm stays armed.
- Every script refuses input paths under a held-out root (the harness guard `ebr_common::heldout` semantics, including `\\wsl.localhost` forms).

## PCP-7 — Held-out evaluation placeholder (Phase D)

After the single program-wide design freeze (Commit A/B, decision-method §5.5) and unlock (Commit C), the same specs are re-targeted once to `split: heldout` for the same families, with `EB_HELDOUT_UNLOCK` set to Commit A's SHA; held-out MDEs are stated in Commit A from validation group variance and fingerprint-level group counts only (R5, §4.12). Generated fixtures use a sealed tranche: the held-out fixture seed is drawn by the program owner after the freeze and only its SHA-256 is committed beforehand (§5.4 sealing route), so these cells carry no `identity_aware_heldout` cap. No re-selection after unlock (§5.6); FORMAL parts skip R5 with the reason recorded.

## PCP-8 — Cost accounting

Cost tiers are computed, not self-assigned (§7.2): C1-C3 by the checked-in cost scripts (§14 item 7, not yet built) over each candidate's normative diff and dependency graph; C4-C7 by an assessor not involved in the candidates before the first validation look. This experiment supplies no tier by itself; where it measures an input (for example unsafe-code counts or reader fragmentation) the metrics table marks it `cost_input`.
