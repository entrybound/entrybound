//! Process self-measurement: this process's own peak resident memory.
//!
//! The target semantics are the high-water mark, matching what the Python
//! baselines record for external tools via `/usr/bin/time -v`
//! (`research/raw/*/*.jsonl` `resource.max_rss_bytes`, parsed from GNU time's
//! report): a peak, not a snapshot at the moment of the call. Do not use those
//! rows' `measurement.rusage.ru_maxrss_kib`: it is `wait4` on the `time`
//! process, which inherits the Python runner's own high-water mark at `exec`
//! (harness review round 1, finding R1-16).
//!
//! - **Linux:** [`nix::sys::resource::getrusage`], a safe wrapper (no
//!   `unsafe` at the call site) over libc's `getrusage(2)`; `max_rss()` is
//!   already reported in kibibytes on Linux. Caveat found in harness review
//!   round 1: a process started with `posix_spawn`/`vfork` (which Rust's
//!   `std::process::Command` uses on Linux) has its parent's memory
//!   high-water mark folded into this counter at `exec`, so `max_rss` of a
//!   small child spawned by a large parent reports the parent's peak. Use
//!   [`MemoryScope`] (`/proc/self/status` `VmHWM`, which belongs to the
//!   child's own address space) for a child's own peak.
//! - **Windows:** there is, as far as this crate's author could establish, no
//!   published crate whose public API reaches `GetProcessMemoryInfo`'s
//!   `PeakWorkingSetSize` without an `unsafe` call at the site -- the
//!   `windows`/`windows-sys` crates' bindings for it are `pub unsafe fn`, and
//!   `sysinfo` (used here) and `simple-process-stats` only expose *current*,
//!   not peak, memory. Since this workspace forbids `unsafe_code`
//!   (`research/harness/Cargo.toml`), this module reports CURRENT resident
//!   memory on Windows via `sysinfo` instead of peak, and says so honestly
//!   through [`PeakMemory::is_true_peak`] and [`PeakMemory::method`] rather
//!   than silently mislabeling a snapshot as a high-water mark. Revisit if a
//!   safe peak-memory crate appears; a narrowly-scoped, separately-reviewed
//!   FFI shim crate is also possible but is a deliberate choice this crate
//!   does not make unilaterally.

/// One memory reading for the calling process.
#[derive(Debug, Clone, Copy)]
pub struct PeakMemory {
    pub bytes: u64,
    /// `true` when `bytes` is an OS-reported high-water mark; `false` when it
    /// is only a current snapshot (see the module docs: Windows today).
    pub is_true_peak: bool,
    pub method: &'static str,
}

#[cfg(target_os = "linux")]
pub fn peak_memory() -> Result<PeakMemory, String> {
    use nix::sys::resource::{UsageWho, getrusage};
    let usage = getrusage(UsageWho::RUSAGE_SELF).map_err(|e| format!("getrusage: {e}"))?;
    let kib = u64::try_from(usage.max_rss().max(0)).unwrap_or(0);
    Ok(PeakMemory {
        bytes: kib.saturating_mul(1024),
        is_true_peak: true,
        method: "getrusage(RUSAGE_SELF).max_rss",
    })
}

#[cfg(target_os = "windows")]
pub fn peak_memory() -> Result<PeakMemory, String> {
    use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};
    let pid = sysinfo::get_current_pid().map_err(|e| format!("get_current_pid: {e}"))?;
    let mut system = System::new();
    system.refresh_processes_specifics(
        ProcessesToUpdate::Some(&[pid]),
        false,
        ProcessRefreshKind::nothing().with_memory(),
    );
    let process = system
        .process(pid)
        .ok_or_else(|| "sysinfo has no record of the current process".to_string())?;
    Ok(PeakMemory {
        bytes: process.memory(),
        is_true_peak: false,
        method: "sysinfo current resident memory (peak working set needs an unsafe FFI call; see module docs)",
    })
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn peak_memory() -> Result<PeakMemory, String> {
    Err("peak memory measurement is not implemented for this platform".to_string())
}

// ---------------------------------------------------------------------------
// Phase-scoped peak memory (harness review round 1, finding R1-03/R1-05)
// ---------------------------------------------------------------------------
//
// `getrusage(RUSAGE_SELF).max_rss` is a *process-lifetime* high-water mark:
// once a harness binary has held a large buffer (an in-memory corpus copy, a
// planned `Archive` with every Chunk's plaintext, a previous stage's
// allocations), every later reading reports that earlier peak, so it cannot
// attribute a peak to the phase being measured. On Linux >= 4.0 the kernel
// lets a process reset its own high-water mark to its current resident set
// by writing `5` to `/proc/self/clear_refs` (proc(5)), after which
// `/proc/self/status`'s `VmHWM` tracks only the peak since the reset. That
// is a plain file write and read, so it needs no `unsafe` code.
//
// What a scoped reading does and does not mean:
//
// - `peak_rss_bytes` is the OS high-water mark since `begin`, which includes
//   everything already resident at `begin` (`baseline_rss_bytes`).
// - `peak_delta_bytes = peak - baseline` is the growth the phase caused. It
//   can *under*-state the phase's working set when the allocator reuses
//   memory that an earlier phase freed but did not return to the OS, so a
//   decision-grade memory number should still come from a phase run in its
//   own process (see `ebr-access --probe-isolate`, and one candidate per
//   process for `ebr-codec --candidate`). The scoped reading is still far
//   closer to the phase than a lifetime peak.
// - On Windows (and when the reset is refused) `scoped` is `false`: the
//   reading is a current-resident snapshot at `end`, never a peak.

/// A phase-scoped memory reading. See the comment block above for semantics.
#[derive(Debug, Clone, Copy, serde::Serialize)]
pub struct ScopedMemory {
    /// Resident memory when the scope began.
    pub baseline_rss_bytes: u64,
    /// High-water mark since the scope began (`scoped == true`), or a current
    /// snapshot at the end of the scope (`scoped == false`).
    pub peak_rss_bytes: u64,
    /// `peak_rss_bytes.saturating_sub(baseline_rss_bytes)`.
    pub peak_delta_bytes: u64,
    /// `true` only when the kernel high-water mark was reset at `begin` and
    /// read back at `end`.
    pub scoped: bool,
    pub method: &'static str,
}

/// Resident memory right now (`VmRSS` on Linux, `sysinfo` on Windows).
pub fn current_rss_bytes() -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        read_status_kib("VmRSS:").map(|kib| kib.saturating_mul(1024))
    }
    #[cfg(target_os = "windows")]
    {
        peak_memory().ok().map(|sample| sample.bytes)
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        None
    }
}

#[cfg(target_os = "linux")]
fn read_status_kib(key: &str) -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    status.lines().find_map(|line| {
        line.strip_prefix(key)
            .and_then(|rest| rest.trim().strip_suffix("kB"))
            .and_then(|value| value.trim().parse::<u64>().ok())
    })
}

#[cfg(target_os = "linux")]
fn high_water_mark_bytes() -> Option<u64> {
    read_status_kib("VmHWM:").map(|kib| kib.saturating_mul(1024))
}

#[cfg(not(target_os = "linux"))]
fn high_water_mark_bytes() -> Option<u64> {
    None
}

/// Resets the kernel's resident-set high-water mark for this process to its
/// current resident set. Returns `false` when the platform or kernel does not
/// support it (in which case later scoped readings report `scoped: false`).
pub fn reset_peak_rss() -> bool {
    #[cfg(target_os = "linux")]
    {
        std::fs::write("/proc/self/clear_refs", b"5").is_ok()
    }
    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}

/// An open memory scope. Create with [`MemoryScope::begin`] immediately
/// before the measured phase and close with [`MemoryScope::end`] immediately
/// after it, before any harness-only work (verification passes, byte
/// accounting, serialization) runs.
#[derive(Debug, Clone, Copy)]
pub struct MemoryScope {
    baseline_rss_bytes: u64,
    reset: bool,
}

impl MemoryScope {
    pub fn begin() -> Self {
        let reset = reset_peak_rss();
        MemoryScope {
            baseline_rss_bytes: current_rss_bytes().unwrap_or(0),
            reset,
        }
    }

    pub fn end(self) -> ScopedMemory {
        if self.reset
            && let Some(hwm) = high_water_mark_bytes()
        {
            let peak = hwm.max(self.baseline_rss_bytes);
            return ScopedMemory {
                baseline_rss_bytes: self.baseline_rss_bytes,
                peak_rss_bytes: peak,
                peak_delta_bytes: peak.saturating_sub(self.baseline_rss_bytes),
                scoped: true,
                method: "/proc/self/clear_refs=5 reset, then /proc/self/status VmHWM",
            };
        }
        let current = current_rss_bytes().unwrap_or(0);
        ScopedMemory {
            baseline_rss_bytes: self.baseline_rss_bytes,
            peak_rss_bytes: current,
            peak_delta_bytes: current.saturating_sub(self.baseline_rss_bytes),
            scoped: false,
            method: "current resident snapshot at scope end (no resettable high-water mark on this platform)",
        }
    }
}

/// Runs `f` inside a [`MemoryScope`] and returns its value, the scope's
/// memory reading, and its monotonic wall-clock seconds.
pub fn measure_scoped<T>(f: impl FnOnce() -> T) -> (T, ScopedMemory, f64) {
    let scope = MemoryScope::begin();
    let clock = crate::timing::Stopwatch::start();
    let value = f();
    let seconds = clock.elapsed_secs_f64();
    (value, scope.end(), seconds)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(any(target_os = "linux", target_os = "windows"))]
    #[test]
    fn peak_memory_reports_a_plausible_nonzero_value() {
        // Allocate and touch a few MiB first so there is something to see.
        let mut v = vec![0u8; 8 * 1024 * 1024];
        for (i, b) in v.iter_mut().enumerate() {
            *b = i as u8;
        }
        std::hint::black_box(&v);

        let sample =
            peak_memory().expect("peak_memory should succeed on Linux and Windows CI hosts");
        assert!(
            sample.bytes > 0,
            "expected a nonzero memory reading, got {}",
            sample.bytes
        );
        v.clear();
    }

    /// A scope that follows a large, already-freed allocation must not
    /// report that earlier peak: this is exactly the contamination a
    /// process-lifetime `max_rss` suffers from.
    #[cfg(target_os = "linux")]
    #[test]
    fn scoped_peak_excludes_an_earlier_larger_allocation() {
        let big = vec![1u8; 256 * 1024 * 1024];
        std::hint::black_box(&big);
        drop(big); // 256 MiB is mmap-backed, so it is returned to the OS here
        let lifetime = peak_memory().unwrap().bytes;
        assert!(lifetime >= 256 * 1024 * 1024);

        let (_, scoped, _) = measure_scoped(|| {
            let small = vec![2u8; 8 * 1024 * 1024];
            std::hint::black_box(&small);
            small.len()
        });
        assert!(
            scoped.scoped,
            "clear_refs reset should be supported on Linux >= 4.0"
        );
        assert!(
            scoped.peak_rss_bytes < lifetime,
            "scoped peak {} should not include the earlier 256 MiB peak {}",
            scoped.peak_rss_bytes,
            lifetime
        );
        assert!(
            scoped.peak_delta_bytes >= 7 * 1024 * 1024,
            "scoped delta {} should see the 8 MiB allocation",
            scoped.peak_delta_bytes
        );
    }

    #[test]
    fn current_rss_is_reported_on_supported_platforms() {
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        assert!(current_rss_bytes().unwrap_or(0) > 0);
    }
}
