# code-model: research-harness API notes

Scope: `crates/entrybound/src/{lib.rs, eam/*.rs, canonical.rs, identity.rs, diagnostics.rs}` at dev 9e44608.
A research harness crate can depend on `entrybound = { path = "../../crates/entrybound" }`.
Everything below is callable without modifying production code unless marked **internal**.

## 1. Building EAM models directly (no filesystem)

| Item | Path | Use in experiments |
|---|---|---|
| `LogicalPath::from_utf8(iter)`, `LogicalPath::new(Vec<PathComponent>)`, `PathComponent::new(bytes, PathEncoding::Utf8)` | `entrybound::eam` | Build paths. Test P3/P4/P6/P7 rejection behavior. `Ord` impl gives canonical order (length-then-bytes per component). |
| `Entry::new(path, EntryData, MetadataSet, EntryIdentity::default())` | `entrybound::eam` | Build entries. Digests are placeholders that `apply_native_identities` recomputes. |
| `EntryData::{Directory, File{content: ContentRef::Internal(d)}, Symlink{target}, ReparsePoint{value}}` | `entrybound::eam` | Only these kinds exist. No special files, no External refs. |
| `EntrySet::new(Vec<Entry>)` / `EntrySet::from_canonical` | `entrybound::eam` | Canonical sort plus P4/P6 validation. |
| `MetadataItem::{executable, mtime, posix_mode, posix_uid, posix_gid, hardlink_group, xattrs, sparse_map, acls, windows_security_descriptor, windows_file_attributes, windows_creation_time, windows_reparse_original, macos_flags, macos_birthtime}` | `entrybound::eam` | Closed 15-name registry. Every item is `Optional`. `Critical` cannot be constructed. |
| `MetadataSet::new(Vec<MetadataItem>)`, `MetadataSet::with_item` | `entrybound::eam` | Canonical name-sorted set. |
| `Timestamp::new(secs, nanos, precision, restorable)`, `LinkTarget::canonical(bytes)`, `XAttr::new`, `SparseMap::new`, `Acl::new`, `AclEntry::new`, `WindowsSecurityDescriptor::new`, `WindowsReparsePoint::new` | `entrybound::eam` | Validated constructors. Good fuzz and property targets. `WindowsSecurityDescriptor::new` is a full binary parser. |
| `hardlink_group_id(content, &[LogicalPath])` | `entrybound::identity` | Canonical, inode-independent group id. |
| `Archive { descriptor: ArchiveDescriptor{..}, entry_set, content_store: ContentStore{..}, transform_plans, fidelity, conversion, preservation, index }` | `entrybound::eam` | All fields are `pub`, so a harness can assemble archives by hand. See `crates/entrybound/tests/native_ecf.rs:419-578` (`empty_archive`, `complete_fixture`, `descriptor`, `store_plan`) for a working template. |
| `Archive::validate()`, `Archive::validate_without_retained_plaintext()`, `Archive::total_logical_size()` | `entrybound::eam` | Run model invariants alone and time them. |

## 2. Content and identity construction

| Item | Path | Use |
|---|---|---|
| `build_content(plaintext, chunk_size, plan_ref)` | `entrybound::identity` | Fixed-size chunking to `(ContentObject, BTreeMap<Digest, Chunk>)`. |
| `build_content_from_ranges(plaintext, &[Range<usize>], plan_ref)` | `entrybound::identity` | Build content from arbitrary boundaries, for example from `chunker::chunk_ranges`. Lets you compare chunkers under identical identity code. |
| `BOOTSTRAP_CHUNK_SIZE`, `STORE_PLAN_ID`, `STORE_PLAN_IDENTIFIER`, `STORE_CODEC_IDENTIFIER` | `entrybound::identity` | Constants for a minimal store-only `TransformPlan`. |
| `apply_native_identities(&Archive) -> (Archive, NativeRoots)` | `entrybound::identity` | Compute LAI/PCR/AUX without serializing. Use it for identity-invariance experiments: re-chunking, metadata perturbation, layout, and canonicalization of fidelity/conversion lists. |
| `sha256_exact`, `physical_container_identity(bytes)` | `entrybound::identity` | Content digest and PCI. |
| `LogicalArchiveIdentity`, `PhysicalContentRoot`, `AuxiliaryRoot`, `PhysicalContainerIdentity`, `IdentitySet`, `NativeRoots` | `entrybound::identity` | Result types. |
| `VerificationState` | `entrybound::identity` | Public but unused anywhere. Do not rely on it. |

## 3. Serialization, open, verify (adjacent slices, public)

- `entrybound::ecf::{encode(&Archive, WriteOptions{include_index}), open, open_with_policy, open_with_limits, verify, verify_with_policy, verify_with_limits, peek_layout}`. `EncodedArchive{bytes, archive, identities}`. `VerificationReport` has per-check booleans plus `IndexStatus`/`index_reason`.
- STREAM: `ecf::{encode_stream(&Archive, StreamWriteOptions, impl Write), open_stream, open_stream_with_limits(impl Read, SequentialLimits), verify_stream, bootstrap_sequential_limits, bootstrap_staging_limits}`.
- Random access: `ecf::open_indexed_random(impl random_access::RandomReadSource, random_access::RandomAccessPolicy)` with `random_access::{MemoryRandomReadSource, LocalFileRandomReadSource, HttpRangeSource}`. The `RandomReadSource` trait is public, so a harness can wrap a source to count or trace range requests.
- Filesystem: `archive::{pack_directory, pack_directory_stream, plan_directory, replan_archive, unpack, unpack_opened, unpack_stream, inspect, list, explain, archive_diff, prepare_repack, inspection_json, structured_explain}`, `archive::{bootstrap_resource_policy, bootstrap_decode_policy, ExtractionPolicy}`.
- Chunking and planning: `chunker::{chunk_ranges, evaluate, select_parameters, ChunkingParameters, FAST_V2..EXTREME_V2}`, `planner::{plan_archive(_v2..v6), analyze, zstandard_wins, CompressionProfile}`, `similarity::{cluster, SimilarityPolicy, *_V3_SIMILARITY}`.

Typical harness loop for model experiments: build or plan an `Archive`, call `identity::apply_native_identities`, then `ecf::encode`, then `ecf::open`/`verify`, and compare `identities` and the `VerificationReport`.

## 4. Diagnostics

- `entrybound::diagnostics::{Diagnostic, OutcomeClass, ReasonCode, Result}`. Read errors with `Diagnostic::class()`, `code()`, `code().as_str()` (`EB_*`) and `detail()`.
- A harness can enumerate observed codes to build an observed-outcome matrix. `ReasonCode` has no iterator. A harness needs its own list, or a research feature could add `ReasonCode::ALL`.
- 27 codes are emitted but never asserted by any test. They are listed in `code-model.jsonl` as `CLAIM_NEEDING_EVIDENCE` records.

## 5. Internal-only: needs a research feature flag

| Capability | Location | Why a harness needs it |
|---|---|---|
| Canonical TLV record builder and decoder (`RecordBuilder`, `decode_record`, `decode_record_stream`, `encode_sequence`, `FieldType`) | `canonical.rs` (module private, items `pub(crate)`) | Manifest-size and overhead experiments per record type. Differential testing against an independent reader. Fuzzing the record decoder directly. |
| Record encoders and decoders per object (`encode_transform_plans*`, `encode_conversion`, `decode_entry`, `decode_descriptor`, ...) | `ecf/records.rs` (`pub(crate)`) | Per-object byte cost. Targeted mutation corpora. |
| Structured hash and Merkle internals (`structured_hash`, `merkle_root`, `chunk_root_from_leaves`, `entry_identity_digest`, `entry_aux_digest`, `verify_metadata_lai`, `verify_metadata_aux`, `native_identities_from_verified`) | `identity.rs` (private or `pub(crate)`) | Merkle proof-size and verified-range experiments. Hashing throughput per domain. Checking identity vectors independently. No inclusion-proof or outboard API exists, so this would be new research-only code. |
| Codec encode and decode (`encode_payload*`, `decode_payload*`, `zstd_plan`, `lz4_plan`, `lzma2_plan`, `train_dictionary`, `aggregate_decode_requirements`) | `codec.rs` (module private) | Codec-portfolio and dictionary experiments on raw buffers without building archives. |
| Transforms and reconstruction (`transform.rs`, `reconstruction.rs`, `jpeg_reconstruction.rs` constants such as `MAX_REGION_CHUNKS`, `MAX_JXL_BYTES`) | module private | Structural transforms (delta, BCJ) and JPEG/DEFLATE reconstruction studies in isolation. |
| `Entry::replace_metadata`, `LogicalPath::prefix`, `EntrySet.entries` field | `eam` (`pub(crate)`) | Minor. Harnesses can rebuild entries instead. |
| Constructing `Criticality::Critical` metadata or unknown metadata names | `MetadataItem` fields private, registry closed | Needed to test fail-closed behavior for unknown Critical items. This cannot be done today without editing code. |

Suggested shape: a `research-internals` cargo feature that re-exports the items above from a `#[doc(hidden)] pub mod research` module, never enabled by the CLI.
