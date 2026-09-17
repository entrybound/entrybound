//! Structural transform candidates beyond production's set.
//!
//! `entrybound::research::transform` exposes production's own two structural
//! transforms (`delta8`, single-order byte delta; `byte-shuffle`, byte-plane
//! splitting at width 2, 4, or 8 exactly) plus its two reconstructive
//! transforms (DEFLATE, JPEG). This module adds transforms production has
//! never considered, each exactly invertible over arbitrary bytes (the same
//! `ReversibilityClass::Structural` contract production's own delta8/shuffle
//! satisfy), so a false-positive scan ([`crate::matrix`]) can treat them the
//! same way: apply the transform, then compress, and see whether the
//! transform actually helped.
//!
//! - [`bcj`] -- x86/ARM64/RISC-V branch/call/jump filters, via
//!   `lzma-rust2`'s standalone `BcjWriter`/`BcjReader` (the same crate and
//!   pin backs the xz+BCJ codec candidate in `external.rs`, but applied here
//!   on its own, ahead of a plain codec, not inside an xz container).
//! - [`delta_of_delta`] -- second-order delta (the delta of production's
//!   `delta8` deltas) over a configurable element width, for float/integer
//!   columnar data where a single first-order delta is not always the best
//!   fit.
//! - [`byte_plane_split`] -- production's own `byte-shuffle` algorithm
//!   (verified byte-for-byte identical to it at width 2/4/8 in this module's
//!   tests), generalized to widths outside {2, 4, 8}: 3 (RGB pixels), 5, 6,
//!   7, and other record widths production's `byte_shuffle_step` refuses.

use std::io::{Read, Write};

/// Which BCJ filter to apply. RISC-V is the widest-interest addition beyond
/// x86/ARM64 (a growing corpus target, and not one of xz's original four
/// filters).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BcjKind {
    X86,
    Arm64,
    RiscV,
}

impl BcjKind {
    pub fn label(self) -> &'static str {
        match self {
            BcjKind::X86 => "bcj-x86",
            BcjKind::Arm64 => "bcj-arm64",
            BcjKind::RiscV => "bcj-riscv",
        }
    }
}

/// Applies a BCJ filter to `data` (encoder direction: converts relative
/// branch targets to absolute, the direction xz/7z apply before LZMA so
/// repeated targets become visible to the match finder). Same length in and
/// out; a BCJ filter is a byte-for-byte substitution, never an expansion or
/// contraction.
pub fn bcj_forward(kind: BcjKind, data: &[u8]) -> std::io::Result<Vec<u8>> {
    use lzma_rust2::filter::bcj::BcjWriter;
    let mut writer: BcjWriter<Vec<u8>> = match kind {
        BcjKind::X86 => BcjWriter::new_x86(Vec::with_capacity(data.len()), 0),
        BcjKind::Arm64 => BcjWriter::new_arm64(Vec::with_capacity(data.len()), 0),
        BcjKind::RiscV => BcjWriter::new_riscv(Vec::with_capacity(data.len()), 0),
    };
    writer.write_all(data)?;
    writer.finish()
}

/// Inverts [`bcj_forward`] (decoder direction).
pub fn bcj_inverse(kind: BcjKind, data: &[u8]) -> std::io::Result<Vec<u8>> {
    use lzma_rust2::filter::bcj::BcjReader;
    let mut reader: BcjReader<&[u8]> = match kind {
        BcjKind::X86 => BcjReader::new_x86(data, 0),
        BcjKind::Arm64 => BcjReader::new_arm64(data, 0),
        BcjKind::RiscV => BcjReader::new_riscv(data, 0),
    };
    let mut out = Vec::with_capacity(data.len());
    reader.read_to_end(&mut out)?;
    Ok(out)
}

/// Second-order delta ("delta of deltas") over `width`-byte little-endian
/// unsigned lanes (1, 2, 4, or 8 bytes: `u8`/`u16`/`u32`/`u64`), each lane
/// wrapping independently so the transform is exactly invertible over any
/// input, including one that is not a whole number of `width`-byte elements
/// (the remainder is passed through, matching production's own
/// `byte-shuffle` tail handling). Useful where a single first-order delta
/// (production's `delta8`, always 1-byte lanes) still leaves a strong linear
/// trend, e.g. a monotonically-increasing or near-constant-slope numeric
/// column (timestamps, incrementing ids, sample-rate audio, sorted floats
/// reinterpreted as bit patterns).
pub fn delta_of_delta_forward(width: u8, data: &[u8]) -> Vec<u8> {
    delta_pass(width, &delta_pass(width, data))
}

/// Inverts [`delta_of_delta_forward`]: two cumulative-sum passes.
pub fn delta_of_delta_inverse(width: u8, data: &[u8]) -> Vec<u8> {
    integrate_pass(width, &integrate_pass(width, data))
}

fn lane_width(width: u8) -> usize {
    match width {
        1 | 2 | 4 | 8 => width as usize,
        other => panic!("delta-of-delta lane width must be 1, 2, 4, or 8 bytes, got {other}"),
    }
}

fn read_lane(width: usize, bytes: &[u8]) -> u64 {
    let mut buf = [0u8; 8];
    buf[..width].copy_from_slice(bytes);
    u64::from_le_bytes(buf)
}

fn write_lane(width: usize, value: u64, out: &mut Vec<u8>) {
    out.extend_from_slice(&value.to_le_bytes()[..width]);
}

fn delta_pass(width: u8, data: &[u8]) -> Vec<u8> {
    let width = lane_width(width);
    let complete = data.len() / width;
    let tail = complete * width;
    let mut out = Vec::with_capacity(data.len());
    let mut previous = 0u64;
    for lane_index in 0..complete {
        let lane = read_lane(width, &data[lane_index * width..][..width]);
        write_lane(width, lane.wrapping_sub(previous), &mut out);
        previous = lane;
    }
    out.extend_from_slice(&data[tail..]);
    out
}

fn integrate_pass(width: u8, data: &[u8]) -> Vec<u8> {
    let width_bytes = lane_width(width);
    let complete = data.len() / width_bytes;
    let tail = complete * width_bytes;
    let mut out = Vec::with_capacity(data.len());
    let mut running = 0u64;
    for lane_index in 0..complete {
        let delta = read_lane(
            width_bytes,
            &data[lane_index * width_bytes..][..width_bytes],
        );
        running = running.wrapping_add(delta);
        write_lane(width_bytes, running, &mut out);
    }
    out.extend_from_slice(&data[tail..]);
    out
}

/// Byte-plane splitting ("shuffle") at an arbitrary `width`, including
/// widths production's `byte_shuffle_step` refuses (anything other than 2,
/// 4, or 8): 3-byte RGB pixels, 5/6/7-byte fixed records, and so on. The
/// algorithm is exactly production's `byte-shuffle/v1` (this module's tests
/// check byte-for-byte agreement at width 2/4/8 against
/// `entrybound::research::transform::byte_shuffle_step` +
/// `forward_pipeline`); the only thing this widens is which widths are
/// accepted.
pub fn byte_plane_split_forward(width: u8, data: &[u8]) -> Vec<u8> {
    let width = width as usize;
    assert!(width >= 1, "byte-plane split width must be at least 1");
    let complete = data.len() / width;
    let tail = complete * width;
    let mut out = Vec::with_capacity(data.len());
    for lane in 0..width {
        for item in 0..complete {
            out.push(data[item * width + lane]);
        }
    }
    out.extend_from_slice(&data[tail..]);
    out
}

/// Inverts [`byte_plane_split_forward`].
pub fn byte_plane_split_inverse(width: u8, data: &[u8]) -> Vec<u8> {
    let width = width as usize;
    assert!(width >= 1, "byte-plane split width must be at least 1");
    let complete = data.len() / width;
    let tail = complete * width;
    let mut out = vec![0u8; tail];
    for lane in 0..width {
        for item in 0..complete {
            out[item * width + lane] = data[lane * complete + item];
        }
    }
    out.extend_from_slice(&data[tail..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pseudo_random_bytes(len: usize, seed: u64) -> Vec<u8> {
        // A small xorshift generator: deterministic, no extra dependency,
        // and not a whole number of any lane width by default (`len` is
        // chosen per test to probe the tail-handling path too).
        let mut state = seed | 1;
        (0..len)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                (state & 0xff) as u8
            })
            .collect()
    }

    #[test]
    fn bcj_filters_round_trip_arbitrary_bytes() {
        for kind in [BcjKind::X86, BcjKind::Arm64, BcjKind::RiscV] {
            let data = pseudo_random_bytes(10_003, 42);
            let filtered = bcj_forward(kind, &data).unwrap();
            assert_eq!(filtered.len(), data.len(), "{}", kind.label());
            let restored = bcj_inverse(kind, &filtered).unwrap();
            assert_eq!(restored, data, "{}", kind.label());
        }
    }

    #[test]
    fn bcj_filter_is_not_a_no_op_on_x86_code_like_bytes() {
        // A run of `0xE8` (CALL rel32) opcodes followed by plausible relative
        // displacements is exactly what the x86 BCJ filter targets.
        let mut data = Vec::new();
        for i in 0..200u32 {
            data.push(0xE8);
            data.extend_from_slice(&(100_i32 + i as i32).to_le_bytes());
        }
        let filtered = bcj_forward(BcjKind::X86, &data).unwrap();
        assert_ne!(filtered, data);
        assert_eq!(bcj_inverse(BcjKind::X86, &filtered).unwrap(), data);
    }

    #[test]
    fn delta_of_delta_round_trips_and_collapses_a_linear_ramp() {
        for width in [1u8, 2, 4, 8] {
            let data = pseudo_random_bytes(4099, 7);
            let transformed = delta_of_delta_forward(width, &data);
            assert_eq!(transformed.len(), data.len());
            assert_eq!(delta_of_delta_inverse(width, &transformed), data);
        }

        // A perfectly linear ramp (constant first-order delta) collapses to
        // all-zero lanes under a second-order delta.
        let ramp: Vec<u8> = (0..1000u32).flat_map(|i| (i * 7).to_le_bytes()).collect();
        let transformed = delta_of_delta_forward(4, &ramp);
        // Lane 0 is the first value itself and lane 1 is the first-order
        // delta (both have no earlier lane to subtract); every lane from
        // index 2 on is the delta of two equal first-order deltas, so it
        // collapses to 0.
        for lane in transformed.as_chunks::<4>().0.iter().skip(2) {
            assert_eq!(u32::from_le_bytes(*lane), 0);
        }
        assert_eq!(delta_of_delta_inverse(4, &transformed), ramp);
    }

    #[test]
    fn byte_plane_split_round_trips_at_widths_production_refuses() {
        for width in [1u8, 3, 5, 6, 7, 16] {
            let data = pseudo_random_bytes(2503, width as u64 + 1);
            let transformed = byte_plane_split_forward(width, &data);
            assert_eq!(transformed.len(), data.len());
            assert_eq!(byte_plane_split_inverse(width, &transformed), data);
        }
    }

    /// Cross-checks this module's width-2/4/8 byte-plane split against
    /// production's own `byte-shuffle/v1` (`entrybound::research::transform`)
    /// byte-for-byte: this is not a reimplementation production could drift
    /// from unnoticed, it is the same algorithm, widened.
    #[test]
    fn byte_plane_split_agrees_with_production_byte_shuffle_at_shared_widths() {
        use entrybound::research::transform;
        let data = pseudo_random_bytes(10_007, 99);
        for width in [2u8, 4, 8] {
            let step = transform::byte_shuffle_step(width).unwrap();
            let production = transform::forward_pipeline(&[step], &data).unwrap();
            let harness = byte_plane_split_forward(width, &data);
            assert_eq!(harness, production, "width {width}");
        }
    }
}
