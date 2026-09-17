//! `dedup` subcommand: cross-version deduplication ratio and estimated
//! index/reference overhead, using production's own physical-cost
//! constants applied to whichever algorithm produced the chunk ranges.

use crate::algorithms::Algorithm;
use entrybound::chunker::{
    CHUNK_FRAME_OVERHEAD_BYTES, CHUNK_REFERENCE_OVERHEAD_BYTES, INDEX_ENTRY_OVERHEAD_BYTES,
};
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize)]
pub struct CostConstants {
    pub chunk_frame_overhead_bytes: u64,
    pub index_entry_overhead_bytes: u64,
    pub chunk_reference_overhead_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DedupResult {
    pub algorithm_id: String,
    pub version_count: usize,
    pub total_logical_bytes: u64,
    pub unique_chunk_count: u64,
    pub unique_chunk_bytes: u64,
    pub manifest_chunk_references: u64,
    /// `total_logical_bytes / manifest_chunk_references`: the realized mean
    /// chunk size. Dedup ratios of different algorithms are only comparable
    /// at matched realized means (harness review round 1, finding R1-09).
    pub mean_chunk_bytes: f64,
    /// `unique_chunk_bytes / unique_chunk_count`.
    pub mean_unique_chunk_bytes: f64,
    /// `total_logical_bytes / unique_chunk_bytes`; `1.0` when nothing is
    /// shared across (or within) versions, higher is better.
    pub dedup_ratio: f64,
    /// `entrybound::chunker::evaluate`'s exact cost model applied to this
    /// algorithm's ranges: `unique_chunk_bytes +
    /// (CHUNK_FRAME_OVERHEAD_BYTES + INDEX_ENTRY_OVERHEAD_BYTES) *
    /// unique_chunk_count + CHUNK_REFERENCE_OVERHEAD_BYTES *
    /// manifest_chunk_references`.
    pub estimated_cost_bytes: u64,
    pub cost_constants: CostConstants,
}

/// Measures dedup over `versions` (one or more byte buffers -- a single
/// corpus item is `versions.len() == 1` and measures within-item dedup
/// only; multiple versions of the same logical item let this measure
/// cross-version dedup too). Applies production's cost constants
/// (`entrybound::chunker::{CHUNK_FRAME_OVERHEAD_BYTES,
/// INDEX_ENTRY_OVERHEAD_BYTES, CHUNK_REFERENCE_OVERHEAD_BYTES}`) so an
/// alternative algorithm's estimated overhead is directly comparable to
/// production's own `ChunkingEvaluation`.
pub fn measure(versions: &[Vec<u8>], algorithm: &Algorithm) -> Result<DedupResult, String> {
    let mut unique_chunks: HashMap<String, u64> = HashMap::new();
    let mut manifest_chunk_references = 0u64;
    let mut total_logical_bytes = 0u64;

    for version in versions {
        total_logical_bytes += version.len() as u64;
        let ranges = algorithm.chunk_ranges(version)?;
        manifest_chunk_references += ranges.len() as u64;
        for range in &ranges {
            let bytes = &version[range.start..range.end];
            let hash = ebr_common::hash::sha256_hex(bytes);
            unique_chunks.entry(hash).or_insert(bytes.len() as u64);
        }
    }

    let unique_chunk_count = unique_chunks.len() as u64;
    let unique_chunk_bytes: u64 = unique_chunks.values().sum();
    let dedup_ratio = if unique_chunk_bytes == 0 {
        1.0
    } else {
        total_logical_bytes as f64 / unique_chunk_bytes as f64
    };
    let estimated_cost_bytes = unique_chunk_bytes
        + (CHUNK_FRAME_OVERHEAD_BYTES + INDEX_ENTRY_OVERHEAD_BYTES) * unique_chunk_count
        + CHUNK_REFERENCE_OVERHEAD_BYTES * manifest_chunk_references;

    Ok(DedupResult {
        algorithm_id: algorithm.algorithm_id(),
        version_count: versions.len(),
        total_logical_bytes,
        unique_chunk_count,
        unique_chunk_bytes,
        manifest_chunk_references,
        mean_chunk_bytes: if manifest_chunk_references == 0 {
            0.0
        } else {
            total_logical_bytes as f64 / manifest_chunk_references as f64
        },
        mean_unique_chunk_bytes: if unique_chunk_count == 0 {
            0.0
        } else {
            unique_chunk_bytes as f64 / unique_chunk_count as f64
        },
        dedup_ratio,
        estimated_cost_bytes,
        cost_constants: CostConstants {
            chunk_frame_overhead_bytes: CHUNK_FRAME_OVERHEAD_BYTES,
            index_entry_overhead_bytes: INDEX_ENTRY_OVERHEAD_BYTES,
            chunk_reference_overhead_bytes: CHUNK_REFERENCE_OVERHEAD_BYTES,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algorithms::fixed_size;

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
    fn a_single_version_with_no_internal_repetition_has_ratio_near_one() {
        let data = pseudo_random_bytes(50_000, 1);
        let algorithm = Algorithm::FixedSize(fixed_size::Params { chunk_size: 4096 });
        let result = measure(&[data], &algorithm).unwrap();
        assert!(
            result.dedup_ratio < 1.05,
            "expected close to no dedup for one non-repeating version, got {}",
            result.dedup_ratio
        );
    }

    #[test]
    fn two_identical_versions_dedup_almost_completely() {
        let data = pseudo_random_bytes(50_000, 2);
        let algorithm = Algorithm::FixedSize(fixed_size::Params { chunk_size: 4096 });
        let result = measure(&[data.clone(), data], &algorithm).unwrap();
        // Two byte-identical versions chunked deterministically produce the
        // exact same chunk set, so unique bytes should equal one version's
        // worth and the ratio should be almost exactly 2.0.
        assert!(
            (result.dedup_ratio - 2.0).abs() < 0.01,
            "expected dedup_ratio close to 2.0 for two identical versions, got {}",
            result.dedup_ratio
        );
        assert_eq!(result.version_count, 2);
    }

    #[test]
    fn cost_constants_match_production_exactly() {
        let algorithm = Algorithm::FixedSize(fixed_size::Params { chunk_size: 4096 });
        let result = measure(&[vec![1, 2, 3]], &algorithm).unwrap();
        assert_eq!(result.cost_constants.chunk_frame_overhead_bytes, 64);
        assert_eq!(result.cost_constants.index_entry_overhead_bytes, 100);
        assert_eq!(result.cost_constants.chunk_reference_overhead_bytes, 40);
    }
}
