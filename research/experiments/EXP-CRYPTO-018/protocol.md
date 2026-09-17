# EXP-CRYPTO-018: Secret hygiene, zeroization, memory-residue and signing-key file protection

<!-- BEGIN AUTO:meta -->
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
<!-- END AUTO:common -->
