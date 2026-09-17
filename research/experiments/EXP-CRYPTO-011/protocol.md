# EXP-CRYPTO-011: Primitive and construction vector/KAT evidence gate (FORMAL / correctness)

<!-- BEGIN AUTO:meta -->
| Field | Value |
|---|---|
| Index status | **NEEDS_TOOLING** |
| Kind | decision |
| Domain | crypto (program §22.1, §22.2) |
| Decisions informed | DEC-CRY-014, DEC-CRY-021, DEC-CRY-051, DEC-CRY-048, DEC-CRY-106, DEC-CRY-049 |
| Platforms | Windows+Linux/x86-64 |
| Requires timing | False |
| Estimates | 10 machine-hours; 1 GB disk |
| Tooling to build | vector files (owner-fetched); ebr-crypto vectors; xcheck_{xwing,gcmsiv,mlkem}; negvec_gen.py |
| Environment prerequisites | none beyond the harness and the ebr venv |
| Blocked arms | none |
| Specs | `spec.yaml` |
| Validation looks | owns no validation look |
| Gates | G-A and G-B unmet at this revision (generated `gate_state` column of `research/experiments/index.csv`); timing runs also need a PASS calibration record for the calibration identity (PA-04) |
| Conventions | `research/experiments/_index/crypto-conventions.md` (CR0-CR10); program amendments `research/experiments/_program/design-revision-round1.md` |
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
#### DEC-CRY-014 — What primitive and construction vector evidence gates v1 (full RFC 5869, 4231 and 9106 sets, Wycheproof, negative vectors, independent reproduction of V1-V8, full-mode Python reference runs) and what dependency review of hkdf, hmac, argon2 and aes-gcm-siv is required?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CONF-012, EXP-ECO-001.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-single-vectors` | Status quo: one positive vector per primitive; V5/V6 cross-implemented by project authors | status quo | yes | — |
| `full-rfc-sets` | Complete RFC vector sets for HKDF, HMAC, Argon2id and AES-GCM-SIV | ledger alternative | yes | EXP-CONF-012 |
| `plus-wycheproof-negative` | Add Wycheproof and negative or malformed vectors | ledger alternative | yes | EXP-CONF-012 |
| `third-party-reproduction` | Independent third party reproduces V1-V8 | ledger alternative | yes | EXP-CONF-012 |
| `dependency-review` | Documented review of pinned primitive crates | ledger alternative | yes | EXP-ECO-001 |

**Ledger exclusions:** {"candidate_id": "status-quo-single-vectors", "reason": "RFC 5869 Appendix A vectors and complete required sets are mandated before production; one case per primitive does not satisfy this", "invariant_violated": "docs/crypto-wire-v1.md L1082-1084"}.
**Exclusions proposed in this protocol (need independent sign-off, CR2):** `status-quo-draft10-kats-only`, `status-quo-single-vectors`, `zip215-cofactored`.

**R0 checklist:** 1 status quo: `status-quo-single-vectors`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 5 ledger candidates listed above; 4 strongest incumbent: `plus-wycheproof-negative`; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-CRY-021 — Which Ed25519 verification equation and encoding rules are normative (ed25519-dalek verify_strict plus weak-key rejection, RFC 8032 cofactorless canonical, cofactored ZIP-215, or a vector-defined profile), and how are cross-implementation verification differentials prevented?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CONF-012, EXP-CRYPTO-020.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-dalek-verify-strict` | Status quo: verify_strict plus explicit weak-key rejection | status quo | yes | — |
| `rfc8032-cofactorless-canonical` | RFC 8032 §5.1.7 cofactorless equation with canonical-encoding checks | ledger alternative | yes | — |
| `zip215-cofactored` | ZIP-215 cofactored verification accepting noncanonical A/R encodings | ledger alternative | yes | — |
| `libsodium-semantics` | libsodium crypto_sign_verify_detached acceptance rules | ledger alternative | yes | — |
| `vector-defined-profile` | Normative accept/reject table defined by speccheck and Wycheproof vectors | ledger alternative | yes | — |

**Ledger exclusions:** {"candidate_id": "zip215-cofactored", "reason": "Accepts noncanonical point encodings and small-order public keys that the frozen suite requires rejecting", "invariant_violated": "docs/crypto-suite-v1.md L651-654 strict verification freeze"}.
**Exclusions proposed in this protocol (need independent sign-off, CR2):** `status-quo-draft10-kats-only`, `status-quo-single-vectors`, `zip215-cofactored`.

**R0 checklist:** 1 status quo: `status-quo-dalek-verify-strict`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 5 ledger candidates listed above; 4 strongest incumbent: `rfc8032-cofactorless-canonical`, `zip215-cofactored`, `libsodium-semantics`, `vector-defined-profile`; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: `vector-defined-profile`.

#### DEC-CRY-051 — Where are normative keyed-boundary vectors (Gear table derivation, PHTE rejection sampling, rolling versus direct evaluation, normalization, empty/tail) published, and which independent implementation confirms them?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CONF-012.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-in-code-partial` | Status quo: first four table values, one polynomial and one self-generated range vector in unit tests | status quo | yes | — |
| `append-to-crypto-wire-vectors` | Publish boundary vectors in docs/crypto-wire-v1-vectors.txt generated by both helpers | ledger alternative | yes | — |
| `independent-python-phte` | Add PHTE and secret-table generation to reference.py for independent recomputation | ledger alternative | yes | — |
| `paper-artifact-crosscheck` | Cross-check against the Truong et al. reference implementation where applicable | ledger alternative | yes | — |

**Ledger exclusions:** none recorded in the ledger.
**Exclusions proposed in this protocol (need independent sign-off, CR2):** `status-quo-draft10-kats-only`, `status-quo-single-vectors`, `zip215-cofactored`.

**R0 checklist:** 1 status quo: `status-quo-in-code-partial`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 4 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-CRY-048 — Is every algorithm and mode identifier and the file-key commitment bound before use so identifier rewriting and non-committing KEM/AEAD attacks fail, and does the frozen HMAC commitment over the public crypto context suffice versus other committing constructions?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-021.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-hmac-commitment-public-context` | Status quo: HMAC-SHA256 commitment over public context plus envelope MAC, wrap AD and record AD | status quo | no | — |
| `hkdf-derived-commitment` | AWS Encryption SDK style HKDF-derived commitment value | ledger alternative | no | — |
| `ctx-committing-transform` | CTX-style committing transform over the AEAD tag | ledger alternative | no | — |
| `committing-aead` | Replace payload AEAD with a committing AEAD construction | ledger alternative | no | — |
| `no-explicit-commitment` | Rely on AES-GCM-SIV and KEM without explicit commitment | ledger alternative | no | — |

**Ledger exclusions:** {"candidate_id": "no-explicit-commitment", "reason": "AES-GCM-SIV and HPKE/KEM wrapping are not committing; the frozen suite requires explicit commitment", "invariant_violated": "docs/crypto-suite-v1.md L116-131 commitment freeze"}.
**Exclusions proposed in this protocol (need independent sign-off, CR2):** `status-quo-draft10-kats-only`, `status-quo-single-vectors`, `zip215-cofactored`.

**R0 checklist:** 1 status quo: `status-quo-hmac-commitment-public-context`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 5 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `status-quo-hmac-commitment-public-context`, `hkdf-derived-commitment`, `ctx-committing-transform`, `committing-aead`, `no-explicit-commitment`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-CRY-106 — What exact evidence gate must X-Wing and ML-KEM pass before production release (official KATs with provenance, one or two independent implementations, dependency review, FIPS 203 errata assessment, malformed-key and implicit-rejection tests), and is it met?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CONF-012, EXP-CRYPTO-021, EXP-ECO-001.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-draft10-kats-only` | Status quo: three draft-10 KATs, no interop or dependency review record | status quo | yes | — |
| `kats-plus-one-independent` | Draft-10 KATs plus cross-check with one independent implementation (docs/crypto-review-v1.md L157-158) | ledger alternative | yes | EXP-CONF-012 |
| `kats-plus-two-independent-and-review` | KATs plus libcrux and draft C reference cross-checks plus dependency review (AR-25, test plan item 3) | ledger alternative | yes | EXP-CONF-012 |
| `plus-acvp-negative-and-errata` | Above plus ACVP ML-KEM-768 vectors, malformed-key and implicit-rejection tests and FIPS 203 errata assessment | ledger alternative | yes | — |
| `replace-crate-with-audited-impl` | Replace x-wing 0.1.0 with an audited implementation reproducing draft-10 bytes | ledger alternative | yes | EXP-ECO-001 |

**Ledger exclusions:** {"candidate_id": "status-quo-draft10-kats-only", "reason": "Crate KATs alone do not satisfy the frozen gate requiring independent interoperability", "invariant_violated": "docs/crypto-suite-v1.md L469-471"}.
**Exclusions proposed in this protocol (need independent sign-off, CR2):** `status-quo-draft10-kats-only`, `status-quo-single-vectors`, `zip215-cofactored`.

**R0 checklist:** 1 status quo: `status-quo-draft10-kats-only`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 5 ledger candidates listed above; 4 strongest incumbent: `plus-acvp-negative-and-errata`; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-CRY-049 — What formal analysis and independent review of the key hierarchy, commitment, envelope MAC, recipient wrap, unlock order and AFK-sharing mutations must complete before production crypto, and in what form?

Ledger status `EXTERNAL_REVIEW_REQUIRED`, blocker class `EXTERNAL_REVIEW`. Other experiments informing it: EXP-CRYPTO-021, EXP-CRYPTO-015, EXP-CRYPTO-023.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-internal-adversarial-review` | Status quo: internal adversarial review AR-01..AR-34 only | status quo | no | — |
| `symbolic-model` | Symbolic model (Tamarin or ProVerif) of envelope, unwrap order, mutations and snapshot/storage adversaries | ledger alternative | no | — |
| `computational-proof` | Game-based proof or CryptoVerif model of the HKDF hierarchy and HMAC commitment | ledger alternative | no | — |
| `external-audit-without-model` | Third-party cryptographic audit of design and code without a mechanised model | ledger alternative | no | — |
| `combined-model-proof-and-audit` | Symbolic model plus computational argument plus external audit | ledger alternative | no | — |
| `defer-to-post-v1` | Ship v1 with documented residual risk and audit after release | defer / no change | no | — |

**Ledger exclusions:** {"candidate_id": "status-quo-internal-adversarial-review", "reason": "Internal review alone does not satisfy the routing of formal analysis to independent security review", "invariant_violated": "SPEC §25.2 L2559-2562"}; {"candidate_id": "defer-to-post-v1", "reason": "Production crypto requires an external cryptographic audit before release", "invariant_violated": "docs/crypto-review-v1.md L10-12"}.
**Exclusions proposed in this protocol (need independent sign-off, CR2):** `status-quo-draft10-kats-only`, `status-quo-single-vectors`, `zip215-cofactored`.

**R0 checklist:** 1 status quo: `status-quo-internal-adversarial-review`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 6 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: `defer-to-post-v1`; 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `status-quo-internal-adversarial-review`, `symbolic-model`, `computational-proof`, `external-audit-without-model`, `combined-model-proof-and-audit`, `defer-to-post-v1`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.
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

_Generated by `python research/tools/experiments/fill_crypto_auto_blocks.py` for EXP-CRYPTO-011; edit the inputs, not this block._
<!-- END AUTO:common -->
