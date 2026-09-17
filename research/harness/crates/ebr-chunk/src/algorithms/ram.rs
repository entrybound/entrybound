//! RAM (Rapid Asymmetric Maximum) CDC: a fixed-size window followed by a
//! first-byte-at-least-the-maximum scan, no rolling hash.
//!
//! Widodo, R. N. S., Lim, H., & Atiquzzaman, M. (2017). *A new
//! content-defined chunking algorithm for data deduplication in cloud
//! storage.* Future Generation Computer Systems, 71, 145-156.
//!
//! **Construction (the paper's rule).** For a chunk starting at `s`, find
//! the maximum byte value `M` in the fixed-size window `[s, s + window)`.
//! Then scan forward from `s + window`; the first byte whose value is
//! `>= M` is the cut point, and it is included in the chunk. RAM needs fewer
//! comparisons than AE because the maximum is fixed once the window has been
//! read.
//!
//! **Corrections in harness review round 1 (finding R1-11).** Earlier
//! revisions attributed RAM to a 2021 paper that does not describe it, and
//! implemented a different rule: "position `i` is a cut when `data[i]` is the
//! maximum of the trailing `window` bytes ending at `i`" (a sliding window
//! that forgets the initial window's maximum as it scrolls away). That rule
//! can cut earlier than RAM and was not the published algorithm. This module
//! now implements the paper's rule directly.
//!
//! **Calibration.** With byte values and a window of more than a few hundred
//! bytes, `M` is almost always the alphabet maximum on random data, so the
//! expected chunk size is about `window + 256`; [`Params::from_profile`]
//! defaults `window` to `target - 256`, matching the right-horizon
//! calibration the 2024 comparative study (Gregoriadis et al.,
//! arXiv:2409.06066) uses for AE. `min_size` and `max_size` still clamp.
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
    /// `window` defaults to `target - 256` (see module docs) unless given.
    #[must_use]
    pub fn from_profile(profile: ChunkingParameters, window: Option<usize>) -> Self {
        let window = window.unwrap_or(profile.target_size.saturating_sub(256).max(1));
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
            "ram-widodo2017-v2/min-{}/target-{}/max-{}/window-{}",
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
        let window_end = start.saturating_add(params.window).min(limit);
        let maximum = data[start..window_end].iter().copied().max().unwrap_or(0);
        // The scan starts after the fixed window, and never cuts a chunk
        // shorter than `min_size` (an addition to the paper, which has no
        // separate minimum; with the default window it never binds).
        let scan_from = window_end.max(start.saturating_add(params.min_size).saturating_sub(1));
        let end = (scan_from..limit)
            .find(|&i| data[i] >= maximum)
            .map_or(limit, |i| i + 1);
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

    /// Backs the module docs' calibration: `window = target - 256` yields a
    /// mean chunk size near `target` on uniformly random bytes.
    #[test]
    fn default_window_yields_mean_chunk_size_near_target() {
        let data = pseudo_random_bytes(8_000_000, 56);
        let target = 200_000usize;
        let p = params(1, target, target * 8, target - 256);
        let ranges = chunk_ranges(&data, &p);
        let mean = ranges.iter().map(|r| r.len() as f64).sum::<f64>() / ranges.len() as f64;
        let relative_error = (mean - target as f64).abs() / target as f64;
        assert!(
            relative_error < 0.05,
            "expected mean chunk size near {target}, got {mean} ({relative_error:.3} relative error)"
        );
    }

    /// The paper's rule, checked by brute force: the cut is the first byte at
    /// or after the fixed window whose value is >= the window's maximum.
    #[test]
    fn cuts_at_the_first_byte_reaching_the_fixed_window_maximum() {
        let data = pseudo_random_bytes(50_000, 11);
        let p = params(1, 512, 50_000, 300);
        let ranges = chunk_ranges(&data, &p);
        let mut start = 0;
        for range in &ranges[..ranges.len() - 1] {
            assert_eq!(range.start, start);
            let maximum = *data[start..start + p.window].iter().max().unwrap();
            let expected = (start + p.window..data.len())
                .find(|&i| data[i] >= maximum)
                .map(|i| i + 1)
                .unwrap();
            assert_eq!(range.end, expected);
            start = range.end;
        }
    }

    /// A text-like alphabet whose maximum is rare: the earlier trailing-window
    /// rule cut as soon as the initial window's maximum scrolled out of view,
    /// while RAM keeps waiting for a byte that reaches it.
    #[test]
    fn keeps_the_initial_window_maximum_after_it_scrolls_away() {
        let mut data = vec![b'a'; 4_000];
        data[10] = b'z';
        data[3_000] = b'z';
        let p = params(1, 512, 4_000, 100);
        let ranges = chunk_ranges(&data, &p);
        assert_eq!(ranges[0].end, 3_001);
    }

    #[test]
    fn constant_byte_input_cuts_right_after_the_window_or_min_size() {
        // Every byte ties the window maximum, so the first scanned byte cuts:
        // at `min_size` when it exceeds the window, else just past the window.
        let data = vec![0u8; 10_000];
        let ranges = chunk_ranges(&data, &params(64, 512, 1024, 32));
        assert!(ranges[..ranges.len() - 1].iter().all(|r| r.len() == 64));
        let ranges = chunk_ranges(&data, &params(16, 512, 1024, 32));
        assert!(ranges[..ranges.len() - 1].iter().all(|r| r.len() == 33));
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
