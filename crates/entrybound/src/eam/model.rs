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
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Layout {
    /// Footer-indexed layout with contiguous authoritative manifest records.
    Indexed,
}

/// The entry kinds supported by the first native slice.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EntryKind {
    Directory,
    File,
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

/// Closed metadata names implemented by this slice.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum MetadataName {
    CoreExecutable,
    CoreMtime,
}

impl MetadataName {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CoreExecutable => "core.executable",
            Self::CoreMtime => "core.mtime",
        }
    }

    #[must_use]
    pub const fn participates_in_identity_v1(self) -> bool {
        matches!(self, Self::CoreExecutable)
    }
}

/// Typed values supported by the initial metadata registry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MetadataValue {
    Bool(bool),
    Timestamp(Timestamp),
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
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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
    pub const fn name(self) -> MetadataName {
        self.name
    }

    #[must_use]
    pub const fn value(self) -> MetadataValue {
        self.value
    }

    #[must_use]
    pub const fn criticality(self) -> Criticality {
        self.criticality
    }

    #[must_use]
    pub const fn restorability(self) -> Restorability {
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
        items.sort_by_key(|item| item.name());
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

    #[must_use]
    pub fn executable(&self) -> bool {
        self.items
            .iter()
            .find_map(|item| {
                (item.name == MetadataName::CoreExecutable).then_some(match item.value {
                    MetadataValue::Bool(value) => value,
                    MetadataValue::Timestamp(_) => false,
                })
            })
            .unwrap_or(false)
    }

    #[must_use]
    pub fn mtime(&self) -> Option<Timestamp> {
        self.items.iter().find_map(|item| {
            if item.name == MetadataName::CoreMtime
                && let MetadataValue::Timestamp(value) = item.value
            {
                return Some(value);
            }
            None
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
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EntryData {
    Directory,
    File { content: ContentRef },
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
    pub const fn data(&self) -> EntryData {
        self.data
    }

    #[must_use]
    pub const fn kind(&self) -> EntryKind {
        match self.data {
            EntryData::Directory => EntryKind::Directory,
            EntryData::File { .. } => EntryKind::File,
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
    pub index: Index,
}
