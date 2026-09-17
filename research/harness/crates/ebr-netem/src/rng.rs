//! A shared, seeded PRNG for the jitter and loss-as-stall models.
//!
//! `rand_pcg::Pcg64` (a 128-bit-state PCG-XSL-RR generator) is used instead
//! of the `rand` crate's default `ThreadRng`/`StdRng` so a given `--seed`
//! reproduces the same *sequence of draws*. It depends on nothing but
//! `rand_core` (no `chacha20`/`getrandom`), keeping this crate's dependency
//! tree the same shape it already is via `entrybound`'s `reqwest`/`rustls`
//! stack (see `Cargo.toml`).
//!
//! Reproducibility caveat, spelled out here so nobody trusts it further than
//! it goes: seeding makes the *distribution each draw is sampled from*
//! reproducible. It does **not** make a concurrent proxy run byte-for-byte
//! reproducible, because which in-flight request's task calls
//! [`SeededRng::unit_f64`] first on a given poll of the shared
//! `std::sync::Mutex` depends on the Tokio scheduler, not on the seed. A
//! single-request calibration run (see `src/bin/ebr-netem-calibrate.rs`) is
//! unaffected by this; a multi-connection experiment that needs bit-exact
//! replay would need one RNG per connection, seeded from a derived stream
//! id, which this module does not do.

use rand_core::{Rng, SeedableRng};
use rand_pcg::Pcg64;
use std::sync::{Arc, Mutex};

/// Cheaply [`Clone`]able handle to one shared, seeded PRNG stream.
#[derive(Clone)]
pub struct SeededRng(Arc<Mutex<Pcg64>>);

impl SeededRng {
    pub fn new(seed: u64) -> Self {
        SeededRng(Arc::new(Mutex::new(Pcg64::seed_from_u64(seed))))
    }

    /// A uniform sample in `[0, 1)` with the full 53 bits of `f64` mantissa
    /// precision (the standard `next_u64() >> 11` construction).
    pub fn unit_f64(&self) -> f64 {
        let mut rng = self
            .0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let bits = rng.next_u64() >> 11;
        (bits as f64) * (1.0 / (1u64 << 53) as f64)
    }

    /// A sample from `Normal(mean, std_dev)` via the Box-Muller transform,
    /// spending two draws from the shared stream per call.
    pub fn normal_f64(&self, mean: f64, std_dev: f64) -> f64 {
        // u1 in (0, 1] (never exactly 0, so ln() is finite), u2 in [0, 1).
        let (u1, u2) = {
            let mut rng = self
                .0
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let a = ((rng.next_u64() >> 11) as f64 + 1.0) * (1.0 / (1u64 << 53) as f64);
            let b = (rng.next_u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64);
            (a, b)
        };
        let z0 = (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos();
        mean + z0 * std_dev
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_reproduces_the_same_draw_sequence() {
        let a = SeededRng::new(42);
        let b = SeededRng::new(42);
        let seq_a: Vec<f64> = (0..8).map(|_| a.unit_f64()).collect();
        let seq_b: Vec<f64> = (0..8).map(|_| b.unit_f64()).collect();
        assert_eq!(seq_a, seq_b);
    }

    #[test]
    fn different_seeds_diverge() {
        let a = SeededRng::new(1);
        let b = SeededRng::new(2);
        let seq_a: Vec<f64> = (0..8).map(|_| a.unit_f64()).collect();
        let seq_b: Vec<f64> = (0..8).map(|_| b.unit_f64()).collect();
        assert_ne!(seq_a, seq_b);
    }

    #[test]
    fn unit_f64_stays_in_range() {
        let rng = SeededRng::new(7);
        for _ in 0..10_000 {
            let v = rng.unit_f64();
            assert!((0.0..1.0).contains(&v), "{v} out of [0,1)");
        }
    }

    #[test]
    fn normal_samples_cluster_near_the_mean() {
        let rng = SeededRng::new(1234);
        let n = 5_000;
        let mean = 100.0;
        let std_dev = 10.0;
        let samples: Vec<f64> = (0..n).map(|_| rng.normal_f64(mean, std_dev)).collect();
        let sample_mean = samples.iter().sum::<f64>() / n as f64;
        // Loose bound: this is a distribution sanity check, not a
        // statistical test suite -- it only needs to catch a badly wrong
        // Box-Muller implementation (e.g. swapped sin/cos, wrong scale).
        assert!(
            (sample_mean - mean).abs() < 1.0,
            "sample mean {sample_mean} too far from {mean}"
        );
    }

    #[test]
    fn cloned_handle_shares_the_same_underlying_stream() {
        let rng = SeededRng::new(99);
        let clone = rng.clone();
        let a = rng.unit_f64();
        let b = clone.unit_f64();
        let c = rng.unit_f64();
        // a, b, c are three successive draws from one stream, not three
        // independent streams that would repeat.
        assert_ne!(a, b);
        assert_ne!(b, c);
    }
}
