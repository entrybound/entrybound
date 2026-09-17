//! `ebr-codec` binary: the command an `ebr` runner spec's template invokes.
//!
//! ```text
//! ebr-codec <subcommand> --item-path <file> --experiment-id <id> --env-name <name> \
//!           [--level <i32>] [--profile <fast|balanced|dense|extreme>] \
//!           [--candidate <label-substring>] \
//!           [--run-id <id>] [--out <path>]
//! ```
//!
//! Subcommands (first positional argument; `compare` is assumed when it is
//! missing, so existing invocations that predate this file's subcommands
//! keep working unchanged):
//!
//! - `compare` (default) -- [`ebr_codec::compare_zstd_level`]'s
//!   production-vs-external-zstd comparison at `--level` (default `3`, must
//!   be one of `entrybound::research::codec::SUPPORTED_LEVELS`). Writes one
//!   row.
//! - `object-matrix` -- [`ebr_codec::matrix::candidates`] over the whole
//!   item. Writes one row per candidate.
//! - `chunk-matrix` -- splits the item into chunks with
//!   [`ebr_codec::matrix::chunking_parameters_for_profile`] at `--profile`
//!   (default `balanced`), then [`ebr_codec::matrix::candidates`] over each
//!   chunk. Writes one row per (chunk, candidate).
//! - `transform-false-positives` --
//!   [`ebr_codec::matrix::transform_false_positive_analysis`] over the whole
//!   item. Writes one row per structural transform.
//!
//! See `README.md` for the shared flag contract and example `ebr` spec
//! command templates.

use ebr_common::cli::Args;
use ebr_common::results::ResultWriter;
use ebr_common::runenv::{EnvIndex, RunContext};
use entrybound::planner::CompressionProfile;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("ebr-codec: {e}");
            ExitCode::FAILURE
        }
    }
}

fn parse_profile(name: &str) -> Result<CompressionProfile, Box<dyn std::error::Error>> {
    match name {
        "fast" => Ok(CompressionProfile::Fast),
        "balanced" => Ok(CompressionProfile::Balanced),
        "dense" => Ok(CompressionProfile::Dense),
        "extreme" => Ok(CompressionProfile::Extreme),
        other => Err(format!(
            "unknown --profile {other:?}; expected fast, balanced, dense, or extreme"
        )
        .into()),
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut raw_args: Vec<String> = std::env::args().skip(1).collect();
    let subcommand = if raw_args.first().is_some_and(|a| !a.starts_with("--")) {
        raw_args.remove(0)
    } else {
        "compare".to_owned()
    };

    let args = Args::parse(raw_args)?;
    let item_path = PathBuf::from(args.require("item-path")?);
    let experiment_id = args.require("experiment-id")?.to_string();
    let env_name = args.require("env-name")?.to_string();

    let repo_root = ebr_common::discover_repo_root(Path::new(env!("CARGO_MANIFEST_DIR")))
        .ok_or("could not locate the entrybound repo root from CARGO_MANIFEST_DIR")?;
    let env_id = EnvIndex::load(&repo_root)?.resolve(&env_name)?.to_string();

    let context = match args.get("run-id") {
        Some(run_id) => RunContext::with_run_id(experiment_id, run_id.to_string(), env_id),
        None => RunContext::new(experiment_id, env_id),
    };

    ebr_common::heldout::assert_inputs_not_heldout(&repo_root, &[item_path.as_path()])?;
    let candidate_filter = args.get("candidate");
    let data = std::fs::read(&item_path)?;
    let item_id = item_path.to_string_lossy().into_owned();

    let mut writer: Box<dyn RowSink> = match args.get("out") {
        Some(out_path) => Box::new(ResultWriter::create(Path::new(out_path), context)?),
        None => Box::new(ResultWriter::new(std::io::stdout(), context)),
    };

    match subcommand.as_str() {
        "compare" => {
            let level: i32 = args.get("level").unwrap_or("3").parse()?;
            let comparison = ebr_codec::compare_zstd_level(level, &data)?;
            writer.write_row(Some(&item_id), serde_json::to_value(&comparison)?)?;
        }
        "object-matrix" => {
            for outcome in ebr_codec::matrix::candidates_matching(&data, candidate_filter) {
                writer.write_row(Some(&item_id), serde_json::to_value(&outcome)?)?;
            }
        }
        "chunk-matrix" => {
            let profile = parse_profile(args.get("profile").unwrap_or("balanced"))?;
            let parameters = ebr_codec::matrix::chunking_parameters_for_profile(profile);
            let rows =
                ebr_codec::matrix::chunk_matrix_matching(parameters, &data, candidate_filter)
                    .map_err(|d| d.to_string())?;
            for row in rows {
                let chunk_item_id = format!("{item_id}#chunk={}", row.chunk_index);
                for outcome in &row.outcomes {
                    writer.write_row(Some(&chunk_item_id), serde_json::to_value(outcome)?)?;
                }
            }
        }
        "transform-false-positives" => {
            let rows = ebr_codec::matrix::transform_false_positive_analysis(&data)?;
            for row in rows {
                writer.write_row(Some(&item_id), serde_json::to_value(&row)?)?;
            }
        }
        other => return Err(format!("unknown subcommand {other:?}").into()),
    }
    Ok(())
}

/// Small seam so `compare`/`object-matrix`/`chunk-matrix`/
/// `transform-false-positives` can share one `--out`-or-stdout choice
/// without repeating `ResultWriter`'s two constructors' distinct sink types.
trait RowSink {
    fn write_row(
        &mut self,
        item_id: Option<&str>,
        payload: serde_json::Value,
    ) -> std::io::Result<()>;
}

impl RowSink for ResultWriter<std::io::BufWriter<std::fs::File>> {
    fn write_row(
        &mut self,
        item_id: Option<&str>,
        payload: serde_json::Value,
    ) -> std::io::Result<()> {
        self.append(item_id, payload)
    }
}

impl RowSink for ResultWriter<std::io::Stdout> {
    fn write_row(
        &mut self,
        item_id: Option<&str>,
        payload: serde_json::Value,
    ) -> std::io::Result<()> {
        self.append(item_id, payload)
    }
}
