//! Native Entrybound Container Format declarations.
//!
//! ECF remains an encoding of EAM. The Index is explicitly absent from the
//! authoritative section set so a reader can discard and rebuild it.

use crate::diagnostics::Result;
use crate::eam::{ArchiveRole, Layout};
use crate::eam::{ChunkGroup, Dictionary, Digest, ReconstructionData, TransformPlan};

mod container;
mod records;
mod staging;
mod stream;

pub use container::{
    EncodedArchive, IndexStatus, OpenedArchive, VerificationReport, WriteOptions, encode, open,
    open_with_limits, open_with_policy, peek_layout, verify, verify_with_limits,
    verify_with_policy,
};
pub use staging::StagingLimits;
pub(crate) use stream::StagedChunks;
pub use stream::{
    STREAM_FOOTER_LEN, STREAM_ITEM_HEADER_LEN, STREAM_ITEM_MAGIC, SequentialArchive,
    SequentialLimits, StreamAccessProfile, StreamContentPolicy, StreamItemTag, StreamReport,
    StreamWindow, StreamWriteOptions, StreamWriteSummary, bootstrap_sequential_limits,
    bootstrap_staging_limits, encode_stream, open_stream, open_stream_with_limits,
    open_stream_with_policy, verify_stream, verify_stream_with_limits,
};

/// Candidate Entrybound magic selected by the architecture specification.
pub const MAGIC: [u8; 8] = [0x8e, b'E', b'B', b'1', b'\r', b'\n', 0x1a, b'\n'];

/// Fixed bootstrap preamble width.
pub const PREAMBLE_LEN: u64 = 256;

/// Fixed bootstrap footer width.
pub const FOOTER_LEN: u64 = 128;

/// Fixed section-header width.
pub const SECTION_HEADER_LEN: u64 = 64;

/// Fixed plan-driven Chunk-frame header width.
pub const CHUNK_FRAME_HEADER_LEN: u64 = 64;

/// Extended Chunk frame width carrying the sole authoritative group_ref.
pub const CHUNK_FRAME_V2_HEADER_LEN: u64 = 96;

/// Required capability for Dictionary/ChunkGroup sections and v2 frames.
pub const FEATURE_CROSS_FILE_COMPRESSION_V1: u64 = 1 << 0;
/// Required capability for first-class TransformSteps and the v4 codec registry.
pub const FEATURE_CODEC_TRANSFORM_V1: u64 = 1 << 1;
/// Required capability for TransformStep v2 and ReconstructionData objects.
pub const FEATURE_RECONSTRUCTIVE_TRANSFORM_V1: u64 = 1 << 2;
/// Required capability for whole-ContentObject reconstruction regions and v3 steps.
pub const FEATURE_WHOLE_OBJECT_RECONSTRUCTION_V1: u64 = 1 << 3;
/// Required capability for the single sequential tagged `STREAM_BODY`.
///
/// A reader that does not implement this bit must refuse the archive rather
/// than attempt to interpret its bytes as INDEXED sections.
pub const FEATURE_STREAM_LAYOUT_V1: u64 = 1 << 4;
pub(crate) const SUPPORTED_INCOMPAT_FEATURES: u64 = FEATURE_CROSS_FILE_COMPRESSION_V1
    | FEATURE_CODEC_TRANSFORM_V1
    | FEATURE_RECONSTRUCTIVE_TRANSFORM_V1
    | FEATURE_WHOLE_OBJECT_RECONSTRUCTION_V1
    | FEATURE_STREAM_LAYOUT_V1
    | crate::crypto::CRYPTO_FEATURES;

pub(crate) use container::{
    EncryptedDecodedParts, EncryptedPlainParts, open_encrypted_plain_parts,
    prepare_encrypted_plain_parts,
};

/// Versioned namespace for the experimental encoding.
pub const FORMAT_NAMESPACE: &str = "ecf/bootstrap-v1";

pub(crate) fn encoded_transform_plan_len(plan: &TransformPlan) -> Result<u64> {
    u64::try_from(records::encode_transform_plans(std::slice::from_ref(plan), true)?.len()).map_err(
        |_| {
            crate::diagnostics::Diagnostic::new(
                crate::diagnostics::OutcomeClass::PolicyRefused,
                crate::diagnostics::ReasonCode::ResourceLimit,
                "encoded TransformPlan length exceeds u64",
            )
        },
    )
}

pub(crate) fn encoded_transform_plan_v2_len(plan: &TransformPlan) -> Result<u64> {
    u64::try_from(records::encode_transform_plans_v2(std::slice::from_ref(plan))?.len()).map_err(
        |_| {
            crate::diagnostics::Diagnostic::new(
                crate::diagnostics::OutcomeClass::PolicyRefused,
                crate::diagnostics::ReasonCode::ResourceLimit,
                "encoded TransformPlan v2 length exceeds u64",
            )
        },
    )
}

pub(crate) fn encoded_transform_plan_v3_len(plan: &TransformPlan) -> Result<u64> {
    u64::try_from(records::encode_transform_plans_v3(std::slice::from_ref(plan))?.len()).map_err(
        |_| {
            crate::diagnostics::Diagnostic::new(
                crate::diagnostics::OutcomeClass::PolicyRefused,
                crate::diagnostics::ReasonCode::ResourceLimit,
                "encoded TransformPlan v3 length exceeds u64",
            )
        },
    )
}

pub(crate) fn encoded_reconstruction_region_len(
    region: &crate::eam::ReconstructionRegion,
) -> Result<u64> {
    let regions = std::collections::BTreeMap::from([(region.region_id, region.clone())]);
    u64::try_from(
        records::encode_reconstruction_regions(&regions, &std::collections::BTreeMap::new())?.len(),
    )
    .map_err(|_| {
        crate::diagnostics::Diagnostic::new(
            crate::diagnostics::OutcomeClass::PolicyRefused,
            crate::diagnostics::ReasonCode::ResourceLimit,
            "encoded ReconstructionRegion length exceeds u64",
        )
    })
}

pub(crate) fn encoded_reconstruction_data_len(value: &ReconstructionData) -> Result<u64> {
    let values = std::collections::BTreeMap::from([(value.reconstruction_id, value.clone())]);
    u64::try_from(records::encode_reconstruction_data(&values)?.len()).map_err(|_| {
        crate::diagnostics::Diagnostic::new(
            crate::diagnostics::OutcomeClass::PolicyRefused,
            crate::diagnostics::ReasonCode::ResourceLimit,
            "encoded ReconstructionData length exceeds u64",
        )
    })
}

pub(crate) fn encoded_dictionary_len(dictionary: &Dictionary) -> Result<u64> {
    let dictionaries = std::collections::BTreeMap::<Digest, Dictionary>::from([(
        dictionary.dictionary_id,
        dictionary.clone(),
    )]);
    u64::try_from(records::encode_dictionaries(&dictionaries)?.len()).map_err(|_| {
        crate::diagnostics::Diagnostic::new(
            crate::diagnostics::OutcomeClass::PolicyRefused,
            crate::diagnostics::ReasonCode::ResourceLimit,
            "encoded Dictionary length exceeds u64",
        )
    })
}

pub(crate) fn encoded_chunk_group_len(group: &ChunkGroup) -> Result<u64> {
    let groups =
        std::collections::BTreeMap::<Digest, ChunkGroup>::from([(group.group_id, group.clone())]);
    u64::try_from(records::encode_chunk_groups(&groups)?.len()).map_err(|_| {
        crate::diagnostics::Diagnostic::new(
            crate::diagnostics::OutcomeClass::PolicyRefused,
            crate::diagnostics::ReasonCode::ResourceLimit,
            "encoded ChunkGroup length exceeds u64",
        )
    })
}

/// The supported experimental format version.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FormatVersion {
    pub major: u16,
    pub minor: u16,
}

impl FormatVersion {
    pub const BOOTSTRAP: Self = Self { major: 0, minor: 1 };
}

/// Section types justified by the first native vertical slice.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SectionKind {
    Descriptor,
    TransformPlans,
    Dictionaries,
    ChunkGroups,
    ReconstructionData,
    ReconstructionRegions,
    ChunkData,
    ManifestRecords,
    Fidelity,
    /// Non-authoritative and fully reconstructible.
    Index,
}

/// Capabilities a bootstrap writer declares before payload allocation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BootstrapCapabilities {
    pub version: FormatVersion,
    pub layout: Layout,
    pub role: ArchiveRole,
    pub budget_declared: bool,
}

impl Default for BootstrapCapabilities {
    fn default() -> Self {
        Self {
            version: FormatVersion::BOOTSTRAP,
            layout: Layout::Indexed,
            role: ArchiveRole::Complete,
            budget_declared: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BootstrapCapabilities, FEATURE_STREAM_LAYOUT_V1, MAGIC, SUPPORTED_INCOMPAT_FEATURES,
    };
    use crate::eam::{ArchiveRole, Layout};

    #[test]
    fn bootstrap_capabilities_are_complete_and_indexed() {
        let capabilities = BootstrapCapabilities::default();
        assert_eq!(capabilities.layout, Layout::Indexed);
        assert_eq!(capabilities.role, ArchiveRole::Complete);
        assert!(capabilities.budget_declared);
        assert_eq!(MAGIC, [0x8e, b'E', b'B', b'1', 13, 10, 26, 10]);
    }

    #[test]
    fn stream_layout_is_a_required_incompatibility_bit() {
        assert_eq!(FEATURE_STREAM_LAYOUT_V1, 0x10);
        assert_ne!(SUPPORTED_INCOMPAT_FEATURES & FEATURE_STREAM_LAYOUT_V1, 0);
        assert_eq!(Layout::Stream.wire_id(), 2);
        assert!(!Layout::Stream.supports_random_entry_lookup());
    }
}
