//! `random-access` subcommand: bytes that must be physically read to serve
//! a random logical byte range, given one algorithm's chunk plan.
//!
//! A content object stored as a sequence of chunks can only be read at
//! chunk granularity: satisfying a logical range `[offset, offset +
//! range_bytes)` requires reading every chunk that overlaps it, in full,
//! even though most of those chunks' bytes fall outside the requested
//! range. This subcommand samples random ranges and reports the
//! amplification factor (physical bytes read / requested bytes) that
//! results -- the smaller a chunker's chunks near the access point, the
//! less it costs to seek into the middle of an object.

use crate::algorithms::Algorithm;
use crate::seeded_rng::SeededRng;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct RandomAccessSample {
    pub offset: usize,
    pub requested_bytes: usize,
    pub physical_bytes_read: usize,
    pub chunks_touched: usize,
    pub amplification: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RandomAccessResult {
    pub algorithm_id: String,
    pub input_bytes: usize,
    pub chunk_count: usize,
    pub sample_count: usize,
    pub mean_amplification: f64,
    pub median_amplification: f64,
    pub max_amplification: f64,
    pub samples: Vec<RandomAccessSample>,
}

/// Samples `sample_count` random `range_bytes`-wide logical ranges (seeded
/// by `seed`) and measures each one's random-access amplification under
/// `algorithm`'s chunk plan. Returns an error for empty `data` (there is no
/// meaningful random range to sample).
pub fn measure(
    data: &[u8],
    algorithm: &Algorithm,
    range_bytes: usize,
    sample_count: usize,
    seed: u64,
) -> Result<RandomAccessResult, String> {
    if data.is_empty() {
        return Err("random-access amplification needs non-empty input".to_string());
    }
    let ranges = algorithm.chunk_ranges(data)?;
    let mut rng = SeededRng::new(seed);
    let mut samples = Vec::with_capacity(sample_count);

    for _ in 0..sample_count {
        let offset = rng.next_below(data.len());
        let end = (offset + range_bytes).min(data.len());
        let requested_bytes = end - offset;
        let touched: Vec<_> = ranges
            .iter()
            .filter(|r| r.start < end && r.end > offset)
            .collect();
        let physical_bytes_read: usize = touched.iter().map(|r| r.len()).sum();
        let amplification = physical_bytes_read as f64 / requested_bytes.max(1) as f64;
        samples.push(RandomAccessSample {
            offset,
            requested_bytes,
            physical_bytes_read,
            chunks_touched: touched.len(),
            amplification,
        });
    }

    let mut sorted_amp: Vec<f64> = samples.iter().map(|s| s.amplification).collect();
    sorted_amp.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mean_amplification = sorted_amp.iter().sum::<f64>() / sorted_amp.len().max(1) as f64;
    let median_amplification = if sorted_amp.is_empty() {
        0.0
    } else {
        sorted_amp[sorted_amp.len() / 2]
    };
    let max_amplification = sorted_amp.last().copied().unwrap_or(0.0);

    Ok(RandomAccessResult {
        algorithm_id: algorithm.algorithm_id(),
        input_bytes: data.len(),
        chunk_count: ranges.len(),
        sample_count: samples.len(),
        mean_amplification,
        median_amplification,
        max_amplification,
        samples,
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
    fn rejects_empty_input() {
        let algorithm = Algorithm::FixedSize(fixed_size::Params { chunk_size: 100 });
        assert!(measure(&[], &algorithm, 10, 5, 1).is_err());
    }

    #[test]
    fn amplification_is_never_below_one() {
        let data = pseudo_random_bytes(100_000, 1);
        let algorithm = Algorithm::FixedSize(fixed_size::Params { chunk_size: 4096 });
        let result = measure(&data, &algorithm, 100, 30, 5).unwrap();
        for sample in &result.samples {
            assert!(
                sample.physical_bytes_read >= sample.requested_bytes,
                "physical read must cover at least the requested range"
            );
            assert!(sample.amplification >= 1.0 - 1e-9);
        }
    }

    #[test]
    fn smaller_chunks_amplify_less_for_the_same_request() {
        let data = pseudo_random_bytes(1_000_000, 2);
        let coarse = Algorithm::FixedSize(fixed_size::Params { chunk_size: 65536 });
        let fine = Algorithm::FixedSize(fixed_size::Params { chunk_size: 4096 });
        let coarse_result = measure(&data, &coarse, 100, 50, 9).unwrap();
        let fine_result = measure(&data, &fine, 100, 50, 9).unwrap();
        assert!(
            fine_result.mean_amplification < coarse_result.mean_amplification,
            "finer chunking should amplify less: fine={} coarse={}",
            fine_result.mean_amplification,
            coarse_result.mean_amplification
        );
    }

    #[test]
    fn deterministic_given_the_same_seed() {
        let data = pseudo_random_bytes(50_000, 3);
        let algorithm = Algorithm::FixedSize(fixed_size::Params { chunk_size: 1024 });
        let a = measure(&data, &algorithm, 50, 10, 42).unwrap();
        let b = measure(&data, &algorithm, 50, 10, 42).unwrap();
        assert_eq!(
            a.samples.iter().map(|s| s.offset).collect::<Vec<_>>(),
            b.samples.iter().map(|s| s.offset).collect::<Vec<_>>()
        );
    }
}
