//! `boundaries` subcommand: chunk-size histogram, quantiles, count, and a
//! throughput sample for one algorithm run over one input.

use crate::algorithms::Algorithm;
use serde::Serialize;
use std::time::Instant;

#[derive(Debug, Clone, Serialize)]
pub struct HistogramBucket {
    pub lower_bytes: u64,
    pub upper_bytes: u64,
    pub count: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct BoundariesResult {
    pub algorithm_id: String,
    pub input_bytes: u64,
    pub chunk_count: u64,
    pub min_chunk_bytes: u64,
    pub max_chunk_bytes: u64,
    pub mean_chunk_bytes: f64,
    pub p50_chunk_bytes: u64,
    pub p90_chunk_bytes: u64,
    pub p99_chunk_bytes: u64,
    pub histogram: Vec<HistogramBucket>,
    /// One in-process wall-clock chunking pass. Per this harness's shared
    /// rules (`research/PROGRESS.md`): this is a smoke measurement, **not**
    /// decision-grade timing -- no quiet-machine guard, no repetitions, and
    /// it shares a process with everything else this binary does.
    pub throughput_bytes_per_sec_smoke_non_decision_grade: f64,
}

/// Runs `algorithm` over `data` once and summarizes the resulting chunk-size
/// distribution into `bucket_count` equal-width histogram buckets (clamped
/// to at least 1).
pub fn measure(
    data: &[u8],
    algorithm: &Algorithm,
    bucket_count: usize,
) -> Result<BoundariesResult, String> {
    let bucket_count = bucket_count.max(1);
    let started = Instant::now();
    let ranges = algorithm.chunk_ranges(data)?;
    let elapsed = started.elapsed();

    let mut lengths: Vec<u64> = ranges.iter().map(|r| r.len() as u64).collect();
    lengths.sort_unstable();

    let chunk_count = lengths.len() as u64;
    let min_chunk_bytes = lengths.first().copied().unwrap_or(0);
    let max_chunk_bytes = lengths.last().copied().unwrap_or(0);
    let mean_chunk_bytes = if lengths.is_empty() {
        0.0
    } else {
        lengths.iter().sum::<u64>() as f64 / lengths.len() as f64
    };
    let histogram = build_histogram(&lengths, bucket_count);

    let elapsed_secs = elapsed.as_secs_f64();
    let throughput = if elapsed_secs > 0.0 {
        data.len() as f64 / elapsed_secs
    } else {
        0.0
    };

    Ok(BoundariesResult {
        algorithm_id: algorithm.algorithm_id(),
        input_bytes: data.len() as u64,
        chunk_count,
        min_chunk_bytes,
        max_chunk_bytes,
        mean_chunk_bytes,
        p50_chunk_bytes: quantile(&lengths, 0.50),
        p90_chunk_bytes: quantile(&lengths, 0.90),
        p99_chunk_bytes: quantile(&lengths, 0.99),
        histogram,
        throughput_bytes_per_sec_smoke_non_decision_grade: throughput,
    })
}

/// Nearest-rank quantile of an already-sorted slice. `q` is in `[0, 1]`.
fn quantile(sorted: &[u64], q: f64) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let idx = (q * (sorted.len() - 1) as f64).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

fn build_histogram(sorted_lengths: &[u64], bucket_count: usize) -> Vec<HistogramBucket> {
    if sorted_lengths.is_empty() {
        return Vec::new();
    }
    let min = *sorted_lengths.first().unwrap();
    let max = *sorted_lengths.last().unwrap();
    if min == max {
        return vec![HistogramBucket {
            lower_bytes: min,
            upper_bytes: max,
            count: sorted_lengths.len() as u64,
        }];
    }
    let width = (max - min) as f64 / bucket_count as f64;
    let mut counts = vec![0u64; bucket_count];
    for &len in sorted_lengths {
        let idx = (((len - min) as f64) / width) as usize;
        counts[idx.min(bucket_count - 1)] += 1;
    }
    counts
        .into_iter()
        .enumerate()
        .map(|(i, count)| {
            let lower = min + (i as f64 * width) as u64;
            let upper = if i + 1 == bucket_count {
                max
            } else {
                min + ((i + 1) as f64 * width) as u64
            };
            HistogramBucket {
                lower_bytes: lower,
                upper_bytes: upper,
                count,
            }
        })
        .collect()
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
    fn histogram_bucket_counts_sum_to_the_chunk_count() {
        let data = pseudo_random_bytes(100_000, 1);
        let algorithm = Algorithm::FixedSize(fixed_size::Params { chunk_size: 777 });
        let result = measure(&data, &algorithm, 10).unwrap();
        let total: u64 = result.histogram.iter().map(|b| b.count).sum();
        assert_eq!(total, result.chunk_count);
    }

    #[test]
    fn quantiles_are_nondecreasing_and_within_range() {
        let data = pseudo_random_bytes(200_000, 2);
        let algorithm = Algorithm::FixedSize(fixed_size::Params { chunk_size: 333 });
        let result = measure(&data, &algorithm, 5).unwrap();
        assert!(result.p50_chunk_bytes <= result.p90_chunk_bytes);
        assert!(result.p90_chunk_bytes <= result.p99_chunk_bytes);
        assert!(result.min_chunk_bytes <= result.p50_chunk_bytes);
        assert!(result.p99_chunk_bytes <= result.max_chunk_bytes);
    }

    #[test]
    fn a_single_bucket_input_does_not_divide_by_zero() {
        // Every chunk the same size (fixed-size chunking over an exact
        // multiple) makes min == max; the histogram must not panic.
        let data = vec![0u8; 1000];
        let algorithm = Algorithm::FixedSize(fixed_size::Params { chunk_size: 100 });
        let result = measure(&data, &algorithm, 10).unwrap();
        assert_eq!(result.histogram.len(), 1);
        assert_eq!(result.histogram[0].count, result.chunk_count);
    }
}
