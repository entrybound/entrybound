//! `ebr-netem-calibrate`: measures `ebr-netem-proxy`'s *achieved* RTT and
//! throughput against its *configured* values, across the task's example
//! settings (RTT 1/20/100/300 ms; bandwidth 1/10/100/1000 Mbit/s), and
//! writes:
//!
//! - Raw per-request JSONL rows (schema `ebr.harness.raw.v1`, via
//!   [`ebr_common::results::ResultWriter`]) under
//!   `research/raw/EXP-NETEM-CAL/<run_id>.jsonl`.
//! - A generated report, `calibration.md`, next to this crate.
//!
//! Both sweeps run one real `ebr-origin` (in-process, on an ephemeral
//! loopback port, serving one generated scratch archive) and a fresh
//! `ebr-netem-proxy` per setting in front of it, with the *other* dimension
//! held out of the way: the RTT sweep uses unmetered bandwidth and a 1-byte
//! range (so bandwidth pacing cannot contribute delay), and the bandwidth
//! sweep uses a fixed, small `--rtt-ms 1` (so the RTT floor is negligible
//! next to the transfer time) and a payload sized to take about one second
//! at the configured rate.
//!
//! **Every number in `calibration.md` is labeled provisional.** This
//! process makes no attempt to quiet the machine (Defender, Hyper-V/VBS,
//! and whatever else is running stay on -- see `PROGRESS.md`'s
//! execution-environment notes), so these are the "smoke timings... labeled
//! non-decision-grade" the task allows, not a substitute for a real
//! quiet-window run.

use ebr_common::results::ResultWriter;
use ebr_common::runenv::RunContext;
use ebr_netem::config::{CacheMode, JitterModel, OriginConfig, ProxyConfig, mbit_to_bytes_per_sec};
use ebr_netem::etag::EtagMode;
use ebr_netem::{origin, proxy};
use http_body_util::{BodyExt, Full};
use hyper::body::Bytes;
use hyper::header;
use hyper_util::client::legacy::Client;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::rt::TokioExecutor;
use serde_json::json;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

const RTT_SETTINGS_MS: [u64; 4] = [1, 20, 100, 300];
const BANDWIDTH_SETTINGS_MBIT: [f64; 4] = [1.0, 10.0, 100.0, 1000.0];
const RTT_REPS: u32 = 15;
const BANDWIDTH_REPS: u32 = 3;
/// Target wall-clock duration for one bandwidth-sweep transfer, at the
/// configured rate. `1000 Mbit/s * 1s` (125,000,000 bytes) sizes the scratch
/// archive below.
const BANDWIDTH_TARGET_SECONDS: f64 = 1.0;
const BANDWIDTH_BURST_BYTES: u64 = 4_096;
const ARCHIVE_NAME: &str = "calib.eb";
/// Comfortably above `1000 Mbit/s * BANDWIDTH_TARGET_SECONDS` (125,000,000
/// bytes).
const ARCHIVE_BYTES: usize = 128 * 1024 * 1024;

struct Row {
    configured: f64,
    mean: f64,
    median: f64,
    p90: f64,
    n: usize,
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return f64::NAN;
    }
    let idx = (((sorted.len() - 1) as f64) * p).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

fn summarize(configured: f64, samples: &[f64]) -> Row {
    if samples.is_empty() {
        return Row {
            configured,
            mean: f64::NAN,
            median: f64::NAN,
            p90: f64::NAN,
            n: 0,
        };
    }
    let mut sorted = samples.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).expect("samples are never NaN"));
    let n = sorted.len();
    let mean = sorted.iter().sum::<f64>() / n as f64;
    Row {
        configured,
        mean,
        median: percentile(&sorted, 0.5),
        p90: percentile(&sorted, 0.9),
        n,
    }
}

async fn spawn_origin(root: PathBuf) -> (JoinHandle<()>, SocketAddr) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind origin listener");
    let addr = listener.local_addr().expect("origin local_addr");
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
        env_id: "calibration".to_string(),
    };
    let handle = tokio::spawn(async move {
        if let Err(error) = origin::run(config, listener).await {
            eprintln!("ebr-netem-calibrate: origin error: {error}");
        }
    });
    tokio::time::sleep(Duration::from_millis(20)).await; // let the accept loop start
    (handle, addr)
}

async fn spawn_proxy(
    upstream: SocketAddr,
    rtt: Duration,
    bandwidth_down_bps: Option<f64>,
    burst_down_bytes: u64,
) -> (JoinHandle<()>, SocketAddr) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind proxy listener");
    let addr = listener.local_addr().expect("proxy local_addr");
    let config = ProxyConfig {
        listen: addr,
        upstream_authority: upstream.to_string(),
        rtt,
        jitter: JitterModel::None,
        bandwidth_up_bps: None,
        bandwidth_down_bps,
        burst_up_bytes: 65_536,
        burst_down_bytes,
        // Isolates the RTT sweep from connection-setup delay: this
        // calibration measures per-request RTT specifically, not the
        // one-time handshake cost (which src/config.rs's own doc comment
        // and README.md already describe analytically: it is just
        // `rtt * setup_rtt_multiple`, nothing to calibrate empirically).
        setup_rtt_multiple: 0.0,
        loss_p: 0.0,
        loss_rto: Duration::ZERO,
        loss_unit: ebr_netem::loss::LossUnit::Packet {
            mss_bytes: ebr_netem::loss::DEFAULT_MSS_BYTES,
        },
        seed: 42,
        max_connections: 64,
        cache_mode: CacheMode::Off,
        cache_capacity_bytes: 0,
        cache_warm_list: None,
        chunk_size: 16_384,
        log_path: None,
        env_id: "calibration".to_string(),
    };
    let handle = tokio::spawn(async move {
        if let Err(error) = proxy::run(config, listener).await {
            eprintln!("ebr-netem-calibrate: proxy error: {error}");
        }
    });
    tokio::time::sleep(Duration::from_millis(15)).await;
    (handle, addr)
}

/// Issues one ranged `GET` through `proxy_addr` and returns the wall-clock
/// elapsed time and the number of body bytes actually received.
async fn timed_range_get(
    client: &Client<HttpConnector, Full<Bytes>>,
    proxy_addr: SocketAddr,
    range: &str,
) -> Result<(Duration, u64), String> {
    let uri: hyper::Uri = format!("http://{proxy_addr}/{ARCHIVE_NAME}")
        .parse()
        .map_err(|error| format!("{error}"))?;
    let request = hyper::Request::builder()
        .method(hyper::Method::GET)
        .uri(uri)
        .header(header::RANGE, range)
        .body(Full::new(Bytes::new()))
        .map_err(|error| format!("{error}"))?;
    let start = std::time::Instant::now();
    let response = client
        .request(request)
        .await
        .map_err(|error| format!("{error}"))?;
    let status = response.status();
    let collected = response
        .into_body()
        .collect()
        .await
        .map_err(|error| format!("{error}"))?;
    let bytes = collected.to_bytes();
    let elapsed = start.elapsed();
    if status != hyper::StatusCode::PARTIAL_CONTENT {
        return Err(format!("expected 206, got {status}"));
    }
    Ok((elapsed, bytes.len() as u64))
}

fn render_report(
    run_id: &str,
    raw_path: &std::path::Path,
    rtt_rows: &[Row],
    bandwidth_rows: &[Row],
) -> String {
    let mut out = String::new();
    out.push_str("# ebr-netem calibration\n\n");
    out.push_str("**Status: PROVISIONAL.** This machine was not confirmed quiet when these\n");
    out.push_str("numbers were captured (Defender real-time protection and Hyper-V/VBS stay\n");
    out.push_str("on throughout -- see `research/PROGRESS.md`'s execution-environment notes),\n");
    out.push_str("and no other Phase B2 agent activity was excluded. Treat every row below as\n");
    out.push_str("a smoke measurement confirming the emulator is roughly calibrated, not as\n");
    out.push_str("decision-grade timing. **Re-run this binary in a confirmed quiet window\n");
    out.push_str("before citing these numbers in an experiment analysis or a Decision Ledger\n");
    out.push_str("entry.**\n\n");
    out.push_str(&format!(
        "Run id: `{run_id}`. Raw JSONL: `{}`.\n\n",
        raw_path.display()
    ));

    out.push_str("## RTT sweep\n\n");
    out.push_str("Bandwidth unmetered, jitter off, loss off, a 1-byte range per request, ");
    out.push_str(&format!(
        "{RTT_REPS} repetitions per setting. Isolates per-request RTT\n"
    ));
    out.push_str("from bandwidth pacing (see `src/proxy.rs`'s doc comment for the one-way\n");
    out.push_str("delay split this measures the sum of).\n\n");
    out.push_str("| Configured RTT (ms) | Achieved mean (ms) | Achieved median (ms) | Achieved p90 (ms) | Error % (mean) | N |\n");
    out.push_str("|---:|---:|---:|---:|---:|---:|\n");
    for row in rtt_rows {
        let error_pct = if row.configured > 0.0 {
            (row.mean - row.configured) / row.configured * 100.0
        } else {
            row.mean
        };
        out.push_str(&format!(
            "| {:.0} | {:.3} | {:.3} | {:.3} | {:+.1}% | {} |\n",
            row.configured, row.mean, row.median, row.p90, error_pct, row.n
        ));
    }

    out.push_str("\n## Bandwidth sweep\n\n");
    out.push_str(&format!(
        "RTT fixed at 1 ms, jitter off, loss off, `--burst-down-bytes {BANDWIDTH_BURST_BYTES}`, \
         a range sized to take about {BANDWIDTH_TARGET_SECONDS:.1}s at the configured rate, \
         {BANDWIDTH_REPS} repetitions per setting.\n\n"
    ));
    out.push_str(
        "The fixed token-bucket burst allowance is included in every transfer's byte \
         count; at the lowest setting (1 Mbit/s) it is a non-negligible fraction of the \
         sampled bytes and biases the achieved rate slightly high. This is a real property \
         of the token-bucket model (an initial burst is *supposed* to be free), not a \
         measurement bug, but it means the 1 Mbit/s row is the least representative of the \
         sustained rate alone.\n\n",
    );
    out.push_str("| Configured (Mbit/s) | Achieved mean (Mbit/s) | Achieved median (Mbit/s) | Achieved p90 (Mbit/s) | Error % (mean) | N |\n");
    out.push_str("|---:|---:|---:|---:|---:|---:|\n");
    for row in bandwidth_rows {
        let to_mbit = |bps: f64| bps * 8.0 / 1_000_000.0;
        let configured_mbit = to_mbit(row.configured);
        let mean_mbit = to_mbit(row.mean);
        let median_mbit = to_mbit(row.median);
        let p90_mbit = to_mbit(row.p90);
        let error_pct = (mean_mbit - configured_mbit) / configured_mbit * 100.0;
        out.push_str(&format!(
            "| {configured_mbit:.0} | {mean_mbit:.2} | {median_mbit:.2} | {p90_mbit:.2} | {error_pct:+.1}% | {} |\n",
            row.n
        ));
    }
    out.push_str(&format!(
        "\nAt high configured rates, expect the *achieved* rate to fall increasingly \
         short of the configured one: [`TokenBucket::consume`](src/bandwidth.rs) is called \
         once per `--chunk-size` ({BANDWIDTH_TARGET_SECONDS}s of bytes / many small chunks \
         at a high rate), and each call's fixed overhead -- a mutex lock, an `Instant::now()`, \
         and the async scheduling and one syscall-sized socket write around it -- does not \
         shrink as the configured rate grows, while the *time budget* between chunks (what \
         the token bucket would otherwise make the proxy sleep for) does. Once that overhead \
         is comparable to or larger than the sleep it's supposed to replace, it dominates and \
         the achieved rate plateaus below the configured one. If this run's 100/1000 Mbit/s \
         rows show a large negative error, that is almost certainly this effect (confirm by \
         re-running `ebr-netem-proxy` by hand with a much larger `--chunk-size` at the same \
         rate and checking whether the gap shrinks), not a bug in the token bucket's math \
         (`src/bandwidth.rs`'s own unit tests check that in isolation, without any real I/O \
         in the loop).\n"
    ));

    out.push_str("\n## Threats to validity\n\n");
    out.push_str(
        "See `README.md`'s \"Threats to validity\" section for the general \
         application-level-emulation caveats (no real TCP congestion control, no real \
         packet loss, HOL blocking differences). This calibration additionally: runs both \
         legs (origin, proxy, and this measuring client) on one host over loopback, so it \
         cannot detect emulator behavior that only manifests over a real NIC or a real \
         multi-hop path; and shares that host's CPU with whatever else is running, which \
         is exactly why every number above is provisional.\n",
    );
    out
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> std::process::ExitCode {
    let repo_root =
        match ebr_common::discover_repo_root(std::path::Path::new(env!("CARGO_MANIFEST_DIR"))) {
            Some(root) => root,
            None => {
                eprintln!("ebr-netem-calibrate: must run from within the entrybound repo checkout");
                return std::process::ExitCode::FAILURE;
            }
        };

    let scratch_dir =
        std::env::temp_dir().join(format!("ebr-netem-calibrate-{}", std::process::id()));
    if let Err(error) = std::fs::create_dir_all(&scratch_dir) {
        eprintln!("ebr-netem-calibrate: cannot create scratch dir: {error}");
        return std::process::ExitCode::FAILURE;
    }
    // Content is irrelevant to timing/throughput measurement; a single
    // repeated byte fills ARCHIVE_BYTES near-instantly regardless of build
    // profile (unlike a per-byte computed pattern).
    if let Err(error) = std::fs::write(scratch_dir.join(ARCHIVE_NAME), vec![0xABu8; ARCHIVE_BYTES])
    {
        eprintln!("ebr-netem-calibrate: cannot write scratch archive: {error}");
        return std::process::ExitCode::FAILURE;
    }

    let run_context = RunContext::new("EXP-NETEM-CAL", "calibration-adhoc");
    let raw_path = repo_root
        .join("research/raw/EXP-NETEM-CAL")
        .join(format!("{}.jsonl", run_context.run_id));
    let mut writer = match ResultWriter::create(&raw_path, run_context.clone()) {
        Ok(writer) => writer,
        Err(error) => {
            eprintln!(
                "ebr-netem-calibrate: cannot create {}: {error}",
                raw_path.display()
            );
            return std::process::ExitCode::FAILURE;
        }
    };

    let (origin_handle, origin_addr) = spawn_origin(scratch_dir.clone()).await;
    let client: Client<HttpConnector, Full<Bytes>> =
        Client::builder(TokioExecutor::new()).build_http();

    let mut rtt_rows = Vec::new();
    for &rtt_ms in &RTT_SETTINGS_MS {
        let (proxy_handle, proxy_addr) =
            spawn_proxy(origin_addr, Duration::from_millis(rtt_ms), None, 65_536).await;
        let mut samples = Vec::new();
        for rep in 0..RTT_REPS {
            match timed_range_get(&client, proxy_addr, "bytes=0-0").await {
                Ok((elapsed, _bytes)) => {
                    let achieved_ms = elapsed.as_secs_f64() * 1000.0;
                    samples.push(achieved_ms);
                    let _ = writer.append(
                        Some("rtt-sweep"),
                        json!({"sweep": "rtt", "configured_rtt_ms": rtt_ms, "rep": rep, "achieved_ms": achieved_ms}),
                    );
                }
                Err(error) => {
                    eprintln!("ebr-netem-calibrate: rtt-sweep rtt={rtt_ms}ms rep={rep}: {error}")
                }
            }
        }
        proxy_handle.abort();
        rtt_rows.push(summarize(rtt_ms as f64, &samples));
    }

    let mut bandwidth_rows = Vec::new();
    for &mbit in &BANDWIDTH_SETTINGS_MBIT {
        let bps = mbit_to_bytes_per_sec(mbit);
        let target_bytes = ((bps * BANDWIDTH_TARGET_SECONDS) as u64)
            .clamp(BANDWIDTH_BURST_BYTES * 2, ARCHIVE_BYTES as u64 - 1);
        let range_header = format!("bytes=0-{}", target_bytes - 1);
        let (proxy_handle, proxy_addr) = spawn_proxy(
            origin_addr,
            Duration::from_millis(1),
            Some(bps),
            BANDWIDTH_BURST_BYTES,
        )
        .await;
        let mut samples = Vec::new();
        for rep in 0..BANDWIDTH_REPS {
            match timed_range_get(&client, proxy_addr, &range_header).await {
                Ok((elapsed, bytes_len)) => {
                    let achieved_bps = bytes_len as f64 / elapsed.as_secs_f64();
                    samples.push(achieved_bps);
                    let _ = writer.append(
                        Some("bandwidth-sweep"),
                        json!({
                            "sweep": "bandwidth", "configured_mbit": mbit, "configured_bps": bps,
                            "rep": rep, "bytes": bytes_len, "elapsed_ms": elapsed.as_secs_f64() * 1000.0,
                            "achieved_bps": achieved_bps,
                        }),
                    );
                }
                Err(error) => {
                    eprintln!("ebr-netem-calibrate: bandwidth-sweep mbit={mbit} rep={rep}: {error}")
                }
            }
        }
        proxy_handle.abort();
        bandwidth_rows.push(summarize(bps, &samples));
    }

    origin_handle.abort();
    std::fs::remove_dir_all(&scratch_dir).ok();

    let report = render_report(&run_context.run_id, &raw_path, &rtt_rows, &bandwidth_rows);
    let report_path = repo_root.join("research/harness/crates/ebr-netem/calibration.md");
    if let Err(error) = std::fs::write(&report_path, report) {
        eprintln!(
            "ebr-netem-calibrate: cannot write {}: {error}",
            report_path.display()
        );
        return std::process::ExitCode::FAILURE;
    }
    println!("ebr-netem-calibrate: wrote {}", report_path.display());
    println!("ebr-netem-calibrate: raw JSONL at {}", raw_path.display());
    std::process::ExitCode::SUCCESS
}
