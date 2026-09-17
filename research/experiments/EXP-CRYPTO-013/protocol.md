# EXP-CRYPTO-013: RFC 3161 timestamp verification profile, dependency attack surface and long-term validity

<!-- BEGIN AUTO:meta -->
<!-- END AUTO:meta -->

## 1. Question

Which TSTInfo/CMS fields must the v1 timestamp verifier constrain, and does the constrained profile reject substitution and malformed-token attacks while accepting real TSA tokens (DEC-CRY-076, DEC-CRY-098, DEC-CRY-099)? Is shipping RFC 3161 verification on the pre-release `cms` crate acceptable, or must it be gated, vendored-and-fuzzed, or replaced by a minimal parser (DEC-CRY-006)? What are the right frozen signature/timestamp limits (DEC-CRY-083), how is timestamp evidence renewed (DEC-CRY-066), and how is validity established decades later without live infrastructure (DEC-CRY-097)?

## 2. Hypotheses and falsification

- **H1 (profile).** The status-quo minimal checks (imprint, signed attributes, EKU, chain) accept at least one certificate-substitution or attribute-omission attack from the negative corpus. Adding ESS signing-certificate-v2, TSA-name match, policy-OID allowlist and critical-extension rejection reduces `benign_false_refusal_fraction` acceptably while raising `import_acceptance_fraction` on real tokens to 1.
- **H2 (dependency surface).** The `cms =0.3.0-pre.2` parser has more untrusted-input LoC and attacker-controlled parameters, and more fuzzing findings at a fixed coverage target, than a minimal profile parser limited to the frozen fields.
- **H3 (fuzzing).** A local cargo-fuzz campaign on `verify_timestamp` finds zero unresolved findings at the coverage target for the minimal profile parser; the pre-release `cms` path may not.
- **Falsification:** H1 fails if the minimal checks reject nothing extra the corpus contains and the constrained profile rejects real tokens. H3 fails if the minimal parser also has unresolved findings.

## 3. Decisions informed and evidence route

<!-- BEGIN AUTO:candidates -->
<!-- END AUTO:candidates -->

Evidence route: `EMPIRICALLY_MEASURED` accept/reject fractions over pre-registered case sets (T-20), attack-surface cost inputs (C4), and fuzzing findings (M24.2, binary at the coverage target). Long-term-validity semantics (DEC-CRY-097) and renewal (DEC-CRY-066) are largely analytical/standards comparisons; the mechanical parts (what verify reports, whether bytes are rewritten) are measured, and the trust semantics feed external review.

## 4. Candidates and arms

- **DEC-CRY-076 profile fields:** `status-quo-minimal-checks`; `require-ess-signing-certificate-v2`; `policy-oid-allowlist`; `tsa-name-match`; `reject-unknown-critical-extensions`; `accuracy-and-skew-window`; `full-rfc3161-rfc5816-profile`. Evaluated as a cumulative set and per field.
- **DEC-CRY-098 algorithms:** `status-quo-ed25519-sha256-only`; `ed25519-with-sha512`; `add-ecdsa`; `add-rsa`; `accept-null-parameters`; `separately-versioned-profile`; `gate-timestamps-unsupported`.
- **DEC-CRY-006 dependency:** `status-quo-prerelease-cms`; `pin-stable-release`; `gate-timestamps-unsupported`; `minimal-profile-parser`; `vendor-and-fuzz`; `sandboxed-parse`.
- **DEC-CRY-099 requirability/stripping:** `status-quo-optional-unrequirable`; `require-timestamp-policy`; `profile-mandated-timestamps`; `report-transcript-duplicates`; `unique-transcript-on-embed`.
- **DEC-CRY-083 limits:** `status-quo-limits` (64 KiB token, 16-cert chain, 16-byte signer id) versus alternatives, measured against real TSA token/chain-size distributions.
- **DEC-CRY-066 renewal, DEC-CRY-097 long-term, DEC-CRY-100 anchors/reference-time:** the mechanical behaviour (report at genTime, NOT_EVALUATED without anchors, whether bytes are rewritten) is measured; the trust policy is analytical/external.

Reference candidate: status quo of each decision.

## 5. Corpus selectors

- **Real tokens:** a committed corpus `research/tools/crypto/tsa_tokens/` of RFC 3161 tokens obtained by the program owner from at least six public/commercial TSAs (algorithms, digest params, chain algorithms, token sizes), each with provenance and hash. No live TSA is contacted during the run. Because network fetching is a program-owner action, this is a precondition, not an agent step.
- **Negative corpus:** `research/tools/crypto/tsa_negatives/`, generated per constrained field (substituted certificate, omitted ESS attribute, wrong TSA name, disallowed policy OID, unknown critical extension, out-of-skew genTime, malformed DER, oversized token, deep chain).
- No tuning/validation/held-out corpus items (this is CLI/parser behaviour over a fixed case set). Case lists are fixed before results (T-20).

## 6. Environment and platform requirements

`wsl-ubuntu`, Linux x86-64. `timing: false` for accept/reject; a `time_cpu` sub-run reports fuzzing throughput only. cargo-fuzz 0.13.2 and nightly-2026-08-01 are available (PROGRESS). Docker sandboxing (DEC-CRY-006 `sandboxed-parse`) is BLOCKED (Docker unavailable); a process-isolation prototype with resource limits is the substitute and is labelled.

## 7. Commands and tooling

- `ebr-crypto tsverify --profile <candidate> --token <der> --anchors <der...>`: runs the verifier at each candidate profile and prints outcome, per-field checks and exit status.
- Negative generator `research/tools/crypto/tsa_negatives/gen.py` (the committed counterexample-search protocol).
- `research/tools/crypto/dep_surface.py`: counts untrusted-input LoC, attacker-controlled parameters and native/unsafe surface of each parser candidate from `cargo tree`/`cargo metadata` and source (C4 cost inputs).
- `cargo fuzz` targets `fuzz_targets/verify_timestamp_*.rs` for the pre-release `cms`, the minimal parser and the vendored path; a fixed coverage target (line coverage percentage recorded before results) defines M24.2.

## 8. Seeds, warmup, repetitions and adaptive rule

- Accept/reject: deterministic, run once, repeat-hash of the outcome table.
- Fuzzing: fixed corpus seeds, a fixed wall budget (for example 4 CPU-hours per target) and a fixed coverage target; findings are triaged; the run stops at the budget or the coverage target.

## 9. Measurement method and metrics

| Metric (canonical) | ID | OD | Role |
|---|---|---|---|
| `import_acceptance_fraction` (real tokens accepted) | T-20 | OD-19 | **primary** |
| `benign_false_refusal_fraction` (real tokens wrongly refused) | T-20 | OD-17/OD-25 | **primary** |
| `error_actionability_fraction` (refusal messages name the field) | T-20 | OD-25 | secondary |
| negative-corpus rejection fraction | T-20-style binary (must be 1) | OD-04 | **binary screen** |
| `unresolved_fuzz_findings_at_coverage_target` | M24.2 | OD-24 | **binary** (= 0) |
| `untrusted_input_loc`, `attacker_controlled_parameters`, `decode_path_native_unsafe_count` | C4 cost inputs | OD-24 | cost |
| `rustsec_advisory_count`, `maintainer_count`, `release_age_years`, `msrv_gap` for `cms` | C3 cost inputs | OD-22 | cost |
| real TSA token/chain sizes | descriptive | — | reported (DEC-CRY-083) |

## 10. Normalization and statistical analysis

- Coverage fractions over fixed case sets use the T-20 absolute floor `0.5/n_cases`: a one-case difference exceeds the band (deterministic; exact).
- Cost inputs are counts (AGG-P, per design), reported per candidate.
- Fuzzing findings are binary at the coverage target.
- No corpus aggregation (no tuning/validation items). No sensitivity weighting (fixed case set).

## 11. Practical significance

T-20 bands and the cost tiers of §7.2 from `research/methods/thresholds.json`. Adding an algorithm or a wire field is at least T2; the minimal-parser choice trades attack surface (C4) against maintenance (C3). Ed25519-only status quo is a frozen wire fact (SHA-256 imprint); alternatives are new versions.

## 12. Sensitivity analysis

Not weight-based (fixed case sets). Reported arms: profile field subsets; anchor-set presence/absence; reference-time source (system clock, `--at-time`, genTime only); coverage-target level for the fuzzing comparison.

## 13. Expected negative results worth recording

- The status-quo minimal checks accept a certificate-substitution token, which selects the ESS/TSA-name additions.
- The pre-release `cms` crate carries a larger untrusted-input surface and at least one fuzzing finding, supporting a minimal profile parser.
- Real TSA tokens exceed 16 KiB or chains exceed a low depth, informing the DEC-CRY-083 limits.

## 14. Threats to validity

- Real-token corpus completeness (six TSAs is a sample); reported as a limitation.
- Fuzzing time budget bounds coverage; findings are a lower bound on defects.
- Docker sandboxing unavailable; the process-isolation substitute is not the shipped design.
- Trust/long-term-validity semantics (DEC-CRY-097, DEC-CRY-066) are analytical and route to external review; only mechanical behaviour is measured.

## 15. Held-out placeholder (Phase D)

Not applicable (fixed case sets, no corpus). The accept/reject and fuzzing gates are re-run at the freeze commit for the frozen builds.

## 16. Cost estimate and agent effort

- Compute: about 20 core-hours (fuzzing dominates). Disk: about 5 GB.
- Agent effort: **judgment** (profile design, negative generation, dependency-surface assessment, fuzz triage); execution **scripted**. Long-term-validity analysis is **judgment** feeding external review.

<!-- BEGIN AUTO:common -->
<!-- END AUTO:common -->
