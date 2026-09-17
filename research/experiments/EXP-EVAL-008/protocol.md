# EXP-EVAL-008: Stable-v1 boundary scoring method (disposition rubric counterexample search and robustness), evidence-classification scheme reliability, and negative-results registry audit

| Field | Value |
|---|---|
| Domain | evaluation (program sections 6, 7, 34, 35) |
| Status | NEEDS_TOOLING (`research/tools/decide/v1_scope.py`, synthetic-ledger generator, property checker; ledger schema v2 fields `hc_ids`, `od_ids`, `decision_type`, computed cost tier, plus a new `add_later_class` and `depends_on`; `outcome.json` convention of §10.1) |
| Kind | Arm A is FORMAL, with a committed counterexample-search protocol (`decision-method.md` §1.2 FORMAL route): the synthetic-ledger property search below *is* that protocol. Arm B is an EMPIRICAL reliability measurement using agent raters. Arm C is a mechanical audit. No product candidate is selected here. For every §34 decision, this experiment produces only the **v1 disposition** (stable, gated, post-v1, rejected or release-blocking), computed from that decision's own evidence. Its candidate selection belongs to the owning domain's experiments. |
| Final application | Phase E synthesis: `research/decisions/stable-v1-scope.csv` (program §34 deliverable) is the output of the admissible rubric applied to the final ledger |
| Method | `research/decision-method.md` §1.2, §2 (R0-R10), §7.1 C7, §7.4, §8, §9.1, §10 at `14b977c`; `research/archetypal-objective.md` §2.0 rules 4-5, §4.4-§4.6 |

## Author, independence and exposure

Designed by the Phase C-design evaluation-domain session. Arm A uses synthetic ledgers and the real ledger's non-result fields. Arms B and C raters and auditors must be sessions without a recorded L7 exposure and not involved in any decision's candidates (§8.6 gate-audit independence). The FORMAL sign-off (§1.2) is by a reviewer who did not write `v1_scope.py`. The recorded L7 exposure of this designer is in the EXP-EVAL-001 protocol and does not bear on arm A (no corpus data).

## Question

1. **(A, DEC-ECO-068 and the disposition of every program-§34 decision)** Which scoring rubric for the stable-v1 boundary satisfies the pre-registered safety and honesty properties on every ledger state a counterexample search can construct? Which of those is stable under perturbation of its cut-offs, and simplest? What disposition does it assign each capability on the final ledger?
2. **(B, DEC-ECO-079)** Which evidence-classification scheme gives the highest inter-rater agreement, and the fewest lost distinctions among those the method requires, at what tooling cost?
3. **(C, DEC-ECO-080)** Does every committed experiment have a recorded outcome, and does every product claim cite the negative results that bound it? How many negative, null or inconclusive outcomes would each publication policy leave unpublished?

## Disposition classes and inputs (arm A)

**Classes.**
- `STABLE_V1`: frozen, normative, indefinitely decodable.
- `V1_FEATURE_GATED`: shipped behind an experimental `incompat`/`ro_compat` feature bit outside the baseline set; readers without it fail closed; no emission-stability promise.
- `POST_V1`: not in v1, with a recorded add-later path.
- `REJECTED`: all inclusion candidates eliminated at R1 or R2, or a SPEC §24 non-goal.
- `RELEASE_BLOCKING_UNRESOLVED`: v1 cannot ship until resolved.

**Mechanical inputs per decision**, from ledger schema v2 plus gate-audit artifacts:
- `release_relevance`;
- `status`;
- `confidence`;
- `selected_decision`, with its disposition intent (include, gate, defer, exclude);
- computed final cost tier;
- per-HC status (PASS, HC_UNVERIFIED with platform, FAIL);
- `external_review_requirement` state;
- `decision_scope` platforms, with `PLATFORM_BLOCKED` flags;
- cluster technical-gate results:
  - normative spec text present (C5);
  - conformance classes for the capability with an empty UNRESOLVED class;
  - `cleanroom_pass_fraction = 1`;
  - `unresolved_fuzz_findings_at_coverage_target = 0`;
  - external crypto review received (crypto cluster);
- **`add_later_class`**:
  - `ADDABLE_COMPAT`: later addition is invisible to v1 readers and identities;
  - `ADDABLE_INCOMPAT`: later addition needs a new `incompat` bit, but existing v1 archives and identities are unaffected;
  - `NOT_ADDABLE`: deferral would change frozen identity, wire or semantics, so it must be settled before the freeze.
- **`depends_on`**: capability edges.

`add_later_class` and `depends_on` are judged inputs (C7). They are assessed by two independent sessions, and a disagreement resolves to the more restrictive class (§7.2 "a disputed tier resolves to the higher tier").

## Rubric candidates (DEC-ECO-068 candidate set, made executable)

| id | Rubric |
|---|---|
| `R-SQ` | Status quo: no exit criteria; every capability stays experimental (`V1_FEATURE_GATED`), and there is no stable set |
| `R-TG` | Technical gates: `STABLE_V1` iff DECIDED at confidence MEDIUM or higher (HIGH for T3/T4 without an owner acceptance record), every applicable HC PASS in scope, the cluster technical gates pass, and the selection intent is include; otherwise per the precedence rules below |
| `R-TAG` | `R-TG` plus adoption gates as release blockers: named adopter (DEC-ECO-040), published security policy (DEC-ECO-061), licences (DEC-ECO-067), binaries (DEC-ECO-003) |
| `R-TBF` | Time-boxed: at a fixed date, every non-DECIDED capability becomes `V1_FEATURE_GATED`, whatever its `add_later_class` |
| `R-PCL` | Per-capability ledger: disposition equals the decision's own selected intent, with no global property checks |
| `R-MSC` | Minimal stable core: `STABLE_V1` only for unencrypted INDEXED and STREAM, the baseline codecs and strict import, if their gates pass; everything else `V1_FEATURE_GATED` or `POST_V1` |
| `R-SSS` | Subsystem-staged: `R-TG` gates evaluated per subsystem (core format; crypto after external review; legacy adapters; remote access), with independent stability per subsystem |

A critic-added candidate (R0 item 6) is admitted if proposed before arm A runs. It is logged in the record.

**Precedence rules shared by every non-SQ rubric**, applied after its own assignment. A disposition can be moved only toward `RELEASE_BLOCKING_UNRESOLVED`, never away from it.
1. A V1_BLOCKING decision that is not DECIDED becomes `RELEASE_BLOCKING_UNRESOLVED`, unless its DECIDED selection is defer or exclude.
2. A NOT_ADDABLE capability that is not DECIDED becomes `RELEASE_BLOCKING_UNRESOLVED`.
3. EXTERNAL_REVIEW_REQUIRED means not `STABLE_V1`.
4. PLATFORM_BLOCKED inside the claimed scope means not `STABLE_V1` for that scope. The scope is narrowed (objective §4.6), or the decision stays blocking.
5. Dependency closure is enforced by demotion to a fixed point.

## Hypotheses and falsification

- **H-A1 (safety).** `R-TG`, `R-TAG` and `R-SSS` have zero violations of properties P1-P9 on the synthetic-ledger search and on the adversarial scenario set. `R-TBF` and `R-PCL` have at least one violation: P2 for `R-TBF` (a gated NOT_ADDABLE capability) and P3 for `R-PCL` (dependency closure). `R-SQ` violates P9's non-degeneracy clause. Each prediction is falsified by the observed violation counts; any violation eliminates that rubric (R1-analogue for method candidates).
- **H-A2 (stability).** At least one admissible rubric preserves at least 90% of dispositions on the final ledger under every single cut-off perturbation (P11). Falsified if none does. The disposition table is then `band_dependent` and capped at MEDIUM.
- **H-B (classification).** The provenance-dimension scheme (evidence_state plus §9.1 tags and qualifiers) reaches Cohen's kappa ≥ 0.8 between two independent raters and represents all 12 required distinctions. GRADE-style reaches kappa < 0.8 because of its judgement-heavy downgrade reasons. Binary loses at least 6 distinctions. Falsified by the measured kappa and distinction counts.
- **H-C (negative results).** At Phase E, 100% of committed experiment directories carry a valid `outcome.json` (§10.1 item 1). The `status-quo-counterevidence-field` and `decision-relevant-negatives-only` policies would leave at least 1 negative, null or inconclusive outcome unpublished, and the complete registry leaves 0. Falsified by any missing `outcome.json`, which is a §10.1 violation to fix and record, or by the counts.

## Properties (pre-registered; each checked mechanically on every ledger instance)

| id | Property |
|---|---|
| P1 | `STABLE_V1` implies DECIDED, confidence at least MEDIUM (HIGH or owner acceptance for T3/T4), every applicable HC PASS for every platform in scope, and no block of §8.4 |
| P2 | A NOT_ADDABLE capability is never `POST_V1` or `V1_FEATURE_GATED` unless its gating is itself DECIDED and identity-neutral |
| P3 | Dependency closure: `STABLE_V1(x)` and depends(x, y) imply `STABLE_V1(y)`; `V1_FEATURE_GATED(x)` and depends(x, y) imply y is `STABLE_V1` or `V1_FEATURE_GATED`; cycles are reported |
| P4 | A V1_BLOCKING decision that is not DECIDED is `RELEASE_BLOCKING_UNRESOLVED`, unless its DECIDED selection is defer or exclude |
| P5 | A PLATFORM_BLOCKED platform is never inside a `STABLE_V1` scope (objective §2.0 rule 4; DEC-PLT-062) |
| P6 | EXTERNAL_REVIEW_REQUIRED, or a negative or partial external review, is never `STABLE_V1` (§8.3) |
| P7 | Determinism: identical inputs give a byte-identical disposition table |
| P8 | Monotonicity: improving any input (status, confidence, HC PASS, gate pass, review received) never moves a capability to a less stable class, where the order is REJECTED, RELEASE_BLOCKING, POST_V1, GATED, STABLE (improvement may also resolve a block to any class) |
| P9 | Coverage and non-degeneracy: every decision with `release_relevance` V1_* or program section 34 gets exactly one class, and a ledger in which every input is maximally favourable yields at least one `STABLE_V1` |
| P11 | Stability: the fraction of unchanged dispositions under each single perturbation is at least 0.9. The perturbations are: confidence cut-off MEDIUM to HIGH; the tier requiring HIGH moved from T3 to T2; disputed `add_later_class` treated as NOT_ADDABLE; technical gate "empty UNRESOLVED class" relaxed to "no V1_BLOCKING-linked UNRESOLVED case" |
| P12 | Rater reliability of judged inputs: Cohen's kappa ≥ 0.8 for `add_later_class` on a seeded sample of 40 program-§34 decisions, and ≥ 0.8 for `depends_on` edge presence on 60 sampled capability pairs |

## Decisions informed

- `DEC-ECO-068` (rubric and dispositions), `DEC-ECO-079`, `DEC-ECO-080`.
- The **v1-disposition aspect** of every decision whose `program_sections` contain 34, as listed in `research/experiments/_index/evaluation.jsonl` for this experiment. For each one, this experiment answers "is it stable in v1, gated, post-v1, rejected or release-blocking?" from that decision's own evidence. It never selects among its technical candidates.

## Candidates, arm B (DEC-ECO-079)

`SQ` evidence_state plus blocker_class; `PROV` plus a provenance dimension with ceilings (the §9.1 tags and qualifiers as adopted); `GRADE` certainty with downgrade reasons; `SCOPE` scope-qualified claims; `FLAG` decision_grade flag only; `BIN` binary supported or unsupported.

**The 12 required distinctions** (fixed now):
1. measured versus inferred;
2. decision-grade versus informational;
3. SIMULATED;
4. EMULATED;
5. VIRTUALIZED;
6. PLATFORM_BLOCKED;
7. EXTERNAL_REVIEW_REQUIRED;
8. identity-aware held-out versus sealed held-out;
9. pre-freeze versus held-out-validated;
10. UNADJUSTED secondary;
11. contradicted;
12. scope (platform, family, tier).

## Candidates, arm C (DEC-ECO-080)

`status-quo-counterevidence-field`, `dedicated-negative-results-report`, `complete-experiment-outcome-registry`, `decision-relevant-negatives-only`, `external-registered-reports`. The last is evaluated analytically only: publishing to an external registry is an owner action that needs explicit permission.

## Corpus selectors

- Arm A: synthetic ledgers (generator below), a 30-scenario adversarial set written before the search runs, and the real `research/decision-ledger.jsonl` (non-result fields only; pilot at design-freeze time, final at Phase E).
- Arm B: a seeded sample (seed 20260934) of 40 claims drawn from the ledger `counterevidence`, `selected_decision` and `known_limitations` fields plus `research/findings/*.md` at Phase E. If fewer than 40 evidence-bearing claims exist, the shortfall is filled from SPEC §22.2 matrix claims, and the source of each is recorded.
- Arm C: all `research/experiments/*/` directories and claim-bearing docs (`README.md`, published claims tables) at Phase E.

No corpus items; no held-out access.

## Environment

Any host (Python stdlib); rater sessions are agent sessions in fresh contexts.

## Commands (TO BE WRITTEN)

```sh
# arm A
C:/Python313/python.exe research/tools/decide/v1_scope.py gen-synthetic --n 10000 --seed 20260935 --out research/normalized/EXP-EVAL-008/synthetic/
C:/Python313/python.exe research/tools/decide/v1_scope.py check --rubrics R-SQ,R-TG,R-TAG,R-TBF,R-PCL,R-MSC,R-SSS \
    --ledgers research/normalized/EXP-EVAL-008/synthetic/ --scenarios research/experiments/EXP-EVAL-008/adversarial-scenarios.jsonl \
    --out research/normalized/EXP-EVAL-008/property_violations.csv
C:/Python313/python.exe research/tools/decide/v1_scope.py perturb --rubric <admissible> --ledger research/decision-ledger.jsonl --out research/normalized/EXP-EVAL-008/p11_stability.csv
C:/Python313/python.exe research/tools/decide/v1_scope.py apply --rubric <selected> --ledger research/decision-ledger.jsonl --out research/decisions/stable-v1-scope.csv   # Phase E only
# arm B/C
C:/Python313/python.exe research/experiments/EXP-EVAL-008/rater_packets.py --sample 40 --seed 20260934 --schemes SQ,PROV,GRADE,SCOPE,FLAG,BIN --out research/raw/EXP-EVAL-008/rater-packets/
C:/Python313/python.exe research/experiments/EXP-EVAL-008/kappa.py --ratings research/raw/EXP-EVAL-008/ratings/ --out research/normalized/EXP-EVAL-008/kappa.csv
C:/Python313/python.exe research/experiments/EXP-EVAL-008/outcome_audit.py --experiments research/experiments --claims README.md --out research/normalized/EXP-EVAL-008/outcome_audit.csv
```

The adversarial scenario file `research/experiments/EXP-EVAL-008/adversarial-scenarios.jsonl` is written and committed before the search runs. It contains at least one scenario per property, among them:

- a NOT_ADDABLE V1_BLOCKING identity decision that is INSUFFICIENT at a time-box;
- a DECIDED crypto construction whose external review is still pending;
- a capability whose scope includes macOS;
- a stable capability depending on a POST_V1 one;
- a T4 selection at MEDIUM confidence without an owner acceptance record;
- a dependency cycle.

## Seeds

- Synthetic ledgers: 20260935.
- Rater sample: 20260934.
- Rater presentation order: 20260936.

## Warmup

Not applicable.

## Measurement method and metrics

No canonical Appendix A.2 metric applies: these are method candidates, not product candidates. The pre-registered measures are:

- property-violation counts per rubric (P1-P9, integer, AGG-W);
- P11 preservation fractions;
- Cohen's kappa for nominal classes, weighted kappa for ordinal ones (P12 and arm B);
- representable-distinction count out of 12;
- tooling LoC delta (a C2-style counter over the `assemble.py` diff per scheme prototype);
- missing `outcome.json` count;
- unpublished negative-outcome count per policy.

All are descriptive method evidence under §9.1 (`EMPIRICALLY_MEASURED` for counts; the arm B kappa carries a shared-base-model limitation).

## Replication and adaptive rule

- Arm A is exhaustive over the synthetic set. If any admissible rubric shows 0 violations, the search size doubles once (20,000; seed + 1) to confirm. Stated detection bound: 10,000 independent synthetic ledgers detect a per-ledger violation probability of at least 0.0003 at 95% (Appendix D.5).
- Arm B: 2 independent raters per scheme on the same 40 claims, plus a third rater for adjudication only.
- Arm C: a single exhaustive audit, re-run after fixes.

## Normalization

None beyond per-rubric and per-scheme counts.

## Statistical analysis

- **Arm A.** Elimination by any violation of P1-P9. Among admissible rubrics: P11 at 0.9, then simplicity (fewest rules, measured as distinct precedence rules plus gate predicates, the analogue of R10 key 2). The selection is recorded as FORMAL with reviewer sign-off.
- **Arm B.** Kappa with a 95% bootstrap interval (2,000 resamples of claims, seed 20260937), reported with the caution that agent raters share a base model, so kappa is an upper bound on independent-human agreement. A scheme with kappa < 0.8 or fewer than 12 representable distinctions is inadmissible. Among the admissible, the lower tooling cost wins.
- **Arm C.** Counts. Any missing outcome is a §10.1 defect, fixed before the gate audit.

## Practical-significance thresholds

No Appendix A.2 metric is used. The pre-registered criteria are those stated above: zero property violations; P11 at 0.9, matching `thresholds.json` `sensitivity.min_fraction_preserving_winner`; kappa ≥ 0.8. None of them restates a decision band.

## Sensitivity analysis

- P11 perturbations.
- Synthetic generator parameter grid: dependency density {0.02, 0.05, 0.1}; fraction NOT_ADDABLE {0.1, 0.3}; fraction PLATFORM_BLOCKED scopes {0, 0.2}. Violations must be zero in every cell.
- Arm B with and without adjudicated items.

## Expected negative results worth recording

- `R-TBF` (time-boxed) shipping NOT_ADDABLE identity decisions as gated.
- `R-PCL` producing dependency-inconsistent scopes.
- A large `RELEASE_BLOCKING_UNRESOLVED` set on the final ledger: the honest outcome if C-exec evidence is thin.
- The GRADE scheme's low agreement.
- Missing `outcome.json` files found by the audit.

## Threats to validity

- **Synthetic-ledger realism.** Generated ledgers may miss real structure. The adversarial scenarios and the real-ledger pilot mitigate this.
- **Judged inputs.** `add_later_class` and `depends_on` are judgements, hence P12 plus conservative dispute resolution.
- **Agent raters.** Correlated raters share a base model; this is recorded as a limitation. No human-comprehension claim is made (§4.16).
- **Rubric design by the same program.** Taxonomy bias (objective §5 item 1). The round-2 method review and the Phase F gate audit re-examine the selected rubric.

## Platform requirements

Any; no network; no privileges; agent sessions for arm B raters (unexposed, not candidate designers).

## Estimates

- Machine time: under 2 h.
- Disk: under 1 GiB.
- Agent effort: judgment. Arm B uses 3 rater sessions of about 40 short classifications each; the FORMAL sign-off uses one reviewer session; the rest is scripted.
