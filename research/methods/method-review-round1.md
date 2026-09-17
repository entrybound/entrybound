# Method Review, Round 1: Archetypal Objective and Decision Method

| Field | Value |
|---|---|
| Review | Adversarial method review, round 1 (program Phase A method gate). |
| Date | 2026-09-16 |
| Subjects | `research/archetypal-objective.md` SHA-256 `f9aea31ed3dc8a4cd78908006212447229c5b36bb2640b61c15cc00575b1de37`; `research/decision-method.md` SHA-256 `ca23c8a2389a5c662425a1be20b4dc736807c7a8e8aefde84ebaaf7cc0caacfa` |
| Inputs consulted | SPEC `design/2026-08-29-entrybound-product-architecture.md` `1f881c7b…fe29c` (§3.9, Appendix A, §5.4-§5.8, §7.6, §9.8, §22.1, §23.4-§23.5, §25.3); `research/decision-ledger.jsonl` `98b5401d…068b` (by script only); `research/tools/ebr/stats.py` `309c92ac…5f25`; `research/tools/ebr/{guard,normalize,spec,run}.py`; `research/corpus/manifest.json` `8c4e4cc6…988b5c494` (split, family and group fields only); `research/methods/thresholds.json` `56c6cbb8…fd31`; `CONTRIBUTING.md`; `docs/format-v0.md`; `docs/random-access-v1.md`; `research/PROGRESS.md`. Repository HEAD at review: `6bef329948d03f217fa13f93e4d4ca16aec31112`. |
| Reviewer independence | A separate agent session. It did not draft either subject document, any candidate or any threshold. It shares a base model with the drafting sessions, so it is not an independent expert review in the sense of `decision-method.md` §8.3. |
| Held-out disclosure | While checking the git state, `git status` showed untracked file names under `research/corpus/fingerprints/`. One of them contains a held-out `item_id`. No held-out content, listing, statistic, path under `/root/eb-research/heldout` or fingerprint body was opened. Held-out counts below come from the `split`, `family` and `independence_group` fields, which `decision-method.md` §5.4 permits. Occurrence counts of the string "heldout" in corpus files were taken without printing any matched line. This is recorded so the §5.8 audit can account for it (see G01). |
| Verdict | **NOT READY TO PRE-REGISTER.** 85 findings: 10 BLOCKER, 36 HIGH, 33 MEDIUM, 6 LOW. The revision must address every BLOCKER and HIGH finding, and must disposition every finding, before the revised commit can serve as the pre-registration record. |

## 0. How to use this review

### 0.1 Severity scale

| Severity | Meaning | Required disposition |
|---|---|---|
| BLOCKER | A rule is contradictory, cannot be executed, or lets a decision reach `DECIDED` on invalid grounds at scale. | Fix before the pre-registration commit. It cannot be accepted as a known risk. |
| HIGH | Material bias, leakage, gameability or missing coverage that would distort many decisions. | Fix in the round-1 revision, or reject with a written counterargument that round 2 checks. |
| MEDIUM | A localized defect, vagueness or inconsistency. | Fix, or record an accepted-risk rationale in the revision history. |
| LOW | Editorial, clarity or bookkeeping. | Fix when convenient. |

### 0.2 Disposition protocol

The revision adds a table named `Round-1 dispositions` to each subject document's revision history. Each row gives the finding ID, the disposition (`FIXED` with section references, `REJECTED` with argument, or `ACCEPTED_RISK` for MEDIUM and LOW only), and the commit. Round 2 re-reviews every `REJECTED` and `ACCEPTED_RISK` row and samples `FIXED` rows.

### 0.3 Evidence conventions

Evidence lines cite a section or a script in Appendix R. `[DERIVED]` marks deduction from the cited texts. `[COMPUTED]` marks output of a command in Appendix R. Simulations in Appendix R are method checks, not experiments under `decision-method.md` §4.

### 0.4 What is sound

Future edits should keep these properties:

- Lexicographic hard constraints ahead of trade-offs.
- Precision-based, not significance-based, adaptive stopping.
- Randomized blocks with within-round pairing.
- The refusal to widen bands after A/A failure.
- The explicit `EQUIVALENT`/`INCONCLUSIVE` split.
- Hash-chained raw data and the number-provenance rule.
- The negative-results obligation.
- The confirmation-bias trigger table.
- The decision to run every frozen candidate on held-out, not only the winner.

---

## Summary of findings

| ID | Sev. | Title |
|---|---|---|
| A01 | BLOCKER | Rule order is defined twice and inconsistently; precedence between the documents does not cover it |
| A02 | BLOCKER | I24 is handled three incompatible ways (method R1, objective HC-14, SPEC §7.6/§9.8) |
| A03 | BLOCKER | Library-version-defined decode steps: eliminated by method R2, admitted by objective HC-11/HC-12 |
| A04 | BLOCKER | Freeze exception, T4 "change a frozen wire format" and "no sunk-cost credit" contradict HC-16, objective §1.4 and CONTRIBUTING freezes |
| A05 | HIGH | Profile utility contradicts SPEC §5.4 constrained objective and SPEC §25.3 fitted weights |
| A06 | HIGH | Parameter sets differ between the documents (network profiles, threads, repetitions, blast radius, leakage, scenarios, tiers) |
| A07 | HIGH | OD-13 speedup compares affinity configurations that §4.2 forbids comparing |
| B01 | BLOCKER | No mapping from 989 free-text ledger constraints to HC or freeze IDs; HC applicability per decision is uncomputable |
| B02 | HIGH | Missing HC: input consistency during pack (SPEC §5.7a) |
| B03 | HIGH | Missing HC: CPU and algorithmic-complexity bounds on hostile input |
| B04 | HIGH | HC-06 oracle uses noise bands, process RSS and an uncomputed H |
| B05 | HIGH | Intermittent nondeterminism escapes through the "confirming rerun" rule |
| B06 | HIGH | HC-05 escape oracle relies on self-reported traces and an i.i.d. race model |
| B07 | HIGH | HC-03/HC-12 triage lets the program attribute every disagreement to the independent reader |
| B08 | HIGH | MVT-12(b) traceability keys on RFC 2119 keywords the SPEC almost never uses |
| B09 | MEDIUM | HC-13 cross-host and recreated-tree determinism is ill-posed without fixed metadata inputs |
| B10 | MEDIUM | HC-11 variation matrix is not verified to take effect (locale, CPU features, cgroup memory) |
| B11 | MEDIUM | HC-02 round-trip matrix has no path-coverage obligation |
| B12 | MEDIUM | Instrumented research builds serve as HC oracles without equivalence to production |
| B13 | MEDIUM | HC-15 fault-class matrix lacks a precedence table for ambiguous injections |
| B14 | MEDIUM | HC-14 statement items with no MVT (key commitment, constant time, secret hygiene, downgrade) |
| B15 | MEDIUM | HC-16 frozen-ID regression covers generated seeds only |
| B16 | MEDIUM | Writer crash consistency and HTTP origin safety are not hard constraints |
| B17 | MEDIUM | HC-10(d) cross-producer AUX equality may conflict with provenance in AUX |
| B18 | LOW | Probabilistic MVTs lack the detection bounds §2.0(2) requires |
| B19 | MEDIUM | L0 and R1(b) eliminations need no independent reviewer |
| C01 | MEDIUM | Appendix A and its reverse check disagree with the §2.1 table |
| C02 | MEDIUM | Invariant clauses mapped but not tested (I7, I10, I11, I20, I22, I23) |
| C03 | LOW | Appendix C.1 counts only bare `I<n>` tokens |
| D01 | BLOCKER | Most OD metrics have no canonical name, band or role; later thresholds would be set with results visible |
| D02 | HIGH | Wrong-direction and gameable metrics (M22.4, M21.3, M24.2, M04.3, M23.1, M01.1, M21.2) |
| D03 | HIGH | OD-03 headline can be raised by declaring fewer classes restorable |
| D04 | HIGH | M08.3 bounded-memory exponent has no threshold and is biased toward 0 |
| D05 | MEDIUM | Vague metric definitions (M05.3, M10.2, M14.1, M15.2, M16.2, M19.2, M25.1, M26.1) |
| D06 | HIGH | "Serves a CC" is undefined for combinations no incumbent serves, and the 1% band contradicts SPEC §23.4 |
| D07 | MEDIUM | Missing dimensions (scale limits, byte-weighted totals, reader fragmentation, crash consistency) |
| D08 | MEDIUM | Equal-group geometric aggregation hides large-tier regressions; tiers unused in rules |
| D09 | MEDIUM | Equal weights per primary metric make weight a function of how many metrics are declared |
| E01 | HIGH | Composite "EQUIVALENT if either" plus 5 ms process floors hides large relative slowdowns |
| E02 | HIGH | T-15 blast-radius floor (1 MiB) and band erase the chunk and group design space |
| E03 | HIGH | Minimum-gain multipliers are linear in r and infeasible for bounded absolute metrics |
| E04 | MEDIUM | T-01 64 B floor is inconsistent with 1% materiality on small archives |
| E05 | MEDIUM | T-09 and T-11 floors disagree; T-11 rationale not reproduced by the emulator |
| E06 | MEDIUM | Polled scratch cannot support zero-scratch binary claims |
| E07 | MEDIUM | T-18 simulated sessions are not independent samples |
| E08 | LOW | A/A criterion 2 (at most 2% distinguishable items) is loose and underpowered |
| E09 | MEDIUM | R9(iii) compromise bound of 2.0 weighted band units is unbounded per metric |
| F01 | BLOCKER | Family-stratified cluster bootstrap undercovers badly (nominal 0.99, simulated 0.91) |
| F02 | HIGH | Median-of-ratios percentile bootstrap at Bonferroni confidence over about 10 rounds degenerates |
| F03 | HIGH | A/A calibration cannot detect binary-layout, Defender-reputation or scheduler-interaction bias, or lack of power |
| F04 | HIGH | Laptop power-limit and thermal confounds are not controlled for multi-threaded or long samples |
| F05 | HIGH | Quiet-machine guard limits are too loose for a 5% band (32-CPU average; loadavg 32) |
| F06 | HIGH | WSL2 timing has no VIRTUALIZED qualifier or cap; the cross-environment claim rule is weak |
| F07 | HIGH | Defender format- and reputation-dependent scanning biases incumbent comparisons |
| F08 | HIGH | Bonferroni scheme collapses power; "not DISTINGUISHABLE_WORSE" treats absence of evidence as absence of harm |
| F09 | MEDIUM | Floor snapping on point estimates is undefined inside bootstrap replicates |
| F10 | MEDIUM | Remote emulation has no pre-registered calibration acceptance criteria |
| G01 | BLOCKER | Held-out identities, sources, generator inputs and paths are committed and readable by every agent |
| G02 | HIGH | Cross-split upstream sharing is invisible to the independence-group validator |
| G03 | HIGH | Design-freeze and unlock granularity (single or waves) is undefined |
| G04 | HIGH | Validation second look and cross-decision reuse are not confirmatory |
| G05 | HIGH | Post-unlock correctness fixes may still reach `DECIDED`, contradicting L2 |
| G06 | MEDIUM | Freeze commit omits harness, runner, toolchain, images and thresholds |
| G07 | MEDIUM | Held-out access control is advisory only |
| G08 | LOW | Relock replacement criteria are "pre-declared" but not declared |
| H01 | HIGH | Quantized utilities with ties counted as preserved make weight sensitivity pass trivially |
| H02 | HIGH | No sampling-uncertainty sensitivity of the selection |
| H03 | MEDIUM | Weight perturbation explores only a neighbourhood of inferred weights |
| H04 | MEDIUM | Threshold-scaling consequence ("narrower scope") does not address what the test detects |
| H05 | MEDIUM | LOFO over about 20 families with a 0.90 bar is insensitive; key axes missing |
| I01 | HIGH | Permanent costs omitted or under-tiered |
| I02 | HIGH | Cost tier is self-assigned yet gates dominance (R4 condition 4) |
| I03 | MEDIUM | Minimum-gain exemption keys on program-assigned `V1_BLOCKING` labels |
| I04 | MEDIUM | Two unconnected cost vocabularies (OD-21 to OD-24 versus C1-C7/T0-T4) |
| I05 | LOW | Copyleft R2 screen over-scoped relative to SPEC §5.8 |
| J01 | BLOCKER | `HC_UNVERIFIED` is absent from §8.2; scope-exclusion and PLATFORM_BLOCKED loopholes |
| J02 | HIGH | Held-out replication passes about 50% of null effects (R5 c) |
| J03 | HIGH | MEDIUM confidence permits `DECIDED` under caps that should block |
| J04 | HIGH | Decision type is self-selected and unrecorded; FORMAL skips held-out and sensitivity |
| J05 | HIGH | R10 under INCONCLUSIVE defaults to the cheapest candidate without triggering CB-1 |
| J06 | MEDIUM | Several §8.2 conditions rely on an undefined gate audit; trivially satisfiable counterevidence; amendment loophole |
| J07 | MEDIUM | HUMAN-FACING decisions can be `DECIDED` on simulated evidence |
| K01 | BLOCKER | Ledger schema lacks fields the method needs; R3's "dimensions assigned to the decision" do not exist |
| K02 | MEDIUM | The method's own ledger decisions lack their required evidence (rule simulation) before binding |
| K03 | LOW | `thresholds.json` proposal is labelled `pre-registered` before review |

---

## A. Cross-document, SPEC and freeze conflicts

### A01 [BLOCKER] Rule order is defined twice and inconsistently

- **Where:** objective §1.2 (steps 1-5) and the Companion row; method §2 (R0-R10), §2.11.
- **Problem:** The two procedures differ in substance.
  - *Objective:* feasibility, then epsilon-Pareto over the OD vector, then capability-combination coverage, then permanent cost "when remaining differences are not practically significant", then residue.
  - *Method:* cost tier is already a dominance condition (R4 condition 4). R6 eliminates higher-tier candidates even when their differences *are* significant. R8 selects by weighted band-unit utility. No step counts CC coverage.

  The Companion precedence clause covers thresholds and statistics (method governs) and violations and metrics (objective governs). Rule order falls under neither.
- **Evidence:** objective L60-L64; method L128-L202. [DERIVED]
- **Required fix:** Put one normative decision procedure in `decision-method.md`. Replace objective §1.2 with a pointer to it plus the ideal it operationalizes. Decide whether CC coverage is a selection step (define where it sits relative to R4, R6 and R8) or a reporting obligation. State once where permanent cost enters. Extend the precedence clause to cover "procedure and rule order: method governs".

### A02 [BLOCKER] I24 is handled three incompatible ways

- **Where:**
  - method R1 bullet 4, §3.2 T-17 ("An I24 violation is R1: `fingerprint_attack_success` DISTINGUISHABLE above the length-only reference"), §3.3;
  - objective HC-14 statement ("I24 is a design objective; its residual is graded under OD-16"), MVT-14(e), OD-05 note ("Padding bytes … cannot be removed to improve M05.1 (HC-14)"), OD-16 type GRADED.
- **Problem:**
  1. The method makes a hard constraint depend on a CI, a 0.05 band and a comparison to a reference. That contradicts objective §2.0 rule 1 (no bands on HCs), and the objective governs what counts as a violation.
  2. SPEC §7.6 and §9.8 describe the default bucketed padding as "quantised, not hidden". They note that a published attack needed only two chunk sizes. They provide `--pad none` as a recorded opt-out and `--chunk-boundary=keyed-prf` as the strong option. The method's R1 test would therefore eliminate the SPEC default and the SPEC opt-out.
  3. The objective's claim that padding cannot be removed contradicts SPEC `--pad none|buckets|max` (SPEC CLI L1848; §23.4 "Opt-out exists and is recorded as a declared leak").
  4. The T-17 observer model is "declared by DEC-CRY-053", which is `EXTERNAL_REVIEW_REQUIRED`. The attack is "program Task 22.2", which is not in the repository. The R1 test cannot be executed.
- **Evidence:** SPEC L846-L854, L1143-L1152, L2485; ledger DEC-CRY-053 status (Appendix R.2). [DERIVED]
- **Required fix:** Define the I24 hard constraint as mechanically checkable facts only:
  - boundaries in encrypted archives are keyed (MVT-14(e) with a stated chance rate);
  - the declared padding mode is recorded, and `inspect`/`verify` report `pad none` as a declared leak;
  - observed public lengths never exceed what the declared mode permits.

  Grade residual leakage per padding mode in OD-16 with pre-registered attacks. Route effectiveness to external review per SPEC §25.2. Delete the R1 statistical test. Correct the OD-05 padding note. Commit the attack definition and observer model to the repository before any OD-16 result, or mark OD-16 as not decision-grade.

### A03 [BLOCKER] Library-version-defined decode steps

- **Where:** method R2 "Unspecifiable … decoding depends on … library version" (L117-L118); objective HC-11 statement (L357) and Status (L371), HC-12 statement (L375).
- **Problem:** The objective admits a decode step defined by a pinned library version if the version identity is recorded and verified by digest, with gated features declaring the missing public spec. The method eliminates such designs at R2. Production planner v5 (`deflate-reconstruct`, preflate class, F-03) is exactly this case. The status-quo candidate (required by R0) is therefore HC-feasible yet R2-eliminated. The documents give no rule for which view wins.
- **Required fix:** Choose one rule and put it in both documents.
  - If admitted: R2 exempts gated, registry-declared, version-pinned steps. Charge them the highest longevity cost (see I01), require permanent vectors, and make them ineligible for the baseline set.
  - If not admitted: record production v5 and v6 as defects under objective §2.0 rule 5, and add "remove from baseline or re-specify" candidates to the affected decisions.

### A04 [BLOCKER] Freeze handling contradicts HC-16 and the repository freezes

- **Where:**
  - method R1 "Freeze exception" (L109);
  - T4 "any change to a frozen wire format … subject to R1 unless the decision is explicitly about revising that freeze" (L735);
  - §7.4 "no sunk-cost credit … removing them before v1 is cheap" (L758-L760);
  - objective §1.4 "may not vary: the behavior of already-published identifiers", HC-16 "Historical archives remain decodable with unchanged interpretation", §4.5 "This program cannot relax an HC", F-01 to F-27;
  - `CONTRIBUTING.md` ("Never change those in place"; "requires a new layout feature bit, not an edit in place").
- **Problem:** The method builds a path to revising freezes that the objective declares outside program authority. The freeze exception applies to "that freeze". Most freezes are HC-16 identifier freezes, so the exception relaxes an HC. §7.4 treats removal of published features as cheap, but HC-16 requires historical archives to stay decodable. README's "not stable" status does not override CONTRIBUTING or HC-16 inside this program.
- **Required fix:**
  1. Delete the freeze exception, or restrict it to ledger constraints that the B01 crosswalk classifies as DECISION_INPUT (not HC or F).
  2. Redefine T4 as "new identifier that supersedes frozen behavior". In-place change is L0-excluded.
  3. Rewrite §7.4. Before v1, a writer may stop *emitting* a frozen feature, but decode support persists (HC-16). Retained decode support is charged as ongoing cost to every candidate that keeps it reachable, and to none that only stops writing. Any proposal to drop decode support is escalated as UNRESOLVED per objective §4.5.

### A05 [HIGH] Profile decisions contradict SPEC §5.4 and §25.3

- **Where:** method Appendix B.1 weights, B.2 scenarios, §6.5; objective OD-05 to OD-10.
- **Problem:**
  - SPEC §5.4 defines a profile as *minimise stored_bytes subject to* decode throughput, decode memory, access granularity and compress budget. Appendix B instead scalarizes `artifact_bytes`, `encode_wall_s`, `decode_wall_s`, memory and `access_granularity_bytes` with weights. The note that bounds are R2 constraints does not remove the double counting: the constrained quantities are also weighted objectives.
  - SPEC §25.3 requires profile *weights* to be "fit to a corpus that reflects real file-type mixes, not to a general-purpose benchmark". The method uses INFERRED weights and equal family weights.
  - Ledger DEC-CMP-035 lists §25.3 as a hard constraint.
- **Required fix:**
  - For profile-scope decisions, apply the SPEC form: R2 screens the declared profile bounds, then the primary objective is `artifact_bytes` alone. Other metrics act only as R4 not-worse guards and R10 keys.
  - Where weights remain (non-profile decisions, scenarios), derive the family mix from a pre-registered, cited file-type-mix source committed before results. Keep equal weights as a sensitivity arm.
  - Record the conflict resolution in DEC-CMP-035.

### A06 [HIGH] Parameter sets differ between the documents

- **Where and evidence:** [DERIVED]

  | Parameter | Objective | Method |
  |---|---|---|
  | Network profiles | OD-12: RTT {1, 20, 100, 300} ms × bandwidth {1, 10, 100, 1000} Mbit/s; loss as seeded stalls | §4.15: NP-LAN 2 ms/1000, METRO 20/200, CONT 80/50, INTER 200/20; no performance claims under loss |
  | Thread sets | MVT-11(a) {1, 2, 8, 32}; MVT-13(a) {1, 8, 32}; OD-06 {1, 8, all logical}; OD-13 {1, 2, 4, 8, 16, 24, 32}; P-cores "logical 0-15" | §4.2 pinned sets: `st-p` {2}, `mt-p-n` n ≤ 7, `mt-psmt-14`, `st-e`, `mt-e-n` n ≤ 16; CPUs 0-1 avoided; `os-scheduled` never paired with pinned |
  | Determinism repetitions | MVT-13(a) "at least 3 times per variation cell" | §4.6 "2 repetitions" |
  | Blast radius | OD-14 M14.1 per damaged stored byte; single-bit, 512 B, 64 KiB, 1 MiB overwrites | T-15 per single-byte corruption, ≥ 1000 injections, stratified by region |
  | Verified partial retrieval | M11.1 ratio of bytes and time | T-13 `verify_overhead_pp` in percentage points of wall time |
  | Presence confirmation | M16.2 TPR − FPR at a pre-registered FPR | T-17 `fingerprint_attack_success` fraction |
  | Scenarios | AGG-S headline is the worst scenario | §6.5 scenario weights 0.8/0.2 as a sensitivity test |
  | Scale tiers | "reported separately in addition to the pooled value" | Not used in L1-L4 or in any rule |

- **Required fix:** Keep one canonical parameter table in `decision-method.md`, with objective sections citing it by ID. Add a generated crosswalk from M-number to canonical metric name to T-id to ebr family. Make every row above consistent. Unpinned 8- and 32-thread runs are not decision-grade under §4.2, so decide which thread configurations are decision-grade.

### A07 [HIGH] OD-13 speedup compares forbidden configurations

- **Where:** objective OD-13; method §4.2 "Hybrid confound: Samples from different affinity configurations are never compared"; T-14.
- **Problem:** S(n) = T(1)/T(n) is a comparison across affinity configurations by definition. 8, 16, 24 and 32 threads either do not exist as pinned configurations or mix P-cores and E-cores, so efficiency S(n)/n has no meaning on this hybrid host.
- **Required fix:** Define speedup within one core class (P-only n ≤ 7 via `mt-p-n`, E-only n ≤ 16 via `mt-e-n`, SMT variant separately) as a derived metric with its own A/A-calibrated noise estimate. Label `os-scheduled` scaling informational. Amend §4.2 to permit this one derived comparison explicitly.

---

## B. Hard constraints

### B01 [BLOCKER] Ledger constraints are free text with no HC mapping

- **Where:** method R1 ("hard constraints in `archetypal-objective.md` … and the repository freezes recorded in the ledger `hard_constraints`"); objective §2.0 rule 4, Appendix B, Appendix C.
- **Problem:**
  - 545 of 593 decisions carry at least one non-invariant constraint string. There are 989 distinct such strings.
  - A crude keyword screen against the F-register matches none of 562 of them. Examples: "SPEC §20.7 no in-archive decompressor VM", "SPEC §9.6 L1115 addressing must not survive audience change", "CLI exclusive-create no-overwrite", "SPEC §16.2 provenance removal must invalidate signatures".
  - Some strings are deferral or scope decisions rather than constraints, such as "SPEC §25.1 #21 library API surface deferred" and "SPEC §14.4 Tier 2 (post-v1)". Treating them as R1 screens pre-empts the decision.
  - Without a mapping, the applicable HCs per decision are unknown, so `HC_UNVERIFIED` blocking (objective §2.0 rule 4) cannot be computed.
- **Evidence:** Appendix R.2. [COMPUTED]
- **Required fix:**
  1. Commit a generated crosswalk, for example `research/methods/constraint-crosswalk.csv`. It classifies every distinct ledger constraint string as `HC-xx`, `F-xx`, `NON_GOAL` (SPEC §24), `PROGRAM` (standing constraint), or `DECISION_INPUT` (not a screen).
  2. Record reviewer sign-off for the classification.
  3. Add the ledger field `hc_ids` (see K01).
  4. State in R1 that only `HC-xx` and `F-xx` classes screen.
  5. Extend the F-register with every freeze the crosswalk finds (for example `docs/format-v0.md` L477 and `docs/random-access-v1.md` L3-4).

### B02 [HIGH] Missing HC: input consistency during pack

- **Where:** No HC or OD covers SPEC §5.7a. Only DEC-CMP-028, DEC-ECO-066 and DEC-PLT-017 mention it.
- **Problem:** SPEC §5.7a requires one open and one read per file, mandatory change detection with bounded retry, and a `fidelity.unstable_source` declaration, "never included silently". An archive whose metadata, hashed bytes and sparse map come from different versions of a file passes MVT-02, which compares against "the exact octet sequence read at creation", and passes MVT-07.
- **Required fix:** Add an HC, or extend HC-07 (A6 "loss declared") with MVT-07(d):
  - A seeded mutator process rewrites, truncates, extends, swaps and touches mtime on source files during `pack`.
  - **Oracle:** each packed entry equals one coherent source version, with content, size and metadata from the same descriptor state, or carries `unstable_source`.
  - **Violation:** a mixed-version entry without declaration.
  - Add a static audit for re-stat by path. Add an OD metric for retry cost.

### B03 [HIGH] Missing HC: CPU and algorithmic-complexity bounds

- **Where:** objective HC-06 covers memory, scratch and KDF only. Method R2 lists "CPU work per input byte" as a resource but defines no test. No ledger decision mentions quadratic or algorithmic complexity.
- **Problem:** Several paths can take superlinear time on hostile input: collision detection over hostile names (case-fold and NFC maps), hash flooding, pathological metadata, sparse-map and Merkle recomputation per range read, and decode time per compressed byte. None is a violation today. A9 ("declared and bounded") and HC-06's intent are unmet.
- **Required fix:** Add MVT-06(d):
  - Build adversarial scaling series of 2^10 to 2^20 entries, components, records and ranges per structure.
  - Measure CPU time from process accounting or in-process counters.
  - **Violation:** the fitted log-log exponent's CI lower bound exceeds 1 + δ (δ pre-registered, for example 0.15), or CPU time per input byte exceeds a declared bound.
  - Add the corresponding OD-17 and OD-24 metrics.

### B04 [HIGH] HC-06 oracle uses noise bands and an uncomputed H

- **Where:** objective MVT-06(a) "peak RSS ≤ R0 + H + the `memory_peak` noise band"; Status "numeric value of H is not yet computed".
- **Problem:**
  - An HC oracle with a 10% plus 4 MiB tolerance contradicts §2.0 rule 1.
  - It lets up to about 4 MiB of proportional allocation happen before validation on every open.
  - R0 is a median, not a bound.
  - Windows exposes job commit, not RSS.
  - With H uncomputed, the oracle is unusable.
- **Required fix:** Make the primary HC-06 oracle an instrumented counting-allocator build with allocation-site attribution. **Violation:** any allocation sized from an untrusted field before its budget check, or allocator peak > R0_max + H, with exact arithmetic and no band. Keep process-level RSS and commit as secondary corroboration. Compute H per candidate from the wire specification before pre-registration, or mark HC-06 `HC_UNVERIFIED` for every candidate.

### B05 [HIGH] Intermittent nondeterminism escapes

- **Where:** method §4.13 "a confirming rerun that reproduces the mismatch makes it an R1 elimination"; §4.6 two deterministic repetitions; objective §2.0 rule 2, MVT-13(a).
- **Problem:** Races and scheduling-dependent output are intermittent. If a rerun does not reproduce the mismatch, the rule is silent and in practice the candidate survives. Two or three repetitions per cell have low detection power for rare interleavings.
- **Required fix:**
  - Two differing outputs from identical input, configuration and build are themselves the reproducible violation artifact. Commit both artifacts and their metadata.
  - Waive the violation only when an infrastructure fault is proven, for example the stored artifact's re-hash differs from its recorded hash, or storage errors are logged.
  - Add a stress mode for any determinism claim: at least 20 repetitions at the maximum thread count with seeded background load.
  - State the per-cell detection bound.

### B06 [HIGH] HC-05 escape oracle is self-reported and assumes i.i.d. races

- **Where:** objective MVT-05(a) Windows "API-level logging in a research build"; MVT-05(b) rule-of-three bound over 10,000 trials.
- **Problem:**
  - API-level logging records what the extractor *requested*, not what the kernel *resolved* (reparse-point traversal, junction swaps). It is blind to the escape class it must detect.
  - The rule-of-three bound assumes a fixed per-trial probability. Adaptive attackers widen race windows with oplocks and directory change notifications, so random trials bound nothing adversarial.
- **Required fix:**
  - Build the oracle from observations independent of the extractor: sentinel directories outside the root with deny-write ACL canaries, a post-run tree diff of sibling directories, and handle final-path checks.
  - Add deterministic interleaving tests: a filesystem shim or harness that performs the swap at every path-resolution step in turn.
  - Label the random-trial result as regression evidence, not a security bound.
  - Record whether the session can create NTFS symlinks (Developer Mode or privilege). If not, symlink cases on NTFS are `HC_UNVERIFIED`.

### B07 [HIGH] Triage loophole in HC-03 and HC-12

- **Where:** objective MVT-03(b) "any disagreement that triage does not attribute to a bug in the independent reader"; MVT-12(a) "failure not triaged as an independent-reader bug".
- **Problem:** Triage is done inside the program. Every disagreement can be attributed to the independent reader, which empties both HCs.
- **Required fix:** Pre-register the triage rule. A disagreement is an independent-reader bug only if a cited normative sentence unambiguously mandates the production behavior. A session involved in neither reader adjudicates. Commit the triage log with a citation per case. Report the counts of disagreements attributed to each side. Any case without a citation counts as a specification violation.

### B08 [HIGH] Traceability keys on keywords the SPEC barely uses

- **Where:** objective MVT-12(b) "extracts every normative MUST, MUST NOT and SHALL".
- **Problem:** The SPEC has 3 lines containing uppercase `MUST` and 92 case-insensitive occurrences of the word "must". Repository docs have 6 and 72. Normative force is mostly carried by "must", "never", "is prohibited", "only" and normative tables. The extraction script would find almost nothing, so 100% coverage would be vacuous.
- **Evidence:** Appendix R.3. [COMPUTED]
- **Required fix:**
  - Define the normative-statement grammar: lowercase modal verbs, "never", "always", "prohibited", "only", and tables marked normative.
  - Validate extraction recall on a seeded random sample of 100 sentences, labelled by a session outside the program, with a pre-registered recall of at least 0.95.
  - Otherwise require the specification to be rewritten with RFC 2119 keywords before HC-12 can pass.

### B09 [MEDIUM] HC-13 cross-host determinism is ill-posed

- **Where:** MVT-13(a) "host {Windows, WSL, QEMU arm64}; source tree recreated with shuffled creation order … Violation: any PCI difference or any LAI difference".
- **Problem:** Recreated trees have different mtimes and possibly ownership. NTFS and ext4 capture different metadata classes. AUX and PCI therefore differ legitimately, and LAI may too, depending on the identity profile. The test yields false violations or ad hoc explanations.
- **Required fix:** Define the determinism input as (EAM, identity profile), or fix timestamps and ownership per the SPEC §12.1 deterministic-mode rules. List the metadata classes captured per host. Require PCI identity only within one host class. Across hosts, require LAI identity plus mechanically FidelityReport-explained AUX differences.

### B10 [MEDIUM] HC-11 variation is not verified to take effect

- **Where:** MVT-11(a), (b).
- **Problem:**
  - Locales `tr_TR.UTF-8` and `ja_JP.UTF-8` may not be generated in the images, so they silently fall back to C.
  - Windows system locale and time zone changes are settings changes agents may not make (method §4.2).
  - The statement lists "CPU feature detection", but nothing varies it.
  - QEMU arm64 exposes one CPU model.
  - `docker --memory` 1× includes page cache, so OOM kills create false outcome differences.
- **Required fix:**
  - Record `locale -a` and run a locale canary (Turkish dotted-i case mapping) per run.
  - Declare the Windows locale and time-zone variation `HC_UNVERIFIED` unless it is per-process.
  - Add QEMU x86-64 user mode with a `-cpu` model lacking AVX2, and at least two aarch64 `-cpu` models.
  - Define the memory limit as R0 + declared requirement + a page-cache allowance, and classify OOM kills separately.

### B11 [MEDIUM] HC-02 round-trip matrix lacks path coverage

- **Where:** MVT-02(a)-(d).
- **Problem:** Rare reconstruction and encoding paths may never be exercised by default planner choices on the corpus, so zero mismatches is vacuous for them. Examples: deflate and JPEG reconstruction, dictionaries, lookback groups, cross-entry dedup, sparse files, empty files, and symlink targets with invalid UTF-8.
- **Required fix:**
  - Emit per-path coverage counters from the recorded plans.
  - A zero-mismatch claim for a (profile, layout, encryption) cell requires each registered codec, transform and feature path in that cell to be exercised at least N times (N pre-registered). Otherwise that path is `HC_UNVERIFIED`.
  - Mutation-test the write-time verification code, for example with `cargo-mutants` on the round-trip comparison, and require every mutant to be killed.

### B12 [MEDIUM] Instrumented builds as oracles

- **Where:** MVT-02(b), MVT-14(c), MVT-15(a) research builds (F-24).
- **Problem:** Instrumentation changes code paths and timing, and no MVT shows that the research build with its feature off equals production.
- **Required fix:** Add MVT-16(c). The research build with features off must produce byte-identical artifacts and identical outcomes to the production build over the conformance corpus plus tuning items. Replicate race- or timing-sensitive HC results on the production build black-box where possible.

### B13 [MEDIUM] HC-15 needs a fault-class precedence table

- **Where:** MVT-15(b).
- **Problem:** Some injections are both truncation-like and corruption-like, for example a corrupted length field that points past EOF, or footer damage. The confusion-matrix oracle has no defined correct class for them.
- **Required fix:** Pre-register a precedence table derived from SPEC §4.2, §4.4 and §20.5, keyed by injection taxonomy. Count off-diagonal cells only against that table.

### B14 [MEDIUM] HC-14 statement items without tests

- **Where:** HC-14 statement lists key commitment, constant-time secret comparison, secret hygiene and unlock order. MVT-14(a)-(e) test none of them directly, and none tests authentication downgrade.
- **Required fix:** Add these MVTs:
  - (f) Key commitment: vectors and constructions where one ciphertext decrypts under two keys must be rejected.
  - (g) Secret hygiene: a compile-time and test audit of secret types (no `Debug`, `Display` or implicit `Clone`; zeroize on drop verified in a research build).
  - (h) Constant time: code audit plus a dudect-style leakage test, labelled `HC_UNVERIFIED` on this noisy laptop unless it passes.
  - (i) Downgrade: strip signature records, encryption markers or feature bits and recompute unkeyed digests. The reported verification state must not exceed the oracle state (HC-15).

### B15 [MEDIUM] HC-16 regression covers generated seeds only

- **Where:** MVT-16(a), MVT-13(c).
- **Required fix:** Run frozen planner and chunker IDs differentially: baseline binary at `9e44608` versus the candidate binary, over every tuning and validation item and the conformance corpus. Require byte-identical plans, boundaries and containers. This is cheap and far stronger than per-seed goldens.

### B16 [MEDIUM] Crash consistency and HTTP origin safety are not HCs

- **Where:** F-22 (all-or-nothing publish), the ledger's "CLI exclusive-create no-overwrite", and F-10 (strong ETag, exact 206, no 200 fallback). These appear only as OD-12 M12.4 "must pass" inside a graded OD.
- **Required fix:** Fold them into HC-15 or a new HC-18 with MVTs:
  - kill -9 at seeded points during pack, repack, export and publish. **Oracle:** no destination file that verifies as `OK`; no partial publish; nothing overwritten.
  - An `ebr-origin` misbehaviour suite covering weak or missing ETag, 200 fallback, revision change and short 206 responses.

### B17 [MEDIUM] HC-10(d) cross-producer AUX equality

- **Where:** MVT-10(d) "Pack the same tree under every planner ID and profile. Violation: unequal LAI or AUX."
- **Problem:** If any creation provenance (planner or profile identifiers) is AUX-bound under SPEC §8.1 and F-19, AUX differs legitimately.
- **Required fix:** Cite the SPEC §8.1 cell that places planner and profile provenance outside AUX, or compare with provenance stripped.

### B18 [LOW] Missing detection bounds

- **Where:** §2.0 rule 2 requires them. Only MVT-05(b) states one. MVT-03(b), MVT-06(a), MVT-14(b), MVT-14(c) and MVT-15(b) do not.
- **Required fix:** State the bound, or label each as regression evidence.

### B19 [MEDIUM] Unreviewed L0 and R1(b) eliminations

- **Where:** objective §2.0 rule 3 L0; method R1(b) "a formal argument that the design admits a violating archive or state".
- **Problem:** A designer can prune rivals of a favoured candidate by argument alone.
- **Required fix:** Every L0 or R1(b) exclusion needs a written counterexample (archive bytes, state sequence or specification clause) and sign-off by a session not involved in the candidate set. The exclusion is recorded in `candidate_exclusion_reasons` with the reviewer.

---

## C. Invariant mapping (I1-I31)

All 31 invariants appear in objective Appendix A, and every HC has at least one invariant. No invariant is unmapped. The defects are consistency and depth.

### C01 [MEDIUM] Appendix A and the reverse check disagree with §2.1

- **Evidence:** In 10 of 17 HCs, the §2.1 table lists invariants that Appendix A's HC column (and its reverse check, which matches Appendix A) omits. Four of the omissions are *primary* invariants. [COMPUTED, Appendix R.4]

  | HC | Invariants in §2.1 missing from Appendix A (primary marked *) |
  |---|---|
  | HC-02 | I30 |
  | HC-03 | I19, I21 |
  | HC-05 | I17, I25 |
  | HC-08 | I26 |
  | HC-09 | I10, I30 |
  | HC-10 | I23 |
  | HC-11 | I31 |
  | HC-12 | I12*, I15*, I21 |
  | HC-13 | I15, I28*, I30 |
  | HC-14 | I12, I25* |

  Appendix A is tagged `FORMALLY_DERIVED`, but the claim does not hold.
- **Required fix:** Generate §2.1's invariant column, Appendix A's HC column and the reverse check from one data file with primary and supporting roles, via `objective_screen.py` or a sibling script. Commit the data file.

### C02 [MEDIUM] Mapped invariant clauses with no test

| Invariant clause | Gap | Required MVT item |
|---|---|---|
| I10 "reconstructible" | MVT-01(b) tests cache disagreement; MVT-10(a) tests identity after `--rebuild-index`. Nothing tests reconstruction itself. | Delete or zero the Index. Decode must be identical, `EB_ECF_INDEX_ABSENT_REBUILT` must be reported, and the rebuilt Index must equal the original canonically. |
| I11 "explicitly scoped", "typed" | Only unknown Critical names (MVT-04) and `x-` participation (MVT-10(c)) are tested. | An archive-scope item at entry scope, an item with the wrong value type, and an unregistered non-`x-` namespace must each give a stable refusal code. |
| I7 explicit entry identity | Not mutated anywhere. | Mutate `identity_digest` and `aux_digest`. The result must be `CORRUPT` with a stable code. |
| I20 "validated against the descriptor" | MVT-06(a) covers bombs, not count-versus-descriptor disagreement. | Structures whose element count differs from the declared count in both directions. |
| I22 "never a bare root" | MVT-10(b) covers vectors only. | A signature or binding over a bare Merkle root, or descriptor-field-swapped roots, must be rejected. |
| I23 / F-17 "never infer hardlinks from equal content" | Not tested. | Equal-content, distinct-inode files must not become a hardlink group, and restore must not link them. |

### C03 [LOW] C.1 invariant counting

- **Problem:** `objective_screen.py` counts only exact `I<n>` constraint tokens. Ledger strings such as "I15 unique deterministic interpretation", "I12 unknown optional stanzas ignored but authenticated" and "I24 (keyed variants must exist for encrypted archives)" are missed.
- **Required fix:** Match `\bI([1-9]|[12][0-9]|3[01])\b` inside strings.

---

## D. Metrics

### D01 [BLOCKER] Most OD metrics have no threshold, canonical name or role

- **Where:** objective §3.2; method §3.2 (T-01 to T-19), §3.6 (family `other` null), §0.2.
- **Problem:** These objective metrics have no canonical name, band or declared role in the method: M01.1, M01.2, M01.3, M03.1-M03.3, M04.2, M04.3, M05.3, M05.4, M06.3, M06.4 (encode throughput has a T-id; the logical-throughput definition differs), M08.3, M09.1-M09.4, M10.2-M10.4, M11.1-M11.3, M12.4, M13.2, M14.1-M14.3, M15.2-M15.5, M16.1, M16.2, M16.4, M17.1-M17.4, M18.1-M18.4, M19.1-M19.5, M20.1-M20.4, M21.1-M21.5, M22.1-M22.5, M23.1-M23.3, M24.1-M24.4, M25.2 (partly T-18), M25.3, M25.5, and M26.1-M26.4.

  Decisions in the access, legacy, platform, ecosystem and integrity clusters will need these as primary metrics. §0.2 freezes only thresholds that already exist, so bands created later would be set with results visible.
- **Required fix:**
  - Give every OD metric a canonical name and one of: (a) a T-id with band and floor, (b) "binary" (§3.3), or (c) "descriptive only; cannot eliminate or select".
  - Add a rule to §0.2: a metric without a band committed before its first decision-relevant result is secondary for every decision, permanently.
  - Transcribe the result into `metric_thresholds`.

### D02 [HIGH] Wrong-direction and gameable metrics

| Metric | Defect | Required fix |
|---|---|---|
| M22.4 | "Minimize M22.1-M22.4" includes maintainer count. Fewer maintainers is worse; method §7.2 wants ≥ 2 publishers. | Maximize maintainer count. Make release age descriptive, since feature-complete crates are legitimate. |
| M21.3 | Specification defects per 1,000 words are minimized and counted by an in-program implementer, which rewards not reporting defects. | Descriptive only, or counted by an outside reviewer on a blinded sample. |
| M24.2 | Fuzz findings per CPU-hour are minimized, which rewards weak fuzzing. | Report at a fixed coverage target. Findings are defects (HC-03, HC-06), not a graded objective. |
| M04.3 | Distinct outcome and reason pairs per CPU-hour are maximized, which rewards reason-code proliferation (conflicts with OD-23) and measures the fuzzer. | Replace with coverage of pre-registered decided classes (fraction). |
| M23.1 | Normative words are minimized, which rewards terse, underspecified text (conflicts with HC-12). | Use words per normative rule, or restrict to R10 keys. |
| M01.1 | CACHE+CLAIM count is minimized, which penalizes indexes (conflicts with OD-10), and the classification is gameable. | Descriptive, with census classification by a non-designer. |
| M21.2 | Independent-reader LoC is minimized and written by the same program. | Descriptive; charge C2 instead. |

### D03 [HIGH] OD-03 headline gameable by declaration

- **Where:** OD-03 aggregation "Headline is the minimum over classes declared restorable for that pair".
- **Problem:** A candidate raises the headline by declaring fewer classes restorable.
- **Required fix:** Fix a class list per platform pair before results, derived from independently observed platform capability (MVT-07(a) observers). Count every class on the list, with undeclared non-restoration as 0. Report declared-non-restorable classes against incumbent capability.

### D04 [HIGH] M08.3 bounded-memory test has no threshold and is biased toward 0

- **Where:** OD-08 M08.3, "practically indistinguishable from 0 under `decision-method.md`". The method defines no such threshold.
- **Problem:** Regressing ln(peak RSS) on ln(bytes) includes the constant baseline R0. At small and medium sizes R0 dominates, so the slope sits near 0 even for linear growth, and bounded-memory claims pass spuriously.
- **Required fix:** Regress (allocator peak − R0) or use the 1 to 64 GiB probe directly. Pre-register "bounded" as: growth from 1 to 64 GiB at most max(4 MiB, 0.10 × R0), with the CI upper bound inside that. Otherwise the claim is false.

### D05 [MEDIUM] Vague definitions

| Metric | Vagueness | Required fix |
|---|---|---|
| M05.3 | "best single-stream codec" | Enumerate codecs and levels. |
| M10.2 | Range-size distribution unspecified; amplification for 1-byte ranges equals the chunk size | Pre-register range-size strata (for example 1 B, 4 KiB, 1 MiB) and report per stratum. |
| M14.1 | "per injected event divided by damaged stored bytes" is ambiguous; region weighting unspecified, so uniform byte sampling makes Index or manifest damage rare | Define per event; pre-register region strata weights plus a worst-region guard. |
| M15.2 | "authenticated coverage" undefined for unsigned plaintext archives | Define scope per archive type. |
| M16.2 | "best attack" set and FPR not pre-registered; the metric is a lower bound and gameable by weak attacks | Commit the attack suite and FPR, and include adaptive attacks. |
| M19.2 | "consensus of reference readers" undefined | Define as unanimity, with majority reported separately. |
| M25.1 | "pre-registered completion criterion" not written | Commit criteria. |
| M26.1 | Scenarios "from SPEC §21" not listed | Enumerate. |

### D06 [HIGH] "Serves a CC" is undefined or contradicts the SPEC

- **Where:** objective §1.3.
- **Problem:**
  1. "Best capability-matched baseline" does not exist for combinations that SPEC §23.2 says no incumbent serves.
  2. Using the T-01 1% size band makes CC-01 unservable by construction, while SPEC §23.4 accepts "a few percent versus solid" as the price of independence.
  3. Misassignments: CC-03 maps to OD-26, which has no single-file-portability metric. CC-04 has no metric for encrypted-index random access (OD-10 and OD-12 on encrypted archives).
  4. How the "default configuration" served count affects selection is undefined.
- **Required fix:** Give each CC a pre-registered declared-cost tolerance per OD, taken from SPEC §23.4 magnitudes where stated, and a named specialist baseline per OD (for example non-solid 7z for random access, solid 7z or tar.xz for ratio). Correct the OD assignments. Define the role of the default-configuration count: a reported outcome, or an R-rule.

### D07 [MEDIUM] Missing dimensions

- **Scale limits:** maximum entries, paths, depth and archive size before failure or superlinear behavior, beyond the 8 GiB tier.
- **Byte-weighted storage totals:** total stored bytes is what storage cost is (see D08).
- **Reader fragmentation:** archives unreadable by minimal readers because of optional or extended features.
- **Writer crash consistency:** see B16.
- **Required fix:** Add each as an OD metric or as an explicit non-goal with rationale.

### D08 [MEDIUM] Aggregation hides large-tier regressions

- **Where:** objective AGG-G; method §4.9 L2-L4. Neither the worst-family guard nor tier reporting is used by any R-rule.
- **Problem:** Items weigh equally within groups and groups equally within families, whatever their size. A 3% regression on large-tier items can be outvoted by small items.
- **Required fix:** R8 and R4 not-worse conditions apply per scale tier as well as per family. Report the byte-weighted aggregate (ratio of totals) in sensitivity (§6).

### D09 [MEDIUM] Weight depends on the number of declared metrics

- **Where:** method §6.3 "default is equal weights over the primary metrics"; R3.
- **Problem:** Declaring `artifact_bytes`, `metadata_bytes_per_entry` and `http_bytes` together triples the weight on size. Omitting a dimension's metric avoids R3's "unmeasured primary dimension" block, because the author chooses the primaries.
- **Required fix:** Assign weights per OD, not per metric. Take the decision's OD set from the K01 assignment. Allow at most one primary metric per OD unless a written justification is reviewed.

---

## E. Thresholds

### E01 [HIGH] Composite rule and process floors hide large relative slowdowns

- **Where:** method §3.1 composite rule and §4.11 "`EQUIVALENT` holds if either is `EQUIVALENT`"; floor snapping; T-03, T-04 and T-08 process floor of 5 ms.
- **Problem:** For operations under about 5 ms (list, inspect, open, random entry on small archives), a 0.5 ms versus 4 ms difference (8×) is `EQUIVALENT` and snaps to ratio 1. T-08's own rationale ("batch tools doing thousands of lookups") contradicts that floor. Small-archive performance is structurally unmeasurable at process level.
- **Required fix:** Measure per-operation latency metrics (T-08, T-10, M07.3, M07.4, M10.3) only with in-process timers (0.1 ms floor), or as batches of N operations inside one process. Forbid process-level samples for them. Report the small tier separately.

### E02 [HIGH] T-15 floor and band erase the design space

- **Where:** T-15, relative 1.0 (band [0.5, 2]) and floor 1 MiB, justified by the historical `fixed-1mib/v1` chunk size [INFERRED].
- **Problem:** The live decisions (chunk size, group lookback, dictionary scope) move blast radius between about 64 KiB and a few MiB. Under this floor and band, 64 KiB versus 900 KiB is `EQUIVALENT`.
- **Required fix:** Set the floor at or below the smallest chunk size under consideration (for example 64 KiB). Use a log band with r ≤ 0.5. Report per region stratum with pre-registered weights and a worst-region guard.

### E03 [HIGH] Minimum-gain multipliers are inconsistent and sometimes infeasible

- **Where:** method §3.1 "Multipliers … `1+k*r`"; §7.3.
- **Problem:**
  - Linear scaling of r makes tiers mean different things per metric. T4 is ×1.10 for size (r = 0.01) but ×11 for blast radius (r = 1.0).
  - For absolute-only bounded metrics, gains become impossible or near-impossible:
    - T4 × `task_success_rate` requires a gain of 1.5 on a fraction.
    - T3 × `fingerprint_attack_success` requires 0.25.
    - Any T2 new CLI flag requires +0.45 task success.
- **Required fix:** Use log-consistent multipliers ((1+r)^k, or k·ln(1+r) in log space). For absolute-only metrics, publish a per-metric minimum-gain table capped at feasible values, for example at most 0.5 of the metric's range. Check that at least one plausible candidate per tier can pass.

### E04 [MEDIUM] T-01 64 B floor on small archives

- **Problem:** The justification (64 MB over 10^6 archives) is 6% for 10^6 archives of 1 KiB each. SPEC §23.4 lists tiny-archive overhead ("a few hundred bytes") as a real cost.
- **Required fix:** Make small-tier fixed overhead (M05.3) its own metric with a floor of at most 16 B, or apply relative-only bands on the small tier.

### E05 [MEDIUM] T-09 and T-11 floors disagree; T-11 rationale not emulated

- **Problem:** Access granularity drives remote fetch bytes, yet T-09's floor (64 KiB) is four times T-11's (16 KiB). T-11 relies on TCP initial-window behavior (RFC 6928), which a loopback application-level proxy does not reproduce: there is no slow start and the congestion window is large.
- **Required fix:** Align the floors. Compute remote latency from modelled round trips plus a transfer model, or calibrate the emulator's initial-window behavior (see F10).

### E06 [MEDIUM] Zero-scratch claims from polling

- **Where:** T-07 (0.05 s polling, lower bound); §3.3 "zero-scratch or streaming capability, when claimed".
- **Required fix:** Base binary scratch claims on write accounting: `/proc/<pid>/io` `write_bytes` deltas on the scratch mount, Windows job I/O write transfer counts, or an in-process filesystem shim. Tolerance is 0.

### E07 [MEDIUM] T-18 sessions are not independent samples

- **Problem:** Twenty sessions of the same base model are strongly correlated, so bootstrap CIs over them do not describe uncertainty.
- **Required fix:** Treat simulated sessions as heuristic evaluation. Report per-task success counts and qualitative failure modes. Allow no CI-based verdicts. Usability evidence may never be the sole selector (see J07).

### E08 [LOW] A/A criterion 2 is loose and underpowered

- **Problem:** Under the band-plus-0.99 rule, the pure-noise rate of `DISTINGUISHABLE` item verdicts is far below 1%, so 2% is loose. With about 78 tuning items the check has little power.
- **Required fix:** Allow at most 1 item, or use an exact binomial test at a pre-registered rate. Pair it with the known-effect calibration in F03.

### E09 [MEDIUM] R9(iii) regret bound

- **Problem:** Weighted band-unit regret ≤ 2.0 allows 2/w_m band units on one metric: 20 bands at w = 0.1. For T-15 (r = 1) that is a factor of 2^20.
- **Required fix:** Bound per-metric band-unit regret as well (for example ≤ 1 band unit per metric) and cap confidence.

---

## F. Statistics and measurement environment

### F01 [BLOCKER] Cluster bootstrap undercovers

- **Where:** method §4.10 corpus-level family-stratified cluster percentile bootstrap; single-group families "contribute a fixed value".
- **Evidence:** A simulation used the committed group structure: validation has 20 families with group counts {1×1, 2×9, 3×9, 4×1}; held-out has {1×1, 2×4, 3×10, 4×4, 6×1}. It used Gaussian group effects around family means and the percentile interval, with 2000 simulations × 2000 resamples. Actual coverage:

  | Split | Nominal 0.95 | Nominal 0.99 | Nominal 0.999 |
  |---|---|---|---|
  | Validation | 0.8245 | 0.9085 | 0.9585 |
  | Held-out | 0.8365 | 0.9290 | 0.9620 |

  Resampling n = 2-3 groups underestimates the variance by a factor of (n−1)/n, and single-group families add zero variance. Every corpus-level `DISTINGUISHABLE` verdict is anti-conservative. [COMPUTED, Appendix R.5]
- **Required fix:**
  - Replace the interval with one whose coverage is verified on this structure. Options: within-family deviations rescaled by sqrt(n/(n−1)) before resampling; a group-level wild (Rademacher) bootstrap; or a t-interval with Satterthwaite degrees of freedom on group-level estimates.
  - Exclude single-group families from CI construction, or give them a pooled variance.
  - Pre-register, as §14 item 9 rule simulation, that the chosen interval reaches nominal coverage within ±1 percentage point at the confidences in use.

### F02 [HIGH] Item-level bootstrap degenerates

- **Where:** method §4.10 item level (`median_of_ratios` percentile bootstrap over rounds), §4.6 (10 minimum rounds), §4.12 (Bonferroni).
- **Problem:** With n = 10 to 30 paired rounds and per-comparison confidence of 0.9997 (see F08), the percentile interval of a bootstrapped median approaches the sample range. The width is driven by extreme rounds, the adaptive cap is reached, and items are flagged `imprecise` (a MEDIUM cap) or verdicts become `INCONCLUSIVE`, which feeds R10.
- **Required fix:** Use an exact distribution-free order-statistic CI for the median (binomial), or a studentized CI on the mean log ratio. Restrict the Bonferroni family to confirmatory comparisons (F08). Pre-register a minimum detectable effect computed on tuning A/A data, and forbid confirmatory experiments whose MDE exceeds the band.

### F03 [HIGH] A/A calibration detects only one failure mode

- **Where:** method §4.7.
- **Problem:** Identical binaries cannot reveal:
  - (a) code-layout, link-order and environment-size bias, documented at magnitudes comparable to a 5% band (Mytkowicz, Diwan, Hauswirth & Sweeney, "Producing wrong data without doing anything obviously wrong!", ASPLOS 2009);
  - (b) Defender reputation and scan differences between distinct binaries (F07);
  - (c) candidate × scheduler interaction on the hybrid CPU, where single-threaded and multi-threaded candidates meet different P/E placement and power states;
  - (d) sensitivity: A/A passing says nothing about whether real 5% effects are resolvable.
- **Required fix:**
  - Add **A/A′**: functionally identical, byte-different builds (relinked, padded, different symbol order) run with randomized environment size. They must be `EQUIVALENT` at corpus level and at most at the E08 limit per item.
  - Add **known-effect calibration**: inject +2.5%, +5% and +10% CPU work into one arm. Pass requires +10% detected `DISTINGUISHABLE` on at least 90% of items, the correct sign at +5% on at least 80%, and no false direction.
  - Run both per environment and family before decision-grade status.

### F04 [HIGH] Laptop power-limit and thermal confounds

- **Where:** method §4.2 thermal controls: cooldown; 1 s single-thread drift canary on `st-p` with a 1.05 limit; `power_limit_confound` at wall ratio > 10× and > 20 s.
- **Problem:**
  - Mobile i9-14900HX packages drop from the short-term to the long-term power limit after a time constant of tens of seconds. Multi-threaded or long samples throttle in ways a 1 s single-thread canary does not see.
  - A drift limit equal to the band tolerates 4.9% drift.
  - Candidates whose sample durations straddle the throttling onset are biased even at a 2× wall ratio, not only at 10×.
  - Frequency changes affect CPU time as well as wall time.
- **Required fix:**
  - Record effective frequency per sample without admin rights, for example PDH `\Processor Information(_Total)\% Processor Performance` sampled during the window. Verify availability in the environment capture.
  - Invalidate samples whose mean relative performance falls below a pre-registered fraction of the session baseline.
  - Run the canary on the experiment's own affinity set and thread count, with a duration at least as long as the median sample.
  - Tighten the canary limit to 0.5 × band.
  - Flag any comparison whose candidates' sample walls straddle the measured throttling onset, not only 10× cases.

### F05 [HIGH] Quiet-machine guard too loose

- **Where:** method §4.4, `max_other_cpu_percent` 5% of 32 logical CPUs, `max_loadavg_1m` 32; `guard.py` averages other-process CPU over the window and all logical CPUs.
- **Problem:** About 1.6 busy CPUs can sit on the SMT sibling of a pinned P-core or draw from the shared package power budget. That shifts single-thread timings by more than 5%. On a 32-vCPU VM, a load average of 32 means saturation, so the WSL limit is effectively no guard.
- **Required fix:**
  - Account other-process CPU per core class and require the SMT siblings of pinned CPUs to be idle, with small limits such as ≤ 2% on the pinned P-core set.
  - Record MsMpEng and SearchIndexer CPU per sample as covariates.
  - In WSL, check load average after cooldown and before the sample against a small limit (for example ≤ 1.0 plus the measured tree's configured threads).
  - Validate the limits with injected background load during A/A (F03).

### F06 [HIGH] WSL2 virtualization is unlabelled and uncapped

- **Where:** method §4.1, §4.3, §9.1 qualifiers (EMULATED, but no VIRTUALIZED); objective OD-06 "WSL results are labeled virtualized".
- **Problem:**
  - vCPU placement on P- and E-cores is uncontrolled.
  - ext4 lives on a VHDX over NTFS.
  - `vm-cold` leaves the host cache warm.
  - Linux-native performance claims would rest on a VM hosted on a hybrid laptop.
  - The cross-environment rule ("no `DISTINGUISHABLE` verdicts in opposite directions") lets one `DISTINGUISHABLE` and one `INCONCLUSIVE` environment support a claim for both.
- **Required fix:**
  - Add a `VIRTUALIZED` qualifier and a MEDIUM cap for Linux-native performance claims.
  - A claim covering both environments requires same-direction support in both.
  - Forbid cold-cache claims; label `vm-cold` "VM cache dropped".
  - Record host MsMpEng CPU and VHDX host I/O during WSL samples.

### F07 [HIGH] Defender biases incumbent comparisons

- **Where:** method §4.2 "Encode timings include scan-on-close … labelled `defender_on`".
- **Problem:** On-access scanning cost depends on file format recognition (ZIP, 7z and tar containers may be inspected; an unknown `.eb` may not) and on binary reputation. Labelling the result realistic does not remove the bias in REF-INC comparisons. The unknown `.eb` format may be favoured, and freshly built unsigned candidate binaries may be penalized relative to distribution-signed incumbents.
- **Required fix:**
  - For external comparisons, report both Defender-on results and a run in a directory the program owner has excluded, recording the exclusion in the environment capture. Agents do not change security settings.
  - Alternatively, use in-process timers that exclude close.
  - Record MsMpEng CPU as a per-sample covariate.
  - Include distinct-binary A/A′ (F03).

### F08 [HIGH] Multiple-comparison scheme collapses power

- **Where:** method §4.12, R4 condition 2, R8 condition 3, §4.9 unanimity rule.
- **Problem:**
  1. m counts (pair × metric × environment × level). With the six R0 candidate kinds there are 15 pairs; × 5 metrics × 2 levels = 150, giving per-comparison confidence 0.999667 and B = 600,000 resamples. With 2-3 groups per family, verdicts become mostly `INCONCLUSIVE`, which routes to R10's lowest-tier key. Adding candidates is therefore a lever for defaulting to the cheapest.
  2. R4 condition 2 and R8 condition 3 use "not `DISTINGUISHABLE_WORSE`". Under unanimity (every group and every item beyond the band), a family-level worse verdict is nearly unreachable, so absence of evidence of harm passes as no harm.
  3. There is no program-level control across roughly 500 empirical decisions, and R5 is weak (J02).
- **Required fix:**
  - Limit confirmatory comparisons to the provisional winner against each other survivor, after R4 on tuning, with Holm or a fixed-sequence gatekeeping procedure.
  - Supporting conditions require non-inferiority: the CI bound on the unfavourable side lies inside the band. Mere absence of `DISTINGUISHABLE_WORSE` is not enough.
  - Report the program-level expected false-selection rate from the §14 item 9 rule simulation, and a Benjamini-Yekutieli-adjusted listing of decision-supporting comparisons.

### F09 [MEDIUM] Floor snapping inside bootstrap

- **Where:** method §3.1 "if an item's median paired |D| ≤ a, that item's ratio is set to exactly 1 before aggregating".
- **Problem:** Snapping on the point estimate but not inside replicates makes CIs inconsistent with point estimates, and vice versa.
- **Required fix:** Specify snapping inside each bootstrap replicate (recompute the item median per replicate), or apply floors only at verdict level.

### F10 [MEDIUM] Remote emulation calibration criteria

- **Where:** method §4.15; objective OD-12 cites `EXP-NETEM-CAL`.
- **Required fix:** Pre-register acceptance criteria before any remote result: measured RTT within ±10% of the profile, sustained throughput within ±10%, and request and byte counts exact. Also declare the connection model per profile.

---

## G. Held-out and validation leakage

### G01 [BLOCKER] Held-out set is not sealed

- **Where:** method §5.4 ("Forbidden to everyone: held-out content, listings, statistics and paths"), §5.7 L1, §5.8 audits; corpus files.
- **Evidence:** [COMPUTED, Appendix R.6; counts only, no matched line printed]
  - `research/corpus/manifest.json` items, held-out rows included, carry `description`, `provenance`, `source_file`, `materialization_key` and `materialized_relpath`.
  - Fingerprint filenames containing "heldout": 70 of 212 at the first count, 82 of 251 at the second. `item_id` values encode upstream identity (the reviewer saw one via `git status`).
  - "heldout" occurs in `research/corpus/sources/*.json` (37-53 occurrences per file) and in generator inputs. Examples: `generators/g5-media/selection/commons-usgov-photos.json` (260), `generators/g2-generated/make_sources.py` (47-50), and `generators/g1-code/authoring/f03_lockfile_audit.py` (22).
  - `statistics.json` (66) and `coverage.md` (123) also mention it. PROGRESS says statistics cover 131 tuning+validation items; whether held-out values are absent is unverified.
  - Counts come from the working tree. Concurrent, uncommitted corpus work changed several of them between two runs during this review. The exposure is structural, not a one-off.
  - Most real items are public upstream artifacts, so identity amounts to access. Generated held-out items can be regenerated from committed generator inputs.
  - The §5.8 audits check tool callers and git history, not reads.
- **Problem:** Every design agent can learn held-out identities and, for generated families, reconstruct their content. Under §5.7 L1 as written, this affects potentially every family, so every empirical decision would lose R5 eligibility.
- **Required fix:** Choose and document one option in the revision.
  - **(a) Re-seal.**
    - Rotate held-out to new independence groups whose sources, selection lists and seeds are withheld: an encrypted bundle with an owner-held key, or generation after freeze from an owner-held seed committed only as a hash.
    - Publish `manifest.public.json` without held-out rows for design and analysis agents, and use opaque held-out item ids and fingerprint filenames.
    - Remove held-out mentions from files agents read routinely.
  - **(b) Redefine the threat model** as "result-blind, identity-aware". State the residual risk, apply a MEDIUM cap to every R5 pass, and forbid candidates tuned on public data from the same upstreams.

  In either case:
  - Add an L1 audit that scans agent session transcripts for held-out item ids and paths before the freeze.
  - Separate roles: corpus-construction sessions are held-out-aware and may not design candidates, experiments or analyses.

### G02 [HIGH] Cross-split upstream sharing

- **Where:** method §5.7 L6; corpus validator.
- **Evidence:**
  - PROGRESS 2026-09-13 (g1-code): "vendor trees share common packages across splits".
  - PROGRESS 2026-09-13 notes "held-out large tier is all linux-kernel group".
  - PROGRESS 2026-09-16: kernel-headers duplicate-tree items were added to tuning, validation and held-out.
  - `research/corpus/generators/g1-code/authoring/f03_shared_packages_covariate.md` exists. Shared packages are being handled as a covariate, not as an L6 split violation.
  - The manifest script finds 0 independence groups spanning splits, because groups are labels, not content-derived. [COMPUTED]
- **Required fix:**
  - Add a content-overlap audit over fingerprint-level file SHA-256 sets (hashes only, which §5.4 allows): the overlapping logical-byte fraction between each pair of splits, per family, with a pre-registered maximum (for example ≤ 1%).
  - Group by upstream lineage: same project, different version or subset means the same group.
  - Rerun the L6 check and relock before the freeze.

### G03 [HIGH] Freeze and unlock granularity undefined

- **Where:** method §5.5. There is one `heldout-unlock.json` and one `EB_HELDOUT_UNLOCK`, yet `design-freeze.json` lists "frozen decision ids".
- **Problem:** If decisions freeze in waves, wave-1 held-out results are visible when wave-2 designs are built. That is L2 across decisions, and the method never treats it.
- **Required fix:** Declare either (a) a single program-wide freeze and unlock, where any decision not frozen by then needs fresh post-freeze data, or (b) sealed held-out tranches per wave with disjoint groups, sized so each tranche meets the §4.9 minimum support. This resolves DEC-ECO-076's candidate choice explicitly.

### G04 [HIGH] Validation looks are not confirmatory

- **Where:** method §5.2.
- **Problem:**
  1. The second look after a documented change uses the same items that §5.2 then reclassifies as tuning for the changed design.
  2. Validation is shared across 593 decisions. A decision designed after another decision's validation results were visible is adaptively tuned on validation. The per-decision look count does not see this.
- **Required fix:**
  - The second look is either on new validation items or labelled tuning.
  - Keep a program-wide validation-look registry. Any candidate created or modified after a validation look of a related decision (shared candidates, metrics or cluster) counts that look.

### G05 [HIGH] Post-unlock fixes can reach DECIDED

- **Where:** method §5.6 "Correctness fixes … labelled `post-unlock-fix` and caps confidence at MEDIUM"; §5.7 L2 "held-out results used to modify a design … As L1".
- **Problem:** The two sections contradict each other. §5.6 lets a candidate modified with held-out knowledge become `DECIDED` at MEDIUM.
- **Required fix:** A post-unlock-fixed candidate cannot be `DECIDED` on that held-out. Publish the results as report-only; re-selection needs fresh confirmatory data (as §5.6's last bullet already says for re-tuning). Remove `post-unlock-fix` from the §8.4 caps and add it to the blocks (J03).

### G06 [MEDIUM] Freeze commit scope

- **Where:** method §5.5 Commit A.
- **Required fix:** `design-freeze.json` also records:
  - harness crates commit;
  - ebr commit;
  - `Cargo.lock` hash and rustc version;
  - Docker image digests;
  - corpus provisioning tool hashes;
  - the `thresholds.json` hash;
  - candidate build flags.

### G07 [MEDIUM] Advisory access control

- **Where:** method §5.4. `/root/eb-research/heldout` (PROGRESS data root) is readable by root sessions. Only ebr and corpus tools check `EB_HELDOUT_UNLOCK`.
- **Required fix:** Do not materialize held-out content until Commit C. Provision after unlock and verify `logical_tree_sha256` against the lock. Otherwise keep it encrypted at rest.

### G08 [LOW] Relock criteria

- **Where:** method §5.3 "chosen by pre-declared criteria".
- **Required fix:** Declare the criteria (family, tier, size band, licence) now.

---

## H. Sensitivity analysis

### H01 [HIGH] Weight sensitivity passes trivially

- **Where:** method §6.2 (`floor` band-unit quantization), §6.3 (FPW counts ties as preserved), `stats.weight_sensitivity` (`tie_tolerance`, `argmax` base winner).
- **Evidence:** The committed `stats.weight_sensitivity` was run on three candidates, each within one band of the best on all five metrics (every q = 0). It returned FPW 1.0 for the grid and 1.0 for Dirichlet (c = 4, 10,000 samples), with `base_winner = 0` by argmax order. [COMPUTED, Appendix R.5]
- **Problem:**
  - The actual selection in that case is made by R10, yet the sensitivity test certifies stability at the top bar.
  - Sub-band deficits accumulate invisibly: a candidate 0.9 band worse on every metric scores the same as the best.
  - The tool's base winner depends on candidate order, not on R10.
- **Required fix:**
  - Compute utility and sensitivity on continuous band units (|ln ratio|/ln(1+r), no floor), and keep floor quantization only as an R10 eligibility screen.
  - When base-weight utilities tie within tolerance, record "no measured winner; R10 selection" and mark sensitivity not applicable, rather than passing.
  - Pass the selected candidate explicitly to the tooling instead of relying on argmax.

### H02 [HIGH] No sampling-uncertainty sensitivity

- **Where:** method §6.1 uses point aggregates.
- **Required fix:** Add a selection-stability bootstrap: resample groups within families (with the F01-corrected procedure), recompute R4 to R10 per replicate, and report the fraction preserving the selection, using the same 0.90/0.80 bars.

### H03 [MEDIUM] Neighbourhood-only weight exploration

- **Where:** method §6.3 (±50% grid; Dirichlet c = 4 around INFERRED weights).
- **Problem:** The test does not probe whether the inferred weights are right, which SPEC §25.3 requires to be fitted.
- **Required fix:** Also report SMAA acceptability indices under a flat Dirichlet over the whole simplex, and the winners at single-criterion vertices. The FPW bars still apply to the base neighbourhood.

### H04 [MEDIUM] Threshold-scaling consequence

- **Where:** method §6.4 "identical under neither: the scope fails, and R9 moves to a narrower scope".
- **Problem:**
  - Doubling the bands turns sub-2-band wins into ties, which R10 then resolves by cost. Narrowing the scope does not address that dependence.
  - Halving the bands can fall below A/A resolution and produce `INCONCLUSIVE` everywhere.
- **Required fix:** The outcome of threshold perturbation is a confidence cap plus a reported "band-dependent" flag, not scope narrowing. Run ×0.5 only for metrics whose A/A half-width at the cap is at most 0.25 × the original log band.

### H05 [MEDIUM] LOFO insensitive; axes missing

- **Where:** method §6.5.
- **Problem:** With about 20 applicable families, leave-one-family-out rarely moves a geometric mean, and a 0.90 bar allows two flips.
- **Required fix:** Add leave-3-families-out (all combinations, or a seeded sample of 1,000), leave-one-scenario-out, and tier-stratified and byte-weighted aggregates (D08). Add environment (Windows versus WSL) and reference (REF-PROD versus REF-INC) arms.

---

## I. Permanent cost accounting

### I01 [HIGH] Permanent costs omitted or under-tiered

- **Where:** method §7.1-§7.4; objective §4.4.
- **Missing or under-tiered costs:**
  - (a) Retained decode support for everything already published (HC-16) is never charged, and §7.4 prices removal as cheap (see A04).
  - (b) New planner and chunker IDs carry permanent golden vectors and determinism maintenance (objective MVT-13(c)), but T1 treats them as free.
  - (c) Identity participation assignments (LAI versus AUX membership, identity profiles) are permanent: once archives exist, a changed assignment changes identities. They should be T4.
  - (d) New reason codes are frozen identifiers (HC-16) but are not counted in C1.
  - (e) Library-version-defined decode steps (preflate, pinned `jixel`/`jxl-oxide`) are the longest-lived dependency, but C7 records them only qualitatively.
  - (f) Writer-path pinned encoders force planner-ID churn, but the maintenance penalty applies only to reader-path dependencies.
  - (g) Recurring external crypto and security audit cost is absent.
  - (h) Reader fragmentation from optional and extended features is absent.
  - (i) There is no add-now versus add-later comparison, although post-v1 additions become incompatible feature bits.
- **Required fix:** Extend C1, C5 and C7 and the tier table:
  - identity participation → T4;
  - library-version-defined decode → T4, plus the A03 resolution;
  - new planner or chunker ID → T1 with a permanent-vector cost line;
  - reason codes counted in C1;
  - maintenance penalty extended to writer-path pinned encoders;
  - "retained decode surface" as a recurring C2 line.

  Record add-later cost in C7 for every "defer" candidate.

### I02 [HIGH] Self-assigned tiers gate dominance

- **Where:** method R4 condition 4; §12.2 template "provisional cost tier".
- **Problem:** The record author assigns the tier, and a lower tier shields a candidate from elimination.
- **Required fix:** Compute tiers from checked-in C1-C3 scripts. A session not involved in the candidates assigns C4-C7 before validation. A disputed tier resolves to the higher tier.

### I03 [MEDIUM] V1_BLOCKING exemption

- **Where:** method §7.3 "Exempt if it is the lowest-tier way to satisfy a hard constraint or a V1_BLOCKING requirement".
- **Problem:** 177 decisions carry program-assigned `V1_BLOCKING` labels, so the exemption is wide open.
- **Required fix:** Exempt only when every lower-tier candidate failed R1 or R2 (the R6 rule), or when a normative SPEC or CONTRIBUTING clause requires the capability, cited by line.

### I04 [MEDIUM] Two cost vocabularies

- **Where:** objective §1.2 step 4 and §4.4 (OD-21 to OD-24); method §7 (C1-C7, T0-T4).
- **Required fix:** Add a crosswalk (C1↔OD-23, C2↔OD-21/OD-24, C3↔OD-22, C4↔OD-24, C5↔OD-21/OD-22, C6↔OD-25, C7↔OD-22) and state that tiers, not OD-21 to OD-24 values, drive R4, R6 and R10.

### I05 [LOW] Copyleft screen scope

- **Where:** method §7.5.
- **Problem:** SPEC §5.8's permissive-licence criterion concerns codecs. Development and test dependencies, and reference runtimes under `tools/` (F-20), are outside the product.
- **Required fix:** Scope R2 to dependencies shipped in production crates.

---

## J. Status rules

### J01 [BLOCKER] HC_UNVERIFIED missing from DECIDED conditions

- **Where:**
  - method §8.2 (no HC-verification condition);
  - §8.4 cap "a `PLATFORM_BLOCKED` part of the scope, with a rationale that the result is platform-independent" (MEDIUM, `DECIDED`);
  - objective §2.0 rule 4 (blocks `DECIDED` "except for a decision whose scope explicitly excludes the unverifiable platform **or property**"), §4.6, §5 threat 2 ("Until [instruments] exist, those HCs are HC_UNVERIFIED for every candidate").
- **Problem:**
  - The method never checks HC verification status.
  - The objective's "or property" lets a decision exclude, for example, crypto correctness from its scope and still be `DECIDED`.
  - The method's PLATFORM_BLOCKED cap contradicts the objective's block.
- **Required fix:**
  - Add §8.2 condition 13: every applicable HC (from the K01 `hc_ids`) is PASS on the conformance corpus and on tuning, validation and held-out.
  - `HC_UNVERIFIED` is allowed only for a platform explicitly excluded from `decision_scope`, never for a property.
  - A PLATFORM_BLOCKED platform inside scope yields `INSUFFICIENT_EVIDENCE`.
  - Delete "or property" from the objective and remove the method cap.

### J02 [HIGH] Held-out replication is too weak

- **Where:** method R5(c) "the held-out point estimate lies on the same side of the band centre … If the held-out point estimate falls inside the band, the verdict is attenuated (confidence at most MEDIUM)"; §8.4 MEDIUM permits `DECIDED`.
- **Problem:** Under a null true effect, the held-out point estimate falls on the favourable side about 50% of the time [DERIVED], so a false validation selection passes R5 at MEDIUM roughly half the time. With about 3 held-out groups per family, R5(e) unanimity rarely triggers either.
- **Required fix:**
  - Every supporting verdict must replicate as non-inferiority on held-out: the one-sided 0.95 CI bound on the unfavourable side lies inside the band (corrected CI per F01).
  - An attenuated verdict can never be the sole support of a selection.
  - Pre-register held-out power per decision (the MDE given held-out groups), and route decisions whose MDE exceeds 2 bands to `INSUFFICIENT_EVIDENCE` before unlock.

### J03 [HIGH] MEDIUM confidence permits DECIDED under blocking caps

- **Where:** method §8.4 caps; LOW only at three or more caps.
- **Problem:** A decision can be `DECIDED` with any two of these: unresolved contamination (L5), an amendment that changes the result, a `post-unlock-fix`, `SIMULATED` usability as primary evidence, or a compromise selection. Irreversible T3 and T4 decisions (wire, identity, crypto construction) need no more confidence than a planner default.
- **Required fix:**
  - Split the caps. **Blocks** (status `INSUFFICIENT_EVIDENCE`): L5 unresolved, amendment changes the result, post-unlock-fix, `SIMULATED` primary evidence for a human-facing claim, and HC_UNVERIFIED in scope. **Soft caps** limit confidence to MEDIUM.
  - T3 and T4 selections require HIGH, or an owner acceptance record naming each cap.

### J04 [HIGH] Decision type is self-selected

- **Where:** method §1 decision types. FORMAL skips R5 and records sensitivity "not applicable". The ledger has no `decision_type` field. The FORMAL reviewer may be another program agent.
- **Problem:** Classifying a decision as FORMAL avoids held-out and sensitivity checks.
- **Required fix:**
  - Add a ledger field `decision_type`, assigned at pre-registration by a session not involved in the candidates.
  - Any selection rationale that relies on measured size, time, memory, leakage or usability forces EMPIRICAL or HUMAN-FACING for those parts.
  - FORMAL requires a committed counterexample-search protocol (generator, budget, result) and reviewer sign-off.

### J05 [HIGH] R10 defaults to the cheapest candidate under INCONCLUSIVE

- **Where:** method R10 (keys: tier, spec words, reader LoC, status quo, id), §10.2 CB-1 (triggers only when the *status-quo key* decides ≥ 50% in a cluster).
- **Problem:** Tier is the first key and usually differs, so the status-quo key rarely decides. Underpowered experiments (F02, F08) then default to the lowest-tier candidate, often the status quo, at MEDIUM confidence without firing CB-1.
- **Required fix:**
  - CB-1 counts every R10 selection made under `INCONCLUSIVE`, whichever key decides.
  - R10 under `INCONCLUSIVE` may reach `DECIDED` only after demonstrated power: the median item CI half-width at most 0.5 × band on every primary metric, and corpus-level MDE at most 1 band. Otherwise the result stays `INSUFFICIENT_EVIDENCE` with a provisional selection.

### J06 [MEDIUM] Gate audit, counterevidence and the amendment loophole

- **Where:** method §8.2 (conditions 4, 7, 11 and 12 are "checked by the gate audit"), condition 5 ("or the search procedure followed and 'none found'"), §0.2 ("unless the amendment corrects a demonstrable error (a failing test or a counterexample)").
- **Required fix:**
  - Define the gate audit: who runs it (a session not involved in the decision), when (before any status change), and the checklist artifact path.
  - Counterevidence must cite a committed adversarial-search artifact with inputs and outputs.
  - An amendment justified by a counterexample needs an independent reviewer and must pass the §14 item 9 rule simulation.

### J07 [MEDIUM] Human-facing decisions on simulated evidence

- **Where:** method §4.16, §8.3 ("human-participant evidence, where simulated evidence is insufficient for the claim being made"). 34 ledger rows have `blocker_class` HUMAN_PARTICIPANTS.
- **Required fix:**
  - Any claim about human comprehension, task success or preference either requires external or human review (`EXTERNAL_REVIEW_REQUIRED`) or must be removed from the decision.
  - Simulated sessions may decide only mechanically checkable properties, such as flag counts, unsafe defaults and message content censuses.
  - List the 34 affected decisions.

---

## K. Ledger integration

### K01 [BLOCKER] Ledger schema cannot carry the method

- **Where:** method R3 ("An optimization dimension that `archetypal-objective.md` assigns to the decision"), §8.2, §14 item 6; ledger fields (Appendix R.2).
- **Problem:** The objective assigns no ODs to decisions. The ledger has none of these fields: `od_ids`, `hc_ids` with per-HC status, `decision_type`, primary metrics, cost tier, validation-look count, freeze wave, or governing method SHA. The R3 block and the §14 validator are therefore unenforceable, and primary-metric selection is unconstrained (D09).
- **Required fix:**
  - Define ledger schema v2 with those fields.
  - Generate an initial `od_ids` assignment per decision from the Appendix C.3 screen plus cluster, then have a session outside the program review it.
  - Commit the schema and assignments before the first experiment record.
  - Extend the §14 validator to enforce R3, J01 and J04.

### K02 [MEDIUM] The method's own decisions lack required evidence

- **Where:** method front matter (DEC-ECO-076, -078, -080, -082 and -083 stay `INSUFFICIENT_EVIDENCE` "until … rule simulation … and A/A calibration").
- **Problem:** DEC-ECO-078's required evidence is a rule simulation that "shows tie, noise and conflicting-family behaviour". F01, H01 and J02 show that such a simulation finds defects. After pre-registration, §0.2 makes fixing them costly.
- **Required fix:** Run the §14 item 9 rule simulation, extended with the F01 coverage, H01 tie and J02 null-pass scenarios, *before* the pre-registration commit. Record its outputs under `research/methods/` and cite them in DEC-ECO-078.

### K03 [LOW] thresholds.json status label

- **Where:** method Appendix A `"status": "pre-registered"`.
- **Required fix:** Use `"draft"` until the revised commit, then `"pre-registered"` with the commit SHA.

---

## Round-2 acceptance checklist

Round 2 passes only if every item holds.

| # | Check | Findings |
|---|---|---|
| 1 | One decision procedure, one parameter table, one metric crosswalk (M-number ↔ canonical name ↔ T-id ↔ family ↔ role), all generated or cross-checked by script. | A01, A06, D01, I04 |
| 2 | I24, library-version decode steps and freeze handling are each defined once, consistent with SPEC §5.4, §7.6, §9.8, §23.4, §25.3, HC-16 and CONTRIBUTING. | A02-A05 |
| 3 | `constraint-crosswalk.csv` covers all 989 free-text constraints; ledger schema v2 has `hc_ids`, `od_ids`, `decision_type`, tier, looks and wave; the validator enforces them. | B01, K01, J04 |
| 4 | New or extended MVTs: input consistency, CPU complexity, allocator-exact HC-06, nondeterminism artifact rule, independent escape oracle, triage rule, normative-statement grammar, path coverage, crash consistency and origin safety, HC-14 (f)-(i), C02 items. | B02-B16, C02 |
| 5 | Corpus CI procedure reaches nominal coverage ±1 pp in committed simulation on the actual group structure; A/A′ and known-effect calibration pre-registered; frequency covariate and per-core-class guard specified. | F01-F05, F08 |
| 6 | Held-out sealing option chosen and executed; content-overlap audit passes; single-wave or tranche design fixed; post-unlock-fix blocks `DECIDED`. | G01-G05 |
| 7 | Sensitivity on continuous band units with ties reported as "no measured winner"; selection bootstrap added. | H01, H02 |
| 8 | Status rules: HC_UNVERIFIED condition, non-inferiority held-out replication, cap-versus-block split, R10 power requirement. | J01-J05 |
| 9 | §14 item 9 rule simulation committed, including the scenarios named in K02. | K02 |

---

## Appendix R. Reproduction

All commands run from `D:/Projects/entrybound/entrybound` unless stated. Outputs quoted above are from these commands at the hashes in the front matter.

### R.1 Invariant and SPEC checks

```sh
# SPEC Appendix A and §3.9 (read in full)
sed -n '307,345p;2688,2724p' ../design/2026-08-29-entrybound-product-architecture.md
```

### R.2 Decision-ledger profile

```sh
C:/Python313/python.exe - <<'EOF'
import json, collections, re
rows=[json.loads(l) for l in open('research/decision-ledger.jsonl',encoding='utf-8') if l.strip()]
print(len(rows), collections.Counter(k for r in rows for k in r))
print(collections.Counter((r['status'],r['blocker_class']) for r in rows))
print(collections.Counter(r['release_relevance'] for r in rows))
nonI=[r for r in rows if any(not re.fullmatch(r'I\d+',h) for h in r['hard_constraints'])]
distinct=collections.Counter(h for r in rows for h in r['hard_constraints'] if not re.fullmatch(r'I\d+',h))
print('decisions with free-text constraints',len(nonI),'distinct free-text',len(distinct))
kw=['crypto','planner','chunker','gear','stream-layout','layout','descriptor','unsafe','zip','tar','7z','export','receipt','posix','security metadata','identifier','feature bit','reused','signature','recipient','staging','random access','http','index','group_ref','stored_length','24 item','non-goal','live infrastructure','key-management','transport','normalis','name bytes','research','default-off','production semantics','entry-v1','historical']
print('no F-register keyword (crude)', sum(1 for h in distinct if not any(k in h.lower() for k in kw)))
for p in [r'5\.7a', r'algorithmic complexity|quadratic|hash[- ]flood|CPU work']:
    print(p, [x['decision_id'] for x in rows if re.search(p, json.dumps(x, ensure_ascii=False), re.I)])
EOF
```

Results: 593 rows; 577 `INSUFFICIENT_EVIDENCE`, 16 `EXTERNAL_REVIEW_REQUIRED`; blocker classes EVIDENCE 542, HUMAN_PARTICIPANTS 34, EXTERNAL_REVIEW 16, PLATFORM 1; `V1_BLOCKING` 177. 545 decisions carry free-text constraints; 989 distinct strings; 562 match no F-register keyword. §5.7a appears in DEC-CMP-028, DEC-ECO-066 and DEC-PLT-017. The complexity pattern matches no decision. No ledger field exists for OD, HC id, decision type, tier or method SHA.

### R.3 Normative keyword counts

```sh
grep -c "MUST" ../design/2026-08-29-entrybound-product-architecture.md        # 3 lines
grep -o -i -w "must" ../design/2026-08-29-entrybound-product-architecture.md | wc -l   # 92
cat docs/*.md | grep -c "MUST"                                                  # 6 lines
cat docs/*.md | grep -o -i -w "must" | wc -l                                    # 72
```

### R.4 Section 2.1 versus Appendix A consistency

```sh
C:/Python313/python.exe - <<'EOF'
import re, collections
t=open('research/archetypal-objective.md',encoding='utf-8').read().splitlines()
s21={}
for l in t:
    m=re.match(r'\| (HC-\d\d) \| [^|]+\| ([^|]+)\| F-',l)
    if m: s21[m.group(1)]=(set(re.findall(r'I\d+',m.group(2).split(';')[0])), set(re.findall(r'I\d+',m.group(2))))
appA=collections.defaultdict(set)
for l in t:
    m=re.match(r'\| (I\d+) \| [^|]+\| [^|]+\| ([^|]+)\|',l)
    if m and 'HC' in m.group(2):
        for h in re.findall(r'HC-\d\d',m.group(2)): appA[h].add(m.group(1))
for h,(prim,allv) in sorted(s21.items()):
    if allv-appA[h]: print(h,'missing from Appendix A:',sorted(allv-appA[h]),'primary:',sorted(prim-appA[h]))
EOF
```

### R.5 Sensitivity tie and bootstrap coverage simulation

Run in the WSL research venv, which has numpy and the ebr dependencies:

```sh
wsl -d Ubuntu -u root -- /root/eb-research/venv/bin/python sim.py
```

`sim.py`:

```python
import sys, numpy as np
sys.path.insert(0,'/mnt/d/Projects/entrybound/entrybound/research/tools')
from ebr import stats
q = np.zeros((3,5))   # every candidate within one band on every metric
r = stats.weight_sensitivity(-q, [0.2]*5, dirichlet_samples=10000, dirichlet_concentration=4.0, seed=1)
print('FPW grid', r.grid_fraction_preserving, 'dirichlet', r.dirichlet_fraction_preserving, 'base_winner', r.base_winner)
rng=np.random.default_rng(7)
def cov(G, conf, nsim=2000, B=2000, sg=0.05, sf=0.03):
    c=0
    for s in range(nsim):
        fm=rng.normal(0,sf,len(G))
        data=[rng.normal(fm[i],sg,g) for i,g in enumerate(G)]
        truth=fm.mean(); reps=np.zeros(B)
        for d in data:
            idx=rng.integers(0,len(d),(B,len(d))); reps+=d[idx].mean(axis=1)
        reps/=len(G)
        lo,hi=np.quantile(reps,[(1-conf)/2,1-(1-conf)/2],method='linear')
        c+= lo<=truth<=hi
    return c/nsim
Gval=[1]+[2]*9+[3]*9+[4]            # validation groups per family (R.6)
Ghel=[1]+[2]*4+[3]*10+[4]*4+[6]     # held-out groups per family (R.6)
for name,G in (('validation',Gval),('heldout',Ghel)):
    for conf in (0.95,0.99,0.999): print(name, conf, cov(G,conf))
```

Output:

```
FPW grid 1.0 dirichlet 1.0 base_winner 0
validation 0.95 0.8245 | 0.99 0.9085 | 0.999 0.9585
heldout    0.95 0.8365 | 0.99 0.929  | 0.999 0.962
```

The simulation uses `method='linear'`, which is type 7, matching `stats.PERCENTILE_METHOD`. Family means are fixed strata and the estimand is the mean of true family means, which matches the §4.10 design.

### R.6 Corpus split structure and held-out exposure (counts only)

```sh
C:/Python313/python.exe - <<'EOF'
import json, collections as c, glob, os
m=json.load(open('research/corpus/manifest.json',encoding='utf-8')); items=m['items']
print(sorted({k for i in items for k in i}))
g=c.defaultdict(set)
for i in items: g[(i['family'],i['split'])].add(i['independence_group'])
fams=sorted({i['family'] for i in items})
for sp in ('tuning','validation','heldout'):
    print(sp, sorted(c.Counter(len(g[(f,sp)]) for f in fams).items()))
gs=c.defaultdict(set)
for i in items: gs[i['independence_group']].add(i['split'])
print('groups spanning splits', sum(1 for v in gs.values() if len(v)>1))
os.chdir('research/corpus')
for p in ['statistics.json','coverage.md']+glob.glob('sources/*.json')+glob.glob('generators/**/*.*',recursive=True):
    if not os.path.isfile(p): continue
    n=open(p,encoding='utf-8',errors='ignore').read().count('heldout')
    if n: print(p, n)
fps=glob.glob('fingerprints/*.json'); print(len(fps), sum('heldout' in os.path.basename(f) for f in fps))
EOF
```

Results:

- Groups per family: tuning {2: 6, 3: 6, 4: 5, 5: 2, 6: 1}; validation {1: 1, 2: 9, 3: 9, 4: 1}; held-out {1: 1, 2: 4, 3: 10, 4: 4, 6: 1}.
- Groups spanning splits: 0.
- Manifest item keys include `description`, `provenance`, `source_file`, `materialization_key` and `materialized_relpath`.
- Fingerprint files: 212 with 70 held-out names at the first run, 251 with 82 at the second.
- "heldout" occurrence counts as quoted in G01. Working-tree counts vary while uncommitted corpus work continues. The committed `manifest.json` hash did not change between the two runs.

## Review disposition

| Field | Value |
|---|---|
| Date | 2026-09-16 |
| Revised documents | `research/archetypal-objective.md` and `research/decision-method.md`, round-1 revision, committed with subject "research: pre-register decision method and archetypal objective" |
| Review text | Unchanged above this section. SHA-256 of this file before the section was added: `6af01821bff38136d53e5033446e9aa819ff51d791028c3076f30e32f41a83f7`. |
| Per-finding table | `decision-method.md`, Revision history, "Round-1 dispositions" (all 85 findings, with sections). `archetypal-objective.md` has the subset whose fix changes it. |
| Totals | 84 `FIXED` (two of them, F01 and F08, with a part rejected below), 1 `ACCEPTED_RISK` (C03), 0 wholly `REJECTED`. Every BLOCKER and HIGH finding is `FIXED`. |
| Revision-session disclosure | While checking repository state, the revision session's `git status --short` displayed untracked fingerprint file names, four of which are held-out item identifiers in three families. No held-out content, listing, statistic or path was opened, and the identifiers are not repeated in any committed file. The exposure is recorded as event L7 in `decision-method.md` §0.1 and §5.8. |

This section argues every rejected part and accepted risk, which round 2 re-reviews, and lists the fixes whose required artifact is an execution gate rather than part of the revision commit.

### RD-1. Partially rejected: F01 acceptance criterion (over-coverage)

- **Adopted.** The percentile cluster bootstrap is withdrawn. The corpus-level interval is the Welch-Satterthwaite t interval on group-level values, with the pooled within-family variance for single-group families (`decision-method.md` §4.10). Its coverage is checked on the actual validation and held-out group structures in committed simulation (Appendix D.1), and the check is a pre-registered admission criterion.
- **Rejected part.** The required fix asks for nominal coverage "within ±1 percentage point". The revision adopts the lower side only: coverage no more than 1.0 pp **below** nominal at 0.90, 0.95, 0.99 and 0.998. Over-coverage is allowed and reported.
- **Argument.**
  1. The BLOCKER is anti-conservatism: under-coverage makes `DISTINGUISHABLE` verdicts, and therefore eliminations and selections, too easy. Over-coverage widens intervals. That lowers power and makes `DISTINGUISHABLE` verdicts rarer; it cannot create a false elimination or a false selection.
  2. The power cost is controlled separately. The MDE gate (`decision-method.md` §4.12) blocks a confirmatory look whose MDE exceeds 1 band unit, so an over-covering interval shows up as a failed gate, not as a hidden bias.
  3. A two-sided criterion would leave no admissible procedure. Appendix D.1 shows the Welch-Satterthwaite interval at 0.9233 under heavy tails at nominal 0.90 (+2.3 pp), and the pooled-variance t interval, which is exact only for homoscedastic Gaussian data, at 0.9235 under heteroscedasticity at nominal 0.95 (−2.65 pp). With 1-4 groups in most families, no simple interval is exact across plausible generating models, and the conservative direction is the one a pre-registration should accept.
- **Round-2 check.** The Appendix D.1 table: the largest shortfall of the admitted interval is 0.27 pp (held-out, heteroscedastic, nominal 0.99).

### RD-2. Partially rejected: F08 non-inferiority at family level

- **Adopted.** The confirmatory set is limited to the provisional winner against each other survivor (§4.12); superiority uses Bonferroni over the declared superiority metrics (Holm's first step); every supporting condition requires `NON_INFERIOR` at corpus level (R4 condition 2, R5 (d)); the program-level false-selection rate comes from the rule simulation, and a Benjamini-Yekutieli listing is reported.
- **Rejected part.** The fix says supporting conditions require non-inferiority. At **family** level (R4 condition 3, R8 condition 3, R5 (e)) the revision instead requires a harm guard: the family's pooled-variance two-sided 0.90 interval is not `DISTINGUISHABLE_WORSE`, and the family point estimate lies no more than 1 band unit beyond the band on the unfavourable side, per family, tier and evaluation condition.
- **Argument.**
  1. Family-level non-inferiority is unreachable with this corpus even when the true effect is zero, which would make every universal default and most scoped winners `INSUFFICIENT_EVIDENCE` by construction. Illustration with the formulas of §4.10: take a between-group standard deviation of 0.03 in log units, a 5% band (ln 1.05 ≈ 0.0488), a 2-group family and pooled degrees of freedom 30. The family standard error is 0.03/√2 ≈ 0.0212. At the non-inferiority confidence of 0.99, t ≈ 2.75, so the upper bound is the estimate plus about 0.058, and non-inferiority needs the estimate below about −0.0095. Under a zero true effect that happens with probability of about one third per family; requiring it in each of about 20 families passes almost never.
  2. Round 0's absence-of-evidence problem is still closed. The guard's harm test uses 0.90, more sensitive than the corpus tests, and the point bound blocks any observed family regression larger than one band unit beyond the band even when it is not statistically distinguishable.
  3. The inferential burden sits where the data can carry it: corpus-level non-inferiority over about 50 groups with Welch-Satterthwaite degrees of freedom.
- **Residual risk**, recorded as a limitation of every scoped claim (`decision-method.md` §4.9): a family regression of up to one band unit beyond the band can pass the guard when it is not distinguishable.
- **Round-2 check.** Whether the full rule simulation (`decision-method.md` §14 item 9) should compare the guard against a family-level non-inferiority test at a larger band multiple.

### RD-3. Accepted risk: C03 (LOW)

- **Finding.** `objective_screen.py` counts only exact `I<n>` constraint tokens, so objective Appendix C.1 undercounts the decisions citing an invariant.
- **Why not fixed in the revision commit.** The fix is in `research/tools/ledger/objective_screen.py` and requires regenerating Appendix C; the revision commit is limited to its four documents.
- **Why the risk is acceptable.** After this revision no rule uses the C.1 counts: the HCs applicable to a decision come from the constraint crosswalk and the ledger `hc_ids` field (`archetypal-objective.md` §2.0 rule 7; `decision-method.md` R1). The undercount affects a descriptive table only. The limitation is stated next to Appendix C, and the pattern fix is `decision-method.md` §14 item 11, tied to the crosswalk gate G-A, so it lands before any decision relies on HC applicability.

### RD-4. Fixes whose required artifact is an execution gate

For these findings the revision fixes the rule in the documents, and the artifact the fix requires lies outside the revision commit's four files. Each artifact is a gate (`decision-method.md` §0.4, §14): until it exists, the rule cannot be exercised and no dependent decision can reach `DECIDED` (§8.4 blocks). All of them are inputs produced before any decision-relevant result, so creating them after the revision commit does not weaken the pre-registration.

| Finding | Rule fixed in | Artifact and gate |
|---|---|---|
| B01 (BLOCKER) | R1; objective §2.0 rule 7 | `research/methods/constraint-crosswalk.csv` with sign-off; G-A (§14 item 10) |
| K01 (BLOCKER) | §14 item 12; §8.2 items 13-15 | Ledger schema v2, reviewed `hc_ids` and `od_ids` assignment, validator; G-A |
| G01 (BLOCKER) | §5.4 (option (b) adopted; option (a) as the route that lifts the cap) | Transcript audit, role-separation record, removal or encryption of materialized held-out content; optional sealed tranches; G-C (§14 item 15) |
| A05 (HIGH) | §2.0; §4.9; Appendix B | `research/methods/workload-mix.json` (G-B); DEC-CMP-035 ledger note at the schema-v2 migration (G-A) |
| G02 (HIGH) | §5.3 | Lineage regrouping and content-overlap audit; G-C |
| G04 (HIGH) | §5.2 | `research/decisions/validation-looks.jsonl`; G-A |
| I02 (HIGH) | §7.2 | C1-C3 scripts and C4-C7 assessment template; before the first validation look |
| B04 (HIGH) | objective MVT-06(a) | Counting allocator and H per candidate; before the candidate's first HC-06 record; HC_UNVERIFIED until then |
| G07 (MEDIUM) | §5.4 | Materialization change; G-C |
| K02 (MEDIUM) | Appendix D | The three scenarios the finding names (F01 coverage, H01 ties, J02 null pass) were run before the revision commit and are recorded in Appendix D rather than as separate files under `research/methods/`, because the commit is limited to its four files. The full R0-R10 rule simulation remains §14 item 9, required before the first validation look. |

**Why this meets "fix before the pre-registration commit" for the BLOCKERs B01, K01 and G01.** Each BLOCKER was a rule that was contradictory, could not be executed, or let a decision reach `DECIDED` on invalid grounds. After revision each rule is executable as written, states what it consumes, and blocks `DECIDED` until that input exists. Round 2 should verify that no path to `DECIDED` bypasses these gates.

### RD-5. Round-2 acceptance checklist status at the revision commit

| # | Status | Where |
|---|---|---|
| 1 | Met in text and checked by script | `decision-method.md` §2.0, Appendix A.2 and A.3 (no mismatches; every M-number mapped), Appendix C.1, §7.6 |
| 2 | Met in text | R1, R2, §7.2, §7.4, Appendix B.1; objective HC-11, HC-14, HC-16 |
| 3 | Rules and schema defined; artifacts pending (G-A) | R1; §14 items 10-12 |
| 4 | Met in text | objective MVT-01(d), MVT-02(e), MVT-04 items 8-10, MVT-05, MVT-06(a) and (d), MVT-07(d), MVT-10(d)-(f), MVT-11(a), MVT-12(b), MVT-13(a), MVT-14(e)-(j), MVT-15(b), MVT-16(c) and (d), MVT-17(d), HC-18 |
| 5 | Met, with the one-sided coverage criterion of RD-1 | §4.10 and Appendix D.1; §4.7; §4.2; §4.4 |
| 6 | Option chosen, single freeze fixed, post-unlock block in place; audits pending (G-C) | §5.3-§5.8 |
| 7 | Met | §6.2, §6.7; Appendix D.3 |
| 8 | Met | §8.2 item 13; R5; §8.4; R10 |
| 9 | Named scenarios committed in Appendix D; full rule simulation pending (§14 item 9) | Appendix D |

## Revision history

| Date | Change |
|---|---|
| 2026-09-16 | Round 1 adversarial method review of `archetypal-objective.md` (`f9aea31e…`) and `decision-method.md` (`ca23c8a2…`): 85 findings (10 BLOCKER, 36 HIGH, 33 MEDIUM, 6 LOW), required fixes, round-2 acceptance checklist, reproduction appendix. |
| 2026-09-16 | Review disposition added by the round-1 revision session: totals, the partial rejections of F01 and F08 with arguments (RD-1, RD-2), the C03 accepted risk (RD-3), fixes whose artifacts are execution gates (RD-4), and round-2 checklist status (RD-5). The review text above the disposition is unchanged. |
