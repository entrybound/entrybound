# EXP-ECO-022: Adopter, gatekeeper, maintainer and consumer field studies

| Field | Value |
|---|---|
| Experiment ID | EXP-ECO-022 |
| Domain | ecosystem (program §32; informs §34 stable-v1 gates and §35 claims) |
| Status | **BLOCKED** |
| Blocked by | (1) no human participants: named teams, registry gatekeepers (PyPI/Warehouse, Maven Central, OCI validation maintainers), build-system maintainers (Bazel, Nix), archivists, model and dataset publishers and integrators are not available to the program; (2) contacting external parties means sending messages on the owner's behalf, which agents may not do; (3) the Maven Central staging upload and registry pilots need accounts and publishing approval |
| Unblocked by | owner-led outreach with written consent to record answers, owner-held accounts for staging registries, and owner approval of every external message and publication |
| Runner | structured interviews and pilots; quantitative pilot measurements reuse EXP-ECO-006, -007, -008, -011 scripts on participants' artifacts with their permission |
| Conventions | `research/experiments/_index/ecosystem-conventions.md` |

## 1. Question

Is there demonstrated demand and a feasible adoption path for Entrybound v1: which user group commits to migrating a real path (the three-team test), what gatekeepers and maintainers would consume (library, CLI, corpus, observed-outcome matrix, policy profiles), which headline and surface they respond to, what single-team pilots measure in defects found and time saved, and which workflows, targets, sidecars, key sources, bindings and language surfaces real adopters require?

## 2. Hypotheses and falsification

- **H1 (three-team test).** At least three named teams agree in writing to migrate a production path to Entrybound v1 (Research I ship condition, REQ-ECO-0100). *Falsified* if fewer than three agree after the pre-registered outreach list is exhausted.
- **H2 (gatekeeper consumption).** At least one registry gatekeeper states in writing that they would consume an externally governed observed-outcome matrix or validation library in CI and names the version pin (Research III reopen trigger, REQ-ECO-0107). *Falsified* if none does.
- **H3 (single-team value).** In at least two single-team pilots, the legacy audit finds at least one real defect in the team's existing artifacts that their current tooling did not report, or deterministic dual publishing removes at least one existing normalization step. *Falsified* otherwise.
- **H4 (surface preference).** A majority of interviewed gatekeepers and integrators prefer a language-neutral surface (CLI or data artifact) over a Rust library for v1. *Falsified* by a majority for the library.

## 3. Decisions informed

DEC-ECO-001, DEC-ECO-004 (Bazel and Nix maintainer interest), DEC-ECO-010 (consumer survey of output stability needs), DEC-ECO-018, DEC-ECO-019 (use of migration reports by consumers), DEC-ECO-022, DEC-ECO-030, DEC-ECO-033, DEC-ECO-034, DEC-ECO-035 (gatekeeper trial), DEC-ECO-039, DEC-ECO-040, DEC-ECO-043 (adopter trial of dual publishing), DEC-ECO-046, DEC-ECO-048, DEC-ECO-050, DEC-ECO-052, DEC-ECO-053 (message testing), DEC-ECO-056 (stakeholder governance interviews), DEC-ECO-058, DEC-ECO-059 (archivist interviews), DEC-ECO-062 (adopter study on sidecar publication), DEC-ECO-065 (maintainer interviews on source releases), DEC-ECO-075 (single-team pilots), DEC-ACC-021 (integrator survey), DEC-CRY-001 (demand survey of recipient key types), DEC-LEG-010 (workflow studies with release engineers), DEC-LEG-017 (consumer receipt discovery study), DEC-LEG-029 (consumer requirement survey for export targets), DEC-MOD-005 (reproducible-builds consumer study), DEC-MOD-017 (consultation with in-toto and reproducible-builds communities).

## 4. Candidates

The ledger candidate sets of the listed decisions (for example DEC-ECO-033 target groups; DEC-ECO-052 adoption surfaces; DEC-ECO-053 headlines; DEC-ECO-018 wedges), presented to participants as neutral descriptions written before outreach and reviewed for leading wording by a second session.

## 5. Participants and materials

- Pre-registered outreach list by group (names withheld from the repository; owner-held): registry gatekeepers (PyPI, Maven Central, crates.io, npm, OCI distribution), build-system maintainers (Bazel `rules_pkg`, Nix, Gradle/Maven archiver maintainers), release engineers of at least five open-source projects with reproducible-build interest, archivists (digital preservation practitioners), model and dataset publishers, Rust and non-Rust integrators.
- Interview guide per group with fixed question order; pilots use the participant's own artifacts with written permission and run EXP-ECO-006 (W01, W02, W08), EXP-ECO-007 and EXP-ECO-011 checkers locally; the Maven Central staging upload uses an owner-held staging account.
- Kill and success criteria for each study are the hypotheses above and are fixed before outreach.

## 6. Environment and platform requirements

Owner-run communication channels; participants' environments for pilots; staging registry accounts; no agent-initiated external contact.

## 7. Procedure

Owner sends pre-approved outreach; interviews recorded as notes with consent; notes coded against the candidate lists by two coders; pilots run the named scripts and return aggregate results only; results committed as anonymized coded tables; analysis by a session not involved in candidate design.

## 8. Seeds and replication

Interview order randomized with seed 2026091722 within groups; no replication beyond the participant count.

## 9. Metrics

Counts of written commitments per candidate; coded preference counts; pilot outcomes using the canonical metrics of the reused experiments (`migration_steps_count`, `export_acceptance_fraction`, `import_acceptance_fraction`, `error_actionability_fraction`) plus defects found and minutes saved as reported by participants (descriptive).

## 10-12. Aggregation, analysis, thresholds

Descriptive counts per group; no Appendix A.2 band applies to interview outcomes; decisions depending on demand remain owner decisions informed by this evidence, and any claim about users' preferences is routed under §8.3.

## 13. Sensitivity analysis

By group; excluding participants with prior involvement in Entrybound research; commitments with versus without a named version pin.

## 14. Held-out (Phase D placeholder)

Not applicable (no corpus); pilot measurements on participants' artifacts are fresh data by construction and are reported separately from corpus results.

## 15. Expected negative results worth recording

Fewer than three committing teams; gatekeepers preferring existing project-local validation; library demand concentrated in one language ecosystem; archivists requiring registrations that cannot exist before v1.

## 16. Threats to validity

Outreach selection bias; social desirability in interviews; commitments that do not translate into migrations; small pilot numbers.

## 17. Cost estimate

About 30 interviews (about 20 participant-hours), 3-5 pilots (about 40 machine-hours on participant artifacts), owner coordination; agent effort judgment for interview guides, coding and analysis.

## 18. Gates and exposure

Conventions §1 and §2. This protocol exists so that `remaining-unknowns` can cite the exact missing participants and access for the listed decisions.
