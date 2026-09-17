//! Process self-measurement: this process's own peak resident memory.
//!
//! The target semantics are the high-water mark, matching what the Python
//! baselines already record for external tools via `/usr/bin/time -v`
//! (`research/raw/*/*.jsonl` `measurement.rusage.ru_maxrss_kib`): a peak, not
//! a snapshot at the moment of the call.
//!
//! - **Linux:** [`nix::sys::resource::getrusage`], a safe wrapper (no
//!   `unsafe` at the call site) over libc's `getrusage(2)`; `max_rss()` is
//!   already reported in kibibytes on Linux.
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
}
