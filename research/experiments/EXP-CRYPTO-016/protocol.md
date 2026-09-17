# EXP-CRYPTO-016: Differential conformance of encrypted readers, structural validation, and version/deprecation gating (fuzzing)

<!-- BEGIN AUTO:meta -->
| Field | Value |
|---|---|
| Index status | **NEEDS_TOOLING** |
| Kind | decision |
| Domain | crypto (program §22.1, §22.5) |
| Decisions informed | DEC-CON-025, DEC-CRY-065, DEC-CRY-079, DEC-CON-012, DEC-CON-011, DEC-CON-008, DEC-CRY-071, DEC-CRY-104, DEC-CRY-032 |
| Platforms | Linux/x86-64 |
| Requires timing | False |
| Estimates | 30 machine-hours; 10 GB disk |
| Tooling to build | ebr-crypto readers; differential_readers fuzz target; ebr-crypto craft |
| Environment prerequisites | none beyond the harness and the ebr venv |
| Blocked arms | none |
| Specs | `spec.yaml` |
| Validation looks | owns look 1 for DEC-CON-008, DEC-CON-011, DEC-CRY-032, DEC-CRY-071, DEC-CRY-079; participates in looks owned by other experiments for DEC-CON-012, DEC-CON-025, DEC-CRY-065, DEC-CRY-104 |
| Gates | G-A and G-B unmet at this revision (generated `gate_state` column of `research/experiments/index.csv`); timing runs also need a PASS calibration record for the calibration identity (PA-04) |
| Conventions | `research/experiments/_index/crypto-conventions.md` (CR0-CR10); program amendments `research/experiments/_program/design-revision-round1.md` |
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
#### DEC-CON-025 — Must every reader path (full encrypted open, encrypted range opener, unencrypted in-memory open, random reader, STREAM reader) accept exactly the same byte set for preamble sentinels, compat words, segment-spanning objects and Index validation, and where they diverge should the wire specification be narrowed or the readers generalised?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CONF-002, EXP-CONF-004, EXP-CONF-010.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-divergent` | Readers keep separate validation with differing acceptance | status quo | yes | — |
| `shared-canonical-validation-module` | One shared preamble, footer and Index validation module for all openers | ledger alternative | yes | — |
| `narrow-wire-spec` | Narrow the wire spec to one object per segment and first counter 0 | SPEC design | yes | — |
| `generalise-range-reader` | Range opener supports segment-spanning objects and full sentinel checks | ledger alternative | yes | — |
| `differential-conformance-gate` | Keep implementations separate but gate releases on a differential corpus | ledger alternative | yes | EXP-CONF-010 |

**Ledger exclusions:** {"candidate_id": "status-quo-divergent", "reason": "Different readers of one implementation accept different byte sets, so interpretation is not unique", "invariant_violated": "I15"}.
**Exclusions proposed in this protocol (need independent sign-off, CR2):** `accept-unknown-protection-class`, `negotiated-version-fallback`, `remove-decode-support`, `status-quo-divergent-checks`.

**R0 checklist:** 1 status quo: `status-quo-divergent`; 2 SPEC design: `narrow-wire-spec`; 3 ledger alternatives: 5 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-CRY-065 — Must the encrypted range reader enforce the same structural rules as the full reader (CONTROL Descriptor at ordinal 0, CONTROL object rank order, per-stanza size limits) before policy decisions?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CONF-004, EXP-CONF-010, EXP-CRYPTO-010.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-divergent-checks` | Status quo: range opener checks Descriptor only when ordinal 0 is CONTROL | status quo | yes | — |
| `enforce-order-in-range-reader` | Add ordinal-0 CONTROL Descriptor and rank-order checks to the range reader | ledger alternative | yes | — |
| `shared-structural-validator` | Factor one structural validator used by full, range and public-inspection readers | ledger alternative | yes | — |
| `differential-fuzzing-gate` | Keep separate code but gate on differential fuzzing | ledger alternative | yes | EXP-CONF-010 |

**Ledger exclusions:** {"candidate_id": "status-quo-divergent-checks", "reason": "Reader acceptance diverges from the frozen object-order rule", "invariant_violated": "I15; docs/crypto-wire-v1.md L515-520"}.
**Exclusions proposed in this protocol (need independent sign-off, CR2):** `accept-unknown-protection-class`, `negotiated-version-fallback`, `remove-decode-support`, `status-quo-divergent-checks`.

**R0 checklist:** 1 status quo: `status-quo-divergent-checks`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 4 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-CRY-079 — What deterministic, normative rule assigns objects to CONTROL versus PAYLOAD segments and closes segments, and do segment boundaries affect physical identity or signatures?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CONF-012.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-implementation-defined` | Status quo: assignment defined by the Rust writer | status quo | yes | — |
| `normative-assignment-table` | Normative assignment and closing table in the wire spec | SPEC design | yes | — |
| `one-object-per-segment` | One object per segment | ledger alternative | yes | — |
| `size-bounded-segments-with-vectors` | Size-bounded segments with published vectors | ledger alternative | yes | — |

**Ledger exclusions:** none recorded in the ledger.
**Exclusions proposed in this protocol (need independent sign-off, CR2):** `accept-unknown-protection-class`, `negotiated-version-fallback`, `remove-decode-support`, `status-quo-divergent-checks`.

**R0 checklist:** 1 status quo: `status-quo-implementation-defined`; 2 SPEC design: `normative-assignment-table`; 3 ledger alternatives: 4 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: `normative-assignment-table`.

#### DEC-CON-012 — When and under what identity does ecf/bootstrap-v1 format 0.1 become frozen Entrybound v1: namespace string, major/minor semantics (unknown minor readable or not), preamble length growth policy, reserved-bit outcome class, magic generation digit relation, and the consequences for crypto transcripts and identity descriptors that bind namespace and version?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CON-008, EXP-CONF-001, EXP-CONF-002, EXP-CONF-004, EXP-CONF-006, EXP-EVAL-008.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `keep-bootstrap-identity` | ecf/bootstrap-v1 and 0.1 become the permanent v1 identity | ledger alternative | yes | EXP-CON-008 |
| `rename-to-v1-major-1` | New namespace and major 1; regenerate vectors; bootstrap archives legacy-readable | ledger alternative | yes | EXP-CON-008 |
| `rename-with-dual-acceptance-window` | New identity for writers; readers accept both for a stated window | ledger alternative | yes | EXP-CON-008 |
| `minor-tolerant-readers` | Unknown minor versions readable when all incompat bits are known | ledger alternative | yes | EXP-CON-008 |
| `strict-version-gate-status-quo` | Any unknown major or minor refused | ledger alternative | yes | EXP-CON-008 |
| `negotiated-version-fallback` | Readers fall back to an older interpretation on unknown versions | ledger alternative | yes | EXP-CON-008 |

**Ledger exclusions:** {"candidate_id": "negotiated-version-fallback", "reason": "Version is checked first and unknown versions are rejected without fallback", "invariant_violated": "SPEC §9.9 rule 2 L1159"}.
**Exclusions proposed in this protocol (need independent sign-off, CR2):** `accept-unknown-protection-class`, `negotiated-version-fallback`, `remove-decode-support`, `status-quo-divergent-checks`.

**R0 checklist:** 1 status quo: no ledger id marked status quo; the critic pass confirms which candidate is the behaviour at `9e44608`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 6 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-CON-011 — What do read_only_compat and compat mean for immutable archives: which operations (repack, repack --add, recipient add/remove, password rotation, signature add, strip-provenance, export, random read) an unknown ro_compat bit forbids, whether tools must refuse unknown ro_compat and compat bits before rewriting, and how refusals name unknown features?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CONF-003, EXP-CONF-004, EXP-EVAL-008.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-uninterpreted` | Unknown ro_compat and compat bits silently accepted; no gating | status quo | yes | — |
| `ext4-rewrite-forbidden` | Unknown ro_compat forbids any operation producing a new archive from the source | ledger alternative | yes | — |
| `mutation-and-rewrite-forbidden` | Unknown ro_compat forbids rewrites and in-place key or signature mutation but allows export and read | ledger alternative | yes | — |
| `two-tier-only` | Drop ro_compat (EROFS style) and use incompat for anything unsafe to rewrite | ledger alternative | yes | — |
| `read-write-version-pair` | SQLite-style read and write version bytes instead of bit tiers | ledger alternative | yes | — |
| `feature-name-registry-in-diagnostics` | Refusals name unknown features from a shipped registry snapshot | ledger alternative | yes | — |

**Ledger exclusions:** none recorded in the ledger.
**Exclusions proposed in this protocol (need independent sign-off, CR2):** `accept-unknown-protection-class`, `negotiated-version-fallback`, `remove-decode-support`, `status-quo-divergent-checks`.

**R0 checklist:** 1 status quo: `status-quo-uninterpreted`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 6 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-CON-008 — How is the v1 deprecation mechanism realised (retirement table location, DEPRECATED diagnostic, recorded writer override, codec removal policy), and which axes (recipient types, signature schemes, codecs, transforms, timestamps) may be added within a format version versus requiring a new version?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-EVAL-008.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `spec-text-only-status-quo` | Mechanism described in SPEC only; no diagnostic, table or override record | SPEC design | yes | — |
| `registry-retirement-table-with-diagnostic` | Retirement table in the normative registry plus DEPRECATED diagnostic class and code | ledger alternative | yes | — |
| `override-recorded-in-aux` | Forced use of retired identifiers recorded in AUX-covered data | ledger alternative | yes | — |
| `new-version-per-axis-change` | Any new algorithm or axis requires a new format version (one suite per version) | ledger alternative | yes | — |
| `independent-axis-registries` | Recipient, signature and codec registries extend within a version (age style) | ledger alternative | yes | — |
| `refuse-retired-on-read` | Readers refuse archives using retired identifiers | ledger alternative | yes | — |
| `remove-decode-support` | Drop decode support for retired codecs (Kopia LZ4 precedent) | ledger alternative | yes | — |

**Ledger exclusions:** {"candidate_id": "refuse-retired-on-read", "reason": "Retired-but-known identifiers must decode normally with a diagnostic", "invariant_violated": "SPEC §20.4 L2217"}; {"candidate_id": "remove-decode-support", "reason": "Files outlive software; retirement never removes decoding", "invariant_violated": "SPEC §20.4 L2214-2217"}.
**Exclusions proposed in this protocol (need independent sign-off, CR2):** `accept-unknown-protection-class`, `negotiated-version-fallback`, `remove-decode-support`, `status-quo-divergent-checks`.

**R0 checklist:** 1 status quo: no ledger id marked status quo; the critic pass confirms which candidate is the behaviour at `9e44608`; 2 SPEC design: `spec-text-only-status-quo`; 3 ledger alternatives: 7 ledger candidates listed above; 4 strongest incumbent: `independent-axis-registries`, `remove-decode-support`; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: `registry-retirement-table-with-diagnostic`.

#### DEC-CRY-071 — What is the extension contract for new recipient stanza types and protection classes: skip-and-authenticate unknown types, where unknown classes fail closed, and whether new methods need feature bits or a new suite?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CONF-004, EXP-EVAL-008.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-skip-unknown-types` | Status quo: unknown class-1 types skipped; wire layer does not validate class | status quo | no | — |
| `enforce-class-in-wire-layer` | Validate protection class in wire decoding | ledger alternative | no | — |
| `feature-bit-per-method` | New required feature bit per recipient method | ledger alternative | no | — |
| `suite-version-per-method` | New suite for new methods | ledger alternative | no | — |
| `grease-reserved-types` | GREASE-style reserved types in tests | ledger alternative | no | — |
| `accept-unknown-protection-class` | Accept stanzas with unknown protection classes | ledger alternative | yes | — |

**Ledger exclusions:** {"candidate_id": "accept-unknown-protection-class", "reason": "Unknown protection classes must fail closed", "invariant_violated": "I12 and docs/crypto-suite-v1.md L439-442"}.
**Exclusions proposed in this protocol (need independent sign-off, CR2):** `accept-unknown-protection-class`, `negotiated-version-fallback`, `remove-decode-support`, `status-quo-divergent-checks`.

**R0 checklist:** 1 status quo: `status-quo-skip-unknown-types`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 6 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `status-quo-skip-unknown-types`, `enforce-class-in-wire-layer`, `feature-bit-per-method`, `suite-version-per-method`, `grease-reserved-types`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-CRY-104 — When no known stanza matches and unknown recipient stanza types are present, what outcome class and detail does a reader report, and must unknown-type stanzas be represented in the private recipient directory? The general extension contract, including class enforcement and feature bits, is decided in recipient-stanza-extension-contract.

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-014.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-no-match-no-entry` | Status quo: NO_MATCHING_RECIPIENT; unknown-type stanzas have no directory entry | status quo | no | — |
| `unsupported-with-type-detail` | Report UNSUPPORTED naming unknown stanza types when no known stanza matches | ledger alternative | no | — |
| `qualified-no-match` | Keep NO_MATCHING_RECIPIENT but add a non-secret note that unsupported stanza types were present | ledger alternative | no | — |
| `generic-directory-entries` | Require generic directory entries for unknown-type stanzas in future writers | ledger alternative | no | — |
| `forbid-unknown-in-v1` | Reject unknown stanza types in crypto v1 archives | ledger alternative | no | — |
| `strip-unknown-stanzas` | Drop unknown stanzas before authentication | ledger alternative | yes | — |

**Ledger exclusions:** {"candidate_id": "strip-unknown-stanzas", "reason": "Unknown stanzas must remain covered by envelope MAC and recipient-set digest", "invariant_violated": "I12; CRYPTO-014/016"}.
**Exclusions proposed in this protocol (need independent sign-off, CR2):** `accept-unknown-protection-class`, `negotiated-version-fallback`, `remove-decode-support`, `status-quo-divergent-checks`.

**R0 checklist:** 1 status quo: `status-quo-no-match-no-entry`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 6 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `status-quo-no-match-no-entry`, `unsupported-with-type-detail`, `qualified-no-match`, `generic-directory-entries`, `forbid-unknown-in-v1`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-CRY-032 — Which experimental encrypted byte forms (pre-correction Descriptor v1 encrypted archives, pre-2026-08-30 wrap-AD archives) must v1 readers open, for how long, and with what labelling?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-EVAL-008.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-read-legacy-descriptor-v1` | Status quo: readers accept legacy Descriptor v1 encrypted form; wrap-AD pre-correction status unknown | status quo | yes | — |
| `read-with-warning-until-v1` | Accept with warning until v1 freeze, then refuse | ledger alternative | yes | — |
| `migration-tool` | Provide one-shot migration re-encrypting to the corrected form | ledger alternative | yes | — |
| `refuse-all-experimental-forms` | Refuse all pre-correction forms now | ledger alternative | yes | — |
| `permanent-support` | Support all experimental forms indefinitely | ledger alternative | yes | — |

**Ledger exclusions:** none recorded in the ledger.
**Exclusions proposed in this protocol (need independent sign-off, CR2):** `accept-unknown-protection-class`, `negotiated-version-fallback`, `remove-decode-support`, `status-quo-divergent-checks`.

**R0 checklist:** 1 status quo: `status-quo-read-legacy-descriptor-v1`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 5 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.
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

_Generated by `python research/tools/experiments/fill_crypto_auto_blocks.py` for EXP-CRYPTO-016; edit the inputs, not this block._
<!-- END AUTO:common -->
