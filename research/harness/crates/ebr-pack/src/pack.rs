//! The phase-timed pack pipeline over entrybound's public production API,
//! for INDEXED, STREAM, and encrypted-INDEXED archives.
//!
//! # What is measured, and in what order (harness review round 1, R1-03)
//!
//! The measured region is exactly the production work a caller of
//! `ebound pack` pays for, and nothing the harness adds:
//!
//! 1. **planning** -- `entrybound::archive::plan_directory`, which captures
//!    the filesystem, chunks every ContentObject, and selects codecs,
//!    transforms, dictionaries, and groups in one opaque call (there is no
//!    public hook to split its capture, chunking, and planning apart).
//! 2. **encoding** -- `entrybound::ecf::encode` or `encode_stream`. For
//!    `--encrypt`, `entrybound::crypto::pack_directory_encrypted` bundles
//!    planning and encoding into one call, so `planning_seconds` is `0.0`
//!    and the whole call is reported as `encoding_seconds`.
//!
//! A [`MemoryScope`] is opened immediately before step 1 and closed
//! immediately after step 2, so [`PackOutcome::pack_memory`] is the phase's
//! own high-water mark on Linux, not the process-lifetime peak.
//!
//! Only after that scope closes does the harness do its own work, none of
//! which is timed as a production phase: open and verify the encoded bytes,
//! inspect them for counts, walk the input tree's *metadata* for
//! `input_logical_bytes`/`input_file_count` (`input_scan_seconds`; no file
//! contents are read), and -- only with `chunking_pass: true` -- run a
//! separate per-file chunker pass (`chunking_pass_seconds`).
//!
//! Earlier revisions read the entire input tree into memory (up to 8 GiB)
//! *before* planning and reported that read as `capture_seconds`. That was
//! harness overhead mislabeled as a production phase, it warmed the page
//! cache for `plan_directory`, and the in-memory copy stayed resident while
//! the production pack ran, inflating every peak-memory reading. It is gone.
//!
//! The per-file chunking pass is not what production does
//! (`plan_directory` selects chunking parameters over its own view of the
//! content); it is an opt-in, harness-only approximation kept for visibility
//! and must not be summed with `planning_seconds`.

use crate::counts::{self, ArchiveCounts};
use crate::encrypt::{CredentialError, TestCredentials};
use ebr_common::measure::{MemoryScope, ScopedMemory};
use ebr_common::timing::Stopwatch;
use entrybound::archive::{PackOptions, inspect, plan_directory};
use entrybound::crypto::{
    EncryptedOpenOptions, EncryptedWriteOptions, Unlock, open_encrypted_authenticated,
    pack_directory_encrypted,
};
use entrybound::diagnostics::Diagnostic;
use entrybound::eam::Layout;
use entrybound::ecf::{
    StreamWindow, StreamWriteOptions, WriteOptions, encode, encode_stream, open, open_stream,
};
use entrybound::planner::CompressionProfile;
use serde::Serialize;
use std::fmt;
use std::path::Path;

#[derive(Debug)]
pub enum PackError {
    Entrybound(Diagnostic),
    Io(std::io::Error),
    Credential(CredentialError),
    /// A request this crate deliberately does not support (e.g. encrypted
    /// STREAM), distinct from an entrybound-reported failure.
    Unsupported(String),
}

impl fmt::Display for PackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PackError::Entrybound(d) => write!(f, "entrybound: {d}"),
            PackError::Io(e) => write!(f, "io: {e}"),
            PackError::Credential(e) => write!(f, "{e}"),
            PackError::Unsupported(msg) => write!(f, "unsupported: {msg}"),
        }
    }
}

impl std::error::Error for PackError {}

impl From<Diagnostic> for PackError {
    fn from(d: Diagnostic) -> Self {
        PackError::Entrybound(d)
    }
}

impl From<std::io::Error> for PackError {
    fn from(e: std::io::Error) -> Self {
        PackError::Io(e)
    }
}

impl From<CredentialError> for PackError {
    fn from(e: CredentialError) -> Self {
        PackError::Credential(e)
    }
}

/// What to pack and how.
#[derive(Debug, Clone)]
pub struct PackRequest {
    pub profile: CompressionProfile,
    pub layout: Layout,
    /// STREAM only: `None` keeps `StreamWindow::default()` (`Ceiling(0)`,
    /// forbidding cross-object historical Chunk dependencies), exactly like
    /// `ebound pack --layout stream` without `--stream-window`.
    pub stream_window: Option<StreamWindow>,
    pub encrypt: bool,
    /// Run the opt-in, harness-only per-file chunker pass after the measured
    /// production pack (see the module docs).
    pub chunking_pass: bool,
}

/// Wall-clock seconds for each phase. Only `planning_seconds` and
/// `encoding_seconds` are production phases; see the module docs.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct PhaseSeconds {
    /// `plan_directory`: production capture + chunking + planning.
    pub planning_seconds: f64,
    /// `encode`/`encode_stream`, or the whole `pack_directory_encrypted`.
    pub encoding_seconds: f64,
    /// Harness-only metadata walk after the measured pack (no contents read).
    pub input_scan_seconds: f64,
    /// Harness-only per-file chunker pass, present only when requested.
    pub chunking_pass_seconds: Option<f64>,
}

/// One completed pack run's output and measurements. Byte accounting is
/// deliberately not computed here -- see `crate::bytes`, which any caller
/// (fresh from `pack_once`, or given an arbitrary archive file) can invoke
/// the same way.
pub struct PackOutcome {
    pub bytes: Vec<u8>,
    pub phases: PhaseSeconds,
    /// Phase-scoped memory of the production plan + encode only.
    pub pack_memory: ScopedMemory,
    pub counts: ArchiveCounts,
    pub verified: bool,
    pub credentials: Option<TestCredentials>,
    pub input_logical_bytes: u64,
    pub input_file_count: u64,
}

/// Runs one full production pack over `input_dir`, then the harness's own
/// verification, inspection, and input scan (see the module docs).
pub fn pack_once(input_dir: &Path, request: &PackRequest) -> Result<PackOutcome, PackError> {
    if request.encrypt && request.layout == Layout::Stream {
        return Err(PackError::Unsupported(
            "encrypted STREAM archives: entrybound's only public encrypted-pack entry point \
             (pack_directory_encrypted) always produces an INDEXED-shaped encrypted container; \
             pass --layout indexed with --encrypt true, or --layout stream without --encrypt"
                .to_owned(),
        ));
    }

    // Credential generation is harness setup, outside the measured scope.
    let credentials = if request.encrypt {
        Some(TestCredentials::generate()?)
    } else {
        None
    };
    let pack_options = PackOptions {
        profile: request.profile,
        ..PackOptions::default()
    };

    let scope = MemoryScope::begin();
    let (bytes, planning_seconds, encoding_seconds) = match (&credentials, request.layout) {
        (Some(creds), _) => {
            let write_options = EncryptedWriteOptions {
                password: Some(&creds.password),
                ..EncryptedWriteOptions::default()
            };
            let encode_sw = Stopwatch::start();
            let encrypted = pack_directory_encrypted(input_dir, pack_options, write_options)?;
            (encrypted.bytes, 0.0_f64, encode_sw.elapsed_secs_f64())
        }
        (None, Layout::Indexed) => {
            let plan_sw = Stopwatch::start();
            let archive = plan_directory(input_dir, pack_options)?;
            let planning_seconds = plan_sw.elapsed_secs_f64();
            let encode_sw = Stopwatch::start();
            let encoded = encode(&archive, WriteOptions::default())?;
            (
                encoded.bytes,
                planning_seconds,
                encode_sw.elapsed_secs_f64(),
            )
        }
        (None, Layout::Stream) => {
            let plan_sw = Stopwatch::start();
            let archive = plan_directory(input_dir, pack_options)?;
            let planning_seconds = plan_sw.elapsed_secs_f64();
            let window = request.stream_window.unwrap_or_default();
            let encode_sw = Stopwatch::start();
            let mut buf = Vec::new();
            let _summary = encode_stream(
                &archive,
                StreamWriteOptions {
                    window,
                    budget_declared: true,
                },
                &mut buf,
            )?;
            (buf, planning_seconds, encode_sw.elapsed_secs_f64())
        }
    };
    let pack_memory = scope.end();

    // Harness-only work from here on.
    let opened = match (&credentials, request.layout) {
        (Some(creds), _) => {
            let open_options = EncryptedOpenOptions::new(Some(Unlock::Password(&creds.password)));
            open_encrypted_authenticated(&bytes, open_options)?.opened
        }
        (None, Layout::Indexed) => open(&bytes)?,
        (None, Layout::Stream) => open_stream(std::io::Cursor::new(bytes.as_slice()))?.opened,
    };
    let verified = verification_all_true(&opened.report);
    let inspection = inspect(&opened)?;
    let counts = counts::from_inspection(&inspection);
    drop(opened);

    let scan_sw = Stopwatch::start();
    let entries = ebr_common::walk::walk_tree(input_dir)?;
    let input_logical_bytes = ebr_common::walk::total_bytes(&entries);
    let input_file_count = entries
        .iter()
        .filter(|entry| !entry.is_dir && !entry.is_symlink)
        .count() as u64;
    let input_scan_seconds = scan_sw.elapsed_secs_f64();

    let chunking_pass_seconds = if request.chunking_pass {
        let chunking_sw = Stopwatch::start();
        for entry in entries
            .iter()
            .filter(|e| !e.is_dir && !e.is_symlink && e.len > 0)
        {
            let content = std::fs::read(input_dir.join(&entry.relative_path))?;
            let refs: [&[u8]; 1] = [content.as_slice()];
            let evaluation = entrybound::chunker::select_parameters(
                &refs,
                request.profile.chunking_candidates(),
            )?;
            let _ranges = entrybound::chunker::chunk_ranges(&content, evaluation.parameters)?;
        }
        Some(chunking_sw.elapsed_secs_f64())
    } else {
        None
    };

    Ok(PackOutcome {
        bytes,
        phases: PhaseSeconds {
            planning_seconds,
            encoding_seconds,
            input_scan_seconds,
            chunking_pass_seconds,
        },
        pack_memory,
        counts,
        verified,
        credentials,
        input_logical_bytes,
        input_file_count,
    })
}

fn verification_all_true(report: &entrybound::ecf::VerificationReport) -> bool {
    report.canonical_encoding
        && report.container_structure
        && report.section_integrity
        && report.semantic_invariants
        && report.chunk_integrity
        && report.dictionary_integrity
        && report.reconstruction_integrity
        && report.chunk_group_integrity
        && report.access_costs
        && report.content_integrity
        && report.entry_identities
        && report.lai
        && report.pcr
        && report.aux
        && report.pci_computed
}

/// One completed `--deterministic-check` run's outcome: `runs` independent
/// `pack_once` calls over the same input and request, hash-compared.
#[derive(Debug, Clone, Serialize)]
pub struct DeterminismCheck {
    pub requested_runs: u32,
    pub completed_runs: u32,
    pub all_identical: bool,
    /// The first run's SHA-256, for reference regardless of `all_identical`.
    pub sha256: String,
    /// Every distinct SHA-256 observed, in first-seen order. Length 1 when
    /// `all_identical` is `true`.
    pub distinct_sha256: Vec<String>,
}

/// Repeats [`pack_once`] `runs` times (`runs >= 1`) over the same input and
/// request, hashing each run's encoded bytes and comparing them. Chunking,
/// planning, and encoding are meant to be deterministic given the same
/// input, profile, and layout, so every run's bytes -- byte for byte,
/// including every identity/digest they embed -- should be identical.
/// Returns `Ok` either way (a mismatch is a real experimental finding, not a
/// tool bug); callers that want that to fail loudly check
/// `!result.all_identical` themselves (this crate's binaries do).
pub fn check_determinism(
    input_dir: &Path,
    request: &PackRequest,
    runs: u32,
) -> Result<DeterminismCheck, PackError> {
    let runs = runs.max(1);
    let mut distinct_sha256: Vec<String> = Vec::new();
    let mut first_sha256 = String::new();
    for i in 0..runs {
        let outcome = pack_once(input_dir, request)?;
        let sha256 = ebr_common::hash::sha256_hex(&outcome.bytes);
        if i == 0 {
            first_sha256 = sha256.clone();
        }
        if !distinct_sha256.contains(&sha256) {
            distinct_sha256.push(sha256);
        }
    }
    Ok(DeterminismCheck {
        requested_runs: runs,
        completed_runs: runs,
        all_identical: distinct_sha256.len() == 1,
        sha256: first_sha256,
        distinct_sha256,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn scratch_input(label: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("ebr-pack-pack-test-{label}-{}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        std::fs::write(
            dir.join("a.txt"),
            b"entrybound research harness\n".repeat(64),
        )
        .unwrap();
        std::fs::write(dir.join("sub").join("b.txt"), vec![7u8; 4096]).unwrap();
        dir
    }

    #[test]
    fn packs_indexed_and_reports_every_phase() {
        let input = scratch_input("indexed");
        let request = PackRequest {
            profile: CompressionProfile::Fast,
            layout: Layout::Indexed,
            stream_window: None,
            encrypt: false,
            chunking_pass: false,
        };
        let outcome = pack_once(&input, &request).expect("indexed pack should succeed");

        assert!(outcome.verified);
        assert!(!outcome.bytes.is_empty());
        assert_eq!(outcome.input_file_count, 2);
        assert!(outcome.input_logical_bytes > 0);
        assert!(outcome.phases.input_scan_seconds >= 0.0);
        assert!(outcome.phases.chunking_pass_seconds.is_none());
        assert!(outcome.phases.planning_seconds >= 0.0);
        assert!(outcome.phases.encoding_seconds >= 0.0);
        assert!(!outcome.counts.planner_id.is_empty());
        assert!(outcome.credentials.is_none());

        std::fs::remove_dir_all(&input).ok();
    }

    #[test]
    fn packs_stream_and_reports_every_phase() {
        let input = scratch_input("stream");
        let request = PackRequest {
            profile: CompressionProfile::Fast,
            layout: Layout::Stream,
            stream_window: None,
            encrypt: false,
            chunking_pass: false,
        };
        let outcome = pack_once(&input, &request).expect("stream pack should succeed");
        assert!(outcome.verified);
        assert!(!outcome.bytes.is_empty());

        std::fs::remove_dir_all(&input).ok();
    }

    #[test]
    fn packs_encrypted_indexed_and_verifies_with_the_generated_password() {
        let input = scratch_input("encrypted");
        let request = PackRequest {
            profile: CompressionProfile::Fast,
            layout: Layout::Indexed,
            stream_window: None,
            encrypt: true,
            chunking_pass: false,
        };
        let outcome = pack_once(&input, &request).expect("encrypted pack should succeed");
        assert!(outcome.verified);
        assert!(outcome.credentials.is_some());
        assert_eq!(outcome.phases.planning_seconds, 0.0);

        std::fs::remove_dir_all(&input).ok();
    }

    #[test]
    fn refuses_encrypted_stream_as_unsupported() {
        let input = scratch_input("encrypted-stream-refused");
        let request = PackRequest {
            profile: CompressionProfile::Fast,
            layout: Layout::Stream,
            stream_window: None,
            encrypt: true,
            chunking_pass: false,
        };
        let result = pack_once(&input, &request);
        assert!(matches!(result, Err(PackError::Unsupported(_))));
        std::fs::remove_dir_all(&input).ok();
    }

    #[test]
    fn determinism_check_finds_identical_bytes_across_repeats() {
        let input = scratch_input("determinism");
        let request = PackRequest {
            profile: CompressionProfile::Balanced,
            layout: Layout::Indexed,
            stream_window: None,
            encrypt: false,
            chunking_pass: false,
        };
        let result = check_determinism(&input, &request, 3).unwrap();
        assert_eq!(result.requested_runs, 3);
        assert_eq!(result.completed_runs, 3);
        assert!(
            result.all_identical,
            "expected identical bytes across repeats, got {} distinct hashes",
            result.distinct_sha256.len()
        );
        assert_eq!(result.distinct_sha256.len(), 1);

        std::fs::remove_dir_all(&input).ok();
    }
}
