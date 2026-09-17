# EXP-CRYPTO-014: Signature and trust-semantics conformance matrix

<!-- BEGIN AUTO:meta -->
| Field | Value |
|---|---|
| Index status | **NEEDS_TOOLING** |
| Kind | decision |
| Domain | crypto (program §22.1, §22.4, §22.5) |
| Decisions informed | DEC-CRY-052, DEC-CRY-082, DEC-CRY-090, DEC-CRY-009, DEC-CRY-103, DEC-CRY-104, DEC-CRY-092, DEC-CRY-060, DEC-CRY-074, DEC-CRY-087, DEC-CRY-004, DEC-CRY-089, DEC-CRY-096, DEC-CRY-018, DEC-CRY-019, DEC-CRY-102, DEC-MOD-037, DEC-INT-038, DEC-MOD-027, DEC-MOD-029, DEC-ECO-063, DEC-INT-032, DEC-CRY-007, DEC-CRY-075, DEC-CRY-094, DEC-CRY-084 |
| Platforms | Windows+Linux/x86-64 |
| Requires timing | False |
| Estimates | 40 machine-hours; 20 GB disk |
| Tooling to build | ebr-crypto sigmatrix; oracle.csv; policy prototypes (allowed-keys, k-of-n, exact-byte signing) |
| Environment prerequisites | none beyond the harness and the ebr venv |
| Blocked arms | none |
| Specs | `spec.yaml` |
| Validation looks | owns look 1 for DEC-CRY-004, DEC-CRY-007, DEC-CRY-009, DEC-CRY-018, DEC-CRY-019, DEC-CRY-052, DEC-CRY-060, DEC-CRY-074, DEC-CRY-075, DEC-CRY-082, DEC-CRY-087, DEC-CRY-089, DEC-CRY-090, DEC-CRY-092, DEC-CRY-094, DEC-CRY-096, DEC-CRY-102, DEC-CRY-103, DEC-CRY-104, DEC-ECO-063, DEC-INT-032, DEC-MOD-029, DEC-MOD-037; participates in looks owned by other experiments for DEC-INT-038, DEC-MOD-027 |
| Gates | G-A and G-B unmet at this revision (generated `gate_state` column of `research/experiments/index.csv`); timing runs also need a PASS calibration record for the calibration identity (PA-04) |
| Conventions | `research/experiments/_index/crypto-conventions.md` (CR0-CR10); program amendments `research/experiments/_program/design-revision-round1.md` |
<!-- END AUTO:meta -->

## 1. Question

Across every read path and CLI command, do signature verdicts stay correct and distinguishable — cryptographic validity, per-binding VALID/STALE/INVALID/ABSENT/NOT_BOUND, authorized-signer, trusted-timestamp, and the keyless "PRIVATE CONTENT UNVERIFIED" level — and do the candidate policies (multi-signature evaluation, allowed-key sets, partial bindings, STREAM signing, exact-byte signing, no-signature reporting) behave as specified under repack, key changes and encryption? What does each binding actually attest?

## 2. Hypotheses and falsification

- **H1 (honesty).** No candidate ever reports a verification or authorization state stronger than the oracle state (HC-15, I25). `status-quo-any-key` satisfies `--require-content-signature` with an attacker-signed detached `.ebsig` over a modified archive (the DEC-CRY-082 attack test); `allowed-public-keys`/`require-key-with-require-flags` refuse it.
- **H2 (STALE vs INVALID).** Candidates that distinguish STALE (binding no longer matches, signature cryptographically valid) from INVALID (signature fails) do so correctly on every mutation; `spec-invalid-on-failure` collapses them.
- **H3 (multi-signature).** `status-quo-any-signer-per-binding`, `per-signer-conjunction` and `threshold-k-of-n` give the specified accept/deny across bindings-split-across-signers scenarios; a single invalid signature fails the set only under the status quo.
- **H4 (coverage).** Signatures over identity roots (LAI/AUX, PCR, addressing) do not detect a parser-differential physical-field mutation that leaves roots unchanged; an exact-byte (PCI/container) signature would (DEC-CRY-092, DEC-CRY-060).
- **Falsification:** any false-verified/false-authorized outcome (H1) is an R1 violation with the committed archive. H2-H4 are conformance verdicts over the fixed case matrix (T-20).

## 3. Decisions informed and evidence route

<!-- BEGIN AUTO:candidates -->
#### DEC-CRY-052 — With multiple signatures, how is policy evaluated: per binding across any signer (status quo), per signer (one signer satisfies all required bindings), k-of-n authorized signers, and does an invalid or malformed signature fail the set, get reported per signature, or count only if from an authorized key?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: none.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-any-signer-per-binding` | Status quo: independent any() per binding; any invalid fails all; malformed key aborts | status quo | yes | — |
| `per-signer-conjunction` | A single signer must satisfy every required binding | ledger alternative | yes | — |
| `threshold-k-of-n` | Require k distinct authorized signers | ledger alternative | yes | — |
| `invalid-reported-not-fatal` | Report invalid signatures per record; fail only if policy-relevant | ledger alternative | no | — |
| `malformed-record-per-signature-status` | Map malformed keys and unknown algorithms to per-signature statuses | ledger alternative | no | — |
| `principal-rules` | Per-principal rules as in ssh allowed_signers | ledger alternative | no | — |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: `status-quo-any-signer-per-binding`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 6 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `invalid-reported-not-fatal`, `malformed-record-per-signature-status`, `principal-rules`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-CRY-082 — Should library SignaturePolicy and CLI verify accept an allowed-key set (full public keys or signer IDs), per-key binding requirements and a require-timestamp flag, so that --require-*-signature means "from an authorized signer", and what is the default when no keys are supplied?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `HUMAN_PARTICIPANTS`. Other experiments informing it: EXP-CRYPTO-022, EXP-ECO-016, EXP-ECO-018, EXP-ECO-019, EXP-ECO-021.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-any-key` | Status quo: require flags satisfied by any valid signature | status quo | yes | — |
| `allowed-public-keys` | Allowed full public keys supplied by file or flag | ledger alternative | yes | EXP-ECO-016, EXP-ECO-018 |
| `allowed-signer-ids` | Allowed 16-byte signer IDs | ledger alternative | no | EXP-ECO-018 |
| `require-key-with-require-flags` | Refuse --require-*-signature unless an allowed-key set is supplied | ledger alternative | yes | EXP-ECO-018 |
| `policy-file` | Policy file naming signers, required bindings and timestamp requirements | ledger alternative | no | — |
| `warn-untrusted-signer` | Keep any-key semantics but print UNTRUSTED signer warnings | ledger alternative | no | — |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: `status-quo-any-key`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 6 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `policy-file`, `warn-untrusted-signer`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-CRY-090 — How do library and CLI keep cryptographically valid, binding current, authorized signer and trusted timestamp as separate verdicts, what combined summary (if any) is returned or printed, and what does verify report when no signatures exist?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-022, EXP-ECO-019, EXP-ECO-021, EXP-INT-003.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-three-dimensions` | Status quo: cryptographic, per-binding and timestamp statuses; no authorization; silent when none | status quo | no | EXP-INT-003 |
| `add-authorization-dimension` | Add AUTHORIZED/UNKNOWN_SIGNER derived from caller key policy | ledger alternative | no | EXP-INT-003 |
| `overall-verdict-enum` | Add summary verdicts such as TRUSTED_CURRENT, VALID_UNTRUSTED, STALE, INVALID | ledger alternative | no | EXP-INT-003 |
| `structured-json-only` | Structured JSON statuses with no summary word | ledger alternative | no | EXP-INT-003 |
| `explicit-no-signatures-line` | Print "no signatures present" whenever signatures are requested and none exist | ledger alternative | no | EXP-INT-003 |
| `verdict-words-require-policy` | Print VALID/TRUSTED wording only when a trust policy was supplied | ledger alternative | no | EXP-CRYPTO-022, EXP-INT-003 |
| `silent-when-no-signatures` | Print nothing about signatures when none exist | ledger alternative | yes | EXP-INT-003 |

**Ledger exclusions:** {"candidate_id": "silent-when-no-signatures", "reason": "Omitting the no-signature statement lets an unsigned archive look signature-verified", "invariant_violated": "SPEC §17.3 L1891 no-signature reporting invariant"}.

**R0 checklist:** 1 status quo: `status-quo-three-dimensions`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 7 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-CRY-009 — Should signers be able to create partial-binding signatures (content-only, or content plus addressing without physical) through the CLI, with --bind-physical replaced by a working opt-out, or should the frozen "sign every available binding" policy stand and the dead flag be removed?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-022, EXP-ECO-018, EXP-ECO-019, EXP-ECO-021.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-all-bindings-dead-flag` | Status quo: sign all available bindings; --bind-physical no-op | status quo | no | — |
| `no-bind-physical-flag` | Add --no-bind-physical (and --no-bind-addressing) | ledger alternative | no | EXP-ECO-018 |
| `explicit-binding-list` | Explicit --bind content,physical,addressing list | ledger alternative | no | EXP-ECO-018 |
| `remove-dead-flag-library-only` | Remove --bind-physical; partial bindings via library only | ledger alternative | no | — |
| `profile-based-defaults` | Defaults chosen by signing purpose (release versus backup) | ledger alternative | no | — |
| `physical-only-signature` | Allow signatures without the CONTENT binding | ledger alternative | yes | — |

**Ledger exclusions:** {"candidate_id": "physical-only-signature", "reason": "Binding mask must always include CONTENT", "invariant_violated": "crypto-v1 wire frozen: mask bit 0 CONTENT mandatory (docs/crypto-wire-v1.md L78-81)"}.

**R0 checklist:** 1 status quo: `status-quo-all-bindings-dead-flag`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 6 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `status-quo-all-bindings-dead-flag`, `remove-dead-flag-library-only`, `profile-based-defaults`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-CRY-103 — How does a v1 verifier treat signature records with unknown version, algorithm, mask bits or critical attributes: abort the open (status quo for embedded), per-signature UNSUPPORTED status, fail closed via new required feature bits, or ignore-but-authenticate as optional features; and is SignatureMethod a second guarded extensibility seam?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CONF-004, EXP-EVAL-008.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-error-aborts` | Status quo: unknown version or algorithm is an error; embedded decode aborts open | status quo | no | — |
| `per-signature-unsupported` | Decode envelope fields and report CryptographicStatus::Unsupported per record | ledger alternative | no | — |
| `feature-bit-per-algorithm` | New required feature bit per algorithm; old readers fail closed | ledger alternative | no | — |
| `optional-feature-authenticated` | Optional feature: old readers skip but authenticate unknown signatures | ledger alternative | no | — |
| `critical-attribute-flag` | OpenPGP-style critical flag for signed attributes in a new version | ledger alternative | no | — |
| `separate-collection-per-algorithm` | Separate EBCS collection per signature algorithm | ledger alternative | no | — |
| `ignore-and-report-verified` | Ignore unknown signatures and report the archive as signature-verified | ledger alternative | yes | — |

**Ledger exclusions:** {"candidate_id": "ignore-and-report-verified", "reason": "Presents unverifiable signatures as verified", "invariant_violated": "I25"}.

**R0 checklist:** 1 status quo: `status-quo-error-aborts`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 7 ledger candidates listed above; 4 strongest incumbent: `critical-attribute-flag`; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `status-quo-error-aborts`, `per-signature-unsupported`, `feature-bit-per-algorithm`, `optional-feature-authenticated`, `critical-attribute-flag`, `separate-collection-per-algorithm`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-CRY-104 — When no known stanza matches and unknown recipient stanza types are present, what outcome class and detail does a reader report, and must unknown-type stanzas be represented in the private recipient directory? The general extension contract, including class enforcement and feature bits, is decided in recipient-stanza-extension-contract.

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-016.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-no-match-no-entry` | Status quo: NO_MATCHING_RECIPIENT; unknown-type stanzas have no directory entry | status quo | no | — |
| `unsupported-with-type-detail` | Report UNSUPPORTED naming unknown stanza types when no known stanza matches | ledger alternative | no | — |
| `qualified-no-match` | Keep NO_MATCHING_RECIPIENT but add a non-secret note that unsupported stanza types were present | ledger alternative | no | — |
| `generic-directory-entries` | Require generic directory entries for unknown-type stanzas in future writers | ledger alternative | no | — |
| `forbid-unknown-in-v1` | Reject unknown stanza types in crypto v1 archives | ledger alternative | no | — |
| `strip-unknown-stanzas` | Drop unknown stanzas before authentication | ledger alternative | no | EXP-CRYPTO-016 |

**Ledger exclusions:** {"candidate_id": "strip-unknown-stanzas", "reason": "Unknown stanzas must remain covered by envelope MAC and recipient-set digest", "invariant_violated": "I12; CRYPTO-014/016"}.

**R0 checklist:** 1 status quo: `status-quo-no-match-no-entry`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 6 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `status-quo-no-match-no-entry`, `unsupported-with-type-detail`, `qualified-no-match`, `generic-directory-entries`, `forbid-unknown-in-v1`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-CRY-092 — Are signatures over identity roots (LAI/AUX, PCR, addressing context) sufficient, or must Entrybound also offer exact-byte signing (PCI or container digest) to address parser-differential and unsigned-physical-layer concerns and user expectations?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: none.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-roots-only` | Status quo: content, physical and addressing bindings; PCI unbound | status quo | no | — |
| `pci-binding-new-version` | Add an optional PCI binding in a future record version | ledger alternative | no | — |
| `external-exact-file-signatures` | Document minisign/cosign over exact bytes as the exact-file path | ledger alternative | no | — |
| `dual-roots-and-bytes` | Emit both a root signature and an exact-byte signature | ledger alternative | no | — |
| `rely-on-single-interpretation` | Rely on I15 strict canonical parsing to close differentials | ledger alternative | no | — |
| `container-bytes-only` | Sign container bytes instead of roots | ledger alternative | yes | — |

**Ledger exclusions:** {"candidate_id": "container-bytes-only", "reason": "Signatures would not survive repack and contradict the frozen root-descriptor design", "invariant_violated": "SPEC §9.6 L1082"}.

**R0 checklist:** 1 status quo: `status-quo-roots-only`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 6 ledger candidates listed above; 4 strongest incumbent: `external-exact-file-signatures`; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `status-quo-roots-only`, `pci-binding-new-version`, `external-exact-file-signatures`, `dual-roots-and-bytes`, `rely-on-single-interpretation`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-CRY-060 — What exactly does a VALID physical (PCR) binding attest, and must decode-path records (TransformPlans, Dictionaries, reconstruction data) be covered by some signed binding?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-021.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-pcr-only` | Status quo: PCR only; scope undocumented | status quo | no | — |
| `rely-on-content-binding` | Document that decoded output is verified against LAI-covered logical digests | ledger alternative | no | — |
| `decode-path-digest-binding-new-version` | Add a decode-path digest binding in a new version | ledger alternative | no | — |
| `plans-in-pcr` | Redefine PCR to include plans and dictionaries (identity change) | ledger alternative | no | — |
| `document-and-test-only` | Document scope and add substitution tests without format change | ledger alternative | no | — |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: `status-quo-pcr-only`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 5 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `status-quo-pcr-only`, `rely-on-content-binding`, `decode-path-digest-binding-new-version`, `plans-in-pcr`, `document-and-test-only`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-CRY-074 — When repacking a signed archive (profile change, strip-provenance), are signatures carried with per-binding STALE reporting, dropped only with --drop-signatures, refused, or re-created, and does --drop-signatures apply only to signatures whose bindings would go stale?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: none.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-no-handling` | Status quo: repack ignores signatures | status quo | no | — |
| `carry-and-report-stale` | Carry embedded signatures and report per-binding status | ledger alternative | no | — |
| `drop-all-with-flag` | --drop-signatures drops all signatures | ledger alternative | no | — |
| `drop-invalidated-only` | Drop only signatures whose bindings the operation stales | ledger alternative | no | — |
| `refuse-signed-input` | Refuse repack of signed archives without an explicit choice | ledger alternative | no | — |
| `re-sign-with-supplied-key` | Re-sign output with a supplied key | ledger alternative | no | — |
| `silently-drop` | Drop signatures silently | ledger alternative | no | — |

**Ledger exclusions:** {"candidate_id": "silently-drop", "reason": "Signature loss without the required flag", "invariant_violated": "SPEC §17.5 L1961 --drop-signatures requirement"}.

**R0 checklist:** 1 status quo: `status-quo-no-handling`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 7 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `status-quo-no-handling`, `carry-and-report-stale`, `drop-all-with-flag`, `drop-invalidated-only`, `refuse-signed-input`, `re-sign-with-supplied-key`, `silently-drop`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-CRY-087 — After key add, key remove, password change or epoch rotation, what happens to embedded signatures whose addressing (and possibly physical) bindings become stale: carry unchanged (status quo), drop with explicit flag, refuse without a flag, re-sign in the same transaction with a supplied key, or report per signature in mutation output?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-ECO-019.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-carry-stale` | Status quo: carry signatures unchanged; key add warns addressing may be stale | status quo | no | — |
| `drop-with-flag` | Drop stale signatures only with explicit --drop-signatures | ledger alternative | no | — |
| `refuse-without-flag` | Refuse mutation of signed archives unless the caller chooses carry or drop | ledger alternative | no | — |
| `re-sign-in-transaction` | Accept --signing-key and re-sign within the mutation | ledger alternative | no | — |
| `report-per-signature` | Carry and print per-signature binding statuses in mutation output | ledger alternative | no | — |
| `drop-silently` | Drop stale signatures silently | ledger alternative | no | — |
| `automatic-re-sign` | Re-sign automatically with a stored key | ledger alternative | yes | — |

**Ledger exclusions:** {"candidate_id": "drop-silently", "reason": "Discards signatures without the required explicit flag", "invariant_violated": "SPEC §17.5 L1961 --drop-signatures requirement"}; {"candidate_id": "automatic-re-sign", "reason": "Entrybound never re-signs automatically", "invariant_violated": "docs/signing-key-management-v1.md L77 no automatic re-signing freeze"}.

**R0 checklist:** 1 status quo: `status-quo-carry-stale`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 7 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `status-quo-carry-stale`, `drop-with-flag`, `refuse-without-flag`, `re-sign-in-transaction`, `report-per-signature`, `drop-silently`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-CRY-004 — Should verify report whether a stale content binding comes from an LAI change or an AUX-only change (comparing stored transcript fields), should AUX become a separate binding in a future version, or should SPEC simply be corrected to the single content binding?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-022.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-single-content-status` | Status quo: one content status | status quo | no | — |
| `diagnose-from-stored-values` | Keep one binding; report which of LAI or AUX differs | ledger alternative | no | — |
| `separate-aux-binding-new-version` | Separate AUX binding bit in a new record version | ledger alternative | no | — |
| `correct-spec-only` | Correct SPEC §8.0 and §17.8 text without behaviour change | SPEC design | no | — |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: `status-quo-single-content-status`; 2 SPEC design: `correct-spec-only`; 3 ledger alternatives: 4 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `status-quo-single-content-status`, `diagnose-from-stored-values`, `separate-aux-binding-new-version`, `correct-spec-only`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-CRY-089 — Must payload_suite_id and crypto version be signed in every signature over an encrypted archive: require addressing for encrypted signing, move suite into the content binding in a new version, argue that commitment and envelope MAC already bind it, or amend SPEC agility rule 7?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-021, EXP-CRYPTO-023.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-suite-in-optional-addressing` | Status quo: suite signed only in optional addressing binding | status quo | no | — |
| `require-addressing-for-encrypted` | Library refuses encrypted-archive signatures without ADDRESSING | ledger alternative | no | — |
| `suite-in-content-binding-new-version` | Add suite to content binding in a new version | ledger alternative | no | — |
| `rely-on-commitment` | Document that commitment and envelope MAC bind the suite | ledger alternative | no | — |
| `amend-rule-7` | Amend SPEC rule 7 to cover only mandatory bindings | SPEC design | no | — |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: `status-quo-suite-in-optional-addressing`; 2 SPEC design: `amend-rule-7`; 3 ledger alternatives: 5 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `status-quo-suite-in-optional-addressing`, `require-addressing-for-encrypted`, `suite-in-content-binding-new-version`, `rely-on-commitment`, `amend-rule-7`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-CRY-096 — Should v1 signing (the sign command, detached and embedded signatures) support STREAM-layout archives, or stay INDEXED-only with repack to INDEXED as the documented remedy?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: none.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-indexed-only-sign` | Status quo: sign opens unencrypted archives only with the INDEXED reader, so STREAM archives cannot be signed via the CLI although detached STREAM verification works | status quo | no | — |
| `detached-sign-via-sequential-reader` | Allow sign --detached over STREAM archives by computing the signed roots with the single-pass sequential reader | ledger alternative | no | — |
| `embedded-stream-signature-item` | Define an embedded signature placement for STREAM archives (for example a trailing signed item before the footer) | ledger alternative | no | — |
| `refuse-with-repack-remedy` | Refuse STREAM signing with a stable reason code and document repack to INDEXED as the remedy | ledger alternative | no | — |
| `defer-post-v1` | Defer STREAM signing support until after v1 | defer / no change | no | — |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: `status-quo-indexed-only-sign`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 5 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: `defer-post-v1`; 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `status-quo-indexed-only-sign`, `detached-sign-via-sequential-reader`, `embedded-stream-signature-item`, `refuse-with-repack-remedy`, `defer-post-v1`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-CRY-018 — What conventions govern detached signatures: .ebsig extension and media type registration, discovery and association with the archive, several signers as several files, and uniqueness of embedded records per signed transcript?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-ECO-007, EXP-ECO-012.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-single-record-default-path` | Status quo: <archive>.ebsig default, exclusive create, one record | status quo | no | EXP-ECO-007 |
| `per-signer-suffix-files` | Multiple files named by signer ID suffix | ledger alternative | no | EXP-ECO-007 |
| `media-type-registration` | Register .ebsig media type and extension | ledger alternative | no | EXP-ECO-007 |
| `multi-record-ebsig-v1` | Allow several records in one .ebsig | ledger alternative | no | EXP-ECO-007 |
| `bundle-new-version` | Signature bundle container in a future version | ledger alternative | no | EXP-ECO-007 |
| `embed-uniqueness-per-transcript` | Refuse embedded records sharing a signed transcript | ledger alternative | no | EXP-ECO-007 |
| `sigstore-bundle-compatible-sidecar` | Offer a Sigstore-bundle-compatible sidecar | ledger alternative | no | EXP-ECO-007 |

**Ledger exclusions:** {"candidate_id": "multi-record-ebsig-v1", "reason": "A v1 .ebsig is exactly one record", "invariant_violated": "crypto-v1 wire frozen (docs/crypto-wire-v1.md L710-711)"}.

**R0 checklist:** 1 status quo: `status-quo-single-record-default-path`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 7 ledger candidates listed above; 4 strongest incumbent: `sigstore-bundle-compatible-sidecar`; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-CRY-019 — For encrypted archives, should the CLI default to embedded signatures, warn about or refuse detached signatures that expose signer and plaintext-derived roots, or add a privacy-preserving detached form?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-007, EXP-ECO-007, EXP-ECO-016.

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

#### DEC-CRY-102 — Should unencrypted native archives gain a normative embedded-signature placement (a signature section outside identity roots with 0x200 semantics) in v1 or later, or remain detached-only?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-EVAL-008.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-detached-only` | Status quo: unencrypted archives support only .ebsig | status quo | no | — |
| `trailing-signature-section` | Signature section after roots, before footer, excluded from LAI/PCR/AUX | ledger alternative | no | — |
| `indexed-signature-set-section` | SignatureSet section in INDEXED layout | ledger alternative | no | — |
| `stream-trailer-signatures` | Trailer signature records for STREAM layout | ledger alternative | no | — |
| `defer-post-v1` | Defer to a later format version | defer / no change | no | — |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: `status-quo-detached-only`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 5 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: `defer-post-v1`; 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `status-quo-detached-only`, `trailing-signature-section`, `indexed-signature-set-section`, `stream-trailer-signatures`, `defer-post-v1`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-MOD-037 — Can one archive carry signatures under different identity profiles, and how does verify report them?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: none.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `single-profile-status-quo` | Status quo: content binding hardcodes identity/v1 | status quo | no | — |
| `per-signature-profile` | Each signature names its profile; archive may hold several | ledger alternative | no | — |
| `archive-profile-only` | Signatures must use the archive declared profile | ledger alternative | no | — |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: `single-profile-status-quo`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 3 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `single-profile-status-quo`, `per-signature-profile`, `archive-profile-only`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-INT-038 — What is the complete signature status vocabulary (cryptographic validity, per-binding VALID/STALE/INVALID/ABSENT/NOT_BOUND), what evidence distinguishes STALE from INVALID, and what stale-binding policy is the CLI default?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-ECO-019, EXP-ECO-021, EXP-INT-003.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-stale-on-failure` | Status quo: cryptographic VALID/INVALID/UNSUPPORTED; bindings VALID/STALE/NOT_BOUND; failed signature reports bindings STALE | status quo | no | EXP-INT-003 |
| `spec-invalid-on-failure` | Suite spec: signature failure makes every binding INVALID; VALID/STALE only after cryptographic validity | SPEC design | yes | EXP-INT-003 |
| `add-not-verifiable` | Add NOT_VERIFIABLE for bindings whose current values could not be computed (partial reads) | ledger alternative | no | EXP-INT-003 |
| `default-require-none` | CLI default reports statuses without requiring any binding | ledger alternative | no | EXP-INT-003 |
| `default-require-content-current` | CLI default requires a current content binding when signatures are present | ledger alternative | no | EXP-INT-003 |
| `default-require-all-present-current` | CLI default requires all present bindings current | ledger alternative | no | EXP-INT-003 |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: `status-quo-stale-on-failure`; 2 SPEC design: `spec-invalid-on-failure`; 3 ledger alternatives: 6 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-MOD-027 — Exactly which parameters must the LAI, PCR and AUX descriptors bind (algorithm id, namespace, format version, identity profile, archive role, entry count, total logical size, chunker id, planner id, transform plans, dictionaries), and is the current field set frozen for v1?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CONF-002, EXP-CONF-012, EXP-EVAL-008.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-fields` | LAI: sha256, manifest root, identity/v1, role, entry count, total logical size; PCR: sha256, (logical_digest, chunk_root) list, unique chunk count, chunker_id; AUX: sha256, entry-aux root, fidelity digest, conversion digest, optional preservation wrapper | status quo | no | — |
| `add-namespace-format-version` | Additionally bind ECF namespace and format major/minor into LAI (currently only in signature content binding) | ledger alternative | no | — |
| `pcr-binds-plans` | Bind TransformPlans/dictionaries/reconstruction into PCR so a signed physical binding authenticates the decode path | ledger alternative | no | — |
| `pcr-binds-planner-id` | Bind planner_id into PCR | ledger alternative | no | — |
| `pcr-binds-per-object-counts` | Bind per-ContentObject chunk count and logical length alongside each chunk_root | ledger alternative | no | — |
| `lai-omits-counts` | Drop entry count/total size from LAI as derivable from the manifest (I19 purity) | ledger alternative | no | — |
| `separate-descriptor-per-root-record` | Publish each descriptor as an explicit canonical record rather than an implicit structured-hash field list | ledger alternative | no | — |
| `lai-binds-codec-or-chunking` | Bind codec, chunking or encryption parameters into LAI (byte-identity-like logical root) | ledger alternative | no | — |

**Ledger exclusions:** {"candidate_id": "lai-binds-codec-or-chunking", "reason": "Binding codec, chunking or encryption parameters into LAI", "invariant_violated": "I8, I9, I28"}.

**R0 checklist:** 1 status quo: `status-quo-fields`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 8 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `status-quo-fields`, `add-namespace-format-version`, `pcr-binds-plans`, `pcr-binds-planner-id`, `pcr-binds-per-object-counts`, `lai-omits-counts`, `separate-descriptor-per-root-record`, `lai-binds-codec-or-chunking`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-MOD-029 — What are the exact LAI/PCR/AUX/PCI and signature-binding outcomes for every archive operation, including operations omitted from SPEC §8.1 and the inconsistent --add/content-change AUX cells, and are they enforced by tests?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CONF-004, EXP-CRYPTO-015.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `spec-table-as-written` | SPEC §8.1 table as written (AUX unchanged for --add) | SPEC design | no | EXP-CRYPTO-015 |
| `derived-from-definitions` | Recompute every cell from root definitions (AUX changes whenever entry set or aux metadata changes) | ledger alternative | no | EXP-CRYPTO-015 |
| `repo-refined-table` | Repo docs refinements for rekey/key add/remove plus derived cells | ledger alternative | no | EXP-CRYPTO-015 |
| `executable-matrix` | Normative executable test matrix covering all operations and bindings | ledger alternative | no | EXP-CRYPTO-015 |

**Ledger exclusions:** {"candidate_id": "spec-table-as-written", "reason": "AUX binds a Merkle root over all entry aux_digests, so adding entries must change it", "invariant_violated": "SPEC §8.0 L876 AUX definition"}.

**R0 checklist:** 1 status quo: no ledger id marked status quo; the critic pass confirms which candidate is the behaviour at `9e44608`; 2 SPEC design: `spec-table-as-written`; 3 ledger alternatives: 4 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: `executable-matrix`.

#### DEC-ECO-063 — Should the CLI let signers choose which bindings to sign (CONTENT only, CONTENT+PHYSICAL, +ADDRESSING), and how are the consequences for repack and recipient changes shown?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-022, EXP-ECO-018, EXP-ECO-019.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-all-bindings` | Sign every available binding; --bind-* only confirm defaults | status quo | no | — |
| `content-only-opt-out` | --content-only flag omitting PHYSICAL and ADDRESSING | ledger alternative | no | EXP-ECO-018 |
| `explicit-binding-list` | --bind content,physical,addressing required | ledger alternative | no | EXP-ECO-018 |
| `policy-presets` | Named signing policies (distribution, archival) | ledger alternative | no | EXP-ECO-018 |
| `library-only-opt-out` | Keep CLI defaults; opt-out only via library | ledger alternative | no | — |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: `status-quo-all-bindings`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 5 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `status-quo-all-bindings`, `library-only-opt-out`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-INT-032 — Is role participation in LAI, plus ContentRef kind in entry identity, sufficient against content-stripping and role-confusion attacks on signed archives, or must signatures or identities also bind base identities and sidecar describes_lai claims?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-021, EXP-INT-007.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-role-in-lai` | Status quo: LAI binds role byte; ContentRef kind in entry identity | status quo | no | EXP-INT-007 |
| `bind-base-identities` | Incremental LAI additionally binds referenced base identities | ledger alternative | no | EXP-INT-007 |
| `signature-explicit-role` | Signature transcript binds role explicitly | ledger alternative | no | EXP-INT-007 |
| `signed-requires-complete` | Only Complete archives may carry content signatures | ledger alternative | no | EXP-INT-007 |
| `describes-lai-separation` | Sidecar describes_lai carried as a separate signed claim, never conflated | defer / no change | no | EXP-INT-007 |
| `role-outside-identity` | Role declared in preamble only, not identity-bearing | ledger alternative | no | EXP-INT-007 |

**Ledger exclusions:** {"candidate_id": "role-outside-identity", "reason": "Reopens the content-deletion attack closed by binding role into LAI", "invariant_violated": "SPEC §8.5 L963 role in LAI (review-pass fix)"}.

**R0 checklist:** 1 status quo: `status-quo-role-in-lai`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 6 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: `describes-lai-separation`; 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-CRY-007 — When the ECF format version, namespace or identity profile changes, how do existing signatures and crypto transcripts behave: stale content signatures (status quo), bind only the identity-profile version, a signature version 2 with evolvable content binding, verifier equivalence tables; and do crypto transcripts keep 0.1 constants or follow the Descriptor?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-EVAL-008.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-format-version-in-binding` | Status quo: format major/minor inside content binding; transcripts hard-code 0.1 | status quo | no | — |
| `identity-profile-only-new-version` | Bind identity profile version but not container format version (new version) | ledger alternative | no | — |
| `signature-v2-namespaced-profile` | Signature record v2 carrying namespace and profile as parameters | ledger alternative | no | — |
| `verifier-equivalence-table` | Verifier maps old format versions to equivalent current bindings | ledger alternative | no | — |
| `crypto-transcripts-keep-constants` | Crypto transcripts stay at 0.1 constants for suite 1 forever | ledger alternative | no | — |
| `crypto-transcripts-follow-descriptor` | Crypto transcripts read Descriptor format version | ledger alternative | no | — |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: `status-quo-format-version-in-binding`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 6 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `status-quo-format-version-in-binding`, `identity-profile-only-new-version`, `signature-v2-namespaced-profile`, `verifier-equivalence-table`, `crypto-transcripts-keep-constants`, `crypto-transcripts-follow-descriptor`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-CRY-075 — When a signature algorithm or crypto suite is retired because it is broken, do readers verify with only DEPRECATED, report signatures untrusted after a retirement date, allow caller policy refusal, or accept only signatures whose trusted timestamp predates the break?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-EVAL-008.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-undefined` | Status quo: no rule | status quo | no | — |
| `decode-with-deprecated` | Verify normally with DEPRECATED diagnostic | ledger alternative | no | — |
| `untrusted-after-retirement` | Report signatures UNTRUSTED after a registry retirement date | ledger alternative | yes | — |
| `caller-policy-refusable` | Caller policy can refuse retired algorithms | ledger alternative | no | — |
| `timestamp-gated-acceptance` | Accept only with a trusted timestamp predating the break | ledger alternative | no | — |
| `refuse-to-decode` | Refuse to decode archives using retired algorithms | ledger alternative | no | — |

**Ledger exclusions:** {"candidate_id": "refuse-to-decode", "reason": "Readers must decode retired identifiers", "invariant_violated": "SPEC §20.4 L2217"}.

**R0 checklist:** 1 status quo: `status-quo-undefined`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 6 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `status-quo-undefined`, `decode-with-deprecated`, `caller-policy-refusable`, `timestamp-gated-acceptance`, `refuse-to-decode`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-CRY-094 — What does a valid Entrybound signature assert about identity: bare self-signed keys with the caller deciding (status quo), TOFU pinning, local keyring or allowed-signers files, X.509 trusted-publisher certificates, or an external identity layer, and how does verify present each?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-022, EXP-ECO-019, EXP-ECO-021.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-bare-keys-caller-decides` | Status quo: raw Ed25519 keys, 16-byte signer id printed, no trust input | status quo | no | — |
| `tofu-pinning` | Trust on first use pinned per archive name or source | ledger alternative | no | — |
| `allowed-signers-file` | ssh-keygen -Y style allowed-signers or minisign public-key files | ledger alternative | no | — |
| `local-keyring` | Local keyring with labels and trust levels | ledger alternative | no | — |
| `x509-publisher-certificates-v1-record` | Carry X.509 publisher certificates inside SignatureRecordV1 | ledger alternative | no | — |
| `x509-publisher-certificates-new-version` | Add certificate chains in a future signature record version | ledger alternative | no | — |
| `external-identity-sidecar` | Optional Sigstore/OIDC identity sidecar alongside offline signature | ledger alternative | no | — |
| `keyless-required` | Require Sigstore keyless identity for trust | ledger alternative | no | — |
| `openpgp-web-of-trust` | Delegate signer identity to OpenPGP keys and web of trust | ledger alternative | no | — |

**Ledger exclusions:** {"candidate_id": "x509-publisher-certificates-v1-record", "reason": "SignatureRecordV1 has no certificate field; adding one changes the frozen record", "invariant_violated": "crypto-v1 wire frozen (docs/crypto-wire-v1.md)"}; {"candidate_id": "keyless-required", "reason": "Trust would depend on live CA, identity provider and transparency log", "invariant_violated": "SPEC §24 item 7 L2509 verification never requires live infrastructure"}.

**R0 checklist:** 1 status quo: `status-quo-bare-keys-caller-decides`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 9 ledger candidates listed above; 4 strongest incumbent: `allowed-signers-file`, `external-identity-sidecar`, `keyless-required`, `openpgp-web-of-trust`; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `status-quo-bare-keys-caller-decides`, `tofu-pinning`, `allowed-signers-file`, `local-keyring`, `x509-publisher-certificates-v1-record`, `x509-publisher-certificates-new-version`, `external-identity-sidecar`, `keyless-required`, `openpgp-web-of-trust`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-CRY-084 — Does the one-signature/three-binding composition (T1 signature/v1 transcript, addressing binding with archive ID, sign-then-encrypt embedding, exposed detached records) resist surreptitious forwarding, cross-context confusion and binding-mixing attacks as SPEC §25.2 requires?

Ledger status `EXTERNAL_REVIEW_REQUIRED`, blocker class `EXTERNAL_REVIEW`. Other experiments informing it: EXP-CRYPTO-021, EXP-CRYPTO-015, EXP-CRYPTO-023.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-three-binding-t1` | Status quo: one Ed25519 signature over T1 transcript of content, optional physical and addressing bindings | status quo | no | — |
| `suite-and-version-in-content-binding` | New version adding suite and crypto version to the content binding | ledger alternative | no | — |
| `sign-encrypt-sign` | Sign/encrypt/sign layering | ledger alternative | no | — |
| `header-hash-binding` | Sign the envelope header hash including commitment directly | ledger alternative | no | — |
| `separate-signature-per-binding` | Separate signatures per binding (first-pass two-scope design) | ledger alternative | no | — |
| `cose-sign1-protected-headers` | COSE_Sign1 with protected headers carrying binding context | ledger alternative | no | — |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: `status-quo-three-binding-t1`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 6 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `status-quo-three-binding-t1`, `suite-and-version-in-content-binding`, `sign-encrypt-sign`, `header-hash-binding`, `separate-signature-per-binding`, `cose-sign1-protected-headers`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.
<!-- END AUTO:candidates -->

Evidence route: `EMPIRICALLY_MEASURED` conformance over a pre-registered case matrix (T-20), with the honesty parts binary (HC-15). Human-comprehension of verdict wording is HUMAN-FACING and routed to EXP-CRYPTO-022 (simulated first-use) and external/human review; only the mechanical message census and outcome correctness are decided here.

## 4. Candidates and arms

Grouped by decision (candidate ids from the ledger):
- **DEC-CRY-052** multi-signature policy; **DEC-CRY-082** allowed-key set and default; **DEC-CRY-090 / DEC-INT-038** verify vocabulary and required-binding defaults; **DEC-CRY-009 / DEC-ECO-063** partial-binding signing; **DEC-CRY-103** unknown signature version/algorithm handling; **DEC-CRY-104 / DEC-CRY-105** unknown/failed stanza and unlock-on-failure reporting; **DEC-CRY-074 / DEC-CRY-087** signature carry/drop/re-sign on repack and key changes; **DEC-CRY-092 / DEC-CRY-060** exact-byte versus root signing and PCR scope; **DEC-CRY-089** suite/version in the signed transcript; **DEC-CRY-096** STREAM signing; **DEC-CRY-004** stale-binding LAI-vs-AUX diagnosis; **DEC-CRY-018** detached `.ebsig` conventions; **DEC-CRY-019** embedded default for encrypted; **DEC-CRY-102** embedded-signature placement for unencrypted; **DEC-MOD-037** multi-profile signatures; **DEC-INT-032** role-confusion binding (signature-coverage test only; formal part external); **DEC-MOD-027 / DEC-MOD-029** descriptor binding fields and per-operation binding outcomes (signature-coverage cells); **DEC-CRY-007** signature and crypto-transcript behaviour when the format version, namespace or identity profile changes (stale content vs identity-profile-only binding vs verifier equivalence table); **DEC-CRY-075** verification of signatures made under a retired/broken algorithm (DEPRECATED-decode vs untrusted-after-retirement vs timestamp-gated acceptance; readers must still decode retired identifiers, SPEC §20.4); **DEC-CRY-094** what a valid signature asserts about identity (bare keys, TOFU, allowed-signers file, local keyring, X.509 publisher certs) — the mechanical verify-output presentation of each trust model is measured here, while human trust-conclusion correctness is HUMAN-FACING (EXP-CRYPTO-022) and any live-infrastructure model is excluded at R1.

Reference candidate: the status quo of each decision. Excluded-at-R1 candidates (for example `physical-only-signature`, `silent-when-no-signatures`, `ignore-and-report-verified`, `automatic-re-sign`, `container-bytes-only`) are recorded with their counterexamples and not run.

## 5. Corpus selectors

- **Fixed case matrix (tuning-analogous, deterministic):** the Cartesian product of {archive type: unsigned, signed-detached, signed-embedded, encrypted-signed} × {mutation: none, repack --profile, repack --strip-provenance, key add, key remove, change-password, rechunk, content edit, AUX-only edit, parser-differential physical mutation, cross-archive splice, attacker re-sign} × {signer set: single, two-signers-split-bindings, k-of-n, unauthorized, unknown-algorithm} × {policy: none, require-content, require-all, allowed-keys}. Case list frozen before results.
- **Base archives:** small real trees `f01-tuning-curl-8-19-0-git`, `f04-tuning-sourcelike`, `f19-tuning-zoo`, plus generated trees, packed INDEXED and STREAM, encrypted and plain.
- **Validation:** the same matrix on `f01-validation-redis-7-2-16-git`, `f19-validation-deep` and generated archives, one registered look, for the graded fractions.

## 6. Environment and platform requirements

`wsl-ubuntu` and `windows-host` (key-file permission behaviour differs by OS; F-0001 metadata differs). `timing: false` for conformance. Test signing keys and recipients are generated per case (seeded). Passwords are supplied through the harness (not the CLI prompt) via the `.testpassword` sidecar mechanism the harness already uses.

## 7. Commands and tooling

- `ebr-crypto sigmatrix`: drives `ebound sign/verify/repack/key`, applies each mutation, and records the full verdict object (cryptographic status, per-binding status, authorized-signer, timestamp status, exit code, returned-bytes state) against the pre-registered oracle.
- Research patches for candidate policies (allowed-key sets, k-of-n, partial bindings, exact-byte signing prototype) as research-internals or CLI-wrapper prototypes; production files are not edited by the agent (owner applies patches; otherwise the harness wraps the library API).
- Oracle table `research/experiments/EXP-CRYPTO-014/oracle.csv`, one row per matrix cell with the expected verdict, reviewed by a session not involved in the candidate.

## 8. Seeds, warmup, repetitions and adaptive rule

- Deterministic outcomes: 2 repetitions plus repeat-hash of the verdict objects. Signer/recipient key seeds in the spec.
- No warmups, no timing. The attacker-re-sign case uses a freshly generated adversary key per repetition.

## 9. Measurement method and metrics

| Metric (canonical) | ID | OD | Role |
|---|---|---|---|
| verdict-honesty (reported ≤ oracle across all cells) | HC-15 | OD-04/OD-15 | **binary** (R1) |
| `reason_code_specificity_fraction` | T-20 | OD-04 | **primary** (STALE vs INVALID vs ABSENT specificity) |
| `error_actionability_fraction` | T-20 | OD-25 | secondary |
| authorization correctness fraction (authorized vs unknown signer) | T-20 | OD-15 | **primary** |
| `undetected_tampers` under parser-differential mutation per signing scope | HC-14/HC-15 | OD-15 | binary (H4) |
| `authenticated_coverage_fraction` | HC-14 (M15.2) | OD-15 | binary |
| CLI flag/concept count per signing task | `flags_per_task` (C6) | OD-25 | cost input |

## 10. Normalization and statistical analysis

- Honesty and coverage: binary over every cell; any violation is R1 with the committed artifact.
- Fractions over the fixed matrix: T-20 exact, floor `0.5/n_cases`; a one-case difference is distinguishable. Aggregated over the matrix (AGG-C, min over strata for the headline).
- Confirmatory W against S on validation for the graded fractions; MDE gate.
- Cost inputs per candidate (AGG-P).

## 11. Practical significance

T-20 bands and §7.2 tiers from `research/methods/thresholds.json`. A new binding, record version, or exact-byte signature is at least T2 (feature bit) or T4 (new record type). Wire-frozen exclusions (for example CONTENT-mandatory mask) are enforced at R1.

## 12. Sensitivity analysis

Not weight-based. Reported arms: OS (Windows vs WSL, F-0001), layout (INDEXED vs STREAM), encrypted vs plain, and matrix-stratum breakdown. CB triggers (§10.2) evaluated (for example self-review, marginal wins).

## 13. Expected negative results worth recording

- `status-quo-any-key` accepts an attacker-signed detached signature under `--require-content-signature`, a real weakness driving DEC-CRY-082 toward an allowed-key set.
- Root-only signatures miss a parser-differential physical mutation, supporting an optional exact-byte binding (DEC-CRY-092) at a new-version cost.
- STREAM archives cannot be signed via the CLI today (DEC-CRY-096), confirmed by a code-path exercise.
- STALE/INVALID collapse loses actionable detail on AUX-only edits (DEC-CRY-004).

## 14. Threats to validity

- Oracle correctness is itself a judgement; reviewed by an independent session and cross-checked against SPEC §8.1/§20.5 line citations.
- Some candidates need library changes not present; prototypes may not match a final implementation, which is noted per candidate.
- Human-comprehension of the vocabulary is out of scope (EXP-CRYPTO-022).
- Role-confusion and cross-context forwarding resistance is formal/external (DEC-INT-032, DEC-CRY-084); only signature coverage is measured here.

## 15. Held-out placeholder (Phase D)

Honesty and coverage binary checks run on held-out signed archives on every split (R1). Graded fractions get one held-out look after unlock. No held-out item named here.

## 16. Cost estimate and agent effort

- Compute: about 40 core-hours (matrix is large but each cell is cheap). Disk: about 20 GB transient.
- Agent effort: tooling **judgment** (matrix driver, oracle, policy prototypes); execution **scripted**; analysis **judgment**.

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

_Generated by `python research/tools/experiments/fill_crypto_auto_blocks.py` for EXP-CRYPTO-014; edit the inputs, not this block._
<!-- END AUTO:common -->
