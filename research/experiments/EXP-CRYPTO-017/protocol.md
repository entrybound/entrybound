# EXP-CRYPTO-017: Password-KDF cost, recipient and attempt budgets, password policy, and indistinguishable secret-dependent failures

<!-- BEGIN AUTO:meta -->
| Field | Value |
|---|---|
| Index status | **NEEDS_TOOLING** — Timing indistinguishability HC_UNVERIFIED on this virtualized host; ARM cost is EXP-CRYPTO-004 |
| Kind | decision |
| Domain | crypto (program §22.1, §22.3, §22.5) |
| Decisions informed | DEC-CRY-002, DEC-CRY-013, DEC-CRY-015, DEC-CRY-058, DEC-CRY-068, DEC-CRY-077, DEC-CRY-046, DEC-CRY-064 |
| Platforms | Windows+Linux/x86-64; requires calibration |
| Requires timing | True |
| Estimates | 40 machine-hours; 5 GB disk |
| Tooling to build | ebr-crypto unlock; dudect_unlock.py; failure_census.py |
| Environment prerequisites | none beyond the harness and the ebr venv |
| Blocked arms | none |
| Specs | `spec.yaml` |
| Validation looks | owns look 1 for DEC-CRY-002, DEC-CRY-013, DEC-CRY-015, DEC-CRY-046, DEC-CRY-058, DEC-CRY-064, DEC-CRY-068, DEC-CRY-077 |
| Gates | G-A and G-B unmet at this revision (generated `gate_state` column of `research/experiments/index.csv`); timing runs also need a PASS calibration record for the calibration identity (PA-04) |
| Conventions | `research/experiments/_index/crypto-conventions.md` (CR0-CR10); program amendments `research/experiments/_program/design-revision-round1.md` |
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
#### DEC-CRY-002 — Is Argon2id v19 the v1 password KDF, and are the frozen creation parameters (256 MiB, t=3, p=4, random unlabelled 16-byte salt) the right defaults, or should defaults, creator tunability or salt domain separation change?

Ledger status `EXTERNAL_REVIEW_REQUIRED`, blocker class `EXTERNAL_REVIEW`. Other experiments informing it: EXP-CRYPTO-004, EXP-CRYPTO-020, EXP-CRYPTO-021.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-256mib-t3-p4` | Status quo: Argon2id m=256 MiB, t=3, p=4 fixed at creation | status quo | yes | — |
| `rfc9106-second-64mib` | RFC 9106 second recommendation m=64 MiB, t=3, p=4 | ledger alternative | yes | — |
| `rfc9106-first-2gib` | RFC 9106 first recommendation m=2 GiB, t=1, p=4 | ledger alternative | yes | — |
| `owasp-19mib` | OWASP m=19 MiB, t=2, p=1 | ledger alternative | yes | — |
| `calibrated-per-host` | Calibrate to a target unlock latency on the creating host within wire bounds | ledger alternative | yes | — |
| `named-cost-profiles` | Named cost profiles selectable at creation | ledger alternative | yes | — |
| `scrypt` | scrypt as age uses (N=2^18, r=8, p=1) | ledger alternative | yes | — |
| `balloon-hashing` | Balloon hashing | ledger alternative | yes | — |
| `pbkdf2-fips` | PBKDF2-HMAC-SHA256 with 600,000 iterations for FIPS environments | ledger alternative | yes | — |
| `labelled-salt-or-ad` | Status quo parameters plus format-label domain separation in salt or Argon2 associated data | status quo | yes | — |

**Ledger exclusions:** {"candidate_id": "rfc9106-first-2gib", "reason": "2 GiB exceeds the frozen 1 GiB crypto v1 reader memory bound", "invariant_violated": "docs/crypto-suite-v1.md L510-520 reader wire bounds"}.
**Exclusions proposed in this protocol (need independent sign-off, CR2):** `public-fingerprint-hints`, `rfc9106-first-2gib`.

**R0 checklist:** 1 status quo: `status-quo-256mib-t3-p4`, `labelled-salt-or-ad`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 10 ledger candidates listed above; 4 strongest incumbent: `rfc9106-second-64mib`, `rfc9106-first-2gib`, `scrypt`, `pbkdf2-fips`; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-CRY-013 — Are the crypto resource ceilings justified: Argon2id caller defaults equal to the wire ceiling (1 GiB, 10 passes, parallelism 16) versus lower open defaults, and the 1,024-recipient cap?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CONF-007, EXP-CONF-014, EXP-CRYPTO-004, EXP-CRYPTO-009, EXP-SCALE-010.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-default-equals-ceiling` | Status quo: open defaults equal wire ceiling; 1,024 recipients | status quo | yes | EXP-CONF-007 |
| `default-equals-creation-default` | Open default equals 256 MiB/3-pass creation default | ledger alternative | yes | EXP-CONF-007 |
| `hardware-probed-default` | Default derived from available memory | ledger alternative | yes | EXP-CONF-007 |
| `confirm-above-threshold` | Interactive confirmation above a cost threshold | ledger alternative | yes | EXP-CONF-007 |
| `lower-recipient-cap` | Lower recipient cap based on measured unlock cost | ledger alternative | yes | EXP-CONF-007 |

**Ledger exclusions:** none recorded in the ledger.
**Exclusions proposed in this protocol (need independent sign-off, CR2):** `public-fingerprint-hints`, `rfc9106-first-2gib`.

**R0 checklist:** 1 status quo: `status-quo-default-equals-ceiling`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 5 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-CRY-015 — What frozen caps and default caller policies should crypto v1 use (1 MiB CONTROL and 64 MiB PAYLOAD records, 2^20-1 DATA and 1 GiB per segment, 1,000,000 segments, 1 GiB working memory, EBCS 1,000,000 items/64 MiB/1 GiB, 4,096 identity attempts)?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CONF-007, EXP-CRYPTO-009.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-limits` | Status quo numeric caps and defaults | status quo | no | EXP-CONF-007, EXP-CRYPTO-009 |
| `measured-defaults-same-caps` | Keep frozen caps; retune caller defaults from measurements | ledger alternative | no | EXP-CONF-007, EXP-CRYPTO-009 |
| `tiered-policy-profiles` | Named policy profiles (constrained, desktop, server) | ledger alternative | no | EXP-CONF-007, EXP-CRYPTO-009 |
| `smaller-record-caps` | Lower PAYLOAD record cap (e.g. 16 MiB) to reduce buffering | ledger alternative | no | EXP-CONF-007, EXP-CRYPTO-009 |
| `larger-control-cap` | Raise CONTROL cap for very large manifests/Index fragments | ledger alternative | no | EXP-CONF-007, EXP-CRYPTO-009 |

**Ledger exclusions:** none recorded in the ledger.
**Exclusions proposed in this protocol (need independent sign-off, CR2):** `public-fingerprint-hints`, `rfc9106-first-2gib`.

**R0 checklist:** 1 status quo: `status-quo-limits`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 5 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-CRY-058 — Should encrypted creation (library and CLI) refuse empty or weak passwords, warn with a strength estimate, or leave password policy to callers?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-022, EXP-ECO-016, EXP-ECO-019.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-inconsistent` | Status quo: library and CLI pack accept empty; change-password rejects empty | status quo | yes | — |
| `reject-empty-library` | Reject empty passwords at the library boundary for creation and change | ledger alternative | yes | — |
| `minimum-length` | Enforce a minimum length | ledger alternative | yes | — |
| `strength-estimate-warning` | zxcvbn-style strength estimate with a warning | ledger alternative | yes | — |
| `strength-enforcement-override` | Enforce a strength threshold with an explicit override flag | ledger alternative | yes | — |
| `caller-policy-only` | No library policy; document caller responsibility | ledger alternative | yes | — |

**Ledger exclusions:** none recorded in the ledger.
**Exclusions proposed in this protocol (need independent sign-off, CR2):** `public-fingerprint-hints`, `rfc9106-first-2gib`.

**R0 checklist:** 1 status quo: `status-quo-inconsistent`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 6 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-CRY-068 — Should recipient stanzas stay anonymous (zero hints) with trial unlock, what attempt-budget semantics and default apply when one identity meets at most 1,024 stanzas, and should unlock be constant-work?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: none.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-zero-hints-4096` | Status quo: zero hints; 4,096 attempts counted after each attempt | status quo | yes | — |
| `budget-checked-before-attempt` | Budget below the stanza cap and checked before each attempt | ledger alternative | yes | — |
| `constant-work-all-stanzas` | Process every stanza regardless of match to hide position | ledger alternative | yes | — |
| `keyed-hints-future` | Future version: privacy-preserving keyed hints | ledger alternative | no | — |
| `local-hint-cache` | Client-side cache mapping archive_id to matching stanza | ledger alternative | yes | — |
| `public-fingerprint-hints` | Public key fingerprints as hints | ledger alternative | yes | — |

**Ledger exclusions:** {"candidate_id": "public-fingerprint-hints", "reason": "Stable public identifiers enable correlation and are forbidden in crypto v1", "invariant_violated": "docs/crypto-wire-v1.md L270; AR-19"}.
**Exclusions proposed in this protocol (need independent sign-off, CR2):** `public-fingerprint-hints`, `rfc9106-first-2gib`.

**R0 checklist:** 1 status quo: `status-quo-zero-hints-4096`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 6 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

**R0 gap — ledger candidates named by no covering protocol:** `keyed-hints-future`. Recorded for the critic pass; they must be measured, excluded with a rule id, or shown to be covered before G-A.

#### DEC-CRY-077 — Which secret-dependent failures (wrong identity, wrong password, tag failure, padding or structure invalid after decryption, policy refusal) must be indistinguishable in reason codes, messages and timing?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CONF-003, EXP-CONF-004, EXP-CRYPTO-021.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-selective-collapse` | Status quo: envelope auth failure collapsed; post-decryption padding/structure errors have distinct codes | status quo | yes | — |
| `collapse-all-post-decryption-errors` | Collapse all post-decryption failures to one integrity code at the user boundary | ledger alternative | yes | — |
| `verbose-local-diagnostics-flag` | Distinct codes only with an explicit local diagnostics flag | ledger alternative | yes | — |
| `constant-time-unlock-loop` | Make unlock attempt timing independent of which stanza matched | ledger alternative | yes | — |

**Ledger exclusions:** none recorded in the ledger.
**Exclusions proposed in this protocol (need independent sign-off, CR2):** `public-fingerprint-hints`, `rfc9106-first-2gib`.

**R0 checklist:** 1 status quo: `status-quo-selective-collapse`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 4 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-CRY-046 — How should inspect report stored password KDF parameters and assess them against current guidance, and how is that baseline versioned in a long-lived tool?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-022, EXP-ECO-019.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-not-reported` | Status quo: inspect reports stanza type only, no parameters or assessment | status quo | yes | — |
| `raw-parameters-only` | Report raw Argon2id parameters without judgement | ledger alternative | yes | — |
| `versioned-baseline-table` | Compare against a baseline table versioned with the tool release | ledger alternative | yes | — |
| `rfc9106-profile-comparison` | Compare against RFC 9106 named profiles | ledger alternative | yes | — |
| `warn-and-suggest-change-password` | Warn below baseline and suggest key change-password | ledger alternative | yes | — |

**Ledger exclusions:** none recorded in the ledger.
**Exclusions proposed in this protocol (need independent sign-off, CR2):** `public-fingerprint-hints`, `rfc9106-first-2gib`.

**R0 checklist:** 1 status quo: `status-quo-not-reported`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 5 ledger candidates listed above; 4 strongest incumbent: `rfc9106-profile-comparison`; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-CRY-064 — Which recipient-related public fields (stanza count, types, classes, X-Wing encapsulations, password salt and Argon2 parameters) are acceptable, should recipient count be padded with dummy stanzas, and what may keyless inspect --crypto and diff --public report about recipients? The whole-archive public byte inventory is decided in encrypted-public-byte-leakage-inventory.

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-006, EXP-CRYPTO-007, EXP-CRYPTO-021.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-public-count-types` | Status quo: count, types, classes and parameters public; no stanza padding; keyless inspect and diff show them | status quo | no | EXP-CRYPTO-007 |
| `dummy-stanza-padding` | Optional dummy stanzas bucketing recipient count (SPEC §9.8 optional stanza padding) | SPEC design | no | EXP-CRYPTO-007 |
| `uniform-stanza-encoding` | Indistinguishable stanza encodings across types in a later version | defer / no change | no | EXP-CRYPTO-007 |
| `restrict-keyless-reporting` | Keyless inspect and diff omit recipient count and types by default | ledger alternative | no | EXP-CRYPTO-007 |
| `document-only` | Keep exposure and document it precisely in inspect --crypto | defer / no change | no | EXP-CRYPTO-007 |

**Ledger exclusions:** none recorded in the ledger.
**Exclusions proposed in this protocol (need independent sign-off, CR2):** `public-fingerprint-hints`, `rfc9106-first-2gib`.

**R0 checklist:** 1 status quo: `status-quo-public-count-types`; 2 SPEC design: `dummy-stanza-padding`; 3 ledger alternatives: 5 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: `uniform-stanza-encoding`, `document-only`; 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.
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

_Generated by `python research/tools/experiments/fill_crypto_auto_blocks.py` for EXP-CRYPTO-017; edit the inputs, not this block._
<!-- END AUTO:common -->
