//! Research-only alternative content-defined chunking (CDC) algorithms,
//! each compared against production `gear-norm-v1`
//! (`entrybound::chunker`, reached here through
//! [`crate::production_chunk_ranges`]) on the same input. Every algorithm is
//! explicitly parameterized and carries a stable, self-describing algorithm
//! id (mirroring production's `ChunkingParameters::chunker_id` convention,
//! e.g. `"gear-norm-v1/min-131072/target-524288/max-2097152"`). None of them
//! are wired into any production code path.
//!
//! | id prefix | Paper |
//! |---|---|
//! | `fixed-size-v1` | none -- a byte-count baseline with no content dependence, see [`fixed_size`] |
//! | `fastcdc-2016` | Xia, Jiang, Feng, Douglis, Shilane, Hua, Fu, Zhou & Zhou, "FastCDC: a Fast and Efficient Content-Defined Chunking Approach for Data Deduplication", USENIX ATC 2016 -- see [`fastcdc2016`] |
//! | `fastcdc-2020` | Xia et al., "FastCDC 2.0: A Simple, Efficient, and Scalable Content-Defined Chunking Approach", IEEE TPDS 2020 -- see [`fastcdc2020`] |
//! | `rabin-cdc-v1` | Rabin, "Fingerprinting by Random Polynomials" (1981); Broder, "Some Applications of Rabin's Fingerprinting Method" (1993); CDC use per Muthitacharoen, Chen & Mazieres, "A Low-bandwidth Network File System" (LBFS), SOSP 2001 -- see [`rabin`] |
//! | `buzhash-cdc-v1` | Broder (1993)'s cyclic-polynomial rolling hash, as used by rsync-adjacent/backup CDC tools (e.g. borg/attic) -- see [`buzhash`] |
//! | `ae-v1` | Zhang, Jiang, Feng, Xia, Fu & Zhou, "AE: An Asymmetric Extremum Content Defined Chunking Algorithm for Fast and Bandwidth-Efficient Data Deduplication", IEEE INFOCOM 2015 -- see [`ae`] |
//! | `ram-v1` | Zhang, Feng, Hua, Xia, Zhou, Zhou & Wang, "RAM: A Fast and Space-Efficient Content-Defined Chunking Algorithm via Rapid Asymmetric Maximum Sampling", 2021 -- see [`ram`] for exactly what boundary rule is (and is not) reproduced |
//! | `tttd-v1` | Eshghi & Tang, "A Framework for Analyzing and Improving Content-Based Chunking Algorithms", HP Labs technical report HPL-2005-30 (2005) -- see [`tttd`] |
//!
//! [`fastcdc2016`] and [`fastcdc2020`] reuse the exact published Gear table
//! and normalized-chunking mask table (see their module docs for
//! provenance); each has a test that cross-checks its boundaries against the
//! `fastcdc` crate (pinned exactly as a dev-dependency, `research/harness`
//! convention for third-party pins) as an independent reference
//! implementation.

pub mod ae;
pub mod buzhash;
pub mod fastcdc2016;
pub mod fastcdc2020;
pub mod fixed_size;
pub mod rabin;
pub mod ram;
pub mod tttd;

use entrybound::chunker::ChunkingParameters;

/// A complete, non-overlapping plaintext range. Mirrors
/// `entrybound::chunker::ChunkRange`'s shape without depending on it, since
/// every algorithm in this module is research-only and returns its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ChunkRange {
    pub start: usize,
    pub end: usize,
}

impl ChunkRange {
    #[must_use]
    pub const fn len(self) -> usize {
        self.end - self.start
    }

    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.start == self.end
    }
}

/// The next-lower power of two, minus one: a cut mask giving an average
/// chunk size at that power of two under a uniform-hash assumption. Shared
/// by every single-mask rolling-hash algorithm in this module ([`rabin`],
/// [`buzhash`], [`tttd`]); [`fastcdc2016`]/[`fastcdc2020`] instead use the
/// published two-level normalized mask table, and [`ae`]/[`ram`] use no mask
/// at all (their cut rule is a plain extremum comparison).
#[must_use]
pub(crate) fn pow2_mask_for_target(target_size: usize) -> u64 {
    let clamped = target_size.max(2);
    let bits = usize::BITS - clamped.leading_zeros() - 1;
    (1u64 << bits) - 1
}

/// Rounded base-2 logarithm, matching `fastcdc` crate's own `logarithm2`
/// (round-to-nearest, not floor) so [`fastcdc2016`]/[`fastcdc2020`] pick the
/// same mask-table bucket the reference implementation does for the same
/// `avg_size`.
pub(crate) fn round_log2(value: usize) -> u32 {
    (value as f64).log2().round() as u32
}

/// One research algorithm, fully parameterized, plus production
/// `gear-norm-v1` -- a uniform dispatch target every subcommand
/// (`boundaries`, `dedup`, `shift-stability`, `random-access`) shares.
#[derive(Debug, Clone)]
pub enum Algorithm {
    GearNormV1(ChunkingParameters),
    FixedSize(fixed_size::Params),
    FastCdc2016(fastcdc2016::Params),
    FastCdc2020(fastcdc2020::Params),
    Rabin(rabin::Params),
    Buzhash(buzhash::Params),
    Ae(ae::Params),
    Ram(ram::Params),
    Tttd(tttd::Params),
}

impl Algorithm {
    /// Every algorithm name this crate accepts from `--algorithm`.
    pub const NAMES: &'static [&'static str] = &[
        "gear-norm-v1",
        "fixed-size",
        "fastcdc-2016",
        "fastcdc-2020",
        "rabin",
        "buzhash",
        "ae",
        "ram",
        "tttd",
    ];

    /// Builds an algorithm from a `--algorithm` name and a production preset
    /// (`--profile`'s `ChunkingParameters`), applying each algorithm's own
    /// default extra parameter (Rabin's 48-byte window, Buzhash's 64-byte
    /// window, FastCDC's normalization level 2) unless `window` /
    /// `normalization` override it. Every algorithm is sized to the *same*
    /// min/target/max as the chosen production preset, so runs across
    /// algorithms are comparable at matching size targets.
    pub fn from_profile(
        name: &str,
        profile: ChunkingParameters,
        window: Option<usize>,
        normalization: Option<u8>,
    ) -> Result<Self, String> {
        match name {
            "gear-norm-v1" => Ok(Algorithm::GearNormV1(profile)),
            "fixed-size" => Ok(Algorithm::FixedSize(fixed_size::Params {
                chunk_size: profile.target_size,
            })),
            "fastcdc-2016" => Ok(Algorithm::FastCdc2016(fastcdc2016::Params::from_profile(
                profile,
                normalization,
            )?)),
            "fastcdc-2020" => Ok(Algorithm::FastCdc2020(fastcdc2020::Params::from_profile(
                profile,
                normalization,
            )?)),
            "rabin" => Ok(Algorithm::Rabin(rabin::Params::from_profile(
                profile,
                window.unwrap_or(rabin::DEFAULT_WINDOW),
            ))),
            "buzhash" => Ok(Algorithm::Buzhash(buzhash::Params::from_profile(
                profile,
                window.unwrap_or(buzhash::DEFAULT_WINDOW),
            ))),
            "ae" => Ok(Algorithm::Ae(ae::Params::from_profile(profile, window))),
            "ram" => Ok(Algorithm::Ram(ram::Params::from_profile(profile, window))),
            "tttd" => Ok(Algorithm::Tttd(tttd::Params::from_profile(profile))),
            other => Err(format!(
                "unknown --algorithm {other:?} (expected one of: {})",
                Algorithm::NAMES.join(", ")
            )),
        }
    }

    #[must_use]
    pub fn algorithm_id(&self) -> String {
        match self {
            Algorithm::GearNormV1(p) => p.chunker_id.to_string(),
            Algorithm::FixedSize(p) => p.algorithm_id(),
            Algorithm::FastCdc2016(p) => p.algorithm_id(),
            Algorithm::FastCdc2020(p) => p.algorithm_id(),
            Algorithm::Rabin(p) => p.algorithm_id(),
            Algorithm::Buzhash(p) => p.algorithm_id(),
            Algorithm::Ae(p) => p.algorithm_id(),
            Algorithm::Ram(p) => p.algorithm_id(),
            Algorithm::Tttd(p) => p.algorithm_id(),
        }
    }

    /// Runs this algorithm over `data`, returning complete, non-overlapping,
    /// gap-free ranges covering it (empty input has no ranges). Every
    /// research algorithm enforces this itself (see each module's tests);
    /// `gear-norm-v1` additionally goes through production's own validation
    /// (`entrybound::chunker::chunk_ranges`), surfaced here as `Err`.
    pub fn chunk_ranges(&self, data: &[u8]) -> Result<Vec<ChunkRange>, String> {
        match self {
            Algorithm::GearNormV1(p) => crate::production_chunk_ranges(data, *p)
                .map(|ranges| {
                    ranges
                        .iter()
                        .map(|r| ChunkRange {
                            start: r.start,
                            end: r.end,
                        })
                        .collect()
                })
                .map_err(|d| d.to_string()),
            Algorithm::FixedSize(p) => Ok(fixed_size::chunk_ranges(data, p)),
            Algorithm::FastCdc2016(p) => Ok(fastcdc2016::chunk_ranges(data, p)),
            Algorithm::FastCdc2020(p) => Ok(fastcdc2020::chunk_ranges(data, p)),
            Algorithm::Rabin(p) => Ok(rabin::chunk_ranges(data, p)),
            Algorithm::Buzhash(p) => Ok(buzhash::chunk_ranges(data, p)),
            Algorithm::Ae(p) => Ok(ae::chunk_ranges(data, p)),
            Algorithm::Ram(p) => Ok(ram::chunk_ranges(data, p)),
            Algorithm::Tttd(p) => Ok(tttd::chunk_ranges(data, p)),
        }
    }
}

/// Asserts the universal CDC contract every algorithm in this module must
/// satisfy: ranges are non-overlapping, gap-free, cover the complete input,
/// and (other than possibly the last) fall within `[min_size, max_size]`.
/// Shared by every module's tests so the contract is checked identically
/// everywhere, rather than each module re-deriving its own assertions.
#[cfg(test)]
pub(crate) fn assert_valid_ranges(
    data_len: usize,
    ranges: &[ChunkRange],
    min_size: usize,
    max_size: usize,
) {
    if data_len == 0 {
        assert!(ranges.is_empty(), "empty input must have no ranges");
        return;
    }
    assert!(
        !ranges.is_empty(),
        "non-empty input must have at least one range"
    );
    assert_eq!(ranges[0].start, 0, "first range must start at 0");
    assert_eq!(
        ranges.last().unwrap().end,
        data_len,
        "last range must end at the input length"
    );
    for window in ranges.windows(2) {
        assert_eq!(
            window[0].end, window[1].start,
            "ranges must be contiguous with no gap or overlap"
        );
    }
    for (index, range) in ranges.iter().enumerate() {
        assert!(!range.is_empty(), "range must not be empty: {range:?}");
        assert!(
            range.len() <= max_size,
            "range {range:?} exceeds max_size {max_size}"
        );
        if index + 1 != ranges.len() {
            assert!(
                range.len() >= min_size,
                "non-final range {range:?} is below min_size {min_size}"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pow2_mask_matches_expected_bit_widths() {
        assert_eq!(pow2_mask_for_target(2), 0x1);
        assert_eq!(pow2_mask_for_target(1024), 0x3ff);
        assert_eq!(pow2_mask_for_target(1000), 0x1ff); // next lower power of two: 512
    }

    #[test]
    fn round_log2_matches_fastcdc_crate_convention() {
        assert_eq!(round_log2(131_072), 17); // exact power of two
        assert_eq!(round_log2(1024), 10);
    }

    #[test]
    fn from_profile_rejects_an_unknown_algorithm_name() {
        let profile = entrybound::chunker::BALANCED_V2;
        assert!(Algorithm::from_profile("not-a-real-algorithm", profile, None, None).is_err());
    }

    #[test]
    fn every_named_algorithm_builds_from_every_production_profile() {
        use entrybound::chunker::{BALANCED_V2, DENSE_V2, EXTREME_V2, FAST_V2};
        for profile in [FAST_V2, BALANCED_V2, DENSE_V2, EXTREME_V2] {
            for name in Algorithm::NAMES {
                Algorithm::from_profile(name, profile, None, None)
                    .unwrap_or_else(|e| panic!("{name} should build from {profile:?}: {e}"));
            }
        }
    }
}
