//! `ebr-origin` binary entry point.
//!
//! ```text
//! ebr-origin --root <dir>
//!            [--listen 127.0.0.1:0] [--etag strong|weak|missing]
//!            [--multi-range true] [--max-ranges 16]
//!            [--full-body-fallback false] [--revision-churn-after <n>]
//!            [--h2 true] [--log <path>] [--env-id adhoc]
//! ```
//!
//! Prints the bound address (useful with `--listen ...:0`) to stdout as
//! `ebr-origin: listening on <addr>` and then serves forever -- see
//! `ebr_netem::origin` for the server itself and `README.md` for the full
//! flag reference and the object-store-misbehavior toggles' semantics.

use std::process::ExitCode;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> ExitCode {
    let args = match ebr_common::cli::Args::parse(std::env::args().skip(1)) {
        Ok(args) => args,
        Err(error) => {
            eprintln!("ebr-origin: {error}");
            return ExitCode::FAILURE;
        }
    };
    let config = match ebr_netem::config::OriginConfig::from_args(&args) {
        Ok(config) => config,
        Err(error) => {
            eprintln!("ebr-origin: {error}");
            return ExitCode::FAILURE;
        }
    };
    // Harness review round 1, finding R1-01: the origin reads every file
    // under --root, so --root must not be (or be inside) a held-out root.
    let Some(repo_root) =
        ebr_common::discover_repo_root(std::path::Path::new(env!("CARGO_MANIFEST_DIR")))
    else {
        eprintln!("ebr-origin: could not locate the entrybound repo root");
        return ExitCode::FAILURE;
    };
    if let Err(error) =
        ebr_common::heldout::assert_inputs_not_heldout(&repo_root, &[config.root.as_path()])
    {
        eprintln!("ebr-origin: {error}");
        return ExitCode::FAILURE;
    }
    let listen = config.listen;
    let listener = match tokio::net::TcpListener::bind(listen).await {
        Ok(listener) => listener,
        Err(error) => {
            eprintln!("ebr-origin: cannot bind {listen}: {error}");
            return ExitCode::FAILURE;
        }
    };
    if let Ok(addr) = listener.local_addr() {
        println!("ebr-origin: listening on {addr}");
    }
    if let Err(error) = ebr_netem::origin::run(config, listener).await {
        eprintln!("ebr-origin: {error}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}
