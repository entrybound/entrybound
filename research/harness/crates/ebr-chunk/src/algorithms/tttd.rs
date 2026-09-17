//! TTTD (Two Thresholds, Two Divisors): a byte-content-agnostic rolling-hash
//! CDC rule with a backup checkpoint, so pathological runs land on a
//! content-defined cut rather than always piling up at `max_size`.
//!
//! Eshghi, K., & Tang, H. K. (2005). *A Framework for Analyzing and
//! Improving Content-Based Chunking Algorithms.* HP Labs technical report
//! HPL-2005-30. The original paper builds TTTD on a Rabin fingerprint, so
//! this module reuses [`super::rabin`]'s exact windowed rolling hash
//! (`rabin::step`/`rabin::build_remove_table`) as its underlying
//! fingerprint source rather than re-deriving one -- TTTD's own
//! contribution is the two-divisor decision rule layered on top, not the
//! hash itself. It operates purely on the byte window's rolling
//! fingerprint (no format- or content-type-specific logic of any kind), so
//! it applies unchanged to any byte stream.
//!
//! **Construction.** Scanning forward from `min_size` (`MinThresh`), a
//! position is a boundary as soon as the rolling fingerprint satisfies the
//! main divisor's mask (`mask_d`, calibrated to `target_size`). While
//! scanning, the *first* position that additionally satisfies a looser
//! backup mask (`mask_d_prime`, calibrated to a smaller target so it is hit
//! more often -- the "second divisor," `D' < D`) is remembered. If the scan
//! reaches `max_size` (`MaxThresh`) without ever satisfying `mask_d`, the
//! chunk is cut at the remembered backup position instead of forcing a hard
//! `max_size` cut when one was seen; only if no backup position was seen
//! either does the chunk get forced to exactly `max_size`. This is the
//! paper's headline fix for plain min/max-clamped CDC's tendency to pile up
//! chunks at `max_size` on runs the primary divisor never fires on.
//!
//! Not wired into any production code path.

use super::{ChunkRange, pow2_mask_for_target, rabin};
use entrybound::chunker::ChunkingParameters;

#[derive(Debug, Clone, Copy)]
pub struct Params {
    pub min_size: usize,
    pub target_size: usize,
    pub max_size: usize,
    pub window: usize,
}

impl Params {
    /// `window` defaults to [`rabin::DEFAULT_WINDOW`] (48 bytes, matching
    /// this crate's Rabin-fingerprint module) unless overridden.
    #[must_use]
    pub fn from_profile(profile: ChunkingParameters) -> Self {
        Params {
            min_size: profile.minimum_size,
            target_size: profile.target_size,
            max_size: profile.maximum_size,
            window: rabin::DEFAULT_WINDOW,
        }
    }

    #[must_use]
    pub fn algorithm_id(&self) -> String {
        format!(
            "tttd-v1/min-{}/target-{}/max-{}/window-{}",
            self.min_size, self.target_size, self.max_size, self.window
        )
    }
}

/// Finds TTTD ranges. Empty input has no ranges. Panics if `params.window ==
/// 0`, `params.min_size == 0`, or `min_size > target_size` or `target_size >
/// max_size`.
#[must_use]
pub fn chunk_ranges(data: &[u8], params: &Params) -> Vec<ChunkRange> {
    assert!(params.window > 0, "tttd window must be nonzero");
    assert!(params.min_size > 0, "min_size must be nonzero");
    assert!(
        params.min_size <= params.target_size && params.target_size <= params.max_size,
        "tttd requires min_size <= target_size <= max_size"
    );
    if data.is_empty() {
        return Vec::new();
    }

    let mask_d = pow2_mask_for_target(params.target_size);
    let mask_d_prime = pow2_mask_for_target(params.target_size / 2);
    let remove_table = rabin::build_remove_table(params.window);

    let mut ranges = Vec::new();
    let mut start = 0usize;
    while start < data.len() {
        let limit = start.saturating_add(params.max_size).min(data.len());
        let mut state = 0u64;
        let mut backup: Option<usize> = None;
        let mut cut_at: Option<usize> = None;
        for i in start..limit {
            state = rabin::step(state, data[i]);
            let pos_in_chunk = i - start;
            if pos_in_chunk >= params.window {
                let out_byte = data[i - params.window];
                state ^= remove_table[out_byte as usize];
            }
            let len = pos_in_chunk + 1;
            if len < params.min_size {
                continue;
            }
            if state & mask_d == 0 {
                cut_at = Some(i + 1);
                break;
            }
            if backup.is_none() && state & mask_d_prime == 0 {
                backup = Some(i + 1);
            }
        }
        let end = cut_at.or(backup).unwrap_or(limit);
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
        let data = pseudo_random_bytes(60_000, 9);
        let p = params(64, 512, 4096, 48);
        let a = chunk_ranges(&data, &p);
        let b = chunk_ranges(&data, &p);
        assert_eq!(a, b);
        assert_valid_ranges(data.len(), &a, p.min_size, p.max_size);
    }

    #[test]
    fn repetitive_input_still_produces_valid_ranges() {
        // A degenerate, low-entropy input (constant-byte runs make the
        // rolling fingerprint's low bits pathological) is exactly the case
        // TTTD's backup divisor exists for; whatever it decides, the
        // contract (contiguous, covers the input, size-clamped) must hold.
        let data: Vec<u8> = (0..10_000u32)
            .map(|i| if i % 2 == 0 { 0 } else { 1 })
            .collect();
        let p = params(64, 512, 1024, 48);
        let ranges = chunk_ranges(&data, &p);
        assert_valid_ranges(data.len(), &ranges, p.min_size, p.max_size);
    }

    #[test]
    fn insertion_only_perturbs_nearby_boundaries() {
        let mut data = pseudo_random_bytes(200_000, 42);
        let insert_at = 5_000;
        let insertion = pseudo_random_bytes(37, 99);
        let p = params(256, 4096, 32_768, 48);

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
            ratio > 0.8,
            "expected most tail boundaries to survive a small insertion; matched {matching}/{}",
            modified_tail.len()
        );
    }
}
