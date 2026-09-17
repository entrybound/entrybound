//! `Serialize`-able mirror of `entrybound::archive::ArchiveInspection`.
//!
//! `ArchiveInspection` (and the types it nests: `CodecUsage`,
//! `ChunkStatistics`, `CrossFileInspection`, `ReconstructionInspection`,
//! `PlanInspection`) derive `Clone, Debug, Eq, PartialEq` but not `Serialize`
//! -- they are a stable-shaped Rust view for library callers, not a wire
//! schema. This module copies the fields this crate's JSON rows need (every
//! field this task's byte-accounting and counts requirements name: entry,
//! chunk, unique-chunk, dictionary, and group counts; per-codec stored and
//! logical bytes; planner id) into small, plain `Serialize` structs, rather
//! than adding a `serde` dependency edge into `crates/entrybound` itself
//! (out of scope for a research-only crate, and unrelated to this task).

use entrybound::archive::ArchiveInspection;
use entrybound::diagnostics::Diagnostic;
use entrybound::eam::Layout;
use serde::Serialize;
use std::fmt;

/// One codec's aggregate Chunk usage.
#[derive(Debug, Clone, Serialize)]
pub struct CodecUsage {
    pub codec: String,
    pub chunk_count: u64,
    pub logical_bytes: u64,
    pub stored_bytes: u64,
}

/// One recorded TransformPlan, enough to attribute a Chunk frame's stored
/// bytes to a codec and a transform pipeline during the byte-accounting walk
/// (see [`crate::bytes`]).
#[derive(Debug, Clone, Serialize)]
pub struct PlanSummary {
    pub plan_id: u64,
    pub codec: String,
    /// Human-readable transform steps (`entrybound::transform::display_step`
    /// output, already computed by `ArchiveInspection::plans`), joined with
    /// `+`, or `"none"` when the plan applies no structural transform.
    pub transform_signature: String,
}

/// Logical/unique-Chunk statistics (mirrors `ChunkStatistics`, which is
/// already plain `u64` fields; this only adds `Serialize`).
#[derive(Debug, Clone, Copy, Serialize)]
pub struct ChunkStatistics {
    pub unique_chunk_count: u64,
    pub logical_chunk_references: u64,
    pub unique_plaintext_bytes: u64,
    pub deduplicated_bytes: u64,
    pub dedup_ratio_milli: u64,
    pub minimum_chunk_bytes: u64,
    pub average_chunk_bytes: u64,
    pub maximum_chunk_bytes: u64,
}

/// Dictionary/ChunkGroup presence and counts (mirrors `CrossFileInspection`).
#[derive(Debug, Clone, Copy, Serialize)]
pub struct CrossFileCounts {
    pub feature_present: bool,
    pub dictionary_count: u64,
    pub dictionary_bytes: u64,
    pub dictionary_backed_chunks: u64,
    pub chunk_group_count: u64,
}

/// Reconstructive transform presence and counts (mirrors
/// `ReconstructionInspection`).
#[derive(Debug, Clone, Copy, Serialize)]
pub struct ReconstructionCounts {
    pub feature_present: bool,
    pub object_count: u64,
    pub object_bytes: u64,
    pub chunk_count: u64,
}

/// Everything this crate's JSON rows report about an opened archive's
/// logical structure. Built once, right after `open`/`open_stream`/
/// `open_encrypted_authenticated`, by [`from_inspection`].
#[derive(Debug, Clone, Serialize)]
pub struct ArchiveCounts {
    pub entry_count: u64,
    pub total_logical_bytes: u64,
    pub planner_id: String,
    pub chunker_id: String,
    pub codec_usage: Vec<CodecUsage>,
    pub plans: Vec<PlanSummary>,
    pub chunks: ChunkStatistics,
    pub cross_file: CrossFileCounts,
    pub reconstruction: ReconstructionCounts,
    /// `ArchiveInspection::whole_object.region_count`: ReconstructionRegions
    /// whose representations live outside ChunkData.
    pub whole_object_region_count: u64,
}

/// Copies the fields this crate needs out of a real `ArchiveInspection`.
#[must_use]
pub fn from_inspection(inspection: &ArchiveInspection) -> ArchiveCounts {
    ArchiveCounts {
        entry_count: inspection.entry_count,
        total_logical_bytes: inspection.total_logical_bytes,
        planner_id: inspection.planner_id.clone(),
        chunker_id: inspection.chunker_id.clone(),
        codec_usage: inspection
            .codec_usage
            .iter()
            .map(|usage| CodecUsage {
                codec: usage.codec.clone(),
                chunk_count: usage.chunk_count,
                logical_bytes: usage.logical_bytes,
                stored_bytes: usage.stored_bytes,
            })
            .collect(),
        plans: inspection
            .plans
            .iter()
            .map(|plan| PlanSummary {
                plan_id: plan.plan_id,
                codec: plan.codec.clone(),
                transform_signature: if plan.transforms.is_empty() {
                    "none".to_owned()
                } else {
                    plan.transforms.join("+")
                },
            })
            .collect(),
        chunks: ChunkStatistics {
            unique_chunk_count: inspection.chunks.unique_chunk_count,
            logical_chunk_references: inspection.chunks.logical_chunk_references,
            unique_plaintext_bytes: inspection.chunks.unique_plaintext_bytes,
            deduplicated_bytes: inspection.chunks.deduplicated_bytes,
            dedup_ratio_milli: inspection.chunks.dedup_ratio_milli,
            minimum_chunk_bytes: inspection.chunks.minimum_chunk_bytes,
            average_chunk_bytes: inspection.chunks.average_chunk_bytes,
            maximum_chunk_bytes: inspection.chunks.maximum_chunk_bytes,
        },
        cross_file: CrossFileCounts {
            feature_present: inspection.cross_file.feature_present,
            dictionary_count: inspection.cross_file.dictionary_count,
            dictionary_bytes: inspection.cross_file.dictionary_bytes,
            dictionary_backed_chunks: inspection.cross_file.dictionary_backed_chunks,
            chunk_group_count: inspection.cross_file.chunk_group_count,
        },
        reconstruction: ReconstructionCounts {
            feature_present: inspection.reconstruction.feature_present,
            object_count: inspection.reconstruction.object_count,
            object_bytes: inspection.reconstruction.object_bytes,
            chunk_count: inspection.reconstruction.chunk_count,
        },
        whole_object_region_count: inspection.whole_object.region_count,
    }
}

/// Error opening an archive's bytes purely to derive [`ArchiveCounts`] (used
/// by `ebr-inspect-bytes`, which has no fresh `Archive` to hand -- only an
/// already-encoded file's bytes).
#[derive(Debug)]
pub enum OpenCountsError {
    Entrybound(Diagnostic),
    MissingPassword,
}

impl fmt::Display for OpenCountsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OpenCountsError::Entrybound(d) => write!(f, "entrybound: {d}"),
            OpenCountsError::MissingPassword => {
                write!(f, "archive is encrypted but no unlock password was given")
            }
        }
    }
}

impl std::error::Error for OpenCountsError {}

impl From<Diagnostic> for OpenCountsError {
    fn from(d: Diagnostic) -> Self {
        OpenCountsError::Entrybound(d)
    }
}

/// Opens `bytes` under the given layout/encryption facts and derives
/// [`ArchiveCounts`] plus whether every `VerificationReport` guarantee held.
/// Standalone: unlike [`crate::pack::pack_once`], this needs no source
/// directory, only an already-encoded archive's bytes.
pub fn from_bytes(
    bytes: &[u8],
    layout: Layout,
    encrypted: bool,
    password: Option<&[u8]>,
) -> Result<(ArchiveCounts, bool), OpenCountsError> {
    let opened = if encrypted {
        let password = password.ok_or(OpenCountsError::MissingPassword)?;
        let options = entrybound::crypto::EncryptedOpenOptions::new(Some(
            entrybound::crypto::Unlock::Password(password),
        ));
        entrybound::crypto::open_encrypted_authenticated(bytes, options)?.opened
    } else {
        match layout {
            Layout::Indexed => entrybound::ecf::open(bytes)?,
            Layout::Stream => {
                entrybound::ecf::open_stream(std::io::Cursor::new(bytes.to_vec()))?.opened
            }
        }
    };
    let verified = verification_all_true(&opened.report);
    let inspection = entrybound::archive::inspect(&opened)?;
    Ok((from_inspection(&inspection), verified))
}

fn verification_all_true(report: &entrybound::ecf::VerificationReport) -> bool {
    report.canonical_encoding
        && report.container_structure
        && report.section_integrity
        && report.semantic_invariants
        && report.chunk_integrity
        && report.dictionary_integrity
        && report.reconstruction_integrity
        && report.chunk_group_integrity
        && report.access_costs
        && report.content_integrity
        && report.entry_identities
        && report.lai
        && report.pcr
        && report.aux
        && report.pci_computed
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn scratch_input(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "ebr-pack-counts-test-{label}-{}",
            std::process::id()
        ));
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.txt"), b"hello world".repeat(32)).unwrap();
        dir
    }

    #[test]
    fn mirrors_a_real_archive_inspection() {
        let input = scratch_input("basic");
        let archive = entrybound::archive::plan_directory(
            &input,
            entrybound::archive::PackOptions {
                profile: entrybound::planner::CompressionProfile::Fast,
                ..Default::default()
            },
        )
        .unwrap();
        let encoded =
            entrybound::ecf::encode(&archive, entrybound::ecf::WriteOptions::default()).unwrap();
        let opened = entrybound::ecf::open(&encoded.bytes).unwrap();
        let inspection = entrybound::archive::inspect(&opened).unwrap();

        let counts = from_inspection(&inspection);
        assert_eq!(counts.entry_count, inspection.entry_count);
        assert_eq!(counts.total_logical_bytes, inspection.total_logical_bytes);
        assert_eq!(counts.planner_id, inspection.planner_id);
        assert!(!counts.codec_usage.is_empty());
        assert!(!counts.plans.is_empty());
        assert_eq!(
            counts.chunks.unique_chunk_count,
            inspection.chunks.unique_chunk_count
        );

        std::fs::remove_dir_all(&input).ok();
    }

    #[test]
    fn from_bytes_matches_from_inspection_for_a_fresh_archive() {
        let input = scratch_input("from-bytes");
        let archive = entrybound::archive::plan_directory(
            &input,
            entrybound::archive::PackOptions {
                profile: entrybound::planner::CompressionProfile::Fast,
                ..Default::default()
            },
        )
        .unwrap();
        let encoded =
            entrybound::ecf::encode(&archive, entrybound::ecf::WriteOptions::default()).unwrap();

        let (counts, verified) = from_bytes(&encoded.bytes, Layout::Indexed, false, None).unwrap();
        assert!(verified);
        assert!(counts.entry_count > 0);
        assert!(!counts.plans.is_empty());

        std::fs::remove_dir_all(&input).ok();
    }

    #[test]
    fn from_bytes_refuses_an_encrypted_archive_without_a_password() {
        let input = scratch_input("from-bytes-missing-password");
        let creds = crate::encrypt::TestCredentials::generate().unwrap();
        let write_options = entrybound::crypto::EncryptedWriteOptions {
            password: Some(&creds.password),
            ..Default::default()
        };
        let encrypted = entrybound::crypto::pack_directory_encrypted(
            &input,
            entrybound::archive::PackOptions {
                profile: entrybound::planner::CompressionProfile::Fast,
                ..Default::default()
            },
            write_options,
        )
        .unwrap();

        let result = from_bytes(&encrypted.bytes, Layout::Indexed, true, None);
        assert!(matches!(result, Err(OpenCountsError::MissingPassword)));

        std::fs::remove_dir_all(&input).ok();
    }
}
