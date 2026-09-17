//! CLI configuration for both binaries, built on [`ebr_common::cli::Args`]
//! (this workspace's shared flat `--key value` parser -- see that module's
//! doc comment) rather than a new config-file format, matching every other
//! crate in this workspace.
//!
//! `research/harness/README.md`'s shared-flag convention
//! (`--item-path`/`--experiment-id`/...) is about experiment-runner
//! bookkeeping; these two binaries are long-running servers configured by
//! their own flags instead, documented on each flag below and in
//! `README.md`.

use crate::etag::EtagMode;
use std::fmt;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

#[derive(Debug)]
pub struct ConfigError(pub String);

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for ConfigError {}

impl From<ebr_common::cli::ArgsError> for ConfigError {
    fn from(err: ebr_common::cli::ArgsError) -> Self {
        ConfigError(err.0)
    }
}

fn parse_flag<T>(args: &ebr_common::cli::Args, key: &str, default: T) -> Result<T, ConfigError>
where
    T: FromStr,
    T::Err: fmt::Display,
{
    match args.get(key) {
        None => Ok(default),
        Some(value) => value
            .parse()
            .map_err(|error| ConfigError(format!("--{key}={value:?}: {error}"))),
    }
}

fn parse_bool_flag(
    args: &ebr_common::cli::Args,
    key: &str,
    default: bool,
) -> Result<bool, ConfigError> {
    match args.get(key) {
        None => Ok(default),
        Some(value) => match value.to_ascii_lowercase().as_str() {
            "true" | "1" | "yes" => Ok(true),
            "false" | "0" | "no" => Ok(false),
            other => Err(ConfigError(format!(
                "--{key}: expected true/false, got {other:?}"
            ))),
        },
    }
}

fn parse_millis_flag(
    args: &ebr_common::cli::Args,
    key: &str,
    default_ms: u64,
) -> Result<Duration, ConfigError> {
    Ok(Duration::from_millis(parse_flag(args, key, default_ms)?))
}

/// `--etag strong|weak|missing`, `--full-body-fallback`,
/// `--revision-churn-after`, and `--max-ranges`/`--multi-range` are the
/// "configurable object-store-like behaviors" the task asks for; see
/// `origin.rs` for exactly how each one changes response bytes, and
/// `etag.rs`'s doc comment for why `weak`/`missing` and `full-body-fallback`
/// exist (to deliberately violate the contract a client relies on).
#[derive(Debug, Clone)]
pub struct OriginConfig {
    /// Directory of `.eb` archives (and anything else) served read-only.
    pub root: PathBuf,
    pub listen: SocketAddr,
    pub etag_mode: EtagMode,
    pub allow_multi_range: bool,
    /// A request with more ranges than this gets `416` (if
    /// `allow_multi_range`) -- see `origin.rs`'s doc comment for the full
    /// decision table.
    pub max_ranges_per_request: usize,
    pub full_body_fallback: bool,
    /// After this many `GET`/`HEAD` requests have been served (across the
    /// whole server, not per file), every file's bytes and ETag flip to a
    /// second, deterministic "revision" -- see `origin.rs::churn`.
    pub revision_churn_after_requests: Option<u64>,
    pub enable_h2: bool,
    /// JSONL request log path (`ebr.harness.raw.v1` via
    /// `ebr_common::results`); `None` disables logging.
    pub log_path: Option<PathBuf>,
    pub env_id: String,
}

impl OriginConfig {
    pub fn from_args(args: &ebr_common::cli::Args) -> Result<Self, ConfigError> {
        let root = PathBuf::from(args.require("root")?);
        let listen: SocketAddr = parse_flag(args, "listen", "127.0.0.1:0".parse().unwrap())?;
        let etag_mode = match args.get("etag") {
            None => EtagMode::Strong,
            Some(value) => EtagMode::parse(value)
                .ok_or_else(|| ConfigError(format!("--etag: unknown mode {value:?}")))?,
        };
        Ok(OriginConfig {
            root,
            listen,
            etag_mode,
            allow_multi_range: parse_bool_flag(args, "multi-range", true)?,
            max_ranges_per_request: parse_flag(args, "max-ranges", 16usize)?,
            full_body_fallback: parse_bool_flag(args, "full-body-fallback", false)?,
            revision_churn_after_requests: match args.get("revision-churn-after") {
                None => None,
                Some(value) => Some(
                    value
                        .parse::<u64>()
                        .map_err(|error| ConfigError(format!("--revision-churn-after: {error}")))?,
                ),
            },
            enable_h2: parse_bool_flag(args, "h2", true)?,
            log_path: args.get("log").map(PathBuf::from),
            env_id: args.get("env-id").unwrap_or("adhoc").to_string(),
        })
    }
}

/// A jitter distribution sampled once per proxied request, on top of the
/// fixed configured RTT (see `proxy.rs`). Sampling clamps at zero: a jitter
/// draw can never make the emulated network *faster* than a request with no
/// jitter at all, only slower, matching how jitter is commonly modeled
/// (extra queuing delay, never negative delay).
#[derive(Debug, Clone, Copy)]
pub enum JitterModel {
    None,
    Uniform { max: Duration },
    Normal { mean: Duration, std_dev: Duration },
}

impl JitterModel {
    pub fn sample(&self, rng: &crate::rng::SeededRng) -> Duration {
        match *self {
            JitterModel::None => Duration::ZERO,
            JitterModel::Uniform { max } => {
                Duration::from_secs_f64(rng.unit_f64() * max.as_secs_f64())
            }
            JitterModel::Normal { mean, std_dev } => {
                let seconds = rng
                    .normal_f64(mean.as_secs_f64(), std_dev.as_secs_f64())
                    .max(0.0);
                Duration::from_secs_f64(seconds)
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheMode {
    Off,
    Cold,
    Warm,
}

impl CacheMode {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "off" => Some(CacheMode::Off),
            "cold" => Some(CacheMode::Cold),
            "warm" => Some(CacheMode::Warm),
            _ => None,
        }
    }
}

/// One line of `--cache-warm-list`: `<path>` or `<path> <range-header>`,
/// e.g. `/archive.eb` or `/archive.eb bytes=0-1023`. Blank lines and lines
/// starting with `#` are ignored. A plain text format (not JSON) so warming
/// a cache before an experiment needs no schema beyond "one request per
/// line" -- consistent with this crate's CLI-flags-not-config-files
/// convention.
pub fn parse_warm_list(text: &str) -> Vec<(String, Option<String>)> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| match line.split_once(' ') {
            Some((path, range)) => (path.to_string(), Some(range.trim().to_string())),
            None => (line.to_string(), None),
        })
        .collect()
}

/// Every `ebr-netem-proxy` knob from the task list: per-connection RTT,
/// bandwidth token buckets in each direction, jitter, connection-setup
/// delay, the loss-as-stall model, a concurrent-connection cap, and the CDN
/// cache layer. See `proxy.rs` for how these compose into one request's
/// handling.
#[derive(Debug, Clone)]
pub struct ProxyConfig {
    pub listen: SocketAddr,
    /// Origin authority (`host:port`), always dialed as plain `http://` --
    /// see `README.md`'s "Threats to validity" for why no TLS layer exists
    /// here (the "TCP+TLS handshake" cost is a simulated delay only, see
    /// `setup_rtt_multiple`).
    pub upstream_authority: String,
    /// Configured round-trip time. Split as one added one-way delay before
    /// the request reaches the origin and one more before the response's
    /// first byte reaches the client, per the task's "added one-way delay
    /// on request and response first byte".
    pub rtt: Duration,
    pub jitter: JitterModel,
    /// `None` = unmetered (an unlimited [`crate::bandwidth::TokenBucket`]).
    pub bandwidth_up_bps: Option<f64>,
    pub bandwidth_down_bps: Option<f64>,
    pub burst_up_bytes: u64,
    pub burst_down_bytes: u64,
    /// Multiple of `rtt` charged once per accepted client TCP connection,
    /// before its first request is served -- approximates "TCP+TLS
    /// handshake RTT multiples" (recommend `1.0` for a bare TCP handshake,
    /// `2.0`-`3.0` to also approximate a TLS handshake on top).
    pub setup_rtt_multiple: f64,
    pub loss_p: f64,
    pub loss_rto: Duration,
    pub seed: u64,
    pub max_connections: usize,
    pub cache_mode: CacheMode,
    pub cache_capacity_bytes: u64,
    /// Only consulted when `cache_mode == Warm`; see [`parse_warm_list`].
    pub cache_warm_list: Option<PathBuf>,
    /// Frame size the paced downstream/upstream body streams chunk at
    /// (`proxy.rs::shaped_body_stream`). Smaller chunks pace more smoothly
    /// but call into the token bucket and loss model more often.
    pub chunk_size: usize,
    pub log_path: Option<PathBuf>,
    pub env_id: String,
}

/// `mbit_per_sec` uses the networking convention of decimal
/// megabits-per-second (1 Mbit/s = 1,000,000 bits/s = 125,000 bytes/s), not
/// mebibytes.
pub fn mbit_to_bytes_per_sec(mbit_per_sec: f64) -> f64 {
    mbit_per_sec * 1_000_000.0 / 8.0
}

impl ProxyConfig {
    pub fn from_args(args: &ebr_common::cli::Args) -> Result<Self, ConfigError> {
        let listen: SocketAddr = parse_flag(args, "listen", "127.0.0.1:0".parse().unwrap())?;
        let upstream_raw = args.require("upstream")?;
        let upstream_authority = upstream_raw
            .strip_prefix("http://")
            .unwrap_or(upstream_raw)
            .trim_end_matches('/')
            .to_string();
        let rtt = parse_millis_flag(args, "rtt-ms", 0)?;
        let jitter = match args.get("jitter").unwrap_or("none") {
            "none" => JitterModel::None,
            "uniform" => JitterModel::Uniform {
                max: parse_millis_flag(args, "jitter-max-ms", 0)?,
            },
            "normal" => JitterModel::Normal {
                mean: parse_millis_flag(args, "jitter-mean-ms", 0)?,
                std_dev: parse_millis_flag(args, "jitter-stddev-ms", 0)?,
            },
            other => return Err(ConfigError(format!("--jitter: unknown model {other:?}"))),
        };
        let bandwidth_up_bps = match args.get("bandwidth-up-mbit") {
            None => None,
            Some(value) => {
                Some(mbit_to_bytes_per_sec(value.parse::<f64>().map_err(
                    |error| ConfigError(format!("--bandwidth-up-mbit: {error}")),
                )?))
            }
        };
        let bandwidth_down_bps = match args.get("bandwidth-down-mbit") {
            None => None,
            Some(value) => {
                Some(mbit_to_bytes_per_sec(value.parse::<f64>().map_err(
                    |error| ConfigError(format!("--bandwidth-down-mbit: {error}")),
                )?))
            }
        };
        let cache_mode = match args.get("cache-mode") {
            None => CacheMode::Off,
            Some(value) => CacheMode::parse(value)
                .ok_or_else(|| ConfigError(format!("--cache-mode: unknown mode {value:?}")))?,
        };
        Ok(ProxyConfig {
            listen,
            upstream_authority,
            rtt,
            jitter,
            bandwidth_up_bps,
            bandwidth_down_bps,
            burst_up_bytes: parse_flag(args, "burst-up-bytes", 65_536u64)?,
            burst_down_bytes: parse_flag(args, "burst-down-bytes", 65_536u64)?,
            setup_rtt_multiple: parse_flag(args, "setup-rtt-multiple", 1.0f64)?,
            loss_p: parse_flag(args, "loss-p", 0.0f64)?,
            loss_rto: parse_millis_flag(args, "loss-rto-ms", 200)?,
            seed: parse_flag(args, "seed", 42u64)?,
            max_connections: parse_flag(args, "max-connections", 64usize)?,
            cache_mode,
            cache_capacity_bytes: parse_flag(args, "cache-capacity-bytes", 16 * 1024 * 1024u64)?,
            cache_warm_list: args.get("cache-warm-list").map(PathBuf::from),
            chunk_size: parse_flag(args, "chunk-size", 16 * 1024usize)?,
            log_path: args.get("log").map(PathBuf::from),
            env_id: args.get("env-id").unwrap_or("adhoc").to_string(),
        })
    }

    pub fn setup_delay(&self) -> Duration {
        Duration::from_secs_f64(self.rtt.as_secs_f64() * self.setup_rtt_multiple)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(pairs: &[&str]) -> ebr_common::cli::Args {
        ebr_common::cli::Args::parse(pairs.iter().map(|s| s.to_string())).unwrap()
    }

    #[test]
    fn origin_config_applies_documented_defaults() {
        let config = OriginConfig::from_args(&args(&["--root", "/tmp/archives"])).unwrap();
        assert_eq!(config.root, PathBuf::from("/tmp/archives"));
        assert_eq!(config.etag_mode, EtagMode::Strong);
        assert!(config.allow_multi_range);
        assert_eq!(config.max_ranges_per_request, 16);
        assert!(!config.full_body_fallback);
        assert_eq!(config.revision_churn_after_requests, None);
        assert!(config.enable_h2);
        assert_eq!(config.env_id, "adhoc");
    }

    #[test]
    fn origin_config_missing_required_root_errors() {
        assert!(OriginConfig::from_args(&args(&[])).is_err());
    }

    #[test]
    fn origin_config_rejects_an_unknown_etag_mode() {
        let err = OriginConfig::from_args(&args(&["--root", "/x", "--etag", "bogus"])).unwrap_err();
        assert!(err.0.contains("etag"));
    }

    #[test]
    fn proxy_config_parses_all_directions_and_models() {
        let config = ProxyConfig::from_args(&args(&[
            "--upstream",
            "http://127.0.0.1:9000",
            "--rtt-ms",
            "100",
            "--jitter",
            "uniform",
            "--jitter-max-ms",
            "20",
            "--bandwidth-up-mbit",
            "10",
            "--bandwidth-down-mbit",
            "100",
            "--loss-p",
            "0.05",
            "--cache-mode",
            "warm",
        ]))
        .unwrap();
        assert_eq!(config.upstream_authority, "127.0.0.1:9000");
        assert_eq!(config.rtt, Duration::from_millis(100));
        assert!(
            matches!(config.jitter, JitterModel::Uniform { max } if max == Duration::from_millis(20))
        );
        assert_eq!(config.bandwidth_up_bps, Some(1_250_000.0));
        assert_eq!(config.bandwidth_down_bps, Some(12_500_000.0));
        assert_eq!(config.loss_p, 0.05);
        assert_eq!(config.cache_mode, CacheMode::Warm);
    }

    #[test]
    fn proxy_config_strips_an_http_scheme_and_trailing_slash_from_upstream() {
        let config =
            ProxyConfig::from_args(&args(&["--upstream", "http://127.0.0.1:9000/"])).unwrap();
        assert_eq!(config.upstream_authority, "127.0.0.1:9000");
    }

    #[test]
    fn setup_delay_scales_rtt_by_the_configured_multiple() {
        let config = ProxyConfig::from_args(&args(&[
            "--upstream",
            "x:1",
            "--rtt-ms",
            "50",
            "--setup-rtt-multiple",
            "2.5",
        ]))
        .unwrap();
        assert_eq!(config.setup_delay(), Duration::from_millis(125));
    }

    #[test]
    fn mbit_conversion_uses_decimal_megabits() {
        assert_eq!(mbit_to_bytes_per_sec(1.0), 125_000.0);
        assert_eq!(mbit_to_bytes_per_sec(1000.0), 125_000_000.0);
    }

    #[test]
    fn warm_list_skips_blank_and_comment_lines_and_splits_the_range() {
        let parsed = parse_warm_list("\n# comment\n/a.eb\n/b.eb bytes=0-99\n  \n");
        assert_eq!(
            parsed,
            vec![
                ("/a.eb".to_string(), None),
                ("/b.eb".to_string(), Some("bytes=0-99".to_string())),
            ]
        );
    }
}
