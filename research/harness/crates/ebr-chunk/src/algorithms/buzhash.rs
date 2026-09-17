//! Buzhash CDC: a cyclic-polynomial rolling hash over a fixed-size window,
//! cut when the low bits of the hash are zero.
//!
//! Broder, A. Z. (1993). *Some Applications of Rabin's Fingerprinting
//! Method.* Buzhash is the cyclic-polynomial rolling hash construction
//! described there (rotate-and-XOR rather than Rabin's polynomial-ring
//! reduction, see [`super::rabin`] for that one); it is the rolling hash
//! several real backup/dedup tools use for CDC (e.g. borg/attic-style
//! chunkers).
//!
//! **Construction.** For each byte entering a full window: `H = rol(H, 1)
//! XOR TABLE[byte_in] XOR rol(TABLE[byte_out], window)`. A cut point is any
//! position where `H & mask == 0`, clamped to `[min_size, max_size]`. The
//! `rol(TABLE[byte_out], window)` term is exact (not approximate) for
//! `window` up to and including 64: `u64::rotate_left` computes its shift
//! amount modulo the bit width, which is precisely the rotation `byte_out`'s
//! original contribution has accumulated after `window` further rotations.
//!
//! This module's `TABLE` is a fixed pseudo-random table, distinct from
//! `entrybound::chunker`'s `GEAR_TABLE` (different seed, different mixing,
//! and a different kind of table -- Buzhash needs values that mix well
//! *under rotation*, Gear hash needs values that mix well under
//! shift-and-add). Not wired into any production code path.

use super::{ChunkRange, pow2_mask_for_target};
use entrybound::chunker::ChunkingParameters;

/// Default window width in bytes (64, per this crate's brief -- the widest a
/// single rotation position can be while every byte in the window still
/// maps to a distinct rotation amount in a 64-bit word).
pub const DEFAULT_WINDOW: usize = 64;

/// A fixed pseudo-random table, distinct from `entrybound::chunker`'s
/// `GEAR_TABLE` (different seed, different mixing -- see [`build_table`]).
const TABLE: [u64; 256] = build_table();

const fn build_table() -> [u64; 256] {
    // A small, deterministic xorshift64* stream seeded by index. Not
    // cryptographic and not meant to be: it only needs to spread bits well
    // enough across 256 table entries for a rolling hash, computed once at
    // compile time.
    let mut table = [0u64; 256];
    let mut i = 0usize;
    let mut seed: u64 = 0x9E37_79B9_7F4A_7C15; // arbitrary (golden-ratio-derived) constant
    while i < 256 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed = seed
            .wrapping_add(i as u64)
            .wrapping_mul(0x2545_F491_4F6C_DD1D);
        table[i] = seed;
        i += 1;
    }
    table
}

#[derive(Debug, Clone, Copy)]
pub struct Params {
    pub min_size: usize,
    pub target_size: usize,
    pub max_size: usize,
    /// Sliding window width in bytes. Must be in `1..=64` (see
    /// [`chunk_ranges`]).
    pub window: usize,
}

impl Params {
    #[must_use]
    pub fn from_profile(profile: ChunkingParameters, window: usize) -> Self {
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
            "buzhash-cdc-v1/min-{}/target-{}/max-{}/window-{}",
            self.min_size, self.target_size, self.max_size, self.window
        )
    }
}

/// Finds Buzhash CDC ranges. Empty input has no ranges, matching
/// `entrybound::chunker::chunk_ranges`'s convention. Panics if
/// `params.window` is 0 or greater than 64 (beyond 64, distinct bytes in the
/// window would alias to the same rotation amount in a 64-bit word) or
/// `params.min_size == 0`.
#[must_use]
pub fn chunk_ranges(data: &[u8], params: &Params) -> Vec<ChunkRange> {
    assert!(
        params.window >= 1 && params.window <= 64,
        "buzhash window must be in 1..=64, got {}",
        params.window
    );
    assert!(params.min_size > 0, "min_size must be nonzero");
    if data.is_empty() {
        return Vec::new();
    }

    let mask = pow2_mask_for_target(params.target_size);
    let mut ranges = Vec::new();
    let mut start = 0usize;
    let mut hash: u64 = 0;

    for i in 0..data.len() {
        let pos_in_chunk = i - start;
        hash = hash.rotate_left(1) ^ TABLE[data[i] as usize];
        if pos_in_chunk >= params.window {
            let out_byte = data[i - params.window];
            hash ^= TABLE[out_byte as usize].rotate_left(params.window as u32);
        }
        let len = i - start + 1;
        if len >= params.min_size && (hash & mask == 0 || len >= params.max_size) {
            ranges.push(ChunkRange { start, end: i + 1 });
            start = i + 1;
            hash = 0;
        }
    }
    if start < data.len() {
        ranges.push(ChunkRange {
            start,
            end: data.len(),
        });
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
        assert!(chunk_ranges(&[], &params(4, 16, 64, 16)).is_empty());
    }

    #[test]
    fn ranges_are_valid_and_deterministic() {
        let data = pseudo_random_bytes(60_000, 1);
        let p = params(64, 512, 4096, 64);
        let a = chunk_ranges(&data, &p);
        let b = chunk_ranges(&data, &p);
        assert_eq!(a, b);
        assert_valid_ranges(data.len(), &a, p.min_size, p.max_size);
    }

    #[test]
    #[should_panic(expected = "window must be in 1..=64")]
    fn rejects_a_window_wider_than_64() {
        let data = pseudo_random_bytes(1_000, 1);
        let _ = chunk_ranges(&data, &params(4, 16, 64, 65));
    }

    /// The property that actually distinguishes CDC from fixed-size
    /// chunking: inserting a few bytes near the start of the input should
    /// only desynchronize boundaries locally. Most boundaries in the back
    /// two-thirds of the data should reappear, shifted by exactly the
    /// insertion length, in the modified input.
    #[test]
    fn insertion_only_perturbs_nearby_boundaries() {
        let mut data = pseudo_random_bytes(200_000, 42);
        let insert_at = 5_000;
        let insertion = pseudo_random_bytes(37, 99);
        let p = params(256, 4096, 32_768, 64);

        let original = chunk_ranges(&data, &p);
        data.splice(insert_at..insert_at, insertion.iter().copied());
        let modified = chunk_ranges(&data, &p);

        let shifted_original: std::collections::HashSet<usize> =
            original.iter().map(|r| r.end + insertion.len()).collect();
        let modified_ends_in_tail: Vec<usize> = modified
            .iter()
            .map(|r| r.end)
            .filter(|&end| end > insert_at + insertion.len() + 20_000)
            .collect();

        assert!(
            !modified_ends_in_tail.is_empty(),
            "expected boundaries well past the insertion point"
        );
        let matching = modified_ends_in_tail
            .iter()
            .filter(|end| shifted_original.contains(end))
            .count();
        let ratio = matching as f64 / modified_ends_in_tail.len() as f64;
        assert!(
            ratio > 0.8,
            "expected most tail boundaries to survive a small insertion (CDC's whole point); \
             matched {matching}/{} ({ratio:.2})",
            modified_ends_in_tail.len()
        );
    }
}
