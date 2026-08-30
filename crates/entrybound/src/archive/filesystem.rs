use std::collections::BTreeMap;
use std::ffi::{OsStr, OsString};
use std::fs::FileTimes;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use cap_std::ambient_authority;
#[cfg(unix)]
use cap_std::fs::PermissionsExt;
use cap_std::fs::{Dir, DirEntry, Metadata, OpenOptions};

use super::{CollisionPolicy, ConfinementMode, ExtractionPolicy};
use crate::chunker::{chunk_ranges, select_parameters};
use crate::diagnostics::{Diagnostic, OutcomeClass, ReasonCode, Result};
use crate::eam::{
    Archive, ArchiveDescriptor, ArchiveRole, ContentRef, ContentStore, DecodeRequirements, Digest,
    DigestAlgorithm, Entry, EntryData, EntryIdentity, EntrySet, FeatureSet, FidelityIssue,
    FidelityReport, IdentityProfile, Index, Layout, LogicalPath, MetadataItem, MetadataSet,
    ResourceBudget, Timestamp, TimestampPrecision,
};
use crate::ecf::{EncodedArchive, WriteOptions, encode, open_with_limits};
use crate::identity::{build_content_from_ranges, sha256_exact};
use crate::planner::{CompressionProfile, UNPLANNED_PLAN_ID, plan_archive_v2};

/// Bounded source-consistency and writer options for filesystem packing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PackOptions {
    /// Number of fresh-handle retries after the initial capture attempt.
    pub source_retries: usize,
    pub include_index: bool,
    /// Creation-time policy. It is resolved into recorded TransformPlans.
    pub profile: CompressionProfile,
}

impl Default for PackOptions {
    fn default() -> Self {
        Self {
            source_retries: 2,
            include_index: true,
            profile: CompressionProfile::Balanced,
        }
    }
}

/// Extraction outcome, including the confinement achieved and fidelity gaps.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExtractionReport {
    pub entries_created: u64,
    pub logical_bytes_written: u64,
    pub confinement: ConfinementMode,
    pub metadata_not_restored: Vec<String>,
}

/// Builds a validated EAM from a local directory and invokes the native writer.
pub fn pack_directory(input: &Path, options: PackOptions) -> Result<EncodedArchive> {
    let root_metadata = std::fs::symlink_metadata(input).map_err(|error| {
        Diagnostic::new(
            OutcomeClass::PolicyRefused,
            ReasonCode::InputNotDirectory,
            format!(
                "cannot inspect input directory '{}': {error}",
                input.display()
            ),
        )
    })?;
    if root_metadata.file_type().is_symlink() {
        return Err(unsupported(input.display().to_string(), "symbolic link"));
    }
    if !root_metadata.is_dir() {
        return Err(Diagnostic::new(
            OutcomeClass::PolicyRefused,
            ReasonCode::InputNotDirectory,
            format!("'{}' is not a directory", input.display()),
        ));
    }
    let root = Dir::open_ambient_dir(input, ambient_authority()).map_err(|error| {
        let code = if error.kind() == std::io::ErrorKind::NotADirectory {
            ReasonCode::InputNotDirectory
        } else {
            ReasonCode::Io
        };
        Diagnostic::new(
            OutcomeClass::PolicyRefused,
            code,
            format!("cannot open input directory '{}': {error}", input.display()),
        )
    })?;
    let mut scan = Scan::default();
    scan_directory(&root, &[], options.source_retries, &mut scan)?;
    let archive = scan.finish(options.profile)?;
    encode(
        &archive,
        WriteOptions {
            include_index: options.include_index,
        },
    )
}

/// Predictable CLI output name: the input's final name plus `.eb` in the CWD.
#[must_use]
pub fn default_pack_output(input: &Path) -> PathBuf {
    let mut name = input
        .file_name()
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| OsStr::new("archive"))
        .to_os_string();
    name.push(".eb");
    PathBuf::from(name)
}

/// Predictable unpack destination beside the archive, without its extension.
#[must_use]
pub fn default_unpack_destination(archive: &Path) -> PathBuf {
    if archive.extension().is_some() {
        archive.with_extension("")
    } else {
        let mut destination = archive.as_os_str().to_os_string();
        destination.push(".unpacked");
        PathBuf::from(destination)
    }
}

/// Fully opens and verifies an archive, then extracts it beneath a held root.
pub fn unpack(
    bytes: &[u8],
    destination: &Path,
    policy: ExtractionPolicy,
) -> Result<ExtractionReport> {
    if policy.collision() != CollisionPolicy::Refuse {
        return Err(Diagnostic::new(
            OutcomeClass::PolicyRefused,
            ReasonCode::ExtractionCollision,
            "the bootstrap extractor supports only collision refusal",
        ));
    }
    let opened = open_with_limits(bytes, policy.budget(), policy.decode())?;

    match std::fs::create_dir(destination) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            if !destination.is_dir() {
                return Err(collision(format!(
                    "destination root '{}' is not a directory",
                    destination.display()
                )));
            }
        }
        Err(error) => return Err(io("create destination root", error)),
    }
    let root = Dir::open_ambient_dir(destination, ambient_authority())
        .map_err(|error| containment(format!("cannot hold destination root: {error}")))?;

    let mut report = ExtractionReport {
        entries_created: 0,
        logical_bytes_written: 0,
        confinement: ConfinementMode::KernelEnforced,
        metadata_not_restored: Vec::new(),
    };

    for entry in opened.archive.entry_set.entries() {
        let (parent, name) = resolve_parent(&root, entry.path())?;
        ensure_absent(&parent, name, entry.path())?;
        match entry.data() {
            EntryData::Directory => {
                parent.create_dir(name).map_err(|error| {
                    if error.kind() == std::io::ErrorKind::AlreadyExists
                        || parent.symlink_metadata(name).is_ok()
                    {
                        collision(entry.path().to_string())
                    } else {
                        io(format!("create directory {}", entry.path()), error)
                    }
                })?;
            }
            EntryData::File {
                content: ContentRef::Internal(digest),
            } => {
                let mut options = OpenOptions::new();
                options.write(true).create_new(true);
                let mut file = parent.open_with(name, &options).map_err(|error| {
                    if error.kind() == std::io::ErrorKind::AlreadyExists
                        || parent.symlink_metadata(name).is_ok()
                    {
                        collision(entry.path().to_string())
                    } else {
                        io(format!("create file {}", entry.path()), error)
                    }
                })?;
                let object = opened
                    .archive
                    .content_store
                    .objects
                    .get(&digest)
                    .ok_or_else(|| {
                        Diagnostic::new(
                            OutcomeClass::Nonconforming,
                            ReasonCode::UnknownContentObject,
                            entry.path().to_string(),
                        )
                    })?;
                for chunk_ref in &object.chunks {
                    let chunk = opened
                        .archive
                        .content_store
                        .chunks
                        .get(&chunk_ref.chunk_id)
                        .ok_or_else(|| {
                            Diagnostic::new(
                                OutcomeClass::Nonconforming,
                                ReasonCode::UnknownChunk,
                                chunk_ref.chunk_id.to_string(),
                            )
                        })?;
                    file.write_all(&chunk.plaintext)
                        .map_err(|error| io(format!("write file {}", entry.path()), error))?;
                    report.logical_bytes_written = report
                        .logical_bytes_written
                        .checked_add(chunk.logical_len)
                        .ok_or_else(|| resource("extracted byte count exceeds u64"))?;
                }
                apply_file_metadata(
                    file.into_std(),
                    entry.path().to_string(),
                    entry.metadata(),
                    &mut report,
                );
            }
        }
        report.entries_created = report
            .entries_created
            .checked_add(1)
            .ok_or_else(|| resource("extracted entry count exceeds u64"))?;
    }

    for entry in opened.archive.entry_set.entries().iter().rev() {
        if matches!(entry.data(), EntryData::Directory) {
            let directory = resolve_directory(&root, entry.path())?;
            apply_file_metadata(
                directory.into_std_file(),
                entry.path().to_string(),
                entry.metadata(),
                &mut report,
            );
        }
    }
    Ok(report)
}

#[derive(Default)]
struct Scan {
    entries: Vec<Entry>,
    files: Vec<Box<[u8]>>,
}

impl Scan {
    fn finish(self, profile: CompressionProfile) -> Result<Archive> {
        let contents = self.files.iter().map(Box::as_ref).collect::<Vec<_>>();
        let selection = select_parameters(&contents, profile.chunking_candidates())?;
        let mut objects = BTreeMap::new();
        let mut chunks = BTreeMap::new();
        for plaintext in &self.files {
            let ranges = chunk_ranges(plaintext, selection.parameters)?
                .iter()
                .map(|range| range.start..range.end)
                .collect::<Vec<_>>();
            let (object, object_chunks) =
                build_content_from_ranges(plaintext, &ranges, UNPLANNED_PLAN_ID)?;
            match objects.entry(object.logical_digest) {
                std::collections::btree_map::Entry::Vacant(entry) => {
                    entry.insert(object);
                }
                std::collections::btree_map::Entry::Occupied(entry) if entry.get() != &object => {
                    return Err(Diagnostic::new(
                        OutcomeClass::Corrupt,
                        ReasonCode::ContentDigestMismatch,
                        format!(
                            "distinct ContentObjects produced logical digest {}",
                            object.logical_digest
                        ),
                    ));
                }
                std::collections::btree_map::Entry::Occupied(_) => {}
            }
            for (chunk_id, chunk) in object_chunks {
                match chunks.entry(chunk_id) {
                    std::collections::btree_map::Entry::Vacant(entry) => {
                        entry.insert(chunk);
                    }
                    std::collections::btree_map::Entry::Occupied(entry)
                        if entry.get() != &chunk =>
                    {
                        return Err(Diagnostic::new(
                            OutcomeClass::Corrupt,
                            ReasonCode::ChunkIdentityCollision,
                            format!("distinct plaintext Chunks produced ID {chunk_id}"),
                        ));
                    }
                    std::collections::btree_map::Entry::Occupied(_) => {}
                }
            }
        }
        let mut archive = Archive {
            descriptor: ArchiveDescriptor {
                format_major: 0,
                format_minor: 1,
                format_namespace: crate::ecf::FORMAT_NAMESPACE.to_owned(),
                features: FeatureSet::default(),
                layout: Layout::Indexed,
                role: ArchiveRole::Complete,
                budget_declared: true,
                budget: ResourceBudget::default(),
                decode: DecodeRequirements::default(),
                identity_profile: IdentityProfile::IdentityV1,
                digest_algorithm: DigestAlgorithm::Sha256,
                planner_id: profile.planner_id().to_owned(),
                chunker_id: selection.parameters.chunker_id.to_owned(),
                lai: Digest::ZERO,
                pcr: Digest::ZERO,
                aux: Digest::ZERO,
                pci: None,
            },
            entry_set: EntrySet::new(self.entries)?,
            content_store: ContentStore { objects, chunks },
            transform_plans: Box::default(),
            fidelity: bootstrap_fidelity(),
            index: Index::default(),
        };
        plan_archive_v2(&mut archive, profile)?;
        Ok(archive)
    }
}

fn scan_directory(
    directory: &Dir,
    ancestors: &[String],
    retries: usize,
    scan: &mut Scan,
) -> Result<()> {
    let mut entries = directory
        .entries()
        .map_err(|error| io("enumerate source directory", error))?
        .collect::<std::io::Result<Vec<_>>>()
        .map_err(|error| io("enumerate source directory", error))?;
    entries.sort_by_key(DirEntry::file_name);

    for source_entry in entries {
        let name = utf8_name(source_entry.file_name())?;
        let mut components = ancestors.to_vec();
        components.push(name);
        let path = LogicalPath::from_utf8(&components)?;
        let file_type = source_entry
            .file_type()
            .map_err(|error| io(format!("inspect source entry {path}"), error))?;
        if file_type.is_symlink() {
            return Err(unsupported(path.to_string(), "symbolic link"));
        }
        if file_type.is_dir() {
            let child = source_entry
                .open_dir()
                .map_err(|error| io(format!("open source directory {path}"), error))?;
            let metadata = child
                .dir_metadata()
                .map_err(|error| io(format!("inspect source directory {path}"), error))?;
            scan.entries.push(Entry::new(
                path,
                EntryData::Directory,
                metadata_set(&metadata)?,
                EntryIdentity::default(),
            ));
            scan_directory(&child, &components, retries, scan)?;
        } else if file_type.is_file() {
            let (plaintext, metadata) =
                capture_entry_with_probe(&source_entry, retries, |_| Ok(false))?;
            let digest = sha256_exact(&plaintext);
            scan.entries.push(Entry::new(
                path,
                EntryData::File {
                    content: ContentRef::Internal(digest),
                },
                metadata_set(&metadata)?,
                EntryIdentity::default(),
            ));
            scan.files.push(plaintext.into_boxed_slice());
        } else {
            return Err(unsupported(path.to_string(), "special filesystem object"));
        }
    }
    Ok(())
}

fn capture_entry_with_probe<F>(
    entry: &DirEntry,
    retries: usize,
    mut additional_change_probe: F,
) -> Result<(Vec<u8>, Metadata)>
where
    F: FnMut(usize) -> Result<bool>,
{
    for attempt in 0..=retries {
        let mut file = entry
            .open()
            .map_err(|error| io("open source file", error))?;
        let before = file
            .metadata()
            .map_err(|error| io("inspect opened source file", error))?;
        if !before.is_file() {
            return Err(unsupported(
                entry.file_name().to_string_lossy(),
                "non-regular opened object",
            ));
        }
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)
            .map_err(|error| io("read opened source file", error))?;
        let after = file
            .metadata()
            .map_err(|error| io("reinspect opened source file", error))?;
        let changed = before.len() != after.len()
            || after.len() != u64::try_from(bytes.len()).unwrap_or(u64::MAX)
            || before.modified().ok() != after.modified().ok()
            || executable(&before) != executable(&after)
            || additional_change_probe(attempt)?;
        if !changed {
            return Ok((bytes, after));
        }
    }
    Err(Diagnostic::new(
        OutcomeClass::PolicyRefused,
        ReasonCode::SourceUnstable,
        format!(
            "source '{}' changed during every bounded capture attempt",
            entry.file_name().to_string_lossy()
        ),
    ))
}

fn metadata_set(metadata: &Metadata) -> Result<MetadataSet> {
    let modified = metadata
        .modified()
        .map_err(|error| io("read source modification time", error))?
        .into_std();
    MetadataSet::new(vec![
        MetadataItem::executable(executable(metadata)),
        MetadataItem::mtime(timestamp(modified)?),
    ])
}

#[cfg(unix)]
fn executable(metadata: &Metadata) -> bool {
    metadata.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn executable(_metadata: &Metadata) -> bool {
    false
}

fn timestamp(value: SystemTime) -> Result<Timestamp> {
    let (seconds, nanoseconds) = match value.duration_since(UNIX_EPOCH) {
        Ok(duration) => (
            i64::try_from(duration.as_secs()).map_err(|_| resource("mtime seconds exceed i64"))?,
            duration.subsec_nanos(),
        ),
        Err(error) => {
            let duration = error.duration();
            let seconds = i64::try_from(duration.as_secs())
                .map_err(|_| resource("pre-epoch mtime seconds exceed i64"))?;
            if duration.subsec_nanos() == 0 {
                (-seconds, 0)
            } else {
                (
                    seconds
                        .checked_add(1)
                        .and_then(|value| value.checked_neg())
                        .ok_or_else(|| resource("pre-epoch mtime overflow"))?,
                    1_000_000_000 - duration.subsec_nanos(),
                )
            }
        }
    };
    Timestamp::new(seconds, nanoseconds, native_timestamp_precision(), true)
}

const fn native_timestamp_precision() -> TimestampPrecision {
    if cfg!(windows) {
        TimestampPrecision::Hectonanosecond
    } else {
        TimestampPrecision::Nanosecond
    }
}

fn bootstrap_fidelity() -> FidelityReport {
    let mut captured = vec!["core.mtime".to_owned()];
    #[cfg(unix)]
    captured.push("core.executable".to_owned());
    captured.sort();
    let mut unavailable = [
        "acl",
        "extended-attributes",
        "ownership",
        "hardlink-identity",
        "platform-specific-metadata",
        "symlinks-and-special-files",
    ]
    .into_iter()
    .map(|class| FidelityIssue {
        class: class.to_owned(),
        reason: "not captured by the native bootstrap filesystem slice".to_owned(),
        entry_scope: None,
    })
    .collect::<Vec<_>>();
    #[cfg(not(unix))]
    unavailable.push(FidelityIssue {
        class: "core.executable".to_owned(),
        reason: "the source platform does not represent a portable executable bit".to_owned(),
        entry_scope: None,
    });
    unavailable.sort_by(|left, right| left.class.cmp(&right.class));
    FidelityReport {
        captured: captured.into_boxed_slice(),
        unavailable: unavailable.into_boxed_slice(),
        degraded: Box::default(),
        platform: std::env::consts::OS.to_owned(),
        filesystem: Box::default(),
    }
}

fn resolve_parent<'a>(root: &Dir, path: &'a LogicalPath) -> Result<(Dir, &'a OsStr)> {
    let (name, parents) = path
        .components()
        .split_last()
        .ok_or_else(|| containment("LogicalPath unexpectedly had no components"))?;
    let mut directory = root
        .try_clone()
        .map_err(|error| containment(format!("cannot clone destination root: {error}")))?;
    for component in parents {
        directory = directory
            .open_dir(component_os(component.bytes())?)
            .map_err(|error| containment(format!("cannot resolve parent of {path}: {error}")))?;
    }
    Ok((directory, component_os(name.bytes())?))
}

fn resolve_directory(root: &Dir, path: &LogicalPath) -> Result<Dir> {
    let mut directory = root
        .try_clone()
        .map_err(|error| containment(format!("cannot clone destination root: {error}")))?;
    for component in path.components() {
        directory = directory
            .open_dir(component_os(component.bytes())?)
            .map_err(|error| containment(format!("cannot reopen directory {path}: {error}")))?;
    }
    Ok(directory)
}

fn ensure_absent(parent: &Dir, name: &OsStr, path: &LogicalPath) -> Result<()> {
    match parent.symlink_metadata(name) {
        Ok(_) => Err(collision(path.to_string())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(io(format!("inspect destination for {path}"), error)),
    }
}

fn component_os(bytes: &[u8]) -> Result<&OsStr> {
    std::str::from_utf8(bytes).map(OsStr::new).map_err(|_| {
        Diagnostic::new(
            OutcomeClass::Unsupported,
            ReasonCode::InvalidPathComponent,
            "bootstrap extraction requires UTF-8 path components",
        )
    })
}

fn apply_file_metadata(
    file: std::fs::File,
    path: String,
    metadata: &MetadataSet,
    report: &mut ExtractionReport,
) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        match file.metadata() {
            Ok(file_metadata) => {
                let mut permissions = file_metadata.permissions();
                let mut mode = permissions.mode();
                if metadata.executable() {
                    mode |= 0o100;
                } else {
                    mode &= !0o111;
                }
                permissions.set_mode(mode);
                if let Err(error) = file.set_permissions(permissions) {
                    report
                        .metadata_not_restored
                        .push(format!("{path}: core.executable ({error})"));
                }
            }
            Err(error) => report
                .metadata_not_restored
                .push(format!("{path}: core.executable ({error})")),
        }
    }
    #[cfg(not(unix))]
    if metadata.executable() {
        report.metadata_not_restored.push(format!(
            "{path}: core.executable is not restorable on this platform"
        ));
    }

    if let Some(value) = metadata.mtime() {
        match system_time(value) {
            Some(modified) => {
                let times = FileTimes::new().set_modified(modified);
                if let Err(error) = file.set_times(times) {
                    report
                        .metadata_not_restored
                        .push(format!("{path}: core.mtime ({error})"));
                }
            }
            None => report
                .metadata_not_restored
                .push(format!("{path}: core.mtime is outside platform range")),
        }
    }
}

fn system_time(timestamp: Timestamp) -> Option<SystemTime> {
    if timestamp.seconds() >= 0 {
        UNIX_EPOCH.checked_add(Duration::new(
            u64::try_from(timestamp.seconds()).ok()?,
            timestamp.nanoseconds(),
        ))
    } else if timestamp.nanoseconds() == 0 {
        UNIX_EPOCH.checked_sub(Duration::from_secs(timestamp.seconds().unsigned_abs()))
    } else {
        UNIX_EPOCH.checked_sub(Duration::new(
            timestamp.seconds().unsigned_abs().checked_sub(1)?,
            1_000_000_000 - timestamp.nanoseconds(),
        ))
    }
}

fn utf8_name(name: OsString) -> Result<String> {
    name.into_string().map_err(|name| {
        Diagnostic::new(
            OutcomeClass::Unsupported,
            ReasonCode::InvalidPathComponent,
            format!(
                "filesystem name '{}' is not valid UTF-8",
                name.to_string_lossy()
            ),
        )
    })
}

fn unsupported(path: impl Into<String>, kind: &str) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::Unsupported,
        ReasonCode::UnsupportedEntryKind,
        format!("{}: {kind} is not supported", path.into()),
    )
}

fn collision(path: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::PolicyRefused,
        ReasonCode::ExtractionCollision,
        format!("refusing to replace existing destination '{}'", path.into()),
    )
}

fn containment(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::PolicyRefused,
        ReasonCode::ExtractionContainmentUnavailable,
        detail,
    )
}

fn io(context: impl AsRef<str>, error: std::io::Error) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::PolicyRefused,
        ReasonCode::Io,
        format!("{}: {error}", context.as_ref()),
    )
}

fn resource(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::PolicyRefused,
        ReasonCode::ResourceLimit,
        detail,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn unstable_source_has_a_deterministic_bounded_failure() {
        let root_path = std::env::temp_dir().join(format!(
            "entrybound-unstable-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&root_path).unwrap();
        std::fs::write(root_path.join("changing"), b"stable bytes").unwrap();
        let root = Dir::open_ambient_dir(&root_path, ambient_authority()).unwrap();
        let entry = root.entries().unwrap().next().unwrap().unwrap();
        let mut probes = 0;
        let error = capture_entry_with_probe(&entry, 2, |_| {
            probes += 1;
            Ok(true)
        })
        .unwrap_err();
        assert_eq!(error.code(), ReasonCode::SourceUnstable);
        assert_eq!(probes, 3);
        drop(entry);
        drop(root);
        std::fs::remove_dir_all(root_path).unwrap();
    }

    #[test]
    fn timestamp_round_trip_handles_both_sides_of_epoch() {
        for value in [
            UNIX_EPOCH + Duration::new(12, 34),
            UNIX_EPOCH - Duration::new(12, 34),
        ] {
            assert_eq!(system_time(timestamp(value).unwrap()), Some(value));
        }
    }
}
