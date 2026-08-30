use std::collections::BTreeMap;

use crate::diagnostics::{Diagnostic, OutcomeClass, ReasonCode, Result};
use crate::eam::{
    Archive, ArchiveRole, ContentRef, DecodeRequirements, EntryData, EntryKind, FeatureSet,
    FidelityReport, Layout, ResourceBudget,
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
    pub codec_transform_feature_present: bool,
    pub planner_id: String,
    pub chunker_id: String,
    pub plans: Vec<PlanInspection>,
    pub codec_usage: Vec<CodecUsage>,
    pub transform_usage: Vec<TransformUsage>,
    pub transformed_chunk_count: u64,
    pub chunks: ChunkStatistics,
    pub cross_file: CrossFileInspection,
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
    let chunks = chunk_statistics(archive)?;
    let cross_file = cross_file_statistics(archive)?;
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
        codec_transform_feature_present: archive.descriptor.features.incompat
            & crate::ecf::FEATURE_CODEC_TRANSFORM_V1
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
    let chunks = chunk_statistics(&opened.archive)?;
    let (
        ordinary_codec_savings_bytes,
        shared_dictionary_savings_bytes,
        bounded_lookback_savings_bytes,
        structural_transform_savings_bytes,
    ) = separated_codec_savings(opened)?;
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
        .filter(|chunk| plans[&chunk.plan_ref].dictionary.is_some())
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
    for (chunk_id, chunk) in &archive.content_store.chunks {
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
                if !plan.transforms.is_empty() {
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
                plans[&chunk.plan_ref].dictionary.is_none() && chunk.group_ref.is_none()
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
    u64::try_from(
        archive
            .content_store
            .chunks
            .values()
            .filter(|chunk| !plans[&chunk.plan_ref].transforms.is_empty())
            .count(),
    )
    .map_err(|_| resource("transformed Chunk count exceeds u64"))
}

fn transform_usage(archive: &Archive) -> Result<Vec<TransformUsage>> {
    let plans = archive
        .transform_plans
        .iter()
        .map(|plan| (plan.plan_id, plan))
        .collect::<BTreeMap<_, _>>();
    let mut usage = BTreeMap::<String, u64>::new();
    for chunk in archive.content_store.chunks.values() {
        for step in &plans[&chunk.plan_ref].transforms {
            let value = usage
                .entry(crate::transform::display_step(step))
                .or_default();
            *value = value
                .checked_add(1)
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
    if !archive.descriptor.planner_id.ends_with("-v4") {
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
                    && plans[&chunk.plan_ref].transforms.is_empty()
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
                let object = archive.content_store.objects.get(&digest).ok_or_else(|| {
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
