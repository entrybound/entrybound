# EXP-CRYPTO-011: Primitive and construction vector/KAT evidence gate (FORMAL / correctness)

<!-- BEGIN AUTO:meta -->
<!-- END AUTO:meta -->

## 1. Question

Does the implementation pass the complete external primitive and construction vector sets that the frozen suite requires before production (DEC-CRY-014), and are the derived-construction vectors (keyed boundaries, Ed25519 verification profile, key-commitment binding, HKDF label injectivity) published and independently reproducible (DEC-CRY-021, DEC-CRY-051, DEC-CRY-048, DEC-CRY-106)? This is a correctness gate, not an optimization.

## 2. Hypotheses and falsification

- **H1.** Every applicable known-answer vector passes: AES-256-GCM-SIV RFC 8452 Appendix C.2 (including failed-tag and counter-wrap), HKDF-SHA-256 RFC 5869 Appendix A, HMAC-SHA-256 RFC 4231, Argon2id RFC 9106 §5.3 and V6, ML-KEM-768 NIST ACVP/FIPS 203 (including invalid-ciphertext behaviour), every X-Wing draft-10 vector, Ed25519 RFC 8032 plus Project Wycheproof invalid encodings, and the boundary vectors (table derivation, PHTE field/rejection sampling, rolling versus direct evaluation, normalization, empty/tail).
- **H2 (independence).** X-Wing keygen/encapsulation/decapsulation matches at least one independent implementation (libcrux or the draft C reference); AES-GCM-SIV records match an implementation independent of RustCrypto; a Python reference (`reference.py` extended with PHTE and secret-table generation) reproduces the boundary and root/commitment vectors byte-for-byte.
- **H3 (negatives).** Negative vectors (invalid tags, noncanonical Ed25519 encodings, small-order points, invalid ML-KEM/X25519 inputs, truncated commitments) are all rejected.
- **Falsification:** any KAT failure, any independent-implementation mismatch, or any accepted negative vector falsifies the gate. The failure is committed as the artifact and blocks production crypto (`docs/crypto-review-v1.md` AR-25).

## 3. Decisions informed and evidence route

<!-- BEGIN AUTO:candidates -->
<!-- END AUTO:candidates -->

Evidence route: `FORMALLY_DERIVED` from exact test vectors (definitional, `PROVEN` for the vectors that pass) plus `EXTERNALLY_SOURCED` vector files with provenance URLs and hashes. FORMAL decision type: no held-out, no sensitivity (§1.2). A committed counterexample-search protocol (the negative-vector generator) and a reviewer not involved in the implementation are required (§1.2 FORMAL route).

## 4. Candidates and arms

The decisions here are evidence-gate decisions, so the "candidates" are evidence levels:
- **DEC-CRY-014:** `status-quo-single-vectors` (excluded at R1: one case per primitive does not satisfy the frozen requirement), `full-rfc-sets`, `plus-wycheproof-negative`, `third-party-reproduction`, `dependency-review`. This experiment executes the union (`plus-wycheproof-negative` + `third-party-reproduction` + `dependency-review`) and reports which level each primitive reaches.
- **DEC-CRY-021 Ed25519 profile:** `status-quo-dalek-verify-strict` versus `rfc8032-cofactorless-canonical`, `zip215-cofactored` (excluded at R1: accepts noncanonical/small-order), `libsodium-semantics`, `vector-defined-profile`. The accept/reject table is produced from Wycheproof and speccheck vectors run against `ed25519-dalek` verify_strict and cross-checked against libsodium, BoringSSL, Go and Python cryptography.
- **DEC-CRY-051 boundary vectors:** `status-quo-in-code-partial` versus `append-to-crypto-wire-vectors`, `independent-python-phte`, `paper-artifact-crosscheck`.
- **DEC-CRY-048 commitment binding coverage:** the identifier-to-transcript coverage table plus an identifier-rewrite mutation corpus and invisible-salamander/partitioning-oracle tests; candidates as in the ledger.
- **DEC-CRY-106 X-Wing/ML-KEM release gate:** `status-quo-draft10-kats-only` (excluded at R1), `kats-plus-one-independent`, `kats-plus-two-independent-and-review`, `plus-acvp-negative-and-errata`, `replace-crate-with-audited-impl`. This experiment executes `plus-acvp-negative-and-errata` where the vectors are available.

## 5. Corpus selectors

No corpus items. Inputs are vector files fetched from primary sources into `research/tools/crypto/vectors/` (each with a provenance URL, fetch date and SHA-256), plus generated random messages for property tests (rolling-versus-direct PHTE agreement, no `(key, nonce)` repeat across 10,000 records, stanza-order-independent set digest). No tuning, validation or held-out corpus item is used.

## 6. Environment and platform requirements

`wsl-ubuntu` (and re-run on `windows-host` for the RustCrypto path). Independent cross-checks need: `libcrux`/`ml-kem` reference (Rust), a Go implementation of AES-GCM-SIV or ML-KEM (Go 1.22 is present), and the draft C reference where available. `timing: false`. Network was used once to fetch vectors and is then offline; vector files are committed by hash. Note: `docs/crypto-review-v1.md` cites some sources (ePrint) as returning 403 to automated fetches; those are fetched by the program owner and committed by hash.

## 7. Commands and tooling

- Extend `crates/entrybound/tests/crypto_primitives.rs` and `crypto_v1.rs` with the full vector sets (these are production tests; per program discipline, agents propose the additions as a research patch in `research/harness/wip/` and the vectors live under `research/tools/crypto/vectors/`, loaded by a research harness test binary `ebr-crypto vectors` rather than by editing tracked test files, unless the owner applies the patch).
- `research/tools/crypto/vectors/manifest.json`: every vector file with source, date, hash.
- `tools/crypto-vector-helper/reference.py` extended (research copy under `research/tools/crypto/reference_ext.py`, never modifying tracked tooling) with PHTE and secret-table generation.
- Cross-implementation drivers: `research/tools/crypto/xcheck_{xwing,gcmsiv,mlkem}.{rs,go}`.
- Negative-vector generator `research/tools/crypto/negvec_gen.py` (the committed counterexample-search protocol).

## 8. Seeds, warmup, repetitions and adaptive rule

- KATs are exact and run once; the suite is deterministic. Property tests use seeded random inputs, 10,000 records for the `(key, nonce)` uniqueness bound (detection about 95% at 3×10⁻⁴ per record, §D.5) and 10^6 comparisons for the constant-time dudect test (MVT-14(h)).
- No warmups, no timing (except the dudect leakage test, which reports HC_UNVERIFIED on this noisy laptop rather than a violation per MVT-14(h)).

## 9. Measurement method and metrics

| Metric (canonical) | ID | Role |
|---|---|---|
| KAT pass/fail per vector set | `baseline_codec_public_spec`-style binary (correctness) | **binary** gate |
| cross-implementation agreement | binary | gate |
| negative-vector rejection | binary | gate |
| `(key, nonce)` uniqueness over 10,000 records | binary (HC-14(c)) | gate |
| PHTE rolling-versus-direct agreement | binary | gate |
| constant-time dudect result | binary or HC_UNVERIFIED (MVT-14(h)) | reported |

## 10. Normalization and statistical analysis

No statistics beyond the pre-registered detection bounds (§D.5). Pass/fail per vector set; a single failure fails the gate for that primitive or construction. `research/normalized/EXP-CRYPTO-011/decision/vector-gate.json` records each result with the vector source and hash.

## 11. Practical significance

None (correctness gate; §3.3 binary metrics, band [0,0]). Any non-zero failure is flagged.

## 12. Sensitivity analysis

Not applicable (FORMAL; reason recorded). The dudect timing test is re-run under stricter affinity if it triggers, but a timing failure is HC_UNVERIFIED, not a violation, on this host.

## 13. Expected negative results worth recording

- A primitive crate passes positive KATs but accepts a Wycheproof edge case (for example a noncanonical Ed25519 encoding under a non-strict verify), which selects `verify_strict` over the alternatives.
- The status-quo three-draft-10 X-Wing KATs are insufficient without an independent cross-check, confirming the R1 exclusion of `status-quo-draft10-kats-only`.
- The in-code partial boundary vectors do not cover empty/tail cases, requiring the published set.

## 14. Threats to validity

- Same-base-model implementation and cross-checker could share a bug; the cross-implementation requirement uses genuinely independent codebases (libcrux, Go, C reference).
- Vector provenance: a mirror could serve altered vectors, mitigated by committing hashes and, where possible, fetching from the standards body.
- The dudect constant-time test is unreliable on a virtualized laptop; it is routed to external review, not treated as a pass or a violation.

## 15. Held-out placeholder (Phase D)

Not applicable (FORMAL, no corpus). The gate is re-run at the design-freeze commit to confirm it still passes for the frozen builds.

## 16. Cost estimate and agent effort

- Compute: about 10 core-hours. Disk: under 1 GB (vector files).
- Agent effort: **judgment** to assemble vectors from primary sources and write cross-checks and negative generators; execution **scripted**; an independent reviewer (not the implementer) signs off (FORMAL route).

<!-- BEGIN AUTO:common -->
<!-- END AUTO:common -->
