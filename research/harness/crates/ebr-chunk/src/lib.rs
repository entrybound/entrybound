//! Chunking/dedup research: production `gear-norm-v1`
//! (`entrybound::chunker`) versus research-only alternative CDC
//! implementations, on the same input. See `README.md`.

pub mod alt_cdc;

use entrybound::chunker::{ChunkRange as ProductionChunkRange, ChunkingParameters};
use entrybound::diagnostics::Diagnostic;
use serde::Serialize;

/// Thin re-export: production chunking, unmodified. Kept here (rather than
/// asking every caller to depend on `entrybound` directly just to chunk)
/// so this crate is the one place research code calls into `gear-norm-v1`.
pub fn production_chunk_ranges(
    plaintext: &[u8],
    parameters: ChunkingParameters,
) -> Result<Box<[ProductionChunkRange]>, Diagnostic> {
    entrybound::chunker::chunk_ranges(plaintext, parameters)
}

/// Summary statistics for one chunker's output over one input.
#[derive(Debug, Clone, Serialize)]
pub struct ChunkerSummary {
    pub chunker_id: String,
    pub input_bytes: u64,
    pub chunk_count: u64,
    pub min_chunk_bytes: u64,
    pub max_chunk_bytes: u64,
    pub mean_chunk_bytes: f64,
    /// Distinct chunk byte-content count, i.e. deduplication after chunking:
    /// `unique_chunk_count <= chunk_count`, with equality meaning no
    /// duplicate chunks were found.
    pub unique_chunk_count: u64,
}

fn summarize(
    chunker_id: &str,
    input_bytes: usize,
    ranges: &[(usize, usize)],
    data: &[u8],
) -> ChunkerSummary {
    let chunk_count = ranges.len() as u64;
    let lengths: Vec<u64> = ranges.iter().map(|&(s, e)| (e - s) as u64).collect();
    let min_chunk_bytes = lengths.iter().copied().min().unwrap_or(0);
    let max_chunk_bytes = lengths.iter().copied().max().unwrap_or(0);
    let mean_chunk_bytes = if lengths.is_empty() {
        0.0
    } else {
        lengths.iter().sum::<u64>() as f64 / lengths.len() as f64
    };

    let unique_chunk_count = {
        let mut seen = std::collections::HashSet::new();
        for &(s, e) in ranges {
            seen.insert(ebr_common::hash::sha256_hex(&data[s..e]));
        }
        seen.len() as u64
    };

    ChunkerSummary {
        chunker_id: chunker_id.to_string(),
        input_bytes: input_bytes as u64,
        chunk_count,
        min_chunk_bytes,
        max_chunk_bytes,
        mean_chunk_bytes,
        unique_chunk_count,
    }
}

/// Both chunkers' summaries over the same input, at matching size targets.
#[derive(Debug, Clone, Serialize)]
pub struct Comparison {
    pub production: ChunkerSummary,
    pub alternative: ChunkerSummary,
}

/// Runs production `gear-norm-v1` and the research-only Buzhash alternative
/// ([`alt_cdc`], parameterized to the same min/target/max via
/// [`alt_cdc::BuzhashParameters::matching`]) over the same `data`, and
/// summarizes both.
pub fn compare(
    data: &[u8],
    production_parameters: ChunkingParameters,
) -> Result<Comparison, Diagnostic> {
    let production_ranges = production_chunk_ranges(data, production_parameters)?;
    let production_pairs: Vec<(usize, usize)> =
        production_ranges.iter().map(|r| (r.start, r.end)).collect();
    let production = summarize(
        production_parameters.chunker_id,
        data.len(),
        &production_pairs,
        data,
    );

    let alt_parameters = alt_cdc::BuzhashParameters::matching(production_parameters);
    let alt_ranges = alt_cdc::chunk_ranges(data, &alt_parameters);
    let alt_pairs: Vec<(usize, usize)> = alt_ranges.iter().map(|r| (r.start, r.end)).collect();
    let alternative = summarize("buzhash-v1 (research-only)", data.len(), &alt_pairs, data);

    Ok(Comparison {
        production,
        alternative,
    })
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

    #[test]
    fn compare_runs_both_chunkers_and_covers_the_input() {
        // EXTREME_V2's minimum (32 KiB) needs a reasonably large input to see
        // more than one chunk from either chunker.
        let data = pseudo_random_bytes(600_000, 11);
        let comparison = compare(&data, EXTREME_V2).unwrap();

        assert_eq!(comparison.production.input_bytes, data.len() as u64);
        assert_eq!(comparison.alternative.input_bytes, data.len() as u64);
        assert!(comparison.production.chunk_count >= 1);
        assert!(comparison.alternative.chunk_count >= 1);
        assert_eq!(comparison.production.chunker_id, EXTREME_V2.chunker_id);
    }
}
