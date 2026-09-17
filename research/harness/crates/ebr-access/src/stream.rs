//! STREAM sequential encode -> open -> verify -> list -> unpack measurement,
//! over entrybound's public production API. STREAM is the layout meant for
//! a source that cannot seek (`entrybound::ecf::stream` module docs); this
//! module drives it over an in-memory `Vec<u8>` today (see crate README
//! "Non-goals").
//!
//! [`measure_stream`] runs, over one already-planned `Archive`
//! (`entrybound::archive::plan_directory` or equivalent):
//!
//! 1. **encode** (`entrybound::ecf::encode_stream`) into an in-memory sink.
//! 2. **open** (`entrybound::ecf::open_stream`), which is also entrybound's
//!    only verification pass for STREAM: `StreamAccessProfile`
//!    (`entrybound::ecf::StreamAccessProfile`) states
//!    `listing_requires_full_scan: true`, so there is no cheaper way to list
//!    a STREAM archive's entries than to run this same full sequential scan.
//!    `StreamReport`'s own memory/scratch fields (`peak_retained_chunks`,
//!    `peak_resident_staging_bytes`, `spilled_staging_bytes`) come straight
//!    from this pass.
//! 3. **verify** (`entrybound::ecf::verify_stream`), a second, independent
//!    sequential pass -- entrybound does not itself reuse the `open_stream`
//!    result to answer `verify_stream`; this crate calls both so a caller
//!    can compare their cost.
//! 4. **list** (`entrybound::archive::list`), over the already-verified
//!    in-memory model the *open* pass produced. It is cheap on top of an
//!    already-open archive (each entry's already-known ContentObject size is
//!    summed, no new Chunk is read), which is the honest way to present
//!    "listing" for a layout whose *first* listing is never cheap.
//! 5. **unpack** (`entrybound::archive::unpack_stream`), a third sequential
//!    pass with `StreamContentPolicy::Stage`
//!    (`entrybound::ecf::StreamContentPolicy`), extracting into a scratch
//!    directory this module creates and removes itself.

use std::path::PathBuf;

use ebr_common::measure::{ScopedMemory, measure_scoped};
use ebr_common::timing::Stopwatch;
use entrybound::archive::{ExtractionPolicy, ExtractionReport, ListedEntry, list, unpack_stream};
use entrybound::diagnostics::Diagnostic;
use entrybound::eam::Archive;
use entrybound::ecf::{
    StreamReport, StreamWriteOptions, VerificationReport, bootstrap_sequential_limits,
    encode_stream, open_stream, verify_stream,
};
use serde::Serialize;

/// A `StreamReport`'s memory/scratch-relevant fields, carried through as-is.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct StreamPassStats {
    pub dedup_window: u64,
    pub chunk_frames: u64,
    pub manifest_records: u64,
    pub peak_retained_chunks: u64,
    pub peak_resident_staging_bytes: u64,
    pub spilled_staging_bytes: u64,
    pub plaintext_retained: bool,
}

impl From<&StreamReport> for StreamPassStats {
    fn from(report: &StreamReport) -> Self {
        Self {
            dedup_window: report.dedup_window,
            chunk_frames: report.chunk_frames,
            manifest_records: report.manifest_records,
            peak_retained_chunks: report.peak_retained_chunks,
            peak_resident_staging_bytes: report.peak_resident_staging_bytes,
            spilled_staging_bytes: report.spilled_staging_bytes,
            plaintext_retained: report.plaintext_retained,
        }
    }
}

fn verification_report_all_true(report: &VerificationReport) -> bool {
    report.canonical_encoding
        && report.container_structure
        && report.section_integrity
        && report.semantic_invariants
        && report.chunk_integrity
        && report.dictionary_integrity
        && report.reconstruction_integrity
        && report.chunk_group_integrity
        && report.content_integrity
        && report.entry_identities
        && report.lai
        && report.pcr
        && report.aux
}

/// Every measurement one [`measure_stream`] call produces.
#[derive(Debug, Clone, Serialize)]
pub struct StreamMeasurement {
    pub encode_seconds: f64,
    pub encoded_bytes: u64,

    pub open_seconds: f64,
    pub verified_by_open: bool,
    pub open_pass: StreamPassStats,

    pub verify_seconds: f64,
    pub verified_by_verify_stream: bool,

    pub list_seconds: f64,
    pub listed_entries: u64,
    pub listed_total_logical_bytes: u64,

    pub unpack_seconds: f64,
    pub unpacked_entries_created: u64,
    pub unpacked_logical_bytes: u64,
    pub unpack_pass: StreamPassStats,

    /// Per-pass memory scoped to that pass alone (harness review round 1,
    /// R1-05; the encoded archive stays in memory throughout, as input).
    pub encode_memory: ScopedMemory,
    pub open_memory: ScopedMemory,
    pub verify_memory: ScopedMemory,
    pub unpack_memory: ScopedMemory,
}

#[derive(Debug)]
pub enum StreamMeasureError {
    Entrybound(Diagnostic),
    Io(std::io::Error),
}

impl std::fmt::Display for StreamMeasureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StreamMeasureError::Entrybound(d) => write!(f, "entrybound: {d}"),
            StreamMeasureError::Io(e) => write!(f, "io: {e}"),
        }
    }
}

impl std::error::Error for StreamMeasureError {}

impl From<Diagnostic> for StreamMeasureError {
    fn from(d: Diagnostic) -> Self {
        StreamMeasureError::Entrybound(d)
    }
}

impl From<std::io::Error> for StreamMeasureError {
    fn from(e: std::io::Error) -> Self {
        StreamMeasureError::Io(e)
    }
}

/// Encodes `archive` (already planned, e.g. by
/// `entrybound::archive::plan_directory`) as STREAM, then runs the
/// encode/open/verify/list/unpack sequence this module's docs describe.
/// `entrybound::ecf::encode_stream` accepts any planned `Archive` and sets
/// the STREAM layout itself; a caller does not need to re-plan for it.
pub fn measure_stream(archive: &Archive) -> Result<StreamMeasurement, StreamMeasureError> {
    let mut encoded = Vec::new();
    let (summary, encode_memory, encode_seconds) =
        measure_scoped(|| encode_stream(archive, StreamWriteOptions::default(), &mut encoded));
    let summary = summary?;
    let encoded_bytes = summary.total_len;

    let (opened, open_memory, open_seconds) = measure_scoped(|| open_stream(encoded.as_slice()));
    let opened = opened?;
    let verified_by_open = verification_report_all_true(&opened.opened.report);
    let open_pass = StreamPassStats::from(&opened.stream);

    let (verify_report, verify_memory, verify_seconds) =
        measure_scoped(|| verify_stream(encoded.as_slice()));
    let verify_report = verify_report?;
    let verified_by_verify_stream = verification_report_all_true(&verify_report);

    let list_sw = Stopwatch::start();
    let listed: Vec<ListedEntry> = list(&opened.opened.archive)?;
    let list_seconds = list_sw.elapsed_secs_f64();
    let listed_entries = listed.len() as u64;
    let listed_total_logical_bytes = listed.iter().map(|entry| entry.logical_bytes).sum();

    let scratch: PathBuf = std::env::temp_dir().join(format!(
        "ebr-access-stream-unpack-{}-{}",
        std::process::id(),
        ebr_common::timing::generate_run_id()
    ));
    let (unpacked, unpack_memory, unpack_seconds) = measure_scoped(|| {
        unpack_stream(
            encoded.as_slice(),
            &scratch,
            ExtractionPolicy::default(),
            bootstrap_sequential_limits(),
        )
    });
    let (extraction, unpack_stream_report): (ExtractionReport, StreamReport) = unpacked?;
    std::fs::remove_dir_all(&scratch).ok();
    let unpack_pass = StreamPassStats::from(&unpack_stream_report);

    Ok(StreamMeasurement {
        encode_seconds,
        encoded_bytes,
        open_seconds,
        verified_by_open,
        open_pass,
        verify_seconds,
        verified_by_verify_stream,
        list_seconds,
        listed_entries,
        listed_total_logical_bytes,
        unpack_seconds,
        unpacked_entries_created: extraction.entries_created,
        unpacked_logical_bytes: extraction.logical_bytes_written,
        unpack_pass,
        encode_memory,
        open_memory,
        verify_memory,
        unpack_memory,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use entrybound::archive::{PackOptions, plan_directory};
    use entrybound::planner::CompressionProfile;

    fn scratch_input(label: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "ebr-access-stream-input-test-{label}-{}",
            std::process::id()
        ));
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        std::fs::write(
            dir.join("a.txt"),
            b"hello entrybound research harness access crate\n".repeat(200),
        )
        .unwrap();
        std::fs::write(dir.join("sub").join("b.bin"), vec![9u8; 8192]).unwrap();
        dir
    }

    #[test]
    fn stream_round_trip_verifies_lists_and_unpacks() {
        let input = scratch_input("stream");
        let planned = plan_directory(
            &input,
            PackOptions {
                profile: CompressionProfile::Fast,
                ..PackOptions::default()
            },
        )
        .unwrap();

        let measurement = measure_stream(&planned).unwrap();
        assert!(
            measurement.verified_by_open,
            "open_stream should report every check true for a freshly encoded archive"
        );
        assert!(
            measurement.verified_by_verify_stream,
            "verify_stream should agree with open_stream"
        );
        assert!(measurement.encoded_bytes > 0);
        // 3 Entries: "a.txt", the "sub" directory, and "sub/b.bin" -- `list`
        // and extraction both walk every Entry, not just regular files.
        assert_eq!(measurement.listed_entries, 3);
        assert!(measurement.listed_total_logical_bytes > 0);
        assert_eq!(measurement.unpacked_entries_created, 3);
        assert!(measurement.unpacked_logical_bytes > 0);

        std::fs::remove_dir_all(&input).ok();
    }
}
