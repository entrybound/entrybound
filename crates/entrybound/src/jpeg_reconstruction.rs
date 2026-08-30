//! Bounded, bit-exact JPEG/JPEG XL whole-object reconstruction.

use std::io::{Cursor, Write};

use jxl_oxide::{AllocTracker, JpegReconstructionStatus, JxlImage};

use crate::diagnostics::{Diagnostic, OutcomeClass, ReasonCode, Result};
use crate::eam::{Digest, ReconstructionRegion};
use crate::identity::sha256_exact;

pub(crate) const JPEG_RECONSTRUCT_ID: &str = "jpeg-jxl-reconstruct/v1";
pub(crate) const JPEG_RECONSTRUCT_PARAMETERS: &[u8] =
    b"JJ01\0jixel-0.2.19\0jxl-oxide-0.12.6\0threads=1";
pub(crate) const MAX_JPEG_BYTES: usize = 64 * 1024 * 1024;
pub(crate) const MAX_JXL_BYTES: usize = 64 * 1024 * 1024;
pub(crate) const MAX_JPEG_PIXELS: u64 = 100_000_000;
pub(crate) const JPEG_WORKING_SET_BYTES: u64 = 256 * 1024 * 1024;
pub(crate) const MAX_REGION_CHUNKS: u64 = 4_096;
pub(crate) const MAX_REGION_EXPANSION_RATIO: u64 = 64;
pub(crate) const REGION_MEMBER_PLAN_REF: u64 = 0;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct VerifiedJpegRepresentation {
    pub bytes: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum JpegAttemptFailure {
    NotRecognized,
    Unsupported,
    VerificationFailed,
    ResourceExcluded,
}

pub(crate) fn verified_forward(
    original: &[u8],
) -> std::result::Result<VerifiedJpegRepresentation, JpegAttemptFailure> {
    if original.len() > MAX_JPEG_BYTES {
        return Err(JpegAttemptFailure::ResourceExcluded);
    }
    let (width, height) = jpeg_dimensions(original).ok_or(JpegAttemptFailure::NotRecognized)?;
    let pixels = u64::from(width)
        .checked_mul(u64::from(height))
        .ok_or(JpegAttemptFailure::ResourceExcluded)?;
    if pixels == 0 || pixels > MAX_JPEG_PIXELS {
        return Err(JpegAttemptFailure::ResourceExcluded);
    }
    let config = jixel::JpegTranscodeConfig::default()
        .with_num_threads(1)
        .with_jpeg_reconstruction(true);
    let bytes = jixel::encode_jpeg_lossless_with_config(original, &config)
        .map_err(|_| JpegAttemptFailure::Unsupported)?;
    if bytes.len() > MAX_JXL_BYTES {
        return Err(JpegAttemptFailure::ResourceExcluded);
    }
    let recreated = inverse(&bytes).map_err(|_| JpegAttemptFailure::VerificationFailed)?;
    if sha256_exact(original) != sha256_exact(&recreated) || original != recreated {
        return Err(JpegAttemptFailure::VerificationFailed);
    }
    Ok(VerifiedJpegRepresentation {
        bytes,
        width,
        height,
    })
}

pub(crate) fn inverse(representation: &[u8]) -> Result<Vec<u8>> {
    if representation.len() > MAX_JXL_BYTES {
        return Err(resource("JPEG XL representation exceeds the v1 bound"));
    }
    let tracker = AllocTracker::with_limit(
        usize::try_from(JPEG_WORKING_SET_BYTES)
            .map_err(|_| resource("JPEG reconstruction working set exceeds usize"))?,
    );
    let image = JxlImage::builder()
        .alloc_tracker(tracker)
        .read(Cursor::new(representation))
        .map_err(|error| malformed(format!("parse JPEG XL representation: {error}")))?;
    let pixels = u64::from(image.width())
        .checked_mul(u64::from(image.height()))
        .ok_or_else(|| resource("JPEG XL dimensions overflow"))?;
    if pixels == 0 || pixels > MAX_JPEG_PIXELS {
        return Err(resource("JPEG XL dimensions exceed the v1 pixel bound"));
    }
    if image.jpeg_reconstruction_status() != JpegReconstructionStatus::Available {
        return Err(malformed(
            "JPEG XL representation has no complete JPEG reconstruction data",
        ));
    }
    let mut output = BoundedWriter::new(MAX_JPEG_BYTES);
    image
        .reconstruct_jpeg(&mut output)
        .map_err(|error| malformed(format!("reconstruct JPEG bitstream: {error}")))?;
    Ok(output.finish())
}

pub(crate) fn validate_parameters(parameters: &[u8]) -> Result<()> {
    if parameters == JPEG_RECONSTRUCT_PARAMETERS {
        Ok(())
    } else {
        Err(Diagnostic::new(
            OutcomeClass::Nonconforming,
            ReasonCode::InvalidTransformParameters,
            "jpeg-jxl-reconstruct/v1 parameters are not canonical",
        ))
    }
}

pub(crate) fn region_identity(region: &ReconstructionRegion) -> Digest {
    let representation_digest = sha256_exact(&region.representation);
    let mut bytes = Vec::with_capacity(128);
    bytes.extend_from_slice(b"entrybound/reconstruction-region/jpeg-jxl/v1\0");
    bytes.extend_from_slice(region.content_object.as_bytes());
    bytes.extend_from_slice(&region.start_chunk_index.to_be_bytes());
    bytes.extend_from_slice(&region.chunk_count.to_be_bytes());
    bytes.extend_from_slice(&region.plan_ref.to_be_bytes());
    bytes.extend_from_slice(&region.logical_bytes.to_be_bytes());
    bytes.extend_from_slice(&region.transformed_bytes.to_be_bytes());
    bytes.extend_from_slice(&region.ordinary_physical_bytes.to_be_bytes());
    bytes.extend_from_slice(&region.region_overhead_bytes.to_be_bytes());
    bytes.extend_from_slice(&region.access.logical_bytes.to_be_bytes());
    bytes.extend_from_slice(&region.access.logical_chunks.to_be_bytes());
    bytes.extend_from_slice(&region.access.worst_reconstructed_bytes.to_be_bytes());
    bytes.extend_from_slice(representation_digest.as_bytes());
    sha256_exact(&bytes)
}

fn jpeg_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.len() < 4 || bytes[..2] != [0xff, 0xd8] || bytes[bytes.len() - 2..] != [0xff, 0xd9] {
        return None;
    }
    let mut cursor = 2_usize;
    while cursor + 4 <= bytes.len() {
        if bytes[cursor] != 0xff {
            return None;
        }
        while cursor < bytes.len() && bytes[cursor] == 0xff {
            cursor += 1;
        }
        let marker = *bytes.get(cursor)?;
        cursor += 1;
        if marker == 0xda || marker == 0xd9 {
            break;
        }
        if marker == 0x01 || (0xd0..=0xd8).contains(&marker) {
            continue;
        }
        let length = usize::from(u16::from_be_bytes([
            *bytes.get(cursor)?,
            *bytes.get(cursor + 1)?,
        ]));
        if length < 2 || cursor.checked_add(length)? > bytes.len() {
            return None;
        }
        if is_start_of_frame(marker) {
            if length < 8 {
                return None;
            }
            let height = u32::from(u16::from_be_bytes([bytes[cursor + 3], bytes[cursor + 4]]));
            let width = u32::from(u16::from_be_bytes([bytes[cursor + 5], bytes[cursor + 6]]));
            return Some((width, height));
        }
        cursor += length;
    }
    None
}

fn is_start_of_frame(marker: u8) -> bool {
    matches!(
        marker,
        0xc0 | 0xc1 | 0xc2 | 0xc3 | 0xc5 | 0xc6 | 0xc7 | 0xc9 | 0xca | 0xcb | 0xcd | 0xce | 0xcf
    )
}

struct BoundedWriter {
    bytes: Vec<u8>,
    limit: usize,
}

impl BoundedWriter {
    fn new(limit: usize) -> Self {
        Self {
            bytes: Vec::new(),
            limit,
        }
    }

    fn finish(self) -> Vec<u8> {
        self.bytes
    }
}

impl Write for BoundedWriter {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        let next = self
            .bytes
            .len()
            .checked_add(buffer.len())
            .filter(|length| *length <= self.limit)
            .ok_or_else(|| std::io::Error::other("reconstructed JPEG exceeds configured bound"))?;
        self.bytes.reserve(next - self.bytes.len());
        self.bytes.extend_from_slice(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn malformed(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::Corrupt,
        ReasonCode::MalformedReconstructionPayload,
        detail,
    )
}

fn resource(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::PolicyRefused,
        ReasonCode::ResourceLimit,
        detail,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::codecs::jpeg::JpegEncoder;
    use image::{ExtendedColorType, ImageEncoder};

    #[test]
    fn pure_rust_transcode_reconstructs_generated_jpeg_exactly() {
        let jpeg = generated_jpeg(128, 96);
        let candidate = verified_forward(&jpeg).unwrap();
        assert_eq!(inverse(&candidate.bytes).unwrap(), jpeg);
        assert_eq!((candidate.width, candidate.height), (128, 96));
    }

    #[test]
    fn common_metadata_markers_are_exact_or_ineligible() {
        let jpeg = generated_jpeg(96, 64);
        for (name, marker) in [
            ("comment", marker(0xfe, b"Entrybound deterministic comment")),
            ("exif", marker(0xe1, b"Exif\0\0MM\0*\0\0\0\x08\0\0")),
            ("icc", marker(0xe2, b"ICC_PROFILE\0\x01\x01entrybound-icc")),
        ] {
            let mut with_marker = Vec::with_capacity(jpeg.len() + marker.len());
            with_marker.extend_from_slice(&jpeg[..2]);
            with_marker.extend_from_slice(&marker);
            with_marker.extend_from_slice(&jpeg[2..]);
            match verified_forward(&with_marker) {
                Ok(candidate) => {
                    assert_eq!(inverse(&candidate.bytes).unwrap(), with_marker, "{name}")
                }
                Err(JpegAttemptFailure::Unsupported | JpegAttemptFailure::VerificationFailed) => {
                    // A producer/library combination that cannot preserve this marker
                    // exactly is deliberately outside v1 eligibility.
                }
                Err(error) => panic!("unexpected {name} marker outcome: {error:?}"),
            }
        }
    }

    #[test]
    fn malformed_and_non_jpeg_inputs_are_ineligible() {
        assert_eq!(
            verified_forward(b"not a jpeg").unwrap_err(),
            JpegAttemptFailure::NotRecognized
        );
        let mut jpeg = generated_jpeg(32, 32);
        jpeg.truncate(jpeg.len() / 2);
        assert_eq!(
            verified_forward(&jpeg).unwrap_err(),
            JpegAttemptFailure::NotRecognized
        );
    }

    fn generated_jpeg(width: u32, height: u32) -> Vec<u8> {
        let pixels = (0..height)
            .flat_map(|y| {
                (0..width).flat_map(move |x| {
                    [
                        x.wrapping_mul(13).wrapping_add(y * 3) as u8,
                        y.wrapping_mul(17).wrapping_add(x * 5) as u8,
                        x.wrapping_mul(7).wrapping_add(y * 11) as u8,
                    ]
                })
            })
            .collect::<Vec<_>>();
        let mut jpeg = Vec::new();
        JpegEncoder::new_with_quality(&mut jpeg, 84)
            .write_image(&pixels, width, height, ExtendedColorType::Rgb8)
            .unwrap();
        jpeg
    }

    fn marker(kind: u8, payload: &[u8]) -> Vec<u8> {
        let length = u16::try_from(payload.len() + 2).unwrap();
        let mut marker = vec![0xff, kind];
        marker.extend_from_slice(&length.to_be_bytes());
        marker.extend_from_slice(payload);
        marker
    }
}
