# EXP-CRYPTO-022: Crypto CLI mechanical complexity and simulated first-use of trust and privacy output (HUMAN-FACING)

<!-- BEGIN AUTO:meta -->
<!-- END AUTO:meta -->

## 1. Question

For the crypto-facing CLI surfaces — verify output (validity, per-binding status, authorized signer, trusted timestamp, keyless "PRIVATE CONTENT UNVERIFIED"), signing-binding selection, `inspect` KDF-parameter reporting, password-strength and plaintext-downgrade warnings, recipient directory/key management, and the AUX-vs-LAI stale-content diagnosis — what are the mechanically checkable properties (flag and concept counts, unsafe defaults, message-content census), and what do independently simulated first-use sessions reveal as qualitative failure modes? Human comprehension, task success and preference are routed to external/human review; only the mechanical properties decide.

## 2. Hypotheses and falsification

- **H1 (mechanical).** Candidate output/flag designs differ in `flags_per_task`, concept count and `error_actionability_fraction` over a fixed task set; some status-quo surfaces have `unsafe_defaults` above 0 (for example a require-flag satisfied by any key, or an exit 0 on keyless-unverified without a machine-readable qualifier).
- **H2 (simulated, heuristic only).** Simulated sessions surface reproducible misinterpretations (for example reading "cryptographically valid" as "trusted", or "PRIVATE CONTENT UNVERIFIED, exit 0" as "verified"). These are qualitative failure modes, never a CI-based verdict.
- **Falsification:** H1 is a mechanical comparison at T-20 bands. H2 cannot be falsified statistically by design (T-18 heuristic); it can only surface or fail to surface failure modes.

## 3. Decisions informed and evidence route

<!-- BEGIN AUTO:candidates -->
<!-- END AUTO:candidates -->

Evidence route: HUMAN-FACING (decision-method.md §1.2, §4.16). **Mechanical properties (flag/concept counts, unsafe defaults, message census) are EMPIRICAL and may decide the mechanical parts.** Any claim about human comprehension, task success or preference is `EXTERNAL_REVIEW_REQUIRED` or removed (§4.16); simulated evidence is `[SIMULATED]`, capped at `SUPPORTED_WITH_LIMITATIONS`, and never the sole support (block, §8.4). The five HUMAN_PARTICIPANTS crypto rows (DEC-CRY-001, DEC-CRY-061, DEC-CRY-082, DEC-CRY-101, DEC-ECO-022) are HUMAN-FACING for their human claims.

## 4. Candidates and arms

Grouped by decision (candidate ids from the ledger):
- **DEC-CRY-090 / DEC-INT-038 / DEC-ECO-064** verify-output vocabulary and required-binding defaults;
- **DEC-INT-021** keyless verify exit and label;
- **DEC-CRY-082** allowed-key set and default (mechanical: does `--require-*` refuse without keys);
- **DEC-CRY-009 / DEC-ECO-063** binding-selection flags;
- **DEC-CRY-046** `inspect` KDF reporting;
- **DEC-CRY-058** password policy warnings;
- **DEC-CRY-061** plaintext-downgrade warning/flag;
- **DEC-CRY-004** stale-content LAI-vs-AUX diagnosis wording;
- **DEC-CRY-063** recipient directory contents and labels;
- **DEC-CRY-095 / DEC-CRY-108** key-file permission warning wording;
- **DEC-CRY-101** timestamp-request workflow messaging (mechanical parts; TSA client is a dependency-cost question in EXP-CRYPTO-013);
- **DEC-CRY-001 / DEC-ECO-022** recipient-type demand (POST_V1; demand survey is human/external, not decided here).

Reference candidate: the status-quo CLI output.

## 5. Corpus selectors

- **Task set (pre-committed, held-out task subset written before design iteration for the R5 mechanical part):** verify a signed archive; verify a stale-binding archive; verify an unauthorized-signer archive; verify a keyless-encrypted archive; sign with default bindings; sign content-only; inspect KDF parameters; create with a weak password; export encrypted to plaintext; add/remove a recipient. Completion criteria are mechanically checkable (exit code, presence of required message strings, no unsafe default triggered).
- **Base archives:** small generated and real (`f01-tuning-curl-8-19-0-git`, `f19-tuning-zoo`) archives in each state.
- No tuning/validation/held-out *corpus* item drives statistics; the corpus here is the task set and candidate CLIs.

## 6. Environment and platform requirements

`wsl-ubuntu` and `windows-host` (key-file permission messaging differs by OS). `timing: false`. Simulated sessions are run by agent sessions not involved in CLI design, which see only the CLI's own help/man text and the task statement (§4.16).

## 7. Commands and tooling

- `research/tools/crypto/cli_census.py`: counts flags and concepts per task, detects unsafe defaults, and runs the message-content census (`error_actionability_fraction`, presence of required strings) over each candidate CLI.
- `research/tools/crypto/first_use_sessions/`: a harness that presents help text and a task to an isolated agent session, records the commands issued, the completion outcome (by the mechanical criteria) and a qualitative failure-mode note. At least 20 sessions per (task, candidate), randomized order.
- Candidate output variants are prototyped as CLI wrappers over the library (production files not edited by the agent).

## 8. Seeds, warmup, repetitions and adaptive rule

- Mechanical census: deterministic, run once, repeat-hash.
- Simulated sessions: at least 20 per (task, candidate); randomized order; no statistical stopping rule (T-18 heuristic).

## 9. Measurement method and metrics

| Metric (canonical) | ID | OD | Role |
|---|---|---|---|
| `flags_per_task` | C6 | OD-25 | cost input / **primary mechanical** |
| `unsafe_defaults` | HC-05 (M25.4) | OD-25 | **binary** (= 0) |
| `error_actionability_fraction` | T-20 (M25.3) | OD-25 | **primary mechanical** |
| required-message presence (for example "no signatures present", downgrade recorded) | T-20 | OD-04/OD-25 | **primary mechanical** |
| `task_success_rate`, `task_steps`, `task_errors`, `task_completion_time_s` | T-18 | OD-25 | **heuristic only** (no CI verdict) |

## 10. Normalization and statistical analysis

- Mechanical metrics: T-20/C6 exact over the fixed task set; unsafe defaults binary.
- Simulated metrics: per-task success counts and qualitative failure-mode notes only; no confidence intervals (T-18: sessions of one base model are correlated).
- The mechanical parts get one held-out-task look (the pre-committed held-out task subset) for R5 of the mechanical claims.
- No human claim is decided; those are routed to `EXTERNAL_REVIEW_REQUIRED` or removed.

## 11. Practical significance

T-20 and C6 from `research/methods/thresholds.json`. A new flag or concept is a T2 user-visible change; `unsafe_defaults` must be 0 (HC-05). No band applies to the heuristic T-18 metrics.

## 12. Sensitivity analysis

Not weight-based. Reported arms: OS, task, and candidate. Simulated results are reported with the `[SIMULATED]` qualifier and the MEDIUM cap.

## 13. Expected negative results worth recording

- Simulated users read "cryptographically valid" as "trusted" absent a policy, supporting a verdict-words-require-policy design (DEC-CRY-090) — recorded as a qualitative failure mode, routed to human/external review, never decided on simulation alone.
- The status-quo `--require-content-signature` is satisfiable by any key (mechanical unsafe default), a DEC-CRY-082 finding usable mechanically.
- Keyless verify exit 0 without a machine-readable qualifier confuses scripts (DEC-INT-021), a mechanical finding.

## 14. Threats to validity

- Simulated sessions share a base model with the designers, so they are heuristic and correlated (T-18; objective §5 item 8). They never carry a human claim.
- The mechanical criteria may not capture real confusion; that gap is exactly why human claims are routed out.
- HUMAN_PARTICIPANTS rows stay `INSUFFICIENT_EVIDENCE`/`EXTERNAL_REVIEW_REQUIRED` for their human parts regardless of the mechanical results.

## 15. Held-out placeholder (Phase D)

The pre-committed held-out task subset serves R5 for the mechanical parts (§4.16). No held-out corpus content is used.

## 16. Cost estimate and agent effort

- Compute: negligible (CLI runs) plus simulated-session agent tokens (the one budget-sensitive cost here).
- Agent effort: **judgment** (census design, session harness) plus **scripted** execution; the simulated sessions themselves consume agent tokens, so they are batched and capped. Human claims are BLOCKED on human/external review.

<!-- BEGIN AUTO:common -->
<!-- END AUTO:common -->
