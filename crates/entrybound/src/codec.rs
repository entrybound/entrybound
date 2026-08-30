//! Operational codecs selected by recorded TransformPlans.
//!
//! This module contains no profile or planner logic. Readers dispatch only
//! from canonical TransformPlan data carried by the archive.

use std::io::{BufReader, Cursor, Read, Write};

use crate::diagnostics::{Diagnostic, OutcomeClass, ReasonCode, Result};
use crate::eam::{
    ChunkGroup, DecodeRequirements, Dictionary, Digest, TransformPlan, TransformStep,
};
use crate::ecf::FEATURE_CODEC_TRANSFORM_V1;
use crate::identity::{STORE_CODEC_IDENTIFIER, STORE_PLAN_ID, STORE_PLAN_IDENTIFIER, sha256_exact};
use crate::transform::{
    display_step, forward_pipeline, inverse_pipeline, required_features as transform_features,
    validate_pipeline,
};

pub(crate) const ZSTD_CODEC_IDENTIFIER: &str = "zstandard/v1";
pub(crate) const LZ4_CODEC_IDENTIFIER: &str = "lz4/v1";
pub(crate) const LZMA2_CODEC_IDENTIFIER: &str = "lzma2/v1";
pub(crate) const ZSTD_WINDOW_LOG: u8 = 20;
pub(crate) const ZSTD_WINDOW_BYTES: u64 = 1 << ZSTD_WINDOW_LOG;
pub(crate) const ZSTD_WORKING_SET_BYTES: u64 = 4 * 1024 * 1024;
const ZSTD_PARAMETER_MAGIC: [u8; 4] = *b"ZP01";
const ZSTD_DICTIONARY_PARAMETER_MAGIC: [u8; 4] = *b"ZD01";
const ZSTD_PREFIX_PARAMETER_MAGIC: [u8; 4] = *b"ZX01";
const ZSTD_PLAN_BASE: u64 = 1_000;
const ZSTD_PREFIX_PLAN_BASE: u64 = 10_000;
const LZ4_PLAN_ID: u64 = 20_001;
const LZMA2_PLAN_BASE: u64 = 30_000;
const TRANSFORMED_PLAN_NAMESPACE: u64 = 0x6000_0000_0000_0000;
const LZ4_PARAMETER_MAGIC: [u8; 4] = *b"L401";
const LZMA2_PARAMETER_MAGIC: [u8; 4] = *b"LM21";
const SUPPORTED_LEVELS: [i32; 7] = [1, 3, 5, 9, 15, 19, 22];
const SUPPORTED_LOOKBACKS: [u32; 4] = [1, 2, 4, 8];
const SUPPORTED_LZMA2_CONFIGURATIONS: [(u8, u32); 3] =
    [(4, 1024 * 1024), (6, 4 * 1024 * 1024), (9, 8 * 1024 * 1024)];
pub(crate) const ZSTD_DICTIONARY_FORMAT: &str = "zstd-trained/v1";
pub(crate) const ZSTD_DICTIONARY_CONSTRUCTION_PREFIX: &str = "zstd-1.5.7-train-buffer-v1/";
const SUPPORTED_DICTIONARY_CONSTRUCTIONS: [&str; 6] = [
    "zstd-1.5.7-train-buffer-v1/balanced-v3-digest-order-samples16-sample-cap16384-dict-cap8192",
    "zstd-1.5.7-train-buffer-v1/dense-v3-digest-order-samples32-sample-cap32768-dict-cap16384",
    "zstd-1.5.7-train-buffer-v1/extreme-v3-digest-order-samples64-sample-cap65536-dict-cap32768",
    "zstd-1.5.7-train-buffer-v1/balanced-v4-digest-order-samples16-sample-cap16384-dict-cap8192",
    "zstd-1.5.7-train-buffer-v1/dense-v4-digest-order-samples32-sample-cap32768-dict-cap16384",
    "zstd-1.5.7-train-buffer-v1/extreme-v4-digest-order-samples64-sample-cap65536-dict-cap32768",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PlanMode {
    Independent,
    Dictionary(Digest),
    Prefix { lookback: u32 },
}

type ValidateCodec = fn(&TransformPlan) -> Result<TransformPlan>;
type EncodeCodec = fn(&TransformPlan, &[u8], CodecContext<'_>) -> Result<Vec<u8>>;
type DecodeCodec = fn(&TransformPlan, &[u8], u64, CodecContext<'_>) -> Result<Vec<u8>>;

pub(crate) struct CodecRegistration {
    pub identifier: &'static str,
    pub format_version: u16,
    pub required_feature: u64,
    validate: ValidateCodec,
    encode: EncodeCodec,
    decode: DecodeCodec,
}

#[derive(Clone, Copy, Default)]
struct CodecContext<'a> {
    dictionary: Option<&'a Dictionary>,
    prefix: Option<&'a [u8]>,
}

static CODECS: [CodecRegistration; 4] = [
    CodecRegistration {
        identifier: STORE_CODEC_IDENTIFIER,
        format_version: 1,
        required_feature: 0,
        validate: validate_store_registration,
        encode: encode_store_registration,
        decode: decode_store_registration,
    },
    CodecRegistration {
        identifier: ZSTD_CODEC_IDENTIFIER,
        format_version: 1,
        required_feature: 0,
        validate: validate_zstd_registration,
        encode: encode_zstd_registration,
        decode: decode_zstd_registration,
    },
    CodecRegistration {
        identifier: LZ4_CODEC_IDENTIFIER,
        format_version: 1,
        required_feature: FEATURE_CODEC_TRANSFORM_V1,
        validate: validate_lz4_registration,
        encode: encode_lz4_registration,
        decode: decode_lz4_registration,
    },
    CodecRegistration {
        identifier: LZMA2_CODEC_IDENTIFIER,
        format_version: 1,
        required_feature: FEATURE_CODEC_TRANSFORM_V1,
        validate: validate_lzma2_registration,
        encode: encode_lzma2_registration,
        decode: decode_lzma2_registration,
    },
];

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

pub(crate) fn zstd_transformed_plan(
    level: i32,
    transforms: Box<[TransformStep]>,
) -> Result<TransformPlan> {
    with_pipeline(zstd_plan(level)?, transforms)
}

pub(crate) fn lz4_plan(transforms: Box<[TransformStep]>) -> Result<TransformPlan> {
    let mut parameters = Vec::with_capacity(8);
    parameters.extend_from_slice(&LZ4_PARAMETER_MAGIC);
    parameters.extend_from_slice(&[1, 0, 0, 0]);
    with_pipeline(
        TransformPlan {
            plan_id: LZ4_PLAN_ID,
            identifier: "lz4-block-v1-safe-default".to_owned(),
            transforms: Box::default(),
            codec: LZ4_CODEC_IDENTIFIER.to_owned(),
            codec_params: parameters.into_boxed_slice(),
            dictionary: None,
            decode: DecodeRequirements {
                window_bytes: 64 * 1024,
                working_set_bytes: 128 * 1024,
                flags: 0,
            },
        },
        transforms,
    )
}

pub(crate) fn lzma2_plan(
    preset: u8,
    dictionary_bytes: u32,
    transforms: Box<[TransformStep]>,
) -> Result<TransformPlan> {
    if !SUPPORTED_LZMA2_CONFIGURATIONS.contains(&(preset, dictionary_bytes)) {
        return Err(invalid_parameters(format!(
            "LZMA2 preset {preset} with dictionary {dictionary_bytes} is not registered"
        )));
    }
    let mut parameters = Vec::with_capacity(12);
    parameters.extend_from_slice(&LZMA2_PARAMETER_MAGIC);
    parameters.push(preset);
    parameters.push(0);
    parameters.extend_from_slice(&0_u16.to_be_bytes());
    parameters.extend_from_slice(&dictionary_bytes.to_be_bytes());
    let working_set_bytes = u64::from(lzma_rust2::lzma2_get_memory_usage(dictionary_bytes))
        .checked_mul(1024)
        .ok_or_else(|| invalid_parameters("LZMA2 memory declaration overflow"))?;
    with_pipeline(
        TransformPlan {
            plan_id: LZMA2_PLAN_BASE
                + u64::from(preset) * 100
                + u64::from(dictionary_bytes.trailing_zeros()),
            identifier: format!(
                "lzma2-raw-v1-preset-{preset}-dictionary-{dictionary_bytes}-single-thread"
            ),
            transforms: Box::default(),
            codec: LZMA2_CODEC_IDENTIFIER.to_owned(),
            codec_params: parameters.into_boxed_slice(),
            dictionary: None,
            decode: DecodeRequirements {
                window_bytes: u64::from(dictionary_bytes),
                working_set_bytes,
                flags: 0,
            },
        },
        transforms,
    )
}

fn with_pipeline(
    mut base: TransformPlan,
    transforms: Box<[TransformStep]>,
) -> Result<TransformPlan> {
    validate_pipeline(&transforms)?;
    if transforms.is_empty() {
        return Ok(base);
    }
    if plan_mode(&base)? != PlanMode::Independent {
        return Err(invalid_parameters(
            "bootstrap structural transforms require an independent codec plan",
        ));
    }
    let mut identity = Vec::new();
    identity.extend_from_slice(b"entrybound/transform-pipeline-plan/v1\0");
    append_identity_field(&mut identity, base.codec.as_bytes())?;
    append_identity_field(&mut identity, &base.codec_params)?;
    for step in &transforms {
        append_identity_field(&mut identity, step.transform_id.as_bytes())?;
        append_identity_field(&mut identity, &step.parameters)?;
    }
    let digest = sha256_exact(&identity);
    let low = u64::from_be_bytes(digest.as_bytes()[..8].try_into().unwrap_or([0; 8]));
    base.plan_id = TRANSFORMED_PLAN_NAMESPACE | (low & 0x1fff_ffff_ffff_ffff);
    base.identifier = format!(
        "{} -> {}",
        transforms
            .iter()
            .map(display_step)
            .collect::<Vec<_>>()
            .join(" -> "),
        base.identifier
    );
    base.transforms = transforms;
    Ok(base)
}

fn append_identity_field(output: &mut Vec<u8>, value: &[u8]) -> Result<()> {
    output.extend_from_slice(
        &u64::try_from(value.len())
            .map_err(|_| invalid_parameters("pipeline identity field exceeds u64"))?
            .to_be_bytes(),
    );
    output.extend_from_slice(value);
    Ok(())
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
    validate_pipeline(&plan.transforms)?;
    let registration = codec_registration(&plan.codec)?;
    let canonical = (registration.validate)(plan)?;
    if plan != &canonical {
        return Err(invalid_parameters(format!(
            "TransformPlan {} is not canonical for {}",
            plan.plan_id, registration.identifier
        )));
    }
    Ok(())
}

pub(crate) fn required_features(plan: &TransformPlan) -> Result<u64> {
    let codec = codec_registration(&plan.codec)?;
    Ok(codec.required_feature | transform_features(&plan.transforms)?)
}

pub(crate) fn without_transforms(plan: &TransformPlan) -> Result<TransformPlan> {
    if plan.transforms.is_empty() {
        return Ok(plan.clone());
    }
    match plan.codec.as_str() {
        ZSTD_CODEC_IDENTIFIER => {
            let parameters = parse_zstd_parameters(plan)?;
            if parameters.mode != PlanMode::Independent {
                return Err(invalid_parameters(
                    "dependent Zstandard plans cannot use structural transforms",
                ));
            }
            zstd_plan(parameters.level)
        }
        LZ4_CODEC_IDENTIFIER => lz4_plan(Box::default()),
        LZMA2_CODEC_IDENTIFIER => {
            let (preset, dictionary_bytes) = parse_lzma2_parameters(plan)?;
            lzma2_plan(preset, dictionary_bytes, Box::default())
        }
        STORE_CODEC_IDENTIFIER => Ok(store_plan()),
        identifier => {
            codec_registration(identifier)?;
            Err(invalid_parameters(
                "registered codec cannot remove transforms",
            ))
        }
    }
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
    if plan_mode(plan)? != PlanMode::Independent {
        return Err(invalid_parameters(
            "dependent codec plan requires its recorded dictionary or prefix",
        ));
    }
    encode_with_context(plan, plaintext, CodecContext::default())
}

pub(crate) fn encode_payload_with_dictionary(
    plan: &TransformPlan,
    plaintext: &[u8],
    dictionary: &Dictionary,
) -> Result<Vec<u8>> {
    validate_dictionary(dictionary)?;
    if plan_mode(plan)? != PlanMode::Dictionary(dictionary.dictionary_id) {
        return Err(invalid_parameters(
            "TransformPlan and Dictionary dependency do not agree",
        ));
    }
    encode_with_context(
        plan,
        plaintext,
        CodecContext {
            dictionary: Some(dictionary),
            prefix: None,
        },
    )
}

pub(crate) fn encode_payload_with_prefix(
    plan: &TransformPlan,
    plaintext: &[u8],
    prefix: &[u8],
) -> Result<Vec<u8>> {
    if !matches!(plan_mode(plan)?, PlanMode::Prefix { .. }) {
        return Err(invalid_parameters(
            "non-prefix TransformPlan cannot consume lookback bytes",
        ));
    }
    encode_with_context(
        plan,
        plaintext,
        CodecContext {
            dictionary: None,
            prefix: Some(prefix),
        },
    )
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
    if plan_mode(plan)? != PlanMode::Independent {
        return Err(invalid_parameters(
            "dependent codec plan requires its recorded dictionary or prefix",
        ));
    }
    decode_with_context(plan, stored, logical_len, CodecContext::default())
}

pub(crate) fn decode_payload_with_dictionary(
    plan: &TransformPlan,
    stored: &[u8],
    logical_len: u64,
    dictionary: &Dictionary,
) -> Result<Vec<u8>> {
    validate_dictionary(dictionary)?;
    if plan_mode(plan)? != PlanMode::Dictionary(dictionary.dictionary_id) {
        return Err(invalid_parameters(
            "TransformPlan and Dictionary dependency do not agree",
        ));
    }
    decode_with_context(
        plan,
        stored,
        logical_len,
        CodecContext {
            dictionary: Some(dictionary),
            prefix: None,
        },
    )
}

pub(crate) fn decode_payload_with_prefix(
    plan: &TransformPlan,
    stored: &[u8],
    logical_len: u64,
    prefix: &[u8],
) -> Result<Vec<u8>> {
    if !matches!(plan_mode(plan)?, PlanMode::Prefix { .. }) {
        return Err(invalid_parameters(
            "non-prefix TransformPlan cannot consume lookback bytes",
        ));
    }
    decode_with_context(
        plan,
        stored,
        logical_len,
        CodecContext {
            dictionary: None,
            prefix: Some(prefix),
        },
    )
}

fn codec_registration(identifier: &str) -> Result<&'static CodecRegistration> {
    let registration = CODECS
        .iter()
        .find(|registration| registration.identifier == identifier)
        .ok_or_else(|| {
            Diagnostic::new(
                OutcomeClass::Unsupported,
                ReasonCode::UnknownCodec,
                format!("required codec '{identifier}' is not registered"),
            )
        })?;
    debug_assert_ne!(registration.format_version, 0);
    Ok(registration)
}

fn encode_with_context(
    plan: &TransformPlan,
    plaintext: &[u8],
    context: CodecContext<'_>,
) -> Result<Vec<u8>> {
    validate_plan(plan)?;
    let transformed = forward_pipeline(&plan.transforms, plaintext)?;
    (codec_registration(&plan.codec)?.encode)(plan, &transformed, context)
}

fn decode_with_context(
    plan: &TransformPlan,
    stored: &[u8],
    logical_len: u64,
    context: CodecContext<'_>,
) -> Result<Vec<u8>> {
    validate_plan(plan)?;
    let transformed =
        (codec_registration(&plan.codec)?.decode)(plan, stored, logical_len, context)?;
    let plaintext = inverse_pipeline(&plan.transforms, &transformed)?;
    validate_decoded_length(plaintext, logical_len)
}

fn validate_store_registration(_plan: &TransformPlan) -> Result<TransformPlan> {
    Ok(store_plan())
}

fn encode_store_registration(
    _plan: &TransformPlan,
    input: &[u8],
    context: CodecContext<'_>,
) -> Result<Vec<u8>> {
    require_empty_context(context, STORE_CODEC_IDENTIFIER)?;
    Ok(input.to_vec())
}

fn decode_store_registration(
    _plan: &TransformPlan,
    stored: &[u8],
    logical_len: u64,
    context: CodecContext<'_>,
) -> Result<Vec<u8>> {
    require_empty_context(context, STORE_CODEC_IDENTIFIER)?;
    validate_decoded_length(stored.to_vec(), logical_len)
}

fn validate_zstd_registration(plan: &TransformPlan) -> Result<TransformPlan> {
    let parameters = parse_zstd_parameters(plan)?;
    let base = match parameters.mode {
        PlanMode::Independent => zstd_plan(parameters.level)?,
        PlanMode::Dictionary(dictionary_id) => {
            zstd_dictionary_plan(parameters.level, dictionary_id)?
        }
        PlanMode::Prefix { lookback } => zstd_prefix_plan(parameters.level, lookback)?,
    };
    if parameters.mode == PlanMode::Independent {
        with_pipeline(base, plan.transforms.clone())
    } else if plan.transforms.is_empty() {
        Ok(base)
    } else {
        Err(invalid_parameters(
            "dictionary and prefix Zstandard plans cannot carry structural transforms in v4",
        ))
    }
}

fn encode_zstd_registration(
    plan: &TransformPlan,
    input: &[u8],
    context: CodecContext<'_>,
) -> Result<Vec<u8>> {
    let parameters = parse_zstd_parameters(plan)?;
    match parameters.mode {
        PlanMode::Independent => {
            require_empty_context(context, ZSTD_CODEC_IDENTIFIER)?;
            encode_zstd(input, parameters.level, None)
        }
        PlanMode::Dictionary(dictionary_id) => {
            let dictionary = context.dictionary.ok_or_else(|| {
                invalid_parameters("dictionary Zstandard plan requires its Dictionary")
            })?;
            if dictionary.dictionary_id != dictionary_id || context.prefix.is_some() {
                return Err(invalid_parameters(
                    "Zstandard Dictionary context does not match TransformPlan",
                ));
            }
            encode_zstd(input, parameters.level, Some(&dictionary.bytes))
        }
        PlanMode::Prefix { .. } => {
            let prefix = context.prefix.ok_or_else(|| {
                invalid_parameters("prefix Zstandard plan requires preceding plaintext")
            })?;
            if context.dictionary.is_some() {
                return Err(invalid_parameters(
                    "prefix Zstandard plan cannot consume a Dictionary",
                ));
            }
            encode_zstd_prefix(input, parameters.level, prefix)
        }
    }
}

fn decode_zstd_registration(
    plan: &TransformPlan,
    stored: &[u8],
    logical_len: u64,
    context: CodecContext<'_>,
) -> Result<Vec<u8>> {
    let parameters = parse_zstd_parameters(plan)?;
    match parameters.mode {
        PlanMode::Independent => {
            require_empty_context(context, ZSTD_CODEC_IDENTIFIER)?;
            decode_zstd(stored, logical_len, None)
        }
        PlanMode::Dictionary(dictionary_id) => {
            let dictionary = context.dictionary.ok_or_else(|| {
                invalid_parameters("dictionary Zstandard plan requires its Dictionary")
            })?;
            if dictionary.dictionary_id != dictionary_id || context.prefix.is_some() {
                return Err(invalid_parameters(
                    "Zstandard Dictionary context does not match TransformPlan",
                ));
            }
            decode_zstd(stored, logical_len, Some(&dictionary.bytes))
        }
        PlanMode::Prefix { .. } => {
            let prefix = context.prefix.ok_or_else(|| {
                invalid_parameters("prefix Zstandard plan requires preceding plaintext")
            })?;
            if context.dictionary.is_some() {
                return Err(invalid_parameters(
                    "prefix Zstandard plan cannot consume a Dictionary",
                ));
            }
            decode_zstd_prefix(stored, logical_len, prefix)
        }
    }
}

fn validate_lz4_registration(plan: &TransformPlan) -> Result<TransformPlan> {
    if plan.codec_params.as_ref() != [b'L', b'4', b'0', b'1', 1, 0, 0, 0]
        || plan.dictionary.is_some()
    {
        return Err(invalid_parameters(
            "LZ4 parameters must declare raw block v1 with no external dependency",
        ));
    }
    lz4_plan(plan.transforms.clone())
}

fn encode_lz4_registration(
    _plan: &TransformPlan,
    input: &[u8],
    context: CodecContext<'_>,
) -> Result<Vec<u8>> {
    require_empty_context(context, LZ4_CODEC_IDENTIFIER)?;
    Ok(lz4_flex::block::compress(input))
}

fn decode_lz4_registration(
    _plan: &TransformPlan,
    stored: &[u8],
    logical_len: u64,
    context: CodecContext<'_>,
) -> Result<Vec<u8>> {
    require_empty_context(context, LZ4_CODEC_IDENTIFIER)?;
    let capacity = logical_capacity(logical_len)?;
    let mut output = vec![0_u8; capacity];
    let decoded = lz4_flex::block::decompress_into(stored, &mut output)
        .map_err(|error| decompression(format!("decode LZ4 block: {error}")))?;
    if decoded != capacity {
        return Err(Diagnostic::new(
            OutcomeClass::Corrupt,
            ReasonCode::DecompressedLengthMismatch,
            format!("LZ4 decoded {decoded} bytes but expected {capacity}"),
        ));
    }
    Ok(output)
}

fn validate_lzma2_registration(plan: &TransformPlan) -> Result<TransformPlan> {
    let (preset, dictionary_bytes) = parse_lzma2_parameters(plan)?;
    lzma2_plan(preset, dictionary_bytes, plan.transforms.clone())
}

fn encode_lzma2_registration(
    plan: &TransformPlan,
    input: &[u8],
    context: CodecContext<'_>,
) -> Result<Vec<u8>> {
    require_empty_context(context, LZMA2_CODEC_IDENTIFIER)?;
    let (preset, dictionary_bytes) = parse_lzma2_parameters(plan)?;
    let mut options = lzma_rust2::Lzma2Options::with_preset(u32::from(preset));
    options.lzma_options.dict_size = dictionary_bytes;
    options.chunk_size = None;
    let mut writer = lzma_rust2::Lzma2Writer::new(Vec::new(), options);
    writer
        .write_all(input)
        .map_err(|error| compression(format!("encode raw LZMA2 stream: {error}")))?;
    writer
        .finish()
        .map_err(|error| compression(format!("finish raw LZMA2 stream: {error}")))
}

fn decode_lzma2_registration(
    plan: &TransformPlan,
    stored: &[u8],
    logical_len: u64,
    context: CodecContext<'_>,
) -> Result<Vec<u8>> {
    require_empty_context(context, LZMA2_CODEC_IDENTIFIER)?;
    let (_, dictionary_bytes) = parse_lzma2_parameters(plan)?;
    let capacity = logical_capacity(logical_len)?;
    let cursor = Cursor::new(stored);
    let mut reader = lzma_rust2::Lzma2Reader::new(cursor, dictionary_bytes, None);
    let mut output = Vec::with_capacity(capacity);
    reader
        .by_ref()
        .take(logical_len.saturating_add(1))
        .read_to_end(&mut output)
        .map_err(|error| decompression(format!("decode raw LZMA2 stream: {error}")))?;
    let consumed = reader.into_inner().position();
    if consumed != u64::try_from(stored.len()).unwrap_or(u64::MAX) {
        return Err(decompression(
            "raw LZMA2 payload contains trailing or unconsumed bytes",
        ));
    }
    validate_decoded_length(output, logical_len)
}

fn require_empty_context(context: CodecContext<'_>, codec: &str) -> Result<()> {
    if context.dictionary.is_none() && context.prefix.is_none() {
        Ok(())
    } else {
        Err(invalid_parameters(format!(
            "codec '{codec}' does not accept Dictionary or prefix context"
        )))
    }
}

fn encode_zstd_prefix(input: &[u8], level: i32, prefix: &[u8]) -> Result<Vec<u8>> {
    let mut encoder = zstd::stream::write::Encoder::with_ref_prefix(Vec::new(), level, prefix)
        .map_err(|error| compression(format!("create Zstandard prefix context: {error}")))?;
    encoder
        .window_log(u32::from(ZSTD_WINDOW_LOG))
        .and_then(|()| encoder.long_distance_matching(false))
        .and_then(|()| encoder.include_checksum(false))
        .and_then(|()| encoder.include_contentsize(true))
        .and_then(|()| encoder.include_dictid(false))
        .and_then(|()| encoder.set_pledged_src_size(Some(input.len() as u64)))
        .map_err(|error| compression(format!("set Zstandard prefix parameters: {error}")))?;
    encoder
        .write_all(input)
        .map_err(|error| compression(format!("encode Zstandard prefix frame: {error}")))?;
    encoder
        .finish()
        .map_err(|error| compression(format!("finish Zstandard prefix frame: {error}")))
}

fn decode_zstd_prefix(stored: &[u8], logical_len: u64, prefix: &[u8]) -> Result<Vec<u8>> {
    let capacity = logical_capacity(logical_len)?;
    let reader = BufReader::new(Cursor::new(stored));
    let mut decoder = zstd::stream::read::Decoder::with_ref_prefix(reader, prefix)
        .map_err(|error| decompression(format!("create Zstandard prefix context: {error}")))?
        .single_frame();
    decoder
        .window_log_max(u32::from(ZSTD_WINDOW_LOG))
        .map_err(|error| decompression(format!("set Zstandard window limit: {error}")))?;
    let mut output = Vec::with_capacity(capacity);
    decoder
        .take(logical_len.saturating_add(1))
        .read_to_end(&mut output)
        .map_err(|error| decompression(format!("decode Zstandard prefix frame: {error}")))?;
    validate_decoded_length(output, logical_len)
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
    match plan.codec.as_str() {
        STORE_CODEC_IDENTIFIER | LZ4_CODEC_IDENTIFIER | LZMA2_CODEC_IDENTIFIER => {
            Ok(PlanMode::Independent)
        }
        ZSTD_CODEC_IDENTIFIER => Ok(parse_zstd_parameters(plan)?.mode),
        identifier => {
            codec_registration(identifier)?;
            Err(invalid_parameters(
                "registered codec has no dependency mode",
            ))
        }
    }
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

fn parse_lzma2_parameters(plan: &TransformPlan) -> Result<(u8, u32)> {
    let parameters = &plan.codec_params;
    if parameters.len() != 12
        || parameters[..4] != LZMA2_PARAMETER_MAGIC
        || parameters[5..8] != [0, 0, 0]
        || plan.dictionary.is_some()
    {
        return Err(invalid_parameters(
            "LZMA2 parameters must declare raw-v1, single-threaded encoding with no external dependency",
        ));
    }
    let preset = parameters[4];
    let dictionary_bytes = u32::from_be_bytes(
        parameters[8..12]
            .try_into()
            .map_err(|_| invalid_parameters("LZMA2 dictionary size must be a big-endian u32"))?,
    );
    if !SUPPORTED_LZMA2_CONFIGURATIONS.contains(&(preset, dictionary_bytes)) {
        return Err(invalid_parameters(format!(
            "LZMA2 preset {preset} with dictionary {dictionary_bytes} is not registered"
        )));
    }
    Ok((preset, dictionary_bytes))
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
    fn speed_density_and_transformed_plans_round_trip_without_hidden_state() {
        let slowly_varying = (0..32 * 1024)
            .map(|index| u8::try_from((index / 64) % 256).unwrap())
            .collect::<Vec<_>>();
        let plans = [
            lz4_plan(Box::default()).unwrap(),
            lzma2_plan(4, 1024 * 1024, Box::default()).unwrap(),
            zstd_transformed_plan(3, vec![crate::transform::delta8_step()].into()).unwrap(),
            lzma2_plan(
                6,
                4 * 1024 * 1024,
                vec![crate::transform::byte_shuffle_step(4).unwrap()].into(),
            )
            .unwrap(),
        ];
        for plan in plans {
            validate_plan(&plan).unwrap();
            let stored = encode_payload(&plan, &slowly_varying).unwrap();
            assert_eq!(
                decode_payload(&plan, &stored, slowly_varying.len() as u64).unwrap(),
                slowly_varying
            );
        }
    }

    #[test]
    fn malformed_new_codec_parameters_and_payloads_are_typed() {
        let mut invalid = lz4_plan(Box::default()).unwrap();
        invalid.codec_params[4] = 2;
        assert_eq!(
            validate_plan(&invalid).unwrap_err().code(),
            ReasonCode::InvalidCodecParameters
        );
        let plan = lzma2_plan(4, 1024 * 1024, Box::default()).unwrap();
        assert_eq!(
            decode_payload(&plan, b"not-lzma2", 100).unwrap_err().code(),
            ReasonCode::DecompressionFailed
        );
        let plan = lz4_plan(Box::default()).unwrap();
        assert_eq!(
            decode_payload(&plan, &[0xff], 100).unwrap_err().code(),
            ReasonCode::DecompressionFailed
        );

        let transformed =
            zstd_transformed_plan(3, vec![crate::transform::delta8_step()].into()).unwrap();
        let original = vec![17_u8; 4096];
        let mut altered = original.clone();
        altered[2048] ^= 1;
        let stored = encode_payload(&transformed, &altered).unwrap();
        let decoded = decode_payload(&transformed, &stored, altered.len() as u64).unwrap();
        assert_ne!(sha256_exact(&decoded), sha256_exact(&original));
        assert_eq!(
            decode_payload(&transformed, &stored, altered.len() as u64 + 1)
                .unwrap_err()
                .code(),
            ReasonCode::DecompressedLengthMismatch
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
