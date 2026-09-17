//! The loss-as-stall model.
//!
//! There is no way to actually drop bytes at the application layer without
//! either corrupting the transfer (which is not what "packet loss" should
//! mean for a client correctness test -- the bytes must still arrive intact)
//! or reimplementing TCP retransmission from scratch. Instead, per the
//! task's own framing, this crate models loss as **a stochastic stall**:
//! before a chunk of the response body is forwarded, every simulated
//! packet in it is lost independently with probability `p_loss`, and the
//! transfer pauses for one RTO ("retransmission timeout") per lost packet,
//! then continues normally. The bytes are never dropped or
//! corrupted, only delayed -- **this is an approximation of packet loss's
//! time cost, not of packet loss itself** (no congestion-window reduction,
//! no actual retransmission, no reordering). `README.md`'s "Threats to
//! validity" section carries this forward; so must any experiment writeup
//! that uses this model.

//!
//! **Loss unit (harness review round 1, finding R1-13).** Earlier revisions
//! rolled once per proxy body chunk (`--chunk-size`, default 16 KiB). The
//! same `--loss-p` then meant a different per-byte loss rate at every chunk
//! size, and `README.md` separately tells operators to raise `--chunk-size`
//! by up to 64x to reach high bandwidths -- silently cutting the effective
//! loss rate by the same factor. The default unit is now a simulated TCP
//! segment of [`DEFAULT_MSS_BYTES`] payload bytes, independent of
//! `--chunk-size`; `LossUnit::Chunk` keeps the old behavior for comparison.

use crate::rng::SeededRng;
use std::time::Duration;

/// Payload bytes per simulated segment: a 1500-byte Ethernet MTU minus
/// 40 bytes of IPv4/TCP headers and 12 bytes of TCP timestamp option.
pub const DEFAULT_MSS_BYTES: usize = 1448;

/// What one loss roll applies to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LossUnit {
    /// One roll per simulated segment of this many payload bytes.
    Packet { mss_bytes: usize },
    /// One roll per proxy body chunk (legacy; couples loss to `--chunk-size`).
    Chunk,
}

impl LossUnit {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "packet" => Some(Self::Packet {
                mss_bytes: DEFAULT_MSS_BYTES,
            }),
            "chunk" => Some(Self::Chunk),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LossModel {
    /// Probability, in `[0, 1]`, that one unit (see `unit`) is lost.
    pub p_loss: f64,
    /// How long each loss stalls the transfer.
    pub rto: Duration,
    pub unit: LossUnit,
}

impl LossModel {
    pub fn none() -> Self {
        LossModel {
            p_loss: 0.0,
            rto: Duration::ZERO,
            unit: LossUnit::Packet {
                mss_bytes: DEFAULT_MSS_BYTES,
            },
        }
    }

    /// Per-packet loss (the default unit).
    pub fn new(p_loss: f64, rto: Duration) -> Self {
        Self::with_unit(
            p_loss,
            rto,
            LossUnit::Packet {
                mss_bytes: DEFAULT_MSS_BYTES,
            },
        )
    }

    pub fn with_unit(p_loss: f64, rto: Duration, unit: LossUnit) -> Self {
        LossModel {
            p_loss: p_loss.clamp(0.0, 1.0),
            rto,
            unit,
        }
    }

    /// Total stall owed before forwarding a `chunk_len`-byte body chunk:
    /// `rto` times the number of lost units in it. A disabled model returns
    /// zero without drawing from `rng`.
    pub fn stall_for_chunk(&self, chunk_len: usize, rng: &SeededRng) -> Duration {
        if self.p_loss <= 0.0 || chunk_len == 0 {
            return Duration::ZERO;
        }
        let units = match self.unit {
            LossUnit::Chunk => 1,
            LossUnit::Packet { mss_bytes } => chunk_len.div_ceil(mss_bytes.max(1)),
        };
        let lost = (0..units).filter(|_| self.roll(rng)).count();
        self.rto
            .saturating_mul(u32::try_from(lost).unwrap_or(u32::MAX))
    }

    /// Draws once from `rng` and reports whether this chunk should stall.
    /// `p_loss <= 0.0` never draws at all, so a disabled loss model costs
    /// nothing and consumes no entropy from the shared stream.
    pub fn roll(&self, rng: &SeededRng) -> bool {
        self.p_loss > 0.0 && rng.unit_f64() < self.p_loss
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_probability_never_stalls() {
        let model = LossModel::new(0.0, Duration::from_millis(200));
        let rng = SeededRng::new(1);
        for _ in 0..1_000 {
            assert!(!model.roll(&rng));
        }
    }

    #[test]
    fn certain_loss_always_stalls() {
        let model = LossModel::new(1.0, Duration::from_millis(200));
        let rng = SeededRng::new(1);
        for _ in 0..1_000 {
            assert!(model.roll(&rng));
        }
    }

    #[test]
    fn probability_is_clamped_into_range() {
        assert_eq!(LossModel::new(-1.0, Duration::ZERO).p_loss, 0.0);
        assert_eq!(LossModel::new(2.0, Duration::ZERO).p_loss, 1.0);
    }

    #[test]
    fn moderate_probability_stalls_at_roughly_the_configured_rate() {
        let model = LossModel::new(0.2, Duration::from_millis(1));
        let rng = SeededRng::new(2024);
        let trials = 20_000;
        let hits = (0..trials).filter(|_| model.roll(&rng)).count();
        let rate = hits as f64 / trials as f64;
        assert!(
            (rate - 0.2).abs() < 0.02,
            "observed rate {rate} far from 0.2"
        );
    }

    #[test]
    fn per_packet_loss_rate_does_not_depend_on_chunk_size() {
        let model = LossModel::new(0.01, Duration::from_millis(1));
        let rng = SeededRng::new(7);
        let total_bytes = 64 * 1024 * 1024usize;
        let stalls = |chunk: usize| -> u128 {
            (0..total_bytes / chunk)
                .map(|_| model.stall_for_chunk(chunk, &rng).as_millis())
                .sum()
        };
        let small = stalls(16 * 1024) as f64;
        let large = stalls(1024 * 1024) as f64;
        let expected = (total_bytes / DEFAULT_MSS_BYTES) as f64 * 0.01;
        for observed in [small, large] {
            assert!(
                (observed / expected - 1.0).abs() < 0.1,
                "observed {observed} lost packets, expected about {expected}"
            );
        }
        let legacy = LossModel::with_unit(0.01, Duration::from_millis(1), LossUnit::Chunk);
        let legacy_large: u128 = (0..total_bytes / (1024 * 1024))
            .map(|_| legacy.stall_for_chunk(1024 * 1024, &rng).as_millis())
            .sum();
        assert!((legacy_large as f64) < expected / 10.0);
    }

    #[test]
    fn none_never_stalls_and_never_touches_the_rng() {
        let model = LossModel::none();
        let rng = SeededRng::new(1);
        let before: Vec<f64> = (0..4).map(|_| rng.unit_f64()).collect();
        // A fresh rng with the same seed, never drawn from by `roll`, should
        // reproduce the same next draws -- confirming `roll` short-circuited
        // before calling `unit_f64`.
        let untouched = SeededRng::new(1);
        for _ in 0..100 {
            assert!(!model.roll(&untouched));
        }
        let after: Vec<f64> = (0..4).map(|_| untouched.unit_f64()).collect();
        assert_eq!(before, after);
    }
}
