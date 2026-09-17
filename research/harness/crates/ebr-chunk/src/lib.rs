//! Chunking/dedup research: production `gear-norm-v1`
//! (`entrybound::chunker`) alongside research-only alternative
//! content-defined chunking (CDC) algorithms, all reachable through one
//! [`algorithms::Algorithm`] dispatch so the `boundaries`, `dedup`,
//! `shift-stability`, and `random-access` subcommands ([`boundaries`],
//! [`dedup`], [`shift_stability`], [`random_access`]) work identically
//! regardless of which algorithm produced the chunk ranges. See
//! `README.md` for the algorithm list and citations, and each module's own
//! docs for its specific construction.

pub mod algorithms;
pub mod boundaries;
pub mod dedup;
pub mod random_access;
mod seeded_rng;
pub mod shift_stability;

pub use seeded_rng::SeededRng;

use entrybound::chunker::{ChunkRange as ProductionChunkRange, ChunkingParameters};
use entrybound::diagnostics::Diagnostic;

/// Thin re-export: production chunking, unmodified. Kept here (rather than
/// asking every caller to depend on `entrybound` directly just to chunk)
/// so this crate is the one place research code calls into `gear-norm-v1`,
/// and so [`algorithms::Algorithm::GearNormV1`] can dispatch to it without
/// a circular module reference.
pub fn production_chunk_ranges(
    plaintext: &[u8],
    parameters: ChunkingParameters,
) -> Result<Box<[ProductionChunkRange]>, Diagnostic> {
    entrybound::chunker::chunk_ranges(plaintext, parameters)
}

#[cfg(test)]
mod tests {
    use super::*;
    use entrybound::chunker::EXTREME_V2;

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
    fn production_wrapper_matches_entrybound_directly() {
        let data = pseudo_random_bytes(300_000, 3);
        let via_wrapper = production_chunk_ranges(&data, EXTREME_V2).unwrap();
        let direct = entrybound::chunker::chunk_ranges(&data, EXTREME_V2).unwrap();
        assert_eq!(via_wrapper, direct);
    }

    /// The dispatch-level guarantee every subcommand relies on: routing
    /// through `Algorithm::GearNormV1` produces exactly the same ranges as
    /// calling production directly, just wrapped in this crate's own
    /// [`algorithms::ChunkRange`] shape.
    #[test]
    fn gear_norm_v1_algorithm_matches_production_directly() {
        let data = pseudo_random_bytes(300_000, 4);
        let direct = entrybound::chunker::chunk_ranges(&data, EXTREME_V2).unwrap();
        let via_algorithm = algorithms::Algorithm::GearNormV1(EXTREME_V2)
            .chunk_ranges(&data)
            .unwrap();
        assert_eq!(via_algorithm.len(), direct.len());
        for (a, b) in via_algorithm.iter().zip(direct.iter()) {
            assert_eq!(a.start, b.start);
            assert_eq!(a.end, b.end);
        }
    }
}
