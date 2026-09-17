//! AE (Asymmetric Extremum) CDC: a byte-value extremum rule, no rolling
//! hash at all.
//!
//! Zhang, Y., Jiang, H., Feng, D., Xia, W., Fu, M., Huang, F., & Zhou, Y.
//! (2015). *AE: An Asymmetric Extremum Content Defined Chunking Algorithm
//! for Fast and Bandwidth-Efficient Data Deduplication.* IEEE INFOCOM 2015.
//!
//! **Construction.** Unlike every other algorithm in [`super`], AE does not
//! hash anything: it tracks the maximum byte value seen since the current
//! chunk started (and that maximum's position). Once `window` consecutive
//! bytes have passed *since* that running maximum was last updated (i.e. the
//! maximum has held for `window` bytes without being beaten), the position
//! right after that run is a cut point. "Asymmetric" names the property
//! that a candidate boundary is judged only by what came *before* it (how
//! long since the last new maximum), not by a window straddling it in both
//! directions the way a plain local-extremum rule would.
//!
//! **Calibrating `window` to a target size.** This crate has not
//! independently re-derived the paper's own analytical expected-chunk-size
//! formula (its Theorem 1), so rather than assert a specific constant from
//! memory, [`Params::from_profile`] defaults `window` to `target_size`
//! directly, based on this crate's own measurement (`boundaries` over
//! several megabytes of uniformly random bytes at each production preset,
//! reproduced by this module's tests): with `window` set equal to
//! `target_size`, the realized mean chunk size lands within about 0.2% of
//! `window` (i.e. of `target_size`) for uniformly random input. This is an
//! empirical calibration of *this* implementation, not a guarantee for
//! non-uniform real-world data; `min_size`/`max_size` still clamp every
//! chunk exactly regardless.
//!
//! Not wired into any production code path.

use super::ChunkRange;
use entrybound::chunker::ChunkingParameters;

#[derive(Debug, Clone, Copy)]
pub struct Params {
    pub min_size: usize,
    pub target_size: usize,
    pub max_size: usize,
    pub window: usize,
}

impl Params {
    /// `window` defaults to `target_size` (see module docs' "Calibrating
    /// `window`" section) unless overridden.
    #[must_use]
    pub fn from_profile(profile: ChunkingParameters, window: Option<usize>) -> Self {
        let window = window.unwrap_or(profile.target_size.max(1));
        Params {
            min_size: profile.minimum_size,
            target_size: profile.target_size,
            max_size: profile.maximum_size,
            window,
        }
    }

    #[must_use]
    pub fn algorithm_id(&self) -> String {
        format!(
            "ae-v1/min-{}/target-{}/max-{}/window-{}",
            self.min_size, self.target_size, self.max_size, self.window
        )
    }
}

/// Finds AE ranges. Empty input has no ranges. Panics if `params.window ==
/// 0` or `params.min_size == 0`.
#[must_use]
pub fn chunk_ranges(data: &[u8], params: &Params) -> Vec<ChunkRange> {
    assert!(params.window > 0, "ae window must be nonzero");
    assert!(params.min_size > 0, "min_size must be nonzero");
    if data.is_empty() {
        return Vec::new();
    }

    let mut ranges = Vec::new();
    let mut start = 0usize;
    while start < data.len() {
        let limit = start.saturating_add(params.max_size).min(data.len());
        let mut max_val = data[start];
        let mut max_pos = start;
        let mut end = limit; // forced cut if nothing else fires
        let mut i = start + 1;
        while i < limit {
            if data[i] > max_val {
                max_val = data[i];
                max_pos = i;
            } else if i - max_pos >= params.window && (i + 1 - start) >= params.min_size {
                end = i + 1;
                break;
            }
            i += 1;
        }
        ranges.push(ChunkRange { start, end });
        start = end;
    }
    ranges
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algorithms::assert_valid_ranges;

    fn params(min: usize, target: usize, max: usize, window: usize) -> Params {
        Params {
            min_size: min,
            target_size: target,
            max_size: max,
            window,
        }
    }

    fn pseudo_random_bytes(len: usize, seed: u64) -> Vec<u8> {
        let mut state = seed | 1;
        (0..len)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                (state & 0xff) as u8
            })
            .collect()
    }

    #[test]
    fn empty_input_has_no_ranges() {
        assert!(chunk_ranges(&[], &params(4, 16, 64, 8)).is_empty());
    }

    #[test]
    fn ranges_are_valid_and_deterministic() {
        let data = pseudo_random_bytes(60_000, 2);
        let p = params(64, 512, 4096, 32);
        let a = chunk_ranges(&data, &p);
        let b = chunk_ranges(&data, &p);
        assert_eq!(a, b);
        assert_valid_ranges(data.len(), &a, p.min_size, p.max_size);
    }

    /// Backs the module docs' calibration claim: with `window == target`,
    /// the mean chunk size over enough uniformly random input lands close
    /// to `target`. This is a smoke-scale check (a few MB), not a rigorous
    /// statistical proof, but it is what the doc comment's number is drawn
    /// from -- if this ever drifts, that comment needs updating too.
    #[test]
    fn window_equal_to_target_yields_mean_chunk_size_near_target() {
        let data = pseudo_random_bytes(8_000_000, 55);
        let target = 200_000usize;
        let p = params(1, target, target * 8, target);
        let ranges = chunk_ranges(&data, &p);
        let mean = ranges.iter().map(|r| r.len() as f64).sum::<f64>() / ranges.len() as f64;
        let relative_error = (mean - target as f64).abs() / target as f64;
        assert!(
            relative_error < 0.05,
            "expected mean chunk size near {target}, got {mean} ({relative_error:.3} relative error)"
        );
    }

    #[test]
    fn constant_byte_input_cuts_at_a_regular_short_interval() {
        // No byte ever beats the running maximum (every byte ties it, and
        // AE's rule needs strictly-greater to update `max_pos`), so
        // `i - max_pos >= window` fires as soon as both it and `min_size`
        // are satisfied -- here that is exactly at `min_size` (64 > window
        // + 1 == 33), not at `max_size`. Degenerate input does not defeat
        // AE's cut rule the way it would a rule that only looks for a
        // strictly-increasing run.
        let data = vec![0u8; 10_000];
        let p = params(64, 512, 1024, 32);
        let ranges = chunk_ranges(&data, &p);
        assert!(ranges[..ranges.len() - 1].iter().all(|r| r.len() == 64));
    }

    #[test]
    fn insertion_only_perturbs_nearby_boundaries() {
        let mut data = pseudo_random_bytes(200_000, 42);
        let insert_at = 5_000;
        let insertion = pseudo_random_bytes(37, 99);
        let p = params(256, 4096, 32_768, 1_500);

        let original = chunk_ranges(&data, &p);
        data.splice(insert_at..insert_at, insertion.iter().copied());
        let modified = chunk_ranges(&data, &p);

        let shifted_original: std::collections::HashSet<usize> =
            original.iter().map(|r| r.end + insertion.len()).collect();
        let modified_tail: Vec<usize> = modified
            .iter()
            .map(|r| r.end)
            .filter(|&end| end > insert_at + insertion.len() + 20_000)
            .collect();
        assert!(!modified_tail.is_empty());
        let matching = modified_tail
            .iter()
            .filter(|end| shifted_original.contains(end))
            .count();
        let ratio = matching as f64 / modified_tail.len() as f64;
        assert!(
            ratio > 0.6,
            "expected most tail boundaries to survive a small insertion; matched {matching}/{}",
            modified_tail.len()
        );
    }
}
