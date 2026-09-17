//! A tiny deterministic PRNG shared by [`crate::shift_stability`] and
//! [`crate::random_access`] for seeded offsets and filler bytes.
//!
//! This is the same xorshift64* construction used throughout this crate's
//! test modules for pseudo-random test data (not cryptographic, not meant
//! to be -- only reproducible given a seed, which is the whole point of
//! "seeded" edits/samples). It lives here once instead of being copy-pasted
//! per module.

#[derive(Debug, Clone, Copy)]
pub struct SeededRng {
    state: u64,
}

impl SeededRng {
    #[must_use]
    pub fn new(seed: u64) -> Self {
        SeededRng { state: seed | 1 }
    }

    /// Advances and returns the next pseudo-random `u64`.
    pub fn next_u64(&mut self) -> u64 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        self.state
    }

    /// A uniformly-chosen value in `0..bound`. Panics if `bound == 0`.
    pub fn next_below(&mut self, bound: usize) -> usize {
        assert!(bound > 0, "next_below bound must be nonzero");
        (self.next_u64() as usize) % bound
    }

    /// `len` pseudo-random bytes.
    pub fn next_bytes(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| (self.next_u64() & 0xff) as u8).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_reproduces_the_same_sequence() {
        let mut a = SeededRng::new(42);
        let mut b = SeededRng::new(42);
        for _ in 0..10 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn next_below_stays_in_bounds() {
        let mut rng = SeededRng::new(7);
        for _ in 0..1000 {
            assert!(rng.next_below(17) < 17);
        }
    }
}
