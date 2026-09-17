//! `ebr-planner` binary: the command an `ebr` runner spec's template
//! invokes.
//!
//! ```text
//! ebr-planner --item-path <file> --experiment-id <id> --env-name <name> \
//!             [--profile fast|balanced|dense|extreme] [--run-id <id>] [--out <path>] \
//!             [--patience <n>] [--csv-out <path>]
//! ```
//!
//! Reads `--item-path` fully into memory as a single-object, single-chunk
//! archive input (via [`ebr_planner::archive_for`]) and appends `ebr.harness.raw.v1`
//! JSONL rows to `--out` (or stdout):
//!
//! - One [`ebr_planner::DriftReport`] per planner generation
//!   (`entrybound::research::planner::PlannerVersion::ALL`) -- unchanged
//!   from this binary's original behavior.
//! - One `exhaustive_search` row: the full [`ebr_planner::exhaustive::ChunkExhaustiveSearch`]
//!   over the item (every candidate [`ebr_planner::exhaustive::SearchCaps::full`]
//!   describes, its cost, and the optimum found).
//! - One `exhaustive_regret` row per planner generation: that generation's
//!   real selected candidate, recosted the same way, against the exhaustive
//!   optimum above (see [`ebr_planner::exhaustive::chunk_regret`]).
//! - One `greedy` row: [`ebr_planner::strategies::select_greedy`]'s pruned
//!   walk, and how it compares to the optimum.
//! - One `tree` row: [`ebr_planner::strategies::select_by_tree`]'s
//!   frozen-threshold choice, and how it compares to the optimum.
//!
//! `--csv-out`, if given, appends one candidate-regret CSV row per exhaustive
//! candidate (writing the header first if the file did not already exist).
//!
//! See `README.md` for full ebr runner spec command templates.

use ebr_common::cli::Args;
use ebr_common::results::ResultWriter;
use ebr_common::runenv::{EnvIndex, RunContext};
use ebr_planner::exhaustive::{ChunkExhaustiveSearch, SearchCaps, chunk_regret, search_chunk};
use ebr_planner::strategies::{evaluate_tree_choice, select_by_tree, select_greedy};
use ebr_planner::{NamedInput, archive_for, check_enumeration_matches_real_planner};
use entrybound::planner::{CompressionProfile, analyze};
use entrybound::research::planner::{PlannerVersion, trace_chunk};
use serde::Serialize;
use std::fs::OpenOptions;
use std::io::Write as _;
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
    let patience: usize = match args.get("patience") {
        Some(value) => value
            .parse()
            .map_err(|_| format!("--patience {value:?} is not a non-negative integer"))?,
        None => 3,
    };

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
        bytes: bytes.clone(),
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

    // 1. Drift reports (unchanged from this binary's original behavior).
    for version in PlannerVersion::ALL {
        let report = check_enumeration_matches_real_planner(&unplanned, profile, version)?;
        let payload = serde_json::to_value(&report)?;
        writer.append(Some(&item_id), payload)?;
    }

    // 2. The exhaustive chunk-level search, once per item.
    let search = search_chunk(&bytes, &SearchCaps::full())?;
    writer.append(
        Some(&item_id),
        serde_json::to_value(ExhaustiveSearchRow {
            kind: "exhaustive_search",
            profile: profile.as_str(),
            search: &search,
        })?,
    )?;

    if let Some(csv_path) = args.get("csv-out") {
        write_candidate_regret_csv(Path::new(csv_path), &item_id, &search)?;
    }

    // 3. Per-generation regret against that exhaustive optimum.
    for version in PlannerVersion::ALL {
        let trace = trace_chunk(profile, version, &bytes)?;
        let selected = &trace.candidates[trace.selected];
        let regret = chunk_regret(trace.selected_plan(), &bytes, &search)?;
        writer.append(
            Some(&item_id),
            serde_json::to_value(ExhaustiveRegretRow {
                kind: "exhaustive_regret",
                profile: profile.as_str(),
                version: format!("{version:?}"),
                selected_stage: format!("{:?}", selected.stage),
                selected_complete_cost: selected.complete_cost,
                optimal_cost: search.optimal_cost,
                regret: i64::try_from(regret).unwrap_or(i64::MAX),
            })?,
        )?;
    }

    // 4. The greedy alternative.
    let greedy = select_greedy(&bytes, &SearchCaps::full(), patience.max(1))?;
    writer.append(
        Some(&item_id),
        serde_json::to_value(GreedyRow {
            kind: "greedy",
            profile: profile.as_str(),
            patience: patience.max(1),
            optimal_cost: search.optimal_cost,
            greedy: &greedy,
        })?,
    )?;

    // 5. The frozen-threshold decision-tree alternative.
    let analysis = analyze(&bytes);
    let choice = select_by_tree(analysis, chunk_size as u64);
    let evaluated = evaluate_tree_choice(choice, &bytes)?;
    writer.append(
        Some(&item_id),
        serde_json::to_value(TreeRow {
            kind: "tree",
            profile: profile.as_str(),
            rule: choice.rule,
            complete_cost: evaluated.complete_cost,
            optimal_cost: search.optimal_cost,
        })?,
    )?;

    Ok(())
}

#[derive(Serialize)]
struct ExhaustiveSearchRow<'a> {
    kind: &'static str,
    profile: &'static str,
    search: &'a ChunkExhaustiveSearch,
}

#[derive(Serialize)]
struct ExhaustiveRegretRow {
    kind: &'static str,
    profile: &'static str,
    version: String,
    selected_stage: String,
    selected_complete_cost: u64,
    optimal_cost: u64,
    /// `selected_complete_cost - optimal_cost`, both costed the same way
    /// (`v5_independent_cost`); `>= 0` except for a v5 DEFLATE-reconstruction
    /// selection, which is out of this search's scope (see `exhaustive.rs`).
    regret: i64,
}

#[derive(Serialize)]
struct GreedyRow<'a> {
    kind: &'static str,
    profile: &'static str,
    patience: usize,
    optimal_cost: u64,
    greedy: &'a ebr_planner::strategies::GreedyResult,
}

#[derive(Serialize)]
struct TreeRow {
    kind: &'static str,
    profile: &'static str,
    rule: &'static str,
    complete_cost: u64,
    optimal_cost: u64,
}

/// Appends one candidate-regret CSV row per exhaustive candidate, writing the
/// header first if `path` did not already exist.
fn write_candidate_regret_csv(
    path: &Path,
    item_id: &str,
    search: &ChunkExhaustiveSearch,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let is_new = !path.exists();
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    if is_new {
        writeln!(
            file,
            "item_id,candidate_index,codec,transform,payload_bytes,plan_record_bytes,complete_cost,is_optimal,regret_vs_optimal"
        )?;
    }
    for (index, candidate) in search.candidates.iter().enumerate() {
        writeln!(
            file,
            "{},{},{},{},{},{},{},{},{}",
            csv_field(item_id),
            index,
            csv_field(&format!("{:?}", candidate.codec)),
            csv_field(&format!("{:?}", candidate.transform)),
            candidate.payload_bytes,
            candidate.plan_record_bytes,
            candidate.complete_cost,
            u8::from(index == search.optimal),
            candidate.complete_cost.saturating_sub(search.optimal_cost),
        )?;
    }
    Ok(())
}

fn csv_field(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
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
