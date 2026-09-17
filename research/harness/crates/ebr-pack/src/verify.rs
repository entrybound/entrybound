//! Standalone verification measurement over an already-encoded archive
//! file: `entrybound::ecf::verify`/`verify_stream`/`crypto::verify_encrypted`,
//! timed, with every `VerificationReport` field surfaced (not just a single
//! collapsed boolean) so a failing run shows exactly which guarantee broke.

use ebr_common::timing::Stopwatch;
use entrybound::crypto::{EncryptedOpenOptions, Unlock, verify_encrypted};
use entrybound::diagnostics::Diagnostic;
use entrybound::eam::Layout;
use entrybound::ecf::{VerificationReport, verify, verify_stream};
use serde::Serialize;
use std::fmt;
use std::fs::File;
use std::path::Path;

#[derive(Debug)]
pub enum VerifyError {
    Entrybound(Diagnostic),
    Io(std::io::Error),
    MissingPassword,
}

impl fmt::Display for VerifyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VerifyError::Entrybound(d) => write!(f, "entrybound: {d}"),
            VerifyError::Io(e) => write!(f, "io: {e}"),
            VerifyError::MissingPassword => write!(
                f,
                "archive is encrypted but no --unlock-password-file was given"
            ),
        }
    }
}

impl std::error::Error for VerifyError {}

impl From<Diagnostic> for VerifyError {
    fn from(d: Diagnostic) -> Self {
        VerifyError::Entrybound(d)
    }
}

impl From<std::io::Error> for VerifyError {
    fn from(e: std::io::Error) -> Self {
        VerifyError::Io(e)
    }
}

#[derive(Debug, Clone)]
pub struct VerifyRequest {
    pub layout: Layout,
    pub encrypted: bool,
    pub password: Option<Vec<u8>>,
}

/// Every `VerificationReport` boolean, individually, plus their conjunction
/// and how long the whole verify pass took.
#[derive(Debug, Clone, Serialize)]
pub struct VerifyMeasurement {
    pub archive_bytes: u64,
    pub verify_seconds: f64,
    pub verified: bool,
    pub canonical_encoding: bool,
    pub container_structure: bool,
    pub section_integrity: bool,
    pub semantic_invariants: bool,
    pub chunk_integrity: bool,
    pub dictionary_integrity: bool,
    pub reconstruction_integrity: bool,
    pub chunk_group_integrity: bool,
    pub access_costs: bool,
    pub content_integrity: bool,
    pub entry_identities: bool,
    pub lai: bool,
    pub pcr: bool,
    pub aux: bool,
    pub pci_computed: bool,
    pub index_status: String,
}

/// Verifies `archive_path` under `request`'s layout/encryption facts.
pub fn verify_measure(
    archive_path: &Path,
    request: &VerifyRequest,
) -> Result<VerifyMeasurement, VerifyError> {
    let bytes = std::fs::read(archive_path)?;
    let archive_bytes = bytes.len() as u64;

    let sw = Stopwatch::start();
    let report = if request.encrypted {
        let password = request
            .password
            .as_deref()
            .ok_or(VerifyError::MissingPassword)?;
        let options = EncryptedOpenOptions::new(Some(Unlock::Password(password)));
        verify_encrypted(&bytes, options)?
    } else {
        match request.layout {
            Layout::Indexed => verify(&bytes)?,
            Layout::Stream => verify_stream(File::open(archive_path)?)?,
        }
    };
    let verify_seconds = sw.elapsed_secs_f64();

    Ok(build_measurement(archive_bytes, verify_seconds, &report))
}

fn build_measurement(
    archive_bytes: u64,
    verify_seconds: f64,
    report: &VerificationReport,
) -> VerifyMeasurement {
    let verified = report.canonical_encoding
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
        && report.pci_computed;

    VerifyMeasurement {
        archive_bytes,
        verify_seconds,
        verified,
        canonical_encoding: report.canonical_encoding,
        container_structure: report.container_structure,
        section_integrity: report.section_integrity,
        semantic_invariants: report.semantic_invariants,
        chunk_integrity: report.chunk_integrity,
        dictionary_integrity: report.dictionary_integrity,
        reconstruction_integrity: report.reconstruction_integrity,
        chunk_group_integrity: report.chunk_group_integrity,
        access_costs: report.access_costs,
        content_integrity: report.content_integrity,
        entry_identities: report.entry_identities,
        lai: report.lai,
        pcr: report.pcr,
        aux: report.aux,
        pci_computed: report.pci_computed,
        index_status: format!("{:?}", report.index_status),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encrypt::TestCredentials;
    use entrybound::archive::PackOptions;
    use entrybound::planner::CompressionProfile;
    use std::path::PathBuf;

    fn scratch_input(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "ebr-pack-verify-test-{label}-{}",
            std::process::id()
        ));
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.txt"), b"entrybound".repeat(300)).unwrap();
        dir
    }

    #[test]
    fn verifies_an_indexed_archive() {
        let input = scratch_input("indexed");
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
        let path = input.join("../archive-indexed.ecf");
        std::fs::write(&path, &encoded.bytes).unwrap();

        let measurement = verify_measure(
            &path,
            &VerifyRequest {
                layout: Layout::Indexed,
                encrypted: false,
                password: None,
            },
        )
        .unwrap();
        assert!(measurement.verified);
        assert!(measurement.canonical_encoding);
        assert_eq!(measurement.archive_bytes, encoded.bytes.len() as u64);

        std::fs::remove_file(&path).ok();
        std::fs::remove_dir_all(&input).ok();
    }

    #[test]
    fn verifies_a_stream_archive() {
        let input = scratch_input("stream");
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
        let path = input.join("../archive-stream.ecfs");
        std::fs::write(&path, &buf).unwrap();

        let measurement = verify_measure(
            &path,
            &VerifyRequest {
                layout: Layout::Stream,
                encrypted: false,
                password: None,
            },
        )
        .unwrap();
        assert!(measurement.verified);

        std::fs::remove_file(&path).ok();
        std::fs::remove_dir_all(&input).ok();
    }

    #[test]
    fn verifies_an_encrypted_archive_with_its_password() {
        let input = scratch_input("encrypted");
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
        let path = input.join("../archive-encrypted.ecf");
        std::fs::write(&path, &encrypted.bytes).unwrap();

        let measurement = verify_measure(
            &path,
            &VerifyRequest {
                layout: Layout::Indexed,
                encrypted: true,
                password: Some(creds.password.clone()),
            },
        )
        .unwrap();
        assert!(measurement.verified);

        std::fs::remove_file(&path).ok();
        std::fs::remove_dir_all(&input).ok();
    }
}
