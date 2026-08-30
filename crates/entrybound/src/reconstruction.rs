//! Bounded bit-exact reconstruction of complete DEFLATE representations.

use preflate_rs::{PreflateConfig, preflate_whole_deflate_stream, recreate_whole_deflate_stream};

use crate::diagnostics::{Diagnostic, OutcomeClass, ReasonCode, Result};
use crate::eam::ReconstructionData;
use crate::identity::sha256_exact;

pub(crate) const DEFLATE_RECONSTRUCT_ID: &str = "deflate-reconstruct/v1";
pub(crate) const DEFLATE_RECONSTRUCTION_FORMAT: &str = "preflate-rs-0.7.6/deflate-reconstruct-v1";
pub(crate) const MAX_RECONSTRUCTION_INPUT_BYTES: usize = 16 * 1024 * 1024;
pub(crate) const MAX_RECONSTRUCTION_DATA_BYTES: usize = 16 * 1024 * 1024;
pub(crate) const MAX_INTERMEDIATE_BYTES: usize = 64 * 1024 * 1024;
pub(crate) const RECONSTRUCTION_WORKING_SET_BYTES: u64 = 80 * 1024 * 1024;
pub(crate) const MAX_EXPANSION_RATIO: usize = 64;
const SIDE_MAGIC: [u8; 4] = *b"ERD1";
const SIDE_HEADER_LEN: usize = 24;

#[derive(Clone, Debug)]
pub(crate) struct ReconstructCandidate {
    pub intermediate: Vec<u8>,
    pub data: ReconstructionData,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Wrapper {
    Raw = 0,
    Zlib = 1,
    Gzip = 2,
}

pub(crate) fn parameters(max_chain_length: u32) -> Box<[u8]> {
    let mut value = Vec::with_capacity(8);
    value.extend_from_slice(b"DR01");
    value.extend_from_slice(&max_chain_length.to_be_bytes());
    value.into_boxed_slice()
}

pub(crate) fn validate_parameters(value: &[u8]) -> Result<u32> {
    if value.len() != 8 || value[..4] != *b"DR01" {
        return Err(invalid(
            "deflate-reconstruct/v1 parameters are not DR01 + max-chain",
        ));
    }
    let max_chain = u32::from_be_bytes(
        value[4..8]
            .try_into()
            .map_err(|_| invalid("deflate-reconstruct/v1 max-chain parameter is malformed"))?,
    );
    if !matches!(max_chain, 512 | 2_048 | 4_096) {
        return Err(invalid(
            "deflate-reconstruct/v1 max-chain must be 512, 2048, or 4096",
        ));
    }
    Ok(max_chain)
}

/// Attempts one complete, bounded representation. `Ok(None)` is ordinary
/// ineligibility; malformed apparent wrappers are not accepted as raw data.
pub(crate) fn try_forward(original: &[u8], max_chain: u32) -> Result<Option<ReconstructCandidate>> {
    validate_parameters(&parameters(max_chain))?;
    if original.is_empty() || original.len() > MAX_RECONSTRUCTION_INPUT_BYTES {
        return Ok(None);
    }
    let Some((wrapper, prefix_len, suffix_len)) = recognize(original).unwrap_or(None) else {
        return Ok(None);
    };
    let body_end = original
        .len()
        .checked_sub(suffix_len)
        .ok_or_else(|| failed("wrapper length underflow"))?;
    let raw = &original[prefix_len..body_end];
    let config = PreflateConfig {
        max_chain_length: max_chain,
        plain_text_limit: MAX_INTERMEDIATE_BYTES,
        verify_compression: true,
    };
    let Ok((result, plaintext)) = preflate_whole_deflate_stream(raw, &config) else {
        return Ok(None);
    };
    if result.compressed_size != raw.len() {
        return Ok(None);
    }
    let intermediate = plaintext.text().to_vec();
    if intermediate.len() > MAX_INTERMEDIATE_BYTES
        || intermediate.len() > original.len().saturating_mul(MAX_EXPANSION_RATIO).max(1)
    {
        return Ok(None);
    }
    if !wrapper_checksum_valid(wrapper, original, prefix_len, suffix_len, &intermediate) {
        return Ok(None);
    }
    let bytes = encode_side_data(
        wrapper,
        &original[..prefix_len],
        &result.corrections,
        &original[body_end..],
    )?;
    if bytes.len() > MAX_RECONSTRUCTION_DATA_BYTES {
        return Ok(None);
    }
    let data = ReconstructionData {
        reconstruction_id: sha256_exact(&bytes),
        format: DEFLATE_RECONSTRUCTION_FORMAT.to_owned(),
        intermediate_len: u64::try_from(intermediate.len())
            .map_err(|_| failed("DEFLATE intermediate length exceeds u64"))?,
        bytes: bytes.into_boxed_slice(),
    };
    let Ok(recreated) = inverse(&intermediate, &data) else {
        return Ok(None);
    };
    // Mandatory dual check: the digest check is not substituted for equality.
    if sha256_exact(original) != sha256_exact(&recreated) || original != recreated {
        return Ok(None);
    }
    Ok(Some(ReconstructCandidate { intermediate, data }))
}

/// Re-runs forward recognition and exact verification against the recorded
/// side object. Used by the native writer so unverified caller data cannot be
/// committed as a reconstructive representation.
pub(crate) fn verified_forward(
    original: &[u8],
    max_chain: u32,
    expected: &ReconstructionData,
) -> Result<Vec<u8>> {
    validate_data(expected)?;
    let candidate = try_forward(original, max_chain)?.ok_or_else(|| {
        failed("recorded reconstructive plan is not valid for the original Chunk")
    })?;
    if candidate.data != *expected {
        return Err(Diagnostic::new(
            OutcomeClass::Corrupt,
            ReasonCode::ReconstructionDataDigestMismatch,
            "recorded ReconstructionData differs from verified forward output",
        ));
    }
    let recreated = inverse(&candidate.intermediate, expected)?;
    if sha256_exact(original) != sha256_exact(&recreated) || original != recreated {
        return Err(Diagnostic::new(
            OutcomeClass::Corrupt,
            ReasonCode::ReconstructedDigestMismatch,
            "writer round-trip verification did not recreate the exact original bytes",
        ));
    }
    Ok(candidate.intermediate)
}

pub(crate) fn inverse(intermediate: &[u8], data: &ReconstructionData) -> Result<Vec<u8>> {
    validate_data(data)?;
    if u64::try_from(intermediate.len()).unwrap_or(u64::MAX) != data.intermediate_len {
        return Err(Diagnostic::new(
            OutcomeClass::Corrupt,
            ReasonCode::ReconstructedLengthMismatch,
            "reconstructive intermediate length differs from ReconstructionData declaration",
        ));
    }
    let side = decode_side_data(&data.bytes)?;
    let raw = recreate_whole_deflate_stream(intermediate, side.corrections)
        .map_err(|error| failed(format!("preflate recreation failed: {error}")))?;
    let capacity = side
        .prefix
        .len()
        .checked_add(raw.len())
        .and_then(|v| v.checked_add(side.suffix.len()))
        .ok_or_else(|| failed("reconstructed output length overflow"))?;
    if capacity > MAX_RECONSTRUCTION_INPUT_BYTES {
        return Err(failed("reconstructed output exceeds v1 limit"));
    }
    let mut output = Vec::with_capacity(capacity);
    output.extend_from_slice(side.prefix);
    output.extend_from_slice(&raw);
    output.extend_from_slice(side.suffix);
    if !wrapper_checksum_valid(
        side.wrapper,
        &output,
        side.prefix.len(),
        side.suffix.len(),
        intermediate,
    ) {
        return Err(failed(
            "reconstructed wrapper checksum does not match its plaintext",
        ));
    }
    Ok(output)
}

pub(crate) fn validate_data(data: &ReconstructionData) -> Result<()> {
    if data.format != DEFLATE_RECONSTRUCTION_FORMAT {
        return Err(Diagnostic::new(
            OutcomeClass::Unsupported,
            ReasonCode::UnsupportedReconstructionFormat,
            format!("unsupported reconstruction format '{}'", data.format),
        ));
    }
    if data.bytes.len() > MAX_RECONSTRUCTION_DATA_BYTES
        || data.intermediate_len > MAX_INTERMEDIATE_BYTES as u64
    {
        return Err(Diagnostic::new(
            OutcomeClass::PolicyRefused,
            ReasonCode::ResourceLimit,
            "ReconstructionData exceeds deflate-reconstruct/v1 bounds",
        ));
    }
    if sha256_exact(&data.bytes) != data.reconstruction_id {
        return Err(Diagnostic::new(
            OutcomeClass::Corrupt,
            ReasonCode::ReconstructionDataDigestMismatch,
            data.reconstruction_id.to_string(),
        ));
    }
    decode_side_data(&data.bytes)?;
    Ok(())
}

fn recognize(original: &[u8]) -> Result<Option<(Wrapper, usize, usize)>> {
    if original.starts_with(&[0x1f, 0x8b]) {
        return gzip_header_len(original).map(|len| Some((Wrapper::Gzip, len, 8)));
    }
    if original.len() >= 6 {
        let cmf = original[0];
        let flg = original[1];
        if cmf & 0x0f == 8 && cmf >> 4 <= 7 && u16::from_be_bytes([cmf, flg]).is_multiple_of(31) {
            if flg & 0x20 != 0 {
                return Ok(None);
            }
            return Ok(Some((Wrapper::Zlib, 2, 4)));
        }
    }
    // Raw recognition is intentionally the final bounded attempt. Full input
    // consumption and exact recreation are required before it is accepted.
    Ok(Some((Wrapper::Raw, 0, 0)))
}

fn gzip_header_len(input: &[u8]) -> Result<usize> {
    if input.len() < 18 || input[2] != 8 || input[3] & 0xe0 != 0 {
        return Err(failed("unsupported or truncated gzip header"));
    }
    let flags = input[3];
    let mut cursor = 10_usize;
    if flags & 0x04 != 0 {
        if cursor + 2 > input.len() {
            return Err(failed("truncated gzip extra length"));
        }
        let len = usize::from(u16::from_le_bytes([input[cursor], input[cursor + 1]]));
        cursor = cursor
            .checked_add(2 + len)
            .ok_or_else(|| failed("gzip header overflow"))?;
    }
    for flag in [0x08, 0x10] {
        if flags & flag != 0 {
            let Some(end) = input
                .get(cursor..)
                .and_then(|tail| tail.iter().position(|b| *b == 0))
            else {
                return Err(failed("unterminated gzip header string"));
            };
            cursor = cursor
                .checked_add(end + 1)
                .ok_or_else(|| failed("gzip header overflow"))?;
        }
    }
    if flags & 0x02 != 0 {
        cursor = cursor
            .checked_add(2)
            .ok_or_else(|| failed("gzip header overflow"))?;
    }
    if cursor > input.len().saturating_sub(8) {
        return Err(failed("gzip header overlaps footer"));
    }
    Ok(cursor)
}

fn wrapper_checksum_valid(
    wrapper: Wrapper,
    original: &[u8],
    prefix_len: usize,
    suffix_len: usize,
    plaintext: &[u8],
) -> bool {
    if original.len() < prefix_len + suffix_len {
        return false;
    }
    let suffix = &original[original.len() - suffix_len..];
    match wrapper {
        Wrapper::Raw => true,
        Wrapper::Zlib => suffix.len() == 4 && suffix == adler32(plaintext).to_be_bytes(),
        Wrapper::Gzip => {
            suffix.len() == 8
                && suffix[..4] == crc32fast::hash(plaintext).to_le_bytes()
                && suffix[4..] == (plaintext.len() as u32).to_le_bytes()
        }
    }
}

fn adler32(bytes: &[u8]) -> u32 {
    const MOD: u32 = 65_521;
    let (mut a, mut b) = (1_u32, 0_u32);
    for chunk in bytes.chunks(5_552) {
        for byte in chunk {
            a += u32::from(*byte);
            b += a;
        }
        a %= MOD;
        b %= MOD;
    }
    (b << 16) | a
}

fn encode_side_data(
    wrapper: Wrapper,
    prefix: &[u8],
    corrections: &[u8],
    suffix: &[u8],
) -> Result<Vec<u8>> {
    let prefix_len =
        u32::try_from(prefix.len()).map_err(|_| failed("wrapper prefix exceeds u32"))?;
    let corrections_len =
        u64::try_from(corrections.len()).map_err(|_| failed("corrections exceed u64"))?;
    let suffix_len =
        u32::try_from(suffix.len()).map_err(|_| failed("wrapper suffix exceeds u32"))?;
    let mut output =
        Vec::with_capacity(SIDE_HEADER_LEN + prefix.len() + corrections.len() + suffix.len());
    output.extend_from_slice(&SIDE_MAGIC);
    output.push(wrapper as u8);
    output.extend_from_slice(&[0; 3]);
    output.extend_from_slice(&prefix_len.to_be_bytes());
    output.extend_from_slice(&corrections_len.to_be_bytes());
    output.extend_from_slice(&suffix_len.to_be_bytes());
    output.extend_from_slice(prefix);
    output.extend_from_slice(corrections);
    output.extend_from_slice(suffix);
    Ok(output)
}

struct SideData<'a> {
    wrapper: Wrapper,
    prefix: &'a [u8],
    corrections: &'a [u8],
    suffix: &'a [u8],
}

fn decode_side_data(bytes: &[u8]) -> Result<SideData<'_>> {
    if bytes.len() < SIDE_HEADER_LEN || bytes[..4] != SIDE_MAGIC || bytes[5..8] != [0; 3] {
        return Err(failed("ReconstructionData header is malformed"));
    }
    let wrapper = match bytes[4] {
        0 => Wrapper::Raw,
        1 => Wrapper::Zlib,
        2 => Wrapper::Gzip,
        _ => return Err(failed("unknown DEFLATE wrapper kind")),
    };
    let prefix_len = usize::try_from(u32::from_be_bytes(
        bytes[8..12]
            .try_into()
            .map_err(|_| failed("prefix length malformed"))?,
    ))
    .map_err(|_| failed("prefix length exceeds usize"))?;
    let corrections_len = usize::try_from(u64::from_be_bytes(
        bytes[12..20]
            .try_into()
            .map_err(|_| failed("corrections length malformed"))?,
    ))
    .map_err(|_| failed("corrections length exceeds usize"))?;
    let suffix_len = usize::try_from(u32::from_be_bytes(
        bytes[20..24]
            .try_into()
            .map_err(|_| failed("suffix length malformed"))?,
    ))
    .map_err(|_| failed("suffix length exceeds usize"))?;
    let prefix_start = 24_usize;
    let corrections_start = prefix_start
        .checked_add(prefix_len)
        .ok_or_else(|| failed("side-data offset overflow"))?;
    let suffix_start = corrections_start
        .checked_add(corrections_len)
        .ok_or_else(|| failed("side-data offset overflow"))?;
    let end = suffix_start
        .checked_add(suffix_len)
        .ok_or_else(|| failed("side-data offset overflow"))?;
    if end != bytes.len() {
        return Err(failed(
            "ReconstructionData lengths do not consume the exact object",
        ));
    }
    let side = SideData {
        wrapper,
        prefix: &bytes[prefix_start..corrections_start],
        corrections: &bytes[corrections_start..suffix_start],
        suffix: &bytes[suffix_start..end],
    };
    validate_side_shape(&side)?;
    Ok(side)
}

fn validate_side_shape(side: &SideData<'_>) -> Result<()> {
    match side.wrapper {
        Wrapper::Raw if side.prefix.is_empty() && side.suffix.is_empty() => Ok(()),
        Wrapper::Zlib
            if side.prefix.len() == 2
                && side.suffix.len() == 4
                && side.prefix[0] & 0x0f == 8
                && side.prefix[0] >> 4 <= 7
                && u16::from_be_bytes([side.prefix[0], side.prefix[1]]).is_multiple_of(31)
                && side.prefix[1] & 0x20 == 0 =>
        {
            Ok(())
        }
        Wrapper::Gzip if side.suffix.len() == 8 => {
            let mut framed = side.prefix.to_vec();
            framed.extend_from_slice(&[0; 8]);
            if gzip_header_len(&framed).ok() == Some(side.prefix.len()) {
                Ok(())
            } else {
                Err(failed("gzip ReconstructionData prefix is not canonical"))
            }
        }
        _ => Err(failed(
            "ReconstructionData wrapper prefix/suffix shape is invalid",
        )),
    }
}

fn invalid(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::Nonconforming,
        ReasonCode::InvalidTransformParameters,
        detail,
    )
}

fn failed(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::Corrupt,
        ReasonCode::ReconstructionFailed,
        detail,
    )
}

#[cfg(test)]
mod tests {
    use flate2::Compression;
    use flate2::write::{DeflateEncoder, GzEncoder, ZlibEncoder};

    use super::*;

    fn source() -> Vec<u8> {
        (0..20_000)
            .flat_map(|index| {
                format!(
                    "row={index:06};value={:08x};category={}\n",
                    index * 17,
                    index % 31
                )
                .into_bytes()
            })
            .collect()
    }

    fn finish(mut encoder: impl WriteFinish) -> Vec<u8> {
        encoder.write_all(&source()).unwrap();
        encoder.finish_bytes()
    }

    trait WriteFinish: std::io::Write {
        fn finish_bytes(self) -> Vec<u8>;
    }
    impl WriteFinish for DeflateEncoder<Vec<u8>> {
        fn finish_bytes(self) -> Vec<u8> {
            self.finish().unwrap()
        }
    }
    impl WriteFinish for ZlibEncoder<Vec<u8>> {
        fn finish_bytes(self) -> Vec<u8> {
            self.finish().unwrap()
        }
    }
    impl WriteFinish for GzEncoder<Vec<u8>> {
        fn finish_bytes(self) -> Vec<u8> {
            self.finish().unwrap()
        }
    }

    #[test]
    fn generated_raw_zlib_and_gzip_recreate_bit_exact_bytes() {
        let fixtures = [
            finish(DeflateEncoder::new(Vec::new(), Compression::new(6))),
            finish(ZlibEncoder::new(Vec::new(), Compression::new(4))),
            finish(GzEncoder::new(Vec::new(), Compression::new(6))),
        ];
        for (index, original) in fixtures.into_iter().enumerate() {
            let candidate = try_forward(&original, 512).unwrap().unwrap_or_else(|| {
                panic!(
                    "fixture {index} was not recognized ({} bytes)",
                    original.len()
                )
            });
            assert_eq!(
                inverse(&candidate.intermediate, &candidate.data).unwrap(),
                original
            );
            assert_eq!(
                verified_forward(&original, 512, &candidate.data).unwrap(),
                candidate.intermediate
            );
        }
    }

    #[test]
    fn malformed_and_truncated_representations_are_ineligible() {
        assert!(try_forward(&[0x1f, 0x8b, 8, 0], 512).unwrap().is_none());
        assert!(
            try_forward(b"not a complete deflate stream", 512)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn corrupt_side_data_and_wrong_intermediate_fail_closed() {
        let original = finish(ZlibEncoder::new(Vec::new(), Compression::new(6)));
        let candidate = try_forward(&original, 512).unwrap().unwrap();
        let mut corrupt = candidate.data.clone();
        corrupt.bytes[corrupt.bytes.len() - 1] ^= 1;
        assert_eq!(
            validate_data(&corrupt).unwrap_err().code(),
            ReasonCode::ReconstructionDataDigestMismatch
        );
        assert_eq!(
            inverse(
                &candidate.intermediate[..candidate.intermediate.len() - 1],
                &candidate.data
            )
            .unwrap_err()
            .code(),
            ReasonCode::ReconstructedLengthMismatch
        );
    }

    #[test]
    fn structural_step_after_reconstruction_is_exactly_invertible() {
        let original = finish(DeflateEncoder::new(Vec::new(), Compression::new(4)));
        let candidate = try_forward(&original, 512).unwrap().unwrap();
        let steps = vec![
            crate::transform::deflate_reconstruct_step(512, candidate.data.reconstruction_id)
                .unwrap(),
            crate::transform::delta8_step(),
        ];
        let values =
            std::collections::BTreeMap::from([(candidate.data.reconstruction_id, candidate.data)]);
        let transformed =
            crate::transform::forward_pipeline_with_reconstruction(&steps, &original, &values)
                .unwrap();
        let recreated =
            crate::transform::inverse_pipeline_with_reconstruction(&steps, &transformed, &values)
                .unwrap();
        assert_eq!(recreated, original);
    }
}
