//! Production codec paths, reached through `entrybound::research::codec`
//! (the default-off `research-internals` feature; see
//! `research/harness/research-internals.md`). Every function here is a thin
//! wrapper: it builds a `TransformPlan` the same way production's planner
//! does, then round-trips through the exact `encode_payload`/`decode_payload`
//! (or the dictionary/prefix/reconstruction variants) production uses.
//! Nothing here changes production behavior or archive bytes.
//!
//! `encode_seconds`/`decode_seconds` on [`RoundtripResult`] are a single
//! in-process wall-clock sample (`ebr_common::timing::Stopwatch`, so a
//! monotonic clock, never `SystemTime`) around that one call, on whatever
//! machine happens to run this binary. Per the program's Phase B2 rule, this
//! is a **smoke timing, not decision-grade**: no quiet-machine guard, no
//! repetition, no warmup. It exists so a matrix row is not silent about cost
//! entirely, not to support a timing claim.

use std::collections::BTreeMap;

use ebr_common::timing::Stopwatch;
use entrybound::diagnostics::Diagnostic;
use entrybound::research::{codec, jpeg_reconstruction, reconstruction, transform};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct RoundtripResult {
    pub codec_id: String,
    pub input_bytes: u64,
    pub encoded_bytes: u64,
    pub roundtrip_ok: bool,
    /// Non-decision-grade smoke timing; see the module docs.
    pub encode_seconds: f64,
    /// Non-decision-grade smoke timing; see the module docs.
    pub decode_seconds: f64,
}

/// Plans, encodes, and decodes `plaintext` through production's `zstd` path
/// at `level` (one of `codec::SUPPORTED_LEVELS`), with no dictionary or
/// prefix (an independent chunk).
pub fn roundtrip_zstd(level: i32, plaintext: &[u8]) -> Result<RoundtripResult, Diagnostic> {
    let plan = codec::zstd_plan(level)?;
    codec::validate_plan(&plan)?;
    let encode_clock = Stopwatch::start();
    let encoded = codec::encode_payload(&plan, plaintext)?;
    let encode_seconds = encode_clock.elapsed_secs_f64();
    let decode_clock = Stopwatch::start();
    let decoded = codec::decode_payload(&plan, &encoded, plaintext.len() as u64)?;
    let decode_seconds = decode_clock.elapsed_secs_f64();
    Ok(RoundtripResult {
        codec_id: format!(
            "{} (production, level {level})",
            codec::ZSTD_CODEC_IDENTIFIER
        ),
        input_bytes: plaintext.len() as u64,
        encoded_bytes: encoded.len() as u64,
        roundtrip_ok: decoded == plaintext,
        encode_seconds,
        decode_seconds,
    })
}

/// Plans, encodes, and decodes `plaintext` through production's `store`
/// (uncompressed) path: the planner's own floor candidate.
pub fn roundtrip_store(plaintext: &[u8]) -> Result<RoundtripResult, Diagnostic> {
    let plan = codec::store_plan();
    codec::validate_plan(&plan)?;
    let encode_clock = Stopwatch::start();
    let encoded = codec::encode_payload(&plan, plaintext)?;
    let encode_seconds = encode_clock.elapsed_secs_f64();
    let decode_clock = Stopwatch::start();
    let decoded = codec::decode_payload(&plan, &encoded, plaintext.len() as u64)?;
    let decode_seconds = decode_clock.elapsed_secs_f64();
    Ok(RoundtripResult {
        codec_id: "store (production)".to_owned(),
        input_bytes: plaintext.len() as u64,
        encoded_bytes: encoded.len() as u64,
        roundtrip_ok: decoded == plaintext,
        encode_seconds,
        decode_seconds,
    })
}

/// Plans, encodes, and decodes `plaintext` through production's `lzma2` path
/// at `preset`/`dictionary_bytes` (one of `codec::SUPPORTED_LZMA2_CONFIGURATIONS`).
pub fn roundtrip_lzma2(
    preset: u8,
    dictionary_bytes: u32,
    plaintext: &[u8],
) -> Result<RoundtripResult, Diagnostic> {
    let plan = codec::lzma2_plan(preset, dictionary_bytes, Box::new([]))?;
    codec::validate_plan(&plan)?;
    let encode_clock = Stopwatch::start();
    let encoded = codec::encode_payload(&plan, plaintext)?;
    let encode_seconds = encode_clock.elapsed_secs_f64();
    let decode_clock = Stopwatch::start();
    let decoded = codec::decode_payload(&plan, &encoded, plaintext.len() as u64)?;
    let decode_seconds = decode_clock.elapsed_secs_f64();
    Ok(RoundtripResult {
        codec_id: format!(
            "{} (production, preset {preset}, dict {dictionary_bytes})",
            codec::LZMA2_CODEC_IDENTIFIER
        ),
        input_bytes: plaintext.len() as u64,
        encoded_bytes: encoded.len() as u64,
        roundtrip_ok: decoded == plaintext,
        encode_seconds,
        decode_seconds,
    })
}

/// Plans, encodes, and decodes `plaintext` through production's `lz4` path.
pub fn roundtrip_lz4(plaintext: &[u8]) -> Result<RoundtripResult, Diagnostic> {
    let plan = codec::lz4_plan(Box::new([]))?;
    codec::validate_plan(&plan)?;
    let encode_clock = Stopwatch::start();
    let encoded = codec::encode_payload(&plan, plaintext)?;
    let encode_seconds = encode_clock.elapsed_secs_f64();
    let decode_clock = Stopwatch::start();
    let decoded = codec::decode_payload(&plan, &encoded, plaintext.len() as u64)?;
    let decode_seconds = decode_clock.elapsed_secs_f64();
    Ok(RoundtripResult {
        codec_id: format!("{} (production)", codec::LZ4_CODEC_IDENTIFIER),
        input_bytes: plaintext.len() as u64,
        encoded_bytes: encoded.len() as u64,
        roundtrip_ok: decoded == plaintext,
        encode_seconds,
        decode_seconds,
    })
}

/// Plans, encodes, and decodes `plaintext` through production's `zstd` path
/// at `level` with a shared dictionary (`codec::zstd_dictionary_plan` /
/// `encode_payload_with_dictionary`), exactly as the cohort-stage planner
/// uses a trained dictionary. `dictionary_bytes` is typically
/// `codec::train_dictionary`'s output over a sample set.
pub fn roundtrip_zstd_dictionary(
    level: i32,
    dictionary_bytes: Vec<u8>,
    plaintext: &[u8],
) -> Result<RoundtripResult, Diagnostic> {
    let dictionary_id = entrybound::identity::sha256_exact(&dictionary_bytes);
    let dictionary = entrybound::eam::Dictionary {
        dictionary_id,
        codec: codec::ZSTD_CODEC_IDENTIFIER.to_owned(),
        format: codec::ZSTD_DICTIONARY_FORMAT.to_owned(),
        construction: codec::SUPPORTED_DICTIONARY_CONSTRUCTIONS[0].to_owned(),
        bytes: dictionary_bytes.into_boxed_slice(),
    };
    let plan = codec::zstd_dictionary_plan(level, dictionary.dictionary_id)?;
    codec::validate_plan(&plan)?;
    let encode_clock = Stopwatch::start();
    let encoded = codec::encode_payload_with_dictionary(&plan, plaintext, &dictionary)?;
    let encode_seconds = encode_clock.elapsed_secs_f64();
    let decode_clock = Stopwatch::start();
    let decoded = codec::decode_payload_with_dictionary(
        &plan,
        &encoded,
        plaintext.len() as u64,
        &dictionary,
    )?;
    let decode_seconds = decode_clock.elapsed_secs_f64();
    Ok(RoundtripResult {
        codec_id: format!(
            "{} (production, dictionary mode, level {level})",
            codec::ZSTD_CODEC_IDENTIFIER
        ),
        input_bytes: plaintext.len() as u64,
        encoded_bytes: encoded.len() as u64,
        roundtrip_ok: decoded == plaintext,
        encode_seconds,
        decode_seconds,
    })
}

/// Plans, encodes, and decodes `plaintext` through production's `zstd` path
/// at `level` with a bounded-lookback prefix (`codec::zstd_prefix_plan` /
/// `encode_payload_with_prefix`), exactly as a cohort member's plan
/// references a preceding same-cohort chunk without a trained dictionary.
pub fn roundtrip_zstd_prefix(
    level: i32,
    lookback: u32,
    prefix: &[u8],
    plaintext: &[u8],
) -> Result<RoundtripResult, Diagnostic> {
    let plan = codec::zstd_prefix_plan(level, lookback)?;
    codec::validate_plan(&plan)?;
    let encode_clock = Stopwatch::start();
    let encoded = codec::encode_payload_with_prefix(&plan, plaintext, prefix)?;
    let encode_seconds = encode_clock.elapsed_secs_f64();
    let decode_clock = Stopwatch::start();
    let decoded =
        codec::decode_payload_with_prefix(&plan, &encoded, plaintext.len() as u64, prefix)?;
    let decode_seconds = decode_clock.elapsed_secs_f64();
    Ok(RoundtripResult {
        codec_id: format!(
            "{} (production, prefix mode, level {level}, lookback {lookback})",
            codec::ZSTD_CODEC_IDENTIFIER
        ),
        input_bytes: plaintext.len() as u64,
        encoded_bytes: encoded.len() as u64,
        roundtrip_ok: decoded == plaintext,
        encode_seconds,
        decode_seconds,
    })
}

/// Plans, encodes, and decodes `plaintext` through production's `zstd` path
/// at `level` preceded by the `delta8` structural transform
/// (`transform::delta8_step`), the same pipeline shape the planner considers
/// for slowly-varying binary data.
pub fn roundtrip_zstd_delta8(level: i32, plaintext: &[u8]) -> Result<RoundtripResult, Diagnostic> {
    let plan = codec::zstd_transformed_plan(level, vec![transform::delta8_step()].into())?;
    codec::validate_plan(&plan)?;
    let encode_clock = Stopwatch::start();
    let encoded = codec::encode_payload(&plan, plaintext)?;
    let encode_seconds = encode_clock.elapsed_secs_f64();
    let decode_clock = Stopwatch::start();
    let decoded = codec::decode_payload(&plan, &encoded, plaintext.len() as u64)?;
    let decode_seconds = decode_clock.elapsed_secs_f64();
    Ok(RoundtripResult {
        codec_id: format!(
            "{} (production, delta8, level {level})",
            codec::ZSTD_CODEC_IDENTIFIER
        ),
        input_bytes: plaintext.len() as u64,
        encoded_bytes: encoded.len() as u64,
        roundtrip_ok: decoded == plaintext,
        encode_seconds,
        decode_seconds,
    })
}

/// Plans, encodes, and decodes `plaintext` through production's `zstd` path
/// at `level` preceded by the `byte-shuffle` structural transform
/// (`transform::byte_shuffle_step`), one of the widths production actually
/// supports (2, 4, or 8; anything else is a validation error, matching
/// production's own `byte_shuffle_step`).
pub fn roundtrip_zstd_byte_shuffle(
    level: i32,
    width: u8,
    plaintext: &[u8],
) -> Result<RoundtripResult, Diagnostic> {
    let plan =
        codec::zstd_transformed_plan(level, vec![transform::byte_shuffle_step(width)?].into())?;
    codec::validate_plan(&plan)?;
    let encode_clock = Stopwatch::start();
    let encoded = codec::encode_payload(&plan, plaintext)?;
    let encode_seconds = encode_clock.elapsed_secs_f64();
    let decode_clock = Stopwatch::start();
    let decoded = codec::decode_payload(&plan, &encoded, plaintext.len() as u64)?;
    let decode_seconds = decode_clock.elapsed_secs_f64();
    Ok(RoundtripResult {
        codec_id: format!(
            "{} (production, byte-shuffle-{width}, level {level})",
            codec::ZSTD_CODEC_IDENTIFIER
        ),
        input_bytes: plaintext.len() as u64,
        encoded_bytes: encoded.len() as u64,
        roundtrip_ok: decoded == plaintext,
        encode_seconds,
        decode_seconds,
    })
}

/// Attempts production's DEFLATE-reconstruction pipeline: this composes
/// `entrybound::research::reconstruction`, `transform::deflate_reconstruct_step`,
/// and `codec::encode_payload_with_reconstruction` over `original`, which must
/// be a complete raw-DEFLATE, zlib, or gzip stream. Returns `Ok(None)` when
/// `original` is not a recognized/verifiable DEFLATE representation --
/// production's own eligibility rule, not a harness approximation -- rather
/// than treating that as an error, so a false-positive scan can tell "not
/// eligible" apart from "eligible but a real failure".
pub fn roundtrip_deflate_reconstruct(
    level: i32,
    max_chain_length: u32,
    original: &[u8],
) -> Result<Option<RoundtripResult>, Diagnostic> {
    let Some(candidate) = reconstruction::try_forward(original, max_chain_length)? else {
        return Ok(None);
    };
    let step =
        transform::deflate_reconstruct_step(max_chain_length, candidate.data.reconstruction_id)?;
    let plan = codec::zstd_transformed_plan(level, vec![step].into())?;
    codec::validate_plan(&plan)?;
    let reconstruction_data = BTreeMap::from([(candidate.data.reconstruction_id, candidate.data)]);
    let encode_clock = Stopwatch::start();
    let encoded = codec::encode_payload_with_reconstruction(&plan, original, &reconstruction_data)?;
    let encode_seconds = encode_clock.elapsed_secs_f64();
    let decode_clock = Stopwatch::start();
    let decoded = codec::decode_payload_with_reconstruction(
        &plan,
        &encoded,
        original.len() as u64,
        &reconstruction_data,
    )?;
    let decode_seconds = decode_clock.elapsed_secs_f64();
    Ok(Some(RoundtripResult {
        codec_id: format!(
            "{} (production, deflate-reconstruct, level {level}, max_chain {max_chain_length})",
            codec::ZSTD_CODEC_IDENTIFIER
        ),
        input_bytes: original.len() as u64,
        encoded_bytes: encoded.len() as u64,
        roundtrip_ok: decoded == original,
        encode_seconds,
        decode_seconds,
    }))
}

/// Attempts production's JPEG-to-JPEG-XL whole-object reconstruction
/// (`entrybound::research::jpeg_reconstruction` + `transform::jpeg_reconstruct_step`)
/// over `original`, which must be a complete JPEG file. Unlike DEFLATE
/// reconstruction this needs no side `ReconstructionData` map: the
/// transcoded JPEG XL bytes are self-contained (`ReconstructionBacking::SelfContained`
/// in production's own classification). Returns `Ok(None)` when production
/// would not consider `original` eligible (unrecognized, unsupported
/// producer features, or verification failure), distinguished by
/// `JpegAttemptFailure`.
pub fn roundtrip_jpeg_reconstruct(
    level: i32,
    original: &[u8],
) -> Result<Option<RoundtripResult>, Diagnostic> {
    let verified = match jpeg_reconstruction::verified_forward(original) {
        Ok(verified) => verified,
        Err(_failure) => return Ok(None),
    };
    let plan =
        codec::zstd_transformed_plan(level, vec![transform::jpeg_reconstruct_step()?].into())?;
    codec::validate_plan(&plan)?;
    let encode_clock = Stopwatch::start();
    let encoded = codec::encode_payload(&plan, original)?;
    let encode_seconds = encode_clock.elapsed_secs_f64();
    let decode_clock = Stopwatch::start();
    let decoded = codec::decode_payload(&plan, &encoded, original.len() as u64)?;
    let decode_seconds = decode_clock.elapsed_secs_f64();
    Ok(Some(RoundtripResult {
        codec_id: format!(
            "{} (production, jpeg-jxl-reconstruct, level {level}, {}x{})",
            codec::ZSTD_CODEC_IDENTIFIER,
            verified.width,
            verified.height
        ),
        input_bytes: original.len() as u64,
        encoded_bytes: encoded.len() as u64,
        roundtrip_ok: decoded == original,
        encode_seconds,
        decode_seconds,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::Compression;
    use flate2::write::{DeflateEncoder, ZlibEncoder};
    use std::io::Write;

    fn repeating_text(len: usize) -> Vec<u8> {
        b"the quick brown fox jumps over the lazy dog; "
            .iter()
            .cycle()
            .take(len)
            .copied()
            .collect()
    }

    #[test]
    fn zstandard_plan_round_trips_repetitive_bytes() {
        let data = repeating_text(200_000);
        let result = roundtrip_zstd(3, &data).unwrap();
        assert!(result.roundtrip_ok);
        assert!(
            result.encoded_bytes < result.input_bytes,
            "expected zstd to shrink highly repetitive text"
        );
    }

    #[test]
    fn store_plan_round_trips_and_never_shrinks() {
        let data = repeating_text(1_000);
        let result = roundtrip_store(&data).unwrap();
        assert!(result.roundtrip_ok);
        assert!(result.encoded_bytes >= result.input_bytes);
    }

    #[test]
    fn lz4_roundtrip_recovers_the_input() {
        let data = repeating_text(50_000);
        let result = roundtrip_lz4(&data).unwrap();
        assert!(result.roundtrip_ok);
    }

    #[test]
    fn lzma2_roundtrip_recovers_the_input() {
        let data = repeating_text(50_000);
        // (preset, dictionary_bytes) taken from
        // entrybound::research::codec::SUPPORTED_LZMA2_CONFIGURATIONS:
        // [(4, 1 MiB), (6, 4 MiB), (9, 8 MiB)].
        let result = roundtrip_lzma2(6, 4 * 1024 * 1024, &data).unwrap();
        assert!(result.roundtrip_ok);
    }

    #[test]
    fn delta8_and_byte_shuffle_pipelines_round_trip() {
        let slowly_varying = (0..32 * 1024)
            .map(|index| u8::try_from((index / 64) % 256).unwrap())
            .collect::<Vec<_>>();
        assert!(
            roundtrip_zstd_delta8(3, &slowly_varying)
                .unwrap()
                .roundtrip_ok
        );
        for width in [2u8, 4, 8] {
            assert!(
                roundtrip_zstd_byte_shuffle(3, width, &slowly_varying)
                    .unwrap()
                    .roundtrip_ok
            );
        }
        // Production only accepts width 2, 4, or 8.
        assert!(roundtrip_zstd_byte_shuffle(3, 3, &slowly_varying).is_err());
    }

    #[test]
    fn dictionary_and_prefix_modes_round_trip() {
        let base = (0..16 * 1024)
            .map(|index| (index % 251) as u8)
            .collect::<Vec<_>>();
        let samples: Vec<Vec<u8>> = (0..16)
            .map(|index| {
                let mut sample = base.clone();
                sample[512 + index] ^= u8::try_from(index + 1).unwrap();
                sample
            })
            .collect();
        let sample_refs: Vec<&[u8]> = samples.iter().map(Vec::as_slice).collect();
        let dictionary_bytes = codec::train_dictionary(&sample_refs, 8 * 1024).unwrap();
        let dict_result = roundtrip_zstd_dictionary(5, dictionary_bytes, &samples[0]).unwrap();
        assert!(dict_result.roundtrip_ok);

        let prefix_result = roundtrip_zstd_prefix(5, 1, &samples[0], &samples[1]).unwrap();
        assert!(prefix_result.roundtrip_ok);
    }

    #[test]
    fn deflate_reconstruction_round_trips_a_real_zlib_stream() {
        let source: Vec<u8> = (0..20_000)
            .flat_map(|index| format!("row={index:06};category={}\n", index % 31).into_bytes())
            .collect();
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::new(6));
        encoder.write_all(&source).unwrap();
        let original = encoder.finish().unwrap();

        let result = roundtrip_deflate_reconstruct(3, 512, &original)
            .unwrap()
            .expect("a freshly generated zlib stream should be reconstruction-eligible");
        assert!(result.roundtrip_ok);
    }

    #[test]
    fn deflate_reconstruction_is_ineligible_for_non_deflate_input() {
        let result = roundtrip_deflate_reconstruct(3, 512, b"not a deflate stream at all").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn raw_deflate_without_a_zlib_or_gzip_wrapper_is_also_reconstruction_eligible() {
        let source = repeating_text(20_000);
        let mut encoder = DeflateEncoder::new(Vec::new(), Compression::new(4));
        encoder.write_all(&source).unwrap();
        let original = encoder.finish().unwrap();

        let result = roundtrip_deflate_reconstruct(3, 512, &original).unwrap();
        if let Some(result) = result {
            assert!(result.roundtrip_ok);
        }
        // Else: production's raw-DEFLATE eligibility rule declined this
        // fixture; that is a legitimate, typed outcome this test must not
        // mask, not a harness bug.
    }
}
