//! Candidate external codecs, reached directly rather than through
//! `entrybound::eam::TransformPlan`.
//!
//! Two kinds live here:
//!
//! - **Parameters production's own codecs cannot reach.** Production's
//!   `zstd` path (`entrybound::codec::zstd_plan`) fixes a 1 MiB window
//!   (`ZSTD_WINDOW_LOG = 20`) and never enables long-distance matching.
//!   Calling the `zstd` crate directly reaches both
//!   ([`roundtrip_zstd_window`], [`roundtrip_zstd_ldm`]).
//! - **Codecs production has never used at all**: brotli, bzip2, DEFLATE and
//!   zlib, xz with BCJ filters, LZ4 HC levels, and PPMd7.
//!
//! # Fairness and resource reporting (harness review round 1, finding R1-12)
//!
//! A size comparison between codecs is only interpretable next to the
//! resources each configuration assumes, so every [`ExternalCodecResult`]
//! (and `crate::production::RoundtripResult`) carries:
//!
//! - `window_bytes`: the match window / dictionary / block / model memory the
//!   configuration uses, which bounds what the *decoder* must hold. For the
//!   zstd crate's default frame settings it is parsed from the encoded
//!   frame's own header ([`zstd_frame_window_bytes`]), not assumed.
//! - `threads`: always `1`. No codec here is built or configured with worker
//!   threads (the `zstd` crate's `zstdmt` feature is off, `XzWriter` is the
//!   single-threaded writer, brotli/bzip2/flate2/lz4/ppmd are
//!   single-threaded), so no candidate gets parallelism the others lack.
//! - `container`: what framing the encoded size includes (for example the
//!   xz container's headers, index, and CRC64; the lz4 frame's content
//!   checksum), since production payloads carry no container of their own.
//! - `encode_memory` / `decode_memory`: phase-scoped resident-memory growth
//!   ([`ebr_common::measure::ScopedMemory`]). Each includes that phase's
//!   output buffer (roughly the encoded or the plaintext size), and can be
//!   understated by allocator reuse inside one process; run one candidate per
//!   process (`ebr-codec object-matrix --candidate <label>`) for a clean
//!   per-candidate memory figure.
//!
//! Configurations follow each tool's own defaults where one exists: brotli's
//! command-line default window (`lgwin` 24) is included next to the library
//! default (22); lz4 uses the `lz4` command line's 4 MiB blocks; PPMd uses
//! 7-Zip's level-to-(order, memory) mapping including 7-Zip's reduction of
//! the model size for small inputs ([`ppmd_7zip_parameters`]). Earlier
//! revisions ran PPMd only at order 6 with a 1 MiB model, which forces model
//! restarts on anything but small inputs and is not a configuration any
//! PPMd tool ships.
//!
//! `encode_seconds`/`decode_seconds` are the same kind of non-decision-grade
//! smoke timing as `production::RoundtripResult`'s.

use ebr_common::measure::{ScopedMemory, measure_scoped};
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
    pub window_bytes: Option<u64>,
    pub threads: u32,
    pub container: &'static str,
    pub encode_memory: ScopedMemory,
    pub decode_memory: ScopedMemory,
}

/// Runs `f` in a memory scope and a monotonic stopwatch.
fn timed<T>(f: impl FnOnce() -> std::io::Result<T>) -> std::io::Result<(T, f64, ScopedMemory)> {
    let (value, memory, seconds) = measure_scoped(f);
    value.map(|value| (value, seconds, memory))
}

struct Outcome {
    codec_id: String,
    encoded: Vec<u8>,
    decoded: Vec<u8>,
    encode: (f64, ScopedMemory),
    decode: (f64, ScopedMemory),
    window_bytes: Option<u64>,
    container: &'static str,
}

fn result(outcome: Outcome, original: &[u8]) -> ExternalCodecResult {
    ExternalCodecResult {
        codec_id: outcome.codec_id,
        input_bytes: original.len() as u64,
        encoded_bytes: outcome.encoded.len() as u64,
        roundtrip_ok: outcome.decoded == original,
        encode_seconds: outcome.encode.0,
        decode_seconds: outcome.decode.0,
        window_bytes: outcome.window_bytes,
        threads: 1,
        container: outcome.container,
        encode_memory: outcome.encode.1,
        decode_memory: outcome.decode.1,
    }
}

/// The window size a Zstandard frame header declares (RFC 8878 section
/// 3.1.1.1.2): the `Window_Descriptor` when present, otherwise the
/// single-segment frame's content size. `None` for a malformed header.
#[must_use]
pub fn zstd_frame_window_bytes(frame: &[u8]) -> Option<u64> {
    if frame.len() < 6 || frame[..4] != [0x28, 0xB5, 0x2F, 0xFD] {
        return None;
    }
    let descriptor = frame[4];
    let single_segment = descriptor & 0x20 != 0;
    if !single_segment {
        let window = frame[5];
        let exponent = u32::from(window >> 3);
        let mantissa = u64::from(window & 0x07);
        let base = 1u64.checked_shl(exponent + 10)?;
        return Some(base + (base / 8) * mantissa);
    }
    let dictionary_id_bytes = match descriptor & 0x03 {
        0 => 0,
        1 => 1,
        2 => 2,
        _ => 4,
    };
    let size_bytes = match descriptor >> 6 {
        0 => 1,
        1 => 2,
        2 => 4,
        _ => 8,
    };
    let start = 5 + dictionary_id_bytes;
    let field = frame.get(start..start + size_bytes)?;
    let mut value = 0u64;
    for (index, byte) in field.iter().enumerate() {
        value |= u64::from(*byte) << (8 * index);
    }
    if size_bytes == 2 {
        value += 256;
    }
    Some(value)
}

/// Plain `zstd` crate encode/decode at `level`, default frame settings (no
/// long-distance matching, the level's own window for an unknown-size stream,
/// single-threaded). The window actually used is read back from the frame.
pub fn roundtrip_zstd_crate(level: i32, plaintext: &[u8]) -> std::io::Result<ExternalCodecResult> {
    let (encoded, encode_seconds, encode_memory) =
        timed(|| zstd::stream::encode_all(plaintext, level))?;
    let (decoded, decode_seconds, decode_memory) =
        timed(|| zstd::stream::decode_all(encoded.as_slice()))?;
    Ok(result(
        Outcome {
            codec_id: format!(
                "zstd (external crate =0.13.3, level {level}, default frame settings)"
            ),
            window_bytes: zstd_frame_window_bytes(&encoded),
            encoded,
            decoded,
            encode: (encode_seconds, encode_memory),
            decode: (decode_seconds, decode_memory),
            container: "zstd frame (no checksum)",
        },
        plaintext,
    ))
}

fn roundtrip_zstd_with(
    level: i32,
    window_log: u32,
    long_distance_matching: bool,
    plaintext: &[u8],
) -> std::io::Result<ExternalCodecResult> {
    let (encoded, encode_seconds, encode_memory) = timed(|| {
        let mut compressor = zstd::bulk::Compressor::new(level)?;
        compressor.window_log(window_log)?;
        compressor.long_distance_matching(long_distance_matching)?;
        compressor.include_contentsize(true)?;
        compressor.compress(plaintext)
    })?;
    let (decoded, decode_seconds, decode_memory) = timed(|| {
        let mut decoder = zstd::stream::read::Decoder::new(encoded.as_slice())?;
        decoder.window_log_max(window_log.max(27))?;
        let mut decoded = Vec::new();
        decoder.read_to_end(&mut decoded)?;
        Ok(decoded)
    })?;
    Ok(result(
        Outcome {
            codec_id: format!(
                "zstd (external crate =0.13.3, level {level}, LDM {}, window_log {window_log})",
                if long_distance_matching { "on" } else { "off" }
            ),
            window_bytes: zstd_frame_window_bytes(&encoded),
            encoded,
            decoded,
            encode: (encode_seconds, encode_memory),
            decode: (decode_seconds, decode_memory),
            container: "zstd frame (no checksum)",
        },
        plaintext,
    ))
}

/// `zstd` with an explicit window log and long-distance matching **off**:
/// the window-matched control for [`roundtrip_zstd_ldm`], so a size gain can
/// be attributed to LDM rather than to a window larger than production's.
pub fn roundtrip_zstd_window(
    level: i32,
    window_log: u32,
    plaintext: &[u8],
) -> std::io::Result<ExternalCodecResult> {
    roundtrip_zstd_with(level, window_log, false, plaintext)
}

/// `zstd` with long-distance matching enabled and an explicit window log --
/// the two parameters `entrybound::codec::zstd_plan` never sets.
pub fn roundtrip_zstd_ldm(
    level: i32,
    window_log: u32,
    plaintext: &[u8],
) -> std::io::Result<ExternalCodecResult> {
    roundtrip_zstd_with(level, window_log, true, plaintext)
}

/// Pure-Rust `brotli` at `quality` (0-11) and `lgwin` (window bits, 10-24).
pub fn roundtrip_brotli(
    quality: u32,
    lgwin: u32,
    plaintext: &[u8],
) -> std::io::Result<ExternalCodecResult> {
    let (encoded, encode_seconds, encode_memory) = timed(|| {
        let mut writer = brotli::CompressorWriter::new(Vec::new(), 4096, quality, lgwin);
        writer.write_all(plaintext)?;
        writer.flush()?;
        Ok(writer.into_inner())
    })?;
    let (decoded, decode_seconds, decode_memory) = timed(|| {
        let mut reader = brotli::Decompressor::new(encoded.as_slice(), 4096);
        let mut decoded = Vec::new();
        reader.read_to_end(&mut decoded)?;
        Ok(decoded)
    })?;
    Ok(result(
        Outcome {
            codec_id: format!("brotli (external crate =9.0.0, quality {quality}, lgwin {lgwin})"),
            // RFC 7932: the sliding window is (1 << WBITS) - 16 bytes.
            window_bytes: Some((1u64 << lgwin) - 16),
            encoded,
            decoded,
            encode: (encode_seconds, encode_memory),
            decode: (decode_seconds, decode_memory),
            container: "raw brotli stream",
        },
        plaintext,
    ))
}

/// `bzip2` at `level` (1-9; block size `level * 100 kB`).
pub fn roundtrip_bzip2(level: u32, plaintext: &[u8]) -> std::io::Result<ExternalCodecResult> {
    let (encoded, encode_seconds, encode_memory) = timed(|| {
        let mut encoder = bzip2::write::BzEncoder::new(Vec::new(), bzip2::Compression::new(level));
        encoder.write_all(plaintext)?;
        encoder.finish()
    })?;
    let (decoded, decode_seconds, decode_memory) = timed(|| {
        let mut decoder = bzip2::read::BzDecoder::new(encoded.as_slice());
        let mut decoded = Vec::new();
        decoder.read_to_end(&mut decoded)?;
        Ok(decoded)
    })?;
    Ok(result(
        Outcome {
            codec_id: format!("bzip2 (external crate =0.6.1, level {level})"),
            window_bytes: Some(u64::from(level) * 100_000),
            encoded,
            decoded,
            encode: (encode_seconds, encode_memory),
            decode: (decode_seconds, decode_memory),
            container: "bzip2 stream (block and stream CRC32)",
        },
        plaintext,
    ))
}

/// Raw DEFLATE (no zlib/gzip wrapper) via `flate2` at `level` (0-9). The
/// `flate2` backend here is `miniz_oxide`, the same backend a production CLI
/// release build links; it is not zlib, zlib-ng, or libdeflate, whose ratios
/// at high levels differ.
pub fn roundtrip_deflate(level: u32, plaintext: &[u8]) -> std::io::Result<ExternalCodecResult> {
    let (encoded, encode_seconds, encode_memory) = timed(|| {
        let mut encoder =
            flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::new(level));
        encoder.write_all(plaintext)?;
        encoder.finish()
    })?;
    let (decoded, decode_seconds, decode_memory) = timed(|| {
        let mut decoder = flate2::read::DeflateDecoder::new(encoded.as_slice());
        let mut decoded = Vec::new();
        decoder.read_to_end(&mut decoded)?;
        Ok(decoded)
    })?;
    Ok(result(
        Outcome {
            codec_id: format!("deflate (external crate flate2 =1.1.9 miniz_oxide, level {level})"),
            window_bytes: Some(32_768),
            encoded,
            decoded,
            encode: (encode_seconds, encode_memory),
            decode: (decode_seconds, decode_memory),
            container: "raw DEFLATE",
        },
        plaintext,
    ))
}

/// zlib (DEFLATE plus the zlib wrapper/checksum) via `flate2` at `level`
/// (0-9); the same underlying DEFLATE as [`roundtrip_deflate`].
pub fn roundtrip_zlib(level: u32, plaintext: &[u8]) -> std::io::Result<ExternalCodecResult> {
    let (encoded, encode_seconds, encode_memory) = timed(|| {
        let mut encoder =
            flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::new(level));
        encoder.write_all(plaintext)?;
        encoder.finish()
    })?;
    let (decoded, decode_seconds, decode_memory) = timed(|| {
        let mut decoder = flate2::read::ZlibDecoder::new(encoded.as_slice());
        let mut decoded = Vec::new();
        decoder.read_to_end(&mut decoded)?;
        Ok(decoded)
    })?;
    Ok(result(
        Outcome {
            codec_id: format!("zlib (external crate flate2 =1.1.9 miniz_oxide, level {level})"),
            window_bytes: Some(32_768),
            encoded,
            decoded,
            encode: (encode_seconds, encode_memory),
            decode: (decode_seconds, decode_memory),
            container: "zlib wrapper (2-byte header, Adler-32)",
        },
        plaintext,
    ))
}

/// Which BCJ (branch/call/jump) filter, if any, an xz candidate prepends
/// ahead of LZMA2. `None` is a plain LZMA2-in-xz baseline for the same
/// preset. The xz container records which filter (if any) it used, so
/// decoding needs no matching hint.
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
/// preceded by a BCJ filter. Note that xz presets carry their own dictionary
/// sizes (preset 9 is 64 MiB), unlike production's three fixed
/// `(preset, dictionary)` pairs (at most 8 MiB); `window_bytes` records it.
pub fn roundtrip_xz(
    preset: u32,
    filter: XzBcjFilter,
    plaintext: &[u8],
) -> std::io::Result<ExternalCodecResult> {
    let mut options = lzma_rust2::XzOptions::with_preset(preset);
    match filter {
        XzBcjFilter::None => {}
        XzBcjFilter::X86 => options.prepend_pre_filter(lzma_rust2::FilterType::BcjX86, 0),
        XzBcjFilter::Arm64 => options.prepend_pre_filter(lzma_rust2::FilterType::BcjArm64, 0),
    }
    let dictionary_bytes = u64::from(options.lzma_options.dict_size);
    let (encoded, encode_seconds, encode_memory) = timed(|| {
        let mut writer = lzma_rust2::XzWriter::new(Vec::new(), options)?;
        writer.write_all(plaintext)?;
        writer.finish()
    })?;
    let (decoded, decode_seconds, decode_memory) = timed(|| {
        let mut reader = lzma_rust2::XzReader::new(encoded.as_slice(), false);
        let mut decoded = Vec::new();
        reader.read_to_end(&mut decoded)?;
        Ok(decoded)
    })?;
    Ok(result(
        Outcome {
            codec_id: format!(
                "xz (external crate lzma-rust2 =0.20.0, preset {preset}, filter {})",
                filter.label()
            ),
            window_bytes: Some(dictionary_bytes),
            encoded,
            decoded,
            encode: (encode_seconds, encode_memory),
            decode: (decode_seconds, decode_memory),
            container: "xz container (stream/block headers, index, CRC64)",
        },
        plaintext,
    ))
}

/// LZ4 frame format via `lz4` (liblz4 binding) at `level`, with the `lz4`
/// command line's defaults: 4 MiB blocks, content checksum on, block
/// checksums off. liblz4 switches to HC internally at level >= 3.
pub fn roundtrip_lz4_hc(level: u32, plaintext: &[u8]) -> std::io::Result<ExternalCodecResult> {
    let (encoded, encode_seconds, encode_memory) = timed(|| {
        let mut encoder = lz4::EncoderBuilder::new()
            .level(level)
            .block_size(lz4::BlockSize::Max4MB)
            .block_checksum(lz4::liblz4::BlockChecksum::NoBlockChecksum)
            .build(Vec::new())?;
        encoder.write_all(plaintext)?;
        let (encoded, status) = encoder.finish();
        status.map(|()| encoded)
    })?;
    let (decoded, decode_seconds, decode_memory) = timed(|| {
        let mut decoder = lz4::Decoder::new(encoded.as_slice())?;
        let mut decoded = Vec::new();
        decoder.read_to_end(&mut decoded)?;
        Ok(decoded)
    })?;
    Ok(result(
        Outcome {
            codec_id: format!("lz4 (external crate =1.28.1, level {level}, 4 MiB blocks)"),
            // The LZ4 format's match distance limit.
            window_bytes: Some(65_536),
            encoded,
            decoded,
            encode: (encode_seconds, encode_memory),
            decode: (decode_seconds, decode_memory),
            container: "lz4 frame (4 MiB linked blocks, content checksum)",
        },
        plaintext,
    ))
}

/// 7-Zip's PPMd level mapping (`PpmdEncoder.cpp` `CEncProps::Normalize`):
/// model size `1 << (level + 19)`, order from `{3, 4, 4, 5, 5, 6, 8, 16, 24,
/// 32}`, and -- like 7-Zip -- the model size reduced to the smallest power of
/// two (at least 64 KiB) that is at least 16 times the input size when that
/// is smaller. Returns `(order, mem_size)`.
#[must_use]
pub fn ppmd_7zip_parameters(level: u32, input_len: u64) -> (u32, u32) {
    const ORDERS: [u32; 10] = [3, 4, 4, 5, 5, 6, 8, 16, 24, 32];
    let level = level.min(9);
    let mut mem_size = 1u32 << (level + 19);
    if u64::from(mem_size / 16) > input_len {
        for shift in 16..=31u32 {
            let candidate = 1u64 << shift;
            if input_len <= candidate / 16 {
                if u64::from(mem_size) > candidate {
                    mem_size = candidate as u32;
                }
                break;
            }
        }
    }
    (ORDERS[level as usize], mem_size)
}

/// PPMd7 (PPMdH, the variant 7-Zip's `.7z` PPMd uses) via `ppmd-rust`, at
/// `order` and `mem_size` bytes. See [`ppmd_7zip_parameters`] for the
/// configurations the matrix uses.
pub fn roundtrip_ppmd(
    order: u32,
    mem_size: u32,
    plaintext: &[u8],
) -> std::io::Result<ExternalCodecResult> {
    let (encoded, encode_seconds, encode_memory) = timed(|| {
        let mut encoder = ppmd_rust::Ppmd7Encoder::new(Vec::new(), order, mem_size)
            .map_err(std::io::Error::other)?;
        encoder.write_all(plaintext)?;
        encoder.finish(true)
    })?;
    let (decoded, decode_seconds, decode_memory) = timed(|| {
        let mut decoder = ppmd_rust::Ppmd7Decoder::new(encoded.as_slice(), order, mem_size)
            .map_err(std::io::Error::other)?;
        let mut decoded = Vec::new();
        decoder.read_to_end(&mut decoded)?;
        Ok(decoded)
    })?;
    Ok(result(
        Outcome {
            codec_id: format!(
                "ppmd7 (external crate ppmd-rust =1.5.0, order {order}, mem_size {mem_size})"
            ),
            window_bytes: Some(u64::from(mem_size)),
            encoded,
            decoded,
            encode: (encode_seconds, encode_memory),
            decode: (decode_seconds, decode_memory),
            container: "raw PPMd7 range-coded stream with end marker",
        },
        plaintext,
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

    #[test]
    fn zstd_frame_window_is_parsed_from_the_header() {
        let data = repeating_text(4_000_000);
        // A frame that records its content size and fits the window is a
        // single-segment frame: the decoder window is the content size.
        let ldm = roundtrip_zstd_ldm(3, 24, &data).unwrap();
        assert_eq!(ldm.window_bytes, Some(data.len() as u64));
        let control = roundtrip_zstd_window(3, 24, &data).unwrap();
        assert!(control.roundtrip_ok);
        assert_eq!(control.window_bytes, Some(data.len() as u64));
        let small_window = roundtrip_zstd_window(3, 20, &data).unwrap();
        assert_eq!(small_window.window_bytes, Some(1 << 20));
        let default = roundtrip_zstd_crate(19, &data).unwrap();
        assert!(default.window_bytes.unwrap() >= 1 << 20);
        assert_eq!(default.threads, 1);
    }

    #[test]
    fn ppmd_7zip_mapping_matches_7zip_levels_and_reduces_for_small_inputs() {
        assert_eq!(ppmd_7zip_parameters(5, u64::MAX), (6, 16 << 20));
        assert_eq!(ppmd_7zip_parameters(7, u64::MAX), (16, 64 << 20));
        assert_eq!(ppmd_7zip_parameters(9, u64::MAX), (32, 256 << 20));
        // 100 kB input: the smallest power of two >= 16 x 100 kB is 2 MiB.
        assert_eq!(ppmd_7zip_parameters(9, 100_000), (32, 2 << 20));
        // Tiny input clamps at 64 KiB.
        assert_eq!(ppmd_7zip_parameters(5, 10), (6, 1 << 16));
    }

    #[test]
    fn every_result_reports_window_threads_and_container() {
        let data = repeating_text(50_000);
        for result in [
            roundtrip_brotli(11, 24, &data).unwrap(),
            roundtrip_bzip2(9, &data).unwrap(),
            roundtrip_deflate(9, &data).unwrap(),
            roundtrip_xz(9, XzBcjFilter::None, &data).unwrap(),
            roundtrip_lz4_hc(12, &data).unwrap(),
            roundtrip_ppmd(6, 1 << 20, &data).unwrap(),
        ] {
            assert!(result.roundtrip_ok, "{}", result.codec_id);
            assert!(result.window_bytes.is_some(), "{}", result.codec_id);
            assert_eq!(result.threads, 1);
            assert!(!result.container.is_empty());
        }
        assert_eq!(
            roundtrip_xz(9, XzBcjFilter::None, &data)
                .unwrap()
                .window_bytes,
            Some(64 << 20)
        );
    }
}
