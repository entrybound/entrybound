# code-cli: research-harness API notes

Slice: `crates/entrybound-cli/src/lib.rs`, `src/main.rs`, `src/bin/entrybound.rs`,
`tests/cli_workflow.rs`, `tests/crypto_cli.rs` (dev @ 9e44608).
Core paths below are relative to `crates/entrybound/src` unless prefixed.

## 0. What the CLI crate itself exposes

- `entrybound_cli::main_entry() -> ExitCode` (lib.rs:4974-4991) is the **only** public item.
  It reads `std::env::args_os()`, so it can't be driven in-process with custom arguments.
- `run(arguments)` (lib.rs:276) and every `command_*` / helper function are private.
- A harness that wants CLI-level measurements has two options today:
  1. **Spawn the binaries**: `ebound` or `entrybound` (same implementation), as
     `tests/cli_workflow.rs` does. Useful for exit-code, stderr/stdout routing, pipe and
     wall-clock/RSS experiments. Exit codes: 0 ok, 2 Unsupported, 3 Truncated, 4 Corrupt,
     5 Nonconforming, 6 PolicyRefused.
  2. **Call the library functions the CLI composes** (section 1). This covers almost every CLI
     behaviour without the CLI's argument parsing.

## 1. Public library entry points that reproduce each CLI command

| CLI command | Library composition (all public) | Research use |
|---|---|---|
| `pack` (plain) | `archive::plan_directory(&Path, PackOptions) -> Result<eam::Archive>` (archive/filesystem.rs:106); then `ecf::encode(&Archive, WriteOptions{include_index})` (ecf/container.rs:105) or `ecf::encode_stream(&Archive, StreamWriteOptions{window,..}, W) -> StreamWriteSummary` (ecf/stream.rs:625). Shortcuts: `archive::pack_directory`, `archive::pack_directory_stream`. | Profile comparison (`PackOptions.profile` = `planner::CompressionProfile::{Fast,Balanced,Dense,Extreme}`), Index on/off overhead, STREAM window requirement (`StreamWindow::Ceiling(n)` vs `Auto`), how often window 0 is refused, determinism across runs/threads (compare bytes/PCI). |
| `pack` (encrypted) | `crypto::pack_directory_encrypted(&Path, PackOptions, EncryptedWriteOptions)` (crypto/mod.rs:408) or `crypto::encrypt_archive(&Archive, EncryptedWriteOptions)` (crypto/container.rs:186). Options: `recipients: &[XWingRecipient]`, `password`, `padding: PaddingMode::{None,Bucketed,Maximum}`, `boundary: BoundaryMode::{SecretGearTable,PhteAes128}`, `include_index`, `embedded_signatures`. Keys: `crypto::XWingIdentity::generate()`. | Padding overhead vs privacy class, boundary-mode dedup loss, public framing leakage (`crypto::inspect_encrypted`). |
| `convert` (import) | `legacy::import::detect`, `legacy::import::import_strict(bytes, Option<LegacySourceFormat>, Option<&str>, LegacyImportPolicy, CompressionProfile)` (legacy/import.rs:147); ZIP compat/preserve via `legacy::zip::import(bytes, ZipImportPolicy, CompressionProfile, ImportPolicy)` (legacy/zip.rs:918) with `CompatibilityProfileId`; `legacy::zip::recover_preserved_source`. | Adapter conformance corpus, conflict-class counts (`archive.conversion`), preservation overhead, compat-profile outcome matrix. |
| `convert --to` (export) | `legacy::export::prepare_export(&Archive, ExportTarget, ExportSourceSecurity)` (legacy/export.rs:406) -> `.analysis` (LOSSLESS/LOSSY/REFUSED + issues) and `.accept(allow_lossy)` -> artifact bytes + receipt (`receipt.to_canonical_json()`). | LOSSY-rate survey over real trees, export byte stability, re-import round trips. |
| `publish` | `legacy::migration::prepare_migration(&Archive, &[ExportTarget], ExportSourceSecurity, allow_lossy)` (legacy/migration.rs:209); `identity::apply_native_identities` for directory sources. | Target-order independence, report determinism. Note the transactional hard-link commit (`transactional_publish`, lib.rs:1600) is CLI-private. |
| `sidecar` | `legacy::migration::prepare_sidecar(&LegacyImportResult, exact_source, Layout)` (legacy/migration.rs:282); `identity::sha256_exact`. | Provenance binding checks, sidecar size overhead. |
| `unpack` | `archive::unpack(bytes, &Path, ExtractionPolicy)` (archive/filesystem.rs:490), `archive::unpack_stream(R, &Path, ExtractionPolicy, SequentialLimits)` (L533) -> `(ExtractionReport, StreamReport)`, `archive::unpack_opened` (encrypted after `crypto::open_encrypted`). Policy builders `ExtractionPolicy::default().with_symlinks/with_ownership/with_xattrs/with_sparse/with_acls/with_windows_security/with_reparse/with_platform_metadata`; `ExtractionPolicy::new(CollisionPolicy, ResourceBudget)`. | Confinement mode per platform (`report.confinement`), metadata restoration gaps (`metadata_not_restored`), STREAM staging peak/spill (`StreamReport.peak_resident_staging_bytes`, `spilled_staging_bytes`) under custom `ecf::StagingLimits`. |
| `read` | `ecf::open_indexed_random(source, RandomAccessPolicy)` (ecf/random.rs:149) -> `RandomAccessArchive::read_entry(&LogicalPath)` -> bytes + `RandomAccessVerificationReport` (access trace, ranges, bytes fetched); encrypted: `crypto::open_indexed_random_encrypted`. Sources: `random_access::LocalFileRandomReadSource::open`, `random_access::HttpRangeSource::open(url)`, or a custom `RandomReadSource` impl (latency/fault injection). | Range count / bytes-fetched vs archive shape, coalescing gap tuning (`RandomAccessPolicy.coalesce_gap_bytes`), cache sizing, dependency-chunk cost of ChunkGroups. |
| `list` | `archive::list(&Archive)`; remote: `RandomAccessArchive::metadata()` + `metadata_report()`. | Metadata-only fetch cost. |
| `repack` | `archive::prepare_repack(&OpenedArchive, RepackOptions{mode: RepackMode::{RepresentationOnly, Replan(profile)}, layout, index: IndexPolicy, stream_window})` (archive/tooling.rs:85) -> `.analysis` + `.encoded.bytes`. Also `archive::replan_archive`. | Identity-root preservation (LAI/AUX/PCR equality flags), layout conversion cost, no-op byte reproduction. |
| `diff` | `archive::archive_diff(&OpenedArchive, &OpenedArchive)` (tooling.rs:370), `archive::archive_metadata_diff(...)` (tooling.rs:407); `ArchiveDiffReport::to_canonical_json`. | Tier separation experiments (same content, different profile/layout). |
| `inspect` | `archive::inspect(&OpenedArchive)` (archive/inspection.rs:213), `archive::inspection_json(opened, InspectionViews)`, `inspection_json_with_security`, `random_inspection_json`; `crypto::inspect_encrypted(bytes, None, CryptoPolicy)`, `crypto::inspect_indexed_random_encrypted_public`. | Machine-readable statistics collection over a corpus (chunk stats, codec usage, reconstruction, declared resources). |
| `explain` | `archive::explain(&OpenedArchive) -> CompressionExplanation` (inspection.rs:285), `archive::structured_explain(&OpenedArchive, Option<&str>)` (tooling.rs:1831). | Planner decision audit; replayed NOT_RECORDED savings estimates. |
| `verify` | `ecf::open`/`open_with_limits`/`verify_with_limits`; `ecf::open_stream_with_limits(R, SequentialLimits)`; `crypto::open_encrypted_authenticated`; signatures `crypto::current_bindings`, `crypto::verify_signature`, `crypto::read_detached_signature`, `SignaturePolicy::enforce`. | Corruption-detection matrices (bit flips/truncation -> OutcomeClass/ReasonCode), limit-threshold refusal experiments. |
| `sign` | `crypto::SigningKey::generate/read_file`, `crypto::sign_archive(&CurrentBindings, &SigningKey, bind_physical, bind_addressing)` (crypto/signature.rs:366), `SignatureRecord::with_timestamp_token`, `crypto::embed_signature`. | Content-only vs content+physical signature survival across repack (library allows `bind_physical=false`; the CLI can't). |
| `key add/remove/change-password` | `crypto::add_recipient`, `crypto::reencrypt_recipients`, `crypto::change_password` (crypto/container.rs:762/830/867); each verifies its output internally. | Rekey cost vs archive size; signature staleness after mutation. |

Supporting public knobs: `archive::bootstrap_resource_policy()`, `archive::bootstrap_decode_policy()`,
`ecf::bootstrap_sequential_limits()`, `ecf::bootstrap_staging_limits()`, `RandomAccessPolicy::default()`,
`ecf::peek_layout`, `ecf::MAGIC`, feature constants `crypto::FEATURE_ENCRYPTED_INDEXED_V1`,
`crypto::FEATURE_PRIVATE_RESOURCE_DECLARATION_V1`. Lower-level public modules also reachable:
`chunker::{chunk_ranges, evaluate, select_parameters}`, `planner::{plan_archive_v6, analyze}`,
`similarity::cluster`, `identity::{sha256_exact, apply_native_identities}`.

## 2. CLI-only behaviour that would need either spawning or a research feature flag

These are private to `entrybound-cli` and have no library equivalent. Reproducing them in-process
would need a research-only `pub` shim (for example a `research` cargo feature exporting `run` and helpers):

- `run(args)`: in-process argument parsing, option-validation matrix and usage diagnostics (lib.rs:276-312).
- `transactional_publish` / `temporary_sibling` / `validate_transaction_paths`: the temp + hard-link
  multi-artifact commit and rollback (lib.rs:1564-1673). Needed for crash-injection and filesystem
  compatibility (exFAT/SMB/NFS) experiments.
- `publish_staged_file`: repack's staged write, re-verify and hard-link (lib.rs:3476-3522).
- `replace_verified`: in-place archive replacement using backup renames (lib.rs:2936-2972).
- `write_export_transaction`: export target + receipt write/rollback (lib.rs:1114-1159).
- `read_stable_source`: length+mtime source stability check (lib.rs:1911-1935).
- `path_is_encrypted` / `path_layout` / `load`: preamble sniffing and reader routing, including the
  stdin->STREAM-reader routing that refuses INDEXED input (lib.rs:478-546).
- `open_export_source`: encrypted-source security summary (signature valid/invalid/stale counts) (lib.rs:1031-1082).
- `public_crypto_diff` and `append_security_diff`: the Container-tier security diff fields (lib.rs:3683-3891).
- `prompt_password`: terminal-only password entry via rpassword. Any automated password-archive
  experiment at the CLI level needs a pty, or a feature-gated non-interactive password source (lib.rs:468-476).
- Hand-formatted locked-encrypted inspection JSON (lib.rs:3965-3972).

Library-internal (private core modules `codec`, `transform`, `canonical`, `reconstruction`,
`jpeg_reconstruction`) aren't reachable from the CLI or from a harness. Codec-level or
transform-level experiments (encode one chunk with a given codec/level, evaluate one structural
transform) would need a research feature in `crates/entrybound`, not in the CLI crate.

## 3. Caveats for harness authors

- Every CLI path reads INDEXED archives fully into memory (`std::fs::read`). For large-archive
  memory experiments, use `open_indexed_random` with `LocalFileRandomReadSource`, or the STREAM reader.
- Default STREAM window is `Ceiling(0)` for pack, convert and publish, but `Auto` for repack. Set it
  explicitly in experiments.
- CLI limits are the bootstrap policies documented as "deliberately generous". Pass explicit
  `ResourceBudget`, `DecodeRequirements`, `StagingLimits` and `RandomAccessPolicy` when measuring
  refusal thresholds.
