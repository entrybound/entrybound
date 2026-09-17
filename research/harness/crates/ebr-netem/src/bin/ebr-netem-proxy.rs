//! `ebr-netem-proxy` binary entry point.
//!
//! ```text
//! ebr-netem-proxy --upstream <host:port>
//!                  [--listen 127.0.0.1:0] [--rtt-ms 0]
//!                  [--jitter none|uniform|normal]
//!                  [--jitter-max-ms <n>] [--jitter-mean-ms <n>] [--jitter-stddev-ms <n>]
//!                  [--bandwidth-up-mbit <n>] [--bandwidth-down-mbit <n>]
//!                  [--burst-up-bytes 65536] [--burst-down-bytes 65536]
//!                  [--setup-rtt-multiple 1.0]
//!                  [--loss-p 0.0] [--loss-rto-ms 200] [--seed 42]
//!                  [--max-connections 64]
//!                  [--cache-mode off|cold|warm] [--cache-capacity-bytes <n>] [--cache-warm-list <path>]
//!                  [--chunk-size 16384] [--log <path>] [--env-id adhoc]
//! ```
//!
//! See `ebr_netem::proxy` for what each knob does and `README.md` for the
//! full flag reference, the RTT/bandwidth-unit conventions, and the
//! documented threats to validity of application-level network emulation.

use std::process::ExitCode;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> ExitCode {
    let args = match ebr_common::cli::Args::parse(std::env::args().skip(1)) {
        Ok(args) => args,
        Err(error) => {
            eprintln!("ebr-netem-proxy: {error}");
            return ExitCode::FAILURE;
        }
    };
    let config = match ebr_netem::config::ProxyConfig::from_args(&args) {
        Ok(config) => config,
        Err(error) => {
            eprintln!("ebr-netem-proxy: {error}");
            return ExitCode::FAILURE;
        }
    };
    let listen = config.listen;
    let listener = match tokio::net::TcpListener::bind(listen).await {
        Ok(listener) => listener,
        Err(error) => {
            eprintln!("ebr-netem-proxy: cannot bind {listen}: {error}");
            return ExitCode::FAILURE;
        }
    };
    if let Ok(addr) = listener.local_addr() {
        println!(
            "ebr-netem-proxy: listening on {addr}, upstream {}",
            config.upstream_authority
        );
    }
    if let Err(error) = ebr_netem::proxy::run(config, listener).await {
        eprintln!("ebr-netem-proxy: {error}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}
