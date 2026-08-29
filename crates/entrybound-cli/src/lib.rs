use std::env;
use std::ffi::OsString;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use entrybound::archive::{
    ConfinementMode, ExtractionPolicy, PackOptions, default_pack_output,
    default_unpack_destination, inspect, list, pack_directory, unpack,
};
use entrybound::diagnostics::{Diagnostic, OutcomeClass, ReasonCode, Result};
use entrybound::eam::{ArchiveRole, EntryKind, Layout};
use entrybound::ecf::{IndexStatus, open, verify};

const HELP: &str = "\
Entrybound (experimental native bootstrap)\n\
\n\
Usage:\n\
  ebound pack <input-directory> [output.eb]\n\
  ebound unpack <archive.eb> [destination]\n\
  ebound list <archive.eb>\n\
  ebound inspect <archive.eb>\n\
  ebound verify <archive.eb>\n\
\n\
This build supports unencrypted Complete INDEXED archives with directories,\n\
regular files, fixed 1 MiB chunking, and STORE only.\n";

fn run(arguments: impl IntoIterator<Item = OsString>) -> Result<()> {
    let mut arguments = arguments.into_iter();
    let _program = arguments.next();
    let Some(command) = arguments.next() else {
        print!("{HELP}");
        return Ok(());
    };
    let command = command
        .to_str()
        .ok_or_else(|| usage("command is not valid UTF-8"))?;
    match command {
        "help" | "--help" | "-h" => {
            ensure_no_more(arguments)?;
            print!("{HELP}");
            Ok(())
        }
        "pack" => command_pack(arguments.collect()),
        "unpack" => command_unpack(arguments.collect()),
        "list" => command_list(arguments.collect()),
        "inspect" => command_inspect(arguments.collect()),
        "verify" => command_verify(arguments.collect()),
        other => Err(Diagnostic::new(
            OutcomeClass::Unsupported,
            ReasonCode::CommandNotImplemented,
            format!("command '{other}' is not implemented"),
        )),
    }
}

fn command_pack(arguments: Vec<OsString>) -> Result<()> {
    if !(1..=2).contains(&arguments.len()) {
        return Err(usage("pack requires <input-directory> [output.eb]"));
    }
    let input = PathBuf::from(&arguments[0]);
    let output = arguments
        .get(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| default_pack_output(&input));
    let encoded = pack_directory(&input, PackOptions::default())?;
    write_exclusive(&output, &encoded.bytes)?;
    println!(
        "OK packed {} entries into {}",
        encoded.archive.entry_set.len(),
        output.display()
    );
    println!("LAI {}", encoded.identities.lai.0);
    println!("PCR {}", encoded.identities.pcr.0);
    println!("AUX {}", encoded.identities.aux.0);
    println!("PCI {}", encoded.identities.pci.0);
    Ok(())
}

fn command_unpack(arguments: Vec<OsString>) -> Result<()> {
    if !(1..=2).contains(&arguments.len()) {
        return Err(usage("unpack requires <archive.eb> [destination]"));
    }
    let archive = PathBuf::from(&arguments[0]);
    let destination = arguments
        .get(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| default_unpack_destination(&archive));
    let bytes = read(&archive)?;
    let report = unpack(&bytes, &destination, ExtractionPolicy::default())?;
    println!(
        "OK unpacked {} entries and {} logical bytes into {}",
        report.entries_created,
        report.logical_bytes_written,
        destination.display()
    );
    println!(
        "confinement: {}",
        match report.confinement {
            ConfinementMode::KernelEnforced => "kernel-enforced capability-relative",
            ConfinementMode::WeakerReported => "weaker platform mode (reported)",
        }
    );
    if report.metadata_not_restored.is_empty() {
        println!("metadata restored: all represented and platform-supported items");
    } else {
        println!(
            "metadata limitations: {}",
            report.metadata_not_restored.join("; ")
        );
    }
    Ok(())
}

fn command_list(arguments: Vec<OsString>) -> Result<()> {
    let archive = one_path("list", arguments)?;
    let bytes = read(&archive)?;
    let opened = open(&bytes)?;
    for entry in list(&opened.archive)? {
        let kind = match entry.kind {
            EntryKind::Directory => "directory",
            EntryKind::File => "file",
        };
        println!("{kind}\t{}", entry.path);
    }
    Ok(())
}

fn command_inspect(arguments: Vec<OsString>) -> Result<()> {
    let archive = one_path("inspect", arguments)?;
    let bytes = read(&archive)?;
    let opened = open(&bytes)?;
    let view = inspect(&opened)?;
    println!("format: {}", view.format_namespace);
    println!("version: {}.{}", view.version.major, view.version.minor);
    println!(
        "layout: {}",
        match view.layout {
            Layout::Indexed => "INDEXED",
        }
    );
    println!(
        "archive role: {}",
        match view.role {
            ArchiveRole::Complete => "Complete",
        }
    );
    println!("entry count: {}", view.entry_count);
    println!("total logical bytes: {}", view.total_logical_bytes);
    println!(
        "features: incompat={:#x}, read-only-compat={:#x}, compat={:#x}",
        view.features.incompat, view.features.read_only_compat, view.features.compat
    );
    println!("planner: {}", view.planner_id);
    println!("chunker: {}", view.chunker_id);
    for plan in view.plans {
        println!("transform plan: {} (codec {})", plan.identifier, plan.codec);
    }
    println!("LAI: {}", view.identities.lai.0);
    println!("PCR: {}", view.identities.pcr.0);
    println!("AUX: {}", view.identities.aux.0);
    println!("PCI: {}", view.identities.pci.0);
    println!("index: {}", index_status(view.index_status));
    println!("fidelity captured: {}", view.fidelity.captured.join(", "));
    println!(
        "fidelity unavailable: {}",
        view.fidelity
            .unavailable
            .iter()
            .map(|issue| issue.class.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );
    let budget = view.declared_resources;
    println!(
        "declared resources: entries={}, total-bytes={}, max-entry-bytes={}, chunks={}, path-depth={}, metadata-bytes={}",
        budget.entry_count,
        budget.total_logical_bytes,
        budget.max_single_entry_logical_bytes,
        budget.chunk_count,
        budget.max_path_depth,
        budget.max_metadata_bytes
    );
    println!(
        "decode requirements: STORE; window={}, working-set={}, flags={:#x}, kdf-cost={}",
        view.decode_requirements.window_bytes,
        view.decode_requirements.working_set_bytes,
        view.decode_requirements.flags,
        budget.max_key_derivation_cost
    );
    Ok(())
}

fn command_verify(arguments: Vec<OsString>) -> Result<()> {
    let archive = one_path("verify", arguments)?;
    let bytes = read(&archive)?;
    let report = verify(&bytes)?;
    println!(
        "OK verified canonical structure, section integrity, semantic invariants, Chunk/content integrity, Entry identities, LAI, PCR, AUX, and exact-byte PCI"
    );
    println!("index: {}", index_status(report.index_status));
    println!("LAI {}", report.identities.lai.0);
    println!("PCR {}", report.identities.pcr.0);
    println!("AUX {}", report.identities.aux.0);
    println!("PCI {}", report.identities.pci.0);
    Ok(())
}

fn index_status(status: IndexStatus) -> &'static str {
    match status {
        IndexStatus::PresentValid => "present and valid",
        IndexStatus::RebuiltAbsent => "absent; rebuilt from authoritative CHUNK_DATA",
        IndexStatus::RebuiltInvalid => "invalid; rebuilt from authoritative CHUNK_DATA",
    }
}

fn one_path(command: &str, arguments: Vec<OsString>) -> Result<PathBuf> {
    if arguments.len() != 1 {
        return Err(usage(format!("{command} requires <archive.eb>")));
    }
    Ok(PathBuf::from(&arguments[0]))
}

fn ensure_no_more(mut arguments: impl Iterator<Item = OsString>) -> Result<()> {
    if arguments.next().is_some() {
        return Err(usage("help does not accept additional arguments"));
    }
    Ok(())
}

fn read(path: &Path) -> Result<Vec<u8>> {
    std::fs::read(path).map_err(|error| {
        Diagnostic::new(
            OutcomeClass::PolicyRefused,
            ReasonCode::Io,
            format!("cannot read '{}': {error}", path.display()),
        )
    })
}

fn write_exclusive(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| {
            Diagnostic::new(
                OutcomeClass::PolicyRefused,
                ReasonCode::Io,
                format!("cannot create output '{}': {error}", path.display()),
            )
        })?;
    file.write_all(bytes).map_err(|error| {
        Diagnostic::new(
            OutcomeClass::PolicyRefused,
            ReasonCode::Io,
            format!("cannot write output '{}': {error}", path.display()),
        )
    })
}

fn usage(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::PolicyRefused,
        ReasonCode::CommandUsage,
        detail,
    )
}

/// Runs the shared Entrybound CLI implementation for either executable name.
#[must_use]
pub fn main_entry() -> ExitCode {
    match run(env::args_os()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(match error.class() {
                OutcomeClass::Ok => 0,
                OutcomeClass::Unsupported => 2,
                OutcomeClass::Truncated => 3,
                OutcomeClass::Corrupt => 4,
                OutcomeClass::Nonconforming => 5,
                OutcomeClass::PolicyRefused => 6,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::run;
    use entrybound::diagnostics::ReasonCode;
    use std::ffi::OsString;

    fn args(values: &[&str]) -> Vec<OsString> {
        values.iter().map(OsString::from).collect()
    }

    #[test]
    fn help_is_available() {
        assert!(run(args(&["ebound"])).is_ok());
    }

    #[test]
    fn future_commands_fail_explicitly() {
        let error = run(args(&["ebound", "convert"])).unwrap_err();
        assert_eq!(error.code(), ReasonCode::CommandNotImplemented);
    }
}
