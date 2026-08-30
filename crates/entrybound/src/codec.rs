//! Operational codecs selected by recorded TransformPlans.
//!
//! This module contains no profile or planner logic. Readers dispatch only
//! from canonical TransformPlan data carried by the archive.

use crate::diagnostics::{Diagnostic, OutcomeClass, ReasonCode, Result};
use std::io::{BufReader, Cursor, Read, Write};

use crate::eam::{ChunkGroup, DecodeRequirements, Dictionary, Digest, TransformPlan};
use crate::identity::{STORE_CODEC_IDENTIFIER, STORE_PLAN_ID, STORE_PLAN_IDENTIFIER, sha256_exact};

pub(crate) const ZSTD_CODEC_IDENTIFIER: &str = "zstandard/v1";
pub(crate) const ZSTD_WINDOW_LOG: u8 = 20;
pub(crate) const ZSTD_WINDOW_BYTES: u64 = 1 << ZSTD_WINDOW_LOG;
pub(crate) const ZSTD_WORKING_SET_BYTES: u64 = 4 * 1024 * 1024;
const ZSTD_PARAMETER_MAGIC: [u8; 4] = *b"ZP01";
const ZSTD_DICTIONARY_PARAMETER_MAGIC: [u8; 4] = *b"ZD01";
const ZSTD_PREFIX_PARAMETER_MAGIC: [u8; 4] = *b"ZX01";
const ZSTD_PLAN_BASE: u64 = 1_000;
const ZSTD_PREFIX_PLAN_BASE: u64 = 10_000;
const SUPPORTED_LEVELS: [i32; 7] = [1, 3, 5, 9, 15, 19, 22];
const SUPPORTED_LOOKBACKS: [u32; 4] = [1, 2, 4, 8];
pub(crate) const ZSTD_DICTIONARY_FORMAT: &str = "zstd-trained/v1";
pub(crate) const ZSTD_DICTIONARY_CONSTRUCTION_PREFIX: &str = "zstd-1.5.7-train-buffer-v1/";
const SUPPORTED_DICTIONARY_CONSTRUCTIONS: [&str; 3] = [
    "zstd-1.5.7-train-buffer-v1/balanced-v3-digest-order-samples16-sample-cap16384-dict-cap8192",
    "zstd-1.5.7-train-buffer-v1/dense-v3-digest-order-samples32-sample-cap32768-dict-cap16384",
    "zstd-1.5.7-train-buffer-v1/extreme-v3-digest-order-samples64-sample-cap65536-dict-cap32768",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PlanMode {
    Independent,
    Dictionary(Digest),
    Prefix { lookback: u32 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ZstdParameters {
    level: i32,
    mode: PlanMode,
}

pub(crate) fn store_plan() -> TransformPlan {
    TransformPlan {
        plan_id: STORE_PLAN_ID,
        identifier: STORE_PLAN_IDENTIFIER.to_owned(),
        transforms: Box::default(),
        codec: STORE_CODEC_IDENTIFIER.to_owned(),
        codec_params: Box::default(),
        dictionary: None,
        decode: DecodeRequirements::default(),
    }
}

pub(crate) fn zstd_plan(level: i32) -> Result<TransformPlan> {
    if !SUPPORTED_LEVELS.contains(&level) {
        return Err(invalid_parameters(format!(
            "Zstandard level {level} is not registered by planner v1"
        )));
    }
    let mut parameters = Vec::with_capacity(12);
    parameters.extend_from_slice(&ZSTD_PARAMETER_MAGIC);
    parameters.extend_from_slice(&level.to_be_bytes());
    parameters.extend_from_slice(&[ZSTD_WINDOW_LOG, 0, 1, 0]);
    Ok(TransformPlan {
        plan_id: zstd_plan_id(level)?,
        identifier: zstd_plan_identifier(level),
        transforms: Box::default(),
        codec: ZSTD_CODEC_IDENTIFIER.to_owned(),
        codec_params: parameters.into_boxed_slice(),
        dictionary: None,
        decode: zstd_decode_requirements(),
    })
}

pub(crate) fn zstd_dictionary_plan(level: i32, dictionary_id: Digest) -> Result<TransformPlan> {
    ensure_supported_level(level)?;
    let mut parameters = Vec::with_capacity(12);
    parameters.extend_from_slice(&ZSTD_DICTIONARY_PARAMETER_MAGIC);
    parameters.extend_from_slice(&level.to_be_bytes());
    parameters.extend_from_slice(&[ZSTD_WINDOW_LOG, 0, 1, 0]);
    Ok(TransformPlan {
        plan_id: zstd_dictionary_plan_id(level, dictionary_id),
        identifier: format!(
            "zstandard-dictionary-v1-level-{level}-window-{ZSTD_WINDOW_LOG}-{}",
            dictionary_id
        ),
        transforms: Box::default(),
        codec: ZSTD_CODEC_IDENTIFIER.to_owned(),
        codec_params: parameters.into_boxed_slice(),
        dictionary: Some(dictionary_id),
        decode: zstd_decode_requirements(),
    })
}

pub(crate) fn zstd_prefix_plan(level: i32, lookback: u32) -> Result<TransformPlan> {
    ensure_supported_level(level)?;
    if !SUPPORTED_LOOKBACKS.contains(&lookback) {
        return Err(invalid_parameters(format!(
            "Zstandard prefix lookback {lookback} is not registered"
        )));
    }
    let mut parameters = Vec::with_capacity(12);
    parameters.extend_from_slice(&ZSTD_PREFIX_PARAMETER_MAGIC);
    parameters.extend_from_slice(&level.to_be_bytes());
    parameters.extend_from_slice(&[
        ZSTD_WINDOW_LOG,
        u8::try_from(lookback).map_err(|_| invalid_parameters("lookback exceeds u8"))?,
        1,
        0,
    ]);
    Ok(TransformPlan {
        plan_id: ZSTD_PREFIX_PLAN_BASE
            + (u64::try_from(level).unwrap_or(0) << 8)
            + u64::from(lookback),
        identifier: format!(
            "zstandard-prefix-v1-level-{level}-window-{ZSTD_WINDOW_LOG}-lookback-{lookback}"
        ),
        transforms: Box::default(),
        codec: ZSTD_CODEC_IDENTIFIER.to_owned(),
        codec_params: parameters.into_boxed_slice(),
        dictionary: None,
        decode: zstd_decode_requirements(),
    })
}

pub(crate) fn validate_plans(plans: &[TransformPlan]) -> Result<()> {
    if plans.is_empty() {
        return Err(Diagnostic::new(
            OutcomeClass::Nonconforming,
            ReasonCode::UnknownTransformPlan,
            "at least one TransformPlan is required",
        ));
    }
    for plan in plans {
        validate_plan(plan)?;
    }
    Ok(())
}

pub(crate) fn validate_plan(plan: &TransformPlan) -> Result<()> {
    if plan.codec == STORE_CODEC_IDENTIFIER {
        if plan != &store_plan() {
            return Err(Diagnostic::new(
                OutcomeClass::Unsupported,
                ReasonCode::UnknownTransformPlan,
                "STORE TransformPlan does not match bootstrap-store-v1",
            ));
        }
        return Ok(());
    }
    if plan.codec != ZSTD_CODEC_IDENTIFIER {
        return Err(Diagnostic::new(
            OutcomeClass::Unsupported,
            ReasonCode::UnknownCodec,
            format!("required codec '{}' is not implemented", plan.codec),
        ));
    }
    let parameters = parse_zstd_parameters(plan)?;
    let canonical = match parameters.mode {
        PlanMode::Independent => zstd_plan(parameters.level)?,
        PlanMode::Dictionary(dictionary_id) => {
            zstd_dictionary_plan(parameters.level, dictionary_id)?
        }
        PlanMode::Prefix { lookback } => zstd_prefix_plan(parameters.level, lookback)?,
    };
    if plan != &canonical {
        return Err(invalid_parameters(format!(
            "Zstandard TransformPlan {} is not canonical",
            plan.plan_id
        )));
    }
    Ok(())
}

pub(crate) fn aggregate_decode_requirements(plans: &[TransformPlan]) -> DecodeRequirements {
    plans
        .iter()
        .fold(DecodeRequirements::default(), |aggregate, plan| {
            DecodeRequirements {
                window_bytes: aggregate.window_bytes.max(plan.decode.window_bytes),
                working_set_bytes: aggregate
                    .working_set_bytes
                    .max(plan.decode.working_set_bytes),
                flags: aggregate.flags | plan.decode.flags,
            }
        })
}

pub(crate) fn aggregate_archive_decode_requirements(
    plans: &[TransformPlan],
    dictionaries: &std::collections::BTreeMap<Digest, Dictionary>,
    groups: &std::collections::BTreeMap<Digest, ChunkGroup>,
) -> Result<DecodeRequirements> {
    let mut aggregate = aggregate_decode_requirements(plans);
    let dictionary_bytes = dictionaries.values().try_fold(0_u64, |total, dictionary| {
        total
            .checked_add(u64::try_from(dictionary.bytes.len()).map_err(|_| {
                Diagnostic::new(
                    OutcomeClass::PolicyRefused,
                    ReasonCode::ResourceLimit,
                    "Dictionary length exceeds u64",
                )
            })?)
            .ok_or_else(|| {
                Diagnostic::new(
                    OutcomeClass::PolicyRefused,
                    ReasonCode::ResourceLimit,
                    "aggregate Dictionary bytes exceed u64",
                )
            })
    })?;
    let maximum_access = groups
        .values()
        .map(|group| group.max_preceding_bytes)
        .max()
        .unwrap_or(0);
    aggregate.working_set_bytes = aggregate
        .working_set_bytes
        .checked_add(dictionary_bytes)
        .and_then(|value| value.checked_add(maximum_access))
        .ok_or_else(|| {
            Diagnostic::new(
                OutcomeClass::PolicyRefused,
                ReasonCode::ResourceLimit,
                "aggregate decoder working set exceeds u64",
            )
        })?;
    Ok(aggregate)
}

pub(crate) fn encode_payload(plan: &TransformPlan, plaintext: &[u8]) -> Result<Vec<u8>> {
    validate_plan(plan)?;
    if plan.codec == STORE_CODEC_IDENTIFIER {
        return Ok(plaintext.to_vec());
    }
    let parameters = parse_zstd_parameters(plan)?;
    if parameters.mode != PlanMode::Independent {
        return Err(invalid_parameters(
            "dependent Zstandard plan requires its recorded dictionary or prefix",
        ));
    }
    encode_zstd(plaintext, parameters.level, None)
}

pub(crate) fn encode_payload_with_dictionary(
    plan: &TransformPlan,
    plaintext: &[u8],
    dictionary: &Dictionary,
) -> Result<Vec<u8>> {
    validate_plan(plan)?;
    validate_dictionary(dictionary)?;
    let parameters = parse_zstd_parameters(plan)?;
    if parameters.mode != PlanMode::Dictionary(dictionary.dictionary_id) {
        return Err(invalid_parameters(
            "TransformPlan and Dictionary dependency do not agree",
        ));
    }
    encode_zstd(plaintext, parameters.level, Some(&dictionary.bytes))
}

pub(crate) fn encode_payload_with_prefix(
    plan: &TransformPlan,
    plaintext: &[u8],
    prefix: &[u8],
) -> Result<Vec<u8>> {
    validate_plan(plan)?;
    let parameters = parse_zstd_parameters(plan)?;
    if !matches!(parameters.mode, PlanMode::Prefix { .. }) {
        return Err(invalid_parameters(
            "non-prefix TransformPlan cannot consume lookback bytes",
        ));
    }
    let mut encoder =
        zstd::stream::write::Encoder::with_ref_prefix(Vec::new(), parameters.level, prefix)
            .map_err(|error| compression(format!("create Zstandard prefix context: {error}")))?;
    encoder
        .window_log(u32::from(ZSTD_WINDOW_LOG))
        .and_then(|()| encoder.long_distance_matching(false))
        .and_then(|()| encoder.include_checksum(false))
        .and_then(|()| encoder.include_contentsize(true))
        .and_then(|()| encoder.include_dictid(false))
        .and_then(|()| encoder.set_pledged_src_size(Some(plaintext.len() as u64)))
        .map_err(|error| compression(format!("set Zstandard prefix parameters: {error}")))?;
    encoder
        .write_all(plaintext)
        .map_err(|error| compression(format!("encode Zstandard prefix frame: {error}")))?;
    encoder
        .finish()
        .map_err(|error| compression(format!("finish Zstandard prefix frame: {error}")))
}

fn encode_zstd(plaintext: &[u8], level: i32, dictionary: Option<&[u8]>) -> Result<Vec<u8>> {
    let mut compressor = match dictionary {
        Some(dictionary) => zstd::bulk::Compressor::with_dictionary(level, dictionary),
        None => zstd::bulk::Compressor::new(level),
    }
    .map_err(|error| compression(format!("create Zstandard context: {error}")))?;
    compressor
        .window_log(u32::from(ZSTD_WINDOW_LOG))
        .and_then(|()| compressor.long_distance_matching(false))
        .and_then(|()| compressor.include_checksum(false))
        .and_then(|()| compressor.include_contentsize(true))
        .and_then(|()| compressor.include_dictid(false))
        .map_err(|error| compression(format!("set Zstandard parameters: {error}")))?;
    compressor
        .compress(plaintext)
        .map_err(|error| compression(format!("encode Zstandard frame: {error}")))
}

pub(crate) fn decode_payload(
    plan: &TransformPlan,
    stored: &[u8],
    logical_len: u64,
) -> Result<Vec<u8>> {
    validate_plan(plan)?;
    if plan.codec == STORE_CODEC_IDENTIFIER {
        if u64::try_from(stored.len()).unwrap_or(u64::MAX) != logical_len {
            return Err(Diagnostic::new(
                OutcomeClass::Corrupt,
                ReasonCode::DecompressedLengthMismatch,
                "STORE stored length differs from logical length",
            ));
        }
        return Ok(stored.to_vec());
    }
    let parameters = parse_zstd_parameters(plan)?;
    if parameters.mode != PlanMode::Independent {
        return Err(invalid_parameters(
            "dependent Zstandard plan requires its recorded dictionary or prefix",
        ));
    }
    decode_zstd(stored, logical_len, None)
}

pub(crate) fn decode_payload_with_dictionary(
    plan: &TransformPlan,
    stored: &[u8],
    logical_len: u64,
    dictionary: &Dictionary,
) -> Result<Vec<u8>> {
    validate_plan(plan)?;
    validate_dictionary(dictionary)?;
    let parameters = parse_zstd_parameters(plan)?;
    if parameters.mode != PlanMode::Dictionary(dictionary.dictionary_id) {
        return Err(invalid_parameters(
            "TransformPlan and Dictionary dependency do not agree",
        ));
    }
    decode_zstd(stored, logical_len, Some(&dictionary.bytes))
}

pub(crate) fn decode_payload_with_prefix(
    plan: &TransformPlan,
    stored: &[u8],
    logical_len: u64,
    prefix: &[u8],
) -> Result<Vec<u8>> {
    validate_plan(plan)?;
    if !matches!(parse_zstd_parameters(plan)?.mode, PlanMode::Prefix { .. }) {
        return Err(invalid_parameters(
            "non-prefix TransformPlan cannot consume lookback bytes",
        ));
    }
    let capacity = logical_capacity(logical_len)?;
    let reader = BufReader::new(Cursor::new(stored));
    let mut decoder = zstd::stream::read::Decoder::with_ref_prefix(reader, prefix)
        .map_err(|error| decompression(format!("create Zstandard prefix context: {error}")))?
        .single_frame();
    decoder
        .window_log_max(u32::from(ZSTD_WINDOW_LOG))
        .map_err(|error| decompression(format!("set Zstandard window limit: {error}")))?;
    let mut plaintext = Vec::with_capacity(capacity);
    decoder
        .take(
            u64::try_from(capacity)
                .unwrap_or(u64::MAX)
                .saturating_add(1),
        )
        .read_to_end(&mut plaintext)
        .map_err(|error| decompression(format!("decode Zstandard prefix frame: {error}")))?;
    validate_decoded_length(plaintext, logical_len)
}

fn decode_zstd(stored: &[u8], logical_len: u64, dictionary: Option<&[u8]>) -> Result<Vec<u8>> {
    let capacity = logical_capacity(logical_len)?;
    let mut decompressor = match dictionary {
        Some(dictionary) => zstd::bulk::Decompressor::with_dictionary(dictionary),
        None => zstd::bulk::Decompressor::new(),
    }
    .map_err(|error| decompression(format!("create Zstandard context: {error}")))?;
    decompressor
        .window_log_max(u32::from(ZSTD_WINDOW_LOG))
        .map_err(|error| decompression(format!("set Zstandard window limit: {error}")))?;
    let plaintext = decompressor
        .decompress(stored, capacity)
        .map_err(|error| decompression(format!("decode Zstandard frame: {error}")))?;
    validate_decoded_length(plaintext, logical_len)
}

fn logical_capacity(logical_len: u64) -> Result<usize> {
    usize::try_from(logical_len).map_err(|_| {
        Diagnostic::new(
            OutcomeClass::PolicyRefused,
            ReasonCode::ResourceLimit,
            "Chunk logical length exceeds the platform allocation range",
        )
    })
}

fn validate_decoded_length(plaintext: Vec<u8>, logical_len: u64) -> Result<Vec<u8>> {
    if u64::try_from(plaintext.len()).unwrap_or(u64::MAX) != logical_len {
        return Err(Diagnostic::new(
            OutcomeClass::Corrupt,
            ReasonCode::DecompressedLengthMismatch,
            format!(
                "decoded {} bytes but Chunk declares {logical_len}",
                plaintext.len()
            ),
        ));
    }
    Ok(plaintext)
}

pub(crate) fn validate_dictionary(dictionary: &Dictionary) -> Result<()> {
    if dictionary.codec != ZSTD_CODEC_IDENTIFIER
        || dictionary.format != ZSTD_DICTIONARY_FORMAT
        || !SUPPORTED_DICTIONARY_CONSTRUCTIONS.contains(&dictionary.construction.as_str())
    {
        return Err(Diagnostic::new(
            OutcomeClass::Unsupported,
            ReasonCode::UnsupportedDictionaryFormat,
            format!(
                "Dictionary {} has an unsupported format",
                dictionary.dictionary_id
            ),
        ));
    }
    if sha256_exact(&dictionary.bytes) != dictionary.dictionary_id {
        return Err(Diagnostic::new(
            OutcomeClass::Corrupt,
            ReasonCode::DictionaryDigestMismatch,
            dictionary.dictionary_id.to_string(),
        ));
    }
    Ok(())
}

pub(crate) fn train_dictionary(samples: &[&[u8]], maximum_size: usize) -> Result<Vec<u8>> {
    if samples.len() < 2 || maximum_size == 0 {
        return Err(compression(
            "dictionary training requires at least two samples and non-zero capacity",
        ));
    }
    zstd::dict::from_samples(samples, maximum_size)
        .map_err(|error| compression(format!("train deterministic Zstandard dictionary: {error}")))
}

pub(crate) fn plan_mode(plan: &TransformPlan) -> Result<PlanMode> {
    if plan.codec == STORE_CODEC_IDENTIFIER {
        validate_plan(plan)?;
        return Ok(PlanMode::Independent);
    }
    Ok(parse_zstd_parameters(plan)?.mode)
}

pub(crate) const fn zstd_decode_requirements() -> DecodeRequirements {
    DecodeRequirements {
        window_bytes: ZSTD_WINDOW_BYTES,
        working_set_bytes: ZSTD_WORKING_SET_BYTES,
        flags: 0,
    }
}

fn parse_zstd_parameters(plan: &TransformPlan) -> Result<ZstdParameters> {
    let parameters = &plan.codec_params;
    if parameters.len() != 12
        || parameters[8] != ZSTD_WINDOW_LOG
        || parameters[10] != 1
        || parameters[11] != 0
    {
        return Err(invalid_parameters(
            "Zstandard parameters must use a registered namespace, level, and fixed window/frame flags",
        ));
    }
    let level = i32::from_be_bytes(
        parameters[4..8]
            .try_into()
            .map_err(|_| invalid_parameters("Zstandard level must be an exact big-endian i32"))?,
    );
    ensure_supported_level(level)?;
    let mode = if parameters[..4] == ZSTD_PARAMETER_MAGIC {
        if plan.dictionary.is_some() || parameters[9] != 0 {
            return Err(invalid_parameters(
                "ordinary Zstandard plan cannot reference a Dictionary",
            ));
        }
        PlanMode::Independent
    } else if parameters[..4] == ZSTD_DICTIONARY_PARAMETER_MAGIC {
        if parameters[9] != 0 {
            return Err(invalid_parameters(
                "dictionary Zstandard plan has non-zero reserved mode byte",
            ));
        }
        PlanMode::Dictionary(plan.dictionary.ok_or_else(|| {
            invalid_parameters("dictionary Zstandard plan is missing dictionary reference")
        })?)
    } else if parameters[..4] == ZSTD_PREFIX_PARAMETER_MAGIC {
        let lookback = u32::from(parameters[9]);
        if plan.dictionary.is_some() || !SUPPORTED_LOOKBACKS.contains(&lookback) {
            return Err(invalid_parameters(
                "prefix Zstandard plan has invalid Dictionary/lookback fields",
            ));
        }
        return Ok(ZstdParameters {
            level,
            mode: PlanMode::Prefix { lookback },
        });
    } else {
        return Err(invalid_parameters("unknown Zstandard parameter namespace"));
    };
    Ok(ZstdParameters { level, mode })
}

fn ensure_supported_level(level: i32) -> Result<()> {
    if SUPPORTED_LEVELS.contains(&level) {
        Ok(())
    } else {
        Err(invalid_parameters(format!(
            "Zstandard level {level} is not supported by codec plan v1"
        )))
    }
}

fn zstd_plan_id(level: i32) -> Result<u64> {
    let level = u64::try_from(level)
        .map_err(|_| invalid_parameters("Zstandard plan levels must be non-negative"))?;
    ZSTD_PLAN_BASE
        .checked_add(level)
        .ok_or_else(|| invalid_parameters("Zstandard plan identifier overflow"))
}

fn zstd_plan_identifier(level: i32) -> String {
    format!("zstandard-v1-level-{level}-window-{ZSTD_WINDOW_LOG}")
}

fn zstd_dictionary_plan_id(level: i32, dictionary_id: Digest) -> u64 {
    let mut input = Vec::new();
    input.extend_from_slice(b"entrybound/zstd-dictionary-plan/v1\0");
    input.extend_from_slice(&level.to_be_bytes());
    input.extend_from_slice(dictionary_id.as_bytes());
    let digest = sha256_exact(&input);
    let low = u64::from_be_bytes(digest.as_bytes()[..8].try_into().unwrap_or([0; 8]));
    0x8000_0000_0000_0000 | (low & 0x7fff_ffff_ffff_ffff)
}

fn invalid_parameters(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::Nonconforming,
        ReasonCode::InvalidCodecParameters,
        detail,
    )
}

fn compression(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::PolicyRefused,
        ReasonCode::CompressionFailed,
        detail,
    )
}

fn decompression(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::Corrupt,
        ReasonCode::DecompressionFailed,
        detail,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zstandard_plan_round_trips_repetitive_bytes() {
        let plan = zstd_plan(3).unwrap();
        let plaintext = vec![b'a'; 32 * 1024];
        let stored = encode_payload(&plan, &plaintext).unwrap();
        assert!(stored.len() < plaintext.len());
        assert_eq!(
            decode_payload(&plan, &stored, plaintext.len() as u64).unwrap(),
            plaintext
        );
    }

    #[test]
    fn parameter_encoding_is_closed_and_versioned() {
        let mut plan = zstd_plan(3).unwrap();
        plan.codec_params[0] ^= 1;
        assert_eq!(
            validate_plan(&plan).unwrap_err().code(),
            ReasonCode::InvalidCodecParameters
        );
    }

    #[test]
    fn dictionary_and_prefix_modes_round_trip_without_hidden_state() {
        let base = (0..16 * 1024)
            .map(|index| (index % 251) as u8)
            .collect::<Vec<_>>();
        let samples = (0..16)
            .map(|index| {
                let mut sample = base.clone();
                sample[512 + index] ^= u8::try_from(index + 1).unwrap();
                sample
            })
            .collect::<Vec<_>>();
        let sample_refs = samples.iter().map(Vec::as_slice).collect::<Vec<_>>();
        let bytes = train_dictionary(&sample_refs, 8 * 1024).unwrap();
        let dictionary = Dictionary {
            dictionary_id: sha256_exact(&bytes),
            codec: ZSTD_CODEC_IDENTIFIER.to_owned(),
            format: ZSTD_DICTIONARY_FORMAT.to_owned(),
            construction: SUPPORTED_DICTIONARY_CONSTRUCTIONS[0].to_owned(),
            bytes: bytes.into_boxed_slice(),
        };
        let dictionary_plan = zstd_dictionary_plan(5, dictionary.dictionary_id).unwrap();
        let stored =
            encode_payload_with_dictionary(&dictionary_plan, &samples[0], &dictionary).unwrap();
        assert_eq!(
            decode_payload_with_dictionary(
                &dictionary_plan,
                &stored,
                samples[0].len() as u64,
                &dictionary,
            )
            .unwrap(),
            samples[0]
        );

        let prefix_plan = zstd_prefix_plan(5, 1).unwrap();
        let stored = encode_payload_with_prefix(&prefix_plan, &samples[1], &samples[0]).unwrap();
        assert_eq!(
            decode_payload_with_prefix(
                &prefix_plan,
                &stored,
                samples[1].len() as u64,
                &samples[0],
            )
            .unwrap(),
            samples[1]
        );
    }
}
