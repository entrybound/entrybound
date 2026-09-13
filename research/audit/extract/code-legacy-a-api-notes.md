# code-legacy-a API notes: ZIP and 7z legacy import

Scope: `crates/entrybound/src/legacy/zip.rs` and `crates/entrybound/src/legacy/sevenz.rs`, dev at 9e44608.
These are the calls a separate research harness crate could make with a path dependency on `crates/entrybound`,
plus the internals it cannot reach today.

## Public surface you can call from a harness (no production changes)

All of it works on a complete in-memory `&[u8]`. There is no streaming or reader-based entry point.

### ZIP (`entrybound::legacy::zip`)
- `ZipImportPolicy` has 15 `pub u64` limits and implements `Default`. Build it with `ZipImportPolicy { max_conflicts: .., ..Default::default() }` to sweep budgets.
- `observe(source: &[u8], policy) -> Result<ZipObservation>` gives the parse and LOM only, without decoding content.
  - `ZipObservation::lom() -> &LegacyArchiveObservation` has public fields `archive_fields`, `entries[].fields`, `conflicts`, plus `observation_count()` and `conflict_count(class)`.
- `resolve(observation, policy, CompressionProfile, ImportPolicy) -> Result<ZipImportResult>` and `resolve_strict(...)`.
- `import(source, policy, profile, ImportPolicy)` and `import_strict(source, policy, profile)`.
- `ImportPolicy::{Strict, Compatibility(id), Preservation(id)}`, with `compatibility_profile()`, `preserves_evidence()` and `mode()`.
- `CompatibilityProfileId::{PythonZipfile3135, JavaZipFile210121, JavaZipInputStream210121, LibarchiveBsdtar388}`, with `supported()`, `as_str()` and `FromStr`.
- `ZipImportResult { archive: eam::Archive, report: ZipConversionReport { observation, resolutions, synthesized_ancestors } }`.
- `recover_preserved_source(&Archive) -> Result<Box<[u8]>>`.
- Errors are `diagnostics::Diagnostic`, with `.code()` (ReasonCode), `.class()` (OutcomeClass) and `.detail()`.

### 7z (`entrybound::legacy::sevenz`)
- `SevenZImportPolicy` has 20 `pub u64` limits and implements `Default`.
- `looks_like_sevenz(&[u8]) -> bool` and `SIGNATURE`.
- `observe(source, policy) -> Result<SevenZObservation>`. Its `lom()` exposes observations and Refinement-only conflicts.
- `resolve_strict(observation, policy, CompressionProfile) -> Result<SevenZImportResult>`.
- `import_strict(source, policy, profile)`.
- `SevenZConversionReport { observation, resolutions, synthesized_ancestors, folder_count, solid_folder_count, coder_count, encoded_header }`.

### Format-neutral dispatch (`entrybound::legacy::import`)
- `detect(&[u8]) -> Option<LegacySourceFormat>`.
- `import_strict(source, explicit_format, entry_name, LegacyImportPolicy, profile)`. `LegacyImportPolicy` has `pub zip`, `sevenz`, `tar` and `wrapper` fields.

### Downstream native pipeline, for end-to-end experiments
- `entrybound::planner::CompressionProfile::{Fast, Balanced, Dense, Extreme}`.
- `entrybound::ecf::{encode, open, WriteOptions, EncodedArchive}` handle INDEXED. `EncodedArchive.identities` gives LAI/PCR/AUX/PCI and `.bytes` gives the encoded length.
- `entrybound::ecf::{encode_stream, open_stream, StreamWriteOptions}` handle STREAM.
- `Archive.conversion: Option<ConversionProvenance>`, `Archive.preservation: Option<LegacyPreservation>`, `Archive.descriptor.{lai,pcr,aux}`, `Archive.entry_set` and `Archive.content_store.objects` are all public fields. Use them to measure provenance size and dedup.
- `ecf::FEATURE_CONVERSION_PROVENANCE_V1` (bit 13) and `ecf::FEATURE_LEGACY_PRESERVATION_V1` (bit 14).

## Experiments that need no production changes
1. **Conflict and provenance inflation vs entry count.** Generate STORE ZIPs with 1k to 1M entries and call `observe` then `lom().conflicts.len()`. Then call `import_strict`, `encode` the result and diff the encoded size with `archive.conversion = None` (clear the feature bit as the unit test at zip.rs L3524-3528 does). This tests the suspected refusal at about 250k entries under default `max_conflicts`/`max_resolutions`/`max_conversion_record_bytes`.
2. **Peak memory.** Run `import_strict` on large ZIP/7z sources under an RSS sampler. Try solid 7z folders near 4 GiB and 7-Zip `-mx9 -md=1536m` archives.
3. **Budget sweeps.** Vary each `ZipImportPolicy`/`SevenZImportPolicy` field against a real-writer corpus and record refusal `ReasonCode`s and outcome classes.
4. **Compat matrix conformance.** For each case in `tools/zip-compat/observed-outcomes-v1.json`, regenerate the bytes and run `import(.., ImportPolicy::Compatibility(id))`. Compare the selected path, content SHA-256 and error class with the matrix. No Rust test does this today.
5. **Compat safety.** Build ZIPs with Unix symlink/FIFO external attributes, local compressed size larger than central, duplicate names with the central directory reordered relative to local order, and declared-small/actual-large DEFLATE. Run them through all 4 profiles and check that each is refused.
6. **Preserve overhead.** Compare the encoded size of `Compatibility(id)` against `Preservation(id)` results. Check `recover_preserved_source` round-trips through INDEXED and STREAM.
7. **7z differential tests.** Import archives made by 7-Zip, p7zip, py7zr, sevenz-rust2 and libarchive (BCJ/Delta+LZMA2, odd tail sizes, no CRCs, Windows high attribute bits, backslash names). Compare restored SHA-256 with 7-Zip. Use `report.folder_count`, `solid_folder_count` and `coder_count` for stratification.
8. **Fuzzing.** Run cargo-fuzz or libFuzzer targets over `zip::observe` + `zip::resolve` (all 5 policies) and `sevenz::observe` + `sevenz::resolve_strict`. Every error is a `Diagnostic`, so assert on panics and timeouts only.

## Internal-only functionality (a research-only feature flag or `pub(crate)` exposure would be needed)
- **ZIP structural claims:** `HeaderClaims`, `DescriptorClaims`, `ObservedZipEntry`, `ExtraField`, `parse_central`, `parse_local`, `parse_descriptor`, `zip64_values`, `find_eocd`, `classify_extent_conflicts`. These are needed to inspect selected extents or per-entry data offsets directly.
- **ZIP compat rule table:** `CompatibilityRules` and `compatibility_rules(profile)`. They would allow table-driven comparison against the observed matrix without running full imports.
- **ZIP decoders and name handling:** `decode_entry`, `decode_entry_compat`, `decode_cp437`, `parse_unicode_extra`, `logical_path`, `resolve_mtime`, `parse_ntfs_mtime`, `entry_kind`, `unix_mode`.
- **ZIP refusal and fidelity helpers:** `refuse_unresolved_conflicts`, `refuse_irreconcilable_conflicts`, `refuse_unmodeled_compatibility_conflicts`, `preserve_observation`, `zip_fidelity`.
- **7z structures:** `StreamsInfo`, `Folder`, `Coder`, `BindPair`, `Substream`, `ObservedFile`, `ByteCursor`. The folder graph, pack sizes and CRC presence are not observable except through coarse LOM fields.
- **7z decoders and validators:** `decode_streams`, `decode_folder`, `decode_core` (per-codec decode with exact-consumption checks), `apply_filter` (Delta/BCJ in place), `validate_folder`, `folder_order`, `validate_coder_properties`, `lzma2_dictionary`.
- **7z projection helpers:** `logical_path`, `filetime_timestamp`, `executable_from_attributes`, `reject_unsupported_attribute_kind`, `sevenz_fidelity`.
- **Shared crate-private hooks:** `crate::archive::plan_observed_archive` (`pub(crate)`, archive/filesystem.rs:1053) and `crate::ecf::encoded_legacy_evidence_len` (`pub(crate)`, ecf.rs:107). The first would let a harness plan synthetic entry sets without a foreign parser; the second measures provenance bytes directly.
- **Unit-test fixture builders** are `#[cfg(test)]` and not reachable: `zip()` and `zip64_one()` in zip.rs, `plain_copy_archive()` and `encoded_copy_archive()` in sevenz.rs. A harness must reimplement them or they must move to a `research`/`test-support` feature.

Suggested flag: a `research-internals` cargo feature that re-exports the items above under `entrybound::legacy::{zip,sevenz}::internals`, plus `plan_observed_archive` and `encoded_legacy_evidence_len`. Leave it off by default so production API stability is unaffected.
