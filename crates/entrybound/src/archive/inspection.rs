use std::collections::BTreeMap;

use crate::diagnostics::{Diagnostic, OutcomeClass, ReasonCode, Result};
use crate::eam::{
    Archive, ArchiveRole, ContentRef, ConversionProvenance, DecodeRequirements, EntryData,
    EntryKind, FeatureSet, FidelityReport, Layout, ResourceBudget,
};
use crate::ecf::{FORMAT_NAMESPACE, FormatVersion, IndexStatus, OpenedArchive};
use crate::identity::IdentitySet;
use crate::planner::{CompressionProfile, independent_encoded_len};
use crate::similarity::cluster;

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
    pub transforms: Vec<String>,
    pub dictionary: Option<crate::eam::Digest>,
    pub decode: DecodeRequirements,
}

/// Actual unique Chunk usage for one recorded structural transform.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransformUsage {
    pub transform: String,
    pub chunk_count: u64,
}

/// Actual unique Chunk usage for one recorded codec.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodecUsage {
    pub codec: String,
    pub chunk_count: u64,
    pub logical_bytes: u64,
    pub stored_bytes: u64,
}

/// Logical-reference and unique physical-Chunk statistics.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChunkStatistics {
    pub unique_chunk_count: u64,
    pub logical_chunk_references: u64,
    pub unique_plaintext_bytes: u64,
    pub deduplicated_bytes: u64,
    pub dedup_ratio_milli: u64,
    pub minimum_chunk_bytes: u64,
    pub average_chunk_bytes: u64,
    pub maximum_chunk_bytes: u64,
}

/// Dictionary/group presence and bounded random-access costs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CrossFileInspection {
    pub feature_present: bool,
    pub dictionary_count: u64,
    pub dictionary_bytes: u64,
    pub dictionary_backed_chunks: u64,
    pub chunk_group_count: u64,
    pub maximum_lookback: u32,
    pub worst_random_access_chunks: u32,
    pub worst_random_access_bytes: u64,
    pub every_chunk_independently_decodable: bool,
}

/// Reconstructive physical representation and bounded side-data usage.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReconstructionInspection {
    pub feature_present: bool,
    pub object_count: u64,
    pub object_bytes: u64,
    pub chunk_count: u64,
    pub transform_types: Vec<String>,
    pub maximum_intermediate_bytes: u64,
}

/// Whole-ContentObject reconstruction and its declared access tradeoff.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WholeObjectInspection {
    pub feature_present: bool,
    pub region_count: u64,
    pub jpeg_region_count: u64,
    pub logical_bytes: u64,
    pub jpeg_xl_bytes: u64,
    pub stored_representation_bytes: u64,
    pub largest_region_bytes: u64,
    pub worst_access_chunks: u64,
    pub worst_access_bytes: u64,
    pub every_chunk_independently_decodable: bool,
}

/// First observable compression summary, derived without an audit-trail record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompressionExplanation {
    pub planner_id: String,
    pub total_logical_bytes: u64,
    pub total_plaintext_chunk_bytes: u64,
    pub total_stored_chunk_bytes: u64,
    pub chunks: ChunkStatistics,
    pub codec_usage: Vec<CodecUsage>,
    pub store_chunk_count: u64,
    pub store_logical_bytes: u64,
    pub store_stored_bytes: u64,
    pub zstandard_chunk_count: u64,
    pub zstandard_logical_bytes: u64,
    pub zstandard_stored_bytes: u64,
    pub physical_savings_bytes: i128,
    pub ordinary_codec_savings_bytes: i128,
    pub shared_dictionary_savings_bytes: i128,
    pub bounded_lookback_savings_bytes: i128,
    pub structural_transform_savings_bytes: i128,
    pub reconstructive_gross_savings_bytes: i128,
    pub reconstruction_data_overhead_bytes: u64,
    pub reconstructive_net_savings_bytes: i128,
    pub reconstructive_chunk_count: u64,
    pub reconstructive_fallback_chunk_count: u64,
    pub reconstructive_fallback_reason: Option<String>,
    pub jpeg_reconstructive_gross_savings_bytes: i128,
    pub jpeg_representation_bytes: u64,
    pub jpeg_region_overhead_bytes: u64,
    pub jpeg_reconstructive_net_savings_bytes: i128,
    pub jpeg_fallback_reason: Option<String>,
    pub transformed_chunk_count: u64,
    pub transform_rejected_chunk_count: u64,
    pub transform_usage: Vec<TransformUsage>,
    pub representative_pipelines: Vec<String>,
    pub transform_rejection_reason: Option<String>,
    pub dictionary_storage_bytes: u64,
    pub similarity_cohort_count: u64,
    pub similarity_cohort_chunks: u64,
    pub similarity_cohort_logical_bytes: u64,
    pub independent_similarity_cohort_count: u64,
    pub independent_cohort_reason: Option<String>,
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
    /// Declared bound on sequential historical Chunk dependencies. Always zero
    /// in INDEXED layout, where random access makes retention unnecessary.
    pub stream_dedup_window: u64,
    /// Whether the producer declared its resource budget before the payload.
    /// Absence is never a claim that resources are unlimited.
    pub budget_declared: bool,
    /// Whether the layout can resolve one Entry without scanning the container.
    /// This follows from the layout, never from whether the source happened to
    /// be a seekable file.
    pub random_entry_lookup: bool,
    /// Whether the layout carries an Index at all.
    pub index_applicable: bool,
    pub codec_transform_feature_present: bool,
    pub reconstructive_transform_feature_present: bool,
    pub stream_layout_feature_present: bool,
    pub planner_id: String,
    pub chunker_id: String,
    pub plans: Vec<PlanInspection>,
    pub codec_usage: Vec<CodecUsage>,
    pub transform_usage: Vec<TransformUsage>,
    pub transformed_chunk_count: u64,
    pub chunks: ChunkStatistics,
    pub cross_file: CrossFileInspection,
    pub reconstruction: ReconstructionInspection,
    pub whole_object: WholeObjectInspection,
    pub identities: IdentitySet,
    pub index_status: IndexStatus,
    pub fidelity: FidelityReport,
    pub conversion: Option<ConversionProvenance>,
    pub preservation: Option<crate::eam::LegacyPreservation>,
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
                EntryData::Symlink { .. } => 0,
                EntryData::File {
                    content: ContentRef::Internal(digest),
                } => object_size(archive, *digest)?,
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
    let chunks = chunk_statistics(archive)?;
    let cross_file = cross_file_statistics(archive)?;
    let reconstruction = reconstruction_statistics(archive)?;
    let whole_object = whole_object_statistics(archive)?;
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
        stream_dedup_window: archive.descriptor.stream_dedup_window,
        budget_declared: archive.descriptor.budget_declared,
        random_entry_lookup: archive.descriptor.layout.supports_random_entry_lookup(),
        index_applicable: archive.descriptor.layout == Layout::Indexed,
        codec_transform_feature_present: archive.descriptor.features.incompat
            & crate::ecf::FEATURE_CODEC_TRANSFORM_V1
            != 0,
        reconstructive_transform_feature_present: archive.descriptor.features.incompat
            & crate::ecf::FEATURE_RECONSTRUCTIVE_TRANSFORM_V1
            != 0,
        stream_layout_feature_present: archive.descriptor.features.incompat
            & crate::ecf::FEATURE_STREAM_LAYOUT_V1
            != 0,
        planner_id: archive.descriptor.planner_id.clone(),
        chunker_id: archive.descriptor.chunker_id.clone(),
        plans: archive
            .transform_plans
            .iter()
            .map(|plan| PlanInspection {
                plan_id: plan.plan_id,
                identifier: plan.identifier.clone(),
                codec: plan.codec.clone(),
                transforms: plan
                    .transforms
                    .iter()
                    .map(crate::transform::display_step)
                    .collect(),
                dictionary: plan.dictionary,
                decode: plan.decode,
            })
            .collect(),
        codec_usage: codec_usage(opened)?,
        transform_usage: transform_usage(archive)?,
        transformed_chunk_count: transformed_chunk_count(archive)?,
        chunks,
        cross_file,
        reconstruction,
        whole_object,
        identities: opened.report.identities,
        index_status: opened.report.index_status,
        fidelity: archive.fidelity.clone(),
        conversion: archive.conversion.clone(),
        preservation: archive.preservation.clone(),
        declared_resources: archive.descriptor.budget,
        decode_requirements: archive.descriptor.decode,
    })
}

/// Derives physical compression totals from verified Chunk frames and plans.
///
/// This re-derives the physical alternatives the planner considered, so it
/// needs the retained plaintext of every Chunk. A sequential pass that released
/// its plaintext cannot answer these questions; scan again with a retaining
/// content policy rather than reporting derived numbers from absent bytes.
pub fn explain(opened: &OpenedArchive) -> Result<CompressionExplanation> {
    if opened
        .archive
        .content_store
        .chunks
        .values()
        .any(|chunk| chunk.logical_len != 0 && chunk.plaintext.is_empty())
    {
        return Err(Diagnostic::new(
            OutcomeClass::Unsupported,
            ReasonCode::CommandNotImplemented,
            "compression explanation requires retained Chunk plaintext; \
             re-scan the archive with a retaining content policy",
        ));
    }
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
    let chunks = chunk_statistics(&opened.archive)?;
    let (
        ordinary_codec_savings_bytes,
        shared_dictionary_savings_bytes,
        bounded_lookback_savings_bytes,
        structural_transform_savings_bytes,
    ) = separated_codec_savings(opened)?;
    let reconstruction = reconstruction_statistics(&opened.archive)?;
    let reconstructive_gross_savings_bytes = reconstructive_explanation(opened)?;
    let (reconstructive_fallback_chunk_count, reconstructive_fallback_reason) =
        reconstruction_fallback_summary(&opened.archive)?;
    let whole_object = whole_object_statistics(&opened.archive)?;
    let (jpeg_gross, jpeg_overhead, jpeg_net, jpeg_fallback_reason) =
        jpeg_reconstruction_explanation(&opened.archive)?;
    let (
        similarity_cohort_count,
        similarity_cohort_chunks,
        similarity_cohort_logical_bytes,
        independent_similarity_cohort_count,
    ) = similarity_statistics(&opened.archive)?;
    Ok(CompressionExplanation {
        planner_id: opened.archive.descriptor.planner_id.clone(),
        total_logical_bytes: opened.archive.total_logical_size()?,
        total_plaintext_chunk_bytes,
        total_stored_chunk_bytes,
        chunks,
        store_chunk_count: store.map_or(0, |item| item.chunk_count),
        store_logical_bytes: store.map_or(0, |item| item.logical_bytes),
        store_stored_bytes: store.map_or(0, |item| item.stored_bytes),
        zstandard_chunk_count: zstandard.map_or(0, |item| item.chunk_count),
        zstandard_logical_bytes: zstandard.map_or(0, |item| item.logical_bytes),
        zstandard_stored_bytes: zstandard.map_or(0, |item| item.stored_bytes),
        codec_usage: usage,
        physical_savings_bytes: i128::from(total_plaintext_chunk_bytes)
            - i128::from(total_stored_chunk_bytes),
        ordinary_codec_savings_bytes,
        shared_dictionary_savings_bytes,
        bounded_lookback_savings_bytes,
        structural_transform_savings_bytes,
        reconstructive_gross_savings_bytes,
        reconstruction_data_overhead_bytes: reconstruction.object_bytes,
        reconstructive_net_savings_bytes: reconstructive_gross_savings_bytes
            - i128::from(reconstruction.object_bytes),
        reconstructive_chunk_count: reconstruction.chunk_count,
        reconstructive_fallback_chunk_count,
        reconstructive_fallback_reason,
        jpeg_reconstructive_gross_savings_bytes: jpeg_gross,
        jpeg_representation_bytes: whole_object.stored_representation_bytes,
        jpeg_region_overhead_bytes: jpeg_overhead,
        jpeg_reconstructive_net_savings_bytes: jpeg_net,
        jpeg_fallback_reason,
        transformed_chunk_count: transformed_chunk_count(&opened.archive)?,
        transform_rejected_chunk_count: transform_rejected_chunk_count(&opened.archive)?,
        transform_usage: transform_usage(&opened.archive)?,
        representative_pipelines: opened
            .archive
            .transform_plans
            .iter()
            .filter(|plan| !plan.transforms.is_empty())
            .map(|plan| plan.identifier.clone())
            .take(8)
            .collect(),
        transform_rejection_reason: opened
            .archive
            .descriptor
            .planner_id
            .ends_with("-v4")
            .then(|| "transform candidates remained unselected where their complete encoded payload plus canonical plan cost did not clear the frozen gain threshold".to_owned()),
        dictionary_storage_bytes: opened
            .archive
            .content_store
            .dictionaries
            .values()
            .try_fold(0_u64, |total, dictionary| {
                total
                    .checked_add(u64::try_from(dictionary.bytes.len()).map_err(|_| {
                        resource("Dictionary length exceeds u64")
                    })?)
                    .ok_or_else(|| resource("Dictionary byte total exceeds u64"))
            })?,
        similarity_cohort_count,
        similarity_cohort_chunks,
        similarity_cohort_logical_bytes,
        independent_similarity_cohort_count,
        independent_cohort_reason: (independent_similarity_cohort_count != 0).then(|| {
            "independent encoding won because dictionary/lookback training was unavailable or complete cost did not clear the frozen gain threshold".to_owned()
        }),
    })
}

fn reconstruction_statistics(archive: &Archive) -> Result<ReconstructionInspection> {
    let plans = archive
        .transform_plans
        .iter()
        .map(|plan| (plan.plan_id, plan))
        .collect::<BTreeMap<_, _>>();
    let reconstructive_plans = archive
        .content_store
        .chunks
        .values()
        .filter_map(|chunk| {
            let plan = plans.get(&chunk.plan_ref)?;
            plan.transforms
                .iter()
                .any(|step| step.reconstruction_ref.is_some())
                .then_some(plan)
        })
        .collect::<Vec<_>>();
    let transform_types = reconstructive_plans
        .iter()
        .flat_map(|plan| plan.transforms.iter())
        .filter(|step| step.reconstruction_ref.is_some())
        .map(|step| step.transform_id.clone())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    let object_bytes = archive
        .content_store
        .reconstruction_data
        .values()
        .try_fold(0_u64, |total, data| {
            total
                .checked_add(
                    u64::try_from(data.bytes.len())
                        .map_err(|_| resource("ReconstructionData length exceeds u64"))?,
                )
                .ok_or_else(|| resource("ReconstructionData total exceeds u64"))
        })?;
    Ok(ReconstructionInspection {
        feature_present: archive.descriptor.features.incompat
            & crate::ecf::FEATURE_RECONSTRUCTIVE_TRANSFORM_V1
            != 0,
        object_count: u64::try_from(archive.content_store.reconstruction_data.len())
            .map_err(|_| resource("ReconstructionData count exceeds u64"))?,
        object_bytes,
        chunk_count: u64::try_from(reconstructive_plans.len())
            .map_err(|_| resource("reconstructive Chunk count exceeds u64"))?,
        transform_types,
        maximum_intermediate_bytes: archive
            .content_store
            .reconstruction_data
            .values()
            .map(|data| data.intermediate_len)
            .max()
            .unwrap_or(0),
    })
}

fn whole_object_statistics(archive: &Archive) -> Result<WholeObjectInspection> {
    let mut logical_bytes = 0_u64;
    let mut jpeg_xl_bytes = 0_u64;
    let mut stored_representation_bytes = 0_u64;
    let mut largest_region_bytes = 0_u64;
    let mut worst_access_chunks = 0_u64;
    let mut worst_access_bytes = 0_u64;
    for region in archive.content_store.reconstruction_regions.values() {
        logical_bytes = logical_bytes
            .checked_add(region.logical_bytes)
            .ok_or_else(|| resource("region logical-byte total exceeds u64"))?;
        jpeg_xl_bytes = jpeg_xl_bytes
            .checked_add(region.transformed_bytes)
            .ok_or_else(|| resource("JPEG XL byte total exceeds u64"))?;
        stored_representation_bytes = stored_representation_bytes
            .checked_add(
                u64::try_from(region.representation.len())
                    .map_err(|_| resource("region representation exceeds u64"))?,
            )
            .ok_or_else(|| resource("region representation total exceeds u64"))?;
        largest_region_bytes = largest_region_bytes.max(region.logical_bytes);
        worst_access_chunks = worst_access_chunks.max(region.access.logical_chunks);
        worst_access_bytes = worst_access_bytes.max(region.access.worst_reconstructed_bytes);
    }
    let region_count = u64::try_from(archive.content_store.reconstruction_regions.len())
        .map_err(|_| resource("ReconstructionRegion count exceeds u64"))?;
    Ok(WholeObjectInspection {
        feature_present: archive.descriptor.features.incompat
            & crate::ecf::FEATURE_WHOLE_OBJECT_RECONSTRUCTION_V1
            != 0,
        region_count,
        jpeg_region_count: region_count,
        logical_bytes,
        jpeg_xl_bytes,
        stored_representation_bytes,
        largest_region_bytes,
        worst_access_chunks,
        worst_access_bytes,
        every_chunk_independently_decodable: archive
            .content_store
            .reconstruction_regions
            .values()
            .all(|region| region.chunk_count <= 1),
    })
}

fn jpeg_reconstruction_explanation(archive: &Archive) -> Result<(i128, u64, i128, Option<String>)> {
    use crate::eam::ReconstructionAuditReason;

    let mut gross = 0_i128;
    let mut overhead = 0_u64;
    for region in archive.content_store.reconstruction_regions.values() {
        let stored = u64::try_from(region.representation.len())
            .map_err(|_| resource("region representation exceeds u64"))?;
        gross += i128::from(region.ordinary_physical_bytes) - i128::from(stored);
        overhead = overhead
            .checked_add(region.region_overhead_bytes)
            .ok_or_else(|| resource("region overhead total exceeds u64"))?;
    }
    let net = gross - i128::from(overhead);
    let mut counts = [0_u64; 6];
    for audit in archive.content_store.reconstruction_audits.values() {
        let index = match audit.reason {
            ReconstructionAuditReason::NotRecognized => 0,
            ReconstructionAuditReason::Unsupported => 1,
            ReconstructionAuditReason::ExactVerificationFailed => 2,
            ReconstructionAuditReason::CompleteCostDidNotWin => 3,
            ReconstructionAuditReason::RegionDedupConflict => 4,
            ReconstructionAuditReason::ResourcePolicyExcluded => 5,
        };
        counts[index] = counts[index]
            .checked_add(1)
            .ok_or_else(|| resource("JPEG audit count exceeds u64"))?;
    }
    let fallback = counts.iter().any(|count| *count != 0).then(|| {
        format!(
            "not-recognized={}, unsupported={}, exact-verification-failed={}, complete-cost-rejected={}, dedup-conflict={}, resource-policy-excluded={}",
            counts[0], counts[1], counts[2], counts[3], counts[4], counts[5]
        )
    });
    Ok((gross, overhead, net, fallback))
}

fn region_owned_chunks(
    archive: &Archive,
) -> Result<std::collections::BTreeSet<crate::eam::Digest>> {
    let mut owned = std::collections::BTreeSet::new();
    for region in archive.content_store.reconstruction_regions.values() {
        let object = archive
            .content_store
            .objects
            .get(&region.content_object)
            .ok_or_else(|| resource("region ContentObject is absent"))?;
        let start = usize::try_from(region.start_chunk_index)
            .map_err(|_| resource("region start exceeds usize"))?;
        let end = start
            .checked_add(
                usize::try_from(region.chunk_count)
                    .map_err(|_| resource("region count exceeds usize"))?,
            )
            .ok_or_else(|| resource("region range overflows"))?;
        let range = object
            .chunks
            .get(start..end)
            .ok_or_else(|| resource("region range is invalid"))?;
        owned.extend(range.iter().map(|chunk_ref| chunk_ref.chunk_id));
    }
    Ok(owned)
}

fn reconstructive_explanation(opened: &OpenedArchive) -> Result<i128> {
    let archive = &opened.archive;
    if !archive.descriptor.planner_id.ends_with("-v5")
        && !archive.descriptor.planner_id.ends_with("-v6")
    {
        return Ok(0);
    }
    let profile = CompressionProfile::from_planner_id(&archive.descriptor.planner_id)
        .ok_or_else(|| resource("unknown v5 profile"))?;
    let plans = archive
        .transform_plans
        .iter()
        .map(|plan| (plan.plan_id, plan))
        .collect::<BTreeMap<_, _>>();
    let mut gross = 0_i128;
    let region_owned = region_owned_chunks(archive)?;
    for (chunk_id, chunk) in &archive.content_store.chunks {
        if region_owned.contains(chunk_id) {
            continue;
        }
        let plan = plans[&chunk.plan_ref];
        if plan
            .transforms
            .iter()
            .any(|step| step.reconstruction_ref.is_some())
        {
            let ordinary =
                independent_encoded_len(profile, profile.planner_v4_id(), &chunk.plaintext)?;
            gross += i128::try_from(ordinary)
                .map_err(|_| resource("ordinary size exceeds i128"))?
                - i128::from(archive.index.chunks[chunk_id].stored_len);
        }
    }
    Ok(gross)
}

fn reconstruction_fallback_summary(archive: &Archive) -> Result<(u64, Option<String>)> {
    use crate::eam::ReconstructionFallbackReason;

    let mut unrecognized = 0_u64;
    let mut cost_rejected = 0_u64;
    for reason in archive.content_store.reconstruction_fallbacks.values() {
        match reason {
            ReconstructionFallbackReason::UnrecognizedOrVerificationFailed => {
                unrecognized = unrecognized
                    .checked_add(1)
                    .ok_or_else(|| resource("reconstruction fallback count exceeds u64"))?;
            }
            ReconstructionFallbackReason::CompleteCostDidNotWin => {
                cost_rejected = cost_rejected
                    .checked_add(1)
                    .ok_or_else(|| resource("reconstruction fallback count exceeds u64"))?;
            }
        }
    }
    let total = unrecognized
        .checked_add(cost_rejected)
        .ok_or_else(|| resource("reconstruction fallback count exceeds u64"))?;
    let summary = (total != 0).then(|| {
        format!(
            "unrecognized-or-verification-failed={unrecognized}, complete-cost-rejected={cost_rejected}"
        )
    });
    Ok((total, summary))
}

fn cross_file_statistics(archive: &Archive) -> Result<CrossFileInspection> {
    let plans = archive
        .transform_plans
        .iter()
        .map(|plan| (plan.plan_id, plan))
        .collect::<BTreeMap<_, _>>();
    let dictionary_backed_chunks = archive
        .content_store
        .chunks
        .values()
        .filter(|chunk| {
            plans
                .get(&chunk.plan_ref)
                .is_some_and(|plan| plan.dictionary.is_some())
        })
        .count();
    let dictionary_bytes =
        archive
            .content_store
            .dictionaries
            .values()
            .try_fold(0_u64, |total, dictionary| {
                total
                    .checked_add(
                        u64::try_from(dictionary.bytes.len())
                            .map_err(|_| resource("Dictionary length exceeds u64"))?,
                    )
                    .ok_or_else(|| resource("Dictionary byte total exceeds u64"))
            })?;
    let maximum_lookback = archive
        .content_store
        .chunk_groups
        .values()
        .map(|group| group.max_lookback)
        .max()
        .unwrap_or(0);
    Ok(CrossFileInspection {
        feature_present: archive.descriptor.features.incompat
            & crate::ecf::FEATURE_CROSS_FILE_COMPRESSION_V1
            != 0,
        dictionary_count: u64::try_from(archive.content_store.dictionaries.len())
            .map_err(|_| resource("Dictionary count exceeds u64"))?,
        dictionary_bytes,
        dictionary_backed_chunks: u64::try_from(dictionary_backed_chunks)
            .map_err(|_| resource("dictionary-backed Chunk count exceeds u64"))?,
        chunk_group_count: u64::try_from(archive.content_store.chunk_groups.len())
            .map_err(|_| resource("ChunkGroup count exceeds u64"))?,
        maximum_lookback,
        worst_random_access_chunks: maximum_lookback,
        worst_random_access_bytes: archive
            .content_store
            .chunk_groups
            .values()
            .map(|group| group.max_preceding_bytes)
            .max()
            .unwrap_or(0),
        every_chunk_independently_decodable: archive.content_store.chunk_groups.is_empty(),
    })
}

fn separated_codec_savings(opened: &OpenedArchive) -> Result<(i128, i128, i128, i128)> {
    let archive = &opened.archive;
    let profile = CompressionProfile::from_planner_id(&archive.descriptor.planner_id);
    let plans = archive
        .transform_plans
        .iter()
        .map(|plan| (plan.plan_id, plan))
        .collect::<BTreeMap<_, _>>();
    let mut ordinary = 0_i128;
    let mut dictionary = 0_i128;
    let mut lookback = 0_i128;
    let mut transforms = 0_i128;
    let region_owned = region_owned_chunks(archive)?;
    for (chunk_id, chunk) in &archive.content_store.chunks {
        if region_owned.contains(chunk_id) {
            continue;
        }
        let stored = archive.index.chunks[chunk_id].stored_len;
        let baseline = match profile {
            Some(profile) => {
                independent_encoded_len(profile, &archive.descriptor.planner_id, &chunk.plaintext)?
            }
            None => chunk.plaintext.len(),
        };
        let baseline = i128::try_from(baseline)
            .map_err(|_| resource("independent payload length exceeds i128"))?;
        match crate::codec::plan_mode(plans[&chunk.plan_ref])? {
            crate::codec::PlanMode::Independent => {
                let plan = plans[&chunk.plan_ref];
                if plan
                    .transforms
                    .iter()
                    .any(|step| step.reconstruction_ref.is_some())
                {
                    ordinary += i128::from(chunk.logical_len) - baseline;
                } else if !plan.transforms.is_empty() {
                    let base = crate::codec::without_transforms(plan)?;
                    let base_stored = crate::codec::encode_payload(&base, &chunk.plaintext)?;
                    let base_stored = i128::try_from(base_stored.len())
                        .map_err(|_| resource("base codec payload length exceeds i128"))?;
                    ordinary += i128::from(chunk.logical_len) - base_stored;
                    transforms += base_stored - i128::from(stored);
                } else if plan.codec != "store/v1" {
                    ordinary += i128::from(chunk.logical_len) - i128::from(stored);
                }
            }
            crate::codec::PlanMode::Dictionary(_) => {
                ordinary += i128::from(chunk.logical_len) - baseline;
                dictionary += baseline - i128::from(stored);
            }
            crate::codec::PlanMode::Prefix { .. } => {
                ordinary += i128::from(chunk.logical_len) - baseline;
                lookback += baseline - i128::from(stored);
            }
        }
    }
    Ok((ordinary, dictionary, lookback, transforms))
}

fn similarity_statistics(archive: &Archive) -> Result<(u64, u64, u64, u64)> {
    let Some(profile) = CompressionProfile::from_planner_id(&archive.descriptor.planner_id) else {
        return Ok((0, 0, 0, 0));
    };
    if !archive.descriptor.planner_id.ends_with("-v3")
        && !archive.descriptor.planner_id.ends_with("-v4")
        && !archive.descriptor.planner_id.ends_with("-v5")
        && !archive.descriptor.planner_id.ends_with("-v6")
    {
        return Ok((0, 0, 0, 0));
    }
    let cohorts = cluster(&archive.content_store.chunks, profile.similarity_policy());
    let plans = archive
        .transform_plans
        .iter()
        .map(|plan| (plan.plan_id, plan))
        .collect::<BTreeMap<_, _>>();
    let independent = cohorts
        .iter()
        .filter(|cohort| {
            cohort.chunks.iter().all(|chunk_id| {
                let chunk = &archive.content_store.chunks[chunk_id];
                plans
                    .get(&chunk.plan_ref)
                    .is_none_or(|plan| plan.dictionary.is_none())
                    && chunk.group_ref.is_none()
            })
        })
        .count();
    let cohort_chunks = cohorts.iter().try_fold(0_u64, |total, cohort| {
        total
            .checked_add(
                u64::try_from(cohort.chunks.len())
                    .map_err(|_| resource("cohort Chunk count exceeds u64"))?,
            )
            .ok_or_else(|| resource("cohort Chunk total exceeds u64"))
    })?;
    let cohort_bytes = cohorts.iter().try_fold(0_u64, |total, cohort| {
        total
            .checked_add(cohort.logical_bytes)
            .ok_or_else(|| resource("cohort logical byte total exceeds u64"))
    })?;
    Ok((
        u64::try_from(cohorts.len()).map_err(|_| resource("cohort count exceeds u64"))?,
        cohort_chunks,
        cohort_bytes,
        u64::try_from(independent).map_err(|_| resource("independent cohort count exceeds u64"))?,
    ))
}

fn transformed_chunk_count(archive: &Archive) -> Result<u64> {
    let plans = archive
        .transform_plans
        .iter()
        .map(|plan| (plan.plan_id, plan))
        .collect::<BTreeMap<_, _>>();
    let ordinary = u64::try_from(
        archive
            .content_store
            .chunks
            .values()
            .filter(|chunk| {
                plans
                    .get(&chunk.plan_ref)
                    .is_some_and(|plan| !plan.transforms.is_empty())
            })
            .count(),
    )
    .map_err(|_| resource("transformed Chunk count exceeds u64"))?;
    archive
        .content_store
        .reconstruction_regions
        .values()
        .try_fold(ordinary, |total, region| {
            total
                .checked_add(region.chunk_count)
                .ok_or_else(|| resource("transformed Chunk count exceeds u64"))
        })
}

fn transform_usage(archive: &Archive) -> Result<Vec<TransformUsage>> {
    let plans = archive
        .transform_plans
        .iter()
        .map(|plan| (plan.plan_id, plan))
        .collect::<BTreeMap<_, _>>();
    let mut usage = BTreeMap::<String, u64>::new();
    for chunk in archive.content_store.chunks.values() {
        let Some(plan) = plans.get(&chunk.plan_ref) else {
            continue;
        };
        for step in &plan.transforms {
            let value = usage
                .entry(crate::transform::display_step(step))
                .or_default();
            *value = value
                .checked_add(1)
                .ok_or_else(|| resource("transform usage count exceeds u64"))?;
        }
    }
    for region in archive.content_store.reconstruction_regions.values() {
        let plan = plans.get(&region.plan_ref).ok_or_else(|| {
            resource("ReconstructionRegion TransformPlan is absent from inspection")
        })?;
        for step in &plan.transforms {
            let value = usage
                .entry(crate::transform::display_step(step))
                .or_default();
            *value = value
                .checked_add(region.chunk_count)
                .ok_or_else(|| resource("transform usage count exceeds u64"))?;
        }
    }
    Ok(usage
        .into_iter()
        .map(|(transform, chunk_count)| TransformUsage {
            transform,
            chunk_count,
        })
        .collect())
}

fn transform_rejected_chunk_count(archive: &Archive) -> Result<u64> {
    if !archive.descriptor.planner_id.ends_with("-v4")
        && !archive.descriptor.planner_id.ends_with("-v5")
        && !archive.descriptor.planner_id.ends_with("-v6")
    {
        return Ok(0);
    }
    let Some(profile) = CompressionProfile::from_planner_id(&archive.descriptor.planner_id) else {
        return Ok(0);
    };
    if profile == CompressionProfile::Fast {
        return Ok(0);
    }
    let plans = archive
        .transform_plans
        .iter()
        .map(|plan| (plan.plan_id, plan))
        .collect::<BTreeMap<_, _>>();
    u64::try_from(
        archive
            .content_store
            .chunks
            .values()
            .filter(|chunk| {
                crate::planner::analyze(&chunk.plaintext).likely_compressible
                    && plans
                        .get(&chunk.plan_ref)
                        .is_none_or(|plan| plan.transforms.is_empty())
            })
            .count(),
    )
    .map_err(|_| resource("rejected transform candidate count exceeds u64"))
}

fn chunk_statistics(archive: &Archive) -> Result<ChunkStatistics> {
    let unique_chunk_count = u64::try_from(archive.content_store.chunks.len())
        .map_err(|_| resource("unique Chunk count exceeds u64"))?;
    let unique_plaintext_bytes =
        archive
            .content_store
            .chunks
            .values()
            .try_fold(0_u64, |total, chunk| {
                total
                    .checked_add(chunk.logical_len)
                    .ok_or_else(|| resource("unique plaintext byte total exceeds u64"))
            })?;
    let logical_chunk_references =
        archive
            .entry_set
            .entries()
            .iter()
            .try_fold(0_u64, |total, entry| {
                let EntryData::File {
                    content: ContentRef::Internal(digest),
                } = entry.data()
                else {
                    return Ok(total);
                };
                let object = archive.content_store.objects.get(digest).ok_or_else(|| {
                    Diagnostic::new(
                        OutcomeClass::Nonconforming,
                        ReasonCode::UnknownContentObject,
                        digest.to_string(),
                    )
                })?;
                total
                    .checked_add(
                        u64::try_from(object.chunks.len())
                            .map_err(|_| resource("logical Chunk refs exceed u64"))?,
                    )
                    .ok_or_else(|| resource("logical Chunk-reference count exceeds u64"))
            })?;
    let total_logical = archive.total_logical_size()?;
    let deduplicated_bytes = total_logical.saturating_sub(unique_plaintext_bytes);
    let dedup_ratio_milli = if unique_plaintext_bytes == 0 {
        1_000
    } else {
        let ratio = u128::from(total_logical)
            .checked_mul(1_000)
            .ok_or_else(|| resource("dedup ratio exceeds u128"))?
            / u128::from(unique_plaintext_bytes);
        u64::try_from(ratio).map_err(|_| resource("dedup ratio exceeds u64"))?
    };
    let minimum_chunk_bytes = archive
        .content_store
        .chunks
        .values()
        .map(|chunk| chunk.logical_len)
        .min()
        .unwrap_or(0);
    let maximum_chunk_bytes = archive
        .content_store
        .chunks
        .values()
        .map(|chunk| chunk.logical_len)
        .max()
        .unwrap_or(0);
    let average_chunk_bytes = unique_plaintext_bytes
        .checked_div(unique_chunk_count)
        .unwrap_or(0);
    Ok(ChunkStatistics {
        unique_chunk_count,
        logical_chunk_references,
        unique_plaintext_bytes,
        deduplicated_bytes,
        dedup_ratio_milli,
        minimum_chunk_bytes,
        average_chunk_bytes,
        maximum_chunk_bytes,
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
    let region_owned = region_owned_chunks(archive)?;
    for (chunk_id, chunk) in &archive.content_store.chunks {
        if region_owned.contains(chunk_id) {
            continue;
        }
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
    for region in archive.content_store.reconstruction_regions.values() {
        let codec = plans.get(&region.plan_ref).ok_or_else(|| {
            Diagnostic::new(
                OutcomeClass::Unsupported,
                ReasonCode::UnknownTransformPlan,
                format!(
                    "region {} references plan {}",
                    region.region_id, region.plan_ref
                ),
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
            .checked_add(region.chunk_count)
            .ok_or_else(|| resource("codec Chunk count exceeds u64"))?;
        item.logical_bytes = item
            .logical_bytes
            .checked_add(region.logical_bytes)
            .ok_or_else(|| resource("codec logical byte total exceeds u64"))?;
        item.stored_bytes = item
            .stored_bytes
            .checked_add(
                u64::try_from(region.representation.len())
                    .map_err(|_| resource("region representation exceeds u64"))?,
            )
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
