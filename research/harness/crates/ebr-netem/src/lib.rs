//! Application-level network emulation for remote-access research.
//!
//! `research/PROGRESS.md`'s execution-environment notes are the reason this
//! crate exists at all: `sch_netem` (Linux kernel-level `tc qdisc ... netem`)
//! is unavailable in both WSL2 and Docker on this host, so latency,
//! bandwidth-cap, jitter, and loss emulation for
//! `entrybound::random_access::HttpRangeSource` has to happen in an
//! application-level HTTP proxy instead of the kernel. **This is a
//! documented threat to validity, not something this crate quietly
//! resolves** -- see `README.md`'s "Threats to validity" section, and carry
//! the same caveat into any experiment writeup that uses this crate.
//!
//! Two binaries built from this library:
//!
//! - [`origin`]: `ebr-origin`, a static HTTP range-serving file server for
//!   `.eb` archives, with the exact single-range contract
//!   `entrybound::random_access::HttpRangeSource` requires (strong quoted
//!   ETag, exact `206`/`Content-Range`, `412` on `If-Match` mismatch), plus
//!   configurable "object-store-like" misbehavior for testing client
//!   robustness (weak/missing ETag, a 200-full-body fallback, multi-range
//!   limits, mid-session revision churn).
//! - [`proxy`]: `ebr-netem-proxy`, a reverse proxy that sits between a
//!   client and an origin (this one or a real one) and applies a configured
//!   RTT, bandwidth token-bucket, jitter, connection-setup delay, a
//!   loss-as-stall model, a concurrent-connection cap, and an optional CDN
//!   cache layer.
//!
//! Shared, independently-testable building blocks live in their own modules:
//! [`range`] (Range-header parsing and `multipart/byteranges` framing),
//! [`etag`] (strong/weak ETag formatting and `If-Match` evaluation),
//! [`bandwidth`] (the token bucket), [`loss`] (the loss-as-stall model),
//! [`cache`] (the CDN cache), [`rng`] (the seeded PRNG jitter/loss draw
//! from), and [`config`] (CLI flag parsing for both binaries, built on
//! `ebr_common::cli::Args`).
//!
//! `ebr-netem-calibrate` (`src/bin/ebr-netem-calibrate.rs`) drives both
//! `origin` and `proxy` in-process over a small RTT/bandwidth sweep and
//! writes `calibration.md` from the raw JSONL it records -- see that
//! binary's own doc comment and `README.md`'s "Calibration" section.

pub mod bandwidth;
pub mod cache;
pub mod config;
pub mod etag;
pub mod loss;
pub mod origin;
pub mod proxy;
pub mod range;
pub mod rng;

/// The response body type every handler in this crate returns: either a
/// [`http_body_util::Full`] (origin responses, which are never paced) or a
/// paced [`http_body_util::StreamBody`] (proxy responses), boxed to a single
/// concrete type. Its error type is [`std::convert::Infallible`] because
/// nothing downstream of buffering the bytes in memory can fail; see
/// `proxy.rs`'s doc comment for where a real upstream error is turned into a
/// `502` response instead of a body-level error.
pub type RespBody =
    http_body_util::combinators::BoxBody<hyper::body::Bytes, std::convert::Infallible>;

/// Wraps already-in-memory bytes as a non-paced, non-streamed response body.
pub fn full_body(bytes: hyper::body::Bytes) -> RespBody {
    use http_body_util::BodyExt as _;
    http_body_util::Full::new(bytes).boxed()
}
