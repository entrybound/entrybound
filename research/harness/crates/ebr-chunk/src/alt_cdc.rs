//! Research-only alternative content-defined chunking (CDC).
//!
//! This is a Buzhash-windowed rolling hash: XOR-rotate over a fixed-size
//! sliding window, cut when the low bits of the hash are zero. It is a
//! different construction from production `gear-norm-v1`
//! (`entrybound::chunker`, a *gear* hash: shift-and-add over the whole
//! preceding run, not a fixed window) -- deliberately not a copy of it, and
//! not wired into any production code path. Its only purpose is to give
//! [`crate::compare`] something else to measure against.
//!
//! Buzhash construction (see e.g. Broder 1993; used by borg/attic-style
//! backup tools): maintain `H`, and for each byte entering a full window,
//! `H = rol(H, 1) XOR table[byte_in] XOR rol(table[byte_out], window)`. A cut
//! point is any position where `H & mask == 0`, clamped to `[min_size,
//! max_size]`.

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

/// Parameters for [`chunk_ranges`]. Unlike `entrybound::chunker`'s frozen
/// presets, these are freely chosen so small test inputs can exercise real
/// cut points.
#[derive(Debug, Clone, Copy)]
pub struct BuzhashParameters {
    /// Sliding window width in bytes. Must be `< 64` (see [`chunk_ranges`]).
    pub window: usize,
    pub min_size: usize,
    /// Target mean chunk size; the cut mask is the next power of two below
    /// this (so the true mean is a power of two near, not exactly,
    /// `target_size`).
    pub target_size: usize,
    pub max_size: usize,
}

impl BuzhashParameters {
    /// Mirrors an `entrybound::chunker::ChunkingParameters` preset's
    /// min/target/max at a fixed 48-byte window, so [`crate::compare`] can
    /// run both chunkers with matching size targets.
    #[must_use]
    pub fn matching(production: entrybound::chunker::ChunkingParameters) -> Self {
        BuzhashParameters {
            window: 48,
            min_size: production.minimum_size,
            target_size: production.target_size,
            max_size: production.maximum_size,
        }
    }
}

/// A complete, non-overlapping plaintext range (mirrors
/// `entrybound::chunker::ChunkRange`'s shape without depending on it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChunkRange {
    pub start: usize,
    pub end: usize,
}

impl ChunkRange {
    #[must_use]
    pub const fn len(self) -> usize {
        self.end - self.start
    }

    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.start == self.end
    }
}

fn mask_for_target(target_size: usize) -> u64 {
    // The largest power of two <= target_size, minus one: an average chunk
    // size of that power of two under a uniform-hash assumption.
    let clamped = target_size.max(2);
    let bits = usize::BITS - clamped.leading_zeros() - 1;
    (1u64 << bits) - 1
}

/// Finds Buzhash CDC ranges. Empty input has no ranges, matching
/// `entrybound::chunker::chunk_ranges`'s convention. Panics if
/// `params.window >= 64` (u64 rotation is only meaningful below the type's
/// bit width) or `params.min_size == 0`.
#[must_use]
pub fn chunk_ranges(data: &[u8], params: &BuzhashParameters) -> Vec<ChunkRange> {
    assert!(
        params.window < 64,
        "buzhash window must be < 64, got {}",
        params.window
    );
    assert!(params.min_size > 0, "min_size must be nonzero");
    if data.is_empty() {
        return Vec::new();
    }

    let mask = mask_for_target(params.target_size);
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

    fn params(min: usize, target: usize, max: usize) -> BuzhashParameters {
        BuzhashParameters {
            window: 16,
            min_size: min,
            target_size: target,
            max_size: max,
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
        assert!(chunk_ranges(&[], &params(4, 16, 64)).is_empty());
    }

    #[test]
    fn ranges_are_contiguous_nonoverlapping_and_cover_the_input() {
        let data = pseudo_random_bytes(20_000, 1);
        let ranges = chunk_ranges(&data, &params(64, 512, 4096));
        assert!(!ranges.is_empty());
        assert_eq!(ranges[0].start, 0);
        assert_eq!(ranges.last().unwrap().end, data.len());
        for window in ranges.windows(2) {
            assert_eq!(window[0].end, window[1].start);
        }
        for r in &ranges {
            assert!(r.len() >= 64, "chunk shorter than min_size: {r:?}");
            assert!(r.len() <= 4096, "chunk longer than max_size: {r:?}");
        }
    }

    #[test]
    fn chunking_is_deterministic() {
        let data = pseudo_random_bytes(50_000, 7);
        let a = chunk_ranges(&data, &params(64, 1024, 8192));
        let b = chunk_ranges(&data, &params(64, 1024, 8192));
        assert_eq!(a, b);
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

        let original = chunk_ranges(&data, &params(256, 4096, 32_768));
        data.splice(insert_at..insert_at, insertion.iter().copied());
        let modified = chunk_ranges(&data, &params(256, 4096, 32_768));

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
