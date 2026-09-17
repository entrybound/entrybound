# Evaluation domain (program sections 6, 7, 34, 35): cross-domain contracts

The evaluation experiments (EXP-EVAL-001 to EXP-EVAL-013) supply shared references, gates and final-evaluation machinery that other domains' protocols rely on. This file states those contracts once. Where it differs from `research/decision-method.md` (pre-registration commit `14b977c`), the method governs.

| Field | Value |
|---|---|
| Designer | Phase C-design evaluation-domain session, 2026-09-17. It constructed no ledger candidate set. |
| State of evidence | No EXP-EVAL experiment has run. Spec files were validated with `ebr validate`, and `ebr plan` (dry run, nothing executed) resolved the items of EXP-EVAL-001 `spec.yaml`, shard `large-s0` and EXP-EVAL-009. |
| L7 disclosure | Recorded in `EXP-EVAL-001/protocol.md` ("Author, independence and exposure"): held-out identifiers and upstream sources seen via `PROGRESS.md` and a grep of `research/corpus/critique-round2.md`, in families F01, F04, F07, F08, F11-F17, F19 and F20. No content, listing, statistic or held-out path was opened. Analyses designed here are assigned to unexposed sessions. |

## 1. Timing and memory gate (every domain)

- A decision-grade timing or memory verdict in any domain requires two passes for its (environment, metric family, affinity configuration):
  - **EXP-EVAL-003** (instrument validation, including the harness review R1-16 fix);
  - **EXP-EVAL-004** (§4.7 calibration).
- The §4.7 names map onto EXP-EVAL-004 arm ids: `EXP-CAL-AA-<env>` is `EXP-EVAL-004-AA-<env>-<workload>`, and likewise `-AAP-`, `-KE-`, `-BG-` and `-SPD-`. A strengthening re-run appends `-r<k>`.
- Until the R1-16 fix lands, analyses read Linux external-process memory only from rows with `measurement.method = gnu-time-v+wait4` (harness review use constraint 3).
- Harness in-process timers stand in for production timing only if EXP-EVAL-003 arm P (pack and unpack) or arm P-access (random reads) shows `EQUIVALENT`. Otherwise timing claims use the production `ebound` binary (use constraint 2).

## 2. Shared references

- **REF-PROD binary.** `ebound` built from the production workspace at `9e44608e5a3ed44fb8f2c4cc5423a6f69b0d0155` with `cargo +1.98.1 build --release --locked -p entrybound-cli`. Paths: `/root/eb-research/target/refprod/release/ebound` (WSL) and `D:/eb-research/target/refprod-win` (Windows). The SHA-256 is recorded in run meta. The build recipe is in `EXP-EVAL-002/protocol.md`.
- **REF-FROZEN.** The same recipe at the design-freeze Commit A, used by the pre-unlock validation reruns and by EXP-EVAL-012.
- **REF-INC and specialists (objective §1.3).**
  - Size rows come from `research/normalized/EXP-EVAL-001/merged/best_incumbent_per_item.csv`, joined from EXP-BASE-SIZE plus EXP-EVAL-001, never re-measured.
  - Timing and access rows come from EXP-EVAL-005.
  - Capability rows (tamper, names, recipients, signatures, fidelity, safe extraction, streaming and access marks) come from EXP-EVAL-006.
- **Fair-comparison rules.** `research/baselines/README.md` rules 1-5 apply to every incumbent comparison. In particular, items that Entrybound refuses are never dropped from the incumbent side. The refused share is reported.

## 3. Held-out (every domain)

- **No domain writes held-out specs before the freeze.** At Commit A, `research/decisions/tools/make_heldout_specs.py` (EXP-EVAL-012) derives every held-out spec from the frozen validation specs, by the same selection rule applied to split `heldout` and to `heldout-sealed`.
- **R5 for every frozen decision** runs once, inside EXP-EVAL-012, with three reporting strata: sealed only, identity-aware only, and combined. Any pass that includes identity-aware items carries the `identity_aware_heldout` cap.
- **Pre-freeze audits** (content overlap at most 0.01 per family and split pair, transcript L7/L1 audit, materialization, same-upstream tuning, public-benchmark tagging, §5.8 (a)-(h)) are EXP-EVAL-010. The content-overlap audit runs **before** the held-out lock freeze.
- **Same-upstream tuning check.** Before a decision's first validation look, its experiment's `record.md` must list every public tuning data source. `research/corpus/tools/upstream_lineage_check.py` (EXP-EVAL-010) records PASS or FAIL only.
- **Sealed tranche S1** (method review G01) is EXP-EVAL-011. The owner holds the seed and commits its hash before Commit A. Items are drawn after Commit A and before Commit C. Only `research/corpus/sealed/manifest.public.json` (opaque ids) is ever visible to design and analysis sessions. Tranche size follows the EXP-EVAL-007 Q3 sizing rule.

## 4. Stable-v1 dispositions (program §34)

EXP-EVAL-008 assigns every program-§34 decision exactly one v1 disposition: `STABLE_V1`, `V1_FEATURE_GATED`, `POST_V1`, `REJECTED` or `RELEASE_BLOCKING_UNRESOLVED`. It does so by the admissible rubric applied to the final ledger. It does not select technical candidates.

Owning domains must supply two ledger schema-v2 inputs for their decisions:
- `add_later_class`: `ADDABLE_COMPAT`, `ADDABLE_INCOMPAT` or `NOT_ADDABLE`;
- `depends_on` capability edges.

Two independent sessions assess both. A disputed `add_later_class` resolves to the more restrictive class.

## 5. Negative results and the final evaluation (program §35)

- Every experiment writes `outcome.json` (§10.1). EXP-EVAL-008 arm C audits this.
- Every outcome where a named incumbent beats Entrybound beyond the band, in any domain, is listed in the negative-results register. EXP-EVAL-012 compiles these into `research/reports/final-system-evaluation/` under "workloads where Entrybound is not best".
- macOS/APFS and native ARM64 claim scope is EXP-EVAL-013 (BLOCKED). Decisions whose scope includes those platforms either exclude them explicitly or stay `INSUFFICIENT_EVIDENCE`.
