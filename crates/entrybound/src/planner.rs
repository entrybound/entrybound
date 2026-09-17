//! Deterministic creation-time compression planning.
//!
//! Profiles exist only here. The native reader uses recorded TransformPlans
//! and operational codecs without consulting this module.

use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr;

use crate::chunker::{BALANCED_V2, ChunkingParameters, DENSE_V2, EXTREME_V2, FAST_V2};
use crate::codec::{
    LZ4_CODEC_IDENTIFIER, LZMA2_CODEC_IDENTIFIER, ZSTD_CODEC_IDENTIFIER,
    ZSTD_DICTIONARY_CONSTRUCTION_PREFIX, ZSTD_DICTIONARY_FORMAT, ZSTD_WINDOW_BYTES,
    aggregate_archive_decode_requirements, aggregate_decode_requirements, encode_payload,
    encode_payload_with_dictionary, encode_payload_with_prefix, encode_payload_with_reconstruction,
    encode_transformed_payload, lz4_plan, lzma2_plan, store_plan, train_dictionary, with_pipeline,
    zstd_dictionary_plan, zstd_plan, zstd_prefix_plan, zstd_transformed_plan,
};
use crate::diagnostics::{Diagnostic, OutcomeClass, ReasonCode, Result};
use crate::eam::{
    Archive, ChunkGroup, Dictionary, Digest, ReconstructionAudit, ReconstructionAuditReason,
    ReconstructionAuditTarget, ReconstructionData, ReconstructionFallbackReason,
    ReconstructionRegion, RegionAccessCost, TransformPlan,
};
use crate::ecf::{
    FEATURE_CODEC_TRANSFORM_V1, FEATURE_CROSS_FILE_COMPRESSION_V1,
    FEATURE_RECONSTRUCTIVE_TRANSFORM_V1, FEATURE_WHOLE_OBJECT_RECONSTRUCTION_V1,
    SECTION_HEADER_LEN, encoded_chunk_group_len, encoded_dictionary_len,
    encoded_reconstruction_data_len, encoded_reconstruction_region_len, encoded_transform_plan_len,
    encoded_transform_plan_v2_len, encoded_transform_plan_v3_len,
};
use crate::identity::sha256_exact;
use crate::similarity::{
    BALANCED_V3_SIMILARITY, DENSE_V3_SIMILARITY, EXTREME_V3_SIMILARITY, FAST_V3_SIMILARITY,
    SimilarityCohort, SimilarityPolicy, cluster,
};
use crate::transform::{
    byte_shuffle_step, deflate_reconstruct_step, delta8_step, jpeg_reconstruct_step,
};

/// Temporary plan marker used only between plaintext chunking and planning.
pub const UNPLANNED_PLAN_ID: u64 = 0;
/// Per-chunk cost assigned to selecting a non-STORE plan.
pub const ZSTD_ATTRIBUTABLE_OVERHEAD_BYTES: u64 = 16;
/// Absolute minimum physical gain required before Zstandard may win.
pub const MINIMUM_GAIN_BYTES: u64 = 32;
/// Relative minimum gain: one percent, expressed in basis points.
pub const MINIMUM_GAIN_BASIS_POINTS: u64 = 100;
const MINIMUM_ZSTD_INPUT_BYTES: usize = 64;
const PROBE_BYTES: usize = 4096;
const MINIMUM_COHORT_GAIN_BYTES: u64 = 128;
const DICTIONARY_SECTION_OVERHEAD_BYTES: u64 = 64;
const CHUNK_GROUP_SECTION_OVERHEAD_BYTES: u64 = 64;
const RECONSTRUCTION_SECTION_OVERHEAD_BYTES: u64 = 64;
const MINIMUM_RECONSTRUCTION_GAIN_BYTES: u64 = 256;
const MINIMUM_RECONSTRUCTION_GAIN_BASIS_POINTS: u64 = 200;

const FAST_LEVELS: [i32; 1] = [1];
const BALANCED_LEVELS: [i32; 3] = [1, 3, 5];
const BALANCED_FALLBACK_LEVELS: [i32; 1] = [1];
const DENSE_LEVELS: [i32; 3] = [5, 9, 15];
const EXTREME_LEVELS: [i32; 4] = [9, 15, 19, 22];

/// Public creation profiles. New filesystem archives use their frozen v6 IDs.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CompressionProfile {
    Fast,
    #[default]
    Balanced,
    Dense,
    Extreme,
}

impl CompressionProfile {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Fast => "fast",
            Self::Balanced => "balanced",
            Self::Dense => "dense",
            Self::Extreme => "extreme",
        }
    }

    #[must_use]
    pub const fn planner_id(self) -> &'static str {
        match self {
            Self::Fast => "fast-v6",
            Self::Balanced => "balanced-v6",
            Self::Dense => "dense-v6",
            Self::Extreme => "extreme-v6",
        }
    }

    #[must_use]
    pub const fn planner_v5_id(self) -> &'static str {
        match self {
            Self::Fast => "fast-v5",
            Self::Balanced => "balanced-v5",
            Self::Dense => "dense-v5",
            Self::Extreme => "extreme-v5",
        }
    }

    #[must_use]
    pub const fn planner_v4_id(self) -> &'static str {
        match self {
            Self::Fast => "fast-v4",
            Self::Balanced => "balanced-v4",
            Self::Dense => "dense-v4",
            Self::Extreme => "extreme-v4",
        }
    }

    /// Frozen IDs for similarity/dictionary/group planning.
    #[must_use]
    pub const fn planner_v3_id(self) -> &'static str {
        match self {
            Self::Fast => "fast-v3",
            Self::Balanced => "balanced-v3",
            Self::Dense => "dense-v3",
            Self::Extreme => "extreme-v3",
        }
    }

    /// Frozen IDs for CDC plus exact-dedup and independent codec planning.
    #[must_use]
    pub const fn planner_v2_id(self) -> &'static str {
        match self {
            Self::Fast => "fast-v2",
            Self::Balanced => "balanced-v2",
            Self::Dense => "dense-v2",
            Self::Extreme => "extreme-v2",
        }
    }

    /// Frozen ID used by the original fixed-chunk planner implementation.
    #[must_use]
    pub const fn planner_v1_id(self) -> &'static str {
        match self {
            Self::Fast => "fast-v1",
            Self::Balanced => "balanced-v1",
            Self::Dense => "dense-v1",
            Self::Extreme => "extreme-v1",
        }
    }

    /// Ordered CDC candidates. Earlier candidates win complete-cost ties.
    #[must_use]
    pub const fn chunking_candidates(self) -> &'static [ChunkingParameters] {
        match self {
            Self::Fast => &[FAST_V2],
            Self::Balanced => &[BALANCED_V2],
            Self::Dense => &[BALANCED_V2, DENSE_V2],
            Self::Extreme => &[FAST_V2, BALANCED_V2, DENSE_V2, EXTREME_V2],
        }
    }

    fn levels(self, analysis: ContentAnalysis) -> &'static [i32] {
        match self {
            Self::Fast if analysis.likely_compressible => &FAST_LEVELS,
            Self::Fast => &[],
            Self::Balanced if analysis.likely_compressible => &BALANCED_LEVELS,
            Self::Balanced => &BALANCED_FALLBACK_LEVELS,
            Self::Dense => &DENSE_LEVELS,
            Self::Extreme => &EXTREME_LEVELS,
        }
    }

    #[must_use]
    pub const fn similarity_policy(self) -> SimilarityPolicy {
        match self {
            Self::Fast => FAST_V3_SIMILARITY,
            Self::Balanced => BALANCED_V3_SIMILARITY,
            Self::Dense => DENSE_V3_SIMILARITY,
            Self::Extreme => EXTREME_V3_SIMILARITY,
        }
    }

    const fn dictionary_levels(self) -> &'static [i32] {
        match self {
            Self::Fast => &[],
            Self::Balanced => &[3, 5],
            Self::Dense => &[5, 9, 15],
            Self::Extreme => &[9, 15, 19],
        }
    }

    #[must_use]
    pub const fn lookback_candidates(self) -> &'static [u32] {
        match self {
            Self::Fast | Self::Balanced => &[],
            Self::Dense => &[1, 2, 4],
            Self::Extreme => &[1, 2, 4, 8],
        }
    }

    const fn lookback_levels(self) -> &'static [i32] {
        match self {
            Self::Fast | Self::Balanced => &[],
            Self::Dense => &[5, 9],
            Self::Extreme => &[9, 15, 19],
        }
    }

    #[must_use]
    pub fn from_planner_id(planner_id: &str) -> Option<Self> {
        match planner_id {
            "fast-v1" | "fast-v2" | "fast-v3" | "fast-v4" | "fast-v5" | "fast-v6" => {
                Some(Self::Fast)
            }
            "balanced-v1" | "balanced-v2" | "balanced-v3" | "balanced-v4" | "balanced-v5"
            | "balanced-v6" => Some(Self::Balanced),
            "dense-v1" | "dense-v2" | "dense-v3" | "dense-v4" | "dense-v5" | "dense-v6" => {
                Some(Self::Dense)
            }
            "extreme-v1" | "extreme-v2" | "extreme-v3" | "extreme-v4" | "extreme-v5"
            | "extreme-v6" => Some(Self::Extreme),
            _ => None,
        }
    }
}

impl FromStr for CompressionProfile {
    type Err = Diagnostic;

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "fast" => Ok(Self::Fast),
            "balanced" => Ok(Self::Balanced),
            "dense" => Ok(Self::Dense),
            "extreme" => Ok(Self::Extreme),
            _ => Err(Diagnostic::new(
                OutcomeClass::PolicyRefused,
                ReasonCode::CommandUsage,
                format!(
                    "unknown compression profile '{value}'; expected fast, balanced, dense, or extreme"
                ),
            )),
        }
    }
}

/// Integer-only deterministic probe used to bound profile search effort.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ContentAnalysis {
    pub plaintext_len: u64,
    pub empty: bool,
    pub sample_len: u64,
    pub distinct_symbols: u16,
    pub maximum_symbol_frequency_ppm: u32,
    pub adjacent_repeat_ppm: u32,
    pub likely_compressible: bool,
}

/// Aggregate result of applying one planner profile to an Archive.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlanningReport {
    pub planner_id: String,
    pub store_chunks: u64,
    pub zstandard_chunks: u64,
    pub lz4_chunks: u64,
    pub lzma2_chunks: u64,
    pub transformed_chunks: u64,
    pub dictionary_chunks: u64,
    pub lookback_chunks: u64,
    pub similarity_cohorts: u64,
    pub reconstructive_chunks: u64,
    pub reconstruction_attempts: u64,
    pub reconstruction_cost_rejections: u64,
    pub selected_plans: Box<[TransformPlan]>,
}

/// Frozen v1 codec selection for callers constructing historical fixed chunks.
pub fn plan_archive(archive: &mut Archive, profile: CompressionProfile) -> Result<PlanningReport> {
    clear_whole_object_state(archive);
    archive.descriptor.features.incompat &= !FEATURE_RECONSTRUCTIVE_TRANSFORM_V1;
    archive.content_store.reconstruction_data.clear();
    archive.content_store.reconstruction_fallbacks.clear();
    plan_archive_with_id(archive, profile, profile.planner_v1_id())
}

/// V2 codec selection after CDC and archive-wide exact deduplication.
pub fn plan_archive_v2(
    archive: &mut Archive,
    profile: CompressionProfile,
) -> Result<PlanningReport> {
    clear_whole_object_state(archive);
    archive.descriptor.features.incompat &= !FEATURE_RECONSTRUCTIVE_TRANSFORM_V1;
    archive.content_store.reconstruction_data.clear();
    archive.content_store.reconstruction_fallbacks.clear();
    plan_archive_with_id(archive, profile, profile.planner_v2_id())
}

/// V3 planning adds deterministic cohorts, shared dictionaries, bounded
/// lookback groups, and an explicit physical Chunk order.
pub fn plan_archive_v3(
    archive: &mut Archive,
    profile: CompressionProfile,
) -> Result<PlanningReport> {
    clear_whole_object_state(archive);
    let planner_id = profile.planner_v3_id();
    archive.descriptor.features.incompat &= !FEATURE_RECONSTRUCTIVE_TRANSFORM_V1;
    archive.content_store.reconstruction_data.clear();
    archive.content_store.reconstruction_fallbacks.clear();
    archive.descriptor.features.incompat &= !FEATURE_CODEC_TRANSFORM_V1;
    let report = plan_archive_with_id(archive, profile, planner_id)?;
    finish_cross_file_planning(
        archive,
        profile,
        planner_id,
        FEATURE_CROSS_FILE_COMPRESSION_V1,
        report,
    )
}

/// V4 planning competes registered independent codec/transform pipelines, then
/// applies the frozen v3 cross-file strategies against that stronger baseline.
pub fn plan_archive_v4(
    archive: &mut Archive,
    profile: CompressionProfile,
) -> Result<PlanningReport> {
    clear_whole_object_state(archive);
    let planner_id = profile.planner_v4_id();
    archive.descriptor.features.incompat &= !FEATURE_RECONSTRUCTIVE_TRANSFORM_V1;
    archive.content_store.reconstruction_data.clear();
    archive.content_store.reconstruction_fallbacks.clear();
    let report = plan_archive_v4_independent(archive, profile, planner_id)?;
    finish_cross_file_planning(
        archive,
        profile,
        planner_id,
        FEATURE_CROSS_FILE_COMPRESSION_V1 | FEATURE_CODEC_TRANSFORM_V1,
        report,
    )
}

/// V5 adds verified, cost-qualified bit-exact DEFLATE reconstruction before
/// the unchanged v3 cohort strategies compete against the independent result.
pub fn plan_archive_v5(
    archive: &mut Archive,
    profile: CompressionProfile,
) -> Result<PlanningReport> {
    clear_whole_object_state(archive);
    let planner_id = profile.planner_v5_id();
    let report = plan_archive_v5_independent(archive, profile, planner_id)?;
    let mut features = FEATURE_CROSS_FILE_COMPRESSION_V1 | FEATURE_CODEC_TRANSFORM_V1;
    if !archive.content_store.reconstruction_data.is_empty()
        || !archive.content_store.reconstruction_fallbacks.is_empty()
    {
        features |= FEATURE_RECONSTRUCTIVE_TRANSFORM_V1;
    }
    finish_cross_file_planning(archive, profile, planner_id, features, report)
}

fn clear_whole_object_state(archive: &mut Archive) {
    archive.descriptor.features.incompat &= !FEATURE_WHOLE_OBJECT_RECONSTRUCTION_V1;
    archive.content_store.reconstruction_regions.clear();
    archive.content_store.reconstruction_audits.clear();
}

/// V6 preserves the frozen v5 baseline, then competes one verified,
/// self-contained JPEG reconstruction region against each eligible complete
/// ContentObject. Region selection never changes logical Chunk boundaries.
pub fn plan_archive_v6(
    archive: &mut Archive,
    profile: CompressionProfile,
) -> Result<PlanningReport> {
    let mut report = plan_archive_v5(archive, profile)?;
    archive.descriptor.planner_id = profile.planner_id().to_owned();
    clear_whole_object_state(archive);
    if profile == CompressionProfile::Fast {
        report.planner_id = profile.planner_id().to_owned();
        return Ok(report);
    }

    let object_owners = chunk_object_owners(archive);
    let objects = archive
        .content_store
        .objects
        .values()
        .cloned()
        .collect::<Vec<_>>();
    let mut plans = archive
        .transform_plans
        .iter()
        .cloned()
        .map(|plan| (plan.plan_id, plan))
        .collect::<BTreeMap<_, _>>();
    let mut emitted_region_section = false;

    for object in objects {
        let target = ReconstructionAuditTarget::ContentObject(object.logical_digest);
        increment(
            &mut report.reconstruction_attempts,
            "JPEG reconstruction attempt",
        )?;
        if object.chunks.is_empty() {
            insert_v6_audit(archive, target, ReconstructionAuditReason::NotRecognized);
            continue;
        }
        if profile == CompressionProfile::Balanced && object.chunks.len() != 1 {
            insert_v6_audit(
                archive,
                target,
                ReconstructionAuditReason::ResourcePolicyExcluded,
            );
            continue;
        }
        if u64::try_from(object.chunks.len()).unwrap_or(u64::MAX)
            > crate::jpeg_reconstruction::MAX_REGION_CHUNKS
        {
            insert_v6_audit(
                archive,
                target,
                ReconstructionAuditReason::ResourcePolicyExcluded,
            );
            continue;
        }
        if object.chunks.iter().any(|chunk_ref| {
            object_owners
                .get(&chunk_ref.chunk_id)
                .is_some_and(|owners| owners.len() != 1)
        }) || object.chunks.iter().any(|chunk_ref| {
            let chunk = &archive.content_store.chunks[&chunk_ref.chunk_id];
            chunk.group_ref.is_some()
                || plans
                    .get(&chunk.plan_ref)
                    .is_none_or(|plan| plan.dictionary.is_some())
        }) {
            insert_v6_audit(
                archive,
                target,
                ReconstructionAuditReason::RegionDedupConflict,
            );
            continue;
        }

        let logical_bytes = object.chunks.iter().try_fold(0_u64, |total, chunk_ref| {
            total
                .checked_add(archive.content_store.chunks[&chunk_ref.chunk_id].logical_len)
                .ok_or_else(|| resource("JPEG ContentObject length overflows"))
        })?;
        if logical_bytes
            > u64::try_from(crate::jpeg_reconstruction::MAX_JPEG_BYTES).unwrap_or(u64::MAX)
        {
            insert_v6_audit(
                archive,
                target,
                ReconstructionAuditReason::ResourcePolicyExcluded,
            );
            continue;
        }
        let mut original = Vec::with_capacity(
            usize::try_from(logical_bytes)
                .map_err(|_| resource("JPEG ContentObject length exceeds usize"))?,
        );
        for chunk_ref in &object.chunks {
            original
                .extend_from_slice(&archive.content_store.chunks[&chunk_ref.chunk_id].plaintext);
        }
        let verified = match crate::jpeg_reconstruction::verified_forward(&original) {
            Ok(value) => value,
            Err(failure) => {
                let reason = match failure {
                    crate::jpeg_reconstruction::JpegAttemptFailure::NotRecognized => {
                        ReconstructionAuditReason::NotRecognized
                    }
                    crate::jpeg_reconstruction::JpegAttemptFailure::Unsupported => {
                        ReconstructionAuditReason::Unsupported
                    }
                    crate::jpeg_reconstruction::JpegAttemptFailure::VerificationFailed => {
                        ReconstructionAuditReason::ExactVerificationFailed
                    }
                    crate::jpeg_reconstruction::JpegAttemptFailure::ResourceExcluded => {
                        ReconstructionAuditReason::ResourcePolicyExcluded
                    }
                };
                insert_v6_audit(archive, target, reason);
                continue;
            }
        };

        let ordinary_cost = v6_ordinary_payload_cost(archive, &object, &plans)?;
        let mut best: Option<(TransformPlan, Vec<u8>, u64)> = None;
        for plan in jpeg_region_candidates(profile)? {
            let representation = encode_transformed_payload(&plan, &verified.bytes)?;
            let plan_cost = if plans.contains_key(&plan.plan_id) {
                0
            } else {
                encoded_transform_plan_v3_len(&plan)?
            };
            let provisional = ReconstructionRegion {
                region_id: Digest::ZERO,
                content_object: object.logical_digest,
                start_chunk_index: 0,
                chunk_count: u64::try_from(object.chunks.len())
                    .map_err(|_| resource("JPEG region Chunk count exceeds u64"))?,
                plan_ref: plan.plan_id,
                logical_bytes,
                transformed_bytes: u64::try_from(verified.bytes.len())
                    .map_err(|_| resource("JPEG XL representation exceeds u64"))?,
                ordinary_physical_bytes: ordinary_cost,
                region_overhead_bytes: 0,
                access: RegionAccessCost {
                    logical_bytes,
                    logical_chunks: u64::try_from(object.chunks.len())
                        .map_err(|_| resource("JPEG region Chunk count exceeds u64"))?,
                    worst_reconstructed_bytes: logical_bytes,
                },
                representation: representation.clone().into_boxed_slice(),
            };
            let representation_len = u64::try_from(representation.len())
                .map_err(|_| resource("JPEG region representation exceeds u64"))?;
            let record_cost = encoded_reconstruction_region_len(&provisional)?
                .checked_sub(representation_len)
                .ok_or_else(|| resource("JPEG region record cost underflows"))?;
            let overhead = plan_cost
                .checked_add(record_cost)
                .and_then(|value| {
                    value.checked_add(if emitted_region_section {
                        0
                    } else {
                        SECTION_HEADER_LEN
                    })
                })
                .ok_or_else(|| resource("JPEG region overhead overflows"))?;
            let complete_cost = representation_len
                .checked_add(overhead)
                .ok_or_else(|| resource("JPEG region cost overflows"))?;
            if best
                .as_ref()
                .is_none_or(|(_, _, previous)| complete_cost < *previous)
            {
                best = Some((plan, representation, complete_cost));
            }
        }
        let Some((plan, representation, complete_cost)) = best else {
            insert_v6_audit(archive, target, ReconstructionAuditReason::Unsupported);
            continue;
        };
        if !qualifies_reconstruction(complete_cost, ordinary_cost)? {
            insert_v6_audit(
                archive,
                target,
                ReconstructionAuditReason::CompleteCostDidNotWin,
            );
            increment(
                &mut report.reconstruction_cost_rejections,
                "JPEG reconstruction cost rejection",
            )?;
            continue;
        }

        let representation_len = u64::try_from(representation.len())
            .map_err(|_| resource("JPEG region representation exceeds u64"))?;
        let region_overhead_bytes = complete_cost
            .checked_sub(representation_len)
            .ok_or_else(|| resource("JPEG region cost underflows"))?;
        let chunk_count = u64::try_from(object.chunks.len())
            .map_err(|_| resource("JPEG region Chunk count exceeds u64"))?;
        let transformed_bytes = u64::try_from(verified.bytes.len())
            .map_err(|_| resource("JPEG XL representation exceeds u64"))?;
        let mut region = ReconstructionRegion {
            region_id: Digest::ZERO,
            content_object: object.logical_digest,
            start_chunk_index: 0,
            chunk_count,
            plan_ref: plan.plan_id,
            logical_bytes,
            transformed_bytes,
            ordinary_physical_bytes: ordinary_cost,
            region_overhead_bytes,
            access: RegionAccessCost {
                logical_bytes,
                logical_chunks: chunk_count,
                worst_reconstructed_bytes: logical_bytes,
            },
            representation: representation.into_boxed_slice(),
        };
        region.region_id = crate::jpeg_reconstruction::region_identity(&region);
        let region_id = region.region_id;
        plans.entry(plan.plan_id).or_insert(plan);
        for chunk_ref in &object.chunks {
            let chunk = archive
                .content_store
                .chunks
                .get_mut(&chunk_ref.chunk_id)
                .expect("validated ContentObject Chunk");
            chunk.plan_ref = crate::jpeg_reconstruction::REGION_MEMBER_PLAN_REF;
            chunk.group_ref = None;
            archive
                .content_store
                .reconstruction_fallbacks
                .remove(&chunk_ref.chunk_id);
        }
        archive
            .content_store
            .reconstruction_regions
            .insert(region_id, region);
        emitted_region_section = true;
    }

    if !archive.content_store.reconstruction_regions.is_empty()
        || !archive.content_store.reconstruction_audits.is_empty()
    {
        archive.descriptor.features.incompat |= FEATURE_CROSS_FILE_COMPRESSION_V1
            | FEATURE_CODEC_TRANSFORM_V1
            | FEATURE_RECONSTRUCTIVE_TRANSFORM_V1
            | FEATURE_WHOLE_OBJECT_RECONSTRUCTION_V1;
    }
    archive.transform_plans = plans.into_values().collect::<Vec<_>>().into_boxed_slice();
    archive.descriptor.decode = aggregate_archive_decode_requirements(
        &archive.transform_plans,
        &archive.content_store.dictionaries,
        &archive.content_store.chunk_groups,
    )?;
    report.planner_id = profile.planner_id().to_owned();
    report.selected_plans = archive.transform_plans.clone();
    report.reconstructive_chunks = archive
        .content_store
        .reconstruction_regions
        .values()
        .try_fold(0_u64, |total, region| {
            total
                .checked_add(region.chunk_count)
                .ok_or_else(|| resource("JPEG reconstructive Chunk count overflows"))
        })?;
    Ok(report)
}

fn chunk_object_owners(archive: &Archive) -> BTreeMap<Digest, BTreeSet<Digest>> {
    let mut owners = BTreeMap::<Digest, BTreeSet<Digest>>::new();
    for object in archive.content_store.objects.values() {
        for chunk_ref in &object.chunks {
            owners
                .entry(chunk_ref.chunk_id)
                .or_default()
                .insert(object.logical_digest);
        }
    }
    owners
}

fn insert_v6_audit(
    archive: &mut Archive,
    target: ReconstructionAuditTarget,
    reason: ReconstructionAuditReason,
) {
    archive.content_store.reconstruction_audits.insert(
        target,
        ReconstructionAudit {
            target,
            transform_id: crate::jpeg_reconstruction::JPEG_RECONSTRUCT_ID.to_owned(),
            reason,
        },
    );
}

fn v6_ordinary_payload_cost(
    archive: &Archive,
    object: &crate::eam::ContentObject,
    plans: &BTreeMap<u64, TransformPlan>,
) -> Result<u64> {
    let mut total = 0_u64;
    let mut unique = BTreeSet::new();
    for chunk_ref in &object.chunks {
        if !unique.insert(chunk_ref.chunk_id) {
            continue;
        }
        let chunk = &archive.content_store.chunks[&chunk_ref.chunk_id];
        let plan = plans.get(&chunk.plan_ref).ok_or_else(|| {
            Diagnostic::new(
                OutcomeClass::Unsupported,
                ReasonCode::UnknownTransformPlan,
                chunk.plan_ref.to_string(),
            )
        })?;
        let payload = if plan
            .transforms
            .iter()
            .any(|step| step.reconstruction_ref.is_some())
        {
            encode_payload_with_reconstruction(
                plan,
                &chunk.plaintext,
                &archive.content_store.reconstruction_data,
            )?
        } else {
            encode_payload(plan, &chunk.plaintext)?
        };
        total = total
            .checked_add(
                u64::try_from(payload.len())
                    .map_err(|_| resource("ordinary JPEG payload exceeds u64"))?,
            )
            .ok_or_else(|| resource("ordinary JPEG representation cost overflows"))?;
    }
    Ok(total)
}

fn jpeg_region_candidates(profile: CompressionProfile) -> Result<Vec<TransformPlan>> {
    let transform = jpeg_reconstruct_step()?;
    let mut bases = vec![store_plan()];
    match profile {
        CompressionProfile::Fast => {}
        CompressionProfile::Balanced => {
            bases.push(lz4_plan(Box::default())?);
            bases.push(zstd_plan(3)?);
            bases.push(zstd_plan(5)?);
        }
        CompressionProfile::Dense => {
            bases.push(zstd_plan(9)?);
            bases.push(zstd_plan(15)?);
            bases.push(lzma2_plan(6, 4 * 1024 * 1024, Box::default())?);
        }
        CompressionProfile::Extreme => {
            bases.push(lz4_plan(Box::default())?);
            bases.push(zstd_plan(15)?);
            bases.push(zstd_plan(19)?);
            bases.push(lzma2_plan(6, 4 * 1024 * 1024, Box::default())?);
            bases.push(lzma2_plan(9, 8 * 1024 * 1024, Box::default())?);
        }
    }
    bases
        .into_iter()
        .map(|base| with_pipeline(base, vec![transform.clone()].into_boxed_slice()))
        .collect()
}

fn finish_cross_file_planning(
    archive: &mut Archive,
    profile: CompressionProfile,
    planner_id: &str,
    required_features: u64,
    mut report: PlanningReport,
) -> Result<PlanningReport> {
    archive.descriptor.features.incompat |= required_features;
    archive.content_store.dictionaries.clear();
    archive.content_store.chunk_groups.clear();
    for chunk in archive.content_store.chunks.values_mut() {
        chunk.group_ref = None;
    }

    let mut plans = archive
        .transform_plans
        .iter()
        .cloned()
        .map(|plan| (plan.plan_id, plan))
        .collect::<BTreeMap<_, _>>();
    let mut cohorts = cluster(&archive.content_store.chunks, profile.similarity_policy());
    cohorts.sort_by_key(|cohort| cohort.cohort_id);
    let mut physically_ordered = std::collections::BTreeSet::new();
    let mut physical_order = Vec::with_capacity(archive.content_store.chunks.len());
    let mut dictionary_chunks = 0_u64;
    let mut lookback_chunks = 0_u64;

    for cohort in &cohorts {
        match select_cohort_plan(archive, cohort, profile, planner_id, &plans)? {
            CohortSelection::Independent => {}
            CohortSelection::Dictionary { dictionary, plan } => {
                insert_dictionary(&mut archive.content_store.dictionaries, dictionary)?;
                insert_plan(&mut plans, plan.clone())?;
                for chunk_id in &cohort.chunks {
                    archive
                        .content_store
                        .chunks
                        .get_mut(chunk_id)
                        .ok_or_else(|| {
                            Diagnostic::new(
                                OutcomeClass::Nonconforming,
                                ReasonCode::UnknownChunk,
                                chunk_id.to_string(),
                            )
                        })?
                        .plan_ref = plan.plan_id;
                    dictionary_chunks = dictionary_chunks
                        .checked_add(1)
                        .ok_or_else(|| resource("dictionary Chunk count exceeds u64"))?;
                }
            }
            CohortSelection::Lookback { group, plan } => {
                insert_plan(&mut plans, plan.clone())?;
                for chunk_id in &cohort.chunks {
                    let chunk =
                        archive
                            .content_store
                            .chunks
                            .get_mut(chunk_id)
                            .ok_or_else(|| {
                                Diagnostic::new(
                                    OutcomeClass::Nonconforming,
                                    ReasonCode::UnknownChunk,
                                    chunk_id.to_string(),
                                )
                            })?;
                    chunk.plan_ref = plan.plan_id;
                    chunk.group_ref = Some(group.group_id);
                    lookback_chunks = lookback_chunks
                        .checked_add(1)
                        .ok_or_else(|| resource("lookback Chunk count exceeds u64"))?;
                }
                archive
                    .content_store
                    .chunk_groups
                    .insert(group.group_id, group);
            }
        }
        for chunk_id in &cohort.chunks {
            if physically_ordered.insert(*chunk_id) {
                physical_order.push(*chunk_id);
            }
        }
    }
    for chunk_id in archive.content_store.chunks.keys() {
        if physically_ordered.insert(*chunk_id) {
            physical_order.push(*chunk_id);
        }
    }
    archive.content_store.physical_order = physical_order.into_boxed_slice();
    if planner_id.ends_with("-v5") {
        for chunk in archive.content_store.chunks.values() {
            let reconstructive = plans[&chunk.plan_ref]
                .transforms
                .iter()
                .any(|step| step.reconstruction_ref.is_some());
            if reconstructive {
                archive
                    .content_store
                    .reconstruction_fallbacks
                    .remove(&chunk.chunk_id);
            } else if profile != CompressionProfile::Fast {
                archive
                    .content_store
                    .reconstruction_fallbacks
                    .entry(chunk.chunk_id)
                    .or_insert(ReconstructionFallbackReason::CompleteCostDidNotWin);
            }
        }
        let used_plans = archive
            .content_store
            .chunks
            .values()
            .map(|chunk| chunk.plan_ref)
            .collect::<std::collections::BTreeSet<_>>();
        plans.retain(|plan_id, plan| {
            used_plans.contains(plan_id)
                || !plan
                    .transforms
                    .iter()
                    .any(|step| step.reconstruction_ref.is_some())
        });
        let used_reconstruction = plans
            .values()
            .flat_map(|plan| plan.transforms.iter())
            .filter_map(|step| step.reconstruction_ref)
            .collect::<std::collections::BTreeSet<_>>();
        archive
            .content_store
            .reconstruction_data
            .retain(|identity, _| used_reconstruction.contains(identity));
        if archive.content_store.reconstruction_data.is_empty()
            && archive.content_store.reconstruction_fallbacks.is_empty()
        {
            archive.descriptor.features.incompat &= !FEATURE_RECONSTRUCTIVE_TRANSFORM_V1;
        } else {
            archive.descriptor.features.incompat |= FEATURE_RECONSTRUCTIVE_TRANSFORM_V1;
        }
    }
    archive.transform_plans = plans.into_values().collect::<Vec<_>>().into_boxed_slice();
    archive.descriptor.decode = aggregate_archive_decode_requirements(
        &archive.transform_plans,
        &archive.content_store.dictionaries,
        &archive.content_store.chunk_groups,
    )?;
    report.dictionary_chunks = dictionary_chunks;
    report.lookback_chunks = lookback_chunks;
    report.store_chunks = 0;
    report.zstandard_chunks = 0;
    report.lz4_chunks = 0;
    report.lzma2_chunks = 0;
    report.transformed_chunks = 0;
    report.reconstructive_chunks = 0;
    let selected_by_id = archive
        .transform_plans
        .iter()
        .map(|plan| (plan.plan_id, plan))
        .collect::<BTreeMap<_, _>>();
    for chunk in archive.content_store.chunks.values() {
        let plan = selected_by_id.get(&chunk.plan_ref).ok_or_else(|| {
            Diagnostic::new(
                OutcomeClass::Unsupported,
                ReasonCode::UnknownTransformPlan,
                chunk.plan_ref.to_string(),
            )
        })?;
        match plan.codec.as_str() {
            crate::identity::STORE_CODEC_IDENTIFIER => {
                increment(&mut report.store_chunks, "STORE")?
            }
            ZSTD_CODEC_IDENTIFIER => increment(&mut report.zstandard_chunks, "Zstandard")?,
            LZ4_CODEC_IDENTIFIER => increment(&mut report.lz4_chunks, "LZ4")?,
            LZMA2_CODEC_IDENTIFIER => increment(&mut report.lzma2_chunks, "LZMA2")?,
            codec => {
                return Err(Diagnostic::new(
                    OutcomeClass::Unsupported,
                    ReasonCode::UnknownCodec,
                    codec.to_owned(),
                ));
            }
        }
        if !plan.transforms.is_empty() {
            increment(&mut report.transformed_chunks, "transformed")?;
        }
        if plan
            .transforms
            .iter()
            .any(|step| step.reconstruction_ref.is_some())
        {
            increment(&mut report.reconstructive_chunks, "reconstructive")?;
        }
    }
    report.similarity_cohorts = u64::try_from(cohorts.len())
        .map_err(|_| resource("similarity cohort count exceeds u64"))?;
    report.selected_plans = archive.transform_plans.clone();
    Ok(report)
}

fn plan_archive_with_id(
    archive: &mut Archive,
    profile: CompressionProfile,
    planner_id: &str,
) -> Result<PlanningReport> {
    let mut plans = BTreeMap::new();
    let store = store_plan();
    plans.insert(store.plan_id, store);
    let mut store_chunks = 0_u64;
    let mut zstandard_chunks = 0_u64;

    for chunk in archive.content_store.chunks.values_mut() {
        if chunk.logical_len != u64::try_from(chunk.plaintext.len()).unwrap_or(u64::MAX)
            || sha256_exact(&chunk.plaintext) != chunk.chunk_id
        {
            return Err(Diagnostic::new(
                OutcomeClass::Corrupt,
                ReasonCode::ChunkDigestMismatch,
                chunk.chunk_id.to_string(),
            ));
        }
        let analysis = analyze(&chunk.plaintext);
        let mut selected: Option<(TransformPlan, usize)> = None;
        if chunk.plaintext.len() >= MINIMUM_ZSTD_INPUT_BYTES {
            for level in profile.levels(analysis) {
                let plan = zstd_plan(*level)?;
                let encoded = encode_payload(&plan, &chunk.plaintext)?;
                if zstandard_wins(chunk.logical_len, encoded.len())?
                    && selected
                        .as_ref()
                        .is_none_or(|(_, previous_len)| encoded.len() < *previous_len)
                {
                    selected = Some((plan, encoded.len()));
                }
            }
        }
        if let Some((plan, _)) = selected {
            chunk.plan_ref = plan.plan_id;
            plans.entry(plan.plan_id).or_insert(plan);
            zstandard_chunks = zstandard_chunks
                .checked_add(1)
                .ok_or_else(|| resource("Zstandard Chunk count exceeds u64"))?;
        } else {
            chunk.plan_ref = crate::identity::STORE_PLAN_ID;
            store_chunks = store_chunks
                .checked_add(1)
                .ok_or_else(|| resource("STORE Chunk count exceeds u64"))?;
        }
    }

    let selected_plans = plans.into_values().collect::<Vec<_>>().into_boxed_slice();
    archive.transform_plans = selected_plans.clone();
    archive.descriptor.planner_id = planner_id.to_owned();
    archive.descriptor.decode = aggregate_decode_requirements(&archive.transform_plans);
    Ok(PlanningReport {
        planner_id: planner_id.to_owned(),
        store_chunks,
        zstandard_chunks,
        lz4_chunks: 0,
        lzma2_chunks: 0,
        transformed_chunks: 0,
        dictionary_chunks: 0,
        lookback_chunks: 0,
        similarity_cohorts: 0,
        reconstructive_chunks: 0,
        reconstruction_attempts: 0,
        reconstruction_cost_rejections: 0,
        selected_plans,
    })
}

fn plan_archive_v4_independent(
    archive: &mut Archive,
    profile: CompressionProfile,
    planner_id: &str,
) -> Result<PlanningReport> {
    let mut plans = BTreeMap::new();
    let store = store_plan();
    plans.insert(store.plan_id, store);
    let mut store_chunks = 0_u64;
    let mut zstandard_chunks = 0_u64;
    let mut lz4_chunks = 0_u64;
    let mut lzma2_chunks = 0_u64;
    let mut transformed_chunks = 0_u64;

    for chunk in archive.content_store.chunks.values_mut() {
        if chunk.logical_len != u64::try_from(chunk.plaintext.len()).unwrap_or(u64::MAX)
            || sha256_exact(&chunk.plaintext) != chunk.chunk_id
        {
            return Err(Diagnostic::new(
                OutcomeClass::Corrupt,
                ReasonCode::ChunkDigestMismatch,
                chunk.chunk_id.to_string(),
            ));
        }
        let selected_plan = select_v4_plan(profile, &chunk.plaintext)?;
        chunk.plan_ref = selected_plan.plan_id;
        match selected_plan.codec.as_str() {
            crate::identity::STORE_CODEC_IDENTIFIER => increment(&mut store_chunks, "STORE")?,
            ZSTD_CODEC_IDENTIFIER => increment(&mut zstandard_chunks, "Zstandard")?,
            LZ4_CODEC_IDENTIFIER => increment(&mut lz4_chunks, "LZ4")?,
            LZMA2_CODEC_IDENTIFIER => increment(&mut lzma2_chunks, "LZMA2")?,
            _ => {
                return Err(Diagnostic::new(
                    OutcomeClass::Unsupported,
                    ReasonCode::UnknownCodec,
                    selected_plan.codec.clone(),
                ));
            }
        }
        if !selected_plan.transforms.is_empty() {
            increment(&mut transformed_chunks, "transformed")?;
        }
        insert_plan(&mut plans, selected_plan)?;
    }

    archive.transform_plans = plans.into_values().collect::<Vec<_>>().into_boxed_slice();
    archive.descriptor.planner_id = planner_id.to_owned();
    archive.descriptor.decode = aggregate_decode_requirements(&archive.transform_plans);
    Ok(PlanningReport {
        planner_id: planner_id.to_owned(),
        store_chunks,
        zstandard_chunks,
        lz4_chunks,
        lzma2_chunks,
        transformed_chunks,
        dictionary_chunks: 0,
        lookback_chunks: 0,
        similarity_cohorts: 0,
        reconstructive_chunks: 0,
        reconstruction_attempts: 0,
        reconstruction_cost_rejections: 0,
        selected_plans: archive.transform_plans.clone(),
    })
}

fn plan_archive_v5_independent(
    archive: &mut Archive,
    profile: CompressionProfile,
    planner_id: &str,
) -> Result<PlanningReport> {
    archive.content_store.reconstruction_data.clear();
    archive.content_store.reconstruction_fallbacks.clear();
    let mut plans = BTreeMap::new();
    let store = store_plan();
    plans.insert(store.plan_id, store);
    let mut report = PlanningReport {
        planner_id: planner_id.to_owned(),
        store_chunks: 0,
        zstandard_chunks: 0,
        lz4_chunks: 0,
        lzma2_chunks: 0,
        transformed_chunks: 0,
        dictionary_chunks: 0,
        lookback_chunks: 0,
        similarity_cohorts: 0,
        reconstructive_chunks: 0,
        reconstruction_attempts: 0,
        reconstruction_cost_rejections: 0,
        selected_plans: Box::default(),
    };
    let chunk_ids = archive
        .content_store
        .chunks
        .keys()
        .copied()
        .collect::<Vec<_>>();
    for chunk_id in chunk_ids {
        let plaintext = &archive.content_store.chunks[&chunk_id].plaintext;
        if sha256_exact(plaintext) != chunk_id {
            return Err(Diagnostic::new(
                OutcomeClass::Corrupt,
                ReasonCode::ChunkDigestMismatch,
                chunk_id.to_string(),
            ));
        }
        let ordinary = select_v4_plan(profile, plaintext)?;
        let mut selected = ordinary.clone();
        let mut selected_data = None;
        if profile != CompressionProfile::Fast {
            increment(
                &mut report.reconstruction_attempts,
                "reconstruction attempt",
            )?;
            if let Some(candidate) =
                crate::reconstruction::try_forward(plaintext, reconstruction_max_chain(profile))?
            {
                let baseline = v5_independent_cost(&ordinary, plaintext, None)?;
                let data_map =
                    BTreeMap::from([(candidate.data.reconstruction_id, candidate.data.clone())]);
                let mut best_cost = baseline;
                for plan in reconstruction_candidates(profile, &candidate.data)? {
                    let cost =
                        v5_independent_cost(&plan, plaintext, Some((&candidate.data, &data_map)))?;
                    if qualifies_reconstruction(cost, baseline)? && cost < best_cost {
                        best_cost = cost;
                        selected = plan;
                        selected_data = Some(candidate.data.clone());
                    }
                }
                if selected_data.is_none() {
                    archive.content_store.reconstruction_fallbacks.insert(
                        chunk_id,
                        ReconstructionFallbackReason::CompleteCostDidNotWin,
                    );
                    increment(
                        &mut report.reconstruction_cost_rejections,
                        "reconstruction cost rejection",
                    )?;
                }
            } else {
                archive.content_store.reconstruction_fallbacks.insert(
                    chunk_id,
                    ReconstructionFallbackReason::UnrecognizedOrVerificationFailed,
                );
            }
        }
        if let Some(data) = selected_data {
            archive
                .content_store
                .reconstruction_data
                .entry(data.reconstruction_id)
                .or_insert(data);
            increment(&mut report.reconstructive_chunks, "reconstructive")?;
        }
        archive
            .content_store
            .chunks
            .get_mut(&chunk_id)
            .expect("known Chunk")
            .plan_ref = selected.plan_id;
        match selected.codec.as_str() {
            crate::identity::STORE_CODEC_IDENTIFIER => {
                increment(&mut report.store_chunks, "STORE")?
            }
            ZSTD_CODEC_IDENTIFIER => increment(&mut report.zstandard_chunks, "Zstandard")?,
            LZ4_CODEC_IDENTIFIER => increment(&mut report.lz4_chunks, "LZ4")?,
            LZMA2_CODEC_IDENTIFIER => increment(&mut report.lzma2_chunks, "LZMA2")?,
            codec => {
                return Err(Diagnostic::new(
                    OutcomeClass::Unsupported,
                    ReasonCode::UnknownCodec,
                    codec.to_owned(),
                ));
            }
        }
        if !selected.transforms.is_empty() {
            increment(&mut report.transformed_chunks, "transformed")?;
        }
        insert_plan(&mut plans, selected)?;
    }
    archive.transform_plans = plans.into_values().collect::<Vec<_>>().into_boxed_slice();
    archive.descriptor.planner_id = planner_id.to_owned();
    archive.descriptor.decode = aggregate_decode_requirements(&archive.transform_plans);
    report.selected_plans = archive.transform_plans.clone();
    Ok(report)
}

fn reconstruction_max_chain(profile: CompressionProfile) -> u32 {
    match profile {
        CompressionProfile::Fast => 512,
        CompressionProfile::Balanced => 512,
        CompressionProfile::Dense => 2_048,
        CompressionProfile::Extreme => 4_096,
    }
}

fn reconstruction_candidates(
    profile: CompressionProfile,
    data: &ReconstructionData,
) -> Result<Vec<TransformPlan>> {
    let reconstruct =
        deflate_reconstruct_step(reconstruction_max_chain(profile), data.reconstruction_id)?;
    let mut candidates = Vec::new();
    match profile {
        CompressionProfile::Fast => {}
        CompressionProfile::Balanced => {
            for level in [3, 5] {
                candidates.push(zstd_transformed_plan(
                    level,
                    vec![reconstruct.clone()].into(),
                )?);
            }
        }
        CompressionProfile::Dense => {
            candidates.push(zstd_transformed_plan(9, vec![reconstruct.clone()].into())?);
            candidates.push(lzma2_plan(
                6,
                4 * 1024 * 1024,
                vec![reconstruct.clone()].into(),
            )?);
            candidates.push(zstd_transformed_plan(
                9,
                vec![reconstruct.clone(), delta8_step()].into(),
            )?);
        }
        CompressionProfile::Extreme => {
            for level in [15, 19] {
                candidates.push(zstd_transformed_plan(
                    level,
                    vec![reconstruct.clone()].into(),
                )?);
            }
            candidates.push(lzma2_plan(
                9,
                8 * 1024 * 1024,
                vec![reconstruct.clone()].into(),
            )?);
            candidates.push(zstd_transformed_plan(
                15,
                vec![reconstruct.clone(), delta8_step()].into(),
            )?);
            for width in [2, 4, 8] {
                candidates.push(lzma2_plan(
                    9,
                    8 * 1024 * 1024,
                    vec![reconstruct.clone(), byte_shuffle_step(width)?].into(),
                )?);
            }
        }
    }
    Ok(candidates)
}

fn v5_independent_cost(
    plan: &TransformPlan,
    plaintext: &[u8],
    reconstruction: Option<(&ReconstructionData, &BTreeMap<Digest, ReconstructionData>)>,
) -> Result<u64> {
    let payload = match reconstruction {
        Some((_, values)) => encode_payload_with_reconstruction(plan, plaintext, values)?,
        None => encode_payload(plan, plaintext)?,
    };
    let mut cost = u64::try_from(payload.len())
        .map_err(|_| resource("v5 candidate payload exceeds u64"))?
        .checked_add(encoded_transform_plan_v2_len(plan)?)
        .ok_or_else(|| resource("v5 candidate cost overflow"))?;
    if let Some((data, _)) = reconstruction {
        cost = cost
            .checked_add(encoded_reconstruction_data_len(data)?)
            .and_then(|value| value.checked_add(RECONSTRUCTION_SECTION_OVERHEAD_BYTES))
            .ok_or_else(|| resource("v5 ReconstructionData cost overflow"))?;
    }
    Ok(cost)
}

fn qualifies_reconstruction(candidate: u64, baseline: u64) -> Result<bool> {
    let relative = baseline
        .checked_mul(MINIMUM_RECONSTRUCTION_GAIN_BASIS_POINTS)
        .and_then(|value| value.checked_add(9_999))
        .map(|value| value / 10_000)
        .ok_or_else(|| resource("v5 reconstruction margin overflow"))?;
    candidate
        .checked_add(MINIMUM_RECONSTRUCTION_GAIN_BYTES.max(relative))
        .map(|value| value < baseline)
        .ok_or_else(|| resource("v5 reconstruction candidate cost overflow"))
}

fn v4_candidates(
    profile: CompressionProfile,
    plaintext: &[u8],
) -> Result<(Vec<TransformPlan>, Vec<TransformPlan>)> {
    let mut ordinary = vec![store_plan()];
    let mut transformed = Vec::new();
    match profile {
        CompressionProfile::Fast => {
            ordinary.push(lz4_plan(Box::default())?);
            ordinary.push(zstd_plan(1)?);
        }
        CompressionProfile::Balanced => {
            ordinary.push(lz4_plan(Box::default())?);
            for level in [1, 3, 5] {
                ordinary.push(zstd_plan(level)?);
            }
            if analyze(plaintext).likely_compressible {
                transformed.push(zstd_transformed_plan(3, vec![delta8_step()].into())?);
                transformed.push(zstd_transformed_plan(
                    3,
                    vec![byte_shuffle_step(4)?].into(),
                )?);
            }
        }
        CompressionProfile::Dense => {
            for level in [5, 9, 15] {
                ordinary.push(zstd_plan(level)?);
            }
            ordinary.push(lzma2_plan(4, 1024 * 1024, Box::default())?);
            ordinary.push(lzma2_plan(6, 4 * 1024 * 1024, Box::default())?);
            if analyze(plaintext).likely_compressible {
                append_dense_transform_candidates(&mut transformed, false)?;
            }
        }
        CompressionProfile::Extreme => {
            ordinary.push(lz4_plan(Box::default())?);
            for level in [9, 15, 19, 22] {
                ordinary.push(zstd_plan(level)?);
            }
            ordinary.push(lzma2_plan(4, 1024 * 1024, Box::default())?);
            ordinary.push(lzma2_plan(6, 4 * 1024 * 1024, Box::default())?);
            ordinary.push(lzma2_plan(9, 8 * 1024 * 1024, Box::default())?);
            if analyze(plaintext).likely_compressible {
                append_dense_transform_candidates(&mut transformed, true)?;
            }
        }
    }
    Ok((ordinary, transformed))
}

fn select_v4_plan(profile: CompressionProfile, plaintext: &[u8]) -> Result<TransformPlan> {
    let (ordinary, transformed) = v4_candidates(profile, plaintext)?;
    let store_cost = complete_candidate_cost(&ordinary[0], plaintext)?;
    let mut selected = (ordinary[0].clone(), store_cost);
    for plan in ordinary.iter().skip(1) {
        let cost = complete_candidate_cost(plan, plaintext)?;
        if qualifies_v4(cost, store_cost)? && cost < selected.1 {
            selected = (plan.clone(), cost);
        }
    }
    let best_ordinary = selected.1;
    for plan in transformed {
        let cost = complete_candidate_cost(&plan, plaintext)?;
        if qualifies_v4(cost, best_ordinary)? && cost < selected.1 {
            selected = (plan, cost);
        }
    }
    Ok(selected.0)
}

fn append_dense_transform_candidates(
    candidates: &mut Vec<TransformPlan>,
    extreme: bool,
) -> Result<()> {
    let transforms = [
        delta8_step(),
        byte_shuffle_step(2)?,
        byte_shuffle_step(4)?,
        byte_shuffle_step(8)?,
    ];
    for transform in transforms {
        let steps = vec![transform].into_boxed_slice();
        candidates.push(zstd_transformed_plan(
            if extreme { 15 } else { 9 },
            steps.clone(),
        )?);
        candidates.push(lzma2_plan(6, 4 * 1024 * 1024, steps.clone())?);
        if extreme {
            candidates.push(lzma2_plan(9, 8 * 1024 * 1024, steps)?);
        }
    }
    Ok(())
}

fn complete_candidate_cost(plan: &TransformPlan, plaintext: &[u8]) -> Result<u64> {
    let encoded = encode_payload(plan, plaintext)?;
    u64::try_from(encoded.len())
        .map_err(|_| resource("v4 candidate payload length exceeds u64"))?
        .checked_add(encoded_plan_cost(plan)?)
        .ok_or_else(|| resource("v4 complete candidate cost exceeds u64"))
}

fn qualifies_v4(candidate: u64, baseline: u64) -> Result<bool> {
    let relative = baseline
        .checked_mul(MINIMUM_GAIN_BASIS_POINTS)
        .and_then(|value| value.checked_add(9_999))
        .map(|value| value / 10_000)
        .ok_or_else(|| resource("v4 minimum-gain calculation overflow"))?;
    candidate
        .checked_add(MINIMUM_GAIN_BYTES.max(relative))
        .map(|cost| cost < baseline)
        .ok_or_else(|| resource("v4 candidate cost overflow"))
}

fn increment(value: &mut u64, label: &str) -> Result<()> {
    *value = value
        .checked_add(1)
        .ok_or_else(|| resource(format!("{label} Chunk count exceeds u64")))?;
    Ok(())
}

enum CohortSelection {
    Independent,
    Dictionary {
        dictionary: Dictionary,
        plan: TransformPlan,
    },
    Lookback {
        group: ChunkGroup,
        plan: TransformPlan,
    },
}

fn select_cohort_plan(
    archive: &Archive,
    cohort: &SimilarityCohort,
    profile: CompressionProfile,
    planner_id: &str,
    plans: &BTreeMap<u64, TransformPlan>,
) -> Result<CohortSelection> {
    let mut independent_cost = cohort.chunks.iter().try_fold(0_u64, |total, chunk_id| {
        let chunk = &archive.content_store.chunks[chunk_id];
        let plan = plans.get(&chunk.plan_ref).ok_or_else(|| {
            Diagnostic::new(
                OutcomeClass::Unsupported,
                ReasonCode::UnknownTransformPlan,
                chunk.plan_ref.to_string(),
            )
        })?;
        let encoded = if plan
            .transforms
            .iter()
            .any(|step| step.reconstruction_ref.is_some())
        {
            encode_payload_with_reconstruction(
                plan,
                &chunk.plaintext,
                &archive.content_store.reconstruction_data,
            )?
        } else {
            encode_payload(plan, &chunk.plaintext)?
        };
        total
            .checked_add(
                u64::try_from(encoded.len())
                    .map_err(|_| resource("independent cohort payload length exceeds u64"))?,
            )
            .ok_or_else(|| resource("independent cohort cost exceeds u64"))
    })?;
    if planner_id.ends_with("-v4") || planner_id.ends_with("-v5") {
        let plan_ids = cohort
            .chunks
            .iter()
            .map(|chunk_id| archive.content_store.chunks[chunk_id].plan_ref)
            .collect::<std::collections::BTreeSet<_>>();
        for plan_id in plan_ids {
            independent_cost = independent_cost
                .checked_add(if planner_id.ends_with("-v5") {
                    encoded_transform_plan_v2_len(plans.get(&plan_id).ok_or_else(|| {
                        Diagnostic::new(
                            OutcomeClass::Unsupported,
                            ReasonCode::UnknownTransformPlan,
                            plan_id.to_string(),
                        )
                    })?)?
                } else {
                    encoded_plan_cost(plans.get(&plan_id).ok_or_else(|| {
                        Diagnostic::new(
                            OutcomeClass::Unsupported,
                            ReasonCode::UnknownTransformPlan,
                            plan_id.to_string(),
                        )
                    })?)?
                })
                .ok_or_else(|| resource("v4 independent cohort complete cost exceeds u64"))?;
        }
        if planner_id.ends_with("-v5") {
            let references = cohort
                .chunks
                .iter()
                .filter_map(|chunk_id| {
                    let plan = plans.get(&archive.content_store.chunks[chunk_id].plan_ref)?;
                    plan.transforms
                        .iter()
                        .find_map(|step| step.reconstruction_ref)
                })
                .collect::<std::collections::BTreeSet<_>>();
            for reference in references {
                let data = archive
                    .content_store
                    .reconstruction_data
                    .get(&reference)
                    .ok_or_else(|| {
                        Diagnostic::new(
                            OutcomeClass::Nonconforming,
                            ReasonCode::UnknownReconstructionData,
                            reference.to_string(),
                        )
                    })?;
                independent_cost = independent_cost
                    .checked_add(encoded_reconstruction_data_len(data)?)
                    .and_then(|value| value.checked_add(RECONSTRUCTION_SECTION_OVERHEAD_BYTES))
                    .ok_or_else(|| resource("v5 independent reconstruction cost exceeds u64"))?;
            }
        }
    }
    let mut best_cost = independent_cost;
    let mut best = CohortSelection::Independent;

    if let Some(dictionary) = train_cohort_dictionary(archive, cohort, profile, planner_id)? {
        let dictionary_cost = encoded_dictionary_cost(&dictionary)?
            .checked_add(DICTIONARY_SECTION_OVERHEAD_BYTES)
            .ok_or_else(|| resource("Dictionary section cost exceeds u64"))?;
        for level in profile.dictionary_levels() {
            let plan = zstd_dictionary_plan(*level, dictionary.dictionary_id)?;
            let payload_cost = cohort.chunks.iter().try_fold(0_u64, |total, chunk_id| {
                let chunk = &archive.content_store.chunks[chunk_id];
                let encoded = encode_payload_with_dictionary(&plan, &chunk.plaintext, &dictionary)?;
                total
                    .checked_add(
                        u64::try_from(encoded.len()).map_err(|_| {
                            resource("dictionary cohort payload length exceeds u64")
                        })?,
                    )
                    .ok_or_else(|| resource("dictionary cohort payload cost exceeds u64"))
            })?;
            let plan_cost = encoded_plan_cost(&plan)?;
            let cost = payload_cost
                .checked_add(dictionary_cost)
                .and_then(|value| value.checked_add(plan_cost))
                .ok_or_else(|| resource("dictionary cohort complete cost exceeds u64"))?;
            if qualifies_against_independent(cost, independent_cost)? && cost < best_cost {
                best_cost = cost;
                best = CohortSelection::Dictionary {
                    dictionary: dictionary.clone(),
                    plan,
                };
            }
        }
    }

    for lookback in profile.lookback_candidates() {
        for level in profile.lookback_levels() {
            let plan = zstd_prefix_plan(*level, *lookback)?;
            let mut payload_cost = 0_u64;
            for (position, chunk_id) in cohort.chunks.iter().enumerate() {
                let prefix = cohort_prefix(archive, &cohort.chunks, position, *lookback)?;
                let encoded = encode_payload_with_prefix(
                    &plan,
                    &archive.content_store.chunks[chunk_id].plaintext,
                    &prefix,
                )?;
                payload_cost = payload_cost
                    .checked_add(
                        u64::try_from(encoded.len())
                            .map_err(|_| resource("lookback cohort payload length exceeds u64"))?,
                    )
                    .ok_or_else(|| resource("lookback cohort payload cost exceeds u64"))?;
            }
            let group = ChunkGroup {
                group_id: chunk_group_id(cohort.cohort_id, *level, *lookback),
                max_lookback: *lookback,
                max_preceding_bytes: maximum_preceding_bytes(archive, &cohort.chunks, *lookback)?,
            };
            let plan_cost = encoded_plan_cost(&plan)?;
            let group_cost = encoded_group_cost(&group)?;
            let cost = payload_cost
                .checked_add(plan_cost)
                .and_then(|value| value.checked_add(group_cost))
                .and_then(|value| value.checked_add(CHUNK_GROUP_SECTION_OVERHEAD_BYTES))
                .ok_or_else(|| resource("lookback cohort complete cost exceeds u64"))?;
            if qualifies_against_independent(cost, independent_cost)? && cost < best_cost {
                best_cost = cost;
                best = CohortSelection::Lookback { group, plan };
            }
        }
    }
    let _ = best_cost;
    Ok(best)
}

fn train_cohort_dictionary(
    archive: &Archive,
    cohort: &SimilarityCohort,
    profile: CompressionProfile,
    planner_id: &str,
) -> Result<Option<Dictionary>> {
    let policy = profile.similarity_policy();
    if policy.dictionary_bytes == 0 || profile.dictionary_levels().is_empty() {
        return Ok(None);
    }
    let samples = cohort
        .chunks
        .iter()
        .take(policy.maximum_training_samples)
        .map(|chunk_id| {
            let plaintext = &archive.content_store.chunks[chunk_id].plaintext;
            &plaintext[..plaintext.len().min(policy.maximum_sample_bytes)]
        })
        .collect::<Vec<_>>();
    let Ok(bytes) = train_dictionary(&samples, policy.dictionary_bytes) else {
        return Ok(None);
    };
    let dictionary_id = sha256_exact(&bytes);
    Ok(Some(Dictionary {
        dictionary_id,
        codec: "zstandard/v1".to_owned(),
        format: ZSTD_DICTIONARY_FORMAT.to_owned(),
        construction: format!(
            "{ZSTD_DICTIONARY_CONSTRUCTION_PREFIX}{}-digest-order-samples{}-sample-cap{}-dict-cap{}",
            planner_id,
            policy.maximum_training_samples,
            policy.maximum_sample_bytes,
            policy.dictionary_bytes
        ),
        bytes: bytes.into_boxed_slice(),
    }))
}

fn cohort_prefix(
    archive: &Archive,
    chunks: &[Digest],
    position: usize,
    lookback: u32,
) -> Result<Vec<u8>> {
    let lookback = usize::try_from(lookback).map_err(|_| resource("lookback exceeds usize"))?;
    let first = position.saturating_sub(lookback);
    let preceding = &chunks[first..position];
    let total = preceding.iter().try_fold(0_usize, |total, chunk_id| {
        total
            .checked_add(archive.content_store.chunks[chunk_id].plaintext.len())
            .ok_or_else(|| resource("lookback prefix size exceeds usize"))
    })?;
    let retained = total.min(usize::try_from(ZSTD_WINDOW_BYTES).unwrap_or(usize::MAX));
    let mut skip = total - retained;
    let mut prefix = Vec::with_capacity(retained);
    for chunk_id in preceding {
        let plaintext = &archive.content_store.chunks[chunk_id].plaintext;
        if skip >= plaintext.len() {
            skip -= plaintext.len();
            continue;
        }
        prefix.extend_from_slice(&plaintext[skip..]);
        skip = 0;
    }
    Ok(prefix)
}

fn maximum_preceding_bytes(archive: &Archive, chunks: &[Digest], lookback: u32) -> Result<u64> {
    let lookback = usize::try_from(lookback).map_err(|_| resource("lookback exceeds usize"))?;
    let mut maximum = 0_u64;
    for position in 0..chunks.len() {
        let first = position.saturating_sub(lookback);
        let bytes = chunks[first..position]
            .iter()
            .try_fold(0_u64, |total, chunk_id| {
                total
                    .checked_add(archive.content_store.chunks[chunk_id].logical_len)
                    .ok_or_else(|| resource("lookback access bytes exceed u64"))
            })?;
        maximum = maximum.max(bytes);
    }
    Ok(maximum)
}

fn qualifies_against_independent(candidate: u64, independent: u64) -> Result<bool> {
    Ok(candidate
        .checked_add(MINIMUM_COHORT_GAIN_BYTES)
        .ok_or_else(|| resource("cohort minimum-gain cost exceeds u64"))?
        < independent)
}

fn encoded_dictionary_cost(dictionary: &Dictionary) -> Result<u64> {
    encoded_dictionary_len(dictionary)
}

fn encoded_plan_cost(plan: &TransformPlan) -> Result<u64> {
    encoded_transform_plan_len(plan)
}

fn encoded_group_cost(group: &ChunkGroup) -> Result<u64> {
    encoded_chunk_group_len(group)
}

fn insert_dictionary(
    dictionaries: &mut BTreeMap<Digest, Dictionary>,
    dictionary: Dictionary,
) -> Result<()> {
    match dictionaries.entry(dictionary.dictionary_id) {
        std::collections::btree_map::Entry::Vacant(entry) => {
            entry.insert(dictionary);
            Ok(())
        }
        std::collections::btree_map::Entry::Occupied(entry) if entry.get() == &dictionary => Ok(()),
        std::collections::btree_map::Entry::Occupied(entry) => Err(Diagnostic::new(
            OutcomeClass::Corrupt,
            ReasonCode::DictionaryDigestMismatch,
            entry.key().to_string(),
        )),
    }
}

fn insert_plan(plans: &mut BTreeMap<u64, TransformPlan>, plan: TransformPlan) -> Result<()> {
    match plans.entry(plan.plan_id) {
        std::collections::btree_map::Entry::Vacant(entry) => {
            entry.insert(plan);
            Ok(())
        }
        std::collections::btree_map::Entry::Occupied(entry) if entry.get() == &plan => Ok(()),
        std::collections::btree_map::Entry::Occupied(entry) => Err(Diagnostic::new(
            OutcomeClass::Nonconforming,
            ReasonCode::DuplicateSemanticDeclaration,
            format!("TransformPlan ID collision at {}", entry.key()),
        )),
    }
}

fn chunk_group_id(cohort_id: Digest, level: i32, lookback: u32) -> Digest {
    let mut input = Vec::new();
    input.extend_from_slice(b"entrybound/chunk-group/v1\0");
    input.extend_from_slice(cohort_id.as_bytes());
    input.extend_from_slice(&level.to_be_bytes());
    input.extend_from_slice(&lookback.to_be_bytes());
    sha256_exact(&input)
}

/// Computes the planner's deterministic, integer-only compressibility probe.
#[must_use]
pub fn analyze(plaintext: &[u8]) -> ContentAnalysis {
    if plaintext.is_empty() {
        return ContentAnalysis {
            plaintext_len: 0,
            empty: true,
            sample_len: 0,
            distinct_symbols: 0,
            maximum_symbol_frequency_ppm: 0,
            adjacent_repeat_ppm: 0,
            likely_compressible: false,
        };
    }
    let sample_len = plaintext.len().min(PROBE_BYTES);
    let mut histogram = [0_u32; 256];
    let mut adjacent_repeats = 0_u64;
    let mut previous = None;
    for sample_index in 0..sample_len {
        let index = sample_index
            .checked_mul(plaintext.len())
            .map_or(plaintext.len() - 1, |value| value / sample_len);
        let byte = plaintext[index];
        histogram[usize::from(byte)] += 1;
        if previous == Some(byte) {
            adjacent_repeats += 1;
        }
        previous = Some(byte);
    }
    let distinct_symbols = histogram.iter().filter(|count| **count != 0).count() as u16;
    let maximum = u64::from(*histogram.iter().max().unwrap_or(&0));
    let sample_u64 = sample_len as u64;
    let maximum_symbol_frequency_ppm = ((maximum * 1_000_000) / sample_u64) as u32;
    let adjacent_repeat_ppm = if sample_len <= 1 {
        0
    } else {
        ((adjacent_repeats * 1_000_000) / (sample_u64 - 1)) as u32
    };
    let likely_compressible = distinct_symbols <= 192
        || maximum_symbol_frequency_ppm >= 12_000
        || adjacent_repeat_ppm >= 20_000;
    ContentAnalysis {
        plaintext_len: plaintext.len() as u64,
        empty: false,
        sample_len: sample_u64,
        distinct_symbols,
        maximum_symbol_frequency_ppm,
        adjacent_repeat_ppm,
        likely_compressible,
    }
}

/// Returns true only when measured stored cost clears both gain thresholds.
pub fn zstandard_wins(logical_len: u64, encoded_len: usize) -> Result<bool> {
    let relative = logical_len
        .checked_mul(MINIMUM_GAIN_BASIS_POINTS)
        .and_then(|value| value.checked_add(9_999))
        .map(|value| value / 10_000)
        .ok_or_else(|| resource("minimum-gain calculation overflow"))?;
    let minimum_gain = MINIMUM_GAIN_BYTES.max(relative);
    let encoded_cost = u64::try_from(encoded_len)
        .map_err(|_| resource("encoded candidate length exceeds u64"))?
        .checked_add(ZSTD_ATTRIBUTABLE_OVERHEAD_BYTES)
        .and_then(|value| value.checked_add(minimum_gain))
        .ok_or_else(|| resource("encoded candidate cost overflow"))?;
    Ok(encoded_cost < logical_len)
}

pub(crate) fn independent_encoded_len(
    profile: CompressionProfile,
    planner_id: &str,
    plaintext: &[u8],
) -> Result<usize> {
    if planner_id.ends_with("-v4") || planner_id.ends_with("-v5") {
        return Ok(encode_payload(&select_v4_plan(profile, plaintext)?, plaintext)?.len());
    }
    let logical_len =
        u64::try_from(plaintext.len()).map_err(|_| resource("plaintext length exceeds u64"))?;
    let mut selected = plaintext.len();
    if plaintext.len() >= MINIMUM_ZSTD_INPUT_BYTES {
        for level in profile.levels(analyze(plaintext)) {
            let encoded = encode_payload(&zstd_plan(*level)?, plaintext)?;
            if zstandard_wins(logical_len, encoded.len())? && encoded.len() < selected {
                selected = encoded.len();
            }
        }
    }
    Ok(selected)
}

fn resource(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::PolicyRefused,
        ReasonCode::ResourceLimit,
        detail,
    )
}

/// Read-only research access to planner internals.
///
/// Two layers are provided:
///
/// * thin wrappers that forward to the private production functions the
///   planners call (candidate lists, cost functions, qualification rules,
///   cohort selection, dictionary training, lookback prefix sizing, JPEG
///   region candidates), and
/// * a candidate enumeration ([`trace_chunk`], [`trace_chunk_stage`],
///   [`trace_cohort`], [`trace_jpeg_object`], [`trace_archive`]) that re-runs each production
///   selection loop outside the production function bodies, calling the same
///   private functions, and records every candidate with its cost components,
///   qualification result, exclusion reason, and the production selection.
///
/// No production function body is modified or hooked. Research harnesses must
/// check that [`PlannedAssignment::differences`] is empty against an Archive
/// planned by the real planner before trusting any enumeration.
#[cfg(feature = "research-internals")]
pub mod research {
    use std::collections::{BTreeMap, BTreeSet};

    use super::{CompressionProfile, ContentAnalysis, PlanningReport};
    use crate::codec::{
        encode_payload, encode_payload_with_dictionary, encode_payload_with_prefix,
        encode_payload_with_reconstruction, encode_transformed_payload, store_plan,
        zstd_dictionary_plan, zstd_plan, zstd_prefix_plan,
    };
    use crate::diagnostics::{Diagnostic, OutcomeClass, ReasonCode, Result};
    use crate::eam::{
        Archive, ChunkGroup, ContentObject, Dictionary, Digest, ReconstructionAuditReason,
        ReconstructionAuditTarget, ReconstructionData, ReconstructionFallbackReason,
        ReconstructionRegion, RegionAccessCost, TransformPlan,
    };
    use crate::ecf::{
        SECTION_HEADER_LEN, encoded_reconstruction_data_len, encoded_reconstruction_region_len,
        encoded_transform_plan_v2_len, encoded_transform_plan_v3_len,
    };
    use crate::identity::sha256_exact;
    use crate::similarity::{SimilarityCohort, cluster};

    pub const MINIMUM_ZSTD_INPUT_BYTES: usize = super::MINIMUM_ZSTD_INPUT_BYTES;
    pub const PROBE_BYTES: usize = super::PROBE_BYTES;
    pub const MINIMUM_COHORT_GAIN_BYTES: u64 = super::MINIMUM_COHORT_GAIN_BYTES;
    pub const DICTIONARY_SECTION_OVERHEAD_BYTES: u64 = super::DICTIONARY_SECTION_OVERHEAD_BYTES;
    pub const CHUNK_GROUP_SECTION_OVERHEAD_BYTES: u64 = super::CHUNK_GROUP_SECTION_OVERHEAD_BYTES;
    pub const RECONSTRUCTION_SECTION_OVERHEAD_BYTES: u64 =
        super::RECONSTRUCTION_SECTION_OVERHEAD_BYTES;
    pub const MINIMUM_RECONSTRUCTION_GAIN_BYTES: u64 = super::MINIMUM_RECONSTRUCTION_GAIN_BYTES;
    pub const MINIMUM_RECONSTRUCTION_GAIN_BASIS_POINTS: u64 =
        super::MINIMUM_RECONSTRUCTION_GAIN_BASIS_POINTS;

    // ---------------------------------------------------------------------
    // Thin wrappers over private production functions.
    // ---------------------------------------------------------------------

    /// v1–v3 Zstandard levels scanned for a Chunk with this probe result.
    #[must_use]
    pub fn levels(profile: CompressionProfile, analysis: ContentAnalysis) -> &'static [i32] {
        profile.levels(analysis)
    }

    #[must_use]
    pub const fn dictionary_levels(profile: CompressionProfile) -> &'static [i32] {
        profile.dictionary_levels()
    }

    #[must_use]
    pub const fn lookback_levels(profile: CompressionProfile) -> &'static [i32] {
        profile.lookback_levels()
    }

    /// Ordinary and structural-transform candidate lists, in evaluation order.
    pub fn v4_candidates(
        profile: CompressionProfile,
        plaintext: &[u8],
    ) -> Result<(Vec<TransformPlan>, Vec<TransformPlan>)> {
        super::v4_candidates(profile, plaintext)
    }

    pub fn select_v4_plan(profile: CompressionProfile, plaintext: &[u8]) -> Result<TransformPlan> {
        super::select_v4_plan(profile, plaintext)
    }

    pub fn complete_candidate_cost(plan: &TransformPlan, plaintext: &[u8]) -> Result<u64> {
        super::complete_candidate_cost(plan, plaintext)
    }

    pub fn qualifies_v4(candidate: u64, baseline: u64) -> Result<bool> {
        super::qualifies_v4(candidate, baseline)
    }

    #[must_use]
    pub fn reconstruction_max_chain(profile: CompressionProfile) -> u32 {
        super::reconstruction_max_chain(profile)
    }

    pub fn reconstruction_candidates(
        profile: CompressionProfile,
        data: &ReconstructionData,
    ) -> Result<Vec<TransformPlan>> {
        super::reconstruction_candidates(profile, data)
    }

    pub fn v5_independent_cost(
        plan: &TransformPlan,
        plaintext: &[u8],
        reconstruction: Option<(&ReconstructionData, &BTreeMap<Digest, ReconstructionData>)>,
    ) -> Result<u64> {
        super::v5_independent_cost(plan, plaintext, reconstruction)
    }

    pub fn qualifies_reconstruction(candidate: u64, baseline: u64) -> Result<bool> {
        super::qualifies_reconstruction(candidate, baseline)
    }

    /// Public mirror of the private cohort selection.
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub enum CohortSelection {
        Independent,
        Dictionary {
            dictionary: Dictionary,
            plan: TransformPlan,
        },
        Lookback {
            group: ChunkGroup,
            plan: TransformPlan,
        },
    }

    impl From<super::CohortSelection> for CohortSelection {
        fn from(value: super::CohortSelection) -> Self {
            match value {
                super::CohortSelection::Independent => Self::Independent,
                super::CohortSelection::Dictionary { dictionary, plan } => {
                    Self::Dictionary { dictionary, plan }
                }
                super::CohortSelection::Lookback { group, plan } => Self::Lookback { group, plan },
            }
        }
    }

    pub fn select_cohort_plan(
        archive: &Archive,
        cohort: &SimilarityCohort,
        profile: CompressionProfile,
        planner_id: &str,
        plans: &BTreeMap<u64, TransformPlan>,
    ) -> Result<CohortSelection> {
        super::select_cohort_plan(archive, cohort, profile, planner_id, plans)
            .map(CohortSelection::from)
    }

    pub fn train_cohort_dictionary(
        archive: &Archive,
        cohort: &SimilarityCohort,
        profile: CompressionProfile,
        planner_id: &str,
    ) -> Result<Option<Dictionary>> {
        super::train_cohort_dictionary(archive, cohort, profile, planner_id)
    }

    pub fn cohort_prefix(
        archive: &Archive,
        chunks: &[Digest],
        position: usize,
        lookback: u32,
    ) -> Result<Vec<u8>> {
        super::cohort_prefix(archive, chunks, position, lookback)
    }

    pub fn maximum_preceding_bytes(
        archive: &Archive,
        chunks: &[Digest],
        lookback: u32,
    ) -> Result<u64> {
        super::maximum_preceding_bytes(archive, chunks, lookback)
    }

    pub fn qualifies_against_independent(candidate: u64, independent: u64) -> Result<bool> {
        super::qualifies_against_independent(candidate, independent)
    }

    pub fn jpeg_region_candidates(profile: CompressionProfile) -> Result<Vec<TransformPlan>> {
        super::jpeg_region_candidates(profile)
    }

    pub fn v6_ordinary_payload_cost(
        archive: &Archive,
        object: &ContentObject,
        plans: &BTreeMap<u64, TransformPlan>,
    ) -> Result<u64> {
        super::v6_ordinary_payload_cost(archive, object, plans)
    }

    #[must_use]
    pub fn chunk_group_id(cohort_id: Digest, level: i32, lookback: u32) -> Digest {
        super::chunk_group_id(cohort_id, level, lookback)
    }

    pub fn independent_encoded_len(
        profile: CompressionProfile,
        planner_id: &str,
        plaintext: &[u8],
    ) -> Result<usize> {
        super::independent_encoded_len(profile, planner_id, plaintext)
    }

    #[must_use]
    pub fn chunk_object_owners(archive: &Archive) -> BTreeMap<Digest, BTreeSet<Digest>> {
        super::chunk_object_owners(archive)
    }

    // ---------------------------------------------------------------------
    // Candidate enumeration.
    // ---------------------------------------------------------------------

    /// Frozen planner generations the enumeration reproduces.
    #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    pub enum PlannerVersion {
        V1,
        V2,
        V3,
        V4,
        V5,
        V6,
    }

    impl PlannerVersion {
        pub const ALL: [Self; 6] = [Self::V1, Self::V2, Self::V3, Self::V4, Self::V5, Self::V6];

        /// Planner ID the production planner records for this generation.
        #[must_use]
        pub const fn planner_id(self, profile: CompressionProfile) -> &'static str {
            match self {
                Self::V1 => profile.planner_v1_id(),
                Self::V2 => profile.planner_v2_id(),
                Self::V3 => profile.planner_v3_id(),
                Self::V4 => profile.planner_v4_id(),
                Self::V5 => profile.planner_v5_id(),
                Self::V6 => profile.planner_id(),
            }
        }

        /// Planner ID the Chunk and cohort stages receive. v6 runs the frozen v5 stages.
        #[must_use]
        pub const fn stage_planner_id(self, profile: CompressionProfile) -> &'static str {
            match self {
                Self::V6 => profile.planner_v5_id(),
                _ => self.planner_id(profile),
            }
        }

        /// Runs the real production planner of this generation.
        pub fn plan(
            self,
            archive: &mut Archive,
            profile: CompressionProfile,
        ) -> Result<PlanningReport> {
            match self {
                Self::V1 => super::plan_archive(archive, profile),
                Self::V2 => super::plan_archive_v2(archive, profile),
                Self::V3 => super::plan_archive_v3(archive, profile),
                Self::V4 => super::plan_archive_v4(archive, profile),
                Self::V5 => super::plan_archive_v5(archive, profile),
                Self::V6 => super::plan_archive_v6(archive, profile),
            }
        }

        const fn has_cohort_stage(self) -> bool {
            !matches!(self, Self::V1 | Self::V2)
        }

        const fn has_reconstruction_stage(self) -> bool {
            matches!(self, Self::V5 | Self::V6)
        }
    }

    /// Production rule that evaluated a per-Chunk candidate.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub enum CandidateStage {
        /// v1–v3 Zstandard level scan, qualified by `zstandard_wins` against the logical length.
        IndependentZstandard,
        /// v1–v3 STORE fallback, used when no Zstandard level qualifies.
        IndependentStore,
        /// v4 ordinary candidates. The first (STORE) is the qualification baseline.
        V4Ordinary,
        /// v4 structural-transform candidates, qualified against the best ordinary cost.
        V4Transformed,
        /// v5 baseline: the v4 winner recosted with a TransformPlan v2 record.
        V5Baseline,
        /// v5 DEFLATE-reconstructive candidates, qualified against the v5 baseline.
        V5Reconstruction,
    }

    /// Why a traced candidate is not the production selection.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub enum ExclusionReason {
        /// Failed the stage's minimum-gain qualification rule.
        InsufficientGain,
        /// Qualified, but did not strictly beat the incumbent evaluated before it.
        /// Earlier candidates win exact ties.
        NotCheaperThanIncumbent,
        /// Was the baseline, fallback, or incumbent, but a later candidate won.
        Superseded,
        /// Won its stage and became the next stage's baseline (v4 winner -> v5 baseline).
        CarriedToNextStage,
    }

    /// One per-Chunk candidate with the cost components the production rule used.
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub struct CandidateCost {
        pub stage: CandidateStage,
        pub plan: TransformPlan,
        /// Operational codec output length.
        pub payload_bytes: u64,
        /// Encoded TransformPlan record bytes the stage charges (zero for v1–v3,
        /// the v1 record for v4 stages, the v2 record for v5 stages).
        pub plan_record_bytes: u64,
        /// ReconstructionData record plus section overhead (v5 reconstructive candidates).
        pub side_data_bytes: u64,
        /// The value the production rule compares: the payload for v1–v3, otherwise
        /// `payload_bytes + plan_record_bytes + side_data_bytes`.
        pub complete_cost: u64,
        /// Value the qualification rule compared against (the logical length for
        /// v1–v3); `None` for a stage baseline or fallback.
        pub qualification_baseline: Option<u64>,
        pub qualified: bool,
        /// True only for the Chunk-stage plan production assigns.
        pub selected: bool,
        pub exclusion: Option<ExclusionReason>,
    }

    /// Outcome of the v5 DEFLATE reconstruction attempt for one Chunk.
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub struct ReconstructionAttempt {
        pub max_chain: u32,
        /// Verified side data when `try_forward` produced a candidate.
        pub data: Option<ReconstructionData>,
        /// `encoded_reconstruction_data_len` plus the reconstruction section overhead.
        pub side_data_bytes: Option<u64>,
        /// Fallback reason the independent stage records. The cohort stage may later
        /// add `CompleteCostDidNotWin` or remove it; see [`PlannedAssignment`].
        pub fallback: Option<ReconstructionFallbackReason>,
    }

    /// Every candidate one production Chunk stage evaluates for one plaintext.
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub struct ChunkTrace {
        pub chunk_id: Digest,
        pub logical_len: u64,
        pub profile: CompressionProfile,
        pub version: PlannerVersion,
        pub analysis: ContentAnalysis,
        /// Candidates in production evaluation order.
        pub candidates: Vec<CandidateCost>,
        pub reconstruction: Option<ReconstructionAttempt>,
        /// Index in `candidates` of the plan the production Chunk stage assigns.
        pub selected: usize,
    }

    impl ChunkTrace {
        #[must_use]
        pub fn selected_plan(&self) -> &TransformPlan {
            &self.candidates[self.selected].plan
        }
    }

    /// Strategy of one cross-file cohort candidate.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub enum CohortStrategy {
        Dictionary { level: i32 },
        Lookback { level: i32, lookback: u32 },
    }

    /// One cohort candidate with the cost components `select_cohort_plan` used.
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub struct CohortCandidate {
        pub strategy: CohortStrategy,
        pub plan: TransformPlan,
        /// Declared ChunkGroup of a lookback candidate.
        pub group: Option<ChunkGroup>,
        /// Sum of member payloads under this strategy.
        pub payload_bytes: u64,
        /// `encoded_transform_plan_len` of the shared plan.
        pub plan_record_bytes: u64,
        /// Dictionary record plus section overhead, or ChunkGroup record plus section overhead.
        pub side_data_bytes: u64,
        pub complete_cost: u64,
        /// `qualifies_against_independent(complete_cost, independent)`.
        pub qualified: bool,
        pub selected: bool,
        pub exclusion: Option<ExclusionReason>,
    }

    /// Independent (Chunk-stage) cost of a cohort as `select_cohort_plan` computes it.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct IndependentCohortCost {
        pub payload_bytes: u64,
        /// Distinct TransformPlan records (zero for v3, v1 records for v4, v2 records for v5).
        pub plan_record_bytes: u64,
        /// Distinct ReconstructionData records plus section overhead (v5 only).
        pub side_data_bytes: u64,
        pub complete_cost: u64,
    }

    /// Every candidate `select_cohort_plan` evaluates for one cohort.
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub struct CohortTrace {
        pub cohort: SimilarityCohort,
        pub planner_id: String,
        pub independent: IndependentCohortCost,
        /// Dictionary produced by `train_cohort_dictionary`, if training ran and succeeded.
        pub dictionary: Option<Dictionary>,
        /// Dictionary candidates first, then lookback candidates, in evaluation order.
        pub candidates: Vec<CohortCandidate>,
        /// Index in `candidates`, or `None` when the independent assignment is kept.
        pub selected: Option<usize>,
    }

    impl CohortTrace {
        /// The selection the traced rule reproduces.
        #[must_use]
        pub fn selection(&self) -> CohortSelection {
            let Some(candidate) = self.selected.map(|index| &self.candidates[index]) else {
                return CohortSelection::Independent;
            };
            match (&candidate.strategy, &self.dictionary, &candidate.group) {
                (CohortStrategy::Dictionary { .. }, Some(dictionary), _) => {
                    CohortSelection::Dictionary {
                        dictionary: dictionary.clone(),
                        plan: candidate.plan.clone(),
                    }
                }
                (CohortStrategy::Lookback { .. }, _, Some(group)) => CohortSelection::Lookback {
                    group: group.clone(),
                    plan: candidate.plan.clone(),
                },
                _ => CohortSelection::Independent,
            }
        }
    }

    /// One v6 JPEG region candidate with the cost components production used.
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub struct RegionCandidate {
        pub plan: TransformPlan,
        /// Codec output over the verified JPEG XL representation.
        pub representation_bytes: u64,
        /// `encoded_transform_plan_v3_len`, or zero when the plan is already recorded.
        pub plan_record_bytes: u64,
        /// ReconstructionRegion record bytes excluding the representation.
        pub region_record_bytes: u64,
        /// Section header charged only while no region section has been emitted.
        pub section_header_bytes: u64,
        pub complete_cost: u64,
        /// `qualifies_reconstruction(complete_cost, ordinary_cost)`. Production applies
        /// the rule only to the cheapest candidate; for others this is informational.
        pub qualified: bool,
        pub selected: bool,
        pub exclusion: Option<ExclusionReason>,
    }

    /// The v6 whole-object attempt for one ContentObject.
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub struct ObjectTrace {
        pub content_object: Digest,
        pub chunk_count: u64,
        /// Summed logical bytes, once the eligibility checks reach that step.
        pub logical_bytes: Option<u64>,
        /// JPEG XL representation length after `verified_forward` succeeded.
        pub transformed_bytes: Option<u64>,
        /// `v6_ordinary_payload_cost` of the v5 assignment, once costed.
        pub ordinary_cost: Option<u64>,
        pub candidates: Vec<RegionCandidate>,
        pub selected: Option<usize>,
        /// Region production records when a candidate is selected.
        pub region: Option<ReconstructionRegion>,
        /// Audit reason production records when no region is selected.
        pub audit: Option<ReconstructionAuditReason>,
    }

    /// Final per-Chunk physical assignment.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct ChunkAssignment {
        pub plan_ref: u64,
        pub group_ref: Option<Digest>,
    }

    /// The physical planning state the traced rules reproduce for a whole Archive.
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub struct PlannedAssignment {
        pub planner_id: String,
        pub chunks: BTreeMap<Digest, ChunkAssignment>,
        pub transform_plans: BTreeMap<u64, TransformPlan>,
        pub dictionaries: BTreeMap<Digest, Dictionary>,
        pub chunk_groups: BTreeMap<Digest, ChunkGroup>,
        pub reconstruction_data: BTreeMap<Digest, ReconstructionData>,
        pub reconstruction_fallbacks: BTreeMap<Digest, ReconstructionFallbackReason>,
        pub reconstruction_regions: BTreeMap<Digest, ReconstructionRegion>,
        pub reconstruction_audits: BTreeMap<ReconstructionAuditTarget, ReconstructionAuditReason>,
        pub physical_order: Box<[Digest]>,
    }

    impl PlannedAssignment {
        /// Differences from an Archive planned by the real planner of the same
        /// profile and generation. An empty result means the enumeration reproduced
        /// every production choice exactly.
        #[must_use]
        pub fn differences(&self, archive: &Archive) -> Vec<String> {
            let mut differences = Vec::new();
            if archive.descriptor.planner_id != self.planner_id {
                differences.push(format!(
                    "planner_id: planner {}, trace {}",
                    archive.descriptor.planner_id, self.planner_id
                ));
            }
            if archive.content_store.chunks.len() != self.chunks.len() {
                differences.push(format!(
                    "chunk count: planner {}, trace {}",
                    archive.content_store.chunks.len(),
                    self.chunks.len()
                ));
            }
            for (chunk_id, chunk) in &archive.content_store.chunks {
                let planned = ChunkAssignment {
                    plan_ref: chunk.plan_ref,
                    group_ref: chunk.group_ref,
                };
                if self.chunks.get(chunk_id) != Some(&planned) {
                    differences.push(format!(
                        "chunk {chunk_id}: planner {planned:?}, trace {:?}",
                        self.chunks.get(chunk_id)
                    ));
                }
            }
            let plans = archive
                .transform_plans
                .iter()
                .map(|plan| (plan.plan_id, plan.clone()))
                .collect::<BTreeMap<_, _>>();
            if plans.len() != archive.transform_plans.len() || plans != self.transform_plans {
                differences.push(format!(
                    "transform plans: planner {:?}, trace {:?}",
                    plans.keys().collect::<Vec<_>>(),
                    self.transform_plans.keys().collect::<Vec<_>>()
                ));
            }
            if archive.content_store.dictionaries != self.dictionaries {
                differences.push(format!(
                    "dictionaries: planner {:?}, trace {:?}",
                    archive
                        .content_store
                        .dictionaries
                        .keys()
                        .collect::<Vec<_>>(),
                    self.dictionaries.keys().collect::<Vec<_>>()
                ));
            }
            if archive.content_store.chunk_groups != self.chunk_groups {
                differences.push(format!(
                    "chunk groups: planner {:?}, trace {:?}",
                    archive.content_store.chunk_groups, self.chunk_groups
                ));
            }
            if archive.content_store.reconstruction_data != self.reconstruction_data {
                differences.push(format!(
                    "reconstruction data: planner {:?}, trace {:?}",
                    archive
                        .content_store
                        .reconstruction_data
                        .keys()
                        .collect::<Vec<_>>(),
                    self.reconstruction_data.keys().collect::<Vec<_>>()
                ));
            }
            if archive.content_store.reconstruction_fallbacks != self.reconstruction_fallbacks {
                differences.push(format!(
                    "reconstruction fallbacks: planner {:?}, trace {:?}",
                    archive.content_store.reconstruction_fallbacks, self.reconstruction_fallbacks
                ));
            }
            if archive.content_store.reconstruction_regions != self.reconstruction_regions {
                differences.push(format!(
                    "reconstruction regions: planner {:?}, trace {:?}",
                    archive
                        .content_store
                        .reconstruction_regions
                        .keys()
                        .collect::<Vec<_>>(),
                    self.reconstruction_regions.keys().collect::<Vec<_>>()
                ));
            }
            let audits = archive
                .content_store
                .reconstruction_audits
                .iter()
                .map(|(target, audit)| (*target, audit.reason))
                .collect::<BTreeMap<_, _>>();
            if audits != self.reconstruction_audits
                || archive
                    .content_store
                    .reconstruction_audits
                    .iter()
                    .any(|(target, audit)| {
                        audit.target != *target
                            || audit.transform_id != crate::jpeg_reconstruction::JPEG_RECONSTRUCT_ID
                    })
            {
                differences.push(format!(
                    "reconstruction audits: planner {:?}, trace {:?}",
                    archive.content_store.reconstruction_audits, self.reconstruction_audits
                ));
            }
            if archive.content_store.physical_order != self.physical_order {
                differences.push("physical order differs".to_owned());
            }
            differences
        }
    }

    /// Complete enumeration of one production planning run over an Archive.
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub struct ArchiveTrace {
        pub profile: CompressionProfile,
        pub version: PlannerVersion,
        /// Chunk-stage traces in Chunk digest order.
        pub chunks: Vec<ChunkTrace>,
        /// Cohort traces in cohort ID order (v3 and later).
        pub cohorts: Vec<CohortTrace>,
        /// Whole-object traces in logical digest order (v6, non-Fast).
        pub objects: Vec<ObjectTrace>,
        pub assignment: PlannedAssignment,
    }

    impl ArchiveTrace {
        #[must_use]
        pub fn chunk(&self, chunk_id: &Digest) -> Option<&ChunkTrace> {
            self.chunks
                .binary_search_by(|trace| trace.chunk_id.cmp(chunk_id))
                .ok()
                .map(|index| &self.chunks[index])
        }
    }

    trait Ranked {
        fn is_qualified(&self) -> bool;
        fn exclude(&mut self, reason: ExclusionReason);
    }

    impl Ranked for CandidateCost {
        fn is_qualified(&self) -> bool {
            self.qualified
        }

        fn exclude(&mut self, reason: ExclusionReason) {
            self.exclusion = Some(reason);
        }
    }

    impl Ranked for CohortCandidate {
        fn is_qualified(&self) -> bool {
            self.qualified
        }

        fn exclude(&mut self, reason: ExclusionReason) {
            self.exclusion = Some(reason);
        }
    }

    /// Records one candidate under the production incumbent rule: it replaces the
    /// incumbent only when `wins` is true.
    fn push_ranked<T: Ranked>(
        candidates: &mut Vec<T>,
        incumbent: &mut Option<usize>,
        mut candidate: T,
        wins: bool,
    ) {
        let index = candidates.len();
        if wins {
            if let Some(previous) = incumbent.replace(index) {
                candidates[previous].exclude(ExclusionReason::Superseded);
            }
        } else if candidate.is_qualified() {
            candidate.exclude(ExclusionReason::NotCheaperThanIncumbent);
        } else {
            candidate.exclude(ExclusionReason::InsufficientGain);
        }
        candidates.push(candidate);
    }

    fn u64_len(bytes: &[u8], detail: &'static str) -> Result<u64> {
        u64::try_from(bytes.len()).map_err(|_| super::resource(detail))
    }

    fn checked_sum(values: &[u64], detail: &'static str) -> Result<u64> {
        values.iter().try_fold(0_u64, |total, value| {
            total
                .checked_add(*value)
                .ok_or_else(|| super::resource(detail))
        })
    }

    fn unknown_plan(plan_ref: u64) -> Diagnostic {
        Diagnostic::new(
            OutcomeClass::Unsupported,
            ReasonCode::UnknownTransformPlan,
            plan_ref.to_string(),
        )
    }

    fn is_reconstructive_plan(plan: &TransformPlan) -> bool {
        plan.transforms
            .iter()
            .any(|step| step.reconstruction_ref.is_some())
    }

    /// Enumerates every candidate the production Chunk stage of `version` evaluates
    /// for `plaintext`: the v1–v3 Zstandard scan, the v4 measured
    /// codec/transform competition, or the v4 competition followed by the v5
    /// DEFLATE reconstruction competition (v5 and v6).
    pub fn trace_chunk(
        profile: CompressionProfile,
        version: PlannerVersion,
        plaintext: &[u8],
    ) -> Result<ChunkTrace> {
        trace_chunk_with_id(profile, version, sha256_exact(plaintext), plaintext)
    }

    fn trace_chunk_with_id(
        profile: CompressionProfile,
        version: PlannerVersion,
        chunk_id: Digest,
        plaintext: &[u8],
    ) -> Result<ChunkTrace> {
        let logical_len = u64_len(plaintext, "plaintext length exceeds u64")?;
        let analysis = super::analyze(plaintext);
        let mut candidates = Vec::new();
        let mut reconstruction = None;
        let selected = match version {
            PlannerVersion::V1 | PlannerVersion::V2 | PlannerVersion::V3 => {
                trace_zstandard_scan(profile, plaintext, logical_len, analysis, &mut candidates)?
            }
            PlannerVersion::V4 => trace_v4(profile, plaintext, &mut candidates)?,
            PlannerVersion::V5 | PlannerVersion::V6 => {
                let ordinary = trace_v4(profile, plaintext, &mut candidates)?;
                if profile == CompressionProfile::Fast {
                    ordinary
                } else {
                    let (selected, attempt) =
                        trace_v5_reconstruction(profile, plaintext, ordinary, &mut candidates)?;
                    reconstruction = Some(attempt);
                    selected
                }
            }
        };
        candidates[selected].selected = true;
        Ok(ChunkTrace {
            chunk_id,
            logical_len,
            profile,
            version,
            analysis,
            candidates,
            reconstruction,
            selected,
        })
    }

    /// Mirrors the per-Chunk loop of `plan_archive_with_id` (v1–v3).
    fn trace_zstandard_scan(
        profile: CompressionProfile,
        plaintext: &[u8],
        logical_len: u64,
        analysis: ContentAnalysis,
        candidates: &mut Vec<CandidateCost>,
    ) -> Result<usize> {
        let mut incumbent = None;
        let mut incumbent_len = None::<usize>;
        if plaintext.len() >= super::MINIMUM_ZSTD_INPUT_BYTES {
            for level in profile.levels(analysis) {
                let plan = zstd_plan(*level)?;
                let encoded = encode_payload(&plan, plaintext)?;
                let qualified = super::zstandard_wins(logical_len, encoded.len())?;
                let wins =
                    qualified && incumbent_len.is_none_or(|previous| encoded.len() < previous);
                if wins {
                    incumbent_len = Some(encoded.len());
                }
                let payload_bytes = u64_len(&encoded, "encoded candidate length exceeds u64")?;
                push_ranked(
                    candidates,
                    &mut incumbent,
                    CandidateCost {
                        stage: CandidateStage::IndependentZstandard,
                        plan,
                        payload_bytes,
                        plan_record_bytes: 0,
                        side_data_bytes: 0,
                        complete_cost: payload_bytes,
                        qualification_baseline: Some(logical_len),
                        qualified,
                        selected: false,
                        exclusion: None,
                    },
                    wins,
                );
            }
        }
        let store_index = candidates.len();
        candidates.push(CandidateCost {
            stage: CandidateStage::IndependentStore,
            plan: store_plan(),
            payload_bytes: logical_len,
            plan_record_bytes: 0,
            side_data_bytes: 0,
            complete_cost: logical_len,
            qualification_baseline: None,
            qualified: true,
            selected: false,
            exclusion: incumbent.map(|_| ExclusionReason::Superseded),
        });
        Ok(incumbent.unwrap_or(store_index))
    }

    fn measure_v4(
        stage: CandidateStage,
        plan: &TransformPlan,
        plaintext: &[u8],
    ) -> Result<CandidateCost> {
        let payload_bytes = u64_len(
            &encode_payload(plan, plaintext)?,
            "v4 candidate payload length exceeds u64",
        )?;
        let plan_record_bytes = super::encoded_plan_cost(plan)?;
        Ok(CandidateCost {
            stage,
            plan: plan.clone(),
            payload_bytes,
            plan_record_bytes,
            side_data_bytes: 0,
            complete_cost: checked_sum(
                &[payload_bytes, plan_record_bytes],
                "v4 complete candidate cost exceeds u64",
            )?,
            qualification_baseline: None,
            qualified: false,
            selected: false,
            exclusion: None,
        })
    }

    /// Mirrors `select_v4_plan`. Returns the index of the v4 winner.
    fn trace_v4(
        profile: CompressionProfile,
        plaintext: &[u8],
        candidates: &mut Vec<CandidateCost>,
    ) -> Result<usize> {
        let (ordinary, transformed) = super::v4_candidates(profile, plaintext)?;
        let mut store = measure_v4(CandidateStage::V4Ordinary, &ordinary[0], plaintext)?;
        store.qualified = true;
        let store_cost = store.complete_cost;
        let store_index = candidates.len();
        let mut incumbent = None;
        push_ranked(candidates, &mut incumbent, store, true);
        let mut selected_cost = store_cost;
        for plan in ordinary.iter().skip(1) {
            let mut candidate = measure_v4(CandidateStage::V4Ordinary, plan, plaintext)?;
            candidate.qualification_baseline = Some(store_cost);
            candidate.qualified = super::qualifies_v4(candidate.complete_cost, store_cost)?;
            let wins = candidate.qualified && candidate.complete_cost < selected_cost;
            if wins {
                selected_cost = candidate.complete_cost;
            }
            push_ranked(candidates, &mut incumbent, candidate, wins);
        }
        let best_ordinary = selected_cost;
        for plan in &transformed {
            let mut candidate = measure_v4(CandidateStage::V4Transformed, plan, plaintext)?;
            candidate.qualification_baseline = Some(best_ordinary);
            candidate.qualified = super::qualifies_v4(candidate.complete_cost, best_ordinary)?;
            let wins = candidate.qualified && candidate.complete_cost < selected_cost;
            if wins {
                selected_cost = candidate.complete_cost;
            }
            push_ranked(candidates, &mut incumbent, candidate, wins);
        }
        Ok(incumbent.unwrap_or(store_index))
    }

    fn reconstruction_side_data_cost(data: &ReconstructionData) -> Result<u64> {
        checked_sum(
            &[
                encoded_reconstruction_data_len(data)?,
                super::RECONSTRUCTION_SECTION_OVERHEAD_BYTES,
            ],
            "v5 ReconstructionData cost overflow",
        )
    }

    fn measure_v5(
        stage: CandidateStage,
        plan: &TransformPlan,
        plaintext: &[u8],
        reconstruction: Option<(&ReconstructionData, &BTreeMap<Digest, ReconstructionData>)>,
    ) -> Result<CandidateCost> {
        let payload = match reconstruction {
            Some((_, values)) => encode_payload_with_reconstruction(plan, plaintext, values)?,
            None => encode_payload(plan, plaintext)?,
        };
        let payload_bytes = u64_len(&payload, "v5 candidate payload exceeds u64")?;
        let plan_record_bytes = encoded_transform_plan_v2_len(plan)?;
        let side_data_bytes = match reconstruction {
            Some((data, _)) => reconstruction_side_data_cost(data)?,
            None => 0,
        };
        Ok(CandidateCost {
            stage,
            plan: plan.clone(),
            payload_bytes,
            plan_record_bytes,
            side_data_bytes,
            complete_cost: checked_sum(
                &[payload_bytes, plan_record_bytes, side_data_bytes],
                "v5 candidate cost overflow",
            )?,
            qualification_baseline: None,
            qualified: false,
            selected: false,
            exclusion: None,
        })
    }

    /// Mirrors the reconstruction branch of `plan_archive_v5_independent`.
    fn trace_v5_reconstruction(
        profile: CompressionProfile,
        plaintext: &[u8],
        ordinary_index: usize,
        candidates: &mut Vec<CandidateCost>,
    ) -> Result<(usize, ReconstructionAttempt)> {
        let max_chain = super::reconstruction_max_chain(profile);
        let Some(forward) = crate::reconstruction::try_forward(plaintext, max_chain)? else {
            return Ok((
                ordinary_index,
                ReconstructionAttempt {
                    max_chain,
                    data: None,
                    side_data_bytes: None,
                    fallback: Some(ReconstructionFallbackReason::UnrecognizedOrVerificationFailed),
                },
            ));
        };
        let ordinary = candidates[ordinary_index].plan.clone();
        candidates[ordinary_index].exclusion = Some(ExclusionReason::CarriedToNextStage);
        let mut baseline = measure_v5(CandidateStage::V5Baseline, &ordinary, plaintext, None)?;
        baseline.qualified = true;
        let baseline_cost = baseline.complete_cost;
        let baseline_index = candidates.len();
        let mut incumbent = None;
        push_ranked(candidates, &mut incumbent, baseline, true);

        let data_map = BTreeMap::from([(forward.data.reconstruction_id, forward.data.clone())]);
        let mut best_cost = baseline_cost;
        let mut reconstructive_won = false;
        for plan in super::reconstruction_candidates(profile, &forward.data)? {
            let mut candidate = measure_v5(
                CandidateStage::V5Reconstruction,
                &plan,
                plaintext,
                Some((&forward.data, &data_map)),
            )?;
            candidate.qualification_baseline = Some(baseline_cost);
            candidate.qualified =
                super::qualifies_reconstruction(candidate.complete_cost, baseline_cost)?;
            let wins = candidate.qualified && candidate.complete_cost < best_cost;
            if wins {
                best_cost = candidate.complete_cost;
                reconstructive_won = true;
            }
            push_ranked(candidates, &mut incumbent, candidate, wins);
        }
        let side_data_bytes = reconstruction_side_data_cost(&forward.data)?;
        Ok((
            incumbent.unwrap_or(baseline_index),
            ReconstructionAttempt {
                max_chain,
                data: Some(forward.data),
                side_data_bytes: Some(side_data_bytes),
                fallback: (!reconstructive_won)
                    .then_some(ReconstructionFallbackReason::CompleteCostDidNotWin),
            },
        ))
    }

    /// Mirrors the independent-cost prelude of `select_cohort_plan`.
    fn independent_cohort_cost(
        archive: &Archive,
        cohort: &SimilarityCohort,
        planner_id: &str,
        plans: &BTreeMap<u64, TransformPlan>,
    ) -> Result<IndependentCohortCost> {
        let payload_bytes = cohort.chunks.iter().try_fold(0_u64, |total, chunk_id| {
            let chunk = &archive.content_store.chunks[chunk_id];
            let plan = plans
                .get(&chunk.plan_ref)
                .ok_or_else(|| unknown_plan(chunk.plan_ref))?;
            let encoded = if is_reconstructive_plan(plan) {
                encode_payload_with_reconstruction(
                    plan,
                    &chunk.plaintext,
                    &archive.content_store.reconstruction_data,
                )?
            } else {
                encode_payload(plan, &chunk.plaintext)?
            };
            total
                .checked_add(u64_len(
                    &encoded,
                    "independent cohort payload length exceeds u64",
                )?)
                .ok_or_else(|| super::resource("independent cohort cost exceeds u64"))
        })?;
        let mut plan_record_bytes = 0_u64;
        let mut side_data_bytes = 0_u64;
        let v5 = planner_id.ends_with("-v5");
        if planner_id.ends_with("-v4") || v5 {
            let plan_ids = cohort
                .chunks
                .iter()
                .map(|chunk_id| archive.content_store.chunks[chunk_id].plan_ref)
                .collect::<BTreeSet<_>>();
            for plan_id in plan_ids {
                let plan = plans.get(&plan_id).ok_or_else(|| unknown_plan(plan_id))?;
                let record = if v5 {
                    encoded_transform_plan_v2_len(plan)?
                } else {
                    super::encoded_plan_cost(plan)?
                };
                plan_record_bytes = checked_sum(
                    &[plan_record_bytes, record],
                    "v4 independent cohort complete cost exceeds u64",
                )?;
            }
            if v5 {
                let references = cohort
                    .chunks
                    .iter()
                    .filter_map(|chunk_id| {
                        let plan = plans.get(&archive.content_store.chunks[chunk_id].plan_ref)?;
                        plan.transforms
                            .iter()
                            .find_map(|step| step.reconstruction_ref)
                    })
                    .collect::<BTreeSet<_>>();
                for reference in references {
                    let data = archive
                        .content_store
                        .reconstruction_data
                        .get(&reference)
                        .ok_or_else(|| {
                            Diagnostic::new(
                                OutcomeClass::Nonconforming,
                                ReasonCode::UnknownReconstructionData,
                                reference.to_string(),
                            )
                        })?;
                    side_data_bytes = checked_sum(
                        &[side_data_bytes, reconstruction_side_data_cost(data)?],
                        "v5 independent reconstruction cost exceeds u64",
                    )?;
                }
            }
        }
        Ok(IndependentCohortCost {
            payload_bytes,
            plan_record_bytes,
            side_data_bytes,
            complete_cost: checked_sum(
                &[payload_bytes, plan_record_bytes, side_data_bytes],
                "independent cohort complete cost exceeds u64",
            )?,
        })
    }

    /// Enumerates every candidate `select_cohort_plan` evaluates for one cohort,
    /// given the Archive state and plan table that exist when production reaches it
    /// (for example [`ChunkStageTrace::archive`] and [`ChunkStageTrace::plans`]).
    pub fn trace_cohort(
        archive: &Archive,
        cohort: &SimilarityCohort,
        profile: CompressionProfile,
        planner_id: &str,
        plans: &BTreeMap<u64, TransformPlan>,
    ) -> Result<CohortTrace> {
        let independent = independent_cohort_cost(archive, cohort, planner_id, plans)?;
        let independent_cost = independent.complete_cost;
        let mut best_cost = independent_cost;
        let mut candidates = Vec::new();
        let mut incumbent = None;

        let dictionary = super::train_cohort_dictionary(archive, cohort, profile, planner_id)?;
        if let Some(dictionary) = &dictionary {
            let dictionary_cost = checked_sum(
                &[
                    super::encoded_dictionary_cost(dictionary)?,
                    super::DICTIONARY_SECTION_OVERHEAD_BYTES,
                ],
                "Dictionary section cost exceeds u64",
            )?;
            for level in profile.dictionary_levels() {
                let plan = zstd_dictionary_plan(*level, dictionary.dictionary_id)?;
                let payload_bytes = cohort.chunks.iter().try_fold(0_u64, |total, chunk_id| {
                    let chunk = &archive.content_store.chunks[chunk_id];
                    let encoded =
                        encode_payload_with_dictionary(&plan, &chunk.plaintext, dictionary)?;
                    total
                        .checked_add(u64_len(
                            &encoded,
                            "dictionary cohort payload length exceeds u64",
                        )?)
                        .ok_or_else(|| {
                            super::resource("dictionary cohort payload cost exceeds u64")
                        })
                })?;
                let plan_record_bytes = super::encoded_plan_cost(&plan)?;
                let complete_cost = checked_sum(
                    &[payload_bytes, dictionary_cost, plan_record_bytes],
                    "dictionary cohort complete cost exceeds u64",
                )?;
                let qualified =
                    super::qualifies_against_independent(complete_cost, independent_cost)?;
                let wins = qualified && complete_cost < best_cost;
                if wins {
                    best_cost = complete_cost;
                }
                push_ranked(
                    &mut candidates,
                    &mut incumbent,
                    CohortCandidate {
                        strategy: CohortStrategy::Dictionary { level: *level },
                        plan,
                        group: None,
                        payload_bytes,
                        plan_record_bytes,
                        side_data_bytes: dictionary_cost,
                        complete_cost,
                        qualified,
                        selected: false,
                        exclusion: None,
                    },
                    wins,
                );
            }
        }

        for lookback in profile.lookback_candidates() {
            for level in profile.lookback_levels() {
                let plan = zstd_prefix_plan(*level, *lookback)?;
                let mut payload_bytes = 0_u64;
                for (position, chunk_id) in cohort.chunks.iter().enumerate() {
                    let prefix =
                        super::cohort_prefix(archive, &cohort.chunks, position, *lookback)?;
                    let encoded = encode_payload_with_prefix(
                        &plan,
                        &archive.content_store.chunks[chunk_id].plaintext,
                        &prefix,
                    )?;
                    payload_bytes = checked_sum(
                        &[
                            payload_bytes,
                            u64_len(&encoded, "lookback cohort payload length exceeds u64")?,
                        ],
                        "lookback cohort payload cost exceeds u64",
                    )?;
                }
                let group = ChunkGroup {
                    group_id: super::chunk_group_id(cohort.cohort_id, *level, *lookback),
                    max_lookback: *lookback,
                    max_preceding_bytes: super::maximum_preceding_bytes(
                        archive,
                        &cohort.chunks,
                        *lookback,
                    )?,
                };
                let plan_record_bytes = super::encoded_plan_cost(&plan)?;
                let side_data_bytes = checked_sum(
                    &[
                        super::encoded_group_cost(&group)?,
                        super::CHUNK_GROUP_SECTION_OVERHEAD_BYTES,
                    ],
                    "lookback cohort complete cost exceeds u64",
                )?;
                let complete_cost = checked_sum(
                    &[payload_bytes, plan_record_bytes, side_data_bytes],
                    "lookback cohort complete cost exceeds u64",
                )?;
                let qualified =
                    super::qualifies_against_independent(complete_cost, independent_cost)?;
                let wins = qualified && complete_cost < best_cost;
                if wins {
                    best_cost = complete_cost;
                }
                push_ranked(
                    &mut candidates,
                    &mut incumbent,
                    CohortCandidate {
                        strategy: CohortStrategy::Lookback {
                            level: *level,
                            lookback: *lookback,
                        },
                        plan,
                        group: Some(group),
                        payload_bytes,
                        plan_record_bytes,
                        side_data_bytes,
                        complete_cost,
                        qualified,
                        selected: false,
                        exclusion: None,
                    },
                    wins,
                );
            }
        }
        if let Some(index) = incumbent {
            candidates[index].selected = true;
        }
        Ok(CohortTrace {
            cohort: cohort.clone(),
            planner_id: planner_id.to_owned(),
            independent,
            dictionary,
            candidates,
            selected: incumbent,
        })
    }

    /// Enumerates the v6 whole-object attempt for one ContentObject, given the
    /// post-v5 Archive state, the current plan table, and whether a region section
    /// has already been emitted by an earlier (lower-digest) object. For the Fast
    /// profile production makes no attempt, so the trace carries no audit.
    pub fn trace_jpeg_object(
        archive: &Archive,
        object: &ContentObject,
        profile: CompressionProfile,
        plans: &BTreeMap<u64, TransformPlan>,
        emitted_region_section: bool,
    ) -> Result<ObjectTrace> {
        let owners = super::chunk_object_owners(archive);
        trace_object_with_owners(
            archive,
            object,
            profile,
            plans,
            emitted_region_section,
            &owners,
        )
    }

    /// Mirrors one iteration of the object loop in `plan_archive_v6`.
    fn trace_object_with_owners(
        archive: &Archive,
        object: &ContentObject,
        profile: CompressionProfile,
        plans: &BTreeMap<u64, TransformPlan>,
        emitted_region_section: bool,
        owners: &BTreeMap<Digest, BTreeSet<Digest>>,
    ) -> Result<ObjectTrace> {
        let chunk_count = u64::try_from(object.chunks.len())
            .map_err(|_| super::resource("JPEG region Chunk count exceeds u64"))?;
        let mut trace = ObjectTrace {
            content_object: object.logical_digest,
            chunk_count,
            logical_bytes: None,
            transformed_bytes: None,
            ordinary_cost: None,
            candidates: Vec::new(),
            selected: None,
            region: None,
            audit: None,
        };
        if profile == CompressionProfile::Fast {
            return Ok(trace);
        }
        if object.chunks.is_empty() {
            trace.audit = Some(ReconstructionAuditReason::NotRecognized);
            return Ok(trace);
        }
        if (profile == CompressionProfile::Balanced && object.chunks.len() != 1)
            || chunk_count > crate::jpeg_reconstruction::MAX_REGION_CHUNKS
        {
            trace.audit = Some(ReconstructionAuditReason::ResourcePolicyExcluded);
            return Ok(trace);
        }
        if object.chunks.iter().any(|chunk_ref| {
            owners
                .get(&chunk_ref.chunk_id)
                .is_some_and(|objects| objects.len() != 1)
        }) || object.chunks.iter().any(|chunk_ref| {
            let chunk = &archive.content_store.chunks[&chunk_ref.chunk_id];
            chunk.group_ref.is_some()
                || plans
                    .get(&chunk.plan_ref)
                    .is_none_or(|plan| plan.dictionary.is_some())
        }) {
            trace.audit = Some(ReconstructionAuditReason::RegionDedupConflict);
            return Ok(trace);
        }

        let logical_bytes = object.chunks.iter().try_fold(0_u64, |total, chunk_ref| {
            total
                .checked_add(archive.content_store.chunks[&chunk_ref.chunk_id].logical_len)
                .ok_or_else(|| super::resource("JPEG ContentObject length overflows"))
        })?;
        trace.logical_bytes = Some(logical_bytes);
        if logical_bytes
            > u64::try_from(crate::jpeg_reconstruction::MAX_JPEG_BYTES).unwrap_or(u64::MAX)
        {
            trace.audit = Some(ReconstructionAuditReason::ResourcePolicyExcluded);
            return Ok(trace);
        }
        let mut original = Vec::with_capacity(
            usize::try_from(logical_bytes)
                .map_err(|_| super::resource("JPEG ContentObject length exceeds usize"))?,
        );
        for chunk_ref in &object.chunks {
            original
                .extend_from_slice(&archive.content_store.chunks[&chunk_ref.chunk_id].plaintext);
        }
        let verified = match crate::jpeg_reconstruction::verified_forward(&original) {
            Ok(value) => value,
            Err(failure) => {
                trace.audit = Some(match failure {
                    crate::jpeg_reconstruction::JpegAttemptFailure::NotRecognized => {
                        ReconstructionAuditReason::NotRecognized
                    }
                    crate::jpeg_reconstruction::JpegAttemptFailure::Unsupported => {
                        ReconstructionAuditReason::Unsupported
                    }
                    crate::jpeg_reconstruction::JpegAttemptFailure::VerificationFailed => {
                        ReconstructionAuditReason::ExactVerificationFailed
                    }
                    crate::jpeg_reconstruction::JpegAttemptFailure::ResourceExcluded => {
                        ReconstructionAuditReason::ResourcePolicyExcluded
                    }
                });
                return Ok(trace);
            }
        };
        let transformed_bytes = u64_len(&verified.bytes, "JPEG XL representation exceeds u64")?;
        trace.transformed_bytes = Some(transformed_bytes);
        let ordinary_cost = super::v6_ordinary_payload_cost(archive, object, plans)?;
        trace.ordinary_cost = Some(ordinary_cost);

        let mut best: Option<(usize, u64, Vec<u8>)> = None;
        for plan in super::jpeg_region_candidates(profile)? {
            let representation = encode_transformed_payload(&plan, &verified.bytes)?;
            let plan_record_bytes = if plans.contains_key(&plan.plan_id) {
                0
            } else {
                encoded_transform_plan_v3_len(&plan)?
            };
            let provisional = ReconstructionRegion {
                region_id: Digest::ZERO,
                content_object: object.logical_digest,
                start_chunk_index: 0,
                chunk_count,
                plan_ref: plan.plan_id,
                logical_bytes,
                transformed_bytes,
                ordinary_physical_bytes: ordinary_cost,
                region_overhead_bytes: 0,
                access: RegionAccessCost {
                    logical_bytes,
                    logical_chunks: chunk_count,
                    worst_reconstructed_bytes: logical_bytes,
                },
                representation: representation.clone().into_boxed_slice(),
            };
            let representation_bytes =
                u64_len(&representation, "JPEG region representation exceeds u64")?;
            let region_record_bytes = encoded_reconstruction_region_len(&provisional)?
                .checked_sub(representation_bytes)
                .ok_or_else(|| super::resource("JPEG region record cost underflows"))?;
            let section_header_bytes = if emitted_region_section {
                0
            } else {
                SECTION_HEADER_LEN
            };
            let complete_cost = checked_sum(
                &[
                    representation_bytes,
                    plan_record_bytes,
                    region_record_bytes,
                    section_header_bytes,
                ],
                "JPEG region cost overflows",
            )?;
            let index = trace.candidates.len();
            let wins = best
                .as_ref()
                .is_none_or(|(_, previous, _)| complete_cost < *previous);
            let exclusion = if wins {
                if let Some((previous, _, _)) = &best {
                    trace.candidates[*previous].exclusion = Some(ExclusionReason::Superseded);
                }
                None
            } else {
                Some(ExclusionReason::NotCheaperThanIncumbent)
            };
            trace.candidates.push(RegionCandidate {
                plan,
                representation_bytes,
                plan_record_bytes,
                region_record_bytes,
                section_header_bytes,
                complete_cost,
                qualified: super::qualifies_reconstruction(complete_cost, ordinary_cost)
                    .unwrap_or(false),
                selected: false,
                exclusion,
            });
            if wins {
                best = Some((index, complete_cost, representation));
            }
        }
        let Some((index, complete_cost, representation)) = best else {
            trace.audit = Some(ReconstructionAuditReason::Unsupported);
            return Ok(trace);
        };
        if !super::qualifies_reconstruction(complete_cost, ordinary_cost)? {
            trace.candidates[index].exclusion = Some(ExclusionReason::InsufficientGain);
            trace.audit = Some(ReconstructionAuditReason::CompleteCostDidNotWin);
            return Ok(trace);
        }
        let representation_bytes =
            u64_len(&representation, "JPEG region representation exceeds u64")?;
        let mut region = ReconstructionRegion {
            region_id: Digest::ZERO,
            content_object: object.logical_digest,
            start_chunk_index: 0,
            chunk_count,
            plan_ref: trace.candidates[index].plan.plan_id,
            logical_bytes,
            transformed_bytes,
            ordinary_physical_bytes: ordinary_cost,
            region_overhead_bytes: complete_cost
                .checked_sub(representation_bytes)
                .ok_or_else(|| super::resource("JPEG region cost underflows"))?,
            access: RegionAccessCost {
                logical_bytes,
                logical_chunks: chunk_count,
                worst_reconstructed_bytes: logical_bytes,
            },
            representation: representation.into_boxed_slice(),
        };
        region.region_id = crate::jpeg_reconstruction::region_identity(&region);
        trace.candidates[index].selected = true;
        trace.selected = Some(index);
        trace.region = Some(region);
        Ok(trace)
    }

    fn chunk_digest_mismatch(chunk_id: Digest) -> Diagnostic {
        Diagnostic::new(
            OutcomeClass::Corrupt,
            ReasonCode::ChunkDigestMismatch,
            chunk_id.to_string(),
        )
    }

    fn unknown_chunk(chunk_id: Digest) -> Diagnostic {
        Diagnostic::new(
            OutcomeClass::Nonconforming,
            ReasonCode::UnknownChunk,
            chunk_id.to_string(),
        )
    }

    /// Archive state after the production Chunk stage of one planner generation.
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub struct ChunkStageTrace {
        pub profile: CompressionProfile,
        pub version: PlannerVersion,
        /// Chunk-stage traces in Chunk digest order.
        pub chunks: Vec<ChunkTrace>,
        /// Copy of the input Archive with the Chunk-stage `plan_ref`s,
        /// ReconstructionData, and fallbacks applied and whole-object state
        /// cleared. For v3 and later, dictionaries, ChunkGroups, and `group_ref`s
        /// are also cleared, exactly as the cohort stage clears them before it
        /// calls `select_cohort_plan`, so this Archive and [`Self::plans`] are
        /// the inputs [`trace_cohort`] and [`select_cohort_plan`] expect.
        pub archive: Archive,
        /// The TransformPlan table the cohort stage receives.
        pub plans: BTreeMap<u64, TransformPlan>,
    }

    /// Enumerates the production Chunk stage of `version` over every Chunk of
    /// `archive` without mutating it, and returns the state the cohort stage
    /// receives.
    pub fn trace_chunk_stage(
        archive: &Archive,
        profile: CompressionProfile,
        version: PlannerVersion,
    ) -> Result<ChunkStageTrace> {
        let mut working = archive.clone();
        working.content_store.reconstruction_data.clear();
        working.content_store.reconstruction_fallbacks.clear();
        working.content_store.reconstruction_regions.clear();
        working.content_store.reconstruction_audits.clear();

        let mut plans = BTreeMap::new();
        let store = store_plan();
        plans.insert(store.plan_id, store);
        let mut chunk_traces = Vec::with_capacity(archive.content_store.chunks.len());
        for chunk in archive.content_store.chunks.values() {
            let length_matches =
                chunk.logical_len == u64::try_from(chunk.plaintext.len()).unwrap_or(u64::MAX);
            if (!version.has_reconstruction_stage() && !length_matches)
                || sha256_exact(&chunk.plaintext) != chunk.chunk_id
            {
                return Err(chunk_digest_mismatch(chunk.chunk_id));
            }
            let trace = trace_chunk_with_id(profile, version, chunk.chunk_id, &chunk.plaintext)?;
            let selected = trace.selected_plan().clone();
            working
                .content_store
                .chunks
                .get_mut(&chunk.chunk_id)
                .ok_or_else(|| unknown_chunk(chunk.chunk_id))?
                .plan_ref = selected.plan_id;
            if version.has_reconstruction_stage() || version == PlannerVersion::V4 {
                super::insert_plan(&mut plans, selected)?;
            } else {
                plans.entry(selected.plan_id).or_insert(selected);
            }
            if let Some(attempt) = &trace.reconstruction {
                match (attempt.fallback, &attempt.data) {
                    (Some(reason), _) => {
                        working
                            .content_store
                            .reconstruction_fallbacks
                            .insert(chunk.chunk_id, reason);
                    }
                    (None, Some(data)) => {
                        working
                            .content_store
                            .reconstruction_data
                            .entry(data.reconstruction_id)
                            .or_insert_with(|| data.clone());
                    }
                    (None, None) => {}
                }
            }
            chunk_traces.push(trace);
        }
        if version.has_cohort_stage() {
            working.content_store.dictionaries.clear();
            working.content_store.chunk_groups.clear();
            for chunk in working.content_store.chunks.values_mut() {
                chunk.group_ref = None;
            }
        }
        Ok(ChunkStageTrace {
            profile,
            version,
            chunks: chunk_traces,
            archive: working,
            plans,
        })
    }

    /// Reproduces a complete production planning run of `version` over `archive`
    /// without mutating it: the Chunk stage, the cohort stage (v3+), the v5
    /// fallback/retention finish, and the v6 whole-object stage. Every stage
    /// decision is taken from the enumerations above, and the resulting
    /// [`PlannedAssignment`] can be compared with a real planner run.
    pub fn trace_archive(
        archive: &Archive,
        profile: CompressionProfile,
        version: PlannerVersion,
    ) -> Result<ArchiveTrace> {
        let stage_planner_id = version.stage_planner_id(profile);
        let ChunkStageTrace {
            chunks: chunk_traces,
            archive: mut working,
            mut plans,
            ..
        } = trace_chunk_stage(archive, profile, version)?;

        let mut cohort_traces = Vec::new();
        if version.has_cohort_stage() {
            let mut cohorts = cluster(&working.content_store.chunks, profile.similarity_policy());
            cohorts.sort_by_key(|cohort| cohort.cohort_id);
            let mut physically_ordered = BTreeSet::new();
            let mut physical_order = Vec::with_capacity(working.content_store.chunks.len());
            for cohort in &cohorts {
                let trace = trace_cohort(&working, cohort, profile, stage_planner_id, &plans)?;
                match trace.selection() {
                    CohortSelection::Independent => {}
                    CohortSelection::Dictionary { dictionary, plan } => {
                        super::insert_dictionary(
                            &mut working.content_store.dictionaries,
                            dictionary,
                        )?;
                        super::insert_plan(&mut plans, plan.clone())?;
                        for chunk_id in &cohort.chunks {
                            working
                                .content_store
                                .chunks
                                .get_mut(chunk_id)
                                .ok_or_else(|| unknown_chunk(*chunk_id))?
                                .plan_ref = plan.plan_id;
                        }
                    }
                    CohortSelection::Lookback { group, plan } => {
                        super::insert_plan(&mut plans, plan.clone())?;
                        for chunk_id in &cohort.chunks {
                            let chunk = working
                                .content_store
                                .chunks
                                .get_mut(chunk_id)
                                .ok_or_else(|| unknown_chunk(*chunk_id))?;
                            chunk.plan_ref = plan.plan_id;
                            chunk.group_ref = Some(group.group_id);
                        }
                        working
                            .content_store
                            .chunk_groups
                            .insert(group.group_id, group);
                    }
                }
                for chunk_id in &cohort.chunks {
                    if physically_ordered.insert(*chunk_id) {
                        physical_order.push(*chunk_id);
                    }
                }
                cohort_traces.push(trace);
            }
            for chunk_id in working.content_store.chunks.keys() {
                if physically_ordered.insert(*chunk_id) {
                    physical_order.push(*chunk_id);
                }
            }
            working.content_store.physical_order = physical_order.into_boxed_slice();

            if stage_planner_id.ends_with("-v5") {
                for chunk in working.content_store.chunks.values() {
                    let plan = plans
                        .get(&chunk.plan_ref)
                        .ok_or_else(|| unknown_plan(chunk.plan_ref))?;
                    if is_reconstructive_plan(plan) {
                        working
                            .content_store
                            .reconstruction_fallbacks
                            .remove(&chunk.chunk_id);
                    } else if profile != CompressionProfile::Fast {
                        working
                            .content_store
                            .reconstruction_fallbacks
                            .entry(chunk.chunk_id)
                            .or_insert(ReconstructionFallbackReason::CompleteCostDidNotWin);
                    }
                }
                let used_plans = working
                    .content_store
                    .chunks
                    .values()
                    .map(|chunk| chunk.plan_ref)
                    .collect::<BTreeSet<_>>();
                plans.retain(|plan_id, plan| {
                    used_plans.contains(plan_id) || !is_reconstructive_plan(plan)
                });
                let used_reconstruction = plans
                    .values()
                    .flat_map(|plan| plan.transforms.iter())
                    .filter_map(|step| step.reconstruction_ref)
                    .collect::<BTreeSet<_>>();
                working
                    .content_store
                    .reconstruction_data
                    .retain(|identity, _| used_reconstruction.contains(identity));
            }
        }

        let mut object_traces = Vec::new();
        let mut regions = BTreeMap::new();
        let mut audits = BTreeMap::new();
        if version == PlannerVersion::V6 && profile != CompressionProfile::Fast {
            let owners = super::chunk_object_owners(&working);
            let objects = working
                .content_store
                .objects
                .values()
                .cloned()
                .collect::<Vec<_>>();
            let mut emitted_region_section = false;
            for object in &objects {
                let trace = trace_object_with_owners(
                    &working,
                    object,
                    profile,
                    &plans,
                    emitted_region_section,
                    &owners,
                )?;
                match (&trace.region, trace.selected, trace.audit) {
                    (Some(region), Some(index), _) => {
                        let plan = trace.candidates[index].plan.clone();
                        plans.entry(plan.plan_id).or_insert(plan);
                        for chunk_ref in &object.chunks {
                            let chunk = working
                                .content_store
                                .chunks
                                .get_mut(&chunk_ref.chunk_id)
                                .ok_or_else(|| unknown_chunk(chunk_ref.chunk_id))?;
                            chunk.plan_ref = crate::jpeg_reconstruction::REGION_MEMBER_PLAN_REF;
                            chunk.group_ref = None;
                            working
                                .content_store
                                .reconstruction_fallbacks
                                .remove(&chunk_ref.chunk_id);
                        }
                        regions.insert(region.region_id, region.clone());
                        emitted_region_section = true;
                    }
                    (_, _, Some(reason)) => {
                        audits.insert(
                            ReconstructionAuditTarget::ContentObject(object.logical_digest),
                            reason,
                        );
                    }
                    _ => {}
                }
                object_traces.push(trace);
            }
        }

        let assignment = PlannedAssignment {
            planner_id: version.planner_id(profile).to_owned(),
            chunks: working
                .content_store
                .chunks
                .iter()
                .map(|(chunk_id, chunk)| {
                    (
                        *chunk_id,
                        ChunkAssignment {
                            plan_ref: chunk.plan_ref,
                            group_ref: chunk.group_ref,
                        },
                    )
                })
                .collect(),
            transform_plans: plans,
            dictionaries: working.content_store.dictionaries,
            chunk_groups: working.content_store.chunk_groups,
            reconstruction_data: working.content_store.reconstruction_data,
            reconstruction_fallbacks: working.content_store.reconstruction_fallbacks,
            reconstruction_regions: regions,
            reconstruction_audits: audits,
            physical_order: working.content_store.physical_order,
        };
        Ok(ArchiveTrace {
            profile,
            version,
            chunks: chunk_traces,
            cohorts: cohort_traces,
            objects: object_traces,
            assignment,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write as _;

    use flate2::{Compression, write::GzEncoder};

    use super::*;

    #[test]
    fn profiles_expose_v6_and_preserve_frozen_historical_identifiers() {
        assert_eq!(CompressionProfile::default(), CompressionProfile::Balanced);
        assert_eq!(CompressionProfile::Fast.planner_id(), "fast-v6");
        assert_eq!(CompressionProfile::Balanced.planner_id(), "balanced-v6");
        assert_eq!(CompressionProfile::Dense.planner_id(), "dense-v6");
        assert_eq!(CompressionProfile::Extreme.planner_id(), "extreme-v6");
        assert_eq!(CompressionProfile::Fast.planner_v5_id(), "fast-v5");
        assert_eq!(CompressionProfile::Balanced.planner_v5_id(), "balanced-v5");
        assert_eq!(CompressionProfile::Dense.planner_v5_id(), "dense-v5");
        assert_eq!(CompressionProfile::Extreme.planner_v5_id(), "extreme-v5");
        assert_eq!(CompressionProfile::Fast.planner_v4_id(), "fast-v4");
        assert_eq!(CompressionProfile::Balanced.planner_v4_id(), "balanced-v4");
        assert_eq!(CompressionProfile::Dense.planner_v4_id(), "dense-v4");
        assert_eq!(CompressionProfile::Extreme.planner_v4_id(), "extreme-v4");
        assert_eq!(CompressionProfile::Fast.planner_v3_id(), "fast-v3");
        assert_eq!(CompressionProfile::Balanced.planner_v3_id(), "balanced-v3");
        assert_eq!(CompressionProfile::Dense.planner_v3_id(), "dense-v3");
        assert_eq!(CompressionProfile::Extreme.planner_v3_id(), "extreme-v3");
        assert_eq!(CompressionProfile::Fast.planner_v2_id(), "fast-v2");
        assert_eq!(CompressionProfile::Balanced.planner_v2_id(), "balanced-v2");
        assert_eq!(CompressionProfile::Dense.planner_v2_id(), "dense-v2");
        assert_eq!(CompressionProfile::Extreme.planner_v2_id(), "extreme-v2");
        assert_eq!(CompressionProfile::Fast.planner_v1_id(), "fast-v1");
        assert_eq!(CompressionProfile::Balanced.planner_v1_id(), "balanced-v1");
        assert_eq!(CompressionProfile::Dense.planner_v1_id(), "dense-v1");
        assert_eq!(CompressionProfile::Extreme.planner_v1_id(), "extreme-v1");
        assert_eq!(CompressionProfile::Fast.chunking_candidates(), &[FAST_V2]);
        assert_eq!(
            CompressionProfile::Balanced.chunking_candidates(),
            &[BALANCED_V2]
        );
        assert_eq!(
            CompressionProfile::Dense.chunking_candidates(),
            &[BALANCED_V2, DENSE_V2]
        );
        assert_eq!(
            CompressionProfile::Extreme.chunking_candidates(),
            &[FAST_V2, BALANCED_V2, DENSE_V2, EXTREME_V2]
        );
    }

    #[test]
    fn probe_distinguishes_repetition_from_deterministic_noise() {
        let empty = analyze(&[]);
        let repetitive = analyze(&vec![7; 8192]);
        let noise = analyze(&deterministic_noise(8192));
        assert!(empty.empty);
        assert!(!empty.likely_compressible);
        assert!(repetitive.likely_compressible);
        assert!(!noise.likely_compressible);
    }

    #[test]
    fn verified_deflate_candidate_can_beat_complete_ordinary_cost() {
        let source = (0..20_000)
            .flat_map(|index| {
                format!(
                    "row={index:06};value={:08x};category={}\n",
                    index * 17,
                    index % 31
                )
                .into_bytes()
            })
            .collect::<Vec<_>>();
        let mut encoder = GzEncoder::new(Vec::new(), Compression::new(6));
        encoder.write_all(&source).unwrap();
        let original = encoder.finish().unwrap();
        let candidate = crate::reconstruction::try_forward(&original, 2_048)
            .unwrap()
            .expect("eligible complete gzip stream");
        let ordinary = select_v4_plan(CompressionProfile::Dense, &original).unwrap();
        let baseline = v5_independent_cost(&ordinary, &original, None).unwrap();
        let values = BTreeMap::from([(candidate.data.reconstruction_id, candidate.data.clone())]);
        let best = reconstruction_candidates(CompressionProfile::Dense, &candidate.data)
            .unwrap()
            .into_iter()
            .map(|plan| {
                v5_independent_cost(&plan, &original, Some((&candidate.data, &values))).unwrap()
            })
            .min()
            .unwrap();
        assert!(
            qualifies_reconstruction(best, baseline).unwrap(),
            "best={best}, baseline={baseline}"
        );
    }

    #[test]
    fn minimum_gain_includes_overhead_and_prefers_store_on_ties() {
        assert!(!zstandard_wins(100, 52).unwrap());
        assert!(zstandard_wins(100, 51).unwrap());
        assert!(!zstandard_wins(49, 1).unwrap());
    }

    #[test]
    fn v4_measured_candidates_cover_store_speed_density_and_transforms() {
        let noise = deterministic_noise(16 * 1024);
        let store = select_v4_plan(CompressionProfile::Extreme, &noise).unwrap();
        assert_eq!(store.codec, crate::identity::STORE_CODEC_IDENTIFIER);

        let small_repetition = vec![b'x'; 512];
        let speed = select_v4_plan(CompressionProfile::Fast, &small_repetition).unwrap();
        assert_eq!(speed.codec, LZ4_CODEC_IDENTIFIER, "selected {speed:?}");

        let numeric = (0_u32..65_536)
            .flat_map(u32::to_le_bytes)
            .collect::<Vec<_>>();
        let transformed = select_v4_plan(CompressionProfile::Dense, &numeric).unwrap();
        assert!(
            !transformed.transforms.is_empty(),
            "selected {transformed:?}"
        );

        let text = (0..8_192)
            .flat_map(|index| format!("record {index:06}: alpha beta gamma delta\n").into_bytes())
            .collect::<Vec<_>>();
        let density = select_v4_plan(CompressionProfile::Extreme, &text).unwrap();
        assert!(
            matches!(
                density.codec.as_str(),
                ZSTD_CODEC_IDENTIFIER | LZMA2_CODEC_IDENTIFIER
            ),
            "selected {density:?}"
        );

        let block = deterministic_noise(1_100_000);
        let long_distance = [block.as_slice(), block.as_slice()].concat();
        let density_specialist = select_v4_plan(CompressionProfile::Dense, &long_distance).unwrap();
        assert_eq!(
            density_specialist.codec, LZMA2_CODEC_IDENTIFIER,
            "selected {density_specialist:?}"
        );
    }

    fn deterministic_noise(len: usize) -> Vec<u8> {
        let mut state = 0x4d59_5df4_d0f3_3173_u64;
        (0..len)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                state as u8
            })
            .collect()
    }
}
