//! Corpus manifest reader: `research/corpus/manifest.json`, when present.
//!
//! The manifest is produced by `research/corpus/tools/assemble.py`
//! (`manifest_format: "ebrc-manifest-v1"`, `corpus_version: "ebrc-2026.09-v1"`
//! as of Phase B1d, 196 items). This reader is deliberately tolerant of
//! fields it does not know about: it captures the fields experiment code
//! needs and keeps everything else per item and at the top level in `extra`,
//! so a manifest schema addition on the Python side never breaks this crate.
//! It never reads `items[].fingerprint_ref`/`items[].source_file` targets
//! itself, and it never resolves an item to its on-disk bytes -- see
//! [`crate::walk`] and [`crate::heldout`] for that, which is the boundary
//! that keeps this reader from ever needing to know about the held-out guard.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

/// One entry of `manifest.json`'s `items` array.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ManifestItem {
    pub item_id: String,
    pub family: String,
    #[serde(default)]
    pub family_name: Option<String>,
    pub split: String,
    #[serde(default)]
    pub scale: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub real_or_generated: Option<String>,
    #[serde(default)]
    pub independence_group: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub tree_sha256: Option<String>,
    #[serde(default)]
    pub fingerprint_ref: Option<String>,
    /// Every manifest field this struct does not name explicitly (license,
    /// provenance, output_pin, materialization_key, ...), untouched.
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

/// `research/corpus/manifest.json` in full.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Manifest {
    pub manifest_format: String,
    pub corpus_version: String,
    #[serde(default)]
    pub item_count: Option<u64>,
    pub items: Vec<ManifestItem>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug)]
pub enum ManifestError {
    Io(std::io::Error),
    Json(serde_json::Error),
}

impl fmt::Display for ManifestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ManifestError::Io(e) => write!(f, "reading corpus manifest: {e}"),
            ManifestError::Json(e) => write!(f, "parsing corpus manifest: {e}"),
        }
    }
}

impl std::error::Error for ManifestError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ManifestError::Io(e) => Some(e),
            ManifestError::Json(e) => Some(e),
        }
    }
}

impl Manifest {
    /// Loads and parses a manifest file at an exact path.
    pub fn load(path: &Path) -> Result<Self, ManifestError> {
        let text = std::fs::read_to_string(path).map_err(ManifestError::Io)?;
        serde_json::from_str(&text).map_err(ManifestError::Json)
    }

    /// Loads `research/corpus/manifest.json` under `repo_root`. Returns
    /// `Ok(None)` (not an error) when the file does not exist yet: corpus
    /// assembly (Phase B1) is a separate, earlier step this crate does not
    /// require to already be done.
    pub fn load_default(repo_root: &Path) -> Result<Option<Self>, ManifestError> {
        let path = repo_root.join("research/corpus/manifest.json");
        if !path.exists() {
            return Ok(None);
        }
        Self::load(&path).map(Some)
    }

    /// Items in one split (`"tuning"`, `"validation"`, `"heldout"`).
    ///
    /// Iterating `"heldout"` items here is fine: this only reads the small,
    /// git-tracked manifest (identities and metadata), never held-out
    /// *content*. Resolving an item to its on-disk bytes goes through
    /// [`crate::heldout::HeldoutGuard`], which is where the refusal lives.
    pub fn items_in_split<'a>(&'a self, split: &'a str) -> impl Iterator<Item = &'a ManifestItem> {
        self.items.iter().filter(move |item| item.split == split)
    }

    pub fn items_in_family<'a>(
        &'a self,
        family: &'a str,
    ) -> impl Iterator<Item = &'a ManifestItem> {
        self.items.iter().filter(move |item| item.family == family)
    }

    pub fn find(&self, item_id: &str) -> Option<&ManifestItem> {
        self.items.iter().find(|item| item.item_id == item_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> &'static str {
        r#"{
            "manifest_format": "ebrc-manifest-v1",
            "corpus_version": "ebrc-2026.09-v1",
            "item_count": 2,
            "items": [
                {
                    "item_id": "f01-tuning-example",
                    "family": "F01",
                    "family_name": "source-code repositories",
                    "split": "tuning",
                    "scale": "small",
                    "kind": "git-archive",
                    "real_or_generated": "real",
                    "status": "ok",
                    "tree_sha256": "deadbeef",
                    "license": {"spdx_or_name": "MIT"}
                },
                {
                    "item_id": "f05-heldout-example",
                    "family": "F05",
                    "split": "heldout",
                    "status": "ok"
                }
            ],
            "manifest_sha256": "unused-here"
        }"#
    }

    #[test]
    fn parses_known_fields_and_keeps_extra() {
        let manifest: Manifest = serde_json::from_str(sample()).unwrap();
        assert_eq!(manifest.manifest_format, "ebrc-manifest-v1");
        assert_eq!(manifest.items.len(), 2);
        assert!(manifest.extra.contains_key("manifest_sha256"));

        let item = manifest.find("f01-tuning-example").unwrap();
        assert_eq!(item.family, "F01");
        assert!(item.extra.contains_key("license"));
    }

    #[test]
    fn filters_by_split_without_touching_heldout_content() {
        let manifest: Manifest = serde_json::from_str(sample()).unwrap();
        let tuning: Vec<_> = manifest.items_in_split("tuning").collect();
        assert_eq!(tuning.len(), 1);
        let heldout: Vec<_> = manifest.items_in_split("heldout").collect();
        assert_eq!(heldout.len(), 1);
        assert_eq!(heldout[0].item_id, "f05-heldout-example");
    }

    #[test]
    fn load_default_returns_none_when_missing() {
        let dir =
            std::env::temp_dir().join(format!("ebr-common-manifest-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let result = Manifest::load_default(&dir).unwrap();
        assert!(result.is_none());
        std::fs::remove_dir_all(&dir).ok();
    }
}
