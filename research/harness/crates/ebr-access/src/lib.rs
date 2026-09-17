//! Random/remote access and streaming/memory measurements, over
//! entrybound's public production API.
//!
//! - [`measure_random_access`]: INDEXED metadata-first random access
//!   (`entrybound::ecf::open_indexed_random`) over every regular-file entry,
//!   with the exact byte/range accounting
//!   `RandomAccessVerificationReport` already tracks (`bytes_fetched`,
//!   `range_request_count`) -- the same accounting a real remote backend
//!   (`entrybound::random_access::HttpRangeSource`) would pay for, even
//!   though this crate drives it over an in-memory source
//!   (`MemoryRandomReadSource`) so far.
//! - [`measure_stream`]: STREAM sequential encode -> open -> verify
//!   (`entrybound::ecf::{encode_stream, open_stream, verify_stream}`), the
//!   layout meant for a source that cannot seek.
//!
//! Both attach [`ebr_common::measure::peak_memory`] after the operation, so
//! a caller can compare INDEXED random access's metadata-first footprint
//! against STREAM's necessarily-more-sequential one.

use ebr_common::measure::peak_memory;
use ebr_common::timing::Stopwatch;
use entrybound::diagnostics::Diagnostic;
use entrybound::eam::{Archive, LogicalPath};
use entrybound::ecf::{
    StreamWriteOptions, encode_stream, open_indexed_random, open_stream, verify_stream,
};
use entrybound::random_access::{MemoryRandomReadSource, RandomAccessPolicy};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct RandomReadMeasurement {
    pub path: String,
    pub read_seconds: f64,
    pub bytes_returned: u64,
    /// Bytes a range-backed source had to serve for this one read
    /// (`RandomAccessVerificationReport::bytes_fetched`): metadata-first
    /// access reads only what a request needs, not the whole archive.
    pub bytes_fetched_from_source: u64,
    pub range_request_count: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RandomAccessMeasurement {
    pub open_seconds: f64,
    pub reads: Vec<RandomReadMeasurement>,
    pub total_bytes_fetched: u64,
    pub peak_memory_bytes: Option<u64>,
    pub peak_memory_is_true_peak: bool,
}

/// Opens `encoded` (Complete INDEXED bytes) for random access and reads
/// back every path in `paths`, in order, measuring each read.
pub fn measure_random_access(
    encoded: &[u8],
    paths: &[LogicalPath],
) -> Result<RandomAccessMeasurement, Diagnostic> {
    let source = MemoryRandomReadSource::new(encoded.to_vec());
    let open_sw = Stopwatch::start();
    let mut archive = open_indexed_random(source, RandomAccessPolicy::default())?;
    let open_seconds = open_sw.elapsed_secs_f64();

    let mut reads = Vec::with_capacity(paths.len());
    let mut total_bytes_fetched = 0u64;
    for path in paths {
        let read_sw = Stopwatch::start();
        let read = archive.read_entry(path)?;
        let read_seconds = read_sw.elapsed_secs_f64();
        total_bytes_fetched += read.report.bytes_fetched;
        reads.push(RandomReadMeasurement {
            path: path.to_string(),
            read_seconds,
            bytes_returned: read.bytes.len() as u64,
            bytes_fetched_from_source: read.report.bytes_fetched,
            range_request_count: read.report.range_request_count,
        });
    }

    let (peak_memory_bytes, peak_memory_is_true_peak) = match peak_memory() {
        Ok(sample) => (Some(sample.bytes), sample.is_true_peak),
        Err(_) => (None, false),
    };

    Ok(RandomAccessMeasurement {
        open_seconds,
        reads,
        total_bytes_fetched,
        peak_memory_bytes,
        peak_memory_is_true_peak,
    })
}

#[derive(Debug, Clone, Serialize)]
pub struct StreamMeasurement {
    pub encode_seconds: f64,
    pub encoded_bytes: u64,
    pub open_verify_seconds: f64,
    pub verified: bool,
    pub peak_memory_bytes: Option<u64>,
    pub peak_memory_is_true_peak: bool,
}

/// Encodes `archive` (already planned, e.g. by
/// `entrybound::archive::plan_directory`) as STREAM, then opens and
/// verifies that STREAM byte sequence in one sequential pass.
/// `entrybound::ecf::encode_stream` accepts any planned Archive and sets the
/// STREAM layout itself; a caller does not need to re-plan for it.
pub fn measure_stream(archive: &Archive) -> Result<StreamMeasurement, Diagnostic> {
    let mut encoded = Vec::new();
    let encode_sw = Stopwatch::start();
    let summary = encode_stream(archive, StreamWriteOptions::default(), &mut encoded)?;
    let encode_seconds = encode_sw.elapsed_secs_f64();
    let encoded_bytes = summary.total_len;

    let verify_sw = Stopwatch::start();
    let opened = open_stream(encoded.as_slice())?;
    let open_verify_seconds = verify_sw.elapsed_secs_f64();
    let verified = opened.opened.report.canonical_encoding
        && opened.opened.report.container_structure
        && opened.opened.report.chunk_integrity
        && opened.opened.report.content_integrity;
    debug_assert!(
        verify_stream(encoded.as_slice())
            .map(|r| r.canonical_encoding)
            .unwrap_or(false)
            == opened.opened.report.canonical_encoding,
        "verify_stream and open_stream should agree on canonical_encoding"
    );

    let (peak_memory_bytes, peak_memory_is_true_peak) = match peak_memory() {
        Ok(sample) => (Some(sample.bytes), sample.is_true_peak),
        Err(_) => (None, false),
    };

    Ok(StreamMeasurement {
        encode_seconds,
        encoded_bytes: encoded_bytes as u64,
        open_verify_seconds,
        verified,
        peak_memory_bytes,
        peak_memory_is_true_peak,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use entrybound::archive::{PackOptions, plan_directory};
    use entrybound::eam::EntryKind;
    use entrybound::ecf::{WriteOptions, encode};
    use entrybound::planner::CompressionProfile;

    fn scratch_input(label: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "ebr-access-input-test-{label}-{}",
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
    fn random_access_reads_every_regular_file_entry() {
        let input = scratch_input("random");
        let planned = plan_directory(
            &input,
            PackOptions {
                profile: CompressionProfile::Fast,
                ..PackOptions::default()
            },
        )
        .unwrap();
        let encoded = encode(&planned, WriteOptions::default()).unwrap();

        let paths: Vec<LogicalPath> = planned
            .entry_set
            .entries()
            .iter()
            .filter(|e| e.kind() == EntryKind::File)
            .map(|e| e.path().clone())
            .collect();
        assert_eq!(paths.len(), 2);

        let measurement = measure_random_access(&encoded.bytes, &paths).unwrap();
        assert_eq!(measurement.reads.len(), 2);
        assert!(measurement.total_bytes_fetched > 0);
        for read in &measurement.reads {
            assert!(read.bytes_returned > 0);
        }

        std::fs::remove_dir_all(&input).ok();
    }

    #[test]
    fn stream_round_trip_verifies() {
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
            measurement.verified,
            "STREAM encode/open/verify should agree for a freshly planned archive"
        );
        assert!(measurement.encoded_bytes > 0);

        std::fs::remove_dir_all(&input).ok();
    }
}
