//! The phase-timed pack pipeline: `capture -> chunking -> planning ->
//! encoding`, over entrybound's public production API, for INDEXED, STREAM,
//! and encrypted-INDEXED archives.
//!
//! # Why "chunking" and "planning" overlap
//!
//! entrybound's single public directory-to-archive entry point,
//! `entrybound::archive::plan_directory`, captures the filesystem, chunks
//! every ContentObject's bytes (`entrybound::chunker::{select_parameters,
//! chunk_ranges}`), and selects codecs/transforms/dictionaries/groups
//! (`entrybound::planner`), all as one opaque call -- there is no public
//! hook to time chunking separately from planning *inside* it. [`pack_once`]
//! reports both anyway, honestly: `chunking_seconds` is a **separate,
//! additional** pass that calls the same public chunker directly over each
//! captured file's bytes, purely so chunking has its own visible timing;
//! `planning_seconds` is the full `plan_directory` call, which repeats that
//! chunking work internally. The two are not mutually exclusive components
//! of one total -- summing every phase double-counts chunking by design,
//! and this module says so rather than quietly presenting the sum as if it
//! were a clean breakdown.
//!
//! `--encrypt` uses entrybound's only public encrypted-pack entry point,
//! `entrybound::crypto::pack_directory_encrypted`, which likewise bundles
//! planning and encoding into one opaque call; there [`PhaseSeconds::
//! planning_seconds`] is `0.0` and the whole call's time is reported as
//! [`PhaseSeconds::encoding_seconds`] instead, again documented rather than
//! guessed at.

use crate::counts::{self, ArchiveCounts};
use crate::encrypt::{CredentialError, TestCredentials};
use ebr_common::timing::Stopwatch;
use entrybound::archive::{PackOptions, inspect, plan_directory};
use entrybound::crypto::{EncryptedOpenOptions, EncryptedWriteOptions, Unlock, open_encrypted_authenticated, pack_directory_encrypted};
use entrybound::diagnostics::Diagnostic;
use entrybound::eam::Layout;
use entrybound::ecf::{StreamWindow, StreamWriteOptions, WriteOptions, encode, encode_stream, open, open_stream};
use entrybound::planner::CompressionProfile;
use serde::Serialize;
use std::fmt;
use std::path::Path;

/// Generous enough for every tuning/validation-scale corpus item this
/// harness packs; a validation-scale item that does not fit should be
/// measured through `entrybound::archive::pack_directory_stream` instead
/// (out of scope for this crate today -- see `README.md`).
const CAPTURE_BUDGET_BYTES: u64 = 8 * 1024 * 1024 * 1024;

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
    /// forbidding cross-object historical Chunk dependencies); `Some(n)`
    /// requests `StreamWindow::Ceiling(n)`.
    pub stream_window: Option<u64>,
    pub encrypt: bool,
}

/// Wall-clock seconds for each phase. See the module docs for why
/// `chunking_seconds` and `planning_seconds` are not mutually exclusive.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct PhaseSeconds {
    pub capture_seconds: f64,
    pub chunking_seconds: f64,
    pub planning_seconds: f64,
    pub encoding_seconds: f64,
}

/// One completed pack run's output and measurements. Byte accounting is
/// deliberately not computed here -- see `crate::bytes`, which any caller
/// (fresh from `pack_once`, or given an arbitrary archive file) can invoke
/// the same way.
pub struct PackOutcome {
    pub bytes: Vec<u8>,
    pub phases: PhaseSeconds,
    pub counts: ArchiveCounts,
    pub verified: bool,
    pub credentials: Option<TestCredentials>,
    pub input_logical_bytes: u64,
    pub input_file_count: u64,
}

/// Runs one full capture/chunk/plan/encode pass over `input_dir`.
pub fn pack_once(input_dir: &Path, request: &PackRequest) -> Result<PackOutcome, PackError> {
    if request.encrypt && request.layout == Layout::Stream {
        return Err(PackError::Unsupported(
            "encrypted STREAM archives: entrybound's only public encrypted-pack entry point \
             (pack_directory_encrypted) always produces an INDEXED-shaped encrypted container; \
             pass --layout indexed with --encrypt true, or --layout stream without --encrypt"
                .to_owned(),
        ));
    }

    let capture_sw = Stopwatch::start();
    let tree = ebr_common::walk::read_tree_in_memory(input_dir, CAPTURE_BUDGET_BYTES)?;
    let capture_seconds = capture_sw.elapsed_secs_f64();
    let input_logical_bytes = tree.total_bytes;
    let input_file_count = tree.files.len() as u64;

    let chunking_sw = Stopwatch::start();
    for (_relative_path, content) in &tree.files {
        if content.is_empty() {
            continue;
        }
        let refs: [&[u8]; 1] = [content.as_slice()];
        let evaluation =
            entrybound::chunker::select_parameters(&refs, request.profile.chunking_candidates())?;
        let _ranges = entrybound::chunker::chunk_ranges(content, evaluation.parameters)?;
    }
    let chunking_seconds = chunking_sw.elapsed_secs_f64();

    let (bytes, planning_seconds, encoding_seconds, opened, credentials) = if request.encrypt {
        let creds = TestCredentials::generate()?;
        let write_options = EncryptedWriteOptions {
            password: Some(&creds.password),
            ..EncryptedWriteOptions::default()
        };
        let encode_sw = Stopwatch::start();
        let encrypted = pack_directory_encrypted(
            input_dir,
            PackOptions {
                profile: request.profile,
                ..PackOptions::default()
            },
            write_options,
        )?;
        let encoding_seconds = encode_sw.elapsed_secs_f64();
        let bytes = encrypted.bytes;
        let open_options = EncryptedOpenOptions::new(Some(Unlock::Password(&creds.password)));
        let authenticated = open_encrypted_authenticated(&bytes, open_options)?;
        (bytes, 0.0_f64, encoding_seconds, authenticated.opened, Some(creds))
    } else {
        let plan_sw = Stopwatch::start();
        let archive = plan_directory(
            input_dir,
            PackOptions {
                profile: request.profile,
                ..PackOptions::default()
            },
        )?;
        let planning_seconds = plan_sw.elapsed_secs_f64();

        match request.layout {
            Layout::Indexed => {
                let encode_sw = Stopwatch::start();
                let encoded = encode(&archive, WriteOptions::default())?;
                let encoding_seconds = encode_sw.elapsed_secs_f64();
                let bytes = encoded.bytes;
                let opened = open(&bytes)?;
                (bytes, planning_seconds, encoding_seconds, opened, None)
            }
            Layout::Stream => {
                let window = request
                    .stream_window
                    .map(StreamWindow::Ceiling)
                    .unwrap_or_default();
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
                let encoding_seconds = encode_sw.elapsed_secs_f64();
                let sequential = open_stream(std::io::Cursor::new(buf.clone()))?;
                (buf, planning_seconds, encoding_seconds, sequential.opened, None)
            }
        }
    };

    let verified = verification_all_true(&opened.report);
    let inspection = inspect(&opened)?;
    let counts = counts::from_inspection(&inspection);

    Ok(PackOutcome {
        bytes,
        phases: PhaseSeconds {
            capture_seconds,
            chunking_seconds,
            planning_seconds,
            encoding_seconds,
        },
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
        let dir = std::env::temp_dir().join(format!(
            "ebr-pack-pack-test-{label}-{}",
            std::process::id()
        ));
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        std::fs::write(dir.join("a.txt"), b"entrybound research harness\n".repeat(64)).unwrap();
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
        };
        let outcome = pack_once(&input, &request).expect("indexed pack should succeed");

        assert!(outcome.verified);
        assert!(!outcome.bytes.is_empty());
        assert_eq!(outcome.input_file_count, 2);
        assert!(outcome.input_logical_bytes > 0);
        assert!(outcome.phases.capture_seconds >= 0.0);
        assert!(outcome.phases.chunking_seconds >= 0.0);
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
