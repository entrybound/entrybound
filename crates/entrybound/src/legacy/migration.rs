//! Aggregate legacy migration planning over one verified EAM.
//!
//! This module is intentionally free of filesystem transaction logic. It
//! prepares deterministic artifacts and one canonical report; callers decide
//! where an all-or-nothing publication transaction is committed.

use std::collections::BTreeSet;
use std::fmt::Write as _;

use super::export::{
    ExportArtifact, ExportIssue, ExportOutcome, ExportSourceSecurity, ExportTarget, prepare_export,
};
use super::import::LegacyImportResult;
use crate::diagnostics::Result;
use crate::diagnostics::{Diagnostic, OutcomeClass, ReasonCode};
use crate::eam::{Archive, Digest, Layout};
use crate::ecf::{
    SequentialLimits, StreamContentPolicy, StreamWriteOptions, WriteOptions,
    bootstrap_sequential_limits, encode, encode_stream, open, open_stream_with_limits,
};
use crate::identity::sha256_exact;

pub const MIGRATION_REPORT_FORMAT: &str = "entrybound/migration-report-v1";
pub const MIGRATION_REPORT_VERSION: u16 = 1;

/// Aggregate readiness/result for the complete requested artifact set.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MigrationOutcome {
    Ready,
    LossyApprovalRequired,
    Refused,
    Published,
    Failed,
}

impl MigrationOutcome {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "READY",
            Self::LossyApprovalRequired => "LOSSY_APPROVAL_REQUIRED",
            Self::Refused => "REFUSED",
            Self::Published => "PUBLISHED",
            Self::Failed => "FAILED",
        }
    }
}

/// Native artifact relation in a migration transaction.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeArtifactReport {
    pub output_path: String,
    pub relation: String,
    pub byte_length: u64,
    pub sha256: Digest,
    pub produced: bool,
}

/// One planned or published legacy artifact.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MigrationTargetReport {
    pub target: ExportTarget,
    pub outcome: ExportOutcome,
    pub issues: Box<[ExportIssue]>,
    pub lossy_approved: bool,
    pub output_path: String,
    pub produced: bool,
    pub artifact_byte_length: Option<u64>,
    pub artifact_sha256: Option<Digest>,
    pub strict_reimport_validated: bool,
    pub reimport_lai: Option<Digest>,
    pub relation_to_native: String,
}

/// Optional inverse-migration details for a legacy-to-Entrybound sidecar.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SidecarMigrationReport {
    pub source_format: String,
    pub source_sha256: Digest,
    pub import_mode: String,
    pub compatibility_profile: Option<String>,
    pub conflict_count: u64,
    pub resolution_count: u64,
    pub exact_source_preserved: bool,
    pub sidecar_path: String,
    pub sidecar_byte_length: u64,
    pub sidecar_sha256: Digest,
    pub verification_succeeded: bool,
}

/// Deterministic aggregate report. Target records are ordered by exact profile
/// ID, independent of command-line order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MigrationReportV1 {
    pub source_kind: String,
    pub source_lai: Digest,
    pub source_aux: Digest,
    pub source_pcr: Digest,
    pub source_security: ExportSourceSecurity,
    pub source_has_conversion_evidence: bool,
    pub source_has_preserved_evidence: bool,
    pub requested_targets: Box<[MigrationTargetReport]>,
    pub native_artifact: Option<NativeArtifactReport>,
    pub sidecar: Option<SidecarMigrationReport>,
    pub overall_outcome: MigrationOutcome,
}

impl MigrationReportV1 {
    /// Canonical UTF-8 JSON: fixed field order, sorted target records, compact
    /// encoding, and one trailing newline.
    #[must_use]
    pub fn to_canonical_json(&self) -> Vec<u8> {
        let mut output = String::new();
        output.push('{');
        string_pair(&mut output, "format", MIGRATION_REPORT_FORMAT);
        let _ = write!(output, "\"version\":{MIGRATION_REPORT_VERSION},");
        string_pair(&mut output, "source_kind", &self.source_kind);
        string_pair(&mut output, "source_lai", &self.source_lai.to_string());
        string_pair(&mut output, "source_aux", &self.source_aux.to_string());
        string_pair(&mut output, "source_pcr", &self.source_pcr.to_string());
        let _ = write!(
            output,
            "\"source_security\":{{\"encrypted\":{},\"embedded_signature_count\":{},\"detached_signature_count\":{},\"signatures_valid\":{},\"signatures_invalid\":{},\"signatures_stale\":{}}},",
            self.source_security.encrypted,
            self.source_security.embedded_signature_count,
            self.source_security.detached_signature_count,
            self.source_security.signatures_valid,
            self.source_security.signatures_invalid,
            self.source_security.signatures_stale,
        );
        let _ = write!(
            output,
            "\"source_evidence\":{{\"conversion_provenance\":{},\"exact_preserved_source\":{}}},",
            self.source_has_conversion_evidence, self.source_has_preserved_evidence
        );
        output.push_str("\"requested_targets\":[");
        for (index, target) in self.requested_targets.iter().enumerate() {
            if index != 0 {
                output.push(',');
            }
            write_target(&mut output, target);
        }
        output.push_str("],\"native_artifact\":");
        match &self.native_artifact {
            Some(native) => {
                output.push('{');
                string_pair(&mut output, "output_path", &native.output_path);
                string_pair(&mut output, "relation", &native.relation);
                let _ = write!(
                    output,
                    "\"byte_length\":{},\"sha256\":\"{}\",\"produced\":{}",
                    native.byte_length, native.sha256, native.produced
                );
                output.push('}');
            }
            None => output.push_str("null"),
        }
        output.push_str(",\"sidecar\":");
        match &self.sidecar {
            Some(sidecar) => write_sidecar(&mut output, sidecar),
            None => output.push_str("null"),
        }
        output.push(',');
        string_pair(
            &mut output,
            "overall_publish_outcome",
            self.overall_outcome.as_str(),
        );
        output.pop();
        output.push_str("}\n");
        output.into_bytes()
    }
}

/// Prepared multi-target migration over exactly one source EAM.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreparedMigration {
    pub report: MigrationReportV1,
    pub artifacts: Box<[(ExportTarget, ExportArtifact)]>,
}

/// Fully encoded and independently reopened sidecar bytes. Filesystem callers
/// may publish this value transactionally without re-running import policy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreparedSidecar {
    pub bytes: Vec<u8>,
    pub verified_archive: Archive,
    pub source_digest: Digest,
}

impl PreparedMigration {
    #[must_use]
    pub fn is_ready(&self) -> bool {
        self.report.overall_outcome == MigrationOutcome::Ready
    }

    /// Marks the report as successfully committed without changing artifact
    /// identities or target order.
    pub fn mark_published(&mut self) {
        self.report.overall_outcome = MigrationOutcome::Published;
        for target in &mut self.report.requested_targets {
            target.produced = true;
        }
    }
}

/// Runs every target analyzer/encoder/strict-reimport validator before callers
/// create output files. Exact duplicate target requests are collapsed.
pub fn prepare_migration(
    archive: &Archive,
    targets: &[ExportTarget],
    source_security: ExportSourceSecurity,
    allow_lossy: bool,
) -> Result<PreparedMigration> {
    archive.validate()?;
    let mut unique = BTreeSet::new();
    unique.extend(targets.iter().copied());
    let mut selected = unique.into_iter().collect::<Vec<_>>();
    selected.sort_by_key(|target| target.profile_id());

    let mut target_reports = Vec::with_capacity(selected.len());
    let mut artifacts = Vec::with_capacity(selected.len());
    let mut overall = MigrationOutcome::Ready;
    for target in selected {
        let prepared = prepare_export(archive, target, source_security)?;
        let analysis = prepared.analysis.clone();
        let acceptable = match analysis.outcome {
            ExportOutcome::Lossless => true,
            ExportOutcome::Lossy if allow_lossy => true,
            ExportOutcome::Lossy => {
                if overall != MigrationOutcome::Refused {
                    overall = MigrationOutcome::LossyApprovalRequired;
                }
                false
            }
            ExportOutcome::Refused => {
                overall = MigrationOutcome::Refused;
                false
            }
        };
        if acceptable {
            artifacts.push((target, prepared.accept(allow_lossy)?));
        }
        target_reports.push(MigrationTargetReport {
            target,
            outcome: analysis.outcome,
            issues: analysis.issues,
            lossy_approved: analysis.outcome != ExportOutcome::Lossy || allow_lossy,
            output_path: String::new(),
            produced: false,
            artifact_byte_length: analysis.planned_target_bytes,
            artifact_sha256: analysis.planned_target_digest,
            strict_reimport_validated: analysis.strict_reimport_validated,
            reimport_lai: analysis.reimport_lai,
            relation_to_native: "same-source-eam".to_owned(),
        });
    }
    if overall != MigrationOutcome::Ready {
        artifacts.clear();
    }
    Ok(PreparedMigration {
        report: MigrationReportV1 {
            source_kind: "entrybound".to_owned(),
            source_lai: archive.descriptor.lai,
            source_aux: archive.descriptor.aux,
            source_pcr: archive.descriptor.pcr,
            source_security,
            source_has_conversion_evidence: archive.conversion.is_some(),
            source_has_preserved_evidence: archive.preservation.is_some(),
            requested_targets: target_reports.into_boxed_slice(),
            native_artifact: None,
            sidecar: None,
            overall_outcome: overall,
        },
        artifacts: artifacts.into_boxed_slice(),
    })
}

/// Encodes one already-reconciled legacy import as INDEXED or STREAM, reopens
/// it through the matching reader, and verifies that ConversionProvenance binds
/// the exact legacy source snapshot. The source artifact is never mutated.
pub fn prepare_sidecar(
    imported: &LegacyImportResult,
    exact_source: &[u8],
    layout: Layout,
) -> Result<PreparedSidecar> {
    imported.archive.validate()?;
    let bytes = match layout {
        Layout::Indexed => encode(&imported.archive, WriteOptions::default())?.bytes,
        Layout::Stream => {
            let mut bytes = Vec::new();
            encode_stream(&imported.archive, StreamWriteOptions::default(), &mut bytes)?;
            bytes
        }
    };
    let verified_archive = match layout {
        Layout::Indexed => open(&bytes)?.archive,
        Layout::Stream => {
            let limits = SequentialLimits {
                content: StreamContentPolicy::Retain,
                ..bootstrap_sequential_limits()
            };
            open_stream_with_limits(std::io::Cursor::new(&bytes), limits)?
                .opened
                .archive
        }
    };
    let source_digest = sha256_exact(exact_source);
    let provenance = verified_archive.conversion.as_ref().ok_or_else(|| {
        Diagnostic::new(
            OutcomeClass::Corrupt,
            ReasonCode::AuxMismatch,
            "sidecar verification found no ConversionProvenance",
        )
    })?;
    if provenance.source_digest != source_digest {
        return Err(Diagnostic::new(
            OutcomeClass::Corrupt,
            ReasonCode::AuxMismatch,
            "sidecar ConversionProvenance does not bind the exact legacy source",
        ));
    }
    if verified_archive.descriptor.lai != imported.archive.descriptor.lai
        || verified_archive.descriptor.aux != imported.archive.descriptor.aux
        || verified_archive.descriptor.pcr != imported.archive.descriptor.pcr
    {
        return Err(Diagnostic::new(
            OutcomeClass::Corrupt,
            ReasonCode::LaiMismatch,
            "verified sidecar identities differ from its imported EAM",
        ));
    }
    Ok(PreparedSidecar {
        bytes,
        verified_archive,
        source_digest,
    })
}

fn write_target(output: &mut String, target: &MigrationTargetReport) {
    output.push('{');
    string_pair(output, "target_profile", target.target.profile_id());
    string_pair(output, "outcome", target.outcome.as_str());
    let _ = write!(output, "\"lossy_approved\":{},", target.lossy_approved);
    string_pair(output, "output_path", &target.output_path);
    let _ = write!(output, "\"produced\":{},", target.produced);
    option_u64(output, "artifact_byte_length", target.artifact_byte_length);
    option_digest(output, "artifact_sha256", target.artifact_sha256);
    let _ = write!(
        output,
        "\"strict_reimport_validated\":{},",
        target.strict_reimport_validated
    );
    option_digest(output, "reimport_lai", target.reimport_lai);
    string_pair(output, "relation_to_native", &target.relation_to_native);
    output.push_str("\"target_encrypted\":false,\"entrybound_signatures_embedded\":false,");
    output.push_str("\"issues\":[");
    for (index, issue) in target.issues.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        output.push('{');
        string_pair(output, "category", issue.category.as_str());
        match &issue.entry {
            Some(path) => string_pair(output, "entry", &path.to_string()),
            None => output.push_str("\"entry\":null,"),
        }
        string_pair(output, "semantic_field", &issue.semantic_field);
        string_pair(output, "source_value", &issue.source_value);
        string_pair(output, "target_capability", &issue.target_capability);
        string_pair(output, "disposition", issue.disposition.as_str());
        string_pair(output, "reason", &issue.reason);
        output.pop();
        output.push('}');
    }
    output.push_str("]}");
}

fn write_sidecar(output: &mut String, sidecar: &SidecarMigrationReport) {
    output.push('{');
    string_pair(output, "source_format", &sidecar.source_format);
    string_pair(output, "source_sha256", &sidecar.source_sha256.to_string());
    string_pair(output, "import_mode", &sidecar.import_mode);
    match &sidecar.compatibility_profile {
        Some(profile) => string_pair(output, "compatibility_profile", profile),
        None => output.push_str("\"compatibility_profile\":null,"),
    }
    let _ = write!(
        output,
        "\"conflict_count\":{},\"resolution_count\":{},\"exact_source_preserved\":{},",
        sidecar.conflict_count, sidecar.resolution_count, sidecar.exact_source_preserved
    );
    string_pair(output, "sidecar_path", &sidecar.sidecar_path);
    let _ = write!(
        output,
        "\"sidecar_byte_length\":{},\"sidecar_sha256\":\"{}\",\"verification_succeeded\":{}",
        sidecar.sidecar_byte_length, sidecar.sidecar_sha256, sidecar.verification_succeeded
    );
    output.push('}');
}

fn option_u64(output: &mut String, key: &str, value: Option<u64>) {
    match value {
        Some(value) => {
            let _ = write!(output, "\"{key}\":{value},");
        }
        None => {
            let _ = write!(output, "\"{key}\":null,");
        }
    }
}

fn option_digest(output: &mut String, key: &str, value: Option<Digest>) {
    match value {
        Some(value) => {
            let _ = write!(output, "\"{key}\":\"{value}\",");
        }
        None => {
            let _ = write!(output, "\"{key}\":null,");
        }
    }
}

fn string_pair(output: &mut String, key: &str, value: &str) {
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
        ConversionProvenance, Entry, EntryData, EntryIdentity, FidelityReport, LogicalPath,
        MetadataItem, MetadataSet, Timestamp, TimestampPrecision,
    };
    use crate::identity::sha256_exact;
    use crate::planner::CompressionProfile;

    #[test]
    fn target_order_is_canonical_and_duplicate_requests_collapse() {
        let archive = plan_observed_archive(
            vec![Entry::new(
                LogicalPath::from_utf8(["docs"]).unwrap(),
                EntryData::Directory,
                MetadataSet::new(vec![
                    MetadataItem::executable(true),
                    MetadataItem::mtime(
                        Timestamp::new(1_700_000_000, 0, TimestampPrecision::Second, true).unwrap(),
                    ),
                ])
                .unwrap(),
                EntryIdentity::default(),
            )],
            Vec::new(),
            FidelityReport::default(),
            ConversionProvenance {
                source_format: "test".to_owned(),
                adapter_id: "entrybound/migration-test-v1".to_owned(),
                source_digest: sha256_exact(b"test"),
                import_mode: "strict".to_owned(),
                source_entry_count: 1,
                observation_count: 0,
                omission_count: 0,
                refinement_count: 0,
                divergence_count: 0,
                irreconcilable_count: 0,
                resolutions: Box::default(),
                synthesized_ancestors: Box::default(),
                unsupported_metadata: Box::default(),
                outcome: "success".to_owned(),
            },
            None,
            CompressionProfile::Fast,
        )
        .unwrap();
        let first = prepare_migration(
            &archive,
            &[
                ExportTarget::TarZstandardPaxV1,
                ExportTarget::ZipPortableV1,
                ExportTarget::TarZstandardPaxV1,
            ],
            ExportSourceSecurity::default(),
            true,
        )
        .unwrap();
        let second = prepare_migration(
            &archive,
            &[ExportTarget::ZipPortableV1, ExportTarget::TarZstandardPaxV1],
            ExportSourceSecurity::default(),
            true,
        )
        .unwrap();
        assert_eq!(first.artifacts, second.artifacts);
        assert_eq!(
            first.report.to_canonical_json(),
            second.report.to_canonical_json()
        );
        assert!(first.is_ready());
        assert_eq!(first.artifacts.len(), 2);
    }
}
