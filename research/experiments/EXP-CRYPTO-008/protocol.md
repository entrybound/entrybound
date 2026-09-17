# EXP-CRYPTO-008: Remote encrypted access — requests, bytes, modelled latency, access-pattern leakage, sublinearity

<!-- BEGIN AUTO:meta -->
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
<!-- END AUTO:common -->
