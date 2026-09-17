# EXP-CONF-012 — Clean-room reproduction of permanent vectors and cross-implementation checks of primitives, identity roots and canonical encodings

| Field | Value |
|---|---|
| Status | `NEEDS_TOOLING` (CC§2): clean-room vector reproducers (Python standard library; Go with pinned modules), pinned vector sets (Wycheproof, Ed25519 speccheck, RFC 5869/4231/9106 KATs, X-Wing draft-10 KATs), interop probe crate (`x-wing` production crate versus `libcrux-kem`), X-Wing C reference build, JCS reference, comparison driver |
| Domain / program sections | conformance / §28 (primary: independent reproduction), §29 (negative vectors into the corpus); crypto construction review stays EXTERNAL (crypto domain) |
| Decision IDs informed | DEC-CRY-014, DEC-CRY-016, DEC-CRY-017, DEC-CRY-020, DEC-CRY-021, DEC-CRY-051, DEC-CRY-079, DEC-CRY-106, DEC-INT-024, DEC-MOD-027, DEC-MOD-034, DEC-CMP-005, DEC-LEG-018, DEC-LEG-048, DEC-ECO-010, DEC-PLT-050 |
| HC oracles | HC-14 MVT-14(a) (vector gate), MVT-14(e) scalar-reference boundaries; HC-16 MVT-16(a); HC-12 MVT-12(a)/(c) for identity, Merkle, boundary and canonical-JSON rules; proposal for the assigner: `hc_ids` HC-10, HC-12, HC-14, HC-16; `od_ids` OD-15, OD-21 |
| Decision type | not assigned (gate G-A); construction correctness claims are `EXTERNAL_REVIEW_REQUIRED` and are not made here |
| Method / pre-registration | `decision-method.md` @ `14b977c`; design pre-registration (CC§3) |
| Author / exposure | CC§1 |
| Depends on | clean-room snapshot of EXP-CONF-001 §5; EXP-CONF-004 F-OPS archives (V2, V7); EXP-CONF-006 historical binaries (V8 receipt reproduction across builds) |

## 1. Question

Can sessions without access to Entrybound source reproduce every permanent vector and every identity, Merkle, chunk
boundary, keyed-boundary, segment-assignment and canonical-JSON byte string from the written documents alone; do
independent implementations of the primitives the documents pin agree with production on full standard and adversarial
vector sets; and where they disagree, is the disagreement a production defect, an implementation's documented profile,
or underdetermined text?

## 2. Hypotheses and falsification

- **H1 (MVT-16(a), MVT-12(c)).** A clean-room reproducer regenerates every vector line of `docs/descriptor-vectors-v1.txt`,
  `docs/posix-metadata-v1-vectors.txt`, `docs/security-metadata-v1-vectors.txt` and `docs/crypto-wire-v1-vectors.txt`
  (V1-V8) byte for byte. *Falsified* by one mismatch not attributed by triage (CC§6) to the reproducer.
- **H2 (identity and Merkle roots).** Clean-room LAI/PCR/AUX/PCI and Merkle roots equal production's on every F-OPS archive
  and on the Merkle leaf counts 0..1,025. *Falsified* by one difference.
- **H3 (boundaries; MVT-14(e) reference).** Clean-room scalar `gear-norm-v1` boundaries, keyed Gear-table derivation, PHTE
  rejection sampling and AES-keyed boundaries equal production's on every input of §4 V3, including empty and tail
  inputs, and rolling equals direct evaluation. *Falsified* by one difference.
- **H4 (primitive KATs; MVT-14(a)).** Production passes every vector of the full RFC 5869, RFC 4231 and RFC 9106 sets,
  the Wycheproof sets for HKDF, HMAC-SHA256, AES-GCM-SIV, X25519 and Ed25519, and the X-Wing draft-10 KATs. *Falsified*
  by one failure.
- **H5 (Ed25519 profile; DEC-CRY-021).** Production's accept/reject decisions on the speccheck and Wycheproof EdDSA cases
  match the rule documented in `docs/crypto-suite-v1.md` (strict verification rejecting noncanonical encodings,
  small-order points, invalid scalars). *Falsified* by one case where production contradicts the written rule.
- **H6 (X-Wing interop; DEC-CRY-106).** Encapsulations by each implementation decapsulate identically in every other, and
  malformed-key and implicit-rejection cases behave identically. *Falsified* by one disagreement.
- **H7 (segment assignment; DEC-CRY-079).** A clean-room implementation of the CONTROL/PAYLOAD assignment rule predicts
  production's public segment layout for every generated encrypted archive. *Falsified* by one misprediction
  (the rule is then underdetermined by text or production deviates).
- **H8 (canonical JSON; DEC-LEG-048).** A clean-room implementation of the documented receipt and migration-report
  grammars reproduces production bytes from receipt fields; RFC 8785 conformance of current bytes is measured (not
  hypothesized).

## 3. Arms

| Arm | Implementations | Inputs | Output |
|---|---|---|---|
| V1 vectors | clean-room Python 3.12 standard library (`vec_py`) | the four vector files | per-line match |
| V2 identity and Merkle | clean-room Python (`ident_py`); production (`ebound inspect --json`) | EXP-CONF-004 F-OPS archives; synthetic leaf lists n = 0..1,025 | per-archive and per-n match; brute-force search for leaf/node encoding collisions of the named-domain construction over all trees with n ≤ 16 leaves drawn from a 4-symbol leaf alphabet (structural preimage collisions, not hash collisions) |
| V3 boundaries | clean-room Python and Go scalar references (`chunk_py`, `chunk_go`); production via `ebr-chunk` boundaries subcommand | `f20-tuning-gear-extremes`, `f20-tuning-csprng`, `f01-tuning-zstd-v1-5-7-git` (tuning), generated inputs of 0, 1, min−1, min, target, max, max+1 bytes and 1 MiB random (`random-v1`, seed 2809171201); 1,000 seeded distinct file keys for keyed derivations | boundary-set equality per input and key; rolling-versus-direct property |
| V4 crypto wire vectors and KATs | clean-room Go (`crypto_go`: `golang.org/x/crypto` HKDF and Argon2, `github.com/google/tink/go` AES-GCM-SIV, `filippo.io/mlkem768` plus X25519 for the X-Wing combiner written from draft-10 text); production through the research harness | V1-V8 from `docs/crypto-wire-v1-vectors.txt` (independent V7 regeneration); pinned RFC 5869, RFC 4231, RFC 9106 test vectors; pinned Wycheproof commit sets | per-vector pass/fail for both |
| V5 Ed25519 profile matrix | production (`ed25519-dalek` via harness probe), Go 1.22.2 `crypto/ed25519`, PyNaCl (libsodium) pinned, Python `cryptography` (OpenSSL 3.0.13), Node 22 `crypto`, Java 21 EdDSA | ed25519-speccheck cases (pinned commit), Wycheproof `eddsa_test.json` | accept/reject table per implementation; mapping to the written rule and to named profiles (RFC 8032 cofactorless canonical, ZIP-215, libsodium) |
| V6 X-Wing interop | production `x-wing` crate, `libcrux-kem` (Rust, pinned), X-Wing draft-10 C reference (pinned commit, built with the system C compiler), `crypto_go` combiner | 1,000 seeded key pairs; malformed encapsulation keys and ciphertexts (wrong length, non-canonical encodings, all-zero, implicit-rejection cases) | cross-decapsulation matrix; KAT provenance (URL, commit, SHA-256) |
| V7 segment assignment | clean-room Python (`seg_py`) written from `docs/crypto-wire-v1.md`; production encrypted packs | 200 seeded generated trees (entry counts 1..10^4, sizes log-uniform) packed with the public test password | predicted versus observed public segment counts, sizes and object ordinals |
| V8 canonical JSON | clean-room Python (`json_py`) from `docs/compressed-tar-export-v1.md` and `docs/migration-workflows-v1.md`; pinned RFC 8785 reference (`json-canonicalization` Python reference); production `ebound` at `ebound-baseline`, `ebound-head` and EXP-CONF-006 historical builds | receipts and migration reports produced for §4 inputs plus Unicode and control-character fixtures | byte equality of reproduction; JCS equality of current bytes; cross-build receipt stability |
| V9 mixed-encoding order (DEC-CRY-020) | clean-room Python (`order_py`) implementing both the EBCS sort key and canonical LogicalPath order | exhaustive path sets over a 3-symbol component alphabet, components of length ≤ 3, two declared encodings, depth ≤ 2 | count of disagreeing pairs; affected existing encrypted vectors |
| V10 timestamps (DEC-PLT-050) | `vec_py` | Timestamp records for i64 seconds at −2^63, −1, 0, pre-1601, post-2446, 2^63−1 and nanoseconds 0, 999,999,999, 10^9 | canonical validity per value against production's acceptance |

Clean-room arms (V1, V2, V3, V4 combiner, V7, V8, V9, V10) are built by sessions under the EXP-CONF-001 §6 provenance
controls (CC§11) that read only the snapshot, the vector files and the pinned third-party documentation; they never read
`tools/crypto-vector-helper` (a same-project reference) or production source. Each keeps an ambiguity log in
`ebr.ambiguity.v1`.

## 4. Candidates

Implementations are the compared objects. Design candidates of the informed decisions are the ledger's (CC§1: none
added); for example DEC-CRY-014 `full-rfc-sets`, `plus-wycheproof-negative`, `third-party-reproduction` are evidenced by
V4; DEC-CRY-021 candidates are the named profiles of V5; DEC-CRY-106 `kats-plus-one-independent` and
`kats-plus-two-independent-and-review` by V6 (review remains external); DEC-INT-024 and DEC-MOD-034 `status-quo` versus
`rfc9162-exact`/`rfc6962-byte-prefix` by V2 (the byte-prefix alternatives are computed by `ident_py` as alternative
constructions over the same leaves so that their roots and proof sizes are reported beside the status quo); DEC-LEG-048
`rfc-8785-jcs` by V8.

## 5. Environment and commands

WSL2 Ubuntu 24.04 x86-64; Python 3.12.3 (a dedicated venv `/root/eb-research/venv-vectors` with pinned PyNaCl and
`cryptography`); Go 1.22.2; Node 22; Java 21; Rust `1.98.1` for probes. Network once for pinned downloads (Wycheproof,
speccheck, KAT files, Go modules, crates, C reference). No timing.

```sh
bash research/conformance/vectors/fetch_pinned.sh --lock research/conformance/vectors/pins-v1.json   # records URL, commit, SHA-256
cd /mnt/d/Projects/entrybound/entrybound && export PYTHONPATH=research/tools
/root/eb-research/venv/bin/python -m ebr validate --spec research/experiments/EXP-CONF-012/spec.yaml
/root/eb-research/venv/bin/python -m ebr run --spec research/experiments/EXP-CONF-012/spec.yaml
python3 research/conformance/vectors/tables.py --exp EXP-CONF-012
```

## 6. Seeds and replication

Seeds: V3 inputs `2809171201`, keys `2809171202`; V6 key pairs `2809171203`; V7 trees `2809171204`. Every arm runs
twice (deterministic, §4.13 repeat-hash of the result tables). Argon2id KATs run at the vector's parameters only.

## 7. Metrics (CC§9)

| Metric | Role | Unit | Arms |
|---|---|---|---|
| `cleanroom_pass_fraction` over baseline-scope vectors (descriptor, POSIX and security metadata, identity, Merkle, `gear-norm-v1` boundaries) | binary (HC-12) | fraction | V1, V2, V3 |
| `unspecified_baseline_decode_steps` contributions (rules that the clean-room arms could not implement from text) | binary (HC-12) | count | V1-V3, V7-V10 |
| MVT-14(a) vector failures of production; MVT-16(a) vector mismatches | binary (HC oracles) | count | V4; V1 |
| Accept/reject disagreements per implementation pair; profile mapping; cross-decapsulation disagreements; segment mispredictions; JCS inequalities; order disagreements | descriptive, non-canonical | count | V5-V9 |

## 8. Analysis

Deterministic tables; every mismatch is triaged (CC§6) into reproducer bug, production defect, third-party profile
difference or underdetermined text. No statistics. Crypto results are internal conformance evidence for the external
review dossier (DEC-CRY-016, DEC-CRY-017); they never establish construction security (§8.3). The analysis plan needs
adoption by a non-exposed session (CC§1).

## 9. Practical-significance thresholds

§3.3 for binary checks; descriptive counts have no band.

## 10. Sensitivity analysis

- V5 with Wycheproof only versus speccheck only.
- V2 with and without the alternative byte-prefix constructions.
- V8 across `ebound-baseline`, `ebound-head` and historical builds (receipt byte stability).

## 11. Split discipline and held-out placeholder

Only tuning items (V3) and generated inputs are used; vectors are specification artifacts. No validation or held-out use.

## 12. Expected negative results worth recording

- Vector fields unlabelled in the files (REQ-ECO-0039) forcing reproducers to guess field boundaries.
- Implementations differing on Ed25519 edge cases (expected across profiles), and whether production's behaviour matches
  its written rule.
- Segment assignment not determined by text (DEC-CRY-079 status quo "implementation-defined").
- Receipt bytes that are not RFC 8785 canonical.
- EBCS order and LogicalPath order disagreeing for mixed encodings (DEC-CRY-020).

## 13. Threats to validity

- Clean-room sessions share a base model with production authors (CC§11); crypto vector reproduction can succeed from
  prior knowledge of the standards, which is legitimate for standard primitives but weak evidence for Entrybound-specific
  transcripts.
- Third-party libraries have their own defects; triage attributes them before any production claim.
- Pinned vector sets may lag newer errata (FIPS 203 errata for ML-KEM is assessed in the crypto domain).
- The structural collision search in V2 is bounded (n ≤ 16, 4-symbol alphabet); it is a counterexample search, not a proof.

## 14. Cost estimates

| Item | Estimate |
|---|---|
| Compute | 12 CPU-hours (V3 keyed derivations over 1,000 keys × inputs ≈ 6 h; V2 structural search ≈ 3 h; others ≈ 3 h; ×2 repetitions included) |
| Disk | 5 GB |
| Agent effort | **judgment, high**: clean-room reproducer sessions (V1-V4 combiner, V7-V10: 3-5 sessions), triage (1 session); V5, V6 scripted |
| Platform | Linux x86-64 (WSL); network once for pinned downloads; no timing |

## 15. Outcome obligations

`outcome.json` (§10.1); negative vectors (V5 rejected cases, V6 malformed cases, V10 invalid timestamps) become cases in
`ebcc-v1.1` families F-SIG, F-CRYPTO and F-EAM with expectations from the written rules; reproducer ambiguity entries are
merged into the EXP-CONF-001 ambiguity statistics as a separate arm.
