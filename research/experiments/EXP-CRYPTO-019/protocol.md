# EXP-CRYPTO-019: Legacy encrypted import/export, confidentiality downgrade, and receipt verification

<!-- BEGIN AUTO:meta -->
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
<!-- END AUTO:common -->
