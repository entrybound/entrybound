//! Fresh, ephemeral test-only X-Wing recipient and password generation for
//! `--encrypt` runs.
//!
//! This crate never reads or writes a real secret. Every credential this
//! module produces is generated fresh for one measurement run, used only to
//! prove an encrypted archive's round trip (pack -> open -> verify), and
//! (for the password only, see [`TestCredentials::persist_password`]) may be
//! written to a sidecar file purely so a *separate* `ebr-verify`/`ebr-unpack`
//! process invoked later in the same experiment can unlock the same archive
//! -- there is no other channel to share it, since `ebr-pack` never keeps
//! the recipient's secret key or the password past its own process exit.
//! That sidecar file is plainly named (`<archive path>.testpassword`),
//! documented in `README.md`, and never anything a caller is expected to
//! protect.

use entrybound::crypto::{XWingIdentity, XWingRecipient};
use serde::Serialize;
use std::fmt;
use std::fmt::Write as _;

/// Test-only encrypted-archive credentials generated for one pack run.
pub struct TestCredentials {
    /// Kept only so a future measurement could also exercise identity-based
    /// unlock; today's binaries unlock with [`TestCredentials::password`].
    pub identity: XWingIdentity,
    pub recipient: XWingRecipient,
    /// Test-only entropy (see [`generate_test_password`]), never a real
    /// secret and never reused across runs.
    pub password: Vec<u8>,
}

/// The part of [`TestCredentials`] safe to put in a JSON row: a fingerprint
/// and a length, never the password bytes themselves.
#[derive(Debug, Clone, Serialize)]
pub struct CredentialSummary {
    pub recipient_label: String,
    pub recipient_fingerprint_hex: String,
    pub password_len_bytes: usize,
}

#[derive(Debug)]
pub struct CredentialError(pub String);

impl fmt::Display for CredentialError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "generating test encryption credentials: {}", self.0)
    }
}

impl std::error::Error for CredentialError {}

impl From<entrybound::diagnostics::Diagnostic> for CredentialError {
    fn from(d: entrybound::diagnostics::Diagnostic) -> Self {
        CredentialError(d.to_string())
    }
}

impl TestCredentials {
    /// Generates one fresh X-Wing identity/recipient pair and a fresh
    /// 32-byte test password.
    pub fn generate() -> Result<Self, CredentialError> {
        let (identity, recipient) = XWingIdentity::generate()?;
        let password = generate_test_password(32);
        Ok(TestCredentials {
            identity,
            recipient,
            password,
        })
    }

    #[must_use]
    pub fn summary(&self) -> CredentialSummary {
        CredentialSummary {
            recipient_label: self.recipient.label().to_owned(),
            recipient_fingerprint_hex: to_hex(&self.recipient.fingerprint()),
            password_len_bytes: self.password.len(),
        }
    }

    /// Writes the test password to `<archive_path>.testpassword`, so a
    /// separately invoked `ebr-verify`/`ebr-unpack` process can unlock the
    /// same archive. Best-effort: a failure here does not invalidate the
    /// measurement already taken, so callers should log, not fail, on error.
    pub fn persist_password(&self, archive_path: &std::path::Path) -> std::io::Result<std::path::PathBuf> {
        let mut sidecar = archive_path.as_os_str().to_owned();
        sidecar.push(".testpassword");
        let sidecar = std::path::PathBuf::from(sidecar);
        std::fs::write(&sidecar, &self.password)?;
        Ok(sidecar)
    }
}

/// Test-only entropy: not a cryptographically reviewed RNG, only ever used
/// to generate a password for an archive this same run creates and (usually)
/// deletes. Draws from wall-clock nanoseconds, the process id, and an
/// incrementing counter, each folded through SHA-256 -- the same technique
/// `ebr_common::timing::generate_run_id` uses for run ids, extended to
/// however many bytes are requested.
#[must_use]
pub fn generate_test_password(len: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(len);
    let mut counter: u64 = 0;
    while out.len() < len {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        let mut seed = Vec::with_capacity(24);
        seed.extend_from_slice(&now.as_nanos().to_le_bytes());
        seed.extend_from_slice(&(std::process::id() as u64).to_le_bytes());
        seed.extend_from_slice(&counter.to_le_bytes());
        let digest_hex = ebr_common::hash::sha256_hex(&seed);
        for pair in digest_hex.as_bytes().chunks(2) {
            if out.len() >= len {
                break;
            }
            let byte_str = std::str::from_utf8(pair).unwrap_or("00");
            out.push(u8::from_str_radix(byte_str, 16).unwrap_or(0));
        }
        counter += 1;
    }
    out.truncate(len);
    out
}

fn to_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(s, "{b:02x}");
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_a_password_of_the_requested_length() {
        let password = generate_test_password(32);
        assert_eq!(password.len(), 32);
        let again = generate_test_password(32);
        assert_ne!(
            password, again,
            "two consecutive test passwords should not collide"
        );
    }

    #[test]
    fn generates_usable_test_credentials() {
        let creds = TestCredentials::generate().expect("credential generation should succeed");
        assert_eq!(creds.password.len(), 32);
        let summary = creds.summary();
        assert_eq!(summary.password_len_bytes, 32);
        assert_eq!(summary.recipient_fingerprint_hex.len(), 64);
    }

    #[test]
    fn persists_and_removes_the_password_sidecar() {
        let dir = std::env::temp_dir().join(format!(
            "ebr-pack-encrypt-test-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let archive_path = dir.join("archive.ecf");
        std::fs::write(&archive_path, b"placeholder").unwrap();

        let creds = TestCredentials::generate().unwrap();
        let sidecar = creds.persist_password(&archive_path).unwrap();
        assert!(sidecar.ends_with("archive.ecf.testpassword"));
        assert_eq!(std::fs::read(&sidecar).unwrap(), creds.password);

        std::fs::remove_dir_all(&dir).ok();
    }
}
