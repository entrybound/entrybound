//! AE (Asymmetric Extremum) CDC: an extremum rule over byte values, no
//! rolling hash at all.
//!
//! Zhang, Y., Jiang, H., Feng, D., Xia, W., Fu, M., Huang, F., & Zhou, Y.
//! (2015). *AE: An Asymmetric Extremum Content Defined Chunking Algorithm
//! for Fast and Bandwidth-Efficient Data Deduplication.* IEEE INFOCOM 2015.
//!
//! **Construction (the paper's Algorithm 1).** Track the maximum value seen
//! since the current chunk started and that maximum's position. When `window`
//! positions have passed since the running maximum was last (strictly)
//! beaten, the position right after that run is a cut point. "Asymmetric"
//! names the property that the maximum is compared against an unbounded
//! left horizon and a fixed-size right horizon.
//!
//! **Value width (harness review round 1, finding R1-11).** The paper's rule
//! is stated over "values"; two published instantiations differ:
//!
//! - `value_width = 1` (default here): single byte values. This is the
//!   variant the 2024 comparative study of CDC algorithms uses (Gregoriadis
//!   et al., arXiv:2409.06066), with a right horizon `h = target - 256` for
//!   targets of at least 2 KiB. With byte values the running maximum usually
//!   reaches the alphabet's maximum within a few hundred bytes of the chunk
//!   start, so a boundary is effectively "first maximal byte + `h`".
//! - `value_width = 8`: 8-byte big-endian values, as in the reference
//!   implementation maintained by the paper's authors' group (destor
//!   `src/chunking/ae_chunking.c`, which byte-swaps a `uint64_t` and sets
//!   `window_size = avg / (e - 1)`). Multi-byte values rarely tie and rarely
//!   saturate, so the maximum keeps moving and the expected chunk size is
//!   about `(e - 1) * window`.
//!
//! [`Params::from_profile_with_width`] picks the matching default window for
//! each width unless `window` is given explicitly. Both are empirical
//! calibrations for uniformly random input, checked by this module's tests;
//! `min_size`/`max_size` still clamp every chunk.
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
    /// Compared value width in bytes: 1 or 8.
    pub value_width: u8,
}

impl Params {
    /// Single-byte values with the default window (see module docs).
    #[must_use]
    pub fn from_profile(profile: ChunkingParameters, window: Option<usize>) -> Self {
        Self::from_profile_with_width(profile, window, 1)
            .expect("value width 1 is always supported")
    }

    /// `value_width` must be 1 or 8. The default window is `target - 256`
    /// for width 1 and `target / (e - 1)` for width 8 (see module docs).
    pub fn from_profile_with_width(
        profile: ChunkingParameters,
        window: Option<usize>,
        value_width: u8,
    ) -> Result<Self, String> {
        let default_window = match value_width {
            1 => profile.target_size.saturating_sub(256).max(1),
            8 => ((profile.target_size as f64) / (std::f64::consts::E - 1.0)).round() as usize,
            other => return Err(format!("AE value width must be 1 or 8, got {other}")),
        };
        Ok(Params {
            min_size: profile.minimum_size,
            target_size: profile.target_size,
            max_size: profile.maximum_size,
            window: window.unwrap_or(default_window).max(1),
            value_width,
        })
    }

    #[must_use]
    pub fn algorithm_id(&self) -> String {
        format!(
            "ae-v1/min-{}/target-{}/max-{}/window-{}/value-width-{}",
            self.min_size, self.target_size, self.max_size, self.window, self.value_width
        )
    }
}

#[inline]
fn value_at(data: &[u8], index: usize, width: u8) -> u64 {
    if width == 1 {
        return u64::from(data[index]);
    }
    // Big-endian comparison of the 8 bytes starting at `index`, zero-padded
    // past the end of the input (destor stops scanning 8 bytes before the
    // end; padding keeps every position comparable instead).
    let mut buf = [0u8; 8];
    let end = (index + 8).min(data.len());
    buf[..end - index].copy_from_slice(&data[index..end]);
    u64::from_be_bytes(buf)
}

/// Finds AE ranges. Empty input has no ranges. Panics if `params.window ==
/// 0`, `params.min_size == 0`, or `params.value_width` is not 1 or 8.
#[must_use]
pub fn chunk_ranges(data: &[u8], params: &Params) -> Vec<ChunkRange> {
    assert!(params.window > 0, "ae window must be nonzero");
    assert!(params.min_size > 0, "min_size must be nonzero");
    assert!(
        params.value_width == 1 || params.value_width == 8,
        "ae value width must be 1 or 8"
    );
    if data.is_empty() {
        return Vec::new();
    }

    let mut ranges = Vec::new();
    let mut start = 0usize;
    while start < data.len() {
        let limit = start.saturating_add(params.max_size).min(data.len());
        let mut max_val = value_at(data, start, params.value_width);
        let mut max_pos = start;
        let mut end = limit; // forced cut if nothing else fires
        let mut i = start + 1;
        while i < limit {
            let value = value_at(data, i, params.value_width);
            if value > max_val {
                max_val = value;
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
            value_width: 1,
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

    fn mean_len(ranges: &[ChunkRange]) -> f64 {
        ranges.iter().map(|r| r.len() as f64).sum::<f64>() / ranges.len() as f64
    }

    /// Backs the module docs' width-1 calibration: `window = target - 256`
    /// yields a mean chunk size near `target` on uniformly random bytes.
    #[test]
    fn byte_width_default_window_yields_mean_chunk_size_near_target() {
        let data = pseudo_random_bytes(8_000_000, 55);
        let target = 200_000usize;
        let p = params(1, target, target * 8, target - 256);
        let mean = mean_len(&chunk_ranges(&data, &p));
        let relative_error = (mean - target as f64).abs() / target as f64;
        assert!(
            relative_error < 0.05,
            "expected mean chunk size near {target}, got {mean} ({relative_error:.3} relative error)"
        );
    }

    /// Backs the width-8 calibration from the authors' reference code:
    /// `window = target / (e - 1)` yields a mean near `target`.
    #[test]
    fn eight_byte_width_default_window_yields_mean_chunk_size_near_target() {
        let data = pseudo_random_bytes(8_000_000, 56);
        let target = 64_000usize;
        let profile = ChunkingParameters {
            minimum_size: 1,
            target_size: target,
            maximum_size: target * 16,
            ..entrybound::chunker::BALANCED_V2
        };
        let p = Params::from_profile_with_width(profile, None, 8).unwrap();
        let mean = mean_len(&chunk_ranges(&data, &p));
        let relative_error = (mean - target as f64).abs() / target as f64;
        assert!(
            relative_error < 0.12,
            "expected 8-byte-width mean chunk size near {target}, got {mean} ({relative_error:.3})"
        );
        assert!(Params::from_profile_with_width(profile, None, 4).is_err());
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
