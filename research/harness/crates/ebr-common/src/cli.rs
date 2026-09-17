//! Minimal `--key value` argument parsing shared by every harness binary.
//!
//! This exists so each binary's argv shape -- how an `ebr` runner spec's
//! command template fills it in -- is uniform, without pulling in a
//! third-party argument-parsing crate. See `research/harness/README.md` for
//! the shared flag names every binary in this workspace accepts
//! (`--item-path`, `--experiment-id`, `--env-name`, `--run-id`, `--out`, plus
//! whatever a specific binary adds).

use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug)]
pub struct ArgsError(pub String);

impl fmt::Display for ArgsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for ArgsError {}

/// A flat set of `--key value` pairs. Repeating a flag keeps only its last
/// value. A flag with no following value, or a bare positional argument, is
/// a parse error: every argument this workspace's binaries take is named.
#[derive(Debug, Default, Clone)]
pub struct Args {
    values: BTreeMap<String, String>,
}

impl Args {
    /// Parses `--key value --key2 value2 ...` from `argv`, typically
    /// `std::env::args().skip(1)`.
    pub fn parse<I: IntoIterator<Item = String>>(argv: I) -> Result<Self, ArgsError> {
        let mut values = BTreeMap::new();
        let mut iter = argv.into_iter();
        while let Some(arg) = iter.next() {
            let key = arg
                .strip_prefix("--")
                .ok_or_else(|| ArgsError(format!("expected a --flag, got {arg:?}")))?
                .to_string();
            let value = iter
                .next()
                .ok_or_else(|| ArgsError(format!("--{key} needs a value")))?;
            values.insert(key, value);
        }
        Ok(Args { values })
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(String::as_str)
    }

    pub fn require(&self, key: &str) -> Result<&str, ArgsError> {
        self.get(key)
            .ok_or_else(|| ArgsError(format!("missing required --{key}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args_from(pairs: &[&str]) -> Args {
        Args::parse(pairs.iter().map(|s| s.to_string())).unwrap()
    }

    #[test]
    fn parses_flag_value_pairs() {
        let args = args_from(&["--item-path", "/tmp/x", "--experiment-id", "EXP-1"]);
        assert_eq!(args.get("item-path"), Some("/tmp/x"));
        assert_eq!(args.require("experiment-id").unwrap(), "EXP-1");
        assert!(args.get("missing").is_none());
    }

    #[test]
    fn require_fails_loudly_on_a_missing_flag() {
        let args = args_from(&["--a", "1"]);
        assert!(args.require("b").is_err());
    }

    #[test]
    fn rejects_a_flag_with_no_value() {
        let err = Args::parse(vec!["--a".to_string()]);
        assert!(err.is_err());
    }

    #[test]
    fn rejects_a_bare_positional_argument() {
        let err = Args::parse(vec!["not-a-flag".to_string()]);
        assert!(err.is_err());
    }
}
