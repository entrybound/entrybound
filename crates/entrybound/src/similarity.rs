//! Deterministic creation-time similarity analysis over unique plaintext Chunks.
//!
//! Similarity is only a bounded planning hint. SHA-256 remains the sole
//! authority for exact Chunk equality and no fingerprint enters archive
//! identity or decoding.

use std::collections::{BTreeMap, BTreeSet};

use crate::eam::{Chunk, Digest};
use crate::identity::sha256_exact;

pub const SIMILARITY_ID: &str = "bottom-k-shingle-v1";
const SHINGLE_BYTES: usize = 32;
const SHINGLE_STRIDE: usize = 16;
const MAX_SCANNED_SHINGLES: usize = 4096;
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// Frozen profile-owned similarity and dictionary-training bounds.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SimilarityPolicy {
    pub policy_id: &'static str,
    pub enabled: bool,
    pub sketch_items: usize,
    pub threshold_milli: u16,
    pub minimum_cohort_chunks: usize,
    pub maximum_cohort_chunks: usize,
    pub maximum_candidate_cohorts: usize,
    pub dictionary_bytes: usize,
    pub maximum_training_samples: usize,
    pub maximum_sample_bytes: usize,
}

pub const FAST_V3_SIMILARITY: SimilarityPolicy = SimilarityPolicy {
    policy_id: "similarity-disabled/fast-v3",
    enabled: false,
    sketch_items: 0,
    threshold_milli: 0,
    minimum_cohort_chunks: usize::MAX,
    maximum_cohort_chunks: 0,
    maximum_candidate_cohorts: 0,
    dictionary_bytes: 0,
    maximum_training_samples: 0,
    maximum_sample_bytes: 0,
};

pub const BALANCED_V3_SIMILARITY: SimilarityPolicy = SimilarityPolicy {
    policy_id: "bottom-k-shingle-v1/k8/t500/min8/max32",
    enabled: true,
    sketch_items: 8,
    threshold_milli: 500,
    minimum_cohort_chunks: 8,
    maximum_cohort_chunks: 32,
    maximum_candidate_cohorts: 64,
    dictionary_bytes: 8 * 1024,
    maximum_training_samples: 16,
    maximum_sample_bytes: 16 * 1024,
};

pub const DENSE_V3_SIMILARITY: SimilarityPolicy = SimilarityPolicy {
    policy_id: "bottom-k-shingle-v1/k16/t375/min4/max64",
    enabled: true,
    sketch_items: 16,
    threshold_milli: 375,
    minimum_cohort_chunks: 4,
    maximum_cohort_chunks: 64,
    maximum_candidate_cohorts: 128,
    dictionary_bytes: 16 * 1024,
    maximum_training_samples: 32,
    maximum_sample_bytes: 32 * 1024,
};

pub const EXTREME_V3_SIMILARITY: SimilarityPolicy = SimilarityPolicy {
    policy_id: "bottom-k-shingle-v1/k32/t250/min3/max128",
    enabled: true,
    sketch_items: 32,
    threshold_milli: 250,
    minimum_cohort_chunks: 3,
    maximum_cohort_chunks: 128,
    maximum_candidate_cohorts: 256,
    dictionary_bytes: 32 * 1024,
    maximum_training_samples: 64,
    maximum_sample_bytes: 64 * 1024,
};

/// One deterministic similarity cohort. Chunk IDs are digest ordered.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SimilarityCohort {
    pub cohort_id: Digest,
    pub chunks: Box<[Digest]>,
    pub logical_bytes: u64,
}

#[derive(Clone, Debug)]
struct Fingerprint {
    items: Box<[u64]>,
}

#[derive(Clone, Debug)]
struct WorkingCohort {
    cohort_id: Digest,
    leader: Fingerprint,
    chunks: Vec<Digest>,
    logical_bytes: u64,
}

/// Builds reproducible bounded cohorts from digest-ordered unique Chunks.
#[must_use]
pub fn cluster(
    chunks: &BTreeMap<Digest, Chunk>,
    policy: SimilarityPolicy,
) -> Vec<SimilarityCohort> {
    if !policy.enabled || chunks.is_empty() {
        return Vec::new();
    }
    let mut cohorts = Vec::<WorkingCohort>::new();
    let mut buckets = BTreeMap::<u64, Vec<usize>>::new();
    for chunk in chunks.values() {
        let fingerprint = fingerprint(&chunk.plaintext, policy.sketch_items);
        let mut candidates = BTreeSet::new();
        for item in &fingerprint.items {
            if let Some(cohort_ids) = buckets.get(item) {
                for cohort in cohort_ids {
                    if candidates.len() == policy.maximum_candidate_cohorts {
                        break;
                    }
                    candidates.insert(*cohort);
                }
            }
        }
        let mut selected = None::<(usize, usize)>;
        for cohort_index in candidates {
            let cohort = &cohorts[cohort_index];
            if cohort.chunks.len() == policy.maximum_cohort_chunks {
                continue;
            }
            let shared = intersection_count(&fingerprint.items, &cohort.leader.items);
            let denominator = fingerprint.items.len().min(cohort.leader.items.len());
            if denominator == 0
                || shared * 1_000 < denominator * usize::from(policy.threshold_milli)
            {
                continue;
            }
            if selected.is_none_or(|(best_shared, best_index)| {
                shared > best_shared
                    || (shared == best_shared && cohort.cohort_id < cohorts[best_index].cohort_id)
            }) {
                selected = Some((shared, cohort_index));
            }
        }
        if let Some((_, cohort_index)) = selected {
            let cohort = &mut cohorts[cohort_index];
            cohort.chunks.push(chunk.chunk_id);
            cohort.logical_bytes = cohort.logical_bytes.saturating_add(chunk.logical_len);
        } else {
            let cohort_index = cohorts.len();
            let cohort_id = cohort_id(chunk.chunk_id, policy);
            for item in &fingerprint.items {
                buckets.entry(*item).or_default().push(cohort_index);
            }
            cohorts.push(WorkingCohort {
                cohort_id,
                leader: fingerprint,
                chunks: vec![chunk.chunk_id],
                logical_bytes: chunk.logical_len,
            });
        }
    }
    cohorts
        .into_iter()
        .filter(|cohort| cohort.chunks.len() >= policy.minimum_cohort_chunks)
        .map(|mut cohort| {
            cohort.chunks.sort_unstable();
            SimilarityCohort {
                cohort_id: cohort.cohort_id,
                chunks: cohort.chunks.into_boxed_slice(),
                logical_bytes: cohort.logical_bytes,
            }
        })
        .collect()
}

fn fingerprint(bytes: &[u8], sketch_items: usize) -> Fingerprint {
    if bytes.is_empty() || sketch_items == 0 {
        return Fingerprint {
            items: Box::default(),
        };
    }
    let windows = if bytes.len() < SHINGLE_BYTES {
        1
    } else {
        (bytes.len() - SHINGLE_BYTES) / SHINGLE_STRIDE + 1
    };
    let sample_step = windows.div_ceil(MAX_SCANNED_SHINGLES).max(1);
    let mut minima = BTreeSet::new();
    for window in (0..windows).step_by(sample_step) {
        let start = if bytes.len() < SHINGLE_BYTES {
            0
        } else {
            window * SHINGLE_STRIDE
        };
        let end = start.saturating_add(SHINGLE_BYTES).min(bytes.len());
        minima.insert(fnv1a(&bytes[start..end]));
        if minima.len() > sketch_items
            && let Some(largest) = minima.last().copied()
        {
            minima.remove(&largest);
        }
    }
    Fingerprint {
        items: minima.into_iter().collect::<Vec<_>>().into_boxed_slice(),
    }
}

fn intersection_count(left: &[u64], right: &[u64]) -> usize {
    let mut left_index = 0;
    let mut right_index = 0;
    let mut shared = 0;
    while left_index < left.len() && right_index < right.len() {
        match left[left_index].cmp(&right[right_index]) {
            std::cmp::Ordering::Less => left_index += 1,
            std::cmp::Ordering::Greater => right_index += 1,
            std::cmp::Ordering::Equal => {
                shared += 1;
                left_index += 1;
                right_index += 1;
            }
        }
    }
    shared
}

fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(FNV_OFFSET, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(FNV_PRIME)
    })
}

fn cohort_id(leader: Digest, policy: SimilarityPolicy) -> Digest {
    let mut input = Vec::new();
    input.extend_from_slice(b"entrybound/similarity-cohort/v1\0");
    input.extend_from_slice(
        &u64::try_from(policy.policy_id.len())
            .unwrap_or(u64::MAX)
            .to_be_bytes(),
    );
    input.extend_from_slice(policy.policy_id.as_bytes());
    input.extend_from_slice(leader.as_bytes());
    sha256_exact(&input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::sha256_exact;
    use crate::planner::UNPLANNED_PLAN_ID;

    #[test]
    fn localized_differences_form_one_deterministic_cohort() {
        let mut chunks = BTreeMap::new();
        let base = (0..64 * 1024)
            .map(|index| (index % 251) as u8)
            .collect::<Vec<_>>();
        for index in 0..10 {
            let mut bytes = base.clone();
            bytes[4096 + index] ^= u8::try_from(index + 1).unwrap();
            let chunk_id = sha256_exact(&bytes);
            chunks.insert(
                chunk_id,
                Chunk {
                    chunk_id,
                    logical_len: bytes.len() as u64,
                    plan_ref: UNPLANNED_PLAN_ID,
                    group_ref: None,
                    plaintext: bytes.into_boxed_slice(),
                },
            );
        }
        let first = cluster(&chunks, BALANCED_V3_SIMILARITY);
        let second = cluster(&chunks, BALANCED_V3_SIMILARITY);
        assert_eq!(first, second);
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].chunks.len(), 10);
    }

    #[test]
    fn unrelated_noise_does_not_become_a_similarity_cohort() {
        let mut chunks = BTreeMap::new();
        for seed in 1..12_u64 {
            let mut state = seed;
            let bytes = (0..32 * 1024)
                .map(|_| {
                    state ^= state << 13;
                    state ^= state >> 7;
                    state ^= state << 17;
                    state as u8
                })
                .collect::<Vec<_>>();
            let chunk_id = sha256_exact(&bytes);
            chunks.insert(
                chunk_id,
                Chunk {
                    chunk_id,
                    logical_len: bytes.len() as u64,
                    plan_ref: UNPLANNED_PLAN_ID,
                    group_ref: None,
                    plaintext: bytes.into_boxed_slice(),
                },
            );
        }
        assert!(cluster(&chunks, BALANCED_V3_SIMILARITY).is_empty());
    }

    #[test]
    fn structured_text_is_clustered_from_plaintext_not_names() {
        let mut chunks = BTreeMap::new();
        for index in 0..12 {
            let header = format!("record={index:04}\nkind=entrybound-test\n");
            let rows = (0..2_000)
                .map(|row| format!("row={row:04},field=shared-value,status=active\n"))
                .collect::<String>();
            let bytes = format!("{header}{rows}tail={index:04}\n").into_bytes();
            let chunk_id = sha256_exact(&bytes);
            chunks.insert(
                chunk_id,
                Chunk {
                    chunk_id,
                    logical_len: bytes.len() as u64,
                    plan_ref: UNPLANNED_PLAN_ID,
                    group_ref: None,
                    plaintext: bytes.into_boxed_slice(),
                },
            );
        }
        let cohorts = cluster(&chunks, BALANCED_V3_SIMILARITY);
        assert_eq!(cohorts.len(), 1);
        assert_eq!(cohorts[0].chunks.len(), 12);
    }
}
