# code-fs: research-harness API notes

Slice: `crates/entrybound/src/archive/filesystem.rs` and `crates/entrybound/src/archive/tooling.rs`
(dev @ 9e44608), plus the policy types they consume in `src/archive.rs`. Paths below are relative to
`crates/entrybound/src`. Everything listed as public is reachable through `entrybound::archive`
(re-exports at archive.rs:11-30). The workspace forbids `unsafe_code` (root Cargo.toml:11-12), so a
harness crate should also stay safe to get comparable platform behaviour.

## 1. Public entry points usable without modifying production code

### Filesystem capture and planning (filesystem.rs)
| Item | Signature / location | Research use |
|---|---|---|
| `archive::PackOptions` | `{ source_retries: usize, include_index: bool, profile: CompressionProfile }`, default `2 / true / Balanced` (L46-64) | Sweep profiles and retry counts. There is no budget, memory or thread knob. |
| `archive::plan_directory` | `fn(&Path, PackOptions) -> Result<eam::Archive>` (L101-108) | Scan + CDC parameter selection + `plan_archive_v6` without encoding. Identity roots are still `Digest::ZERO`: call `identity::apply_native_identities` (identity.rs:187) or `ecf::encode` to get LAI/PCR/AUX. Main hook for planner, dedup and chunking experiments on real trees. |
| `archive::pack_directory` | `fn(&Path, PackOptions) -> Result<ecf::EncodedArchive>` (L75-84) | Build INDEXED bytes. `EncodedArchive { bytes, archive, identities }`. Use it for ratio, determinism (pack twice, compare bytes) and cross-filesystem AUX-stability experiments. |
| `archive::pack_directory_stream` | `fn(&Path, PackOptions, ecf::StreamWriteOptions, W: Write) -> Result<ecf::StreamWriteSummary>` (L86-99) | STREAM pack to a sink or pipe. It has no test and no CLI caller (the CLI uses `plan_directory` + `ecf::encode_stream`), so check LAI/PCR/AUX equality against `pack_directory`. |
| `archive::replan_archive` | `fn(&eam::Archive, CompressionProfile) -> Result<eam::Archive>` (L251-371) | Rechunk and replan an already-verified EAM without touching the filesystem. Useful for profile sweeps on one corpus. Note that it emits chunker_id `entrybound/gear-cdc-v1/...`, while pack emits `gear-norm-v1/...`, so PCR differs from a fresh pack even when the chunking is identical. |
| `archive::default_pack_output`, `default_unpack_destination` | L426-448 | CLI naming conventions (usability study). |

### Extraction (filesystem.rs, policy in archive.rs)
| Item | Signature / location | Research use |
|---|---|---|
| `archive::unpack` | `fn(&[u8], &Path, ExtractionPolicy) -> Result<ExtractionReport>` (L489-503) | Full open + verify, then confined materialization. The whole plaintext stays in RAM and is cloned once, which matters for peak-memory measurements. |
| `archive::unpack_opened` | `fn(&ecf::OpenedArchive, &Path, ExtractionPolicy)` (L505-522) | Materialize an archive that has already been opened or decrypted (for example from `crypto` open functions). |
| `archive::unpack_stream` | `fn(R: Read, &Path, ExtractionPolicy, ecf::SequentialLimits) -> Result<(ExtractionReport, ecf::StreamReport)>` (L524-565) | Pipe extraction with bounded staging. Vary `SequentialLimits.staging`. `policy.budget()/decode()` override `limits.budget/decode`. |
| `archive::ExtractionPolicy` | `new`, `new_with_decode`, `with_symlinks/ownership/xattrs/sparse/acls/windows_security/reparse/platform_metadata` (archive.rs:145-310) | Policy-matrix experiments (the dangerous-restoration matrix). `CollisionPolicy` only has `Refuse`. |
| `archive::ExtractionReport` | `{ entries_created, logical_bytes_written, confinement, metadata_not_restored: Vec<String> }` (L66-73) | Fidelity-loss reporting. The loss notes are free-text strings, so a harness must parse them. `confinement` is always `KernelEnforced`. |
| `archive::bootstrap_resource_policy`, `bootstrap_decode_policy` | archive.rs:32-59 | Baseline limits to perturb. |

### Repack, diff, inspect, explain (tooling.rs)
| Item | Signature / location | Research use |
|---|---|---|
| `archive::prepare_repack` | `fn(&ecf::OpenedArchive, RepackOptions) -> Result<PreparedRepack>` (L84-200) | Representation-only versus replan, INDEXED versus STREAM, Index policy. Everything is in memory with no filesystem writes. `RepackAnalysis` (L46-74) gives chunk counts, unique chunks, stored chunk bytes, working-set bytes, dictionary/group/region counts, PCRs, output bytes and LAI/AUX/PCR equality. This is ready-made instrumentation for planner A/B comparisons. Encrypted sources are refused. |
| `archive::RepackOptions`, `RepackMode`, `IndexPolicy` | L22-44 (`stream_window: ecf::StreamWindow`) | Window sweeps for STREAM dedup. |
| `archive::archive_diff` | `fn(&OpenedArchive, &OpenedArchive) -> Result<ArchiveDiffReport>` (L369-403) | Tier-classified change sets. `PhysicalDiffSummary` (chunks reused/added/removed, objects with boundary changes) gives CDC resynchronization and incremental-update dedup measurements with no extra code. `ArchiveDiffReport::to_canonical_json` (L323-367). |
| `archive::archive_metadata_diff` | `fn(&RandomAccessMetadata, &RandomAccessVerificationReport, &RandomAccessMetadata, &RandomAccessVerificationReport)` (L405-504) | Remote metadata-only diff. The report carries `bytes_fetched` and `range_request_count` in its scope strings, and the same numbers are fields of `RandomAccessVerificationReport`. Obtain the inputs from `ecf::open_indexed_random` or `random_access`. |
| `archive::inspection_json`, `inspection_json_with_security`, `InspectionViews`, `InspectionSecurity` | L1252-1734 | Stable JSON with per-chunk records (digest, physical ordinal, logical bytes, plan id, group, stored bytes), per-plan codec parameters and reconstruction regions. A harness can parse this instead of walking internals. |
| `archive::random_inspection_json` | L1736-1784 | Range-backed inspection; access cost fields. |
| `archive::structured_explain` | `fn(&OpenedArchive, Option<&str>) -> Result<StructuredExplanation>` (L1830-2264) | Per-entry dependency estimates (`dependency_closure_logical_bytes`, `estimated_range_count`, `lookback_predecessors`, `decode_working_set_bytes`). Compare them with measured random-access fetches (tests/random_access.rs) to validate or refute the one-level lookback closure. |
| `archive::inspect`, `explain`, `list` | inspection.rs:190-285 (outside this slice) | Aggregate statistics used by `inspection_json`. |

### Supporting public APIs from other modules a harness will combine with the above
- `chunker::{chunk_ranges, select_parameters, evaluate, ChunkingParameters (all fields pub, custom parameters constructible), FAST_V2, BALANCED_V2, DENSE_V2, EXTREME_V2}`: chunk arbitrary byte slices with custom min/target/max, and evaluate archive-wide cost.
- `planner::{CompressionProfile (planner_id, chunking_candidates, lookback_candidates, similarity_policy), plan_archive_v6, analyze, zstandard_wins}`.
- `identity::{sha256_exact, build_content_from_ranges, apply_native_identities, hardlink_group_id}`: construct a synthetic `eam::Archive` without the filesystem (integration tests build `Archive` literals, e.g. tests/jpeg_whole_object.rs `archive_for`), then `plan_archive_v6` + `ecf::encode`.
- `ecf::{encode, encode_stream, open, open_with_limits, open_stream_with_limits, verify, WriteOptions, StreamWriteOptions, StreamWindow}`.

## 2. Functionality that is internal only and would need a research feature flag or a `pub` shim

| Internal item | Location | Why a harness would want it |
|---|---|---|
| `plan_observed_archive` (pub(crate)) | filesystem.rs:1049-1074 | Plan synthetic reconciled entries with `FidelityReport` + `ConversionProvenance` through the legacy-import path, without writing a temp tree. Used by every legacy adapter and by the tooling unit tests. |
| `plan_directory_encrypted`, `replan_archive_encrypted` (pub(crate)) | filesystem.rs:114-121, 123-249 | Encrypted-CDC boundary leakage and dedup-loss experiments need `EncryptedBoundaryKey` (chunker.rs:35) plus a chunker prefix. The key derivation lives inside `crypto`. `chunker::chunk_ranges_encrypted`/`select_parameters_encrypted` are pub, but a real key is only produced internally. |
| `Scan`, `scan_directory`, `capture_entry_with_probe` | filesystem.rs:836-1250 | Inject source mutations deterministically (the probe closure) to measure source-consistency retry behaviour. Today this is reachable only from the in-file unit test. |
| `capture_metadata_stable` (unix / non-unix variants) | filesystem.rs:1252-1272 | Measure false-negative mutation detection on Windows (it compares only length and mtime). |
| `bootstrap_fidelity` | filesystem.rs:1698-1776 | Enumerate declared captured/unavailable classes per platform for a FidelityReport completeness audit. Without a flag, read them from a packed archive's `archive.fidelity`. |
| `validate_symlink_policy` | filesystem.rs:1832-1876 | Fuzz and differentially test the lexical Safe-symlink rule against kernel resolution (symlink-chain escapes) without packing archives. Without a flag: build EAMs with Symlink entries, encode, and call `unpack` with default policy. |
| `materialize`, `ChunkSource`, `resolve_parent`, `write_content` | filesystem.rs:450-834, 1778-1830 | Instrument extraction ordering, sparse writing and confinement under races (for example a concurrent attacker swapping directories). |
| `decode_linux_posix_acl`, `encode_linux_posix_acl`, `linux_acl_permissions_to_canonical` | filesystem.rs:1443-1525 (linux) | Round-trip and conformance of the canonical ACL registry against Linux xattr encoding. |
| `timestamp`, `system_time`, `windows_filetime` | filesystem.rs:1329-1350, 1664-1696, 2151-2165 | Timestamp edge-case corpus (pre-epoch, FILETIME range). |
| `physical_metrics`, `semantic_changes`, `physical_changes`, `metadata_item_text` | tooling.rs:202-248, 659-1211 | Reuse the diff classifiers on unencoded EAMs (the public API requires `OpenedArchive`, so encoding and reopening first). |
| Lookback closure computation in `structured_explain` | tooling.rs:2097-2167 | Not separately exposed; parse `ExplanationFact` values instead. |
| `ecf::StagedChunks` (pub(crate)) | ecf.rs:26 | Direct staging experiments; use `unpack_stream`/`open_stream_with_limits` instead. |

## 3. Capabilities absent from this slice (no API to call; research needs new code)
- Pack with a caller-supplied resource budget, bounded memory, or streaming capture. `Scan` holds every file's plaintext in memory (filesystem.rs:836-843, 1236-1238).
- No progress, timing or allocation hooks: time and measure memory externally (for example peak RSS of a harness process).
- No VFS abstraction: platform-capture experiments need real directories on the target filesystem (ext4/xfs/btrfs/APFS/NTFS).
- No parallel capture or extraction (single-threaded scan and materialize).
- No skip-and-report mode for special files, reparse points, non-UTF-8 names or unstable sources: all of these abort the whole pack.
- No non-`Refuse` collision policy, no transactional extraction, no reparse, Windows security descriptor or platform-metadata restoration, no encrypted repack.
- The planner does not persist candidate comparisons: `structured_explain` reports `unselected_candidate_comparisons` as NOT_RECORDED (tooling.rs:2254-2259). Planner-decision research must re-run the planner, e.g. `prepare_repack` with each profile.
