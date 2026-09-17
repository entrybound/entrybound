//! `ebr-planner` binary: the command an `ebr` runner spec's template
//! invokes.
//!
//! ```text
//! ebr-planner --item-path <file> --experiment-id <id> --env-name <name> \
//!             [--profile fast|balanced|dense|extreme] [--run-id <id>] [--out <path>]
//! ```
//!
//! Reads `--item-path` fully into memory as a single-object, single-chunk
//! archive input (via [`ebr_planner::archive_for`]) and appends one
//! `ebr.harness.raw.v1` JSONL row per planner generation
//! (`entrybound::research::planner::PlannerVersion::ALL`), each carrying a
//! [`ebr_planner::DriftReport`]. See `README.md`.

use ebr_common::cli::Args;
use ebr_common::results::ResultWriter;
use ebr_common::runenv::{EnvIndex, RunContext};
use ebr_planner::{NamedInput, archive_for, check_enumeration_matches_real_planner};
use entrybound::planner::CompressionProfile;
use entrybound::research::planner::PlannerVersion;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("ebr-planner: {e}");
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

    let bytes = std::fs::read(&item_path)?;
    let chunk_size = bytes.len().max(1);
    let inputs = [NamedInput {
        name: "item",
        bytes,
        chunk_size,
    }];
    let unplanned = archive_for(&inputs)?;
    let item_id = item_path.to_string_lossy().into_owned();

    let sink: Box<dyn std::io::Write> = match args.get("out") {
        Some(out_path) => {
            let out_path = Path::new(out_path);
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(out_path)?;
            Box::new(std::io::BufWriter::new(file))
        }
        None => Box::new(std::io::stdout()),
    };
    let mut writer = ResultWriter::new(sink, context);
    for version in PlannerVersion::ALL {
        let report = check_enumeration_matches_real_planner(&unplanned, profile, version)?;
        let payload = serde_json::to_value(&report)?;
        writer.append(Some(&item_id), payload)?;
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
