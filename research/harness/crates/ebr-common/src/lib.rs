//! Shared library for the Entrybound research harness (Phase B2).
//!
//! Nothing here is a stable API, nothing here changes production behavior,
//! and nothing here reads held-out corpus *content* (see [`heldout`]). See
//! `research/harness/README.md` for the harness contract this crate exists
//! to serve.

pub mod cli;
pub mod hash;
pub mod heldout;
pub mod manifest;
pub mod measure;
pub mod results;
pub mod runenv;
pub mod timing;
pub mod walk;

use std::path::{Path, PathBuf};

/// Best-effort repo root discovery for a binary built from this workspace:
/// walks up from `start` (typically `env!("CARGO_MANIFEST_DIR")`) until it
/// finds a directory that looks like the entrybound repo root (it contains
/// both `crates/entrybound` and `research/harness`). Returns `None` rather
/// than guessing when it cannot tell.
pub fn discover_repo_root(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .find(|dir| {
            dir.join("crates").join("entrybound").is_dir()
                && dir.join("research").join("harness").is_dir()
        })
        .map(Path::to_path_buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovers_this_repo_root_from_its_own_manifest_dir() {
        let start = Path::new(env!("CARGO_MANIFEST_DIR"));
        let root = discover_repo_root(start).expect("should find the entrybound repo root");
        assert!(root.join("Cargo.toml").is_file());
        assert!(
            root.join("research")
                .join("harness")
                .join("Cargo.toml")
                .is_file()
        );
    }
}
