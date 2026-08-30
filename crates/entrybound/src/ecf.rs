//! Native Entrybound Container Format declarations.
//!
//! ECF remains an encoding of EAM. The Index is explicitly absent from the
//! authoritative section set so a reader can discard and rebuild it.

use crate::diagnostics::Result;
use crate::eam::{ArchiveRole, Layout};
use crate::eam::{ChunkGroup, Dictionary, Digest, TransformPlan};

mod container;
mod records;

pub use container::{
    EncodedArchive, IndexStatus, OpenedArchive, VerificationReport, WriteOptions, encode, open,
    open_with_limits, open_with_policy, verify, verify_with_limits, verify_with_policy,
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
pub(crate) const SUPPORTED_INCOMPAT_FEATURES: u64 = FEATURE_CROSS_FILE_COMPRESSION_V1;

/// Versioned namespace for the experimental encoding.
pub const FORMAT_NAMESPACE: &str = "ecf/bootstrap-v1";

pub(crate) fn encoded_transform_plan_len(plan: &TransformPlan) -> Result<u64> {
    u64::try_from(records::encode_transform_plans(std::slice::from_ref(plan))?.len()).map_err(
        |_| {
            crate::diagnostics::Diagnostic::new(
                crate::diagnostics::OutcomeClass::PolicyRefused,
                crate::diagnostics::ReasonCode::ResourceLimit,
                "encoded TransformPlan length exceeds u64",
            )
        },
    )
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
    use super::{BootstrapCapabilities, MAGIC};
    use crate::eam::{ArchiveRole, Layout};

    #[test]
    fn bootstrap_capabilities_are_complete_and_indexed() {
        let capabilities = BootstrapCapabilities::default();
        assert_eq!(capabilities.layout, Layout::Indexed);
        assert_eq!(capabilities.role, ArchiveRole::Complete);
        assert!(capabilities.budget_declared);
        assert_eq!(MAGIC, [0x8e, b'E', b'B', b'1', 13, 10, 26, 10]);
    }
}
