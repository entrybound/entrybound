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
use crate::chunker::{
    EncryptedBoundaryKey, chunk_ranges, chunk_ranges_encrypted, select_parameters,
    select_parameters_encrypted,
};
use crate::diagnostics::{Diagnostic, OutcomeClass, ReasonCode, Result};
use crate::eam::{
    Archive, ArchiveDescriptor, ArchiveRole, ContentRef, ContentStore, ConversionProvenance,
    DecodeRequirements, Digest, DigestAlgorithm, Entry, EntryData, EntryIdentity, EntrySet,
    FeatureSet, FidelityIssue, FidelityReport, IdentityProfile, Index, Layout, LogicalPath,
    MetadataItem, MetadataSet, ResourceBudget, Timestamp, TimestampPrecision,
};
use crate::ecf::{
    EncodedArchive, FEATURE_CONVERSION_PROVENANCE_V1, SequentialLimits, StagedChunks,
    StreamContentPolicy, StreamReport, StreamWriteOptions, StreamWriteSummary, WriteOptions,
    encode, encode_stream, open_stream_with_limits, open_with_limits,
};
use crate::identity::{build_content_from_ranges, sha256_exact};
use crate::planner::{CompressionProfile, UNPLANNED_PLAN_ID, plan_archive_v6};

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
    let archive = build_archive(input, options)?;
    encode(
        &archive,
        WriteOptions {
            include_index: options.include_index,
        },
    )
}

/// Builds the same validated EAM and writes it as STREAM to any `Write` sink.
///
/// The sink is never seeked, so it may be a pipe or standard output. The
/// resulting archive has the same LAI, PCR, and AUX as the INDEXED encoding of
/// the same directory; only PCI differs.
pub fn pack_directory_stream<W: std::io::Write>(
    input: &Path,
    options: PackOptions,
    stream: StreamWriteOptions,
    sink: W,
) -> Result<StreamWriteSummary> {
    let archive = build_archive(input, options)?;
    encode_stream(&archive, stream, sink)
}

/// Scans a directory and returns the planned EAM without encoding it.
///
/// Callers that must choose or create their output only after planning has
/// succeeded use this, so a plan that cannot be represented never leaves a
/// partial artifact behind.
pub fn plan_directory(input: &Path, options: PackOptions) -> Result<Archive> {
    build_archive(input, options)
}

fn build_archive(input: &Path, options: PackOptions) -> Result<Archive> {
    build_archive_with_boundary(input, options, None)
}

pub(crate) fn plan_directory_encrypted(
    input: &Path,
    options: PackOptions,
    boundary: &EncryptedBoundaryKey,
    chunker_prefix: &'static str,
) -> Result<Archive> {
    build_archive_with_boundary(input, options, Some((boundary, chunker_prefix)))
}

/// Rebuilds only the encrypted physical plan from an already authenticated
/// EAM. Logical Entries, metadata, FidelityReport, and ContentObject plaintext
/// remain authoritative; AFK-derived boundaries and all physical plans are
/// regenerated for a fresh encryption epoch.
pub(crate) fn replan_archive_encrypted(
    source: &Archive,
    profile: CompressionProfile,
    boundary: &EncryptedBoundaryKey,
    chunker_prefix: &'static str,
) -> Result<Archive> {
    source.validate()?;
    let mut plaintext_objects = Vec::with_capacity(source.content_store.objects.len());
    for (digest, object) in &source.content_store.objects {
        let mut plaintext = Vec::new();
        for reference in &object.chunks {
            let chunk = source
                .content_store
                .chunks
                .get(&reference.chunk_id)
                .ok_or_else(|| {
                    Diagnostic::new(
                        OutcomeClass::Nonconforming,
                        ReasonCode::UnknownChunk,
                        format!("ContentObject {digest} references an unknown Chunk"),
                    )
                })?;
            plaintext.extend_from_slice(&chunk.plaintext);
        }
        if sha256_exact(&plaintext) != *digest {
            return Err(Diagnostic::new(
                OutcomeClass::Corrupt,
                ReasonCode::ContentDigestMismatch,
                format!("ContentObject {digest} plaintext does not match its digest"),
            ));
        }
        plaintext_objects.push((*digest, plaintext.into_boxed_slice()));
    }
    let contents = plaintext_objects
        .iter()
        .map(|(_, bytes)| bytes.as_ref())
        .collect::<Vec<_>>();
    let selection =
        select_parameters_encrypted(&contents, profile.chunking_candidates(), boundary)?;
    let mut objects = BTreeMap::new();
    let mut chunks = BTreeMap::new();
    for (expected_digest, plaintext) in plaintext_objects {
        let selected = chunk_ranges_encrypted(&plaintext, selection.parameters, boundary)?;
        let ranges = selected
            .iter()
            .map(|range| range.start..range.end)
            .collect::<Vec<_>>();
        let (object, object_chunks) =
            build_content_from_ranges(&plaintext, &ranges, UNPLANNED_PLAN_ID)?;
        if object.logical_digest != expected_digest {
            return Err(Diagnostic::new(
                OutcomeClass::Corrupt,
                ReasonCode::ContentDigestMismatch,
                "rechunking changed a ContentObject logical digest",
            ));
        }
        objects.insert(object.logical_digest, object);
        for (digest, chunk) in object_chunks {
            if let Some(existing) = chunks.insert(digest, chunk.clone())
                && existing != chunk
            {
                return Err(Diagnostic::new(
                    OutcomeClass::Corrupt,
                    ReasonCode::ChunkIdentityCollision,
                    format!("distinct plaintext Chunks produced ID {digest}"),
                ));
            }
        }
    }
    let mut descriptor = source.descriptor.clone();
    descriptor.features = FeatureSet::default();
    descriptor.layout = Layout::Indexed;
    descriptor.role = ArchiveRole::Complete;
    descriptor.budget_declared = true;
    descriptor.stream_dedup_window = 0;
    descriptor.budget = ResourceBudget::default();
    descriptor.decode = DecodeRequirements::default();
    descriptor.planner_id = profile.planner_id().to_owned();
    descriptor.chunker_id = format!(
        "{chunker_prefix}/min-{}/target-{}/max-{}",
        selection.parameters.minimum_size,
        selection.parameters.target_size,
        selection.parameters.maximum_size
    );
    descriptor.lai = Digest::ZERO;
    descriptor.pcr = Digest::ZERO;
    descriptor.aux = Digest::ZERO;
    descriptor.pci = None;
    let physical_order = chunks
        .keys()
        .copied()
        .collect::<Vec<_>>()
        .into_boxed_slice();
    let mut archive = Archive {
        descriptor,
        entry_set: source.entry_set.clone(),
        content_store: ContentStore {
            physical_order,
            objects,
            chunks,
            dictionaries: BTreeMap::new(),
            reconstruction_data: BTreeMap::new(),
            reconstruction_fallbacks: BTreeMap::new(),
            reconstruction_regions: BTreeMap::new(),
            reconstruction_audits: BTreeMap::new(),
            chunk_groups: BTreeMap::new(),
        },
        transform_plans: Box::default(),
        fidelity: source.fidelity.clone(),
        conversion: source.conversion.clone(),
        index: Index::default(),
    };
    plan_archive_v6(&mut archive, profile)?;
    Ok(archive)
}

fn build_archive_with_boundary(
    input: &Path,
    options: PackOptions,
    boundary: Option<(&EncryptedBoundaryKey, &'static str)>,
) -> Result<Archive> {
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
    scan.finish(options.profile, boundary)
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

/// Supplies verified plaintext for one Chunk during materialization.
///
/// Extraction never reads archive bytes through this trait; it reads content
/// that a reader has already decoded and digest-verified.
trait ChunkSource {
    fn plaintext(&mut self, chunk_id: &Digest) -> Result<Vec<u8>>;
}

/// The INDEXED reader retains every Chunk's plaintext in the opened model.
struct RetainedChunks<'a> {
    chunks: &'a BTreeMap<Digest, crate::eam::Chunk>,
}

impl ChunkSource for RetainedChunks<'_> {
    fn plaintext(&mut self, chunk_id: &Digest) -> Result<Vec<u8>> {
        self.chunks
            .get(chunk_id)
            .map(|chunk| chunk.plaintext.to_vec())
            .ok_or_else(|| {
                Diagnostic::new(
                    OutcomeClass::Nonconforming,
                    ReasonCode::UnknownChunk,
                    chunk_id.to_string(),
                )
            })
    }
}

/// The sequential reader stages plaintext in bounded temporary storage.
struct StagedSource<'a> {
    staged: &'a mut StagedChunks,
}

impl ChunkSource for StagedSource<'_> {
    fn plaintext(&mut self, chunk_id: &Digest) -> Result<Vec<u8>> {
        self.staged.read(chunk_id)
    }
}

/// Fully opens and verifies an archive, then extracts it beneath a held root.
pub fn unpack(
    bytes: &[u8],
    destination: &Path,
    policy: ExtractionPolicy,
) -> Result<ExtractionReport> {
    let opened = open_with_limits(bytes, policy.budget(), policy.decode())?;
    let chunks = opened.archive.content_store.chunks.clone();
    materialize(
        &opened.archive,
        &mut RetainedChunks { chunks: &chunks },
        destination,
        policy,
    )
}

/// Materializes a fully authenticated and verified opened archive.
///
/// Crypto callers use this only after recipient unlock, envelope/segment
/// authentication, and ordinary EAM verification have all succeeded, so a
/// failed credential or ciphertext never creates a destination object.
pub fn unpack_opened(
    opened: &crate::ecf::OpenedArchive,
    destination: &Path,
    policy: ExtractionPolicy,
) -> Result<ExtractionReport> {
    let chunks = opened.archive.content_store.chunks.clone();
    materialize(
        &opened.archive,
        &mut RetainedChunks { chunks: &chunks },
        destination,
        policy,
    )
}

/// Reads a STREAM archive from an unseekable source and extracts it safely.
///
/// The complete sequential pass runs first. Decoded content is held in bounded
/// staging, not in the destination, until framing, every Chunk digest, the EAM
/// invariants, the identity roots, the footer binding, and the exact total
/// length have all been established. Only then is anything created under the
/// caller's destination, using the same policy as INDEXED extraction. Staging
/// is released when the pass ends, including when it ends in failure or
/// truncation.
pub fn unpack_stream<R: std::io::Read>(
    source: R,
    destination: &Path,
    policy: ExtractionPolicy,
    limits: SequentialLimits,
) -> Result<(ExtractionReport, StreamReport)> {
    let mut sequential = open_stream_with_limits(
        source,
        SequentialLimits {
            budget: policy.budget(),
            decode: policy.decode(),
            content: StreamContentPolicy::Stage,
            ..limits
        },
    )?;
    let stream_report = sequential.stream;
    let mut staged = sequential.staged.take().ok_or_else(|| {
        Diagnostic::new(
            OutcomeClass::PolicyRefused,
            ReasonCode::Io,
            "the sequential pass produced no staged content to extract",
        )
    })?;
    let report = materialize(
        &sequential.opened.archive,
        &mut StagedSource {
            staged: &mut staged,
        },
        destination,
        policy,
    )?;
    Ok((report, stream_report))
}

/// Creates destination objects from content that is already fully verified.
fn materialize(
    archive: &Archive,
    source: &mut dyn ChunkSource,
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

    for entry in archive.entry_set.entries() {
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
                let object = archive.content_store.objects.get(&digest).ok_or_else(|| {
                    Diagnostic::new(
                        OutcomeClass::Nonconforming,
                        ReasonCode::UnknownContentObject,
                        entry.path().to_string(),
                    )
                })?;
                for chunk_ref in &object.chunks {
                    let plaintext = source.plaintext(&chunk_ref.chunk_id)?;
                    file.write_all(&plaintext)
                        .map_err(|error| io(format!("write file {}", entry.path()), error))?;
                    report.logical_bytes_written = report
                        .logical_bytes_written
                        .checked_add(
                            u64::try_from(plaintext.len())
                                .map_err(|_| resource("extracted Chunk exceeds u64"))?,
                        )
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

    for entry in archive.entry_set.entries().iter().rev() {
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
    fn finish(
        self,
        profile: CompressionProfile,
        boundary: Option<(&EncryptedBoundaryKey, &'static str)>,
    ) -> Result<Archive> {
        self.finish_with(profile, boundary, bootstrap_fidelity(), None)
    }

    fn finish_with(
        self,
        profile: CompressionProfile,
        boundary: Option<(&EncryptedBoundaryKey, &'static str)>,
        fidelity: FidelityReport,
        conversion: Option<ConversionProvenance>,
    ) -> Result<Archive> {
        let contents = self.files.iter().map(Box::as_ref).collect::<Vec<_>>();
        let selection = if let Some((boundary, _)) = boundary {
            select_parameters_encrypted(&contents, profile.chunking_candidates(), boundary)?
        } else {
            select_parameters(&contents, profile.chunking_candidates())?
        };
        let mut objects = BTreeMap::new();
        let mut chunks = BTreeMap::new();
        for plaintext in &self.files {
            let selected_ranges = if let Some((boundary, _)) = boundary {
                chunk_ranges_encrypted(plaintext, selection.parameters, boundary)?
            } else {
                chunk_ranges(plaintext, selection.parameters)?
            };
            let ranges = selected_ranges
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
                stream_dedup_window: 0,
                budget: ResourceBudget::default(),
                decode: DecodeRequirements::default(),
                identity_profile: IdentityProfile::IdentityV1,
                digest_algorithm: DigestAlgorithm::Sha256,
                planner_id: profile.planner_id().to_owned(),
                chunker_id: boundary.map_or_else(
                    || selection.parameters.chunker_id.to_owned(),
                    |(_, prefix)| {
                        format!(
                            "{prefix}/min-{}/target-{}/max-{}",
                            selection.parameters.minimum_size,
                            selection.parameters.target_size,
                            selection.parameters.maximum_size
                        )
                    },
                ),
                lai: Digest::ZERO,
                pcr: Digest::ZERO,
                aux: Digest::ZERO,
                pci: None,
            },
            entry_set: EntrySet::new(self.entries)?,
            content_store: ContentStore {
                physical_order: chunks
                    .keys()
                    .copied()
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
                objects,
                chunks,
                dictionaries: BTreeMap::new(),
                reconstruction_data: BTreeMap::new(),
                reconstruction_fallbacks: BTreeMap::new(),
                reconstruction_regions: BTreeMap::new(),
                reconstruction_audits: BTreeMap::new(),
                chunk_groups: BTreeMap::new(),
            },
            transform_plans: Box::default(),
            fidelity,
            conversion,
            index: Index::default(),
        };
        plan_archive_v6(&mut archive, profile)?;
        Ok(archive)
    }
}

/// Plans already-reconciled plaintext observations through the ordinary native v6 pipeline.
///
/// Legacy adapters use this only after they have resolved foreign evidence into valid EAM
/// entries. ZIP compression and ZIP structure never cross this boundary.
pub(crate) fn plan_observed_archive(
    entries: Vec<Entry>,
    files: Vec<Box<[u8]>>,
    fidelity: FidelityReport,
    conversion: ConversionProvenance,
    profile: CompressionProfile,
) -> Result<Archive> {
    let mut archive =
        Scan { entries, files }.finish_with(profile, None, fidelity, Some(conversion))?;
    archive.descriptor.features.incompat |= FEATURE_CONVERSION_PROVENANCE_V1;
    Ok(archive)
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
