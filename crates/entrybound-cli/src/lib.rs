use std::env;
use std::ffi::{OsStr, OsString};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use entrybound::archive::{
    AclPolicy, ArchiveDiffReport, ConfinementMode, DiffChange, DiffIdentityStatus, DiffTier,
    ExtractionPolicy, IndexPolicy, InspectionSecurity, InspectionViews, OwnershipPolicy,
    PackOptions, PlatformMetadataPolicy, RepackMode, RepackOptions, ReparsePolicy, SparsePolicy,
    SymlinkPolicy, WindowsSecurityPolicy, XAttrPolicy, archive_diff, archive_metadata_diff,
    default_pack_output, default_unpack_destination, explain as compression_explain, inspect,
    inspection_json, inspection_json_with_security, list, plan_directory, prepare_repack,
    random_inspection_json, structured_explain, unpack, unpack_opened, unpack_stream,
};
use entrybound::crypto::{
    BindingStatus, BoundaryMode, CryptoPolicy, CryptographicStatus, EncryptedOpenOptions,
    EncryptedWriteOptions, FEATURE_ENCRYPTED_INDEXED_V1, FEATURE_PRIVATE_RESOURCE_DECLARATION_V1,
    PaddingMode, SignaturePolicy, SignatureRecord, SignatureStatus, SigningKey, TimestampPolicy,
    TimestampStatus, TimestampTrustAnchor, Unlock, XWingIdentity, XWingRecipient, add_recipient,
    change_password, current_bindings, embed_signature, inspect_encrypted,
    inspect_indexed_random_encrypted_public, open_encrypted, open_encrypted_authenticated,
    open_indexed_random_encrypted, pack_directory_encrypted, read_detached_signature,
    reencrypt_recipients, sign_archive, verify_signature,
};
use entrybound::diagnostics::{Diagnostic, OutcomeClass, ReasonCode, Result};
use entrybound::eam::{ArchiveRole, EntryKind, Layout, LogicalPath};
use entrybound::ecf::{
    IndexStatus, OpenedArchive, RandomAccessVerificationReport, SequentialLimits,
    StreamContentPolicy, StreamReport, StreamWindow, StreamWriteOptions, WriteOptions,
    bootstrap_sequential_limits, encode, encode_stream, open, open_indexed_random,
    open_stream_with_limits, peek_layout,
};
use entrybound::legacy::export::{
    ExportOutcome, ExportProfileId, ExportSourceSecurity, ExportTarget, prepare_export,
};
use entrybound::legacy::import::{
    LegacyConversionReport, LegacyImportPolicy, LegacyImportResult, LegacySourceFormat,
    detect as detect_legacy, import_strict as import_legacy_strict,
};
use entrybound::legacy::migration::{
    MigrationOutcome, MigrationReportV1, NativeArtifactReport, SidecarMigrationReport,
    prepare_migration, prepare_sidecar,
};
use entrybound::legacy::zip::{
    CompatibilityProfileId, ImportPolicy, ZipImportPolicy, import as import_zip,
};
use entrybound::planner::CompressionProfile;
use entrybound::random_access::{
    AccessTraceEntry, HttpRangeSource, LocalFileRandomReadSource, RandomAccessPolicy,
    RandomReadSource,
};
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
                         [--from zip|7z|tar|gzip|zstd|xz|bzip2|tar.gz|tar.zst|tar.xz|tar.bz2]\n\
                         [--entry-name <logical-path>]\n\
                         [--layout indexed|stream]\n\
                         [--profile fast|balanced|dense|extreme]\n\
  ebound convert <archive.eb|-> <legacy-output|-> --to <target>\n\
                         [--target-profile <versioned-profile>]\n\
                         [--dry-run] [--allow-lossy] [--receipt <file>]\n\
                         [--identity <file>|--password]\n\
  ebound publish <archive.eb|directory> --output-dir <directory>\n\
                         [--native] [--target <target> ...]\n\
                         [--base-name <name>] [--allow-lossy] [--dry-run]\n\
                         [--report <file>] [--identity <file>|--password]\n\
                         [--layout indexed|stream]\n\
                         [--profile fast|balanced|dense|extreme]\n\
  ebound sidecar <legacy-artifact> [output.eb]\n\
                         [--strict|--compat=<versioned-profile>]\n\
                         [--preserve --compat=<versioned-profile>]\n\
                         [--from <format>] [--entry-name <logical-path>]\n\
                         [--layout indexed|stream] [--profile <profile>]\n\
                         [--report <file>]\n\
  ebound unpack <archive.eb|-> [destination] [--identity <file>|--password]\n\
                [--symlinks refuse|safe|all] [--restore-owner]\n\
                [--xattrs ignore|restore] [--sparse logical|restore]\n\
                [--acls ignore|restore] [--windows-security ignore|restore]\n\
                [--reparse refuse|known-safe|all] [--platform-metadata ignore|restore]\n\
  ebound read <archive.eb|URL> <logical-path> [--output <file|->]\n\
                               [--identity <file>|--password] [--access-report]\n\
  ebound list <archive.eb|URL|-> [--identity <file>|--password]\n\
  ebound repack <source.eb> <output.eb> [--layout indexed|stream]\n\
                 [--profile fast|balanced|dense|extreme]\n\
                 [--index preserve|present|absent] [--stream-window <n>|auto] [--dry-run]\n\
  ebound diff <left.eb|URL> <right.eb|URL> [--json] [--public]\n\
                 [--left-identity <file>|--left-password]\n\
                 [--right-identity <file>|--right-password]\n\
  ebound inspect <archive.eb|URL|-> [--json] [--entries|--plans|--chunks]\n\
                 [--reconstruction|--provenance|--security|--access]\n\
                 [--crypto] [--identity <file>|--password]\n\
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
  ebound explain <archive.eb|-> [logical-path]\n\
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
Usage (import): ebound convert <input> <output.eb|->\n\
                      [--strict | --compat=<versioned-profile>]\n\
                      [--preserve --compat=<versioned-profile>] [--dry-run]\n\
                      [--from zip|7z|tar|gzip|zstd|xz|bzip2|tar.gz|tar.zst|tar.xz|tar.bz2]\n\
                      [--entry-name <logical-path>]\n\
                      [--layout indexed|stream]\n\
                      [--profile fast|balanced|dense|extreme]\n\
\n\
Usage (export): ebound convert <archive.eb|-> <legacy-output|->\n\
                      --to zip|tar|tar.gz|tar.zst|tar.xz|tar.bz2\n\
                      [--target-profile <exact-versioned-profile>]\n\
                      [--dry-run] [--allow-lossy] [--receipt <file>]\n\
                      [--identity <file>|--password]\n\
\n\
ZIP modes retain independent central, local, and descriptor observations. Strict\n\
7z validates plain/encoded headers, folder graphs, solid streams, and its bounded\n\
codec/filter subset. Strict tar supports ustar, pax, GNU long-name, and base-256\n\
evidence. gzip, Zstandard,\n\
XZ, and bzip2 are bounded transport layers whose decoded children use the same\n\
tar adapter. A non-tar stream requires --entry-name. ZIP compatibility and\n\
preservation remain available only through exact versioned ZIP profiles.\n\
Export authenticates and verifies the source EAM, then completes a typed\n\
LOSSLESS/LOSSY/REFUSED preflight before creating output. zip/portable-v1,\n\
tar/pax-v1, and the deterministic compressed-tar profiles are strict-reimported\n\
before publication; LOSSY output requires --allow-lossy.\n";

const PUBLISH_HELP: &str = "\
Usage: ebound publish <archive.eb|directory> --output-dir <directory>\n\
               [--native] [--target zip|tar|tar.gz|tar.zst|tar.xz|tar.bz2 ...]\n\
               [--base-name <name>] [--allow-lossy] [--dry-run]\n\
               [--report <migration.json>] [--identity <file>|--password]\n\
               [--layout indexed|stream] [--profile fast|balanced|dense|extreme]\n\
\n\
The source is verified/planned once. Every target is fully analyzed, encoded,\n\
and strict-reimported before any final output name is published. All temporary\n\
artifacts are removed and newly published names rolled back if the transaction\n\
cannot complete. Target order does not affect artifact or report bytes.\n";

const SIDECAR_HELP: &str = "\
Usage: ebound sidecar <legacy-artifact> [output.eb]\n\
               [--strict|--compat=<versioned-zip-profile>]\n\
               [--preserve --compat=<versioned-zip-profile>]\n\
               [--from <format>] [--entry-name <logical-path>]\n\
               [--layout indexed|stream] [--profile <profile>]\n\
               [--report <migration.json>]\n\
\n\
The default output is <legacy-artifact>.eb. The source is never modified. The\n\
sidecar is reopened and verified, including its ConversionProvenance binding to\n\
the exact source SHA-256, before its final name is published.\n";

const REPACK_HELP: &str = "\
Usage: ebound repack <source.eb> <output.eb> [--layout indexed|stream]\n\
               [--profile fast|balanced|dense|extreme]\n\
               [--index preserve|present|absent]\n\
               [--stream-window <n>|auto] [--dry-run]\n\
\n\
With no --profile, recorded Chunk boundaries, plans, dictionaries, groups,\n\
reconstruction, and physical order are retained and LAI/AUX/PCR must remain\n\
equal. Supplying --profile performs a current-v6 replan and requires LAI/AUX\n\
equality. Encrypted repack is deliberately unsupported.\n";

const DIFF_HELP: &str = "\
Usage: ebound diff <left.eb|URL> <right.eb|URL> [--json] [--public]\n\
               [--left-identity <file>|--left-password]\n\
               [--right-identity <file>|--right-password]\n\
\n\
Reports SEMANTIC (LAI), AUXILIARY (AUX), PHYSICAL (PCR), and CONTAINER/PCI\n\
changes separately. URL inputs use verified range-backed metadata and report\n\
PCR as NOT_VERIFIED and PCI as NOT_COMPUTED unless fully read.\n";

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
        "publish" => command_publish(arguments.collect()),
        "sidecar" => command_sidecar(arguments.collect()),
        "unpack" => command_unpack(arguments.collect()),
        "read" => command_read(arguments.collect()),
        "list" => command_list(arguments.collect()),
        "repack" => command_repack(arguments.collect()),
        "diff" => command_diff(arguments.collect()),
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
// deterministic legacy export
// ---------------------------------------------------------------------------

fn command_export(arguments: Vec<OsString>) -> Result<()> {
    let mut positionals = Vec::new();
    let mut target = None;
    let mut target_profile = None;
    let mut dry_run = false;
    let mut allow_lossy = false;
    let mut receipt = None;
    let mut identity = None;
    let mut password = false;
    let mut cursor = 0;
    while cursor < arguments.len() {
        let value = &arguments[cursor];
        if let Some(name) = value.to_str().and_then(|value| value.strip_prefix("--to=")) {
            if target.is_some() {
                return Err(usage("export accepts --to only once"));
            }
            target = Some(name.parse::<ExportTarget>()?);
        } else if value == "--to" {
            if target.is_some() {
                return Err(usage("export accepts --to only once"));
            }
            cursor += 1;
            target = Some(
                arguments
                    .get(cursor)
                    .and_then(|value| value.to_str())
                    .ok_or_else(|| usage("--to requires a supported legacy target"))?
                    .parse::<ExportTarget>()?,
            );
        } else if let Some(name) = value
            .to_str()
            .and_then(|value| value.strip_prefix("--target-profile="))
        {
            if target_profile.is_some() {
                return Err(usage("export accepts --target-profile only once"));
            }
            target_profile = Some(ExportProfileId::parse(name)?);
        } else if value == "--target-profile" {
            if target_profile.is_some() {
                return Err(usage("export accepts --target-profile only once"));
            }
            cursor += 1;
            target_profile = Some(ExportProfileId::parse(
                arguments
                    .get(cursor)
                    .and_then(|value| value.to_str())
                    .ok_or_else(|| usage("--target-profile requires a versioned profile"))?,
            )?);
        } else if value == "--dry-run" {
            if dry_run {
                return Err(usage("export accepts --dry-run only once"));
            }
            dry_run = true;
        } else if value == "--allow-lossy" {
            if allow_lossy {
                return Err(usage("export accepts --allow-lossy only once"));
            }
            allow_lossy = true;
        } else if value == "--receipt" {
            if receipt.is_some() {
                return Err(usage("export accepts --receipt only once"));
            }
            cursor += 1;
            receipt = Some(PathBuf::from(
                arguments
                    .get(cursor)
                    .ok_or_else(|| usage("--receipt requires a path"))?,
            ));
        } else if value == "--identity" {
            if identity.is_some() {
                return Err(usage("export accepts --identity only once"));
            }
            cursor += 1;
            identity =
                Some(PathBuf::from(arguments.get(cursor).ok_or_else(|| {
                    usage("--identity requires an identity key file")
                })?));
        } else if value == "--password" {
            if password {
                return Err(usage("export accepts --password only once"));
            }
            password = true;
        } else if value.to_string_lossy().starts_with("--") {
            return Err(usage(format!(
                "export does not recognize option '{}'",
                value.to_string_lossy()
            )));
        } else {
            positionals.push(value.clone());
        }
        cursor += 1;
    }
    if positionals.len() != 2 {
        return Err(usage(
            "export requires <archive.eb|-> <target|-> --to <legacy-target>",
        ));
    }
    if password && identity.is_some() {
        return Err(usage("--identity and --password cannot be combined"));
    }
    let selected = match (target, target_profile.as_ref()) {
        (Some(target), Some(profile)) if target != profile.target()? => {
            return Err(usage("--to and --target-profile select different targets"));
        }
        (Some(target), _) => target,
        (None, Some(profile)) => profile.target()?,
        (None, None) => return Err(usage("export requires --to or --target-profile")),
    };
    let source = Source::parse(&positionals[0]);
    let destination = if positionals[1] == OsStr::new("-") {
        Destination::Stdout
    } else {
        Destination::Path(PathBuf::from(&positionals[1]))
    };
    if dry_run && receipt.is_some() {
        return Err(usage("--receipt is unavailable during --dry-run"));
    }
    if matches!(destination, Destination::Stdout) && receipt.is_some() {
        return Err(usage(
            "--receipt with target stdout is unsupported because output cannot be rolled back",
        ));
    }
    if let (Destination::Path(target_path), Some(receipt_path)) = (&destination, &receipt)
        && target_path == receipt_path
    {
        return Err(usage("target and receipt paths must differ"));
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
    let source_bytes = read_source_fully(&source)?;
    let (opened, source_security) = open_export_source(&source_bytes, unlock.as_ref())?;
    let prepared = prepare_export(&opened.archive, selected, source_security)?;
    let status = Status {
        to_stderr: matches!(destination, Destination::Stdout),
    };
    print_export_analysis(&status, &prepared.analysis);
    if dry_run {
        if prepared.analysis.outcome == ExportOutcome::Refused {
            return prepared.accept(allow_lossy).map(|_| ());
        }
        status.line("OK dry-run completed full export preflight; no target bytes written");
        return Ok(());
    }
    let artifact = prepared.accept(allow_lossy)?;
    if source_security.encrypted {
        status.line("warning: authenticated encrypted source is being exported to an unencrypted legacy target");
    }
    write_export_transaction(
        &destination,
        receipt.as_deref(),
        &artifact.bytes,
        &artifact.receipt.to_canonical_json(),
    )?;
    status.line(format!(
        "OK exported {} entries to {} using {}",
        artifact.receipt.entry_count,
        describe(&destination),
        artifact.receipt.target_profile
    ));
    status.line(format!(
        "target bytes: {}",
        artifact.receipt.target_byte_length
    ));
    status.line(format!(
        "target SHA-256: {}",
        artifact.receipt.target_sha256
    ));
    if let Some(path) = receipt {
        status.line(format!("receipt: {}", path.display()));
    }
    Ok(())
}

fn open_export_source(
    bytes: &[u8],
    unlock: Option<&OwnedUnlock>,
) -> Result<(OpenedArchive, ExportSourceSecurity)> {
    if bytes_are_encrypted(bytes) {
        let unlock = unlock
            .map(OwnedUnlock::borrowed)
            .ok_or_else(|| usage("encrypted export requires --identity or --password"))?;
        let authenticated =
            open_encrypted_authenticated(bytes, EncryptedOpenOptions::new(Some(unlock)))?;
        let current = current_bindings(&authenticated.opened, Some(authenticated.addressing))?;
        let statuses = authenticated
            .embedded_signatures
            .iter()
            .map(|signature| verify_signature(signature, &current, None))
            .collect::<Result<Vec<_>>>()?;
        let mut security = ExportSourceSecurity {
            encrypted: true,
            embedded_signature_count: u64::try_from(statuses.len()).unwrap_or(u64::MAX),
            ..ExportSourceSecurity::default()
        };
        for signature in statuses {
            match signature.cryptographic {
                CryptographicStatus::Valid => security.signatures_valid += 1,
                CryptographicStatus::Invalid | CryptographicStatus::Unsupported => {
                    security.signatures_invalid += 1;
                }
            }
            if [signature.content, signature.physical, signature.addressing]
                .contains(&BindingStatus::Stale)
            {
                security.signatures_stale += 1;
            }
        }
        return Ok((authenticated.opened, security));
    }
    if unlock.is_some() {
        return Err(usage(
            "an identity/password was supplied for an unencrypted archive",
        ));
    }
    if peek_layout(bytes).unwrap_or(Layout::Indexed) == Layout::Stream {
        let limits = SequentialLimits {
            content: StreamContentPolicy::Retain,
            ..bootstrap_sequential_limits()
        };
        let sequential = open_stream_with_limits(std::io::Cursor::new(bytes), limits)?;
        Ok((sequential.opened, ExportSourceSecurity::default()))
    } else {
        Ok((open(bytes)?, ExportSourceSecurity::default()))
    }
}

fn bytes_are_encrypted(bytes: &[u8]) -> bool {
    bytes.len() >= 24
        && &bytes[..8] == entrybound::ecf::MAGIC.as_slice()
        && u64::from_be_bytes(bytes[16..24].try_into().expect("checked length"))
            & FEATURE_ENCRYPTED_INDEXED_V1
            != 0
}

fn print_export_analysis(status: &Status, analysis: &entrybound::legacy::export::ExportAnalysis) {
    status.line(format!("target profile: {}", analysis.profile.as_str()));
    status.line(format!("outcome: {}", analysis.outcome.as_str()));
    status.line(format!("entries: {}", analysis.entry_count));
    status.line(format!("logical bytes: {}", analysis.total_logical_bytes));
    if let Some(bytes) = analysis.planned_target_bytes {
        status.line(format!("planned target bytes: {bytes}"));
    }
    for issue in &analysis.issues {
        status.line(format!(
            "issue {} {} {}: {}",
            issue.category.as_str(),
            issue.disposition.as_str(),
            issue
                .entry
                .as_ref()
                .map_or_else(|| "archive".to_owned(), ToString::to_string),
            issue.reason
        ));
    }
}

fn write_export_transaction(
    destination: &Destination,
    receipt: Option<&Path>,
    target_bytes: &[u8],
    receipt_bytes: &[u8],
) -> Result<()> {
    match destination {
        Destination::Stdout => {
            let stdout = std::io::stdout();
            let mut handle = stdout.lock();
            handle
                .write_all(target_bytes)
                .map_err(|error| io_error("write exported target to standard output", &error))?;
            handle
                .flush()
                .map_err(|error| io_error("flush exported target", &error))
        }
        Destination::Path(path) => {
            let mut target = create_exclusive(path)?;
            if let Err(error) = target
                .write_all(target_bytes)
                .and_then(|()| target.sync_all())
                .map_err(|error| io_error("write exported target", &error))
            {
                drop(target);
                let _ = std::fs::remove_file(path);
                return Err(error);
            }
            drop(target);
            if let Some(receipt_path) = receipt {
                let receipt_result = (|| {
                    let mut file = create_exclusive(receipt_path)?;
                    file.write_all(receipt_bytes)
                        .and_then(|()| file.sync_all())
                        .map_err(|error| io_error("write export receipt", &error))
                })();
                if let Err(error) = receipt_result {
                    let _ = std::fs::remove_file(path);
                    let _ = std::fs::remove_file(receipt_path);
                    return Err(error);
                }
            }
            Ok(())
        }
    }
}

// ---------------------------------------------------------------------------
// aggregate migration publishing
// ---------------------------------------------------------------------------

fn command_publish(arguments: Vec<OsString>) -> Result<()> {
    if arguments.len() == 1 && matches!(arguments[0].to_str(), Some("--help" | "-h")) {
        print!("{PUBLISH_HELP}");
        return Ok(());
    }
    let mut positionals = Vec::new();
    let mut output_dir = None;
    let mut native = false;
    let mut targets = Vec::new();
    let mut base_name = None;
    let mut allow_lossy = false;
    let mut dry_run = false;
    let mut report_path = None;
    let mut identity = None;
    let mut password = false;
    let mut layout = None;
    let mut profile = None;
    let mut cursor = 0;
    while cursor < arguments.len() {
        let value = &arguments[cursor];
        if value == "--output-dir" {
            cursor += 1;
            set_once_path(
                &mut output_dir,
                arguments.get(cursor),
                "--output-dir requires a directory",
            )?;
        } else if value == "--native" {
            if native {
                return Err(usage("publish accepts --native only once"));
            }
            native = true;
        } else if value == "--target" {
            cursor += 1;
            targets.push(
                arguments
                    .get(cursor)
                    .and_then(|candidate| candidate.to_str())
                    .ok_or_else(|| usage("--target requires a supported target"))?
                    .parse::<ExportTarget>()?,
            );
        } else if let Some(candidate) = value
            .to_str()
            .and_then(|value| value.strip_prefix("--target="))
        {
            targets.push(candidate.parse::<ExportTarget>()?);
        } else if value == "--base-name" {
            cursor += 1;
            if base_name.is_some() {
                return Err(usage("publish accepts --base-name only once"));
            }
            base_name = Some(
                arguments
                    .get(cursor)
                    .and_then(|candidate| candidate.to_str())
                    .ok_or_else(|| usage("--base-name requires UTF-8 text"))?
                    .to_owned(),
            );
        } else if let Some(candidate) = value
            .to_str()
            .and_then(|value| value.strip_prefix("--base-name="))
        {
            if base_name.is_some() {
                return Err(usage("publish accepts --base-name only once"));
            }
            base_name = Some(candidate.to_owned());
        } else if value == "--allow-lossy" {
            if allow_lossy {
                return Err(usage("publish accepts --allow-lossy only once"));
            }
            allow_lossy = true;
        } else if value == "--dry-run" {
            if dry_run {
                return Err(usage("publish accepts --dry-run only once"));
            }
            dry_run = true;
        } else if value == "--report" {
            cursor += 1;
            set_once_path(
                &mut report_path,
                arguments.get(cursor),
                "--report requires a path",
            )?;
        } else if value == "--identity" {
            cursor += 1;
            set_once_path(
                &mut identity,
                arguments.get(cursor),
                "--identity requires an identity key file",
            )?;
        } else if value == "--password" {
            if password {
                return Err(usage("publish accepts --password only once"));
            }
            password = true;
        } else if value == "--layout" {
            if layout.is_some() {
                return Err(usage("publish accepts --layout only once"));
            }
            cursor += 1;
            layout = Some(parse_layout_option(arguments.get(cursor))?);
        } else if value == "--profile" {
            if profile.is_some() {
                return Err(usage("publish accepts --profile only once"));
            }
            cursor += 1;
            profile = Some(
                arguments
                    .get(cursor)
                    .and_then(|candidate| candidate.to_str())
                    .ok_or_else(|| usage("--profile requires a UTF-8 profile name"))?
                    .parse::<CompressionProfile>()?,
            );
        } else if value.to_string_lossy().starts_with("--") {
            return Err(usage(format!(
                "publish does not recognize option '{}'",
                value.to_string_lossy()
            )));
        } else {
            positionals.push(value.clone());
        }
        cursor += 1;
    }
    if positionals.len() != 1 {
        return Err(usage("publish requires exactly one <archive.eb|directory>"));
    }
    let output_dir = output_dir.ok_or_else(|| usage("publish requires --output-dir"))?;
    if !output_dir.is_dir() {
        return Err(usage("--output-dir must name an existing directory"));
    }
    if !native && targets.is_empty() {
        return Err(usage(
            "publish requires --native and/or at least one --target",
        ));
    }
    if password && identity.is_some() {
        return Err(usage("--identity and --password cannot be combined"));
    }
    let source_path = PathBuf::from(&positionals[0]);
    if source_path == Path::new("-") {
        return Err(usage("multi-target publish does not accept standard input"));
    }
    let base_name = base_name.unwrap_or_else(|| default_publish_base(&source_path));
    validate_publish_base(&base_name)?;
    let unlock = load_owned_unlock(identity.as_deref(), password, "Archive password: ")?;

    let directory_source = source_path.is_dir();
    let source_bytes = if directory_source {
        Vec::new()
    } else {
        read(&source_path)?
    };
    let opened;
    let source_security;
    let planned;
    let archive = if directory_source {
        if unlock.is_some() {
            return Err(usage(
                "--identity/--password applies only to an encrypted .eb source",
            ));
        }
        planned = entrybound::identity::apply_native_identities(&plan_directory(
            &source_path,
            PackOptions {
                profile: profile.unwrap_or_default(),
                ..PackOptions::default()
            },
        )?)?
        .0;
        source_security = ExportSourceSecurity::default();
        &planned
    } else {
        if profile.is_some() || layout.is_some() {
            return Err(usage(
                "--profile/--layout apply only when publishing a directory",
            ));
        }
        let source = open_export_source(&source_bytes, unlock.as_ref())?;
        opened = source.0;
        source_security = source.1;
        &opened.archive
    };

    let mut prepared = prepare_migration(archive, &targets, source_security, allow_lossy)?;
    for target in &mut prepared.report.requested_targets {
        target.output_path = output_dir
            .join(format!("{base_name}.{}", target.target.extension()))
            .display()
            .to_string();
    }

    let mut pending = Vec::<(PathBuf, Vec<u8>)>::new();
    if native {
        let native_path = output_dir.join(format!("{base_name}.eb"));
        if directory_source {
            let native_layout = layout.unwrap_or(Layout::Indexed);
            let bytes = encode_native_archive(archive, native_layout)?;
            verify_native_bytes(&bytes, native_layout, archive.descriptor.lai)?;
            prepared.report.native_artifact = Some(NativeArtifactReport {
                output_path: native_path.display().to_string(),
                relation: "encoded-from-same-source-eam".to_owned(),
                byte_length: u64::try_from(bytes.len())
                    .map_err(|_| usage("native artifact length exceeds u64"))?,
                sha256: entrybound::identity::sha256_exact(&bytes),
                produced: false,
            });
            pending.push((native_path, bytes));
        } else if native_path == source_path {
            prepared.report.native_artifact = Some(NativeArtifactReport {
                output_path: native_path.display().to_string(),
                relation: "verified-source-in-place".to_owned(),
                byte_length: u64::try_from(source_bytes.len())
                    .map_err(|_| usage("native artifact length exceeds u64"))?,
                sha256: entrybound::identity::sha256_exact(&source_bytes),
                produced: true,
            });
        } else {
            prepared.report.native_artifact = Some(NativeArtifactReport {
                output_path: native_path.display().to_string(),
                relation: "exact-verified-source-copy".to_owned(),
                byte_length: u64::try_from(source_bytes.len())
                    .map_err(|_| usage("native artifact length exceeds u64"))?,
                sha256: entrybound::identity::sha256_exact(&source_bytes),
                produced: false,
            });
            pending.push((native_path, source_bytes.clone()));
        }
    }
    for (target, artifact) in &prepared.artifacts {
        let path = output_dir.join(format!("{base_name}.{}", target.extension()));
        pending.push((path, artifact.bytes.clone()));
    }
    validate_transaction_paths(&pending, report_path.as_deref())?;

    if dry_run {
        println!(
            "{}",
            String::from_utf8_lossy(&prepared.report.to_canonical_json()).trim_end()
        );
        if !prepared.is_ready() {
            return Err(migration_not_ready(prepared.report.overall_outcome));
        }
        println!("OK dry-run completed aggregate migration preflight; no artifacts written");
        return Ok(());
    }
    if !prepared.is_ready() {
        return Err(migration_not_ready(prepared.report.overall_outcome));
    }
    prepared.mark_published();
    if let Some(native) = &mut prepared.report.native_artifact {
        native.produced = true;
    }
    if let Some(path) = &report_path {
        pending.push((path.clone(), prepared.report.to_canonical_json()));
    }
    transactional_publish(&pending)?;
    println!(
        "OK published {} artifact(s) from one verified EAM",
        pending.len()
    );
    println!("source LAI {}", prepared.report.source_lai);
    if source_security.encrypted && !prepared.artifacts.is_empty() {
        println!(
            "warning: legacy targets are unencrypted; the native .eb retains Entrybound crypto/signature state"
        );
    }
    if let Some(path) = report_path {
        println!("migration report: {}", path.display());
    }
    Ok(())
}

fn set_once_path(
    slot: &mut Option<PathBuf>,
    candidate: Option<&OsString>,
    missing: &str,
) -> Result<()> {
    if slot.is_some() {
        return Err(usage("option may be supplied only once"));
    }
    *slot = Some(PathBuf::from(candidate.ok_or_else(|| usage(missing))?));
    Ok(())
}

fn parse_layout_option(candidate: Option<&OsString>) -> Result<Layout> {
    match candidate.and_then(|value| value.to_str()) {
        Some("indexed") => Ok(Layout::Indexed),
        Some("stream") => Ok(Layout::Stream),
        _ => Err(usage("--layout requires 'indexed' or 'stream'")),
    }
}

fn load_owned_unlock(
    identity: Option<&Path>,
    password: bool,
    prompt: &str,
) -> Result<Option<OwnedUnlock>> {
    if let Some(path) = identity {
        warn_identity_permissions(path);
        return Ok(Some(OwnedUnlock::Identity(XWingIdentity::read_file(path)?)));
    }
    if password {
        return Ok(Some(OwnedUnlock::Password(Zeroizing::new(
            prompt_password(prompt)?,
        ))));
    }
    Ok(None)
}

fn default_publish_base(source: &Path) -> String {
    if source.is_dir() {
        source
            .file_name()
            .and_then(OsStr::to_str)
            .unwrap_or("release")
            .to_owned()
    } else {
        source
            .file_stem()
            .and_then(OsStr::to_str)
            .unwrap_or("release")
            .to_owned()
    }
}

fn validate_publish_base(base: &str) -> Result<()> {
    if base.is_empty()
        || matches!(base, "." | "..")
        || base
            .chars()
            .any(|character| matches!(character, '/' | '\\' | ':' | '\0'))
    {
        return Err(usage(
            "--base-name must be one safe filename stem without path separators or colon",
        ));
    }
    Ok(())
}

fn encode_native_archive(archive: &entrybound::eam::Archive, layout: Layout) -> Result<Vec<u8>> {
    match layout {
        Layout::Indexed => Ok(encode(archive, WriteOptions::default())?.bytes),
        Layout::Stream => {
            let mut bytes = Vec::new();
            encode_stream(archive, StreamWriteOptions::default(), &mut bytes)?;
            Ok(bytes)
        }
    }
}

fn verify_native_bytes(
    bytes: &[u8],
    layout: Layout,
    expected_lai: entrybound::eam::Digest,
) -> Result<()> {
    let actual = match layout {
        Layout::Indexed => open(bytes)?.archive.descriptor.lai,
        Layout::Stream => {
            let limits = SequentialLimits {
                content: StreamContentPolicy::Retain,
                ..bootstrap_sequential_limits()
            };
            open_stream_with_limits(std::io::Cursor::new(bytes), limits)?
                .opened
                .archive
                .descriptor
                .lai
        }
    };
    if actual != expected_lai {
        return Err(Diagnostic::new(
            OutcomeClass::Corrupt,
            ReasonCode::LaiMismatch,
            "native publication validation changed the source LAI",
        ));
    }
    Ok(())
}

fn migration_not_ready(outcome: MigrationOutcome) -> Diagnostic {
    match outcome {
        MigrationOutcome::LossyApprovalRequired => Diagnostic::new(
            OutcomeClass::PolicyRefused,
            ReasonCode::LegacyExportLossyApprovalRequired,
            "one or more requested targets are LOSSY; no artifacts were written",
        ),
        MigrationOutcome::Refused => Diagnostic::new(
            OutcomeClass::Unsupported,
            ReasonCode::LegacyExportRefused,
            "one or more requested targets are REFUSED; no artifacts were written",
        ),
        _ => Diagnostic::new(
            OutcomeClass::Nonconforming,
            ReasonCode::LegacyExportTargetInvalid,
            "migration is not ready for publication",
        ),
    }
}

fn validate_transaction_paths(
    artifacts: &[(PathBuf, Vec<u8>)],
    report: Option<&Path>,
) -> Result<()> {
    let mut paths = std::collections::BTreeSet::new();
    for path in artifacts
        .iter()
        .map(|(path, _)| path.as_path())
        .chain(report)
    {
        if !paths.insert(path.to_path_buf()) {
            return Err(usage(format!(
                "publish output collision at {}",
                path.display()
            )));
        }
        if path.exists() {
            return Err(Diagnostic::new(
                OutcomeClass::PolicyRefused,
                ReasonCode::ExtractionCollision,
                format!("publish destination already exists: {}", path.display()),
            ));
        }
        if !path.parent().is_some_and(Path::is_dir) {
            return Err(usage(format!(
                "publish destination parent does not exist: {}",
                path.display()
            )));
        }
    }
    Ok(())
}

/// Commits prepared siblings without overwriting. A hard-link publication step
/// keeps each final name exclusive; any mid-commit failure removes every final
/// name created by this transaction before its temporary siblings are cleaned.
fn transactional_publish(artifacts: &[(PathBuf, Vec<u8>)]) -> Result<()> {
    let mut temporary = Vec::with_capacity(artifacts.len());
    for (ordinal, (final_path, bytes)) in artifacts.iter().enumerate() {
        let temporary_path = temporary_sibling(final_path, ordinal)?;
        let write_result = (|| {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary_path)
                .map_err(|error| io_error("create private publication temporary", &error))?;
            file.write_all(bytes)
                .and_then(|()| file.sync_all())
                .map_err(|error| io_error("write publication temporary", &error))
        })();
        if let Err(error) = write_result {
            let _ = std::fs::remove_file(&temporary_path);
            for (path, _) in &temporary {
                let _ = std::fs::remove_file(path);
            }
            return Err(error);
        }
        temporary.push((temporary_path, final_path.clone()));
    }

    let mut published = Vec::new();
    for (temporary_path, final_path) in &temporary {
        if let Err(error) = std::fs::hard_link(temporary_path, final_path) {
            for path in &published {
                let _ = std::fs::remove_file(path);
            }
            for (path, _) in &temporary {
                let _ = std::fs::remove_file(path);
            }
            return Err(io_error("atomically publish artifact", &error));
        }
        published.push(final_path.clone());
    }
    for (path, _) in &temporary {
        if let Err(error) = std::fs::remove_file(path) {
            for final_path in &published {
                let _ = std::fs::remove_file(final_path);
            }
            for (temporary_path, _) in &temporary {
                let _ = std::fs::remove_file(temporary_path);
            }
            return Err(io_error("remove publication temporary", &error));
        }
    }
    Ok(())
}

fn temporary_sibling(final_path: &Path, ordinal: usize) -> Result<PathBuf> {
    let parent = final_path
        .parent()
        .ok_or_else(|| usage("publication output has no parent directory"))?;
    let name = final_path
        .file_name()
        .and_then(OsStr::to_str)
        .ok_or_else(|| usage("publication output name must be valid UTF-8"))?;
    let process = std::process::id();
    for attempt in 0_u32..1024 {
        let candidate = parent.join(format!(
            ".{name}.entrybound-tmp-{process}-{ordinal}-{attempt}"
        ));
        if !candidate.exists() {
            return Ok(candidate);
        }
    }
    Err(Diagnostic::new(
        OutcomeClass::PolicyRefused,
        ReasonCode::ExtractionCollision,
        "could not reserve a private publication temporary name",
    ))
}

// ---------------------------------------------------------------------------
// verified legacy-to-Entrybound sidecars
// ---------------------------------------------------------------------------

fn command_sidecar(arguments: Vec<OsString>) -> Result<()> {
    if arguments.len() == 1 && matches!(arguments[0].to_str(), Some("--help" | "-h")) {
        print!("{SIDECAR_HELP}");
        return Ok(());
    }
    let mut positionals = Vec::new();
    let mut strict = false;
    let mut compat = None;
    let mut preserve = false;
    let mut from = None;
    let mut entry_name = None;
    let mut layout = None;
    let mut profile = None;
    let mut report_path = None;
    let mut cursor = 0;
    while cursor < arguments.len() {
        let value = &arguments[cursor];
        if value == "--strict" {
            if strict {
                return Err(usage("sidecar accepts --strict only once"));
            }
            strict = true;
        } else if value == "--preserve" {
            if preserve {
                return Err(usage("sidecar accepts --preserve only once"));
            }
            preserve = true;
        } else if let Some(profile_id) = value
            .to_str()
            .and_then(|value| value.strip_prefix("--compat="))
        {
            if compat.is_some() {
                return Err(usage("sidecar accepts --compat only once"));
            }
            compat = Some(profile_id.parse::<CompatibilityProfileId>()?);
        } else if value == "--compat" {
            if compat.is_some() {
                return Err(usage("sidecar accepts --compat only once"));
            }
            cursor += 1;
            compat = Some(
                arguments
                    .get(cursor)
                    .and_then(|candidate| candidate.to_str())
                    .ok_or_else(|| usage("--compat requires an exact versioned profile"))?
                    .parse::<CompatibilityProfileId>()?,
            );
        } else if value == "--from" {
            if from.is_some() {
                return Err(usage("sidecar accepts --from only once"));
            }
            cursor += 1;
            from = Some(
                arguments
                    .get(cursor)
                    .and_then(|candidate| candidate.to_str())
                    .ok_or_else(|| usage("--from requires a supported format"))?
                    .parse::<LegacySourceFormat>()?,
            );
        } else if let Some(format) = value
            .to_str()
            .and_then(|value| value.strip_prefix("--from="))
        {
            if from.is_some() {
                return Err(usage("sidecar accepts --from only once"));
            }
            from = Some(format.parse::<LegacySourceFormat>()?);
        } else if value == "--entry-name" {
            if entry_name.is_some() {
                return Err(usage("sidecar accepts --entry-name only once"));
            }
            cursor += 1;
            entry_name = Some(
                arguments
                    .get(cursor)
                    .and_then(|candidate| candidate.to_str())
                    .ok_or_else(|| usage("--entry-name requires a UTF-8 LogicalPath"))?
                    .to_owned(),
            );
        } else if value == "--layout" {
            if layout.is_some() {
                return Err(usage("sidecar accepts --layout only once"));
            }
            cursor += 1;
            layout = Some(parse_layout_option(arguments.get(cursor))?);
        } else if value == "--profile" {
            if profile.is_some() {
                return Err(usage("sidecar accepts --profile only once"));
            }
            cursor += 1;
            profile = Some(
                arguments
                    .get(cursor)
                    .and_then(|candidate| candidate.to_str())
                    .ok_or_else(|| usage("--profile requires a UTF-8 profile name"))?
                    .parse::<CompressionProfile>()?,
            );
        } else if let Some(candidate) = value
            .to_str()
            .and_then(|value| value.strip_prefix("--profile="))
        {
            if profile.is_some() {
                return Err(usage("sidecar accepts --profile only once"));
            }
            profile = Some(candidate.parse::<CompressionProfile>()?);
        } else if value == "--report" {
            cursor += 1;
            set_once_path(
                &mut report_path,
                arguments.get(cursor),
                "--report requires a path",
            )?;
        } else if value.to_string_lossy().starts_with("--") {
            return Err(usage(format!(
                "sidecar does not recognize option '{}'",
                value.to_string_lossy()
            )));
        } else {
            positionals.push(value.clone());
        }
        cursor += 1;
    }
    if !(1..=2).contains(&positionals.len()) {
        return Err(usage(
            "sidecar requires <legacy-artifact> [output.eb] and import options",
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
    let source_path = PathBuf::from(&positionals[0]);
    if source_path == Path::new("-") {
        return Err(usage("sidecar requires a stable legacy source path"));
    }
    let output_path = positionals.get(1).map_or_else(
        || {
            let mut name = source_path.as_os_str().to_os_string();
            name.push(".eb");
            PathBuf::from(name)
        },
        PathBuf::from,
    );
    if output_path == source_path {
        return Err(usage("sidecar output must differ from its legacy source"));
    }
    let source_bytes = read_stable_source(&source_path)?;
    let imported = import_legacy_for_policy(
        &source_bytes,
        from,
        entry_name.as_deref(),
        profile.unwrap_or_default(),
        import_policy,
    )?;
    let selected_layout = layout.unwrap_or(Layout::Indexed);
    let sidecar = prepare_sidecar(&imported, &source_bytes, selected_layout)?;
    let sidecar_bytes = sidecar.bytes;
    let reopened = sidecar.verified_archive;
    let source_digest = sidecar.source_digest;
    let provenance = reopened
        .conversion
        .as_ref()
        .expect("prepare_sidecar verifies ConversionProvenance");
    let conflict_count = provenance
        .omission_count
        .saturating_add(provenance.refinement_count)
        .saturating_add(provenance.divergence_count)
        .saturating_add(provenance.irreconcilable_count);
    let sidecar_sha256 = entrybound::identity::sha256_exact(&sidecar_bytes);
    let sidecar_length =
        u64::try_from(sidecar_bytes.len()).map_err(|_| usage("sidecar length exceeds u64"))?;
    let report = MigrationReportV1 {
        source_kind: "legacy-sidecar".to_owned(),
        source_lai: reopened.descriptor.lai,
        source_aux: reopened.descriptor.aux,
        source_pcr: reopened.descriptor.pcr,
        source_security: ExportSourceSecurity::default(),
        source_has_conversion_evidence: true,
        source_has_preserved_evidence: reopened.preservation.is_some(),
        requested_targets: Box::default(),
        native_artifact: Some(NativeArtifactReport {
            output_path: output_path.display().to_string(),
            relation: "native-sidecar-of-exact-legacy-source".to_owned(),
            byte_length: sidecar_length,
            sha256: sidecar_sha256,
            produced: true,
        }),
        sidecar: Some(SidecarMigrationReport {
            source_format: provenance.source_format.clone(),
            source_sha256: source_digest,
            import_mode: provenance.import_mode.clone(),
            compatibility_profile: import_policy
                .compatibility_profile()
                .map(|profile| profile.as_str().to_owned()),
            conflict_count,
            resolution_count: u64::try_from(provenance.resolutions.len()).unwrap_or(u64::MAX),
            exact_source_preserved: reopened.preservation.is_some(),
            sidecar_path: output_path.display().to_string(),
            sidecar_byte_length: sidecar_length,
            sidecar_sha256,
            verification_succeeded: true,
        }),
        overall_outcome: MigrationOutcome::Published,
    };
    let mut pending = vec![(output_path.clone(), sidecar_bytes)];
    if let Some(path) = &report_path {
        pending.push((path.clone(), report.to_canonical_json()));
    }
    validate_transaction_paths(&pending, None)?;
    transactional_publish(&pending)?;
    println!(
        "OK created verified sidecar {} for {}",
        output_path.display(),
        source_path.display()
    );
    println!("sidecar source: {}", source_path.display());
    println!("source SHA-256: {source_digest}");
    println!("native LAI: {}", reopened.descriptor.lai);
    if let Some(path) = report_path {
        println!("migration report: {}", path.display());
    }
    Ok(())
}

fn read_stable_source(path: &Path) -> Result<Vec<u8>> {
    let mut file = File::open(path).map_err(|error| read_error(path, &error))?;
    let before = file.metadata().map_err(|error| read_error(path, &error))?;
    let mut bytes = Vec::new();
    std::io::Read::read_to_end(&mut file, &mut bytes).map_err(|error| read_error(path, &error))?;
    let after = file.metadata().map_err(|error| read_error(path, &error))?;
    let expected_length = u64::try_from(bytes.len()).map_err(|_| {
        Diagnostic::new(
            OutcomeClass::PolicyRefused,
            ReasonCode::LegacyResourcePolicyRefused,
            "legacy source length exceeds u64",
        )
    })?;
    if before.len() != after.len()
        || after.len() != expected_length
        || before.modified().ok() != after.modified().ok()
    {
        return Err(Diagnostic::new(
            OutcomeClass::PolicyRefused,
            ReasonCode::SourceUnstable,
            "legacy source changed while its sidecar snapshot was read",
        ));
    }
    Ok(bytes)
}

// ---------------------------------------------------------------------------
// legacy import conversion policies
// ---------------------------------------------------------------------------

fn command_convert(arguments: Vec<OsString>) -> Result<()> {
    if arguments.len() == 1 && matches!(arguments[0].to_str(), Some("--help" | "-h")) {
        print!("{CONVERT_HELP}");
        return Ok(());
    }
    if arguments.iter().any(|value| {
        value == "--to"
            || value == "--target-profile"
            || value.to_str().is_some_and(|value| {
                value.starts_with("--to=") || value.starts_with("--target-profile=")
            })
    }) {
        return command_export(arguments);
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
    let imported = import_legacy_for_policy(
        &source_bytes,
        from,
        entry_name.as_deref(),
        creation_profile,
        import_policy,
    )?;
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
    for (name, value) in &imported.report.format_statistics {
        status.line(format!("{name}: {value}"));
    }
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

fn import_legacy_for_policy(
    source_bytes: &[u8],
    from: Option<LegacySourceFormat>,
    entry_name: Option<&str>,
    creation_profile: CompressionProfile,
    import_policy: ImportPolicy,
) -> Result<LegacyImportResult> {
    if import_policy == ImportPolicy::Strict {
        return import_legacy_strict(
            source_bytes,
            from,
            entry_name,
            LegacyImportPolicy::default(),
            creation_profile,
        );
    }
    if entry_name.is_some() {
        return Err(usage(
            "--entry-name is unavailable for ZIP compatibility/preservation",
        ));
    }
    if from.is_some_and(|format| format != LegacySourceFormat::Zip)
        || detect_legacy(source_bytes).is_some_and(|format| format != LegacySourceFormat::Zip)
    {
        return Err(Diagnostic::new(
            OutcomeClass::Unsupported,
            ReasonCode::UnsupportedRequiredFeature,
            "--compat and --preserve are currently defined only for ZIP",
        ));
    }
    let imported = import_zip(
        source_bytes,
        ZipImportPolicy::default(),
        creation_profile,
        import_policy,
    )?;
    Ok(LegacyImportResult {
        archive: imported.archive,
        report: LegacyConversionReport {
            observation: imported.report.observation,
            synthesized_ancestors: imported.report.synthesized_ancestors,
            layers: Box::from(["zip".to_owned()]),
            wrapper_members: 0,
            decoded_child_digest: None,
            projection: "archive".to_owned(),
            format_statistics: Box::default(),
        },
    })
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
    let (parsed, extraction_policy) = parse_unpack_arguments(arguments)?;
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
        let report = unpack_opened(&opened, &destination, extraction_policy)?;
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
        report_metadata_restoration(&status, &report);
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
                extraction_policy,
                bootstrap_sequential_limits(),
            )?;
            (report, Some(stream))
        }
        Source::Path(path) if path_layout(path)? == Layout::Stream => {
            let file = File::open(path).map_err(|error| read_error(path, &error))?;
            let (report, stream) = unpack_stream(
                file,
                &destination,
                extraction_policy,
                bootstrap_sequential_limits(),
            )?;
            (report, Some(stream))
        }
        Source::Path(path) => (unpack(&read(path)?, &destination, extraction_policy)?, None),
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
    report_metadata_restoration(&status, &report);
    Ok(())
}

fn report_metadata_restoration(status: &Status, report: &entrybound::archive::ExtractionReport) {
    if report.metadata_not_restored.is_empty() {
        status.line("metadata restored: all represented and platform-supported items");
    } else {
        status.line(format!(
            "metadata limitations: {}",
            report.metadata_not_restored.join("; ")
        ));
    }
}

fn parse_unpack_arguments(arguments: Vec<OsString>) -> Result<(ReadArguments, ExtractionPolicy)> {
    let mut filtered = Vec::new();
    let mut policy = ExtractionPolicy::default();
    let mut cursor = 0_usize;
    while cursor < arguments.len() {
        let value = &arguments[cursor];
        if value == "--symlinks" {
            cursor += 1;
            policy = policy.with_symlinks(
                match arguments.get(cursor).and_then(|value| value.to_str()) {
                    Some("refuse") => SymlinkPolicy::Refuse,
                    Some("safe") => SymlinkPolicy::Safe,
                    Some("all") => SymlinkPolicy::All,
                    _ => return Err(usage("--symlinks requires refuse, safe, or all")),
                },
            );
        } else if value == "--restore-owner" {
            policy = policy.with_ownership(OwnershipPolicy::Restore);
        } else if value == "--xattrs" {
            cursor += 1;
            policy = policy.with_xattrs(
                match arguments.get(cursor).and_then(|value| value.to_str()) {
                    Some("ignore") => XAttrPolicy::Ignore,
                    Some("restore") => XAttrPolicy::Restore,
                    _ => return Err(usage("--xattrs requires ignore or restore")),
                },
            );
        } else if value == "--sparse" {
            cursor += 1;
            policy = policy.with_sparse(
                match arguments.get(cursor).and_then(|value| value.to_str()) {
                    Some("logical") => SparsePolicy::Logical,
                    Some("restore") => SparsePolicy::Restore,
                    _ => return Err(usage("--sparse requires logical or restore")),
                },
            );
        } else if value == "--acls" {
            cursor += 1;
            policy = policy.with_acls(
                match arguments.get(cursor).and_then(|value| value.to_str()) {
                    Some("ignore") => AclPolicy::Ignore,
                    Some("restore") => AclPolicy::Restore,
                    _ => return Err(usage("--acls requires ignore or restore")),
                },
            );
        } else if value == "--windows-security" {
            cursor += 1;
            policy = policy.with_windows_security(
                match arguments.get(cursor).and_then(|value| value.to_str()) {
                    Some("ignore") => WindowsSecurityPolicy::Ignore,
                    Some("restore") => WindowsSecurityPolicy::Restore,
                    _ => return Err(usage("--windows-security requires ignore or restore")),
                },
            );
        } else if value == "--reparse" {
            cursor += 1;
            policy = policy.with_reparse(
                match arguments.get(cursor).and_then(|value| value.to_str()) {
                    Some("refuse") => ReparsePolicy::Refuse,
                    Some("known-safe") => ReparsePolicy::KnownSafe,
                    Some("all") => ReparsePolicy::All,
                    _ => return Err(usage("--reparse requires refuse, known-safe, or all")),
                },
            );
        } else if value == "--platform-metadata" {
            cursor += 1;
            policy = policy.with_platform_metadata(
                match arguments.get(cursor).and_then(|value| value.to_str()) {
                    Some("ignore") => PlatformMetadataPolicy::Ignore,
                    Some("restore") => PlatformMetadataPolicy::Restore,
                    _ => return Err(usage("--platform-metadata requires ignore or restore")),
                },
            );
        } else {
            filtered.push(value.clone());
        }
        cursor += 1;
    }
    Ok((parse_read_arguments("unpack", filtered, false)?, policy))
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

struct RandomReadArguments {
    read: ReadArguments,
    output: Option<Destination>,
    access_report: bool,
}

fn parse_random_read_arguments(arguments: Vec<OsString>) -> Result<RandomReadArguments> {
    let mut ordinary = Vec::new();
    let mut output = None;
    let mut access_report = false;
    let mut cursor = 0;
    while cursor < arguments.len() {
        if arguments[cursor] == "--output" {
            if output.is_some() {
                return Err(usage("read accepts --output only once"));
            }
            cursor += 1;
            let value = arguments
                .get(cursor)
                .ok_or_else(|| usage("--output requires a file or '-'"))?;
            output = Some(if value == "-" {
                Destination::Stdout
            } else {
                Destination::Path(PathBuf::from(value))
            });
        } else if arguments[cursor] == "--access-report" {
            if access_report {
                return Err(usage("read accepts --access-report only once"));
            }
            access_report = true;
        } else {
            ordinary.push(arguments[cursor].clone());
        }
        cursor += 1;
    }
    Ok(RandomReadArguments {
        read: parse_read_arguments("read", ordinary, false)?,
        output,
        access_report,
    })
}

fn is_http_source(value: &OsStr) -> bool {
    value
        .to_str()
        .is_some_and(|value| value.starts_with("https://") || value.starts_with("http://"))
}

fn make_random_source(value: &OsStr) -> Result<Box<dyn RandomReadSource>> {
    if value == "-" {
        return Err(Diagnostic::new(
            OutcomeClass::Unsupported,
            ReasonCode::RandomAccessNotIndexed,
            "random access requires a seekable local file or an HTTP(S) range source",
        ));
    }
    if is_http_source(value) {
        let url = value
            .to_str()
            .ok_or_else(|| usage("HTTP URL is not valid UTF-8"))?;
        Ok(Box::new(HttpRangeSource::open(url)?))
    } else {
        Ok(Box::new(LocalFileRandomReadSource::open(PathBuf::from(
            value,
        ))?))
    }
}

fn parse_logical_path(value: &OsStr) -> Result<LogicalPath> {
    let value = value
        .to_str()
        .ok_or_else(|| usage("logical path is not valid UTF-8"))?;
    LogicalPath::from_utf8(value.split('/'))
}

fn command_read(arguments: Vec<OsString>) -> Result<()> {
    let parsed = parse_random_read_arguments(arguments)?;
    if parsed.read.positionals.len() != 2 {
        return Err(usage(
            "read requires <archive.eb|URL> <logical-path> [--output <file|->] [unlock] [--access-report]",
        ));
    }
    let archive_source = &parsed.read.positionals[0];
    let path = parse_logical_path(&parsed.read.positionals[1])?;
    let destination = parsed.output.unwrap_or(Destination::Stdout);
    let source = make_random_source(archive_source)?;
    let result = if let Some(unlock) = parsed.read.unlock.as_ref() {
        let mut archive = open_indexed_random_encrypted(
            source,
            RandomAccessPolicy::default(),
            EncryptedOpenOptions::new(Some(unlock.borrowed())),
        )?;
        archive.read_entry(&path)?
    } else {
        let mut archive = open_indexed_random(source, RandomAccessPolicy::default())?;
        archive.read_entry(&path)?
    };
    write_verified_entry(&destination, &result.bytes)?;
    let status = Status {
        to_stderr: matches!(destination, Destination::Stdout),
    };
    status.line(format!(
        "requested entry VERIFIED: {} ({} bytes); whole archive verified: false",
        path,
        result.bytes.len()
    ));
    if parsed.access_report {
        print_access_report(&status, &result.report);
    }
    Ok(())
}

fn write_verified_entry(destination: &Destination, bytes: &[u8]) -> Result<()> {
    match destination {
        Destination::Stdout => {
            let mut output = std::io::stdout().lock();
            output
                .write_all(bytes)
                .and_then(|()| output.flush())
                .map_err(|error| io_error("write verified entry to standard output", &error))
        }
        Destination::Path(path) => {
            let mut output = create_exclusive(path)?;
            if let Err(error) = output
                .write_all(bytes)
                .and_then(|()| output.sync_all())
                .map_err(|error| io_error("write verified entry", &error))
            {
                drop(output);
                let _ = std::fs::remove_file(path);
                return Err(error);
            }
            Ok(())
        }
    }
}

fn print_access_report(status: &Status, report: &RandomAccessVerificationReport) {
    status.line("access verification scope: requested object only; not whole-archive verification");
    status.line(format!(
        "source revision stable: {}",
        report.source_revision_stable
    ));
    status.line(format!("index: {:?}", report.index_status));
    status.line(format!(
        "verified chunks: {}; dependency chunks: {}",
        report.chunk_count_verified, report.dependency_chunk_count
    ));
    status.line(format!(
        "identity status: LAI={:?} AUX={:?} PCR={:?} PCI={:?}",
        report.lai, report.aux, report.pcr, report.pci
    ));
    status.line(format!(
        "ranges: {}; transferred bytes: {}; whole_archive_verified=false",
        report.range_request_count, report.bytes_fetched
    ));
    for AccessTraceEntry {
        offset,
        length,
        purpose,
        cache_hit,
    } in &*report.access_trace
    {
        status.line(format!(
            "range: offset={offset} bytes={length} purpose={purpose:?} cache={}",
            if *cache_hit { "hit" } else { "miss" }
        ));
    }
}

fn command_list(arguments: Vec<OsString>) -> Result<()> {
    let parsed = parse_read_arguments("list", arguments, false)?;
    if parsed.positionals.len() != 1 {
        return Err(usage(
            "list requires <archive.eb|URL|-> [--identity <file>|--password]",
        ));
    }
    if is_http_source(&parsed.positionals[0]) || parsed.unlock.is_some() {
        let source = make_random_source(&parsed.positionals[0])?;
        let (entries, report) = if let Some(unlock) = parsed.unlock.as_ref() {
            let archive = open_indexed_random_encrypted(
                source,
                RandomAccessPolicy::default(),
                EncryptedOpenOptions::new(Some(unlock.borrowed())),
            )?;
            (
                archive.metadata().entries.clone(),
                archive.metadata_report()?,
            )
        } else {
            let archive = open_indexed_random(source, RandomAccessPolicy::default())?;
            (
                archive.metadata().entries.clone(),
                archive.metadata_report()?,
            )
        };
        eprintln!(
            "note: range-backed metadata view only; logical entries are authenticated/hashed as reported, but the whole archive was not verified"
        );
        eprintln!(
            "ranges={} bytes={} index={:?} whole_archive_verified=false",
            report.range_request_count, report.bytes_fetched, report.index_status
        );
        for entry in entries.entries() {
            let kind = match entry.kind() {
                EntryKind::Directory => "directory",
                EntryKind::File => "file",
                EntryKind::Symlink => "symlink",
                EntryKind::ReparsePoint => "reparse-point",
            };
            println!("{kind}\t{}", entry.path());
        }
        return Ok(());
    }
    let source = Source::parse(&parsed.positionals[0]);
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
            EntryKind::Symlink => "symlink",
            EntryKind::ReparsePoint => "reparse-point",
        };
        println!("{kind}\t{}", entry.path);
    }
    Ok(())
}

fn command_inspect_random(parsed: ReadArguments) -> Result<()> {
    let source_argument = &parsed.positionals[0];
    if parsed.unlock.is_none() && parsed.crypto {
        let public = inspect_indexed_random_encrypted_public(
            make_random_source(source_argument)?,
            RandomAccessPolicy::default(),
            CryptoPolicy::default(),
        )?;
        println!("inspection scope: public range-backed crypto framing only");
        println!("encrypted: yes");
        println!("layout: INDEXED");
        println!("payload suite: {}", public.public.payload_suite);
        println!("recipient count: {}", public.public.recipient_count);
        println!(
            "recipient stanza types: {}",
            public.public.recipient_types.join(", ")
        );
        println!("padding: {:?}", public.public.padding);
        println!("encrypted boundary mode: {:?}", public.public.boundary);
        println!(
            "encrypted segment count: {}",
            public.public.segment_count.unwrap_or_default()
        );
        println!("required features: {:#x}", public.required_features);
        println!("source revision: {:?}", public.source_revision);
        println!("container bytes: {}", public.public.total_container_bytes);
        println!(
            "ranges: {}; transferred bytes: {}",
            public.range_request_count, public.bytes_fetched
        );
        println!("private metadata: locked/not fetched");
        println!("PCR: not fetched");
        println!("PCI: NOT_COMPUTED");
        println!("whole_archive_verified: false");
        return Ok(());
    }
    let (metadata, report) = if let Some(unlock) = parsed.unlock.as_ref() {
        let archive = open_indexed_random_encrypted(
            make_random_source(source_argument)?,
            RandomAccessPolicy::default(),
            EncryptedOpenOptions::new(Some(unlock.borrowed())),
        )?;
        (archive.metadata().clone(), archive.metadata_report()?)
    } else {
        let archive = open_indexed_random(
            make_random_source(source_argument)?,
            RandomAccessPolicy::default(),
        )?;
        (archive.metadata().clone(), archive.metadata_report()?)
    };
    println!("inspection scope: verified range-backed metadata view");
    println!("layout: INDEXED");
    println!("encrypted: {}", metadata.encrypted);
    println!("source revision: {:?}", metadata.source_revision);
    println!("container bytes: {}", metadata.source_length);
    println!("entries: {}", metadata.entries.len());
    println!("planner: {}", metadata.descriptor.planner_id);
    println!("chunker: {}", metadata.descriptor.chunker_id);
    println!("LAI: {} ({:?})", metadata.descriptor.lai, report.lai);
    println!("AUX: {} ({:?})", metadata.descriptor.aux, report.aux);
    println!("PCR: {} ({:?})", metadata.descriptor.pcr, report.pcr);
    println!("PCI: {:?}", report.pci);
    println!("index: {:?}", report.index_status);
    println!("physical directory records: {}", metadata.section_count);
    for section in &*metadata.section_directory {
        println!(
            "physical directory: kind={} offset={} payload-bytes={}",
            section.kind, section.offset, section.payload_length
        );
    }
    println!(
        "ranges: {}; transferred bytes: {}",
        report.range_request_count, report.bytes_fetched
    );
    println!("whole_archive_verified: false");
    Ok(())
}

fn command_repack(arguments: Vec<OsString>) -> Result<()> {
    if arguments.len() == 1 && matches!(arguments[0].to_str(), Some("--help" | "-h")) {
        print!("{REPACK_HELP}");
        return Ok(());
    }
    let mut positionals = Vec::new();
    let mut layout = None;
    let mut profile = None;
    let mut index = IndexPolicy::Preserve;
    let mut index_seen = false;
    let mut stream_window = StreamWindow::Auto;
    let mut window_seen = false;
    let mut dry_run = false;
    let mut cursor = 0;
    while cursor < arguments.len() {
        let value = &arguments[cursor];
        if value == "--layout" {
            cursor += 1;
            layout = Some(
                match arguments.get(cursor).and_then(|value| value.to_str()) {
                    Some("indexed") => Layout::Indexed,
                    Some("stream") => Layout::Stream,
                    _ => return Err(usage("--layout requires 'indexed' or 'stream'")),
                },
            );
        } else if value == "--profile" {
            cursor += 1;
            profile = Some(
                arguments
                    .get(cursor)
                    .and_then(|value| value.to_str())
                    .ok_or_else(|| usage("--profile requires a UTF-8 profile"))?
                    .parse::<CompressionProfile>()?,
            );
        } else if value == "--index" {
            cursor += 1;
            index = match arguments.get(cursor).and_then(|value| value.to_str()) {
                Some("preserve") => IndexPolicy::Preserve,
                Some("present") => IndexPolicy::Present,
                Some("absent") => IndexPolicy::Absent,
                _ => return Err(usage("--index requires preserve, present, or absent")),
            };
            index_seen = true;
        } else if value == "--stream-window" {
            cursor += 1;
            stream_window = match arguments.get(cursor).and_then(|value| value.to_str()) {
                Some("auto") => StreamWindow::Auto,
                Some(value) => StreamWindow::Ceiling(value.parse::<u64>().map_err(|_| {
                    usage("--stream-window requires a non-negative integer or auto")
                })?),
                None => return Err(usage("--stream-window requires a value")),
            };
            window_seen = true;
        } else if value == "--dry-run" {
            dry_run = true;
        } else if value.to_string_lossy().starts_with("--") {
            return Err(usage(format!(
                "repack does not recognize option '{}'",
                value.to_string_lossy()
            )));
        } else {
            positionals.push(value.clone());
        }
        cursor += 1;
    }
    if positionals.len() != 2 {
        return Err(usage("repack requires <source.eb> <output.eb>"));
    }
    let source_path = PathBuf::from(&positionals[0]);
    let output_path = PathBuf::from(&positionals[1]);
    if path_is_encrypted(&source_path)? {
        return Err(Diagnostic::new(
            OutcomeClass::Unsupported,
            ReasonCode::CryptoLayoutUnsupported,
            "encrypted repack requires a future crypto-aware mutation contract",
        ));
    }
    let loaded = load(
        &Source::Path(source_path.clone()),
        StreamContentPolicy::Retain,
    )?;
    let target_layout = layout.unwrap_or(loaded.opened.archive.descriptor.layout);
    if window_seen && target_layout != Layout::Stream {
        return Err(usage("--stream-window applies only to STREAM output"));
    }
    if target_layout == Layout::Stream && index_seen && index != IndexPolicy::Preserve {
        return Err(usage(
            "STREAM has no Index; --index present/absent is invalid",
        ));
    }
    let prepared = prepare_repack(
        &loaded.opened,
        RepackOptions {
            mode: profile.map_or(RepackMode::RepresentationOnly, RepackMode::Replan),
            layout: target_layout,
            index,
            stream_window,
        },
    )?;
    print_repack_analysis(&prepared.analysis, dry_run);
    if dry_run {
        println!("prospective output only; no file written");
        return Ok(());
    }
    let source_bytes = read(&source_path)?;
    publish_staged_file(&output_path, &prepared.encoded.bytes)?;
    if prepared.analysis.mode == RepackMode::RepresentationOnly
        && source_bytes == prepared.encoded.bytes
    {
        println!("exact no-op repack: PCI and bytes reproduced");
    } else if prepared.analysis.mode == RepackMode::RepresentationOnly {
        println!("semantic/physical equivalent; container rewritten");
    }
    println!(
        "OK repacked and post-write verified {}",
        output_path.display()
    );
    Ok(())
}

fn print_repack_analysis(analysis: &entrybound::archive::RepackAnalysis, prospective: bool) {
    let label = if prospective {
        "PROSPECTIVE"
    } else {
        "VERIFIED"
    };
    println!("{label} repack comparison");
    println!(
        "mode: {}",
        match analysis.mode {
            RepackMode::RepresentationOnly => "representation-only",
            RepackMode::Replan(_) => "replan",
        }
    );
    println!(
        "layout: {} -> {}; planner: {} -> {}",
        analysis.source_layout.as_str(),
        analysis.target_layout.as_str(),
        analysis.source_planner,
        analysis.target_planner
    );
    println!(
        "chunks logical/unique: {}/{} -> {}/{}",
        analysis.source_chunk_count,
        analysis.source_unique_chunk_count,
        analysis.target_chunk_count,
        analysis.target_unique_chunk_count
    );
    println!(
        "stored bytes: {} -> {}; working set: {} -> {}",
        analysis.source_stored_bytes,
        analysis.target_stored_bytes,
        analysis.source_working_set_bytes,
        analysis.target_working_set_bytes
    );
    println!(
        "dictionaries/groups/regions: {}/{}/{} -> {}/{}/{}",
        analysis.source_dictionary_count,
        analysis.source_group_count,
        analysis.source_region_count,
        analysis.target_dictionary_count,
        analysis.target_group_count,
        analysis.target_region_count
    );
    println!(
        "LAI equal: {}; AUX equal: {}; PCR equal: {}; prospective container bytes: {}",
        analysis.lai_equal, analysis.aux_equal, analysis.pcr_equal, analysis.output_bytes
    );
    println!("PCR: {} -> {}", analysis.source_pcr, analysis.target_pcr);
}

fn publish_staged_file(path: &Path, bytes: &[u8]) -> Result<()> {
    if path.exists() {
        return Err(Diagnostic::new(
            OutcomeClass::PolicyRefused,
            ReasonCode::Io,
            format!("output '{}' already exists", path.display()),
        ));
    }
    let temporary = temporary_sibling(path, 0)?;
    let mut file = create_exclusive(&temporary)?;
    if let Err(error) = file
        .write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|error| io_error("write and sync staged repack", &error))
    {
        drop(file);
        let _ = std::fs::remove_file(&temporary);
        return Err(error);
    }
    drop(file);
    let staged_verification = (|| {
        let staged_bytes = read(&temporary)?;
        match peek_layout(&staged_bytes)? {
            Layout::Indexed => {
                open(&staged_bytes)?;
            }
            Layout::Stream => {
                let limits = SequentialLimits {
                    content: StreamContentPolicy::Verify,
                    ..bootstrap_sequential_limits()
                };
                open_stream_with_limits(staged_bytes.as_slice(), limits)?;
            }
        }
        Ok(())
    })();
    if let Err(error) = staged_verification {
        let _ = std::fs::remove_file(&temporary);
        return Err(error);
    }
    if let Err(error) = std::fs::hard_link(&temporary, path) {
        let _ = std::fs::remove_file(&temporary);
        return Err(io_error("publish staged repack exclusively", &error));
    }
    let _ = std::fs::remove_file(&temporary);
    Ok(())
}

struct DiffArguments {
    left: OsString,
    right: OsString,
    left_unlock: Option<OwnedUnlock>,
    right_unlock: Option<OwnedUnlock>,
    json: bool,
    public_only: bool,
}

fn command_diff(arguments: Vec<OsString>) -> Result<()> {
    if arguments.len() == 1 && matches!(arguments[0].to_str(), Some("--help" | "-h")) {
        print!("{DIFF_HELP}");
        return Ok(());
    }
    let parsed = parse_diff_arguments(arguments)?;
    let remote = is_http_source(&parsed.left) || is_http_source(&parsed.right);
    let report = if parsed.public_only {
        public_crypto_diff(&parsed.left, &parsed.right)?
    } else if remote {
        let (left_metadata, left_report) =
            open_diff_metadata(&parsed.left, parsed.left_unlock.as_ref())?;
        let (right_metadata, right_report) =
            open_diff_metadata(&parsed.right, parsed.right_unlock.as_ref())?;
        archive_metadata_diff(&left_metadata, &left_report, &right_metadata, &right_report)?
    } else {
        let (left, left_security) = open_diff_full(&parsed.left, parsed.left_unlock.as_ref())?;
        let (right, right_security) = open_diff_full(&parsed.right, parsed.right_unlock.as_ref())?;
        let mut report = archive_diff(&left, &right)?;
        append_security_diff(&mut report, &left_security, &right_security);
        report
    };
    if parsed.json {
        std::io::stdout()
            .write_all(&report.to_canonical_json())
            .map_err(|error| io_error("write diff JSON", &error))?;
    } else {
        println!("LAI  {}", report.lai.as_str());
        println!("AUX  {}", report.aux.as_str());
        println!("PCR  {}", report.pcr.as_str());
        println!("PCI  {}", report.pci.as_str());
        println!("{}", report.interpretation);
        println!("left verification: {}", report.left_scope);
        println!("right verification: {}", report.right_scope);
        if let Some(summary) = report.physical_summary {
            println!(
                "physical summary: chunks reused={} added={} removed={} boundary-changed-objects={}",
                summary.chunks_reused,
                summary.chunks_added,
                summary.chunks_removed,
                summary.content_objects_with_boundary_changes
            );
        }
        for change in &report.changes {
            println!(
                "{} {} {}: {} -> {}",
                change.tier.as_str(),
                change.subject,
                change.field,
                change.left.as_deref().unwrap_or("<absent>"),
                change.right.as_deref().unwrap_or("<absent>")
            );
        }
    }
    Ok(())
}

fn parse_diff_arguments(arguments: Vec<OsString>) -> Result<DiffArguments> {
    let mut positionals = Vec::new();
    let mut left_identity = None;
    let mut right_identity = None;
    let mut left_password = false;
    let mut right_password = false;
    let mut json = false;
    let mut public_only = false;
    let mut cursor = 0;
    while cursor < arguments.len() {
        let value = &arguments[cursor];
        if value == "--json" {
            json = true;
        } else if value == "--public" {
            public_only = true;
        } else if value == "--left-identity" || value == "--right-identity" {
            let left = value == "--left-identity";
            cursor += 1;
            let path = PathBuf::from(
                arguments
                    .get(cursor)
                    .ok_or_else(|| usage("identity option requires a key file"))?,
            );
            if left {
                left_identity = Some(path);
            } else {
                right_identity = Some(path);
            }
        } else if value == "--left-password" {
            left_password = true;
        } else if value == "--right-password" {
            right_password = true;
        } else if value.to_string_lossy().starts_with("--") {
            return Err(usage(format!(
                "diff does not recognize option '{}'",
                value.to_string_lossy()
            )));
        } else {
            positionals.push(value.clone());
        }
        cursor += 1;
    }
    if positionals.len() != 2 {
        return Err(usage("diff requires <left.eb|URL> <right.eb|URL>"));
    }
    if left_password && left_identity.is_some() || right_password && right_identity.is_some() {
        return Err(usage(
            "each side accepts either an identity or a password, not both",
        ));
    }
    if public_only
        && (left_password || right_password || left_identity.is_some() || right_identity.is_some())
    {
        return Err(usage("--public does not accept private unlock material"));
    }
    let left_unlock = if let Some(path) = left_identity {
        Some(OwnedUnlock::Identity(XWingIdentity::read_file(&path)?))
    } else if left_password {
        Some(OwnedUnlock::Password(Zeroizing::new(prompt_password(
            "Left archive password: ",
        )?)))
    } else {
        None
    };
    let right_unlock = if let Some(path) = right_identity {
        Some(OwnedUnlock::Identity(XWingIdentity::read_file(&path)?))
    } else if right_password {
        Some(OwnedUnlock::Password(Zeroizing::new(prompt_password(
            "Right archive password: ",
        )?)))
    } else {
        None
    };
    Ok(DiffArguments {
        left: positionals.remove(0),
        right: positionals.remove(0),
        left_unlock,
        right_unlock,
        json,
        public_only,
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct DiffSecurityContext {
    encrypted: bool,
    payload_suite: Option<u16>,
    recipient_set_digest: Option<entrybound::eam::Digest>,
    archive_id: Option<entrybound::eam::Digest>,
    embedded_signature_count: usize,
    stale_signature_count: usize,
}

fn public_crypto_diff(left: &OsStr, right: &OsStr) -> Result<ArchiveDiffReport> {
    let left = inspect_indexed_random_encrypted_public(
        make_random_source(left)?,
        RandomAccessPolicy::default(),
        CryptoPolicy::default(),
    )?;
    let right = inspect_indexed_random_encrypted_public(
        make_random_source(right)?,
        RandomAccessPolicy::default(),
        CryptoPolicy::default(),
    )?;
    let mut changes = Vec::new();
    let mut push = |field: &str, left: String, right: String| {
        if left != right {
            changes.push(DiffChange {
                tier: DiffTier::Container,
                subject: "public-crypto".to_owned(),
                field: field.to_owned(),
                left: Some(left),
                right: Some(right),
            });
        }
    };
    push(
        "required_features",
        format!("{:#x}", left.required_features),
        format!("{:#x}", right.required_features),
    );
    push(
        "payload_suite",
        left.public.payload_suite.to_owned(),
        right.public.payload_suite.to_owned(),
    );
    push(
        "recipient_count",
        left.public.recipient_count.to_string(),
        right.public.recipient_count.to_string(),
    );
    push(
        "recipient_types",
        left.public.recipient_types.join(","),
        right.public.recipient_types.join(","),
    );
    push(
        "padding",
        format!("{:?}", left.public.padding),
        format!("{:?}", right.public.padding),
    );
    push(
        "boundary",
        format!("{:?}", left.public.boundary),
        format!("{:?}", right.public.boundary),
    );
    push(
        "segment_count",
        format!("{:?}", left.public.segment_count),
        format!("{:?}", right.public.segment_count),
    );
    push(
        "total_container_bytes",
        left.public.total_container_bytes.to_string(),
        right.public.total_container_bytes.to_string(),
    );
    changes.sort();
    Ok(ArchiveDiffReport {
        lai: DiffIdentityStatus::NotVerified,
        aux: DiffIdentityStatus::NotVerified,
        pcr: DiffIdentityStatus::NotVerified,
        pci: DiffIdentityStatus::NotComputed,
        interpretation:
            "public encrypted-container framing compared; all private identity tiers locked"
                .to_owned(),
        left_scope: format!(
            "public crypto framing; {} bytes in {} requests",
            left.bytes_fetched, left.range_request_count
        ),
        right_scope: format!(
            "public crypto framing; {} bytes in {} requests",
            right.bytes_fetched, right.range_request_count
        ),
        physical_summary: None,
        changes: changes.into_boxed_slice(),
    })
}

fn open_diff_full(
    value: &OsStr,
    unlock: Option<&OwnedUnlock>,
) -> Result<(OpenedArchive, DiffSecurityContext)> {
    let path = PathBuf::from(value);
    if path_is_encrypted(&path)? {
        let unlock =
            unlock.ok_or_else(|| usage("encrypted diff tier requires per-side unlock material"))?;
        let authenticated = open_encrypted_authenticated(
            &read(&path)?,
            EncryptedOpenOptions::new(Some(unlock.borrowed())),
        )?;
        let current = current_bindings(&authenticated.opened, Some(authenticated.addressing))?;
        let statuses = verify_signatures(&authenticated.embedded_signatures, &current, None)?;
        let stale_signature_count = statuses
            .iter()
            .filter(|status| {
                status.content == BindingStatus::Stale
                    || status.physical == BindingStatus::Stale
                    || status.addressing == BindingStatus::Stale
            })
            .count();
        let security = DiffSecurityContext {
            encrypted: true,
            payload_suite: Some(authenticated.addressing.payload_suite_id),
            recipient_set_digest: Some(entrybound::eam::Digest::from_bytes(
                authenticated.addressing.recipient_set_digest,
            )),
            archive_id: Some(entrybound::eam::Digest::from_bytes(
                authenticated.addressing.archive_id,
            )),
            embedded_signature_count: authenticated.embedded_signatures.len(),
            stale_signature_count,
        };
        Ok((authenticated.opened, security))
    } else {
        if unlock.is_some() {
            return Err(usage(
                "unlock material was supplied for an unencrypted diff side",
            ));
        }
        Ok((
            load(&Source::Path(path), StreamContentPolicy::Retain)?.opened,
            DiffSecurityContext {
                encrypted: false,
                payload_suite: None,
                recipient_set_digest: None,
                archive_id: None,
                embedded_signature_count: 0,
                stale_signature_count: 0,
            },
        ))
    }
}

fn append_security_diff(
    report: &mut entrybound::archive::ArchiveDiffReport,
    left: &DiffSecurityContext,
    right: &DiffSecurityContext,
) {
    let mut changes = report.changes.to_vec();
    let mut push = |field: &str, left: String, right: String| {
        changes.push(DiffChange {
            tier: DiffTier::Container,
            subject: "security".to_owned(),
            field: field.to_owned(),
            left: Some(left),
            right: Some(right),
        });
    };
    if left.encrypted != right.encrypted {
        push(
            "encrypted",
            left.encrypted.to_string(),
            right.encrypted.to_string(),
        );
    }
    if left.payload_suite != right.payload_suite {
        push(
            "payload_suite",
            left.payload_suite
                .map_or_else(|| "none".to_owned(), |value| value.to_string()),
            right
                .payload_suite
                .map_or_else(|| "none".to_owned(), |value| value.to_string()),
        );
    }
    if left.recipient_set_digest != right.recipient_set_digest {
        push(
            "recipient_set_digest",
            left.recipient_set_digest
                .map_or_else(|| "none".to_owned(), |value| value.to_string()),
            right
                .recipient_set_digest
                .map_or_else(|| "none".to_owned(), |value| value.to_string()),
        );
    }
    if left.archive_id != right.archive_id {
        push(
            "archive_id",
            left.archive_id
                .map_or_else(|| "none".to_owned(), |value| value.to_string()),
            right
                .archive_id
                .map_or_else(|| "none".to_owned(), |value| value.to_string()),
        );
    }
    if left.embedded_signature_count != right.embedded_signature_count {
        push(
            "embedded_signature_count",
            left.embedded_signature_count.to_string(),
            right.embedded_signature_count.to_string(),
        );
    }
    if left.stale_signature_count != right.stale_signature_count {
        push(
            "stale_signature_count",
            left.stale_signature_count.to_string(),
            right.stale_signature_count.to_string(),
        );
    }
    changes.sort();
    report.changes = changes.into_boxed_slice();
}

fn open_diff_metadata(
    value: &OsStr,
    unlock: Option<&OwnedUnlock>,
) -> Result<(
    entrybound::ecf::RandomAccessMetadata,
    RandomAccessVerificationReport,
)> {
    if let Some(unlock) = unlock {
        let archive = open_indexed_random_encrypted(
            make_random_source(value)?,
            RandomAccessPolicy::default(),
            EncryptedOpenOptions::new(Some(unlock.borrowed())),
        )?;
        Ok((archive.metadata().clone(), archive.metadata_report()?))
    } else {
        let archive =
            open_indexed_random(make_random_source(value)?, RandomAccessPolicy::default())?;
        Ok((archive.metadata().clone(), archive.metadata_report()?))
    }
}

struct InspectArguments {
    read: ReadArguments,
    json: bool,
    views: InspectionViews,
}

fn parse_inspect_arguments(arguments: Vec<OsString>) -> Result<InspectArguments> {
    let mut ordinary = Vec::new();
    let mut json = false;
    let mut views = InspectionViews::default();
    for value in arguments {
        match value.to_str() {
            Some("--json") => json = true,
            Some("--entries") => views.entries = true,
            Some("--plans") => views.plans = true,
            Some("--chunks") => views.chunks = true,
            Some("--reconstruction") => views.reconstruction = true,
            Some("--provenance") => views.provenance = true,
            Some("--security") => views.security = true,
            Some("--access") => views.access = true,
            _ => ordinary.push(value),
        }
    }
    Ok(InspectArguments {
        read: parse_read_arguments("inspect", ordinary, true)?,
        json,
        views,
    })
}

fn command_inspect_json(parsed: InspectArguments) -> Result<()> {
    if parsed.read.positionals.len() != 1 {
        return Err(usage("inspect requires <archive.eb|URL|->"));
    }
    let value = &parsed.read.positionals[0];
    if is_http_source(value) {
        let (metadata, report) = open_diff_metadata(value, parsed.read.unlock.as_ref())?;
        std::io::stdout()
            .write_all(&random_inspection_json(&metadata, &report))
            .map_err(|error| io_error("write inspection JSON", &error))?;
        return Ok(());
    }
    let source = Source::parse(value);
    let encrypted = match &source {
        Source::Path(path) => path_is_encrypted(path)?,
        Source::Stdin => parsed.read.unlock.is_some(),
    };
    let bytes = if encrypted {
        let bytes = read_source_fully(&source)?;
        let Some(unlock) = parsed.read.unlock.as_ref() else {
            let public = inspect_encrypted(&bytes, None, CryptoPolicy::default())?;
            let json = format!(
                "{{\"format\":\"entrybound/inspection-v1\",\"version\":1,\"verification_scope\":\"public crypto framing only\",\"archive\":{{\"layout\":\"INDEXED\",\"encrypted\":true,\"features\":{}}},\"security\":{{\"payload_suite\":\"{}\",\"recipient_count\":{},\"padding\":\"{:?}\",\"boundary\":\"{:?}\",\"private_metadata_authenticated\":false}},\"identities\":{{\"lai\":null,\"aux\":null,\"pcr\":null,\"pci\":null}},\"whole_archive_verified\":false}}\n",
                u64::from_be_bytes(bytes[16..24].try_into().unwrap()),
                public.public.payload_suite,
                public.public.recipient_count,
                public.public.padding,
                public.public.boundary
            );
            std::io::stdout()
                .write_all(json.as_bytes())
                .map_err(|error| io_error("write public crypto inspection JSON", &error))?;
            return Ok(());
        };
        let authenticated = open_encrypted_authenticated(
            &bytes,
            EncryptedOpenOptions::new(Some(unlock.borrowed())),
        )?;
        let current = current_bindings(&authenticated.opened, Some(authenticated.addressing))?;
        let statuses = verify_signatures(&authenticated.embedded_signatures, &current, None)?;
        let security = InspectionSecurity {
            encrypted: true,
            payload_suite: Some("payload-suite-v1".to_owned()),
            recipient_set_digest: Some(entrybound::eam::Digest::from_bytes(
                authenticated.addressing.recipient_set_digest,
            )),
            archive_id: Some(entrybound::eam::Digest::from_bytes(
                authenticated.addressing.archive_id,
            )),
            embedded_signature_count: authenticated.embedded_signatures.len() as u64,
            signatures_valid: statuses
                .iter()
                .filter(|status| status.cryptographic == CryptographicStatus::Valid)
                .count() as u64,
            signatures_invalid: statuses
                .iter()
                .filter(|status| status.cryptographic == CryptographicStatus::Invalid)
                .count() as u64,
            signatures_unsupported: statuses
                .iter()
                .filter(|status| status.cryptographic == CryptographicStatus::Unsupported)
                .count() as u64,
            signatures_stale: statuses
                .iter()
                .filter(|status| {
                    status.content == BindingStatus::Stale
                        || status.physical == BindingStatus::Stale
                        || status.addressing == BindingStatus::Stale
                })
                .count() as u64,
        };
        inspection_json_with_security(&authenticated.opened, parsed.views, &security)?
    } else {
        if parsed.read.unlock.is_some() {
            return Err(usage(
                "unlock material was supplied for an unencrypted archive",
            ));
        }
        let loaded = load(&source, StreamContentPolicy::Retain)?;
        inspection_json(&loaded.opened, parsed.views)?
    };
    std::io::stdout()
        .write_all(&bytes)
        .map_err(|error| io_error("write inspection JSON", &error))?;
    Ok(())
}

fn command_inspect(arguments: Vec<OsString>) -> Result<()> {
    let inspection = parse_inspect_arguments(arguments)?;
    if inspection.json {
        return command_inspect_json(inspection);
    }
    let focused_views = inspection.views;
    let parsed = inspection.read;
    if parsed.positionals.len() != 1 {
        return Err(usage(
            "inspect requires <archive.eb|URL|-> [--crypto] [--identity <file>|--password]",
        ));
    }
    if is_http_source(&parsed.positionals[0]) {
        return command_inspect_random(parsed);
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
    if focused_views.entries
        || focused_views.plans
        || focused_views.chunks
        || focused_views.reconstruction
        || focused_views.provenance
        || focused_views.security
        || focused_views.access
    {
        return print_focused_inspection(&view, stream.as_ref(), focused_views, encrypted);
    }
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

fn print_focused_inspection(
    view: &entrybound::archive::ArchiveInspection,
    stream: Option<&StreamReport>,
    selected: InspectionViews,
    encrypted: bool,
) -> Result<()> {
    println!("inspection scope: whole archive verified");
    if selected.entries {
        println!("entries: {}", view.entry_count);
        println!("total logical bytes: {}", view.total_logical_bytes);
    }
    if selected.plans {
        println!("planner: {}; chunker: {}", view.planner_id, view.chunker_id);
        for plan in &view.plans {
            println!(
                "plan {}: {} codec={} transforms={} dictionary={}",
                plan.plan_id,
                plan.identifier,
                plan.codec,
                plan.transforms.join(" -> "),
                plan.dictionary
                    .map_or_else(|| "none".to_owned(), |value| value.to_string())
            );
        }
    }
    if selected.chunks {
        println!(
            "chunks: unique={} logical-references={} plaintext-bytes={} deduplicated-bytes={}",
            view.chunks.unique_chunk_count,
            view.chunks.logical_chunk_references,
            view.chunks.unique_plaintext_bytes,
            view.chunks.deduplicated_bytes
        );
    }
    if selected.reconstruction {
        println!(
            "reconstruction: data={} regions={} jpeg-regions={} worst-access-chunks={} worst-access-bytes={}",
            view.reconstruction.object_count,
            view.whole_object.region_count,
            view.whole_object.jpeg_region_count,
            view.whole_object.worst_access_chunks,
            view.whole_object.worst_access_bytes
        );
    }
    if selected.provenance {
        match &view.conversion {
            Some(value) => println!(
                "conversion: source={} digest={} mode={} resolutions={}",
                value.source_format,
                value.source_digest,
                value.import_mode,
                value.resolutions.len()
            ),
            None => println!("conversion: none"),
        }
        println!("legacy preservation: {}", view.preservation.is_some());
        println!(
            "fidelity: unavailable={} degraded={}",
            view.fidelity.unavailable.len(),
            view.fidelity.degraded.len()
        );
    }
    if selected.security {
        println!("encrypted: {encrypted}");
        println!("secret material exposed: false");
    }
    if selected.access {
        println!("layout: {}", view.layout.as_str());
        println!("random entry lookup: {}", view.random_entry_lookup);
        println!("index: {}", index_status(view.index_status));
        println!("stream dedup window: {}", view.stream_dedup_window);
        println!(
            "lookback: groups={} maximum={} worst-bytes={}",
            view.cross_file.chunk_group_count,
            view.cross_file.maximum_lookback,
            view.cross_file.worst_random_access_bytes
        );
        if let Some(stream) = stream {
            println!("sequential scan bytes: {}", stream.total_len);
        }
    }
    Ok(())
}

fn command_explain(arguments: Vec<OsString>) -> Result<()> {
    let parsed = parse_read_arguments("explain", arguments, false)?;
    if !(1..=2).contains(&parsed.positionals.len()) {
        return Err(usage("explain requires <archive.eb|-> [logical-path]"));
    }
    let source = Source::parse(&parsed.positionals[0]);
    // Compression explanation re-derives the alternatives the planner weighed,
    // so a STREAM source must be scanned with a retaining content policy.
    let encrypted = match &source {
        Source::Path(path) => path_is_encrypted(path)?,
        Source::Stdin => parsed.unlock.is_some(),
    };
    let loaded = if encrypted {
        let unlock = parsed
            .unlock
            .as_ref()
            .ok_or_else(|| usage("encrypted explain requires --identity or --password"))?;
        Loaded {
            opened: open_encrypted_authenticated(
                &read_source_fully(&source)?,
                EncryptedOpenOptions::new(Some(unlock.borrowed())),
            )?
            .opened,
            stream: None,
        }
    } else {
        if parsed.unlock.is_some() {
            return Err(usage(
                "unlock material was supplied for an unencrypted archive",
            ));
        }
        load(&source, StreamContentPolicy::Retain)?
    };
    if loaded.stream.is_some() {
        eprintln!(
            "note: STREAM layout has no Index; this explanation required a complete sequential pass"
        );
    }
    let path = parsed.positionals.get(1).and_then(|value| value.to_str());
    if parsed.positionals.len() == 2 && path.is_none() {
        return Err(usage("logical path is not valid UTF-8"));
    }
    let structured = structured_explain(&loaded.opened, path)?;
    println!("evidence classes: RECORDED, DERIVED, AUDIT, NOT_RECORDED");
    for fact in &structured.facts {
        println!(
            "[{}] {} {}: {}",
            fact.class.as_str(),
            fact.subject,
            fact.field,
            fact.value
        );
    }
    if path.is_some() {
        return Ok(());
    }
    println!("[DERIVED] aggregate compression summary follows");
    let explanation = compression_explain(&loaded.opened)?;
    println!("[RECORDED] planner: {}", explanation.planner_id);
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
        "[NOT_RECORDED] replayed independent-codec comparison estimate: {} bytes",
        explanation.ordinary_codec_savings_bytes
    );
    println!(
        "[NOT_RECORDED] shared-dictionary payload savings: {} bytes (replayed comparison estimate; recorded dictionary storage: {} bytes)",
        explanation.shared_dictionary_savings_bytes, explanation.dictionary_storage_bytes
    );
    println!(
        "[NOT_RECORDED] bounded-lookback payload savings: {} bytes (replayed comparison estimate)",
        explanation.bounded_lookback_savings_bytes
    );
    println!(
        "[NOT_RECORDED] replayed reconstructive comparison: chunks={}, gross-savings={} bytes, recorded reconstruction-data-overhead={} bytes, net-savings={} bytes",
        explanation.reconstructive_chunk_count,
        explanation.reconstructive_gross_savings_bytes,
        explanation.reconstruction_data_overhead_bytes,
        explanation.reconstructive_net_savings_bytes
    );
    println!(
        "[AUDIT] reconstructive fallbacks: chunks={}{}",
        explanation.reconstructive_fallback_chunk_count,
        explanation
            .reconstructive_fallback_reason
            .as_ref()
            .map_or_else(String::new, |reason| format!(" ({reason})"))
    );
    println!(
        "[DERIVED] JPEG reconstruction: gross-savings={} bytes, representation={} bytes, region-overhead={} bytes, net-savings={} bytes{}",
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
        "[NOT_RECORDED] replayed structural-transform comparison: {} bytes (recorded transformed chunks: {}, inferred rejected eligible chunks: {})",
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
        println!("[NOT_RECORDED] transform candidate replay rule: {reason}");
    }
    println!(
        "similarity cohorts: count={}, chunks={}, logical-bytes={}, independently-encoded={}",
        explanation.similarity_cohort_count,
        explanation.similarity_cohort_chunks,
        explanation.similarity_cohort_logical_bytes,
        explanation.independent_similarity_cohort_count
    );
    if let Some(reason) = explanation.independent_cohort_reason {
        println!("[NOT_RECORDED] inferred independent cohort decision: {reason}");
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
    use super::{parse_unpack_arguments, run};
    use entrybound::archive::{
        AclPolicy, OwnershipPolicy, PlatformMetadataPolicy, ReparsePolicy, SparsePolicy,
        SymlinkPolicy, WindowsSecurityPolicy, XAttrPolicy,
    };
    use entrybound::diagnostics::ReasonCode;
    use std::ffi::OsString;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

    fn args(values: &[&str]) -> Vec<OsString> {
        values.iter().map(OsString::from).collect()
    }

    #[test]
    fn unpack_metadata_policies_are_explicit() {
        let (parsed, policy) = parse_unpack_arguments(args(&[
            "archive.eb",
            "destination",
            "--symlinks",
            "all",
            "--restore-owner",
            "--xattrs",
            "restore",
            "--sparse",
            "restore",
            "--acls",
            "restore",
            "--windows-security",
            "restore",
            "--reparse",
            "all",
            "--platform-metadata",
            "restore",
        ]))
        .unwrap();

        assert_eq!(parsed.positionals, args(&["archive.eb", "destination"]));
        assert_eq!(policy.symlinks(), SymlinkPolicy::All);
        assert_eq!(policy.ownership(), OwnershipPolicy::Restore);
        assert_eq!(policy.xattrs(), XAttrPolicy::Restore);
        assert_eq!(policy.sparse(), SparsePolicy::Restore);
        assert_eq!(policy.acls(), AclPolicy::Restore);
        assert_eq!(policy.windows_security(), WindowsSecurityPolicy::Restore);
        assert_eq!(policy.reparse(), ReparsePolicy::All);
        assert_eq!(policy.platform_metadata(), PlatformMetadataPolicy::Restore);
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

    fn store_sevenz(name: &str, content: &[u8]) -> Vec<u8> {
        fn number(value: u64) -> Vec<u8> {
            for extra in 0..8_u32 {
                let high_bits = 7 - extra;
                if u128::from(value) < (1_u128 << (high_bits + extra * 8)) {
                    let prefix = if extra == 0 {
                        0
                    } else {
                        0xff_u8 << (8 - extra)
                    };
                    let mut bytes = vec![prefix | u8::try_from(value >> (extra * 8)).unwrap()];
                    for index in 0..extra {
                        bytes.push(u8::try_from((value >> (index * 8)) & 0xff).unwrap());
                    }
                    return bytes;
                }
            }
            let mut bytes = vec![0xff];
            bytes.extend_from_slice(&value.to_le_bytes());
            bytes
        }

        let mut header = vec![0x01, 0x04, 0x06];
        header.extend(number(0));
        header.extend(number(1));
        header.push(0x09);
        header.extend(number(u64::try_from(content.len()).unwrap()));
        header.push(0);
        header.push(0x07);
        header.push(0x0b);
        header.extend(number(1));
        header.push(0);
        header.extend(number(1));
        header.push(1);
        header.push(0);
        header.push(0x0c);
        header.extend(number(u64::try_from(content.len()).unwrap()));
        header.push(0x0a);
        header.push(1);
        header.extend_from_slice(&crc32fast::hash(content).to_le_bytes());
        header.push(0);
        header.push(0x08);
        header.push(0);
        header.push(0);
        header.push(0x05);
        header.extend(number(1));
        header.push(0x11);
        let mut names = vec![0];
        for unit in name.encode_utf16() {
            names.extend_from_slice(&unit.to_le_bytes());
        }
        names.extend_from_slice(&0_u16.to_le_bytes());
        header.extend(number(u64::try_from(names.len()).unwrap()));
        header.extend(names);
        header.push(0);
        header.push(0);

        let mut start = Vec::new();
        start.extend_from_slice(&u64::try_from(content.len()).unwrap().to_le_bytes());
        start.extend_from_slice(&u64::try_from(header.len()).unwrap().to_le_bytes());
        start.extend_from_slice(&crc32fast::hash(&header).to_le_bytes());
        let mut source = Vec::new();
        source.extend_from_slice(b"7z\xbc\xaf'\x1c");
        source.extend_from_slice(&[0, 4]);
        source.extend_from_slice(&crc32fast::hash(&start).to_le_bytes());
        source.extend(start);
        source.extend_from_slice(content);
        source.extend(header);
        source
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
    fn sevenz_convert_supports_indexed_stream_and_dry_run() {
        let id = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let root =
            std::env::temp_dir().join(format!("entrybound-cli-7z-{}-{id}", std::process::id()));
        std::fs::create_dir(&root).unwrap();
        let input = root.join("source.bin");
        std::fs::write(
            &input,
            store_sevenz("nested/file.txt", b"strict 7z conversion"),
        )
        .unwrap();
        for layout in ["indexed", "stream"] {
            let archive = root.join(format!("{layout}.eb"));
            let restored = root.join(format!("{layout}-restored"));
            run(vec![
                OsString::from("ebound"),
                OsString::from("convert"),
                input.as_os_str().to_owned(),
                archive.as_os_str().to_owned(),
                OsString::from("--strict"),
                OsString::from("--from=7z"),
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
                std::fs::read(restored.join("nested/file.txt")).unwrap(),
                b"strict 7z conversion"
            );
        }
        let dry = root.join("dry.eb");
        run(vec![
            OsString::from("ebound"),
            OsString::from("convert"),
            input.as_os_str().to_owned(),
            dry.as_os_str().to_owned(),
            OsString::from("--strict"),
            OsString::from("--dry-run"),
        ])
        .unwrap();
        assert!(!dry.exists());
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
        let error = run(args(&["ebound", "salvage"])).unwrap_err();
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

    #[test]
    fn deterministic_zip_tar_export_receipt_dry_run_and_reimport_workflow() {
        let id = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let root =
            std::env::temp_dir().join(format!("entrybound-cli-export-{}-{id}", std::process::id()));
        let input = root.join("input");
        std::fs::create_dir_all(input.join("nested")).unwrap();
        std::fs::write(input.join("nested/file.txt"), b"exported bytes").unwrap();
        let source = root.join("source.eb");
        run(vec![
            OsString::from("ebound"),
            OsString::from("pack"),
            input.as_os_str().to_owned(),
            source.as_os_str().to_owned(),
            OsString::from("--profile"),
            OsString::from("fast"),
        ])
        .unwrap();

        let unapproved = root.join("unapproved-lossy.zip");
        assert_eq!(
            run(vec![
                OsString::from("ebound"),
                OsString::from("convert"),
                source.as_os_str().to_owned(),
                unapproved.as_os_str().to_owned(),
                OsString::from("--to=zip"),
            ])
            .unwrap_err()
            .code(),
            ReasonCode::LegacyExportLossyApprovalRequired
        );
        assert!(!unapproved.exists());

        for (name, target) in [("target.zip", "zip"), ("target.tar", "tar")] {
            let legacy = root.join(name);
            let receipt = root.join(format!("{name}.receipt.json"));
            run(vec![
                OsString::from("ebound"),
                OsString::from("convert"),
                source.as_os_str().to_owned(),
                legacy.as_os_str().to_owned(),
                OsString::from("--to"),
                OsString::from(target),
                OsString::from("--allow-lossy"),
                OsString::from("--receipt"),
                receipt.as_os_str().to_owned(),
            ])
            .unwrap();
            let receipt_text = std::fs::read_to_string(&receipt).unwrap();
            let target_hash = entrybound::identity::sha256_exact(&std::fs::read(&legacy).unwrap());
            assert!(receipt_text.contains(&target_hash.to_string()));
            assert!(receipt_text.contains("\"deterministic\":true"));

            let original_target = std::fs::read(&legacy).unwrap();
            assert_eq!(
                run(vec![
                    OsString::from("ebound"),
                    OsString::from("convert"),
                    source.as_os_str().to_owned(),
                    legacy.as_os_str().to_owned(),
                    OsString::from("--to"),
                    OsString::from(target),
                    OsString::from("--allow-lossy"),
                ])
                .unwrap_err()
                .code(),
                ReasonCode::Io
            );
            assert_eq!(std::fs::read(&legacy).unwrap(), original_target);

            let reimported = root.join(format!("{name}.eb"));
            run(vec![
                OsString::from("ebound"),
                OsString::from("convert"),
                legacy.as_os_str().to_owned(),
                reimported.as_os_str().to_owned(),
                OsString::from("--strict"),
            ])
            .unwrap();
            run(vec![
                OsString::from("ebound"),
                OsString::from("verify"),
                reimported.as_os_str().to_owned(),
            ])
            .unwrap();
        }

        let dry = root.join("dry.zip");
        run(vec![
            OsString::from("ebound"),
            OsString::from("convert"),
            source.as_os_str().to_owned(),
            dry.as_os_str().to_owned(),
            OsString::from("--target-profile=zip/portable-v1"),
            OsString::from("--dry-run"),
        ])
        .unwrap();
        assert!(!dry.exists());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn publish_is_canonical_multi_target_and_transactional() {
        let id = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "entrybound-cli-publish-{}-{id}",
            std::process::id()
        ));
        let input = root.join("input");
        let first = root.join("first");
        let second = root.join("second");
        std::fs::create_dir_all(input.join("nested")).unwrap();
        std::fs::create_dir(&first).unwrap();
        std::fs::create_dir(&second).unwrap();
        std::fs::write(input.join("nested/file.txt"), b"one semantic source").unwrap();

        for (output, targets) in [(&first, ["zip", "tar.zst"]), (&second, ["tar.zst", "zip"])] {
            let report = output.join("release.migration.json");
            run(vec![
                OsString::from("ebound"),
                OsString::from("publish"),
                input.as_os_str().to_owned(),
                OsString::from("--output-dir"),
                output.as_os_str().to_owned(),
                OsString::from("--native"),
                OsString::from("--target"),
                OsString::from(targets[0]),
                OsString::from("--target"),
                OsString::from(targets[1]),
                OsString::from("--base-name"),
                OsString::from("release"),
                OsString::from("--profile"),
                OsString::from("fast"),
                OsString::from("--allow-lossy"),
                OsString::from("--report"),
                report.as_os_str().to_owned(),
            ])
            .unwrap();
            assert!(output.join("release.eb").is_file());
            assert!(output.join("release.zip").is_file());
            assert!(output.join("release.tar.zst").is_file());
            let report = std::fs::read_to_string(report).unwrap();
            assert!(report.contains("entrybound/migration-report-v1"));
            assert!(report.contains("\"overall_publish_outcome\":\"PUBLISHED\""));
            assert!(
                report.find("tar.zst/pax-v1").unwrap() < report.find("zip/portable-v1").unwrap()
            );
        }
        assert_eq!(
            std::fs::read(first.join("release.zip")).unwrap(),
            std::fs::read(second.join("release.zip")).unwrap()
        );
        assert_eq!(
            std::fs::read(first.join("release.tar.zst")).unwrap(),
            std::fs::read(second.join("release.tar.zst")).unwrap()
        );

        let blocked = root.join("blocked");
        std::fs::create_dir(&blocked).unwrap();
        std::fs::write(blocked.join("release.zip"), b"pre-existing").unwrap();
        let error = run(vec![
            OsString::from("ebound"),
            OsString::from("publish"),
            input.as_os_str().to_owned(),
            OsString::from("--output-dir"),
            blocked.as_os_str().to_owned(),
            OsString::from("--native"),
            OsString::from("--target=zip"),
            OsString::from("--target=tar.zst"),
            OsString::from("--base-name=release"),
            OsString::from("--allow-lossy"),
        ])
        .unwrap_err();
        assert_eq!(error.code(), ReasonCode::ExtractionCollision);
        assert_eq!(
            std::fs::read(blocked.join("release.zip")).unwrap(),
            b"pre-existing"
        );
        assert!(!blocked.join("release.eb").exists());
        assert!(!blocked.join("release.tar.zst").exists());

        let staged = root.join("staged");
        std::fs::create_dir(&staged).unwrap();
        let first_final = staged.join("first.bin");
        let missing_final = staged.join("missing").join("last.bin");
        assert!(
            super::transactional_publish(&[
                (first_final.clone(), b"first".to_vec()),
                (missing_final, b"last".to_vec()),
            ])
            .is_err()
        );
        assert!(!first_final.exists());
        assert_eq!(std::fs::read_dir(&staged).unwrap().count(), 0);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn sidecar_binds_exact_legacy_source_and_verifies_before_publish() {
        let id = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "entrybound-cli-sidecar-{}-{id}",
            std::process::id()
        ));
        std::fs::create_dir(&root).unwrap();
        let source = root.join("release.zip");
        let sidecar = root.join("release.zip.eb");
        let report = root.join("release.zip.migration.json");
        let source_bytes = store_zip(b"payload.bin", b"sidecar content");
        std::fs::write(&source, &source_bytes).unwrap();
        run(vec![
            OsString::from("ebound"),
            OsString::from("sidecar"),
            source.as_os_str().to_owned(),
            OsString::from("--strict"),
            OsString::from("--profile"),
            OsString::from("fast"),
            OsString::from("--report"),
            report.as_os_str().to_owned(),
        ])
        .unwrap();
        assert_eq!(std::fs::read(&source).unwrap(), source_bytes);
        let opened = entrybound::ecf::open(&std::fs::read(&sidecar).unwrap()).unwrap();
        assert_eq!(
            opened.archive.conversion.as_ref().unwrap().source_digest,
            entrybound::identity::sha256_exact(&source_bytes)
        );
        let report = std::fs::read_to_string(report).unwrap();
        assert!(report.contains("\"source_kind\":\"legacy-sidecar\""));
        assert!(report.contains("\"verification_succeeded\":true"));
        assert_eq!(
            run(vec![
                OsString::from("ebound"),
                OsString::from("sidecar"),
                source.as_os_str().to_owned(),
            ])
            .unwrap_err()
            .code(),
            ReasonCode::ExtractionCollision
        );
        assert_eq!(std::fs::read(&source).unwrap(), source_bytes);
        for (name, bytes, explicit, layout) in [
            (
                "release.tar.zst",
                zstd::stream::encode_all(store_tar(b"payload.bin", b"tar sidecar").as_slice(), 1)
                    .unwrap(),
                "--from=tar.zst",
                "stream",
            ),
            (
                "release.7z",
                store_sevenz("payload.bin", b"7z sidecar"),
                "--from=7z",
                "indexed",
            ),
        ] {
            let legacy = root.join(name);
            std::fs::write(&legacy, &bytes).unwrap();
            run(vec![
                OsString::from("ebound"),
                OsString::from("sidecar"),
                legacy.as_os_str().to_owned(),
                OsString::from(explicit),
                OsString::from("--profile=fast"),
                OsString::from("--layout"),
                OsString::from(layout),
            ])
            .unwrap();
            let mut sidecar_name = legacy.as_os_str().to_os_string();
            sidecar_name.push(".eb");
            let sidecar_bytes = std::fs::read(std::path::PathBuf::from(sidecar_name)).unwrap();
            let opened = if layout == "stream" {
                entrybound::ecf::open_stream_with_limits(
                    std::io::Cursor::new(sidecar_bytes),
                    entrybound::ecf::SequentialLimits {
                        content: entrybound::ecf::StreamContentPolicy::Retain,
                        ..entrybound::ecf::bootstrap_sequential_limits()
                    },
                )
                .unwrap()
                .opened
            } else {
                entrybound::ecf::open(&sidecar_bytes).unwrap()
            };
            assert_eq!(
                opened.archive.conversion.as_ref().unwrap().source_digest,
                entrybound::identity::sha256_exact(&bytes)
            );
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn encrypted_export_requires_authentication_and_records_security_transition() {
        let id = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "entrybound-cli-encrypted-export-{}-{id}",
            std::process::id()
        ));
        let input = root.join("input");
        std::fs::create_dir_all(&input).unwrap();
        std::fs::write(input.join("secret.txt"), b"authenticated export").unwrap();
        let (identity, recipient) = entrybound::crypto::XWingIdentity::generate().unwrap();
        let encrypted = entrybound::crypto::pack_directory_encrypted(
            &input,
            entrybound::archive::PackOptions::default(),
            entrybound::crypto::EncryptedWriteOptions {
                recipients: std::slice::from_ref(&recipient),
                ..entrybound::crypto::EncryptedWriteOptions::default()
            },
        )
        .unwrap();
        let source = root.join("encrypted.eb");
        std::fs::write(&source, encrypted.bytes).unwrap();
        let identity_path = root.join("identity.key");
        std::fs::write(&identity_path, identity.encode_file().unwrap().as_slice()).unwrap();

        let refused = root.join("no-key.zip");
        assert_eq!(
            run(vec![
                OsString::from("ebound"),
                OsString::from("convert"),
                source.as_os_str().to_owned(),
                refused.as_os_str().to_owned(),
                OsString::from("--to=zip"),
                OsString::from("--allow-lossy"),
            ])
            .unwrap_err()
            .code(),
            ReasonCode::CommandUsage
        );
        assert!(!refused.exists());

        let target = root.join("authenticated.zip");
        let receipt = root.join("authenticated.receipt.json");
        run(vec![
            OsString::from("ebound"),
            OsString::from("convert"),
            source.as_os_str().to_owned(),
            target.as_os_str().to_owned(),
            OsString::from("--to=zip"),
            OsString::from("--allow-lossy"),
            OsString::from("--identity"),
            identity_path.as_os_str().to_owned(),
            OsString::from("--receipt"),
            receipt.as_os_str().to_owned(),
        ])
        .unwrap();
        let receipt = std::fs::read_to_string(receipt).unwrap();
        assert!(receipt.contains("\"encrypted\":true"));
        assert!(receipt.contains("\"target_encrypted\":false"));
        assert!(target.exists());
        let publish_dir = root.join("published");
        std::fs::create_dir(&publish_dir).unwrap();
        let migration = publish_dir.join("secure.migration.json");
        run(vec![
            OsString::from("ebound"),
            OsString::from("publish"),
            source.as_os_str().to_owned(),
            OsString::from("--output-dir"),
            publish_dir.as_os_str().to_owned(),
            OsString::from("--native"),
            OsString::from("--target=tar.gz"),
            OsString::from("--base-name=secure"),
            OsString::from("--allow-lossy"),
            OsString::from("--identity"),
            identity_path.as_os_str().to_owned(),
            OsString::from("--report"),
            migration.as_os_str().to_owned(),
        ])
        .unwrap();
        assert!(publish_dir.join("secure.eb").is_file());
        assert!(publish_dir.join("secure.tar.gz").is_file());
        let migration = std::fs::read_to_string(migration).unwrap();
        assert!(migration.contains("\"encrypted\":true"));
        assert!(migration.contains("tar.gz/pax-v1"));
        let plain = entrybound::archive::pack_directory(
            &input,
            entrybound::archive::PackOptions::default(),
        )
        .unwrap();
        let plain_target = entrybound::legacy::export::prepare_export(
            &plain.archive,
            entrybound::legacy::export::ExportTarget::ZipPortableV1,
            entrybound::legacy::export::ExportSourceSecurity::default(),
        )
        .unwrap()
        .accept(true)
        .unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), plain_target.bytes);
        let plain_tar_gzip = entrybound::legacy::export::prepare_export(
            &plain.archive,
            entrybound::legacy::export::ExportTarget::TarGzipPaxV1,
            entrybound::legacy::export::ExportSourceSecurity::default(),
        )
        .unwrap()
        .accept(true)
        .unwrap();
        assert_eq!(
            std::fs::read(publish_dir.join("secure.tar.gz")).unwrap(),
            plain_tar_gzip.bytes
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn read_command_materializes_only_a_verified_indexed_entry() {
        let id = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "entrybound-cli-random-read-{}-{id}",
            std::process::id()
        ));
        let input = root.join("input");
        std::fs::create_dir_all(&input).unwrap();
        std::fs::write(input.join("wanted.txt"), b"CLI verified range content").unwrap();
        std::fs::write(input.join("unread.bin"), vec![42_u8; 1024 * 1024]).unwrap();
        let archive = root.join("source.eb");
        let output = root.join("wanted.out");
        run(vec![
            OsString::from("ebound"),
            OsString::from("pack"),
            input.as_os_str().to_owned(),
            archive.as_os_str().to_owned(),
            OsString::from("--profile"),
            OsString::from("fast"),
        ])
        .unwrap();
        run(vec![
            OsString::from("ebound"),
            OsString::from("read"),
            archive.as_os_str().to_owned(),
            OsString::from("wanted.txt"),
            OsString::from("--output"),
            output.as_os_str().to_owned(),
            OsString::from("--access-report"),
        ])
        .unwrap();
        assert_eq!(
            std::fs::read(output).unwrap(),
            b"CLI verified range content"
        );
        let planned = entrybound::archive::plan_directory(
            &input,
            entrybound::archive::PackOptions::default(),
        )
        .unwrap();
        let (identity, recipient) = entrybound::crypto::XWingIdentity::generate().unwrap();
        let encrypted = entrybound::crypto::encrypt_archive(
            &planned,
            entrybound::crypto::EncryptedWriteOptions {
                recipients: &[recipient],
                ..entrybound::crypto::EncryptedWriteOptions::default()
            },
        )
        .unwrap();
        let encrypted_path = root.join("encrypted.eb");
        let identity_path = root.join("identity.key");
        let encrypted_output = root.join("encrypted-wanted.out");
        std::fs::write(&encrypted_path, encrypted.bytes).unwrap();
        std::fs::write(&identity_path, identity.encode_file().unwrap()).unwrap();
        run(vec![
            OsString::from("ebound"),
            OsString::from("read"),
            encrypted_path.as_os_str().to_owned(),
            OsString::from("wanted.txt"),
            OsString::from("--identity"),
            identity_path.as_os_str().to_owned(),
            OsString::from("--output"),
            encrypted_output.as_os_str().to_owned(),
        ])
        .unwrap();
        assert_eq!(
            std::fs::read(encrypted_output).unwrap(),
            b"CLI verified range content"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn repack_diff_structured_inspect_and_entry_explain_workflow() {
        let id = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "entrybound-cli-native-tooling-{}-{id}",
            std::process::id()
        ));
        let input = root.join("input");
        std::fs::create_dir_all(input.join("nested")).unwrap();
        std::fs::write(
            input.join("nested/file.txt"),
            b"native tooling keeps semantic bytes",
        )
        .unwrap();
        let indexed = root.join("source.eb");
        let stream = root.join("stream.eb");
        let dense = root.join("dense.eb");
        let dry = root.join("dry.eb");
        run(vec![
            OsString::from("ebound"),
            OsString::from("pack"),
            input.as_os_str().to_owned(),
            indexed.as_os_str().to_owned(),
            OsString::from("--profile"),
            OsString::from("fast"),
        ])
        .unwrap();
        run(vec![
            OsString::from("ebound"),
            OsString::from("repack"),
            indexed.as_os_str().to_owned(),
            stream.as_os_str().to_owned(),
            OsString::from("--layout"),
            OsString::from("stream"),
            OsString::from("--stream-window"),
            OsString::from("auto"),
        ])
        .unwrap();
        run(vec![
            OsString::from("ebound"),
            OsString::from("repack"),
            stream.as_os_str().to_owned(),
            dense.as_os_str().to_owned(),
            OsString::from("--layout"),
            OsString::from("indexed"),
            OsString::from("--profile"),
            OsString::from("dense"),
            OsString::from("--index"),
            OsString::from("absent"),
        ])
        .unwrap();
        run(vec![
            OsString::from("ebound"),
            OsString::from("repack"),
            indexed.as_os_str().to_owned(),
            dry.as_os_str().to_owned(),
            OsString::from("--profile"),
            OsString::from("balanced"),
            OsString::from("--dry-run"),
        ])
        .unwrap();
        assert!(!dry.exists());
        run(vec![
            OsString::from("ebound"),
            OsString::from("diff"),
            indexed.as_os_str().to_owned(),
            stream.as_os_str().to_owned(),
            OsString::from("--json"),
        ])
        .unwrap();
        run(vec![
            OsString::from("ebound"),
            OsString::from("inspect"),
            dense.as_os_str().to_owned(),
            OsString::from("--json"),
            OsString::from("--plans"),
            OsString::from("--chunks"),
        ])
        .unwrap();
        run(vec![
            OsString::from("ebound"),
            OsString::from("explain"),
            dense.as_os_str().to_owned(),
            OsString::from("nested/file.txt"),
        ])
        .unwrap();
        run(vec![
            OsString::from("ebound"),
            OsString::from("verify"),
            dense.as_os_str().to_owned(),
        ])
        .unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }
}
