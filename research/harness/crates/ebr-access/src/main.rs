//! `ebr-access` binary: the command an `ebr` runner spec's template
//! invokes.
//!
//! ```text
//! ebr-access --item-path <dir> --experiment-id <id> --env-name <name> \
//!            [--profile fast|balanced|dense|extreme] [--run-id <id>] [--out <path>]
//! ```
//!
//! Plans and encodes `--item-path` once (INDEXED), then runs
//! [`ebr_access::measure_random_access`] over every regular-file entry and
//! [`ebr_access::measure_stream`] over the same planned archive, and appends
//! one `ebr.harness.raw.v1` JSONL row carrying both. See `README.md`.

use ebr_common::cli::Args;
use ebr_common::results::ResultWriter;
use ebr_common::runenv::{EnvIndex, RunContext};
use entrybound::archive::{PackOptions, plan_directory};
use entrybound::eam::{EntryKind, LogicalPath};
use entrybound::ecf::{WriteOptions, encode};
use entrybound::planner::CompressionProfile;
use serde_json::json;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("ebr-access: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse(std::env::args().skip(1))?;
    let item_path = PathBuf::from(args.require("item-path")?);
    let experiment_id = args.require("experiment-id")?.to_string();
    let env_name = args.require("env-name")?.to_string();
    let profile = parse_profile(args.get("profile").unwrap_or("balanced"))?;

    let repo_root = ebr_common::discover_repo_root(Path::new(env!("CARGO_MANIFEST_DIR")))
        .ok_or("could not locate the entrybound repo root from CARGO_MANIFEST_DIR")?;
    let env_id = EnvIndex::load(&repo_root)?.resolve(&env_name)?.to_string();

    let context = match args.get("run-id") {
        Some(run_id) => RunContext::with_run_id(experiment_id, run_id.to_string(), env_id),
        None => RunContext::new(experiment_id, env_id),
    };

    let planned = plan_directory(
        &item_path,
        PackOptions {
            profile,
            ..PackOptions::default()
        },
    )?;
    let encoded = encode(&planned, WriteOptions::default())?;
    let paths: Vec<LogicalPath> = planned
        .entry_set
        .entries()
        .iter()
        .filter(|e| e.kind() == EntryKind::File)
        .map(|e| e.path().clone())
        .collect();

    let random_access = ebr_access::measure_random_access(&encoded.bytes, &paths)?;
    let stream = ebr_access::measure_stream(&planned)?;
    let payload = json!({
        "profile": profile.as_str(),
        "entry_count": paths.len(),
        "random_access": random_access,
        "stream": stream,
    });
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

fn parse_profile(name: &str) -> Result<CompressionProfile, Box<dyn std::error::Error>> {
    match name {
        "fast" => Ok(CompressionProfile::Fast),
        "balanced" => Ok(CompressionProfile::Balanced),
        "dense" => Ok(CompressionProfile::Dense),
        "extreme" => Ok(CompressionProfile::Extreme),
        other => Err(format!(
            "unknown --profile {other:?} (expected fast, balanced, dense, or extreme)"
        )
        .into()),
    }
}
