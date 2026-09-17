# EXP-CRYPTO-020: Primary-source primitive and standards-status review protocol (FORMAL / literature)

<!-- BEGIN AUTO:meta -->
| Field | Value |
|---|---|
| Index status | **NEEDS_TOOLING** — Primary-source documents are owner-fetched (some sources 403 to automated fetch) |
| Kind | decision |
| Domain | crypto (program §22.1, §22.5) |
| Decisions informed | DEC-CRY-034, DEC-CRY-037, DEC-CRY-062, DEC-CRY-088, DEC-CRY-002, DEC-CRY-011, DEC-CRY-035, DEC-CRY-036, DEC-CRY-038, DEC-CRY-059, DEC-CRY-070, DEC-CRY-021, DEC-CRY-107, DEC-MOD-016 |
| Platforms | any host (documentary) |
| Requires timing | False |
| Estimates | 2 machine-hours; 1 GB disk |
| Tooling to build | research/security/review/* records; sources manifest; standards-status.csv; second-session review |
| Environment prerequisites | none beyond the harness and the ebr venv |
| Blocked arms | none |
| Specs | none (non-runner experiment) |
| Validation looks | owns no validation look |
| Gates | G-A and G-B unmet at this revision (generated `gate_state` column of `research/experiments/index.csv`); timing runs also need a PASS calibration record for the calibration identity (PA-04) |
| Conventions | `research/experiments/_index/crypto-conventions.md` (CR0-CR10); program amendments `research/experiments/_program/design-revision-round1.md` |
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
#### DEC-CRY-034 — How is a future crypto suite (e.g. wider nonces, AEGIS, PQ changes) introduced, registered, deprecated and protected against downgrade, and who re-verifies external standards status on what cadence?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-EVAL-008, EXP-CRYPTO-021, EXP-CRYPTO-023.

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

#### DEC-CRY-037 — Will Entrybound keep the X-Wing hybrid KEM indefinitely or plan a pure ML-KEM (or higher-level hybrid) suite, and what trigger governs the change?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-021.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-hybrid-indefinite` | Status quo: X-Wing hybrid indefinitely | status quo | no | — |
| `planned-pure-ml-kem-suite` | Plan a pure ML-KEM-768 suite | ledger alternative | no | — |
| `hybrid-until-cnsa-2035` | Hybrid until CNSA 2.0 transition completes | ledger alternative | no | — |
| `ml-kem-1024-hybrid-suite` | Add an ML-KEM-1024 hybrid suite | ledger alternative | no | — |
| `follow-final-x-wing-rfc` | Track the final X-Wing RFC with a new suite | ledger alternative | no | — |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: `status-quo-hybrid-indefinite`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 5 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `status-quo-hybrid-indefinite`, `planned-pure-ml-kem-suite`, `hybrid-until-cnsa-2035`, `ml-kem-1024-hybrid-suite`, `follow-final-x-wing-rfc`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-CRY-062 — When and with which scheme (ML-DSA-65, composite ML-DSA-65+Ed25519, SLH-DSA, or FN-DSA once FIPS 206 publishes) will Entrybound add post-quantum or hybrid archive signatures, and what event triggers it?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-021.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-ed25519-until-trigger` | Status quo: Ed25519 only until a defined trigger | status quo | no | — |
| `ml-dsa-65` | Add ML-DSA-65 as algorithm 2 | ledger alternative | no | — |
| `composite-ml-dsa-ed25519` | Add composite ML-DSA-65+Ed25519 | ledger alternative | no | — |
| `dual-independent-signatures` | Allow Ed25519 and ML-DSA records side by side | ledger alternative | no | — |
| `slh-dsa-long-term` | SLH-DSA for long-term release signing | ledger alternative | no | — |
| `wait-for-fn-dsa` | Wait for FIPS 206 FN-DSA | ledger alternative | no | — |
| `stateful-hash-based-lms-xmss` | Stateful hash-based signatures (LMS/HSS or XMSS, NIST SP 800-208) for long-lived release-signing keys, as CNSA 2.0 lists for software and firmware signing; requires signer state management, and the reported CNSA 2.0 FAQ v2.1 removal of HSS/XMSS^MT is unverified | ledger alternative | no | — |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: `status-quo-ed25519-until-trigger`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 7 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `status-quo-ed25519-until-trigger`, `ml-dsa-65`, `composite-ml-dsa-ed25519`, `dual-independent-signatures`, `slh-dsa-long-term`, `wait-for-fn-dsa`, `stateful-hash-based-lms-xmss`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-CRY-088 — Which signature scheme(s) instantiate SignatureMethod for v1 archives: the frozen pure Ed25519, or an alternative or additional scheme, as approved by the dedicated security review SPEC §25.2 requires?

Ledger status `EXTERNAL_REVIEW_REQUIRED`, blocker class `EXTERNAL_REVIEW`. Other experiments informing it: EXP-CRYPTO-021.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-pure-ed25519` | Status quo: pure Ed25519 (RFC 8032) over the T1 transcript, ed25519-dalek =3.0.0 | status quo | no | — |
| `ed25519ph` | Ed25519ph prehash variant (RFC 8032 §5.1) with context string | ledger alternative | no | — |
| `ed448` | Ed448 (RFC 8032) for a higher classical security level | ledger alternative | no | — |
| `ecdsa-p256` | ECDSA P-256 as in Apple AEA profiles 0/2/4 | ledger alternative | no | — |
| `rsa-pss` | RSA-PSS (RFC 8017) | ledger alternative | no | — |
| `ml-dsa-65` | ML-DSA-65 (FIPS 204) as sole scheme | ledger alternative | no | — |
| `composite-ml-dsa-65-ed25519` | Composite ML-DSA-65+Ed25519 as mandated by RFC 9980 for OpenPGP PQC | ledger alternative | no | — |
| `slh-dsa` | SLH-DSA (FIPS 205) hash-based signatures | ledger alternative | no | — |
| `dual-independent-records` | Ed25519 plus a separate PQ signature record per signer | ledger alternative | no | — |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: `status-quo-pure-ed25519`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 9 ledger candidates listed above; 4 strongest incumbent: `status-quo-pure-ed25519`, `ed25519ph`, `ed448`, `rsa-pss`, `composite-ml-dsa-65-ed25519`; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `status-quo-pure-ed25519`, `ed25519ph`, `ed448`, `ecdsa-p256`, `rsa-pss`, `ml-dsa-65`, `composite-ml-dsa-65-ed25519`, `slh-dsa`, `dual-independent-records`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-CRY-002 — Is Argon2id v19 the v1 password KDF, and are the frozen creation parameters (256 MiB, t=3, p=4, random unlabelled 16-byte salt) the right defaults, or should defaults, creator tunability or salt domain separation change?

Ledger status `EXTERNAL_REVIEW_REQUIRED`, blocker class `EXTERNAL_REVIEW`. Other experiments informing it: EXP-CRYPTO-004, EXP-CRYPTO-017, EXP-CRYPTO-021.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-256mib-t3-p4` | Status quo: Argon2id m=256 MiB, t=3, p=4 fixed at creation | status quo | no | EXP-CRYPTO-017 |
| `rfc9106-second-64mib` | RFC 9106 second recommendation m=64 MiB, t=3, p=4 | ledger alternative | no | EXP-CRYPTO-017 |
| `rfc9106-first-2gib` | RFC 9106 first recommendation m=2 GiB, t=1, p=4 | ledger alternative | no | EXP-CRYPTO-017 |
| `owasp-19mib` | OWASP m=19 MiB, t=2, p=1 | ledger alternative | no | EXP-CRYPTO-017 |
| `calibrated-per-host` | Calibrate to a target unlock latency on the creating host within wire bounds | ledger alternative | no | EXP-CRYPTO-017 |
| `named-cost-profiles` | Named cost profiles selectable at creation | ledger alternative | no | EXP-CRYPTO-017 |
| `scrypt` | scrypt as age uses (N=2^18, r=8, p=1) | ledger alternative | no | EXP-CRYPTO-017 |
| `balloon-hashing` | Balloon hashing | ledger alternative | no | EXP-CRYPTO-017 |
| `pbkdf2-fips` | PBKDF2-HMAC-SHA256 with 600,000 iterations for FIPS environments | ledger alternative | no | EXP-CRYPTO-017 |
| `labelled-salt-or-ad` | Status quo parameters plus format-label domain separation in salt or Argon2 associated data | status quo | no | EXP-CRYPTO-017 |

**Ledger exclusions:** {"candidate_id": "rfc9106-first-2gib", "reason": "2 GiB exceeds the frozen 1 GiB crypto v1 reader memory bound", "invariant_violated": "docs/crypto-suite-v1.md L510-520 reader wire bounds"}.

**R0 checklist:** 1 status quo: `status-quo-256mib-t3-p4`, `labelled-salt-or-ad`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 10 ledger candidates listed above; 4 strongest incumbent: `rfc9106-second-64mib`, `rfc9106-first-2gib`, `scrypt`, `pbkdf2-fips`; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-CRY-011 — Is SHA-256 confirmed for suite-1 crypto transcripts, encrypted object IDs and public digests, and must it stay coupled to the identity digest chosen under model decision digest-algorithm-v1?

Ledger status `EXTERNAL_REVIEW_REQUIRED`, blocker class `EXTERNAL_REVIEW`. Other experiments informing it: EXP-CRYPTO-012, EXP-CRYPTO-021.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-sha-256-coupled` | Status quo: SHA-256 for crypto transcripts and identities | status quo | no | — |
| `sha-256-crypto-decoupled` | Keep SHA-256 in suite 1 regardless of identity-digest changes (suite-versioned) | ledger alternative | no | — |
| `sha-512-256` | SHA-512/256 for transcripts | ledger alternative | no | — |
| `blake3` | BLAKE3 for transcripts | ledger alternative | no | — |
| `sha3-256` | SHA3-256 for transcripts | ledger alternative | no | — |
| `follow-identity-decision` | Adopt whatever digest-algorithm-v1 selects, changing suite id if needed | ledger alternative | no | — |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: `status-quo-sha-256-coupled`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 6 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `status-quo-sha-256-coupled`, `sha-256-crypto-decoupled`, `sha-512-256`, `blake3`, `sha3-256`, `follow-identity-decision`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-CRY-035 — Does HKDF-SHA-256 with T1-labelled salts and info remain the crypto v1 KDF (root hierarchy, segment keys, recipient wrap keys) after Task 22.1 primary-standard review and independent security review?

Ledger status `EXTERNAL_REVIEW_REQUIRED`, blocker class `EXTERNAL_REVIEW`. Other experiments informing it: EXP-CRYPTO-021.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-hkdf-sha256-t1-labels` | Status quo: HKDF-SHA-256 (RFC 5869) extract/expand with T1-encoded labels and archive_id-derived salts | status quo | no | — |
| `hkdf-sha384-or-sha512` | HKDF over SHA-384/SHA-512 (FLOE uses HKDF-Expand-SHA-384) for a larger margin | ledger alternative | no | — |
| `kmac256-sp800-185` | KMAC256-based derivation per NIST SP 800-108r1/SP 800-185 | ledger alternative | no | — |
| `blake3-derive-key` | BLAKE3 derive_key mode with context strings (borg 2 direction) | ledger alternative | no | — |
| `sp800-108-hmac-counter` | SP 800-56C extraction plus SP 800-108 counter-mode HMAC-SHA-256 expansion | ledger alternative | no | — |
| `hpke-labeled-key-schedule` | HPKE-style LabeledExtract/LabeledExpand schedule (RFC 9180) with suite-id prefixed labels | ledger alternative | no | — |
| `single-expand-key-nonce-commitment` | One HKDF-Expand per file key yielding key, nonce and commitment (age payload, C2SP chunked-encryption) | ledger alternative | no | — |
| `negotiated-kdf-choice` | Per-archive negotiable KDF identifier | ledger alternative | no | — |

**Ledger exclusions:** {"candidate_id": "negotiated-kdf-choice", "reason": "A negotiated KDF matrix contradicts one non-negotiated suite per format version", "invariant_violated": "SPEC §9.2 one payload suite per format version"}.

**R0 checklist:** 1 status quo: `status-quo-hkdf-sha256-t1-labels`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 8 ledger candidates listed above; 4 strongest incumbent: `status-quo-hkdf-sha256-t1-labels`, `blake3-derive-key`, `hpke-labeled-key-schedule`, `single-expand-key-nonce-commitment`; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `status-quo-hkdf-sha256-t1-labels`, `hkdf-sha384-or-sha512`, `kmac256-sp800-185`, `blake3-derive-key`, `sp800-108-hmac-counter`, `hpke-labeled-key-schedule`, `single-expand-key-nonce-commitment`, `negotiated-kdf-choice`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-CRY-036 — Is the HMAC-SHA-256 key commitment over the public crypto context, with a separately keyed HMAC envelope MAC, the right v1 commitment construction, and which commitment notion (CMT-1, context commitment, CMT-4) may Entrybound claim? Identifier-binding coverage beyond the commitment is decided in key-commitment-and-identifier-binding-coverage; this row carries the Task 22.1 primitive and notion review.

Ledger status `EXTERNAL_REVIEW_REQUIRED`, blocker class `EXTERNAL_REVIEW`. Other experiments informing it: EXP-CRYPTO-021.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-hmac-context-commitment` | Status quo: commitment = HMAC-SHA-256(HKDF commitment key, T1 key-commitment/v1 context); envelope MAC under a separate derived key | status quo | no | — |
| `hkdf-expand-commitment` | Commitment emitted directly by HKDF-Expand beside keys (C2SP chunked-encryption, FLOE) | ledger alternative | no | — |
| `committing-aead-transform` | Generic committing transform (CTX, UtC/HtE) over AES-GCM-SIV instead of a separate commitment | ledger alternative | no | — |
| `commitment-covers-policy-and-features` | Extend the commitment transcript to feature bits and protection policy, not only the envelope MAC | ledger alternative | no | — |
| `raw-sha256-commitment` | Commitment = SHA-256(AFK \|\| context), rejected by internal review as ad hoc | ledger alternative | no | — |
| `aead-tag-only` | Treat AES-GCM-SIV tag success as commitment | ledger alternative | no | — |
| `truncated-commitment` | 16-byte truncated HMAC commitment | ledger alternative | no | — |
| `aegis-native` | Future suite on AEGIS relying on its commitment claims | ledger alternative | no | — |

**Ledger exclusions:** {"candidate_id": "aead-tag-only", "reason": "AES-GCM-SIV is not key-committing and the repo freeze forbids assuming it", "invariant_violated": "docs/crypto-suite-v1.md L35-37 (RFC 9771)"}; {"candidate_id": "truncated-commitment", "reason": "Commitment and MAC tags are frozen at full 32 bytes", "invariant_violated": "docs/crypto-review-v1.md L89-91"}.

**R0 checklist:** 1 status quo: `status-quo-hmac-context-commitment`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 8 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `status-quo-hmac-context-commitment`, `hkdf-expand-commitment`, `committing-aead-transform`, `commitment-covers-policy-and-features`, `raw-sha256-commitment`, `aead-tag-only`, `truncated-commitment`, `aegis-native`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-CRY-038 — Which hybrid post-quantum KEM instantiates v1 public-key recipients: X-Wing draft 10 as frozen, the HPKE-PQ MLKEM768-X25519 hybrid used by age, a final X-Wing RFC, or another hybrid?

Ledger status `EXTERNAL_REVIEW_REQUIRED`, blocker class `EXTERNAL_REVIEW`. Other experiments informing it: EXP-CRYPTO-021.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-xwing-draft10` | Status quo: X-Wing draft-connolly-cfrg-xwing-kem-10 (ML-KEM-768 + X25519 + SHA3-256) with pinned wire name | status quo | no | — |
| `xwing-final-rfc` | Final X-Wing RFC under a new stanza type once published | ledger alternative | no | — |
| `hpke-pq-mlkem768-x25519` | HPKE with MLKEM768-X25519 from draft-ietf-hpke-pq (age mlkem768x25519 stanza) | ledger alternative | no | — |
| `higher-level-hybrid` | Higher security-level hybrid (P-384+ML-KEM-1024 or X448+ML-KEM-1024) | ledger alternative | no | — |
| `tls-x25519mlkem768` | TLS X25519MLKEM768 concatenation construction | ledger alternative | no | — |
| `dual-hybrid-types` | Ship both X-Wing and HPKE-PQ hybrid stanza types for interoperability | ledger alternative | no | — |
| `mlkem768-only` | ML-KEM-768 alone | ledger alternative | no | — |
| `x25519-only` | X25519 alone | ledger alternative | no | — |
| `custom-hkdf-combiner` | Entrybound-defined HKDF concatenation combiner | ledger alternative | no | — |

**Ledger exclusions:** {"candidate_id": "mlkem768-only", "reason": "Provides no classical hedge while a hybrid KEM is mandatory", "invariant_violated": "SPEC §9.10 hybrid PQ KEM mandatory"}; {"candidate_id": "x25519-only", "reason": "Not post-quantum while a hybrid PQ KEM is mandatory", "invariant_violated": "SPEC §9.10 hybrid PQ KEM mandatory"}; {"candidate_id": "custom-hkdf-combiner", "reason": "Custom combiners are forbidden", "invariant_violated": "docs/crypto-suite-v1.md L457-458"}.

**R0 checklist:** 1 status quo: `status-quo-xwing-draft10`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 9 ledger candidates listed above; 4 strongest incumbent: `hpke-pq-mlkem768-x25519`, `tls-x25519mlkem768`; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `status-quo-xwing-draft10`, `xwing-final-rfc`, `hpke-pq-mlkem768-x25519`, `higher-level-hybrid`, `tls-x25519mlkem768`, `dual-hybrid-types`, `mlkem768-only`, `x25519-only`, `custom-hkdf-combiner`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-CRY-059 — Does independent review confirm AES-256-GCM-SIV as the suite-1 payload, control and wrap AEAD, or must suite 1 change before v1 freeze?

Ledger status `EXTERNAL_REVIEW_REQUIRED`, blocker class `EXTERNAL_REVIEW`. Other experiments informing it: EXP-CRYPTO-012, EXP-CRYPTO-021.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-aes-256-gcm-siv` | Status quo: AES-256-GCM-SIV (RFC 8452), 12-byte structured nonces, aes-gcm-siv =0.12.1 | status quo | no | — |
| `aes-256-gcm` | AES-256-GCM (SP 800-38D) with the same structured nonces | ledger alternative | no | — |
| `chacha20-poly1305` | ChaCha20-Poly1305 (RFC 8439) | ledger alternative | no | — |
| `xchacha20-poly1305` | XChaCha20-Poly1305 (no RFC) | ledger alternative | no | — |
| `xaes-256-gcm` | XAES-256-GCM (C2SP) with 192-bit nonces | ledger alternative | no | — |
| `aegis-256` | AEGIS-256/256X (RFC pending) | ledger alternative | no | — |
| `aes-siv` | AES-SIV (RFC 5297) with explicit nonce | ledger alternative | no | — |
| `deoxys-ii-256` | Deoxys-II-256-128 (CAESAR final portfolio, defense-in-depth use case): nonce-misuse-resistant tweakable-block-cipher AEAD and the direct nonce-misuse-resistant alternative to GCM-SIV | ledger alternative | no | — |
| `ascon-aead128` | Ascon-AEAD128 (SP 800-232) | ledger alternative | no | — |
| `committing-aead-construction` | Status quo AEAD plus construction-level key commitment (commitment decisions owned by crypto-keys-recipients) | status quo | no | — |
| `dndk-gcm` | DNDK-GCM (IETF CFRG draft): AES-GCM with double-nonce key derivation and optional built-in key commitment; named with AES-XGCM and XAES-256-GCM in NIST's SP 800-38D Rev. 1 first pre-draft call (2025-01-06) | ledger alternative | no | — |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: `status-quo-aes-256-gcm-siv`, `committing-aead-construction`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 11 ledger candidates listed above; 4 strongest incumbent: `status-quo-aes-256-gcm-siv`, `chacha20-poly1305`, `aes-siv`; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `status-quo-aes-256-gcm-siv`, `aes-256-gcm`, `chacha20-poly1305`, `xchacha20-poly1305`, `xaes-256-gcm`, `aegis-256`, `aes-siv`, `deoxys-ii-256`, `ascon-aead128`, `committing-aead-construction`, `dndk-gcm`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-CRY-070 — Is omitting the X-Wing recipient public key from method context and wrap AD safe against key-substitution, re-encapsulation and multi-recipient confusion, or must a future wrap version bind a public-key digest?

Ledger status `EXTERNAL_REVIEW_REQUIRED`, blocker class `EXTERNAL_REVIEW`. Other experiments informing it: EXP-CRYPTO-021.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-rely-on-xwing` | Status quo: rely on X-Wing draft-10 shared-secret binding | status quo | no | — |
| `bind-public-key-digest-new-version` | Bind SHA-256 of public key in a new wrap version | ledger alternative | no | — |
| `bind-fingerprint-in-wrap-ad` | Bind recipient fingerprint in wrap AD (new version) | ledger alternative | no | — |
| `hpke-style-kem-context` | HPKE-style key schedule including pkR | ledger alternative | no | — |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: `status-quo-rely-on-xwing`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 4 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `status-quo-rely-on-xwing`, `bind-public-key-digest-new-version`, `bind-fingerprint-in-wrap-ad`, `hpke-style-kem-context`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-CRY-021 — Which Ed25519 verification equation and encoding rules are normative (ed25519-dalek verify_strict plus weak-key rejection, RFC 8032 cofactorless canonical, cofactored ZIP-215, or a vector-defined profile), and how are cross-implementation verification differentials prevented?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CONF-012, EXP-CRYPTO-011.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-dalek-verify-strict` | Status quo: verify_strict plus explicit weak-key rejection | status quo | no | EXP-CRYPTO-011 |
| `rfc8032-cofactorless-canonical` | RFC 8032 §5.1.7 cofactorless equation with canonical-encoding checks | ledger alternative | no | EXP-CRYPTO-011 |
| `zip215-cofactored` | ZIP-215 cofactored verification accepting noncanonical A/R encodings | ledger alternative | no | EXP-CRYPTO-011 |
| `libsodium-semantics` | libsodium crypto_sign_verify_detached acceptance rules | ledger alternative | no | EXP-CRYPTO-011 |
| `vector-defined-profile` | Normative accept/reject table defined by speccheck and Wycheproof vectors | ledger alternative | no | EXP-CRYPTO-011 |

**Ledger exclusions:** {"candidate_id": "zip215-cofactored", "reason": "Accepts noncanonical point encodings and small-order public keys that the frozen suite requires rejecting", "invariant_violated": "docs/crypto-suite-v1.md L651-654 strict verification freeze"}.

**R0 checklist:** 1 status quo: `status-quo-dalek-verify-strict`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 5 ledger candidates listed above; 4 strongest incumbent: `rfc8032-cofactorless-canonical`, `zip215-cofactored`, `libsodium-semantics`, `vector-defined-profile`; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: `vector-defined-profile`.

#### DEC-CRY-107 — If X-Wing is finalised with bytes differing from draft 10, how do v1 archives migrate: new stanza type with indefinite draft-10 decoding, dual wrapping, a deprecation window, re-encryption tooling, or delaying v1 until an RFC?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-EVAL-008.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-no-plan` | Status quo: pinned draft-10 type with no migration policy | status quo | no | — |
| `new-type-indefinite-read` | Add a final-RFC stanza type, stop creating draft-10, keep draft-10 readable indefinitely | ledger alternative | no | — |
| `dual-wrap-transition` | During a transition write both draft-10 and RFC stanzas per recipient | ledger alternative | no | — |
| `read-window-deprecation` | Draft-10 readable for a declared support window, then only through a migration tool | ledger alternative | no | — |
| `rekey-migration-tool` | Re-encryption command converting draft-10 stanzas to RFC stanzas | ledger alternative | no | — |
| `delay-v1-until-rfc` | Delay v1 until X-Wing or HPKE-PQ is an RFC | ledger alternative | no | — |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: `status-quo-no-plan`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 6 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `status-quo-no-plan`, `new-type-indefinite-read`, `dual-wrap-transition`, `read-window-deprecation`, `rekey-migration-tool`, `delay-v1-until-rfc`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-MOD-016 — Which single digest algorithm does Entrybound format version 1 use for chunk, content, structured, section, PCI and identity-descriptor digests?

Ledger status `EXTERNAL_REVIEW_REQUIRED`, blocker class `EXTERNAL_REVIEW`. Other experiments informing it: EXP-CRYPTO-004, EXP-CRYPTO-012, EXP-CRYPTO-021, EXP-EVAL-008.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `sha-256-status-quo` | SHA-256 (FIPS 180-4) via RustCrypto sha2 =0.11.0, as implemented for every digest and bound as algorithm id 1 | ledger alternative | no | — |
| `sha-512-256` | SHA-512/256 (FIPS 180-4): faster on 64-bit CPUs, same standard family, length-extension resistant | ledger alternative | no | — |
| `blake2b-256` | BLAKE2b-256 (RFC 7693): fast in software on 64-bit CPUs without SHA extensions; the Borg incumbent's chunk-ID hash; no tree mode in the RFC profile | ledger alternative | no | — |
| `blake3` | BLAKE3 (C2SP BLAKE3 v1.0.0): tree-native, SIMD/parallel, not NIST/IETF standardised | ledger alternative | no | — |
| `sha3-256` | SHA3-256 (FIPS 202) | ledger alternative | no | — |
| `kangarootwelve-kt128` | KT128/KT256 (RFC 9861, IRTF informational): tree-capable Keccak variant | ledger alternative | no | — |
| `ascon-hash256` | Ascon-Hash256/Ascon-CXOF128 (NIST SP 800-232): lightweight, customization strings for domain separation | ledger alternative | no | — |
| `sha-256-with-hardware-accel-only-fallback` | SHA-256 retained but mandate SHA-NI/ARMv8 crypto-extension paths and parallel per-chunk hashing | ledger alternative | no | — |
| `dual-digest-transition` | SHA-256 for v1 plus a registered second algorithm id reserved for a future format version (no in-version negotiation) | ledger alternative | no | — |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: no ledger id marked status quo; the critic pass confirms which candidate is the behaviour at `9e44608`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 9 ledger candidates listed above; 4 strongest incumbent: `blake2b-256`, `kangarootwelve-kt128`; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `sha-256-status-quo`, `sha-512-256`, `blake2b-256`, `blake3`, `sha3-256`, `kangarootwelve-kt128`, `ascon-hash256`, `sha-256-with-hardware-accel-only-fallback`, `dual-digest-transition`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.
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
- `assurance_state` (descriptive, M15.4) per property: vector-verified < internally reviewed < externally reviewed. This protocol can raise an item to "internally reviewed" only; "externally reviewed" needs the external review of the EXP-CRYPTO-021 dossier.
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

_Generated by `python research/tools/experiments/fill_crypto_auto_blocks.py` for EXP-CRYPTO-020; edit the inputs, not this block._
<!-- END AUTO:common -->
