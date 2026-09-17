//! Standalone full verify+extract measurement over an already-encoded
//! archive file, independent of a fresh pack (see [`crate::pack`] for that).
//!
//! Extraction runs on the caller's thread while [`crate::scratch::ScratchSampler`]
//! polls the destination directory's total size on a background thread, so
//! `peak_scratch_bytes` is a real, sampled high-water mark of on-disk usage
//! during extraction, not an estimate derived from the archive's logical
//! size.

use crate::scratch::ScratchSampler;
use ebr_common::timing::Stopwatch;
use entrybound::archive::{ExtractionPolicy, unpack, unpack_opened, unpack_stream};
use entrybound::crypto::{EncryptedOpenOptions, Unlock, open_encrypted_authenticated};
use entrybound::diagnostics::Diagnostic;
use entrybound::ecf::{bootstrap_sequential_limits, open, open_stream};
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
}

#[derive(Debug, Clone, Serialize)]
pub struct UnpackMeasurement {
    pub archive_bytes: u64,
    pub verified: bool,
    pub open_and_verify_seconds: f64,
    pub extract_seconds: f64,
    pub entries_created: u64,
    pub extracted_logical_bytes: u64,
    pub peak_scratch_bytes: u64,
    pub scratch_sample_count: u64,
}

/// Opens, verifies, and fully extracts `archive_path` into `scratch_dir`
/// (created if missing), sampling `scratch_dir`'s peak on-disk size while
/// extraction runs. `scratch_dir` is left in place afterward -- the caller
/// (typically a short-lived `ebr-unpack` process) is responsible for
/// removing it once it has inspected or hashed the results, mirroring how
/// `ebr-pack` leaves its produced archive on disk for later inspection.
pub fn unpack_measure(
    archive_path: &Path,
    scratch_dir: &Path,
    request: &UnpackRequest,
) -> Result<UnpackMeasurement, UnpackError> {
    let bytes = std::fs::read(archive_path)?;
    let archive_bytes = bytes.len() as u64;
    std::fs::create_dir_all(scratch_dir)?;

    let policy = ExtractionPolicy::default();
    let sampler = ScratchSampler::start(scratch_dir.to_path_buf(), ScratchSampler::default_interval());

    let open_sw = Stopwatch::start();
    let (verified, open_and_verify_seconds, extract_seconds, extraction) = if request.encrypted {
        let password = request
            .password
            .as_deref()
            .ok_or(UnpackError::MissingPassword)?;
        let authenticated = open_encrypted_authenticated(
            &bytes,
            EncryptedOpenOptions::new(Some(Unlock::Password(password))),
        )?;
        let verified = verification_all_true(&authenticated.opened.report);
        let open_and_verify_seconds = open_sw.elapsed_secs_f64();
        let extract_sw = Stopwatch::start();
        let extraction = unpack_opened(&authenticated.opened, scratch_dir, policy)?;
        (verified, open_and_verify_seconds, extract_sw.elapsed_secs_f64(), extraction)
    } else {
        match request.layout {
            entrybound::eam::Layout::Indexed => {
                let opened = open(&bytes)?;
                let verified = verification_all_true(&opened.report);
                let open_and_verify_seconds = open_sw.elapsed_secs_f64();
                let extract_sw = Stopwatch::start();
                let extraction = unpack(&bytes, scratch_dir, policy)?;
                (verified, open_and_verify_seconds, extract_sw.elapsed_secs_f64(), extraction)
            }
            entrybound::eam::Layout::Stream => {
                let sequential = open_stream(std::io::Cursor::new(bytes.clone()))?;
                let verified = verification_all_true(&sequential.opened.report);
                let open_and_verify_seconds = open_sw.elapsed_secs_f64();
                let extract_sw = Stopwatch::start();
                let (extraction, _stream_report) = unpack_stream(
                    File::open(archive_path)?,
                    scratch_dir,
                    policy,
                    bootstrap_sequential_limits(),
                )?;
                (verified, open_and_verify_seconds, extract_sw.elapsed_secs_f64(), extraction)
            }
        }
    };

    let peak = sampler.stop();

    Ok(UnpackMeasurement {
        archive_bytes,
        verified,
        open_and_verify_seconds,
        extract_seconds,
        entries_created: extraction.entries_created,
        extracted_logical_bytes: extraction.logical_bytes_written,
        peak_scratch_bytes: peak.peak_bytes,
        scratch_sample_count: peak.sample_count,
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
        let encoded = entrybound::ecf::encode(&archive, entrybound::ecf::WriteOptions::default()).unwrap();
        let archive_path = work.join("archive.ecf");
        std::fs::write(&archive_path, &encoded.bytes).unwrap();

        let request = UnpackRequest {
            layout: Layout::Indexed,
            encrypted: false,
            password: None,
        };
        let scratch_dir = work.join("extracted");
        let measurement = unpack_measure(&archive_path, &scratch_dir, &request).unwrap();

        assert!(measurement.verified);
        assert_eq!(measurement.entries_created, 3); // sub/ dir + 2 files
        assert!(measurement.extracted_logical_bytes > 0);
        assert!(measurement.peak_scratch_bytes >= measurement.extracted_logical_bytes);
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
        entrybound::ecf::encode_stream(&archive, entrybound::ecf::StreamWriteOptions::default(), &mut buf)
            .unwrap();
        let archive_path = work.join("archive.ecfs");
        std::fs::write(&archive_path, &buf).unwrap();

        let request = UnpackRequest {
            layout: Layout::Stream,
            encrypted: false,
            password: None,
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
        };
        let scratch_dir = work.join("extracted");
        let result = unpack_measure(&archive_path, &scratch_dir, &request);
        assert!(matches!(result, Err(UnpackError::MissingPassword)));

        std::fs::remove_dir_all(&input).ok();
        std::fs::remove_dir_all(&work).ok();
    }
}
