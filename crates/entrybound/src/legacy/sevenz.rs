//! Independent bounded 7z observation, folder decoding, and strict projection.
//!
//! This module owns 7z structural interpretation. Codec crates only decode a
//! packed byte stream after Entrybound has validated the folder graph.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{Cursor, Read};

use bzip2::bufread::BzDecoder;
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

pub const SIGNATURE: &[u8; 6] = b"7z\xbc\xaf'\x1c";
const SIGNATURE_HEADER_LEN: usize = 32;

const K_END: u8 = 0x00;
const K_HEADER: u8 = 0x01;
const K_ARCHIVE_PROPERTIES: u8 = 0x02;
const K_ADDITIONAL_STREAMS_INFO: u8 = 0x03;
const K_MAIN_STREAMS_INFO: u8 = 0x04;
const K_FILES_INFO: u8 = 0x05;
const K_PACK_INFO: u8 = 0x06;
const K_UNPACK_INFO: u8 = 0x07;
const K_SUBSTREAMS_INFO: u8 = 0x08;
const K_SIZE: u8 = 0x09;
const K_CRC: u8 = 0x0a;
const K_FOLDER: u8 = 0x0b;
const K_CODERS_UNPACK_SIZE: u8 = 0x0c;
const K_NUM_UNPACK_STREAM: u8 = 0x0d;
const K_EMPTY_STREAM: u8 = 0x0e;
const K_EMPTY_FILE: u8 = 0x0f;
const K_ANTI: u8 = 0x10;
const K_NAME: u8 = 0x11;
const K_CTIME: u8 = 0x12;
const K_ATIME: u8 = 0x13;
const K_MTIME: u8 = 0x14;
const K_WIN_ATTRIBUTES: u8 = 0x15;
const K_ENCODED_HEADER: u8 = 0x17;
const K_START_POS: u8 = 0x18;
const K_DUMMY: u8 = 0x19;

const METHOD_COPY: &[u8] = &[0x00];
const METHOD_LZMA: &[u8] = &[0x03, 0x01, 0x01];
const METHOD_LZMA2: &[u8] = &[0x21];
const METHOD_BZIP2: &[u8] = &[0x04, 0x02, 0x02];
const METHOD_DEFLATE: &[u8] = &[0x04, 0x01, 0x08];
const METHOD_DELTA: &[u8] = &[0x03];
const METHOD_BCJ_X86: &[u8] = &[0x03, 0x03, 0x01, 0x03];
const METHOD_AES: &[u8] = &[0x06, 0xf1, 0x07, 0x01];
const METHOD_PPMD: &[u8] = &[0x03, 0x04, 0x01];
const METHOD_BCJ2: &[u8] = &[0x03, 0x03, 0x01, 0x1b];

/// Caller-owned limits for all 7z structure, evidence, and decode work.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SevenZImportPolicy {
    pub max_archive_bytes: u64,
    pub max_next_header_bytes: u64,
    pub max_decoded_header_bytes: u64,
    pub max_files: u64,
    pub max_folders: u64,
    pub max_coders_per_folder: u64,
    pub max_coder_streams: u64,
    pub max_packed_streams: u64,
    pub max_substreams_per_folder: u64,
    pub max_coder_property_bytes: u64,
    pub max_packed_input_bytes: u64,
    pub max_folder_decoded_bytes: u64,
    pub max_single_file_bytes: u64,
    pub max_total_decoded_bytes: u64,
    pub max_expansion_ratio: u64,
    pub max_dictionary_bytes: u64,
    pub max_observations: u64,
    pub max_observations_per_subject: u64,
    pub max_conflicts: u64,
    pub max_resolutions: u64,
}

impl Default for SevenZImportPolicy {
    fn default() -> Self {
        Self {
            max_archive_bytes: 16 * 1024 * 1024 * 1024,
            max_next_header_bytes: 64 * 1024 * 1024,
            max_decoded_header_bytes: 64 * 1024 * 1024,
            max_files: 1_000_000,
            max_folders: 1_000_000,
            max_coders_per_folder: 8,
            max_coder_streams: 16,
            max_packed_streams: 1_000_000,
            max_substreams_per_folder: 1_000_000,
            max_coder_property_bytes: 1024 * 1024,
            max_packed_input_bytes: 8 * 1024 * 1024 * 1024,
            max_folder_decoded_bytes: 4 * 1024 * 1024 * 1024,
            max_single_file_bytes: 4 * 1024 * 1024 * 1024,
            max_total_decoded_bytes: 16 * 1024 * 1024 * 1024,
            max_expansion_ratio: 10_000,
            max_dictionary_bytes: 1024 * 1024 * 1024,
            max_observations: 8_000_000,
            max_observations_per_subject: 4096,
            max_conflicts: 1_000_000,
            max_resolutions: 1_000_000,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SevenZConversionReport {
    pub observation: LegacyArchiveObservation,
    pub resolutions: Box<[ConversionResolution]>,
    pub synthesized_ancestors: Box<[LogicalPath]>,
    pub folder_count: u64,
    pub solid_folder_count: u64,
    pub coder_count: u64,
    pub encoded_header: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SevenZImportResult {
    pub archive: Archive,
    pub report: SevenZConversionReport,
}

#[derive(Clone, Debug)]
pub struct SevenZObservation {
    lom: LegacyArchiveObservation,
    source: Box<[u8]>,
    streams: StreamsInfo,
    files: Box<[ObservedFile]>,
    unsupported_metadata: BTreeSet<String>,
    encoded_header: bool,
}

impl SevenZObservation {
    #[must_use]
    pub const fn lom(&self) -> &LegacyArchiveObservation {
        &self.lom
    }
}

#[derive(Clone, Debug, Default)]
struct StreamsInfo {
    pack_pos: u64,
    pack_sizes: Vec<u64>,
    pack_crcs: Vec<Option<u32>>,
    folders: Vec<Folder>,
    substreams: Vec<Vec<Substream>>,
}

#[derive(Clone, Debug)]
struct Folder {
    coders: Vec<Coder>,
    bind_pairs: Vec<BindPair>,
    packed_indices: Vec<u64>,
    unpack_sizes: Vec<u64>,
    crc: Option<u32>,
}

#[derive(Clone, Debug)]
struct Coder {
    method: Box<[u8]>,
    properties: Box<[u8]>,
    input_count: u64,
    output_count: u64,
    first_input: u64,
    first_output: u64,
}

#[derive(Clone, Copy, Debug)]
struct BindPair {
    input: u64,
    output: u64,
}

#[derive(Clone, Copy, Debug)]
struct Substream {
    size: u64,
    crc: Option<u32>,
}

#[derive(Clone, Debug, Default)]
struct ObservedFile {
    name: Option<String>,
    empty_stream: bool,
    empty_file: bool,
    anti: bool,
    ctime: Option<u64>,
    atime: Option<u64>,
    mtime: Option<u64>,
    attributes: Option<u32>,
    fields: Vec<LegacyFieldObservation<LegacyObservedValue>>,
}

#[derive(Clone, Debug)]
struct ResolvedFile {
    path: LogicalPath,
    components: Vec<String>,
    directory: bool,
    executable: bool,
    mtime: Timestamp,
    plaintext: Box<[u8]>,
}

#[derive(Clone, Debug)]
struct ParsedHeader {
    streams: StreamsInfo,
    files: Vec<ObservedFile>,
    unsupported_metadata: BTreeSet<String>,
}

#[derive(Clone, Debug)]
struct ByteCursor<'a> {
    bytes: &'a [u8],
    position: usize,
    source_base: usize,
}

impl<'a> ByteCursor<'a> {
    const fn new(bytes: &'a [u8], source_base: usize) -> Self {
        Self {
            bytes,
            position: 0,
            source_base,
        }
    }

    fn byte(&mut self, what: &str) -> Result<u8> {
        let value = *self
            .bytes
            .get(self.position)
            .ok_or_else(|| truncated(format!("{what} is truncated")))?;
        self.position += 1;
        Ok(value)
    }

    fn bytes(&mut self, length: usize, what: &str) -> Result<&'a [u8]> {
        let end = self
            .position
            .checked_add(length)
            .ok_or_else(|| structure(format!("{what} extent overflow")))?;
        let value = self
            .bytes
            .get(self.position..end)
            .ok_or_else(|| truncated(format!("{what} is truncated")))?;
        self.position = end;
        Ok(value)
    }

    fn number(&mut self, what: &str) -> Result<u64> {
        let first = self.byte(what)?;
        let mut mask = 0x80_u8;
        let mut value = 0_u64;
        for index in 0..8_u32 {
            if first & mask == 0 {
                value |= u64::from(first & (mask - 1)) << (index * 8);
                return Ok(value);
            }
            value |= u64::from(self.byte(what)?) << (index * 8);
            mask >>= 1;
        }
        Ok(value)
    }

    fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.position)
    }
}

#[must_use]
pub fn looks_like_sevenz(source: &[u8]) -> bool {
    source.starts_with(SIGNATURE)
}

pub fn observe(source: &[u8], policy: SevenZImportPolicy) -> Result<SevenZObservation> {
    policy_check(
        u64::try_from(source.len()).unwrap_or(u64::MAX) <= policy.max_archive_bytes,
        "7z source exceeds max_archive_bytes",
    )?;
    if source.len() < SIGNATURE_HEADER_LEN {
        return Err(truncated("7z signature header is truncated"));
    }
    if !source.starts_with(SIGNATURE) {
        return Err(structure("7z signature is invalid"));
    }
    let major = source[6];
    let minor = source[7];
    if major != 0 || minor > 4 {
        return Err(unsupported(format!(
            "7z version {major}.{minor} is unsupported by strict-v1"
        )));
    }
    let expected_start_crc = le_u32(&source[8..12]);
    let actual_start_crc = crc32fast::hash(&source[12..32]);
    if expected_start_crc != actual_start_crc {
        return Err(integrity("7z Start Header CRC mismatch"));
    }
    let next_offset = le_u64(&source[12..20]);
    let next_size = le_u64(&source[20..28]);
    let expected_next_crc = le_u32(&source[28..32]);
    policy_check(
        next_size <= policy.max_next_header_bytes,
        "7z Next Header exceeds max_next_header_bytes",
    )?;
    let next_start = 32_u64
        .checked_add(next_offset)
        .ok_or_else(|| structure("7z Next Header offset overflow"))?;
    let next_end = next_start
        .checked_add(next_size)
        .ok_or_else(|| structure("7z Next Header extent overflow"))?;
    let start = usize::try_from(next_start)
        .map_err(|_| structure("7z Next Header offset is not addressable"))?;
    let size = usize::try_from(next_size)
        .map_err(|_| structure("7z Next Header size is not addressable"))?;
    let end = usize::try_from(next_end)
        .map_err(|_| structure("7z Next Header extent is not addressable"))?;
    if end > source.len() {
        return Err(truncated("7z Next Header is truncated"));
    }
    if end != source.len() {
        return Err(structure("7z has unexplained bytes after its Next Header"));
    }
    let next = &source[start..end];
    if crc32fast::hash(next) != expected_next_crc {
        return Err(integrity("7z Next Header CRC mismatch"));
    }

    let signature_authority = authority("signature-header", 0);
    let mut archive_fields = vec![
        field_u64("format.major", &signature_authority, u64::from(major), 6, 1),
        field_u64("format.minor", &signature_authority, u64::from(minor), 7, 1),
        field_u64(
            "next-header.offset",
            &signature_authority,
            next_offset,
            12,
            8,
        ),
        field_u64("next-header.size", &signature_authority, next_size, 20, 8),
        field_u64(
            "next-header.crc32",
            &signature_authority,
            u64::from(expected_next_crc),
            28,
            4,
        ),
    ];
    let mut encoded_header = false;
    let mut main_pack_ceiling = next_start;
    let parsed = match next.first().copied() {
        None if next_size == 0 => ParsedHeader {
            streams: StreamsInfo::default(),
            files: Vec::new(),
            unsupported_metadata: BTreeSet::new(),
        },
        Some(K_HEADER) => parse_plain_header(next, start, policy, &mut archive_fields)?,
        Some(K_ENCODED_HEADER) => {
            encoded_header = true;
            let mut cursor = ByteCursor::new(&next[1..], start + 1);
            let encoded_streams = parse_streams_info(&mut cursor, policy, &mut archive_fields)?;
            if cursor.remaining() != 0 {
                return Err(structure("Encoded Header has trailing bytes"));
            }
            let encoded_start = validate_pack_extent(&encoded_streams, next_start)?;
            main_pack_ceiling = encoded_start;
            let decoded = decode_streams(source, &encoded_streams, policy, true)?;
            if decoded.len() != 1 {
                return Err(irreconcilable(
                    "Encoded Header must resolve to exactly one decoded substream",
                ));
            }
            let decoded_header = &decoded[0];
            policy_check(
                u64::try_from(decoded_header.len()).unwrap_or(u64::MAX)
                    <= policy.max_decoded_header_bytes,
                "decoded 7z header exceeds max_decoded_header_bytes",
            )?;
            let encoded_authority = authority("encoded-header", 0);
            archive_fields.push(field_u64(
                "encoded-header.decoded-size",
                &encoded_authority,
                u64::try_from(decoded_header.len()).unwrap_or(u64::MAX),
                start,
                size,
            ));
            archive_fields.push(field_bytes(
                "encoded-header.decoded-sha256",
                &encoded_authority,
                sha256_exact(decoded_header).as_bytes(),
                start,
            ));
            parse_plain_header(decoded_header, 0, policy, &mut archive_fields)?
        }
        Some(other) => {
            return Err(structure(format!(
                "7z Next Header begins with unknown NID 0x{other:02x}"
            )));
        }
        None => unreachable!("zero-length Next Header handled above"),
    };
    validate_pack_extent(&parsed.streams, main_pack_ceiling)?;
    policy_check(
        u64::try_from(parsed.files.len()).unwrap_or(u64::MAX) <= policy.max_files,
        "7z file count exceeds policy",
    )?;
    let coder_count = parsed
        .streams
        .folders
        .iter()
        .map(|folder| folder.coders.len())
        .sum::<usize>();
    archive_fields.push(field_u64(
        "archive.file-count",
        &authority("files-info", 0),
        u64::try_from(parsed.files.len()).unwrap_or(u64::MAX),
        start,
        size,
    ));
    for (folder_ordinal, folder) in parsed.streams.folders.iter().enumerate() {
        let folder_authority =
            authority("folder", u64::try_from(folder_ordinal).unwrap_or(u64::MAX));
        for (coder_ordinal, coder) in folder.coders.iter().enumerate() {
            archive_fields.push(field_bytes(
                &format!("folder.{folder_ordinal}.coder.{coder_ordinal}.method"),
                &folder_authority,
                &coder.method,
                start,
            ));
            archive_fields.push(field_bytes(
                &format!("folder.{folder_ordinal}.coder.{coder_ordinal}.properties"),
                &folder_authority,
                &coder.properties,
                start,
            ));
            archive_fields.push(field_u64(
                &format!("folder.{folder_ordinal}.coder.{coder_ordinal}.inputs"),
                &folder_authority,
                coder.input_count,
                start,
                size,
            ));
            archive_fields.push(field_u64(
                &format!("folder.{folder_ordinal}.coder.{coder_ordinal}.outputs"),
                &folder_authority,
                coder.output_count,
                start,
                size,
            ));
        }
        for (pair_ordinal, pair) in folder.bind_pairs.iter().enumerate() {
            archive_fields.push(field_text(
                &format!("folder.{folder_ordinal}.bind.{pair_ordinal}"),
                &folder_authority,
                &format!("output={} -> input={}", pair.output, pair.input),
                start,
                size,
            ));
        }
    }
    archive_fields.push(field_u64(
        "archive.folder-count",
        &authority("unpack-info", 0),
        u64::try_from(parsed.streams.folders.len()).unwrap_or(u64::MAX),
        start,
        size,
    ));
    archive_fields.push(field_u64(
        "archive.coder-count",
        &authority("unpack-info", 0),
        u64::try_from(coder_count).unwrap_or(u64::MAX),
        start,
        size,
    ));
    policy_check(
        u64::try_from(archive_fields.len())
            .unwrap_or(u64::MAX)
            .saturating_add(
                parsed
                    .files
                    .iter()
                    .map(|file| u64::try_from(file.fields.len()).unwrap_or(u64::MAX))
                    .sum::<u64>(),
            )
            <= policy.max_observations,
        "7z observations exceed policy",
    )?;
    for file in &parsed.files {
        policy_check(
            u64::try_from(file.fields.len()).unwrap_or(u64::MAX)
                <= policy.max_observations_per_subject,
            "7z observations per file exceed policy",
        )?;
    }
    let lom_entries = parsed
        .files
        .iter()
        .enumerate()
        .map(|(ordinal, file)| LegacyEntryObservation {
            ordinal: u64::try_from(ordinal).unwrap_or(u64::MAX),
            fields: file.fields.clone().into_boxed_slice(),
        })
        .collect::<Vec<_>>();
    let mut conflicts = Vec::new();
    for (ordinal, file) in parsed.files.iter().enumerate() {
        if file.empty_stream {
            conflicts.push(LegacyConflict {
                semantic_field: format!("entry.{ordinal}.kind"),
                authorities: Box::from([
                    authority(
                        "files-info-empty-stream",
                        u64::try_from(ordinal).unwrap_or(u64::MAX),
                    ),
                    authority(
                        "files-info-empty-file",
                        u64::try_from(ordinal).unwrap_or(u64::MAX),
                    ),
                ]),
                observed_values: Box::from([
                    LegacyObservedValue::Boolean(true),
                    LegacyObservedValue::Boolean(file.empty_file),
                ]),
                evidence: Box::from([location(start, size), location(start, size)]),
                classification: ConflictClass::Refinement,
                resolution: Some(LegacyResolution {
                    action: if file.empty_file {
                        "EmptyStream + EmptyFile selects an empty regular file"
                    } else {
                        "EmptyStream without EmptyFile selects a directory"
                    }
                    .to_owned(),
                    selected_authority: Some(authority(
                        "files-info-empty-file",
                        u64::try_from(ordinal).unwrap_or(u64::MAX),
                    )),
                }),
            });
        }
    }
    for (ordinal, streams) in parsed.streams.substreams.iter().enumerate() {
        conflicts.push(LegacyConflict {
            semantic_field: format!("folder.{ordinal}.stream-ownership"),
            authorities: Box::from([
                authority("unpack-info", u64::try_from(ordinal).unwrap_or(u64::MAX)),
                authority("substreams-info", u64::try_from(ordinal).unwrap_or(u64::MAX)),
            ]),
            observed_values: Box::from([
                LegacyObservedValue::Unsigned(
                    u64::try_from(parsed.streams.folders[ordinal].coders.len())
                        .unwrap_or(u64::MAX),
                ),
                LegacyObservedValue::Unsigned(
                    u64::try_from(streams.len()).unwrap_or(u64::MAX),
                ),
            ]),
            evidence: Box::from([location(start, size), location(start, size)]),
            classification: ConflictClass::Refinement,
            resolution: Some(LegacyResolution {
                action: "validated folder output and SubStreamsInfo partition jointly refine stream ownership"
                    .to_owned(),
                selected_authority: Some(authority(
                    "substreams-info",
                    u64::try_from(ordinal).unwrap_or(u64::MAX),
                )),
            }),
        });
    }
    policy_check(
        u64::try_from(conflicts.len()).unwrap_or(u64::MAX) <= policy.max_conflicts,
        "7z conflicts exceed policy",
    )?;
    Ok(SevenZObservation {
        lom: LegacyArchiveObservation {
            source_format: "7z".to_owned(),
            source_digest: sha256_exact(source),
            archive_fields: archive_fields.into_boxed_slice(),
            entries: lom_entries.into_boxed_slice(),
            conflicts: conflicts.into_boxed_slice(),
        },
        source: source.to_vec().into_boxed_slice(),
        streams: parsed.streams,
        files: parsed.files.into_boxed_slice(),
        unsupported_metadata: parsed.unsupported_metadata,
        encoded_header,
    })
}

fn parse_plain_header(
    bytes: &[u8],
    source_base: usize,
    policy: SevenZImportPolicy,
    archive_fields: &mut Vec<LegacyFieldObservation<LegacyObservedValue>>,
) -> Result<ParsedHeader> {
    let mut cursor = ByteCursor::new(bytes, source_base);
    if cursor.byte("Header NID")? != K_HEADER {
        return Err(structure("decoded 7z metadata is not a plain Header"));
    }
    let mut streams = StreamsInfo::default();
    let mut files = Vec::new();
    let mut unsupported_metadata = BTreeSet::new();
    loop {
        match cursor.byte("Header property")? {
            K_END => break,
            K_ARCHIVE_PROPERTIES => parse_archive_properties(&mut cursor, archive_fields)?,
            K_ADDITIONAL_STREAMS_INFO => {
                return Err(unsupported(
                    "7z AdditionalStreamsInfo is unsupported in strict-v1",
                ));
            }
            K_MAIN_STREAMS_INFO => {
                streams = parse_streams_info(&mut cursor, policy, archive_fields)?;
            }
            K_FILES_INFO => {
                files = parse_files_info(&mut cursor, policy, &mut unsupported_metadata)?;
            }
            other => {
                return Err(structure(format!("unknown plain Header NID 0x{other:02x}")));
            }
        }
    }
    if cursor.remaining() != 0 {
        return Err(structure("plain 7z Header has trailing bytes"));
    }
    if files.iter().any(|file| !file.empty_stream) && streams.folders.is_empty() {
        return Err(irreconcilable(
            "7z files with streams exist without MainStreamsInfo",
        ));
    }
    Ok(ParsedHeader {
        streams,
        files,
        unsupported_metadata,
    })
}

fn parse_archive_properties(
    cursor: &mut ByteCursor<'_>,
    fields: &mut Vec<LegacyFieldObservation<LegacyObservedValue>>,
) -> Result<()> {
    let authority = authority("archive-properties", 0);
    loop {
        let id = cursor.byte("ArchiveProperties NID")?;
        if id == K_END {
            return Ok(());
        }
        let length = usize_from_u64(
            cursor.number("ArchiveProperties length")?,
            "property length",
        )?;
        let start = cursor.position;
        let value = cursor.bytes(length, "ArchiveProperties value")?;
        fields.push(field_bytes(
            &format!("archive.property-0x{id:02x}"),
            &authority,
            value,
            cursor.source_base + start,
        ));
    }
}

fn parse_streams_info(
    cursor: &mut ByteCursor<'_>,
    policy: SevenZImportPolicy,
    fields: &mut Vec<LegacyFieldObservation<LegacyObservedValue>>,
) -> Result<StreamsInfo> {
    let mut streams = StreamsInfo::default();
    let mut have_pack = false;
    let mut have_unpack = false;
    loop {
        match cursor.byte("StreamsInfo NID")? {
            K_END => break,
            K_PACK_INFO if !have_pack && !have_unpack => {
                parse_pack_info(cursor, policy, &mut streams, fields)?;
                have_pack = true;
            }
            K_UNPACK_INFO if !have_unpack => {
                parse_unpack_info(cursor, policy, &mut streams, fields)?;
                have_unpack = true;
            }
            K_SUBSTREAMS_INFO if have_unpack => {
                parse_substreams_info(cursor, policy, &mut streams)?;
            }
            other => {
                return Err(structure(format!(
                    "unexpected or duplicate StreamsInfo NID 0x{other:02x}"
                )));
            }
        }
    }
    if !streams.folders.is_empty() && streams.pack_sizes.is_empty() {
        return Err(irreconcilable("7z folders exist without packed streams"));
    }
    if streams.substreams.is_empty() && !streams.folders.is_empty() {
        streams.substreams = streams
            .folders
            .iter()
            .map(|folder| {
                Ok(vec![Substream {
                    size: folder_output_size(folder)?,
                    crc: folder.crc,
                }])
            })
            .collect::<Result<Vec<_>>>()?;
    }
    let required_pack_streams = streams
        .folders
        .iter()
        .map(|folder| folder.packed_indices.len())
        .sum::<usize>();
    if required_pack_streams != streams.pack_sizes.len() {
        return Err(irreconcilable(format!(
            "7z folder graph needs {required_pack_streams} packed streams but PackInfo declares {}",
            streams.pack_sizes.len()
        )));
    }
    Ok(streams)
}

fn parse_pack_info(
    cursor: &mut ByteCursor<'_>,
    policy: SevenZImportPolicy,
    streams: &mut StreamsInfo,
    fields: &mut Vec<LegacyFieldObservation<LegacyObservedValue>>,
) -> Result<()> {
    streams.pack_pos = cursor.number("PackInfo PackPos")?;
    let count = cursor.number("PackInfo NumPackStreams")?;
    policy_check(
        count <= policy.max_packed_streams,
        "7z packed stream count exceeds policy",
    )?;
    let count_usize = usize_from_u64(count, "packed stream count")?;
    let mut have_sizes = false;
    let mut have_crc = false;
    loop {
        match cursor.byte("PackInfo NID")? {
            K_END => break,
            K_SIZE if !have_sizes => {
                for _ in 0..count_usize {
                    let size = cursor.number("packed stream size")?;
                    policy_check(
                        size <= policy.max_packed_input_bytes,
                        "7z packed input exceeds policy",
                    )?;
                    streams.pack_sizes.push(size);
                }
                have_sizes = true;
            }
            K_CRC if !have_crc => {
                streams.pack_crcs = parse_digests(cursor, count_usize, "packed stream CRC")?;
                have_crc = true;
            }
            other => return Err(structure(format!("unexpected PackInfo NID 0x{other:02x}"))),
        }
    }
    if !have_sizes {
        return Err(irreconcilable("7z PackInfo omits packed stream sizes"));
    }
    if !have_crc {
        streams.pack_crcs = vec![None; count_usize];
    }
    fields.push(field_u64(
        "pack.position",
        &authority("pack-info", 0),
        streams.pack_pos,
        cursor.source_base,
        cursor.position,
    ));
    fields.push(field_u64(
        "pack.stream-count",
        &authority("pack-info", 0),
        count,
        cursor.source_base,
        cursor.position,
    ));
    Ok(())
}

fn parse_unpack_info(
    cursor: &mut ByteCursor<'_>,
    policy: SevenZImportPolicy,
    streams: &mut StreamsInfo,
    fields: &mut Vec<LegacyFieldObservation<LegacyObservedValue>>,
) -> Result<()> {
    if cursor.byte("UnpackInfo Folder NID")? != K_FOLDER {
        return Err(structure("UnpackInfo must begin with Folder"));
    }
    let folder_count = cursor.number("folder count")?;
    policy_check(
        folder_count <= policy.max_folders,
        "7z folder count exceeds policy",
    )?;
    if cursor.byte("external folder flag")? != 0 {
        return Err(unsupported(
            "external 7z folder definitions are unsupported",
        ));
    }
    for ordinal in 0..folder_count {
        streams.folders.push(parse_folder(cursor, policy, ordinal)?);
    }
    if cursor.byte("CodersUnpackSize NID")? != K_CODERS_UNPACK_SIZE {
        return Err(structure("UnpackInfo omits CodersUnpackSize"));
    }
    for folder in &mut streams.folders {
        let output_count = folder
            .coders
            .iter()
            .map(|coder| coder.output_count)
            .sum::<u64>();
        for _ in 0..output_count {
            let size = cursor.number("coder unpack size")?;
            policy_check(
                size <= policy.max_folder_decoded_bytes,
                "7z coder output exceeds policy",
            )?;
            folder.unpack_sizes.push(size);
        }
        validate_folder(folder, policy)?;
    }
    let nid = cursor.byte("UnpackInfo CRC or End")?;
    let end = if nid == K_CRC {
        let digests = parse_digests(cursor, streams.folders.len(), "folder CRC")?;
        for (folder, crc) in streams.folders.iter_mut().zip(digests) {
            folder.crc = crc;
        }
        cursor.byte("UnpackInfo End")?
    } else {
        nid
    };
    if end != K_END {
        return Err(structure("UnpackInfo has invalid terminator"));
    }
    fields.push(field_u64(
        "unpack.folder-count",
        &authority("unpack-info", 0),
        folder_count,
        cursor.source_base,
        cursor.position,
    ));
    Ok(())
}

fn parse_folder(
    cursor: &mut ByteCursor<'_>,
    policy: SevenZImportPolicy,
    _ordinal: u64,
) -> Result<Folder> {
    let coder_count = cursor.number("folder coder count")?;
    policy_check(
        coder_count > 0 && coder_count <= policy.max_coders_per_folder,
        "7z coder count exceeds policy",
    )?;
    let mut coders = Vec::new();
    let mut total_inputs = 0_u64;
    let mut total_outputs = 0_u64;
    for _ in 0..coder_count {
        let flags = cursor.byte("coder flags")?;
        if flags & 0xc0 != 0 {
            return Err(unsupported("alternative 7z coder methods are unsupported"));
        }
        let method_len = usize::from(flags & 0x0f);
        if method_len == 0 {
            return Err(structure("7z coder method ID is empty"));
        }
        let method = cursor
            .bytes(method_len, "coder method ID")?
            .to_vec()
            .into_boxed_slice();
        let complex = flags & 0x10 != 0;
        let input_count = if complex {
            cursor.number("coder input count")?
        } else {
            1
        };
        let output_count = if complex {
            cursor.number("coder output count")?
        } else {
            1
        };
        if input_count == 0 || output_count == 0 {
            return Err(structure("7z coder has zero streams"));
        }
        total_inputs = total_inputs
            .checked_add(input_count)
            .ok_or_else(|| structure("coder input count overflow"))?;
        total_outputs = total_outputs
            .checked_add(output_count)
            .ok_or_else(|| structure("coder output count overflow"))?;
        policy_check(
            total_inputs <= policy.max_coder_streams && total_outputs <= policy.max_coder_streams,
            "7z coder stream count exceeds policy",
        )?;
        let properties = if flags & 0x20 != 0 {
            let length = cursor.number("coder property length")?;
            policy_check(
                length <= policy.max_coder_property_bytes,
                "7z coder properties exceed policy",
            )?;
            cursor
                .bytes(
                    usize_from_u64(length, "coder property length")?,
                    "coder properties",
                )?
                .to_vec()
                .into_boxed_slice()
        } else {
            Box::default()
        };
        coders.push(Coder {
            method,
            properties,
            input_count,
            output_count,
            first_input: total_inputs - input_count,
            first_output: total_outputs - output_count,
        });
    }
    if total_outputs == 0 || total_inputs + 1 < total_outputs {
        return Err(irreconcilable("7z folder stream cardinality is impossible"));
    }
    let bind_count = total_outputs - 1;
    let mut bind_pairs = Vec::new();
    for _ in 0..bind_count {
        let input = cursor.number("bind input index")?;
        let output = cursor.number("bind output index")?;
        if input >= total_inputs || output >= total_outputs {
            return Err(irreconcilable("7z bind pair index is out of range"));
        }
        bind_pairs.push(BindPair { input, output });
    }
    let packed_count = total_inputs
        .checked_sub(bind_count)
        .ok_or_else(|| irreconcilable("7z packed stream count underflow"))?;
    if packed_count == 0 {
        return Err(irreconcilable("7z folder has no packed input"));
    }
    let packed_indices = if packed_count == 1 {
        let bound = bind_pairs
            .iter()
            .map(|pair| pair.input)
            .collect::<BTreeSet<_>>();
        let values = (0..total_inputs)
            .filter(|index| !bound.contains(index))
            .collect::<Vec<_>>();
        if values.len() != 1 {
            return Err(irreconcilable("7z folder packed input cannot be inferred"));
        }
        values
    } else {
        let mut values = Vec::new();
        for _ in 0..packed_count {
            values.push(cursor.number("packed input index")?);
        }
        values
    };
    Ok(Folder {
        coders,
        bind_pairs,
        packed_indices,
        unpack_sizes: Vec::new(),
        crc: None,
    })
}

fn validate_folder(folder: &Folder, policy: SevenZImportPolicy) -> Result<()> {
    if folder.packed_indices.len() != 1 {
        return Err(unsupported(
            "strict-v1 supports one packed input per 7z folder",
        ));
    }
    if folder
        .coders
        .iter()
        .any(|coder| coder.input_count != 1 || coder.output_count != 1)
    {
        return Err(unsupported("strict-v1 supports only 1-in/1-out 7z coders"));
    }
    let total = u64::try_from(folder.coders.len()).unwrap_or(u64::MAX);
    if u64::try_from(folder.bind_pairs.len()).unwrap_or(u64::MAX) + 1 != total {
        return Err(irreconcilable(
            "7z linear folder has the wrong number of bind pairs",
        ));
    }
    let order = folder_order(folder)?;
    let mut core = 0_u8;
    let mut filter = 0_u8;
    for (position, index) in order.iter().enumerate() {
        let coder = &folder.coders[*index];
        if is_core(&coder.method) {
            core = core.saturating_add(1);
            if position != 0 {
                return Err(unsupported(
                    "7z core codec must consume the packed input directly",
                ));
            }
            validate_coder_properties(coder, policy)?;
        } else if is_filter(&coder.method) {
            filter = filter.saturating_add(1);
            if position == 0 {
                return Err(unsupported(
                    "7z filter cannot consume a packed stream directly",
                ));
            }
            validate_coder_properties(coder, policy)?;
        } else {
            return Err(unsupported_coder(&coder.method));
        }
    }
    if core != 1 || filter > 1 {
        return Err(unsupported(
            "strict-v1 requires one core codec and at most one simple filter",
        ));
    }
    let size = folder_output_size(folder)?;
    policy_check(
        size <= policy.max_folder_decoded_bytes,
        "7z folder output exceeds policy",
    )?;
    Ok(())
}

fn folder_order(folder: &Folder) -> Result<Vec<usize>> {
    let mut input_to_coder = BTreeMap::new();
    for (index, coder) in folder.coders.iter().enumerate() {
        if input_to_coder.insert(coder.first_input, index).is_some() {
            return Err(irreconcilable("7z input stream is multiply owned"));
        }
    }
    let mut output_to_input = BTreeMap::new();
    for pair in &folder.bind_pairs {
        if output_to_input.insert(pair.output, pair.input).is_some() {
            return Err(irreconcilable("7z output stream is multiply bound"));
        }
    }
    let mut input = folder.packed_indices[0];
    let mut seen = BTreeSet::new();
    let mut order = Vec::new();
    loop {
        let index = *input_to_coder
            .get(&input)
            .ok_or_else(|| irreconcilable("7z folder has a missing input stream"))?;
        if !seen.insert(index) {
            return Err(irreconcilable("7z folder coder graph contains a cycle"));
        }
        order.push(index);
        let output = folder.coders[index].first_output;
        let Some(next_input) = output_to_input.get(&output).copied() else {
            break;
        };
        input = next_input;
    }
    if order.len() != folder.coders.len() {
        return Err(irreconcilable("7z folder coder graph is disconnected"));
    }
    Ok(order)
}

fn validate_coder_properties(coder: &Coder, policy: SevenZImportPolicy) -> Result<()> {
    match coder.method.as_ref() {
        METHOD_COPY | METHOD_BZIP2 | METHOD_DEFLATE => {
            if !coder.properties.is_empty() {
                return Err(structure("7z coder has forbidden properties"));
            }
        }
        METHOD_LZMA => {
            if coder.properties.len() != 5 {
                return Err(structure("7z LZMA properties must be exactly 5 bytes"));
            }
            let dictionary = u64::from(le_u32(&coder.properties[1..5]));
            policy_check(
                dictionary <= policy.max_dictionary_bytes,
                "7z LZMA dictionary exceeds policy",
            )?;
        }
        METHOD_LZMA2 => {
            if coder.properties.len() != 1 || coder.properties[0] > 40 {
                return Err(structure("7z LZMA2 properties are invalid"));
            }
            let dictionary = u64::from(lzma2_dictionary(coder.properties[0])?);
            policy_check(
                dictionary <= policy.max_dictionary_bytes,
                "7z LZMA2 dictionary exceeds policy",
            )?;
        }
        METHOD_DELTA => {
            if coder.properties.len() != 1 {
                return Err(structure("7z Delta properties must be exactly one byte"));
            }
        }
        METHOD_BCJ_X86 => {
            if !coder.properties.is_empty() && coder.properties.len() != 4 {
                return Err(structure("7z x86 BCJ properties must be empty or u32le"));
            }
        }
        _ => return Err(unsupported_coder(&coder.method)),
    }
    Ok(())
}

fn parse_substreams_info(
    cursor: &mut ByteCursor<'_>,
    policy: SevenZImportPolicy,
    streams: &mut StreamsInfo,
) -> Result<()> {
    let folder_count = streams.folders.len();
    let mut counts = vec![1_u64; folder_count];
    let mut sizes: Option<Vec<Vec<u64>>> = None;
    let mut digests: Option<Vec<Option<u32>>> = None;
    loop {
        match cursor.byte("SubStreamsInfo NID")? {
            K_END => break,
            K_NUM_UNPACK_STREAM => {
                for count in &mut counts {
                    *count = cursor.number("folder substream count")?;
                    policy_check(
                        *count <= policy.max_substreams_per_folder,
                        "7z substreams per folder exceed policy",
                    )?;
                }
            }
            K_SIZE => {
                let mut all = Vec::with_capacity(folder_count);
                for (folder, count) in streams.folders.iter().zip(&counts) {
                    let total = folder_output_size(folder)?;
                    let mut folder_sizes = Vec::new();
                    let mut used = 0_u64;
                    for _ in 0..count.saturating_sub(1) {
                        let size = cursor.number("substream size")?;
                        used = used
                            .checked_add(size)
                            .ok_or_else(|| irreconcilable("substream sizes overflow"))?;
                        if used > total {
                            return Err(irreconcilable("substream sizes exceed folder output"));
                        }
                        folder_sizes.push(size);
                    }
                    if *count > 0 {
                        folder_sizes.push(total - used);
                    } else if total != 0 {
                        return Err(irreconcilable(
                            "zero substreams leave unexplained folder bytes",
                        ));
                    }
                    all.push(folder_sizes);
                }
                sizes = Some(all);
            }
            K_CRC => {
                let crc_count = streams
                    .folders
                    .iter()
                    .zip(&counts)
                    .map(|(folder, count)| {
                        if *count == 1 && folder.crc.is_some() {
                            0
                        } else {
                            *count
                        }
                    })
                    .sum::<u64>();
                digests = Some(parse_digests(
                    cursor,
                    usize_from_u64(crc_count, "substream CRC count")?,
                    "substream CRC",
                )?);
            }
            other => {
                return Err(structure(format!(
                    "unexpected SubStreamsInfo NID 0x{other:02x}"
                )));
            }
        }
    }
    let mut crc_cursor = 0_usize;
    let sizes = sizes.unwrap_or_else(|| {
        streams
            .folders
            .iter()
            .zip(&counts)
            .map(|(folder, count)| {
                if *count == 1 {
                    vec![folder_output_size(folder).unwrap_or(0)]
                } else {
                    Vec::new()
                }
            })
            .collect()
    });
    if sizes
        .iter()
        .zip(&counts)
        .any(|(value, count)| value.len() != usize::try_from(*count).unwrap_or(usize::MAX))
    {
        return Err(irreconcilable("SubStreamsInfo omits required sizes"));
    }
    let digest_values = digests;
    streams.substreams.clear();
    for ((folder, count), folder_sizes) in streams.folders.iter().zip(counts).zip(sizes) {
        let mut values = Vec::new();
        for size in folder_sizes {
            let crc = if count == 1 && folder.crc.is_some() {
                folder.crc
            } else if let Some(values) = &digest_values {
                let value = values.get(crc_cursor).copied().flatten();
                crc_cursor += 1;
                value
            } else {
                None
            };
            values.push(Substream { size, crc });
        }
        streams.substreams.push(values);
    }
    if digest_values
        .as_ref()
        .is_some_and(|values| crc_cursor != values.len())
    {
        return Err(irreconcilable("SubStreamsInfo has unused CRC declarations"));
    }
    Ok(())
}

fn parse_digests(
    cursor: &mut ByteCursor<'_>,
    count: usize,
    what: &str,
) -> Result<Vec<Option<u32>>> {
    let all_defined = cursor.byte(what)? != 0;
    let defined = if all_defined {
        vec![true; count]
    } else {
        read_bits(cursor, count, what)?
    };
    let mut values = Vec::with_capacity(count);
    for is_defined in defined {
        values.push(if is_defined {
            Some(le_u32(cursor.bytes(4, what)?))
        } else {
            None
        });
    }
    Ok(values)
}

fn parse_files_info(
    cursor: &mut ByteCursor<'_>,
    policy: SevenZImportPolicy,
    unsupported_metadata: &mut BTreeSet<String>,
) -> Result<Vec<ObservedFile>> {
    let count = cursor.number("FilesInfo file count")?;
    policy_check(count <= policy.max_files, "7z file count exceeds policy")?;
    let count_usize = usize_from_u64(count, "file count")?;
    let mut files = vec![ObservedFile::default(); count_usize];
    let mut empty_stream = vec![false; count_usize];
    let mut empty_file_values: Option<Vec<bool>> = None;
    let mut anti_values: Option<Vec<bool>> = None;
    loop {
        let id = cursor.byte("FilesInfo property")?;
        if id == K_END {
            break;
        }
        let length = usize_from_u64(
            cursor.number("FilesInfo property length")?,
            "FilesInfo property length",
        )?;
        let property_start = cursor.position;
        let property = cursor.bytes(length, "FilesInfo property")?;
        let mut nested = ByteCursor::new(property, cursor.source_base + property_start);
        match id {
            K_NAME => {
                if nested.byte("Name external flag")? != 0 {
                    return Err(unsupported("external 7z file names are unsupported"));
                }
                let names =
                    parse_names(nested.bytes(nested.remaining(), "7z names")?, count_usize)?;
                for (ordinal, (file, name)) in files.iter_mut().zip(names).enumerate() {
                    file.fields.push(field_text(
                        "path",
                        &authority(
                            "files-info-name",
                            u64::try_from(ordinal).unwrap_or(u64::MAX),
                        ),
                        &name,
                        nested.source_base,
                        property.len(),
                    ));
                    file.name = Some(name);
                }
                nested.position = nested.bytes.len();
            }
            K_EMPTY_STREAM => {
                empty_stream = read_bits(&mut nested, count_usize, "EmptyStream bitmap")?;
            }
            K_EMPTY_FILE => {
                let empty_count = empty_stream.iter().filter(|value| **value).count();
                empty_file_values = Some(read_bits(&mut nested, empty_count, "EmptyFile bitmap")?);
            }
            K_ANTI => {
                let empty_count = empty_stream.iter().filter(|value| **value).count();
                anti_values = Some(read_bits(&mut nested, empty_count, "Anti bitmap")?);
            }
            K_CTIME => {
                assign_u64_property(
                    &mut nested,
                    &mut files,
                    |file, value| file.ctime = Some(value),
                    "ctime",
                )?;
                unsupported_metadata.insert("legacy.7z.creation-time".to_owned());
            }
            K_ATIME => {
                assign_u64_property(
                    &mut nested,
                    &mut files,
                    |file, value| file.atime = Some(value),
                    "atime",
                )?;
                unsupported_metadata.insert("legacy.7z.access-time".to_owned());
            }
            K_MTIME => {
                assign_u64_property(
                    &mut nested,
                    &mut files,
                    |file, value| file.mtime = Some(value),
                    "mtime",
                )?;
            }
            K_WIN_ATTRIBUTES => {
                assign_u32_property(
                    &mut nested,
                    &mut files,
                    |file, value| file.attributes = Some(value),
                    "windows-attributes",
                )?;
                unsupported_metadata.insert("legacy.7z.windows-attributes".to_owned());
            }
            K_START_POS => {
                consume_defined_u64(&mut nested, count_usize, "start-position")?;
                unsupported_metadata.insert("legacy.7z.start-position".to_owned());
            }
            K_DUMMY => {
                if property.iter().any(|byte| *byte != 0) {
                    return Err(structure("7z Dummy property contains nonzero bytes"));
                }
                nested.position = nested.bytes.len();
            }
            _ => {
                unsupported_metadata.insert(format!("legacy.7z.files-property-0x{id:02x}"));
                for (ordinal, file) in files.iter_mut().enumerate() {
                    file.fields.push(field_bytes(
                        &format!("files-property-0x{id:02x}"),
                        &authority(
                            "files-info-unknown-property",
                            u64::try_from(ordinal).unwrap_or(u64::MAX),
                        ),
                        property,
                        nested.source_base,
                    ));
                }
                nested.position = nested.bytes.len();
            }
        }
        if nested.remaining() != 0 {
            return Err(structure(format!(
                "7z FilesInfo property 0x{id:02x} has trailing bytes"
            )));
        }
    }
    let mut empty_index = 0_usize;
    for (ordinal, file) in files.iter_mut().enumerate() {
        file.empty_stream = empty_stream[ordinal];
        if file.empty_stream {
            file.empty_file = empty_file_values
                .as_ref()
                .and_then(|values| values.get(empty_index))
                .copied()
                .unwrap_or(false);
            file.anti = anti_values
                .as_ref()
                .and_then(|values| values.get(empty_index))
                .copied()
                .unwrap_or(false);
            empty_index += 1;
        }
        let auth = authority(
            "files-info-bitmap",
            u64::try_from(ordinal).unwrap_or(u64::MAX),
        );
        file.fields.push(field_bool(
            "empty-stream",
            &auth,
            file.empty_stream,
            cursor.source_base,
            0,
        ));
        file.fields.push(field_bool(
            "empty-file",
            &auth,
            file.empty_file,
            cursor.source_base,
            0,
        ));
        file.fields
            .push(field_bool("anti", &auth, file.anti, cursor.source_base, 0));
    }
    if files.iter().any(|file| file.name.is_none()) {
        return Err(irreconcilable("7z FilesInfo omits one or more names"));
    }
    Ok(files)
}

fn parse_names(bytes: &[u8], count: usize) -> Result<Vec<String>> {
    if !bytes.len().is_multiple_of(2) {
        return Err(structure("7z Name property is not aligned UTF-16LE"));
    }
    let units = bytes.chunks_exact(2).map(le_u16).collect::<Vec<_>>();
    let mut names = Vec::new();
    let mut start = 0_usize;
    for (index, unit) in units.iter().enumerate() {
        if *unit == 0 {
            let name = String::from_utf16(&units[start..index])
                .map_err(|_| unsafe_path("7z filename contains malformed UTF-16"))?;
            if name.chars().any(|character| character == '\0') {
                return Err(unsafe_path("7z filename contains embedded NUL"));
            }
            names.push(name);
            start = index + 1;
        }
    }
    if start != units.len() || names.len() != count {
        return Err(irreconcilable(
            "7z Name property count does not match FilesInfo",
        ));
    }
    Ok(names)
}

fn assign_u64_property<F>(
    cursor: &mut ByteCursor<'_>,
    files: &mut [ObservedFile],
    mut assign: F,
    field: &str,
) -> Result<()>
where
    F: FnMut(&mut ObservedFile, u64),
{
    let defined = read_defined_vector(cursor, files.len(), field)?;
    if cursor.byte("external property flag")? != 0 {
        return Err(unsupported(format!(
            "external 7z {field} property is unsupported"
        )));
    }
    for (ordinal, (file, is_defined)) in files.iter_mut().zip(defined).enumerate() {
        if is_defined {
            let start = cursor.position;
            let value = le_u64(cursor.bytes(8, field)?);
            assign(file, value);
            file.fields.push(field_u64(
                field,
                &authority(
                    "files-info-property",
                    u64::try_from(ordinal).unwrap_or(u64::MAX),
                ),
                value,
                cursor.source_base + start,
                8,
            ));
        }
    }
    Ok(())
}

fn assign_u32_property<F>(
    cursor: &mut ByteCursor<'_>,
    files: &mut [ObservedFile],
    mut assign: F,
    field: &str,
) -> Result<()>
where
    F: FnMut(&mut ObservedFile, u32),
{
    let defined = read_defined_vector(cursor, files.len(), field)?;
    if cursor.byte("external property flag")? != 0 {
        return Err(unsupported(format!(
            "external 7z {field} property is unsupported"
        )));
    }
    for (ordinal, (file, is_defined)) in files.iter_mut().zip(defined).enumerate() {
        if is_defined {
            let start = cursor.position;
            let value = le_u32(cursor.bytes(4, field)?);
            assign(file, value);
            file.fields.push(field_u64(
                field,
                &authority(
                    "files-info-property",
                    u64::try_from(ordinal).unwrap_or(u64::MAX),
                ),
                u64::from(value),
                cursor.source_base + start,
                4,
            ));
        }
    }
    Ok(())
}

fn consume_defined_u64(cursor: &mut ByteCursor<'_>, count: usize, field: &str) -> Result<()> {
    let defined = read_defined_vector(cursor, count, field)?;
    if cursor.byte("external property flag")? != 0 {
        return Err(unsupported(format!(
            "external 7z {field} property is unsupported"
        )));
    }
    for value in defined {
        if value {
            cursor.bytes(8, field)?;
        }
    }
    Ok(())
}

fn read_defined_vector(cursor: &mut ByteCursor<'_>, count: usize, what: &str) -> Result<Vec<bool>> {
    if cursor.byte(what)? != 0 {
        Ok(vec![true; count])
    } else {
        read_bits(cursor, count, what)
    }
}

fn read_bits(cursor: &mut ByteCursor<'_>, count: usize, what: &str) -> Result<Vec<bool>> {
    let mut values = Vec::with_capacity(count);
    let mut current = 0_u8;
    let mut mask = 0_u8;
    for _ in 0..count {
        if mask == 0 {
            current = cursor.byte(what)?;
            mask = 0x80;
        }
        values.push(current & mask != 0);
        mask >>= 1;
    }
    Ok(values)
}

fn decode_streams(
    source: &[u8],
    streams: &StreamsInfo,
    policy: SevenZImportPolicy,
    header_mode: bool,
) -> Result<Vec<Box<[u8]>>> {
    let pack_start = 32_u64
        .checked_add(streams.pack_pos)
        .ok_or_else(|| structure("7z packed-data base overflow"))?;
    let mut pack_offsets = Vec::with_capacity(streams.pack_sizes.len());
    let mut offset = pack_start;
    for (index, size) in streams.pack_sizes.iter().copied().enumerate() {
        let end = offset
            .checked_add(size)
            .ok_or_else(|| structure("7z packed stream extent overflow"))?;
        if end > u64::try_from(source.len()).unwrap_or(u64::MAX) {
            return Err(truncated("7z packed stream is truncated"));
        }
        let start_usize = usize_from_u64(offset, "packed stream offset")?;
        let size_usize = usize_from_u64(size, "packed stream size")?;
        let packed = &source[start_usize..start_usize + size_usize];
        if let Some(expected) = streams.pack_crcs.get(index).copied().flatten()
            && crc32fast::hash(packed) != expected
        {
            return Err(integrity(format!("7z packed stream {index} CRC mismatch")));
        }
        pack_offsets.push((start_usize, size_usize));
        offset = end;
    }
    let mut pack_ordinal = 0_usize;
    let mut output = Vec::new();
    let mut total = 0_u64;
    for (folder_index, folder) in streams.folders.iter().enumerate() {
        let (start, length) = *pack_offsets
            .get(pack_ordinal)
            .ok_or_else(|| irreconcilable("folder has no corresponding packed stream"))?;
        pack_ordinal += folder.packed_indices.len();
        let expected = folder_output_size(folder)?;
        let limit = if header_mode {
            policy.max_decoded_header_bytes
        } else {
            policy.max_folder_decoded_bytes
        };
        policy_check(expected <= limit, "7z folder output exceeds decode policy")?;
        let packed_length = u64::try_from(length).unwrap_or(u64::MAX);
        if packed_length == 0 {
            if expected != 0 {
                return Err(irreconcilable("nonempty 7z folder has empty packed input"));
            }
        } else {
            let maximum = packed_length
                .checked_mul(policy.max_expansion_ratio)
                .ok_or_else(|| policy_error("7z expansion-ratio bound overflow"))?;
            policy_check(
                expected <= maximum,
                "7z folder expansion ratio exceeds policy",
            )?;
        }
        let decoded = decode_folder(folder, &source[start..start + length], expected, policy)?;
        if let Some(expected_crc) = folder.crc
            && crc32fast::hash(&decoded) != expected_crc
        {
            return Err(integrity(format!("7z folder {folder_index} CRC mismatch")));
        }
        let mut cursor = 0_usize;
        for (sub_index, substream) in streams
            .substreams
            .get(folder_index)
            .ok_or_else(|| irreconcilable("folder has no SubStreamsInfo mapping"))?
            .iter()
            .enumerate()
        {
            let size = usize_from_u64(substream.size, "substream size")?;
            let end = cursor
                .checked_add(size)
                .ok_or_else(|| irreconcilable("substream extent overflow"))?;
            let bytes = decoded
                .get(cursor..end)
                .ok_or_else(|| irreconcilable("substream extent exceeds folder output"))?;
            if let Some(expected_crc) = substream.crc
                && crc32fast::hash(bytes) != expected_crc
            {
                return Err(integrity(format!(
                    "7z folder {folder_index} substream {sub_index} CRC mismatch"
                )));
            }
            total = total
                .checked_add(substream.size)
                .ok_or_else(|| policy_error("7z total decoded bytes overflow"))?;
            policy_check(
                total <= policy.max_total_decoded_bytes,
                "7z total decoded bytes exceed policy",
            )?;
            output.push(bytes.to_vec().into_boxed_slice());
            cursor = end;
        }
        if cursor != decoded.len() {
            return Err(irreconcilable("7z folder leaves unexplained decoded bytes"));
        }
    }
    if pack_ordinal != streams.pack_sizes.len() {
        return Err(irreconcilable("7z packed streams remain unused"));
    }
    Ok(output)
}

fn validate_pack_extent(streams: &StreamsInfo, expected_end: u64) -> Result<u64> {
    let start = 32_u64
        .checked_add(streams.pack_pos)
        .ok_or_else(|| structure("7z packed-data base overflow"))?;
    let end = streams.pack_sizes.iter().try_fold(start, |offset, size| {
        offset
            .checked_add(*size)
            .ok_or_else(|| structure("7z packed-data extent overflow"))
    })?;
    if end != expected_end {
        return Err(irreconcilable(format!(
            "7z packed streams end at {end}, expected {expected_end}; gap or overlap is unexplained"
        )));
    }
    Ok(start)
}

fn decode_folder(
    folder: &Folder,
    packed: &[u8],
    expected: u64,
    policy: SevenZImportPolicy,
) -> Result<Vec<u8>> {
    let order = folder_order(folder)?;
    let core = &folder.coders[order[0]];
    let mut decoded = decode_core(core, packed, expected)?;
    if let Some(filter_index) = order.get(1) {
        apply_filter(&folder.coders[*filter_index], &mut decoded)?;
    }
    if u64::try_from(decoded.len()).unwrap_or(u64::MAX) != expected {
        return Err(irreconcilable("7z folder decoded length mismatch"));
    }
    let packed_len = u64::try_from(packed.len()).unwrap_or(u64::MAX);
    if packed_len > 0 {
        let maximum = packed_len
            .checked_mul(policy.max_expansion_ratio)
            .ok_or_else(|| policy_error("7z expansion-ratio bound overflow"))?;
        policy_check(
            expected <= maximum,
            "7z folder expansion ratio exceeds policy",
        )?;
    } else if expected != 0 {
        return Err(irreconcilable("nonempty 7z folder has empty packed input"));
    }
    Ok(decoded)
}

fn decode_core(coder: &Coder, packed: &[u8], expected: u64) -> Result<Vec<u8>> {
    let capacity = usize_from_u64(expected, "7z decoded size")?;
    match coder.method.as_ref() {
        METHOD_COPY => {
            if u64::try_from(packed.len()).unwrap_or(u64::MAX) != expected {
                return Err(irreconcilable("7z COPY packed and unpacked sizes disagree"));
            }
            Ok(packed.to_vec())
        }
        METHOD_LZMA => {
            let props = coder.properties[0];
            let dictionary = le_u32(&coder.properties[1..5]);
            let cursor = Cursor::new(packed);
            let mut reader =
                lzma_rust2::LzmaReader::new_with_props(cursor, expected, props, dictionary, None)
                    .map_err(|error| structure(format!("initialize 7z LZMA decoder: {error}")))?;
            let mut output = Vec::with_capacity(capacity);
            reader
                .by_ref()
                .take(expected.saturating_add(1))
                .read_to_end(&mut output)
                .map_err(|error| integrity(format!("decode 7z LZMA stream: {error}")))?;
            if reader.into_inner().position() != u64::try_from(packed.len()).unwrap_or(u64::MAX) {
                return Err(irreconcilable("7z LZMA stream has trailing bytes"));
            }
            Ok(output)
        }
        METHOD_LZMA2 => {
            let dictionary = lzma2_dictionary(coder.properties[0])?;
            let cursor = Cursor::new(packed);
            let mut reader = lzma_rust2::Lzma2Reader::new(cursor, dictionary, None);
            let mut output = Vec::with_capacity(capacity);
            reader
                .by_ref()
                .take(expected.saturating_add(1))
                .read_to_end(&mut output)
                .map_err(|error| integrity(format!("decode 7z LZMA2 stream: {error}")))?;
            if reader.into_inner().position() != u64::try_from(packed.len()).unwrap_or(u64::MAX) {
                return Err(irreconcilable("7z LZMA2 stream has trailing bytes"));
            }
            Ok(output)
        }
        METHOD_DEFLATE => {
            let mut reader = DeflateDecoder::new(packed);
            let mut output = Vec::with_capacity(capacity);
            reader
                .by_ref()
                .take(expected.saturating_add(1))
                .read_to_end(&mut output)
                .map_err(|error| integrity(format!("decode 7z DEFLATE stream: {error}")))?;
            if reader.total_in() != u64::try_from(packed.len()).unwrap_or(u64::MAX) {
                return Err(irreconcilable("7z DEFLATE stream has trailing bytes"));
            }
            Ok(output)
        }
        METHOD_BZIP2 => {
            let cursor = Cursor::new(packed);
            let mut reader = BzDecoder::new(cursor);
            let mut output = Vec::with_capacity(capacity);
            reader
                .by_ref()
                .take(expected.saturating_add(1))
                .read_to_end(&mut output)
                .map_err(|error| integrity(format!("decode 7z BZip2 stream: {error}")))?;
            if reader.into_inner().position() != u64::try_from(packed.len()).unwrap_or(u64::MAX) {
                return Err(irreconcilable("7z BZip2 stream has trailing bytes"));
            }
            Ok(output)
        }
        _ => Err(unsupported_coder(&coder.method)),
    }
}

fn apply_filter(coder: &Coder, bytes: &mut [u8]) -> Result<()> {
    let config = match coder.method.as_ref() {
        METHOD_DELTA => lzma_rust2::FilterConfig::new_delta(u32::from(coder.properties[0]) + 1),
        METHOD_BCJ_X86 => {
            let start = if coder.properties.is_empty() {
                0
            } else {
                le_u32(&coder.properties)
            };
            lzma_rust2::FilterConfig::new_bcj_x86(start)
        }
        _ => return Err(unsupported_coder(&coder.method)),
    };
    let mut filter = lzma_rust2::filter::StreamFilter::new(&config)
        .map_err(|error| structure(format!("initialize 7z filter: {error}")))?;
    let _settled = filter.decode(bytes);
    filter.finish();
    Ok(())
}

pub fn resolve_strict(
    observation: SevenZObservation,
    policy: SevenZImportPolicy,
    profile: CompressionProfile,
) -> Result<SevenZImportResult> {
    let decoded = decode_streams(&observation.source, &observation.streams, policy, false)?;
    let stream_file_count = observation
        .files
        .iter()
        .filter(|file| !file.empty_stream)
        .count();
    if stream_file_count != decoded.len() {
        return Err(irreconcilable(format!(
            "FilesInfo maps {stream_file_count} files to {} SubStreamsInfo streams",
            decoded.len()
        )));
    }
    let mut stream_iter = decoded.into_iter();
    let mut resolved = Vec::with_capacity(observation.files.len());
    let mut total = 0_u64;
    for file in &observation.files {
        if file.anti {
            return Err(unsupported(
                "7z anti-items/deletion markers are unsupported",
            ));
        }
        let name = file
            .name
            .as_deref()
            .ok_or_else(|| irreconcilable("7z file has no name"))?;
        let (path, components) = logical_path(name)?;
        let attribute_directory = file.attributes.map(|value| value & 0x10 != 0);
        reject_unsupported_attribute_kind(file.attributes, &path)?;
        let directory = file.empty_stream && !file.empty_file;
        if let Some(marked_directory) = attribute_directory
            && marked_directory != directory
        {
            return Err(divergence(format!(
                "7z EmptyStream/EmptyFile and attributes disagree on entry kind for {path}"
            )));
        }
        if !file.empty_stream && attribute_directory == Some(true) {
            return Err(divergence(format!(
                "stream-bearing 7z entry {path} is marked directory"
            )));
        }
        let plaintext = if file.empty_stream {
            Box::default()
        } else {
            stream_iter
                .next()
                .ok_or_else(|| irreconcilable("7z file stream is missing"))?
        };
        policy_check(
            u64::try_from(plaintext.len()).unwrap_or(u64::MAX) <= policy.max_single_file_bytes,
            "7z single file exceeds policy",
        )?;
        if !directory {
            total = total
                .checked_add(u64::try_from(plaintext.len()).unwrap_or(u64::MAX))
                .ok_or_else(|| policy_error("7z total file bytes overflow"))?;
            policy_check(
                total <= policy.max_total_decoded_bytes,
                "7z total file bytes exceed policy",
            )?;
        }
        let executable = executable_from_attributes(file.attributes);
        let mtime = file
            .mtime
            .map(filetime_timestamp)
            .transpose()?
            .unwrap_or(Timestamp::new(0, 0, TimestampPrecision::Second, false)?);
        resolved.push(ResolvedFile {
            path,
            components,
            directory,
            executable,
            mtime,
            plaintext,
        });
    }
    if stream_iter.next().is_some() {
        return Err(irreconcilable("7z has unassigned decoded substreams"));
    }

    let mut by_path = BTreeMap::<LogicalPath, ResolvedFile>::new();
    for file in resolved {
        if by_path.insert(file.path.clone(), file).is_some() {
            return Err(Diagnostic::new(
                OutcomeClass::Nonconforming,
                ReasonCode::DuplicateLogicalPath,
                "duplicate reconciled 7z path",
            ));
        }
    }
    let mut kinds = by_path
        .iter()
        .map(|(path, file)| (path.clone(), file.directory))
        .collect::<BTreeMap<_, _>>();
    let mut synthesized = BTreeSet::new();
    let mut resolutions = observation
        .lom
        .conflicts
        .iter()
        .filter_map(conflict_resolution)
        .collect::<Vec<_>>();
    for file in by_path.values() {
        for depth in 1..file.components.len() {
            let ancestor = LogicalPath::from_utf8(&file.components[..depth])?;
            match kinds.get(&ancestor) {
                Some(true) => {}
                Some(false) => {
                    return Err(Diagnostic::new(
                        OutcomeClass::Nonconforming,
                        ReasonCode::FileAsAncestor,
                        format!("7z file {ancestor} is an ancestor"),
                    ));
                }
                None => {
                    kinds.insert(ancestor.clone(), true);
                    synthesized.insert(ancestor.clone());
                    resolutions.push(ConversionResolution {
                        conflict_class: ConflictClass::Omission.as_str().to_owned(),
                        semantic_field: format!("directory:{ancestor}"),
                        authorities: Box::from(["7z child paths".to_owned()]),
                        observed_values: Box::from(["directory entry omitted".to_owned()]),
                        action: "synthesized explicit ancestor required by EAM".to_owned(),
                    });
                }
            }
        }
    }
    let folder_count = u64::try_from(observation.streams.folders.len()).unwrap_or(u64::MAX);
    let solid_folder_count = observation
        .streams
        .substreams
        .iter()
        .filter(|streams| streams.len() > 1)
        .count()
        .try_into()
        .unwrap_or(u64::MAX);
    let coder_count = observation
        .streams
        .folders
        .iter()
        .map(|folder| folder.coders.len())
        .sum::<usize>()
        .try_into()
        .unwrap_or(u64::MAX);
    resolutions.push(ConversionResolution {
        conflict_class: ConflictClass::Refinement.as_str().to_owned(),
        semantic_field: "7z.structure-summary".to_owned(),
        authorities: Box::from([
            "PackInfo".to_owned(),
            "UnpackInfo".to_owned(),
            "SubStreamsInfo".to_owned(),
            "FilesInfo".to_owned(),
        ]),
        observed_values: Box::from([
            format!("folders={folder_count}"),
            format!("solid-folders={solid_folder_count}"),
            format!("coders={coder_count}"),
        ]),
        action: "validated folder graph and mapped every decoded substream exactly once".to_owned(),
    });
    resolutions.sort();
    resolutions.dedup();
    policy_check(
        u64::try_from(resolutions.len()).unwrap_or(u64::MAX) <= policy.max_resolutions,
        "7z resolutions exceed policy",
    )?;

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
    for item in by_path.into_values() {
        let metadata = MetadataSet::new(vec![
            MetadataItem::executable(item.executable),
            MetadataItem::mtime(item.mtime),
        ])?;
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
    let unsupported = observation
        .unsupported_metadata
        .into_iter()
        .collect::<Vec<_>>();
    let provenance = ConversionProvenance {
        source_format: "7z".to_owned(),
        adapter_id: "entrybound/sevenz-strict-v1".to_owned(),
        source_digest: observation.lom.source_digest,
        import_mode: "strict".to_owned(),
        source_entry_count: u64::try_from(observation.files.len()).unwrap_or(u64::MAX),
        observation_count: observation.lom.observation_count(),
        omission_count: count_resolution(&resolutions, ConflictClass::Omission),
        refinement_count: count_resolution(&resolutions, ConflictClass::Refinement),
        divergence_count: observation.lom.conflict_count(ConflictClass::Divergence),
        irreconcilable_count: observation
            .lom
            .conflict_count(ConflictClass::Irreconcilable),
        resolutions: resolutions.clone().into_boxed_slice(),
        synthesized_ancestors: synthesized
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .into_boxed_slice(),
        unsupported_metadata: unsupported.clone().into_boxed_slice(),
        outcome: "success".to_owned(),
    };
    let archive = plan_observed_archive(
        entries,
        files,
        sevenz_fidelity(&unsupported),
        provenance,
        None,
        profile,
    )?;
    Ok(SevenZImportResult {
        archive,
        report: SevenZConversionReport {
            observation: observation.lom,
            resolutions: resolutions.into_boxed_slice(),
            synthesized_ancestors: synthesized
                .into_iter()
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            folder_count,
            solid_folder_count,
            coder_count,
            encoded_header: observation.encoded_header,
        },
    })
}

pub fn import_strict(
    source: &[u8],
    policy: SevenZImportPolicy,
    profile: CompressionProfile,
) -> Result<SevenZImportResult> {
    resolve_strict(observe(source, policy)?, policy, profile)
}

fn executable_from_attributes(attributes: Option<u32>) -> bool {
    let Some(attributes) = attributes else {
        return false;
    };
    let mode = attributes >> 16;
    let kind = mode & 0o170000;
    matches!(kind, 0 | 0o040000 | 0o100000) && mode & 0o111 != 0
}

fn reject_unsupported_attribute_kind(attributes: Option<u32>, path: &LogicalPath) -> Result<()> {
    let Some(attributes) = attributes else {
        return Ok(());
    };
    if attributes & 0x400 != 0 {
        return Err(unsupported(format!(
            "7z reparse/junction entry {path} is unsupported"
        )));
    }
    let kind = (attributes >> 16) & 0o170000;
    match kind {
        0 | 0o040000 | 0o100000 => Ok(()),
        0o120000 => Err(unsupported(format!(
            "7z symbolic link {path} is unsupported"
        ))),
        0o010000 => Err(unsupported(format!("7z FIFO {path} is unsupported"))),
        0o020000 => Err(unsupported(format!(
            "7z character device {path} is unsupported"
        ))),
        0o060000 => Err(unsupported(format!(
            "7z block device {path} is unsupported"
        ))),
        _ => Err(unsupported(format!(
            "7z special entry {path} has unsupported mode kind {kind:#o}"
        ))),
    }
}

fn filetime_timestamp(value: u64) -> Result<Timestamp> {
    const WINDOWS_TO_UNIX_100NS: i128 = 116_444_736_000_000_000;
    let ticks = i128::from(value) - WINDOWS_TO_UNIX_100NS;
    let seconds = ticks.div_euclid(10_000_000);
    let nanos = ticks.rem_euclid(10_000_000) * 100;
    Timestamp::new(
        i64::try_from(seconds).map_err(|_| structure("7z FILETIME seconds overflow"))?,
        u32::try_from(nanos).map_err(|_| structure("7z FILETIME nanoseconds overflow"))?,
        TimestampPrecision::Hectonanosecond,
        true,
    )
}

fn logical_path(value: &str) -> Result<(LogicalPath, Vec<String>)> {
    if value.is_empty()
        || value.starts_with('/')
        || value.starts_with('\\')
        || value.as_bytes().get(1).is_some_and(|byte| *byte == b':')
        || value.contains('\0')
    {
        return Err(unsafe_path(format!("unsafe 7z path '{value}'")));
    }
    let normalized = value.replace('\\', "/");
    let trimmed = normalized.strip_suffix('/').unwrap_or(&normalized);
    let components = trimmed.split('/').map(str::to_owned).collect::<Vec<_>>();
    if components.iter().any(|component| component.contains(':')) {
        return Err(unsupported(format!(
            "7z alternate-stream/colon path '{value}' is unsupported"
        )));
    }
    if components.is_empty()
        || components
            .iter()
            .any(|component| component.is_empty() || component == "." || component == "..")
    {
        return Err(unsafe_path(format!("unsafe 7z path '{value}'")));
    }
    LogicalPath::from_utf8(&components)
        .map(|path| (path, components))
        .map_err(|error| unsafe_path(error.detail()))
}

fn folder_output_size(folder: &Folder) -> Result<u64> {
    let bound_outputs = folder
        .bind_pairs
        .iter()
        .map(|pair| pair.output)
        .collect::<BTreeSet<_>>();
    let unbound = (0..u64::try_from(folder.unpack_sizes.len()).unwrap_or(u64::MAX))
        .filter(|index| !bound_outputs.contains(index))
        .collect::<Vec<_>>();
    if unbound.len() != 1 {
        return Err(irreconcilable("7z folder does not have one final output"));
    }
    folder
        .unpack_sizes
        .get(usize_from_u64(unbound[0], "folder output index")?)
        .copied()
        .ok_or_else(|| irreconcilable("7z folder final output size is missing"))
}

fn is_core(method: &[u8]) -> bool {
    matches!(
        method,
        METHOD_COPY | METHOD_LZMA | METHOD_LZMA2 | METHOD_BZIP2 | METHOD_DEFLATE
    )
}

fn is_filter(method: &[u8]) -> bool {
    matches!(method, METHOD_DELTA | METHOD_BCJ_X86)
}

fn lzma2_dictionary(property: u8) -> Result<u32> {
    if property > 40 {
        return Err(structure("7z LZMA2 property exceeds 40"));
    }
    if property == 40 {
        Ok(u32::MAX)
    } else {
        Ok((2_u32 | u32::from(property & 1)) << (u32::from(property / 2) + 11))
    }
}

fn sevenz_fidelity(unsupported: &[String]) -> FidelityReport {
    FidelityReport {
        captured: Box::from([
            "core.executable".to_owned(),
            "core.mtime".to_owned(),
            "legacy.conversion-provenance".to_owned(),
        ]),
        unavailable: unsupported
            .iter()
            .map(|class| FidelityIssue {
                class: class.clone(),
                reason:
                    "observed as 7z evidence but unsupported by the current EAM metadata subset"
                        .to_owned(),
                entry_scope: None,
            })
            .collect::<Vec<_>>()
            .into_boxed_slice(),
        degraded: Box::default(),
        platform: "legacy:7z".to_owned(),
        filesystem: Box::default(),
    }
}

fn count_resolution(resolutions: &[ConversionResolution], class: ConflictClass) -> u64 {
    resolutions
        .iter()
        .filter(|resolution| resolution.conflict_class == class.as_str())
        .count()
        .try_into()
        .unwrap_or(u64::MAX)
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
                .map(|authority| {
                    format!(
                        "{}:{}:{}",
                        authority.format, authority.structure, authority.instance
                    )
                })
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

fn authority(structure: &str, instance: u64) -> LegacyAuthority {
    LegacyAuthority {
        format: "7z".to_owned(),
        structure: structure.to_owned(),
        instance,
    }
}

fn field_u64(
    semantic_field: &str,
    authority: &LegacyAuthority,
    value: u64,
    offset: usize,
    length: usize,
) -> LegacyFieldObservation<LegacyObservedValue> {
    LegacyFieldObservation {
        semantic_field: semantic_field.to_owned(),
        authority: authority.clone(),
        raw_value: value.to_le_bytes().to_vec().into_boxed_slice(),
        interpreted_value: Some(LegacyObservedValue::Unsigned(value)),
        evidence: location(offset, length),
        validity: ObservationValidity::Valid,
    }
}

fn field_bool(
    semantic_field: &str,
    authority: &LegacyAuthority,
    value: bool,
    offset: usize,
    length: usize,
) -> LegacyFieldObservation<LegacyObservedValue> {
    LegacyFieldObservation {
        semantic_field: semantic_field.to_owned(),
        authority: authority.clone(),
        raw_value: Box::from([u8::from(value)]),
        interpreted_value: Some(LegacyObservedValue::Boolean(value)),
        evidence: location(offset, length),
        validity: ObservationValidity::Valid,
    }
}

fn field_text(
    semantic_field: &str,
    authority: &LegacyAuthority,
    value: &str,
    offset: usize,
    length: usize,
) -> LegacyFieldObservation<LegacyObservedValue> {
    LegacyFieldObservation {
        semantic_field: semantic_field.to_owned(),
        authority: authority.clone(),
        raw_value: value.as_bytes().to_vec().into_boxed_slice(),
        interpreted_value: Some(LegacyObservedValue::Text(value.to_owned())),
        evidence: location(offset, length),
        validity: ObservationValidity::Valid,
    }
}

fn field_bytes(
    semantic_field: &str,
    authority: &LegacyAuthority,
    value: &[u8],
    offset: usize,
) -> LegacyFieldObservation<LegacyObservedValue> {
    LegacyFieldObservation {
        semantic_field: semantic_field.to_owned(),
        authority: authority.clone(),
        raw_value: value.to_vec().into_boxed_slice(),
        interpreted_value: Some(LegacyObservedValue::Bytes(
            value.to_vec().into_boxed_slice(),
        )),
        evidence: location(offset, value.len()),
        validity: ObservationValidity::Valid,
    }
}

fn location(offset: usize, length: usize) -> LegacyEvidenceLocation {
    LegacyEvidenceLocation {
        offset: u64::try_from(offset).unwrap_or(u64::MAX),
        length: u64::try_from(length).unwrap_or(u64::MAX),
    }
}

fn le_u16(bytes: &[u8]) -> u16 {
    u16::from_le_bytes(bytes.try_into().expect("exact u16 slice"))
}

fn le_u32(bytes: &[u8]) -> u32 {
    u32::from_le_bytes(bytes.try_into().expect("exact u32 slice"))
}

fn le_u64(bytes: &[u8]) -> u64 {
    u64::from_le_bytes(bytes.try_into().expect("exact u64 slice"))
}

fn usize_from_u64(value: u64, what: &str) -> Result<usize> {
    usize::try_from(value).map_err(|_| structure(format!("{what} is not addressable")))
}

fn method_hex(method: &[u8]) -> String {
    method
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
}

fn unsupported_coder(method: &[u8]) -> Diagnostic {
    let label = if method == METHOD_AES {
        "AES/encrypted content"
    } else if method == METHOD_PPMD {
        "PPMd"
    } else if method == METHOD_BCJ2 {
        "BCJ2"
    } else {
        "unknown"
    };
    Diagnostic::new(
        OutcomeClass::Unsupported,
        ReasonCode::SevenZUnsupportedCoder,
        format!(
            "unsupported 7z coder {label} (method 0x{})",
            method_hex(method)
        ),
    )
}

fn policy_check(condition: bool, detail: impl Into<String>) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(policy_error(detail))
    }
}

fn structure(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::Nonconforming,
        ReasonCode::SevenZStructureInvalid,
        detail,
    )
}

fn truncated(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(OutcomeClass::Truncated, ReasonCode::TruncatedStream, detail)
}

fn integrity(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::Corrupt,
        ReasonCode::SevenZIntegrityMismatch,
        detail,
    )
}

fn unsupported(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::Unsupported,
        ReasonCode::SevenZUnsupportedFeature,
        detail,
    )
}

fn divergence(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::Nonconforming,
        ReasonCode::SevenZConflictDivergence,
        detail,
    )
}

fn irreconcilable(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::Nonconforming,
        ReasonCode::SevenZConflictIrreconcilable,
        detail,
    )
}

fn unsafe_path(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::Nonconforming,
        ReasonCode::SevenZUnsafePath,
        detail,
    )
}

fn policy_error(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::PolicyRefused,
        ReasonCode::LegacyResourcePolicyRefused,
        detail,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn number(value: u64) -> Vec<u8> {
        for extra in 0..8_u32 {
            let high_bits = 7 - extra;
            let max = 1_u128 << (high_bits + extra * 8);
            if u128::from(value) < max {
                let prefix = if extra == 0 {
                    0
                } else {
                    0xff_u8 << (8 - extra)
                };
                let high = u8::try_from(value >> (extra * 8)).unwrap();
                let mut bytes = vec![prefix | high];
                for index in 0..extra {
                    bytes.push(u8::try_from((value >> (index * 8)) & 0xff).unwrap());
                }
                return bytes;
            }
        }
        let mut bytes = vec![0xff];
        bytes.extend_from_slice(&value.to_le_bytes());
        bytes
    }

    fn plain_copy_archive(entries: &[(&str, &[u8])], solid: bool) -> Vec<u8> {
        let joined = entries
            .iter()
            .flat_map(|(_, value)| *value)
            .copied()
            .collect::<Vec<_>>();
        let mut streams = Vec::new();
        streams.push(K_PACK_INFO);
        streams.extend(number(0));
        streams.extend(number(1));
        streams.push(K_SIZE);
        streams.extend(number(u64::try_from(joined.len()).unwrap()));
        streams.push(K_END);
        streams.push(K_UNPACK_INFO);
        streams.push(K_FOLDER);
        streams.extend(number(1));
        streams.push(0);
        streams.extend(number(1));
        streams.push(1);
        streams.push(0);
        streams.push(K_CODERS_UNPACK_SIZE);
        streams.extend(number(u64::try_from(joined.len()).unwrap()));
        streams.push(K_CRC);
        streams.push(1);
        streams.extend_from_slice(&crc32fast::hash(&joined).to_le_bytes());
        streams.push(K_END);
        streams.push(K_SUBSTREAMS_INFO);
        if solid && entries.len() > 1 {
            streams.push(K_NUM_UNPACK_STREAM);
            streams.extend(number(u64::try_from(entries.len()).unwrap()));
            streams.push(K_SIZE);
            for (_, value) in entries.iter().take(entries.len() - 1) {
                streams.extend(number(u64::try_from(value.len()).unwrap()));
            }
            streams.push(K_CRC);
            streams.push(1);
            for (_, value) in entries {
                streams.extend_from_slice(&crc32fast::hash(value).to_le_bytes());
            }
        }
        streams.push(K_END);
        streams.push(K_END);

        let mut header = vec![K_HEADER, K_MAIN_STREAMS_INFO];
        header.extend(streams);
        header.push(K_FILES_INFO);
        header.extend(number(u64::try_from(entries.len()).unwrap()));
        header.push(K_NAME);
        let mut names = vec![0];
        for (name, _) in entries {
            for unit in name.encode_utf16() {
                names.extend_from_slice(&unit.to_le_bytes());
            }
            names.extend_from_slice(&0_u16.to_le_bytes());
        }
        header.extend(number(u64::try_from(names.len()).unwrap()));
        header.extend(names);
        header.push(K_END);
        header.push(K_END);

        let next_offset = u64::try_from(joined.len()).unwrap();
        let mut archive = Vec::new();
        archive.extend_from_slice(SIGNATURE);
        archive.extend_from_slice(&[0, 4]);
        let mut start_fields = Vec::new();
        start_fields.extend_from_slice(&next_offset.to_le_bytes());
        start_fields.extend_from_slice(&u64::try_from(header.len()).unwrap().to_le_bytes());
        start_fields.extend_from_slice(&crc32fast::hash(&header).to_le_bytes());
        archive.extend_from_slice(&crc32fast::hash(&start_fields).to_le_bytes());
        archive.extend(start_fields);
        archive.extend(joined);
        archive.extend(header);
        archive
    }

    fn encoded_copy_archive(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let plain = plain_copy_archive(entries, entries.len() > 1);
        let packed_len = entries.iter().map(|(_, value)| value.len()).sum::<usize>();
        let plain_header = plain[32 + packed_len..].to_vec();
        let mut encoded = vec![K_ENCODED_HEADER, K_PACK_INFO];
        encoded.extend(number(u64::try_from(packed_len).unwrap()));
        encoded.extend(number(1));
        encoded.push(K_SIZE);
        encoded.extend(number(u64::try_from(plain_header.len()).unwrap()));
        encoded.push(K_END);
        encoded.push(K_UNPACK_INFO);
        encoded.push(K_FOLDER);
        encoded.extend(number(1));
        encoded.push(0);
        encoded.extend(number(1));
        encoded.push(1);
        encoded.push(0);
        encoded.push(K_CODERS_UNPACK_SIZE);
        encoded.extend(number(u64::try_from(plain_header.len()).unwrap()));
        encoded.push(K_CRC);
        encoded.push(1);
        encoded.extend_from_slice(&crc32fast::hash(&plain_header).to_le_bytes());
        encoded.push(K_END);
        encoded.push(K_END);

        let mut source = Vec::new();
        source.extend_from_slice(SIGNATURE);
        source.extend_from_slice(&[0, 4]);
        let next_offset = u64::try_from(packed_len + plain_header.len()).unwrap();
        let mut start_fields = Vec::new();
        start_fields.extend_from_slice(&next_offset.to_le_bytes());
        start_fields.extend_from_slice(&u64::try_from(encoded.len()).unwrap().to_le_bytes());
        start_fields.extend_from_slice(&crc32fast::hash(&encoded).to_le_bytes());
        source.extend_from_slice(&crc32fast::hash(&start_fields).to_le_bytes());
        source.extend(start_fields);
        source.extend_from_slice(&plain[32..32 + packed_len]);
        source.extend(plain_header);
        source.extend(encoded);
        source
    }

    #[test]
    fn variable_length_integer_boundaries_round_trip() {
        for value in [0, 0x7f, 0x80, 0x3fff, 0x4000, u32::MAX as u64, u64::MAX] {
            let bytes = number(value);
            let mut cursor = ByteCursor::new(&bytes, 0);
            assert_eq!(cursor.number("test").unwrap(), value);
            assert_eq!(cursor.remaining(), 0);
        }
        assert_eq!(
            ByteCursor::new(&[0xff], 0)
                .number("truncated")
                .unwrap_err()
                .code(),
            ReasonCode::TruncatedStream
        );
    }

    #[test]
    fn copy_and_solid_substreams_import() {
        let archive = plain_copy_archive(&[("a.txt", b"alpha"), ("d/b.txt", b"beta")], true);
        let imported = import_strict(
            &archive,
            SevenZImportPolicy::default(),
            CompressionProfile::Fast,
        )
        .unwrap();
        assert_eq!(imported.report.folder_count, 1);
        assert_eq!(imported.report.solid_folder_count, 1);
        assert_eq!(imported.archive.entry_set.len(), 3);
    }

    #[test]
    fn generated_encoded_header_uses_the_same_folder_decoder() {
        let source = encoded_copy_archive(&[("encoded/a", b"alpha"), ("encoded/b", b"beta")]);
        let imported = import_strict(
            &source,
            SevenZImportPolicy::default(),
            CompressionProfile::Fast,
        )
        .unwrap();
        assert!(imported.report.encoded_header);
        assert_eq!(imported.report.solid_folder_count, 1);
        assert_eq!(imported.archive.entry_set.len(), 3);
    }

    #[test]
    fn start_and_next_header_crc_are_integrity_authorities() {
        let archive = plain_copy_archive(&[("a", b"value")], false);
        let mut start_bad = archive.clone();
        start_bad[12] ^= 1;
        assert_eq!(
            observe(&start_bad, SevenZImportPolicy::default())
                .unwrap_err()
                .code(),
            ReasonCode::SevenZIntegrityMismatch
        );
        let mut next_bad = archive;
        *next_bad.last_mut().unwrap() ^= 1;
        assert_eq!(
            observe(&next_bad, SevenZImportPolicy::default())
                .unwrap_err()
                .code(),
            ReasonCode::SevenZIntegrityMismatch
        );
    }

    #[test]
    fn unsafe_and_duplicate_paths_fail_closed() {
        for archive in [
            plain_copy_archive(&[("../escape", b"x")], false),
            plain_copy_archive(&[("same", b"alpha"), ("same", b"beta")], true),
        ] {
            let error = import_strict(
                &archive,
                SevenZImportPolicy::default(),
                CompressionProfile::Fast,
            )
            .unwrap_err();
            assert!(
                matches!(
                    error.code(),
                    ReasonCode::SevenZUnsafePath | ReasonCode::DuplicateLogicalPath
                ),
                "{error}"
            );
        }
        assert_eq!(
            import_strict(
                &plain_copy_archive(&[("file:stream", b"x")], false),
                SevenZImportPolicy::default(),
                CompressionProfile::Fast,
            )
            .unwrap_err()
            .code(),
            ReasonCode::SevenZUnsupportedFeature
        );
    }

    #[test]
    fn graph_cycles_and_unsupported_coders_are_stable() {
        let folder = Folder {
            coders: vec![Coder {
                method: METHOD_PPMD.into(),
                properties: Box::default(),
                input_count: 1,
                output_count: 1,
                first_input: 0,
                first_output: 0,
            }],
            bind_pairs: Vec::new(),
            packed_indices: vec![0],
            unpack_sizes: vec![0],
            crc: None,
        };
        assert_eq!(
            validate_folder(&folder, SevenZImportPolicy::default())
                .unwrap_err()
                .code(),
            ReasonCode::SevenZUnsupportedCoder
        );
    }

    #[test]
    fn forged_extents_truncation_and_payload_crc_fail_stably() {
        let source = plain_copy_archive(&[("a", b"checked")], false);
        let mut forged = source.clone();
        forged[12..20].copy_from_slice(&u64::MAX.to_le_bytes());
        let start_crc = crc32fast::hash(&forged[12..32]);
        forged[8..12].copy_from_slice(&start_crc.to_le_bytes());
        assert_eq!(
            observe(&forged, SevenZImportPolicy::default())
                .unwrap_err()
                .code(),
            ReasonCode::SevenZStructureInvalid
        );

        let mut truncated = source.clone();
        truncated.pop();
        assert_eq!(
            observe(&truncated, SevenZImportPolicy::default())
                .unwrap_err()
                .class(),
            OutcomeClass::Truncated
        );

        let mut corrupt = source;
        corrupt[32] ^= 1;
        assert_eq!(
            import_strict(
                &corrupt,
                SevenZImportPolicy::default(),
                CompressionProfile::Fast,
            )
            .unwrap_err()
            .code(),
            ReasonCode::SevenZIntegrityMismatch
        );
    }

    #[test]
    fn dictionary_and_graph_bombs_are_refused_before_decode() {
        let mut folder = Folder {
            coders: vec![Coder {
                method: METHOD_LZMA2.into(),
                properties: Box::from([40]),
                input_count: 1,
                output_count: 1,
                first_input: 0,
                first_output: 0,
            }],
            bind_pairs: Vec::new(),
            packed_indices: vec![0],
            unpack_sizes: vec![1],
            crc: None,
        };
        let policy = SevenZImportPolicy {
            max_dictionary_bytes: 64 * 1024 * 1024,
            ..SevenZImportPolicy::default()
        };
        assert_eq!(
            validate_folder(&folder, policy).unwrap_err().class(),
            OutcomeClass::PolicyRefused
        );

        folder.coders = vec![
            Coder {
                method: METHOD_COPY.into(),
                properties: Box::default(),
                input_count: 1,
                output_count: 1,
                first_input: 0,
                first_output: 0,
            },
            Coder {
                method: METHOD_DELTA.into(),
                properties: Box::from([0]),
                input_count: 1,
                output_count: 1,
                first_input: 1,
                first_output: 1,
            },
        ];
        folder.bind_pairs = vec![
            BindPair {
                input: 1,
                output: 0,
            },
            BindPair {
                input: 0,
                output: 1,
            },
        ];
        assert_eq!(
            folder_order(&folder).unwrap_err().code(),
            ReasonCode::SevenZConflictIrreconcilable
        );
    }

    #[test]
    fn independent_sevenz_rust2_oracle_agrees_on_lzma2() {
        let expected = b"independent-oracle-".repeat(4096);
        let cursor = Cursor::new(Vec::new());
        let mut writer = sevenz_rust2::ArchiveWriter::new(cursor).unwrap();
        writer.set_encrypt_header(true);
        writer
            .push_archive_entry(
                sevenz_rust2::ArchiveEntry::new_file("oracle/data.bin"),
                Some(expected.as_slice()),
            )
            .unwrap();
        let source = writer.finish().unwrap().into_inner();

        let imported = import_strict(
            &source,
            SevenZImportPolicy::default(),
            CompressionProfile::Fast,
        )
        .unwrap();
        assert_eq!(imported.report.coder_count, 1);

        let mut oracle = Vec::new();
        let mut reader =
            sevenz_rust2::ArchiveReader::new(Cursor::new(source), sevenz_rust2::Password::empty())
                .unwrap();
        reader
            .for_each_entries(|entry, input| {
                if !entry.is_directory() {
                    input.read_to_end(&mut oracle)?;
                }
                Ok(true)
            })
            .unwrap();
        assert_eq!(oracle, expected);
    }

    #[test]
    fn generated_core_codec_payloads_round_trip() {
        let original = b"codec-frontier-".repeat(8192);
        let expected = u64::try_from(original.len()).unwrap();

        let lzma_options = lzma_rust2::LzmaOptions::default();
        let mut lzma_writer =
            lzma_rust2::LzmaWriter::new(Vec::new(), &lzma_options, false, false, Some(expected))
                .unwrap();
        lzma_writer.write_all(&original).unwrap();
        let lzma = lzma_writer.finish().unwrap();
        let mut lzma_properties = vec![lzma_options.get_props()];
        lzma_properties.extend_from_slice(&lzma_options.dict_size.to_le_bytes());

        let mut lzma2_writer =
            lzma_rust2::Lzma2Writer::new(Vec::new(), lzma_rust2::Lzma2Options::default());
        lzma2_writer.write_all(&original).unwrap();
        let lzma2 = lzma2_writer.finish().unwrap();

        let mut deflate =
            flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
        deflate.write_all(&original).unwrap();
        let deflate = deflate.finish().unwrap();

        let mut bzip2 = bzip2::write::BzEncoder::new(Vec::new(), bzip2::Compression::best());
        bzip2.write_all(&original).unwrap();
        let bzip2 = bzip2.finish().unwrap();

        for (method, properties, packed) in [
            (METHOD_LZMA, lzma_properties.as_slice(), lzma.as_slice()),
            (METHOD_LZMA2, &[22][..], lzma2.as_slice()),
            (METHOD_DEFLATE, &[][..], deflate.as_slice()),
            (METHOD_BZIP2, &[][..], bzip2.as_slice()),
        ] {
            let coder = Coder {
                method: method.into(),
                properties: properties.into(),
                input_count: 1,
                output_count: 1,
                first_input: 0,
                first_output: 0,
            };
            let decoded = decode_core(&coder, packed, expected).unwrap();
            assert_eq!(decoded, original);
        }
    }

    #[test]
    fn generated_delta_and_x86_filters_are_exactly_inverted() {
        let original = (0..16_384_u32)
            .flat_map(|value| {
                let mut bytes = vec![0xe8];
                bytes.extend_from_slice(&value.to_le_bytes());
                bytes
            })
            .collect::<Vec<_>>();

        let distance = 4_usize;
        let mut delta_encoded = original.clone();
        for index in (0..delta_encoded.len()).rev() {
            let prior = index
                .checked_sub(distance)
                .map_or(0, |prior| original[prior]);
            delta_encoded[index] = original[index].wrapping_sub(prior);
        }
        let delta = Coder {
            method: METHOD_DELTA.into(),
            properties: Box::from([u8::try_from(distance - 1).unwrap()]),
            input_count: 1,
            output_count: 1,
            first_input: 0,
            first_output: 0,
        };
        apply_filter(&delta, &mut delta_encoded).unwrap();
        assert_eq!(delta_encoded, original);

        let mut bcj_writer = lzma_rust2::filter::bcj::BcjWriter::new_x86(Vec::new(), 0);
        bcj_writer.write_all(&original).unwrap();
        let mut bcj_encoded = bcj_writer.finish().unwrap();
        let bcj = Coder {
            method: METHOD_BCJ_X86.into(),
            properties: Box::default(),
            input_count: 1,
            output_count: 1,
            first_input: 0,
            first_output: 0,
        };
        apply_filter(&bcj, &mut bcj_encoded).unwrap();
        assert_eq!(bcj_encoded, original);
    }
}
