# EXP-CHUNK-009: Local and remote access cost by chunk size policy

## 1. Pre-registration

| Field | Value |
|---|---|
| Experiment ID | EXP-CHUNK-009 |
| Status | NEEDS_TOOLING. Required tools: `ebr-access --mode archive --archive-path` (open an existing INDEXED archive, including research archives from `ebr-cdcpack`) with `--source-kind http-spawn-origin` (loopback `ebr-origin`, exact request log) and `--verified-range`; the §4.15 latency model reading `protocol.network_profiles` and `protocol.latency_model` from `research/methods/thresholds.json`; the per-profile connection model from `EXP-NETEM-CAL` (§14 item 18). Timing arms also need §14 items 3-4 (runner additions, calibration). The emulated arm needs `calibration.md` regenerated in a quiet window (review use constraint 7). |
| Arms | **D** deterministic bytes, requests and modelled latency (`spec.yaml`). **T** in-process local latency (`first_entry_latency_s`, `random_entry_latency_s`, `open_latency_s`) and full extraction `decode_wall_s` (`spec-timing.yaml`, generated). **E** emulated-network corroboration through `ebr-netem-proxy` at NP-METRO, NP-CONT, NP-INTER (`spec-emulated.yaml`, generated after calibration). **W/S** arms for the EXP-CHUNK-004 winners (generated). **V1** validation. **H** placeholder. |
| decision_ids | DEC-CMP-002 (extraction and random-read latency and bytes per entry by size policy, local and remote; `decode_wall_s` guard), DEC-CMP-001 (random-access cost per construction for W/S), DEC-CMP-017 (single-entry extraction under dedup indirection, remote form) |
| Proposed od_ids | DEC-CMP-002 adds OD-07, OD-10, OD-11, OD-12 to its EXP-CHUNK-004 set |
| Proposed hc_ids | HC-15 (verification honesty of partial reads), HC-18 (origin safety; owned by the remote domain), HC-09 |
| Gates / author / L7 | As EXP-CHUNK-001 section 1 |

## 2. Question

How do chunk size policies (and, for W/S, constructions) change the cost of local and remote access to INDEXED archives? Specifically:

- bytes read to open;
- bytes and HTTP range requests per random entry and per 4 KiB range;
- verified-range fetch bytes;
- modelled remote latency per network profile;
- local first/random entry latency;
- full extraction time.

## 3. Hypotheses

- **H-D1.** Above 64 KiB targets, `http_bytes_range_4k_mean` and `verified_range_fetch_bytes` grow roughly proportionally to target chunk size. `sq-fast` is `DISTINGUISHABLE_WORSE` than `sq-balanced` on `verified_range_fetch_bytes` by at least 1 band unit (T-11). **Falsified if** not worse, or worse by less than 1 band unit, at corpus level.
- **H-D2.** `http_requests` per random entry is `EQUIVALENT` (T-12) across the size grid for items whose median entry is smaller than the minimum chunk size (F01, F03, F04, F19). **Falsified if** distinguishable on those families.
- **H-D3.** `open_bytes_read` increases as target size falls (more Index records). `gear-norm-v1-t64k-balanced-codec` is `DISTINGUISHABLE_WORSE` than `sq-balanced` on `open_bytes_read` on F17/F18. **Falsified if** not.
- **H-D4 (modelled remote).** At NP-CONT and NP-INTER the modelled `remote_random_entry_latency_s` of `sq-fast` is worse than `sq-balanced` by at least 1 band unit (T-10) on families with large files (F05-F08, F13, F15, F16). At NP-METRO the two are `EQUIVALENT`. **Falsified** for each profile where the verdict differs.
- **H-T.** Local `random_entry_latency_s` does not distinguish `sq-balanced` from `sq-dense` (T-08). `decode_wall_s` is `EQUIVALENT` across frozen presets for `balanced` codec policy.
- **H0.** Size policies are equivalent on all access metrics.
- **Binary checks:**
  - `verified_fraction` = 1 for chunk-digest-verified range reads (HC-15: no unverified bytes reported as verified);
  - request and byte counts from the spawned origin equal the proxy-free exact counts (tool validity).

## 4. Decisions informed

| Decision | Contribution |
|---|---|
| DEC-CMP-002 | Ledger evidence "Extraction and random-read latency and bytes per entry by size policy, local and remote"; the profile-scope guard `decode_wall_s` (Appendix B.1) for W_P vs S_P; OD-11/OD-12 secondary evidence for per-profile Pareto reports |
| DEC-CMP-001 | Random-access and remote cost of W_A vs S_A at matched sizes (generated W/S arm) |
| DEC-CMP-017 | Remote single-entry extraction under dedup indirection (request counts measured on actual archive-wide archives; alternative scopes are modelled in EXP-CHUNK-007) |

Range-coalescing policy, concurrency and cache defaults are not settled here: DEC-ACC-039 and DEC-ACC-046 belong to the remote domain, which owns the slow-start sensitivity (review R1-14). This experiment holds `RandomAccessPolicy` at production defaults.

## 5. Candidates

| Candidate | Definition | Role |
|---|---|---|
| `sq-fast`, `sq-balanced`, `sq-dense`, `sq-extreme` | Frozen presets with their own codec policies and frozen proxy selection | **Status quo**; `sq-balanced` is the **reference** |
| `gear-norm-v1-t{64k,256k,1m,4m}-balanced-codec` | Single size set (t/4, t, 4t) under `balanced` codec and cohort policy | Size curve for DEC-CMP-002 (`fastcdc-reference-small`-scale through `large-4m`-scale) |
| W/S arm (generated) | W_P and S_P per profile, W_A and S_A from EXP-CHUNK-004 | Confirmatory inputs |

- Exclusions as EXP-CHUNK-004 section 5.
- R0 item 6: critic candidate open.

## 6. Corpus

- **Arm D and arm T tuning:** the 40-item timing subset of EXP-CHUNK-008 (two items per family from distinct groups; ids in `spec.yaml`).
- **Arm E:** 10 items (one per family for F01, F03, F05, F06, F07, F12, F13, F15, F16, F18, first by id), to bound emulation time.
- **V1:** the validation counterpart per EXP-CHUNK-008 section 6, registered as part of the owning decision's look.
- **Held-out:** Phase D placeholder (§5.5; EXP-CHUNK-004 section 6).

## 7. Environment and platform

- **Arm D:** WSL x86-64, loopback origin (no latency emulation). Deterministic counts only.
- **Arm T:** WSL `st-v` and Windows `st-p` (in-process timers; batches ≥ 1,000 operations or ≥ 40 sampled operations per round for p95, T-08).
- **Arm E:** WSL, `ebr-netem-proxy` with achieved RTT from logs.
  - RTT < 20 ms profiles are not emulated (use constraint 7), so NP-LAN is model-only.
  - Every result is labelled `emulated-network`.
  - Slow start is absent in the emulator (R1-14), so the modelled value (which includes the RFC 6928 initial window) is primary, and emulated values must agree in direction (§4.15).
- **Network:** loopback only; no external network.

## 8. Commands

```sh
# preconditions/builds as EXP-CHUNK-004/008
$PY -m ebr validate --spec research/experiments/EXP-CHUNK-009/spec.yaml
$PY -m ebr run --spec research/experiments/EXP-CHUNK-009/spec.yaml --gzip
$PY -m ebr verify-raw --spec research/experiments/EXP-CHUNK-009/spec.yaml --refingerprint
$PY -m ebr normalize --spec research/experiments/EXP-CHUNK-009/spec.yaml
# EXP-NETEM-CAL must be regenerated in a quiet window and pass the section 4.15 acceptance criteria first
$PY research/experiments/EXP-CHUNK-009/make_specs.py --arms timing,emulated,w-s \
    --w-s research/normalized/EXP-CHUNK-004/decision/w-s.json --out-dir research/experiments/EXP-CHUNK-009
#   timing command: ebr-access --mode archive --archive-path <prebuilt> --source-kind local-file --batch-ops 1000 ...
#   emulated command: ebr-access --mode archive --source-kind http --source-url <proxy> (proxy at NP-METRO|NP-CONT|NP-INTER)
$PY -m ebr run --spec research/experiments/EXP-CHUNK-009/spec-timing.yaml --timing --gzip       # exclusive window
$PY -m decide analyze --experiment EXP-CHUNK-009 --out research/normalized/EXP-CHUNK-009/decision/
```

## 9. Seeds

- **Spec seeds:** `{order: 2026091791, bootstrap: 2026091792, command: 2026091793}`.
- **Samples:** entry and range samples use seed 20260917 (candidate-independent, so ranges pair across candidates).
- **Warmups:** arm D 0; arm T 2 (small/medium) with hot cache (§4.5).
- **Arm E:** 3 warm-up requests per session (discarded) before measured reads.

## 10. Metrics

| Metric | Canonical | OD | Role | Threshold |
|---|---|---|---|---|
| `verified_range_fetch_bytes` (4 KiB ranges) | yes | OD-11 | **primary** for OD-11 (DEC-CMP-002 secondary scope evidence; guard reporting) | T-11 |
| `http_bytes` (as `http_bytes_random_entry_mean`, `http_bytes_range_4k_mean`) | yes | OD-12 | **primary** for OD-12 (per network profile, AGG-S) | T-11 |
| `http_requests` (`http_requests_open`, `http_requests_random_entry_mean`) | yes | OD-12 | secondary (one primary per OD) | T-12 |
| `remote_random_entry_latency_s`, `remote_first_entry_latency_s` (modelled per profile; emulated corroboration) | yes | OD-12 | secondary (a latency claim needs model and emulation agreeing in direction) | T-10 |
| `open_bytes_read` | yes | OD-10 | secondary | T-11 |
| `verification_digest_ops` | yes | OD-11 | diagnostic | T-12 |
| `verified_fraction` | yes | OD-11 | binary check here (must be 1 for digest-verified reads) | T-20 / §3.3 |
| `random_entry_latency_s`, `first_entry_latency_s`, `open_latency_s` (arm T, in-process only) | yes | OD-10, OD-07 | `random_entry_latency_s` **primary** for OD-10 timing in DEC-CMP-002 guard reporting; others secondary | T-08 |
| `decode_wall_s` (arm T, `ebr-unpack` full extraction, in-process timer) | yes | OD-07 | **profile-scope guard** for DEC-CMP-002 | T-04 |
| `artifact_bytes`, `archive_sha256`, `request_log_sha256` | yes/no | — | cross-reference to EXP-CHUNK-004; repeat-hash and provenance | T-01 / §3.3 |

**OD-10 primary.** DEC-CMP-002 already declares `access_amplification` (EXP-CHUNK-004) as its OD-10 metric for DEC-CMP-001. For DEC-CMP-002 (profile scope) the OD-10 guard is `access_granularity_bytes`. `random_entry_latency_s` is therefore reported as a guard-supporting secondary unless the decision record declares otherwise before V1. The record must choose one primary per OD.

## 11. Replication

- **Arm D:** 2 repetitions and repeat-hash (counts are exact).
- **Arm T:** §4.6 minimum, step and cap by tier, with the adaptive precision rule (`precision_step.py` of EXP-CHUNK-008), ≥ 40 operations per item per round for p95.
- **Arm E:** 10 rounds per (candidate, item, profile), randomized blocks. It is corroboration only, so no adaptive rule applies. A disagreement in direction with the model voids the latency claim for that profile.

## 12. Analysis

- **Arm D:** L1 exact values. AGG-S over network profiles, never averaged across profiles. L2-L4 per profile as EXP-CHUNK-004 section 12.
- **Modelled latency:** RTT × (setup + request round trips) + bytes / bandwidth + slow-start rounds (§4.15), under the connection model of the `EXP-NETEM-CAL` record.
- **Arm T:** exact order-statistic item intervals, Welch-Satterthwaite corpus intervals (§4.10), verdict classes (§4.11).
- **Profile-scope guard evaluation for DEC-CMP-002:** W_P vs S_P `NON_INFERIOR` at 0.99 and family harm guard at 0.90 on `decode_wall_s` (and the OD-10 guard from EXP-CHUNK-004).
- **Pareto report per family and per network profile:** (`artifact_bytes` from EXP-CHUNK-004, `verified_range_fetch_bytes`, `http_bytes_range_4k_mean`, modelled latency). Informational.
- **Labels:** everything remote is `EMULATED` or `MODELLED`, and every WSL timing is `VIRTUALIZED`.

## 13. Thresholds

T-04, T-08, T-10, T-11, T-12 as transcribed in `research/methods/thresholds.json` `metric_thresholds` (T-10's RTT-dependent absolute threshold is computed per profile from `protocol.network_profiles`). Not restated.

## 14. Sensitivity

- §6.4 and §6.5 arms for the guard comparisons.
- Profile-by-profile reporting (no averaging).
- **Slow-start arm:** the model with the initial window disabled versus RFC 6928 (reported beside the corroboration, R1-14).
- Range length 4 KiB versus 1 MiB (10 items).
- Sample size 1,000 versus 4,000 (10 items).
- Source kind local-file versus spawned origin for byte counts (must be identical).

## 15. Expected negative results

- Size policies differ on remote bytes but not on request counts.
- Latency differences at NP-METRO are inside T-10 for all presets.
- The frozen `fast` preset is dominated remotely by `balanced` on every family.
- Local latency does not separate `balanced` and `dense`.
- The emulator and model disagree at NP-INTER because the emulator lacks slow start.

## 16. Threats to validity

- Application-level emulation (no packet loss dynamics, no slow start): model primary, emulation corroborative, remote domain owns calibration.
- Production `RandomAccessPolicy` defaults are held fixed; different coalescing could reorder policies (remote domain sensitivity).
- Loopback-origin counts assume one connection model; HTTP/2 multiplexing is not modelled unless the calibration record declares it.
- Research archives for non-frozen sizes are readable by production readers (decoders never run the chunker), so `ebr-access` measures real production decode paths.
- The 40-item subset limits family coverage to 2 groups per family: minimum support holds at tuning but family guards are weak (§4.9 limitation).

## 17. Estimates

| Arm | Estimate |
|---|---|
| D | 8 candidates × 40 items × 2 reps; pack about 4.9 GiB per candidate (extreme slowest) plus 1,000 entries and 16 ranges per large entry: about **30 CPU-hours** |
| T | about 8 candidates × 40 items × 12-22 rounds × batch ≥ 1 s, plus cooldown: about **40 wall-hours per environment** |
| E | 8 × 10 items × 3 profiles × 10 rounds, sessions of seconds to minutes at NP-INTER: about **30 wall-hours** |

- **Disk:** request logs about 3 GB; archives transient.
- **Agent effort:** none during execution; judgment for the latency-model integration and analysis.

## 18. Tooling to build

1. **`ebr-access --mode archive`:** `--archive-path`, `--source-kind local-file|http|http-spawn-origin`, `--verified-range`, `--batch-ops`, `--request-log-out`, `--latency-model <thresholds.json>`, `--connection-model <json>`. Payload fields `open.*`, `random_entry.*`, `random_range_4k.*`, `model.<profile>.*` as named in `spec.yaml`.
2. **Latency model module** implementing `decision-method.md` §4.15 (shared with the remote domain if it builds one; one implementation program-wide).
3. **`research/raw/EXP-NETEM-CAL/connection-model.json`** from the quiet-window calibration (owned by the remote domain).
4. **`research/experiments/EXP-CHUNK-009/make_specs.py`.**

## 19. Deviations

(append-only)
