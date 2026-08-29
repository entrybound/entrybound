use std::collections::BTreeMap;

use crate::diagnostics::{Diagnostic, OutcomeClass, ReasonCode, Result};
use crate::eam::{
    Archive, ArchiveRole, ContentRef, DecodeRequirements, EntryData, EntryKind, FeatureSet,
    FidelityReport, Layout, ResourceBudget,
};
use crate::ecf::{FORMAT_NAMESPACE, FormatVersion, IndexStatus, OpenedArchive};
use crate::identity::IdentitySet;

/// One canonical entry prepared for a user-facing listing.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ListedEntry {
    pub path: String,
    pub kind: EntryKind,
    pub logical_bytes: u64,
}

/// One TransformPlan summary without exposing private ECF records.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlanInspection {
    pub plan_id: u64,
    pub identifier: String,
    pub codec: String,
    pub decode: DecodeRequirements,
}

/// Actual unique Chunk usage for one recorded codec.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodecUsage {
    pub codec: String,
    pub chunk_count: u64,
    pub logical_bytes: u64,
    pub stored_bytes: u64,
}

/// First observable compression summary, derived without an audit-trail record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompressionExplanation {
    pub planner_id: String,
    pub total_logical_bytes: u64,
    pub total_plaintext_chunk_bytes: u64,
    pub total_stored_chunk_bytes: u64,
    pub codec_usage: Vec<CodecUsage>,
    pub store_chunk_count: u64,
    pub store_logical_bytes: u64,
    pub store_stored_bytes: u64,
    pub zstandard_chunk_count: u64,
    pub zstandard_logical_bytes: u64,
    pub zstandard_stored_bytes: u64,
    pub physical_savings_bytes: i128,
}

/// Stable library-level information used by the thin `inspect` command.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArchiveInspection {
    pub format_namespace: String,
    pub version: FormatVersion,
    pub layout: Layout,
    pub role: ArchiveRole,
    pub entry_count: u64,
    pub total_logical_bytes: u64,
    pub features: FeatureSet,
    pub planner_id: String,
    pub chunker_id: String,
    pub plans: Vec<PlanInspection>,
    pub codec_usage: Vec<CodecUsage>,
    pub identities: IdentitySet,
    pub index_status: IndexStatus,
    pub fidelity: FidelityReport,
    pub declared_resources: ResourceBudget,
    pub decode_requirements: DecodeRequirements,
}

/// Returns entries in the EAM's canonical order.
pub fn list(archive: &Archive) -> Result<Vec<ListedEntry>> {
    archive
        .entry_set
        .entries()
        .iter()
        .map(|entry| {
            let logical_bytes = match entry.data() {
                EntryData::Directory => 0,
                EntryData::File {
                    content: ContentRef::Internal(digest),
                } => object_size(archive, digest)?,
            };
            Ok(ListedEntry {
                path: entry.path().to_string(),
                kind: entry.kind(),
                logical_bytes,
            })
        })
        .collect()
}

/// Derives an inspection view from an already opened and verified archive.
pub fn inspect(opened: &OpenedArchive) -> Result<ArchiveInspection> {
    let archive = &opened.archive;
    Ok(ArchiveInspection {
        format_namespace: FORMAT_NAMESPACE.to_owned(),
        version: FormatVersion {
            major: archive.descriptor.format_major,
            minor: archive.descriptor.format_minor,
        },
        layout: archive.descriptor.layout,
        role: archive.descriptor.role,
        entry_count: u64::try_from(archive.entry_set.len())
            .map_err(|_| resource("entry count exceeds u64"))?,
        total_logical_bytes: archive.total_logical_size()?,
        features: archive.descriptor.features,
        planner_id: archive.descriptor.planner_id.clone(),
        chunker_id: archive.descriptor.chunker_id.clone(),
        plans: archive
            .transform_plans
            .iter()
            .map(|plan| PlanInspection {
                plan_id: plan.plan_id,
                identifier: plan.identifier.clone(),
                codec: plan.codec.clone(),
                decode: plan.decode,
            })
            .collect(),
        codec_usage: codec_usage(opened)?,
        identities: opened.report.identities,
        index_status: opened.report.index_status,
        fidelity: archive.fidelity.clone(),
        declared_resources: archive.descriptor.budget,
        decode_requirements: archive.descriptor.decode,
    })
}

/// Derives physical compression totals from verified Chunk frames and plans.
pub fn explain(opened: &OpenedArchive) -> Result<CompressionExplanation> {
    let usage = codec_usage(opened)?;
    let total_plaintext_chunk_bytes = usage.iter().try_fold(0_u64, |total, item| {
        total
            .checked_add(item.logical_bytes)
            .ok_or_else(|| resource("plaintext Chunk byte total exceeds u64"))
    })?;
    let total_stored_chunk_bytes = usage.iter().try_fold(0_u64, |total, item| {
        total
            .checked_add(item.stored_bytes)
            .ok_or_else(|| resource("stored Chunk byte total exceeds u64"))
    })?;
    let store = usage.iter().find(|item| item.codec == "store/v1");
    let zstandard = usage.iter().find(|item| item.codec == "zstandard/v1");
    Ok(CompressionExplanation {
        planner_id: opened.archive.descriptor.planner_id.clone(),
        total_logical_bytes: opened.archive.total_logical_size()?,
        total_plaintext_chunk_bytes,
        total_stored_chunk_bytes,
        store_chunk_count: store.map_or(0, |item| item.chunk_count),
        store_logical_bytes: store.map_or(0, |item| item.logical_bytes),
        store_stored_bytes: store.map_or(0, |item| item.stored_bytes),
        zstandard_chunk_count: zstandard.map_or(0, |item| item.chunk_count),
        zstandard_logical_bytes: zstandard.map_or(0, |item| item.logical_bytes),
        zstandard_stored_bytes: zstandard.map_or(0, |item| item.stored_bytes),
        codec_usage: usage,
        physical_savings_bytes: i128::from(total_plaintext_chunk_bytes)
            - i128::from(total_stored_chunk_bytes),
    })
}

fn codec_usage(opened: &OpenedArchive) -> Result<Vec<CodecUsage>> {
    let archive = &opened.archive;
    let plans = archive
        .transform_plans
        .iter()
        .map(|plan| (plan.plan_id, plan.codec.as_str()))
        .collect::<BTreeMap<_, _>>();
    let mut usage = BTreeMap::<String, CodecUsage>::new();
    for (chunk_id, chunk) in &archive.content_store.chunks {
        let codec = plans.get(&chunk.plan_ref).ok_or_else(|| {
            Diagnostic::new(
                OutcomeClass::Unsupported,
                ReasonCode::UnknownTransformPlan,
                format!("Chunk {chunk_id} references plan {}", chunk.plan_ref),
            )
        })?;
        let location = archive.index.chunks.get(chunk_id).ok_or_else(|| {
            Diagnostic::new(
                OutcomeClass::Corrupt,
                ReasonCode::IndexInvalidRebuilt,
                format!("rebuilt Index is missing Chunk {chunk_id}"),
            )
        })?;
        let item = usage.entry((*codec).to_owned()).or_insert(CodecUsage {
            codec: (*codec).to_owned(),
            chunk_count: 0,
            logical_bytes: 0,
            stored_bytes: 0,
        });
        item.chunk_count = item
            .chunk_count
            .checked_add(1)
            .ok_or_else(|| resource("codec Chunk count exceeds u64"))?;
        item.logical_bytes = item
            .logical_bytes
            .checked_add(chunk.logical_len)
            .ok_or_else(|| resource("codec logical byte total exceeds u64"))?;
        item.stored_bytes = item
            .stored_bytes
            .checked_add(location.stored_len)
            .ok_or_else(|| resource("codec stored byte total exceeds u64"))?;
    }
    Ok(usage.into_values().collect())
}

fn object_size(archive: &Archive, digest: crate::eam::Digest) -> Result<u64> {
    let object = archive.content_store.objects.get(&digest).ok_or_else(|| {
        Diagnostic::new(
            OutcomeClass::Nonconforming,
            ReasonCode::UnknownContentObject,
            digest.to_string(),
        )
    })?;
    object.chunks.iter().try_fold(0_u64, |total, chunk_ref| {
        let chunk = archive
            .content_store
            .chunks
            .get(&chunk_ref.chunk_id)
            .ok_or_else(|| {
                Diagnostic::new(
                    OutcomeClass::Nonconforming,
                    ReasonCode::UnknownChunk,
                    chunk_ref.chunk_id.to_string(),
                )
            })?;
        total
            .checked_add(chunk.logical_len)
            .ok_or_else(|| resource("file logical size exceeds u64"))
    })
}

fn resource(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::PolicyRefused,
        ReasonCode::ResourceLimit,
        detail,
    )
}
