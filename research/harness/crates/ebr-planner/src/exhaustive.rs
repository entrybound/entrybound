//! Exhaustive candidate search over a widened universe, plus the regret that
//! search finds against production's real choice.
//!
//! `entrybound::research::planner::trace_chunk`/`trace_cohort` (see
//! [`crate::check_enumeration_matches_real_planner`] and
//! `research/harness/research-internals.md`) enumerate exactly the
//! candidates production's own frozen, profile-restricted lists build --
//! "the enumeration mirrors loops, not costs" and never invents a candidate
//! production would not have tried. This module is the complement: it builds
//! a search space that is a strict *superset* of every candidate any
//! `CompressionProfile`/`PlannerVersion` combination ever tries (every
//! `entrybound::research::codec::SUPPORTED_LEVELS` Zstandard level and every
//! `SUPPORTED_LZMA2_CONFIGURATIONS` LZMA2 configuration, crossed with every
//! `TransformKind`, for chunks; every `SUPPORTED_LEVELS` dictionary/lookback
//! level and every `SUPPORTED_LOOKBACKS` lookback distance, for cohorts) and
//! costs every candidate with the exact same production accounting
//! primitives `trace_chunk`/`trace_cohort` use
//! (`entrybound::research::planner::v5_independent_cost`,
//! `encoded_transform_plan_v2_len`, `encoded_transform_plan_len`,
//! `encoded_dictionary_len`, `encoded_chunk_group_len`, plus the
//! `*_SECTION_OVERHEAD_BYTES` constants). Because production's real choice is
//! always one exact member of this wider universe (excluding v5 DEFLATE
//! reconstruction candidates -- see "Scope" below), the minimum this module
//! finds can never cost more than what production chose. That is "regret":
//! `production's chosen complete cost - this module's optimal complete cost`,
//! always `>= 0` within scope.
//!
//! Every search takes an explicit, recorded [`SearchCaps`]/[`CohortSearchCaps`]
//! so a caller -- or a JSONL/CSV row -- always states exactly how wide the
//! search was, per the harness rule that any bound must be "explicit ... and
//! recorded", not silently assumed.
//!
//! ## Scope
//!
//! - Chunk-level search omits v5 DEFLATE-reconstruction candidates
//!   (`entrybound::research::planner::reconstruction_candidates`, which embed
//!   a `deflate_reconstruct_step`): those are a different shape than
//!   codec x transform x level and only apply to inputs that already carry a
//!   recognizable DEFLATE/gzip stream. A chunk whose real planner run selected
//!   a reconstructive candidate is out of scope for the "never exceeds
//!   production" guarantee; this crate's tests use synthetic, non-gzip inputs
//!   so that case never arises.
//! - Cohort-level search omits JPEG region candidates
//!   (`entrybound::research::planner::trace_jpeg_object`), which are a
//!   whole-object, not a per-chunk-group, mechanism.

use std::collections::BTreeSet;

use entrybound::diagnostics::Diagnostic;
use entrybound::eam::{Archive, ChunkGroup, Digest, TransformPlan, TransformStep};
use entrybound::identity::sha256_exact;
use entrybound::planner::CompressionProfile;
use entrybound::research::codec::{
    SUPPORTED_LEVELS, SUPPORTED_LOOKBACKS, SUPPORTED_LZMA2_CONFIGURATIONS, encode_payload,
    encode_payload_with_dictionary, encode_payload_with_prefix, lz4_plan, lzma2_plan, store_plan,
    zstd_dictionary_plan, zstd_plan, zstd_prefix_plan, zstd_transformed_plan,
};
use entrybound::research::ecf::{
    encoded_chunk_group_len, encoded_dictionary_len, encoded_transform_plan_len,
    encoded_transform_plan_v2_len,
};
use entrybound::research::planner::{
    CHUNK_GROUP_SECTION_OVERHEAD_BYTES, DICTIONARY_SECTION_OVERHEAD_BYTES, chunk_group_id,
    cohort_prefix, maximum_preceding_bytes, train_cohort_dictionary, v5_independent_cost,
};
use entrybound::research::transform::{byte_shuffle_step, delta8_step};
use entrybound::similarity::SimilarityCohort;
use serde::Serialize;
use std::collections::BTreeMap;

// ---------------------------------------------------------------------
// Candidate shapes: the codec x transform universe.
// ---------------------------------------------------------------------

/// A structural transform tried before the codec, or none. Production only
/// ever pairs ONE transform with ONE level per profile
/// (`entrybound::planner::v4_candidates`: `Delta8` or one `ByteShuffle` width
/// at a single frozen level); [`TransformKind::ALL`] tries every width at
/// every level this module scans.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum TransformKind {
    None,
    Delta8,
    ByteShuffle(u8),
}

impl TransformKind {
    /// Every transform kind this module knows how to build. `ByteShuffle`
    /// widths are the same three production ever uses
    /// (`append_dense_transform_candidates`); this module just does not
    /// restrict which level or codec they pair with.
    pub const ALL: [Self; 5] = [
        Self::None,
        Self::Delta8,
        Self::ByteShuffle(2),
        Self::ByteShuffle(4),
        Self::ByteShuffle(8),
    ];

    fn step(self) -> Result<Option<TransformStep>, Diagnostic> {
        Ok(match self {
            Self::None => None,
            Self::Delta8 => Some(delta8_step()),
            Self::ByteShuffle(width) => Some(byte_shuffle_step(width)?),
        })
    }
}

/// One codec family and its parameters, independent of any transform.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum CodecChoice {
    Store,
    Lz4,
    Zstd { level: i32 },
    Lzma2 { preset: u8, dictionary_bytes: u32 },
}

impl CodecChoice {
    fn plan(self, step: Option<TransformStep>) -> Result<TransformPlan, Diagnostic> {
        match self {
            Self::Store => Ok(store_plan()),
            Self::Lz4 => lz4_plan(step.into_iter().collect::<Vec<_>>().into_boxed_slice()),
            Self::Zstd { level } => match step {
                Some(step) => zstd_transformed_plan(level, vec![step].into()),
                None => zstd_plan(level),
            },
            Self::Lzma2 {
                preset,
                dictionary_bytes,
            } => lzma2_plan(
                preset,
                dictionary_bytes,
                step.into_iter().collect::<Vec<_>>().into_boxed_slice(),
            ),
        }
    }
}

/// Explicit, recorded bounds on the chunk-level search: which levels,
/// configurations, and transforms it crosses, and a hard cap on how many
/// candidates it will ever build (a truncated search still returns its best
/// finding, but flags [`ChunkExhaustiveSearch::truncated`]).
#[derive(Clone, Debug, Serialize)]
pub struct SearchCaps {
    pub zstd_levels: Vec<i32>,
    pub lzma2_configs: Vec<(u8, u32)>,
    pub transforms: Vec<TransformKind>,
    pub max_candidates: usize,
}

impl SearchCaps {
    /// The full universe this module builds: every
    /// `entrybound::research::codec::SUPPORTED_LEVELS` Zstandard level, every
    /// `SUPPORTED_LZMA2_CONFIGURATIONS` LZMA2 configuration, STORE, LZ4, and
    /// every [`TransformKind`] crossed with every Zstandard level and LZMA2
    /// configuration -- `2 + 7 + 3 + 4 * (7 + 3) = 52` candidates, well under
    /// `max_candidates`. This is a strict superset of any one profile's
    /// `entrybound::planner::v4_candidates` (see the module docs).
    #[must_use]
    pub fn full() -> Self {
        Self {
            zstd_levels: SUPPORTED_LEVELS.to_vec(),
            lzma2_configs: SUPPORTED_LZMA2_CONFIGURATIONS.to_vec(),
            transforms: TransformKind::ALL.to_vec(),
            max_candidates: 256,
        }
    }
}

/// Every (codec, transform) shape `caps` describes, in a fixed, deterministic
/// order (STORE, LZ4, Zstandard levels ascending, LZMA2 configurations in
/// declaration order, then each non-`None` transform crossed with Zstandard
/// levels then LZMA2 configurations). [`search_chunk`] costs all of them
/// (bounded by `max_candidates`); [`crate::strategies::select_greedy`] walks
/// them in this same order and can stop early.
pub(crate) fn candidate_shapes(caps: &SearchCaps) -> Vec<(CodecChoice, TransformKind)> {
    let mut shapes = vec![
        (CodecChoice::Store, TransformKind::None),
        (CodecChoice::Lz4, TransformKind::None),
    ];
    for level in caps.zstd_levels.iter().copied() {
        shapes.push((CodecChoice::Zstd { level }, TransformKind::None));
    }
    for (preset, dictionary_bytes) in caps.lzma2_configs.iter().copied() {
        shapes.push((
            CodecChoice::Lzma2 {
                preset,
                dictionary_bytes,
            },
            TransformKind::None,
        ));
    }
    for transform in caps.transforms.iter().copied() {
        if transform == TransformKind::None {
            continue;
        }
        for level in caps.zstd_levels.iter().copied() {
            shapes.push((CodecChoice::Zstd { level }, transform));
        }
        for (preset, dictionary_bytes) in caps.lzma2_configs.iter().copied() {
            shapes.push((
                CodecChoice::Lzma2 {
                    preset,
                    dictionary_bytes,
                },
                transform,
            ));
        }
    }
    shapes
}

/// One priced candidate: `payload_bytes + plan_record_bytes` costed exactly
/// as `entrybound::research::planner::v5_independent_cost` costs a chunk
/// plan (no side data -- chunk-level candidates never carry reconstruction,
/// dictionary, or group side data).
#[derive(Clone, Debug, Serialize)]
pub struct ExhaustiveCandidate {
    pub codec: CodecChoice,
    pub transform: TransformKind,
    pub payload_bytes: u64,
    pub plan_record_bytes: u64,
    pub complete_cost: u64,
}

pub(crate) fn cost_candidate(
    codec: CodecChoice,
    transform: TransformKind,
    plaintext: &[u8],
) -> Result<ExhaustiveCandidate, Diagnostic> {
    let step = transform.step()?;
    let plan = codec.plan(step)?;
    let payload = encode_payload(&plan, plaintext)?;
    let payload_bytes =
        u64::try_from(payload.len()).expect("tractable-sample payload length fits in u64");
    let plan_record_bytes = encoded_transform_plan_v2_len(&plan)?;
    let complete_cost = payload_bytes
        .checked_add(plan_record_bytes)
        .expect("tractable-sample candidate cost fits in u64");
    Ok(ExhaustiveCandidate {
        codec,
        transform,
        payload_bytes,
        plan_record_bytes,
        complete_cost,
    })
}

/// The complete chunk-level search: every candidate `caps` describes
/// (truncated to `caps.max_candidates`, in the fixed order
/// [`candidate_shapes`] returns), plus the cheapest one found.
#[derive(Clone, Debug, Serialize)]
pub struct ChunkExhaustiveSearch {
    pub logical_len: u64,
    pub caps: SearchCaps,
    pub candidates: Vec<ExhaustiveCandidate>,
    /// Index into `candidates` of the cheapest candidate found.
    pub optimal: usize,
    pub optimal_cost: u64,
    /// True when `caps.max_candidates` cut the search short of the full
    /// cross product `caps` describes.
    pub truncated: bool,
}

/// Runs the full chunk-level exhaustive search described by `caps` over one
/// chunk's plaintext.
pub fn search_chunk(
    plaintext: &[u8],
    caps: &SearchCaps,
) -> Result<ChunkExhaustiveSearch, Diagnostic> {
    let logical_len =
        u64::try_from(plaintext.len()).expect("tractable-sample plaintext length fits in u64");
    let mut shapes = candidate_shapes(caps);
    let truncated = shapes.len() > caps.max_candidates;
    shapes.truncate(caps.max_candidates);

    let mut candidates = Vec::with_capacity(shapes.len());
    for (codec, transform) in shapes {
        candidates.push(cost_candidate(codec, transform, plaintext)?);
    }

    let (optimal, optimal_cost) = candidates
        .iter()
        .enumerate()
        .min_by_key(|(_, candidate)| candidate.complete_cost)
        .map(|(index, candidate)| (index, candidate.complete_cost))
        .expect("STORE is always the first candidate shape");

    Ok(ChunkExhaustiveSearch {
        logical_len,
        caps: caps.clone(),
        candidates,
        optimal,
        optimal_cost,
        truncated,
    })
}

/// `production's chosen complete cost - search.optimal_cost`, both costed
/// with `v5_independent_cost` so the comparison is exact regardless of which
/// internal formula production's own trace used to pick `selected_plan`
/// (see the module docs' "Scope"). `>= 0` whenever `selected_plan` is not a
/// v5 DEFLATE-reconstruction plan.
pub fn chunk_regret(
    selected_plan: &TransformPlan,
    plaintext: &[u8],
    search: &ChunkExhaustiveSearch,
) -> Result<i128, Diagnostic> {
    let selected_cost = v5_independent_cost(selected_plan, plaintext, None)?;
    Ok(i128::from(selected_cost) - i128::from(search.optimal_cost))
}

// ---------------------------------------------------------------------
// Cohort-level search: dictionary x level, lookback x level.
// ---------------------------------------------------------------------

/// Which cross-chunk strategy one cohort candidate represents.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum CohortStrategyKind {
    Independent,
    Dictionary { level: i32 },
    Lookback { level: i32, lookback: u32 },
}

/// Explicit, recorded bounds on the cohort-level search: every level and
/// lookback distance it crosses.
#[derive(Clone, Debug, Serialize)]
pub struct CohortSearchCaps {
    pub levels: Vec<i32>,
    pub lookbacks: Vec<u32>,
}

impl CohortSearchCaps {
    /// The full universe: every `SUPPORTED_LEVELS` level (for both dictionary
    /// and lookback candidates) crossed with every `SUPPORTED_LOOKBACKS`
    /// lookback distance. `entrybound::planner::CompressionProfile`'s own
    /// `dictionary_levels`/`lookback_levels`/`lookback_candidates` are always
    /// subsets of these two canonical lists, so this is a strict superset of
    /// what any one profile's `select_cohort_plan` tries.
    #[must_use]
    pub fn full() -> Self {
        Self {
            levels: SUPPORTED_LEVELS.to_vec(),
            lookbacks: SUPPORTED_LOOKBACKS.to_vec(),
        }
    }
}

/// One priced dictionary or lookback candidate, costed exactly as
/// `entrybound::research::planner::trace_cohort` costs it: `payload_bytes`
/// summed over cohort members, `plan_record_bytes` as the v1 TransformPlan
/// record (cohort candidates always use the v1 record regardless of planner
/// generation -- see `research/harness/research-internals.md`'s
/// "Candidate enumeration" table), and `side_data_bytes` as the Dictionary or
/// ChunkGroup record plus its section overhead.
#[derive(Clone, Debug, Serialize)]
pub struct CohortExhaustiveCandidate {
    pub strategy: CohortStrategyKind,
    pub payload_bytes: u64,
    pub plan_record_bytes: u64,
    pub side_data_bytes: u64,
    pub complete_cost: u64,
}

/// The complete cohort-level search: the independent baseline plus every
/// dictionary and lookback candidate `caps` describes, and the cheapest
/// strategy found.
#[derive(Clone, Debug, Serialize)]
pub struct CohortExhaustiveSearch {
    pub caps: CohortSearchCaps,
    pub independent_cost: u64,
    pub candidates: Vec<CohortExhaustiveCandidate>,
    pub optimal_strategy: CohortStrategyKind,
    pub optimal_cost: u64,
}

fn checked_len(bytes: &[u8]) -> u64 {
    u64::try_from(bytes.len()).expect("tractable-sample payload length fits in u64")
}

/// The v5-style independent-cohort baseline
/// (`entrybound::research::planner`'s private `independent_cohort_cost`'s
/// `-v5` branch, reimplemented over only the publicly exposed research
/// primitives): the sum of each member's own already-assigned-plan payload,
/// plus the v2 TransformPlan record for each *distinct* plan referenced
/// (production charges a shared plan's record once, not once per chunk).
/// Reconstruction side data is out of scope (see the module docs).
fn independent_cohort_cost(
    archive: &Archive,
    cohort: &SimilarityCohort,
    plans: &BTreeMap<u64, TransformPlan>,
) -> Result<u64, Diagnostic> {
    let mut payload_bytes = 0_u64;
    let mut plan_ids = BTreeSet::new();
    for chunk_id in cohort.chunks.iter() {
        let chunk = archive
            .content_store
            .chunks
            .get(chunk_id)
            .expect("cohort chunk id exists in the archive it was built from");
        let plan = plans
            .get(&chunk.plan_ref)
            .expect("chunk plan_ref exists in the plan table it was built from");
        let encoded = encode_payload(plan, &chunk.plaintext)?;
        payload_bytes = payload_bytes
            .checked_add(checked_len(&encoded))
            .expect("tractable-sample independent cohort payload cost fits in u64");
        plan_ids.insert(chunk.plan_ref);
    }
    let mut plan_record_bytes = 0_u64;
    for plan_id in plan_ids {
        let plan = plans
            .get(&plan_id)
            .expect("plan id collected from a chunk exists in the same plan table");
        plan_record_bytes = plan_record_bytes
            .checked_add(encoded_transform_plan_v2_len(plan)?)
            .expect("tractable-sample independent cohort record cost fits in u64");
    }
    Ok(payload_bytes
        .checked_add(plan_record_bytes)
        .expect("tractable-sample independent cohort complete cost fits in u64"))
}

/// Runs the full cohort-level exhaustive search described by `caps` over one
/// `SimilarityCohort`: the independent baseline, every dictionary candidate
/// at every `caps.levels` level (if `train_cohort_dictionary` succeeds), and
/// every lookback candidate at every `caps.lookbacks` x `caps.levels` pair.
/// `archive`, `profile`, `planner_id`, and `plans` are exactly what
/// `entrybound::research::planner::trace_cohort` expects (for example
/// `ChunkStageTrace::{archive, plans}`).
pub fn search_cohort(
    archive: &Archive,
    cohort: &SimilarityCohort,
    profile: CompressionProfile,
    planner_id: &str,
    plans: &BTreeMap<u64, TransformPlan>,
    caps: &CohortSearchCaps,
) -> Result<CohortExhaustiveSearch, Diagnostic> {
    let independent_cost = independent_cohort_cost(archive, cohort, plans)?;
    let mut candidates = Vec::new();

    if let Some(dictionary) = train_cohort_dictionary(archive, cohort, profile, planner_id)? {
        let dictionary_cost = encoded_dictionary_len(&dictionary)?
            .checked_add(DICTIONARY_SECTION_OVERHEAD_BYTES)
            .expect("dictionary section cost fits in u64");
        for level in caps.levels.iter().copied() {
            let plan = zstd_dictionary_plan(level, dictionary.dictionary_id)?;
            let mut payload_bytes = 0_u64;
            for chunk_id in cohort.chunks.iter() {
                let chunk = archive
                    .content_store
                    .chunks
                    .get(chunk_id)
                    .expect("cohort chunk id exists in the archive it was built from");
                let encoded = encode_payload_with_dictionary(&plan, &chunk.plaintext, &dictionary)?;
                payload_bytes = payload_bytes
                    .checked_add(checked_len(&encoded))
                    .expect("tractable-sample dictionary payload cost fits in u64");
            }
            let plan_record_bytes = encoded_transform_plan_len(&plan)?;
            let complete_cost = payload_bytes
                .checked_add(dictionary_cost)
                .and_then(|value| value.checked_add(plan_record_bytes))
                .expect("tractable-sample dictionary candidate cost fits in u64");
            candidates.push(CohortExhaustiveCandidate {
                strategy: CohortStrategyKind::Dictionary { level },
                payload_bytes,
                plan_record_bytes,
                side_data_bytes: dictionary_cost,
                complete_cost,
            });
        }
    }

    for lookback in caps.lookbacks.iter().copied() {
        for level in caps.levels.iter().copied() {
            let plan = zstd_prefix_plan(level, lookback)?;
            let mut payload_bytes = 0_u64;
            for (position, chunk_id) in cohort.chunks.iter().enumerate() {
                let prefix = cohort_prefix(archive, &cohort.chunks, position, lookback)?;
                let chunk = archive
                    .content_store
                    .chunks
                    .get(chunk_id)
                    .expect("cohort chunk id exists in the archive it was built from");
                let encoded = encode_payload_with_prefix(&plan, &chunk.plaintext, &prefix)?;
                payload_bytes = payload_bytes
                    .checked_add(checked_len(&encoded))
                    .expect("tractable-sample lookback payload cost fits in u64");
            }
            let group = ChunkGroup {
                group_id: chunk_group_id(cohort.cohort_id, level, lookback),
                max_lookback: lookback,
                max_preceding_bytes: maximum_preceding_bytes(archive, &cohort.chunks, lookback)?,
            };
            let side_data_bytes = encoded_chunk_group_len(&group)?
                .checked_add(CHUNK_GROUP_SECTION_OVERHEAD_BYTES)
                .expect("chunk group section cost fits in u64");
            let plan_record_bytes = encoded_transform_plan_len(&plan)?;
            let complete_cost = payload_bytes
                .checked_add(plan_record_bytes)
                .and_then(|value| value.checked_add(side_data_bytes))
                .expect("tractable-sample lookback candidate cost fits in u64");
            candidates.push(CohortExhaustiveCandidate {
                strategy: CohortStrategyKind::Lookback { level, lookback },
                payload_bytes,
                plan_record_bytes,
                side_data_bytes,
                complete_cost,
            });
        }
    }

    let (optimal_strategy, optimal_cost) = candidates
        .iter()
        .map(|candidate| (candidate.strategy, candidate.complete_cost))
        .fold(
            (CohortStrategyKind::Independent, independent_cost),
            |best, candidate| {
                if candidate.1 < best.1 {
                    candidate
                } else {
                    best
                }
            },
        );

    Ok(CohortExhaustiveSearch {
        caps: caps.clone(),
        independent_cost,
        candidates,
        optimal_strategy,
        optimal_cost,
    })
}

/// The complete cost of a real `entrybound::research::planner::CohortTrace`'s
/// own selection: the selected candidate's cost, or the independent
/// baseline's cost when no candidate won.
#[must_use]
pub fn cohort_selected_cost(trace: &entrybound::research::planner::CohortTrace) -> u64 {
    match trace.selected {
        Some(index) => trace.candidates[index].complete_cost,
        None => trace.independent.complete_cost,
    }
}

/// `cohort_selected_cost(trace) - search.optimal_cost`. `>= 0` whenever
/// `search`'s caps were built from [`CohortSearchCaps::full`] (a superset of
/// whatever the profile that produced `trace` actually tried).
#[must_use]
pub fn cohort_regret(
    trace: &entrybound::research::planner::CohortTrace,
    search: &CohortExhaustiveSearch,
) -> i128 {
    i128::from(cohort_selected_cost(trace)) - i128::from(search.optimal_cost)
}

// ---------------------------------------------------------------------
// Archive-level (chunk-stage) regret with plan records charged once.
// ---------------------------------------------------------------------

/// One chunk's production selection, costed for archive-level aggregation.
#[derive(Clone, Debug)]
pub struct ChunkSelection<'a> {
    pub plan: &'a TransformPlan,
    /// The selected candidate's codec output length.
    pub payload_bytes: u64,
    /// Reconstruction side data the selected candidate carries (v5 DEFLATE).
    pub side_data_bytes: u64,
}

/// Archive-level chunk-stage cost bounds for one planner generation.
///
/// Harness review round 1, finding R1-07: per-chunk regret charges every
/// candidate's TransformPlan record once *per chunk*, but an archive stores
/// each distinct plan record once. Summing per-chunk regret over an item's
/// chunks therefore does not measure the complete archive cost. The true
/// archive-level optimum (payloads plus the records of the distinct plans
/// used) is a set-cover-like problem, so this reports exact bounds instead:
///
/// - `selected_cost`: Σ selected payload + Σ side data + Σ record of each
///   distinct selected plan.
/// - `optimum_upper_bound`: the cheaper of two feasible assignments (per-chunk
///   argmin of `payload + record`, and per-chunk argmin of `payload`), each
///   charged its distinct records once. The true optimum is `<=` this.
/// - `optimum_lower_bound`: Σ per-chunk minimum payload + the smallest record
///   of any candidate (at least one record must be stored). The true
///   optimum is `>=` this.
/// - `regret_lower_bound = selected - upper_bound` and
///   `regret_upper_bound = selected - lower_bound` bracket the true regret.
///
/// This covers the Chunk stage only: cohort (dictionary/lookback) and JPEG
/// region stages can later override a chunk's plan, and are measured by
/// [`search_cohort`] and the drift reports instead.
#[derive(Clone, Debug, Serialize)]
pub struct ArchiveRegretBounds {
    pub chunk_count: usize,
    pub selected_cost: u64,
    pub selected_distinct_plans: usize,
    pub optimum_upper_bound: u64,
    pub optimum_lower_bound: u64,
    pub regret_lower_bound: i128,
    pub regret_upper_bound: i128,
}

fn plan_key(codec: CodecChoice, transform: TransformKind) -> String {
    format!("{codec:?}/{transform:?}")
}

/// Computes [`ArchiveRegretBounds`] from each chunk's exhaustive search and
/// production selection (same order, same length).
pub fn archive_regret_bounds(
    searches: &[&ChunkExhaustiveSearch],
    selections: &[ChunkSelection<'_>],
) -> Result<ArchiveRegretBounds, Diagnostic> {
    assert_eq!(
        searches.len(),
        selections.len(),
        "one selection per searched chunk"
    );
    let mut selected_cost = 0u64;
    let mut selected_records: BTreeMap<u64, u64> = BTreeMap::new();
    for selection in selections {
        selected_cost = selected_cost
            .saturating_add(selection.payload_bytes)
            .saturating_add(selection.side_data_bytes);
        selected_records
            .entry(selection.plan.plan_id)
            .or_insert(encoded_transform_plan_v2_len(selection.plan)?);
    }
    let selected_distinct_plans = selected_records.len();
    selected_cost = selected_records
        .values()
        .fold(selected_cost, |sum, record| sum.saturating_add(*record));

    let assignment_cost = |pick: &dyn Fn(&ExhaustiveCandidate) -> u64| -> u64 {
        let mut payload = 0u64;
        let mut records: BTreeMap<String, u64> = BTreeMap::new();
        for search in searches {
            if let Some(best) = search.candidates.iter().min_by_key(|c| pick(c)) {
                payload = payload.saturating_add(best.payload_bytes);
                records
                    .entry(plan_key(best.codec, best.transform))
                    .or_insert(best.plan_record_bytes);
            }
        }
        records
            .values()
            .fold(payload, |sum, r| sum.saturating_add(*r))
    };
    let optimum_upper_bound =
        assignment_cost(&|c| c.complete_cost).min(assignment_cost(&|c| c.payload_bytes));

    let mut optimum_lower_bound = 0u64;
    let mut min_record = u64::MAX;
    for search in searches {
        if let Some(min_payload) = search.candidates.iter().map(|c| c.payload_bytes).min() {
            optimum_lower_bound = optimum_lower_bound.saturating_add(min_payload);
        }
        if let Some(record) = search.candidates.iter().map(|c| c.plan_record_bytes).min() {
            min_record = min_record.min(record);
        }
    }
    if !searches.is_empty() && min_record != u64::MAX {
        optimum_lower_bound = optimum_lower_bound.saturating_add(min_record);
    }

    Ok(ArchiveRegretBounds {
        chunk_count: searches.len(),
        selected_cost,
        selected_distinct_plans,
        optimum_upper_bound,
        optimum_lower_bound,
        regret_lower_bound: i128::from(selected_cost) - i128::from(optimum_upper_bound),
        regret_upper_bound: i128::from(selected_cost) - i128::from(optimum_lower_bound),
    })
}

/// Whether a selected plan is inside the chunk-level search's scope (no
/// reconstructive transform step; see the module docs' "Scope").
pub fn plan_in_chunk_search_scope(plan: &TransformPlan) -> Result<bool, Diagnostic> {
    for step in plan.transforms.iter() {
        if entrybound::research::transform::is_reconstructive(step)? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Builds a `Digest` label for research constructs (DP segments, ad hoc
/// cohorts) that need *a* stable, deterministic identity but are not
/// production cohorts with their own similarity-derived `cohort_id`.
#[must_use]
pub fn label_digest(label: &str) -> Digest {
    sha256_exact(label.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{NamedInput, archive_for};
    use entrybound::research::planner::{PlannerVersion, trace_chunk_stage, trace_cohort};

    fn shared_filler_member(index: usize) -> Vec<u8> {
        let mut bytes = format!("cohort-member-{index:02}\n").into_bytes();
        bytes.extend(
            b"shared filler content repeated across every cohort member, so a trained \
              dictionary and a lookback prefix both have real savings to find here; "
                .iter()
                .cycle()
                .take(4096),
        );
        bytes
    }

    #[test]
    fn cohort_exhaustive_search_never_exceeds_the_real_cohort_selection() {
        let profile = CompressionProfile::Extreme;
        let names = ["m0", "m1", "m2", "m3", "m4", "m5"];
        let inputs = names
            .iter()
            .enumerate()
            .map(|(index, name)| {
                let bytes = shared_filler_member(index);
                NamedInput {
                    name,
                    chunk_size: bytes.len(),
                    bytes,
                }
            })
            .collect::<Vec<_>>();
        let unplanned = archive_for(&inputs).unwrap();

        let stage = trace_chunk_stage(&unplanned, profile, PlannerVersion::V6).unwrap();
        let stage_planner_id = PlannerVersion::V6.stage_planner_id(profile);

        let chunk_ids = stage
            .archive
            .content_store
            .chunks
            .keys()
            .copied()
            .collect::<Vec<_>>();
        let logical_bytes = stage
            .archive
            .content_store
            .chunks
            .values()
            .map(|chunk| chunk.logical_len)
            .sum();
        let cohort = SimilarityCohort {
            cohort_id: label_digest("ebr-planner/test-cohort"),
            chunks: chunk_ids.into_boxed_slice(),
            logical_bytes,
        };

        let real_trace = trace_cohort(
            &stage.archive,
            &cohort,
            profile,
            stage_planner_id,
            &stage.plans,
        )
        .unwrap();
        let search = search_cohort(
            &stage.archive,
            &cohort,
            profile,
            stage_planner_id,
            &stage.plans,
            &CohortSearchCaps::full(),
        )
        .unwrap();

        assert_eq!(
            search.independent_cost, real_trace.independent.complete_cost,
            "this crate's independent-cohort cost formula must match production's exactly, \
             not just bound it"
        );

        let regret = cohort_regret(&real_trace, &search);
        assert!(
            regret >= 0,
            "regret={regret}, optimal={} ({:?}), production selected cost={}",
            search.optimal_cost,
            search.optimal_strategy,
            cohort_selected_cost(&real_trace)
        );
    }

    #[test]
    fn chunk_search_finds_store_as_optimal_for_empty_input() {
        let search = search_chunk(&[], &SearchCaps::full()).unwrap();
        assert!(matches!(
            search.candidates[search.optimal].codec,
            CodecChoice::Store
        ));
    }

    /// Harness review round 1, finding R1-07: the "strict superset" claim is
    /// checked mechanically, not asserted in prose. Every non-reconstructive
    /// plan any profile/generation's Chunk stage evaluates must be a member of
    /// the exhaustive universe.
    #[test]
    fn every_production_chunk_candidate_is_in_the_exhaustive_universe() {
        let caps = SearchCaps::full();
        let universe: Vec<TransformPlan> = candidate_shapes(&caps)
            .into_iter()
            .map(|(codec, transform)| codec.plan(transform.step().unwrap()).unwrap())
            .collect();
        let numeric: Vec<u8> = (0_u32..4_096).flat_map(u32::to_le_bytes).collect();
        let text: Vec<u8> = b"superset check; entrybound planner candidates; "
            .iter()
            .cycle()
            .take(16 * 1024)
            .copied()
            .collect();
        for bytes in [numeric, text] {
            for profile in [
                CompressionProfile::Fast,
                CompressionProfile::Balanced,
                CompressionProfile::Dense,
                CompressionProfile::Extreme,
            ] {
                for version in entrybound::research::planner::PlannerVersion::ALL {
                    let trace =
                        entrybound::research::planner::trace_chunk(profile, version, &bytes)
                            .unwrap();
                    for candidate in &trace.candidates {
                        if !plan_in_chunk_search_scope(&candidate.plan).unwrap() {
                            continue;
                        }
                        assert!(
                            universe.contains(&candidate.plan),
                            "{profile:?}/{version:?}: production candidate {} is outside the \
                             exhaustive universe",
                            candidate.plan.identifier
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn archive_regret_bounds_charge_shared_plan_records_once() {
        let a: Vec<u8> = b"alpha alpha alpha; "
            .iter()
            .cycle()
            .take(8192)
            .copied()
            .collect();
        let b: Vec<u8> = b"beta beta beta beta; "
            .iter()
            .cycle()
            .take(8192)
            .copied()
            .collect();
        let caps = SearchCaps::full();
        let search_a = search_chunk(&a, &caps).unwrap();
        let search_b = search_chunk(&b, &caps).unwrap();
        let plan = zstd_plan(3).unwrap();
        let selections = [
            ChunkSelection {
                plan: &plan,
                payload_bytes: encode_payload(&plan, &a).unwrap().len() as u64,
                side_data_bytes: 0,
            },
            ChunkSelection {
                plan: &plan,
                payload_bytes: encode_payload(&plan, &b).unwrap().len() as u64,
                side_data_bytes: 0,
            },
        ];
        let bounds = archive_regret_bounds(&[&search_a, &search_b], &selections).unwrap();
        let record = encoded_transform_plan_v2_len(&plan).unwrap();
        assert_eq!(bounds.selected_distinct_plans, 1);
        assert_eq!(
            bounds.selected_cost,
            selections[0].payload_bytes + selections[1].payload_bytes + record
        );
        assert!(bounds.optimum_lower_bound <= bounds.optimum_upper_bound);
        assert!(bounds.regret_lower_bound <= bounds.regret_upper_bound);
        assert!(bounds.regret_upper_bound >= 0);
    }
}
