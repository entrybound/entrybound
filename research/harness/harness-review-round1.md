# Research harness review, round 1

| Item | Value |
|---|---|
| Date | 2026-09-17 |
| Role | Phase B2 adversarial reviewer (measurement validity, before any decision use) |
| Reviewed | `research/harness/**` at `68dfa5d`, including all seven `ebr-*` crates and their READMEs, `research-internals.md`, `internals-identity-check.{sh,ps1,md}`, `internals_identity_tools.py`, and the `research-internals` diff in `crates/entrybound` (commit `c9a2d3a`, `git show c9a2d3a -- crates/`). The `ebr` runner's POSIX resource measurement (`research/tools/ebr/execute.py`) was checked where harness binaries rely on it. |
| Method | Source reading of every measured code path against the production code it calls, empirical probes (lock-file closure diff, `wait4`/`getrusage` inheritance, Windows metadata staleness, bandwidth smoke), regression tests that fail on the old behavior, and an end-to-end smoke of every binary. |
| Verdict | **Not decision-ready at `68dfa5d`.** Eight HIGH and six MEDIUM harness defects are fixed in this round's commit; one MEDIUM runner defect (outside the Rust harness) and one MEDIUM emulator limitation remain open with use constraints (below). The harness is usable for C-exec **only under the "Use constraints" section**. |
| Timing | Every number in this document is non-decision-grade smoke output (other agents were running; no quiet-machine guard). |

Severity: **HIGH** = a reported number is wrong or not what its name says, or
a safety guard is absent, so a decision could be corrupted. **MEDIUM** = a
comparison is confounded, unfair, or unverifiable, or a measurement is wrong
in a subset of cases. **LOW** = a documented simplification, a narrow edge,
or a documentation error with no effect on current numbers.

## Summary

| ID | Sev. | Area | Finding | Disposition |
|---|---|---|---|---|
| R1-01 | HIGH | (h) held-out guard | No harness binary called the held-out guard; the guard itself could be defeated by an `EB_DATA_ROOT` override and never matched `\\wsl.localhost\...` paths | Fixed |
| R1-02 | HIGH | (b) byte accounting | Random-access per-read `bytes_fetched`/`range_request_count` were session-cumulative counters, reported per read and summed | Fixed |
| R1-03 | HIGH | (f) timing/RSS | `ebr-pack` read the whole input into memory before packing, called that "capture", kept it resident during the production pack, and measured a process-lifetime peak after verification and accounting | Fixed |
| R1-04 | HIGH | (f) peak scratch | `ebr-unpack` peak scratch ignored STREAM's staging spill files (production writes them to the OS temp directory); the timed path included an extra full open/verify | Fixed |
| R1-05 | HIGH | (f) memory probe | Probe stage peaks were a process-lifetime `max_rss` with the planned `Archive` kept alive, so every stage reported the pack peak; children spawned for isolation also inherit the parent's `max_rss` | Fixed |
| R1-06 | HIGH | (a) build identity | The harness `Cargo.lock` resolved 28 of entrybound's transitive dependencies differently from production (`zstd-sys`, `zstd-safe`, `cc`, ...); harness builds also enable `research-internals`, whose identity proof covers bytes, not codegen | Fixed (lock), constraint (codegen) |
| R1-07 | HIGH | (e) planner | `ebr-planner` planned each item as one whole-file Chunk; regret rows mixed cost bases; reconstructive selections gave silent negative regret; per-chunk regret charged plan records per chunk; the "strict superset" claim was untested | Fixed |
| R1-08 | MEDIUM | (b) byte accounting | Named byte buckets did not sum to the file size (ChunkData section headers and unknown kinds unattributed); the only balance check was near-tautological; no independent cross-check existed in code | Fixed |
| R1-09 | MEDIUM | (c) CDC | Rabin/Buzhash/TTTD power-of-two masks realize about 1.2x the target mean chunk size at production presets, while FastCDC and `gear-norm-v1` realize about 1x | Fixed |
| R1-10 | MEDIUM | (c) CDC | TTTD used the first backup breakpoint; the paper uses the last | Fixed |
| R1-11 | MEDIUM | (c) CDC | RAM carried a wrong citation and implemented a different (sliding-window) rule; AE lacked the authors' 8-byte-value variant and its calibration | Fixed |
| R1-12 | HIGH | (d) codecs | Codec candidates reported size/time only: no window, decoder memory, threads, or container, so gains could not be separated from resources; PPMd ran at a 1 MiB strawman configuration; LDM had no window-matched control | Fixed |
| R1-13 | MEDIUM | (g) netem | Loss was rolled per `--chunk-size` chunk (loss rate coupled to a pacing knob); the token bucket discarded timer-oversleep credit, undershooting configured bandwidth (62%/11% at 100/1000 Mbit/s) | Fixed |
| R1-14 | MEDIUM | (g) netem | No TCP slow start: emulated results favor few large range requests on fresh connections | Open: use constraint |
| R1-15 | MEDIUM | (f) peak scratch | On Windows, `DirEntry::metadata` returns a stale (often zero) size for files still being written, so the scratch sampler undercounted in-progress extraction and spill files | Fixed |
| R1-16 | MEDIUM | (f) runner RSS | The `ebr` runner's raw `measurement.rusage.ru_maxrss_kib` (and its no-GNU-time fallback for `resource.max_rss_bytes`) includes the Python runner's own high-water mark | Open: deferred to runner owner |
| L-01..L-14 | LOW | various | See "LOW findings" | Fixed, documented, or accepted |

## Verdicts on the review questions

**(a) The feature cannot change default builds — PASS, with one HIGH build
finding (R1-06).** `research-internals = []` is declared in
`crates/entrybound/Cargo.toml` and is not a default feature. Every production
source change in `c9a2d3a` is one `#[cfg(feature = "research-internals")] pub
mod research { ... }` block per file plus one gated re-export module in
`lib.rs`; `git show c9a2d3a -- crates/` has no removed lines, and
`internals_identity_tools.py additive-check` strips exactly those blocks and
requires byte identity with the base commit for every audited file and no
other changed `crates/` path (reviewed: the stripper cannot silently remove a
production line, because any removed doc comment or body would make the
remainder differ from the base). No production function body, visibility
modifier, or item order changed. `tests/research_internals.rs` is gated at
crate level. Neither `entrybound-cli` nor any dev-dependency in the production
workspace enables the feature, and the harness is a separate Cargo workspace,
so its feature requests cannot unify into production builds. Pitfalls found:
`cargo build --all-features` in the production workspace would enable the
feature (L-01); `ebr-netem`'s dev-dependency comment claimed a feature-free
build that workspace-wide feature unification does not give (L-02, fixed);
and the harness's own lock file and feature set make harness timings a
different build from the shipped CLI (R1-06).

**(b) Byte accounting is exact and independently cross-checked — FAIL at
`68dfa5d`, PASS after fixes (R1-02, R1-08).**

**(c) Alternative CDC implementations match their papers and are not
strawmen — PARTIAL at `68dfa5d`, PASS after fixes (R1-09..R1-11).** FastCDC
2016/2020 were already faithful: verbatim Gear and mask tables, `cut()` ports,
and tests against the pinned `fastcdc` crate.

**(d) External codecs are configured fairly and their memory/decoder
complexity is measurable — FAIL at `68dfa5d`, PASS after fixes (R1-12),** with
the in-process memory caveat in "Use constraints".

**(e) Exhaustive planner search is exhaustive within stated caps and regret
uses complete cost — FAIL at `68dfa5d`, PASS for the Chunk stage after fixes
(R1-07).** Cohort-stage search remains library-only (L-10).

**(f) Timing/RSS is not dominated by harness overhead, and peak scratch is
real — FAIL at `68dfa5d`, PASS after fixes (R1-03..R1-05, R1-15)** in the
harness; the runner-side R1-16 remains open.

**(g) Network emulator semantics vs real TCP — PARTIAL.** Two artifacts fixed
(R1-13); slow start is not modeled (R1-14, constraint); the documented
simplifications remain (L-11, L-12).

**(h) The held-out guard cannot be bypassed accidentally — FAIL at `68dfa5d`,
PASS after fixes (R1-01).** Residual: copies and hard links cannot be detected
by any path check (L-13).

## HIGH and MEDIUM findings

### R1-01 (HIGH) — held-out guard never invoked; guard bypassable

- **Evidence.** `grep -rn heldout research/harness/crates --include=*.rs` found
  the guard only inside `ebr-common`; no binary called it on `--item-path`,
  `--archive-path`, `--versions`, or `ebr-origin --root`. The guard derived a
  single root from `EB_DATA_ROOT`, so setting `EB_DATA_ROOT` for any other
  reason stopped protecting `/root/eb-research/heldout`; a Windows-side binary
  reaching the distro through `\\wsl.localhost\Ubuntu\root\eb-research\heldout`
  was never matched; and it ignored the runner's `EB_HELDOUT_ROOT` override.
  Protection existed only at the `ebr` runner's spec level, so any manual or
  scripted harness invocation could read held-out content.
- **Fix.** `ebr_common::heldout::assert_inputs_not_heldout` is now called by
  every binary (`ebr-pack`, `ebr-unpack`, `ebr-verify`, `ebr-inspect-bytes`,
  `ebr-access` roundtrip, `ebr-chunk` including `--versions`, `ebr-codec`,
  `ebr-planner`, `ebr-origin --root`) before any read. The guard always
  protects the default root, adds `EB_DATA_ROOT/heldout` and `EB_HELDOUT_ROOT`,
  refuses WSL UNC paths (including `\\?\UNC\` forms) into the default root,
  refuses paths that lexically land under a root (`corpus/../heldout/...`)
  before touching the filesystem, and resolves `..` in a missing path tail
  lexically (previously over-refused).
- **Verification.** New unit tests: default root still guarded when
  `EB_DATA_ROOT` points elsewhere; UNC forms refused and near-misses allowed;
  symlink into the root refused; `..` paths; `assert_inputs_not_heldout`.
  `review-round1-smoke.sh` checks that all nine binaries exit with
  "refusing to read held-out path" for a nonexistent child of the held-out
  root, including with `EB_DATA_ROOT` overridden and via a `../` path. No
  held-out content was read: the refusal fires lexically before any `stat`.

### R1-02 (HIGH) — cumulative random-access counters reported as per-read costs

- **Evidence.** `crates/entrybound/src/ecf/random.rs` builds every
  `RandomAccessVerificationReport` with `bytes_fetched:
  self.session.bytes_fetched()` and `range_request_count:
  self.session.range_requests()`, the session totals. `ebr-access` copied
  them into each `RandomReadMeasurement::bytes_fetched_from_source` and summed
  them into `eager_total_bytes_fetched`, so every read re-counted the metadata
  open and all earlier reads (quadratic overcount), and a warm revisit showed
  the full session total instead of zero. The pre-existing unit test
  `measures_first_entry_sample_and_eager_reads` asserted
  `eager_total_bytes_fetched > 0` for an eager pass that is in fact entirely
  warm; it passed only because of the defect.
- **Fix.** Per-read deltas against the session counters before the read;
  new `metadata_open_bytes_fetched`, `metadata_open_range_requests`,
  `session_total_bytes_fetched`, `session_total_range_requests`, and
  `session_bytes_fetched_after`. The cumulative raw values stay in
  `VerificationFlags`, documented as cumulative.
- **Verification.** `per_read_bytes_are_deltas_that_sum_to_the_session_total`
  (open + every read's delta equals the session total; a fully warm eager pass
  fetches 0 bytes in 0 requests); the old test corrected; smoke check on a
  real tree.

### R1-03 (HIGH) — `ebr-pack` timing and memory dominated by harness work

- **Evidence.** `pack_once` called `read_tree_in_memory` (up to 8 GiB) and
  reported it as `capture_seconds`; that copy stayed resident while
  `plan_directory` (which captures the tree itself) ran, and the read warmed
  the page cache for it. A second, per-file chunker pass (different from
  production's parameter selection) ran before planning. Peak memory was one
  `getrusage` reading at the very end, after `open`, `inspect`, byte accounting
  (which copies the archive), and any `--deterministic-check` repeats, so it
  measured the harness's maximum, not the pack.
- **Fix.** The measured region is now exactly `plan_directory` + `encode`/
  `encode_stream` (or `pack_directory_encrypted`), wrapped in a new
  `ebr_common::measure::MemoryScope`: on Linux it resets the kernel high-water
  mark by writing `5` to `/proc/self/clear_refs` and reads `VmHWM` afterwards
  (safe Rust, no FFI). Input statistics come from a metadata-only walk after
  the pack; the chunker pass is opt-in (`--chunking-pass true`) and runs
  afterwards; `STREAM` verification no longer clones the archive. Rows report
  `pack_memory` and keep `process_lifetime_peak_rss_bytes` as a labeled
  diagnostic, plus a `build_note` (R1-06). `--stream-window auto` was added,
  matching `ebound pack` (L-05).
- **Verification.** `scoped_peak_excludes_an_earlier_larger_allocation`
  (a scope after a freed 256 MiB allocation reports far below the lifetime
  peak and still sees an 8 MiB allocation); ebr-pack tests; smoke asserts
  `pack_memory.scoped == true`.

### R1-04 (HIGH) — STREAM peak scratch missed production's staging spill

- **Evidence.** `unpack_stream` stages decoded plaintext in `ChunkStaging`,
  which spills beyond `bootstrap_staging_limits().memory_bytes` (256 MiB) to
  `std::env::temp_dir()/entrybound-stage-<pid>-*.tmp`
  (`crates/entrybound/src/ecf/staging.rs`). The sampler walked only the
  destination directory, so for any STREAM item with more than 256 MiB of
  staged plaintext the largest scratch consumer was invisible. The measured
  path also opened and verified the archive in memory first (for STREAM,
  on a cloned buffer) before calling the production entry point, which
  verifies again.
- **Fix.** The sampler sums the destination and this process's spill files
  per sample (`scratch.peak_total_bytes`), reports allocated bytes on Unix,
  backs off to at most ~20% of wall time on large trees
  (`sampler_walk_seconds`), and `ebr-unpack` reports production's own
  `StreamReport` staging counters next to it. `unpack_seconds` and
  `unpack_memory` cover only the production entry point (archive read
  included); verification is a separate pass afterwards
  (`verify_pass_seconds`, not additive).
- **Verification.** `stream_unpack_counts_the_staging_spill` forces a spill
  (4 KiB in-memory staging, 24 MiB input) and requires the sampler to observe
  it; `sampler_counts_this_process_staging_spill_files_and_ignores_others`.
  The Windows run of that test exposed R1-15.

### R1-05 (HIGH) — probe stage memory contaminated; isolated children inherit the parent peak

- **Evidence.** Samples were `getrusage(RUSAGE_SELF).max_rss`, monotone over
  the process lifetime, and `run_probe` kept the planned `Archive` (every
  Chunk's plaintext) alive through `verify` and `unpack`. Every stage after
  `pack` therefore reported at least `pack`'s peak, so the probe could never
  show bounded streaming memory, which is its purpose. While fixing this, a
  probe showed that a Linux child started with `posix_spawn`/`vfork` or
  `fork`+`exec` reports its parent's high-water mark as its own `max_rss`
  (Python parent with 512 MiB ballast: `wait4` of `/bin/true` reported
  535,696 KiB; `D:/eb-research/vfork_rss_probe.py`).
- **Fix.** Each stage opens a `MemoryScope` and samples current `VmRSS`;
  `repack`/`export` run right after `pack`; the archive is dropped before
  `verify`/`unpack`; and `--probe-isolate true` (the binary's default) runs
  `verify` and `unpack` in fresh child processes (`--mode probe-stage`) whose
  peak is taken from their own `VmHWM`, not `max_rss`. `measure_stream` and
  `measure_random_access` report per-pass scoped memory.
- **Verification.** `verify_stage_memory_does_not_inherit_the_pack_peak`
  (64 MiB input); smoke on a 32 MiB probe: `pack` delta 152.9 MB, isolated
  `verify` 51.8 MB, isolated `unpack` 51.9 MB (before the `VmHWM` change the
  children's `max_rss` reported the parent's 155.1 MB).

### R1-06 (HIGH) — harness builds are not the shipped dependency build

- **Evidence.** `lock_parity.py` (new) walks entrybound's normal-dependency
  closure in both lock files: 28 crates differed, including `zstd-sys`
  2.0.16 vs 2.1.0 and `zstd-safe` 7.2.4 vs 7.3.0 (the zstd binding and its
  build script inputs), `cc` 1.4.4 vs 1.4.6 (compiles libzstd), `smallvec`,
  `tinyvec`, `hybrid-array`, `rustls`, `bitflags`, and `syn`. The
  research-internals identity proof ran only in the production workspace, so
  nothing showed that harness "production" measurements used the same build.
  Separately, the harness always enables `research-internals`; the proof
  covers archive bytes, not inlining or code generation.
- **Fix.** The harness lock was realigned (`cargo update --precise` for 21
  crates; the `wasm-bindgen` family and `io-extras`'s `windows-sys` edge
  copied from the production lock and validated with `cargo metadata
  --locked`). `lock_parity.py` now reports `RESULT PASS`.
  `harness-cli-parity.sh` (new) builds `ebound` from the production workspace
  (`--locked`, default features) and `ebr-pack` from the harness and compares
  archive SHA-256 for 4 profiles x 2 layouts. `ebr-pack` rows carry a
  `build_note`. Informational: production's `cargo test` resolve enables
  `flate2`'s `zlib-rs` backend through a dev-dependency, while release
  builds of `ebound` and the harness use `miniz_oxide`.
- **Constraint (open).** Decision-grade timing against incumbents must use
  `ebound` from the production workspace, or first demonstrate timing parity
  between the harness and CLI builds on the same inputs.
- **Verification.** `lock_parity.py`: PASS. `harness-cli-parity.sh`: see
  "Verification record".

### R1-07 (HIGH) — planner regret did not describe production decisions

- **Evidence.** `ebr-planner` built its archive with `chunk_size =
  bytes.len()`, one Chunk per file, while production splits every object with
  `chunker::select_parameters` + `chunk_ranges` (at most 2-8 MiB per Chunk),
  so regret was computed for Chunks production never plans. The
  `exhaustive_regret` row printed `selected_complete_cost` in the
  generation's own basis (payload only for v1-v3) next to a v5-basis
  `optimal_cost`, while `regret` used a third recosting. A selected DEFLATE
  reconstruction (common in gzip-bearing corpus items) produced negative regret
  with no flag. Per-chunk regret charges each candidate's plan record per
  Chunk, but an archive stores each distinct plan once, so summing it is not
  the complete archive cost. The "strict superset of production's candidates"
  claim was stated, not tested.
- **Fix.** The binary chunks items with production's chunker
  (`production_chunk_ranges`, `archive_for_ranges`), runs drift reports,
  exhaustive search, regret, greedy, and tree per Chunk (explicit
  `--max-chunks`, recorded with `truncated`), and reports
  `selected_trace_cost` + `trace_cost_basis`, `selected_recosted_cost`,
  `in_scope`, and `regret` (`None` out of scope). New `archive_regret` rows
  give `ArchiveRegretBounds`: selected Chunk-stage archive cost with distinct
  plan records charged once, a feasible upper bound and a valid lower bound on
  the optimum, and the bracketing regret bounds.
- **Verification.** `every_production_chunk_candidate_is_in_the_exhaustive_universe`
  (all profiles x generations, numeric and text inputs);
  `archive_regret_bounds_charge_shared_plan_records_once`; smoke on a 4.4 MB
  item at `balanced` (multiple Chunks, 2 searched, 0 drift differences, bounds
  ordered).

### R1-08 (MEDIUM) — byte buckets unbalanced; no independent cross-check

- **Evidence.** In `indexed_byte_accounting` the ChunkData section's own
  64-byte header was in no named bucket, unknown section kinds fell into
  `_ => {}`, and the only check summed the section extents from the same
  authenticated walk that refuses gaps. The "cross-validates against
  production's own codec_usage" claim in the progress log was a manual smoke
  step, not code.
- **Fix.** New `chunk_data_section_header_bytes`, `chunk_frame_count`,
  `other_section_bytes`/`other_section_kinds`, and `named_bucket_total_bytes`
  with a hard error unless the named buckets sum to the file size.
  `cross_check_indexed` compares the frame walk against `inspect`'s per-codec
  `stored_bytes`/`chunk_count` (derived from the Index and Chunk table) and
  fails on mismatch; it is marked not applicable for archives with whole-object
  ReconstructionRegions (`ArchiveCounts::whole_object_region_count`).
  `ebr-pack` and `ebr-inspect-bytes` report it.
- **Verification.** `named_buckets_sum_exactly_to_the_file_size`;
  `frame_walk_matches_inspection_independently` (including a tampered
  inspection that must fail); smoke on a real tree.

### R1-09 (MEDIUM) — rolling-hash CDC baselines compared at different realized chunk sizes

- **Evidence.** A mask equal to the next-lower power of two of `target`
  gives expected size `min + T(1 - e^(-(max-min)/T))`; with production presets
  (`min = target/4`, `max = 4*target`) that is about 1.2x `target`. FastCDC
  normalized chunking and `gear-norm-v1` land near `target`, so dedup ratios
  "at matching size targets" were compared at different real chunk sizes,
  biasing against Rabin/Buzhash/TTTD.
- **Fix.** `SizeCalibration::MeanMatched` (default): an LBFS/TTTD-style
  modulo divisor `D = target - min` (expected size within 1% of `target` at
  every preset); `--size-calibration pow2` reproduces the old rule; algorithm
  ids record the rule; `dedup` rows report `mean_chunk_bytes` and
  `mean_unique_chunk_bytes`.
- **Verification.** `mean_matched_calibration_hits_target_and_pow2_overshoots`
  for Rabin and Buzhash (12 MB random input: mean within 8% of target;
  pow2 above 1.12x).

### R1-10 (MEDIUM) — TTTD used the first backup breakpoint

- **Evidence.** Eshghi & Tang's pseudocode overwrites `backupBreak` at every
  `D'` match (`if ((p mod D') == D'-1) backupBreak = currP`), so the last match
  before `Tmax` is used. The implementation kept the first, cutting
  systematically earlier.
- **Fix.** Last backup, modulo divisors `D = target - min` and `D' = D/2`
  (the paper's 540/270 ratio); ids record `backup-last`.
- **Verification.** `uses_the_last_backup_breakpoint_like_the_paper`
  (brute-force recomputation of every backup position).

### R1-11 (MEDIUM) — RAM misattributed and not RAM; AE value width

- **Evidence.** RAM is Widodo, Lim & Atiquzzaman, *A new content-defined
  chunking algorithm for data deduplication in cloud storage*, FGCS 71 (2017):
  take the maximum byte `M` of a fixed window at the chunk start, then cut at
  the first later byte `>= M`. The module cited a 2021 paper by other authors
  and implemented "cut where the byte is the maximum of the trailing
  `window` bytes", which forgets the initial maximum once it scrolls out and
  can cut earlier. For AE, the authors' own reference implementation (destor
  `src/chunking/ae_chunking.c`) compares 8-byte big-endian values with
  `window = avg/(e-1)`, while the 2024 comparative study (Gregoriadis et al.,
  arXiv:2409.06066) uses byte values with `h = target - 256`; the harness had
  only the byte variant with an ad hoc `window = target`.
- **Fix.** RAM implements the published rule (`ram-widodo2017-v2`, default
  window `target - 256`); AE gains `--value-width 8` with the destor
  calibration and uses `target - 256` for width 1.
- **Verification.** `cuts_at_the_first_byte_reaching_the_fixed_window_maximum`
  (brute force), `keeps_the_initial_window_maximum_after_it_scrolls_away`
  (fails on the old rule), calibration tests for RAM and both AE widths.

### R1-12 (HIGH) — codec resources unmeasured; unfair configurations

- **Evidence.** `ExternalCodecResult`/`RoundtripResult` held sizes and times
  only. Windows differed widely and invisibly: production zstd uses a 1 MiB
  window (`ZSTD_WINDOW_LOG = 20`), the external zstd default frame 2-8 MiB, the
  LDM candidate 16 MiB, xz preset 9 a 64 MiB dictionary, brotli 4 MiB. The
  only LDM candidate changed the window and LDM together. PPMd ran only at
  order 6 with a 1 MiB model, which no PPMd tool ships and which restarts the
  model on all but small inputs. The matrix ran every candidate in one process,
  so no per-candidate memory could be measured even externally.
- **Fix.** Every result and matrix row carries `window_bytes` (zstd parsed
  from the frame header), `threads` (1), `container`, and scoped
  `encode_memory`/`decode_memory`. PPMd runs at 7-Zip levels 5/7/9 with
  7-Zip's model-size reduction for small inputs; a window-matched zstd
  control (`window_log 24`, LDM off) precedes the LDM candidate; brotli q11
  also runs at `lgwin` 24; lz4 uses the CLI's 4 MiB blocks without block
  checksums; DEFLATE rows name their `miniz_oxide` backend. `--candidate`
  filters before execution so one candidate can run per process.
- **Verification.** `zstd_frame_window_is_parsed_from_the_header`,
  `ppmd_7zip_mapping_matches_7zip_levels_and_reduces_for_small_inputs`,
  `every_result_reports_window_threads_and_container`,
  `candidate_filter_runs_only_matching_labels_and_reports_resources`; smoke
  runs one PPMd candidate in its own process.

### R1-13 (MEDIUM) — netem loss unit and bandwidth granularity artifacts

- **Evidence.** `shaped_body_stream` rolled loss once per `--chunk-size`
  chunk, and the README advises raising `--chunk-size` up to 1 MiB for high
  rates, which silently cut the per-byte loss rate 64x. The token bucket
  clamps refilled tokens to `capacity`; `tokio::time::sleep` rounds up to
  whole milliseconds, so with a burst smaller than the per-millisecond byte
  budget every oversleep credit was discarded. The provisional calibration
  (`--burst-down-bytes 4096`, 16 KiB chunks) achieved 61.7 and 106.7 Mbit/s
  at configured 100 and 1000, matching whole-millisecond sleeps of 16 KiB
  chunks (not per-call overhead, as `calibration.md` guessed).
- **Fix.** Loss is rolled per simulated 1448-byte segment
  (`--loss-unit packet`, default; `chunk` keeps the legacy rule) with one RTO
  stall per lost segment. The bucket's effective capacity is at least 5 ms of
  the configured rate (as `tc tbf` requires `burst >= rate/HZ`).
  `calibration.md` is marked superseded.
- **Verification.** `per_packet_loss_rate_does_not_depend_on_chunk_size`;
  existing bandwidth/proxy tests; the ignored smoke test
  `smoke_high_rate_bandwidth_is_not_limited_by_timer_granularity` achieved
  95.8% of 100 Mbit/s with the same 4096-byte burst and 16 KiB chunks.

### R1-14 (MEDIUM, open) — no TCP slow start

- **Evidence.** The proxy charges one RTT per request regardless of response
  size and paces at the full configured rate immediately. Real TCP on a fresh
  connection needs about `log2(size / (10 * MSS))` extra round trips before
  reaching full rate, so the emulator favors few large range requests.
- **Disposition.** A documented simplification rather than a code defect;
  adding a congestion-window model is future emulator work. Use constraint
  recorded in `crates/ebr-netem/README.md` and below.

### R1-15 (MEDIUM) — stale Windows directory-entry sizes in the scratch sampler

- **Evidence.** The Windows run of `stream_unpack_counts_the_staging_spill`
  failed: production had spilled 25,165,824 bytes, the sampler saw 0.
  `DirEntry::metadata` on Windows returns the directory enumeration's cached
  size, which stays stale for a file another handle is still writing. The
  pre-review sampler had the same flaw for in-progress extraction files.
- **Fix.** On Windows the sampler queries each path directly
  (`symlink_metadata`).
- **Verification.** The test passes on Windows after the fix.

### R1-16 (MEDIUM, open) — runner raw `ru_maxrss_kib` includes the runner's own peak

- **Evidence.** `research/tools/ebr/execute.py` records `wait4` rusage of the
  `/usr/bin/time` process as `measurement.rusage.ru_maxrss_kib`, and uses
  `ru.ru_maxrss` for `resource.max_rss_bytes` when GNU time is unavailable.
  Mirroring that path with 512 MiB of runner memory
  (`D:/eb-research/vfork_rss_probe2.py`): `wait4` reported 536,428 KiB for
  `/usr/bin/time -v true`, while GNU time's own report was 1,300 KiB.
  `resource.max_rss_bytes` from GNU time is correct and is what the current
  specs use.
- **Disposition.** Deferred to the runner owner: `research/tools/ebr` is
  outside the Rust harness and in use by the detached size pass. Required fix:
  label `rusage.ru_maxrss_kib` as including the runner's high-water mark, and
  record `max_rss_bytes` as null with an error instead of falling back to
  `ru_maxrss` when GNU time is absent. Until then, analyses must read
  `resource.max_rss_bytes` only from rows whose `measurement.method` is
  `gnu-time-v+wait4`. `ebr_common::measure`'s docs, which pointed at
  `ru_maxrss_kib`, were corrected.

## LOW findings

| ID | Area | Finding | Disposition |
|---|---|---|---|
| L-01 | (a) | `cargo build --all-features` in the production workspace would enable `research-internals` (bytes unchanged, API surface added) | Accepted: never ship `--all-features` builds; noted here |
| L-02 | (a) | `ebr-netem`'s dev-dependency comment claimed a feature-free entrybound build; workspace feature unification enables `research-internals` | Fixed (comment) |
| L-03 | (b) | STREAM and encrypted accounting is a single exact lump (no public per-item or per-segment directory); encrypted `preamble_bytes` assumes the plaintext preamble width | Accepted, documented |
| L-04 | (d) | Container overhead (xz index/CRC64, zlib wrapper, lz4 frame) is inside external sizes; production payloads have none | Fixed by the `container` field; small-object comparisons must account for it |
| L-05 | (f) | `ebr-pack` STREAM lacked `--stream-window auto` | Fixed |
| L-06 | (c) | Rabin modulus is not verified irreducible; Buzhash window is 64, not borg's 4095 | Accepted, documented in modules |
| L-07 | (c) | FastCDC is sized to production presets (`max = 4*target`) rather than the paper's `8*avg` | Accepted: intentional matched sizing |
| L-08 | (c) | `shift-stability` applies one seeded edit per kind per run | Accepted: experiments must sweep seeds |
| L-09 | (d) | `chunk-matrix` uses one fixed preset per profile, while production `dense`/`extreme` select among 2/4 presets per object | Accepted, documented in code; use `ebr-planner` chunking for production boundaries |
| L-10 | (e) | Cohort search covers only the given profile's dictionary construction and is library-only | Accepted, documented |
| L-11 | (g) | Low-RTT bias (about +3 ms at 20 ms) | Use constraint in netem README |
| L-12 | (g) | Shaping only on the client leg; cache not revision-aware; no HOL/TCP-loss dynamics | Accepted, already documented |
| L-13 | (h) | Copies and hard links of held-out content cannot be detected by any path check; the unlock path does not check `heldout-lock.json` or ancestry | Accepted: runner spec guard remains the primary control; unlock requires a design-freeze file that does not exist |
| L-14 | (f) | Windows memory readings are snapshots (`scoped: false`); in-process scoped deltas can be understated by allocator reuse | Use constraint below |

## Use constraints for C-exec (binding until a later review lifts them)

1. Run `python research/harness/lock_parity.py` (must print `RESULT PASS`)
   and `research/harness/harness-cli-parity.sh` (must print `RESULT PASS`)
   before any measurement campaign and after any `Cargo.lock` change.
2. Decision-grade timing comparisons against incumbents use `ebound` built
   from the production workspace (R1-06).
3. Memory: use `pack_memory`, `unpack_memory`, per-stage `memory`, or
   `encode_memory`/`decode_memory`, never `process_lifetime_peak_rss_bytes`.
   Streaming-memory claims require `ebr-access --probe-isolate true`;
   per-codec memory claims require one `--candidate` per process. Windows
   memory claims require the runner's job/psutil sampling. External-tool RSS
   comes only from `resource.max_rss_bytes` with GNU time (R1-16).
4. CDC comparisons are made at matched realized `mean_chunk_bytes`.
5. Codec comparisons report `window_bytes` and decoder memory next to size.
6. Planner regret uses `archive_regret` bounds (or per-chunk rows with
   `in_scope == true`); cohort and JPEG-region stages need their own analysis.
7. Netem: use achieved RTT from logs, avoid configured RTT below 20 ms, and
   any range-coalescing decision needs a slow-start sensitivity analysis or a
   real-path confirmation (R1-14). Regenerate `calibration.md` in a quiet
   window before citing it.

## Verification record

All builds used toolchain 1.98.1 with per-agent targets
(`/root/eb-research/target/b2-review`, `D:/eb-research/target/b2-review-win`,
`/root/eb-research/target/b2-review-cli`). `disk_guard.py` passed before the
builds (C: 374 GB, D: 442 GB free).

| Check | Result |
|---|---|
| `lock_parity.py` | before: FAIL (28 drifted crates); after: PASS |
| WSL `build.sh test --workspace` | 277 passed, 0 failed, 1 ignored (was 249 at `68dfa5d`) |
| WSL `build.sh clippy --workspace --all-targets -- -D warnings` | clean |
| WSL `build.sh fmt --all -- --check` | clean |
| Windows `cargo test --workspace` | 274 passed, 0 failed, 1 ignored (the two Linux-only memory/symlink tests are not compiled on Windows); `stream_unpack_counts_the_staging_spill` failed before the R1-15 fix |
| Windows `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `review-round1-smoke.sh` (WSL, release) | SMOKE PASS: all binaries run; scoped memory, named-bucket balance, independent cross-check, STREAM staging report, random-access deltas, isolated probe stages (pack delta 152.6 MB; isolated verify 52.0 MB, unpack 51.9 MB), calibrated CDC ids, one-candidate codec run, production-chunked planner rows, and held-out refusal by all nine binaries (also with `EB_DATA_ROOT` overridden and via `../`) |
| `harness-cli-parity.sh` (WSL) | RESULT PASS: `ebr-pack` and production `ebound pack` archives SHA-256 identical for 4 profiles x INDEXED/STREAM (8/8) |
| Ignored netem smoke (100 Mbit/s, 4096-byte burst) | 95.8% of configured rate (non-decision-grade) |
| `vfork_rss_probe.py`, `vfork_rss_probe2.py` (WSL, outside the repo) | child `max_rss` inherits the parent peak; GNU time report does not |

Production `crates/` were not modified by this review.

## Files changed in this round

- New: `harness-review-round1.md` (this file), `lock_parity.py`,
  `harness-cli-parity.sh`, `review-round1-smoke.sh`.
- `Cargo.lock` (realigned with production).
- `crates/ebr-common`: `heldout.rs` (R1-01), `measure.rs` (scoped memory,
  documentation).
- `crates/ebr-pack`: `pack.rs`, `main.rs`, `lib.rs` (R1-03), `bytes.rs`,
  `counts.rs`, `bin/ebr-inspect-bytes.rs` (R1-08), `scratch.rs`, `unpack.rs`,
  `bin/ebr-unpack.rs` (R1-04, R1-15), `bin/ebr-verify.rs` (R1-01),
  `encrypt.rs` (rustfmt only), `README.md`.
- `crates/ebr-access`: `random_access.rs` (R1-02), `probe.rs`, `stream.rs`,
  `main.rs`, `lib.rs` (R1-05, R1-01), `README.md`.
- `crates/ebr-chunk`: `algorithms/{mod,rabin,buzhash,tttd,ae,ram}.rs`,
  `dedup.rs`, `main.rs`, `shift_stability.rs` (R1-09..R1-11, R1-01),
  `README.md`.
- `crates/ebr-codec`: `external.rs`, `production.rs`, `matrix.rs`, `main.rs`
  (R1-12, R1-01), `README.md`.
- `crates/ebr-planner`: `lib.rs`, `exhaustive.rs`, `main.rs` (R1-07, R1-01),
  `README.md`.
- `crates/ebr-netem`: `loss.rs`, `bandwidth.rs`, `config.rs`, `proxy.rs`,
  `bin/ebr-origin.rs`, `bin/ebr-netem-calibrate.rs`, `tests/proxy_netem.rs`,
  `Cargo.toml` (R1-13, R1-01, L-02), `README.md`, `calibration.md`
  (superseded note).
- `README.md` (harness).
