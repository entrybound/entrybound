//! SHA-256 helpers.
//!
//! Thin wrappers over the `sha2` crate (pinned to the same exact version,
//! `=0.11.0`, that `crates/entrybound` uses) so fingerprints computed by the
//! harness are directly comparable to the ones in
//! `research/corpus/fingerprints/*.json`.

use sha2::{Digest, Sha256};
use std::fmt::Write as _;
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

/// Lower-case hex SHA-256 of bytes already in memory.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    to_hex(hasher.finalize().as_slice())
}

/// Lower-case hex SHA-256 of a file's contents, read in fixed-size chunks so
/// multi-gigabyte corpus items are never fully buffered just to hash them.
pub fn sha256_file(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 1 << 16];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(to_hex(hasher.finalize().as_slice()))
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
    fn empty_input_matches_known_sha256() {
        // The well-known SHA-256 of the empty string, also seen throughout
        // this repo's own JSONL results (e.g. `stdout_sha256` for no output).
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn file_hash_matches_in_memory_hash() {
        let dir = std::env::temp_dir().join(format!("ebr-common-hash-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("sample.bin");
        let data = b"entrybound research harness".repeat(1000);
        std::fs::write(&path, &data).unwrap();
        assert_eq!(sha256_file(&path).unwrap(), sha256_hex(&data));
        std::fs::remove_dir_all(&dir).ok();
    }
}
