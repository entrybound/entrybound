//! End-to-end archive measurements using entrybound's *public* production
//! API: `plan -> encode -> open -> verify -> unpack -> random read`, with
//! byte accounting at every stage. See `README.md` for the crate's purpose
//! and how its binary plugs into an `ebr` runner spec.
//!
//! This is Phase B2 skeleton code: [`measure_roundtrip`] is a real,
//! compiling, working round trip (it is exercised by this crate's own
//! tests), not a placeholder -- but it does not yet cover encrypted
//! archives, STREAM layout, or repack/diff, which are all also public API
//! (`entrybound::crypto`, `entrybound::ecf::{encode_stream, open_stream,
//! verify_stream}`, `entrybound::archive::{prepare_repack, archive_diff}`).
//! Extending it to those is future work for whichever experiment needs them.

use ebr_common::hash::sha256_hex;
use ebr_common::timing::Stopwatch;
use ebr_common::walk;
use entrybound::archive::{ExtractionPolicy, PackOptions, plan_directory, unpack};
use entrybound::diagnostics::Diagnostic;
use entrybound::eam::EntryKind;
use entrybound::ecf::{WriteOptions, encode, open, open_indexed_random, verify};
use entrybound::planner::CompressionProfile;
use entrybound::random_access::{MemoryRandomReadSource, RandomAccessPolicy};
use serde::Serialize;
use std::fmt;
use std::path::Path;

/// Every stage's timing and byte accounting for one pack/unpack round trip.
#[derive(Debug, Clone, Serialize)]
pub struct RoundtripMeasurement {
    pub profile: &'static str,
    pub input_logical_bytes: u64,
    pub input_file_count: u64,
    pub plan_seconds: f64,
    pub encode_seconds: f64,
    pub encoded_bytes: u64,
    pub encoded_sha256: String,
    pub open_seconds: f64,
    pub verify_seconds: f64,
    pub verified: bool,
    pub unpack_seconds: f64,
    pub unpacked_logical_bytes: u64,
    /// `None` when the input tree has no regular files to read back.
    pub random_read_seconds: Option<f64>,
    pub random_read_bytes: Option<u64>,
}

#[derive(Debug)]
pub enum RoundtripError {
    Entrybound(Diagnostic),
    Io(std::io::Error),
}

impl fmt::Display for RoundtripError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RoundtripError::Entrybound(d) => write!(f, "entrybound: {d}"),
            RoundtripError::Io(e) => write!(f, "io: {e}"),
        }
    }
}

impl std::error::Error for RoundtripError {}

impl From<Diagnostic> for RoundtripError {
    fn from(d: Diagnostic) -> Self {
        RoundtripError::Entrybound(d)
    }
}

impl From<std::io::Error> for RoundtripError {
    fn from(e: std::io::Error) -> Self {
        RoundtripError::Io(e)
    }
}

/// Runs one full plan -> encode -> open -> verify -> unpack -> random-read
/// round trip over `input_dir`, at `profile`, using only entrybound's public
/// API. Extracts into a fresh scratch directory it creates and removes
/// itself; never writes inside `input_dir`.
pub fn measure_roundtrip(
    input_dir: &Path,
    profile: CompressionProfile,
) -> Result<RoundtripMeasurement, RoundtripError> {
    let input_entries = walk::walk_tree(input_dir)?;
    let input_logical_bytes = walk::total_bytes(&input_entries);
    let input_file_count = input_entries
        .iter()
        .filter(|e| !e.is_dir && !e.is_symlink)
        .count() as u64;

    let plan_sw = Stopwatch::start();
    let archive = plan_directory(
        input_dir,
        PackOptions {
            profile,
            ..PackOptions::default()
        },
    )?;
    let plan_seconds = plan_sw.elapsed_secs_f64();

    let encode_sw = Stopwatch::start();
    let encoded = encode(&archive, WriteOptions::default())?;
    let encode_seconds = encode_sw.elapsed_secs_f64();
    let encoded_bytes = encoded.bytes.len() as u64;
    let encoded_sha256 = sha256_hex(&encoded.bytes);

    let open_sw = Stopwatch::start();
    let _opened = open(&encoded.bytes)?;
    let open_seconds = open_sw.elapsed_secs_f64();

    let verify_sw = Stopwatch::start();
    let report = verify(&encoded.bytes)?;
    let verify_seconds = verify_sw.elapsed_secs_f64();
    let verified = report.canonical_encoding
        && report.container_structure
        && report.section_integrity
        && report.semantic_invariants
        && report.chunk_integrity
        && report.content_integrity
        && report.entry_identities;

    let scratch = std::env::temp_dir().join(format!(
        "ebr-pack-unpack-{}-{}",
        std::process::id(),
        ebr_common::timing::generate_run_id()
    ));
    let unpack_sw = Stopwatch::start();
    let extraction = unpack(&encoded.bytes, &scratch, ExtractionPolicy::default())?;
    let unpack_seconds = unpack_sw.elapsed_secs_f64();
    let unpacked_logical_bytes = extraction.logical_bytes_written;
    std::fs::remove_dir_all(&scratch).ok();

    let first_file_path = archive
        .entry_set
        .entries()
        .iter()
        .find(|e| e.kind() == EntryKind::File)
        .map(|e| e.path().clone());
    let (random_read_seconds, random_read_bytes) = match first_file_path {
        Some(path) => {
            let source = MemoryRandomReadSource::new(encoded.bytes.clone());
            let random_sw = Stopwatch::start();
            let mut random_archive = open_indexed_random(source, RandomAccessPolicy::default())?;
            let read = random_archive.read_entry(&path)?;
            (
                Some(random_sw.elapsed_secs_f64()),
                Some(read.bytes.len() as u64),
            )
        }
        None => (None, None),
    };

    Ok(RoundtripMeasurement {
        profile: profile.as_str(),
        input_logical_bytes,
        input_file_count,
        plan_seconds,
        encode_seconds,
        encoded_bytes,
        encoded_sha256,
        open_seconds,
        verify_seconds,
        verified,
        unpack_seconds,
        unpacked_logical_bytes,
        random_read_seconds,
        random_read_bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch_input(label: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "ebr-pack-input-test-{label}-{}",
            std::process::id()
        ));
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        std::fs::write(
            dir.join("a.txt"),
            b"hello entrybound research harness\n".repeat(64),
        )
        .unwrap();
        std::fs::write(dir.join("sub").join("b.txt"), vec![7u8; 4096]).unwrap();
        dir
    }

    #[test]
    fn round_trip_measures_every_stage_and_verifies() {
        let input = scratch_input("roundtrip");
        let measurement =
            measure_roundtrip(&input, CompressionProfile::Fast).expect("round trip should succeed");

        assert!(measurement.input_logical_bytes > 0);
        assert_eq!(measurement.input_file_count, 2);
        assert!(measurement.encoded_bytes > 0);
        assert!(
            measurement.verified,
            "verify() should report every check true for a freshly encoded archive"
        );
        assert_eq!(
            measurement.unpacked_logical_bytes,
            measurement.input_logical_bytes
        );
        assert!(measurement.random_read_bytes.is_some());

        std::fs::remove_dir_all(&input).ok();
    }

    #[test]
    fn encoded_bytes_hash_is_deterministic_across_two_encodes() {
        let input = scratch_input("determinism");
        let first = measure_roundtrip(&input, CompressionProfile::Balanced).unwrap();
        let second = measure_roundtrip(&input, CompressionProfile::Balanced).unwrap();
        assert_eq!(first.encoded_sha256, second.encoded_sha256);

        std::fs::remove_dir_all(&input).ok();
    }
}
