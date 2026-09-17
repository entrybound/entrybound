//! Rabin fingerprint CDC: a sliding-window polynomial rolling hash over
//! GF(2), cut when the low bits of the windowed fingerprint are zero.
//!
//! Rabin, M. O. (1981). *Fingerprinting by Random Polynomials.* Harvard
//! Aiken Computation Laboratory TR-15-81. Broder, A. Z. (1993). *Some
//! Applications of Rabin's Fingerprinting Method.* Content-defined chunking
//! built on a Rabin-style rolling fingerprint (the specific construction CDC
//! popularized): Muthitacharoen, A., Chen, B., & Mazieres, D. (2001). *A
//! Low-Bandwidth Network File System* (LBFS). SOSP 2001.
//!
//! **Construction.** The fingerprint is an element of the polynomial ring
//! GF(2)\[x\] reduced modulo a fixed degree-64 polynomial [`POLY`]
//! (represented, as is conventional for a degree-*n* modulus, by its low 64
//! bits -- the implicit leading coefficient at `x^64` is 1). Sliding a
//! `window`-byte window over the input maintains a 64-bit state via two
//! precomputed 256-entry tables built the same way CRC table generators are
//! (see [`MOD_TABLE`]/[`step`]): entering byte `b` computes `state =
//! reduce(state * x^8 + b) mod POLY`, and once the window is full, the byte
//! leaving it is un-added via [`build_remove_table`]'s
//! `reduce(b * x^(8*window)) mod POLY` (linearity over GF(2) makes each
//! byte's contribution independently trackable and cancellable this way,
//! exactly parallel to how [`super::buzhash`] undoes a byte's rotation).
//!
//! **Honesty about `POLY`.** [`POLY`] is an arbitrary fixed odd-valued
//! 64-bit constant -- the golden-ratio-derived constant popularized by
//! Knuth's multiplicative hashing and reused elsewhere in this codebase for
//! unrelated purposes (e.g. `entrybound::chunker::GEAR_SEED`'s splitmix64
//! seed, and `alt_cdc`-style buzhash table seeds). It has **not** been
//! verified irreducible over GF(2). This module therefore does not claim
//! the formal collision-probability bound a true Rabin fingerprint gets from
//! an irreducible modulus (Broder 1993 section 3); it only reuses the same
//! rolling-construction mechanics LBFS-style CDC needs, which depends on the
//! modulus being fixed and well-mixing, not on irreducibility. If this
//! caveat turns out to matter for a downstream decision, swap in a modulus
//! verified irreducible via polynomial factorization first.
//!
//! Not wired into any production code path.

use super::{ChunkRange, pow2_mask_for_target};
use entrybound::chunker::ChunkingParameters;

/// Default window width in bytes (48, per this crate's brief).
pub const DEFAULT_WINDOW: usize = 48;

/// Fixed GF(2) reduction modulus (degree 64, leading term implicit). See the
/// module docs' "Honesty about POLY" section.
const POLY: u64 = 0x9E37_79B9_7F4A_7C15;

/// `MOD_TABLE[t] == reduce((t as u64) << 64) mod POLY`: the standard
/// MSB-first CRC table-generation construction, applied here to a Rabin
/// polynomial modulus instead of a CRC's. See module docs for what `step`
/// uses it for. `pub(crate)` so [`super::tttd`] can reuse this exact rolling
/// hash as its underlying fingerprint source (the original TTTD paper is
/// itself built on a Rabin fingerprint).
pub(crate) const MOD_TABLE: [u64; 256] = build_mod_table();

const fn reduce_one_bit(mut r: u64) -> u64 {
    let carry = (r & 0x8000_0000_0000_0000) != 0;
    r <<= 1;
    if carry {
        r ^= POLY;
    }
    r
}

const fn build_mod_table() -> [u64; 256] {
    let mut table = [0u64; 256];
    let mut b = 0usize;
    while b < 256 {
        let mut r = (b as u64) << 56;
        let mut i = 0;
        while i < 8 {
            r = reduce_one_bit(r);
            i += 1;
        }
        table[b] = r;
        b += 1;
    }
    table
}

/// One step of the rolling fingerprint: `reduce(state * x^8 + incoming) mod
/// POLY`, given `state` already reduced (`< 2^64`). `pub(crate)` so
/// [`super::tttd`] can drive the same rolling hash with its own two masks.
pub(crate) fn step(state: u64, incoming: u8) -> u64 {
    let top = (state >> 56) as usize;
    ((state << 8) | u64::from(incoming)) ^ MOD_TABLE[top]
}

/// `table[b] == reduce((b as u64) * x^(8*window)) mod POLY`: what byte `b`'s
/// contribution decays into after `window` more bytes have entered the
/// rolling state, i.e. exactly what must be XORed out of the state to
/// "forget" it once the sliding window has moved past it. `pub(crate)` so
/// [`super::tttd`] can drive the same windowed rolling hash.
pub(crate) fn build_remove_table(window: usize) -> [u64; 256] {
    let mut table = [0u64; 256];
    for (b, slot) in table.iter_mut().enumerate() {
        let mut v = b as u64;
        for _ in 0..window {
            v = step(v, 0);
        }
        *slot = v;
    }
    table
}

#[derive(Debug, Clone, Copy)]
pub struct Params {
    pub min_size: usize,
    pub target_size: usize,
    pub max_size: usize,
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
            "rabin-cdc-v1/min-{}/target-{}/max-{}/window-{}",
            self.min_size, self.target_size, self.max_size, self.window
        )
    }
}

/// Finds Rabin-fingerprint CDC ranges. Empty input has no ranges. Panics if
/// `params.window == 0` or `params.min_size == 0`.
#[must_use]
pub fn chunk_ranges(data: &[u8], params: &Params) -> Vec<ChunkRange> {
    assert!(params.window > 0, "rabin window must be nonzero");
    assert!(params.min_size > 0, "min_size must be nonzero");
    if data.is_empty() {
        return Vec::new();
    }

    let mask = pow2_mask_for_target(params.target_size);
    let remove_table = build_remove_table(params.window);
    let mut ranges = Vec::new();
    let mut start = 0usize;
    let mut state: u64 = 0;

    for i in 0..data.len() {
        let pos_in_chunk = i - start;
        state = step(state, data[i]);
        if pos_in_chunk >= params.window {
            let out_byte = data[i - params.window];
            state ^= remove_table[out_byte as usize];
        }
        let len = pos_in_chunk + 1;
        if len >= params.min_size && (state & mask == 0 || len >= params.max_size) {
            ranges.push(ChunkRange { start, end: i + 1 });
            start = i + 1;
            state = 0;
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

    /// The tricky part of this module: confirms the incremental
    /// add-then-remove rolling state exactly matches a fingerprint of the
    /// last `window` bytes recomputed from scratch, at several positions.
    #[test]
    fn rolling_state_matches_bruteforce_recomputation() {
        let data = pseudo_random_bytes(5_000, 3);
        let window = 48;
        let remove_table = build_remove_table(window);
        let mut state = 0u64;
        for i in 0..data.len() {
            state = step(state, data[i]);
            if i + 1 > window {
                state ^= remove_table[data[i - window] as usize];
            }
            if i + 1 >= window {
                let bruteforce = data[i + 1 - window..=i]
                    .iter()
                    .fold(0u64, |s, &b| step(s, b));
                assert_eq!(
                    state, bruteforce,
                    "rolling state diverged from bruteforce at position {i}"
                );
            }
        }
    }

    #[test]
    fn empty_input_has_no_ranges() {
        assert!(chunk_ranges(&[], &params(4, 16, 64, 8)).is_empty());
    }

    #[test]
    fn ranges_are_valid_and_deterministic() {
        let data = pseudo_random_bytes(60_000, 1);
        let p = params(64, 512, 4096, 48);
        let a = chunk_ranges(&data, &p);
        let b = chunk_ranges(&data, &p);
        assert_eq!(a, b);
        assert_valid_ranges(data.len(), &a, p.min_size, p.max_size);
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
