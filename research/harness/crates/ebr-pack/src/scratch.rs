//! Peak on-disk scratch-directory byte usage, sampled while `ebr-unpack`
//! extracts an archive.
//!
//! Unlike [`ebr_common::measure::peak_memory`] (an OS-reported high-water
//! mark), nothing in the operating system tracks a directory's peak size for
//! us: extraction only ever grows the directory, then the caller measures
//! and removes it. This module samples the directory's total size on a
//! background thread at a fixed interval while extraction runs on the
//! caller's thread, and reports the largest total it observed -- a real,
//! if necessarily discrete-interval, measurement, not a computed estimate.
//! A finer interval catches a higher true peak at the cost of more sampling
//! overhead; see [`ScratchSampler::start`]'s default.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

/// One completed sampling run's result.
#[derive(Debug, Clone, Copy)]
pub struct PeakScratch {
    pub peak_bytes: u64,
    pub sample_count: u64,
}

/// A background sampler over one directory. Construct with [`start`],
/// finish with [`stop`].
///
/// [`start`]: ScratchSampler::start
/// [`stop`]: ScratchSampler::stop
pub struct ScratchSampler {
    stop: Arc<AtomicBool>,
    peak_bytes: Arc<AtomicU64>,
    sample_count: Arc<AtomicU64>,
    handle: Option<JoinHandle<()>>,
}

impl ScratchSampler {
    /// Starts sampling `dir`'s total size every `interval`, from a fresh
    /// background thread. `dir` need not exist yet (extraction may not have
    /// created it the instant this is called); a missing directory samples
    /// as zero bytes rather than erroring.
    #[must_use]
    pub fn start(dir: PathBuf, interval: Duration) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let peak_bytes = Arc::new(AtomicU64::new(0));
        let sample_count = Arc::new(AtomicU64::new(0));

        let stop_clone = Arc::clone(&stop);
        let peak_clone = Arc::clone(&peak_bytes);
        let count_clone = Arc::clone(&sample_count);
        let handle = std::thread::spawn(move || {
            while !stop_clone.load(Ordering::Relaxed) {
                let sampled = directory_bytes(&dir);
                peak_clone.fetch_max(sampled, Ordering::Relaxed);
                count_clone.fetch_add(1, Ordering::Relaxed);
                std::thread::sleep(interval);
            }
            // One final sample after the caller signals completion, so a
            // fast extraction that finished between two sleeps is not
            // measured only at zero.
            let sampled = directory_bytes(&dir);
            peak_clone.fetch_max(sampled, Ordering::Relaxed);
            count_clone.fetch_add(1, Ordering::Relaxed);
        });

        ScratchSampler {
            stop,
            peak_bytes,
            sample_count,
            handle: Some(handle),
        }
    }

    /// The interval [`start`](Self::start) uses when a caller has no
    /// specific reason to pick another one: fine enough to catch a peak in a
    /// tuning-scale corpus item's extraction, coarse enough that the
    /// sampling thread's own overhead stays negligible.
    #[must_use]
    pub const fn default_interval() -> Duration {
        Duration::from_millis(2)
    }

    /// Signals the sampler to stop, joins its thread, and returns the peak
    /// observed. Panics only if the sampling thread itself panicked, which
    /// would indicate a bug in this module, not in the extraction it watched.
    #[must_use]
    pub fn stop(mut self) -> PeakScratch {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            handle.join().expect("scratch sampler thread should not panic");
        }
        PeakScratch {
            peak_bytes: self.peak_bytes.load(Ordering::Relaxed),
            sample_count: self.sample_count.load(Ordering::Relaxed),
        }
    }
}

/// Total bytes of every regular file under `dir`, recursively. Missing
/// directories and races against concurrent extraction (a file listed by
/// `read_dir` but removed/renamed before its metadata is read) both count as
/// zero for that entry rather than failing the whole sample: a transient
/// undercount is the correct behavior for a *peak* sampler racing a writer,
/// since the peak is retained across samples via `fetch_max`.
fn directory_bytes(dir: &std::path::Path) -> u64 {
    let mut total = 0u64;
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&current) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_dir() {
                stack.push(entry.path());
            } else if file_type.is_file() {
                total = total.saturating_add(entry.metadata().map(|m| m.len()).unwrap_or(0));
            }
        }
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "ebr-pack-scratch-sampler-test-{label}-{}",
            std::process::id()
        ));
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn samples_zero_for_an_empty_directory() {
        let dir = scratch_dir("empty");
        assert_eq!(directory_bytes(&dir), 0);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn sums_nested_file_bytes() {
        let dir = scratch_dir("nested");
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        std::fs::write(dir.join("a.bin"), vec![0u8; 100]).unwrap();
        std::fs::write(dir.join("sub").join("b.bin"), vec![0u8; 250]).unwrap();
        assert_eq!(directory_bytes(&dir), 350);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn sampler_observes_growth_while_running() {
        let dir = scratch_dir("growth");
        let sampler = ScratchSampler::start(dir.clone(), Duration::from_millis(1));
        std::fs::write(dir.join("grown.bin"), vec![0u8; 4096]).unwrap();
        std::thread::sleep(Duration::from_millis(20));
        let result = sampler.stop();
        assert!(result.peak_bytes >= 4096, "peak was {}", result.peak_bytes);
        assert!(result.sample_count >= 1);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn missing_directory_samples_as_zero_without_panicking() {
        let dir = scratch_dir("missing-parent").join("does-not-exist");
        let sampler = ScratchSampler::start(dir, Duration::from_millis(1));
        std::thread::sleep(Duration::from_millis(5));
        let result = sampler.stop();
        assert_eq!(result.peak_bytes, 0);
    }
}
