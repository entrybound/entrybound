//! Research-only alternative selection strategies, compared against
//! [`crate::exhaustive`]'s optimal costs but never fed back into production.
//! All three are deterministic: same input, same output, every time.
//!
//! - [`select_greedy`]: chunk-level, cheap-first with pruning.
//! - [`select_group_lookback`]: cohort-level, a partition dynamic program
//!   over lookback GROUP boundaries.
//! - [`select_by_tree`]: a frozen-threshold decision tree over
//!   `entrybound::planner::analyze`'s integer-only probe -- no search at all.

use entrybound::diagnostics::Diagnostic;
use entrybound::eam::{Archive, ChunkGroup, Digest};
use entrybound::planner::ContentAnalysis;
use entrybound::research::codec::{encode_payload_with_prefix, zstd_prefix_plan};
use entrybound::research::ecf::{encoded_chunk_group_len, encoded_transform_plan_len};
use entrybound::research::planner::{
    CHUNK_GROUP_SECTION_OVERHEAD_BYTES, chunk_group_id, cohort_prefix, maximum_preceding_bytes,
};
use serde::Serialize;

use crate::exhaustive::{
    CodecChoice, CohortSearchCaps, ExhaustiveCandidate, SearchCaps, TransformKind,
    candidate_shapes, cost_candidate, label_digest, search_chunk,
};

// ---------------------------------------------------------------------
// 1. Greedy chunk-level selection with pruning.
// ---------------------------------------------------------------------

/// A greedy, pruned walk over [`crate::exhaustive::candidate_shapes`]'s fixed
/// order: cheap/fast shapes first (STORE, LZ4, ascending Zstandard levels,
/// LZMA2 configurations), then each transform crossed with those same codecs.
/// The walk keeps the best candidate seen and stops as soon as `patience`
/// candidates in a row fail to beat it. Determinism comes from the fixed
/// shape order and the fixed patience rule -- there is no randomness and no
/// dependency on evaluation-time cost estimates beyond the exact costs
/// already computed.
#[derive(Clone, Debug, Serialize)]
pub struct GreedyResult {
    pub chosen: ExhaustiveCandidate,
    /// How many shapes this walk actually costed, out of
    /// `candidate_shapes(caps).len()`. Less than the total when pruning
    /// stopped the walk early.
    pub evaluated: usize,
    pub caps: SearchCaps,
    pub patience: usize,
}

/// Runs the greedy walk. `patience` must be at least 1 (the first candidate
/// -- STORE -- is always kept, so a `patience` of 1 stops as soon as
/// anything at all fails to improve on it).
pub fn select_greedy(
    plaintext: &[u8],
    caps: &SearchCaps,
    patience: usize,
) -> Result<GreedyResult, Diagnostic> {
    assert!(patience >= 1, "patience must be at least 1");
    let shapes = candidate_shapes(caps);
    let mut best: Option<ExhaustiveCandidate> = None;
    let mut stale = 0_usize;
    let mut evaluated = 0_usize;
    for (codec, transform) in shapes {
        evaluated += 1;
        let candidate = cost_candidate(codec, transform, plaintext)?;
        let improved = best
            .as_ref()
            .is_none_or(|current| candidate.complete_cost < current.complete_cost);
        if improved {
            best = Some(candidate);
            stale = 0;
        } else {
            stale += 1;
            if stale >= patience {
                break;
            }
        }
    }
    Ok(GreedyResult {
        chosen: best.expect("STORE is always the first shape tried"),
        evaluated,
        caps: caps.clone(),
        patience,
    })
}

// ---------------------------------------------------------------------
// 2. Dynamic programming over lookback group/lookback decisions.
// ---------------------------------------------------------------------

/// One contiguous segment of an ordered chunk sequence, and the strategy the
/// DP chose for it.
#[derive(Clone, Copy, Debug, Serialize)]
pub enum SegmentStrategy {
    /// A single chunk, compressed on its own (its own chunk-level optimum
    /// from [`crate::exhaustive::search_chunk`]).
    Independent,
    /// `chunks[start..end]` as one lookback group: chunk at position `p`
    /// (within the segment) is prefixed with up to `lookback` preceding
    /// same-segment chunks, exactly as
    /// `entrybound::research::planner::cohort_prefix` builds a production
    /// lookback prefix.
    Lookback { level: i32, lookback: u32 },
}

#[derive(Clone, Debug, Serialize)]
pub struct DpSegment {
    pub start: usize,
    /// Exclusive.
    pub end: usize,
    pub strategy: SegmentStrategy,
    pub cost: u64,
}

/// The DP's chosen partition of the whole sequence and its total cost.
#[derive(Clone, Debug, Serialize)]
pub struct DpPlan {
    pub segments: Vec<DpSegment>,
    pub total_cost: u64,
}

fn lookback_segment_cost(
    archive: &Archive,
    chunk_ids: &[Digest],
    start: usize,
    end: usize,
    level: i32,
    lookback: u32,
) -> Result<u64, Diagnostic> {
    let segment = &chunk_ids[start..end];
    let plan = zstd_prefix_plan(level, lookback)?;
    let mut payload_bytes = 0_u64;
    for (position, chunk_id) in segment.iter().enumerate() {
        let prefix = cohort_prefix(archive, segment, position, lookback)?;
        let chunk = archive
            .content_store
            .chunks
            .get(chunk_id)
            .expect("DP segment chunk id exists in the archive it was built from");
        let encoded = encode_payload_with_prefix(&plan, &chunk.plaintext, &prefix)?;
        payload_bytes = payload_bytes
            .checked_add(u64::try_from(encoded.len()).expect("payload length fits in u64"))
            .expect("tractable-sample DP segment payload cost fits in u64");
    }
    let group_id = chunk_group_id(
        label_digest(&format!("ebr-planner/dp-segment/{start}-{end}")),
        level,
        lookback,
    );
    let group = ChunkGroup {
        group_id,
        max_lookback: lookback,
        max_preceding_bytes: maximum_preceding_bytes(archive, segment, lookback)?,
    };
    let side_data_bytes = encoded_chunk_group_len(&group)?
        .checked_add(CHUNK_GROUP_SECTION_OVERHEAD_BYTES)
        .expect("chunk group section cost fits in u64");
    let plan_record_bytes = encoded_transform_plan_len(&plan)?;
    Ok(payload_bytes
        .checked_add(plan_record_bytes)
        .and_then(|value| value.checked_add(side_data_bytes))
        .expect("tractable-sample DP segment cost fits in u64"))
}

fn independent_chunk_cost(archive: &Archive, chunk_id: &Digest) -> Result<u64, Diagnostic> {
    let chunk = archive
        .content_store
        .chunks
        .get(chunk_id)
        .expect("chunk id exists in the archive it was built from");
    let search = search_chunk(&chunk.plaintext, &SearchCaps::full())?;
    Ok(search.optimal_cost)
}

/// A genuine dynamic program, distinct from production's own cohort
/// mechanism: production forms at most one lookback group per whole
/// similarity cohort. This DP instead chooses, for an ordered sequence of
/// chunks, where to cut it into contiguous segments -- each either
/// independent or one lookback group at its own `(level, lookback)` -- to
/// minimize total cost, by the standard partition recurrence
/// `dp[end] = min over start of dp[start] + segment_cost(start, end)`.
/// Segment length is capped by `max_group_len` (an explicit, recorded bound
/// keeping the search `O(chunks * max_group_len * caps candidates)`, hence
/// tractable). It reuses production's own `cohort_prefix` (via
/// [`lookback_segment_cost`]) so a segment's cost is exactly what production
/// would charge for that same grouping; only the choice of *where to group*
/// is this module's own alternative.
pub fn select_group_lookback(
    archive: &Archive,
    chunk_ids: &[Digest],
    caps: &CohortSearchCaps,
    max_group_len: usize,
) -> Result<DpPlan, Diagnostic> {
    assert!(max_group_len >= 1, "max_group_len must be at least 1");
    let n = chunk_ids.len();
    let mut dp_cost = vec![0_u64; n + 1];
    let mut choice: Vec<Option<DpSegment>> = vec![None; n + 1];

    for end in 1..=n {
        let start_floor = end.saturating_sub(max_group_len);
        let mut best: Option<(u64, DpSegment)> = None;
        for start in start_floor..end {
            let base = dp_cost[start];
            if end - start == 1 {
                let cost = independent_chunk_cost(archive, &chunk_ids[start])?;
                let total = base
                    .checked_add(cost)
                    .expect("tractable-sample DP total cost fits in u64");
                let segment = DpSegment {
                    start,
                    end,
                    strategy: SegmentStrategy::Independent,
                    cost,
                };
                if best.as_ref().is_none_or(|(current, _)| total < *current) {
                    best = Some((total, segment));
                }
            } else {
                for lookback in caps.lookbacks.iter().copied() {
                    for level in caps.levels.iter().copied() {
                        let cost =
                            lookback_segment_cost(archive, chunk_ids, start, end, level, lookback)?;
                        let total = base
                            .checked_add(cost)
                            .expect("tractable-sample DP total cost fits in u64");
                        let segment = DpSegment {
                            start,
                            end,
                            strategy: SegmentStrategy::Lookback { level, lookback },
                            cost,
                        };
                        if best.as_ref().is_none_or(|(current, _)| total < *current) {
                            best = Some((total, segment));
                        }
                    }
                }
            }
        }
        let (total, segment) = best.expect(
            "at least one start position (end - 1, the singleton independent split) is always considered",
        );
        dp_cost[end] = total;
        choice[end] = Some(segment);
    }

    let mut segments = Vec::new();
    let mut cursor = n;
    while cursor > 0 {
        let segment = choice[cursor]
            .clone()
            .expect("a DP choice was recorded for every position from 1 to n");
        cursor = segment.start;
        segments.push(segment);
    }
    segments.reverse();

    Ok(DpPlan {
        segments,
        total_cost: dp_cost[n],
    })
}

/// The trivial "every chunk independent" baseline, for comparing against
/// [`select_group_lookback`]'s result: since that is one of the partitions
/// the DP itself considers (every segment a singleton), the DP's
/// `total_cost` can never exceed this.
pub fn naive_independent_cost(archive: &Archive, chunk_ids: &[Digest]) -> Result<u64, Diagnostic> {
    let mut total = 0_u64;
    for chunk_id in chunk_ids {
        total = total
            .checked_add(independent_chunk_cost(archive, chunk_id)?)
            .expect("tractable-sample naive total cost fits in u64");
    }
    Ok(total)
}

// ---------------------------------------------------------------------
// 3. Deterministic, frozen-threshold decision tree.
// ---------------------------------------------------------------------

/// One leaf of [`select_by_tree`]: which candidate shape it picked, and a
/// short label for which rule fired (useful in a JSONL row; not load-bearing
/// for behavior).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct TreeChoice {
    pub codec: CodecChoice,
    pub transform: TransformKind,
    pub rule: &'static str,
}

/// A deterministic selector over only integer (or integer-derived boolean)
/// features from `entrybound::planner::analyze`'s `ContentAnalysis` --
/// `distinct_symbols`, `maximum_symbol_frequency_ppm`, `adjacent_repeat_ppm`,
/// `likely_compressible`, `empty` -- plus the chunk's own `logical_len`.
/// Every threshold below is frozen: chosen once, by inspecting
/// [`crate::exhaustive::search_chunk`] traces over this crate's own
/// generated tuning fixtures (see this module's tests), not retrained
/// against the real corpus. Promoting these thresholds, or this strategy,
/// beyond research use needs a Decision Ledger entry like everything else
/// this crate builds. There is no search here at all: the same
/// `(analysis, logical_len)` always yields the same choice.
#[must_use]
pub fn select_by_tree(analysis: ContentAnalysis, logical_len: u64) -> TreeChoice {
    if analysis.empty {
        return TreeChoice {
            codec: CodecChoice::Store,
            transform: TransformKind::None,
            rule: "empty",
        };
    }
    if !analysis.likely_compressible {
        return TreeChoice {
            codec: CodecChoice::Store,
            transform: TransformKind::None,
            rule: "incompressible",
        };
    }
    if analysis.distinct_symbols <= 4 || analysis.maximum_symbol_frequency_ppm >= 500_000 {
        return TreeChoice {
            codec: CodecChoice::Zstd { level: 19 },
            transform: TransformKind::None,
            rule: "highly-redundant",
        };
    }
    if analysis.adjacent_repeat_ppm >= 200_000 {
        return TreeChoice {
            codec: CodecChoice::Zstd { level: 9 },
            transform: TransformKind::Delta8,
            rule: "adjacent-repeats",
        };
    }
    if logical_len >= 256 * 1024 {
        return TreeChoice {
            codec: CodecChoice::Zstd { level: 9 },
            transform: TransformKind::None,
            rule: "large-mixed",
        };
    }
    TreeChoice {
        codec: CodecChoice::Zstd { level: 5 },
        transform: TransformKind::None,
        rule: "small-mixed",
    }
}

/// Costs a [`TreeChoice`] the same way [`crate::exhaustive::search_chunk`]
/// costs any other candidate, so its regret against the exhaustive optimum
/// can be computed directly.
pub fn evaluate_tree_choice(
    choice: TreeChoice,
    plaintext: &[u8],
) -> Result<ExhaustiveCandidate, Diagnostic> {
    cost_candidate(choice.codec, choice.transform, plaintext)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exhaustive::search_chunk;
    use crate::{NamedInput, archive_for};

    #[test]
    fn tree_is_a_pure_function_of_its_inputs() {
        let analysis = entrybound::planner::analyze(b"abababababababababababababababab");
        let first = select_by_tree(analysis, 33);
        let second = select_by_tree(analysis, 33);
        assert_eq!(first, second);
    }

    #[test]
    fn tree_stores_empty_input() {
        let analysis = entrybound::planner::analyze(&[]);
        let choice = select_by_tree(analysis, 0);
        assert_eq!(choice.codec, CodecChoice::Store);
        assert_eq!(choice.rule, "empty");
    }

    #[test]
    fn tree_choice_never_beats_the_exhaustive_optimum() {
        let bytes = (0_u32..8_192)
            .flat_map(u32::to_le_bytes)
            .collect::<Vec<_>>();
        let analysis = entrybound::planner::analyze(&bytes);
        let choice = select_by_tree(analysis, bytes.len() as u64);
        let evaluated = evaluate_tree_choice(choice, &bytes).unwrap();
        let search = search_chunk(&bytes, &SearchCaps::full()).unwrap();
        assert!(
            evaluated.complete_cost >= search.optimal_cost,
            "tree chose {:?}/{:?} at {} but the exhaustive optimum is {}",
            choice.codec,
            choice.transform,
            evaluated.complete_cost,
            search.optimal_cost
        );
    }

    fn text_bytes(prefix: &str, len: usize) -> Vec<u8> {
        prefix
            .bytes()
            .chain(
                b"the quick brown fox jumps over the lazy dog; "
                    .iter()
                    .copied(),
            )
            .cycle()
            .take(len)
            .collect()
    }

    #[test]
    fn greedy_never_beats_the_exhaustive_optimum() {
        let bytes = text_bytes("greedy", 20 * 1024);
        let caps = SearchCaps::full();
        let search = search_chunk(&bytes, &caps).unwrap();
        let greedy = select_greedy(&bytes, &caps, 3).unwrap();
        assert!(greedy.chosen.complete_cost >= search.optimal_cost);
        assert!(greedy.evaluated <= candidate_shapes(&caps).len());
    }

    #[test]
    fn greedy_prunes_before_scanning_every_shape() {
        // Highly redundant content: the very first Zstandard level already
        // gets close to the achievable minimum, so a patience of 1 should
        // stop well short of the full 52-shape universe.
        let bytes = vec![b'x'; 32 * 1024];
        let caps = SearchCaps::full();
        let greedy = select_greedy(&bytes, &caps, 1).unwrap();
        assert!(greedy.evaluated < candidate_shapes(&caps).len());
    }

    #[test]
    fn greedy_is_deterministic() {
        let bytes = text_bytes("determinism", 20 * 1024);
        let caps = SearchCaps::full();
        let first = select_greedy(&bytes, &caps, 3).unwrap();
        let second = select_greedy(&bytes, &caps, 3).unwrap();
        assert_eq!(first.chosen.complete_cost, second.chosen.complete_cost);
        assert_eq!(first.evaluated, second.evaluated);
    }

    #[test]
    fn dp_never_exceeds_the_naive_independent_baseline() {
        let names = ["s0", "s1", "s2", "s3"];
        let inputs = names
            .iter()
            .enumerate()
            .map(|(index, name)| {
                let mut bytes = format!("segment-{index}\n").into_bytes();
                bytes.extend(
                    b"shared repeated body content every segment carries, so a lookback \
                      group has real savings available; "
                        .iter()
                        .cycle()
                        .take(2048),
                );
                NamedInput {
                    name,
                    chunk_size: bytes.len(),
                    bytes,
                }
            })
            .collect::<Vec<_>>();
        let unplanned = archive_for(&inputs).unwrap();
        let chunk_ids = unplanned
            .content_store
            .chunks
            .keys()
            .copied()
            .collect::<Vec<_>>();
        let caps = CohortSearchCaps::full();

        let plan = select_group_lookback(&unplanned, &chunk_ids, &caps, 3).unwrap();
        let naive = naive_independent_cost(&unplanned, &chunk_ids).unwrap();
        assert!(
            plan.total_cost <= naive,
            "DP total {} exceeded the naive independent baseline {naive}",
            plan.total_cost
        );

        // Reconstructing the partition from `segments` must exactly cover
        // `0..chunk_ids.len()` with no gap or overlap.
        let mut cursor = 0;
        for segment in &plan.segments {
            assert_eq!(segment.start, cursor);
            assert!(segment.end > segment.start);
            cursor = segment.end;
        }
        assert_eq!(cursor, chunk_ids.len());
    }

    #[test]
    fn dp_is_deterministic() {
        let names = ["d0", "d1", "d2"];
        let inputs = names
            .iter()
            .enumerate()
            .map(|(index, name)| {
                let mut bytes = format!("dp-{index}\n").into_bytes();
                bytes.extend(b"repeated shared body; ".iter().cycle().take(1024));
                NamedInput {
                    name,
                    chunk_size: bytes.len(),
                    bytes,
                }
            })
            .collect::<Vec<_>>();
        let unplanned = archive_for(&inputs).unwrap();
        let chunk_ids = unplanned
            .content_store
            .chunks
            .keys()
            .copied()
            .collect::<Vec<_>>();
        let caps = CohortSearchCaps::full();
        let first = select_group_lookback(&unplanned, &chunk_ids, &caps, 3).unwrap();
        let second = select_group_lookback(&unplanned, &chunk_ids, &caps, 3).unwrap();
        assert_eq!(first.total_cost, second.total_cost);
        assert_eq!(first.segments.len(), second.segments.len());
    }
}
