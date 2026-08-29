use std::collections::BTreeMap;

use crate::canonical::{Record, RecordBuilder, decode_record, decode_record_stream};
use crate::diagnostics::{Diagnostic, OutcomeClass, ReasonCode, Result};
use crate::eam::{
    Criticality, Digest, Entry, EntryData, EntryIdentity, EntrySet, FidelityIssue, FidelityReport,
    LogicalPath, MetadataItem, MetadataName, MetadataSet, PathComponent, PathEncoding,
    Restorability, Timestamp, TimestampPrecision, TransformPlan,
};
use crate::identity::{STORE_CODEC_IDENTIFIER, STORE_PLAN_ID, STORE_PLAN_IDENTIFIER};

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
}

pub(super) fn encode_descriptor(value: &DescriptorBody) -> Result<Vec<u8>> {
    let mut record = RecordBuilder::new(RECORD_DESCRIPTOR);
    record
        .utf8(1, &value.namespace)?
        .u8(2, value.identity_profile)?
        .u8(3, value.digest_algorithm)?
        .utf8(4, &value.planner_id)?
        .utf8(5, &value.chunker_id)?
        .bytes(6, value.lai.as_bytes())?
        .bytes(7, value.pcr.as_bytes())?
        .bytes(8, value.aux.as_bytes())?;
    record.finish()
}

pub(super) fn decode_descriptor(bytes: &[u8]) -> Result<DescriptorBody> {
    let (record, consumed) = decode_record(bytes)?;
    if consumed != bytes.len() || record.kind != RECORD_DESCRIPTOR {
        return Err(noncanonical(
            "DESCRIPTOR must contain exactly one Descriptor record",
        ));
    }
    record.expect_tags(&[1, 2, 3, 4, 5, 6, 7, 8], &[])?;
    let value = DescriptorBody {
        namespace: record.field(1)?.as_utf8()?.to_owned(),
        identity_profile: record.field(2)?.as_u8()?,
        digest_algorithm: record.field(3)?.as_u8()?,
        planner_id: record.field(4)?.as_utf8()?.to_owned(),
        chunker_id: record.field(5)?.as_utf8()?.to_owned(),
        lai: digest(record.field(6)?.as_bytes()?)?,
        pcr: digest(record.field(7)?.as_bytes()?)?,
        aux: digest(record.field(8)?.as_bytes()?)?,
    };
    if encode_descriptor(&value)? != bytes {
        return Err(noncanonical("Descriptor record is not in canonical form"));
    }
    Ok(value)
}

pub(super) fn encode_transform_plans(plans: &[TransformPlan]) -> Result<Vec<u8>> {
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
            .map(|value| value.as_bytes().to_vec())
            .collect::<Vec<_>>();
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

pub(super) fn decode_transform_plans(bytes: &[u8]) -> Result<Box<[TransformPlan]>> {
    let records = decode_record_stream(bytes)?;
    let mut plans = Vec::with_capacity(records.len());
    for record in records {
        if record.kind != RECORD_TRANSFORM_PLAN {
            return Err(noncanonical("TRANSFORM_PLANS contains a non-plan record"));
        }
        record.expect_tags(&[1, 2, 3, 4, 5, 7, 8, 9], &[6])?;
        let transforms = record
            .field(3)?
            .as_sequence()?
            .into_iter()
            .map(|value| {
                std::str::from_utf8(value)
                    .map(str::to_owned)
                    .map_err(|_| noncanonical("transform identifier is not UTF-8"))
            })
            .collect::<Result<Vec<_>>>()?;
        plans.push(TransformPlan {
            plan_id: record.field(1)?.as_u64()?,
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
    if plans.len() != 1
        || plans[0].plan_id != STORE_PLAN_ID
        || plans[0].identifier != STORE_PLAN_IDENTIFIER
        || !plans[0].transforms.is_empty()
        || plans[0].codec != STORE_CODEC_IDENTIFIER
        || !plans[0].codec_params.is_empty()
        || plans[0].dictionary.is_some()
        || plans[0].decode != crate::eam::DecodeRequirements::default()
    {
        return Err(Diagnostic::new(
            OutcomeClass::Unsupported,
            ReasonCode::UnknownTransformPlan,
            "bootstrap archives require exactly bootstrap-store-v1",
        ));
    }
    let plans = plans.into_boxed_slice();
    if encode_transform_plans(&plans)? != bytes {
        return Err(noncanonical("TransformPlan records are not canonical"));
    }
    Ok(plans)
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
        encoded.extend_from_slice(&record.finish()?);
    }
    Ok(encoded)
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

fn duplicate(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::Nonconforming,
        ReasonCode::DuplicateSemanticDeclaration,
        detail,
    )
}
