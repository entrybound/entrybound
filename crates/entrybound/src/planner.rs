//! Deterministic creation-time compression planning.
//!
//! Profiles exist only here. The native reader uses recorded TransformPlans
//! and operational codecs without consulting this module.

use std::collections::BTreeMap;
use std::str::FromStr;

use crate::chunker::{BALANCED_V2, ChunkingParameters, DENSE_V2, EXTREME_V2, FAST_V2};
use crate::codec::{
    ZSTD_DICTIONARY_CONSTRUCTION_PREFIX, ZSTD_DICTIONARY_FORMAT, ZSTD_WINDOW_BYTES,
    aggregate_archive_decode_requirements, aggregate_decode_requirements, encode_payload,
    encode_payload_with_dictionary, encode_payload_with_prefix, store_plan, train_dictionary,
    zstd_dictionary_plan, zstd_plan, zstd_prefix_plan,
};
use crate::diagnostics::{Diagnostic, OutcomeClass, ReasonCode, Result};
use crate::eam::{Archive, ChunkGroup, Dictionary, Digest, TransformPlan};
use crate::ecf::{
    FEATURE_CROSS_FILE_COMPRESSION_V1, encoded_chunk_group_len, encoded_dictionary_len,
    encoded_transform_plan_len,
};
use crate::identity::sha256_exact;
use crate::similarity::{
    BALANCED_V3_SIMILARITY, DENSE_V3_SIMILARITY, EXTREME_V3_SIMILARITY, FAST_V3_SIMILARITY,
    SimilarityCohort, SimilarityPolicy, cluster,
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

const FAST_LEVELS: [i32; 1] = [1];
const BALANCED_LEVELS: [i32; 3] = [1, 3, 5];
const BALANCED_FALLBACK_LEVELS: [i32; 1] = [1];
const DENSE_LEVELS: [i32; 3] = [5, 9, 15];
const EXTREME_LEVELS: [i32; 4] = [9, 15, 19, 22];

/// Public creation profiles. New filesystem archives use their v3 IDs.
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
            "fast-v1" | "fast-v2" | "fast-v3" => Some(Self::Fast),
            "balanced-v1" | "balanced-v2" | "balanced-v3" => Some(Self::Balanced),
            "dense-v1" | "dense-v2" | "dense-v3" => Some(Self::Dense),
            "extreme-v1" | "extreme-v2" | "extreme-v3" => Some(Self::Extreme),
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
    pub dictionary_chunks: u64,
    pub lookback_chunks: u64,
    pub similarity_cohorts: u64,
    pub selected_plans: Box<[TransformPlan]>,
}

/// Frozen v1 codec selection for callers constructing historical fixed chunks.
pub fn plan_archive(archive: &mut Archive, profile: CompressionProfile) -> Result<PlanningReport> {
    plan_archive_with_id(archive, profile, profile.planner_v1_id())
}

/// V2 codec selection after CDC and archive-wide exact deduplication.
pub fn plan_archive_v2(
    archive: &mut Archive,
    profile: CompressionProfile,
) -> Result<PlanningReport> {
    plan_archive_with_id(archive, profile, profile.planner_v2_id())
}

/// V3 planning adds deterministic cohorts, shared dictionaries, bounded
/// lookback groups, and an explicit physical Chunk order.
pub fn plan_archive_v3(
    archive: &mut Archive,
    profile: CompressionProfile,
) -> Result<PlanningReport> {
    let mut report = plan_archive_with_id(archive, profile, profile.planner_id())?;
    archive.descriptor.features.incompat |= FEATURE_CROSS_FILE_COMPRESSION_V1;
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
        match select_cohort_plan(archive, cohort, profile, &plans)? {
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
    archive.transform_plans = plans.into_values().collect::<Vec<_>>().into_boxed_slice();
    archive.descriptor.decode = aggregate_archive_decode_requirements(
        &archive.transform_plans,
        &archive.content_store.dictionaries,
        &archive.content_store.chunk_groups,
    )?;
    report.dictionary_chunks = dictionary_chunks;
    report.lookback_chunks = lookback_chunks;
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
        dictionary_chunks: 0,
        lookback_chunks: 0,
        similarity_cohorts: 0,
        selected_plans,
    })
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
    plans: &BTreeMap<u64, TransformPlan>,
) -> Result<CohortSelection> {
    let independent_cost = cohort.chunks.iter().try_fold(0_u64, |total, chunk_id| {
        let chunk = &archive.content_store.chunks[chunk_id];
        let plan = plans.get(&chunk.plan_ref).ok_or_else(|| {
            Diagnostic::new(
                OutcomeClass::Unsupported,
                ReasonCode::UnknownTransformPlan,
                chunk.plan_ref.to_string(),
            )
        })?;
        let encoded = encode_payload(plan, &chunk.plaintext)?;
        total
            .checked_add(
                u64::try_from(encoded.len())
                    .map_err(|_| resource("independent cohort payload length exceeds u64"))?,
            )
            .ok_or_else(|| resource("independent cohort cost exceeds u64"))
    })?;
    let mut best_cost = independent_cost;
    let mut best = CohortSelection::Independent;

    if let Some(dictionary) = train_cohort_dictionary(archive, cohort, profile)? {
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
            profile.planner_id(),
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
    plaintext: &[u8],
) -> Result<usize> {
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
    use super::*;

    #[test]
    fn profiles_expose_v3_and_preserve_frozen_v1_v2_identifiers() {
        assert_eq!(CompressionProfile::default(), CompressionProfile::Balanced);
        assert_eq!(CompressionProfile::Fast.planner_id(), "fast-v3");
        assert_eq!(CompressionProfile::Balanced.planner_id(), "balanced-v3");
        assert_eq!(CompressionProfile::Dense.planner_id(), "dense-v3");
        assert_eq!(CompressionProfile::Extreme.planner_id(), "extreme-v3");
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
    fn minimum_gain_includes_overhead_and_prefers_store_on_ties() {
        assert!(!zstandard_wins(100, 52).unwrap());
        assert!(zstandard_wins(100, 51).unwrap());
        assert!(!zstandard_wins(49, 1).unwrap());
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
