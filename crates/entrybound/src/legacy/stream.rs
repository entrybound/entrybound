//! Bounded transport-stream observation and decoding.
//!
//! Transport metadata is auxiliary foreign evidence. It is never promoted to
//! an Entry name or metadata field by this module.

use std::io::Read;

use bzip2::bufread::BzDecoder;
use crc32fast::Hasher as Crc32;
use flate2::read::DeflateDecoder;
use lzma_rust2::{Action, Status, XzStream};

use super::lom::{
    LegacyArchiveObservation, LegacyAuthority, LegacyEvidenceLocation, LegacyFieldObservation,
    LegacyObservedValue, ObservationValidity,
};
use crate::diagnostics::{Diagnostic, OutcomeClass, ReasonCode, Result};
use crate::eam::Digest;
use crate::identity::sha256_exact;

const GZIP_MAGIC: [u8; 2] = [0x1f, 0x8b];
const ZSTD_MAGIC: u32 = 0xfd2f_b528;
const ZSTD_SKIPPABLE_MIN: u32 = 0x184d_2a50;
const ZSTD_SKIPPABLE_MAX: u32 = 0x184d_2a5f;
const XZ_MAGIC: [u8; 6] = [0xfd, b'7', b'z', b'X', b'Z', 0x00];

/// Supported foreign compression transports.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum TransportFormat {
    Gzip,
    Zstandard,
    Xz,
    Bzip2,
}

impl TransportFormat {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Gzip => "gzip",
            Self::Zstandard => "zstd",
            Self::Xz => "xz",
            Self::Bzip2 => "bzip2",
        }
    }
}

/// Caller-owned limits applied before and during transport decoding.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WrapperImportPolicy {
    pub max_source_bytes: u64,
    pub max_members: u64,
    pub max_metadata_bytes: u64,
    pub max_decoded_child_bytes: u64,
    pub max_aggregate_decoded_bytes: u64,
    pub max_expansion_ratio_milli: u64,
    pub max_decoder_memory_bytes: u64,
}

impl Default for WrapperImportPolicy {
    fn default() -> Self {
        Self {
            max_source_bytes: 4 * 1024 * 1024 * 1024,
            max_members: 1_000_000,
            max_metadata_bytes: 64 * 1024 * 1024,
            max_decoded_child_bytes: 16 * 1024 * 1024 * 1024,
            max_aggregate_decoded_bytes: 16 * 1024 * 1024 * 1024,
            max_expansion_ratio_milli: 1_000_000,
            max_decoder_memory_bytes: 512 * 1024 * 1024,
        }
    }
}

/// Authenticated/verified decoded transport bytes and their independent LOM.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecodedTransport {
    pub format: TransportFormat,
    pub decoded: Box<[u8]>,
    pub decoded_digest: Digest,
    pub member_count: u64,
    pub observation: LegacyArchiveObservation,
}

#[must_use]
pub fn detect(source: &[u8]) -> Option<TransportFormat> {
    if source.starts_with(&GZIP_MAGIC) {
        Some(TransportFormat::Gzip)
    } else if looks_like_zstd(source) {
        Some(TransportFormat::Zstandard)
    } else if source.starts_with(&XZ_MAGIC) {
        Some(TransportFormat::Xz)
    } else if source.len() >= 4 && source[..3] == *b"BZh" && matches!(source[3], b'1'..=b'9') {
        Some(TransportFormat::Bzip2)
    } else {
        None
    }
}

pub fn decode(
    source: &[u8],
    format: TransportFormat,
    policy: WrapperImportPolicy,
) -> Result<DecodedTransport> {
    policy_check(
        u64::try_from(source.len()).unwrap_or(u64::MAX) <= policy.max_source_bytes,
        "compressed transport exceeds max_source_bytes",
    )?;
    match format {
        TransportFormat::Gzip => decode_gzip(source, policy),
        TransportFormat::Zstandard => decode_zstd(source, policy),
        TransportFormat::Xz => decode_xz(source, policy),
        TransportFormat::Bzip2 => decode_bzip2(source, policy),
    }
}

fn decode_gzip(source: &[u8], policy: WrapperImportPolicy) -> Result<DecodedTransport> {
    let mut cursor = 0_usize;
    let mut members = 0_u64;
    let mut metadata_bytes = 0_u64;
    let mut decoded = Vec::new();
    let mut fields = Vec::new();
    while cursor < source.len() {
        members = members
            .checked_add(1)
            .ok_or_else(|| structure("gzip member count overflow"))?;
        policy_check(
            members <= policy.max_members,
            "gzip member count exceeds policy",
        )?;
        let start = cursor;
        let fixed = slice(source, cursor, 10, "gzip header")?;
        if fixed[..2] != GZIP_MAGIC || fixed[2] != 8 {
            return Err(structure(
                "gzip member magic or compression method is invalid",
            ));
        }
        let flags = fixed[3];
        if flags & 0xe0 != 0 {
            return Err(structure("gzip reserved header flags are nonzero"));
        }
        cursor += 10;
        let authority = authority("gzip-member", members - 1);
        fields.push(field_u64(
            "wrapper.flags",
            &authority,
            flags.into(),
            start + 3,
            1,
        ));
        fields.push(field_u64(
            "wrapper.mtime",
            &authority,
            u32::from_le_bytes(fixed[4..8].try_into().unwrap()).into(),
            start + 4,
            4,
        ));
        fields.push(field_u64(
            "wrapper.xfl",
            &authority,
            fixed[8].into(),
            start + 8,
            1,
        ));
        fields.push(field_u64(
            "wrapper.os",
            &authority,
            fixed[9].into(),
            start + 9,
            1,
        ));

        if flags & 0x04 != 0 {
            let length_bytes = slice(source, cursor, 2, "gzip FEXTRA length")?;
            let length = usize::from(u16::from_le_bytes(length_bytes.try_into().unwrap()));
            cursor += 2;
            let value = slice(source, cursor, length, "gzip FEXTRA")?;
            fields.push(field_bytes("wrapper.extra", &authority, value, cursor));
            cursor += length;
            metadata_bytes =
                metadata_bytes.saturating_add(u64::try_from(length + 2).unwrap_or(u64::MAX));
        }
        if flags & 0x08 != 0 {
            let (value, next) = nul_terminated(source, cursor, "gzip FNAME")?;
            fields.push(field_bytes("wrapper.filename", &authority, value, cursor));
            metadata_bytes =
                metadata_bytes.saturating_add(u64::try_from(next - cursor).unwrap_or(u64::MAX));
            cursor = next;
        }
        if flags & 0x10 != 0 {
            let (value, next) = nul_terminated(source, cursor, "gzip FCOMMENT")?;
            fields.push(field_bytes("wrapper.comment", &authority, value, cursor));
            metadata_bytes =
                metadata_bytes.saturating_add(u64::try_from(next - cursor).unwrap_or(u64::MAX));
            cursor = next;
        }
        if flags & 0x02 != 0 {
            let expected =
                u16::from_le_bytes(slice(source, cursor, 2, "gzip FHCRC")?.try_into().unwrap());
            let mut crc = Crc32::new();
            crc.update(&source[start..cursor]);
            if crc.finalize() as u16 != expected {
                return Err(integrity("gzip FHCRC mismatch"));
            }
            fields.push(field_u64(
                "wrapper.header_crc16",
                &authority,
                expected.into(),
                cursor,
                2,
            ));
            cursor += 2;
            metadata_bytes = metadata_bytes.saturating_add(2);
        }
        policy_check(
            metadata_bytes <= policy.max_metadata_bytes,
            "gzip metadata exceeds policy",
        )?;

        let deflate_start = cursor;
        let decoder = DeflateDecoder::new(&source[deflate_start..]);
        let remaining_limit = remaining_decoded_limit(source.len(), decoded.len(), policy);
        let mut limited = decoder.take(remaining_limit.saturating_add(1));
        let member_output_start = decoded.len();
        limited
            .read_to_end(&mut decoded)
            .map_err(|error| structure(format!("gzip DEFLATE decode failed: {error}")))?;
        if u64::try_from(decoded.len() - member_output_start).unwrap_or(u64::MAX) > remaining_limit
        {
            return Err(policy_error("gzip decoded bytes exceed policy"));
        }
        let decoder = limited.into_inner();
        let consumed = usize::try_from(decoder.total_in())
            .map_err(|_| structure("gzip DEFLATE extent exceeds addressable memory"))?;
        cursor = deflate_start
            .checked_add(consumed)
            .ok_or_else(|| structure("gzip DEFLATE extent overflow"))?;
        let trailer = slice(source, cursor, 8, "gzip trailer")?;
        let expected_crc = u32::from_le_bytes(trailer[..4].try_into().unwrap());
        let expected_size = u32::from_le_bytes(trailer[4..8].try_into().unwrap());
        let member_output = &decoded[member_output_start..];
        let mut crc = Crc32::new();
        crc.update(member_output);
        if crc.finalize() != expected_crc {
            return Err(integrity("gzip member CRC-32 mismatch"));
        }
        if u32::try_from(member_output.len()).unwrap_or(u32::MAX) != expected_size {
            return Err(integrity("gzip member ISIZE mismatch"));
        }
        fields.push(field_u64(
            "wrapper.crc32",
            &authority,
            expected_crc.into(),
            cursor,
            4,
        ));
        fields.push(field_u64(
            "wrapper.isize",
            &authority,
            expected_size.into(),
            cursor + 4,
            4,
        ));
        cursor += 8;
        fields.push(field_u64(
            "wrapper.member_extent",
            &authority,
            u64::try_from(cursor - start).unwrap_or(u64::MAX),
            start,
            cursor - start,
        ));
        enforce_decoded_limits(source.len(), decoded.len(), policy)?;
    }
    finish(TransportFormat::Gzip, source, decoded, members, fields)
}

fn decode_zstd(source: &[u8], policy: WrapperImportPolicy) -> Result<DecodedTransport> {
    let mut cursor = 0_usize;
    let mut members = 0_u64;
    let mut metadata_bytes = 0_u64;
    let mut decoded = Vec::new();
    let mut fields = Vec::new();
    while cursor < source.len() {
        let prefix = slice(source, cursor, 4, "Zstandard frame magic")?;
        let magic = u32::from_le_bytes(prefix.try_into().unwrap());
        let authority = authority("zstd-frame", members);
        members = members
            .checked_add(1)
            .ok_or_else(|| structure("Zstandard frame count overflow"))?;
        policy_check(
            members <= policy.max_members,
            "Zstandard frame count exceeds policy",
        )?;
        if (ZSTD_SKIPPABLE_MIN..=ZSTD_SKIPPABLE_MAX).contains(&magic) {
            let length = usize::try_from(u32::from_le_bytes(
                slice(source, cursor + 4, 4, "Zstandard skippable length")?
                    .try_into()
                    .unwrap(),
            ))
            .map_err(|_| structure("Zstandard skippable size is not addressable"))?;
            let total = 8_usize
                .checked_add(length)
                .ok_or_else(|| structure("Zstandard skippable extent overflow"))?;
            slice(source, cursor, total, "Zstandard skippable frame")?;
            metadata_bytes =
                metadata_bytes.saturating_add(u64::try_from(length).unwrap_or(u64::MAX));
            policy_check(
                metadata_bytes <= policy.max_metadata_bytes,
                "Zstandard skippable metadata exceeds policy",
            )?;
            fields.push(field_u64(
                "wrapper.skippable_magic",
                &authority,
                magic.into(),
                cursor,
                4,
            ));
            fields.push(field_u64(
                "wrapper.skippable_length",
                &authority,
                u64::try_from(length).unwrap_or(u64::MAX),
                cursor + 4,
                4,
            ));
            cursor += total;
            continue;
        }
        if magic != ZSTD_MAGIC {
            return Err(structure(
                "Zstandard frame sequence contains an unknown magic",
            ));
        }
        let frame_size = zstd::zstd_safe::find_frame_compressed_size(&source[cursor..])
            .map_err(|error| structure(format!("invalid Zstandard frame: {error}")))?;
        let frame = slice(source, cursor, frame_size, "Zstandard frame")?;
        if let Some(dictionary_id) = zstd::zstd_safe::get_dict_id_from_frame(frame) {
            return Err(Diagnostic::new(
                OutcomeClass::Unsupported,
                ReasonCode::WrapperDictionaryUnsupported,
                format!("Zstandard transport requires external dictionary {dictionary_id}"),
            ));
        }
        if let Some(content_size) = zstd::zstd_safe::get_frame_content_size(frame)
            .map_err(|error| structure(format!("invalid Zstandard frame header: {error:?}")))?
        {
            policy_check(
                content_size <= remaining_decoded_limit(source.len(), decoded.len(), policy),
                "Zstandard frame content size exceeds policy",
            )?;
            fields.push(field_u64(
                "wrapper.frame_content_size",
                &authority,
                content_size,
                cursor,
                frame_size.min(18),
            ));
        }
        let mut decoder = zstd::stream::read::Decoder::new(frame)
            .map_err(|error| structure(format!("Zstandard decoder rejected frame: {error}")))?;
        let window_log = maximum_window_log(policy.max_decoder_memory_bytes)?;
        decoder
            .window_log_max(window_log)
            .map_err(|error| policy_error(format!("Zstandard window exceeds policy: {error}")))?;
        let output_start = decoded.len();
        let remaining_limit = remaining_decoded_limit(source.len(), output_start, policy);
        let mut limited = decoder
            .single_frame()
            .take(remaining_limit.saturating_add(1));
        limited.read_to_end(&mut decoded).map_err(|error| {
            integrity(format!("Zstandard frame decode/checksum failed: {error}"))
        })?;
        if u64::try_from(decoded.len() - output_start).unwrap_or(u64::MAX) > remaining_limit {
            return Err(policy_error("Zstandard decoded bytes exceed policy"));
        }
        fields.push(field_u64(
            "wrapper.frame_extent",
            &authority,
            u64::try_from(frame_size).unwrap_or(u64::MAX),
            cursor,
            frame_size,
        ));
        fields.push(field_u64(
            "wrapper.content_checksum",
            &authority,
            u64::from(frame.get(4).is_some_and(|value| value & 0x04 != 0)),
            cursor + 4,
            1,
        ));
        cursor += frame_size;
        enforce_decoded_limits(source.len(), decoded.len(), policy)?;
    }
    finish(TransportFormat::Zstandard, source, decoded, members, fields)
}

fn decode_xz(source: &[u8], policy: WrapperImportPolicy) -> Result<DecodedTransport> {
    if !source.starts_with(&XZ_MAGIC) {
        return Err(structure("XZ stream magic is invalid"));
    }
    let memory_kib = u32::try_from(policy.max_decoder_memory_bytes / 1024)
        .unwrap_or(u32::MAX)
        .max(1);
    let mut position = 0_usize;
    let mut members = 0_u64;
    let mut decoded = Vec::new();
    let mut fields = Vec::new();
    while position < source.len() {
        if members != 0 {
            let padding_start = position;
            while source.get(position) == Some(&0) {
                position += 1;
            }
            let padding = position - padding_start;
            if !padding.is_multiple_of(4) {
                return Err(structure(
                    "XZ stream padding is not a multiple of four bytes",
                ));
            }
            if padding != 0 {
                fields.push(field_u64(
                    "wrapper.stream_padding_bytes",
                    &authority("xz-stream-padding", members - 1),
                    u64::try_from(padding).unwrap_or(u64::MAX),
                    padding_start,
                    padding,
                ));
            }
            if position == source.len() {
                break;
            }
        }
        if !source[position..].starts_with(&XZ_MAGIC) {
            return Err(structure(
                "XZ concatenation contains unknown trailing bytes",
            ));
        }
        members = members.saturating_add(1);
        policy_check(
            members <= policy.max_members,
            "XZ stream count exceeds policy",
        )?;
        let stream_start = position;
        let mut decoder = XzStream::new_mem_limit(false, memory_kib);
        let mut buffer = [0_u8; 64 * 1024];
        loop {
            let action = if position == source.len() {
                Action::Finish
            } else {
                Action::Run
            };
            let result = decoder
                .process(&source[position..], &mut buffer, action)
                .map_err(|error| integrity(format!("XZ decode/check failure: {error}")))?;
            position = position
                .checked_add(result.bytes_consumed)
                .ok_or_else(|| structure("XZ input position overflow"))?;
            decoded.extend_from_slice(&buffer[..result.bytes_produced]);
            enforce_decoded_limits(source.len(), decoded.len(), policy)?;
            if result.status == Status::StreamEnd {
                break;
            }
            if result.bytes_consumed == 0 && result.bytes_produced == 0 {
                return Err(structure("XZ decoder made no progress before stream end"));
            }
        }
        fields.push(field_u64(
            "wrapper.stream_extent",
            &authority("xz-stream", members - 1),
            u64::try_from(position - stream_start).unwrap_or(u64::MAX),
            stream_start,
            position - stream_start,
        ));
    }
    fields.extend([
        field_u64(
            "wrapper.stream_count",
            &authority("xz-stream-sequence", 0),
            members,
            0,
            source.len(),
        ),
        field_u64(
            "wrapper.decoder_memory_limit",
            &authority("xz-stream-sequence", 0),
            policy.max_decoder_memory_bytes,
            0,
            XZ_MAGIC.len(),
        ),
    ]);
    finish(TransportFormat::Xz, source, decoded, members, fields)
}

fn decode_bzip2(source: &[u8], policy: WrapperImportPolicy) -> Result<DecodedTransport> {
    let mut position = 0_usize;
    let mut members = 0_u64;
    let mut decoded = Vec::new();
    let mut fields = Vec::new();
    while position < source.len() {
        let header = source
            .get(position..position.saturating_add(4))
            .ok_or_else(|| structure("bzip2 member header is truncated"))?;
        if header[..3] != *b"BZh" || !matches!(header[3], b'1'..=b'9') {
            return Err(structure(
                "bzip2 member magic or block-size digit is invalid",
            ));
        }
        members = members.saturating_add(1);
        policy_check(
            members <= policy.max_members,
            "bzip2 stream count exceeds policy",
        )?;
        let block_digit = u64::from(header[3] - b'0');
        let declared_working_set = block_digit.saturating_mul(100_000).saturating_mul(6);
        policy_check(
            declared_working_set <= policy.max_decoder_memory_bytes,
            "bzip2 block working set exceeds policy",
        )?;
        let mut decoder = BzDecoder::new(std::io::Cursor::new(&source[position..]));
        let mut buffer = [0_u8; 64 * 1024];
        loop {
            let count = decoder
                .read(&mut buffer)
                .map_err(|error| integrity(format!("bzip2 decode/CRC failure: {error}")))?;
            if count == 0 {
                break;
            }
            decoded.extend_from_slice(&buffer[..count]);
            enforce_decoded_limits(source.len(), decoded.len(), policy)?;
        }
        let consumed = usize::try_from(decoder.total_in())
            .map_err(|_| structure("bzip2 member extent is not addressable"))?;
        if consumed == 0 {
            return Err(structure("bzip2 decoder consumed no input"));
        }
        fields.push(field_u64(
            "wrapper.member_compressed_bytes",
            &authority("bzip2-member", members - 1),
            u64::try_from(consumed).unwrap_or(u64::MAX),
            position,
            consumed,
        ));
        fields.push(field_u64(
            "wrapper.block_size_100k",
            &authority("bzip2-member", members - 1),
            block_digit,
            position + 3,
            1,
        ));
        position = position
            .checked_add(consumed)
            .ok_or_else(|| structure("bzip2 member extent overflow"))?;
    }
    fields.extend([field_u64(
        "wrapper.stream_count",
        &authority("bzip2-stream-sequence", 0),
        members,
        0,
        source.len(),
    )]);
    finish(TransportFormat::Bzip2, source, decoded, members, fields)
}

fn finish(
    format: TransportFormat,
    source: &[u8],
    decoded: Vec<u8>,
    members: u64,
    mut fields: Vec<LegacyFieldObservation<LegacyObservedValue>>,
) -> Result<DecodedTransport> {
    let decoded_digest = sha256_exact(&decoded);
    let layer = authority("transport-layer", 0);
    fields.push(field_u64("layer.ordinal", &layer, 0, 0, 0));
    fields.push(field_text(
        "layer.format",
        &layer,
        format.as_str(),
        0,
        source.len().min(8),
    ));
    fields.push(field_bytes(
        "layer.decoded_child_digest",
        &layer,
        decoded_digest.as_bytes(),
        0,
    ));
    fields.push(field_u64(
        "layer.member_count",
        &layer,
        members,
        0,
        source.len(),
    ));
    fields.push(field_u64(
        "layer.decoded_bytes",
        &layer,
        u64::try_from(decoded.len()).unwrap_or(u64::MAX),
        0,
        source.len(),
    ));
    Ok(DecodedTransport {
        format,
        decoded: decoded.into_boxed_slice(),
        decoded_digest,
        member_count: members,
        observation: LegacyArchiveObservation {
            source_format: format.as_str().to_owned(),
            source_digest: sha256_exact(source),
            archive_fields: fields.into_boxed_slice(),
            entries: Box::default(),
            conflicts: Box::default(),
        },
    })
}

fn enforce_decoded_limits(
    source_len: usize,
    decoded_len: usize,
    policy: WrapperImportPolicy,
) -> Result<()> {
    let decoded = u64::try_from(decoded_len).unwrap_or(u64::MAX);
    policy_check(
        decoded <= policy.max_decoded_child_bytes && decoded <= policy.max_aggregate_decoded_bytes,
        "compressed transport decoded bytes exceed policy",
    )?;
    let source = u64::try_from(source_len).unwrap_or(u64::MAX).max(1);
    let ratio = decoded.saturating_mul(1000) / source;
    policy_check(
        ratio <= policy.max_expansion_ratio_milli,
        "compressed transport expansion ratio exceeds policy",
    )
}

fn remaining_decoded_limit(
    source_len: usize,
    decoded_len: usize,
    policy: WrapperImportPolicy,
) -> u64 {
    let source = u64::try_from(source_len).unwrap_or(u64::MAX).max(1);
    let ratio_limit = source.saturating_mul(policy.max_expansion_ratio_milli) / 1000;
    policy
        .max_decoded_child_bytes
        .min(policy.max_aggregate_decoded_bytes)
        .min(ratio_limit)
        .saturating_sub(u64::try_from(decoded_len).unwrap_or(u64::MAX))
}

fn maximum_window_log(bytes: u64) -> Result<u32> {
    if bytes < 1024 {
        return Err(policy_error(
            "Zstandard decoder memory limit is below 1 KiB",
        ));
    }
    Ok((63 - bytes.leading_zeros()).clamp(10, 31))
}

fn looks_like_zstd(source: &[u8]) -> bool {
    source.get(..4).is_some_and(|prefix| {
        let magic = u32::from_le_bytes(prefix.try_into().unwrap());
        magic == ZSTD_MAGIC || (ZSTD_SKIPPABLE_MIN..=ZSTD_SKIPPABLE_MAX).contains(&magic)
    })
}

fn nul_terminated<'a>(source: &'a [u8], offset: usize, what: &str) -> Result<(&'a [u8], usize)> {
    let relative = source
        .get(offset..)
        .ok_or_else(|| structure(format!("{what} starts beyond EOF")))?
        .iter()
        .position(|byte| *byte == 0)
        .ok_or_else(|| structure(format!("{what} is not NUL-terminated")))?;
    let end = offset
        .checked_add(relative)
        .ok_or_else(|| structure(format!("{what} offset overflow")))?;
    Ok((&source[offset..end], end + 1))
}

fn slice<'a>(source: &'a [u8], offset: usize, length: usize, what: &str) -> Result<&'a [u8]> {
    let end = offset
        .checked_add(length)
        .ok_or_else(|| structure(format!("{what} extent overflow")))?;
    source
        .get(offset..end)
        .ok_or_else(|| structure(format!("{what} is truncated")))
}

fn authority(structure_name: &str, instance: u64) -> LegacyAuthority {
    LegacyAuthority {
        format: "compressed-stream".to_owned(),
        structure: structure_name.to_owned(),
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
        raw_value: value.to_be_bytes().to_vec().into_boxed_slice(),
        interpreted_value: Some(LegacyObservedValue::Unsigned(value)),
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

fn location(offset: usize, length: usize) -> LegacyEvidenceLocation {
    LegacyEvidenceLocation {
        offset: u64::try_from(offset).unwrap_or(u64::MAX),
        length: u64::try_from(length).unwrap_or(u64::MAX),
    }
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
        ReasonCode::WrapperStructureInvalid,
        detail,
    )
}

fn integrity(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::Corrupt,
        ReasonCode::WrapperIntegrityMismatch,
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
    use std::io::Write;

    use flate2::Compression;
    use flate2::write::GzEncoder;
    use lzma_rust2::{XzOptions, XzWriter};

    use super::*;

    fn gzip(value: &[u8]) -> Vec<u8> {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::fast());
        encoder.write_all(value).unwrap();
        encoder.finish().unwrap()
    }

    fn xz(value: &[u8]) -> Vec<u8> {
        let mut encoder = XzWriter::new(Vec::new(), XzOptions::with_preset(1)).unwrap();
        encoder.write_all(value).unwrap();
        encoder.finish().unwrap()
    }

    fn bzip2(value: &[u8]) -> Vec<u8> {
        let mut encoder = bzip2::write::BzEncoder::new(Vec::new(), bzip2::Compression::fast());
        encoder.write_all(value).unwrap();
        encoder.finish().unwrap()
    }

    fn zstd_with_checksum(value: &[u8]) -> Vec<u8> {
        let mut encoder = zstd::stream::Encoder::new(Vec::new(), 1).unwrap();
        encoder.include_checksum(true).unwrap();
        encoder.write_all(value).unwrap();
        encoder.finish().unwrap()
    }

    #[test]
    fn gzip_members_are_independently_verified_and_concatenated() {
        let mut source = gzip(b"alpha");
        source.extend(gzip(b"beta"));
        let decoded = decode(
            &source,
            TransportFormat::Gzip,
            WrapperImportPolicy::default(),
        )
        .unwrap();
        assert_eq!(decoded.decoded.as_ref(), b"alphabeta");
        assert_eq!(decoded.member_count, 2);

        let last = source.len() - 8;
        source[last] ^= 1;
        assert_eq!(
            decode(
                &source,
                TransportFormat::Gzip,
                WrapperImportPolicy::default()
            )
            .unwrap_err()
            .code(),
            ReasonCode::WrapperIntegrityMismatch
        );
    }

    #[test]
    fn zstd_frames_and_skippable_frames_are_composed_and_checked() {
        let first = zstd::stream::encode_all(&b"alpha"[..], 1).unwrap();
        let second = zstd::stream::encode_all(&b"beta"[..], 1).unwrap();
        let mut source = first;
        source.extend_from_slice(&ZSTD_SKIPPABLE_MIN.to_le_bytes());
        source.extend_from_slice(&3_u32.to_le_bytes());
        source.extend_from_slice(b"aux");
        source.extend(second);
        let decoded = decode(
            &source,
            TransportFormat::Zstandard,
            WrapperImportPolicy::default(),
        )
        .unwrap();
        assert_eq!(decoded.decoded.as_ref(), b"alphabeta");
        assert_eq!(decoded.member_count, 3);
        assert!(
            decoded
                .observation
                .archive_fields
                .iter()
                .any(|field| field.semantic_field == "wrapper.skippable_length")
        );
    }

    #[test]
    fn xz_and_bzip2_concatenated_streams_are_integrity_checked() {
        let mut xz_source = xz(b"alpha");
        xz_source.extend(xz(b"beta"));
        assert_eq!(
            decode(
                &xz_source,
                TransportFormat::Xz,
                WrapperImportPolicy::default()
            )
            .unwrap()
            .decoded
            .as_ref(),
            b"alphabeta"
        );

        let mut bz_source = bzip2(b"alpha");
        bz_source.extend(bzip2(b"beta"));
        assert_eq!(
            decode(
                &bz_source,
                TransportFormat::Bzip2,
                WrapperImportPolicy::default(),
            )
            .unwrap()
            .decoded
            .as_ref(),
            b"alphabeta"
        );
    }

    #[test]
    fn wrapper_limits_apply_before_unbounded_output() {
        let source = gzip(&vec![0_u8; 4096]);
        let policy = WrapperImportPolicy {
            max_decoded_child_bytes: 64,
            ..WrapperImportPolicy::default()
        };
        assert_eq!(
            decode(&source, TransportFormat::Gzip, policy)
                .unwrap_err()
                .code(),
            ReasonCode::LegacyResourcePolicyRefused
        );
    }

    #[test]
    fn zstd_xz_and_bzip2_corruption_never_yields_accepted_plaintext() {
        let mut zstd = zstd_with_checksum(b"checked zstandard payload");
        *zstd.last_mut().unwrap() ^= 1;
        assert_eq!(
            decode(
                &zstd,
                TransportFormat::Zstandard,
                WrapperImportPolicy::default()
            )
            .unwrap_err()
            .code(),
            ReasonCode::WrapperIntegrityMismatch
        );

        let mut xz = xz(b"checked xz payload");
        *xz.last_mut().unwrap() ^= 1;
        assert_eq!(
            decode(&xz, TransportFormat::Xz, WrapperImportPolicy::default())
                .unwrap_err()
                .code(),
            ReasonCode::WrapperIntegrityMismatch
        );

        let mut bzip2 = bzip2(b"checked bzip2 payload");
        bzip2.truncate(bzip2.len() - 3);
        assert_eq!(
            decode(
                &bzip2,
                TransportFormat::Bzip2,
                WrapperImportPolicy::default()
            )
            .unwrap_err()
            .code(),
            ReasonCode::WrapperIntegrityMismatch
        );
    }
}
