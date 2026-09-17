//! Wire-level contract tests for `ebr-origin`, run against a real server on
//! an ephemeral loopback port -- not the unit-level `Range`/ETag logic
//! already covered in `src/range.rs`/`src/etag.rs`'s own `#[cfg(test)]`
//! modules.
//!
//! The most important tests here (`real_client_reads_a_range_it_asks_for`,
//! `real_client_refuses_a_weak_etag`, `real_client_refuses_a_full_body_fallback`)
//! drive the real, unmodified
//! `entrybound::random_access::HttpRangeSource` (a dev-dependency, default
//! features, no `research-internals`) against this origin, proving it
//! satisfies the production client's actual contract -- not just this
//! crate's own reading of `crates/entrybound/src/random_access.rs`.

use ebr_netem::config::OriginConfig;
use ebr_netem::etag::EtagMode;
use entrybound::diagnostics::ReasonCode;
use entrybound::random_access::{HttpRangeSource, RandomReadSource};
use http_body_util::{BodyExt, Full};
use hyper::body::Bytes;
use hyper::header::{self, HeaderName};
use hyper::{HeaderMap, Method, Request, StatusCode, Uri};
use hyper_util::client::legacy::Client;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::rt::TokioExecutor;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

const FILE_NAME: &str = "archive.eb";
const FILE_CONTENTS: &[u8] = b"0123456789ABCDEF"; // 16 bytes

fn scratch_dir(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "ebr-netem-origin-contract-{label}-{}",
        std::process::id()
    ));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join(FILE_NAME), FILE_CONTENTS).unwrap();
    dir
}

fn base_config(root: PathBuf) -> OriginConfig {
    OriginConfig {
        root,
        listen: "127.0.0.1:0".parse().unwrap(),
        etag_mode: EtagMode::Strong,
        allow_multi_range: true,
        max_ranges_per_request: 16,
        full_body_fallback: false,
        revision_churn_after_requests: None,
        enable_h2: true,
        log_path: None,
        env_id: "test".to_string(),
    }
}

async fn start(config: OriginConfig) -> (JoinHandle<()>, SocketAddr) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        let _ = ebr_netem::origin::run(config, listener).await;
    });
    tokio::time::sleep(Duration::from_millis(20)).await;
    (handle, addr)
}

fn client() -> Client<HttpConnector, Full<Bytes>> {
    Client::builder(TokioExecutor::new()).build_http()
}

async fn request(
    http: &Client<HttpConnector, Full<Bytes>>,
    addr: SocketAddr,
    method: Method,
    path: &str,
    headers: &[(HeaderName, &str)],
) -> (StatusCode, HeaderMap, Bytes) {
    let uri: Uri = format!("http://{addr}{path}").parse().unwrap();
    let mut builder = Request::builder().method(method).uri(uri);
    for (name, value) in headers {
        builder = builder.header(name.clone(), *value);
    }
    let response = http
        .request(builder.body(Full::new(Bytes::new())).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let response_headers = response.headers().clone();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    (status, response_headers, body)
}

fn header_str(headers: &HeaderMap, name: HeaderName) -> Option<&str> {
    headers.get(name).and_then(|v| v.to_str().ok())
}

#[tokio::test]
async fn head_reports_strong_etag_and_no_content_encoding() {
    let dir = scratch_dir("head");
    let (_handle, addr) = start(base_config(dir.clone())).await;
    let http = client();
    let (status, headers, body) =
        request(&http, addr, Method::HEAD, &format!("/{FILE_NAME}"), &[]).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(header_str(&headers, header::CONTENT_LENGTH), Some("16"));
    assert_eq!(header_str(&headers, header::ACCEPT_RANGES), Some("bytes"));
    assert!(header_str(&headers, header::CONTENT_ENCODING).is_none());
    let etag = header_str(&headers, header::ETAG).expect("strong mode must send an ETag");
    assert!(etag.starts_with('"') && etag.ends_with('"') && !etag.starts_with("W/"));
    assert!(body.is_empty(), "HEAD must never send a body");
    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn single_range_returns_an_exact_206() {
    let dir = scratch_dir("single-range");
    let (_handle, addr) = start(base_config(dir.clone())).await;
    let http = client();
    let (_, head_headers, _) =
        request(&http, addr, Method::HEAD, &format!("/{FILE_NAME}"), &[]).await;
    let etag = header_str(&head_headers, header::ETAG).unwrap().to_string();

    let (status, headers, body) = request(
        &http,
        addr,
        Method::GET,
        &format!("/{FILE_NAME}"),
        &[
            (header::RANGE, "bytes=2-5"),
            (header::IF_MATCH, etag.as_str()),
        ],
    )
    .await;
    assert_eq!(status, StatusCode::PARTIAL_CONTENT);
    assert_eq!(
        header_str(&headers, header::CONTENT_RANGE),
        Some("bytes 2-5/16")
    );
    assert_eq!(header_str(&headers, header::CONTENT_LENGTH), Some("4"));
    assert_eq!(header_str(&headers, header::ETAG), Some(etag.as_str()));
    assert_eq!(body.as_ref(), &FILE_CONTENTS[2..=5]);
    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn a_range_entirely_past_the_end_is_416_with_a_star_content_range() {
    let dir = scratch_dir("416");
    let (_handle, addr) = start(base_config(dir.clone())).await;
    let http = client();
    let (status, headers, body) = request(
        &http,
        addr,
        Method::GET,
        &format!("/{FILE_NAME}"),
        &[(header::RANGE, "bytes=100-200")],
    )
    .await;
    assert_eq!(status, StatusCode::RANGE_NOT_SATISFIABLE);
    assert_eq!(
        header_str(&headers, header::CONTENT_RANGE),
        Some("bytes */16")
    );
    assert!(body.is_empty());
    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn a_stale_if_match_is_412() {
    let dir = scratch_dir("412");
    let (_handle, addr) = start(base_config(dir.clone())).await;
    let http = client();
    let (status, _, body) = request(
        &http,
        addr,
        Method::GET,
        &format!("/{FILE_NAME}"),
        &[
            (header::RANGE, "bytes=0-3"),
            (header::IF_MATCH, "\"not-the-real-etag\""),
        ],
    )
    .await;
    assert_eq!(status, StatusCode::PRECONDITION_FAILED);
    assert!(body.is_empty());
    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn a_genuine_multirange_request_gets_multipart_byteranges() {
    let dir = scratch_dir("multipart");
    let (_handle, addr) = start(base_config(dir.clone())).await;
    let http = client();
    let (status, headers, body) = request(
        &http,
        addr,
        Method::GET,
        &format!("/{FILE_NAME}"),
        &[(header::RANGE, "bytes=0-1,4-5")],
    )
    .await;
    assert_eq!(status, StatusCode::PARTIAL_CONTENT);
    let content_type = header_str(&headers, header::CONTENT_TYPE)
        .unwrap()
        .to_string();
    assert!(content_type.starts_with("multipart/byteranges; boundary="));
    let boundary = content_type
        .strip_prefix("multipart/byteranges; boundary=")
        .unwrap();

    let ranges =
        ebr_netem::range::parse_range_header("bytes=0-1,4-5", FILE_CONTENTS.len() as u64).unwrap();
    let expected = ebr_netem::range::build_multipart_body(
        boundary,
        FILE_CONTENTS.len() as u64,
        FILE_CONTENTS,
        &ranges,
    );
    assert_eq!(body.as_ref(), expected.as_slice());
    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn multirange_with_multi_range_disabled_serves_only_the_first_range() {
    let mut config = base_config(scratch_dir("multi-disabled"));
    config.allow_multi_range = false;
    let dir = config.root.clone();
    let (_handle, addr) = start(config).await;
    let http = client();
    let (status, headers, body) = request(
        &http,
        addr,
        Method::GET,
        &format!("/{FILE_NAME}"),
        &[(header::RANGE, "bytes=0-1,4-5")],
    )
    .await;
    assert_eq!(status, StatusCode::PARTIAL_CONTENT);
    assert_eq!(
        header_str(&headers, header::CONTENT_RANGE),
        Some("bytes 0-1/16")
    );
    assert_eq!(body.as_ref(), &FILE_CONTENTS[0..=1]);
    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn too_many_ranges_is_416() {
    let mut config = base_config(scratch_dir("too-many"));
    config.max_ranges_per_request = 1;
    let dir = config.root.clone();
    let (_handle, addr) = start(config).await;
    let http = client();
    let (status, _, _) = request(
        &http,
        addr,
        Method::GET,
        &format!("/{FILE_NAME}"),
        &[(header::RANGE, "bytes=0-1,4-5")],
    )
    .await;
    assert_eq!(status, StatusCode::RANGE_NOT_SATISFIABLE);
    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn revision_churn_flips_bytes_and_etag_after_the_threshold() {
    let mut config = base_config(scratch_dir("churn"));
    config.revision_churn_after_requests = Some(2);
    let dir = config.root.clone();
    let (_handle, addr) = start(config).await;
    let http = client();

    // Request 1: HEAD, captures the original ETag.
    let (_, head_headers, _) =
        request(&http, addr, Method::HEAD, &format!("/{FILE_NAME}"), &[]).await;
    let original_etag = header_str(&head_headers, header::ETAG).unwrap().to_string();

    // Request 2: still before the threshold (revision_churn_after_requests
    // counts requests *strictly greater than* 2 as churned).
    let (status, _, body) = request(
        &http,
        addr,
        Method::GET,
        &format!("/{FILE_NAME}"),
        &[
            (header::RANGE, "bytes=0-3"),
            (header::IF_MATCH, original_etag.as_str()),
        ],
    )
    .await;
    assert_eq!(status, StatusCode::PARTIAL_CONTENT);
    assert_eq!(body.as_ref(), &FILE_CONTENTS[0..=3]);

    // Request 3: past the threshold, so the original ETag is now stale.
    let (status, _, _) = request(
        &http,
        addr,
        Method::GET,
        &format!("/{FILE_NAME}"),
        &[
            (header::RANGE, "bytes=0-3"),
            (header::IF_MATCH, original_etag.as_str()),
        ],
    )
    .await;
    assert_eq!(
        status,
        StatusCode::PRECONDITION_FAILED,
        "the original ETag must no longer match past the churn threshold"
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// `entrybound::random_access::HttpRangeSource` is built on
/// `reqwest::blocking`, which must never be called from a thread that
/// already has a Tokio runtime entered on it (reqwest's blocking client
/// builds its own internal runtime and the two would conflict). So, unlike
/// every other test in this file, these four run the origin server on its
/// own dedicated background OS thread with its own dedicated runtime, and
/// call the real client from the *test* thread, which never enters Tokio at
/// all -- a plain `#[test]`, not `#[tokio::test]`.
fn start_on_background_thread(config: OriginConfig) -> SocketAddr {
    let (addr_tx, addr_rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Runtime::new()
            .expect("build a Tokio runtime for the background origin thread");
        runtime.block_on(async move {
            let listener = TcpListener::bind("127.0.0.1:0")
                .await
                .expect("bind origin listener");
            addr_tx
                .send(listener.local_addr().unwrap())
                .expect("test thread is still waiting for the address");
            let _ = ebr_netem::origin::run(config, listener).await;
        });
    });
    addr_rx
        .recv()
        .expect("background origin thread reports its bound address")
}

#[test]
fn real_client_reads_a_range_it_asks_for() {
    let dir = scratch_dir("real-client-ok");
    let addr = start_on_background_thread(base_config(dir.clone()));
    let url = format!("http://{addr}/{FILE_NAME}");
    let source = HttpRangeSource::open(&url).unwrap();
    assert_eq!(source.len().unwrap(), FILE_CONTENTS.len() as u64);
    let slice = source.read_exact_at(2, 4).unwrap();
    assert_eq!(slice, FILE_CONTENTS[2..6]);
    let whole = source.read_exact_at(0, FILE_CONTENTS.len() as u64).unwrap();
    assert_eq!(whole, FILE_CONTENTS);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn real_client_refuses_a_weak_etag() {
    let mut config = base_config(scratch_dir("real-client-weak"));
    config.etag_mode = EtagMode::Weak;
    let dir = config.root.clone();
    let addr = start_on_background_thread(config);
    let url = format!("http://{addr}/{FILE_NAME}");
    // `HttpRangeSource` (the `Ok` side) is not `Debug`, so `unwrap_err()`
    // cannot be used directly; match instead.
    let error = match HttpRangeSource::open(&url) {
        Err(error) => error,
        Ok(_) => panic!("expected open() to refuse a weak ETag"),
    };
    assert_eq!(error.code(), ReasonCode::HttpRangeUnsupported);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn real_client_refuses_a_missing_etag() {
    let mut config = base_config(scratch_dir("real-client-missing"));
    config.etag_mode = EtagMode::Missing;
    let dir = config.root.clone();
    let addr = start_on_background_thread(config);
    let url = format!("http://{addr}/{FILE_NAME}");
    let error = match HttpRangeSource::open(&url) {
        Err(error) => error,
        Ok(_) => panic!("expected open() to refuse a missing ETag"),
    };
    // Unlike a weak ETag (a present-but-non-conformant value, which
    // `parse_strong_etag` reports as `HttpRangeUnsupported`), a wholly
    // *absent* ETag header fails earlier in that same function, at the
    // `.ok_or_else(|| invalid_http(...))` before the strong/weak check ever
    // runs, and `invalid_http` reports the distinct `HttpRangeInvalid` code
    // (see `crates/entrybound/src/random_access.rs`'s `parse_strong_etag`
    // and `invalid_http`).
    assert_eq!(error.code(), ReasonCode::HttpRangeInvalid);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn real_client_refuses_a_full_body_fallback() {
    let mut config = base_config(scratch_dir("real-client-fallback"));
    config.full_body_fallback = true;
    let dir = config.root.clone();
    let addr = start_on_background_thread(config);
    let url = format!("http://{addr}/{FILE_NAME}");
    // `open()` only issues a HEAD, which still looks conformant (strong
    // ETag, Accept-Ranges) -- the fallback only misbehaves on the ranged
    // GET that read_exact_at issues.
    let source = HttpRangeSource::open(&url).unwrap();
    let error = source.read_exact_at(2, 4).unwrap_err();
    assert_eq!(error.code(), ReasonCode::HttpRangeUnsupported);
    std::fs::remove_dir_all(&dir).ok();
}
