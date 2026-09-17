# EXP-CRYPTO-017: Password-KDF cost, recipient and attempt budgets, password policy, and indistinguishable secret-dependent failures

<!-- BEGIN AUTO:meta -->
<!-- END AUTO:meta -->

## 1. Question

What unlock latency and peak memory do the Argon2id creation parameters and the wire ceiling impose across host classes, and what defaults should creation and open use (DEC-CRY-002, DEC-CRY-013, DEC-CRY-015)? What envelope size and unlock latency does the 1,024-recipient cap and the 4,096-attempt budget impose, and should the attempt budget be checked before each trial and unlock be constant-work (DEC-CRY-068, DEC-CRY-064)? Should creation refuse empty/weak passwords (DEC-CRY-058), how should `inspect` report and assess KDF parameters (DEC-CRY-046), and are secret-dependent failures indistinguishable in reason code, message and timing (DEC-CRY-077)?

## 2. Hypotheses and falsification

- **H1 (KDF cost).** Status-quo 256 MiB/t3/p4 unlock latency on the x86-64 host is under about 1 s single-thread; at the wire ceiling (1 GiB/t10/p16) it is many seconds and exceeds 512 MiB memory caps. RFC 9106 second (64 MiB/t3/p4) and OWASP (19 MiB/t2/p1) are cheaper; scrypt and PBKDF2 differ in memory/CPU profile.
- **H2 (recipient/attempt cost).** Envelope `fixed_overhead_bytes` grows linearly with recipient count; unlock latency at 1,024 stanzas grows with the number of trial decryptions. `constant-work-all-stanzas` removes the position-timing signal at a latency cost; `budget-checked-before-attempt` bounds hostile-archive work.
- **H3 (timing indistinguishability).** Under the status quo, wrong-identity, wrong-password, tag-failure and post-decryption structure-error paths have distinguishable timing or reason codes (a dudect-style test finds |t| > 10 for at least one pair); `collapse-all-post-decryption-errors` and `constant-time-unlock-loop` remove the distinguishable reason code and (where measurable) the timing signal.
- **Falsification:** each H is a directional verdict at the registered bands (T-08 latency, T-06 memory, T-17/other). A timing test that fails on this noisy laptop yields HC_UNVERIFIED (MVT-14(h)), not a violation.

## 3. Decisions informed and evidence route

<!-- BEGIN AUTO:candidates -->
<!-- END AUTO:candidates -->

Evidence route: `EMPIRICALLY_MEASURED` latency, memory, envelope size and a message/reason-code census; the KDF *selection* is `EXTERNAL_REVIEW_REQUIRED` (DEC-CRY-002, DEC-CRY-035), so this experiment supplies measured inputs. Timing indistinguishability on a virtualized laptop is HC_UNVERIFIED and routed to external review; the reason-code/message census is decided here.

## 4. Candidates and arms

- **DEC-CRY-002 KDF params:** `status-quo-256mib-t3-p4`, `rfc9106-second-64mib`, `owasp-19mib`, `calibrated-per-host`, `named-cost-profiles`, `scrypt`, `balloon-hashing`, `pbkdf2-fips`, `labelled-salt-or-ad` (`rfc9106-first-2gib` excluded at R1: exceeds the 1 GiB reader bound).
- **DEC-CRY-013 open defaults:** `status-quo-default-equals-ceiling`, `default-equals-creation-default`, `hardware-probed-default`, `confirm-above-threshold`, `lower-recipient-cap`.
- **DEC-CRY-015 caps:** as EXP-CRYPTO-009, here on the attempt-budget/unlock side.
- **DEC-CRY-068 attempt budget:** `status-quo-zero-hints-4096`, `budget-checked-before-attempt`, `constant-work-all-stanzas`, `local-hint-cache` (`public-fingerprint-hints` excluded at R1).
- **DEC-CRY-064 dummy stanzas:** unlock-cost and size side (leakage side in EXP-CRYPTO-007).
- **DEC-CRY-058 password policy:** `status-quo-inconsistent`, `reject-empty-library`, `minimum-length`, `strength-estimate-warning`, `strength-enforcement-override`, `caller-policy-only`.
- **DEC-CRY-046 inspect reporting:** `status-quo-not-reported`, `raw-parameters-only`, `versioned-baseline-table`, `rfc9106-profile-comparison`, `warn-and-suggest-change-password`.
- **DEC-CRY-077 indistinguishable failures:** `status-quo-selective-collapse`, `collapse-all-post-decryption-errors`, `verbose-local-diagnostics-flag`, `constant-time-unlock-loop`.

Reference candidate: status quo of each decision.

## 5. Corpus selectors

- **Latency/memory (tuning):** a small fixed encrypted archive per protection type (one password stanza; 1, 2, 16, 128, 1,024 X-Wing recipients), plus generated archives. Unlock is measured, not archive content, so a small item suffices; `f20-tuning-generated-private-vault` provides diverse protection modes.
- **Timing indistinguishability:** fixed archives with a known-good and known-bad identity/password.
- **Message/reason-code census:** every failure path exercised once (fixed case set).
- **Validation:** the same on validation-analogous fixed archives, one registered look for the graded latency/memory metrics.

## 6. Environment and platform requirements

- `windows-host` and `wsl-ubuntu` for latency/memory; calibration (§4.7) for `time_wall` and `memory_peak` required; runner additions (§14 item 3).
- cgroup memory caps (256/512 MiB) on WSL for the memory-feasibility arm (Docker unavailable). ARM/mobile/32-bit KDF cost is EXP-CRYPTO-004 (BLOCKED).
- Timing indistinguishability uses in-process timers over 10^6 iterations (dudect, MVT-14(h)); on this virtualized host a failure is HC_UNVERIFIED.

## 7. Commands and tooling

- `ebr-crypto unlock --params ... --recipients N --wrong {identity,password,tag,structure}`: measures unlock latency, peak memory, attempt count and outcome; supplies passwords/identities through the harness, never the CLI prompt.
- `research/tools/crypto/dudect_unlock.py` or an `ebr-crypto` timing subcommand for the Welch-t leakage test.
- Message/reason-code census `research/tools/crypto/failure_census.py` over the CLI outputs.
- Argon2id/scrypt/PBKDF2 use the production `argon2` crate and pinned alternatives; `argon2-cffi` in the venv cross-checks parameter cost independently.

## 8. Seeds, warmup, repetitions and adaptive rule

- Latency/memory: timing rounds per §4.6 (small tier: in-process timer primary), warmups 2, adaptive precision rule.
- Dudect: 10^6 timed comparisons per pair, threshold |t| > 10.
- Census: deterministic, run once, repeat-hash.

## 9. Measurement method and metrics

| Metric (canonical) | ID | OD | Role |
|---|---|---|---|
| `open_latency_s` (unlock) | T-08 | OD-10 | **primary** |
| `alloc_peak_bytes`, `peak_rss_bytes` (unlock) | T-06 | OD-08 | **primary** |
| `fixed_overhead_bytes` / envelope bytes by recipient count | T-21 / T-02 | OD-05 | primary / diagnostic |
| unlock feasibility under a memory cap | binary | OD-08 | screen |
| dudect Welch-t per failure pair | binary or HC_UNVERIFIED (MVT-14(h)) | OD-15 | reported |
| `reason_code_specificity_fraction` collapsed appropriately | T-20 | OD-04/OD-15 | **primary** (DEC-CRY-077) |
| `error_actionability_fraction`, `flags_per_task` | T-20, C6 | OD-25 | secondary/cost |
| GPU/ASIC cost-per-guess estimate | descriptive | — | reported (analytical, DEC-CRY-002) |

## 10. Normalization and statistical analysis

- Latency/memory per §4.9-§4.12; environment same-direction; per host class as strata.
- Envelope size: exact per recipient count (each a condition).
- Census/fractions: T-20 exact.
- Timing indistinguishability: dudect binary; on failure HC_UNVERIFIED, external review.
- KDF selection is not made here (external); the report is an input table.

## 11. Practical significance

T-08, T-06, T-21, T-02, T-20 bands from `research/methods/thresholds.json`. Constant-work unlock trades latency (T-08) for the timing signal; a new KDF or cost profile is at least T2, with selection external.

## 12. Sensitivity analysis

§6 arms plus: host class, thread count for Argon2id parallelism, recipient-count strata, and memory-cap level.

## 13. Expected negative results worth recording

- The open default equal to the wire ceiling makes hostile archives a DoS vector (a removed-recipient or attacker archive can demand 1 GiB/16-thread unlock); this supports lowering the open default (DEC-CRY-013).
- Constant-work unlock over 1,024 stanzas costs seconds of latency, a real usability cost.
- Post-decryption structure errors carry distinct reason codes today, a distinguishability finding for DEC-CRY-077.

## 14. Threats to validity

- Timing indistinguishability is unreliable on a virtualized laptop; treated as HC_UNVERIFIED and external, never a pass or violation.
- GPU/ASIC cost is analytical (no such hardware here).
- ARM/mobile/32-bit KDF cost is BLOCKED (EXP-CRYPTO-004).
- Simulated-usability of warnings is HUMAN-FACING (EXP-CRYPTO-022).

## 15. Held-out placeholder (Phase D)

Latency/memory confirmatory verdicts on held-out encrypted archives after unlock (unlock cost is content-independent, so held-out adds little; run for completeness). No held-out item named here.

## 16. Cost estimate and agent effort

- Compute: about 40 machine-hours (KDF cost at the ceiling and 1,024-recipient unlocks dominate) plus dudect runs. Disk: under 5 GB.
- Agent effort: tooling **scripted** plus **judgment** (dudect, census); execution **scripted**; analysis **judgment**.

<!-- BEGIN AUTO:common -->
<!-- END AUTO:common -->
