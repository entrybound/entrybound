//! Tree walking of corpus items into in-memory or streaming inputs.
//!
//! A corpus item is a directory tree (see `research/corpus/tools/corpuslib.py`
//! `FAMILIES` and `KINDS`). [`walk_tree`] lists one without reading any file
//! contents, which is always safe to call regardless of item size.
//! [`read_tree_in_memory`] additionally reads every regular file's bytes,
//! subject to an explicit byte budget, for callers happy to hold a
//! small/tuning-scale item fully in memory. Neither function performs the
//! held-out check itself: callers must pass the item's path through
//! [`crate::heldout::HeldoutGuard::assert_not_heldout`] first. Keeping that
//! check at the call site (not baked into these functions) means a caller
//! that also wants to walk a non-corpus directory (a scratch/staging tree,
//! for instance) is not forced to thread a guard through for no reason.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// One filesystem object under a walked tree, relative to its root.
#[derive(Debug, Clone)]
pub struct TreeEntry {
    pub relative_path: PathBuf,
    pub is_dir: bool,
    pub is_symlink: bool,
    /// Byte length for regular files; `0` for directories and symlinks (a
    /// symlink's target length is a separate, deliberately unread, question).
    pub len: u64,
}

/// Walks `root` depth-first (children sorted by file name for determinism)
/// and yields one [`TreeEntry`] per filesystem object: files, directories,
/// and symlinks. Symlinks are listed, never followed.
pub fn walk_tree(root: &Path) -> io::Result<Vec<TreeEntry>> {
    let mut out = Vec::new();
    walk_into(root, root, &mut out)?;
    Ok(out)
}

fn walk_into(root: &Path, dir: &Path, out: &mut Vec<TreeEntry>) -> io::Result<()> {
    let mut entries: Vec<fs::DirEntry> = fs::read_dir(dir)?.collect::<Result<_, _>>()?;
    entries.sort_by_key(fs::DirEntry::file_name);
    for entry in entries {
        let path = entry.path();
        // `DirEntry::metadata` does not follow a symlink entry, unlike
        // `fs::metadata`; that is exactly the "list, never follow" contract
        // this walker needs.
        let meta = entry.metadata()?;
        let relative_path = path
            .strip_prefix(root)
            .map(Path::to_path_buf)
            .unwrap_or_else(|_| path.clone());
        let is_symlink = meta.file_type().is_symlink();
        let is_dir = meta.file_type().is_dir();
        out.push(TreeEntry {
            relative_path: relative_path.clone(),
            is_dir,
            is_symlink,
            len: if is_dir || is_symlink { 0 } else { meta.len() },
        });
        if is_dir && !is_symlink {
            walk_into(root, &path, out)?;
        }
    }
    Ok(())
}

/// Total bytes across every regular file entry (directories and symlinks
/// contribute nothing, matching [`TreeEntry::len`]).
pub fn total_bytes(entries: &[TreeEntry]) -> u64 {
    entries.iter().map(|e| e.len).sum()
}

/// A tree fully materialized in memory: every regular file's relative path
/// and bytes. Directory and symlink entries are listed but their contents
/// (a symlink's target, in particular) are never read here.
#[derive(Debug)]
pub struct InMemoryTree {
    pub entries: Vec<TreeEntry>,
    pub files: Vec<(PathBuf, Vec<u8>)>,
    pub total_bytes: u64,
}

/// Reads every regular file under `root` fully into memory, refusing before
/// exceeding `max_total_bytes`. Intended for small/tuning-scale items; a
/// validation/large-scale item should be measured through [`walk_tree`] plus
/// a streaming reader (or the production
/// `entrybound::archive::filesystem::pack_directory_stream`) instead.
pub fn read_tree_in_memory(root: &Path, max_total_bytes: u64) -> io::Result<InMemoryTree> {
    let entries = walk_tree(root)?;
    let mut total = 0u64;
    let mut files = Vec::new();
    for entry in &entries {
        if entry.is_dir || entry.is_symlink {
            continue;
        }
        total = total.saturating_add(entry.len);
        if total > max_total_bytes {
            return Err(io::Error::other(format!(
                "tree at {} exceeds the {max_total_bytes}-byte in-memory budget; \
                 use walk_tree plus a streaming reader instead",
                root.display()
            )));
        }
        let full_path = root.join(&entry.relative_path);
        files.push((entry.relative_path.clone(), fs::read(&full_path)?));
    }
    Ok(InMemoryTree {
        entries,
        files,
        total_bytes: total,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "ebr-common-walk-test-{label}-{}",
            std::process::id()
        ));
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn walks_files_and_directories() {
        let root = scratch_dir("walk-basic");
        std::fs::create_dir_all(root.join("sub")).unwrap();
        std::fs::write(root.join("a.txt"), b"hello").unwrap();
        std::fs::write(root.join("sub").join("b.txt"), b"world!").unwrap();

        let entries = walk_tree(&root).unwrap();
        let names: Vec<_> = entries
            .iter()
            .map(|e| e.relative_path.to_string_lossy().replace('\\', "/"))
            .collect();
        assert!(names.contains(&"a.txt".to_string()));
        assert!(names.contains(&"sub".to_string()));
        assert!(names.contains(&"sub/b.txt".to_string()));
        assert_eq!(total_bytes(&entries), 5 + 6);

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn reads_tree_in_memory_within_budget() {
        let root = scratch_dir("walk-mem-ok");
        std::fs::write(root.join("a.txt"), b"0123456789").unwrap();

        let tree = read_tree_in_memory(&root, 100).unwrap();
        assert_eq!(tree.total_bytes, 10);
        assert_eq!(tree.files.len(), 1);
        assert_eq!(tree.files[0].1, b"0123456789");

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn refuses_to_read_tree_over_budget() {
        let root = scratch_dir("walk-mem-over");
        std::fs::write(root.join("a.txt"), vec![0u8; 1000]).unwrap();

        let result = read_tree_in_memory(&root, 10);
        assert!(result.is_err());

        std::fs::remove_dir_all(&root).ok();
    }
}
