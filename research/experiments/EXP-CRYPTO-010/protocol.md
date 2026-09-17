# EXP-CRYPTO-010: Encrypted partial-read tamper resistance, salvage and keyless verification status (fault injection)

<!-- BEGIN AUTO:meta -->
| Field | Value |
|---|---|
| Index status | **NEEDS_TOOLING** |
| Kind | decision |
| Domain | crypto (program §22.1, §22.4, §22.5) |
| Decisions informed | DEC-INT-010, DEC-INT-021, DEC-INT-033, DEC-CRY-105, DEC-CRY-080, DEC-CRY-065, DEC-CRY-016 |
| Platforms | Linux/x86-64 |
| Requires timing | False |
| Estimates | 120 machine-hours; 30 GB disk |
| Tooling to build | fault_inject.py; blast_radius.py; byte-verification log (F-24); kill injector |
| Environment prerequisites | none beyond the harness and the ebr venv |
| Blocked arms | none |
| Specs | `spec.yaml` |
| Validation looks | owns look 1 for DEC-CRY-016, DEC-CRY-065, DEC-CRY-080; participates in looks owned by other experiments for DEC-CRY-105, DEC-INT-010, DEC-INT-021, DEC-INT-033 |
| Gates | G-A and G-B unmet at this revision (generated `gate_state` column of `research/experiments/index.csv`); timing runs also need a PASS calibration record for the calibration identity (PA-04) |
| Conventions | `research/experiments/_index/crypto-conventions.md` (CR0-CR10); program amendments `research/experiments/_program/design-revision-round1.md` |
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
#### DEC-INT-010 — What verification status is reported for encrypted random reads that do not verify segment END and the full segment-sequence digest, and is incremental remote whole-archive verification (PCR/PCI via ranges) required in v1?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-008, EXP-CRYPTO-021, EXP-INT-016, EXP-INT-020, EXP-REMOTE-010.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-per-object-plus-archivefinal` | Status quo: per-object AEAD plus authenticated ArchiveFinal and Descriptor/Manifest binding; PCR DeclaredNotFullyVerified; PCI NotComputed; no remote whole verification | status quo | yes | EXP-CRYPTO-008, EXP-INT-016, EXP-REMOTE-010 |
| `verify-touched-segment-ends` | Also verify END records of every touched segment and report segment-level status | ledger alternative | yes | EXP-CRYPTO-008, EXP-INT-016, EXP-REMOTE-010 |
| `per-segment-digests-in-archivefinal` | Bind per-segment digests usable by partial readers into ArchiveFinal | ledger alternative | no | EXP-CRYPTO-008, EXP-INT-016, EXP-REMOTE-010 |
| `remote-streaming-whole-verify` | Remote whole-archive verification fetching all ranges with bounded memory, computing PCR and PCI | ledger alternative | yes | EXP-CRYPTO-008, EXP-INT-016, EXP-REMOTE-010 |
| `full-download-only` | Whole-archive verification only after full download | ledger alternative | yes | EXP-CRYPTO-008, EXP-INT-016, EXP-REMOTE-010 |
| `merkle-over-segments` | Authenticated Merkle tree over segment digests enabling logarithmic partial proofs | ledger alternative | no | EXP-CRYPTO-008, EXP-INT-016, EXP-REMOTE-010 |

**Ledger exclusions:** {"candidate_id": "per-segment-digests-in-archivefinal", "reason": "Changes ArchiveFinalV1 fields; only possible in a new crypto wire version", "invariant_violated": "crypto-v1 wire frozen"}; {"candidate_id": "merkle-over-segments", "reason": "Requires new authenticated wire structures; only possible in a new crypto wire version", "invariant_violated": "crypto-v1 wire frozen"}.

**R0 checklist:** 1 status quo: `status-quo-per-object-plus-archivefinal`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 6 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-INT-021 — Should verify of an encrypted archive without unlock material exit 0 with 'PRIVATE CONTENT UNVERIFIED', exit with a distinct status, or refuse, and is keyless public verification a named level with defined guarantees for custodians?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-022, EXP-ECO-013, EXP-ECO-015, EXP-ECO-016, EXP-ECO-019, EXP-INT-003.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-exit-zero-label` | Status quo: exit 0 with PRIVATE CONTENT UNVERIFIED text | status quo | yes | EXP-INT-003 |
| `distinct-exit-code` | Dedicated non-zero exit code for OK-but-unverified | ledger alternative | yes | EXP-ECO-016, EXP-INT-003 |
| `require-public-only-flag` | Exit 0 only with explicit --public-only; otherwise refuse with usage/policy error | ledger alternative | yes | EXP-INT-003 |
| `refuse-without-unlock` | verify refuses encrypted archives without unlock material | ledger alternative | yes | EXP-INT-003 |
| `json-qualifier-exit-zero` | Exit 0 with machine-readable UNVERIFIED qualifier in JSON output | ledger alternative | yes | EXP-INT-003 |
| `policy-flag-require-full` | --require-full-verification converts keyless result into failure; default unchanged | ledger alternative | yes | EXP-INT-003 |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: `status-quo-exit-zero-label`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 6 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-INT-033 — What salvage guarantees apply to damaged encrypted archives: fail closed, recover individually authenticated records with keys, require intact commitment and envelope, or rely on external parity?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-021, EXP-INT-016.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `fail-closed-no-salvage` | No salvage for encrypted archives | ledger alternative | yes | EXP-INT-016 |
| `authenticated-record-recovery` | With keys, recover records whose AEAD authenticates, reporting archive unverified | ledger alternative | yes | EXP-INT-016 |
| `require-intact-envelope` | Salvage only when commitment, envelope and recipient directory are intact | ledger alternative | yes | EXP-INT-016 |
| `parity-required-for-encrypted` | Recommend or require external parity for encrypted archives | ledger alternative | yes | EXP-INT-016 |
| `public-framing-recovery-only` | Recover and report only public framing information | ledger alternative | yes | EXP-INT-016 |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: no ledger id marked status quo; the critic pass confirms which candidate is the behaviour at `9e44608`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 5 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-CRY-105 — When a stanza opens but the candidate AFK fails commitment or envelope MAC, should unlock hard-fail (status quo), continue to remaining stanzas, or evaluate all stanzas before deciding?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-015, EXP-CRYPTO-021, EXP-CRYPTO-023.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-hard-fail` | Status quo: integrity failure at the first commitment or MAC mismatch | status quo | yes | EXP-CRYPTO-023 |
| `continue-then-report` | Continue to remaining stanzas; report integrity failure only if none succeed | ledger alternative | yes | EXP-CRYPTO-023 |
| `evaluate-all-require-unique` | Evaluate all matching stanzas and require one consistent AFK | ledger alternative | yes | EXP-CRYPTO-023 |
| `continue-with-tamper-warning` | Continue and surface a tamper warning if another stanza succeeds | ledger alternative | yes | EXP-CRYPTO-023 |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: `status-quo-hard-fail`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 4 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-CRY-080 — Do segment ID binding, monotonic counters, mandatory SegmentEnd and terminal ArchiveFinal/footer binding defeat truncation, reordering and cross-archive splicing, as established by formal analysis rather than internal adversarial review?

Ledger status `EXTERNAL_REVIEW_REQUIRED`, blocker class `EXTERNAL_REVIEW`. Other experiments informing it: EXP-CRYPTO-021, EXP-INT-016, EXP-CRYPTO-023.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-segmentend-archivefinal` | Status quo: segment-bound keys and nonces, SegmentEnd, terminal ArchiveFinal and footer | status quo | no | EXP-INT-016 |
| `stream-construction` | Hoang-Reyhanitabar-Rogaway-Vizar STREAM online AEAD | ledger alternative | no | EXP-INT-016 |
| `final-flag-in-nonce` | age/Tink style final-chunk flag in nonce | ledger alternative | no | EXP-INT-016 |
| `merkle-over-segments` | Merkle tree over segment digests | ledger alternative | no | EXP-INT-016 |
| `per-record-counter-only` | Per-record counters without final markers | ledger alternative | no | EXP-INT-016 |

**Ledger exclusions:** {"candidate_id": "per-record-counter-only", "reason": "Truncation after a valid record would be accepted (AR-07)", "invariant_violated": "I21"}.

**R0 checklist:** 1 status quo: `status-quo-segmentend-archivefinal`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 5 ledger candidates listed above; 4 strongest incumbent: `final-flag-in-nonce`; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-CRY-065 — Must the encrypted range reader enforce the same structural rules as the full reader (CONTROL Descriptor at ordinal 0, CONTROL object rank order, per-stanza size limits) before policy decisions?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CONF-004, EXP-CONF-010, EXP-CRYPTO-016.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-divergent-checks` | Status quo: range opener checks Descriptor only when ordinal 0 is CONTROL | status quo | no | EXP-CRYPTO-016 |
| `enforce-order-in-range-reader` | Add ordinal-0 CONTROL Descriptor and rank-order checks to the range reader | ledger alternative | no | EXP-CRYPTO-016 |
| `shared-structural-validator` | Factor one structural validator used by full, range and public-inspection readers | ledger alternative | no | EXP-CRYPTO-016 |
| `differential-fuzzing-gate` | Keep separate code but gate on differential fuzzing | ledger alternative | no | EXP-CONF-010, EXP-CRYPTO-016 |

**Ledger exclusions:** {"candidate_id": "status-quo-divergent-checks", "reason": "Reader acceptance diverges from the frozen object-order rule", "invariant_violated": "I15; docs/crypto-wire-v1.md L515-520"}.

**R0 checklist:** 1 status quo: `status-quo-divergent-checks`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 4 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-CRY-016 — Who performs the independent cryptographic and implementation review of crypto v1 (AD and splice binding, END/ArchiveFinal truncation, partial-read guarantees, PHTE instantiation, T1/EBPO/EBCS grammars), with what scope, and does v1 encryption ship only after it?

Ledger status `EXTERNAL_REVIEW_REQUIRED`, blocker class `EXTERNAL_REVIEW`. Other experiments informing it: EXP-CONF-009, EXP-CONF-012, EXP-CONF-013, EXP-CRYPTO-021.

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
<!-- END AUTO:candidates -->

Evidence route: `EMPIRICALLY_MEASURED` fault outcomes and blast radius; the "no false verified" and fault-class parts are binary HC tests (HC-14, HC-15; R1). The formal splicing/truncation resistance of the segment construction (DEC-CRY-080) and partial-read guarantees (DEC-CRY-016) require external formal analysis; this experiment supplies the empirical fault corpus and results for the dossier (EXP-CRYPTO-021).

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

_Generated by `python research/tools/experiments/fill_crypto_auto_blocks.py` for EXP-CRYPTO-010; edit the inputs, not this block._
<!-- END AUTO:common -->
