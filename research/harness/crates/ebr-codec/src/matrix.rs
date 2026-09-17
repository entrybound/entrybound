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

use ebr_common::measure::ScopedMemory;
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
/// `error` is `Some` (the call itself failed).
#[derive(Debug, Clone, Serialize)]
pub struct CandidateOutcome {
    pub family: Family,
    /// Stable matrix label (`zstd(production,level=3)`, ...): the value
    /// `--candidate` filters on, identical for success and failure rows.
    pub label: String,
    /// The codec's own descriptive id on success, or `label` otherwise.
    pub candidate: String,
    pub input_bytes: u64,
    pub stored_bytes: Option<u64>,
    pub roundtrip_ok: Option<bool>,
    /// Non-decision-grade smoke timing; see `production`'s module docs.
    pub encode_seconds: Option<f64>,
    /// Non-decision-grade smoke timing; see `production`'s module docs.
    pub decode_seconds: Option<f64>,
    /// Match window / dictionary / block / model size (see `external`).
    pub window_bytes: Option<u64>,
    pub threads: Option<u32>,
    pub container: Option<&'static str>,
    pub encode_memory: Option<ScopedMemory>,
    pub decode_memory: Option<ScopedMemory>,
    pub error: Option<String>,
    /// True for a reconstruction candidate (`deflate-reconstruct`,
    /// `jpeg-jxl-reconstruct`) production itself would not attempt over this
    /// input (not a failure -- an eligibility rule).
    pub ineligible: bool,
}

fn empty_outcome(family: Family, label: &str, input_bytes: u64) -> CandidateOutcome {
    CandidateOutcome {
        family,
        label: label.to_owned(),
        candidate: label.to_owned(),
        input_bytes,
        stored_bytes: None,
        roundtrip_ok: None,
        encode_seconds: None,
        decode_seconds: None,
        window_bytes: None,
        threads: None,
        container: None,
        encode_memory: None,
        decode_memory: None,
        error: None,
        ineligible: false,
    }
}

fn from_production_result(
    label: &str,
    input_bytes: u64,
    r: production::RoundtripResult,
) -> CandidateOutcome {
    CandidateOutcome {
        candidate: r.codec_id,
        stored_bytes: Some(r.encoded_bytes),
        roundtrip_ok: Some(r.roundtrip_ok),
        encode_seconds: Some(r.encode_seconds),
        decode_seconds: Some(r.decode_seconds),
        window_bytes: r.window_bytes,
        threads: Some(r.threads),
        container: Some("production payload (no container)"),
        encode_memory: Some(r.encode_memory),
        decode_memory: Some(r.decode_memory),
        ..empty_outcome(Family::Production, label, input_bytes)
    }
}

fn from_production(
    label: &str,
    input_bytes: u64,
    outcome: Result<production::RoundtripResult, Diagnostic>,
) -> CandidateOutcome {
    match outcome {
        Ok(r) => from_production_result(label, input_bytes, r),
        Err(e) => CandidateOutcome {
            error: Some(e.to_string()),
            ..empty_outcome(Family::Production, label, input_bytes)
        },
    }
}

fn from_optional_production(
    label: &str,
    input_bytes: u64,
    outcome: Result<Option<production::RoundtripResult>, Diagnostic>,
) -> CandidateOutcome {
    match outcome {
        Ok(Some(r)) => from_production_result(label, input_bytes, r),
        Ok(None) => CandidateOutcome {
            ineligible: true,
            ..empty_outcome(Family::Production, label, input_bytes)
        },
        Err(e) => CandidateOutcome {
            error: Some(e.to_string()),
            ..empty_outcome(Family::Production, label, input_bytes)
        },
    }
}

fn from_external(
    label: &str,
    input_bytes: u64,
    outcome: std::io::Result<external::ExternalCodecResult>,
) -> CandidateOutcome {
    match outcome {
        Ok(r) => CandidateOutcome {
            candidate: r.codec_id,
            stored_bytes: Some(r.encoded_bytes),
            roundtrip_ok: Some(r.roundtrip_ok),
            encode_seconds: Some(r.encode_seconds),
            decode_seconds: Some(r.decode_seconds),
            window_bytes: r.window_bytes,
            threads: Some(r.threads),
            container: Some(r.container),
            encode_memory: Some(r.encode_memory),
            decode_memory: Some(r.decode_memory),
            ..empty_outcome(Family::External, label, input_bytes)
        },
        Err(e) => CandidateOutcome {
            error: Some(e.to_string()),
            ..empty_outcome(Family::External, label, input_bytes)
        },
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
/// 7-Zip PPMd levels 5 (order 6, 16 MiB), 7 (order 16, 64 MiB), and 9
/// (order 32, 256 MiB), each reduced for small inputs exactly as 7-Zip does
/// (`external::ppmd_7zip_parameters`).
const MATRIX_PPMD_7ZIP_LEVELS: [u32; 3] = [5, 7, 9];

/// Runs every production and external candidate over one buffer. See the
/// module docs for the parameter-list rationale and for what is
/// deliberately excluded (dictionary/prefix modes).
pub fn candidates(plaintext: &[u8]) -> Vec<CandidateOutcome> {
    candidates_matching(plaintext, None)
}

/// [`candidates`], restricted to labels containing `filter` when given, so a
/// runner can execute one candidate per process for a clean memory reading.
/// A filter is evaluated before the candidate runs, never after.
pub fn candidates_matching(plaintext: &[u8], filter: Option<&str>) -> Vec<CandidateOutcome> {
    let input_bytes = plaintext.len() as u64;
    let wanted = |label: &str| filter.is_none_or(|needle| label.contains(needle));
    let mut out = Vec::new();

    macro_rules! run {
        ($label:expr, $convert:ident, $call:expr) => {{
            let label: String = $label;
            if wanted(&label) {
                out.push($convert(&label, input_bytes, $call));
            }
        }};
    }

    run!(
        "store(production)".to_owned(),
        from_production,
        production::roundtrip_store(plaintext)
    );
    for level in MATRIX_ZSTD_LEVELS {
        run!(
            format!("zstd(production,level={level})"),
            from_production,
            production::roundtrip_zstd(level, plaintext)
        );
        run!(
            format!("zstd+delta8(production,level={level})"),
            from_production,
            production::roundtrip_zstd_delta8(level, plaintext)
        );
        for width in MATRIX_BYTE_SHUFFLE_WIDTHS {
            run!(
                format!("zstd+byte-shuffle-{width}(production,level={level})"),
                from_production,
                production::roundtrip_zstd_byte_shuffle(level, width, plaintext)
            );
        }
    }
    run!(
        "lz4(production)".to_owned(),
        from_production,
        production::roundtrip_lz4(plaintext)
    );
    for (preset, dictionary_bytes) in MATRIX_LZMA2_CONFIGURATIONS {
        run!(
            format!("lzma2(production,preset={preset},dict={dictionary_bytes})"),
            from_production,
            production::roundtrip_lzma2(preset, dictionary_bytes, plaintext)
        );
    }
    run!(
        "deflate-reconstruct(production)".to_owned(),
        from_optional_production,
        production::roundtrip_deflate_reconstruct(
            3,
            MATRIX_DEFLATE_RECONSTRUCT_MAX_CHAIN,
            plaintext
        )
    );
    run!(
        "jpeg-jxl-reconstruct(production)".to_owned(),
        from_optional_production,
        production::roundtrip_jpeg_reconstruct(3, plaintext)
    );

    for level in MATRIX_ZSTD_LEVELS {
        run!(
            format!("zstd(external,level={level})"),
            from_external,
            external::roundtrip_zstd_crate(level, plaintext)
        );
    }
    // Window-matched control first, then LDM at the same window, so a gain
    // can be attributed to LDM rather than to the larger window.
    run!(
        "zstd-window(external,level=19,window_log=24,ldm=off)".to_owned(),
        from_external,
        external::roundtrip_zstd_window(19, 24, plaintext)
    );
    run!(
        "zstd-ldm(external,level=19,window_log=24)".to_owned(),
        from_external,
        external::roundtrip_zstd_ldm(19, 24, plaintext)
    );
    for quality in MATRIX_BROTLI_QUALITIES {
        run!(
            format!("brotli(external,quality={quality},lgwin=22)"),
            from_external,
            external::roundtrip_brotli(quality, 22, plaintext)
        );
    }
    run!(
        "brotli(external,quality=11,lgwin=24)".to_owned(),
        from_external,
        external::roundtrip_brotli(11, 24, plaintext)
    );
    for level in MATRIX_LEVELS_1_6_9 {
        run!(
            format!("bzip2(external,level={level})"),
            from_external,
            external::roundtrip_bzip2(level, plaintext)
        );
        run!(
            format!("deflate(external,level={level})"),
            from_external,
            external::roundtrip_deflate(level, plaintext)
        );
        run!(
            format!("zlib(external,level={level})"),
            from_external,
            external::roundtrip_zlib(level, plaintext)
        );
        for filter in [
            external::XzBcjFilter::None,
            external::XzBcjFilter::X86,
            external::XzBcjFilter::Arm64,
        ] {
            run!(
                format!("xz(external,preset={level},filter={})", filter.label()),
                from_external,
                external::roundtrip_xz(level, filter, plaintext)
            );
        }
    }
    for level in MATRIX_LZ4_HC_LEVELS {
        run!(
            format!("lz4-hc(external,level={level})"),
            from_external,
            external::roundtrip_lz4_hc(level, plaintext)
        );
    }
    for level in MATRIX_PPMD_7ZIP_LEVELS {
        let (order, mem_size) = external::ppmd_7zip_parameters(level, input_bytes);
        run!(
            format!("ppmd7(external,7zip-level={level},order={order},mem_size={mem_size})"),
            from_external,
            external::roundtrip_ppmd(order, mem_size, plaintext)
        );
    }

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
    chunk_matrix_matching(parameters, plaintext, None)
}

/// [`chunk_matrix`] restricted like [`candidates_matching`].
pub fn chunk_matrix_matching(
    parameters: ChunkingParameters,
    plaintext: &[u8],
    filter: Option<&str>,
) -> Result<Vec<ChunkMatrixRow>, Diagnostic> {
    let ranges = chunker::chunk_ranges(plaintext, parameters)?;
    Ok(ranges
        .iter()
        .enumerate()
        .map(|(chunk_index, range)| ChunkMatrixRow {
            chunk_index,
            chunk_start: range.start as u64,
            chunk_len: range.len() as u64,
            outcomes: candidates_matching(&plaintext[range.start..range.end], filter),
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

    #[test]
    fn candidate_filter_runs_only_matching_labels_and_reports_resources() {
        let data = repeating_text(100_000);
        let outcomes = candidates_matching(&data, Some("ppmd7"));
        assert_eq!(outcomes.len(), 3);
        for outcome in &outcomes {
            assert!(outcome.label.starts_with("ppmd7(external,7zip-level="));
            assert_eq!(outcome.roundtrip_ok, Some(true), "{}", outcome.label);
            assert_eq!(outcome.threads, Some(1));
            assert!(outcome.window_bytes.is_some());
            assert!(outcome.encode_memory.is_some());
        }
        let control = candidates_matching(&data, Some("zstd-window"));
        assert_eq!(control.len(), 1);
        // Single-segment frame: the window is the (smaller) content size.
        assert_eq!(control[0].window_bytes, Some(data.len() as u64));
    }
}
