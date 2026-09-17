//! Exact section-level byte accounting for an encoded archive, derived by
//! walking the container with production parsing APIs.
//!
//! # Plaintext INDEXED archives: granular, exactly to the byte
//!
//! `entrybound::ecf::open_indexed_random` (production's own random-access
//! opener) already walks and authenticates every top-level section and
//! returns their exact on-disk extents as `RandomAccessMetadata::
//! section_directory` (`entrybound::ecf::RandomAccessSection { kind, offset,
//! payload_length }`, `offset` pointing at the section's own 64-byte header
//! -- see that reader's `walk_sections`, which itself refuses to open an
//! archive whose sections do not tile the file exactly from the fixed
//! 256-byte preamble to the fixed 128-byte footer with no gap). That
//! directory alone accounts for the preamble, footer, Descriptor,
//! TransformPlans, Dictionaries, ChunkGroups, ReconstructionData,
//! ReconstructionRegions, ManifestRecords, Fidelity (this crate's "metadata
//! records"), and Index sections exactly.
//!
//! The one section this crate additionally walks *inside* is ChunkData:
//! `entrybound::research::ecf::{chunk_frame_header_len, parse_chunk_frame_header}`
//! (the same header-width constant and header parser production's own
//! decoder uses, reached through the default-off `research-internals`
//! feature) split each physical Chunk frame into its fixed-width header and
//! its `stored_len`-byte payload, and each payload is attributed to the
//! codec and transform pipeline its `plan_ref` names (via the archive's own
//! recorded `TransformPlan`s -- see [`crate::counts::PlanSummary`]).
//!
//! [`indexed_byte_accounting`] checks two sums and fails loudly (returns
//! [`BytesError::Imbalance`], never a fabricated number) if either is off:
//! `preamble + footer + sum(section header + payload)` must equal the exact
//! file size, and -- harness review round 1, finding R1-08 -- so must the
//! sum of the *named* buckets a consumer actually reads (`descriptor_bytes`,
//! ..., `chunk_data_section_header_bytes`, `chunk_frame_header_bytes`,
//! `chunk_payload_bytes`, `other_section_bytes`, `padding_bytes`,
//! `signature_bytes`). The first sum alone is nearly tautological (the
//! section extents come from the same authenticated walk that refuses gaps);
//! the second catches a section kind or header that no named bucket
//! claims. Earlier revisions left every ChunkData section's own 64-byte
//! header, and any unrecognized section kind, out of every named bucket.
//!
//! [`cross_check_indexed`] is the *independent* check: it compares the
//! frame walk's `chunk_payload_bytes` and frame count against
//! `entrybound::archive::inspect`'s per-codec `stored_bytes`/`chunk_count`,
//! which production derives from the Index and the Chunk table rather than
//! from walking ChunkData frames. It applies when the archive has no
//! whole-object ReconstructionRegions (region representations are counted by
//! inspection but stored outside ChunkData); otherwise it says so.
//!
//! # STREAM and encrypted archives: coarser, still exactly cross-checked
//!
//! Production exposes no equivalent physical directory for STREAM's tagged
//! item stream (`entrybound::ecf::stream`'s per-item tag/length wire layout
//! is private; only aggregate counts -- `StreamReport::{chunk_frames,
//! manifest_records, body_len, total_len}` -- are public) or for an
//! encrypted container (encrypted archives are deliberately
//! metadata-private; `entrybound::crypto::inspect_encrypted`'s public view
//! is `PublicCryptoInspection::total_container_bytes` plus segment/recipient
//! counts, not a section-by-section extent list). [`stream_byte_accounting`]
//! and [`encrypted_byte_accounting`] report a single lump body/segment
//! figure instead of hand-parsing a private wire format from outside the
//! crate, and say so in [`ByteAccounting::note`]. Both still cross-check
//! their lump total against the file's actual size and fail loudly on a
//! mismatch: the accounting is coarser, never approximate.

use crate::counts::{ArchiveCounts, PlanSummary};
use entrybound::crypto::{CryptoPolicy, Unlock, inspect_encrypted};
use entrybound::diagnostics::Diagnostic;
use entrybound::ecf::{
    FEATURE_CROSS_FILE_COMPRESSION_V1, FEATURE_WHOLE_OBJECT_RECONSTRUCTION_V1, FOOTER_LEN,
    PREAMBLE_LEN, SECTION_HEADER_LEN, STREAM_FOOTER_LEN, open_indexed_random, open_stream,
};
use entrybound::random_access::{MemoryRandomReadSource, RandomAccessPolicy};
use entrybound::research::ecf::{chunk_frame_header_len, parse_chunk_frame_header};
use serde::Serialize;
use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug)]
pub enum BytesError {
    Entrybound(Diagnostic),
    /// The accounted total did not equal the file's actual size. Carries
    /// both numbers so a caller can log the exact discrepancy rather than
    /// just "it didn't match".
    Imbalance {
        file_size: u64,
        accounted_bytes: u64,
    },
    /// A declared extent ran past the bytes actually available (a Chunk
    /// frame header or payload that does not fit inside its section).
    Truncated(String),
}

impl fmt::Display for BytesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BytesError::Entrybound(d) => write!(f, "entrybound: {d}"),
            BytesError::Imbalance {
                file_size,
                accounted_bytes,
            } => write!(
                f,
                "byte accounting does not sum to the file size: accounted \
                 {accounted_bytes} bytes, file is {file_size} bytes"
            ),
            BytesError::Truncated(msg) => write!(f, "byte accounting walk: {msg}"),
        }
    }
}

impl std::error::Error for BytesError {}

impl From<Diagnostic> for BytesError {
    fn from(d: Diagnostic) -> Self {
        BytesError::Entrybound(d)
    }
}

/// One top-level section's exact on-disk extent: a fixed-width header
/// (`SECTION_HEADER_LEN` bytes, always 64 today) plus its declared payload.
#[derive(Debug, Clone, Serialize)]
pub struct SectionBytes {
    pub kind: String,
    pub header_bytes: u64,
    pub payload_bytes: u64,
}

/// Exact section-level byte accounting for one encoded archive. See the
/// module docs for what "exact" and "granular" mean per layout.
#[derive(Debug, Clone, Serialize)]
pub struct ByteAccounting {
    pub file_size: u64,
    /// `true` for plaintext INDEXED archives (every field below is a real,
    /// independently walked figure); `false` for STREAM/encrypted archives,
    /// where several fields are `0` because production exposes no public
    /// API to derive them (see [`ByteAccounting::note`]).
    pub granular: bool,
    pub preamble_bytes: u64,
    pub footer_bytes: u64,
    /// Every top-level section this archive's directory reported, in
    /// on-disk order. Exhaustive for INDEXED; a single synthetic lump entry
    /// for STREAM/encrypted (see `note`).
    pub sections: Vec<SectionBytes>,
    pub descriptor_bytes: u64,
    pub transform_plan_bytes: u64,
    pub manifest_bytes: u64,
    /// The Fidelity section: the FidelityReport plus, when present, the
    /// ConversionProvenance/LegacyPreservation records -- entrybound's
    /// `SectionKind` enumeration has no separate kind for those, so this is
    /// the closest exact mapping to this task's "metadata records" bucket.
    pub metadata_bytes: u64,
    pub index_bytes: u64,
    pub dictionary_bytes: u64,
    pub chunk_group_bytes: u64,
    /// ReconstructionData and ReconstructionRegions sections combined.
    pub reconstruction_bytes: u64,
    /// The 64-byte section header of every ChunkData section (the frames
    /// inside are `chunk_frame_header_bytes` + `chunk_payload_bytes`).
    pub chunk_data_section_header_bytes: u64,
    pub chunk_frame_header_bytes: u64,
    pub chunk_payload_bytes: u64,
    /// Number of Chunk frames walked inside ChunkData.
    pub chunk_frame_count: u64,
    /// Header + payload of any section whose kind no named bucket above
    /// claims (always `0` for the ten `SectionKind`s production defines
    /// today; a new kind lands here instead of silently vanishing).
    pub other_section_bytes: u64,
    pub other_section_kinds: Vec<String>,
    pub chunk_payload_bytes_by_codec: BTreeMap<String, u64>,
    pub chunk_payload_bytes_by_transform: BTreeMap<String, u64>,
    /// Always `0` today: production's INDEXED section walk refuses any gap
    /// between sections (see module docs), and no signature/padding
    /// mechanism exists in the current format. Kept as an explicit,
    /// always-checked field rather than omitted, so a future format
    /// revision that *does* add padding fails this module's balance check
    /// loudly instead of silently under-accounting.
    pub padding_bytes: u64,
    /// Always `0` today: no in-band signature section exists yet (see
    /// `padding_bytes`).
    pub signature_bytes: u64,
    pub accounted_bytes: u64,
    /// Sum of every named bucket (`preamble_bytes` through `signature_bytes`,
    /// excluding the `sections` list and the by-codec/by-transform maps,
    /// which re-divide `chunk_payload_bytes`). Equals `file_size` on `Ok`.
    pub named_bucket_total_bytes: u64,
    /// `accounted_bytes == file_size`. Always `true` on `Ok` -- a mismatch
    /// is returned as [`BytesError::Imbalance`], never carried in a `false`
    /// here, so a caller can never silently ignore an imbalance by skipping
    /// this field.
    pub balanced: bool,
    /// Explains a coarser (`granular: false`) accounting; `None` for a fully
    /// granular one.
    pub note: Option<String>,
    /// Encrypted archives only: the number of ciphertext segments
    /// `entrybound::crypto::inspect_encrypted` reports.
    pub segment_count: Option<u64>,
    /// Encrypted archives only: the number of recipient stanzas.
    pub recipient_count: Option<u64>,
}

/// Granular byte accounting for a plaintext INDEXED archive's complete
/// encoded bytes. `plans` should be `ArchiveCounts::plans` from the same
/// archive (see [`crate::counts`]), used to attribute each Chunk frame's
/// stored bytes to a codec and transform pipeline.
pub fn indexed_byte_accounting(
    bytes: &[u8],
    plans: &[PlanSummary],
) -> Result<ByteAccounting, BytesError> {
    let plan_lookup: BTreeMap<u64, &PlanSummary> =
        plans.iter().map(|plan| (plan.plan_id, plan)).collect();

    let source = MemoryRandomReadSource::new(bytes.to_vec());
    let archive = open_indexed_random(source, RandomAccessPolicy::default())?;
    let meta = archive.metadata();

    let extended = meta.descriptor.features.incompat & FEATURE_CROSS_FILE_COMPRESSION_V1 != 0;
    let whole_object =
        meta.descriptor.features.incompat & FEATURE_WHOLE_OBJECT_RECONSTRUCTION_V1 != 0;
    let header_len = usize::try_from(chunk_frame_header_len(extended))
        .map_err(|_| BytesError::Truncated("Chunk frame header length exceeds usize".into()))?;

    let mut sections = Vec::with_capacity(meta.section_directory.len());
    let mut descriptor_bytes = 0u64;
    let mut transform_plan_bytes = 0u64;
    let mut manifest_bytes = 0u64;
    let mut metadata_bytes = 0u64;
    let mut index_bytes = 0u64;
    let mut dictionary_bytes = 0u64;
    let mut chunk_group_bytes = 0u64;
    let mut reconstruction_bytes = 0u64;
    let mut chunk_data_section_header_bytes = 0u64;
    let mut chunk_frame_header_bytes = 0u64;
    let mut chunk_payload_bytes = 0u64;
    let mut chunk_frame_count = 0u64;
    let mut other_section_bytes = 0u64;
    let mut other_section_kinds: Vec<String> = Vec::new();
    let mut by_codec: BTreeMap<String, u64> = BTreeMap::new();
    let mut by_transform: BTreeMap<String, u64> = BTreeMap::new();

    for section in meta.section_directory.iter() {
        let header_bytes = SECTION_HEADER_LEN;
        let payload_bytes = section.payload_length;
        let section_total = header_bytes + payload_bytes;
        sections.push(SectionBytes {
            kind: section.kind.clone(),
            header_bytes,
            payload_bytes,
        });

        match section.kind.as_str() {
            "Descriptor" => descriptor_bytes += section_total,
            "TransformPlans" => transform_plan_bytes += section_total,
            "ManifestRecords" => manifest_bytes += section_total,
            "Fidelity" => metadata_bytes += section_total,
            "Index" => index_bytes += section_total,
            "Dictionaries" => dictionary_bytes += section_total,
            "ChunkGroups" => chunk_group_bytes += section_total,
            "ReconstructionData" | "ReconstructionRegions" => reconstruction_bytes += section_total,
            "ChunkData" => {
                chunk_data_section_header_bytes += header_bytes;
                let payload_start = usize::try_from(section.offset + header_bytes)
                    .map_err(|_| BytesError::Truncated("section offset exceeds usize".into()))?;
                let payload_len = usize::try_from(payload_bytes)
                    .map_err(|_| BytesError::Truncated("payload length exceeds usize".into()))?;
                let payload_end = payload_start.checked_add(payload_len).ok_or_else(|| {
                    BytesError::Truncated("ChunkData section extent overflows usize".into())
                })?;
                let payload = bytes.get(payload_start..payload_end).ok_or_else(|| {
                    BytesError::Truncated("ChunkData section runs past the end of the file".into())
                })?;

                let mut cursor = 0usize;
                while cursor < payload.len() {
                    let header_end = cursor
                        .checked_add(header_len)
                        .filter(|&end| end <= payload.len())
                        .ok_or_else(|| {
                            BytesError::Truncated("Chunk frame header is truncated".into())
                        })?;
                    let parsed = parse_chunk_frame_header(
                        &payload[cursor..header_end],
                        extended,
                        whole_object,
                    )?;
                    chunk_frame_header_bytes += header_len as u64;
                    chunk_frame_count += 1;
                    let stored = parsed.stored_len;
                    chunk_payload_bytes += stored;

                    let (codec, transform_signature) = plan_lookup
                        .get(&parsed.plan_ref)
                        .map(|plan| (plan.codec.clone(), plan.transform_signature.clone()))
                        .unwrap_or_else(|| {
                            (
                                format!("unknown-plan-{}", parsed.plan_ref),
                                format!("unknown-plan-{}", parsed.plan_ref),
                            )
                        });
                    *by_codec.entry(codec).or_insert(0) += stored;
                    *by_transform.entry(transform_signature).or_insert(0) += stored;

                    let stored_usize = usize::try_from(stored).map_err(|_| {
                        BytesError::Truncated("Chunk frame stored_len exceeds usize".into())
                    })?;
                    cursor = header_end
                        .checked_add(stored_usize)
                        .filter(|&end| end <= payload.len())
                        .ok_or_else(|| {
                            BytesError::Truncated("Chunk frame payload is truncated".into())
                        })?;
                }
                if cursor != payload.len() {
                    return Err(BytesError::Truncated(
                        "ChunkData section did not end exactly at its declared payload length"
                            .into(),
                    ));
                }
            }
            other => {
                other_section_bytes += section_total;
                if !other_section_kinds.iter().any(|kind| kind == other) {
                    other_section_kinds.push(other.to_owned());
                }
            }
        }
    }

    let preamble_bytes = PREAMBLE_LEN;
    let footer_bytes = FOOTER_LEN;
    let section_bytes_total: u64 = sections
        .iter()
        .map(|section| section.header_bytes + section.payload_bytes)
        .sum();
    let accounted_bytes = preamble_bytes + footer_bytes + section_bytes_total;
    let file_size = bytes.len() as u64;
    if accounted_bytes != file_size {
        return Err(BytesError::Imbalance {
            file_size,
            accounted_bytes,
        });
    }
    let named_bucket_total_bytes = preamble_bytes
        + footer_bytes
        + descriptor_bytes
        + transform_plan_bytes
        + manifest_bytes
        + metadata_bytes
        + index_bytes
        + dictionary_bytes
        + chunk_group_bytes
        + reconstruction_bytes
        + chunk_data_section_header_bytes
        + chunk_frame_header_bytes
        + chunk_payload_bytes
        + other_section_bytes;
    if named_bucket_total_bytes != file_size {
        return Err(BytesError::Imbalance {
            file_size,
            accounted_bytes: named_bucket_total_bytes,
        });
    }

    Ok(ByteAccounting {
        file_size,
        granular: true,
        preamble_bytes,
        footer_bytes,
        sections,
        descriptor_bytes,
        transform_plan_bytes,
        manifest_bytes,
        metadata_bytes,
        index_bytes,
        dictionary_bytes,
        chunk_group_bytes,
        reconstruction_bytes,
        chunk_data_section_header_bytes,
        chunk_frame_header_bytes,
        chunk_payload_bytes,
        chunk_frame_count,
        other_section_bytes,
        other_section_kinds,
        chunk_payload_bytes_by_codec: by_codec,
        chunk_payload_bytes_by_transform: by_transform,
        padding_bytes: 0,
        signature_bytes: 0,
        accounted_bytes,
        named_bucket_total_bytes,
        balanced: true,
        note: None,
        segment_count: None,
        recipient_count: None,
    })
}

/// Coarse byte accounting for a STREAM archive: see the module docs for why
/// this cannot be as granular as [`indexed_byte_accounting`].
pub fn stream_byte_accounting(bytes: &[u8]) -> Result<ByteAccounting, BytesError> {
    let sequential = open_stream(std::io::Cursor::new(bytes.to_vec()))?;
    let report = sequential.stream;

    let preamble_bytes = PREAMBLE_LEN;
    let footer_bytes = STREAM_FOOTER_LEN;
    let body_bytes = report.body_len;
    let accounted_bytes = preamble_bytes + footer_bytes + body_bytes;
    let file_size = bytes.len() as u64;
    if accounted_bytes != file_size || report.total_len != file_size {
        return Err(BytesError::Imbalance {
            file_size,
            accounted_bytes,
        });
    }

    Ok(ByteAccounting {
        file_size,
        granular: false,
        preamble_bytes,
        footer_bytes,
        sections: vec![SectionBytes {
            kind: "StreamBody".to_owned(),
            header_bytes: 0,
            payload_bytes: body_bytes,
        }],
        descriptor_bytes: 0,
        transform_plan_bytes: 0,
        manifest_bytes: 0,
        metadata_bytes: 0,
        index_bytes: 0,
        dictionary_bytes: 0,
        chunk_group_bytes: 0,
        reconstruction_bytes: 0,
        chunk_data_section_header_bytes: 0,
        chunk_frame_header_bytes: 0,
        chunk_payload_bytes: 0,
        chunk_frame_count: 0,
        other_section_bytes: body_bytes,
        other_section_kinds: vec!["StreamBody".to_owned()],
        chunk_payload_bytes_by_codec: BTreeMap::new(),
        chunk_payload_bytes_by_transform: BTreeMap::new(),
        padding_bytes: 0,
        signature_bytes: 0,
        accounted_bytes,
        named_bucket_total_bytes: accounted_bytes,
        balanced: true,
        note: Some(format!(
            "STREAM layout: entrybound exposes no public per-item physical directory (unlike \
             INDEXED's RandomAccessMetadata::section_directory), so body_bytes is a single lump \
             (StreamReport::body_len), not a per-section/per-Chunk-frame breakdown. StreamReport \
             does report chunk_frames={} and manifest_records={} as counts, surfaced separately \
             in this row's counts, not here.",
            report.chunk_frames, report.manifest_records
        )),
        segment_count: None,
        recipient_count: None,
    })
}

/// Coarse byte accounting for an encrypted archive: see the module docs for
/// why this cannot be as granular as [`indexed_byte_accounting`].
pub fn encrypted_byte_accounting(
    bytes: &[u8],
    password: &[u8],
) -> Result<ByteAccounting, BytesError> {
    let inspection = inspect_encrypted(
        bytes,
        Some(Unlock::Password(password)),
        CryptoPolicy::default(),
    )?;
    let file_size = bytes.len() as u64;
    let total = inspection.public.total_container_bytes;
    if total != file_size {
        return Err(BytesError::Imbalance {
            file_size,
            accounted_bytes: total,
        });
    }

    let preamble_bytes = PREAMBLE_LEN;
    let body_bytes = total.saturating_sub(preamble_bytes);
    Ok(ByteAccounting {
        file_size,
        granular: false,
        preamble_bytes,
        footer_bytes: 0,
        sections: vec![SectionBytes {
            kind: "EncryptedBody".to_owned(),
            header_bytes: 0,
            payload_bytes: body_bytes,
        }],
        descriptor_bytes: 0,
        transform_plan_bytes: 0,
        manifest_bytes: 0,
        metadata_bytes: 0,
        index_bytes: 0,
        dictionary_bytes: 0,
        chunk_group_bytes: 0,
        reconstruction_bytes: 0,
        chunk_data_section_header_bytes: 0,
        chunk_frame_header_bytes: 0,
        chunk_payload_bytes: 0,
        chunk_frame_count: 0,
        other_section_bytes: body_bytes,
        other_section_kinds: vec!["EncryptedBody".to_owned()],
        chunk_payload_bytes_by_codec: BTreeMap::new(),
        chunk_payload_bytes_by_transform: BTreeMap::new(),
        padding_bytes: 0,
        signature_bytes: 0,
        accounted_bytes: total,
        named_bucket_total_bytes: preamble_bytes + body_bytes,
        balanced: true,
        note: Some(format!(
            "encrypted archive: entrybound's encrypted containers are metadata-private by \
             design, so no public API exposes their internal segment layout; this reports only \
             the publicly-verifiable total ({total} bytes over {segments:?} ciphertext segments) \
             via entrybound::crypto::inspect_encrypted, folded here into preamble_bytes plus a \
             single EncryptedBody lump (segments and the encrypted footer combined, since the \
             encrypted footer's exact width is not a public constant), not a \
             per-section/per-Chunk-frame breakdown.",
            total = total,
            segments = inspection.public.segment_count,
        )),
        segment_count: inspection.public.segment_count,
        recipient_count: Some(inspection.public.recipient_count),
    })
}

/// The independent INDEXED cross-check described in the module docs.
#[derive(Debug, Clone, Serialize)]
pub struct IndexedCrossCheck {
    /// `false` when the archive has whole-object ReconstructionRegions, whose
    /// representations `inspect` counts but ChunkData does not hold.
    pub applicable: bool,
    pub frame_walk_payload_bytes: u64,
    pub inspection_stored_bytes: u64,
    pub frame_walk_frame_count: u64,
    pub inspection_chunk_count: u64,
    /// Both pairs equal (`true` whenever `applicable` is `false`).
    pub matched: bool,
}

/// Compares a granular INDEXED accounting against `inspect`'s per-codec
/// totals for the same archive. Returns `Err(BytesError::Imbalance)` when the
/// check applies and either pair differs.
pub fn cross_check_indexed(
    accounting: &ByteAccounting,
    counts: &ArchiveCounts,
) -> Result<IndexedCrossCheck, BytesError> {
    let applicable = accounting.granular && counts.whole_object_region_count == 0;
    let inspection_stored_bytes: u64 = counts.codec_usage.iter().map(|u| u.stored_bytes).sum();
    let inspection_chunk_count: u64 = counts.codec_usage.iter().map(|u| u.chunk_count).sum();
    let matched = !applicable
        || (inspection_stored_bytes == accounting.chunk_payload_bytes
            && inspection_chunk_count == accounting.chunk_frame_count);
    if !matched {
        return Err(BytesError::Truncated(format!(
            "independent cross-check failed: frame walk found {} frames / {} payload bytes, \
             inspection reports {} chunks / {} stored bytes",
            accounting.chunk_frame_count,
            accounting.chunk_payload_bytes,
            inspection_chunk_count,
            inspection_stored_bytes
        )));
    }
    Ok(IndexedCrossCheck {
        applicable,
        frame_walk_payload_bytes: accounting.chunk_payload_bytes,
        inspection_stored_bytes,
        frame_walk_frame_count: accounting.chunk_frame_count,
        inspection_chunk_count,
        matched,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn scratch_input(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "ebr-pack-bytes-test-{label}-{}",
            std::process::id()
        ));
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        std::fs::write(dir.join("a.txt"), b"entrybound research".repeat(200)).unwrap();
        std::fs::write(dir.join("sub").join("b.txt"), vec![9u8; 8192]).unwrap();
        dir
    }

    fn encode_indexed(input: &std::path::Path) -> (Vec<u8>, Vec<PlanSummary>) {
        let archive = entrybound::archive::plan_directory(
            input,
            entrybound::archive::PackOptions {
                profile: entrybound::planner::CompressionProfile::Balanced,
                ..Default::default()
            },
        )
        .unwrap();
        let encoded =
            entrybound::ecf::encode(&archive, entrybound::ecf::WriteOptions::default()).unwrap();
        let opened = entrybound::ecf::open(&encoded.bytes).unwrap();
        let inspection = entrybound::archive::inspect(&opened).unwrap();
        let counts = crate::counts::from_inspection(&inspection);
        (encoded.bytes, counts.plans)
    }

    #[test]
    fn indexed_accounting_sums_exactly_to_file_size() {
        let input = scratch_input("indexed-sum");
        let (bytes, plans) = encode_indexed(&input);
        let accounting = indexed_byte_accounting(&bytes, &plans).unwrap();

        assert!(accounting.granular);
        assert_eq!(accounting.file_size, bytes.len() as u64);
        assert_eq!(accounting.accounted_bytes, accounting.file_size);
        assert!(accounting.balanced);
        assert!(accounting.chunk_payload_bytes > 0);
        assert!(accounting.chunk_frame_header_bytes > 0);
        assert!(!accounting.chunk_payload_bytes_by_codec.is_empty());
        assert_eq!(accounting.padding_bytes, 0);
        assert_eq!(accounting.signature_bytes, 0);

        // Every named per-section field must be independently derivable
        // from the same `sections` list this row also reports.
        let manifest_from_sections: u64 = accounting
            .sections
            .iter()
            .filter(|s| s.kind == "ManifestRecords")
            .map(|s| s.header_bytes + s.payload_bytes)
            .sum();
        assert_eq!(manifest_from_sections, accounting.manifest_bytes);

        std::fs::remove_dir_all(&input).ok();
    }

    #[test]
    fn indexed_chunk_frame_bytes_never_exceed_the_chunk_data_section() {
        let input = scratch_input("indexed-chunkdata-bound");
        let (bytes, plans) = encode_indexed(&input);
        let accounting = indexed_byte_accounting(&bytes, &plans).unwrap();

        let chunk_data_total: u64 = accounting
            .sections
            .iter()
            .filter(|s| s.kind == "ChunkData")
            .map(|s| s.header_bytes + s.payload_bytes)
            .sum();
        assert_eq!(
            accounting.chunk_frame_header_bytes
                + accounting.chunk_payload_bytes
                + SECTION_HEADER_LEN,
            chunk_data_total
        );
        assert_eq!(
            accounting.chunk_data_section_header_bytes,
            SECTION_HEADER_LEN
        );

        std::fs::remove_dir_all(&input).ok();
    }

    #[test]
    fn stream_accounting_sums_exactly_to_file_size() {
        let input = scratch_input("stream-sum");
        let archive = entrybound::archive::plan_directory(
            &input,
            entrybound::archive::PackOptions {
                profile: entrybound::planner::CompressionProfile::Fast,
                ..Default::default()
            },
        )
        .unwrap();
        let mut buf = Vec::new();
        entrybound::ecf::encode_stream(
            &archive,
            entrybound::ecf::StreamWriteOptions::default(),
            &mut buf,
        )
        .unwrap();

        let accounting = stream_byte_accounting(&buf).unwrap();
        assert!(!accounting.granular);
        assert_eq!(accounting.accounted_bytes, buf.len() as u64);
        assert!(accounting.balanced);
        assert!(accounting.note.is_some());

        std::fs::remove_dir_all(&input).ok();
    }

    #[test]
    fn indexed_accounting_rejects_a_truncated_file() {
        let input = scratch_input("indexed-truncated");
        let (bytes, plans) = encode_indexed(&input);
        let truncated = &bytes[..bytes.len() - 16];
        assert!(indexed_byte_accounting(truncated, &plans).is_err());
        std::fs::remove_dir_all(&input).ok();
    }

    #[test]
    fn named_buckets_sum_exactly_to_the_file_size() {
        let input = scratch_input("indexed-named-buckets");
        let (bytes, plans) = encode_indexed(&input);
        let accounting = indexed_byte_accounting(&bytes, &plans).unwrap();
        let named = accounting.preamble_bytes
            + accounting.footer_bytes
            + accounting.descriptor_bytes
            + accounting.transform_plan_bytes
            + accounting.manifest_bytes
            + accounting.metadata_bytes
            + accounting.index_bytes
            + accounting.dictionary_bytes
            + accounting.chunk_group_bytes
            + accounting.reconstruction_bytes
            + accounting.chunk_data_section_header_bytes
            + accounting.chunk_frame_header_bytes
            + accounting.chunk_payload_bytes
            + accounting.other_section_bytes
            + accounting.padding_bytes
            + accounting.signature_bytes;
        assert_eq!(named, bytes.len() as u64);
        assert_eq!(accounting.named_bucket_total_bytes, named);
        assert_eq!(accounting.other_section_bytes, 0);
        let by_codec: u64 = accounting.chunk_payload_bytes_by_codec.values().sum();
        assert_eq!(by_codec, accounting.chunk_payload_bytes);
        std::fs::remove_dir_all(&input).ok();
    }

    #[test]
    fn frame_walk_matches_inspection_independently() {
        let input = scratch_input("indexed-cross-check");
        let archive = entrybound::archive::plan_directory(
            &input,
            entrybound::archive::PackOptions {
                profile: entrybound::planner::CompressionProfile::Dense,
                ..Default::default()
            },
        )
        .unwrap();
        let encoded =
            entrybound::ecf::encode(&archive, entrybound::ecf::WriteOptions::default()).unwrap();
        let opened = entrybound::ecf::open(&encoded.bytes).unwrap();
        let counts =
            crate::counts::from_inspection(&entrybound::archive::inspect(&opened).unwrap());
        let accounting = indexed_byte_accounting(&encoded.bytes, &counts.plans).unwrap();
        let check = cross_check_indexed(&accounting, &counts).unwrap();
        assert!(check.applicable);
        assert!(check.matched);
        assert!(check.frame_walk_frame_count > 0);

        // A deliberately wrong inspection total must be caught.
        let mut tampered = counts.clone();
        tampered.codec_usage[0].stored_bytes += 1;
        assert!(cross_check_indexed(&accounting, &tampered).is_err());
        std::fs::remove_dir_all(&input).ok();
    }
}
