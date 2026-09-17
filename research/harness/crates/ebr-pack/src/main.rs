//! `ebr-pack` binary: packs a directory (a corpus item's materialized path,
//! or any other directory) and reports one `ebr.harness.raw.v1` JSON row
//! carrying complete artifact bytes, exact section-level byte accounting,
//! logical/codec counts, planner id, per-phase wall-clock, and this
//! process's peak memory. See `README.md` for the full flag reference and
//! example `ebr` runner spec command templates.
//!
//! ```text
//! ebr-pack --item-path <dir> --experiment-id <id> --env-name <name>
//!          [--profile fast|balanced|dense|extreme] [--layout indexed|stream]
//!          [--stream-window <u64>] [--encrypt true|false]
//!          [--deterministic-check <N>] [--archive-out <path>]
//!          [--run-id <id>] [--out <path>]
//! ```

use ebr_common::cli::Args;
use ebr_common::hash::sha256_hex;
use ebr_common::results::ResultWriter;
use ebr_common::runenv::{EnvIndex, RunContext};
use ebr_pack::bytes::ByteAccounting;
use ebr_pack::counts::ArchiveCounts;
use ebr_pack::encrypt::CredentialSummary;
use ebr_pack::pack::{DeterminismCheck, PackRequest, PhaseSeconds};
use ebr_pack::{bytes, layout_str, parse_bool_flag, parse_layout, parse_profile};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("ebr-pack: {e}");
            ExitCode::FAILURE
        }
    }
}

#[derive(Debug, Serialize)]
struct PackRow<'a> {
    profile: &'static str,
    layout: &'static str,
    encrypted: bool,
    archive_path: String,
    artifact_bytes: u64,
    input_logical_bytes: u64,
    input_file_count: u64,
    verified: bool,
    phases: PhaseSeconds,
    writing_seconds: f64,
    byte_accounting: &'a ByteAccounting,
    counts: &'a ArchiveCounts,
    peak_rss_bytes: u64,
    peak_rss_is_true_peak: bool,
    peak_rss_method: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    credentials: Option<CredentialSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    credentials_password_file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    determinism: Option<DeterminismCheck>,
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse(std::env::args().skip(1))?;
    let item_path = PathBuf::from(args.require("item-path")?);
    let experiment_id = args.require("experiment-id")?.to_string();
    let env_name = args.require("env-name")?.to_string();
    let profile = parse_profile(args.get("profile").unwrap_or("balanced"))?;
    let layout = parse_layout(args.get("layout").unwrap_or("indexed"))?;
    let stream_window = args
        .get("stream-window")
        .map(|v| v.parse::<u64>())
        .transpose()
        .map_err(|e| format!("--stream-window must be a u64: {e}"))?;
    let encrypt = parse_bool_flag(args.get("encrypt"), false)?;
    let deterministic_check = args
        .get("deterministic-check")
        .map(|v| v.parse::<u32>())
        .transpose()
        .map_err(|e| format!("--deterministic-check must be a u32: {e}"))?;

    let repo_root = ebr_common::discover_repo_root(Path::new(env!("CARGO_MANIFEST_DIR")))
        .ok_or("could not locate the entrybound repo root from CARGO_MANIFEST_DIR")?;
    let env_id = EnvIndex::load(&repo_root)?.resolve(&env_name)?.to_string();
    let context = match args.get("run-id") {
        Some(run_id) => RunContext::with_run_id(experiment_id, run_id.to_string(), env_id),
        None => RunContext::new(experiment_id, env_id),
    };

    let request = PackRequest {
        profile,
        layout,
        stream_window,
        encrypt,
    };

    let outcome = ebr_pack::pack::pack_once(&item_path, &request)?;
    let canonical_sha256 = sha256_hex(&outcome.bytes);

    let determinism = match deterministic_check {
        Some(runs) if runs > 1 => {
            let mut distinct_sha256 = vec![canonical_sha256.clone()];
            for _ in 1..runs {
                let repeat = ebr_pack::pack::pack_once(&item_path, &request)?;
                let sha256 = sha256_hex(&repeat.bytes);
                if !distinct_sha256.contains(&sha256) {
                    distinct_sha256.push(sha256);
                }
            }
            let check = DeterminismCheck {
                requested_runs: runs,
                completed_runs: runs,
                all_identical: distinct_sha256.len() == 1,
                sha256: canonical_sha256.clone(),
                distinct_sha256,
            };
            if !check.all_identical {
                return Err(format!(
                    "--deterministic-check {runs}: {} distinct SHA-256 hashes across {runs} \
                     repeated packs of the same input/profile/layout, expected exactly 1: {:?}",
                    check.distinct_sha256.len(),
                    check.distinct_sha256
                )
                .into());
            }
            Some(check)
        }
        _ => None,
    };

    let archive_path = match args.get("archive-out") {
        Some(path) => PathBuf::from(path),
        None => std::env::temp_dir().join(format!("ebr-pack-{}.ecf", context.run_id)),
    };
    if let Some(parent) = archive_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let writing_sw = ebr_common::timing::Stopwatch::start();
    std::fs::write(&archive_path, &outcome.bytes)?;
    let writing_seconds = writing_sw.elapsed_secs_f64();

    let mut credentials_password_file: Option<String> = None;
    if let Some(creds) = &outcome.credentials {
        match creds.persist_password(&archive_path) {
            Ok(sidecar) => credentials_password_file = Some(sidecar.to_string_lossy().into_owned()),
            Err(e) => eprintln!(
                "ebr-pack: warning: could not persist test password sidecar: {e}"
            ),
        }
    }

    let byte_accounting = if encrypt {
        let password = outcome
            .credentials
            .as_ref()
            .expect("encrypted pack always returns credentials")
            .password
            .clone();
        bytes::encrypted_byte_accounting(&outcome.bytes, &password)?
    } else {
        match layout {
            entrybound::eam::Layout::Indexed => {
                bytes::indexed_byte_accounting(&outcome.bytes, &outcome.counts.plans)?
            }
            entrybound::eam::Layout::Stream => bytes::stream_byte_accounting(&outcome.bytes)?,
        }
    };

    let peak_rss = ebr_common::measure::peak_memory()
        .map_err(|e| format!("measuring peak memory: {e}"))?;

    let row = PackRow {
        profile: profile.as_str(),
        layout: layout_str(layout),
        encrypted: encrypt,
        archive_path: archive_path.to_string_lossy().into_owned(),
        artifact_bytes: outcome.bytes.len() as u64,
        input_logical_bytes: outcome.input_logical_bytes,
        input_file_count: outcome.input_file_count,
        verified: outcome.verified,
        phases: outcome.phases,
        writing_seconds,
        byte_accounting: &byte_accounting,
        counts: &outcome.counts,
        peak_rss_bytes: peak_rss.bytes,
        peak_rss_is_true_peak: peak_rss.is_true_peak,
        peak_rss_method: peak_rss.method,
        credentials: outcome.credentials.as_ref().map(|c| c.summary()),
        credentials_password_file,
        determinism,
    };
    let payload = serde_json::to_value(&row)?;
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
