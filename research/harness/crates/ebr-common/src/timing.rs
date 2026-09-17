//! Monotonic timing and id-generation helpers.
//!
//! Elapsed-time measurement in this crate only ever uses
//! [`std::time::Instant`] (via [`Stopwatch`]), never [`std::time::SystemTime`]
//! -- the latter can jump (NTP step, `SetSystemTime`, VM migration) and must
//! never be used to compute a duration. `SystemTime` is used only to *label*
//! a moment (a timestamp field, a run id), never to measure one.

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// A monotonic stopwatch. The only clock this crate uses for elapsed time.
#[derive(Debug, Clone, Copy)]
pub struct Stopwatch {
    start: Instant,
}

impl Stopwatch {
    pub fn start() -> Self {
        Stopwatch {
            start: Instant::now(),
        }
    }

    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }

    pub fn elapsed_secs_f64(&self) -> f64 {
        self.elapsed().as_secs_f64()
    }
}

impl Default for Stopwatch {
    fn default() -> Self {
        Self::start()
    }
}

/// Current wall-clock time as RFC 3339 UTC with second precision, e.g.
/// `"2026-09-17T03:12:01Z"`. Pure calendar arithmetic over `SystemTime`; no
/// timezone database and no `chrono`/`time` dependency.
pub fn now_utc_rfc3339() -> String {
    let epoch_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let (y, mo, d, h, mi, s) = civil_from_unix(epoch_secs);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z")
}

/// A run id shaped like the Python baselines use
/// (`research/raw/*/*.jsonl` `run_id`, e.g. `"20260917T030222Z-b5741c"`):
/// a compact UTC timestamp, then a 6-hex-character suffix. The suffix comes
/// from hashing the current time in nanoseconds together with the process id;
/// it exists only to keep run ids distinct when two runs start in the same
/// second, not for any cryptographic property.
pub fn generate_run_id() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let (y, mo, d, h, mi, s) = civil_from_unix(now.as_secs());
    let compact = format!("{y:04}{mo:02}{d:02}T{h:02}{mi:02}{s:02}Z");
    let mut seed = Vec::with_capacity(16);
    seed.extend_from_slice(&now.as_nanos().to_le_bytes());
    seed.extend_from_slice(&(std::process::id() as u64).to_le_bytes());
    let digest = crate::hash::sha256_hex(&seed);
    format!("{compact}-{}", &digest[..6])
}

/// Splits a Unix timestamp into proleptic-Gregorian UTC
/// `(year, month, day, hour, minute, second)`. Adapted from Howard Hinnant's
/// `civil_from_days` (public domain / CC0,
/// <https://howardhinnant.github.io/date_algorithms.html>), extended to also
/// split the time-of-day. Valid for every date this program will ever see.
fn civil_from_unix(epoch_secs: u64) -> (i64, u32, u32, u32, u32, u32) {
    let days = (epoch_secs / 86400) as i64;
    let secs_of_day = epoch_secs % 86400;
    let (h, mi, s) = (
        (secs_of_day / 3600) as u32,
        ((secs_of_day % 3600) / 60) as u32,
        (secs_of_day % 60) as u32,
    );

    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365; // [0, 399]
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32; // [1, 12]
    let y = yoe as i64 + era * 400 + i64::from(m <= 2);
    (y, m, d, h, mi, s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_zero_is_1970_01_01() {
        assert_eq!(civil_from_unix(0), (1970, 1, 1, 0, 0, 0));
    }

    #[test]
    fn known_timestamp_round_trips() {
        // 2026-09-17T03:12:01Z, computed independently in Python:
        // datetime.datetime(2026, 9, 17, 3, 12, 1, tzinfo=timezone.utc).timestamp()
        let epoch = 1_789_614_721u64;
        assert_eq!(civil_from_unix(epoch), (2026, 9, 17, 3, 12, 1));
    }

    #[test]
    fn stopwatch_elapsed_is_monotonic_and_nonzero_eventually() {
        let sw = Stopwatch::start();
        std::thread::sleep(Duration::from_millis(5));
        assert!(sw.elapsed() >= Duration::from_millis(1));
    }

    #[test]
    fn run_id_has_the_expected_shape() {
        let id = generate_run_id();
        let (timestamp, suffix) = id.split_once('-').expect("run id has a '-'");
        assert_eq!(timestamp.len(), "20260917T030222Z".len());
        assert!(timestamp.ends_with('Z'));
        assert_eq!(suffix.len(), 6);
        assert!(suffix.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
