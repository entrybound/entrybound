# EXP-CRYPTO-015: Feature-composition and mutation matrix — identity roots, binding status, nonce/key uniqueness under attacker-controlled input

<!-- BEGIN AUTO:meta -->
| Field | Value |
|---|---|
| Index status | **NEEDS_TOOLING** — Interrupted-write cells need the HC-18 kill injector |
| Kind | decision |
| Domain | crypto (program §22.1, §22.4, §22.5) |
| Decisions informed | DEC-CRY-010, DEC-MOD-029, DEC-CRY-054, DEC-CRY-050, DEC-CRY-069, DEC-CRY-031, DEC-CRY-045, DEC-ACC-049, DEC-CON-016, DEC-LEG-040, DEC-CRY-105, DEC-CRY-057, DEC-CRY-084, DEC-CRY-049 |
| Platforms | Linux/x86-64 |
| Requires timing | False |
| Estimates | 60 machine-hours; 25 GB disk |
| Tooling to build | ebr-crypto compose; nonce logger (F-24); oracle; kill injector |
| Environment prerequisites | none beyond the harness and the ebr venv |
| Blocked arms | none |
| Specs | `spec.yaml` |
| Validation looks | owns look 1 for DEC-ACC-049, DEC-CON-016, DEC-CRY-010, DEC-CRY-031, DEC-CRY-045, DEC-CRY-050, DEC-CRY-054, DEC-CRY-057, DEC-CRY-069, DEC-CRY-105; participates in looks owned by other experiments for DEC-LEG-040, DEC-MOD-029 |
| Gates | G-A and G-B unmet at this revision (generated `gate_state` column of `research/experiments/index.csv`); timing runs also need a PASS calibration record for the calibration identity (PA-04) |
| Conventions | `research/experiments/_index/crypto-conventions.md` (CR0-CR10); program amendments `research/experiments/_program/design-revision-round1.md` |
<!-- END AUTO:meta -->

## 1. Question

For every combination of conversion evidence, preservation, encryption, signatures, recipient add/remove, password change, epoch rotation, random access and repack, what outcome (supported, refused, declared loss) and what identity roots (LAI/AUX/PCR/PCI) and binding statuses result, are they consistent with the identity definitions, and are they enforced by tests (DEC-CRY-010, DEC-MOD-029)? Under every mutation on an attacker-controlled input archive, does every produced record have a unique (key, nonce) and fresh salts (DEC-CRY-054), and which mutation contract do recipient changes and rotation use (DEC-CRY-050, DEC-CRY-069, DEC-CRY-031, DEC-CRY-045, DEC-CRY-105)?

## 2. Hypotheses and falsification

- **H1 (matrix completeness).** A normative executable matrix with a test per cell covers every operation combination; the status-quo per-operation behaviour has cells that are untested or inconsistent with the root definitions (for example AUX for `--add` per DEC-MOD-029's derived-from-definitions candidate).
- **H2 (nonce uniqueness).** Over 10,000 seeded mutation sequences on attacker-controlled inputs (duplicate salts, crafted ordinals), no produced record repeats a (derived-key, nonce) pair (MVT-14(c)). The status-quo reuse-payload/regenerate-control contract holds; `full-reencryption-every-mutation` trivially holds; a naive rewrap-only contract would repeat.
- **H3 (rotation cost vs safety).** Boundary-preserving rotation (DEC-CRY-050) keeps PCR stable but re-uses boundary secrets known to removed recipients, a measurable leakage against removed recipients; full rotation costs re-encryption time (EXP-CRYPTO-003) but leaks nothing to them.
- **Falsification:** any repeated (key, nonce) (H2) is an R1/HC-14 violation with the sequence as the artifact. H1/H3 are conformance/leakage verdicts.

## 3. Decisions informed and evidence route

<!-- BEGIN AUTO:candidates -->
#### DEC-CRY-010 — For each combination of conversion evidence, preservation, encryption, signatures, recipient changes, password rotation, random access and repack, which outcomes are supported, refused or declared loss in v1, and which identity roots, binding statuses and evidence records must result?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: none.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-per-operation` | Status quo: behaviour defined per operation, partially tested | status quo | yes | — |
| `normative-executable-matrix` | Normative matrix with a test per cell | ledger alternative | yes | — |
| `refuse-untested-compositions` | Refuse combinations without a tested cell | ledger alternative | yes | — |
| `declared-loss-records` | Record declared loss whenever a composition drops evidence | ledger alternative | yes | — |
| `restricted-v1-subset` | Limit stable v1 to a tested subset of compositions | ledger alternative | yes | — |
| `silent-evidence-drop` | Allow compositions to drop signatures or provenance without record | ledger alternative | yes | — |

**Ledger exclusions:** {"candidate_id": "silent-evidence-drop", "reason": "Evidence loss without a typed record", "invariant_violated": "I13"}.
**Exclusions proposed in this protocol (need independent sign-off, CR2):** `mixed-hybrid-and-password`, `rewrap-only-removal`, `silent-evidence-drop`, `spec-table-as-written`.

**R0 checklist:** 1 status quo: `status-quo-per-operation`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 6 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: `normative-executable-matrix`.

#### DEC-MOD-029 — What are the exact LAI/PCR/AUX/PCI and signature-binding outcomes for every archive operation, including operations omitted from SPEC §8.1 and the inconsistent --add/content-change AUX cells, and are they enforced by tests?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CONF-004, EXP-CRYPTO-014.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `spec-table-as-written` | SPEC §8.1 table as written (AUX unchanged for --add) | SPEC design | yes | — |
| `derived-from-definitions` | Recompute every cell from root definitions (AUX changes whenever entry set or aux metadata changes) | ledger alternative | yes | — |
| `repo-refined-table` | Repo docs refinements for rekey/key add/remove plus derived cells | ledger alternative | yes | — |
| `executable-matrix` | Normative executable test matrix covering all operations and bindings | ledger alternative | yes | — |

**Ledger exclusions:** {"candidate_id": "spec-table-as-written", "reason": "AUX binds a Merkle root over all entry aux_digests, so adding entries must change it", "invariant_violated": "SPEC §8.0 L876 AUX definition"}.
**Exclusions proposed in this protocol (need independent sign-off, CR2):** `mixed-hybrid-and-password`, `rewrap-only-removal`, `silent-evidence-drop`, `spec-table-as-written`.

**R0 checklist:** 1 status quo: no ledger id marked status quo; the critic pass confirms which candidate is the behaviour at `9e44608`; 2 SPEC design: `spec-table-as-written`; 3 ledger alternatives: 4 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: `executable-matrix`.

#### DEC-CRY-054 — Do all mutation operations (recipient add/remove, signature embedding, epoch rotation, interrupted writes) guarantee fresh keys and nonces when the input archive is attacker-controlled, and which mutation strategy is adopted?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CONF-010, EXP-CRYPTO-021.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-reuse-payload-regenerate-control` | Status quo: reuse PAYLOAD segments byte-for-byte with pre-registered salts; regenerate CONTROL/terminal segments with fresh salts | status quo | yes | — |
| `full-reencryption-every-mutation` | Re-encrypt everything under a fresh AFK on every mutation | ledger alternative | yes | — |
| `epoch-rotation-on-untrusted-input` | Rotate encryption epoch whenever the input archive is not locally produced | ledger alternative | yes | — |
| `session-key-per-invocation` | Borg-2-style per-invocation session keys for new segments | ledger alternative | yes | — |

**Ledger exclusions:** none recorded in the ledger.
**Exclusions proposed in this protocol (need independent sign-off, CR2):** `mixed-hybrid-and-password`, `rewrap-only-removal`, `silent-evidence-drop`, `spec-table-as-written`.

**R0 checklist:** 1 status quo: `status-quo-reuse-payload-regenerate-control`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 4 ledger candidates listed above; 4 strongest incumbent: `session-key-per-invocation`; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-CRY-050 — Should key removal and password change keep rotating the AFK and rechunking under new boundary keys (PCR and physical bindings change), or offer a boundary-preserving or lazily rotated mode that keeps PCR stable at the cost of reusing boundary secrets known to removed recipients?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-SCALE-010, EXP-CRYPTO-021.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-full-rotation-rechunk` | Status quo: new AFK, archive ID and boundary keys; full re-plan | status quo | yes | — |
| `boundary-preserving-rotation` | Rotate AFK but keep boundary key and chunking | ledger alternative | yes | EXP-SCALE-010 |
| `separate-boundary-key-new-version` | Derive boundary keys independently of AFK (new version) | ledger alternative | yes | — |
| `lazy-rotation-on-repack` | Rotate boundary keys only at next repack | ledger alternative | yes | EXP-SCALE-010 |
| `rewrap-only-removal` | Remove stanza without rotating the file key | ledger alternative | yes | EXP-SCALE-010 |

**Ledger exclusions:** {"candidate_id": "rewrap-only-removal", "reason": "Removed recipients would keep the file key", "invariant_violated": "docs/crypto-threat-model-v1.md L105-107 removal rotates file key"}.
**Exclusions proposed in this protocol (need independent sign-off, CR2):** `mixed-hybrid-and-password`, `rewrap-only-removal`, `silent-evidence-drop`, `spec-table-as-written`.

**R0 checklist:** 1 status quo: `status-quo-full-rotation-rechunk`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 5 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-CRY-069 — Which recipient-mutation semantics bind the envelope in v1: AFK-preserving addition producing key-sharing sibling archives, fresh-epoch removal and password change with full re-encryption, rotation keeping every recipient, combined add and remove, and carry-over of directory labels, at what measured cost? Boundary-preserving rotation, beyond-RAM re-encryption and signature retention are decided in key-rotation-boundary-preservation, encryption-beyond-ram and signature-retention-after-recipient-mutation.

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-SCALE-010, EXP-CRYPTO-021, EXP-CRYPTO-023.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo` | Status quo: add rewraps the same AFK; remove and password change rotate AFK and archive_id, re-chunk and re-encrypt; no keep-all rotation; labels come from caller | status quo | no | — |
| `add-also-rotates` | Addition also rotates AFK and archive_id, avoiding key-sharing sibling archives at full re-encryption cost | ledger alternative | no | — |
| `add-rotates-archive-id-only` | Addition keeps the AFK but derives a fresh archive_id or segment salts to separate sibling versions | ledger alternative | no | — |
| `rotate-keep-all-command` | Explicit rotation keeping every recipient | ledger alternative | no | — |
| `combined-transaction` | Atomic add-and-remove in one epoch | ledger alternative | no | — |
| `carry-authenticated-labels` | Rotation carries authenticated directory labels for retained recipients | ledger alternative | no | — |
| `remove-rewrap-only` | Removal only deletes the stanza and rewraps | ledger alternative | no | — |

**Ledger exclusions:** {"candidate_id": "remove-rewrap-only", "reason": "A removed recipient already holds the AFK, so removal must rotate the AFK and re-encrypt", "invariant_violated": "CRYPTO-030/031"}.
**Exclusions proposed in this protocol (need independent sign-off, CR2):** `mixed-hybrid-and-password`, `rewrap-only-removal`, `silent-evidence-drop`, `spec-table-as-written`.

**R0 checklist:** 1 status quo: `status-quo`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 7 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `status-quo`, `add-also-rotates`, `add-rotates-archive-id-only`, `rotate-keep-all-command`, `combined-transaction`, `carry-authenticated-labels`, `remove-rewrap-only`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-CRY-031 — Is encrypted-to-encrypted repack (including boundary-mode or padding change) required for v1, and under which mutation contract (fresh AFK, AFK-preserving, signature staleness)?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-EVAL-008.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-refuse` | Status quo: encrypted repack refused (EB_CRYPTO_LAYOUT_UNSUPPORTED) | status quo | no | — |
| `fresh-epoch-repack` | Decrypt in memory and re-encrypt under fresh AFK and archive id, recipients preserved | ledger alternative | no | — |
| `afk-preserving-repack` | Repack keeping AFK and reusing unchanged payload segments | ledger alternative | no | — |
| `explicit-decrypt-then-pack` | Require explicit unpack plus encrypted pack | ledger alternative | no | — |
| `implicit-decrypt-output` | Repack encrypted input to unencrypted output implicitly | ledger alternative | no | — |

**Ledger exclusions:** {"candidate_id": "implicit-decrypt-output", "reason": "Implicit decryption on repack is forbidden", "invariant_violated": "docs/repack-v1.md L42; archive/tooling.rs:85-94"}.
**Exclusions proposed in this protocol (need independent sign-off, CR2):** `mixed-hybrid-and-password`, `rewrap-only-removal`, `silent-evidence-drop`, `spec-table-as-written`.

**R0 checklist:** 1 status quo: `status-quo-refuse`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 5 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `status-quo-refuse`, `fresh-epoch-repack`, `afk-preserving-repack`, `explicit-decrypt-then-pack`, `implicit-decrypt-output`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-CRY-045 — Is recipient-set integrity against other legitimate recipients a v1 goal: status quo (any file-key holder can edit the envelope; only an expected addressing signature detects it), mandatory signatures for multi-recipient archives, sender-authenticated KEM, per-recipient MACs, or a documented non-goal?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-021, EXP-CRYPTO-023.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-addressing-signature-only` | Status quo: envelope MAC keyed from AFK; addressing signature detects edits | status quo | no | — |
| `require-signature-multi-recipient` | Require a signature on multi-recipient archives | ledger alternative | no | — |
| `authenticated-kem` | Sender-authenticated KEM mode | ledger alternative | no | — |
| `per-recipient-mac` | Per-recipient MAC keys over the stanza set | ledger alternative | no | — |
| `documented-non-goal` | Declare insider integrity a non-goal in threat model and CLI output | defer / no change | no | — |

**Ledger exclusions:** none recorded in the ledger.
**Exclusions proposed in this protocol (need independent sign-off, CR2):** `mixed-hybrid-and-password`, `rewrap-only-removal`, `silent-evidence-drop`, `spec-table-as-written`.

**R0 checklist:** 1 status quo: `status-quo-addressing-signature-only`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 5 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: `documented-non-goal`; 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `status-quo-addressing-signature-only`, `require-signature-multi-recipient`, `authenticated-kem`, `per-recipient-mac`, `documented-non-goal`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-ACC-049 — How should repack publish and handle edge cases: in-place rewrite atomicity and crash safety, --rebuild-index spelling vs --index present, window violations in representation-only mode, declaring previously undeclared budgets, and ciphertext-preserving encrypted repack?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-ECO-015.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo` | prepare_repack in memory, caller publication, --index preserve\|present\|absent, undeclared budget preserved for STREAM->STREAM | status quo | no | EXP-ECO-015 |
| `atomic-replace-in-place` | Write temp then atomic rename when output omitted | ledger alternative | no | — |
| `require-output-path` | Refuse in-place repack | ledger alternative | no | — |
| `alias-rebuild-index` | Add --rebuild-index alias for --index present | ledger alternative | no | — |
| `always-declare-budget` | Repack always declares measured budgets | ledger alternative | no | — |
| `suggest-replan-on-window-violation` | Window refusal suggests replan or auto | ledger alternative | no | — |
| `ciphertext-relocation` | Encrypted repack moving segments without re-encryption (future) | ledger alternative | no | — |

**Ledger exclusions:** none recorded in the ledger.
**Exclusions proposed in this protocol (need independent sign-off, CR2):** `mixed-hybrid-and-password`, `rewrap-only-removal`, `silent-evidence-drop`, `spec-table-as-written`.

**R0 checklist:** 1 status quo: `status-quo`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 7 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `atomic-replace-in-place`, `require-output-path`, `alias-rebuild-index`, `always-declare-budget`, `suggest-replan-on-window-violation`, `ciphertext-relocation`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-CON-016 — How long must readers accept the pre-bb8cca9 legacy experimental crypto-v1 form (Descriptor v1 without 0x1000), should writers hard-refuse Descriptor-v1 plain parts outside tests, and will unencrypted archives ever carry an authenticated producer resource declaration?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-EVAL-008.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `accept-legacy-indefinitely-status-quo` | Keep accepting the legacy encrypted form without end date | ledger alternative | no | — |
| `drop-legacy-before-v1` | Remove legacy form acceptance before v1 | ledger alternative | no | — |
| `legacy-with-deprecated-diagnostic` | Accept with DEPRECATED diagnostics until a dated retirement | ledger alternative | no | — |
| `writer-hard-refusal` | encrypt_with_material refuses Descriptor-v1 plain parts in all builds | ledger alternative | no | — |
| `unencrypted-descriptor-v1-only-status-quo` | Unencrypted archives keep preamble-only declarations | ledger alternative | no | — |
| `unencrypted-descriptor-v2-optional` | Unencrypted archives may carry Descriptor v2 under a new feature bit | ledger alternative | no | — |
| `signature-bound-preamble-declaration` | Signatures bind preamble declarations in unencrypted archives | ledger alternative | no | — |
| `synthesize-declaration-for-legacy` | Readers synthesize declarations for legacy archives from actuals | ledger alternative | no | — |

**Ledger exclusions:** {"candidate_id": "synthesize-declaration-for-legacy", "reason": "Readers must never synthesize a producer declaration", "invariant_violated": "docs/crypto-implementation-v1.md L201-203"}.
**Exclusions proposed in this protocol (need independent sign-off, CR2):** `mixed-hybrid-and-password`, `rewrap-only-removal`, `silent-evidence-drop`, `spec-table-as-written`.

**R0 checklist:** 1 status quo: no ledger id marked status quo; the critic pass confirms which candidate is the behaviour at `9e44608`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 8 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `accept-legacy-indefinitely-status-quo`, `drop-legacy-before-v1`, `legacy-with-deprecated-diagnostic`, `writer-hard-refusal`, `unencrypted-descriptor-v1-only-status-quo`, `unencrypted-descriptor-v2-optional`, `signature-bound-preamble-declaration`, `synthesize-declaration-for-legacy`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-LEG-040 — Do physical or trust rewrites (repack, transcode, re-encryption, key add/remove, strip-provenance) append provenance records to a chain?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-EVAL-008, EXP-INT-009.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-copy-only` | Status quo: repack copies existing provenance, appends nothing | status quo | no | EXP-INT-009 |
| `append-rewrite-record` | Append a rewrite record (tool, operation, input PCI) affecting AUX | ledger alternative | no | EXP-INT-009 |
| `external-rewrite-receipt` | Emit external rewrite receipts without changing AUX | ledger alternative | no | EXP-INT-009 |
| `no-rewrite-provenance` | Never record rewrites (identity roots already show invariance) | defer / no change | no | EXP-INT-009 |

**Ledger exclusions:** none recorded in the ledger.
**Exclusions proposed in this protocol (need independent sign-off, CR2):** `mixed-hybrid-and-password`, `rewrap-only-removal`, `silent-evidence-drop`, `spec-table-as-written`.

**R0 checklist:** 1 status quo: `status-quo-copy-only`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 4 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: `no-rewrite-provenance`; 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-CRY-105 — When a stanza opens but the candidate AFK fails commitment or envelope MAC, should unlock hard-fail (status quo), continue to remaining stanzas, or evaluate all stanzas before deciding?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-010, EXP-CRYPTO-021, EXP-CRYPTO-023.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-hard-fail` | Status quo: integrity failure at the first commitment or MAC mismatch | status quo | no | EXP-CRYPTO-010, EXP-CRYPTO-023 |
| `continue-then-report` | Continue to remaining stanzas; report integrity failure only if none succeed | ledger alternative | no | EXP-CRYPTO-010, EXP-CRYPTO-023 |
| `evaluate-all-require-unique` | Evaluate all matching stanzas and require one consistent AFK | ledger alternative | no | EXP-CRYPTO-010, EXP-CRYPTO-023 |
| `continue-with-tamper-warning` | Continue and surface a tamper warning if another stanza succeeds | ledger alternative | no | EXP-CRYPTO-010, EXP-CRYPTO-023 |

**Ledger exclusions:** none recorded in the ledger.
**Exclusions proposed in this protocol (need independent sign-off, CR2):** `mixed-hybrid-and-password`, `rewrap-only-removal`, `silent-evidence-drop`, `spec-table-as-written`.

**R0 checklist:** 1 status quo: `status-quo-hard-fail`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 4 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-CRY-057 — Should a later version allow multiple password stanzas, recipient addition to password archives, protection-mode conversion, or a recovery mechanism, given that passwords may never share an archive with stronger recipients?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-ECO-021, EXP-EVAL-008.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-single-password-only` | Status quo: PASSWORD_ONLY with exactly one stanza; no conversion | status quo | no | — |
| `multiple-password-stanzas` | Allow several password stanzas in PASSWORD_ONLY archives | ledger alternative | no | — |
| `conversion-command-full-rekey` | Protection-mode conversion via full fresh-epoch re-encryption | ledger alternative | no | — |
| `separate-recovery-archive-or-escrow` | Recovery via a separately stored escrow artefact that never shares the hybrid envelope | defer / no change | no | — |
| `mixed-hybrid-and-password` | Hybrid recipients and a password in one archive | ledger alternative | yes | — |

**Ledger exclusions:** {"candidate_id": "mixed-hybrid-and-password", "reason": "A password recipient may not be combined with stronger recipient types", "invariant_violated": "SPEC §9.11 L1185; CRYPTO-018"}.
**Exclusions proposed in this protocol (need independent sign-off, CR2):** `mixed-hybrid-and-password`, `rewrap-only-removal`, `silent-evidence-drop`, `spec-table-as-written`.

**R0 checklist:** 1 status quo: `status-quo-single-password-only`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 5 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: `separate-recovery-archive-or-escrow`; 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `status-quo-single-password-only`, `multiple-password-stanzas`, `conversion-command-full-rekey`, `separate-recovery-archive-or-escrow`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-CRY-084 — Does the one-signature/three-binding composition (T1 signature/v1 transcript, addressing binding with archive ID, sign-then-encrypt embedding, exposed detached records) resist surreptitious forwarding, cross-context confusion and binding-mixing attacks as SPEC §25.2 requires?

Ledger status `EXTERNAL_REVIEW_REQUIRED`, blocker class `EXTERNAL_REVIEW`. Other experiments informing it: EXP-CRYPTO-021, EXP-CRYPTO-014, EXP-CRYPTO-023.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-three-binding-t1` | Status quo: one Ed25519 signature over T1 transcript of content, optional physical and addressing bindings | status quo | no | — |
| `suite-and-version-in-content-binding` | New version adding suite and crypto version to the content binding | ledger alternative | no | — |
| `sign-encrypt-sign` | Sign/encrypt/sign layering | ledger alternative | no | — |
| `header-hash-binding` | Sign the envelope header hash including commitment directly | ledger alternative | no | — |
| `separate-signature-per-binding` | Separate signatures per binding (first-pass two-scope design) | ledger alternative | no | — |
| `cose-sign1-protected-headers` | COSE_Sign1 with protected headers carrying binding context | ledger alternative | no | — |

**Ledger exclusions:** none recorded in the ledger.
**Exclusions proposed in this protocol (need independent sign-off, CR2):** `mixed-hybrid-and-password`, `rewrap-only-removal`, `silent-evidence-drop`, `spec-table-as-written`.

**R0 checklist:** 1 status quo: `status-quo-three-binding-t1`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 6 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `status-quo-three-binding-t1`, `suite-and-version-in-content-binding`, `sign-encrypt-sign`, `header-hash-binding`, `separate-signature-per-binding`, `cose-sign1-protected-headers`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-CRY-049 — What formal analysis and independent review of the key hierarchy, commitment, envelope MAC, recipient wrap, unlock order and AFK-sharing mutations must complete before production crypto, and in what form?

Ledger status `EXTERNAL_REVIEW_REQUIRED`, blocker class `EXTERNAL_REVIEW`. Other experiments informing it: EXP-CRYPTO-021, EXP-CRYPTO-011, EXP-CRYPTO-023.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-internal-adversarial-review` | Status quo: internal adversarial review AR-01..AR-34 only | status quo | no | — |
| `symbolic-model` | Symbolic model (Tamarin or ProVerif) of envelope, unwrap order, mutations and snapshot/storage adversaries | ledger alternative | no | — |
| `computational-proof` | Game-based proof or CryptoVerif model of the HKDF hierarchy and HMAC commitment | ledger alternative | no | — |
| `external-audit-without-model` | Third-party cryptographic audit of design and code without a mechanised model | ledger alternative | no | — |
| `combined-model-proof-and-audit` | Symbolic model plus computational argument plus external audit | ledger alternative | no | — |
| `defer-to-post-v1` | Ship v1 with documented residual risk and audit after release | defer / no change | no | — |

**Ledger exclusions:** {"candidate_id": "status-quo-internal-adversarial-review", "reason": "Internal review alone does not satisfy the routing of formal analysis to independent security review", "invariant_violated": "SPEC §25.2 L2559-2562"}; {"candidate_id": "defer-to-post-v1", "reason": "Production crypto requires an external cryptographic audit before release", "invariant_violated": "docs/crypto-review-v1.md L10-12"}.
**Exclusions proposed in this protocol (need independent sign-off, CR2):** `mixed-hybrid-and-password`, `rewrap-only-removal`, `silent-evidence-drop`, `spec-table-as-written`.

**R0 checklist:** 1 status quo: `status-quo-internal-adversarial-review`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 6 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: `defer-to-post-v1`; 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `status-quo-internal-adversarial-review`, `symbolic-model`, `computational-proof`, `external-audit-without-model`, `combined-model-proof-and-audit`, `defer-to-post-v1`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.
<!-- END AUTO:candidates -->

Evidence route: `EMPIRICALLY_MEASURED` matrix outcomes, root/binding conformance (some `FORMALLY_DERIVED` from root definitions), and nonce-uniqueness property tests (binary, HC-14). A written uniqueness argument and mutation-sequence resistance go to external review (EXP-CRYPTO-021). This matrix also provides the composition cells for legacy provenance/preservation decisions and container mutation decisions owned by sibling domains.

## 4. Candidates and arms

- **DEC-CRY-010** composition matrix: `status-quo-per-operation`, `normative-executable-matrix`, `refuse-untested-compositions`, `declared-loss-records`, `restricted-v1-subset` (`silent-evidence-drop` excluded at R1).
- **DEC-MOD-029** per-operation binding outcomes: `spec-table-as-written` (excluded at R1: AUX must change on `--add`), `derived-from-definitions`, `repo-refined-table`, `executable-matrix`.
- **DEC-CRY-054** mutation nonce strategy: `status-quo-reuse-payload-regenerate-control`, `full-reencryption-every-mutation`, `epoch-rotation-on-untrusted-input`, `session-key-per-invocation`.
- **DEC-CRY-050** rotation: `status-quo-full-rotation-rechunk`, `boundary-preserving-rotation`, `separate-boundary-key-new-version`, `lazy-rotation-on-repack` (`rewrap-only-removal` excluded at R1).
- **DEC-CRY-069** recipient-mutation semantics; **DEC-CRY-031** encrypted-to-encrypted repack; **DEC-CRY-045** recipient-set integrity; **DEC-CRY-105** unlock-on-failure; **DEC-ACC-049** repack edge cases (encrypted repack cell); **DEC-CON-016** legacy crypto-v1 acceptance window (composition with mutation); **DEC-LEG-040** rewrite provenance chain (AUX/signature effect cell); **DEC-CRY-057** (POST_V1) multiple password stanzas, recipient addition to password archives and protection-mode conversion — the mutation-contract and re-encryption-cost cells are measured here (offline-guessing analysis of multi-password archives is in EXP-CRYPTO-017), with `mixed-hybrid-and-password` excluded at R1 (no-mixing freeze).

Reference candidate: the status quo of each decision.

## 5. Corpus selectors

- **Matrix base archives (tuning):** small real trees `f01-tuning-curl-8-19-0-git`, `f04-tuning-sourcelike`, `f19-tuning-zoo`, `f17-tuning-duptree-mixed`, plus generated trees, in each of {plain, encrypted, signed, converted-from-legacy, preserved} initial states. `f20-tuning-generated-private-vault` supplies attacker-controlled input archives (crafted salts/ordinals).
- **Mutation sequences (tuning):** up to length 5 over {key add, key remove, change-password, epoch rotation, sign --embed, repack --profile, repack --strip-provenance, interrupted write}. 10,000 seeded sequences for the nonce-uniqueness property test.
- **Validation:** the matrix on `f01-validation-redis-7-2-16-git`, `f17-validation-duptree-vendor` and generated archives, one registered look.

## 6. Environment and platform requirements

`wsl-ubuntu`, `timing: false` (matrix and nonce uniqueness are deterministic given seeds and use a research-only instrumented build that logs (derived key id, nonce) pairs, F-24, with MVT-16(c)). Rotation-cost timing is deferred to EXP-CRYPTO-003. Interrupted-write cells use the kill injector of the HC-18 harness (planned); where the injector is unavailable, those cells are marked NEEDS_TOOLING.

## 7. Commands and tooling

- `ebr-crypto compose`: applies a mutation sequence, records outcome class, all four roots (from `ebound inspect --json` with keys), binding statuses, and the (key, nonce) log from the research build.
- Nonce logger: research-internals accessor that records every (derived-key identifier, nonce) a mutation emits, per MVT-14(c).
- Oracle: root/binding expectations derived from the SPEC §8.0/§8.1 definitions by a session not involved in the writer; the derived-from-definitions candidate is the reference oracle for roots.
- Kill injector (shared with the HC-18 / EXP-CRYPTO-010 tooling) for the interrupted-write cells.

## 8. Seeds, warmup, repetitions and adaptive rule

- Deterministic matrix: 2 repetitions plus repeat-hash of the outcome/root/binding records (encryption randomness differs per run, so roots that must differ are checked to differ and roots that must be stable are checked stable across the two runs).
- Nonce uniqueness: 10,000 sequences; the detection bound is about 95% at a 3×10⁻⁴ per-sequence defect rate (rule of three, §D.5), and the (key, nonce) set is checked exhaustively within the run.

## 9. Measurement method and metrics

| Metric (canonical) | ID | OD | Role |
|---|---|---|---|
| repeated (key, nonce) count | HC-14 (M14(c)) | OD-15 | **binary** (R1) |
| root-conformance fraction (LAI/AUX/PCR/PCI per cell vs oracle) | T-20 / binary | OD-20 (M20.1/M20.2) | **primary** / binary where identity-rule |
| binding-status correctness fraction | T-20 | OD-04/OD-15 | **primary** |
| outcome-class correctness (supported/refused/declared-loss vs oracle) | T-20 | OD-04 | **primary** |
| `migration_identity_conformance_fraction`, `migration_determinism_fraction` | HC-10/HC-13 | OD-20 | binary |
| leakage against removed recipients (boundary-preserving rotation) | `presence_advantage` (T-17) | OD-16 | secondary (cross-ref EXP-CRYPTO-002) |
| declared-loss-record completeness | `receipt_completeness_fraction` (M20.4) | OD-07 | binary |

## 10. Normalization and statistical analysis

- Binary identity-rule and nonce metrics: any failure is R1 with the committed cell/sequence.
- Fractions over the fixed matrix: T-20 exact; AGG-C headline (min over strata).
- Confirmatory W against S on validation for the graded fractions; MDE gate.
- Leakage against removed recipients uses the EXP-CRYPTO-002 pipeline.

## 11. Practical significance

T-20 and T-17 bands and §7.2 tiers from `research/methods/thresholds.json`. `separate-boundary-key-new-version` and `full-reencryption` differ in tier and cost; the executable matrix itself is a T1 test-and-doc change. Wire-frozen exclusions enforced at R1.

## 12. Sensitivity analysis

Not weight-based. Reported arms: initial archive state, mutation-sequence length, attacker-controlled versus locally produced input, and rotation contract. CB triggers evaluated.

## 13. Expected negative results worth recording

- The SPEC §8.1 table as written is inconsistent with the AUX definition for `--add` (DEC-MOD-029), so the derived-from-definitions table is required.
- Boundary-preserving rotation leaks re-used boundary secrets to removed recipients, a real cost against DEC-CRY-050's cheaper option.
- Some status-quo composition cells are untested, supporting the normative executable matrix (DEC-CRY-010).

## 14. Threats to validity

- Oracle derivation is a judgement; independently reviewed against SPEC lines.
- The research nonce logger must be byte-identical to production when off (MVT-16(c)).
- Interrupted-write cells depend on the kill injector; missing, they are NEEDS_TOOLING.
- Mutation-sequence resistance to splicing is external (EXP-CRYPTO-021); only uniqueness and outcomes are measured.

## 15. Held-out placeholder (Phase D)

Binary identity and nonce checks run on held-out on every split (R1). Graded fractions get one held-out look. No held-out item named here.

## 16. Cost estimate and agent effort

- Compute: about 60 core-hours (10,000 mutation sequences plus the matrix). Disk: about 25 GB transient.
- Agent effort: tooling **judgment** (compose driver, nonce logger, oracle, kill injector); execution **scripted**; analysis **judgment**.

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

_Generated by `python research/tools/experiments/fill_crypto_auto_blocks.py` for EXP-CRYPTO-015; edit the inputs, not this block._
<!-- END AUTO:common -->
