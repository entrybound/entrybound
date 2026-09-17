//! Fixed-size chunking: a byte-count baseline with no content dependence.
//!
//! Every chunk is exactly `chunk_size` bytes except possibly the last, which
//! holds whatever remains (nonzero, and only ever shorter than `chunk_size`
//! when the input length is not a multiple of it). This is the standard
//! contrast case for any CDC algorithm's headline claim: unlike
//! [`super::rabin`], [`super::buzhash`], [`super::fastcdc2016`], etc.,
//! inserting or deleting even one byte shifts *every* following boundary,
//! not just the ones near the edit -- the `shift-stability` subcommand
//! measures this directly, and fixed-size is the algorithm that should show
//! the worst preserved-boundary fraction of the set.

use super::ChunkRange;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Params {
    pub chunk_size: usize,
}

impl Params {
    #[must_use]
    pub fn algorithm_id(&self) -> String {
        format!("fixed-size-v1/size-{}", self.chunk_size)
    }
}

/// Splits `data` into `params.chunk_size`-byte ranges. Empty input has no
/// ranges. Panics if `chunk_size == 0`.
#[must_use]
pub fn chunk_ranges(data: &[u8], params: &Params) -> Vec<ChunkRange> {
    assert!(
        params.chunk_size > 0,
        "fixed-size chunk_size must be nonzero"
    );
    let mut ranges = Vec::new();
    let mut start = 0usize;
    while start < data.len() {
        let end = (start + params.chunk_size).min(data.len());
        ranges.push(ChunkRange { start, end });
        start = end;
    }
    ranges
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_has_no_ranges() {
        assert!(chunk_ranges(&[], &Params { chunk_size: 16 }).is_empty());
    }

    #[test]
    fn every_chunk_is_exact_except_possibly_the_last() {
        let data = vec![0u8; 1000];
        let params = Params { chunk_size: 64 };
        let ranges = chunk_ranges(&data, &params);
        // 1000 = 15*64 + 40: 15 full chunks plus one 40-byte remainder.
        assert_eq!(ranges.len(), 16);
        for r in &ranges[..ranges.len() - 1] {
            assert_eq!(r.len(), 64);
        }
        let last = *ranges.last().unwrap();
        assert!(!last.is_empty() && last.len() <= 64);
        super::super::assert_valid_ranges(data.len(), &ranges, 1, 64);
    }

    #[test]
    fn exact_multiple_has_no_short_final_chunk() {
        let data = vec![7u8; 128];
        let ranges = chunk_ranges(&data, &Params { chunk_size: 64 });
        assert_eq!(ranges.len(), 2);
        assert!(ranges.iter().all(|r| r.len() == 64));
    }

    #[test]
    fn deterministic() {
        let data = vec![3u8; 999];
        let params = Params { chunk_size: 40 };
        assert_eq!(chunk_ranges(&data, &params), chunk_ranges(&data, &params));
    }

    #[test]
    fn a_single_byte_insertion_shifts_every_later_boundary() {
        // The defining contrast with CDC: fixed-size has no content
        // dependence, so an edit anywhere shifts every subsequent boundary
        // by exactly the edit's length, with zero survivors.
        let params = Params { chunk_size: 100 };
        let mut data: Vec<u8> = (0..10_000u32).map(|i| (i % 251) as u8).collect();
        let original = chunk_ranges(&data, &params);
        data.insert(5, 0xAB);
        let modified = chunk_ranges(&data, &params);

        // Exclude each side's final (EOF) boundary: it always lands at the
        // new data length regardless of chunking strategy, so it is not a
        // meaningful "survivor" for this comparison.
        let shifted: std::collections::HashSet<usize> = original[..original.len() - 1]
            .iter()
            .map(|r| r.end + 1)
            .collect();
        let modified_interior: std::collections::HashSet<usize> = modified[..modified.len() - 1]
            .iter()
            .map(|r| r.end)
            .collect();
        let survivors = shifted.intersection(&modified_interior).count();
        assert_eq!(
            survivors, 0,
            "fixed-size boundaries should not survive an insertion"
        );
    }
}
