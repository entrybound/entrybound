//! Observable-behavior tests for `ebr-netem-proxy`, run against a real
//! `ebr-origin` + `ebr-netem-proxy` pair on ephemeral loopback ports. Uses
//! small, test-fast settings (tens of milliseconds, kilobytes), not the
//! task's example calibration settings (`src/bin/ebr-netem-calibrate.rs`
//! covers those, and is explicitly non-decision-grade timing).
//!
//! These are still real wall-clock timing assertions against a shared,
//! possibly busy CI/dev machine, so every bound below is loose (a lower
//! bound close to the configured delay, an upper bound generous enough to
//! absorb scheduling noise) -- this proves the mechanism engages at all,
//! not that it is precisely calibrated (`calibration.md` is what that's
//! for).

use ebr_netem::config::{CacheMode, JitterModel, OriginConfig, ProxyConfig};
use ebr_netem::etag::EtagMode;
use ebr_netem::{origin, proxy};
use http_body_util::{BodyExt, Full};
use hyper::body::Bytes;
use hyper::header;
use hyper_util::client::legacy::Client;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::rt::TokioExecutor;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio::net::TcpListener;

const FILE_NAME: &str = "archive.eb";

fn scratch_dir(label: &str, contents: &[u8]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "ebr-netem-proxy-test-{label}-{}",
        std::process::id()
    ));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join(FILE_NAME), contents).unwrap();
    dir
}

async fn start_origin(root: PathBuf) -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let config = OriginConfig {
        root,
        listen: addr,
        etag_mode: EtagMode::Strong,
        allow_multi_range: true,
        max_ranges_per_request: 16,
        full_body_fallback: false,
        revision_churn_after_requests: None,
        enable_h2: true,
        log_path: None,
        env_id: "test".to_string(),
    };
    tokio::spawn(async move {
        let _ = origin::run(config, listener).await;
    });
    tokio::time::sleep(Duration::from_millis(20)).await;
    addr
}

fn base_proxy_config(listen: SocketAddr, upstream: SocketAddr) -> ProxyConfig {
    ProxyConfig {
        listen,
        upstream_authority: upstream.to_string(),
        rtt: Duration::ZERO,
        jitter: JitterModel::None,
        bandwidth_up_bps: None,
        bandwidth_down_bps: None,
        burst_up_bytes: 65_536,
        burst_down_bytes: 65_536,
        setup_rtt_multiple: 0.0,
        loss_p: 0.0,
        loss_rto: Duration::ZERO,
        seed: 42,
        max_connections: 64,
        cache_mode: CacheMode::Off,
        cache_capacity_bytes: 0,
        cache_warm_list: None,
        chunk_size: 16_384,
        log_path: None,
        env_id: "test".to_string(),
    }
}

async fn start_proxy(config: ProxyConfig) -> (std::sync::Arc<proxy::ProxyState>, SocketAddr) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let mut config = config;
    config.listen = addr;
    let state = proxy::build(config).await.unwrap();
    tokio::spawn(proxy::serve(state.clone(), listener));
    tokio::time::sleep(Duration::from_millis(15)).await;
    (state, addr)
}

fn client() -> Client<HttpConnector, Full<Bytes>> {
    Client::builder(TokioExecutor::new()).build_http()
}

async fn get_range(
    http: &Client<HttpConnector, Full<Bytes>>,
    addr: SocketAddr,
    range: Option<&str>,
) -> (hyper::StatusCode, Bytes, Duration) {
    let uri: hyper::Uri = format!("http://{addr}/{FILE_NAME}").parse().unwrap();
    // `Connection: close` so the proxy's accepted TCP connection (and the
    // `--max-connections` semaphore permit it holds -- see `src/proxy.rs`)
    // is released right after this one response, instead of idling on
    // HTTP/1.1 keep-alive until some later timeout. Without this,
    // `a_low_max_connections_still_serves_every_request_correctly` below
    // would deadlock: a kept-alive first connection would never free its
    // permit for the next one to `accept()`.
    let mut builder = hyper::Request::builder()
        .method(hyper::Method::GET)
        .uri(uri)
        .header(header::CONNECTION, "close");
    if let Some(range) = range {
        builder = builder.header(header::RANGE, range);
    }
    let start = Instant::now();
    let response = http
        .request(builder.body(Full::new(Bytes::new())).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    (status, body, start.elapsed())
}

#[tokio::test]
async fn configured_rtt_is_actually_paid() {
    let origin_addr = start_origin(scratch_dir("rtt", b"0123456789")).await;
    let mut config = base_proxy_config("127.0.0.1:0".parse().unwrap(), origin_addr);
    config.rtt = Duration::from_millis(120);
    let (_state, proxy_addr) = start_proxy(config).await;

    let http = client();
    let (status, body, elapsed) = get_range(&http, proxy_addr, Some("bytes=0-3")).await;
    assert_eq!(status, hyper::StatusCode::PARTIAL_CONTENT);
    assert_eq!(body.as_ref(), b"0123");
    assert!(
        elapsed >= Duration::from_millis(110),
        "elapsed {elapsed:?} should be at least ~1x the 120ms RTT"
    );
    assert!(
        elapsed <= Duration::from_millis(600),
        "elapsed {elapsed:?} suspiciously far above the 120ms RTT"
    );
}

#[tokio::test]
async fn a_zero_rtt_baseline_is_fast() {
    let origin_addr = start_origin(scratch_dir("baseline", b"0123456789")).await;
    let config = base_proxy_config("127.0.0.1:0".parse().unwrap(), origin_addr);
    let (_state, proxy_addr) = start_proxy(config).await;
    let http = client();
    let (status, _, elapsed) = get_range(&http, proxy_addr, Some("bytes=0-3")).await;
    assert_eq!(status, hyper::StatusCode::PARTIAL_CONTENT);
    assert!(
        elapsed <= Duration::from_millis(500),
        "an unshaped local proxy hop took {elapsed:?}, unexpectedly slow"
    );
}

#[tokio::test]
async fn a_bandwidth_cap_measurably_slows_a_transfer() {
    let payload = vec![0xABu8; 64 * 1024]; // 64 KiB
    let origin_addr = start_origin(scratch_dir("bandwidth", &payload)).await;
    let mut config = base_proxy_config("127.0.0.1:0".parse().unwrap(), origin_addr);
    // 64 KiB at 64,000 B/s should take about 1s, well clear of noise.
    config.bandwidth_down_bps = Some(64_000.0);
    config.burst_down_bytes = 1_024;
    config.chunk_size = 4_096;
    let (_state, proxy_addr) = start_proxy(config).await;

    let http = client();
    let (status, body, elapsed) = get_range(&http, proxy_addr, None).await;
    assert_eq!(status, hyper::StatusCode::OK);
    assert_eq!(body.len(), payload.len());
    assert!(
        elapsed >= Duration::from_millis(700),
        "elapsed {elapsed:?} should reflect the ~1s bandwidth cap, not a near-instant transfer"
    );
}

#[tokio::test]
async fn certain_loss_adds_a_stall_per_chunk() {
    let payload = vec![0xCDu8; 5 * 1024]; // 5 chunks at chunk_size=1024
    let origin_addr = start_origin(scratch_dir("loss", &payload)).await;
    let mut config = base_proxy_config("127.0.0.1:0".parse().unwrap(), origin_addr);
    config.chunk_size = 1_024;
    config.loss_p = 1.0; // every chunk stalls
    config.loss_rto = Duration::from_millis(40);
    let (_state, proxy_addr) = start_proxy(config).await;

    let http = client();
    let (status, body, elapsed) = get_range(&http, proxy_addr, None).await;
    assert_eq!(status, hyper::StatusCode::OK);
    assert_eq!(body.len(), payload.len());
    // 5 chunks * 40ms = 200ms of stalling, at minimum.
    assert!(
        elapsed >= Duration::from_millis(180),
        "elapsed {elapsed:?} should reflect 5 forced loss stalls at 40ms each"
    );
}

#[tokio::test]
async fn zero_loss_probability_never_stalls() {
    let payload = vec![0xCDu8; 5 * 1024];
    let origin_addr = start_origin(scratch_dir("no-loss", &payload)).await;
    let mut config = base_proxy_config("127.0.0.1:0".parse().unwrap(), origin_addr);
    config.chunk_size = 1_024;
    config.loss_p = 0.0;
    config.loss_rto = Duration::from_millis(40);
    let (_state, proxy_addr) = start_proxy(config).await;

    let http = client();
    let (status, _, elapsed) = get_range(&http, proxy_addr, None).await;
    assert_eq!(status, hyper::StatusCode::OK);
    // Loose upper bound: this machine is not confirmed quiet (see the
    // module doc comment), so this only needs to rule out something like
    // *every* chunk stalling (5 * 40ms = 200ms) by a wide margin, not
    // pin down an exact fast baseline.
    assert!(
        elapsed <= Duration::from_millis(500),
        "elapsed {elapsed:?} should not include any loss stalls"
    );
}

#[tokio::test]
async fn a_cache_hit_is_counted_and_skips_the_origin_leg() {
    let origin_addr = start_origin(scratch_dir("cache", b"0123456789ABCDEF")).await;
    let mut config = base_proxy_config("127.0.0.1:0".parse().unwrap(), origin_addr);
    config.rtt = Duration::from_millis(60); // large enough that a cache hit is visibly faster
    config.cache_mode = CacheMode::Cold;
    config.cache_capacity_bytes = 4096;
    let (state, proxy_addr) = start_proxy(config).await;
    let http = client();

    let (status_a, body_a, elapsed_miss) = get_range(&http, proxy_addr, Some("bytes=0-3")).await;
    assert_eq!(status_a, hyper::StatusCode::PARTIAL_CONTENT);
    assert_eq!(body_a.as_ref(), b"0123");

    let (status_b, body_b, elapsed_hit) = get_range(&http, proxy_addr, Some("bytes=0-3")).await;
    assert_eq!(status_b, hyper::StatusCode::PARTIAL_CONTENT);
    assert_eq!(body_b.as_ref(), b"0123");

    let stats = state.cache_stats();
    assert_eq!(stats.misses, 1);
    assert_eq!(stats.hits, 1);
    // Both legs still pay the client-facing RTT (see proxy.rs's doc
    // comment: a cache hit skips the *origin* leg, not the client-facing
    // one), so this only asserts the hit is not dramatically slower --
    // the meaningful, non-flaky assertion here is the hit/miss count above.
    assert!(
        elapsed_hit <= elapsed_miss + Duration::from_millis(100),
        "cache hit ({elapsed_hit:?}) unexpectedly slower than the miss ({elapsed_miss:?})"
    );
}

#[tokio::test]
async fn distinct_ranges_are_independent_cache_entries() {
    let origin_addr = start_origin(scratch_dir("cache-ranges", b"0123456789ABCDEF")).await;
    let mut config = base_proxy_config("127.0.0.1:0".parse().unwrap(), origin_addr);
    config.cache_mode = CacheMode::Cold;
    config.cache_capacity_bytes = 4096;
    let (state, proxy_addr) = start_proxy(config).await;
    let http = client();

    get_range(&http, proxy_addr, Some("bytes=0-3")).await;
    get_range(&http, proxy_addr, Some("bytes=4-7")).await;
    get_range(&http, proxy_addr, Some("bytes=0-3")).await; // repeat: a hit

    let stats = state.cache_stats();
    assert_eq!(stats.misses, 2, "two distinct ranges should each miss once");
    assert_eq!(stats.hits, 1, "the repeated range should hit once");
}

#[tokio::test]
async fn a_low_max_connections_still_serves_every_request_correctly() {
    let origin_addr = start_origin(scratch_dir("maxconn", b"0123456789ABCDEF")).await;
    let mut config = base_proxy_config("127.0.0.1:0".parse().unwrap(), origin_addr);
    config.max_connections = 1;
    let (_state, proxy_addr) = start_proxy(config).await;

    // Concurrency is bounded, not refused: every request must still
    // eventually complete with the right bytes, just serialized.
    let mut tasks = Vec::new();
    for _ in 0..4 {
        let http = client();
        tasks.push(tokio::spawn(async move {
            get_range(&http, proxy_addr, Some("bytes=0-3")).await
        }));
    }
    for task in tasks {
        let (status, body, _elapsed) = task.await.unwrap();
        assert_eq!(status, hyper::StatusCode::PARTIAL_CONTENT);
        assert_eq!(body.as_ref(), b"0123");
    }
}
