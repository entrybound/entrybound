# EXP-ECO-019: Simulated first-use sessions, output-interpretation probes and the usability-method pilot (SIMULATED, heuristic)

| Field | Value |
|---|---|
| Experiment ID | EXP-ECO-019 |
| Domain | ecosystem (program §33) |
| Status | NEEDS_TOOLING: sandboxed session harness, argument shims, output-variant renderer, answer checkers |
| Runner | agent-executed sessions orchestrated by `research/tools/usability/` (not an `ebr` spec: samples are agent sessions, not processes over corpus items) |
| Evidence label | `[EMPIRICALLY_MEASURED; SIMULATED]`; role `heuristic` (T-18); maximum evidence state `SUPPORTED_WITH_LIMITATIONS`; never sole support (§4.16, §8.4 block) |
| Conventions | `research/experiments/_index/ecosystem-conventions.md` (§5) |
| Timing | none (session wall time recorded only as `task_completion_time_s`, heuristic) |

## 1. Question

When independent agents who have never seen Entrybound's internals attempt pre-registered first-use tasks using only a CLI's own help text, or read pre-registered command outputs and answer factual questions about them, where do they fail, what do they misread, and do the failures concentrate on the concepts and outputs that the §33 decisions are about? And, for DEC-ECO-083, how well do simulated sessions, an independent cognitive walkthrough, mechanical task-complexity counts and a convention-conformance audit agree on which tasks are hard?

## 2. Hypotheses (heuristic; no CI-based verdicts)

These are pre-registered expectations whose outcome is reported as per-task success counts and coded failure modes (T-18). Each has a stated disconfirming observation, but none can eliminate or select a candidate.

- **H1 (verification semantics are misread).** On probes of signature status (valid, stale, unauthorized signer, no signatures) and of keyless verification of encrypted archives, fewer than 15 of 20 sessions answer "was the content authenticated by the expected signer?" correctly for the status-quo output. *Disconfirmed* if at least 15 of 20 answer correctly on every such probe.
- **H2 (loss is under-read).** On lossy-export dry-run reports and restoration reports, at least one quarter of sessions fail to list every lost metadata class. *Disconfirmed* if fewer than one quarter fail on every such probe.
- **H3 (profile choice).** Given workload descriptions, sessions choose the profile the SPEC positions for that workload (fast for CI artifacts; dense for release distribution; extreme for cold archival) in fewer than 15 of 20 sessions for at least one workload with names-only help (DEC-ECO-055 status quo). *Disconfirmed* if at least 15 of 20 choose correctly for every workload.
- **H4 (operational tasks).** Status-quo success counts on T01, T02, T03 and T06 are at least 18 of 20; on T04, T11, T15, T17 and T25 at most 15 of 20. *Disconfirmed* per task by the opposite.
- **H5 (method agreement, arm C).** Simulated-session task difficulty ranking and task-complexity ranking (EXP-ECO-018 `flags_per_task` + `concepts`) have Kendall τ ≥ 0.4 over the pilot tasks. *Disconfirmed* if τ < 0.4 (reported as a descriptive statistic of agreement, not an inferential test).

## 3. Decisions informed

Arm A and B (heuristic evidence): DEC-ECO-023, DEC-ECO-011, DEC-ECO-007, DEC-ECO-005, DEC-ECO-066, DEC-ECO-036, DEC-ECO-055, DEC-ECO-064, DEC-ECO-063, DEC-ECO-024, DEC-ECO-035, DEC-ECO-057, DEC-CMP-036, DEC-CMP-029, DEC-CMP-013, DEC-MOD-023, DEC-MOD-015, DEC-CRY-009, DEC-CRY-040, DEC-CRY-046, DEC-CRY-058, DEC-CRY-061, DEC-CRY-063, DEC-CRY-082, DEC-CRY-086, DEC-CRY-087, DEC-CRY-090, DEC-CRY-094, DEC-INT-004, DEC-INT-021, DEC-INT-038, DEC-INT-042, DEC-ACC-008, DEC-ACC-022, DEC-ACC-035, DEC-ACC-054, DEC-PLT-004, DEC-PLT-007, DEC-PLT-009, DEC-PLT-012, DEC-PLT-013, DEC-PLT-015, DEC-PLT-022, DEC-PLT-023, DEC-PLT-043, DEC-PLT-056, DEC-PLT-059, DEC-LEG-006, DEC-LEG-013.

Arm C: DEC-ECO-083.

## 4. Candidates

### Arm A — operational sessions (runnable candidates only)

| Candidate | Definition | Decisions |
|---|---|---|
| `ebound-status-quo` | dev-HEAD `ebound`, help text as shipped | all arm A decisions (reference) |
| `shim-spec-aliases` | a thin argument-translating wrapper exposing `create`/`c`, `extract`/`x`, `t` mapped to `pack`, `unpack`, `list`; help text from the committed synopsis | DEC-ECO-005 spec-aliases-only |
| `shim-restore-preset` | wrapper adding `--preserve=all|portable|none` mapped to the existing per-class unpack flags | DEC-ECO-011 preset-flag |
| `shim-split-verbs` | wrapper exposing `import` and `export` verbs mapped to `convert` directions, with SPEC flag names | DEC-LEG-013 split-import-export-verbs, spec-flag-names |
| `shim-workload-aliases` | wrapper adding `--for ci|release|archive` mapped to profiles | DEC-ECO-055 workload-named-aliases |
| incumbent controls | GNU tar (T01-T03, T06, T15, T16, T25), Info-ZIP (T01, T02, T06), 7-Zip (T01, T02, T06), age (T04, T05, T17), minisign (T11, T12) | capability-matched comparisons only |

Shims are pure argument translators (no behaviour change), committed with their help text before any session; their authorship and review follow conventions §1 (translation reviewed by the candidate owner). Tasks: T01-T06, T08, T09, T11, T13, T15, T17, T19, T25, T26, T27 of EXP-ECO-018 §5, with the same completion checkers. Each candidate runs only the tasks its decision concerns plus T01-T03.

### Arm B — output-interpretation probes (no runnable candidate needed)

Each probe shows a fixed output text and asks pre-registered questions with mechanically checkable answers (multiple choice or a JSON field list). Variants: the status-quo output captured from dev HEAD on a committed fixture, and candidate variants rendered from templates `variants/<decision>/<candidate_id>.tmpl` written by the candidate owners before any probe session (the template must render the same underlying facts).

| Probe | Output shown | Questions (examples) | Decisions |
|---|---|---|---|
| IP01-IP04 | `verify --signatures` for valid, stale-after-repack, unauthorized-key, no-signature archives | authenticated by the expected key? content current? | DEC-ECO-064, DEC-CRY-090, DEC-CRY-094, DEC-INT-038, DEC-CRY-082 |
| IP05 | `verify` of an encrypted archive without unlock material | was private content verified? is exit status success? | DEC-INT-021, DEC-INT-042 |
| IP06 | `read --access-report` for a remote read with partial verification | which bytes are verified? | DEC-ACC-035, DEC-INT-042 |
| IP07 | `diff` of remote archives reporting SAME under partial verification | were contents compared? | DEC-MOD-015, DEC-ACC-008 |
| IP08 | `convert --to tar --dry-run` LOSSY report | which classes are lost? what flag approves? | DEC-ECO-036, DEC-LEG-013 |
| IP09 | unpack restoration report with refusals | what was not restored and why? which grant would allow it? | DEC-PLT-013, DEC-PLT-015, DEC-PLT-007, DEC-PLT-043 |
| IP10 | `explain` output with NOT_RECORDED items | was the decision recorded, not applicable or prospective? | DEC-MOD-023, DEC-CMP-029 |
| IP11 | key add/remove output followed by `verify --signatures` showing STALE | is the signature still trustworthy for content? | DEC-CRY-087, DEC-CRY-009, DEC-ECO-063 |
| IP12 | STREAM window refusal message | what should be changed to succeed? | DEC-ACC-054 |
| IP13 | non-seekable input diagnostic | what went wrong? | DEC-ACC-022, DEC-ECO-024 |
| IP14 | confinement-mode report (kernel-enforced versus userspace) | is extraction confinement guaranteed on this platform? | DEC-PLT-004 |
| IP15 | Windows pack refusal on reparse points (abort) versus skip-with-report | which files are in the archive? | DEC-PLT-056 |
| IP16 | warning volume for a Linux home tree | which warnings require action? | DEC-PLT-059, DEC-ECO-024 |
| IP17 | legacy audit (`convert --strict --dry-run`) with annotations | is the zip safe to accept? which constructs are ambiguous? | DEC-ECO-035, DEC-LEG-006 |
| IP18 | `inspect --crypto` showing KDF parameters | are the parameters current guidance? | DEC-CRY-046 |
| IP19 | password strength warning variants | would the password be accepted? | DEC-CRY-058 |
| IP20 | exec-bit flip `diff` output (one record versus both tiers) | what changed? | DEC-PLT-023 |
| IP21 | profile help text and workload descriptions | which profile for CI / release / archival? | DEC-CMP-036, DEC-ECO-055 |
| IP22 | `key list` output with recipient directory labels | who can decrypt? | DEC-CRY-063, DEC-CRY-040 |
| IP23 | remote read failure diagnostics (server without ranges; ETag change) | what must the publisher fix? | DEC-ECO-057 |
| IP24 | plaintext export of an encrypted archive warning | is the output confidential? | DEC-CRY-061 |
| IP25 | refusal on existing destination | how to proceed without losing files? | DEC-INT-004, DEC-PLT-009, DEC-PLT-012 |
| IP26 | `pack --help` and `unpack --help` with class names | which option restores ACLs? | DEC-PLT-022, DEC-ECO-011, DEC-ECO-023 |
| IP27 | signature command synopsis variants | how to sign at pack time and list signatures? | DEC-CRY-086, DEC-ECO-007, DEC-ECO-066 |
| IP28 | override flags help (lookback, dictionary) versus profile-only help | how to make a smaller archive? | DEC-CMP-013 |

### Arm C — method pilot (DEC-ECO-083)

Pilot tasks: T01, T04, T08, T11, T15, T17 on `ebound-status-quo` and one incumbent control each. Methods compared: `simulated-first-use-sessions` (arm A data), `cognitive-walkthrough` (3 independent walkthrough sessions per task following a fixed four-question protocol per step, predicting failure points), `task-complexity-metrics` (EXP-ECO-018 counts), `convention-conformance-audit` (mechanical checklist over help, option syntax and exit statuses from EXP-ECO-015 against a pre-registered subset of POSIX utility syntax guidelines and clig.dev recommendations, each checklist item cited with URL and access date). `expert-heuristic-evaluation` and `later-human-beta-study` need external humans (EXP-ECO-021, BLOCKED); `status-quo-block-on-human-study` and `defer-usability-claims` are policy options with no measurement.

## 5. Inputs

Fixtures from EXP-ECO-018 `fixtures.py` (seed 2026091719 for session variants: names and contents vary per session so transcripts are not reusable across sessions); probe outputs captured from dev HEAD on committed fixtures; no corpus items.

## 6. Environment and platform requirements

- Sessions: a base model tier recorded per session (one tier per arm; cheaper tier allowed for arm B), fresh context, no repository access, no internet, no memory across sessions; the system prompt is the committed `session-prompt.md` (task statement, sandbox rules, "use only the tool's own help").
- Sandbox: WSL2, per session a scratch directory; commands executed through `sandbox_shell.py` which allows only the candidate binary, the fixture tools needed by the task (`ls`, `cat`, `head`, `file`, `sha256sum`) and `man`/`--help` of the candidate, runs under `unshare --net --map-root-user` (no network), caps tool calls at 40 and session wall time at 20 min, and logs every command and output.
- Orchestration: an agent-workflow script (or the Anthropic API if the owner provides access) launching sessions in seeded random order; the orchestrating session is not the designer of any candidate.

## 7. Procedure and commands

Tooling to build (`research/tools/usability/`): `sandbox_shell.py`; `shims/` (four wrappers, their help files); `render_variants.py` (templates → probe texts); `probe_checker.py` and `task_checker.py` (reusing EXP-ECO-018 checkers); `launch_sessions.py --arm A|B|C --seed 2026091719`; `code_failures.py` (failure-mode coding with a pre-registered codebook `codebook.yaml`: wrong command, wrong option, misread output, gave up, unsafe action taken, false success belief; two independent coders, agreement reported); `transcripts/` committed per session with the checker verdict.

Staging (budget-aware, never weakening the method): Stage 1 = arm C pilot (6 tasks × 2 tools × 20 sessions + 18 walkthroughs); Stage 2 = arm B (28 probes × status quo + candidate variants × 20 sessions); Stage 3 = arm A remaining tasks and shims. The owner decides whether to launch each stage from weekly usage; any cell with fewer than 20 sessions is reported as below the §4.6 simulated minimum and is not used.

## 8. Seeds, warmup, replication

Session order and fixture variation seed 2026091719; at least 20 sessions per (task or probe, candidate), randomized order (§4.6, §4.16). No warmups.

## 9. Metrics and measurement

| Metric | Canonical | Role |
|---|---|---|
| `task_success_rate` | yes (T-18, M25.1) | heuristic: per (task or probe, candidate) success count out of n |
| `task_steps` | yes (T-18, M25.1) | heuristic: commands issued |
| `task_errors` | yes (T-18, M25.1) | heuristic: failed commands and wrong answers |
| `task_completion_time_s` | yes (T-18, M25.5) | heuristic: session wall time |
| failure-mode codes, misconception quotes (short), coder agreement, method agreement (Kendall τ, Jaccard of predicted versus observed failure points) | no | record fields |

## 10. Normalization and aggregation

Per (task or probe, candidate) counts; AGG-S minimum over tasks for M25.1 as a reported headline; no pooling across candidates or tasks into a single score.

## 11. Statistical analysis and use in the decision rule

None beyond per-task success counts and qualitative failure modes (§4.16, T-18): sessions of one base model are correlated. Results may motivate candidates, identify where mechanical metrics miss problems, and appear in `known_limitations` and counterevidence; they are never the primary support of a selection (block, §8.4) and every human-comprehension claim is routed to `EXTERNAL_REVIEW_REQUIRED` (EXP-ECO-021).

## 12. Practical-significance thresholds

None: role `heuristic` (T-18, Appendix A.2). The "15 of 20" figures in §2 are expectation statements for heuristic reporting, not thresholds.

## 13. Sensitivity analysis

Results by model tier where two tiers were used; excluding sessions that hit the tool-call cap; coder A versus coder B failure codes.

## 14. Held-out (Phase D placeholder)

Conventions §5: sealed held-out tasks and probes authored before candidate iteration, run once after the freeze for every candidate that reached it, reported as heuristic evidence.

## 15. Expected negative results worth recording

Agents succeeding where humans would not (agents read help exhaustively); agents failing on shell mechanics unrelated to the CLI; candidate variants not improving interpretation over the status quo.

## 16. Threats to validity

Simulated users are not humans (the central limitation); one base model family; help-only access differs from real users who search the web; probe questions may cue answers (questions are reviewed by a second session for leading wording before any session).

## 17. Cost estimate

- Machine time: about 20 h of sandbox execution.
- Agent tokens (the scarce resource, §13): Stage 1 ≈ 258 sessions; Stage 2 ≈ 28 probes × about 3 variants × 20 = about 1,700 short sessions; Stage 3 ≈ 16 tasks × up to 3 candidates × 20 = about 1,000 sessions; order of 50-80 million tokens in total. Agent effort category: agent-executed sessions plus judgment for codebook coding and walkthroughs.
- Disk: about 10 GB (transcripts, scratch).

## 18. Gates and exposure

Conventions §1, §2 and §5. No corpus family is used. Session agents must not be sessions that designed CLI candidates or this experiment.
