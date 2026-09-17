//! INDEXED metadata-first random access measurement, over entrybound's
//! public production API (`entrybound::ecf::open_indexed_random`).
//!
//! [`measure_random_access`] is generic over any
//! `entrybound::random_access::RandomReadSource`, so one function covers
//! `MemoryRandomReadSource`, `LocalFileRandomReadSource`, and
//! `HttpRangeSource` alike (a caller picks the concrete source, or boxes one
//! as `Box<dyn RandomReadSource>` -- boxed trait objects implement the trait
//! too, see `entrybound::random_access`'s blanket impl). Over one opened
//! archive, in this order:
//!
//! 1. **first-entry read**: a dedicated `read_entry` call for `entries[0]`,
//!    taken before anything else touches the session, so its latency is the
//!    coldest this session can offer. INDEXED random access has no partial
//!    plaintext streaming API (`read_entry` always returns a whole
//!    `ContentObject`'s plaintext at once -- see `RandomAccessRead::bytes`),
//!    so "time to first plaintext byte" and "time to the complete first
//!    entry" are the same measurement against today's production API; this
//!    is stated, not glossed over.
//! 2. **a seeded random sample** of entries, each read once, in an order
//!    determined by [`crate::rng::sample_indices`] -- reproducible across
//!    runs given the same `seed`.
//! 3. **eager full consumption**: every requested entry, read once, in the
//!    order given.
//!
//! Reads in steps 2 and 3 can revisit an entry steps 1 or 2 already read;
//! `entrybound::random_access::RangeSession` caches fetched ranges for the
//! lifetime of one `RandomAccessArchive`, so a revisit is a warm-cache hit
//! (zero `bytes_fetched_from_source`). That is disclosed here, not
//! hidden: a caller that needs a strictly cold eager pass sets
//! `sample_size: 0`, and every `RandomReadMeasurement` carries its own
//! `bytes_fetched_from_source`/`range_request_count` so warm and cold reads
//! are always distinguishable after the fact.
//!
//! **Per-read byte accounting (harness review round 1, finding R1-02).**
//! `RandomAccessVerificationReport::{bytes_fetched, range_request_count}`
//! are the *session's cumulative* counters (`RangeSession::bytes_fetched`),
//! not per-call figures. Earlier revisions reported them as each read's own
//! cost and summed them into `eager_total_bytes_fetched`, which counted the
//! metadata open and every earlier read again on every later read. Each
//! read's `bytes_fetched_from_source`/`range_request_count` is now the
//! difference from the session counters before that read;
//! `metadata_open_bytes_fetched` is the open's own cost; and
//! `session_total_bytes_fetched` equals the metadata open plus the sum of
//! every read's delta (a unit test enforces this).
//!
//! **Random byte-range reads within large entries**: every time a read in
//! any of the three steps above returns a plaintext at or above
//! `large_entry_threshold_bytes`, `byte_ranges_per_large_entry` seeded random
//! `(offset, len)` slices are additionally cut from that already-fetched
//! plaintext and timed. Because `RandomAccessArchive::read_entry` has no
//! sub-object range API, cutting a slice costs nothing extra at the source:
//! the whole ContentObject's dependency Chunk closure was already fetched to
//! answer the surrounding `read_entry` call, so `slice_seconds` measures
//! pure in-memory sub-slicing, not a source fetch. The read that produced
//! the plaintext (its own `bytes_fetched_from_source`,
//! `range_request_count`, `dependency_chunk_count`) is what a caller should
//! read to learn what a "just this byte range" request truly costs a
//! range-backed source today. See the crate README's "Non-goals".
//!
//! Every read's [`RandomAccessVerificationReport`] flags
//! (`entrybound::ecf::RandomAccessVerificationReport`) are carried through
//! verbatim in [`VerificationFlags`], including `dependency_chunk_count`.

use ebr_common::measure::{MemoryScope, ScopedMemory};
use ebr_common::timing::Stopwatch;
use entrybound::diagnostics::Diagnostic;
use entrybound::eam::LogicalPath;
use entrybound::ecf::{RandomAccessArchive, RandomAccessVerificationReport, open_indexed_random};
use entrybound::random_access::{RandomAccessPolicy, RandomReadSource};
use serde::Serialize;

use crate::rng::{SplitMix64, sample_indices};

/// `RandomAccessVerificationReport`'s boolean/count fields, carried through
/// as-is (enum fields become their `Debug` string so this type can derive
/// `Serialize` without entrybound needing to depend on serde).
#[derive(Debug, Clone, Serialize)]
pub struct VerificationFlags {
    pub source_revision_stable: bool,
    pub preamble_footer_verified: bool,
    pub section_structure_verified: bool,
    pub semantic_metadata_sections_verified: bool,
    pub index_status: String,
    pub requested_path: Option<String>,
    pub content_object_digest_verified: bool,
    pub chunk_count_verified: u64,
    pub dependency_chunk_count: u64,
    pub dictionaries_verified: bool,
    pub groups_verified: bool,
    pub reconstruction_verified: bool,
    pub bytes_fetched: u64,
    pub range_request_count: u64,
    pub lai: String,
    pub aux: String,
    pub pcr: String,
    pub pci: String,
    pub whole_archive_verified: bool,
    /// Session-cumulative trace length (like `bytes_fetched` and
    /// `range_request_count` above, which are also session-cumulative).
    pub access_trace_entries: u64,
}

impl From<&RandomAccessVerificationReport> for VerificationFlags {
    fn from(report: &RandomAccessVerificationReport) -> Self {
        Self {
            source_revision_stable: report.source_revision_stable,
            preamble_footer_verified: report.preamble_footer_verified,
            section_structure_verified: report.section_structure_verified,
            semantic_metadata_sections_verified: report.semantic_metadata_sections_verified,
            index_status: format!("{:?}", report.index_status),
            requested_path: report.requested_path.clone(),
            content_object_digest_verified: report.content_object_digest_verified,
            chunk_count_verified: report.chunk_count_verified,
            dependency_chunk_count: report.dependency_chunk_count,
            dictionaries_verified: report.dictionaries_verified,
            groups_verified: report.groups_verified,
            reconstruction_verified: report.reconstruction_verified,
            bytes_fetched: report.bytes_fetched,
            range_request_count: report.range_request_count,
            lai: format!("{:?}", report.lai),
            aux: format!("{:?}", report.aux),
            pcr: format!("{:?}", report.pcr),
            pci: format!("{:?}", report.pci),
            whole_archive_verified: report.whole_archive_verified,
            access_trace_entries: u64::try_from(report.access_trace.len()).unwrap_or(u64::MAX),
        }
    }
}

/// One `read_entry` call's timing, byte accounting, and verification report.
#[derive(Debug, Clone, Serialize)]
pub struct RandomReadMeasurement {
    pub path: String,
    pub read_seconds: f64,
    pub bytes_returned: u64,
    /// Bytes this read fetched from the source (session delta).
    pub bytes_fetched_from_source: u64,
    /// Range requests this read issued (session delta).
    pub range_request_count: u64,
    /// Session-cumulative bytes fetched after this read.
    pub session_bytes_fetched_after: u64,
    pub dependency_chunk_count: u64,
    pub verification: VerificationFlags,
}

/// One in-memory byte-range slice cut from an already-fetched large entry.
#[derive(Debug, Clone, Serialize)]
pub struct ByteRangeReadMeasurement {
    pub path: String,
    pub entry_len: u64,
    pub range_offset: u64,
    pub range_len: u64,
    pub slice_seconds: f64,
}

/// Knobs for one [`measure_random_access`] call.
#[derive(Debug, Clone)]
pub struct RandomAccessConfig {
    pub policy: RandomAccessPolicy,
    /// Size of the seeded random entry sample (capped at `entries.len()`).
    /// `0` skips the sample pass entirely (a strictly cold eager pass).
    pub sample_size: usize,
    pub seed: u64,
    /// An entry at or above this plaintext size is eligible for byte-range
    /// sampling.
    pub large_entry_threshold_bytes: u64,
    pub byte_ranges_per_large_entry: usize,
    /// Requested byte-range length; capped to the entry's own length.
    pub byte_range_len: u64,
}

impl Default for RandomAccessConfig {
    fn default() -> Self {
        Self {
            policy: RandomAccessPolicy::default(),
            sample_size: 8,
            seed: 0,
            large_entry_threshold_bytes: 64 * 1024,
            byte_ranges_per_large_entry: 4,
            byte_range_len: 4096,
        }
    }
}

/// Every measurement one [`measure_random_access`] call produces.
#[derive(Debug, Clone, Serialize)]
pub struct RandomAccessMeasurement {
    pub open_seconds: f64,
    /// Bytes and range requests the metadata-first open itself fetched.
    pub metadata_open_bytes_fetched: u64,
    pub metadata_open_range_requests: u64,
    /// Cost of metadata-first opening alone, before any entry's payload was
    /// touched (`RandomAccessArchive::metadata_report`).
    pub metadata_open: VerificationFlags,
    pub entry_count: usize,
    /// `None` only when `entries` is empty.
    pub first_entry_read: Option<RandomReadMeasurement>,
    pub sample_seed: u64,
    pub sampled_entry_reads: Vec<RandomReadMeasurement>,
    pub byte_range_reads: Vec<ByteRangeReadMeasurement>,
    pub eager_reads: Vec<RandomReadMeasurement>,
    pub eager_total_seconds: f64,
    /// Sum of the eager pass's per-read deltas.
    pub eager_total_bytes_fetched: u64,
    /// Session-cumulative bytes fetched at the end (open + every read).
    pub session_total_bytes_fetched: u64,
    pub session_total_range_requests: u64,
    /// Memory scoped to open + every read (not the process-lifetime peak,
    /// which includes planning and encoding the archive being read).
    pub access_memory: ScopedMemory,
}

/// Opens `source` (Complete INDEXED bytes behind any `RandomReadSource`) for
/// random access under `config.policy`, then runs the dedicated first-entry
/// read, the seeded sample, and the eager full pass described in this
/// module's docs, over every path in `entries` (in the order given --
/// `entries[0]` is what "first entry" means throughout).
pub fn measure_random_access<S>(
    source: S,
    entries: &[LogicalPath],
    config: RandomAccessConfig,
) -> Result<RandomAccessMeasurement, Diagnostic>
where
    S: RandomReadSource + 'static,
{
    let scope = MemoryScope::begin();
    let open_sw = Stopwatch::start();
    let mut archive = open_indexed_random(source, config.policy.clone())?;
    let open_seconds = open_sw.elapsed_secs_f64();

    let metadata_report = archive.metadata_report()?;
    let metadata_open_bytes_fetched = metadata_report.bytes_fetched;
    let metadata_open_range_requests = metadata_report.range_request_count;
    let mut session = (metadata_open_bytes_fetched, metadata_open_range_requests);
    let metadata_open = VerificationFlags::from(&metadata_report);
    let mut range_rng = SplitMix64::new(config.seed ^ 0xA5A5_A5A5_A5A5_A5A5);
    let mut byte_range_reads = Vec::new();

    let first_entry_read = match entries.first() {
        Some(path) => {
            let (measurement, bytes) = read_one(&mut archive, path, &mut session)?;
            collect_byte_ranges(path, &bytes, &config, &mut range_rng, &mut byte_range_reads);
            Some(measurement)
        }
        None => None,
    };

    let sample = sample_indices(entries.len(), config.sample_size, config.seed);
    let mut sampled_entry_reads = Vec::with_capacity(sample.len());
    for &index in &sample {
        let path = &entries[index];
        let (measurement, bytes) = read_one(&mut archive, path, &mut session)?;
        collect_byte_ranges(path, &bytes, &config, &mut range_rng, &mut byte_range_reads);
        sampled_entry_reads.push(measurement);
    }

    let mut eager_reads = Vec::with_capacity(entries.len());
    let eager_sw = Stopwatch::start();
    for path in entries {
        let (measurement, bytes) = read_one(&mut archive, path, &mut session)?;
        collect_byte_ranges(path, &bytes, &config, &mut range_rng, &mut byte_range_reads);
        eager_reads.push(measurement);
    }
    let eager_total_seconds = eager_sw.elapsed_secs_f64();
    let eager_total_bytes_fetched = eager_reads
        .iter()
        .map(|r| r.bytes_fetched_from_source)
        .sum();

    let access_memory = scope.end();

    Ok(RandomAccessMeasurement {
        open_seconds,
        metadata_open_bytes_fetched,
        metadata_open_range_requests,
        metadata_open,
        entry_count: entries.len(),
        first_entry_read,
        sample_seed: config.seed,
        sampled_entry_reads,
        byte_range_reads,
        eager_reads,
        eager_total_seconds,
        eager_total_bytes_fetched,
        session_total_bytes_fetched: session.0,
        session_total_range_requests: session.1,
        access_memory,
    })
}

fn read_one(
    archive: &mut RandomAccessArchive,
    path: &LogicalPath,
    session: &mut (u64, u64),
) -> Result<(RandomReadMeasurement, Box<[u8]>), Diagnostic> {
    let read_sw = Stopwatch::start();
    let read = archive.read_entry(path)?;
    let read_seconds = read_sw.elapsed_secs_f64();
    let bytes_delta = read.report.bytes_fetched.saturating_sub(session.0);
    let requests_delta = read.report.range_request_count.saturating_sub(session.1);
    *session = (read.report.bytes_fetched, read.report.range_request_count);
    let measurement = RandomReadMeasurement {
        path: path.to_string(),
        read_seconds,
        bytes_returned: read.bytes.len() as u64,
        bytes_fetched_from_source: bytes_delta,
        range_request_count: requests_delta,
        session_bytes_fetched_after: read.report.bytes_fetched,
        dependency_chunk_count: read.report.dependency_chunk_count,
        verification: VerificationFlags::from(&read.report),
    };
    Ok((measurement, read.bytes))
}

fn collect_byte_ranges(
    path: &LogicalPath,
    bytes: &[u8],
    config: &RandomAccessConfig,
    rng: &mut SplitMix64,
    out: &mut Vec<ByteRangeReadMeasurement>,
) {
    let entry_len = bytes.len() as u64;
    if entry_len == 0 || entry_len < config.large_entry_threshold_bytes {
        return;
    }
    for _ in 0..config.byte_ranges_per_large_entry {
        let range_len = config.byte_range_len.min(entry_len);
        let max_offset = entry_len - range_len;
        let range_offset = rng.next_below(max_offset + 1);
        let slice_sw = Stopwatch::start();
        let start = usize::try_from(range_offset).unwrap_or(0);
        let len = usize::try_from(range_len).unwrap_or(0);
        let slice = &bytes[start..start + len];
        std::hint::black_box(slice);
        let slice_seconds = slice_sw.elapsed_secs_f64();
        out.push(ByteRangeReadMeasurement {
            path: path.to_string(),
            entry_len,
            range_offset,
            range_len,
            slice_seconds,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use entrybound::archive::{PackOptions, plan_directory};
    use entrybound::eam::EntryKind;
    use entrybound::ecf::{WriteOptions, encode};
    use entrybound::planner::CompressionProfile;
    use entrybound::random_access::MemoryRandomReadSource;

    fn scratch_input(label: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "ebr-access-random-input-test-{label}-{}",
            std::process::id()
        ));
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        std::fs::write(
            dir.join("a.txt"),
            b"hello entrybound research harness access crate\n".repeat(200),
        )
        .unwrap();
        // Large enough to trigger byte-range sampling under the default
        // 64 KiB threshold.
        std::fs::write(dir.join("sub").join("b.bin"), vec![9u8; 200_000]).unwrap();
        dir
    }

    fn encode_scratch(label: &str) -> (std::path::PathBuf, Vec<u8>, Vec<LogicalPath>) {
        let input = scratch_input(label);
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
        (input, encoded.bytes, paths)
    }

    #[test]
    fn measures_first_entry_sample_and_eager_reads() {
        let (input, encoded, paths) = encode_scratch("basic");
        assert_eq!(paths.len(), 2);

        let source = MemoryRandomReadSource::new(encoded);
        let measurement = measure_random_access(
            source,
            &paths,
            RandomAccessConfig {
                sample_size: 1,
                ..RandomAccessConfig::default()
            },
        )
        .unwrap();

        assert_eq!(measurement.entry_count, 2);
        assert!(measurement.first_entry_read.is_some());
        assert_eq!(measurement.eager_reads.len(), 2);
        assert_eq!(measurement.sampled_entry_reads.len(), 1);
        // The first-entry read and the one-entry sample may already have
        // fetched both entries, in which case the eager pass is entirely
        // warm and its per-read deltas are zero; an earlier revision asserted
        // `eager_total_bytes_fetched > 0`, which held only because it summed
        // session-cumulative counters (review finding R1-02).
        assert!(measurement.session_total_bytes_fetched > 0);
        assert!(measurement.eager_total_bytes_fetched <= measurement.session_total_bytes_fetched);
        assert!(!measurement.byte_range_reads.is_empty());
        for range in &measurement.byte_range_reads {
            assert!(range.range_len <= range.entry_len);
            assert!(range.range_offset + range.range_len <= range.entry_len);
        }

        std::fs::remove_dir_all(&input).ok();
    }

    #[test]
    fn sample_is_reproducible_for_the_same_seed() {
        let (input, encoded, paths) = encode_scratch("reproducible");
        let config = RandomAccessConfig {
            sample_size: 2,
            seed: 12345,
            ..RandomAccessConfig::default()
        };

        let first = measure_random_access(
            MemoryRandomReadSource::new(encoded.clone()),
            &paths,
            config.clone(),
        )
        .unwrap();
        let second =
            measure_random_access(MemoryRandomReadSource::new(encoded), &paths, config).unwrap();

        let first_paths: Vec<_> = first
            .sampled_entry_reads
            .iter()
            .map(|r| r.path.clone())
            .collect();
        let second_paths: Vec<_> = second
            .sampled_entry_reads
            .iter()
            .map(|r| r.path.clone())
            .collect();
        assert_eq!(first_paths, second_paths);

        std::fs::remove_dir_all(&input).ok();
    }

    #[test]
    fn zero_sample_size_yields_a_strictly_cold_eager_pass() {
        let (input, encoded, paths) = encode_scratch("cold-eager");
        let measurement = measure_random_access(
            MemoryRandomReadSource::new(encoded),
            &paths,
            RandomAccessConfig {
                sample_size: 0,
                ..RandomAccessConfig::default()
            },
        )
        .unwrap();
        assert!(measurement.sampled_entry_reads.is_empty());
        assert_eq!(measurement.eager_reads.len(), 2);
        assert!(measurement.eager_total_bytes_fetched > 0);

        std::fs::remove_dir_all(&input).ok();
    }

    #[test]
    fn empty_entries_yields_no_first_entry_read() {
        let (input, encoded, _paths) = encode_scratch("empty-entries");
        let measurement = measure_random_access(
            MemoryRandomReadSource::new(encoded),
            &[],
            RandomAccessConfig::default(),
        )
        .unwrap();
        assert!(measurement.first_entry_read.is_none());
        assert!(measurement.eager_reads.is_empty());
        assert!(measurement.sampled_entry_reads.is_empty());

        std::fs::remove_dir_all(&input).ok();
    }

    /// Harness review round 1, finding R1-02: per-read byte counts must be
    /// deltas, so open + reads sum to the session total, and a revisit of an
    /// already-fetched entry costs nothing.
    #[test]
    fn per_read_bytes_are_deltas_that_sum_to_the_session_total() {
        let (input, encoded, paths) = encode_scratch("deltas");
        let source_len = encoded.len() as u64;
        let measurement = measure_random_access(
            MemoryRandomReadSource::new(encoded),
            &paths,
            RandomAccessConfig {
                sample_size: paths.len(),
                ..RandomAccessConfig::default()
            },
        )
        .unwrap();

        let mut sum = measurement.metadata_open_bytes_fetched;
        for read in measurement
            .first_entry_read
            .iter()
            .chain(&measurement.sampled_entry_reads)
            .chain(&measurement.eager_reads)
        {
            sum += read.bytes_fetched_from_source;
        }
        assert_eq!(sum, measurement.session_total_bytes_fetched);
        assert!(measurement.session_total_bytes_fetched <= source_len);
        // Every entry was already read by the sample pass, so the eager pass
        // is entirely warm-cache.
        assert_eq!(measurement.eager_total_bytes_fetched, 0);
        assert!(
            measurement
                .eager_reads
                .iter()
                .all(|r| r.range_request_count == 0)
        );
        std::fs::remove_dir_all(&input).ok();
    }
}
