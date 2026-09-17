# EXP-CRYPTO-023: Internal FORMAL preparation — symbolic models of the key hierarchy, commitment, unlock order, bindings and mutations (dossier input)

<!-- BEGIN AUTO:meta -->
| Field | Value |
|---|---|
| Index status | **NEEDS_TOOLING** |
| Kind | decision |
| Domain | crypto (program §22.1, §22.4, §22.5) |
| Decisions informed | DEC-CRY-049, DEC-CRY-053, DEC-CRY-084, DEC-CRY-105, DEC-CRY-045, DEC-CRY-069, DEC-CRY-089, DEC-CRY-080, DEC-CRY-034 |
| Platforms | Linux/x86-64 (WSL); symbolic tools only |
| Requires timing | False |
| Estimates | 120 machine-hours; 2 GB disk |
| Tooling to build | apt proverif (pinned version recorded) and tamarin-prover release binary with maude (pinned SHA-256); research/security/formal/{key-hierarchy,unlock-order,bindings,mutations}.{pv,spthy} model files written from docs/crypto-wire-v1.md and SPEC section 9 by a session not involved in the production writer; research/tools/crypto/formal/run_models.py (runs every query under the pre-registered budget, records tool versions, verdicts and counterexample traces); research/tools/crypto/formal/trace_to_vector.py (turns each attack trace into a concrete byte-level test case for EXP-CRYPTO-015/016) |
| Environment prerequisites | wsl-proverif, wsl-tamarin |
| Blocked arms | Independent sign-off of the models by an external cryptographer is part of the EXP-CRYPTO-021 dossier review, which is BLOCKED (no external reviewers); the internal FORMAL route needs a reviewer session not involved in the models (available). |
| Specs | none (non-runner experiment) |
| Validation looks | owns no validation look |
| Gates | G-A and G-B unmet at this revision (generated `gate_state` column of `research/experiments/index.csv`); timing runs also need a PASS calibration record for the calibration identity (PA-04) |
| Conventions | `research/experiments/_index/crypto-conventions.md` (CR0-CR10); program amendments `research/experiments/_program/design-revision-round1.md` |
<!-- END AUTO:meta -->

Added by the Phase C design revision, round 1 (design review DR1-08 fix item 3). `decision-method.md` §8.3 requires
the external-review dossier to contain "the internal evidence under this method"; the crypto designs routed the
construction questions of the key hierarchy, commitment, unlock order and signature/binding composition only to the
BLOCKED dossier (EXP-CRYPTO-021) without an internal FORMAL preparation. This protocol is that preparation. It
decides nothing on its own: every selection it informs stays provisional until the external review (CR3).

## 1. Question

Under a committed Dolev-Yao style adversary model that includes the named adversaries of DEC-CRY-053 (snapshot,
persistent read-write, storage provider, malicious producer, removed recipient, remote range observer, chosen-plaintext
co-packer), do symbolic models of crypto-v1 as frozen in `docs/crypto-wire-v1.md` and SPEC §9 satisfy the secrecy,
authentication, binding and injectivity properties the frozen documents claim, and which candidate variants of the
unlock order (DEC-CRY-105), recipient-set integrity (DEC-CRY-045), recipient mutation semantics (DEC-CRY-069),
suite-id binding in signatures (DEC-CRY-089), signature composition (DEC-CRY-084), segment binding (DEC-CRY-080) and
suite introduction and downgrade (DEC-CRY-034) change any verdict?

## 2. Hypotheses and falsification

| ID | Decision | Hypothesis | Falsified by |
|---|---|---|---|
| H1 | DEC-CRY-049, DEC-CRY-053 | For the status-quo key hierarchy (AFK, HKDF labels, key commitment, envelope MAC, X-Wing and password wraps), the archive file key and every derived key stay secret against every modelled adversary that does not hold unlock material, and the key commitment binds each accepted AFK to exactly one envelope (no invisible-salamander class attack in the model). | A trace in which the adversary learns a derived key, or two distinct AFKs pass commitment for one envelope. |
| H2 | DEC-CRY-105 | Status-quo hard-fail after a commitment or envelope-MAC failure leaks no more than "some stanza opened"; `continue-then-report` and `evaluate-all-require-unique` allow an adversary holding one recipient key to make another recipient accept an attacker-chosen AFK in at least one mutation sequence. | Status quo shows such an oracle, or both alternatives are shown safe for every sequence in budget. |
| H3 | DEC-CRY-045, DEC-CRY-069 | Without an expected addressing signature, any file-key holder can add or remove recipients undetectably (documented status quo); `sender-authenticated` and `mandatory-signatures-for-multi-recipient` candidates make the recipient set an authenticated attribute. | The status quo model authenticates the set, or a candidate fails to. |
| H4 | DEC-CRY-084, DEC-CRY-089 | The one-signature/three-binding composition resists cross-context confusion and binding mixing when the suite id is bound by commitment and envelope MAC; a signature over a PCR binding alone admits a suite-substitution trace under a modelled weak suite. | A mixing or substitution trace against the status quo, or no trace against the PCR-only variant. |
| H5 | DEC-CRY-080 | SegmentEnd, monotonic counters and ArchiveFinal binding prevent truncation, reordering and cross-archive splicing of segments in the model. | A splice, reorder or truncation trace accepted as complete. |
| H6 | DEC-CRY-034 | A reader that accepts a suite list without a signed or committed suite id admits a downgrade trace; the status-quo single-suite rule admits none. | The opposite result. |

A symbolic proof is evidence about the model, not the implementation: "no attack found" is reported as "no attack in
the model within budget" (CR3). Every trace found is translated into a concrete byte-level test case and run against
the production reader in EXP-CRYPTO-015 or EXP-CRYPTO-016; a trace that does not reproduce is recorded as a modelling
artefact with its reason.

## 3. Decisions informed and evidence route

<!-- BEGIN AUTO:candidates -->
#### DEC-CRY-049 — What formal analysis and independent review of the key hierarchy, commitment, envelope MAC, recipient wrap, unlock order and AFK-sharing mutations must complete before production crypto, and in what form?

Ledger status `EXTERNAL_REVIEW_REQUIRED`, blocker class `EXTERNAL_REVIEW`. Other experiments informing it: EXP-CRYPTO-021, EXP-CRYPTO-011, EXP-CRYPTO-015.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-internal-adversarial-review` | Status quo: internal adversarial review AR-01..AR-34 only | status quo | no | — |
| `symbolic-model` | Symbolic model (Tamarin or ProVerif) of envelope, unwrap order, mutations and snapshot/storage adversaries | ledger alternative | no | — |
| `computational-proof` | Game-based proof or CryptoVerif model of the HKDF hierarchy and HMAC commitment | ledger alternative | no | — |
| `external-audit-without-model` | Third-party cryptographic audit of design and code without a mechanised model | ledger alternative | no | — |
| `combined-model-proof-and-audit` | Symbolic model plus computational argument plus external audit | ledger alternative | no | — |
| `defer-to-post-v1` | Ship v1 with documented residual risk and audit after release | defer / no change | no | — |

**Ledger exclusions:** {"candidate_id": "status-quo-internal-adversarial-review", "reason": "Internal review alone does not satisfy the routing of formal analysis to independent security review", "invariant_violated": "SPEC §25.2 L2559-2562"}; {"candidate_id": "defer-to-post-v1", "reason": "Production crypto requires an external cryptographic audit before release", "invariant_violated": "docs/crypto-review-v1.md L10-12"}.

**R0 checklist:** 1 status quo: `status-quo-internal-adversarial-review`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 6 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: `defer-to-post-v1`; 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `status-quo-internal-adversarial-review`, `symbolic-model`, `computational-proof`, `external-audit-without-model`, `combined-model-proof-and-audit`, `defer-to-post-v1`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-CRY-053 — Which named adversaries does v1 encryption defend against (snapshot, persistent read-write, partial plaintext, storage provider, malicious producer, recipient, remote range observer, chosen-plaintext co-packer), and is the documented set complete and traceable?

Ledger status `EXTERNAL_REVIEW_REQUIRED`, blocker class `EXTERNAL_REVIEW`. Other experiments informing it: EXP-CRYPTO-021.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-six-classes` | Status quo: six attacker classes in docs/crypto-threat-model-v1.md L40-110 | status quo | no | — |
| `add-remote-observer-and-co-packer` | Add remote range observer and chosen-plaintext co-packer classes | ledger alternative | no | — |
| `gocryptfs-eve-dragon-mallory` | Adopt gocryptfs Eve/Dragon/Mallory terminology mapped to existing classes | ledger alternative | no | — |
| `traceability-matrix` | Add adversary x asset x mechanism x evidence traceability matrix | ledger alternative | no | — |
| `stride-style-model` | Replace with STRIDE-style per-component model | ledger alternative | no | — |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: `status-quo-six-classes`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 5 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `status-quo-six-classes`, `add-remote-observer-and-co-packer`, `gocryptfs-eve-dragon-mallory`, `traceability-matrix`, `stride-style-model`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-CRY-084 — Does the one-signature/three-binding composition (T1 signature/v1 transcript, addressing binding with archive ID, sign-then-encrypt embedding, exposed detached records) resist surreptitious forwarding, cross-context confusion and binding-mixing attacks as SPEC §25.2 requires?

Ledger status `EXTERNAL_REVIEW_REQUIRED`, blocker class `EXTERNAL_REVIEW`. Other experiments informing it: EXP-CRYPTO-021, EXP-CRYPTO-014, EXP-CRYPTO-015.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-three-binding-t1` | Status quo: one Ed25519 signature over T1 transcript of content, optional physical and addressing bindings | status quo | no | — |
| `suite-and-version-in-content-binding` | New version adding suite and crypto version to the content binding | ledger alternative | no | — |
| `sign-encrypt-sign` | Sign/encrypt/sign layering | ledger alternative | no | — |
| `header-hash-binding` | Sign the envelope header hash including commitment directly | ledger alternative | no | — |
| `separate-signature-per-binding` | Separate signatures per binding (first-pass two-scope design) | ledger alternative | no | — |
| `cose-sign1-protected-headers` | COSE_Sign1 with protected headers carrying binding context | ledger alternative | no | — |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: `status-quo-three-binding-t1`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 6 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `status-quo-three-binding-t1`, `suite-and-version-in-content-binding`, `sign-encrypt-sign`, `header-hash-binding`, `separate-signature-per-binding`, `cose-sign1-protected-headers`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-CRY-105 — When a stanza opens but the candidate AFK fails commitment or envelope MAC, should unlock hard-fail (status quo), continue to remaining stanzas, or evaluate all stanzas before deciding?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-010, EXP-CRYPTO-015, EXP-CRYPTO-021.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-hard-fail` | Status quo: integrity failure at the first commitment or MAC mismatch | status quo | yes | EXP-CRYPTO-010 |
| `continue-then-report` | Continue to remaining stanzas; report integrity failure only if none succeed | ledger alternative | yes | EXP-CRYPTO-010 |
| `evaluate-all-require-unique` | Evaluate all matching stanzas and require one consistent AFK | ledger alternative | yes | EXP-CRYPTO-010 |
| `continue-with-tamper-warning` | Continue and surface a tamper warning if another stanza succeeds | ledger alternative | yes | EXP-CRYPTO-010 |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: `status-quo-hard-fail`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 4 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-CRY-045 — Is recipient-set integrity against other legitimate recipients a v1 goal: status quo (any file-key holder can edit the envelope; only an expected addressing signature detects it), mandatory signatures for multi-recipient archives, sender-authenticated KEM, per-recipient MACs, or a documented non-goal?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-015, EXP-CRYPTO-021.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-addressing-signature-only` | Status quo: envelope MAC keyed from AFK; addressing signature detects edits | status quo | no | — |
| `require-signature-multi-recipient` | Require a signature on multi-recipient archives | ledger alternative | no | — |
| `authenticated-kem` | Sender-authenticated KEM mode | ledger alternative | no | — |
| `per-recipient-mac` | Per-recipient MAC keys over the stanza set | ledger alternative | no | — |
| `documented-non-goal` | Declare insider integrity a non-goal in threat model and CLI output | defer / no change | no | — |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: `status-quo-addressing-signature-only`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 5 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: `documented-non-goal`; 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `status-quo-addressing-signature-only`, `require-signature-multi-recipient`, `authenticated-kem`, `per-recipient-mac`, `documented-non-goal`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-CRY-069 — Which recipient-mutation semantics bind the envelope in v1: AFK-preserving addition producing key-sharing sibling archives, fresh-epoch removal and password change with full re-encryption, rotation keeping every recipient, combined add and remove, and carry-over of directory labels, at what measured cost? Boundary-preserving rotation, beyond-RAM re-encryption and signature retention are decided in key-rotation-boundary-preservation, encryption-beyond-ram and signature-retention-after-recipient-mutation.

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-015, EXP-SCALE-010, EXP-CRYPTO-021.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo` | Status quo: add rewraps the same AFK; remove and password change rotate AFK and archive_id, re-chunk and re-encrypt; no keep-all rotation; labels come from caller | status quo | no | — |
| `add-also-rotates` | Addition also rotates AFK and archive_id, avoiding key-sharing sibling archives at full re-encryption cost | ledger alternative | no | — |
| `add-rotates-archive-id-only` | Addition keeps the AFK but derives a fresh archive_id or segment salts to separate sibling versions | ledger alternative | no | — |
| `rotate-keep-all-command` | Explicit rotation keeping every recipient | ledger alternative | no | — |
| `combined-transaction` | Atomic add-and-remove in one epoch | ledger alternative | no | — |
| `carry-authenticated-labels` | Rotation carries authenticated directory labels for retained recipients | ledger alternative | no | — |
| `remove-rewrap-only` | Removal only deletes the stanza and rewraps | ledger alternative | no | — |

**Ledger exclusions:** {"candidate_id": "remove-rewrap-only", "reason": "A removed recipient already holds the AFK, so removal must rotate the AFK and re-encrypt", "invariant_violated": "CRYPTO-030/031"}.

**R0 checklist:** 1 status quo: `status-quo`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 7 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `status-quo`, `add-also-rotates`, `add-rotates-archive-id-only`, `rotate-keep-all-command`, `combined-transaction`, `carry-authenticated-labels`, `remove-rewrap-only`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-CRY-089 — Must payload_suite_id and crypto version be signed in every signature over an encrypted archive: require addressing for encrypted signing, move suite into the content binding in a new version, argue that commitment and envelope MAC already bind it, or amend SPEC agility rule 7?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-014, EXP-CRYPTO-021.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-suite-in-optional-addressing` | Status quo: suite signed only in optional addressing binding | status quo | no | — |
| `require-addressing-for-encrypted` | Library refuses encrypted-archive signatures without ADDRESSING | ledger alternative | no | — |
| `suite-in-content-binding-new-version` | Add suite to content binding in a new version | ledger alternative | no | — |
| `rely-on-commitment` | Document that commitment and envelope MAC bind the suite | ledger alternative | no | — |
| `amend-rule-7` | Amend SPEC rule 7 to cover only mandatory bindings | SPEC design | no | — |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: `status-quo-suite-in-optional-addressing`; 2 SPEC design: `amend-rule-7`; 3 ledger alternatives: 5 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `status-quo-suite-in-optional-addressing`, `require-addressing-for-encrypted`, `suite-in-content-binding-new-version`, `rely-on-commitment`, `amend-rule-7`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-CRY-080 — Do segment ID binding, monotonic counters, mandatory SegmentEnd and terminal ArchiveFinal/footer binding defeat truncation, reordering and cross-archive splicing, as established by formal analysis rather than internal adversarial review?

Ledger status `EXTERNAL_REVIEW_REQUIRED`, blocker class `EXTERNAL_REVIEW`. Other experiments informing it: EXP-CRYPTO-010, EXP-CRYPTO-021, EXP-INT-016.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-segmentend-archivefinal` | Status quo: segment-bound keys and nonces, SegmentEnd, terminal ArchiveFinal and footer | status quo | no | EXP-INT-016 |
| `stream-construction` | Hoang-Reyhanitabar-Rogaway-Vizar STREAM online AEAD | ledger alternative | no | EXP-INT-016 |
| `final-flag-in-nonce` | age/Tink style final-chunk flag in nonce | ledger alternative | no | EXP-INT-016 |
| `merkle-over-segments` | Merkle tree over segment digests | ledger alternative | no | EXP-INT-016 |
| `per-record-counter-only` | Per-record counters without final markers | ledger alternative | no | EXP-INT-016 |

**Ledger exclusions:** {"candidate_id": "per-record-counter-only", "reason": "Truncation after a valid record would be accepted (AR-07)", "invariant_violated": "I21"}.

**R0 checklist:** 1 status quo: `status-quo-segmentend-archivefinal`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 5 ledger candidates listed above; 4 strongest incumbent: `final-flag-in-nonce`; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-CRY-034 — How is a future crypto suite (e.g. wider nonces, AEGIS, PQ changes) introduced, registered, deprecated and protected against downgrade, and who re-verifies external standards status on what cadence?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-020, EXP-EVAL-008, EXP-CRYPTO-021.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-new-suite-id-and-feature` | Status quo: new suite id plus required feature bit, no negotiation, process unwritten | status quo | no | — |
| `written-suite-registry-process` | Written registry process with review, vectors and deprecation schedule | ledger alternative | no | — |
| `explicit-downgrade-sentinel` | Add TLS-1.3-style downgrade signalling across suite versions | ledger alternative | no | — |
| `reader-policy-min-suite` | Reader policy option refusing older suites after deprecation | ledger alternative | no | — |
| `in-archive-negotiation` | Per-archive cipher negotiation or user cipher choice | ledger alternative | no | — |

**Ledger exclusions:** {"candidate_id": "in-archive-negotiation", "reason": "Negotiation and user cipher choice are frozen out", "invariant_violated": "SPEC §9.9 rule 1; docs/crypto-review-v1.md L82-83"}.

**R0 checklist:** 1 status quo: `status-quo-new-suite-id-and-feature`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 5 ledger candidates listed above; 4 strongest incumbent: `explicit-downgrade-sentinel`; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `status-quo-new-suite-id-and-feature`, `written-suite-registry-process`, `explicit-downgrade-sentinel`, `reader-policy-min-suite`, `in-archive-negotiation`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.
<!-- END AUTO:candidates -->

Evidence route: FORMAL (§1.2): derivation plus the committed counterexample-search protocol of §7, checked and signed
off by a reviewer session that did not write the models. EXTERNAL for every security-claim part (§8.3): the model
files, queries, tool versions, verdicts and traces are dossier sections of EXP-CRYPTO-021.

## 4. Candidates and arms

| Arm | Model | Variants (candidate ids from the ledger) |
|---|---|---|
| M1 key hierarchy and commitment | AFK generation, HKDF label tree, HMAC key commitment, envelope MAC, X-Wing and password wraps | status quo only; label-injectivity mutants (two labels merged) as positive controls |
| M2 unlock order | stanza iteration, commitment check, envelope MAC check, failure handling | `status-quo-hard-fail`, `continue-then-report`, `evaluate-all-require-unique`, `continue-with-tamper-warning` (DEC-CRY-105) |
| M3 recipient set | key add, key remove, change-password, epoch rotation | DEC-CRY-045 and DEC-CRY-069 ledger candidates named in EXP-CRYPTO-015 |
| M4 signatures and bindings | T1 signature transcript, addressing binding with archive id, content binding, physical binding, sign-then-encrypt embedding | DEC-CRY-084 status quo and PCR-only variant; DEC-CRY-089 ledger candidates |
| M5 segment framing | segment id binding, counters, SegmentEnd, ArchiveFinal, footer | status quo; `per-segment-digests-in-archivefinal` future-version model (labelled, not a v1 candidate) |
| M6 suite agility | suite id placement and reader acceptance | DEC-CRY-034 ledger candidates |

**Positive controls (required before any "no attack" verdict counts):** each model has at least one planted flaw
(a dropped MAC input, an unbound counter, a merged label, an unsigned suite id) for which the tool must find the
known attack within budget. A model whose planted flaw is not found is not evidence.

## 5. Corpus selectors

No corpus items. Inputs are `docs/crypto-wire-v1.md`, `docs/crypto-threat-model-v1.md`, SPEC §9 and §25.2 (cited by
line), and the EXP-CRYPTO-001 observer model for the adversary classes. No tuning, validation or held-out item is
used; no validation look is spent.

## 6. Environment and platform requirements

`wsl-ubuntu`, Linux x86-64, `timing: false`. ProVerif (apt, version recorded) and Tamarin prover (release binary
with Maude, SHA-256 recorded). Not installed at this revision (`_program/runnable-probe.json`: `wsl-proverif` and
`wsl-tamarin` are `no`), hence NEEDS_TOOLING. No privileged operation, no network after the pinned download.

## 7. Commands and counterexample-search protocol

```sh
wsl.exe -d Ubuntu -- bash -lc 'cd /mnt/d/Projects/entrybound/entrybound && \
  /root/eb-research/venv/bin/python research/tools/crypto/formal/run_models.py \
    --models research/security/formal --budget-file research/experiments/EXP-CRYPTO-023/budget.json \
    --out research/normalized/EXP-CRYPTO-023/formal-results.jsonl'
```

- **Budget (pre-registered, `budget.json` committed before any run):** per query, ProVerif 4 CPU-hours and 32 GiB;
  Tamarin 8 CPU-hours with the default heuristic, then one re-run with the `--heuristic=O` oracle if the first run
  times out. Mutation-sequence depth up to 5 operations (matching EXP-CRYPTO-015).
- **Outcome classes per query:** `PROVEN_IN_MODEL`, `ATTACK_TRACE`, `TIMEOUT_WITHIN_BUDGET`,
  `POSITIVE_CONTROL_MISSED` (invalidates the model's other verdicts).
- **Trace translation:** `trace_to_vector.py` emits a byte-level case for EXP-CRYPTO-015/016; reproduction status is
  recorded next to the trace.
- **Review:** a reviewer session not involved in the models checks each model against the cited document lines and
  signs `research/security/formal/review-<model>.md` before results are cited.

## 8. Seeds, warmup, repetitions and adaptive rule

Symbolic tools are deterministic for a given version and heuristic; each query runs once, and each `PROVEN_IN_MODEL`
or `ATTACK_TRACE` verdict is re-run once in a fresh process with the output hash compared (§4.13). No warmups, no
adaptive stopping on outcomes.

## 9. Measurement method and metrics

| Quantity | Kind | Role |
|---|---|---|
| verdict per (model, variant, query) | outcome class | FORMAL evidence (no CI) |
| positive-control detection | binary | validity gate per model |
| attack traces reproduced on production bytes | count | reported; a reproduced trace is an HC-14 finding routed through EXP-CRYPTO-015/016 |
| `assurance_state` per property | descriptive (M15.4) | stays at most "internally reviewed" |

## 10. Normalization and statistical analysis

None: formal outcomes are exact per model. Results are tabulated per decision and variant. A variant "safe in model"
is never compared by magnitude; only verdict classes are reported.

## 11. Practical significance

Not applicable (no banded metric). Selection effects are limited to R1/R2 eliminations for variants with a
reproduced attack trace; all other uses are dossier input.

## 12. Sensitivity analysis

Not applicable under the FORMAL route (§1.2; reason: no measured metric). Modelling sensitivity is reported instead:
each `PROVEN_IN_MODEL` verdict is re-checked with the adversary allowed one additional capability (key-holding of a
removed recipient; control of the storage provider), and changes are reported.

## 13. Expected negative results worth recording

- Every status-quo query is `PROVEN_IN_MODEL`: the model then adds no attack but narrows the dossier questions to
  computational assumptions the symbolic model abstracts away (for example AES-GCM-SIV nonce-misuse bounds).
- A trace against the status quo that does not reproduce on bytes, showing a modelling gap rather than a flaw.
- `TIMEOUT_WITHIN_BUDGET` for M3 at depth 5, leaving mutation-sequence safety to external review.

## 14. Threats to validity

- Symbolic models abstract primitives as perfect; computational attacks (nonce misuse, multi-key bounds, partitioning
  oracles on weak passwords) are outside the model.
- Model-to-document fidelity depends on the modeller and one internal reviewer sharing a base model with the other
  program sessions (limited independence; §8.3).
- Budget-bounded search: absence of an attack within budget is not a proof for unbounded sessions unless the tool
  reports a proof.

## 15. Held-out placeholder (Phase D)

Not applicable (FORMAL route, no corpus; reason recorded as §1.2 requires).

## 16. Cost estimate and agent effort

- Compute: about 120 CPU-hours for all queries and re-runs; disk about 2 GB. No timing windows.
- Agent effort: **judgment** for model writing and review (two separate sessions); execution **scripted**.

<!-- BEGIN AUTO:common -->
**Shared provisions (binding; `research/experiments/_index/crypto-conventions.md`).**

- **Author and L7 exposure (CR1):** designed by the Phase C-design crypto session, which read `research/corpus/coverage.md` and `research/PROGRESS.md` under the shared design instructions and is recorded as L7-exposed for all families (`research/experiments/_program/l7-exposures-design-phase.jsonl`). Its candidate operationalizations, exclusions and analysis plans are re-signed by an unexposed session before G-A (PA-01); decision analyses are executed by unexposed sessions only.
- **Gates (CR0):** runs before G-A/G-B are `NOT_DECISION_GRADE`; specs with non-empty `decision_ids` launch only through `research/tools/experiments/run_guarded.py` (PA-13).
- **Candidates (CR2):** the tables above are generated; R0 item 6 is open until the crypto-cluster critic pass (PA-12).
- **Security claims (CR3):** attack, leakage and tamper results are lower bounds; selections resting on a security claim, sufficiency of a mitigation or acceptance of leakage are provisional until the EXP-CRYPTO-021 external review is received.
- **Metrics (CR6):** component throughput uses `__component_<name>` strata; chunk-stage throughput is owned by EXP-CHUNK-008 T1 (PA-17); HC oracle counts are binary under their MVT ids pending the PA-02 metric-registry disposition.
- **Timing (CR5):** affinity and guard per §4.2-§4.4 (lint-checked), staged helpers (PA-16), calibration identity and instrument-class calibration (PA-04, PA-15).
- **Split discipline (CR7):** committed specs are tuning-only or binary HC screens; graded looks use look specs derived at look time with candidates restricted to W ∪ S, registered by the owner in `research/experiments/_program/look-plan.csv` (PA-03, PA-09).
- **Held-out (CR8):** generated only by EXP-EVAL-012 at Commit A (PA-20).
- **Power (PA-06):** a banded comparison enters its full tuning run only after `research/tools/experiments/mde_feasibility.py` projects an MDE of at most 1 band unit on a tuning proxy.

_Generated by `python research/tools/experiments/fill_crypto_auto_blocks.py` for EXP-CRYPTO-023; edit the inputs, not this block._
<!-- END AUTO:common -->
