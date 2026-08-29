//! Operational codecs selected by recorded TransformPlans.
//!
//! This module contains no profile or planner logic. Readers dispatch only
//! from canonical TransformPlan data carried by the archive.

use crate::diagnostics::{Diagnostic, OutcomeClass, ReasonCode, Result};
use crate::eam::{DecodeRequirements, TransformPlan};
use crate::identity::{STORE_CODEC_IDENTIFIER, STORE_PLAN_ID, STORE_PLAN_IDENTIFIER};

pub(crate) const ZSTD_CODEC_IDENTIFIER: &str = "zstandard/v1";
pub(crate) const ZSTD_WINDOW_LOG: u8 = 20;
pub(crate) const ZSTD_WINDOW_BYTES: u64 = 1 << ZSTD_WINDOW_LOG;
pub(crate) const ZSTD_WORKING_SET_BYTES: u64 = 4 * 1024 * 1024;
const ZSTD_PARAMETER_MAGIC: [u8; 4] = *b"ZP01";
const ZSTD_PLAN_BASE: u64 = 1_000;
const SUPPORTED_LEVELS: [i32; 7] = [1, 3, 5, 9, 15, 19, 22];

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
    let level = parse_zstd_parameters(&plan.codec_params)?;
    if plan.plan_id != zstd_plan_id(level)?
        || plan.identifier != zstd_plan_identifier(level)
        || !plan.transforms.is_empty()
        || plan.dictionary.is_some()
        || plan.decode != zstd_decode_requirements()
    {
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

pub(crate) fn encode_payload(plan: &TransformPlan, plaintext: &[u8]) -> Result<Vec<u8>> {
    validate_plan(plan)?;
    if plan.codec == STORE_CODEC_IDENTIFIER {
        return Ok(plaintext.to_vec());
    }
    let level = parse_zstd_parameters(&plan.codec_params)?;
    let mut compressor = zstd::bulk::Compressor::new(level)
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
    let capacity = usize::try_from(logical_len).map_err(|_| {
        Diagnostic::new(
            OutcomeClass::PolicyRefused,
            ReasonCode::ResourceLimit,
            "Chunk logical length exceeds the platform allocation range",
        )
    })?;
    let mut decompressor = zstd::bulk::Decompressor::new().map_err(|error| {
        Diagnostic::new(
            OutcomeClass::Corrupt,
            ReasonCode::DecompressionFailed,
            format!("create Zstandard context: {error}"),
        )
    })?;
    decompressor
        .window_log_max(u32::from(ZSTD_WINDOW_LOG))
        .map_err(|error| {
            Diagnostic::new(
                OutcomeClass::Corrupt,
                ReasonCode::DecompressionFailed,
                format!("set Zstandard window limit: {error}"),
            )
        })?;
    let plaintext = decompressor.decompress(stored, capacity).map_err(|error| {
        Diagnostic::new(
            OutcomeClass::Corrupt,
            ReasonCode::DecompressionFailed,
            format!("decode Zstandard frame: {error}"),
        )
    })?;
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

pub(crate) const fn zstd_decode_requirements() -> DecodeRequirements {
    DecodeRequirements {
        window_bytes: ZSTD_WINDOW_BYTES,
        working_set_bytes: ZSTD_WORKING_SET_BYTES,
        flags: 0,
    }
}

fn parse_zstd_parameters(parameters: &[u8]) -> Result<i32> {
    if parameters.len() != 12
        || parameters[..4] != ZSTD_PARAMETER_MAGIC
        || parameters[8] != ZSTD_WINDOW_LOG
        || parameters[9] != 0
        || parameters[10] != 1
        || parameters[11] != 0
    {
        return Err(invalid_parameters(
            "Zstandard parameters must be ZP01, level, window=20, checksum=0, content-size=1, dictionary-id=0",
        ));
    }
    let level = i32::from_be_bytes(
        parameters[4..8]
            .try_into()
            .map_err(|_| invalid_parameters("Zstandard level must be an exact big-endian i32"))?,
    );
    if !SUPPORTED_LEVELS.contains(&level) {
        return Err(invalid_parameters(format!(
            "Zstandard level {level} is not supported by codec plan v1"
        )));
    }
    Ok(level)
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
}
