//! Environment and run identity.
//!
//! Reads `research/environment/index.json` (see
//! `research/environment/README.md`) to resolve a short environment name --
//! `"windows-host"`, `"wsl-ubuntu"`, or `"docker-ubuntu-24.04"` today -- to
//! its current `env_id`, and bundles the small set of ids
//! (`experiment_id`/`run_id`/`env_id`) every result row and self-measurement
//! carries alongside it.

use serde::Deserialize;
use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

/// `research/environment/index.json` in full.
#[derive(Debug, Clone, Deserialize)]
pub struct EnvIndex {
    pub schema: String,
    /// name -> current env_id.
    pub environments: BTreeMap<String, String>,
    /// name -> every env_id ever recorded for it, oldest first.
    #[serde(default)]
    pub history: BTreeMap<String, Vec<String>>,
}

#[derive(Debug)]
pub enum EnvIndexError {
    Io(std::io::Error),
    Json(serde_json::Error),
    UnknownName { name: String, known: Vec<String> },
}

impl fmt::Display for EnvIndexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EnvIndexError::Io(e) => write!(f, "reading research/environment/index.json: {e}"),
            EnvIndexError::Json(e) => write!(f, "parsing research/environment/index.json: {e}"),
            EnvIndexError::UnknownName { name, known } => {
                write!(
                    f,
                    "unknown environment name {name:?}; known names: {}",
                    known.join(", ")
                )
            }
        }
    }
}

impl std::error::Error for EnvIndexError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            EnvIndexError::Io(e) => Some(e),
            EnvIndexError::Json(e) => Some(e),
            EnvIndexError::UnknownName { .. } => None,
        }
    }
}

impl EnvIndex {
    pub fn load(repo_root: &Path) -> Result<Self, EnvIndexError> {
        let path = repo_root.join("research/environment/index.json");
        let text = std::fs::read_to_string(&path).map_err(EnvIndexError::Io)?;
        serde_json::from_str(&text).map_err(EnvIndexError::Json)
    }

    /// The current `env_id` for a short environment name.
    pub fn resolve(&self, name: &str) -> Result<&str, EnvIndexError> {
        self.environments
            .get(name)
            .map(String::as_str)
            .ok_or_else(|| EnvIndexError::UnknownName {
                name: name.to_string(),
                known: self.environments.keys().cloned().collect(),
            })
    }
}

/// The identity triple every JSONL result row and self-measurement carries.
#[derive(Debug, Clone)]
pub struct RunContext {
    pub experiment_id: String,
    pub run_id: String,
    pub env_id: String,
}

impl RunContext {
    /// Generates a fresh [`crate::timing::generate_run_id`] run id.
    pub fn new(experiment_id: impl Into<String>, env_id: impl Into<String>) -> Self {
        RunContext {
            experiment_id: experiment_id.into(),
            run_id: crate::timing::generate_run_id(),
            env_id: env_id.into(),
        }
    }

    /// Builds a context for resuming or re-labeling a specific run id.
    pub fn with_run_id(
        experiment_id: impl Into<String>,
        run_id: impl Into<String>,
        env_id: impl Into<String>,
    ) -> Self {
        RunContext {
            experiment_id: experiment_id.into(),
            run_id: run_id.into(),
            env_id: env_id.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_known_names_from_the_committed_index() {
        let index: EnvIndex = serde_json::from_str(
            r#"{
                "schema": "entrybound.research.env-index/1",
                "environments": {"wsl-ubuntu": "8576a4a4947e"},
                "history": {"wsl-ubuntu": ["8576a4a4947e"]}
            }"#,
        )
        .unwrap();
        assert_eq!(index.resolve("wsl-ubuntu").unwrap(), "8576a4a4947e");
        assert!(index.resolve("does-not-exist").is_err());
    }

    #[test]
    fn run_context_carries_the_identity_triple() {
        let ctx =
            RunContext::with_run_id("EXP-SMOKE-000", "20260101T000000Z-aaaaaa", "8576a4a4947e");
        assert_eq!(ctx.experiment_id, "EXP-SMOKE-000");
        assert_eq!(ctx.run_id, "20260101T000000Z-aaaaaa");
        assert_eq!(ctx.env_id, "8576a4a4947e");
    }
}
