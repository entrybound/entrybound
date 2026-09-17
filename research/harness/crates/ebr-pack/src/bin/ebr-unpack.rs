//! `ebr-unpack` binary: full verify+extract of an already-encoded archive
//! file into a scratch directory, with peak scratch-directory byte usage
//! sampled while extraction runs. Standalone -- independent of a fresh
//! `ebr-pack` run, so it can be pointed at any archive file (typically one
//! `ebr-pack --archive-out <path>` produced). See `README.md`.
//!
//! ```text
//! ebr-unpack --archive-path <file> --experiment-id <id> --env-name <name>
//!            [--layout indexed|stream] [--encrypted true|false]
//!            [--unlock-password-file <path>] [--scratch-dir <dir>]
//!            [--keep-scratch true|false] [--run-id <id>] [--out <path>]
//! ```

use ebr_common::cli::Args;
use ebr_common::results::ResultWriter;
use ebr_common::runenv::{EnvIndex, RunContext};
use ebr_pack::unpack::{UnpackRequest, unpack_measure};
use ebr_pack::{layout_str, parse_bool_flag, parse_layout};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("ebr-unpack: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse(std::env::args().skip(1))?;
    let archive_path = PathBuf::from(args.require("archive-path")?);
    let experiment_id = args.require("experiment-id")?.to_string();
    let env_name = args.require("env-name")?.to_string();
    let layout = parse_layout(args.get("layout").unwrap_or("indexed"))?;
    let encrypted = parse_bool_flag(args.get("encrypted"), false)?;
    let keep_scratch = parse_bool_flag(args.get("keep-scratch"), false)?;
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

    let scratch_dir = match args.get("scratch-dir") {
        Some(path) => PathBuf::from(path),
        None => std::env::temp_dir().join(format!("ebr-unpack-{}", context.run_id)),
    };

    let request = UnpackRequest {
        layout,
        encrypted,
        password,
    };
    let measurement = unpack_measure(&archive_path, &scratch_dir, &request)?;

    let mut payload = serde_json::to_value(&measurement)?;
    if let serde_json::Value::Object(map) = &mut payload {
        map.insert("layout".to_owned(), serde_json::Value::String(layout_str(layout).to_owned()));
        map.insert("encrypted".to_owned(), serde_json::Value::Bool(encrypted));
        map.insert(
            "scratch_dir".to_owned(),
            serde_json::Value::String(scratch_dir.to_string_lossy().into_owned()),
        );
    }
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

    if !keep_scratch {
        std::fs::remove_dir_all(&scratch_dir).ok();
    }
    Ok(())
}
