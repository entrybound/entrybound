//! `ebr-chunk` binary: the command an `ebr` runner spec's template invokes.
//!
//! ```text
//! ebr-chunk --item-path <file> --experiment-id <id> --env-name <name> \
//!           [--profile fast|balanced|dense|extreme] [--run-id <id>] [--out <path>]
//! ```
//!
//! Reads `--item-path` fully into memory (chunking needs the whole
//! plaintext slice; this is a production API constraint, not a choice this
//! binary makes) and appends one `ebr.harness.raw.v1` JSONL row with
//! [`ebr_chunk::compare`]'s production-vs-research-CDC summary. See
//! `README.md`.

use ebr_common::cli::Args;
use ebr_common::results::ResultWriter;
use ebr_common::runenv::{EnvIndex, RunContext};
use entrybound::chunker::{BALANCED_V2, ChunkingParameters, DENSE_V2, EXTREME_V2, FAST_V2};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("ebr-chunk: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse(std::env::args().skip(1))?;
    let item_path = PathBuf::from(args.require("item-path")?);
    let experiment_id = args.require("experiment-id")?.to_string();
    let env_name = args.require("env-name")?.to_string();
    let parameters = parse_profile(args.get("profile").unwrap_or("balanced"))?;

    let repo_root = ebr_common::discover_repo_root(Path::new(env!("CARGO_MANIFEST_DIR")))
        .ok_or("could not locate the entrybound repo root from CARGO_MANIFEST_DIR")?;
    let env_id = EnvIndex::load(&repo_root)?.resolve(&env_name)?.to_string();

    let context = match args.get("run-id") {
        Some(run_id) => RunContext::with_run_id(experiment_id, run_id.to_string(), env_id),
        None => RunContext::new(experiment_id, env_id),
    };

    let data = std::fs::read(&item_path)?;
    let comparison = ebr_chunk::compare(&data, parameters)?;
    let payload = serde_json::to_value(&comparison)?;
    let item_id = item_path.to_string_lossy().into_owned();

    match args.get("out") {
        Some(out_path) => {
            let mut writer = ResultWriter::create(Path::new(out_path), context)?;
            writer.append(Some(&item_id), payload)?;
        }
        None => {
            let mut writer = ResultWriter::new(std::io::stdout(), context);
            writer.append(Some(&item_id), payload)?;
        }
    }
    Ok(())
}

fn parse_profile(name: &str) -> Result<ChunkingParameters, Box<dyn std::error::Error>> {
    match name {
        "fast" => Ok(FAST_V2),
        "balanced" => Ok(BALANCED_V2),
        "dense" => Ok(DENSE_V2),
        "extreme" => Ok(EXTREME_V2),
        other => Err(format!(
            "unknown --profile {other:?} (expected fast, balanced, dense, or extreme)"
        )
        .into()),
    }
}
