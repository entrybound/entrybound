# code-legacy-b API notes: legacy tar / transport / export / migration

Scope: `crates/entrybound/src/legacy/{tar,stream,import,export,migration,lom,mod}.rs` at dev 9e44608.
Audience: authors of a research harness crate that takes a path dependency on
`crates/entrybound` and runs experiments without modifying production code.

All paths below are `entrybound::...` public items unless marked internal.

## 1. Detection and strict import (reachable today)

| Item | Signature / use | Experiments it enables |
|---|---|---|
| `legacy::import::detect(&[u8]) -> Option<LegacySourceFormat>` | Byte-level format sniffing (ZIP, 7z, gzip, zstd, xz, bzip2, tar, trailing EOCD) | Misdetection/false-positive rate over a corpus; tar-names-starting-with-magic fixtures |
| `legacy::import::import_strict(source, explicit_format, entry_name, LegacyImportPolicy, CompressionProfile) -> Result<LegacyImportResult>` | Full dispatch incl. wrapper + tar composition and standalone streams | Corpus acceptance matrix (outcome + `Diagnostic::code()` reason), identity comparison (LAI/PCR/AUX) bare vs wrapped |
| `legacy::import::{LegacySourceFormat, LegacyImportPolicy, LegacyConversionReport, LegacyImportResult}` | `LegacySourceFormat: FromStr` (exact tokens only); policy is `Copy + Default` with pub sub-policies | Sweep limits (`policy.tar.max_observations`, `policy.wrapper.max_expansion_ratio_milli`, ...) to find binding constraints |
| `legacy::tar::observe(&[u8], TarImportPolicy) -> Result<TarObservation>` | Observation only, no EAM projection | Measure LOM size/observation counts, conflict classification, time per entry without planning cost |
| `legacy::tar::TarObservation::lom(&self) -> &LegacyArchiveObservation` | Read the immutable LOM | Count fields per entry (default 19/header), inspect conflicts/resolutions |
| `legacy::tar::TarObservation::attach_transport(&mut self, &DecodedTransport)` | Layer a decoded transport onto a tar observation | Reproduce layered provenance manually |
| `legacy::tar::resolve_strict(TarObservation, TarImportPolicy, CompressionProfile) -> Result<TarImportResult>` | Strict reconciliation + native planning | Separate observe vs resolve+plan cost; compare profiles (Fast/Balanced/Dense/Extreme) on tar-derived EAMs |
| `legacy::tar::import_strict(&[u8], TarImportPolicy, CompressionProfile)` | observe + resolve_strict | Bare tar benchmarks |
| `legacy::tar::looks_like_tar(&[u8]) -> bool` | Dispatch preflight | All-zero payload misclassification experiments |
| `legacy::tar::{TarImportPolicy, TarImportResult, TarConversionReport}` | pub fields (`report.resolutions`, `report.synthesized_ancestors`, `report.layers`) | Provenance/resolution statistics |
| `legacy::stream::decode(&[u8], TransportFormat, WrapperImportPolicy) -> Result<DecodedTransport>` | Verified transport decode (gzip members, zstd frames incl. skippable, xz streams, bzip2 members) | Decoder throughput/memory, ratio-limit false refusals, ISIZE >4 GiB defect, zstd no-checksum / xz check=None acceptance, window/memory limits |
| `legacy::stream::detect(&[u8]) -> Option<TransportFormat>` | Transport magic only | Magic collision tests |
| `legacy::stream::{TransportFormat, WrapperImportPolicy, DecodedTransport}` | `DecodedTransport { format, decoded, decoded_digest, member_count, observation }` all pub | Evidence/observation inspection |
| `legacy::lom::*` (re-exported from `legacy`) | `LegacyArchiveObservation::{observation_count, conflict_count}`, `ConflictClass::as_str` | Cross-adapter LOM statistics |

Synthetic-archive trick: because `archive::plan_observed_archive` is `pub(crate)`,
a harness can still build arbitrary regular-file/directory/symlink/hardlink EAMs
without the filesystem by synthesizing tar bytes in the harness and calling
`legacy::tar::import_strict`. Note this always attaches ConversionProvenance and
posix.mode/uid/gid metadata (which makes every later legacy export LOSSY).
Filesystem-backed alternatives: `archive::plan_directory(&Path, PackOptions)`,
`archive::replan_archive(&Archive, CompressionProfile)`.

## 2. Export (reachable today)

| Item | Signature / use | Experiments |
|---|---|---|
| `legacy::export::prepare_export(&Archive, ExportTarget, ExportSourceSecurity) -> Result<PreparedExport>` | Validate, analyze, encode, compress, self-validate wrapper, strict re-import | Outcome distribution (LOSSLESS/LOSSY/REFUSED) over EAM corpora; export time/memory incl. re-import overhead; failure modes at default policy limits (ratio, 4 GiB, observation cap) |
| `PreparedExport.analysis: ExportAnalysis` (pub field) | `outcome`, `issues`, `planned_target_bytes`, `planned_target_digest`, `strict_reimport_validated`, `reimport_lai` | Issue-category histograms without writing files |
| `PreparedExport::accept(self, allow_lossy: bool) -> Result<ExportArtifact>` | Only path to target bytes (`bytes` field is private) | Obtain bytes for extraction matrices (Windows Explorer, 7-Zip, bsdtar, GNU tar, Python) |
| `ExportArtifact { bytes, receipt }`, `ExportReceipt::to_canonical_json()` | Receipt v1/v2 | Canonical JSON validation vs RFC 8785; cross-platform byte determinism (golden hashes) |
| `ExportTarget` (`FromStr`, `profile_id`, `format`, `semantic_target`, `transport_profile`, `extension`), `ExportProfileId::parse/target` | Profile selection | Alias vs versioned-id behavior |
| `ExportOutcome`, `ExportIssue`, `ExportIssueCategory`, `ExportDisposition`, `ExportSourceSecurity`, `AuxiliaryEvidenceSummary`, `WrappedTargetReceipt` | Typed results | Stable-string conformance checks |
| constants `EXPORTER_ID`, `RECEIPT_FORMAT`, `RECEIPT_V2_FORMAT`, `ZIP_PROFILE`, `TAR_PROFILE`, `TAR_*_PROFILE` | Frozen ids | Registry audits |

Supporting native APIs used by export/migration experiments:
`ecf::{encode, open, encode_stream, open_stream_with_limits, WriteOptions,
StreamWriteOptions, SequentialLimits, StreamContentPolicy, bootstrap_sequential_limits}`,
`identity::sha256_exact`, `planner::CompressionProfile`, and `eam::Archive`
(pub fields `descriptor`, `entry_set`, `content_store`, `fidelity`, `conversion`,
`preservation`; `validate()`, `total_logical_size()`).

## 3. Migration and sidecars (reachable today)

| Item | Use | Experiments |
|---|---|---|
| `legacy::migration::prepare_migration(&Archive, &[ExportTarget], ExportSourceSecurity, allow_lossy) -> Result<PreparedMigration>` | Aggregate multi-target preflight (all-or-nothing) | Peak memory of multi-target publish (all artifacts held in memory); ordering/duplicate invariance |
| `PreparedMigration { report, artifacts }`, `is_ready()`, `mark_published()` | Report + artifact bytes | Report semantics (e.g., `lossy_approved` on non-LOSSY targets) |
| `MigrationReportV1::to_canonical_json()`, `MIGRATION_REPORT_FORMAT`, `MIGRATION_REPORT_VERSION`, `MigrationOutcome` | Canonical report | JSON canonicalization checks |
| `legacy::migration::prepare_sidecar(&LegacyImportResult, exact_source: &[u8], eam::Layout) -> Result<PreparedSidecar>` | Encode INDEXED/STREAM, reopen, bind provenance digest | Sidecar size/overhead vs source, INDEXED vs STREAM sidecar cost, verification time |

## 4. Internal-only functionality that would need a research-only feature flag

These are private or `pub(crate)`; measuring them in isolation requires either
a `research` cargo feature that re-exports them or `#[cfg(feature = "research")] pub` shims:

- `archive::plan_observed_archive` (`pub(crate)`, archive/filesystem.rs:1053): build an Archive from arbitrary `Entry` + file bytes + FidelityReport + ConversionProvenance without tar synthesis or filesystem capture. Needed for clean export experiments on EAMs that lack tar-induced POSIX metadata.
- Export encoders without strict re-import: `export::encode_target`, `encode_zip`, `encode_tar`, `encode_{gzip,zstd,xz,bzip2}_wrapper` (private). Needed to separate encoder cost from validation cost, and to generate targets that exceed default import limits (large members, >1000:1 ratio) without the validation error.
- Export with caller-supplied import/wrapper policy for self-validation and re-import (`validate_strict_reimport` and `encode_target` hard-code `LegacyImportPolicy::default()` / `WrapperImportPolicy::default()`, export.rs:1260, 1363-1369).
- `export::analyze_entries` / `analyze_path` / `zip_portable_component` (private): run representability analysis alone over large synthetic name sets (portability/collision research) without encoding bytes.
- `export::pax_timestamp`, `split_ustar_path`, `pax_payload`, `zip_timestamp`, `dos_timestamp`, `civil_from_days` (private): golden-vector and property tests for timestamp/pax framing.
- Tar internals: `parse_pax`, `parse_pax_time`, `classify_overrides`, `resolve_entry`, `checksum_valid`, `signed_number`, `logical_path` (private): unit-level fuzzing and property testing (e.g., './' normalization alternatives, old-GNU header dispatch) and per-stage timing. `TarObservation` entries (`ObservedTarEntry`) are private; only the LOM is visible.
- Per-transport decoders `decode_gzip/zstd/xz/bzip2` and limit helpers `remaining_decoded_limit`, `enforce_decoded_limits`, `maximum_window_log` (private): instrumented decoding (bytes consumed per member, peak buffer size) would need hooks or a feature-gated metrics callback.
- No streaming/`Read`-based entry points exist for legacy import or export; streaming experiments require new code, not just visibility changes.
- Fuzz harnesses: no `fuzz/` crate exists; `legacy::tar::observe`, `legacy::stream::decode` and `legacy::import::import_strict` are already public and suitable as cargo-fuzz targets without a feature flag.
