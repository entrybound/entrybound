# Entrybound Decision Method

| Field | Value |
|---|---|
| Document | `research/decision-method.md` |
| Status | **PRE-REGISTERED, round-1 revision.** This revision dispositions every finding of `research/methods/method-review-round1.md` ([Revision history](#revision-history), Round-1 dispositions). Its commit is the pre-registration record (program §3.3, §37; REQ-ECO-0185). No decision-relevant result exists at that commit (§0.1). Round-2 review re-examines every `REJECTED` and `ACCEPTED_RISK` disposition and samples `FIXED` ones; changes it requires are revisions under §0.2 item 1 for as long as no decision-relevant result exists. |
| Authority | This document is authoritative for the decision procedure and its rule order, every decision threshold, statistical rule, status rule and cost rule, the canonical parameter values (Appendix C) and the canonical metric names and roles (Appendix A.2). `research/methods/thresholds.json` is a mechanical transcription of it (§3.7, Appendix A). Where the two disagree, this document governs and the JSON is corrected. |
| Companion | `research/archetypal-objective.md` defines the hard constraints HC-01 to HC-18, what counts as a violation, and what each optimization-dimension metric measures; on those questions it governs. On procedure and rule order, thresholds, statistics, canonical parameters and canonical metric names and roles, this document governs. |
| Ledger decisions addressed | DEC-ECO-078 (selection rule), DEC-ECO-082 (measurement protocol), DEC-ECO-076 (post-unlock policy default, and the single program-wide freeze of §5.5), DEC-ECO-080 (negative results; minimum obligation), DEC-ECO-083 (usability substitute; evidence cap only). These rows stay `INSUFFICIENT_EVIDENCE` until their own required evidence exists: the full rule simulation (§14 item 9) and A/A, A/A′ and known-effect calibration (§4.7). Until then this document binds the program. |

The key words MUST, MUST NOT, SHOULD and MAY are used as in RFC 2119.

---

## 0. Pre-registration record

### 0.1 State of evidence at this revision (2026-09-16)

- **Repository.** The round-1 revision began at HEAD `8a04436d49ca7ef80fcb917d6150e3ef9237f7ac`. A host storage incident interrupted it, and the unfinished revision was checkpointed in commit `3fda8011b9a23ed4b479a2ad908ef79db510d055` ("research: checkpoint interrupted method revision (WIP)"). That WIP commit is **not** the pre-registration record, although its files already carried the pre-registered labels (Appendix A.1); no decision-relevant result existed at it either. A resumed session completed and verified the revision, and the revision commit descends from `3fda801`. Between `8a04436` and the revision commit, other sessions committed only corpus, PROGRESS and orchestration files. Every input in the table below has the listed SHA-256 at the revision commit, re-verified by hash immediately before it. Other sessions have uncommitted corpus work under `research/corpus/`; it is not an input to this document.
- **Results.** `research/raw/` and `research/normalized/` contain only `EXP-SMOKE-000`, a pipeline smoke test (gzip levels on one generated 1 MiB file, `decision_ids: []`) whose `thresholds_override` values are declared meaningless and were not used. No experiment with non-empty `decision_ids` has run. **No decision-relevant experiment result exists.**
- **Ledger.** 593 rows: 577 `INSUFFICIENT_EVIDENCE`, 16 `EXTERNAL_REVIEW_REQUIRED`, none `DECIDED`. Command: `python -c "import json,collections;print(collections.Counter(json.loads(l)['status'] for l in open('research/decision-ledger.jsonl',encoding='utf-8')))"`.
- **Held-out.** No held-out content, listing, statistic or path under `/root/eb-research/heldout` has been opened by the reviewer or by this revision. **Identity-exposure disclosure** (event L7, §5.7): (i) the round-1 reviewer's `git status` displayed untracked fingerprint file names, one containing a held-out `item_id` (review front matter); (ii) the first revision session's `git status --short` displayed untracked fingerprint file names, four of which are held-out item identifiers in three families; (iii) the resumed revision session read `research/PROGRESS.md` change-log entries, the review's G02 evidence and `git log --stat` output (fingerprint and pin file names) that name the upstream sources of held-out items in families F04, F12, F13, F15, F16 and F17. No session opened held-out content, a listing, a statistic or a path under the held-out data roots. The identifiers are not repeated in any committed file. None of these sessions designs candidates, experiments or analyses (§5.4).
- **Not consulted when setting any threshold:** corpus indicator statistics (`research/corpus/statistics.json`), Research III instrumentation measurements, and published incumbent benchmark figures. The method checks of Appendix D use synthetic data and the published group counts per family only.
- **Inputs read,** with SHA-256:

| Input | SHA-256 |
|---|---|
| SPEC `design/2026-08-29-entrybound-product-architecture.md` (§3.9, §5.4, §5.7, §5.7a, §5.8, §6, §7.6, §9.8, §20.5, §21.1, §22.1, §23.2, §23.4, §25.3) | `1f881c7b13f193b41353b90066d33ca6abcb7d4c3dcd4f58e22834801f0fe29c` |
| `research/methods/method-review-round1.md` (before its disposition section was appended) | `6af01821bff38136d53e5033446e9aa819ff51d791028c3076f30e32f41a83f7` |
| `research/decision-method.md`, round 0 | `ca23c8a2389a5c662425a1be20b4dc736807c7a8e8aefde84ebaaf7cc0caacfa` |
| `research/archetypal-objective.md`, round 0 | `f9aea31ed3dc8a4cd78908006212447229c5b36bb2640b61c15cc00575b1de37` |
| `research/decision-ledger.jsonl` | `98b5401d79c621fab8db2a72e4825dcef9d74dc60de042fdb3990c46daa4068b` |
| `research/methods/thresholds.json` (null template, before transcription) | `56c6cbb8c8a0422afe6d0761d372826ae662df511731d3fe9b89c2ea2141fd31` |
| `research/tools/ebr/stats.py` | `309c92ac511b27bb27515bd76f498b6fb68e72ad81e0fa7d2c5d094009955f25` |
| `research/tools/ebr/thresholds.py` | `c75733409022c0135c8971a9f00c70734e919e72032ce65590f9de0ab986ce31` |
| `research/tools/tests/test_spec.py` (§14 item 1) | `144f3540605d34f13e610bca6628e33ddaa898663e6cac1c57457984e8b7ae06` |
| `research/tools/ebr/guard.py` | `2901c340bb6d7e13af5c75e135276cb2a8bb36165b3f14737c19e05af244bf21` |
| `research/corpus/heldout-lock.json` (status `draft`, 65 items) | `df7331c1d1f2b486986a7f273dd694aaa2c3c6985ad0c952440b80397dc3beb5` |
| `research/corpus/manifest.json` | `8c4e4cc61079ba8c34cafd468f549441d9cb296b53f1957b6346363988b5c494` |
| `CONTRIBUTING.md` | `ab28c1ce8e2ae114603304643167537a90ce32366506a48967caa4efa32f16cc` |

- The pre-registration commit body MUST state that no decision-relevant result exists at that commit. Incumbent-only baseline runs (for example `EXP-BASE-SIZE`, Phase B1) are not decision experiments, but any threshold change made after their outputs exist MUST be declared as made with those outputs visible.

### 0.2 Amendment policy

1. **Before any decision-relevant result exists:** changes are made by revision (review rounds) and recorded in [Revision history](#revision-history) with a disposition per finding.
2. **After a decision-relevant result exists for a metric:** that metric's band, absolute floor, epsilon, role, confidence level, stability bar, minimum-gain multiplier or cap, and guard limit MUST NOT change, except to correct a transcription error against the pre-registered text.
   - Any other amendment, such as fixing a statistical defect, is logged as a deviation. The log records date, author, whether results had been seen, and the reason.
   - Every affected decision is analysed under both the original and the amended method, and both analyses are reported.
   - The original rule governs unless the amendment corrects a demonstrable error. Such an amendment needs all three of: a committed failing test or counterexample; sign-off by an independent reviewer (§10.2); and the §14 item 9 rule simulation passing with the amended rule.
   - If the two analyses disagree on the result, the decision is blocked at `INSUFFICIENT_EVIDENCE` (§8.4) until an analysis on fresh confirmatory data agrees with one of them.
3. **Protocol controls that do not decide outcomes** (repetition counts, warmups, affinity, cooldown, canary cadence, guard limits made tighter) MAY be strengthened after calibration (§4.7). They MUST NOT be weakened once decision-relevant results exist.
4. **Late bands.** A metric without a band and role committed in Appendix A.2 before its first decision-relevant result is secondary for every decision, permanently. Adding a band later cannot make it primary.
5. **Metric definitions** in `archetypal-objective.md` follow the same rule: a definition changed after a metric's first decision-relevant result makes that metric secondary for every decision that had used it.

### 0.3 Tags

Material claims carry evidence tags from §9.1, such as `[FORMALLY_DERIVED]` and `[INFERRED]`. An untagged normative statement is a method rule: a choice this pre-registration makes, not an empirical claim.

### 0.4 Execution gates

Several artifacts this method requires do not exist at this commit. They are gates, not open questions: each is listed in §14 with the latest point at which it must exist.

| Gate | Must hold before |
|---|---|
| G-A | any experiment record with non-empty `decision_ids` is committed |
| G-B | the first decision-relevant result is committed |
| G-C | the design freeze (Commit A, §5.5) |
| G-D | any decision becomes `DECIDED`, and this document is adopted as DEC-ECO-078's resolution |

An unmet gate is a block (§8.4). A result produced while a gate it depends on is unmet is not decision-grade.

---

## 1. Terms and decision types

### 1.1 Terms

| Term | Meaning |
|---|---|
| Candidate | One fully specified design alternative for one ledger decision (`candidate_set[].candidate_id`). A changed design is a new candidate id, for example with a version suffix. |
| Scope | The population a selection applies to. Hierarchy: universal, then profile, policy, workload mix, family (R9). |
| OD set | The optimization dimensions assigned to a decision: ledger field `od_ids` (schema v2, §14 item 12). |
| Applicable HCs | The hard constraints that screen a decision's candidates: ledger field `hc_ids`, derived from the constraint crosswalk (R1). |
| Canonical metric | A metric name from Appendix A.2, with its threshold ID, ebr family, unit, direction, band and **role**. |
| Role | `banded` (may be primary), `diagnostic` (primary only when the decision is about that component), `secondary_only`, `sensitivity_arm` (used only in §6.5), `binary` (§3.3; any failure is R1 or R2), `descriptive` (reported; never eliminates or selects), `cost_input` (feeds §7), `heuristic` (simulated sessions; no CI-based verdict). |
| Primary metric | A `banded` or justified `diagnostic` metric declared in the experiment record before execution (§12), at most one per OD (R3). Only primary metrics can eliminate or select a candidate. |
| Secondary metric | Reported, and able to trigger review. Cannot eliminate or select. |
| Band | The practical-significance band of a metric: a relative half-width `r` (ratio band `[1/(1+r), 1+r]`), an absolute threshold `a` (difference band `[-a, +a]`), or both (§3.1). A **band unit** is `ln(1+r)` in log space, or `a` for absolute-only metrics. |
| Unit of analysis | Levels: sample, item, independence group, family, corpus (§4.9). Scale tiers and evaluation conditions are strata reported at every level. |
| Evaluation condition | A pre-registered condition reported separately and never averaged: network profile, platform pair, user task, legacy format, padding mode (objective AGG-S). |
| Workload mix | A reweighting of families for sensitivity (WM-*, Appendix B.2). Not an evaluation condition. |
| Provisional winner W | The candidate selected on tuning (R3); the anchor of every confirmatory comparison. |
| Confirmatory comparison | W against another tuning survivor, on validation, at corpus level, per primary metric (§4.12). |
| Verdict classes | `DISTINGUISHABLE_BETTER`, `DISTINGUISHABLE_WORSE`, `EQUIVALENT`, `INCONCLUSIVE` (§4.11); plus the property `NON_INFERIOR`. ebr's `indistinguishable` is `EQUIVALENT` or `INCONCLUSIVE`. |
| Decision-grade | A verdict from runs and analyses that meet every requirement of §4 and §14 and every gate they depend on. Anything else is informational. |
| Cost tier | T0 to T4, the permanent-cost class of a candidate, computed under §7.2. |
| Reference candidate | The comparison anchor declared in the experiment record (ebr `analysis.baseline`), normally the status quo. |

### 1.2 Decision types

| Decision type | Evidence route | Held-out validation (R5) | Sensitivity (R7) |
|---|---|---|---|
| EMPIRICAL | Measured experiments under §4 | Required | Required |
| FORMAL | Derivation from invariants, SPEC, standards or exact vectors, plus a committed counterexample-search protocol (generator, budget, result), checked and signed off by a reviewer who did not produce it | Not applicable; the reason is recorded | Not applicable; the reason is recorded |
| EXTERNAL | Dossier plus independent expert review (§8.3) | Per the empirical parts, if any | Per the empirical parts |
| HUMAN-FACING | Mechanically checkable properties under §4.16 (flag counts, unsafe defaults, message censuses), with simulated sessions as heuristic evidence only. Any claim about human comprehension, task success or preference is routed to `EXTERNAL_REVIEW_REQUIRED` or removed from the decision. | Required for the mechanical parts, on a held-out task set | Required |

- **Assignment.** The ledger field `decision_type` (schema v2) is assigned at pre-registration by a session not involved in the decision's candidates. The designer cannot choose it.
- **Forced types.** Any part of a selection rationale that relies on measured size, time, memory, leakage or usability is EMPIRICAL or HUMAN-FACING for that part, whatever type the rest has.
- **Mixed decisions** satisfy the route of every type they contain.
- **Human-participant decisions.** The 34 ledger rows with `blocker_class` HUMAN_PARTICIPANTS are HUMAN-FACING for their human claims: DEC-ACC-004, DEC-ACC-021, DEC-ACC-027, DEC-ACC-039, DEC-ACC-047, DEC-CRY-001, DEC-CRY-061, DEC-CRY-082, DEC-CRY-101, DEC-ECO-001, DEC-ECO-005, DEC-ECO-007, DEC-ECO-010, DEC-ECO-011, DEC-ECO-022, DEC-ECO-023, DEC-ECO-025, DEC-ECO-030, DEC-ECO-033, DEC-ECO-034, DEC-ECO-039, DEC-ECO-048, DEC-ECO-056, DEC-ECO-059, DEC-ECO-065, DEC-ECO-066, DEC-LEG-006, DEC-LEG-013, DEC-LEG-029, DEC-LEG-036, DEC-LEG-088, DEC-LEG-099, DEC-LEG-100, DEC-MOD-028. Command: `python -c "import json;print(sorted(r['decision_id'] for r in map(json.loads,open('research/decision-ledger.jsonl',encoding='utf-8')) if r['blocker_class']=='HUMAN_PARTICIPANTS'))"`.

---

## 2. Decision rule

### 2.0 One procedure

- **Single normative procedure.** R0-R10 below are the only decision procedure. `archetypal-objective.md` §1.2 states the ideal and points here.
- **Precedence is lexicographic.** A later rule never rescues a candidate eliminated by an earlier one. Every elimination records the rule id and the evidence reference; R1 and R2 eliminations also record the violated HC or freeze and its mechanical test.
- **Where permanent cost enters.** In exactly three places: R4 condition 5 (a more expensive candidate cannot eliminate a cheaper one by measurement alone), R6 (minimum gain across tiers) and R10 key 1 (ties). Nowhere else.
- **Capability-combination coverage** (objective §1.3) is a reporting obligation of program synthesis (§10.1 item 5). It never selects.
- **Profile-scope decisions** use the constrained form of SPEC §5.4 ("minimise stored_bytes subject to" the declared bounds): R2 screens the declared profile bounds; the only superiority metric is `artifact_bytes`; the other declared profile metrics act only as non-inferiority guards (R4 conditions 2 and 3) and R10 keys. No weights are used (Appendix B.1). [FORMALLY_DERIVED from SPEC §5.4; review finding A05]
- **Timeline.** The rule order is the logical order; §2.11 maps it onto the program phases.

### R0. Candidate completeness (precondition)

A candidate set is complete only if it contains all of the following:

1. The status quo: behaviour at the research baseline `9e44608`, or the currently frozen decision.
2. The SPEC-proposed design.
3. Every alternative named for the question in the source ledgers (R1-R3, APPX, repo docs, code).
4. The strongest incumbent technique for the capability (SPEC §22), where one exists.
5. "Defer / not in v1", where release relevance permits, with its add-later cost recorded (§7.1 C7).
6. At least one candidate proposed by a critic not involved in designing the others.
7. Where the status quo reaches a defect under objective §2.0 rule 5 (for example a library-version-defined decode step on a default path, R2), a "re-specify or remove from the default path" candidate.

Completeness is checked by a critic pass recorded in the ledger `history`. Adding a candidate after results exist is allowed; the added candidate enters at R1 and the addition is logged. Gate G-A requires the decision's `hc_ids`, `od_ids` and `decision_type` to be assigned before its first experiment record.

### R1. Eliminate hard-constraint and freeze violations

**What screens.** Exactly two classes:

- the hard constraints in the decision's `hc_ids` (objective HC-01 to HC-18, each with its MVTs); and
- the repository freezes F-xx of objective Appendix B.

The ledger's free-text `hard_constraints` strings screen only through the **constraint crosswalk** `research/methods/constraint-crosswalk.csv` (gate G-A). The crosswalk is generated by a checked-in script over every distinct constraint string of every ledger row and has the columns `constraint_string`, `string_sha256`, `class`, `target`, `rationale`, `classifier`, `reviewer`. Each string is classified as one of:

| Class | Meaning | Screens? |
|---|---|---|
| `HC-xx` | The string restates or narrows a hard constraint | Yes, as that HC |
| `F-xx` | The string restates a repository freeze; freezes the crosswalk finds that are missing from objective Appendix B are added there by revision (as F-28 and F-29 were) | Yes, as that freeze |
| `NON_GOAL` | A SPEC §24 non-goal | Only through F-25 |
| `PROGRAM` | A standing program constraint (environments, research isolation) | Only through F-24 and §4; never a per-decision screen |
| `DECISION_INPUT` | A deferral, scope statement or design input (for example "library API surface deferred", "Tier 2 (post-v1)") | Never; it informs candidates and scope |

A session not involved in the classification signs off the crosswalk before it is used. Until it exists and is signed off, HC applicability is undefined and no decision can leave `INSUFFICIENT_EVIDENCE`.

**Test.** A single counterexample suffices; no statistics are involved. A violation is either:

- (a) an applicable MVT failing on any available item of any split, or on the conformance corpus; or
- (b) a formal argument that the design admits a violating archive or state. An R1(b) exclusion, like an objective L0 exclusion, needs a written counterexample (archive bytes, state sequence, or the specification clause that admits the violation) and sign-off by a session not involved in constructing the candidate set, recorded in `candidate_exclusion_reasons` with the reviewer.

**Binary failures are R1, not trade-offs:**

- correctness failures: round-trip mismatch, or unverifiable output presented as verified (I25);
- failure to detect truncation or corruption (I21);
- nondeterminism where determinism is claimed, under the nondeterminism artifact rule (objective §2.0 rule 2; §4.13);
- the three mechanical I24 facts: unkeyed boundaries in encrypted archives, an unrecorded padding mode or an unreported `pad none`, or a public length the declared padding mode does not permit (objective HC-14, MVT-14(e) and (j)). Leakage *magnitude* is never an R1 matter; it is graded under OD-16 (T-17);
- crash-consistency and remote-origin failures (objective HC-18).

**No freeze exception.** Freezes and HCs are screens for every decision, including decisions whose subject is a frozen feature. Behaviour bound to a published identifier is never changed in place: such a candidate is excluded at L0 (objective HC-16). A candidate that supersedes frozen behaviour allocates a new identifier and is costed under §7.2. A proposal to drop *decode* support for a published identifier is not decided under this method; it is escalated as `UNRESOLVED` under objective §4.5.

**Bugs.** A candidate eliminated for an implementation bug that is fixable without changing the design MAY re-enter as a new candidate version. It restarts at R1 on tuning and validation, and the old elimination stays recorded.

### R2. Eliminate unboundable or unspecifiable designs

- **Unboundable.** Some resource needed to decode, verify, extract or randomly access the archive has no bound computable before payload from declared parameters (I17, I19, I20, SPEC §5.4, §6.0a). Resources include memory, codec window, scratch, decoded bytes per accessed byte, lookback depth, request count, and CPU work per input byte.
  - Test: a written bound formula exists, and measured values on every available item, including family F20 adversarial items, never exceed the declared bound. CPU work per input byte is tested by objective MVT-06(d).
  - One exceedance eliminates the (candidate, bound) pair. A revised bound is a new candidate version.
- **Unspecifiable.** Decoder behaviour cannot be stated normatively independent of one implementation (SPEC §5.8), or decoding depends on something other than declared data: floating-point evaluation, thread count, iteration order, environment, or an unrecorded library version (SPEC §5.7, I15, I30).
  - Test: a normative description exists and has been reviewed by a reader not involved in the candidate; every decode-affecting computation is integer or fixed-point; every parameter is recorded as data.
- **Gated library-version-defined decode steps.** A decode step whose output is defined by a pinned library version (for example planner v5 `deflate-reconstruct`, preflate class, and planner v6 `jixel`/`jxl-oxide` JPEG reconstruction) is **not** eliminated by R2 if and only if it meets every condition of objective HC-11: outside the baseline set behind an `incompat` feature; the registry declares that no public specification exists; the version identity is part of the recorded plan's registry identifier; its output is digest-verified before release; and permanent vectors exist. Such a step is tier T4 (§7.2) and is never eligible for the baseline set. A candidate that places such a step on a baseline or default decode path is eliminated. Existing production behaviour that does so is a defect under objective §2.0 rule 5, and R0 item 7 adds the corrective candidate. [review finding A03]
- **Profile bounds.** Once DEC-CMP-035 fixes numeric profile values, a candidate that exceeds a profile's declared bounds is eliminated for that profile scope. Until then, declared requirements are reported but not screened, and no profile-scope decision can become `DECIDED`, because its constrained form (§2.0) is undefined.
- **Licence screen:** §7.5.

### R3. Measure survivors

- **Experiment record first.** The record (§12) MUST be committed before execution. It declares: the assigned decision type; the OD set from `od_ids`; **at most one primary metric per OD** (a second needs a written justification reviewed by a session not involved in the decision); the reference candidate; per-OD base weights (§6.3); the family-mix version (§4.9); scope, applicable families and evaluation conditions; and, for validation looks, the provisional winner W, the tuning survivor set S, the declared superiority metrics for each pair (W, s) and the MDE computation (§4.12).
- **Tuning (exploratory).** Search, sweeps and refinement. R4 is applied on tuning without multiplicity control to form S and W. W is the continuous band-unit utility winner (§6.2), or, for profile-scope decisions, the smallest `artifact_bytes` candidate meeting the bounds and guards.
- **Validation (confirmatory).** Only after the MDE gate of §4.12 passes.
- **Coverage.** An OD in the decision's OD set that goes unmeasured is listed as a gap, and a decision with an unmeasured OD cannot be `DECIDED`. The designer cannot avoid this by omitting a primary metric, because the OD set is assigned, not chosen.

### R4. Eliminate epsilon-dominated candidates

Within a scope, candidate A eliminates candidate B if and only if all of the following hold on validation results:

1. **Superiority.** A is `DISTINGUISHABLE_BETTER` than B on at least one primary metric declared as a superiority metric for the pair, at corpus level (§4.11), at the confidence of §4.12.
2. **Non-inferiority.** A is `NON_INFERIOR` to B on every primary metric at corpus level (§4.11). The mere absence of a `DISTINGUISHABLE_WORSE` verdict is not enough.
3. **Family, tier and condition guard.** In every applicable family, every scale tier and every evaluation condition of the scope, on every primary metric: A is not `DISTINGUISHABLE_WORSE` than B under the family harm test (§4.9), and A's point estimate lies no more than 1 band unit beyond the band on the unfavourable side. For the blast-radius metrics (T-15) the guard also applies to the worst region stratum.
4. **Epsilon.** On every primary metric, A's point estimate is better than B's or within the band of it (tolerance `max(r·|B|, a)`, §3.1). This is implied by condition 2 and is kept for consistency with `stats.pareto_frontier`.
5. **Cost.** A's final cost tier (§7.2) is no higher than B's.

- **Which comparisons are confirmatory.** On validation, only W against each other member of S (§4.12). Comparisons between two non-W survivors are secondary and cannot eliminate.
- **Survivor set.** The candidates not eliminated. Epsilon-dominance is not transitive, and both members of a tie survive.
- **Universal scope.** Conditions 1 and 2 are evaluated at corpus level with the minimum support of §4.9.

### R5. Validate on held-out

Held-out results are obtained once, under the single program-wide freeze (§5.5). **Every candidate that reached the freeze is run, not only W.** Before unlock, the held-out record states the held-out MDE of every supporting comparison (§4.12), computed from validation group-level variance and the held-out group counts per family (fingerprint-level fields, §5.4). A decision with any supporting comparison whose held-out MDE exceeds 2 band units is routed to `INSUFFICIENT_EVIDENCE` before unlock; its held-out results are still produced and published, as report-only.

Validation passes if and only if:

- (a) no R1 or R2 failure occurs on any held-out item;
- (b) W is not eliminated when R4 is applied to held-out results with the held-out confidences of §4.12;
- (c) **superiority replication.** Each validation superiority verdict that supports the selection (W over s on metric m) is classified on held-out:
  - *replicated:* W is `NON_INFERIOR` at one-sided 0.95 (the two-sided 0.90 interval bound on the unfavourable side lies inside the band) **and** the held-out point estimate lies beyond the band edge on the favourable side;
  - *attenuated:* `NON_INFERIOR` at one-sided 0.95, but the point estimate lies inside the band;
  - *failed:* otherwise.

  No verdict may be failed. For every pair (W, s) whose selection rests on superiority, at least one supporting verdict must be replicated: an attenuated verdict can never be the sole support.
- (d) every non-inferiority and `EQUIVALENT` verdict used by R4 condition 2 or by R10 is `NON_INFERIOR` on held-out at one-sided 0.95, and no `EQUIVALENT` verdict used by R10 is `DISTINGUISHABLE` on held-out in the direction unfavourable to W;
- (e) the R4 condition 3 guard holds on held-out;
- (f) the R7 bars hold when re-evaluated on held-out results (§6.6).

Appendix D.2 shows the effect: with a held-out standard error of 0.8 band units (MDE of 2 band units), a validation false positive whose true effect is zero passes the round-0 rule 49.6% of the time and this rule 10.4% of the time; a true effect of 2 band units replicates 89.5% of the time.

On failure the decision does not become `DECIDED`. The held-out results are recorded as counterevidence, and re-selection requires fresh confirmatory data (§5.6). FORMAL and EXTERNAL decision types skip R5 and record why. Every R5 pass whose held-out evidence includes items that are not in a sealed tranche carries the `identity_aware_heldout` soft cap (§5.4, §8.4).

### R6. Account permanent cost

Each survivor receives its final cost record and its **computed** tier (§7.2).

- **Minimum gain.** Let L be the best surviving candidate of a lower tier. A higher-tier candidate H is eliminated unless all of the following hold against L:
  - H is `DISTINGUISHABLE_BETTER` on at least one primary metric against the minimum-gain band of H's tier multiplier k (§3.1: `[(1+r)^-k, (1+r)^k]`, or `min(k·a, cap)` for absolute-only metrics);
  - H is `NON_INFERIOR` on every primary metric;
  - the R4 condition 3 guard holds.
- **Exemption.** The minimum-gain band is not applied, and the lowest-tier constraint-satisfying candidate stands, if and only if every lower-tier candidate was eliminated at R1 or R2, or a normative SPEC or `CONTRIBUTING.md` clause, cited by document and line, requires the capability that only the higher tier provides. A `V1_BLOCKING` release label alone never exempts. [review finding I03]
- **Tier changes.** If a final tier differs from the tier used earlier, R4 is re-run.

### R7. Sensitivity analysis

Perturbations of weights, thresholds, workload mix and sampling are applied under §6. The result determines the broadest scope at which a winner is supported, and the confidence caps that apply.

### R8. Universal default only when supported

A single candidate c* becomes the universal default (the v1 behaviour, the frozen parameter, the default policy) only if it meets all of these:

1. It survives R1-R6 in the universal scope.
2. It is the **measured winner**: the continuous band-unit utility winner (§6.2) at corpus level under the base weights and the cited family mix, and not a member of a multi-member R10 tie set. For profile-scope decisions, it is `DISTINGUISHABLE_BETTER` on `artifact_bytes` than every other survivor, or R10 selects it. If the R10 tie set has more than one member, the record states "no measured winner; R10 selection" and R10 selects.
3. It meets the R4 condition 3 guard against every other survivor.
4. It passes every R7 stability test that applies at the pass bar (§6.6).
5. It passes R5, where R5 applies.

### R9. Otherwise, profile-, policy- or workload-specific winners

If R8 fails, select scoped winners using the broadest partition of the scope in which every part has a candidate meeting the R8 conditions restricted to that part. Partitions are tried in this order:

1. Profile: fast, balanced, dense, extreme.
2. Policy: STREAM or INDEXED layout, encrypted or plain, deterministic or adaptive, local or remote access.
3. Workload mix (Appendix B.2).
4. Family.

Constraints on the partition:

- **(i) Expressibility.** The product must be able to express the partition at the time of the decision. For example, per-family selection exists only where the profile categorizes content, and `fast` performs no deep categorization (SPEC §6.1). Winners at a scope the product cannot express are recorded as limitations and negative results.
- **(ii) Cost.** A partition into scoped winners is itself a cost: at least T1 for a planner or policy split, and higher if it needs wire or user-visible changes. Each scoped winner must beat, within its part, the best single candidate for the whole scope against the minimum-gain band of that tier (§3.1).
- **(iii) Compromise.** If no partition satisfies (i) and (ii), select the surviving candidate that minimizes the maximum weighted continuous band-unit regret across the parts (§6.2). It is accepted only if that maximum is at most 2.0 band units **and** its regret on every single primary metric in every part is at most 1.0 band unit. A compromise selection is soft-capped at MEDIUM, and a compromise at final tier T3 or T4 needs an owner acceptance record (§8.4). Otherwise the decision stays `INSUFFICIENT_EVIDENCE` and the non-dominated frontier is published.

### R10. Prefer the simpler candidate when differences are negligible

- **Tie set.** The candidates whose continuous band-unit deficit from the best value is below 1 band unit (`floor(q) = 0`, §6.2) on every primary metric, and whose confirmatory comparisons are all `EQUIVALENT`, or `INCONCLUSIVE` at the repetition cap.
- **Keys**, in order:
  1. Lower final cost tier.
  2. Fewer normative specification words plus wire items added (§7.1 C1).
  3. Fewer reader lines of code (C2).
  4. Status quo.
  5. Lexicographically smallest `candidate_id`, recorded as a deterministic tie-break.
- **Burden of proof** lies on the more complex candidate. Status quo ranks below cost because pre-v1 *emission* gets no sunk-cost credit; decode support for already-published identifiers is retained by every candidate and is not a differential cost (§7.4).
- **Under `INCONCLUSIVE`.** If any comparison relied on here is `INCONCLUSIVE` rather than `EQUIVALENT`, the selection may become `DECIDED` only after **demonstrated power**: the median item-level interval half-width is at most 0.5 band units on every primary metric, and the corpus-level MDE of every `INCONCLUSIVE` comparison is at most 1 band unit (§4.12). Otherwise the decision stays `INSUFFICIENT_EVIDENCE` with a provisional selection. With power demonstrated, the ledger records "simplicity default under inconclusive evidence", the selection is soft-capped at MEDIUM, and it counts toward CB-1 (§10.2) whichever key decided it.

### 2.11 Timeline mapping

| Program phase | Split | Rules applied | Ledger state afterwards |
|---|---|---|---|
| C-exec, tuning | tuning | R0-R4 exploratory: search, sweeps, refinement; S and W; MDE computation (§4.12) | unchanged |
| C-exec, validation | validation (§5.2 budget and look registry) | R1-R4 confirmatory (W against S), R6, R7 on validation, R8-R10: provisional selection | `INSUFFICIENT_EVIDENCE`, `selected_decision` marked `provisional`, confidence recorded |
| D, freeze | none | Single program-wide design freeze: Commit A and Commit B (§5.5); held-out MDE gate (R5) | frozen provisional selections |
| D, held-out | heldout, once | R1, R2, R4, R5, R7 re-run; **no re-selection** | per §8 |
| E, synthesis | none | Gate audit (§8.6), status rules (§8), negative results and the CC outcome table (§10) | `DECIDED` / `INSUFFICIENT_EVIDENCE` / `EXTERNAL_REVIEW_REQUIRED` |

"Rule followed" means that the ledger `history` entry cites the commit SHA of `decision-method.md` that governed the decision, records the outcome of each of R0-R10, and lists every deviation.

---

## 3. Practical-significance thresholds

### 3.1 Semantics

- **Relative band.** For a paired ratio `R = candidate / reference`, the band is `[1/(1+r), 1+r]`, symmetric in log space, so swapping candidate and reference swaps sides exactly. This matches ebr's epsilon semantics (`log1p(r)`, `stats.eps_dominates`).
- **Absolute threshold.** For a paired difference `D = candidate - reference`, in metric units, the band is `[-a, +a]`.
- **Composite rule.** When a metric has both `r` and `a`, a difference is practically significant only if it exceeds both.
  - `DISTINGUISHABLE` requires the ratio interval and the difference interval both to lie wholly outside their bands, on the same side.
  - `EQUIVALENT` holds if either interval lies wholly inside its band.
  - `NON_INFERIOR` holds if, for either the ratio or the difference, the interval bound on the unfavourable side lies inside the band.
- **Floors and aggregation.** Absolute floors act at item verdict level and on the item point estimates that enter aggregation: an item whose median paired `|D| <= a` contributes log ratio 0 to levels L2-L4. Family- and corpus-level intervals (§4.10) are computed analytically from those group values, so point estimates and intervals use the same data; no bootstrap replicate recomputes item medians at those levels. The selection-stability bootstrap (§6.7) resamples the same group values. [review finding F09]
- **Per-operation latency metrics** (T-08) use only the in-process floor; whole-process samples of them are not decision-grade.
- **Metrics with only `a`** (bits, percentage points, fractions, counts where noted) use the difference band at every level. Differences are aggregated by arithmetic mean.
- **Edge rule.** An interval that touches a band edge overlaps the band (`stats.noise_rule`).
- **Integer counts.** Floors on integer metrics are 0.5, so that a difference of one unit exceeds the band; touching counts as overlap.
- **Coverage fractions** over a pre-registered case set of size n (T-20) have `a = 0.5/n`: a difference of one case exceeds the band.
- **Epsilon equals band.** For R4 condition 4 the tolerance is `max(r·|reference|, a)`.
- **Minimum-gain multipliers (log-consistent).** A tier multiplier k (§7.2) scales the **log** half-width: the minimum-gain band is `[(1+r)^-k, (1+r)^k]`. Absolute floors are not multiplied. For absolute-only metrics the minimum gain is `min(k·a, cap)`, where `cap` is the metric's `minimum_gain_cap` in Appendix A.2, at most 0.25 of the range of a bounded metric. Round 0 used the linear form `1 + k·r`, which meant ×1.10 for size but ×11 for blast radius at T4 and demanded infeasible gains on bounded metrics (review finding E03). Values are tabulated in Appendix D.6.

### 3.2 Thresholds by metric

Canonical metric names are the keys used in experiment specs and in `metric_thresholds` (Appendix A.2, which also gives each metric's role and objective M-numbers). The "Family" column is the ebr metric family (`thresholds.METRIC_FAMILIES`). "Process" floors apply to samples measured around a whole process (ebr `resource` source); "in-process" floors apply to harness-crate timers.

| ID | Metrics (canonical) | Unit | Dir | Family | r | a | Justification |
|---|---|---|---|---|---|---|---|
| T-01 | `artifact_bytes`: every output byte needed for the claimed capability (framing, Index, manifest, padding, signatures, sidecars, receipts; program Task 7); also `total_stored_bytes` (sensitivity arm) and `dedup_stored_fraction` (diagnostic, no floor) | B | min | size | 0.01 | medium and large tiers 64 B; **small tier none** (relative only) | Storage and transfer cost are linear in bytes, and SPEC §6.4 names distribution at a scale where bytes multiply by downloads, so 1% is material. Below 1%, differences are the size of incidental framing, varint or ordering choices that later work can shift `[INFERRED]`. On medium and large items a fixed 64 B difference stays negligible even across 10^6 archives (64 MB total) `[FORMALLY_DERIVED]`. On the small tier (≤ 16 MiB) a 64 B floor can be several percent of a tiny archive, and SPEC §23.4 names tiny-archive overhead ("a few hundred bytes") as a real cost, so the small tier uses the relative band alone and fixed overhead has its own metric, T-21 (review finding E04). The metric is deterministic, so intervals reflect content heterogeneity only. |
| T-02 | `metadata_bytes` and its components `index_bytes`, `dictionary_bytes`, `side_data_bytes` (manifest, Index, entry metadata, integrity records, dictionaries, signatures, receipts; padding excluded, see T-16); `refusal_bytes_read` | B | min | size | 0.05 | 64 B per artifact | These components matter through first-access fetch (T-11), index memory (T-06) and per-entry overhead, each with its own threshold. Where metadata dominates the artifact (many-small-file trees, F04), T-01 already catches the change. The components are diagnostic by default and primary only when the decision is about encoding that component. Components MUST reconcile exactly to `artifact_bytes`. |
| T-02b | `metadata_bytes_per_entry` | B/entry | min | size | 0.05 | 0.5 B/entry | At 10^6 entries, 0.5 B/entry is 0.5 MB, below T-01's 1% band whenever mean file size exceeds 50 B `[FORMALLY_DERIVED]`. |
| T-03 | `encode_wall_s` | s | min | time_wall | fast 0.05; balanced 0.05; dense 0.10; extreme 0.25 | 5 ms process; 0.1 ms in-process. **Small tier:** the in-process timer is primary and process samples are informational. | `fast` exists for throughput and `balanced` is the default (SPEC §6.1-6.2), so they get the smallest timing band this environment is expected to resolve. `dense` treats encode time as secondary (SPEC §6.3). `extreme` treats compression time as nearly free but budget-bounded (SPEC §6.4). The process floor covers multi-millisecond per-invocation jitter from process creation, image load and on-access scanning `[INFERRED; A/A verifies]`, far below the roughly 100 ms at which responses stop feeling immediate `[EXTERNALLY_SOURCED: R. B. Miller, "Response time in man-computer conversational transactions", AFIPS FJCC 1968; Card, Robertson & Mackinlay, "The information visualizer", CHI 1991]`. On small-tier items that floor would hide large relative differences, and batch tools pack many small archives in-process, so the small tier is judged on in-process time (review finding E01). |
| T-04 | `decode_wall_s`, `verify_wall_s`, `stream_unpack_wall_s` | s | min | time_wall | 0.05 | as T-03, including the small-tier rule | Read-many use (SPEC §6.3) runs decode many times per archive, so decode gets the tightest timing band. |
| T-05 | `encode_cpu_s`, `decode_cpu_s` (user+system, whole process tree); `refusal_cpu_s` (in-process only) | s | min | time_cpu | as T-03 / T-04 | as T-03; `refusal_cpu_s` 0.1 ms in-process | CPU time is the proxy for energy and shared-runner cost, with the same materiality as wall time. Parallel effects are covered by T-14. |
| T-05b | `decode_throughput_bps`, `encode_throughput_bps` (logical input bytes per wall second), `verify_throughput_bps` | B/s | max | throughput | as T-04 / T-03 / T-04 | none (time floors are applied first) | Throughput is the reciprocal of time, so the log-symmetric band is identical `[FORMALLY_DERIVED]`. |
| T-06 | `peak_rss_bytes` (Linux `ru_maxrss`), `peak_commit_bytes` (Windows job peak commit), `alloc_peak_bytes` (in-process counting allocator); decode and encode variants; `refusal_alloc_peak_bytes` | B | min | memory_peak | 0.10 | 4 MiB (RSS, commit); 1 MiB (allocator) | Memory is a declared ceiling (`max_decode_memory`, SPEC §6.0a). Crossing a ceiling is an HC-06, R1 or R2 matter judged by the exact allocator oracle with no band (objective MVT-06(a)); this band grades memory within the bound. Differences under 10% rarely change which machines can decode `[INFERRED]`. Allocator arenas and 4 KiB pages make RSS and commit jitter of a few MiB common `[INFERRED; A/A verifies]`; the counting allocator is exact, so its floor is smaller. |
| T-07 | `scratch_peak_bytes` | B | min | scratch_peak | 0.10 | 16 MiB | 16 MiB is below any practical scratch budget on CI runners `[INFERRED]`. ebr samples scratch every `poll_interval_s` (0.05 s), so measured peaks are lower bounds and serve only this graded metric. **Zero-scratch and streaming claims are binary** and are checked by write accounting with tolerance 0 (`scratch_write_bytes`, §4.14), never by polling (review finding E06). |
| T-08 | `first_entry_latency_s`, `random_entry_latency_s`, `encrypted_random_entry_latency_s`, `list_latency_s`, `open_latency_s` (local; to first *verified* byte, with first unverified byte also reported per I25) | s | min | time_wall | 0.10 | 0.1 ms, in-process only | Single operations are dominated by fixed setup costs and vary more, relative to their size, than bulk decode. About 10% is the smallest difference likely to change batch tools doing thousands of lookups `[INFERRED]`. Because those tools run in-process, these metrics are measured **only** with in-process timers, or as batches of at least 1,000 operations inside one process (per-operation time = batch time / N). Whole-process samples of them are not decision-grade: a 5 ms process floor would call a 0.5 ms versus 4 ms difference equivalent (review finding E01). p95 claims need at least 40 sampled operations per item per round (§4.6). |
| T-09 | `access_granularity_bytes` (worst-case decoded bytes needed to read one arbitrary byte; SPEC §6.0a `max_access_granularity`) | B | min | size | 0.25 | 16 KiB | A worst-case bound whose user-visible effect is measured by T-08. Differences under a quarter are dominated by the latency band `[INFERRED]`. The floor equals T-11's so that decoded-granularity and fetched-byte differences are judged at the same scale; round 0 used 64 KiB here and 16 KiB there, and 64 KiB would call 64 KiB versus 128 KiB granularity equivalent although the live chunk-size decisions start at 64 KiB (review findings E02, E05). The metric is deterministic. |
| T-10 | `remote_first_entry_latency_s`, `remote_random_entry_latency_s`, `remote_task_latency_s` under a network profile (§4.15) | s | min | time_wall | 0.10 | max(10 ms, 0.5 × profile RTT) | Remote latency comes in whole round trips; a difference smaller than half an RTT cannot reflect a structural change `[INFERRED]`. The primary value is **modelled** from exact request and byte logs (§4.15); the emulated measurement corroborates it after calibration. |
| T-11 | `http_bytes`, `verified_range_fetch_bytes`, `open_bytes_read` per logical operation | B | min | size | 0.05 | 16 KiB | Bytes are counted exactly. RFC 6928 §2 sets TCP's initial window to `min(10*MSS, max(2*MSS, 14600 bytes))` `[EXTERNALLY_SOURCED]`, so in the §4.15 latency model fetch differences below about 14.6 KB usually add no round trip `[FORMALLY_DERIVED from the model]`. This rationale rests on the model, not on the application-level emulator, which does not reproduce slow start (review finding E05). The 5% relative band reflects metered transfer and CDN cost on batch reads. |
| T-12 | `http_requests`, `range_length_signatures`, `verification_digest_ops` (diagnostic) | count | min | other | 0.10 | 0.5 | Each request that is not multiplexed costs at least one RTT, so one unit is the smallest structural difference. The 10% relative band governs batch operations. |
| T-13 | `verify_overhead_pp` = 100 × (verified wall / unverified wall − 1) | pp | min | other | none | 5.0 pp | Any overhead difference inside the decode-wall band (T-04, 5%) is indistinguishable in total decode time `[FORMALLY_DERIVED from T-04]`. Unverifiable output is governed by I25 and R1, not by this threshold. |
| T-14 | `parallel_speedup` = T(1 thread) / T(n threads) **within one core class** (§4.2), and `parallel_efficiency` (diagnostic) | ratio | max | other | 0.10 | 0.2 | log S = log T1 − log Tn, so to first order the two 5% timing bands add `[FORMALLY_DERIVED]`. The 0.2 floor prevents low-n differences such as 1.10 versus 1.25 from counting. Speedup across P-core and E-core classes has no meaning on this hybrid host and is informational (review finding A07). |
| T-15 | `blast_logical_bytes_p95`, `blast_logical_bytes_max`, `blast_entries_p95`: unreadable logical bytes and entries **per injected event**, with at least 200 seeded single-byte corruptions per region stratum per archive (strata: payload, Index, metadata and manifest, dictionaries, integrity records and footer); headline over strata with equal weights 0.2; worst-stratum guard in R4 and R8 | B; count | min | other | 0.5 (band [0.667, 1.5]) | 64 KiB; 0.5 entry | The live decisions (chunk size, group lookback, dictionary scope) move blast radius between about 64 KiB and a few MiB, and lookback-k alternatives differ by small integer factors (k+1 chunks) `[FORMALLY_DERIVED from I29 lookback semantics]`. A log band of 0.5 separates a factor of 2; the floor equals the smallest chunk size under consideration, so 64 KiB versus 900 KiB is distinguishable. Round 0's band of 1.0 with a 1 MiB floor made that difference equivalent (review finding E02). Equal stratum weights keep Index and manifest damage, rare under uniform byte sampling, from being outvoted. Detection of every injected corruption MUST be 100% (I21, I25; R1). |
| T-16 | `padding_overhead_pp` = 100 × padding bytes / artifact bytes without padding | pp | min | other | none | 1.0 pp, and a per-artifact padding-byte difference above 4 KiB | One percentage point of padding equals T-01's size band `[FORMALLY_DERIVED]`. Below one 4 KiB page or cluster, single-artifact differences vanish into filesystem allocation `[INFERRED]`. Compared within one padding mode; the trade-off against T-17 is resolved at R4 with both as primary metrics. |
| T-17 | Leakage for encrypted archives **per padding mode**, per object over the corpus, for the committed observer model: `leak_min_entropy_bits` (for a deterministic observation channel with a uniform prior, log2 of the number of distinct observations); `leak_shannon_bits` (mutual information between plaintext length or boundary sequence and observations); `anonymity_set_median` (objects sharing an identical observation); `presence_advantage` (maximum over the committed attack suite of TPR − FPR at FPR 0.01, objective M16.2) | bits; bits; count; fraction | min; min; max; min | other | none; none; 1.0; none | 1.0 bit; 1.0 bit; 0.5; 0.05 | One bit of min-entropy leakage doubles or halves the adversary's one-guess vulnerability `[EXTERNALLY_SOURCED: G. Smith, "On the Foundations of Quantitative Information Flow", FoSSaCS 2009]`. The anonymity-set band uses the same factor of two. 5 percentage points is the smallest advantage change a corpus-scale simulation with a few hundred target files can resolve `[INFERRED]`. The framing follows PURBs/Padmé `[EXTERNALLY_SOURCED: Nikitin et al., PoPETs 2019(4)]`. **These metrics are graded, never R1** (I24's hard part is mechanical, objective HC-14). They are not decision-grade until the attack suite and observer model are committed to the repository (§14 item 16), and effectiveness claims go to external review (review finding A02). |
| T-18 | `task_success_rate`, `task_steps`, `task_errors`, `task_completion_time_s` (simulated first-use sessions, §4.16) | fraction; count; count; s | max; min; min; min | other | none | none | **Heuristic only.** Simulated sessions of one base model are correlated, so intervals over them do not describe uncertainty and no CI-based verdict is made (review finding E07). Results are per-task success counts and qualitative failure modes. |
| T-19 | `fs_read_ops`, `fs_write_ops` | count | min | io | 0.10 | 64 | **Secondary only.** Page-cache dependent; diagnostic. |
| T-20 | Coverage fractions over a pre-registered case set: `capture_fidelity_fraction`, `restore_fidelity_fraction`, `cross_platform_restore_fidelity_fraction`, `reason_code_specificity_fraction`, `verified_fraction`, `localization_precision`, `truncation_salvage_fraction`, `benign_false_refusal_fraction` (min), `import_acceptance_fraction`, `import_agreement_fraction`, `export_acceptance_fraction`, `error_actionability_fraction` | fraction | max (except where noted) | other | none | 0.5 / n_cases | These are deterministic counts over a fixed case list, so one case is a real, reproducible difference; the half-case floor makes a one-case difference exceed the band `[FORMALLY_DERIVED]`. Case lists are fixed before results (objective OD-03, OD-19). |
| T-21 | `fixed_overhead_bytes` (small tier, objective M05.3) | B | min | size | none | 16 B | The small-tier companion to T-01: SPEC §23.4 accepts "a few hundred bytes" of framing and index on tiny archives, and 16 B is below one record header `[INFERRED]`. Minimum-gain cap 512 B. |
| T-22 | `access_amplification` per range-size stratum {1 B, 4 KiB, 1 MiB, whole entry} (objective M10.2) | ratio | min | other | 0.25 | 16384 / stratum_bytes | The same materiality as T-09, expressed per stratum: a decoded-byte difference of 16 KiB is the floor in every stratum. Strata are never pooled. |
| T-23 | `declared_tightness_ratio` (declared requirement / measured actual, objective M17.1) | ratio | min (toward 1) | other | 0.10 | none | Values below 1 are HC-06 violations. Above 1, a 10% looser declaration rarely changes a caller's admission decision `[INFERRED]`. |

### 3.3 Binary (constraint) metrics

These metrics have no band. Any failure is an R1 or R2 elimination (roles `binary` in Appendix A.2):

- round-trip byte equality (SPEC §6.5, I14);
- truthful verification status (I25);
- corruption and truncation detection rate of 100% (I21);
- output-hash equality where determinism is claimed (§4.13, I15, SPEC §5.7);
- measured resource use at or below the declared bound, by the exact allocator oracle (I17, I20);
- confinement and policy refusal tests (I16);
- the mechanical I24 facts (objective HC-14);
- zero-scratch or streaming capability, when the candidate claims it, by write accounting with tolerance 0;
- bounded-memory claims (objective M08.3);
- crash consistency and origin safety (objective HC-18).

The ebr family `correctness` carries the degenerate absolute band `[0, 0]`, so any non-zero pass-rate difference is flagged.

### 3.4 Why these values

Every band was chosen to satisfy three requirements:

1. **Above resolvable noise.** It must exceed the noise the environment can resolve with affordable repetitions. A/A calibration (§4.7) verifies this, and a band is never widened to pass.
2. **Below a real choice.** It must be smaller than the smallest difference that would change a user's or implementer's choice.
3. **Consistent across related metrics:** T-04 with T-13, T-05b and T-14; T-01 with T-16 and T-21; T-09 with T-11 and T-22.

Threshold perturbation (§6.4) tests whether conclusions depend on the particular values chosen.

### 3.5 Relative-only and absolute-only choices

- **Relative-only.** Throughput (T-05b) has no floor of its own, because the time floors are applied before the reciprocal is taken. `artifact_bytes` on the small tier is relative-only (T-01).
- **Absolute-only.** Percentage-point, bit and fraction metrics (T-13, T-16, T-17, T-20) and `fixed_overhead_bytes` (T-21) are already normalized or are small absolute quantities, so a relative band would double-normalize.

### 3.6 Scope of ebr's built-in verdicts

ebr v1 applies **one band and one epsilon per metric family** (`thresholds.Thresholds.band(family)`) with a percentile bootstrap at item level. It does not apply absolute floors, profile- or tier-specific bands, the §4.10 interval procedures, group aggregation, non-inferiority or three-way verdicts. Consequently:

- The family values in §3.7 are the bands of each family's reference metric.
- Verdicts, Pareto sets and sensitivity results that ebr's `normalize` computes are **informational**. Decision-grade verdicts come only from the decision-analysis tooling of §14 item 2, which reads `metric_thresholds`, `statistics` and the other extension keys.
- Family `other` is intentionally `null`, because no family-wide band is meaningful for counts, bits, percentage points, fractions and speedups. ebr therefore reports `threshold_unset` for it and never substitutes a default.

### 3.7 Transcription into `thresholds.json` (schema `ebr.thresholds.v1`)

Every key ebr v1 reads is listed below with its value. Each key in the file carries a `source` (or an entry in its block's `sources` map) citing the section here.

| Key | Value | Source |
|---|---|---|
| `families.size.practical_significance_band` | `{kind: ratio, lower: 0.9900990099, upper: 1.01}` | T-01 |
| `families.size.pareto_epsilon` | `{kind: relative, value: 0.01}` | T-01 |
| `families.time_wall.practical_significance_band` | `{kind: ratio, lower: 0.9523809524, upper: 1.05}` | T-04 |
| `families.time_wall.pareto_epsilon` | `{kind: relative, value: 0.05}` | T-04 |
| `families.time_cpu.practical_significance_band` | `{kind: ratio, lower: 0.9523809524, upper: 1.05}` | T-05 (decode) |
| `families.time_cpu.pareto_epsilon` | `{kind: relative, value: 0.05}` | T-05 |
| `families.memory_peak.practical_significance_band` | `{kind: ratio, lower: 0.9090909091, upper: 1.1}` | T-06 |
| `families.memory_peak.pareto_epsilon` | `{kind: relative, value: 0.1}` | T-06 |
| `families.scratch_peak.practical_significance_band` | `{kind: ratio, lower: 0.9090909091, upper: 1.1}` | T-07 |
| `families.scratch_peak.pareto_epsilon` | `{kind: relative, value: 0.1}` | T-07 |
| `families.throughput.practical_significance_band` | `{kind: ratio, lower: 0.9523809524, upper: 1.05}` | T-05b (decode) |
| `families.throughput.pareto_epsilon` | `{kind: relative, value: 0.05}` | T-05b |
| `families.io.practical_significance_band` | `{kind: ratio, lower: 0.9090909091, upper: 1.1}` | T-19 (secondary only) |
| `families.io.pareto_epsilon` | `{kind: relative, value: 0.1}` | T-19 |
| `families.correctness.practical_significance_band` | `{kind: absolute, lower: 0, upper: 0}` | §3.3 |
| `families.correctness.pareto_epsilon` | `{kind: absolute, value: 0}` | §3.3 |
| `families.other.*` | `null` (intentional) | §3.6 |
| `bootstrap.confidence` | `0.99` (base two-sided per-comparison confidence) | §4.12 |
| `bootstrap.resamples` | `20000` | §4.12 |
| `quiet_machine_guard.max_other_cpu_percent` | `3.0` | §4.4 |
| `quiet_machine_guard.max_loadavg_1m` | `9.0` (WSL envelope; decision specs tighten it to 1.0 + threads) | §4.4 |
| `sensitivity.min_fraction_preserving_winner` | `0.90` | §6.6 |
| `sensitivity.dirichlet_samples` | `10000` | §6.3 |
| `sensitivity.dirichlet_concentration` | `4.0` | §6.3 |

- **Extension keys**, read by the decision tooling and ignored by ebr v1 validation (`thresholds.validate` ignores unknown keys): `pre_registration`, `statistics`, `protocol`, `decision_rules`, `cost_tiers`, `metric_thresholds`, and the remaining keys of `bootstrap`, `quiet_machine_guard` and `sensitivity` (including `grid_factors`, the selection-bootstrap and threshold-perturbation settings).
- **Spec lint.** Decision specs MUST NOT set `analysis.sensitivity.dirichlet_samples` or `dirichlet_concentration` (ebr `normalize` lets a spec override them), MUST NOT carry `thresholds_override`, and MAY set `guard` values only tighter than the file's. A checked-in lint enforces this (§14 item 3).

---

## 4. Statistical protocol

### 4.1 Environments and comparability

- **Environments** are identified by `env_id` from `research/environment/`: `windows-host`, `wsl-ubuntu`, `docker-ubuntu-24.04`, plus QEMU arm64 and QEMU user-mode amd64 (always `EMULATED`).
- **Pairing within an environment.** All candidates in one comparison run in the same `env_id`, in the same ebr run (interleaved `order: randomized_blocks`), with the same cache state (§4.5) and the same affinity configuration (§4.2). Numbers from different environments are never paired.
- **Docker.** Docker timing is informational unless calibration (§4.7) passes in that environment.
- **WSL2 filesystem.** WSL2 timing never uses `/mnt/*` (drvfs/9P) paths. Inputs and scratch live on ext4 under `/root/eb-research`.
- **Windows filesystem.** Windows timing uses local NTFS outside the repository (ebr refuses in-repo scratch).
- **QEMU** is used only for correctness and determinism, never for timing.
- **Timing scope.**
  - A decision scoped to Windows uses the Windows host.
  - Linux-scoped timing uses WSL2 under §4.3, carries the `VIRTUALIZED` qualifier (§9.1), and a Linux-native performance claim resting on it is soft-capped at MEDIUM: vCPU placement on P- and E-cores is uncontrolled, ext4 sits on a VHDX over NTFS, and host caches stay warm.
  - A claim covering both environments requires **same-direction decision-grade support in both**: superiority in both for a superiority verdict, non-inferiority in both for a guard. An `INCONCLUSIVE` environment supports nothing (review finding F06).
  - macOS/APFS is `PLATFORM_BLOCKED`: a scope that includes it cannot be `DECIDED` (§8.4); a decision excludes it explicitly instead.

### 4.2 Windows host controls

The host is an Intel i9-14900HX: 8 P-cores with SMT (Windows logical processors 0-15) and 16 E-cores (logical processors 16-31), 64 GB RAM. At capture, Defender real-time protection was on, VBS/Hyper-V was running, and the power scheme was Balanced on AC, all recorded in the `windows-host` fingerprint.

| Control | Rule |
|---|---|
| AC power | Required. AC line status is checked before and after each run. Any battery interval invalidates every sample of the run (reason `on_battery`). |
| Power configuration | The power scheme and power-mode overlay are part of `platform_id`, and a timing campaign uses a single `platform_id`. Agents never change power or security settings. The program owner MAY select a scheme before a campaign and re-capture the environment. |
| Process power throttling | The runner opts measured processes out of power throttling via `SetProcessInformation(ProcessPowerThrottling)`, with the execution-speed control bit set and the state bit cleared `[EXTERNALLY_SOURCED: Microsoft Learn, SetProcessInformation / PROCESS_POWER_THROTTLING_STATE]`. Windows timing is not decision-grade until this is implemented (§14 item 3). |
| Affinity | Named configurations, used as declared per experiment. Candidate thread counts MUST equal the number of logical CPUs in the set. SMT sibling pairs MUST be verified from `GetLogicalProcessorInformationEx` before use; the expected pairing is (0,1), (2,3), …, (14,15). |
| `st-p` | {2}; sibling 3 left idle; CPUs 0-1 avoided because interrupt and DPC load concentrates there `[INFERRED]`. Single-thread reference. |
| `mt-p-n`, n ∈ {2, 4, 7} | First n of {2, 4, 6, 8, 10, 12, 14}: one logical CPU per physical P-core, no SMT siblings. |
| `mt-psmt-14` | {2..15} |
| `st-e`, `mt-e-n`, n ∈ {2, 4, 8, 16} | {18} and {16..16+n−1}. Reported as the low-end core class. |
| `os-scheduled` | Unpinned. End-to-end realism only, informational, never paired with pinned samples. |
| Hybrid confound | Samples from different affinity configurations are never compared, with one exception: the derived speedup metric T-14 within one core class (`st-p` with `mt-p-n`; `st-e` with `mt-e-n`; `mt-psmt-14` with `mt-p-7`), whose noise is calibrated by its own A/A run (§4.7). |
| Cooldown | An idle gap before each sample of min(60 s, max(2 s, previous sample wall)). |
| Drift canary | A fixed CPU-bound workload run **on the experiment's own affinity configuration and thread count**, lasting max(1 s, median sample wall of the previous block), at most 60 s: 3× at session start (median = baseline), then once every 10 blocks or 300 s, whichever comes first. If a canary exceeds **1.025 ×** baseline (0.5 × the T-04 band), pause 120 s and repeat it. If it still exceeds, abort the session and mark every sample since the last good canary invalid (`drift_canary`). |
| Frequency covariate | The runner samples the PDH counter `\Processor Information(_Total)\% Processor Performance` (no admin rights needed) every 1 s during every sample window. A sample whose mean relative performance falls below **0.975 ×** the session-start baseline median is invalid (`frequency_drop`). The environment capture verifies that the counter is readable; if it is not, multi-threaded samples and samples longer than 20 s on Windows are not decision-grade. |
| Throttling onset | At session start, a sustained load probe on the experiment's affinity configuration records t_on, the time until relative performance first falls below 0.975 of its initial value (or "none within 300 s"). A comparison whose candidates' median sample walls lie on different sides of t_on is flagged `power_limit_confound` (soft cap MEDIUM), whatever their ratio. |
| Blocks | Randomized blocks with pairing inside rounds (§4.9). |
| Defender and VBS | Recorded, never changed by agents. Warmup rounds absorb first-open scanning of inputs. Encode timings include scan-on-close of outputs for every candidate and are labelled `defender_on`. MsMpEng and SearchIndexer CPU time are recorded per sample as covariates. **External comparisons** (against REF-INC incumbents), where scanning can depend on the output format and on binary reputation: (a) in-process harness timers that exclude file close are reported beside process timings; (b) where the program owner has excluded a scratch directory from real-time scanning (recorded in the environment capture), a second run in that directory is reported. A claim against an incumbent needs the same direction in the `defender_on` process timing and in (a), and in (b) where it exists. Distinct-binary effects are calibrated by A/A′ (§4.7). |
| Exclusivity | Timing windows are exclusive: no other workflow agents, builds, Docker containers or corpus provisioning. The orchestrator schedules the windows; the guard (§4.4) is a check, not the control. |
| Priority | Normal process priority, the realistic setting. |

### 4.3 WSL2 controls

The VM runs Ubuntu 24.04 with 32 vCPUs on kernel 5.15.167.4, filesystem ext4.

- **Known limits.** vCPUs cannot be pinned to host cores. Host load appears as steal time, and `taskset` pins to vCPUs only (PROGRESS execution-environment caveats). `vm-cold` clears only the VM's page cache.
- **Consistency controls.**
  - Measured processes are pinned to fixed vCPU sets: `st-v` = {2}; `mt-v-n` = {2..n+1} for n ∈ {2, 4, 8}. Other thread counts are informational.
  - The guard inside the VM counts steal time as busy (`guard.py`).
  - A host-side sampler on Windows records, during every WSL sample, host other-process CPU with the §4.4 limits, MsMpEng CPU time, and the disk I/O counters of the `vmmem`/`vmwp` processes that back the VHDX.
  - The Windows host is otherwise idle.
  - Cooldown, the drift canary and the load-average check run as in §4.2 and §4.4, inside WSL.
- **Qualifier.** WSL timing is `VIRTUALIZED` (§4.1). Calibration (§4.7) decides decision-grade status per metric family; where it fails, WSL2 serves only deterministic metrics, correctness, and informational timing.

### 4.4 Quiet-machine guard limits

| Key | Value | Rationale |
|---|---|---|
| `max_other_cpu_percent` (system-wide; ebr v1 key) | **3.0%** of all logical CPUs, over the pre and post idle windows and each sample window | About 1 logical CPU of scattered background. Round 0 allowed 5% (about 1.6 CPUs), enough to occupy the SMT sibling of a pinned P-core or draw from the shared package power budget and shift single-thread timing by more than a 5% band (review finding F05). Candidate-dependent exclusion (kernel I/O threads and Defender scanning a candidate's files are counted as "other") is handled by the invalid-sample balance rule and the covariates below, not by a loose limit `[INFERRED]`. |
| Other-process CPU on the pinned set | ≤ **2.0%** of the set's capacity | Interference that lands on the measured cores matters most. |
| Other-process CPU on each SMT sibling of a pinned CPU | ≤ **2.0%** | Siblings share the physical core; they must be effectively idle. |
| `max_loadavg_1m` (Linux and WSL) | Decision specs set `guard.max_loadavg_1m` = **1.0 + n**, where n is the thread count of the affinity configuration, checked after cooldown and before each sample. `thresholds.json` holds the envelope **9.0** (1.0 + 8, the largest decision-grade WSL configuration). | The 1-minute load average includes the measured tree's own recent threads through exponential decay, bounded by n; more than one additional runnable task means real concurrent work. Round 0's limit of 32 equalled the vCPU count and was no guard. |
| `guard.max_wait_s` in decision specs | ≥ 300 s | Lets transient background work finish. A sample that cannot start after the wait aborts the run (ebr behaviour). |

- **Per-core-class accounting** (pinned set, siblings) requires runner additions (§14 item 3). Until they exist, Windows timing is not decision-grade.
- **Covariates.** MsMpEng and SearchIndexer CPU time per sample, and the §4.2 frequency covariate, are stored with every sample and reported with every timing verdict.
- **Validation of the limits.** During A/A calibration (§4.7) a background-load arm injects a busy loop at the limits: 3.0% of all logical CPUs on E-cores, and 2.0% on a sibling of the pinned set. The A/A criteria must still pass with the load present; otherwise the limits are tightened and calibration is repeated. Limits are never loosened.

**Invalid-sample balance.**

- A (candidate, item) cell with more than 10% invalid samples is not decision-grade.
- If two compared candidates' invalid rates on the same item differ by more than 5 percentage points, that item is excluded from the comparison and the exclusion is reported.
- If more than 20% of items are excluded, the comparison is not decision-grade.

### 4.5 Warmups and cache state

- **Warmup rounds** (ebr `warmups`): 2 for small and medium tier items, 1 for large tier items. They are recorded (`record_warmups: true`) and excluded from analysis.
- **Cache state** is declared per experiment:
  - `hot` (default): inputs are read during warmups;
  - `vm-cold` (WSL only): `sync` then `echo 3 > /proc/sys/vm/drop_caches` before each sample, labelled "VM cache dropped". It clears only the VM's page cache, not the host's, so **no cold-cache claim** is made from it.
  - Cold-cache Windows runs need admin rights and are not used.
- **Warmup adequacy** is checked in calibration. If the median per-item Kendall τ between repetition index and value (`stats.kendall_tau`) lies outside ±0.3, one warmup round is added for that environment and family, and the change is logged as protocol strengthening.

### 4.6 Repetitions

**Deterministic metrics** (sizes, counts, request counts, decoded bytes, leakage computed from deterministic outputs, blast radius under seeded injection, coverage fractions): 2 repetitions, in different rounds and processes, plus the repeat-hash check (§4.13). If the check passes, the values are exact and nothing more is repeated. **Determinism claims** are HC tests with their own counts (objective MVT-13(a): 3 per variation cell plus a 20-repetition stress cell).

**Timing and memory metrics:**

| Tier | Minimum measured rounds | Step | Cap |
|---|---|---|---|
| small (≤ 16 MiB) | 10 | 10 | 30 |
| medium (≤ 512 MiB) | 10 | 10 | 20 |
| large (≤ 8 GiB) | 10 | 5 | 20 |

- **Minimum rounds for the confidence in use.** The exact order-statistic interval (§4.10) exists at confidence c only if 1 − 2·0.5^n ≥ c. The minimum is therefore max(tier minimum, n_min(c)): 8 rounds at 0.99, 9 at 0.995, 10 at 0.998, 11 at 0.999 (Appendix D.4). Round 0's minimum of 5 rounds for the large tier could not reach 0.99.
- **Adaptive rule.** The rule depends on precision, not on significance, which avoids optional-stopping bias `[method choice; cf. Kalibera & Jones, "Rigorous Benchmarking in Reasonable Time", ISMM 2013]`.
  1. After the minimum rounds, compute for each primary comparison on each item the exact order-statistic interval of the median paired log ratio, at the comparison's confidence (§4.12).
  2. Its half-width is the larger of (ln median − ln lower) and (ln upper − ln median). If it exceeds 0.5 × `log1p(r)`, add one step of rounds for that item, covering every candidate.
  3. Stop when the target is met or at the cap. Items still above target at the cap are flagged `imprecise` (soft cap, §8.4), and their verdicts are computed normally.
  4. The rule never looks at the direction of the effect or at where the interval sits relative to the band.
- **Operation sampling.** Per-operation latency metrics (T-08, T-10 measured values) are measured in-process or in batches (§3.2 T-08), with at least 40 operations per item per round whenever p95 is reported or claimed.
- **Simulated usability:** at least 20 sessions per (task, candidate), heuristic only (§4.16).

### 4.7 Calibration (noise floor, distinct binaries, known effects)

Before the first decision-grade timing or memory analysis in an environment, run the following experiments for every timing and memory family a decision will use, on tuning items covering at least 10 families and all three scale tiers, with the full protocol. They are protocol calibration, not decision results, and are repeated whenever `env_id` changes.

| Experiment | Arms | Detects |
|---|---|---|
| `EXP-CAL-AA-<env>` | Two candidates with identical binaries and arguments but distinct names | Residual noise and scheduling bias between identical arms |
| `EXP-CAL-AAP-<env>` (A/A′) | Two functionally identical but byte-different builds (different link order, an added unused padded section, different symbol order), each run with a seeded random environment-block size (0-4096 bytes of padding variables) | Code-layout, link-order and environment-size bias `[EXTERNALLY_SOURCED: Mytkowicz, Diwan, Hauswirth & Sweeney, "Producing wrong data without doing anything obviously wrong!", ASPLOS 2009]`, and Defender reputation or scan differences between distinct binaries |
| `EXP-CAL-KE-<env>` (known effect) | One arm adds a calibrated deterministic CPU busy-loop of +0.5 r, +1 r and +2 r of the measured wall time (r = the family reference band) | Sensitivity: whether effects of band size are resolvable |
| `EXP-CAL-BG-<env>` (background load) | A/A with the §4.4 injected background load | Adequacy of the guard limits |
| `EXP-CAL-SPD-<env>` | A/A of the T-14 speedup ratio per core class | Noise of the one permitted cross-configuration comparison |

**Pass criteria**, per (environment, metric family):

- A/A, A/A′, background load and speedup A/A:
  1. The corpus-level ratio interval (§4.10, 0.99) is `EQUIVALENT`.
  2. **At most 1** item-level verdict is `DISTINGUISHABLE`. Under the verdict rule the pure-noise rate of item-level `DISTINGUISHABLE` verdicts is far below 1%, so round 0's 2% allowance was loose and, over about 78 tuning items, underpowered (review finding E08).
  3. At most 10% of samples are invalid.
  4. The median item-level interval half-width at the cap is at most 0.5 × `log1p(r)`.
- Known effect:
  1. At +2 r, at least 90% of items and the corpus-level verdict are `DISTINGUISHABLE_WORSE`.
  2. At +1 r, the point estimate has the correct sign on at least 80% of items.
  3. At no injected level is any item `DISTINGUISHABLE_BETTER`.

**On failure**, strengthen controls in this order and re-run the failed experiment under a new experiment id after each step: add one warmup round; double the cap; use stricter affinity (`st-p`, or `mt-p-n` without SMT); double the cooldown; tighten the guard limits by half. If all fail, that (environment, family) is not decision-grade. Decisions that need it use another environment or stay `INSUFFICIENT_EVIDENCE`. **Bands are never widened.**

**Outputs used later.** The A/A item-level half-widths at the cap feed the item-level MDE (§4.12) and the threshold-halving condition (§6.4).

### 4.8 Descriptives and raw samples

- **Raw records.** Every sample is stored in raw JSONL with a hash chain, and `ebr verify-raw` MUST pass before `normalize`. Warmups are recorded. Invalid samples are kept with their reasons. Raw runs are never deleted; abandoned runs remain in place, labelled.
- **Descriptives** per (env, candidate, item, metric), via `stats.describe`: n, median, p90, p95, IQR, MAD (unscaled), min, max, mean, geomean. Percentiles use Hyndman-Fan type 7 `[EXTERNALLY_SOURCED: Hyndman & Fan, The American Statistician 50(4), 1996]`.
- **Small n.** p90 with n < 10 and p95 with n < 20 are flagged `low_n` and cannot support tail claims.
- **No outlier removal.** Every valid sample counts, and medians provide the robustness.

### 4.9 Units of analysis and aggregation

| Level | Unit | Statistic |
|---|---|---|
| L0 | sample | raw value |
| L1 | item | Median of the per-round paired log ratios (timing and memory), or the exact value (deterministic, §4.13). Pairs are formed within rounds (ebr `pairing: item_repetition`). Floors apply here (§3.1). |
| L2 | independence group | Arithmetic mean of the item log ratios in the group (a geometric mean of ratios). Items sharing an `independence_group` are not independent. |
| L3 | family | Mean over groups, with equal group weights. |
| L4 | corpus | Weighted mean over applicable families. **Base weights** come from the cited workload mix `research/methods/workload-mix.json` (Appendix B.3; gate G-B), because SPEC §25.3 requires weights "fit to a corpus that reflects real file-type mixes". **Equal family weights** are a sensitivity arm (§6.5). |

- **Absolute-only metrics** use arithmetic means of item-level median differences at L2-L4.
- **Applicability.** A candidate is inapplicable to families its design excludes, for example a JPEG transform outside F12. Applicable families are declared in the experiment record. Transforms are still measured on non-target families, to charge their detection cost (§7.3).
- **Strata.** Every L3 and L4 value is also computed per scale tier and per evaluation condition. Tier and condition values are never averaged into each other for decisions; the R4 and R8 guards apply to each.
- **Minimum support.**
  - A corpus-level verdict needs at least 10 independence groups across at least 5 applicable families in the split used.
  - The family guard needs at least 2 groups; a family with a single group is labelled `single-group`, its guard uses the pooled within-family variance, and it cannot on its own support a scoped winner.
- **Family harm guard** (R4 condition 3, R5 (e)). For each applicable family (and tier and condition), a pooled-variance interval at two-sided 0.90 (§4.10) must not be `DISTINGUISHABLE_WORSE`, and the point estimate must lie no more than 1 band unit beyond the band on the unfavourable side. This replaces round 0's unanimity rule, under which a family-level "worse" verdict was nearly unreachable, so absence of evidence of harm passed as no harm (review finding F08). Full family-level non-inferiority is not required: with 2-4 groups per family it is unreachable at realistic between-group heterogeneity even when the true effect is zero, which would make every decision `INSUFFICIENT_EVIDENCE`. Corpus-level non-inferiority (R4 condition 2) carries the inferential burden; the guard combines a sensitive harm test with a point bound. This is a recorded limitation of every scoped claim.
- **Corpus context at pre-registration.** Independence groups per family: tuning {2: 6, 3: 6, 4: 5, 5: 2, 6: 1}; validation {1: 1, 2: 9, 3: 9, 4: 1}; held-out {1: 1, 2: 4, 3: 10, 4: 4, 6: 1} (count of families with that many groups; command in the review, Appendix R.6, using only the `split`, `family` and `independence_group` fields).

### 4.10 Confidence intervals

- **Item level.** The exact distribution-free order-statistic interval for the median of the per-round paired log ratios, and for the median paired difference of absolute metrics: `(x_(k), x_(n+1−k))` with the largest k such that `1 − 2·BinomCDF(k−1; n, 1/2) ≥ c` `[EXTERNALLY_SOURCED: standard binomial order-statistic interval for a median, e.g. Conover, Practical Nonparametric Statistics, 3rd ed., 1999]`. It assumes independent, identically distributed rounds within an item, which randomized block order supports. With n below n_min(c), the item verdict is `INCONCLUSIVE` (`underpowered_n`). Round 0's percentile bootstrap of a median over 10-30 rounds at Bonferroni confidence approached the sample range and made verdicts depend on extreme rounds (review finding F02). Deterministic metrics use the exact value.
- **Corpus level.** A Welch-Satterthwaite t interval on group-level values `[EXTERNALLY_SOURCED: Satterthwaite, Biometrics Bulletin 2(6), 1946; Welch, Biometrika 34, 1947]`:
  - estimate `E = Σ_f w_f·ȳ_f`, with family weights w_f (L4) and ȳ_f the family mean of group values;
  - variance `V = Σ_f w_f²·v_f`, where `v_f = s_f²/n_f` with s_f² the sample variance of the family's group values when n_f ≥ 2, and `v_f = s_p²/n_f` for single-group families, with s_p² the pooled within-family variance (degrees of freedom Σ(n_f − 1));
  - degrees of freedom `df = (Σ w_f²·v_f)² / Σ (w_f²·v_f)²/df_f`, with df_f = n_f − 1, or the pooled degrees of freedom for single-group families;
  - interval `E ± t_{df,(1+c)/2}·√V`.
- **Family level** (the guard). A pooled-variance t interval: `ȳ_f ± t_{df_p,(1+c)/2}·s_p/√n_f`.
- **Coverage acceptance criterion** (pre-registered; Appendix D.1). An interval procedure is admissible only if its simulated coverage on the actual validation and held-out group structures, under Gaussian, heteroscedastic, heavy-tailed and one-outlier-family generating models, is **no more than 1.0 percentage point below nominal** at 0.90, 0.95, 0.99 and 0.998. Over-coverage is allowed and reported as a power cost. Results with equal family weights, 20,000 simulations per cell:
  - the Welch-Satterthwaite interval's largest shortfall is 0.27 pp (held-out, heteroscedastic, at 0.99: 0.9873); it is admitted;
  - a pooled-variance t interval at corpus level falls 2.65 pp short (validation, heteroscedastic, at 0.95: 0.9235); it is not admitted at corpus level;
  - round 0's family-stratified percentile cluster bootstrap fell 8.15 pp short at 0.99 (review Appendix R.5: 0.9085); it is withdrawn (review finding F01).

  The pooled family-level interval is used only inside the harm guard, where under-coverage makes the guard more sensitive, not less. Any decision whose L4 weights depart from equal weights by more than a factor of 4 between families re-runs the Appendix D.1 check with those weights before its first validation look.
- **Bootstrap.** Used only for ebr's informational item-level intervals, the selection-stability bootstrap (§6.7) and the rule simulation. Resamples: `B = max(20000, ⌈200/(1 − c)⌉)` for intervals, which keeps at least 100 replicates in each tail; 2,000 replicates for §6.7. PCG64 seeded from the spec's `seeds.bootstrap`, type-7 quantiles (`stats.py` conventions).

### 4.11 Noise rule, verdict classes and non-inferiority

Given an interval [L, U] for a ratio or difference at the comparison's confidence and a band [bL, bU]:

| Verdict | Condition |
|---|---|
| `DISTINGUISHABLE` (below or above) | U < bL or L > bU. Touching an edge overlaps (`stats.noise_rule`). Mapped to BETTER or WORSE by the metric's direction. |
| `EQUIVALENT` | bL < L and U < bU (wholly and strictly inside) |
| `INCONCLUSIVE` | anything else |
| property `NON_INFERIOR` | The bound on the unfavourable side lies strictly inside the band: U < bU for a minimized metric, L > bL for a maximized one. |

- **One-sided statements.** "Non-inferior at one-sided 0.95" uses the unfavourable bound of the two-sided 0.90 interval.
- **Composite metrics** (both `r` and `a`): §3.1.
- ebr's `normalize` labels `indistinguishable` without separating the classes. The decision tooling MUST separate them and compute `NON_INFERIOR` (§14 item 2).

### 4.12 Multiple comparisons and power

- **Confirmatory set.** After tuning, the record names W and S (R3). The confirmatory comparisons are W against each s ∈ S∖{W}, on validation, at corpus level, on each primary metric. For each pair the record declares, from tuning results, the **superiority metrics** (those on which W is expected to be better; k_sup ≥ 1). Round 0 enumerated every candidate pair × metric × environment × level, so adding candidates collapsed power toward `INCONCLUSIVE` and R10's cheapest candidate (review finding F08).
- **Superiority.** For a pair with k_sup declared superiority metrics, the per-comparison two-sided confidence is `1 − 0.01/k_sup` (Bonferroni; for an "at least one metric" claim this equals the first step of Holm's procedure `[EXTERNALLY_SOURCED: Holm, Scandinavian Journal of Statistics 6(2), 1979]`).
- **Non-inferiority and equivalence.** Two-sided 0.99 (one-sided 0.005), unadjusted. A selection needs **every** such condition, which is an intersection-union test, valid at the per-condition level without adjustment `[EXTERNALLY_SOURCED: Berger, "Multiparameter hypothesis testing and acceptance sampling", Technometrics 24(4), 1982]`.
- **Across pairs.** A selection needs every pair to hold, which is again an intersection-union test, so no adjustment across pairs. Under these rules a false selection of W over a given s requires a false superiority claim, bounded at one-sided 0.005 per pair.
- **Guards.** The family harm guard uses two-sided 0.90, unadjusted: a more sensitive harm detector is more conservative.
- **Held-out.** Non-inferiority and replication at one-sided 0.95 (R5).
- **MDE gate before a validation look.** For each confirmatory comparison, `MDE = (t_{df,1−α} + t_{df,0.80})·SE` in log units, where α is the comparison's one-sided error, SE is the corpus-level standard error of §4.10 computed with the tuning-split group-level variance of the same comparison and the validation group structure, and df likewise. Every MDE MUST be at most 1 band unit, and the item-level MDE implied by the A/A half-widths at the cap (§4.7) MUST be at most 1 band unit. Otherwise the experiment is redesigned (more groups, items or rounds) before the look, or the decision stays `INSUFFICIENT_EVIDENCE`. The held-out MDE limit is 2 band units (R5).
- **Secondary and diagnostic comparisons** use 0.99 unadjusted, are labelled `unadjusted`, and cannot eliminate or select. Any comparison added after results are visible is secondary.
- **Program level.** There is no adjustment across decisions. Instead: (1) the §14 item 9 rule simulation reports the program-level expected false-selection rate; (2) the phase E synthesis report lists every decision-supporting comparison with Benjamini-Yekutieli adjusted q-values at q = 0.05 `[EXTERNALLY_SOURCED: Benjamini & Yekutieli, Annals of Statistics 29(4), 2001]`, and a decision whose support does not survive that adjustment has this recorded as counterevidence; (3) held-out replication (R5) and the confirmation-bias triggers (§10.2) apply.
- **Resamples:** §4.10.

### 4.13 Deterministic byte metrics: repeat-hash check

- Every experiment that produces artifacts records the output SHA-256 of each sample (ebr metric source `path_sha256`).
- **Acceptance.** All valid samples of an (env, candidate, item) share exactly one output hash, shown as a single value under `string_metrics` in `summary.json`. Byte metrics are then exact: the item-level interval collapses to a point, and the item verdict compares the exact values against band and floor.
- **Mismatch.**
  - If the candidate claims determinism (deterministic mode per SPEC §5.7 and SPEC §12, or I15), the two differing outputs are themselves the violation artifact under objective §2.0 rule 2: both are committed, and the mismatch is an R1 elimination **without** a confirming rerun, unless an infrastructure fault is proven (the stored artifact's re-hash differs from its recorded hash, or the environment capture logs a storage or memory error for the sample). Round 0 required a reproducing rerun, which let intermittent races escape (review finding B05).
  - Otherwise the metric is reclassified as stochastic, with timing-style repetitions (§4.6).
- **Determinism claims** across thread counts, hosts and architectures follow objective MVT-13(a), including its 20-repetition stress cell.
- **Size reconciliation.** The per-section byte accounting of the harness (`ebr-pack`) MUST sum exactly to `artifact_bytes`. A mismatch invalidates the sample.

### 4.14 Memory and scratch measurement

- **Linux:** `ru_maxrss` via GNU time (`max_rss_kib`), a per-process high-water mark. For multi-process pipelines the value is the maximum over processes, not the sum; sampled tree sums are informational.
- **Windows:** job peak commit (`peak_job_commit_bytes`, exact) is primary. Sampled working set is informational and a lower bound.
- **In-process:** a counting global allocator in the harness crates (`alloc_peak_bytes`, exact) is primary for library-level decode memory against `max_decode_memory`, and is the exact oracle of objective MVT-06(a), with allocation-site attribution.
- **No cross-OS comparisons.** The quantities differ, so memory is never compared across operating systems.
- **Scratch peaks** are polled every `poll_interval_s` (default 0.05 s) and are lower bounds; they serve only T-07.
- **Write accounting** decides every binary scratch or streaming claim, with tolerance 0 bytes: on Linux, `write_bytes` deltas from `/proc/<pid>/io` for the whole process tree, with scratch on a dedicated per-sample mount so writes are attributable; on Windows, the job's I/O write transfer counts with scratch on a dedicated volume, or an in-process filesystem shim in the harness (review finding E06).

### 4.15 Remote measurement conditions

`sch_netem` is unavailable, so an application-level proxy emulates the network. Every remote result is labelled `emulated-network`. Profile values are representative orders of magnitude `[INFERRED]`. These four profiles are the only network profiles; objective OD-12 cites them.

| Profile | RTT | Bandwidth |
|---|---|---|
| NP-LAN | 2 ms | 1000 Mbit/s |
| NP-METRO | 20 ms | 200 Mbit/s |
| NP-CONT | 80 ms | 50 Mbit/s |
| NP-INTER | 200 ms | 20 Mbit/s |

- **Exact counts.** The proxy or origin counts requests and bytes exactly.
- **Connection model**, declared per profile in the calibration record and fixed for every experiment that uses the profile: HTTP version, TLS or not, setup round trips, connection reuse, maximum parallel connections.
- **Modelled latency (primary).** Latency is computed from the exact request and byte log: RTT × (setup round trips + request round trips) + transfer time (bytes / bandwidth) + slow-start round trips, using an initial window of `min(10·MSS, max(2·MSS, 14600 B))` with MSS 1460 B, doubling per RTT without loss `[EXTERNALLY_SOURCED: RFC 6928 §2]`. The model is exact given the declared connection model, and it does not depend on the emulator reproducing TCP behaviour, which an application-level proxy does not.
- **Emulator calibration** (`EXP-NETEM-CAL`), with acceptance criteria fixed before any remote result: measured RTT within ±10% of the profile, sustained throughput within ±10% of the profile bandwidth, and request and byte counts exact. A profile that fails calibration has no reportable emulated latency. After calibration, emulated latency corroborates the modelled value and is reported beside it; a claim needs both in the same direction.
- **Loss** is injected only to test failure behaviour (objective HC-18). No performance claim is made under loss.

### 4.16 Usability evaluation protocol (simulated)

Pending DEC-ECO-083:

- **What may decide.** Only mechanically checkable properties: flag and concept counts per task (C6), unsafe defaults (binary), and message-content censuses such as `error_actionability_fraction` (T-20). These are EMPIRICAL measurements of the CLI, not of people.
- **What may not decide.** Any claim about human comprehension, task success or preference. Such a claim either goes to external or human review (`EXTERNAL_REVIEW_REQUIRED`) or is removed from the decision. Simulated evidence is never the sole support of a selection (review finding J07). The 34 affected ledger rows are listed in §1.2.
- **Sessions.** Independent simulated first-use sessions, run by agents not involved in CLI design, which see only the CLI's own help or man text and the task statement. At least 20 sessions per (task, candidate), in randomized order.
- **Pre-committed materials.** Tasks, mechanically checked completion criteria and candidate CLIs are committed before any session. A held-out task set written before design iteration serves R5 for the mechanical parts.
- **Statistics.** None beyond per-task success counts and qualitative failure modes: sessions of one base model are correlated, so no CI-based verdict is made (T-18).
- **Labelling and cap.** Everything is labelled `SIMULATED`, and the maximum evidence state it supports is `SUPPORTED_WITH_LIMITATIONS`.

---

## 5. Split discipline

### 5.1 Roles

| Split | Permitted use | Forbidden |
|---|---|---|
| `tuning` | Search, parameter sweeps, candidate refinement, iterative development | Nothing beyond the corpus rules (`research/corpus/methodology.md` §3) |
| `validation` | Confirming selections made on tuning; noise estimation; regression checks | Search. Choosing among variants by validation results is tuning (§5.2). |
| `heldout` | One final evaluation of the frozen design (R5) | Everything before unlock (§5.4). Any re-selection after unlock (§5.6). |

### 5.2 Validation budget and look registry

- **Registry.** Every validation look is appended to the program-wide registry `research/decisions/validation-looks.jsonl` (gate G-A): decision ids, experiment id, candidates, primary metrics, item ids, commit SHA and UTC time.
- **Budget.** Each decision may consult validation for at most **two** confirmation looks.
- **Second look.** A second look after a documented change is either on **new validation items** from new independence groups, committed and fingerprinted before use, or it is labelled tuning and cannot confirm. Round 0 let the second look reuse the items that the change had just made tuning evidence (review finding G04).
- **Cross-decision looks.** Any candidate created or modified after a validation look of a related decision counts that look against its own decision's budget. Decisions are related when they share a candidate, a primary metric, or the ledger `cluster`. The registry makes this computable.
- **Consequence.** Once a candidate, parameter, metric or analysis changes after a validation look, that validation evidence counts as tuning evidence for the changed design. A third look requires new validation items from new independence groups.

### 5.3 Freezing the held-out set

- **Freeze.** `research/corpus/heldout-lock.json` MUST reach status `frozen` before the first decision-relevant result is committed (gate G-B). Freezing (`assemble.py --freeze`) records `heldout_set_sha256` over the lines `<item_id>\t<logical_tree_sha256>\n`, sorted by `item_id`. At this revision the lock is `draft` with 65 items.
- **Identity of record.** `logical_tree_sha256`; `tree_sha256` is unstable on ext4 (the `st_blocks` and extent issues noted in PROGRESS).
- **Lineage and content overlap** (gate G-C). Independence groups follow upstream lineage: the same project at another version, or a subset of it, is the same group. Before the freeze, a content-overlap audit over fingerprint-level file SHA-256 sets (hashes only, §5.4) computes, per family and per pair of splits, the fraction of logical bytes whose file hashes occur in both splits. The maximum is **0.01**. A violation is resolved by regrouping and relocking, and the L6 check is re-run, before the freeze. Round 0's validator compared group labels, so vendor trees shared across splits were invisible to it (review finding G02).
- **Relocking after the freeze** is allowed only for integrity failures (an item unreadable or corrupt), never for content-based reasons; the reason goes into the lock's `history`. **Replacement criteria:** the replacement comes from a new independence group of the same family and scale tier, has logical size within ±50% of the replaced item and the same licence class, and is drawn by a seeded random draw (seed recorded in the lock history) from a candidate pool listed before the draw, without looking at content statistics.

### 5.4 Threat model and access before unlock

**Adopted threat model: result-blind, identity-aware.** Held-out item identities, sources, generator inputs and fingerprint file names are committed and readable by every agent; most real items are public upstream artifacts, so identity amounts to access; generated held-out items can be regenerated from committed generator inputs; `research/PROGRESS.md` change-log entries name the upstream sources of several held-out items; and at least three sessions have seen held-out identifiers (§0.1). The held-out split therefore protects against result-driven tuning, not against knowledge of which items are held out. Re-sealing every existing held-out item is corpus work outside this document. [review finding G01, option (b), with option (a) as the route that lifts the cap]

**Consequences.**

1. **Cap.** Every R5 pass whose held-out evidence includes items not in a sealed tranche carries the soft cap `identity_aware_heldout` (MEDIUM, §8.4).
2. **No same-upstream tuning.** A candidate's experiment records declare every public data source used to tune it. Before the first validation look, a held-out-aware session checks those sources against the held-out upstream lineages and records only pass or fail, without revealing which upstreams are held out. A failing candidate cannot be `DECIDED` on held-out evidence from the overlapping families.
3. **Role separation.** Corpus-construction sessions are held-out-aware and may not design candidates, experiments or analyses. A session with a recorded identity exposure (event L7) may not design candidates or analyses for decisions whose applicable families include the exposed items. The round-1 reviewer and both round-1 revision sessions (§0.1) design neither.
4. **Transcript audit.** Before the freeze (gate G-C), a checked-in script scans the transcripts of every design and analysis session of the program on this host for held-out item ids and held-out paths. Every hit is an L7 event, and an opened held-out file is an L1 event.

**Sealing route (lifts the cap).** A **sealed tranche** consists of new independence groups whose sources, selection lists and seeds are withheld: generated families are generated after the freeze from an owner-held seed that is committed only as its SHA-256; real items are held in an encrypted bundle under an owner-held key. Sealed items appear to design and analysis agents only through `manifest.public.json`, with opaque item ids and fingerprint file names. A decision whose held-out evidence comes only from sealed tranches carries no `identity_aware_heldout` cap. Each tranche must meet the §4.9 minimum support for the decisions that use it.

**Access rules before unlock.**

- **Forbidden to everyone:** held-out content, listings, statistics and paths, including `/root/eb-research/heldout` and `/root/eb-research/fingerprints/heldout`.
- **Allowed:** fingerprint-only records (hashes, byte and file counts, family, tier, group). The held-out MDE computation (R5) uses group counts only.
- **Materialization** (gate G-C). Held-out content MUST NOT exist on disk in usable form before Commit C: content already materialized under `/root/eb-research/heldout` is removed by the corpus owner before the freeze, then re-provisioned after unlock with every `logical_tree_sha256` verified against the frozen lock; alternatively it is kept encrypted at rest under an owner-held key. Round 0 relied on advisory tool checks while the content sat readable (review finding G07).
- **Tool enforcement:**
  - The ebr runner refuses `split: heldout` unless `EB_HELDOUT_UNLOCK` equals the recorded `design_freeze_sha`.
  - `research/corpus/tools/stats.py` requires `--unlock-heldout` together with a committed `heldout-unlock.json`.
  - `corpuslib.assert_not_heldout` guards corpus paths.
  - `allow_heldout=True` appears only in `provision.py` and `assemble.py --verify-heldout`.

### 5.5 Freeze and unlock sequence

**A single program-wide freeze and unlock.** Every decision to be validated on this held-out set reaches its provisional selection before Commit A. A decision not frozen by then needs fresh confirmatory data collected after the freeze (§5.6). Per-wave tranches of the current held-out set are infeasible: it has 1-6 groups per family, mostly 2-4 (§4.9), and disjoint tranches would fall below the §4.9 minimum support. Sealed tranches (§5.4) may later serve decisions frozen after this freeze. This resolves DEC-ECO-076's choice between one freeze and waves (review finding G03); the ledger note is written at the schema-v2 migration (§14 item 12).

1. **Commit A**, the design freeze, contains:
   - the decision ledger with provisional selections (`selected_decision` marked `provisional`, status `INSUFFICIENT_EVIDENCE`);
   - every held-out experiment spec and record (§12), including held-out MDEs;
   - the decision tooling at the exact version the held-out analysis will use;
   - the harness crates commit and the ebr commit;
   - the `Cargo.lock` SHA-256 and the rustc version;
   - Docker image digests;
   - SHA-256 of the corpus provisioning tools;
   - SHA-256 of `research/methods/thresholds.json`, `research/methods/workload-mix.json`, `research/methods/constraint-crosswalk.csv` and `research/decisions/validation-looks.jsonl`;
   - every candidate's build flags.
2. **Commit B**: `research/decisions/design-freeze.json` containing `design_freeze_sha` (the SHA of commit A), `decision_ledger_sha256`, `heldout_set_sha256` (copied from the frozen lock), `decisions` (the frozen decision ids), `heldout_specs` (paths with SHA-256), every hash listed in Commit A, `approved_by` and `date`.

   **This program uses only the key `design_freeze_sha`.** ebr also accepts `commit_sha` and `sha` for compatibility, and they MUST NOT be used.
3. **Commit C**: `research/corpus/heldout-unlock.json` with `design_freeze_commit` equal to A's SHA, plus approver and reason. A MUST be an ancestor of HEAD.
4. **Provision and run.** Held-out content is provisioned and verified (§5.4). Every held-out experiment runs exactly once, with `EB_HELDOUT_UNLOCK` set to A's SHA. ebr records `unlock_sha` in each held-out raw record.

### 5.6 After unlock

This is the default pending DEC-ECO-076; REQ-ECO-0188 applies.

- **Report-only.** Held-out results are published as obtained.
- **Re-runs** are allowed only for infrastructure failures: invalid samples, crashes, guard aborts. Every attempt is kept and reported. A re-run chosen because of its outcome is leakage (§5.7).
- **Correctness fixes** to candidates after unlock are disclosed with before-and-after held-out results, labelled `post-unlock-fix`. **A post-unlock-fixed candidate cannot become `DECIDED` on that held-out evidence** (block, §8.4); its held-out results are report-only. Round 0 allowed such a candidate to be `DECIDED` at MEDIUM, contradicting L2 (review finding G05).
- **Re-tuning, re-selection or selecting a fixed candidate** requires fresh confirmatory data: items collected after the freeze, from new independence groups, committed and frozen before use. The original held-out results stay published.

### 5.7 Leakage events and consequences

| Event | Detection | Consequence |
|---|---|---|
| L1: any held-out content, listing, statistic or path read before unlock | Tool refusal logs; git history of the lock, freeze and unlock files; transcript audit (§5.4); audits (§5.8) | Every decision with evidence from the leaked families loses R5 eligibility and reverts to `INSUFFICIENT_EVIDENCE`. The leaked items are relocked as `spent` with the reason. Re-validation needs fresh held-out items. |
| L2: held-out results used to modify a design, candidate, parameter or analysis, including a decision not yet frozen | Commit order; the deviation log | As L1 for the affected decisions. The modified design needs fresh confirmatory data (§5.6). |
| L3: search on validation | Validation look registry; experiment records | That validation evidence becomes tuning evidence, and a new validation split of new groups is needed. |
| L4: corpus items added, removed or replaced after results were visible, without a decision record | Manifest history; corpus methodology §11 | The affected experiments' results are invalid for decisions. Rerun on the pre-registered items. |
| L5: undeclared public-benchmark contamination in held-out | `tags: ["public-benchmark"]` audit | Report with and without the contaminated items. The decision is blocked until resolved (§8.4). |
| L6: independence group, upstream lineage or file content shared across splits beyond §5.3 | Corpus validator (methodology §3); content-overlap audit (§5.3) | Items of the offending group are excluded from held-out analyses. If the validator missed the sharing, treat it as L1 for the affected families. |
| L7: a session saw held-out item ids, sources, selection lists or paths without content | Session disclosures; transcript audit (§5.4) | The exposure is recorded in the audit record. The session is excluded from designing candidates and analyses for decisions whose families include the exposed items (§5.4). Exposed items can never lift the `identity_aware_heldout` cap. If content was seen, it is L1. |

### 5.8 Audits

- **Before the freeze:**
  - (a) `heldout-unlock.json` and `design-freeze.json` are absent from git history;
  - (b) `grep -rn allow_heldout research/` shows only the two permitted callers;
  - (c) no raw record has split `heldout`;
  - (d) every decision experiment's raw `meta` git SHA descends from its spec's pre-registration commit, with `dirty = false`;
  - (e) the transcript audit (§5.4) has run and every hit is recorded;
  - (f) the content-overlap audit (§5.3) passes;
  - (g) the validation look registry agrees with the raw records;
  - (h) the L7 record lists every exposure, including the three disclosed in §0.1.
- **After unlock:** every held-out raw record cites `unlock_sha` = A, and every held-out item was verified against the frozen lock when provisioned.

---

## 6. Sensitivity analysis

### 6.1 Inputs

- **Candidates:** those surviving R1-R6 in the scope.
- **Metrics:** primary metrics only.
- **Values:** scope-level aggregates (§4.9), from validation before the freeze and again from held-out after unlock.
- **Tooling:** the checked-in decision tooling (§14 item 2). ebr's own `sensitivity.csv` uses min-max-normalized scores and is informational.

### 6.2 Band-unit utility, ties and regret

For primary metric m of OD o, candidate c, and the best value v_best in the scope:

- **Continuous band units.** Relative metrics: `q_m(c) = |ln(v_c / v_best)| / ln(1 + r_m)`, after floor handling (§3.1). Absolute-only metrics: `q_m(c) = |v_c − v_best| / a_m`. No `floor()` is applied.
- **Utility.** `U(c) = − Σ_o w_o · q_{m(o)}(c)`, with one weight per OD (§6.3) and weights summing to 1. If an OD has two justified primary metrics, its weight is split equally between them.
- **R10 tie screen.** Quantization survives only as the R10 eligibility screen: a candidate is in the tie set when `floor(q_m(c)) = 0` on every primary metric and its confirmatory comparisons are `EQUIVALENT` or `INCONCLUSIVE` at the cap (R10).
- **Measured winner.** If the U-maximizer belongs to a tie set of two or more candidates, the record states **"no measured winner; R10 selection"**, R10 selects, and the weight and threshold perturbation tests are recorded as "not applicable: R10 selection" rather than as passes; the selection-stability bootstrap (§6.7) is still reported. Otherwise the U-maximizer is the measured winner.
- **Explicit selection.** The decision tooling passes the selected candidate to every sensitivity computation explicitly; `stats.weight_sensitivity` chooses its base winner by argmax order, which depends on candidate order, not on R10.
- **Regret** in a part p of a partition: `regret_p(c) = U_p(best_p) − U_p(c)`, and per metric `q_m` within the part, both in continuous band units (R9 iii).

Why: round 0 quantized each metric with `floor()`, so a candidate 0.9 band worse on every metric scored the same as the best, and, with all candidates within a band, weight sensitivity passed at 1.0 while R10 actually made the choice. Appendix D.3 reproduces this: the quantized utilities pick candidate 0 with a weight-preservation fraction of 1.0, and continuous units pick candidate 1 (utilities −0.72, −0.30, −0.44) with grid 0.9981 and Dirichlet 0.9541 (review finding H01). The scores are passed to `stats.weight_sensitivity` without `normalize_scores`; the `sum` aggregate accepts any real-valued scores where higher is better.

### 6.3 Weight perturbation

- **Base weights, per OD.** Weights attach to ODs, not to metrics, so declaring several size metrics cannot multiply the weight on size (review finding D09). Profile-scope decisions use no weights (Appendix B.1). Other decisions declare per-OD base weights in the experiment record; the default is equal weights over the decision's OD set.
- **Grid.** The full Cartesian product of the factors (0.5, 0.75, 1.0, 1.25, 1.5) (`stats.GRID_FACTORS`) over the k weighted ODs, renormalized. Records SHOULD keep k ≤ 8; above 8, use `one_at_a_time` mode against the same bar.
- **Dirichlet.** 10,000 samples with concentration c = 4.0, so alpha = c·k·w_base (`stats.weight_sensitivity`), seeded from the spec. With k = 5 equal weights, each weight's coefficient of variation is sqrt((k−1)/(ck+1)) = sqrt(4/21) ≈ 0.44 `[FORMALLY_DERIVED]`.
- **Stability metric.** FPW, the fraction preserving the winner: the share of weightings under which the selected candidate attains the maximal U within a numerical tolerance of 1e-9. Genuine ties are handled by the R10 screen (§6.2), not by the tolerance.
- **Whole-simplex report (no bar).** SMAA acceptability indices under a flat Dirichlet(1, …, 1) over the whole simplex (10,000 samples), and the winner at every single-OD vertex, are reported for every decision. They show whether the inferred base weights matter, which SPEC §25.3 asks to be fitted; the FPW bars apply only to the base neighbourhood `[EXTERNALLY_SOURCED: Lahdelma, Hokkanen & Salminen, "SMAA – Stochastic multiobjective acceptability analysis", European Journal of Operational Research 106(1), 1998]` (review finding H03).

### 6.4 Threshold perturbation

Re-run R4-R10 with bands and floors scaled:

- **Global:** all metrics ×0.5, then all metrics ×2.
- **Single-metric:** each primary metric's band individually ×0.5 and ×2.
- **Resolvability condition.** A ×0.5 perturbation is run only for a metric whose A/A median item half-width at the cap (§4.7) is at most 0.25 × `log1p(r)` (or 0.25 × a); otherwise it is recorded as "not resolvable at half band" and skipped, because halving a band below A/A resolution only produces `INCONCLUSIVE` verdicts.

**Outcome.** Threshold perturbation never narrows the scope. If the selection changes under either global scaling, or is preserved in fewer than 90% of the single-metric perturbations, the decision records a `band_dependent` flag naming the perturbations that changed it, and confidence is soft-capped at MEDIUM. Round 0 moved to a narrower scope instead, which does not address the dependence on the band values (review finding H04).

### 6.5 Workload-mix reweighting and other arms

Base family weights are the cited workload mix (§4.9, Appendix B.3).

| Test | Procedure | Stability metric and bar |
|---|---|---|
| Equal-weight arm | Recompute L4 and the selection with equal weights over applicable families | Selection preserved; otherwise R9 |
| Leave-one-family-out | Drop each applicable family in turn | Fraction preserving ≥ 0.90 |
| Leave-three-families-out | Every combination of three applicable families if there are at most 1,000 combinations, else a seeded sample of 1,000 | Fraction preserving ≥ 0.90 |
| Family Dirichlet | 10,000 family-weight samples, concentration 4.0 around the base mix, seeded | FPW ≥ 0.90 |
| Workload mixes WM-* | For each mix in Appendix B.2: 0.8 of the weight uniform over its families, 0.2 uniform over the other applicable families | Selection preserved in every mix; otherwise WM-scoped winners are considered at R9, subject to expressibility and cost |
| Leave-one-mix-out | Drop the families of each WM-* mix in turn | Fraction preserving ≥ 0.90 |
| Tier-stratified | The selection computed within each scale tier | Preserved in every tier; a tier where it is not is reported as a limitation of the claim |
| Byte-weighted | Ratio of totals over all items (`total_stored_bytes` for size; total time for timing metrics) | Selection preserved; otherwise reported as counterevidence |
| Environment arm | Windows host versus WSL, where both are in scope | Same-direction support (§4.1) |
| Reference arm | REF-PROD versus REF-INC, for external comparison claims | Reported beside every external claim |

Round 0 used leave-one-family-out alone, which rarely moves a mean over about 20 families (review findings H05, D08).

### 6.6 Stability bars

| Test | Pass | MEDIUM band | Fail |
|---|---|---|---|
| Weight grid FPW | ≥ 0.90 | [0.80, 0.90) | < 0.80 |
| Weight Dirichlet FPW | ≥ 0.90 | [0.80, 0.90) | < 0.80 |
| Selection-stability bootstrap (§6.7) | ≥ 0.90 | [0.80, 0.90) | < 0.80 |
| Leave-one-family-out, leave-three-families-out, leave-one-mix-out | ≥ 0.90 | [0.80, 0.90) | < 0.80 |
| Family Dirichlet FPW | ≥ 0.90 | [0.80, 0.90) | < 0.80 |
| Equal-weight arm, WM-* mixes | all preserved | not applicable: scoped handling at R9 | not applicable |
| Threshold perturbation | no `band_dependent` flag | `band_dependent` flag | none (§6.4) |

- **Failing a test** means the scope does not support a winner; R9 moves to a narrower scope.
- **Held-out re-evaluation** (R5 f) uses the same bars.
- **Not applicable** under "no measured winner; R10 selection" (§6.2): the weight grid, Dirichlet and threshold tests.
- **Rationale for 0.90.** A selection that flips under more than one plausible re-weighting in ten depends on preferences the program cannot justify. The 0.80 lower bar acknowledges that base weights are `[INFERRED]` judgments.
- **Recording.** The ledger field `sensitivity_analysis` cites the generated CSV path, each test's stability value, and the bar outcome.

### 6.7 Selection-stability bootstrap

Point aggregates ignore sampling uncertainty in the selection itself (review finding H02). For each decision and scope:

1. Resample independence groups with replacement within each applicable family, 2,000 replicates, seeded. Before resampling, each family's group values are re-centred on the family mean and their deviations scaled by `sqrt(n_f/(n_f − 1))`, so resampling does not shrink the variance of 2- and 3-group families; single-group families keep their point value.
2. In each replicate, recompute L2-L4, the §4.10 intervals and verdicts, and R4-R10.
3. Report the fraction of replicates that preserve the selection, and hold it to the §6.6 bars.

---

## 7. Permanent cost accounting

A format decision costs something forever. Every conforming reader carries each wire feature for the life of the format (SPEC §4.9, §19.2), and every dependency must be maintained, audited and licensed. This section makes that cost explicit and sets the gains needed to justify it.

### 7.1 Cost elements

| ID | Element | Unit | Measurement |
|---|---|---|---|
| C1 | Specification | Normative words added or changed; new wire record types; new fields, enum values, feature bits, **reason codes** and registry identifiers with decode semantics; new **identity participation assignments** (LAI or AUX membership, identity profiles); new distinct decode behaviours | A checked-in counter over the candidate's normative text diff (table cells count as words) and registry census |
| C2 | Implementation | Non-blank, non-comment lines, counted separately for writer, full reader, minimal reader (SPEC §19.2) and tests; and the **retained decode surface**: reader lines kept reachable to decode already-published identifiers | Pinned counting tool or a checked-in counter over the candidate's research patch. Reader LoC is reported separately because readers are forever. The retained decode surface is a recurring line reported for every candidate; it is differential only between candidates that keep emitting an identifier and candidates that stop (§7.4). |
| C3 | Dependencies | New direct and transitive crates (unique name@version, `cargo tree -e normal --target all`); `unsafe` blocks and functions in new dependency sources; native code (`-sys` crates, `build.rs` compiling C/C++/asm, bytes of native source); SPDX licence expression; maintenance (last release date, number of people with publish rights, open RUSTSEC advisories, MSRV relative to the workspace MSRV), for reader-path dependencies **and writer-path pinned encoders** | Checked-in scripts over `cargo tree` and `cargo metadata` output, the vendored sources, and a pinned advisory database snapshot |
| C4 | Attack surface and assurance | New parser entry points reachable from untrusted archive bytes; new allocation sites sized by untrusted values, each needing a declared bound (I17, I20); new `unsafe` or native code reachable from untrusted input; new fuzz targets; new cryptographic material (keys, nonces, associated data); **recurring external review**: constructions or primitives that need external cryptographic or security review under SPEC §25.2, and how often they must be re-reviewed | Enumerated from the candidate's reader design and cross-checked against the fuzz-target inventory |
| C5 | Independent implementability | Whether a normative specification exists independent of any implementation (SPEC §5.8); whether external normative references are stable, freely available standards; test vectors; independent minimal decoder LoC; **reader fragmentation**: optional or extended features that make archives unreadable by a minimal reader (objective M21.6) | Specification review by a reader not involved in the candidate; independent-reader measurements |
| C6 | User complexity | New CLI flags, modes and concepts; decision points added to first-use tasks | Mechanical task-complexity counts per task (§4.16) |
| C7 | Longevity and churn | Planner-ID churn caused by pinned encoder versions, on reader **and writer** paths (SPEC §5.7); **library-version-defined decode steps** (count, pinned versions, permanent-vector obligations); **permanent golden vectors** and determinism maintenance for every new planner or chunker ID (objective MVT-13(c), MVT-16(d)); migration cost for existing pre-v1 archives (DEC-CON-013); reliance on one upstream implementation; for every "defer" candidate, the **add-later cost** (post-v1 additions become incompatible feature bits; migration steps; identity effects) | Recorded with references, and counted where countable |

### 7.2 Cost tiers

A candidate's tier is the **highest** tier implied by any of its cost elements.

| Tier | Multiplier k | Includes |
|---|---|---|
| T0 | 1 | Implementation-only change: no normative, wire, dependency or user-visible change |
| T1 | 2 | Planner, policy or default change, or a parameter value, within existing normative mechanisms and with no new decoder behaviour; **a new planner or chunker ID** (with its permanent-vector cost line in C7); a new pure-Rust writer-only dependency without `unsafe`; a partition into scoped winners implemented as planner or policy logic (R9) |
| T2 | 3 | New field, enum value, **reason code** or feature bit with decode semantics in an existing record; new codec in the *registered extended* set (SPEC §5.8); new user-visible flag or mode; new profile; new pure-Rust reader-path dependency without `unsafe` |
| T3 | 5 | New codec or reversible transform in the *baseline* set; new record or section type; any dependency containing `unsafe` or native code; a new required user concept |
| T4 | 10 | New layout or container structure, identity or digest root, **identity participation assignment**, or cryptographic construction or primitive; **any library-version-defined decode step** (R2); a **new identifier that supersedes frozen behaviour** of a structure in this row. An in-place change of frozen behaviour has no tier: it is excluded at L0 (objective HC-16; R1). |

- **Multiplier semantics.** k scales the log band (§3.1).
- **Computed, not self-assigned** (review finding I02). C1-C3 are computed by the checked-in scripts of §14 item 7. C4-C7 are assessed, before the decision's first validation look, by a session not involved in the candidates, and recorded in the experiment record. A disputed tier resolves to the higher tier. The final tier is the maximum over elements.
- **Maintenance penalty.** A reader-path dependency or writer-path pinned encoder that fails any of the following raises the candidate's tier by one, up to T4:
  - a release within the last 24 months, or declared feature-complete with a stable normative spec;
  - at least 2 people with publish rights, or ownership by an organization;
  - no unresolved RUSTSEC advisory;
  - MSRV at or below the workspace MSRV.

### 7.3 Minimum gains

The general rule is R6. The following specific rules apply on held-out-replicated results. Band arithmetic is log-consistent (§3.1; values in Appendix D.6).

| Addition | Tier | Minimum gain (the interval must be `DISTINGUISHABLE` against the minimum-gain band) | Additional conditions |
|---|---|---|---|
| New baseline-set codec | T3 (k = 5) | Versus the best existing baseline-set configuration meeting the same profile constraints: `artifact_bytes` ratio interval upper bound < (1.01)^-5 = 0.9514656876; **or** `decode_throughput_bps` ratio interval lower bound > (1.05)^5 = 1.2762815625. The other metric must be `NON_INFERIOR`. Holds in ≥ 2 applicable families with ≥ 2 groups each. | Meets the SPEC §5.8 criteria (R2): permissive licence compatible with redistribution, normative spec independent of any implementation, published test vectors, bounded declarable decode memory, two implementations or a tractable second one |
| New registered-extended codec | T2 (k = 3) | `artifact_bytes` upper bound < (1.01)^-3 = 0.9705901479, or `decode_throughput_bps` lower bound > (1.05)^3 = 1.157625 | Fails closed via `incompat` features (SPEC §5.8) |
| New reversible transform | T3 (k = 5) | Target families: `artifact_bytes` upper bound < 0.9514656876, including verification cost (I31) | Non-target families: `artifact_bytes` `NON_INFERIOR`, and `encode_wall_s` `NON_INFERIOR` against the k = 2 band (1.05)^2 = 1.1025, which charges detection cost |
| New record or section type | T3 (k = 5) | At least the k = 5 band on one primary metric of the decision's scope. Examples: `http_requests` ratio < (1.10)^-5 = 0.6209213231 with a difference beyond 0.5 request; `artifact_bytes` ratio < 0.9514656876 (−4.85%); `first_entry_latency_s` ratio < 0.6209213231 | Exemption only under R6 |
| New field, enum value, reason code or feature bit | T2 (k = 3) | At least the k = 3 band on one primary metric, for example `artifact_bytes` < 0.9705901479 or `random_entry_latency_s` < (1.10)^-3 = 0.7513148009 | Exemption only under R6 |
| New profile | T2 (k = 3) | On the new profile's primary objective (`artifact_bytes` under its constrained form), at least the k = 3 band versus every existing profile under the new profile's own bounds, in ≥ 3 families | A mechanically checked guidance census: every profile's help text states its use case and bounds. Any claim that users choose the right profile is a human claim routed to external review (§4.16). No T1 parameter change to an existing profile achieves at least 50% of the gain in log units. |
| New dependency (addition or replacement) | Per §7.2 | Per the tier of the feature it enables | Licence rules (§7.5) |

- **Absolute-only metrics.** The minimum gain is `min(k·a, cap)` with the caps of Appendix A.2: `verify_overhead_pp` 25 pp, `padding_overhead_pp` 5 pp, `leak_min_entropy_bits` and `leak_shannon_bits` 4 bits, `presence_advantage` 0.25, T-20 fractions 0.25, `fixed_overhead_bytes` 512 B. Each cap is at most a quarter of a bounded metric's range. Round 0 demanded, for example, a gain of 1.5 on a fraction at T4 (review finding E03).
- **Feasibility check** (pre-registered reasoning, `[INFERRED]`). Every tier has a plausible passing candidate: T1 at (1.01)^2 needs a 2% size gain, within the range of planner and level changes; T2 at (1.01)^3 needs 3%; T3 at (1.01)^5 needs 4.9%, within the range reported for dictionary and transform families on their target content; T4 at (1.01)^10 needs 9.5%, or (1.10)^-10 = 0.386 on latency, the scale of layout changes that turn scans into indexed access; for blast radius T4 needs a factor of 57.7, the scale of moving from archive-level to chunk-level localization.

### 7.4 Emission, decode support and sunk cost before v1

- **Decode support persists.** Before and after v1, archives that use an already-published identifier stay decodable with unchanged interpretation (objective HC-16). No candidate removes decode support; a proposal to drop it is escalated as `UNRESOLVED` (objective §4.5), not decided here.
- **Emission gets no sunk-cost credit.** Until the v1 freeze, a writer MAY stop *emitting* a frozen feature. Existing wire features and implemented code receive no credit for being *emitted* in v1: their C1-C5 emission costs are counted as if new.
- **Retained decode surface** (C2) is charged as a recurring cost to every candidate that keeps the feature reachable for new archives. A candidate that only stops writing is not charged extra, because every candidate carries the same decode support for historical archives, so that common cost cancels in comparisons.
- **Migration cost** for existing pre-v1 archives is recorded separately (C7) and considered under DEC-CON-013.
- **After the v1 freeze,** stopping emission of a frozen feature is itself a costed change (at least T1), and removing decode support remains outside this program's authority.

Round 0 priced removal of published features as cheap before v1, which contradicted HC-16 and the repository freezes (review finding A04).

### 7.5 Licence and unsafe-code screens (R1/R2)

- **Workspace crates.** Candidates that add `unsafe` to workspace crates violate the repository freeze `unsafe_code = "forbid"` (F-23) and are eliminated at R1.
- **Copyleft.** A dependency shipped in production crates (a normal, non-dev, non-build dependency of a workspace crate built into `ebound` or its published libraries) under a strong copyleft licence (GPL, LGPL, AGPL) is eliminated at R2, because it fails SPEC §5.8's permissive-licence criterion. Development, test and build-only dependencies, and reference runtimes under `tools/` (F-20), are outside the product and are not screened (review finding I05).
- **Weak copyleft and interpretation.** Weak copyleft (for example MPL-2.0), unusual licences, and licence interpretation beyond SPDX compatibility lists are routed to external review (§8.3).

### 7.6 Ledger recording and the objective crosswalk

For every surviving candidate the ledger fields hold:

- `complexity_cost`: C1, C2, C6;
- `dependency_cost`: C3, C7;
- `security_cost`: C4;
- `wire_cost`: C1 wire items, C5.

Each field states the computed tier and cites the generating command output. Qualitative items carry evidence tags (§9).

**Crosswalk to the objective's cost dimensions** (review finding I04):

| Cost element | Objective OD | Objective metrics (Appendix A.2 role `cost_input`) |
|---|---|---|
| C1 | OD-23 | M23.2, M23.3 |
| C2 | OD-21, OD-24 | M21.2 |
| C3 | OD-22 | M22.1, M22.2, M22.3, M22.4 |
| C4 | OD-24 | M24.1, M24.3, M24.4 |
| C5 | OD-21, OD-22 | M21.6 |
| C6 | OD-25 | M25.2 |
| C7 | OD-22 | (qualitative and counted items above) |

Cost **tiers**, not OD-21 to OD-24 values, drive R4 condition 5, R6 and R10 key 1. OD-21 to OD-24 metrics never enter the utility of §6.2.

---

## 8. Status rules and confidence

### 8.1 Statuses

| Status | Meaning |
|---|---|
| `DECIDED` | Every condition of §8.2 holds and no block of §8.4 applies. |
| `INSUFFICIENT_EVIDENCE` | Any condition of §8.2 fails or any block applies. `selected_decision` MAY hold a selection explicitly marked `provisional`, with its confidence. |
| `EXTERNAL_REVIEW_REQUIRED` | The internal route is complete as far as it can go, and correctness depends on independent expertise (§8.3). |

The ledger's `blocker_class` (EVIDENCE, HUMAN_PARTICIPANTS, EXTERNAL_REVIEW, PLATFORM) records *why* a decision is not decided and is independent of status.

### 8.2 Conditions for `DECIDED`

| # | Condition | Ledger fields |
|---|---|---|
| 1 | Complete candidate set (R0), with the completeness critic recorded | `candidate_set`, `history` |
| 2 | Every excluded candidate justified by rule id and evidence reference; R1 exclusions name the HC or freeze and its mechanical test; L0 and R1(b) exclusions cite the counterexample and the independent reviewer | `candidate_exclusion_reasons` |
| 3 | Linked evidence: every experiment is pre-registered (§12), its raw data passes `verify-raw`, and every number follows §12.3 | `experiment_ids`, `raw_result_refs`, `normalized_result_refs` |
| 4 | Held-out validation passed (R5) for EMPIRICAL and HUMAN-FACING parts | `heldout_result_refs` |
| 5 | Counterevidence is non-empty and cites a committed adversarial-search artifact (`research/experiments/<id>/adversarial-search/` with inputs, procedure and outputs), together with the relevant negative results (§10). "None found" is allowed only with that artifact. | `counterevidence` |
| 6 | Sensitivity analysis done, with the bars of §6.6 met at the selected scope; FORMAL decisions write "not applicable" with a reason | `sensitivity_analysis` |
| 7 | Practical significance: every comparison supporting the selection is a decision-grade confirmatory verdict, or R10 was applied explicitly with its power requirement | `selected_decision`, `history` |
| 8 | Costs recorded with computed tier (§7.2, §7.6) and the minimum-gain rule applied | `complexity_cost`, `dependency_cost`, `security_cost`, `wire_cost` |
| 9 | Rule followed: the history cites the `decision-method.md` commit SHA, the outcome of each of R0-R10, and all deviations | `history` |
| 10 | Scope, known limitations, a measurable reopen trigger (§11), external review requirement ("none", with reason, or a review reference), confidence of MEDIUM or higher, and a remaining-unknowns reference | `decision_scope`, `known_limitations`, `reopen_trigger`, `external_review_requirement`, `confidence`, `remaining_unknowns_ref` |
| 11 | No unresolved confirmation-bias trigger (§10.2) | `history` |
| 12 | No leakage event L1-L6 affecting the evidence (§5.7) | `history` |
| 13 | **Every applicable HC** (`hc_ids`) is PASS on the conformance corpus and on the tuning, validation and held-out splits. HC_UNVERIFIED is allowed only for a platform that `decision_scope` explicitly excludes, never for a property (review finding J01) | `hc_ids` (schema v2) |
| 14 | `decision_type` assigned by an independent session; FORMAL parts carry the committed counterexample-search protocol and reviewer sign-off (§1.2) | `decision_type` (schema v2) |
| 15 | Every OD in `od_ids` measured (R3), with at most one primary metric per OD or a reviewed justification | `od_ids` (schema v2) |
| 16 | Every gate of §0.4 that the evidence depends on was met when the evidence was produced | `history` |
| 17 | A gate audit record exists and passes (§8.6) | `history` |
| 18 | A selection whose final tier is T3 or T4 has confidence HIGH, or an owner acceptance record that names each soft cap | `confidence`, `history` |

A ledger validator enforces conditions 1-3, 5, 6, 8-10, 13-15 and 18 mechanically (§14 items 6 and 12). The remaining conditions are checked by the gate audit.

### 8.3 `EXTERNAL_REVIEW_REQUIRED`

Set this status when a decision's correctness depends on independent expertise the program cannot supply:

- cryptographic constructions and final primitive selection (SPEC §25.2; `docs/crypto-review-v1.md`), including the effectiveness of padding against leakage attacks (T-17);
- security proofs;
- licence interpretation beyond SPDX compatibility lists;
- registrations with external authorities (media type, magic number);
- any claim about human comprehension, task success or preference (§4.16).

**Internal adversarial review by program agents is not independent expert review.**

A dossier MUST exist, containing the question, the candidates, the internal evidence under this method, and specific review questions. The decision becomes `DECIDED` only after the review is received, dispositioned in the ledger, and §8.2 holds. A negative or partial review returns the decision to `INSUFFICIENT_EVIDENCE`.

### 8.4 Blocks, soft caps and confidence

**Blocks.** Each of the following keeps the decision at `INSUFFICIENT_EVIDENCE` whatever the confidence would otherwise be (review finding J03):

- unresolved public-benchmark contamination (L5);
- an amendment whose original and amended analyses disagree on the result (§0.2);
- a `post-unlock-fix` candidate selected on that held-out evidence (§5.6);
- `SIMULATED` evidence as primary support for a human-facing claim (§4.16);
- HC_UNVERIFIED for a property, or for a platform inside the scope; a `PLATFORM_BLOCKED` platform inside the scope (§8.2 item 13);
- R10 under `INCONCLUSIVE` without demonstrated power (R10);
- a held-out MDE above 2 band units for a supporting comparison (R5);
- an unmet gate the evidence depends on (§0.4).

**Soft caps.** Each of the following caps confidence at MEDIUM:

- a stability test in the MEDIUM band (§6.6);
- a `band_dependent` flag (§6.4);
- an attenuated held-out verdict (R5 c), which is never sole support;
- R10 applied under `INCONCLUSIVE` comparisons with power demonstrated;
- a compromise selection (R9 iii);
- a `power_limit_confound` flag (§4.2);
- single-environment timing for a scope covering several platforms;
- `VIRTUALIZED` evidence for a Linux-native performance claim (§4.1);
- `identity_aware_heldout` (§5.4);
- an evidence line marked `imprecise` for a primary comparison (§4.6);
- a confirmation-bias trigger resolved by a disposition that accepted a residual risk.

**Confidence scale.**

| Level | Criteria | Allowed status |
|---|---|---|
| HIGH | All §8.2 conditions hold; no block; no soft cap; every §6.6 test at pass; R5 passed with no attenuated verdict; every family in scope has ≥ 2 held-out groups; at least two independent evidence lines for the central claim, such as a measurement plus a formal bound, or agreement between two environments | `DECIDED` |
| MEDIUM | All §8.2 conditions hold; no block; one or two soft caps | `DECIDED`, except that a selection at final tier T3 or T4 needs an owner acceptance record naming each cap (§8.2 item 18) |
| LOW | Some §8.2 condition fails, a block applies, or three or more distinct soft caps apply | `INSUFFICIENT_EVIDENCE` (provisional selections only) |

Irreversible decisions (wire format, identity, cryptographic construction: tiers T3 and T4) therefore need more confidence than a planner default.

### 8.5 Downgrades

A `DECIDED` decision returns to `INSUFFICIENT_EVIDENCE` when any of these happens:

- a leakage event affects its evidence;
- `verify-raw` fails on its raw data;
- regenerated numbers differ from the reported ones (§12.3);
- replication on new data contradicts it (a `DISTINGUISHABLE` verdict in the opposite direction);
- a reopen trigger fires that invalidates the evidence (§11).

### 8.6 Gate audit

- **Who.** A session not involved in the decision's candidates, experiments or analyses.
- **When.** Before any status change to `DECIDED` or `EXTERNAL_REVIEW_REQUIRED`, and again after any downgrade is resolved.
- **Artifact.** `research/decisions/<decision_id>/gate-audit-<n>.md`, listing every §8.2 condition with its evidence reference and result, every §8.4 block and soft cap with its status, and the confidence level derived from them. The ledger `history` cites the artifact and its commit.
- **Failure.** Any failed condition or applicable block keeps the decision at `INSUFFICIENT_EVIDENCE`.

---

## 9. Evidence classification and citation

### 9.1 Tags

| Tag | Definition | Maximum ledger `evidence_state` it can support alone |
|---|---|---|
| `FORMALLY_DERIVED` | Follows by proof, arithmetic, exact test vectors or definition from stated premises. The premises are cited. | `PROVEN`, only for definitional facts and exact vectors; otherwise as for its premises |
| `EMPIRICALLY_MEASURED` | Produced by this program under §4 and §12 | `EMPIRICALLY_SUPPORTED` when decision-grade and held-out-validated; else `SUPPORTED_WITH_LIMITATIONS` |
| `EXTERNALLY_SOURCED` | Taken from a primary source (§9.3) | `SUPPORTED_WITH_LIMITATIONS`. Where the fact is testable locally it MUST be re-verified by experiment before a decision relies on it (SPEC §22.1: matrix entries are architectural, not measurements). |
| `INFERRED` | Reasoned from other evidence without direct measurement or proof | `INSUFFICIENT` for any empirical claim a decision depends on |
| `EXPERT_REVIEW_REQUIRED` | Correctness requires independent expert review (§8.3) | `EXTERNAL_REVIEW_REQUIRED` |
| `UNRESOLVED` | Open; no adequate evidence yet | `UNTESTED` or `INSUFFICIENT` |

A measured result that is `DISTINGUISHABLE` against a claim sets that claim to `CONTRADICTED`.

**Qualifiers**, appended to a tag, for example `[EMPIRICALLY_MEASURED; SIMULATED]`:

- `SIMULATED`: usability sessions run by agents;
- `EMULATED`: QEMU, or an application-level network;
- `VIRTUALIZED`: timing measured inside the WSL2 VM (§4.1);
- `PLATFORM_BLOCKED`;
- `NOT_DECISION_GRADE`;
- `PRE_FREEZE`: tuning or validation evidence only;
- `HELDOUT`, and `IDENTITY_AWARE_HELDOUT` for held-out evidence from items outside sealed tranches (§5.4);
- `UNADJUSTED`: a secondary comparison.

`SIMULATED` and `EMULATED` cap `evidence_state` at `SUPPORTED_WITH_LIMITATIONS`.

### 9.2 What counts as a decision's evidence

- Evidence cited by a `DECIDED` row MUST be `EMPIRICALLY_MEASURED`, `FORMALLY_DERIVED`, or `EXTERNALLY_SOURCED` from primary sources.
- `INFERRED` material may motivate candidates, experiments and weights, and may appear in `known_limitations`. It cannot carry a decision.

### 9.3 Primary-source citation rule

- **Primary sources:**
  - standards and specifications (IETF RFCs, ISO/IEC, NIST FIPS and SP, IEEE, POSIX / The Open Group, W3C and WHATWG, IANA registries);
  - peer-reviewed papers, or the authors' own versions, with venue and version;
  - official documentation of the tool *at the pinned version*;
  - source code at a pinned commit, cited as `path:line`;
  - release notes;
  - vulnerability records (CVE, RUSTSEC, GHSA);
  - measurements produced by this program.
- **Secondary sources** motivate but never carry a decision:
  - Research I-III and the research appendix, which are internal syntheses; the primary sources beneath them MUST be re-cited;
  - blogs, third-party benchmarks, Wikipedia, forums and vendor marketing;
  - anything written by a language model.
- **Never a source:** an agent's recollection. A claim an agent "knows" MUST be checked by opening the primary source at the cited location.
- **Citation format for evidence in decisions:** source identity (title, number or identifier); version, commit or publication date; locator (section, page or line); access date for web sources; for the program's unpublished inputs, path and SHA-256 as listed in PROGRESS.
- **Quotations** from external sources are limited to short excerpts. Paraphrase otherwise.
- **Citations in this document** that justify *method choices* (§3, §4, §6) identify the work. Locators are required for evidence cited by decisions.

---
## 10. Negative results and confirmation bias

### 10.1 Negative-results obligation

This is the minimum obligation, pending DEC-ECO-080; REQ-ECO-0189 applies.

1. **Every committed experiment spec gets an outcome record.** `research/experiments/<id>/outcome.json` classifies the outcome as `positive`, `negative`, `null`, `inconclusive` or `abandoned`, with the reason, and links the raw and normalized outputs. Abandoned experiments keep their raw data. No experiment is silently dropped.
2. **Negative, null and inconclusive outcomes** carry the same provenance as positive ones and appear in:
   - the `counterevidence` of every decision they constrain;
   - a register under `research/reports/` listing every such outcome;
   - the final evaluation's section on workloads where Entrybound is not best (program Task 35), including capability-matched incumbent comparisons (DEC-ECO-031).

   Examples: candidates that failed to beat the status quo, frozen parameters shown suboptimal, contradicted claims, workloads where incumbents win.
3. **Product claims** (README, website, SPEC §23-style claims) MUST cite the negative results that bound them.
4. **Families where the selected candidate loses** at R9 or R5 are reported by name, with magnitudes produced per §12.3.
5. **Capability-combination outcomes.** Program synthesis reports the objective §1.3 outcome (`SERVED`, `SERVED_WITH_DECLARED_COST`, `NOT_SERVED`) for CC-01 to CC-10, with measured gaps, separately for the set of decided configurations and for the default configuration. This is a report, not a selection rule (§2.0).

### 10.2 Confirmation-bias triggers

| ID | Trigger | Action |
|---|---|---|
| CB-1 | Simplicity drift: in a cluster with ≥ 6 decided or provisional decisions, **R10 selections made under `INCONCLUSIVE` comparisons, whichever key decided them,** account for ≥ 50% of them | Power review: for each affected decision, report interval half-widths and MDEs against band half-widths. Where the power requirement of R10 is not met, the decision cannot become `DECIDED` until rounds, items or groups increase. |
| CB-2 | The SPEC-proposed or status-quo candidate wins ≥ 90% of ≥ 10 decisions in a cluster | Independent re-analysis of 3 of those decisions, drawn with a recorded seed |
| CB-3 | Candidates, metrics, items, primary comparisons, weights or analysis changed after results were visible, outside the §0.2 or §5.2 procedures | The affected analysis is re-run as originally registered, and both analyses are reported |
| CB-4 | Marginal win: the interval bound nearest the band edge lies within 0.5 band units of the edge, on a comparison that supports the selection | Independent re-analysis. Confidence at most MEDIUM until dispositioned. |
| CB-5 | Exclusion sensitivity: a verdict changes when invalid samples or excluded items are included | Report both. Independent re-analysis. |
| CB-6 | Replication failure: validation reverses tuning, or held-out is attenuated or reversed | Recorded as counterevidence. For held-out, per R5. |
| CB-7 | Self-review: the only analysis supporting the winner came from the agent or session that proposed the winning candidate | Independent re-analysis |
| CB-8 | `counterevidence` lacks a committed adversarial-search artifact for a decision with ≥ 3 candidates | The decision cannot be `DECIDED` until the search is run and committed (§8.2 item 5). |

**Independent re-analysis** is performed by a reviewer that was not involved: a separate agent session with fresh context, not the designer of the candidates or experiment. The reviewer re-derives verdicts from raw artifacts using the checked-in commands and writes a disposition to `research/experiments/<id>/review-<n>.md`. Until the disposition exists, the decision cannot move to `DECIDED` and confidence is at most MEDIUM.

---

## 11. Reopen rules

Every `DECIDED` decision records at least one **measurable** reopen trigger specific to that decision in `reopen_trigger`, for example "a codec release changes `artifact_bytes` on F01 by more than the band". The following triggers apply to every decision:

1. New decision-grade evidence is `DISTINGUISHABLE` against the selection in any family within its scope.
2. A dependency gains a RUSTSEC advisory, changes licence, or loses maintenance (§7.2 penalty).
3. A blocked platform becomes available and is within the decision's scope.
4. An upstream tool or codec version change moves a primary metric of the decision by more than its band.
5. A method amendment changes a rule the decision used (§0.2).
6. A leakage event or raw-verification failure affects the decision's evidence (§8.5).
7. An external review finding bears on the decision.

**On a trigger:**

- If the trigger invalidates evidence (items 1, 5 when the result changes, 6, and 7 when negative), the status becomes `INSUFFICIENT_EVIDENCE`.
- Otherwise a `reopen_pending` history entry is added, product claims that rely on the decision are suspended, and the decision is re-evaluated under this method.

---

## 12. Experiment records and number provenance

### 12.1 Files per experiment

```
research/experiments/<EXP-ID>/
  spec.yaml     ebr spec (schema ebr.spec.v1): candidates, command, corpus, metrics, repetitions, warmups, seeds, timing, platform, guard, analysis
  record.md     pre-registration record (template below); committed together with spec.yaml BEFORE any run
  outcome.json  outcome classification (§10.1); written after analysis
  review-<n>.md independent re-analysis dispositions (§10.2), if any
  adversarial-search/  inputs, procedure and outputs of the counterevidence search (§8.2 item 5)
research/raw/<EXP-ID>/          raw JSONL (hash-chained) + meta, produced by `ebr run`
research/normalized/<EXP-ID>/   CSV + summary.json, produced by `ebr normalize`
research/reports/{tables,figures}/<EXP-ID>/  produced by `ebr report`
```

Decision-analysis outputs (§14 item 2) are written under `research/normalized/<EXP-ID>/decision/`, with the same provenance header convention as ebr outputs.

### 12.2 `record.md` template

```markdown
# <EXP-ID>: <title>

## Pre-registration
- decision_ids: [DEC-...]
- decision type (as assigned in the ledger, with the assigning session): EMPIRICAL | FORMAL | HUMAN-FACING (+ EXTERNAL dossier ref)
- od_ids and hc_ids (ledger schema v2): <...>
- method: research/decision-method.md @ <commit sha>
- pre-registration commit: <filled by the commit that adds this file; runs must descend from it>
- author (agent/session), independence from candidate designers, and any L7 exposure: <...>

## Question and hypotheses
- question: <one sentence>
- H1 (directional, with predicted effect size in band units): <...>
- H0 / equivalence hypothesis: <...>
- what result would falsify H1: <...>

## Candidates
| candidate_id | exact definition (commit, flags, versions, params) | computed cost tier (C1-C3 script output; C4-C7 assessor) | applicable families | public tuning data sources |
- reference candidate: <id> (normally status quo)
- provisional winner W and tuning survivor set S (validation looks only): <...>
- candidates excluded before execution and why (rule id; counterexample and reviewer for L0/R1(b)): <...>

## Corpus
- splits used: tuning | validation (look number from research/decisions/validation-looks.jsonl) | heldout (post-unlock only, unlock sha)
- items: list of item_id with logical_tree_sha256, or manifest sha256 + selection rule
- families, scale tiers, evaluation conditions, and the minimum-support check (§4.9)
- family weights: research/methods/workload-mix.json @ <sha256>
- generated items: generator, params, seed, sha256

## Metrics
| metric (canonical, Appendix A.2) | OD | role (primary/secondary/diagnostic) | family | unit | direction | threshold ID | measurement source (process / in-process / batch) |
- at most one primary metric per OD; justification and reviewer for any second one

## Primary comparisons
- confirmatory: W vs each s in S, per primary metric, corpus level; declared superiority metrics per pair (k_sup)
- confidences: superiority 1 - 0.01/k_sup; non-inferiority 0.99; family guard 0.90; held-out 0.90 (one-sided 0.95)
- MDE per comparison (validation <= 1 band unit; held-out <= 2 band units) with the variance source

## Protocol
- env_id(s) and qualifiers (VIRTUALIZED, EMULATED); timing: true|false; affinity configuration (§4.2/§4.3); cache state (§4.5)
- warmups; minimum rounds, step, cap (§4.6, including n_min for the confidence); operations per round or batch size (latency/remote)
- guard limits (thresholds.json, spec tightening), max_wait_s, cooldown, canary, frequency covariate availability, throttling onset
- network profile(s) and connection model (§4.15) or usability task set (§4.16), if applicable
- calibration experiment ids covering this env and these families (A/A, A/A', known effect, background load; §4.7)
- seeds: order, bootstrap, command

## Analysis plan
- aggregation levels and weights (§4.9); applicability; strata
- R4 dominance scopes; R9 candidate partitions and their expressibility
- per-OD base weights for sensitivity (§6.3 / Appendix B); workload mixes used; §6.5 arms; selection bootstrap seed
- exact commands: ebr validate/run/verify-raw/normalize/report + decision tooling invocations

## Expected cost and budget
- estimated machine time; agent involvement limited to <design | analysis | review>

## Deviations (append-only; each with UTC time, author, reason, results-visible yes/no)

## Outcome (after analysis)
- classification: positive | negative | null | inconclusive | abandoned (reason)
- verdict table reference: <path#row key>
- confirmation-bias triggers evaluated (§10.2): <ids fired / none>
- threats to validity specific to this experiment
```

### 12.3 Every reported number is generated from raw artifacts

1. **Scope.** Every number reported as a result or used in a decision or claim is covered: ledger fields, reports, PROGRESS result notes, README and website claims, and commit messages that state results. Each such number MUST be produced by a checked-in command from raw artifacts, via the chain `ebr run`, `ebr verify-raw`, `ebr normalize`, `ebr report`, decision tooling (§14).
2. **Citation.** Each cited number gives the output file, the row key (and column), and the generating command recorded in that output's provenance header.
3. **No manual numbers.** Numbers MUST NOT be hand-typed from memory, computed by an agent's own arithmetic, or copied from terminal scrollback.
   - Prose may round a generated value to 3 significant figures (round half to even), or keep fewer digits than the table shows. The generated table value remains authoritative.
   - Derived quantities in prose, such as percentages or differences, MUST themselves be generated.
4. **Regeneration.** `normalize`, `report` and the decision tooling MUST regenerate byte-identical outputs from the same raw data and tool commit, as `research/experiments/EXP-SMOKE-000/run_smoke.sh` demonstrates. A mismatch invalidates the dependent numbers until resolved (§8.5).
5. **Figures** come only from `ebr report` or the decision tooling.
6. **Not results.** Numbers taken from external sources carry `EXTERNALLY_SOURCED` citations. Pre-registered thresholds in this document are method choices. Method-check outputs (Appendix D) are generated by the command recorded there. Bookkeeping counts (for example corpus or ledger sizes) cite their command where practical, as in §0.1 and §4.9.

---

## 13. Execution budget reality

Experiments are **compute-heavy** and **agent-budget-light** by design. Agent tokens are the scarce resource: this program has been interrupted twice by usage limits (PROGRESS, 2026-09-13 and 2026-09-16). Machine time is comparatively plentiful.

- **Scripted execution.** Experiments run as scripts: ebr specs executed in the background, resumable, with raw output committed at checkpoints. Agents do not babysit runs, poll verbosely, or read raw JSONL. They read `summary.json`, generated tables and failure lists.
- **Agent judgment is reserved for:** candidate-set construction, experiment design and pre-registration records, analysis and interpretation of generated outputs, adversarial review, gate audits, and ledger decisions.
- **Mechanical stages** use the cheaper model, per the PROGRESS 2026-09-16 pacing decision: provisioning, verification, harness boilerplate, experiment execution, bulk ledger field updates by script.
- **Budget never weakens the method.** If budget prevents executing the protocol, the decision stays `INSUFFICIENT_EVIDENCE`, and the gap is recorded as a deviation and a limitation. It is never allowed to:
  - cut repetitions below the §4.6 minimums;
  - skip calibration, held-out validation, sensitivity analysis or a gate;
  - substitute agent estimates for measurements.
- **Batching.** Experiments SHOULD share runs where specs allow, for example one pack pass that measures size, determinism and section accounting together. Deterministic metrics (§4.13) SHOULD precede timing, because they need only 2 repetitions and often eliminate candidates (R1, R2, R4) before any expensive timing campaign.
- **Timing windows** are exclusive (§4.2) and are scheduled when no agent workflow runs, which suits overnight unattended execution.

---

## 14. Integration prerequisites and gates

No verdict is decision-grade until each item it depends on exists, is tested (unit tests plus golden files where applicable), and is committed. The gate column refers to §0.4. At this revision every item is `[UNRESOLVED]` except item 1, which this commit transcribes (with the test update still open).

| # | Item | Gate |
|---|---|---|
| 1 | `research/methods/thresholds.json` transcribed per §3.7 and Appendix A (done at this commit) and loaded by ebr v1 with the documented values (Appendix A.1). The ebr suite (`research/tools/run_tests.sh`) passes at this commit except `research/tools/tests/test_spec.py::test_repo_thresholds_file_is_template`, which asserts the pre-transcription null template. That test must be updated, outside this commit, to assert `status: pre-registered` and the Appendix A.3 check. | G-B |
| 2 | Decision-analysis tooling (for example `research/tools/decide/`): metric-level thresholds with floors, profile, tier and stratum handling; exact order-statistic item intervals; Welch-Satterthwaite corpus intervals and pooled family intervals (§4.10); three-way verdicts plus `NON_INFERIOR`; confirmatory-set confidences and MDEs (§4.12); L2-L4 aggregation with cited-mix weights; the family harm guard; R4 including computed tier; log-consistent minimum-gain bands and absolute caps; continuous band-unit utility, R10 tie set, regret and R9 partitions, with the selected candidate passed explicitly; §6 sensitivity (weights, SMAA report, thresholds with the resolvability condition, all §6.5 arms, the selection-stability bootstrap) | G-B |
| 3 | Runner additions: AC-power check; Windows power-throttling opt-out; SMT sibling verification; inter-sample cooldown; drift canary on the experiment's affinity set; PDH frequency covariate and throttling-onset probe; per-core-class guard accounting; MsMpEng and SearchIndexer covariates; host-side sampler for WSL runs (CPU, MsMpEng, VHDX I/O); invalid-rate balance check; `vm-cold` cache mode; write accounting for scratch claims; environment-block padding for A/A′; in-process and batch timing hooks; clean-worktree and pre-registration-ancestry check for decision specs; spec lint (no `thresholds_override`, no sensitivity-sample overrides, guard values only tighter, `max_loadavg_1m` = 1.0 + threads) | G-B for timing |
| 4 | Calibration experiments per environment and family: A/A, A/A′, known effect, background load, speedup A/A (§4.7) | G-B for timing and memory |
| 5 | Held-out lock frozen (§5.3); consistency checker for `design-freeze.json` and `heldout-unlock.json` (§5.5) | G-B; G-C |
| 6 | Ledger validator for `DECIDED` rows (§8.2) with blocks and caps (§8.4) | G-D |
| 7 | Cost-accounting scripts for C1-C3 and the C4-C7 assessment template (§7.1, §7.2) | before the first validation look |
| 8 | Outcome records and negative-results register (§10.1) | every experiment |
| 9 | Full rule simulation on synthetic and pilot (non-decision) outcome tables through R0-R10: ties, noise, conflicting families, interval coverage on the actual group structures (extending Appendix D.1), tie handling (D.3), null-pass rates of R5 (D.2), and the program-level expected false-selection rate. Outputs under `research/methods/` and cited in DEC-ECO-078. Appendix D records the scenarios already run. | before the first validation look; G-D for DEC-ECO-078 |
| 10 | Constraint crosswalk `research/methods/constraint-crosswalk.csv` over every distinct ledger constraint string, with classifier and reviewer sign-off (R1); objective Appendix B extended with every freeze it finds | G-A |
| 11 | `research/tools/ledger/objective_screen.py` pattern corrected to `\bI([1-9]|[12][0-9]|3[01])\b` within strings, and objective Appendix C regenerated | with item 10 |
| 12 | Ledger schema v2: `hc_ids` with per-HC status, `od_ids`, `decision_type` with assigning session, primary metrics per OD, computed cost tier, validation look references, freeze reference, governing method SHA, gate-audit references. An initial `od_ids` and `hc_ids` assignment generated from objective Appendix C.3, the crosswalk and the ledger `cluster`, reviewed by a session outside the program's design work. Validator extension enforcing R3, §8.2 items 13-15. History notes on DEC-CMP-035 (the SPEC §5.4 constrained form and SPEC §25.3 fitted mix, §2.0 and §4.9), DEC-ECO-076 (single freeze, §5.5) and DEC-ECO-078 (Appendix D) | G-A |
| 13 | Workload mix `research/methods/workload-mix.json` (Appendix B.3) with primary-source citations | G-B |
| 14 | Validation look registry `research/decisions/validation-looks.jsonl` (§5.2) | G-A |
| 15 | Held-out: lineage regrouping and content-overlap audit (§5.3); transcript-audit script (§5.4); removal or encryption of materialized held-out content (§5.4); L7 exposure record (§5.8 h). Optional: sealed tranches (§5.4). | G-C |
| 16 | OD-16 attack suite, including an adaptive attack, and the observer model, committed (T-17) | before any OD-16 result |
| 17 | Counting allocator with allocation-site attribution, and the pre-validation bound H per candidate (objective MVT-06(a)) | before a candidate's first HC-06 record |
| 18 | `EXP-NETEM-CAL` with the §4.15 acceptance criteria, and the connection model per profile | before any remote result |
| 19 | Specialist baseline configurations of objective §1.3 in `research/baselines/` | before any CC outcome |

---

## Appendix A. Machine-readable transcription

### A.1 The file

- `research/methods/thresholds.json` is the exact transcription of §3.2, §3.7, §4, §5.2-§5.3, §6, §7 and §10.2, of the numeric parameters of Appendix C.1, and of Appendix A.2, in schema `ebr.thresholds.v1`, with extension keys (§3.7).
- `status` is `pre-registered`. A file cannot contain the SHA of the commit that adds it, so `pre_registration.commit` holds the command that resolves it by commit subject: `git log --format=%H --fixed-strings --grep="research: pre-register decision method and archetypal objective" -- research/methods/thresholds.json`. A pickaxe search for the status label would find the unfinished WIP checkpoint `3fda801`, which already carried the label and is not the pre-registration record (§0.1).
- Every value group carries a `source`, or a `sources` map with one entry per key, citing the section of this document. Appendix A.3 checks that every cited section exists.
- Checked against ebr v1 in the WSL research venv: `ebr.thresholds.Thresholds.load` accepts the file; the bands and epsilons of every family except `other` resolve; `other` raises `ThresholdUnset` by design (§3.6); `bootstrap_confidence()` returns 0.99, `bootstrap_resamples()` 20000, and `guard()` returns 9.0 and 3.0.
- If this appendix and the sections it cites disagree, the sections govern and both the table and the file are corrected.

### A.2 Canonical metrics, roles and thresholds

Generated from the same registry as `thresholds.json` `metric_thresholds` and checked against it by A.3. "r" is the relative half-width (per profile where listed); "a" is the absolute floor (per tier or as a rule where listed); "–" means none. Roles are defined in §1.1. Objective metrics are the M-numbers of `archetypal-objective.md` §3.2 that the canonical metric implements; every M-number there appears at least once.

| Canonical metric | ID | Role | ebr family | Unit | Direction | r | a | Objective metrics |
|---|---|---|---|---|---|---|---|---|
| `artifact_bytes` | T-01 | banded | size | B | minimize | 0.01 | small 0; medium 64; large 64 | M05.1, M09.2, M09.4, M15.3, M20.3 |
| `total_stored_bytes` | T-01 | sensitivity_arm | size | B | minimize | 0.01 | 64 | M05.5 |
| `dedup_stored_fraction` | T-01 | diagnostic | size | ratio | minimize | 0.01 | – | M05.4 |
| `fixed_overhead_bytes` | T-21 | banded | size | B | minimize | – | 16 | M05.3 |
| `metadata_bytes` | T-02 | diagnostic | size | B | minimize | 0.05 | 64 | M05.2 |
| `index_bytes` | T-02 | diagnostic | size | B | minimize | 0.05 | 64 | M05.2 |
| `dictionary_bytes` | T-02 | diagnostic | size | B | minimize | 0.05 | 64 | M05.2 |
| `side_data_bytes` | T-02 | diagnostic | size | B | minimize | 0.05 | 64 | M05.2 |
| `refusal_bytes_read` | T-02 | banded | size | B | minimize | 0.05 | 64 | M17.2 |
| `metadata_bytes_per_entry` | T-02b | banded | size | B/entry | minimize | 0.05 | 0.5 | M05.2 |
| `encode_wall_s` | T-03 | banded | time_wall | s | minimize | fast 0.05; balanced 0.05; dense 0.1; extreme 0.25 | process 0.005; in_process 0.0001 | M06.1, M15.3, M19.5, M20.3 |
| `decode_wall_s` | T-04 | banded | time_wall | s | minimize | 0.05 | process 0.005; in_process 0.0001 | M07.1, M15.3, M19.5 |
| `verify_wall_s` | T-04 | banded | time_wall | s | minimize | 0.05 | process 0.005; in_process 0.0001 | M07.2 |
| `stream_unpack_wall_s` | T-04 | banded | time_wall | s | minimize | 0.05 | process 0.005; in_process 0.0001 | M09.3 |
| `encode_cpu_s` | T-05 | banded | time_cpu | s | minimize | fast 0.05; balanced 0.05; dense 0.1; extreme 0.25 | process 0.005; in_process 0.0001 | M06.2 |
| `decode_cpu_s` | T-05 | banded | time_cpu | s | minimize | 0.05 | process 0.005; in_process 0.0001 | M07.1 |
| `refusal_cpu_s` | T-05 | banded | time_cpu | s | minimize | 0.05 | in_process 0.0001 | M17.2 |
| `encode_throughput_bps` | T-05b | banded | throughput | B/s | maximize | fast 0.05; balanced 0.05; dense 0.1; extreme 0.25 | – | M06.4 |
| `decode_throughput_bps` | T-05b | banded | throughput | B/s | maximize | 0.05 | – | M07.1 |
| `verify_throughput_bps` | T-05b | banded | throughput | B/s | maximize | 0.05 | – | M07.2 |
| `peak_rss_bytes` | T-06 | banded | memory_peak | B | minimize | 0.1 | 4194304 | M08.1, M19.5, M20.3 |
| `peak_commit_bytes` | T-06 | banded | memory_peak | B | minimize | 0.1 | 4194304 | M08.1 |
| `alloc_peak_bytes` | T-06 | banded | memory_peak | B | minimize | 0.1 | 1048576 | M08.1 |
| `refusal_alloc_peak_bytes` | T-06 | banded | memory_peak | B | minimize | 0.1 | 1048576 | M17.2 |
| `scratch_peak_bytes` | T-07 | banded | scratch_peak | B | minimize | 0.1 | 16777216 | M08.2, M20.3 |
| `scratch_write_bytes` | T-07 | binary | correctness | B | equal_zero_when_claimed | – | – | M08.2 |
| `bounded_memory_growth_bytes` | HC-06 | binary | correctness | B | at_most_bound | – | – | M08.3 |
| `first_entry_latency_s` | T-08 | banded | time_wall | s | minimize | 0.1 | in_process 0.0001 | M07.4 |
| `random_entry_latency_s` | T-08 | banded | time_wall | s | minimize | 0.1 | in_process 0.0001 | M10.1 |
| `encrypted_random_entry_latency_s` | T-08 | banded | time_wall | s | minimize | 0.1 | in_process 0.0001 | M10.5 |
| `list_latency_s` | T-08 | banded | time_wall | s | minimize | 0.1 | in_process 0.0001 | M07.3 |
| `open_latency_s` | T-08 | banded | time_wall | s | minimize | 0.1 | in_process 0.0001 | M10.3 |
| `access_granularity_bytes` | T-09 | banded | size | B | minimize | 0.25 | 16384 | – |
| `access_amplification` | T-22 | banded | other | ratio | minimize | 0.25 | 16384/stratum_bytes | M10.2 |
| `remote_first_entry_latency_s` | T-10 | banded | time_wall | s | minimize | 0.1 | max(0.010, 0.5*rtt_s) | M12.3 |
| `remote_random_entry_latency_s` | T-10 | banded | time_wall | s | minimize | 0.1 | max(0.010, 0.5*rtt_s) | M12.3 |
| `remote_task_latency_s` | T-10 | banded | time_wall | s | minimize | 0.1 | max(0.010, 0.5*rtt_s) | M12.3 |
| `http_bytes` | T-11 | banded | size | B | minimize | 0.05 | 16384 | M12.2 |
| `verified_range_fetch_bytes` | T-11 | banded | size | B | minimize | 0.05 | 16384 | M11.1 |
| `open_bytes_read` | T-11 | banded | size | B | minimize | 0.05 | 16384 | M10.3 |
| `http_requests` | T-12 | banded | other | count | minimize | 0.1 | 0.5 | M12.1 |
| `range_length_signatures` | T-12 | banded | other | count | minimize | 0.1 | 0.5 | M16.4 |
| `verification_digest_ops` | T-12 | diagnostic | other | count | minimize | 0.1 | 0.5 | M11.2 |
| `verify_overhead_pp` | T-13 | banded | other | pp | minimize | – | 5 | M11.1 |
| `parallel_speedup` | T-14 | banded | other | ratio | maximize | 0.1 | 0.2 | M13.1 |
| `parallel_efficiency` | T-14 | diagnostic | other | ratio | maximize | 0.1 | – | M13.2 |
| `blast_logical_bytes_p95` | T-15 | banded | other | B | minimize | 0.5 | 65536 | M14.1 |
| `blast_logical_bytes_max` | T-15 | banded | other | B | minimize | 0.5 | 65536 | M14.1 |
| `blast_entries_p95` | T-15 | banded | other | count | minimize | 0.5 | 0.5 | M14.1 |
| `padding_overhead_pp` | T-16 | banded | other | pp | minimize | – | 1 | M16.3 |
| `leak_min_entropy_bits` | T-17 | banded | other | bits | minimize | – | 1 | M16.1 |
| `leak_shannon_bits` | T-17 | banded | other | bits | minimize | – | 1 | M16.1 |
| `anonymity_set_median` | T-17 | banded | other | count | maximize | 1 | 0.5 | M16.1 |
| `presence_advantage` | T-17 | banded | other | fraction | minimize | – | 0.05 | M16.2 |
| `task_success_rate` | T-18 | heuristic | other | fraction | maximize | – | – | M25.1 |
| `task_steps` | T-18 | heuristic | other | count | minimize | – | – | M25.1 |
| `task_errors` | T-18 | heuristic | other | count | minimize | – | – | M25.1 |
| `task_completion_time_s` | T-18 | heuristic | other | s | minimize | – | – | M25.5 |
| `fs_read_ops` | T-19 | secondary_only | io | count | minimize | 0.1 | 64 | – |
| `fs_write_ops` | T-19 | secondary_only | io | count | minimize | 0.1 | 64 | – |
| `capture_fidelity_fraction` | T-20 | banded | other | fraction | maximize | – | 0.5/n_cases | M03.1 |
| `restore_fidelity_fraction` | T-20 | banded | other | fraction | maximize | – | 0.5/n_cases | M03.2 |
| `reason_code_specificity_fraction` | T-20 | banded | other | fraction | maximize | – | 0.5/n_cases | M04.2 |
| `verified_fraction` | T-20 | banded | other | fraction | maximize | – | 0.5/n_cases | M11.3 |
| `localization_precision` | T-20 | banded | other | fraction | maximize | – | 0.5/n_cases | M14.2 |
| `truncation_salvage_fraction` | T-20 | banded | other | fraction | maximize | – | 0.5/n_cases | M14.3 |
| `benign_false_refusal_fraction` | T-20 | banded | other | fraction | minimize | – | 0.5/n_cases | M17.3 |
| `cross_platform_restore_fidelity_fraction` | T-20 | banded | other | fraction | maximize | – | 0.5/n_cases | M18.2 |
| `import_acceptance_fraction` | T-20 | banded | other | fraction | maximize | – | 0.5/n_cases | M19.1 |
| `import_agreement_fraction` | T-20 | banded | other | fraction | maximize | – | 0.5/n_cases | M19.2 |
| `export_acceptance_fraction` | T-20 | banded | other | fraction | maximize | – | 0.5/n_cases | M19.4 |
| `error_actionability_fraction` | T-20 | banded | other | fraction | maximize | – | 0.5/n_cases | M25.3 |
| `declared_tightness_ratio` | T-23 | banded | other | ratio | minimize | 0.1 | – | M17.1 |
| `equivalence_assertion_pass_fraction` | HC-01/HC-03 | binary | correctness | fraction | equal_one | – | – | M01.3 |
| `roundtrip_mismatches` | HC-02 | binary | correctness | count | equal_zero | – | – | M02.1 |
| `reader_disagreements` | HC-03 | binary | correctness | count | equal_zero | – | – | M04.1 |
| `streaming_capability` | HC-06/F-09 | binary | correctness | binary | equal_one_when_claimed | – | – | M09.1 |
| `declared_cost_accuracy` | HC-09 | binary | correctness | ratio | at_most_one | – | – | M10.4 |
| `origin_protocol_safety` | HC-18 | binary | correctness | binary | equal_one | – | – | M12.4 |
| `localization_recall` | HC-15 | binary | correctness | fraction | equal_one | – | – | M14.2 |
| `undetected_tampers` | HC-14 | binary | correctness | count | equal_zero | – | – | M15.1 |
| `authenticated_coverage_fraction` | HC-14 | binary | correctness | fraction | equal_one_per_archive_type_scope | – | – | M15.2 |
| `adversarial_true_refusal_fraction` | HC-06 | binary | correctness | fraction | equal_one | – | – | M17.4 |
| `cross_host_identity_agreement` | HC-08 | binary | correctness | binary | equal_one | – | – | M18.1 |
| `hostile_name_handling_fraction` | HC-08 | binary | correctness | fraction | equal_one | – | – | M18.3 |
| `ambiguity_refusal_fraction` | HC-07 | binary | correctness | fraction | equal_one | – | – | M19.3 |
| `migration_identity_conformance_fraction` | HC-10 | binary | correctness | fraction | equal_one | – | – | M20.1 |
| `migration_determinism_fraction` | HC-13 | binary | correctness | fraction | equal_one | – | – | M20.2 |
| `receipt_completeness_fraction` | HC-07 | binary | correctness | fraction | equal_one | – | – | M20.4 |
| `cleanroom_pass_fraction` | HC-12 | binary | correctness | fraction | equal_one | – | – | M21.1 |
| `normative_rule_coverage_fraction` | HC-12 | binary | correctness | fraction | equal_one | – | – | M21.4 |
| `unspecified_baseline_decode_steps` | HC-12 | binary | correctness | count | equal_zero | – | – | M21.5 |
| `license_compatibility` | R2 | binary | correctness | binary | equal_one | – | – | M22.3 |
| `baseline_codec_public_spec` | HC-12 | binary | correctness | binary | equal_one | – | – | M22.5 |
| `unresolved_fuzz_findings_at_coverage_target` | HC-03/HC-06 | binary | correctness | count | equal_zero | – | – | M24.2 |
| `unsafe_defaults` | HC-05 | binary | correctness | count | equal_zero | – | – | M25.4 |
| `single_file_self_contained` | HC-09 | binary | correctness | binary | equal_one | – | – | M26.5 |
| `validation_obligations_count` | - | descriptive | other | count | report | – | – | M01.1 |
| `conditional_decoding_rules_count` | - | descriptive | other | count | report | – | – | M01.2 |
| `declared_gap_count` | - | descriptive | other | count | report | – | – | M03.3 |
| `decided_class_reach_fraction` | - | descriptive | other | fraction | report | – | – | M04.3 |
| `verification_cpu_share` | - | descriptive | other | fraction | report | – | – | M06.3 |
| `unstable_source_retry_cost_s` | - | descriptive | other | s | report | – | – | M06.5 |
| `assurance_state` | - | descriptive | other | ordinal | report | – | – | M15.4 |
| `declared_security_level_bits` | - | descriptive | other | bits | report | – | – | M15.5 |
| `hostile_cpu_scaling_exponent` | - | descriptive | other | 1 | report | – | – | M17.5 |
| `scale_limits` | - | descriptive | other | count; B | report | – | – | M17.6 |
| `platform_coverage_count` | - | descriptive | other | count | report | – | – | M18.4 |
| `import_majority_agreement_fraction` | - | descriptive | other | fraction | report | – | – | M19.2 |
| `export_outcome_distribution` | - | descriptive | other | fraction | report | – | – | M19.4 |
| `spec_defects_per_1000_words` | - | descriptive | other | count/1000 words | report | – | – | M21.3 |
| `release_age_years` | - | descriptive | other | years | report | – | – | M22.4 |
| `normative_words_per_rule` | - | descriptive | other | words/rule | report | – | – | M23.1 |
| `superlinear_structure_count` | - | descriptive | other | count | report | – | – | M24.5 |
| `zero_install_scenario_fraction` | - | descriptive | other | fraction | report | – | – | M26.1 |
| `migration_steps_count` | - | descriptive | other | count | report | – | – | M26.2 |
| `build_availability` | - | descriptive | other | count; s | report | – | – | M26.3 |
| `identification_support_count` | - | descriptive | other | count | report | – | – | M26.4 |
| `independent_reader_loc` | C2 | cost_input | other | LoC | minimize | – | – | M21.2 |
| `reader_fragmentation_count` | C5 | cost_input | other | count | minimize | – | – | M21.6 |
| `decode_path_dependency_count` | C3 | cost_input | other | count | minimize | – | – | M22.1 |
| `decode_path_native_unsafe_count` | C3/C4 | cost_input | other | count | minimize | – | – | M22.2 |
| `rustsec_advisory_count` | C3 | cost_input | other | count | minimize | – | – | M22.3 |
| `maintainer_count` | C3 | cost_input | other | count | maximize | – | – | M22.4 |
| `msrv_gap` | C3 | cost_input | other | versions | minimize | – | – | M22.4 |
| `normative_words_added` | C1 | cost_input | other | words | minimize | – | – | M23.1, M23.3 |
| `wire_items_added` | C1 | cost_input | other | count | minimize | – | – | M23.2, M23.3 |
| `untrusted_input_loc` | C4 | cost_input | other | LoC | minimize | – | – | M24.1 |
| `attacker_controlled_parameters` | C4 | cost_input | other | count | minimize | – | – | M24.3 |
| `validated_only_vulnerability_classes` | C4 | cost_input | other | count | minimize | – | – | M24.4 |
| `flags_per_task` | C6 | cost_input | other | count | minimize | – | – | M25.2 |

Minimum-gain caps for absolute-only metrics (§7.3): `verify_overhead_pp` 25.0, `padding_overhead_pp` 5.0, `leak_min_entropy_bits` 4.0, `leak_shannon_bits` 4.0, `presence_advantage` 0.25, every T-20 fraction 0.25, `fixed_overhead_bytes` 512.

### A.3 Consistency check

Run from the repository root. It checks that:

1. the A.2 table and `thresholds.json` agree on every canonical metric's ID, role, family, unit, direction, r, a and objective M-numbers; every objective M-number appears in A.2; and every family band equals `[1/(1+r), 1+r]` for its epsilon r;
2. the ebr family given in brackets for each metric bullet of `archetypal-objective.md` §3.2 equals the A.2 families of its canonical metrics;
3. every key ebr v1 reads has the §3.7 value and carries a source;
4. every section, rule, appendix item and threshold ID cited by a `source`, `sources` or `decision_method_section` entry exists in this document.

Output at this revision:

```
metrics 131 | table rows 131 | mismatches none
objective M-numbers 105 | missing from A.2 none | unknown in A.2 none
family band/epsilon inconsistencies none
objective family brackets 26 bullets | differences from A.2 none
section 3.7 keys 24 | value mismatches none | without source none
source pointers 582 | unresolved none
```

```sh
C:/Python313/python.exe - <<'EOF'
import json, re
dm = open('research/decision-method.md', encoding='utf-8').read()
obj = open('research/archetypal-objective.md', encoding='utf-8').read()
th = json.load(open('research/methods/thresholds.json', encoding='utf-8'))
mt = th['metric_thresholds']


def fmt(v):
    if v is None:
        return '–'
    if isinstance(v, dict):
        return '; '.join(f'{k} {fmt(x)}' for k, x in v.items())
    if isinstance(v, float) and v.is_integer():
        return str(int(v)) if v >= 1 else str(v)
    return str(v)


# 1. Appendix A.2 table versus metric_thresholds; every objective M-number covered
sec = dm[dm.index('### A.2'):dm.index('### A.3')]
rows = {}
for line in sec.splitlines():
    m = re.match(r'\| `([a-z0-9_]+)` \|(.*)\|$', line)
    if m:
        rows[m.group(1)] = [c.strip() for c in m.group(2).split('|')]
bad = []
for name, e in mt.items():
    r = e.get('relative_by_profile', e.get('relative'))
    a = e.get('absolute_rule') or e.get('absolute_by_tier') or e.get('absolute')
    want = [e['id'], e['role'], e['family'], e['unit'], e['direction'], fmt(r), fmt(a), ', '.join(e['od_metrics']) or '–']
    if rows.get(name) != want:
        bad.append((name, rows.get(name), want))
bad += [(n, 'in table only') for n in set(rows) - set(mt)]
mids = set(re.findall(r'\*\*(M\d\d\.\d)\*\*', obj))
covered = {m for e in mt.values() for m in e['od_metrics']}
fam_bad = [f for f, v in th['families'].items() if v['pareto_epsilon'].get('kind') == 'relative' and
           (abs(v['practical_significance_band']['lower'] - 1 / (1 + v['pareto_epsilon']['value'])) > 5e-11 or
            abs(v['practical_significance_band']['upper'] - (1 + v['pareto_epsilon']['value'])) > 5e-11)]
print('metrics', len(mt), '| table rows', len(rows), '| mismatches', bad or 'none')
print('objective M-numbers', len(mids), '| missing from A.2', sorted(mids - covered) or 'none', '| unknown in A.2', sorted(covered - mids) or 'none')
print('family band/epsilon inconsistencies', fam_bad or 'none')

# 2. objective ebr-family brackets versus A.2 families, per metric bullet
FAM = {'time_wall', 'time_cpu', 'memory_peak', 'scratch_peak', 'size', 'throughput', 'io', 'correctness', 'other'}
m2f = {}
for e in mt.values():
    for m in e['od_metrics']:
        m2f.setdefault(m, set()).add(e['family'])
odsec = obj[obj.index('### 3.2 Dimension definitions'):obj.index('## 4. Relationship')]
fam_obj_bad = []
for bullet in re.findall(r'^- \*\*M\d\d\.\d\*\*.*$', odsec, re.M):
    ms = re.findall(r'\*\*(M\d\d\.\d)\*\*', bullet)
    stated = {f for u in re.findall(r'Units?: [^\[]*\[([^\]]+)\]', bullet) for f in re.findall(r'`([a-z_]+)`', u)} & FAM
    canon = set().union(*(m2f.get(m, set()) for m in ms))
    if stated != canon:
        fam_obj_bad.append((ms[0], sorted(stated), sorted(canon)))
print('objective family brackets', len(re.findall(r'^- \*\*M\d\d\.\d\*\*', odsec, re.M)), 'bullets | differences from A.2', fam_obj_bad or 'none')

# 3. every key ebr v1 reads: value equals the section 3.7 table, and it carries a source
t37 = dm[dm.index('### 3.7'):dm.index('## 4. Statistical protocol')]
diffs, nosrc, nkeys = [], [], 0
for key, val in re.findall(r'^\| `([a-z0-9_.*]+)` \| (.+?) \| [^|]+ \|$', t37, re.M):
    nkeys += 1
    parts = key.split('.')
    if parts[-1] == '*':
        fam = th['families'][parts[1]]
        got = [fam['practical_significance_band'], fam['pareto_epsilon']]
        if any(v is not None for g in got for v in g.values()) or 'source' not in fam:
            diffs.append((key, got))
        continue
    node = th
    for p in parts:
        node = node[p]
    if isinstance(node, dict):
        want = dict(re.findall(r'(\w+): ([\w.-]+)', val))
        for k, v in want.items():
            if str(node[k]) != v and not (k != 'kind' and float(node[k]) == float(v)):
                diffs.append((key, k, node[k], v))
        if 'source' not in node:
            nosrc.append(key)
    else:
        v = re.match(r'`([\d.]+)`', val).group(1)
        if float(node) != float(v):
            diffs.append((key, node, v))
        parent = th
        for p in parts[:-1]:
            parent = parent[p]
        if parts[-1] not in parent.get('sources', {}):
            nosrc.append(key)
print('section 3.7 keys', nkeys, '| value mismatches', diffs or 'none', '| without source', nosrc or 'none')

# 4. every source pointer resolves to a section of this document
heads = set(re.findall(r'^#{2,3} (?:Appendix )?([A-Z]?\d*(?:\.\d+)?)[. ]', dm, re.M))
heads |= set(re.findall(r'^### (R\d+)\.', dm, re.M)) | set(re.findall(r'\*\*(D\.\d)\*\*', dm))
heads |= set(re.findall(r'^\| (T-\d\d[a-z]?) \|', dm, re.M))
unresolved, npointers = [], 0


def walk(node, path):
    global npointers
    if isinstance(node, dict):
        for k, v in node.items():
            walk(v, path + [k])
    elif isinstance(node, str) and (path[-1] in ('source', 'decision_method_section') or (len(path) > 1 and path[-2] == 'sources')):
        text = re.sub(r'objective [^,;)]*', '', node.split('research/decision-method.md', 1)[-1])
        toks = re.findall(r'§(\d+(?:\.\d+)?)|\b(R\d+)\b|Appendix ([A-Z](?:\.\d)?)|\b(T-\d\d[a-z]?)\b|(?<![\w.§-])(\d+(?:\.\d+)?)(?![\w.])', text)
        for t in toks:
            ref = next(x for x in t if x)
            npointers += 1
            if ref not in heads:
                unresolved.append(('.'.join(path), ref))


walk(th, [])
print('source pointers', npointers, '| unresolved', unresolved or 'none')
EOF
```

---

## Appendix B. Profile decisions, workload mixes and family weights

### B.1 Profile-scope decisions: the constrained form (no weights)

SPEC §5.4 defines a profile as minimising stored bytes subject to decode throughput, decode memory, access granularity and compression budget. Round 0 scalarized those quantities with weights, which counted the constrained quantities twice and contradicted SPEC §5.4 and §25.3 (review finding A05). Profile-scope decisions therefore use no weights:

| Role in the decision | Metrics |
|---|---|
| R2 screen (declared profile bounds, once DEC-CMP-035 fixes them) | decode memory (`alloc_peak_bytes`, or the platform primary of §4.14), `access_granularity_bytes`, compression budget (`encode_wall_s`, `encode_cpu_s`), decode throughput floor (`decode_throughput_bps`) |
| Superiority metric (R4 condition 1, R8) | `artifact_bytes` only |
| Non-inferiority guards (R4 conditions 2 and 3, R5 d and e) | `encode_wall_s` (at the profile's T-03 band), `decode_wall_s`, `alloc_peak_bytes`, `access_granularity_bytes` |
| R10 keys | as R10 |

Sensitivity for profile decisions uses the threshold perturbations, the §6.5 family arms and the selection-stability bootstrap; there are no weights to perturb.

### B.2 Workload mixes (§6.5)

The mixes are `[INFERRED]` from the SPEC §6 "use when" statements and the corpus families (`research/corpus/methodology.md` §2). Round 0 called them "scenarios"; they are renamed so they are not confused with the objective's evaluation conditions (AGG-S).

| Mix | Families (0.8 of the weight, uniform) | Source of intent |
|---|---|---|
| WM-DIST: software distribution and release artifacts | F01, F02, F03, F09, F11, F14, F18 | SPEC §6.3 |
| WM-CI: CI build artifacts and intermediate data | F02, F03, F04, F05, F09, F17 | SPEC §6.1 |
| WM-BACKUP: backup, cold storage, long-term archival | F04, F05, F08, F15, F16, F17, F18, F19 | SPEC §6.4 |
| WM-DATA: data exchange and scientific data | F06, F07, F08, F10 | corpus families F06-F08, F10 |
| WM-MEDIA: media collections | F11, F12, F13 | corpus families F11-F13 |

- The remaining 0.2 of the weight is spread uniformly over the other applicable families.
- F20 (adversarial and high-entropy inputs) is in no mix. It is the main screening family for R1 and R2.

### B.3 Base family weights: the cited workload mix

`research/methods/workload-mix.json` (§14 item 13, gate G-B) fixes the L4 base weights before any decision-relevant result, as SPEC §25.3 requires ("fit to a corpus that reflects real file-type mixes"). Its content rules:

- one weight per corpus family F01-F19, derived from primary sources on real file-type mixes (for example published measurements of file-type composition by bytes in software repositories, package registries, backup populations or public data archives), each cited with title, version or date, locator and access date (§9.3);
- the derivation from sources to weights is a checked-in script, and the file records its SHA-256;
- every weight is at least 0.01; F20 receives exactly 0.01, because adversarial inputs have no real-world share but must stay in the aggregate; weights are renormalized to sum to 1;
- the file is written by a session that has not seen any decision-relevant result, and is frozen by Commit A (§5.5).

Equal family weights remain a sensitivity arm (§6.5). Until the file exists, no L4 value is decision-grade.

---

## Appendix C. Canonical parameters

### C.1 Parameter table

`archetypal-objective.md` cites these values; where a copy there differs, this table governs and the objective is corrected (review finding A06).

| Parameter | Canonical value | Section |
|---|---|---|
| Network profiles | NP-LAN 2 ms / 1000 Mbit/s; NP-METRO 20 ms / 200 Mbit/s; NP-CONT 80 ms / 50 Mbit/s; NP-INTER 200 ms / 20 Mbit/s. Loss only for failure behaviour. | §4.15; objective OD-12 |
| Decision-grade Windows timing configurations | `st-p` {2}; `mt-p-n` n ∈ {2, 4, 7}; `mt-psmt-14` {2..15}; `st-e` {18}; `mt-e-n` n ∈ {2, 4, 8, 16}. `os-scheduled` informational. | §4.2 |
| Decision-grade WSL timing configurations | `st-v` {2}; `mt-v-n` n ∈ {2, 4, 8} | §4.3 |
| Thread counts for HC correctness variation (not timing) | {1, 2, 8, 32} for decode (MVT-11(a)); {1, 8, 32} for deterministic pack (MVT-13(a)) | objective HC-11, HC-13 |
| Speedup | within one core class: P {1 → 2, 4, 7}; E {1 → 2, 4, 8, 16}; SMT `mt-psmt-14` against `mt-p-7` | T-14; objective OD-13 |
| Deterministic metric repetitions | 2 | §4.6 |
| Determinism claim repetitions | 3 per variation cell; 20-repetition stress cell at maximum thread count with seeded background load | §4.13; objective MVT-13(a) |
| Timing rounds (min / step / cap) | small 10 / 10 / 30; medium 10 / 10 / 20; large 10 / 5 / 20; minimum also ≥ n_min(c) | §4.6 |
| Blast-radius injection | ≥ 200 single-byte corruptions per region stratum per archive (5 strata: payload, Index, metadata and manifest, dictionaries, integrity and footer); secondary classes 512 B, 64 KiB, 1 MiB overwrites and truncation; per injected event; equal stratum weights 0.2; worst-stratum guard | T-15; objective OD-14 |
| Verified partial retrieval | `verified_range_fetch_bytes` (T-11) and `verify_overhead_pp` (T-13) | objective M11.1 |
| Presence confirmation | `presence_advantage`: max over the committed attack suite of TPR − FPR at FPR 0.01, per padding mode | T-17; objective M16.2 |
| Access amplification strata | 1 B, 4 KiB, 1 MiB, whole entry | T-22; objective M10.2 |
| Scale tiers | small ≤ 16 MiB, medium ≤ 512 MiB, large ≤ 8 GiB; guards and reporting per tier | §4.9 |
| Evaluation conditions versus workload mixes | conditions: network profile, platform pair, user task, legacy format, padding mode (never averaged); mixes: WM-* (Appendix B.2, sensitivity only) | §1.1 |
| Batch size for per-operation latency | ≥ 1,000 operations per batch; ≥ 40 sampled operations per item per round for p95 | T-08; §4.6 |
| Path coverage for HC-02 | ≥ 5 exercises per registered path per (profile, layout, encryption) cell | objective MVT-02(e) |
| Complexity bound | series 2^10-2^20 elements; 3 runs per point; slope 0.99 interval; δ = 0.15 | objective MVT-06(d) |
| Allocator baseline | R0_max = maximum over 20 runs | objective MVT-06(a) |
| Bounded-memory claim | probe sizes 1, 4, 16 and 64 GiB, 3 runs each; growth 1 → 64 GiB ≤ max(4 MiB, 0.10 × R0) | objective M08.3 |
| Keyed-boundary chance rate | 1,000 distinct-key pairs per item; 0.999 quantile; at most 1 key in 1,000 above it | objective MVT-14(e) |
| Content overlap across splits | ≤ 0.01 of logical bytes per family per split pair | §5.3 |
| Validation looks | ≤ 2 per decision; program-wide registry | §5.2 |
| Quiet-machine guard | system 3.0%; pinned set 2.0%; SMT siblings 2.0%; WSL load average 1.0 + n (envelope 9.0); max wait ≥ 300 s | §4.4 |
| Drift canary and frequency | canary limit 1.025 on the experiment's affinity set; frequency ≥ 0.975 of session baseline | §4.2 |
| Emulator calibration | RTT ±10%; throughput ±10%; counts exact | §4.15 |

---

## Appendix D. Method checks (synthetic)

These are checks of the method itself on synthetic data, not experiments under §4 and not decision-relevant results. They use only the standard library, so they run on the Windows host without the research venv. Command, from any directory: `C:/Python313/python.exe method_sim_r1.py all` with the script below saved as `method_sim_r1.py` (about 40 s).

- **D.1** Corpus-level interval coverage on the actual validation and held-out group structures (review Appendix R.6 counts), 20,000 simulations per cell, family means N(0, 0.03²) as fixed strata, group-level values with standard deviation 0.05 under four models: `gauss`; `hetero` (per-family standard deviation multiplied by exp(N(0, 0.75²))); `heavy` (t with 3 degrees of freedom, unit variance); `onewild` (one family with 5× the standard deviation). `satt` is the §4.10 Welch-Satterthwaite interval; `pooled_t` is shown for comparison. Monte-Carlo standard error is about 0.0021 at 0.90 and 0.0007 at 0.99.
- **D.2** Held-out replication (R5 c) for a minimized metric with band b = ln 1.05, held-out structure, corpus standard error set to 0.25, 0.5 and 0.8 band units (0.8 corresponds to an MDE of 2 band units, D.7), true effects of 0, −1 and −2 band units. `old_R5c_pass` is round 0's rule (point estimate on the favourable side); `new_replicated` and `new_attenuated` are this revision's classes.
- **D.3** The review's tie scenario (every candidate within one band on five equally weighted metrics) and a sub-band-deficit scenario, with the grid and Dirichlet weight-preservation fractions computed as `stats.weight_sensitivity` does.
- **D.4** Minimum rounds and ranks of the exact order-statistic median interval (§4.6, §4.10).
- **D.5** Detection bounds used by objective MVT-05, MVT-13, MVT-14 and MVT-15.
- **D.6** Log-consistent minimum-gain bands (§3.1, §7.3).
- **D.7** The standard error, in band units, at which the one-sided 0.05, power 0.80 MDE equals 2 band units.

Output at this revision:

```
## D.4 exact order-statistic median CI: n_min and (rank k, coverage) for interval x_(k)..x_(n+1-k)
0.9 n_min 5 {8: (2, 0.92969), 10: (2, 0.97852), 11: (3, 0.93457), 15: (4, 0.96484), 20: (6, 0.95861), 30: (11, 0.90126)}
0.99 n_min 8 {8: (1, 0.99219), 10: (1, 0.99805), 11: (1, 0.99902), 15: (3, 0.99261), 20: (4, 0.99742), 30: (8, 0.99478)}
0.995 n_min 9 {8: None, 10: (1, 0.99805), 11: (1, 0.99902), 15: (2, 0.99902), 20: (4, 0.99742), 30: (7, 0.99857)}
0.998 n_min 10 {8: None, 10: (1, 0.99805), 11: (1, 0.99902), 15: (2, 0.99902), 20: (3, 0.9996), 30: (7, 0.99857)}
0.999 n_min 11 {8: None, 10: None, 11: (1, 0.99902), 15: (2, 0.99902), 20: (3, 0.9996), 30: (6, 0.99968)}
## D.5 detection bounds: 20 stress runs detect per-run mismatch probability >= 0.1391 with 95%; 10000 trials (rule of three) >= 0.0003 ; 1000 injections >= 0.003
## D.6 log-consistent tier bands (1+r)^k and (1+r)^-k
0.01 [(1, 1.01, 0.9900990099), (2, 1.0201, 0.9802960494), (3, 1.030301, 0.9705901479), (5, 1.0510100501, 0.9514656876), (10, 1.1046221254, 0.9052869547)]
0.05 [(1, 1.05, 0.9523809524), (2, 1.1025, 0.9070294785), (3, 1.157625, 0.8638375985), (5, 1.2762815625, 0.7835261665), (10, 1.6288946268, 0.6139132535)]
0.1 [(1, 1.1, 0.9090909091), (2, 1.21, 0.826446281), (3, 1.331, 0.7513148009), (5, 1.61051, 0.6209213231), (10, 2.5937424601, 0.3855432894)]
0.25 [(1, 1.25, 0.8), (2, 1.5625, 0.64), (3, 1.953125, 0.512), (5, 3.0517578125, 0.32768), (10, 9.3132257462, 0.1073741824)]
0.5 [(1, 1.5, 0.6666666667), (2, 2.25, 0.4444444444), (3, 3.375, 0.2962962963), (5, 7.59375, 0.1316872428), (10, 57.6650390625, 0.0173415299)]
## D.7 held-out MDE and null pass: SE/b at MDE = 2 bands (one-sided 0.05, power 0.80, normal) = 0.8044
## D.3 sensitivity under ties and sub-band deficits (grid factors 0.5..1.5 full, Dirichlet c=4, 10000 samples)
all-within-one-band (quantized): FPW grid 1.0 dirichlet 1.0 argmax base 0 | R10-eligible tie set size 3
quantized utilities [0.0, -0.2, -0.2] winner 0 FPW grid 1.0 dirichlet 1.0
continuous utilities [-0.72, -0.3, -0.44] winner 1 FPW grid 0.9981 dirichlet 0.9541
## D.2 held-out replication (minimized metric, band b = ln 1.05, pooled_t two-sided 0.90), nsim 20000
SE/b=0.25 true_effect=+0 bands: old_R5c_pass 0.4955 new_replicated 0.0001 new_attenuated 0.9889
SE/b=0.25 true_effect=-1 bands: old_R5c_pass 1.0000 new_replicated 0.4955 new_attenuated 0.5044
SE/b=0.25 true_effect=-2 bands: old_R5c_pass 1.0000 new_replicated 1.0000 new_attenuated 0.0000
SE/b=0.5 true_effect=+0 bands: old_R5c_pass 0.4955 new_replicated 0.0237 new_attenuated 0.6012
SE/b=0.5 true_effect=-1 bands: old_R5c_pass 0.9795 new_replicated 0.4955 new_attenuated 0.4934
SE/b=0.5 true_effect=-2 bands: old_R5c_pass 1.0000 new_replicated 0.9795 new_attenuated 0.0205
SE/b=0.8 true_effect=+0 bands: old_R5c_pass 0.4955 new_replicated 0.1045 new_attenuated 0.2339
SE/b=0.8 true_effect=-1 bands: old_R5c_pass 0.8952 new_replicated 0.4955 new_attenuated 0.2957
SE/b=0.8 true_effect=-2 bands: old_R5c_pass 0.9947 new_replicated 0.8952 new_attenuated 0.0853
## D.1 corpus-level interval coverage, nsim 20000 (covered at confidence c iff t_cdf(|est-truth|/se, df) <= 0.5 + c/2)
validation gauss 0.9: pooled_t 0.8953 satt 0.9049 | 0.95: pooled_t 0.9473 satt 0.9567 | 0.99: pooled_t 0.9899 satt 0.9928 | 0.998: pooled_t 0.9981 satt 0.9990
validation hetero 0.9: pooled_t 0.8735 satt 0.9038 | 0.95: pooled_t 0.9235 satt 0.9505 | 0.99: pooled_t 0.9750 satt 0.9880 | 0.998: pooled_t 0.9900 satt 0.9966
validation heavy 0.9: pooled_t 0.9011 satt 0.9233 | 0.95: pooled_t 0.9508 satt 0.9670 | 0.99: pooled_t 0.9886 satt 0.9943 | 0.998: pooled_t 0.9967 satt 0.9982
validation onewild 0.9: pooled_t 0.9043 satt 0.9040 | 0.95: pooled_t 0.9483 satt 0.9505 | 0.99: pooled_t 0.9867 satt 0.9892 | 0.998: pooled_t 0.9962 satt 0.9976
heldout gauss 0.9: pooled_t 0.8972 satt 0.9062 | 0.95: pooled_t 0.9498 satt 0.9559 | 0.99: pooled_t 0.9919 satt 0.9936 | 0.998: pooled_t 0.9983 satt 0.9990
heldout hetero 0.9: pooled_t 0.8791 satt 0.9020 | 0.95: pooled_t 0.9278 satt 0.9494 | 0.99: pooled_t 0.9756 satt 0.9873 | 0.998: pooled_t 0.9909 satt 0.9963
heldout heavy 0.9: pooled_t 0.9064 satt 0.9239 | 0.95: pooled_t 0.9525 satt 0.9657 | 0.99: pooled_t 0.9897 satt 0.9922 | 0.998: pooled_t 0.9966 satt 0.9973
heldout onewild 0.9: pooled_t 0.9305 satt 0.9132 | 0.95: pooled_t 0.9647 satt 0.9557 | 0.99: pooled_t 0.9921 satt 0.9904 | 0.998: pooled_t 0.9980 satt 0.9982
```

Script:

```python
"""Round-1 method checks (decision-method.md Appendix D), standard-library Python only. Synthetic data; reads no corpus or experiment data."""
import itertools, math, random

GVAL = [1] + [2] * 9 + [3] * 9 + [4]
GHEL = [1] + [2] * 4 + [3] * 10 + [4] * 4 + [6]
CONFS = (0.90, 0.95, 0.99, 0.998)


def betacf(a, b, x):
    qab, qap, qam = a + b, a + 1, a - 1
    c, d = 1.0, 1 - qab * x / qap
    d = 1 / (d if abs(d) > 1e-300 else 1e-300); h = d
    for m in range(1, 300):
        m2 = 2 * m
        aa = m * (b - m) * x / ((qam + m2) * (a + m2))
        d = 1 + aa * d; d = 1 / (d if abs(d) > 1e-300 else 1e-300)
        c = 1 + aa / c if abs(c) > 1e-300 else 1e300
        h *= d * c
        aa = -(a + m) * (qab + m) * x / ((a + m2) * (qap + m2))
        d = 1 + aa * d; d = 1 / (d if abs(d) > 1e-300 else 1e-300)
        c = 1 + aa / c if abs(c) > 1e-300 else 1e300
        de = d * c; h *= de
        if abs(de - 1) < 1e-14:
            break
    return h


def ibeta(a, b, x):
    if x <= 0: return 0.0
    if x >= 1: return 1.0
    bt = math.exp(math.lgamma(a + b) - math.lgamma(a) - math.lgamma(b) + a * math.log(x) + b * math.log(1 - x))
    return bt * betacf(a, b, x) / a if x < (a + 1) / (a + b + 2) else 1 - bt * betacf(b, a, 1 - x) / b


def t_cdf(t, df):
    x = df / (df + t * t)
    p = 0.5 * ibeta(df / 2, 0.5, x)
    return 1 - p if t > 0 else p


_TQ = {}
def t_ppf(p, df):
    key = (round(p, 12), round(df, 6))
    if key in _TQ: return _TQ[key]
    lo, hi = 0.0, 1e4
    for _ in range(200):
        mid = (lo + hi) / 2
        if t_cdf(mid, df) < p: lo = mid
        else: hi = mid
    _TQ[key] = (lo + hi) / 2
    return _TQ[key]


def binom_cdf(k, n):
    return sum(math.comb(n, i) for i in range(k + 1)) / 2 ** n


def draw(rng, G, model, sd=0.05, delta=0.0, fam_sd=0.03):
    ys, mus = [], []
    for f, n in enumerate(G):
        mu = delta + rng.gauss(0, fam_sd); mus.append(mu)
        if model == "hetero": s = sd * math.exp(rng.gauss(0, 0.75))
        elif model == "onewild": s = 5 * sd if f == len(G) - 2 else sd
        else: s = sd
        if model == "heavy":
            e = [rng.gauss(0, 1) / math.sqrt(rng.gammavariate(1.5, 2) / 3) / math.sqrt(3) for _ in range(n)]
        else:
            e = [rng.gauss(0, 1) for _ in range(n)]
        ys.append([mu + s * v for v in e])
    return ys, sum(mus) / len(mus)


def pooled_parts(ys):
    F = len(ys)
    means = [sum(y) / len(y) for y in ys]
    est = sum(means) / F
    df = sum(len(y) - 1 for y in ys)
    ss = sum(sum((v - m) ** 2 for v in y) for y, m in zip(ys, means))
    sp2 = ss / df
    se = math.sqrt(sp2 * sum(1 / len(y) for y in ys)) / F
    return est, se, df, sp2, means


def pooled_t(ys, conf):
    est, se, df, _, _ = pooled_parts(ys)
    t = t_ppf(0.5 + conf / 2, df)
    return est - t * se, est + t * se, est, se


def coverage(nsim=20000):
    print("## D.1 corpus-level interval coverage, nsim", nsim, "(covered at confidence c iff t_cdf(|est-truth|/se, df) <= 0.5 + c/2)")
    for name, G in (("validation", GVAL), ("heldout", GHEL)):
        for model in ("gauss", "hetero", "heavy", "onewild"):
            rng = random.Random(20260916)
            hit = {(c, k): 0 for c in CONFS for k in ("pooled_t", "satt")}
            for _ in range(nsim):
                ys, truth = draw(rng, G, model)
                est, se, df, sp2, means = pooled_parts(ys)
                p1 = t_cdf(abs(est - truth) / se, df)
                num = den = 0.0
                for y, m in zip(ys, means):
                    n = len(y)
                    if n >= 2:
                        v = sum((x - m) ** 2 for x in y) / (n - 1) / n; d = n - 1
                    else:
                        v = sp2 / n; d = df
                    num += v; den += v * v / d
                p2 = t_cdf(abs(est - truth) / (math.sqrt(num) / len(ys)), num * num / den)
                for c in CONFS:
                    hit[(c, "pooled_t")] += p1 <= 0.5 + c / 2
                    hit[(c, "satt")] += p2 <= 0.5 + c / 2
            print(name, model, " | ".join(f"{c}: pooled_t {hit[(c,'pooled_t')]/nsim:.4f} satt {hit[(c,'satt')]/nsim:.4f}" for c in CONFS), flush=True)


def heldout_replication(nsim=20000):
    print("## D.2 held-out replication (minimized metric, band b = ln 1.05, pooled_t two-sided 0.90), nsim", nsim)
    b = math.log(1.05); F = len(GHEL)
    for se_ratio in (0.25, 0.5, 0.8):
        sd = se_ratio * b * F / math.sqrt(sum(1 / n for n in GHEL))
        for eff in (0.0, -1.0, -2.0):
            rng = random.Random(11); old = new = att = 0
            for _ in range(nsim):
                ys, _ = draw(rng, GHEL, "gauss", sd=sd, delta=eff * b, fam_sd=0.0)
                lo, hi, est, se = pooled_t(ys, 0.90)
                old += est < 0
                new += (est < -b) and (hi < b)
                att += (est >= -b) and (hi < b)
            print(f"SE/b={se_ratio} true_effect={eff:+.0f} bands: old_R5c_pass {old/nsim:.4f} new_replicated {new/nsim:.4f} new_attenuated {att/nsim:.4f}")


def fpw(scores, w0, samples, conc, seed):
    k = len(w0)
    def winners(w):
        u = [sum(wi * s for wi, s in zip(w, row)) for row in scores]
        return u
    u0 = winners(w0); base = max(range(len(scores)), key=lambda i: u0[i])
    grid = keep = 0
    for combo in itertools.product((0.5, 0.75, 1.0, 1.25, 1.5), repeat=k):
        w = [c * x for c, x in zip(combo, w0)]; s = sum(w); w = [x / s for x in w]
        u = winners(w); grid += 1; keep += u[base] >= max(u) - 1e-12
    rng = random.Random(seed); dk = 0
    for _ in range(samples):
        g = [rng.gammavariate(conc * k * x, 1) for x in w0]; s = sum(g); w = [x / s for x in g]
        u = winners(w); dk += u[base] >= max(u) - 1e-12
    return base, u0, keep / grid, dk / samples


def ties():
    print("## D.3 sensitivity under ties and sub-band deficits (grid factors 0.5..1.5 full, Dirichlet c=4, 10000 samples)")
    w = [0.2] * 5
    q = [[0.0] * 5 for _ in range(3)]
    base, u, g, d = fpw([[-x for x in r] for r in q], w, 10000, 4.0, 1)
    print("all-within-one-band (quantized): FPW grid", g, "dirichlet", d, "argmax base", base,
          "| R10-eligible tie set size", sum(all(math.floor(x) == 0 for x in r) for r in q))
    qc = [[0, 0.9, 0.9, 0.9, 0.9], [1.5, 0, 0, 0, 0], [1.2, 0.5, 0.5, 0, 0]]
    for label, qq in (("quantized", [[math.floor(x) for x in r] for r in qc]), ("continuous", qc)):
        base, u, g, d = fpw([[-x for x in r] for r in qq], w, 10000, 4.0, 1)
        print(label, "utilities", [round(x, 3) for x in u], "winner", base, "FPW grid", round(g, 4), "dirichlet", round(d, 4))


def arithmetic():
    print("## D.4 exact order-statistic median CI: n_min and (rank k, coverage) for interval x_(k)..x_(n+1-k)")
    for conf in (0.90, 0.99, 0.995, 0.998, 0.999):
        nmin = min(n for n in range(1, 64) if 1 - 2 * 0.5 ** n >= conf)
        best = {}
        for n in (8, 10, 11, 15, 20, 30):
            ks = [k for k in range(1, n // 2 + 1) if 1 - 2 * binom_cdf(k - 1, n) >= conf]
            best[n] = (max(ks), round(1 - 2 * binom_cdf(max(ks) - 1, n), 5)) if ks else None
        print(conf, "n_min", nmin, best)
    print("## D.5 detection bounds: 20 stress runs detect per-run mismatch probability >=", round(1 - 0.05 ** (1 / 20), 4),
          "with 95%; 10000 trials (rule of three) >=", 3 / 10000, "; 1000 injections >=", 3 / 1000)
    print("## D.6 log-consistent tier bands (1+r)^k and (1+r)^-k")
    for r in (0.01, 0.05, 0.10, 0.25, 0.5):
        print(r, [(k, round((1 + r) ** k, 10), round((1 + r) ** -k, 10)) for k in (1, 2, 3, 5, 10)])
    print("## D.7 held-out MDE and null pass: SE/b at MDE = 2 bands (one-sided 0.05, power 0.80, normal) =",
          round(2 / (1.6448536 + 0.8416212), 4))


if __name__ == "__main__":
    import sys
    part = sys.argv[1] if len(sys.argv) > 1 else "all"
    if part in ("all", "arith"): arithmetic(); ties()
    if part in ("all", "rep"): heldout_replication()
    if part in ("all", "cov"): coverage()
```

---

## Revision history

| Date | Version | Change |
|---|---|---|
| 2026-09-16 | round 0 (pre-registration draft) | Initial draft written before any decision-relevant experiment result existed. |
| 2026-09-16 | round-1 revision (pre-registered) | Revision against `research/methods/method-review-round1.md` (85 findings). Main changes: single normative procedure with stated cost entry points (§2.0); constraint crosswalk and `hc_ids` (R1); freeze exception deleted and emission separated from decode support (R1, §7.2, §7.4); gated library-version decode steps (R2); SPEC §5.4 constrained form for profile decisions and a cited family mix (§2.0, §4.9, Appendix B); one primary metric per OD and per-OD weights (R3, §6.3); non-inferiority conditions, confirmatory set and intersection-union multiplicity (R4, §4.11, §4.12); exact order-statistic item intervals and Welch-Satterthwaite corpus intervals verified by simulation (§4.10, Appendix D); held-out replication as non-inferiority plus favourable point estimate, with held-out MDE gate (R5); blocks separated from soft caps, HC verification condition, gate audit (§8); identity-aware held-out threat model with sealing route, single program-wide freeze, look registry, content-overlap audit (§5); A/A′, known-effect and background-load calibration, frequency covariate, tighter guard (§4.2-§4.7); continuous band units with explicit R10 ties, selection bootstrap, extended mix arms (§6); log-consistent minimum gains with caps and computed tiers (§3.1, §7); canonical metric registry with roles, and canonical parameters (Appendices A, C); `thresholds.json` transcribed. No decision-relevant result existed. |
| 2026-09-16 | round-1 revision, completed (pre-registered) | The round-1 revision was interrupted by a host storage incident and checkpointed unfinished as WIP commit `3fda801` (§0.1); a resumed session completed and verified it. **Verification:** every input SHA-256 of §0.1 re-checked; the Appendix D script re-run with output identical to the recorded block; the objective Appendix A check and the extended Appendix A.3 check re-run with no differences; ebr v1 loads `thresholds.json` with the values stated in Appendix A.1 (WSL research venv); ebr suite status recorded in §14 item 1. **Changes:** §0.1 (repository history including the WIP commit; third identity-exposure disclosure; test file input); §3.7 and Appendix A.1 (`decision_rules` block; pre-registration commit resolved by subject, because the WIP commit already carried the status label); `thresholds.json` extension keys completing the transcription of §4, §5.2-§5.3, §6, §7.3, §8.4, §10.2 and the numeric parameters of Appendix C.1; Appendix A.2 (`encode_wall_s` and `decode_wall_s` also implement objective M15.3); Appendix A.3 extended to the objective's ebr-family brackets, the §3.7 values and every source pointer; Appendix C.1 bounded-memory probe sizes; §5.4 and §5.8 (h) for the third exposure; §14 item 1. In `archetypal-objective.md`: ebr-family brackets aligned with Appendix A.2, absolute-only aggregation under AGG-G, M08.2, M15.3, M16.3 and M19.5 definitions, §4.2 table order, §4.5 count, threat 7. No rule, threshold, confidence level or stability bar changed. No decision-relevant result existed. |

### Round-1 dispositions

Commit: the commit with subject "research: pre-register decision method and archetypal objective". `FIXED` rows cite the sections that fix the finding in this document ("obj." = `archetypal-objective.md`). Where a fix defines a rule whose artifact does not exist yet, the artifact is an execution gate (§0.4, §14), and the row says so. `REJECTED` and `ACCEPTED_RISK` rows, and partial rejections, are argued in the review file's "Review disposition" section.

| ID | Sev. | Disposition | Where |
|---|---|---|---|
| A01 | BLOCKER | FIXED | §2.0 (single procedure; cost entry points; CC coverage as report); obj. §1.2 replaced by pointer; obj. Companion precedence covers procedure and rule order |
| A02 | BLOCKER | FIXED | R1 (I24 statistical test deleted; mechanical facts); T-17 (graded per padding mode, not decision-grade until attack suite committed, external review); obj. HC-14 statement, MVT-14(e) chance rate, MVT-14(j); obj. OD-05 note, OD-16; §14 item 16 |
| A03 | BLOCKER | FIXED | R2 (gated library-version-defined steps admitted under conditions, T4, never baseline; default-path use is a defect with R0 item 7 candidate); obj. HC-11 statement and status, MVT-12(c) |
| A04 | BLOCKER | FIXED | R1 (no freeze exception; in-place change L0-excluded; decode-support removal escalated); §7.2 T4; §7.4 rewritten; obj. HC-16 statement, §1.4, §4.4, §4.5 |
| A05 | HIGH | FIXED | §2.0 and Appendix B.1 (constrained form, no weights); §4.9 and Appendix B.3 (cited mix base, equal arm); ledger notes on DEC-CMP-035 at §14 item 12 (gate G-A) |
| A06 | HIGH | FIXED | Appendix C.1 (canonical parameters); Appendix A.2 (M-number, canonical name, threshold ID, family, role), checked by A.3, which also checks the objective's ebr-family brackets and every `thresholds.json` source pointer; obj. §1.4 parameters clause, OD-06, OD-12, OD-13, OD-14, MVT-11(a), MVT-13(a) aligned |
| A07 | HIGH | FIXED | §4.2 hybrid-confound exception; T-14; §4.7 speedup A/A; obj. OD-13 |
| B01 | BLOCKER | FIXED (rule; artifact gate G-A) | R1 crosswalk classes and screening rule; obj. §2.0 rule 7; obj. Appendix B F-28, F-29; §14 items 10-12 |
| B02 | HIGH | FIXED | obj. MVT-07(d), HC-07 summary row, M06.5 |
| B03 | HIGH | FIXED | obj. MVT-06(d), M17.5, M24.5; R2 |
| B04 | HIGH | FIXED | obj. MVT-06(a) allocator oracle, R0_max, H before first record else HC_UNVERIFIED; §4.14; §14 item 17 |
| B05 | HIGH | FIXED | obj. §2.0 rule 2 nondeterminism artifact rule; obj. MVT-13(a) stress mode and bound; §4.13; Appendix D.5 |
| B06 | HIGH | FIXED | obj. MVT-05(a) independent Windows oracle; MVT-05(b) deterministic interleaving, random trials as regression evidence, NTFS symlink capability |
| B07 | HIGH | FIXED | obj. §2.0 rule 8 triage rule; MVT-03(b), MVT-12(a) |
| B08 | HIGH | FIXED | obj. MVT-12(b) normative grammar and recall ≥ 0.95 validation |
| B09 | MEDIUM | FIXED | obj. MVT-13(a) determinism input, per-host-class PCI, cross-host LAI plus explained AUX |
| B10 | MEDIUM | FIXED | obj. MVT-11(a) locale canary, Windows HC_UNVERIFIED, QEMU CPU models, memory limit definition, OOM classification |
| B11 | MEDIUM | FIXED | obj. MVT-02(e) path coverage (N = 5) and mutation adequacy |
| B12 | MEDIUM | FIXED | obj. MVT-16(c); obj. §2.0 rule 9 |
| B13 | MEDIUM | FIXED | obj. MVT-15(b) precedence table |
| B14 | MEDIUM | FIXED | obj. MVT-14(f)-(i) |
| B15 | MEDIUM | FIXED | obj. MVT-16(d) |
| B16 | MEDIUM | FIXED | obj. HC-18 and MVT-18; §3.3; R1 |
| B17 | MEDIUM | FIXED | obj. MVT-10(d) provenance stripped, table-cell citation |
| B18 | LOW | FIXED | obj. detection bounds or regression labels in MVT-03(b), MVT-06(a), MVT-14(b), MVT-14(c), MVT-15(b) |
| B19 | MEDIUM | FIXED | obj. §2.0 rule 3; R1(b); §8.2 item 2 |
| C01 | MEDIUM | FIXED | obj. Appendix A regenerated from the §2.1 column with roles, embedded check (no differences) |
| C02 | MEDIUM | FIXED | obj. MVT-01(d) (I10), MVT-04 items 8-10 (I11), MVT-10(e) (I7), MVT-06(a) count disagreement (I20), MVT-10(f) (I22), MVT-17(d) (I23) |
| C03 | LOW | ACCEPTED_RISK | obj. Appendix C limitation note; §14 item 11 |
| D01 | BLOCKER | FIXED | Appendix A.2 (every M-number has a canonical name and role); §0.2 item 4 (late bands secondary forever); `thresholds.json` `metric_thresholds` |
| D02 | HIGH | FIXED | Appendix A.2 roles; obj. M01.1, M04.3, M21.2, M21.3, M22.4, M23.1, M24.2 |
| D03 | HIGH | FIXED | obj. OD-03 aggregation (fixed class list, undeclared non-restoration counts 0) |
| D04 | HIGH | FIXED | obj. M08.3 redefined; Appendix C.1 |
| D05 | MEDIUM | FIXED | obj. M05.3, M10.2, M14.1, M15.2, M16.2, M19.2, M25.1, M26.1; T-15, T-17, T-22 |
| D06 | HIGH | FIXED | obj. §1.3 (specialist baseline per OD, SPEC §23.4 tolerances, corrected OD assignments, M26.5, M10.5, default count as report); §10.1 item 5; §14 item 19 |
| D07 | MEDIUM | FIXED | obj. M17.6 (scale limits), M05.5 (byte-weighted), M21.6 (reader fragmentation), HC-18 (crash consistency) |
| D08 | MEDIUM | FIXED | R4 condition 3 and R8 per tier; §4.9 strata; §6.5 byte-weighted and tier arms |
| D09 | MEDIUM | FIXED | R3 (one primary metric per OD, OD set assigned); §6.2, §6.3 per-OD weights |
| E01 | HIGH | FIXED | T-03, T-04 small-tier rule; T-08 in-process or batch only; §3.1; obj. OD-07, OD-10 |
| E02 | HIGH | FIXED | T-15 (r = 0.5, 64 KiB, region strata, worst-stratum guard) |
| E03 | HIGH | FIXED | §3.1 log-consistent multipliers; §7.3 recomputed table, absolute caps, feasibility check; Appendix D.6 |
| E04 | MEDIUM | FIXED | T-01 small tier relative-only; T-21 |
| E05 | MEDIUM | FIXED | T-09 and T-11 floors aligned at 16 KiB; T-10, T-11 and §4.15 modelled latency |
| E06 | MEDIUM | FIXED | T-07; §3.3; §4.14 write accounting |
| E07 | MEDIUM | FIXED | T-18 heuristic; §4.16 |
| E08 | LOW | FIXED | §4.7 criterion 2 (at most 1 item) plus known-effect calibration |
| E09 | MEDIUM | FIXED | R9 (iii) per-metric bound 1.0 band unit, cap, T3/T4 owner acceptance |
| F01 | BLOCKER | FIXED (acceptance criterion partly REJECTED: over-coverage allowed) | §4.10 Welch-Satterthwaite corpus interval, single-group handling, pre-registered coverage criterion; Appendix D.1 |
| F02 | HIGH | FIXED | §4.10 exact order-statistic item interval; §4.6 n_min; §4.12 confirmatory set and MDE gate |
| F03 | HIGH | FIXED | §4.7 A/A′, known effect, background load |
| F04 | HIGH | FIXED | §4.2 canary on own affinity set at 1.025, frequency covariate, throttling onset and straddle flag |
| F05 | HIGH | FIXED | §4.4 system 3.0%, pinned set and siblings 2.0%, WSL load 1.0 + n, covariates, injected-load validation |
| F06 | HIGH | FIXED | §4.1 VIRTUALIZED qualifier and cap, same-direction rule, no cold-cache claims; §4.3 host sampler; §4.5; §9.1 |
| F07 | HIGH | FIXED | §4.2 Defender row (in-process timers, owner-excluded directory run, covariates); §4.7 A/A′ |
| F08 | HIGH | FIXED (family-level non-inferiority partly REJECTED: harm guard instead) | §4.12 confirmatory set, Bonferroni/Holm for superiority, intersection-union for the rest, BY listing, rule simulation; R4 conditions 2-3; §4.9 family harm guard |
| F09 | MEDIUM | FIXED | §3.1 floors and aggregation |
| F10 | MEDIUM | FIXED | §4.15 calibration criteria and connection model |
| G01 | BLOCKER | FIXED (option (b) adopted, option (a) as cap-lifting route; execution items gate G-C) | §5.4 threat model, consequences, sealing route, transcript audit, role separation; §5.7 L7; §5.8; §0.1 disclosures (three sessions); §14 item 15 |
| G02 | HIGH | FIXED (rule; audit gate G-C) | §5.3 lineage grouping and content-overlap audit (≤ 0.01); §5.7 L6 |
| G03 | HIGH | FIXED | §5.5 single program-wide freeze and unlock |
| G04 | HIGH | FIXED | §5.2 second look on new items or labelled tuning; program-wide look registry and cross-decision rule |
| G05 | HIGH | FIXED | §5.6; §8.4 blocks |
| G06 | MEDIUM | FIXED | §5.5 Commit A and Commit B contents |
| G07 | MEDIUM | FIXED (rule; execution gate G-C) | §5.4 materialization rule; §5.5 item 4 |
| G08 | LOW | FIXED | §5.3 replacement criteria |
| H01 | HIGH | FIXED | §6.2 continuous band units, R10 tie screen, "no measured winner", explicit selection; Appendix D.3 |
| H02 | HIGH | FIXED | §6.7 selection-stability bootstrap; §6.6 bar |
| H03 | MEDIUM | FIXED | §6.3 SMAA flat-simplex and vertex report |
| H04 | MEDIUM | FIXED | §6.4 cap plus `band_dependent` flag; resolvability condition for ×0.5 |
| H05 | MEDIUM | FIXED | §6.5 leave-three-families-out, leave-one-mix-out, tier, byte-weighted, environment and reference arms |
| I01 | HIGH | FIXED | §7.1 C1-C7 extensions; §7.2 tiers (identity participation T4, library-version decode T4, planner or chunker ID T1 with vectors, reason codes T2); §7.4 |
| I02 | HIGH | FIXED | §7.2 computed tiers, independent C4-C7 assessment, disputes to higher tier; §14 item 7 |
| I03 | MEDIUM | FIXED | R6 exemption |
| I04 | MEDIUM | FIXED | §7.6 crosswalk; obj. §4.4 |
| I05 | LOW | FIXED | §7.5 copyleft scope |
| J01 | BLOCKER | FIXED | §8.2 item 13; §8.4 blocks (PLATFORM_BLOCKED cap removed); obj. §2.0 rule 4 ("or property" deleted), §4.6 |
| J02 | HIGH | FIXED | R5 (c)-(e) and held-out MDE gate; Appendix D.2 |
| J03 | HIGH | FIXED | §8.4 blocks versus soft caps; T3/T4 need HIGH or owner acceptance (§8.2 item 18) |
| J04 | HIGH | FIXED | §1.2 assignment, forced types, FORMAL protocol; §8.2 item 14; §14 item 12 |
| J05 | HIGH | FIXED | R10 power requirement; §10.2 CB-1 counts every R10 selection under `INCONCLUSIVE` |
| J06 | MEDIUM | FIXED | §8.6 gate audit; §8.2 item 5 adversarial-search artifact; §0.2 item 2 amendment requirements |
| J07 | MEDIUM | FIXED | §1.2 (34 decisions listed); §4.16; §8.3 |
| K01 | BLOCKER | FIXED (schema defined; artifact gate G-A) | §14 item 12 (schema v2 fields, reviewed assignment, validator); R3; §8.2 items 13-15 |
| K02 | MEDIUM | FIXED (named scenarios run; full simulation remains §14 item 9) | Appendix D (F01 coverage, H01 ties, J02 null pass); §14 item 9 |
| K03 | LOW | FIXED | `thresholds.json` `status` and `pre_registration` (commit resolved by subject; the unfinished WIP checkpoint `3fda801` carried the label early and is disclosed in §0.1); Appendix A.1 |
