//! Streaming memory probe: runs a sequence of entrybound operations over a
//! synthetic, generated-on-the-fly input of a caller-chosen size, sampling
//! this process's own memory (`ebr_common::measure::peak_memory`) on a
//! background thread at a fixed interval throughout each stage, to see how
//! that stage's resident memory scales with total plaintext.
//!
//! ## What this probe found (and is designed to keep surfacing)
//!
//! `entrybound::archive::plan_directory` (which every stage below builds
//! on) fully materializes every Chunk's plaintext into the returned
//! `Archive` before any encoder runs -- `entrybound::eam::Chunk::plaintext`
//! is a `Box<[u8]>` held in `ContentStore.chunks`, for every Chunk, for as
//! long as the `Archive` value is alive. That is true regardless of which
//! layout is requested afterward, INDEXED or STREAM: STREAM only changes
//! physical frame order, not whether the plan holds plaintext. So **`pack`
//! costs at least `total_bytes` of process memory today, streaming sink or
//! not** -- this probe exists to make that scaling directly observable (the
//! `pack` stage's `peak_bytes_observed` should track `requested_total_bytes`
//! roughly linearly) rather than to disprove or route around it; changing
//! that behavior would be a production semantics change, which is out of
//! scope for this crate (see `research/PROGRESS.md`'s standing constraints).
//!
//! ## How stage memory is measured (harness review round 1, finding R1-05)
//!
//! Earlier revisions sampled `getrusage(RUSAGE_SELF).max_rss`, a
//! process-lifetime high-water mark, and kept the planned `Archive` (every
//! Chunk's plaintext) alive through every later stage. Every stage after
//! `pack` therefore reported `pack`'s peak, and `verify`/`unpack` could
//! never show bounded streaming memory. Now:
//!
//! - each stage's `memory` is a [`ScopedMemory`] whose kernel high-water
//!   mark is reset at the start of the stage (Linux), and `samples` are
//!   the *current* resident set over time;
//! - `repack`/`export` (which need the planned archive) run right after
//!   `pack`, and the archive is dropped before `verify`/`unpack`;
//! - with `isolate_exe` (`ebr-access --probe-isolate true`), `verify` and
//!   `unpack` each run in a fresh child process, so allocator memory the
//!   `pack` stage freed but did not return to the OS cannot hide or inflate
//!   their footprint. Decision-grade streaming-memory claims must use the
//!   isolated mode.
//!
//! ## What this probe cannot avoid storing
//!
//! `plan_directory`/`pack_directory*` take a real directory (`&Path`), not
//! an arbitrary `Read` stream -- there is no production entry point that
//! plans an archive from in-memory or piped bytes alone. So this probe's
//! [`SyntheticGenerator`] (a bounded-memory, seed-derived byte generator)
//! writes its output to one **scratch file** before `pack` ever runs; that
//! on-disk footprint is `scratch_input_bytes` in [`ProbeMeasurement`], and
//! is what "detect scaling with total plaintext" is measured against on the
//! disk side. The `pack` stage's STREAM output is likewise written straight
//! to a second scratch file, never buffered whole in memory by this probe
//! (though, per the finding above, entrybound's own planning step already
//! holds the plaintext in memory regardless of what this probe does with
//! the encoded bytes afterward).
//!
//! ## Stages
//!
//! - **pack** (always runs -- every other stage needs its output):
//!   `entrybound::archive::plan_directory` then
//!   `entrybound::ecf::encode_stream` to the scratch STREAM file.
//! - **verify**: `entrybound::ecf::verify_stream_with_limits`, reading the
//!   scratch STREAM file back.
//! - **unpack**: `entrybound::archive::unpack_stream` into a scratch
//!   extraction directory, removed again once the stage ends.
//! - **repack** (opt-in -- see below): `entrybound::ecf::encode` (INDEXED,
//!   fully in memory) + `entrybound::ecf::open` +
//!   `entrybound::archive::prepare_repack`.
//! - **export** (opt-in): `entrybound::legacy::export::prepare_export` to
//!   `tar/pax-v1`, fully in memory.
//!
//! `repack` and `export` are opt-in only ([`default_stages`] excludes them):
//! both require a second fully in-memory encoding of the whole archive on
//! top of `pack`'s own (INDEXED bytes for `repack`, a legacy target's bytes
//! for `export`), so including them roughly doubles a run's peak memory
//! and is a materially different question from the streaming layouts
//! `pack`/`verify`/`unpack` cover.
//!
//! There is no **publish** stage: entrybound's public API has no operation
//! named or shaped like a "publish" step (nothing beyond `encode`/
//! `encode_stream`, which `pack` already measures), so this probe does not
//! invent one.
//!
//! ## Actually running this at 1-64 GiB
//!
//! This crate's own tests only ever request a few hundred KiB (see
//! `tests` below) -- enough to prove the mechanism -- and every wall-clock
//! number this crate has produced so far is non-decision-grade smoke timing
//! (`research/PROGRESS.md`'s shared Phase B2 agent rules). Actually running
//! the full 1-64 GiB sweep this probe is built for, on a machine that is not
//! otherwise busy, and recording it as decision-grade timing, is future
//! C-exec work.

use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use ebr_common::measure::{MemoryScope, ScopedMemory, current_rss_bytes};
use ebr_common::timing::Stopwatch;
use entrybound::archive::{
    ExtractionPolicy, IndexPolicy, PackOptions, RepackMode, RepackOptions, plan_directory,
    prepare_repack, unpack_stream,
};
use entrybound::diagnostics::Diagnostic;
use entrybound::eam::{Archive, Layout};
use entrybound::ecf::{
    StreamWindow, StreamWriteOptions, WriteOptions, bootstrap_sequential_limits, encode,
    encode_stream, open, verify_stream_with_limits,
};
use entrybound::legacy::export::{ExportSourceSecurity, ExportTarget, prepare_export};
use entrybound::planner::CompressionProfile;
use serde::Serialize;

use crate::rng::SplitMix64;

// ---------------------------------------------------------------------------
// Synthetic generator
// ---------------------------------------------------------------------------

/// Generates `total_bytes` of deterministic, seed-derived synthetic
/// plaintext. Implements [`Read`], and never materializes more than the
/// caller's own read buffer at a time -- there is no internal `Vec`/`Box<[u8]>`
/// sized to `total_bytes` anywhere in this type, however large `total_bytes`
/// is.
pub struct SyntheticGenerator {
    remaining: u64,
    rng: SplitMix64,
}

impl SyntheticGenerator {
    #[must_use]
    pub const fn new(total_bytes: u64, seed: u64) -> Self {
        Self {
            remaining: total_bytes,
            rng: SplitMix64::new(seed),
        }
    }
}

impl Read for SyntheticGenerator {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.remaining == 0 || buf.is_empty() {
            return Ok(0);
        }
        let want = if u64::try_from(buf.len()).is_ok_and(|len| len <= self.remaining) {
            buf.len()
        } else {
            usize::try_from(self.remaining).unwrap_or(buf.len())
        };
        let mut written = 0;
        while written < want {
            let word = self.rng.next_u64().to_le_bytes();
            let take = word.len().min(want - written);
            buf[written..written + take].copy_from_slice(&word[..take]);
            written += take;
        }
        self.remaining -= written as u64;
        Ok(written)
    }
}

/// Writes exactly `total_bytes` of [`SyntheticGenerator`] output to `path`
/// through a buffered writer, so the file grows incrementally rather than
/// through one `total_bytes`-sized in-memory buffer.
pub fn write_synthetic_file(path: &Path, total_bytes: u64, seed: u64) -> io::Result<()> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    let mut generator = SyntheticGenerator::new(total_bytes, seed);
    io::copy(&mut generator, &mut writer)?;
    writer.flush()
}

// ---------------------------------------------------------------------------
// Memory sampler
// ---------------------------------------------------------------------------

/// One resident-memory reading taken during a probe stage, at
/// `elapsed_seconds` since that stage began. `rss_bytes` is the *current*
/// resident set (`VmRSS` on Linux), not a high-water mark, so the series
/// shows how memory evolves within the stage.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct MemorySample {
    pub elapsed_seconds: f64,
    pub rss_bytes: Option<u64>,
}

/// Samples [`current_rss_bytes`] on a background thread at a fixed interval
/// until [`MemorySampler::stop`] is called.
struct MemorySampler {
    stop: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<Vec<MemorySample>>>,
}

impl MemorySampler {
    fn start(interval: Duration) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let stop_flag = Arc::clone(&stop);
        let stopwatch = Stopwatch::start();
        let handle = thread::spawn(move || {
            let mut samples = Vec::new();
            loop {
                samples.push(MemorySample {
                    elapsed_seconds: stopwatch.elapsed_secs_f64(),
                    rss_bytes: current_rss_bytes(),
                });
                if stop_flag.load(Ordering::Relaxed) {
                    break;
                }
                thread::sleep(interval);
            }
            samples
        });
        Self {
            stop,
            handle: Some(handle),
        }
    }

    /// Signals the background thread to stop (it takes one final sample
    /// before exiting) and joins it, returning every sample it took, in
    /// chronological order. A poisoned sampler thread yields no samples
    /// rather than panicking the caller.
    fn stop(mut self) -> Vec<MemorySample> {
        self.stop.store(true, Ordering::Relaxed);
        self.handle
            .take()
            .and_then(|handle| handle.join().ok())
            .unwrap_or_default()
    }
}

// ---------------------------------------------------------------------------
// Stages and configuration
// ---------------------------------------------------------------------------

/// One operation this probe can run over the synthetic input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProbeStage {
    Pack,
    Verify,
    Unpack,
    Repack,
    Export,
}

impl ProbeStage {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pack => "pack",
            Self::Verify => "verify",
            Self::Unpack => "unpack",
            Self::Repack => "repack",
            Self::Export => "export",
        }
    }

    /// Parses a stage name as accepted on the `ebr-access` command line
    /// (`--probe-stages pack,verify,unpack`).
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "pack" => Ok(Self::Pack),
            "verify" => Ok(Self::Verify),
            "unpack" => Ok(Self::Unpack),
            "repack" => Ok(Self::Repack),
            "export" => Ok(Self::Export),
            other => Err(format!(
                "unknown probe stage {other:?} (expected pack, verify, unpack, repack, or export)"
            )),
        }
    }
}

/// `pack`, `verify`, `unpack` -- the layouts every archive already exercises
/// elsewhere in this crate. `repack` and `export` are opt-in (see module
/// docs).
#[must_use]
pub fn default_stages() -> Vec<ProbeStage> {
    vec![ProbeStage::Pack, ProbeStage::Verify, ProbeStage::Unpack]
}

/// Knobs for one [`run_probe`] call.
#[derive(Debug, Clone)]
pub struct ProbeConfig {
    pub total_bytes: u64,
    pub seed: u64,
    pub sample_interval: Duration,
    /// `pack` always runs (every other stage needs its output) whether or
    /// not it appears here.
    pub stages: Vec<ProbeStage>,
    pub profile: CompressionProfile,
    /// Base directory this probe creates its own scratch subdirectory
    /// under, and removes (best-effort) when the run ends.
    pub scratch_root: PathBuf,
    /// When set, the `verify` and `unpack` stages each run in a fresh child
    /// process of this executable (`ebr-access --mode probe-stage ...`), so
    /// their memory readings cannot inherit anything the `pack` stage left
    /// resident or retained in the allocator. The `ebr-access` binary sets
    /// this for `--probe-isolate true`; library tests leave it `None`.
    pub isolate_exe: Option<PathBuf>,
}

impl Default for ProbeConfig {
    fn default() -> Self {
        Self {
            total_bytes: 8 * 1024 * 1024,
            seed: 0,
            sample_interval: Duration::from_millis(50),
            stages: default_stages(),
            profile: CompressionProfile::Fast,
            scratch_root: std::env::temp_dir(),
            isolate_exe: None,
        }
    }
}

/// One stage's wall time, phase-scoped memory, and resident-memory series.
#[derive(Debug, Clone, Serialize)]
pub struct StageMeasurement {
    pub stage: &'static str,
    /// `true` when this stage ran in its own child process.
    pub isolated: bool,
    pub wall_seconds: f64,
    /// High-water mark scoped to this stage (see
    /// `ebr_common::measure::ScopedMemory`). For an isolated stage this is
    /// also the child process's whole-lifetime peak.
    pub memory: ScopedMemory,
    /// Largest `samples[i].rss_bytes` (current-RSS series; can miss a spike
    /// shorter than the sampling interval, which `memory.peak_rss_bytes`
    /// does not).
    pub peak_sampled_rss_bytes: Option<u64>,
    pub samples: Vec<MemorySample>,
    /// Isolated stages only: the child process's peak since the stage began
    /// (its kernel high-water mark; see `run_probe_stage_child` in
    /// `main.rs` for why this is not `getrusage`'s `max_rss`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process_lifetime_peak_rss_bytes: Option<u64>,
}

fn stage_measurement(
    stage: ProbeStage,
    wall_seconds: f64,
    memory: ScopedMemory,
    samples: Vec<MemorySample>,
) -> StageMeasurement {
    StageMeasurement {
        stage: stage.as_str(),
        isolated: false,
        wall_seconds,
        memory,
        peak_sampled_rss_bytes: samples.iter().filter_map(|s| s.rss_bytes).max(),
        samples,
        process_lifetime_peak_rss_bytes: None,
    }
}

/// Every measurement one [`run_probe`] call produces. `stages` holds one
/// serialized [`StageMeasurement`] per stage, in execution order: `pack`,
/// then the opt-in `repack`/`export` (which need the planned archive), then
/// `verify`/`unpack` (which run only after the planned archive is dropped).
/// Stages are JSON values because isolated stages come back from a child
/// process as JSON.
#[derive(Debug, Clone, Serialize)]
pub struct ProbeMeasurement {
    pub requested_total_bytes: u64,
    /// Exact size of the synthetic scratch input file.
    pub scratch_input_bytes: u64,
    pub scratch_encoded_bytes: Option<u64>,
    pub scratch_unpacked_logical_bytes: Option<u64>,
    pub stages: Vec<serde_json::Value>,
}

#[derive(Debug)]
pub enum ProbeError {
    Entrybound(Diagnostic),
    Io(io::Error),
    /// An isolated child stage failed or printed something unparseable.
    Child(String),
}

impl std::fmt::Display for ProbeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProbeError::Entrybound(d) => write!(f, "entrybound: {d}"),
            ProbeError::Io(e) => write!(f, "io: {e}"),
            ProbeError::Child(message) => write!(f, "isolated probe stage: {message}"),
        }
    }
}

impl std::error::Error for ProbeError {}

impl From<Diagnostic> for ProbeError {
    fn from(d: Diagnostic) -> Self {
        ProbeError::Entrybound(d)
    }
}

impl From<io::Error> for ProbeError {
    fn from(e: io::Error) -> Self {
        ProbeError::Io(e)
    }
}

impl From<serde_json::Error> for ProbeError {
    fn from(e: serde_json::Error) -> Self {
        ProbeError::Child(e.to_string())
    }
}

/// Removes its directory (best-effort) when dropped, including on an early
/// return from [`run_probe`] via `?`.
struct ScratchGuard(PathBuf);

impl Drop for ScratchGuard {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).ok();
    }
}

// ---------------------------------------------------------------------------
// Orchestration
// ---------------------------------------------------------------------------

/// Runs the streaming memory probe under `config`. See module docs for what
/// each stage measures and why `repack`/`export` are opt-in.
pub fn run_probe(config: ProbeConfig) -> Result<ProbeMeasurement, ProbeError> {
    if config.stages.is_empty() {
        return Ok(ProbeMeasurement {
            requested_total_bytes: config.total_bytes,
            scratch_input_bytes: 0,
            scratch_encoded_bytes: None,
            scratch_unpacked_logical_bytes: None,
            stages: Vec::new(),
        });
    }

    let scratch_dir = config.scratch_root.join(format!(
        "ebr-access-probe-{}-{}",
        std::process::id(),
        ebr_common::timing::generate_run_id()
    ));
    fs::create_dir_all(&scratch_dir)?;
    let _guard = ScratchGuard(scratch_dir.clone());

    let input_dir = scratch_dir.join("input");
    fs::create_dir_all(&input_dir)?;
    let input_file = input_dir.join("synthetic.bin");
    write_synthetic_file(&input_file, config.total_bytes, config.seed)?;
    let scratch_input_bytes = fs::metadata(&input_file)?.len();

    let (archive, encoded_path, pack_stage, encoded_bytes) =
        run_pack_stage(&input_dir, &scratch_dir, &config)?;
    let mut stages = vec![serde_json::to_value(&pack_stage)?];

    if config.stages.contains(&ProbeStage::Repack) {
        stages.push(serde_json::to_value(run_repack_stage(&archive, &config)?)?);
    }
    if config.stages.contains(&ProbeStage::Export) {
        stages.push(serde_json::to_value(run_export_stage(&archive, &config)?)?);
    }
    // Harness review round 1, finding R1-05: the planned Archive holds every
    // Chunk's plaintext. Earlier revisions kept it alive through verify and
    // unpack, so those stages' memory readings always included it.
    drop(archive);

    let mut scratch_unpacked_logical_bytes = None;
    for stage in [ProbeStage::Verify, ProbeStage::Unpack] {
        if !config.stages.contains(&stage) {
            continue;
        }
        let value = match &config.isolate_exe {
            Some(exe) => run_isolated_stage(exe, stage, &encoded_path, &scratch_dir, &config)?,
            None => serde_json::to_value(run_single_stage(
                stage,
                &encoded_path,
                &scratch_dir,
                config.sample_interval,
            )?)?,
        };
        if stage == ProbeStage::Unpack {
            scratch_unpacked_logical_bytes = value
                .get("unpacked_logical_bytes")
                .and_then(serde_json::Value::as_u64);
        }
        stages.push(value);
    }

    Ok(ProbeMeasurement {
        requested_total_bytes: config.total_bytes,
        scratch_input_bytes,
        scratch_encoded_bytes: Some(encoded_bytes),
        scratch_unpacked_logical_bytes,
        stages,
    })
}

/// Runs one `verify` or `unpack` stage over an existing STREAM file. This is
/// what an isolated child process (`ebr-access --mode probe-stage`) runs.
/// The returned measurement carries `unpacked_logical_bytes` for `unpack`
/// via [`SingleStageMeasurement`].
pub fn run_single_stage(
    stage: ProbeStage,
    encoded_path: &Path,
    scratch_dir: &Path,
    sample_interval: Duration,
) -> Result<SingleStageMeasurement, ProbeError> {
    match stage {
        ProbeStage::Verify => Ok(SingleStageMeasurement {
            measurement: run_verify_stage(encoded_path, sample_interval)?,
            unpacked_logical_bytes: None,
        }),
        ProbeStage::Unpack => {
            let (measurement, bytes) =
                run_unpack_stage(encoded_path, scratch_dir, sample_interval)?;
            Ok(SingleStageMeasurement {
                measurement,
                unpacked_logical_bytes: Some(bytes),
            })
        }
        other => Err(ProbeError::Child(format!(
            "stage {} needs the planned archive and cannot run on its own",
            other.as_str()
        ))),
    }
}

/// A [`StageMeasurement`] plus the unpack stage's logical byte count.
#[derive(Debug, Clone, Serialize)]
pub struct SingleStageMeasurement {
    #[serde(flatten)]
    pub measurement: StageMeasurement,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unpacked_logical_bytes: Option<u64>,
}

fn run_isolated_stage(
    exe: &Path,
    stage: ProbeStage,
    encoded_path: &Path,
    scratch_dir: &Path,
    config: &ProbeConfig,
) -> Result<serde_json::Value, ProbeError> {
    let interval_ms = config.sample_interval.as_millis().max(1).to_string();
    let output = std::process::Command::new(exe)
        .arg("--mode")
        .arg("probe-stage")
        .arg("--probe-stage")
        .arg(stage.as_str())
        .arg("--probe-encoded-path")
        .arg(encoded_path)
        .arg("--probe-stage-scratch")
        .arg(scratch_dir)
        .arg("--probe-sample-interval-ms")
        .arg(interval_ms)
        .output()?;
    if !output.status.success() {
        return Err(ProbeError::Child(format!(
            "{} stage exited with {}: {}",
            stage.as_str(),
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let line = text
        .lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .ok_or_else(|| ProbeError::Child(format!("{} stage printed nothing", stage.as_str())))?;
    Ok(serde_json::from_str(line)?)
}

fn run_pack_stage(
    input_dir: &Path,
    scratch_dir: &Path,
    config: &ProbeConfig,
) -> Result<(Archive, PathBuf, StageMeasurement, u64), ProbeError> {
    let encoded_path = scratch_dir.join("encoded.stream");
    let sampler = MemorySampler::start(config.sample_interval);
    let scope = MemoryScope::begin();
    let sw = Stopwatch::start();
    let archive = plan_directory(
        input_dir,
        PackOptions {
            profile: config.profile,
            ..PackOptions::default()
        },
    )?;
    let file = File::create(&encoded_path)?;
    let mut writer = BufWriter::new(file);
    let summary = encode_stream(&archive, StreamWriteOptions::default(), &mut writer)?;
    writer.flush()?;
    let wall_seconds = sw.elapsed_secs_f64();
    let memory = scope.end();
    let samples = sampler.stop();
    Ok((
        archive,
        encoded_path,
        stage_measurement(ProbeStage::Pack, wall_seconds, memory, samples),
        summary.total_len,
    ))
}

fn run_verify_stage(
    encoded_path: &Path,
    sample_interval: Duration,
) -> Result<StageMeasurement, ProbeError> {
    let sampler = MemorySampler::start(sample_interval);
    let scope = MemoryScope::begin();
    let sw = Stopwatch::start();
    let reader = BufReader::new(File::open(encoded_path)?);
    let _report = verify_stream_with_limits(reader, bootstrap_sequential_limits())?;
    let wall_seconds = sw.elapsed_secs_f64();
    let memory = scope.end();
    let samples = sampler.stop();
    Ok(stage_measurement(
        ProbeStage::Verify,
        wall_seconds,
        memory,
        samples,
    ))
}

fn run_unpack_stage(
    encoded_path: &Path,
    scratch_dir: &Path,
    sample_interval: Duration,
) -> Result<(StageMeasurement, u64), ProbeError> {
    let destination = scratch_dir.join(format!(
        "unpacked-{}",
        ebr_common::timing::generate_run_id()
    ));
    let sampler = MemorySampler::start(sample_interval);
    let scope = MemoryScope::begin();
    let sw = Stopwatch::start();
    let reader = BufReader::new(File::open(encoded_path)?);
    let result = unpack_stream(
        reader,
        &destination,
        ExtractionPolicy::default(),
        bootstrap_sequential_limits(),
    );
    let wall_seconds = sw.elapsed_secs_f64();
    let memory = scope.end();
    let samples = sampler.stop();
    fs::remove_dir_all(&destination).ok();
    let (extraction, _stream_report) = result?;
    Ok((
        stage_measurement(ProbeStage::Unpack, wall_seconds, memory, samples),
        extraction.logical_bytes_written,
    ))
}

fn run_repack_stage(
    archive: &Archive,
    config: &ProbeConfig,
) -> Result<StageMeasurement, ProbeError> {
    let sampler = MemorySampler::start(config.sample_interval);
    let scope = MemoryScope::begin();
    let sw = Stopwatch::start();
    let encoded_indexed = encode(archive, WriteOptions::default())?;
    let opened = open(&encoded_indexed.bytes)?;
    let _prepared = prepare_repack(
        &opened,
        RepackOptions {
            mode: RepackMode::RepresentationOnly,
            layout: Layout::Indexed,
            index: IndexPolicy::Present,
            stream_window: StreamWindow::default(),
        },
    )?;
    let wall_seconds = sw.elapsed_secs_f64();
    let memory = scope.end();
    let samples = sampler.stop();
    Ok(stage_measurement(
        ProbeStage::Repack,
        wall_seconds,
        memory,
        samples,
    ))
}

fn run_export_stage(
    archive: &Archive,
    config: &ProbeConfig,
) -> Result<StageMeasurement, ProbeError> {
    let sampler = MemorySampler::start(config.sample_interval);
    let scope = MemoryScope::begin();
    let sw = Stopwatch::start();
    let _prepared = prepare_export(
        archive,
        ExportTarget::TarPaxV1,
        ExportSourceSecurity::default(),
    )?;
    let wall_seconds = sw.elapsed_secs_f64();
    let memory = scope.end();
    let samples = sampler.stop();
    Ok(stage_measurement(
        ProbeStage::Export,
        wall_seconds,
        memory,
        samples,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch_root(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "ebr-access-probe-test-root-{label}-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).ok();
        dir
    }

    #[test]
    fn synthetic_generator_produces_exactly_the_requested_length() {
        let mut generator = SyntheticGenerator::new(100_003, 7);
        let mut out = Vec::new();
        generator.read_to_end(&mut out).unwrap();
        assert_eq!(out.len(), 100_003);
    }

    #[test]
    fn synthetic_generator_is_reproducible_for_the_same_seed() {
        let mut a = SyntheticGenerator::new(5_000, 42);
        let mut b = SyntheticGenerator::new(5_000, 42);
        let mut out_a = Vec::new();
        let mut out_b = Vec::new();
        a.read_to_end(&mut out_a).unwrap();
        b.read_to_end(&mut out_b).unwrap();
        assert_eq!(out_a, out_b);
    }

    #[test]
    fn synthetic_generator_diverges_for_different_seeds() {
        let mut a = SyntheticGenerator::new(5_000, 1);
        let mut b = SyntheticGenerator::new(5_000, 2);
        let mut out_a = Vec::new();
        let mut out_b = Vec::new();
        a.read_to_end(&mut out_a).unwrap();
        b.read_to_end(&mut out_b).unwrap();
        assert_ne!(out_a, out_b);
    }

    #[test]
    fn write_synthetic_file_writes_the_exact_requested_length() {
        let root = scratch_root("write-file");
        let path = root.join("synthetic.bin");
        write_synthetic_file(&path, 65_537, 1).unwrap();
        assert_eq!(std::fs::metadata(&path).unwrap().len(), 65_537);
        std::fs::remove_dir_all(&root).ok();
    }

    /// A small (kilobyte-scale) smoke run of the default stages
    /// (`pack`, `verify`, `unpack`). Non-decision-grade: it only proves the
    /// probe mechanism runs end to end and reports plausible numbers, not
    /// anything about how memory scales at the 1-64 GiB scale this probe is
    /// built for (see module docs).
    #[test]
    fn default_stages_run_end_to_end_and_report_plausible_measurements() {
        let root = scratch_root("default-stages");
        let measurement = run_probe(ProbeConfig {
            total_bytes: 256 * 1024,
            seed: 1,
            sample_interval: Duration::from_millis(5),
            scratch_root: root.clone(),
            isolate_exe: None,
            ..ProbeConfig::default()
        })
        .unwrap();

        assert_eq!(measurement.requested_total_bytes, 256 * 1024);
        assert_eq!(measurement.scratch_input_bytes, 256 * 1024);
        assert!(measurement.scratch_encoded_bytes.unwrap() > 0);
        assert!(measurement.scratch_unpacked_logical_bytes.unwrap() > 0);

        let stage_names: Vec<&str> = measurement
            .stages
            .iter()
            .map(|s| s["stage"].as_str().unwrap())
            .collect();
        assert_eq!(stage_names, vec!["pack", "verify", "unpack"]);
        for stage in &measurement.stages {
            assert!(stage["wall_seconds"].as_f64().unwrap() >= 0.0);
            assert!(
                !stage["samples"].as_array().unwrap().is_empty(),
                "{} stage should have taken at least one memory sample",
                stage["stage"]
            );
            #[cfg(target_os = "linux")]
            assert_eq!(stage["memory"]["scoped"], true);
        }

        // The probe must not leave its scratch directory behind.
        let entries: Vec<_> = std::fs::read_dir(&root).unwrap().collect();
        assert!(
            entries.is_empty(),
            "run_probe should remove its own scratch subdirectory"
        );

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn repack_and_export_stages_are_opt_in_and_also_run() {
        let root = scratch_root("opt-in-stages");
        let measurement = run_probe(ProbeConfig {
            total_bytes: 64 * 1024,
            seed: 2,
            sample_interval: Duration::from_millis(5),
            stages: vec![ProbeStage::Repack, ProbeStage::Export],
            scratch_root: root.clone(),
            isolate_exe: None,
            ..ProbeConfig::default()
        })
        .unwrap();

        let stage_names: Vec<&str> = measurement
            .stages
            .iter()
            .map(|s| s["stage"].as_str().unwrap())
            .collect();
        // "pack" always runs first even though it was not listed explicitly.
        assert_eq!(stage_names, vec!["pack", "repack", "export"]);

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn empty_stages_is_a_no_op() {
        let measurement = run_probe(ProbeConfig {
            stages: Vec::new(),
            ..ProbeConfig::default()
        })
        .unwrap();
        assert!(measurement.stages.is_empty());
        assert_eq!(measurement.scratch_input_bytes, 0);
    }

    /// The verify stage must not inherit the pack stage's peak: pack holds
    /// every Chunk's plaintext, verify streams.
    #[cfg(target_os = "linux")]
    #[test]
    fn verify_stage_memory_does_not_inherit_the_pack_peak() {
        let root = scratch_root("scoped-stages");
        let measurement = run_probe(ProbeConfig {
            total_bytes: 64 * 1024 * 1024,
            seed: 3,
            sample_interval: Duration::from_millis(5),
            stages: default_stages(),
            scratch_root: root.clone(),
            isolate_exe: None,
            ..ProbeConfig::default()
        })
        .unwrap();
        let peak = |name: &str| {
            measurement
                .stages
                .iter()
                .find(|s| s["stage"] == name)
                .and_then(|s| s["memory"]["peak_delta_bytes"].as_u64())
                .unwrap()
        };
        assert!(
            peak("pack") >= 32 * 1024 * 1024,
            "pack should hold most of the 64 MiB plaintext, delta was {}",
            peak("pack")
        );
        assert!(
            peak("verify") < peak("pack"),
            "verify delta {} should be below pack delta {}",
            peak("verify"),
            peak("pack")
        );
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn single_stage_refuses_stages_that_need_the_planned_archive() {
        let result = run_single_stage(
            ProbeStage::Repack,
            Path::new("unused"),
            Path::new("unused"),
            Duration::from_millis(5),
        );
        assert!(matches!(result, Err(ProbeError::Child(_))));
    }
}
