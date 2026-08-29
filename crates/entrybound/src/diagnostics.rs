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
    IncorrectTotalLength,
    FooterBindingMismatch,
    SectionStructure,
    SectionDigestMismatch,
    DuplicateSemanticDeclaration,
    NoncanonicalEncoding,
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
    UnknownTransformPlan,
    ChunkDigestMismatch,
    ContentDigestMismatch,
    ChunkRootMismatch,
    EntryIdentityMismatch,
    EntryAuxMismatch,
    LaiMismatch,
    PcrMismatch,
    AuxMismatch,
    UnsupportedEntryKind,
    SourceUnstable,
    InputNotDirectory,
    ExtractionCollision,
    ExtractionContainmentUnavailable,
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
            Self::IncorrectTotalLength => "EB_ECF_INCORRECT_TOTAL_LENGTH",
            Self::FooterBindingMismatch => "EB_ECF_FOOTER_BINDING_MISMATCH",
            Self::SectionStructure => "EB_ECF_SECTION_STRUCTURE",
            Self::SectionDigestMismatch => "EB_ECF_SECTION_DIGEST_MISMATCH",
            Self::DuplicateSemanticDeclaration => "EB_ECF_DUPLICATE_SEMANTIC_DECLARATION",
            Self::NoncanonicalEncoding => "EB_ECF_NONCANONICAL_ENCODING",
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
            Self::UnknownTransformPlan => "EB_EAM_UNKNOWN_TRANSFORM_PLAN",
            Self::ChunkDigestMismatch => "EB_INTEGRITY_CHUNK_DIGEST_MISMATCH",
            Self::ContentDigestMismatch => "EB_INTEGRITY_CONTENT_DIGEST_MISMATCH",
            Self::ChunkRootMismatch => "EB_INTEGRITY_CHUNK_ROOT_MISMATCH",
            Self::EntryIdentityMismatch => "EB_INTEGRITY_ENTRY_IDENTITY_MISMATCH",
            Self::EntryAuxMismatch => "EB_INTEGRITY_ENTRY_AUX_MISMATCH",
            Self::LaiMismatch => "EB_INTEGRITY_LAI_MISMATCH",
            Self::PcrMismatch => "EB_INTEGRITY_PCR_MISMATCH",
            Self::AuxMismatch => "EB_INTEGRITY_AUX_MISMATCH",
            Self::UnsupportedEntryKind => "EB_INPUT_UNSUPPORTED_ENTRY_KIND",
            Self::SourceUnstable => "EB_INPUT_SOURCE_UNSTABLE",
            Self::InputNotDirectory => "EB_INPUT_NOT_DIRECTORY",
            Self::ExtractionCollision => "EB_EXTRACT_COLLISION",
            Self::ExtractionContainmentUnavailable => "EB_EXTRACT_CONTAINMENT_UNAVAILABLE",
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
