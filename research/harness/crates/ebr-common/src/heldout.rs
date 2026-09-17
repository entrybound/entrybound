//! Held-out corpus guard.
//!
//! Equivalent in *effect* (deliberately not in code -- this is Rust, the
//! source of truth is Python) to `research/corpus/tools/corpuslib.py`'s
//! `assert_not_heldout`/`Layout`: refuse to touch any path that resolves
//! under a held-out root. The default root `/root/eb-research/heldout` is
//! always guarded; `$EB_DATA_ROOT/heldout` and `$EB_HELDOUT_ROOT` are guarded
//! in addition when set, and so is any `\\wsl.localhost\<distro>\...` or
//! `\\wsl$\<distro>\...` path into the default root (see
//! [`HeldoutGuard`] for why, harness review round 1 finding R1-01). Every
//! harness binary calls [`assert_inputs_not_heldout`] on every input path it
//! is given before reading anything.
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

/// Default data root, matching `research/corpus/tools/corpuslib.py`'s
/// `DEFAULT_DATA_ROOT`.
pub const DEFAULT_DATA_ROOT: &str = "/root/eb-research";

/// Guards every held-out root this process could plausibly reach, with an
/// optional unlocked commit id resolved at construction time.
///
/// Harness review round 1 (finding R1-01) found that a single root derived
/// from `EB_DATA_ROOT` let an unrelated `EB_DATA_ROOT` override silently stop
/// protecting the real `/root/eb-research/heldout`, and that a Windows-side
/// binary reaching the WSL distro through `\\wsl.localhost\<distro>\...` or
/// `\\wsl$\<distro>\...` was never matched at all. The guard therefore always
/// includes the default root, adds `EB_DATA_ROOT/heldout` and
/// `EB_HELDOUT_ROOT` (the Python runner's override, `research/tools/ebr/paths.py`)
/// when set, and additionally refuses any WSL UNC path whose distro-relative
/// part is under `/root/eb-research/heldout`.
#[derive(Debug, Clone)]
pub struct HeldoutGuard {
    heldout_roots: Vec<PathBuf>,
    unlocked: bool,
}

impl HeldoutGuard {
    /// Builds a guard directly, without touching the environment. `unlock`
    /// is the value of `EB_HELDOUT_UNLOCK`, if any; `repo_root` is where
    /// `research/decisions/design-freeze.json` is looked up. The default
    /// `/root/eb-research/heldout` root is always guarded in addition to
    /// `data_root/heldout`.
    pub fn new(data_root: impl Into<PathBuf>, repo_root: &Path, unlock: Option<&str>) -> Self {
        let mut heldout_roots = vec![data_root.into().join("heldout")];
        push_unique(
            &mut heldout_roots,
            Path::new(DEFAULT_DATA_ROOT).join("heldout"),
        );
        let unlocked = unlock.is_some_and(|wanted| design_freeze_names(repo_root, wanted));
        HeldoutGuard {
            heldout_roots,
            unlocked,
        }
    }

    /// Reads `EB_DATA_ROOT` (default `/root/eb-research`, matching
    /// corpuslib.py), `EB_HELDOUT_ROOT`, and `EB_HELDOUT_UNLOCK` from the
    /// process environment. Every root named by any of them is guarded, and
    /// the default root is guarded regardless.
    pub fn from_env(repo_root: &Path) -> Self {
        let data_root = std::env::var("EB_DATA_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(DEFAULT_DATA_ROOT));
        let unlock = std::env::var("EB_HELDOUT_UNLOCK").ok();
        let mut guard = Self::new(data_root, repo_root, unlock.as_deref());
        if let Ok(root) = std::env::var("EB_HELDOUT_ROOT") {
            push_unique(&mut guard.heldout_roots, PathBuf::from(root));
        }
        guard
    }

    /// A guard over an exact held-out root, never unlockable. Handy for
    /// tests and for callers that only want the path check. The WSL UNC
    /// check still applies.
    pub fn with_root(heldout_root: impl Into<PathBuf>) -> Self {
        HeldoutGuard {
            heldout_roots: vec![heldout_root.into()],
            unlocked: false,
        }
    }

    /// Every root this guard refuses.
    pub fn roots(&self) -> &[PathBuf] {
        &self.heldout_roots
    }

    /// Whether `path` resolves under any guarded held-out root. Both sides go
    /// through [`canonicalize_best_effort`], so a not-yet-created path (a
    /// corpus item that has not been materialized, say) still compares
    /// correctly against the held-out root: on Windows in particular,
    /// `fs::canonicalize` prefixes its result with `\\?\`, so comparing a
    /// canonicalized root against a literal, non-canonicalized child path
    /// would silently never match. Symlinks are resolved for the existing
    /// part of the path; hard links and copies cannot be detected by any
    /// path check.
    pub fn is_heldout(&self, path: &Path) -> bool {
        // A path that is lexically under a held-out root (after resolving
        // `.`/`..` without touching the filesystem) is refused before any
        // `stat` of it happens, so the guard never probes held-out entries.
        if is_wsl_unc_heldout(path) {
            return true;
        }
        let lexical = lexical_absolute(path);
        if self.heldout_roots.iter().any(|root| {
            let root = lexical_absolute(root);
            lexical == root || lexical.starts_with(&root)
        }) {
            return true;
        }
        // Otherwise resolve symlinks in the existing part of the path.
        let real = canonicalize_best_effort(path);
        if is_wsl_unc_heldout(&real) {
            return true;
        }
        self.heldout_roots.iter().any(|root| {
            let root = canonicalize_best_effort(root);
            real == root || real.starts_with(&root)
        })
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

/// The check every harness binary runs on every input path it was given
/// (`--item-path`, `--archive-path`, `--versions`, `--root`, ...) before
/// reading anything: builds a [`HeldoutGuard::from_env`] guard and refuses
/// the first held-out path.
pub fn assert_inputs_not_heldout<P: AsRef<Path>>(
    repo_root: &Path,
    paths: &[P],
) -> Result<(), HeldoutError> {
    let guard = HeldoutGuard::from_env(repo_root);
    for path in paths {
        guard.assert_not_heldout(path.as_ref())?;
    }
    Ok(())
}

/// `path` made absolute against the current directory and with `.`/`..`
/// resolved purely lexically (no filesystem access).
fn lexical_absolute(path: &Path) -> PathBuf {
    let joined = if path.has_root() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|dir| dir.join(path))
            .unwrap_or_else(|_| path.to_path_buf())
    };
    let mut out = PathBuf::new();
    for component in joined.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

fn push_unique(roots: &mut Vec<PathBuf>, root: PathBuf) {
    if !roots.contains(&root) {
        roots.push(root);
    }
}

/// Matches `\\wsl.localhost\<distro>\root\eb-research\heldout[\...]` and
/// `\\wsl$\<distro>\...` (also in their `\\?\UNC\` verbatim forms), case-
/// insensitively and with either separator, on every platform: the Windows
/// host reaches the WSL distro's held-out root only through these paths.
fn is_wsl_unc_heldout(path: &Path) -> bool {
    let text = path
        .to_string_lossy()
        .replace('\\', "/")
        .to_ascii_lowercase();
    let rest = if let Some(rest) = text.strip_prefix("//?/unc/") {
        rest
    } else if let Some(rest) = text.strip_prefix("//") {
        rest
    } else {
        return false;
    };
    let Some((host, rest)) = rest.split_once('/') else {
        return false;
    };
    if host != "wsl.localhost" && host != "wsl$" {
        return false;
    }
    let Some((_distro, distro_path)) = rest.split_once('/') else {
        return false;
    };
    distro_path
        .trim_end_matches('/')
        .strip_prefix("root/eb-research/heldout")
        .is_some_and(|tail| tail.is_empty() || tail.starts_with('/'))
}

/// Canonicalizes the nearest existing ancestor of `path` and re-appends
/// whatever tail does not exist yet, instead of giving up and returning
/// `path` unchanged the moment `fs::canonicalize` fails. This matters
/// specifically on Windows: `fs::canonicalize` returns a `\\?\`-prefixed
/// verbatim path, so a literal path and its canonicalized self never compare
/// equal by prefix, and a not-yet-created child of an existing, canonicalized
/// directory needs the same prefix to compare correctly against it.
fn canonicalize_best_effort(path: &Path) -> PathBuf {
    use std::path::Component;
    let mut existing = path;
    let mut missing_tail: Vec<Component<'_>> = Vec::new();
    while !existing.exists() {
        let mut components = existing.components();
        match components.next_back() {
            Some(last @ (Component::Normal(_) | Component::ParentDir | Component::CurDir)) => {
                missing_tail.push(last);
                existing = components.as_path();
            }
            _ => break,
        }
    }
    let mut result = std::fs::canonicalize(existing).unwrap_or_else(|_| existing.to_path_buf());
    // The tail does not exist, so none of it can be a symlink: `..` and `.`
    // in it are resolved lexically.
    for component in missing_tail.into_iter().rev() {
        match component {
            Component::ParentDir => {
                result.pop();
            }
            Component::CurDir => {}
            other => result.push(other.as_os_str()),
        }
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
    fn always_guards_the_default_root_even_when_data_root_points_elsewhere() {
        let base = scratch_dir("default-root");
        let repo_root = base.join("repo");
        std::fs::create_dir_all(&repo_root).unwrap();
        let guard = HeldoutGuard::new(base.join("some-other-data-root"), &repo_root, None);
        let default_item = Path::new(DEFAULT_DATA_ROOT)
            .join("heldout")
            .join("__ebr_guard_selftest_nonexistent__");
        assert!(guard.is_heldout(&default_item));
        assert!(guard.assert_not_heldout(&default_item).is_err());
        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn refuses_dot_dot_paths_that_lexically_land_in_the_heldout_root() {
        let guard = HeldoutGuard::with_root("/nonexistent-root-for-test/heldout");
        assert!(guard.is_heldout(Path::new(
            "/nonexistent-root-for-test/corpus/../heldout/item"
        )));
        assert!(!guard.is_heldout(Path::new(
            "/nonexistent-root-for-test/heldout/../corpus/item"
        )));
    }

    #[test]
    fn refuses_wsl_unc_paths_to_the_heldout_root() {
        let guard = HeldoutGuard::with_root("/nonexistent-root-for-test/heldout");
        for raw in [
            r"\\wsl.localhost\Ubuntu\root\eb-research\heldout\f05-item\a.bin",
            r"\\wsl$\Ubuntu\root\eb-research\heldout",
            r"\\?\UNC\wsl.localhost\Ubuntu\root\eb-research\HELDOUT\x",
            "//wsl.localhost/Ubuntu/root/eb-research/heldout/x",
        ] {
            assert!(guard.is_heldout(Path::new(raw)), "{raw} should be refused");
        }
        for raw in [
            r"\\wsl.localhost\Ubuntu\root\eb-research\corpus\f05-item",
            r"\\wsl.localhost\Ubuntu\root\eb-research\heldout-notes.txt",
            r"\\fileserver\share\root\eb-research\heldout",
        ] {
            assert!(!guard.is_heldout(Path::new(raw)), "{raw} should be allowed");
        }
    }

    #[cfg(unix)]
    #[test]
    fn refuses_a_symlink_that_points_into_the_heldout_root() {
        let base = scratch_dir("symlink");
        let heldout = base.join("data").join("heldout").join("item");
        std::fs::create_dir_all(&heldout).unwrap();
        let link = base.join("innocent-looking-link");
        std::fs::remove_file(&link).ok();
        std::os::unix::fs::symlink(&heldout, &link).unwrap();
        let guard = HeldoutGuard::with_root(base.join("data").join("heldout"));
        assert!(guard.is_heldout(&link));
        assert!(guard.is_heldout(&link.join("not-yet-created-child")));
        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn assert_inputs_not_heldout_refuses_the_default_root() {
        let base = scratch_dir("inputs");
        let repo_root = base.join("repo");
        std::fs::create_dir_all(&repo_root).unwrap();
        let allowed = base.join("allowed");
        let refused = Path::new(DEFAULT_DATA_ROOT)
            .join("heldout")
            .join("__ebr_guard_selftest_nonexistent__");
        assert!(assert_inputs_not_heldout(&repo_root, &[allowed.as_path()]).is_ok());
        assert!(
            assert_inputs_not_heldout(&repo_root, &[allowed.as_path(), refused.as_path()]).is_err()
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
