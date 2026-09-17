//! `ebr-chunk` binary: the command an `ebr` runner spec's template invokes.
//!
//! ```text
//! ebr-chunk <boundaries|dedup|shift-stability|random-access> \
//!           --item-path <file> --experiment-id <id> --env-name <name> \
//!           --algorithm <gear-norm-v1|fixed-size|fastcdc-2016|fastcdc-2020|rabin|buzhash|ae|ram|tttd> \
//!           [--profile fast|balanced|dense|extreme] [--window N] [--normalization 0-3] \
//!           [--run-id <id>] [--out <path>] [subcommand-specific flags]
//! ```
//!
//! Reads `--item-path` fully into memory (chunking needs the whole
//! plaintext slice; this is a production API constraint, not a choice this
//! binary makes) and appends one `ebr.harness.raw.v1` JSONL row with the
//! chosen subcommand's measurement. See `README.md` for every flag and
//! example `ebr` spec command templates.

use ebr_chunk::algorithms::Algorithm;
use ebr_common::cli::Args;
use ebr_common::results::ResultWriter;
use ebr_common::runenv::{EnvIndex, RunContext};
use entrybound::chunker::{BALANCED_V2, ChunkingParameters, DENSE_V2, EXTREME_V2, FAST_V2};
use serde_json::Value;
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
    let mut argv = std::env::args().skip(1);
    let subcommand = argv.next().ok_or(
        "missing subcommand (expected boundaries, dedup, shift-stability, or random-access)",
    )?;
    if subcommand.starts_with("--") {
        return Err(format!(
            "expected a subcommand (boundaries, dedup, shift-stability, random-access) before any flags, got {subcommand:?}"
        )
        .into());
    }
    let args = Args::parse(argv)?;

    let item_path = PathBuf::from(args.require("item-path")?);
    let experiment_id = args.require("experiment-id")?.to_string();
    let env_name = args.require("env-name")?.to_string();

    let profile = parse_profile(args.get("profile").unwrap_or("balanced"))?;
    let window = args.get("window").map(|v| v.parse::<usize>()).transpose()?;
    let normalization = args
        .get("normalization")
        .map(|v| v.parse::<u8>())
        .transpose()?;
    let algorithm_name = args.get("algorithm").unwrap_or("gear-norm-v1");
    let algorithm = Algorithm::from_profile(algorithm_name, profile, window, normalization)?;

    let repo_root = ebr_common::discover_repo_root(Path::new(env!("CARGO_MANIFEST_DIR")))
        .ok_or("could not locate the entrybound repo root from CARGO_MANIFEST_DIR")?;
    let env_id = EnvIndex::load(&repo_root)?.resolve(&env_name)?.to_string();

    let context = match args.get("run-id") {
        Some(run_id) => RunContext::with_run_id(experiment_id, run_id.to_string(), env_id),
        None => RunContext::new(experiment_id, env_id),
    };

    let data = std::fs::read(&item_path)?;
    let item_id = item_path.to_string_lossy().into_owned();

    let payload: Value = match subcommand.as_str() {
        "boundaries" => {
            let buckets: usize = args.get("buckets").unwrap_or("10").parse()?;
            serde_json::to_value(ebr_chunk::boundaries::measure(&data, &algorithm, buckets)?)?
        }
        "dedup" => {
            let mut versions = vec![data.clone()];
            if let Some(list) = args.get("versions") {
                for path in list.split(',').filter(|s| !s.is_empty()) {
                    versions.push(std::fs::read(path)?);
                }
            }
            serde_json::to_value(ebr_chunk::dedup::measure(&versions, &algorithm)?)?
        }
        "shift-stability" => {
            let seed: u64 = args.get("seed").unwrap_or("1").parse()?;
            let insert_bytes: usize = args.get("insert-bytes").unwrap_or("32").parse()?;
            let delete_bytes: usize = args.get("delete-bytes").unwrap_or("32").parse()?;
            let overwrite_bytes: usize = args.get("overwrite-bytes").unwrap_or("32").parse()?;
            serde_json::to_value(ebr_chunk::shift_stability::measure(
                &data,
                &algorithm,
                seed,
                insert_bytes,
                delete_bytes,
                overwrite_bytes,
            )?)?
        }
        "random-access" => {
            let range_bytes: usize = args.get("range-bytes").unwrap_or("4096").parse()?;
            let samples: usize = args.get("samples").unwrap_or("20").parse()?;
            let seed: u64 = args.get("seed").unwrap_or("1").parse()?;
            serde_json::to_value(ebr_chunk::random_access::measure(
                &data,
                &algorithm,
                range_bytes,
                samples,
                seed,
            )?)?
        }
        other => {
            return Err(format!(
                "unknown subcommand {other:?} (expected boundaries, dedup, shift-stability, or random-access)"
            )
            .into());
        }
    };

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
