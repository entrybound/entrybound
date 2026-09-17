//! `ebr-inspect-bytes` binary: exact section-level byte accounting (see
//! `ebr_pack::bytes`) plus logical/codec counts (see `ebr_pack::counts`) for
//! an already-encoded archive file. Standalone -- independent of a fresh
//! `ebr-pack` run, so it can be pointed at any archive file, including one
//! this crate did not produce, to double-check its own accounting. See
//! `README.md`.
//!
//! ```text
//! ebr-inspect-bytes --archive-path <file> --experiment-id <id> --env-name <name>
//!                    [--layout indexed|stream] [--encrypted true|false]
//!                    [--unlock-password-file <path>] [--run-id <id>] [--out <path>]
//! ```

use ebr_common::cli::Args;
use ebr_common::results::ResultWriter;
use ebr_common::runenv::{EnvIndex, RunContext};
use ebr_pack::{bytes, counts, layout_str, parse_bool_flag, parse_layout};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("ebr-inspect-bytes: {e}");
            ExitCode::FAILURE
        }
    }
}

#[derive(Debug, Serialize)]
struct InspectRow<'a> {
    layout: &'static str,
    encrypted: bool,
    artifact_bytes: u64,
    verified: bool,
    byte_accounting: &'a bytes::ByteAccounting,
    counts: &'a counts::ArchiveCounts,
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse(std::env::args().skip(1))?;
    let archive_path = PathBuf::from(args.require("archive-path")?);
    let experiment_id = args.require("experiment-id")?.to_string();
    let env_name = args.require("env-name")?.to_string();
    let layout = parse_layout(args.get("layout").unwrap_or("indexed"))?;
    let encrypted = parse_bool_flag(args.get("encrypted"), false)?;
    let password = args
        .get("unlock-password-file")
        .map(std::fs::read)
        .transpose()
        .map_err(|e| format!("reading --unlock-password-file: {e}"))?;

    let repo_root = ebr_common::discover_repo_root(Path::new(env!("CARGO_MANIFEST_DIR")))
        .ok_or("could not locate the entrybound repo root from CARGO_MANIFEST_DIR")?;
    let env_id = EnvIndex::load(&repo_root)?.resolve(&env_name)?.to_string();
    let context = match args.get("run-id") {
        Some(run_id) => RunContext::with_run_id(experiment_id, run_id.to_string(), env_id),
        None => RunContext::new(experiment_id, env_id),
    };

    let archive_bytes = std::fs::read(&archive_path)?;

    let (counts, verified) =
        counts::from_bytes(&archive_bytes, layout, encrypted, password.as_deref())?;

    let byte_accounting = if encrypted {
        let password = password
            .as_deref()
            .ok_or("archive is encrypted but no --unlock-password-file was given")?;
        bytes::encrypted_byte_accounting(&archive_bytes, password)?
    } else {
        match layout {
            entrybound::eam::Layout::Indexed => {
                bytes::indexed_byte_accounting(&archive_bytes, &counts.plans)?
            }
            entrybound::eam::Layout::Stream => bytes::stream_byte_accounting(&archive_bytes)?,
        }
    };

    let row = InspectRow {
        layout: layout_str(layout),
        encrypted,
        artifact_bytes: archive_bytes.len() as u64,
        verified,
        byte_accounting: &byte_accounting,
        counts: &counts,
    };
    let payload = serde_json::to_value(&row)?;
    let item_id = archive_path.to_string_lossy().into_owned();

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
