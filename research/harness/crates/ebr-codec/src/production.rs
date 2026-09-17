//! Production codec paths, reached through `entrybound::research::codec`
//! (the default-off `research-internals` feature; see
//! `research/harness/research-internals.md`). Every function here is a thin
//! wrapper: it builds a `TransformPlan` the same way production's planner
//! does, then round-trips through the exact `encode_payload`/`decode_payload`
//! production uses. Nothing here changes production behavior or archive
//! bytes.

use entrybound::diagnostics::Diagnostic;
use entrybound::research::codec;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct RoundtripResult {
    pub codec_id: String,
    pub input_bytes: u64,
    pub encoded_bytes: u64,
    pub roundtrip_ok: bool,
}

/// Plans, encodes, and decodes `plaintext` through production's `zstd` path
/// at `level` (one of `codec::SUPPORTED_LEVELS`), with no dictionary or
/// prefix (an independent chunk).
pub fn roundtrip_zstd(level: i32, plaintext: &[u8]) -> Result<RoundtripResult, Diagnostic> {
    let plan = codec::zstd_plan(level)?;
    codec::validate_plan(&plan)?;
    let encoded = codec::encode_payload(&plan, plaintext)?;
    let decoded = codec::decode_payload(&plan, &encoded, plaintext.len() as u64)?;
    Ok(RoundtripResult {
        codec_id: format!(
            "{} (production, level {level})",
            codec::ZSTD_CODEC_IDENTIFIER
        ),
        input_bytes: plaintext.len() as u64,
        encoded_bytes: encoded.len() as u64,
        roundtrip_ok: decoded == plaintext,
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
    let encoded = codec::encode_payload(&plan, plaintext)?;
    let decoded = codec::decode_payload(&plan, &encoded, plaintext.len() as u64)?;
    Ok(RoundtripResult {
        codec_id: format!(
            "{} (production, preset {preset}, dict {dictionary_bytes})",
            codec::LZMA2_CODEC_IDENTIFIER
        ),
        input_bytes: plaintext.len() as u64,
        encoded_bytes: encoded.len() as u64,
        roundtrip_ok: decoded == plaintext,
    })
}

/// Plans, encodes, and decodes `plaintext` through production's `lz4` path.
pub fn roundtrip_lz4(plaintext: &[u8]) -> Result<RoundtripResult, Diagnostic> {
    let plan = codec::lz4_plan(Box::new([]))?;
    codec::validate_plan(&plan)?;
    let encoded = codec::encode_payload(&plan, plaintext)?;
    let decoded = codec::decode_payload(&plan, &encoded, plaintext.len() as u64)?;
    Ok(RoundtripResult {
        codec_id: format!("{} (production)", codec::LZ4_CODEC_IDENTIFIER),
        input_bytes: plaintext.len() as u64,
        encoded_bytes: encoded.len() as u64,
        roundtrip_ok: decoded == plaintext,
    })
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
    fn zstd_roundtrip_recovers_the_input_and_compresses_repetitive_text() {
        let data = repeating_text(200_000);
        let result = roundtrip_zstd(3, &data).unwrap();
        assert!(result.roundtrip_ok);
        assert!(
            result.encoded_bytes < result.input_bytes,
            "expected zstd to shrink highly repetitive text"
        );
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
}
