# code-access: research-harness API notes

Slice: `crates/entrybound/src/ecf/stream.rs`, `src/ecf/random.rs`, `src/random_access.rs`,
`src/archive.rs`, `src/archive/inspection.rs` (branch `dev` @ 9e44608).

A research harness crate with `entrybound = { path = "../crates/entrybound" }` can run the
experiments below using only public items. Items marked **internal** would need a
research-only feature flag (or a `#[doc(hidden)] pub` re-export) to be reachable.

Note: `entrybound` declares no Cargo features today, and `reqwest` (blocking + rustls) is an
unconditional dependency (`crates/entrybound/Cargo.toml:32`).

## 1. Building archives in both layouts

| Goal | Public API |
|---|---|
| Plan a directory into an in-memory EAM | `entrybound::archive::plan_directory(&Path, PackOptions) -> Result<Archive>` (`PackOptions { source_retries, include_index, profile: planner::CompressionProfile }`) |
| Re-plan an existing model with another profile | `archive::replan_archive(&Archive, CompressionProfile)` |
| Encode INDEXED (with/without Index) | `ecf::encode(&Archive, ecf::WriteOptions { include_index }) -> EncodedArchive { bytes, archive, identities }` |
| Encode STREAM to any `Write` | `ecf::encode_stream(&Archive, StreamWriteOptions { window: StreamWindow::{Auto, Ceiling(n)}, budget_declared }, W) -> StreamWriteSummary` |
| Pack directly | `archive::pack_directory`, `archive::pack_directory_stream` (both build the full in-memory Archive first) |
| Layout conversion / repack | `archive::prepare_repack(&OpenedArchive, RepackOptions { mode, layout, index, stream_window })` |

Required STREAM window for a plan (no private access needed): encode with
`StreamWindow::Auto` into `std::io::sink()` and read `StreamWriteSummary::dedup_window`
(costs a full encode). Refusal path: `StreamWindow::Ceiling(n)` returns
`ReasonCode::StreamWindowExceeded`.

## 2. Sequential (STREAM) reading experiments

- `ecf::open_stream_with_limits(R: Read, SequentialLimits) -> SequentialArchive { opened, stream: StreamReport }`
- `ecf::verify_stream_with_limits`, `open_stream`, `open_stream_with_policy`, `verify_stream`
- `SequentialLimits { budget: ResourceBudget, decode: DecodeRequirements, staging: ecf::StagingLimits { memory_bytes, total_bytes }, max_item_bytes, max_container_bytes, content: StreamContentPolicy::{Verify, Stage, Retain} }`
- Defaults: `ecf::bootstrap_sequential_limits()`, `ecf::bootstrap_staging_limits()`,
  `archive::bootstrap_resource_policy()`, `archive::bootstrap_decode_policy()`
- Memory/retention metrics already exposed in `StreamReport`: `peak_retained_chunks`,
  `peak_resident_staging_bytes`, `spilled_staging_bytes`, `chunk_frames`, `manifest_records`,
  `body_len`, `total_len`, `dedup_window`, `access: StreamAccessProfile`.
- Extraction under staging: `archive::unpack_stream(R, &Path, ExtractionPolicy, SequentialLimits) -> (ExtractionReport, StreamReport)`.
- Framing constants for byte-level fault injection: `ecf::STREAM_ITEM_MAGIC`,
  `STREAM_ITEM_HEADER_LEN`, `STREAM_FOOTER_LEN`, `StreamItemTag::{wire_id, from_wire}`,
  `ecf::PREAMBLE_LEN`, `CHUNK_FRAME_HEADER_LEN`, `CHUNK_FRAME_V2_HEADER_LEN`.
- Harness adapters (pattern from `tests/stream_layout.rs` L42-96): a write-only sink and a
  `DripSource` returning N bytes per `read`; add a counting/tracking global allocator to
  measure RSS; wrap `Read` to inject truncation or bit flips at item boundaries.

## 3. Random-access (INDEXED) experiments

- `ecf::open_indexed_random(impl RandomReadSource + 'static, RandomAccessPolicy) -> RandomAccessArchive`
- `RandomAccessArchive::metadata() -> &RandomAccessMetadata` (entries, content objects,
  section directory, source revision), `metadata_report()`, `read_entry(&LogicalPath) -> RandomAccessRead { bytes, report }`
- `RandomAccessVerificationReport` exposes `bytes_fetched`, `range_request_count`,
  `dependency_chunk_count`, `chunk_count_verified`, `index_status`
  (`PresentValid | RebuiltAbsent | RebuiltInvalid`), identity statuses and the full
  `access_trace: [AccessTraceEntry { offset, length, purpose, cache_hit }]`.
- Sources: `random_access::MemoryRandomReadSource::new(bytes)`,
  `LocalFileRandomReadSource::open(path)`, `HttpRangeSource::open(url)`.
- **Custom sources are public**: implement `random_access::RandomReadSource`
  (`len`, `read_exact_at`, `revision`) to inject latency, count/log requests, simulate
  ETag changes, short reads, or server misbehavior, without touching production code.
- Policy knobs (`RandomAccessPolicy`): `max_individual_range_bytes`,
  `max_total_bytes_fetched`, `max_range_requests`, `max_metadata_bytes`,
  `max_section_count`, `max_chunk_frames_scanned`, `max_dependency_chunks`,
  `max_decoded_logical_bytes`, `max_cached_bytes`, `max_encrypted_segment_header_walk`,
  `max_trace_entries`, `coalesce_gap_bytes`, `resource_policy`, `decode_policy`.
- Force the Index-less header-scan path with `WriteOptions { include_index: false }`;
  force the invalid-Index path by flipping a byte in the Index section (locate via
  `metadata().section_directory`).
- Encrypted variants (outside this slice): `crypto::open_indexed_random_encrypted`,
  `crypto::inspect_indexed_random_encrypted_public`.
- HTTP experiments: tests show a minimal range server (`tests/random_access.rs` L45-138, `RangeServer::start`);
  a harness can reuse the pattern or front a real server/CDN.

## 4. Inspection and explanation

- `archive::list(&Archive) -> Vec<ListedEntry>`
- `archive::inspect(&OpenedArchive) -> ArchiveInspection` (chunk statistics, codec usage,
  cross-file access costs, reconstruction and whole-object views, declared resources)
- `archive::explain(&OpenedArchive) -> CompressionExplanation` (needs retained plaintext:
  INDEXED `ecf::open*` or STREAM with `StreamContentPolicy::Retain`)
- `archive::structured_explain`, `inspection_json`, `random_inspection_json`,
  `archive_diff`, `archive_metadata_diff`
- Full in-memory verification for comparison: `ecf::open`, `open_with_limits`,
  `verify`, `verify_with_limits`, `peek_layout`.

## 5. Internal-only functionality (would need a research feature flag)

| Internal item | Location | Why a harness wants it | Public workaround |
|---|---|---|---|
| `RangeSession` (`new`, `read`, `prefetch`, `check_stable`, `trace`) | `src/random_access.rs:431-664` (`pub(crate)`) | Evaluate coalescing gap, cache eviction and request shaping in isolation | Only via `open_indexed_random` + custom `RandomReadSource` |
| `plan_emission` / `EmissionPlan` | `src/ecf/stream.rs:364-512` (private) | Compute STREAM frame order and required window without encoding; compare alternative orderings | Encode to `io::sink()` with `Auto` |
| `Scanner` state incl. `observed_window` | `src/ecf/stream.rs:1077-2244` (private) | Compare declared vs observed window; per-item timing | None (observed window not reported) |
| `StagedChunks` / `ChunkStaging` | `src/ecf/stream.rs:302-312`, `src/ecf/staging.rs` (`pub(crate)`/private) | Staging spill policy experiments | Only `StagingLimits` + `StreamReport` metrics |
| Dependency-closure computation (`group_prerequisites`, `decode_one`, `scan_all_chunk_headers`) | `src/ecf/random.rs:692-956` (private) | Measure transitive lookback closure without fetching | `read_entry` then `report.dependency_chunk_count` |
| Frame codec helpers (`ChunkFrameEncoder`, `parse_chunk_frame_header`, `decode_frame_payload`, `physical_prefix_from_slices`, `reconstruct_region_members`, `enforce_chunk_bounds`) | `src/ecf/container.rs` (`pub(crate)`) | Per-frame decode cost, prefix construction, malformed-frame fuzzing | Byte-level mutation of encoded archives |
| Codec plans (`store_plan`, `zstd_plan`, `lz4_plan`, `lzma2_plan`, `plan_mode`, `encode_payload`) | `src/codec.rs` (`mod codec` is private in `lib.rs:10`) | Build archives with hand-chosen codecs per Chunk (as the stream.rs unit test does) | Only via planner profiles |
| Inspection helpers (`chunk_statistics`, `separated_codec_savings`, `similarity_statistics`, ...) | `src/archive/inspection.rs:403-1097` (private) | Isolate explain attribution steps | Whole `inspect`/`explain` |
| Record encoders/decoders (`encode_content_object_record`, `decode_manifest_record`, ...) | `src/ecf/records.rs` (`pub(crate)`) | Craft adversarial STREAM items with valid digests | Hand-encode after reading existing items |

Suggested flag shape: `features = ["research-internals"]` gating `pub use` of the items
above under `entrybound::research::*`, compiled out of release builds.
