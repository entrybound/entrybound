# code-ecf: research-harness API notes

Slice: `crates/entrybound/src/ecf.rs`, `src/ecf/records.rs`, `src/ecf/container.rs`,
`src/ecf/staging.rs` (dev @ 9e44608). Paths below are relative to `crates/entrybound/src`.
A harness crate taking a path dependency on `crates/entrybound` sees only `pub` items reachable
through `lib.rs` (`archive`, `chunker`, `crypto`, `diagnostics`, `eam`, `ecf`, `identity`,
`legacy`, `planner`, `random_access`, `similarity` are public modules; `canonical`, `codec`,
`jpeg_reconstruction`, `reconstruction`, `transform` are private).

## 1. Public entry points usable without modifying production code

### INDEXED writer / reader (ecf.rs:16-20, ecf/container.rs)
| Item | Signature / location | Research use |
|---|---|---|
| `ecf::encode` | `fn encode(&eam::Archive, WriteOptions) -> Result<EncodedArchive>` (container.rs:105) | Build canonical unencrypted Complete INDEXED bytes from any EAM the harness constructs or obtains from `archive::plan_directory`. `EncodedArchive { bytes, archive, identities }`. |
| `ecf::WriteOptions` | `{ include_index: bool }` (container.rs:43-53) | Measure Index overhead (Index on/off), PCI change vs LAI/PCR/AUX stability. |
| `ecf::open`, `open_with_policy`, `open_with_limits` | container.rs:808-828 | Full verification under default or synthetic `ResourceBudget` / `DecodeRequirements`; refusal-threshold experiments; peak-memory experiments (whole archive + all plaintext in RAM). |
| `ecf::verify`, `verify_with_policy`, `verify_with_limits` | container.rs:1086-1102 | Same as open, returning `VerificationReport` (all booleans true on success; failure is `Err(Diagnostic)` with `code()`/`class()`). |
| `ecf::peek_layout` | container.rs:1081 | Layout dispatch from preamble only. |
| `ecf::OpenedArchive { archive, report }` | container.rs:98-102 | `archive.index.chunks: BTreeMap<Digest, ChunkLocation{offset, stored_len}>` is always the rebuilt locator map: per-chunk stored sizes and absolute frame offsets for overhead/compression/random-access modelling. `archive.content_store.physical_order` gives frame order. |
| `ecf::IndexStatus`, `VerificationReport` | container.rs:63-95 | Index tolerance and diagnostic experiments. |
| Constants | `MAGIC`, `PREAMBLE_LEN`(256), `FOOTER_LEN`(128), `SECTION_HEADER_LEN`(64), `CHUNK_FRAME_HEADER_LEN`(64), `CHUNK_FRAME_V2_HEADER_LEN`(96), `FEATURE_*` bits, `FORMAT_NAMESPACE`, `FormatVersion::BOOTSTRAP`, `SectionKind` (ecf.rs:35-93, 217-242) | Offset arithmetic for byte-mutation corpora and overhead accounting. |
| `archive::bootstrap_resource_policy`, `bootstrap_decode_policy` | archive.rs:35-59 | Baseline caller policies to perturb. |

Byte layouts a harness can mutate directly (then re-hash) without internal APIs:
preamble container.rs:2628-2662 (feature checksum over bytes 16..40 at 40..72), section header
container.rs:2595-2621 (SHA-256 of payload at header 24..56), footer container.rs:2850-2869
(SHA-256 of preamble at footer 64..96). Integration tests already contain reusable helpers:
`tests/native_ecf.rs` `locate_section`/`rehash_section`/`field_header`/`field_value` (L726-782),
`tests/compression_planner.rs` `rehash_preamble`, `chunk_section_kind`, `next_record`.

### STREAM + bounded staging (ecf.rs:25-33; staging.rs is reached only through these)
| Item | Research use |
|---|---|
| `ecf::encode_stream(&Archive, StreamWriteOptions, W: Write) -> StreamWriteSummary` | Layout-equivalence (LAI/PCR/AUX equal, PCI differs), STREAM overhead. |
| `ecf::open_stream_with_limits(R: Read, SequentialLimits)` / `verify_stream_with_limits` | Staging experiments: vary `SequentialLimits.staging = StagingLimits { memory_bytes, total_bytes }` or `StagingLimits::memory_only(n)`; read `SequentialArchive.stream.peak_resident_staging_bytes`, `spilled_staging_bytes`, `peak_retained_chunks` (stream.rs:284-300). |
| `ecf::bootstrap_sequential_limits`, `bootstrap_staging_limits` | Defaults: staging 256 MiB resident / 64 GiB cumulative (stream.rs:340-358). |
| `archive::unpack_stream`, `pack_directory_stream` | End-to-end pipe experiments (see tests/stream_layout.rs `DripSource`). |

### Random access (INDEXED range reads; outside this slice but consumes the same wire)
`ecf::open_indexed_random(source, random_access::RandomAccessPolicy)` with
`random_access::{MemoryRandomReadSource, LocalFileRandomReadSource, HttpRangeSource}` and
`AccessTraceEntry` traces (ecf.rs:21-24): count header/section fetches caused by the absence of a
section directory (container.rs:2947-3038 walks every section header).

### Building inputs
- `archive::plan_directory(&Path, PackOptions) -> eam::Archive`, `pack_directory`, `replan_archive`,
  `PackOptions { source_retries, include_index, profile }`.
- `planner::{plan_archive, plan_archive_v2 .. plan_archive_v6, CompressionProfile, analyze}`.
- `chunker::{chunk_ranges, evaluate, select_parameters, FAST_V2, BALANCED_V2, DENSE_V2, EXTREME_V2,
  CHUNK_FRAME_OVERHEAD_BYTES, INDEX_ENTRY_OVERHEAD_BYTES}`.
- `identity::{build_content, build_content_from_ranges, apply_native_identities, sha256_exact,
  physical_container_identity, BOOTSTRAP_CHUNK_SIZE}`.
- `eam` structs have public fields (`Archive`, `ArchiveDescriptor`, `ContentStore`, `Chunk`,
  `ChunkGroup`, `FeatureSet`, `ResourceBudget`, `DecodeRequirements`, `Index`), so a harness can
  hand-build archives: e.g. a ContentObject with more than 1,000,000 chunk references (probe the
  decoder sequence cap in canonical.rs:10/381 against the unchecked encoder), a physical_order
  permutation, or feature-bit combinations, then call `ecf::encode`/`ecf::open`.

### Experiments possible today
1. Fixed overhead: `bytes.len()` minus sum of `index.chunks[*].stored_len` for tiny/large archives,
   with/without Index; compare 64- vs 96-byte frame headers (cross-file feature 0x1).
2. Chunk-cost model error: `chunker::evaluate` charges 64+100 bytes per unique chunk
   (chunker.rs:19-21, 339) while 0x1 archives emit 96-byte frames; measure actual vs modelled.
3. Refusal thresholds: `open_with_limits` over grids of `ResourceBudget` / `DecodeRequirements`.
4. Blast radius / diagnostics: flip bytes per region (preamble, each section header/payload,
   Index header vs payload, footer) and tabulate `(OutcomeClass, ReasonCode)`; confirms that
   Index header damage is fatal while Index payload damage is tolerated.
5. Memory scaling: RSS of `encode`/`open` vs logical size (both fully in-memory).
6. Staging: cumulative `total_bytes` semantics and temp-file growth using `open_stream_with_limits`
   on archives with many shared/late-referenced chunks.
7. Determinism: repeated `encode` byte equality across platforms/threads.

## 2. Internal-only functionality that would need a research-only feature flag

Everything below is `pub(crate)`, `pub(super)` or private. Suggested gate: a
`research-internals` Cargo feature exposing a `#[cfg(feature = "research-internals")] pub mod
research` that re-exports, without changing production behaviour:

| Internal item | Location | Why a harness wants it |
|---|---|---|
| `decode_preamble` / `Preamble` | container.rs:2680-2831 | Parse preamble fields without full open. |
| `decode_sections` / `SectionView`, `decode_footer`, `section_id`, `section_kind` | container.rs:2871-3170 | Section-level size accounting, targeted corruption, section schema experiments. |
| `derived_budget`, `maximum_expansion_ratio`, `expansion_ratio` | container.rs:1337-1450 | Evaluate declared-budget tightness and expansion-ratio distributions on corpora. |
| `normalize_descriptor`, `validate_feature_model`, `PhysicalDeclaration` | container.rs:1159-1335 | Feature-combination validity sweeps. |
| `ChunkFrameEncoder`, `parse_chunk_frame_header`, `chunk_frame_header_len`, `enforce_chunk_bounds` | container.rs:1452-1814 | Per-frame encode/decode micro-benchmarks, frame-level fuzzing. |
| `decode_frame_payload`, `physical_prefix_from_slices`, `validate_frame_dependencies`, `is_group_prerequisite` | container.rs:2338-2537 | Lookback/prefix window experiments (1 MiB cap), group access-cost validation, prerequisite blast radius. |
| `reconstruct_region_members`, `verify_region_representations`, `region_owned_chunk_ids` | container.rs:2044-2336 | JPEG whole-object region cost/verification experiments. |
| `enforce_caller_policy`, `enforce_decode_policy` | container.rs:1104-1141 | Policy comparison in isolation. |
| `prepare_encrypted_plain_parts`, `open_encrypted_plain_parts`, `private_descriptor_declaration` | container.rs:335-742 | Crypto framing size/memory experiments on identical plain records. |
| `ecf::records` encoders/decoders (`encode_descriptor`, `encode_manifest`, `encode_entry_record`, `encode_content_object_record`, `encode_index`, `decode_transform_plans{,_v2,_v3}`, `encode_conversion`, `encode_legacy_preservation`, ...) | records.rs:19-2508 (module is `pub(crate)`) | Manifest/metadata size modelling per record type, compact-manifest feasibility, canonical-encoding fuzzers, independent-reader differential tests. |
| `encoded_transform_plan_len{,_v2,_v3}`, `encoded_dictionary_len`, `encoded_chunk_group_len`, `encoded_reconstruction_region_len`, `encoded_reconstruction_data_len`, `encoded_legacy_evidence_len` | ecf.rs:95-215 | Planner objective container-cost terms (used by planner.rs:27-28, 514-523, 1679-1687). |
| `staging::ChunkStaging` (`insert`, `read`, `release`, `peak_resident_bytes`, `spilled_bytes`) | staging.rs:47-246 | Direct staging benchmarks (FIFO spill, O(n) release, temp-file growth) without a full STREAM pass. |
| `canonical::{RecordBuilder, decode_record, decode_record_stream, MAX_SEQUENCE_ITEMS}` | canonical.rs | Record-level fuzzing and sequence-limit probes. |
| `codec::{plan_mode, zstd_plan, zstd_prefix_plan, encode_payload*, decode_payload*, aggregate_archive_decode_requirements, ZSTD_WINDOW_BYTES}` | codec.rs | Codec-per-frame experiments outside the planner. |
| `jpeg_reconstruction::{MAX_JPEG_BYTES, MAX_JXL_BYTES, MAX_REGION_CHUNKS, REGION_MEMBER_PLAN_REF}` | jpeg_reconstruction.rs:15-21 | Bound-sensitivity experiments. |

No fuzz targets exist in the repository (no `fuzz/` directory, no cargo-fuzz/proptest dependency);
a fuzz crate would need the same feature gate for record- and frame-level entry points, or can
fuzz whole containers through public `ecf::open`/`ecf::open_stream_with_limits` today.
