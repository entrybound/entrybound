use std::collections::{BTreeMap, BTreeSet};
use std::ffi::{OsStr, OsString};
use std::fs::FileTimes;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use cap_std::ambient_authority;
#[cfg(windows)]
use cap_std::fs::MetadataExt;
use cap_std::fs::{Dir, DirEntry, Metadata, OpenOptions};
#[cfg(unix)]
use cap_std::fs::{MetadataExt, PermissionsExt};

#[cfg(unix)]
use super::{AclPolicy, OwnershipPolicy, XAttrPolicy};
use super::{
    CollisionPolicy, ConfinementMode, ExtractionPolicy, PlatformMetadataPolicy, ReparsePolicy,
    SparsePolicy, SymlinkPolicy, WindowsSecurityPolicy, bootstrap_resource_policy,
};
use crate::chunker::{
    EncryptedBoundaryKey, chunk_ranges, chunk_ranges_encrypted, select_parameters,
    select_parameters_encrypted,
};
use crate::diagnostics::{Diagnostic, OutcomeClass, ReasonCode, Result};
#[cfg(target_os = "linux")]
use crate::eam::SparseExtent;
use crate::eam::{
    Acl, Archive, ArchiveDescriptor, ArchiveRole, ContentRef, ContentStore, ConversionProvenance,
    DecodeRequirements, Digest, DigestAlgorithm, Entry, EntryData, EntryIdentity, EntrySet,
    FeatureSet, FidelityIssue, FidelityReport, IdentityProfile, Index, Layout, LinkTarget,
    LogicalPath, MetadataItem, MetadataSet, ResourceBudget, SparseMap, Timestamp,
    TimestampPrecision, XAttr,
};
#[cfg(target_os = "linux")]
use crate::eam::{AclDialect, AclEntry, AclEntryType, AclPrincipal, AclScope};
use crate::ecf::{
    EncodedArchive, FEATURE_CONVERSION_PROVENANCE_V1, FEATURE_PLATFORM_SECURITY_METADATA_V1,
    FEATURE_POSIX_METADATA_V1, SequentialLimits, StagedChunks, StreamContentPolicy, StreamReport,
    StreamWriteOptions, StreamWriteSummary, WriteOptions, encode, encode_stream,
    open_stream_with_limits, open_with_limits,
};
use crate::identity::{build_content_from_ranges, hardlink_group_id, sha256_exact};
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
    if source.conversion.is_some() {
        descriptor.features.incompat |= FEATURE_CONVERSION_PROVENANCE_V1;
    }
    if source.preservation.is_some() {
        descriptor.features.incompat |= crate::ecf::FEATURE_LEGACY_PRESERVATION_V1;
    }
    apply_metadata_features(source.entry_set.entries(), &mut descriptor.features);
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
        preservation: source.preservation.clone(),
        index: Index::default(),
    };
    plan_archive_v6(&mut archive, profile)?;
    Ok(archive)
}

/// Rebuilds the native physical plan from the verified logical objects of an
/// existing archive. This never uses the host filesystem as an intermediate
/// authority and deliberately discards every reconstructible planning object
/// before invoking the selected current planner.
pub fn replan_archive(source: &Archive, profile: CompressionProfile) -> Result<Archive> {
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
    let selection = select_parameters(&contents, profile.chunking_candidates())?;
    let mut objects = BTreeMap::new();
    let mut chunks = BTreeMap::new();
    for (expected_digest, plaintext) in plaintext_objects {
        let selected = chunk_ranges(&plaintext, selection.parameters)?;
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
    if source.conversion.is_some() {
        descriptor.features.incompat |= FEATURE_CONVERSION_PROVENANCE_V1;
    }
    if source.preservation.is_some() {
        descriptor.features.incompat |= crate::ecf::FEATURE_LEGACY_PRESERVATION_V1;
    }
    apply_metadata_features(source.entry_set.entries(), &mut descriptor.features);
    descriptor.layout = Layout::Indexed;
    descriptor.role = ArchiveRole::Complete;
    descriptor.budget_declared = true;
    descriptor.stream_dedup_window = 0;
    descriptor.budget = ResourceBudget::default();
    descriptor.decode = DecodeRequirements::default();
    descriptor.planner_id = profile.planner_id().to_owned();
    descriptor.chunker_id = format!(
        "entrybound/gear-cdc-v1/min-{}/target-{}/max-{}",
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
        preservation: source.preservation.clone(),
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
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt as _;
        if root_metadata.file_attributes() & 0x0000_0400 != 0 {
            return Err(Diagnostic::new(
                OutcomeClass::Unsupported,
                ReasonCode::InvalidReparsePoint,
                "source root is a Windows reparse point; exact no-follow reparse capture is unavailable",
            ));
        }
    }
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
    scan_directory(&root, input, &[], options.source_retries, &mut scan)?;
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

    for entry in archive.entry_set.entries() {
        if let EntryData::Symlink { target } = entry.data() {
            validate_symlink_policy(entry.path(), target, policy.symlinks())?;
        }
        if matches!(entry.data(), EntryData::ReparsePoint { .. }) {
            let detail = match policy.reparse() {
                ReparsePolicy::Refuse => "opaque Windows reparse extraction is refused by policy",
                ReparsePolicy::KnownSafe => {
                    "opaque Windows reparse Entries are not a recognized KnownSafe type"
                }
                ReparsePolicy::All => {
                    "exact opaque reparse restoration has no audited safe API in this build"
                }
            };
            return Err(Diagnostic::new(
                OutcomeClass::Unsupported,
                ReasonCode::InvalidReparsePoint,
                format!("{}: {detail}", entry.path()),
            ));
        }
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

    let mut hardlink_representatives = BTreeMap::<Digest, LogicalPath>::new();
    for entry in archive.entry_set.entries() {
        if let Some(group) = entry.metadata().hardlink_group() {
            hardlink_representatives
                .entry(group)
                .or_insert_with(|| entry.path().clone());
        }
    }

    // Directories and representative regular files are created before any link.
    for entry in archive.entry_set.entries() {
        let (parent, name) = resolve_parent(&root, entry.path())?;
        match entry.data() {
            EntryData::Directory => {
                ensure_absent(&parent, name, entry.path())?;
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
                if entry
                    .metadata()
                    .hardlink_group()
                    .is_some_and(|group| hardlink_representatives.get(&group) != Some(entry.path()))
                {
                    continue;
                }
                ensure_absent(&parent, name, entry.path())?;
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
                let object = archive.content_store.objects.get(digest).ok_or_else(|| {
                    Diagnostic::new(
                        OutcomeClass::Nonconforming,
                        ReasonCode::UnknownContentObject,
                        entry.path().to_string(),
                    )
                })?;
                write_content(
                    &mut file,
                    object,
                    source,
                    entry.metadata(),
                    policy.sparse(),
                    entry.path(),
                )?;
                let logical_len = object.chunks.iter().try_fold(0_u64, |total, reference| {
                    let chunk = archive
                        .content_store
                        .chunks
                        .get(&reference.chunk_id)
                        .ok_or_else(|| {
                            Diagnostic::new(
                                OutcomeClass::Nonconforming,
                                ReasonCode::UnknownChunk,
                                reference.chunk_id.to_string(),
                            )
                        })?;
                    total
                        .checked_add(chunk.logical_len)
                        .ok_or_else(|| resource("ContentObject logical length exceeds u64"))
                })?;
                report.logical_bytes_written = report
                    .logical_bytes_written
                    .checked_add(logical_len)
                    .ok_or_else(|| resource("extracted byte count exceeds u64"))?;
                apply_file_metadata(
                    file.into_std(),
                    entry.path().to_string(),
                    entry.metadata(),
                    policy,
                    &mut report,
                );
            }
            EntryData::Symlink { .. } | EntryData::ReparsePoint { .. } => continue,
        }
        report.entries_created = report
            .entries_created
            .checked_add(1)
            .ok_or_else(|| resource("extracted entry count exceeds u64"))?;
    }

    // Every non-representative hardlink is created from its canonical first path.
    for entry in archive.entry_set.entries() {
        let Some(group) = entry.metadata().hardlink_group() else {
            continue;
        };
        let representative = &hardlink_representatives[&group];
        if representative == entry.path() {
            continue;
        }
        let (parent, name) = resolve_parent(&root, entry.path())?;
        ensure_absent(&parent, name, entry.path())?;
        root.hard_link(logical_os_path(representative)?, &parent, name)
            .map_err(|error| io(format!("create hardlink {}", entry.path()), error))?;
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
                policy,
                &mut report,
            );
        }
    }

    // Symlinks are deliberately last, so no later extraction write can traverse one.
    for entry in archive.entry_set.entries() {
        let EntryData::Symlink { target } = entry.data() else {
            continue;
        };
        let (parent, name) = resolve_parent(&root, entry.path())?;
        ensure_absent(&parent, name, entry.path())?;
        create_symlink(&parent, name, target)
            .map_err(|error| io(format!("create symlink {}", entry.path()), error))?;
        restore_symlink_metadata(
            destination,
            entry,
            policy,
            &mut report.metadata_not_restored,
        );
        report.entries_created = report
            .entries_created
            .checked_add(1)
            .ok_or_else(|| resource("extracted entry count exceeds u64"))?;
    }
    Ok(report)
}

fn write_content(
    file: &mut cap_std::fs::File,
    object: &crate::eam::ContentObject,
    source: &mut dyn ChunkSource,
    metadata: &MetadataSet,
    sparse_policy: SparsePolicy,
    path: &LogicalPath,
) -> Result<()> {
    if sparse_policy == SparsePolicy::Restore
        && let Some(map) = metadata.sparse_map()
    {
        file.set_len(map.logical_size())
            .map_err(|error| io(format!("size sparse file {path}"), error))?;
        let mut logical_offset = 0_u64;
        let mut extent_index = 0_usize;
        for chunk_ref in &object.chunks {
            let plaintext = source.plaintext(&chunk_ref.chunk_id)?;
            let chunk_end = logical_offset
                .checked_add(u64::try_from(plaintext.len()).unwrap_or(u64::MAX))
                .ok_or_else(|| resource("logical sparse write offset overflow"))?;
            while let Some(extent) = map.extents().get(extent_index) {
                let extent_end = extent.offset + extent.length;
                if extent_end <= logical_offset {
                    extent_index += 1;
                    continue;
                }
                if extent.offset >= chunk_end {
                    break;
                }
                let start = extent.offset.max(logical_offset);
                let end = extent_end.min(chunk_end);
                let local_start = usize::try_from(start - logical_offset)
                    .map_err(|_| resource("sparse slice offset exceeds usize"))?;
                let local_end = usize::try_from(end - logical_offset)
                    .map_err(|_| resource("sparse slice end exceeds usize"))?;
                file.seek(SeekFrom::Start(start))
                    .map_err(|error| io(format!("seek sparse file {path}"), error))?;
                file.write_all(&plaintext[local_start..local_end])
                    .map_err(|error| io(format!("write sparse file {path}"), error))?;
                if end == extent_end {
                    extent_index += 1;
                } else {
                    break;
                }
            }
            logical_offset = chunk_end;
        }
        return Ok(());
    }
    for chunk_ref in &object.chunks {
        let plaintext = source.plaintext(&chunk_ref.chunk_id)?;
        file.write_all(&plaintext)
            .map_err(|error| io(format!("write file {path}"), error))?;
    }
    Ok(())
}

#[derive(Default)]
struct Scan {
    entries: Vec<Entry>,
    files: Vec<Box<[u8]>>,
    hardlink_members: BTreeMap<SourceFileId, Vec<HardlinkObservation>>,
    fidelity_unavailable: BTreeSet<String>,
    observed_metadata_bytes: u64,
}

type SourceFileId = (u64, u64);
type HardlinkObservation = (LogicalPath, Digest, MetadataSet);

impl Scan {
    fn account_metadata_bytes(&mut self, bytes: usize) -> Result<()> {
        self.observed_metadata_bytes = self
            .observed_metadata_bytes
            .checked_add(u64::try_from(bytes).unwrap_or(u64::MAX))
            .ok_or_else(|| resource("captured metadata byte count exceeds u64"))?;
        if self.observed_metadata_bytes > bootstrap_resource_policy().max_metadata_bytes {
            return Err(resource(
                "captured POSIX metadata exceeds the caller bootstrap metadata policy",
            ));
        }
        Ok(())
    }

    fn finish(
        self,
        profile: CompressionProfile,
        boundary: Option<(&EncryptedBoundaryKey, &'static str)>,
    ) -> Result<Archive> {
        let mut fidelity = bootstrap_fidelity();
        let mut unavailable = fidelity.unavailable.into_vec();
        for class in &self.fidelity_unavailable {
            unavailable.push(FidelityIssue {
                class: class.clone(),
                reason: "source filesystem/platform did not provide a reliable no-follow value"
                    .to_owned(),
                entry_scope: None,
            });
        }
        unavailable.sort_by(|left, right| left.class.cmp(&right.class));
        unavailable.dedup_by(|left, right| left.class == right.class);
        fidelity.unavailable = unavailable.into_boxed_slice();
        self.finish_with(profile, boundary, fidelity, None)
    }

    fn finish_with(
        mut self,
        profile: CompressionProfile,
        boundary: Option<(&EncryptedBoundaryKey, &'static str)>,
        fidelity: FidelityReport,
        conversion: Option<ConversionProvenance>,
    ) -> Result<Archive> {
        self.finalize_hardlinks()?;
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
        let entry_set = EntrySet::new(self.entries)?;
        let mut features = FeatureSet::default();
        apply_metadata_features(entry_set.entries(), &mut features);
        let mut archive = Archive {
            descriptor: ArchiveDescriptor {
                format_major: 0,
                format_minor: 1,
                format_namespace: crate::ecf::FORMAT_NAMESPACE.to_owned(),
                features,
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
            entry_set,
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
            preservation: None,
            index: Index::default(),
        };
        plan_archive_v6(&mut archive, profile)?;
        Ok(archive)
    }

    fn finalize_hardlinks(&mut self) -> Result<()> {
        for members in self.hardlink_members.values() {
            if members.len() < 2 {
                continue;
            }
            let content = members[0].1;
            if members
                .iter()
                .any(|member| member.1 != content || member.2 != members[0].2)
            {
                return Err(Diagnostic::new(
                    OutcomeClass::PolicyRefused,
                    ReasonCode::SourceUnstable,
                    "hardlink aliases changed or exposed disagreeing inode-scoped metadata",
                ));
            }
            let paths = members
                .iter()
                .map(|member| member.0.clone())
                .collect::<Vec<_>>();
            let group = hardlink_group_id(content, &paths)?;
            for path in paths {
                let entry = self
                    .entries
                    .iter_mut()
                    .find(|entry| entry.path() == &path)
                    .ok_or_else(|| {
                        Diagnostic::new(
                            OutcomeClass::Corrupt,
                            ReasonCode::InvalidHardlinkGroup,
                            "hardlink traversal evidence references a missing Entry",
                        )
                    })?;
                entry.replace_metadata(
                    entry
                        .metadata()
                        .with_item(MetadataItem::hardlink_group(group))?,
                );
            }
        }
        Ok(())
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
    preservation: Option<crate::eam::LegacyPreservation>,
    profile: CompressionProfile,
) -> Result<Archive> {
    let mut archive = Scan {
        entries,
        files,
        ..Scan::default()
    }
    .finish_with(profile, None, fidelity, Some(conversion))?;
    archive.preservation = preservation;
    archive.descriptor.features.incompat |= FEATURE_CONVERSION_PROVENANCE_V1;
    if archive.preservation.is_some() {
        archive.descriptor.features.incompat |= crate::ecf::FEATURE_LEGACY_PRESERVATION_V1;
    }
    let (archive, _) = crate::identity::apply_native_identities(&archive)?;
    Ok(archive)
}

fn apply_metadata_features(entries: &[Entry], features: &mut FeatureSet) {
    if entries.iter().any(Entry::uses_platform_security_v1) {
        features.incompat |= FEATURE_POSIX_METADATA_V1 | FEATURE_PLATFORM_SECURITY_METADATA_V1;
    } else if entries.iter().any(Entry::uses_posix_v1) {
        features.incompat |= FEATURE_POSIX_METADATA_V1;
    }
}

fn scan_directory(
    directory: &Dir,
    ambient_root: &Path,
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
        #[cfg(windows)]
        {
            let no_follow = directory
                .symlink_metadata(source_entry.file_name())
                .map_err(|error| io(format!("inspect source reparse state {path}"), error))?;
            if no_follow.file_attributes() & 0x0000_0400 != 0 {
                return Err(Diagnostic::new(
                    OutcomeClass::Unsupported,
                    ReasonCode::InvalidReparsePoint,
                    format!(
                        "source entry {path} is a Windows reparse object; exact tag/payload capture requires an audited safe platform API"
                    ),
                ));
            }
        }
        if file_type.is_symlink() {
            let metadata = directory
                .symlink_metadata(source_entry.file_name())
                .map_err(|error| io(format!("inspect source symlink {path}"), error))?;
            let target_path = directory
                .read_link_contents(source_entry.file_name())
                .map_err(|error| io(format!("read source symlink {path}"), error))?;
            let target = link_target_from_os(target_path.as_os_str())?;
            scan.account_metadata_bytes(target.bytes().len())?;
            let source_path = ambient_path(ambient_root, &components);
            let xattrs = captured_xattrs(&source_path, scan)?;
            let platform = captured_macos_metadata(&source_path, scan)?;
            let after = directory
                .symlink_metadata(source_entry.file_name())
                .map_err(|error| io(format!("reinspect source symlink {path}"), error))?;
            if !capture_metadata_stable(&metadata, &after) {
                return Err(source_unstable(path.to_string()));
            }
            scan.entries.push(Entry::new(
                path,
                EntryData::Symlink { target },
                metadata_set(&metadata, xattrs, None, None, platform)?,
                EntryIdentity::default(),
            ));
        } else if file_type.is_dir() {
            let child = source_entry
                .open_dir()
                .map_err(|error| io(format!("open source directory {path}"), error))?;
            let metadata = child
                .dir_metadata()
                .map_err(|error| io(format!("inspect source directory {path}"), error))?;
            let source_path = ambient_path(ambient_root, &components);
            let acls = captured_acls(&source_path, true, scan)?;
            let xattrs = captured_xattrs(&source_path, scan)?;
            let platform = captured_macos_metadata(&source_path, scan)?;
            let after = child
                .dir_metadata()
                .map_err(|error| io(format!("reinspect source directory {path}"), error))?;
            if !capture_metadata_stable(&metadata, &after) {
                return Err(source_unstable(path.to_string()));
            }
            scan.entries.push(Entry::new(
                path.clone(),
                EntryData::Directory,
                metadata_set(&metadata, xattrs, None, acls, platform)?,
                EntryIdentity::default(),
            ));
            scan_directory(&child, ambient_root, &components, retries, scan)?;
            let final_metadata = child
                .dir_metadata()
                .map_err(|error| io(format!("reinspect source directory {path}"), error))?;
            if !capture_metadata_stable(&metadata, &final_metadata) {
                return Err(source_unstable(path.to_string()));
            }
        } else if file_type.is_file() {
            let (plaintext, metadata) =
                capture_entry_with_probe(&source_entry, retries, |_| Ok(false))?;
            let source_path = ambient_path(ambient_root, &components);
            let acls = captured_acls(&source_path, false, scan)?;
            let xattrs = captured_xattrs(&source_path, scan)?;
            let sparse = captured_sparse(&source_entry, metadata.len(), scan)?;
            let platform = captured_macos_metadata(&source_path, scan)?;
            let after = source_entry
                .open()
                .and_then(|file| file.metadata())
                .map_err(|error| io(format!("reinspect source file {path}"), error))?;
            if !capture_metadata_stable(&metadata, &after) {
                return Err(source_unstable(path.to_string()));
            }
            let digest = sha256_exact(&plaintext);
            let metadata_set = metadata_set(&metadata, xattrs, sparse, acls, platform)?;
            scan.entries.push(Entry::new(
                path.clone(),
                EntryData::File {
                    content: ContentRef::Internal(digest),
                },
                metadata_set.clone(),
                EntryIdentity::default(),
            ));
            scan.files.push(plaintext.into_boxed_slice());
            if let Some(identity) = hardlink_identity(&metadata) {
                scan.hardlink_members.entry(identity).or_default().push((
                    path,
                    digest,
                    metadata_set,
                ));
            }
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
        let changed = !capture_metadata_stable(&before, &after)
            || after.len() != u64::try_from(bytes.len()).unwrap_or(u64::MAX)
            || additional_change_probe(attempt)?;
        if !changed {
            return Ok((bytes, after));
        }
    }
    Err(source_unstable(entry.file_name().to_string_lossy()))
}

#[cfg(unix)]
fn capture_metadata_stable(before: &Metadata, after: &Metadata) -> bool {
    before.dev() == after.dev()
        && before.ino() == after.ino()
        && before.mode() == after.mode()
        && before.nlink() == after.nlink()
        && before.uid() == after.uid()
        && before.gid() == after.gid()
        && before.size() == after.size()
        && before.mtime() == after.mtime()
        && before.mtime_nsec() == after.mtime_nsec()
        && before.ctime() == after.ctime()
        && before.ctime_nsec() == after.ctime_nsec()
}

#[cfg(not(unix))]
fn capture_metadata_stable(before: &Metadata, after: &Metadata) -> bool {
    before.len() == after.len()
        && before.modified().ok() == after.modified().ok()
        && executable(before) == executable(after)
}

fn metadata_set(
    metadata: &Metadata,
    xattrs: Option<Vec<XAttr>>,
    sparse: Option<SparseMap>,
    acls: Option<Vec<Acl>>,
    platform: Vec<MetadataItem>,
) -> Result<MetadataSet> {
    let modified = metadata
        .modified()
        .map_err(|error| io("read source modification time", error))?
        .into_std();
    let mut items = vec![
        MetadataItem::executable(executable(metadata)),
        MetadataItem::mtime(timestamp(modified)?),
    ];
    #[cfg(unix)]
    {
        items.extend([
            MetadataItem::posix_mode(metadata.permissions().mode() & 0o7777),
            MetadataItem::posix_uid(metadata.uid()),
            MetadataItem::posix_gid(metadata.gid()),
        ]);
    }
    if let Some(xattrs) = xattrs {
        items.push(MetadataItem::xattrs(xattrs)?);
    }
    if let Some(sparse) = sparse {
        items.push(MetadataItem::sparse_map(sparse));
    }
    if let Some(acls) = acls
        && !acls.is_empty()
    {
        items.push(MetadataItem::acls(acls)?);
    }
    items.extend(platform);
    #[cfg(windows)]
    capture_windows_portable_metadata(metadata, &mut items)?;
    MetadataSet::new(items)
}

#[cfg(windows)]
fn capture_windows_portable_metadata(
    metadata: &Metadata,
    items: &mut Vec<MetadataItem>,
) -> Result<()> {
    const SEMANTIC_AUTHORITY_BITS: u32 = 0x0000_0010 | 0x0000_0200 | 0x0000_0400;
    items.push(MetadataItem::windows_file_attributes(
        metadata.file_attributes() & !SEMANTIC_AUTHORITY_BITS,
    )?);
    items.push(MetadataItem::windows_creation_time(windows_filetime(
        metadata.creation_time(),
    )?));
    Ok(())
}

#[cfg(windows)]
fn windows_filetime(value: u64) -> Result<Timestamp> {
    const TICKS_PER_SECOND: u64 = 10_000_000;
    const WINDOWS_TO_UNIX_SECONDS: i128 = 11_644_473_600;
    let seconds = i128::from(value / TICKS_PER_SECOND) - WINDOWS_TO_UNIX_SECONDS;
    let seconds = i64::try_from(seconds).map_err(|_| {
        Diagnostic::new(
            OutcomeClass::Nonconforming,
            ReasonCode::InvalidWindowsMetadata,
            "Windows creation time is outside Entrybound's signed-seconds range",
        )
    })?;
    let nanoseconds = u32::try_from(value % TICKS_PER_SECOND)
        .unwrap_or_default()
        .saturating_mul(100);
    Timestamp::new(
        seconds,
        nanoseconds,
        TimestampPrecision::Hectonanosecond,
        true,
    )
}

#[cfg(target_os = "macos")]
fn captured_macos_metadata(path: &Path, scan: &mut Scan) -> Result<Vec<MetadataItem>> {
    use std::os::macos::fs::MetadataExt as _;

    let metadata = std::fs::symlink_metadata(path).map_err(|error| {
        io(
            format!("capture macOS metadata for {}", path.display()),
            error,
        )
    })?;
    let nanoseconds = u32::try_from(metadata.st_birthtime_nsec()).map_err(|_| {
        Diagnostic::new(
            OutcomeClass::Nonconforming,
            ReasonCode::InvalidMacosMetadata,
            "macOS birthtime nanoseconds are outside the canonical range",
        )
    })?;
    scan.account_metadata_bytes(16)?;
    Ok(vec![
        MetadataItem::macos_flags(metadata.st_flags())?,
        MetadataItem::macos_birthtime(Timestamp::new(
            metadata.st_birthtime(),
            nanoseconds,
            TimestampPrecision::Nanosecond,
            false,
        )?),
    ])
}

#[cfg(not(target_os = "macos"))]
fn captured_macos_metadata(_path: &Path, _scan: &mut Scan) -> Result<Vec<MetadataItem>> {
    Ok(Vec::new())
}

fn ambient_path(root: &Path, components: &[String]) -> PathBuf {
    components
        .iter()
        .fold(root.to_path_buf(), |path, component| path.join(component))
}

#[cfg(unix)]
fn link_target_from_os(value: &OsStr) -> Result<LinkTarget> {
    use std::os::unix::ffi::OsStrExt as _;
    LinkTarget::canonical(value.as_bytes().to_vec().into_boxed_slice())
}

#[cfg(not(unix))]
fn link_target_from_os(value: &OsStr) -> Result<LinkTarget> {
    let value = value.to_str().ok_or_else(|| {
        Diagnostic::new(
            OutcomeClass::Unsupported,
            ReasonCode::InvalidSymlinkTarget,
            "this platform cannot expose the symlink target as exact POSIX bytes",
        )
    })?;
    LinkTarget::canonical(value.as_bytes().to_vec().into_boxed_slice())
}

#[cfg(target_os = "linux")]
fn captured_acls(path: &Path, directory: bool, scan: &mut Scan) -> Result<Option<Vec<Acl>>> {
    let mut acls = Vec::new();
    for (name, scope) in [
        ("system.posix_acl_access", AclScope::Access),
        ("system.posix_acl_default", AclScope::Default),
    ] {
        if scope == AclScope::Default && !directory {
            continue;
        }
        let value = match xattr::get(path, name) {
            Ok(value) => value,
            Err(error) if matches!(error.raw_os_error(), Some(1 | 13 | 45 | 61 | 95)) => {
                scan.fidelity_unavailable.insert("security.acls".to_owned());
                return Ok(None);
            }
            Err(error) => return Err(io(format!("read ACL for {}", path.display()), error)),
        };
        let Some(value) = value else {
            continue;
        };
        scan.account_metadata_bytes(value.len())?;
        acls.push(decode_linux_posix_acl(&value, scope)?);
    }
    Ok(Some(acls))
}

#[cfg(not(target_os = "linux"))]
fn captured_acls(_path: &Path, _directory: bool, scan: &mut Scan) -> Result<Option<Vec<Acl>>> {
    scan.fidelity_unavailable.insert("security.acls".to_owned());
    Ok(None)
}

#[cfg(target_os = "linux")]
fn decode_linux_posix_acl(bytes: &[u8], scope: AclScope) -> Result<Acl> {
    if bytes.len() < 4
        || (bytes.len() - 4) % 8 != 0
        || u32::from_le_bytes(bytes[0..4].try_into().unwrap_or_default()) != 2
    {
        return Err(Diagnostic::new(
            OutcomeClass::Nonconforming,
            ReasonCode::InvalidAcl,
            "Linux POSIX ACL xattr framing is malformed",
        ));
    }
    let mut entries = Vec::with_capacity((bytes.len() - 4) / 8);
    for encoded in bytes[4..].chunks_exact(8) {
        let tag = u16::from_le_bytes(encoded[0..2].try_into().unwrap_or_default());
        let permissions = linux_acl_permissions_to_canonical(u16::from_le_bytes(
            encoded[2..4].try_into().unwrap_or_default(),
        ))?;
        let id = u32::from_le_bytes(encoded[4..8].try_into().unwrap_or_default());
        let principal = match (tag, id) {
            (0x01, u32::MAX) => AclPrincipal::UserObj,
            (0x02, value) if value != u32::MAX => AclPrincipal::User(value),
            (0x04, u32::MAX) => AclPrincipal::GroupObj,
            (0x08, value) if value != u32::MAX => AclPrincipal::Group(value),
            (0x10, u32::MAX) => AclPrincipal::Mask,
            (0x20, u32::MAX) => AclPrincipal::Other,
            _ => {
                return Err(Diagnostic::new(
                    OutcomeClass::Nonconforming,
                    ReasonCode::InvalidAcl,
                    "Linux POSIX ACL xattr contains an invalid tag/qualifier",
                ));
            }
        };
        entries.push(AclEntry::new(
            AclEntryType::Allow,
            principal,
            permissions,
            0,
        )?);
    }
    Acl::new(AclDialect::Posix1e, scope, entries)
}

#[cfg(target_os = "linux")]
fn encode_linux_posix_acl(acl: &Acl) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(4 + acl.entries().len() * 8);
    bytes.extend_from_slice(&2_u32.to_le_bytes());
    for entry in acl.entries() {
        let (tag, id) = match entry.principal() {
            AclPrincipal::UserObj => (0x01_u16, u32::MAX),
            AclPrincipal::User(value) => (0x02, *value),
            AclPrincipal::GroupObj => (0x04, u32::MAX),
            AclPrincipal::Group(value) => (0x08, *value),
            AclPrincipal::Mask => (0x10, u32::MAX),
            AclPrincipal::Other => (0x20, u32::MAX),
            _ => unreachable!("POSIX ACL validation screened the principal"),
        };
        bytes.extend_from_slice(&tag.to_le_bytes());
        bytes.extend_from_slice(
            &canonical_permissions_to_linux_acl(entry.permissions()).to_le_bytes(),
        );
        bytes.extend_from_slice(&id.to_le_bytes());
    }
    bytes
}

#[cfg(target_os = "linux")]
fn linux_acl_permissions_to_canonical(value: u16) -> Result<u32> {
    if value & !0x7 != 0 {
        return Err(Diagnostic::new(
            OutcomeClass::Nonconforming,
            ReasonCode::InvalidAcl,
            "Linux POSIX ACL xattr contains unknown permission bits",
        ));
    }
    Ok(u32::from(value & 0x2) | u32::from(value & 0x4) >> 2 | u32::from(value & 0x1) << 2)
}

#[cfg(target_os = "linux")]
fn canonical_permissions_to_linux_acl(value: u32) -> u16 {
    u16::try_from((value & 0x2) | (value & 0x1) << 2 | (value & 0x4) >> 2).unwrap_or_default()
}

#[cfg(unix)]
fn captured_xattrs(path: &Path, scan: &mut Scan) -> Result<Option<Vec<XAttr>>> {
    use std::os::unix::ffi::OsStrExt as _;

    let names = match xattr::list(path) {
        Ok(names) => names,
        Err(error) if matches!(error.raw_os_error(), Some(1 | 13 | 45 | 61 | 95)) => {
            scan.fidelity_unavailable.insert("posix.xattrs".to_owned());
            return Ok(None);
        }
        Err(error) => {
            return Err(io(
                format!("enumerate xattrs for {}", path.display()),
                error,
            ));
        }
    };
    let mut values = Vec::new();
    for name in names {
        if matches!(
            name.as_os_str().as_bytes(),
            b"system.posix_acl_access" | b"system.posix_acl_default"
        ) {
            continue;
        }
        if values.len() == 4096 {
            return Err(resource("xattr count exceeds the per-entry format bound"));
        }
        let value = xattr::get(path, &name)
            .map_err(|error| io(format!("read xattr for {}", path.display()), error))?
            .ok_or_else(|| {
                Diagnostic::new(
                    OutcomeClass::PolicyRefused,
                    ReasonCode::SourceUnstable,
                    format!("an enumerated xattr disappeared from {}", path.display()),
                )
            })?;
        let item = XAttr::new(
            name.as_os_str().as_bytes().to_vec().into_boxed_slice(),
            value.into_boxed_slice(),
        )?;
        scan.account_metadata_bytes(item.name().len().saturating_add(item.value().len()))?;
        values.push(item);
    }
    Ok(Some(values))
}

#[cfg(not(unix))]
fn captured_xattrs(_path: &Path, scan: &mut Scan) -> Result<Option<Vec<XAttr>>> {
    scan.fidelity_unavailable.insert("posix.xattrs".to_owned());
    Ok(None)
}

#[cfg(target_os = "linux")]
fn captured_sparse(
    entry: &DirEntry,
    logical_size: u64,
    scan: &mut Scan,
) -> Result<Option<SparseMap>> {
    use rustix::fs::SeekFrom;
    use rustix::io::Errno;

    let file = entry
        .open()
        .map_err(|error| io("open source file for sparse discovery", error))?;
    let mut extents = Vec::new();
    let mut cursor = 0_u64;
    while cursor < logical_size {
        let data = match rustix::fs::seek(&file, SeekFrom::Data(cursor)) {
            Ok(value) => value,
            Err(Errno::NXIO) => break,
            Err(Errno::INVAL | Errno::NOTSUP) => {
                scan.fidelity_unavailable
                    .insert("posix.sparse-map".to_owned());
                return Ok(None);
            }
            Err(error) => {
                return Err(io(
                    "discover sparse data extent",
                    std::io::Error::from(error),
                ));
            }
        };
        if data >= logical_size {
            break;
        }
        let hole = rustix::fs::seek(&file, SeekFrom::Hole(data))
            .map_err(|error| io("discover sparse hole extent", std::io::Error::from(error)))?
            .min(logical_size);
        if hole <= data {
            return Err(Diagnostic::new(
                OutcomeClass::PolicyRefused,
                ReasonCode::SourceUnstable,
                "filesystem returned a non-progressing sparse extent",
            ));
        }
        extents.push(SparseExtent {
            offset: data,
            length: hole - data,
        });
        cursor = hole;
    }
    scan.account_metadata_bytes(extents.len().saturating_mul(16).saturating_add(8))?;
    Ok(Some(SparseMap::new(logical_size, extents)?))
}

#[cfg(not(target_os = "linux"))]
fn captured_sparse(
    _entry: &DirEntry,
    _logical_size: u64,
    scan: &mut Scan,
) -> Result<Option<SparseMap>> {
    scan.fidelity_unavailable
        .insert("posix.sparse-map".to_owned());
    Ok(None)
}

#[cfg(unix)]
fn hardlink_identity(metadata: &Metadata) -> Option<(u64, u64)> {
    (metadata.nlink() > 1).then(|| (metadata.dev(), metadata.ino()))
}

#[cfg(not(unix))]
fn hardlink_identity(_metadata: &Metadata) -> Option<(u64, u64)> {
    None
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
    captured.extend([
        "core.executable".to_owned(),
        "posix.mode".to_owned(),
        "posix.uid".to_owned(),
        "posix.gid".to_owned(),
        "posix.hardlink-group".to_owned(),
        "posix.xattrs".to_owned(),
    ]);
    #[cfg(target_os = "linux")]
    captured.extend(["posix.sparse-map".to_owned(), "security.acls".to_owned()]);
    #[cfg(windows)]
    captured.extend([
        "windows.file-attributes".to_owned(),
        "windows.creation-time".to_owned(),
    ]);
    #[cfg(target_os = "macos")]
    captured.extend(["macos.flags".to_owned(), "macos.birthtime".to_owned()]);
    captured.push("symlink-target".to_owned());
    captured.sort();
    let mut unavailable = ["special-files"]
        .into_iter()
        .map(|class| FidelityIssue {
            class: class.to_owned(),
            reason: "not captured by the native bootstrap filesystem slice".to_owned(),
            entry_scope: None,
        })
        .collect::<Vec<_>>();
    #[cfg(not(unix))]
    unavailable.extend([
        FidelityIssue {
            class: "core.executable".to_owned(),
            reason: "the source platform does not represent a portable executable bit".to_owned(),
            entry_scope: None,
        },
        FidelityIssue {
            class: "posix ownership/mode/hardlinks".to_owned(),
            reason: "the source platform does not expose POSIX inode metadata".to_owned(),
            entry_scope: None,
        },
    ]);
    #[cfg(not(target_os = "linux"))]
    unavailable.push(FidelityIssue {
        class: "posix.sparse-map".to_owned(),
        reason: "reliable SEEK_DATA/SEEK_HOLE capture is unavailable on this platform".to_owned(),
        entry_scope: None,
    });
    #[cfg(not(target_os = "linux"))]
    unavailable.push(FidelityIssue {
        class: "security.acls".to_owned(),
        reason: "this build has no audited safe exact ACL capture adapter for the source platform"
            .to_owned(),
        entry_scope: None,
    });
    #[cfg(windows)]
    unavailable.extend([
        FidelityIssue {
            class: "windows.security-descriptor".to_owned(),
            reason: "no audited safe wrapper exposes the exact self-relative descriptor bytes"
                .to_owned(),
            entry_scope: None,
        },
        FidelityIssue {
            class: "windows.reparse-original".to_owned(),
            reason: "no audited safe wrapper exposes exact opaque reparse payload bytes".to_owned(),
            entry_scope: None,
        },
    ]);
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

fn logical_os_path(path: &LogicalPath) -> Result<PathBuf> {
    let mut result = PathBuf::new();
    for component in path.components() {
        result.push(component_os(component.bytes())?);
    }
    Ok(result)
}

fn validate_symlink_policy(
    path: &LogicalPath,
    target: &LinkTarget,
    policy: SymlinkPolicy,
) -> Result<()> {
    match policy {
        SymlinkPolicy::Refuse => Err(Diagnostic::new(
            OutcomeClass::PolicyRefused,
            ReasonCode::UnsupportedEntryKind,
            format!("symlink extraction is disabled: {path}"),
        )),
        SymlinkPolicy::All => Ok(()),
        SymlinkPolicy::Safe => {
            let bytes = target.bytes();
            if bytes.starts_with(b"/")
                || cfg!(windows)
                    && (bytes.starts_with(b"\\")
                        || bytes.get(1) == Some(&b':')
                        || bytes.contains(&b'\\'))
            {
                return Err(Diagnostic::new(
                    OutcomeClass::PolicyRefused,
                    ReasonCode::ExtractionUnsafeSymlink,
                    format!("symlink {path} has an absolute or platform-rooted target"),
                ));
            }
            let mut depth = path.components().len().saturating_sub(1);
            for component in bytes.split(|byte| *byte == b'/') {
                match component {
                    b"" | b"." => {}
                    b".." if depth == 0 => {
                        return Err(Diagnostic::new(
                            OutcomeClass::PolicyRefused,
                            ReasonCode::ExtractionUnsafeSymlink,
                            format!("symlink {path} lexically escapes the extraction root"),
                        ));
                    }
                    b".." => depth -= 1,
                    _ => depth += 1,
                }
            }
            Ok(())
        }
    }
}

#[cfg(unix)]
fn create_symlink(parent: &Dir, name: &OsStr, target: &LinkTarget) -> std::io::Result<()> {
    use std::os::unix::ffi::OsStrExt as _;
    parent.symlink_contents(OsStr::from_bytes(target.bytes()), name)
}

#[cfg(windows)]
fn create_symlink(parent: &Dir, name: &OsStr, target: &LinkTarget) -> std::io::Result<()> {
    let target = std::str::from_utf8(target.bytes())
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "non-UTF8 target"))?;
    parent.symlink_file(target, name)
}

#[cfg(not(any(unix, windows)))]
fn create_symlink(_parent: &Dir, _name: &OsStr, _target: &LinkTarget) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "symbolic links are unsupported on this platform",
    ))
}

fn apply_file_metadata(
    file: std::fs::File,
    path: String,
    metadata: &MetadataSet,
    policy: ExtractionPolicy,
    report: &mut ExtractionReport,
) {
    if metadata.windows_security_descriptor().is_some() {
        let reason = if policy.windows_security() == WindowsSecurityPolicy::Restore {
            "exact Windows security restoration has no audited safe API in this build"
        } else {
            "windows.security-descriptor skipped by policy"
        };
        report
            .metadata_not_restored
            .push(format!("{path}: {reason}"));
    }
    if metadata.windows_file_attributes().is_some()
        || metadata.windows_creation_time().is_some()
        || metadata.windows_reparse_original().is_some()
        || metadata.macos_flags().is_some()
        || metadata.macos_birthtime().is_some()
    {
        let reason = if policy.platform_metadata() == PlatformMetadataPolicy::Restore {
            "complete platform metadata restoration is unavailable for one or more recorded fields"
        } else {
            "platform metadata skipped by policy"
        };
        report
            .metadata_not_restored
            .push(format!("{path}: {reason}"));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let stored_uid = metadata.posix_uid();
        let stored_gid = metadata.posix_gid();
        if policy.ownership() == OwnershipPolicy::Restore
            && (stored_uid.is_some() || stored_gid.is_some())
        {
            let owner = stored_uid
                .filter(|uid| *uid != u32::MAX)
                .map(rustix::process::Uid::from_raw);
            let group = stored_gid
                .filter(|gid| *gid != u32::MAX)
                .map(rustix::process::Gid::from_raw);
            if stored_uid == Some(u32::MAX) || stored_gid == Some(u32::MAX) {
                report.metadata_not_restored.push(format!(
                    "{path}: POSIX ownership value is the platform no-change sentinel"
                ));
            } else if let Err(error) = rustix::fs::fchown(&file, owner, group) {
                report
                    .metadata_not_restored
                    .push(format!("{path}: posix.uid/gid ({error})"));
            }
        } else if stored_uid.is_some() || stored_gid.is_some() {
            report
                .metadata_not_restored
                .push(format!("{path}: posix.uid/gid skipped by policy"));
        }
        if policy.xattrs() == XAttrPolicy::Restore {
            use std::os::unix::ffi::OsStrExt as _;
            use xattr::FileExt as _;
            for attribute in metadata.xattrs() {
                if let Err(error) =
                    file.set_xattr(OsStr::from_bytes(attribute.name()), attribute.value())
                {
                    report.metadata_not_restored.push(format!(
                        "{path}: xattr {} ({error})",
                        String::from_utf8_lossy(attribute.name())
                    ));
                }
            }
        } else if !metadata.xattrs().is_empty() {
            report
                .metadata_not_restored
                .push(format!("{path}: posix.xattrs skipped by policy"));
        }
        if !metadata.acls().is_empty() {
            if policy.acls() == AclPolicy::Restore {
                #[cfg(target_os = "linux")]
                restore_linux_acls(&file, &path, metadata.acls(), report);
                #[cfg(not(target_os = "linux"))]
                report.metadata_not_restored.push(format!(
                    "{path}: ACL restoration is unavailable for this platform/dialect"
                ));
            } else {
                report
                    .metadata_not_restored
                    .push(format!("{path}: security.acls skipped by policy"));
            }
        }
        match file.metadata() {
            Ok(file_metadata) => {
                let mut permissions = file_metadata.permissions();
                let mode = metadata.posix_mode().unwrap_or_else(|| {
                    let mut mode = permissions.mode();
                    if metadata.executable() {
                        mode |= 0o100;
                    } else {
                        mode &= !0o111;
                    }
                    mode
                });
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
    #[cfg(not(unix))]
    {
        if metadata.posix_mode().is_some() {
            report.metadata_not_restored.push(format!(
                "{path}: posix.mode is not restorable on this platform"
            ));
        }
        if metadata.posix_uid().is_some() || metadata.posix_gid().is_some() {
            report.metadata_not_restored.push(format!(
                "{path}: posix.uid/gid are not restorable on this platform"
            ));
        }
        if !metadata.xattrs().is_empty() {
            report.metadata_not_restored.push(format!(
                "{path}: posix.xattrs are not restorable on this platform"
            ));
        }
        if !metadata.acls().is_empty() {
            report.metadata_not_restored.push(format!(
                "{path}: security.acls are not restorable on this platform"
            ));
        }
    }
    if metadata.sparse_map().is_some() {
        match policy.sparse() {
            SparsePolicy::Logical => report.metadata_not_restored.push(format!(
                "{path}: posix.sparse-map materialized as logical bytes by policy"
            )),
            #[cfg(not(unix))]
            SparsePolicy::Restore => report.metadata_not_restored.push(format!(
                "{path}: sparse topology restoration is unavailable on this platform"
            )),
            #[cfg(unix)]
            SparsePolicy::Restore => {}
        }
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

#[cfg(target_os = "linux")]
fn restore_linux_acls(
    file: &std::fs::File,
    path: &str,
    acls: &[Acl],
    report: &mut ExtractionReport,
) {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt as _;
    use xattr::FileExt as _;

    for acl in acls {
        if acl.dialect() != AclDialect::Posix1e {
            report.metadata_not_restored.push(format!(
                "{path}: NFS4 ACL is incompatible with Linux POSIX1E restoration"
            ));
            continue;
        }
        let name = match acl.scope() {
            AclScope::Access => b"system.posix_acl_access".as_slice(),
            AclScope::Default => b"system.posix_acl_default".as_slice(),
        };
        let value = encode_linux_posix_acl(acl);
        if let Err(error) = file.set_xattr(OsStr::from_bytes(name), &value) {
            report.metadata_not_restored.push(format!(
                "{path}: {} ({error})",
                String::from_utf8_lossy(name)
            ));
        }
    }
}

fn restore_symlink_metadata(
    destination: &Path,
    entry: &Entry,
    policy: ExtractionPolicy,
    report: &mut Vec<String>,
) {
    #[cfg(not(unix))]
    let _ = (destination, policy);
    if entry.metadata().posix_uid().is_some() || entry.metadata().posix_gid().is_some() {
        report.push(format!(
            "{}: symlink ownership is capture-only in this implementation",
            entry.path()
        ));
    }
    if !entry.metadata().xattrs().is_empty() {
        #[cfg(unix)]
        if policy.xattrs() == XAttrPolicy::Restore {
            use std::os::unix::ffi::OsStrExt as _;
            let path = destination.join(logical_os_path(entry.path()).unwrap_or_default());
            for attribute in entry.metadata().xattrs() {
                if let Err(error) = xattr::set(
                    &path,
                    OsStr::from_bytes(attribute.name()),
                    attribute.value(),
                ) {
                    report.push(format!(
                        "{}: symlink xattr {} ({error})",
                        entry.path(),
                        String::from_utf8_lossy(attribute.name())
                    ));
                }
            }
        } else {
            report.push(format!("{}: posix.xattrs skipped by policy", entry.path()));
        }
    }
    if entry.metadata().mtime().is_some() {
        report.push(format!(
            "{}: no-follow symlink mtime restoration is unavailable",
            entry.path()
        ));
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

fn source_unstable(path: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::PolicyRefused,
        ReasonCode::SourceUnstable,
        format!("source '{}' changed during capture", path.into()),
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

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_acl_xattr_permissions_round_trip_the_canonical_registry() {
        let acl = Acl::new(
            AclDialect::Posix1e,
            AclScope::Access,
            vec![
                AclEntry::new(AclEntryType::Allow, AclPrincipal::UserObj, 0x7, 0).unwrap(),
                AclEntry::new(AclEntryType::Allow, AclPrincipal::GroupObj, 0x5, 0).unwrap(),
                AclEntry::new(AclEntryType::Allow, AclPrincipal::Other, 0x1, 0).unwrap(),
            ],
        )
        .unwrap();
        let encoded = encode_linux_posix_acl(&acl);
        assert_eq!(
            decode_linux_posix_acl(&encoded, AclScope::Access).unwrap(),
            acl
        );
    }

    #[cfg(windows)]
    #[test]
    fn windows_capture_records_bounded_attributes_and_creation_time() {
        let root_path = std::env::temp_dir().join(format!(
            "entrybound-windows-metadata-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&root_path).unwrap();
        std::fs::write(root_path.join("file"), b"platform metadata").unwrap();

        let encoded = pack_directory(&root_path, PackOptions::default()).unwrap();
        let opened = crate::ecf::open(&encoded.bytes).unwrap();
        let entry = opened
            .archive
            .entry_set
            .entries()
            .iter()
            .find(|entry| entry.path().to_string() == "file")
            .unwrap();
        assert!(entry.metadata().windows_file_attributes().is_some());
        assert!(entry.metadata().windows_creation_time().is_some());
        assert_ne!(
            opened.archive.descriptor.features.incompat & FEATURE_PLATFORM_SECURITY_METADATA_V1,
            0
        );
        assert_ne!(
            opened.archive.descriptor.features.incompat & FEATURE_POSIX_METADATA_V1,
            0
        );

        std::fs::remove_dir_all(root_path).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn unix_posix_capture_and_policy_restoration_round_trip() {
        use std::os::unix::ffi::OsStrExt as _;
        use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _, symlink};

        let nonce = NEXT.fetch_add(1, Ordering::Relaxed);
        let source = std::env::temp_dir().join(format!(
            "entrybound-posix-source-{}-{nonce}",
            std::process::id()
        ));
        let destination = std::env::temp_dir().join(format!(
            "entrybound-posix-output-{}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir(&source).unwrap();
        let file_path = source.join("file");
        std::fs::write(&file_path, b"hardlinked payload").unwrap();
        std::fs::set_permissions(&file_path, std::fs::Permissions::from_mode(0o6750)).unwrap();
        std::fs::hard_link(&file_path, source.join("alias")).unwrap();
        symlink("file", source.join("link")).unwrap();
        symlink(OsStr::from_bytes(&[0xff]), source.join("byte-link")).unwrap();

        let sparse_path = source.join("sparse");
        let mut sparse = std::fs::File::create(&sparse_path).unwrap();
        sparse.seek(SeekFrom::Start(128 * 1024)).unwrap();
        sparse.write_all(b"data").unwrap();
        sparse.set_len(256 * 1024).unwrap();
        drop(sparse);

        let xattr_written =
            xattr::set(&file_path, "user.entrybound-test", b"opaque\0value").is_ok();
        let encoded = pack_directory(&source, PackOptions::default()).unwrap();
        let opened = crate::ecf::open(&encoded.bytes).unwrap();
        assert_ne!(
            opened.archive.descriptor.features.incompat & FEATURE_POSIX_METADATA_V1,
            0
        );

        let file_entry = opened
            .archive
            .entry_set
            .entries()
            .iter()
            .find(|entry| entry.path().to_string() == "file")
            .unwrap();
        let alias_entry = opened
            .archive
            .entry_set
            .entries()
            .iter()
            .find(|entry| entry.path().to_string() == "alias")
            .unwrap();
        assert_eq!(
            file_entry.metadata().hardlink_group(),
            alias_entry.metadata().hardlink_group()
        );
        assert!(file_entry.metadata().hardlink_group().is_some());
        assert_eq!(file_entry.metadata().posix_mode(), Some(0o6750));
        if xattr_written {
            assert_eq!(
                file_entry
                    .metadata()
                    .xattrs()
                    .iter()
                    .find(|item| item.name() == b"user.entrybound-test")
                    .map(XAttr::value),
                Some(b"opaque\0value".as_slice())
            );
        }

        unpack(
            &encoded.bytes,
            &destination,
            ExtractionPolicy::default()
                .with_ownership(OwnershipPolicy::Restore)
                .with_xattrs(XAttrPolicy::Restore)
                .with_sparse(SparsePolicy::Restore),
        )
        .unwrap();
        assert_eq!(
            std::fs::read(destination.join("file")).unwrap(),
            b"hardlinked payload"
        );
        assert_eq!(
            std::fs::symlink_metadata(destination.join("file"))
                .unwrap()
                .ino(),
            std::fs::symlink_metadata(destination.join("alias"))
                .unwrap()
                .ino()
        );
        assert_eq!(
            std::fs::read_link(destination.join("link")).unwrap(),
            Path::new("file")
        );
        assert_eq!(
            std::fs::read_link(destination.join("byte-link"))
                .unwrap()
                .as_os_str()
                .as_bytes(),
            &[0xff]
        );
        assert_eq!(
            std::fs::metadata(destination.join("file"))
                .unwrap()
                .permissions()
                .mode()
                & 0o7777,
            0o6750
        );
        let source_owner = std::fs::metadata(&file_path).unwrap();
        let restored_owner = std::fs::metadata(destination.join("file")).unwrap();
        assert_eq!(restored_owner.uid(), source_owner.uid());
        assert_eq!(restored_owner.gid(), source_owner.gid());
        assert_eq!(
            std::fs::read(destination.join("sparse")).unwrap().len(),
            256 * 1024
        );
        if xattr_written {
            assert_eq!(
                xattr::get(destination.join("file"), "user.entrybound-test").unwrap(),
                Some(b"opaque\0value".to_vec())
            );
        }

        std::fs::remove_dir_all(destination).unwrap();
        std::fs::remove_dir_all(source).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn unsafe_symlink_is_refused_before_destination_creation() {
        use std::os::unix::fs::symlink;

        let nonce = NEXT.fetch_add(1, Ordering::Relaxed);
        let source = std::env::temp_dir().join(format!(
            "entrybound-unsafe-link-source-{}-{nonce}",
            std::process::id()
        ));
        let destination = std::env::temp_dir().join(format!(
            "entrybound-unsafe-link-output-{}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir(&source).unwrap();
        symlink("/outside", source.join("link")).unwrap();
        let encoded = pack_directory(&source, PackOptions::default()).unwrap();
        let error = unpack(&encoded.bytes, &destination, ExtractionPolicy::default()).unwrap_err();
        assert_eq!(error.code(), ReasonCode::ExtractionUnsafeSymlink);
        assert!(!destination.exists());
        std::fs::remove_dir_all(source).unwrap();
    }
}
