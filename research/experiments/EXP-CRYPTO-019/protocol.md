# EXP-CRYPTO-019: Legacy encrypted import/export, confidentiality downgrade, and receipt verification

<!-- BEGIN AUTO:meta -->
| Field | Value |
|---|---|
| Index status | **NEEDS_TOOLING** — Decrypt-path soundness is EXTERNAL_REVIEW_REQUIRED |
| Kind | decision |
| Domain | crypto (program §22.1, §22.4, §22.5) |
| Decisions informed | DEC-LEG-012, DEC-LEG-089, DEC-CRY-061, DEC-LEG-009, DEC-CRY-093, DEC-CRY-091, DEC-CRY-027, DEC-LEG-011 |
| Platforms | Linux/x86-64 |
| Requires timing | False |
| Estimates | 20 machine-hours; 15 GB disk |
| Tooling to build | ebr-crypto legacy; receipt_verify.py + forgery generator |
| Environment prerequisites | none beyond the harness and the ebr venv |
| Blocked arms | none |
| Specs | `spec.yaml` |
| Validation looks | owns look 1 for DEC-CRY-061, DEC-CRY-091, DEC-CRY-093, DEC-LEG-011; participates in looks owned by other experiments for DEC-CRY-027, DEC-LEG-009, DEC-LEG-012, DEC-LEG-089 |
| Gates | G-A and G-B unmet at this revision (generated `gate_state` column of `research/experiments/index.csv`); timing runs also need a PASS calibration record for the calibration identity (PA-04) |
| Conventions | `research/experiments/_index/crypto-conventions.md` (CR0-CR10); program amendments `research/experiments/_program/design-revision-round1.md` |
<!-- END AUTO:meta -->

## 1. Question

Should encrypted legacy inputs (ZipCrypto, WinZip AES, 7z AES) be refused, decrypted with typed warnings, or preserved opaquely (DEC-LEG-012, DEC-LEG-089)? Must exporting or publishing an authenticated encrypted archive to a plaintext legacy target require explicit opt-in and record the downgrade (DEC-CRY-061)? How does a consumer verify a legacy artifact was produced from an `.eb`, and can receipts and sidecars be signed (DEC-LEG-009, DEC-CRY-093, DEC-ECO-072)? Should `convert`/`publish` produce encrypted native output directly, without staging plaintext on disk (DEC-CRY-027)?

## 2. Hypotheses and falsification

- **H1 (import safety).** Decrypting ZipCrypto (broken CRC-based cipher, Biham-Kocher) cannot be authenticated, so `decrypt-all-including-zipcrypto` yields an unauthenticated-source outcome that must be typed as declared loss; WinZip AES and 7z AES carry authentication that can be verified with a caller password under a KDF-cost budget.
- **H2 (downgrade recording).** `status-quo-warning-and-receipt` records the confidentiality downgrade in the receipt but a scripted export can miss the warning; `explicit-flag-required` refuses plaintext output without `--allow-plaintext`. `silent-export` is excluded at R1.
- **H3 (receipt verification).** `reexport-byte-compare` reproduces the legacy artifact deterministically for a measured subset; `signed-receipt` binds source LAI and result digest and detects a forged receipt; `status-quo-unsigned-receipt` has no verification procedure.
- **H4 (direct encrypted output).** `direct-encrypted-import` keeps working memory bounded and writes no plaintext to disk; `encrypted-import-with-temp-staging` writes plaintext-derived bytes to a temp file (a confidentiality note).
- **Falsification:** each H is a directional/conformance verdict; a silent downgrade or an unverifiable forged receipt accepted as genuine is a finding.

## 3. Decisions informed and evidence route

<!-- BEGIN AUTO:candidates -->
#### DEC-LEG-012 — Should encrypted legacy inputs (ZipCrypto, WinZip AES, 7z AES, RAR5) remain refused, be decrypted with typed integrity warnings in provenance, or be preserved opaquely?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-LEGACY-001, EXP-LEGACY-003, EXP-LEGACY-010.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-refuse` | Status quo: refuse all encrypted legacy inputs | status quo | yes | EXP-LEGACY-001, EXP-LEGACY-003 |
| `decrypt-winzip-aes-and-7z-aes-only` | Decrypt authenticated or CRC-checked modern schemes with explicit password and KDF cost budget, recording unauthenticated-source warnings | ledger alternative | yes | EXP-LEGACY-003 |
| `decrypt-all-including-zipcrypto` | Decrypt all schemes including ZipCrypto with warnings | ledger alternative | yes | EXP-LEGACY-003 |
| `opaque-preservation` | Preserve encrypted source opaquely with provenance but no entry projection | ledger alternative | yes | EXP-LEGACY-003 |

**Ledger exclusions:** none recorded in the ledger.
**Exclusions proposed in this protocol (need independent sign-off, CR2):** `embed-signature-in-legacy-target`, `silent-export`.

**R0 checklist:** 1 status quo: `status-quo-refuse`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 4 ledger candidates listed above; 4 strongest incumbent: `decrypt-winzip-aes-and-7z-aes-only`; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-LEG-089 — Should ZIP import support encrypted entries: keep refusing (status quo), support WinZip AES (AE-1/AE-2) with caller-supplied password, accept ZipCrypto with a broken-crypto warning, or list metadata only?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-021, EXP-LEGACY-001, EXP-LEGACY-010.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-refuse` | Refuse traditional and strong encryption flags under every mode | status quo | yes | EXP-LEGACY-001 |
| `aes-ae1-ae2` | Decrypt WinZip AES entries with a caller password; AE-2 CRC=0 handled by HMAC verification | ledger alternative | yes | — |
| `zipcrypto-with-warning` | Decrypt ZipCrypto with explicit broken-crypto diagnostic | ledger alternative | yes | — |
| `list-only` | Observe and list encrypted entries without decrypting; refuse conversion | ledger alternative | yes | — |
| `post-v1-deferral` | Defer all encrypted import post-v1 | defer / no change | yes | — |

**Ledger exclusions:** none recorded in the ledger.
**Exclusions proposed in this protocol (need independent sign-off, CR2):** `embed-signature-in-legacy-target`, `silent-export`.

**R0 checklist:** 1 status quo: `status-quo-refuse`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 5 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: `post-v1-deferral`; 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-CRY-061 — Must exporting or publishing an authenticated encrypted archive to plaintext legacy targets require explicit opt-in, and how is the confidentiality downgrade recorded and classified?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `HUMAN_PARTICIPANTS`. Other experiments informing it: EXP-CRYPTO-022, EXP-ECO-016, EXP-ECO-019, EXP-ECO-021, EXP-LEGACY-030, EXP-LEGACY-070.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-warning-and-receipt` | Status quo: warning line plus receipt fields | status quo | yes | EXP-LEGACY-030, EXP-LEGACY-070 |
| `explicit-flag-required` | Refuse plaintext output without --allow-plaintext or --downgrade-encryption | ledger alternative | yes | EXP-LEGACY-030, EXP-LEGACY-070 |
| `interactive-confirmation` | Prompt on TTY; flag required non-interactively | ledger alternative | yes | EXP-LEGACY-030, EXP-LEGACY-070 |
| `declared-loss-outcome` | Classify as declared loss in outcome class and receipt | ledger alternative | yes | EXP-LEGACY-030, EXP-LEGACY-070 |
| `silent-export` | Export plaintext with no warning or record | ledger alternative | yes | EXP-LEGACY-030, EXP-LEGACY-070 |

**Ledger exclusions:** {"candidate_id": "silent-export", "reason": "Silent confidentiality downgrade is prohibited and receipt must record the transition", "invariant_violated": "SPEC §15.2 L1697; docs/legacy-export-v1.md L49-51"}.
**Exclusions proposed in this protocol (need independent sign-off, CR2):** `embed-signature-in-legacy-target`, `silent-export`.

**R0 checklist:** 1 status quo: `status-quo-warning-and-receipt`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 5 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-LEG-009 — How does a consumer holding the .eb verify a legacy artifact was produced from it: deterministic re-export and byte comparison, logical comparison against LAI via strict import, a signed receipt, or a combination?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-ECO-007, EXP-ECO-008, EXP-INT-017.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-unsigned-receipt` | Status quo: receipt names source LAI; no verification procedure | status quo | yes | EXP-ECO-007, EXP-INT-017 |
| `reexport-byte-compare` | Re-export with the recorded profile and compare SHA-256 | ledger alternative | yes | EXP-ECO-007, EXP-ECO-008, EXP-INT-017 |
| `strict-import-lai-compare` | Strict-import the artifact and compare LAI (LOSSLESS only) or issue-predicted differences | ledger alternative | yes | EXP-ECO-007, EXP-ECO-008, EXP-INT-017 |
| `signed-receipt` | Receipt signed with the archive signing key binding source LAI and result digest | ledger alternative | yes | EXP-ECO-007, EXP-INT-017 |
| `signed-archive-embeds-result-digests` | Native archive signature covers expected legacy result digests | ledger alternative | yes | EXP-ECO-007, EXP-INT-017 |

**Ledger exclusions:** none recorded in the ledger.
**Exclusions proposed in this protocol (need independent sign-off, CR2):** `embed-signature-in-legacy-target`, `silent-export`.

**R0 checklist:** 1 status quo: `status-quo-unsigned-receipt`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 5 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-CRY-093 — Should export and migration receipts and sidecars be signable (SignatureRecord over receipt digest, DSSE/in-toto attestation, embedded native signatures), what does receipt --chain assert, and should unsigned sidecars stop being called "authenticated"?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-INT-008, EXP-LEGACY-034.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-unsigned` | Status quo: unsigned receipts and sidecars | status quo | yes | EXP-INT-008, EXP-LEGACY-034 |
| `detached-ebsig-over-receipt` | Detached Entrybound signature over receipt bytes | ledger alternative | yes | EXP-INT-008, EXP-LEGACY-034 |
| `dsse-in-toto-receipt` | Receipts as DSSE/in-toto attestations | ledger alternative | yes | EXP-INT-008, EXP-LEGACY-034 |
| `signed-sidecar-native` | Sidecars created with signing options | ledger alternative | yes | EXP-INT-008, EXP-LEGACY-034 |
| `wording-correction-only` | Correct terminology without signing | ledger alternative | yes | EXP-INT-008, EXP-LEGACY-034 |
| `embed-signature-in-legacy-target` | Embed Entrybound signatures into ZIP/tar targets | ledger alternative | yes | EXP-INT-008, EXP-LEGACY-034 |

**Ledger exclusions:** {"candidate_id": "embed-signature-in-legacy-target", "reason": "Legacy targets never embed Entrybound signatures", "invariant_violated": "docs/legacy-export-v1.md L51-52 export boundary freeze"}.
**Exclusions proposed in this protocol (need independent sign-off, CR2):** `embed-signature-in-legacy-target`, `silent-export`.

**R0 checklist:** 1 status quo: `status-quo-unsigned`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 6 ledger candidates listed above; 4 strongest incumbent: `dsse-in-toto-receipt`, `embed-signature-in-legacy-target`; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-CRY-091 — When importing, repacking or exporting signed legacy containers (JAR, APK, signed ZIP), does Entrybound detect and warn, preserve signature-bearing bytes for exact reconstruction, record declared loss, or refuse normalizing profiles?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-LEGACY-010, EXP-LEGACY-050.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-no-detection` | Status quo: no detection | status quo | yes | EXP-LEGACY-050 |
| `detect-and-warn` | Detect META-INF signatures and APK signing blocks and warn | ledger alternative | yes | EXP-LEGACY-050 |
| `preserve-exact-reconstruction` | Preserve bytes so export reproduces the signed container | ledger alternative | yes | EXP-LEGACY-050 |
| `record-declared-loss` | Record legacy signature loss in FidelityReport | ledger alternative | yes | EXP-LEGACY-050 |
| `refuse-normalizing-profile` | Refuse normalizing profiles on signed inputs | ledger alternative | yes | EXP-LEGACY-050 |

**Ledger exclusions:** none recorded in the ledger.
**Exclusions proposed in this protocol (need independent sign-off, CR2):** `embed-signature-in-legacy-target`, `silent-export`.

**R0 checklist:** 1 status quo: `status-quo-no-detection`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 5 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-CRY-027 — Should convert import and publish accept --recipient or --password to produce encrypted native output directly from plaintext or legacy sources, and without staging plaintext on disk? Encrypted-to-encrypted repack is decided in encrypted-repack-mutation-contract.

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-009, EXP-EVAL-008.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-no-encrypted-output` | Status quo: import and publish cannot encrypt; users pack an extracted tree or publish an existing .eb | status quo | yes | — |
| `direct-encrypted-import` | convert import --recipient/--password writes encrypted INDEXED output directly | ledger alternative | yes | EXP-CRYPTO-009 |
| `direct-encrypted-publish` | publish --recipient/--password encrypts native artefacts from directories | ledger alternative | yes | — |
| `documented-two-step-workflow` | Document a two-step workflow and its plaintext exposure | ledger alternative | yes | — |
| `encrypted-import-with-temp-staging` | Encrypted import via temporary plaintext staging | ledger alternative | yes | EXP-CRYPTO-009 |

**Ledger exclusions:** none recorded in the ledger.
**Exclusions proposed in this protocol (need independent sign-off, CR2):** `embed-signature-in-legacy-target`, `silent-export`.

**R0 checklist:** 1 status quo: `status-quo-no-encrypted-output`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 5 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-LEG-011 — Must encrypted archives carry conversion provenance and preservation evidence in v1, and via which crypto record assignment and required feature, given crypto-v1 refuses archives with conversion data?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-EVAL-008.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-refuse` | Status quo: encrypting converted archives is refused; convert to unencrypted ECF | status quo | yes | — |
| `crypto-v2-provenance-record` | New crypto wire version assigning an encrypted provenance record and feature | ledger alternative | yes | — |
| `strip-then-encrypt` | Require stripping provenance before encryption with explicit loss | ledger alternative | yes | — |
| `external-encrypted-receipt` | Keep provenance in an external receipt encrypted to the same recipients | ledger alternative | yes | — |

**Ledger exclusions:** none recorded in the ledger.
**Exclusions proposed in this protocol (need independent sign-off, CR2):** `embed-signature-in-legacy-target`, `silent-export`.

**R0 checklist:** 1 status quo: `status-quo-refuse`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 4 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.
<!-- END AUTO:candidates -->

Evidence route: `EMPIRICALLY_MEASURED` outcomes over a fixed case set (T-20) plus size/memory where relevant. Legacy decrypt-path and credential-handling security review is `EXTERNAL_REVIEW_REQUIRED` (DEC-LEG-089 notes crypto-cluster review for any KDF/cipher; no credential handling in unreviewed code paths). The preservation-storage and forensic-recovery questions (DEC-LEG-060, DEC-LEG-096/097/098, DEC-LEG-046, DEC-LEG-040) are primarily legacy-domain; their encryption-composition cells are in EXP-CRYPTO-015 and their receipt-signing cells here.

## 4. Candidates and arms

- **DEC-LEG-012 encrypted legacy input:** `status-quo-refuse`, `decrypt-winzip-aes-and-7z-aes-only`, `decrypt-all-including-zipcrypto`, `opaque-preservation`.
- **DEC-LEG-089 ZIP encrypted entries:** `status-quo-refuse`, `aes-ae1-ae2`, `zipcrypto-with-warning`, `list-only`, `post-v1-deferral`.
- **DEC-CRY-061 confidentiality downgrade:** `status-quo-warning-and-receipt`, `explicit-flag-required`, `interactive-confirmation`, `declared-loss-outcome` (`silent-export` excluded at R1). HUMAN parts (accidental-export usability) → EXP-CRYPTO-022.
- **DEC-LEG-009 artifact-from-.eb verification:** `status-quo-unsigned-receipt`, `reexport-byte-compare`, `strict-import-lai-compare`, `signed-receipt`, `signed-archive-embeds-result-digests`.
- **DEC-CRY-093 signable receipts:** `status-quo-unsigned`, `detached-ebsig-over-receipt`, `dsse-in-toto-receipt`, `signed-sidecar-native`, `wording-correction-only` (`embed-signature-in-legacy-target` excluded at R1).
- **DEC-CRY-091 signed legacy containers:** `status-quo-no-detection`, `detect-and-warn`, `preserve-exact-reconstruction`, `record-declared-loss`, `refuse-normalizing-profile`.
- **DEC-CRY-027 direct encrypted output (POST_V1):** `status-quo-no-encrypted-output`, `direct-encrypted-import`, `direct-encrypted-publish`, `documented-two-step-workflow`, `encrypted-import-with-temp-staging`.
- **DEC-LEG-011 encrypted conversion provenance:** `status-quo-refuse`, `crypto-v2-provenance-record` (future version), `strip-then-encrypt`, `external-encrypted-receipt`.
- **DEC-ECO-072 supply-chain attestation** (mechanical composition cells only; trust semantics → external review).

Reference candidate: status quo of each decision.

## 5. Corpus selectors

- **Encrypted legacy inputs (tuning):** ZipCrypto, WinZip AES (AE-1/AE-2), 7z AES-256, from `f20-tuning-generated-private-vault` and `f20-validation-generated-private-vault` (which cover ZipCrypto, WinZip/7z AES, 7z encrypted headers, GPG, age, KeePass), plus small generated encrypted zips/7z with caller-known passwords.
- **Export downgrade / receipts (tuning):** encrypted `.eb` archives of `f01-tuning-curl-8-19-0-git`, `f04-tuning-sourcelike`; legacy targets ZIP and tar.
- **Signed legacy containers:** jarsigner-signed JARs and v1/v2 APKs from `f11` maven/APK-like items (or generated).
- **Validation:** analogous validation-split items, one registered look for graded fractions.

## 6. Environment and platform requirements

`wsl-ubuntu`, Linux x86-64. `7z` 23.01, `gpg` 2.4.4, `age` 1.1.1 available for producing/consuming legacy encrypted inputs. `timing: false` for outcomes; a bounded-memory sub-run for direct encrypted output (cross-ref EXP-CRYPTO-009). Passwords supplied through the harness. **Prohibited-action note:** the harness never enters real user credentials; all passwords are throwaway test secrets generated by the harness.

## 7. Commands and tooling

- `ebr-crypto legacy`: drives `ebound convert`/`publish`/`export` over each legacy encrypted input and export target, recording outcome class, receipt fields, warnings, exit status and (for direct output) peak memory and any plaintext temp file.
- `research/tools/crypto/receipt_verify.py`: implements `reexport-byte-compare`, `strict-import-lai-compare` and signed-receipt verification, and a forged-receipt generator (the counterexample-search protocol).
- Legacy encrypted producers use `7z`, `gpg`, `age` and Python `pyzipper`-style generation (from the venv) with committed test passwords.

## 8. Seeds, warmup, repetitions and adaptive rule

- Deterministic outcomes: 2 repetitions plus repeat-hash.
- Re-export determinism: 3 runs across two builds/hosts, compared byte-for-byte.
- No timing except the bounded-memory sub-run (§4.6 rounds).

## 9. Measurement method and metrics

| Metric (canonical) | ID | OD | Role |
|---|---|---|---|
| import outcome correctness over the case set | T-20 (`import_acceptance_fraction`, `import_agreement_fraction`) | OD-19 | **primary** |
| downgrade-record completeness (receipt records the transition) | `receipt_completeness_fraction` (M20.4) | OD-07 | **binary** |
| silent-downgrade occurrences | binary | OD-15 | **binary screen** (must be 0) |
| forged-receipt detection | binary | OD-15 | **binary** (signed-receipt candidates) |
| re-export byte-equivalence fraction | `export_acceptance_fraction` (T-20) | OD-19 | **primary** |
| unauthenticated-decryption typed-loss correctness | T-20 | OD-19 | primary |
| direct-encrypted-output peak memory + plaintext-temp presence | T-06 + binary | OD-08/OD-15 | secondary |
| `flags_per_task` for the downgrade opt-in | C6 | OD-25 | cost |

## 10. Normalization and statistical analysis

- Fixed-case fractions: T-20 exact; AGG-C headline.
- Binary screens (silent downgrade, forged-receipt detection): any failure is a finding.
- Confirmatory W against S on validation for graded fractions; MDE gate.
- Re-export determinism per objective MVT-13 style (3 runs, must match).

## 11. Practical significance

T-20, M20.4, T-06 bands and §7.2 tiers from `research/methods/thresholds.json`. A crypto-v2 provenance record is a new wire version (T4); a decrypt path adds untrusted-input surface (C4) and needs external review before shipping.

## 12. Sensitivity analysis

Not weight-based. Reported arms: legacy scheme (ZipCrypto vs AES), export target format, receipt-signing method, and interactive vs non-interactive downgrade.

## 13. Expected negative results worth recording

- ZipCrypto decryption cannot be authenticated, so any import must type it as declared loss (or refuse), against `decrypt-all-including-zipcrypto`.
- A scripted (non-TTY) export can miss the status-quo warning, supporting `explicit-flag-required` for DEC-CRY-061.
- Re-export is not always byte-reproducible (compat-profile drift), so `reexport-byte-compare` needs a measured-subset scope for DEC-LEG-009.

## 14. Threats to validity

- Legacy decrypt paths handle attacker-controlled ciphertext; their security is external-review-gated, so this experiment measures behaviour, not soundness.
- Test passwords are throwaway; no real credentials are handled.
- Preservation-storage decisions (DEC-LEG-060/096/097/098) are legacy-domain; only their crypto-composition cells are covered.
- APK/JAR signing-block detection heuristics may miss variants (reported).

## 15. Held-out placeholder (Phase D)

Import/export outcome and re-export determinism run on held-out legacy items after unlock; silent-downgrade and forged-receipt screens run on every split (R1-relevant). No held-out item named here.

## 16. Cost estimate and agent effort

- Compute: about 20 core-hours. Disk: about 15 GB.
- Agent effort: tooling **judgment** (legacy driver, receipt verify, forgery generator); execution **scripted**; analysis **judgment**; decrypt-path soundness routed to external review.

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

_Generated by `python research/tools/experiments/fill_crypto_auto_blocks.py` for EXP-CRYPTO-019; edit the inputs, not this block._
<!-- END AUTO:common -->
