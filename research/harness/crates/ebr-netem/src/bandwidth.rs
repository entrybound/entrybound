//! A token-bucket bandwidth shaper.
//!
//! One [`TokenBucket`] models one direction of one configured bandwidth cap.
//! `ebr-netem-proxy` holds two -- upstream (client -> origin request bytes)
//! and downstream (origin -> client response bytes) -- per the task's
//! "bandwidth token-bucket shaping in each direction". The bucket starts
//! full (an immediate burst up to `burst_bytes` is free), then refills at
//! `rate_bps` bytes/second, standard token-bucket behavior.
//!
//! [`TokenBucket::consume`] is the only entry point and is async: it awaits
//! (via [`tokio::time::sleep`]) until enough tokens exist rather than
//! consuming a partial grant, so a caller pacing a stream one frame at a
//! time (see `proxy.rs`'s `shaped_body_stream`) gets exactly the configured
//! rate over time, not bursty catch-up behavior.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Minimum burst allowance, in seconds of the configured rate.
///
/// Harness review round 1, finding R1-13: `tokio::time::sleep` resolves to
/// whole milliseconds and never wakes early, so every paced chunk oversleeps
/// by up to about a millisecond. The bucket credits that oversleep back as
/// tokens only up to its capacity; with a capacity smaller than
/// `rate * oversleep` the credit is thrown away and the achieved rate falls
/// short. That is why the provisional calibration achieved 62 Mbit/s at a
/// configured 100 and 107 Mbit/s at 1000 (with `--burst-down-bytes 4096`,
/// 16 KiB chunks): the achieved rates match whole-millisecond sleeps of
/// 16 KiB chunks almost exactly. Linux `tc tbf` has the same constraint
/// (its burst must be at least `rate / HZ`). The effective capacity is
/// therefore never below `rate * MIN_BURST_SECONDS`.
pub const MIN_BURST_SECONDS: f64 = 0.005;

struct State {
    rate_bps: f64,
    capacity: f64,
    tokens: f64,
    last_refill: Instant,
}

impl State {
    fn refill(&mut self) {
        if !self.rate_bps.is_finite() {
            return; // unlimited: never meters, see TokenBucket::unlimited
        }
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.last_refill = now;
        self.tokens = (self.tokens + elapsed * self.rate_bps).min(self.capacity);
    }
}

#[derive(Clone)]
pub struct TokenBucket(Arc<Mutex<State>>);

impl TokenBucket {
    /// `rate_bps` bytes/second sustained; `burst_bytes` is both the starting
    /// token count and the refill ceiling (how far ahead of the sustained
    /// rate a caller can burst after being idle).
    pub fn new(rate_bps: f64, burst_bytes: u64) -> Self {
        let capacity = (burst_bytes.max(1) as f64).max(rate_bps.max(0.0) * MIN_BURST_SECONDS);
        TokenBucket(Arc::new(Mutex::new(State {
            rate_bps: rate_bps.max(0.0),
            capacity,
            tokens: capacity,
            last_refill: Instant::now(),
        })))
    }

    /// The effective burst capacity in bytes (see [`MIN_BURST_SECONDS`]).
    pub fn capacity(&self) -> f64 {
        self.0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .capacity
    }

    /// A bucket that never delays -- `rate_bps` and `burst_bytes` config
    /// values of `0`/absent map to this (see `config.rs`), not to a bucket
    /// that stalls forever.
    pub fn unlimited() -> Self {
        TokenBucket(Arc::new(Mutex::new(State {
            rate_bps: f64::INFINITY,
            capacity: f64::INFINITY,
            tokens: f64::INFINITY,
            last_refill: Instant::now(),
        })))
    }

    /// Waits until `n` bytes' worth of tokens are available, then consumes
    /// them (going into debt if `n` exceeds what is currently available --
    /// see below). Never holds the internal lock across an `.await`.
    ///
    /// `n` is allowed to exceed `capacity` entirely -- a single request for
    /// more bytes than one burst's worth is paced at the sustained rate for
    /// whatever exceeds the free burst, not rejected or (as an earlier,
    /// buggy version of this method did) looped on forever because
    /// [`State::refill`] never lets `tokens` exceed `capacity` and so
    /// `tokens >= n` could never become true for `n > capacity`. The fix is
    /// to always grant the request immediately, deducting `n` from
    /// `tokens` even when that leaves it negative (a debt), and to compute
    /// the wait from that debt directly instead of re-looping to re-check.
    /// This also correctly serializes concurrent callers sharing one
    /// bucket: a second call that locks while the first's debt has not yet
    /// been paid down sees that (still-negative) balance and so waits for
    /// both its own bytes and the first call's debt ahead of it.
    pub async fn consume(&self, n: u64) {
        if n == 0 {
            return;
        }
        let wait = {
            let mut state = self
                .0
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            state.refill();
            if !state.rate_bps.is_finite() {
                return;
            }
            state.tokens -= n as f64;
            if state.tokens >= 0.0 {
                Duration::ZERO
            } else if state.rate_bps > 0.0 {
                Duration::from_secs_f64((-state.tokens) / state.rate_bps)
            } else {
                // A configured rate of exactly 0 B/s with capacity already
                // spent is a fully blocked link. There is no finite correct
                // wait; cap it instead of returning an infinite duration.
                Duration::from_secs(3600)
            }
        };
        if !wait.is_zero() {
            tokio::time::sleep(wait).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn an_initial_burst_up_to_capacity_does_not_wait() {
        let bucket = TokenBucket::new(1_000.0, 500);
        let start = tokio::time::Instant::now();
        bucket.consume(500).await;
        assert_eq!(
            tokio::time::Instant::now(),
            start,
            "burst should be instantaneous"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn exceeding_capacity_waits_for_the_deficit_at_the_configured_rate() {
        let bucket = TokenBucket::new(1_000.0, 500); // 1000 B/s, burst 500 B
        bucket.consume(500).await; // drains the initial burst instantly
        let start = tokio::time::Instant::now();
        bucket.consume(1_000).await; // needs 1000 more tokens at 1000 B/s => 1s
        let elapsed = tokio::time::Instant::now() - start;
        assert!(
            elapsed >= Duration::from_millis(950) && elapsed <= Duration::from_millis(1_050),
            "elapsed {elapsed:?} not close to 1s"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn unlimited_never_waits_for_a_large_transfer() {
        let bucket = TokenBucket::unlimited();
        let start = tokio::time::Instant::now();
        bucket.consume(10_000_000_000).await;
        assert_eq!(tokio::time::Instant::now(), start);
    }

    #[tokio::test(start_paused = true)]
    async fn consuming_zero_bytes_never_waits() {
        let bucket = TokenBucket::new(1.0, 1);
        bucket.consume(1).await; // drain the single token
        let start = tokio::time::Instant::now();
        bucket.consume(0).await;
        assert_eq!(tokio::time::Instant::now(), start);
    }

    /// Regression test for the exact bug this method's doc comment
    /// describes: an earlier version of `consume` looped forever whenever
    /// `n` exceeded `capacity`, because `refill()` never lets `tokens`
    /// exceed `capacity`, so `tokens >= n` was never true. A single request
    /// larger than the whole burst capacity must still be paced at the
    /// sustained rate, not hang.
    #[tokio::test(start_paused = true)]
    async fn a_single_request_larger_than_capacity_is_paced_not_stuck() {
        let bucket = TokenBucket::new(1_000.0, 100); // 1000 B/s, only a 100 B burst
        let start = tokio::time::Instant::now();
        bucket.consume(1_100).await; // 100 free, 1000 more at 1000 B/s => 1s
        let elapsed = tokio::time::Instant::now() - start;
        assert!(
            elapsed >= Duration::from_millis(950) && elapsed <= Duration::from_millis(1_050),
            "elapsed {elapsed:?} not close to 1s"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn concurrent_callers_queue_behind_each_others_debt() {
        let bucket = TokenBucket::new(1_000.0, 0); // 1000 B/s, no free burst
        let a = bucket.clone();
        let b = bucket.clone();
        let start = tokio::time::Instant::now();
        // Both want 1000 B at once; between them that is 2s of sustained
        // transfer at 1000 B/s, so whichever finishes last must do so at
        // very nearly the 2s mark, not both finishing at ~1s as if they
        // were independent.
        let (_, _) = tokio::join!(a.consume(1_000), b.consume(1_000));
        let elapsed = tokio::time::Instant::now() - start;
        assert!(
            elapsed >= Duration::from_millis(1_950) && elapsed <= Duration::from_millis(2_050),
            "elapsed {elapsed:?} should reflect both requests sharing one bucket, not ~1s each independently"
        );
    }
}
