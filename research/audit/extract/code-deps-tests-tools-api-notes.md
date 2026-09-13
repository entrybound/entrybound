# API notes for a research harness (deps / tests / tools slice)

Agent key: `code-deps-tests-tools`. Repo `D:/Projects/entrybound/entrybound`, branch `dev` at `9e44608`.
Scope: what a separate research-harness crate could call on `crates/entrybound` without touching production
code, and what would need a research-only feature flag. Paths are relative to the repo root. Line numbers come
from this commit.

## 0. How to link the harness

- Put the harness in its own Cargo workspace, the way `tools/crypto-vector-helper/Cargo.toml:9` does with an
  empty `[workspace]` table, and depend on `entrybound = { path = "../../crates/entrybound" }`. If you add it to
  the root `Cargo.toml:2` `members`, the root `Cargo.lock` changes. It also inherits the `unsafe_code = "forbid"`
  and clippy lints (`Cargo.toml:11-15`).
- MSRV is Rust 1.94 and the edition is 2024 (`Cargo.toml:7-8`). The toolchain floats on `stable`
  (`rust-toolchain.toml:2`).
- Neither `crates/entrybound/Cargo.toml` nor `crates/entrybound-cli/Cargo.toml` has a `[features]` table, so the
  harness always compiles every dependency. That includes `reqwest` (with tokio, hyper, quinn, and rustls on
  aws-lc-rs/aws-lc-sys, which needs a C toolchain and cmake), `ring`, `zstd-sys` (C libzstd 1.5.7), `jxl-oxide`,
  `jixel`, `preflate-rs`, and the full crypto stack. You cannot trim this without a feature flag.
- Output bytes depend on the exact pinned encoder crates: zstd-sys `2.0.16+zstd.1.5.7`, lzma-rust2 0.20.0,
  jixel 0.2.19, preflate-rs 0.7.6, and flate2 1.1.9 (zlib-rs/miniz_oxide). Record `Cargo.lock` next to every
  measurement.

## 1. Chunking / CDC experiments (public)

`entrybound::chunker` (`crates/entrybound/src/chunker.rs`):
- `chunk_ranges(&[u8], ChunkingParameters) -> Result<Box<[ChunkRange]>>` (L111). Uses the built-in gear table.
- `chunk_ranges_encrypted(&[u8], ChunkingParameters, &EncryptedBoundaryKey)` (L116).
  `EncryptedBoundaryKey::{SecretGearTable(Box<[u64;256]>), PhteAes128{polynomial,aes_key}}` (L35-38) has public
  variants, so the harness can supply arbitrary keys or tables. That supports boundary-leakage experiments and
  gear-table substitution without changing production code.
- `evaluate(&[&[u8]], ChunkingParameters) -> ChunkingEvaluation` (L287): estimated cost, unique chunks, manifest
  references, unique bytes.
- `select_parameters` / `select_parameters_encrypted` (L246 / L266): lowest estimated cost wins, and candidate
  order breaks ties.
- Constants: `FAST_V2`, `BALANCED_V2`, `DENSE_V2`, `EXTREME_V2` (L79-110); `CHUNK_FRAME_OVERHEAD_BYTES`,
  `INDEX_ENTRY_OVERHEAD_BYTES`, `CHUNK_REFERENCE_OVERHEAD_BYTES` (L19-23).
- `ChunkingParameters` has public fields (`chunker_id: &'static str`, min, target, max), so parameter sweeps are
  possible. Use `Box::leak` or consts for ids. Custom sizes still go through the private `validate_ranges`
  (L375), so check that it accepts them.
- `entrybound::identity::{build_content, build_content_from_ranges, sha256_exact, BOOTSTRAP_CHUNK_SIZE}`
  (identity.rs L23, L87-115) turn ranges into `ContentObject` plus `Chunk` maps.

## 2. Similarity and planner experiments (public, coarse)

- `entrybound::similarity::{cluster(&BTreeMap<Digest,Chunk>, SimilarityPolicy) -> Vec<SimilarityCohort>,
  SimilarityPolicy, FAST_V3_SIMILARITY..EXTREME_V3_SIMILARITY, SIMILARITY_ID}` (similarity.rs L12-109).
- `entrybound::planner::{plan_archive (v6), plan_archive_v2..plan_archive_v6, CompressionProfile, PlanningReport,
  analyze(&[u8]) -> ContentAnalysis, zstandard_wins(logical_len, encoded_len), MINIMUM_GAIN_BYTES,
  MINIMUM_GAIN_BASIS_POINTS, ZSTD_ATTRIBUTABLE_OVERHEAD_BYTES, UNPLANNED_PLAN_ID}` (planner.rs L41-1785).
  `CompressionProfile::{planner_id, planner_vN_id, chunking_candidates, similarity_policy,
  lookback_candidates, from_planner_id}` expose the frozen per-profile candidate sets.
- To evaluate a profile, build an `eam::Archive` in memory (see the test helpers `archive_for` in
  `tests/jpeg_whole_object.rs:410-460` and `tests/reconstructive_transform.rs:181-235`), call `plan_archive_vN`,
  then `ecf::encode`, then `archive::{inspect, explain}` to read the metrics: codec usage, dedup ratio, cross-file
  savings decomposition, worst random-access bytes, JPEG and DEFLATE savings, and fallback reasons.
- Not available: custom objective weights, codec levels, or candidate lists. `CompressionProfile` is a closed
  enum, and per-chunk candidate tables and cost terms are private.

## 3. Building and reading INDEXED / STREAM archives (public)

- Filesystem: `entrybound::archive::{plan_directory, pack_directory, pack_directory_stream, replan_archive,
  PackOptions{source_retries, include_index, profile}, unpack, unpack_opened, unpack_stream, ExtractionPolicy,
  CollisionPolicy, bootstrap_resource_policy, bootstrap_decode_policy}` (archive.rs L15-60,
  archive/filesystem.rs L48-533).
- In-memory INDEXED: `entrybound::ecf::{encode, WriteOptions{include_index}, open, open_with_limits,
  open_with_policy, verify, verify_with_limits, peek_layout, EncodedArchive{bytes, identities, archive},
  OpenedArchive, VerificationReport, IndexStatus}` (ecf.rs L16-20).
- STREAM: `ecf::{encode_stream<W: Write>, StreamWriteOptions{window, budget_declared}, StreamWindow::{Auto,
  Ceiling(n)}, open_stream, open_stream_with_limits, verify_stream_with_limits, SequentialLimits{content, staging,
  budget, max_container_bytes..}, StagingLimits{memory_bytes,total_bytes}, StreamContentPolicy::Retain,
  bootstrap_sequential_limits, bootstrap_staging_limits, StreamReport, StreamWriteSummary}` (ecf.rs L27-33).
  `StreamReport` exposes `spilled_staging_bytes` and `peak_resident_staging_bytes` for memory-bound experiments
  (`tests/stream_layout.rs:655-664`).
- Wire constants for mutation or fuzz harnesses: `MAGIC`, `PREAMBLE_LEN`=256, `FOOTER_LEN`=128,
  `SECTION_HEADER_LEN`=64, `CHUNK_FRAME_HEADER_LEN`=64, `CHUNK_FRAME_V2_HEADER_LEN`=96, `FEATURE_*` bits
  (ecf.rs L36-93), `STREAM_FOOTER_LEN`, `STREAM_ITEM_HEADER_LEN`, `STREAM_ITEM_MAGIC`, `StreamItemTag`.
  `ecf.rs:35` still calls the magic a "Candidate".
- Identity roots: `identity::{apply_native_identities, physical_container_identity, IdentitySet, NativeRoots,
  hardlink_group_id}`.
- Diagnostics: `entrybound::diagnostics::{ReasonCode, OutcomeClass, Diagnostic}`. Use `code()` and `class()` for
  differential or fuzz oracles.

## 4. Random / remote access (public)

- `ecf::open_indexed_random(source, RandomAccessPolicy)` (ecf/random.rs L149) returns a `RandomAccessArchive`
  with `read_entry(&LogicalPath) -> RandomAccessRead{bytes, report}` (L347). The
  `RandomAccessVerificationReport` (L74-95) has `bytes_fetched`, `range_request_count`,
  `dependency_chunk_count`, `dictionaries_verified`, `groups_verified`, `reconstruction_verified`,
  `whole_archive_verified`, and `access_trace: Box<[AccessTraceEntry{offset,length,purpose,cache_hit}]>`.
- `random_access::RandomReadSource` is a public trait (random_access.rs L38-46: `len`, `read_exact_at`,
  `revision`). A harness can write latency-injecting, RTT-simulating, or request-counting sources to evaluate
  fetch strategy without a network. Built-in sources are `MemoryRandomReadSource` (L63), `LocalFileRandomReadSource`
  (L110), and `HttpRangeSource::open(url)` (L181).
- `RandomAccessPolicy` (L388-403) has public fields, so you can sweep `coalesce_gap_bytes` (default 4096),
  `max_cached_bytes` (128 MiB), `max_individual_range_bytes` (64 MiB), and the rest. Defaults are at L405-420.
- Encrypted: `crypto::{open_indexed_random_encrypted, inspect_indexed_random_encrypted_public}`.
- Gap: `HttpRangeSource::open` takes only a URL, so there is no public client configuration (timeouts, TLS roots,
  HTTP/2 or HTTP/3 selection, proxies, custom headers). Varying those needs a feature flag or a custom
  `RandomReadSource`.

## 5. Crypto experiments (public)

- `crypto::{encrypt_archive, pack_directory_encrypted, EncryptedWriteOptions{recipients, password, padding,
  boundary, ..}, PaddingMode::{None,Bucketed,Maximum}, BoundaryMode::{SecretGearTable, PhteAes128}, XWingIdentity::
  generate, XWingRecipient::from_bytes, Unlock, EncryptedOpenOptions, CryptoPolicy, open_encrypted,
  open_encrypted_authenticated, inspect_encrypted, verify_encrypted, add_recipient, reencrypt_recipients,
  change_password, embed_signature, sign_archive, verify_signature, current_bindings, SigningKey::from_seed,
  read_detached_signature, FEATURE_*}` (crypto/mod.rs L31-58, L408).
- Size-leakage and padding experiments can compare `EncryptedArchive.bytes.len()` and `.public` across
  `PaddingMode` for crafted inputs.
- Internal only: AFK to boundary-key derivation, padding bucket computation, segment layout, and T1/HKDF label
  derivations (`crypto/wire.rs`, `crypto/container.rs`; `archive::plan_directory_encrypted` is `pub(crate)`,
  archive.rs L10-12). A harness that measures CDC boundary leakage under the real derived key needs a flag.
  Without one, use `chunk_ranges_encrypted` with a random `EncryptedBoundaryKey` as a proxy.

## 6. Legacy import / export differential harness (public)

- `legacy::import::{detect, import_strict, LegacyImportPolicy, LegacySourceFormat}` (import.rs L18-147).
- `legacy::zip::{observe, resolve, resolve_strict, import, import_strict, recover_preserved_source,
  ImportPolicy::{Strict, Compatibility(id), Preservation(id)}, CompatibilityProfileId::supported(),
  ZipImportPolicy, ZipObservation}` (zip.rs L49-928). You can replay `tools/zip-compat` cases through all four
  frozen profiles and compare against `observed-outcomes-v1.json`. No test does this today.
- `legacy::tar::{observe, resolve_strict, import_strict, looks_like_tar, TarImportPolicy}`,
  `legacy::sevenz::{observe, resolve_strict, import_strict, looks_like_sevenz}`,
  `legacy::stream::{detect, decode, WrapperImportPolicy, TransportFormat}`, `legacy::lom::*` (LOM, ConflictClass).
- `legacy::export::{prepare_export, ExportTarget, ExportReceipt, ZIP_PROFILE, TAR_PROFILE, ..}`,
  `legacy::migration::{prepare_migration, prepare_sidecar}`.

## 7. Tooling APIs (public)

- `archive::{inspect, explain, list, structured_explain, inspection_json, inspection_json_with_security,
  random_inspection_json, archive_diff, archive_metadata_diff, prepare_repack, RepackOptions, RepackMode,
  IndexPolicy}` (archive.rs L19-30).

## 8. Internal-only functionality that needs a research-only feature flag

Suggested: a non-default `research-internals` cargo feature that exposes a `entrybound::research` facade
re-exporting the items below. Keep it out of release and CLI builds.

| Need | Private location | Why a harness wants it |
|---|---|---|
| Encode or decode one payload with a given codec, level, dictionary, or prefix | `codec.rs` `encode_payload`, `decode_payload`, `*_with_dictionary`, `*_with_prefix` (L467-656); plan builders `zstd_plan`, `zstd_dictionary_plan`, `zstd_prefix_plan`, `lz4_plan`, `lzma2_plan` (L135-277); `codec_registration` (L679) | codec portfolio and level sweeps without going through the planner |
| Train Zstandard dictionaries | `codec.rs` L1069 (`zstd::dict::from_samples`) | dictionary size and sample-cap experiments |
| Structural transforms | `transform.rs` `delta8_step`, `byte_shuffle_step`, `forward_pipeline`, `inverse_pipeline`, `validate_pipeline` (L91-260) | delta/shuffle benefit measurement |
| DEFLATE reconstruction | `reconstruction.rs` `try_forward`, `verified_forward`, `inverse`, constants L9-15 (format id `preflate-rs-0.7.6/deflate-reconstruct-v1`) | success rate across real-world gzip/zip producers |
| JPEG to JXL reconstruction | `jpeg_reconstruction.rs` `verified_forward`, `inverse`, limits L12-21 | success rate and savings on real JPEG corpora |
| Canonical record encoding | `canonical.rs` `encode_sequence`, `decode_record`, `decode_record_stream` (L160-363) | independent-reader and fuzz oracles |
| ECF container and records internals | `ecf::container`, `ecf::records` (`pub(crate)`, ecf.rs L10-12) | section-level mutation corpus |
| Planner candidate tables, cost and weights, ChunkGroup and lookback search | `planner.rs` private fns | objective-weight research, explaining choices |
| Built-in gear table and table-driven chunker | `chunker.rs` `GEAR_TABLE` (L26), `chunk_ranges_with_table` (L132), `chunk_ranges_phte` (L176) | comparing gear tables. Partly reachable through `EncryptedBoundaryKey::SecretGearTable` |
| Real encrypted boundary-key and padding derivation | `crypto/wire.rs`, `crypto/container.rs`; `archive::plan_directory_encrypted` (`pub(crate)`) | CDC leakage under real keys |
| HTTP client configuration | `random_access.rs` L181-200 (`reqwest` client built internally, redirects limited to 3) | CDN, S3, and latency studies |
| Observed-archive planning | `archive::plan_observed_archive` (`pub(crate)`, archive.rs L10-12) | planning LOM-projected archives directly |

## 9. Existing test fixtures to reuse (generated, not binary)

- xorshift noise and similar-file generators: `tests/cross_file_compression.rs:328-350`,
  `tests/stream_layout.rs:1047-1075`.
- Synthetic JPEGs through the `image` dev-dependency: `tests/jpeg_whole_object.rs:371-408`. These are baseline
  JPEGs only, so real corpora are needed.
- gzip level-6 CSV-like fixture: `tests/reconstructive_transform.rs:165-179`.
- Loopback HTTP/1.1 range server with ETag and If-Match: `tests/random_access.rs:45-138`. Copy it into the
  harness for fetch-count experiments.
- In-memory ECF mutation helpers (`locate_section`, `rehash_section`, `field_value`):
  `tests/native_ecf.rs:718-782`.
