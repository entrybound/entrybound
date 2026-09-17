//! Candidate external codecs, reached directly rather than through
//! `entrybound::eam::TransformPlan`.
//!
//! Production's `zstd` path (`entrybound::codec::zstd_plan`) fixes several
//! parameters that this crate's own `Cargo.toml` pins to the exact same
//! `zstd = "=0.13.3"` -- but calling the `zstd` crate directly, instead of
//! through a `TransformPlan`, reaches encoder parameters `TransformPlan`
//! does not model at all (long-distance matching, an explicit window log
//! decoupled from the level, multithreaded encoding, ...). That is the
//! actual research question this module exists to ask: does something
//! outside what `TransformPlan` can express win enough to be worth adding to
//! it? This module does not yet answer that; it wires up one directly
//! comparable baseline (plain `zstd` at a level, default frame settings) so
//! a later experiment has somewhere to add those extra parameters.

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ExternalCodecResult {
    pub codec_id: String,
    pub input_bytes: u64,
    pub encoded_bytes: u64,
    pub roundtrip_ok: bool,
}

/// Plain `zstd` crate encode/decode at `level`, default frame settings (no
/// long-distance matching, no explicit window log, single-threaded). Not the
/// same call path as `entrybound::codec::zstd_plan` +
/// `encode_payload`/`decode_payload`, even though both ultimately call into
/// the same underlying `libzstd`.
pub fn roundtrip_zstd_crate(level: i32, plaintext: &[u8]) -> std::io::Result<ExternalCodecResult> {
    let encoded = zstd::stream::encode_all(plaintext, level)?;
    let decoded = zstd::stream::decode_all(encoded.as_slice())?;
    Ok(ExternalCodecResult {
        codec_id: format!("zstd (external crate =0.13.3, level {level}, default frame settings)"),
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
    fn zstd_crate_roundtrip_recovers_the_input() {
        let data = repeating_text(200_000);
        let result = roundtrip_zstd_crate(3, &data).unwrap();
        assert!(result.roundtrip_ok);
        assert!(result.encoded_bytes < result.input_bytes);
    }
}
