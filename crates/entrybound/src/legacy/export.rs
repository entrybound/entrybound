//! Deterministic EAM-to-legacy export with mandatory representability preflight.
//!
//! Export adapters consume only a validated, verified EAM. They never inspect
//! ECF sections, source codec choices, Chunk physical order, or host filesystem
//! metadata.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::io::Write as _;
use std::str::FromStr;

use bzip2::write::BzEncoder;
use crc32fast::Hasher as Crc32;
use flate2::{Compression, write::DeflateEncoder};
use lzma_rust2::{XzOptions, XzWriter};

use super::import::{LegacyImportPolicy, LegacySourceFormat, import_strict};
use super::stream::{self, TransportFormat, WrapperImportPolicy};
use crate::diagnostics::{Diagnostic, OutcomeClass, ReasonCode, Result};
use crate::eam::{
    Archive, ContentRef, Digest, Entry, EntryData, LogicalPath, MetadataName, MetadataValue,
    Timestamp, TimestampPrecision,
};
use crate::identity::sha256_exact;

pub const EXPORTER_ID: &str = "entrybound/legacy-export-v1";
pub const RECEIPT_FORMAT: &str = "entrybound/export-receipt-v1";
pub const RECEIPT_V2_FORMAT: &str = "entrybound/export-receipt-v2";
pub const ZIP_PROFILE: &str = "zip/portable-v1";
pub const TAR_PROFILE: &str = "tar/pax-v1";
pub const TAR_GZIP_PROFILE: &str = "tar.gz/pax-v1";
pub const TAR_ZSTD_PROFILE: &str = "tar.zst/pax-v1";
pub const TAR_XZ_PROFILE: &str = "tar.xz/pax-v1";
pub const TAR_BZIP2_PROFILE: &str = "tar.bz2/pax-v1";

const ZIP_UTF8_FLAG: u16 = 1 << 11;
const ZIP_STORE: u16 = 0;
const ZIP_DEFLATE: u16 = 8;
const ZIP_VERSION_20: u16 = 20;
const ZIP_VERSION_45: u16 = 45;
const ZIP_U32_SENTINEL: u64 = u32::MAX as u64;
const ZIP_U16_SENTINEL: u64 = u16::MAX as u64;
const TAR_BLOCK: usize = 512;
const TAR_OCTAL_SIZE_MAX: u64 = 0o77_777_777_777;
const TAR_OCTAL_TIME_MAX: u64 = 0o77_777_777_777;

/// The frozen export target and behavior contract.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ExportTarget {
    ZipPortableV1,
    TarPaxV1,
    TarGzipPaxV1,
    TarZstandardPaxV1,
    TarXzPaxV1,
    TarBzip2PaxV1,
}

impl ExportTarget {
    #[must_use]
    pub const fn profile_id(self) -> &'static str {
        match self {
            Self::ZipPortableV1 => ZIP_PROFILE,
            Self::TarPaxV1 => TAR_PROFILE,
            Self::TarGzipPaxV1 => TAR_GZIP_PROFILE,
            Self::TarZstandardPaxV1 => TAR_ZSTD_PROFILE,
            Self::TarXzPaxV1 => TAR_XZ_PROFILE,
            Self::TarBzip2PaxV1 => TAR_BZIP2_PROFILE,
        }
    }

    #[must_use]
    pub const fn format(self) -> &'static str {
        match self {
            Self::ZipPortableV1 => "zip",
            Self::TarPaxV1 => "tar",
            Self::TarGzipPaxV1 => "tar.gz",
            Self::TarZstandardPaxV1 => "tar.zst",
            Self::TarXzPaxV1 => "tar.xz",
            Self::TarBzip2PaxV1 => "tar.bz2",
        }
    }

    /// The semantic target whose representability rules govern this artifact.
    #[must_use]
    pub const fn semantic_target(self) -> Self {
        match self {
            Self::ZipPortableV1 => Self::ZipPortableV1,
            Self::TarPaxV1
            | Self::TarGzipPaxV1
            | Self::TarZstandardPaxV1
            | Self::TarXzPaxV1
            | Self::TarBzip2PaxV1 => Self::TarPaxV1,
        }
    }

    #[must_use]
    pub const fn transport_profile(self) -> Option<&'static str> {
        match self {
            Self::ZipPortableV1 | Self::TarPaxV1 => None,
            Self::TarGzipPaxV1 => Some("gzip-v1"),
            Self::TarZstandardPaxV1 => Some("zstd-v1"),
            Self::TarXzPaxV1 => Some("xz-v1"),
            Self::TarBzip2PaxV1 => Some("bzip2-v1"),
        }
    }

    #[must_use]
    pub const fn extension(self) -> &'static str {
        match self {
            Self::ZipPortableV1 => "zip",
            Self::TarPaxV1 => "tar",
            Self::TarGzipPaxV1 => "tar.gz",
            Self::TarZstandardPaxV1 => "tar.zst",
            Self::TarXzPaxV1 => "tar.xz",
            Self::TarBzip2PaxV1 => "tar.bz2",
        }
    }

    const fn import_format(self) -> LegacySourceFormat {
        match self {
            Self::ZipPortableV1 => LegacySourceFormat::Zip,
            Self::TarPaxV1 => LegacySourceFormat::Tar,
            Self::TarGzipPaxV1 => LegacySourceFormat::TarGzip,
            Self::TarZstandardPaxV1 => LegacySourceFormat::TarZstandard,
            Self::TarXzPaxV1 => LegacySourceFormat::TarXz,
            Self::TarBzip2PaxV1 => LegacySourceFormat::TarBzip2,
        }
    }
}

impl FromStr for ExportTarget {
    type Err = Diagnostic;

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "zip" | ZIP_PROFILE => Ok(Self::ZipPortableV1),
            "tar" | TAR_PROFILE => Ok(Self::TarPaxV1),
            "tar.gz" | TAR_GZIP_PROFILE => Ok(Self::TarGzipPaxV1),
            "tar.zst" | TAR_ZSTD_PROFILE => Ok(Self::TarZstandardPaxV1),
            "tar.xz" | TAR_XZ_PROFILE => Ok(Self::TarXzPaxV1),
            "tar.bz2" | TAR_BZIP2_PROFILE => Ok(Self::TarBzip2PaxV1),
            _ => Err(Diagnostic::new(
                OutcomeClass::Unsupported,
                ReasonCode::LegacyExportTargetInvalid,
                format!("unsupported export target/profile '{value}'"),
            )),
        }
    }
}

/// Frozen target profile identifier. It is never an unversioned runtime alias.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ExportProfileId(String);

impl ExportProfileId {
    pub fn parse(value: &str) -> Result<Self> {
        let target = value.parse::<ExportTarget>()?;
        if value != target.profile_id() {
            return Err(Diagnostic::new(
                OutcomeClass::Nonconforming,
                ReasonCode::LegacyExportTargetInvalid,
                "--target-profile requires an exact versioned profile ID",
            ));
        }
        Ok(Self(value.to_owned()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn target(&self) -> Result<ExportTarget> {
        self.0.parse()
    }
}

/// The mandatory, typed representability result.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ExportOutcome {
    Lossless,
    Lossy,
    Refused,
}

impl ExportOutcome {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Lossless => "LOSSLESS",
            Self::Lossy => "LOSSY",
            Self::Refused => "REFUSED",
        }
    }
}

/// Stable machine-readable representability issue class.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ExportIssueCategory {
    PathUnrepresentable,
    PathCollision,
    EntryKindUnsupported,
    TimestampRangeLoss,
    TimestampPrecisionLoss,
    MetadataUnsupported,
    TargetLimitExceeded,
}

impl ExportIssueCategory {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PathUnrepresentable => "PATH_UNREPRESENTABLE",
            Self::PathCollision => "PATH_COLLISION",
            Self::EntryKindUnsupported => "ENTRY_KIND_UNSUPPORTED",
            Self::TimestampRangeLoss => "TIMESTAMP_RANGE_LOSS",
            Self::TimestampPrecisionLoss => "TIMESTAMP_PRECISION_LOSS",
            Self::MetadataUnsupported => "METADATA_UNSUPPORTED",
            Self::TargetLimitExceeded => "TARGET_LIMIT_EXCEEDED",
        }
    }
}

/// How one target-relevant semantic claim is handled.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ExportDisposition {
    Preserved,
    Degraded,
    Omitted,
    Refused,
}

impl ExportDisposition {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Preserved => "preserved",
            Self::Degraded => "degraded",
            Self::Omitted => "omitted",
            Self::Refused => "refused",
        }
    }
}

/// One typed preflight finding.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ExportIssue {
    pub category: ExportIssueCategory,
    pub entry: Option<LogicalPath>,
    pub semantic_field: String,
    pub source_value: String,
    pub target_capability: String,
    pub disposition: ExportDisposition,
    pub reason: String,
}

/// Entrybound-only evidence/security state intentionally outside the target's
/// semantic representability classification.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct AuxiliaryEvidenceSummary {
    pub fidelity_issue_count: u64,
    pub conversion_provenance: bool,
    pub exact_preserved_source: bool,
    pub reconstruction_audit_count: u64,
}

/// Authenticated source security state supplied by the archive-opening layer.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ExportSourceSecurity {
    pub encrypted: bool,
    pub embedded_signature_count: u64,
    pub detached_signature_count: u64,
    pub signatures_valid: u64,
    pub signatures_invalid: u64,
    pub signatures_stale: u64,
}

/// Complete preflight result produced before any target is created.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExportAnalysis {
    pub target: ExportTarget,
    pub profile: ExportProfileId,
    pub outcome: ExportOutcome,
    pub issues: Box<[ExportIssue]>,
    pub entry_count: u64,
    pub total_logical_bytes: u64,
    pub planned_target_bytes: Option<u64>,
    pub planned_target_digest: Option<Digest>,
    pub strict_reimport_validated: bool,
    pub reimport_lai: Option<Digest>,
}

/// Receipt-v2 transport details for a compressed-tar composition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WrappedTargetReceipt {
    pub semantic_profile: String,
    pub transport_profile: String,
    pub inner_tar_byte_length: u64,
    pub inner_tar_sha256: Digest,
}

/// Canonical receipt for one accepted export.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExportReceipt {
    pub source_lai: Digest,
    pub source_aux: Digest,
    pub source_pcr: Digest,
    pub source_security: ExportSourceSecurity,
    pub auxiliary: AuxiliaryEvidenceSummary,
    pub target_format: String,
    pub target_profile: String,
    pub outcome: ExportOutcome,
    pub issues: Box<[ExportIssue]>,
    pub entry_count: u64,
    pub total_logical_bytes: u64,
    pub target_byte_length: u64,
    pub target_sha256: Digest,
    pub deterministic: bool,
    /// Whether the generated target passed its strict Entrybound re-import.
    pub strict_reimport_validated: bool,
    /// Logical identity obtained through strict re-import of the generated target.
    pub reimport_lai: Option<Digest>,
    /// Present only for wrapped tar targets. Its presence selects receipt v2;
    /// bare ZIP/tar receipts retain their frozen v1 bytes.
    pub wrapped_target: Option<WrappedTargetReceipt>,
}

/// Fully planned target bytes. Constructing this value performs all compression
/// and target framing before the caller is allowed to create an output file.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreparedExport {
    pub analysis: ExportAnalysis,
    bytes: Option<Vec<u8>>,
    source_lai: Digest,
    source_aux: Digest,
    source_pcr: Digest,
    source_security: ExportSourceSecurity,
    auxiliary: AuxiliaryEvidenceSummary,
    wrapped_target: Option<WrappedTargetReceipt>,
}

/// Accepted export bytes and their final receipt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExportArtifact {
    pub bytes: Vec<u8>,
    pub receipt: ExportReceipt,
}

impl PreparedExport {
    /// Accepts the analyzed result. LOSSY requires explicit approval and
    /// REFUSED can never be accepted.
    pub fn accept(self, allow_lossy: bool) -> Result<ExportArtifact> {
        match self.analysis.outcome {
            ExportOutcome::Lossless => {}
            ExportOutcome::Lossy if allow_lossy => {}
            ExportOutcome::Lossy => {
                return Err(Diagnostic::new(
                    OutcomeClass::PolicyRefused,
                    ReasonCode::LegacyExportLossyApprovalRequired,
                    "legacy export is LOSSY; repeat with explicit lossy approval",
                ));
            }
            ExportOutcome::Refused => {
                return Err(Diagnostic::new(
                    OutcomeClass::Unsupported,
                    ReasonCode::LegacyExportRefused,
                    first_issue_detail(&self.analysis),
                ));
            }
        }
        let bytes = self.bytes.ok_or_else(|| {
            Diagnostic::new(
                OutcomeClass::Unsupported,
                ReasonCode::LegacyExportRefused,
                "refused export has no target representation",
            )
        })?;
        let target_sha256 = sha256_exact(&bytes);
        let target_byte_length =
            u64::try_from(bytes.len()).map_err(|_| target_limit("target length exceeds u64"))?;
        let receipt = ExportReceipt {
            source_lai: self.source_lai,
            source_aux: self.source_aux,
            source_pcr: self.source_pcr,
            source_security: self.source_security,
            auxiliary: self.auxiliary,
            target_format: self.analysis.target.format().to_owned(),
            target_profile: self.analysis.profile.as_str().to_owned(),
            outcome: self.analysis.outcome,
            issues: self.analysis.issues.clone(),
            entry_count: self.analysis.entry_count,
            total_logical_bytes: self.analysis.total_logical_bytes,
            target_byte_length,
            target_sha256,
            deterministic: true,
            strict_reimport_validated: self.analysis.strict_reimport_validated,
            reimport_lai: self.analysis.reimport_lai,
            wrapped_target: self.wrapped_target,
        };
        Ok(ExportArtifact { bytes, receipt })
    }
}

/// Performs validation, target representability analysis, deterministic
/// compression, and complete target framing without writing target bytes.
pub fn prepare_export(
    archive: &Archive,
    target: ExportTarget,
    source_security: ExportSourceSecurity,
) -> Result<PreparedExport> {
    archive.validate()?;
    let entry_count = u64::try_from(archive.entry_set.len())
        .map_err(|_| target_limit("entry count exceeds u64"))?;
    let total_logical_bytes = archive.total_logical_size()?;
    let semantic_target = target.semantic_target();
    let mut issues = analyze_entries(archive, semantic_target)?;
    issues.sort();
    issues.dedup();
    let mut outcome = classify(&issues);
    let (bytes, wrapped_target) = if outcome == ExportOutcome::Refused {
        (None, None)
    } else {
        let (bytes, wrapped) = encode_target(archive, target, &mut issues)?;
        (Some(bytes), wrapped)
    };
    issues.sort();
    issues.dedup();
    outcome = classify(&issues);
    let (planned_target_bytes, planned_target_digest) =
        bytes.as_ref().map_or((None, None), |value| {
            (u64::try_from(value.len()).ok(), Some(sha256_exact(value)))
        });
    let reimport_lai = bytes
        .as_deref()
        .map(|value| validate_strict_reimport(archive, target, value, outcome, &issues))
        .transpose()?;
    let analysis = ExportAnalysis {
        target,
        profile: ExportProfileId(target.profile_id().to_owned()),
        outcome,
        issues: issues.into_boxed_slice(),
        entry_count,
        total_logical_bytes,
        planned_target_bytes,
        planned_target_digest,
        strict_reimport_validated: reimport_lai.is_some(),
        reimport_lai,
    };
    Ok(PreparedExport {
        analysis,
        bytes,
        source_lai: archive.descriptor.lai,
        source_aux: archive.descriptor.aux,
        source_pcr: archive.descriptor.pcr,
        source_security,
        auxiliary: AuxiliaryEvidenceSummary {
            fidelity_issue_count: u64::try_from(
                archive.fidelity.unavailable.len() + archive.fidelity.degraded.len(),
            )
            .unwrap_or(u64::MAX),
            conversion_provenance: archive.conversion.is_some(),
            exact_preserved_source: archive.preservation.is_some(),
            reconstruction_audit_count: u64::try_from(
                archive.content_store.reconstruction_audits.len()
                    + archive.content_store.reconstruction_fallbacks.len(),
            )
            .unwrap_or(u64::MAX),
        },
        wrapped_target,
    })
}

fn first_issue_detail(analysis: &ExportAnalysis) -> String {
    analysis.issues.first().map_or_else(
        || "target representation was refused".to_owned(),
        |issue| {
            format!(
                "{} {}: {}",
                issue.category.as_str(),
                issue
                    .entry
                    .as_ref()
                    .map_or_else(|| "archive".to_owned(), ToString::to_string),
                issue.reason
            )
        },
    )
}

fn classify(issues: &[ExportIssue]) -> ExportOutcome {
    if issues
        .iter()
        .any(|issue| issue.disposition == ExportDisposition::Refused)
    {
        ExportOutcome::Refused
    } else if issues.iter().any(|issue| {
        matches!(
            issue.disposition,
            ExportDisposition::Degraded | ExportDisposition::Omitted
        )
    }) {
        ExportOutcome::Lossy
    } else {
        ExportOutcome::Lossless
    }
}

fn analyze_entries(archive: &Archive, target: ExportTarget) -> Result<Vec<ExportIssue>> {
    let mut issues = Vec::new();
    let mut zip_equivalence = BTreeMap::<String, LogicalPath>::new();
    for entry in archive.entry_set.entries() {
        if matches!(entry.data(), EntryData::Symlink { .. }) {
            issues.push(issue(
                ExportIssueCategory::EntryKindUnsupported,
                Some(entry.path().clone()),
                "entry.kind",
                "symlink",
                target.profile_id(),
                ExportDisposition::Refused,
                "the frozen target profile has no native symlink representation",
            ));
        }
        let path = target_path(entry.path());
        analyze_path(
            entry.path(),
            &path,
            matches!(entry.data(), EntryData::Directory),
            target,
            &mut issues,
            &mut zip_equivalence,
        );
        for metadata in entry.metadata().items() {
            match (metadata.name(), metadata.value()) {
                (MetadataName::CoreExecutable, MetadataValue::Bool(_)) => {}
                (MetadataName::CoreMtime, MetadataValue::Timestamp(timestamp)) => {
                    analyze_mtime(entry.path(), *timestamp, target, &mut issues);
                }
                (
                    MetadataName::PosixMode
                    | MetadataName::PosixUid
                    | MetadataName::PosixGid
                    | MetadataName::PosixHardlinkGroup
                    | MetadataName::PosixXattrs
                    | MetadataName::PosixSparseMap,
                    _,
                ) => issues.push(issue(
                    ExportIssueCategory::MetadataUnsupported,
                    Some(entry.path().clone()),
                    metadata.name().as_str(),
                    format!("{:?}", metadata.value()),
                    target.profile_id(),
                    ExportDisposition::Omitted,
                    "the frozen v1 target profile exports safe logical bytes but omits this newer AUX semantic",
                )),
                _ => unreachable!("MetadataSet validates registry value types"),
            }
        }
        if entry.metadata().mtime().is_none() {
            issues.push(issue(
                ExportIssueCategory::MetadataUnsupported,
                Some(entry.path().clone()),
                "core.mtime",
                "absent",
                match target {
                    ExportTarget::ZipPortableV1 => "ZIP local headers require a DOS timestamp",
                    ExportTarget::TarPaxV1 => "POSIX tar headers require an mtime field",
                    _ => unreachable!("semantic target is normalized"),
                },
                ExportDisposition::Degraded,
                match target {
                    ExportTarget::ZipPortableV1 => {
                        "zip/portable-v1 emits the deterministic construction value 1980-01-01T00:00:00Z"
                    }
                    ExportTarget::TarPaxV1 => {
                        "tar/pax-v1 emits the deterministic construction value 0 seconds"
                    }
                    _ => unreachable!("semantic target is normalized"),
                },
            ));
        }
        if target == ExportTarget::TarPaxV1
            && matches!(entry.data(), EntryData::Directory)
            && !entry.metadata().executable()
        {
            issues.push(issue(
                ExportIssueCategory::MetadataUnsupported,
                Some(entry.path().clone()),
                "core.executable",
                "false",
                "tar/pax-v1 fixes directory mode at 0755",
                ExportDisposition::Degraded,
                "directory executable/search state is raised to the frozen target constant",
            ));
        }
        if let EntryData::File { content } = entry.data() {
            let _ = logical_content(archive, *content)?;
        }
    }
    Ok(issues)
}

fn analyze_path(
    path: &LogicalPath,
    rendered: &str,
    directory: bool,
    target: ExportTarget,
    issues: &mut Vec<ExportIssue>,
    zip_equivalence: &mut BTreeMap<String, LogicalPath>,
) {
    let hostile = path.components().iter().any(|component| {
        let text = std::str::from_utf8(component.bytes()).expect("LogicalPath is UTF-8");
        text.contains('\\') || text.contains(':')
    });
    if hostile {
        issues.push(issue(
            ExportIssueCategory::PathUnrepresentable,
            Some(path.clone()),
            "entry.path",
            rendered,
            target.profile_id(),
            ExportDisposition::Refused,
            "backslash or colon has unsafe cross-platform target interpretation",
        ));
    }
    if target == ExportTarget::ZipPortableV1 {
        for component in path.components() {
            let text = std::str::from_utf8(component.bytes()).expect("LogicalPath is UTF-8");
            if !zip_portable_component(text) {
                issues.push(issue(
                    ExportIssueCategory::PathUnrepresentable,
                    Some(path.clone()),
                    "entry.path",
                    rendered,
                    ZIP_PROFILE,
                    ExportDisposition::Refused,
                    "component is not portable across common ZIP extraction targets",
                ));
            }
        }
        let key = rendered.to_lowercase();
        if let Some(prior) = zip_equivalence.insert(key, path.clone())
            && prior != *path
        {
            issues.push(issue(
                ExportIssueCategory::PathCollision,
                Some(path.clone()),
                "entry.path",
                rendered,
                "case-insensitive ZIP target namespace",
                ExportDisposition::Refused,
                format!("target-equivalent collision with {prior}"),
            ));
        }
        let directory_suffix = usize::from(directory);
        if rendered.len().saturating_add(directory_suffix) > usize::from(u16::MAX) {
            issues.push(issue(
                ExportIssueCategory::TargetLimitExceeded,
                Some(path.clone()),
                "entry.path",
                rendered.len().to_string(),
                "ZIP filename length <= 65535 bytes",
                ExportDisposition::Refused,
                "encoded ZIP name exceeds its u16 framing field",
            ));
        }
    }
}

fn zip_portable_component(value: &str) -> bool {
    if value.ends_with(['.', ' '])
        || value.chars().any(|character| {
            character.is_control() || matches!(character, '<' | '>' | '"' | '|' | '?' | '*')
        })
    {
        return false;
    }
    let stem = value
        .split('.')
        .next()
        .unwrap_or(value)
        .to_ascii_uppercase();
    !matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        && !(stem.len() == 4
            && (stem.starts_with("COM") || stem.starts_with("LPT"))
            && matches!(stem.as_bytes()[3], b'1'..=b'9'))
}

fn analyze_mtime(
    path: &LogicalPath,
    timestamp: Timestamp,
    target: ExportTarget,
    issues: &mut Vec<ExportIssue>,
) {
    match target {
        ExportTarget::ZipPortableV1 => {
            if timestamp.source_precision() != TimestampPrecision::Second
                || timestamp.nanoseconds() != 0
            {
                issues.push(issue(
                    ExportIssueCategory::TimestampPrecisionLoss,
                    Some(path.clone()),
                    "core.mtime",
                    timestamp_display(timestamp),
                    "Info-ZIP extended timestamp has one-second precision",
                    ExportDisposition::Degraded,
                    format!(
                        "precision reduced from {:?} to one second",
                        timestamp.source_precision()
                    ),
                ));
            }
            if i32::try_from(timestamp.seconds()).is_err() {
                issues.push(issue(
                    ExportIssueCategory::TimestampRangeLoss,
                    Some(path.clone()),
                    "core.mtime",
                    timestamp_display(timestamp),
                    "portable extended ZIP mtime is signed 32-bit Unix seconds",
                    ExportDisposition::Degraded,
                    "timestamp is clamped to the nearest representable second",
                ));
            }
        }
        ExportTarget::TarPaxV1 => {
            if !timestamp_alignment_valid(timestamp) {
                issues.push(issue(
                    ExportIssueCategory::TimestampPrecisionLoss,
                    Some(path.clone()),
                    "core.mtime",
                    timestamp_display(timestamp),
                    "pax decimal precision matching the declared Entrybound precision",
                    ExportDisposition::Refused,
                    "timestamp value contradicts its declared precision",
                ));
            }
        }
        _ => unreachable!("semantic target is normalized"),
    }
}

fn timestamp_alignment_valid(timestamp: Timestamp) -> bool {
    match timestamp.source_precision() {
        TimestampPrecision::Second => timestamp.nanoseconds() == 0,
        TimestampPrecision::Centisecond => timestamp.nanoseconds().is_multiple_of(10_000_000),
        TimestampPrecision::Microsecond => timestamp.nanoseconds().is_multiple_of(1_000),
        TimestampPrecision::Hectonanosecond => timestamp.nanoseconds().is_multiple_of(100),
        TimestampPrecision::Nanosecond => true,
    }
}

fn timestamp_display(value: Timestamp) -> String {
    format!(
        "{}.{:09} ({:?})",
        value.seconds(),
        value.nanoseconds(),
        value.source_precision()
    )
}

fn issue(
    category: ExportIssueCategory,
    entry: Option<LogicalPath>,
    semantic_field: impl Into<String>,
    source_value: impl Into<String>,
    target_capability: impl Into<String>,
    disposition: ExportDisposition,
    reason: impl Into<String>,
) -> ExportIssue {
    ExportIssue {
        category,
        entry,
        semantic_field: semantic_field.into(),
        source_value: source_value.into(),
        target_capability: target_capability.into(),
        disposition,
        reason: reason.into(),
    }
}

fn target_path(path: &LogicalPath) -> String {
    path.components()
        .iter()
        .map(|component| std::str::from_utf8(component.bytes()).expect("LogicalPath is UTF-8"))
        .collect::<Vec<_>>()
        .join("/")
}

fn logical_content(archive: &Archive, content: ContentRef) -> Result<Vec<u8>> {
    let ContentRef::Internal(object_id) = content;
    let object = archive
        .content_store
        .objects
        .get(&object_id)
        .ok_or_else(|| {
            Diagnostic::new(
                OutcomeClass::Nonconforming,
                ReasonCode::UnknownContentObject,
                object_id.to_string(),
            )
        })?;
    let capacity = object.chunks.iter().try_fold(0_usize, |total, reference| {
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
            .checked_add(chunk.plaintext.len())
            .ok_or_else(|| target_limit("logical content length exceeds usize"))
    })?;
    let mut bytes = Vec::with_capacity(capacity);
    for reference in &object.chunks {
        bytes.extend_from_slice(&archive.content_store.chunks[&reference.chunk_id].plaintext);
    }
    if sha256_exact(&bytes) != object.logical_digest {
        return Err(Diagnostic::new(
            OutcomeClass::Corrupt,
            ReasonCode::ContentDigestMismatch,
            object.logical_digest.to_string(),
        ));
    }
    Ok(bytes)
}

fn target_limit(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::PolicyRefused,
        ReasonCode::LegacyExportTargetInvalid,
        detail,
    )
}

// ---------------------------------------------------------------------------
// zip/portable-v1
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
struct ZipEntryPlan {
    name: Vec<u8>,
    method: u16,
    crc32: u32,
    uncompressed_len: u64,
    stored: Vec<u8>,
    dos_time: u16,
    dos_date: u16,
    timestamp_extra: Vec<u8>,
    external_attributes: u32,
}

#[derive(Clone, Copy, Debug)]
struct ZipCentralPlan {
    local_offset: u64,
    version_needed: u16,
}

fn encode_zip(archive: &Archive, _issues: &mut Vec<ExportIssue>) -> Result<Vec<u8>> {
    let mut plans = Vec::with_capacity(archive.entry_set.len());
    for entry in archive.entry_set.entries() {
        plans.push(zip_entry_plan(archive, entry)?);
    }
    let mut output = Vec::new();
    let mut central_plans = Vec::with_capacity(plans.len());
    for plan in &plans {
        let local_offset = u64::try_from(output.len())
            .map_err(|_| target_limit("ZIP local offset exceeds u64"))?;
        let zip64_sizes = plan.uncompressed_len >= ZIP_U32_SENTINEL
            || u64::try_from(plan.stored.len()).unwrap_or(u64::MAX) >= ZIP_U32_SENTINEL;
        let version_needed = if zip64_sizes {
            ZIP_VERSION_45
        } else {
            ZIP_VERSION_20
        };
        write_zip_local(&mut output, plan, version_needed, zip64_sizes)?;
        central_plans.push(ZipCentralPlan {
            local_offset,
            version_needed,
        });
    }
    let central_offset = u64::try_from(output.len())
        .map_err(|_| target_limit("ZIP central-directory offset exceeds u64"))?;
    for (plan, central) in plans.iter().zip(&central_plans) {
        write_zip_central(&mut output, plan, *central)?;
    }
    let central_end = u64::try_from(output.len())
        .map_err(|_| target_limit("ZIP central-directory length exceeds u64"))?;
    let central_size = central_end
        .checked_sub(central_offset)
        .ok_or_else(|| target_limit("ZIP central-directory extent underflow"))?;
    write_zip_end(
        &mut output,
        u64::try_from(plans.len()).map_err(|_| target_limit("ZIP entry count exceeds u64"))?,
        central_offset,
        central_size,
    )?;
    Ok(output)
}

fn zip_entry_plan(archive: &Archive, entry: &Entry) -> Result<ZipEntryPlan> {
    let directory = matches!(entry.data(), EntryData::Directory);
    let mut name = target_path(entry.path()).into_bytes();
    if directory {
        name.push(b'/');
    }
    if name.len() > usize::from(u16::MAX) {
        return Err(target_limit("ZIP filename exceeds u16"));
    }
    let plaintext = match entry.data() {
        EntryData::Directory => Vec::new(),
        EntryData::File { content } => logical_content(archive, *content)?,
        EntryData::Symlink { .. } => {
            return Err(target_limit(
                "zip/portable-v1 cannot encode a symlink Entry",
            ));
        }
    };
    let mut crc = Crc32::new();
    crc.update(&plaintext);
    let crc32 = crc.finalize();
    let (method, stored) = if directory || plaintext.is_empty() {
        (ZIP_STORE, plaintext.clone())
    } else {
        let mut encoder = DeflateEncoder::new(Vec::new(), Compression::new(6));
        encoder
            .write_all(&plaintext)
            .map_err(|error| target_io("compress deterministic ZIP DEFLATE", error))?;
        let deflated = encoder
            .finish()
            .map_err(|error| target_io("finish deterministic ZIP DEFLATE", error))?;
        if deflated.len() < plaintext.len() {
            (ZIP_DEFLATE, deflated)
        } else {
            (ZIP_STORE, plaintext.clone())
        }
    };
    let (dos_time, dos_date, timestamp_extra) = zip_timestamp(entry.metadata().mtime());
    let unix_mode = if directory && entry.metadata().executable() {
        0o040755_u32
    } else if directory {
        0o040644_u32
    } else if entry.metadata().executable() {
        0o100755_u32
    } else {
        0o100644_u32
    };
    let dos_attributes = if directory { 0x10 } else { 0x20 };
    Ok(ZipEntryPlan {
        name,
        method,
        crc32,
        uncompressed_len: u64::try_from(plaintext.len())
            .map_err(|_| target_limit("ZIP entry size exceeds u64"))?,
        stored,
        dos_time,
        dos_date,
        timestamp_extra,
        external_attributes: (unix_mode << 16) | dos_attributes,
    })
}

fn write_zip_local(
    output: &mut Vec<u8>,
    plan: &ZipEntryPlan,
    version_needed: u16,
    zip64_sizes: bool,
) -> Result<()> {
    let compressed_len = u64::try_from(plan.stored.len())
        .map_err(|_| target_limit("ZIP compressed length exceeds u64"))?;
    let mut extra = plan.timestamp_extra.clone();
    if zip64_sizes {
        let mut body = Vec::with_capacity(16);
        body.extend_from_slice(&plan.uncompressed_len.to_le_bytes());
        body.extend_from_slice(&compressed_len.to_le_bytes());
        zip_extra(&mut extra, 0x0001, &body)?;
    }
    let extra_len =
        u16::try_from(extra.len()).map_err(|_| target_limit("ZIP local extras exceed u16"))?;
    output.extend_from_slice(&0x0403_4b50_u32.to_le_bytes());
    output.extend_from_slice(&version_needed.to_le_bytes());
    output.extend_from_slice(&ZIP_UTF8_FLAG.to_le_bytes());
    output.extend_from_slice(&plan.method.to_le_bytes());
    output.extend_from_slice(&plan.dos_time.to_le_bytes());
    output.extend_from_slice(&plan.dos_date.to_le_bytes());
    output.extend_from_slice(&plan.crc32.to_le_bytes());
    output.extend_from_slice(&zip_u32_size(compressed_len, zip64_sizes).to_le_bytes());
    output.extend_from_slice(&zip_u32_size(plan.uncompressed_len, zip64_sizes).to_le_bytes());
    output.extend_from_slice(
        &u16::try_from(plan.name.len())
            .map_err(|_| target_limit("ZIP filename exceeds u16"))?
            .to_le_bytes(),
    );
    output.extend_from_slice(&extra_len.to_le_bytes());
    output.extend_from_slice(&plan.name);
    output.extend_from_slice(&extra);
    output.extend_from_slice(&plan.stored);
    Ok(())
}

fn write_zip_central(
    output: &mut Vec<u8>,
    plan: &ZipEntryPlan,
    central: ZipCentralPlan,
) -> Result<()> {
    let compressed_len = u64::try_from(plan.stored.len())
        .map_err(|_| target_limit("ZIP compressed length exceeds u64"))?;
    let zip64_uncompressed = plan.uncompressed_len >= ZIP_U32_SENTINEL;
    let zip64_compressed = compressed_len >= ZIP_U32_SENTINEL;
    let zip64_offset = central.local_offset >= ZIP_U32_SENTINEL;
    let mut extra = plan.timestamp_extra.clone();
    if zip64_uncompressed || zip64_compressed || zip64_offset {
        let mut body = Vec::new();
        if zip64_uncompressed {
            body.extend_from_slice(&plan.uncompressed_len.to_le_bytes());
        }
        if zip64_compressed {
            body.extend_from_slice(&compressed_len.to_le_bytes());
        }
        if zip64_offset {
            body.extend_from_slice(&central.local_offset.to_le_bytes());
        }
        zip_extra(&mut extra, 0x0001, &body)?;
    }
    let version_needed = if zip64_uncompressed || zip64_compressed || zip64_offset {
        ZIP_VERSION_45
    } else {
        central.version_needed
    };
    output.extend_from_slice(&0x0201_4b50_u32.to_le_bytes());
    output.extend_from_slice(&((3_u16 << 8) | version_needed).to_le_bytes());
    output.extend_from_slice(&version_needed.to_le_bytes());
    output.extend_from_slice(&ZIP_UTF8_FLAG.to_le_bytes());
    output.extend_from_slice(&plan.method.to_le_bytes());
    output.extend_from_slice(&plan.dos_time.to_le_bytes());
    output.extend_from_slice(&plan.dos_date.to_le_bytes());
    output.extend_from_slice(&plan.crc32.to_le_bytes());
    output.extend_from_slice(&zip_u32_size(compressed_len, zip64_compressed).to_le_bytes());
    output
        .extend_from_slice(&zip_u32_size(plan.uncompressed_len, zip64_uncompressed).to_le_bytes());
    output.extend_from_slice(
        &u16::try_from(plan.name.len())
            .map_err(|_| target_limit("ZIP filename exceeds u16"))?
            .to_le_bytes(),
    );
    output.extend_from_slice(
        &u16::try_from(extra.len())
            .map_err(|_| target_limit("ZIP central extras exceed u16"))?
            .to_le_bytes(),
    );
    output.extend_from_slice(&0_u16.to_le_bytes()); // comment length
    output.extend_from_slice(&0_u16.to_le_bytes()); // disk start
    output.extend_from_slice(&0_u16.to_le_bytes()); // internal attributes
    output.extend_from_slice(&plan.external_attributes.to_le_bytes());
    output.extend_from_slice(
        &(if zip64_offset {
            u32::MAX
        } else {
            u32::try_from(central.local_offset).expect("checked ZIP offset")
        })
        .to_le_bytes(),
    );
    output.extend_from_slice(&plan.name);
    output.extend_from_slice(&extra);
    Ok(())
}

fn write_zip_end(
    output: &mut Vec<u8>,
    entry_count: u64,
    central_offset: u64,
    central_size: u64,
) -> Result<()> {
    let zip64 = entry_count >= ZIP_U16_SENTINEL
        || central_offset >= ZIP_U32_SENTINEL
        || central_size >= ZIP_U32_SENTINEL;
    if zip64 {
        let zip64_offset = u64::try_from(output.len())
            .map_err(|_| target_limit("ZIP64 EOCD offset exceeds u64"))?;
        output.extend_from_slice(&0x0606_4b50_u32.to_le_bytes());
        output.extend_from_slice(&44_u64.to_le_bytes());
        output.extend_from_slice(&((3_u16 << 8) | ZIP_VERSION_45).to_le_bytes());
        output.extend_from_slice(&ZIP_VERSION_45.to_le_bytes());
        output.extend_from_slice(&0_u32.to_le_bytes());
        output.extend_from_slice(&0_u32.to_le_bytes());
        output.extend_from_slice(&entry_count.to_le_bytes());
        output.extend_from_slice(&entry_count.to_le_bytes());
        output.extend_from_slice(&central_size.to_le_bytes());
        output.extend_from_slice(&central_offset.to_le_bytes());
        output.extend_from_slice(&0x0706_4b50_u32.to_le_bytes());
        output.extend_from_slice(&0_u32.to_le_bytes());
        output.extend_from_slice(&zip64_offset.to_le_bytes());
        output.extend_from_slice(&1_u32.to_le_bytes());
    }
    output.extend_from_slice(&0x0605_4b50_u32.to_le_bytes());
    output.extend_from_slice(&0_u16.to_le_bytes());
    output.extend_from_slice(&0_u16.to_le_bytes());
    let count16 = if zip64 {
        u16::MAX
    } else {
        u16::try_from(entry_count).map_err(|_| target_limit("ZIP entry count exceeds u16"))?
    };
    output.extend_from_slice(&count16.to_le_bytes());
    output.extend_from_slice(&count16.to_le_bytes());
    output.extend_from_slice(
        &(if zip64 {
            u32::MAX
        } else {
            u32::try_from(central_size).map_err(|_| target_limit("ZIP central size exceeds u32"))?
        })
        .to_le_bytes(),
    );
    output.extend_from_slice(
        &(if zip64 {
            u32::MAX
        } else {
            u32::try_from(central_offset)
                .map_err(|_| target_limit("ZIP central offset exceeds u32"))?
        })
        .to_le_bytes(),
    );
    output.extend_from_slice(&0_u16.to_le_bytes());
    Ok(())
}

fn zip_u32_size(value: u64, sentinel: bool) -> u32 {
    if sentinel {
        u32::MAX
    } else {
        u32::try_from(value).expect("checked ZIP32 size")
    }
}

fn zip_extra(output: &mut Vec<u8>, id: u16, body: &[u8]) -> Result<()> {
    output.extend_from_slice(&id.to_le_bytes());
    output.extend_from_slice(
        &u16::try_from(body.len())
            .map_err(|_| target_limit("ZIP extra field exceeds u16"))?
            .to_le_bytes(),
    );
    output.extend_from_slice(body);
    Ok(())
}

fn zip_timestamp(timestamp: Option<Timestamp>) -> (u16, u16, Vec<u8>) {
    let Some(timestamp) = timestamp else {
        return (0, (1 << 5) | 1, Vec::new()); // 1980-01-01 00:00:00
    };
    let seconds = timestamp
        .seconds()
        .clamp(i64::from(i32::MIN), i64::from(i32::MAX));
    let (dos_time, dos_date) = dos_timestamp(seconds);
    let mut extra = Vec::with_capacity(9);
    extra.extend_from_slice(&0x5455_u16.to_le_bytes());
    extra.extend_from_slice(&5_u16.to_le_bytes());
    extra.push(1);
    extra.extend_from_slice(&(seconds as i32).to_le_bytes());
    (dos_time, dos_date, extra)
}

fn dos_timestamp(seconds: i64) -> (u16, u16) {
    let days = seconds.div_euclid(86_400);
    let day_seconds = seconds.rem_euclid(86_400);
    let (mut year, mut month, mut day) = civil_from_days(days);
    let mut hour = day_seconds / 3600;
    let mut minute = (day_seconds % 3600) / 60;
    let mut second = day_seconds % 60;
    if year < 1980 {
        (year, month, day, hour, minute, second) = (1980, 1, 1, 0, 0, 0);
    } else if year > 2107 {
        (year, month, day, hour, minute, second) = (2107, 12, 31, 23, 59, 58);
    }
    let time = (u16::try_from(hour).unwrap() << 11)
        | (u16::try_from(minute).unwrap() << 5)
        | u16::try_from(second / 2).unwrap();
    let date = (u16::try_from(year - 1980).unwrap() << 9)
        | (u16::try_from(month).unwrap() << 5)
        | u16::try_from(day).unwrap();
    (time, date)
}

// Howard Hinnant's proleptic-Gregorian civil-from-days mapping, with day zero
// at the Unix epoch. Integer-only and platform-independent.
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let days = days + 719_468;
    let era = if days >= 0 { days } else { days - 146_096 } / 146_097;
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    (year, month, day)
}

fn target_io(action: &str, error: std::io::Error) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::Corrupt,
        ReasonCode::LegacyExportTargetInvalid,
        format!("cannot {action}: {error}"),
    )
}

fn encode_target(
    archive: &Archive,
    target: ExportTarget,
    issues: &mut Vec<ExportIssue>,
) -> Result<(Vec<u8>, Option<WrappedTargetReceipt>)> {
    match target {
        ExportTarget::ZipPortableV1 => encode_zip(archive, issues).map(|bytes| (bytes, None)),
        ExportTarget::TarPaxV1 => encode_tar(archive, issues).map(|bytes| (bytes, None)),
        ExportTarget::TarGzipPaxV1
        | ExportTarget::TarZstandardPaxV1
        | ExportTarget::TarXzPaxV1
        | ExportTarget::TarBzip2PaxV1 => {
            let tar = encode_tar(archive, issues)?;
            let inner_tar_byte_length = u64::try_from(tar.len())
                .map_err(|_| target_limit("inner tar length exceeds u64"))?;
            let inner_tar_sha256 = sha256_exact(&tar);
            let (bytes, transport) = match target {
                ExportTarget::TarGzipPaxV1 => (encode_gzip_wrapper(&tar)?, TransportFormat::Gzip),
                ExportTarget::TarZstandardPaxV1 => {
                    (encode_zstd_wrapper(&tar)?, TransportFormat::Zstandard)
                }
                ExportTarget::TarXzPaxV1 => (encode_xz_wrapper(&tar)?, TransportFormat::Xz),
                ExportTarget::TarBzip2PaxV1 => {
                    (encode_bzip2_wrapper(&tar)?, TransportFormat::Bzip2)
                }
                ExportTarget::ZipPortableV1 | ExportTarget::TarPaxV1 => unreachable!(),
            };
            let decoded = stream::decode(&bytes, transport, WrapperImportPolicy::default())?;
            if decoded.member_count != 1 || decoded.decoded.as_ref() != tar {
                return Err(Diagnostic::new(
                    OutcomeClass::Corrupt,
                    ReasonCode::LegacyExportTargetInvalid,
                    format!(
                        "{} self-validation did not reproduce exact tar/pax-v1 bytes",
                        target.profile_id()
                    ),
                ));
            }
            Ok((
                bytes,
                Some(WrappedTargetReceipt {
                    semantic_profile: TAR_PROFILE.to_owned(),
                    transport_profile: target
                        .transport_profile()
                        .expect("wrapped target has transport")
                        .to_owned(),
                    inner_tar_byte_length,
                    inner_tar_sha256,
                }),
            ))
        }
    }
}

fn encode_gzip_wrapper(tar: &[u8]) -> Result<Vec<u8>> {
    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::new(6));
    encoder
        .write_all(tar)
        .map_err(|error| target_io("compress deterministic gzip payload", error))?;
    let deflate = encoder
        .finish()
        .map_err(|error| target_io("finish deterministic gzip payload", error))?;
    let mut crc = Crc32::new();
    crc.update(tar);
    let tar_length = u64::try_from(tar.len())
        .map_err(|_| target_limit("tar payload length does not fit deterministic gzip"))?;
    let isize = u32::try_from(tar_length & u64::from(u32::MAX))
        .expect("masking to 32 bits always fits u32");
    let mut output = Vec::with_capacity(10 + deflate.len() + 8);
    output.extend_from_slice(&[0x1f, 0x8b, 8, 0]);
    output.extend_from_slice(&0_u32.to_le_bytes());
    output.push(0); // XFL: the RFC-defined neutral value for frozen level 6.
    output.push(255); // OS: unknown, independent of the host.
    output.extend_from_slice(&deflate);
    output.extend_from_slice(&crc.finalize().to_le_bytes());
    output.extend_from_slice(&isize.to_le_bytes());
    Ok(output)
}

fn encode_zstd_wrapper(tar: &[u8]) -> Result<Vec<u8>> {
    let tar_length = u64::try_from(tar.len())
        .map_err(|_| target_limit("tar payload length does not fit deterministic Zstandard"))?;
    let mut encoder = zstd::stream::write::Encoder::new(Vec::new(), 9)
        .map_err(|error| target_io("create deterministic Zstandard encoder", error))?;
    encoder
        .include_checksum(true)
        .and_then(|()| encoder.include_contentsize(true))
        .and_then(|()| encoder.include_dictid(false))
        .and_then(|()| encoder.long_distance_matching(false))
        .and_then(|()| encoder.set_pledged_src_size(Some(tar_length)))
        .map_err(|error| target_io("configure deterministic Zstandard frame", error))?;
    encoder
        .write_all(tar)
        .map_err(|error| target_io("compress deterministic Zstandard frame", error))?;
    encoder
        .finish()
        .map_err(|error| target_io("finish deterministic Zstandard frame", error))
}

fn encode_xz_wrapper(tar: &[u8]) -> Result<Vec<u8>> {
    // Preset 6 freezes its LZMA2 parameters; XzOptions freezes one block and
    // CRC64 by default in lzma-rust2 0.20.0.
    let options = XzOptions::with_preset(6);
    let mut encoder = XzWriter::new(Vec::new(), options)
        .map_err(|error| target_io("create deterministic XZ encoder", error))?;
    encoder
        .write_all(tar)
        .map_err(|error| target_io("compress deterministic XZ stream", error))?;
    encoder
        .finish()
        .map_err(|error| target_io("finish deterministic XZ stream", error))
}

fn encode_bzip2_wrapper(tar: &[u8]) -> Result<Vec<u8>> {
    let mut encoder = BzEncoder::new(Vec::new(), bzip2::Compression::best());
    encoder
        .write_all(tar)
        .map_err(|error| target_io("compress deterministic bzip2 stream", error))?;
    encoder
        .finish()
        .map_err(|error| target_io("finish deterministic bzip2 stream", error))
}

fn validate_strict_reimport(
    source: &Archive,
    target: ExportTarget,
    bytes: &[u8],
    outcome: ExportOutcome,
    issues: &[ExportIssue],
) -> Result<Digest> {
    let imported = import_strict(
        bytes,
        Some(target.import_format()),
        None,
        LegacyImportPolicy::default(),
        crate::planner::CompressionProfile::Fast,
    )?;
    if source.entry_set.len() != imported.archive.entry_set.len() {
        return Err(reimport_mismatch(target, "entry count changed"));
    }
    for (expected, actual) in source
        .entry_set
        .entries()
        .iter()
        .zip(imported.archive.entry_set.entries())
    {
        if expected.path() != actual.path() {
            return Err(reimport_mismatch(target, "entry path/order changed"));
        }
        match (expected.data(), actual.data()) {
            (EntryData::Directory, EntryData::Directory) => {}
            (
                EntryData::File {
                    content: expected_content,
                },
                EntryData::File {
                    content: actual_content,
                },
            ) => {
                if logical_content(source, *expected_content)?
                    != logical_content(&imported.archive, *actual_content)?
                {
                    return Err(reimport_mismatch(target, "regular-file bytes changed"));
                }
            }
            _ => return Err(reimport_mismatch(target, "entry kind changed")),
        }
        ensure_metadata_difference_declared(expected, actual, outcome, issues, target)?;
    }
    let reimport_lai = imported.archive.descriptor.lai;
    if outcome == ExportOutcome::Lossless && reimport_lai != source.descriptor.lai {
        return Err(reimport_mismatch(
            target,
            "LOSSLESS strict re-import LAI differs from source LAI",
        ));
    }
    Ok(reimport_lai)
}

fn ensure_metadata_difference_declared(
    expected: &Entry,
    actual: &Entry,
    outcome: ExportOutcome,
    issues: &[ExportIssue],
    target: ExportTarget,
) -> Result<()> {
    for (field, differs) in [
        (
            "core.executable",
            executable_claim(expected) != executable_claim(actual),
        ),
        (
            "core.mtime",
            expected.metadata().mtime() != actual.metadata().mtime(),
        ),
    ] {
        if differs
            && (outcome != ExportOutcome::Lossy
                || !issues.iter().any(|issue| {
                    issue.entry.as_ref() == Some(expected.path()) && issue.semantic_field == field
                }))
        {
            return Err(reimport_mismatch(
                target,
                format!(
                    "undeclared {field} semantic difference at {}",
                    expected.path()
                ),
            ));
        }
    }
    Ok(())
}

fn executable_claim(entry: &Entry) -> Option<bool> {
    entry.metadata().items().iter().find_map(|item| {
        (item.name() == MetadataName::CoreExecutable).then(|| match item.value() {
            MetadataValue::Bool(value) => *value,
            _ => unreachable!("validated core.executable type"),
        })
    })
}

fn reimport_mismatch(target: ExportTarget, detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::Corrupt,
        ReasonCode::LegacyExportTargetInvalid,
        format!(
            "{} strict re-import validation failed: {}",
            target.profile_id(),
            detail.into()
        ),
    )
}

// ---------------------------------------------------------------------------
// tar/pax-v1
// ---------------------------------------------------------------------------

fn encode_tar(archive: &Archive, _issues: &mut Vec<ExportIssue>) -> Result<Vec<u8>> {
    let mut output = Vec::new();
    for (ordinal, entry) in archive.entry_set.entries().iter().enumerate() {
        write_tar_entry(&mut output, archive, entry, ordinal)?;
    }
    output.resize(
        output
            .len()
            .checked_add(TAR_BLOCK * 2)
            .ok_or_else(|| target_limit("tar end blocks overflow usize"))?,
        0,
    );
    Ok(output)
}

fn write_tar_entry(
    output: &mut Vec<u8>,
    archive: &Archive,
    entry: &Entry,
    ordinal: usize,
) -> Result<()> {
    let directory = matches!(entry.data(), EntryData::Directory);
    let plaintext = match entry.data() {
        EntryData::Directory => Vec::new(),
        EntryData::File { content } => logical_content(archive, *content)?,
        EntryData::Symlink { .. } => {
            return Err(target_limit("tar/pax-v1 cannot encode a symlink Entry"));
        }
    };
    let size = u64::try_from(plaintext.len()).map_err(|_| target_limit("tar file exceeds u64"))?;
    let mut path = target_path(entry.path());
    if directory {
        path.push('/');
    }
    let split = split_ustar_path(path.as_bytes());
    let mtime = entry.metadata().mtime();
    let mtime_in_header = mtime.is_some_and(|value| {
        value.source_precision() == TimestampPrecision::Second
            && value.nanoseconds() == 0
            && value.seconds() >= 0
            && u64::try_from(value.seconds()).is_ok_and(|value| value <= TAR_OCTAL_TIME_MAX)
    });
    let size_in_header = size <= TAR_OCTAL_SIZE_MAX;
    let mut pax = BTreeMap::<&str, String>::new();
    if !mtime_in_header && let Some(value) = mtime {
        pax.insert("mtime", pax_timestamp(value));
    }
    if split.is_none() {
        pax.insert("path", path.clone());
    }
    if !size_in_header {
        pax.insert("size", size.to_string());
    }
    if !pax.is_empty() {
        let payload = pax_payload(&pax)?;
        let pax_name = format!("PaxHeaders/{ordinal:016x}");
        let header = tar_header(TarHeaderInput {
            name: pax_name.as_bytes(),
            prefix: &[],
            mode: 0o644,
            size: u64::try_from(payload.len()).map_err(|_| target_limit("pax data exceeds u64"))?,
            mtime: 0,
            typeflag: b'x',
        })?;
        output.extend_from_slice(&header);
        tar_payload(output, &payload)?;
    }
    let (name, prefix) =
        split.unwrap_or_else(|| (format!("PaxFiles/{ordinal:016x}").into_bytes(), Vec::new()));
    let mode = if directory || entry.metadata().executable() {
        0o755
    } else {
        0o644
    };
    let header = tar_header(TarHeaderInput {
        name: &name,
        prefix: &prefix,
        mode,
        size: if size_in_header { size } else { 0 },
        mtime: if mtime_in_header {
            u64::try_from(mtime.expect("checked mtime").seconds()).unwrap()
        } else {
            0
        },
        typeflag: if directory { b'5' } else { b'0' },
    })?;
    output.extend_from_slice(&header);
    if !directory {
        tar_payload(output, &plaintext)?;
    }
    Ok(())
}

struct TarHeaderInput<'a> {
    name: &'a [u8],
    prefix: &'a [u8],
    mode: u64,
    size: u64,
    mtime: u64,
    typeflag: u8,
}

fn tar_header(input: TarHeaderInput<'_>) -> Result<[u8; TAR_BLOCK]> {
    if input.name.len() > 100 || input.prefix.len() > 155 {
        return Err(target_limit(
            "ustar name/prefix exceeds fixed header fields",
        ));
    }
    let mut header = [0_u8; TAR_BLOCK];
    header[..input.name.len()].copy_from_slice(input.name);
    tar_octal(&mut header[100..108], input.mode, "mode")?;
    tar_octal(&mut header[108..116], 0, "uid")?;
    tar_octal(&mut header[116..124], 0, "gid")?;
    tar_octal(&mut header[124..136], input.size, "size")?;
    tar_octal(&mut header[136..148], input.mtime, "mtime")?;
    header[148..156].fill(b' ');
    header[156] = input.typeflag;
    header[257..263].copy_from_slice(b"ustar\0");
    header[263..265].copy_from_slice(b"00");
    header[345..345 + input.prefix.len()].copy_from_slice(input.prefix);
    let checksum = header.iter().map(|value| u64::from(*value)).sum::<u64>();
    let checksum_text = format!("{checksum:06o}");
    if checksum_text.len() != 6 {
        return Err(target_limit("tar checksum exceeds six octal digits"));
    }
    header[148..154].copy_from_slice(checksum_text.as_bytes());
    header[154] = 0;
    header[155] = b' ';
    Ok(header)
}

fn tar_octal(field: &mut [u8], value: u64, label: &str) -> Result<()> {
    let digits = field
        .len()
        .checked_sub(1)
        .ok_or_else(|| target_limit("empty tar numeric field"))?;
    let text = format!("{value:0digits$o}");
    if text.len() > digits {
        return Err(target_limit(format!("tar {label} exceeds octal field")));
    }
    field.fill(b'0');
    field[..digits].copy_from_slice(text.as_bytes());
    field[digits] = 0;
    Ok(())
}

fn tar_payload(output: &mut Vec<u8>, payload: &[u8]) -> Result<()> {
    output.extend_from_slice(payload);
    let padding = (TAR_BLOCK - payload.len() % TAR_BLOCK) % TAR_BLOCK;
    output.resize(
        output
            .len()
            .checked_add(padding)
            .ok_or_else(|| target_limit("tar padding overflows usize"))?,
        0,
    );
    Ok(())
}

fn split_ustar_path(path: &[u8]) -> Option<(Vec<u8>, Vec<u8>)> {
    if path.len() <= 100 {
        return Some((path.to_vec(), Vec::new()));
    }
    path.iter()
        .enumerate()
        .filter(|(_, byte)| **byte == b'/')
        .rev()
        .find_map(|(separator, _)| {
            let prefix = &path[..separator];
            let name = &path[separator + 1..];
            (!name.is_empty() && name.len() <= 100 && prefix.len() <= 155)
                .then(|| (name.to_vec(), prefix.to_vec()))
        })
}

fn pax_payload(values: &BTreeMap<&str, String>) -> Result<Vec<u8>> {
    let mut output = Vec::new();
    for (key, value) in values {
        if key.as_bytes().contains(&b'=') || value.as_bytes().contains(&b'\n') {
            return Err(target_limit(
                "pax key/value is not canonically representable",
            ));
        }
        let body = format!("{key}={value}\n");
        let mut length = body.len() + 2;
        loop {
            let next = body
                .len()
                .checked_add(length.to_string().len())
                .and_then(|value| value.checked_add(1))
                .ok_or_else(|| target_limit("pax record length overflows usize"))?;
            if next == length {
                break;
            }
            length = next;
        }
        output.extend_from_slice(length.to_string().as_bytes());
        output.push(b' ');
        output.extend_from_slice(body.as_bytes());
    }
    Ok(output)
}

fn pax_timestamp(timestamp: Timestamp) -> String {
    let digits = match timestamp.source_precision() {
        TimestampPrecision::Second => 0,
        TimestampPrecision::Centisecond => 2,
        TimestampPrecision::Microsecond => 6,
        TimestampPrecision::Hectonanosecond => 7,
        TimestampPrecision::Nanosecond => 9,
    };
    if digits == 0 {
        return timestamp.seconds().to_string();
    }
    let scale = 10_u32.pow(9 - digits);
    if timestamp.seconds() >= 0 {
        return format!(
            "{}.{:0digits$}",
            timestamp.seconds(),
            timestamp.nanoseconds() / scale,
            digits = usize::try_from(digits).unwrap()
        );
    }
    if timestamp.nanoseconds() == 0 {
        return format!(
            "{}.{:0digits$}",
            timestamp.seconds(),
            0,
            digits = usize::try_from(digits).unwrap()
        );
    }
    let whole = timestamp.seconds() + 1;
    let fraction = (1_000_000_000 - timestamp.nanoseconds()) / scale;
    if whole == 0 {
        format!(
            "-0.{fraction:0digits$}",
            digits = usize::try_from(digits).unwrap()
        )
    } else {
        format!(
            "{whole}.{fraction:0digits$}",
            digits = usize::try_from(digits).unwrap()
        )
    }
}

// ---------------------------------------------------------------------------
// ExportReceipt v1 canonical JSON
// ---------------------------------------------------------------------------

impl ExportReceipt {
    /// Returns UTF-8 canonical JSON with fixed field order, no insignificant
    /// whitespace, deterministic issue order, and a final newline.
    #[must_use]
    pub fn to_canonical_json(&self) -> Vec<u8> {
        if self.wrapped_target.is_some() {
            return self.to_canonical_json_v2();
        }
        let mut output = String::new();
        output.push('{');
        json_pair(&mut output, "format", RECEIPT_FORMAT, true);
        output.push_str("\"version\":1,");
        json_pair(&mut output, "exporter", EXPORTER_ID, false);
        json_pair(
            &mut output,
            "source_lai",
            &self.source_lai.to_string(),
            false,
        );
        json_pair(
            &mut output,
            "source_aux",
            &self.source_aux.to_string(),
            false,
        );
        json_pair(
            &mut output,
            "source_pcr",
            &self.source_pcr.to_string(),
            false,
        );
        output.push_str("\"source_security\":{");
        let _ = write!(
            output,
            "\"encrypted\":{},\"embedded_signature_count\":{},\"detached_signature_count\":{},\"signatures_valid\":{},\"signatures_invalid\":{},\"signatures_stale\":{},\"target_encrypted\":false",
            self.source_security.encrypted,
            self.source_security.embedded_signature_count,
            self.source_security.detached_signature_count,
            self.source_security.signatures_valid,
            self.source_security.signatures_invalid,
            self.source_security.signatures_stale,
        );
        output.push_str("},\"entrybound_only_evidence\":{");
        let _ = write!(
            output,
            "\"fidelity_issue_count\":{},\"conversion_provenance\":{},\"exact_preserved_source\":{},\"reconstruction_audit_count\":{},\"embedded_in_target\":false",
            self.auxiliary.fidelity_issue_count,
            self.auxiliary.conversion_provenance,
            self.auxiliary.exact_preserved_source,
            self.auxiliary.reconstruction_audit_count,
        );
        output.push_str("},");
        json_pair(&mut output, "target_format", &self.target_format, false);
        json_pair(&mut output, "target_profile", &self.target_profile, false);
        json_pair(&mut output, "outcome", self.outcome.as_str(), false);
        output.push_str("\"issues\":[");
        for (index, issue) in self.issues.iter().enumerate() {
            if index != 0 {
                output.push(',');
            }
            output.push('{');
            json_pair(&mut output, "category", issue.category.as_str(), true);
            match &issue.entry {
                Some(path) => json_pair(&mut output, "entry", &path.to_string(), false),
                None => output.push_str("\"entry\":null,"),
            }
            json_pair(&mut output, "semantic_field", &issue.semantic_field, false);
            json_pair(&mut output, "source_value", &issue.source_value, false);
            json_pair(
                &mut output,
                "target_capability",
                &issue.target_capability,
                false,
            );
            json_pair(
                &mut output,
                "disposition",
                issue.disposition.as_str(),
                false,
            );
            json_pair(&mut output, "reason", &issue.reason, false);
            output.pop(); // trailing comma
            output.push('}');
        }
        output.push_str("],");
        let _ = write!(
            output,
            "\"entry_count\":{},\"total_logical_bytes\":{},\"target_byte_length\":{},",
            self.entry_count, self.total_logical_bytes, self.target_byte_length
        );
        json_pair(
            &mut output,
            "target_sha256",
            &self.target_sha256.to_string(),
            false,
        );
        let _ = write!(output, "\"deterministic\":{}", self.deterministic);
        output.push_str("}\n");
        output.into_bytes()
    }

    fn to_canonical_json_v2(&self) -> Vec<u8> {
        let wrapped = self
            .wrapped_target
            .as_ref()
            .expect("receipt v2 is selected only for wrapped targets");
        let mut output = String::new();
        output.push('{');
        json_pair(&mut output, "format", RECEIPT_V2_FORMAT, true);
        output.push_str("\"version\":2,");
        json_pair(&mut output, "exporter", EXPORTER_ID, false);
        json_pair(
            &mut output,
            "source_lai",
            &self.source_lai.to_string(),
            false,
        );
        json_pair(
            &mut output,
            "source_aux",
            &self.source_aux.to_string(),
            false,
        );
        json_pair(
            &mut output,
            "source_pcr",
            &self.source_pcr.to_string(),
            false,
        );
        output.push_str("\"source_security\":{");
        let _ = write!(
            output,
            "\"encrypted\":{},\"embedded_signature_count\":{},\"detached_signature_count\":{},\"signatures_valid\":{},\"signatures_invalid\":{},\"signatures_stale\":{},\"target_encrypted\":false",
            self.source_security.encrypted,
            self.source_security.embedded_signature_count,
            self.source_security.detached_signature_count,
            self.source_security.signatures_valid,
            self.source_security.signatures_invalid,
            self.source_security.signatures_stale,
        );
        output.push_str("},\"entrybound_only_evidence\":{");
        let _ = write!(
            output,
            "\"fidelity_issue_count\":{},\"conversion_provenance\":{},\"exact_preserved_source\":{},\"reconstruction_audit_count\":{},\"embedded_in_target\":false",
            self.auxiliary.fidelity_issue_count,
            self.auxiliary.conversion_provenance,
            self.auxiliary.exact_preserved_source,
            self.auxiliary.reconstruction_audit_count,
        );
        output.push_str("},");
        json_pair(&mut output, "target_format", &self.target_format, false);
        json_pair(&mut output, "target_profile", &self.target_profile, false);
        json_pair(
            &mut output,
            "semantic_target",
            &wrapped.semantic_profile,
            false,
        );
        json_pair(
            &mut output,
            "transport_target",
            &wrapped.transport_profile,
            false,
        );
        let _ = write!(
            output,
            "\"inner_tar_byte_length\":{},",
            wrapped.inner_tar_byte_length
        );
        json_pair(
            &mut output,
            "inner_tar_sha256",
            &wrapped.inner_tar_sha256.to_string(),
            false,
        );
        json_pair(&mut output, "outcome", self.outcome.as_str(), false);
        output.push_str("\"issues\":[");
        write_issues_json(&mut output, &self.issues);
        output.push_str("],");
        let _ = write!(
            output,
            "\"entry_count\":{},\"total_logical_bytes\":{},\"target_byte_length\":{},",
            self.entry_count, self.total_logical_bytes, self.target_byte_length
        );
        json_pair(
            &mut output,
            "target_sha256",
            &self.target_sha256.to_string(),
            false,
        );
        let _ = write!(
            output,
            "\"strict_reimport_validated\":{},",
            self.strict_reimport_validated
        );
        match self.reimport_lai {
            Some(digest) => json_pair(&mut output, "reimport_lai", &digest.to_string(), false),
            None => output.push_str("\"reimport_lai\":null,"),
        }
        let _ = write!(output, "\"deterministic\":{}", self.deterministic);
        output.push_str("}\n");
        output.into_bytes()
    }
}

fn write_issues_json(output: &mut String, issues: &[ExportIssue]) {
    for (index, issue) in issues.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        output.push('{');
        json_pair(output, "category", issue.category.as_str(), true);
        match &issue.entry {
            Some(path) => json_pair(output, "entry", &path.to_string(), false),
            None => output.push_str("\"entry\":null,"),
        }
        json_pair(output, "semantic_field", &issue.semantic_field, false);
        json_pair(output, "source_value", &issue.source_value, false);
        json_pair(output, "target_capability", &issue.target_capability, false);
        json_pair(output, "disposition", issue.disposition.as_str(), false);
        json_pair(output, "reason", &issue.reason, false);
        output.pop();
        output.push('}');
    }
}

fn json_pair(output: &mut String, key: &str, value: &str, first: bool) {
    if !first && !output.ends_with(['{', ',']) {
        output.push(',');
    }
    json_string(output, key);
    output.push(':');
    json_string(output, value);
    output.push(',');
}

fn json_string(output: &mut String, value: &str) {
    output.push('"');
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            character if character <= '\u{1f}' => {
                let _ = write!(output, "\\u{:04x}", u32::from(character));
            }
            character => output.push(character),
        }
    }
    output.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::archive::plan_observed_archive;
    use crate::eam::{
        ConversionProvenance, EntryIdentity, FidelityReport, LinkTarget, MetadataItem, MetadataSet,
    };
    use crate::ecf::{
        SequentialLimits, StreamContentPolicy, StreamWriteOptions, WriteOptions,
        bootstrap_sequential_limits, encode, encode_stream, open, open_stream_with_limits,
    };
    use crate::legacy::import::{
        LegacyImportPolicy, LegacySourceFormat, import_strict as import_legacy_strict,
    };
    use crate::planner::CompressionProfile;

    fn conversion() -> ConversionProvenance {
        ConversionProvenance {
            source_format: "test".to_owned(),
            adapter_id: "entrybound/export-test-source-v1".to_owned(),
            source_digest: sha256_exact(b"test source"),
            import_mode: "strict".to_owned(),
            source_entry_count: 0,
            observation_count: 0,
            omission_count: 0,
            refinement_count: 0,
            divergence_count: 0,
            irreconcilable_count: 0,
            resolutions: Box::default(),
            synthesized_ancestors: Box::default(),
            unsupported_metadata: Box::default(),
            outcome: "success".to_owned(),
        }
    }

    fn metadata(executable: bool, timestamp: Timestamp) -> MetadataSet {
        MetadataSet::new(vec![
            MetadataItem::executable(executable),
            MetadataItem::mtime(timestamp),
        ])
        .unwrap()
    }

    fn second_timestamp() -> Timestamp {
        Timestamp::new(1_700_000_000, 0, TimestampPrecision::Second, true).unwrap()
    }

    fn test_archive(timestamp: Timestamp) -> Archive {
        test_archive_with_profile(timestamp, CompressionProfile::Fast)
    }

    fn test_archive_with_profile(timestamp: Timestamp, profile: CompressionProfile) -> Archive {
        let dir = LogicalPath::from_utf8(["nested"]).unwrap();
        let first = b"compressible compressible compressible compressible".repeat(128);
        let mut noise = vec![0_u8; 4096];
        for (index, value) in noise.iter_mut().enumerate() {
            *value = (index.wrapping_mul(131).wrapping_add(17) & 0xff) as u8;
        }
        let first_digest = sha256_exact(&first);
        let noise_digest = sha256_exact(&noise);
        let entries = vec![
            Entry::new(
                dir,
                EntryData::Directory,
                metadata(true, timestamp),
                EntryIdentity::default(),
            ),
            Entry::new(
                LogicalPath::from_utf8(["nested", "alpha.txt"]).unwrap(),
                EntryData::File {
                    content: ContentRef::Internal(first_digest),
                },
                metadata(true, timestamp),
                EntryIdentity::default(),
            ),
            Entry::new(
                LogicalPath::from_utf8(["noise.bin"]).unwrap(),
                EntryData::File {
                    content: ContentRef::Internal(noise_digest),
                },
                metadata(false, timestamp),
                EntryIdentity::default(),
            ),
        ];
        plan_observed_archive(
            entries,
            vec![first.into_boxed_slice(), noise.into_boxed_slice()],
            FidelityReport::default(),
            conversion(),
            None,
            profile,
        )
        .unwrap()
    }

    #[test]
    fn zip_and_tar_are_deterministic_and_strictly_reimportable() {
        let archive = test_archive(second_timestamp());
        for (target, source_format) in [
            (ExportTarget::ZipPortableV1, LegacySourceFormat::Zip),
            (ExportTarget::TarPaxV1, LegacySourceFormat::Tar),
        ] {
            let first = prepare_export(&archive, target, ExportSourceSecurity::default())
                .unwrap()
                .accept(false)
                .unwrap();
            let second = prepare_export(&archive, target, ExportSourceSecurity::default())
                .unwrap()
                .accept(false)
                .unwrap();
            assert_eq!(first.bytes, second.bytes);
            assert_eq!(first.receipt.outcome, ExportOutcome::Lossless);
            assert_eq!(first.receipt.target_sha256, sha256_exact(&first.bytes));
            let imported = import_legacy_strict(
                &first.bytes,
                Some(source_format),
                None,
                LegacyImportPolicy::default(),
                CompressionProfile::Fast,
            )
            .unwrap();
            assert_eq!(archive.descriptor.lai, imported.archive.descriptor.lai);
            for entry in archive.entry_set.entries() {
                if let EntryData::File { content } = entry.data() {
                    let imported_entry = imported
                        .archive
                        .entry_set
                        .entries()
                        .iter()
                        .find(|candidate| candidate.path() == entry.path())
                        .unwrap();
                    let EntryData::File {
                        content: imported_content,
                    } = imported_entry.data()
                    else {
                        panic!("file became directory")
                    };
                    assert_eq!(
                        logical_content(&archive, *content).unwrap(),
                        logical_content(&imported.archive, *imported_content).unwrap()
                    );
                }
            }
        }
    }

    #[test]
    fn compressed_tar_profiles_are_deterministic_compositions() {
        let archive = test_archive(second_timestamp());
        let inner = prepare_export(
            &archive,
            ExportTarget::TarPaxV1,
            ExportSourceSecurity::default(),
        )
        .unwrap()
        .accept(false)
        .unwrap();
        for (target, transport, prefix) in [
            (
                ExportTarget::TarGzipPaxV1,
                TransportFormat::Gzip,
                b"\x1f\x8b".as_slice(),
            ),
            (
                ExportTarget::TarZstandardPaxV1,
                TransportFormat::Zstandard,
                b"\x28\xb5\x2f\xfd".as_slice(),
            ),
            (
                ExportTarget::TarXzPaxV1,
                TransportFormat::Xz,
                b"\xfd7zXZ\0".as_slice(),
            ),
            (
                ExportTarget::TarBzip2PaxV1,
                TransportFormat::Bzip2,
                b"BZh9".as_slice(),
            ),
        ] {
            let first = prepare_export(&archive, target, ExportSourceSecurity::default())
                .unwrap()
                .accept(false)
                .unwrap();
            let second = prepare_export(&archive, target, ExportSourceSecurity::default())
                .unwrap()
                .accept(false)
                .unwrap();
            assert_eq!(first.bytes, second.bytes);
            assert!(first.bytes.starts_with(prefix));
            assert!(first.receipt.strict_reimport_validated);
            assert_eq!(first.receipt.reimport_lai, Some(archive.descriptor.lai));
            let wrapped = first.receipt.wrapped_target.as_ref().unwrap();
            assert_eq!(wrapped.semantic_profile, TAR_PROFILE);
            assert_eq!(wrapped.inner_tar_sha256, sha256_exact(&inner.bytes));
            assert_eq!(
                stream::decode(&first.bytes, transport, WrapperImportPolicy::default())
                    .unwrap()
                    .decoded
                    .as_ref(),
                inner.bytes.as_slice()
            );
            let receipt = String::from_utf8(first.receipt.to_canonical_json()).unwrap();
            assert!(receipt.contains(RECEIPT_V2_FORMAT));
            assert!(receipt.contains(target.profile_id()));
        }
    }

    #[test]
    fn empty_archive_has_canonical_target_bytes() {
        let archive = plan_observed_archive(
            Vec::new(),
            Vec::new(),
            FidelityReport::default(),
            conversion(),
            None,
            CompressionProfile::Fast,
        )
        .unwrap();
        for (target, source_format, expected_len) in [
            (ExportTarget::ZipPortableV1, LegacySourceFormat::Zip, 22),
            (ExportTarget::TarPaxV1, LegacySourceFormat::Tar, 1024),
        ] {
            let artifact = prepare_export(&archive, target, ExportSourceSecurity::default())
                .unwrap()
                .accept(false)
                .unwrap();
            assert_eq!(artifact.bytes.len(), expected_len);
            let imported = import_legacy_strict(
                &artifact.bytes,
                Some(source_format),
                None,
                LegacyImportPolicy::default(),
                CompressionProfile::Fast,
            )
            .unwrap();
            assert!(imported.archive.entry_set.is_empty());
            assert_eq!(archive.descriptor.lai, imported.archive.descriptor.lai);
        }
    }

    #[test]
    fn zip_measures_store_and_deflate_and_zip64_threshold_is_canonical() {
        let archive = test_archive(second_timestamp());
        let artifact = prepare_export(
            &archive,
            ExportTarget::ZipPortableV1,
            ExportSourceSecurity::default(),
        )
        .unwrap()
        .accept(false)
        .unwrap();
        assert!(
            artifact
                .bytes
                .windows(2)
                .any(|value| value == ZIP_DEFLATE.to_le_bytes())
        );
        assert!(
            artifact
                .bytes
                .windows(2)
                .any(|value| value == ZIP_STORE.to_le_bytes())
        );
        let mut end = Vec::new();
        write_zip_end(&mut end, ZIP_U16_SENTINEL, 0, 0).unwrap();
        assert!(end.starts_with(&0x0606_4b50_u32.to_le_bytes()));
        assert!(
            end.windows(4)
                .any(|value| value == 0x0706_4b50_u32.to_le_bytes())
        );
    }

    #[test]
    fn lossy_requires_approval_and_receipt_is_canonical() {
        let timestamp = Timestamp::new(
            1_700_000_000,
            123_456_789,
            TimestampPrecision::Nanosecond,
            true,
        )
        .unwrap();
        let prepared = prepare_export(
            &test_archive(timestamp),
            ExportTarget::ZipPortableV1,
            ExportSourceSecurity {
                encrypted: true,
                embedded_signature_count: 2,
                ..ExportSourceSecurity::default()
            },
        )
        .unwrap();
        assert_eq!(prepared.analysis.outcome, ExportOutcome::Lossy);
        assert_eq!(
            prepared.clone().accept(false).unwrap_err().code(),
            ReasonCode::LegacyExportLossyApprovalRequired
        );
        let artifact = prepared.accept(true).unwrap();
        let json = artifact.receipt.to_canonical_json();
        let text = std::str::from_utf8(&json).unwrap();
        assert!(text.ends_with("}\n"));
        assert!(text.contains(RECEIPT_FORMAT));
        assert!(!text.contains(RECEIPT_V2_FORMAT));
        assert!(text.contains("\"outcome\":\"LOSSY\""));
        assert!(text.contains("\"encrypted\":true"));
        assert!(text.contains(&artifact.receipt.target_sha256.to_string()));
    }

    #[test]
    fn hostile_paths_are_refused_without_target_bytes() {
        let bytes = b"refused".to_vec();
        let digest = sha256_exact(&bytes);
        let archive = plan_observed_archive(
            vec![Entry::new(
                LogicalPath::from_utf8(["file:stream"]).unwrap(),
                EntryData::File {
                    content: ContentRef::Internal(digest),
                },
                metadata(false, second_timestamp()),
                EntryIdentity::default(),
            )],
            vec![bytes.into_boxed_slice()],
            FidelityReport::default(),
            conversion(),
            None,
            CompressionProfile::Fast,
        )
        .unwrap();
        for target in [
            ExportTarget::ZipPortableV1,
            ExportTarget::TarPaxV1,
            ExportTarget::TarGzipPaxV1,
            ExportTarget::TarZstandardPaxV1,
            ExportTarget::TarXzPaxV1,
            ExportTarget::TarBzip2PaxV1,
        ] {
            let prepared =
                prepare_export(&archive, target, ExportSourceSecurity::default()).unwrap();
            assert_eq!(prepared.analysis.outcome, ExportOutcome::Refused);
            assert!(prepared.bytes.is_none());
            assert_eq!(
                prepared.accept(true).unwrap_err().code(),
                ReasonCode::LegacyExportRefused
            );
        }
    }

    #[test]
    fn frozen_v1_targets_report_new_posix_semantics_without_redefinition() {
        let bytes = b"same logical bytes".to_vec();
        let digest = sha256_exact(&bytes);
        let posix = plan_observed_archive(
            vec![Entry::new(
                LogicalPath::from_utf8(["mode.txt"]).unwrap(),
                EntryData::File {
                    content: ContentRef::Internal(digest),
                },
                MetadataSet::new(vec![
                    MetadataItem::executable(true),
                    MetadataItem::mtime(second_timestamp()),
                    MetadataItem::posix_mode(0o4755),
                    MetadataItem::posix_uid(1000),
                ])
                .unwrap(),
                EntryIdentity::default(),
            )],
            vec![bytes.into_boxed_slice()],
            FidelityReport::default(),
            conversion(),
            None,
            CompressionProfile::Fast,
        )
        .unwrap();
        for target in [ExportTarget::ZipPortableV1, ExportTarget::TarPaxV1] {
            let prepared = prepare_export(&posix, target, ExportSourceSecurity::default()).unwrap();
            assert_eq!(prepared.analysis.outcome, ExportOutcome::Lossy);
            assert!(prepared.analysis.issues.iter().any(|issue| {
                issue.category == ExportIssueCategory::MetadataUnsupported
                    && issue.semantic_field == "posix.mode"
            }));
            assert!(prepared.clone().accept(false).is_err());
            prepared.accept(true).unwrap();
        }

        let symlink = plan_observed_archive(
            vec![Entry::new(
                LogicalPath::from_utf8(["link"]).unwrap(),
                EntryData::Symlink {
                    target: LinkTarget::canonical(b"target".to_vec()).unwrap(),
                },
                MetadataSet::default(),
                EntryIdentity::default(),
            )],
            Vec::new(),
            FidelityReport::default(),
            conversion(),
            None,
            CompressionProfile::Fast,
        )
        .unwrap();
        for target in [ExportTarget::ZipPortableV1, ExportTarget::TarPaxV1] {
            let prepared =
                prepare_export(&symlink, target, ExportSourceSecurity::default()).unwrap();
            assert_eq!(prepared.analysis.outcome, ExportOutcome::Refused);
            assert!(prepared.bytes.is_none());
            assert!(prepared.analysis.issues.iter().any(|issue| {
                issue.category == ExportIssueCategory::EntryKindUnsupported
                    && issue.disposition == ExportDisposition::Refused
            }));
        }
    }

    #[test]
    fn portable_path_collisions_and_backslashes_are_refused() {
        let first = b"first".to_vec();
        let second = b"second".to_vec();
        let first_digest = sha256_exact(&first);
        let second_digest = sha256_exact(&second);
        let collision = plan_observed_archive(
            vec![
                Entry::new(
                    LogicalPath::from_utf8(["Name.txt"]).unwrap(),
                    EntryData::File {
                        content: ContentRef::Internal(first_digest),
                    },
                    metadata(false, second_timestamp()),
                    EntryIdentity::default(),
                ),
                Entry::new(
                    LogicalPath::from_utf8(["name.txt"]).unwrap(),
                    EntryData::File {
                        content: ContentRef::Internal(second_digest),
                    },
                    metadata(false, second_timestamp()),
                    EntryIdentity::default(),
                ),
            ],
            vec![first.into_boxed_slice(), second.into_boxed_slice()],
            FidelityReport::default(),
            conversion(),
            None,
            CompressionProfile::Fast,
        )
        .unwrap();
        let collision = prepare_export(
            &collision,
            ExportTarget::ZipPortableV1,
            ExportSourceSecurity::default(),
        )
        .unwrap();
        assert_eq!(collision.analysis.outcome, ExportOutcome::Refused);
        assert!(
            collision
                .analysis
                .issues
                .iter()
                .any(|issue| issue.category == ExportIssueCategory::PathCollision)
        );

        let bytes = b"backslash".to_vec();
        let digest = sha256_exact(&bytes);
        let backslash = plan_observed_archive(
            vec![Entry::new(
                LogicalPath::from_utf8([r"unsafe\name"]).unwrap(),
                EntryData::File {
                    content: ContentRef::Internal(digest),
                },
                metadata(false, second_timestamp()),
                EntryIdentity::default(),
            )],
            vec![bytes.into_boxed_slice()],
            FidelityReport::default(),
            conversion(),
            None,
            CompressionProfile::Fast,
        )
        .unwrap();
        for target in [
            ExportTarget::ZipPortableV1,
            ExportTarget::TarPaxV1,
            ExportTarget::TarGzipPaxV1,
            ExportTarget::TarZstandardPaxV1,
            ExportTarget::TarXzPaxV1,
            ExportTarget::TarBzip2PaxV1,
        ] {
            let prepared =
                prepare_export(&backslash, target, ExportSourceSecurity::default()).unwrap();
            assert_eq!(prepared.analysis.outcome, ExportOutcome::Refused);
            assert!(prepared.bytes.is_none());
        }
    }

    #[test]
    fn long_unicode_paths_use_deterministic_target_framing() {
        let name = "雪".repeat(60);
        let path = LogicalPath::from_utf8([name.as_str()]).unwrap();
        let bytes = b"unicode path".to_vec();
        let digest = sha256_exact(&bytes);
        let archive = plan_observed_archive(
            vec![Entry::new(
                path,
                EntryData::File {
                    content: ContentRef::Internal(digest),
                },
                metadata(false, second_timestamp()),
                EntryIdentity::default(),
            )],
            vec![bytes.into_boxed_slice()],
            FidelityReport::default(),
            conversion(),
            None,
            CompressionProfile::Fast,
        )
        .unwrap();
        for (target, source_format) in [
            (ExportTarget::ZipPortableV1, LegacySourceFormat::Zip),
            (ExportTarget::TarPaxV1, LegacySourceFormat::Tar),
        ] {
            let artifact = prepare_export(&archive, target, ExportSourceSecurity::default())
                .unwrap()
                .accept(false)
                .unwrap();
            let imported = import_legacy_strict(
                &artifact.bytes,
                Some(source_format),
                None,
                LegacyImportPolicy::default(),
                CompressionProfile::Fast,
            )
            .unwrap();
            assert_eq!(archive.descriptor.lai, imported.archive.descriptor.lai);
        }
    }

    #[test]
    fn pax_time_and_record_bytes_are_frozen_and_exact() {
        let timestamp =
            Timestamp::new(-2, 500_000_000, TimestampPrecision::Nanosecond, true).unwrap();
        assert_eq!(pax_timestamp(timestamp), "-1.500000000");
        let values = BTreeMap::from([("mtime", "1700000000.25".to_owned())]);
        assert_eq!(pax_payload(&values).unwrap(), b"23 mtime=1700000000.25\n");
        assert_eq!(split_ustar_path(&[b'a'; 101]), None);
    }

    #[test]
    fn pax_nanoseconds_reimport_without_logical_identity_loss() {
        let timestamp = Timestamp::new(
            1_700_000_000,
            123_456_789,
            TimestampPrecision::Nanosecond,
            true,
        )
        .unwrap();
        let archive = test_archive(timestamp);
        let artifact = prepare_export(
            &archive,
            ExportTarget::TarPaxV1,
            ExportSourceSecurity::default(),
        )
        .unwrap()
        .accept(false)
        .unwrap();
        let imported = import_legacy_strict(
            &artifact.bytes,
            Some(LegacySourceFormat::Tar),
            None,
            LegacyImportPolicy::default(),
            CompressionProfile::Fast,
        )
        .unwrap();
        assert_eq!(archive.descriptor.lai, imported.archive.descriptor.lai);
    }

    #[test]
    fn indexed_and_stream_physical_layouts_export_identically() {
        let archive = test_archive(second_timestamp());
        let dense = test_archive_with_profile(second_timestamp(), CompressionProfile::Dense);
        assert_eq!(archive.descriptor.lai, dense.descriptor.lai);
        let indexed = encode(&archive, WriteOptions::default()).unwrap();
        let indexed = open(&indexed.bytes).unwrap();
        let mut stream = Vec::new();
        encode_stream(&archive, StreamWriteOptions::default(), &mut stream).unwrap();
        let streamed = open_stream_with_limits(
            stream.as_slice(),
            SequentialLimits {
                content: StreamContentPolicy::Retain,
                ..bootstrap_sequential_limits()
            },
        )
        .unwrap();
        for target in [
            ExportTarget::ZipPortableV1,
            ExportTarget::TarPaxV1,
            ExportTarget::TarGzipPaxV1,
            ExportTarget::TarZstandardPaxV1,
            ExportTarget::TarXzPaxV1,
            ExportTarget::TarBzip2PaxV1,
        ] {
            let indexed_target =
                prepare_export(&indexed.archive, target, ExportSourceSecurity::default())
                    .unwrap()
                    .accept(false)
                    .unwrap();
            let streamed_target = prepare_export(
                &streamed.opened.archive,
                target,
                ExportSourceSecurity::default(),
            )
            .unwrap()
            .accept(false)
            .unwrap();
            let dense_target = prepare_export(&dense, target, ExportSourceSecurity::default())
                .unwrap()
                .accept(false)
                .unwrap();
            assert_eq!(indexed_target.bytes, streamed_target.bytes);
            assert_eq!(indexed_target.bytes, dense_target.bytes);
            assert_eq!(
                indexed_target.receipt.target_sha256,
                streamed_target.receipt.target_sha256
            );
        }
    }
}
