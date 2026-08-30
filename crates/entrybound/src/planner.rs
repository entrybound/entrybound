//! Deterministic creation-time compression planning.
//!
//! Profiles exist only here. The native reader uses recorded TransformPlans
//! and operational codecs without consulting this module.

use std::collections::BTreeMap;
use std::str::FromStr;

use crate::chunker::{BALANCED_V2, ChunkingParameters, DENSE_V2, EXTREME_V2, FAST_V2};
use crate::codec::{
    LZ4_CODEC_IDENTIFIER, LZMA2_CODEC_IDENTIFIER, ZSTD_CODEC_IDENTIFIER,
    ZSTD_DICTIONARY_CONSTRUCTION_PREFIX, ZSTD_DICTIONARY_FORMAT, ZSTD_WINDOW_BYTES,
    aggregate_archive_decode_requirements, aggregate_decode_requirements, encode_payload,
    encode_payload_with_dictionary, encode_payload_with_prefix, encode_payload_with_reconstruction,
    lz4_plan, lzma2_plan, store_plan, train_dictionary, zstd_dictionary_plan, zstd_plan,
    zstd_prefix_plan, zstd_transformed_plan,
};
use crate::diagnostics::{Diagnostic, OutcomeClass, ReasonCode, Result};
use crate::eam::{
    Archive, ChunkGroup, Dictionary, Digest, ReconstructionData, ReconstructionFallbackReason,
    TransformPlan,
};
use crate::ecf::{
    FEATURE_CODEC_TRANSFORM_V1, FEATURE_CROSS_FILE_COMPRESSION_V1,
    FEATURE_RECONSTRUCTIVE_TRANSFORM_V1, encoded_chunk_group_len, encoded_dictionary_len,
    encoded_reconstruction_data_len, encoded_transform_plan_len, encoded_transform_plan_v2_len,
};
use crate::identity::sha256_exact;
use crate::similarity::{
    BALANCED_V3_SIMILARITY, DENSE_V3_SIMILARITY, EXTREME_V3_SIMILARITY, FAST_V3_SIMILARITY,
    SimilarityCohort, SimilarityPolicy, cluster,
};
use crate::transform::{byte_shuffle_step, deflate_reconstruct_step, delta8_step};

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

/// Public creation profiles. New filesystem archives use their v5 IDs.
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
            "fast-v1" | "fast-v2" | "fast-v3" | "fast-v4" | "fast-v5" => Some(Self::Fast),
            "balanced-v1" | "balanced-v2" | "balanced-v3" | "balanced-v4" | "balanced-v5" => {
                Some(Self::Balanced)
            }
            "dense-v1" | "dense-v2" | "dense-v3" | "dense-v4" | "dense-v5" => Some(Self::Dense),
            "extreme-v1" | "extreme-v2" | "extreme-v3" | "extreme-v4" | "extreme-v5" => {
                Some(Self::Extreme)
            }
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
    let planner_id = profile.planner_id();
    let report = plan_archive_v5_independent(archive, profile, planner_id)?;
    let mut features = FEATURE_CROSS_FILE_COMPRESSION_V1 | FEATURE_CODEC_TRANSFORM_V1;
    if !archive.content_store.reconstruction_data.is_empty()
        || !archive.content_store.reconstruction_fallbacks.is_empty()
    {
        features |= FEATURE_RECONSTRUCTIVE_TRANSFORM_V1;
    }
    finish_cross_file_planning(archive, profile, planner_id, features, report)
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

#[cfg(test)]
mod tests {
    use std::io::Write as _;

    use flate2::{Compression, write::GzEncoder};

    use super::*;

    #[test]
    fn profiles_expose_v5_and_preserve_frozen_historical_identifiers() {
        assert_eq!(CompressionProfile::default(), CompressionProfile::Balanced);
        assert_eq!(CompressionProfile::Fast.planner_id(), "fast-v5");
        assert_eq!(CompressionProfile::Balanced.planner_id(), "balanced-v5");
        assert_eq!(CompressionProfile::Dense.planner_id(), "dense-v5");
        assert_eq!(CompressionProfile::Extreme.planner_id(), "extreme-v5");
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
