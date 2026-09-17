//! Candidate matrices over one input, and a structural-transform
//! false-positive scan. Backs the `ebr-codec` binary's three subcommands
//! (`chunk-matrix`, `object-matrix`, `transform-false-positives`; see
//! `README.md`).
//!
//! [`candidates`] runs every production codec path ([`crate::production`])
//! and every external codec candidate ([`crate::external`]) over one buffer
//! of plaintext, at a fixed, deliberately small set of representative
//! parameters per codec (every level/preset/quality would make a matrix run
//! over a real corpus item impractically slow; the parameter lists below are
//! what a first pass needs, not an exhaustive sweep). Dictionary and prefix
//! modes are not included here: they need a second, related sample, which
//! does not fit a single-input matrix row (see `production.rs`'s own tests
//! for those two instead).

use entrybound::chunker::{self, ChunkingParameters};
use entrybound::diagnostics::Diagnostic;
use entrybound::planner::CompressionProfile;
use serde::Serialize;

use crate::{external, production, transform};

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Family {
    Production,
    External,
}

/// One candidate's outcome over one buffer. `stored_bytes`/`roundtrip_ok`/
/// `encode_seconds`/`decode_seconds` are `None` only when `ineligible` is
/// true (a reconstruction candidate production itself would decline) or
/// `error` is `Some` (the call itself failed -- a validation error, not a
/// codec failure, since every candidate here is only ever invoked with
/// parameters it accepts).
#[derive(Debug, Clone, Serialize)]
pub struct CandidateOutcome {
    pub family: Family,
    pub candidate: String,
    pub input_bytes: u64,
    pub stored_bytes: Option<u64>,
    pub roundtrip_ok: Option<bool>,
    /// Non-decision-grade smoke timing; see `production`'s module docs.
    pub encode_seconds: Option<f64>,
    /// Non-decision-grade smoke timing; see `production`'s module docs.
    pub decode_seconds: Option<f64>,
    pub error: Option<String>,
    /// True for a reconstruction candidate (`deflate-reconstruct`,
    /// `jpeg-jxl-reconstruct`) production itself would not attempt over this
    /// input (not a failure -- an eligibility rule).
    pub ineligible: bool,
}

fn ok_outcome(
    family: Family,
    candidate: String,
    input_bytes: u64,
    stored_bytes: u64,
    roundtrip_ok: bool,
    encode_seconds: f64,
    decode_seconds: f64,
) -> CandidateOutcome {
    CandidateOutcome {
        family,
        candidate,
        input_bytes,
        stored_bytes: Some(stored_bytes),
        roundtrip_ok: Some(roundtrip_ok),
        encode_seconds: Some(encode_seconds),
        decode_seconds: Some(decode_seconds),
        error: None,
        ineligible: false,
    }
}

fn error_outcome(
    family: Family,
    candidate: String,
    input_bytes: u64,
    error: String,
) -> CandidateOutcome {
    CandidateOutcome {
        family,
        candidate,
        input_bytes,
        stored_bytes: None,
        roundtrip_ok: None,
        encode_seconds: None,
        decode_seconds: None,
        error: Some(error),
        ineligible: false,
    }
}

fn ineligible_outcome(family: Family, candidate: String, input_bytes: u64) -> CandidateOutcome {
    CandidateOutcome {
        family,
        candidate,
        input_bytes,
        stored_bytes: None,
        roundtrip_ok: None,
        encode_seconds: None,
        decode_seconds: None,
        error: None,
        ineligible: true,
    }
}

fn from_production(
    label: &str,
    input_bytes: u64,
    outcome: Result<production::RoundtripResult, Diagnostic>,
) -> CandidateOutcome {
    match outcome {
        Ok(r) => ok_outcome(
            Family::Production,
            r.codec_id,
            input_bytes,
            r.encoded_bytes,
            r.roundtrip_ok,
            r.encode_seconds,
            r.decode_seconds,
        ),
        Err(e) => error_outcome(
            Family::Production,
            label.to_owned(),
            input_bytes,
            e.to_string(),
        ),
    }
}

fn from_optional_production(
    label: &str,
    input_bytes: u64,
    outcome: Result<Option<production::RoundtripResult>, Diagnostic>,
) -> CandidateOutcome {
    match outcome {
        Ok(Some(r)) => ok_outcome(
            Family::Production,
            r.codec_id,
            input_bytes,
            r.encoded_bytes,
            r.roundtrip_ok,
            r.encode_seconds,
            r.decode_seconds,
        ),
        Ok(None) => ineligible_outcome(Family::Production, label.to_owned(), input_bytes),
        Err(e) => error_outcome(
            Family::Production,
            label.to_owned(),
            input_bytes,
            e.to_string(),
        ),
    }
}

fn from_external(
    label: &str,
    input_bytes: u64,
    outcome: std::io::Result<external::ExternalCodecResult>,
) -> CandidateOutcome {
    match outcome {
        Ok(r) => ok_outcome(
            Family::External,
            r.codec_id,
            input_bytes,
            r.encoded_bytes,
            r.roundtrip_ok,
            r.encode_seconds,
            r.decode_seconds,
        ),
        Err(e) => error_outcome(
            Family::External,
            label.to_owned(),
            input_bytes,
            e.to_string(),
        ),
    }
}

/// Representative zstd levels: production's `SUPPORTED_LEVELS` is `[1, 3, 5,
/// 9, 15, 19, 22]`; a matrix row uses a fast/mid/slow subset of it rather
/// than all seven.
const MATRIX_ZSTD_LEVELS: [i32; 3] = [1, 3, 19];
const MATRIX_LZMA2_CONFIGURATIONS: [(u8, u32); 3] =
    [(4, 1024 * 1024), (6, 4 * 1024 * 1024), (9, 8 * 1024 * 1024)];
const MATRIX_BYTE_SHUFFLE_WIDTHS: [u8; 3] = [2, 4, 8];
const MATRIX_BROTLI_QUALITIES: [u32; 3] = [1, 6, 11];
const MATRIX_LEVELS_1_6_9: [u32; 3] = [1, 6, 9];
const MATRIX_LZ4_HC_LEVELS: [u32; 3] = [0, 3, 12];
const MATRIX_DEFLATE_RECONSTRUCT_MAX_CHAIN: u32 = 512;
const MATRIX_PPMD_ORDER: u32 = 6;
const MATRIX_PPMD_MEM_SIZE: u32 = 1 << 20;

/// Runs every production and external candidate over one buffer. See the
/// module docs for the parameter-list rationale and for what is
/// deliberately excluded (dictionary/prefix modes).
pub fn candidates(plaintext: &[u8]) -> Vec<CandidateOutcome> {
    let input_bytes = plaintext.len() as u64;
    let mut out = Vec::new();

    out.push(from_production(
        "store",
        input_bytes,
        production::roundtrip_store(plaintext),
    ));
    for level in MATRIX_ZSTD_LEVELS {
        out.push(from_production(
            &format!("zstd(production,level={level})"),
            input_bytes,
            production::roundtrip_zstd(level, plaintext),
        ));
        out.push(from_production(
            &format!("zstd+delta8(production,level={level})"),
            input_bytes,
            production::roundtrip_zstd_delta8(level, plaintext),
        ));
        for width in MATRIX_BYTE_SHUFFLE_WIDTHS {
            out.push(from_production(
                &format!("zstd+byte-shuffle-{width}(production,level={level})"),
                input_bytes,
                production::roundtrip_zstd_byte_shuffle(level, width, plaintext),
            ));
        }
    }
    out.push(from_production(
        "lz4(production)",
        input_bytes,
        production::roundtrip_lz4(plaintext),
    ));
    for (preset, dictionary_bytes) in MATRIX_LZMA2_CONFIGURATIONS {
        out.push(from_production(
            &format!("lzma2(production,preset={preset},dict={dictionary_bytes})"),
            input_bytes,
            production::roundtrip_lzma2(preset, dictionary_bytes, plaintext),
        ));
    }
    out.push(from_optional_production(
        "deflate-reconstruct(production)",
        input_bytes,
        production::roundtrip_deflate_reconstruct(
            3,
            MATRIX_DEFLATE_RECONSTRUCT_MAX_CHAIN,
            plaintext,
        ),
    ));
    out.push(from_optional_production(
        "jpeg-jxl-reconstruct(production)",
        input_bytes,
        production::roundtrip_jpeg_reconstruct(3, plaintext),
    ));

    for level in MATRIX_ZSTD_LEVELS {
        out.push(from_external(
            &format!("zstd(external,level={level})"),
            input_bytes,
            external::roundtrip_zstd_crate(level, plaintext),
        ));
    }
    out.push(from_external(
        "zstd-ldm(external,level=19,window_log=24)",
        input_bytes,
        external::roundtrip_zstd_ldm(19, 24, plaintext),
    ));
    for quality in MATRIX_BROTLI_QUALITIES {
        out.push(from_external(
            &format!("brotli(external,quality={quality})"),
            input_bytes,
            external::roundtrip_brotli(quality, 22, plaintext),
        ));
    }
    for level in MATRIX_LEVELS_1_6_9 {
        out.push(from_external(
            &format!("bzip2(external,level={level})"),
            input_bytes,
            external::roundtrip_bzip2(level, plaintext),
        ));
        out.push(from_external(
            &format!("deflate(external,level={level})"),
            input_bytes,
            external::roundtrip_deflate(level, plaintext),
        ));
        out.push(from_external(
            &format!("zlib(external,level={level})"),
            input_bytes,
            external::roundtrip_zlib(level, plaintext),
        ));
        for filter in [
            external::XzBcjFilter::None,
            external::XzBcjFilter::X86,
            external::XzBcjFilter::Arm64,
        ] {
            out.push(from_external(
                &format!("xz(external,preset={level},filter={})", filter.label()),
                input_bytes,
                external::roundtrip_xz(level, filter, plaintext),
            ));
        }
    }
    for level in MATRIX_LZ4_HC_LEVELS {
        out.push(from_external(
            &format!("lz4-hc(external,level={level})"),
            input_bytes,
            external::roundtrip_lz4_hc(level, plaintext),
        ));
    }
    out.push(from_external(
        &format!("ppmd7(external,order={MATRIX_PPMD_ORDER},mem_size={MATRIX_PPMD_MEM_SIZE})"),
        input_bytes,
        external::roundtrip_ppmd(MATRIX_PPMD_ORDER, MATRIX_PPMD_MEM_SIZE, plaintext)
            .map_err(std::io::Error::other),
    ));

    out
}

/// One chunk's row in a per-chunk matrix.
#[derive(Debug, Clone, Serialize)]
pub struct ChunkMatrixRow {
    pub chunk_index: usize,
    pub chunk_start: u64,
    pub chunk_len: u64,
    pub outcomes: Vec<CandidateOutcome>,
}

/// The `gear-norm-v1` parameters production's planner pairs with each
/// `CompressionProfile` (`entrybound::chunker::{FAST_V2, BALANCED_V2,
/// DENSE_V2, EXTREME_V2}`). A profile's own planner may pick a different
/// candidate from a content-dependent evaluation
/// (`chunker::select_parameters`); this is the one fixed policy per profile,
/// used here so a chunk matrix's boundaries are reproducible from the
/// profile name alone.
pub fn chunking_parameters_for_profile(profile: CompressionProfile) -> ChunkingParameters {
    match profile {
        CompressionProfile::Fast => chunker::FAST_V2,
        CompressionProfile::Balanced => chunker::BALANCED_V2,
        CompressionProfile::Dense => chunker::DENSE_V2,
        CompressionProfile::Extreme => chunker::EXTREME_V2,
    }
}

/// Splits `plaintext` into chunks with production's own `chunk_ranges` at
/// `parameters`, then runs [`candidates`] over each chunk independently.
pub fn chunk_matrix(
    parameters: ChunkingParameters,
    plaintext: &[u8],
) -> Result<Vec<ChunkMatrixRow>, Diagnostic> {
    let ranges = chunker::chunk_ranges(plaintext, parameters)?;
    Ok(ranges
        .iter()
        .enumerate()
        .map(|(chunk_index, range)| ChunkMatrixRow {
            chunk_index,
            chunk_start: range.start as u64,
            chunk_len: range.len() as u64,
            outcomes: candidates(&plaintext[range.start..range.end]),
        })
        .collect())
}

/// One structural-transform candidate's false-positive check: does applying
/// the transform ahead of a reference codec (zstd level 3, the same
/// reference every row uses) actually help, or does it lose bytes relative
/// to compressing the untransformed input directly? `false_positive` is true
/// exactly when it does not help (`transformed_compressed_bytes >=
/// reference_compressed_bytes`) -- the concrete form "a transform applied
/// where it loses bytes" takes for a transform that never changes length
/// (BCJ, byte-plane split, delta-of-delta: all are pure permutations or
/// wrapping substitutions, so `transformed_len_bytes` always equals
/// `input_bytes` and only the downstream codec's output can differ).
#[derive(Debug, Clone, Serialize)]
pub struct TransformFalsePositiveRow {
    pub transform: String,
    pub input_bytes: u64,
    pub transformed_len_bytes: u64,
    pub reference_compressed_bytes: u64,
    pub transformed_compressed_bytes: u64,
    pub false_positive: bool,
}

fn zstd3_len(data: &[u8]) -> std::io::Result<u64> {
    Ok(external::roundtrip_zstd_crate(3, data)?.encoded_bytes)
}

/// Runs every structural transform this crate knows (production's own
/// `byte-shuffle` at width 2/4/8, plus this crate's BCJ/delta-of-delta/
/// byte-plane-split candidates) against `plaintext`, each followed by zstd
/// level 3, and reports which ones lost bytes relative to compressing
/// `plaintext` directly.
pub fn transform_false_positive_analysis(
    plaintext: &[u8],
) -> std::io::Result<Vec<TransformFalsePositiveRow>> {
    let input_bytes = plaintext.len() as u64;
    let reference_compressed_bytes = zstd3_len(plaintext)?;
    let mut rows = Vec::new();

    let mut push = |name: String, transformed: Vec<u8>| -> std::io::Result<()> {
        let transformed_len_bytes = transformed.len() as u64;
        let transformed_compressed_bytes = zstd3_len(&transformed)?;
        rows.push(TransformFalsePositiveRow {
            transform: name,
            input_bytes,
            transformed_len_bytes,
            reference_compressed_bytes,
            transformed_compressed_bytes,
            false_positive: transformed_compressed_bytes >= reference_compressed_bytes,
        });
        Ok(())
    };

    for width in MATRIX_BYTE_SHUFFLE_WIDTHS {
        use entrybound::research::transform as production_transform;
        let step = production_transform::byte_shuffle_step(width)
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        let transformed = production_transform::forward_pipeline(&[step], plaintext)
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        push(format!("byte-shuffle-{width}(production)"), transformed)?;
    }

    for kind in [
        transform::BcjKind::X86,
        transform::BcjKind::Arm64,
        transform::BcjKind::RiscV,
    ] {
        let transformed = transform::bcj_forward(kind, plaintext)?;
        push(format!("{}(harness)", kind.label()), transformed)?;
    }

    for width in [1u8, 2, 4, 8] {
        let transformed = transform::delta_of_delta_forward(width, plaintext);
        push(format!("delta-of-delta-{width}(harness)"), transformed)?;
    }

    for width in [3u8, 5, 6, 7] {
        let transformed = transform::byte_plane_split_forward(width, plaintext);
        push(format!("byte-plane-split-{width}(harness)"), transformed)?;
    }

    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repeating_text(len: usize) -> Vec<u8> {
        b"the quick brown fox jumps over the lazy dog; "
            .iter()
            .cycle()
            .take(len)
            .copied()
            .collect()
    }

    #[test]
    fn candidates_all_round_trip_or_are_typed_ineligible() {
        let data = repeating_text(300_000);
        let outcomes = candidates(&data);
        assert!(outcomes.len() > 20, "expected a broad candidate matrix");
        for outcome in &outcomes {
            assert!(
                outcome.error.is_none(),
                "candidate {} failed: {:?}",
                outcome.candidate,
                outcome.error
            );
            if !outcome.ineligible {
                assert_eq!(
                    outcome.roundtrip_ok,
                    Some(true),
                    "candidate {} did not round-trip",
                    outcome.candidate
                );
            }
        }
    }

    #[test]
    fn chunk_matrix_splits_and_runs_every_chunk() {
        // Below EXTREME_V2's minimum size, so this is exactly one chunk --
        // still exercises the real chunk_ranges call and row shape.
        let data = repeating_text(20_000);
        let rows = chunk_matrix(entrybound::chunker::EXTREME_V2, &data).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].chunk_len, data.len() as u64);
        assert!(!rows[0].outcomes.is_empty());
    }

    #[test]
    fn transform_false_positive_analysis_flags_bcj_on_non_executable_text() {
        // Plain repetitive text has no x86/ARM64/RISC-V branch instructions
        // for a BCJ filter to usefully rewrite, so scrambling it this way is
        // expected to not help (and often to slightly hurt) a general
        // compressor -- exactly the false-positive case this scan exists to
        // catch.
        let data = repeating_text(200_000);
        let rows = transform_false_positive_analysis(&data).unwrap();
        assert!(!rows.is_empty());
        for row in &rows {
            assert_eq!(row.transformed_len_bytes, row.input_bytes);
        }
        let bcj_x86 = rows
            .iter()
            .find(|r| r.transform.starts_with("bcj-x86"))
            .expect("bcj-x86 row present");
        assert!(
            bcj_x86.false_positive,
            "expected BCJ on plain text to lose bytes"
        );
    }

    #[test]
    fn transform_false_positive_analysis_does_not_flag_delta_of_delta_on_a_ramp() {
        let ramp: Vec<u8> = (0..50_000u32).flat_map(|i| (i * 3).to_le_bytes()).collect();
        let rows = transform_false_positive_analysis(&ramp).unwrap();
        let dod4 = rows
            .iter()
            .find(|r| r.transform == "delta-of-delta-4(harness)")
            .expect("delta-of-delta-4 row present");
        assert!(
            !dod4.false_positive,
            "expected second-order delta to help compress a linear ramp"
        );
    }
}
