# EXP-CRYPTO-021: External cryptographic-review dossier assembly (EXPERT_REVIEW_REQUIRED)

<!-- BEGIN AUTO:meta -->
<!-- END AUTO:meta -->

## 1. Question

Assemble the complete dossier that SPEC §25.2 and `docs/crypto-review-v1.md` require for the independent external cryptographic and implementation review, so that every EXTERNAL_REVIEW_REQUIRED crypto decision has a self-contained review package: the frozen specification excerpts, the internal evidence produced under this method, the threat model with named adversaries, the formal-analysis targets, and the specific review questions per decision. The dossier is the artifact that lets a decision move from `EXTERNAL_REVIEW_REQUIRED` toward `DECIDED` once the review is received and dispositioned (decision-method.md §8.3).

## 2. Hypotheses and falsification

Not a hypothesis-testing experiment. Completeness criteria (each is a pass/fail check the dossier must satisfy):

- **P1.** Every EXTERNAL_REVIEW_REQUIRED crypto decision and every crypto row whose `external_review_requirement` is non-empty has a dossier section with: the question, the frozen spec excerpts (with `docs/` line citations), the candidate set with R1/R2 exclusions, the internal evidence references (experiment ids, normalized outputs), and specific review questions.
- **P2.** Every internal-evidence reference resolves to a committed normalized output or a review record from EXP-CRYPTO-001..020.
- **P3.** The threat model lists the SPEC §25.2-required adversaries (snapshot, persistent read-write, partial-plaintext) plus the added classes (storage-provider, remote observer, chosen-plaintext co-packer, recipient), with a traceability matrix (adversary × asset × mechanism × evidence).
- **P4.** The formal-analysis targets (segment/AD binding, unlock order, mutation resistance, commitment, envelope MAC, recipient wrap) are stated with their assumptions.
- **Falsification:** a missing section, an unresolved evidence reference, or a missing required adversary fails the completeness gate.

## 3. Decisions informed and evidence route

<!-- BEGIN AUTO:candidates -->
<!-- END AUTO:candidates -->

Evidence route: `EXPERT_REVIEW_REQUIRED`. The dossier is assembled internally; **the decisions become `DECIDED` only after independent external review is received and dispositioned** (decision-method.md §8.3; internal adversarial review is not independent expert review). This experiment is BLOCKED on the external reviewer, which the program cannot supply (PROGRESS known blockers; objective §5 items 5, 6, 8).

## 4. Candidates and arms

Not applicable. The dossier covers the union of EXTERNAL_REVIEW_REQUIRED crypto decisions and ER-noted rows:

- Status `EXTERNAL_REVIEW_REQUIRED`: DEC-CRY-002, DEC-CRY-011, DEC-CRY-016, DEC-CRY-035, DEC-CRY-036, DEC-CRY-038, DEC-CRY-039, DEC-CRY-049, DEC-CRY-053, DEC-CRY-055, DEC-CRY-059, DEC-CRY-070, DEC-CRY-080, DEC-CRY-084, DEC-CRY-088, DEC-MOD-016.
- ER-noted (evidence feeds review): DEC-CRY-023, DEC-CRY-026, DEC-CRY-037, DEC-CRY-048, DEC-CRY-054, DEC-CRY-062, DEC-CRY-106, DEC-INT-009, DEC-INT-010, DEC-INT-032, DEC-INT-033, DEC-ECO-022, DEC-ECO-072, DEC-CMP-001 (keyed-boundary leakage), DEC-LEG-089 (legacy decrypt paths).

Each section names the internal experiment that produced its evidence (attack reproduction EXP-CRYPTO-001/002/005; padding EXP-CRYPTO-006; public leakage EXP-CRYPTO-007; partial-read/segment binding EXP-CRYPTO-010; vectors EXP-CRYPTO-011; primitive throughput EXP-CRYPTO-012; timestamp EXP-CRYPTO-013; signature semantics EXP-CRYPTO-014; mutation/nonce EXP-CRYPTO-015; hygiene EXP-CRYPTO-018; primary-source review EXP-CRYPTO-020).

## 5. Corpus selectors

None. Inputs are committed research artifacts and frozen spec documents. No corpus item is used.

## 6. Environment and platform requirements

- Assembly runs on any host (documentary).
- **BLOCKED, missing access:** independent external cryptographic reviewers. The program has no external crypto reviewers (PROGRESS; SPEC §25.2 requires review independent of the container designers). Internal agents share a base model, so they cannot substitute (objective §5 item 8). The dossier is produced; the review is the missing input.

## 7. Commands and tooling

- `research/security/dossier/` : one section per decision, generated from a template that pulls the ledger row, the frozen spec excerpts and the internal-evidence references.
- `research/tools/crypto/build_dossier.py` : assembles sections, resolves every evidence reference against committed outputs, and fails if any is missing (the P2 gate).
- `research/security/dossier/threat-model-traceability.csv` : the adversary × asset × mechanism × evidence matrix (P3).
- `research/security/dossier/formal-targets.md` : the P4 targets and assumptions.

## 8. Seeds, warmup, repetitions and adaptive rule

Not applicable.

## 9. Measurement method and metrics

Completeness checks only:
- section coverage of the required decision set (fraction = 1 required);
- evidence-reference resolution (fraction = 1 required);
- required-adversary coverage (fraction = 1 required);
- `assurance_state` per property remains at most "internally reviewed" until the external review is received.

## 10. Normalization and statistical analysis

None. `build_dossier.py` emits a completeness report; a decision cannot advance past `EXTERNAL_REVIEW_REQUIRED` until its section is complete and the external review is dispositioned.

## 11. Practical significance

Not applicable. External review governs; a negative or partial review returns a decision to `INSUFFICIENT_EVIDENCE` (§8.3).

## 12. Sensitivity analysis

Not applicable.

## 13. Expected negative results worth recording

- An internal experiment failed to produce a required piece of evidence (for example an OD-16 attack the suite could not implement faithfully), which the dossier must state as a gap rather than hide.
- The internal adversarial review found an issue the formal targets must cover.

## 14. Threats to validity

- Same-base-model internal preparation may frame questions in a way that steers the reviewer; mitigated by stating raw evidence and explicit assumptions, and by an independent session reviewing the dossier's framing.
- The dossier cannot make a decision; it only prepares one.

## 15. Held-out placeholder (Phase D)

Not applicable. Where a decision has empirical parts (for example padding effectiveness), those parts follow their own experiments' held-out placeholders; the external review covers the crypto-soundness parts.

## 16. Cost estimate and agent effort

- Compute: negligible. Disk: small.
- Agent effort: **judgment** (dossier assembly and framing), independent-session review of framing. Status remains BLOCKED on the external reviewer.

<!-- BEGIN AUTO:common -->
<!-- END AUTO:common -->
