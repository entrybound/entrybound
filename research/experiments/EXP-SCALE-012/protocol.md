# EXP-SCALE-012: Deterministic parallel pipeline prototypes (hashing, chunk analysis, candidate evaluation, compression, verification) with byte identity across worker counts and schedules

| Field | Value |
|---|---|
| Status | **NEEDS_TOOLING** |
| Domain | scale (program §14) |
| Platforms | Linux x86-64 (WSL2); Windows 11 host (second host class); emulated arm64 repeat in EXP-SCALE-014 |
| Requires timing | false (throughput is descriptive; decision-grade speedups are EXP-SCALE-013) |
| Compute estimate | about 35 machine-hours |
| Disk estimate | about 30 GB |
| Agent effort | judgment (prototype construction, ordering proofs for intra-file CDC, loom models); scripted execution |
| Tooling to build | research-only harness crate `research/harness/crates/ebr-par` (binary `ebr-par`) over `entrybound` public API and default-off `research-internals` wrappers (chunker, hashing, planner candidate evaluation, frame encode/decode); pinned `crossbeam-channel` and `rayon` (harness only); pinned `loom` for model-checking tests of the reorder buffer (`cargo test -p ebr-par --features loom`); `--chaos-yield <seed>` schedule perturbation (seeded `std::thread::yield_now`/sleep injection at queue operations); `research/tools/scale/bgload.py` (EXP-SCALE-005); fixed-N HTTP/1.1 range-fetch prototype against local `ebr-origin` for report determinism; `cargo tree` dependency census script for C3 of each concurrency-model shell (§7.1, §14 item 7) |

## Pre-registration

- **decision_ids:** DEC-ACC-030, DEC-ACC-031, DEC-ACC-032, DEC-ACC-033, DEC-ACC-007, DEC-ACC-043, DEC-MOD-013, DEC-CMP-001, DEC-CMP-041.
- **Decision type:** EMPIRICAL (identity, memory bound, report determinism) with FORMAL parts (intra-file CDC boundary-equivalence argument, reorder-buffer memory bound formula), each needing a reviewer who did not build the prototype.
- **od_ids / hc_ids:** PENDING-G-A. Proposal: OD-13 (floor only here), OD-08 (queue memory), OD-22/OD-24 cost inputs for concurrency models; HC-11, HC-13, HC-06 (declared queue bounds), HC-15 (deterministic reports and error selection).
- **Method, gate, author, L7:** as EXP-SCALE-001; critic candidate and non-exposed sign-off before G-A.

## Question

Which parallel designs for hashing, content-defined chunk analysis, similarity sketching, codec candidate evaluation, final compression, verification and decode, remote range fetching, and pipeline queueing produce archives, identities, reports and error selections byte-identical to the sequential status quo for every worker count (1, 2, 4, 8, 16, 32), under perturbed schedules and background load, with queue memory within a declared bound; and what does each concurrency model (internal thread pools, async runtime, sans-io core with sync shell, caller-provided executor) add in dependencies?

## Hypotheses

- **H1a:** `parallel-chunk-hashing`, `per-file-parallel-cdc`, `parallel-candidates-single-thread-codecs`, `parallel-final-encode-only`, `parallel-independent-chunks` (verify/decode) and `stream-read-ahead-pipeline` with ordinal-keyed results and canonical emission produce PCI, LAI, PCR, AUX and verify reports identical to the sequential reference for every worker count, schedule seed and load condition.
- **H1b:** `intra-file-parallel-cdc` with overlap resynchronization reproduces sequential gear-norm-v1 boundaries exactly on every tuning file including `f20-tuning-gear-extremes`, given an overlap of at least the maximum chunk size (FORMAL argument: a boundary depends only on the hash state since the previous cut, which resets per chunk, so any segment start at a known cut resynchronizes).
- **H1c:** `bounded-mpmc-with-reorder-buffer` peak queued plaintext stays within the declared bound `workers × max_chunk_bytes + reorder_capacity × max_chunk_bytes` under adversarial size skew; `rayon-style-scoped` collect-in-order has no such bound (memory grows with the number of pending results).
- **H1d:** with multiple injected corruptions, parallel verify reports the same first error (canonical order) as sequential verify.
- **H1e:** fixed-N concurrent range fetching yields identical `bytes`, `requests` and verified outcomes, but non-identical request trace order unless the report canonicalizes it.
- **H0 / falsification:** one differing output, report, first-error selection or bound exceedance (binary, committed artifact).

## Decisions informed

| Decision | Addressed here | Routed elsewhere |
|---|---|---|
| DEC-ACC-030 | Regret unchanged under parallel candidate evaluation (plan identity); cross-platform byte identity under parallel candidate evaluation | codec thread invariance: EXP-SCALE-011; planner wall vs workers: EXP-SCALE-013 |
| DEC-ACC-031 | Byte-identical archives and identities across worker counts (and, in EXP-SCALE-014, emulated arm64); boundary equivalence test for intra-file parallel CDC | speedup curves on P/E cores: EXP-SCALE-013 |
| DEC-ACC-032 | Memory bound of reorder buffers under skewed work sizes; byte identity across worker counts and schedulers with loom model checking | throughput vs queue depth: EXP-SCALE-013 |
| DEC-ACC-033 | Deterministic reports (counts, error selection with multiple corruptions) under parallel verify; queue memory overhead | speedup vs workers and lookback depth, HDD/network storage: EXP-SCALE-013, EXP-SCALE-016 |
| DEC-ACC-007 | Determinism verification under each concurrency model; dependency cost (C3 census) per model; Send/Sync audit of public types via compile-time tests | API ergonomics judgment: human-facing (not measurable); throughput: EXP-SCALE-013 |
| DEC-ACC-043 | Proof by test that report fields (bytes, requests, trace order) remain deterministic, or are labelled non-deterministic, under fixed-N connections | latency/throughput under network profiles, CDN limits, HTTP/2/3, binary size: remote domain |
| DEC-MOD-013 | Research-harness prototype of parallel pack/verify: byte and LAI/PCR/AUX comparison across worker counts 1..32 | cross-machine: EXP-SCALE-014/015 |
| DEC-CMP-001 | Boundary identity of gear-norm-v1 under per-file and intra-file parallel chunk analysis (the parallelizability property of the frozen chunker) | algorithm selection and SIMD/ISA identity: chunking domain and EXP-SCALE-014 |
| DEC-CMP-041 | Determinism of similarity sketching and cohort formation computed in parallel | method choice: crossfile domain |

## Candidates

| Stage | candidate | Definition |
|---|---|---|
| Reference | `sequential-status-quo` | production pack/verify (identity reference) |
| Hashing | `parallel-chunk-hashing` | SHA-256 of chunks in a worker pool, results keyed by ordinal |
| | `tree-hash-parallel` | parallel Merkle subtree hashing for roots |
| | `simd-hash-single-thread` | accelerated single-thread SHA-256 (the `sha2` crate's CPU-feature dispatch; identity must hold) |
| Chunk analysis | `per-file-parallel-cdc` | CDC per file in parallel, canonical merge by path order |
| | `intra-file-parallel-cdc` | large files split into segments with overlap resynchronization |
| Similarity | `parallel-sketches` | bottom-k shingle sketches computed in parallel, cohorts formed sequentially in digest order |
| Planning | `parallel-candidates-single-thread-codecs` | per-chunk/cohort candidate evaluation in a pool; codecs single-threaded |
| | `codec-internal-mt-pinned` | zstd workers with fixed job size (only if EXP-SCALE-011 shows worker-count invariance) |
| | `parallel-final-encode-only` | sequential selection, parallel final encoding |
| | `speculative-pruning` | deterministic pruning instead of parallelism (reference for work reduction) |
| Verify/decode | `parallel-independent-chunks` | lookback-0 chunks decoded in a pool; groups serial |
| | `stream-read-ahead-pipeline` | reader thread plus decode pool with bounded queue |
| | `parallel-entry-extraction` | parallel per-entry materialization after verification |
| | `parallel-closure-decode` | parallel decode of random-read dependency closures |
| Pipeline structure | `bounded-mpmc-with-reorder-buffer`, `fork-join-per-object`, `rayon-style-scoped`, `actor-stages` | as named in DEC-ACC-032, each applied to the pack pipeline |
| Concurrency model shells | `internal-thread-pools`, `async-runtime` (pinned tokio), `sans-io-core-sync-async-shells`, `caller-provided-executor` | the `parallel-independent-chunks` verify implemented behind each API shape |
| Remote | `fixed-n-connections-<n>` for n in {1, 4, 8, 16} | concurrent HTTP/1.1 range fetch of a random-read closure against local `ebr-origin` (no emulated latency) |

- Worker counts: {1, 2, 4, 8, 16, 32} for every parallel candidate (1 must equal the reference exactly).
- Schedule perturbation: `--chaos-yield` seeds {0 (off), 3 seeded values}; load: none or `bgload.py --procs 31`.
- **Excluded:** `nondeterministic-fast-mode` from identity claims (it is allowed to differ by definition; recorded only for LAI stability, MVT scope); `http2-multiplexing`, `http3-quic`, `adaptive-aimd` (remote domain); `io-probe-adaptive` (EXP-SCALE-013/016).

## Corpus selectors

Manifest `research/corpus/manifest.json`, split `tuning`: small and medium items of every family (the same selection rule as EXP-SCALE-005, tuning only), including `f20-tuning-gear-extremes` and `f20-tuning-bombs-dense`; skew generator inputs (seed 2201: alternating 8 MiB incompressible and 64 B files) for reorder-buffer bounds; generated STREAM archives with 1, 3 and 10 injected corruptions (seeds 2211-2213) for error selection. One registered validation look (validation small and medium items) confirms identity for candidates proposed as provisional selections. **Held-out placeholder (Phase D):** identity re-check on held-out small and medium items after unlock (§5.5), R5 (a).

## Environment

`env_id` `wsl-ubuntu` and `windows-host`; no timing; `taskset -c 0-1` oversubscription cell (32 workers on 2 CPUs) as an additional schedule perturbation; loom tests run in CI-like `cargo test` on WSL.

## Commands

```sh
wsl.exe -d Ubuntu -- bash research/harness/build.sh build --release -p ebr-par
wsl.exe -d Ubuntu -- bash research/harness/build.sh test -p ebr-par --features loom --release
PYTHONPATH=research/tools /root/eb-research/venv/bin/python -m ebr run --spec research/experiments/EXP-SCALE-012/spec.yaml
C:/Python313/python.exe -m ebr run --spec research/experiments/EXP-SCALE-012/spec-windows.yaml   # generated from spec.yaml with Windows paths before running
```

Binary contract: `ebr-par --design <candidate> --workers <n> --chaos-yield <seed> --item-path <dir> --op pack|verify|fetch --profile <p> --seed <n> --scratch <dir> --experiment-id EXP-SCALE-012 --env-name <env>`; the binary runs the sequential reference in-process first (cached per item and profile) and reports equality fields.

## Seeds

`seeds.order` 1312001, `seeds.bootstrap` 1312002, `seeds.command` 1312003.

## Warmup

0.

## Measurement method and metrics

| Metric | Canonical name | Role | Family |
|---|---|---|---|
| `pci_equal_ref`, `lai_equal_ref`, `pcr_equal_ref`, `aux_equal_ref`, `plan_equal_ref`, `boundaries_equal_ref` | HC-11/HC-13 identity | binary | correctness |
| `report_equal_ref`, `first_error_equal_ref` | HC-15 determinism of reports | binary | correctness |
| `queue_peak_bytes`, `declared_queue_bound_bytes`, `queue_bound_ok` | HC-06 declared bound; T-06 for graded comparison | binary; graded diagnostic | correctness; memory_peak |
| `alloc_peak_bytes` | T-06 | diagnostic | memory_peak |
| `loom_models_passed` | none | binary | correctness |
| `decode_path_dependency_count`, `decode_path_native_unsafe_count` | cost inputs (C3) | cost_input | other |
| `op_wall_s` | none (descriptive) | descriptive | time_wall |
| `fetch_bytes_equal`, `fetch_requests_equal`, `trace_order_equal` | T-11/T-12 values compared for equality | binary | correctness |

## Replication

- **Design matrix (pre-registered, to keep the factorial tractable):** (a) small tuning items (about 30): every applicable candidate × workers {1, 2, 4, 8, 16, 32} × schedule seeds {off, 3 seeded} × load {none} × profile balanced, plus profiles fast, dense, extreme at workers {1, 8, 32} with schedule off; (b) medium tuning items (about 45): workers {1, 32} × schedule {off, one seeded} × balanced and dense; (c) stress: workers 32 with `bgload.py --procs 31` and the oversubscription cell (`taskset -c 0-1`), 20 repetitions, on the 10 smallest items of distinct families; (d) error selection and queue-bound inputs: all workers × schedule {off, 3 seeded}.
- 3 repetitions per cell of (a), (b) and (d); 20 in (c) (Appendix D.5 detection bound).

## Normalization and statistical analysis

Binary identity per group; violation artifacts committed. Queue bound: exact comparison against the declared formula (no band). Dependency counts: exact per candidate, feeding §7.2 tier computation. Descriptive throughput only.

## Practical-significance thresholds

None for binary oracles; T-06 referenced for any later graded queue-memory comparison; cost tiers per §7.2.

## Sensitivity analysis

Profile arm (fast, balanced, dense, extreme); item-scale arm; oversubscription arm; host arm (WSL vs Windows).

## Expected negative results worth recording

Similarity sketches or cohort formation order-sensitive under parallelism; intra-file CDC resynchronization failing on pathological inputs when the overlap is below the maximum chunk size; `rayon-style-scoped` unbounded memory; async shells adding dozens of transitive dependencies; request trace order non-deterministic under N connections.

## Threats to validity

1. Prototypes built on research-internals wrappers (review R1-06 codegen caveat; identity results are unaffected, timing is not claimed).
2. Schedule perturbation explores a finite set of interleavings; loom exhausts only small models; stress repetitions are regression evidence with the D.5 bound.
3. Local `ebr-origin` has no latency; ordering effects that appear only under latency are the remote domain's.

## Platform requirements

Linux x86-64 WSL2 (32 vCPUs), Windows host; about 30 GB; harness dependencies pinned (downloads approved).

## Estimated compute and effort

About 35 h after tooling under the design matrix above (small items dominate the cell count; medium items dominate the time). Tooling judgment.

## Deviations (append-only)

None.
