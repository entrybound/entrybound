//! `ebr-access` binary: the command an `ebr` runner spec's template
//! invokes. See `README.md` for the full flag reference and example specs.
//!
//! Two modes, chosen with `--mode` (default `roundtrip`):
//!
//! ```text
//! ebr-access --mode roundtrip --item-path <dir> --experiment-id <id> --env-name <name> \
//!            [--profile fast|balanced|dense|extreme] \
//!            [--source-kind memory|local-file|http] [--source-url <url>] \
//!            [--sample-size <n>] [--sample-seed <n>] \
//!            [--large-entry-threshold-bytes <n>] [--byte-ranges-per-large-entry <n>] \
//!            [--byte-range-len <n>] \
//!            [--coalesce-gap-bytes <n>] [--max-dependency-chunks <n>] \
//!            [--max-total-bytes-fetched <n>] [--max-individual-range-bytes <n>] \
//!            [--run-id <id>] [--out <path>]
//!
//! ebr-access --mode probe --experiment-id <id> --env-name <name> \
//!            [--item-path <label>] [--profile fast|balanced|dense|extreme] \
//!            [--probe-total-bytes <n>] [--probe-seed <n>] \
//!            [--probe-sample-interval-ms <n>] \
//!            [--probe-stages pack,verify,unpack,repack,export] \
//!            [--probe-scratch-root <dir>] [--probe-isolate true|false] \
//!            [--run-id <id>] [--out <path>]
//! ```
//!
//! `roundtrip` plans and encodes `--item-path` once (INDEXED for random
//! access, STREAM for [`ebr_access::measure_stream`]), then runs
//! [`ebr_access::measure_random_access`] over every regular-file entry and
//! [`ebr_access::measure_stream`] over the same planned archive. `probe`
//! runs [`ebr_access::run_probe`]'s streaming memory probe over a synthetic
//! generated input instead of a real `--item-path` (see
//! `ebr_access::ProbeConfig`'s module docs for what it measures and why
//! `repack`/`export` are opt-in). Either way, one `ebr.harness.raw.v1` JSONL
//! row is appended carrying the result.
//!
//! `--probe-isolate` (default `true`) runs the probe's `verify` and `unpack`
//! stages in fresh child processes of this binary (`--mode probe-stage`, an
//! internal mode that prints one raw JSON stage measurement), so their memory
//! cannot inherit the `pack` stage's resident or allocator-retained memory
//! (harness review round 1, finding R1-05).

use ebr_access::{
    ProbeConfig, ProbeStage, RandomAccessConfig, default_stages, measure_random_access,
    measure_stream, run_probe, run_single_stage,
};
use ebr_common::cli::Args;
use ebr_common::results::ResultWriter;
use ebr_common::runenv::{EnvIndex, RunContext};
use entrybound::archive::{PackOptions, plan_directory};
use entrybound::eam::{EntryKind, LogicalPath};
use entrybound::ecf::{WriteOptions, encode};
use entrybound::planner::CompressionProfile;
use entrybound::random_access::{
    HttpRangeSource, LocalFileRandomReadSource, MemoryRandomReadSource, RandomAccessPolicy,
};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

type BoxError = Box<dyn std::error::Error>;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("ebr-access: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), BoxError> {
    let args = Args::parse(std::env::args().skip(1))?;
    if args.get("mode") == Some("probe-stage") {
        return run_probe_stage_child(&args);
    }
    let experiment_id = args.require("experiment-id")?.to_string();
    let env_name = args.require("env-name")?.to_string();
    let mode = args.get("mode").unwrap_or("roundtrip").to_string();

    let repo_root = ebr_common::discover_repo_root(Path::new(env!("CARGO_MANIFEST_DIR")))
        .ok_or("could not locate the entrybound repo root from CARGO_MANIFEST_DIR")?;
    let env_id = EnvIndex::load(&repo_root)?.resolve(&env_name)?.to_string();
    let context = match args.get("run-id") {
        Some(run_id) => RunContext::with_run_id(experiment_id, run_id.to_string(), env_id),
        None => RunContext::new(experiment_id, env_id),
    };

    let (item_id, payload) = match mode.as_str() {
        "roundtrip" => {
            let item_path = PathBuf::from(args.require("item-path")?);
            ebr_common::heldout::assert_inputs_not_heldout(&repo_root, &[item_path.as_path()])?;
            run_roundtrip(&args)?
        }
        "probe" => run_probe_mode(&args)?,
        other => {
            return Err(format!("unknown --mode {other:?} (expected roundtrip or probe)").into());
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

fn run_roundtrip(args: &Args) -> Result<(String, Value), BoxError> {
    let item_path = PathBuf::from(args.require("item-path")?);
    let profile = parse_profile(args.get("profile").unwrap_or("balanced"))?;

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

    let random_config = RandomAccessConfig {
        policy: build_random_access_policy(args)?,
        sample_size: parse_or(args, "sample-size", 8)?,
        seed: parse_or(args, "sample-seed", 0)?,
        large_entry_threshold_bytes: parse_or(args, "large-entry-threshold-bytes", 64 * 1024)?,
        byte_ranges_per_large_entry: parse_or(args, "byte-ranges-per-large-entry", 4)?,
        byte_range_len: parse_or(args, "byte-range-len", 4096)?,
    };

    let source_kind = args.get("source-kind").unwrap_or("memory").to_string();
    let random_access = match source_kind.as_str() {
        "memory" => measure_random_access(
            MemoryRandomReadSource::new(encoded.bytes.clone()),
            &paths,
            random_config,
        )?,
        "local-file" => {
            let scratch_dir = std::env::temp_dir().join(format!(
                "ebr-access-cli-source-{}-{}",
                std::process::id(),
                ebr_common::timing::generate_run_id()
            ));
            std::fs::create_dir_all(&scratch_dir)?;
            let source_path = scratch_dir.join("archive.eb");
            std::fs::write(&source_path, &encoded.bytes)?;
            let source = match LocalFileRandomReadSource::open(&source_path) {
                Ok(source) => source,
                Err(e) => {
                    std::fs::remove_dir_all(&scratch_dir).ok();
                    return Err(e.into());
                }
            };
            let result = measure_random_access(source, &paths, random_config);
            std::fs::remove_dir_all(&scratch_dir).ok();
            result?
        }
        "http" => {
            // The caller is responsible for `--source-url` already serving
            // these exact `--item-path` encoded bytes with byte-range
            // support (e.g. via `ebr-netem`, built separately -- see
            // README "Non-goals"); this binary has no server of its own.
            let url = args.require("source-url")?;
            measure_random_access(HttpRangeSource::open(url)?, &paths, random_config)?
        }
        other => {
            return Err(format!(
                "unknown --source-kind {other:?} (expected memory, local-file, or http)"
            )
            .into());
        }
    };

    let stream = measure_stream(&planned)?;
    let source_page_cache = match source_kind.as_str() {
        "memory" => "not applicable: archive bytes held in process memory",
        "local-file" => "hot: archive file written by this process immediately before measurement",
        _ => "unknown: remote source",
    };
    let payload = json!({
        "mode": "roundtrip",
        "profile": profile.as_str(),
        "source_kind": source_kind,
        "source_page_cache": source_page_cache,
        "entry_count": paths.len(),
        "random_access": random_access,
        "stream": stream,
    });
    Ok((item_path.to_string_lossy().into_owned(), payload))
}

fn run_probe_mode(args: &Args) -> Result<(String, Value), BoxError> {
    let profile = parse_profile(args.get("profile").unwrap_or("fast"))?;
    let total_bytes = parse_or(args, "probe-total-bytes", 8 * 1024 * 1024)?;
    let seed = parse_or(args, "probe-seed", 0)?;
    let sample_interval_ms: u64 = parse_or(args, "probe-sample-interval-ms", 50)?;
    let scratch_root = args
        .get("probe-scratch-root")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let stages = match args.get("probe-stages") {
        Some(value) => value
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ProbeStage::parse)
            .collect::<Result<Vec<_>, _>>()?,
        None => default_stages(),
    };

    let isolate = match args.get("probe-isolate") {
        None | Some("true") => true,
        Some("false") => false,
        Some(other) => {
            return Err(format!("--probe-isolate expects true or false, got {other:?}").into());
        }
    };
    let isolate_exe = if isolate {
        Some(std::env::current_exe()?)
    } else {
        None
    };
    let measurement = run_probe(ProbeConfig {
        total_bytes,
        seed,
        sample_interval: Duration::from_millis(sample_interval_ms),
        stages,
        profile,
        scratch_root,
        isolate_exe,
    })?;

    let item_id = args
        .get("item-path")
        .map(str::to_string)
        .unwrap_or_else(|| format!("synthetic:{total_bytes}-bytes"));
    let payload = json!({
        "mode": "probe",
        "profile": profile.as_str(),
        "isolated_verify_unpack": isolate,
        "probe": measurement,
    });
    Ok((item_id, payload))
}

/// Internal child mode for `--probe-isolate`: runs one `verify` or `unpack`
/// stage in this fresh process and prints its measurement as one JSON line.
fn run_probe_stage_child(args: &Args) -> Result<(), BoxError> {
    let stage = ProbeStage::parse(args.require("probe-stage")?)?;
    let encoded_path = PathBuf::from(args.require("probe-encoded-path")?);
    let scratch_dir = PathBuf::from(args.require("probe-stage-scratch")?);
    let interval_ms: u64 = parse_or(args, "probe-sample-interval-ms", 50)?;
    let mut measurement = run_single_stage(
        stage,
        &encoded_path,
        &scratch_dir,
        Duration::from_millis(interval_ms),
    )?;
    measurement.measurement.isolated = true;
    // Not getrusage(RUSAGE_SELF).max_rss: a child started with
    // posix_spawn/vfork (as std::process::Command does on Linux) inherits its
    // parent's high-water mark into that counter at exec (review round 1,
    // R1-05). The stage's own kernel high-water mark, reset when the stage
    // began in this fresh process, is the child's real peak.
    measurement.measurement.process_lifetime_peak_rss_bytes =
        Some(measurement.measurement.memory.peak_rss_bytes);
    println!("{}", serde_json::to_string(&measurement)?);
    Ok(())
}

fn build_random_access_policy(args: &Args) -> Result<RandomAccessPolicy, BoxError> {
    let mut policy = RandomAccessPolicy::default();
    if let Some(value) = args.get("coalesce-gap-bytes") {
        policy.coalesce_gap_bytes = value
            .parse()
            .map_err(|e| format!("--coalesce-gap-bytes: {e}"))?;
    }
    if let Some(value) = args.get("max-dependency-chunks") {
        policy.max_dependency_chunks = value
            .parse()
            .map_err(|e| format!("--max-dependency-chunks: {e}"))?;
    }
    if let Some(value) = args.get("max-total-bytes-fetched") {
        policy.max_total_bytes_fetched = value
            .parse()
            .map_err(|e| format!("--max-total-bytes-fetched: {e}"))?;
    }
    if let Some(value) = args.get("max-individual-range-bytes") {
        policy.max_individual_range_bytes = value
            .parse()
            .map_err(|e| format!("--max-individual-range-bytes: {e}"))?;
    }
    Ok(policy)
}

/// Parses `--{key}` as `T` if present, otherwise `default`.
fn parse_or<T: std::str::FromStr>(args: &Args, key: &str, default: T) -> Result<T, BoxError>
where
    T::Err: std::fmt::Display,
{
    match args.get(key) {
        Some(value) => value.parse().map_err(|e| format!("--{key}: {e}").into()),
        None => Ok(default),
    }
}

fn parse_profile(name: &str) -> Result<CompressionProfile, BoxError> {
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
