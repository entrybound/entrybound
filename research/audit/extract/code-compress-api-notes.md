# code-compress: research-harness API notes

Scope: `crates/entrybound/src/{planner,chunker,codec,transform,reconstruction,jpeg_reconstruction,similarity}.rs`
at `dev` 9e44608. Goal: tell a separate research harness crate (path dependency on
`crates/entrybound`) what it can call **without modifying production code**, and what
would need a research-only feature flag.

Module visibility (`crates/entrybound/src/lib.rs`):

| Module | Visibility | Harness access |
|---|---|---|
| `planner` | `pub mod` | yes (only the `pub` items) |
| `chunker` | `pub mod` | yes |
| `similarity` | `pub mod` | yes |
| `codec` | `mod` (private) | **no** |
| `transform` | `mod` (private) | **no** |
| `reconstruction` | `mod` (private) | **no** |
| `jpeg_reconstruction` | `mod` (private) | **no** |
| `eam`, `ecf`, `identity`, `archive`, `random_access`, `crypto`, `diagnostics` | `pub mod` | yes |

## 1. Public API usable today

### 1.1 Chunking (`entrybound::chunker`)

- `ChunkingParameters { chunker_id: &'static str, minimum_size, target_size, maximum_size }`.
  All fields are public, so custom policies can be built. `chunker_id` is `&'static str`;
  use a `const` or `Box::leak(String)` for sweeps. Validation rules: min > 0, min <= target <= max,
  target a power of two >= 2 (`chunker.rs:359-373`).
- Frozen presets: `FAST_V2`, `BALANCED_V2`, `DENSE_V2`, `EXTREME_V2` (`chunker.rs:78-108`).
- `chunk_ranges(&[u8], ChunkingParameters) -> Result<Box<[ChunkRange]>>`: gear-norm-v1 CDC.
- `chunk_ranges_encrypted(&[u8], ChunkingParameters, &EncryptedBoundaryKey)`.
  `EncryptedBoundaryKey::{SecretGearTable(Box<[u64;256]>), PhteAes128 { polynomial: u128, aes_key: [u8;16] }}`
  is a public enum with public fields. A harness can build test keys directly (polynomial < 2^127-1)
  to benchmark or analyse boundary leakage without the crypto key hierarchy.
- `evaluate(&[&[u8]], ChunkingParameters) -> Result<ChunkingEvaluation>`: estimated cost,
  unique chunk count, manifest references, unique plaintext bytes.
- `select_parameters(&[&[u8]], &[ChunkingParameters])` and `select_parameters_encrypted(...)`.
- Cost constants: `CHUNK_FRAME_OVERHEAD_BYTES`, `INDEX_ENTRY_OVERHEAD_BYTES`, `CHUNK_REFERENCE_OVERHEAD_BYTES`.

Experiments enabled: chunk-size sweeps, boundary distribution and resync studies, FastCDC
comparisons (harness implements its own alternative), proxy-cost versus final-size validation
(combine with 1.4), PHTE and secret-table throughput.

### 1.2 Similarity (`entrybound::similarity`)

- `SimilarityPolicy` (all fields public; `policy_id: &'static str`) and presets
  `FAST_V3_SIMILARITY`, `BALANCED_V3_SIMILARITY`, `DENSE_V3_SIMILARITY`, `EXTREME_V3_SIMILARITY`.
- `cluster(&BTreeMap<Digest, eam::Chunk>, SimilarityPolicy) -> Vec<SimilarityCohort>`.
  Build the map with `identity::build_content_from_ranges(plaintext, &ranges, planner::UNPLANNED_PLAN_ID)`.
- `SIMILARITY_ID`.

Experiments enabled: parameter sweeps (k, threshold, cohort bounds), cohort recall/precision
against exact resemblance, and sensitivity to shifted content in large chunks.
Not exposed: the fingerprint function (`fingerprint`, `intersection_count`, `fnv1a` are private).

### 1.3 Planner (`entrybound::planner`)

- `CompressionProfile` with `planner_id()`, `planner_v1_id()` through `planner_v5_id()`,
  `chunking_candidates()`, `similarity_policy()`, `lookback_candidates()`,
  `from_planner_id()`, `FromStr`.
- `plan_archive` (v1), `plan_archive_v2`, `plan_archive_v3`, `plan_archive_v4`, `plan_archive_v5`,
  `plan_archive_v6`: `(&mut eam::Archive, CompressionProfile) -> Result<PlanningReport>`.
  Each mutates the archive in place and resets the relevant physical state, so you can clone one
  chunked Archive and re-plan it under every version and profile.
- `PlanningReport`: per-codec chunk counts, transformed, dictionary, lookback and reconstructive
  counts, cohorts, attempts, cost rejections, `selected_plans`.
  Caveat: after v6 the per-codec counters still describe the pre-region v5 assignment.
- `analyze(&[u8]) -> ContentAnalysis`: the integer compressibility probe.
- `zstandard_wins(logical_len, encoded_len)`, `MINIMUM_GAIN_BYTES`, `MINIMUM_GAIN_BASIS_POINTS`,
  `ZSTD_ATTRIBUTABLE_OVERHEAD_BYTES`, `UNPLANNED_PLAN_ID`.

After planning, inspect decisions on the Archive itself:
- `archive.transform_plans`: codec, params and transforms.
- `archive.content_store.chunks[..].plan_ref` and `.group_ref`.
- `archive.content_store.dictionaries`, `chunk_groups` (`max_lookback`, `max_preceding_bytes`) and `physical_order`.
- `archive.content_store.reconstruction_data`, `reconstruction_fallbacks` (v5 reasons),
  `reconstruction_regions` and `reconstruction_audits` (v6 reasons).
- `archive.descriptor.features.incompat`, `descriptor.decode`, `planner_id`, `chunker_id`.

### 1.4 Building archives by hand and measuring encoded output

- `identity::build_content(plaintext, chunk_size, plan_ref)` for fixed chunks, and
  `identity::build_content_from_ranges(plaintext, &[Range<usize>], plan_ref)`.
- `eam::Archive`, `ArchiveDescriptor`, `ContentStore`, `EntrySet`, `Entry`, `LogicalPath`, etc.
  have public fields and constructors. Template: `archive_with_content` in
  `crates/entrybound/tests/cdc_dedup.rs:240-309`; `archive_for` in `tests/reconstructive_transform.rs:181+`
  and `tests/jpeg_whole_object.rs`.
- `ecf::encode(&Archive, ecf::WriteOptions { include_index }) -> EncodedArchive { bytes, archive, identities }`
  for INDEXED size and PCI/PCR/LAI/AUX. `identity::apply_native_identities` gives roots without encoding.
- `ecf::encode_stream(&Archive, StreamWriteOptions { window, budget_declared }, sink)` for the STREAM layout.
- `ecf::open`, `open_with_limits(bytes, ResourceBudget, DecodeRequirements)`, `verify` for decode
  and decoder-memory enforcement. `archive::bootstrap_resource_policy()` and `bootstrap_decode_policy()`
  are the defaults.
- `archive::inspect(&OpenedArchive)` gives `CodecUsage { codec, chunk_count, logical_bytes, stored_bytes }`,
  `ChunkStatistics`, `CrossFileInspection` and `WholeObjectInspection`.
  `archive::explain(&OpenedArchive)` gives a `CompressionExplanation` savings breakdown.
  Caveat: for `*-v6` archives explain's baseline uses the v1-v3 Zstd-only path (`planner.rs:1800-1820`).
  Recompute baselines in the harness rather than trusting the savings split.

### 1.5 Filesystem-level end-to-end

- `archive::plan_directory(&Path, PackOptions { source_retries, include_index, profile }) -> Archive`:
  capture, CDC selection and `plan_archive_v6`, without writing.
- `archive::pack_directory` and `pack_directory_stream` produce encoded bytes.
- `archive::replan_archive(&Archive, CompressionProfile)` replans from verified logical objects.
  Note it records a different `chunker_id` string (`filesystem.rs:334-339`), so compare PCR against a fresh pack.
- `archive::prepare_repack(&OpenedArchive, RepackOptions)` returns `RepackAnalysis` with before/after statistics.
- `crypto::pack_directory_encrypted(&Path, PackOptions, EncryptedWriteOptions)`: encrypted pack,
  which overwrites planner_id with `*-enc-v1`.

### 1.6 Random access and remote access costs

- `ecf::open_indexed_random(source, random_access::RandomAccessPolicy::default())`, then
  `.read_entry(&LogicalPath)` returns `RandomAccessRead { bytes, report }`.
- `report` (`RandomAccessVerificationReport`) exposes `dependency_chunk_count`, `bytes_fetched`,
  `range_request_count`, `access_trace` and verification flags. Use it to measure transitive lookback
  dependency cost, region reconstruction cost, and HTTP range coalescing versus physical order.
- Sources: `random_access::MemoryRandomReadSource::new(bytes)`, `LocalFileRandomReadSource::open(path)`,
  `HttpRangeSource::open(url)`.
- `RandomAccessPolicy` fields are public: `max_dependency_chunks`, `max_decoded_logical_bytes`,
  `coalesce_gap_bytes`, and others.

### 1.7 Experiment-to-API map

| Research question | Calls |
|---|---|
| Chunk-size / proxy validity | `chunker::evaluate`, `chunk_ranges` -> `build_content_from_ranges` -> `plan_archive_v6` -> `ecf::encode` (bytes.len()) |
| Gain margins / profile trade-offs | clone Archive; `plan_archive_v4/v5/v6` per profile; `encode`; time with `std::time::Instant` in the harness |
| Probe false negatives | `planner::analyze` per chunk; then single-chunk Archive planned by `plan_archive_v4` and check `report.transformed_chunks` |
| Lookback transitive access | `plan_directory(Dense/Extreme)` on similar files; `encode`; `open_indexed_random(...).read_entry` on last cohort member; `dependency_chunk_count` vs `chunk_groups[..].max_lookback` |
| DEFLATE eligibility under real CDC | `plan_directory(Dense)` on .gz/.zlib corpus; count `reconstruction_data`, `reconstruction_fallbacks` |
| JPEG eligibility / savings | single-object Archive -> `plan_archive_v6(Dense)`; `reconstruction_regions`, `reconstruction_audits`; `explain().jpeg_*` |
| Audit/feature-bit overhead | `plan_directory(Balanced)` on non-JPEG corpus; `descriptor.features.incompat`; `reconstruction_audits.len()`; encode size delta vs `plan_archive_v4` |
| Cross-platform determinism | `pack_directory` on fixed corpus; hash `bytes` and `identities` on each platform |
| Encrypted CDC throughput/leakage | `chunk_ranges_encrypted` with synthetic `EncryptedBoundaryKey` values |
| Decoder memory declarations | `open_with_limits` with shrinking `DecodeRequirements`; external RSS measurement during `verify` |

## 2. Internal-only functionality (would need a research-only feature flag)

None of the following is reachable from outside the crate. Exposing it requires adding a
cfg-gated re-export module to production code, for example `#[cfg(feature = "research-internals")] pub mod research`.

**codec.rs (private module)**
- `store_plan`, `zstd_plan(level)`, `zstd_dictionary_plan`, `zstd_prefix_plan`, `zstd_transformed_plan`,
  `lz4_plan`, `lzma2_plan(preset, dict, transforms)` and `with_pipeline`: construct canonical plans.
- `encode_payload`, `decode_payload`, `encode_payload_with_dictionary` and `_with_prefix`,
  `_with_reconstruction`, `encode_transformed_payload`, and matching decoders.
  These give exact Entrybound codec output for per-codec/level size, speed and decode-memory benchmarks.
- `train_dictionary(samples, cap)`, `validate_dictionary`, `validate_plan(s)`, `plan_mode`,
  `aggregate_decode_requirements` and `aggregate_archive_decode_requirements`.
- Constants: `SUPPORTED_LEVELS`, `SUPPORTED_LOOKBACKS`, `SUPPORTED_LZMA2_CONFIGURATIONS`,
  `SUPPORTED_DICTIONARY_CONSTRUCTIONS`, `ZSTD_WINDOW_LOG/BYTES` and `ZSTD_WORKING_SET_BYTES`.
- Workaround without a flag: call the `zstd` 0.13.3 / `lz4_flex` 0.14.0 / `lzma-rust2` 0.20.0 crates
  directly with the same settings (window log 20, no LDM, no checksum, content size on; raw LZ4 block;
  raw LZMA2 with the preset/dict pairs). Byte-equality with Entrybound is then unverified.

**transform.rs (private)**
- `delta8_step`, `byte_shuffle_step`, `deflate_reconstruct_step`, `jpeg_reconstruct_step`,
  `validate_pipeline`, `forward_pipeline(_with_reconstruction)`, `inverse_pipeline(_with_reconstruction)`.
  The harness can reimplement delta8 and shuffle from `docs/codec-transform-v1.md` but cannot prove equivalence.

**reconstruction.rs (private)**
- `try_forward(original, max_chain) -> Option<ReconstructCandidate { intermediate, data }>`,
  `verified_forward`, `inverse`, `validate_data`, and the limits constants.
  Needed for per-stream DEFLATE eligibility, correction-size and memory studies independent of CDC.
  Without a flag: plan a single-chunk Archive via `plan_archive_v5`, or call `preflate-rs` 0.7.6 directly.

**jpeg_reconstruction.rs (private)**
- `verified_forward(&[u8]) -> Result<VerifiedJpegRepresentation, JpegAttemptFailure>`, `inverse`,
  `region_identity`, and limits (`MAX_JPEG_BYTES`, `MAX_JPEG_PIXELS`, `JPEG_WORKING_SET_BYTES`,
  `MAX_REGION_CHUNKS`, `MAX_REGION_EXPANSION_RATIO`).
  Needed for JPEG-corpus eligibility, memory-per-megapixel and cross-decoder (libjxl) tests.
  Without a flag: use `plan_archive_v6(Dense)` on single-object archives and read
  `reconstruction_audits` reasons, or call `jixel` 0.2.19 / `jxl-oxide` 0.12.6 directly.

**planner.rs private functions**
- `select_v4_plan`, `v4_candidates`, `complete_candidate_cost`, `qualifies_v4`,
  `reconstruction_candidates`, `reconstruction_max_chain`, `v5_independent_cost`, `qualifies_reconstruction`,
  `select_cohort_plan` / `CohortSelection`, `train_cohort_dictionary`, `cohort_prefix`,
  `maximum_preceding_bytes`, `qualifies_against_independent`, `jpeg_region_candidates`,
  `v6_ordinary_payload_cost`, `chunk_group_id`.
- `independent_encoded_len` is `pub(crate)`.
- These are needed to evaluate every candidate's cost (not just the winner) and to sweep margins
  without forking the planner. A flag could expose a `candidate_costs(profile, planner_version, plaintext)`
  enumerator returning (plan, payload bytes, record bytes, qualified?).

**ecf (pub(crate) helpers used by the cost model)**
- `ecf::encoded_transform_plan_len`, `_v2_len`, `_v3_len`, `encoded_dictionary_len`,
  `encoded_chunk_group_len`, `encoded_reconstruction_data_len`, `encoded_reconstruction_region_len`:
  exact record costs for validating planner cost constants.
- `ecf::container::physical_prefix_from_slices` (lookback prefix construction).

**similarity.rs private**
- `fingerprint`, `intersection_count`: needed to measure sketch overlap directly.

## 3. Practical cautions for harness authors

- Planners assume `Chunk.plaintext` is present and its SHA-256 matches `chunk_id`; v1/v4/v5 re-verify and fail with `ChunkDigestMismatch`.
- All planning is in-memory and single-threaded. Size corpora accordingly, and run profiles in separate processes for parallel sweeps.
- `plan_archive_v6` always runs v5 first. To isolate JPEG effects, compare `plan_archive_v5` versus `plan_archive_v6` on clones (as `tests/jpeg_whole_object.rs:55-60` does).
- Encoding validates that feature bits cover used techniques. Hand-edited archives must keep `descriptor.features.incompat` consistent or `encode` fails with `UnsupportedRequiredFeature`.
- Non-frozen custom `chunker_id` strings are accepted by the chunker. Whether `encode` or readers accept arbitrary chunker IDs was not verified in this slice.
