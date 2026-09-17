# EXP-CRYPTO-018: Secret hygiene, zeroization, memory-residue and signing-key file protection

<!-- BEGIN AUTO:meta -->
| Field | Value |
|---|---|
| Index status | **NEEDS_TOOLING** |
| Kind | decision |
| Domain | crypto (program §22.1, §22.2, §22.4) |
| Decisions informed | DEC-CRY-005, DEC-CRY-078, DEC-CRY-095, DEC-CRY-108, DEC-CRY-041 |
| Platforms | Windows+Linux/x86-64 |
| Requires timing | False |
| Estimates | 5 machine-hours; 1 GB disk |
| Tooling to build | trybuild hygiene audit; ebr-crypto residue (F-24); keyfile_perms runner; dep_zeroize_audit.py |
| Environment prerequisites | none beyond the harness and the ebr venv |
| Blocked arms | none |
| Specs | `spec.yaml` |
| Validation looks | owns no validation look |
| Gates | G-A and G-B unmet at this revision (generated `gate_state` column of `research/experiments/index.csv`); timing runs also need a PASS calibration record for the calibration identity (PA-04) |
| Conventions | `research/experiments/_index/crypto-conventions.md` (CR0-CR10); program amendments `research/experiments/_program/design-revision-round1.md` |
<!-- END AUTO:meta -->

## 1. Question

Do secret-bearing types satisfy the frozen hygiene rules — no `Debug`/`Display`/implicit `Clone`, zeroize-on-drop where ownership permits — and are residual secret copies eliminated or bounded (DEC-CRY-005, DEC-CRY-078)? Should `EncryptedBoundaryKey` implement `ZeroizeOnDrop` and avoid `Clone` (DEC-CRY-005)? What protection do signing keys and identity files get at rest, and how are group/world-readable key files handled on Unix and Windows (DEC-CRY-095, DEC-CRY-108, DEC-CRY-041)?

## 2. Hypotheses and falsification

- **H1 (audit).** A compile-fail audit (`trybuild`) shows secret-bearing types implement none of `Debug`, `Display` or implicit `Clone` (MVT-14(g)); any that do are findings. `EncryptedBoundaryKey` currently derives `Clone, Eq, PartialEq` without `Zeroize` (per `crates/entrybound/src/chunker.rs`), so the status quo fails the audit for that type.
- **H2 (residue).** A research-build memory-residue test finds non-zero secret bytes after drop for at least one path in the status quo (candidate AFK buffer, HKDF/HMAC state, X-Wing internals, long-lived range-session keys); the `fix-owned-gaps` candidate clears them.
- **H3 (cost).** Zeroization and removing `Clone` (pass-by-reference) are `EQUIVALENT` on chunking and unlock timing (cross-ref EXP-CRYPTO-003, EXP-CRYPTO-017), so hygiene costs no measurable performance.
- **H4 (key files).** On Unix, a group/world-readable identity/signing key is detected (mode & 0o077); on Windows the status quo does nothing (F-0001-adjacent gap). A DACL check under `unsafe_code = forbid` is feasible or not (recorded).
- **Falsification:** H1/H2 falsified if the audit and residue tests are clean for the status quo. H3 falsified if hygiene is `DISTINGUISHABLE_WORSE` on any timing metric.

## 3. Decisions informed and evidence route

<!-- BEGIN AUTO:candidates -->
#### DEC-CRY-005 — Should EncryptedBoundaryKey (Gear table, PHTE polynomial, AES key) implement ZeroizeOnDrop and avoid Clone, and what lifetime do boundary secrets have during planning?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-003.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-clone-no-zeroize` | Status quo: derive(Clone, Eq, PartialEq) without Zeroize | status quo | yes | — |
| `zeroize-on-drop-no-clone` | ZeroizeOnDrop, remove Clone, pass by reference | ledger alternative | yes | — |
| `secrecy-wrapper` | Wrap secrets in a secrecy-style type with explicit expose | ledger alternative | yes | — |
| `derive-on-demand` | Derive boundary material inside the chunker and drop immediately | ledger alternative | yes | — |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: `status-quo-clone-no-zeroize`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 4 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-CRY-078 — Which residual secret copies (unzeroized candidate AFK buffer, HKDF/HMAC key state, X-Wing identity internals, long-lived range-session key hierarchies) must be eliminated or bounded before v1, and is page locking warranted?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: none.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-partial` | Status quo: owned secret types zeroize; known gaps remain | status quo | yes | — |
| `fix-owned-gaps` | Zeroizing buffers for candidate AFKs and derivation state plus dependency drop audit | ledger alternative | yes | — |
| `page-locking` | Lock secret pages in memory (mlock/VirtualLock) | ledger alternative | yes | — |
| `short-key-lifetime-range` | Drop or re-derive segment roots during long range sessions | ledger alternative | yes | — |
| `document-residual-risk` | Accept and document residual copies | ledger alternative | yes | — |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: `status-quo-partial`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 5 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-CRY-095 — What protection do signing keys get in v1: plaintext EBSK with Unix 0600 (status quo; nothing on Windows), Windows ACL restriction, passphrase-encrypted EBSK, OS keystore or agent, PKCS#11/HSM, or an external signer protocol?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-022, EXP-ECO-017.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-plaintext-ebsk` | Status quo: plaintext seed, 0600 on Unix, no-op on Windows | status quo | yes | EXP-ECO-017 |
| `windows-acl-restriction` | Restrict EBSK ACLs on Windows and warn when readable | ledger alternative | yes | EXP-ECO-017 |
| `passphrase-encrypted-ebsk` | EBSK v2 encrypted with Argon2id-derived key | ledger alternative | yes | EXP-ECO-017 |
| `os-keystore` | OS keystore or agent integration (DPAPI/CNG, Keychain, ssh-agent) | ledger alternative | yes | — |
| `pkcs11-hsm` | PKCS#11/HSM signing | ledger alternative | yes | — |
| `external-signer-protocol` | Sign the transcript via an external signer plugin over stdin/stdout | ledger alternative | yes | — |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: `status-quo-plaintext-ebsk`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 6 ledger candidates listed above; 4 strongest incumbent: `os-keystore`, `pkcs11-hsm`; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-CRY-108 — When the CLI loads an X-Wing identity or Ed25519 signing-key file readable beyond its owner, must it refuse (frozen crypto-suite-v1 behaviour), warn (current code) or apply another policy, how are Windows ACLs checked, and how is an override expressed?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-022, EXP-ECO-017, EXP-PLAT-016, EXP-PLAT-020.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-warn-unix-only` | Status quo: stderr warning when Unix mode & 0o077 is non-zero; no check on other platforms (entrybound-cli/src/lib.rs:453-466) | status quo | yes | EXP-ECO-017, EXP-PLAT-016, EXP-PLAT-020 |
| `refuse-unix-default-with-override` | Refuse group/world-readable keys on Unix unless an explicit flag or caller policy overrides; check Windows ACLs where a safe API exists, otherwise warn (frozen suite text) | ledger alternative | yes | EXP-ECO-017, EXP-PLAT-016, EXP-PLAT-020 |
| `refuse-without-override` | Hard refusal with no override, as OpenSSH does for private keys accessible by other users | ledger alternative | yes | EXP-ECO-017, EXP-PLAT-016, EXP-PLAT-020 |
| `warn-all-platforms-with-dacl-check` | Never refuse; warn on Unix and on Windows when the DACL grants non-owner access | defer / no change | yes | EXP-ECO-017, EXP-PLAT-016, EXP-PLAT-020 |
| `policy-knob-default-warn` | Library and CLI key-permission policy defaulting to warn, refusing under a strict policy | ledger alternative | yes | EXP-ECO-017, EXP-PLAT-016, EXP-PLAT-020 |
| `no-check-caller-responsibility` | Drop the check and document caller responsibility (docs/crypto-implementation-v1.md L83) | ledger alternative | yes | EXP-ECO-017, EXP-PLAT-016, EXP-PLAT-020 |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: `status-quo-warn-unix-only`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 6 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: `warn-all-platforms-with-dacl-check`; 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-CRY-041 — Must identity files be protected at rest in v1 (tool-set restrictive permissions, passphrase-wrapped identities, OS keychain), or is a raw 32-byte seed with caller-managed permissions acceptable?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-ECO-017.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-raw-seed` | Status quo: raw seed; permissions are the caller's responsibility | status quo | yes | EXP-ECO-017 |
| `restrictive-permissions` | Create identity files with owner-only permissions and ACLs | ledger alternative | yes | EXP-ECO-017 |
| `passphrase-wrapped-identity` | Passphrase-encrypted identity file (Argon2id-wrapped seed) | ledger alternative | yes | EXP-ECO-017 |
| `os-keychain` | OS keychain, DPAPI or Secret Service storage | ledger alternative | yes | — |
| `hardware-token-identity` | Hardware-backed identity | ledger alternative | yes | — |

**Ledger exclusions:** none recorded in the ledger.

**R0 checklist:** 1 status quo: `status-quo-raw-seed`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 5 ledger candidates listed above; 4 strongest incumbent: `os-keychain`; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.
<!-- END AUTO:candidates -->

Evidence route: `EMPIRICALLY_MEASURED` (compile-fail audit, memory-residue test, permission-handling behaviour over a fixed case set) plus a code audit (FORMAL, reviewer-signed). Workspace `unsafe_code = forbid` (F-23) is a repository freeze; any candidate adding `unsafe` is eliminated at R1.

## 4. Candidates and arms

- **DEC-CRY-005 boundary-key hygiene:** `status-quo-clone-no-zeroize`, `zeroize-on-drop-no-clone`, `secrecy-wrapper`, `derive-on-demand`.
- **DEC-CRY-078 residual copies:** `status-quo-partial`, `fix-owned-gaps`, `page-locking`, `short-key-lifetime-range`, `document-residual-risk`.
- **DEC-CRY-095 signing-key protection:** `status-quo-plaintext-ebsk`, `windows-acl-restriction`, `passphrase-encrypted-ebsk`, `os-keystore`, `pkcs11-hsm`, `external-signer-protocol`.
- **DEC-CRY-108 key-file permission on load:** `status-quo-warn-unix-only`, `refuse-unix-default-with-override`, `refuse-without-override`, `warn-all-platforms-with-dacl-check`, `policy-knob-default-warn`, `no-check-caller-responsibility`.
- **DEC-CRY-041 identity at rest:** `status-quo-raw-seed`, `restrictive-permissions`, `passphrase-wrapped-identity`, `os-keychain`, `hardware-token-identity`.

Reference candidate: status quo of each decision.

## 5. Corpus selectors

No corpus items. Inputs are generated identity/signing key files with controlled permissions (0600, 0640, 0644, 0666 on Unix; owner-only and everyone-readable DACLs on Windows), and small archives for the residue tests. Case lists are fixed before results (T-20).

## 6. Environment and platform requirements

- `wsl-ubuntu` for the compile-fail audit, residue tests and Unix permission handling; `windows-host` for the Windows ACL behaviour (per DEC-CRY-108 and F-0001 context).
- Memory-residue tests need a research-only instrumented build (F-24, MVT-16(c) identity-proof re-run) with a heap canary/scan; on a normal build, zeroization is checked by a drop-observing wrapper, not by scanning freed memory.
- `page-locking` (mlock/VirtualLock) is measured for feasibility only; it is a system resource action, not enabled by default.
- `timing: false` for audits; cost re-uses EXP-CRYPTO-003/017 timing.

## 7. Commands and tooling

- `trybuild` compile-fail cases in the harness verifying secret types reject `Debug`/`Display`/`Clone` (MVT-14(g)).
- `ebr-crypto residue`: a research-build test that unlocks/encrypts, drops the secrets, and scans the drop wrapper (or the research heap canary) for non-zero secret bytes; reports per secret type.
- `research/tools/crypto/keyfile_perms.py` (or `ebr-crypto keyperm`): creates key files with each permission, invokes `ebound` load, and records the outcome (accept, warn, refuse) and message.
- Dependency drop-semantics audit `research/tools/crypto/dep_zeroize_audit.py` over `hkdf`, `hmac`, `x-wing`, `argon2` sources.

## 8. Seeds, warmup, repetitions and adaptive rule

- Audits and permission cases: deterministic, run once, repeat-hash of outcomes.
- Residue test: 20 repetitions per secret type to catch intermittent non-zeroing; any non-zero result is a finding.
- No timing here.

## 9. Measurement method and metrics

| Metric (canonical) | ID | OD | Role |
|---|---|---|---|
| secret-type hygiene audit pass | binary (HC-14(g)) | OD-15 | **binary** |
| non-zero secret bytes after drop | binary (HC-14(g)) | OD-15 | **binary** |
| key-file permission handling correctness over the case set | T-20 | OD-25/OD-05 | **primary** |
| `error_actionability_fraction` of permission warnings | T-20 | OD-25 | secondary |
| DACL-check feasibility under `unsafe_code = forbid` | descriptive (binary yes/no) | OD-24 | reported |
| `decode_path_native_unsafe_count` added by each candidate | C4 cost input | OD-24 | cost |

## 10. Normalization and statistical analysis

- Binary audits: any failure is a finding (HC-14(g) violation for the hygiene rules; R1 where the frozen rule requires it).
- Fixed-case fractions: T-20 exact.
- No corpus aggregation; no sensitivity weighting (fixed case sets).
- Cost re-used from EXP-CRYPTO-003/017 for H3.

## 11. Practical significance

T-20 bands and §7.2 tiers from `research/methods/thresholds.json`. `passphrase-encrypted-ebsk` is a new key-file format (T2); `os-keystore`/`pkcs11-hsm` add dependencies (tier per §7.2). Adding `Zeroize` is a T0 implementation change.

## 12. Sensitivity analysis

Not weight-based. Reported arms: OS (Unix vs Windows), permission level, and secret-type. The performance sensitivity (H3) is the EXP-CRYPTO-003/017 timing.

## 13. Expected negative results worth recording

- `EncryptedBoundaryKey` fails the hygiene audit (derives `Clone` without `Zeroize`), a concrete DEC-CRY-005 finding.
- The status quo leaves the candidate AFK buffer un-zeroized on one path (DEC-CRY-078).
- Windows key-file protection is absent (no ACL check), a DEC-CRY-108/DEC-CRY-095 gap; a safe-Rust DACL check may be infeasible under `unsafe_code = forbid`.

## 14. Threats to validity

- Memory-residue scanning under a research build is not the shipped build; the drop-observing wrapper is the shipped-behaviour proxy, and MVT-16(c) proves identity when off.
- Dependency internals (hkdf/hmac/x-wing) may hold copies the program cannot zeroize; recorded as residual risk.
- Windows DACL semantics are complex; feasibility is recorded, not assumed.
- Usability of passphrase prompts is HUMAN-FACING (EXP-CRYPTO-022).

## 15. Held-out placeholder (Phase D)

Not applicable (no corpus items; audits and permission cases are content-independent). Re-run at the freeze commit for the frozen build.

## 16. Cost estimate and agent effort

- Compute: about 5 core-hours. Disk: under 1 GB.
- Agent effort: **judgment** (audits, residue test, feasibility assessment); execution **scripted**; a reviewer signs off the code-audit (FORMAL parts).

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

_Generated by `python research/tools/experiments/fill_crypto_auto_blocks.py` for EXP-CRYPTO-018; edit the inputs, not this block._
<!-- END AUTO:common -->
