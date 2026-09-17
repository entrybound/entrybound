# EXP-REMOTE-001: Network emulation calibration, connection model and emulation-method fidelity

| Field | Value |
|---|---|
| Status | **NEEDS_TOOLING** (T2 latency model twins, T3 origin/proxy/calibrator extensions, T7 kernel-TCP emulator feasibility) |
| Domain / program section | remote / §12 (with §7 environment) |
| Role | Fulfils the `EXP-NETEM-CAL` calibration that `decision-method.md` §4.15 and §14 item 18 require before any remote result. It is protocol calibration (like §4.7), not a decision result, except for DEC-ACC-063, whose question is the emulation method itself. |
| decision_ids | DEC-ACC-063 |
| Decision type (provisional; assigned by an independent session, §1.2) | EMPIRICAL (method fidelity) |
| OD / HC (provisional until ledger schema v2, gate G-A) | OD-12 (measurement validity of M12.3); HC-18 not exercised here |
| Method | `research/decision-method.md` at the pre-registration commit `14b977c` or its successor revision in force when the record is instantiated |
| Machine-readable inputs | `connection-model.json` (this directory): connection models CM-*, slow-start arms SS-*, header-byte accounting, the request-latency algorithm and the slow-start robustness rule. Profiles come from `research/methods/thresholds.json#protocol.network_profiles`. |
| Author / independence / L7 | Phase C-design remote-domain designer session. Did not construct the ledger candidate sets. **L7 disclosure:** this session read `research/corpus/coverage.md` as instructed; its group table names held-out item identifiers and upstream sources for many families (no content, listing, statistics or path under a held-out root was opened). Recorded for the §5.8(h) L7 record; the gate audit decides the consequence under §5.4 item 3. |
| Gates | G-A unmet at design time (crosswalk, ledger schema v2, validation-look registry). This file is the design pre-registration; the §12.2 `record.md` is instantiated from it once G-A holds. No run with non-empty `decision_ids` starts before then. |

## 1. Question

Does the application-level emulator (`ebr-netem-proxy` in front of `ebr-origin`) reproduce the four canonical network profiles within the §4.15 acceptance tolerances; does the trace-replay latency model of `connection-model.json` predict the emulator's per-request latency when the model uses the emulator's own physics (arm SS-NONE); and which emulation method gives rank-stable conclusions for the remote domain's candidate comparisons (DEC-ACC-063)?

## 2. Hypotheses

- **H1 (calibration).** With a shared-link token bucket and `--chunk-size` sized per profile, NP-METRO, NP-CONT and NP-INTER meet the tolerances of `thresholds.json#protocol.network_calibration_tolerance` (RTT, throughput, exact counts). NP-LAN (configured RTT below the 20 ms floor of harness review use constraint 7) fails the RTT tolerance and therefore has **no reportable emulated latency** (§4.15); its latency is model-only.
- **H2 (model agreement).** For every request in the replayed traces, emulated latency minus client CPU time agrees with the SS-NONE model: per profile, the median absolute relative error is at most the RTT tolerance fraction of the calibration tolerance and the median signed error lies inside it.
- **H3 (rank stability, DEC-ACC-063).** For the pre-registered comparison pairs (section 4, list RP), the direction of every pair's log-ratio under the primary model arm (SS-RFC6928) equals its direction under the kernel-TCP arm A3 whenever both are outside the metric's band.
- **H0.** Any of H1-H3 fails as stated.
- **Falsification.** H1 is falsified for a profile by any tolerance failure after the §4.7-style strengthening sequence below; H2 by an error statistic outside tolerance for any passing profile; H3 by one pair with opposite, both-out-of-band directions between arms.

## 3. Decisions informed

| Decision | What this experiment supplies | What remains elsewhere |
|---|---|---|
| DEC-ACC-063 | Calibration of the application-level proxy; agreement of trace-replay model and proxy; rank stability of candidate pairs across proxy, model arms and a real-kernel-TCP loopback emulator; documented fidelity limits (TCP loss recovery, TLS, HTTP/2 multiplexing) | Calibration against a real remote endpoint and real-path rank stability: EXP-REMOTE-013 (BLOCKED) |
| All remote-domain decisions | The gate: no emulated latency from a profile that fails here is reportable | none |

## 4. Candidates (emulation methods for DEC-ACC-063)

| candidate_id | Definition | Status in this experiment |
|---|---|---|
| `status-quo-application-level-proxy` | `ebr-netem-proxy` + `ebr-origin` (harness `research/harness/crates/ebr-netem` at the harness commit recorded in the record), shared-link shaping (T3), `--loss-p 0`, `--setup-rtt-multiple 2.0` for CM-H1-TLS-KA | Arm A1, measured |
| `trace-replay-cost-model` | `connection-model.json` algorithm, arms SS-RFC6928 (primary), SS-IDLE-RESET, SS-NONE, SS-IW4, driven by exact request logs | Arm A2, computed |
| `mahimahi-record-replay` (implemented as a purpose-built equivalent) | Real Linux kernel TCP (cubic, slow start, `tcp_slow_start_after_idle`) between two network namespaces joined through a userspace TUN relay that adds per-packet delay and a token-bucket rate (`ebr-tunem`, T7). Mahimahi's own shells are not built: they need a C++ toolchain, protobuf and iptables setup outside the pinned harness; the TUN relay reproduces the mechanism that matters here (real kernel congestion control over emulated delay) | Arm A3, measured if the T7 feasibility gate passes |
| `toxiproxy` | Shopify Toxiproxy (latency + bandwidth toxics) as a second application-level emulator | Arm A4, optional; needs owner approval to download the pinned release binary (download policy in PROGRESS standing decisions); if not approved it is recorded as unmeasured |
| `custom-kernel-netem` | WSL2 or Hyper-V kernel built with `sch_netem` | **BLOCKED:** replacing the WSL kernel (`.wslconfig kernel=`) or creating a Hyper-V VM is a system-configuration change that agents may not perform and Hyper-V needs admin. The owner may perform it; the arm then runs the same probes. |
| `real-object-stores-and-cdns` | Real S3/R2/GCS/CloudFront endpoints | **BLOCKED here**, designed in EXP-REMOTE-013 (credentials, publication approval) |

No candidate is excluded. The status quo is A1 (the method named in PROGRESS "Known blockers").

**Rank-stability pair list RP (fixed now; instantiated from the tuning runs of the experiments named).** RP-1: `status-quo-4k-same-purpose` vs `fixed-gap-1m` (EXP-REMOTE-003, WL-SUBTREE); RP-2: `no-coalescing` vs `status-quo-4k-same-purpose` (EXP-REMOTE-003, WL-RAND1); RP-3: `status-quo-header-walk` vs `speculative-tail-window-256k` (EXP-REMOTE-004, WL-OPEN); RP-4: `status-quo-serial-blocking` vs `fixed-n-connections-8` (EXP-REMOTE-006, WL-SUBTREE); RP-5: `status-quo-lazy-only` vs `eager-full-download-then-verify` (EXP-REMOTE-005, WL-FRACTION-0.5). Each pair is evaluated on 20 seeded tuning items (seed `order` of the spec) under all four profiles.

## 5. Corpus selectors

- Calibration objects: generated items only (`random-v1`, 64 MiB and 1 KiB, seeds in `spec.yaml`), served as opaque files. No corpus split is touched by probes P1-P5.
- Probe P6/P7 replays: request logs recorded by EXP-REMOTE-002 on **tuning** items only (20 items, seeded selection from its tuning item list, at least 10 independence groups across at least 5 families). Held-out: never (Phase D placeholder in section 15).

## 6. Environment and platform requirements

- `wsl-ubuntu` env (WSL2 Ubuntu 24.04, ext4 under `/root/eb-research`), loopback only; label `EMULATED` and `VIRTUALIZED` on every timing number.
- Process placement: client pinned to `st-v` {2}; proxy pinned to vCPUs {4,5}; origin to {6,7} (informational placement: the WSL decision-grade sets of Appendix C.1 govern only the measured client). A3 additionally needs root inside WSL for `ip netns`, `/dev/net/tun` (present, `CONFIG_TUN=y`, `CONFIG_NET_NS=y`, `CONFIG_VETH=y` observed at design time; `sch_tbf`/`sch_netem` absent).
- Quiet-machine timing window (§4.2 exclusivity, §4.4 guard; `guard.max_loadavg_1m` = 1.0 + client threads + 2 server threads declared as the spec tightening).
- Platform matrix: Linux x86-64 required; Windows host not used (no loopback emulator build validated there); macOS/ARM64 not applicable; no large storage; no privileged host operations (root inside the WSL VM only); network: loopback only (A4 download needs owner approval; A5 owner action; A6 in EXP-REMOTE-013).

**Platform matrix.** Linux x86-64 required (WSL2); Windows: not used; macOS: not applicable; ARM64: not applicable (network emulation only); network: loopback (Toxiproxy download and custom kernel are owner actions); privileged: root inside the WSL VM for the kernel-TCP arm, no host admin; large storage: no; external review: no.

## 7. Tooling and commands

Tooling to build before the run (index field `tooling_to_build`):

- **T2** `ebr_netem::model` (Rust) and `research/tools/remote/latency_model.py` (Python) implementing `connection-model.json#request_latency_algorithm`; cross-check test `research/tools/remote/test_latency_model_twins.py` (10,000 seeded sequences per condition, tolerance 1e-9 s).
- **T3** `ebr-origin --log` adds `header_bytes` per response; `ebr-netem-proxy --shared-link true` (one down/up token bucket shared by all connections of the proxy instance, required so N connections cannot multiply bandwidth); `ebr-netem-calibrate --profiles NP-LAN,NP-METRO,NP-CONT,NP-INTER --connection-model CM-H1-TLS-KA --probes P1,P2,P3,P4,P5 --out-raw research/raw/EXP-REMOTE-001/` with acceptance evaluation written into its JSON output; `--chunk-size` per profile from the rule `max(16 KiB, bw_bps/8 * 0.005)` (5 ms of link rate, matching the R1-13 minimum burst).
- **T7** `ebr-tunem` (new binary in `ebr-netem`): creates namespaces `ebr-cli` and `ebr-srv`, a TUN device in each, and relays packets in userspace with delay `rtt/2` per direction and a per-direction token bucket at the profile rate; queue limit 1 BDP. Feasibility gate FG-7: a 10 s iperf-style transfer through the relay at NP-INTER reaches the throughput tolerance and 1,000 1-byte ranges reach the RTT tolerance; if FG-7 fails at any profile, A3 is recorded as not feasible for that profile and H3 is evaluated on the remaining profiles only (at least NP-CONT and NP-INTER, else H3 is INCONCLUSIVE and DEC-ACC-063 routes rank stability to EXP-REMOTE-013).
- Replay driver: `ebr-remote emulate --replay <request-log.jsonl> --profile <NP> --arm proxy|tunem` (tooling T1, defined in EXP-REMOTE-002) issues exactly the logged request sequence (method, range, If-Match) over one CM-H1 connection and logs per-request wall time and client CPU time.

Run sequence (WSL):

```sh
cd /mnt/d/Projects/entrybound/entrybound
python research/orchestration/disk_guard.py
python research/harness/lock_parity.py            # must print RESULT PASS (use constraint 1)
PYTHONPATH=research/tools /root/eb-research/venv/bin/python research/tools/remote/test_latency_model_twins.py
PYTHONPATH=research/tools /root/eb-research/venv/bin/python -m ebr validate --spec research/experiments/EXP-REMOTE-001/spec.yaml
PYTHONPATH=research/tools /root/eb-research/venv/bin/python -m ebr run --spec research/experiments/EXP-REMOTE-001/spec.yaml --timing
PYTHONPATH=research/tools /root/eb-research/venv/bin/python -m ebr verify-raw --spec research/experiments/EXP-REMOTE-001/spec.yaml
PYTHONPATH=research/tools /root/eb-research/venv/bin/python -m ebr normalize --spec research/experiments/EXP-REMOTE-001/spec.yaml
PYTHONPATH=research/tools /root/eb-research/venv/bin/python research/experiments/EXP-REMOTE-001/analyze_calibration.py \
  --normalized research/normalized/EXP-REMOTE-001 --connection-model research/experiments/EXP-REMOTE-001/connection-model.json \
  --out research/normalized/EXP-REMOTE-001/decision/calibration-verdicts.json
```

`analyze_calibration.py` (to be written, T3) regenerates `research/harness/crates/ebr-netem/calibration.md` from raw data, replacing the superseded provisional report.

## 8. Probes, seeds, warmups, repetitions

| Probe | What | Samples per (arm, profile) |
|---|---|---|
| P1 RTT | 1-byte ranged GET on a warmed persistent connection | 200 requests per round |
| P2 throughput | range sized for about 2 s at the profile rate on a warmed connection; for A3 the first 4 BDPs are excluded from the rate estimate | 5 requests per round |
| P3 setup | first request on a fresh connection, 1-byte range | 50 per round |
| P4 counts | fixed script of 1,000 requests (mixed HEAD/GET, 0 B-64 MiB bodies, seeded): client-issued, origin-logged and proxy-logged counts and bytes | 1 script per round |
| P5 header bytes | response header bytes per (server, method, status) | from P4 logs |
| P6 model agreement | replay 20 EXP-REMOTE-002 tuning request logs | 1 replay per log per round |
| P7 slow-start signature (A3 only) | 64 ranged GETs of sizes 2^k·1 KiB, k = 0..15, on fresh connections | 1 per size per round |
| RP rank stability | the RP pairs, model arms computed, A1 and A3 measured | per the originating experiment's workload |

- Seeds: `order` 20260917, `bootstrap` 20260918, `command` 20260919 (P4 script and P6 log selection derive from `command`).
- Warmups: 2 rounds (small-tier rule of §4.5; the objects are small).
- Rounds: minimum 10, step 10, cap 30 (§4.6 small tier) with the precision-based adaptive rule on the median per-request latency; n_min(0.99) = 8 is below the minimum.
- Strengthening on calibration failure (applied in order, each under a new experiment id suffix, never widening a tolerance): halve `--chunk-size` pacing granularity; double the minimum burst to 10 ms of rate; pin proxy and origin to disjoint vCPUs {8..11}; double rounds cap; if still failing the profile is recorded as failing.

## 9. Metrics

These are calibration quantities, not canonical decision metrics; none is primary for any decision other than DEC-ACC-063, which is decided by the pass/fail and rank-stability outcomes below (a FORMAL-plus-EMPIRICAL method choice).

| Metric (spec name) | Meaning | Unit | Criterion |
|---|---|---|---|
| `achieved_rtt_ms_median` | median P1 latency minus measured client CPU | ms | within `network_calibration_tolerance.rtt_fraction` of the profile RTT |
| `achieved_throughput_mbit_median` | median P2 rate | Mbit/s | within `network_calibration_tolerance.throughput_fraction` of profile bandwidth |
| `setup_ms_median` | median P3 extra latency over P1 | ms | within the RTT tolerance of `setup_rtts_per_connection x RTT` |
| `count_mismatches` | P4 differences between client, origin and proxy counts and bytes | count | exactly 0 (`counts: exact`) |
| `header_bytes_median` | P5 | B | reported (model input) |
| `model_abs_rel_error_median`, `model_signed_rel_error_median` | P6 per-request (emulated − client CPU) vs SS-NONE model | fraction | H2 |
| `rank_disagreements` | RP pairs with opposite out-of-band directions between A2(SS-RFC6928) and A3; also A1 vs A2(SS-NONE) | count | 0 (H3) |
| `wall_s`, `cpu_s` (runner resource) | per sample | s | informational |

For RP, "out of band" uses the band of `remote_task_latency_s` (T-10 relative r and the profile-dependent absolute rule; `thresholds.json#metric_thresholds.remote_task_latency_s`). Nothing is restated here.

## 10. Normalization and statistical analysis

- Per (arm, profile) the P1-P3 statistics are medians over all valid samples of all rounds with the §4.10 exact order-statistic interval at 0.99; a tolerance passes only if the whole interval lies inside the tolerance (a stricter reading than the point estimate, pre-registered here).
- P4 is exact; any mismatch fails the profile (and blocks every experiment's count metrics until fixed).
- H2 uses item = replayed log; per log the median over rounds of per-request relative error; the corpus-level statistic is the median over logs.
- H3 is a sign test per pair per profile; no multiplicity adjustment (each disagreement is itself the failure).
- Invalid-sample and guard rules of §4.4 apply to A1/A3 timing samples.

## 11. Practical significance

No decision band is set here. Tolerances are `thresholds.json#protocol.network_calibration_tolerance` (§4.15, Appendix C.1). RP directions use T-10 via `metric_thresholds.remote_task_latency_s`.

## 12. Sensitivity analysis

- A2 evaluated under all four slow-start arms and all three single-connection CMs; RP stability reported per arm (the `decision_robustness_rule` of `connection-model.json` is exercised on the RP pairs as a dry run).
- A1 repeated with `--chunk-size` halved and doubled (pacing-granularity sensitivity), and with `--jitter uniform --jitter-max-ms` = 5% of RTT (reported only; canonical profiles have no jitter).
- Placement sensitivity: proxy and origin unpinned (`os-scheduled`, informational).

## 13. Expected negative results worth recording

- NP-LAN fails RTT calibration under A1 (expected; it becomes model-only).
- A1 underestimates latency for large first responses relative to A3 (slow start absent), so RP-1 and RP-3 may disagree between A1 and A3 at NP-CONT/NP-INTER: that is exactly the R1-14 risk and would be a positive finding for the method choice.
- FG-7 may fail at NP-LAN rates (1000 Mbit/s through a userspace relay); a failure is recorded, not worked around.

## 14. Threats to validity

- Loopback has no NIC queueing, MTU effects or cross traffic; all arms share the host (EMULATED, VIRTUALIZED).
- A3 uses real kernel TCP but the WSL2 VM kernel (5.15.167.4, cubic) may differ from origin-server stacks (CDN initial windows, BBR); SS-IW4 and SS-NONE bracket but do not cover larger CDN initial windows ([INFERRED]).
- TLS cost is a modelled sleep in A1 and absent in A3 unless the replay driver uses TLS; record which.
- The model excludes client CPU; for small objects on NP-LAN client CPU may dominate, which is why NP-LAN comparisons also report measured local CPU from EXP-REMOTE-002.
- The emulator's cache layer is disabled here (not revision-aware, harness review L-12).

## 15. Held-out (Phase D placeholder)

Calibration uses no corpus content. Phase D reruns P1-P5 immediately before held-out remote runs if `env_id` changed since this run (§4.7 rule "repeated whenever env_id changes"); held-out access follows `decision-method.md` §5.5 only.

## 16. Estimates

- Compute: about 6 machine-hours (P6 replays at NP-INTER dominate; A3 doubles A1), exclusive timing window.
- Disk: under 2 GB (raw request logs).
- Agent effort: scripted execution; judgment only for the DEC-ACC-063 method disposition and for FG-7 failure handling.
