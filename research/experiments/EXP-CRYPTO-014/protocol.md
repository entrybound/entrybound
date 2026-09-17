# EXP-CRYPTO-014: Signature and trust-semantics conformance matrix

<!-- BEGIN AUTO:meta -->
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
<!-- END AUTO:common -->
