# EXP-REMOTE-006: Remote fetch concurrency model

| Field | Value |
|---|---|
| Status | **NEEDS_TOOLING** (T1 `ebr-remote exec-plan` concurrent executor with per-Chunk verification, T2 multi-connection model, T3 shared-link shaping, 429/connection-cap faults, T5 batched fetch plans, T7 kernel-TCP arm) |
| Domain / program section | remote / §12 (§14 deterministic parallelism) |
| decision_ids | DEC-ACC-043 |
| Decision type (provisional) | EMPIRICAL (+ FORMAL determinism argument for report fields) |
| OD / HC (provisional until G-A) | OD-12 (M12.3 primary; M12.1, M12.2), OD-13 not applicable (I/O concurrency, not CPU speedup); HC-11 and HC-13 (outputs and verification reports invariant under concurrency), HC-15, HC-06 (bounded in-flight buffers) |
| Author / L7 / gates | As EXP-REMOTE-001. Emulated corroboration is decisive for disagreements (section 10) and needs EXP-REMOTE-001 calibration passed with `--shared-link true`. |

## 1. Question

Which remote fetch concurrency model (serial blocking, fixed N HTTP/1.1 connections, adaptive concurrency, HTTP/2 multiplexing, multi-range single requests, or a caller-supplied executor over a deterministic fetch plan) minimizes remote task latency for single-entry, many-entry and whole-archive reads on high-RTT and bandwidth-limited links, under server connection limits, while keeping outputs, verification states and report fields deterministic?

## 2. Hypotheses

- **H1.** For WL-SUBTREE and WL-FULL at NP-CONT and NP-INTER, `fixed-n-connections-8` and `http2-multiplexing-16` are `DISTINGUISHABLE_BETTER` than `status-quo-serial-blocking` by at least 3 band units of `remote_task_latency_s`; for WL-RAND1 on small files all candidates are `EQUIVALENT` (a single small entry offers nothing to parallelize after header-payload merging).
- **H2.** Beyond the point where the summed per-connection windows saturate the link, N = 16 is not better than N = 8 (link-limited), and under a server cap of 6 connections `adaptive-aimd` is `NON_INFERIOR` to the best fixed N while fixed N = 16 degrades.
- **H3 (determinism).** With reports defined over the fetch plan (sorted by offset, not by completion order), every output digest, verification state and report field except timing is identical across 20 repetitions with seeded scheduling perturbation for every concurrent candidate.
- **H0 / falsification.** H1 falsified if neither concurrent candidate reaches the 3-band-unit gain at NP-INTER on WL-SUBTREE or WL-FULL, or if any candidate is `DISTINGUISHABLE_BETTER` on WL-RAND1 small files by more than 1 band unit; H2 falsified if N = 16 is `DISTINGUISHABLE_BETTER` than N = 8 at every profile or if AIMD is `DISTINGUISHABLE_WORSE` than best fixed N under the cap; H3 falsified by any differing field.

## 3. Decisions informed

| Decision | Supplied here | Remaining |
|---|---|---|
| DEC-ACC-043 | Latency/throughput of serial vs N-connection vs HTTP/2 vs multi-range fetch on high-RTT and bandwidth-limited links (loss excluded from performance claims by §4.15) for single-entry, many-entry and whole-archive reads; behaviour under emulated connection caps and 429 responses; proof that report fields are deterministic or labelled | Real server/CDN rate-limit and connection-limit behaviour (GitHub Releases, S3, CloudFront, R2): EXP-REMOTE-013 (BLOCKED) plus primary-source documentation review (analytical); dependency and binary-size impact of async/HTTP2 stacks: ecosystem domain (§31 dependency audit); HTTP/3: unmeasurable here (section 4) |

## 4. Candidates

| candidate_id | Definition | Realization |
|---|---|---|
| `status-quo-serial-blocking` (reference) | production reqwest blocking client, one range at a time (`random_access.rs:173-357`) | production client through the emulator |
| `fixed-n-connections-<N>`, N ∈ {2, 4, 8, 16} | the operation's fetch plan (after metadata) is split across N persistent HTTP/1.1 connections, one request in flight per connection, per-Chunk digest verification before release, output assembled in plan order | `ebr-remote exec-plan` (T1) over `fetchsim` batched plans (`header-payload-merge` coalescing from EXP-REMOTE-003 applied first, so concurrency is not credited with coalescing gains) |
| `adaptive-aimd` | start at 2 connections; +1 after a plan batch whose throughput improved by more than the T-05b relative band; halve on a 429/503 or on a batch latency increase beyond the T-10 relative band; cap 16 | executor variant |
| `http2-multiplexing-<S>`, S ∈ {4, 16, 100} | one HTTP/2 connection (prior-knowledge h2c against `ebr-origin --h2 true`; TLS modelled) with up to S concurrent streams | executor variant |
| `multi-range-single-request` | ranges of a batch sent as `multipart/byteranges` requests of up to 16 ranges on one connection | executor variant against `ebr-origin --multi-range true` |
| `caller-supplied-executor` | library exposes the deterministic fetch plan (sans-io); caller performs I/O. Its latency equals the executor the caller chooses (measured as `fixed-n-connections-8`); what is measured specifically is plan determinism (H3) and the plan's API size (C6 count, secondary) | plan emission in `ebr-remote` |
| `http3-quic` | HTTP/3 where available | **Unmeasured (not excluded):** the harness origin and emulator are TCP-only and no UDP delay/rate emulation exists; recorded as a PLATFORM/tooling gap; real-path evidence would come from EXP-REMOTE-013 |
| critic slot | R0 item 6 | before first look |

## 5. Corpus selectors

Tuning archives (EXP-REMOTE-002 `balanced-plain`): the E12 subset of EXP-REMOTE-003 plus 4 items with ≥ 10,000 regular files and 4 items with a regular file ≥ 256 MiB, selected by seeded draw from manifest fields `file_count` and item size only (list committed before the run by `make_subset.py`), total ≥ 10 groups over ≥ 5 families. Validation look for W vs S; held-out section 15.

## 6. Environment and platform requirements

`wsl-ubuntu`; `ebr-origin` + `ebr-netem-proxy --shared-link true --setup-rtt-multiple 2.0` at NP-METRO, NP-CONT, NP-INTER (NP-LAN only if calibrated); connection-cap arm `--max-connections 6`; 429 arm (T3 fault `status-429` at a seeded 5% of requests beyond 6 concurrent); client pinned `mt-v-4` {2..5} for concurrent candidates and `st-v` for serial (placement is not a compared variable: all latency is network-dominated, and client CPU time is reported); kernel-TCP arm A3 (`ebr-tunem`) for N ∈ {1, 8} and h2-16 if FG-7 passed. Exclusive quiet window. Linux x86-64.

**Platform matrix.** Linux x86-64 required (WSL2); Windows: not used; macOS: not applicable; ARM64: not required; network: loopback emulation (HTTP/3 not emulable); privileged: WSL root only for the kernel-TCP arm; large storage: no; external review: no.

## 7. Commands

```sh
cd /mnt/d/Projects/entrybound/entrybound; PY=/root/eb-research/venv/bin/python; export PYTHONPATH=research/tools
python research/orchestration/disk_guard.py && python research/harness/lock_parity.py
$PY research/tools/remote/fetchsim.py plans --strategy batched-header-payload-merge --items research/experiments/EXP-REMOTE-006/items.txt \
    --workloads research/experiments/EXP-REMOTE-002/workloads.json --out /root/eb-research/raw-detail/EXP-REMOTE-006/plans
$PY research/experiments/EXP-REMOTE-006/make_profile_specs.py        # spec-NP-METRO.yaml, spec-NP-INTER.yaml from spec.yaml (profile substitution)
for P in NP-CONT NP-METRO NP-INTER; do
  S=research/experiments/EXP-REMOTE-006/spec$( [ $P = NP-CONT ] || echo -$P ).yaml
  $PY -m ebr run --spec $S --timing && $PY -m ebr verify-raw --spec $S && $PY -m ebr normalize --spec $S
done
$PY research/tools/remote/latency_model.py multi --plans /root/eb-research/raw-detail/EXP-REMOTE-006/plans \
    --connection-model research/experiments/EXP-REMOTE-001/connection-model.json --out research/raw/EXP-REMOTE-006/model/
$PY research/experiments/EXP-REMOTE-006/determinism_stress.py --reps 20 --items research/experiments/EXP-REMOTE-006/items.txt   # H3
$PY -m decide analyze --experiment EXP-REMOTE-006
```

## 8. Seeds, warmups, repetitions

Seeds `order` 20261002, `bootstrap` 20261003, `command` 20261004 (plan batches, fault schedules, scheduling perturbation). Emulated timing: warmups 2 (1 for ≥ 256 MiB items), rounds min 10, step 10 (medium) or 5 (large), cap 20, precision-based adaptive rule on the item-level median paired log ratio vs the reference. Determinism stress: 20 repetitions per (item, candidate) at the maximum concurrency with seeded per-request delay perturbation (uniform 0-5 ms) injected by the proxy, matching the objective MVT-13(a) stress-cell count.

## 9. Metrics

| Metric | OD | Role | Threshold |
|---|---|---|---|
| `remote_task_latency_s` per (profile, workload) — modelled (CM-H1-TLS-N / CM-H2-TLS link-sharing model, SS-RFC6928) | OD-12 | **primary** (§4.15) | T-10 |
| emulated `remote_task_latency_s` at calibrated profiles | OD-12 | corroboration; same direction required (§4.15) | T-10 |
| `http_requests`, `http_bytes` | OD-12 | secondary | T-12, T-11 |
| `client_cpu_s` (executor process) | OD-07 | secondary | T-05 |
| `alloc_peak_bytes` of in-flight buffers (scoped RSS until §14 item 17) | OD-08 | guard | T-06 |
| `report_field_mismatches` across the 20-repetition stress | – | binary (HC-11/HC-13) | §3.3 |
| `roundtrip_ok` (output digest equals status quo) | – | binary | §3.3 |

## 10. Normalization and statistical analysis

- Timing protocol per §4.3 (WSL), §4.6 repetitions, §4.10 exact item intervals, Welch-Satterthwaite corpus intervals, verdicts per §4.11; confirmatory W vs S on validation with confidences of §4.12 and the MDE gate.
- The modelled value is primary (§4.15). Because concurrent transfer latency depends on congestion dynamics that both the fluid link-sharing model and the application-level emulator only approximate, the §4.15 same-direction requirement is applied per condition and, where model and emulator disagree, the kernel-TCP arm A3 is consulted: if A3 agrees with the model the selection may proceed with the disagreement recorded; if A3 is unavailable or agrees with neither, the decision stays `INSUFFICIENT_EVIDENCE` for that condition. No threshold or rule of the method is changed.
- Strata: profile × workload × cap arm; never averaged.
- R9 partitions: local vs remote source; then workload class (single-entry vs multi-entry vs whole-archive), expressible if the API distinguishes batch reads (T2 API addition charged).
- Tiers (independent assessment): executor-internal concurrency without user-visible options T0/T1; user-visible concurrency flags or a sans-io plan API T2; an HTTP/2 or async stack as a new reader-path dependency is C3-assessed (possibly T3 if it contains `unsafe` or native code).

## 11. Practical significance

T-10, T-12, T-11, T-05, T-06 by reference; binaries §3.3; minimum gains per computed tier.

## 12. Sensitivity analysis

Slow-start arms in the model (each connection's own slow start is the crux of N-connection gains); SS-NONE vs A3 comparison; connection cap 6 and 429 fault arms; bandwidth-halved and RTT-doubled profile perturbations (reported, non-canonical); plan batch size 64 vs 1,024 ranges; §6.3-§6.7 arms.

## 13. Expected negative results worth recording

- No gain for single small entries; possible loss for NP-LAN where client overhead dominates.
- N = 16 no better than N = 8 once the link saturates; worse under a 6-connection cap.
- HTTP/2 with one shared window may lose to N connections at NP-INTER in slow-start-limited transfers (a result the application-level emulator alone would not show).
- HTTP/3 cannot be evaluated locally.

## 14. Threats to validity

- Application-level emulation lacks per-connection congestion interplay; the shared-link token bucket is a fluid approximation (mitigated by A3 and the model agreement rule).
- No loss: concurrency's resilience benefits under loss are not claimed.
- Server-side limits are emulated with fixed caps and scripted 429s, not real CDN policies.
- The executor is a research prototype; production integration could add overhead (reported as client CPU).

## 15. Held-out (Phase D placeholder)

Frozen W and reference rerun once on held-out archives through the calibrated emulator at the same profiles after Commit C (`decision-method.md` §5.5), with the determinism stress repeated; report-only per §5.6.

## 16. Estimates

- Compute: about 14 machine-hours of exclusive timing window (3 profiles × 9 candidates × 20 items × ≥ 10 rounds, network-dominated, dominated by NP-INTER serial runs; A3 adds about 4 h).
- Disk: under 3 GB (plans, logs).
- Agent effort: scripted; judgment for tier assessments, the emulated-vs-model disagreement disposition and analysis.
