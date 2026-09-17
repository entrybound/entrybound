//! `ebr-planner` binary: the command an `ebr` runner spec's template
//! invokes.
//!
//! ```text
//! ebr-planner --item-path <file> --experiment-id <id> --env-name <name> \
//!             [--profile fast|balanced|dense|extreme] [--run-id <id>] [--out <path>] \
//!             [--patience <n>] [--csv-out <path>] [--max-chunks <n>]
//! ```
//!
//! Reads `--item-path` fully into memory and splits it into the Chunks
//! production's own content-defined chunker assigns under `--profile`
//! ([`ebr_planner::production_chunk_ranges`]). Earlier revisions planned the
//! whole file as one Chunk, which production never does for anything larger
//! than one maximum Chunk size (harness review round 1, finding R1-07), so
//! their regret numbers did not describe production decisions.
//!
//! Appends `ebr.harness.raw.v1` JSONL rows to `--out` (or stdout):
//!
//! - One [`ebr_planner::DriftReport`] per planner generation, over the
//!   chunked archive.
//! - `chunking`: the chunker parameters, Chunk count, and the explicit
//!   `--max-chunks` cap (and whether it truncated the per-chunk rows below).
//! - Per searched Chunk: one `exhaustive_search` row, one `exhaustive_regret`
//!   row per generation, one `greedy` row, and one `tree` row.
//! - `archive_regret`: one row per generation with
//!   [`ebr_planner::exhaustive::ArchiveRegretBounds`], which charge each
//!   distinct plan record once (see that type's docs).
//!
//! `--csv-out`, if given, appends one candidate-regret CSV row per exhaustive
//! candidate per searched Chunk (writing the header first if the file did not
//! already exist).

use ebr_common::cli::Args;
use ebr_common::results::ResultWriter;
use ebr_common::runenv::{EnvIndex, RunContext};
use ebr_planner::exhaustive::{
    ArchiveRegretBounds, ChunkExhaustiveSearch, ChunkSelection, SearchCaps, archive_regret_bounds,
    chunk_regret, plan_in_chunk_search_scope, search_chunk,
};
use ebr_planner::strategies::{evaluate_tree_choice, select_by_tree, select_greedy};
use ebr_planner::{
    archive_for_ranges, check_enumeration_matches_real_planner, production_chunk_ranges,
};
use entrybound::planner::{CompressionProfile, analyze};
use entrybound::research::planner::{ChunkTrace, PlannerVersion, trace_chunk};
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
    let max_chunks: Option<usize> = args
        .get("max-chunks")
        .map(|value| {
            value
                .parse()
                .map_err(|_| format!("--max-chunks {value:?} is not a non-negative integer"))
        })
        .transpose()?;

    let repo_root = ebr_common::discover_repo_root(Path::new(env!("CARGO_MANIFEST_DIR")))
        .ok_or("could not locate the entrybound repo root from CARGO_MANIFEST_DIR")?;
    ebr_common::heldout::assert_inputs_not_heldout(&repo_root, &[item_path.as_path()])?;
    let env_id = EnvIndex::load(&repo_root)?.resolve(&env_name)?.to_string();

    let context = match args.get("run-id") {
        Some(run_id) => RunContext::with_run_id(experiment_id, run_id.to_string(), env_id),
        None => RunContext::new(experiment_id, env_id),
    };

    let bytes = std::fs::read(&item_path)?;
    let (chunking, ranges) = production_chunk_ranges(&bytes, profile)?;
    let unplanned = archive_for_ranges("item", &bytes, &ranges)?;
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

    // 1. Drift reports over the production-chunked archive.
    for version in PlannerVersion::ALL {
        let report = check_enumeration_matches_real_planner(&unplanned, profile, version)?;
        writer.append(Some(&item_id), serde_json::to_value(&report)?)?;
    }

    // 2. Chunking row: what was searched, and any explicit cap.
    let searched = max_chunks.map_or(ranges.len(), |cap| cap.min(ranges.len()));
    writer.append(
        Some(&item_id),
        serde_json::to_value(ChunkingRow {
            kind: "chunking",
            profile: profile.as_str(),
            chunker_id: chunking.chunker_id,
            logical_bytes: bytes.len() as u64,
            chunk_count: ranges.len(),
            searched_chunk_count: searched,
            max_chunks,
            truncated: searched < ranges.len(),
            stage_scope: "chunk-stage; cohort and JPEG-region stages are covered by the drift \
                          reports, not by the regret rows",
        })?,
    )?;

    // 3. Per-chunk exhaustive search, regret, and alternative strategies.
    let caps = SearchCaps::full();
    let mut searches: Vec<ChunkExhaustiveSearch> = Vec::with_capacity(searched);
    let mut traces: Vec<Vec<ChunkTrace>> = Vec::with_capacity(searched);
    for (chunk_index, range) in ranges.iter().take(searched).enumerate() {
        let chunk = &bytes[range.clone()];
        let chunk_item_id = format!("{item_id}#chunk={chunk_index}");
        let search = search_chunk(chunk, &caps)?;
        writer.append(
            Some(&chunk_item_id),
            serde_json::to_value(ExhaustiveSearchRow {
                kind: "exhaustive_search",
                profile: profile.as_str(),
                chunk_index,
                search: &search,
            })?,
        )?;
        if let Some(csv_path) = args.get("csv-out") {
            write_candidate_regret_csv(Path::new(csv_path), &chunk_item_id, &search)?;
        }

        let mut chunk_traces = Vec::with_capacity(PlannerVersion::ALL.len());
        for version in PlannerVersion::ALL {
            let trace = trace_chunk(profile, version, chunk)?;
            let selected = &trace.candidates[trace.selected];
            let in_scope = plan_in_chunk_search_scope(trace.selected_plan())?;
            let recosted = entrybound::research::planner::v5_independent_cost(
                trace.selected_plan(),
                chunk,
                None,
            )
            .ok();
            let regret = if in_scope {
                Some(i64::try_from(chunk_regret(
                    trace.selected_plan(),
                    chunk,
                    &search,
                )?)?)
            } else {
                None
            };
            writer.append(
                Some(&chunk_item_id),
                serde_json::to_value(ExhaustiveRegretRow {
                    kind: "exhaustive_regret",
                    profile: profile.as_str(),
                    version: format!("{version:?}"),
                    chunk_index,
                    selected_stage: format!("{:?}", selected.stage),
                    selected_plan: trace.selected_plan().identifier.clone(),
                    selected_trace_cost: selected.complete_cost,
                    trace_cost_basis: trace_cost_basis(version),
                    selected_recosted_cost: recosted,
                    optimal_cost: search.optimal_cost,
                    in_scope,
                    regret,
                })?,
            )?;
            chunk_traces.push(trace);
        }
        traces.push(chunk_traces);

        let greedy = select_greedy(chunk, &caps, patience.max(1))?;
        writer.append(
            Some(&chunk_item_id),
            serde_json::to_value(GreedyRow {
                kind: "greedy",
                profile: profile.as_str(),
                chunk_index,
                patience: patience.max(1),
                optimal_cost: search.optimal_cost,
                greedy: &greedy,
            })?,
        )?;

        let choice = select_by_tree(analyze(chunk), chunk.len() as u64);
        let evaluated = evaluate_tree_choice(choice, chunk)?;
        writer.append(
            Some(&chunk_item_id),
            serde_json::to_value(TreeRow {
                kind: "tree",
                profile: profile.as_str(),
                chunk_index,
                rule: choice.rule,
                complete_cost: evaluated.complete_cost,
                optimal_cost: search.optimal_cost,
            })?,
        )?;
        searches.push(search);
    }

    // 4. Archive-level (chunk-stage) regret bounds per generation.
    let search_refs: Vec<&ChunkExhaustiveSearch> = searches.iter().collect();
    for (version_index, version) in PlannerVersion::ALL.into_iter().enumerate() {
        let mut selections = Vec::with_capacity(traces.len());
        let mut all_in_scope = true;
        for chunk_traces in &traces {
            let trace = &chunk_traces[version_index];
            let selected = &trace.candidates[trace.selected];
            all_in_scope &= plan_in_chunk_search_scope(trace.selected_plan())?;
            selections.push(ChunkSelection {
                plan: trace.selected_plan(),
                payload_bytes: selected.payload_bytes,
                side_data_bytes: selected.side_data_bytes,
            });
        }
        let bounds = archive_regret_bounds(&search_refs, &selections)?;
        writer.append(
            Some(&item_id),
            serde_json::to_value(ArchiveRegretRow {
                kind: "archive_regret",
                profile: profile.as_str(),
                version: format!("{version:?}"),
                all_selections_in_scope: all_in_scope,
                truncated: searched < ranges.len(),
                bounds,
            })?,
        )?;
    }

    Ok(())
}

fn trace_cost_basis(version: PlannerVersion) -> &'static str {
    match version {
        PlannerVersion::V1 | PlannerVersion::V2 | PlannerVersion::V3 => "payload only",
        PlannerVersion::V4 => "payload + v1 plan record",
        PlannerVersion::V5 | PlannerVersion::V6 => "payload + v2 plan record + side data",
    }
}

#[derive(Serialize)]
struct ChunkingRow {
    kind: &'static str,
    profile: &'static str,
    chunker_id: &'static str,
    logical_bytes: u64,
    chunk_count: usize,
    searched_chunk_count: usize,
    max_chunks: Option<usize>,
    truncated: bool,
    stage_scope: &'static str,
}

#[derive(Serialize)]
struct ExhaustiveSearchRow<'a> {
    kind: &'static str,
    profile: &'static str,
    chunk_index: usize,
    search: &'a ChunkExhaustiveSearch,
}

#[derive(Serialize)]
struct ExhaustiveRegretRow {
    kind: &'static str,
    profile: &'static str,
    version: String,
    chunk_index: usize,
    selected_stage: String,
    selected_plan: String,
    /// The trace's own `complete_cost`, in that generation's cost basis
    /// (`trace_cost_basis`); not directly comparable with `optimal_cost`.
    selected_trace_cost: u64,
    trace_cost_basis: &'static str,
    /// The selected plan recosted exactly as the exhaustive search costs
    /// candidates (`v5_independent_cost`); comparable with `optimal_cost`.
    selected_recosted_cost: Option<u64>,
    optimal_cost: u64,
    /// `false` when the selected plan carries a reconstructive step, which
    /// the chunk-level search does not model.
    in_scope: bool,
    /// `selected_recosted_cost - optimal_cost`; `None` when out of scope.
    regret: Option<i64>,
}

#[derive(Serialize)]
struct ArchiveRegretRow {
    kind: &'static str,
    profile: &'static str,
    version: String,
    all_selections_in_scope: bool,
    truncated: bool,
    bounds: ArchiveRegretBounds,
}

#[derive(Serialize)]
struct GreedyRow<'a> {
    kind: &'static str,
    profile: &'static str,
    chunk_index: usize,
    patience: usize,
    optimal_cost: u64,
    greedy: &'a ebr_planner::strategies::GreedyResult,
}

#[derive(Serialize)]
struct TreeRow {
    kind: &'static str,
    profile: &'static str,
    chunk_index: usize,
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
