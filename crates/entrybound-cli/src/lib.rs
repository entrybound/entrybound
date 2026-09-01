use std::env;
use std::ffi::{OsStr, OsString};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use entrybound::archive::{
    ConfinementMode, ExtractionPolicy, PackOptions, default_pack_output,
    default_unpack_destination, explain as compression_explain, inspect, list, plan_directory,
    unpack, unpack_opened, unpack_stream,
};
use entrybound::crypto::{
    BindingStatus, BoundaryMode, CryptoPolicy, CryptographicStatus, EncryptedOpenOptions,
    EncryptedWriteOptions, FEATURE_ENCRYPTED_INDEXED_V1, FEATURE_PRIVATE_RESOURCE_DECLARATION_V1,
    PaddingMode, SignaturePolicy, SignatureRecord, SignatureStatus, SigningKey, TimestampPolicy,
    TimestampStatus, TimestampTrustAnchor, Unlock, XWingIdentity, XWingRecipient, add_recipient,
    change_password, current_bindings, embed_signature, inspect_encrypted, open_encrypted,
    open_encrypted_authenticated, pack_directory_encrypted, read_detached_signature,
    reencrypt_recipients, sign_archive, verify_signature,
};
use entrybound::diagnostics::{Diagnostic, OutcomeClass, ReasonCode, Result};
use entrybound::eam::{ArchiveRole, EntryKind, Layout};
use entrybound::ecf::{
    IndexStatus, OpenedArchive, SequentialLimits, StreamContentPolicy, StreamReport, StreamWindow,
    StreamWriteOptions, WriteOptions, bootstrap_sequential_limits, encode, encode_stream, open,
    open_stream_with_limits, peek_layout,
};
use entrybound::legacy::import::{
    LegacyConversionReport, LegacyImportPolicy, LegacyImportResult, LegacySourceFormat,
    detect as detect_legacy, import_strict as import_legacy_strict,
};
use entrybound::legacy::zip::{
    CompatibilityProfileId, ImportPolicy, ZipImportPolicy, import as import_zip,
};
use entrybound::planner::CompressionProfile;
use zeroize::Zeroizing;

const HELP: &str = "\
Entrybound (experimental native bootstrap)\n\
\n\
Usage:\n\
  ebound pack <input-directory> [output.eb|-] [--layout indexed|stream]\n\
                                [--stream-window <n>|auto]\n\
                                [--profile fast|balanced|dense|extreme]\n\
                                [--recipient <file> ... | --password]\n\
                                [--pad bucketed|none|max]\n\
                                [--chunk-boundary default|keyed-prf]\n\
  ebound convert <input> <output.eb|-> [--strict|--compat=<versioned-profile>]\n\
                         [--preserve --compat=<versioned-profile>] [--dry-run]\n\
                         [--from zip|tar|gzip|zstd|xz|bzip2|tar.gz|tar.zst|tar.xz|tar.bz2]\n\
                         [--entry-name <logical-path>]\n\
                         [--layout indexed|stream]\n\
                         [--profile fast|balanced|dense|extreme]\n\
  ebound unpack <archive.eb|-> [destination] [--identity <file>|--password]\n\
  ebound list <archive.eb|->\n\
  ebound inspect <archive.eb|-> [--crypto] [--identity <file>|--password]\n\
                                [--timestamp-trust <anchor.der> ...]\n\
  ebound verify <archive.eb|-> [--identity <file>|--password]\n\
                               [--signatures|--signature <archive.ebsig>]\n\
  ebound sign <archive.eb> --signing-key <file> [--detached [file]|--embed]\n\
                           [--identity <file>|--password]\n\
                           [--bind-physical] [--bind-addressing]\n\
                           [--timestamp-token <token.der>]\n\
  ebound key generate-signing <signing-key>\n\
  ebound key list <archive.eb> [--identity <file>|--password]\n\
  ebound key add <archive.eb> --identity <file> --recipient <recipient.pub>\n\
  ebound key remove <archive.eb> --identity <file> --retain <recipient.pub> ...\n\
  ebound key change-password <archive.eb> --password\n\
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
Crypto v1 adds metadata-private encrypted INDEXED archives using either one\n\
or more X-Wing draft-10 hybrid recipients, or one password-only Argon2id\n\
recipient. Ed25519 signatures may be detached, or embedded as encrypted private\n\
CONTROL data; RFC 3161 verification is offline against caller trust anchors.\n\
Authenticated recipient addition preserves the AFK, while removal and password\n\
rotation use full fresh-key re-encryption. Encrypted STREAM is unsupported.\n\
\n\
Use `-` for standard input or standard output. When archive bytes go to\n\
standard output, all status output goes to standard error.\n\
The default creation profile is balanced; decoding is self-describing and\n\
never requires a profile.\n";

const PACK_HELP: &str = "\
Usage: ebound pack <input-directory> [output.eb|-] [--layout indexed|stream]\n\
                                     [--stream-window <n>|auto]\n\
                                     [--profile fast|balanced|dense|extreme]\n\
                                     [--recipient <file> ... | --password]\n\
                                     [--pad bucketed|none|max]\n\
                                     [--chunk-boundary default|keyed-prf]\n\
\n\
Creates a native .eb archive. Unencrypted output is deterministic; encrypted\n\
output uses fresh secure randomness. The default profile is balanced.\n\
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
may require a non-zero window.\n\
\n\
--recipient may be repeated for X-Wing draft-10 recipients. --password creates\n\
one password-only archive and prompts twice on the controlling terminal; the\n\
two recipient modes cannot be mixed. Encryption is INDEXED-only. Encrypted\n\
padding defaults to bucketed; --pad none is a recorded privacy-reducing opt-out\n\
and --pad max uses the maximum class. The encrypted boundary default is a\n\
secret AFK-derived Gear table; --chunk-boundary keyed-prf selects PHTE/AES-128.\n";

const CONVERT_HELP: &str = "\
Usage: ebound convert <input> <output.eb|->\n\
                      [--strict | --compat=<versioned-profile>]\n\
                      [--preserve --compat=<versioned-profile>] [--dry-run]\n\
                      [--from zip|tar|gzip|zstd|xz|bzip2|tar.gz|tar.zst|tar.xz|tar.bz2]\n\
                      [--entry-name <logical-path>]\n\
                      [--layout indexed|stream]\n\
                      [--profile fast|balanced|dense|extreme]\n\
\n\
ZIP modes retain independent central, local, and descriptor observations. Strict\n\
tar supports ustar, pax, GNU long-name, and base-256 evidence. gzip, Zstandard,\n\
XZ, and bzip2 are bounded transport layers whose decoded children use the same\n\
tar adapter. A non-tar stream requires --entry-name. ZIP compatibility and\n\
preservation remain available only through exact versioned ZIP profiles.\n";

const SIGN_HELP: &str = "\
Usage: ebound sign <archive.eb> --signing-key <file>\n\
             [--detached [archive.ebsig] | --embed]\n\
             [--identity <file> | --password]\n\
             [--bind-physical] [--bind-addressing]\n\
             [--timestamp-token <token.der>]\n\
\n\
CONTENT is always bound. PHYSICAL binds PCR, not PCI. ADDRESSING is available\n\
only after authenticating an encrypted INDEXED archive. Signing defaults to\n\
every binding available for the archive. Detached signatures\n\
are one exact canonical type-26 record. Embedded signatures are encrypted\n\
private CONTROL data and do not rotate the archive file key or rewrite bulk\n\
payload ciphertext. RFC 3161 tokens are supplied externally; Entrybound never\n\
contacts a TSA.\n";

const KEY_HELP: &str = "\
Usage:\n\
  ebound key generate-signing <signing-key>\n\
  ebound key list <archive.eb> [--identity <file>|--password]\n\
  ebound key add <archive.eb> --identity <file> --recipient <recipient.pub>\n\
  ebound key remove <archive.eb> --identity <file> --retain <recipient.pub> ...\n\
  ebound key change-password <archive.eb> --password\n\
\n\
Adding a hybrid recipient rewraps the existing AFK and preserves bulk PAYLOAD\n\
ciphertext. Removal and password changes rotate AFK/archive ID and perform a\n\
complete verified re-encryption; deleting a stanza is never treated as\n\
revocation. Mutations replace the original only after the replacement verifies.\n";

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
        "convert" => command_convert(arguments.collect()),
        "unpack" => command_unpack(arguments.collect()),
        "list" => command_list(arguments.collect()),
        "inspect" => command_inspect(arguments.collect()),
        "verify" => command_verify(arguments.collect()),
        "sign" => command_sign(arguments.collect()),
        "key" => command_key(arguments.collect()),
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

enum OwnedUnlock {
    Identity(XWingIdentity),
    Password(Zeroizing<String>),
}

impl OwnedUnlock {
    fn borrowed(&self) -> Unlock<'_> {
        match self {
            Self::Identity(identity) => Unlock::Identity(identity),
            Self::Password(password) => Unlock::Password(password.as_bytes()),
        }
    }
}

struct ReadArguments {
    positionals: Vec<OsString>,
    unlock: Option<OwnedUnlock>,
    crypto: bool,
    timestamp_trust: Vec<PathBuf>,
}

fn parse_read_arguments(
    command: &str,
    arguments: Vec<OsString>,
    allow_crypto_flag: bool,
) -> Result<ReadArguments> {
    let mut positionals = Vec::new();
    let mut identity = None;
    let mut password = false;
    let mut crypto = false;
    let mut timestamp_trust = Vec::new();
    let mut cursor = 0;
    while cursor < arguments.len() {
        let value = &arguments[cursor];
        if value == "--identity" {
            if identity.is_some() {
                return Err(usage(format!("{command} accepts --identity only once")));
            }
            cursor += 1;
            let path = arguments
                .get(cursor)
                .ok_or_else(|| usage("--identity requires an identity key file"))?;
            identity = Some(PathBuf::from(path));
        } else if value == "--password" {
            if password {
                return Err(usage(format!("{command} accepts --password only once")));
            }
            password = true;
        } else if value == "--crypto" && allow_crypto_flag {
            if crypto {
                return Err(usage("inspect accepts --crypto only once"));
            }
            crypto = true;
        } else if value == "--timestamp-trust" && allow_crypto_flag {
            cursor += 1;
            timestamp_trust.push(PathBuf::from(
                arguments
                    .get(cursor)
                    .ok_or_else(|| usage("--timestamp-trust requires a DER certificate"))?,
            ));
        } else if value.to_string_lossy().starts_with("--") {
            return Err(usage(format!(
                "{command} does not recognize option '{}'",
                value.to_string_lossy()
            )));
        } else {
            positionals.push(value.clone());
        }
        cursor += 1;
    }
    if password && identity.is_some() {
        return Err(usage("--identity and --password cannot be combined"));
    }
    let unlock = if let Some(path) = identity {
        warn_identity_permissions(&path);
        Some(OwnedUnlock::Identity(XWingIdentity::read_file(&path)?))
    } else if password {
        Some(OwnedUnlock::Password(Zeroizing::new(prompt_password(
            "Archive password: ",
        )?)))
    } else {
        None
    };
    Ok(ReadArguments {
        positionals,
        unlock,
        crypto,
        timestamp_trust,
    })
}

#[cfg(unix)]
fn warn_identity_permissions(path: &Path) {
    use std::os::unix::fs::PermissionsExt as _;

    if std::fs::metadata(path).is_ok_and(|metadata| metadata.permissions().mode() & 0o077 != 0) {
        eprintln!(
            "warning: identity file '{}' is readable by group or other users",
            path.display()
        );
    }
}

#[cfg(not(unix))]
fn warn_identity_permissions(_path: &Path) {}

fn prompt_password(prompt: &str) -> Result<String> {
    rpassword::prompt_password(prompt).map_err(|error| {
        Diagnostic::new(
            OutcomeClass::PolicyRefused,
            ReasonCode::Io,
            format!("cannot read password from the controlling terminal: {error}"),
        )
    })
}

fn path_is_encrypted(path: &Path) -> Result<bool> {
    let mut preamble = [0_u8; 24];
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
    Ok(filled == preamble.len()
        && &preamble[..8] == entrybound::ecf::MAGIC.as_slice()
        && u64::from_be_bytes(preamble[16..24].try_into().unwrap()) & FEATURE_ENCRYPTED_INDEXED_V1
            != 0)
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
    let mut recipient_paths = Vec::new();
    let mut password = false;
    let mut padding = None;
    let mut boundary = None;
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
        } else if value == "--recipient" {
            cursor += 1;
            let path = arguments
                .get(cursor)
                .ok_or_else(|| usage("--recipient requires a recipient key file"))?;
            recipient_paths.push(PathBuf::from(path));
        } else if value == "--password" {
            if password {
                return Err(usage("pack accepts --password only once"));
            }
            password = true;
        } else if value == "--pad" {
            if padding.is_some() {
                return Err(usage("pack accepts --pad only once"));
            }
            cursor += 1;
            padding = Some(
                match arguments.get(cursor).and_then(|value| value.to_str()) {
                    Some("none") => PaddingMode::None,
                    Some("bucketed") => PaddingMode::Bucketed,
                    Some("max") => PaddingMode::Maximum,
                    _ => return Err(usage("--pad requires 'bucketed', 'none', or 'max'")),
                },
            );
        } else if value == "--chunk-boundary" {
            if boundary.is_some() {
                return Err(usage("pack accepts --chunk-boundary only once"));
            }
            cursor += 1;
            boundary = Some(
                match arguments.get(cursor).and_then(|value| value.to_str()) {
                    Some("default") => BoundaryMode::SecretGearTable,
                    Some("keyed-prf") => BoundaryMode::PhteAes128,
                    _ => {
                        return Err(usage("--chunk-boundary requires 'default' or 'keyed-prf'"));
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
    let encrypted = password || !recipient_paths.is_empty();
    if password && !recipient_paths.is_empty() {
        return Err(Diagnostic::new(
            OutcomeClass::PolicyRefused,
            ReasonCode::CryptoRecipientPolicyInvalid,
            "password and hybrid recipients cannot be mixed",
        ));
    }
    if (padding.is_some() || boundary.is_some()) && !encrypted {
        return Err(usage(
            "--pad and --chunk-boundary require --recipient or --password",
        ));
    }
    if encrypted && (layout == Layout::Stream || matches!(destination, Destination::Stdout)) {
        return Err(Diagnostic::new(
            OutcomeClass::Unsupported,
            ReasonCode::CryptoLayoutUnsupported,
            "crypto v1 supports seekable INDEXED output only; no bytes were emitted",
        ));
    }
    let status = Status {
        to_stderr: matches!(destination, Destination::Stdout),
    };
    let options = PackOptions {
        profile: profile.unwrap_or_default(),
        ..PackOptions::default()
    };

    if encrypted {
        let recipients = recipient_paths
            .iter()
            .map(|path| XWingRecipient::read_file(path))
            .collect::<Result<Vec<_>>>()?;
        let password_value = if password {
            let first = Zeroizing::new(prompt_password("Encryption password: ")?);
            let second = Zeroizing::new(prompt_password("Confirm encryption password: ")?);
            if *first != *second {
                return Err(usage("password confirmation did not match"));
            }
            Some(first)
        } else {
            None
        };
        let encoded = pack_directory_encrypted(
            &input,
            options,
            EncryptedWriteOptions {
                recipients: &recipients,
                password: password_value.as_ref().map(|value| value.as_bytes()),
                padding: padding.unwrap_or_default(),
                boundary: boundary.unwrap_or_default(),
                include_index: options.include_index,
                embedded_signatures: &[],
            },
        )?;
        let Destination::Path(path) = &destination else {
            unreachable!("encrypted stdout was rejected before planning")
        };
        let mut file = create_exclusive(path)?;
        if let Err(error) = file
            .write_all(&encoded.bytes)
            .map_err(|error| io_error("write encrypted archive output", &error))
        {
            drop(file);
            let _ = std::fs::remove_file(path);
            return Err(error);
        }
        status.line(format!(
            "OK packed and encrypted {} entries into {}",
            encoded.archive.entry_set.len(),
            path.display()
        ));
        status.line("layout INDEXED (encrypted crypto-v1)");
        status.line(format!("padding {:?}", encoded.public.padding));
        status.line(format!("boundary {:?}", encoded.public.boundary));
        print_identities(&status, &encoded.identities);
        status.line(format!("planner {}", encoded.archive.descriptor.planner_id));
        return Ok(());
    }

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

// ---------------------------------------------------------------------------
// legacy conversion policies
// ---------------------------------------------------------------------------

fn command_convert(arguments: Vec<OsString>) -> Result<()> {
    if arguments.len() == 1 && matches!(arguments[0].to_str(), Some("--help" | "-h")) {
        print!("{CONVERT_HELP}");
        return Ok(());
    }
    let mut positionals = Vec::new();
    let mut profile = None;
    let mut layout = None;
    let mut strict = false;
    let mut compat = None;
    let mut preserve = false;
    let mut dry_run = false;
    let mut from = None;
    let mut entry_name = None;
    let mut cursor = 0;
    while cursor < arguments.len() {
        let value = &arguments[cursor];
        if value == "--strict" {
            if strict {
                return Err(usage("convert accepts --strict only once"));
            }
            strict = true;
        } else if value == "--preserve" {
            if preserve {
                return Err(usage("convert accepts --preserve only once"));
            }
            preserve = true;
        } else if value == "--dry-run" {
            if dry_run {
                return Err(usage("convert accepts --dry-run only once"));
            }
            dry_run = true;
        } else if let Some(profile_id) = value
            .to_str()
            .and_then(|value| value.strip_prefix("--compat="))
        {
            if compat.is_some() {
                return Err(usage("convert accepts --compat only once"));
            }
            compat = Some(profile_id.parse::<CompatibilityProfileId>()?);
        } else if value == "--compat" {
            if compat.is_some() {
                return Err(usage("convert accepts --compat only once"));
            }
            cursor += 1;
            compat = Some(
                arguments
                    .get(cursor)
                    .and_then(|value| value.to_str())
                    .ok_or_else(|| usage("--compat requires an exact versioned profile"))?
                    .parse::<CompatibilityProfileId>()?,
            );
        } else if let Some(format) = value
            .to_str()
            .and_then(|value| value.strip_prefix("--from="))
        {
            if from.is_some() {
                return Err(usage("convert accepts --from only once"));
            }
            from = Some(format.parse::<LegacySourceFormat>()?);
        } else if value == "--from" {
            if from.is_some() {
                return Err(usage("convert accepts --from only once"));
            }
            cursor += 1;
            from = Some(
                arguments
                    .get(cursor)
                    .and_then(|value| value.to_str())
                    .ok_or_else(|| usage("--from requires a supported format"))?
                    .parse::<LegacySourceFormat>()?,
            );
        } else if let Some(name) = value
            .to_str()
            .and_then(|value| value.strip_prefix("--entry-name="))
        {
            if entry_name.is_some() {
                return Err(usage("convert accepts --entry-name only once"));
            }
            entry_name = Some(name.to_owned());
        } else if value == "--entry-name" {
            if entry_name.is_some() {
                return Err(usage("convert accepts --entry-name only once"));
            }
            cursor += 1;
            entry_name = Some(
                arguments
                    .get(cursor)
                    .and_then(|value| value.to_str())
                    .ok_or_else(|| usage("--entry-name requires a UTF-8 LogicalPath"))?
                    .to_owned(),
            );
        } else if value == "--profile" {
            if profile.is_some() {
                return Err(usage("convert accepts --profile only once"));
            }
            cursor += 1;
            profile = Some(
                arguments
                    .get(cursor)
                    .and_then(|value| value.to_str())
                    .ok_or_else(|| usage("--profile requires a UTF-8 profile name"))?
                    .parse::<CompressionProfile>()?,
            );
        } else if value == "--layout" {
            if layout.is_some() {
                return Err(usage("convert accepts --layout only once"));
            }
            cursor += 1;
            layout = Some(
                match arguments.get(cursor).and_then(|value| value.to_str()) {
                    Some("indexed") => Layout::Indexed,
                    Some("stream") => Layout::Stream,
                    _ => return Err(usage("--layout requires 'indexed' or 'stream'")),
                },
            );
        } else if value.to_string_lossy().starts_with("--") {
            return Err(usage(format!(
                "convert does not recognize option '{}'",
                value.to_string_lossy()
            )));
        } else {
            positionals.push(value.clone());
        }
        cursor += 1;
    }
    if positionals.len() != 2 {
        return Err(usage(
            "convert requires <input> <output.eb|-> [--strict] [--from <format>] \
             [--layout indexed|stream] [--profile ...]",
        ));
    }
    if strict && compat.is_some() {
        return Err(usage("--strict and --compat are mutually exclusive"));
    }
    if preserve && compat.is_none() {
        return Err(usage("--preserve requires --compat=<versioned-profile>"));
    }
    let import_policy = match (preserve, compat) {
        (true, Some(profile)) => ImportPolicy::Preservation(profile),
        (false, Some(profile)) => ImportPolicy::Compatibility(profile),
        (false, None) => ImportPolicy::Strict,
        (true, None) => unreachable!("validated above"),
    };
    let input = PathBuf::from(&positionals[0]);
    let destination = if positionals[1] == OsStr::new("-") {
        Destination::Stdout
    } else {
        Destination::Path(PathBuf::from(&positionals[1]))
    };
    let layout = layout.unwrap_or(match destination {
        Destination::Stdout => Layout::Stream,
        Destination::Path(_) => Layout::Indexed,
    });
    let source_bytes = read(&input)?;
    let creation_profile = profile.unwrap_or_default();
    let imported = if import_policy == ImportPolicy::Strict {
        import_legacy_strict(
            &source_bytes,
            from,
            entry_name.as_deref(),
            LegacyImportPolicy::default(),
            creation_profile,
        )?
    } else {
        if entry_name.is_some() {
            return Err(usage(
                "--entry-name is unavailable for ZIP compatibility/preservation",
            ));
        }
        if from.is_some_and(|format| format != LegacySourceFormat::Zip)
            || detect_legacy(&source_bytes).is_some_and(|format| format != LegacySourceFormat::Zip)
        {
            return Err(Diagnostic::new(
                OutcomeClass::Unsupported,
                ReasonCode::UnsupportedRequiredFeature,
                "--compat and --preserve are currently defined only for ZIP",
            ));
        }
        let imported = import_zip(
            &source_bytes,
            ZipImportPolicy::default(),
            creation_profile,
            import_policy,
        )?;
        LegacyImportResult {
            archive: imported.archive,
            report: LegacyConversionReport {
                observation: imported.report.observation,
                synthesized_ancestors: imported.report.synthesized_ancestors,
                layers: Box::from(["zip".to_owned()]),
                wrapper_members: 0,
                decoded_child_digest: None,
                projection: "archive".to_owned(),
            },
        }
    };
    let status = Status {
        to_stderr: matches!(destination, Destination::Stdout),
    };
    let (identities, output_entries) = if dry_run {
        match layout {
            Layout::Indexed => {
                let encoded = encode(&imported.archive, WriteOptions::default())?;
                (encoded.identities, encoded.archive.entry_set.len())
            }
            Layout::Stream => {
                let mut discarded = Vec::new();
                let summary = encode_stream(
                    &imported.archive,
                    StreamWriteOptions::default(),
                    &mut discarded,
                )?;
                (summary.identities, summary.archive.entry_set.len())
            }
        }
    } else {
        match layout {
            Layout::Indexed => {
                let encoded = encode(&imported.archive, WriteOptions::default())?;
                write_archive_destination(&destination, &encoded.bytes)?;
                (encoded.identities, encoded.archive.entry_set.len())
            }
            Layout::Stream => {
                let summary = match &destination {
                    Destination::Stdout => {
                        let stdout = std::io::stdout();
                        let mut handle = stdout.lock();
                        let summary = encode_stream(
                            &imported.archive,
                            StreamWriteOptions::default(),
                            &mut handle,
                        )?;
                        handle
                            .flush()
                            .map_err(|error| io_error("flush standard output", &error))?;
                        summary
                    }
                    Destination::Path(path) => {
                        let mut file = create_exclusive(path)?;
                        match encode_stream(
                            &imported.archive,
                            StreamWriteOptions::default(),
                            &mut file,
                        ) {
                            Ok(summary) => summary,
                            Err(error) => {
                                drop(file);
                                let _ = std::fs::remove_file(path);
                                return Err(error);
                            }
                        }
                    }
                };
                (summary.identities, summary.archive.entry_set.len())
            }
        }
    };
    let observation = &imported.report.observation;
    let provenance = imported
        .archive
        .conversion
        .as_ref()
        .expect("legacy conversion records provenance");
    status.line(format!("source: {}", provenance.source_format));
    status.line(format!("mode: {}", import_policy.mode()));
    if let Some(profile) = import_policy.compatibility_profile() {
        status.line(format!("compat-profile: {}", profile.as_str()));
    }
    if import_policy.preserves_evidence() {
        status.line("preservation: exact-source + observations");
    }
    status.line(format!("layers: {}", imported.report.layers.join(" -> ")));
    if imported.report.wrapper_members != 0 {
        status.line(format!(
            "wrapper members: {}",
            imported.report.wrapper_members
        ));
    }
    if let Some(digest) = imported.report.decoded_child_digest {
        status.line(format!("decoded child digest: {digest}"));
    }
    status.line(format!("projection: {}", imported.report.projection));
    status.line(format!("entries observed: {}", observation.entries.len()));
    status.line(format!(
        "conflicts: omission={}, refinement={}, divergence={}, irreconcilable={}",
        provenance.omission_count,
        provenance.refinement_count,
        provenance.divergence_count,
        provenance.irreconcilable_count,
    ));
    status.line(format!(
        "resolved: {} (synthesized ancestors: {})",
        provenance.resolutions.len(),
        imported.report.synthesized_ancestors.len(),
    ));
    status.line(format!(
        "OK {} {output_entries} native entries{}",
        if dry_run {
            "dry-run resolved"
        } else {
            "converted"
        },
        if dry_run {
            String::new()
        } else {
            format!(" into {} {}", layout.as_str(), describe(&destination))
        },
    ));
    print_identities(&status, &identities);
    Ok(())
}

fn write_archive_destination(destination: &Destination, bytes: &[u8]) -> Result<()> {
    match destination {
        Destination::Stdout => {
            let stdout = std::io::stdout();
            let mut handle = stdout.lock();
            handle
                .write_all(bytes)
                .map_err(|error| io_error("write standard output", &error))?;
            handle
                .flush()
                .map_err(|error| io_error("flush standard output", &error))
        }
        Destination::Path(path) => {
            let mut file = create_exclusive(path)?;
            if let Err(error) = file
                .write_all(bytes)
                .map_err(|error| io_error("write converted archive output", &error))
            {
                drop(file);
                let _ = std::fs::remove_file(path);
                return Err(error);
            }
            Ok(())
        }
    }
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
    let parsed = parse_read_arguments("unpack", arguments, false)?;
    if !(1..=2).contains(&parsed.positionals.len()) {
        return Err(usage(
            "unpack requires <archive.eb|-> [destination] [--identity <file>|--password]",
        ));
    }
    let source = Source::parse(&parsed.positionals[0]);
    let destination = match parsed.positionals.get(1) {
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
    let encrypted = match &source {
        Source::Path(path) => path_is_encrypted(path)?,
        Source::Stdin => parsed.unlock.is_some(),
    };
    if encrypted {
        let bytes = read_source_fully(&source)?;
        let unlock = parsed.unlock.as_ref().map(OwnedUnlock::borrowed);
        let opened = open_encrypted(&bytes, EncryptedOpenOptions::new(unlock))?;
        let report = unpack_opened(&opened, &destination, ExtractionPolicy::default())?;
        status.line(format!(
            "OK authenticated, verified, and unpacked {} entries and {} logical bytes into {}",
            report.entries_created,
            report.logical_bytes_written,
            destination.display()
        ));
        status.line("layout: encrypted INDEXED crypto-v1");
        status.line(format!(
            "confinement: {}",
            match report.confinement {
                ConfinementMode::KernelEnforced => "kernel-enforced capability-relative",
                ConfinementMode::WeakerReported => "weaker platform mode (reported)",
            }
        ));
        return Ok(());
    }
    if parsed.unlock.is_some() {
        return Err(usage(
            "an identity/password was supplied for an unencrypted archive",
        ));
    }
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
// signatures and authenticated recipient mutation
// ---------------------------------------------------------------------------

fn command_sign(arguments: Vec<OsString>) -> Result<()> {
    if arguments.len() == 1 && matches!(arguments[0].to_str(), Some("--help" | "-h")) {
        print!("{SIGN_HELP}");
        return Ok(());
    }
    let mut positionals = Vec::new();
    let mut signing_key = None;
    let mut detached = None::<Option<PathBuf>>;
    let mut embed = false;
    let mut identity = None;
    let mut password = false;
    let mut bind_physical = true;
    let mut bind_addressing = false;
    let mut timestamp = None;
    let mut cursor = 0;
    while cursor < arguments.len() {
        let value = &arguments[cursor];
        if value == "--signing-key" {
            cursor += 1;
            signing_key = Some(PathBuf::from(
                arguments
                    .get(cursor)
                    .ok_or_else(|| usage("--signing-key requires a file"))?,
            ));
        } else if value == "--detached" {
            if detached.is_some() {
                return Err(usage("sign accepts --detached only once"));
            }
            let explicit = arguments
                .get(cursor + 1)
                .filter(|next| !next.to_string_lossy().starts_with("--") && positionals.len() == 1);
            if let Some(path) = explicit {
                cursor += 1;
                detached = Some(Some(PathBuf::from(path)));
            } else {
                detached = Some(None);
            }
        } else if value == "--embed" {
            embed = true;
        } else if value == "--identity" {
            cursor += 1;
            identity = Some(PathBuf::from(
                arguments
                    .get(cursor)
                    .ok_or_else(|| usage("--identity requires a file"))?,
            ));
        } else if value == "--password" {
            password = true;
        } else if value == "--bind-physical" {
            bind_physical = true;
        } else if value == "--bind-addressing" {
            bind_addressing = true;
        } else if value == "--timestamp-token" {
            cursor += 1;
            timestamp =
                Some(PathBuf::from(arguments.get(cursor).ok_or_else(|| {
                    usage("--timestamp-token requires a DER token")
                })?));
        } else if value.to_string_lossy().starts_with("--") {
            return Err(usage(format!(
                "sign does not recognize option '{}'",
                value.to_string_lossy()
            )));
        } else {
            positionals.push(value.clone());
        }
        cursor += 1;
    }
    if positionals.len() != 1 || signing_key.is_none() || (embed && detached.is_some()) {
        return Err(usage(
            "sign requires <archive.eb> --signing-key <file> and at most one of --embed/--detached",
        ));
    }
    let archive_path = PathBuf::from(&positionals[0]);
    let encrypted = path_is_encrypted(&archive_path)?;
    // The frozen CLI policy signs every binding the archive can provide:
    // CONTENT+PHYSICAL for plaintext, plus ADDRESSING after encrypted unlock.
    bind_addressing |= encrypted;
    if !encrypted && embed {
        return Err(Diagnostic::new(
            OutcomeClass::Unsupported,
            ReasonCode::SignatureUnsupported,
            "embedded signatures are defined only for encrypted INDEXED crypto-v1; use .ebsig",
        ));
    }
    let unlock = load_unlock(identity, password, "Archive password: ")?;
    if encrypted && unlock.is_none() {
        return Err(usage(
            "signing an encrypted archive requires --identity or --password",
        ));
    }
    if !encrypted && unlock.is_some() {
        return Err(usage(
            "unlock material was supplied for an unencrypted archive",
        ));
    }
    let bytes = read(&archive_path)?;
    let (opened, addressing) = if encrypted {
        let authenticated = open_encrypted_authenticated(
            &bytes,
            EncryptedOpenOptions::new(unlock.as_ref().map(OwnedUnlock::borrowed)),
        )?;
        (authenticated.opened, Some(authenticated.addressing))
    } else {
        (open(&bytes)?, None)
    };
    let current = current_bindings(&opened, addressing)?;
    let signing_key_path = signing_key.expect("validated signing key");
    warn_identity_permissions(&signing_key_path);
    let key = SigningKey::read_file(&signing_key_path)?;
    let mut signature = sign_archive(&current, &key, bind_physical, bind_addressing)?;
    if let Some(path) = timestamp {
        signature = signature.with_timestamp_token(read(&path)?)?;
    }
    if embed {
        let replacement = embed_signature(
            &bytes,
            EncryptedOpenOptions::new(unlock.as_ref().map(OwnedUnlock::borrowed)),
            signature,
        )?;
        replace_verified(&archive_path, &replacement.bytes)?;
        println!(
            "OK embedded signature and atomically replaced {}",
            archive_path.display()
        );
    } else {
        let output = detached
            .flatten()
            .unwrap_or_else(|| detached_path(&archive_path));
        let mut file = create_exclusive(&output)?;
        file.write_all(&signature.encode()?)
            .and_then(|()| file.sync_all())
            .map_err(|error| io_error("write detached signature", &error))?;
        println!("OK wrote detached signature {}", output.display());
    }
    Ok(())
}

fn command_key(arguments: Vec<OsString>) -> Result<()> {
    if arguments.len() == 1 && matches!(arguments[0].to_str(), Some("--help" | "-h")) {
        print!("{KEY_HELP}");
        return Ok(());
    }
    let Some(operation) = arguments.first().and_then(|value| value.to_str()) else {
        return Err(usage(
            "key requires generate-signing, list, add, remove, or change-password",
        ));
    };
    let rest = arguments[1..].to_vec();
    match operation {
        "generate-signing" => command_key_generate_signing(rest),
        "list" => command_key_list(rest),
        "add" => command_key_add(rest),
        "remove" => command_key_remove(rest),
        "change-password" => command_key_change_password(rest),
        _ => Err(usage("unknown key operation")),
    }
}

fn command_key_generate_signing(arguments: Vec<OsString>) -> Result<()> {
    if arguments.len() != 1 {
        return Err(usage("key generate-signing requires <signing-key>"));
    }
    let path = PathBuf::from(&arguments[0]);
    let key = SigningKey::generate()?;
    let mut file = create_exclusive(&path)?;
    file.write_all(&key.encode_file())
        .and_then(|()| file.sync_all())
        .map_err(|error| io_error("write signing key", &error))?;
    restrict_secret_permissions(&path)?;
    println!("OK generated Ed25519 signing key {}", path.display());
    Ok(())
}

fn command_key_list(arguments: Vec<OsString>) -> Result<()> {
    let parsed = parse_read_arguments("key list", arguments, false)?;
    if parsed.positionals.len() != 1 || parsed.unlock.is_none() {
        return Err(usage(
            "key list requires <archive.eb> --identity <file>|--password",
        ));
    }
    let path = PathBuf::from(&parsed.positionals[0]);
    let authenticated = open_encrypted_authenticated(
        &read(&path)?,
        EncryptedOpenOptions::new(parsed.unlock.as_ref().map(OwnedUnlock::borrowed)),
    )?;
    if authenticated.recipient_directory.is_empty() {
        println!("protection: PASSWORD_ONLY");
        println!("recipients: one password stanza (no public-key directory)");
    } else {
        println!("protection: HYBRID_ONLY");
        for entry in authenticated.recipient_directory {
            println!(
                "stanza={} type={} class=HYBRID_PQ fingerprint={} label={}",
                hex(&entry.stanza_id),
                entry.stanza_type,
                hex(&entry.fingerprint),
                if entry.label.is_empty() {
                    "-"
                } else {
                    &entry.label
                }
            );
        }
    }
    Ok(())
}

fn command_key_add(arguments: Vec<OsString>) -> Result<()> {
    let mutation = parse_key_mutation("key add", arguments, "--recipient")?;
    if mutation.keys.len() != 1 {
        return Err(usage(
            "key add requires exactly one --recipient <public-key>",
        ));
    }
    let identity = mutation
        .identity
        .as_ref()
        .ok_or_else(|| usage("key add requires --identity"))?;
    let recipient = XWingRecipient::read_file(&mutation.keys[0])?;
    let bytes = read(&mutation.archive)?;
    let replacement = add_recipient(
        &bytes,
        EncryptedOpenOptions::new(Some(Unlock::Identity(identity))),
        &recipient,
    )?;
    write_mutation_output(
        &mutation.archive,
        mutation.output.as_deref(),
        &replacement.bytes,
    )?;
    println!(
        "OK added recipient; AFK and archive ID preserved, addressing signatures may now be stale"
    );
    Ok(())
}

fn command_key_remove(arguments: Vec<OsString>) -> Result<()> {
    let mutation = parse_key_mutation("key remove", arguments, "--retain")?;
    if mutation.keys.is_empty() {
        return Err(usage(
            "key remove requires every retained public key via --retain",
        ));
    }
    let identity = mutation
        .identity
        .as_ref()
        .ok_or_else(|| usage("key remove requires --identity"))?;
    let retained = mutation
        .keys
        .iter()
        .map(|path| XWingRecipient::read_file(path))
        .collect::<Result<Vec<_>>>()?;
    let bytes = read(&mutation.archive)?;
    let replacement = reencrypt_recipients(
        &bytes,
        EncryptedOpenOptions::new(Some(Unlock::Identity(identity))),
        &retained,
    )?;
    write_mutation_output(
        &mutation.archive,
        mutation.output.as_deref(),
        &replacement.bytes,
    )?;
    println!("OK removed recipient through fresh-AFK full re-encryption");
    Ok(())
}

fn command_key_change_password(arguments: Vec<OsString>) -> Result<()> {
    let mut archive = None;
    let mut output = None;
    let mut old_password = false;
    let mut cursor = 0;
    while cursor < arguments.len() {
        if arguments[cursor] == "--password" {
            old_password = true;
        } else if arguments[cursor] == "--output" {
            cursor += 1;
            output = Some(PathBuf::from(
                arguments
                    .get(cursor)
                    .ok_or_else(|| usage("--output requires a path"))?,
            ));
        } else if arguments[cursor].to_string_lossy().starts_with("--") {
            return Err(usage("key change-password received an unknown option"));
        } else if archive.replace(PathBuf::from(&arguments[cursor])).is_some() {
            return Err(usage("key change-password accepts one archive"));
        }
        cursor += 1;
    }
    let archive =
        archive.ok_or_else(|| usage("key change-password requires <archive.eb> --password"))?;
    if !old_password {
        return Err(usage(
            "key change-password requires --password for the old password",
        ));
    }
    let old = Zeroizing::new(prompt_password("Current archive password: ")?);
    let new = Zeroizing::new(prompt_password("New archive password: ")?);
    let confirmation = Zeroizing::new(prompt_password("Confirm new archive password: ")?);
    if *new != *confirmation || new.is_empty() {
        return Err(usage(
            "new password confirmation did not match or was empty",
        ));
    }
    let replacement = change_password(
        &read(&archive)?,
        EncryptedOpenOptions::new(Some(Unlock::Password(old.as_bytes()))),
        new.as_bytes(),
    )?;
    write_mutation_output(&archive, output.as_deref(), &replacement.bytes)?;
    println!("OK changed password through fresh-AFK full re-encryption");
    Ok(())
}

struct KeyMutationArguments {
    archive: PathBuf,
    identity: Option<XWingIdentity>,
    keys: Vec<PathBuf>,
    output: Option<PathBuf>,
}

fn parse_key_mutation(
    command: &str,
    arguments: Vec<OsString>,
    key_flag: &str,
) -> Result<KeyMutationArguments> {
    let mut archive = None;
    let mut identity = None;
    let mut keys = Vec::new();
    let mut output = None;
    let mut cursor = 0;
    while cursor < arguments.len() {
        if arguments[cursor] == "--identity" {
            cursor += 1;
            let path = PathBuf::from(
                arguments
                    .get(cursor)
                    .ok_or_else(|| usage("--identity requires a file"))?,
            );
            warn_identity_permissions(&path);
            identity = Some(XWingIdentity::read_file(&path)?);
        } else if arguments[cursor] == key_flag {
            cursor += 1;
            keys.push(PathBuf::from(
                arguments
                    .get(cursor)
                    .ok_or_else(|| usage(format!("{key_flag} requires a file")))?,
            ));
        } else if arguments[cursor] == "--output" {
            cursor += 1;
            output = Some(PathBuf::from(
                arguments
                    .get(cursor)
                    .ok_or_else(|| usage("--output requires a path"))?,
            ));
        } else if arguments[cursor].to_string_lossy().starts_with("--") {
            return Err(usage(format!("{command} received an unknown option")));
        } else if archive.replace(PathBuf::from(&arguments[cursor])).is_some() {
            return Err(usage(format!("{command} accepts one archive")));
        }
        cursor += 1;
    }
    Ok(KeyMutationArguments {
        archive: archive.ok_or_else(|| usage(format!("{command} requires <archive.eb>")))?,
        identity,
        keys,
        output,
    })
}

fn load_unlock(
    identity: Option<PathBuf>,
    password: bool,
    prompt: &str,
) -> Result<Option<OwnedUnlock>> {
    if identity.is_some() && password {
        return Err(usage("--identity and --password cannot be combined"));
    }
    if let Some(path) = identity {
        warn_identity_permissions(&path);
        Ok(Some(OwnedUnlock::Identity(XWingIdentity::read_file(
            &path,
        )?)))
    } else if password {
        Ok(Some(OwnedUnlock::Password(Zeroizing::new(
            prompt_password(prompt)?,
        ))))
    } else {
        Ok(None)
    }
}

fn detached_path(archive: &Path) -> PathBuf {
    let mut value = archive.as_os_str().to_os_string();
    value.push(".ebsig");
    PathBuf::from(value)
}

fn write_mutation_output(source: &Path, output: Option<&Path>, bytes: &[u8]) -> Result<()> {
    if let Some(output) = output.filter(|output| *output != source) {
        let mut file = create_exclusive(output)?;
        file.write_all(bytes)
            .and_then(|()| file.sync_all())
            .map_err(|error| io_error("write replacement archive", &error))?;
        Ok(())
    } else {
        replace_verified(source, bytes)
    }
}

fn replace_verified(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let name = path
        .file_name()
        .ok_or_else(|| usage("archive path has no file name"))?;
    let temporary = parent.join(format!(
        ".{}.entrybound-tmp-{}",
        name.to_string_lossy(),
        std::process::id()
    ));
    let backup = parent.join(format!(
        ".{}.entrybound-backup-{}",
        name.to_string_lossy(),
        std::process::id()
    ));
    let mut file = create_exclusive(&temporary)?;
    if let Err(error) = file
        .write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|error| io_error("write verified replacement", &error))
    {
        drop(file);
        let _ = std::fs::remove_file(&temporary);
        return Err(error);
    }
    drop(file);
    std::fs::rename(path, &backup)
        .map_err(|error| io_error("preserve original archive", &error))?;
    if let Err(error) = std::fs::rename(&temporary, path) {
        let _ = std::fs::rename(&backup, path);
        let _ = std::fs::remove_file(&temporary);
        return Err(io_error("install replacement archive", &error));
    }
    std::fs::remove_file(&backup)
        .map_err(|error| io_error("remove replaced archive backup", &error))?;
    Ok(())
}

#[cfg(unix)]
fn restrict_secret_permissions(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt as _;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
        .map_err(|error| io_error("restrict signing-key permissions", &error))
}

#[cfg(not(unix))]
fn restrict_secret_permissions(_path: &Path) -> Result<()> {
    Ok(())
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
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
    let parsed = parse_read_arguments("inspect", arguments, true)?;
    if parsed.positionals.len() != 1 {
        return Err(usage(
            "inspect requires <archive.eb|-> [--crypto] [--identity <file>|--password]",
        ));
    }
    let source = Source::parse(&parsed.positionals[0]);
    let encrypted = match &source {
        Source::Path(path) => path_is_encrypted(path)?,
        Source::Stdin => parsed.unlock.is_some(),
    };
    if parsed.crypto && !encrypted {
        println!("encrypted: no");
    }
    let (view, stream) = if encrypted {
        let bytes = read_source_fully(&source)?;
        let authenticated_crypto = parsed
            .unlock
            .as_ref()
            .map(|unlock| {
                open_encrypted_authenticated(
                    &bytes,
                    EncryptedOpenOptions::new(Some(unlock.borrowed())),
                )
            })
            .transpose()?;
        let result = inspect_encrypted(&bytes, None, CryptoPolicy::default())?;
        println!("encrypted: yes");
        println!("payload suite: {}", result.public.payload_suite);
        println!("recipient count: {}", result.public.recipient_count);
        println!(
            "recipient stanza types: {}",
            result.public.recipient_types.join(", ")
        );
        println!("padding: {:?}", result.public.padding);
        if result.public.padding == PaddingMode::None {
            println!("privacy warning: encrypted-record padding is disabled");
        }
        println!("encrypted boundary mode: {:?}", result.public.boundary);
        println!(
            "encrypted segment count: {}",
            result
                .public
                .segment_count
                .expect("public encrypted scan reports segment count")
        );
        println!("container bytes: {}", result.public.total_container_bytes);
        let Some(authenticated_crypto) = authenticated_crypto else {
            println!(
                "private archive metadata: locked; supply --identity or --password for authenticated inspection"
            );
            return Ok(());
        };
        let authenticated = inspect(&authenticated_crypto.opened)?;
        let producer_declaration_present = u64::from_be_bytes(bytes[16..24].try_into().unwrap())
            & FEATURE_PRIVATE_RESOURCE_DECLARATION_V1
            != 0;
        println!(
            "encrypted Descriptor record version: {}",
            if producer_declaration_present { 2 } else { 1 }
        );
        if producer_declaration_present {
            println!("producer resource declaration: present");
            println!(
                "producer resource declaration validation: matches authenticated archive reality"
            );
            let budget = authenticated.declared_resources;
            let decode = authenticated.decode_requirements;
            println!(
                "declared resources: entries={} logical-bytes={} max-entry={} expansion-ratio-milli={} chunks={} path-depth={} metadata-bytes={} key-derivation-cost={}",
                budget.entry_count,
                budget.total_logical_bytes,
                budget.max_single_entry_logical_bytes,
                budget.max_expansion_ratio_milli,
                budget.chunk_count,
                budget.max_path_depth,
                budget.max_metadata_bytes,
                budget.max_key_derivation_cost
            );
            println!(
                "declared decode: window-bytes={} working-set-bytes={} flags={:#010x}",
                decode.window_bytes, decode.working_set_bytes, decode.flags
            );
        } else {
            println!("producer resource declaration: absent (legacy experimental crypto-v1)");
        }
        println!(
            "embedded signatures: {}",
            authenticated_crypto.embedded_signatures.len()
        );
        let current = current_bindings(
            &authenticated_crypto.opened,
            Some(authenticated_crypto.addressing),
        )?;
        let timestamp_policy = timestamp_policy(&parsed.timestamp_trust)?;
        let statuses = verify_signatures(
            &authenticated_crypto.embedded_signatures,
            &current,
            timestamp_policy.as_ref(),
        )?;
        print_signature_statuses(&statuses);
        (authenticated, None)
    } else {
        if parsed.unlock.is_some() {
            return Err(usage(
                "an identity/password was supplied for an unencrypted archive",
            ));
        }
        let loaded = load(&source, StreamContentPolicy::Verify)?;
        (inspect(&loaded.opened)?, loaded.stream)
    };
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
    if let Some(stream) = &stream {
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
    if let Some(conversion) = &view.conversion {
        println!(
            "conversion provenance: source={} adapter={} mode={} source-digest={} outcome={}",
            conversion.source_format,
            conversion.adapter_id,
            conversion.import_mode,
            conversion.source_digest,
            conversion.outcome,
        );
        println!(
            "conversion conflicts: omission={} refinement={} divergence={} irreconcilable={} resolutions={} synthesized-ancestors={}",
            conversion.omission_count,
            conversion.refinement_count,
            conversion.divergence_count,
            conversion.irreconcilable_count,
            conversion.resolutions.len(),
            conversion.synthesized_ancestors.len(),
        );
        if let Some((wrapper, _)) = conversion.source_format.split_once('+') {
            println!("conversion layers: {wrapper} -> tar");
        }
        if let Some(layer) = conversion
            .resolutions
            .iter()
            .find(|resolution| resolution.semantic_field == "layer.transport-decoded-child")
        {
            println!("conversion layer integrity: {}", layer.action);
            if conversion.adapter_id.ends_with("-stream-strict/v1") {
                println!("conversion projection: single-file ({})", layer.action);
            }
        }
        for resolution in conversion.resolutions.iter().take(16) {
            println!(
                "conversion resolution: class={} field={} action={}",
                resolution.conflict_class, resolution.semantic_field, resolution.action
            );
        }
    }
    if let Some(preservation) = &view.preservation {
        let opaque = preservation
            .observations
            .iter()
            .filter(|item| {
                matches!(
                    item.validity,
                    entrybound::eam::PreservedLegacyValidity::Uninterpreted
                )
            })
            .count();
        println!("preservation format: {}", preservation.preservation_format);
        println!(
            "preserved source: bytes={} digest={}",
            preservation.source_bytes.len(),
            preservation.source_digest
        );
        println!(
            "preservation evidence: observations={} conflicts={} opaque={} exact-source-recovery=yes",
            preservation.observations.len(),
            preservation.conflicts.len(),
            opaque
        );
    }
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

struct VerifyArguments {
    read: ReadArguments,
    embedded: bool,
    detached: Vec<PathBuf>,
    trust_anchors: Vec<PathBuf>,
    policy: SignaturePolicy,
}

fn parse_verify_arguments(arguments: Vec<OsString>) -> Result<VerifyArguments> {
    let mut ordinary = Vec::new();
    let mut embedded = false;
    let mut detached = Vec::new();
    let mut trust_anchors = Vec::new();
    let mut policy = SignaturePolicy::default();
    let mut cursor = 0;
    while cursor < arguments.len() {
        if arguments[cursor] == "--signatures" {
            embedded = true;
        } else if arguments[cursor] == "--signature" {
            cursor += 1;
            detached.push(PathBuf::from(
                arguments
                    .get(cursor)
                    .ok_or_else(|| usage("--signature requires an .ebsig file"))?,
            ));
        } else if arguments[cursor] == "--timestamp-trust" {
            cursor += 1;
            trust_anchors.push(PathBuf::from(
                arguments
                    .get(cursor)
                    .ok_or_else(|| usage("--timestamp-trust requires a DER certificate"))?,
            ));
        } else if arguments[cursor] == "--require-signature" {
            policy.require_signature = true;
        } else if arguments[cursor] == "--require-content-signature" {
            policy.require_content = true;
        } else if arguments[cursor] == "--require-physical-signature" {
            policy.require_physical = true;
        } else if arguments[cursor] == "--require-addressing-signature" {
            policy.require_addressing = true;
        } else {
            ordinary.push(arguments[cursor].clone());
        }
        cursor += 1;
    }
    Ok(VerifyArguments {
        read: parse_read_arguments("verify", ordinary, false)?,
        embedded,
        detached,
        trust_anchors,
        policy,
    })
}

fn command_verify(arguments: Vec<OsString>) -> Result<()> {
    let parsed = parse_verify_arguments(arguments)?;
    let read_arguments = &parsed.read;
    if read_arguments.positionals.len() != 1 {
        return Err(usage(
            "verify requires <archive.eb|-> [unlock] [signature policy/options]",
        ));
    }
    let source = Source::parse(&read_arguments.positionals[0]);
    let encrypted = match &source {
        Source::Path(path) => path_is_encrypted(path)?,
        Source::Stdin => read_arguments.unlock.is_some(),
    };
    let detached = parsed
        .detached
        .iter()
        .map(|path| read_detached_signature(path))
        .collect::<Result<Vec<_>>>()?;
    let timestamp_policy = timestamp_policy(&parsed.trust_anchors)?;
    if encrypted {
        let bytes = read_source_fully(&source)?;
        let Some(unlock) = read_arguments.unlock.as_ref().map(OwnedUnlock::borrowed) else {
            if parsed.embedded
                || !detached.is_empty()
                || parsed.policy != SignaturePolicy::default()
            {
                return Err(usage(
                    "encrypted signature evaluation requires --identity or --password",
                ));
            }
            let public = inspect_encrypted(&bytes, None, CryptoPolicy::default())?;
            println!(
                "PUBLIC framing valid: encrypted={}, suite={}, recipients={}, padding={:?}, boundary={:?}",
                public.public.encrypted,
                public.public.payload_suite,
                public.public.recipient_count,
                public.public.padding,
                public.public.boundary
            );
            println!(
                "PRIVATE CONTENT UNVERIFIED: supply --identity or --password to authenticate and verify the archive"
            );
            return Ok(());
        };
        let authenticated =
            open_encrypted_authenticated(&bytes, EncryptedOpenOptions::new(Some(unlock)))?;
        let current = current_bindings(&authenticated.opened, Some(authenticated.addressing))?;
        let mut signatures = detached;
        if parsed.embedded {
            signatures.extend(authenticated.embedded_signatures.iter().cloned());
        }
        let statuses = verify_signatures(&signatures, &current, timestamp_policy.as_ref())?;
        parsed.policy.enforce(&statuses)?;
        let report = authenticated.opened.report;
        println!(
            "OK authenticated and verified CryptoEnvelope commitment/MAC, every AEAD DATA/END record, segment order/finality, canonical private ECF, semantic invariants, Chunk/content integrity, LAI, PCR, AUX, and exact-byte PCI"
        );
        println!("index: {}", index_status(report.index_status));
        println!("LAI {}", report.identities.lai.0);
        println!("PCR {}", report.identities.pcr.0);
        println!("AUX {}", report.identities.aux.0);
        println!("PCI {}", report.identities.pci.0);
        print_signature_statuses(&statuses);
        return Ok(());
    }
    if read_arguments.unlock.is_some() {
        return Err(usage(
            "an identity/password was supplied for an unencrypted archive",
        ));
    }
    if parsed.embedded {
        return Err(Diagnostic::new(
            OutcomeClass::Unsupported,
            ReasonCode::SignatureUnsupported,
            "unencrypted ECF has no normative embedded-signature placement; supply .ebsig",
        ));
    }
    let (opened, stream) = match &source {
        Source::Stdin => {
            let sequential =
                open_stream_with_limits(std::io::stdin().lock(), bootstrap_sequential_limits())?;
            (sequential.opened, Some(sequential.stream))
        }
        Source::Path(path) if path_layout(path)? == Layout::Stream => {
            let file = File::open(path).map_err(|error| read_error(path, &error))?;
            let sequential = open_stream_with_limits(file, bootstrap_sequential_limits())?;
            (sequential.opened, Some(sequential.stream))
        }
        Source::Path(path) => (open(&read(path)?)?, None),
    };
    let current = current_bindings(&opened, None)?;
    let statuses = verify_signatures(&detached, &current, timestamp_policy.as_ref())?;
    parsed.policy.enforce(&statuses)?;
    let report = opened.report;
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
    print_signature_statuses(&statuses);
    Ok(())
}

fn timestamp_policy(paths: &[PathBuf]) -> Result<Option<TimestampPolicy>> {
    if paths.is_empty() {
        return Ok(None);
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| usage("system time precedes the Unix epoch"))?;
    Ok(Some(TimestampPolicy {
        trust_anchors: paths
            .iter()
            .map(|path| read(path).map(|der| TimestampTrustAnchor { der }))
            .collect::<Result<Vec<_>>>()?,
        verification_unix_seconds: i64::try_from(now.as_secs())
            .map_err(|_| usage("system time exceeds timestamp policy range"))?,
    }))
}

fn verify_signatures(
    signatures: &[SignatureRecord],
    current: &entrybound::crypto::CurrentBindings,
    timestamp: Option<&TimestampPolicy>,
) -> Result<Vec<SignatureStatus>> {
    signatures
        .iter()
        .map(|signature| verify_signature(signature, current, timestamp))
        .collect()
}

fn print_signature_statuses(statuses: &[SignatureStatus]) {
    if statuses.is_empty() {
        return;
    }
    for status in statuses {
        println!(
            "signature signer={} mask={:#04x} cryptographic={} content={} physical={} addressing={} timestamp={}{}",
            hex(&status.signer_id),
            status.binding_mask,
            cryptographic_status(status.cryptographic),
            binding_status(status.content),
            binding_status(status.physical),
            binding_status(status.addressing),
            timestamp_status(status.timestamp),
            status
                .timestamp_unix_seconds
                .map_or_else(String::new, |time| format!(" time={time}"))
        );
    }
}

const fn cryptographic_status(value: CryptographicStatus) -> &'static str {
    match value {
        CryptographicStatus::Valid => "VALID",
        CryptographicStatus::Invalid => "INVALID",
        CryptographicStatus::Unsupported => "UNSUPPORTED",
    }
}

const fn binding_status(value: BindingStatus) -> &'static str {
    match value {
        BindingStatus::Valid => "VALID",
        BindingStatus::Stale => "STALE",
        BindingStatus::NotBound => "NOT_BOUND",
    }
}

const fn timestamp_status(value: TimestampStatus) -> &'static str {
    match value {
        TimestampStatus::Valid => "VALID",
        TimestampStatus::Invalid => "INVALID",
        TimestampStatus::Unsupported => "UNSUPPORTED",
        TimestampStatus::Absent => "ABSENT",
    }
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

fn read_source_fully(source: &Source) -> Result<Vec<u8>> {
    match source {
        Source::Path(path) => read(path),
        Source::Stdin => {
            let mut bytes = Vec::new();
            std::io::Read::read_to_end(&mut std::io::stdin().lock(), &mut bytes)
                .map_err(|error| io_error("read encrypted archive from standard input", &error))?;
            Ok(bytes)
        }
    }
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
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

    fn args(values: &[&str]) -> Vec<OsString> {
        values.iter().map(OsString::from).collect()
    }

    fn store_zip(name: &[u8], content: &[u8]) -> Vec<u8> {
        const LOCAL: u32 = 0x0403_4b50;
        const CENTRAL: u32 = 0x0201_4b50;
        const EOCD: u32 = 0x0605_4b50;
        let crc = crc32fast::hash(content);
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&LOCAL.to_le_bytes());
        bytes.extend_from_slice(&20_u16.to_le_bytes());
        bytes.extend_from_slice(&0_u16.to_le_bytes());
        bytes.extend_from_slice(&0_u16.to_le_bytes());
        bytes.extend_from_slice(&0_u16.to_le_bytes());
        bytes.extend_from_slice(&0_u16.to_le_bytes());
        bytes.extend_from_slice(&crc.to_le_bytes());
        bytes.extend_from_slice(&(content.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&(content.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&(name.len() as u16).to_le_bytes());
        bytes.extend_from_slice(&0_u16.to_le_bytes());
        bytes.extend_from_slice(name);
        bytes.extend_from_slice(content);
        let central_offset = bytes.len() as u32;
        bytes.extend_from_slice(&CENTRAL.to_le_bytes());
        bytes.extend_from_slice(&0x0314_u16.to_le_bytes());
        bytes.extend_from_slice(&20_u16.to_le_bytes());
        bytes.extend_from_slice(&0_u16.to_le_bytes());
        bytes.extend_from_slice(&0_u16.to_le_bytes());
        bytes.extend_from_slice(&0_u16.to_le_bytes());
        bytes.extend_from_slice(&0_u16.to_le_bytes());
        bytes.extend_from_slice(&crc.to_le_bytes());
        bytes.extend_from_slice(&(content.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&(content.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&(name.len() as u16).to_le_bytes());
        bytes.extend_from_slice(&0_u16.to_le_bytes());
        bytes.extend_from_slice(&0_u16.to_le_bytes());
        bytes.extend_from_slice(&0_u16.to_le_bytes());
        bytes.extend_from_slice(&0_u16.to_le_bytes());
        bytes.extend_from_slice(&0_u32.to_le_bytes());
        bytes.extend_from_slice(&0_u32.to_le_bytes());
        bytes.extend_from_slice(name);
        let central_size = bytes.len() as u32 - central_offset;
        bytes.extend_from_slice(&EOCD.to_le_bytes());
        bytes.extend_from_slice(&0_u16.to_le_bytes());
        bytes.extend_from_slice(&0_u16.to_le_bytes());
        bytes.extend_from_slice(&1_u16.to_le_bytes());
        bytes.extend_from_slice(&1_u16.to_le_bytes());
        bytes.extend_from_slice(&central_size.to_le_bytes());
        bytes.extend_from_slice(&central_offset.to_le_bytes());
        bytes.extend_from_slice(&0_u16.to_le_bytes());
        bytes
    }

    fn store_tar(name: &[u8], content: &[u8]) -> Vec<u8> {
        fn octal(field: &mut [u8], value: u64) {
            field.fill(b'0');
            let text = format!("{value:o}");
            let start = field.len() - 1 - text.len();
            field[start..start + text.len()].copy_from_slice(text.as_bytes());
            field[field.len() - 1] = 0;
        }
        let mut header = [0_u8; 512];
        header[..name.len()].copy_from_slice(name);
        octal(&mut header[100..108], 0o100644);
        octal(&mut header[108..116], 0);
        octal(&mut header[116..124], 0);
        octal(&mut header[124..136], content.len() as u64);
        octal(&mut header[136..148], 1_700_000_000);
        header[148..156].fill(b' ');
        header[156] = b'0';
        header[257..263].copy_from_slice(b"ustar\0");
        header[263..265].copy_from_slice(b"00");
        let checksum = header.iter().map(|byte| u64::from(*byte)).sum::<u64>();
        let checksum = format!("{checksum:06o}\0 ");
        header[148..156].copy_from_slice(checksum.as_bytes());
        let mut bytes = header.to_vec();
        bytes.extend_from_slice(content);
        bytes.resize(bytes.len().div_ceil(512) * 512, 0);
        bytes.resize(bytes.len() + 1024, 0);
        bytes
    }

    fn gzip(content: &[u8]) -> Vec<u8> {
        use std::io::Write as _;

        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(content).unwrap();
        encoder.finish().unwrap()
    }

    fn xz(content: &[u8]) -> Vec<u8> {
        use std::io::Write as _;

        let mut encoder =
            lzma_rust2::XzWriter::new(Vec::new(), lzma_rust2::XzOptions::with_preset(1)).unwrap();
        encoder.write_all(content).unwrap();
        encoder.finish().unwrap()
    }

    fn bzip2(content: &[u8]) -> Vec<u8> {
        use std::io::Write as _;

        let mut encoder = bzip2::write::BzEncoder::new(Vec::new(), bzip2::Compression::fast());
        encoder.write_all(content).unwrap();
        encoder.finish().unwrap()
    }

    #[test]
    fn help_is_available() {
        assert!(run(args(&["ebound"])).is_ok());
    }

    #[test]
    fn strict_convert_verify_inspect_and_unpack_workflow() {
        let id = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let root =
            std::env::temp_dir().join(format!("entrybound-cli-zip-{}-{id}", std::process::id()));
        std::fs::create_dir(&root).unwrap();
        let input = root.join("source.bin");
        let archive = root.join("converted.eb");
        let restored = root.join("restored");
        std::fs::write(&input, store_zip(b"nested/file.txt", b"zip conversion")).unwrap();
        run(vec![
            OsString::from("ebound"),
            OsString::from("convert"),
            input.as_os_str().to_owned(),
            archive.as_os_str().to_owned(),
            OsString::from("--strict"),
            OsString::from("--profile"),
            OsString::from("fast"),
        ])
        .unwrap();
        run(vec![
            OsString::from("ebound"),
            OsString::from("verify"),
            archive.as_os_str().to_owned(),
        ])
        .unwrap();
        run(vec![
            OsString::from("ebound"),
            OsString::from("inspect"),
            archive.as_os_str().to_owned(),
        ])
        .unwrap();
        run(vec![
            OsString::from("ebound"),
            OsString::from("unpack"),
            archive.as_os_str().to_owned(),
            restored.as_os_str().to_owned(),
        ])
        .unwrap();
        assert_eq!(
            std::fs::read(restored.join("nested/file.txt")).unwrap(),
            b"zip conversion"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn tar_and_standalone_wrapper_convert_to_indexed_and_stream() {
        let id = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "entrybound-cli-tar-stream-{}-{id}",
            std::process::id()
        ));
        std::fs::create_dir(&root).unwrap();
        let tar_input = root.join("source.data");
        let tar_archive = root.join("tar.eb");
        let tar_restored = root.join("tar-restored");
        std::fs::write(&tar_input, store_tar(b"nested/file", b"tar bytes")).unwrap();
        run(vec![
            OsString::from("ebound"),
            OsString::from("convert"),
            tar_input.as_os_str().to_owned(),
            tar_archive.as_os_str().to_owned(),
            OsString::from("--strict"),
            OsString::from("--from=tar"),
            OsString::from("--profile"),
            OsString::from("fast"),
        ])
        .unwrap();
        run(vec![
            OsString::from("ebound"),
            OsString::from("unpack"),
            tar_archive.as_os_str().to_owned(),
            tar_restored.as_os_str().to_owned(),
        ])
        .unwrap();
        assert_eq!(
            std::fs::read(tar_restored.join("nested/file")).unwrap(),
            b"tar bytes"
        );

        let tar_bytes = store_tar(b"wrapped/file", b"wrapped tar bytes");
        let wrappers = [
            ("tar.gz", gzip(&tar_bytes), "indexed"),
            (
                "tar.zst",
                zstd::stream::encode_all(tar_bytes.as_slice(), 1).unwrap(),
                "indexed",
            ),
            ("tar.xz", xz(&tar_bytes), "stream"),
            ("tar.bz2", bzip2(&tar_bytes), "stream"),
        ];
        for (index, (format, source, layout)) in wrappers.into_iter().enumerate() {
            let input = root.join(format!("wrapped-{index}.data"));
            let archive = root.join(format!("wrapped-{index}.eb"));
            let restored = root.join(format!("wrapped-{index}-restored"));
            std::fs::write(&input, source).unwrap();
            run(vec![
                OsString::from("ebound"),
                OsString::from("convert"),
                input.as_os_str().to_owned(),
                archive.as_os_str().to_owned(),
                OsString::from("--strict"),
                OsString::from(format!("--from={format}")),
                OsString::from("--layout"),
                OsString::from(layout),
                OsString::from("--profile"),
                OsString::from("fast"),
            ])
            .unwrap();
            run(vec![
                OsString::from("ebound"),
                OsString::from("verify"),
                archive.as_os_str().to_owned(),
            ])
            .unwrap();
            run(vec![
                OsString::from("ebound"),
                OsString::from("inspect"),
                archive.as_os_str().to_owned(),
            ])
            .unwrap();
            run(vec![
                OsString::from("ebound"),
                OsString::from("unpack"),
                archive.as_os_str().to_owned(),
                restored.as_os_str().to_owned(),
            ])
            .unwrap();
            assert_eq!(
                std::fs::read(restored.join("wrapped/file")).unwrap(),
                b"wrapped tar bytes"
            );
        }

        let wrapped_input = root.join("payload.data");
        let stream_archive = root.join("payload.eb");
        let stream_restored = root.join("payload-restored");
        std::fs::write(&wrapped_input, gzip(b"standalone bytes")).unwrap();
        run(vec![
            OsString::from("ebound"),
            OsString::from("convert"),
            wrapped_input.as_os_str().to_owned(),
            stream_archive.as_os_str().to_owned(),
            OsString::from("--from=gzip"),
            OsString::from("--entry-name=payload.bin"),
            OsString::from("--layout"),
            OsString::from("stream"),
            OsString::from("--dry-run"),
        ])
        .unwrap();
        assert!(!stream_archive.exists());
        run(vec![
            OsString::from("ebound"),
            OsString::from("convert"),
            wrapped_input.as_os_str().to_owned(),
            stream_archive.as_os_str().to_owned(),
            OsString::from("--from=gzip"),
            OsString::from("--entry-name=payload.bin"),
            OsString::from("--layout"),
            OsString::from("stream"),
            OsString::from("--profile"),
            OsString::from("fast"),
        ])
        .unwrap();
        run(vec![
            OsString::from("ebound"),
            OsString::from("unpack"),
            stream_archive.as_os_str().to_owned(),
            stream_restored.as_os_str().to_owned(),
        ])
        .unwrap();
        assert_eq!(
            std::fs::read(stream_restored.join("payload.bin")).unwrap(),
            b"standalone bytes"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn compat_preserve_and_dry_run_workflows_are_explicit() {
        let id = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "entrybound-cli-zip-compat-{}-{id}",
            std::process::id()
        ));
        std::fs::create_dir(&root).unwrap();
        let input = root.join("source.zip");
        let dry_output = root.join("dry-run.eb");
        let archive = root.join("preserved.eb");
        std::fs::write(&input, store_zip(b"file", b"compatibility")).unwrap();

        run(vec![
            OsString::from("ebound"),
            OsString::from("convert"),
            input.as_os_str().to_owned(),
            dry_output.as_os_str().to_owned(),
            OsString::from("--compat=zip/java-zipfile@21.0.12.1"),
            OsString::from("--dry-run"),
            OsString::from("--profile"),
            OsString::from("fast"),
        ])
        .unwrap();
        assert!(!dry_output.exists());

        run(vec![
            OsString::from("ebound"),
            OsString::from("convert"),
            input.as_os_str().to_owned(),
            archive.as_os_str().to_owned(),
            OsString::from("--preserve"),
            OsString::from("--compat=zip/java-zipfile@21.0.12.1"),
            OsString::from("--profile"),
            OsString::from("fast"),
        ])
        .unwrap();
        let opened = entrybound::ecf::open(&std::fs::read(&archive).unwrap()).unwrap();
        assert_eq!(
            entrybound::legacy::zip::recover_preserved_source(&opened.archive)
                .unwrap()
                .as_ref(),
            std::fs::read(&input).unwrap()
        );
        assert!(opened.archive.preservation.is_some());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn preserve_requires_an_exact_compatibility_profile() {
        let error = run(args(&[
            "ebound",
            "convert",
            "source.zip",
            "output.eb",
            "--preserve",
        ]))
        .unwrap_err();
        assert_eq!(error.code(), ReasonCode::CommandUsage);
        let error = run(args(&[
            "ebound",
            "convert",
            "source.zip",
            "output.eb",
            "--compat=zip/java",
        ]))
        .unwrap_err();
        assert_eq!(error.code(), ReasonCode::UnsupportedRequiredFeature);
    }

    #[test]
    fn future_commands_fail_explicitly() {
        let error = run(args(&["ebound", "repack"])).unwrap_err();
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

    #[test]
    fn encrypted_stream_is_rejected_before_input_or_output_is_touched() {
        let error = run(args(&[
            "ebound",
            "pack",
            "input-does-not-exist",
            "-",
            "--layout",
            "stream",
            "--recipient",
            "recipient-does-not-exist",
        ]))
        .unwrap_err();
        assert_eq!(error.code(), ReasonCode::CryptoLayoutUnsupported);
    }

    #[test]
    fn password_and_hybrid_creation_cannot_be_mixed() {
        let error = run(args(&[
            "ebound",
            "pack",
            "input-does-not-exist",
            "output.eb",
            "--recipient",
            "recipient-does-not-exist",
            "--password",
        ]))
        .unwrap_err();
        assert_eq!(error.code(), ReasonCode::CryptoRecipientPolicyInvalid);
    }
}
