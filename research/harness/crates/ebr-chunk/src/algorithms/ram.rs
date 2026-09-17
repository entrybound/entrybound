//! RAM (Rapid Asymmetric Maximum) CDC: a bounded-window backward-looking
//! local-maximum rule, no rolling hash.
//!
//! Zhang, Y., Feng, D., Hua, Y., Xia, W., Zhou, J., Zhou, Y., Li, Z., Wan,
//! Y., Xiao, X., & Zhao, S. (2021). *RAM: A Fast and Space-Efficient
//! Content-Defined Chunking Algorithm via Rapid Asymmetric Maximum
//! Sampling.* Frontiers of Computer Science.
//!
//! **What this module reproduces, and what it does not.** RAM's own
//! headline contribution is a *sampling-and-skip* scheme that avoids
//! examining every byte (jumping ahead once a maximum is confirmed) for
//! throughput, while keeping [`super::ae`]'s core idea: judge a boundary by
//! comparing a candidate against a bounded backward window rather than
//! [`super::ae`]'s unbounded look-back-since-the-last-maximum. This module
//! implements exactly that boundary *rule* -- position `i` is a cut point
//! when `data[i]` is the maximum of the trailing `window` bytes ending at
//! `i` -- using a monotonic-deque sliding-window maximum (the standard
//! O(n)-amortized technique, not requiring per-byte comparisons against the
//! whole window) rather than the paper's specific skip-distance
//! bookkeeping. Since the *rule*, not the skip optimization, is what
//! determines chunk boundaries (and therefore dedup ratio), this should
//! match the paper's boundary decisions; it has not been verified against
//! the paper's own reference implementation or test vectors the way
//! [`super::fastcdc2016`]/[`super::fastcdc2020`] have been against the
//! `fastcdc` crate, and should be read as this crate's best-effort
//! reproduction of the published rule rather than a byte-for-byte port.
//!
//! Not wired into any production code path.

use super::ChunkRange;
use entrybound::chunker::ChunkingParameters;
use std::collections::VecDeque;

#[derive(Debug, Clone, Copy)]
pub struct Params {
    pub min_size: usize,
    pub target_size: usize,
    pub max_size: usize,
    pub window: usize,
}

impl Params {
    /// `window` defaults to `target_size`, the same empirical calibration
    /// [`super::ae::Params::from_profile`] uses (see that module's docs for
    /// how it was measured; RAM's bounded-window rule behaves the same way
    /// here), unless `window` overrides it.
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
            "ram-v1/min-{}/target-{}/max-{}/window-{}",
            self.min_size, self.target_size, self.max_size, self.window
        )
    }
}

/// Finds RAM ranges. Empty input has no ranges. Panics if `params.window ==
/// 0` or `params.min_size == 0`.
#[must_use]
pub fn chunk_ranges(data: &[u8], params: &Params) -> Vec<ChunkRange> {
    assert!(params.window > 0, "ram window must be nonzero");
    assert!(params.min_size > 0, "min_size must be nonzero");
    if data.is_empty() {
        return Vec::new();
    }

    let mut ranges = Vec::new();
    let mut start = 0usize;
    while start < data.len() {
        let limit = start.saturating_add(params.max_size).min(data.len());
        // Indices into `data`, values strictly decreasing: the front is
        // always the index of the maximum over the current trailing window.
        let mut deque: VecDeque<usize> = VecDeque::new();
        let mut end = limit; // forced cut if nothing else fires
        for i in start..limit {
            while let Some(&back) = deque.back() {
                if data[back] <= data[i] {
                    deque.pop_back();
                } else {
                    break;
                }
            }
            deque.push_back(i);
            while let Some(&front) = deque.front() {
                if front + params.window <= i {
                    deque.pop_front();
                } else {
                    break;
                }
            }
            let len = i - start + 1;
            if len >= params.min_size && len >= params.window && deque.front() == Some(&i) {
                end = i + 1;
                break;
            }
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
        let data = pseudo_random_bytes(60_000, 4);
        let p = params(64, 512, 4096, 32);
        let a = chunk_ranges(&data, &p);
        let b = chunk_ranges(&data, &p);
        assert_eq!(a, b);
        assert_valid_ranges(data.len(), &a, p.min_size, p.max_size);
    }

    /// Mirrors `ae`'s calibration check: with `window == target`, mean
    /// chunk size over enough uniformly random input lands close to
    /// `target`.
    #[test]
    fn window_equal_to_target_yields_mean_chunk_size_near_target() {
        let data = pseudo_random_bytes(8_000_000, 56);
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
        // Every byte ties the running window's maximum, so the strictly
        // less-or-equal tie-break in the monotonic deque keeps only the
        // current index -- position `i` is always its own window's max,
        // and the cut fires as soon as both `min_size` and `window` are
        // satisfied (here, `min_size` at 64 dominates `window` at 32).
        // Degenerate input does not defeat RAM's cut rule either.
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
