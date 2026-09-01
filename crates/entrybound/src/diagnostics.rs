//! Stable diagnostic classes and reason codes.

use std::error::Error;
use std::fmt;

/// The architecture-defined top-level outcome class.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OutcomeClass {
    /// Conforming and verified.
    Ok,
    /// Well-formed but requiring an unsupported capability.
    Unsupported,
    /// A well-formed prefix whose declared bytes are missing.
    Truncated,
    /// Structurally damaged or failing an integrity check.
    Corrupt,
    /// Violating a normative semantic or canonical rule.
    Nonconforming,
    /// Conforming but outside caller-owned policy.
    PolicyRefused,
}

impl OutcomeClass {
    /// Returns the stable machine-readable class name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "OK",
            Self::Unsupported => "UNSUPPORTED",
            Self::Truncated => "TRUNCATED",
            Self::Corrupt => "CORRUPT",
            Self::Nonconforming => "NONCONFORMING",
            Self::PolicyRefused => "POLICY_REFUSED",
        }
    }
}

/// Stable reason codes used by the initial native conformance corpus.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReasonCode {
    BadMagic,
    UnsupportedVersion,
    UnsupportedRequiredFeature,
    TruncatedFooter,
    TruncatedStream,
    IncorrectTotalLength,
    FooterBindingMismatch,
    SectionStructure,
    SectionDigestMismatch,
    IndexAbsentRebuilt,
    IndexInvalidRebuilt,
    DuplicateSemanticDeclaration,
    NoncanonicalEncoding,
    StreamItemOrdering,
    StreamWindowExceeded,
    StreamForwardReference,
    ResourceLimit,
    DuplicateLogicalPath,
    DotComponent,
    DotDotComponent,
    InvalidPathComponent,
    MissingAncestor,
    FileAsAncestor,
    DirectoryHasContent,
    FileMissingContent,
    UnknownContentObject,
    UnknownChunk,
    UnknownDictionary,
    DictionaryDigestMismatch,
    UnsupportedDictionaryFormat,
    InvalidGroupReference,
    InvalidGroupOrdering,
    LookbackViolation,
    AccessCostMismatch,
    PrerequisiteChunkCorrupt,
    UnknownTransformPlan,
    UnknownTransform,
    InvalidTransformParameters,
    TransformFailed,
    TransformedLengthMismatch,
    UnknownReconstructionData,
    ReconstructionDataDigestMismatch,
    UnsupportedReconstructionFormat,
    ReconstructionFailed,
    ReconstructedLengthMismatch,
    ReconstructedDigestMismatch,
    UnknownReconstructionRegion,
    OverlappingReconstructionRegion,
    InvalidReconstructionRegion,
    InvalidRegionAccess,
    MalformedReconstructionPayload,
    RegionMemberDigestMismatch,
    UnknownCodec,
    InvalidCodecParameters,
    CompressionFailed,
    DecompressionFailed,
    DecompressedLengthMismatch,
    ChunkDigestMismatch,
    ChunkIdentityCollision,
    ContentDigestMismatch,
    ChunkRootMismatch,
    EntryIdentityMismatch,
    EntryAuxMismatch,
    LaiMismatch,
    PcrMismatch,
    AuxMismatch,
    CryptoSuiteUnsupported,
    CryptoLayoutUnsupported,
    CryptoNoMatchingRecipient,
    CryptoRecipientStanzaInvalid,
    CryptoRecipientPolicyInvalid,
    CryptoEnvelopeAuthFailed,
    CryptoKeyCommitmentFailed,
    CryptoPasswordKdfPolicyRefused,
    CryptoAeadAuthFailed,
    CryptoSegmentStructureInvalid,
    CryptoPaddingInvalid,
    CryptoPrivateObjectInvalid,
    CryptoBoundaryModeUnsupported,
    CryptoResourcePolicyRefused,
    SignatureAbsent,
    SignatureUnsupported,
    SignatureInvalid,
    SignatureStaleContent,
    SignatureStalePhysical,
    SignatureStaleAddressing,
    SignatureTimestampInvalid,
    SignatureTimestampUnsupported,
    LegacyFormatUnsupported,
    LegacyResourcePolicyRefused,
    ZipStructureInvalid,
    ZipUnsupportedFeature,
    ZipConflictDivergence,
    ZipConflictIrreconcilable,
    ZipCrcMismatch,
    ZipSizeMismatch,
    ZipUnsafePath,
    ZipAmbiguousName,
    ZipOverlappingExtent,
    TarStructureInvalid,
    TarUnsupportedFeature,
    TarConflictDivergence,
    TarConflictIrreconcilable,
    TarChecksumMismatch,
    TarUnsafePath,
    WrapperStructureInvalid,
    WrapperIntegrityMismatch,
    WrapperDictionaryUnsupported,
    UnsupportedEntryKind,
    SourceUnstable,
    InputNotDirectory,
    ExtractionCollision,
    ExtractionContainmentUnavailable,
    CommandUsage,
    CommandNotImplemented,
    Io,
}

impl ReasonCode {
    /// Returns the version-stable textual reason code.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::BadMagic => "EB_ECF_BAD_MAGIC",
            Self::UnsupportedVersion => "EB_ECF_UNSUPPORTED_VERSION",
            Self::UnsupportedRequiredFeature => "EB_ECF_UNSUPPORTED_REQUIRED_FEATURE",
            Self::TruncatedFooter => "EB_ECF_TRUNCATED_FOOTER",
            Self::TruncatedStream => "EB_ECF_TRUNCATED_STREAM",
            Self::IncorrectTotalLength => "EB_ECF_INCORRECT_TOTAL_LENGTH",
            Self::FooterBindingMismatch => "EB_ECF_FOOTER_BINDING_MISMATCH",
            Self::SectionStructure => "EB_ECF_SECTION_STRUCTURE",
            Self::SectionDigestMismatch => "EB_ECF_SECTION_DIGEST_MISMATCH",
            Self::IndexAbsentRebuilt => "EB_ECF_INDEX_ABSENT_REBUILT",
            Self::IndexInvalidRebuilt => "EB_ECF_INDEX_INVALID_REBUILT",
            Self::DuplicateSemanticDeclaration => "EB_ECF_DUPLICATE_SEMANTIC_DECLARATION",
            Self::NoncanonicalEncoding => "EB_ECF_NONCANONICAL_ENCODING",
            Self::StreamItemOrdering => "EB_ECF_STREAM_ITEM_ORDERING",
            Self::StreamWindowExceeded => "EB_ECF_STREAM_WINDOW_EXCEEDED",
            Self::StreamForwardReference => "EB_ECF_STREAM_FORWARD_REFERENCE",
            Self::ResourceLimit => "EB_RESOURCE_LIMIT",
            Self::DuplicateLogicalPath => "EB_EAM_DUPLICATE_LOGICAL_PATH",
            Self::DotComponent => "EB_EAM_DOT_COMPONENT",
            Self::DotDotComponent => "EB_EAM_DOT_DOT_COMPONENT",
            Self::InvalidPathComponent => "EB_EAM_INVALID_PATH_COMPONENT",
            Self::MissingAncestor => "EB_EAM_MISSING_ANCESTOR",
            Self::FileAsAncestor => "EB_EAM_FILE_AS_ANCESTOR",
            Self::DirectoryHasContent => "EB_EAM_DIRECTORY_HAS_CONTENT",
            Self::FileMissingContent => "EB_EAM_FILE_MISSING_CONTENT",
            Self::UnknownContentObject => "EB_EAM_UNKNOWN_CONTENT_OBJECT",
            Self::UnknownChunk => "EB_EAM_UNKNOWN_CHUNK",
            Self::UnknownDictionary => "EB_EAM_UNKNOWN_DICTIONARY",
            Self::DictionaryDigestMismatch => "EB_INTEGRITY_DICTIONARY_DIGEST_MISMATCH",
            Self::UnsupportedDictionaryFormat => "EB_CODEC_UNSUPPORTED_DICTIONARY_FORMAT",
            Self::InvalidGroupReference => "EB_EAM_INVALID_CHUNK_GROUP_REFERENCE",
            Self::InvalidGroupOrdering => "EB_ECF_INVALID_CHUNK_GROUP_ORDERING",
            Self::LookbackViolation => "EB_CODEC_LOOKBACK_VIOLATION",
            Self::AccessCostMismatch => "EB_ECF_ACCESS_COST_MISMATCH",
            Self::PrerequisiteChunkCorrupt => "EB_INTEGRITY_PREREQUISITE_CHUNK_CORRUPT",
            Self::UnknownTransformPlan => "EB_EAM_UNKNOWN_TRANSFORM_PLAN",
            Self::UnknownTransform => "EB_TRANSFORM_UNKNOWN",
            Self::InvalidTransformParameters => "EB_TRANSFORM_INVALID_PARAMETERS",
            Self::TransformFailed => "EB_TRANSFORM_FAILED",
            Self::TransformedLengthMismatch => "EB_TRANSFORM_LENGTH_MISMATCH",
            Self::UnknownReconstructionData => "EB_RECONSTRUCTION_DATA_UNKNOWN",
            Self::ReconstructionDataDigestMismatch => {
                "EB_INTEGRITY_RECONSTRUCTION_DATA_DIGEST_MISMATCH"
            }
            Self::UnsupportedReconstructionFormat => "EB_RECONSTRUCTION_FORMAT_UNSUPPORTED",
            Self::ReconstructionFailed => "EB_RECONSTRUCTION_FAILED",
            Self::ReconstructedLengthMismatch => "EB_RECONSTRUCTION_LENGTH_MISMATCH",
            Self::ReconstructedDigestMismatch => "EB_RECONSTRUCTION_DIGEST_MISMATCH",
            Self::UnknownReconstructionRegion => "EB_RECONSTRUCTION_REGION_UNKNOWN",
            Self::OverlappingReconstructionRegion => "EB_RECONSTRUCTION_REGION_OVERLAP",
            Self::InvalidReconstructionRegion => "EB_RECONSTRUCTION_REGION_INVALID",
            Self::InvalidRegionAccess => "EB_RECONSTRUCTION_REGION_ACCESS_INVALID",
            Self::MalformedReconstructionPayload => "EB_RECONSTRUCTION_PAYLOAD_MALFORMED",
            Self::RegionMemberDigestMismatch => {
                "EB_INTEGRITY_RECONSTRUCTION_REGION_MEMBER_DIGEST_MISMATCH"
            }
            Self::UnknownCodec => "EB_CODEC_UNKNOWN",
            Self::InvalidCodecParameters => "EB_CODEC_INVALID_PARAMETERS",
            Self::CompressionFailed => "EB_CODEC_COMPRESSION_FAILED",
            Self::DecompressionFailed => "EB_CODEC_DECOMPRESSION_FAILED",
            Self::DecompressedLengthMismatch => "EB_CODEC_DECOMPRESSED_LENGTH_MISMATCH",
            Self::ChunkDigestMismatch => "EB_INTEGRITY_CHUNK_DIGEST_MISMATCH",
            Self::ChunkIdentityCollision => "EB_INTEGRITY_CHUNK_IDENTITY_COLLISION",
            Self::ContentDigestMismatch => "EB_INTEGRITY_CONTENT_DIGEST_MISMATCH",
            Self::ChunkRootMismatch => "EB_INTEGRITY_CHUNK_ROOT_MISMATCH",
            Self::EntryIdentityMismatch => "EB_INTEGRITY_ENTRY_IDENTITY_MISMATCH",
            Self::EntryAuxMismatch => "EB_INTEGRITY_ENTRY_AUX_MISMATCH",
            Self::LaiMismatch => "EB_INTEGRITY_LAI_MISMATCH",
            Self::PcrMismatch => "EB_INTEGRITY_PCR_MISMATCH",
            Self::AuxMismatch => "EB_INTEGRITY_AUX_MISMATCH",
            Self::CryptoSuiteUnsupported => "EB_CRYPTO_SUITE_UNSUPPORTED",
            Self::CryptoLayoutUnsupported => "EB_CRYPTO_LAYOUT_UNSUPPORTED",
            Self::CryptoNoMatchingRecipient => "EB_CRYPTO_NO_MATCHING_RECIPIENT",
            Self::CryptoRecipientStanzaInvalid => "EB_CRYPTO_RECIPIENT_STANZA_INVALID",
            Self::CryptoRecipientPolicyInvalid => "EB_CRYPTO_RECIPIENT_POLICY_INVALID",
            Self::CryptoEnvelopeAuthFailed => "EB_CRYPTO_ENVELOPE_AUTH_FAILED",
            Self::CryptoKeyCommitmentFailed => "EB_CRYPTO_KEY_COMMITMENT_FAILED",
            Self::CryptoPasswordKdfPolicyRefused => "EB_CRYPTO_PASSWORD_KDF_POLICY_REFUSED",
            Self::CryptoAeadAuthFailed => "EB_CRYPTO_AEAD_AUTH_FAILED",
            Self::CryptoSegmentStructureInvalid => "EB_CRYPTO_SEGMENT_STRUCTURE_INVALID",
            Self::CryptoPaddingInvalid => "EB_CRYPTO_PADDING_INVALID",
            Self::CryptoPrivateObjectInvalid => "EB_CRYPTO_PRIVATE_OBJECT_INVALID",
            Self::CryptoBoundaryModeUnsupported => "EB_CRYPTO_BOUNDARY_MODE_UNSUPPORTED",
            Self::CryptoResourcePolicyRefused => "EB_CRYPTO_RESOURCE_POLICY_REFUSED",
            Self::SignatureAbsent => "EB_SIGNATURE_ABSENT",
            Self::SignatureUnsupported => "EB_SIGNATURE_UNSUPPORTED",
            Self::SignatureInvalid => "EB_SIGNATURE_INVALID",
            Self::SignatureStaleContent => "EB_SIGNATURE_STALE_CONTENT",
            Self::SignatureStalePhysical => "EB_SIGNATURE_STALE_PHYSICAL",
            Self::SignatureStaleAddressing => "EB_SIGNATURE_STALE_ADDRESSING",
            Self::SignatureTimestampInvalid => "EB_SIGNATURE_TIMESTAMP_INVALID",
            Self::SignatureTimestampUnsupported => "EB_SIGNATURE_TIMESTAMP_UNSUPPORTED",
            Self::LegacyFormatUnsupported => "EB_LEGACY_FORMAT_UNSUPPORTED",
            Self::LegacyResourcePolicyRefused => "EB_LEGACY_RESOURCE_POLICY_REFUSED",
            Self::ZipStructureInvalid => "EB_ZIP_STRUCTURE_INVALID",
            Self::ZipUnsupportedFeature => "EB_ZIP_UNSUPPORTED_FEATURE",
            Self::ZipConflictDivergence => "EB_ZIP_CONFLICT_DIVERGENCE",
            Self::ZipConflictIrreconcilable => "EB_ZIP_CONFLICT_IRRECONCILABLE",
            Self::ZipCrcMismatch => "EB_ZIP_CRC_MISMATCH",
            Self::ZipSizeMismatch => "EB_ZIP_SIZE_MISMATCH",
            Self::ZipUnsafePath => "EB_ZIP_UNSAFE_PATH",
            Self::ZipAmbiguousName => "EB_ZIP_AMBIGUOUS_NAME",
            Self::ZipOverlappingExtent => "EB_ZIP_OVERLAPPING_EXTENT",
            Self::TarStructureInvalid => "EB_TAR_STRUCTURE_INVALID",
            Self::TarUnsupportedFeature => "EB_TAR_UNSUPPORTED_FEATURE",
            Self::TarConflictDivergence => "EB_TAR_CONFLICT_DIVERGENCE",
            Self::TarConflictIrreconcilable => "EB_TAR_CONFLICT_IRRECONCILABLE",
            Self::TarChecksumMismatch => "EB_TAR_CHECKSUM_MISMATCH",
            Self::TarUnsafePath => "EB_TAR_UNSAFE_PATH",
            Self::WrapperStructureInvalid => "EB_WRAPPER_STRUCTURE_INVALID",
            Self::WrapperIntegrityMismatch => "EB_WRAPPER_INTEGRITY_MISMATCH",
            Self::WrapperDictionaryUnsupported => "EB_WRAPPER_DICTIONARY_UNSUPPORTED",
            Self::UnsupportedEntryKind => "EB_INPUT_UNSUPPORTED_ENTRY_KIND",
            Self::SourceUnstable => "EB_INPUT_SOURCE_UNSTABLE",
            Self::InputNotDirectory => "EB_INPUT_NOT_DIRECTORY",
            Self::ExtractionCollision => "EB_EXTRACT_COLLISION",
            Self::ExtractionContainmentUnavailable => "EB_EXTRACT_CONTAINMENT_UNAVAILABLE",
            Self::CommandUsage => "EB_CLI_USAGE",
            Self::CommandNotImplemented => "EB_CLI_NOT_IMPLEMENTED",
            Self::Io => "EB_IO",
        }
    }
}

/// A typed Entrybound failure with stable classification.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    class: OutcomeClass,
    code: ReasonCode,
    detail: String,
}

impl Diagnostic {
    /// Constructs a diagnostic without erasing its architecture-level class.
    #[must_use]
    pub fn new(class: OutcomeClass, code: ReasonCode, detail: impl Into<String>) -> Self {
        Self {
            class,
            code,
            detail: detail.into(),
        }
    }

    /// Returns the top-level outcome class.
    #[must_use]
    pub const fn class(&self) -> OutcomeClass {
        self.class
    }

    /// Returns the stable reason code.
    #[must_use]
    pub const fn code(&self) -> ReasonCode {
        self.code
    }

    /// Returns the human-readable detail record.
    #[must_use]
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} {}: {}",
            self.class.as_str(),
            self.code.as_str(),
            self.detail
        )
    }
}

impl Error for Diagnostic {}

/// The library-wide result type.
pub type Result<T> = std::result::Result<T, Diagnostic>;

#[cfg(test)]
mod tests {
    use super::{Diagnostic, OutcomeClass, ReasonCode};

    #[test]
    fn reason_codes_are_stable_and_visible() {
        let error = Diagnostic::new(
            OutcomeClass::Nonconforming,
            ReasonCode::DuplicateLogicalPath,
            "a/b",
        );
        assert_eq!(error.code().as_str(), "EB_EAM_DUPLICATE_LOGICAL_PATH");
        assert!(error.to_string().contains("NONCONFORMING"));
    }
}
