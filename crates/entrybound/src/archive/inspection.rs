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
    pub identifier: String,
    pub codec: String,
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
                identifier: plan.identifier.clone(),
                codec: plan.codec.clone(),
            })
            .collect(),
        identities: opened.report.identities,
        index_status: opened.report.index_status,
        fidelity: archive.fidelity.clone(),
        declared_resources: archive.descriptor.budget,
        decode_requirements: archive.descriptor.decode,
    })
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
