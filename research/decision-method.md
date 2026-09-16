# Entrybound Decision Method

| Field | Value |
|---|---|
| Document | `research/decision-method.md` |
| Status | **PRE-REGISTRATION DRAFT, round 0.** Adversarial review (`research/methods/method-review-round1.md`) and revision are still to come. The commit of the revised document is the pre-registration record (program §3.3, §37; REQ-ECO-0185). |
| Authority | This document is authoritative for every decision threshold, statistical rule, status rule and cost rule in the research program. `research/methods/thresholds.json` is a mechanical transcription of it (§3.7, Appendix A). Where the two disagree, this document governs and the JSON is corrected. |
| Companion | `research/archetypal-objective.md` defines the hard constraints (HC-xx) and optimization dimensions (OD-xx). This document defines how evidence about them becomes a decision. |
| Ledger decisions addressed | DEC-ECO-078 (selection rule), DEC-ECO-082 (measurement protocol), DEC-ECO-076 (post-unlock policy; default only), DEC-ECO-080 (negative results; minimum obligation), DEC-ECO-083 (usability substitute; evidence cap only). These rows stay `INSUFFICIENT_EVIDENCE` until their own required evidence exists: method review, rule simulation on synthetic and pilot tables, and A/A calibration. Until then this document binds the program. |

The key words MUST, MUST NOT, SHOULD and MAY are used as in RFC 2119.

---

## 0. Pre-registration record

### 0.1 State of evidence at writing (2026-09-16)

- Repository HEAD: `f56dd3d6d32a2b0123ec784bdb598a1b84da0ce3` ("research: assemble Phase A ledgers round 1 with critic addenda"). No uncommitted changes under `research/`.
- `research/raw/` and `research/normalized/` contain only `EXP-SMOKE-000`. That is a pipeline smoke test (gzip levels on one generated 1 MiB file, `decision_ids: []`). Its spec declares its `thresholds_override` values meaningless, and they were not used here.
- No experiment with non-empty `decision_ids` has run. No held-out item has been read.
- Decision ledger: 593 rows, of which 577 are `INSUFFICIENT_EVIDENCE`, 16 are `EXTERNAL_REVIEW_REQUIRED` and none are `DECIDED`. Command: `python -c "import json,collections;print(collections.Counter(json.loads(l)['status'] for l in open('research/decision-ledger.jsonl',encoding='utf-8')))"`.
- Not consulted when setting any threshold: corpus indicator statistics (`research/corpus/statistics.json`), Research III instrumentation measurements, and published incumbent benchmark figures. Each value derives from the reasoning stated beside it, from the SPEC, and from the externally sourced facts named inline.
- Inputs read, with SHA-256:

| Input | SHA-256 |
|---|---|
| SPEC `design/2026-08-29-entrybound-product-architecture.md` (§3.9, §5.4, §5.7, §5.8, §6, §22.1) | `1f881c7b13f193b41353b90066d33ca6abcb7d4c3dcd4f58e22834801f0fe29c` |
| `research/decision-ledger.jsonl` | `98b5401d79c621fab8db2a72e4825dcef9d74dc60de042fdb3990c46daa4068b` |
| `research/methods/thresholds.json` (null template) | `56c6cbb8c8a0422afe6d0761d372826ae662df511731d3fe9b89c2ea2141fd31` |
| `research/tools/ebr/stats.py` | `309c92ac511b27bb27515bd76f498b6fb68e72ad81e0fa7d2c5d094009955f25` |
| `research/tools/ebr/thresholds.py` | `c75733409022c0135c8971a9f00c70734e919e72032ce65590f9de0ab986ce31` |
| `research/corpus/heldout-lock.json` (status `draft`, 65 items) | `df7331c1d1f2b486986a7f273dd694aaa2c3c6985ad0c952440b80397dc3beb5` |
| `research/corpus/manifest.json` | `8c4e4cc61079ba8c34cafd468f549441d9cb296b53f1957b6346363988b5c494` |

- The revision commit MUST re-verify this state (no raw results with non-empty `decision_ids`, held-out unread) and restate it in its commit body. Incumbent-only baseline runs (for example `EXP-BASE-SIZE`, planned in Phase B1) are not decision experiments. However, any threshold change made after their outputs exist MUST be declared as made with those outputs visible.

### 0.2 Amendment policy

1. **Before any decision-relevant result exists:** changes are made by revision (review rounds) and recorded in [Revision history](#revision-history).
2. **After a decision-relevant result exists for a metric:** that metric's band, absolute floor, epsilon, confidence level, stability bar, minimum-gain multiplier and guard limit MUST NOT change, except to correct a transcription error against the pre-registered text.
   - Any other amendment, such as fixing a statistical defect, is logged as a deviation. The log records date, author, whether results had been seen, and the reason.
   - Every affected decision is analysed under both the original and the amended method, and both analyses are reported.
   - If the two disagree, the decision's confidence cannot exceed MEDIUM. The original rule governs unless the amendment corrects a demonstrable error (a failing test or a counterexample).
3. **Protocol controls that do not decide outcomes** (repetition counts, warmups, affinity, cooldown, canary cadence) MAY be strengthened after A/A calibration (§4.7). They MUST NOT be weakened once decision-relevant results exist.

### 0.3 Tags

Material claims carry evidence tags from §9.1, such as `[FORMALLY_DERIVED]` and `[INFERRED]`. An untagged normative statement is a method rule: a choice this pre-registration makes, not an empirical claim.

---

## 1. Terms and decision types

| Term | Meaning |
|---|---|
| Candidate | One fully specified design alternative for one ledger decision (`candidate_set[].candidate_id`). A changed design is a new candidate id, for example with a version suffix. |
| Scope | The population a selection applies to. Hierarchy: universal, then profile, policy, workload scenario, family (§2, R9). |
| Primary metric | Declared in the experiment record before execution (§12). Only primary metrics can eliminate or select a candidate. |
| Secondary metric | Reported, and able to trigger review. Cannot eliminate or select. |
| Diagnostic metric | A decomposition, for example bytes per section, used to explain primary results. Never double-counted as an objective. |
| Band | The practical-significance band of a metric. It has a relative half-width `r` (ratio band `[1/(1+r), 1+r]`), an absolute threshold `a` (difference band `[-a, +a]`), or both (§3.1). |
| Unit of analysis | Levels: sample, item, independence group, family, corpus (§4.9). |
| Verdict classes | `DISTINGUISHABLE_BETTER`, `DISTINGUISHABLE_WORSE`, `EQUIVALENT`, `INCONCLUSIVE` (§4.11). ebr's `indistinguishable` is `EQUIVALENT` or `INCONCLUSIVE`. |
| Decision-grade | A verdict from runs and analyses that meet every requirement of §4 and §14. Anything else is informational. |
| Cost tier | T0 to T4, the permanent-cost class of a candidate (§7.2). |
| Reference candidate | The comparison anchor declared in the experiment record (ebr `analysis.baseline`), normally the status quo. |

| Decision type | Evidence route | Held-out validation (R5) |
|---|---|---|
| EMPIRICAL | Measured experiments under §4 | Required |
| FORMAL | Derivation from invariants, SPEC, standards or exact vectors, checked by a reviewer who did not produce it, plus a recorded counterexample search | Not applicable; the reason is recorded |
| EXTERNAL | Dossier plus independent expert review (§8.3) | Per the empirical parts, if any |
| HUMAN-FACING | Simulated or heuristic evaluation under §4.16, with capped evidence | Required, on a held-out task set |

A decision that mixes types MUST satisfy the route of every type it contains.

---

## 2. Decision rule

**Precedence is lexicographic.** A later rule never rescues a candidate eliminated by an earlier one. Every elimination records the rule id, the evidence reference, and, for R1, the violated hard constraint and its mechanical test. The rule order is the logical order; §2.11 maps it onto the program timeline.

### R0. Candidate completeness (precondition)

A candidate set is complete only if it contains all of the following:

1. The status quo: behaviour at the research baseline `9e44608`, or the currently frozen decision.
2. The SPEC-proposed design.
3. Every alternative named for the question in the source ledgers (R1–R3, APPX, repo docs, code).
4. The strongest incumbent technique for the capability (SPEC §22), where one exists.
5. "Defer / not in v1", where release relevance permits.
6. At least one candidate proposed by a critic not involved in designing the others.

Completeness is checked by a critic pass recorded in the ledger `history`. Adding a candidate after results exist is allowed, but the added candidate enters at R1 and the addition is logged.

### R1. Eliminate hard-constraint violations

A candidate is eliminated if it violates any hard constraint in `archetypal-objective.md`. Those constraints map invariants I1–I31 (SPEC §3.9) and the repository freezes recorded in the ledger `hard_constraints`: crypto-v1 wire frozen, planner IDs frozen, workspace `unsafe_code = "forbid"`, frozen legacy export profiles, historical Entry-v1 identity bytes, and the others.

- **Test.** A single counterexample suffices; no statistics are involved. A violation is either:
  - (a) the hard constraint's mechanical test failing on any available item of any split; or
  - (b) a formal argument that the design admits a violating archive or state.
- **Binary failures are R1, not trade-offs:**
  - correctness failures: round-trip mismatch, or unverifiable output presented as verified (I25);
  - failure to detect truncation or corruption (I21);
  - nondeterminism where determinism is claimed (§4.13);
  - plaintext-structure leakage through chunk boundaries (I24, §3.2 T-17).
- **Freeze exception.** When the decision question is whether to revise a particular freeze, that freeze is not an R1 screen for that decision. Invariants I1–I31 always are.
- **Bugs.** A candidate eliminated for an implementation bug that is fixable without changing the design MAY re-enter as a new candidate version. It restarts at R1 on tuning and validation, and the old elimination stays recorded.

### R2. Eliminate unboundable or unspecifiable designs

- **Unboundable.** Some resource needed to decode, verify, extract or randomly access the archive has no bound computable before payload from declared parameters (I17, I19, I20, SPEC §5.4, §6.0a). Resources include memory, codec window, scratch, decoded bytes per accessed byte, lookback depth, request count, and CPU work per input byte.
  - Test: a written bound formula exists, and measured values on every available item, including family F20 adversarial items, never exceed the declared bound.
  - One exceedance eliminates the (candidate, bound) pair. A revised bound is a new candidate version.
- **Unspecifiable.** Decoder behaviour cannot be stated normatively independent of one implementation (SPEC §5.8), or decoding depends on something other than declared data: floating-point evaluation, thread count, iteration order, environment, or library version (SPEC §5.7, I15, I30).
  - Test: a normative description exists and has been reviewed by a reader not involved in the candidate; every decode-affecting computation is integer or fixed-point; every parameter is recorded as data.
- **Profile bounds.** Once DEC-CMP-035 fixes numeric profile values, a candidate that exceeds a profile's declared bounds is eliminated for that profile scope only. Until then, declared requirements are reported but not screened.

### R3. Measure survivors

Survivors are measured on their primary metrics under the §4 protocol: on the tuning split for search, and on the validation split for confirmation.

- The experiment record (§12) MUST be committed before execution. It declares the primary metrics, reference candidate, primary comparisons (§4.12), base weights (§6.3), scope and applicable families.
- An optimization dimension that `archetypal-objective.md` assigns to the decision but that goes unmeasured is listed as a gap. A decision with an unmeasured primary dimension cannot be `DECIDED`.

### R4. Eliminate epsilon-dominated candidates

Within a scope, candidate A eliminates candidate B if and only if all of the following hold on validation-split results:

1. A is `DISTINGUISHABLE_BETTER` than B on at least one primary metric (§4.11, including absolute floors).
2. A is not `DISTINGUISHABLE_WORSE` than B on any primary metric.
3. On every primary metric, A's point estimate is better than B's or within epsilon of it. The epsilon tolerance is `max(r*|B|, a)` for that metric (§3.1).
4. A's provisional cost tier is no higher than B's. Cost is an objective, and measured dominance cannot override it (§7).
5. For the universal scope, conditions 1–3 hold at corpus level (§4.9) and condition 2 holds in every applicable family under the unanimity rule.

The survivor set is the set of candidates not eliminated by any other candidate. This matches `stats.pareto_frontier`: epsilon-dominance is not transitive, and both members of a tie survive.

### R5. Validate on held-out

Held-out results are obtained once, under the frozen design (§5.5). **Every candidate that reached the freeze is run, not only the provisional winner.** Validation passes if and only if:

- (a) no R1 or R2 failure occurs on any held-out item;
- (b) the provisionally selected candidate is not eliminated when R4 is applied to held-out results;
- (c) every corpus-level `DISTINGUISHABLE` verdict supporting the selection replicates in direction. The held-out point estimate lies on the same side of the band centre, and the held-out verdict is not `DISTINGUISHABLE` in the opposite direction. If the held-out point estimate falls inside the band, the verdict is **attenuated** (confidence at most MEDIUM).
- (d) no `EQUIVALENT` verdict used by R10 is `DISTINGUISHABLE` on held-out in the direction unfavourable to the selected candidate;
- (e) no family in scope shows a unanimous held-out reversal, meaning every held-out group of that family lies beyond the band in the unfavourable direction;
- (f) the R7 bars hold when re-evaluated on held-out results (§6.6).

On failure the decision does not become `DECIDED`. The held-out results are recorded as counterevidence, and re-selection requires fresh confirmatory data (§5.6). FORMAL and EXTERNAL decision types skip R5 and record why.

### R6. Account permanent cost

Each survivor receives its final cost record and tier (§7).

- **Minimum gain.** Let L be the best surviving candidate of a lower tier. A higher-tier candidate H is eliminated unless both of the following hold:
  - H's advantage over L on at least one primary metric is `DISTINGUISHABLE` against the band multiplied by H's tier multiplier (§7.3);
  - H is not `DISTINGUISHABLE_WORSE` than L on any primary metric.
- **Invariant-required features.** If every lower-tier candidate was eliminated at R1 or R2, the multiplier does not apply, and the lowest-tier constraint-satisfying candidate stands.
- **Tier changes.** If a final tier differs from its provisional tier, R4 is re-run.

### R7. Sensitivity analysis

Perturbations of weights, thresholds and workload mix are applied under §6. The result determines the broadest scope at which a winner is supported.

### R8. Universal default only when supported

A single candidate c* becomes the universal default (the v1 behaviour, the frozen parameter, the default policy) only if it meets all of these:

1. It survives R1–R6 in the universal scope.
2. It is the band-unit utility winner (§6.2) at corpus level under the base weights. Ties are resolved by R10.
3. It is not `DISTINGUISHABLE_WORSE` than any survivor in any applicable family (unanimity rule, §4.9).
4. It passes every R7 stability test at the pass bar (§6.6).
5. It passes R5, where R5 applies.

### R9. Otherwise, profile-, policy- or workload-specific winners

If R8 fails, select scoped winners using the broadest partition of the scope in which every part has a candidate meeting the R8 conditions restricted to that part. Partitions are tried in this order:

1. Profile: fast, balanced, dense, extreme.
2. Policy: STREAM or INDEXED layout, encrypted or plain, deterministic or adaptive, local or remote access.
3. Workload scenario (Appendix B).
4. Family.

Constraints on the partition:

- **(i) Expressibility.** The product must be able to express the partition at the time of the decision. For example, per-family selection exists only where the profile categorizes content, and `fast` performs no deep categorization (SPEC §6.1). Winners at a scope the product cannot express are recorded as limitations and negative results.
- **(ii) Cost.** A partition into scoped winners is itself a cost: at least T1 for a planner or policy split, and higher if it needs wire or user-visible changes. Each scoped winner must beat, within its part, the best single candidate for the whole scope by the multiplier of that tier (§7.3).
- **(iii) Compromise.** If no partition satisfies (i) and (ii), select the surviving candidate that minimizes the maximum band-unit regret across the parts (§6.2). This is accepted only if that maximum regret is at most 2.0 band units, and it caps confidence at MEDIUM. Otherwise the decision stays `INSUFFICIENT_EVIDENCE` and the non-dominated frontier is published.

### R10. Prefer the simpler candidate when differences are negligible

When candidates in a scope are not distinguishable on any primary metric (every comparison `EQUIVALENT`, or `INCONCLUSIVE` at the repetition cap), choose by these keys in order:

1. Lower final cost tier.
2. Fewer normative spec words plus wire fields added (§7.1, C1).
3. Fewer reader lines of code (C2).
4. Status quo.
5. Lexicographically smallest `candidate_id`, recorded as a deterministic tie-break.

The burden of proof lies on the more complex candidate. Status quo ranks below cost because pre-v1 wire gets no sunk-cost credit (§7.4). If any comparison relied on here is `INCONCLUSIVE` rather than `EQUIVALENT`, the ledger records "simplicity default under inconclusive evidence" and confidence is capped at MEDIUM.

### 2.11 Timeline mapping

| Program phase | Split | Rules applied | Ledger state afterwards |
|---|---|---|---|
| C-exec, tuning | tuning | R0–R3: search, parameter sweeps, candidate refinement | unchanged |
| C-exec, validation | validation (§5.2 budget) | R1–R4, R6, R7 on validation, R8–R10: provisional selection | `INSUFFICIENT_EVIDENCE`, `selected_decision` marked `provisional`, confidence recorded |
| D, freeze | none | Design-freeze commit; held-out specs committed (§5.5) | frozen provisional selections |
| D, held-out | heldout, once | R1, R2, R4, R5, R7 re-run; **no re-selection** | per §8 |
| E, synthesis | none | Status rules §8, negative results §10 | `DECIDED` / `INSUFFICIENT_EVIDENCE` / `EXTERNAL_REVIEW_REQUIRED` |

"Rule followed" means that the ledger `history` entry cites the commit SHA of `decision-method.md` that governed the decision, records the outcome of each of R0–R10, and lists every deviation.

---

## 3. Practical-significance thresholds

### 3.1 Semantics

- **Relative band.** For a paired ratio `R = candidate / reference`, the band is `[1/(1+r), 1+r]`. The band is symmetric in log space, so swapping candidate and reference swaps sides exactly. This matches ebr's epsilon semantics (`log1p(r)`, `stats.eps_dominates`).
- **Absolute threshold.** For a paired difference `D = candidate - reference`, in metric units, the band is `[-a, +a]`.
- **Composite rule.** When a metric has both `r` and `a`, a difference is practically significant only if it exceeds both.
  - Item level: a verdict is `DISTINGUISHABLE` only if the ratio CI and the difference CI both lie wholly outside their bands, on the same side.
  - Aggregation (floor snapping): if an item's median paired `|D| <= a`, that item's ratio is set to exactly 1 before aggregating to group, family and corpus.
- **Metrics with only `a`** (bits, percentage points, fractions, counts where noted) use the difference band at every level. Differences are aggregated by arithmetic mean.
- **Edge rule.** A CI that touches a band edge overlaps the band (`stats.noise_rule`).
- **Integer counts.** Floors on integer metrics are set at 0.5 so that a difference of one unit exceeds the band. A floor of exactly 1 would never be exceeded by a one-unit difference, because touching counts as overlap.
- **Epsilon equals band.** For R4 condition 3 the tolerance is `max(r*|reference|, a)`: a candidate is "not worse" if it is within the band of the other.
- **Multipliers.** Minimum gains (§7.3) multiply `r` by the tier multiplier `k`, giving the band `[1/(1+k*r), 1+k*r]`. Absolute floors are not multiplied. For absolute-only metrics the multiplier applies to `a`.

### 3.2 Thresholds by metric

Canonical metric names are the keys used in experiment specs and in `metric_thresholds` (Appendix A). The "Family" column is the ebr metric family (`thresholds.METRIC_FAMILIES`). "Process" floors apply to samples measured around a whole process (ebr `resource` source); "in-process" floors apply to harness-crate timers.

| ID | Metric (canonical) | Unit | Dir | Family | r | a | Justification |
|---|---|---|---|---|---|---|---|
| T-01 | `artifact_bytes`: every output byte needed for the claimed capability (framing, Index, manifest, padding, signatures, sidecars, receipts; program Task 7) | B | min | size | 0.01 | 64 B per artifact | Storage and transfer cost are linear in bytes, and SPEC §6.4 names distribution at a scale where bytes multiply by downloads, so 1% is material. Below 1%, differences are the size of incidental framing, varint or ordering choices that later implementation work can shift, and of adjacent-level steps within one codec `[INFERRED]`. Fixed per-archive differences under 64 B stay negligible even across 10^6 archives (64 MB total) `[FORMALLY_DERIVED]`. The metric is deterministic, so CIs reflect content heterogeneity only. |
| T-02 | `metadata_bytes` and its components `index_bytes`, `dictionary_bytes`, `side_data_bytes` (manifest, Index, entry metadata, integrity records, dictionaries, signatures, receipts; padding excluded, see T-16) | B | min | size | 0.05 | 64 B per artifact | These components matter through first-access fetch (T-11), index memory (T-06) and per-entry overhead, and each of those has its own threshold. Where metadata dominates the artifact (many-small-file trees, F04), T-01 already catches the change. The metric is diagnostic by default and primary only when the decision is about encoding that component. Components MUST reconcile exactly to `artifact_bytes`. |
| T-02b | `metadata_bytes_per_entry` | B/entry | min | size | 0.05 | 0.5 B/entry | At 10^6 entries, 0.5 B/entry is 0.5 MB. That is below T-01's 1% band whenever mean file size exceeds 50 B `[FORMALLY_DERIVED]`. |
| T-03 | `encode_wall_s` | s | min | time_wall | fast 0.05; balanced 0.05; dense 0.10; extreme 0.25 | 5 ms process; 0.1 ms in-process | `fast` exists for throughput and `balanced` is the default (SPEC §6.1–6.2), so they get the smallest timing band this environment is expected to resolve. `dense` treats encode time as secondary (SPEC §6.3). `extreme` treats compression time as nearly free but budget-bounded (SPEC §6.4), so only differences of a quarter matter. The process floor covers multi-millisecond per-invocation jitter from process creation, image load and on-access antivirus scanning on the Windows host `[INFERRED; A/A calibration verifies]`. It is far below the roughly 100 ms at which responses stop feeling immediate `[EXTERNALLY_SOURCED: R. B. Miller, "Response time in man-computer conversational transactions", AFIPS FJCC 1968; Card, Robertson & Mackinlay, "The information visualizer", CHI 1991]`. |
| T-04 | `decode_wall_s` | s | min | time_wall | 0.05 | 5 ms process; 0.1 ms in-process | Read-many use (SPEC §6.3) runs decode many times per archive, so decode gets the tightest timing band. Using the same value as T-03 for fast/balanced keeps one ebr family band. |
| T-05 | `encode_cpu_s`, `decode_cpu_s` (user+system, whole process tree) | s | min | time_cpu | as T-03 / T-04 | as T-03 | CPU time is the proxy for energy and for shared-runner cost, with the same materiality as wall time. Parallel effects are covered by T-14. |
| T-05b | `decode_throughput_bps`, `encode_throughput_bps`, `verify_throughput_bps` | B/s | max | throughput | as T-04 / T-03 / T-04 | none (the time floors are snapped first) | Throughput is the reciprocal of time, so the log-symmetric band is identical `[FORMALLY_DERIVED]`. |
| T-06 | `peak_rss_bytes` (Linux `ru_maxrss`), `peak_commit_bytes` (Windows job peak commit), `alloc_peak_bytes` (in-process counting allocator); decode and encode variants | B | min | memory_peak | 0.10 | 4 MiB (RSS, commit); 1 MiB (allocator) | Memory is a declared ceiling (`max_decode_memory`, SPEC §6.0a). Crossing a ceiling is an R1/R2 matter, not a trade-off. Within the bound, differences under 10% rarely change which machines can decode `[INFERRED]`. Allocator arenas and 4 KiB page granularity make RSS and commit jitter of a few MiB common `[INFERRED; A/A verifies]`. The counting allocator is exact, so its floor is smaller. |
| T-07 | `scratch_peak_bytes` | B | min | scratch_peak | 0.10 | 16 MiB | 16 MiB is below any practical scratch budget on CI runners `[INFERRED]`. ebr samples scratch every `poll_interval_s` (0.05 s by default), so measured peaks are lower bounds. A scratch verdict therefore also requires the candidate's design-derived scratch expectation to agree with the measurement. Zero scratch versus non-zero scratch is a binary capability when claimed (§3.3). |
| T-08 | `first_entry_latency_s`, `random_entry_latency_s` (local; from open to first *verified* byte of the target entry, with first unverified byte also reported per I25) | s | min | time_wall | 0.10 | 5 ms process; 0.1 ms in-process | Single operations are dominated by fixed setup costs and vary more, relative to their size, than bulk decode. About 10% is the smallest difference likely to change batch tools doing thousands of lookups `[INFERRED]`. p95 claims need at least 40 sampled operations per item (§4.6). |
| T-09 | `access_granularity_bytes` (worst-case decoded bytes needed to read one arbitrary byte; SPEC §6.0a `max_access_granularity`) | B | min | size | 0.25 | 64 KiB | This is a worst-case bound whose user-visible effect is measured by T-08. The band matters for profile-level utility, and differences under a quarter are dominated by the latency band `[INFERRED]`. Decoding 64 KiB takes on the order of tens of microseconds at GB/s decode rates `[INFERRED]`. The metric is deterministic. |
| T-10 | `remote_first_entry_latency_s`, `remote_random_entry_latency_s` under a network profile (§4.15) | s | min | time_wall | 0.10 | max(10 ms, 0.5 × profile RTT) | Remote latency comes in whole round trips. A difference smaller than half an RTT cannot reflect a structural change and is within application-level emulation error `[INFERRED]`. |
| T-11 | `http_bytes` per logical operation | B | min | size | 0.05 | 16 KiB | RFC 6928 §2 sets TCP's initial window to `min(10*MSS, max(2*MSS, 14600 bytes))` `[EXTERNALLY_SOURCED]`. Fetch differences below about 14.6 KB therefore usually add no round trip `[INFERRED]`. The 5% relative band reflects metered transfer and CDN cost on batch reads. |
| T-12 | `http_requests` per logical operation | count | min | other | 0.10 | 0.5 | Each request that is not multiplexed costs at least one RTT, so one request is the smallest structural difference. The 10% relative band governs batch operations. |
| T-13 | `verify_overhead_pp` = 100 × (verified wall / unverified wall − 1) | pp | min | other | none | 5.0 pp | Any overhead difference inside the decode-wall band (T-04, 5%) is indistinguishable in total decode time `[FORMALLY_DERIVED from T-04]`. When verification cannot be done, that is governed by I25/R1, not by this threshold. |
| T-14 | `parallel_speedup` = T(1 thread) / T(n threads), at fixed n and affinity configuration (§4.2) | ratio | max | other | 0.10 | 0.2 | Speedup is a ratio of two timings, each with a 5% band. log S = log T1 − log Tn, so to first order the bands add `[FORMALLY_DERIVED]`. The 0.2 floor prevents low-n differences such as 1.10 versus 1.25 from counting. |
| T-15 | `blast_logical_bytes_p95`, `blast_logical_bytes_max`, `blast_entries_p95` (unreadable logical bytes and entries per seeded single-byte corruption; at least 1000 injections per archive, stratified by region: payload, Index, metadata, dictionary, integrity) | B; count | min | other | 1.0 (band [0.5, 2.0]) | 1 MiB; 0.5 entry | Corruption is rare and is recovered from redundancy outside the archive. The design distinction that matters is the order of magnitude of localization: chunk, group, dictionary users, or archive. Lookback-k alternatives differ by small integer factors (k+1 chunks) `[FORMALLY_DERIVED from I29 lookback semantics]`. 1 MiB is the chunk size of the historical `fixed-1mib/v1` layout, the smallest existing localization unit `[INFERRED]`. The detection rate MUST be 100% (I21, I25; R1). |
| T-16 | `padding_overhead_pp` = 100 × padding bytes / artifact bytes without padding | pp | min | other | none | 1.0 pp, and a per-artifact padding-byte difference above 4 KiB | One percentage point of padding equals T-01's size band `[FORMALLY_DERIVED]`. Below one 4 KiB page or cluster, single-artifact differences vanish into filesystem allocation `[INFERRED]`. The trade-off against T-17 is resolved at R4, with both as primary metrics. |
| T-17 | Leakage for encrypted archives, per object over the corpus, for the observer model declared by DEC-CRY-053: `leak_min_entropy_bits` (for a deterministic observation channel with a uniform prior, log2 of the number of distinct observations); `leak_shannon_bits` (mutual information between plaintext length or boundary sequence and observations); `anonymity_set_median` (objects sharing an identical observation); `fingerprint_attack_success` (fraction of known files identified by the pre-registered attack of program Task 22.2) | bits; bits; count; fraction | min; min; max; min | other | none; none; 1.0; none | 1.0 bit; 1.0 bit; 0.5; 0.05 | One bit of min-entropy leakage doubles or halves the adversary's one-guess vulnerability `[EXTERNALLY_SOURCED: G. Smith, "On the Foundations of Quantitative Information Flow", FoSSaCS 2009]`. The anonymity-set band uses the same factor of two. 5 percentage points is the smallest attack-success change that a corpus-scale simulation with a few hundred target files can resolve `[INFERRED]`. The leakage-versus-padding framing follows PURBs/Padmé `[EXTERNALLY_SOURCED: Nikitin et al., PoPETs 2019(4)]`. **An I24 violation is R1:** `fingerprint_attack_success` `DISTINGUISHABLE` above the length-only reference, in which the observer sees only total padded object lengths. |
| T-18 | `task_success_rate`, `task_steps`, `task_errors` (simulated first-use sessions, §4.16) | fraction; count; count | max; min; min | other | none; 0.25; none | 0.15; 0.5; 0.5 | Differences under about 15 points of completion rate are below what task studies of this size can resolve `[INFERRED; see Sauro & Lewis, Quantifying the User Experience, 2nd ed., 2016, on completion-rate sample sizes]`. Evidence is capped at `SUPPORTED_WITH_LIMITATIONS` (§9.1). |
| T-19 | `fs_read_ops`, `fs_write_ops` | count | min | io | 0.10 | 64 | **Secondary only.** These counts depend on the page cache, cannot eliminate or select, and are used diagnostically. |

### 3.3 Binary (constraint) metrics

These metrics have no band. Any failure is an R1 or R2 elimination:

- round-trip byte equality (SPEC §6.5, I14);
- truthful verification status (I25);
- corruption and truncation detection rate of 100% (I21);
- output-hash equality where determinism is claimed (§4.13, I15, SPEC §5.7);
- measured resource use at or below the declared bound (I17, I20);
- confinement and policy refusal tests (I16);
- I24 per T-17;
- zero-scratch or streaming capability, when the candidate claims it.

The ebr family `correctness` carries the degenerate absolute band `[0, 0]`, so any non-zero pass-rate difference is flagged.

### 3.4 Why these values

Every band was chosen to satisfy three requirements:

1. **Above resolvable noise.** It must exceed the noise the environment can resolve with affordable repetitions. A/A calibration (§4.7) verifies this, and a band is never widened to pass.
2. **Below a real choice.** It must be smaller than the smallest difference that would change a user's or implementer's choice.
3. **Consistent across related metrics:** T-04 with T-13, T-05b and T-14; T-01 with T-16.

Threshold perturbation (×0.5 and ×2, §6.4) tests whether conclusions depend on the particular values chosen.

### 3.5 Relative-only and absolute-only choices

- **Relative-only.** Throughput (T-05b) has no floor of its own, because the time floors are applied before the reciprocal is taken.
- **Absolute-only.** Percentage-point, bit and fraction metrics (T-13, T-16, T-17, T-18) are already normalized, so a relative band on top of them would double-normalize.

### 3.6 Scope of ebr's built-in verdicts

ebr v1 applies **one band and one epsilon per metric family** (`thresholds.Thresholds.band(family)`). It does not apply absolute floors, profile-specific bands, group aggregation or three-way verdicts. Consequently:

- The family values in §3.7 are the bands of each family's reference metric.
- Verdicts, Pareto sets and sensitivity results that ebr's `normalize` computes are **informational**. Decision-grade verdicts come only from the decision-analysis tooling of §14, which reads `metric_thresholds`.
- Family `other` is intentionally left `null`, because no family-wide band is meaningful for request counts, bits, percentage points and success rates. ebr therefore reports `threshold_unset` for it and never substitutes a default.

### 3.7 Transcription into `thresholds.json` (schema `ebr.thresholds.v1`)

| Key | Value | Reference metric / source |
|---|---|---|
| `families.size.practical_significance_band` | `{kind: ratio, lower: 0.9900990099, upper: 1.01}` | T-01, §3.2 |
| `families.size.pareto_epsilon` | `{kind: relative, value: 0.01}` | T-01 |
| `families.time_wall.practical_significance_band` | `{kind: ratio, lower: 0.9523809524, upper: 1.05}` | T-04 |
| `families.time_wall.pareto_epsilon` | `{kind: relative, value: 0.05}` | T-04 |
| `families.time_cpu.practical_significance_band` | `{kind: ratio, lower: 0.9523809524, upper: 1.05}` | T-05 (decode) |
| `families.time_cpu.pareto_epsilon` | `{kind: relative, value: 0.05}` | T-05 |
| `families.memory_peak.practical_significance_band` | `{kind: ratio, lower: 0.9090909091, upper: 1.10}` | T-06 |
| `families.memory_peak.pareto_epsilon` | `{kind: relative, value: 0.10}` | T-06 |
| `families.scratch_peak.practical_significance_band` | `{kind: ratio, lower: 0.9090909091, upper: 1.10}` | T-07 |
| `families.scratch_peak.pareto_epsilon` | `{kind: relative, value: 0.10}` | T-07 |
| `families.throughput.practical_significance_band` | `{kind: ratio, lower: 0.9523809524, upper: 1.05}` | T-05b (decode) |
| `families.throughput.pareto_epsilon` | `{kind: relative, value: 0.05}` | T-05b |
| `families.io.practical_significance_band` | `{kind: ratio, lower: 0.9090909091, upper: 1.10}` | T-19 (secondary) |
| `families.io.pareto_epsilon` | `{kind: relative, value: 0.10}` | T-19 |
| `families.correctness.practical_significance_band` | `{kind: absolute, lower: 0, upper: 0}` | §3.3 |
| `families.correctness.pareto_epsilon` | `{kind: absolute, value: 0}` | §3.3 |
| `families.other.*` | `null` (intentional) | §3.6 |
| `bootstrap.confidence` | `0.99` | §4.12 |
| `bootstrap.resamples` | `20000` | §4.12 |
| `quiet_machine_guard.max_loadavg_1m` | `32.0` | §4.4 |
| `quiet_machine_guard.max_other_cpu_percent` | `5.0` | §4.4 |
| `sensitivity.min_fraction_preserving_winner` | `0.90` | §6.6 |
| `sensitivity.dirichlet_samples` | `10000` | §6.3 |
| `sensitivity.dirichlet_concentration` | `4.0` | §6.3 |

Every `decision_method_section` key cites the section given above. The extension keys in Appendix A (`absolute_floor`, `metric_thresholds`, `protocol`, `cost_tiers`, and the remaining `bootstrap` and `sensitivity` keys) pass ebr v1 validation (`thresholds.validate` ignores unknown keys) and are read by the decision tooling. Decision specs MUST NOT set `analysis.sensitivity.dirichlet_samples` or `dirichlet_concentration`: `normalize` lets a spec override them, and a checked-in spec lint MUST reject that (§14).

---

## 4. Statistical protocol

### 4.1 Environments and comparability

- Environments are identified by `env_id` from `research/environment/`: `windows-host`, `wsl-ubuntu`, `docker-ubuntu-24.04`, plus QEMU arm64 (always labelled `EMULATED`).
- **Pairing within an environment.** All candidates in one comparison run in the same `env_id`, in the same ebr run (interleaved `order: randomized_blocks`), with the same cache state (§4.5) and the same affinity configuration (§4.2). Numbers from different environments are never paired.
- **Docker.** Docker timing is informational unless A/A calibration (§4.7) passes in that environment.
- **WSL2 filesystem.** WSL2 timing never uses `/mnt/*` (drvfs/9P) paths. Inputs and scratch live on ext4 under `/root/eb-research`.
- **Windows filesystem.** Windows timing uses local NTFS outside the repository (ebr refuses in-repo scratch).
- **QEMU arm64** is used only for correctness and determinism, never for timing.
- **Timing scope.**
  - A decision scoped to Windows uses the Windows host.
  - Linux-scoped timing uses WSL2, with the limitations in §4.3.
  - A claim covering both requires that the two environments produce no `DISTINGUISHABLE` verdicts in opposite directions.
  - macOS/APFS is `PLATFORM_BLOCKED` and is recorded as a limitation of any scope that includes it.

### 4.2 Windows host controls

The host is an Intel i9-14900HX: 8 P-cores with SMT (Windows logical processors 0–15) and 16 E-cores (logical processors 16–31), 64 GB RAM. At capture, Defender real-time protection was on, VBS/Hyper-V was running, and the power scheme was Balanced on AC, all recorded in the `windows-host` fingerprint.

| Control | Rule |
|---|---|
| AC power | Required. AC line status is checked before and after each run. Any battery interval invalidates every sample of the run (reason `on_battery`). |
| Power configuration | The power scheme and power-mode overlay are part of `platform_id`, and a timing campaign uses a single `platform_id`. Agents never change power or security settings. The program owner MAY select a scheme before a campaign and re-capture the environment. |
| Process power throttling | The runner opts measured processes out of power throttling via `SetProcessInformation(ProcessPowerThrottling)`, with the execution-speed control bit set and the state bit cleared `[EXTERNALLY_SOURCED: Microsoft Learn, SetProcessInformation / PROCESS_POWER_THROTTLING_STATE]`. Windows timing is not decision-grade until this is implemented (§14). |
| Affinity | Named configurations, used as declared per experiment. Candidate thread counts MUST equal the number of logical CPUs in the set. SMT sibling pairs MUST be verified from `GetLogicalProcessorInformationEx` before use; the expected pairing is (0,1), (2,3), …, (14,15). |
| `st-p` | {2}; sibling 3 left idle; CPUs 0–1 avoided because interrupt and DPC load concentrates there `[INFERRED]`. Single-thread reference. |
| `mt-p-n`, n ∈ {2, 4, 7} | First n of {2, 4, 6, 8, 10, 12, 14}: one logical CPU per physical P-core, no SMT siblings. |
| `mt-psmt-14` | {2..15} |
| `st-e`, `mt-e-n`, n ∈ {2, 4, 8, 16} | {18} and {16..16+n−1}. Reported as the low-end core class. |
| `os-scheduled` | Unpinned. End-to-end realism only, never paired with pinned samples. |
| Hybrid confound | Samples from different affinity configurations are never compared. |
| Thermal and power limits | No temperature access without admin. Controls: (1) **cooldown**, an idle gap before each sample of min(60 s, max(2 s, previous sample wall)); (2) **drift canary**, a fixed single-thread CPU-bound workload of about 1 s on `st-p`, run 3× at session start (its median is the baseline) and then once every 10 blocks or 5 minutes, whichever comes first. If a canary exceeds 1.05 × baseline (the T-04 band), pause 120 s and repeat the canary. If it still exceeds, abort the session and mark every sample since the last good canary invalid (`drift_canary`). (3) Randomized blocks with pairing inside rounds (§4.9). (4) A comparison whose candidates' median sample walls differ by more than 10×, with the longer above 20 s, is flagged `power_limit_confound` (sustained power-limit throttling) and capped at MEDIUM confidence. |
| Defender and VBS | Recorded, never changed. Warmup rounds absorb first-open scanning of inputs. Encode timings include scan-on-close of outputs for every candidate, which is the realistic default user configuration, and are labelled `defender_on`. Results where candidates write different numbers of files also include antivirus cost proportional to file count, and that is part of the Windows result. Scheduled scans show up as other-process CPU and trip the guard. |
| Exclusivity | Timing windows are exclusive: no other workflow agents, builds, Docker containers or corpus provisioning. The orchestrator schedules the windows; the guard (§4.4) is a check, not the control. |
| Priority | Normal process priority, the realistic setting. |

### 4.3 WSL2 controls

The VM runs Ubuntu 24.04 with 32 vCPUs on kernel 5.15.167.4, filesystem ext4.

- **Known limits.** vCPUs cannot be pinned to host cores. Host load appears as steal time, and `taskset` pins to vCPUs only (PROGRESS execution-environment caveats).
- **Consistency controls.**
  - Measured processes are pinned to fixed vCPU sets: `st-v` = {2}; `mt-v-n` = {2..n+1}.
  - The guard inside the VM counts steal time as busy (`guard.py`).
  - A host-side sampler on Windows records host other-process CPU during WSL samples, with the same limit (§14).
  - The Windows host is otherwise idle.
  - Cooldown and the drift canary run as in §4.2, with the canary executed inside WSL.
- **Hybrid confound.** Whether a vCPU lands on a P-core or an E-core is uncontrolled. A/A calibration decides decision-grade status for each metric family. Where it fails, WSL2 serves only deterministic metrics, correctness, and informational timing.

### 4.4 Quiet-machine guard limits

| Key | Value | Rationale |
|---|---|---|
| `max_other_cpu_percent` | 5.0% of all logical CPUs, in the pre and post idle windows and over each sample window | 5% of 32 is about 1.6 logical CPUs. `guard.py` attributes kernel I/O threads working for the measured process, and Defender scanning its files, to "other". A tighter limit would invalidate I/O-heavy candidates' samples more often, which is a candidate-dependent exclusion bias. Background above roughly 1.6 CPUs indicates real concurrent work `[INFERRED]`. |
| `max_loadavg_1m` | 32.0 (Linux/WSL only) | Load average includes the measured workload's own recent threads through the 1-minute exponential decay. Any tighter value would exclude samples following heavy candidates, again a candidate-dependent exclusion. 32 equals the vCPU count; above it, runnable demand exceeds CPUs whatever its origin. |
| `guard.max_wait_s` in decision specs | ≥ 300 s | Lets transient background work finish. A sample that cannot start after the wait aborts the run (ebr behaviour). |

**Invalid-sample balance.**

- A (candidate, item) cell with more than 10% invalid samples is not decision-grade.
- If two compared candidates' invalid rates on the same item differ by more than 5 percentage points, that item is excluded from the comparison and the exclusion is reported.
- If more than 20% of items are excluded, the comparison is not decision-grade.

A/A calibration verifies that the limits are adequate. When it fails, controls are strengthened (§4.7); the limits are never loosened.

### 4.5 Warmups and cache state

- **Warmup rounds** (ebr `warmups`): 2 for small and medium tier items, 1 for large tier items. They are recorded (`record_warmups: true`) and excluded from analysis.
- **Cache state** is declared per experiment:
  - `hot` (default): inputs are read during warmups;
  - `vm-cold` (WSL only): `sync` then `echo 3 > /proc/sys/vm/drop_caches` before each sample. This clears only the VM's page cache, not the host's.
  - Cold-cache Windows runs need admin rights and are not used.
- **Warmup adequacy** is checked in A/A calibration. If the median per-item Kendall τ between repetition index and value (`stats.kendall_tau`) lies outside ±0.3, one warmup round is added for that environment and family, and the change is logged as protocol strengthening.

### 4.6 Repetitions

**Deterministic metrics** (sizes, counts, request counts, decoded bytes, leakage computed from deterministic outputs, blast radius under seeded injection): 2 repetitions, in different rounds and processes, plus the repeat-hash check (§4.13). If the check passes, the values are exact and nothing more is repeated.

**Timing and memory metrics:**

| Tier | Minimum measured rounds | Step | Cap |
|---|---|---|---|
| small (≤ 16 MiB) | 10 | 10 | 30 |
| medium (≤ 512 MiB) | 10 | 10 | 20 |
| large (≤ 8 GiB) | 5 | 5 | 15 |

**Adaptive rule.** The rule depends on precision, not on significance, which avoids optional-stopping bias `[method choice; cf. Kalibera & Jones, "Rigorous Benchmarking in Reasonable Time", ISMM 2013]`.

1. After the minimum rounds, compute for each primary comparison on each item the paired bootstrap CI of the log ratio, at the decision's per-comparison confidence.
2. If its half-width exceeds 0.5 × `log1p(r)`, add one step of rounds for that item, covering every candidate.
3. Stop when the target is met or at the cap. Items still above target at the cap are flagged `imprecise`, and their verdicts are computed normally.
4. The rule never looks at the direction of the effect or at where the CI sits relative to the band.

**Operation sampling.** Latency and remote experiments sample at least 40 operations per item per round whenever p95 is reported or claimed.

**Simulated usability:** at least 20 sessions per (task, candidate) (§4.16).

### 4.7 A/A calibration (noise floor)

Before the first decision-grade timing or memory analysis in an environment, run `EXP-CAL-AA-<env>`. It uses two candidates with identical binaries and arguments but distinct names, the full protocol, and tuning items covering at least 10 families and all three scale tiers, for every timing and memory family a decision will use.

**Pass criteria**, per (environment, metric family):

1. The corpus-level A/A ratio CI (0.99, family-stratified group bootstrap, §4.10) is `EQUIVALENT`.
2. At most 2% of item-level A/A verdicts are `DISTINGUISHABLE`.
3. At most 10% of samples are invalid.
4. The median item-level CI half-width at the cap is at most 0.5 × `log1p(r)`.

**On failure**, strengthen controls in this order and re-run A/A under a new experiment id after each step:

1. Add one warmup round.
2. Double the cap.
3. Use stricter affinity (`st-p`, or `mt-p` without SMT).
4. Double the cooldown.

If all four fail, that (environment, family) is not decision-grade. Decisions that need it use another environment or stay `INSUFFICIENT_EVIDENCE`. **Bands are never widened.**

A/A runs are protocol calibration, not decision results. They are repeated whenever `env_id` changes.

### 4.8 Descriptives and raw samples

- **Raw records.** Every sample is stored in raw JSONL with a hash chain, and `ebr verify-raw` MUST pass before `normalize`. Warmups are recorded. Invalid samples are kept with their reasons. Raw runs are never deleted; abandoned runs remain in place, labelled.
- **Descriptives** per (env, candidate, item, metric), via `stats.describe`: n, median, p90, p95, IQR, MAD (unscaled), min, max, mean, geomean. Percentiles use Hyndman–Fan type 7 `[EXTERNALLY_SOURCED: Hyndman & Fan, The American Statistician 50(4), 1996]`.
- **Small n.** p90 with n < 10 and p95 with n < 20 are flagged `low_n` and cannot support tail claims.
- **No outlier removal.** Every valid sample counts, and medians provide the robustness.

### 4.9 Units of analysis and aggregation

| Level | Unit | Statistic |
|---|---|---|
| L0 | sample | raw value |
| L1 | item | Per-item median over rounds (timing), or exact value (deterministic, §4.13). Item-level CIs pair candidates within the same round (ebr `pairing: item_repetition`). Floor snapping (§3.1) happens here. |
| L2 | independence group | Geometric mean of the item ratios in the group. Items sharing an `independence_group` are not independent. |
| L3 | family | Geometric mean over groups, with equal group weights. |
| L4 | corpus | Weighted geometric mean over applicable families. Base weights are equal, following the corpus methodology's unweighted per-family reporting; §6.5 reweights. |

- **Absolute-only metrics** use arithmetic means of item-level median differences at L2–L4.
- **Applicability.** A candidate is inapplicable to families its design excludes, for example a JPEG transform outside F12. Applicable families are declared in the experiment record. Transforms are still measured on non-target families, to charge their detection cost (§7.3).
- **Minimum support.**
  - A corpus-level verdict needs at least 10 independence groups across at least 5 applicable families in the split used.
  - A family-level verdict needs at least 2 groups and uses the **unanimity rule**:
    - `DISTINGUISHABLE` in a direction if every group lies beyond the band on that side and every constituent item verdict is `DISTINGUISHABLE` on that side;
    - `EQUIVALENT` if every group lies inside the band;
    - `INCONCLUSIVE` otherwise.
  - A family with a single group is labelled `single-group`. It is reported but cannot on its own support a scoped winner.
- **Corpus context at pre-registration.** Most family×split cells hold 2–4 independence groups; the range is 1–6. Command: `python -c "import json,collections as c;m=json.load(open('research/corpus/manifest.json'));g=c.defaultdict(set);[g[(i['family'],i['split'])].add(i['independence_group']) for i in m['items']];print(sorted(c.Counter(len(v) for v in g.values()).items()))"`. Family-level CIs are therefore not attempted, and the unanimity rule applies instead.

### 4.10 Bootstrap confidence intervals

- **Method.** Percentile bootstrap, PCG64 seeded from the spec's `seeds.bootstrap`, with type-7 quantiles, following the `stats.py` conventions `[EXTERNALLY_SOURCED: Efron & Tibshirani, An Introduction to the Bootstrap, 1993]`.
- **Item level.** Rounds are resampled as pairs: `stats.bootstrap_paired_ratio_ci` with `statistic="median_of_ratios"`, and `stats.bootstrap_paired_difference_ci` for differences.
- **Corpus level.** Family-stratified cluster bootstrap: families are fixed strata, and within each applicable family, independence groups are resampled with replacement before recomputing L4 `[EXTERNALLY_SOURCED: Davison & Hinkley, Bootstrap Methods and their Application, 1997, stratified and clustered resampling]`. Single-group families contribute a fixed value. This is not yet in `stats.py` (§14).
- **Small-sample caveat.** Percentile intervals undercover when there are fewer than 10 units. The mitigations are the conservative bands, the unanimity rule and held-out replication. BCa and studentized intervals are not used, because `stats.py` fixes the convention.

### 4.11 Noise rule and verdict classes

Given a CI [L, U] for a ratio or difference and a band [bL, bU]:

| Verdict | Condition |
|---|---|
| `DISTINGUISHABLE` (below or above) | U < bL or L > bU. Touching an edge overlaps (`stats.noise_rule`). Mapped to BETTER or WORSE by the metric's direction. |
| `EQUIVALENT` | bL < L and U < bU (wholly and strictly inside) |
| `INCONCLUSIVE` | anything else |

- **Composite metrics** (both `r` and `a`): `DISTINGUISHABLE` requires the ratio and the difference to be `DISTINGUISHABLE` on the same side. `EQUIVALENT` holds if either is `EQUIVALENT`, because significance requires both thresholds to be exceeded.
- ebr's `normalize` labels `indistinguishable` without separating the two classes. The decision tooling MUST separate them (§14).

### 4.12 Multiple comparisons

- **Enumeration.** The experiment record enumerates every **primary comparison**, meaning each (candidate pair, primary metric, environment, aggregation level) that R4–R10 will use. Their count is m.
- **Family-wise error rate** α = 0.05 per decision, controlled by Bonferroni:
  - per-comparison confidence 0.99 when m ≤ 5, since 5 × 0.01 = 0.05;
  - 1 − 0.05/m when m > 5.
- **Resamples:** B = max(20000, ⌈200 / (1 − confidence)⌉), which keeps at least 100 replicates in each tail.
- **Family-level unanimity verdicts** use item-level CIs at the decision's per-comparison confidence.
- **Secondary and diagnostic comparisons** use 0.99 unadjusted, are labelled `unadjusted`, and cannot eliminate or select. Any comparison added after results are visible is secondary.
- **Across decisions** there is no adjustment. Program-level multiplicity is controlled by held-out replication (R5) and the confirmation-bias triggers (§10.2).

### 4.13 Deterministic byte metrics: repeat-hash check

- Every experiment that produces artifacts records the output SHA-256 of each sample (ebr metric source `path_sha256`).
- **Acceptance.** All valid samples of an (env, candidate, item) share exactly one output hash, shown as a single value under `string_metrics` in `summary.json`. Byte metrics are then exact: the item-level CI collapses to a point, and the item verdict compares the exact values against band and floor.
- **Mismatch.**
  - If the candidate claims determinism (deterministic mode per SPEC §5.7 and SPEC §12, or I15), a confirming rerun that reproduces the mismatch makes it an R1 elimination.
  - Otherwise the metric is reclassified as stochastic, with timing-style repetitions (§4.6) and bootstrap CIs.
- **Determinism claims** across thread counts, hosts and architectures (Windows, WSL2, QEMU arm64 emulated) require identical hashes for identical inputs, planner ID and declared resource model.
- **Size reconciliation.** The per-section byte accounting of the harness (`ebr-pack`) MUST sum exactly to `artifact_bytes`. A mismatch invalidates the sample.

### 4.14 Memory and scratch measurement

- **Linux:** `ru_maxrss` via GNU time (`max_rss_kib`), a per-process high-water mark. For multi-process pipelines the value is the maximum over processes, not the sum; sampled tree sums are informational.
- **Windows:** job peak commit (`peak_job_commit_bytes`, exact) is primary. Sampled working set is informational and a lower bound.
- **In-process:** a counting global allocator in the harness crates (`alloc_peak_bytes`, exact) is primary for library-level decode memory against `max_decode_memory`.
- **No cross-OS comparisons.** The quantities differ, so memory is never compared across operating systems.
- **Scratch** is sampled every `poll_interval_s` (default 0.05 s); see T-07.

### 4.15 Remote measurement conditions

`sch_netem` is unavailable, so an application-level proxy emulates the network. Every remote result is labelled `emulated-network`. Profile values are representative orders of magnitude `[INFERRED]`.

| Profile | RTT | Bandwidth |
|---|---|---|
| NP-LAN | 2 ms | 1000 Mbit/s |
| NP-METRO | 20 ms | 200 Mbit/s |
| NP-CONT | 80 ms | 50 Mbit/s |
| NP-INTER | 200 ms | 20 Mbit/s |

- The proxy or origin counts requests and bytes exactly.
- The connection model is declared per experiment: setup round trips, reuse, maximum parallel connections, HTTP version.
- Packet loss is injected only to test failure behaviour. No performance claim is made under loss.

### 4.16 Usability evaluation protocol (simulated)

Pending DEC-ECO-083, any usability-derived verdict requires at least the following:

- **Sessions.** Independent simulated first-use sessions, run by agents not involved in CLI design. They see only the CLI's own help or man text and the task statement.
- **Pre-committed materials.** Tasks, mechanically checked success criteria, and candidate CLIs are committed before any session.
- **Volume and order.** At least 20 sessions per (task, candidate), in randomized order.
- **Held-out tasks.** A task set written before design iteration and never used during it serves R5.
- **Statistics.** CIs by bootstrap over sessions.
- **Labelling and cap.** Everything is labelled `SIMULATED`, the maximum evidence state is `SUPPORTED_WITH_LIMITATIONS`, and no claim about human usability is made from simulated evidence.

---

## 5. Split discipline

### 5.1 Roles

| Split | Permitted use | Forbidden |
|---|---|---|
| `tuning` | Search, parameter sweeps, candidate refinement, iterative development | None beyond the corpus rules (`research/corpus/methodology.md` §3) |
| `validation` | Confirming selections made on tuning; noise estimation; regression checks | Search. Choosing among variants by validation results is tuning (§5.2). |
| `heldout` | One final evaluation of the frozen design (R5) | Everything before unlock (§5.4). Any re-selection after unlock (§5.6). |

### 5.2 Validation budget

- Each decision may consult validation for at most **two** confirmation looks: the initial confirmation, and one more after a documented change.
- Once a candidate, parameter, metric or analysis changes after a validation look, that validation evidence counts as tuning evidence for the changed design.
- A third look requires new validation items from new independence groups, committed and fingerprinted before use.

### 5.3 Freezing the held-out set

- `research/corpus/heldout-lock.json` MUST reach status `frozen` before the first decision-relevant experiment result is committed. Freezing (`assemble.py --freeze`) records `heldout_set_sha256` over the lines `<item_id>\t<logical_tree_sha256>\n`, sorted by `item_id`. At pre-registration the lock is `draft` with 65 items.
- `logical_tree_sha256` is the identity of record. `tree_sha256` is unstable on ext4 (the `st_blocks` and extent issues noted in PROGRESS).
- **Relocking after that point** is allowed only for integrity failures (an item unreadable or corrupt), never for content-based reasons. The reason goes into the lock's `history`. Replacement items come from new independence groups and are chosen by pre-declared criteria, without looking at their content statistics.

### 5.4 Access before unlock

- **Forbidden to everyone:** held-out content, listings, statistics and paths, including `/root/eb-research/heldout` and `/root/eb-research/fingerprints/heldout`.
- **Allowed:** fingerprint-only records (hashes, byte and file counts, family, tier, group).
- **Tool enforcement:**
  - The ebr runner refuses `split: heldout` unless `EB_HELDOUT_UNLOCK` equals the recorded `design_freeze_sha`.
  - `research/corpus/tools/stats.py` requires `--unlock-heldout` together with a committed `heldout-unlock.json`.
  - `corpuslib.assert_not_heldout` guards corpus paths.
  - `allow_heldout=True` appears only in `provision.py` and `assemble.py --verify-heldout`.

### 5.5 Freeze and unlock sequence

1. **Commit A**, the design freeze: the decision ledger with provisional selections (`selected_decision` marked `provisional`, status `INSUFFICIENT_EVIDENCE`); every held-out experiment spec and record (§12); and the decision tooling at the exact version the held-out analysis will use.
2. **Commit B**: `research/decisions/design-freeze.json` containing:
   - `design_freeze_sha` = SHA of commit A;
   - `decision_ledger_sha256`;
   - `heldout_set_sha256` (copied from the frozen lock);
   - `decisions`: the frozen decision ids;
   - `heldout_specs`: paths with SHA-256;
   - `approved_by`, `date`.

   **This program uses only the key `design_freeze_sha`.** ebr also accepts `commit_sha` and `sha` for compatibility, and they MUST NOT be used.
3. **Commit C**: `research/corpus/heldout-unlock.json` with `design_freeze_commit` equal to A's SHA, plus approver and reason. A MUST be an ancestor of HEAD.
4. **Run** every held-out experiment exactly once, with `EB_HELDOUT_UNLOCK` set to A's SHA. ebr records `unlock_sha` in each held-out raw record.

### 5.6 After unlock

This is the default pending DEC-ECO-076; REQ-ECO-0188 applies.

- **Report-only.** Held-out results are published as obtained.
- **Re-runs** are allowed only for infrastructure failures: invalid samples, crashes, guard aborts. Every attempt is kept and reported. A re-run chosen because of its outcome is leakage (§5.7).
- **Correctness fixes** to candidates after unlock are disclosed with before-and-after held-out results. The result is labelled `post-unlock-fix` and caps confidence at MEDIUM.
- **Re-tuning or re-selection** after unlock requires fresh confirmatory data: items collected after the freeze, from new independence groups, committed and frozen before use. The original held-out results stay published.

### 5.7 Leakage events and consequences

| Event | Detection | Consequence |
|---|---|---|
| L1: any held-out content, listing, statistic or path read before unlock | Tool refusal logs; git history of the lock, freeze and unlock files; audit (§5.8) | Every decision with evidence from the leaked families loses R5 eligibility and reverts to `INSUFFICIENT_EVIDENCE`. The leaked items are relocked as `spent` with the reason. Re-validation needs fresh held-out items. |
| L2: held-out results used to modify a design, candidate, parameter or analysis | Commit order; the deviation log | As L1 for the affected decisions. The modified design needs fresh confirmatory data (§5.6). |
| L3: search on validation | Validation look count; experiment records | That validation evidence becomes tuning evidence, and a new validation split of new groups is needed. |
| L4: corpus items added, removed or replaced after results were visible, without a decision record | Manifest history; corpus methodology §11 | The affected experiments' results are invalid for decisions. Rerun on the pre-registered items. |
| L5: undeclared public-benchmark contamination in held-out | `tags: ["public-benchmark"]` audit | Report with and without the contaminated items. Confidence is capped at MEDIUM until resolved. |
| L6: independence group or upstream shared across splits | Corpus validator (methodology §3) | Items of the offending group are excluded from held-out analyses. If the validator missed the sharing, treat it as L1 for the affected families. |

### 5.8 Audits

- **Before the freeze:**
  - (a) `heldout-unlock.json` and `design-freeze.json` are absent from git history;
  - (b) `grep -rn allow_heldout research/` shows only the two permitted callers;
  - (c) no raw record has split `heldout`;
  - (d) every decision experiment's raw `meta` git SHA descends from its spec's pre-registration commit, with `dirty = false`.
- **After unlock:** every held-out raw record cites `unlock_sha` = A.

---

## 6. Sensitivity analysis

### 6.1 Inputs

- **Candidates:** those surviving R1–R6 in the scope.
- **Metrics:** primary metrics only.
- **Values:** scope-level aggregates (§4.9), from validation before the freeze and again from held-out after unlock.
- **Tooling:** the checked-in decision tooling (§14). ebr's own `sensitivity.csv` uses min-max-normalized scores and is informational (§6.2).

### 6.2 Band-unit utility and regret

For primary metric m, candidate c, and the best value v_best in the scope:

- **Relative metrics:** `q_m(c) = floor( |ln(v_c / v_best)| / ln(1 + r_m) )`. This counts whole band-widths from the best value, after floor snapping.
- **Absolute-only metrics:** `q_m(c) = floor( |v_c - v_best| / a_m )`.
- **Utility:** `U(c) = - sum_m w_m * q_m(c)`, with weights summing to 1. The winner maximizes U; ties are resolved by R10.
- **Regret** in a part p of a partition: `regret_p(c) = U_p(best_p) - U_p(c)`, in band units.

Why not min-max scores: min-max normalization stretches a difference that lies inside the band across the full [0, 1] range, so a practically negligible difference could decide the winner. Quantizing to band units removes such differences by construction. The scores are passed to `stats.weight_sensitivity` without `normalize_scores`; the `sum` aggregate accepts any real-valued scores where higher is better.

### 6.3 Weight perturbation

- **Base weights.** Profile decisions use Appendix B. Other decisions declare base weights in the experiment record, derived from the dimensions in `archetypal-objective.md`; the default is equal weights over the primary metrics.
- **Grid.** The full Cartesian product of the factors (0.5, 0.75, 1.0, 1.25, 1.5) (`stats.GRID_FACTORS`: ±25% and ±50%) over the k criteria, renormalized. Experiment records SHOULD keep k ≤ 8; above 8, use `one_at_a_time` mode against the same bar.
- **Dirichlet.** 10000 samples with concentration c = 4.0, so alpha = c·k·w_base (`stats.weight_sensitivity`), seeded from the spec. With k = 5 equal weights, each weight's coefficient of variation is sqrt((k−1)/(ck+1)) = sqrt(4/21) ≈ 0.44, a little wider than the grid factor spread (standard deviation 0.354) and with correlated shifts that the grid does not explore `[FORMALLY_DERIVED]`.
- **Stability metric.** FPW, the fraction preserving the winner: the share of weightings under which the selected candidate attains the maximal U, with ties counted as preserved (`weight_sensitivity` semantics).

### 6.4 Threshold perturbation

Re-run R4–R10 with bands and floors scaled:

- **Global:** all metrics ×0.5, then all metrics ×2.
- **Single-metric:** each primary metric's band individually ×0.5 and ×2.

Outcomes:

- **Global:**
  - selection identical under both scalings: pass;
  - identical under exactly one: confidence at most MEDIUM;
  - identical under neither: the scope fails, and R9 moves to a narrower scope.
- **Single-metric:**
  - selection identical in at least 90% of perturbations: pass;
  - 80% to 90%: confidence at most MEDIUM;
  - below 80%: the scope fails.

### 6.5 Workload-mix reweighting

Base family weights are equal over applicable families.

| Test | Procedure | Stability metric |
|---|---|---|
| Leave-one-family-out (LOFO) | Drop each applicable family in turn and recompute L4 and the selection. | Fraction of LOFO sets preserving the selection |
| Family Dirichlet | 10000 family-weight samples, concentration 4.0 around the equal weights, seeded | FPW |
| Scenario weights | Each scenario in Appendix B: 0.8 of the weight spread uniformly over the scenario's families, 0.2 uniformly over the other applicable families | Selection preserved in every scenario. Otherwise scenario-scoped winners are considered at R9, subject to expressibility and cost. |

### 6.6 Stability bars

| Test | Pass | MEDIUM band | Fail |
|---|---|---|---|
| Weight grid FPW | ≥ 0.90 | [0.80, 0.90) | < 0.80 |
| Weight Dirichlet FPW | ≥ 0.90 | [0.80, 0.90) | < 0.80 |
| Global threshold scaling | identical at ×0.5 and ×2 | identical at one of them | identical at neither |
| Single-metric threshold scaling | ≥ 0.90 | [0.80, 0.90) | < 0.80 |
| LOFO | ≥ 0.90 | [0.80, 0.90) | < 0.80 |
| Family Dirichlet FPW | ≥ 0.90 | [0.80, 0.90) | < 0.80 |
| Scenario weights | all preserved | not applicable: scenario-scoped handling at R9 | not applicable |

- **Failing a test** means the scope does not support a winner; R9 moves to a narrower scope.
- **Held-out re-evaluation** (R5 f) uses the same bars.
- **Rationale for 0.90.** A selection that flips under more than one plausible re-weighting in ten depends on preferences the program cannot justify. The 0.80 lower bar acknowledges that base weights are `[INFERRED]` judgments. The approach follows stochastic multicriteria acceptability analysis `[EXTERNALLY_SOURCED: Lahdelma, Hokkanen & Salminen, "SMAA – Stochastic multiobjective acceptability analysis", European Journal of Operational Research 106(1), 1998]`.
- **Recording.** The ledger field `sensitivity_analysis` cites the generated CSV path, each test's stability value, and the bar outcome.

---

## 7. Permanent cost accounting

A format decision costs something forever. Every conforming reader carries each wire feature for the life of the format (SPEC §4.9, §19.2), and every dependency must be maintained, audited and licensed. This section makes that cost explicit and sets the gains needed to justify it.

### 7.1 Cost elements

| ID | Element | Unit | Measurement |
|---|---|---|---|
| C1 | Specification | Normative words added or changed; new wire record types; new fields, enum values, feature bits and registry identifiers with decode semantics; new distinct decode behaviours | A checked-in counter over the candidate's normative text diff; table cells are counted as words. Records and fields are counted from the candidate's normative description. |
| C2 | Implementation | Non-blank, non-comment lines, counted separately for writer, full reader, minimal reader (SPEC §19.2) and tests | Pinned counting tool or a checked-in counter over the candidate's research patch. Reader LoC is reported separately because readers are forever. |
| C3 | Dependencies | New direct and transitive crates (unique name@version, `cargo tree -e normal --target all`); `unsafe` blocks and functions in new dependency sources; native code (`-sys` crates, `build.rs` compiling C/C++/asm, bytes of native source); SPDX licence expression; maintenance (last release date, number of people with publish rights, open RUSTSEC advisories, MSRV relative to the workspace MSRV) | Checked-in scripts over `cargo tree` and `cargo metadata` output, the vendored sources, and a pinned advisory database snapshot |
| C4 | Attack surface | New parser entry points reachable from untrusted archive bytes; new allocation sites sized by untrusted values, each needing a declared bound (I17, I20); new `unsafe` or native code reachable from untrusted input; new fuzz targets required; new cryptographic material (keys, nonces, associated data) | Enumerated from the candidate's reader design, and cross-checked against the fuzz-target inventory |
| C5 | Independent implementability | Whether a normative specification exists independent of any implementation (SPEC §5.8); whether external normative references are stable, freely available standards (RFC, ISO/IEC, FIPS, W3C) or code-defined; test vectors; lines of code in an independent minimal decoder written from the spec alone, where the independent-reader track has one | Spec review by a reader not involved in the candidate; independent-reader measurements |
| C6 | User complexity | New CLI flags, modes and concepts; decision points added to first-use tasks | Task-complexity counts per task (DEC-ECO-083), plus T-18 when simulated sessions exist |
| C7 | Longevity and churn | Planner-ID churn caused by pinned encoder versions (SPEC §5.7); migration cost for existing pre-v1 archives (DEC-CON-013); reliance on one upstream implementation | Recorded qualitatively with references, and counted where countable |

### 7.2 Cost tiers

A candidate's tier is the **highest** tier implied by any of its cost elements.

| Tier | Multiplier k | Includes |
|---|---|---|
| T0 | 1 | Implementation-only change: no normative, wire, dependency or user-visible change |
| T1 | 2 | Planner, policy or default change, or a parameter value, within existing normative mechanisms and with no new decoder behaviour; a new pure-Rust writer-only dependency without `unsafe`; a partition into scoped winners implemented as planner or policy logic (R9) |
| T2 | 3 | New field, enum value or feature bit with decode semantics in an existing record; new codec in the *registered extended* set (SPEC §5.8); new user-visible flag or mode; new profile; new pure-Rust reader-path dependency without `unsafe` |
| T3 | 5 | New codec or reversible transform in the *baseline* set; new record or section type; any dependency containing `unsafe` or native code; a new required user concept |
| T4 | 10 | New layout or container structure, identity or digest root, or cryptographic construction or primitive; any change to a frozen wire format. A frozen format may also be subject to R1 unless the decision is explicitly about revising that freeze. |

**Maintenance penalty.** A reader-path dependency that fails any of the following raises the candidate's tier by one, up to T4:

- a release within the last 24 months, or declared feature-complete with a stable normative spec;
- at least 2 people with publish rights, or ownership by an organization;
- no unresolved RUSTSEC advisory;
- MSRV at or below the workspace MSRV.

### 7.3 Minimum gains

The general rule is R6. The following specific rules apply on held-out-replicated results.

| Addition | Tier | Minimum gain (the CI must be `DISTINGUISHABLE` against the multiplied band) | Additional conditions |
|---|---|---|---|
| New baseline-set codec | T3 | Versus the best existing baseline-set configuration that meets the same profile constraints: `artifact_bytes` ratio beyond 1/(1+5×0.01), i.e. CI upper bound < 0.9523809524; **or** `decode_throughput_bps` ratio beyond 1+5×0.05, i.e. CI lower bound > 1.25. The other metric must not be `DISTINGUISHABLE_WORSE`. Holds in ≥ 2 applicable families with ≥ 2 groups each. | Meets the SPEC §5.8 criteria (R2): permissive licence compatible with redistribution, normative spec independent of any implementation, published test vectors, bounded declarable decode memory, two implementations or a tractable second one |
| New registered-extended codec | T2 | Same metrics at k = 3: CI upper bound < 0.9708737864 on size, or CI lower bound > 1.15 on decode throughput | Fails closed via `incompat` features (SPEC §5.8) |
| New reversible transform | T3 | Target families: `artifact_bytes` CI upper bound < 0.9523809524, including verification cost (I31) | Non-target families: `artifact_bytes` `EQUIVALENT` or better, and `encode_wall_s` not `DISTINGUISHABLE_WORSE` at k = 2, which charges detection cost |
| New record or section type | T3 | ≥ 5× band improvement on at least one primary metric of the decision's scope. Examples: `http_requests` −33% at ≥ 1 request (bands 1/(1+5×0.10) and a = 0.5); `artifact_bytes` −4.8%; `first_entry_latency_s` ratio < 1/(1+5×0.10) = 0.6667 | Exempt if it is the lowest-tier way to satisfy a hard constraint or a V1_BLOCKING requirement (R6) |
| New field, enum value or feature bit | T2 | ≥ 3× band on at least one primary metric | As above |
| New profile | T2 | On the new profile's primary objective, ≥ 3× band versus every existing profile under the new profile's own constraints, in ≥ 3 families | Simulated first-use "choose the right profile" task success ≥ 0.80 (§4.16). No T1 parameter change to an existing profile achieves ≥ 50% of the gain. |
| New dependency (addition or replacement) | Per §7.2 | Per the tier of the feature it enables | Licence rules (§7.5) |

### 7.4 No sunk-cost credit before v1

Until the v1 freeze, existing wire features and implemented code receive **no sunk-cost credit** for v1 inclusion. Their C1–C5 costs are counted as if they were new, because removing them before v1 is cheap. Migration cost for existing pre-v1 archives is recorded separately (C7) and considered under DEC-CON-013. After the v1 freeze, removing a frozen feature is itself a T4 change.

### 7.5 Licence and unsafe-code screens (R1/R2)

- **Workspace crates.** Candidates that add `unsafe` to workspace crates violate the repository freeze `unsafe_code = "forbid"` (Cargo.toml) and are eliminated at R1.
- **Copyleft.** A reader-path or writer-path dependency under a strong copyleft licence (GPL, LGPL, AGPL) is eliminated at R2, because it fails SPEC §5.8's permissive-licence criterion.
- **Weak copyleft and interpretation.** Weak copyleft (for example MPL-2.0), unusual licences, and licence interpretation beyond SPDX compatibility lists are routed to external review (§8.3).

### 7.6 Ledger recording

For every surviving candidate the ledger fields hold:

- `complexity_cost`: C1, C2, C6;
- `dependency_cost`: C3, C7;
- `security_cost`: C4;
- `wire_cost`: C1 wire items, C5.

Each field states the tier and cites the generating command output. Qualitative items carry evidence tags (§9).

---

## 8. Status rules and confidence

### 8.1 Statuses

| Status | Meaning |
|---|---|
| `DECIDED` | Every condition of §8.2 holds. |
| `INSUFFICIENT_EVIDENCE` | Any condition of §8.2 fails. `selected_decision` MAY hold a selection explicitly marked `provisional`, with its confidence. |
| `EXTERNAL_REVIEW_REQUIRED` | The internal route is complete as far as it can go, and correctness depends on independent expertise (§8.3). |

The ledger's `blocker_class` (EVIDENCE, HUMAN_PARTICIPANTS, EXTERNAL_REVIEW, PLATFORM) records *why* a decision is not decided and is independent of status.

### 8.2 Conditions for `DECIDED`

| # | Condition | Ledger fields |
|---|---|---|
| 1 | Complete candidate set (R0), with the completeness critic recorded | `candidate_set`, `history` |
| 2 | Every excluded candidate justified by rule id and evidence reference; R1 exclusions name the hard constraint and mechanical test | `candidate_exclusion_reasons` |
| 3 | Linked evidence: every experiment is pre-registered (§12), its raw data passes `verify-raw`, and every number follows §12.3 | `experiment_ids`, `raw_result_refs`, `normalized_result_refs` |
| 4 | Held-out validation passed (R5) for EMPIRICAL and HUMAN-FACING parts | `heldout_result_refs` |
| 5 | Counterevidence is non-empty: the strongest evidence or argument against the selection, the relevant negative results (§10), or the search procedure followed and "none found" | `counterevidence` |
| 6 | Sensitivity analysis done, with the bars of §6.6 met at the selected scope; FORMAL decisions write "not applicable" with a reason | `sensitivity_analysis` |
| 7 | Practical significance: every comparison supporting the selection is decision-grade `DISTINGUISHABLE`, or R10 was applied explicitly | `selected_decision`, `history` |
| 8 | Costs recorded with tier (§7.6) and the minimum-gain rule applied | `complexity_cost`, `dependency_cost`, `security_cost`, `wire_cost` |
| 9 | Rule followed: the history cites the `decision-method.md` commit SHA, the outcome of each of R0–R10, and all deviations | `history` |
| 10 | Scope, known limitations, a measurable reopen trigger (§11), external review requirement ("none", with reason, or a review reference), confidence of MEDIUM or higher, and a remaining-unknowns reference | `decision_scope`, `known_limitations`, `reopen_trigger`, `external_review_requirement`, `confidence`, `remaining_unknowns_ref` |
| 11 | No unresolved confirmation-bias trigger (§10.2) | `history` |
| 12 | No leakage event affecting the evidence (§5.7) | `history` |

A ledger validator enforces conditions 1–3, 5, 6 and 8–10 mechanically (§14). The remaining conditions are checked by the gate audit.

### 8.3 `EXTERNAL_REVIEW_REQUIRED`

Set this status when a decision's correctness depends on independent expertise the program cannot supply:

- cryptographic constructions and final primitive selection (SPEC §25.2; `docs/crypto-review-v1.md`);
- security proofs;
- licence interpretation beyond SPDX compatibility lists;
- registrations with external authorities (media type, magic number);
- human-participant evidence, where simulated evidence is insufficient for the claim being made.

**Internal adversarial review by program agents is not independent expert review.**

A dossier MUST exist, containing the question, the candidates, the internal evidence under this method, and specific review questions. The decision becomes `DECIDED` only after the review is received, dispositioned in the ledger, and §8.2 holds. A negative or partial review returns the decision to `INSUFFICIENT_EVIDENCE`.

### 8.4 Confidence scale

| Level | Criteria | Allowed status |
|---|---|---|
| HIGH | All §8.2 conditions hold. Every §6.6 test at pass. R5 passed with no attenuated verdict. No R10 choice made under `INCONCLUSIVE` comparisons. Every family in scope has ≥ 2 held-out groups. No confidence cap applies. At least two independent evidence lines for the central claim, such as a measurement plus a formal bound, or agreement between two environments. | `DECIDED` |
| MEDIUM | All §8.2 conditions hold, and one or two caps apply (list below) | `DECIDED` |
| LOW | Some §8.2 condition fails, or three or more distinct caps apply | `INSUFFICIENT_EVIDENCE` (provisional selections only) |

**Caps.** Each of the following caps confidence at MEDIUM:

- a stability test in the MEDIUM band (§6.6);
- selection changes under one global threshold scaling (§6.4);
- an attenuated held-out verdict (R5 c);
- R10 applied under `INCONCLUSIVE` comparisons;
- a compromise selection (R9 iii);
- a `post-unlock-fix` (§5.6);
- a `power_limit_confound` flag (§4.2);
- single-environment timing for a scope covering several platforms;
- a `PLATFORM_BLOCKED` part of the scope, with a rationale that the result is platform-independent;
- `SIMULATED` usability evidence used as primary;
- an amendment that changes the result (§0.2 item 2);
- contamination (L5) not yet resolved;
- an evidence line marked `imprecise` for a primary comparison (§4.6);
- a confirmation-bias trigger resolved by a disposition that accepted a residual risk.

### 8.5 Downgrades

A `DECIDED` decision returns to `INSUFFICIENT_EVIDENCE` when any of these happens:

- a leakage event affects its evidence;
- `verify-raw` fails on its raw data;
- regenerated numbers differ from the reported ones (§12.3);
- replication on new data contradicts it (a `DISTINGUISHABLE` verdict in the opposite direction);
- a reopen trigger fires that invalidates the evidence (§11).

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
- `EMULATED`: QEMU arm64, or an application-level network;
- `PLATFORM_BLOCKED`;
- `NOT_DECISION_GRADE`;
- `PRE_FREEZE`: tuning or validation evidence only;
- `HELDOUT`;
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
  - Research I–III and the research appendix, which are internal syntheses; the primary sources beneath them MUST be re-cited;
  - blogs, third-party benchmarks, Wikipedia, forums and vendor marketing;
  - anything written by a language model.
- **Never a source:** an agent's recollection. A claim an agent "knows" MUST be checked by opening the primary source at the cited location.
- **Citation format for evidence in decisions:**
  - source identity (title, number or identifier);
  - version, commit or publication date;
  - locator (section, page or line);
  - access date, for web sources;
  - for the program's unpublished inputs, path and SHA-256 as listed in PROGRESS.
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
   Examples of such outcomes: candidates that failed to beat the status quo, frozen parameters shown suboptimal, contradicted claims, workloads where incumbents win.
3. **Product claims** (README, website, SPEC §23-style claims) MUST cite the negative results that bound them.
4. **Families where the selected candidate loses** at R9 or R5 are reported by name, with magnitudes produced per §12.3.

### 10.2 Confirmation-bias triggers

| ID | Trigger | Action |
|---|---|---|
| CB-1 | Status-quo drift: in a cluster with ≥ 6 decided or provisional decisions, the R10 status-quo key decides ≥ 50% of them | Power review: for each affected decision, report CI half-widths against band half-widths. Where the median half-width exceeds the band half-width, the experiment is underpowered, and the decision cannot become `DECIDED` until rounds or items increase. |
| CB-2 | The SPEC-proposed or status-quo candidate wins ≥ 90% of ≥ 10 decisions in a cluster | Independent re-analysis of 3 of those decisions, drawn with a recorded seed |
| CB-3 | Candidates, metrics, items, primary comparisons, weights or analysis changed after results were visible, outside the §0.2 or §5.2 procedures | The affected analysis is re-run as originally registered, and both analyses are reported |
| CB-4 | Marginal win: the CI bound nearest the band edge lies within 0.5 × `log1p(r)` (or 0.5 × a) of the edge, on a comparison that supports the selection | Independent re-analysis. Confidence at most MEDIUM until dispositioned. |
| CB-5 | Exclusion sensitivity: a verdict changes when invalid samples or excluded items are included | Report both. Independent re-analysis. |
| CB-6 | Replication failure: validation reverses tuning, or held-out is attenuated or reversed | Recorded as counterevidence. For held-out, per R5. |
| CB-7 | Self-review: the only analysis supporting the winner came from the agent or session that proposed the winning candidate | Independent re-analysis |
| CB-8 | `counterevidence` is empty for a decision with ≥ 3 candidates | The decision cannot be `DECIDED`. A documented adversarial search is required. |

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
research/raw/<EXP-ID>/          raw JSONL (hash-chained) + meta, produced by `ebr run`
research/normalized/<EXP-ID>/   CSV + summary.json, produced by `ebr normalize`
research/reports/{tables,figures}/<EXP-ID>/  produced by `ebr report`
```

Decision-analysis outputs (§14) are written under `research/normalized/<EXP-ID>/decision/`, with the same provenance header convention as ebr outputs.

### 12.2 `record.md` template

```markdown
# <EXP-ID>: <title>

## Pre-registration
- decision_ids: [DEC-...]
- decision type: EMPIRICAL | FORMAL | HUMAN-FACING (+ EXTERNAL dossier ref)
- method: research/decision-method.md @ <commit sha>
- pre-registration commit: <filled by the commit that adds this file; runs must descend from it>
- author (agent/session) and independence from candidate designers: <...>

## Question and hypotheses
- question: <one sentence>
- H1 (directional, with predicted effect size in band units): <...>
- H0 / equivalence hypothesis: <...>
- what result would falsify H1: <...>

## Candidates
| candidate_id | exact definition (commit, flags, versions, params) | provisional cost tier | applicable families |
- reference candidate: <id> (normally status quo)
- candidates excluded before execution and why (rule id): <...>

## Corpus
- splits used: tuning | validation (look #1 or #2 of §5.2) | heldout (post-unlock only, unlock sha)
- items: list of item_id with logical_tree_sha256, or manifest sha256 + selection rule
- families and minimum-support check (§4.9)
- generated items: generator, params, seed, sha256

## Metrics
| metric | role (primary/secondary/diagnostic) | family | unit | direction | threshold ID (§3.2) | measurement source |

## Primary comparisons
- enumerated list (candidate pair, metric, env, level); m = <n>; per-comparison confidence = <0.99 | 1-0.05/m>; resamples = <B>

## Protocol
- env_id(s); timing: true|false; affinity configuration (§4.2/§4.3); cache state (§4.5)
- warmups; minimum rounds, step, cap (§4.6); operations per round (latency/remote)
- guard limits (thresholds.json), max_wait_s, cooldown, canary
- network profile(s) (§4.15) or usability task set (§4.16), if applicable
- A/A calibration experiment id covering this env and these families (§4.7)
- seeds: order, bootstrap, command

## Analysis plan
- aggregation levels and weights (§4.9); applicability
- R4 dominance scopes; R9 candidate partitions and their expressibility
- base weights for sensitivity (§6.3 / Appendix B); scenario sets used
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

1. **Scope.** Every number reported as a result or used in a decision or claim is covered. That includes ledger fields, reports, PROGRESS result notes, README and website claims, and commit messages that state results. Each such number MUST be produced by a checked-in command from raw artifacts, via the chain `ebr run`, `ebr verify-raw`, `ebr normalize`, `ebr report`, decision tooling (§14).
2. **Citation.** Each cited number gives the output file, the row key (and column), and the generating command recorded in that output's provenance header.
3. **No manual numbers.** Numbers MUST NOT be hand-typed from memory, computed by an agent's own arithmetic, or copied from terminal scrollback.
   - Prose may round a generated value to 3 significant figures (round half to even), or keep fewer digits than the table shows. The generated table value remains authoritative.
   - Derived quantities in prose, such as percentages or differences, MUST themselves be generated.
4. **Regeneration.** `normalize`, `report` and the decision tooling MUST regenerate byte-identical outputs from the same raw data and tool commit, as `research/experiments/EXP-SMOKE-000/run_smoke.sh` demonstrates. A mismatch invalidates the dependent numbers until resolved (§8.5).
5. **Figures** come only from `ebr report` or the decision tooling.
6. **Not results.** Numbers taken from external sources carry `EXTERNALLY_SOURCED` citations. Pre-registered thresholds in this document are method choices. Bookkeeping counts (for example corpus or ledger sizes) cite their command where practical, as in §0.1 and §4.9.

---

## 13. Execution budget reality

Experiments are **compute-heavy** and **agent-budget-light** by design. Agent tokens are the scarce resource: this program has been interrupted twice by usage limits (PROGRESS, 2026-09-13 and 2026-09-16). Machine time is comparatively plentiful.

- **Scripted execution.** Experiments run as scripts: ebr specs executed in the background, resumable, with raw output committed at checkpoints. Agents do not babysit runs, poll verbosely, or read raw JSONL. They read `summary.json`, generated tables and failure lists.
- **Agent judgment is reserved for:** candidate-set construction, experiment design and pre-registration records, analysis and interpretation of generated outputs, adversarial review, and ledger decisions.
- **Mechanical stages** use the cheaper model, per the PROGRESS 2026-09-16 pacing decision: provisioning, verification, harness boilerplate, experiment execution, bulk ledger field updates by script.
- **Budget never weakens the method.** If budget prevents executing the protocol, the decision stays `INSUFFICIENT_EVIDENCE`, and the gap is recorded as a deviation and a limitation. It is never allowed to:
  - cut repetitions below the §4.6 minimums;
  - skip A/A calibration, held-out validation or sensitivity analysis;
  - substitute agent estimates for measurements.
- **Batching.** Experiments SHOULD share runs where specs allow, for example one pack pass that measures size, determinism and section accounting together. Deterministic metrics (§4.13) SHOULD precede timing, because they need only 2 repetitions and often eliminate candidates (R1, R2, R4) before any expensive timing campaign.
- **Timing windows** are exclusive (§4.2) and are scheduled when no agent workflow runs, which suits overnight unattended execution.

---

## 14. Integration prerequisites

No verdict is decision-grade until each item below exists, is tested (unit tests plus golden files), and is committed. All items are currently `[UNRESOLVED]`.

| # | Item | Needed by |
|---|---|---|
| 1 | `research/methods/thresholds.json` filled per §3.7 and Appendix A, and accepted by the ebr test suite | every analysis |
| 2 | Decision-analysis tooling (for example `research/tools/decide/`): metric-level thresholds and absolute floors with snapping; profile-specific bands; L2–L4 aggregation; family-stratified cluster bootstrap; three-way verdicts; Bonferroni confidence and resamples; unanimity rule; R4 dominance including cost tier; minimum-gain multipliers; band-unit utility, regret and R9 partitions; §6 sensitivity (weights, thresholds, LOFO, family Dirichlet, scenarios) | R4–R10 |
| 3 | Runner additions: AC-power check; Windows power-throttling opt-out; SMT sibling verification for affinity sets; inter-sample cooldown; drift canary; host-side guard sampler for WSL runs; invalid-rate balance check; `vm-cold` cache mode; clean-worktree and pre-registration-ancestry check for decision specs; spec lint rejecting `thresholds_override` and sensitivity-sample overrides in decision specs | decision-grade timing |
| 4 | A/A calibration experiments per environment and family (§4.7) | decision-grade timing and memory |
| 5 | Held-out lock frozen (§5.3); consistency checker for `design-freeze.json` and `heldout-unlock.json` (§5.5) | before the first decision-relevant result; freeze |
| 6 | Ledger validator for `DECIDED` rows (§8.2) and confidence caps (§8.4) | status changes |
| 7 | Cost-accounting scripts for C1–C3 and the C4 enumeration template (§7.1) | R6 |
| 8 | Outcome records and negative-results register (§10.1) | every experiment |
| 9 | Rule simulation on synthetic and pilot (non-decision) outcome tables showing tie, noise and conflicting-family behaviour (DEC-ECO-078 required evidence) | adoption of this method as DEC-ECO-078's resolution |

---

## Appendix A. Machine-readable transcription (proposed `thresholds.json`)

This is the exact content proposed for `research/methods/thresholds.json`. It was checked against ebr v1: `ebr.thresholds.Thresholds.load` accepts it, family bands and epsilons resolve, and family `other` raises `ThresholdUnset` as intended. The revision step transcribes it after review changes. If this appendix and the sections it cites disagree, the sections govern.

```json
{
  "schema": "ebr.thresholds.v1",
  "status": "pre-registered",
  "authority": "research/decision-method.md is authoritative; values transcribed from it with section citations.",
  "families": {
    "time_wall": {
      "practical_significance_band": {"kind": "ratio", "lower": 0.9523809524, "upper": 1.05},
      "pareto_epsilon": {"kind": "relative", "value": 0.05},
      "absolute_floor": {"unit": "s", "process": 0.005, "in_process": 0.0001},
      "reference_metric": "decode_wall_s",
      "decision_method_section": "3.2 T-04; 3.7"
    },
    "time_cpu": {
      "practical_significance_band": {"kind": "ratio", "lower": 0.9523809524, "upper": 1.05},
      "pareto_epsilon": {"kind": "relative", "value": 0.05},
      "absolute_floor": {"unit": "s", "process": 0.005, "in_process": 0.0001},
      "reference_metric": "decode_cpu_s",
      "decision_method_section": "3.2 T-05; 3.7"
    },
    "memory_peak": {
      "practical_significance_band": {"kind": "ratio", "lower": 0.9090909091, "upper": 1.10},
      "pareto_epsilon": {"kind": "relative", "value": 0.10},
      "absolute_floor": {"unit": "B", "rss_or_commit": 4194304, "allocator": 1048576},
      "reference_metric": "peak_rss_bytes",
      "decision_method_section": "3.2 T-06; 3.7"
    },
    "scratch_peak": {
      "practical_significance_band": {"kind": "ratio", "lower": 0.9090909091, "upper": 1.10},
      "pareto_epsilon": {"kind": "relative", "value": 0.10},
      "absolute_floor": {"unit": "B", "value": 16777216},
      "reference_metric": "scratch_peak_bytes",
      "decision_method_section": "3.2 T-07; 3.7"
    },
    "size": {
      "practical_significance_band": {"kind": "ratio", "lower": 0.9900990099, "upper": 1.01},
      "pareto_epsilon": {"kind": "relative", "value": 0.01},
      "absolute_floor": {"unit": "B", "value": 64},
      "reference_metric": "artifact_bytes",
      "decision_method_section": "3.2 T-01; 3.7"
    },
    "throughput": {
      "practical_significance_band": {"kind": "ratio", "lower": 0.9523809524, "upper": 1.05},
      "pareto_epsilon": {"kind": "relative", "value": 0.05},
      "absolute_floor": null,
      "reference_metric": "decode_throughput_bps",
      "decision_method_section": "3.2 T-05b; 3.7"
    },
    "io": {
      "practical_significance_band": {"kind": "ratio", "lower": 0.9090909091, "upper": 1.10},
      "pareto_epsilon": {"kind": "relative", "value": 0.10},
      "absolute_floor": {"unit": "count", "value": 64},
      "reference_metric": "fs_read_ops",
      "role": "secondary-only",
      "decision_method_section": "3.2 T-19; 3.7"
    },
    "correctness": {
      "practical_significance_band": {"kind": "absolute", "lower": 0, "upper": 0},
      "pareto_epsilon": {"kind": "absolute", "value": 0},
      "decision_method_section": "3.3; 3.7"
    },
    "other": {
      "practical_significance_band": {"kind": null, "lower": null, "upper": null},
      "pareto_epsilon": {"kind": null, "value": null},
      "note": "intentionally null: metric_thresholds governs (decision-method.md 3.6)",
      "decision_method_section": "3.6"
    }
  },
  "bootstrap": {
    "confidence": 0.99,
    "resamples": 20000,
    "fwer_alpha": 0.05,
    "max_comparisons_at_base_confidence": 5,
    "min_replicates_per_tail": 100,
    "decision_method_section": "4.10; 4.12"
  },
  "quiet_machine_guard": {
    "max_loadavg_1m": 32.0,
    "max_other_cpu_percent": 5.0,
    "min_max_wait_s_decision_specs": 300,
    "decision_method_section": "4.4"
  },
  "sensitivity": {
    "min_fraction_preserving_winner": 0.90,
    "medium_fraction_preserving_winner": 0.80,
    "dirichlet_samples": 10000,
    "dirichlet_concentration": 4.0,
    "grid_factors": [0.5, 0.75, 1.0, 1.25, 1.5],
    "grid_full_max_criteria": 8,
    "threshold_scale_factors": [0.5, 2.0],
    "workload_dirichlet_samples": 10000,
    "workload_dirichlet_concentration": 4.0,
    "lofo_min_fraction": 0.90,
    "scenario_in_family_weight": 0.8,
    "max_compromise_regret_band_units": 2.0,
    "decision_method_section": "6.2-6.6; 2 R9"
  },
  "protocol": {
    "deterministic_repetitions": 2,
    "timing_rounds": {
      "small": {"min": 10, "step": 10, "cap": 30},
      "medium": {"min": 10, "step": 10, "cap": 20},
      "large": {"min": 5, "step": 5, "cap": 15}
    },
    "adaptive_target_halfwidth_fraction_of_log1p_r": 0.5,
    "warmup_rounds": {"small": 2, "medium": 2, "large": 1},
    "warmup_trend_kendall_tau_limit": 0.3,
    "min_operations_per_round_for_p95": 40,
    "min_simulated_sessions_per_task_candidate": 20,
    "cooldown_s": {"min": 2, "max": 60, "rule": "min(max, max(min, previous_sample_wall_s))"},
    "drift_canary": {"baseline_runs": 3, "every_blocks": 10, "every_s": 300, "max_ratio": 1.05, "pause_s": 120},
    "invalid_sample_max_fraction_cell": 0.10,
    "invalid_rate_max_difference_pp": 5,
    "excluded_items_max_fraction": 0.20,
    "power_limit_confound": {"wall_ratio": 10, "min_wall_s": 20},
    "aa_calibration": {"max_item_distinguishable_fraction": 0.02, "max_invalid_fraction": 0.10, "min_families": 10},
    "min_support": {"corpus_groups": 10, "corpus_families": 5, "family_groups": 2},
    "validation_looks_per_decision": 2,
    "decision_method_section": "4.2-4.9; 5.2"
  },
  "cost_tiers": {
    "multipliers": {"T0": 1, "T1": 2, "T2": 3, "T3": 5, "T4": 10},
    "maintenance": {"max_months_since_release": 24, "min_publishers": 2},
    "decision_method_section": "7.2; 7.3"
  },
  "metric_thresholds": {
    "artifact_bytes": {"id": "T-01", "family": "size", "unit": "B", "direction": "minimize", "relative": 0.01, "absolute": 64},
    "metadata_bytes": {"id": "T-02", "family": "size", "unit": "B", "direction": "minimize", "relative": 0.05, "absolute": 64, "role_default": "diagnostic"},
    "index_bytes": {"id": "T-02", "family": "size", "unit": "B", "direction": "minimize", "relative": 0.05, "absolute": 64, "role_default": "diagnostic"},
    "dictionary_bytes": {"id": "T-02", "family": "size", "unit": "B", "direction": "minimize", "relative": 0.05, "absolute": 64, "role_default": "diagnostic"},
    "side_data_bytes": {"id": "T-02", "family": "size", "unit": "B", "direction": "minimize", "relative": 0.05, "absolute": 64, "role_default": "diagnostic"},
    "metadata_bytes_per_entry": {"id": "T-02b", "family": "size", "unit": "B/entry", "direction": "minimize", "relative": 0.05, "absolute": 0.5},
    "encode_wall_s": {"id": "T-03", "family": "time_wall", "unit": "s", "direction": "minimize", "relative": 0.05, "relative_by_profile": {"fast": 0.05, "balanced": 0.05, "dense": 0.10, "extreme": 0.25}, "absolute": {"process": 0.005, "in_process": 0.0001}},
    "decode_wall_s": {"id": "T-04", "family": "time_wall", "unit": "s", "direction": "minimize", "relative": 0.05, "absolute": {"process": 0.005, "in_process": 0.0001}},
    "encode_cpu_s": {"id": "T-05", "family": "time_cpu", "unit": "s", "direction": "minimize", "relative": 0.05, "relative_by_profile": {"fast": 0.05, "balanced": 0.05, "dense": 0.10, "extreme": 0.25}, "absolute": {"process": 0.005, "in_process": 0.0001}},
    "decode_cpu_s": {"id": "T-05", "family": "time_cpu", "unit": "s", "direction": "minimize", "relative": 0.05, "absolute": {"process": 0.005, "in_process": 0.0001}},
    "decode_throughput_bps": {"id": "T-05b", "family": "throughput", "unit": "B/s", "direction": "maximize", "relative": 0.05, "absolute": null},
    "encode_throughput_bps": {"id": "T-05b", "family": "throughput", "unit": "B/s", "direction": "maximize", "relative": 0.05, "relative_by_profile": {"fast": 0.05, "balanced": 0.05, "dense": 0.10, "extreme": 0.25}, "absolute": null},
    "verify_throughput_bps": {"id": "T-05b", "family": "throughput", "unit": "B/s", "direction": "maximize", "relative": 0.05, "absolute": null},
    "peak_rss_bytes": {"id": "T-06", "family": "memory_peak", "unit": "B", "direction": "minimize", "relative": 0.10, "absolute": 4194304},
    "peak_commit_bytes": {"id": "T-06", "family": "memory_peak", "unit": "B", "direction": "minimize", "relative": 0.10, "absolute": 4194304},
    "alloc_peak_bytes": {"id": "T-06", "family": "memory_peak", "unit": "B", "direction": "minimize", "relative": 0.10, "absolute": 1048576},
    "scratch_peak_bytes": {"id": "T-07", "family": "scratch_peak", "unit": "B", "direction": "minimize", "relative": 0.10, "absolute": 16777216},
    "first_entry_latency_s": {"id": "T-08", "family": "time_wall", "unit": "s", "direction": "minimize", "relative": 0.10, "absolute": {"process": 0.005, "in_process": 0.0001}},
    "random_entry_latency_s": {"id": "T-08", "family": "time_wall", "unit": "s", "direction": "minimize", "relative": 0.10, "absolute": {"process": 0.005, "in_process": 0.0001}},
    "access_granularity_bytes": {"id": "T-09", "family": "size", "unit": "B", "direction": "minimize", "relative": 0.25, "absolute": 65536},
    "remote_first_entry_latency_s": {"id": "T-10", "family": "time_wall", "unit": "s", "direction": "minimize", "relative": 0.10, "absolute_rule": "max(0.010, 0.5*rtt_s)"},
    "remote_random_entry_latency_s": {"id": "T-10", "family": "time_wall", "unit": "s", "direction": "minimize", "relative": 0.10, "absolute_rule": "max(0.010, 0.5*rtt_s)"},
    "http_bytes": {"id": "T-11", "family": "size", "unit": "B", "direction": "minimize", "relative": 0.05, "absolute": 16384},
    "http_requests": {"id": "T-12", "family": "other", "unit": "count", "direction": "minimize", "relative": 0.10, "absolute": 0.5},
    "verify_overhead_pp": {"id": "T-13", "family": "other", "unit": "pp", "direction": "minimize", "relative": null, "absolute": 5.0},
    "parallel_speedup": {"id": "T-14", "family": "other", "unit": "ratio", "direction": "maximize", "relative": 0.10, "absolute": 0.2},
    "blast_logical_bytes_p95": {"id": "T-15", "family": "other", "unit": "B", "direction": "minimize", "relative": 1.0, "absolute": 1048576},
    "blast_logical_bytes_max": {"id": "T-15", "family": "other", "unit": "B", "direction": "minimize", "relative": 1.0, "absolute": 1048576},
    "blast_entries_p95": {"id": "T-15", "family": "other", "unit": "count", "direction": "minimize", "relative": 1.0, "absolute": 0.5},
    "padding_overhead_pp": {"id": "T-16", "family": "other", "unit": "pp", "direction": "minimize", "relative": null, "absolute": 1.0, "absolute_secondary": {"padding_bytes": 4096}},
    "leak_min_entropy_bits": {"id": "T-17", "family": "other", "unit": "bits", "direction": "minimize", "relative": null, "absolute": 1.0},
    "leak_shannon_bits": {"id": "T-17", "family": "other", "unit": "bits", "direction": "minimize", "relative": null, "absolute": 1.0},
    "anonymity_set_median": {"id": "T-17", "family": "other", "unit": "count", "direction": "maximize", "relative": 1.0, "absolute": 0.5},
    "fingerprint_attack_success": {"id": "T-17", "family": "other", "unit": "fraction", "direction": "minimize", "relative": null, "absolute": 0.05},
    "task_success_rate": {"id": "T-18", "family": "other", "unit": "fraction", "direction": "maximize", "relative": null, "absolute": 0.15},
    "task_steps": {"id": "T-18", "family": "other", "unit": "count", "direction": "minimize", "relative": 0.25, "absolute": 0.5},
    "task_errors": {"id": "T-18", "family": "other", "unit": "count", "direction": "minimize", "relative": null, "absolute": 0.5},
    "fs_read_ops": {"id": "T-19", "family": "io", "unit": "count", "direction": "minimize", "relative": 0.10, "absolute": 64, "role_default": "secondary-only"},
    "fs_write_ops": {"id": "T-19", "family": "io", "unit": "count", "direction": "minimize", "relative": 0.10, "absolute": 64, "role_default": "secondary-only"}
  },
  "metric_thresholds_semantics": "band = [1/(1+relative), 1+relative] when relative is set; difference band = [-absolute, +absolute]; composite rule, floor snapping and epsilon = band per decision-method.md 3.1; decision_method_section for every entry is 3.2 (row id)"
}
```

---

## Appendix B. Base weights and scenario family sets

### B.1 Profile base weights

These weights apply to profile-level decisions (R8 and R9 utility, §6.3). They are `[INFERRED]` from the intents stated in SPEC §6.1–6.4 and from the constrained objective of SPEC §5.4 (minimize stored bytes, subject to declared decode bounds). The sensitivity analysis (§6) exists so that conclusions do not hinge on these exact values. Declared profile *bounds* (SPEC §6.0a) are constraints (R2), not weights.

| Criterion (metric) | fast | balanced | dense | extreme |
|---|---|---|---|---|
| `artifact_bytes` | 0.15 | 0.30 | 0.45 | 0.60 |
| `encode_wall_s` | 0.40 | 0.20 | 0.10 | 0.05 |
| `decode_wall_s` | 0.25 | 0.25 | 0.20 | 0.15 |
| decode memory (`alloc_peak_bytes`, or the platform primary of §4.14) | 0.10 | 0.15 | 0.15 | 0.10 |
| `access_granularity_bytes` | 0.10 | 0.10 | 0.10 | 0.10 |
| Sum | 1.00 | 1.00 | 1.00 | 1.00 |

All weights are strictly positive, as `stats.weight_sensitivity` requires for Dirichlet concentration.

### B.2 Workload scenarios (§6.5)

The scenarios are `[INFERRED]` from the SPEC §6 "use when" statements and the corpus families (`research/corpus/methodology.md` §2).

| Scenario | Families (0.8 of the weight, uniform) | Source of intent |
|---|---|---|
| SC-DIST: software distribution and release artifacts | F01, F02, F03, F09, F11, F14, F18 | SPEC §6.3 |
| SC-CI: CI build artifacts and intermediate data | F02, F03, F04, F05, F09, F17 | SPEC §6.1 |
| SC-BACKUP: backup, cold storage, long-term archival | F04, F05, F08, F15, F16, F17, F18, F19 | SPEC §6.4 |
| SC-DATA: data exchange and scientific data | F06, F07, F08, F10 | corpus families F06–F08, F10 |
| SC-MEDIA: media collections | F11, F12, F13 | corpus families F11–F13 |

- The remaining 0.2 of the weight is spread uniformly over the other applicable families.
- F20 (adversarial and high-entropy inputs) is in no scenario. It carries base weight in the equal-weight corpus aggregate and is the main screening family for R1 and R2.

---

## Revision history

| Date | Version | Change |
|---|---|---|
| 2026-09-16 | round 0 (pre-registration draft) | Initial draft written before any decision-relevant experiment result existed (§0.1). Awaiting adversarial review (`research/methods/method-review-round1.md`), revision, and transcription into `research/methods/thresholds.json`. |
