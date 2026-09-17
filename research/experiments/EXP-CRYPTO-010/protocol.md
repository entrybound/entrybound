# EXP-CRYPTO-010: Encrypted partial-read tamper resistance, salvage and keyless verification status (fault injection)

<!-- BEGIN AUTO:meta -->
<!-- END AUTO:meta -->

## 1. Question

When an encrypted archive is tampered — a byte flipped, a segment deleted, duplicated, reordered, truncated, or a sibling-archive segment spliced in — what verification status does each read path report, is any tampered data ever returned as verified, and what blast radius results per segment layout? What salvage guarantees hold for damaged encrypted archives (DEC-INT-033), and what should keyless verification report and exit (DEC-INT-021)? This experiment produces the fault-injection corpus and measured outcomes that the segment-construction and partial-read questions (DEC-CRY-080, DEC-CRY-016) hand to external review.

## 2. Hypotheses and falsification

- **H1 (no false verified).** For every injected fault and every read path (full open, range open, list, verify, unpack, salvage), no plaintext byte reaches the caller as verified unless the bytes actually authenticate (MVT-14(b), MVT-15(a)). `undetected_tampers` = 0 and the reported verification state never exceeds the oracle state.
- **H2 (fault-class fidelity).** Truncation, corruption, unsupported and nonconforming faults map to the SPEC §20.5 classes per the MVT-15(b) precedence table, through to CLI exit status.
- **H3 (blast radius by layout).** `blast_logical_bytes_p95` is lower for one-object-per-segment than for batched or fixed-size-padded segments (fewer entries share a segment). The worst region stratum (Index, ArchiveFinal) is the guarded case.
- **H4 (salvage).** `authenticated-record-recovery` recovers exactly the records whose AEAD authenticates and never splices or truncates into misleading output; `require-intact-envelope` recovers a superset only when the envelope is intact.
- **Falsification:** any false-verified byte (H1) is an R1 violation with the archive bytes as the committed artifact. H2 fails on any off-diagonal class. H3/H4 are directional verdicts on validation.

## 3. Decisions informed and evidence route

<!-- BEGIN AUTO:candidates -->
<!-- END AUTO:candidates -->

Evidence route: `EMPIRICALLY_MEASURED` fault outcomes and blast radius; the "no false verified" and fault-class parts are binary HC tests (HC-14, HC-15; R1). The formal splicing/truncation resistance of the segment construction (DEC-CRY-080) and partial-read guarantees (DEC-CRY-016) require external formal analysis; this experiment supplies the empirical fault corpus and results for the dossier (EXP-CRYPTO-022).

## 4. Candidates and arms

- **Segment layouts (DEC-CRY-079, DEC-CRY-081):** L0 one-object-per-segment, L1 batched, L2 fixed-size-padded, L3 group-by-chunkgroup, L4 dummy-segments.
- **DEC-INT-033 salvage:** `fail-closed-no-salvage`; `authenticated-record-recovery`; `require-intact-envelope`; `public-framing-recovery-only`; `parity-required-for-encrypted` (external parity is out of scope; measured as "no salvage without parity").
- **DEC-INT-021 keyless verify:** `status-quo-exit-zero-label`; `distinct-exit-code`; `require-public-only-flag`; `refuse-without-unlock`; `json-qualifier-exit-zero`; `policy-flag-require-full`. (Exit-code semantics also cross-ref the ecosystem exit-code contract.)
- **DEC-INT-010 partial-read status:** `status-quo-per-object-plus-archivefinal`; `verify-touched-segment-ends`; `remote-streaming-whole-verify` (cross-ref EXP-CRYPTO-008); `full-download-only`.
- **DEC-CRY-105 unlock-on-failure:** `status-quo-hard-fail`; `continue-then-report`; `evaluate-all-require-unique`; `continue-with-tamper-warning` (measured for whether any releases plaintext).
- **DEC-CRY-080 segment binding:** status-quo SegmentEnd/ArchiveFinal versus alternatives, measured on the truncation/reorder/splice corpus (the formal comparison is external).

Reference candidate: status quo of each decision.

## 5. Corpus selectors

- **Fault targets (tuning):** encrypted packs of `f01-tuning-curl-8-19-0-git`, `f04-tuning-maildir`, `f07-tuning-mnist-npz`, `f09-alpine-324-rootfs-binaries`, `f11-alpine-324-apks`, `f13-tuning-nasa-ntrs-pdf-small`, `f18-tuning-zstd-releases-1-5-x`, plus `f20-tuning-generated-private-vault` (adversarial encrypted structures) and generated multi-segment archives with controlled object counts. Sibling archives (same content, `key add`) provide splice donors.
- **Injection strata (objective OD-14, Appendix C.1):** at least 200 single-byte corruptions per region stratum per archive (payload, Index, metadata/manifest, dictionaries, integrity/footer), plus 512 B / 64 KiB / 1 MiB overwrites and truncation at section boundaries and 1,000 seeded offsets; plus structural operations: segment delete, duplicate, reorder, cross-archive splice, stanza insertion/removal.
- **Validation:** analogous validation-split items, one registered look.

## 6. Environment and platform requirements

`wsl-ubuntu`, `timing: false`. Fault outcomes and blast radius are deterministic given the seed and archive. Range-read faults use `ebr-origin` to serve tampered bytes for the misbehaving-origin cases (HC-18 cross-ref).

## 7. Commands and tooling

- Fault injector `research/tools/crypto/fault_inject.py` (or `ebr-crypto fault`): applies the strata to a real encrypted archive, records each injection descriptor, and runs each read path (`ebound list/verify/unpack/read`, `ebr-access` range mode, `ebr-verify`), capturing outcome class, exit status, verified state and returned bytes.
- Oracle: for each injection the strongest supportable state is known by construction; a research-internals byte-verification log (F-24) cross-checks per MVT-15(a).
- Blast-radius scorer `research/tools/crypto/blast_radius.py` computing `blast_logical_bytes_p95/max` and `blast_entries_p95` per stratum with the equal 0.2 stratum weights and the worst-region guard.

## 8. Seeds, warmup, repetitions and adaptive rule

- Deterministic: 2 repetitions plus repeat-hash of outcome records. Injection seeds in the spec.
- Exhaustive where the MVT requires (section-boundary truncation, framing/header/stanza bit flips); the 1,000 seeded offsets and 10,000 payload flips are regression evidence (detection bound about 95% at 0.3% defect rate, §D.5).

## 9. Measurement method and metrics

| Metric (canonical) | ID | OD | Role |
|---|---|---|---|
| `undetected_tampers` | HC-14 | OD-15 | **binary** (R1) |
| verification state ≤ oracle | HC-15 | OD-04/OD-15 | **binary** (R1) |
| fault-class confusion matrix (off-diagonal count) | HC-15 | OD-04 | **binary** (R1) |
| `blast_logical_bytes_p95`, `blast_logical_bytes_max`, `blast_entries_p95` | T-15 | OD-14 | **primary** (layout comparison; worst-region guard) |
| `localization_precision` / `localization_recall` | T-20 / HC-15 | OD-14 | primary / binary |
| `truncation_salvage_fraction` | T-20 | OD-14 | primary (salvage) |
| `origin_protocol_safety` | HC-18 | OD-12 | binary (range faults) |
| keyless verify exit code and label | descriptive | OD-11 | reported per candidate (DEC-INT-021) |

## 10. Normalization and statistical analysis

- Binary HC metrics: any failure is an R1 elimination with the committed artifact; no statistics.
- Blast radius: p95 and max per stratum per item, equal 0.2 stratum weights for the headline, worst-stratum guard for R4/R8; then AGG-G across families plus AGG-W. Aggregation and intervals per §4.9-§4.10.
- Confirmatory W against S on validation for blast radius and salvage fraction; MDE gate.
- Keyless verify: a census of (exit code, label) across candidates and a cross-check against the ecosystem exit-code contract; no verdict.

## 11. Practical significance

T-15 (band 0.5, floors 64 KiB / 0.5 entry), T-20 bands from `research/methods/thresholds.json`. Detection must be 100% (I21, I25). A layout change is a policy/writer change costed by the assessor (§7.2); a new authenticated wire structure (segment digests, Merkle) is T4 and out of scope for crypto-v1.

## 12. Sensitivity analysis

§6 arms plus: injection stratum weighting (equal versus byte-proportional, the latter a sensitivity arm), object count per segment, and salvage with and without keys.

## 13. Expected negative results worth recording

- Fixed-size-padded segments (L2) widen blast radius because unrelated entries share a segment; a privacy mitigation that costs corruption locality.
- Continue-on-failure unlock (DEC-CRY-105) enables an insider-poisoning oracle unless it requires a unique AFK across stanzas.
- Salvage of authenticated records is safe only with a splice check; without it, recovered records can be reordered into misleading output.

## 14. Threats to validity

- Regression-evidence strata give a detection bound, not a proof; the exhaustive strata carry the hard guarantee.
- Sibling-splice donors depend on the mutation contract (EXP-CRYPTO-015); the corpus is regenerated if that contract changes.
- Formal splicing resistance is external; this experiment cannot establish it.

## 15. Held-out placeholder (Phase D)

Binary HC fault tests run on held-out encrypted items on every split (R1). Blast-radius and salvage confirmatory verdicts run once on held-out after unlock. No held-out item named here.

## 16. Cost estimate and agent effort

- Compute: about 120 core-hours (hundreds of thousands of injection-and-read cycles). Disk: about 30 GB transient.
- Agent effort: tooling **judgment** (injector, oracle, scorer); execution **scripted**; analysis **judgment**.

<!-- BEGIN AUTO:common -->
<!-- END AUTO:common -->
