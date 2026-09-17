# EXP-CRYPTO-021: External cryptographic-review dossier assembly (EXPERT_REVIEW_REQUIRED)

<!-- BEGIN AUTO:meta -->
| Field | Value |
|---|---|
| Index status | **BLOCKED** — Independent external cryptographic reviewers unavailable (SPEC §25.2 requires reviewers independent of the container designers; internal agents share a base model) |
| Kind | decision |
| Domain | crypto (program §22.1, §22.2, §22.3, §22.4, §22.5) |
| Decisions informed | DEC-CRY-002, DEC-CRY-011, DEC-CRY-016, DEC-CRY-035, DEC-CRY-036, DEC-CRY-038, DEC-CRY-039, DEC-CRY-049, DEC-CRY-053, DEC-CRY-055, DEC-CRY-059, DEC-CRY-070, DEC-CRY-080, DEC-CRY-084, DEC-CRY-088, DEC-MOD-016, DEC-CRY-023, DEC-CRY-026, DEC-CRY-037, DEC-CRY-048, DEC-CRY-054, DEC-CRY-062, DEC-CRY-106, DEC-INT-009, DEC-INT-010, DEC-INT-032, DEC-INT-033, DEC-ECO-022, DEC-ECO-072, DEC-CMP-001, DEC-LEG-089, DEC-CRY-025, DEC-CRY-028, DEC-CRY-030, DEC-CRY-034, DEC-CRY-045, DEC-CRY-050, DEC-CRY-060, DEC-CRY-064, DEC-CRY-069, DEC-CRY-072, DEC-CRY-077, DEC-CRY-089, DEC-CRY-105 |
| Platforms | any host (assembly); external reviewer (BLOCKED) |
| Requires timing | False |
| Estimates | 2 machine-hours; 1 GB disk |
| Tooling to build | research/security/dossier/*; build_dossier.py; threat-model-traceability.csv; formal-targets.md |
| Environment prerequisites | none beyond the harness and the ebr venv |
| Blocked arms | none |
| Specs | none (non-runner experiment) |
| Validation looks | owns no validation look |
| Gates | G-A and G-B unmet at this revision (generated `gate_state` column of `research/experiments/index.csv`); timing runs also need a PASS calibration record for the calibration identity (PA-04) |
| Conventions | `research/experiments/_index/crypto-conventions.md` (CR0-CR10); program amendments `research/experiments/_program/design-revision-round1.md` |
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
#### DEC-CRY-002 — Is Argon2id v19 the v1 password KDF, and are the frozen creation parameters (256 MiB, t=3, p=4, random unlabelled 16-byte salt) the right defaults, or should defaults, creator tunability or salt domain separation change?

Ledger status `EXTERNAL_REVIEW_REQUIRED`, blocker class `EXTERNAL_REVIEW`. Other experiments informing it: EXP-CRYPTO-004, EXP-CRYPTO-017, EXP-CRYPTO-020.

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

Ledger status `EXTERNAL_REVIEW_REQUIRED`, blocker class `EXTERNAL_REVIEW`. Other experiments informing it: EXP-CRYPTO-012, EXP-CRYPTO-020.

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

#### DEC-CRY-016 — Who performs the independent cryptographic and implementation review of crypto v1 (AD and splice binding, END/ArchiveFinal truncation, partial-read guarantees, PHTE instantiation, T1/EBPO/EBCS grammars), with what scope, and does v1 encryption ship only after it?

Ledger status `EXTERNAL_REVIEW_REQUIRED`, blocker class `EXTERNAL_REVIEW`. Other experiments informing it: EXP-CONF-009, EXP-CONF-012, EXP-CONF-013, EXP-CRYPTO-010.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-internal-review-only` | Status quo: internal AR-01..AR-34 pass; crypto labelled experimental | status quo | no | — |
| `external-audit-before-v1-stable` | Commission external audit of design and code before declaring encryption stable in v1 | ledger alternative | no | — |
| `ship-experimental-until-audit` | Ship v1 with encryption explicitly experimental until audit completes | ledger alternative | no | — |
| `formal-model-plus-audit` | Mechanized model (e.g. Tamarin/ProVerif/EasyCrypt) of segment/AD binding plus human audit | ledger alternative | no | — |
| `staged-review` | Design review now, implementation audit and fuzzing before release | ledger alternative | no | — |
| `declare-stable-without-review` | Declare crypto v1 production-stable without external audit | ledger alternative | no | — |

**Ledger exclusions:** {"candidate_id": "declare-stable-without-review", "reason": "Production requires external cryptographic audit and independent review", "invariant_violated": "docs/crypto-review-v1.md L7-12; SPEC §25.2 L2559"}.

**R0 checklist:** 1 status quo: `status-quo-internal-review-only`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 6 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `status-quo-internal-review-only`, `external-audit-before-v1-stable`, `ship-experimental-until-audit`, `formal-model-plus-audit`, `staged-review`, `declare-stable-without-review`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-CRY-035 — Does HKDF-SHA-256 with T1-labelled salts and info remain the crypto v1 KDF (root hierarchy, segment keys, recipient wrap keys) after Task 22.1 primary-standard review and independent security review?

Ledger status `EXTERNAL_REVIEW_REQUIRED`, blocker class `EXTERNAL_REVIEW`. Other experiments informing it: EXP-CRYPTO-020.

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

Ledger status `EXTERNAL_REVIEW_REQUIRED`, blocker class `EXTERNAL_REVIEW`. Other experiments informing it: EXP-CRYPTO-020.

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

Ledger status `EXTERNAL_REVIEW_REQUIRED`, blocker class `EXTERNAL_REVIEW`. Other experiments informing it: EXP-CRYPTO-020.

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

#### DEC-CRY-039 — What residual chunk-size and boundary leakage is acceptable for the default encrypted profile, and how must I24 and related claims (quantised not hidden, provably secure keyed-prf, defense in depth) be worded in docs, help and inspect?

Ledger status `EXTERNAL_REVIEW_REQUIRED`, blocker class `EXTERNAL_REVIEW`. Other experiments informing it: EXP-CRYPTO-002, EXP-CRYPTO-006, EXP-CRYPTO-007.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-objective-and-qualitative-disclosure` | Status quo: I24 as objective; qualitative residual-leakage text; keyed-prf described as provably secure | status quo | no | — |
| `measured-bound-per-mode` | Publish measured leakage metrics per padding x boundary mode and word claims to those numbers | ledger alternative | no | — |
| `strong-claim-only-for-phte-plus-maximum` | Allow a strong boundary-privacy claim only for PHTE plus MAXIMUM padding; everything else labelled quantised | ledger alternative | no | — |
| `restate-i24-as-documented-leakage` | Restate I24 as documented, bounded leakage rather than no leakage | ledger alternative | no | — |
| `disable-dedup-under-encryption` | Eliminate within-archive dedup structure under encryption to strengthen I24 | ledger alternative | no | — |
| `external-review-sets-wording` | Defer all claim wording to the external security review outcome | defer / no change | no | — |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: `status-quo-objective-and-qualitative-disclosure`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 6 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: `external-review-sets-wording`; 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `status-quo-objective-and-qualitative-disclosure`, `measured-bound-per-mode`, `strong-claim-only-for-phte-plus-maximum`, `restate-i24-as-documented-leakage`, `disable-dedup-under-encryption`, `external-review-sets-wording`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-CRY-049 — What formal analysis and independent review of the key hierarchy, commitment, envelope MAC, recipient wrap, unlock order and AFK-sharing mutations must complete before production crypto, and in what form?

Ledger status `EXTERNAL_REVIEW_REQUIRED`, blocker class `EXTERNAL_REVIEW`. Other experiments informing it: EXP-CRYPTO-011, EXP-CRYPTO-015, EXP-CRYPTO-023.

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

Ledger status `EXTERNAL_REVIEW_REQUIRED`, blocker class `EXTERNAL_REVIEW`. Other experiments informing it: EXP-CRYPTO-023.

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

#### DEC-CRY-055 — Which padding schedule is the encrypted default, which modes remain user-selectable (NONE, MAXIMUM) with what warnings, and what overhead bound is declared per profile?

Ledger status `EXTERNAL_REVIEW_REQUIRED`, blocker class `EXTERNAL_REVIEW`. Other experiments informing it: EXP-CRYPTO-006.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-quarter-octave-bucketed` | Status quo: quarter-octave buckets (CONTROL 256 B-1 MiB, PAYLOAD 4 KiB-64 MiB) default; NONE and MAXIMUM selectable | status quo | no | EXP-CRYPTO-006 |
| `power-of-two-buckets` | Power-of-two buckets: coarser, up to ~100% overhead | ledger alternative | no | EXP-CRYPTO-006 |
| `padme` | Padme (PURBs) padding: O(log log) leakage with ~12% max overhead | ledger alternative | no | EXP-CRYPTO-006 |
| `randomized-padding` | Random padding within a range (Borg obfuscate style) | ledger alternative | no | EXP-CRYPTO-006 |
| `profile-specific-schedules` | Schedule chosen per profile or archive size class | ledger alternative | no | EXP-CRYPTO-006 |
| `lower-minimum-payload-bucket` | Smaller PAYLOAD minimum bucket (e.g. 256 B-1 KiB) to cut small-file overhead | ledger alternative | no | EXP-CRYPTO-006 |
| `maximum-default` | MAXIMUM as default | ledger alternative | no | EXP-CRYPTO-006 |
| `none-default` | No padding by default | ledger alternative | no | EXP-CRYPTO-006 |

**Ledger exclusions:** {"candidate_id": "none-default", "reason": "SPEC requires encrypted archives to pad by default and repo freezes BUCKETED as creation default", "invariant_violated": "SPEC §7.6 L844; docs/crypto-suite-v1.md L537-543"}.

**R0 checklist:** 1 status quo: `status-quo-quarter-octave-bucketed`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 8 ledger candidates listed above; 4 strongest incumbent: `randomized-padding`; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-CRY-059 — Does independent review confirm AES-256-GCM-SIV as the suite-1 payload, control and wrap AEAD, or must suite 1 change before v1 freeze?

Ledger status `EXTERNAL_REVIEW_REQUIRED`, blocker class `EXTERNAL_REVIEW`. Other experiments informing it: EXP-CRYPTO-012, EXP-CRYPTO-020.

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

Ledger status `EXTERNAL_REVIEW_REQUIRED`, blocker class `EXTERNAL_REVIEW`. Other experiments informing it: EXP-CRYPTO-020.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-rely-on-xwing` | Status quo: rely on X-Wing draft-10 shared-secret binding | status quo | no | — |
| `bind-public-key-digest-new-version` | Bind SHA-256 of public key in a new wrap version | ledger alternative | no | — |
| `bind-fingerprint-in-wrap-ad` | Bind recipient fingerprint in wrap AD (new version) | ledger alternative | no | — |
| `hpke-style-kem-context` | HPKE-style key schedule including pkR | ledger alternative | no | — |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: `status-quo-rely-on-xwing`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 4 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `status-quo-rely-on-xwing`, `bind-public-key-digest-new-version`, `bind-fingerprint-in-wrap-ad`, `hpke-style-kem-context`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-CRY-080 — Do segment ID binding, monotonic counters, mandatory SegmentEnd and terminal ArchiveFinal/footer binding defeat truncation, reordering and cross-archive splicing, as established by formal analysis rather than internal adversarial review?

Ledger status `EXTERNAL_REVIEW_REQUIRED`, blocker class `EXTERNAL_REVIEW`. Other experiments informing it: EXP-CRYPTO-010, EXP-INT-016, EXP-CRYPTO-023.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-segmentend-archivefinal` | Status quo: segment-bound keys and nonces, SegmentEnd, terminal ArchiveFinal and footer | status quo | no | EXP-INT-016 |
| `stream-construction` | Hoang-Reyhanitabar-Rogaway-Vizar STREAM online AEAD | ledger alternative | no | EXP-INT-016 |
| `final-flag-in-nonce` | age/Tink style final-chunk flag in nonce | ledger alternative | no | EXP-INT-016 |
| `merkle-over-segments` | Merkle tree over segment digests | ledger alternative | no | EXP-INT-016 |
| `per-record-counter-only` | Per-record counters without final markers | ledger alternative | no | EXP-INT-016 |

**Ledger exclusions:** {"candidate_id": "per-record-counter-only", "reason": "Truncation after a valid record would be accepted (AR-07)", "invariant_violated": "I21"}.

**R0 checklist:** 1 status quo: `status-quo-segmentend-archivefinal`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 5 ledger candidates listed above; 4 strongest incumbent: `final-flag-in-nonce`; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-CRY-084 — Does the one-signature/three-binding composition (T1 signature/v1 transcript, addressing binding with archive ID, sign-then-encrypt embedding, exposed detached records) resist surreptitious forwarding, cross-context confusion and binding-mixing attacks as SPEC §25.2 requires?

Ledger status `EXTERNAL_REVIEW_REQUIRED`, blocker class `EXTERNAL_REVIEW`. Other experiments informing it: EXP-CRYPTO-014, EXP-CRYPTO-015, EXP-CRYPTO-023.

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

#### DEC-CRY-088 — Which signature scheme(s) instantiate SignatureMethod for v1 archives: the frozen pure Ed25519, or an alternative or additional scheme, as approved by the dedicated security review SPEC §25.2 requires?

Ledger status `EXTERNAL_REVIEW_REQUIRED`, blocker class `EXTERNAL_REVIEW`. Other experiments informing it: EXP-CRYPTO-020.

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

#### DEC-MOD-016 — Which single digest algorithm does Entrybound format version 1 use for chunk, content, structured, section, PCI and identity-descriptor digests?

Ledger status `EXTERNAL_REVIEW_REQUIRED`, blocker class `EXTERNAL_REVIEW`. Other experiments informing it: EXP-CRYPTO-004, EXP-CRYPTO-012, EXP-CRYPTO-020, EXP-EVAL-008.

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

#### DEC-CRY-023 — Which chunk-boundary construction is the default for encrypted archives (and per profile or threat setting), given secret Gear tables are known-breakable and PHTE/AES costs more?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CHUNK-006, EXP-CHUNK-008, EXP-CHUNK-012, EXP-CHUNK-013, EXP-CRYPTO-002, EXP-CRYPTO-003, EXP-CRYPTO-004.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-secret-gear-default-phte-opt-in` | Status quo: SECRET_GEAR_TABLE default (defense in depth), PHTE_AES128 via --chunk-boundary keyed-prf | status quo | no | EXP-CHUNK-006, EXP-CRYPTO-002 |
| `phte-default-everywhere` | PHTE_AES128 default for all encrypted archives; secret Gear retained only for reading/compatibility | ledger alternative | no | EXP-CHUNK-006, EXP-CRYPTO-002 |
| `phte-default-for-selected-profiles` | PHTE default for remote/storage-provider oriented or Dense/Extreme profiles; secret Gear elsewhere | ledger alternative | no | EXP-CHUNK-006, EXP-CRYPTO-002 |
| `widened-keyed-rolling-hash` | Hardened secret table (wider state, e.g. 256-bit buzhash as recommended to Borg) with mandatory compression; new boundary mode id | ledger alternative | no | EXP-CHUNK-006, EXP-CRYPTO-002 |
| `fixed-size-chunks-under-encryption` | Fixed-size chunking for encrypted archives (no content-defined boundaries), losing shift-resistant dedup | ledger alternative | no | EXP-CHUNK-006, EXP-CRYPTO-002 |
| `keyed-cdc-with-coarse-rechunking` | Keyed CDC followed by packing chunks into fixed-size padded containers so boundaries are not observable | ledger alternative | no | EXP-CHUNK-006, EXP-CRYPTO-002 |
| `public-unkeyed-cdc` | Public unkeyed gear-norm-v1 boundaries under encryption | ledger alternative | no | EXP-CHUNK-006, EXP-CRYPTO-002 |

**Ledger exclusions:** {"candidate_id": "public-unkeyed-cdc", "reason": "Encrypted boundary modes are keyed by frozen registry; unkeyed boundaries directly expose plaintext-derived structure", "invariant_violated": "I24; docs/crypto-wire-v1.md L56-57"}.

**R0 checklist:** 1 status quo: `status-quo-secret-gear-default-phte-opt-in`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 7 ledger candidates listed above; 4 strongest incumbent: `widened-keyed-rolling-hash`; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-CRY-026 — Which metadata-privacy claims are accepted for encrypted INDEXED archives (metadata-private, entry count hidden, per-entry sizes hidden, DATA records reveal only class, no plaintext digests exposed), and must any be downgraded or backed by stronger manifest padding?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-006, EXP-CRYPTO-007.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-claims` | Status quo claims in SPEC §9.8, help text and docs | status quo | no | EXP-CRYPTO-007 |
| `downgrade-to-quantised` | Downgrade entry count and per-entry sizes to quantised in SPEC table and help | SPEC design | no | EXP-CRYPTO-007 |
| `coarser-control-padding` | Coarser manifest/Index padding to hide entry count more strongly | ledger alternative | no | EXP-CRYPTO-006, EXP-CRYPTO-007 |
| `publish-leakage-inventory` | Publish a normative public-field leakage inventory with measured inference bounds | ledger alternative | no | EXP-CRYPTO-007 |
| `keyed-public-digests` | Replace unkeyed public ciphertext digests with keyed MACs where observers gain linkage | ledger alternative | no | EXP-CRYPTO-007 |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: `status-quo-claims`; 2 SPEC design: `downgrade-to-quantised`; 3 ledger alternatives: 5 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: `publish-leakage-inventory`.

#### DEC-CRY-037 — Will Entrybound keep the X-Wing hybrid KEM indefinitely or plan a pure ML-KEM (or higher-level hybrid) suite, and what trigger governs the change?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-020.

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

#### DEC-CRY-048 — Is every algorithm and mode identifier and the file-key commitment bound before use so identifier rewriting and non-committing KEM/AEAD attacks fail, and does the frozen HMAC commitment over the public crypto context suffice versus other committing constructions?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-011.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-hmac-commitment-public-context` | Status quo: HMAC-SHA256 commitment over public context plus envelope MAC, wrap AD and record AD | status quo | no | — |
| `hkdf-derived-commitment` | AWS Encryption SDK style HKDF-derived commitment value | ledger alternative | no | — |
| `ctx-committing-transform` | CTX-style committing transform over the AEAD tag | ledger alternative | no | — |
| `committing-aead` | Replace payload AEAD with a committing AEAD construction | ledger alternative | no | — |
| `no-explicit-commitment` | Rely on AES-GCM-SIV and KEM without explicit commitment | ledger alternative | no | — |

**Ledger exclusions:** {"candidate_id": "no-explicit-commitment", "reason": "AES-GCM-SIV and HPKE/KEM wrapping are not committing; the frozen suite requires explicit commitment", "invariant_violated": "docs/crypto-suite-v1.md L116-131 commitment freeze"}.

**R0 checklist:** 1 status quo: `status-quo-hmac-commitment-public-context`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 5 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `status-quo-hmac-commitment-public-context`, `hkdf-derived-commitment`, `ctx-committing-transform`, `committing-aead`, `no-explicit-commitment`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-CRY-054 — Do all mutation operations (recipient add/remove, signature embedding, epoch rotation, interrupted writes) guarantee fresh keys and nonces when the input archive is attacker-controlled, and which mutation strategy is adopted?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CONF-010, EXP-CRYPTO-015.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-reuse-payload-regenerate-control` | Status quo: reuse PAYLOAD segments byte-for-byte with pre-registered salts; regenerate CONTROL/terminal segments with fresh salts | status quo | no | EXP-CRYPTO-015 |
| `full-reencryption-every-mutation` | Re-encrypt everything under a fresh AFK on every mutation | ledger alternative | no | EXP-CRYPTO-015 |
| `epoch-rotation-on-untrusted-input` | Rotate encryption epoch whenever the input archive is not locally produced | ledger alternative | no | EXP-CRYPTO-015 |
| `session-key-per-invocation` | Borg-2-style per-invocation session keys for new segments | ledger alternative | no | EXP-CRYPTO-015 |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: `status-quo-reuse-payload-regenerate-control`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 4 ledger candidates listed above; 4 strongest incumbent: `session-key-per-invocation`; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-CRY-062 — When and with which scheme (ML-DSA-65, composite ML-DSA-65+Ed25519, SLH-DSA, or FN-DSA once FIPS 206 publishes) will Entrybound add post-quantum or hybrid archive signatures, and what event triggers it?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-020.

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

#### DEC-CRY-106 — What exact evidence gate must X-Wing and ML-KEM pass before production release (official KATs with provenance, one or two independent implementations, dependency review, FIPS 203 errata assessment, malformed-key and implicit-rejection tests), and is it met?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CONF-012, EXP-CRYPTO-011, EXP-ECO-001.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-draft10-kats-only` | Status quo: three draft-10 KATs, no interop or dependency review record | status quo | no | EXP-CRYPTO-011 |
| `kats-plus-one-independent` | Draft-10 KATs plus cross-check with one independent implementation (docs/crypto-review-v1.md L157-158) | ledger alternative | no | EXP-CONF-012, EXP-CRYPTO-011 |
| `kats-plus-two-independent-and-review` | KATs plus libcrux and draft C reference cross-checks plus dependency review (AR-25, test plan item 3) | ledger alternative | no | EXP-CONF-012, EXP-CRYPTO-011 |
| `plus-acvp-negative-and-errata` | Above plus ACVP ML-KEM-768 vectors, malformed-key and implicit-rejection tests and FIPS 203 errata assessment | ledger alternative | no | EXP-CRYPTO-011 |
| `replace-crate-with-audited-impl` | Replace x-wing 0.1.0 with an audited implementation reproducing draft-10 bytes | ledger alternative | no | EXP-CRYPTO-011, EXP-ECO-001 |

**Ledger exclusions:** {"candidate_id": "status-quo-draft10-kats-only", "reason": "Crate KATs alone do not satisfy the frozen gate requiring independent interoperability", "invariant_violated": "docs/crypto-suite-v1.md L469-471"}.

**R0 checklist:** 1 status quo: `status-quo-draft10-kats-only`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 5 ledger candidates listed above; 4 strongest incumbent: `plus-acvp-negative-and-errata`; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-INT-009 — Will encrypted Sidecar or Incremental archives be supported after v1, under which crypto version and key-domain model (per-archive AFK, shared domain across chains), and with what leakage from external plaintext digests?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-007.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `never` | Encrypted archives remain Complete-only permanently | defer / no change | no | — |
| `new-crypto-version-per-archive-afk` | Post-v1 crypto version with per-archive AFK and encrypted External digests | defer / no change | no | — |
| `shared-key-domain-chain` | Key domain spanning a base chain enabling cross-archive dedup | ledger alternative | no | — |
| `encrypt-references-only` | Encrypt External reference records; bases independently encrypted | ledger alternative | no | — |
| `crypto-v1-incremental` | Allow non-Complete roles within crypto-v1 | ledger alternative | no | — |

**Ledger exclusions:** {"candidate_id": "crypto-v1-incremental", "reason": "crypto-v1 commitment and public context fix archive_role=1", "invariant_violated": "crypto-v1 wire frozen"}.

**R0 checklist:** 1 status quo: no ledger id marked status quo; the critic pass confirms which candidate is the behaviour at `9e44608`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 5 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: `never`, `new-crypto-version-per-archive-afk`; 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `never`, `new-crypto-version-per-archive-afk`, `shared-key-domain-chain`, `encrypt-references-only`, `crypto-v1-incremental`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-INT-010 — What verification status is reported for encrypted random reads that do not verify segment END and the full segment-sequence digest, and is incremental remote whole-archive verification (PCR/PCI via ranges) required in v1?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-008, EXP-CRYPTO-010, EXP-INT-016, EXP-INT-020, EXP-REMOTE-010.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-per-object-plus-archivefinal` | Status quo: per-object AEAD plus authenticated ArchiveFinal and Descriptor/Manifest binding; PCR DeclaredNotFullyVerified; PCI NotComputed; no remote whole verification | status quo | no | EXP-CRYPTO-008, EXP-CRYPTO-010, EXP-INT-016, EXP-REMOTE-010 |
| `verify-touched-segment-ends` | Also verify END records of every touched segment and report segment-level status | ledger alternative | no | EXP-CRYPTO-008, EXP-CRYPTO-010, EXP-INT-016, EXP-REMOTE-010 |
| `per-segment-digests-in-archivefinal` | Bind per-segment digests usable by partial readers into ArchiveFinal | ledger alternative | no | EXP-CRYPTO-008, EXP-INT-016, EXP-REMOTE-010 |
| `remote-streaming-whole-verify` | Remote whole-archive verification fetching all ranges with bounded memory, computing PCR and PCI | ledger alternative | no | EXP-CRYPTO-008, EXP-CRYPTO-010, EXP-INT-016, EXP-REMOTE-010 |
| `full-download-only` | Whole-archive verification only after full download | ledger alternative | no | EXP-CRYPTO-008, EXP-CRYPTO-010, EXP-INT-016, EXP-REMOTE-010 |
| `merkle-over-segments` | Authenticated Merkle tree over segment digests enabling logarithmic partial proofs | ledger alternative | no | EXP-CRYPTO-008, EXP-INT-016, EXP-REMOTE-010 |

**Ledger exclusions:** {"candidate_id": "per-segment-digests-in-archivefinal", "reason": "Changes ArchiveFinalV1 fields; only possible in a new crypto wire version", "invariant_violated": "crypto-v1 wire frozen"}; {"candidate_id": "merkle-over-segments", "reason": "Requires new authenticated wire structures; only possible in a new crypto wire version", "invariant_violated": "crypto-v1 wire frozen"}.

**R0 checklist:** 1 status quo: `status-quo-per-object-plus-archivefinal`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 6 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-INT-032 — Is role participation in LAI, plus ContentRef kind in entry identity, sufficient against content-stripping and role-confusion attacks on signed archives, or must signatures or identities also bind base identities and sidecar describes_lai claims?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-014, EXP-INT-007.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-role-in-lai` | Status quo: LAI binds role byte; ContentRef kind in entry identity | status quo | no | EXP-INT-007 |
| `bind-base-identities` | Incremental LAI additionally binds referenced base identities | ledger alternative | no | EXP-INT-007 |
| `signature-explicit-role` | Signature transcript binds role explicitly | ledger alternative | no | EXP-INT-007 |
| `signed-requires-complete` | Only Complete archives may carry content signatures | ledger alternative | no | EXP-INT-007 |
| `describes-lai-separation` | Sidecar describes_lai carried as a separate signed claim, never conflated | defer / no change | no | EXP-INT-007 |
| `role-outside-identity` | Role declared in preamble only, not identity-bearing | ledger alternative | no | EXP-INT-007 |

**Ledger exclusions:** {"candidate_id": "role-outside-identity", "reason": "Reopens the content-deletion attack closed by binding role into LAI", "invariant_violated": "SPEC §8.5 L963 role in LAI (review-pass fix)"}.

**R0 checklist:** 1 status quo: `status-quo-role-in-lai`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 6 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: `describes-lai-separation`; 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-INT-033 — What salvage guarantees apply to damaged encrypted archives: fail closed, recover individually authenticated records with keys, require intact commitment and envelope, or rely on external parity?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-010, EXP-INT-016.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `fail-closed-no-salvage` | No salvage for encrypted archives | ledger alternative | no | EXP-CRYPTO-010, EXP-INT-016 |
| `authenticated-record-recovery` | With keys, recover records whose AEAD authenticates, reporting archive unverified | ledger alternative | no | EXP-CRYPTO-010, EXP-INT-016 |
| `require-intact-envelope` | Salvage only when commitment, envelope and recipient directory are intact | ledger alternative | no | EXP-CRYPTO-010, EXP-INT-016 |
| `parity-required-for-encrypted` | Recommend or require external parity for encrypted archives | ledger alternative | no | EXP-CRYPTO-010, EXP-INT-016 |
| `public-framing-recovery-only` | Recover and report only public framing information | ledger alternative | no | EXP-CRYPTO-010, EXP-INT-016 |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: no ledger id marked status quo; the critic pass confirms which candidate is the behaviour at `9e44608`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 5 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-ECO-022 — Which external key sources (age X25519, SSH keys, OpenPGP, PIV/FIDO2 hardware tokens, cloud KMS, minisign or Sigstore signing identities) must v1 or v1.x support for recipients and signing?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `HUMAN_PARTICIPANTS`. Other experiments informing it: EXP-ECO-022.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-xwing-and-password` | X-Wing hybrid recipients and one Argon2id password recipient | status quo | no | — |
| `age-plugin-model` | age-style plugin protocol for hardware tokens and KMS | ledger alternative | no | — |
| `ssh-key-recipients` | SSH ed25519 keys wrapped in a hybrid construction | ledger alternative | no | — |
| `openpgp-recipients` | OpenPGP key support | ledger alternative | no | — |
| `hardware-token-pq` | Post-quantum-capable hardware tokens when available | ledger alternative | no | — |
| `external-signing-identities` | minisign, SSH signatures or Sigstore identities for signing only | ledger alternative | no | — |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: `status-quo-xwing-and-password`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 6 ledger candidates listed above; 4 strongest incumbent: `age-plugin-model`, `openpgp-recipients`, `external-signing-identities`; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `status-quo-xwing-and-password`, `age-plugin-model`, `ssh-key-recipients`, `openpgp-recipients`, `hardware-token-pq`, `external-signing-identities`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-ECO-072 — How do consumers verify supply-chain properties with Entrybound (signatures, identities, derivation receipts), and how does this compose with Sigstore, in-toto/SLSA provenance, SPDX/CycloneDX and registry attestations?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-ECO-007, EXP-ECO-014.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-native` | Ed25519 detached/embedded signatures, offline RFC 3161 and JSON receipts | status quo | no | EXP-ECO-007 |
| `in-toto-subjects` | Emit in-toto statements with Entrybound identities as subjects | ledger alternative | no | EXP-ECO-007 |
| `sigstore-bundles` | Keyless Sigstore signing over PCI or LAI | ledger alternative | no | EXP-ECO-007 |
| `minisign-compat` | minisign/signify-compatible detached signatures | ledger alternative | no | EXP-ECO-007 |
| `registry-attestations` | Publish through registry attestation APIs (PyPI, npm provenance) | ledger alternative | no | EXP-ECO-007 |
| `verify-third-party-attestations` | Verify existing attestations that reference Entrybound identities | ledger alternative | no | EXP-ECO-007 |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: `status-quo-native`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 6 ledger candidates listed above; 4 strongest incumbent: `status-quo-native`, `in-toto-subjects`, `sigstore-bundles`, `minisign-compat`; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-CMP-001 — Which CDC algorithm, normalization level and boundary-mask construction should the v1 chunker and its keyed variants use: keep frozen gear-norm-v1 (Gear hash, contiguous low-bit masks, one-bit normalization, every byte hashed) or register a measured successor such as spread-mask NC-2 Gear, FastCDC-2020, Rabin, BuzHash, AE, RAM, SeqCDC, VectorCDC or Chonkers?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CHUNK-001, EXP-CHUNK-002, EXP-CHUNK-003, EXP-CHUNK-004, EXP-CHUNK-006, EXP-CHUNK-008, EXP-CHUNK-009, EXP-CHUNK-012, EXP-CHUNK-013, EXP-CRYPTO-003, EXP-EVAL-012, EXP-SCALE-012, EXP-SCALE-014.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-gear-norm-v1` | Frozen gear-norm-v1: Gear table from splitmix64, hash reset per chunk, low b+1/b-1 bit masks (NC-1 equivalent), no cut before minimum, forced maximum | status quo | no | — |
| `gear-spread-mask-nc2` | Gear with FastCDC-2020 spread (high-bit) masks at NC-2, same min/max semantics, new chunker_id | ledger alternative | no | EXP-CHUNK-001, EXP-CHUNK-002, EXP-CHUNK-006 |
| `fastcdc-2016` | Original FastCDC (USENIX ATC 2016): Gear hash with normalized chunking and sub-minimum cut-point skipping, one byte per step; the Phase B2 harness plan lists FastCDC-2016 separately from 2020 | ledger alternative | no | EXP-CHUNK-001, EXP-CHUNK-002 |
| `fastcdc-2020` | Full FastCDC 2020: normalized spread masks, hash skipping before minimum, rolling-two-bytes optimization with identical boundaries | ledger alternative | no | EXP-CHUNK-001, EXP-CHUNK-002 |
| `tttd` | Two Thresholds Two Divisors (Eshghi and Tang 2005): Rabin-based chunking with a backup divisor; the classic literature baseline for chunk-size variance | ledger alternative | no | EXP-CHUNK-001, EXP-CHUNK-002, EXP-CHUNK-008 |
| `rabin-fingerprint` | Rabin polynomial rolling hash over a 48-64 byte window (restic-style), optionally per-archive polynomial | ledger alternative | no | EXP-CHUNK-001, EXP-CHUNK-002, EXP-CHUNK-006 |
| `buzhash` | Cyclic-polynomial BuzHash rolling hash (casync, Borg 1.x buzhash, kopia) | ledger alternative | no | EXP-CHUNK-001, EXP-CHUNK-002 |
| `ae-extremum` | Asymmetric Extremum hashless chunking with corrected h~mu-256 parameterization (lowest size variance) | ledger alternative | no | EXP-CHUNK-001, EXP-CHUNK-002, EXP-CHUNK-006 |
| `ram-hashless` | Rapid Asymmetric Maximum hashless chunking (high throughput; pathological on low-entropy data) | ledger alternative | no | EXP-CHUNK-001, EXP-CHUNK-002, EXP-CHUNK-006 |
| `seqcdc` | SeqCDC hashless monotonic byte-run chunking | ledger alternative | no | EXP-CHUNK-001, EXP-CHUNK-002, EXP-CHUNK-008 |
| `vectorcdc-simd` | VectorCDC/VRAM SIMD hashless chunker with a scalar reference and cross-ISA boundary identity | ledger alternative | no | EXP-CHUNK-001, EXP-CHUNK-002, EXP-CHUNK-008, EXP-CHUNK-013 |
| `chonkers` | Chonkers CDC with provable chunk-size and edit-locality bounds (unoptimised reference today) | ledger alternative | no | EXP-CHUNK-001, EXP-CHUNK-002, EXP-CHUNK-008 |
| `quickcdc-feature-acceptance` | QuickCDC/RapidCDC-style fast-forwarding that accepts memoized lengths on partial feature-vector matches | ledger alternative | no | EXP-CHUNK-001, EXP-CHUNK-002, EXP-CHUNK-003, EXP-CHUNK-004, EXP-CHUNK-008, EXP-CHUNK-013 |
| `fixed-size-blocks` | Fixed-size blocks (Duplicati, MSIX, Nydus) as measurement baseline | ledger alternative | no | EXP-CHUNK-002 |

**Ledger exclusions:** {"candidate_id": "quickcdc-feature-acceptance", "reason": "Accepts a chunk boundary on a partial feature match without content-defined scanning; SPEC excludes this class regardless of throughput as a silent-corruption risk", "invariant_violated": "SPEC §7.3 L803 / §25.1 row 6 boundary-skipping exclusion"}; {"candidate_id": "fixed-size-blocks", "reason": "Frozen SPEC rejection of fixed blocking for new archives; retained only as a measurement baseline", "invariant_violated": "SPEC §7.2 L792"}.

**R0 checklist:** 1 status quo: `status-quo-gear-norm-v1`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 14 ledger candidates listed above; 4 strongest incumbent: `rabin-fingerprint`, `buzhash`; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `status-quo-gear-norm-v1`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-LEG-089 — Should ZIP import support encrypted entries: keep refusing (status quo), support WinZip AES (AE-1/AE-2) with caller-supplied password, accept ZipCrypto with a broken-crypto warning, or list metadata only?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-019, EXP-LEGACY-001, EXP-LEGACY-010.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-refuse` | Refuse traditional and strong encryption flags under every mode | status quo | no | EXP-CRYPTO-019, EXP-LEGACY-001 |
| `aes-ae1-ae2` | Decrypt WinZip AES entries with a caller password; AE-2 CRC=0 handled by HMAC verification | ledger alternative | no | EXP-CRYPTO-019 |
| `zipcrypto-with-warning` | Decrypt ZipCrypto with explicit broken-crypto diagnostic | ledger alternative | no | EXP-CRYPTO-019 |
| `list-only` | Observe and list encrypted entries without decrypting; refuse conversion | ledger alternative | no | EXP-CRYPTO-019 |
| `post-v1-deferral` | Defer all encrypted import post-v1 | defer / no change | no | EXP-CRYPTO-019 |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: `status-quo-refuse`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 5 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: `post-v1-deferral`; 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-CRY-025 — Should dedup under encryption stay within one archive key domain, be disabled for provider-correlation threat models, or later extend to multi-archive key domains?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CHUNK-006, EXP-CHUNK-007, EXP-CRYPTO-006, EXP-CRYPTO-007.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-single-archive-domain` | Status quo: dedup within one archive under a fresh AFK | status quo | no | EXP-CRYPTO-007 |
| `no-dedup-option` | Option to disable dedup under encryption | ledger alternative | no | EXP-CHUNK-006, EXP-CRYPTO-006, EXP-CRYPTO-007 |
| `multi-archive-key-domain` | Shared key domain across incremental/base archives (post-v1) | defer / no change | no | — |
| `cross-tenant-convergent` | Cross-tenant or convergent dedup | ledger alternative | no | EXP-CHUNK-006 |

**Ledger exclusions:** {"candidate_id": "cross-tenant-convergent", "reason": "Confirmation-of-file attacks; frozen non-goal", "invariant_violated": "SPEC §9.7; docs/crypto-threat-model-v1.md §Non-goals L208"}.

**R0 checklist:** 1 status quo: `status-quo-single-archive-domain`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 4 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: `multi-archive-key-domain`; 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `multi-archive-key-domain`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-CRY-028 — Under encryption, should the planner disable or restrict cross-file dictionaries, lookback groups and similarity cohorts, require stronger padding, or keep full compression (which the CDC-attack literature recommends)?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-005, EXP-PLANNER-006, EXP-PLANNER-012.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-same-planner` | Status quo: identical v6 planning with *-enc-v1 planner ids | status quo | no | EXP-CRYPTO-005, EXP-PLANNER-006 |
| `no-cross-file-context-when-encrypted` | Disable shared dictionaries, lookback and similarity mixing under encryption | ledger alternative | no | EXP-CRYPTO-005, EXP-PLANNER-006 |
| `same-owner-cross-file-only` | Allow cross-file context only within one input source/owner | ledger alternative | no | EXP-CRYPTO-005, EXP-PLANNER-006 |
| `require-maximum-padding-for-cross-file` | Cross-file context only with MAXIMUM padding | ledger alternative | no | EXP-CRYPTO-005 |
| `mandatory-compression-under-encryption` | Always compress under encryption; forbid stored-only chunks | ledger alternative | no | EXP-CRYPTO-005, EXP-PLANNER-006 |
| `profile-opt-in` | Cross-file context under encryption only for explicit Dense/Extreme opt-in | ledger alternative | no | EXP-CRYPTO-005, EXP-PLANNER-006 |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: `status-quo-same-planner`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 6 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-CRY-030 — Which bytes of an encrypted archive remain public (preamble feature bits including embedded-signature presence 0x200, recipient count, padded lengths, segment and record counts, buckets) and which must be hidden, padded or declared as accepted leakage?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-007, EXP-CRYPTO-008.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-documented-public-framing` | Status quo: public discovery, envelope, framing and feature bits | status quo | no | EXP-CRYPTO-007 |
| `hide-signature-presence-new-version` | Move signature presence out of public feature bits (new version) | ledger alternative | no | EXP-CRYPTO-007 |
| `pad-record-counts` | Pad record and segment counts | ledger alternative | no | EXP-CRYPTO-007 |
| `maximum-padding-default` | Default to maximum padding mode | ledger alternative | no | EXP-CRYPTO-007 |
| `declare-accepted-leakage` | Accept and document each residual leak | ledger alternative | no | EXP-CRYPTO-007 |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: `status-quo-documented-public-framing`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 5 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-CRY-034 — How is a future crypto suite (e.g. wider nonces, AEGIS, PQ changes) introduced, registered, deprecated and protected against downgrade, and who re-verifies external standards status on what cadence?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-020, EXP-EVAL-008, EXP-CRYPTO-023.

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

#### DEC-CRY-045 — Is recipient-set integrity against other legitimate recipients a v1 goal: status quo (any file-key holder can edit the envelope; only an expected addressing signature detects it), mandatory signatures for multi-recipient archives, sender-authenticated KEM, per-recipient MACs, or a documented non-goal?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-015, EXP-CRYPTO-023.

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

#### DEC-CRY-050 — Should key removal and password change keep rotating the AFK and rechunking under new boundary keys (PCR and physical bindings change), or offer a boundary-preserving or lazily rotated mode that keeps PCR stable at the cost of reusing boundary secrets known to removed recipients?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-015, EXP-SCALE-010.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-full-rotation-rechunk` | Status quo: new AFK, archive ID and boundary keys; full re-plan | status quo | no | EXP-CRYPTO-015 |
| `boundary-preserving-rotation` | Rotate AFK but keep boundary key and chunking | ledger alternative | no | EXP-CRYPTO-015, EXP-SCALE-010 |
| `separate-boundary-key-new-version` | Derive boundary keys independently of AFK (new version) | ledger alternative | no | EXP-CRYPTO-015 |
| `lazy-rotation-on-repack` | Rotate boundary keys only at next repack | ledger alternative | no | EXP-CRYPTO-015, EXP-SCALE-010 |
| `rewrap-only-removal` | Remove stanza without rotating the file key | ledger alternative | no | EXP-CRYPTO-015, EXP-SCALE-010 |

**Ledger exclusions:** {"candidate_id": "rewrap-only-removal", "reason": "Removed recipients would keep the file key", "invariant_violated": "docs/crypto-threat-model-v1.md L105-107 removal rotates file key"}.

**R0 checklist:** 1 status quo: `status-quo-full-rotation-rechunk`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 5 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-CRY-060 — What exactly does a VALID physical (PCR) binding attest, and must decode-path records (TransformPlans, Dictionaries, reconstruction data) be covered by some signed binding?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-014.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-pcr-only` | Status quo: PCR only; scope undocumented | status quo | no | — |
| `rely-on-content-binding` | Document that decoded output is verified against LAI-covered logical digests | ledger alternative | no | — |
| `decode-path-digest-binding-new-version` | Add a decode-path digest binding in a new version | ledger alternative | no | — |
| `plans-in-pcr` | Redefine PCR to include plans and dictionaries (identity change) | ledger alternative | no | — |
| `document-and-test-only` | Document scope and add substitution tests without format change | ledger alternative | no | — |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: `status-quo-pcr-only`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 5 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `status-quo-pcr-only`, `rely-on-content-binding`, `decode-path-digest-binding-new-version`, `plans-in-pcr`, `document-and-test-only`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-CRY-064 — Which recipient-related public fields (stanza count, types, classes, X-Wing encapsulations, password salt and Argon2 parameters) are acceptable, should recipient count be padded with dummy stanzas, and what may keyless inspect --crypto and diff --public report about recipients? The whole-archive public byte inventory is decided in encrypted-public-byte-leakage-inventory.

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-006, EXP-CRYPTO-007, EXP-CRYPTO-017.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-public-count-types` | Status quo: count, types, classes and parameters public; no stanza padding; keyless inspect and diff show them | status quo | no | EXP-CRYPTO-007 |
| `dummy-stanza-padding` | Optional dummy stanzas bucketing recipient count (SPEC §9.8 optional stanza padding) | SPEC design | no | EXP-CRYPTO-007 |
| `uniform-stanza-encoding` | Indistinguishable stanza encodings across types in a later version | defer / no change | no | EXP-CRYPTO-007 |
| `restrict-keyless-reporting` | Keyless inspect and diff omit recipient count and types by default | ledger alternative | no | EXP-CRYPTO-007 |
| `document-only` | Keep exposure and document it precisely in inspect --crypto | defer / no change | no | EXP-CRYPTO-007 |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: `status-quo-public-count-types`; 2 SPEC design: `dummy-stanza-padding`; 3 ledger alternatives: 5 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: `uniform-stanza-encoding`, `document-only`; 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-CRY-069 — Which recipient-mutation semantics bind the envelope in v1: AFK-preserving addition producing key-sharing sibling archives, fresh-epoch removal and password change with full re-encryption, rotation keeping every recipient, combined add and remove, and carry-over of directory labels, at what measured cost? Boundary-preserving rotation, beyond-RAM re-encryption and signature retention are decided in key-rotation-boundary-preservation, encryption-beyond-ram and signature-retention-after-recipient-mutation.

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-015, EXP-SCALE-010, EXP-CRYPTO-023.

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

#### DEC-CRY-072 — Is remote range access-pattern leakage accepted and documented for v1, or mitigated (whole-segment or batched prefetch, dummy fetches, https-only), and should cleartext http:// sources be refused?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-008, EXP-REMOTE-010.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-documented-non-goal` | Status quo: no mitigation; leakage documented; http and https accepted | status quo | no | EXP-CRYPTO-008, EXP-REMOTE-010 |
| `batched-dependency-prefetch` | Fetch whole dependency closures or segment batches to blur which entry is read | ledger alternative | no | EXP-CRYPTO-008, EXP-REMOTE-010 |
| `dummy-range-requests` | Optional dummy or padded range requests | ledger alternative | no | EXP-CRYPTO-008, EXP-REMOTE-010 |
| `https-only-default` | Refuse http:// by default; opt-in flag for cleartext | ledger alternative | no | EXP-CRYPTO-008, EXP-REMOTE-010 |
| `full-oblivious-access` | ORAM/PIR-style oblivious access | ledger alternative | no | EXP-CRYPTO-008, EXP-REMOTE-010 |

**Ledger exclusions:** {"candidate_id": "full-oblivious-access", "reason": "Oblivious access is a frozen non-goal of crypto v1", "invariant_violated": "docs/crypto-threat-model-v1.md §Non-goals L198-199"}.

**R0 checklist:** 1 status quo: `status-quo-documented-non-goal`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 5 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-CRY-077 — Which secret-dependent failures (wrong identity, wrong password, tag failure, padding or structure invalid after decryption, policy refusal) must be indistinguishable in reason codes, messages and timing?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CONF-003, EXP-CONF-004, EXP-CRYPTO-017.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-selective-collapse` | Status quo: envelope auth failure collapsed; post-decryption padding/structure errors have distinct codes | status quo | no | EXP-CRYPTO-017 |
| `collapse-all-post-decryption-errors` | Collapse all post-decryption failures to one integrity code at the user boundary | ledger alternative | no | EXP-CRYPTO-017 |
| `verbose-local-diagnostics-flag` | Distinct codes only with an explicit local diagnostics flag | ledger alternative | no | EXP-CRYPTO-017 |
| `constant-time-unlock-loop` | Make unlock attempt timing independent of which stanza matched | ledger alternative | no | EXP-CRYPTO-017 |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: `status-quo-selective-collapse`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 4 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-CRY-089 — Must payload_suite_id and crypto version be signed in every signature over an encrypted archive: require addressing for encrypted signing, move suite into the content binding in a new version, argue that commitment and envelope MAC already bind it, or amend SPEC agility rule 7?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-014, EXP-CRYPTO-023.

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

#### DEC-CRY-105 — When a stanza opens but the candidate AFK fails commitment or envelope MAC, should unlock hard-fail (status quo), continue to remaining stanzas, or evaluate all stanzas before deciding?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-010, EXP-CRYPTO-015, EXP-CRYPTO-023.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-hard-fail` | Status quo: integrity failure at the first commitment or MAC mismatch | status quo | no | EXP-CRYPTO-010, EXP-CRYPTO-023 |
| `continue-then-report` | Continue to remaining stanzas; report integrity failure only if none succeed | ledger alternative | no | EXP-CRYPTO-010, EXP-CRYPTO-023 |
| `evaluate-all-require-unique` | Evaluate all matching stanzas and require one consistent AFK | ledger alternative | no | EXP-CRYPTO-010, EXP-CRYPTO-023 |
| `continue-with-tamper-warning` | Continue and surface a tamper warning if another stanza succeeds | ledger alternative | no | EXP-CRYPTO-010, EXP-CRYPTO-023 |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: `status-quo-hard-fail`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 4 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.
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

_Generated by `python research/tools/experiments/fill_crypto_auto_blocks.py` for EXP-CRYPTO-021; edit the inputs, not this block._
<!-- END AUTO:common -->
