//! Independent bounded ZIP observation and strict reconciliation.
//!
//! This parser deliberately does not use a ZIP archive library: central,
//! local, and data-descriptor claims remain separate observations until the
//! strict resolver has classified them.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;

use crc32fast::Hasher as Crc32;
use flate2::read::DeflateDecoder;

use super::lom::{
    ConflictClass, LegacyArchiveObservation, LegacyAuthority, LegacyConflict,
    LegacyEntryObservation, LegacyEvidenceLocation, LegacyFieldObservation, LegacyObservedValue,
    LegacyResolution, ObservationValidity,
};
use crate::archive::plan_observed_archive;
use crate::diagnostics::{Diagnostic, OutcomeClass, ReasonCode, Result};
use crate::eam::{
    Archive, ContentRef, ConversionProvenance, ConversionResolution, Entry, EntryData,
    EntryIdentity, FidelityIssue, FidelityReport, LogicalPath, MetadataItem, MetadataSet,
    Timestamp, TimestampPrecision,
};
use crate::identity::sha256_exact;
use crate::planner::CompressionProfile;

const EOCD_SIGNATURE: u32 = 0x0605_4b50;
const ZIP64_LOCATOR_SIGNATURE: u32 = 0x0706_4b50;
const ZIP64_EOCD_SIGNATURE: u32 = 0x0606_4b50;
const CENTRAL_SIGNATURE: u32 = 0x0201_4b50;
const LOCAL_SIGNATURE: u32 = 0x0403_4b50;
const DESCRIPTOR_SIGNATURE: u32 = 0x0807_4b50;
const ZIP64_EXTRA: u16 = 0x0001;
const NTFS_EXTRA: u16 = 0x000a;
const EXTENDED_TIMESTAMP_EXTRA: u16 = 0x5455;
const UNICODE_PATH_EXTRA: u16 = 0x7075;
const UNICODE_COMMENT_EXTRA: u16 = 0x6375;
const UTF8_FLAG: u16 = 1 << 11;
const DESCRIPTOR_FLAG: u16 = 1 << 3;
const ENCRYPTED_FLAG: u16 = 1;
const STRONG_ENCRYPTION_FLAG: u16 = 1 << 6;

/// Caller-owned strict ZIP import limits.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ZipImportPolicy {
    pub max_archive_bytes: u64,
    pub max_entries: u64,
    pub max_central_directory_bytes: u64,
    pub max_extra_field_bytes: u64,
    pub max_name_comment_bytes: u64,
    pub max_compressed_entry_bytes: u64,
    pub max_uncompressed_entry_bytes: u64,
    pub max_total_uncompressed_bytes: u64,
    pub max_expansion_ratio_milli: u64,
}

impl Default for ZipImportPolicy {
    fn default() -> Self {
        Self {
            max_archive_bytes: 4 * 1024 * 1024 * 1024,
            max_entries: 1_000_000,
            max_central_directory_bytes: 256 * 1024 * 1024,
            max_extra_field_bytes: 64 * 1024 * 1024,
            max_name_comment_bytes: 64 * 1024 * 1024,
            max_compressed_entry_bytes: 2 * 1024 * 1024 * 1024,
            max_uncompressed_entry_bytes: 4 * 1024 * 1024 * 1024,
            max_total_uncompressed_bytes: 16 * 1024 * 1024 * 1024,
            max_expansion_ratio_milli: 1_000_000,
        }
    }
}

/// Structured conversion report returned alongside the ordinary native EAM.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ZipConversionReport {
    pub observation: LegacyArchiveObservation,
    pub resolutions: Box<[ConversionResolution]>,
    pub synthesized_ancestors: Box<[LogicalPath]>,
}

/// Result of strict ZIP reconciliation and ordinary native planning.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ZipImportResult {
    pub archive: Archive,
    pub report: ZipConversionReport,
}

/// Parsed ZIP evidence plus the exact source needed by the resolver. The
/// structural internals remain private so callers cannot bypass reconciliation.
#[derive(Clone, Debug)]
pub struct ZipObservation {
    lom: LegacyArchiveObservation,
    source: Box<[u8]>,
    entries: Box<[ObservedZipEntry]>,
    unsupported_metadata: BTreeSet<String>,
}

impl ZipObservation {
    #[must_use]
    pub const fn lom(&self) -> &LegacyArchiveObservation {
        &self.lom
    }
}

#[derive(Clone, Debug)]
struct ExtraField {
    id: u16,
    data: Box<[u8]>,
    location: LegacyEvidenceLocation,
}

#[derive(Clone, Debug)]
struct HeaderClaims {
    authority: LegacyAuthority,
    location: LegacyEvidenceLocation,
    version_made_by: Option<u16>,
    flags: u16,
    method: u16,
    dos_time: u16,
    dos_date: u16,
    crc32: Option<u32>,
    crc32_raw: Box<[u8]>,
    compressed_size: Option<u64>,
    compressed_size_raw: Box<[u8]>,
    uncompressed_size: Option<u64>,
    uncompressed_size_raw: Box<[u8]>,
    name: Box<[u8]>,
    extra: Box<[ExtraField]>,
    comment: Box<[u8]>,
    internal_attributes: Option<u16>,
    external_attributes: Option<u32>,
    disk_start: Option<u32>,
    local_offset: Option<u64>,
}

#[derive(Clone, Debug)]
struct DescriptorClaims {
    authority: LegacyAuthority,
    location: LegacyEvidenceLocation,
    crc32: u32,
    crc32_raw: Box<[u8]>,
    compressed_size: u64,
    compressed_size_raw: Box<[u8]>,
    uncompressed_size: u64,
    uncompressed_size_raw: Box<[u8]>,
}

#[derive(Clone, Debug)]
struct ObservedZipEntry {
    ordinal: u64,
    central: HeaderClaims,
    local: HeaderClaims,
    data_offset: u64,
    data_length: u64,
    extent_end: u64,
}

#[derive(Clone, Debug)]
struct ResolvedEntry {
    path: LogicalPath,
    components: Vec<String>,
    directory: bool,
    executable: bool,
    mtime: Option<Timestamp>,
    plaintext: Box<[u8]>,
}

#[derive(Clone, Copy, Debug)]
struct Zip64DirectoryClaims {
    entry_count: u64,
    central_directory_size: u64,
    central_directory_offset: u64,
    eocd_offset: usize,
    locator_offset: usize,
}

/// Structurally detects and independently observes one ZIP byte sequence.
pub fn observe(source: &[u8], policy: ZipImportPolicy) -> Result<ZipObservation> {
    policy_check(
        u64::try_from(source.len()).unwrap_or(u64::MAX) <= policy.max_archive_bytes,
        "source ZIP exceeds max_archive_bytes",
    )?;
    let source_digest = sha256_exact(source);
    let eocd_offset = find_eocd(source)?;
    let eocd = slice(source, eocd_offset, 22, "EOCD")?;
    let comment_len = usize::from(le_u16(eocd, 20)?);
    if eocd_offset
        .checked_add(22)
        .and_then(|value| value.checked_add(comment_len))
        != Some(source.len())
    {
        return Err(structure("EOCD comment length does not end at EOF"));
    }
    policy_check(
        u64::try_from(comment_len).unwrap_or(u64::MAX) <= policy.max_name_comment_bytes,
        "archive comment exceeds max_name_comment_bytes",
    )?;

    let disk = le_u16(eocd, 4)?;
    let central_disk = le_u16(eocd, 6)?;
    let entries_on_disk = le_u16(eocd, 8)?;
    let entries_total_16 = le_u16(eocd, 10)?;
    if disk != 0 || central_disk != 0 || entries_on_disk != entries_total_16 {
        return Err(unsupported("multi-disk/spanned ZIP is not supported"));
    }
    let cd_size_32 = le_u32(eocd, 12)?;
    let cd_offset_32 = le_u32(eocd, 16)?;
    let needs_zip64 =
        entries_total_16 == u16::MAX || cd_size_32 == u32::MAX || cd_offset_32 == u32::MAX;
    let zip64 = if needs_zip64 {
        Some(parse_zip64_directory(source, eocd_offset)?)
    } else {
        None
    };
    let entry_count = zip64.map_or(u64::from(entries_total_16), |value| value.entry_count);
    let cd_size = zip64.map_or(u64::from(cd_size_32), |value| value.central_directory_size);
    let cd_offset = zip64.map_or(u64::from(cd_offset_32), |value| {
        value.central_directory_offset
    });
    policy_check(
        entry_count <= policy.max_entries,
        "ZIP entry count exceeds policy",
    )?;
    policy_check(
        cd_size <= policy.max_central_directory_bytes,
        "central directory exceeds policy",
    )?;
    let cd_start = to_usize(cd_offset, "central-directory offset")?;
    let cd_len = to_usize(cd_size, "central-directory size")?;
    let cd_end = cd_start
        .checked_add(cd_len)
        .ok_or_else(|| structure("central-directory extent overflows usize"))?;
    if cd_end > eocd_offset || cd_end > source.len() {
        return Err(structure(
            "central directory is outside its declared extent",
        ));
    }
    let expected_directory_end = zip64.map_or(eocd_offset, |claims| claims.eocd_offset);
    if cd_end != expected_directory_end {
        return Err(unsupported(
            "records between the central directory and end record are unsupported",
        ));
    }

    let mut archive_fields = vec![
        field_u64(
            source,
            "disk_number",
            authority("EOCD", 0),
            u64::from(disk),
            eocd_offset + 4,
            2,
        ),
        field_u64(
            source,
            "central_directory_disk",
            authority("EOCD", 0),
            u64::from(central_disk),
            eocd_offset + 6,
            2,
        ),
        field_u64(
            source,
            "entries_on_disk",
            authority("EOCD", 0),
            u64::from(entries_on_disk),
            eocd_offset + 8,
            2,
        ),
        field_u64(
            source,
            "entry_count",
            authority("EOCD", 0),
            u64::from(entries_total_16),
            eocd_offset + 10,
            2,
        ),
        field_u64(
            source,
            "central_directory_size",
            authority("EOCD", 0),
            u64::from(cd_size_32),
            eocd_offset + 12,
            4,
        ),
        field_u64(
            source,
            "central_directory_offset",
            authority("EOCD", 0),
            u64::from(cd_offset_32),
            eocd_offset + 16,
            4,
        ),
    ];
    if let Some(zip64) = zip64 {
        archive_fields.extend([
            field_u64(
                source,
                "zip64_eocd_offset",
                authority("ZIP64-locator", 0),
                u64::try_from(zip64.eocd_offset).unwrap_or(u64::MAX),
                zip64.locator_offset + 8,
                8,
            ),
            field_u64(
                source,
                "entry_count",
                authority("ZIP64-EOCD", 0),
                zip64.entry_count,
                zip64.eocd_offset + 32,
                8,
            ),
            field_u64(
                source,
                "central_directory_size",
                authority("ZIP64-EOCD", 0),
                zip64.central_directory_size,
                zip64.eocd_offset + 40,
                8,
            ),
            field_u64(
                source,
                "central_directory_offset",
                authority("ZIP64-EOCD", 0),
                zip64.central_directory_offset,
                zip64.eocd_offset + 48,
                8,
            ),
        ]);
    }
    if comment_len != 0 {
        archive_fields.push(field_bytes(
            "archive_comment",
            authority("EOCD", 0),
            &source[eocd_offset + 22..],
            eocd_offset + 22,
        ));
    }

    let mut cursor = cd_start;
    let mut entries = Vec::new();
    let mut lom_entries = Vec::new();
    let mut conflicts = Vec::new();
    if let Some(zip64) = zip64 {
        compare_zip32_zip64(
            entries_total_16,
            cd_size_32,
            cd_offset_32,
            zip64,
            eocd_offset,
            &mut conflicts,
        );
    }
    let mut unsupported_metadata = BTreeSet::new();
    if comment_len != 0 {
        unsupported_metadata.insert("zip.archive.comment".to_owned());
    }
    let mut total_extra = 0_u64;
    let mut total_name_comment = u64::try_from(comment_len).unwrap_or(u64::MAX);
    let mut declared_total_uncompressed = 0_u64;
    for ordinal in 0..entry_count {
        let (central, next, extra_bytes) = parse_central(source, cursor, ordinal)?;
        total_extra = total_extra
            .checked_add(extra_bytes)
            .ok_or_else(|| policy_error("extra-field byte count overflow"))?;
        policy_check(
            total_extra <= policy.max_extra_field_bytes,
            "ZIP extras exceed policy",
        )?;
        let local_offset = central
            .local_offset
            .ok_or_else(|| structure("central entry lacks a local-header offset"))?;
        let (local, data_offset, local_extra) = parse_local(
            source,
            to_usize(local_offset, "local-header offset")?,
            ordinal,
        )?;
        total_name_comment = total_name_comment
            .checked_add(u64::try_from(central.name.len()).unwrap_or(u64::MAX))
            .and_then(|value| {
                value.checked_add(u64::try_from(central.comment.len()).unwrap_or(u64::MAX))
            })
            .and_then(|value| {
                value.checked_add(u64::try_from(local.name.len()).unwrap_or(u64::MAX))
            })
            .ok_or_else(|| policy_error("ZIP name/comment byte count overflow"))?;
        policy_check(
            total_name_comment <= policy.max_name_comment_bytes,
            "ZIP names/comments exceed policy",
        )?;
        total_extra = total_extra
            .checked_add(local_extra)
            .ok_or_else(|| policy_error("extra-field byte count overflow"))?;
        policy_check(
            total_extra <= policy.max_extra_field_bytes,
            "ZIP extras exceed policy",
        )?;

        validate_supported_flags(&central)?;
        validate_supported_flags(&local)?;
        compare_header_claims(&central, &local, &mut conflicts);
        compare_extra_claims(&central, &local, &mut conflicts);
        let compressed_size = require_claim(central.compressed_size, "central compressed size")?;
        let uncompressed_size =
            require_claim(central.uncompressed_size, "central uncompressed size")?;
        policy_check(
            compressed_size <= policy.max_compressed_entry_bytes,
            "compressed ZIP entry exceeds policy",
        )?;
        policy_check(
            uncompressed_size <= policy.max_uncompressed_entry_bytes,
            "uncompressed ZIP entry exceeds policy",
        )?;
        declared_total_uncompressed = declared_total_uncompressed
            .checked_add(uncompressed_size)
            .ok_or_else(|| policy_error("declared ZIP output byte count overflows"))?;
        policy_check(
            declared_total_uncompressed <= policy.max_total_uncompressed_bytes,
            "declared total ZIP output exceeds policy",
        )?;
        let ratio = uncompressed_size
            .saturating_mul(1000)
            .checked_div(compressed_size)
            .unwrap_or(if uncompressed_size == 0 { 0 } else { u64::MAX });
        policy_check(
            ratio <= policy.max_expansion_ratio_milli,
            "ZIP expansion ratio exceeds policy",
        )?;

        let data_end_u64 = u64::try_from(data_offset)
            .unwrap_or(u64::MAX)
            .checked_add(compressed_size)
            .ok_or_else(|| structure("file-data extent overflows u64"))?;
        let data_end = to_usize(data_end_u64, "file-data end")?;
        if data_end > cd_start || data_end > source.len() {
            return Err(structure("file-data extent crosses the central directory"));
        }
        let descriptor = if central.flags & DESCRIPTOR_FLAG != 0 {
            let zip64_descriptor = central.compressed_size_raw.as_ref()
                == u32::MAX.to_le_bytes().as_slice()
                || central.uncompressed_size_raw.as_ref() == u32::MAX.to_le_bytes().as_slice();
            Some(parse_descriptor(
                source,
                data_end,
                ordinal,
                zip64_descriptor,
                central.crc32,
            )?)
        } else {
            None
        };
        if let Some(descriptor) = &descriptor {
            compare_descriptor(&central, descriptor, &mut conflicts);
        }
        let extent_end = descriptor.as_ref().map_or(data_end_u64, |descriptor| {
            descriptor.location.offset + descriptor.location.length
        });
        let mut fields = observe_header(&central)?;
        fields.extend(observe_header(&local)?);
        if let Some(descriptor) = &descriptor {
            fields.extend(observe_descriptor(descriptor));
        }
        inspect_extra_metadata(&central, &mut unsupported_metadata)?;
        inspect_extra_metadata(&local, &mut unsupported_metadata)?;
        if !central.comment.is_empty() {
            unsupported_metadata.insert("zip.entry.comment".to_owned());
        }
        if central.internal_attributes != Some(0) || central.external_attributes != Some(0) {
            unsupported_metadata.insert("zip.platform-attributes".to_owned());
        }
        lom_entries.push(LegacyEntryObservation {
            ordinal,
            fields: fields.into_boxed_slice(),
        });
        entries.push(ObservedZipEntry {
            ordinal,
            central,
            local,
            data_offset: u64::try_from(data_offset).unwrap_or(u64::MAX),
            data_length: compressed_size,
            extent_end,
        });
        cursor = next;
    }
    if cursor != cd_end {
        return Err(structure(
            "central-directory count/size does not consume its exact extent",
        ));
    }
    classify_extent_conflicts(&entries, &mut conflicts);

    Ok(ZipObservation {
        lom: LegacyArchiveObservation {
            source_format: "ZIP".to_owned(),
            source_digest,
            archive_fields: archive_fields.into_boxed_slice(),
            entries: lom_entries.into_boxed_slice(),
            conflicts: conflicts.into_boxed_slice(),
        },
        source: source.to_vec().into_boxed_slice(),
        entries: entries.into_boxed_slice(),
        unsupported_metadata,
    })
}

/// Reconciles one observation with strict-v1 policy and plans the resulting
/// plaintext through Entrybound's ordinary native pipeline.
pub fn resolve_strict(
    observation: ZipObservation,
    policy: ZipImportPolicy,
    profile: CompressionProfile,
) -> Result<ZipImportResult> {
    refuse_unresolved_conflicts(&observation.lom.conflicts)?;
    let mut resolved = Vec::new();
    let mut total_plaintext = 0_u64;
    let mut resolution_records = observation
        .lom
        .conflicts
        .iter()
        .filter_map(conflict_resolution)
        .collect::<Vec<_>>();
    for entry in &observation.entries {
        let resolved_entry =
            resolve_entry(entry, &observation.source, policy, &mut resolution_records)?;
        total_plaintext = total_plaintext
            .checked_add(u64::try_from(resolved_entry.plaintext.len()).unwrap_or(u64::MAX))
            .ok_or_else(|| policy_error("total uncompressed byte count overflow"))?;
        policy_check(
            total_plaintext <= policy.max_total_uncompressed_bytes,
            "total ZIP output exceeds policy",
        )?;
        resolved.push(resolved_entry);
    }

    let mut kind_by_path = BTreeMap::<LogicalPath, bool>::new();
    for entry in &resolved {
        if kind_by_path
            .insert(entry.path.clone(), entry.directory)
            .is_some()
        {
            return Err(Diagnostic::new(
                OutcomeClass::Nonconforming,
                ReasonCode::DuplicateLogicalPath,
                format!("duplicate reconciled ZIP path {}", entry.path),
            ));
        }
    }
    let mut synthesized = BTreeSet::new();
    for entry in &resolved {
        for depth in 1..entry.components.len() {
            let ancestor = LogicalPath::from_utf8(&entry.components[..depth])?;
            match kind_by_path.get(&ancestor) {
                Some(true) => {}
                Some(false) => {
                    return Err(Diagnostic::new(
                        OutcomeClass::Nonconforming,
                        ReasonCode::FileAsAncestor,
                        format!("ZIP file {} is an ancestor", ancestor),
                    ));
                }
                None => {
                    synthesized.insert(ancestor.clone());
                    kind_by_path.insert(ancestor.clone(), true);
                    resolution_records.push(ConversionResolution {
                        conflict_class: ConflictClass::Omission.as_str().to_owned(),
                        semantic_field: format!("directory:{}", ancestor),
                        authorities: Box::from(["ZIP child paths".to_owned()]),
                        observed_values: Box::from(["directory entry omitted".to_owned()]),
                        action: "synthesized explicit ancestor required by EAM".to_owned(),
                    });
                }
            }
        }
    }

    let mut entries = synthesized
        .iter()
        .cloned()
        .map(|path| {
            Entry::new(
                path,
                EntryData::Directory,
                MetadataSet::default(),
                EntryIdentity::default(),
            )
        })
        .collect::<Vec<_>>();
    let mut files = Vec::new();
    for item in resolved {
        let mut metadata = vec![MetadataItem::executable(item.executable)];
        if let Some(mtime) = item.mtime {
            metadata.push(MetadataItem::mtime(mtime));
        }
        let metadata = MetadataSet::new(metadata)?;
        if item.directory {
            entries.push(Entry::new(
                item.path,
                EntryData::Directory,
                metadata,
                EntryIdentity::default(),
            ));
        } else {
            let digest = sha256_exact(&item.plaintext);
            entries.push(Entry::new(
                item.path,
                EntryData::File {
                    content: ContentRef::Internal(digest),
                },
                metadata,
                EntryIdentity::default(),
            ));
            files.push(item.plaintext);
        }
    }

    resolution_records.sort();
    resolution_records.dedup();
    let synthesized = synthesized.into_iter().collect::<Vec<_>>();
    let unsupported_metadata = observation
        .unsupported_metadata
        .into_iter()
        .collect::<Vec<_>>();
    let omission_count = resolution_records
        .iter()
        .filter(|resolution| resolution.conflict_class == ConflictClass::Omission.as_str())
        .count()
        .try_into()
        .unwrap_or(u64::MAX);
    let refinement_count = resolution_records
        .iter()
        .filter(|resolution| resolution.conflict_class == ConflictClass::Refinement.as_str())
        .count()
        .try_into()
        .unwrap_or(u64::MAX);
    let provenance = ConversionProvenance {
        source_format: "ZIP".to_owned(),
        adapter_id: "zip-strict/v1".to_owned(),
        source_digest: observation.lom.source_digest,
        import_mode: "strict".to_owned(),
        source_entry_count: u64::try_from(observation.entries.len()).unwrap_or(u64::MAX),
        observation_count: observation.lom.observation_count(),
        omission_count,
        refinement_count,
        divergence_count: observation.lom.conflict_count(ConflictClass::Divergence),
        irreconcilable_count: observation
            .lom
            .conflict_count(ConflictClass::Irreconcilable),
        resolutions: resolution_records.clone().into_boxed_slice(),
        synthesized_ancestors: synthesized.clone().into_boxed_slice(),
        unsupported_metadata: unsupported_metadata.clone().into_boxed_slice(),
        outcome: "success".to_owned(),
    };
    let fidelity = zip_fidelity(&unsupported_metadata);
    let archive = plan_observed_archive(entries, files, fidelity, provenance, profile)?;
    Ok(ZipImportResult {
        archive,
        report: ZipConversionReport {
            observation: observation.lom,
            resolutions: resolution_records.into_boxed_slice(),
            synthesized_ancestors: synthesized.into_boxed_slice(),
        },
    })
}

/// Convenience operation retaining the observe-then-resolve boundary.
pub fn import_strict(
    source: &[u8],
    policy: ZipImportPolicy,
    profile: CompressionProfile,
) -> Result<ZipImportResult> {
    resolve_strict(observe(source, policy)?, policy, profile)
}

fn parse_zip64_directory(source: &[u8], eocd_offset: usize) -> Result<Zip64DirectoryClaims> {
    let locator_offset = eocd_offset
        .checked_sub(20)
        .ok_or_else(|| structure("ZIP64 locator is missing"))?;
    let locator = slice(source, locator_offset, 20, "ZIP64 locator")?;
    if le_u32(locator, 0)? != ZIP64_LOCATOR_SIGNATURE
        || le_u32(locator, 4)? != 0
        || le_u32(locator, 16)? != 1
    {
        return Err(unsupported("invalid or multi-disk ZIP64 locator"));
    }
    let zip64_offset = to_usize(le_u64(locator, 8)?, "ZIP64 EOCD offset")?;
    let prefix = slice(source, zip64_offset, 12, "ZIP64 EOCD prefix")?;
    let record_body_len = le_u64(prefix, 4)?;
    if le_u32(prefix, 0)? != ZIP64_EOCD_SIGNATURE || record_body_len < 44 {
        return Err(structure("invalid ZIP64 EOCD"));
    }
    let record_len = to_usize(
        record_body_len
            .checked_add(12)
            .ok_or_else(|| structure("ZIP64 EOCD length overflows"))?,
        "ZIP64 EOCD length",
    )?;
    let header = slice(source, zip64_offset, record_len, "ZIP64 EOCD")?;
    if zip64_offset.checked_add(record_len) != Some(locator_offset) {
        return Err(structure("ZIP64 EOCD extent does not end at its locator"));
    }
    if le_u32(header, 16)? != 0
        || le_u32(header, 20)? != 0
        || le_u64(header, 24)? != le_u64(header, 32)?
    {
        return Err(unsupported("multi-disk ZIP64 archive is not supported"));
    }
    let entries = le_u64(header, 32)?;
    let size = le_u64(header, 40)?;
    let offset = le_u64(header, 48)?;
    Ok(Zip64DirectoryClaims {
        entry_count: entries,
        central_directory_size: size,
        central_directory_offset: offset,
        eocd_offset: zip64_offset,
        locator_offset,
    })
}

fn compare_zip32_zip64(
    entries16: u16,
    size32: u32,
    offset32: u32,
    zip64: Zip64DirectoryClaims,
    eocd_offset: usize,
    conflicts: &mut Vec<LegacyConflict>,
) {
    compare_zip32_zip64_value(
        "entry_count",
        (entries16 != u16::MAX).then_some(u64::from(entries16)),
        zip64.entry_count,
        location(eocd_offset + 10, 2),
        location(zip64.eocd_offset + 32, 8),
        conflicts,
    );
    compare_zip32_zip64_value(
        "central_directory_size",
        (size32 != u32::MAX).then_some(u64::from(size32)),
        zip64.central_directory_size,
        location(eocd_offset + 12, 4),
        location(zip64.eocd_offset + 40, 8),
        conflicts,
    );
    compare_zip32_zip64_value(
        "central_directory_offset",
        (offset32 != u32::MAX).then_some(u64::from(offset32)),
        zip64.central_directory_offset,
        location(eocd_offset + 16, 4),
        location(zip64.eocd_offset + 48, 8),
        conflicts,
    );
}

fn compare_zip32_zip64_value(
    field: &str,
    zip32: Option<u64>,
    zip64: u64,
    zip32_location: LegacyEvidenceLocation,
    zip64_location: LegacyEvidenceLocation,
    conflicts: &mut Vec<LegacyConflict>,
) {
    if let Some(zip32) = zip32
        && zip32 != zip64
    {
        let zip32_authority = authority("EOCD", 0);
        let zip64_authority = authority("ZIP64-EOCD", 0);
        conflicts.push(conflict(
            field,
            claim(&zip32_authority, zip32.to_string(), zip32_location),
            claim(&zip64_authority, zip64.to_string(), zip64_location),
            ConflictClass::Divergence,
            None,
        ));
    }
}

fn parse_central(source: &[u8], offset: usize, ordinal: u64) -> Result<(HeaderClaims, usize, u64)> {
    let fixed = slice(source, offset, 46, "central-directory entry")?;
    if le_u32(fixed, 0)? != CENTRAL_SIGNATURE {
        return Err(structure("central-directory entry signature is invalid"));
    }
    let name_len = usize::from(le_u16(fixed, 28)?);
    let extra_len = usize::from(le_u16(fixed, 30)?);
    let comment_len = usize::from(le_u16(fixed, 32)?);
    let total = 46_usize
        .checked_add(name_len)
        .and_then(|value| value.checked_add(extra_len))
        .and_then(|value| value.checked_add(comment_len))
        .ok_or_else(|| structure("central entry length overflows"))?;
    let all = slice(source, offset, total, "central-directory entry body")?;
    let name = &all[46..46 + name_len];
    let extra_start = offset + 46 + name_len;
    let extra = parse_extras(&all[46 + name_len..46 + name_len + extra_len], extra_start)?;
    let comment = &all[46 + name_len + extra_len..];
    let raw_uncompressed = le_u32(fixed, 24)?;
    let raw_compressed = le_u32(fixed, 20)?;
    let raw_offset = le_u32(fixed, 42)?;
    let raw_disk = le_u16(fixed, 34)?;
    let zip64 = zip64_values(
        &extra,
        raw_uncompressed == u32::MAX,
        raw_compressed == u32::MAX,
        raw_offset == u32::MAX,
        raw_disk == u16::MAX,
    )?;
    Ok((
        HeaderClaims {
            authority: authority("central-directory", ordinal),
            location: location(offset, total),
            version_made_by: Some(le_u16(fixed, 4)?),
            flags: le_u16(fixed, 8)?,
            method: le_u16(fixed, 10)?,
            dos_time: le_u16(fixed, 12)?,
            dos_date: le_u16(fixed, 14)?,
            crc32: Some(le_u32(fixed, 16)?),
            crc32_raw: fixed[16..20].to_vec().into_boxed_slice(),
            compressed_size: Some(zip64.compressed.unwrap_or(u64::from(raw_compressed))),
            compressed_size_raw: fixed[20..24].to_vec().into_boxed_slice(),
            uncompressed_size: Some(zip64.uncompressed.unwrap_or(u64::from(raw_uncompressed))),
            uncompressed_size_raw: fixed[24..28].to_vec().into_boxed_slice(),
            name: name.to_vec().into_boxed_slice(),
            extra: extra.into_boxed_slice(),
            comment: comment.to_vec().into_boxed_slice(),
            internal_attributes: Some(le_u16(fixed, 36)?),
            external_attributes: Some(le_u32(fixed, 38)?),
            disk_start: Some(zip64.disk.unwrap_or(u32::from(raw_disk))),
            local_offset: Some(zip64.offset.unwrap_or(u64::from(raw_offset))),
        },
        offset + total,
        u64::try_from(extra_len).unwrap_or(u64::MAX),
    ))
}

fn parse_local(source: &[u8], offset: usize, ordinal: u64) -> Result<(HeaderClaims, usize, u64)> {
    let fixed = slice(source, offset, 30, "local file header")?;
    if le_u32(fixed, 0)? != LOCAL_SIGNATURE {
        return Err(structure(
            "central directory points to a non-local-header record",
        ));
    }
    let name_len = usize::from(le_u16(fixed, 26)?);
    let extra_len = usize::from(le_u16(fixed, 28)?);
    let total = 30_usize
        .checked_add(name_len)
        .and_then(|value| value.checked_add(extra_len))
        .ok_or_else(|| structure("local header length overflows"))?;
    let all = slice(source, offset, total, "local file header body")?;
    let name = &all[30..30 + name_len];
    let extra = parse_extras(&all[30 + name_len..], offset + 30 + name_len)?;
    let flags = le_u16(fixed, 6)?;
    let raw_uncompressed = le_u32(fixed, 22)?;
    let raw_compressed = le_u32(fixed, 18)?;
    let zip64 = zip64_values(
        &extra,
        raw_uncompressed == u32::MAX,
        raw_compressed == u32::MAX,
        false,
        false,
    )?;
    let omitted = flags & DESCRIPTOR_FLAG != 0;
    Ok((
        HeaderClaims {
            authority: authority("local-header", ordinal),
            location: location(offset, total),
            version_made_by: None,
            flags,
            method: le_u16(fixed, 8)?,
            dos_time: le_u16(fixed, 10)?,
            dos_date: le_u16(fixed, 12)?,
            crc32: (!omitted || le_u32(fixed, 14)? != 0).then(|| le_u32(fixed, 14).unwrap_or(0)),
            crc32_raw: fixed[14..18].to_vec().into_boxed_slice(),
            compressed_size: (!omitted || raw_compressed != 0)
                .then_some(zip64.compressed.unwrap_or(u64::from(raw_compressed))),
            compressed_size_raw: fixed[18..22].to_vec().into_boxed_slice(),
            uncompressed_size: (!omitted || raw_uncompressed != 0)
                .then_some(zip64.uncompressed.unwrap_or(u64::from(raw_uncompressed))),
            uncompressed_size_raw: fixed[22..26].to_vec().into_boxed_slice(),
            name: name.to_vec().into_boxed_slice(),
            extra: extra.into_boxed_slice(),
            comment: Box::default(),
            internal_attributes: None,
            external_attributes: None,
            disk_start: None,
            local_offset: None,
        },
        offset + total,
        u64::try_from(extra_len).unwrap_or(u64::MAX),
    ))
}

#[derive(Default)]
struct Zip64Values {
    uncompressed: Option<u64>,
    compressed: Option<u64>,
    offset: Option<u64>,
    disk: Option<u32>,
}

fn zip64_values(
    extras: &[ExtraField],
    need_uncompressed: bool,
    need_compressed: bool,
    need_offset: bool,
    need_disk: bool,
) -> Result<Zip64Values> {
    let matches = extras
        .iter()
        .filter(|extra| extra.id == ZIP64_EXTRA)
        .collect::<Vec<_>>();
    if matches.len() > 1 {
        return Err(irreconcilable("duplicate ZIP64 extra fields"));
    }
    let Some(extra) = matches.first() else {
        if need_uncompressed || need_compressed || need_offset || need_disk {
            return Err(structure("ZIP64 sentinel lacks ZIP64 extra data"));
        }
        return Ok(Zip64Values::default());
    };
    let mut cursor = 0;
    let mut result = Zip64Values::default();
    if need_uncompressed {
        result.uncompressed = Some(extra_u64(extra, &mut cursor)?);
    }
    if need_compressed {
        result.compressed = Some(extra_u64(extra, &mut cursor)?);
    }
    if need_offset {
        result.offset = Some(extra_u64(extra, &mut cursor)?);
    }
    if need_disk {
        let bytes = extra
            .data
            .get(cursor..cursor + 4)
            .ok_or_else(|| structure("ZIP64 disk field is truncated"))?;
        result.disk = Some(u32::from_le_bytes(bytes.try_into().unwrap()));
        cursor += 4;
    }
    if cursor != extra.data.len() {
        return Err(divergence(
            "ZIP64 extra contains contradictory unrequested values",
        ));
    }
    Ok(result)
}

fn extra_u64(extra: &ExtraField, cursor: &mut usize) -> Result<u64> {
    let bytes = extra
        .data
        .get(*cursor..*cursor + 8)
        .ok_or_else(|| structure("ZIP64 value is truncated"))?;
    *cursor += 8;
    Ok(u64::from_le_bytes(bytes.try_into().unwrap()))
}

fn parse_extras(bytes: &[u8], absolute_offset: usize) -> Result<Vec<ExtraField>> {
    let mut extras = Vec::new();
    let mut cursor = 0;
    while cursor < bytes.len() {
        if bytes.len() - cursor < 4 {
            return Err(structure("extra-field header is truncated"));
        }
        let id = le_u16(bytes, cursor)?;
        let len = usize::from(le_u16(bytes, cursor + 2)?);
        let end = cursor
            .checked_add(4)
            .and_then(|value| value.checked_add(len))
            .ok_or_else(|| structure("extra-field length overflows"))?;
        if end > bytes.len() {
            return Err(structure("extra-field data exceeds enclosing header"));
        }
        extras.push(ExtraField {
            id,
            data: bytes[cursor + 4..end].to_vec().into_boxed_slice(),
            location: location(absolute_offset + cursor, end - cursor),
        });
        cursor = end;
    }
    Ok(extras)
}

fn parse_descriptor(
    source: &[u8],
    offset: usize,
    ordinal: u64,
    zip64: bool,
    expected_crc: Option<u32>,
) -> Result<DescriptorClaims> {
    let first = le_u32(slice(source, offset, 4, "data descriptor")?, 0)?;
    if first == DESCRIPTOR_SIGNATURE && expected_crc == Some(DESCRIPTOR_SIGNATURE) {
        return Err(irreconcilable(
            "data descriptor signature is ambiguous with the CRC value",
        ));
    }
    let signed = first == DESCRIPTOR_SIGNATURE;
    let base = offset + usize::from(signed) * 4;
    let size_width = if zip64 { 8 } else { 4 };
    let total = usize::from(signed) * 4 + 4 + size_width * 2;
    slice(source, offset, total, "data descriptor")?;
    let crc32 = le_u32(source, base)?;
    let compressed_size = if zip64 {
        le_u64(source, base + 4)?
    } else {
        u64::from(le_u32(source, base + 4)?)
    };
    let uncompressed_size = if zip64 {
        le_u64(source, base + 12)?
    } else {
        u64::from(le_u32(source, base + 8)?)
    };
    Ok(DescriptorClaims {
        authority: authority("data-descriptor", ordinal),
        location: location(offset, total),
        crc32,
        crc32_raw: source[base..base + 4].to_vec().into_boxed_slice(),
        compressed_size,
        compressed_size_raw: source[base + 4..base + 4 + size_width]
            .to_vec()
            .into_boxed_slice(),
        uncompressed_size,
        uncompressed_size_raw: source[base + 4 + size_width..base + 4 + size_width * 2]
            .to_vec()
            .into_boxed_slice(),
    })
}

fn resolve_entry(
    entry: &ObservedZipEntry,
    source: &[u8],
    policy: ZipImportPolicy,
    resolutions: &mut Vec<ConversionResolution>,
) -> Result<ResolvedEntry> {
    let central = &entry.central;
    let local = &entry.local;
    let name = resolve_name(central, local, resolutions)?;
    let (path, components, marker_directory) = logical_path(&name)?;
    let attribute_kind = entry_kind(central)?;
    let directory = match attribute_kind {
        Some(value) if value != marker_directory => {
            return Err(divergence(format!(
                "entry kind declarations disagree for {name}"
            )));
        }
        Some(value) => value,
        None => marker_directory,
    };
    if directory && require_claim(central.uncompressed_size, "directory size")? != 0 {
        return Err(divergence("directory entry declares non-zero content"));
    }
    let data_start = to_usize(entry.data_offset, "file-data offset")?;
    let data_len = to_usize(entry.data_length, "file-data length")?;
    let compressed = slice(source, data_start, data_len, "file data")?;
    let expected_len = require_claim(central.uncompressed_size, "uncompressed size")?;
    let plaintext = if directory {
        Vec::new()
    } else {
        decode_entry(compressed, central.method, expected_len, policy)?
    };
    if u64::try_from(plaintext.len()).unwrap_or(u64::MAX) != expected_len {
        return Err(Diagnostic::new(
            OutcomeClass::Corrupt,
            ReasonCode::ZipSizeMismatch,
            format!("decompressed size mismatch for {name}"),
        ));
    }
    let mut crc = Crc32::new();
    crc.update(&plaintext);
    if crc.finalize() != require_claim(central.crc32, "CRC-32")? {
        return Err(Diagnostic::new(
            OutcomeClass::Corrupt,
            ReasonCode::ZipCrcMismatch,
            format!("CRC-32 mismatch for {name}"),
        ));
    }
    let executable = unix_mode(central).is_some_and(|mode| mode & 0o111 != 0);
    let mtime = resolve_mtime(central, local, resolutions)?;
    Ok(ResolvedEntry {
        path,
        components,
        directory,
        executable,
        mtime,
        plaintext: plaintext.into_boxed_slice(),
    })
}

fn decode_entry(
    compressed: &[u8],
    method: u16,
    expected_len: u64,
    policy: ZipImportPolicy,
) -> Result<Vec<u8>> {
    match method {
        0 => {
            if u64::try_from(compressed.len()).unwrap_or(u64::MAX) != expected_len {
                return Err(Diagnostic::new(
                    OutcomeClass::Corrupt,
                    ReasonCode::ZipSizeMismatch,
                    "STORE lengths differ",
                ));
            }
            Ok(compressed.to_vec())
        }
        8 => {
            let decoder = DeflateDecoder::new(compressed);
            let maximum = expected_len.min(policy.max_uncompressed_entry_bytes);
            let mut limited = decoder.take(maximum.saturating_add(1));
            let mut output =
                Vec::with_capacity(to_usize(maximum.min(16 * 1024 * 1024), "DEFLATE capacity")?);
            limited.read_to_end(&mut output).map_err(|error| {
                Diagnostic::new(
                    OutcomeClass::Corrupt,
                    ReasonCode::ZipStructureInvalid,
                    format!("DEFLATE decode failed: {error}"),
                )
            })?;
            if u64::try_from(output.len()).unwrap_or(u64::MAX) > maximum {
                return Err(Diagnostic::new(
                    OutcomeClass::Corrupt,
                    ReasonCode::ZipSizeMismatch,
                    "DEFLATE output exceeds its reconciled declared length",
                ));
            }
            if limited.into_inner().total_in()
                != u64::try_from(compressed.len()).unwrap_or(u64::MAX)
            {
                return Err(structure(
                    "DEFLATE stream did not consume its exact compressed extent",
                ));
            }
            Ok(output)
        }
        _ => Err(unsupported(format!(
            "ZIP compression method {method} is unsupported"
        ))),
    }
}

fn resolve_name(
    central: &HeaderClaims,
    local: &HeaderClaims,
    resolutions: &mut Vec<ConversionResolution>,
) -> Result<String> {
    if central.name != local.name {
        return Err(divergence("local and central filenames differ"));
    }
    let primary = if central.flags & UTF8_FLAG != 0 {
        std::str::from_utf8(&central.name)
            .map_err(|_| ambiguous("UTF-8 flag is set but filename bytes are invalid UTF-8"))?
            .to_owned()
    } else {
        decode_cp437(&central.name)
    };
    let mut unicode_names = Vec::new();
    for extra in central
        .extra
        .iter()
        .chain(local.extra.iter())
        .filter(|extra| extra.id == UNICODE_PATH_EXTRA)
    {
        unicode_names.push(parse_unicode_extra(extra, &central.name, "Unicode path")?);
    }
    unicode_names.sort();
    unicode_names.dedup();
    if unicode_names.len() > 1
        || unicode_names
            .first()
            .is_some_and(|value| central.flags & UTF8_FLAG != 0 && value != &primary)
    {
        return Err(ambiguous(
            "Unicode path extra conflicts with the primary filename",
        ));
    }
    if let Some(unicode) = unicode_names.into_iter().next() {
        resolutions.push(ConversionResolution {
            conflict_class: ConflictClass::Refinement.as_str().to_owned(),
            semantic_field: "path".to_owned(),
            authorities: Box::from([
                "ZIP primary filename".to_owned(),
                "Info-ZIP Unicode Path".to_owned(),
            ]),
            observed_values: Box::from([primary, unicode.clone()]),
            action: "selected CRC-bound Unicode path refinement".to_owned(),
        });
        Ok(unicode)
    } else {
        Ok(primary)
    }
}

fn parse_unicode_extra(extra: &ExtraField, primary: &[u8], label: &str) -> Result<String> {
    if extra.data.len() < 5 || extra.data[0] != 1 {
        return Err(ambiguous(format!("{label} extra is malformed")));
    }
    let mut crc = Crc32::new();
    crc.update(primary);
    if crc.finalize() != u32::from_le_bytes(extra.data[1..5].try_into().unwrap()) {
        return Err(ambiguous(format!(
            "{label} extra CRC does not bind the primary bytes"
        )));
    }
    std::str::from_utf8(&extra.data[5..])
        .map(str::to_owned)
        .map_err(|_| ambiguous(format!("{label} extra is not valid UTF-8")))
}

fn logical_path(name: &str) -> Result<(LogicalPath, Vec<String>, bool)> {
    if name.contains('\0')
        || name.starts_with('/')
        || name.starts_with('\\')
        || name.contains('\\')
        || (name.len() >= 2
            && name.as_bytes()[1] == b':'
            && name.as_bytes()[0].is_ascii_alphabetic())
    {
        return Err(unsafe_path(name));
    }
    let directory = name.ends_with('/');
    let trimmed = name.strip_suffix('/').unwrap_or(name);
    if trimmed.is_empty() {
        return Err(unsafe_path(name));
    }
    let components = trimmed.split('/').map(str::to_owned).collect::<Vec<_>>();
    if components
        .iter()
        .any(|component| component.is_empty() || component == "." || component == "..")
    {
        return Err(unsafe_path(name));
    }
    let path = LogicalPath::from_utf8(&components).map_err(|_| unsafe_path(name))?;
    Ok((path, components, directory))
}

fn entry_kind(header: &HeaderClaims) -> Result<Option<bool>> {
    let made_by = header.version_made_by.unwrap_or(0);
    let host = (made_by >> 8) as u8;
    let external = header.external_attributes.unwrap_or(0);
    if host == 3 {
        let mode = (external >> 16) as u16;
        let file_type = mode & 0o170000;
        match file_type {
            0 => {}
            0o040000 => return Ok(Some(true)),
            0o100000 => return Ok(Some(false)),
            _ => {
                return Err(unsupported(format!(
                    "Unix ZIP entry type {file_type:#o} is unsupported"
                )));
            }
        }
    }
    if external & 0x10 != 0 {
        Ok(Some(true))
    } else {
        Ok(None)
    }
}

fn unix_mode(header: &HeaderClaims) -> Option<u16> {
    ((header.version_made_by.unwrap_or(0) >> 8) as u8 == 3)
        .then_some((header.external_attributes.unwrap_or(0) >> 16) as u16)
}

fn resolve_mtime(
    central: &HeaderClaims,
    local: &HeaderClaims,
    resolutions: &mut Vec<ConversionResolution>,
) -> Result<Option<Timestamp>> {
    if central.dos_date != local.dos_date || central.dos_time != local.dos_time {
        return Err(divergence("local and central DOS timestamps differ"));
    }
    let mut unix_times = Vec::new();
    for extra in central
        .extra
        .iter()
        .chain(local.extra.iter())
        .filter(|extra| extra.id == EXTENDED_TIMESTAMP_EXTRA)
    {
        if extra.data.is_empty() {
            return Err(structure("extended timestamp extra is empty"));
        }
        if extra.data[0] & 1 != 0 {
            let value = extra
                .data
                .get(1..5)
                .ok_or_else(|| structure("extended mtime is truncated"))?;
            unix_times.push(i64::from(i32::from_le_bytes(value.try_into().unwrap())));
        }
    }
    unix_times.sort();
    unix_times.dedup();
    if unix_times.len() > 1 {
        return Err(divergence("extended timestamp authorities disagree"));
    }
    let mut ntfs_times = central
        .extra
        .iter()
        .chain(local.extra.iter())
        .filter(|extra| extra.id == NTFS_EXTRA)
        .map(parse_ntfs_mtime)
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    ntfs_times.sort();
    ntfs_times.dedup();
    if ntfs_times.len() > 1 {
        return Err(divergence("NTFS timestamp authorities disagree"));
    }
    if let (Some(seconds), Some((ntfs_seconds, _))) =
        (unix_times.first().copied(), ntfs_times.first().copied())
        && seconds != ntfs_seconds
    {
        return Err(divergence("extended and NTFS mtime authorities disagree"));
    }
    if let Some((seconds, nanoseconds)) = ntfs_times.first().copied() {
        resolutions.push(ConversionResolution {
            conflict_class: ConflictClass::Refinement.as_str().to_owned(),
            semantic_field: "core.mtime".to_owned(),
            authorities: Box::from(["ZIP DOS timestamp".to_owned(), "NTFS timestamp".to_owned()]),
            observed_values: Box::from([
                "local-time two-second precision".to_owned(),
                format!("{seconds}.{nanoseconds:09}"),
            ]),
            action: "selected UTC 100-nanosecond NTFS timestamp refinement".to_owned(),
        });
        return Timestamp::new(
            seconds,
            nanoseconds,
            TimestampPrecision::Hectonanosecond,
            true,
        )
        .map(Some);
    }
    if let Some(seconds) = unix_times.first().copied() {
        resolutions.push(ConversionResolution {
            conflict_class: ConflictClass::Refinement.as_str().to_owned(),
            semantic_field: "core.mtime".to_owned(),
            authorities: Box::from([
                "ZIP DOS timestamp".to_owned(),
                "extended timestamp".to_owned(),
            ]),
            observed_values: Box::from([
                "local-time two-second precision".to_owned(),
                seconds.to_string(),
            ]),
            action: "selected UTC Unix timestamp refinement".to_owned(),
        });
        return Timestamp::new(seconds, 0, TimestampPrecision::Second, true).map(Some);
    }
    Ok(None)
}

fn parse_ntfs_mtime(extra: &ExtraField) -> Result<Option<(i64, u32)>> {
    if extra.data.len() < 4 {
        return Err(structure("NTFS extra field is truncated"));
    }
    let mut cursor = 4;
    while cursor < extra.data.len() {
        let header = extra
            .data
            .get(cursor..cursor + 4)
            .ok_or_else(|| structure("NTFS attribute header is truncated"))?;
        let tag = u16::from_le_bytes(header[..2].try_into().unwrap());
        let len = usize::from(u16::from_le_bytes(header[2..].try_into().unwrap()));
        cursor += 4;
        let data = extra
            .data
            .get(cursor..cursor + len)
            .ok_or_else(|| structure("NTFS attribute value is truncated"))?;
        if tag == 1 {
            if data.len() != 24 {
                return Err(structure(
                    "NTFS timestamp attribute must contain three FILETIMEs",
                ));
            }
            let ticks = u64::from_le_bytes(data[..8].try_into().unwrap());
            const EPOCH_TICKS: u64 = 116_444_736_000_000_000;
            let (seconds, remainder) = if ticks >= EPOCH_TICKS {
                let delta = ticks - EPOCH_TICKS;
                (
                    i64::try_from(delta / 10_000_000)
                        .map_err(|_| structure("NTFS timestamp exceeds i64"))?,
                    delta % 10_000_000,
                )
            } else {
                let delta = EPOCH_TICKS - ticks;
                let seconds = delta.div_ceil(10_000_000);
                let remainder = (seconds * 10_000_000) - delta;
                (
                    i64::try_from(seconds)
                        .map_err(|_| structure("NTFS timestamp exceeds i64"))?
                        .checked_neg()
                        .ok_or_else(|| structure("NTFS timestamp underflows i64"))?,
                    remainder,
                )
            };
            return Ok(Some((seconds, u32::try_from(remainder * 100).unwrap())));
        }
        cursor += len;
    }
    Ok(None)
}

fn compare_header_claims(
    central: &HeaderClaims,
    local: &HeaderClaims,
    conflicts: &mut Vec<LegacyConflict>,
) {
    compare_value(
        "filename",
        central,
        &central.name,
        local,
        &local.name,
        conflicts,
    );
    compare_value(
        "flags",
        central,
        &central.flags,
        local,
        &local.flags,
        conflicts,
    );
    compare_value(
        "compression_method",
        central,
        &central.method,
        local,
        &local.method,
        conflicts,
    );
    compare_value(
        "dos_time",
        central,
        &(central.dos_date, central.dos_time),
        local,
        &(local.dos_date, local.dos_time),
        conflicts,
    );
    compare_optional(
        "crc32",
        central,
        central.crc32,
        local,
        local.crc32,
        conflicts,
    );
    compare_optional(
        "compressed_size",
        central,
        central.compressed_size,
        local,
        local.compressed_size,
        conflicts,
    );
    compare_optional(
        "uncompressed_size",
        central,
        central.uncompressed_size,
        local,
        local.uncompressed_size,
        conflicts,
    );
    compare_optional(
        "internal_attributes",
        central,
        central.internal_attributes,
        local,
        local.internal_attributes,
        conflicts,
    );
    compare_optional(
        "external_attributes",
        central,
        central.external_attributes,
        local,
        local.external_attributes,
        conflicts,
    );
    compare_optional(
        "disk_start",
        central,
        central.disk_start,
        local,
        local.disk_start,
        conflicts,
    );
    compare_optional(
        "local_header_offset",
        central,
        central.local_offset,
        local,
        local.local_offset,
        conflicts,
    );
}

fn compare_extra_claims(
    central: &HeaderClaims,
    local: &HeaderClaims,
    conflicts: &mut Vec<LegacyConflict>,
) {
    let central_ids = central
        .extra
        .iter()
        .map(|extra| extra.id)
        .collect::<BTreeSet<_>>();
    let local_ids = local
        .extra
        .iter()
        .map(|extra| extra.id)
        .collect::<BTreeSet<_>>();
    for id in central_ids.symmetric_difference(&local_ids) {
        let (selected, omitted) = if central_ids.contains(id) {
            (&central.authority, &local.authority)
        } else {
            (&local.authority, &central.authority)
        };
        conflicts.push(conflict(
            &format!("extra.{id:04x}"),
            claim(
                selected,
                "present".to_owned(),
                if central_ids.contains(id) { central.location } else { local.location },
            ),
            claim(
                omitted,
                "omitted".to_owned(),
                if central_ids.contains(id) { local.location } else { central.location },
            ),
            ConflictClass::Omission,
            Some(LegacyResolution {
                action: "retained the available extra-field evidence; semantic interpretations are reconciled independently".to_owned(),
                selected_authority: Some(selected.clone()),
            }),
        ));
    }
    for id in central_ids.intersection(&local_ids) {
        let central_values = central
            .extra
            .iter()
            .filter(|extra| extra.id == *id)
            .map(|extra| extra.data.as_ref())
            .collect::<Vec<_>>();
        let local_values = local
            .extra
            .iter()
            .filter(|extra| extra.id == *id)
            .map(|extra| extra.data.as_ref())
            .collect::<Vec<_>>();
        if central_values != local_values {
            conflicts.push(conflict(
                &format!("extra.{id:04x}"),
                claim(
                    &central.authority,
                    format!("{} independently framed value(s)", central_values.len()),
                    central.location,
                ),
                claim(
                    &local.authority,
                    format!("{} independently framed value(s)", local_values.len()),
                    local.location,
                ),
                ConflictClass::Refinement,
                Some(LegacyResolution {
                    action: "retained both encodings; known semantic claims are compared by their decoded values".to_owned(),
                    selected_authority: None,
                }),
            ));
        }
    }
}

fn compare_descriptor(
    central: &HeaderClaims,
    descriptor: &DescriptorClaims,
    conflicts: &mut Vec<LegacyConflict>,
) {
    compare_optional_descriptor(
        "crc32",
        central,
        central.crc32.map(u64::from),
        descriptor,
        u64::from(descriptor.crc32),
        conflicts,
    );
    compare_optional_descriptor(
        "compressed_size",
        central,
        central.compressed_size,
        descriptor,
        descriptor.compressed_size,
        conflicts,
    );
    compare_optional_descriptor(
        "uncompressed_size",
        central,
        central.uncompressed_size,
        descriptor,
        descriptor.uncompressed_size,
        conflicts,
    );
}

fn compare_value<T: Eq + std::fmt::Debug>(
    field: &str,
    left: &HeaderClaims,
    left_value: &T,
    right: &HeaderClaims,
    right_value: &T,
    conflicts: &mut Vec<LegacyConflict>,
) {
    if left_value != right_value {
        conflicts.push(conflict(
            field,
            claim(&left.authority, format!("{left_value:?}"), left.location),
            claim(&right.authority, format!("{right_value:?}"), right.location),
            ConflictClass::Divergence,
            None,
        ));
    }
}

fn compare_optional<T: Eq + std::fmt::Debug + Copy>(
    field: &str,
    left: &HeaderClaims,
    left_value: Option<T>,
    right: &HeaderClaims,
    right_value: Option<T>,
    conflicts: &mut Vec<LegacyConflict>,
) {
    match (left_value, right_value) {
        (Some(left_value), Some(right_value)) if left_value != right_value => {
            conflicts.push(conflict(
                field,
                claim(&left.authority, format!("{left_value:?}"), left.location),
                claim(&right.authority, format!("{right_value:?}"), right.location),
                ConflictClass::Divergence,
                None,
            ))
        }
        (Some(value), None) => conflicts.push(conflict(
            field,
            claim(&left.authority, format!("{value:?}"), left.location),
            claim(&right.authority, "omitted".to_owned(), right.location),
            ConflictClass::Omission,
            Some(LegacyResolution {
                action: "used the sole structurally valid declaration".to_owned(),
                selected_authority: Some(left.authority.clone()),
            }),
        )),
        _ => {}
    }
}

fn compare_optional_descriptor(
    field: &str,
    central: &HeaderClaims,
    central_value: Option<u64>,
    descriptor: &DescriptorClaims,
    descriptor_value: u64,
    conflicts: &mut Vec<LegacyConflict>,
) {
    if let Some(value) = central_value
        && value != descriptor_value
    {
        conflicts.push(conflict(
            field,
            claim(&central.authority, value.to_string(), central.location),
            claim(
                &descriptor.authority,
                descriptor_value.to_string(),
                descriptor.location,
            ),
            ConflictClass::Divergence,
            None,
        ));
    }
}

fn classify_extent_conflicts(entries: &[ObservedZipEntry], conflicts: &mut Vec<LegacyConflict>) {
    let mut extents = entries
        .iter()
        .map(|entry| (entry.local.location.offset, entry.extent_end, entry.ordinal))
        .collect::<Vec<_>>();
    extents.sort();
    for pair in extents.windows(2) {
        if pair[0].1 > pair[1].0 {
            conflicts.push(LegacyConflict {
                semantic_field: "physical_extent".to_owned(),
                authorities: Box::from([
                    authority("entry-extent", pair[0].2),
                    authority("entry-extent", pair[1].2),
                ]),
                observed_values: Box::from([
                    LegacyObservedValue::Text(format!("{}..{}", pair[0].0, pair[0].1)),
                    LegacyObservedValue::Text(format!("{}..{}", pair[1].0, pair[1].1)),
                ]),
                evidence: Box::from([
                    LegacyEvidenceLocation {
                        offset: pair[0].0,
                        length: pair[0].1 - pair[0].0,
                    },
                    LegacyEvidenceLocation {
                        offset: pair[1].0,
                        length: pair[1].1 - pair[1].0,
                    },
                ]),
                classification: ConflictClass::Irreconcilable,
                resolution: None,
            });
        }
    }
}

fn refuse_unresolved_conflicts(conflicts: &[LegacyConflict]) -> Result<()> {
    let divergence_count = conflicts
        .iter()
        .filter(|conflict| conflict.classification == ConflictClass::Divergence)
        .count();
    let irreconcilable_count = conflicts
        .iter()
        .filter(|conflict| conflict.classification == ConflictClass::Irreconcilable)
        .count();
    if let Some(conflict) = conflicts
        .iter()
        .find(|conflict| conflict.classification == ConflictClass::Irreconcilable)
    {
        if conflict.semantic_field == "physical_extent" {
            return Err(Diagnostic::new(
                OutcomeClass::Nonconforming,
                ReasonCode::ZipOverlappingExtent,
                format!(
                    "ZIP entry extents overlap (divergence={divergence_count}, \
                     irreconcilable={irreconcilable_count})"
                ),
            ));
        }
        return Err(irreconcilable(format!(
            "{}: competing ZIP extents/claims cannot form one object \
             (divergence={divergence_count}, irreconcilable={irreconcilable_count})",
            conflict.semantic_field,
        )));
    }
    if let Some(conflict) = conflicts
        .iter()
        .find(|conflict| conflict.classification == ConflictClass::Divergence)
    {
        return Err(divergence(format!(
            "{} has divergent ZIP authorities \
             (divergence={divergence_count}, irreconcilable={irreconcilable_count})",
            conflict.semantic_field,
        )));
    }
    Ok(())
}

fn conflict_resolution(conflict: &LegacyConflict) -> Option<ConversionResolution> {
    conflict
        .resolution
        .as_ref()
        .map(|resolution| ConversionResolution {
            conflict_class: conflict.classification.as_str().to_owned(),
            semantic_field: conflict.semantic_field.clone(),
            authorities: conflict
                .authorities
                .iter()
                .map(authority_name)
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            observed_values: conflict
                .observed_values
                .iter()
                .map(LegacyObservedValue::display_compact)
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            action: resolution.action.clone(),
        })
}

fn inspect_extra_metadata(header: &HeaderClaims, unsupported: &mut BTreeSet<String>) -> Result<()> {
    for extra in &header.extra {
        match extra.id {
            ZIP64_EXTRA | EXTENDED_TIMESTAMP_EXTRA | UNICODE_PATH_EXTRA => {}
            UNICODE_COMMENT_EXTRA => {
                let _ = parse_unicode_extra(extra, &header.comment, "Unicode comment")?;
                unsupported.insert("zip.entry.unicode-comment".to_owned());
            }
            NTFS_EXTRA => {
                let _ = parse_ntfs_mtime(extra)?;
                unsupported.insert("zip.ntfs-timestamps".to_owned());
            }
            id => {
                unsupported.insert(format!("zip.extra.{id:04x}"));
            }
        }
    }
    Ok(())
}

fn validate_supported_flags(header: &HeaderClaims) -> Result<()> {
    if header.flags & (ENCRYPTED_FLAG | STRONG_ENCRYPTION_FLAG) != 0 {
        return Err(unsupported("encrypted ZIP entries are unsupported"));
    }
    if !matches!(header.method, 0 | 8) {
        return Err(unsupported(format!(
            "ZIP compression method {} is unsupported",
            header.method
        )));
    }
    let permitted = UTF8_FLAG | DESCRIPTOR_FLAG | 0x0006;
    if header.flags & !permitted != 0 {
        return Err(unsupported(format!(
            "ZIP general-purpose flags {:#06x} require unsupported behavior",
            header.flags & !permitted
        )));
    }
    if header.disk_start.is_some_and(|disk| disk != 0) {
        return Err(unsupported("multi-disk ZIP entry is unsupported"));
    }
    Ok(())
}

fn observe_header(
    header: &HeaderClaims,
) -> Result<Vec<LegacyFieldObservation<LegacyObservedValue>>> {
    let mut fields = vec![
        observation(
            "filename",
            header,
            header.name.clone(),
            LegacyObservedValue::Bytes(header.name.clone()),
        ),
        observation(
            "flags",
            header,
            header.flags.to_le_bytes().into(),
            LegacyObservedValue::Unsigned(u64::from(header.flags)),
        ),
        observation(
            "compression_method",
            header,
            header.method.to_le_bytes().into(),
            LegacyObservedValue::Unsigned(u64::from(header.method)),
        ),
        observation(
            "dos_time",
            header,
            header.dos_time.to_le_bytes().into(),
            LegacyObservedValue::Unsigned(u64::from(header.dos_time)),
        ),
        observation(
            "dos_date",
            header,
            header.dos_date.to_le_bytes().into(),
            LegacyObservedValue::Unsigned(u64::from(header.dos_date)),
        ),
    ];
    if let Some(crc) = header.crc32 {
        fields.push(observation(
            "crc32",
            header,
            header.crc32_raw.clone(),
            LegacyObservedValue::Unsigned(u64::from(crc)),
        ));
    }
    if let Some(size) = header.compressed_size {
        fields.push(observation(
            "compressed_size",
            header,
            header.compressed_size_raw.clone(),
            LegacyObservedValue::Unsigned(size),
        ));
    }
    if let Some(size) = header.uncompressed_size {
        fields.push(observation(
            "uncompressed_size",
            header,
            header.uncompressed_size_raw.clone(),
            LegacyObservedValue::Unsigned(size),
        ));
    }
    if !header.comment.is_empty() {
        fields.push(observation(
            "comment",
            header,
            header.comment.clone(),
            LegacyObservedValue::Bytes(header.comment.clone()),
        ));
    }
    if let Some(attributes) = header.internal_attributes {
        fields.push(observation(
            "internal_attributes",
            header,
            attributes.to_le_bytes().into(),
            LegacyObservedValue::Unsigned(u64::from(attributes)),
        ));
    }
    if let Some(attributes) = header.external_attributes {
        fields.push(observation(
            "external_attributes",
            header,
            attributes.to_le_bytes().into(),
            LegacyObservedValue::Unsigned(u64::from(attributes)),
        ));
    }
    if let Some(disk) = header.disk_start {
        fields.push(observation(
            "disk_start",
            header,
            disk.to_le_bytes().into(),
            LegacyObservedValue::Unsigned(u64::from(disk)),
        ));
    }
    if let Some(offset) = header.local_offset {
        fields.push(observation(
            "local_header_offset",
            header,
            offset.to_le_bytes().into(),
            LegacyObservedValue::Unsigned(offset),
        ));
    }
    for extra in &header.extra {
        let interpreted_value = match extra.id {
            ZIP64_EXTRA => Some(LegacyObservedValue::Text(
                "ZIP64 extended information".to_owned(),
            )),
            UNICODE_PATH_EXTRA => Some(LegacyObservedValue::Text(parse_unicode_extra(
                extra,
                &header.name,
                "Unicode path",
            )?)),
            UNICODE_COMMENT_EXTRA => Some(LegacyObservedValue::Text(parse_unicode_extra(
                extra,
                &header.comment,
                "Unicode comment",
            )?)),
            EXTENDED_TIMESTAMP_EXTRA if extra.data.first().is_some_and(|flags| flags & 1 != 0) => {
                let value = extra
                    .data
                    .get(1..5)
                    .ok_or_else(|| structure("extended mtime is truncated"))?;
                Some(LegacyObservedValue::Signed(i64::from(i32::from_le_bytes(
                    value.try_into().unwrap(),
                ))))
            }
            NTFS_EXTRA => parse_ntfs_mtime(extra)?.map(|(seconds, nanoseconds)| {
                LegacyObservedValue::Text(format!("{seconds}.{nanoseconds:09} UTC"))
            }),
            _ => None,
        };
        let validity = if interpreted_value.is_some() {
            ObservationValidity::Valid
        } else {
            ObservationValidity::Uninterpreted
        };
        fields.push(LegacyFieldObservation {
            semantic_field: format!("extra.{:04x}", extra.id),
            authority: header.authority.clone(),
            raw_value: extra.data.clone(),
            interpreted_value,
            evidence: extra.location,
            validity,
        });
    }
    Ok(fields)
}

fn observe_descriptor(
    descriptor: &DescriptorClaims,
) -> Vec<LegacyFieldObservation<LegacyObservedValue>> {
    [
        (
            "crc32",
            u64::from(descriptor.crc32),
            descriptor.crc32_raw.clone(),
        ),
        (
            "compressed_size",
            descriptor.compressed_size,
            descriptor.compressed_size_raw.clone(),
        ),
        (
            "uncompressed_size",
            descriptor.uncompressed_size,
            descriptor.uncompressed_size_raw.clone(),
        ),
    ]
    .into_iter()
    .map(|(field, value, raw)| LegacyFieldObservation {
        semantic_field: field.to_owned(),
        authority: descriptor.authority.clone(),
        raw_value: raw,
        interpreted_value: Some(LegacyObservedValue::Unsigned(value)),
        evidence: descriptor.location,
        validity: ObservationValidity::Valid,
    })
    .collect()
}

fn observation(
    semantic_field: &str,
    header: &HeaderClaims,
    raw_value: Box<[u8]>,
    interpreted_value: LegacyObservedValue,
) -> LegacyFieldObservation<LegacyObservedValue> {
    LegacyFieldObservation {
        semantic_field: semantic_field.to_owned(),
        authority: header.authority.clone(),
        raw_value,
        interpreted_value: Some(interpreted_value),
        evidence: header.location,
        validity: ObservationValidity::Valid,
    }
}

fn zip_fidelity(unsupported: &[String]) -> FidelityReport {
    let mut captured = vec![
        "core.executable".to_owned(),
        "legacy.conversion-provenance".to_owned(),
    ];
    captured.sort();
    let mut unavailable = vec![FidelityIssue {
        class: "zip.dos-mtime".to_owned(),
        reason: "DOS timestamps lack an unambiguous UTC offset; only bound extended timestamps become core.mtime".to_owned(),
        entry_scope: None,
    }];
    unavailable.extend(
        unsupported.iter().map(|class| FidelityIssue {
            class: class.clone(),
            reason: "observed as LOM evidence but unsupported by the current EAM metadata subset"
                .to_owned(),
            entry_scope: None,
        }),
    );
    unavailable
        .sort_by(|left, right| (&left.class, &left.reason).cmp(&(&right.class, &right.reason)));
    unavailable.dedup_by(|left, right| left.class == right.class && left.reason == right.reason);
    FidelityReport {
        captured: captured.into_boxed_slice(),
        unavailable: unavailable.into_boxed_slice(),
        degraded: Box::default(),
        platform: "legacy:zip".to_owned(),
        filesystem: Box::default(),
    }
}

fn find_eocd(source: &[u8]) -> Result<usize> {
    if source.len() < 22 {
        return Err(truncated("ZIP EOCD is missing"));
    }
    let start = source.len().saturating_sub(22 + usize::from(u16::MAX));
    let mut saw_complete_signature = false;
    for offset in (start..=source.len() - 22).rev() {
        if source[offset..offset + 4] == EOCD_SIGNATURE.to_le_bytes() {
            saw_complete_signature = true;
            let comment_len = usize::from(u16::from_le_bytes(
                source[offset + 20..offset + 22].try_into().unwrap(),
            ));
            if offset
                .checked_add(22)
                .and_then(|value| value.checked_add(comment_len))
                == Some(source.len())
            {
                return Ok(offset);
            }
        }
    }
    let partial_start = source.len().saturating_sub(22 + usize::from(u16::MAX));
    if source[partial_start..]
        .windows(4)
        .rposition(|window| window == EOCD_SIGNATURE.to_le_bytes())
        .is_some()
    {
        return Err(truncated("ZIP EOCD is truncated"));
    }
    if saw_complete_signature {
        return Err(structure(
            "ZIP EOCD comment length does not identify an exact EOF record",
        ));
    }
    Err(Diagnostic::new(
        OutcomeClass::Unsupported,
        ReasonCode::LegacyFormatUnsupported,
        "input is not structurally recognizable as ZIP",
    ))
}

fn field_u64(
    source: &[u8],
    semantic_field: &str,
    authority: LegacyAuthority,
    value: u64,
    offset: usize,
    length: usize,
) -> LegacyFieldObservation<LegacyObservedValue> {
    LegacyFieldObservation {
        semantic_field: semantic_field.to_owned(),
        authority,
        raw_value: source[offset..offset + length].to_vec().into_boxed_slice(),
        interpreted_value: Some(LegacyObservedValue::Unsigned(value)),
        evidence: location(offset, length),
        validity: ObservationValidity::Valid,
    }
}

fn field_bytes(
    semantic_field: &str,
    authority: LegacyAuthority,
    value: &[u8],
    offset: usize,
) -> LegacyFieldObservation<LegacyObservedValue> {
    LegacyFieldObservation {
        semantic_field: semantic_field.to_owned(),
        authority,
        raw_value: value.to_vec().into_boxed_slice(),
        interpreted_value: Some(LegacyObservedValue::Bytes(
            value.to_vec().into_boxed_slice(),
        )),
        evidence: location(offset, value.len()),
        validity: ObservationValidity::Valid,
    }
}

struct ConflictClaim<'a> {
    authority: &'a LegacyAuthority,
    value: String,
    evidence: LegacyEvidenceLocation,
}

fn claim(
    authority: &LegacyAuthority,
    value: String,
    evidence: LegacyEvidenceLocation,
) -> ConflictClaim<'_> {
    ConflictClaim {
        authority,
        value,
        evidence,
    }
}

fn conflict(
    field: &str,
    left: ConflictClaim<'_>,
    right: ConflictClaim<'_>,
    classification: ConflictClass,
    resolution: Option<LegacyResolution>,
) -> LegacyConflict {
    LegacyConflict {
        semantic_field: field.to_owned(),
        authorities: Box::from([left.authority.clone(), right.authority.clone()]),
        observed_values: Box::from([
            LegacyObservedValue::Text(left.value),
            LegacyObservedValue::Text(right.value),
        ]),
        evidence: Box::from([left.evidence, right.evidence]),
        classification,
        resolution,
    }
}

fn authority(structure: &str, instance: u64) -> LegacyAuthority {
    LegacyAuthority {
        format: "ZIP".to_owned(),
        structure: structure.to_owned(),
        instance,
    }
}

fn authority_name(authority: &LegacyAuthority) -> String {
    format!(
        "{}:{}:{}",
        authority.format, authority.structure, authority.instance
    )
}

fn location(offset: usize, length: usize) -> LegacyEvidenceLocation {
    LegacyEvidenceLocation {
        offset: u64::try_from(offset).unwrap_or(u64::MAX),
        length: u64::try_from(length).unwrap_or(u64::MAX),
    }
}

fn require_claim<T>(value: Option<T>, name: &str) -> Result<T> {
    value.ok_or_else(|| irreconcilable(format!("required ZIP {name} is omitted")))
}

fn slice<'a>(source: &'a [u8], offset: usize, length: usize, label: &str) -> Result<&'a [u8]> {
    let end = offset
        .checked_add(length)
        .ok_or_else(|| structure(format!("{label} extent overflows")))?;
    source
        .get(offset..end)
        .ok_or_else(|| truncated(format!("{label} is truncated")))
}

fn le_u16(source: &[u8], offset: usize) -> Result<u16> {
    Ok(u16::from_le_bytes(
        slice(source, offset, 2, "u16")?.try_into().unwrap(),
    ))
}

fn le_u32(source: &[u8], offset: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(
        slice(source, offset, 4, "u32")?.try_into().unwrap(),
    ))
}

fn le_u64(source: &[u8], offset: usize) -> Result<u64> {
    Ok(u64::from_le_bytes(
        slice(source, offset, 8, "u64")?.try_into().unwrap(),
    ))
}

fn to_usize(value: u64, label: &str) -> Result<usize> {
    usize::try_from(value).map_err(|_| policy_error(format!("{label} exceeds addressable memory")))
}

fn policy_check(condition: bool, detail: impl Into<String>) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(policy_error(detail))
    }
}

fn policy_error(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::PolicyRefused,
        ReasonCode::LegacyResourcePolicyRefused,
        detail,
    )
}

fn structure(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::Corrupt,
        ReasonCode::ZipStructureInvalid,
        detail,
    )
}

fn truncated(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::Truncated,
        ReasonCode::ZipStructureInvalid,
        detail,
    )
}

fn unsupported(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::Unsupported,
        ReasonCode::ZipUnsupportedFeature,
        detail,
    )
}

fn divergence(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::Nonconforming,
        ReasonCode::ZipConflictDivergence,
        detail,
    )
}

fn irreconcilable(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::Nonconforming,
        ReasonCode::ZipConflictIrreconcilable,
        detail,
    )
}

fn ambiguous(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::Nonconforming,
        ReasonCode::ZipAmbiguousName,
        detail,
    )
}

fn unsafe_path(path: impl Into<String>) -> Diagnostic {
    Diagnostic::new(OutcomeClass::Nonconforming, ReasonCode::ZipUnsafePath, path)
}

fn decode_cp437(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| {
            if *byte < 0x80 {
                char::from(*byte)
            } else {
                CP437[usize::from(*byte - 0x80)]
            }
        })
        .collect()
}

// Unicode mapping prescribed for ZIP names when the UTF-8 bit and a valid
// Info-ZIP Unicode Path field are both absent.
const CP437: [char; 128] = [
    'Ç', 'ü', 'é', 'â', 'ä', 'à', 'å', 'ç', 'ê', 'ë', 'è', 'ï', 'î', 'ì', 'Ä', 'Å', 'É', 'æ', 'Æ',
    'ô', 'ö', 'ò', 'û', 'ù', 'ÿ', 'Ö', 'Ü', '¢', '£', '¥', '₧', 'ƒ', 'á', 'í', 'ó', 'ú', 'ñ', 'Ñ',
    'ª', 'º', '¿', '⌐', '¬', '½', '¼', '¡', '«', '»', '░', '▒', '▓', '│', '┤', '╡', '╢', '╖', '╕',
    '╣', '║', '╗', '╝', '╜', '╛', '┐', '└', '┴', '┬', '├', '─', '┼', '╞', '╟', '╚', '╔', '╩', '╦',
    '╠', '═', '╬', '╧', '╨', '╤', '╥', '╙', '╘', '╒', '╓', '╫', '╪', '┘', '┌', '█', '▄', '▌', '▐',
    '▀', 'α', 'ß', 'Γ', 'π', 'Σ', 'σ', 'µ', 'τ', 'Φ', 'Θ', 'Ω', 'δ', '∞', 'φ', 'ε', '∩', '≡', '±',
    '≥', '≤', '⌠', '⌡', '÷', '≈', '°', '∙', '·', '√', 'ⁿ', '²', '■', ' ',
];

#[cfg(test)]
mod tests {
    use std::io::Write;

    use flate2::{Compression, write::DeflateEncoder};

    use super::*;
    use crate::ecf::{StreamWriteOptions, WriteOptions, encode, encode_stream, open, open_stream};

    #[derive(Clone)]
    struct TestEntry<'a> {
        name: &'a [u8],
        content: &'a [u8],
        method: u16,
        descriptor: bool,
        external: u32,
        flags: u16,
        extra: &'a [u8],
    }

    fn zip(entries: &[TestEntry<'_>]) -> Vec<u8> {
        let mut output = Vec::new();
        let mut central = Vec::new();
        for (ordinal, entry) in entries.iter().enumerate() {
            let compressed = if entry.method == 8 {
                let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
                encoder.write_all(entry.content).unwrap();
                encoder.finish().unwrap()
            } else {
                entry.content.to_vec()
            };
            let mut crc = Crc32::new();
            crc.update(entry.content);
            let crc = crc.finalize();
            let offset = output.len() as u32;
            let flags = entry.flags | if entry.descriptor { DESCRIPTOR_FLAG } else { 0 };
            output.extend_from_slice(&LOCAL_SIGNATURE.to_le_bytes());
            output.extend_from_slice(&20_u16.to_le_bytes());
            output.extend_from_slice(&flags.to_le_bytes());
            output.extend_from_slice(&entry.method.to_le_bytes());
            output.extend_from_slice(&0_u16.to_le_bytes());
            output.extend_from_slice(&0_u16.to_le_bytes());
            output.extend_from_slice(&(if entry.descriptor { 0 } else { crc }).to_le_bytes());
            output.extend_from_slice(
                &(if entry.descriptor {
                    0
                } else {
                    compressed.len() as u32
                })
                .to_le_bytes(),
            );
            output.extend_from_slice(
                &(if entry.descriptor {
                    0
                } else {
                    entry.content.len() as u32
                })
                .to_le_bytes(),
            );
            output.extend_from_slice(&(entry.name.len() as u16).to_le_bytes());
            output.extend_from_slice(&(entry.extra.len() as u16).to_le_bytes());
            output.extend_from_slice(entry.name);
            output.extend_from_slice(entry.extra);
            output.extend_from_slice(&compressed);
            if entry.descriptor {
                output.extend_from_slice(&DESCRIPTOR_SIGNATURE.to_le_bytes());
                output.extend_from_slice(&crc.to_le_bytes());
                output.extend_from_slice(&(compressed.len() as u32).to_le_bytes());
                output.extend_from_slice(&(entry.content.len() as u32).to_le_bytes());
            }
            central.extend_from_slice(&CENTRAL_SIGNATURE.to_le_bytes());
            central.extend_from_slice(&0x0314_u16.to_le_bytes());
            central.extend_from_slice(&20_u16.to_le_bytes());
            central.extend_from_slice(&flags.to_le_bytes());
            central.extend_from_slice(&entry.method.to_le_bytes());
            central.extend_from_slice(&0_u16.to_le_bytes());
            central.extend_from_slice(&0_u16.to_le_bytes());
            central.extend_from_slice(&crc.to_le_bytes());
            central.extend_from_slice(&(compressed.len() as u32).to_le_bytes());
            central.extend_from_slice(&(entry.content.len() as u32).to_le_bytes());
            central.extend_from_slice(&(entry.name.len() as u16).to_le_bytes());
            central.extend_from_slice(&(entry.extra.len() as u16).to_le_bytes());
            central.extend_from_slice(&0_u16.to_le_bytes());
            central.extend_from_slice(&0_u16.to_le_bytes());
            central.extend_from_slice(&0_u16.to_le_bytes());
            central.extend_from_slice(&entry.external.to_le_bytes());
            central.extend_from_slice(&offset.to_le_bytes());
            central.extend_from_slice(entry.name);
            central.extend_from_slice(entry.extra);
            let _ = ordinal;
        }
        let central_offset = output.len() as u32;
        output.extend_from_slice(&central);
        output.extend_from_slice(&EOCD_SIGNATURE.to_le_bytes());
        output.extend_from_slice(&0_u16.to_le_bytes());
        output.extend_from_slice(&0_u16.to_le_bytes());
        output.extend_from_slice(&(entries.len() as u16).to_le_bytes());
        output.extend_from_slice(&(entries.len() as u16).to_le_bytes());
        output.extend_from_slice(&(central.len() as u32).to_le_bytes());
        output.extend_from_slice(&central_offset.to_le_bytes());
        output.extend_from_slice(&0_u16.to_le_bytes());
        output
    }

    fn zip64_one(name: &[u8], content: &[u8]) -> Vec<u8> {
        let mut crc = Crc32::new();
        crc.update(content);
        let crc = crc.finalize();
        let mut output = Vec::new();
        output.extend_from_slice(&LOCAL_SIGNATURE.to_le_bytes());
        output.extend_from_slice(&45_u16.to_le_bytes());
        output.extend_from_slice(&0_u16.to_le_bytes());
        output.extend_from_slice(&0_u16.to_le_bytes());
        output.extend_from_slice(&0_u16.to_le_bytes());
        output.extend_from_slice(&0_u16.to_le_bytes());
        output.extend_from_slice(&crc.to_le_bytes());
        output.extend_from_slice(&u32::MAX.to_le_bytes());
        output.extend_from_slice(&u32::MAX.to_le_bytes());
        output.extend_from_slice(&(name.len() as u16).to_le_bytes());
        output.extend_from_slice(&20_u16.to_le_bytes());
        output.extend_from_slice(name);
        output.extend_from_slice(&ZIP64_EXTRA.to_le_bytes());
        output.extend_from_slice(&16_u16.to_le_bytes());
        output.extend_from_slice(&(content.len() as u64).to_le_bytes());
        output.extend_from_slice(&(content.len() as u64).to_le_bytes());
        output.extend_from_slice(content);

        let central_offset = output.len() as u64;
        let mut central = Vec::new();
        central.extend_from_slice(&CENTRAL_SIGNATURE.to_le_bytes());
        central.extend_from_slice(&0x032d_u16.to_le_bytes());
        central.extend_from_slice(&45_u16.to_le_bytes());
        central.extend_from_slice(&0_u16.to_le_bytes());
        central.extend_from_slice(&0_u16.to_le_bytes());
        central.extend_from_slice(&0_u16.to_le_bytes());
        central.extend_from_slice(&0_u16.to_le_bytes());
        central.extend_from_slice(&crc.to_le_bytes());
        central.extend_from_slice(&u32::MAX.to_le_bytes());
        central.extend_from_slice(&u32::MAX.to_le_bytes());
        central.extend_from_slice(&(name.len() as u16).to_le_bytes());
        central.extend_from_slice(&28_u16.to_le_bytes());
        central.extend_from_slice(&0_u16.to_le_bytes());
        central.extend_from_slice(&0_u16.to_le_bytes());
        central.extend_from_slice(&0_u16.to_le_bytes());
        central.extend_from_slice(&0_u32.to_le_bytes());
        central.extend_from_slice(&u32::MAX.to_le_bytes());
        central.extend_from_slice(name);
        central.extend_from_slice(&ZIP64_EXTRA.to_le_bytes());
        central.extend_from_slice(&24_u16.to_le_bytes());
        central.extend_from_slice(&(content.len() as u64).to_le_bytes());
        central.extend_from_slice(&(content.len() as u64).to_le_bytes());
        central.extend_from_slice(&0_u64.to_le_bytes());
        let central_size = central.len() as u64;
        output.extend_from_slice(&central);

        let zip64_offset = output.len() as u64;
        output.extend_from_slice(&ZIP64_EOCD_SIGNATURE.to_le_bytes());
        output.extend_from_slice(&44_u64.to_le_bytes());
        output.extend_from_slice(&45_u16.to_le_bytes());
        output.extend_from_slice(&45_u16.to_le_bytes());
        output.extend_from_slice(&0_u32.to_le_bytes());
        output.extend_from_slice(&0_u32.to_le_bytes());
        output.extend_from_slice(&1_u64.to_le_bytes());
        output.extend_from_slice(&1_u64.to_le_bytes());
        output.extend_from_slice(&central_size.to_le_bytes());
        output.extend_from_slice(&central_offset.to_le_bytes());
        output.extend_from_slice(&ZIP64_LOCATOR_SIGNATURE.to_le_bytes());
        output.extend_from_slice(&0_u32.to_le_bytes());
        output.extend_from_slice(&zip64_offset.to_le_bytes());
        output.extend_from_slice(&1_u32.to_le_bytes());
        output.extend_from_slice(&EOCD_SIGNATURE.to_le_bytes());
        output.extend_from_slice(&0_u16.to_le_bytes());
        output.extend_from_slice(&0_u16.to_le_bytes());
        output.extend_from_slice(&u16::MAX.to_le_bytes());
        output.extend_from_slice(&u16::MAX.to_le_bytes());
        output.extend_from_slice(&u32::MAX.to_le_bytes());
        output.extend_from_slice(&u32::MAX.to_le_bytes());
        output.extend_from_slice(&0_u16.to_le_bytes());
        output
    }

    #[test]
    fn strict_store_deflate_descriptor_and_ancestor_round_trip() {
        let source = zip(&[
            TestEntry {
                name: b"nested/empty",
                content: b"",
                method: 0,
                descriptor: false,
                external: 0,
                flags: 0,
                extra: b"",
            },
            TestEntry {
                name: b"nested/text.txt",
                content: b"hello hello hello",
                method: 8,
                descriptor: true,
                external: 0,
                flags: 0,
                extra: b"",
            },
        ]);
        let imported = import_strict(
            &source,
            ZipImportPolicy::default(),
            CompressionProfile::Fast,
        )
        .unwrap();
        assert_eq!(imported.report.synthesized_ancestors.len(), 1);
        assert!(imported.archive.conversion.is_some());
        let encoded = encode(&imported.archive, WriteOptions::default()).unwrap();
        let reopened = open(&encoded.bytes).unwrap();
        assert_eq!(reopened.archive.entry_set.len(), 3);
        assert_eq!(reopened.archive.conversion, imported.archive.conversion);
    }

    #[test]
    fn empty_directory_zip64_utf8_unicode_and_duplicate_content_are_valid() {
        let empty = import_strict(
            &zip(&[]),
            ZipImportPolicy::default(),
            CompressionProfile::Fast,
        )
        .unwrap();
        assert!(empty.archive.entry_set.is_empty());

        let directory = zip(&[TestEntry {
            name: b"empty/",
            content: b"",
            method: 0,
            descriptor: false,
            external: 0o040755_u32 << 16,
            flags: 0,
            extra: b"",
        }]);
        let imported = import_strict(
            &directory,
            ZipImportPolicy::default(),
            CompressionProfile::Fast,
        )
        .unwrap();
        assert!(matches!(
            imported.archive.entry_set.entries()[0].data(),
            EntryData::Directory
        ));

        let utf8 = zip(&[TestEntry {
            name: "日本.txt".as_bytes(),
            content: b"utf8",
            method: 0,
            descriptor: false,
            external: 0,
            flags: UTF8_FLAG,
            extra: b"",
        }]);
        let imported =
            import_strict(&utf8, ZipImportPolicy::default(), CompressionProfile::Fast).unwrap();
        assert_eq!(
            imported.archive.entry_set.entries()[0].path().to_string(),
            "日本.txt"
        );

        let raw_name = b"caf\x82.txt";
        let mut crc = Crc32::new();
        crc.update(raw_name);
        let mut unicode_extra = Vec::new();
        unicode_extra.extend_from_slice(&UNICODE_PATH_EXTRA.to_le_bytes());
        unicode_extra.extend_from_slice(&(5_u16 + "café.txt".len() as u16).to_le_bytes());
        unicode_extra.push(1);
        unicode_extra.extend_from_slice(&crc.finalize().to_le_bytes());
        unicode_extra.extend_from_slice("café.txt".as_bytes());
        let unicode = zip(&[TestEntry {
            name: raw_name,
            content: b"unicode",
            method: 0,
            descriptor: false,
            external: 0,
            flags: 0,
            extra: &unicode_extra,
        }]);
        let imported = import_strict(
            &unicode,
            ZipImportPolicy::default(),
            CompressionProfile::Fast,
        )
        .unwrap();
        assert_eq!(
            imported.archive.entry_set.entries()[0].path().to_string(),
            "café.txt"
        );

        let zip64 = import_strict(
            &zip64_one(b"large.bin", b"zip64"),
            ZipImportPolicy::default(),
            CompressionProfile::Fast,
        )
        .unwrap();
        assert_eq!(zip64.archive.entry_set.len(), 1);
        assert!(zip64.report.observation.archive_fields.iter().any(|field| {
            field.semantic_field == "entry_count" && field.authority.structure == "EOCD"
        }));
        assert!(zip64.report.observation.archive_fields.iter().any(|field| {
            field.semantic_field == "entry_count" && field.authority.structure == "ZIP64-EOCD"
        }));
        assert!(zip64.report.observation.archive_fields.iter().any(|field| {
            field.semantic_field == "zip64_eocd_offset"
                && field.authority.structure == "ZIP64-locator"
        }));

        let duplicate = zip(&[
            TestEntry {
                name: b"one",
                content: b"same",
                method: 0,
                descriptor: false,
                external: 0,
                flags: 0,
                extra: b"",
            },
            TestEntry {
                name: b"two",
                content: b"same",
                method: 0,
                descriptor: false,
                external: 0,
                flags: 0,
                extra: b"",
            },
        ]);
        let imported = import_strict(
            &duplicate,
            ZipImportPolicy::default(),
            CompressionProfile::Fast,
        )
        .unwrap();
        assert_eq!(imported.archive.content_store.objects.len(), 1);
    }

    #[test]
    fn conversion_provenance_changes_aux_only() {
        let source = zip(&[TestEntry {
            name: b"a",
            content: b"same",
            method: 0,
            descriptor: false,
            external: 0,
            flags: 0,
            extra: b"",
        }]);
        let imported = import_strict(
            &source,
            ZipImportPolicy::default(),
            CompressionProfile::Fast,
        )
        .unwrap();
        let with = encode(&imported.archive, WriteOptions::default()).unwrap();
        let mut without = imported.archive.clone();
        without.conversion = None;
        without.descriptor.features.incompat &= !crate::ecf::FEATURE_CONVERSION_PROVENANCE_V1;
        let without = encode(&without, WriteOptions::default()).unwrap();
        assert_eq!(with.identities.lai, without.identities.lai);
        assert_eq!(with.identities.pcr, without.identities.pcr);
        assert_ne!(with.identities.aux, without.identities.aux);

        let mut missing_feature = imported.archive.clone();
        missing_feature.descriptor.features.incompat &=
            !crate::ecf::FEATURE_CONVERSION_PROVENANCE_V1;
        assert_eq!(
            encode(&missing_feature, WriteOptions::default())
                .unwrap_err()
                .code(),
            ReasonCode::DuplicateSemanticDeclaration,
        );
        let mut missing_record = without.archive.clone();
        missing_record.descriptor.features.incompat |= crate::ecf::FEATURE_CONVERSION_PROVENANCE_V1;
        assert_eq!(
            encode(&missing_record, WriteOptions::default())
                .unwrap_err()
                .code(),
            ReasonCode::DuplicateSemanticDeclaration,
        );

        let mut stream = Vec::new();
        encode_stream(
            &imported.archive,
            StreamWriteOptions::default(),
            &mut stream,
        )
        .unwrap();
        let reopened = open_stream(stream.as_slice()).unwrap();
        assert_eq!(
            reopened.opened.archive.conversion,
            imported.archive.conversion
        );
    }

    #[test]
    fn authorities_remain_independent_and_divergence_refuses() {
        let mut source = zip(&[TestEntry {
            name: b"a",
            content: b"x",
            method: 0,
            descriptor: false,
            external: 0,
            flags: 0,
            extra: b"",
        }]);
        let central = source
            .windows(4)
            .position(|window| window == CENTRAL_SIGNATURE.to_le_bytes())
            .unwrap();
        source[central + 46] = b'b';
        let observed = observe(&source, ZipImportPolicy::default()).unwrap();
        assert!(
            observed
                .lom()
                .conflicts
                .iter()
                .any(
                    |conflict| conflict.classification == ConflictClass::Divergence
                        && conflict.semantic_field == "filename"
                )
        );
        assert_eq!(
            resolve_strict(
                observed,
                ZipImportPolicy::default(),
                CompressionProfile::Fast
            )
            .unwrap_err()
            .code(),
            ReasonCode::ZipConflictDivergence,
        );
    }

    #[test]
    fn generated_strict_conformance_cases_have_stable_outcomes() {
        let base = || {
            zip(&[TestEntry {
                name: b"a",
                content: b"payload",
                method: 0,
                descriptor: false,
                external: 0,
                flags: 0,
                extra: b"",
            }])
        };

        let mut method_mismatch = base();
        method_mismatch[8..10].copy_from_slice(&8_u16.to_le_bytes());
        let observed = observe(&method_mismatch, ZipImportPolicy::default()).unwrap();
        assert!(
            observed
                .lom()
                .conflicts
                .iter()
                .any(|conflict| conflict.semantic_field == "compression_method"
                    && conflict.classification == ConflictClass::Divergence)
        );
        assert_eq!(
            resolve_strict(
                observed,
                ZipImportPolicy::default(),
                CompressionProfile::Fast
            )
            .unwrap_err()
            .code(),
            ReasonCode::ZipConflictDivergence,
        );

        let mut size_mismatch = base();
        size_mismatch[18..22].copy_from_slice(&6_u32.to_le_bytes());
        assert_eq!(
            resolve_strict(
                observe(&size_mismatch, ZipImportPolicy::default()).unwrap(),
                ZipImportPolicy::default(),
                CompressionProfile::Fast,
            )
            .unwrap_err()
            .code(),
            ReasonCode::ZipConflictDivergence,
        );

        let mut uncompressed_size_mismatch = base();
        uncompressed_size_mismatch[22..26].copy_from_slice(&6_u32.to_le_bytes());
        assert_eq!(
            resolve_strict(
                observe(&uncompressed_size_mismatch, ZipImportPolicy::default()).unwrap(),
                ZipImportPolicy::default(),
                CompressionProfile::Fast,
            )
            .unwrap_err()
            .code(),
            ReasonCode::ZipConflictDivergence,
        );

        let duplicate_path = zip(&[
            TestEntry {
                name: b"same",
                content: b"one",
                method: 0,
                descriptor: false,
                external: 0,
                flags: 0,
                extra: b"",
            },
            TestEntry {
                name: b"same",
                content: b"two",
                method: 0,
                descriptor: false,
                external: 0,
                flags: 0,
                extra: b"",
            },
        ]);
        assert_eq!(
            import_strict(
                &duplicate_path,
                ZipImportPolicy::default(),
                CompressionProfile::Fast
            )
            .unwrap_err()
            .code(),
            ReasonCode::DuplicateLogicalPath,
        );

        let encrypted = zip(&[TestEntry {
            name: b"a",
            content: b"x",
            method: 0,
            descriptor: false,
            external: 0,
            flags: ENCRYPTED_FLAG,
            extra: b"",
        }]);
        assert_eq!(
            observe(&encrypted, ZipImportPolicy::default())
                .unwrap_err()
                .code(),
            ReasonCode::ZipUnsupportedFeature
        );
        let unsupported_method = zip(&[TestEntry {
            name: b"a",
            content: b"x",
            method: 99,
            descriptor: false,
            external: 0,
            flags: 0,
            extra: b"",
        }]);
        assert_eq!(
            observe(&unsupported_method, ZipImportPolicy::default())
                .unwrap_err()
                .code(),
            ReasonCode::ZipUnsupportedFeature
        );

        let malformed_extra = zip(&[TestEntry {
            name: b"a",
            content: b"x",
            method: 0,
            descriptor: false,
            external: 0,
            flags: 0,
            extra: &[0x75, 0x70, 0xff, 0xff],
        }]);
        assert_eq!(
            observe(&malformed_extra, ZipImportPolicy::default())
                .unwrap_err()
                .code(),
            ReasonCode::ZipStructureInvalid
        );

        let mut forged_offset = base();
        let central = forged_offset
            .windows(4)
            .position(|window| window == CENTRAL_SIGNATURE.to_le_bytes())
            .unwrap();
        forged_offset[central + 42..central + 46].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(
            observe(&forged_offset, ZipImportPolicy::default())
                .unwrap_err()
                .code(),
            ReasonCode::ZipStructureInvalid
        );

        let mut truncated_central = base();
        let central = truncated_central
            .windows(4)
            .position(|window| window == CENTRAL_SIGNATURE.to_le_bytes())
            .unwrap();
        truncated_central[central + 28..central + 30].copy_from_slice(&u16::MAX.to_le_bytes());
        assert_eq!(
            observe(&truncated_central, ZipImportPolicy::default())
                .unwrap_err()
                .code(),
            ReasonCode::ZipStructureInvalid
        );

        let mut zip64_contradiction = zip64_one(b"a", b"x");
        let eocd = zip64_contradiction.len() - 22;
        zip64_contradiction[eocd + 8..eocd + 10].copy_from_slice(&2_u16.to_le_bytes());
        zip64_contradiction[eocd + 10..eocd + 12].copy_from_slice(&2_u16.to_le_bytes());
        let observed = observe(&zip64_contradiction, ZipImportPolicy::default()).unwrap();
        assert!(observed.lom().conflicts.iter().any(|conflict| {
            conflict.semantic_field == "entry_count"
                && conflict.classification == ConflictClass::Divergence
        }));
        assert_eq!(
            resolve_strict(
                observed,
                ZipImportPolicy::default(),
                CompressionProfile::Fast,
            )
            .unwrap_err()
            .code(),
            ReasonCode::ZipConflictDivergence
        );

        let mut descriptor_mismatch = zip(&[TestEntry {
            name: b"a",
            content: b"x",
            method: 0,
            descriptor: true,
            external: 0,
            flags: 0,
            extra: b"",
        }]);
        let descriptor = descriptor_mismatch
            .windows(4)
            .position(|window| window == DESCRIPTOR_SIGNATURE.to_le_bytes())
            .unwrap();
        descriptor_mismatch[descriptor + 4] ^= 1;
        assert_eq!(
            resolve_strict(
                observe(&descriptor_mismatch, ZipImportPolicy::default()).unwrap(),
                ZipImportPolicy::default(),
                CompressionProfile::Fast,
            )
            .unwrap_err()
            .code(),
            ReasonCode::ZipConflictDivergence,
        );

        let mut descriptor_ambiguity = zip(&[TestEntry {
            name: b"a",
            content: b"x",
            method: 0,
            descriptor: true,
            external: 0,
            flags: 0,
            extra: b"",
        }]);
        let central = descriptor_ambiguity
            .windows(4)
            .position(|window| window == CENTRAL_SIGNATURE.to_le_bytes())
            .unwrap();
        descriptor_ambiguity[central + 16..central + 20]
            .copy_from_slice(&DESCRIPTOR_SIGNATURE.to_le_bytes());
        assert_eq!(
            observe(&descriptor_ambiguity, ZipImportPolicy::default())
                .unwrap_err()
                .code(),
            ReasonCode::ZipConflictIrreconcilable
        );

        let type_conflict = zip(&[TestEntry {
            name: b"not-a-directory",
            content: b"",
            method: 0,
            descriptor: false,
            external: 0o040755_u32 << 16,
            flags: 0,
            extra: b"",
        }]);
        assert_eq!(
            import_strict(
                &type_conflict,
                ZipImportPolicy::default(),
                CompressionProfile::Fast
            )
            .unwrap_err()
            .code(),
            ReasonCode::ZipConflictDivergence
        );

        let symlink = zip(&[TestEntry {
            name: b"link",
            content: b"target",
            method: 0,
            descriptor: false,
            external: 0o120777_u32 << 16,
            flags: 0,
            extra: b"",
        }]);
        assert_eq!(
            import_strict(
                &symlink,
                ZipImportPolicy::default(),
                CompressionProfile::Fast
            )
            .unwrap_err()
            .code(),
            ReasonCode::ZipUnsupportedFeature
        );

        let raw_name = b"primary.txt";
        let mut crc = Crc32::new();
        crc.update(raw_name);
        let conflicting_name = b"different.txt";
        let mut unicode_extra = Vec::new();
        unicode_extra.extend_from_slice(&UNICODE_PATH_EXTRA.to_le_bytes());
        unicode_extra.extend_from_slice(&(5_u16 + conflicting_name.len() as u16).to_le_bytes());
        unicode_extra.push(1);
        unicode_extra.extend_from_slice(&crc.finalize().to_le_bytes());
        unicode_extra.extend_from_slice(conflicting_name);
        let unicode_conflict = zip(&[TestEntry {
            name: raw_name,
            content: b"x",
            method: 0,
            descriptor: false,
            external: 0,
            flags: UTF8_FLAG,
            extra: &unicode_extra,
        }]);
        assert_eq!(
            import_strict(
                &unicode_conflict,
                ZipImportPolicy::default(),
                CompressionProfile::Fast
            )
            .unwrap_err()
            .code(),
            ReasonCode::ZipAmbiguousName
        );

        let mut multi_disk = base();
        let eocd = multi_disk.len() - 22;
        multi_disk[eocd + 4..eocd + 6].copy_from_slice(&1_u16.to_le_bytes());
        assert_eq!(
            observe(&multi_disk, ZipImportPolicy::default())
                .unwrap_err()
                .code(),
            ReasonCode::ZipUnsupportedFeature
        );

        let mut truncated = base();
        truncated.pop();
        assert_eq!(
            observe(&truncated, ZipImportPolicy::default())
                .unwrap_err()
                .code(),
            ReasonCode::ZipStructureInvalid
        );
    }

    #[test]
    fn unsafe_paths_and_resource_bombs_are_refused() {
        for name in [
            &b"../x"[..],
            &b"/root"[..],
            &b"a//b"[..],
            &b"a\\b"[..],
            &b"nul\0name"[..],
        ] {
            let source = zip(&[TestEntry {
                name,
                content: b"x",
                method: 0,
                descriptor: false,
                external: 0,
                flags: 0,
                extra: b"",
            }]);
            assert_eq!(
                import_strict(
                    &source,
                    ZipImportPolicy::default(),
                    CompressionProfile::Fast
                )
                .unwrap_err()
                .code(),
                ReasonCode::ZipUnsafePath
            );
        }
        let source = zip(&[TestEntry {
            name: b"large",
            content: &[0; 128],
            method: 8,
            descriptor: false,
            external: 0,
            flags: 0,
            extra: b"",
        }]);
        let policy = ZipImportPolicy {
            max_uncompressed_entry_bytes: 64,
            ..ZipImportPolicy::default()
        };
        assert_eq!(
            import_strict(&source, policy, CompressionProfile::Fast)
                .unwrap_err()
                .code(),
            ReasonCode::LegacyResourcePolicyRefused
        );
    }

    #[test]
    fn crc_corruption_and_overlapping_extents_fail_with_stable_codes() {
        let mut source = zip(&[TestEntry {
            name: b"a",
            content: b"content",
            method: 0,
            descriptor: false,
            external: 0,
            flags: 0,
            extra: b"",
        }]);
        source[31] ^= 1;
        assert_eq!(
            import_strict(
                &source,
                ZipImportPolicy::default(),
                CompressionProfile::Fast
            )
            .unwrap_err()
            .code(),
            ReasonCode::ZipCrcMismatch
        );

        let mut source = zip(&[
            TestEntry {
                name: b"a",
                content: b"one",
                method: 0,
                descriptor: false,
                external: 0,
                flags: 0,
                extra: b"",
            },
            TestEntry {
                name: b"b",
                content: b"two",
                method: 0,
                descriptor: false,
                external: 0,
                flags: 0,
                extra: b"",
            },
        ]);
        let central_offsets = source
            .windows(4)
            .enumerate()
            .filter_map(|(offset, value)| {
                (value == CENTRAL_SIGNATURE.to_le_bytes()).then_some(offset)
            })
            .collect::<Vec<_>>();
        source[central_offsets[1] + 42..central_offsets[1] + 46]
            .copy_from_slice(&0_u32.to_le_bytes());
        let observed = observe(&source, ZipImportPolicy::default()).unwrap();
        assert_eq!(
            resolve_strict(
                observed,
                ZipImportPolicy::default(),
                CompressionProfile::Fast
            )
            .unwrap_err()
            .code(),
            ReasonCode::ZipOverlappingExtent
        );

        let mut source = zip(&[TestEntry {
            name: b"a",
            content: b"content",
            method: 0,
            descriptor: false,
            external: 0,
            flags: 0,
            extra: b"",
        }]);
        let central = source
            .windows(4)
            .position(|window| window == CENTRAL_SIGNATURE.to_le_bytes())
            .unwrap();
        source[central + 16] ^= 1;
        let observed = observe(&source, ZipImportPolicy::default()).unwrap();
        assert_eq!(
            resolve_strict(
                observed,
                ZipImportPolicy::default(),
                CompressionProfile::Fast
            )
            .unwrap_err()
            .code(),
            ReasonCode::ZipConflictDivergence
        );
    }

    #[test]
    fn cp437_names_are_deterministic() {
        assert_eq!(decode_cp437(&[0x82]), "é");
    }
}
