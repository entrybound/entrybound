//! Native archive tooling whose reports preserve the LAI/AUX/PCR/PCI layers.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::io::Cursor;

use super::{inspect, replan_archive};
use crate::diagnostics::{Diagnostic, OutcomeClass, ReasonCode, Result};
use crate::eam::{Archive, ContentRef, Digest, EntryData, Layout, MetadataValue};
use crate::ecf::{
    EncodedArchive, IdentityVerificationStatus, IndexStatus, OpenedArchive, RandomAccessMetadata,
    RandomAccessVerificationReport, StreamContentPolicy, StreamWindow, StreamWriteOptions,
    WriteOptions, bootstrap_sequential_limits, encode, encode_stream, open,
    open_stream_with_limits,
};
use crate::identity::sha256_exact;
use crate::planner::CompressionProfile;

pub const INSPECTION_FORMAT: &str = "entrybound/inspection-v1";
pub const DIFF_FORMAT: &str = "entrybound/archive-diff-v1";

/// Whether a repack retains or deliberately rebuilds the physical plan.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RepackMode {
    RepresentationOnly,
    Replan(CompressionProfile),
}

/// Policy for the reconstructible INDEXED locator cache.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IndexPolicy {
    Preserve,
    Present,
    Absent,
}

/// Complete caller choice for a native-to-native repack.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RepackOptions {
    pub mode: RepackMode,
    pub layout: Layout,
    pub index: IndexPolicy,
    pub stream_window: StreamWindow,
}

/// Prospective and verified physical comparison for a repack.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepackAnalysis {
    pub mode: RepackMode,
    pub source_layout: Layout,
    pub target_layout: Layout,
    pub source_planner: String,
    pub target_planner: String,
    pub source_chunk_count: u64,
    pub target_chunk_count: u64,
    pub source_unique_chunk_count: u64,
    pub target_unique_chunk_count: u64,
    pub source_stored_bytes: u64,
    pub target_stored_bytes: u64,
    pub source_working_set_bytes: u64,
    pub target_working_set_bytes: u64,
    pub source_dictionary_count: u64,
    pub target_dictionary_count: u64,
    pub source_group_count: u64,
    pub target_group_count: u64,
    pub source_region_count: u64,
    pub target_region_count: u64,
    pub source_pcr: Digest,
    pub target_pcr: Digest,
    pub output_bytes: u64,
    pub lai_equal: bool,
    pub aux_equal: bool,
    pub pcr_equal: bool,
}

/// Fully encoded and reopened repack output, ready for staged publication.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreparedRepack {
    pub encoded: EncodedArchive,
    pub verified: OpenedArchive,
    pub analysis: RepackAnalysis,
}

/// Prepares and fully verifies a native repack without touching the filesystem.
pub fn prepare_repack(source: &OpenedArchive, options: RepackOptions) -> Result<PreparedRepack> {
    if source.archive.descriptor.features.incompat & crate::crypto::FEATURE_ENCRYPTED_INDEXED_V1
        != 0
    {
        return Err(Diagnostic::new(
            OutcomeClass::Unsupported,
            ReasonCode::CryptoLayoutUnsupported,
            "encrypted repack is deferred; decrypting to an unencrypted output is never implicit",
        ));
    }
    if options.layout == Layout::Stream && options.index != IndexPolicy::Preserve {
        return Err(Diagnostic::new(
            OutcomeClass::Nonconforming,
            ReasonCode::CommandUsage,
            "STREAM has no Index; --index present/absent is invalid",
        ));
    }
    let mut target = match options.mode {
        RepackMode::RepresentationOnly => source.archive.clone(),
        RepackMode::Replan(profile) => replan_archive(&source.archive, profile)?,
    };
    target.descriptor.pci = None;
    let include_index = match options.index {
        IndexPolicy::Present => true,
        IndexPolicy::Absent => false,
        IndexPolicy::Preserve => {
            source.archive.descriptor.layout == Layout::Stream
                || source.report.index_status != IndexStatus::RebuiltAbsent
        }
    };
    let (encoded, verified) = match options.layout {
        Layout::Indexed => {
            target.descriptor.layout = Layout::Indexed;
            target.descriptor.features.incompat &= !crate::ecf::FEATURE_STREAM_LAYOUT_V1;
            target.descriptor.stream_dedup_window = 0;
            let encoded = encode(&target, WriteOptions { include_index })?;
            let verified = open(&encoded.bytes)?;
            (encoded, verified)
        }
        Layout::Stream => {
            let mut bytes = Vec::new();
            let budget_declared = !(options.mode == RepackMode::RepresentationOnly
                && source.archive.descriptor.layout == Layout::Stream
                && !source.archive.descriptor.budget_declared);
            let summary = encode_stream(
                &target,
                StreamWriteOptions {
                    window: options.stream_window,
                    budget_declared,
                },
                &mut bytes,
            )?;
            let mut limits = bootstrap_sequential_limits();
            limits.content = StreamContentPolicy::Retain;
            let verified = open_stream_with_limits(Cursor::new(bytes.clone()), limits)?.opened;
            let encoded = EncodedArchive {
                bytes,
                archive: summary.archive,
                identities: summary.identities,
            };
            (encoded, verified)
        }
    };
    let lai_equal = source.report.identities.lai == verified.report.identities.lai;
    let aux_equal = source.report.identities.aux == verified.report.identities.aux;
    let pcr_equal = source.report.identities.pcr == verified.report.identities.pcr;
    if !lai_equal || !aux_equal || (options.mode == RepackMode::RepresentationOnly && !pcr_equal) {
        return Err(Diagnostic::new(
            OutcomeClass::Corrupt,
            if !lai_equal {
                ReasonCode::LaiMismatch
            } else if !aux_equal {
                ReasonCode::AuxMismatch
            } else {
                ReasonCode::PcrMismatch
            },
            "post-write repack identity invariant failed",
        ));
    }
    let (source_chunk_count, source_unique_chunk_count, source_stored_bytes) =
        physical_metrics(source)?;
    let (target_chunk_count, target_unique_chunk_count, target_stored_bytes) =
        physical_metrics(&verified)?;
    let analysis = RepackAnalysis {
        mode: options.mode,
        source_layout: source.archive.descriptor.layout,
        target_layout: options.layout,
        source_planner: source.archive.descriptor.planner_id.clone(),
        target_planner: verified.archive.descriptor.planner_id.clone(),
        source_chunk_count,
        target_chunk_count,
        source_unique_chunk_count,
        target_unique_chunk_count,
        source_stored_bytes,
        target_stored_bytes,
        source_working_set_bytes: source.archive.descriptor.decode.working_set_bytes,
        target_working_set_bytes: verified.archive.descriptor.decode.working_set_bytes,
        source_dictionary_count: source.archive.content_store.dictionaries.len() as u64,
        target_dictionary_count: verified.archive.content_store.dictionaries.len() as u64,
        source_group_count: source.archive.content_store.chunk_groups.len() as u64,
        target_group_count: verified.archive.content_store.chunk_groups.len() as u64,
        source_region_count: source.archive.content_store.reconstruction_regions.len() as u64,
        target_region_count: verified.archive.content_store.reconstruction_regions.len() as u64,
        source_pcr: source.report.identities.pcr.0,
        target_pcr: verified.report.identities.pcr.0,
        output_bytes: encoded.bytes.len() as u64,
        lai_equal,
        aux_equal,
        pcr_equal,
    };
    Ok(PreparedRepack {
        encoded,
        verified,
        analysis,
    })
}

fn physical_metrics(opened: &OpenedArchive) -> Result<(u64, u64, u64)> {
    let logical_references =
        opened
            .archive
            .content_store
            .objects
            .values()
            .try_fold(0_u64, |total, object| {
                total
                    .checked_add(u64::try_from(object.chunks.len()).map_err(|_| {
                        Diagnostic::new(
                            OutcomeClass::PolicyRefused,
                            ReasonCode::ResourceLimit,
                            "Chunk reference count exceeds u64",
                        )
                    })?)
                    .ok_or_else(|| {
                        Diagnostic::new(
                            OutcomeClass::PolicyRefused,
                            ReasonCode::ResourceLimit,
                            "Chunk reference count exceeds u64",
                        )
                    })
            })?;
    let unique = u64::try_from(opened.archive.content_store.chunks.len()).map_err(|_| {
        Diagnostic::new(
            OutcomeClass::PolicyRefused,
            ReasonCode::ResourceLimit,
            "unique Chunk count exceeds u64",
        )
    })?;
    let stored = opened
        .archive
        .index
        .chunks
        .values()
        .try_fold(0_u64, |total, location| {
            total.checked_add(location.stored_len).ok_or_else(|| {
                Diagnostic::new(
                    OutcomeClass::PolicyRefused,
                    ReasonCode::ResourceLimit,
                    "stored Chunk byte total exceeds u64",
                )
            })
        })?;
    Ok((logical_references, unique, stored))
}

/// Native identity/evidence tier for a reported archive change.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum DiffTier {
    Semantic,
    Auxiliary,
    Physical,
    Container,
}

impl DiffTier {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Semantic => "SEMANTIC",
            Self::Auxiliary => "AUXILIARY",
            Self::Physical => "PHYSICAL",
            Self::Container => "CONTAINER",
        }
    }
}

/// Verification-aware equality status for one identity layer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DiffIdentityStatus {
    Same,
    Different,
    NotVerified,
    NotComputed,
}

impl DiffIdentityStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Same => "SAME",
            Self::Different => "DIFFERENT",
            Self::NotVerified => "NOT_VERIFIED",
            Self::NotComputed => "NOT_COMPUTED",
        }
    }
}

/// One canonical, machine-readable difference.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct DiffChange {
    pub tier: DiffTier,
    pub subject: String,
    pub field: String,
    pub left: Option<String>,
    pub right: Option<String>,
}

/// Versioned four-tier diff report.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArchiveDiffReport {
    pub lai: DiffIdentityStatus,
    pub aux: DiffIdentityStatus,
    pub pcr: DiffIdentityStatus,
    pub pci: DiffIdentityStatus,
    pub interpretation: String,
    pub left_scope: String,
    pub right_scope: String,
    pub physical_summary: Option<PhysicalDiffSummary>,
    pub changes: Box<[DiffChange]>,
}

/// Digest- and reference-based physical change totals.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PhysicalDiffSummary {
    pub chunks_reused: u64,
    pub chunks_added: u64,
    pub chunks_removed: u64,
    pub content_objects_with_boundary_changes: u64,
}

impl ArchiveDiffReport {
    #[must_use]
    pub fn to_canonical_json(&self) -> Vec<u8> {
        let mut out = String::new();
        out.push('{');
        json_pair(&mut out, "format", DIFF_FORMAT);
        out.push_str("\"version\":1,");
        json_pair(&mut out, "lai", self.lai.as_str());
        json_pair(&mut out, "aux", self.aux.as_str());
        json_pair(&mut out, "pcr", self.pcr.as_str());
        json_pair(&mut out, "pci", self.pci.as_str());
        json_pair(&mut out, "interpretation", &self.interpretation);
        json_pair(&mut out, "left_scope", &self.left_scope);
        json_pair(&mut out, "right_scope", &self.right_scope);
        out.push_str("\"physical_summary\":");
        match self.physical_summary {
            Some(summary) => {
                let _ = write!(
                    out,
                    "{{\"chunks_reused\":{},\"chunks_added\":{},\"chunks_removed\":{},\"content_objects_with_boundary_changes\":{}}},",
                    summary.chunks_reused,
                    summary.chunks_added,
                    summary.chunks_removed,
                    summary.content_objects_with_boundary_changes
                );
            }
            None => out.push_str("null,"),
        }
        out.push_str("\"changes\":[");
        for (index, change) in self.changes.iter().enumerate() {
            if index != 0 {
                out.push(',');
            }
            out.push('{');
            json_pair(&mut out, "tier", change.tier.as_str());
            json_pair(&mut out, "subject", &change.subject);
            json_pair(&mut out, "field", &change.field);
            json_option(&mut out, "left", change.left.as_deref());
            json_option_last(&mut out, "right", change.right.as_deref());
            out.push('}');
        }
        out.push_str("]}\n");
        out.into_bytes()
    }
}

/// Compares two fully verified EAMs without treating container bytes as content.
pub fn archive_diff(left: &OpenedArchive, right: &OpenedArchive) -> Result<ArchiveDiffReport> {
    let mut changes = Vec::new();
    semantic_changes(&left.archive, &right.archive, &mut changes)?;
    auxiliary_changes(&left.archive, &right.archive, &mut changes);
    physical_changes(&left.archive, &right.archive, &mut changes);
    container_changes(left, right, &mut changes);
    changes.sort();
    let lai = equality(left.report.identities.lai.0, right.report.identities.lai.0);
    let aux = equality(left.report.identities.aux.0, right.report.identities.aux.0);
    let pcr = equality(left.report.identities.pcr.0, right.report.identities.pcr.0);
    let pci = equality(left.report.identities.pci.0, right.report.identities.pci.0);
    let interpretation = if lai == DiffIdentityStatus::Different {
        "semantic content differs"
    } else if aux == DiffIdentityStatus::Different {
        "semantic equivalent; auxiliary evidence differs"
    } else if pcr == DiffIdentityStatus::Different {
        "semantic equivalent; auxiliary equivalent; physical representation differs"
    } else if pci == DiffIdentityStatus::Different {
        "semantic, auxiliary, and physical equivalent; container differs"
    } else {
        "exact verified container identity is equal"
    };
    Ok(ArchiveDiffReport {
        lai,
        aux,
        pcr,
        pci,
        interpretation: interpretation.to_owned(),
        left_scope: "whole archive verified".to_owned(),
        right_scope: "whole archive verified".to_owned(),
        physical_summary: Some(physical_summary(&left.archive, &right.archive)),
        changes: changes.into_boxed_slice(),
    })
}

/// Compares authenticated/verified INDEXED metadata without pretending that
/// unread payload established PCR or PCI.
pub fn archive_metadata_diff(
    left: &RandomAccessMetadata,
    left_report: &RandomAccessVerificationReport,
    right: &RandomAccessMetadata,
    right_report: &RandomAccessVerificationReport,
) -> Result<ArchiveDiffReport> {
    let mut changes = Vec::new();
    semantic_metadata_changes(left, right, &mut changes)?;
    if left.descriptor.planner_id != right.descriptor.planner_id {
        change(
            &mut changes,
            DiffTier::Physical,
            "archive",
            "planner_id",
            Some(&left.descriptor.planner_id),
            Some(&right.descriptor.planner_id),
        );
    }
    if left.descriptor.chunker_id != right.descriptor.chunker_id {
        change(
            &mut changes,
            DiffTier::Physical,
            "archive",
            "chunker_id",
            Some(&left.descriptor.chunker_id),
            Some(&right.descriptor.chunker_id),
        );
    }
    if left.descriptor.layout != right.descriptor.layout {
        change(
            &mut changes,
            DiffTier::Container,
            "archive",
            "layout",
            Some(left.descriptor.layout.as_str()),
            Some(right.descriptor.layout.as_str()),
        );
    }
    if left.descriptor.features != right.descriptor.features {
        change(
            &mut changes,
            DiffTier::Container,
            "archive",
            "feature_bitmap",
            Some(&format!("{:#x}", left.descriptor.features.incompat)),
            Some(&format!("{:#x}", right.descriptor.features.incompat)),
        );
    }
    if left.encrypted != right.encrypted {
        change(
            &mut changes,
            DiffTier::Container,
            "archive",
            "encrypted",
            Some(&left.encrypted.to_string()),
            Some(&right.encrypted.to_string()),
        );
    }
    changes.sort();
    let lai = partial_identity(
        left_report.lai,
        left.descriptor.lai,
        right_report.lai,
        right.descriptor.lai,
    );
    let aux = partial_identity(
        left_report.aux,
        left.descriptor.aux,
        right_report.aux,
        right.descriptor.aux,
    );
    let pcr = DiffIdentityStatus::NotVerified;
    let pci = DiffIdentityStatus::NotComputed;
    let interpretation = if lai == DiffIdentityStatus::Different {
        "semantic content differs"
    } else if lai == DiffIdentityStatus::Same && aux == DiffIdentityStatus::Different {
        "semantic equivalent; auxiliary evidence differs"
    } else {
        "metadata compared; unread physical/container bytes remain unverified"
    };
    Ok(ArchiveDiffReport {
        lai,
        aux,
        pcr,
        pci,
        interpretation: interpretation.to_owned(),
        left_scope: format!(
            "range-backed metadata; {} bytes in {} requests",
            left_report.bytes_fetched, left_report.range_request_count
        ),
        right_scope: format!(
            "range-backed metadata; {} bytes in {} requests",
            right_report.bytes_fetched, right_report.range_request_count
        ),
        physical_summary: None,
        changes: changes.into_boxed_slice(),
    })
}

fn partial_identity(
    left_status: IdentityVerificationStatus,
    left: Digest,
    right_status: IdentityVerificationStatus,
    right: Digest,
) -> DiffIdentityStatus {
    if left_status == IdentityVerificationStatus::Verified
        && right_status == IdentityVerificationStatus::Verified
    {
        equality(left, right)
    } else {
        DiffIdentityStatus::NotVerified
    }
}

fn semantic_metadata_changes(
    left: &RandomAccessMetadata,
    right: &RandomAccessMetadata,
    out: &mut Vec<DiffChange>,
) -> Result<()> {
    let left_entries = left
        .entries
        .entries()
        .iter()
        .map(|entry| (entry.path().to_string(), entry))
        .collect::<BTreeMap<_, _>>();
    let right_entries = right
        .entries
        .entries()
        .iter()
        .map(|entry| (entry.path().to_string(), entry))
        .collect::<BTreeMap<_, _>>();
    let paths = left_entries
        .keys()
        .chain(right_entries.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    for path in paths {
        match (left_entries.get(&path), right_entries.get(&path)) {
            (Some(_), None) => change(
                out,
                DiffTier::Semantic,
                &path,
                "entry",
                Some("present"),
                None,
            ),
            (None, Some(_)) => change(
                out,
                DiffTier::Semantic,
                &path,
                "entry",
                None,
                Some("present"),
            ),
            (Some(left_entry), Some(right_entry)) => {
                if left_entry.kind() != right_entry.kind() {
                    change(
                        out,
                        DiffTier::Semantic,
                        &path,
                        "kind",
                        Some(kind_name(left_entry.kind())),
                        Some(kind_name(right_entry.kind())),
                    );
                }
                metadata_changes(left_entry, right_entry, &path, out);
                if let (
                    EntryData::File {
                        content: ContentRef::Internal(left_digest),
                    },
                    EntryData::File {
                        content: ContentRef::Internal(right_digest),
                    },
                ) = (left_entry.data(), right_entry.data())
                    && left_digest != right_digest
                {
                    change(
                        out,
                        DiffTier::Semantic,
                        &path,
                        "content_digest",
                        Some(&left_digest.to_string()),
                        Some(&right_digest.to_string()),
                    );
                }
                if let (
                    EntryData::Symlink {
                        target: left_target,
                    },
                    EntryData::Symlink {
                        target: right_target,
                    },
                ) = (left_entry.data(), right_entry.data())
                    && left_target != right_target
                {
                    change(
                        out,
                        DiffTier::Semantic,
                        &path,
                        "symlink_target",
                        Some(&format!(
                            "{} bytes sha256:{}",
                            left_target.bytes().len(),
                            sha256_exact(left_target.bytes())
                        )),
                        Some(&format!(
                            "{} bytes sha256:{}",
                            right_target.bytes().len(),
                            sha256_exact(right_target.bytes())
                        )),
                    );
                }
                if let (
                    EntryData::ReparsePoint { value: left_value },
                    EntryData::ReparsePoint { value: right_value },
                ) = (left_entry.data(), right_entry.data())
                    && left_value != right_value
                {
                    change(
                        out,
                        DiffTier::Semantic,
                        &path,
                        "windows_reparse_point",
                        Some(&format!(
                            "tag=0x{:08x} {} bytes sha256:{}",
                            left_value.tag(),
                            left_value.data().len(),
                            sha256_exact(left_value.data())
                        )),
                        Some(&format!(
                            "tag=0x{:08x} {} bytes sha256:{}",
                            right_value.tag(),
                            right_value.data().len(),
                            sha256_exact(right_value.data())
                        )),
                    );
                }
            }
            (None, None) => unreachable!(),
        }
    }
    Ok(())
}

fn equality(left: Digest, right: Digest) -> DiffIdentityStatus {
    if left == right {
        DiffIdentityStatus::Same
    } else {
        DiffIdentityStatus::Different
    }
}

fn semantic_changes(left: &Archive, right: &Archive, out: &mut Vec<DiffChange>) -> Result<()> {
    let left_entries = left
        .entry_set
        .entries()
        .iter()
        .map(|entry| (entry.path().to_string(), entry))
        .collect::<BTreeMap<_, _>>();
    let right_entries = right
        .entry_set
        .entries()
        .iter()
        .map(|entry| (entry.path().to_string(), entry))
        .collect::<BTreeMap<_, _>>();
    let paths = left_entries
        .keys()
        .chain(right_entries.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    for path in paths {
        match (left_entries.get(&path), right_entries.get(&path)) {
            (Some(_), None) => change(
                out,
                DiffTier::Semantic,
                &path,
                "entry",
                Some("present"),
                None,
            ),
            (None, Some(_)) => change(
                out,
                DiffTier::Semantic,
                &path,
                "entry",
                None,
                Some("present"),
            ),
            (Some(left_entry), Some(right_entry)) => {
                if left_entry.kind() != right_entry.kind() {
                    change(
                        out,
                        DiffTier::Semantic,
                        &path,
                        "kind",
                        Some(kind_name(left_entry.kind())),
                        Some(kind_name(right_entry.kind())),
                    );
                }
                metadata_changes(left_entry, right_entry, &path, out);
                if let (
                    EntryData::File {
                        content: ContentRef::Internal(left_digest),
                    },
                    EntryData::File {
                        content: ContentRef::Internal(right_digest),
                    },
                ) = (left_entry.data(), right_entry.data())
                {
                    if left_digest != right_digest {
                        change(
                            out,
                            DiffTier::Semantic,
                            &path,
                            "content_digest",
                            Some(&left_digest.to_string()),
                            Some(&right_digest.to_string()),
                        );
                    }
                    let left_size = object_size(left, *left_digest)?;
                    let right_size = object_size(right, *right_digest)?;
                    if left_size != right_size {
                        change(
                            out,
                            DiffTier::Semantic,
                            &path,
                            "logical_size",
                            Some(&left_size.to_string()),
                            Some(&right_size.to_string()),
                        );
                    }
                }
                if let (
                    EntryData::Symlink {
                        target: left_target,
                    },
                    EntryData::Symlink {
                        target: right_target,
                    },
                ) = (left_entry.data(), right_entry.data())
                    && left_target != right_target
                {
                    change(
                        out,
                        DiffTier::Semantic,
                        &path,
                        "symlink_target",
                        Some(&format!(
                            "{} bytes sha256:{}",
                            left_target.bytes().len(),
                            sha256_exact(left_target.bytes())
                        )),
                        Some(&format!(
                            "{} bytes sha256:{}",
                            right_target.bytes().len(),
                            sha256_exact(right_target.bytes())
                        )),
                    );
                }
                if let (
                    EntryData::ReparsePoint { value: left_value },
                    EntryData::ReparsePoint { value: right_value },
                ) = (left_entry.data(), right_entry.data())
                    && left_value != right_value
                {
                    change(
                        out,
                        DiffTier::Semantic,
                        &path,
                        "windows_reparse_point",
                        Some(&format!(
                            "tag=0x{:08x} {} bytes sha256:{}",
                            left_value.tag(),
                            left_value.data().len(),
                            sha256_exact(left_value.data())
                        )),
                        Some(&format!(
                            "tag=0x{:08x} {} bytes sha256:{}",
                            right_value.tag(),
                            right_value.data().len(),
                            sha256_exact(right_value.data())
                        )),
                    );
                }
            }
            (None, None) => unreachable!(),
        }
    }
    Ok(())
}

fn auxiliary_changes(left: &Archive, right: &Archive, out: &mut Vec<DiffChange>) {
    if left.fidelity != right.fidelity {
        change(
            out,
            DiffTier::Auxiliary,
            "archive",
            "fidelity_report",
            Some(&fidelity_summary(left)),
            Some(&fidelity_summary(right)),
        );
    }
    if left.conversion != right.conversion {
        change(
            out,
            DiffTier::Auxiliary,
            "archive",
            "conversion_provenance",
            left.conversion
                .as_ref()
                .map(|v| v.source_digest.to_string())
                .as_deref(),
            right
                .conversion
                .as_ref()
                .map(|v| v.source_digest.to_string())
                .as_deref(),
        );
    }
    if left.preservation != right.preservation {
        change(
            out,
            DiffTier::Auxiliary,
            "archive",
            "legacy_preservation",
            left.preservation
                .as_ref()
                .map(|v| v.source_digest.to_string())
                .as_deref(),
            right
                .preservation
                .as_ref()
                .map(|v| v.source_digest.to_string())
                .as_deref(),
        );
    }
}

fn physical_changes(left: &Archive, right: &Archive, out: &mut Vec<DiffChange>) {
    if left.descriptor.planner_id != right.descriptor.planner_id {
        change(
            out,
            DiffTier::Physical,
            "archive",
            "planner_id",
            Some(&left.descriptor.planner_id),
            Some(&right.descriptor.planner_id),
        );
    }
    if left.descriptor.chunker_id != right.descriptor.chunker_id {
        change(
            out,
            DiffTier::Physical,
            "archive",
            "chunker_id",
            Some(&left.descriptor.chunker_id),
            Some(&right.descriptor.chunker_id),
        );
    }
    let left_chunks = left
        .content_store
        .chunks
        .keys()
        .copied()
        .collect::<BTreeSet<_>>();
    let right_chunks = right
        .content_store
        .chunks
        .keys()
        .copied()
        .collect::<BTreeSet<_>>();
    if left_chunks != right_chunks {
        change(
            out,
            DiffTier::Physical,
            "archive",
            "chunk_digest_set",
            Some(&format!("{} chunks", left_chunks.len())),
            Some(&format!("{} chunks", right_chunks.len())),
        );
    }
    if left.content_store.physical_order != right.content_store.physical_order {
        change(
            out,
            DiffTier::Physical,
            "archive",
            "chunk_physical_order",
            Some("recorded order"),
            Some("different recorded order"),
        );
    }
    if left.transform_plans != right.transform_plans {
        change(
            out,
            DiffTier::Physical,
            "archive",
            "transform_plans",
            Some(&left.transform_plans.len().to_string()),
            Some(&right.transform_plans.len().to_string()),
        );
    }
    if left.content_store.dictionaries != right.content_store.dictionaries {
        change(
            out,
            DiffTier::Physical,
            "archive",
            "dictionaries",
            Some(&left.content_store.dictionaries.len().to_string()),
            Some(&right.content_store.dictionaries.len().to_string()),
        );
    }
    if left.content_store.chunk_groups != right.content_store.chunk_groups {
        change(
            out,
            DiffTier::Physical,
            "archive",
            "chunk_groups",
            Some(&left.content_store.chunk_groups.len().to_string()),
            Some(&right.content_store.chunk_groups.len().to_string()),
        );
    }
    if left.content_store.reconstruction_data != right.content_store.reconstruction_data {
        change(
            out,
            DiffTier::Physical,
            "archive",
            "reconstruction_data",
            Some(&left.content_store.reconstruction_data.len().to_string()),
            Some(&right.content_store.reconstruction_data.len().to_string()),
        );
    }
    if left.content_store.reconstruction_regions != right.content_store.reconstruction_regions {
        change(
            out,
            DiffTier::Physical,
            "archive",
            "reconstruction_regions",
            Some(&left.content_store.reconstruction_regions.len().to_string()),
            Some(&right.content_store.reconstruction_regions.len().to_string()),
        );
    }
    if left.content_store.reconstruction_fallbacks != right.content_store.reconstruction_fallbacks {
        change(
            out,
            DiffTier::Physical,
            "archive",
            "reconstruction_fallbacks",
            Some(
                &left
                    .content_store
                    .reconstruction_fallbacks
                    .len()
                    .to_string(),
            ),
            Some(
                &right
                    .content_store
                    .reconstruction_fallbacks
                    .len()
                    .to_string(),
            ),
        );
    }
    if left.content_store.reconstruction_audits != right.content_store.reconstruction_audits {
        change(
            out,
            DiffTier::Physical,
            "archive",
            "reconstruction_audits",
            Some(&left.content_store.reconstruction_audits.len().to_string()),
            Some(&right.content_store.reconstruction_audits.len().to_string()),
        );
    }
    for (digest, left_object) in &left.content_store.objects {
        if let Some(right_object) = right.content_store.objects.get(digest)
            && left_object.chunks != right_object.chunks
        {
            change(
                out,
                DiffTier::Physical,
                &digest.to_string(),
                "content_chunk_sequence",
                Some(&left_object.chunks.len().to_string()),
                Some(&right_object.chunks.len().to_string()),
            );
        }
    }
}

fn physical_summary(left: &Archive, right: &Archive) -> PhysicalDiffSummary {
    let left_chunks = left
        .content_store
        .chunks
        .keys()
        .copied()
        .collect::<BTreeSet<_>>();
    let right_chunks = right
        .content_store
        .chunks
        .keys()
        .copied()
        .collect::<BTreeSet<_>>();
    let changed = left
        .content_store
        .objects
        .iter()
        .filter(|(digest, object)| {
            right
                .content_store
                .objects
                .get(digest)
                .is_some_and(|right_object| right_object.chunks != object.chunks)
        })
        .count();
    PhysicalDiffSummary {
        chunks_reused: left_chunks.intersection(&right_chunks).count() as u64,
        chunks_added: right_chunks.difference(&left_chunks).count() as u64,
        chunks_removed: left_chunks.difference(&right_chunks).count() as u64,
        content_objects_with_boundary_changes: changed as u64,
    }
}

fn container_changes(left: &OpenedArchive, right: &OpenedArchive, out: &mut Vec<DiffChange>) {
    if left.report.identities.pci != right.report.identities.pci {
        change(
            out,
            DiffTier::Container,
            "archive",
            "pci",
            Some(&left.report.identities.pci.0.to_string()),
            Some(&right.report.identities.pci.0.to_string()),
        );
    }
    if left.archive.descriptor.layout != right.archive.descriptor.layout {
        change(
            out,
            DiffTier::Container,
            "archive",
            "layout",
            Some(left.archive.descriptor.layout.as_str()),
            Some(right.archive.descriptor.layout.as_str()),
        );
    }
    if left.report.index_status != right.report.index_status {
        change(
            out,
            DiffTier::Container,
            "archive",
            "index_status",
            Some(index_name(left.report.index_status)),
            Some(index_name(right.report.index_status)),
        );
    }
    if left.archive.descriptor.features != right.archive.descriptor.features {
        change(
            out,
            DiffTier::Container,
            "archive",
            "feature_bitmap",
            Some(&format!("{:#x}", left.archive.descriptor.features.incompat)),
            Some(&format!(
                "{:#x}",
                right.archive.descriptor.features.incompat
            )),
        );
    }
}

fn change(
    out: &mut Vec<DiffChange>,
    tier: DiffTier,
    subject: &str,
    field: &str,
    left: Option<&str>,
    right: Option<&str>,
) {
    out.push(DiffChange {
        tier,
        subject: subject.to_owned(),
        field: field.to_owned(),
        left: left.map(str::to_owned),
        right: right.map(str::to_owned),
    });
}

fn kind_name(kind: crate::eam::EntryKind) -> &'static str {
    match kind {
        crate::eam::EntryKind::Directory => "directory",
        crate::eam::EntryKind::File => "file",
        crate::eam::EntryKind::Symlink => "symlink",
        crate::eam::EntryKind::ReparsePoint => "reparse-point",
    }
}
fn index_name(status: IndexStatus) -> &'static str {
    match status {
        IndexStatus::PresentValid => "present-valid",
        IndexStatus::RebuiltAbsent => "rebuilt-absent",
        IndexStatus::RebuiltInvalid => "rebuilt-invalid",
        IndexStatus::NotApplicableStream => "not-applicable-stream",
    }
}

fn metadata_changes(
    left: &crate::eam::Entry,
    right: &crate::eam::Entry,
    path: &str,
    out: &mut Vec<DiffChange>,
) {
    let left_items = left
        .metadata()
        .items()
        .iter()
        .map(|item| (item.name(), item))
        .collect::<BTreeMap<_, _>>();
    let right_items = right
        .metadata()
        .items()
        .iter()
        .map(|item| (item.name(), item))
        .collect::<BTreeMap<_, _>>();
    let names = left_items
        .keys()
        .chain(right_items.keys())
        .copied()
        .collect::<BTreeSet<_>>();
    for name in names {
        let left_item = left_items.get(&name);
        let right_item = right_items.get(&name);
        if left_item != right_item {
            let tier = if name.participates_in_identity_v1() {
                DiffTier::Semantic
            } else {
                DiffTier::Auxiliary
            };
            let left_value = left_item.map(|item| metadata_item_text(item));
            let right_value = right_item.map(|item| metadata_item_text(item));
            change(
                out,
                tier,
                path,
                name.as_str(),
                left_value.as_deref(),
                right_value.as_deref(),
            );
        }
    }
}

fn metadata_item_text(item: &crate::eam::MetadataItem) -> String {
    match item.value() {
        MetadataValue::Bool(value) => value.to_string(),
        MetadataValue::Timestamp(value) => {
            format!("{}.{:09}", value.seconds(), value.nanoseconds())
        }
        MetadataValue::PosixMode(value) => format!("0o{value:04o}"),
        MetadataValue::PosixUid(value) | MetadataValue::PosixGid(value) => value.to_string(),
        MetadataValue::HardlinkGroup(value) => value.to_string(),
        MetadataValue::Xattrs(values) => values
            .iter()
            .map(|value| {
                format!(
                    "{}:{}:{}",
                    String::from_utf8_lossy(value.name()),
                    value.value().len(),
                    crate::identity::sha256_exact(value.value())
                )
            })
            .collect::<Vec<_>>()
            .join(","),
        MetadataValue::SparseMap(value) => {
            let mut bytes = Vec::with_capacity(value.extents().len() * 16 + 8);
            bytes.extend_from_slice(&value.logical_size().to_be_bytes());
            for extent in value.extents() {
                bytes.extend_from_slice(&extent.offset.to_be_bytes());
                bytes.extend_from_slice(&extent.length.to_be_bytes());
            }
            format!(
                "logical_size={},extents={},sha256:{}",
                value.logical_size(),
                value.extents().len(),
                sha256_exact(&bytes)
            )
        }
        MetadataValue::Acls(values) => format!(
            "acls={},aces={}",
            values.len(),
            values.iter().map(|acl| acl.entries().len()).sum::<usize>()
        ),
        MetadataValue::WindowsSecurityDescriptor(value) => format!(
            "bytes={},dacl_aces={:?},sacl_aces={:?},sha256:{}",
            value.bytes().len(),
            value.dacl_entries(),
            value.sacl_entries(),
            sha256_exact(value.bytes())
        ),
        MetadataValue::WindowsFileAttributes(value) => format!("0x{value:08x}"),
        MetadataValue::WindowsReparseOriginal(value) => format!(
            "tag=0x{:08x},bytes={},sha256:{}",
            value.tag(),
            value.data().len(),
            sha256_exact(value.data())
        ),
        MetadataValue::MacosFlags(value) => format!("0x{value:08x}"),
    }
}

fn fidelity_summary(archive: &Archive) -> String {
    format!(
        "captured={},unavailable={},degraded={}",
        archive.fidelity.captured.len(),
        archive.fidelity.unavailable.len(),
        archive.fidelity.degraded.len()
    )
}

fn object_size(archive: &Archive, digest: Digest) -> Result<u64> {
    let object = archive.content_store.objects.get(&digest).ok_or_else(|| {
        Diagnostic::new(
            OutcomeClass::Nonconforming,
            ReasonCode::UnknownContentObject,
            "entry references an unknown ContentObject",
        )
    })?;
    object.chunks.iter().try_fold(0_u64, |total, reference| {
        let chunk = archive
            .content_store
            .chunks
            .get(&reference.chunk_id)
            .ok_or_else(|| {
                Diagnostic::new(
                    OutcomeClass::Nonconforming,
                    ReasonCode::UnknownChunk,
                    "ContentObject references an unknown Chunk",
                )
            })?;
        total.checked_add(chunk.logical_len).ok_or_else(|| {
            Diagnostic::new(
                OutcomeClass::PolicyRefused,
                ReasonCode::ResourceLimit,
                "logical object size overflows u64",
            )
        })
    })
}

/// Focus selectors for stable inspection JSON. No selection means all views.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct InspectionViews {
    pub entries: bool,
    pub plans: bool,
    pub chunks: bool,
    pub reconstruction: bool,
    pub provenance: bool,
    pub security: bool,
    pub access: bool,
}

/// Authenticated container-security facts supplied by the crypto layer. These
/// are report context only and never participate in EAM interpretation.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct InspectionSecurity {
    pub encrypted: bool,
    pub payload_suite: Option<String>,
    pub recipient_set_digest: Option<Digest>,
    pub archive_id: Option<Digest>,
    pub embedded_signature_count: u64,
    pub signatures_valid: u64,
    pub signatures_invalid: u64,
    pub signatures_unsupported: u64,
    pub signatures_stale: u64,
}

impl InspectionViews {
    fn all(self) -> bool {
        !(self.entries
            || self.plans
            || self.chunks
            || self.reconstruction
            || self.provenance
            || self.security
            || self.access)
    }
}

/// Stable, fixed-order JSON inspection over a fully verified archive.
pub fn inspection_json(opened: &OpenedArchive, views: InspectionViews) -> Result<Vec<u8>> {
    inspection_json_with_security(opened, views, &InspectionSecurity::default())
}

/// Stable inspection with caller-authenticated container security context.
pub fn inspection_json_with_security(
    opened: &OpenedArchive,
    views: InspectionViews,
    security: &InspectionSecurity,
) -> Result<Vec<u8>> {
    let view = inspect(opened)?;
    let all = views.all();
    let mut out = String::new();
    out.push('{');
    json_pair(&mut out, "format", INSPECTION_FORMAT);
    out.push_str("\"version\":1,");
    out.push_str("\"verification_scope\":\"whole archive verified\",");
    let _ = write!(
        out,
        "\"archive\":{{\"namespace\":\"{}\",\"format_major\":{},\"format_minor\":{},\"layout\":\"{}\",\"role\":\"Complete\",\"features\":{{\"incompat\":{},\"read_only_compat\":{},\"compat\":{}}}}},",
        view.format_namespace,
        view.version.major,
        view.version.minor,
        view.layout.as_str(),
        view.features.incompat,
        view.features.read_only_compat,
        view.features.compat
    );
    let _ = write!(
        out,
        "\"identities\":{{\"lai\":\"{}\",\"aux\":\"{}\",\"pcr\":\"{}\",\"pci\":\"{}\",\"status\":\"VERIFIED\"}},",
        view.identities.lai.0, view.identities.aux.0, view.identities.pcr.0, view.identities.pci.0
    );
    let _ = write!(
        out,
        "\"resources\":{{\"budget_declared\":{},\"entry_count\":{},\"total_logical_bytes\":{},\"max_single_entry_logical_bytes\":{},\"max_expansion_ratio_milli\":{},\"chunk_count\":{},\"max_path_depth\":{},\"max_metadata_bytes\":{},\"max_key_derivation_cost\":{},\"window_bytes\":{},\"working_set_bytes\":{},\"decode_flags\":{}}},",
        view.budget_declared,
        view.declared_resources.entry_count,
        view.declared_resources.total_logical_bytes,
        view.declared_resources.max_single_entry_logical_bytes,
        view.declared_resources.max_expansion_ratio_milli,
        view.declared_resources.chunk_count,
        view.declared_resources.max_path_depth,
        view.declared_resources.max_metadata_bytes,
        view.declared_resources.max_key_derivation_cost,
        view.decode_requirements.window_bytes,
        view.decode_requirements.working_set_bytes,
        view.decode_requirements.flags
    );
    if all || views.entries {
        out.push_str("\"entries\":[");
        for (index, entry) in opened.archive.entry_set.entries().iter().enumerate() {
            if index != 0 {
                out.push(',');
            }
            out.push_str("{\"path\":");
            json_string(&mut out, &entry.path().to_string());
            let _ = write!(
                out,
                ",\"kind\":\"{}\",\"content_object\":",
                kind_name(entry.kind())
            );
            match entry.data() {
                EntryData::Directory => out.push_str("null,\"logical_bytes\":0"),
                EntryData::Symlink { target } => {
                    out.push_str("null,\"logical_bytes\":0,\"symlink_target\":{");
                    let _ = write!(
                        out,
                        "\"encoding\":\"{:?}\",\"length\":{},\"sha256\":\"{}\"",
                        target.encoding(),
                        target.bytes().len(),
                        sha256_exact(target.bytes())
                    );
                    out.push('}');
                }
                EntryData::ReparsePoint { value } => {
                    out.push_str("null,\"logical_bytes\":0,\"reparse_point\":{");
                    let _ = write!(
                        out,
                        "\"tag\":{},\"length\":{},\"sha256\":\"{}\"",
                        value.tag(),
                        value.data().len(),
                        sha256_exact(value.data())
                    );
                    out.push('}');
                }
                EntryData::File {
                    content: ContentRef::Internal(digest),
                } => {
                    let _ = write!(
                        out,
                        "\"{digest}\",\"logical_bytes\":{}",
                        object_size(&opened.archive, *digest)?
                    );
                }
            }
            let _ = write!(
                out,
                ",\"core_executable\":{}",
                entry.metadata().executable()
            );
            out.push_str(",\"core_mtime\":");
            match entry.metadata().mtime() {
                Some(value) => {
                    let _ = write!(
                        out,
                        "{{\"seconds\":{},\"nanoseconds\":{},\"precision\":\"{:?}\",\"restorable\":{}}}",
                        value.seconds(),
                        value.nanoseconds(),
                        value.source_precision(),
                        value.restorable()
                    );
                }
                None => out.push_str("null"),
            }
            out.push_str(",\"posix\":{");
            let _ = write!(
                out,
                "\"mode\":{},\"uid\":{},\"gid\":{},\"hardlink_group\":{},\"sparse_map\":{}",
                entry
                    .metadata()
                    .posix_mode()
                    .map_or_else(|| "null".to_owned(), |value| value.to_string()),
                entry
                    .metadata()
                    .posix_uid()
                    .map_or_else(|| "null".to_owned(), |value| value.to_string()),
                entry
                    .metadata()
                    .posix_gid()
                    .map_or_else(|| "null".to_owned(), |value| value.to_string()),
                entry
                    .metadata()
                    .hardlink_group()
                    .map_or_else(|| "null".to_owned(), |value| format!("\"{value}\"")),
                entry.metadata().sparse_map().map_or_else(
                    || "null".to_owned(),
                    |value| format!(
                        "{{\"logical_size\":{},\"extent_count\":{}}}",
                        value.logical_size(),
                        value.extents().len()
                    )
                )
            );
            out.push_str(",\"xattrs\":[");
            for (attribute_index, attribute) in entry.metadata().xattrs().iter().enumerate() {
                if attribute_index != 0 {
                    out.push(',');
                }
                let _ = write!(
                    out,
                    "{{\"name_hex\":\"{}\",\"length\":{},\"sha256\":\"{}\"}}",
                    hex_bytes(attribute.name()),
                    attribute.value().len(),
                    sha256_exact(attribute.value())
                );
            }
            out.push_str("]}");
            let acl_count = entry.metadata().acls().len();
            let ace_count = entry
                .metadata()
                .acls()
                .iter()
                .map(|acl| acl.entries().len())
                .sum::<usize>();
            let _ = write!(
                out,
                ",\"platform_security\":{{\"acl_count\":{acl_count},\"ace_count\":{ace_count},\"windows_security_descriptor\":"
            );
            match entry.metadata().windows_security_descriptor() {
                Some(value) => {
                    let _ = write!(
                        out,
                        "{{\"length\":{},\"dacl_aces\":{},\"sacl_aces\":{},\"sha256\":\"{}\"}}",
                        value.bytes().len(),
                        value
                            .dacl_entries()
                            .map_or_else(|| "null".to_owned(), |count| count.to_string()),
                        value
                            .sacl_entries()
                            .map_or_else(|| "null".to_owned(), |count| count.to_string()),
                        sha256_exact(value.bytes())
                    );
                }
                None => out.push_str("null"),
            }
            out.push_str(",\"windows_file_attributes\":");
            match entry.metadata().windows_file_attributes() {
                Some(value) => {
                    let _ = write!(out, "{value}");
                }
                None => out.push_str("null"),
            }
            out.push_str(",\"windows_creation_time\":");
            write_timestamp_json(&mut out, entry.metadata().windows_creation_time());
            out.push_str(",\"windows_reparse_original\":");
            match entry.metadata().windows_reparse_original() {
                Some(value) => {
                    let _ = write!(
                        out,
                        "{{\"tag\":{},\"length\":{},\"sha256\":\"{}\"}}",
                        value.tag(),
                        value.data().len(),
                        sha256_exact(value.data())
                    );
                }
                None => out.push_str("null"),
            }
            out.push_str(",\"macos_flags\":");
            match entry.metadata().macos_flags() {
                Some(value) => {
                    let _ = write!(out, "{value}");
                }
                None => out.push_str("null"),
            }
            out.push_str(",\"macos_birthtime\":");
            write_timestamp_json(&mut out, entry.metadata().macos_birthtime());
            out.push('}');
            out.push('}');
        }
        out.push_str("],\"content_objects\":[");
        for (index, (digest, object)) in opened.archive.content_store.objects.iter().enumerate() {
            if index != 0 {
                out.push(',');
            }
            let _ = write!(
                out,
                "{{\"logical_digest\":\"{digest}\",\"chunk_root\":\"{}\",\"chunk_count\":{},\"logical_bytes\":{}}}",
                object.chunk_root,
                object.chunks.len(),
                object_size(&opened.archive, *digest)?
            );
        }
        out.push_str("],");
    }
    if all || views.plans {
        out.push_str("\"plans\":[");
        for (index, plan) in view.plans.iter().enumerate() {
            if index != 0 {
                out.push(',');
            }
            let _ = write!(out, "{{\"id\":{},\"identifier\":", plan.plan_id);
            json_string(&mut out, &plan.identifier);
            out.push_str(",\"codec\":");
            json_string(&mut out, &plan.codec);
            out.push_str(",\"codec_parameters_hex\":");
            json_string(
                &mut out,
                &hex_bytes(&opened.archive.transform_plans[index].codec_params),
            );
            out.push_str(",\"transforms\":[");
            for (step_index, step) in plan.transforms.iter().enumerate() {
                if step_index != 0 {
                    out.push(',');
                }
                json_string(&mut out, step);
            }
            let _ = write!(
                out,
                "],\"dictionary\":{},\"window_bytes\":{},\"working_set_bytes\":{},\"decode_flags\":{}}}",
                plan.dictionary
                    .map_or_else(|| "null".to_owned(), |value| format!("\"{value}\"")),
                plan.decode.window_bytes,
                plan.decode.working_set_bytes,
                plan.decode.flags
            );
        }
        out.push_str("],");
    }
    if all || views.chunks {
        let _ = write!(
            out,
            "\"chunks\":{{\"unique\":{},\"logical_references\":{},\"unique_plaintext_bytes\":{},\"deduplicated_bytes\":{},\"minimum_bytes\":{},\"average_bytes\":{},\"maximum_bytes\":{}}},",
            view.chunks.unique_chunk_count,
            view.chunks.logical_chunk_references,
            view.chunks.unique_plaintext_bytes,
            view.chunks.deduplicated_bytes,
            view.chunks.minimum_chunk_bytes,
            view.chunks.average_chunk_bytes,
            view.chunks.maximum_chunk_bytes
        );
        out.push_str("\"chunk_records\":[");
        for (ordinal, digest) in opened
            .archive
            .content_store
            .physical_order
            .iter()
            .enumerate()
        {
            if ordinal != 0 {
                out.push(',');
            }
            let chunk = &opened.archive.content_store.chunks[digest];
            let stored = opened
                .archive
                .index
                .chunks
                .get(digest)
                .map(|location| location.stored_len);
            let _ = write!(
                out,
                "{{\"digest\":\"{digest}\",\"physical_ordinal\":{ordinal},\"logical_bytes\":{},\"plan_id\":{},\"group\":",
                chunk.logical_len, chunk.plan_ref
            );
            match chunk.group_ref {
                Some(group) => {
                    let _ = write!(out, "\"{group}\"");
                }
                None => out.push_str("null"),
            }
            out.push_str(",\"stored_bytes\":");
            match stored {
                Some(value) => {
                    let _ = write!(out, "{value}");
                }
                None => out.push_str("null"),
            }
            out.push('}');
        }
        out.push_str("],");
    }
    if all || views.reconstruction {
        let _ = write!(
            out,
            "\"reconstruction\":{{\"data_objects\":{},\"data_bytes\":{},\"regions\":{},\"jpeg_regions\":{},\"worst_access_chunks\":{},\"worst_access_bytes\":{}}},",
            view.reconstruction.object_count,
            view.reconstruction.object_bytes,
            view.whole_object.region_count,
            view.whole_object.jpeg_region_count,
            view.whole_object.worst_access_chunks,
            view.whole_object.worst_access_bytes
        );
        out.push_str("\"reconstruction_regions\":[");
        for (index, region) in opened
            .archive
            .content_store
            .reconstruction_regions
            .values()
            .enumerate()
        {
            if index != 0 {
                out.push(',');
            }
            let _ = write!(
                out,
                "{{\"region_id\":\"{}\",\"content_object\":\"{}\",\"start_chunk_index\":{},\"chunk_count\":{},\"plan_id\":{},\"logical_bytes\":{},\"representation_bytes\":{},\"access\":{{\"logical_bytes\":{},\"logical_chunks\":{},\"worst_reconstructed_bytes\":{}}}}}",
                region.region_id,
                region.content_object,
                region.start_chunk_index,
                region.chunk_count,
                region.plan_ref,
                region.logical_bytes,
                region.representation.len(),
                region.access.logical_bytes,
                region.access.logical_chunks,
                region.access.worst_reconstructed_bytes
            );
        }
        out.push_str("],\"reconstruction_audits\":[");
        for (index, audit) in opened
            .archive
            .content_store
            .reconstruction_audits
            .values()
            .enumerate()
        {
            if index != 0 {
                out.push(',');
            }
            let _ = write!(out, "{{\"target\":\"{:?}\",\"transform\":", audit.target);
            json_string(&mut out, &audit.transform_id);
            let _ = write!(out, ",\"reason\":\"{:?}\"}}", audit.reason);
        }
        out.push_str("],");
    }
    if all || views.provenance {
        out.push_str("\"provenance\":{");
        match &view.conversion {
            Some(value) => {
                json_pair(&mut out, "source_format", &value.source_format);
                json_pair(&mut out, "source_digest", &value.source_digest.to_string());
                json_pair(&mut out, "import_mode", &value.import_mode);
            }
            None => {
                out.push_str("\"source_format\":null,\"source_digest\":null,\"import_mode\":null,");
            }
        }
        let _ = write!(
            out,
            "\"fidelity_unavailable\":{},\"fidelity_degraded\":{},\"preservation_present\":{} }},",
            view.fidelity.unavailable.len(),
            view.fidelity.degraded.len(),
            view.preservation.is_some()
        );
    }
    if all || views.security {
        let _ = write!(
            out,
            "\"security\":{{\"encrypted\":{},\"private_metadata_authenticated\":{},\"payload_suite\":",
            security.encrypted, security.encrypted
        );
        match &security.payload_suite {
            Some(value) => json_string(&mut out, value),
            None => out.push_str("null"),
        }
        out.push_str(",\"recipient_set_digest\":");
        match security.recipient_set_digest {
            Some(value) => json_string(&mut out, &value.to_string()),
            None => out.push_str("null"),
        }
        out.push_str(",\"archive_id\":");
        match security.archive_id {
            Some(value) => json_string(&mut out, &value.to_string()),
            None => out.push_str("null"),
        }
        let _ = write!(
            out,
            ",\"embedded_signature_count\":{},\"signatures_valid\":{},\"signatures_invalid\":{},\"signatures_unsupported\":{},\"signatures_stale\":{},\"secret_material_exposed\":false}},",
            security.embedded_signature_count,
            security.signatures_valid,
            security.signatures_invalid,
            security.signatures_unsupported,
            security.signatures_stale
        );
    }
    if all || views.access {
        let _ = write!(
            out,
            "\"access\":{{\"random_entry_lookup\":{},\"stream_dedup_window\":{},\"index_status\":\"{}\",\"group_count\":{},\"maximum_lookback\":{},\"worst_group_access_bytes\":{}}},",
            view.random_entry_lookup,
            view.stream_dedup_window,
            index_name(view.index_status),
            view.cross_file.chunk_group_count,
            view.cross_file.maximum_lookback,
            view.cross_file.worst_random_access_bytes
        );
    }
    if out.ends_with(',') {
        out.pop();
    }
    out.push_str("}\n");
    Ok(out.into_bytes())
}

/// Stable reduced inspection for a range-backed metadata session. Every
/// identity field carries its actual partial-verification status.
#[must_use]
pub fn random_inspection_json(
    metadata: &RandomAccessMetadata,
    report: &RandomAccessVerificationReport,
) -> Vec<u8> {
    let mut out = String::new();
    out.push('{');
    json_pair(&mut out, "format", INSPECTION_FORMAT);
    out.push_str("\"version\":1,\"verification_scope\":\"range-backed metadata; whole_archive_verified=false\",");
    let _ = write!(
        out,
        "\"archive\":{{\"layout\":\"{}\",\"features\":{},\"encrypted\":{},\"source_length\":{}}},",
        metadata.descriptor.layout.as_str(),
        metadata.descriptor.features.incompat,
        metadata.encrypted,
        metadata.source_length
    );
    let _ = write!(
        out,
        "\"identities\":{{\"lai\":\"{}\",\"lai_status\":\"{}\",\"aux\":\"{}\",\"aux_status\":\"{}\",\"pcr\":\"{}\",\"pcr_status\":\"{}\",\"pci\":null,\"pci_status\":\"{}\"}},",
        metadata.descriptor.lai,
        identity_status(report.lai),
        metadata.descriptor.aux,
        identity_status(report.aux),
        metadata.descriptor.pcr,
        identity_status(report.pcr),
        identity_status(report.pci)
    );
    let _ = write!(
        out,
        "\"metadata\":{{\"entries\":{},\"content_objects\":{},\"planner_id\":",
        metadata.entries.len(),
        metadata.content_objects.len()
    );
    json_string(&mut out, &metadata.descriptor.planner_id);
    out.push_str(",\"chunker_id\":");
    json_string(&mut out, &metadata.descriptor.chunker_id);
    let _ = writeln!(
        out,
        "}},\"access\":{{\"source_revision_stable\":{},\"section_count\":{},\"bytes_fetched\":{},\"range_request_count\":{},\"whole_archive_verified\":false}}}}",
        report.source_revision_stable,
        metadata.section_count,
        report.bytes_fetched,
        report.range_request_count
    );
    out.into_bytes()
}

fn identity_status(status: IdentityVerificationStatus) -> &'static str {
    match status {
        IdentityVerificationStatus::Verified => "VERIFIED",
        IdentityVerificationStatus::NotRequested => "NOT_REQUESTED",
        IdentityVerificationStatus::DeclaredNotFullyVerified => "DECLARED_NOT_FULLY_VERIFIED",
        IdentityVerificationStatus::NotComputed => "NOT_COMPUTED",
    }
}

/// Evidence source for one explanation statement.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvidenceClass {
    Recorded,
    Derived,
    Audit,
    NotRecorded,
}
impl EvidenceClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Recorded => "RECORDED",
            Self::Derived => "DERIVED",
            Self::Audit => "AUDIT",
            Self::NotRecorded => "NOT_RECORDED",
        }
    }
}

/// One explanation fact, classified by its evidence source.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExplanationFact {
    pub class: EvidenceClass,
    pub subject: String,
    pub field: String,
    pub value: String,
}

/// Archive- or entry-scoped recorded-plan/access explanation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StructuredExplanation {
    pub path: Option<String>,
    pub facts: Box<[ExplanationFact]>,
}

/// Explains only recorded facts, direct derivations, and persisted audits.
pub fn structured_explain(
    opened: &OpenedArchive,
    path: Option<&str>,
) -> Result<StructuredExplanation> {
    let archive = &opened.archive;
    let stored_chunk_bytes = archive
        .index
        .chunks
        .values()
        .try_fold(0_u64, |total, location| {
            total.checked_add(location.stored_len).ok_or_else(|| {
                Diagnostic::new(
                    OutcomeClass::PolicyRefused,
                    ReasonCode::ResourceLimit,
                    "stored Chunk byte total exceeds u64",
                )
            })
        })?;
    let unique_plaintext_bytes =
        archive
            .content_store
            .chunks
            .values()
            .try_fold(0_u64, |total, chunk| {
                total.checked_add(chunk.logical_len).ok_or_else(|| {
                    Diagnostic::new(
                        OutcomeClass::PolicyRefused,
                        ReasonCode::ResourceLimit,
                        "plaintext Chunk byte total exceeds u64",
                    )
                })
            })?;
    let logical_chunk_references =
        archive
            .content_store
            .objects
            .values()
            .try_fold(0_u64, |total, object| {
                total
                    .checked_add(u64::try_from(object.chunks.len()).map_err(|_| {
                        Diagnostic::new(
                            OutcomeClass::PolicyRefused,
                            ReasonCode::ResourceLimit,
                            "logical Chunk count exceeds u64",
                        )
                    })?)
                    .ok_or_else(|| {
                        Diagnostic::new(
                            OutcomeClass::PolicyRefused,
                            ReasonCode::ResourceLimit,
                            "logical Chunk count exceeds u64",
                        )
                    })
            })?;
    let mut facts = vec![
        fact(
            EvidenceClass::Recorded,
            "archive",
            "planner_id",
            &archive.descriptor.planner_id,
        ),
        fact(
            EvidenceClass::Recorded,
            "archive",
            "chunker_id",
            &archive.descriptor.chunker_id,
        ),
        fact(
            EvidenceClass::Derived,
            "archive",
            "logical_bytes",
            &archive.total_logical_size()?.to_string(),
        ),
        fact(
            EvidenceClass::Derived,
            "archive",
            "stored_chunk_bytes",
            &stored_chunk_bytes.to_string(),
        ),
        fact(
            EvidenceClass::Derived,
            "archive",
            "physical_savings_bytes",
            &(i128::from(unique_plaintext_bytes) - i128::from(stored_chunk_bytes)).to_string(),
        ),
        fact(
            EvidenceClass::Derived,
            "archive",
            "unique_chunks",
            &archive.content_store.chunks.len().to_string(),
        ),
        fact(
            EvidenceClass::Derived,
            "archive",
            "logical_chunk_references",
            &logical_chunk_references.to_string(),
        ),
        fact(
            EvidenceClass::Recorded,
            "archive",
            "dictionary_count",
            &archive.content_store.dictionaries.len().to_string(),
        ),
        fact(
            EvidenceClass::Recorded,
            "archive",
            "chunk_group_count",
            &archive.content_store.chunk_groups.len().to_string(),
        ),
        fact(
            EvidenceClass::Recorded,
            "archive",
            "reconstruction_region_count",
            &archive
                .content_store
                .reconstruction_regions
                .len()
                .to_string(),
        ),
    ];
    for plan in &archive.transform_plans {
        facts.push(fact(
            EvidenceClass::Recorded,
            &format!("plan:{}", plan.plan_id),
            "codec",
            &plan.codec,
        ));
        facts.push(fact(
            EvidenceClass::Recorded,
            &format!("plan:{}", plan.plan_id),
            "identifier",
            &plan.identifier,
        ));
        facts.push(fact(
            EvidenceClass::Recorded,
            &format!("plan:{}", plan.plan_id),
            "decode_working_set_bytes",
            &plan.decode.working_set_bytes.to_string(),
        ));
    }
    if let Some(path) = path {
        let entry = archive
            .entry_set
            .entries()
            .iter()
            .find(|entry| entry.path().to_string() == path)
            .ok_or_else(|| {
                Diagnostic::new(
                    OutcomeClass::PolicyRefused,
                    ReasonCode::RandomAccessEntryNotFound,
                    format!("logical path '{path}' is absent"),
                )
            })?;
        facts.push(fact(
            EvidenceClass::Recorded,
            path,
            "entry_kind",
            kind_name(entry.kind()),
        ));
        for item in entry.metadata().items() {
            facts.push(fact(
                EvidenceClass::Recorded,
                path,
                item.name().as_str(),
                &metadata_item_text(item),
            ));
        }
        facts.push(fact(
            EvidenceClass::Derived,
            path,
            "metadata_restorability",
            if entry.metadata().uses_posix_v1()
                || entry.metadata().uses_platform_security_v1()
                || matches!(entry.data(), EntryData::ReparsePoint { .. })
            {
                "policy- and platform-dependent"
            } else {
                "bootstrap metadata only"
            },
        ));
        if let EntryData::Symlink { target } = entry.data() {
            facts.push(fact(
                EvidenceClass::Recorded,
                path,
                "symlink_target_encoding",
                match target.encoding() {
                    crate::eam::LinkTargetEncoding::Utf8 => "UTF8",
                    crate::eam::LinkTargetEncoding::PosixBytes => "POSIX_BYTES",
                },
            ));
            facts.push(fact(
                EvidenceClass::Recorded,
                path,
                "symlink_target",
                &format!(
                    "{} bytes sha256:{}",
                    target.bytes().len(),
                    sha256_exact(target.bytes())
                ),
            ));
        }
        if let EntryData::ReparsePoint { value } = entry.data() {
            facts.push(fact(
                EvidenceClass::Recorded,
                path,
                "windows_reparse_point",
                &format!(
                    "tag=0x{:08x} {} bytes sha256:{}",
                    value.tag(),
                    value.data().len(),
                    sha256_exact(value.data())
                ),
            ));
            facts.push(fact(
                EvidenceClass::Derived,
                path,
                "reparse_restorability",
                "refused by default; exact restoration requires Windows and an explicit policy",
            ));
        }
        let EntryData::File {
            content: ContentRef::Internal(digest),
        } = entry.data()
        else {
            facts.push(fact(
                EvidenceClass::NotRecorded,
                path,
                "content_decode_plan",
                "not applicable to a non-file Entry",
            ));
            facts.push(fact(
                EvidenceClass::NotRecorded,
                "archive",
                "unselected_candidate_comparisons",
                "not persisted by this archive",
            ));
            return Ok(StructuredExplanation {
                path: Some(path.to_owned()),
                facts: facts.into_boxed_slice(),
            });
        };
        let object = archive.content_store.objects.get(digest).ok_or_else(|| {
            Diagnostic::new(
                OutcomeClass::Nonconforming,
                ReasonCode::UnknownContentObject,
                "entry references an unknown ContentObject",
            )
        })?;
        facts.push(fact(
            EvidenceClass::Recorded,
            path,
            "content_object",
            &digest.to_string(),
        ));
        facts.push(fact(
            EvidenceClass::Recorded,
            path,
            "chunk_count",
            &object.chunks.len().to_string(),
        ));
        facts.push(fact(
            EvidenceClass::Derived,
            path,
            "logical_bytes",
            &object_size(archive, *digest)?.to_string(),
        ));
        let mut dictionaries = BTreeSet::new();
        let mut group_predecessors = BTreeSet::new();
        let mut entry_working_set = 0_u64;
        let plans = archive
            .transform_plans
            .iter()
            .map(|plan| (plan.plan_id, plan))
            .collect::<BTreeMap<_, _>>();
        let positions = archive
            .content_store
            .physical_order
            .iter()
            .enumerate()
            .map(|(index, digest)| (*digest, index))
            .collect::<BTreeMap<_, _>>();
        let requested = object
            .chunks
            .iter()
            .map(|item| item.chunk_id)
            .collect::<BTreeSet<_>>();
        for reference in &object.chunks {
            let chunk = &archive.content_store.chunks[&reference.chunk_id];
            let plan = plans[&chunk.plan_ref];
            entry_working_set = entry_working_set.max(plan.decode.working_set_bytes);
            facts.push(fact(
                EvidenceClass::Recorded,
                &reference.chunk_id.to_string(),
                "codec",
                &plan.codec,
            ));
            facts.push(fact(
                EvidenceClass::Recorded,
                &reference.chunk_id.to_string(),
                "transform_plan",
                &plan.identifier,
            ));
            if let Some(dictionary) = plan.dictionary {
                dictionaries.insert(dictionary);
            }
            if let Some(group_id) = chunk.group_ref {
                let group = &archive.content_store.chunk_groups[&group_id];
                let position = positions[&reference.chunk_id];
                let start = position.saturating_sub(group.max_lookback as usize);
                for predecessor in &archive.content_store.physical_order[start..position] {
                    if archive.content_store.chunks[predecessor].group_ref == Some(group_id) {
                        group_predecessors.insert(*predecessor);
                    }
                }
            }
        }
        let regions = archive
            .content_store
            .reconstruction_regions
            .values()
            .filter(|region| region.content_object == *digest)
            .collect::<Vec<_>>();
        let closure = requested
            .union(&group_predecessors)
            .copied()
            .collect::<BTreeSet<_>>();
        let closure_logical_bytes = closure.iter().try_fold(0_u64, |total, digest| {
            total
                .checked_add(archive.content_store.chunks[digest].logical_len)
                .ok_or_else(|| {
                    Diagnostic::new(
                        OutcomeClass::PolicyRefused,
                        ReasonCode::ResourceLimit,
                        "access closure logical bytes exceed u64",
                    )
                })
        })?;
        facts.push(fact(
            EvidenceClass::Derived,
            path,
            "dedup_shared_chunks",
            &requested
                .iter()
                .filter(|chunk_id| {
                    archive
                        .content_store
                        .objects
                        .values()
                        .filter(|object| {
                            object
                                .chunks
                                .iter()
                                .any(|reference| reference.chunk_id == **chunk_id)
                        })
                        .count()
                        > 1
                })
                .count()
                .to_string(),
        ));
        facts.push(fact(
            EvidenceClass::Recorded,
            path,
            "dictionary_dependencies",
            &dictionaries.len().to_string(),
        ));
        facts.push(fact(
            EvidenceClass::Derived,
            path,
            "lookback_predecessors",
            &group_predecessors
                .difference(&requested)
                .count()
                .to_string(),
        ));
        facts.push(fact(
            EvidenceClass::Recorded,
            path,
            "reconstruction_regions",
            &regions.len().to_string(),
        ));
        let worst_region_bytes = regions
            .iter()
            .map(|region| region.access.worst_reconstructed_bytes)
            .max()
            .unwrap_or(0);
        facts.push(fact(
            EvidenceClass::Recorded,
            path,
            "worst_reconstruction_bytes",
            &worst_region_bytes.to_string(),
        ));
        facts.push(fact(
            EvidenceClass::Derived,
            path,
            "dependency_closure_logical_bytes",
            &closure_logical_bytes.to_string(),
        ));
        facts.push(fact(
            EvidenceClass::Recorded,
            path,
            "decode_working_set_bytes",
            &entry_working_set.to_string(),
        ));
        facts.push(fact(
            EvidenceClass::Derived,
            path,
            "estimated_range_count",
            &(requested.len()
                + dictionaries.len()
                + group_predecessors.difference(&requested).count()
                + regions.len())
            .to_string(),
        ));
    }
    for audit in archive.content_store.reconstruction_audits.values() {
        facts.push(fact(
            EvidenceClass::Audit,
            "archive",
            "reconstruction_fallback",
            &format!("{}: {:?}", audit.transform_id, audit.reason),
        ));
    }
    facts.push(fact(
        EvidenceClass::NotRecorded,
        "archive",
        "unselected_candidate_comparisons",
        "not persisted by this archive",
    ));
    Ok(StructuredExplanation {
        path: path.map(str::to_owned),
        facts: facts.into_boxed_slice(),
    })
}

fn fact(class: EvidenceClass, subject: &str, field: &str, value: &str) -> ExplanationFact {
    ExplanationFact {
        class,
        subject: subject.to_owned(),
        field: field.to_owned(),
        value: value.to_owned(),
    }
}

fn json_pair(out: &mut String, key: &str, value: &str) {
    json_string(out, key);
    out.push(':');
    json_string(out, value);
    out.push(',');
}
fn json_option(out: &mut String, key: &str, value: Option<&str>) {
    json_string(out, key);
    out.push(':');
    match value {
        Some(value) => json_string(out, value),
        None => out.push_str("null"),
    }
    out.push(',');
}
fn json_option_last(out: &mut String, key: &str, value: Option<&str>) {
    json_string(out, key);
    out.push(':');
    match value {
        Some(value) => json_string(out, value),
        None => out.push_str("null"),
    }
}
fn json_string(out: &mut String, value: &str) {
    out.push('"');
    for character in value.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            character if character <= '\u{1f}' => {
                let _ = write!(out, "\\u{:04x}", u32::from(character));
            }
            character => out.push(character),
        }
    }
    out.push('"');
}

fn write_timestamp_json(out: &mut String, value: Option<crate::eam::Timestamp>) {
    if let Some(value) = value {
        let _ = write!(
            out,
            "{{\"seconds\":{},\"nanoseconds\":{},\"precision\":\"{:?}\",\"restorable\":{}}}",
            value.seconds(),
            value.nanoseconds(),
            value.source_precision(),
            value.restorable()
        );
    } else {
        out.push_str("null");
    }
}

fn hex_bytes(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len().saturating_mul(2));
    for byte in bytes {
        let _ = write!(output, "{byte:02x}");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::archive::plan_observed_archive;
    use crate::eam::{
        Acl, AclDialect, AclEntry, AclEntryType, AclPrincipal, AclScope, ConversionProvenance,
        Entry, EntryData, EntryIdentity, FidelityReport, LogicalPath, MetadataItem, MetadataSet,
        WindowsReparsePoint,
    };

    fn posix_acl(named_permissions: u32) -> Acl {
        Acl::new(
            AclDialect::Posix1e,
            AclScope::Access,
            vec![
                AclEntry::new(AclEntryType::Allow, AclPrincipal::UserObj, 6, 0).unwrap(),
                AclEntry::new(
                    AclEntryType::Allow,
                    AclPrincipal::User(1000),
                    named_permissions,
                    0,
                )
                .unwrap(),
                AclEntry::new(AclEntryType::Allow, AclPrincipal::GroupObj, 4, 0).unwrap(),
                AclEntry::new(AclEntryType::Allow, AclPrincipal::Mask, 4, 0).unwrap(),
                AclEntry::new(AclEntryType::Allow, AclPrincipal::Other, 0, 0).unwrap(),
            ],
        )
        .unwrap()
    }

    fn opened_hardlink_fixture(linked: bool) -> OpenedArchive {
        let content = Box::<[u8]>::from(b"hardlink topology bytes".as_slice());
        let content_digest = crate::identity::sha256_exact(&content);
        let paths = [
            LogicalPath::from_utf8(["left"]).unwrap(),
            LogicalPath::from_utf8(["right"]).unwrap(),
        ];
        let group = crate::identity::hardlink_group_id(content_digest, &paths).unwrap();
        let entries = paths
            .into_iter()
            .map(|path| {
                let metadata = if linked {
                    MetadataSet::new(vec![MetadataItem::hardlink_group(group)]).unwrap()
                } else {
                    MetadataSet::default()
                };
                Entry::new(
                    path,
                    EntryData::File {
                        content: ContentRef::Internal(content_digest),
                    },
                    metadata,
                    EntryIdentity::default(),
                )
            })
            .collect();
        let archive = plan_observed_archive(
            entries,
            vec![content],
            FidelityReport::default(),
            ConversionProvenance {
                source_format: "test".to_owned(),
                adapter_id: "entrybound/test-v1".to_owned(),
                source_digest: Digest::ZERO,
                import_mode: "strict".to_owned(),
                source_entry_count: 2,
                observation_count: 0,
                omission_count: 0,
                refinement_count: 0,
                divergence_count: 0,
                irreconcilable_count: 0,
                resolutions: Box::default(),
                synthesized_ancestors: Box::default(),
                unsupported_metadata: Box::default(),
                outcome: "accepted".to_owned(),
            },
            None,
            CompressionProfile::Fast,
        )
        .unwrap();
        let encoded = encode(&archive, WriteOptions::default()).unwrap();
        open(&encoded.bytes).unwrap()
    }

    fn opened_fixture(profile: CompressionProfile) -> OpenedArchive {
        let content = Box::<[u8]>::from(b"same logical content for native tooling".as_slice());
        let content_digest = crate::identity::sha256_exact(&content);
        let archive = plan_observed_archive(
            vec![
                Entry::new(
                    LogicalPath::from_utf8(["docs"]).unwrap(),
                    EntryData::Directory,
                    MetadataSet::default(),
                    EntryIdentity::default(),
                ),
                Entry::new(
                    LogicalPath::from_utf8(["docs", "readme.txt"]).unwrap(),
                    EntryData::File {
                        content: ContentRef::Internal(content_digest),
                    },
                    MetadataSet::new(vec![MetadataItem::executable(false)]).unwrap(),
                    EntryIdentity::default(),
                ),
            ],
            vec![content],
            FidelityReport::default(),
            ConversionProvenance {
                source_format: "test".to_owned(),
                adapter_id: "entrybound/test-v1".to_owned(),
                source_digest: Digest::ZERO,
                import_mode: "strict".to_owned(),
                source_entry_count: 2,
                observation_count: 0,
                omission_count: 0,
                refinement_count: 0,
                divergence_count: 0,
                irreconcilable_count: 0,
                resolutions: Box::default(),
                synthesized_ancestors: Box::default(),
                unsupported_metadata: Box::default(),
                outcome: "accepted".to_owned(),
            },
            None,
            profile,
        )
        .unwrap();
        let encoded = encode(
            &archive,
            WriteOptions {
                include_index: true,
            },
        )
        .unwrap();
        open(&encoded.bytes).unwrap()
    }

    fn opened_acl_fixture(named_permissions: u32) -> OpenedArchive {
        let content = Box::<[u8]>::from(b"ACL identity-tier bytes".as_slice());
        let content_digest = crate::identity::sha256_exact(&content);
        let archive = plan_observed_archive(
            vec![Entry::new(
                LogicalPath::from_utf8(["secured.txt"]).unwrap(),
                EntryData::File {
                    content: ContentRef::Internal(content_digest),
                },
                MetadataSet::new(vec![
                    MetadataItem::executable(false),
                    MetadataItem::posix_mode(0o640),
                    MetadataItem::acls(vec![posix_acl(named_permissions)]).unwrap(),
                ])
                .unwrap(),
                EntryIdentity::default(),
            )],
            vec![content],
            FidelityReport::default(),
            ConversionProvenance {
                source_format: "test".to_owned(),
                adapter_id: "entrybound/platform-diff-test-v1".to_owned(),
                source_digest: Digest::ZERO,
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
                outcome: "accepted".to_owned(),
            },
            None,
            CompressionProfile::Fast,
        )
        .unwrap();
        let encoded = encode(&archive, WriteOptions::default()).unwrap();
        open(&encoded.bytes).unwrap()
    }

    #[test]
    fn representation_repack_preserves_all_native_roots() {
        let source = opened_fixture(CompressionProfile::Balanced);
        let prepared = prepare_repack(
            &source,
            RepackOptions {
                mode: RepackMode::RepresentationOnly,
                layout: Layout::Stream,
                index: IndexPolicy::Preserve,
                stream_window: StreamWindow::Auto,
            },
        )
        .unwrap();
        assert!(prepared.analysis.lai_equal);
        assert!(prepared.analysis.aux_equal);
        assert!(prepared.analysis.pcr_equal);
        assert_eq!(prepared.verified.archive.descriptor.layout, Layout::Stream);
    }

    #[test]
    fn replanning_preserves_semantic_and_auxiliary_roots() {
        let source = opened_fixture(CompressionProfile::Fast);
        let prepared = prepare_repack(
            &source,
            RepackOptions {
                mode: RepackMode::Replan(CompressionProfile::Dense),
                layout: Layout::Indexed,
                index: IndexPolicy::Absent,
                stream_window: StreamWindow::Auto,
            },
        )
        .unwrap();
        assert!(prepared.analysis.lai_equal);
        assert!(prepared.analysis.aux_equal);
        assert_eq!(prepared.analysis.target_planner, "dense-v6");
        assert_eq!(
            prepared.verified.report.index_status,
            IndexStatus::RebuiltAbsent
        );
        let report = archive_diff(&source, &prepared.verified).unwrap();
        assert_eq!(report.lai, DiffIdentityStatus::Same);
        assert_eq!(report.aux, DiffIdentityStatus::Same);
        assert_eq!(report.pcr, DiffIdentityStatus::Different);
        assert!(
            report
                .changes
                .iter()
                .any(|item| item.tier == DiffTier::Physical)
        );
        assert!(
            !report
                .changes
                .iter()
                .any(|item| { matches!(item.tier, DiffTier::Semantic | DiffTier::Auxiliary) })
        );
    }

    #[test]
    fn diff_places_index_only_change_in_container_tier() {
        let source = opened_fixture(CompressionProfile::Balanced);
        let without_index = encode(
            &source.archive,
            WriteOptions {
                include_index: false,
            },
        )
        .unwrap();
        let right = open(&without_index.bytes).unwrap();
        let report = archive_diff(&source, &right).unwrap();
        assert_eq!(report.lai, DiffIdentityStatus::Same);
        assert_eq!(report.aux, DiffIdentityStatus::Same);
        assert_eq!(report.pcr, DiffIdentityStatus::Same);
        assert!(
            report
                .changes
                .iter()
                .all(|item| item.tier == DiffTier::Container)
        );
    }

    #[test]
    fn diff_keeps_conversion_evidence_in_auxiliary_tier() {
        let source = opened_fixture(CompressionProfile::Balanced);
        let mut changed = source.archive.clone();
        changed
            .conversion
            .as_mut()
            .expect("fixture carries conversion evidence")
            .source_digest = crate::identity::sha256_exact(b"different source evidence");
        let encoded = encode(
            &changed,
            WriteOptions {
                include_index: true,
            },
        )
        .unwrap();
        let right = open(&encoded.bytes).unwrap();
        let report = archive_diff(&source, &right).unwrap();
        assert_eq!(report.lai, DiffIdentityStatus::Same);
        assert_eq!(report.aux, DiffIdentityStatus::Different);
        assert!(report.changes.iter().any(|item| {
            item.tier == DiffTier::Auxiliary && item.field == "conversion_provenance"
        }));
        assert!(
            !report
                .changes
                .iter()
                .any(|item| item.tier == DiffTier::Semantic)
        );
    }

    #[test]
    fn hardlink_topology_is_auxiliary_not_semantic_or_physical() {
        let independent = opened_hardlink_fixture(false);
        let linked = opened_hardlink_fixture(true);
        let report = archive_diff(&independent, &linked).unwrap();
        assert_eq!(report.lai, DiffIdentityStatus::Same);
        assert_eq!(report.aux, DiffIdentityStatus::Different);
        assert_eq!(report.pcr, DiffIdentityStatus::Same);
        assert!(report.changes.iter().any(|change| {
            change.tier == DiffTier::Auxiliary && change.field == "posix.hardlink-group"
        }));
        assert!(
            !report.changes.iter().any(
                |change| change.tier == DiffTier::Semantic || change.tier == DiffTier::Physical
            )
        );
    }

    #[test]
    fn platform_security_diff_respects_auxiliary_and_semantic_tiers() {
        let left = opened_acl_fixture(4);
        let right = opened_acl_fixture(6);
        let report = archive_diff(&left, &right).unwrap();
        assert_eq!(report.lai, DiffIdentityStatus::Same);
        assert_eq!(report.aux, DiffIdentityStatus::Different);
        assert_eq!(report.pcr, DiffIdentityStatus::Same);
        assert!(report.changes.iter().any(|change| {
            change.tier == DiffTier::Auxiliary && change.field == "security.acls"
        }));
        assert!(
            !report
                .changes
                .iter()
                .any(|change| { matches!(change.tier, DiffTier::Semantic | DiffTier::Physical) })
        );

        let repacked = prepare_repack(
            &left,
            RepackOptions {
                mode: RepackMode::RepresentationOnly,
                layout: Layout::Stream,
                index: IndexPolicy::Preserve,
                stream_window: StreamWindow::Auto,
            },
        )
        .unwrap();
        assert_eq!(repacked.verified.archive.entry_set, left.archive.entry_set);
        assert!(repacked.analysis.lai_equal);
        assert!(repacked.analysis.aux_equal);
        assert!(repacked.analysis.pcr_equal);

        let changed = plan_observed_archive(
            vec![Entry::new(
                LogicalPath::from_utf8(["secured.txt"]).unwrap(),
                EntryData::ReparsePoint {
                    value: WindowsReparsePoint::new(0xa000_001d, b"opaque-v1".to_vec()).unwrap(),
                },
                MetadataSet::default(),
                EntryIdentity::default(),
            )],
            Vec::new(),
            FidelityReport::default(),
            left.archive.conversion.clone().unwrap(),
            None,
            CompressionProfile::Fast,
        )
        .unwrap();
        let encoded = encode(&changed, WriteOptions::default()).unwrap();
        let reparse = open(&encoded.bytes).unwrap();
        let report = archive_diff(&left, &reparse).unwrap();
        assert_eq!(report.lai, DiffIdentityStatus::Different);
        assert!(
            report
                .changes
                .iter()
                .any(|change| change.tier == DiffTier::Semantic && change.field == "kind")
        );
    }

    #[test]
    fn structured_json_and_evidence_classes_are_stable() {
        let source = opened_fixture(CompressionProfile::Balanced);
        let first = inspection_json(&source, InspectionViews::default()).unwrap();
        let second = inspection_json(&source, InspectionViews::default()).unwrap();
        assert_eq!(first, second);
        assert!(
            String::from_utf8(first)
                .unwrap()
                .contains(INSPECTION_FORMAT)
        );
        let explanation = structured_explain(&source, Some("docs/readme.txt")).unwrap();
        assert!(
            explanation
                .facts
                .iter()
                .any(|fact| fact.class == EvidenceClass::Recorded)
        );
        assert!(
            explanation
                .facts
                .iter()
                .any(|fact| fact.class == EvidenceClass::Derived)
        );
        assert!(
            explanation
                .facts
                .iter()
                .any(|fact| fact.class == EvidenceClass::NotRecorded)
        );
    }
}
