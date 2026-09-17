# EXP-CRYPTO-009: Bounded memory of encrypted creation, open, verify and range sessions, and justification of crypto-v1 structural limits

<!-- BEGIN AUTO:meta -->
| Field | Value |
|---|---|
| Index status | **NEEDS_TOOLING** |
| Kind | decision |
| Domain | crypto (program §22.1, §22.5) |
| Decisions informed | DEC-ACC-011, DEC-CON-009, DEC-CRY-013, DEC-CRY-015, DEC-ACC-009, DEC-CRY-027 |
| Platforms | Linux/x86-64; cgroup memory caps |
| Requires timing | False |
| Estimates | 200 machine-hours; 200 GB disk |
| Tooling to build | counting allocator (method §14 item 17); ebr-crypto mem; streaming generators; streamed-writer prototypes |
| Environment prerequisites | none beyond the harness and the ebr venv |
| Blocked arms | none |
| Specs | `spec.yaml` |
| Validation looks | owns look 1 for DEC-CRY-027; participates in looks owned by other experiments for DEC-ACC-009, DEC-ACC-011, DEC-CON-009, DEC-CRY-013, DEC-CRY-015 |
| Gates | G-A and G-B unmet at this revision (generated `gate_state` column of `research/experiments/index.csv`); timing runs also need a PASS calibration record for the calibration identity (PA-04) |
| Conventions | `research/experiments/_index/crypto-conventions.md` (CR0-CR10); program amendments `research/experiments/_program/design-revision-round1.md` |
<!-- END AUTO:meta -->

## 1. Question

Do encrypted creation, open, verification and range sessions run in memory bounded by declared parameters, independent of payload, across archive and entry counts up to and beyond the crypto-v1 limits? Are the frozen crypto-v1 structural limits (segment at most 2^20-1 DATA records and 1 GiB plaintext; private object at most 1 GiB; EBCS at most 1,000,000 items and 1 GiB; record caps 1 MiB CONTROL, 64 MiB PAYLOAD; 1,000,000 segments; 4,096 identity attempts) justified by measurement, and is the effective boundary text correct (DEC-CON-009, DEC-CRY-015)? What memory does the status-quo whole-buffer path use versus a streamed-segment writer or plaintext-discarding open (DEC-ACC-011), and what happens when encryption is requested with pipe/STREAM output (DEC-ACC-009)?

## 2. Hypotheses and falsification

- **H1 (status-quo growth).** Status-quo in-memory encrypted write (`container.rs`, whole-buffer with a cloned PAYLOAD reuse map) has `peak_rss_bytes` that grows `DISTINGUISHABLE` with total logical size, so it is not payload-bounded. `streamed-segment-writer` and `plaintext-discarding-open` are `NON_INFERIOR` to a declared bound across sizes (bounded-memory claim, objective M08.3).
- **H2 (limits).** At the 1,000,000-item / 1 GiB EBCS limit, status-quo single-object MANIFEST/ENCRYPTED_INDEX peak memory exceeds a practical CI bound (for example 2 GiB), which supports `sharded-manifest-index-v2` or `declared-scale-limits`. Below about 100,000 items it is within bound, so the status quo is adequate for that scale.
- **H3 (effective boundary).** The effective EBCS maximum equals the documented 2^30-12, verified by boundary conformance vectors at 2^30-12, 2^30-11 and 2^30.
- **H4 (encrypted STREAM).** `explicit-spool-to-temp` keeps working memory bounded while emitting INDEXED-shaped encrypted bytes; the status-quo refusal (`EB_CRYPTO_LAYOUT_UNSUPPORTED`) has zero memory cost by construction.
- **Falsification:** each H is falsified by the opposite verdict at the T-06 band (bounded-memory is binary: any measured value above the declared bound is a violation).

## 3. Decisions informed and evidence route

<!-- BEGIN AUTO:candidates -->
#### DEC-ACC-011 — Must encrypted creation, open, verification and range sessions operate in bounded memory, and do crypto-v1 single-object MANIFEST/ENCRYPTED_INDEX limits (1M items, 1 GiB) require sharding in a new crypto wire version?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-SCALE-010.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-in-memory` | Whole-buffer encrypted write/open, cloned PAYLOAD reuse map, single-object manifest/index (container.rs, wire.rs) | status quo | yes | — |
| `streamed-segment-writer` | Emit segments progressively with bounded staging | ledger alternative | yes | EXP-SCALE-010 |
| `plaintext-discarding-open` | Authenticate and verify segments without retaining ciphertext copies | ledger alternative | yes | EXP-SCALE-010 |
| `sharded-manifest-index-v2` | Shard MANIFEST/ENCRYPTED_INDEX across objects in a future crypto version | ledger alternative | yes | EXP-SCALE-010 |
| `bounded-decrypted-frame-cache` | Byte-bounded decrypted frame cache in range sessions | ledger alternative | yes | — |
| `declared-scale-limits` | Document ~1M entry/Chunk ceilings for crypto-v1 | ledger alternative | yes | — |

**Ledger exclusions:** none recorded in the ledger.
**Exclusions proposed in this protocol (need independent sign-off, CR2):** `silent-layout-switch-or-buffer`.

**R0 checklist:** 1 status quo: `status-quo-in-memory`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 6 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-CON-009 — Are the frozen crypto-v1 structural limits (segment at most 2^20-1 DATA records and 1 GiB plaintext; private object at most 1 GiB; EBCS at most 1,000,000 items, 1 GiB and 64 MiB per item; effective EBCS maximum 2^30-12) justified by measurement, and is the boundary text corrected?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CONF-004, EXP-CONF-007, EXP-SCALE-010.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-limits` | Keep all current limits and clarify the effective boundary | status quo | yes | EXP-CONF-007 |
| `smaller-segments` | Smaller segment plaintext caps (e.g. 64 MiB) to improve random access | ledger alternative | yes | — |
| `larger-limits-declared-budget` | Raise item and object limits and bound them via declared budgets | ledger alternative | yes | — |
| `limits-as-declared-fields` | Encode structural limits as declared resource fields | ledger alternative | yes | — |
| `clarify-boundary-only` | Document the effective EBCS maximum without changing limits | ledger alternative | yes | — |

**Ledger exclusions:** none recorded in the ledger.
**Exclusions proposed in this protocol (need independent sign-off, CR2):** `silent-layout-switch-or-buffer`.

**R0 checklist:** 1 status quo: `status-quo-limits`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 5 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-CRY-013 — Are the crypto resource ceilings justified: Argon2id caller defaults equal to the wire ceiling (1 GiB, 10 passes, parallelism 16) versus lower open defaults, and the 1,024-recipient cap?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CONF-007, EXP-CONF-014, EXP-CRYPTO-004, EXP-CRYPTO-017, EXP-SCALE-010.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-default-equals-ceiling` | Status quo: open defaults equal wire ceiling; 1,024 recipients | status quo | no | EXP-CONF-007, EXP-CRYPTO-017 |
| `default-equals-creation-default` | Open default equals 256 MiB/3-pass creation default | ledger alternative | no | EXP-CONF-007, EXP-CRYPTO-017 |
| `hardware-probed-default` | Default derived from available memory | ledger alternative | no | EXP-CONF-007, EXP-CRYPTO-017 |
| `confirm-above-threshold` | Interactive confirmation above a cost threshold | ledger alternative | no | EXP-CONF-007, EXP-CRYPTO-017 |
| `lower-recipient-cap` | Lower recipient cap based on measured unlock cost | ledger alternative | no | EXP-CONF-007, EXP-CRYPTO-017 |

**Ledger exclusions:** none recorded in the ledger.
**Exclusions proposed in this protocol (need independent sign-off, CR2):** `silent-layout-switch-or-buffer`.

**R0 checklist:** 1 status quo: `status-quo-default-equals-ceiling`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 5 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-CRY-015 — What frozen caps and default caller policies should crypto v1 use (1 MiB CONTROL and 64 MiB PAYLOAD records, 2^20-1 DATA and 1 GiB per segment, 1,000,000 segments, 1 GiB working memory, EBCS 1,000,000 items/64 MiB/1 GiB, 4,096 identity attempts)?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CONF-007, EXP-CRYPTO-017.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-limits` | Status quo numeric caps and defaults | status quo | yes | EXP-CONF-007 |
| `measured-defaults-same-caps` | Keep frozen caps; retune caller defaults from measurements | ledger alternative | yes | EXP-CONF-007 |
| `tiered-policy-profiles` | Named policy profiles (constrained, desktop, server) | ledger alternative | yes | EXP-CONF-007 |
| `smaller-record-caps` | Lower PAYLOAD record cap (e.g. 16 MiB) to reduce buffering | ledger alternative | yes | EXP-CONF-007 |
| `larger-control-cap` | Raise CONTROL cap for very large manifests/Index fragments | ledger alternative | yes | EXP-CONF-007 |

**Ledger exclusions:** none recorded in the ledger.
**Exclusions proposed in this protocol (need independent sign-off, CR2):** `silent-layout-switch-or-buffer`.

**R0 checklist:** 1 status quo: `status-quo-limits`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 5 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-ACC-009 — What does v1 do when encryption is requested with pipe/STREAM output (refuse as now, explicit spool-to-temp then emit INDEXED, or buffer), and when and under what privacy contract is encrypted STREAM specified?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-EVAL-008, EXP-SCALE-010.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-refuse-before-output` | EB_CRYPTO_LAYOUT_UNSUPPORTED before emitting bytes | status quo | yes | — |
| `explicit-spool-to-temp` | Opt-in spool to a private temp file, emit INDEXED encrypted bytes to pipe | ledger alternative | yes | EXP-SCALE-010 |
| `encrypted-stream-v2` | Future crypto version defining encrypted sequential layout with trailer manifest and bounded staging | ledger alternative | yes | — |
| `silent-layout-switch-or-buffer` | Silently buffer or switch to INDEXED | ledger alternative | yes | EXP-SCALE-010 |

**Ledger exclusions:** {"candidate_id": "silent-layout-switch-or-buffer", "reason": "Repo crypto freeze forbids secret buffering or silent layout change", "invariant_violated": "docs/crypto-review-v1.md L136-142"}.
**Exclusions proposed in this protocol (need independent sign-off, CR2):** `silent-layout-switch-or-buffer`.

**R0 checklist:** 1 status quo: `status-quo-refuse-before-output`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 4 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-CRY-027 — Should convert import and publish accept --recipient or --password to produce encrypted native output directly from plaintext or legacy sources, and without staging plaintext on disk? Encrypted-to-encrypted repack is decided in encrypted-repack-mutation-contract.

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-019, EXP-EVAL-008.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-no-encrypted-output` | Status quo: import and publish cannot encrypt; users pack an extracted tree or publish an existing .eb | status quo | no | EXP-CRYPTO-019 |
| `direct-encrypted-import` | convert import --recipient/--password writes encrypted INDEXED output directly | ledger alternative | yes | EXP-CRYPTO-019 |
| `direct-encrypted-publish` | publish --recipient/--password encrypts native artefacts from directories | ledger alternative | no | EXP-CRYPTO-019 |
| `documented-two-step-workflow` | Document a two-step workflow and its plaintext exposure | ledger alternative | no | EXP-CRYPTO-019 |
| `encrypted-import-with-temp-staging` | Encrypted import via temporary plaintext staging | ledger alternative | yes | EXP-CRYPTO-019 |

**Ledger exclusions:** none recorded in the ledger.
**Exclusions proposed in this protocol (need independent sign-off, CR2):** `silent-layout-switch-or-buffer`.

**R0 checklist:** 1 status quo: `status-quo-no-encrypted-output`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 5 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.
<!-- END AUTO:candidates -->

Evidence route: `EMPIRICALLY_MEASURED` peak memory and scale limits. The counting allocator with a pre-validation bound H per candidate is required (§14 item 17) before any HC-06 record. Bounded-memory claims are binary (objective M08.3, §3.3); the graded remainder is T-06. Wire-changing candidates (`sharded-manifest-index-v2`, `segment-locator-table-v2`) are future crypto versions (T4) and are out of scope for crypto-v1 decisions except as future-version evidence.

## 4. Candidates and arms

- **DEC-ACC-011 memory:** `status-quo-in-memory`; `streamed-segment-writer`; `plaintext-discarding-open`; `sharded-manifest-index-v2` (future version, model); `bounded-decrypted-frame-cache`; `declared-scale-limits`.
- **DEC-CON-009 limits:** `status-quo-limits`; `smaller-segments` (64 MiB); `larger-limits-declared-budget`; `limits-as-declared-fields`; `clarify-boundary-only`.
- **DEC-CRY-015 caps:** `status-quo-limits`; `measured-defaults-same-caps`; `tiered-policy-profiles`; `smaller-record-caps` (16 MiB PAYLOAD); `larger-control-cap`.
- **DEC-ACC-009 encrypted STREAM:** `status-quo-refuse-before-output`; `explicit-spool-to-temp`; `encrypted-stream-v2` (future version, model); (`silent-layout-switch-or-buffer` excluded at R1, repo freeze).
- **DEC-CRY-013 open defaults:** measured here as the memory side; unlock latency in EXP-CRYPTO-017.
- **DEC-CRY-027 (POST_V1) direct encrypted import:** measured bounded-memory of `direct-encrypted-import` versus `encrypted-import-with-temp-staging`.

Reference candidate: status quo of each decision.

## 5. Corpus selectors

- **Scale grid (generated, tuning):** `text-v1` and `random-v1` trees at entry counts 10^2, 10^3, 10^4, 10^5, 10^6 and beyond the limit (10^6+1), and total logical sizes 16 MiB, 512 MiB, 8 GiB and beyond (streaming generators required per PROGRESS note on fingerprint memory; generated on the fly, not hashed in memory). Seeds in the spec.
- **Real (tuning):** `f04-tuning-maildir` (many entries), `f07-tuning-qwen2-5-1-5b-safetensors-bf16` (large single objects), `f16-ubuntu-noble-cloudimg-20260911-raw` and `f15-tuning-real-ubuntu-noble-ova-raw` (large), `f08-tuning-postgres17-pgbench-s40`.
- **Boundary vectors:** constructed inputs at 2^30-12, 2^30-11 and 2^30 EBCS units.
- **Validation:** `f04-validation-objstore`, `f07-validation-smollm2-360m-safetensors`, `f16-debian-13-netinst-iso`, plus the scale grid at validation seeds, one registered look.

## 6. Environment and platform requirements

- `wsl-ubuntu`: `alloc_peak_bytes` (counting allocator, exact) is primary for library-level decode/encrypt memory against declared bounds; `peak_rss_bytes` via GNU time is the process-level check. Memory claims never cross OS (§4.14).
- Memory-cap arms use WSL cgroup v2 memory limits (256 MiB, 512 MiB, 1 GiB, 2 GiB) as an x86-64 substitute for constrained hosts (Docker is unavailable; ARM/mobile memory is EXP-CRYPTO-004, BLOCKED).
- `timing: false` for memory and scale; a separate `timing: true` sub-run measures spool-to-temp wall time in a quiet window.
- Streaming generators avoid the in-memory hashing limit noted in PROGRESS.

## 7. Commands and tooling

- Counting allocator with allocation-site attribution in the harness (§14 item 17), exposed through `entrybound::research` and used by `ebr-pack`/`ebr-crypto` encrypted paths.
- `ebr-crypto mem --op {create,open,verify,range} --scheme encrypted --scale ...`: encrypts/opens under a declared bound H and reports `alloc_peak_bytes`, `peak_rss_bytes`, and pass/fail against H.
- Research-internals `streamed-segment-writer` and `plaintext-discarding-open` prototypes (default-off; MVT-16(c) re-proof).
- `research/experiments/EXP-CRYPTO-009/scale_pass.sh` (detached, resumable, checkpointed raw), following the size-pass pattern of `research/baselines/run_size_pass.sh`.

## 8. Seeds, warmup, repetitions and adaptive rule

- Memory (deterministic given inputs and allocator): 2 repetitions plus the exact allocator oracle; the objective M08.3 protocol probes sizes 1, 4, 16 and 64 GiB with 3 runs each and requires growth 1→64 GiB ≤ max(4 MiB, 0.10·R0) (Appendix C.1).
- Spool wall time: timing rounds per §4.6.
- Boundary vectors: single exact evaluation, re-hash checked.

## 9. Measurement method and metrics

| Metric (canonical) | ID | OD | Role |
|---|---|---|---|
| `alloc_peak_bytes` (create/open/verify/range) | T-06 | OD-08 | **primary** (graded, above the bound floor) |
| `peak_rss_bytes` | T-06 | OD-08 | secondary (process check) |
| `bounded_memory_growth_bytes` | HC-06 (M08.3) | OD-08 | **binary screen** (growth 1→64 GiB within bound) |
| `declared_tightness_ratio` | T-23 | OD-17 | diagnostic (declared/actual) |
| `scratch_write_bytes` | T-07 | OD-08/OD-09 | binary (spool path streaming/scratch claim, tolerance 0) |
| `scratch_peak_bytes` | T-07 | OD-08 | secondary (spool size) |
| `encode_wall_s` (spool overhead) | T-03 | OD-06 | secondary |
| scale limit (largest count/size within bound; first failure) | `scale_limits` (descriptive, M17.6) | OD-17 | reported |
| boundary conformance at 2^30-12/-11/2^30 | binary | — | screen |

## 10. Normalization and statistical analysis

- Memory: exact allocator values, compared to the declared bound (binary) and, above it, graded at the T-06 band. Aggregation per §4.9 for the graded comparison.
- Scale limits: reported as the largest passing scale and the first failing scale per candidate; no aggregation.
- Growth fits: the M08.3 rule exactly.
- Confirmatory W against S on validation for the graded memory metric.

## 11. Practical significance

T-06 (RSS/commit 4 MiB, allocator 1 MiB), T-07, T-23, T-03 bands from `research/methods/thresholds.json`. Bounded-memory failure is an R1/R2 matter (no band). A future-version candidate (sharded index, encrypted STREAM v2) is T4.

## 12. Sensitivity analysis

§6 arms plus: memory-cap level, entry-count versus object-size scaling separately, PAYLOAD record cap (16 MiB versus 64 MiB), profile.

## 13. Expected negative results worth recording

- The status-quo whole-buffer writer is not payload-bounded, contradicting bounded-memory expectations for large archives; this is a defect item under objective §2.0 rule 5, and R0 item 7 adds the corrective candidate.
- Million-entry single-object EBCS exceeds a 2 GiB CI bound, supporting a declared scale limit for crypto-v1 and sharding only in a future version.
- Spool-to-temp meets bounded memory but writes plaintext-derived bytes to a temp file, which is a confidentiality note (cross-ref DEC-ACC-009 privacy contract, external review).

## 14. Threats to validity

- Counting allocator covers Rust allocations, not memory-mapped or OS-cache pages; RSS is the cross-check.
- cgroup caps model container memory pressure only on x86-64; ARM/mobile is BLOCKED (EXP-CRYPTO-004).
- Streaming generators must themselves be bounded, verified by their own memory probe.
- R1-16 (runner RSS includes the Python process): decision-grade memory uses `alloc_peak_bytes` and GNU-time `resource.max_rss_bytes`, not the runner rusage.

## 15. Held-out placeholder (Phase D)

Bounded-memory and scale-limit checks on held-out large items after unlock; bounded-memory failures are R1/R2 on every split. No held-out item named here.

## 16. Cost estimate and agent effort

- Compute: about 200 machine-hours (large encrypted packs at scale dominate). Disk: up to 200 GB transient scratch; large generated inputs are streamed, not stored.
- Agent effort: tooling **judgment** (allocator, streaming writers); execution **scripted** (detached scale pass); analysis **judgment**.

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

_Generated by `python research/tools/experiments/fill_crypto_auto_blocks.py` for EXP-CRYPTO-009; edit the inputs, not this block._
<!-- END AUTO:common -->
