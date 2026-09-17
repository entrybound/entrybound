# EXP-PLANNER-012: Planner-domain held-out confirmation (Phase D placeholder)

| Field | Value |
|---|---|
| Domain | planner (program §9, §10) |
| Index status | **BLOCKED**: Phase D. Requires the single program-wide design freeze and unlock sequence of `research/decision-method.md` §5.5: Commit A with provisional selections, Commit B `research/decisions/design-freeze.json` with `design_freeze_sha`, and Commit C `research/corpus/heldout-unlock.json`. Also requires `research/corpus/heldout-lock.json` at status `frozen` (it is `draft` at design time), gate G-C (lineage/content-overlap audit, transcript audit, removal or encryption of materialized held-out content), and the G01 sealed-tranche decision |
| Document state | Placeholder by instruction. No held-out identities, listings, statistics or paths were consulted or are named here (§5.4) |
| Shared conventions | `research/planner/README.md` |
| Runner spec | none at design. Held-out specs are written into Commit A (§5.5 item 1) by copying each planner experiment's look-1 specs with `splits: [heldout]` and the frozen candidate lists; `ebr run` refuses them until `EB_HELDOUT_UNLOCK` equals `design_freeze_sha` |

## 1. Pre-registration header

- **decision_ids:** every planner-domain decision whose provisional selection is frozen in Commit A. Candidates at design: DEC-CMP-002, -003, -004, -006, -009, -012, -013, -015, -016, -019, -020, -022, -025, -029, -030, -031, -032, -033, -034, -035, -036, -039, -040, -045; DEC-ACC-003, -012, -029; DEC-CON-010; DEC-CRY-028; DEC-INT-046; DEC-MOD-039.
- **Decision type:** R5 applies to EMPIRICAL and HUMAN-FACING mechanical parts. FORMAL and EXTERNAL parts skip R5 and record why (§2 R5, last paragraph).
- **Author / L7:** README §10. The designer's identity exposure applies. Held-out-aware sessions must not design or analyse (§5.4 item 3), so the held-out analysis is run by a session without recorded exposure to the families concerned, or the exposure is recorded.

## 2. What runs (after Commit C only)

Every candidate that reached the freeze is run, not only W (R5).

| Source experiment | Held-out run |
|---|---|
| EXP-PLANNER-001 | Census of the four profiles × two layouts on held-out items (HC round trip, repeat-hash, research-baseline identity) |
| EXP-PLANNER-002 | Oracle traces on held-out PSUB-256 sub-items. Sub-items are derived **after** unlock by `make_psub.py` from held-out content verified against the frozen lock; derived held-out content is held-out content (§5.4) |
| EXP-PLANNER-003 to 006 | Materialization and re-plan cells for the frozen W ∪ S per decision and profile |
| EXP-PLANNER-005 | Chunk-count R2 screen on full held-out items |
| EXP-PLANNER-007 and 008 | Timing on the held-out timing subset (TS-1-H, selected by the TS-1 rule with key prefix `"TS-1-H"` at Commit A from fingerprint-level fields only: `family`, `scale`, `independence_group` and `item_id` hashes; §5.4 allowed fields) |
| EXP-PLANNER-009 and 010 | HC identity and replan checks on held-out sub-items (HC tests must pass on every split, §8.2 item 13) |

## 3. Pre-unlock obligations (Commit A)

1. **Held-out MDE (R5, §4.12).** For every supporting comparison: from validation group-level variance and held-out group counts per family (fingerprint-level fields only). A comparison with MDE above `statistics.mde_max_bands.heldout` routes its decision to `INSUFFICIENT_EVIDENCE` before unlock; its results are still produced as report-only.
2. **Frozen specs, tooling and hashes** per §5.5 item 1, including `research/planner/tools/*` at their exact commit, `fitted-profiles.json`, `reference-table.json`, and the TP-03 replay outputs used for W and S.
3. **Upstream-lineage check (§5.4 item 2).** A held-out-aware session records pass or fail only, for the public tuning sources of the learned or fitted candidates (tree-d6, lut-cat-probe, pruned and corpus-fitted tables, retuned probe, corrected constants, TNB-fitted bounds, reference decode table).
4. **Sealed tranche.** If the G01 sealed supplementary held-out set exists and meets §4.9 support for a decision's families, it is used and lifts the `identity_aware_heldout` cap. Otherwise the cap applies to every R5 pass (§5.4, §8.4).

## 4. Analysis (after unlock; no re-selection)

- R1, R2, R4, R5 (a)-(f) and R7 re-run on held-out with confidences from `bootstrap.heldout_noninferiority_confidence` and the replication classes of R5 (c).
- Stability bars per §6.6 on held-out results.
- Results are report-only when a decision was routed to `INSUFFICIENT_EVIDENCE` by the MDE gate.
- Re-runs only for infrastructure failures (§5.6), every attempt kept.
- Post-unlock fixes are labelled `post-unlock-fix` and cannot make a candidate `DECIDED` on this evidence (§5.6).
- Leakage events per §5.7; audits per §5.8.

## 5. Seeds, metrics, practical significance, sensitivity, negatives, threats

- **Seeds:** order 260917121, bootstrap 260917122, command 260917123. The source experiments' seeds are reused for their own held-out replays.
- **Metrics and thresholds:** exactly the source experiments' primary and guard metrics. Practical significance comes from the same `research/methods/thresholds.json` keys, frozen by Commit A.
- **Sensitivity:** §6.6 bars re-evaluated on held-out (R5 f), including §6.5 arms and the §6.7 bootstrap.
- **Expected negative results worth recording:**
  - attenuated replication of small size gains, for example strategies or tables within 1-2 band units;
  - learned or fitted candidates losing on held-out families absent from tuning sources;
  - identity checks unchanged.
- **Threats to validity:**
  - identity-aware held-out (G01) unless a sealed tranche exists;
  - group counts of 1-6 per family limit the held-out MDE, so many decisions may route to report-only;
  - PSUB derivation on held-out must reproduce the frozen rule exactly;
  - the designer's L7 exposure (README §10).

## 6. Cost, platform and status

- **Estimated compute:** about the validation-look totals of EXP-PLANNER-001 to 010 restricted to frozen candidates: about 500 CPU-hours deterministic plus about 200 hours of exclusive timing wall across WSL and Windows.
- **Disk:** about 60 GB transient under `/root/eb-research` (on D:).
- **Platforms:** as the source experiments. ARM64 and macOS stay excluded from scope (EXP-PLANNER-011).
- **Agent effort:** scripted execution; judgment for the R5 write-up by a session satisfying §5.4 item 3.
- **Status:** BLOCKED until Commit C.
