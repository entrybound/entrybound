# EXP-REMOTE-005: Session cache policy, lazy versus eager retrieval, and remote access to STREAM archives

| Field | Value |
|---|---|
| Status | **NEEDS_TOOLING** (T1, T2, T4 `CachePolicySource` and bulk/resume prototype, T3 mid-body reset fault, T5, T6 `balanced-stream` config) |
| Domain / program section | remote / §12 (§13 for STREAM) |
| decision_ids | DEC-ACC-045, DEC-ACC-042, DEC-ACC-059 |
| Decision type (provisional) | EMPIRICAL (all); DEC-ACC-059 also FORMAL for the complexity-visible rule |
| OD / HC (provisional until G-A) | OD-12 (M12.1-M12.3), OD-08 (M08.1 client memory), OD-07 guard (local verify/decode time for eager retrieval), OD-09 (STREAM capability); HC-06 (cache and staging bounds), HC-15 (lazy reads never claim whole-archive verification; eager full verify does), HC-18 (persistent cache under revision change), HC-09 |
| Author / L7 / gates | As EXP-REMOTE-001. Uses EXP-REMOTE-002 archives and H4 validation; the `parallel-index-driven-bulk` candidate reuses the EXP-REMOTE-006 executor prototype. Client memory decision-grade only with the counting allocator (`decision-method.md` §14 item 17); until then memory numbers are labelled scoped-RSS and informational. |

## 1. Question

Which session range-cache policy and size, and which retrieval strategy (lazy per-entry, eager full download, size- or fraction-threshold hybrids, index-driven parallel bulk fetch, background fill, hot-set prefetch), minimize time-to-first-byte and time-to-all-requested-bytes with bounded client memory across archive sizes, read fractions and network profiles; and what does each route to random access into STREAM archives (repack, explicit scan-and-cache, trailing index, sidecar index, full download) cost?

## 2. Hypotheses

- **H1 (DEC-ACC-045).** With whole-containment FIFO (status quo) the 128 MiB bound already yields near-zero refetch on WL-SUBTREE and WL-RANDK256, so LRU, 2Q/ARC and larger sizes are `EQUIVALENT` there; on WL-ZIPF (1,024 reads with replacement) `lru` or `adjacent-range-merge` is `DISTINGUISHABLE_BETTER` on `remote_task_latency_s` at NP-CONT/NP-INTER only for archives whose working set exceeds the bound; `persistent-verified-chunk-cache` dominates WL-REVISIT session 2.
- **H2 (DEC-ACC-042).** There is a read-fraction crossover f* per (profile, archive size): lazy wins time-to-all below f*, eager above; f* decreases with RTT; `size-threshold-hybrid` with a fixed threshold is not the R8 winner across all four profiles, whereas `workload-hint` (caller-declared fraction) is.
- **H3 (DEC-ACC-059).** For read fractions below 0.5, an explicit trailing or sidecar index gives remote WL-FIRST latency within 1 band unit of INDEXED, while remote sequential download and scan-and-cache cost the whole archive; local repack cost grows linearly in archive bytes.
- **H0 / falsification.** H1 falsified if a non-status-quo policy is `DISTINGUISHABLE_BETTER` on WL-SUBTREE, or none is on WL-ZIPF for oversize working sets; H2 falsified if one fixed size threshold is the R8 winner at all four profiles, or if no crossover exists (one strategy dominates for all f); H3 falsified if trailing-index remote WL-FIRST is more than 1 band unit worse than INDEXED.

## 3. Decisions informed

| Decision | Supplied here | Remaining |
|---|---|---|
| DEC-ACC-045 | Hit rate, refetch waste and memory vs cache size/policy on multi-entry read workloads (package-install-like WL-SUBTREE, dataset sampling WL-ZIPF, model shard loading = WL-SEQALL/WL-RANGE on F07 items), warm and cold; memory profile of long-lived encrypted sessions (the unbounded `chunk_frames` path named by the ledger) | Revision-change and hostile-server tests for the persistent cache: EXP-REMOTE-009 (churn and substitution cases); recorded real application traces (not synthetic) remain a limitation |
| DEC-ACC-042 | Time-to-first-byte and time-to-all-bytes for lazy, eager and hybrids across archive sizes (tuning corpus up to about 3.5 GiB measured; up to 100 GiB by the validated linear full-download model, labelled), read fractions 1-100%, RTT and bandwidth; resume-after-interruption with per-Chunk verification | Primary-source verification of SOCI Parallel Pull figures: analytical (§9.3 citation work, no experiment); whether verify/unpack/explain accept remote sources is API scope (FORMAL) informed by these costs |
| DEC-ACC-059 | Cost of repack vs scan-and-cache vs trailing index vs sidecar vs full download on STREAM archives, locally and remotely | Demand evidence for random access into STREAM artifacts: literature/adoption workflows (ecosystem §32, no experiment); sidecar separation risks: integrity §23 |

## 4. Candidates

### 4.1 Cache (DEC-ACC-045)

| candidate_id | Definition | Realization |
|---|---|---|
| `status-quo-128mib-fifo` (reference) | revision-scoped FIFO, 128 MiB, whole-containment hits only (`random_access.rs:606-644`) | production |
| `fifo-<s>`, s ∈ {0, 1, 8, 32, 512} MiB | status-quo policy at other bounds (s = 0 is `no-cache`) | production `max_cached_bytes` |
| `lru-<s>`, s ∈ {8, 32, 128} MiB | least-recently-used eviction, containment hits | production cache disabled (`max_cached_bytes = 0`) + `CachePolicySource` (T4) + `fetchsim` |
| `arc-or-2q-128m` | 2Q with A1in 25%, A1out ghost list of 50% of entries | as above |
| `adjacent-range-merge-128m` | LRU whose hits may span adjacent cached entries | as above |
| `size-scaled` | bound = min(512 MiB, max(8 MiB, 2 × declared metadata bytes + largest declared Chunk × max_dependency closure of the archive)) | as above |
| `persistent-verified-chunk-cache` | on-disk store of verified Chunk plaintext keyed by (chunk digest) and metadata keyed by (URL, strong ETag, length); bound 1 GiB; used across sessions | T4 persistent wrapper + `fetchsim` (WL-REVISIT) |
| `no-cache` | = `fifo-0` | production |

### 4.2 Retrieval strategy (DEC-ACC-042)

| candidate_id | Definition | Realization |
|---|---|---|
| `status-quo-lazy-only` (reference) | metadata-first per-entry reads | production |
| `eager-full-download-then-verify` | one GET (or 64 MiB consecutive ranges) of the archive, then local full verify and local reads | model over exact bytes + measured local `verify_wall_s`/`decode_wall_s` (production `ebound verify` on the cached local file, timing window) |
| `size-threshold-hybrid-<T>`, T ∈ {10 MiB, 64 MiB, 256 MiB} | eager if archive bytes ≤ T else lazy | composition of the two above |
| `workload-hint` | caller declares expected read fraction f̂; eager iff f̂ ≥ φ(profile, size) with φ fitted on tuning from the modelled crossover and frozen before validation | composition |
| `parallel-index-driven-bulk` | Index-driven fetch of all Chunk frames on 8 connections with per-Chunk verification and resume | EXP-REMOTE-006 executor prototype + model CM-H1-TLS-N |
| `background-eager-fill` | lazy reads for requested entries while a single background connection fills a verified cache in physical order | model with two concurrent connections sharing the link (CM-H1-TLS-N with N = 2) + prototype |
| `prefetch-hot-set` | lazy plus one coalesced prefetch of a declared hot set (EXP-REMOTE-003 `hot-set-prefix` definition, status-quo order) | `fetchsim` |

Resume arm: for `eager-full-download-then-verify` and `parallel-index-driven-bulk`, a connection reset is injected at 5 seeded byte offsets per transfer (T3 fault `reset-mid-body`); resume uses ranges from the last verified Chunk boundary (bulk) or the last received byte (full download, whose bytes are unverified until the end).

### 4.3 STREAM random access (DEC-ACC-059)

| candidate_id | Definition | Realization |
|---|---|---|
| `status-quo-repack-only` (reference) | random access refused (`RandomAccessNotIndexed`); repack STREAM to INDEXED locally | production `ebound` repack/convert (docs/repack-v1.md), timing window |
| `scan-and-cache-explicit` | explicit sequential verified scan that builds a session locator map, reported as Sequential | local: production `open_stream` full pass (measured); remote: whole-archive bytes then per-entry cost from the session map (`fetchsim`) |
| `trailing-index-in-stream` (format revision) | optional Index item after the body; remote open = tail fetch of footer + index, then per-entry ranges | `fetchsim` with index bytes computed as the INDEXED Index size for the same plan |
| `sidecar-index` | external index artifact bound by archive digest | `fetchsim`: one extra object (setup + HEAD + index fetch) |
| `remote-sequential-download` | full download then local STREAM verify | model + measured local verify |
| `silent-scan-fallback` | transparent scan when random access is requested | **Proposed L0 exclusion**: violates the SPEC §19.4 complexity-visible rule (ledger hard constraint of DEC-ACC-059); written counterexample and independent sign-off required; its cost equals `scan-and-cache-explicit` and is reported only as counterevidence |

## 5. Corpus selectors

- Tuning archives from EXP-REMOTE-002 (`balanced-plain` all tuning items; `balanced-enc-bucketed` small + medium for the long-lived encrypted session memory arm) plus a new `balanced-stream` config (`ebound pack <tree> <out.eb> --layout stream --profile balanced`) for all tuning items.
- Size ladder for DEC-ACC-042: all tuning items grouped by scale tier; beyond the largest tuning item (about 3.5 GiB logical, manifest `bytes`) the full-download cost is extrapolated with the latency model (linear in bytes, FORMALLY_DERIVED) and the local verify throughput measured on the large tier, labelled `extrapolated`; no 100 GiB object is materialized.
- F07 model-weight items and F16 image items serve the shard-loading and large-entry views; WL-SUBTREE items require ≥ 2 regular files.
- Validation look for W vs S per decision; held-out: section 15.

## 6. Environment and platform requirements

`wsl-ubuntu`. Deterministic arms (counts, bytes, modelled latency, hit rates) without timing; local `verify_wall_s`, `decode_wall_s`, repack `encode_wall_s` and `scratch_peak_bytes` with the production `ebound` in a quiet window (`st-v`, VIRTUALIZED qualifier, §4.3); emulated corroboration of W vs reference at NP-CONT and NP-INTER on E12. Linux x86-64; no privileges; disk for the persistent cache (≤ 1 GiB per item run, cleared between items).

**Platform matrix.** Linux x86-64 required (WSL2); Windows x86-64: eager-path local timing on Windows is out of scope and recorded as a limitation; macOS: not applicable; ARM64: not required; network: loopback emulation; privileged: no; large storage: ~45 GB; external review: no.

## 7. Commands

```sh
cd /mnt/d/Projects/entrybound/entrybound; PY=/root/eb-research/venv/bin/python; export PYTHONPATH=research/tools
python research/orchestration/disk_guard.py && python research/harness/lock_parity.py
$PY research/tools/remote/prepare_archives.py ensure --items research/experiments/EXP-REMOTE-002/items-tuning-all.txt --configs balanced-stream
$PY -m ebr run --spec research/experiments/EXP-REMOTE-005/spec.yaml                  # production cache sizes and wrappers
$PY -m ebr verify-raw --spec research/experiments/EXP-REMOTE-005/spec.yaml && $PY -m ebr normalize --spec research/experiments/EXP-REMOTE-005/spec.yaml
$PY -m ebr run --spec research/experiments/EXP-REMOTE-005/spec-local-timing.yaml --timing   # ebound verify / repack / open_stream on cached files
$PY research/tools/remote/fetchsim.py sweep --strategies research/experiments/EXP-REMOTE-005/strategies.json \
    --workloads research/experiments/EXP-REMOTE-002/workloads.json --connection-model research/experiments/EXP-REMOTE-001/connection-model.json \
    --local-timing research/normalized/EXP-REMOTE-005 --out research/raw/EXP-REMOTE-005/sim/
$PY -m decide analyze --experiment EXP-REMOTE-005
```

`spec-local-timing.yaml` is generated from `spec.yaml` by `make_variant_specs.py` (commands `ebound verify <file>`, `ebound repack ...`, `ebr-remote stream-scan`); only `spec.yaml` is committed at design time.

## 8. Seeds, warmups, repetitions

Seeds `order` 20260929, `bootstrap` 20260930, `command` 20261001. Deterministic arms: 2 repetitions + repeat-hash (encrypted: 3 key replicates). Local timing: warmups 2 (large tier 1), rounds min 10, step per tier, cap per tier (§4.6), precision rule. Resume injections: 5 seeded offsets per (item, candidate), seed from `command`.

## 9. Metrics

| Metric | OD | Role | Threshold |
|---|---|---|---|
| `remote_task_latency_s` (time to all requested bytes) per (profile, workload) — modelled | OD-12 | **primary** | T-10 |
| `remote_first_entry_latency_s` (time to first verified byte) | OD-12 | **second primary for DEC-ACC-042 only, justification pending reviewer (R3):** the decision's question is the trade-off between first-byte and all-bytes times, which one metric cannot represent | T-10 |
| `alloc_peak_bytes` (client, per session; scoped RSS until §14 item 17) | OD-08 | **primary** for DEC-ACC-045 | T-06 |
| `http_bytes`, `http_requests`, `refetch_bytes` (bytes fetched more than once in a session), `cache_hit_fraction` | OD-12 | secondary / diagnostic | T-11, T-12 |
| `verify_wall_s`, `decode_wall_s` (local, eager path) | OD-07 | guard inputs to the modelled eager latency | T-04 |
| `encode_wall_s`, `scratch_peak_bytes` (repack) | OD-06, OD-08 | secondary (DEC-ACC-059) | T-03, T-07 |
| `streaming_capability` | OD-09 | binary for STREAM candidates that claim pipe-compatible creation is unchanged | §3.3 |
| `roundtrip_ok`, `report_mismatches`, `resume_output_ok` | – | binary | §3.3 |
| `bounded_memory_growth_bytes` for the long-lived encrypted session (reads 1 → 1,024 on WL-ZIPF) | OD-08 | binary claim check if bounded behaviour is claimed | §3.3 |

## 10. Normalization and statistical analysis

- Item = archive; L2-L4 with `workload-mix.json` and equal weights arm; strata per tier, profile, workload and read fraction; conditions not averaged.
- Crossover analysis (reported): per (item, profile) the smallest f in {0.01, 0.1, 0.5, 1.0} at which eager's modelled time-to-all is not worse than lazy's, with linear interpolation in log f reported as descriptive.
- Tuning W/S per decision by continuous band-unit utility over the declared primaries; validation W vs S confirmatory comparisons (§4.11-§4.12) with MDE gate.
- R9 partitions: for DEC-ACC-042, workload-declared fraction (expressible only through `workload-hint`, a T1/T2 API addition), then archive size (expressible), then profile (not expressible without RTT measurement; recorded as limitation); for DEC-ACC-045, local vs remote source.
- Tiers (independent assessment): cache policies T0 (client-internal) unless exposed as user-visible options (T2); persistent cache adds on-disk state and hostile-input surface (C4) with tier assessed; `trailing-index-in-stream` is a format revision (≥ T3); `sidecar-index` adds an artifact kind (≥ T2).

## 11. Practical significance

T-10, T-06, T-11, T-12, T-04, T-03, T-07 by reference; binaries §3.3; minimum-gain bands per computed tier.

## 12. Sensitivity analysis

Slow-start and connection-model arms with the robustness rule (eager full download is the case most exposed to slow start at NP-INTER); SS-IDLE-RESET with interactive gaps for WL-ZIPF and WL-RANDK256; Zipf exponent 0.8 and 1.2; hot-set definition variant (first 256 files by path); resume offsets at 1 and 20 per transfer; §6.3-§6.7 arms.

## 13. Expected negative results worth recording

- Cache policy differences vanish for most corpus archives (their metadata plus working sets fit 128 MiB).
- Eager full download loses badly on NP-INTER for large archives even at high read fractions if local verify is slow; lazy loses at f = 1.0 for many-small-file archives because of per-entry requests.
- Full-download resume without per-Chunk verification refetches or trusts unverified bytes; only the verified bulk strategy resumes safely.
- `trailing-index-in-stream` needs a format revision whose permanent cost (§7.2) may exceed its gain for the demand level, which this experiment cannot measure.

## 14. Threats to validity

- Synthetic workloads stand in for recorded package-install, dataset-sampling and shard-loading traces.
- Extrapolation beyond the corpus size ladder is model-only.
- Background fill and parallel strategies depend on the fluid link-sharing model until corroborated by EXP-REMOTE-006's emulated runs.
- Client memory is scoped RSS until the counting allocator exists (not decision-grade).
- Local timing is WSL `VIRTUALIZED`; eager-path claims for Windows need a Windows-host local arm (not in scope; recorded limitation).

## 15. Held-out (Phase D placeholder)

Frozen W/S rerun once on held-out archives (deterministic arms, local timing arm, emulated corroboration of W vs reference) after Commit C per `decision-method.md` §5.5, report-only per §5.6.

## 16. Estimates

- Compute: about 16 machine-hours (STREAM packing of all tuning items ~4 h; local verify/repack timing ~6 h in exclusive windows; simulation and prototypes ~6 h).
- Disk: about 45 GB peak (STREAM archives, persistent cache scratch, repack outputs; cleared per item).
- Agent effort: scripted; judgment for tier assessments, W/S and analysis.
