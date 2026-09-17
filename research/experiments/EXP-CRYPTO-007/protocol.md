# EXP-CRYPTO-007: Encrypted public-byte leakage inventory and count inference

<!-- BEGIN AUTO:meta -->
| Field | Value |
|---|---|
| Index status | **NEEDS_TOOLING** |
| Kind | decision |
| Domain | crypto (program §22.2, §22.3, §22.4, §22.5) |
| Decisions informed | DEC-CRY-030, DEC-CRY-026, DEC-CRY-064, DEC-CRY-008, DEC-CRY-019, DEC-CRY-025, DEC-CMP-022, DEC-CMP-029, DEC-MOD-012, DEC-CRY-039, DEC-INT-009 |
| Platforms | Windows+Linux/x86-64 |
| Requires timing | False |
| Estimates | 40 machine-hours; 65 GB disk |
| Tooling to build | clean-room public_inventory.py; bytescan.py; count_inference.py |
| Environment prerequisites | none beyond the harness and the ebr venv |
| Blocked arms | none |
| Specs | `spec.yaml` |
| Validation looks | owns look 1 for DEC-CMP-022, DEC-CRY-008, DEC-CRY-030; participates in looks owned by other experiments for DEC-CMP-029, DEC-CRY-019, DEC-CRY-025, DEC-CRY-026, DEC-CRY-064, DEC-MOD-012 |
| Gates | G-A and G-B unmet at this revision (generated `gate_state` column of `research/experiments/index.csv`); timing runs also need a PASS calibration record for the calibration identity (PA-04) |
| Conventions | `research/experiments/_index/crypto-conventions.md` (CR0-CR10); program amendments `research/experiments/_program/design-revision-round1.md` |
<!-- END AUTO:meta -->

## 1. Question

In a crypto-v1 encrypted archive, which bytes are public, what plaintext-derived information does each public field carry, and how accurately can an observer infer entry count, unique chunk count, total logical size and within-archive duplication from them? Do the documented privacy claims survive: metadata-private, entry count hidden, per-entry sizes hidden, no plaintext digests exposed? Also, what do detached signatures, content-dependent feature bits and recipient fields add, and how does the status quo compare with restic, borg, age, gpg and 7z encrypted headers on the same inputs?

## 2. Hypotheses and falsification

- **H1 (inventory).** Every public field falls in exactly one class: constant, option-derived, content-derived or random. The content-derived fields are exactly those listed in `docs/crypto-threat-model-v1.md` "Deliberately public information" (L165-192). A content-derived public field missing from that list falsifies H1 and is recorded as an undocumented leak (DEC-CRY-030).
- **H2 (no plaintext digests).** A byte scan of every public region finds zero occurrences of any plaintext-derived digest (LAI, AUX, PCR, chunk SHA-256, file SHA-256, logical digests) and zero plaintext substrings of 8 or more bytes. Any hit falsifies the SPEC §9.8 claim, is an R1-relevant finding under HC-14, and is escalated.
- **H3 (count inference).** Under status-quo BUCKETED padding and one-object-per-segment layout, an estimator trained on tuning archives predicts entry count within a factor of 2 for at least 80% of validation archives and unique chunk count within ±10% for at least 80%. The "entry count hidden" and "per-entry sizes hidden" claims are then quantised, not hidden (DEC-CRY-026 `downgrade-to-quantised`).
- **H4 (detached signatures).** A detached `.ebsig` over an encrypted archive lets an observer who holds a candidate file set confirm the archive's logical content with `presence_advantage` `DISTINGUISHABLE` above 0 whenever the candidate set contains the exact tree, as `docs/crypto-suite-v1.md` L684-686 documents.
- **Falsification:** H3 is falsified if inference accuracy is below those rates, which would support the status-quo claims as worded.

## 3. Decisions informed and evidence route

<!-- BEGIN AUTO:candidates -->
#### DEC-CRY-030 — Which bytes of an encrypted archive remain public (preamble feature bits including embedded-signature presence 0x200, recipient count, padded lengths, segment and record counts, buckets) and which must be hidden, padded or declared as accepted leakage?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-008, EXP-CRYPTO-021.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-documented-public-framing` | Status quo: public discovery, envelope, framing and feature bits | status quo | yes | — |
| `hide-signature-presence-new-version` | Move signature presence out of public feature bits (new version) | ledger alternative | yes | — |
| `pad-record-counts` | Pad record and segment counts | ledger alternative | yes | — |
| `maximum-padding-default` | Default to maximum padding mode | ledger alternative | yes | — |
| `declare-accepted-leakage` | Accept and document each residual leak | ledger alternative | yes | — |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: `status-quo-documented-public-framing`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 5 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-CRY-026 — Which metadata-privacy claims are accepted for encrypted INDEXED archives (metadata-private, entry count hidden, per-entry sizes hidden, DATA records reveal only class, no plaintext digests exposed), and must any be downgraded or backed by stronger manifest padding?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-006, EXP-CRYPTO-021.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-claims` | Status quo claims in SPEC §9.8, help text and docs | status quo | yes | — |
| `downgrade-to-quantised` | Downgrade entry count and per-entry sizes to quantised in SPEC table and help | SPEC design | yes | — |
| `coarser-control-padding` | Coarser manifest/Index padding to hide entry count more strongly | ledger alternative | yes | EXP-CRYPTO-006 |
| `publish-leakage-inventory` | Publish a normative public-field leakage inventory with measured inference bounds | ledger alternative | yes | — |
| `keyed-public-digests` | Replace unkeyed public ciphertext digests with keyed MACs where observers gain linkage | ledger alternative | yes | — |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: `status-quo-claims`; 2 SPEC design: `downgrade-to-quantised`; 3 ledger alternatives: 5 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: `publish-leakage-inventory`.

#### DEC-CRY-064 — Which recipient-related public fields (stanza count, types, classes, X-Wing encapsulations, password salt and Argon2 parameters) are acceptable, should recipient count be padded with dummy stanzas, and what may keyless inspect --crypto and diff --public report about recipients? The whole-archive public byte inventory is decided in encrypted-public-byte-leakage-inventory.

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-006, EXP-CRYPTO-017, EXP-CRYPTO-021.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-public-count-types` | Status quo: count, types, classes and parameters public; no stanza padding; keyless inspect and diff show them | status quo | yes | — |
| `dummy-stanza-padding` | Optional dummy stanzas bucketing recipient count (SPEC §9.8 optional stanza padding) | SPEC design | yes | — |
| `uniform-stanza-encoding` | Indistinguishable stanza encodings across types in a later version | defer / no change | yes | — |
| `restrict-keyless-reporting` | Keyless inspect and diff omit recipient count and types by default | ledger alternative | yes | — |
| `document-only` | Keep exposure and document it precisely in inspect --crypto | defer / no change | yes | — |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: `status-quo-public-count-types`; 2 SPEC design: `dummy-stanza-padding`; 3 ledger alternatives: 5 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: `uniform-stanza-encoding`, `document-only`; 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-CRY-008 — Should encrypted archives expose content-dependent incompatibility bits (0x8000 POSIX/symlinks, 0x10000 platform security) in the public preamble, always set them, or declare them privately?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-PLAT-005.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-content-dependent-bits` | Status quo: set bits exactly when such content exists | status quo | yes | EXP-PLAT-005 |
| `always-set-under-encryption` | Always set content-dependent metadata bits in encrypted archives | ledger alternative | yes | EXP-PLAT-005 |
| `private-declaration` | Move content-dependent requirements into the encrypted Descriptor with a generic public bit (new crypto feature) | ledger alternative | yes | EXP-PLAT-005 |
| `document-as-public-leak` | Keep status quo and list as deliberately public information | ledger alternative | yes | EXP-PLAT-005 |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: `status-quo-content-dependent-bits`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 4 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-CRY-019 — For encrypted archives, should the CLI default to embedded signatures, warn about or refuse detached signatures that expose signer and plaintext-derived roots, or add a privacy-preserving detached form?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-014, EXP-ECO-007, EXP-ECO-016.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-detached-default` | Status quo: detached unless --embed | status quo | no | EXP-ECO-007 |
| `embedded-default-for-encrypted` | Default to --embed for encrypted archives | ledger alternative | no | EXP-ECO-007 |
| `warn-on-detached` | Warn that detached signatures expose signer and roots | ledger alternative | no | EXP-ECO-007 |
| `refuse-detached-without-flag` | Require an explicit flag for detached signatures on encrypted archives | ledger alternative | no | EXP-ECO-007 |
| `addressing-only-public-signature-new-version` | New public signature form binding only addressing context | ledger alternative | no | — |
| `keyed-root-detached-new-version` | Detached form over keyed commitments to roots (new version) | ledger alternative | no | — |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: `status-quo-detached-default`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 6 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `addressing-only-public-signature-new-version`, `keyed-root-detached-new-version`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-CRY-025 — Should dedup under encryption stay within one archive key domain, be disabled for provider-correlation threat models, or later extend to multi-archive key domains?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CHUNK-006, EXP-CHUNK-007, EXP-CRYPTO-006, EXP-CRYPTO-021.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-single-archive-domain` | Status quo: dedup within one archive under a fresh AFK | status quo | yes | — |
| `no-dedup-option` | Option to disable dedup under encryption | ledger alternative | yes | EXP-CHUNK-006, EXP-CRYPTO-006 |
| `multi-archive-key-domain` | Shared key domain across incremental/base archives (post-v1) | defer / no change | no | — |
| `cross-tenant-convergent` | Cross-tenant or convergent dedup | ledger alternative | no | EXP-CHUNK-006 |

**Ledger exclusions:** {"candidate_id": "cross-tenant-convergent", "reason": "Confirmation-of-file attacks; frozen non-goal", "invariant_violated": "SPEC §9.7; docs/crypto-threat-model-v1.md §Non-goals L208"}.

**R0 checklist:** 1 status quo: `status-quo-single-archive-domain`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 4 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: `multi-archive-key-domain`; 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `multi-archive-key-domain`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-CMP-022 — Should encrypted archives record the underlying planner version (for example balanced-v6-enc-v1) instead of profile-only <profile>-enc-v1, and should encryption refuse EAMs whose planner ID does not map to a frozen profile?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-PLANNER-010, EXP-PLANNER-012.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-profile-enc-v1` | Status quo: <profile>-enc-v1 overwrites the v6 ID; unmapped IDs pass through | status quo | no | EXP-PLANNER-010 |
| `versioned-enc-id` | Record <profile>-v6-enc-v1 style IDs | ledger alternative | no | EXP-PLANNER-010 |
| `separate-crypto-and-planner-fields` | Keep planner_id unchanged and record encryption construction separately | ledger alternative | no | EXP-PLANNER-010 |
| `refuse-unmapped` | Additionally refuse encryption of EAMs with unmapped planner IDs | ledger alternative | no | EXP-PLANNER-010 |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: `status-quo-profile-enc-v1`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 4 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-CMP-029 — Should planner decisions (candidate rankings, measured sizes, rejection reasons) be persisted in archives so explain can report them as RECORDED, recomputed from retained plaintext, or left NOT_RECORDED, and where should existing reconstruction audits live (authoritative sections affecting PCI, or a separate non-authoritative area)?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-005, EXP-ECO-019, EXP-PLANNER-009, EXP-PLANNER-010, EXP-PLANNER-012.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-audits-only` | Status quo: reconstruction fallbacks and audits persisted in authoritative sections; rankings NOT_RECORDED; explain re-derives alternatives from plaintext | status quo | no | EXP-PLANNER-010 |
| `persist-full-rankings` | Persist every candidate size per chunk | ledger alternative | no | EXP-PLANNER-010 |
| `persist-top-k-summary` | Persist compact per-plan or per-category summaries (winner margin, runner-up) | ledger alternative | no | EXP-PLANNER-010 |
| `recompute-with-planner-id` | Recompute rankings by rerunning the recorded planner ID, labelled DERIVED | ledger alternative | no | EXP-PLANNER-010 |
| `sidecar-decision-log` | Emit decision log as an optional sidecar outside the archive | ledger alternative | no | EXP-PLANNER-010 |
| `drop-audits` | Remove creation audits from archives entirely | ledger alternative | no | — |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: `status-quo-audits-only`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 6 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `drop-audits`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-MOD-012 — Given that encrypted bytes are intentionally non-reproducible, what reproducibility and identity guarantees apply to encrypted archives (LAI/AUX stability, PCR under keyed boundaries), and what identity material may be exposed outside encryption?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-002, EXP-SCALE-005.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `lai-aux-anchor-status-quo` | Status quo: fresh randomness; LAI and AUX stable, PCR may change with AFK-keyed boundaries, PCI always differs | status quo | no | — |
| `stable-pcr-under-encryption` | Keep boundaries key-independent so PCR is also stable (conflicts with boundary-leakage mitigations) | ledger alternative | no | — |
| `encrypted-roots-only` | Keep LAI/AUX inside encryption; expose only keyed or blinded identities publicly | ledger alternative | yes | — |
| `keyed-lai-commitment` | Publish a keyed commitment to LAI instead of LAI in detached signatures | ledger alternative | yes | — |
| `deterministic-test-vectors-only` | Deterministic encryption only behind test-only interfaces for vectors (status quo for vectors) | ledger alternative | no | — |
| `convergent-encryption-mode` | Deterministic convergent-encryption mode for reproducible ciphertext | ledger alternative | no | — |

**Ledger exclusions:** {"candidate_id": "convergent-encryption-mode", "reason": "Deterministic production ciphertext is prohibited and leaks file presence to holders of candidate files", "invariant_violated": "docs/crypto-threat-model-v1.md §Nondeterminism MUST NOT; SPEC §12.3 L1487 non-goal"}.

**R0 checklist:** 1 status quo: `lai-aux-anchor-status-quo`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 6 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `lai-aux-anchor-status-quo`, `stable-pcr-under-encryption`, `deterministic-test-vectors-only`, `convergent-encryption-mode`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-CRY-039 — What residual chunk-size and boundary leakage is acceptable for the default encrypted profile, and how must I24 and related claims (quantised not hidden, provably secure keyed-prf, defense in depth) be worded in docs, help and inspect?

Ledger status `EXTERNAL_REVIEW_REQUIRED`, blocker class `EXTERNAL_REVIEW`. Other experiments informing it: EXP-CRYPTO-021, EXP-CRYPTO-002, EXP-CRYPTO-006.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-objective-and-qualitative-disclosure` | Status quo: I24 as objective; qualitative residual-leakage text; keyed-prf described as provably secure | status quo | no | — |
| `measured-bound-per-mode` | Publish measured leakage metrics per padding x boundary mode and word claims to those numbers | ledger alternative | no | — |
| `strong-claim-only-for-phte-plus-maximum` | Allow a strong boundary-privacy claim only for PHTE plus MAXIMUM padding; everything else labelled quantised | ledger alternative | no | — |
| `restate-i24-as-documented-leakage` | Restate I24 as documented, bounded leakage rather than no leakage | ledger alternative | no | — |
| `disable-dedup-under-encryption` | Eliminate within-archive dedup structure under encryption to strengthen I24 | ledger alternative | no | — |
| `external-review-sets-wording` | Defer all claim wording to the external security review outcome | defer / no change | no | — |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: `status-quo-objective-and-qualitative-disclosure`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 6 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: `external-review-sets-wording`; 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `status-quo-objective-and-qualitative-disclosure`, `measured-bound-per-mode`, `strong-claim-only-for-phte-plus-maximum`, `restate-i24-as-documented-leakage`, `disable-dedup-under-encryption`, `external-review-sets-wording`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-INT-009 — Will encrypted Sidecar or Incremental archives be supported after v1, under which crypto version and key-domain model (per-archive AFK, shared domain across chains), and with what leakage from external plaintext digests?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-021.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `never` | Encrypted archives remain Complete-only permanently | defer / no change | no | — |
| `new-crypto-version-per-archive-afk` | Post-v1 crypto version with per-archive AFK and encrypted External digests | defer / no change | no | — |
| `shared-key-domain-chain` | Key domain spanning a base chain enabling cross-archive dedup | ledger alternative | no | — |
| `encrypt-references-only` | Encrypt External reference records; bases independently encrypted | ledger alternative | no | — |
| `crypto-v1-incremental` | Allow non-Complete roles within crypto-v1 | ledger alternative | no | — |

**Ledger exclusions:** {"candidate_id": "crypto-v1-incremental", "reason": "crypto-v1 commitment and public context fix archive_role=1", "invariant_violated": "crypto-v1 wire frozen"}.

**R0 checklist:** 1 status quo: no ledger id marked status quo; the critic pass confirms which candidate is the behaviour at `9e44608`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 5 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: `never`, `new-crypto-version-per-archive-afk`; 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `never`, `new-crypto-version-per-archive-afk`, `shared-key-domain-chain`, `encrypt-references-only`, `crypto-v1-incremental`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.
<!-- END AUTO:candidates -->

Evidence route: `EMPIRICALLY_MEASURED` (inventory, byte scan, inference accuracy) plus `FORMALLY_DERIVED` field classification from `docs/crypto-wire-v1.md` with line citations. Acceptance of residual leakage is an external-review input (SPEC §25.2, dossier EXP-CRYPTO-021).

## 4. Candidates and arms

**Archive option factorial (status-quo writer):**

| Factor | Levels |
|---|---|
| Protection | X-Wing recipients 1, 2, 16, 128, 1,024; password (one stanza) |
| Boundary mode | default (secret Gear), `keyed-prf` (PHTE) |
| Padding | `none`, `bucketed`, `max` |
| Signatures | none; embedded (`sign --embed`); detached `.ebsig` |
| Content-dependent metadata | trees with and without symlinks (POSIX feature 0x8000); captured on `windows-host` (platform security 0x10000, F-0001) and on `wsl-ubuntu` |
| Profile | `fast`, `balanced`, `dense`, `extreme` |

**Candidate policies (as exact observation transforms, or research writer where noted):**
- DEC-CRY-030: `status-quo-documented-public-framing`; `hide-signature-presence-new-version` (0x200 removed from the public bitmap; transform); `pad-record-counts` (dummy records to quarter-octave count buckets; transform); `maximum-padding-default` (P6); `declare-accepted-leakage` (inventory only).
- DEC-CRY-026: `status-quo-claims`; `downgrade-to-quantised`; `coarser-control-padding`; `publish-leakage-inventory`; `keyed-public-digests` (the byte scan tests whether any unkeyed public ciphertext digest links archives, for example footer or context digests shared between sibling archives after `key add`).
- DEC-CRY-064: `status-quo-public-count-types`; `dummy-stanza-padding` (stanza count bucketed: 1, 2, 4, 8, … 1,024; size and unlock cost in EXP-CRYPTO-017); `uniform-stanza-encoding` (transform: all stanza types padded to the maximum stanza size); `restrict-keyless-reporting` (CLI output census); `document-only`.
- DEC-CRY-008: `status-quo-content-dependent-bits`; `always-set-under-encryption`; `private-declaration` (transform: generic public bit); `document-as-public-leak`.
- DEC-CRY-081 / DEC-CRY-079: layouts L0-L4 as in EXP-CRYPTO-002 (count-inference impact).
- DEC-CRY-019 / DEC-MOD-012: detached versus embedded signatures; `keyed-lai-commitment` and `encrypted-roots-only` (formal: the public linkage disappears by construction; the residual is measured).
- DEC-CRY-025: `status-quo-single-archive-domain` versus `no-dedup-option` (inference of within-archive duplication from PAYLOAD object count versus total logical size).

**Incumbent references (same trees):**
- `restic` 0.16.4 (pack files and index sizes);
- `borg` 1.2.8 `repokey` with `none`, `lz4` and `obfuscate`;
- `age` 1.1.1 over a tar stream;
- `gpg` 2.4.4 symmetric and pubkey over a tar stream;
- `7z` 23.01 `-mhe=on -p`.

The same inventory schema is applied to each.

## 5. Corpus selectors

- **Tuning:** `f04-tuning-sourcelike`, `f04-tuning-maildir`, `f04-tuning-real-netbsd-pkgsrc`, `f01-tuning-curl-8-19-0-git`, `f03-tuning-ripgrep-cargo-vendor`, `f05-tuning-gutenberg-books`, `f09-alpine-324-rootfs-binaries`, `f11-tuning-ubuntu-noble-debs`, `f13-tuning-librivox-audiobook-medium`, `f17-tuning-duptree-mixed`, `f18-tuning-zstd-releases-1-5-x`, `f19-tuning-zoo`, `f19-tuning-real-ntfs-metadata`, `f19-tuning-real-ext4-acl-selinux`, `f20-tuning-generated-private-vault`, plus generated trees with controlled entry counts (10^1 to 10^6, log-spaced, `text-v1` files of seeded sizes) for the inference training grid.
- **Validation:** `f04-validation-objstore`, `f04-validation-real-gentoo-repo-20260915`, `f01-validation-redis-7-2-16-git`, `f03-validation-yq-go-mod-vendor`, `f05-validation-rfc-texts`, `f09-fedora-44-usr-binaries`, `f11-maven-central-jvm-jars`, `f13-validation-librivox-audiobook-medium`, `f17-validation-duptree-vendor`, `f18-validation-redis-7-2-point-releases`, `f19-validation-deep`, `f19-validation-real-ntfs-metadata`. One registered look; the inference estimator is frozen before it.

## 6. Environment and platform requirements

- `wsl-ubuntu` for most cells.
- `windows-host` for Windows-captured metadata bit cells (F-0001): the Windows `ebound` build packs the NTFS-image-derived trees mounted read-only through `ntfs-3g` in WSL and then copied to NTFS scratch. The procedure is recorded in the run README.
- `timing: false`.

## 7. Commands and tooling

Tooling to build:
- `research/tools/crypto/public_inventory.py`: a clean-room parser of crypto-v1 public framing written only from `docs/crypto-wire-v1.md` by a session not involved in the production writer. It emits `(offset, length, field, value, class)` rows and passes every public byte through a coverage check: every byte is attributed to exactly one field, otherwise the run fails.
- `research/tools/crypto/bytescan.py`: builds the plaintext-derived digest dictionary per archive from the plaintext tree (SHA-256 of files and chunks via `ebr-crypto observe --emit-digests`; LAI/AUX/PCR from `ebound inspect --json` with keys) and scans public regions (Aho-Corasick over digests and all 8-byte plaintext windows sampled at 1 in 64 offsets).
- `research/tools/crypto/count_inference.py`: quantile-regression and gradient-boosting estimators over public features, trained on tuning only.
- `research/tools/crypto/incumbent_inventory.sh`: the same schema for restic, borg, age, gpg and 7z outputs.
- `ebr-crypto keygen` (1,024 test recipients).

`spec.yaml`: candidates are option cells. The command packs, parses, scans and prints the inventory summary JSON.

## 8. Seeds, warmup, repetitions and adaptive rule

- Deterministic public structure given options: 2 repetitions (fresh randomness each). Field classes and lengths must agree. Random fields must differ, which gives a negative check for accidental determinism (HC-14 "no deterministic ciphertext").
- Inference estimator seeds are fixed. Tuning uses 5-fold cross-validation grouped by independence group.
- No warmups, no timing.

## 9. Measurement method and metrics

| Metric (canonical) | ID | OD | Role |
|---|---|---|---|
| `leak_min_entropy_bits` per public field and per archive (uniform prior over the tuning population of the family, deterministic fields) | T-17 | OD-16 (M16.1) | **primary** |
| `anonymity_set_median` over entry-count classes and over trees (known-tree universe) | T-17 | OD-16 | secondary |
| `presence_advantage` (known-tree confirmation from counts and lengths; from detached signatures) | T-17 | OD-16 (M16.2) | **primary** for H4 and the count-based fingerprint |
| byte-scan hits | binary (HC-14-relevant finding) | OD-15/OD-16 | screen |
| unattributed public bytes | binary (tool validity) | — | screen |
| inference error ratios (entry count, unique chunks, logical size) | descriptive | — | reported |
| `metadata_bytes` of CONTROL objects | T-02 | OD-05 | diagnostic |
| `fixed_overhead_bytes` of the envelope by recipient count | T-21 | OD-05 | diagnostic (EXP-CRYPTO-017 owns recipient cost) |

## 10. Normalization and statistical analysis

- **Inventory and byte scan:** exact census, no statistics.
- **Channel metrics:** exact for deterministic fields. Aggregation per §4.9 on absolute differences in bits (absolute-only metric).
- **Inference:** per-archive error, group-level accuracy fractions, family and corpus means. Intervals per §4.10 on group-level values.
- **Candidate comparisons** (for example `pad-record-counts` against the status quo): paired per archive; corpus Welch-Satterthwaite; confirmatory W against S at validation.
- **Incumbents:** a reference arm (REF-INC) reported beside every external comparison claim (§6.5). No superiority claim unless capability-matched: only encrypted, authenticated incumbents are compared on leakage.

## 11. Practical significance

T-17 (`leak_min_entropy_bits`, `anonymity_set_median`, `presence_advantage`), T-02 and T-21 from `research/methods/thresholds.json`. Wire-changing candidates (`hide-signature-presence-new-version`, `private-declaration`, `uniform-stanza-encoding`, `keyed-public-digests`) are at least T2 and possibly T4 (crypto construction), with the assessor computing the tier (§7.2).

## 12. Sensitivity analysis

§6 arms. Experiment-specific:
- prior (uniform versus population-frequency);
- estimator family (quantile regression versus gradient boosting);
- training-grid composition (generated only versus generated plus real);
- F-0001 host arm (Windows versus WSL capture).

## 13. Expected negative results worth recording

- Entry count is inferable within quarter-octave classes from CONTROL padded lengths. "Entry count hidden" is then contradicted as worded (DEC-CRY-026).
- The embedded-signature bit 0x200 reveals signing practice publicly.
- Content-dependent bits 0x8000/0x10000 reveal source-platform class (Windows versus Linux) for every Windows-created encrypted archive.
- restic or borg leak strictly more (per-chunk sizes) than Entrybound BUCKETED. A context claim, recorded with its capability match.
- A sibling-archive linkage exists through an unkeyed public digest after `key add`.

## 14. Threats to validity

- The clean-room parser could share a misreading with the production writer. Mitigated by the full-byte attribution check and by a second session reviewing field classification against the doc lines.
- Population priors from tuning items are not real-world priors.
- Inference estimators are lower bounds on inference power.
- Windows-capture arms depend on the copy procedure preserving security descriptors (checked by `ebound inspect` on the source tree).

## 15. Held-out placeholder (Phase D)

The frozen inventory tool, estimator and candidates run once on held-out items after unlock (`EB_HELDOUT_UNLOCK`). Byte-scan and inventory checks are also R1-relevant on held-out. No held-out item is named here.

## 16. Cost estimate and agent effort

- Compute: about 40 core-hours (option factorial on about 27 items, incumbents, inference training). Disk: about 60 GB transient and 5 GB retained.
- Agent effort: tooling **judgment** (clean-room parser by an independent session); execution **scripted**; analysis **judgment**.

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

_Generated by `python research/tools/experiments/fill_crypto_auto_blocks.py` for EXP-CRYPTO-007; edit the inputs, not this block._
<!-- END AUTO:common -->
