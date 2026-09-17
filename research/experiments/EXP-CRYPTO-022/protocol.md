# EXP-CRYPTO-022: Crypto CLI mechanical complexity and simulated first-use of trust and privacy output (HUMAN-FACING)

<!-- BEGIN AUTO:meta -->
| Field | Value |
|---|---|
| Index status | **NEEDS_TOOLING** — Human comprehension/task-success/preference EXTERNAL_REVIEW_REQUIRED or removed; simulated sessions heuristic only |
| Kind | decision |
| Domain | crypto (program §22.1, §22.4, §22.5) |
| Decisions informed | DEC-CRY-090, DEC-INT-021, DEC-CRY-082, DEC-CRY-009, DEC-CRY-046, DEC-CRY-058, DEC-CRY-061, DEC-CRY-004, DEC-CRY-063, DEC-CRY-095, DEC-CRY-108, DEC-ECO-064, DEC-ECO-063, DEC-CRY-101, DEC-CRY-094 |
| Platforms | Windows+Linux/x86-64 |
| Requires timing | False |
| Estimates | 5 machine-hours; 1 GB disk |
| Tooling to build | cli_census.py; first_use_sessions harness; CLI output prototypes |
| Environment prerequisites | none beyond the harness and the ebr venv |
| Blocked arms | none |
| Specs | `spec.yaml` |
| Validation looks | owns no validation look |
| Gates | G-A and G-B unmet at this revision (generated `gate_state` column of `research/experiments/index.csv`); timing runs also need a PASS calibration record for the calibration identity (PA-04) |
| Conventions | `research/experiments/_index/crypto-conventions.md` (CR0-CR10); program amendments `research/experiments/_program/design-revision-round1.md` |
<!-- END AUTO:meta -->

## 1. Question

For the crypto-facing CLI surfaces — verify output (validity, per-binding status, authorized signer, trusted timestamp, keyless "PRIVATE CONTENT UNVERIFIED"), signing-binding selection, `inspect` KDF-parameter reporting, password-strength and plaintext-downgrade warnings, recipient directory/key management, and the AUX-vs-LAI stale-content diagnosis — what are the mechanically checkable properties (flag and concept counts, unsafe defaults, message-content census), and what do independently simulated first-use sessions reveal as qualitative failure modes? Human comprehension, task success and preference are routed to external/human review; only the mechanical properties decide.

## 2. Hypotheses and falsification

- **H1 (mechanical).** Candidate output/flag designs differ in `flags_per_task`, concept count and `error_actionability_fraction` over a fixed task set; some status-quo surfaces have `unsafe_defaults` above 0 (for example a require-flag satisfied by any key, or an exit 0 on keyless-unverified without a machine-readable qualifier).
- **H2 (simulated, heuristic only).** Simulated sessions surface reproducible misinterpretations (for example reading "cryptographically valid" as "trusted", or "PRIVATE CONTENT UNVERIFIED, exit 0" as "verified"). These are qualitative failure modes, never a CI-based verdict.
- **Falsification:** H1 is a mechanical comparison at T-20 bands. H2 cannot be falsified statistically by design (T-18 heuristic); it can only surface or fail to surface failure modes.

## 3. Decisions informed and evidence route

<!-- BEGIN AUTO:candidates -->
#### DEC-CRY-090 — How do library and CLI keep cryptographically valid, binding current, authorized signer and trusted timestamp as separate verdicts, what combined summary (if any) is returned or printed, and what does verify report when no signatures exist?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-014, EXP-ECO-019, EXP-ECO-021, EXP-INT-003.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-three-dimensions` | Status quo: cryptographic, per-binding and timestamp statuses; no authorization; silent when none | status quo | no | EXP-INT-003 |
| `add-authorization-dimension` | Add AUTHORIZED/UNKNOWN_SIGNER derived from caller key policy | ledger alternative | no | EXP-INT-003 |
| `overall-verdict-enum` | Add summary verdicts such as TRUSTED_CURRENT, VALID_UNTRUSTED, STALE, INVALID | ledger alternative | no | EXP-INT-003 |
| `structured-json-only` | Structured JSON statuses with no summary word | ledger alternative | no | EXP-INT-003 |
| `explicit-no-signatures-line` | Print "no signatures present" whenever signatures are requested and none exist | ledger alternative | no | EXP-INT-003 |
| `verdict-words-require-policy` | Print VALID/TRUSTED wording only when a trust policy was supplied | ledger alternative | yes | EXP-INT-003 |
| `silent-when-no-signatures` | Print nothing about signatures when none exist | ledger alternative | no | EXP-CRYPTO-014, EXP-INT-003 |

**Ledger exclusions:** {"candidate_id": "silent-when-no-signatures", "reason": "Omitting the no-signature statement lets an unsigned archive look signature-verified", "invariant_violated": "SPEC §17.3 L1891 no-signature reporting invariant"}.

**R0 checklist:** 1 status quo: `status-quo-three-dimensions`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 7 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-INT-021 — Should verify of an encrypted archive without unlock material exit 0 with 'PRIVATE CONTENT UNVERIFIED', exit with a distinct status, or refuse, and is keyless public verification a named level with defined guarantees for custodians?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-010, EXP-ECO-013, EXP-ECO-015, EXP-ECO-016, EXP-ECO-019, EXP-INT-003.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-exit-zero-label` | Status quo: exit 0 with PRIVATE CONTENT UNVERIFIED text | status quo | no | EXP-CRYPTO-010, EXP-INT-003 |
| `distinct-exit-code` | Dedicated non-zero exit code for OK-but-unverified | ledger alternative | no | EXP-CRYPTO-010, EXP-ECO-016, EXP-INT-003 |
| `require-public-only-flag` | Exit 0 only with explicit --public-only; otherwise refuse with usage/policy error | ledger alternative | no | EXP-CRYPTO-010, EXP-INT-003 |
| `refuse-without-unlock` | verify refuses encrypted archives without unlock material | ledger alternative | no | EXP-CRYPTO-010, EXP-INT-003 |
| `json-qualifier-exit-zero` | Exit 0 with machine-readable UNVERIFIED qualifier in JSON output | ledger alternative | no | EXP-CRYPTO-010, EXP-INT-003 |
| `policy-flag-require-full` | --require-full-verification converts keyless result into failure; default unchanged | ledger alternative | no | EXP-CRYPTO-010, EXP-INT-003 |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: `status-quo-exit-zero-label`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 6 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-CRY-082 — Should library SignaturePolicy and CLI verify accept an allowed-key set (full public keys or signer IDs), per-key binding requirements and a require-timestamp flag, so that --require-*-signature means "from an authorized signer", and what is the default when no keys are supplied?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `HUMAN_PARTICIPANTS`. Other experiments informing it: EXP-CRYPTO-014, EXP-ECO-016, EXP-ECO-018, EXP-ECO-019, EXP-ECO-021.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-any-key` | Status quo: require flags satisfied by any valid signature | status quo | no | EXP-CRYPTO-014 |
| `allowed-public-keys` | Allowed full public keys supplied by file or flag | ledger alternative | no | EXP-CRYPTO-014, EXP-ECO-016, EXP-ECO-018 |
| `allowed-signer-ids` | Allowed 16-byte signer IDs | ledger alternative | no | EXP-ECO-018 |
| `require-key-with-require-flags` | Refuse --require-*-signature unless an allowed-key set is supplied | ledger alternative | no | EXP-CRYPTO-014, EXP-ECO-018 |
| `policy-file` | Policy file naming signers, required bindings and timestamp requirements | ledger alternative | no | — |
| `warn-untrusted-signer` | Keep any-key semantics but print UNTRUSTED signer warnings | ledger alternative | no | — |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: `status-quo-any-key`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 6 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `policy-file`, `warn-untrusted-signer`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-CRY-009 — Should signers be able to create partial-binding signatures (content-only, or content plus addressing without physical) through the CLI, with --bind-physical replaced by a working opt-out, or should the frozen "sign every available binding" policy stand and the dead flag be removed?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-014, EXP-ECO-018, EXP-ECO-019, EXP-ECO-021.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-all-bindings-dead-flag` | Status quo: sign all available bindings; --bind-physical no-op | status quo | no | — |
| `no-bind-physical-flag` | Add --no-bind-physical (and --no-bind-addressing) | ledger alternative | no | EXP-ECO-018 |
| `explicit-binding-list` | Explicit --bind content,physical,addressing list | ledger alternative | no | EXP-ECO-018 |
| `remove-dead-flag-library-only` | Remove --bind-physical; partial bindings via library only | ledger alternative | no | — |
| `profile-based-defaults` | Defaults chosen by signing purpose (release versus backup) | ledger alternative | no | — |
| `physical-only-signature` | Allow signatures without the CONTENT binding | ledger alternative | no | EXP-CRYPTO-014 |

**Ledger exclusions:** {"candidate_id": "physical-only-signature", "reason": "Binding mask must always include CONTENT", "invariant_violated": "crypto-v1 wire frozen: mask bit 0 CONTENT mandatory (docs/crypto-wire-v1.md L78-81)"}.

**R0 checklist:** 1 status quo: `status-quo-all-bindings-dead-flag`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 6 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `status-quo-all-bindings-dead-flag`, `remove-dead-flag-library-only`, `profile-based-defaults`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-CRY-046 — How should inspect report stored password KDF parameters and assess them against current guidance, and how is that baseline versioned in a long-lived tool?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-017, EXP-ECO-019.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-not-reported` | Status quo: inspect reports stanza type only, no parameters or assessment | status quo | no | EXP-CRYPTO-017 |
| `raw-parameters-only` | Report raw Argon2id parameters without judgement | ledger alternative | no | EXP-CRYPTO-017 |
| `versioned-baseline-table` | Compare against a baseline table versioned with the tool release | ledger alternative | no | EXP-CRYPTO-017 |
| `rfc9106-profile-comparison` | Compare against RFC 9106 named profiles | ledger alternative | no | EXP-CRYPTO-017 |
| `warn-and-suggest-change-password` | Warn below baseline and suggest key change-password | ledger alternative | no | EXP-CRYPTO-017 |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: `status-quo-not-reported`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 5 ledger candidates listed above; 4 strongest incumbent: `rfc9106-profile-comparison`; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-CRY-058 — Should encrypted creation (library and CLI) refuse empty or weak passwords, warn with a strength estimate, or leave password policy to callers?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-017, EXP-ECO-016, EXP-ECO-019.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-inconsistent` | Status quo: library and CLI pack accept empty; change-password rejects empty | status quo | no | EXP-CRYPTO-017 |
| `reject-empty-library` | Reject empty passwords at the library boundary for creation and change | ledger alternative | no | EXP-CRYPTO-017 |
| `minimum-length` | Enforce a minimum length | ledger alternative | no | EXP-CRYPTO-017 |
| `strength-estimate-warning` | zxcvbn-style strength estimate with a warning | ledger alternative | no | EXP-CRYPTO-017 |
| `strength-enforcement-override` | Enforce a strength threshold with an explicit override flag | ledger alternative | no | EXP-CRYPTO-017 |
| `caller-policy-only` | No library policy; document caller responsibility | ledger alternative | no | EXP-CRYPTO-017 |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: `status-quo-inconsistent`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 6 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-CRY-061 — Must exporting or publishing an authenticated encrypted archive to plaintext legacy targets require explicit opt-in, and how is the confidentiality downgrade recorded and classified?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `HUMAN_PARTICIPANTS`. Other experiments informing it: EXP-CRYPTO-019, EXP-ECO-016, EXP-ECO-019, EXP-ECO-021, EXP-LEGACY-030, EXP-LEGACY-070.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-warning-and-receipt` | Status quo: warning line plus receipt fields | status quo | no | EXP-CRYPTO-019, EXP-LEGACY-030, EXP-LEGACY-070 |
| `explicit-flag-required` | Refuse plaintext output without --allow-plaintext or --downgrade-encryption | ledger alternative | no | EXP-CRYPTO-019, EXP-LEGACY-030, EXP-LEGACY-070 |
| `interactive-confirmation` | Prompt on TTY; flag required non-interactively | ledger alternative | no | EXP-CRYPTO-019, EXP-LEGACY-030, EXP-LEGACY-070 |
| `declared-loss-outcome` | Classify as declared loss in outcome class and receipt | ledger alternative | no | EXP-CRYPTO-019, EXP-LEGACY-030, EXP-LEGACY-070 |
| `silent-export` | Export plaintext with no warning or record | ledger alternative | no | EXP-CRYPTO-019, EXP-LEGACY-030, EXP-LEGACY-070 |

**Ledger exclusions:** {"candidate_id": "silent-export", "reason": "Silent confidentiality downgrade is prohibited and receipt must record the transition", "invariant_violated": "SPEC §15.2 L1697; docs/legacy-export-v1.md L49-51"}.

**R0 checklist:** 1 status quo: `status-quo-warning-and-receipt`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 5 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-CRY-004 — Should verify report whether a stale content binding comes from an LAI change or an AUX-only change (comparing stored transcript fields), should AUX become a separate binding in a future version, or should SPEC simply be corrected to the single content binding?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-014.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-single-content-status` | Status quo: one content status | status quo | no | — |
| `diagnose-from-stored-values` | Keep one binding; report which of LAI or AUX differs | ledger alternative | no | — |
| `separate-aux-binding-new-version` | Separate AUX binding bit in a new record version | ledger alternative | no | — |
| `correct-spec-only` | Correct SPEC §8.0 and §17.8 text without behaviour change | SPEC design | no | — |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: `status-quo-single-content-status`; 2 SPEC design: `correct-spec-only`; 3 ledger alternatives: 4 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `status-quo-single-content-status`, `diagnose-from-stored-values`, `separate-aux-binding-new-version`, `correct-spec-only`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-CRY-063 — What should the private recipient directory contain (fingerprints, full public keys, labels), must it be mandatory, and how are labels preserved, given co-recipient disclosure and key remove usability?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-ECO-018, EXP-ECO-019.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-mandatory-fingerprint-label` | Status quo: mandatory directory with fingerprint and caller label; labels replaced on removal | status quo | no | — |
| `optional-directory` | Directory optional at creation; list and remove need external knowledge when absent | ledger alternative | no | EXP-ECO-018 |
| `no-labels-by-default` | Labels omitted unless requested | ledger alternative | no | EXP-ECO-018 |
| `full-public-keys` | Store full public keys so key remove needs no retained key files | ledger alternative | no | — |
| `preserve-authenticated-labels` | Rotation preserves authenticated labels for retained recipients | ledger alternative | no | — |
| `per-recipient-private-labels` | Labels readable only by the named recipient | ledger alternative | no | — |
| `public-directory` | Publicly visible recipient fingerprints | ledger alternative | no | — |

**Ledger exclusions:** {"candidate_id": "public-directory", "reason": "Stable public recipient identifiers are forbidden in crypto v1", "invariant_violated": "docs/crypto-wire-v1.md L270; AR-19"}.

**R0 checklist:** 1 status quo: `status-quo-mandatory-fingerprint-label`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 7 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `status-quo-mandatory-fingerprint-label`, `full-public-keys`, `preserve-authenticated-labels`, `per-recipient-private-labels`, `public-directory`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-CRY-095 — What protection do signing keys get in v1: plaintext EBSK with Unix 0600 (status quo; nothing on Windows), Windows ACL restriction, passphrase-encrypted EBSK, OS keystore or agent, PKCS#11/HSM, or an external signer protocol?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-018, EXP-ECO-017.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-plaintext-ebsk` | Status quo: plaintext seed, 0600 on Unix, no-op on Windows | status quo | no | EXP-CRYPTO-018, EXP-ECO-017 |
| `windows-acl-restriction` | Restrict EBSK ACLs on Windows and warn when readable | ledger alternative | no | EXP-CRYPTO-018, EXP-ECO-017 |
| `passphrase-encrypted-ebsk` | EBSK v2 encrypted with Argon2id-derived key | ledger alternative | no | EXP-CRYPTO-018, EXP-ECO-017 |
| `os-keystore` | OS keystore or agent integration (DPAPI/CNG, Keychain, ssh-agent) | ledger alternative | no | EXP-CRYPTO-018 |
| `pkcs11-hsm` | PKCS#11/HSM signing | ledger alternative | no | EXP-CRYPTO-018 |
| `external-signer-protocol` | Sign the transcript via an external signer plugin over stdin/stdout | ledger alternative | no | EXP-CRYPTO-018 |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: `status-quo-plaintext-ebsk`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 6 ledger candidates listed above; 4 strongest incumbent: `os-keystore`, `pkcs11-hsm`; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-CRY-108 — When the CLI loads an X-Wing identity or Ed25519 signing-key file readable beyond its owner, must it refuse (frozen crypto-suite-v1 behaviour), warn (current code) or apply another policy, how are Windows ACLs checked, and how is an override expressed?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-018, EXP-ECO-017, EXP-PLAT-016, EXP-PLAT-020.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-warn-unix-only` | Status quo: stderr warning when Unix mode & 0o077 is non-zero; no check on other platforms (entrybound-cli/src/lib.rs:453-466) | status quo | no | EXP-CRYPTO-018, EXP-ECO-017, EXP-PLAT-016, EXP-PLAT-020 |
| `refuse-unix-default-with-override` | Refuse group/world-readable keys on Unix unless an explicit flag or caller policy overrides; check Windows ACLs where a safe API exists, otherwise warn (frozen suite text) | ledger alternative | no | EXP-CRYPTO-018, EXP-ECO-017, EXP-PLAT-016, EXP-PLAT-020 |
| `refuse-without-override` | Hard refusal with no override, as OpenSSH does for private keys accessible by other users | ledger alternative | no | EXP-CRYPTO-018, EXP-ECO-017, EXP-PLAT-016, EXP-PLAT-020 |
| `warn-all-platforms-with-dacl-check` | Never refuse; warn on Unix and on Windows when the DACL grants non-owner access | defer / no change | no | EXP-CRYPTO-018, EXP-ECO-017, EXP-PLAT-016, EXP-PLAT-020 |
| `policy-knob-default-warn` | Library and CLI key-permission policy defaulting to warn, refusing under a strict policy | ledger alternative | no | EXP-CRYPTO-018, EXP-ECO-017, EXP-PLAT-016, EXP-PLAT-020 |
| `no-check-caller-responsibility` | Drop the check and document caller responsibility (docs/crypto-implementation-v1.md L83) | ledger alternative | no | EXP-CRYPTO-018, EXP-ECO-017, EXP-PLAT-016, EXP-PLAT-020 |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: `status-quo-warn-unix-only`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 6 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: `warn-all-platforms-with-dacl-check`; 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-ECO-064 — How should verify and key commands present signature results (cryptographic validity, per-binding VALID/STALE, authorized signer, trusted timestamp, keyless PRIVATE CONTENT UNVERIFIED) so first-time users draw correct trust conclusions?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-ECO-019, EXP-ECO-021.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-lines` | Per-signature text lines with binding statuses and require-* policy flags | status quo | no | — |
| `single-verdict-plus-detail` | One trust verdict under an explicit policy followed by details | ledger alternative | no | — |
| `policy-required` | verify refuses to print a trust verdict unless a trust policy is supplied | ledger alternative | no | — |
| `json-first` | Human output minimal; full status only in JSON | ledger alternative | no | — |
| `sigstore-style-summary` | Identity-and-transparency style summary as in cosign verify | ledger alternative | no | — |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: `status-quo-lines`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 5 ledger candidates listed above; 4 strongest incumbent: `sigstore-style-summary`; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `status-quo-lines`, `single-verdict-plus-detail`, `policy-required`, `json-first`, `sigstore-style-summary`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-ECO-063 — Should the CLI let signers choose which bindings to sign (CONTENT only, CONTENT+PHYSICAL, +ADDRESSING), and how are the consequences for repack and recipient changes shown?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-014, EXP-ECO-018, EXP-ECO-019.

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

#### DEC-CRY-101 — Should Entrybound generate TimeStampReq bytes for the record imprint, ship an optional online TSA client (SPEC --timestamp <url>), or keep attach-only with documented external workflows?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `HUMAN_PARTICIPANTS`. Other experiments informing it: EXP-ECO-001, EXP-ECO-018, EXP-ECO-021.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-attach-external-token` | Status quo: --timestamp-token attaches externally obtained DER | status quo | no | — |
| `emit-timestamp-request` | Emit a DER TimeStampReq for the record imprint | ledger alternative | no | EXP-ECO-018 |
| `optional-online-tsa-client` | Optional feature-gated HTTP TSA client | ledger alternative | no | EXP-ECO-018 |
| `documented-openssl-recipe` | Document an openssl ts recipe only | ledger alternative | no | EXP-ECO-018 |
| `sigstore-tsa-integration` | Integrate with Sigstore timestamp-authority services | ledger alternative | no | — |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: `status-quo-attach-external-token`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 5 ledger candidates listed above; 4 strongest incumbent: `sigstore-tsa-integration`; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `status-quo-attach-external-token`, `sigstore-tsa-integration`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-CRY-094 — What does a valid Entrybound signature assert about identity: bare self-signed keys with the caller deciding (status quo), TOFU pinning, local keyring or allowed-signers files, X.509 trusted-publisher certificates, or an external identity layer, and how does verify present each?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-014, EXP-ECO-019, EXP-ECO-021.

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
<!-- END AUTO:candidates -->

Evidence route: HUMAN-FACING (decision-method.md §1.2, §4.16). **Mechanical properties (flag/concept counts, unsafe defaults, message census) are EMPIRICAL and may decide the mechanical parts.** Any claim about human comprehension, task success or preference is `EXTERNAL_REVIEW_REQUIRED` or removed (§4.16); simulated evidence is `[SIMULATED]`, capped at `SUPPORTED_WITH_LIMITATIONS`, and never the sole support (block, §8.4). The five HUMAN_PARTICIPANTS crypto rows (DEC-CRY-001, DEC-CRY-061, DEC-CRY-082, DEC-CRY-101, DEC-ECO-022) are HUMAN-FACING for their human claims.

## 4. Candidates and arms

Grouped by decision (candidate ids from the ledger):
- **DEC-CRY-090 / DEC-INT-038 / DEC-ECO-064** verify-output vocabulary and required-binding defaults;
- **DEC-INT-021** keyless verify exit and label;
- **DEC-CRY-082** allowed-key set and default (mechanical: does `--require-*` refuse without keys);
- **DEC-CRY-009 / DEC-ECO-063** binding-selection flags;
- **DEC-CRY-046** `inspect` KDF reporting;
- **DEC-CRY-058** password policy warnings;
- **DEC-CRY-061** plaintext-downgrade warning/flag;
- **DEC-CRY-004** stale-content LAI-vs-AUX diagnosis wording;
- **DEC-CRY-063** recipient directory contents and labels;
- **DEC-CRY-095 / DEC-CRY-108** key-file permission warning wording;
- **DEC-CRY-101** timestamp-request workflow messaging (mechanical parts; TSA client is a dependency-cost question in EXP-CRYPTO-013);
- **DEC-CRY-001 / DEC-ECO-022** recipient-type demand (POST_V1; demand survey is human/external, not decided here).

Reference candidate: the status-quo CLI output.

## 5. Corpus selectors

- **Task set (pre-committed, held-out task subset written before design iteration for the R5 mechanical part):** verify a signed archive; verify a stale-binding archive; verify an unauthorized-signer archive; verify a keyless-encrypted archive; sign with default bindings; sign content-only; inspect KDF parameters; create with a weak password; export encrypted to plaintext; add/remove a recipient. Completion criteria are mechanically checkable (exit code, presence of required message strings, no unsafe default triggered).
- **Base archives:** small generated and real (`f01-tuning-curl-8-19-0-git`, `f19-tuning-zoo`) archives in each state.
- No tuning/validation/held-out *corpus* item drives statistics; the corpus here is the task set and candidate CLIs.

## 6. Environment and platform requirements

`wsl-ubuntu` and `windows-host` (key-file permission messaging differs by OS). `timing: false`. Simulated sessions are run by agent sessions not involved in CLI design, which see only the CLI's own help/man text and the task statement (§4.16).

## 7. Commands and tooling

- `research/tools/crypto/cli_census.py`: counts flags and concepts per task, detects unsafe defaults, and runs the message-content census (`error_actionability_fraction`, presence of required strings) over each candidate CLI.
- `research/tools/crypto/first_use_sessions/`: a harness that presents help text and a task to an isolated agent session, records the commands issued, the completion outcome (by the mechanical criteria) and a qualitative failure-mode note. At least 20 sessions per (task, candidate), randomized order.
- Candidate output variants are prototyped as CLI wrappers over the library (production files not edited by the agent).

## 8. Seeds, warmup, repetitions and adaptive rule

- Mechanical census: deterministic, run once, repeat-hash.
- Simulated sessions: at least 20 per (task, candidate); randomized order; no statistical stopping rule (T-18 heuristic).

## 9. Measurement method and metrics

| Metric (canonical) | ID | OD | Role |
|---|---|---|---|
| `flags_per_task` | C6 | OD-25 | cost input / **primary mechanical** |
| `unsafe_defaults` | HC-05 (M25.4) | OD-25 | **binary** (= 0) |
| `error_actionability_fraction` | T-20 (M25.3) | OD-25 | **primary mechanical** |
| required-message presence (for example "no signatures present", downgrade recorded) | T-20 | OD-04/OD-25 | **primary mechanical** |
| `task_success_rate`, `task_steps`, `task_errors`, `task_completion_time_s` | T-18 | OD-25 | **heuristic only** (no CI verdict) |

## 10. Normalization and statistical analysis

- Mechanical metrics: T-20/C6 exact over the fixed task set; unsafe defaults binary.
- Simulated metrics: per-task success counts and qualitative failure-mode notes only; no confidence intervals (T-18: sessions of one base model are correlated).
- The mechanical parts get one held-out-task look (the pre-committed held-out task subset) for R5 of the mechanical claims.
- No human claim is decided; those are routed to `EXTERNAL_REVIEW_REQUIRED` or removed.

## 11. Practical significance

T-20 and C6 from `research/methods/thresholds.json`. A new flag or concept is a T2 user-visible change; `unsafe_defaults` must be 0 (HC-05). No band applies to the heuristic T-18 metrics.

## 12. Sensitivity analysis

Not weight-based. Reported arms: OS, task, and candidate. Simulated results are reported with the `[SIMULATED]` qualifier and the MEDIUM cap.

## 13. Expected negative results worth recording

- Simulated users read "cryptographically valid" as "trusted" absent a policy, supporting a verdict-words-require-policy design (DEC-CRY-090) — recorded as a qualitative failure mode, routed to human/external review, never decided on simulation alone.
- The status-quo `--require-content-signature` is satisfiable by any key (mechanical unsafe default), a DEC-CRY-082 finding usable mechanically.
- Keyless verify exit 0 without a machine-readable qualifier confuses scripts (DEC-INT-021), a mechanical finding.

## 14. Threats to validity

- Simulated sessions share a base model with the designers, so they are heuristic and correlated (T-18; objective §5 item 8). They never carry a human claim.
- The mechanical criteria may not capture real confusion; that gap is exactly why human claims are routed out.
- HUMAN_PARTICIPANTS rows stay `INSUFFICIENT_EVIDENCE`/`EXTERNAL_REVIEW_REQUIRED` for their human parts regardless of the mechanical results.

## 15. Held-out placeholder (Phase D)

The pre-committed held-out task subset serves R5 for the mechanical parts (§4.16). No held-out corpus content is used.

## 16. Cost estimate and agent effort

- Compute: negligible (CLI runs) plus simulated-session agent tokens (the one budget-sensitive cost here).
- Agent effort: **judgment** (census design, session harness) plus **scripted** execution; the simulated sessions themselves consume agent tokens, so they are batched and capped. Human claims are BLOCKED on human/external review.

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

_Generated by `python research/tools/experiments/fill_crypto_auto_blocks.py` for EXP-CRYPTO-022; edit the inputs, not this block._
<!-- END AUTO:common -->
