//! Held-out corpus guard.
//!
//! Equivalent in *effect* (deliberately not in code -- this is Rust, the
//! source of truth is Python) to `research/corpus/tools/corpuslib.py`'s
//! `assert_not_heldout`/`Layout`: refuse to touch any path that resolves
//! under the held-out root, which defaults to `/root/eb-research/heldout`
//! (`$EB_DATA_ROOT` overrides the `/root/eb-research` prefix, exactly like
//! corpuslib.py's `Layout(data_root=...)`).
//!
//! Unlike corpuslib.py's `assert_not_heldout` (which is unconditional -- only
//! its separate, stricter `check_heldout_unlock` two-key ceremony grants
//! content access, and that is not reproduced here), this guard's refusal
//! *can* be lifted, by design, when both of the following hold:
//!
//! 1. `EB_HELDOUT_UNLOCK` names a value (intended to be a 40-hex commit id,
//!    though this crate does not itself enforce that shape -- see below).
//! 2. `research/decisions/design-freeze.json` exists, parses as JSON, and one
//!    of the keys `design_freeze_sha`, `commit_sha`, or `sha` (checked in
//!    that order -- `PROGRESS.md`'s "Open integration items" section fixes
//!    this exact key set for whatever writes that file) equals that value.
//!
//! As of this writing (`research/decisions/design-freeze.json` does not
//! exist -- Phase D, design freeze, has not run) unlocking is impossible: with
//! no file to match against, every held-out path is refused unconditionally
//! regardless of what `EB_HELDOUT_UNLOCK` is set to. That is the safe default
//! and it is intentional: this guard fails closed, not open, whenever the
//! freeze file is missing, unreadable, or malformed.
//!
//! This module does not itself validate that the matched value looks like a
//! git commit, nor that it is an ancestor of HEAD (corpuslib.py's
//! `check_heldout_unlock` does both, plus a `heldout-lock.json` "frozen"
//! check, against a different, more elaborate two-file scheme under
//! `research/corpus/`). A caller that needs those stronger guarantees should
//! still shell out to `corpuslib.py` rather than relying on this crate alone.

use std::path::{Path, PathBuf};

/// Error returned when a path is refused.
#[derive(Debug)]
pub struct HeldoutError {
    path: PathBuf,
}

impl std::fmt::Display for HeldoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "refusing to read held-out path {}", self.path.display())
    }
}

impl std::error::Error for HeldoutError {}

/// Guards one held-out root, with an optional unlocked commit id resolved at
/// construction time.
#[derive(Debug, Clone)]
pub struct HeldoutGuard {
    heldout_root: PathBuf,
    unlocked: bool,
}

impl HeldoutGuard {
    /// Builds a guard directly, without touching the environment. `unlock`
    /// is the value of `EB_HELDOUT_UNLOCK`, if any; `repo_root` is where
    /// `research/decisions/design-freeze.json` is looked up.
    pub fn new(data_root: impl Into<PathBuf>, repo_root: &Path, unlock: Option<&str>) -> Self {
        let heldout_root = data_root.into().join("heldout");
        let unlocked = unlock.is_some_and(|wanted| design_freeze_names(repo_root, wanted));
        HeldoutGuard {
            heldout_root,
            unlocked,
        }
    }

    /// Reads `EB_DATA_ROOT` (default `/root/eb-research`, matching
    /// corpuslib.py) and `EB_HELDOUT_UNLOCK` from the process environment.
    pub fn from_env(repo_root: &Path) -> Self {
        let data_root = std::env::var("EB_DATA_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/root/eb-research"));
        let unlock = std::env::var("EB_HELDOUT_UNLOCK").ok();
        Self::new(data_root, repo_root, unlock.as_deref())
    }

    /// A guard over an exact held-out root, never unlockable. Handy for
    /// tests and for callers that only want the path check.
    pub fn with_root(heldout_root: impl Into<PathBuf>) -> Self {
        HeldoutGuard {
            heldout_root: heldout_root.into(),
            unlocked: false,
        }
    }

    /// Whether `path` resolves under the held-out root. Both sides go
    /// through [`canonicalize_best_effort`], so a not-yet-created path (a
    /// corpus item that has not been materialized, say) still compares
    /// correctly against the held-out root: on Windows in particular,
    /// `fs::canonicalize` prefixes its result with `\\?\`, so comparing a
    /// canonicalized root against a literal, non-canonicalized child path
    /// would silently never match.
    pub fn is_heldout(&self, path: &Path) -> bool {
        let real = canonicalize_best_effort(path);
        let root = canonicalize_best_effort(&self.heldout_root);
        real == root || real.starts_with(&root)
    }

    /// True once `EB_HELDOUT_UNLOCK` has been checked against
    /// `research/decisions/design-freeze.json` and matched one of its
    /// accepted keys.
    pub fn is_unlocked(&self) -> bool {
        self.unlocked
    }

    /// Call before opening, reading, or hashing any corpus path. Refuses a
    /// held-out path unless this guard is unlocked.
    pub fn assert_not_heldout(&self, path: &Path) -> Result<(), HeldoutError> {
        if self.is_heldout(path) && !self.unlocked {
            return Err(HeldoutError {
                path: path.to_path_buf(),
            });
        }
        Ok(())
    }
}

/// Canonicalizes the nearest existing ancestor of `path` and re-appends
/// whatever tail does not exist yet, instead of giving up and returning
/// `path` unchanged the moment `fs::canonicalize` fails. This matters
/// specifically on Windows: `fs::canonicalize` returns a `\\?\`-prefixed
/// verbatim path, so a literal path and its canonicalized self never compare
/// equal by prefix, and a not-yet-created child of an existing, canonicalized
/// directory needs the same prefix to compare correctly against it.
fn canonicalize_best_effort(path: &Path) -> PathBuf {
    let mut existing = path;
    let mut missing_tail: Vec<&std::ffi::OsStr> = Vec::new();
    while !existing.exists() {
        match (existing.file_name(), existing.parent()) {
            (Some(name), Some(parent)) => {
                missing_tail.push(name);
                existing = parent;
            }
            _ => break,
        }
    }
    let mut result = std::fs::canonicalize(existing).unwrap_or_else(|_| existing.to_path_buf());
    for component in missing_tail.into_iter().rev() {
        result.push(component);
    }
    result
}

fn design_freeze_names(repo_root: &Path, wanted: &str) -> bool {
    let freeze_path = repo_root.join("research/decisions/design-freeze.json");
    let Ok(text) = std::fs::read_to_string(&freeze_path) else {
        return false;
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
        return false;
    };
    ["design_freeze_sha", "commit_sha", "sha"]
        .iter()
        .filter_map(|key| value.get(key).and_then(|v| v.as_str()))
        .any(|found| found == wanted)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "ebr-common-heldout-test-{label}-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).ok();
        dir
    }

    #[test]
    fn refuses_path_under_heldout_root_without_unlock() {
        let base = scratch_dir("basic");
        let data_root = base.join("data");
        let heldout = data_root.join("heldout");
        std::fs::create_dir_all(heldout.join("f05-item")).unwrap();
        let repo_root = base.join("repo"); // no design-freeze.json here
        std::fs::create_dir_all(&repo_root).unwrap();

        let guard = HeldoutGuard::new(&data_root, &repo_root, None);
        assert!(guard.is_heldout(&heldout.join("f05-item")));
        assert!(guard.assert_not_heldout(&heldout.join("f05-item")).is_err());
        assert!(!guard.is_unlocked());

        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn allows_non_heldout_paths() {
        let base = scratch_dir("nonheldout");
        let data_root = base.join("data");
        std::fs::create_dir_all(data_root.join("corpus").join("tuning")).unwrap();
        let repo_root = base.join("repo");
        std::fs::create_dir_all(&repo_root).unwrap();

        let guard = HeldoutGuard::new(&data_root, &repo_root, None);
        let ok_path = data_root.join("corpus").join("tuning").join("item");
        assert!(!guard.is_heldout(&ok_path));
        assert!(guard.assert_not_heldout(&ok_path).is_ok());

        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn refuses_even_with_unlock_env_when_design_freeze_file_is_absent() {
        let base = scratch_dir("no-freeze-file");
        let data_root = base.join("data");
        std::fs::create_dir_all(data_root.join("heldout")).unwrap();
        let repo_root = base.join("repo"); // deliberately no research/decisions/design-freeze.json
        std::fs::create_dir_all(&repo_root).unwrap();

        let guard = HeldoutGuard::new(
            &data_root,
            &repo_root,
            Some("cccccccccccccccccccccccccccccccccccccccc"),
        );
        assert!(!guard.is_unlocked());
        assert!(
            guard
                .assert_not_heldout(&data_root.join("heldout").join("x"))
                .is_err()
        );

        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn unlocks_only_when_env_matches_design_freeze_json() {
        let base = scratch_dir("matching-freeze");
        let data_root = base.join("data");
        std::fs::create_dir_all(data_root.join("heldout")).unwrap();
        let repo_root = base.join("repo");
        let decisions_dir = repo_root.join("research").join("decisions");
        std::fs::create_dir_all(&decisions_dir).unwrap();
        let commit = "a".repeat(40);
        std::fs::write(
            decisions_dir.join("design-freeze.json"),
            format!(r#"{{"design_freeze_sha": "{commit}"}}"#),
        )
        .unwrap();

        let matching = HeldoutGuard::new(&data_root, &repo_root, Some(commit.as_str()));
        assert!(matching.is_unlocked());
        assert!(
            matching
                .assert_not_heldout(&data_root.join("heldout").join("x"))
                .is_ok()
        );

        let mismatched = HeldoutGuard::new(&data_root, &repo_root, Some("b".repeat(40).as_str()));
        assert!(!mismatched.is_unlocked());
        assert!(
            mismatched
                .assert_not_heldout(&data_root.join("heldout").join("x"))
                .is_err()
        );

        std::fs::remove_dir_all(&base).ok();
    }
}
