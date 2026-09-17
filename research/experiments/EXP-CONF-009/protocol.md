# EXP-CONF-009 — Persistent coverage-guided fuzzing to the adequacy ceiling, with minimization, triage and regression promotion

| Field | Value |
|---|---|
| Status | `NEEDS_TOOLING` (CC§2): all 26 targets of EXP-CONF-008 §3, campaign manager, crash deduplicator, minimizer wrapper, triage log tooling, regression ledger and promotion script, private finding store (CC§8) |
| Domain / program sections | conformance / §30 (primary), §29 (regression cases) |
| Decision IDs informed | DEC-ECO-029, DEC-ECO-028, DEC-CRY-006, DEC-CRY-016, DEC-CRY-017, DEC-CMP-005, DEC-CMP-010, DEC-CMP-016, DEC-CMP-037, DEC-CON-018, DEC-INT-008, DEC-ACC-057, DEC-CRY-029, DEC-CRY-067, DEC-LEG-059, DEC-LEG-064, DEC-LEG-095, DEC-MOD-006, DEC-MOD-038, DEC-PLT-054 |
| Requirements | REQ-ECO-0054, REQ-ECO-0134 |
| HC oracles | objective M24.2 `unresolved_fuzz_findings_at_coverage_target` (HC-03/HC-06); proposal for the assigner: `hc_ids` HC-03, HC-06, HC-15; `od_ids` OD-24 |
| Decision type | not assigned (gate G-A) |
| Method / pre-registration | `decision-method.md` @ `14b977c`; design pre-registration (CC§3) |
| Author / exposure | CC§1 |
| Depends on | EXP-CONF-008 (targets, builds, seed corpora); EXP-CONF-004 (conformance cases for promotion) |

## 1. Question

When every attacker-facing parsing boundary is fuzzed to the objective's pre-registered ceiling, how many distinct crash,
panic, out-of-memory and timeout defects remain unresolved at each candidate adequacy criterion's stop point, what
classes and boundaries do they fall in, and is every finding minimized, triaged and promoted to a named regression and,
where it exposes a format rule, a conformance case?

## 2. Hypotheses and falsification

- **H1 (M24.2).** `unresolved_fuzz_findings_at_coverage_target` = 0 for every target at the objective's stop rule (the
  earlier of the pre-registered coverage target and 72 CPU-hours) on the evaluated `ebound-head` commit. *Falsified* by
  one deduplicated finding without a fix at that commit; each is an HC-03 or HC-06 defect (objective M24.2) that blocks
  `DECIDED` for the affected component.
- **H2 (promotion completeness).** Every deduplicated finding has a regression entry with minimized bytes (or, under
  CC§8, their SHA-256), a triage record, and, if it exposes a format rule, a conformance case with an asserted class and
  code. *Falsified* by one finding without these.
- **H3 (retrospective criterion invariance).** The set of findings detected by each candidate criterion's stop point is
  identical to the set at 72 CPU-hours. *Falsified* by one finding first detected after some criterion would have
  stopped (recorded per criterion; evidence for DEC-ECO-028).

## 3. Campaign design

- **Every target runs to the ceiling.** Each of T01-T26 runs in mode M-ON for **72 CPU-hours** (libFuzzer fork mode,
  4 workers, seed `2809171001`, sanitizer address, `-rss_limit_mb=2048`, `-timeout=10`). Digest-gated targets (T01-T06,
  T09, T10, T13-T15, T20, T21, T23, T24, T26) additionally run M-BYPASS for 24 CPU-hours. Running every target to the
  ceiling lets every DEC-ECO-028 criterion (EXP-CONF-008 §4) be evaluated retrospectively on the same trace, so this
  experiment does not depend on DEC-ECO-028's outcome.
- **Memory-ceiling arm** (DEC-CMP-016, DEC-INT-008): T13, T14 and T15 also run 24 CPU-hours each with
  `-rss_limit_mb` set to the status-quo decoder working set (384 MiB) plus the process baseline measured at start, to
  record OOM findings that a lower ceiling exposes.
- **Structure-aware variants** from EXP-CONF-008 are included for T02, T04 and T13 within their budgets (half the CPU
  budget each for byte-level and structure-aware harnesses).
- **Build**: `ebound-fuzz` at the `ebound-head` commit recorded in `record.md`; clean builds only (no defect patches).
  If production fixes land during the program, affected targets are re-run at the fixed commit for their full budget,
  and both runs are reported.

### 3.1 Candidates

- **Campaign configurations** (the compared objects): each target T01-T26 in M-ON; each digest-gated target in M-BYPASS;
  T13-T15 under the status-quo decoder memory ceiling. Reference: M-ON.
- **DEC-ECO-029 candidates executed here**: `cargo-fuzz-libfuzzer-per-boundary`, `structure-aware-arbitrary-generators`
  (T02, T04, T13), `regression-promotion-ledger` (§4 steps 6-7). Executed elsewhere: `afl-plus-plus-campaigns`
  (EXP-CONF-008 engine arm), `internal-differential-fuzzing` and `property-based-tests` (EXP-CONF-010),
  `incumbent-differential-fuzzing` (EXP-CONF-011 model fuzzing), `oss-fuzz-integration` (EXP-CONF-014, BLOCKED).
- **DEC-ECO-028 candidates**: every stop rule of EXP-CONF-008 §4 is evaluated retrospectively on these traces (H3).
- Candidates of the other decisions in the header are the ledger's (CC§1: none added); findings are evidence against the
  status-quo implementation of each mapped component.

## 4. Finding pipeline

1. **Collection**: every crash, leak, OOM and timeout artifact with its libFuzzer log.
2. **Deduplication** (`dedup.py`): key = (target, kind, first five symbolized frames inside `entrybound` crates, or panic
   message location for Rust panics, or the sanitizer report's first in-project frame for native code).
3. **Minimization**: `cargo +nightly-2026-08-01 fuzz tmin <target> <artifact> -runs=2000000` bounded to 10 wall
   minutes; corpus `cargo fuzz cmin` at the end of each campaign.
4. **Reproduction check**: the minimized input is re-run 3 times in a non-fork process on the same build; a finding that
   never reproduces is recorded `NONREPRODUCIBLE` and kept (not deleted, not counted as resolved).
5. **Triage** (session per CC§11, not a target author): class `HC-03 misinterpretation`, `HC-06 resource`,
   `HC-15 classification`, `panic on untrusted input`, `native-code memory error`, `harness bug`, `sanitizer false
   positive in dependency`, `duplicate`; affected decision ids; security relevance (CC§8).
6. **Promotion** (`promote.py`): regression ledger `research/conformance/regressions.jsonl` row
   (`FZ-<target>-<nnnnn>`, input SHA-256, dedup key, class, triage reference, `ebcc` case id or `none`, public/private
   status, first-seen commit, fixed-at commit); for findings that expose a format rule, a case in `ebcc-v1.1` family
   `F-FUZZ` with an expectation written from the cited normative statement (EXP-CONF-004 §5.3 discipline).
7. **Replay gate**: `replay_regressions.sh` runs every regression input on every new build; a regression that fails again
   re-opens its finding.

## 5. Inputs

Seed corpora of EXP-CONF-008 §6 (tuning items and generated cases only; CC§7), plus the minimized corpora produced by
EXP-CONF-008 runs of the same targets.

## 6. Environment and commands

WSL2 Ubuntu 24.04 x86-64; `nightly-2026-08-01`; cargo-fuzz 0.13.2; 8 campaigns in parallel (32 vCPUs). Never concurrent
with a timing window (CC§3). Resumable: `campaign.py` checkpoints the corpus every CPU-hour and resumes from it.

```sh
cd /mnt/d/Projects/entrybound/entrybound && export PYTHONPATH=research/tools
/root/eb-research/venv/bin/python -m ebr validate --spec research/experiments/EXP-CONF-009/spec.yaml
/root/eb-research/venv/bin/python -m ebr run --spec research/experiments/EXP-CONF-009/spec.yaml
python3 research/harness/fuzz/tools/dedup.py --exp EXP-CONF-009
python3 research/harness/fuzz/tools/minimize_all.py --exp EXP-CONF-009 --tmin-wall-s 600
python3 research/harness/fuzz/tools/promote.py --exp EXP-CONF-009 --ledger research/conformance/regressions.jsonl --private-store /root/eb-research/conformance/findings-private
python3 research/harness/fuzz/tools/criteria_eval.py --exp EXP-CONF-009 --criteria research/harness/fuzz/criteria-v1.json
bash research/harness/fuzz/tools/replay_regressions.sh --ebound-build head
```

## 7. Replication

One persistent campaign per (target, mode) with one libFuzzer seed; persistence (corpus carry-over) is the design, so
multiple seeds are not run here (seed variability is measured in EXP-CONF-008). Reproduction of each finding: 3 runs.
Coverage replays at 4 CPU-hour checkpoints are repeat-hashed.

## 8. Metrics (CC§9)

| Metric | Role | Unit | Definition |
|---|---|---|---|
| `unresolved_fuzz_findings_at_coverage_target` | binary (HC-03/HC-06) | count | deduplicated, reproducible findings without a fix at the evaluated commit, at the objective's stop rule; AGG-W sum over targets, reported per target and class |
| `decided_class_reach_fraction` | descriptive (M04.3) | fraction | per target at 72 CPU-hours (replayed corpus through the reason-code recording build) |
| `untrusted_input_loc` | cost input (C4) | LoC | lines reached by the fuzz corpora in functions of `entrybound` reachable from each boundary, from the coverage export (a lower bound of M24.1, reported beside the static count of the container domain if one exists) |
| Findings per criterion stop point; time to first finding; nonreproducible findings; promotion completeness | descriptive, non-canonical; H2 is binary by definition | count | §4 tools |

## 9. Normalization and analysis

AGG-W counts per target, class and decision (§1 mapping of targets to decisions: T12 → DEC-CRY-006; T08-T12 →
DEC-CRY-016/-017 dossier and acceptance inventory; T14/T15 → DEC-CMP-037; T13 → DEC-INT-008, DEC-CMP-010, DEC-CMP-016;
T19 → DEC-CMP-005, DEC-CMP-010; T03 → DEC-MOD-006, DEC-CON-018; T04 → DEC-ACC-057; T09 → DEC-CRY-029; T08 and T09 at
envelope limits → DEC-CRY-067; T22 → DEC-LEG-064; T20-T24 → DEC-LEG-059, DEC-LEG-095; T18 → DEC-MOD-038; T16 →
DEC-PLT-054). No statistics: counts of distinct defects. A finding is an R1 screen for the component; it never becomes a
rate. The analysis plan needs adoption by a non-exposed session (CC§1).

## 10. Practical-significance thresholds

§3.3 (binary, no band) for `unresolved_fuzz_findings_at_coverage_target` and H2; cost input without band.

## 11. Sensitivity analysis

- Findings under M-ON only versus M-ON ∪ M-BYPASS (bypass findings in code unreachable with valid digests are reported
  separately as "reachable only with forged digests": for unencrypted archives digests are unkeyed, so an attacker can
  recompute them and these findings still count).
- Deduplication with three versus five frames.
- Stop rules of every DEC-ECO-028 criterion (H3).

## 12. Split discipline and held-out placeholder

Seeds from tuning and generated inputs only. No validation or held-out use. At Commit A the regression replay gate is
run on the frozen build and cited by Phase D gate audits.

## 13. Expected negative results worth recording

- Panics on `expect()` paths under hostile lengths (REQ-ECO-0054 notes untested hostile-length classification).
- OOM findings in reconstruction decoders under the status-quo ceiling (DEC-CMP-037, DEC-CMP-016).
- DER/CMS timestamp parser findings in the pre-release `cms` dependency (DEC-CRY-006).
- Unbounded observation growth in legacy adapters (DEC-LEG-059).
- Findings that appear only after 24 CPU-hours (plateau criteria stopping too early; H3).

## 14. Threats to validity

- One seed per campaign: finding discovery is stochastic; absence after 72 CPU-hours is regression evidence, not proof.
- Harness bugs are triaged as such but can hide target defects behind harness crashes; each harness bug is fixed and the
  affected campaign resumed from its corpus.
- Sanitizer and fork-mode overheads reduce executions per CPU-hour relative to non-instrumented builds.
- Coverage-derived `untrusted_input_loc` is a lower bound (unreached code is uncounted).
- Public disclosure risk handled by CC§8.

## 15. Cost estimates

| Item | Estimate |
|---|---|
| Compute | 2,430 CPU-hours: 26 × 72 h M-ON = 1,872 h; 16 digest-gated targets × 24 h M-BYPASS = 384 h; memory-ceiling arm 3 × 24 h = 72 h; minimization, replays and coverage exports ≈ 100 h. ≈ 76 wall hours on 32 vCPUs, resumable |
| Disk | 80 GB (corpora, crash artifacts, coverage exports, builds) |
| Agent effort | **scripted** campaigns; **judgment, medium**: triage sessions sized by finding count |
| Platform | Linux x86-64 (WSL); CPU-saturating; no timing; no network |

## 16. Outcome obligations

`outcome.json` (§10.1); the regression ledger and `F-FUZZ` cases are committed (with CC§8 handling); each unresolved
finding is a defect work item (objective §2.0 rule 5) and appears in `counterevidence` of the decisions mapped in §9.
