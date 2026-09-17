# EXP-CRYPTO-020: Primary-source primitive and standards-status review protocol (FORMAL / literature)

<!-- BEGIN AUTO:meta -->
<!-- END AUTO:meta -->

## 1. Question

For each frozen or candidate primitive and construction — payload/wrap/control AEAD, HKDF key schedule, HMAC key-commitment, X-Wing/ML-KEM recipient KEM, Ed25519 signature, digest — does a primary-source review confirm the choice, its parameters, its standards status and its known cryptanalysis, and does it record every deviation from the cited standard with a locator? This is the review protocol that produces the standards-status and deviation record the external cryptographic audit and the EXTERNAL_REVIEW_REQUIRED decisions depend on. It also fixes the re-verification cadence for standards status (DEC-CRY-034) and the migration/agility triggers (DEC-CRY-037, DEC-CRY-062, DEC-CRY-107).

## 2. Hypotheses and falsification

This is a review protocol, not a comparison. Its deliverable is a per-primitive record. The falsifiable claims it can surface:

- **F1.** A frozen parameter deviates from its cited standard without a recorded justification (for example a nonce-length, label or salt-domain deviation). Such a finding routes the affected decision to external review with the deviation flagged.
- **F2.** A cited standard has changed status since the freeze (draft superseded, RFC published with different bytes, an advisory issued). This fires a reopen trigger (§11) and a migration decision (DEC-CRY-107).
- **F3.** A candidate the ledger lists is not standards-track or lacks independent implementations, which downgrades it in the dossier.

## 3. Decisions informed and evidence route

<!-- BEGIN AUTO:candidates -->
<!-- END AUTO:candidates -->

Evidence route: `EXTERNALLY_SOURCED` from primary standards and peer-reviewed papers (§9.3), re-verified locally by vectors where testable (EXP-CRYPTO-011). This is the internal preparation for the EXTERNAL_REVIEW_REQUIRED decisions; it does not by itself decide any primitive selection (SPEC §25.2: primitive selection is routed to independent review). FORMAL/literature decision type: no corpus, no held-out, no sensitivity.

## 4. Scope: primitives, constructions and the decisions they gate

| Review item | Primary sources (locators recorded in the record) | Decisions |
|---|---|---|
| Payload/wrap/control AEAD (AES-256-GCM-SIV) | RFC 8452; usage-bound analysis; SP 800-38D Rev.1; AEGIS/Ascon/Deoxys status | DEC-CRY-059 |
| HKDF key schedule | RFC 5869; SP 800-56C/108; RFC 9180 key schedule; label injectivity | DEC-CRY-035 |
| HMAC key commitment | SPEC §9.4; Albertini et al. (USENIX 2022); NIST IR 8552; CFRG committing-AEAD status | DEC-CRY-036, DEC-CRY-048 |
| Recipient KEM (X-Wing draft-10 / ML-KEM-768) | draft-connolly-cfrg-xwing-kem-10; FIPS 203; RFC 7748; draft-ietf-hpke-pq; X-Wing security analysis | DEC-CRY-038, DEC-CRY-070, DEC-CRY-106, DEC-CRY-107, DEC-CRY-037 |
| Ed25519 verification profile | RFC 8032; Wycheproof; speccheck; ZIP-215 | DEC-CRY-021, DEC-CRY-088 |
| Digest | FIPS 180-4; FIPS 202; C2SP BLAKE3; RFC 9861; SP 800-232 | DEC-CRY-011, DEC-MOD-016 |
| Password KDF | RFC 9106; OWASP; SP 800-63B; scrypt; balloon | DEC-CRY-002 |
| Suite agility and downgrade | SPEC §9.9; TLS 1.3, age, Kopia, OpenPGP/LibrePGP case studies | DEC-CRY-034 |
| PQ signatures | FIPS 204/205; RFC 9980; CNSA 2.0; NIST IR 8547 | DEC-CRY-062 |

## 5. Corpus selectors

None. Inputs are primary-source documents fetched by the program owner and committed by hash under `research/security/sources/` (the ledger's `EXTERNALLY_SOURCED` rule requires re-opening the primary source, not an agent's recollection, §9.3). No tuning/validation/held-out corpus item is used.

## 6. Environment and platform requirements

Any host; this is documentary. Local vector re-verification (EXP-CRYPTO-011) provides the "re-verify testable facts by experiment" requirement (§9.1 `EXTERNALLY_SOURCED`). Network is used once by the owner to fetch sources; documents are then offline and hash-pinned. Some sources (IACR ePrint) return 403 to automated fetches (`docs/crypto-review-v1.md` note), so the owner supplies them.

## 7. Commands and tooling

- `research/security/review/` : one Markdown record per review item, each with the standard identity, version/date, locator, the frozen value, the deviation (if any) with justification, the standards status as of the review date, known cryptanalysis, independent-implementation availability, and the re-verification cadence.
- `research/security/sources/manifest.json` : every source with URL, fetch date and SHA-256.
- `research/security/review/standards-status.csv` : machine-readable status and next-review date per item (feeds the DEC-CRY-034 cadence and reopen triggers).
- A cross-check script that every `EXTERNALLY_SOURCED` locator in the crypto ledger rows resolves to a source in the manifest.

## 8. Seeds, warmup, repetitions and adaptive rule

Not applicable (documentary). Each record is reviewed by a second session not involved in writing it (independence, §10.2 / §8.3 note that internal review is not external expert review; this is internal preparation).

## 9. Measurement method and metrics

No measured metrics. Deliverables:
- `assurance_state` (descriptive, M15.4) per property: vector-verified < internally reviewed < externally reviewed. This protocol can raise an item to "internally reviewed" only; "externally reviewed" needs EXP-CRYPTO-022's external review.
- `declared_security_level_bits` (descriptive, M15.5) per recipient policy.
- a deviation count per primitive.

## 10. Normalization and statistical analysis

None. The records are the output. `standards-status.csv` is the machine-readable index.

## 11. Practical significance

Not applicable. These are inputs to external-review decisions and to reopen triggers (a standards-status change moves a primary metric or an assurance state and fires §11 item 4/7).

## 12. Sensitivity analysis

Not applicable (FORMAL/literature; reason recorded).

## 13. Expected negative results worth recording

- A frozen label or salt-domain choice deviates from RFC 9180's LabeledExpand convention without a recorded reason (a DEC-CRY-035 finding).
- X-Wing draft-10 differs from a later RFC in bytes, firing the migration decision (DEC-CRY-107).
- A candidate primitive (for example a non-standard AEAD) lacks independent implementations, downgrading it in the dossier.

## 14. Threats to validity

- Internal review shares a base model with the designers (objective §5 item 8); this protocol explicitly does not substitute for external expert review.
- Source provenance: mitigated by hash-pinning and owner-fetched documents.
- Standards status is a moving target; the cadence and reopen triggers manage drift.

## 15. Held-out placeholder (Phase D)

Not applicable. The standards-status records are re-checked at the freeze commit and on each cadence date.

## 16. Cost estimate and agent effort

- Compute: negligible. Disk: under 1 GB (source documents).
- Agent effort: **judgment** (primary-source review), with a second session for independence. No execution.

<!-- BEGIN AUTO:common -->
<!-- END AUTO:common -->
