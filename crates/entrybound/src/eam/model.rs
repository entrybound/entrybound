use std::collections::BTreeMap;
use std::fmt;

use super::LogicalPath;
use crate::diagnostics::{Diagnostic, OutcomeClass, ReasonCode, Result};

/// A 256-bit digest value. The active algorithm is declared by the descriptor.
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct Digest([u8; 32]);

impl Digest {
    /// The all-zero value, useful while constructing values before identity is applied.
    pub const ZERO: Self = Self([0; 32]);

    /// Constructs a digest from its exact bytes.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Returns the exact digest bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Display for Digest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

/// The archive's semantic role.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArchiveRole {
    /// All content is internal and the archive is independently extractable.
    Complete,
}

/// The physical ECF layout.
///
/// Layout is a physical and access-capability choice only. Two archives that
/// encode the same EAM under different layouts have identical LAI, PCR, and
/// AUX and differ only in PCI.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Layout {
    /// Footer-indexed layout with contiguous authoritative manifest records.
    Indexed,
    /// Single sequential tagged body written without `Seek` and read without
    /// `Seek`. No Index exists, and entry lookup requires a full scan.
    Stream,
}

impl Layout {
    /// Returns the stable wire discriminant used by the fixed preamble.
    #[must_use]
    pub const fn wire_id(self) -> u8 {
        match self {
            Self::Indexed => 1,
            Self::Stream => 2,
        }
    }

    /// Returns the stable machine-readable layout name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Indexed => "INDEXED",
            Self::Stream => "STREAM",
        }
    }

    /// Whether the layout can resolve one Entry without scanning the container.
    #[must_use]
    pub const fn supports_random_entry_lookup(self) -> bool {
        matches!(self, Self::Indexed)
    }
}

/// Native entry kinds. Hardlinks remain ordinary Files with auxiliary group metadata.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EntryKind {
    Directory,
    File,
    Symlink,
}

/// The only content reference form supported by a Complete bootstrap archive.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContentRef {
    /// A reference to an internal ContentObject by its plaintext logical digest.
    Internal(Digest),
}

/// Timestamp precision as captured from the source.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimestampPrecision {
    Second,
    Centisecond,
    Microsecond,
    Hectonanosecond,
    Nanosecond,
}

/// A signed-seconds timestamp with explicit precision and restorability.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Timestamp {
    seconds: i64,
    nanoseconds: u32,
    source_precision: TimestampPrecision,
    restorable: bool,
}

impl Timestamp {
    /// Constructs a validated timestamp.
    pub fn new(
        seconds: i64,
        nanoseconds: u32,
        source_precision: TimestampPrecision,
        restorable: bool,
    ) -> Result<Self> {
        if nanoseconds >= 1_000_000_000 {
            return Err(Diagnostic::new(
                OutcomeClass::Nonconforming,
                ReasonCode::NoncanonicalEncoding,
                "timestamp nanoseconds must be less than one billion",
            ));
        }
        Ok(Self {
            seconds,
            nanoseconds,
            source_precision,
            restorable,
        })
    }

    #[must_use]
    pub const fn seconds(self) -> i64 {
        self.seconds
    }

    #[must_use]
    pub const fn nanoseconds(self) -> u32 {
        self.nanoseconds
    }

    #[must_use]
    pub const fn source_precision(self) -> TimestampPrecision {
        self.source_precision
    }

    #[must_use]
    pub const fn restorable(self) -> bool {
        self.restorable
    }
}

/// Canonical encoding for one symbolic-link target.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LinkTargetEncoding {
    Utf8,
    PosixBytes,
}

/// Exact symbolic-link target bytes. A target is not a LogicalPath.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LinkTarget {
    encoding: LinkTargetEncoding,
    bytes: Box<[u8]>,
}

impl LinkTarget {
    /// Constructs a canonical target. Valid UTF-8 must use the UTF8 encoding.
    pub fn new(bytes: impl Into<Box<[u8]>>, encoding: LinkTargetEncoding) -> Result<Self> {
        let bytes = bytes.into();
        if bytes.len() > 1024 * 1024 {
            return Err(Diagnostic::new(
                OutcomeClass::PolicyRefused,
                ReasonCode::ResourceLimit,
                "symlink target exceeds the 1 MiB format bound",
            ));
        }
        if bytes.contains(&0) {
            return Err(Diagnostic::new(
                OutcomeClass::Nonconforming,
                ReasonCode::InvalidSymlinkTarget,
                "symlink target contains NUL",
            ));
        }
        match (encoding, std::str::from_utf8(&bytes).is_ok()) {
            (LinkTargetEncoding::Utf8, false) => {
                return Err(Diagnostic::new(
                    OutcomeClass::Nonconforming,
                    ReasonCode::InvalidSymlinkTarget,
                    "UTF8 symlink target contains invalid UTF-8",
                ));
            }
            (LinkTargetEncoding::PosixBytes, true) => {
                return Err(Diagnostic::new(
                    OutcomeClass::Nonconforming,
                    ReasonCode::NoncanonicalEncoding,
                    "valid UTF-8 symlink targets must use the UTF8 encoding",
                ));
            }
            _ => {}
        }
        Ok(Self { encoding, bytes })
    }

    /// Selects the one canonical encoding for exact target bytes.
    pub fn canonical(bytes: impl Into<Box<[u8]>>) -> Result<Self> {
        let bytes = bytes.into();
        let encoding = if std::str::from_utf8(&bytes).is_ok() {
            LinkTargetEncoding::Utf8
        } else {
            LinkTargetEncoding::PosixBytes
        };
        Self::new(bytes, encoding)
    }

    #[must_use]
    pub const fn encoding(&self) -> LinkTargetEncoding {
        self.encoding
    }

    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// One exact POSIX extended attribute.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct XAttr {
    name: Box<[u8]>,
    value: Box<[u8]>,
}

impl XAttr {
    pub const MAX_NAME_BYTES: usize = 255;
    pub const MAX_VALUE_BYTES: usize = 16 * 1024 * 1024;

    pub fn new(name: impl Into<Box<[u8]>>, value: impl Into<Box<[u8]>>) -> Result<Self> {
        let name = name.into();
        let value = value.into();
        if name.is_empty() || name.len() > Self::MAX_NAME_BYTES || name.contains(&0) {
            return Err(Diagnostic::new(
                OutcomeClass::Nonconforming,
                ReasonCode::InvalidXattr,
                "xattr name must be nonempty, NUL-free, and at most 255 bytes",
            ));
        }
        if value.len() > Self::MAX_VALUE_BYTES {
            return Err(Diagnostic::new(
                OutcomeClass::PolicyRefused,
                ReasonCode::ResourceLimit,
                "xattr value exceeds the 16 MiB format bound",
            ));
        }
        Ok(Self { name, value })
    }

    #[must_use]
    pub fn name(&self) -> &[u8] {
        &self.name
    }

    #[must_use]
    pub fn value(&self) -> &[u8] {
        &self.value
    }
}

/// One allocated data extent in a sparse file.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct SparseExtent {
    pub offset: u64,
    pub length: u64,
}

/// Canonical sparse topology for one full logical ContentObject.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SparseMap {
    logical_size: u64,
    extents: Box<[SparseExtent]>,
}

impl SparseMap {
    pub const MAX_EXTENTS: usize = 1_000_000;

    pub fn new(logical_size: u64, extents: Vec<SparseExtent>) -> Result<Self> {
        if extents.len() > Self::MAX_EXTENTS {
            return Err(Diagnostic::new(
                OutcomeClass::PolicyRefused,
                ReasonCode::ResourceLimit,
                "sparse extent count exceeds the format bound",
            ));
        }
        let mut previous_end = 0_u64;
        for (index, extent) in extents.iter().enumerate() {
            let end = extent.offset.checked_add(extent.length).ok_or_else(|| {
                Diagnostic::new(
                    OutcomeClass::Nonconforming,
                    ReasonCode::InvalidSparseMap,
                    "sparse extent overflows u64",
                )
            })?;
            if extent.length == 0
                || end > logical_size
                || index > 0 && extent.offset <= previous_end
            {
                return Err(Diagnostic::new(
                    OutcomeClass::Nonconforming,
                    ReasonCode::InvalidSparseMap,
                    "sparse extents must be nonzero, ordered, nonoverlapping, nonadjacent, and in range",
                ));
            }
            previous_end = end;
        }
        Ok(Self {
            logical_size,
            extents: extents.into_boxed_slice(),
        })
    }

    #[must_use]
    pub const fn logical_size(&self) -> u64 {
        self.logical_size
    }

    #[must_use]
    pub fn extents(&self) -> &[SparseExtent] {
        &self.extents
    }

    /// Verifies that one logical byte range agrees with the declared holes.
    pub fn validate_range(&self, offset: u64, bytes: &[u8]) -> Result<()> {
        let end = offset
            .checked_add(u64::try_from(bytes.len()).unwrap_or(u64::MAX))
            .ok_or_else(|| {
                Diagnostic::new(
                    OutcomeClass::Nonconforming,
                    ReasonCode::InvalidSparseMap,
                    "sparse validation range overflows u64",
                )
            })?;
        if end > self.logical_size {
            return Err(Diagnostic::new(
                OutcomeClass::Nonconforming,
                ReasonCode::InvalidSparseMap,
                "sparse validation range exceeds logical size",
            ));
        }
        let require_zero = |start: u64, hole_end: u64| -> Result<()> {
            let local_start = usize::try_from(start - offset).map_err(|_| {
                Diagnostic::new(
                    OutcomeClass::PolicyRefused,
                    ReasonCode::ResourceLimit,
                    "sparse validation offset exceeds usize",
                )
            })?;
            let local_end = usize::try_from(hole_end - offset).map_err(|_| {
                Diagnostic::new(
                    OutcomeClass::PolicyRefused,
                    ReasonCode::ResourceLimit,
                    "sparse validation offset exceeds usize",
                )
            })?;
            if bytes[local_start..local_end].iter().any(|byte| *byte != 0) {
                return Err(Diagnostic::new(
                    OutcomeClass::Corrupt,
                    ReasonCode::InvalidSparseMap,
                    "sparse map declares a hole containing a nonzero logical byte",
                ));
            }
            Ok(())
        };
        let mut cursor = offset;
        for extent in &self.extents {
            let extent_end = extent.offset + extent.length;
            if extent_end <= cursor {
                continue;
            }
            if extent.offset >= end {
                break;
            }
            let data_start = extent.offset.max(cursor);
            require_zero(cursor, data_start)?;
            cursor = extent_end.min(end);
            if cursor == end {
                return Ok(());
            }
        }
        require_zero(cursor, end)
    }

    /// Verifies one complete logical file against this topology.
    pub fn validate_plaintext(&self, bytes: &[u8]) -> Result<()> {
        if u64::try_from(bytes.len()).unwrap_or(u64::MAX) != self.logical_size {
            return Err(Diagnostic::new(
                OutcomeClass::Nonconforming,
                ReasonCode::InvalidSparseMap,
                "sparse map logical size differs from file content",
            ));
        }
        self.validate_range(0, bytes)
    }
}

/// Closed metadata names implemented by native metadata v2.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum MetadataName {
    CoreExecutable,
    CoreMtime,
    PosixMode,
    PosixUid,
    PosixGid,
    PosixHardlinkGroup,
    PosixXattrs,
    PosixSparseMap,
}

impl MetadataName {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CoreExecutable => "core.executable",
            Self::CoreMtime => "core.mtime",
            Self::PosixMode => "posix.mode",
            Self::PosixUid => "posix.uid",
            Self::PosixGid => "posix.gid",
            Self::PosixHardlinkGroup => "posix.hardlink-group",
            Self::PosixXattrs => "posix.xattrs",
            Self::PosixSparseMap => "posix.sparse-map",
        }
    }

    #[must_use]
    pub const fn participates_in_identity_v1(self) -> bool {
        matches!(self, Self::CoreExecutable)
    }
}

/// Typed values supported by the metadata-v2 registry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MetadataValue {
    Bool(bool),
    Timestamp(Timestamp),
    PosixMode(u32),
    PosixUid(u32),
    PosixGid(u32),
    HardlinkGroup(Digest),
    Xattrs(Box<[XAttr]>),
    SparseMap(SparseMap),
}

/// Whether an unaware reader may ignore a metadata item.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Criticality {
    Optional,
    Critical,
}

/// Whether and how an extractor may restore a metadata item.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Restorability {
    Restorable,
    CaptureOnly,
}

/// One typed item in a MetadataSet.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MetadataItem {
    name: MetadataName,
    value: MetadataValue,
    criticality: Criticality,
    restorability: Restorability,
}

impl MetadataItem {
    #[must_use]
    pub const fn executable(value: bool) -> Self {
        Self {
            name: MetadataName::CoreExecutable,
            value: MetadataValue::Bool(value),
            criticality: Criticality::Optional,
            restorability: Restorability::Restorable,
        }
    }

    #[must_use]
    pub const fn mtime(value: Timestamp) -> Self {
        Self {
            name: MetadataName::CoreMtime,
            value: MetadataValue::Timestamp(value),
            criticality: Criticality::Optional,
            restorability: Restorability::Restorable,
        }
    }

    #[must_use]
    pub const fn posix_mode(value: u32) -> Self {
        Self {
            name: MetadataName::PosixMode,
            value: MetadataValue::PosixMode(value & 0o7777),
            criticality: Criticality::Optional,
            restorability: Restorability::Restorable,
        }
    }

    #[must_use]
    pub const fn posix_uid(value: u32) -> Self {
        Self {
            name: MetadataName::PosixUid,
            value: MetadataValue::PosixUid(value),
            criticality: Criticality::Optional,
            restorability: Restorability::Restorable,
        }
    }

    #[must_use]
    pub const fn posix_gid(value: u32) -> Self {
        Self {
            name: MetadataName::PosixGid,
            value: MetadataValue::PosixGid(value),
            criticality: Criticality::Optional,
            restorability: Restorability::Restorable,
        }
    }

    #[must_use]
    pub const fn hardlink_group(value: Digest) -> Self {
        Self {
            name: MetadataName::PosixHardlinkGroup,
            value: MetadataValue::HardlinkGroup(value),
            criticality: Criticality::Optional,
            restorability: Restorability::Restorable,
        }
    }

    pub fn xattrs(mut value: Vec<XAttr>) -> Result<Self> {
        if value.len() > 4096 {
            return Err(Diagnostic::new(
                OutcomeClass::PolicyRefused,
                ReasonCode::ResourceLimit,
                "xattr count exceeds the per-entry format bound",
            ));
        }
        value.sort();
        if value
            .windows(2)
            .any(|pair| pair[0].name() == pair[1].name())
        {
            return Err(Diagnostic::new(
                OutcomeClass::Nonconforming,
                ReasonCode::DuplicateXattr,
                "xattr names must be unique",
            ));
        }
        Ok(Self {
            name: MetadataName::PosixXattrs,
            value: MetadataValue::Xattrs(value.into_boxed_slice()),
            criticality: Criticality::Optional,
            restorability: Restorability::Restorable,
        })
    }

    #[must_use]
    pub const fn sparse_map(value: SparseMap) -> Self {
        Self {
            name: MetadataName::PosixSparseMap,
            value: MetadataValue::SparseMap(value),
            criticality: Criticality::Optional,
            restorability: Restorability::Restorable,
        }
    }

    #[must_use]
    pub const fn name(&self) -> MetadataName {
        self.name
    }

    #[must_use]
    pub const fn value(&self) -> &MetadataValue {
        &self.value
    }

    #[must_use]
    pub const fn criticality(&self) -> Criticality {
        self.criticality
    }

    #[must_use]
    pub const fn restorability(&self) -> Restorability {
        self.restorability
    }
}

/// A canonical, name-sorted set of metadata items.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MetadataSet {
    items: Box<[MetadataItem]>,
}

impl MetadataSet {
    /// Constructs a canonical MetadataSet and rejects duplicate declarations.
    pub fn new(mut items: Vec<MetadataItem>) -> Result<Self> {
        items.sort_by_key(MetadataItem::name);
        if items
            .windows(2)
            .any(|pair| pair[0].name() == pair[1].name())
        {
            return Err(Diagnostic::new(
                OutcomeClass::Nonconforming,
                ReasonCode::DuplicateSemanticDeclaration,
                "a metadata name may appear only once in this subset",
            ));
        }
        Ok(Self {
            items: items.into_boxed_slice(),
        })
    }

    #[must_use]
    pub fn items(&self) -> &[MetadataItem] {
        &self.items
    }

    /// Returns a new canonical set containing one additional unique item.
    pub fn with_item(&self, item: MetadataItem) -> Result<Self> {
        let mut items = self.items.to_vec();
        items.push(item);
        Self::new(items)
    }

    #[must_use]
    pub fn executable(&self) -> bool {
        self.items
            .iter()
            .find_map(|item| {
                (item.name == MetadataName::CoreExecutable).then_some(match item.value {
                    MetadataValue::Bool(value) => value,
                    _ => false,
                })
            })
            .unwrap_or(false)
    }

    #[must_use]
    pub fn mtime(&self) -> Option<Timestamp> {
        self.items.iter().find_map(|item| {
            if item.name == MetadataName::CoreMtime
                && let MetadataValue::Timestamp(value) = &item.value
            {
                return Some(*value);
            }
            None
        })
    }

    #[must_use]
    pub fn posix_mode(&self) -> Option<u32> {
        self.items.iter().find_map(|item| match item.value() {
            MetadataValue::PosixMode(value) => Some(*value),
            _ => None,
        })
    }

    #[must_use]
    pub fn posix_uid(&self) -> Option<u32> {
        self.items.iter().find_map(|item| match item.value() {
            MetadataValue::PosixUid(value) => Some(*value),
            _ => None,
        })
    }

    #[must_use]
    pub fn posix_gid(&self) -> Option<u32> {
        self.items.iter().find_map(|item| match item.value() {
            MetadataValue::PosixGid(value) => Some(*value),
            _ => None,
        })
    }

    #[must_use]
    pub fn hardlink_group(&self) -> Option<Digest> {
        self.items.iter().find_map(|item| match item.value() {
            MetadataValue::HardlinkGroup(value) => Some(*value),
            _ => None,
        })
    }

    #[must_use]
    pub fn xattrs(&self) -> &[XAttr] {
        self.items
            .iter()
            .find_map(|item| match item.value() {
                MetadataValue::Xattrs(value) => Some(value.as_ref()),
                _ => None,
            })
            .unwrap_or_default()
    }

    #[must_use]
    pub fn sparse_map(&self) -> Option<&SparseMap> {
        self.items.iter().find_map(|item| match item.value() {
            MetadataValue::SparseMap(value) => Some(value),
            _ => None,
        })
    }

    #[must_use]
    pub fn uses_posix_v1(&self) -> bool {
        self.items.iter().any(|item| {
            !matches!(
                item.name(),
                MetadataName::CoreExecutable | MetadataName::CoreMtime
            )
        })
    }
}

/// The two explicit entry digests defined by the architecture.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct EntryIdentity {
    pub identity_digest: Digest,
    pub aux_digest: Digest,
}

/// Kind-specific Entry data. This prevents directories from carrying content.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EntryData {
    Directory,
    File { content: ContentRef },
    Symlink { target: LinkTarget },
}

/// The sole authority for one archived object.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Entry {
    path: LogicalPath,
    data: EntryData,
    metadata: MetadataSet,
    identity: EntryIdentity,
}

impl Entry {
    #[must_use]
    pub const fn new(
        path: LogicalPath,
        data: EntryData,
        metadata: MetadataSet,
        identity: EntryIdentity,
    ) -> Self {
        Self {
            path,
            data,
            metadata,
            identity,
        }
    }

    #[must_use]
    pub fn path(&self) -> &LogicalPath {
        &self.path
    }

    #[must_use]
    pub const fn data(&self) -> &EntryData {
        &self.data
    }

    #[must_use]
    pub const fn kind(&self) -> EntryKind {
        match &self.data {
            EntryData::Directory => EntryKind::Directory,
            EntryData::File { .. } => EntryKind::File,
            EntryData::Symlink { .. } => EntryKind::Symlink,
        }
    }

    #[must_use]
    pub fn metadata(&self) -> &MetadataSet {
        &self.metadata
    }

    #[must_use]
    pub const fn identity(&self) -> EntryIdentity {
        self.identity
    }

    #[must_use]
    pub fn uses_posix_v1(&self) -> bool {
        matches!(&self.data, EntryData::Symlink { .. }) || self.metadata.uses_posix_v1()
    }

    pub(crate) fn replace_metadata(&mut self, metadata: MetadataSet) {
        self.metadata = metadata;
    }
}

/// The authoritative, canonically ordered set of entries.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EntrySet {
    pub(crate) entries: Box<[Entry]>,
}

impl EntrySet {
    #[must_use]
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// A reference to a plaintext-addressed Chunk.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChunkRef {
    pub chunk_id: Digest,
}

/// An immutable plaintext byte sequence independent of chunking.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContentObject {
    pub logical_digest: Digest,
    pub chunk_root: Digest,
    pub chunks: Box<[ChunkRef]>,
}

/// One plaintext-addressed physical unit. A group reference, when present,
/// declares a bounded dependency on preceding same-group CHUNK_DATA frames.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Chunk {
    pub chunk_id: Digest,
    pub logical_len: u64,
    pub plan_ref: u64,
    pub group_ref: Option<Digest>,
    pub plaintext: Box<[u8]>,
}

/// A first-class shared codec dictionary, addressed by its exact bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Dictionary {
    pub dictionary_id: Digest,
    pub codec: String,
    pub format: String,
    pub construction: String,
    pub bytes: Box<[u8]>,
}

/// Physical side data needed to recreate an original format representation.
/// Its identity covers the exact reconstruction bytes; it is never part of
/// logical archive identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReconstructionData {
    pub reconstruction_id: Digest,
    pub format: String,
    /// Length of the format-neutral intermediate supplied to reconstruction.
    pub intermediate_len: u64,
    pub bytes: Box<[u8]>,
}

/// Creation-time reason that an attempted reconstructive representation was
/// not selected. This is a non-authoritative physical planning audit record.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReconstructionFallbackReason {
    UnrecognizedOrVerificationFailed,
    CompleteCostDidNotWin,
}

/// Explicit target for non-authoritative reconstructive planning evidence.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ReconstructionAuditTarget {
    Chunk(Digest),
    ContentObject(Digest),
    Region(Digest),
}

/// Frozen v6 whole-object reconstruction fallback reasons.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReconstructionAuditReason {
    NotRecognized,
    Unsupported,
    ExactVerificationFailed,
    CompleteCostDidNotWin,
    RegionDedupConflict,
    ResourcePolicyExcluded,
}

/// Non-authoritative creation-time audit for one explicit target.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReconstructionAudit {
    pub target: ReconstructionAuditTarget,
    pub transform_id: String,
    pub reason: ReconstructionAuditReason,
}

/// Declared worst-case cost of accessing any logical Chunk in a region.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RegionAccessCost {
    pub logical_bytes: u64,
    pub logical_chunks: u64,
    pub worst_reconstructed_bytes: u64,
}

/// One physical representation for a contiguous ContentObject Chunk range.
/// Membership is authoritative only through `content_object`, `start_chunk_index`,
/// and `chunk_count`; no member list is stored here.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReconstructionRegion {
    pub region_id: Digest,
    pub content_object: Digest,
    pub start_chunk_index: u64,
    pub chunk_count: u64,
    pub plan_ref: u64,
    pub logical_bytes: u64,
    pub transformed_bytes: u64,
    pub ordinary_physical_bytes: u64,
    pub region_overhead_bytes: u64,
    pub access: RegionAccessCost,
    pub representation: Box<[u8]>,
}

/// Bounded physical dependency declaration. Membership exists only through
/// `Chunk::group_ref`; the CHUNK_DATA order supplies preceding-member order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChunkGroup {
    pub group_id: Digest,
    pub max_lookback: u32,
    pub max_preceding_bytes: u64,
}

/// Authoritative content plus its creation-time physical layout plan.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ContentStore {
    pub objects: BTreeMap<Digest, ContentObject>,
    pub chunks: BTreeMap<Digest, Chunk>,
    pub dictionaries: BTreeMap<Digest, Dictionary>,
    pub reconstruction_data: BTreeMap<Digest, ReconstructionData>,
    pub reconstruction_fallbacks: BTreeMap<Digest, ReconstructionFallbackReason>,
    pub reconstruction_regions: BTreeMap<Digest, ReconstructionRegion>,
    pub reconstruction_audits: BTreeMap<ReconstructionAuditTarget, ReconstructionAudit>,
    pub chunk_groups: BTreeMap<Digest, ChunkGroup>,
    /// Exact CHUNK_DATA frame order. It is physical only and never changes
    /// ContentObject reference order or any logical identity.
    pub physical_order: Box<[Digest]>,
}

/// Decode resources declared by a TransformPlan or aggregate descriptor.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DecodeRequirements {
    pub window_bytes: u64,
    pub working_set_bytes: u64,
    pub flags: u32,
}

/// A decoder-facing plan. The planner itself is not needed to decode it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransformStep {
    pub transform_id: String,
    pub parameters: Box<[u8]>,
    /// Present only for a reconstructive step. Structural steps have no side
    /// data reference.
    pub reconstruction_ref: Option<Digest>,
}

/// A decoder-facing plan. The planner itself is not needed to decode it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransformPlan {
    pub plan_id: u64,
    pub identifier: String,
    pub transforms: Box<[TransformStep]>,
    pub codec: String,
    pub codec_params: Box<[u8]>,
    pub dictionary: Option<Digest>,
    pub decode: DecodeRequirements,
}

/// A typed fidelity limitation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FidelityIssue {
    pub class: String,
    pub reason: String,
    pub entry_scope: Option<LogicalPath>,
}

/// In-band declaration of captured and unsupported fidelity classes.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FidelityReport {
    pub captured: Box<[String]>,
    pub unavailable: Box<[FidelityIssue]>,
    pub degraded: Box<[FidelityIssue]>,
    pub platform: String,
    pub filesystem: Box<[String]>,
}

/// One deterministic reconciliation decision retained as auxiliary evidence.
/// It describes how foreign observations were projected; it is never an
/// authority for the resulting Entry or ContentObject.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ConversionResolution {
    pub conflict_class: String,
    pub semantic_field: String,
    pub authorities: Box<[String]>,
    pub observed_values: Box<[String]>,
    pub action: String,
}

/// In-band provenance for one foreign-to-native conversion.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConversionProvenance {
    pub source_format: String,
    pub adapter_id: String,
    pub source_digest: Digest,
    pub import_mode: String,
    pub source_entry_count: u64,
    pub observation_count: u64,
    pub omission_count: u64,
    pub refinement_count: u64,
    pub divergence_count: u64,
    pub irreconcilable_count: u64,
    pub resolutions: Box<[ConversionResolution]>,
    pub synthesized_ancestors: Box<[LogicalPath]>,
    pub unsupported_metadata: Box<[String]>,
    pub outcome: String,
}

/// Format-neutral authority retained for forensic legacy evidence.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct PreservedLegacyAuthority {
    pub format: String,
    pub structure: String,
    pub instance: u64,
}

/// Exact source range supporting one preserved claim.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct PreservedLegacyLocation {
    pub offset: u64,
    pub length: u64,
}

/// Typed value retained from the format-neutral observation model.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum PreservedLegacyValue {
    Bytes(Box<[u8]>),
    Unsigned(u64),
    Signed(i64),
    Text(String),
    Boolean(bool),
}

/// Parser validity state, frozen independently from reconciliation policy.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum PreservedLegacyValidity {
    Valid,
    Invalid,
    Uninterpreted,
}

/// One ordered LOM observation. Scope 0 is archive-wide and scope 1 identifies
/// a source entry by `subject_ordinal`.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct PreservedLegacyObservation {
    pub scope: u8,
    pub subject_ordinal: u64,
    pub observation_ordinal: u64,
    pub semantic_field: String,
    pub authority: PreservedLegacyAuthority,
    pub raw_value: Box<[u8]>,
    pub interpreted_value: Option<PreservedLegacyValue>,
    pub evidence: PreservedLegacyLocation,
    pub validity: PreservedLegacyValidity,
}

/// Policy-independent resolution attached to a preserved LOM conflict.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct PreservedLegacyResolution {
    pub action: String,
    pub selected_authority: Option<PreservedLegacyAuthority>,
}

/// One ordered preserved conflict with all competing observations retained.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct PreservedLegacyConflict {
    pub ordinal: u64,
    pub semantic_field: String,
    pub authorities: Box<[PreservedLegacyAuthority]>,
    pub observed_values: Box<[PreservedLegacyValue]>,
    pub evidence: Box<[PreservedLegacyLocation]>,
    pub classification: String,
    pub resolution: Option<PreservedLegacyResolution>,
}

/// Exact-source and structured format-neutral evidence retained by preserve-v1.
/// The compatibility profile remains authoritative in ConversionProvenance's
/// versioned adapter ID rather than being duplicated here.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LegacyPreservation {
    pub preservation_format: String,
    pub source_format: String,
    pub source_digest: Digest,
    pub source_bytes: Box<[u8]>,
    pub observations: Box<[PreservedLegacyObservation]>,
    pub conflicts: Box<[PreservedLegacyConflict]>,
    pub selected_resolutions: Box<[ConversionResolution]>,
}

/// A cached physical locator for one chunk frame.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChunkLocation {
    pub offset: u64,
    pub stored_len: u64,
}

/// A reconstructible, non-authoritative acceleration structure.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Index {
    pub present: bool,
    pub valid: bool,
    pub chunks: BTreeMap<Digest, ChunkLocation>,
    pub status: String,
}

/// The archive's three-tier feature declaration.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FeatureSet {
    pub incompat: u64,
    pub read_only_compat: u64,
    pub compat: u64,
}

/// Declared upper bounds used before and during decoding.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ResourceBudget {
    pub entry_count: u64,
    pub total_logical_bytes: u64,
    pub max_single_entry_logical_bytes: u64,
    pub max_expansion_ratio_milli: u64,
    pub chunk_count: u64,
    pub max_path_depth: u64,
    pub max_metadata_bytes: u64,
    pub max_key_derivation_cost: u64,
}

/// The identity profile selected by an archive.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IdentityProfile {
    IdentityV1,
}

/// The single digest algorithm selected by this experimental format version.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DigestAlgorithm {
    Sha256,
}

/// Descriptor data not needed in the fixed preamble.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArchiveDescriptor {
    pub format_major: u16,
    pub format_minor: u16,
    pub format_namespace: String,
    pub features: FeatureSet,
    pub layout: Layout,
    pub role: ArchiveRole,
    pub budget_declared: bool,
    /// Declared bound on how far a sequential semantic reference may depend on
    /// an already emitted unique Chunk. It is always zero in INDEXED layout,
    /// where random access makes historical retention unnecessary.
    pub stream_dedup_window: u64,
    pub budget: ResourceBudget,
    pub decode: DecodeRequirements,
    pub identity_profile: IdentityProfile,
    pub digest_algorithm: DigestAlgorithm,
    pub planner_id: String,
    pub chunker_id: String,
    pub lai: Digest,
    pub pcr: Digest,
    pub aux: Digest,
    pub pci: Option<Digest>,
}

/// The authoritative EAM plus its reconstructible Index cache.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Archive {
    pub descriptor: ArchiveDescriptor,
    pub entry_set: EntrySet,
    pub content_store: ContentStore,
    pub transform_plans: Box<[TransformPlan]>,
    pub fidelity: FidelityReport,
    /// Auxiliary conversion evidence. Native archives have no value here.
    pub conversion: Option<ConversionProvenance>,
    /// Optional exact foreign-source snapshot and structured LOM evidence.
    pub preservation: Option<LegacyPreservation>,
    pub index: Index,
}

#[cfg(test)]
mod posix_tests {
    use super::{LinkTarget, LinkTargetEncoding, MetadataItem, SparseExtent, SparseMap, XAttr};
    use crate::diagnostics::ReasonCode;

    #[test]
    fn link_target_encoding_has_one_canonical_form() {
        assert_eq!(
            LinkTarget::canonical(b"../absolute/semantics-are-preserved".to_vec())
                .unwrap()
                .encoding(),
            LinkTargetEncoding::Utf8
        );
        assert_eq!(
            LinkTarget::canonical(vec![0xff, b'/', b'x'])
                .unwrap()
                .encoding(),
            LinkTargetEncoding::PosixBytes
        );
        assert_eq!(
            LinkTarget::new(b"valid".to_vec(), LinkTargetEncoding::PosixBytes)
                .unwrap_err()
                .code(),
            ReasonCode::NoncanonicalEncoding
        );
        assert_eq!(
            LinkTarget::new(vec![0xff], LinkTargetEncoding::Utf8)
                .unwrap_err()
                .code(),
            ReasonCode::InvalidSymlinkTarget
        );
        assert_eq!(
            LinkTarget::canonical(b"bad\0target".to_vec())
                .unwrap_err()
                .code(),
            ReasonCode::InvalidSymlinkTarget
        );
    }

    #[test]
    fn xattrs_and_sparse_extents_are_closed_and_canonical() {
        let duplicate = MetadataItem::xattrs(vec![
            XAttr::new(b"user.a".to_vec(), b"one".to_vec()).unwrap(),
            XAttr::new(b"user.a".to_vec(), b"two".to_vec()).unwrap(),
        ])
        .unwrap_err();
        assert_eq!(duplicate.code(), ReasonCode::DuplicateXattr);
        assert_eq!(
            XAttr::new(Vec::<u8>::new(), Vec::<u8>::new())
                .unwrap_err()
                .code(),
            ReasonCode::InvalidXattr
        );
        for extents in [
            vec![SparseExtent {
                offset: 0,
                length: 0,
            }],
            vec![
                SparseExtent {
                    offset: 0,
                    length: 4,
                },
                SparseExtent {
                    offset: 4,
                    length: 2,
                },
            ],
            vec![SparseExtent {
                offset: 8,
                length: 4,
            }],
        ] {
            assert_eq!(
                SparseMap::new(10, extents).unwrap_err().code(),
                ReasonCode::InvalidSparseMap
            );
        }
        let all_hole = SparseMap::new(8, Vec::new()).unwrap();
        all_hole.validate_plaintext(&[0; 8]).unwrap();
        assert_eq!(
            all_hole
                .validate_plaintext(&[0, 0, 1, 0, 0, 0, 0, 0])
                .unwrap_err()
                .code(),
            ReasonCode::InvalidSparseMap
        );
    }
}
