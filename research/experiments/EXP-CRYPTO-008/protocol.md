# EXP-CRYPTO-008: Remote encrypted access — requests, bytes, modelled latency, access-pattern leakage, sublinearity

<!-- BEGIN AUTO:meta -->
| Field | Value |
|---|---|
| Index status | **NEEDS_TOOLING** — Emulated latency needs EXP-NETEM-CAL; no TCP slow start (R1-14) |
| Kind | decision |
| Domain | crypto (program §22.3, §22.4, §22.5) |
| Decisions informed | DEC-ACC-010, DEC-CRY-072, DEC-CRY-073, DEC-INT-010, DEC-CRY-030 |
| Platforms | Linux/x86-64; application-level network |
| Requires timing | True |
| Estimates | 50 machine-hours; 40 GB disk |
| Tooling to build | latency_model.py; access_pattern_eval.py; future-version writers; EXP-NETEM-CAL |
| Environment prerequisites | none beyond the harness and the ebr venv |
| Blocked arms | none |
| Specs | `spec.yaml` |
| Validation looks | owns look 1 for DEC-CRY-072, DEC-CRY-073; participates in looks owned by other experiments for DEC-ACC-010, DEC-CRY-030, DEC-INT-010 |
| Gates | G-A and G-B unmet at this revision (generated `gate_state` column of `research/experiments/index.csv`); timing runs also need a PASS calibration record for the calibration identity (PA-04) |
| Conventions | `research/experiments/_index/crypto-conventions.md` (CR0-CR10); program amendments `research/experiments/_program/design-revision-round1.md` |
<!-- END AUTO:meta -->

## 1. Question

For an encrypted archive read over HTTP range requests, how many requests and bytes does each open, list, single-entry read and range read cost, what is the modelled latency per network profile, and what does the origin learn from the request pattern? Is encrypted remote open sublinear in segment count (DEC-ACC-010), and do the candidate access-pattern mitigations reduce origin inference at acceptable byte and latency cost (DEC-CRY-072, DEC-CRY-073)? Also, what does incremental remote whole-archive verification cost (DEC-INT-010)?

## 2. Hypotheses and falsification

- **H1 (sublinearity).** Status-quo one-64-byte-range-per-SegmentHeader opening is linear in segment count: `http_requests` for open grows `DISTINGUISHABLE` with segment count. `coalesced-header-prefetch` reduces it to `NON_INFERIOR` to a constant on `http_requests` without a wire change. `segment-locator-table-v2` reduces `http_bytes` for open below the coalesced prefetch by at least the T-11 band.
- **H2 (access-pattern leakage).** Under status-quo access, an origin classifier identifies the accessed entry from `range_length_signatures` with `presence_advantage` `DISTINGUISHABLE` above 0. `batched-dependency-prefetch` reduces it by at least 2 band units at a bounded `http_bytes` cost; `dummy-range-requests` reduces it further at higher cost.
- **H3 (remote verify).** `remote-streaming-whole-verify` computes PCR/PCI within the declared memory bound at `http_bytes` `EQUIVALENT` to a full download, so it is feasible; `full-download-only` is the baseline.
- **Falsification:** each H is falsified by the opposite verdict at the registered bands, on validation.

## 3. Decisions informed and evidence route

<!-- BEGIN AUTO:candidates -->
#### DEC-ACC-010 — Must encrypted remote open be sublinear in segment count (segment offset/locator table, coalesced header prefetch, authenticated control-object locator), how should whole-segment fetches above range caps be handled, should range Chunk bounds use min(declared, caller), and does this require a new crypto wire version?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-REMOTE-010.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-header-walk` | One 64-byte range per SegmentHeader; whole-segment fetch; declared budget bounds Chunks | status quo | yes | EXP-REMOTE-010 |
| `coalesced-header-prefetch` | Batch header ranges (no wire change) | ledger alternative | yes | EXP-REMOTE-010 |
| `segment-locator-table-v2` | Authenticated segment offset/control locator table in a new crypto version | ledger alternative | yes | EXP-REMOTE-010 |
| `chunked-segment-fetch` | Fetch large segments in bounded sub-ranges with streaming AEAD verification | ledger alternative | yes | EXP-REMOTE-010 |
| `min-declared-caller-bounds` | Use min(declared, caller) for range Chunk bounds | ledger alternative | yes | EXP-REMOTE-010 |

**Ledger exclusions:** none recorded in the ledger.
**Exclusions proposed in this protocol (need independent sign-off, CR2):** `full-oblivious-access`, `merkle-over-segments`.

**R0 checklist:** 1 status quo: `status-quo-header-walk`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 5 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-CRY-072 — Is remote range access-pattern leakage accepted and documented for v1, or mitigated (whole-segment or batched prefetch, dummy fetches, https-only), and should cleartext http:// sources be refused?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-REMOTE-010, EXP-CRYPTO-021.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-documented-non-goal` | Status quo: no mitigation; leakage documented; http and https accepted | status quo | yes | EXP-REMOTE-010 |
| `batched-dependency-prefetch` | Fetch whole dependency closures or segment batches to blur which entry is read | ledger alternative | yes | EXP-REMOTE-010 |
| `dummy-range-requests` | Optional dummy or padded range requests | ledger alternative | yes | EXP-REMOTE-010 |
| `https-only-default` | Refuse http:// by default; opt-in flag for cleartext | ledger alternative | yes | EXP-REMOTE-010 |
| `full-oblivious-access` | ORAM/PIR-style oblivious access | ledger alternative | yes | EXP-REMOTE-010 |

**Ledger exclusions:** {"candidate_id": "full-oblivious-access", "reason": "Oblivious access is a frozen non-goal of crypto v1", "invariant_violated": "docs/crypto-threat-model-v1.md §Non-goals L198-199"}.
**Exclusions proposed in this protocol (need independent sign-off, CR2):** `full-oblivious-access`, `merkle-over-segments`.

**R0 checklist:** 1 status quo: `status-quo-documented-non-goal`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 5 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-CRY-073 — Should remote and range openers decode and verify embedded signatures and the recipient directory, and should plain-http remote access require an expected digest, PCI or signature pin?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-INT-016, EXP-INT-020, EXP-REMOTE-009, EXP-REMOTE-010.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-no-signatures-remote` | Status quo: range opener skips signatures; plain http accepted | status quo | no | EXP-INT-016, EXP-REMOTE-009, EXP-REMOTE-010 |
| `verify-signatures-on-range-open` | Fetch and verify signature CONTROL data on range open | ledger alternative | yes | EXP-INT-016, EXP-REMOTE-009, EXP-REMOTE-010 |
| `require-pin-for-plain-http` | Require expected digest or signature for plain http | ledger alternative | yes | EXP-INT-016, EXP-REMOTE-009 |
| `https-only-default` | Default to https only (cross-ref access http-client-transport-features) | ledger alternative | yes | EXP-INT-016, EXP-REMOTE-009, EXP-REMOTE-010 |
| `expected-pci-before-parse` | Expected PCI checked before parsing (cross-ref integrity pci-and-container-digest-verification) | ledger alternative | no | EXP-INT-016, EXP-REMOTE-009 |

**Ledger exclusions:** none recorded in the ledger.
**Exclusions proposed in this protocol (need independent sign-off, CR2):** `full-oblivious-access`, `merkle-over-segments`.

**R0 checklist:** 1 status quo: `status-quo-no-signatures-remote`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 5 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-INT-010 — What verification status is reported for encrypted random reads that do not verify segment END and the full segment-sequence digest, and is incremental remote whole-archive verification (PCR/PCI via ranges) required in v1?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-010, EXP-CRYPTO-021, EXP-INT-016, EXP-INT-020, EXP-REMOTE-010.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-per-object-plus-archivefinal` | Status quo: per-object AEAD plus authenticated ArchiveFinal and Descriptor/Manifest binding; PCR DeclaredNotFullyVerified; PCI NotComputed; no remote whole verification | status quo | yes | EXP-CRYPTO-010, EXP-INT-016, EXP-REMOTE-010 |
| `verify-touched-segment-ends` | Also verify END records of every touched segment and report segment-level status | ledger alternative | yes | EXP-CRYPTO-010, EXP-INT-016, EXP-REMOTE-010 |
| `per-segment-digests-in-archivefinal` | Bind per-segment digests usable by partial readers into ArchiveFinal | ledger alternative | yes | EXP-INT-016, EXP-REMOTE-010 |
| `remote-streaming-whole-verify` | Remote whole-archive verification fetching all ranges with bounded memory, computing PCR and PCI | ledger alternative | yes | EXP-CRYPTO-010, EXP-INT-016, EXP-REMOTE-010 |
| `full-download-only` | Whole-archive verification only after full download | ledger alternative | yes | EXP-CRYPTO-010, EXP-INT-016, EXP-REMOTE-010 |
| `merkle-over-segments` | Authenticated Merkle tree over segment digests enabling logarithmic partial proofs | ledger alternative | yes | EXP-INT-016, EXP-REMOTE-010 |

**Ledger exclusions:** {"candidate_id": "per-segment-digests-in-archivefinal", "reason": "Changes ArchiveFinalV1 fields; only possible in a new crypto wire version", "invariant_violated": "crypto-v1 wire frozen"}; {"candidate_id": "merkle-over-segments", "reason": "Requires new authenticated wire structures; only possible in a new crypto wire version", "invariant_violated": "crypto-v1 wire frozen"}.
**Exclusions proposed in this protocol (need independent sign-off, CR2):** `full-oblivious-access`, `merkle-over-segments`.

**R0 checklist:** 1 status quo: `status-quo-per-object-plus-archivefinal`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 6 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.

#### DEC-CRY-030 — Which bytes of an encrypted archive remain public (preamble feature bits including embedded-signature presence 0x200, recipient count, padded lengths, segment and record counts, buckets) and which must be hidden, padded or declared as accepted leakage?

Ledger status `INSUFFICIENT_EVIDENCE`, blocker class `EVIDENCE`. Other experiments informing it: EXP-CRYPTO-007, EXP-CRYPTO-021.

| Candidate id | Ledger description | Role | Named in this protocol | Named in other protocols |
|---|---|---|---|---|
| `status-quo-documented-public-framing` | Status quo: public discovery, envelope, framing and feature bits | status quo | no | EXP-CRYPTO-007 |
| `hide-signature-presence-new-version` | Move signature presence out of public feature bits (new version) | ledger alternative | no | EXP-CRYPTO-007 |
| `pad-record-counts` | Pad record and segment counts | ledger alternative | no | EXP-CRYPTO-007 |
| `maximum-padding-default` | Default to maximum padding mode | ledger alternative | no | EXP-CRYPTO-007 |
| `declare-accepted-leakage` | Accept and document each residual leak | ledger alternative | no | EXP-CRYPTO-007 |

**Ledger exclusions:** none recorded in the ledger.
**Exclusions proposed in this protocol (need independent sign-off, CR2):** `full-oblivious-access`, `merkle-over-segments`.

**R0 checklist:** 1 status quo: `status-quo-documented-public-framing`; 2 SPEC design: not separately identified (often the status quo); critic pass confirms; 3 ledger alternatives: 5 ledger candidates listed above; 4 strongest incumbent: no candidate names an incumbent technique; critic pass checks SPEC §22; 5 defer / not in v1: absent from the ledger set; add at the critic pass where release relevance permits (C7 add-later cost); 6 critic-proposed: **OPEN** — supplied only by the unexposed crypto-cluster critic pass (PA-12, `research/experiments/_program/critic-pass-plan.csv`); 7 re-specify: not applicable unless the critic pass finds an objective §2.0 rule 5 defect.
<!-- END AUTO:candidates -->

Evidence route: exact request and byte counts (`EMPIRICALLY_MEASURED`, deterministic), modelled latency (primary, `FORMALLY_DERIVED` from the request/byte log per §4.15), and emulated latency (`EMULATED`, corroborating after `EXP-NETEM-CAL`). Access-pattern leakage is graded (OD-16); mitigation effectiveness feeds external review. Access-pattern hiding beyond documented leakage is a frozen non-goal (`docs/crypto-threat-model-v1.md` L198-199): `full-oblivious-access` is excluded at R1.

## 4. Candidates and arms

- **DEC-ACC-010 open strategy:** `status-quo-header-walk`; `coalesced-header-prefetch`; `segment-locator-table-v2` (research writer, new-crypto-version transform); `chunked-segment-fetch`; `min-declared-caller-bounds`.
- **DEC-CRY-072 / DEC-CRY-073 mitigations:** `status-quo-documented-non-goal`; `batched-dependency-prefetch`; `dummy-range-requests`; `https-only-default` (policy, no byte effect — census only); `require-pin-for-plain-http`; `verify-signatures-on-range-open`.
- **DEC-INT-010 remote verify:** `status-quo-per-object-plus-archivefinal`; `verify-touched-segment-ends`; `remote-streaming-whole-verify`; `full-download-only`. (Wire-changing candidates `per-segment-digests-in-archivefinal` and `merkle-over-segments` are excluded at R1 for crypto-v1; measured only as a future-version model, labelled non-decision for v1.)
- **Network profiles (evaluation conditions, never averaged):** NP-LAN, NP-METRO, NP-CONT, NP-INTER (`decision-method.md` §4.15).
- **Layout/padding conditions:** the L0-L4 layouts and the padding modes of EXP-CRYPTO-002/006 (they change fetch granularity).

Reference candidate: status quo of each decision.

## 5. Corpus selectors

- **Tuning:** encrypted packs of `f01-tuning-curl-8-19-0-git`, `f04-tuning-maildir`, `f07-tuning-pythia-160m-safetensors-main`, `f09-ubuntu-noble-usr-binaries`, `f11-tuning-ubuntu-noble-debs`, `f13-tuning-bbb-video-medium`, `f16-alpine-3241-minirootfs-ext4-raw`, `f18-tuning-curl-8-x-release-series`, plus generated trees with segment counts 10, 100, 1,000 and 10,000 (for the sublinearity curve). Access workloads: open, list, single random entry, 1-byte range, 4 KiB range, 1 MiB range, whole entry, and a dependency-closure read (dedup indirection).
- **Validation:** `f01-validation-redis-7-2-16-git`, `f07-validation-smollm2-135m-safetensors`, `f09-fedora-44-usr-binaries`, `f11-maven-central-jvm-jars`, `f16-debian-13-nocloud-20260831-qcow2`, `f18-validation-redis-7-2-point-releases`, one registered look.

## 6. Environment and platform requirements

`wsl-ubuntu`. Requests and bytes are counted exactly by `ebr-origin` regardless of timing. **Modelled latency is primary** and does not depend on the emulator. **Emulated latency requires `EXP-NETEM-CAL` to pass** (RTT ±10%, throughput ±10%, exact counts; §4.15) and is reported beside the model; a profile that fails calibration has no reportable emulated latency. Per harness use constraint 7: use achieved RTT from logs, avoid configured RTT below 20 ms, and any range-coalescing decision (DEC-ACC-010) carries a slow-start sensitivity analysis because the application-level proxy has no TCP slow start (R1-14, open).

## 7. Commands and tooling

- `ebr-origin` (range-serving origin, exact request/byte logs; ETag, multipart, churn) and `ebr-netem-proxy` (RTT/bandwidth/loss/cache), both existing. A driver like `research/experiments/EXP-HARNESS-SMOKE-NETEM/origin_probe.sh` (README known limitation: these servers run until killed) is generalized to `EXP-CRYPTO-008/remote_access.sh`: start origin, run the entrybound remote reader (`ebound list/read <URL>` and `ebr-access` remote mode) through the proxy, collect logs, kill.
- `research/tools/crypto/latency_model.py`: computes modelled latency from the request/byte log using the §4.15 model (initial window `min(10·MSS, max(2·MSS, 14600))`, MSS 1460, doubling per RTT), per profile and connection model.
- `research/tools/crypto/cdc_attacks/access_pattern_eval.py`: origin classifier over `range_length_signatures` sequences.
- Research writer for `segment-locator-table-v2` and `remote-streaming-whole-verify` (research-internals or `ebr-crypto`, clearly labelled future-version model).

## 8. Seeds, warmup, repetitions and adaptive rule

- Request/byte counts: deterministic, 2 repetitions plus repeat-hash of the request log.
- Emulated latency (if calibrated): timing rounds per §4.6 in a quiet window; adaptive precision rule on the item-level interval; `remote_*_latency_s` floors are `max(10 ms, 0.5×RTT)` (T-10).
- Access-pattern classifier: trained on tuning access sequences, scored on held-back sequences; at least 40 sampled operations per item per round for any p95 claim (T-08).

## 9. Measurement method and metrics

| Metric (canonical) | ID | OD | Role |
|---|---|---|---|
| `http_requests` | T-12 | OD-12 | **primary** (open sublinearity) |
| `http_bytes`, `verified_range_fetch_bytes`, `open_bytes_read` | T-11 | OD-11/OD-12 | **primary** |
| `remote_first_entry_latency_s`, `remote_random_entry_latency_s`, `remote_task_latency_s` | T-10 | OD-12 | primary (modelled), corroborated (emulated) |
| `range_length_signatures` | T-12 | OD-16 (M16.4) | **primary** (access-pattern leakage) |
| `presence_advantage` (entry-identification from access pattern) | T-17 | OD-16 | secondary |
| `verify_overhead_pp`, `verified_fraction` | T-13, T-20 | OD-11 | diagnostic (remote verify) |
| `peak_rss_bytes`/`alloc_peak_bytes` of the range session | T-06 | OD-08 | guard (bounded memory; cross-ref EXP-CRYPTO-009) |
| `origin_protocol_safety` | HC-18 | OD-12 | binary screen (misbehaving-origin cases from `ebr-origin`) |

## 10. Normalization and statistical analysis

- Deterministic counts and bytes: exact per (item, workload, profile-independent). Aggregated per §4.9.
- Modelled latency: exact given the connection model; reported per profile (AGG-S, never averaged across profiles). Emulated latency: order-statistic item intervals, corroboration only.
- Access-pattern advantage: as EXP-CRYPTO-002.
- Confirmatory W against S on validation per primary metric and per profile condition; MDE gate.
- The sublinearity claim is a regression of `http_requests` on segment count with a 95% slope interval; "sublinear" means the slope's upper bound is below 1 on a log-log fit.

## 11. Practical significance

T-10, T-11, T-12, T-13, T-20, T-17 and T-06 bands from `research/methods/thresholds.json`. A wire-changing candidate (`segment-locator-table-v2`, `per-segment-digests-in-archivefinal`) is a new crypto wire version (T4); §7.3 minimum gains apply and it is out of scope for a crypto-v1 decision (excluded at R1 for v1, measured as future-version evidence).

## 12. Sensitivity analysis

§6 arms plus: network profile (the AGG-S condition), connection model (HTTP/1.1 versus HTTP/2 multiplexing, TLS on/off, connection reuse), layout, padding mode, and the slow-start sensitivity analysis required by harness use constraint 7 (model with and without a slow-start term; report both).

## 13. Expected negative results worth recording

- Coalesced header prefetch removes the linearity without any wire change, so `segment-locator-table-v2` fails its minimum gain and stays a future option.
- Dummy requests raise `http_bytes` beyond the T-11 band for small archives, so the mitigation is workload-scoped.
- Remote whole-archive verification needs many round trips on high-RTT profiles even at bounded memory, so it is impractical on NP-INTER.

## 14. Threats to validity

- No TCP slow start in the emulator (R1-14): the model, not the emulator, carries the latency claim; a slow-start sensitivity arm is mandatory for range-coalescing.
- Application-level netem is not packet-level: loss is only for failure behaviour (HC-18), never for a performance claim.
- The origin classifier is a lower bound on access-pattern leakage.
- Future-version writers are research models, not shipped wire.

## 15. Held-out placeholder (Phase D)

Frozen candidates run once on held-out encrypted archives after unlock, requests/bytes exact and latency modelled, with the frozen connection model. No held-out item named here.

## 16. Cost estimate and agent effort

- Compute: about 30 core-hours (mostly count/byte runs) plus about 20 machine-hours in a quiet window for emulated corroboration. Disk: about 40 GB transient.
- Agent effort: tooling **judgment** (latency model, classifiers, future-version writers); execution **scripted**; analysis **judgment**.

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

_Generated by `python research/tools/experiments/fill_crypto_auto_blocks.py` for EXP-CRYPTO-008; edit the inputs, not this block._
<!-- END AUTO:common -->
