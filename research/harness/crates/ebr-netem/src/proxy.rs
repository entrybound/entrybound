//! `ebr-netem-proxy`: an application-level network-emulation reverse proxy.
//!
//! Sits between a client and an origin (typically `ebr-origin`, but any
//! HTTP/1.1 range origin works) and, for every proxied request:
//!
//! 1. Sleeps `rtt/2 + jitter/2` (the "request travels to the origin" leg).
//! 2. Serves from the [`crate::cache::CdnCache`] on a hit (skipping the
//!    origin entirely -- a CDN edge cache hit never touches the origin);
//!    otherwise fetches from the origin over an unshaped local connection
//!    (see the module-level "Where emulation is applied" note below) and,
//!    if the response is cacheable, inserts it.
//! 3. Sleeps `rtt/2 + jitter/2` again (the "response travels back" leg).
//! 4. Streams the response body to the client through a
//!    [`crate::bandwidth::TokenBucket`] with [`crate::loss::LossModel`]
//!    stalls applied per chunk.
//!
//! Each accepted client TCP connection additionally pays a one-time
//! `rtt * setup_rtt_multiple` delay before its first request is served
//! (`--setup-rtt-multiple`, approximating a TCP/TLS handshake), and the
//! accept loop itself is gated by a [`tokio::sync::Semaphore`]
//! (`--max-connections`).
//!
//! ## Where emulation is applied
//!
//! All of the above is applied on the **client-facing leg only**. The
//! proxy-to-origin leg is treated as "inside the datacenter": fast, local,
//! and unshaped. This is a deliberate simplification (equivalent to running
//! `tc qdisc` on a single interface of a real network emulator) so the
//! whole model lives at the HTTP body-framing level -- pacing bytes handed
//! to the client -- rather than needing a custom `AsyncRead`/`AsyncWrite`
//! wrapper around raw sockets in both directions. See `README.md`'s
//! "Threats to validity" for what this costs in realism.
//!
//! The bandwidth token buckets are still genuinely bidirectional per the
//! task ("in each direction"): `up_bucket` paces bytes read from the
//! client's request body before they are forwarded to the origin,
//! `down_bucket` paces the response body sent back. Because
//! `entrybound::random_access::HttpRangeSource` only ever sends `GET`/`HEAD`
//! (empty bodies), `up_bucket` is exercised but, for that workload, always
//! with `n = 0` -- wired through correctly rather than left dead, in case a
//! future workload proxies a request with an actual body.

use crate::RespBody;
use crate::bandwidth::TokenBucket;
use crate::cache::{CacheKey, CacheStatsSnapshot, CachedResponse, CdnCache};
use crate::config::{CacheMode, ProxyConfig};
use crate::loss::LossModel;
use crate::rng::SeededRng;
use ebr_common::results::ResultWriter;
use ebr_common::runenv::RunContext;
use futures_util::Stream;
use http_body_util::{BodyExt, Full, StreamBody};
use hyper::body::{Bytes, Frame, Incoming};
use hyper::header;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode, Uri};
use hyper_util::client::legacy::Client;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto::Builder as AutoBuilder;
use serde_json::json;
use std::convert::Infallible;
use std::fmt;
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;
use tokio::sync::Semaphore;

#[derive(Debug)]
struct FetchError(String);

impl fmt::Display for FetchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for FetchError {}

#[derive(Debug, Default, Clone)]
struct ResponseMeta {
    etag: Option<String>,
    content_range: Option<String>,
    content_type: Option<String>,
}

fn is_cacheable(status: StatusCode) -> bool {
    matches!(status, StatusCode::OK | StatusCode::PARTIAL_CONTENT)
}

fn meta_from_cached(cached: &CachedResponse) -> ResponseMeta {
    ResponseMeta {
        etag: cached.etag.clone(),
        content_range: cached.content_range.clone(),
        content_type: cached.content_type.clone(),
    }
}

pub struct ProxyState {
    config: ProxyConfig,
    upstream: Client<HttpConnector, Full<Bytes>>,
    up_bucket: TokenBucket,
    down_bucket: TokenBucket,
    loss: LossModel,
    rng: SeededRng,
    cache: CdnCache,
    logger: Option<Mutex<ResultWriter<std::io::BufWriter<std::fs::File>>>>,
}

/// One request's fields for [`ProxyState::log`], bundled so that function
/// stays under clippy's argument-count limit.
struct RequestLog<'a> {
    method: &'a str,
    path: &'a str,
    range: Option<&'a str>,
    status: u16,
    bytes_sent: u64,
    elapsed_ms: f64,
    cache_status: &'a str,
}

impl ProxyState {
    pub fn cache_stats(&self) -> CacheStatsSnapshot {
        self.cache.stats().snapshot()
    }

    fn log(&self, fields: RequestLog<'_>) {
        let Some(logger) = &self.logger else { return };
        let payload = json!({
            "method": fields.method,
            "path": fields.path,
            "range": fields.range,
            "status": fields.status,
            "bytes_sent": fields.bytes_sent,
            "elapsed_ms": fields.elapsed_ms,
            "cache_status": fields.cache_status,
        });
        let _ = logger
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .append(Some(fields.path), payload);
    }
}

async fn fetch_from_origin(
    state: &ProxyState,
    method: &Method,
    path_and_query: &str,
    range: Option<&str>,
    if_match: Option<&str>,
    body: Bytes,
) -> Result<(StatusCode, ResponseMeta, Bytes), FetchError> {
    state.up_bucket.consume(body.len() as u64).await;

    let uri: Uri = Uri::builder()
        .scheme("http")
        .authority(state.config.upstream_authority.clone())
        .path_and_query(path_and_query)
        .build()
        .map_err(|error| FetchError(format!("building upstream URI: {error}")))?;
    let mut builder = Request::builder().method(method.clone()).uri(uri);
    if let Some(value) = range {
        builder = builder.header(header::RANGE, value);
    }
    if let Some(value) = if_match {
        builder = builder.header(header::IF_MATCH, value);
    }
    let request = builder
        .body(Full::new(body))
        .map_err(|error| FetchError(format!("building upstream request: {error}")))?;

    let response = state
        .upstream
        .request(request)
        .await
        .map_err(|error| FetchError(format!("upstream request failed: {error}")))?;
    let status = response.status();
    let meta = ResponseMeta {
        etag: header_str(&response, header::ETAG),
        content_range: header_str(&response, header::CONTENT_RANGE),
        content_type: header_str(&response, header::CONTENT_TYPE),
    };
    let collected = response
        .into_body()
        .collect()
        .await
        .map_err(|error| FetchError(format!("reading upstream body: {error}")))?;
    Ok((status, meta, collected.to_bytes()))
}

fn header_str(response: &Response<Incoming>, name: hyper::header::HeaderName) -> Option<String> {
    response
        .headers()
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string)
}

/// Builds the paced downstream (or, symmetrically, upstream) body stream:
/// `bytes` chopped into `chunk_size`-sized [`Frame`]s, each preceded by a
/// loss-model stall roll and gated by the token bucket before being handed
/// onward. See `bandwidth.rs` and `loss.rs` for what each of those does.
fn shaped_body_stream(
    bytes: Bytes,
    bucket: TokenBucket,
    loss: LossModel,
    rng: SeededRng,
    chunk_size: usize,
) -> impl Stream<Item = Result<Frame<Bytes>, Infallible>> {
    let chunk_size = chunk_size.max(1);
    futures_util::stream::unfold(
        (bytes, 0usize, bucket, loss, rng, chunk_size),
        move |(bytes, offset, bucket, loss, rng, chunk_size)| async move {
            if offset >= bytes.len() {
                return None;
            }
            let end = (offset + chunk_size).min(bytes.len());
            let stall = loss.stall_for_chunk(end - offset, &rng);
            if !stall.is_zero() {
                tokio::time::sleep(stall).await;
            }
            let chunk = bytes.slice(offset..end);
            bucket.consume(chunk.len() as u64).await;
            Some((
                Ok(Frame::data(chunk)),
                (bytes, end, bucket, loss, rng, chunk_size),
            ))
        },
    )
}

async fn handle(
    state: Arc<ProxyState>,
    req: Request<Incoming>,
) -> Result<Response<RespBody>, Infallible> {
    let start = std::time::Instant::now();
    let method = req.method().clone();
    let path_only = req.uri().path().to_string();
    let path_and_query = req
        .uri()
        .path_and_query()
        .map(|value| value.as_str().to_string())
        .unwrap_or_else(|| "/".to_string());
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
    let (_, body) = req.into_parts();
    let request_body = body
        .collect()
        .await
        .map(|collected| collected.to_bytes())
        .unwrap_or_default();

    let one_way = state.config.rtt / 2;
    let jitter_half = state.config.jitter.sample(&state.rng) / 2;

    tokio::time::sleep(one_way + jitter_half).await; // request -> origin leg

    let cache_key = CacheKey::new(path_only.clone(), range_header.as_deref());
    let cacheable_request = method == Method::GET;

    let (status, meta, body_bytes, cache_status): (StatusCode, ResponseMeta, Bytes, &str) =
        if cacheable_request {
            if let Some(cached) = state.cache.get(&cache_key) {
                (
                    StatusCode::from_u16(cached.status).unwrap_or(StatusCode::OK),
                    meta_from_cached(&cached),
                    cached.body.clone(),
                    "hit",
                )
            } else {
                match fetch_from_origin(
                    &state,
                    &method,
                    &path_and_query,
                    range_header.as_deref(),
                    if_match_header.as_deref(),
                    request_body,
                )
                .await
                {
                    Ok((status, meta, bytes)) => {
                        if is_cacheable(status) {
                            state.cache.insert(
                                cache_key,
                                CachedResponse {
                                    status: status.as_u16(),
                                    etag: meta.etag.clone(),
                                    content_range: meta.content_range.clone(),
                                    content_type: meta.content_type.clone(),
                                    body: bytes.clone(),
                                },
                            );
                        }
                        (status, meta, bytes, "miss")
                    }
                    Err(error) => {
                        eprintln!("ebr-netem-proxy: {error}");
                        (
                            StatusCode::BAD_GATEWAY,
                            ResponseMeta::default(),
                            Bytes::new(),
                            "error",
                        )
                    }
                }
            }
        } else {
            match fetch_from_origin(
                &state,
                &method,
                &path_and_query,
                range_header.as_deref(),
                if_match_header.as_deref(),
                request_body,
            )
            .await
            {
                Ok((status, meta, bytes)) => (status, meta, bytes, "bypass"),
                Err(error) => {
                    eprintln!("ebr-netem-proxy: {error}");
                    (
                        StatusCode::BAD_GATEWAY,
                        ResponseMeta::default(),
                        Bytes::new(),
                        "error",
                    )
                }
            }
        };

    tokio::time::sleep(one_way + jitter_half).await; // origin -> response-first-byte leg

    let total_len = body_bytes.len() as u64;
    let paced = shaped_body_stream(
        body_bytes,
        state.down_bucket.clone(),
        state.loss,
        state.rng.clone(),
        state.config.chunk_size,
    );

    let mut builder = Response::builder().status(status);
    if let Some(value) = &meta.etag {
        builder = builder.header(header::ETAG, value.as_str());
    }
    if let Some(value) = &meta.content_range {
        builder = builder.header(header::CONTENT_RANGE, value.as_str());
    }
    if let Some(value) = &meta.content_type {
        builder = builder.header(header::CONTENT_TYPE, value.as_str());
    }
    builder = builder.header(header::CONTENT_LENGTH, total_len.to_string());
    let response = builder.body(StreamBody::new(paced).boxed()).unwrap();

    let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
    state.log(RequestLog {
        method: method.as_str(),
        path: &path_only,
        range: range_header.as_deref(),
        status: status.as_u16(),
        bytes_sent: total_len,
        elapsed_ms,
        cache_status,
    });
    Ok(response)
}

/// Builds a [`ProxyState`] from `config`: sets up the bandwidth buckets, the
/// seeded RNG, the upstream client, the cache, and (`--cache-mode warm`
/// only) pre-populates the cache from `--cache-warm-list` by fetching each
/// listed `(path, range)` from the origin directly (bypassing RTT/bandwidth
/// shaping -- warming is a startup cost, not a proxied request).
pub async fn build(config: ProxyConfig) -> std::io::Result<Arc<ProxyState>> {
    let logger = match &config.log_path {
        None => None,
        Some(path) => {
            let context = RunContext::new("EBR-NETEM-PROXY", config.env_id.clone());
            Some(Mutex::new(ResultWriter::create(path, context)?))
        }
    };
    let up_bucket = match config.bandwidth_up_bps {
        Some(bps) => TokenBucket::new(bps, config.burst_up_bytes),
        None => TokenBucket::unlimited(),
    };
    let down_bucket = match config.bandwidth_down_bps {
        Some(bps) => TokenBucket::new(bps, config.burst_down_bytes),
        None => TokenBucket::unlimited(),
    };
    let loss = LossModel::with_unit(config.loss_p, config.loss_rto, config.loss_unit);
    let rng = SeededRng::new(config.seed);
    let cache = CdnCache::new(
        config.cache_capacity_bytes,
        !matches!(config.cache_mode, CacheMode::Off),
    );
    let upstream: Client<HttpConnector, Full<Bytes>> =
        Client::builder(TokioExecutor::new()).build_http();

    let state = Arc::new(ProxyState {
        config,
        upstream,
        up_bucket,
        down_bucket,
        loss,
        rng,
        cache,
        logger,
    });

    if matches!(state.config.cache_mode, CacheMode::Warm) {
        warm_cache(&state).await;
    }
    Ok(state)
}

async fn warm_cache(state: &Arc<ProxyState>) {
    let Some(path) = &state.config.cache_warm_list else {
        return;
    };
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) => {
            eprintln!(
                "ebr-netem-proxy: cannot read --cache-warm-list {}: {error}",
                path.display()
            );
            return;
        }
    };
    for (item_path, range) in crate::config::parse_warm_list(&text) {
        match fetch_from_origin(
            state,
            &Method::GET,
            &item_path,
            range.as_deref(),
            None,
            Bytes::new(),
        )
        .await
        {
            Ok((status, meta, bytes)) if is_cacheable(status) => {
                state.cache.insert(
                    CacheKey::new(item_path, range.as_deref()),
                    CachedResponse {
                        status: status.as_u16(),
                        etag: meta.etag,
                        content_range: meta.content_range,
                        content_type: meta.content_type,
                        body: bytes,
                    },
                );
            }
            Ok((status, _, _)) => eprintln!(
                "ebr-netem-proxy: warm fetch of {item_path} returned {status}, not caching"
            ),
            Err(error) => eprintln!("ebr-netem-proxy: warm fetch of {item_path} failed: {error}"),
        }
    }
}

/// Accepts connections on `listener` and serves them with `state` until the
/// process is killed. Gated by `state.config.max_connections`: a permit is
/// acquired *before* `accept()`, so once the limit is reached, further TCP
/// connections queue in the kernel backlog instead of being handled --
/// approximating a "max concurrent connections" cap at the application
/// layer (see the module doc comment's scope notes).
pub async fn serve(state: Arc<ProxyState>, listener: TcpListener) -> std::io::Result<()> {
    let semaphore = Arc::new(Semaphore::new(state.config.max_connections.max(1)));
    let auto_builder = Arc::new(AutoBuilder::new(TokioExecutor::new()));
    loop {
        let permit = semaphore
            .clone()
            .acquire_owned()
            .await
            .expect("semaphore is never closed");
        let (stream, _peer) = listener.accept().await?;
        let state = state.clone();
        let builder = auto_builder.clone();
        let setup_delay = state.config.setup_delay();
        tokio::spawn(async move {
            let _permit = permit; // held for the connection's lifetime, bounding concurrency
            tokio::time::sleep(setup_delay).await;
            let io = TokioIo::new(stream);
            let service = service_fn(move |req| {
                let state = state.clone();
                async move { handle(state, req).await }
            });
            if let Err(error) = builder.serve_connection(io, service).await {
                eprintln!("ebr-netem-proxy: connection error: {error}");
            }
        });
    }
}

/// Convenience wrapper for the binary: [`build`] then [`serve`]. Tests that
/// need to inspect [`ProxyState::cache_stats`] after the fact call `build`
/// and `serve` directly instead, so they can keep a handle to the state (see
/// `tests/proxy_netem.rs`).
pub async fn run(config: ProxyConfig, listener: TcpListener) -> std::io::Result<()> {
    let state = build(config).await?;
    serve(state, listener).await
}
