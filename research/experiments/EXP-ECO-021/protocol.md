# EXP-ECO-021: Human first-use usability study and independent expert heuristic evaluation of the CLI

| Field | Value |
|---|---|
| Experiment ID | EXP-ECO-021 |
| Domain | ecosystem (program §33) |
| Status | **BLOCKED** |
| Blocked by | (1) no human participants are available to this program (PROGRESS known limitations; `blocker_class` HUMAN_PARTICIPANTS); (2) no independent usability experts are available — internal program agents are not independent expert reviewers (§8.3); (3) recruiting participants, obtaining consent and paying incentives require an owner-run process and ethics/consent handling outside agent authority |
| Unblocked by | owner-recruited participants (at least 12 per tar/zip-literate and 12 per non-archive-expert cohort, see §5), a consent and data-handling procedure approved by the owner, and two external reviewers with published CLI usability expertise not involved in Entrybound |
| Runner | moderated or unmoderated remote sessions with the EXP-ECO-018 task checkers and the EXP-ECO-019 probe renderer and sandbox (reused unchanged) |
| Conventions | `research/experiments/_index/ecosystem-conventions.md` (§5) |

## 1. Question

Do human first-time users complete the pre-registered first-use tasks and correctly interpret verification, loss, restoration and signature outputs with each CLI candidate, how do they compare with tar, zip and 7-Zip for the same tasks, and do independent experts' heuristic evaluations identify the same problems? This is the evidence that can carry the human-comprehension, task-success and preference claims that §4.16 routes out of simulated evidence.

## 2. Hypotheses and falsification

Pre-registered as directional claims with analysis at the participant level (unlike simulated sessions, participants are independent units):

- **H1.** Task success on T01, T02, T03, T06 with the status quo is non-inferior to GNU tar/Info-ZIP at a margin of 10 percentage points. *Falsified* if the one-sided 0.95 bound of the paired difference is below −10 pp.
- **H2.** Correct interpretation of signature outputs (EXP-ECO-019 IP01-IP05) is below 80% for the status quo and improves by at least 15 pp with the best candidate variant. *Falsified* if status-quo correctness is ≥ 80% or no variant improves by 15 pp.
- **H3.** Participants predict lost metadata classes from the LOSSY report (IP08, IP09) correctly in at least 80% of cases with an itemized-loss candidate (DEC-ECO-036). *Falsified* below 80%.
- **H4.** Expert heuristic findings and simulated-session failure modes overlap in at least half of the expert-identified severe problems (DEC-ECO-083 validation of the simulated method). *Falsified* if overlap is below half.

The 10 pp, 15 pp and 80% figures are design parameters of this human study, set here before any human data; they are not bands of Appendix A.2 (T-18 has none), and any decision use requires the external review route of §8.3.

## 3. Decisions informed

Human-claim parts of: DEC-ECO-005, DEC-ECO-007, DEC-ECO-010, DEC-ECO-011, DEC-ECO-020 (recall and typing of executable names), DEC-ECO-023, DEC-ECO-036, DEC-ECO-055, DEC-ECO-057, DEC-ECO-064, DEC-ECO-066, DEC-ECO-083, DEC-ACC-035, DEC-ACC-039 (developer presets versus raw limits), DEC-CMP-036, DEC-CRY-009, DEC-CRY-040, DEC-CRY-057 (recovery and shared-password workflows), DEC-CRY-061, DEC-CRY-082, DEC-CRY-090, DEC-CRY-094, DEC-CRY-101, DEC-INT-038, DEC-LEG-006, DEC-LEG-013, DEC-LEG-014 (interpretation of re-rendered digests), DEC-LEG-036 (approval friction), DEC-MOD-015, DEC-MOD-033 (first-time integrators), DEC-MOD-043 (comprehension of identity-change reporting), DEC-PLT-007.

## 4. Candidates

Identical to EXP-ECO-019 arms A and B (status quo, runnable shims, owner-authored output variants, incumbent controls) and EXP-ECO-020 arm 6 for integrators; plus DEC-ECO-083 `expert-heuristic-evaluation` and `later-human-beta-study` as methods.

## 5. Participants and materials

- Cohort A: at least 12 developers or operators who use tar or zip weekly; cohort B: at least 12 technical users who rarely use archive tools; cohort C (integrators, for DEC-MOD-033): at least 8 Rust developers. Recruitment, screening, consent, incentive and data retention are owner-run; no personal data enters the repository (participant ids only).
- Within-subject design with counterbalanced candidate order (Latin square, seed 2026091721); each participant completes a subset of tasks and probes sized to 60 minutes.
- Materials: EXP-ECO-018 task statements and fixtures, EXP-ECO-019 probes and variants, the sealed held-out task set for the confirmatory session block.
- Experts: two external reviewers apply a pre-registered heuristic checklist (Nielsen's heuristics adapted to CLIs plus POSIX utility conventions and clig.dev guidance) independently to each candidate before seeing any session data.

## 6. Environment and platform requirements

Participants' own Linux or Windows machines through a remote sandbox (the EXP-ECO-019 sandbox exposed over a browser terminal) or moderated screen-sharing sessions; screen and command logs retained under the owner's data policy; macOS participants only if macOS builds exist (EXP-ECO-005).

## 7. Procedure

Pre-registration of this protocol's analysis before recruitment; pilot with 2 participants (excluded from analysis); main sessions; task success by the mechanical checkers; think-aloud notes coded with the EXP-ECO-019 codebook by two coders; expert reports collected before disclosure of session results; analysis by a session not involved in candidate design.

## 8. Seeds and replication

Order seed 2026091721; each participant is one replicate; no repeated measures of the same task-candidate pair within a participant.

## 9. Metrics

`task_success_rate`, `task_errors`, `task_steps`, `task_completion_time_s` (T-18 names, here measured on humans), interpretation correctness per probe, expert problem lists with severity, agreement with EXP-ECO-019 failure modes.

## 10-12. Aggregation, analysis, thresholds

Per participant paired differences between candidates; exact binomial or mixed-effects logistic models by cohort (specified in `record.md` before recruitment); multiplicity by Holm within each decision; decision use only through `EXTERNAL_REVIEW_REQUIRED` dispositions (§8.3) — no Appendix A.2 band applies to human task success.

## 13. Sensitivity analysis

By cohort; excluding participants with prior Entrybound exposure; excluding sessions with technical interruptions.

## 14. Held-out (Phase D placeholder)

The confirmatory block uses the sealed held-out tasks after the freeze (conventions §5).

## 15. Expected negative results worth recording

Humans misread signature and verification states that agents read correctly; tar-literate users transfer tar expectations (stdout defaults, flag bundling) and fail; experts flag the option load of `unpack`.

## 16. Threats to validity

Small cohorts; remote sandbox artificiality; participant self-selection; experts' familiarity with competing tools.

## 17. Cost estimate

About 40 participant-hours plus 16 expert-hours; owner coordination; about 10 h of analysis; agent effort judgment for analysis only.

## 18. Gates and exposure

Conventions §1, §2 and §5. This protocol exists so that `remaining-unknowns` can cite the exact missing participants and experts for the human-claim parts of the listed decisions.
