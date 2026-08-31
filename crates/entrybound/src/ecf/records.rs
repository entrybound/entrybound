use std::collections::BTreeMap;

use crate::canonical::{Record, RecordBuilder, decode_record, decode_record_stream};
use crate::diagnostics::{Diagnostic, OutcomeClass, ReasonCode, Result};
use crate::eam::{
    ChunkGroup, ConversionProvenance, ConversionResolution, Criticality, DecodeRequirements,
    Dictionary, Digest, Entry, EntryData, EntryIdentity, EntrySet, FidelityIssue, FidelityReport,
    LogicalPath, MetadataItem, MetadataName, MetadataSet, PathComponent, PathEncoding,
    ReconstructionAudit, ReconstructionAuditReason, ReconstructionAuditTarget, ReconstructionData,
    ReconstructionFallbackReason, ReconstructionRegion, RegionAccessCost, ResourceBudget,
    Restorability, Timestamp, TimestampPrecision, TransformPlan, TransformStep,
};

pub(super) const RECORD_DESCRIPTOR: u16 = 1;
pub(super) const RECORD_TRANSFORM_PLAN: u16 = 2;
pub(super) const RECORD_ENTRY: u16 = 3;
pub(super) const RECORD_CONTENT_OBJECT: u16 = 4;
pub(super) const RECORD_FIDELITY: u16 = 5;
pub(super) const RECORD_INDEX_ENTRY: u16 = 6;
const RECORD_PATH_COMPONENT: u16 = 7;
const RECORD_METADATA_ITEM: u16 = 8;
const RECORD_TIMESTAMP: u16 = 9;
const RECORD_FIDELITY_ISSUE: u16 = 10;
pub(super) const RECORD_DICTIONARY: u16 = 11;
pub(super) const RECORD_CHUNK_GROUP: u16 = 12;
pub(super) const RECORD_TRANSFORM_STEP: u16 = 13;
pub(super) const RECORD_RECONSTRUCTION_DATA: u16 = 14;
pub(super) const RECORD_TRANSFORM_STEP_V2: u16 = 15;
pub(super) const RECORD_RECONSTRUCTION_FALLBACK: u16 = 16;
pub(super) const RECORD_TRANSFORM_STEP_V3: u16 = 17;
pub(super) const RECORD_RECONSTRUCTION_REGION: u16 = 18;
pub(super) const RECORD_RECONSTRUCTION_AUDIT_V2: u16 = 19;
pub(super) const RECORD_CONVERSION_PROVENANCE: u16 = 28;
const RECORD_CONVERSION_RESOLUTION: u16 = 29;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct DescriptorBody {
    pub namespace: String,
    pub identity_profile: u8,
    pub digest_algorithm: u8,
    pub planner_id: String,
    pub chunker_id: String,
    pub lai: Digest,
    pub pcr: Digest,
    pub aux: Digest,
    /// Present exactly for Descriptor record version 2.
    pub declarations: Option<DescriptorDeclarations>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct DescriptorDeclarations {
    pub decode: DecodeRequirements,
    pub budget: ResourceBudget,
}

pub(super) fn encode_descriptor(value: &DescriptorBody) -> Result<Vec<u8>> {
    let mut record = if value.declarations.is_some() {
        RecordBuilder::new_version(RECORD_DESCRIPTOR, 2)
    } else {
        RecordBuilder::new(RECORD_DESCRIPTOR)
    };
    record
        .utf8(1, &value.namespace)?
        .u8(2, value.identity_profile)?
        .u8(3, value.digest_algorithm)?
        .utf8(4, &value.planner_id)?
        .utf8(5, &value.chunker_id)?
        .bytes(6, value.lai.as_bytes())?
        .bytes(7, value.pcr.as_bytes())?
        .bytes(8, value.aux.as_bytes())?;
    if let Some(declarations) = value.declarations {
        record
            .u64(9, declarations.decode.window_bytes)?
            .u64(10, declarations.decode.working_set_bytes)?
            .u32(11, declarations.decode.flags)?
            .u64(12, declarations.budget.entry_count)?
            .u64(13, declarations.budget.total_logical_bytes)?
            .u64(14, declarations.budget.max_single_entry_logical_bytes)?
            .u64(15, declarations.budget.max_expansion_ratio_milli)?
            .u64(16, declarations.budget.chunk_count)?
            .u64(17, declarations.budget.max_path_depth)?
            .u64(18, declarations.budget.max_metadata_bytes)?
            .u64(19, declarations.budget.max_key_derivation_cost)?;
    }
    record.finish()
}

pub(super) fn decode_descriptor(bytes: &[u8]) -> Result<DescriptorBody> {
    let (record, consumed) = decode_record(bytes)?;
    if consumed != bytes.len() || record.kind != RECORD_DESCRIPTOR {
        return Err(noncanonical(
            "DESCRIPTOR must contain exactly one Descriptor record",
        ));
    }
    let declarations = match record.version {
        1 => {
            record.expect_versioned_tags(1, &[1, 2, 3, 4, 5, 6, 7, 8], &[])?;
            None
        }
        2 => {
            record.expect_versioned_tags(
                2,
                &[
                    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19,
                ],
                &[],
            )?;
            Some(DescriptorDeclarations {
                decode: DecodeRequirements {
                    window_bytes: record.field(9)?.as_u64()?,
                    working_set_bytes: record.field(10)?.as_u64()?,
                    flags: record.field(11)?.as_u32()?,
                },
                budget: ResourceBudget {
                    entry_count: record.field(12)?.as_u64()?,
                    total_logical_bytes: record.field(13)?.as_u64()?,
                    max_single_entry_logical_bytes: record.field(14)?.as_u64()?,
                    max_expansion_ratio_milli: record.field(15)?.as_u64()?,
                    chunk_count: record.field(16)?.as_u64()?,
                    max_path_depth: record.field(17)?.as_u64()?,
                    max_metadata_bytes: record.field(18)?.as_u64()?,
                    max_key_derivation_cost: record.field(19)?.as_u64()?,
                },
            })
        }
        _ => return Err(noncanonical("unsupported Descriptor record version")),
    };
    let value = DescriptorBody {
        namespace: record.field(1)?.as_utf8()?.to_owned(),
        identity_profile: record.field(2)?.as_u8()?,
        digest_algorithm: record.field(3)?.as_u8()?,
        planner_id: record.field(4)?.as_utf8()?.to_owned(),
        chunker_id: record.field(5)?.as_utf8()?.to_owned(),
        lai: digest(record.field(6)?.as_bytes()?)?,
        pcr: digest(record.field(7)?.as_bytes()?)?,
        aux: digest(record.field(8)?.as_bytes()?)?,
        declarations,
    };
    if encode_descriptor(&value)? != bytes {
        return Err(noncanonical("Descriptor record is not in canonical form"));
    }
    Ok(value)
}

pub(super) fn encode_transform_plans(
    plans: &[TransformPlan],
    transform_steps: bool,
) -> Result<Vec<u8>> {
    let mut encoded = Vec::new();
    let mut previous = None;
    for plan in plans {
        if previous.is_some_and(|id| id >= plan.plan_id) {
            return Err(noncanonical(
                "TransformPlans must be strictly ordered by plan_id",
            ));
        }
        previous = Some(plan.plan_id);
        if !transform_steps && !plan.transforms.is_empty() {
            return Err(Diagnostic::new(
                OutcomeClass::Unsupported,
                ReasonCode::UnsupportedRequiredFeature,
                "first-class TransformSteps require codec-transform-v1",
            ));
        }
        let transforms = if transform_steps {
            plan.transforms
                .iter()
                .map(encode_transform_step_v1)
                .collect::<Result<Vec<_>>>()?
        } else {
            Vec::new()
        };
        let mut record = RecordBuilder::new(RECORD_TRANSFORM_PLAN);
        record
            .u64(1, plan.plan_id)?
            .utf8(2, &plan.identifier)?
            .sequence(3, &transforms)?
            .utf8(4, &plan.codec)?
            .bytes(5, &plan.codec_params)?;
        if let Some(dictionary) = plan.dictionary {
            record.bytes(6, dictionary.as_bytes())?;
        }
        record
            .u64(7, plan.decode.window_bytes)?
            .u64(8, plan.decode.working_set_bytes)?
            .u32(9, plan.decode.flags)?;
        encoded.extend_from_slice(&record.finish()?);
    }
    Ok(encoded)
}

pub(super) fn decode_transform_plans(
    bytes: &[u8],
    transform_steps: bool,
) -> Result<Box<[TransformPlan]>> {
    let records = decode_record_stream(bytes)?;
    let mut plans = Vec::with_capacity(records.len());
    let mut previous = None;
    for record in records {
        if record.kind != RECORD_TRANSFORM_PLAN {
            return Err(noncanonical("TRANSFORM_PLANS contains a non-plan record"));
        }
        record.expect_tags(&[1, 2, 3, 4, 5, 7, 8, 9], &[6])?;
        let transform_items = record.field(3)?.as_sequence()?;
        let transforms = if transform_steps {
            transform_items
                .into_iter()
                .map(decode_transform_step_v1)
                .collect::<Result<Vec<_>>>()?
        } else if transform_items.is_empty() {
            Vec::new()
        } else {
            return Err(Diagnostic::new(
                OutcomeClass::Unsupported,
                ReasonCode::UnknownTransform,
                "legacy transform-string placeholders were never operational",
            ));
        };
        let plan_id = record.field(1)?.as_u64()?;
        if previous.is_some_and(|id| id >= plan_id) {
            return Err(noncanonical(
                "TransformPlans must be strictly ordered by plan_id",
            ));
        }
        previous = Some(plan_id);
        plans.push(TransformPlan {
            plan_id,
            identifier: record.field(2)?.as_utf8()?.to_owned(),
            transforms: transforms.into_boxed_slice(),
            codec: record.field(4)?.as_utf8()?.to_owned(),
            codec_params: record.field(5)?.as_bytes()?.into(),
            dictionary: record
                .optional_field(6)
                .map(|field| digest(field.as_bytes()?))
                .transpose()?,
            decode: crate::eam::DecodeRequirements {
                window_bytes: record.field(7)?.as_u64()?,
                working_set_bytes: record.field(8)?.as_u64()?,
                flags: record.field(9)?.as_u32()?,
            },
        });
    }
    let plans = plans.into_boxed_slice();
    crate::codec::validate_plans(&plans)?;
    if encode_transform_plans(&plans, transform_steps)? != bytes {
        return Err(noncanonical("TransformPlan records are not canonical"));
    }
    Ok(plans)
}

fn encode_transform_step_v1(step: &TransformStep) -> Result<Vec<u8>> {
    if step.reconstruction_ref.is_some() {
        return Err(Diagnostic::new(
            OutcomeClass::Unsupported,
            ReasonCode::UnsupportedRequiredFeature,
            "TransformStep reconstruction_ref requires reconstructive-transform-v1",
        ));
    }
    let mut record = RecordBuilder::new(RECORD_TRANSFORM_STEP);
    record
        .utf8(1, &step.transform_id)?
        .bytes(2, &step.parameters)?;
    record.finish()
}

fn decode_transform_step_v1(bytes: &[u8]) -> Result<TransformStep> {
    let (record, consumed) = decode_record(bytes)?;
    if consumed != bytes.len() || record.kind != RECORD_TRANSFORM_STEP {
        return Err(noncanonical(
            "TransformStep sequence item must be exactly one step record",
        ));
    }
    record.expect_tags(&[1, 2], &[])?;
    let step = TransformStep {
        transform_id: record.field(1)?.as_utf8()?.to_owned(),
        parameters: record.field(2)?.as_bytes()?.into(),
        reconstruction_ref: None,
    };
    if encode_transform_step_v1(&step)? != bytes {
        return Err(noncanonical("TransformStep record is not canonical"));
    }
    Ok(step)
}

pub(super) fn encode_transform_plans_v2(plans: &[TransformPlan]) -> Result<Vec<u8>> {
    let mut encoded = Vec::new();
    let mut previous = None;
    for plan in plans {
        if previous.is_some_and(|id| id >= plan.plan_id) {
            return Err(noncanonical(
                "TransformPlans must be strictly ordered by plan_id",
            ));
        }
        previous = Some(plan.plan_id);
        let transforms = plan
            .transforms
            .iter()
            .map(encode_transform_step_v2)
            .collect::<Result<Vec<_>>>()?;
        let mut record = RecordBuilder::new(RECORD_TRANSFORM_PLAN);
        record
            .u64(1, plan.plan_id)?
            .utf8(2, &plan.identifier)?
            .sequence(3, &transforms)?
            .utf8(4, &plan.codec)?
            .bytes(5, &plan.codec_params)?;
        if let Some(dictionary) = plan.dictionary {
            record.bytes(6, dictionary.as_bytes())?;
        }
        record
            .u64(7, plan.decode.window_bytes)?
            .u64(8, plan.decode.working_set_bytes)?
            .u32(9, plan.decode.flags)?;
        encoded.extend_from_slice(&record.finish()?);
    }
    Ok(encoded)
}

pub(super) fn decode_transform_plans_v2(bytes: &[u8]) -> Result<Box<[TransformPlan]>> {
    let records = decode_record_stream(bytes)?;
    let mut plans = Vec::with_capacity(records.len());
    let mut previous = None;
    for record in records {
        if record.kind != RECORD_TRANSFORM_PLAN {
            return Err(noncanonical("TRANSFORM_PLANS contains a non-plan record"));
        }
        record.expect_tags(&[1, 2, 3, 4, 5, 7, 8, 9], &[6])?;
        let plan_id = record.field(1)?.as_u64()?;
        if previous.is_some_and(|id| id >= plan_id) {
            return Err(noncanonical(
                "TransformPlans must be strictly ordered by plan_id",
            ));
        }
        previous = Some(plan_id);
        let transforms = record
            .field(3)?
            .as_sequence()?
            .into_iter()
            .map(decode_transform_step_v2)
            .collect::<Result<Vec<_>>>()?;
        plans.push(TransformPlan {
            plan_id,
            identifier: record.field(2)?.as_utf8()?.to_owned(),
            transforms: transforms.into_boxed_slice(),
            codec: record.field(4)?.as_utf8()?.to_owned(),
            codec_params: record.field(5)?.as_bytes()?.into(),
            dictionary: record
                .optional_field(6)
                .map(|field| digest(field.as_bytes()?))
                .transpose()?,
            decode: crate::eam::DecodeRequirements {
                window_bytes: record.field(7)?.as_u64()?,
                working_set_bytes: record.field(8)?.as_u64()?,
                flags: record.field(9)?.as_u32()?,
            },
        });
    }
    let plans = plans.into_boxed_slice();
    crate::codec::validate_plans(&plans)?;
    if encode_transform_plans_v2(&plans)? != bytes {
        return Err(noncanonical("TransformPlan v2 records are not canonical"));
    }
    Ok(plans)
}

fn encode_transform_step_v2(step: &TransformStep) -> Result<Vec<u8>> {
    let mut record = RecordBuilder::new(RECORD_TRANSFORM_STEP_V2);
    record
        .utf8(1, &step.transform_id)?
        .bytes(2, &step.parameters)?;
    if let Some(reference) = step.reconstruction_ref {
        record.bytes(3, reference.as_bytes())?;
    }
    record.finish()
}

fn decode_transform_step_v2(bytes: &[u8]) -> Result<TransformStep> {
    let (record, consumed) = decode_record(bytes)?;
    if consumed != bytes.len() || record.kind != RECORD_TRANSFORM_STEP_V2 {
        return Err(noncanonical(
            "TransformStep v2 sequence item must be exactly one v2 step record",
        ));
    }
    record.expect_tags(&[1, 2], &[3])?;
    let step = TransformStep {
        transform_id: record.field(1)?.as_utf8()?.to_owned(),
        parameters: record.field(2)?.as_bytes()?.into(),
        reconstruction_ref: record
            .optional_field(3)
            .map(|field| digest(field.as_bytes()?))
            .transpose()?,
    };
    if encode_transform_step_v2(&step)? != bytes {
        return Err(noncanonical("TransformStep v2 record is not canonical"));
    }
    Ok(step)
}

pub(super) fn encode_transform_plans_v3(plans: &[TransformPlan]) -> Result<Vec<u8>> {
    encode_transform_plans_with(plans, encode_transform_step_v3)
}

pub(super) fn decode_transform_plans_v3(bytes: &[u8]) -> Result<Box<[TransformPlan]>> {
    decode_transform_plans_with(bytes, decode_transform_step_v3, encode_transform_plans_v3)
}

fn encode_transform_plans_with(
    plans: &[TransformPlan],
    encode_step: fn(&TransformStep) -> Result<Vec<u8>>,
) -> Result<Vec<u8>> {
    let mut encoded = Vec::new();
    let mut previous = None;
    for plan in plans {
        if previous.is_some_and(|id| id >= plan.plan_id) {
            return Err(noncanonical(
                "TransformPlans must be strictly ordered by plan_id",
            ));
        }
        previous = Some(plan.plan_id);
        let transforms = plan
            .transforms
            .iter()
            .map(encode_step)
            .collect::<Result<Vec<_>>>()?;
        let mut record = RecordBuilder::new(RECORD_TRANSFORM_PLAN);
        record
            .u64(1, plan.plan_id)?
            .utf8(2, &plan.identifier)?
            .sequence(3, &transforms)?
            .utf8(4, &plan.codec)?
            .bytes(5, &plan.codec_params)?;
        if let Some(dictionary) = plan.dictionary {
            record.bytes(6, dictionary.as_bytes())?;
        }
        record
            .u64(7, plan.decode.window_bytes)?
            .u64(8, plan.decode.working_set_bytes)?
            .u32(9, plan.decode.flags)?;
        encoded.extend_from_slice(&record.finish()?);
    }
    Ok(encoded)
}

fn decode_transform_plans_with(
    bytes: &[u8],
    decode_step: fn(&[u8]) -> Result<TransformStep>,
    encode_plans: fn(&[TransformPlan]) -> Result<Vec<u8>>,
) -> Result<Box<[TransformPlan]>> {
    let records = decode_record_stream(bytes)?;
    let mut plans = Vec::with_capacity(records.len());
    let mut previous = None;
    for record in records {
        if record.kind != RECORD_TRANSFORM_PLAN {
            return Err(noncanonical("TRANSFORM_PLANS contains a non-plan record"));
        }
        record.expect_tags(&[1, 2, 3, 4, 5, 7, 8, 9], &[6])?;
        let plan_id = record.field(1)?.as_u64()?;
        if previous.is_some_and(|id| id >= plan_id) {
            return Err(noncanonical(
                "TransformPlans must be strictly ordered by plan_id",
            ));
        }
        previous = Some(plan_id);
        let transforms = record
            .field(3)?
            .as_sequence()?
            .into_iter()
            .map(decode_step)
            .collect::<Result<Vec<_>>>()?;
        plans.push(TransformPlan {
            plan_id,
            identifier: record.field(2)?.as_utf8()?.to_owned(),
            transforms: transforms.into_boxed_slice(),
            codec: record.field(4)?.as_utf8()?.to_owned(),
            codec_params: record.field(5)?.as_bytes()?.into(),
            dictionary: record
                .optional_field(6)
                .map(|field| digest(field.as_bytes()?))
                .transpose()?,
            decode: crate::eam::DecodeRequirements {
                window_bytes: record.field(7)?.as_u64()?,
                working_set_bytes: record.field(8)?.as_u64()?,
                flags: record.field(9)?.as_u32()?,
            },
        });
    }
    let plans = plans.into_boxed_slice();
    crate::codec::validate_plans(&plans)?;
    if encode_plans(&plans)? != bytes {
        return Err(noncanonical("TransformPlan v3 records are not canonical"));
    }
    Ok(plans)
}

fn encode_transform_step_v3(step: &TransformStep) -> Result<Vec<u8>> {
    let mut record = RecordBuilder::new(RECORD_TRANSFORM_STEP_V3);
    record
        .utf8(1, &step.transform_id)?
        .bytes(2, &step.parameters)?;
    if let Some(reference) = step.reconstruction_ref {
        record.bytes(3, reference.as_bytes())?;
    }
    record.finish()
}

fn decode_transform_step_v3(bytes: &[u8]) -> Result<TransformStep> {
    let (record, consumed) = decode_record(bytes)?;
    if consumed != bytes.len() || record.kind != RECORD_TRANSFORM_STEP_V3 {
        return Err(noncanonical(
            "TransformStep v3 sequence item must be exactly one v3 step record",
        ));
    }
    record.expect_tags(&[1, 2], &[3])?;
    let step = TransformStep {
        transform_id: record.field(1)?.as_utf8()?.to_owned(),
        parameters: record.field(2)?.as_bytes()?.into(),
        reconstruction_ref: record
            .optional_field(3)
            .map(|field| digest(field.as_bytes()?))
            .transpose()?,
    };
    if encode_transform_step_v3(&step)? != bytes {
        return Err(noncanonical("TransformStep v3 record is not canonical"));
    }
    Ok(step)
}

pub(super) fn encode_reconstruction_data(
    values: &BTreeMap<Digest, ReconstructionData>,
) -> Result<Vec<u8>> {
    encode_reconstruction_section(values, &BTreeMap::new())
}

pub(super) fn encode_reconstruction_section(
    values: &BTreeMap<Digest, ReconstructionData>,
    fallbacks: &BTreeMap<Digest, ReconstructionFallbackReason>,
) -> Result<Vec<u8>> {
    let mut encoded = Vec::new();
    for value in values.values() {
        let mut record = RecordBuilder::new(RECORD_RECONSTRUCTION_DATA);
        record
            .bytes(1, value.reconstruction_id.as_bytes())?
            .utf8(2, &value.format)?
            .u64(3, value.intermediate_len)?
            .bytes(4, &value.bytes)?;
        encoded.extend_from_slice(&record.finish()?);
    }
    for (chunk_id, reason) in fallbacks {
        let mut record = RecordBuilder::new(RECORD_RECONSTRUCTION_FALLBACK);
        record.bytes(1, chunk_id.as_bytes())?.u8(
            2,
            match reason {
                ReconstructionFallbackReason::UnrecognizedOrVerificationFailed => 1,
                ReconstructionFallbackReason::CompleteCostDidNotWin => 2,
            },
        )?;
        encoded.extend_from_slice(&record.finish()?);
    }
    Ok(encoded)
}

pub(super) fn decode_reconstruction_section(
    bytes: &[u8],
) -> Result<(
    BTreeMap<Digest, ReconstructionData>,
    BTreeMap<Digest, ReconstructionFallbackReason>,
)> {
    let mut values = BTreeMap::new();
    let mut fallbacks = BTreeMap::new();
    let mut previous = None;
    let mut fallbacks_started = false;
    for record in decode_record_stream(bytes)? {
        if record.kind == RECORD_RECONSTRUCTION_FALLBACK {
            fallbacks_started = true;
            record.expect_tags(&[1, 2], &[])?;
            let chunk_id = digest(record.field(1)?.as_bytes()?)?;
            let reason = match record.field(2)?.as_u8()? {
                1 => ReconstructionFallbackReason::UnrecognizedOrVerificationFailed,
                2 => ReconstructionFallbackReason::CompleteCostDidNotWin,
                _ => return Err(noncanonical("unknown ReconstructionFallback reason")),
            };
            if fallbacks.insert(chunk_id, reason).is_some() {
                return Err(duplicate("duplicate ReconstructionFallback Chunk"));
            }
            continue;
        }
        if record.kind != RECORD_RECONSTRUCTION_DATA || fallbacks_started {
            return Err(noncanonical(
                "RECONSTRUCTION_DATA record ordering is invalid",
            ));
        }
        record.expect_tags(&[1, 2, 3, 4], &[])?;
        let reconstruction_id = digest(record.field(1)?.as_bytes()?)?;
        if previous.is_some_and(|id| id >= reconstruction_id) {
            return Err(Diagnostic::new(
                OutcomeClass::Nonconforming,
                ReasonCode::DuplicateSemanticDeclaration,
                "ReconstructionData objects must be uniquely ordered by identity",
            ));
        }
        previous = Some(reconstruction_id);
        let value = ReconstructionData {
            reconstruction_id,
            format: record.field(2)?.as_utf8()?.to_owned(),
            intermediate_len: record.field(3)?.as_u64()?,
            bytes: record.field(4)?.as_bytes()?.into(),
        };
        crate::reconstruction::validate_data(&value)?;
        values.insert(reconstruction_id, value);
    }
    if encode_reconstruction_section(&values, &fallbacks)? != bytes {
        return Err(noncanonical("ReconstructionData records are not canonical"));
    }
    Ok((values, fallbacks))
}

pub(super) fn encode_reconstruction_regions(
    regions: &BTreeMap<Digest, ReconstructionRegion>,
    audits: &BTreeMap<ReconstructionAuditTarget, ReconstructionAudit>,
) -> Result<Vec<u8>> {
    let mut encoded = Vec::new();
    for region in regions.values() {
        let mut record = RecordBuilder::new(RECORD_RECONSTRUCTION_REGION);
        record
            .bytes(1, region.region_id.as_bytes())?
            .bytes(2, region.content_object.as_bytes())?
            .u64(3, region.start_chunk_index)?
            .u64(4, region.chunk_count)?
            .u64(5, region.plan_ref)?
            .u64(6, region.logical_bytes)?
            .u64(7, region.transformed_bytes)?
            .u64(8, region.access.logical_bytes)?
            .u64(9, region.access.logical_chunks)?
            .u64(10, region.access.worst_reconstructed_bytes)?
            .bytes(11, &region.representation)?
            .u64(12, region.ordinary_physical_bytes)?
            .u64(13, region.region_overhead_bytes)?;
        encoded.extend_from_slice(&record.finish()?);
    }
    for audit in audits.values() {
        let (target_kind, target_digest) = encode_audit_target(audit.target);
        let mut record = RecordBuilder::new(RECORD_RECONSTRUCTION_AUDIT_V2);
        record
            .u8(1, target_kind)?
            .bytes(2, target_digest.as_bytes())?
            .utf8(3, &audit.transform_id)?
            .u8(4, encode_audit_reason(audit.reason))?;
        encoded.extend_from_slice(&record.finish()?);
    }
    Ok(encoded)
}

pub(super) fn decode_reconstruction_regions(
    bytes: &[u8],
) -> Result<(
    BTreeMap<Digest, ReconstructionRegion>,
    BTreeMap<ReconstructionAuditTarget, ReconstructionAudit>,
)> {
    let mut regions = BTreeMap::new();
    let mut audits = BTreeMap::new();
    let mut audits_started = false;
    for record in decode_record_stream(bytes)? {
        match record.kind {
            RECORD_RECONSTRUCTION_REGION if !audits_started => {
                record.expect_tags(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13], &[])?;
                let region_id = digest(record.field(1)?.as_bytes()?)?;
                let region = ReconstructionRegion {
                    region_id,
                    content_object: digest(record.field(2)?.as_bytes()?)?,
                    start_chunk_index: record.field(3)?.as_u64()?,
                    chunk_count: record.field(4)?.as_u64()?,
                    plan_ref: record.field(5)?.as_u64()?,
                    logical_bytes: record.field(6)?.as_u64()?,
                    transformed_bytes: record.field(7)?.as_u64()?,
                    ordinary_physical_bytes: record.field(12)?.as_u64()?,
                    region_overhead_bytes: record.field(13)?.as_u64()?,
                    access: RegionAccessCost {
                        logical_bytes: record.field(8)?.as_u64()?,
                        logical_chunks: record.field(9)?.as_u64()?,
                        worst_reconstructed_bytes: record.field(10)?.as_u64()?,
                    },
                    representation: record.field(11)?.as_bytes()?.into(),
                };
                if regions.insert(region_id, region).is_some() {
                    return Err(duplicate("duplicate ReconstructionRegion identity"));
                }
            }
            RECORD_RECONSTRUCTION_AUDIT_V2 => {
                audits_started = true;
                record.expect_tags(&[1, 2, 3, 4], &[])?;
                let target = decode_audit_target(
                    record.field(1)?.as_u8()?,
                    digest(record.field(2)?.as_bytes()?)?,
                )?;
                let audit = ReconstructionAudit {
                    target,
                    transform_id: record.field(3)?.as_utf8()?.to_owned(),
                    reason: decode_audit_reason(record.field(4)?.as_u8()?)?,
                };
                if audits.insert(target, audit).is_some() {
                    return Err(duplicate("duplicate ReconstructionAudit target"));
                }
            }
            _ => {
                return Err(noncanonical(
                    "RECONSTRUCTION_REGIONS record ordering is invalid",
                ));
            }
        }
    }
    if encode_reconstruction_regions(&regions, &audits)? != bytes {
        return Err(noncanonical(
            "ReconstructionRegion records are not canonical",
        ));
    }
    Ok((regions, audits))
}

fn encode_audit_target(target: ReconstructionAuditTarget) -> (u8, Digest) {
    match target {
        ReconstructionAuditTarget::Chunk(digest) => (1, digest),
        ReconstructionAuditTarget::ContentObject(digest) => (2, digest),
        ReconstructionAuditTarget::Region(digest) => (3, digest),
    }
}

fn decode_audit_target(kind: u8, digest: Digest) -> Result<ReconstructionAuditTarget> {
    match kind {
        1 => Ok(ReconstructionAuditTarget::Chunk(digest)),
        2 => Ok(ReconstructionAuditTarget::ContentObject(digest)),
        3 => Ok(ReconstructionAuditTarget::Region(digest)),
        _ => Err(noncanonical("unknown ReconstructionAudit target kind")),
    }
}

fn encode_audit_reason(reason: ReconstructionAuditReason) -> u8 {
    match reason {
        ReconstructionAuditReason::NotRecognized => 1,
        ReconstructionAuditReason::Unsupported => 2,
        ReconstructionAuditReason::ExactVerificationFailed => 3,
        ReconstructionAuditReason::CompleteCostDidNotWin => 4,
        ReconstructionAuditReason::RegionDedupConflict => 5,
        ReconstructionAuditReason::ResourcePolicyExcluded => 6,
    }
}

fn decode_audit_reason(value: u8) -> Result<ReconstructionAuditReason> {
    match value {
        1 => Ok(ReconstructionAuditReason::NotRecognized),
        2 => Ok(ReconstructionAuditReason::Unsupported),
        3 => Ok(ReconstructionAuditReason::ExactVerificationFailed),
        4 => Ok(ReconstructionAuditReason::CompleteCostDidNotWin),
        5 => Ok(ReconstructionAuditReason::RegionDedupConflict),
        6 => Ok(ReconstructionAuditReason::ResourcePolicyExcluded),
        _ => Err(noncanonical("unknown ReconstructionAudit reason")),
    }
}

#[cfg(test)]
pub(super) fn decode_reconstruction_data(
    bytes: &[u8],
) -> Result<BTreeMap<Digest, ReconstructionData>> {
    let (values, fallbacks) = decode_reconstruction_section(bytes)?;
    if !fallbacks.is_empty() {
        return Err(noncanonical("unexpected ReconstructionFallback records"));
    }
    Ok(values)
}

pub(super) fn encode_dictionaries(dictionaries: &BTreeMap<Digest, Dictionary>) -> Result<Vec<u8>> {
    let mut encoded = Vec::new();
    for dictionary in dictionaries.values() {
        let mut record = RecordBuilder::new(RECORD_DICTIONARY);
        record
            .bytes(1, dictionary.dictionary_id.as_bytes())?
            .utf8(2, &dictionary.codec)?
            .utf8(3, &dictionary.format)?
            .utf8(4, &dictionary.construction)?
            .bytes(5, &dictionary.bytes)?;
        encoded.extend_from_slice(&record.finish()?);
    }
    Ok(encoded)
}

pub(super) fn decode_dictionaries(bytes: &[u8]) -> Result<BTreeMap<Digest, Dictionary>> {
    let mut dictionaries = BTreeMap::new();
    let mut previous = None;
    for record in decode_record_stream(bytes)? {
        if record.kind != RECORD_DICTIONARY {
            return Err(noncanonical(
                "DICTIONARIES contains a non-Dictionary record",
            ));
        }
        record.expect_tags(&[1, 2, 3, 4, 5], &[])?;
        let dictionary_id = digest(record.field(1)?.as_bytes()?)?;
        if previous.is_some_and(|value| value >= dictionary_id) {
            return Err(Diagnostic::new(
                OutcomeClass::Nonconforming,
                ReasonCode::DuplicateSemanticDeclaration,
                "Dictionaries must be uniquely ordered by dictionary_id",
            ));
        }
        previous = Some(dictionary_id);
        let dictionary = Dictionary {
            dictionary_id,
            codec: record.field(2)?.as_utf8()?.to_owned(),
            format: record.field(3)?.as_utf8()?.to_owned(),
            construction: record.field(4)?.as_utf8()?.to_owned(),
            bytes: record.field(5)?.as_bytes()?.into(),
        };
        crate::codec::validate_dictionary(&dictionary)?;
        dictionaries.insert(dictionary_id, dictionary);
    }
    if encode_dictionaries(&dictionaries)? != bytes {
        return Err(noncanonical("Dictionary records are not canonical"));
    }
    Ok(dictionaries)
}

pub(super) fn encode_chunk_groups(groups: &BTreeMap<Digest, ChunkGroup>) -> Result<Vec<u8>> {
    let mut encoded = Vec::new();
    for group in groups.values() {
        let mut record = RecordBuilder::new(RECORD_CHUNK_GROUP);
        record
            .bytes(1, group.group_id.as_bytes())?
            .u32(2, group.max_lookback)?
            .u64(3, group.max_preceding_bytes)?;
        encoded.extend_from_slice(&record.finish()?);
    }
    Ok(encoded)
}

pub(super) fn decode_chunk_groups(bytes: &[u8]) -> Result<BTreeMap<Digest, ChunkGroup>> {
    let mut groups = BTreeMap::new();
    let mut previous = None;
    for record in decode_record_stream(bytes)? {
        if record.kind != RECORD_CHUNK_GROUP {
            return Err(noncanonical("CHUNK_GROUPS contains a non-group record"));
        }
        record.expect_tags(&[1, 2, 3], &[])?;
        let group_id = digest(record.field(1)?.as_bytes()?)?;
        if previous.is_some_and(|value| value >= group_id) {
            return Err(Diagnostic::new(
                OutcomeClass::Nonconforming,
                ReasonCode::DuplicateSemanticDeclaration,
                "ChunkGroups must be uniquely ordered by group_id",
            ));
        }
        previous = Some(group_id);
        groups.insert(
            group_id,
            ChunkGroup {
                group_id,
                max_lookback: record.field(2)?.as_u32()?,
                max_preceding_bytes: record.field(3)?.as_u64()?,
            },
        );
    }
    if encode_chunk_groups(&groups)? != bytes {
        return Err(noncanonical("ChunkGroup records are not canonical"));
    }
    Ok(groups)
}

pub(super) fn encode_manifest(
    entries: &EntrySet,
    objects: &BTreeMap<Digest, crate::eam::ContentObject>,
) -> Result<Vec<u8>> {
    let mut encoded = Vec::new();
    for entry in entries.entries() {
        encoded.extend_from_slice(&encode_entry(entry)?);
    }
    for object in objects.values() {
        encoded.extend_from_slice(&encode_content_object_record(object)?);
    }
    Ok(encoded)
}

/// Encodes one canonical ContentObject record.
///
/// STREAM emits manifest records individually so each one can follow the
/// physical data it describes. The bytes are identical to this object's slice
/// of an INDEXED MANIFEST_RECORDS payload.
pub(super) fn encode_content_object_record(object: &crate::eam::ContentObject) -> Result<Vec<u8>> {
    let chunks = object
        .chunks
        .iter()
        .map(|chunk| chunk.chunk_id.as_bytes().to_vec())
        .collect::<Vec<_>>();
    let mut record = RecordBuilder::new(RECORD_CONTENT_OBJECT);
    record
        .bytes(1, object.logical_digest.as_bytes())?
        .bytes(2, object.chunk_root.as_bytes())?
        .sequence(3, &chunks)?;
    record.finish()
}

/// Encodes one canonical Entry record.
pub(super) fn encode_entry_record(entry: &Entry) -> Result<Vec<u8>> {
    encode_entry(entry)
}

/// One semantic record carried by a STREAM `MANIFEST_RECORD` item.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum ManifestRecord {
    Entry(Box<Entry>),
    ContentObject(Box<crate::eam::ContentObject>),
}

/// Decodes exactly one canonical manifest record.
///
/// The item payload must hold a single record and must re-encode to the exact
/// same bytes, so a STREAM manifest item is as strictly canonical as the
/// INDEXED section it corresponds to.
pub(super) fn decode_manifest_record(bytes: &[u8]) -> Result<ManifestRecord> {
    let records = decode_record_stream(bytes)?;
    let [record] = records.as_slice() else {
        return Err(noncanonical(
            "a MANIFEST_RECORD item must carry exactly one canonical record",
        ));
    };
    match record.kind {
        RECORD_ENTRY => {
            let entry = decode_entry(record)?;
            if encode_entry(&entry)? != bytes {
                return Err(noncanonical("Entry record is not byte-canonical"));
            }
            Ok(ManifestRecord::Entry(Box::new(entry)))
        }
        RECORD_CONTENT_OBJECT => {
            record.expect_tags(&[1, 2, 3], &[])?;
            let logical_digest = digest(record.field(1)?.as_bytes()?)?;
            let chunks = record
                .field(3)?
                .as_sequence()?
                .into_iter()
                .map(digest)
                .map(|result| result.map(|chunk_id| crate::eam::ChunkRef { chunk_id }))
                .collect::<Result<Vec<_>>>()?;
            let object = crate::eam::ContentObject {
                logical_digest,
                chunk_root: digest(record.field(2)?.as_bytes()?)?,
                chunks: chunks.into_boxed_slice(),
            };
            if encode_content_object_record(&object)? != bytes {
                return Err(noncanonical("ContentObject record is not byte-canonical"));
            }
            Ok(ManifestRecord::ContentObject(Box::new(object)))
        }
        _ => Err(noncanonical(
            "a MANIFEST_RECORD item must carry an Entry or ContentObject record",
        )),
    }
}

pub(super) fn decode_manifest(
    bytes: &[u8],
) -> Result<(EntrySet, BTreeMap<Digest, crate::eam::ContentObject>)> {
    let records = decode_record_stream(bytes)?;
    let mut entries = Vec::new();
    let mut objects = BTreeMap::new();
    let mut content_started = false;
    let mut previous_object = None;
    for record in records {
        match record.kind {
            RECORD_ENTRY if !content_started => entries.push(decode_entry(&record)?),
            RECORD_CONTENT_OBJECT => {
                content_started = true;
                record.expect_tags(&[1, 2, 3], &[])?;
                let logical_digest = digest(record.field(1)?.as_bytes()?)?;
                if previous_object.is_some_and(|previous| previous >= logical_digest) {
                    return Err(noncanonical(
                        "ContentObjects must be uniquely ordered by logical digest",
                    ));
                }
                previous_object = Some(logical_digest);
                let chunks = record
                    .field(3)?
                    .as_sequence()?
                    .into_iter()
                    .map(digest)
                    .map(|result| result.map(|chunk_id| crate::eam::ChunkRef { chunk_id }))
                    .collect::<Result<Vec<_>>>()?;
                let object = crate::eam::ContentObject {
                    logical_digest,
                    chunk_root: digest(record.field(2)?.as_bytes()?)?,
                    chunks: chunks.into_boxed_slice(),
                };
                if objects.insert(logical_digest, object).is_some() {
                    return Err(duplicate("duplicate ContentObject declaration"));
                }
            }
            _ => return Err(noncanonical("MANIFEST_RECORDS has invalid record ordering")),
        }
    }
    let entries = EntrySet::from_canonical(entries)?;
    if encode_manifest(&entries, &objects)? != bytes {
        return Err(noncanonical("manifest records are not canonical"));
    }
    Ok((entries, objects))
}

fn encode_entry(entry: &Entry) -> Result<Vec<u8>> {
    let path = entry
        .path()
        .components()
        .iter()
        .map(encode_path_component)
        .collect::<Result<Vec<_>>>()?;
    let metadata = entry
        .metadata()
        .items()
        .iter()
        .map(|item| encode_metadata_item(*item))
        .collect::<Result<Vec<_>>>()?;
    let mut record = RecordBuilder::new(RECORD_ENTRY);
    record.sequence(1, &path)?;
    match entry.data() {
        EntryData::Directory => {
            record.u8(2, 1)?.u8(3, 0)?;
        }
        EntryData::File {
            content: crate::eam::ContentRef::Internal(digest),
        } => {
            record.u8(2, 2)?.u8(3, 1)?.bytes(4, digest.as_bytes())?;
        }
    }
    record
        .sequence(5, &metadata)?
        .bytes(6, entry.identity().identity_digest.as_bytes())?
        .bytes(7, entry.identity().aux_digest.as_bytes())?;
    record.finish()
}

fn decode_entry(record: &Record<'_>) -> Result<Entry> {
    record.expect_tags(&[1, 2, 3, 5, 6, 7], &[4])?;
    let components = record
        .field(1)?
        .as_sequence()?
        .into_iter()
        .map(decode_path_component)
        .collect::<Result<Vec<_>>>()?;
    let path = LogicalPath::new(components)?;
    let kind = record.field(2)?.as_u8()?;
    let content_kind = record.field(3)?.as_u8()?;
    let data = match (kind, content_kind, record.optional_field(4)) {
        (1, 0, None) => EntryData::Directory,
        (1, _, Some(_)) => {
            return Err(Diagnostic::new(
                OutcomeClass::Nonconforming,
                ReasonCode::DirectoryHasContent,
                path.to_string(),
            ));
        }
        (2, 1, Some(field)) => EntryData::File {
            content: crate::eam::ContentRef::Internal(digest(field.as_bytes()?)?),
        },
        (2, _, None) => {
            return Err(Diagnostic::new(
                OutcomeClass::Nonconforming,
                ReasonCode::FileMissingContent,
                path.to_string(),
            ));
        }
        _ => {
            return Err(Diagnostic::new(
                OutcomeClass::Unsupported,
                ReasonCode::UnsupportedEntryKind,
                path.to_string(),
            ));
        }
    };
    let metadata = record
        .field(5)?
        .as_sequence()?
        .into_iter()
        .map(decode_metadata_item)
        .collect::<Result<Vec<_>>>()?;
    Ok(Entry::new(
        path,
        data,
        MetadataSet::new(metadata)?,
        EntryIdentity {
            identity_digest: digest(record.field(6)?.as_bytes()?)?,
            aux_digest: digest(record.field(7)?.as_bytes()?)?,
        },
    ))
}

fn encode_path_component(component: &PathComponent) -> Result<Vec<u8>> {
    let mut record = RecordBuilder::new(RECORD_PATH_COMPONENT);
    record
        .u8(
            1,
            match component.encoding() {
                PathEncoding::Utf8 => 1,
            },
        )?
        .bytes(2, component.bytes())?;
    record.finish()
}

fn decode_path_component(bytes: &[u8]) -> Result<PathComponent> {
    let (record, consumed) = decode_record(bytes)?;
    if consumed != bytes.len() || record.kind != RECORD_PATH_COMPONENT {
        return Err(noncanonical("path component item is not canonical"));
    }
    record.expect_tags(&[1, 2], &[])?;
    if record.field(1)?.as_u8()? != 1 {
        return Err(Diagnostic::new(
            OutcomeClass::Unsupported,
            ReasonCode::UnsupportedRequiredFeature,
            "bootstrap paths require UTF-8",
        ));
    }
    PathComponent::new(record.field(2)?.as_bytes()?, PathEncoding::Utf8)
}

fn encode_metadata_item(item: MetadataItem) -> Result<Vec<u8>> {
    let mut record = RecordBuilder::new(RECORD_METADATA_ITEM);
    record
        .u8(
            1,
            match item.name() {
                MetadataName::CoreExecutable => 1,
                MetadataName::CoreMtime => 2,
            },
        )?
        .u8(
            2,
            match item.criticality() {
                Criticality::Optional => 0,
                Criticality::Critical => 1,
            },
        )?
        .u8(
            3,
            match item.restorability() {
                Restorability::Restorable => 1,
                Restorability::CaptureOnly => 2,
            },
        )?;
    match item.value() {
        crate::eam::MetadataValue::Bool(value) => {
            record.bool(4, value)?;
        }
        crate::eam::MetadataValue::Timestamp(value) => {
            record.bytes(5, &encode_timestamp(value)?)?;
        }
    }
    record.finish()
}

fn decode_metadata_item(bytes: &[u8]) -> Result<MetadataItem> {
    let (record, consumed) = decode_record(bytes)?;
    if consumed != bytes.len() || record.kind != RECORD_METADATA_ITEM {
        return Err(noncanonical("metadata item is not a canonical record"));
    }
    record.expect_tags(&[1, 2, 3], &[4, 5])?;
    if record.field(2)?.as_u8()? != 0 || record.field(3)?.as_u8()? != 1 {
        return Err(Diagnostic::new(
            OutcomeClass::Unsupported,
            ReasonCode::UnsupportedRequiredFeature,
            "bootstrap metadata is Optional and Restorable",
        ));
    }
    match (
        record.field(1)?.as_u8()?,
        record.optional_field(4),
        record.optional_field(5),
    ) {
        (1, Some(value), None) => Ok(MetadataItem::executable(value.as_bool()?)),
        (2, None, Some(value)) => Ok(MetadataItem::mtime(decode_timestamp(value.as_bytes()?)?)),
        _ => Err(noncanonical("metadata name and value type disagree")),
    }
}

fn encode_timestamp(value: Timestamp) -> Result<Vec<u8>> {
    let mut record = RecordBuilder::new(RECORD_TIMESTAMP);
    record
        .i64(1, value.seconds())?
        .u32(2, value.nanoseconds())?
        .u8(3, precision_id(value.source_precision()))?
        .bool(4, value.restorable())?;
    record.finish()
}

fn decode_timestamp(bytes: &[u8]) -> Result<Timestamp> {
    let (record, consumed) = decode_record(bytes)?;
    if consumed != bytes.len() || record.kind != RECORD_TIMESTAMP {
        return Err(noncanonical("timestamp is not a canonical record"));
    }
    record.expect_tags(&[1, 2, 3, 4], &[])?;
    Timestamp::new(
        record.field(1)?.as_i64()?,
        record.field(2)?.as_u32()?,
        precision(record.field(3)?.as_u8()?)?,
        record.field(4)?.as_bool()?,
    )
}

pub(super) fn encode_fidelity(value: &FidelityReport) -> Result<Vec<u8>> {
    let captured = string_items(&value.captured);
    let unavailable = value
        .unavailable
        .iter()
        .map(encode_fidelity_issue)
        .collect::<Result<Vec<_>>>()?;
    let degraded = value
        .degraded
        .iter()
        .map(encode_fidelity_issue)
        .collect::<Result<Vec<_>>>()?;
    let filesystem = string_items(&value.filesystem);
    let mut record = RecordBuilder::new(RECORD_FIDELITY);
    record
        .sequence(1, &captured)?
        .sequence(2, &unavailable)?
        .sequence(3, &degraded)?
        .utf8(4, &value.platform)?
        .sequence(5, &filesystem)?;
    record.finish()
}

pub(super) fn decode_fidelity(bytes: &[u8]) -> Result<FidelityReport> {
    let (record, consumed) = decode_record(bytes)?;
    if consumed != bytes.len() || record.kind != RECORD_FIDELITY {
        return Err(noncanonical(
            "FIDELITY must contain exactly one Fidelity record",
        ));
    }
    record.expect_tags(&[1, 2, 3, 4, 5], &[])?;
    let unavailable = record
        .field(2)?
        .as_sequence()?
        .into_iter()
        .map(decode_fidelity_issue)
        .collect::<Result<Vec<_>>>()?;
    let degraded = record
        .field(3)?
        .as_sequence()?
        .into_iter()
        .map(decode_fidelity_issue)
        .collect::<Result<Vec<_>>>()?;
    if !issues_are_canonical(&unavailable) || !issues_are_canonical(&degraded) {
        return Err(noncanonical("fidelity issue sets must be uniquely sorted"));
    }
    let value = FidelityReport {
        captured: decode_string_items(record.field(1)?.as_sequence()?)?.into_boxed_slice(),
        unavailable: unavailable.into_boxed_slice(),
        degraded: degraded.into_boxed_slice(),
        platform: record.field(4)?.as_utf8()?.to_owned(),
        filesystem: decode_string_items(record.field(5)?.as_sequence()?)?.into_boxed_slice(),
    };
    if encode_fidelity(&value)? != bytes {
        return Err(noncanonical("Fidelity record is not canonical"));
    }
    Ok(value)
}

pub(super) fn encode_conversion(value: &ConversionProvenance) -> Result<Vec<u8>> {
    validate_conversion_counts(value)?;
    let mut resolutions = Vec::new();
    for resolution in &value.resolutions {
        resolutions.extend_from_slice(&encode_conversion_resolution(resolution)?);
    }
    let ancestors = encode_string_sequence(
        &value
            .synthesized_ancestors
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
    )?;
    let mut record = RecordBuilder::new(RECORD_CONVERSION_PROVENANCE);
    record
        .utf8(1, &value.source_format)?
        .utf8(2, &value.adapter_id)?
        .bytes(3, value.source_digest.as_bytes())?
        .utf8(4, &value.import_mode)?
        .u64(5, value.source_entry_count)?
        .u64(6, value.observation_count)?
        .u64(7, value.omission_count)?
        .u64(8, value.refinement_count)?
        .u64(9, value.divergence_count)?
        .u64(10, value.irreconcilable_count)?
        .bytes(11, &resolutions)?
        .bytes(12, &ancestors)?
        .bytes(13, &encode_string_sequence(&value.unsupported_metadata)?)?
        .utf8(14, &value.outcome)?;
    record.finish()
}

pub(super) fn decode_conversion(bytes: &[u8]) -> Result<ConversionProvenance> {
    let (record, consumed) = decode_record(bytes)?;
    if consumed != bytes.len() || record.kind != RECORD_CONVERSION_PROVENANCE || record.version != 1
    {
        return Err(noncanonical(
            "ConversionProvenance must be exactly one type-28/version-1 record",
        ));
    }
    record.expect_tags(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14], &[])?;
    let resolutions = decode_record_stream(record.field(11)?.as_bytes()?)?
        .into_iter()
        .map(decode_conversion_resolution)
        .collect::<Result<Vec<_>>>()?;
    if resolutions.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(noncanonical(
            "ConversionResolution records must be unique and canonically ordered",
        ));
    }
    let synthesized_ancestors = decode_string_sequence(record.field(12)?.as_bytes()?)?
        .into_iter()
        .map(|path| LogicalPath::from_utf8(path.split('/')))
        .collect::<Result<Vec<_>>>()?;
    if synthesized_ancestors
        .windows(2)
        .any(|pair| pair[0] >= pair[1])
    {
        return Err(noncanonical(
            "synthesized ancestor paths must be unique and canonically ordered",
        ));
    }
    let unsupported_metadata = decode_string_sequence(record.field(13)?.as_bytes()?)?;
    if unsupported_metadata
        .windows(2)
        .any(|pair| pair[0] >= pair[1])
    {
        return Err(noncanonical(
            "unsupported metadata names must be unique and ordered",
        ));
    }
    let value = ConversionProvenance {
        source_format: record.field(1)?.as_utf8()?.to_owned(),
        adapter_id: record.field(2)?.as_utf8()?.to_owned(),
        source_digest: digest(record.field(3)?.as_bytes()?)?,
        import_mode: record.field(4)?.as_utf8()?.to_owned(),
        source_entry_count: record.field(5)?.as_u64()?,
        observation_count: record.field(6)?.as_u64()?,
        omission_count: record.field(7)?.as_u64()?,
        refinement_count: record.field(8)?.as_u64()?,
        divergence_count: record.field(9)?.as_u64()?,
        irreconcilable_count: record.field(10)?.as_u64()?,
        resolutions: resolutions.into_boxed_slice(),
        synthesized_ancestors: synthesized_ancestors.into_boxed_slice(),
        unsupported_metadata: unsupported_metadata.into_boxed_slice(),
        outcome: record.field(14)?.as_utf8()?.to_owned(),
    };
    validate_conversion_counts(&value)?;
    if encode_conversion(&value)? != bytes {
        return Err(noncanonical("ConversionProvenance is not canonical"));
    }
    Ok(value)
}

fn validate_conversion_counts(value: &ConversionProvenance) -> Result<()> {
    let classes = ["omission", "refinement", "divergence", "irreconcilable"];
    if value
        .resolutions
        .iter()
        .any(|resolution| !classes.contains(&resolution.conflict_class.as_str()))
    {
        return Err(noncanonical(
            "ConversionProvenance contains an unknown conflict class",
        ));
    }
    let resolved_counts = classes.map(|class| {
        value
            .resolutions
            .iter()
            .filter(|resolution| resolution.conflict_class == class)
            .count()
            .try_into()
            .unwrap_or(u64::MAX)
    });
    let declared_counts = [
        value.omission_count,
        value.refinement_count,
        value.divergence_count,
        value.irreconcilable_count,
    ];
    if declared_counts
        .iter()
        .zip(resolved_counts)
        .any(|(declared, resolved)| *declared < resolved)
    {
        return Err(noncanonical(
            "ConversionProvenance resolves more conflicts than it declares",
        ));
    }
    Ok(())
}

fn encode_conversion_resolution(value: &ConversionResolution) -> Result<Vec<u8>> {
    let mut record = RecordBuilder::new(RECORD_CONVERSION_RESOLUTION);
    record
        .utf8(1, &value.conflict_class)?
        .utf8(2, &value.semantic_field)?
        .bytes(3, &encode_string_sequence(&value.authorities)?)?
        .bytes(4, &encode_string_sequence(&value.observed_values)?)?
        .utf8(5, &value.action)?;
    record.finish()
}

fn decode_conversion_resolution(record: Record<'_>) -> Result<ConversionResolution> {
    if record.kind != RECORD_CONVERSION_RESOLUTION || record.version != 1 {
        return Err(noncanonical(
            "conversion resolution collection contains a wrong record",
        ));
    }
    record.expect_tags(&[1, 2, 3, 4, 5], &[])?;
    Ok(ConversionResolution {
        conflict_class: record.field(1)?.as_utf8()?.to_owned(),
        semantic_field: record.field(2)?.as_utf8()?.to_owned(),
        authorities: decode_string_sequence(record.field(3)?.as_bytes()?)?.into_boxed_slice(),
        observed_values: decode_string_sequence(record.field(4)?.as_bytes()?)?.into_boxed_slice(),
        action: record.field(5)?.as_utf8()?.to_owned(),
    })
}

fn encode_string_sequence(values: &[String]) -> Result<Vec<u8>> {
    let mut encoded = Vec::new();
    encoded.extend_from_slice(
        &u64::try_from(values.len())
            .map_err(|_| resource_limit("string sequence count exceeds u64"))?
            .to_be_bytes(),
    );
    for value in values {
        encoded.extend_from_slice(
            &u64::try_from(value.len())
                .map_err(|_| resource_limit("string sequence item exceeds u64"))?
                .to_be_bytes(),
        );
        encoded.extend_from_slice(value.as_bytes());
    }
    Ok(encoded)
}

fn decode_string_sequence(mut bytes: &[u8]) -> Result<Vec<String>> {
    if bytes.len() < 8 {
        return Err(noncanonical("string sequence count is truncated"));
    }
    let count = usize::try_from(u64::from_be_bytes(bytes[..8].try_into().unwrap()))
        .map_err(|_| resource_limit("string sequence count exceeds usize"))?;
    bytes = &bytes[8..];
    let mut values = Vec::with_capacity(count.min(1_000_000));
    for _ in 0..count {
        if bytes.len() < 8 {
            return Err(noncanonical("string sequence item length is truncated"));
        }
        let len = usize::try_from(u64::from_be_bytes(bytes[..8].try_into().unwrap()))
            .map_err(|_| resource_limit("string sequence item exceeds usize"))?;
        bytes = &bytes[8..];
        let value = bytes
            .get(..len)
            .ok_or_else(|| noncanonical("string sequence item is truncated"))?;
        values.push(
            std::str::from_utf8(value)
                .map_err(|_| noncanonical("string sequence item is not UTF-8"))?
                .to_owned(),
        );
        bytes = &bytes[len..];
    }
    if !bytes.is_empty() {
        return Err(noncanonical("string sequence has trailing bytes"));
    }
    Ok(values)
}

fn encode_fidelity_issue(value: &FidelityIssue) -> Result<Vec<u8>> {
    let mut record = RecordBuilder::new(RECORD_FIDELITY_ISSUE);
    record.utf8(1, &value.class)?.utf8(2, &value.reason)?;
    if let Some(path) = &value.entry_scope {
        let components = path
            .components()
            .iter()
            .map(encode_path_component)
            .collect::<Result<Vec<_>>>()?;
        record.sequence(3, &components)?;
    }
    record.finish()
}

fn decode_fidelity_issue(bytes: &[u8]) -> Result<FidelityIssue> {
    let (record, consumed) = decode_record(bytes)?;
    if consumed != bytes.len() || record.kind != RECORD_FIDELITY_ISSUE {
        return Err(noncanonical("fidelity issue is not a canonical record"));
    }
    record.expect_tags(&[1, 2], &[3])?;
    let entry_scope = record
        .optional_field(3)
        .map(|field| {
            let components = field
                .as_sequence()?
                .into_iter()
                .map(decode_path_component)
                .collect::<Result<Vec<_>>>()?;
            LogicalPath::new(components)
        })
        .transpose()?;
    Ok(FidelityIssue {
        class: record.field(1)?.as_utf8()?.to_owned(),
        reason: record.field(2)?.as_utf8()?.to_owned(),
        entry_scope,
    })
}

pub(super) fn encode_index(index: &BTreeMap<Digest, crate::eam::ChunkLocation>) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    for (digest, location) in index {
        let mut record = RecordBuilder::new(RECORD_INDEX_ENTRY);
        record
            .bytes(1, digest.as_bytes())?
            .u64(2, location.offset)?
            .u64(3, location.stored_len)?;
        bytes.extend_from_slice(&record.finish()?);
    }
    Ok(bytes)
}

pub(super) fn decode_index(bytes: &[u8]) -> Result<BTreeMap<Digest, crate::eam::ChunkLocation>> {
    let mut index = BTreeMap::new();
    let mut previous = None;
    for record in decode_record_stream(bytes)? {
        if record.kind != RECORD_INDEX_ENTRY {
            return Err(noncanonical("INDEX contains a non-index record"));
        }
        record.expect_tags(&[1, 2, 3], &[])?;
        let chunk_id = digest(record.field(1)?.as_bytes()?)?;
        if previous.is_some_and(|value| value >= chunk_id) {
            return Err(noncanonical(
                "Index entries must be uniquely ordered by chunk ID",
            ));
        }
        previous = Some(chunk_id);
        index.insert(
            chunk_id,
            crate::eam::ChunkLocation {
                offset: record.field(2)?.as_u64()?,
                stored_len: record.field(3)?.as_u64()?,
            },
        );
    }
    Ok(index)
}

fn string_items(values: &[String]) -> Vec<Vec<u8>> {
    values
        .iter()
        .map(|value| value.as_bytes().to_vec())
        .collect()
}

fn decode_string_items(values: Vec<&[u8]>) -> Result<Vec<String>> {
    let values = values
        .into_iter()
        .map(|value| {
            std::str::from_utf8(value)
                .map(str::to_owned)
                .map_err(|_| noncanonical("string sequence item is not UTF-8"))
        })
        .collect::<Result<Vec<_>>>()?;
    if values.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(noncanonical("string sets must be uniquely sorted"));
    }
    Ok(values)
}

fn issues_are_canonical(values: &[FidelityIssue]) -> bool {
    values.windows(2).all(|pair| {
        (&pair[0].class, &pair[0].reason, &pair[0].entry_scope)
            < (&pair[1].class, &pair[1].reason, &pair[1].entry_scope)
    })
}

fn digest(bytes: &[u8]) -> Result<Digest> {
    let bytes: [u8; 32] = bytes
        .try_into()
        .map_err(|_| noncanonical("digest fields must contain exactly 32 bytes"))?;
    Ok(Digest::from_bytes(bytes))
}

fn precision_id(value: TimestampPrecision) -> u8 {
    match value {
        TimestampPrecision::Second => 1,
        TimestampPrecision::Centisecond => 2,
        TimestampPrecision::Microsecond => 3,
        TimestampPrecision::Hectonanosecond => 4,
        TimestampPrecision::Nanosecond => 5,
    }
}

fn precision(value: u8) -> Result<TimestampPrecision> {
    match value {
        1 => Ok(TimestampPrecision::Second),
        2 => Ok(TimestampPrecision::Centisecond),
        3 => Ok(TimestampPrecision::Microsecond),
        4 => Ok(TimestampPrecision::Hectonanosecond),
        5 => Ok(TimestampPrecision::Nanosecond),
        _ => Err(noncanonical("unknown timestamp precision")),
    }
}

fn noncanonical(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::Nonconforming,
        ReasonCode::NoncanonicalEncoding,
        detail,
    )
}

fn resource_limit(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::PolicyRefused,
        ReasonCode::ResourceLimit,
        detail,
    )
}

fn duplicate(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::Nonconforming,
        ReasonCode::DuplicateSemanticDeclaration,
        detail,
    )
}

#[cfg(test)]
mod tests {
    use std::io::Write as _;

    use flate2::{Compression, write::DeflateEncoder};

    use super::*;
    use crate::codec::zstd_transformed_plan;
    use crate::identity::sha256_exact;
    use crate::transform::{BYTE_SHUFFLE_ID, byte_shuffle_step, delta8_step};

    fn descriptor_fixture(declarations: Option<DescriptorDeclarations>) -> DescriptorBody {
        DescriptorBody {
            namespace: "ecf/bootstrap-v1".to_owned(),
            identity_profile: 1,
            digest_algorithm: 1,
            planner_id: "balanced-v6".to_owned(),
            chunker_id: "gear-norm-v1".to_owned(),
            lai: Digest::from_bytes([0x11; 32]),
            pcr: Digest::from_bytes([0x22; 32]),
            aux: Digest::from_bytes([0x33; 32]),
            declarations,
        }
    }

    fn descriptor_field_offset(bytes: &[u8], wanted: u16) -> usize {
        let mut cursor = crate::canonical::RECORD_HEADER_LEN;
        while cursor < bytes.len() {
            let tag = u16::from_be_bytes(bytes[cursor..cursor + 2].try_into().unwrap());
            if tag == wanted {
                return cursor;
            }
            let len = u64::from_be_bytes(bytes[cursor + 4..cursor + 12].try_into().unwrap());
            cursor += 12 + usize::try_from(len).unwrap();
        }
        panic!("Descriptor field {wanted} is missing");
    }

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    fn descriptor_vector(name: &str) -> &'static str {
        include_str!("../../../../docs/descriptor-vectors-v1.txt")
            .lines()
            .find_map(|line| line.strip_prefix(&format!("{name}=")))
            .expect("Descriptor vector must exist")
    }

    #[test]
    fn descriptor_v1_is_frozen_and_v2_is_closed_and_canonical() {
        let v1 = encode_descriptor(&descriptor_fixture(None)).unwrap();
        assert_eq!(&v1[2..4], &1_u16.to_be_bytes());
        assert_eq!(decode_descriptor(&v1).unwrap(), descriptor_fixture(None));

        let representative = DescriptorDeclarations {
            decode: DecodeRequirements {
                window_bytes: 1_048_576,
                working_set_bytes: 2_097_152,
                flags: 5,
            },
            budget: ResourceBudget {
                entry_count: 3,
                total_logical_bytes: 12_345,
                max_single_entry_logical_bytes: 8_192,
                max_expansion_ratio_milli: 1_500,
                chunk_count: 7,
                max_path_depth: 4,
                max_metadata_bytes: 2_048,
                max_key_derivation_cost: 0,
            },
        };
        let v2 = encode_descriptor(&descriptor_fixture(Some(representative))).unwrap();
        assert_eq!(&v2[2..4], &2_u16.to_be_bytes());
        assert_eq!(
            decode_descriptor(&v2).unwrap(),
            descriptor_fixture(Some(representative))
        );

        let maximum = DescriptorDeclarations {
            decode: DecodeRequirements {
                window_bytes: u64::MAX,
                working_set_bytes: u64::MAX,
                flags: u32::MAX,
            },
            budget: ResourceBudget {
                entry_count: u64::MAX,
                total_logical_bytes: u64::MAX,
                max_single_entry_logical_bytes: u64::MAX,
                max_expansion_ratio_milli: u64::MAX,
                chunk_count: u64::MAX,
                max_path_depth: u64::MAX,
                max_metadata_bytes: u64::MAX,
                max_key_derivation_cost: u64::MAX,
            },
        };
        let max_bytes = encode_descriptor(&descriptor_fixture(Some(maximum))).unwrap();
        assert_eq!(
            decode_descriptor(&max_bytes).unwrap(),
            descriptor_fixture(Some(maximum))
        );
        assert_eq!(hex(&v1), descriptor_vector("DESCRIPTOR_V1"));
        assert_eq!(
            sha256_exact(&v1).to_string(),
            descriptor_vector("DESCRIPTOR_V1_SHA256")
        );
        assert_eq!(hex(&v2), descriptor_vector("DESCRIPTOR_V2"));
        assert_eq!(
            sha256_exact(&v2).to_string(),
            descriptor_vector("DESCRIPTOR_V2_SHA256")
        );
        assert_eq!(hex(&max_bytes), descriptor_vector("DESCRIPTOR_V2_MAX"));
        assert_eq!(
            sha256_exact(&max_bytes).to_string(),
            descriptor_vector("DESCRIPTOR_V2_MAX_SHA256")
        );
    }

    #[test]
    fn malformed_descriptor_v2_cases_are_typed() {
        let declarations = DescriptorDeclarations {
            decode: DecodeRequirements::default(),
            budget: ResourceBudget::default(),
        };
        let valid = encode_descriptor(&descriptor_fixture(Some(declarations))).unwrap();

        let mut missing = valid.clone();
        missing.truncate(missing.len() - 20);
        let payload_len =
            u64::try_from(missing.len() - crate::canonical::RECORD_HEADER_LEN).unwrap();
        missing[8..16].copy_from_slice(&payload_len.to_be_bytes());
        assert_eq!(
            decode_descriptor(&missing).unwrap_err().code(),
            ReasonCode::NoncanonicalEncoding
        );

        let tag_19 = descriptor_field_offset(&valid, 19);
        let mut duplicate = valid.clone();
        duplicate[tag_19..tag_19 + 2].copy_from_slice(&18_u16.to_be_bytes());
        assert_eq!(
            decode_descriptor(&duplicate).unwrap_err().code(),
            ReasonCode::DuplicateSemanticDeclaration
        );

        let mut out_of_order = valid.clone();
        out_of_order[tag_19..tag_19 + 2].copy_from_slice(&17_u16.to_be_bytes());
        assert_eq!(
            decode_descriptor(&out_of_order).unwrap_err().code(),
            ReasonCode::NoncanonicalEncoding
        );

        let mut wrong_type = valid.clone();
        let tag_9 = descriptor_field_offset(&valid, 9);
        wrong_type[tag_9 + 2] = crate::canonical::FieldType::Bytes as u8;
        assert_eq!(
            decode_descriptor(&wrong_type).unwrap_err().code(),
            ReasonCode::NoncanonicalEncoding
        );

        let mut version_three = valid;
        version_three[2..4].copy_from_slice(&3_u16.to_be_bytes());
        assert_eq!(
            decode_descriptor(&version_three).unwrap_err().code(),
            ReasonCode::NoncanonicalEncoding
        );
    }

    #[test]
    fn generated_whole_object_records_are_canonical_and_duplicate_safe() {
        let region_id = sha256_exact(b"ECF-REGION-VALID-001");
        let content_object = sha256_exact(b"region content object");
        let region = ReconstructionRegion {
            region_id,
            content_object,
            start_chunk_index: 0,
            chunk_count: 2,
            plan_ref: 77,
            logical_bytes: 900,
            transformed_bytes: 700,
            ordinary_physical_bytes: 880,
            region_overhead_bytes: 80,
            access: RegionAccessCost {
                logical_bytes: 900,
                logical_chunks: 2,
                worst_reconstructed_bytes: 900,
            },
            representation: vec![9; 600].into_boxed_slice(),
        };
        let target = ReconstructionAuditTarget::ContentObject(content_object);
        let audit = ReconstructionAudit {
            target,
            transform_id: "jpeg-jxl-reconstruct/v1".to_owned(),
            reason: ReconstructionAuditReason::CompleteCostDidNotWin,
        };
        let bytes = encode_reconstruction_regions(
            &BTreeMap::from([(region_id, region.clone())]),
            &BTreeMap::from([(target, audit.clone())]),
        )
        .unwrap();
        let (decoded_regions, decoded_audits) = decode_reconstruction_regions(&bytes).unwrap();
        assert_eq!(decoded_regions[&region_id], region);
        assert_eq!(decoded_audits[&target], audit);

        let region_only =
            encode_reconstruction_regions(&BTreeMap::from([(region_id, region)]), &BTreeMap::new())
                .unwrap();
        let mut duplicate = region_only.clone();
        duplicate.extend_from_slice(&region_only);
        assert_eq!(
            decode_reconstruction_regions(&duplicate)
                .unwrap_err()
                .code(),
            ReasonCode::DuplicateSemanticDeclaration,
            "ECF-REGION-DUPLICATE-001: region identity is unique"
        );

        let audit_only =
            encode_reconstruction_regions(&BTreeMap::new(), &BTreeMap::from([(target, audit)]))
                .unwrap();
        let mut duplicate_audit = audit_only.clone();
        duplicate_audit.extend_from_slice(&audit_only);
        assert_eq!(
            decode_reconstruction_regions(&duplicate_audit)
                .unwrap_err()
                .code(),
            ReasonCode::DuplicateSemanticDeclaration,
            "ECF-REGION-AUDIT-DUPLICATE-001: audit target is unique"
        );
    }

    #[test]
    fn duplicate_dictionary_records_are_typed_nonconformance() {
        let bytes: Box<[u8]> = b"generated dictionary test bytes".as_slice().into();
        let dictionary_id = sha256_exact(&bytes);
        let dictionary = Dictionary {
            dictionary_id,
            codec: "zstandard/v1".to_owned(),
            format: "zstd-trained/v1".to_owned(),
            construction: "zstd-1.5.7-train-buffer-v1/balanced-v3-digest-order-samples16-sample-cap16384-dict-cap8192".to_owned(),
            bytes,
        };
        let encoded = encode_dictionaries(&BTreeMap::from([(dictionary_id, dictionary)])).unwrap();
        let mut duplicate = encoded.clone();
        duplicate.extend_from_slice(&encoded);
        assert_eq!(
            decode_dictionaries(&duplicate).unwrap_err().code(),
            ReasonCode::DuplicateSemanticDeclaration
        );
    }

    #[test]
    fn generated_reconstruction_data_conformance_cases_are_typed() {
        let mut encoder = DeflateEncoder::new(Vec::new(), Compression::new(6));
        let source = (0..5_000)
            .flat_map(|index| format!("record={index:06};value={}\n", index * 19).into_bytes())
            .collect::<Vec<_>>();
        encoder.write_all(&source).unwrap();
        let original = encoder.finish().unwrap();
        let candidate = crate::reconstruction::try_forward(&original, 512)
            .unwrap()
            .unwrap();
        let encoded = encode_reconstruction_data(&BTreeMap::from([(
            candidate.data.reconstruction_id,
            candidate.data.clone(),
        )]))
        .unwrap();
        let mut duplicate = encoded.clone();
        duplicate.extend_from_slice(&encoded);
        assert_eq!(
            decode_reconstruction_data(&duplicate).unwrap_err().code(),
            ReasonCode::DuplicateSemanticDeclaration,
            "ECF-RECONSTRUCTION-DUPLICATE-001"
        );

        let mut corrupt = candidate.data;
        corrupt.bytes[0] ^= 1;
        let bytes =
            encode_reconstruction_data(&BTreeMap::from([(corrupt.reconstruction_id, corrupt)]))
                .unwrap();
        assert_eq!(
            decode_reconstruction_data(&bytes).unwrap_err().code(),
            ReasonCode::ReconstructionDataDigestMismatch,
            "ECF-RECONSTRUCTION-DIGEST-001"
        );

        let fallback = encode_reconstruction_section(
            &BTreeMap::new(),
            &BTreeMap::from([(
                sha256_exact(b"fallback-chunk"),
                ReconstructionFallbackReason::CompleteCostDidNotWin,
            )]),
        )
        .unwrap();
        let mut duplicate_fallback = fallback.clone();
        duplicate_fallback.extend_from_slice(&fallback);
        assert_eq!(
            decode_reconstruction_section(&duplicate_fallback)
                .unwrap_err()
                .code(),
            ReasonCode::DuplicateSemanticDeclaration,
            "ECF-RECONSTRUCTION-FALLBACK-DUPLICATE-001"
        );
    }

    #[test]
    fn transform_steps_are_canonical_decoder_facing_records() {
        let plan =
            zstd_transformed_plan(3, vec![delta8_step(), byte_shuffle_step(4).unwrap()].into())
                .unwrap();
        let encoded = encode_transform_plans(std::slice::from_ref(&plan), true).unwrap();
        assert_eq!(
            decode_transform_plans(&encoded, true).unwrap().as_ref(),
            &[plan]
        );
        assert_eq!(
            decode_transform_plans(&encoded, false).unwrap_err().code(),
            ReasonCode::UnknownTransform
        );
    }

    #[test]
    fn generated_transform_and_codec_conformance_cases_are_typed() {
        let base = crate::codec::zstd_plan(3).unwrap();
        let cases = [
            (
                "ECF-TRANSFORM-UNKNOWN-001",
                TransformStep {
                    transform_id: "unknown-transform/v1".to_owned(),
                    parameters: Box::default(),
                    reconstruction_ref: None,
                },
                ReasonCode::UnknownTransform,
            ),
            (
                "ECF-TRANSFORM-PARAMETERS-001",
                TransformStep {
                    transform_id: BYTE_SHUFFLE_ID.to_owned(),
                    parameters: Box::from([3]),
                    reconstruction_ref: None,
                },
                ReasonCode::InvalidTransformParameters,
            ),
        ];
        for (test_id, step, expected) in cases {
            let mut plan = base.clone();
            plan.transforms = vec![step].into_boxed_slice();
            let bytes = encode_transform_plans(&[plan], true).unwrap();
            assert_eq!(
                decode_transform_plans(&bytes, true).unwrap_err().code(),
                expected,
                "{test_id}: TransformStep registry invariant"
            );
        }

        let mut duplicate = base.clone();
        duplicate.transforms = vec![delta8_step(), delta8_step()].into_boxed_slice();
        let bytes = encode_transform_plans(&[duplicate], true).unwrap();
        assert_eq!(
            decode_transform_plans(&bytes, true).unwrap_err().code(),
            ReasonCode::DuplicateSemanticDeclaration,
            "ECF-TRANSFORM-DUPLICATE-001: one ordered step per transform family"
        );

        let mut unknown_codec = base;
        unknown_codec.codec = "unknown-codec/v1".to_owned();
        let bytes = encode_transform_plans(&[unknown_codec], true).unwrap();
        assert_eq!(
            decode_transform_plans(&bytes, true).unwrap_err().code(),
            ReasonCode::UnknownCodec,
            "ECF-CODEC-UNKNOWN-001: archive strings cannot select unregistered code"
        );
    }
}
