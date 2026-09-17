//! Candidate external codecs, reached directly rather than through
//! `entrybound::eam::TransformPlan`.
//!
//! Two kinds live here:
//!
//! - **Parameters production's own codecs cannot reach.** Production's
//!   `zstd` path (`entrybound::codec::zstd_plan`) fixes several parameters
//!   this module's `zstd` pin matches exactly (`zstd = "=0.13.3"`), but
//!   calling the `zstd` crate directly reaches encoder parameters
//!   `TransformPlan` does not model at all: long-distance matching, an
//!   explicit window log decoupled from the level ([`roundtrip_zstd_ldm`]).
//! - **Codecs production has never used at all**: brotli, bzip2, DEFLATE and
//!   zlib at every level, xz with BCJ filters, LZ4 HC levels, and PPMd7. Each
//!   is a candidate, evaluated as an alternative to production's codec set,
//!   not a replacement for it -- see `research/harness/research-internals.md`
//!   ("Stability and promotion").
//!
//! Every codec here is exact-pinned in `Cargo.toml` (matching production's
//! own pin where the crate is also a production dependency), so a size
//! comparison is never confounded by a different build of the same
//! algorithm. `encode_seconds`/`decode_seconds` are the same kind of
//! non-decision-grade smoke timing as `production::RoundtripResult`'s --
//! see that module's docs.

use ebr_common::timing::Stopwatch;
use serde::Serialize;
use std::io::{Read, Write};

#[derive(Debug, Clone, Serialize)]
pub struct ExternalCodecResult {
    pub codec_id: String,
    pub input_bytes: u64,
    pub encoded_bytes: u64,
    pub roundtrip_ok: bool,
    pub encode_seconds: f64,
    pub decode_seconds: f64,
}

#[allow(clippy::too_many_arguments)]
fn result(
    codec_id: String,
    input_bytes: usize,
    encoded: &[u8],
    decoded: &[u8],
    original: &[u8],
    encode_seconds: f64,
    decode_seconds: f64,
) -> ExternalCodecResult {
    ExternalCodecResult {
        codec_id,
        input_bytes: input_bytes as u64,
        encoded_bytes: encoded.len() as u64,
        roundtrip_ok: decoded == original,
        encode_seconds,
        decode_seconds,
    }
}

/// Plain `zstd` crate encode/decode at `level`, default frame settings (no
/// long-distance matching, no explicit window log, single-threaded). Not the
/// same call path as `entrybound::codec::zstd_plan` +
/// `encode_payload`/`decode_payload`, even though both ultimately call into
/// the same underlying `libzstd`.
pub fn roundtrip_zstd_crate(level: i32, plaintext: &[u8]) -> std::io::Result<ExternalCodecResult> {
    let encode_clock = Stopwatch::start();
    let encoded = zstd::stream::encode_all(plaintext, level)?;
    let encode_seconds = encode_clock.elapsed_secs_f64();
    let decode_clock = Stopwatch::start();
    let decoded = zstd::stream::decode_all(encoded.as_slice())?;
    let decode_seconds = decode_clock.elapsed_secs_f64();
    Ok(result(
        format!("zstd (external crate =0.13.3, level {level}, default frame settings)"),
        plaintext.len(),
        &encoded,
        &decoded,
        plaintext,
        encode_seconds,
        decode_seconds,
    ))
}

/// `zstd` with long-distance matching enabled and an explicit window log --
/// the two parameters `entrybound::codec::zstd_plan` never sets
/// (production always calls `long_distance_matching(false)` and a level-tied
/// window log; see `crates/entrybound/src/codec.rs`'s `encode_zstd`).
/// `window_log` must stay at or below zstd's default decode limit (27, i.e.
/// 128 MiB) so decoding needs no matching `DParameter::WindowLogMax` --
/// callers should not exceed that here.
pub fn roundtrip_zstd_ldm(
    level: i32,
    window_log: u32,
    plaintext: &[u8],
) -> std::io::Result<ExternalCodecResult> {
    let encode_clock = Stopwatch::start();
    let mut compressor = zstd::bulk::Compressor::new(level)?;
    compressor.window_log(window_log)?;
    compressor.long_distance_matching(true)?;
    compressor.include_contentsize(true)?;
    let encoded = compressor.compress(plaintext)?;
    let encode_seconds = encode_clock.elapsed_secs_f64();
    let decode_clock = Stopwatch::start();
    let decoded = zstd::stream::decode_all(encoded.as_slice())?;
    let decode_seconds = decode_clock.elapsed_secs_f64();
    Ok(result(
        format!("zstd (external crate =0.13.3, level {level}, LDM on, window_log {window_log})"),
        plaintext.len(),
        &encoded,
        &decoded,
        plaintext,
        encode_seconds,
        decode_seconds,
    ))
}

/// Pure-Rust `brotli` at `quality` (0-11) and `lgwin` (window bits, 10-24).
/// Not a production dependency at all -- production has no brotli codec.
pub fn roundtrip_brotli(
    quality: u32,
    lgwin: u32,
    plaintext: &[u8],
) -> std::io::Result<ExternalCodecResult> {
    let encode_clock = Stopwatch::start();
    let mut writer = brotli::CompressorWriter::new(Vec::new(), 4096, quality, lgwin);
    writer.write_all(plaintext)?;
    writer.flush()?;
    let encoded = writer.into_inner();
    let encode_seconds = encode_clock.elapsed_secs_f64();

    let decode_clock = Stopwatch::start();
    let mut reader = brotli::Decompressor::new(encoded.as_slice(), 4096);
    let mut decoded = Vec::new();
    reader.read_to_end(&mut decoded)?;
    let decode_seconds = decode_clock.elapsed_secs_f64();

    Ok(result(
        format!("brotli (external crate =9.0.0, quality {quality}, lgwin {lgwin})"),
        plaintext.len(),
        &encoded,
        &decoded,
        plaintext,
        encode_seconds,
        decode_seconds,
    ))
}

/// `bzip2` (libbzip2 binding) at `level` (1-9). Same exact pin
/// (`=0.6.1`) as `crates/entrybound`'s own dependency (used there for
/// legacy `.tar.bz2`/`.7z` reading, not for archive codec output), so this
/// is the identical build of the same algorithm, evaluated as an archive
/// codec candidate rather than a legacy-format reader.
pub fn roundtrip_bzip2(level: u32, plaintext: &[u8]) -> std::io::Result<ExternalCodecResult> {
    let encode_clock = Stopwatch::start();
    let mut encoder = bzip2::write::BzEncoder::new(Vec::new(), bzip2::Compression::new(level));
    encoder.write_all(plaintext)?;
    let encoded = encoder.finish()?;
    let encode_seconds = encode_clock.elapsed_secs_f64();

    let decode_clock = Stopwatch::start();
    let mut decoder = bzip2::read::BzDecoder::new(encoded.as_slice());
    let mut decoded = Vec::new();
    decoder.read_to_end(&mut decoded)?;
    let decode_seconds = decode_clock.elapsed_secs_f64();

    Ok(result(
        format!("bzip2 (external crate =0.6.1, level {level})"),
        plaintext.len(),
        &encoded,
        &decoded,
        plaintext,
        encode_seconds,
        decode_seconds,
    ))
}

/// Raw DEFLATE (no zlib/gzip wrapper) via `flate2` at `level` (0-9). Same
/// exact pin (`=1.1.9`) as `crates/entrybound`'s own dependency (used there
/// only for DEFLATE-reconstruction fixtures/candidates, never as an archive
/// codec) -- evaluated here directly as an archive codec candidate.
pub fn roundtrip_deflate(level: u32, plaintext: &[u8]) -> std::io::Result<ExternalCodecResult> {
    let encode_clock = Stopwatch::start();
    let mut encoder =
        flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::new(level));
    encoder.write_all(plaintext)?;
    let encoded = encoder.finish()?;
    let encode_seconds = encode_clock.elapsed_secs_f64();

    let decode_clock = Stopwatch::start();
    let mut decoder = flate2::read::DeflateDecoder::new(encoded.as_slice());
    let mut decoded = Vec::new();
    decoder.read_to_end(&mut decoded)?;
    let decode_seconds = decode_clock.elapsed_secs_f64();

    Ok(result(
        format!("deflate (external crate flate2 =1.1.9, level {level})"),
        plaintext.len(),
        &encoded,
        &decoded,
        plaintext,
        encode_seconds,
        decode_seconds,
    ))
}

/// zlib (DEFLATE plus the zlib wrapper/checksum) via `flate2` at `level`
/// (0-9); the same underlying DEFLATE as [`roundtrip_deflate`] but with the
/// zlib envelope, so the two report the wrapper's fixed overhead directly.
pub fn roundtrip_zlib(level: u32, plaintext: &[u8]) -> std::io::Result<ExternalCodecResult> {
    let encode_clock = Stopwatch::start();
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::new(level));
    encoder.write_all(plaintext)?;
    let encoded = encoder.finish()?;
    let encode_seconds = encode_clock.elapsed_secs_f64();

    let decode_clock = Stopwatch::start();
    let mut decoder = flate2::read::ZlibDecoder::new(encoded.as_slice());
    let mut decoded = Vec::new();
    decoder.read_to_end(&mut decoded)?;
    let decode_seconds = decode_clock.elapsed_secs_f64();

    Ok(result(
        format!("zlib (external crate flate2 =1.1.9, level {level})"),
        plaintext.len(),
        &encoded,
        &decoded,
        plaintext,
        encode_seconds,
        decode_seconds,
    ))
}

/// Which BCJ (branch/call/jump) filter, if any, an xz candidate prepends
/// ahead of LZMA2. `None` is a plain LZMA2-in-xz baseline for the same
/// preset. The xz container records which filter (if any) it used, so
/// decoding needs no matching hint -- `XzReader` reads it back from the
/// stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XzBcjFilter {
    None,
    X86,
    Arm64,
}

impl XzBcjFilter {
    pub fn label(self) -> &'static str {
        match self {
            XzBcjFilter::None => "none",
            XzBcjFilter::X86 => "bcj-x86",
            XzBcjFilter::Arm64 => "bcj-arm64",
        }
    }
}

/// xz (LZMA2 in the standard `.xz` container) at `preset` (0-9), optionally
/// preceded by a BCJ filter -- the exact candidate production's own
/// `lzma2_plan`/`SUPPORTED_LZMA2_CONFIGURATIONS` cannot express: production
/// stores raw LZMA2 (no xz container, no filter chain at all) at three fixed
/// (preset, dictionary) pairs. `lzma-rust2` (same `=0.20.0` pin production
/// depends on, `xz` feature) backs both this and every structural BCJ
/// transform in `transform.rs`.
pub fn roundtrip_xz(
    preset: u32,
    filter: XzBcjFilter,
    plaintext: &[u8],
) -> std::io::Result<ExternalCodecResult> {
    let encode_clock = Stopwatch::start();
    let mut options = lzma_rust2::XzOptions::with_preset(preset);
    match filter {
        XzBcjFilter::None => {}
        XzBcjFilter::X86 => options.prepend_pre_filter(lzma_rust2::FilterType::BcjX86, 0),
        XzBcjFilter::Arm64 => options.prepend_pre_filter(lzma_rust2::FilterType::BcjArm64, 0),
    }
    let mut writer = lzma_rust2::XzWriter::new(Vec::new(), options)?;
    writer.write_all(plaintext)?;
    let encoded = writer.finish()?;
    let encode_seconds = encode_clock.elapsed_secs_f64();

    let decode_clock = Stopwatch::start();
    let mut reader = lzma_rust2::XzReader::new(encoded.as_slice(), false);
    let mut decoded = Vec::new();
    reader.read_to_end(&mut decoded)?;
    let decode_seconds = decode_clock.elapsed_secs_f64();

    Ok(result(
        format!(
            "xz (external crate lzma-rust2 =0.20.0, preset {preset}, filter {})",
            filter.label()
        ),
        plaintext.len(),
        &encoded,
        &decoded,
        plaintext,
        encode_seconds,
        decode_seconds,
    ))
}

/// LZ4 frame format via `lz4` (libz4 binding) at `level`. `lz4_flex`
/// (`=0.14.0`, already a production dependency backing
/// `entrybound::codec::lz4_plan`) has no high-compression mode at all --
/// only this binding reaches liblz4's HC levels (it switches to HC
/// internally at level >= 3; 0 is its fast, non-HC default).
pub fn roundtrip_lz4_hc(level: u32, plaintext: &[u8]) -> std::io::Result<ExternalCodecResult> {
    let encode_clock = Stopwatch::start();
    let mut encoder = lz4::EncoderBuilder::new().level(level).build(Vec::new())?;
    encoder.write_all(plaintext)?;
    let (encoded, encode_status) = encoder.finish();
    encode_status?;
    let encode_seconds = encode_clock.elapsed_secs_f64();

    let decode_clock = Stopwatch::start();
    let mut decoder = lz4::Decoder::new(encoded.as_slice())?;
    let mut decoded = Vec::new();
    decoder.read_to_end(&mut decoded)?;
    let decode_seconds = decode_clock.elapsed_secs_f64();

    Ok(result(
        format!("lz4 (external crate =1.28.1, HC level {level})"),
        plaintext.len(),
        &encoded,
        &decoded,
        plaintext,
        encode_seconds,
        decode_seconds,
    ))
}

/// PPMd7 (PPMdH, the variant 7-Zip's `.7z` PPMd uses) via `ppmd-rust`, at
/// `order` (`PPMD7_MIN_ORDER..=PPMD7_MAX_ORDER`, i.e. 2..=64) and `mem_size`
/// bytes (`PPMD7_MIN_MEM_SIZE..=PPMD7_MAX_MEM_SIZE`, i.e. 2048..=1<<29).
/// Production has no PPMd codec at all; see the crate README for why this
/// crate was judged "maintained" and "safe at the call site" over the other
/// PPMd crates surveyed.
pub fn roundtrip_ppmd(
    order: u32,
    mem_size: u32,
    plaintext: &[u8],
) -> std::io::Result<ExternalCodecResult> {
    let encode_clock = Stopwatch::start();
    let mut encoder =
        ppmd_rust::Ppmd7Encoder::new(Vec::new(), order, mem_size).map_err(std::io::Error::other)?;
    encoder.write_all(plaintext)?;
    let encoded = encoder.finish(true)?;
    let encode_seconds = encode_clock.elapsed_secs_f64();

    let decode_clock = Stopwatch::start();
    let mut decoder = ppmd_rust::Ppmd7Decoder::new(encoded.as_slice(), order, mem_size)
        .map_err(std::io::Error::other)?;
    let mut decoded = Vec::new();
    decoder.read_to_end(&mut decoded)?;
    let decode_seconds = decode_clock.elapsed_secs_f64();

    Ok(result(
        format!("ppmd7 (external crate ppmd-rust =1.5.0, order {order}, mem_size {mem_size})"),
        plaintext.len(),
        &encoded,
        &decoded,
        plaintext,
        encode_seconds,
        decode_seconds,
    ))
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
    fn zstd_crate_roundtrip_recovers_the_input() {
        let data = repeating_text(200_000);
        let result = roundtrip_zstd_crate(3, &data).unwrap();
        assert!(result.roundtrip_ok);
        assert!(result.encoded_bytes < result.input_bytes);
    }

    #[test]
    fn zstd_ldm_roundtrip_recovers_the_input() {
        let data = repeating_text(200_000);
        let result = roundtrip_zstd_ldm(3, 24, &data).unwrap();
        assert!(result.roundtrip_ok);
    }

    #[test]
    fn brotli_roundtrip_recovers_the_input_at_every_tested_quality() {
        let data = repeating_text(100_000);
        for quality in [0, 5, 11] {
            let result = roundtrip_brotli(quality, 22, &data).unwrap();
            assert!(result.roundtrip_ok, "quality {quality}");
        }
    }

    #[test]
    fn bzip2_roundtrip_recovers_the_input() {
        let data = repeating_text(100_000);
        for level in [1, 6, 9] {
            let result = roundtrip_bzip2(level, &data).unwrap();
            assert!(result.roundtrip_ok, "level {level}");
        }
    }

    #[test]
    fn deflate_and_zlib_roundtrip_recover_the_input() {
        let data = repeating_text(100_000);
        for level in [0, 6, 9] {
            assert!(roundtrip_deflate(level, &data).unwrap().roundtrip_ok);
            assert!(roundtrip_zlib(level, &data).unwrap().roundtrip_ok);
        }
        // zlib's wrapper (header + Adler-32) costs a small, fixed number of
        // extra bytes over raw DEFLATE for the same content and level.
        let raw = roundtrip_deflate(6, &data).unwrap();
        let wrapped = roundtrip_zlib(6, &data).unwrap();
        assert!(wrapped.encoded_bytes > raw.encoded_bytes);
    }

    #[test]
    fn xz_roundtrip_recovers_the_input_with_and_without_bcj() {
        let data = repeating_text(100_000);
        for filter in [XzBcjFilter::None, XzBcjFilter::X86, XzBcjFilter::Arm64] {
            let result = roundtrip_xz(6, filter, &data).unwrap();
            assert!(result.roundtrip_ok, "{filter:?}");
        }
    }

    #[test]
    fn lz4_hc_roundtrip_recovers_the_input() {
        let data = repeating_text(100_000);
        for level in [0, 3, 9, 12] {
            let result = roundtrip_lz4_hc(level, &data).unwrap();
            assert!(result.roundtrip_ok, "level {level}");
        }
    }

    #[test]
    fn ppmd_roundtrip_recovers_the_input() {
        let data = repeating_text(100_000);
        let result = roundtrip_ppmd(6, 1 << 20, &data).unwrap();
        assert!(result.roundtrip_ok);
    }
}
