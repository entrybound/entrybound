//! Deterministic creation-time compression planning.
//!
//! Profiles exist only here. The native reader uses recorded TransformPlans
//! and operational codecs without consulting this module.

use std::collections::BTreeMap;
use std::str::FromStr;

use crate::codec::{aggregate_decode_requirements, encode_payload, store_plan, zstd_plan};
use crate::diagnostics::{Diagnostic, OutcomeClass, ReasonCode, Result};
use crate::eam::{Archive, TransformPlan};
use crate::identity::sha256_exact;

/// Temporary plan marker used only between plaintext chunking and planning.
pub const UNPLANNED_PLAN_ID: u64 = 0;
/// Per-chunk cost assigned to selecting a non-STORE plan.
pub const ZSTD_ATTRIBUTABLE_OVERHEAD_BYTES: u64 = 16;
/// Absolute minimum physical gain required before Zstandard may win.
pub const MINIMUM_GAIN_BYTES: u64 = 32;
/// Relative minimum gain: one percent, expressed in basis points.
pub const MINIMUM_GAIN_BASIS_POINTS: u64 = 100;
const MINIMUM_ZSTD_INPUT_BYTES: usize = 64;
const PROBE_BYTES: usize = 4096;

const FAST_LEVELS: [i32; 1] = [1];
const BALANCED_LEVELS: [i32; 3] = [1, 3, 5];
const BALANCED_FALLBACK_LEVELS: [i32; 1] = [1];
const DENSE_LEVELS: [i32; 3] = [5, 9, 15];
const EXTREME_LEVELS: [i32; 4] = [9, 15, 19, 22];

/// Public creation profiles. Their planner IDs freeze the v1 behavior.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CompressionProfile {
    Fast,
    #[default]
    Balanced,
    Dense,
    Extreme,
}

impl CompressionProfile {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Fast => "fast",
            Self::Balanced => "balanced",
            Self::Dense => "dense",
            Self::Extreme => "extreme",
        }
    }

    #[must_use]
    pub const fn planner_id(self) -> &'static str {
        match self {
            Self::Fast => "fast-v1",
            Self::Balanced => "balanced-v1",
            Self::Dense => "dense-v1",
            Self::Extreme => "extreme-v1",
        }
    }

    fn levels(self, analysis: ContentAnalysis) -> &'static [i32] {
        match self {
            Self::Fast if analysis.likely_compressible => &FAST_LEVELS,
            Self::Fast => &[],
            Self::Balanced if analysis.likely_compressible => &BALANCED_LEVELS,
            Self::Balanced => &BALANCED_FALLBACK_LEVELS,
            Self::Dense => &DENSE_LEVELS,
            Self::Extreme => &EXTREME_LEVELS,
        }
    }
}

impl FromStr for CompressionProfile {
    type Err = Diagnostic;

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "fast" => Ok(Self::Fast),
            "balanced" => Ok(Self::Balanced),
            "dense" => Ok(Self::Dense),
            "extreme" => Ok(Self::Extreme),
            _ => Err(Diagnostic::new(
                OutcomeClass::PolicyRefused,
                ReasonCode::CommandUsage,
                format!(
                    "unknown compression profile '{value}'; expected fast, balanced, dense, or extreme"
                ),
            )),
        }
    }
}

/// Integer-only deterministic probe used to bound profile search effort.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ContentAnalysis {
    pub plaintext_len: u64,
    pub empty: bool,
    pub sample_len: u64,
    pub distinct_symbols: u16,
    pub maximum_symbol_frequency_ppm: u32,
    pub adjacent_repeat_ppm: u32,
    pub likely_compressible: bool,
}

/// Aggregate result of applying one planner profile to an Archive.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlanningReport {
    pub planner_id: String,
    pub store_chunks: u64,
    pub zstandard_chunks: u64,
    pub selected_plans: Box<[TransformPlan]>,
}

/// Selects a recorded plan independently for every unique plaintext Chunk.
pub fn plan_archive(archive: &mut Archive, profile: CompressionProfile) -> Result<PlanningReport> {
    let mut plans = BTreeMap::new();
    let store = store_plan();
    plans.insert(store.plan_id, store);
    let mut store_chunks = 0_u64;
    let mut zstandard_chunks = 0_u64;

    for chunk in archive.content_store.chunks.values_mut() {
        if chunk.logical_len != u64::try_from(chunk.plaintext.len()).unwrap_or(u64::MAX)
            || sha256_exact(&chunk.plaintext) != chunk.chunk_id
        {
            return Err(Diagnostic::new(
                OutcomeClass::Corrupt,
                ReasonCode::ChunkDigestMismatch,
                chunk.chunk_id.to_string(),
            ));
        }
        let analysis = analyze(&chunk.plaintext);
        let mut selected: Option<(TransformPlan, usize)> = None;
        if chunk.plaintext.len() >= MINIMUM_ZSTD_INPUT_BYTES {
            for level in profile.levels(analysis) {
                let plan = zstd_plan(*level)?;
                let encoded = encode_payload(&plan, &chunk.plaintext)?;
                if zstandard_wins(chunk.logical_len, encoded.len())?
                    && selected
                        .as_ref()
                        .is_none_or(|(_, previous_len)| encoded.len() < *previous_len)
                {
                    selected = Some((plan, encoded.len()));
                }
            }
        }
        if let Some((plan, _)) = selected {
            chunk.plan_ref = plan.plan_id;
            plans.entry(plan.plan_id).or_insert(plan);
            zstandard_chunks = zstandard_chunks
                .checked_add(1)
                .ok_or_else(|| resource("Zstandard Chunk count exceeds u64"))?;
        } else {
            chunk.plan_ref = crate::identity::STORE_PLAN_ID;
            store_chunks = store_chunks
                .checked_add(1)
                .ok_or_else(|| resource("STORE Chunk count exceeds u64"))?;
        }
    }

    let selected_plans = plans.into_values().collect::<Vec<_>>().into_boxed_slice();
    archive.transform_plans = selected_plans.clone();
    archive.descriptor.planner_id = profile.planner_id().to_owned();
    archive.descriptor.decode = aggregate_decode_requirements(&archive.transform_plans);
    Ok(PlanningReport {
        planner_id: profile.planner_id().to_owned(),
        store_chunks,
        zstandard_chunks,
        selected_plans,
    })
}

/// Computes the planner's deterministic, integer-only compressibility probe.
#[must_use]
pub fn analyze(plaintext: &[u8]) -> ContentAnalysis {
    if plaintext.is_empty() {
        return ContentAnalysis {
            plaintext_len: 0,
            empty: true,
            sample_len: 0,
            distinct_symbols: 0,
            maximum_symbol_frequency_ppm: 0,
            adjacent_repeat_ppm: 0,
            likely_compressible: false,
        };
    }
    let sample_len = plaintext.len().min(PROBE_BYTES);
    let mut histogram = [0_u32; 256];
    let mut adjacent_repeats = 0_u64;
    let mut previous = None;
    for sample_index in 0..sample_len {
        let index = sample_index
            .checked_mul(plaintext.len())
            .map_or(plaintext.len() - 1, |value| value / sample_len);
        let byte = plaintext[index];
        histogram[usize::from(byte)] += 1;
        if previous == Some(byte) {
            adjacent_repeats += 1;
        }
        previous = Some(byte);
    }
    let distinct_symbols = histogram.iter().filter(|count| **count != 0).count() as u16;
    let maximum = u64::from(*histogram.iter().max().unwrap_or(&0));
    let sample_u64 = sample_len as u64;
    let maximum_symbol_frequency_ppm = ((maximum * 1_000_000) / sample_u64) as u32;
    let adjacent_repeat_ppm = if sample_len <= 1 {
        0
    } else {
        ((adjacent_repeats * 1_000_000) / (sample_u64 - 1)) as u32
    };
    let likely_compressible = distinct_symbols <= 192
        || maximum_symbol_frequency_ppm >= 12_000
        || adjacent_repeat_ppm >= 20_000;
    ContentAnalysis {
        plaintext_len: plaintext.len() as u64,
        empty: false,
        sample_len: sample_u64,
        distinct_symbols,
        maximum_symbol_frequency_ppm,
        adjacent_repeat_ppm,
        likely_compressible,
    }
}

/// Returns true only when measured stored cost clears both gain thresholds.
pub fn zstandard_wins(logical_len: u64, encoded_len: usize) -> Result<bool> {
    let relative = logical_len
        .checked_mul(MINIMUM_GAIN_BASIS_POINTS)
        .and_then(|value| value.checked_add(9_999))
        .map(|value| value / 10_000)
        .ok_or_else(|| resource("minimum-gain calculation overflow"))?;
    let minimum_gain = MINIMUM_GAIN_BYTES.max(relative);
    let encoded_cost = u64::try_from(encoded_len)
        .map_err(|_| resource("encoded candidate length exceeds u64"))?
        .checked_add(ZSTD_ATTRIBUTABLE_OVERHEAD_BYTES)
        .and_then(|value| value.checked_add(minimum_gain))
        .ok_or_else(|| resource("encoded candidate cost overflow"))?;
    Ok(encoded_cost < logical_len)
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
    fn profiles_have_frozen_v1_identifiers_and_balanced_is_default() {
        assert_eq!(CompressionProfile::default(), CompressionProfile::Balanced);
        assert_eq!(CompressionProfile::Fast.planner_id(), "fast-v1");
        assert_eq!(CompressionProfile::Balanced.planner_id(), "balanced-v1");
        assert_eq!(CompressionProfile::Dense.planner_id(), "dense-v1");
        assert_eq!(CompressionProfile::Extreme.planner_id(), "extreme-v1");
    }

    #[test]
    fn probe_distinguishes_repetition_from_deterministic_noise() {
        let empty = analyze(&[]);
        let repetitive = analyze(&vec![7; 8192]);
        let noise = analyze(&deterministic_noise(8192));
        assert!(empty.empty);
        assert!(!empty.likely_compressible);
        assert!(repetitive.likely_compressible);
        assert!(!noise.likely_compressible);
    }

    #[test]
    fn minimum_gain_includes_overhead_and_prefers_store_on_ties() {
        assert!(!zstandard_wins(100, 52).unwrap());
        assert!(zstandard_wins(100, 51).unwrap());
        assert!(!zstandard_wins(49, 1).unwrap());
    }

    fn deterministic_noise(len: usize) -> Vec<u8> {
        let mut state = 0x4d59_5df4_d0f3_3173_u64;
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
