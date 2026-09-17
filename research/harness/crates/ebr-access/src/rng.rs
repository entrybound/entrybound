//! Deterministic, seedable pseudo-randomness for reproducible sampling.
//!
//! [`measure_random_access`](crate::measure_random_access) needs to pick a
//! reproducible random sample of entries and a reproducible random byte
//! range within a large entry, and [`probe`](crate::probe) needs
//! reproducible synthetic plaintext. None of that is security-sensitive, so
//! this crate uses a tiny local splitmix64 generator (Sebastiano Vigna's
//! public-domain construction) instead of adding the `rand` crate as a new
//! dependency -- no other crate in this workspace depends on it yet, and
//! `research/harness/README.md`'s "Third-party dependency pins" note keeps
//! each crate's direct dependency list deliberately small.

/// A splitmix64 generator. Not suitable for anything cryptographic; only
/// ever used here to pick reproducible samples and synthetic fill bytes.
#[derive(Debug, Clone, Copy)]
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    #[must_use]
    pub const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in `[0, bound)`. Always `0` when `bound == 0`.
    ///
    /// This uses the plain `% bound` reduction, which has a small modulo
    /// bias for a `bound` that does not divide `u64::MAX + 1` evenly. That
    /// bias is irrelevant here -- every use in this crate picks a sample
    /// index or a byte offset out of at most a few billion, nowhere near
    /// `u64::MAX` -- and staying with the plain reduction keeps this
    /// generator's output reproducible without pulling in rejection-sampling
    /// machinery a research harness does not need.
    pub fn next_below(&mut self, bound: u64) -> u64 {
        if bound == 0 {
            return 0;
        }
        self.next_u64() % bound
    }
}

/// Picks `sample_size` indices from `0..len` without replacement, in a
/// reproducible order determined by `seed` (a partial Fisher-Yates shuffle).
/// Returns every index (in seeded shuffled order) when `sample_size >= len`.
#[must_use]
pub fn sample_indices(len: usize, sample_size: usize, seed: u64) -> Vec<usize> {
    let mut indices: Vec<usize> = (0..len).collect();
    let mut rng = SplitMix64::new(seed);
    let take = sample_size.min(len);
    for i in 0..take {
        let bound = (len - i) as u64;
        let offset = usize::try_from(rng.next_below(bound)).unwrap_or(0);
        let j = i + offset;
        indices.swap(i, j);
    }
    indices.truncate(take);
    indices
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_is_reproducible() {
        let mut a = SplitMix64::new(42);
        let mut b = SplitMix64::new(42);
        for _ in 0..100 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn different_seeds_diverge() {
        let mut a = SplitMix64::new(1);
        let mut b = SplitMix64::new(2);
        let seq_a: Vec<u64> = (0..8).map(|_| a.next_u64()).collect();
        let seq_b: Vec<u64> = (0..8).map(|_| b.next_u64()).collect();
        assert_ne!(seq_a, seq_b);
    }

    #[test]
    fn next_below_is_within_bound() {
        let mut rng = SplitMix64::new(7);
        for _ in 0..1000 {
            assert!(rng.next_below(10) < 10);
        }
        assert_eq!(rng.next_below(0), 0);
    }

    #[test]
    fn sample_indices_is_reproducible_and_bounded() {
        let a = sample_indices(20, 5, 99);
        let b = sample_indices(20, 5, 99);
        assert_eq!(a, b);
        assert_eq!(a.len(), 5);
        assert!(a.iter().all(|&i| i < 20));
        let mut sorted = a.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), a.len(), "sample must not repeat an index");
    }

    #[test]
    fn sample_indices_caps_at_len() {
        let all = sample_indices(4, 100, 1);
        assert_eq!(all.len(), 4);
    }
}
