//! Standalone extraction measurement over an already-encoded archive file,
//! independent of a fresh pack (see [`crate::pack`] for that).
//!
//! # What is measured (harness review round 1, finding R1-04)
//!
//! `unpack_seconds` and `unpack_memory` cover exactly the production entry
//! point a caller of `ebound unpack` pays for (reading the archive included),
//! with [`crate::scratch::ScratchSampler`] polling the destination directory
//! *and* production's staging spill files on a background thread. Only after
//! that does a separate, harness-only verification pass run to fill in
//! `verified`; its time is `verify_pass_seconds` and is not additive with
//! `unpack_seconds` (production extraction already verifies internally).
//!
//! Earlier revisions opened and verified first (reading the whole archive
//! into memory, and for STREAM cloning it), started the sampler before that
//! pass, and sampled only the destination -- so STREAM's staging spill in
//! the OS temp directory was never counted.

use crate::scratch::{PeakScratch, ScratchSampler};
use ebr_common::measure::{MemoryScope, ScopedMemory};
use ebr_common::timing::Stopwatch;
use entrybound::archive::{
    ExtractionPolicy, ExtractionReport, unpack, unpack_opened, unpack_stream,
};
use entrybound::crypto::{EncryptedOpenOptions, Unlock, open_encrypted_authenticated};
use entrybound::diagnostics::Diagnostic;
use entrybound::ecf::{StagingLimits, bootstrap_sequential_limits, open, open_stream};
use serde::Serialize;
use std::fmt;
use std::fs::File;
use std::path::Path;

#[derive(Debug)]
pub enum UnpackError {
    Entrybound(Diagnostic),
    Io(std::io::Error),
    /// `--encrypted true` without `--unlock-password-file`.
    MissingPassword,
}

impl fmt::Display for UnpackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UnpackError::Entrybound(d) => write!(f, "entrybound: {d}"),
            UnpackError::Io(e) => write!(f, "io: {e}"),
            UnpackError::MissingPassword => write!(
                f,
                "archive is encrypted but no --unlock-password-file was given"
            ),
        }
    }
}

impl std::error::Error for UnpackError {}

impl From<Diagnostic> for UnpackError {
    fn from(d: Diagnostic) -> Self {
        UnpackError::Entrybound(d)
    }
}

impl From<std::io::Error> for UnpackError {
    fn from(e: std::io::Error) -> Self {
        UnpackError::Io(e)
    }
}

/// What archive to open and how (mirrors the layout/encryption facts a
/// caller already knows -- typically because it just ran `ebr-pack` -- since
/// this crate does not sniff a container's layout or encryption from its
/// bytes; see `README.md`).
#[derive(Debug, Clone)]
pub struct UnpackRequest {
    pub layout: entrybound::eam::Layout,
    pub encrypted: bool,
    /// Required when `encrypted` is `true`: the password
    /// `TestCredentials::persist_password` wrote alongside the archive.
    pub password: Option<Vec<u8>>,
    /// STREAM only: overrides `bootstrap_sequential_limits().staging` (the
    /// production CLI default). Tests use it to force a staging spill.
    pub staging_limits: Option<StagingLimits>,
}

/// STREAM extraction's own staging counters (`StreamReport`), reported next
/// to the sampled spill peak as an independent check.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct StreamStagingReport {
    pub peak_resident_staging_bytes: u64,
    pub spilled_staging_bytes: u64,
    pub peak_retained_chunks: u64,
}

/// One `ebr-unpack` measurement. See the module docs for which figures are
/// production timings and which are harness-only.
#[derive(Debug, Clone, Serialize)]
pub struct UnpackMeasurement {
    pub archive_bytes: u64,
    /// Every `VerificationReport` guarantee held, from the separate,
    /// harness-only verification pass that runs after the measured unpack.
    pub verified: bool,
    /// The production entry point, including reading the archive file:
    /// INDEXED `fs::read` + `archive::unpack` (which opens and verifies
    /// internally); STREAM `archive::unpack_stream` over the file; encrypted
    /// `fs::read` + `open_encrypted_authenticated` + `unpack_opened`.
    pub unpack_seconds: f64,
    /// Harness-only verification pass after the measured unpack. Not a
    /// component of `unpack_seconds`; do not add the two.
    pub verify_pass_seconds: f64,
    pub entries_created: u64,
    pub extracted_logical_bytes: u64,
    /// Destination directory peak (apparent size).
    pub peak_scratch_bytes: u64,
    pub scratch: PeakScratch,
    /// Memory of the measured production unpack only.
    pub unpack_memory: ScopedMemory,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_staging: Option<StreamStagingReport>,
}

/// Measures production extraction of `archive_path` into `scratch_dir`
/// (created if missing), sampling on-disk scratch (destination plus
/// production's staging spill files) and scoping memory to the production
/// call, then runs a separate verification pass for the `verified` flags.
/// `scratch_dir` is left in place for the caller to inspect and remove.
pub fn unpack_measure(
    archive_path: &Path,
    scratch_dir: &Path,
    request: &UnpackRequest,
) -> Result<UnpackMeasurement, UnpackError> {
    std::fs::create_dir_all(scratch_dir)?;
    let archive_bytes = std::fs::metadata(archive_path)?.len();
    let password = if request.encrypted {
        Some(
            request
                .password
                .as_deref()
                .ok_or(UnpackError::MissingPassword)?,
        )
    } else {
        None
    };
    let mut limits = bootstrap_sequential_limits();
    if let Some(staging) = request.staging_limits {
        limits.staging = staging;
    }

    let policy = ExtractionPolicy::default();
    let sampler = ScratchSampler::start(
        scratch_dir.to_path_buf(),
        ScratchSampler::default_interval(),
    );
    let scope = MemoryScope::begin();
    let clock = Stopwatch::start();
    let outcome: Result<(ExtractionReport, Option<StreamStagingReport>), UnpackError> =
        (|| match (password, request.layout) {
            (Some(password), _) => {
                let bytes = std::fs::read(archive_path)?;
                let authenticated = open_encrypted_authenticated(
                    &bytes,
                    EncryptedOpenOptions::new(Some(Unlock::Password(password))),
                )?;
                Ok((
                    unpack_opened(&authenticated.opened, scratch_dir, policy)?,
                    None,
                ))
            }
            (None, entrybound::eam::Layout::Indexed) => {
                let bytes = std::fs::read(archive_path)?;
                Ok((unpack(&bytes, scratch_dir, policy)?, None))
            }
            (None, entrybound::eam::Layout::Stream) => {
                let (report, stream) =
                    unpack_stream(File::open(archive_path)?, scratch_dir, policy, limits)?;
                Ok((
                    report,
                    Some(StreamStagingReport {
                        peak_resident_staging_bytes: stream.peak_resident_staging_bytes,
                        spilled_staging_bytes: stream.spilled_staging_bytes,
                        peak_retained_chunks: stream.peak_retained_chunks,
                    }),
                ))
            }
        })();
    let unpack_seconds = clock.elapsed_secs_f64();
    let unpack_memory = scope.end();
    let scratch = sampler.stop();
    let (extraction, stream_staging) = outcome?;

    let verify_clock = Stopwatch::start();
    let report = match (password, request.layout) {
        (Some(password), _) => {
            let bytes = std::fs::read(archive_path)?;
            open_encrypted_authenticated(
                &bytes,
                EncryptedOpenOptions::new(Some(Unlock::Password(password))),
            )?
            .opened
            .report
        }
        (None, entrybound::eam::Layout::Indexed) => open(&std::fs::read(archive_path)?)?.report,
        (None, entrybound::eam::Layout::Stream) => {
            open_stream(std::io::BufReader::new(File::open(archive_path)?))?
                .opened
                .report
        }
    };
    let verify_pass_seconds = verify_clock.elapsed_secs_f64();

    Ok(UnpackMeasurement {
        archive_bytes,
        verified: verification_all_true(&report),
        unpack_seconds,
        verify_pass_seconds,
        entries_created: extraction.entries_created,
        extracted_logical_bytes: extraction.logical_bytes_written,
        peak_scratch_bytes: scratch.peak_bytes,
        scratch,
        unpack_memory,
        stream_staging,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encrypt::TestCredentials;
    use entrybound::archive::PackOptions;
    use entrybound::eam::Layout;
    use entrybound::planner::CompressionProfile;
    use std::path::PathBuf;

    fn scratch_input(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "ebr-pack-unpack-test-input-{label}-{}",
            std::process::id()
        ));
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        std::fs::write(dir.join("a.txt"), b"entrybound".repeat(500)).unwrap();
        std::fs::write(dir.join("sub").join("b.txt"), vec![3u8; 2048]).unwrap();
        dir
    }

    fn workspace(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "ebr-pack-unpack-test-work-{label}-{}",
            std::process::id()
        ));
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn unpacks_an_indexed_archive_and_measures_peak_scratch() {
        let input = scratch_input("indexed");
        let work = workspace("indexed");
        let archive = entrybound::archive::plan_directory(
            &input,
            PackOptions {
                profile: CompressionProfile::Fast,
                ..Default::default()
            },
        )
        .unwrap();
        let encoded =
            entrybound::ecf::encode(&archive, entrybound::ecf::WriteOptions::default()).unwrap();
        let archive_path = work.join("archive.ecf");
        std::fs::write(&archive_path, &encoded.bytes).unwrap();

        let request = UnpackRequest {
            layout: Layout::Indexed,
            encrypted: false,
            password: None,
            staging_limits: None,
        };
        let scratch_dir = work.join("extracted");
        let measurement = unpack_measure(&archive_path, &scratch_dir, &request).unwrap();

        assert!(measurement.verified);
        assert_eq!(measurement.entries_created, 3); // sub/ dir + 2 files
        assert!(measurement.extracted_logical_bytes > 0);
        assert!(measurement.peak_scratch_bytes >= measurement.extracted_logical_bytes);
        assert!(measurement.scratch.peak_total_bytes >= measurement.peak_scratch_bytes);
        assert!(scratch_dir.join("a.txt").is_file());

        std::fs::remove_dir_all(&input).ok();
        std::fs::remove_dir_all(&work).ok();
    }

    #[test]
    fn unpacks_a_stream_archive() {
        let input = scratch_input("stream");
        let work = workspace("stream");
        let archive = entrybound::archive::plan_directory(
            &input,
            PackOptions {
                profile: CompressionProfile::Fast,
                ..Default::default()
            },
        )
        .unwrap();
        let mut buf = Vec::new();
        entrybound::ecf::encode_stream(
            &archive,
            entrybound::ecf::StreamWriteOptions::default(),
            &mut buf,
        )
        .unwrap();
        let archive_path = work.join("archive.ecfs");
        std::fs::write(&archive_path, &buf).unwrap();

        let request = UnpackRequest {
            layout: Layout::Stream,
            encrypted: false,
            password: None,
            staging_limits: None,
        };
        let scratch_dir = work.join("extracted");
        let measurement = unpack_measure(&archive_path, &scratch_dir, &request).unwrap();
        assert!(measurement.verified);
        assert!(scratch_dir.join("a.txt").is_file());

        std::fs::remove_dir_all(&input).ok();
        std::fs::remove_dir_all(&work).ok();
    }

    #[test]
    fn unpacks_an_encrypted_archive_with_its_password() {
        let input = scratch_input("encrypted");
        let work = workspace("encrypted");
        let creds = TestCredentials::generate().unwrap();
        let write_options = entrybound::crypto::EncryptedWriteOptions {
            password: Some(&creds.password),
            ..Default::default()
        };
        let encrypted = entrybound::crypto::pack_directory_encrypted(
            &input,
            PackOptions {
                profile: CompressionProfile::Fast,
                ..Default::default()
            },
            write_options,
        )
        .unwrap();
        let archive_path = work.join("archive.ecf");
        std::fs::write(&archive_path, &encrypted.bytes).unwrap();

        let request = UnpackRequest {
            layout: Layout::Indexed,
            encrypted: true,
            password: Some(creds.password.clone()),
            staging_limits: None,
        };
        let scratch_dir = work.join("extracted");
        let measurement = unpack_measure(&archive_path, &scratch_dir, &request).unwrap();
        assert!(measurement.verified);
        assert!(scratch_dir.join("a.txt").is_file());

        std::fs::remove_dir_all(&input).ok();
        std::fs::remove_dir_all(&work).ok();
    }

    #[test]
    fn refuses_an_encrypted_archive_without_a_password() {
        let input = scratch_input("encrypted-missing-password");
        let work = workspace("encrypted-missing-password");
        let creds = TestCredentials::generate().unwrap();
        let write_options = entrybound::crypto::EncryptedWriteOptions {
            password: Some(&creds.password),
            ..Default::default()
        };
        let encrypted = entrybound::crypto::pack_directory_encrypted(
            &input,
            PackOptions {
                profile: CompressionProfile::Fast,
                ..Default::default()
            },
            write_options,
        )
        .unwrap();
        let archive_path = work.join("archive.ecf");
        std::fs::write(&archive_path, &encrypted.bytes).unwrap();

        let request = UnpackRequest {
            layout: Layout::Indexed,
            encrypted: true,
            password: None,
            staging_limits: None,
        };
        let scratch_dir = work.join("extracted");
        let result = unpack_measure(&archive_path, &scratch_dir, &request);
        assert!(matches!(result, Err(UnpackError::MissingPassword)));

        std::fs::remove_dir_all(&input).ok();
        std::fs::remove_dir_all(&work).ok();
    }

    /// STREAM extraction past the in-memory staging limit spills to
    /// `entrybound-stage-<pid>-*.tmp` in the OS temp directory; the sampler
    /// must see that spill (production's own counter says how much there was).
    #[test]
    fn stream_unpack_counts_the_staging_spill() {
        let input = workspace("stream-spill-input");
        let mut state = 0x9E37_79B9_7F4A_7C15_u64;
        let noise: Vec<u8> = (0..24 * 1024 * 1024)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                (state >> 24) as u8
            })
            .collect();
        std::fs::write(input.join("noise.bin"), &noise).unwrap();
        let work = workspace("stream-spill");
        let archive = entrybound::archive::plan_directory(
            &input,
            PackOptions {
                profile: CompressionProfile::Fast,
                ..Default::default()
            },
        )
        .unwrap();
        let mut buf = Vec::new();
        entrybound::ecf::encode_stream(
            &archive,
            entrybound::ecf::StreamWriteOptions::default(),
            &mut buf,
        )
        .unwrap();
        let archive_path = work.join("archive.ecfs");
        std::fs::write(&archive_path, &buf).unwrap();

        let request = UnpackRequest {
            layout: Layout::Stream,
            encrypted: false,
            password: None,
            staging_limits: Some(StagingLimits {
                memory_bytes: 4096,
                total_bytes: 1 << 34,
            }),
        };
        let scratch_dir = work.join("extracted");
        let measurement = unpack_measure(&archive_path, &scratch_dir, &request).unwrap();
        assert!(measurement.verified);
        let staging = measurement.stream_staging.expect("STREAM reports staging");
        assert!(
            staging.spilled_staging_bytes > 0,
            "test setup should force a spill"
        );
        assert!(
            measurement.scratch.peak_staging_spill_bytes > 0,
            "sampler missed a {}-byte staging spill",
            staging.spilled_staging_bytes
        );
        assert!(
            measurement.scratch.peak_total_bytes >= measurement.scratch.peak_staging_spill_bytes
        );

        std::fs::remove_dir_all(&input).ok();
        std::fs::remove_dir_all(&work).ok();
    }
}
