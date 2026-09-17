//! `ebr-origin`: a static HTTP range-serving file server for `.eb` archives.
//!
//! Implements the exact contract
//! `entrybound::random_access::HttpRangeSource` requires (see
//! `crates/entrybound/src/random_access.rs`, cross-checked in
//! `tests/origin_contract.rs` against the real client): a `HEAD` returns a
//! strong quoted `ETag`, `Accept-Ranges: bytes`, an exact `Content-Length`,
//! and never `Content-Encoding`; a ranged `GET` sent with `If-Match`
//! returns an exact `206`/`Content-Range` when the tag still matches, `412`
//! when it does not.
//!
//! It also implements the task's "configurable object-store-like
//! behaviors", each driven by one [`crate::config::OriginConfig`] field and
//! documented on the field itself:
//!
//! - `--max-ranges`/`--multi-range`: multi-range decision table below.
//! - `--full-body-fallback`: ignore `Range` entirely and always answer
//!   `200` with the whole body -- for proving a client *refuses* a
//!   range-blind origin instead of silently misreading it.
//! - `--etag weak|missing`: violate the strong-ETag requirement, same
//!   purpose (see `etag.rs`).
//! - `--revision-churn-after`: after that many requests, every file's bytes
//!   and ETag deterministically flip (`FileEntry::churned_variant`), so an
//!   in-flight random-access session can be made to observe a stale
//!   `If-Match` and abort, per `HttpRangeSource`'s own "ETag changed during
//!   random access" behavior.
//!
//! Multi-range decision table for a `GET` with `N` comma-separated ranges,
//! none of `--full-body-fallback` engaged:
//!
//! | `N` | `--multi-range` | `N` vs `--max-ranges` | Response |
//! |---|---|---|---|
//! | 0 (no `Range` header, or unparseable) | -- | -- | `200`, whole body |
//! | 1 | -- | -- | `206`, single part |
//! | ≥2, all unsatisfiable | -- | -- | `416` |
//! | ≥2 | `false` | -- | `206`, single part (first satisfiable range only) |
//! | ≥2 | `true` | `N` > max | `416` |
//! | ≥2 | `true` | `N` <= max | `206`, `multipart/byteranges` |

use crate::config::OriginConfig;
use crate::etag;
use crate::range;
use crate::{RespBody, full_body};
use ebr_common::results::ResultWriter;
use ebr_common::runenv::RunContext;
use hyper::body::{Bytes, Incoming};
use hyper::header::{self, HeaderValue};
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto::Builder as AutoBuilder;
use serde_json::json;
use std::collections::HashMap;
use std::convert::Infallible;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use tokio::net::TcpListener;

struct FileEntry {
    bytes: Bytes,
    /// Lower-case hex SHA-256 of `bytes`.
    content_hash_hex: String,
    /// Computed once, lazily, the first time a request lands past the
    /// revision-churn threshold: a deterministic "next revision" (last byte
    /// flipped) with its own hash. Kept per-file so churn is `O(1)` after
    /// the first post-threshold request instead of re-hashing the whole
    /// file on every one.
    churned: OnceLock<(Bytes, String)>,
}

impl FileEntry {
    fn load(path: &Path) -> std::io::Result<Self> {
        let bytes = std::fs::read(path)?;
        let content_hash_hex = ebr_common::hash::sha256_hex(&bytes);
        Ok(FileEntry {
            bytes: Bytes::from(bytes),
            content_hash_hex,
            churned: OnceLock::new(),
        })
    }

    fn churned_variant(&self) -> &(Bytes, String) {
        self.churned.get_or_init(|| {
            if self.bytes.is_empty() {
                return (self.bytes.clone(), self.content_hash_hex.clone());
            }
            let mut mutated = self.bytes.to_vec();
            let last = mutated.len() - 1;
            mutated[last] ^= 0xFF;
            let hash = ebr_common::hash::sha256_hex(&mutated);
            (Bytes::from(mutated), hash)
        })
    }

    /// The bytes and content hash this request should see.
    fn effective(&self, churned: bool) -> (Bytes, &str) {
        if churned {
            let (bytes, hash) = self.churned_variant();
            (bytes.clone(), hash.as_str())
        } else {
            (self.bytes.clone(), self.content_hash_hex.as_str())
        }
    }
}

pub struct OriginState {
    config: OriginConfig,
    files: Mutex<HashMap<PathBuf, Arc<FileEntry>>>,
    /// Total `GET`/`HEAD` requests served, across all files -- what
    /// `--revision-churn-after` counts against (a whole-server trigger, not
    /// a per-file one; see the module doc comment).
    served: AtomicU64,
    logger: Option<Mutex<ResultWriter<std::io::BufWriter<std::fs::File>>>>,
}

impl OriginState {
    pub fn new(config: OriginConfig) -> std::io::Result<Self> {
        let logger = match &config.log_path {
            None => None,
            Some(path) => {
                let context = RunContext::new("EBR-NETEM-ORIGIN", config.env_id.clone());
                Some(Mutex::new(ResultWriter::create(path, context)?))
            }
        };
        Ok(OriginState {
            config,
            files: Mutex::new(HashMap::new()),
            served: AtomicU64::new(0),
            logger,
        })
    }

    fn entry_for(&self, path: &Path) -> std::io::Result<Arc<FileEntry>> {
        if let Some(existing) = self
            .files
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .get(path)
        {
            return Ok(existing.clone());
        }
        let loaded = Arc::new(FileEntry::load(path)?);
        self.files
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .insert(path.to_path_buf(), loaded.clone());
        Ok(loaded)
    }

    fn log(
        &self,
        path: &str,
        method: &str,
        range: Option<&str>,
        status: u16,
        bytes_sent: u64,
        churned: bool,
    ) {
        let Some(logger) = &self.logger else { return };
        let payload = json!({
            "method": method,
            "path": path,
            "range": range,
            "status": status,
            "bytes_sent": bytes_sent,
            "churned": churned,
        });
        // Best-effort: a request log write failure must not take the
        // server down or fail the response already sent.
        let _ = logger
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .append(Some(path), payload);
    }
}

/// Resolves a request path to a file under `root`, refusing anything that
/// escapes it (no `..` components) or names the root itself (no directory
/// listing).
fn resolve_path(root: &Path, url_path: &str) -> Option<PathBuf> {
    let relative = url_path.trim_start_matches('/');
    if relative.is_empty() {
        return None;
    }
    if Path::new(relative).components().any(|component| {
        matches!(
            component,
            std::path::Component::ParentDir | std::path::Component::Prefix(_)
        )
    }) {
        return None;
    }
    Some(root.join(relative))
}

fn empty(status: StatusCode) -> Response<RespBody> {
    Response::builder()
        .status(status)
        .body(full_body(Bytes::new()))
        .unwrap()
}

async fn handle(
    state: Arc<OriginState>,
    req: Request<Incoming>,
) -> Result<Response<RespBody>, Infallible> {
    let method = req.method().clone();
    let path_str = req.uri().path().to_string();
    let range_header = req
        .headers()
        .get(header::RANGE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    let if_match_header = req
        .headers()
        .get(header::IF_MATCH)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);

    if !matches!(method, Method::GET | Method::HEAD) {
        let mut response = empty(StatusCode::METHOD_NOT_ALLOWED);
        response
            .headers_mut()
            .insert(header::ALLOW, HeaderValue::from_static("GET, HEAD"));
        state.log(
            &path_str,
            method.as_str(),
            range_header.as_deref(),
            response.status().as_u16(),
            0,
            false,
        );
        return Ok(response);
    }

    let Some(fs_path) = resolve_path(&state.config.root, &path_str) else {
        let response = empty(StatusCode::BAD_REQUEST);
        state.log(
            &path_str,
            method.as_str(),
            range_header.as_deref(),
            response.status().as_u16(),
            0,
            false,
        );
        return Ok(response);
    };

    let entry = match state.entry_for(&fs_path) {
        Ok(entry) => entry,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let response = empty(StatusCode::NOT_FOUND);
            state.log(
                &path_str,
                method.as_str(),
                range_header.as_deref(),
                response.status().as_u16(),
                0,
                false,
            );
            return Ok(response);
        }
        Err(_) => {
            let response = empty(StatusCode::INTERNAL_SERVER_ERROR);
            state.log(
                &path_str,
                method.as_str(),
                range_header.as_deref(),
                response.status().as_u16(),
                0,
                false,
            );
            return Ok(response);
        }
    };

    let served_so_far = state.served.fetch_add(1, Ordering::Relaxed) + 1;
    let churned = state
        .config
        .revision_churn_after_requests
        .is_some_and(|threshold| served_so_far > threshold);
    let (file_bytes, content_hash_hex) = entry.effective(churned);
    let total_len = file_bytes.len() as u64;
    let strong_etag_quoted = format!("\"{content_hash_hex}\"");
    let response_etag = etag::format_etag(content_hash_hex, state.config.etag_mode);

    if !etag::if_match_satisfied(if_match_header.as_deref(), Some(&strong_etag_quoted)) {
        let mut response = empty(StatusCode::PRECONDITION_FAILED);
        if let Some(tag) = &response_etag
            && let Ok(value) = HeaderValue::from_str(tag)
        {
            response.headers_mut().insert(header::ETAG, value);
        }
        state.log(
            &path_str,
            method.as_str(),
            range_header.as_deref(),
            response.status().as_u16(),
            0,
            churned,
        );
        return Ok(response);
    }

    let mut builder = Response::builder();
    if let Some(tag) = &response_etag {
        builder = builder.header(header::ETAG, tag.as_str());
    }
    builder = builder.header(header::ACCEPT_RANGES, "bytes");

    let (status, content_range, body_bytes): (StatusCode, Option<String>, Bytes) =
        if method == Method::HEAD {
            (StatusCode::OK, None, Bytes::new())
        } else if state.config.full_body_fallback || range_header.is_none() {
            (StatusCode::OK, None, file_bytes.clone())
        } else {
            let raw_range = range_header.as_deref().unwrap();
            match range::parse_range_header(raw_range, total_len) {
                Err(_) => (StatusCode::OK, None, file_bytes.clone()), // malformed Range: ignore it, serve the whole body
                Ok(ranges) if ranges.is_empty() => {
                    builder = builder.header(header::CONTENT_RANGE, format!("bytes */{total_len}"));
                    (StatusCode::RANGE_NOT_SATISFIABLE, None, Bytes::new())
                }
                Ok(ranges) if ranges.len() == 1 || !state.config.allow_multi_range => {
                    let selected = ranges[0]; // "ignore extras, serve first" when multi-range is disabled
                    let slice = file_bytes.slice(selected.start as usize..=selected.end as usize);
                    (
                        StatusCode::PARTIAL_CONTENT,
                        Some(selected.content_range_value(total_len)),
                        slice,
                    )
                }
                Ok(ranges) if ranges.len() > state.config.max_ranges_per_request => {
                    builder = builder.header(header::CONTENT_RANGE, format!("bytes */{total_len}"));
                    (StatusCode::RANGE_NOT_SATISFIABLE, None, Bytes::new())
                }
                Ok(ranges) => {
                    let boundary = range::multipart_boundary(&path_str, &ranges);
                    let multipart =
                        range::build_multipart_body(&boundary, total_len, &file_bytes, &ranges);
                    builder = builder.header(
                        header::CONTENT_TYPE,
                        format!("multipart/byteranges; boundary={boundary}"),
                    );
                    (StatusCode::PARTIAL_CONTENT, None, Bytes::from(multipart))
                }
            }
        };

    if let Some(value) = &content_range {
        builder = builder.header(header::CONTENT_RANGE, value.as_str());
    }
    let content_length = if method == Method::HEAD {
        total_len
    } else {
        body_bytes.len() as u64
    };
    builder = builder.header(header::CONTENT_LENGTH, content_length.to_string());
    let response = builder
        .status(status)
        .body(full_body(body_bytes.clone()))
        .unwrap();

    let bytes_sent = if method == Method::HEAD {
        0
    } else {
        body_bytes.len() as u64
    };
    state.log(
        &path_str,
        method.as_str(),
        range_header.as_deref(),
        status.as_u16(),
        bytes_sent,
        churned,
    );
    Ok(response)
}

/// Serves `config` on `listener` until the process is killed. Each accepted
/// connection is handled by an independent Tokio task; `config.enable_h2`
/// picks whether the shared `hyper_util` auto builder accepts HTTP/2 at all
/// (over plain TCP, via prior-knowledge -- see `README.md`'s "HTTP/2"
/// section for why this works without TLS/ALPN).
pub async fn run(config: OriginConfig, listener: TcpListener) -> std::io::Result<()> {
    let enable_h2 = config.enable_h2;
    let state = Arc::new(OriginState::new(config)?);
    let mut builder = AutoBuilder::new(TokioExecutor::new());
    if !enable_h2 {
        builder = builder.http1_only();
    }
    let builder = Arc::new(builder);
    loop {
        let (stream, _peer) = listener.accept().await?;
        let io = TokioIo::new(stream);
        let state = state.clone();
        let builder = builder.clone();
        tokio::spawn(async move {
            let service = service_fn(move |req| {
                let state = state.clone();
                async move { handle(state, req).await }
            });
            if let Err(error) = builder.serve_connection(io, service).await {
                // Connection-level errors (peer reset, malformed request
                // line, ...) are expected background noise for a research
                // harness server; nothing here should take the process down.
                eprintln!("ebr-origin: connection error: {error}");
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_path_rejects_parent_dir_traversal() {
        let root = Path::new("/srv/archives");
        assert!(resolve_path(root, "/../../etc/passwd").is_none());
        assert!(resolve_path(root, "/a/../../b").is_none());
    }

    #[test]
    fn resolve_path_rejects_the_bare_root() {
        assert!(resolve_path(Path::new("/srv/archives"), "/").is_none());
    }

    #[test]
    fn resolve_path_joins_a_plain_relative_path() {
        let resolved = resolve_path(Path::new("/srv/archives"), "/x/y.eb").unwrap();
        assert_eq!(resolved, PathBuf::from("/srv/archives/x/y.eb"));
    }

    #[test]
    fn churned_variant_flips_the_last_byte_and_the_hash() {
        let entry = FileEntry {
            bytes: Bytes::from_static(b"hello"),
            content_hash_hex: ebr_common::hash::sha256_hex(b"hello"),
            churned: OnceLock::new(),
        };
        let (original_bytes, original_hash) = entry.effective(false);
        let (churned_bytes, churned_hash) = entry.effective(true);
        assert_eq!(original_bytes.as_ref(), b"hello");
        assert_ne!(churned_bytes.as_ref(), b"hello");
        assert_eq!(churned_bytes.len(), original_bytes.len());
        assert_ne!(original_hash, churned_hash);
        // Idempotent: computed once, same answer every time.
        let (again_bytes, again_hash) = entry.effective(true);
        assert_eq!(churned_bytes, again_bytes);
        assert_eq!(churned_hash, again_hash);
    }
}
