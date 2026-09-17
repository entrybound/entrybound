# EXP-CRYPTO-016: Differential conformance of encrypted readers, structural validation, and version/deprecation gating (fuzzing)

<!-- BEGIN AUTO:meta -->
<!-- END AUTO:meta -->

## 1. Question

Do all encrypted reader paths (full encrypted open, encrypted range opener, unencrypted in-memory open, random reader, STREAM reader) accept exactly the same byte set for preamble sentinels, compat words, segment-spanning objects and Index validation, and where they diverge, should the wire specification be narrowed or the readers generalized (DEC-CON-025)? Must the range reader enforce the same structural rules as the full reader before policy decisions (DEC-CRY-065)? What deterministic rule assigns objects to segments (DEC-CRY-079)? How are unknown versions, ro_compat/compat bits, retired identifiers and unknown stanza types treated (DEC-CON-011, DEC-CON-012, DEC-CON-008, DEC-CRY-071, DEC-CRY-104)?

## 2. Hypotheses and falsification

- **H1 (reader divergence).** Differential fuzzing finds inputs accepted by one encrypted reader and rejected by another (a I15 violation) under the status quo; a shared canonical validation module removes the divergence.
- **H2 (range reader).** The status-quo range opener validates the Descriptor only when ordinal 0 is CONTROL; an adversarial PAYLOAD-first archive is accepted by the range reader but rejected by the full reader; enforcing ordinal-0 CONTROL and rank order in the range reader removes the gap.
- **H3 (version/compat gating).** Unknown major/minor versions and unknown incompat bits are refused deterministically; unknown ro_compat bits gate the specified operation set; retired-but-known identifiers decode with a DEPRECATED diagnostic (never refused, SPEC §20.4).
- **H4 (unknown stanzas).** Unknown recipient stanza types remain covered by the envelope MAC and recipient-set digest and are never stripped or reordered; unknown protection classes fail closed.
- **Falsification:** any reader-divergence input, any accepted PAYLOAD-first archive on any reader, any refused retired identifier, or any stripped/ignored unknown stanza is the committed counterexample (I15/I12/HC-15 violation, R1).

## 3. Decisions informed and evidence route

<!-- BEGIN AUTO:candidates -->
<!-- END AUTO:candidates -->

Evidence route: `EMPIRICALLY_MEASURED` differential fuzzing and conformance over a fixed case set (T-20), with the uniqueness-of-interpretation parts binary (I15/HC-03; R1). DEC-CON-008/011/012 are primarily container-cluster; the crypto-transcript/version-binding and encrypted-reader cells are covered here and cross-referenced.

## 4. Candidates and arms

- **DEC-CON-025 readers:** `status-quo-divergent`, `shared-canonical-validation-module`, `narrow-wire-spec`, `generalise-range-reader`, `differential-conformance-gate`.
- **DEC-CRY-065 range reader:** `status-quo-divergent-checks` (excluded at R1: acceptance diverges from the frozen object-order rule), `enforce-order-in-range-reader`, `shared-structural-validator`, `differential-fuzzing-gate`.
- **DEC-CRY-079 segment assignment:** `status-quo-implementation-defined`, `normative-assignment-table`, `one-object-per-segment`, `size-bounded-segments-with-vectors`.
- **DEC-CON-012 version identity:** `keep-bootstrap-identity`, `rename-to-v1-major-1`, `rename-with-dual-acceptance-window`, `minor-tolerant-readers`, `strict-version-gate-status-quo` (`negotiated-version-fallback` excluded at R1).
- **DEC-CON-011 ro_compat semantics:** `status-quo-uninterpreted`, `ext4-rewrite-forbidden`, `mutation-and-rewrite-forbidden`, `two-tier-only`, `read-write-version-pair`, `feature-name-registry-in-diagnostics`.
- **DEC-CON-008 deprecation:** `spec-text-only-status-quo`, `registry-retirement-table-with-diagnostic`, `override-recorded-in-aux`, `new-version-per-axis-change`, `independent-axis-registries` (`refuse-retired-on-read`, `remove-decode-support` excluded at R1).
- **DEC-CRY-071 stanza extension contract; DEC-CRY-104 unknown-stanza reporting:** candidates from the ledger (`strip-unknown-stanzas`, `accept-unknown-protection-class` excluded at R1).
- **DEC-CRY-032 experimental legacy encrypted byte forms:** which pre-correction Descriptor-v1 and pre-2026-08-30 wrap-AD encrypted archives v1 readers must open, for how long, and with what labelling — `status-quo-read-legacy-descriptor-v1`, `read-with-warning-until-v1`, `migration-tool`, `refuse-all-experimental-forms`, `permanent-support`. Reader acceptance of each experimental form is measured here across all reader paths (crafted pre-correction archives); the compatibility-window policy is analytical.

Reference candidate: status quo of each decision.

## 5. Corpus selectors

- **Seed corpus for differential fuzzing (tuning):** real encrypted archives of `f01-tuning-curl-8-19-0-git`, `f04-tuning-sourcelike`, `f20-tuning-generated-private-vault`, and generated multi-object/cross-segment archives; plus a hand-built conformance corpus of preamble/segment/Index variants and version/compat/retired-identifier/unknown-stanza cases (fixed list, frozen before results).
- **Independent writer:** a research writer producing multi-object segments, PAYLOAD-first archives, and unknown-stanza archives (crafted inputs the production writer will not emit).
- **Validation:** analogous validation-split archives plus the fixed case set, one registered look for graded fractions.

## 6. Environment and platform requirements

`wsl-ubuntu`, Linux x86-64. cargo-fuzz 0.13.2 and nightly-2026-08-01 (PROGRESS). `timing: false`. Docker-based differential fuzzing across containers is unavailable; all readers run in one process against the same bytes.

## 7. Commands and tooling

- `ebr-crypto readers`: opens the same bytes through every encrypted reader path (`open_encrypted_authenticated`, the range opener, in-memory open, random reader, STREAM reader) and records each acceptance/rejection and outcome class.
- `cargo fuzz` target `fuzz_targets/differential_readers.rs`: mutates seeds and asserts all readers agree (accept/reject and, on accept, identical logical interpretation); a disagreement is written to the crash corpus and committed as the artifact.
- Version/compat/deprecation case runner over the fixed case set.
- Independent structural writer `ebr-crypto craft` for PAYLOAD-first, multi-object-segment and unknown-stanza archives.

## 8. Seeds, warmup, repetitions and adaptive rule

- Fuzzing: fixed seeds, a fixed wall budget (for example 8 CPU-hours) and a fixed coverage target recorded before results; disagreements triaged.
- Fixed case set: deterministic, 2 repetitions plus repeat-hash of outcomes.

## 9. Measurement method and metrics

| Metric (canonical) | ID | OD | Role |
|---|---|---|---|
| `reader_disagreements` | HC-03 | OD-04 | **binary** (R1) |
| PAYLOAD-first / structural-rule acceptance by any reader | HC-03/HC-15 | OD-04 | **binary** (R1) |
| `ambiguity_refusal_fraction` (unknown critical features refused) | HC-07 (M19.3) | OD-04 | binary |
| retired-identifier decode success with DEPRECATED | T-20 | OD-04 | **primary** (must decode, SPEC §20.4) |
| unknown-stanza coverage under MAC (never stripped/reordered) | HC-14/I12 | OD-15 | **binary** (R1) |
| `unresolved_fuzz_findings_at_coverage_target` | M24.2 | OD-24 | **binary** (= 0) |
| version-gate correctness fraction | T-20 | OD-04 | **primary** |

## 10. Normalization and statistical analysis

- Binary I15/I12/HC metrics: any failure is R1 with the committed input.
- Fixed-case fractions: T-20 exact; AGG-C headline.
- Confirmatory W against S on validation for graded fractions; MDE gate.
- Fuzzing findings binary at the coverage target.

## 11. Practical significance

T-20 and §7.2 tiers from `research/methods/thresholds.json`. A shared validator is a T0/T1 implementation change; narrowing the wire spec or a version rename is a new-identifier change (T2-T4). Frozen-identity and SPEC §20.4 constraints enforced at R1.

## 12. Sensitivity analysis

Not weight-based. Reported arms: reader path, seed-corpus composition, coverage-target level, and version/compat case subsets. CB triggers evaluated.

## 13. Expected negative results worth recording

- Differential fuzzing finds a byte set accepted by the range reader but not the full reader (a real I15 gap driving DEC-CRY-065 and DEC-CON-025).
- The status-quo implementation-defined segment assignment produces different layouts across builds, motivating a normative assignment table (DEC-CRY-079).
- Minor-version tolerance conflicts with reserved-bit rules (DEC-CON-012).

## 14. Threats to validity

- Fuzzing coverage bounds the divergence search; findings are a lower bound.
- Single-process differential testing shares the same crate; the independent writer and clean-room parser (EXP-CRYPTO-007) mitigate shared-code blind spots.
- Container-based differential fuzzing unavailable (Docker); noted.

## 15. Held-out placeholder (Phase D)

Binary I15/I12 conformance runs on held-out on every split (R1). Graded fractions get one held-out look. No held-out item named here.

## 16. Cost estimate and agent effort

- Compute: about 30 core-hours (fuzzing dominates). Disk: about 10 GB.
- Agent effort: tooling **judgment** (differential harness, crafter, case set); execution **scripted**; analysis **judgment**.

<!-- BEGIN AUTO:common -->
<!-- END AUTO:common -->
