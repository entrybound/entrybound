use std::env;
use std::ffi::{OsStr, OsString};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use entrybound::archive::{
    ConfinementMode, ExtractionPolicy, PackOptions, default_pack_output,
    default_unpack_destination, explain as compression_explain, inspect, list, plan_directory,
    unpack, unpack_stream,
};
use entrybound::diagnostics::{Diagnostic, OutcomeClass, ReasonCode, Result};
use entrybound::eam::{ArchiveRole, EntryKind, Layout};
use entrybound::ecf::{
    IndexStatus, OpenedArchive, SequentialLimits, StreamContentPolicy, StreamReport, StreamWindow,
    StreamWriteOptions, WriteOptions, bootstrap_sequential_limits, encode, encode_stream, open,
    open_stream_with_limits, peek_layout, verify,
};
use entrybound::planner::CompressionProfile;

const HELP: &str = "\
Entrybound (experimental native bootstrap)\n\
\n\
Usage:\n\
  ebound pack <input-directory> [output.eb|-] [--layout indexed|stream]\n\
                                [--stream-window <n>|auto]\n\
                                [--profile fast|balanced|dense|extreme]\n\
  ebound unpack <archive.eb|-> [destination]\n\
  ebound list <archive.eb|->\n\
  ebound inspect <archive.eb|->\n\
  ebound verify <archive.eb|->\n\
  ebound explain <archive.eb|->\n\
\n\
This build supports unencrypted Complete archives in two physical layouts,\n\
INDEXED and STREAM, with directories, regular files, normalized\n\
content-defined chunking, archive-wide exact dedup, and per-unique-Chunk\n\
STORE, Zstandard, LZ4, and LZMA2 planning, reversible structural transforms,\n\
verified DEFLATE reconstruction, opportunistic byte-exact JPEG/JPEG XL\n\
whole-object reconstruction, optional shared dictionaries, and explicitly\n\
bounded ChunkGroups in dense/extreme archives.\n\
\n\
Both layouts encode the same archive model: they produce identical LAI, PCR,\n\
and AUX and differ only in PCI, physical organization, and access capability.\n\
STREAM writes without seeking and reads without seeking, carries no Index, and\n\
cannot resolve one entry without a full sequential pass.\n\
\n\
Use `-` for standard input or standard output. When archive bytes go to\n\
standard output, all status output goes to standard error.\n\
The default creation profile is balanced; decoding is self-describing and\n\
never requires a profile.\n";

const PACK_HELP: &str = "\
Usage: ebound pack <input-directory> [output.eb|-] [--layout indexed|stream]\n\
                                     [--stream-window <n>|auto]\n\
                                     [--profile fast|balanced|dense|extreme]\n\
\n\
Creates a deterministic native .eb archive. The default profile is balanced.\n\
Profiles are creation-time policy only; archives record their TransformPlans.\n\
JPEG reconstruction is opportunistic, bounded, and committed only after an\n\
exact byte round trip.\n\
\n\
--layout selects the physical organization. A regular file output defaults to\n\
indexed; an output of `-` defaults to stream. Both layouts carry the same\n\
archive model and the same .eb extension.\n\
\n\
--stream-window declares how far a sequential reference may depend on an\n\
already emitted unique Chunk. The default is 0, which refuses to create any\n\
cross-object historical dependency. `auto` accepts whatever the selected\n\
sequential organization requires and declares exactly that minimum. Packing\n\
fails with a typed diagnostic rather than silently raising a window you asked\n\
for. Shared dictionaries are declared before use and do not themselves consume\n\
the window; historical exact-Chunk references and bounded-lookback ChunkGroups\n\
may require a non-zero window.\n";

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
        "explain" => command_explain(arguments.collect()),
        other => Err(Diagnostic::new(
            OutcomeClass::Unsupported,
            ReasonCode::CommandNotImplemented,
            format!("command '{other}' is not implemented"),
        )),
    }
}

// ---------------------------------------------------------------------------
// Sources and status routing
// ---------------------------------------------------------------------------

/// Where an archive's bytes come from.
enum Source {
    /// Standard input, which is sequential and cannot be replayed.
    Stdin,
    Path(PathBuf),
}

impl Source {
    fn parse(value: &OsStr) -> Self {
        if value == OsStr::new("-") {
            Self::Stdin
        } else {
            Self::Path(PathBuf::from(value))
        }
    }
}

/// Where an archive's bytes go.
enum Destination {
    Stdout,
    Path(PathBuf),
}

/// Status output goes to standard error whenever archive bytes claim stdout.
struct Status {
    to_stderr: bool,
}

impl Status {
    fn line(&self, text: impl AsRef<str>) {
        if self.to_stderr {
            eprintln!("{}", text.as_ref());
        } else {
            println!("{}", text.as_ref());
        }
    }
}

/// A fully verified archive plus whatever the sequential pass established.
struct Loaded {
    opened: OpenedArchive,
    stream: Option<StreamReport>,
}

/// Reads only the fixed preamble to learn which reader an archive needs.
///
/// A STREAM archive that happens to sit in a seekable file is still a STREAM
/// archive, and a large INDEXED archive should not be read twice to find out
/// that it is one.
fn path_layout(path: &Path) -> Result<Layout> {
    let mut preamble = [0_u8; 256];
    let mut file = File::open(path).map_err(|error| read_error(path, &error))?;
    let mut filled = 0;
    while filled < preamble.len() {
        match std::io::Read::read(&mut file, &mut preamble[filled..]) {
            Ok(0) => break,
            Ok(count) => filled += count,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
            Err(error) => return Err(read_error(path, &error)),
        }
    }
    // A container too short or too damaged to classify is left to the
    // random-access reader, which owns the stable diagnostic for it.
    Ok(peek_layout(&preamble[..filled]).unwrap_or(Layout::Indexed))
}

/// Opens and fully verifies an archive, choosing the reader from the archive's
/// declared layout rather than from whether the source happens to be seekable.
fn load(source: &Source, content: StreamContentPolicy) -> Result<Loaded> {
    let limits = SequentialLimits {
        content,
        ..bootstrap_sequential_limits()
    };
    match source {
        Source::Stdin => {
            let sequential = open_stream_with_limits(std::io::stdin().lock(), limits)?;
            Ok(Loaded {
                opened: sequential.opened,
                stream: Some(sequential.stream),
            })
        }
        Source::Path(path) if path_layout(path)? == Layout::Stream => {
            let file = File::open(path).map_err(|error| read_error(path, &error))?;
            let sequential = open_stream_with_limits(file, limits)?;
            Ok(Loaded {
                opened: sequential.opened,
                stream: Some(sequential.stream),
            })
        }
        Source::Path(path) => Ok(Loaded {
            opened: open(&read(path)?)?,
            stream: None,
        }),
    }
}

// ---------------------------------------------------------------------------
// pack
// ---------------------------------------------------------------------------

fn command_pack(arguments: Vec<OsString>) -> Result<()> {
    if arguments.len() == 1 && matches!(arguments[0].to_str(), Some("--help" | "-h")) {
        print!("{PACK_HELP}");
        return Ok(());
    }
    let mut positionals = Vec::new();
    let mut profile = None;
    let mut layout = None;
    let mut window = None;
    let mut cursor = 0;
    while cursor < arguments.len() {
        let value = &arguments[cursor];
        if value == "--profile" {
            if profile.is_some() {
                return Err(usage("pack accepts --profile only once"));
            }
            cursor += 1;
            let selected = arguments
                .get(cursor)
                .and_then(|value| value.to_str())
                .ok_or_else(|| usage("--profile requires a UTF-8 profile name"))?;
            profile = Some(selected.parse::<CompressionProfile>()?);
        } else if value == "--layout" {
            if layout.is_some() {
                return Err(usage("pack accepts --layout only once"));
            }
            cursor += 1;
            layout = Some(
                match arguments.get(cursor).and_then(|value| value.to_str()) {
                    Some("indexed") => Layout::Indexed,
                    Some("stream") => Layout::Stream,
                    _ => return Err(usage("--layout requires 'indexed' or 'stream'")),
                },
            );
        } else if value == "--stream-window" {
            if window.is_some() {
                return Err(usage("pack accepts --stream-window only once"));
            }
            cursor += 1;
            window = Some(
                match arguments.get(cursor).and_then(|value| value.to_str()) {
                    Some("auto") => StreamWindow::Auto,
                    Some(number) => StreamWindow::Ceiling(number.parse::<u64>().map_err(|_| {
                        usage("--stream-window requires a non-negative integer or 'auto'")
                    })?),
                    None => {
                        return Err(usage(
                            "--stream-window requires a non-negative integer or 'auto'",
                        ));
                    }
                },
            );
        } else if value.to_string_lossy().starts_with("--") {
            return Err(usage(format!(
                "pack does not recognize option '{}'",
                value.to_string_lossy()
            )));
        } else {
            positionals.push(value.clone());
        }
        cursor += 1;
    }
    if !(1..=2).contains(&positionals.len()) {
        return Err(usage(
            "pack requires <input-directory> [output.eb|-] and optional --layout, \
             --stream-window, and --profile",
        ));
    }
    let input = PathBuf::from(&positionals[0]);
    let destination = match positionals.get(1) {
        Some(value) if value == OsStr::new("-") => Destination::Stdout,
        Some(value) => Destination::Path(PathBuf::from(value)),
        None => Destination::Path(default_pack_output(&input)),
    };
    // A regular file defaults to the random-access layout; standard output
    // defaults to the layout that never needs to seek.
    let layout = layout.unwrap_or(match destination {
        Destination::Stdout => Layout::Stream,
        Destination::Path(_) => Layout::Indexed,
    });
    if window.is_some() && layout != Layout::Stream {
        return Err(usage("--stream-window applies only to --layout stream"));
    }
    let status = Status {
        to_stderr: matches!(destination, Destination::Stdout),
    };
    let options = PackOptions {
        profile: profile.unwrap_or_default(),
        ..PackOptions::default()
    };

    // Planning happens before any output object is created, so a plan that
    // cannot be represented under the requested constraints never leaves a
    // partial archive behind.
    let planned = plan_directory(&input, options)?;

    match layout {
        Layout::Stream => {
            let stream_options = StreamWriteOptions {
                window: window.unwrap_or_default(),
                ..StreamWriteOptions::default()
            };
            let summary = match &destination {
                Destination::Stdout => {
                    let stdout = std::io::stdout();
                    let mut handle = stdout.lock();
                    let summary = encode_stream(&planned, stream_options, &mut handle)?;
                    handle
                        .flush()
                        .map_err(|error| io_error("flush standard output", &error))?;
                    summary
                }
                Destination::Path(path) => {
                    let mut file = create_exclusive(path)?;
                    match encode_stream(&planned, stream_options, &mut file).and_then(|summary| {
                        file.flush()
                            .map_err(|error| io_error("flush archive output", &error))?;
                        Ok(summary)
                    }) {
                        Ok(summary) => summary,
                        Err(error) => {
                            drop(file);
                            let _ = std::fs::remove_file(path);
                            return Err(error);
                        }
                    }
                }
            };
            status.line(format!(
                "OK packed {} entries into {}",
                summary.archive.entry_set.len(),
                describe(&destination)
            ));
            status.line("layout STREAM");
            status.line(format!("stream dedup window {}", summary.dedup_window));
            status.line(format!("budget declared {}", summary.budget_declared));
            status.line(format!(
                "chunk frames {} manifest records {}",
                summary.chunk_frames, summary.manifest_records
            ));
            print_identities(&status, &summary.identities);
            status.line(format!("planner {}", summary.archive.descriptor.planner_id));
        }
        Layout::Indexed => {
            let encoded = encode(
                &planned,
                WriteOptions {
                    include_index: options.include_index,
                },
            )?;
            match &destination {
                Destination::Stdout => {
                    let stdout = std::io::stdout();
                    let mut handle = stdout.lock();
                    handle
                        .write_all(&encoded.bytes)
                        .map_err(|error| io_error("write standard output", &error))?;
                    handle
                        .flush()
                        .map_err(|error| io_error("flush standard output", &error))?;
                }
                Destination::Path(path) => {
                    let mut file = create_exclusive(path)?;
                    file.write_all(&encoded.bytes)
                        .map_err(|error| io_error("write archive output", &error))?;
                }
            }
            status.line(format!(
                "OK packed {} entries into {}",
                encoded.archive.entry_set.len(),
                describe(&destination)
            ));
            status.line("layout INDEXED");
            print_identities(&status, &encoded.identities);
            status.line(format!("planner {}", encoded.archive.descriptor.planner_id));
        }
    }
    Ok(())
}

fn print_identities(status: &Status, identities: &entrybound::identity::IdentitySet) {
    status.line(format!("LAI {}", identities.lai.0));
    status.line(format!("PCR {}", identities.pcr.0));
    status.line(format!("AUX {}", identities.aux.0));
    status.line(format!("PCI {}", identities.pci.0));
}

fn describe(destination: &Destination) -> String {
    match destination {
        Destination::Stdout => "standard output".to_owned(),
        Destination::Path(path) => path.display().to_string(),
    }
}

// ---------------------------------------------------------------------------
// unpack
// ---------------------------------------------------------------------------

fn command_unpack(arguments: Vec<OsString>) -> Result<()> {
    if !(1..=2).contains(&arguments.len()) {
        return Err(usage("unpack requires <archive.eb|-> [destination]"));
    }
    let source = Source::parse(&arguments[0]);
    let destination = match arguments.get(1) {
        Some(value) => PathBuf::from(value),
        None => match &source {
            Source::Stdin => {
                return Err(usage(
                    "unpack from standard input requires an explicit destination",
                ));
            }
            Source::Path(path) => default_unpack_destination(path),
        },
    };
    let status = Status { to_stderr: false };
    let (report, stream) = match &source {
        Source::Stdin => {
            let (report, stream) = unpack_stream(
                std::io::stdin().lock(),
                &destination,
                ExtractionPolicy::default(),
                bootstrap_sequential_limits(),
            )?;
            (report, Some(stream))
        }
        Source::Path(path) if path_layout(path)? == Layout::Stream => {
            let file = File::open(path).map_err(|error| read_error(path, &error))?;
            let (report, stream) = unpack_stream(
                file,
                &destination,
                ExtractionPolicy::default(),
                bootstrap_sequential_limits(),
            )?;
            (report, Some(stream))
        }
        Source::Path(path) => (
            unpack(&read(path)?, &destination, ExtractionPolicy::default())?,
            None,
        ),
    };
    status.line(format!(
        "OK unpacked {} entries and {} logical bytes into {}",
        report.entries_created,
        report.logical_bytes_written,
        destination.display()
    ));
    if let Some(stream) = stream {
        status.line("layout: STREAM (one complete sequential pass)");
        status.line(format!(
            "staging: peak-resident-bytes={}, spilled-bytes={}, peak-retained-chunks={}",
            stream.peak_resident_staging_bytes,
            stream.spilled_staging_bytes,
            stream.peak_retained_chunks
        ));
        status.line(
            "extraction: destination objects were created only after the complete archive verified",
        );
    }
    status.line(format!(
        "confinement: {}",
        match report.confinement {
            ConfinementMode::KernelEnforced => "kernel-enforced capability-relative",
            ConfinementMode::WeakerReported => "weaker platform mode (reported)",
        }
    ));
    if report.metadata_not_restored.is_empty() {
        status.line("metadata restored: all represented and platform-supported items");
    } else {
        status.line(format!(
            "metadata limitations: {}",
            report.metadata_not_restored.join("; ")
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// list, inspect, verify, explain
// ---------------------------------------------------------------------------

fn command_list(arguments: Vec<OsString>) -> Result<()> {
    let source = one_source("list", arguments)?;
    let loaded = load(&source, StreamContentPolicy::Verify)?;
    if loaded.stream.is_some() {
        eprintln!(
            "note: STREAM layout has no Index; this listing required a complete sequential pass"
        );
    }
    for entry in list(&loaded.opened.archive)? {
        let kind = match entry.kind {
            EntryKind::Directory => "directory",
            EntryKind::File => "file",
        };
        println!("{kind}\t{}", entry.path);
    }
    Ok(())
}

fn command_inspect(arguments: Vec<OsString>) -> Result<()> {
    let source = one_source("inspect", arguments)?;
    let loaded = load(&source, StreamContentPolicy::Verify)?;
    let view = inspect(&loaded.opened)?;
    println!("format: {}", view.format_namespace);
    println!("version: {}.{}", view.version.major, view.version.minor);
    println!("layout: {}", view.layout.as_str());
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
    println!(
        "codec/transform registry feature: {}",
        view.codec_transform_feature_present
    );
    println!(
        "reconstructive transform feature: {}",
        view.reconstructive_transform_feature_present
    );
    println!(
        "stream layout feature: {}",
        view.stream_layout_feature_present
    );
    println!("stream dedup window: {}", view.stream_dedup_window);
    println!(
        "producer budget declaration: {}",
        if view.budget_declared {
            "declared before the payload"
        } else {
            "not declared by the producer; caller policy alone bounded decoding, \
             and absence is not a claim of unlimited resources"
        }
    );
    println!(
        "random entry lookup: {}",
        if view.random_entry_lookup {
            "available"
        } else {
            "unavailable; entry lookup requires a complete sequential scan or a repack"
        }
    );
    println!("planner: {}", view.planner_id);
    println!("chunker: {}", view.chunker_id);
    println!(
        "chunks: unique={}, logical-references={}, min-bytes={}, average-bytes={}, max-bytes={}",
        view.chunks.unique_chunk_count,
        view.chunks.logical_chunk_references,
        view.chunks.minimum_chunk_bytes,
        view.chunks.average_chunk_bytes,
        view.chunks.maximum_chunk_bytes
    );
    println!(
        "cross-file compression: feature={}, dictionaries={}, dictionary-bytes={}, dictionary-backed-chunks={}",
        view.cross_file.feature_present,
        view.cross_file.dictionary_count,
        view.cross_file.dictionary_bytes,
        view.cross_file.dictionary_backed_chunks
    );
    println!(
        "chunk groups: count={}, max-lookback={}, worst-access-chunks={}, worst-access-bytes={}, all-independent={}",
        view.cross_file.chunk_group_count,
        view.cross_file.maximum_lookback,
        view.cross_file.worst_random_access_chunks,
        view.cross_file.worst_random_access_bytes,
        view.cross_file.every_chunk_independently_decodable
    );
    println!(
        "reconstruction: objects={}, bytes={}, chunks={}, transforms={}, max-intermediate-bytes={}",
        view.reconstruction.object_count,
        view.reconstruction.object_bytes,
        view.reconstruction.chunk_count,
        if view.reconstruction.transform_types.is_empty() {
            "none".to_owned()
        } else {
            view.reconstruction.transform_types.join(",")
        },
        view.reconstruction.maximum_intermediate_bytes
    );
    println!(
        "whole-object reconstruction: feature={}, regions={}, jpeg-regions={}, logical-bytes={}, jpeg-xl-bytes={}, stored-bytes={}, largest-region={}, worst-access-chunks={}, worst-access-bytes={}, all-independent={}",
        view.whole_object.feature_present,
        view.whole_object.region_count,
        view.whole_object.jpeg_region_count,
        view.whole_object.logical_bytes,
        view.whole_object.jpeg_xl_bytes,
        view.whole_object.stored_representation_bytes,
        view.whole_object.largest_region_bytes,
        view.whole_object.worst_access_chunks,
        view.whole_object.worst_access_bytes,
        view.whole_object.every_chunk_independently_decodable
    );
    for plan in view.plans {
        println!(
            "transform plan: id={} {} (pipeline={}; codec {}; dictionary={}; window={}, working-set={}, flags={:#x})",
            plan.plan_id,
            plan.identifier,
            if plan.transforms.is_empty() {
                "none".to_owned()
            } else {
                plan.transforms.join(" -> ")
            },
            plan.codec,
            plan.dictionary
                .map_or_else(|| "none".to_owned(), |value| value.to_string()),
            plan.decode.window_bytes,
            plan.decode.working_set_bytes,
            plan.decode.flags
        );
    }
    for usage in view.codec_usage {
        println!(
            "codec usage: {} chunks={}, logical-bytes={}, stored-bytes={}",
            usage.codec, usage.chunk_count, usage.logical_bytes, usage.stored_bytes
        );
    }
    println!("transformed chunks: {}", view.transformed_chunk_count);
    for usage in view.transform_usage {
        println!(
            "transform usage: {} chunks={}",
            usage.transform, usage.chunk_count
        );
    }
    println!("LAI: {}", view.identities.lai.0);
    println!("PCR: {}", view.identities.pcr.0);
    println!("AUX: {}", view.identities.aux.0);
    println!("PCI: {}", view.identities.pci.0);
    println!("index: {}", index_status(view.index_status));
    if let Some(stream) = &loaded.stream {
        println!(
            "stream body: bytes={}, chunk-frames={}, manifest-records={}, total-bytes={}",
            stream.body_len, stream.chunk_frames, stream.manifest_records, stream.total_len
        );
        println!(
            "stream retention: peak-retained-chunks={}, peak-resident-bytes={}, spilled-bytes={}",
            stream.peak_retained_chunks,
            stream.peak_resident_staging_bytes,
            stream.spilled_staging_bytes
        );
        println!(
            "stream access: random-entry-lookup={}, listing-requires-full-scan={}, source-replayable={}",
            stream.access.random_entry_lookup,
            stream.access.listing_requires_full_scan,
            stream.access.source_replayable
        );
    }
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
        "aggregate decode requirements: window={}, working-set={}, flags={:#x}, kdf-cost={}",
        view.decode_requirements.window_bytes,
        view.decode_requirements.working_set_bytes,
        view.decode_requirements.flags,
        budget.max_key_derivation_cost
    );
    Ok(())
}

fn command_explain(arguments: Vec<OsString>) -> Result<()> {
    let source = one_source("explain", arguments)?;
    // Compression explanation re-derives the alternatives the planner weighed,
    // so a STREAM source must be scanned with a retaining content policy.
    let loaded = load(&source, StreamContentPolicy::Retain)?;
    if loaded.stream.is_some() {
        eprintln!(
            "note: STREAM layout has no Index; this explanation required a complete sequential pass"
        );
    }
    let explanation = compression_explain(&loaded.opened)?;
    println!("planner: {}", explanation.planner_id);
    println!("total logical bytes: {}", explanation.total_logical_bytes);
    println!(
        "unique plaintext Chunk bytes: {}",
        explanation.total_plaintext_chunk_bytes
    );
    println!(
        "stored Chunk bytes: {}",
        explanation.total_stored_chunk_bytes
    );
    println!("unique Chunks: {}", explanation.chunks.unique_chunk_count);
    println!(
        "logical Chunk references: {}",
        explanation.chunks.logical_chunk_references
    );
    println!(
        "exact deduplication: eliminated-bytes={}, ratio={}.{:03}x",
        explanation.chunks.deduplicated_bytes,
        explanation.chunks.dedup_ratio_milli / 1_000,
        explanation.chunks.dedup_ratio_milli % 1_000
    );
    println!(
        "unique Chunk sizes: min={}, average={}, max={}",
        explanation.chunks.minimum_chunk_bytes,
        explanation.chunks.average_chunk_bytes,
        explanation.chunks.maximum_chunk_bytes
    );
    println!(
        "STORE: chunks={}, logical-bytes={}, stored-bytes={}",
        explanation.store_chunk_count,
        explanation.store_logical_bytes,
        explanation.store_stored_bytes
    );
    println!(
        "Zstandard: chunks={}, logical-bytes={}, stored-bytes={}",
        explanation.zstandard_chunk_count,
        explanation.zstandard_logical_bytes,
        explanation.zstandard_stored_bytes
    );
    println!(
        "total Chunk-payload compression savings: {} bytes",
        explanation.physical_savings_bytes
    );
    println!(
        "ordinary independent codec savings: {} bytes",
        explanation.ordinary_codec_savings_bytes
    );
    println!(
        "shared-dictionary payload savings: {} bytes (dictionary storage: {} bytes)",
        explanation.shared_dictionary_savings_bytes, explanation.dictionary_storage_bytes
    );
    println!(
        "bounded-lookback payload savings: {} bytes",
        explanation.bounded_lookback_savings_bytes
    );
    println!(
        "reconstructive transform: chunks={}, gross-savings={} bytes, reconstruction-data-overhead={} bytes, net-savings={} bytes",
        explanation.reconstructive_chunk_count,
        explanation.reconstructive_gross_savings_bytes,
        explanation.reconstruction_data_overhead_bytes,
        explanation.reconstructive_net_savings_bytes
    );
    println!(
        "reconstructive fallbacks: chunks={}{}",
        explanation.reconstructive_fallback_chunk_count,
        explanation
            .reconstructive_fallback_reason
            .as_ref()
            .map_or_else(String::new, |reason| format!(" ({reason})"))
    );
    println!(
        "JPEG reconstruction: gross-savings={} bytes, representation={} bytes, region-overhead={} bytes, net-savings={} bytes{}",
        explanation.jpeg_reconstructive_gross_savings_bytes,
        explanation.jpeg_representation_bytes,
        explanation.jpeg_region_overhead_bytes,
        explanation.jpeg_reconstructive_net_savings_bytes,
        explanation
            .jpeg_fallback_reason
            .as_ref()
            .map_or_else(String::new, |reason| format!(" (fallbacks: {reason})"))
    );
    println!(
        "structural-transform payload savings: {} bytes (transformed chunks: {}, rejected eligible chunks: {})",
        explanation.structural_transform_savings_bytes,
        explanation.transformed_chunk_count,
        explanation.transform_rejected_chunk_count
    );
    for usage in explanation.transform_usage {
        println!(
            "transform usage: {} chunks={}",
            usage.transform, usage.chunk_count
        );
    }
    for pipeline in explanation.representative_pipelines {
        println!("selected pipeline: {pipeline}");
    }
    if let Some(reason) = explanation.transform_rejection_reason {
        println!("transform candidate rule: {reason}");
    }
    println!(
        "similarity cohorts: count={}, chunks={}, logical-bytes={}, independently-encoded={}",
        explanation.similarity_cohort_count,
        explanation.similarity_cohort_chunks,
        explanation.similarity_cohort_logical_bytes,
        explanation.independent_similarity_cohort_count
    );
    if let Some(reason) = explanation.independent_cohort_reason {
        println!("independent cohort decision: {reason}");
    }
    Ok(())
}

fn command_verify(arguments: Vec<OsString>) -> Result<()> {
    let source = one_source("verify", arguments)?;
    let (report, stream) = match &source {
        Source::Stdin => {
            let sequential =
                open_stream_with_limits(std::io::stdin().lock(), bootstrap_sequential_limits())?;
            (sequential.opened.report, Some(sequential.stream))
        }
        Source::Path(path) if path_layout(path)? == Layout::Stream => {
            let file = File::open(path).map_err(|error| read_error(path, &error))?;
            let sequential = open_stream_with_limits(file, bootstrap_sequential_limits())?;
            (sequential.opened.report, Some(sequential.stream))
        }
        Source::Path(path) => (verify(&read(path)?)?, None),
    };
    println!(
        "OK verified canonical structure, section integrity, semantic invariants, Dictionary/ChunkGroup/ReconstructionData dependencies and access costs, reconstructed original Chunk bytes, Chunk/content integrity, Entry identities, LAI, PCR, AUX, and exact-byte PCI"
    );
    if let Some(stream) = &stream {
        println!(
            "OK verified STREAM item framing and order, stored lengths, stream dedup-window constraints, footer binding, and exact total length"
        );
        println!("layout: STREAM");
        println!("stream dedup window: {}", stream.dedup_window);
        println!("budget declared: {}", stream.budget_declared);
        println!(
            "stream body: bytes={}, chunk-frames={}, manifest-records={}, total-bytes={}",
            stream.body_len, stream.chunk_frames, stream.manifest_records, stream.total_len
        );
    }
    println!("index: {}", index_status(report.index_status));
    println!("LAI {}", report.identities.lai.0);
    println!("PCR {}", report.identities.pcr.0);
    println!("AUX {}", report.identities.aux.0);
    println!("PCI {}", report.identities.pci.0);
    Ok(())
}

const fn index_status(status: IndexStatus) -> &'static str {
    match status {
        IndexStatus::PresentValid => "present and valid",
        IndexStatus::RebuiltAbsent => "absent; rebuilt from authoritative CHUNK_DATA",
        IndexStatus::RebuiltInvalid => "invalid; rebuilt from authoritative CHUNK_DATA",
        IndexStatus::NotApplicableStream => {
            "not applicable; STREAM layout carries no Index by design"
        }
    }
}

fn one_source(command: &str, arguments: Vec<OsString>) -> Result<Source> {
    if arguments.len() != 1 {
        return Err(usage(format!("{command} requires <archive.eb|->")));
    }
    Ok(Source::parse(&arguments[0]))
}

fn ensure_no_more(mut arguments: impl Iterator<Item = OsString>) -> Result<()> {
    if arguments.next().is_some() {
        return Err(usage("help does not accept additional arguments"));
    }
    Ok(())
}

fn read(path: &Path) -> Result<Vec<u8>> {
    std::fs::read(path).map_err(|error| read_error(path, &error))
}

fn read_error(path: &Path, error: &std::io::Error) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::PolicyRefused,
        ReasonCode::Io,
        format!("cannot read '{}': {error}", path.display()),
    )
}

fn io_error(context: &str, error: &std::io::Error) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::PolicyRefused,
        ReasonCode::Io,
        format!("cannot {context}: {error}"),
    )
}

fn create_exclusive(path: &Path) -> Result<File> {
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| {
            Diagnostic::new(
                OutcomeClass::PolicyRefused,
                ReasonCode::Io,
                format!("cannot create output '{}': {error}", path.display()),
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

    #[test]
    fn stream_window_is_rejected_for_the_indexed_layout() {
        let error = run(args(&[
            "ebound",
            "pack",
            "input",
            "output.eb",
            "--layout",
            "indexed",
            "--stream-window",
            "4",
        ]))
        .unwrap_err();
        assert_eq!(error.code(), ReasonCode::CommandUsage);
    }

    #[test]
    fn an_unknown_layout_is_a_usage_error() {
        let error = run(args(&[
            "ebound",
            "pack",
            "input",
            "output.eb",
            "--layout",
            "mounted",
        ]))
        .unwrap_err();
        assert_eq!(error.code(), ReasonCode::CommandUsage);
    }
}
