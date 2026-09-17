//! Peak on-disk scratch usage, sampled while `ebr-unpack` extracts an
//! archive.
//!
//! Nothing in the operating system tracks a directory's peak size, so this
//! module samples on a background thread while extraction runs on the
//! caller's thread and keeps the largest totals it observed -- a real, if
//! discrete-interval, measurement rather than an estimate.
//!
//! # What is sampled (harness review round 1, finding R1-04)
//!
//! Each sample sums two places:
//!
//! 1. **the destination directory** (recursive; regular files' apparent
//!    length, plus allocated blocks on Unix), and
//! 2. **production's own sequential-reader staging spill files**,
//!    `entrybound-stage-<pid>-*.tmp` directly under `std::env::temp_dir()`
//!    (`crates/entrybound/src/ecf/staging.rs`'s `SpillFile::create`), which
//!    STREAM extraction writes once staged plaintext exceeds the in-memory
//!    staging limit (256 MiB with `bootstrap_sequential_limits`). Earlier
//!    revisions sampled only the destination and so missed STREAM's largest
//!    scratch consumer entirely.
//!
//! `peak_total_bytes` is the largest *per-sample* sum of the two, not the sum
//! of their separate peaks. For STREAM, `StreamReport::spilled_staging_bytes`
//! (production's own counter) is reported next to it by `crate::unpack` as
//! an independent check.
//!
//! # Sampler overhead
//!
//! A sample walks the whole destination tree, so its cost grows with the
//! number of extracted files. To keep the sampler from competing with a
//! many-small-files extraction, the sleep between samples is at least four
//! times the previous walk's own duration (so the sampler spends at most
//! about 20% of wall time walking), and the total walk time is reported as
//! `sampler_walk_seconds` so a caller can see how much it cost.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// One completed sampling run's result.
#[derive(Debug, Clone, Copy, Default, serde::Serialize)]
pub struct PeakScratch {
    /// Largest destination-directory apparent size seen.
    pub peak_bytes: u64,
    /// Largest destination-directory allocated size seen (Unix `st_blocks`;
    /// equals `peak_bytes` elsewhere).
    pub peak_allocated_bytes: u64,
    /// Largest total size of this process's staging spill files seen.
    pub peak_staging_spill_bytes: u64,
    /// Largest per-sample `destination + staging spill` total seen.
    pub peak_total_bytes: u64,
    pub sample_count: u64,
    pub sampler_walk_seconds: f64,
}

#[derive(Debug, Clone, Copy, Default)]
struct Sample {
    apparent: u64,
    allocated: u64,
    spill: u64,
}

/// A background sampler over one destination directory. Construct with
/// [`start`], finish with [`stop`].
///
/// [`start`]: ScratchSampler::start
/// [`stop`]: ScratchSampler::stop
pub struct ScratchSampler {
    stop: Arc<AtomicBool>,
    result: Arc<Mutex<PeakScratch>>,
    handle: Option<JoinHandle<()>>,
}

impl ScratchSampler {
    /// Starts sampling `dir` (and this process's staging spill files) every
    /// `interval` at least, from a fresh background thread. `dir` need not
    /// exist yet; a missing directory samples as zero bytes.
    #[must_use]
    pub fn start(dir: PathBuf, interval: Duration) -> Self {
        Self::start_with_spill_dir(dir, std::env::temp_dir(), interval)
    }

    /// As [`start`](Self::start), with an explicit directory to look for
    /// `entrybound-stage-<pid>-*` spill files in.
    #[must_use]
    pub fn start_with_spill_dir(dir: PathBuf, spill_dir: PathBuf, interval: Duration) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let result = Arc::new(Mutex::new(PeakScratch::default()));
        let stop_clone = Arc::clone(&stop);
        let result_clone = Arc::clone(&result);
        let spill_prefix = format!("entrybound-stage-{}-", std::process::id());
        let handle = std::thread::spawn(move || {
            loop {
                let stopping = stop_clone.load(Ordering::Relaxed);
                let walk_started = Instant::now();
                let sample = Sample {
                    spill: spill_bytes(&spill_dir, &spill_prefix),
                    ..directory_bytes(&dir)
                };
                let walk = walk_started.elapsed();
                record(&result_clone, sample, walk);
                // One final sample after the caller signals completion, so a
                // fast extraction that finished between two sleeps is not
                // measured only at zero.
                if stopping {
                    break;
                }
                std::thread::sleep(interval.max(walk.saturating_mul(4)));
            }
        });
        ScratchSampler {
            stop,
            result,
            handle: Some(handle),
        }
    }

    /// The minimum interval [`start`](Self::start) uses when a caller has no
    /// specific reason to pick another one (adaptively lengthened for large
    /// trees; see the module docs).
    #[must_use]
    pub const fn default_interval() -> Duration {
        Duration::from_millis(2)
    }

    /// Signals the sampler to stop, joins its thread, and returns the peaks
    /// observed. Panics only if the sampling thread itself panicked.
    #[must_use]
    pub fn stop(mut self) -> PeakScratch {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            handle
                .join()
                .expect("scratch sampler thread should not panic");
        }
        *self
            .result
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

fn record(result: &Mutex<PeakScratch>, sample: Sample, walk: Duration) {
    let mut peak = result
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    peak.peak_bytes = peak.peak_bytes.max(sample.apparent);
    peak.peak_allocated_bytes = peak.peak_allocated_bytes.max(sample.allocated);
    peak.peak_staging_spill_bytes = peak.peak_staging_spill_bytes.max(sample.spill);
    peak.peak_total_bytes = peak
        .peak_total_bytes
        .max(sample.apparent.saturating_add(sample.spill));
    peak.sample_count += 1;
    peak.sampler_walk_seconds += walk.as_secs_f64();
}

/// Total bytes of every regular file under `dir`, recursively. Missing
/// directories and races against concurrent extraction (a file listed by
/// `read_dir` but removed/renamed before its metadata is read) count as zero
/// for that entry: a transient undercount is the correct behavior for a
/// *peak* sampler racing a writer, since the peak is retained across samples.
fn directory_bytes(dir: &Path) -> Sample {
    let mut sample = Sample::default();
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
            } else if file_type.is_file()
                && let Ok(metadata) = current_metadata(&entry)
            {
                sample.apparent = sample.apparent.saturating_add(metadata.len());
                sample.allocated = sample.allocated.saturating_add(allocated_len(&metadata));
            }
        }
    }
    sample
}

/// A directory entry's *current* metadata. On Windows, `DirEntry::metadata`
/// returns the directory enumeration's cached size, which stays stale (often
/// zero) for a file another handle is still writing -- exactly the state of a
/// staging spill file or a file mid-extraction -- so the path is queried
/// directly there. Unix `DirEntry::metadata` already calls `lstat`.
#[cfg(windows)]
fn current_metadata(entry: &std::fs::DirEntry) -> std::io::Result<std::fs::Metadata> {
    std::fs::symlink_metadata(entry.path())
}

#[cfg(not(windows))]
fn current_metadata(entry: &std::fs::DirEntry) -> std::io::Result<std::fs::Metadata> {
    entry.metadata()
}

#[cfg(unix)]
fn allocated_len(metadata: &std::fs::Metadata) -> u64 {
    use std::os::unix::fs::MetadataExt;
    metadata.blocks().saturating_mul(512)
}

#[cfg(not(unix))]
fn allocated_len(metadata: &std::fs::Metadata) -> u64 {
    metadata.len()
}

/// Total apparent size of `prefix*` regular files directly under `dir`.
fn spill_bytes(dir: &Path, prefix: &str) -> u64 {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };
    entries
        .flatten()
        .filter(|entry| entry.file_name().to_string_lossy().starts_with(prefix))
        .filter_map(|entry| current_metadata(&entry).ok())
        .filter(std::fs::Metadata::is_file)
        .map(|metadata| metadata.len())
        .fold(0u64, u64::saturating_add)
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
        assert_eq!(directory_bytes(&dir).apparent, 0);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn sums_nested_file_bytes() {
        let dir = scratch_dir("nested");
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        std::fs::write(dir.join("a.bin"), vec![0u8; 100]).unwrap();
        std::fs::write(dir.join("sub").join("b.bin"), vec![0u8; 250]).unwrap();
        assert_eq!(directory_bytes(&dir).apparent, 350);
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
    fn sampler_counts_this_process_staging_spill_files_and_ignores_others() {
        let dest = scratch_dir("spill-dest");
        let spill_dir = scratch_dir("spill-dir");
        let pid = std::process::id();
        std::fs::write(dest.join("out.bin"), vec![0u8; 1000]).unwrap();
        std::fs::write(
            spill_dir.join(format!("entrybound-stage-{pid}-0-1-0.tmp")),
            vec![0u8; 5000],
        )
        .unwrap();
        std::fs::write(
            spill_dir.join("entrybound-stage-999999999-0-1-0.tmp"),
            vec![0u8; 7000],
        )
        .unwrap();
        let sampler = ScratchSampler::start_with_spill_dir(
            dest.clone(),
            spill_dir.clone(),
            Duration::from_millis(1),
        );
        std::thread::sleep(Duration::from_millis(10));
        let result = sampler.stop();
        assert_eq!(result.peak_bytes, 1000);
        assert_eq!(result.peak_staging_spill_bytes, 5000);
        assert_eq!(result.peak_total_bytes, 6000);
        std::fs::remove_dir_all(&dest).ok();
        std::fs::remove_dir_all(&spill_dir).ok();
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
