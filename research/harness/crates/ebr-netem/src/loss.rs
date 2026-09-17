//! The loss-as-stall model.
//!
//! There is no way to actually drop bytes at the application layer without
//! either corrupting the transfer (which is not what "packet loss" should
//! mean for a client correctness test -- the bytes must still arrive intact)
//! or reimplementing TCP retransmission from scratch. Instead, per the
//! task's own framing, this crate models loss as **a stochastic stall**:
//! with probability `p_loss`, before a chunk of the response body is
//! forwarded, the transfer pauses for one RTO ("retransmission timeout")
//! duration, then continues normally. The bytes are never dropped or
//! corrupted, only delayed -- **this is an approximation of packet loss's
//! time cost, not of packet loss itself** (no congestion-window reduction,
//! no actual retransmission, no reordering). `README.md`'s "Threats to
//! validity" section carries this forward; so must any experiment writeup
//! that uses this model.

use crate::rng::SeededRng;
use std::time::Duration;

#[derive(Debug, Clone, Copy)]
pub struct LossModel {
    /// Probability, in `[0, 1]`, that a given chunk triggers a stall.
    pub p_loss: f64,
    /// How long a triggered stall lasts.
    pub rto: Duration,
}

impl LossModel {
    pub fn none() -> Self {
        LossModel {
            p_loss: 0.0,
            rto: Duration::ZERO,
        }
    }

    pub fn new(p_loss: f64, rto: Duration) -> Self {
        LossModel {
            p_loss: p_loss.clamp(0.0, 1.0),
            rto,
        }
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
