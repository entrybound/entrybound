# EXP-REMOTE-003: Range coalescing policy and physical-order locality for remote random access

| Field | Value |
|---|---|
| Status | **NEEDS_TOOLING** (T1, T2, T4 `LayoutPrefetchSource`/`ReadaheadSource`, T5 `fetchsim` validated by EXP-REMOTE-002 H4, T3 multipart logging) |
| Domain / program section | remote / §12 (§9 and §8.3 for the ordering decisions) |
| decision_ids | DEC-ACC-046, DEC-ACC-003, DEC-CMP-006 |
| Decision type (provisional) | EMPIRICAL (all three; DEC-CMP-006 only for its remote leg) |
| OD / HC (provisional until G-A) | OD-12 (M12.1-M12.3), OD-10 (M10.1 local guard), OD-05 guard for ordering (supplied by crossfile domain); HC-15 and HC-03 (coalescing must not change outputs or reports), HC-16 (ordering changes need new planner IDs), HC-18 not exercised |
| Author / L7 / gates | As EXP-REMOTE-001. Requires EXP-REMOTE-001 pass for emulated numbers and EXP-REMOTE-002 H4 = pass for simulated-only candidates. |

## 1. Question

Which range-coalescing policy (gap threshold, merged-length cap, cross-purpose merging, header-with-payload fetch, HTTP multi-range, RTT- or chunk-size-derived gaps) minimizes modelled remote task latency without disproportionate byte waste across the canonical network profiles and user tasks, and how much do alternative physical Chunk orders (file locality, hot-set prefix, trace-derived order, the SPEC clustering-mode orders) change request counts and latency for the same stored bytes?

## 2. Hypotheses

- **H1 (DEC-ACC-046).** On NP-CONT and NP-INTER, at least one candidate that merges ChunkHeader reads with their payload (`header-payload-merge` or `cross-purpose-coalescing`) is `DISTINGUISHABLE_BETTER` than `status-quo-4k-same-purpose` on `remote_task_latency_s` for WL-SUBTREE and WL-RANDK16 by at least 1 band unit, and `NON_INFERIOR` on `http_bytes`. Predicted effect: 2-6 band units (INFERRED from the one-header-request-per-Chunk path, `random.rs:754-790`).
- **H2 (gap threshold).** No single fixed gap is the R8 winner across all four profiles: the best fixed gap grows with RTT × bandwidth, so the universal winner, if any, is `rtt-adaptive-gap` or a header-merge candidate whose advantage does not depend on the gap.
- **H3 (DEC-ACC-003, DEC-CMP-006 remote legs).** `file-locality-order` reduces `http_requests` for WL-SUBTREE by at least 1 band unit versus the status-quo cohort/digest order under every coalescing candidate with gap ≥ 64 KiB, and is equivalent for WL-RAND1.
- **H4 (robustness).** H1's winner is `slow_start_robust` (connection-model.json rule).
- **H0 / falsification.** H1 falsified if no header-merge candidate reaches `DISTINGUISHABLE_BETTER` on either profile for either task, or if it is `DISTINGUISHABLE_WORSE` on `http_bytes`; H2 falsified if one fixed gap is the R8 winner at all four profiles; H3 falsified if file-locality ordering is not `DISTINGUISHABLE_BETTER` on WL-SUBTREE; H4 falsified by a slow_start_dependent flag.

## 3. Decisions informed

| Decision | Supplied here | Remaining |
|---|---|---|
| DEC-ACC-046 | The pre-registered sweep over gap × RTT × bandwidth (canonical profiles replace the ledger's 1/20/80/250 ms list, per `decision-method.md` §4.15 which makes the four profiles the only network profiles), requests, bytes, wasted bytes, p50/p95 latency for open, single-entry and many-small-entry reads; the differential verification test (outputs and reports identical with coalescing on and off); ZIP-over-HTTP and eStargz request-pattern comparison is in EXP-REMOTE-012 | Loss is excluded from performance claims (§4.15), so the ledger's "x loss" axis is covered only as failure behaviour in EXP-REMOTE-009; multipart support survey across S3, GCS, Azure, R2, CloudFront, GitHub Releases, nginx, Caddy, Apache: local servers in EXP-REMOTE-009, cloud endpoints in EXP-REMOTE-013 (BLOCKED); real-path slow-start confirmation: EXP-REMOTE-013 |
| DEC-ACC-003 | Time-to-first-useful-read and request counts per ordering on the tuning corpus, including container-like (F16 OCI) and model-weight (F07) items | Ratio impact of locality ordering: crossfile/planner domains; hot-set list semantics (who declares it) is an API/FORMAL question; POST_V1 release relevance |
| DEC-CMP-006 | Extraction locality and remote range counts per ordering (its "Task 12 harness" evidence line) | Ratio effect, planning CPU and STREAM window per clustering mode: crossfile and planner domains |

## 4. Candidates

### 4.1 Coalescing (DEC-ACC-046); applied to every access purpose unless stated

| candidate_id | Definition | Realization |
|---|---|---|
| `status-quo-4k-same-purpose` (reference) | merge sorted prefetch ranges of identical purpose when gap ≤ 4096 B and merged length ≤ 64 MiB (`random_access.rs:544-580`); Chunk headers fetched individually | production policy default |
| `no-coalescing` | one request per needed range | production with `coalesce_gap_bytes = 0` (payload ranges are separated by frame headers, so no payload merges occur; exactly adjacent ranges still merge, noted) plus a strict `fetchsim` variant |
| `fixed-gap-<g>`, g ∈ {16 KiB, 64 KiB, 256 KiB, 1 MiB, 4 MiB} | same-purpose merging with gap g, merged cap 64 MiB | production policy knob `coalesce_gap_bytes` |
| `merge-cap-<c>`, c ∈ {1 MiB, 8 MiB} with g = 1 MiB | as fixed gap, merged length capped at c | `LayoutPrefetchSource` + `fetchsim` (production ties the merge cap to `max_individual_range_bytes`, which also refuses larger single reads; not used for this) |
| `cross-purpose-coalescing` | Chunk, Lookback, Dictionary, Reconstruction and metadata ranges merge when within gap g ∈ {4 KiB, 64 KiB} | `LayoutPrefetchSource` + `fetchsim` |
| `header-payload-merge` (designer-proposed) | the Index locator's `stored_len` lets the client fetch frame header and payload as one range per Chunk, then apply the status-quo same-purpose merge over these extended ranges; header validation unchanged (header still checked against the locator before decode) | `LayoutPrefetchSource` + `fetchsim` |
| `rtt-adaptive-gap` | gap = floor(bw_bps × rtt_s / 8) bytes clamped to [4 KiB, 4 MiB], from RTT and throughput measured on the session's first two requests (merging a gap is worthwhile when the gap transfers faster than one RTT) [INFERRED cost rule] | `LayoutPrefetchSource` (measures its own RTT) + `fetchsim` (uses the profile values) |
| `chunk-size-proportional-gap-<k>`, k ∈ {0.5, 1, 2} | gap = k × median stored Chunk length from the Index | production knob per archive + `fetchsim` |
| `http-multi-range` | up to 16 ranges per request as `multipart/byteranges`, falling back to single ranges on a 200 or single-part 206; ETag and Content-Range rules applied per part | `LayoutPrefetchSource` against `ebr-origin --multi-range true --max-ranges 16` + `fetchsim` |
| `planner-locality-plus-coalescing` | `header-payload-merge` with gap 64 KiB over the `file-locality-order` permutation of section 4.2 | `fetchsim` over permuted layouts |
| critic slot | reserved for the R0 item 6 completeness critic's candidate | added before the first validation look, enters at R1 |

Exclusions: none. Note on HC-15/HC-03: every candidate must produce byte-identical outputs and identical verification reports except for the `bytes_fetched`, `range_request_count` and `access_trace` fields; a difference is an R1 elimination (differential test, section 9).

### 4.2 Physical order (DEC-ACC-003, DEC-CMP-006 remote legs); stored frame bytes held fixed

| candidate_id | Order of Chunk frames inside CHUNK_DATA |
|---|---|
| `status-quo-cohort-digest-order` (reference) | as packed (cohort ID then digest; `docs/cross-file-compression-v1.md` L81-88) |
| `file-locality-order` | non-cohort Chunks by (path bytes of first referencing Entry, index within its ContentObject); cohort members stay contiguous at the position of their first referencing Entry |
| `path-order-base` (DEC-CMP-006) | all Chunks by (first referencing path, index); cohorts placed at first use |
| `type-then-path-ordering` (DEC-CMP-006) | by (extension-free content category as recorded by the planner, path); if the archive records no category, identical to `path-order-base` (noted, never inferred from extensions per HC-08) |
| `hot-set-prefix` | Chunks of a declared hot set first, rest in status-quo order; hot set = files read by WL-FIRST plus the WL-SUBTREE directory's first 64 files (declared from the manifest only) |
| `trace-derived-order` | frames ordered by first access time in a WL-ZIPF trace recorded with a different seed (`seeds.command + 1`), untouched frames after in status-quo order |
| `spec-clustering-fast-input-order` | (fast profile archives) Chunks in capture input order (path order of the tree walk) |

Ordering candidates are simulated only on `fast` and `balanced` archives, whose plans use lookback 0 (SPEC §7.5 L824), so reordering frames does not change stored bytes; dense/extreme reordering changes compression and is out of scope here (limitation, not a fabricated result). Each ordering is crossed with `status-quo-4k-same-purpose`, `fixed-gap-64k`, `header-payload-merge` and the tuning winner of 4.1.

## 5. Corpus selectors

- Tuning only: archives from the EXP-REMOTE-002 cache for `fast-plain` and `balanced-plain` (all tuning items), `dense-plain` and `extreme-plain` (small + medium), `balanced-enc-bucketed` (small + medium, coalescing of EncryptedPayload ranges). Layout maps and item lists by SHA-256 from EXP-REMOTE-002 outputs.
- Emulated corroboration subset E12: 12 tuning items drawn by seeded stratified sampling (seed `seeds.command`) covering ≥ 10 independence groups and ≥ 5 families, with at least 2 items of ≥ 1,000 regular files; list committed before the run by `make_subset.py` from manifest fields only.
- Validation look (at most 2, §5.2): the confirmatory set W vs S over split `validation` archives packed by the same T6 recipe; generated `spec-validation.yaml` at the look. Held-out: section 15.

## 6. Environment and platform requirements

- Simulation and trace arms: `wsl-ubuntu`, no timing. Emulated arm: `wsl-ubuntu`, loopback proxy at NP-METRO, NP-CONT, NP-INTER (NP-LAN only if EXP-REMOTE-001 passes it), quiet timing window, client `st-v`. Kernel-TCP arm A3 on RP-1 and RP-2 if EXP-REMOTE-001 FG-7 passed.
- Linux x86-64; no privileges beyond WSL root for A3; no large storage beyond the EXP-REMOTE-002 cache.

**Platform matrix.** Linux x86-64 required (WSL2); Windows: not used; macOS: not applicable; ARM64: not required (deterministic request plans); network: loopback emulation; privileged: WSL root only for the kernel-TCP arm; large storage: no (reuses the EXP-REMOTE-002 cache); external review: no.

## 7. Commands

```sh
cd /mnt/d/Projects/entrybound/entrybound; PY=/root/eb-research/venv/bin/python; export PYTHONPATH=research/tools
python research/orchestration/disk_guard.py && python research/harness/lock_parity.py
# (a) production-knob candidates: exact traces through the production client
$PY -m ebr run --spec research/experiments/EXP-REMOTE-003/spec.yaml
$PY -m ebr verify-raw --spec research/experiments/EXP-REMOTE-003/spec.yaml && $PY -m ebr normalize --spec research/experiments/EXP-REMOTE-003/spec.yaml
# (b) simulated and prototype candidates (fetchsim over cached layouts; prototypes spot-validated against origin logs)
$PY research/tools/remote/fetchsim.py sweep --layouts /root/eb-research/cache/remote-archives --items research/experiments/EXP-REMOTE-002/items-tuning-all.txt \
    --strategies research/experiments/EXP-REMOTE-003/strategies.json --workloads research/experiments/EXP-REMOTE-002/workloads.json \
    --connection-model research/experiments/EXP-REMOTE-001/connection-model.json --out research/raw/EXP-REMOTE-003/sim/
$PY research/tools/remote/prototype_check.py --strategies research/experiments/EXP-REMOTE-003/strategies.json --subset research/experiments/EXP-REMOTE-003/E12.txt \
    --out research/raw/EXP-REMOTE-003/prototype-vs-sim.json          # origin-log counts must equal fetchsim counts exactly
# (c) emulated corroboration of the tuning W and S only (spec generated after tuning, before looking at emulated data)
$PY research/experiments/EXP-REMOTE-003/make_emulated_spec.py --tuning-selection research/normalized/EXP-REMOTE-003/decision/tuning-selection.json
$PY -m ebr run --spec research/experiments/EXP-REMOTE-003/spec-emulated.yaml --timing
# (d) decision analysis (T11)
$PY -m decide analyze --experiment EXP-REMOTE-003 --plan research/experiments/EXP-REMOTE-003/protocol.md
```

`strategies.json` (written with T5, committed before (b)) holds every candidate of section 4 with its exact parameters.

## 8. Seeds, warmups, repetitions

- Seeds: `order` 20260923, `bootstrap` 20260924, `command` 20260925.
- Deterministic arms (a), (b): 2 repetitions + trace repeat-hash (§4.6, §4.13); encrypted archives: 3 key replicates reused from EXP-REMOTE-002.
- Emulated arm (c): warmups 2; rounds min 10, step 10, cap 30 (small/medium operations) with the precision-based adaptive rule on the item-level median paired log ratio of task latency; ≥ 40 operations per item per round whenever p95 is reported (WL-RAND1 sessions are the operations).

## 9. Metrics

| Metric | OD | Role | Threshold | Source |
|---|---|---|---|---|
| `remote_task_latency_s` per (profile, workload) — modelled, CM-H1-TLS-KA, SS-RFC6928 | OD-12 | **primary** | T-10 | T2 over exact request logs |
| `http_bytes` per (profile-independent) workload | OD-12 | **second primary, justification pending reviewer (R3):** latency does not price metered egress, which T-11's rationale names; wasted bytes are the cost side of coalescing | T-11 | origin log / fetchsim |
| `http_requests` | OD-12 | secondary | T-12 | origin log / fetchsim |
| `wasted_bytes` (fetched bytes never used by decode or verification) | OD-12 | diagnostic (no band; secondary) | – | trace + layout |
| `remote_first_entry_latency_s`, `remote_random_entry_latency_s` (p50, p95) | OD-12 | secondary (task-specific views of the primary) | T-10 | T2 |
| emulated `remote_task_latency_s` | OD-12 | corroboration (same direction required, §4.15) | T-10 | arm (c) |
| `random_entry_latency_s` (local-file source, in-process batches) | OD-10 | guard (non-inferiority) for the universal scope | T-08 | `ebr-remote trace --source local-file` batches of ≥ 1,000 reads, timing window |
| `report_mismatches` (differential outputs/reports) | – | binary (R1) | §3.3 | T1 |
| `range_length_signatures` | OD-16 | secondary for encrypted archives (coalescing changes the observable length pattern) | T-12 | trace |

## 10. Normalization and statistical analysis

- Units: item = archive (item × profile config); L2 group, L3 family, L4 corpus with `workload-mix.json` weights (G-B) and equal weights as sensitivity arm; strata per scale tier, per network profile and per workload; conditions never averaged (AGG-S).
- Tuning (exploratory, R3/R4 without multiplicity): continuous band-unit utility over the primary metrics (§6.2) per condition; W = utility winner in the universal remote scope; S = survivors. Gap sweep results are tuning evidence only.
- Validation (confirmatory, R4): W vs each s ∈ S at corpus level per primary metric per condition, superiority confidence 1 − 0.01/k_sup, non-inferiority two-sided 0.99, family guard 0.90 (§4.12), MDE gate ≤ 1 band unit using tuning group variance.
- R9 partitions tried in order: policy partition "local vs remote access" (expressible: the source type is known to the client), then per network profile only for `rtt-adaptive-gap` (the only candidate that can express it), then workload mix; each partition charged T1 (§7.2) and must beat the best single candidate by the T1 minimum-gain band.
- Cost tiers (assessed by an independent session before the first look): coalescing and ordering-free candidates are client behaviour changes without wire or decode changes (T0/T1); `http-multi-range` adds a transport feature (T1, plus dependency cost if a multipart parser is added: C3); ordering candidates require new planner IDs (T1 with permanent-vector cost, HC-16).
- Differential test: 100% of tuning and validation archives, every candidate: `report_mismatches` must be 0.

## 11. Practical significance

T-10 (`remote_task_latency_s`), T-11 (`http_bytes`), T-12 (`http_requests`), T-08 (local guard), all by reference to `thresholds.json#metric_thresholds`. Minimum-gain bands per final tier (§3.1, §7.3).

## 12. Sensitivity analysis

- Slow-start arms SS-NONE, SS-IDLE-RESET (interactive gap 1 s), SS-IW4, and connection models CM-H1-PLAIN-KA, CM-H1-TLS-FRESH: winner re-computed; `decision_robustness_rule` applied (slow_start_dependent flag blocks DECIDED until A3 or EXP-REMOTE-013 confirms).
- §6.3 weight perturbation on the two primaries; §6.4 threshold perturbation; §6.5 arms (equal weights, leave-one/three-families-out, family Dirichlet, WM-* mixes, tier-stratified, byte-weighted); §6.7 selection bootstrap.
- Server-capability arm for `http-multi-range`: re-score assuming the origin returns single-part responses (fallback cost), as observed for servers that do not implement multipart in EXP-REMOTE-009.
- Kernel-TCP arm A3 on RP-1, RP-2.

## 13. Expected negative results worth recording

- Larger fixed gaps lose on NP-LAN (bytes dominate) and win on NP-INTER; no fixed gap wins everywhere (H2).
- `http-multi-range` gains may vanish once server fallback is priced.
- Ordering gains for WL-RAND1 are nil (random single files are one ContentObject regardless of order).
- `trace-derived-order` overfits its training seed and does not beat `file-locality-order` on held seeds.

## 14. Threats to validity

- Simulated candidates rest on `fetchsim` fidelity (validated only for production-expressible strategies; prototype-vs-sim check on E12 covers the rest).
- Stored bytes held fixed under reordering (exact only for lookback-0 profiles).
- Synthetic workloads; canonical profiles have no loss, jitter or cross traffic (EMULATED).
- `rtt-adaptive-gap` in simulation uses exact profile values; its prototype measures RTT from two requests and may misestimate at NP-LAN (the arm reports both).

## 15. Held-out (Phase D placeholder)

The frozen W and S are rerun once on held-out archives after Commit C (`decision-method.md` §5.5), deterministic arms only plus emulated corroboration of W vs the reference, report-only per §5.6; held-out MDE computed before unlock from validation group variance.

## 16. Estimates

- Compute: about 12 machine-hours (simulation sweep is CPU-parallel; emulated corroboration about 6 h in an exclusive window; local guard timing about 1 h).
- Disk: under 5 GB beyond the EXP-REMOTE-002 cache (simulated request logs, gzip).
- Agent effort: scripted; judgment for tier assessment request, W/S declaration and analysis.
