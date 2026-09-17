//! Codec/transform/reconstruction research: production internals via
//! `entrybound`'s `research-internals` feature ([`production`]), plus
//! candidate external codecs not shaped by `TransformPlan`'s model
//! ([`external`]), compared on the same input. See `README.md`.

pub mod external;
pub mod matrix;
pub mod production;
pub mod transform;

use serde::Serialize;

/// Both a production codec path and an external candidate over the same
/// bytes, for a side-by-side comparison.
#[derive(Debug, Clone, Serialize)]
pub struct ZstdComparison {
    pub production: production::RoundtripResult,
    pub external: external::ExternalCodecResult,
}

/// Runs production's `zstd` `TransformPlan` path and the plain `zstd` crate
/// at the same `level` over the same `plaintext`.
pub fn compare_zstd_level(
    level: i32,
    plaintext: &[u8],
) -> Result<ZstdComparison, Box<dyn std::error::Error>> {
    let production = production::roundtrip_zstd(level, plaintext).map_err(|d| d.to_string())?;
    let external = external::roundtrip_zstd_crate(level, plaintext)?;
    Ok(ZstdComparison {
        production,
        external,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_production_and_external_zstd_on_the_same_input() {
        let data: Vec<u8> = b"entrybound research harness codec comparison; "
            .iter()
            .cycle()
            .take(100_000)
            .copied()
            .collect();
        let comparison = compare_zstd_level(3, &data).unwrap();
        assert!(comparison.production.roundtrip_ok);
        assert!(comparison.external.roundtrip_ok);
        assert_eq!(comparison.production.input_bytes, data.len() as u64);
        assert_eq!(comparison.external.input_bytes, data.len() as u64);
    }
}
