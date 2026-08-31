//! Deterministic creation-time chunking.
//!
//! Decoders consume recorded Chunk references and never invoke this module.
//! `gear-norm-v1` scans every byte with a fixed Gear table, uses a stronger
//! pre-target mask and weaker post-target mask, and forces the maximum size.

use std::collections::{BTreeMap, BTreeSet};

use aes::{
    Aes128, Block,
    cipher::{BlockCipherEncrypt, KeyInit},
};

use crate::diagnostics::{Diagnostic, OutcomeClass, ReasonCode, Result};
use crate::eam::Digest;
use crate::identity::sha256_exact;

/// Physical cost of one CHUNK_DATA frame header.
pub const CHUNK_FRAME_OVERHEAD_BYTES: u64 = 64;
/// Canonical encoded size of one bootstrap Index-entry record.
pub const INDEX_ENTRY_OVERHEAD_BYTES: u64 = 100;
/// Length prefix plus SHA-256 digest in a ContentObject Chunk-reference list.
pub const CHUNK_REFERENCE_OVERHEAD_BYTES: u64 = 40;

const GEAR_SEED: u64 = 0x6a09_e667_f3bc_c909;
const GEAR_TABLE: [u64; 256] = gear_table();
const PHTE_MODULUS: u128 = (1_u128 << 127) - 1;
const PHTE_WINDOW: usize = 64;

/// File-key-derived boundary material for encrypted archives.
///
/// This is creation-time state only. It is never serialized and never needed
/// by a decoder, which consumes the resulting canonical Chunk references.
#[derive(Clone, Eq, PartialEq)]
pub enum EncryptedBoundaryKey {
    SecretGearTable(Box<[u64; 256]>),
    PhteAes128 { polynomial: u128, aes_key: [u8; 16] },
}

/// Frozen parameters for one `gear-norm-v1` policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChunkingParameters {
    pub chunker_id: &'static str,
    pub minimum_size: usize,
    pub target_size: usize,
    pub maximum_size: usize,
}

/// A complete, non-overlapping plaintext range.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChunkRange {
    pub start: usize,
    pub end: usize,
}

impl ChunkRange {
    #[must_use]
    pub const fn len(self) -> usize {
        self.end.saturating_sub(self.start)
    }

    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.start == self.end
    }
}

/// Deterministic physical-cost estimate for one archive-wide assignment.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChunkingEvaluation {
    pub parameters: ChunkingParameters,
    pub estimated_cost_bytes: u64,
    pub unique_chunk_count: u64,
    pub manifest_chunk_references: u64,
    pub unique_plaintext_bytes: u64,
}

/// The coarsest v2 policy: fewer frames and less Index overhead.
pub const FAST_V2: ChunkingParameters = ChunkingParameters {
    chunker_id: "gear-norm-v1/min-524288/target-2097152/max-8388608",
    minimum_size: 512 * 1024,
    target_size: 2 * 1024 * 1024,
    maximum_size: 8 * 1024 * 1024,
};

/// Default general-purpose v2 policy.
pub const BALANCED_V2: ChunkingParameters = ChunkingParameters {
    chunker_id: "gear-norm-v1/min-131072/target-524288/max-2097152",
    minimum_size: 128 * 1024,
    target_size: 512 * 1024,
    maximum_size: 2 * 1024 * 1024,
};

/// Finer candidate available to dense and extreme planning.
pub const DENSE_V2: ChunkingParameters = ChunkingParameters {
    chunker_id: "gear-norm-v1/min-65536/target-262144/max-1048576",
    minimum_size: 64 * 1024,
    target_size: 256 * 1024,
    maximum_size: 1024 * 1024,
};

/// Finest bounded candidate considered only by extreme planning.
pub const EXTREME_V2: ChunkingParameters = ChunkingParameters {
    chunker_id: "gear-norm-v1/min-32768/target-131072/max-524288",
    minimum_size: 32 * 1024,
    target_size: 128 * 1024,
    maximum_size: 512 * 1024,
};

/// Finds canonical CDC ranges. Empty plaintext has no physical Chunks.
pub fn chunk_ranges(plaintext: &[u8], parameters: ChunkingParameters) -> Result<Box<[ChunkRange]>> {
    chunk_ranges_with_table(plaintext, parameters, &GEAR_TABLE)
}

/// Finds encrypted CDC ranges using the AFK-derived boundary construction.
pub fn chunk_ranges_encrypted(
    plaintext: &[u8],
    parameters: ChunkingParameters,
    key: &EncryptedBoundaryKey,
) -> Result<Box<[ChunkRange]>> {
    match key {
        EncryptedBoundaryKey::SecretGearTable(table) => {
            chunk_ranges_with_table(plaintext, parameters, table)
        }
        EncryptedBoundaryKey::PhteAes128 {
            polynomial,
            aes_key,
        } => chunk_ranges_phte(plaintext, parameters, *polynomial, aes_key),
    }
}

fn chunk_ranges_with_table(
    plaintext: &[u8],
    parameters: ChunkingParameters,
    table: &[u64; 256],
) -> Result<Box<[ChunkRange]>> {
    validate_parameters(parameters)?;
    if plaintext.is_empty() {
        return Ok(Box::default());
    }

    let target_bits = parameters.target_size.trailing_zeros();
    let early_mask = mask(target_bits + 1)?;
    let late_mask = mask(target_bits - 1)?;
    let mut ranges = Vec::new();
    let mut start = 0_usize;
    while start < plaintext.len() {
        let limit = start
            .checked_add(parameters.maximum_size)
            .map_or(plaintext.len(), |value| value.min(plaintext.len()));
        let mut hash = 0_u64;
        let mut end = limit;
        for (relative, byte) in plaintext[start..limit].iter().enumerate() {
            hash = hash.wrapping_shl(1).wrapping_add(table[usize::from(*byte)]);
            let length = relative + 1;
            if length < parameters.minimum_size {
                continue;
            }
            let boundary_mask = if length < parameters.target_size {
                early_mask
            } else {
                late_mask
            };
            if hash & boundary_mask == 0 || length == parameters.maximum_size {
                end = start + length;
                break;
            }
        }
        ranges.push(ChunkRange { start, end });
        start = end;
    }
    validate_ranges(plaintext.len(), &ranges, parameters)?;
    Ok(ranges.into_boxed_slice())
}

fn chunk_ranges_phte(
    plaintext: &[u8],
    parameters: ChunkingParameters,
    polynomial: u128,
    aes_key: &[u8; 16],
) -> Result<Box<[ChunkRange]>> {
    validate_parameters(parameters)?;
    if polynomial >= PHTE_MODULUS {
        return Err(resource("PHTE polynomial is not a canonical field element"));
    }
    if plaintext.is_empty() {
        return Ok(Box::default());
    }
    let cipher = Aes128::new(aes_key.into());
    let target_bits = parameters.target_size.trailing_zeros();
    let early_mask = mask128(target_bits + 1)?;
    let late_mask = mask128(target_bits - 1)?;
    let outgoing_factor = field_pow(polynomial, (PHTE_WINDOW - 1) as u32);
    let mut state = 0_u128;
    let mut window = [0_u8; PHTE_WINDOW];
    let mut seen = 0_usize;
    let mut ranges = Vec::new();
    let mut start = 0_usize;

    for (index, byte) in plaintext.iter().copied().enumerate() {
        if seen < PHTE_WINDOW {
            state = field_add(field_mul(state, polynomial), u128::from(byte));
            window[seen] = byte;
            seen += 1;
        } else {
            let slot = index % PHTE_WINDOW;
            let outgoing = field_mul(u128::from(window[slot]), outgoing_factor);
            state = field_add(
                field_mul(field_sub(state, outgoing), polynomial),
                u128::from(byte),
            );
            window[slot] = byte;
        }

        let length = index + 1 - start;
        if length < parameters.minimum_size {
            continue;
        }
        let mut block = Block::from(state.to_be_bytes());
        cipher.encrypt_block(&mut block);
        let decision = u128::from_be_bytes(block.into());
        let boundary_mask = if length < parameters.target_size {
            early_mask
        } else {
            late_mask
        };
        if decision & boundary_mask == 0 || length == parameters.maximum_size {
            ranges.push(ChunkRange {
                start,
                end: index + 1,
            });
            start = index + 1;
        }
    }
    if start < plaintext.len() {
        ranges.push(ChunkRange {
            start,
            end: plaintext.len(),
        });
    }
    validate_ranges(plaintext.len(), &ranges, parameters)?;
    Ok(ranges.into_boxed_slice())
}

/// Chooses the lowest complete physical-cost estimate; candidate order wins ties.
pub fn select_parameters(
    contents: &[&[u8]],
    candidates: &[ChunkingParameters],
) -> Result<ChunkingEvaluation> {
    let mut selected = None;
    for parameters in candidates {
        let evaluation = evaluate(contents, *parameters)?;
        if selected
            .as_ref()
            .is_none_or(|current: &ChunkingEvaluation| {
                evaluation.estimated_cost_bytes < current.estimated_cost_bytes
            })
        {
            selected = Some(evaluation);
        }
    }
    selected.ok_or_else(|| resource("chunking policy must contain at least one candidate"))
}

/// Selects encrypted CDC parameters using the complete framing/dedup cost.
pub fn select_parameters_encrypted(
    contents: &[&[u8]],
    candidates: &[ChunkingParameters],
    key: &EncryptedBoundaryKey,
) -> Result<ChunkingEvaluation> {
    let mut selected = None;
    for parameters in candidates {
        let evaluation = evaluate_encrypted(contents, *parameters, key)?;
        if selected
            .as_ref()
            .is_none_or(|current: &ChunkingEvaluation| {
                evaluation.estimated_cost_bytes < current.estimated_cost_bytes
            })
        {
            selected = Some(evaluation);
        }
    }
    selected.ok_or_else(|| resource("chunking policy must contain at least one candidate"))
}

/// Evaluates exact archive-wide dedup plus canonical framing/reference overhead.
pub fn evaluate(contents: &[&[u8]], parameters: ChunkingParameters) -> Result<ChunkingEvaluation> {
    evaluate_with(contents, parameters, |bytes, parameters| {
        chunk_ranges(bytes, parameters)
    })
}

fn evaluate_encrypted(
    contents: &[&[u8]],
    parameters: ChunkingParameters,
    key: &EncryptedBoundaryKey,
) -> Result<ChunkingEvaluation> {
    evaluate_with(contents, parameters, |bytes, parameters| {
        chunk_ranges_encrypted(bytes, parameters, key)
    })
}

fn evaluate_with<F>(
    contents: &[&[u8]],
    parameters: ChunkingParameters,
    mut ranges_for: F,
) -> Result<ChunkingEvaluation>
where
    F: FnMut(&[u8], ChunkingParameters) -> Result<Box<[ChunkRange]>>,
{
    let mut unique_objects = BTreeSet::<Digest>::new();
    let mut unique_chunks = BTreeMap::<Digest, u64>::new();
    let mut manifest_chunk_references = 0_u64;
    for plaintext in contents {
        let object_id = sha256_exact(plaintext);
        if !unique_objects.insert(object_id) {
            continue;
        }
        let ranges = ranges_for(plaintext, parameters)?;
        manifest_chunk_references = manifest_chunk_references
            .checked_add(
                u64::try_from(ranges.len()).map_err(|_| resource("Chunk refs exceed u64"))?,
            )
            .ok_or_else(|| resource("manifest Chunk-reference count exceeds u64"))?;
        for range in ranges {
            let bytes = &plaintext[range.start..range.end];
            let length = u64::try_from(bytes.len())
                .map_err(|_| resource("plaintext Chunk length exceeds u64"))?;
            unique_chunks.entry(sha256_exact(bytes)).or_insert(length);
        }
    }
    let unique_plaintext_bytes = unique_chunks.values().try_fold(0_u64, |total, length| {
        total
            .checked_add(*length)
            .ok_or_else(|| resource("unique plaintext byte count exceeds u64"))
    })?;
    let unique_chunk_count =
        u64::try_from(unique_chunks.len()).map_err(|_| resource("Chunk count exceeds u64"))?;
    let unique_overhead = CHUNK_FRAME_OVERHEAD_BYTES
        .checked_add(INDEX_ENTRY_OVERHEAD_BYTES)
        .and_then(|value| value.checked_mul(unique_chunk_count))
        .ok_or_else(|| resource("unique Chunk overhead exceeds u64"))?;
    let reference_overhead = CHUNK_REFERENCE_OVERHEAD_BYTES
        .checked_mul(manifest_chunk_references)
        .ok_or_else(|| resource("Chunk-reference overhead exceeds u64"))?;
    let estimated_cost_bytes = unique_plaintext_bytes
        .checked_add(unique_overhead)
        .and_then(|value| value.checked_add(reference_overhead))
        .ok_or_else(|| resource("chunking cost exceeds u64"))?;
    Ok(ChunkingEvaluation {
        parameters,
        estimated_cost_bytes,
        unique_chunk_count,
        manifest_chunk_references,
        unique_plaintext_bytes,
    })
}

fn validate_parameters(parameters: ChunkingParameters) -> Result<()> {
    if parameters.minimum_size == 0
        || parameters.minimum_size > parameters.target_size
        || parameters.target_size > parameters.maximum_size
        || !parameters.target_size.is_power_of_two()
        || parameters.target_size < 2
    {
        return Err(Diagnostic::new(
            OutcomeClass::Nonconforming,
            ReasonCode::ResourceLimit,
            format!("invalid CDC parameters for {}", parameters.chunker_id),
        ));
    }
    Ok(())
}

fn validate_ranges(
    plaintext_len: usize,
    ranges: &[ChunkRange],
    parameters: ChunkingParameters,
) -> Result<()> {
    let mut expected_start = 0;
    for (index, range) in ranges.iter().enumerate() {
        if range.start != expected_start
            || range.start > range.end
            || range.is_empty()
            || range.end > plaintext_len
            || range.len() > parameters.maximum_size
            || (range.len() < parameters.minimum_size && index + 1 != ranges.len())
        {
            return Err(Diagnostic::new(
                OutcomeClass::Nonconforming,
                ReasonCode::SectionStructure,
                "CDC produced overlapping, gapped, empty, or out-of-bounds ranges",
            ));
        }
        expected_start = range.end;
    }
    if expected_start != plaintext_len || (plaintext_len != 0 && ranges.is_empty()) {
        return Err(Diagnostic::new(
            OutcomeClass::Nonconforming,
            ReasonCode::SectionStructure,
            "CDC ranges do not cover the complete plaintext",
        ));
    }
    Ok(())
}

fn mask(bits: u32) -> Result<u64> {
    1_u64
        .checked_shl(bits)
        .map(|value| value - 1)
        .ok_or_else(|| resource("CDC mask width exceeds u64"))
}

fn mask128(bits: u32) -> Result<u128> {
    1_u128
        .checked_shl(bits)
        .map(|value| value - 1)
        .ok_or_else(|| resource("PHTE mask width exceeds u128"))
}

fn field_add(left: u128, right: u128) -> u128 {
    let sum = left + right;
    if sum >= PHTE_MODULUS {
        sum - PHTE_MODULUS
    } else {
        sum
    }
}

fn field_sub(left: u128, right: u128) -> u128 {
    if left >= right {
        left - right
    } else {
        PHTE_MODULUS - (right - left)
    }
}

fn field_mul(mut left: u128, mut right: u128) -> u128 {
    let mut product = 0_u128;
    while right != 0 {
        if right & 1 != 0 {
            product = field_add(product, left);
        }
        left = field_add(left, left);
        right >>= 1;
    }
    product
}

fn field_pow(mut base: u128, mut exponent: u32) -> u128 {
    let mut result = 1_u128;
    while exponent != 0 {
        if exponent & 1 != 0 {
            result = field_mul(result, base);
        }
        base = field_mul(base, base);
        exponent >>= 1;
    }
    result
}

const fn gear_table() -> [u64; 256] {
    let mut table = [0_u64; 256];
    let mut index = 0_usize;
    while index < table.len() {
        table[index] = splitmix64(GEAR_SEED.wrapping_add(index as u64));
        index += 1;
    }
    table
}

const fn splitmix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9e37_79b9_7f4a_7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

fn resource(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::PolicyRefused,
        ReasonCode::ResourceLimit,
        detail,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_small_and_exact_boundaries_cover_every_byte_once() {
        assert!(chunk_ranges(&[], BALANCED_V2).unwrap().is_empty());
        for length in [
            1,
            BALANCED_V2.minimum_size - 1,
            BALANCED_V2.minimum_size,
            BALANCED_V2.target_size,
            BALANCED_V2.maximum_size,
            BALANCED_V2.maximum_size + 1,
        ] {
            let bytes = deterministic_bytes(length);
            let ranges = chunk_ranges(&bytes, BALANCED_V2).unwrap();
            assert_eq!(ranges.first().unwrap().start, 0);
            assert_eq!(ranges.last().unwrap().end, length);
            for adjacent in ranges.windows(2) {
                assert_eq!(adjacent[0].end, adjacent[1].start);
            }
        }
    }

    #[test]
    fn boundaries_are_deterministic_and_bounded() {
        let bytes = deterministic_bytes(12 * 1024 * 1024);
        let first = chunk_ranges(&bytes, BALANCED_V2).unwrap();
        let second = chunk_ranges(&bytes, BALANCED_V2).unwrap();
        assert_eq!(first, second);
        assert!(first.len() > 2);
        assert!(
            first
                .iter()
                .all(|range| range.len() <= BALANCED_V2.maximum_size)
        );
    }

    #[test]
    fn finer_policy_must_beat_complete_overhead_cost() {
        let data = deterministic_bytes(4 * 1024 * 1024);
        let selected = select_parameters(&[&data], &[BALANCED_V2, DENSE_V2]).unwrap();
        assert_eq!(selected.parameters, BALANCED_V2);
    }

    fn deterministic_bytes(len: usize) -> Vec<u8> {
        let mut state = 0x510e_527f_ade6_82d1_u64;
        (0..len)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                state as u8
            })
            .collect()
    }
}
