//! `shift-stability` subcommand: apply a seeded insertion, deletion, and
//! overwrite (each independently, at a random offset, of a caller-given
//! size) and measure how many chunk boundaries survive and how many bytes
//! would need to be rewritten under content-addressed dedup.

use crate::algorithms::Algorithm;
use crate::seeded_rng::SeededRng;
use serde::Serialize;
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize)]
pub struct EditResult {
    pub kind: &'static str,
    pub offset: usize,
    pub old_len: usize,
    pub new_len: usize,
    pub original_chunk_count: usize,
    pub modified_chunk_count: usize,
    /// Interior boundaries only (every range's `end` except the last,
    /// which always trivially lands at EOF regardless of chunking
    /// strategy and so is excluded from both the numerator and
    /// denominator here).
    pub original_boundary_count: usize,
    pub preserved_boundary_count: usize,
    pub preserved_boundary_fraction: f64,
    pub modified_bytes: u64,
    /// Bytes in modified chunks whose content (by SHA-256) does not appear
    /// anywhere in the original chunk set -- i.e. bytes a content-addressed
    /// store would actually have to write again after this edit.
    pub changed_bytes: u64,
    pub changed_bytes_fraction: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ShiftStabilityResult {
    pub algorithm_id: String,
    pub input_bytes: usize,
    pub seed: u64,
    pub edits: Vec<EditResult>,
}

/// Runs three independent scenarios over `data` -- an insertion, a
/// deletion, and an overwrite, each of the given size at its own seeded
/// random offset -- and reports [`EditResult`] for each. Each scenario
/// starts from the unmodified `data`, so the three are not compounded.
pub fn measure(
    data: &[u8],
    algorithm: &Algorithm,
    seed: u64,
    insert_bytes: usize,
    delete_bytes: usize,
    overwrite_bytes: usize,
) -> Result<ShiftStabilityResult, String> {
    let mut rng = SeededRng::new(seed);
    let mut edits = Vec::new();

    if insert_bytes > 0 {
        let offset = rng.next_below(data.len() + 1);
        let filler = rng.next_bytes(insert_bytes);
        edits.push(measure_edit(data, algorithm, "insert", offset, 0, &filler)?);
    }
    if delete_bytes > 0 && delete_bytes <= data.len() {
        let offset = rng.next_below(data.len() - delete_bytes + 1);
        edits.push(measure_edit(
            data,
            algorithm,
            "delete",
            offset,
            delete_bytes,
            &[],
        )?);
    }
    if overwrite_bytes > 0 && overwrite_bytes <= data.len() {
        let offset = rng.next_below(data.len() - overwrite_bytes + 1);
        let filler = rng.next_bytes(overwrite_bytes);
        edits.push(measure_edit(
            data,
            algorithm,
            "overwrite",
            offset,
            overwrite_bytes,
            &filler,
        )?);
    }

    Ok(ShiftStabilityResult {
        algorithm_id: algorithm.algorithm_id(),
        input_bytes: data.len(),
        seed,
        edits,
    })
}

fn measure_edit(
    original: &[u8],
    algorithm: &Algorithm,
    kind: &'static str,
    offset: usize,
    old_len: usize,
    new_bytes: &[u8],
) -> Result<EditResult, String> {
    let original_ranges = algorithm.chunk_ranges(original)?;

    let mut modified = Vec::with_capacity(original.len() - old_len + new_bytes.len());
    modified.extend_from_slice(&original[..offset]);
    modified.extend_from_slice(new_bytes);
    modified.extend_from_slice(&original[offset + old_len..]);
    let modified_ranges = algorithm.chunk_ranges(&modified)?;

    let shift = new_bytes.len() as isize - old_len as isize;
    let interior_original = &original_ranges[..original_ranges.len().saturating_sub(1)];
    let modified_boundaries: HashSet<usize> = modified_ranges.iter().map(|r| r.end).collect();

    let mut preserved = 0usize;
    for r in interior_original {
        let p = r.end;
        if p <= offset {
            if modified_boundaries.contains(&p) {
                preserved += 1;
            }
        } else if p >= offset + old_len {
            let shifted = (p as isize + shift) as usize;
            if modified_boundaries.contains(&shifted) {
                preserved += 1;
            }
        }
        // A boundary strictly inside the edited region cannot survive it;
        // it is counted in the denominator but never matched.
    }

    let original_hashes: HashSet<String> = original_ranges
        .iter()
        .map(|r| ebr_common::hash::sha256_hex(&original[r.start..r.end]))
        .collect();
    let mut changed_bytes = 0u64;
    for r in &modified_ranges {
        let hash = ebr_common::hash::sha256_hex(&modified[r.start..r.end]);
        if !original_hashes.contains(&hash) {
            changed_bytes += r.len() as u64;
        }
    }

    let original_boundary_count = interior_original.len();
    Ok(EditResult {
        kind,
        offset,
        old_len,
        new_len: new_bytes.len(),
        original_chunk_count: original_ranges.len(),
        modified_chunk_count: modified_ranges.len(),
        original_boundary_count,
        preserved_boundary_count: preserved,
        preserved_boundary_fraction: if original_boundary_count == 0 {
            1.0
        } else {
            preserved as f64 / original_boundary_count as f64
        },
        modified_bytes: modified.len() as u64,
        changed_bytes,
        changed_bytes_fraction: changed_bytes as f64 / modified.len().max(1) as f64,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algorithms::{buzhash, fixed_size};

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
    fn fixed_size_survives_almost_no_boundaries_past_the_edit() {
        // `measure`'s public seeded API places the edit at a random offset,
        // and boundaries strictly before it trivially "survive" for *any*
        // chunking strategy (that region of bytes did not change) -- that
        // is real, but it is not the property this test is after. Calling
        // `measure_edit` directly with an offset near the very start
        // isolates fixed-size's actual headline weakness: it recovers
        // almost none of the boundaries *after* an edit, because they are
        // defined purely by cumulative byte count, not content.
        let data = pseudo_random_bytes(200_000, 1);
        let algorithm = Algorithm::FixedSize(fixed_size::Params { chunk_size: 4096 });
        let filler = vec![0xABu8; 16];
        for edit in [
            measure_edit(&data, &algorithm, "insert", 100, 0, &filler).unwrap(),
            measure_edit(&data, &algorithm, "delete", 100, 16, &[]).unwrap(),
        ] {
            assert!(
                edit.preserved_boundary_fraction < 0.05,
                "{}: expected fixed-size to preserve almost no boundaries after an edit near the start, got {}",
                edit.kind,
                edit.preserved_boundary_fraction
            );
        }
    }

    #[test]
    fn cdc_survives_most_boundaries_far_from_the_edit() {
        let data = pseudo_random_bytes(2_000_000, 3);
        let algorithm = Algorithm::Buzhash(buzhash::Params {
            min_size: 1024,
            target_size: 8192,
            max_size: 32_768,
            window: 64,
        });
        let result = measure(&data, &algorithm, 11, 32, 32, 32).unwrap();
        for edit in &result.edits {
            assert!(
                edit.preserved_boundary_fraction > 0.8,
                "{}: expected CDC to preserve most boundaries far from a small edit, got {}",
                edit.kind,
                edit.preserved_boundary_fraction
            );
            // Only a small handful of chunks near the edit should need
            // rewriting, not the whole file.
            assert!(
                edit.changed_bytes_fraction < 0.1,
                "{}: expected a small edit to only require rewriting a small fraction of bytes, got {}",
                edit.kind,
                edit.changed_bytes_fraction
            );
        }
    }

    #[test]
    fn an_overwrite_changes_only_the_touched_chunks_bytes() {
        // Overwrite does not change length, so every boundary outside the
        // touched region should be an exact (unshifted) survivor.
        let data = pseudo_random_bytes(500_000, 5);
        let algorithm = Algorithm::FixedSize(fixed_size::Params { chunk_size: 4096 });
        let result = measure(&data, &algorithm, 13, 0, 0, 50).unwrap();
        let overwrite = result.edits.iter().find(|e| e.kind == "overwrite").unwrap();
        assert!(overwrite.preserved_boundary_fraction > 0.9);
    }
}
