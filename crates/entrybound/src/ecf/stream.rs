//! Native STREAM layout (`stream-layout-v1`).
//!
//! STREAM is the second physical organization of the same Entrybound Archive
//! Model. It writes to a sink that implements only [`std::io::Write`] and reads
//! from a source that implements only [`std::io::Read`]. It carries no Index,
//! it never patches bytes it has already emitted, and every semantic fact is
//! declared exactly once.
//!
//! STREAM and INDEXED differ only in physical organization and access
//! capability. Encoding one validated model under both layouts produces the
//! same LAI, PCR, and AUX; only PCI differs, because PCI is the digest of the
//! exact container bytes.
//!
//! # Body shape
//!
//! ```text
//! preamble (256 bytes, layout = STREAM, declares stream_dedup_window)
//! STREAM_BODY
//!   TRANSFORM_PLANS
//!   [DICTIONARIES] [CHUNK_GROUPS]          (cross-file-compression-v1)
//!   [RECONSTRUCTION_DATA]                  (reconstructive-transform-v1)
//!   [RECONSTRUCTION_REGIONS]               (whole-object-reconstruction-v1)
//!   ( CHUNK_FRAME* MANIFEST_RECORD )*      ContentObject records follow data
//!   MANIFEST_RECORD*                       Entry records
//!   FIDELITY
//!   DESCRIPTOR                             final identities and roots
//! footer (128 bytes, self-locating at EOF)
//! ```
//!
//! Every item begins with a 16-byte tagged header, so a reader never has to
//! guess whether the next bytes are physical content or a semantic record. A
//! `CHUNK_FRAME` item carries an ordinary `EBCH` frame whose own header is the
//! sole authority for its stored length; the item does not repeat it. Every
//! other item is a record item that declares its payload length and payload
//! digest once.
//!
//! # Physical organization
//!
//! STREAM selects an object-major frame order: a ContentObject's not-yet-emitted
//! Chunks are written immediately before its Manifest record. Bounded-lookback
//! ChunkGroups are emitted as one contiguous run at the point their first member
//! is needed, preserving their relative order and therefore their exact stored
//! bytes. The resulting order differs from an INDEXED archive's `physical_order`
//! and is exactly the "physical organization" the layouts are allowed to differ
//! in; no semantic fact changes.
//!
//! # Stream dedup window
//!
//! Let `S(o)` be the number of Chunk frames emitted before object `o`'s own run
//! begins, and let `frame_index(c)` be a Chunk's frame ordinal. A reference from
//! `o` to a Chunk with `frame_index(c) < S(o)` is a historical cross-object
//! deduplication reference of distance `S(o) - frame_index(c)`. The declared
//! `stream_dedup_window` is the largest such distance in the archive, and zero
//! when there are none. A reader therefore retains the Chunks of the run it is
//! currently reading plus at most `window` older Chunks.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::io::{Read, Write};

use sha2::{Digest as ShaDigest, Sha256};

use super::container::{
    ChunkFrameEncoder, ChunkFrameHeader, FOOTER_MAGIC, IndexStatus, OpenedArchive,
    PhysicalDeclaration, Preamble, VerificationReport, chunk_frame_header_len,
    decode_frame_payload, decode_preamble, encode_preamble, enforce_caller_policy,
    enforce_chunk_bounds, enforce_decode_policy, has_codec_transform_feature,
    has_cross_file_feature, has_reconstructive_feature, has_whole_object_feature,
    normalize_descriptor, parse_chunk_frame_header, physical_prefix_from_slices,
    reconstruct_region_members, validate_feature_model,
};
use super::records::{
    DescriptorBody, ManifestRecord, decode_chunk_groups, decode_descriptor, decode_dictionaries,
    decode_fidelity, decode_manifest_record, decode_reconstruction_regions,
    decode_reconstruction_section, decode_transform_plans, decode_transform_plans_v2,
    decode_transform_plans_v3, encode_chunk_groups, encode_content_object_record,
    encode_descriptor, encode_dictionaries, encode_entry_record, encode_fidelity,
    encode_reconstruction_regions, encode_reconstruction_section, encode_transform_plans,
    encode_transform_plans_v2, encode_transform_plans_v3,
};
use super::staging::{ChunkStaging, StagingLimits};
use super::{FEATURE_STREAM_LAYOUT_V1, FOOTER_LEN, FORMAT_NAMESPACE, PREAMBLE_LEN};
use crate::archive::EntryCursorComplexity;
use crate::codec::{PlanMode, aggregate_archive_decode_requirements, plan_mode, validate_plans};
use crate::diagnostics::{Diagnostic, OutcomeClass, ReasonCode, Result};
use crate::eam::{
    Archive, ArchiveDescriptor, ArchiveRole, Chunk, ChunkGroup, ChunkLocation, ContentObject,
    ContentStore, DecodeRequirements, Dictionary, Digest, DigestAlgorithm, Entry, EntrySet,
    IdentityProfile, Index, Layout, ReconstructionAudit, ReconstructionAuditTarget,
    ReconstructionData, ReconstructionFallbackReason, ReconstructionRegion, ResourceBudget,
    TransformPlan,
};
use crate::identity::{
    IdentitySet, PhysicalContainerIdentity, apply_native_identities,
    native_identities_from_verified, sha256_exact,
};

/// Magic that opens every tagged `STREAM_BODY` item.
pub const STREAM_ITEM_MAGIC: [u8; 4] = *b"EBI1";
/// Fixed tagged-item header width.
pub const STREAM_ITEM_HEADER_LEN: u64 = 16;
/// Fixed STREAM trailer width. It matches the INDEXED footer width.
pub const STREAM_FOOTER_LEN: u64 = FOOTER_LEN;
/// Record-item prefix: payload length plus payload digest.
const RECORD_ITEM_PREFIX_LEN: u64 = 40;
const STREAM_ITEM_VERSION: u16 = 1;

/// Tagged `STREAM_BODY` item kinds.
///
/// The discriminants are wire values and are frozen by `stream-layout-v1`.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum StreamItemTag {
    /// Every recorded TransformPlan. Always present, even when empty.
    TransformPlans,
    /// Shared codec dictionaries. Present with `cross-file-compression-v1`.
    Dictionaries,
    /// Bounded-lookback ChunkGroup declarations.
    ChunkGroups,
    /// Reconstruction side data and creation-time fallback audits.
    ReconstructionData,
    /// Whole-object reconstruction regions and their audits.
    ReconstructionRegions,
    /// One physical Chunk frame. Its `EBCH` header owns its stored length.
    ChunkFrame,
    /// One canonical Entry or ContentObject record.
    ManifestRecord,
    /// The FidelityReport.
    Fidelity,
    /// The authoritative descriptor, including the final identity roots.
    Descriptor,
}

impl StreamItemTag {
    /// The frozen wire discriminant for this tag.
    #[must_use]
    pub const fn wire_id(self) -> u16 {
        match self {
            Self::TransformPlans => 1,
            Self::Dictionaries => 2,
            Self::ChunkGroups => 3,
            Self::ReconstructionData => 4,
            Self::ReconstructionRegions => 5,
            Self::ChunkFrame => 6,
            Self::ManifestRecord => 7,
            Self::Fidelity => 8,
            Self::Descriptor => 9,
        }
    }

    /// Resolves a wire discriminant, refusing tags this version does not define.
    pub fn from_wire(value: u16) -> Result<Self> {
        Ok(match value {
            1 => Self::TransformPlans,
            2 => Self::Dictionaries,
            3 => Self::ChunkGroups,
            4 => Self::ReconstructionData,
            5 => Self::ReconstructionRegions,
            6 => Self::ChunkFrame,
            7 => Self::ManifestRecord,
            8 => Self::Fidelity,
            9 => Self::Descriptor,
            other => {
                return Err(Diagnostic::new(
                    OutcomeClass::Unsupported,
                    ReasonCode::UnsupportedRequiredFeature,
                    format!("unknown STREAM item tag {other}"),
                ));
            }
        })
    }

    /// Monotone canonical ordering stage. `CHUNK_FRAME` and `MANIFEST_RECORD`
    /// share a stage because they interleave by design.
    const fn stage(self) -> u8 {
        match self {
            Self::TransformPlans => 1,
            Self::Dictionaries => 2,
            Self::ChunkGroups => 3,
            Self::ReconstructionData => 4,
            Self::ReconstructionRegions => 5,
            Self::ChunkFrame | Self::ManifestRecord => 6,
            Self::Fidelity => 7,
            Self::Descriptor => 8,
        }
    }

    /// Whether the item appears at most once in a canonical body.
    const fn is_singleton(self) -> bool {
        !matches!(self, Self::ChunkFrame | Self::ManifestRecord)
    }
}

/// Producer policy for the declared stream deduplication window.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StreamWindow {
    /// Refuse to emit an organization needing more than this many historical
    /// Chunks. `Ceiling(0)` forbids cross-object historical dependencies.
    Ceiling(u64),
    /// Accept whatever the selected physical organization requires and declare
    /// exactly that minimum.
    Auto,
}

impl Default for StreamWindow {
    fn default() -> Self {
        Self::Ceiling(0)
    }
}

/// Writer options that affect only reconstructible physical organization.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StreamWriteOptions {
    pub window: StreamWindow,
    /// Whether the producer declares its resource budget before the payload.
    ///
    /// A producer that has already planned the whole archive knows its totals
    /// and should declare them. A producer that cannot measure its own output
    /// before emitting it sets this to `false`; the reader then bounds the pass
    /// with its own policy and learns the final actual totals from the footer.
    pub budget_declared: bool,
}

impl Default for StreamWriteOptions {
    fn default() -> Self {
        Self {
            window: StreamWindow::default(),
            budget_declared: true,
        }
    }
}

/// What a completed sequential write established.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StreamWriteSummary {
    pub archive: Archive,
    pub identities: IdentitySet,
    pub total_len: u64,
    pub body_len: u64,
    pub chunk_frames: u64,
    pub manifest_records: u64,
    pub dedup_window: u64,
    pub budget_declared: bool,
}

/// How much decoded plaintext a sequential pass keeps.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StreamContentPolicy {
    /// Verify every Chunk as it arrives and retain only the declared window.
    /// Sufficient for verification, listing, and inspection.
    Verify,
    /// Stage every Chunk for later materialization. Extraction uses this.
    Stage,
    /// Stage, then place plaintext back into the returned model. Only analysis
    /// that must re-derive physical alternatives, such as `explain`, needs it.
    Retain,
}

/// Access capability a STREAM source actually offers, stated rather than implied.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StreamAccessProfile {
    /// STREAM cannot resolve one Entry without scanning. Always `false`.
    pub random_entry_lookup: bool,
    /// The cursor complexity a caller actually gets.
    pub entry_cursor: EntryCursorComplexity,
    /// Listing requires a complete sequential pass.
    pub listing_requires_full_scan: bool,
    /// The source is consumed by the pass and cannot be replayed.
    pub source_replayable: bool,
}

impl StreamAccessProfile {
    const SEQUENTIAL: Self = Self {
        random_entry_lookup: false,
        entry_cursor: EntryCursorComplexity::Sequential,
        listing_requires_full_scan: true,
        source_replayable: false,
    };
}

/// Sequential-specific facts an INDEXED report has no place for.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StreamReport {
    pub dedup_window: u64,
    pub budget_declared: bool,
    pub actual_entry_count: u64,
    pub actual_total_logical_bytes: u64,
    pub actual_chunk_count: u64,
    pub chunk_frames: u64,
    pub manifest_records: u64,
    pub body_len: u64,
    pub total_len: u64,
    /// Largest number of Chunks the pass had to retain at one time.
    pub peak_retained_chunks: u64,
    pub peak_resident_staging_bytes: u64,
    pub spilled_staging_bytes: u64,
    pub access: StreamAccessProfile,
    pub plaintext_retained: bool,
}

/// Plaintext staged during a sequential pass, addressed by Chunk identity.
#[derive(Debug)]
pub(crate) struct StagedChunks {
    staging: ChunkStaging,
}

impl StagedChunks {
    pub(crate) fn read(&mut self, chunk_id: &Digest) -> Result<Vec<u8>> {
        self.staging.read(chunk_id)
    }
}

/// A fully scanned and verified STREAM archive.
#[derive(Debug)]
pub struct SequentialArchive {
    /// The verified model. Chunk plaintext is present only under
    /// [`StreamContentPolicy::Retain`]; every other pass releases it after
    /// verifying its digest.
    pub opened: OpenedArchive,
    pub stream: StreamReport,
    pub(crate) staged: Option<StagedChunks>,
}

/// Caller-owned limits for one sequential pass.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SequentialLimits {
    pub budget: ResourceBudget,
    pub decode: DecodeRequirements,
    pub staging: StagingLimits,
    /// Largest single item payload the reader will buffer.
    pub max_item_bytes: u64,
    /// Largest total container the reader will consume.
    pub max_container_bytes: u64,
    pub content: StreamContentPolicy,
}

/// Deliberately generous staging limits for the experimental bootstrap CLI.
#[must_use]
pub const fn bootstrap_staging_limits() -> StagingLimits {
    StagingLimits {
        memory_bytes: 256 * 1024 * 1024,
        total_bytes: 64 * 1024 * 1024 * 1024,
    }
}

/// Deliberately generous sequential limits for the experimental bootstrap CLI.
#[must_use]
pub const fn bootstrap_sequential_limits() -> SequentialLimits {
    SequentialLimits {
        budget: crate::archive::bootstrap_resource_policy(),
        decode: crate::archive::bootstrap_decode_policy(),
        staging: bootstrap_staging_limits(),
        max_item_bytes: 4 * 1024 * 1024 * 1024,
        max_container_bytes: 256 * 1024 * 1024 * 1024,
        content: StreamContentPolicy::Verify,
    }
}

// ---------------------------------------------------------------------------
// Emission planning
// ---------------------------------------------------------------------------

/// The deterministic STREAM physical organization for one validated model.
#[derive(Clone, Debug)]
struct EmissionPlan {
    /// Chunk frames in emission order.
    frames: Vec<Digest>,
    /// ContentObject records to emit once this many frames have been written.
    records_at: BTreeMap<u64, Vec<Digest>>,
    /// Minimum historical window the organization requires.
    window: u64,
}

fn plan_emission(archive: &Archive) -> Result<EmissionPlan> {
    let mut group_runs = BTreeMap::<Digest, Vec<Digest>>::new();
    for chunk_id in &archive.content_store.physical_order {
        let chunk = archive
            .content_store
            .chunks
            .get(chunk_id)
            .ok_or_else(|| unknown_chunk(chunk_id))?;
        if let Some(group_id) = chunk.group_ref {
            group_runs.entry(group_id).or_default().push(*chunk_id);
        }
    }

    let mut frames = Vec::with_capacity(archive.content_store.chunks.len());
    let mut frame_index = BTreeMap::<Digest, u64>::new();
    let mut emitted_groups = BTreeSet::<Digest>::new();
    let mut positions = Vec::<(u64, Digest)>::new();

    let push = |frames: &mut Vec<Digest>,
                frame_index: &mut BTreeMap<Digest, u64>,
                chunk_id: Digest|
     -> Result<()> {
        if frame_index.contains_key(&chunk_id) {
            return Ok(());
        }
        let ordinal =
            u64::try_from(frames.len()).map_err(|_| resource("frame count exceeds u64"))?;
        frame_index.insert(chunk_id, ordinal);
        frames.push(chunk_id);
        Ok(())
    };

    for (logical_digest, object) in &archive.content_store.objects {
        for chunk_ref in &object.chunks {
            if frame_index.contains_key(&chunk_ref.chunk_id) {
                continue;
            }
            let chunk = archive
                .content_store
                .chunks
                .get(&chunk_ref.chunk_id)
                .ok_or_else(|| unknown_chunk(&chunk_ref.chunk_id))?;
            match chunk.group_ref {
                Some(group_id) => {
                    if emitted_groups.insert(group_id) {
                        let run = group_runs.get(&group_id).ok_or_else(|| {
                            Diagnostic::new(
                                OutcomeClass::Nonconforming,
                                ReasonCode::InvalidGroupReference,
                                group_id.to_string(),
                            )
                        })?;
                        for member in run {
                            push(&mut frames, &mut frame_index, *member)?;
                        }
                    }
                    if !frame_index.contains_key(&chunk_ref.chunk_id) {
                        return Err(Diagnostic::new(
                            OutcomeClass::Nonconforming,
                            ReasonCode::InvalidGroupReference,
                            format!(
                                "Chunk {} claims group {} but is absent from its run",
                                chunk_ref.chunk_id, group_id
                            ),
                        ));
                    }
                }
                None => push(&mut frames, &mut frame_index, chunk_ref.chunk_id)?,
            }
        }
        let emitted =
            u64::try_from(frames.len()).map_err(|_| resource("frame count exceeds u64"))?;
        positions.push((emitted, *logical_digest));
    }

    // A Chunk that no ContentObject references still needs a frame. The model
    // does not create these, but the writer must never silently drop one.
    for chunk_id in &archive.content_store.physical_order {
        if !frame_index.contains_key(chunk_id) {
            let chunk = archive
                .content_store
                .chunks
                .get(chunk_id)
                .ok_or_else(|| unknown_chunk(chunk_id))?;
            match chunk.group_ref {
                Some(group_id) => {
                    if emitted_groups.insert(group_id) {
                        for member in group_runs.get(&group_id).into_iter().flatten() {
                            push(&mut frames, &mut frame_index, *member)?;
                        }
                    }
                    // Emitting a grouped Chunk outside its run would break the
                    // contiguity a bounded-lookback decoder depends on.
                    if !frame_index.contains_key(chunk_id) {
                        return Err(Diagnostic::new(
                            OutcomeClass::Nonconforming,
                            ReasonCode::InvalidGroupReference,
                            format!(
                                "Chunk {chunk_id} claims group {group_id} but is absent from its run"
                            ),
                        ));
                    }
                }
                None => push(&mut frames, &mut frame_index, *chunk_id)?,
            }
        }
    }

    let mut window = 0_u64;
    let mut records_at = BTreeMap::<u64, Vec<Digest>>::new();
    let mut previous_batch_end = 0_u64;
    let mut cursor = 0_usize;
    while cursor < positions.len() {
        let batch_end = positions[cursor].0;
        let run_start = previous_batch_end;
        while cursor < positions.len() && positions[cursor].0 == batch_end {
            let object = &archive.content_store.objects[&positions[cursor].1];
            for chunk_ref in &object.chunks {
                let ordinal = frame_index[&chunk_ref.chunk_id];
                if ordinal < run_start {
                    window = window.max(run_start - ordinal);
                }
            }
            records_at
                .entry(batch_end)
                .or_default()
                .push(positions[cursor].1);
            cursor += 1;
        }
        previous_batch_end = batch_end;
    }

    Ok(EmissionPlan {
        frames,
        records_at,
        window,
    })
}

// ---------------------------------------------------------------------------
// Writer
// ---------------------------------------------------------------------------

/// A sink wrapper that hashes what it writes. It never seeks.
struct StreamSink<W: Write> {
    sink: W,
    container: Sha256,
    body: Sha256,
    written: u64,
    body_len: u64,
}

impl<W: Write> StreamSink<W> {
    fn new(sink: W) -> Self {
        Self {
            sink,
            container: Sha256::new(),
            body: Sha256::new(),
            written: 0,
            body_len: 0,
        }
    }

    fn write_framing(&mut self, bytes: &[u8]) -> Result<()> {
        self.sink
            .write_all(bytes)
            .map_err(|error| io(format!("write STREAM output: {error}")))?;
        self.container.update(bytes);
        self.written = self
            .written
            .checked_add(u64::try_from(bytes.len()).map_err(|_| resource("write exceeds u64"))?)
            .ok_or_else(|| resource("container length exceeds u64"))?;
        Ok(())
    }

    fn write_body(&mut self, bytes: &[u8]) -> Result<()> {
        self.write_framing(bytes)?;
        self.body.update(bytes);
        self.body_len = self
            .body_len
            .checked_add(u64::try_from(bytes.len()).map_err(|_| resource("write exceeds u64"))?)
            .ok_or_else(|| resource("body length exceeds u64"))?;
        Ok(())
    }

    fn body_offset(&self) -> u64 {
        self.written
    }

    fn emit_record_item(&mut self, tag: StreamItemTag, payload: &[u8]) -> Result<u64> {
        let offset = self.body_offset();
        self.write_body(&item_header(tag))?;
        let length = u64::try_from(payload.len()).map_err(|_| resource("item exceeds u64"))?;
        self.write_body(&length.to_be_bytes())?;
        self.write_body(sha256_exact(payload).as_bytes())?;
        self.write_body(payload)?;
        Ok(offset)
    }

    fn emit_chunk_frame(&mut self, frame: &[u8]) -> Result<u64> {
        let offset = self.body_offset();
        self.write_body(&item_header(StreamItemTag::ChunkFrame))?;
        self.write_body(frame)?;
        Ok(offset)
    }
}

/// Emits every ContentObject record whose physical data is now complete.
fn emit_records_at<W: Write>(
    writer: &mut StreamSink<W>,
    archive: &Archive,
    plan: &EmissionPlan,
    emitted: u64,
    manifest_records: &mut u64,
) -> Result<()> {
    for logical_digest in plan.records_at.get(&emitted).into_iter().flatten() {
        let object = archive
            .content_store
            .objects
            .get(logical_digest)
            .ok_or_else(|| {
                Diagnostic::new(
                    OutcomeClass::Nonconforming,
                    ReasonCode::UnknownContentObject,
                    logical_digest.to_string(),
                )
            })?;
        writer.emit_record_item(
            StreamItemTag::ManifestRecord,
            &encode_content_object_record(object)?,
        )?;
        *manifest_records = manifest_records
            .checked_add(1)
            .ok_or_else(|| resource("manifest record count exceeds u64"))?;
    }
    Ok(())
}

fn item_header(tag: StreamItemTag) -> [u8; 16] {
    let mut header = [0_u8; 16];
    header[0..4].copy_from_slice(&STREAM_ITEM_MAGIC);
    header[4..6].copy_from_slice(&tag.wire_id().to_be_bytes());
    header[6..8].copy_from_slice(&STREAM_ITEM_VERSION.to_be_bytes());
    header
}

/// Serializes a validated EAM as canonical unencrypted Complete STREAM ECF.
///
/// The sink needs only [`std::io::Write`]. Nothing already written is revisited,
/// so the sink may be a pipe, a socket, or standard output.
pub fn encode_stream<W: Write>(
    input: &Archive,
    options: StreamWriteOptions,
    sink: W,
) -> Result<StreamWriteSummary> {
    let mut prepared = input.clone();
    prepared.descriptor.layout = Layout::Stream;
    prepared.descriptor.features.incompat |= FEATURE_STREAM_LAYOUT_V1;
    prepared.descriptor.stream_dedup_window = 0;
    prepared.validate()?;
    validate_plans(&prepared.transform_plans)?;
    validate_feature_model(&prepared)?;
    for plan in &prepared.transform_plans {
        let required = crate::codec::required_features(plan)?;
        if required & !prepared.descriptor.features.incompat != 0 {
            return Err(Diagnostic::new(
                OutcomeClass::Unsupported,
                ReasonCode::UnsupportedRequiredFeature,
                format!(
                    "TransformPlan {} requires undeclared incompat feature bits {:016x}",
                    plan.plan_id,
                    required & !prepared.descriptor.features.incompat
                ),
            ));
        }
    }

    let (mut archive, roots) = apply_native_identities(&prepared)?;
    archive.descriptor.layout = Layout::Stream;
    archive.descriptor.features.incompat |= FEATURE_STREAM_LAYOUT_V1;

    let extended = has_cross_file_feature(archive.descriptor.features);
    let reconstructive = has_reconstructive_feature(archive.descriptor.features);
    let whole_object = has_whole_object_feature(archive.descriptor.features);

    let plan = plan_emission(&archive)?;
    let declared_window = match options.window {
        StreamWindow::Auto => plan.window,
        StreamWindow::Ceiling(ceiling) => {
            if plan.window > ceiling {
                return Err(Diagnostic::new(
                    OutcomeClass::PolicyRefused,
                    ReasonCode::StreamWindowExceeded,
                    format!(
                        "this archive's sequential organization requires a stream dedup window of \
                         {} but the requested ceiling is {ceiling}; re-run with an explicit window \
                         of at least {} or with an automatic window",
                        plan.window, plan.window
                    ),
                ));
            }
            plan.window
        }
    };

    let plans_payload = if whole_object {
        encode_transform_plans_v3(&archive.transform_plans)?
    } else if reconstructive {
        encode_transform_plans_v2(&archive.transform_plans)?
    } else {
        encode_transform_plans(
            &archive.transform_plans,
            has_codec_transform_feature(archive.descriptor.features),
        )?
    };
    let dictionaries_payload = encode_dictionaries(&archive.content_store.dictionaries)?;
    let groups_payload = encode_chunk_groups(&archive.content_store.chunk_groups)?;
    let reconstruction_payload = encode_reconstruction_section(
        &archive.content_store.reconstruction_data,
        &archive.content_store.reconstruction_fallbacks,
    )?;
    let regions_payload = encode_reconstruction_regions(
        &archive.content_store.reconstruction_regions,
        &archive.content_store.reconstruction_audits,
    )?;

    // A producer that declares a pre-payload budget must know its stored
    // lengths first, so it encodes the frames once and reuses those bytes. A
    // producer that declares no budget emits frames as it encodes them.
    let buffered_frames = if options.budget_declared {
        let mut encoder = ChunkFrameEncoder::new(&archive, extended, whole_object)?;
        let mut buffered = Vec::with_capacity(plan.frames.len());
        for chunk_id in &plan.frames {
            buffered.push(encoder.encode_next(chunk_id)?);
        }
        Some(buffered)
    } else {
        None
    };

    let header_len = chunk_frame_header_len(extended);
    let stored_lengths = match buffered_frames.as_ref() {
        Some(frames) => {
            let mut lengths = BTreeMap::new();
            for (chunk_id, frame) in plan.frames.iter().zip(frames) {
                let stored_len = u64::try_from(frame.len())
                    .map_err(|_| resource("frame length exceeds u64"))?
                    .checked_sub(header_len)
                    .ok_or_else(|| structure("Chunk frame is shorter than its header"))?;
                lengths.insert(
                    *chunk_id,
                    ChunkLocation {
                        offset: 0,
                        stored_len,
                    },
                );
            }
            Some(lengths)
        }
        None => None,
    };
    normalize_descriptor(
        &mut archive,
        stored_lengths.as_ref(),
        PhysicalDeclaration {
            layout: Layout::Stream,
            budget_declared: options.budget_declared,
            stream_dedup_window: declared_window,
        },
    )?;

    let descriptor_payload = encode_descriptor(&DescriptorBody {
        namespace: FORMAT_NAMESPACE.to_owned(),
        identity_profile: 1,
        digest_algorithm: 1,
        planner_id: archive.descriptor.planner_id.clone(),
        chunker_id: archive.descriptor.chunker_id.clone(),
        lai: roots.lai.0,
        pcr: roots.pcr.0,
        aux: roots.aux.0,
    })?;
    let fidelity_payload = encode_fidelity(&archive.fidelity)?;

    let mut writer = StreamSink::new(sink);
    writer.write_framing(&encode_preamble(&archive.descriptor, 0)?)?;

    writer.emit_record_item(StreamItemTag::TransformPlans, &plans_payload)?;
    if extended {
        writer.emit_record_item(StreamItemTag::Dictionaries, &dictionaries_payload)?;
        writer.emit_record_item(StreamItemTag::ChunkGroups, &groups_payload)?;
    }
    if reconstructive {
        writer.emit_record_item(StreamItemTag::ReconstructionData, &reconstruction_payload)?;
    }
    if whole_object {
        writer.emit_record_item(StreamItemTag::ReconstructionRegions, &regions_payload)?;
    }

    let mut fresh_encoder = match buffered_frames {
        Some(_) => None,
        None => Some(ChunkFrameEncoder::new(&archive, extended, whole_object)?),
    };
    let mut locators = BTreeMap::<Digest, ChunkLocation>::new();
    let mut manifest_records = 0_u64;
    let mut emitted = 0_u64;

    emit_records_at(&mut writer, &archive, &plan, emitted, &mut manifest_records)?;
    for (ordinal, chunk_id) in plan.frames.iter().enumerate() {
        let frame = match buffered_frames.as_ref() {
            Some(frames) => frames[ordinal].clone(),
            None => fresh_encoder
                .as_mut()
                .ok_or_else(|| structure("STREAM writer lost its frame encoder"))?
                .encode_next(chunk_id)?,
        };
        let stored_len = u64::try_from(frame.len())
            .map_err(|_| resource("frame length exceeds u64"))?
            .checked_sub(header_len)
            .ok_or_else(|| structure("Chunk frame is shorter than its header"))?;
        let item_offset = writer.emit_chunk_frame(&frame)?;
        locators.insert(
            *chunk_id,
            ChunkLocation {
                offset: item_offset
                    .checked_add(STREAM_ITEM_HEADER_LEN)
                    .ok_or_else(|| resource("frame offset overflow"))?,
                stored_len,
            },
        );
        emitted = emitted
            .checked_add(1)
            .ok_or_else(|| resource("frame count exceeds u64"))?;
        emit_records_at(&mut writer, &archive, &plan, emitted, &mut manifest_records)?;
    }
    drop(fresh_encoder);

    for entry in archive.entry_set.entries() {
        writer.emit_record_item(StreamItemTag::ManifestRecord, &encode_entry_record(entry)?)?;
        manifest_records = manifest_records
            .checked_add(1)
            .ok_or_else(|| resource("manifest record count exceeds u64"))?;
    }

    writer.emit_record_item(StreamItemTag::Fidelity, &fidelity_payload)?;
    let descriptor_offset =
        writer.emit_record_item(StreamItemTag::Descriptor, &descriptor_payload)?;
    let descriptor_len = STREAM_ITEM_HEADER_LEN
        .checked_add(RECORD_ITEM_PREFIX_LEN)
        .and_then(|value| value.checked_add(u64::try_from(descriptor_payload.len()).ok()?))
        .ok_or_else(|| resource("descriptor item length overflow"))?;

    let body_len = writer.body_len;
    let body_digest = Digest::from_bytes(writer.body.clone().finalize().into());
    let entry_count =
        u64::try_from(archive.entry_set.len()).map_err(|_| resource("entry count exceeds u64"))?;
    let total_logical = archive.total_logical_size()?;
    let chunk_count = u64::try_from(archive.content_store.chunks.len())
        .map_err(|_| resource("Chunk count exceeds u64"))?;
    let total_len = writer
        .written
        .checked_add(STREAM_FOOTER_LEN)
        .ok_or_else(|| resource("container length overflow"))?;
    let preamble_digest = sha256_exact(&encode_preamble(&archive.descriptor, 0)?);
    let footer = encode_stream_footer(StreamFooter {
        total_len,
        descriptor_offset,
        descriptor_len,
        body_len,
        chunk_count,
        entry_count,
        total_logical,
        preamble_digest,
        body_digest,
    });
    writer.write_framing(&footer)?;
    writer
        .sink
        .flush()
        .map_err(|error| io(format!("flush STREAM output: {error}")))?;

    let pci = PhysicalContainerIdentity(Digest::from_bytes(writer.container.finalize().into()));
    archive.descriptor.pci = Some(pci.0);
    archive.content_store.physical_order = plan.frames.clone().into_boxed_slice();
    archive.index = Index {
        present: false,
        valid: false,
        chunks: locators,
        status: "not applicable; STREAM layout carries no Index".to_owned(),
    };
    Ok(StreamWriteSummary {
        archive,
        identities: roots.with_pci(pci),
        total_len,
        body_len,
        chunk_frames: emitted,
        manifest_records,
        dedup_window: declared_window,
        budget_declared: options.budget_declared,
    })
}

// ---------------------------------------------------------------------------
// Footer
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct StreamFooter {
    total_len: u64,
    descriptor_offset: u64,
    descriptor_len: u64,
    body_len: u64,
    chunk_count: u64,
    entry_count: u64,
    total_logical: u64,
    preamble_digest: Digest,
    body_digest: Digest,
}

fn encode_stream_footer(footer: StreamFooter) -> Vec<u8> {
    let mut bytes = vec![0_u8; usize::try_from(STREAM_FOOTER_LEN).unwrap_or(128)];
    bytes[0..8].copy_from_slice(&FOOTER_MAGIC);
    bytes[8..16].copy_from_slice(&footer.total_len.to_be_bytes());
    bytes[16..24].copy_from_slice(&footer.descriptor_offset.to_be_bytes());
    bytes[24..32].copy_from_slice(&footer.descriptor_len.to_be_bytes());
    bytes[32..40].copy_from_slice(&footer.body_len.to_be_bytes());
    bytes[40..48].copy_from_slice(&footer.chunk_count.to_be_bytes());
    bytes[48..56].copy_from_slice(&footer.entry_count.to_be_bytes());
    bytes[56..64].copy_from_slice(&footer.total_logical.to_be_bytes());
    bytes[64..96].copy_from_slice(footer.preamble_digest.as_bytes());
    bytes[96..128].copy_from_slice(footer.body_digest.as_bytes());
    bytes
}

fn decode_stream_footer(bytes: &[u8]) -> Result<StreamFooter> {
    Ok(StreamFooter {
        total_len: be64(&bytes[8..16])?,
        descriptor_offset: be64(&bytes[16..24])?,
        descriptor_len: be64(&bytes[24..32])?,
        body_len: be64(&bytes[32..40])?,
        chunk_count: be64(&bytes[40..48])?,
        entry_count: be64(&bytes[48..56])?,
        total_logical: be64(&bytes[56..64])?,
        preamble_digest: digest32(&bytes[64..96])?,
        body_digest: digest32(&bytes[96..128])?,
    })
}

// ---------------------------------------------------------------------------
// Reader
// ---------------------------------------------------------------------------

enum Fill {
    Filled,
    Short(usize),
}

struct StreamSource<R: Read> {
    source: R,
    container: Sha256,
    body: Sha256,
    consumed: u64,
    body_len: u64,
    limit: u64,
}

impl<R: Read> StreamSource<R> {
    fn new(source: R, limit: u64) -> Self {
        Self {
            source,
            container: Sha256::new(),
            body: Sha256::new(),
            consumed: 0,
            body_len: 0,
            limit,
        }
    }

    /// Reads exactly `buf.len()` bytes unless the source ends first.
    ///
    /// A source that returns one byte at a time is handled the same as one that
    /// returns the whole buffer, which is what a pipe requires.
    fn fill(&mut self, buf: &mut [u8]) -> Result<Fill> {
        let mut filled = 0;
        while filled < buf.len() {
            match self.source.read(&mut buf[filled..]) {
                Ok(0) => break,
                Ok(count) => filled += count,
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
                Err(error) => return Err(io(format!("read STREAM input: {error}"))),
            }
        }
        self.container.update(&buf[..filled]);
        self.consumed = self
            .consumed
            .checked_add(u64::try_from(filled).map_err(|_| resource("read exceeds u64"))?)
            .ok_or_else(|| resource("consumed length exceeds u64"))?;
        if self.consumed > self.limit {
            return Err(Diagnostic::new(
                OutcomeClass::PolicyRefused,
                ReasonCode::ResourceLimit,
                "STREAM input exceeds the caller's container limit",
            ));
        }
        Ok(if filled == buf.len() {
            Fill::Filled
        } else {
            Fill::Short(filled)
        })
    }

    fn fill_exact(&mut self, buf: &mut [u8], context: &str) -> Result<()> {
        match self.fill(buf)? {
            Fill::Filled => Ok(()),
            Fill::Short(_) => Err(truncated(format!("STREAM input ends inside {context}"))),
        }
    }

    fn feed_body(&mut self, bytes: &[u8]) -> Result<()> {
        self.body.update(bytes);
        self.body_len = self
            .body_len
            .checked_add(u64::try_from(bytes.len()).map_err(|_| resource("body exceeds u64"))?)
            .ok_or_else(|| resource("body length exceeds u64"))?;
        Ok(())
    }

    /// Reads `length` body bytes without trusting `length` as an allocation.
    fn read_body_bytes(&mut self, length: u64, context: &str) -> Result<Vec<u8>> {
        const STEP: u64 = 1 << 20;
        let mut collected = Vec::new();
        let mut remaining = length;
        while remaining > 0 {
            let step = remaining.min(STEP);
            let mut buffer =
                vec![0_u8; usize::try_from(step).map_err(|_| resource("read step exceeds usize"))?];
            self.fill_exact(&mut buffer, context)?;
            self.feed_body(&buffer)?;
            collected.extend_from_slice(&buffer);
            remaining -= step;
        }
        Ok(collected)
    }

    fn at_end(&mut self) -> Result<bool> {
        let mut probe = [0_u8; 1];
        Ok(matches!(self.fill(&mut probe)?, Fill::Short(0)))
    }
}

/// Opens and fully verifies a STREAM archive from an unseekable source.
pub fn open_stream<R: Read>(source: R) -> Result<SequentialArchive> {
    open_stream_with_limits(source, bootstrap_sequential_limits())
}

/// Opens and fully verifies a STREAM archive under a caller-owned budget.
pub fn open_stream_with_policy<R: Read>(
    source: R,
    policy: ResourceBudget,
) -> Result<SequentialArchive> {
    open_stream_with_limits(
        source,
        SequentialLimits {
            budget: policy,
            ..bootstrap_sequential_limits()
        },
    )
}

/// Verifies a STREAM archive without hiding which guarantees were checked.
pub fn verify_stream<R: Read>(source: R) -> Result<VerificationReport> {
    Ok(open_stream(source)?.opened.report)
}

/// Verifies a STREAM archive under explicit caller-owned limits.
pub fn verify_stream_with_limits<R: Read>(
    source: R,
    limits: SequentialLimits,
) -> Result<VerificationReport> {
    Ok(open_stream_with_limits(source, limits)?.opened.report)
}

/// Runs one complete sequential pass under explicit caller-owned limits.
///
/// The pass validates framing, enforces the caller's policy while it reads,
/// decodes and verifies each physical item as it arrives, tracks bounded
/// dependencies, accumulates semantic manifest data, recomputes the identity
/// roots, and validates the descriptor and footer at EOF. It never seeks and
/// never holds the whole archive in memory.
pub fn open_stream_with_limits<R: Read>(
    source: R,
    limits: SequentialLimits,
) -> Result<SequentialArchive> {
    Scanner::new(source, limits)?.run()
}

/// Incremental state for one sequential pass.
struct Scanner<R: Read> {
    source: StreamSource<R>,
    limits: SequentialLimits,
    preamble: Preamble,
    preamble_bytes: Vec<u8>,
    effective: ResourceBudget,
    extended: bool,
    reconstructive: bool,
    whole_object: bool,

    plans: Vec<TransformPlan>,
    plan_map: BTreeMap<u64, TransformPlan>,
    dictionaries: BTreeMap<Digest, Dictionary>,
    chunk_groups: BTreeMap<Digest, ChunkGroup>,
    reconstruction_data: BTreeMap<Digest, ReconstructionData>,
    reconstruction_fallbacks: BTreeMap<Digest, ReconstructionFallbackReason>,
    reconstruction_regions: BTreeMap<Digest, ReconstructionRegion>,
    reconstruction_audits: BTreeMap<ReconstructionAuditTarget, ReconstructionAudit>,
    regions_by_object: BTreeMap<Digest, Vec<Digest>>,
    fidelity: Option<crate::eam::FidelityReport>,
    descriptor_body: Option<DescriptorBody>,
    descriptor_location: Option<(u64, u64)>,

    entries: Vec<Entry>,
    objects: BTreeMap<Digest, ContentObject>,
    chunks: BTreeMap<Digest, Chunk>,
    locators: BTreeMap<Digest, ChunkLocation>,
    physical_order: Vec<Digest>,
    frame_index: BTreeMap<Digest, u64>,
    region_owned_declared: BTreeSet<Digest>,
    group_prefix: BTreeMap<Digest, VecDeque<Vec<u8>>>,
    staging: ChunkStaging,

    stage: u8,
    seen_singletons: BTreeSet<u16>,
    entry_records_started: bool,
    run_start: u64,
    previous_batch_end: u64,
    current_batch_position: Option<u64>,
    observed_window: u64,
    released_upto: usize,
    peak_retained_chunks: u64,
    chunk_frames: u64,
    manifest_records: u64,
    manifest_payload_bytes: u64,
    supporting_metadata_bytes: u64,
    previous_object: Option<Digest>,
    total_logical: u64,
}

impl<R: Read> Scanner<R> {
    fn new(source: R, limits: SequentialLimits) -> Result<Self> {
        let mut source = StreamSource::new(source, limits.max_container_bytes);
        let mut preamble_bytes = vec![0_u8; usize::try_from(PREAMBLE_LEN).unwrap_or(256)];
        match source.fill(&mut preamble_bytes)? {
            Fill::Filled => {}
            Fill::Short(filled) => {
                if filled >= 8 && preamble_bytes[..8] != super::MAGIC {
                    return Err(Diagnostic::new(
                        OutcomeClass::Nonconforming,
                        ReasonCode::BadMagic,
                        "Entrybound magic mismatch",
                    ));
                }
                return Err(truncated("STREAM input ends inside its fixed preamble"));
            }
        }
        let preamble = decode_preamble(&preamble_bytes)?;
        if preamble.layout != Layout::Stream {
            return Err(Diagnostic::new(
                OutcomeClass::Unsupported,
                ReasonCode::UnsupportedRequiredFeature,
                "this archive uses INDEXED layout; open it with the random-access reader",
            ));
        }
        enforce_decode_policy(preamble.decode, limits.decode)?;
        let effective = if preamble.budget_declared {
            enforce_caller_policy(preamble.budget, limits.budget)?;
            preamble.budget
        } else {
            if preamble.budget != ResourceBudget::default() {
                return Err(noncanonical(
                    "an undeclared resource budget must be encoded as zero",
                ));
            }
            limits.budget
        };
        Ok(Self {
            extended: has_cross_file_feature(preamble.features),
            reconstructive: has_reconstructive_feature(preamble.features),
            whole_object: has_whole_object_feature(preamble.features),
            source,
            limits,
            preamble,
            preamble_bytes,
            effective,
            plans: Vec::new(),
            plan_map: BTreeMap::new(),
            dictionaries: BTreeMap::new(),
            chunk_groups: BTreeMap::new(),
            reconstruction_data: BTreeMap::new(),
            reconstruction_fallbacks: BTreeMap::new(),
            reconstruction_regions: BTreeMap::new(),
            reconstruction_audits: BTreeMap::new(),
            regions_by_object: BTreeMap::new(),
            fidelity: None,
            descriptor_body: None,
            descriptor_location: None,
            entries: Vec::new(),
            objects: BTreeMap::new(),
            chunks: BTreeMap::new(),
            locators: BTreeMap::new(),
            physical_order: Vec::new(),
            frame_index: BTreeMap::new(),
            region_owned_declared: BTreeSet::new(),
            group_prefix: BTreeMap::new(),
            staging: ChunkStaging::new(limits.staging),
            stage: 0,
            seen_singletons: BTreeSet::new(),
            entry_records_started: false,
            run_start: 0,
            previous_batch_end: 0,
            current_batch_position: None,
            observed_window: 0,
            released_upto: 0,
            peak_retained_chunks: 0,
            chunk_frames: 0,
            manifest_records: 0,
            manifest_payload_bytes: 0,
            supporting_metadata_bytes: 0,
            previous_object: None,
            total_logical: 0,
        })
    }

    fn run(mut self) -> Result<SequentialArchive> {
        let footer = loop {
            let mut header = [0_u8; 16];
            match self.source.fill(&mut header)? {
                Fill::Filled => {}
                Fill::Short(0) => {
                    return Err(truncated("STREAM input ends without its fixed footer"));
                }
                Fill::Short(_) => {
                    return Err(truncated("STREAM input ends inside an item header"));
                }
            }
            if header[..8] == FOOTER_MAGIC {
                let mut rest = vec![0_u8; usize::try_from(STREAM_FOOTER_LEN).unwrap_or(128) - 16];
                self.source.fill_exact(&mut rest, "the fixed footer")?;
                let mut bytes = header.to_vec();
                bytes.extend_from_slice(&rest);
                break decode_stream_footer(&bytes)?;
            }
            if header[..4] != STREAM_ITEM_MAGIC {
                return Err(Diagnostic::new(
                    OutcomeClass::Corrupt,
                    ReasonCode::StreamItemOrdering,
                    "STREAM item magic is neither a tagged item nor the fixed footer",
                ));
            }
            let tag = StreamItemTag::from_wire(u16::from_be_bytes([header[4], header[5]]))?;
            if u16::from_be_bytes([header[6], header[7]]) != STREAM_ITEM_VERSION
                || header[8..16] != [0; 8]
            {
                return Err(noncanonical(
                    "STREAM item version, flags, or reserved bytes are not canonical",
                ));
            }
            let item_offset = self
                .source
                .consumed
                .checked_sub(STREAM_ITEM_HEADER_LEN)
                .ok_or_else(|| structure("STREAM item offset underflow"))?;
            self.source.feed_body(&header)?;
            self.accept_ordering(tag)?;
            self.read_item(tag, item_offset)?;
        };
        self.finish(footer)
    }

    /// Enforces the canonical item order without guessing at intent.
    fn accept_ordering(&mut self, tag: StreamItemTag) -> Result<()> {
        let stage = tag.stage();
        if stage < self.stage {
            return Err(Diagnostic::new(
                OutcomeClass::Nonconforming,
                ReasonCode::StreamItemOrdering,
                format!("{tag:?} item appears after a later canonical stage"),
            ));
        }
        if tag.is_singleton() && !self.seen_singletons.insert(tag.wire_id()) {
            return Err(Diagnostic::new(
                OutcomeClass::Nonconforming,
                ReasonCode::DuplicateSemanticDeclaration,
                format!("duplicate {tag:?} item"),
            ));
        }
        let permitted = match tag {
            StreamItemTag::TransformPlans
            | StreamItemTag::ChunkFrame
            | StreamItemTag::ManifestRecord
            | StreamItemTag::Fidelity
            | StreamItemTag::Descriptor => true,
            StreamItemTag::Dictionaries | StreamItemTag::ChunkGroups => self.extended,
            StreamItemTag::ReconstructionData => self.reconstructive,
            StreamItemTag::ReconstructionRegions => self.whole_object,
        };
        if !permitted {
            return Err(Diagnostic::new(
                OutcomeClass::Nonconforming,
                ReasonCode::UnsupportedRequiredFeature,
                format!("{tag:?} item requires a feature this archive does not declare"),
            ));
        }
        if tag == StreamItemTag::ChunkFrame && self.entry_records_started {
            return Err(Diagnostic::new(
                OutcomeClass::Nonconforming,
                ReasonCode::StreamItemOrdering,
                "CHUNK_FRAME items cannot follow Entry manifest records",
            ));
        }
        self.stage = stage;
        Ok(())
    }

    fn read_item(&mut self, tag: StreamItemTag, item_offset: u64) -> Result<()> {
        if tag == StreamItemTag::ChunkFrame {
            return self.read_chunk_frame(item_offset);
        }
        let mut prefix = [0_u8; 40];
        self.source
            .fill_exact(&mut prefix, "a record item prefix")?;
        self.source.feed_body(&prefix)?;
        let length = be64(&prefix[0..8])?;
        if length > self.limits.max_item_bytes {
            return Err(Diagnostic::new(
                OutcomeClass::PolicyRefused,
                ReasonCode::ResourceLimit,
                format!("{tag:?} item exceeds the caller's item limit"),
            ));
        }
        let payload = self
            .source
            .read_body_bytes(length, "a record item payload")?;
        if sha256_exact(&payload).as_bytes() != &prefix[8..40] {
            return Err(Diagnostic::new(
                OutcomeClass::Corrupt,
                ReasonCode::SectionDigestMismatch,
                format!("{tag:?} item payload digest mismatch"),
            ));
        }
        let item_len = STREAM_ITEM_HEADER_LEN
            .checked_add(RECORD_ITEM_PREFIX_LEN)
            .and_then(|value| value.checked_add(length))
            .ok_or_else(|| resource("item length overflow"))?;
        match tag {
            StreamItemTag::TransformPlans => {
                let plans = if self.whole_object {
                    decode_transform_plans_v3(&payload)?
                } else if self.reconstructive {
                    decode_transform_plans_v2(&payload)?
                } else {
                    decode_transform_plans(
                        &payload,
                        has_codec_transform_feature(self.preamble.features),
                    )?
                };
                validate_plans(&plans)?;
                self.plan_map = plans
                    .iter()
                    .map(|plan| (plan.plan_id, plan.clone()))
                    .collect();
                self.plans = plans.into_vec();
            }
            StreamItemTag::Dictionaries => {
                self.dictionaries = decode_dictionaries(&payload)?;
                for dictionary in self.dictionaries.values() {
                    crate::codec::validate_dictionary(dictionary)?;
                }
            }
            StreamItemTag::ChunkGroups => self.chunk_groups = decode_chunk_groups(&payload)?,
            StreamItemTag::ReconstructionData => {
                let (data, fallbacks) = decode_reconstruction_section(&payload)?;
                self.reconstruction_data = data;
                self.reconstruction_fallbacks = fallbacks;
                self.add_supporting_metadata(length)?;
            }
            StreamItemTag::ReconstructionRegions => {
                let (regions, audits) = decode_reconstruction_regions(&payload)?;
                for region in regions.values() {
                    self.regions_by_object
                        .entry(region.content_object)
                        .or_default()
                        .push(region.region_id);
                }
                self.reconstruction_regions = regions;
                self.reconstruction_audits = audits;
                self.add_supporting_metadata(length)?;
            }
            StreamItemTag::ManifestRecord => {
                self.manifest_payload_bytes = self
                    .manifest_payload_bytes
                    .checked_add(length)
                    .ok_or_else(|| resource("manifest payload total exceeds u64"))?;
                self.manifest_records = self
                    .manifest_records
                    .checked_add(1)
                    .ok_or_else(|| resource("manifest record count exceeds u64"))?;
                match decode_manifest_record(&payload)? {
                    ManifestRecord::ContentObject(object) => self.accept_content_object(*object)?,
                    ManifestRecord::Entry(entry) => self.accept_entry(*entry)?,
                }
            }
            StreamItemTag::Fidelity => self.fidelity = Some(decode_fidelity(&payload)?),
            StreamItemTag::Descriptor => {
                self.descriptor_body = Some(decode_descriptor(&payload)?);
                self.descriptor_location = Some((item_offset, item_len));
            }
            StreamItemTag::ChunkFrame => unreachable!("handled above"),
        }
        Ok(())
    }

    fn add_supporting_metadata(&mut self, length: u64) -> Result<()> {
        self.supporting_metadata_bytes = self
            .supporting_metadata_bytes
            .checked_add(length)
            .ok_or_else(|| resource("supporting metadata total exceeds u64"))?;
        Ok(())
    }

    fn read_chunk_frame(&mut self, item_offset: u64) -> Result<()> {
        let header_len = usize::try_from(chunk_frame_header_len(self.extended))
            .unwrap_or(if self.extended { 96 } else { 64 });
        let mut header = vec![0_u8; header_len];
        self.source
            .fill_exact(&mut header, "a Chunk frame header")?;
        self.source.feed_body(&header)?;
        let parsed = parse_chunk_frame_header(&header, self.extended, self.whole_object)?;
        if self.frame_index.contains_key(&parsed.chunk_id) {
            return Err(noncanonical("Chunk frame IDs must be unique"));
        }
        enforce_chunk_bounds(&parsed, self.effective)?;
        if parsed.stored_len > self.limits.max_item_bytes {
            return Err(Diagnostic::new(
                OutcomeClass::PolicyRefused,
                ReasonCode::ResourceLimit,
                "Chunk frame exceeds the caller's item limit",
            ));
        }
        self.chunk_frames = self
            .chunk_frames
            .checked_add(1)
            .ok_or_else(|| resource("Chunk frame count exceeds u64"))?;
        if self.chunk_frames > self.effective.chunk_count {
            return Err(Diagnostic::new(
                OutcomeClass::PolicyRefused,
                ReasonCode::ResourceLimit,
                "STREAM Chunk count exceeds the bound in force",
            ));
        }
        let stored = self
            .source
            .read_body_bytes(parsed.stored_len, "a Chunk frame payload")?;

        if !parsed.region_owned {
            let plan = self
                .plan_map
                .get(&parsed.plan_ref)
                .cloned()
                .ok_or_else(|| {
                    Diagnostic::new(
                        OutcomeClass::Unsupported,
                        ReasonCode::UnknownTransformPlan,
                        format!("Chunk {} uses plan {}", parsed.chunk_id, parsed.plan_ref),
                    )
                })?;
            let prefix = self.bounded_prefix(&plan, &parsed)?;
            let plaintext = decode_frame_payload(
                &plan,
                &stored,
                parsed.logical_len,
                &self.dictionaries,
                &self.reconstruction_data,
                prefix.as_deref(),
            )?;
            if sha256_exact(&plaintext) != parsed.chunk_id {
                let code = if plan
                    .transforms
                    .iter()
                    .any(|step| step.reconstruction_ref.is_some())
                {
                    ReasonCode::ReconstructedDigestMismatch
                } else {
                    ReasonCode::ChunkDigestMismatch
                };
                return Err(integrity(code, format!("Chunk {}", parsed.chunk_id)));
            }
            self.record_group_member(&plan, &parsed, &plaintext)?;
            self.staging.insert(parsed.chunk_id, plaintext)?;
        } else {
            self.region_owned_declared.insert(parsed.chunk_id);
        }

        let ordinal = u64::try_from(self.physical_order.len())
            .map_err(|_| resource("frame ordinal exceeds u64"))?;
        self.frame_index.insert(parsed.chunk_id, ordinal);
        self.physical_order.push(parsed.chunk_id);
        self.locators.insert(
            parsed.chunk_id,
            ChunkLocation {
                offset: item_offset
                    .checked_add(STREAM_ITEM_HEADER_LEN)
                    .ok_or_else(|| resource("frame offset overflow"))?,
                stored_len: parsed.stored_len,
            },
        );
        self.chunks.insert(
            parsed.chunk_id,
            Chunk {
                chunk_id: parsed.chunk_id,
                logical_len: parsed.logical_len,
                plan_ref: parsed.plan_ref,
                group_ref: parsed.group_ref,
                plaintext: Box::default(),
            },
        );
        self.peak_retained_chunks = self.peak_retained_chunks.max(
            u64::try_from(self.staging.len()).map_err(|_| resource("staged count exceeds u64"))?,
        );
        Ok(())
    }

    /// Builds the bounded-lookback prefix from this group's declared history.
    fn bounded_prefix(
        &self,
        plan: &TransformPlan,
        parsed: &ChunkFrameHeader,
    ) -> Result<Option<Vec<u8>>> {
        let PlanMode::Prefix { lookback } = plan_mode(plan)? else {
            if parsed.group_ref.is_some() {
                return Err(Diagnostic::new(
                    OutcomeClass::Nonconforming,
                    ReasonCode::InvalidGroupReference,
                    format!(
                        "Chunk {} declares a group but its plan is not prefix coded",
                        parsed.chunk_id
                    ),
                ));
            }
            return Ok(None);
        };
        let group_id = parsed.group_ref.ok_or_else(|| {
            Diagnostic::new(
                OutcomeClass::Nonconforming,
                ReasonCode::InvalidGroupReference,
                format!("prefix Chunk {} has no group_ref", parsed.chunk_id),
            )
        })?;
        let group = self.chunk_groups.get(&group_id).ok_or_else(|| {
            Diagnostic::new(
                OutcomeClass::Nonconforming,
                ReasonCode::InvalidGroupReference,
                group_id.to_string(),
            )
        })?;
        if group.max_lookback != lookback {
            return Err(Diagnostic::new(
                OutcomeClass::Nonconforming,
                ReasonCode::LookbackViolation,
                format!("ChunkGroup {group_id} and TransformPlan disagree"),
            ));
        }
        let history = self
            .group_prefix
            .get(&group_id)
            .map(|values| values.iter().map(Vec::as_slice).collect::<Vec<_>>())
            .unwrap_or_default();
        Ok(Some(physical_prefix_from_slices(&history, lookback)?))
    }

    /// Retains only as much group history as the group's declared lookback.
    fn record_group_member(
        &mut self,
        plan: &TransformPlan,
        parsed: &ChunkFrameHeader,
        plaintext: &[u8],
    ) -> Result<()> {
        let PlanMode::Prefix { lookback } = plan_mode(plan)? else {
            return Ok(());
        };
        let Some(group_id) = parsed.group_ref else {
            return Ok(());
        };
        let history = self.group_prefix.entry(group_id).or_default();
        history.push_back(plaintext.to_vec());
        while history.len() > usize::try_from(lookback).unwrap_or(usize::MAX) {
            history.pop_front();
        }
        Ok(())
    }

    fn accept_content_object(&mut self, object: ContentObject) -> Result<()> {
        if self.entry_records_started {
            return Err(Diagnostic::new(
                OutcomeClass::Nonconforming,
                ReasonCode::StreamItemOrdering,
                "ContentObject records cannot follow Entry records",
            ));
        }
        if self
            .previous_object
            .is_some_and(|previous| previous >= object.logical_digest)
        {
            return Err(noncanonical(
                "ContentObjects must be uniquely ordered by logical digest",
            ));
        }
        self.previous_object = Some(object.logical_digest);

        let emitted =
            u64::try_from(self.physical_order.len()).map_err(|_| resource("frames exceed u64"))?;
        if self.current_batch_position != Some(emitted) {
            self.run_start = self.previous_batch_end;
            self.previous_batch_end = emitted;
            self.current_batch_position = Some(emitted);
            self.release_outside_window();
        }

        let window = self.preamble.stream_dedup_window;
        let retain_from = self.run_start.saturating_sub(window);
        for chunk_ref in &object.chunks {
            let ordinal = *self.frame_index.get(&chunk_ref.chunk_id).ok_or_else(|| {
                Diagnostic::new(
                    OutcomeClass::Nonconforming,
                    ReasonCode::StreamForwardReference,
                    format!(
                        "ContentObject {} references Chunk {} before its CHUNK_FRAME",
                        object.logical_digest, chunk_ref.chunk_id
                    ),
                )
            })?;
            if ordinal < self.run_start {
                let distance = self.run_start - ordinal;
                self.observed_window = self.observed_window.max(distance);
                if ordinal < retain_from {
                    return Err(Diagnostic::new(
                        OutcomeClass::Nonconforming,
                        ReasonCode::StreamWindowExceeded,
                        format!(
                            "ContentObject {} depends on a Chunk {distance} frames before its run \
                             but the archive declares a stream dedup window of {window}",
                            object.logical_digest
                        ),
                    ));
                }
            }
        }

        self.reconstruct_regions_for(&object)?;
        self.verify_content_object(&object)?;
        if self.objects.insert(object.logical_digest, object).is_some() {
            return Err(Diagnostic::new(
                OutcomeClass::Nonconforming,
                ReasonCode::DuplicateSemanticDeclaration,
                "duplicate ContentObject declaration",
            ));
        }
        self.peak_retained_chunks = self.peak_retained_chunks.max(
            u64::try_from(self.staging.len()).map_err(|_| resource("staged count exceeds u64"))?,
        );
        Ok(())
    }

    /// Rebuilds any whole-object region owned by this ContentObject.
    ///
    /// A sequential reader learns which Chunks a region owns only when the
    /// region's ContentObject record arrives, so reconstruction is deferred to
    /// exactly this point. Every member Chunk is verified against its declared
    /// digest before its plaintext is staged.
    fn reconstruct_regions_for(&mut self, object: &ContentObject) -> Result<()> {
        let Some(region_ids) = self.regions_by_object.get(&object.logical_digest).cloned() else {
            return Ok(());
        };
        let plans = self
            .plans
            .iter()
            .map(|plan| (plan.plan_id, plan))
            .collect::<BTreeMap<_, _>>();
        for region_id in region_ids {
            let region = self
                .reconstruction_regions
                .get(&region_id)
                .cloned()
                .ok_or_else(|| {
                    Diagnostic::new(
                        OutcomeClass::Nonconforming,
                        ReasonCode::UnknownReconstructionRegion,
                        region_id.to_string(),
                    )
                })?;
            let start = usize::try_from(region.start_chunk_index)
                .map_err(|_| structure("region start exceeds usize"))?;
            let end = start
                .checked_add(
                    usize::try_from(region.chunk_count)
                        .map_err(|_| structure("region count exceeds usize"))?,
                )
                .ok_or_else(|| structure("region range overflows"))?;
            let members = object.chunks.get(start..end).ok_or_else(|| {
                Diagnostic::new(
                    OutcomeClass::Nonconforming,
                    ReasonCode::InvalidReconstructionRegion,
                    region_id.to_string(),
                )
            })?;
            let mut member_lengths = Vec::with_capacity(members.len());
            for chunk_ref in members {
                if !self.region_owned_declared.contains(&chunk_ref.chunk_id) {
                    return Err(Diagnostic::new(
                        OutcomeClass::Nonconforming,
                        ReasonCode::UnknownReconstructionRegion,
                        format!(
                            "region {region_id} member {} has no region-owned Chunk frame",
                            chunk_ref.chunk_id
                        ),
                    ));
                }
                member_lengths.push(
                    self.chunks
                        .get(&chunk_ref.chunk_id)
                        .map(|chunk| chunk.logical_len)
                        .ok_or_else(|| unknown_chunk(&chunk_ref.chunk_id))?,
                );
            }
            for (chunk_id, plaintext) in
                reconstruct_region_members(&region, object, &plans, &member_lengths)?
            {
                self.staging.insert(chunk_id, plaintext)?;
            }
        }
        Ok(())
    }

    fn verify_content_object(&mut self, object: &ContentObject) -> Result<()> {
        let mut hasher = Sha256::new();
        let mut leaves = Vec::with_capacity(object.chunks.len());
        for chunk_ref in &object.chunks {
            let plaintext = self.staging.read(&chunk_ref.chunk_id)?;
            let chunk = self
                .chunks
                .get(&chunk_ref.chunk_id)
                .ok_or_else(|| unknown_chunk(&chunk_ref.chunk_id))?;
            if u64::try_from(plaintext.len()).unwrap_or(u64::MAX) != chunk.logical_len {
                return Err(integrity(
                    ReasonCode::ChunkDigestMismatch,
                    format!("Chunk {} logical length", chunk_ref.chunk_id),
                ));
            }
            hasher.update(&plaintext);
            leaves.push((chunk_ref.chunk_id, chunk.logical_len));
        }
        if Digest::from_bytes(hasher.finalize().into()) != object.logical_digest {
            return Err(integrity(
                ReasonCode::ContentDigestMismatch,
                object.logical_digest.to_string(),
            ));
        }
        if crate::identity::chunk_root_from_leaves(&leaves) != object.chunk_root {
            return Err(integrity(
                ReasonCode::ChunkRootMismatch,
                object.logical_digest.to_string(),
            ));
        }
        Ok(())
    }

    /// Releases every Chunk older than the declared window allows.
    ///
    /// Each frame is released at most once, so a whole pass costs one release
    /// per Chunk rather than one per record.
    fn release_outside_window(&mut self) {
        if !matches!(self.limits.content, StreamContentPolicy::Verify) {
            return;
        }
        let retain_from = self
            .run_start
            .saturating_sub(self.preamble.stream_dedup_window);
        let limit = usize::try_from(retain_from)
            .unwrap_or(usize::MAX)
            .min(self.physical_order.len());
        for chunk_id in &self.physical_order[self.released_upto..limit] {
            self.staging.release(chunk_id);
        }
        self.released_upto = self.released_upto.max(limit);
    }

    fn accept_entry(&mut self, entry: Entry) -> Result<()> {
        self.entry_records_started = true;
        if let Some(previous) = self.entries.last()
            && previous.path() >= entry.path()
        {
            return Err(noncanonical(
                "Entry records are not in canonical LogicalPath order",
            ));
        }
        let depth = u64::try_from(entry.path().depth()).unwrap_or(u64::MAX);
        if depth > self.effective.max_path_depth {
            return Err(Diagnostic::new(
                OutcomeClass::PolicyRefused,
                ReasonCode::ResourceLimit,
                "Entry path depth exceeds the bound in force",
            ));
        }
        if let crate::eam::EntryData::File {
            content: crate::eam::ContentRef::Internal(logical_digest),
        } = entry.data()
        {
            let object = self.objects.get(&logical_digest).ok_or_else(|| {
                Diagnostic::new(
                    OutcomeClass::Nonconforming,
                    ReasonCode::StreamForwardReference,
                    format!(
                        "Entry {} references ContentObject {logical_digest} before its record",
                        entry.path()
                    ),
                )
            })?;
            let mut size = 0_u64;
            for chunk_ref in &object.chunks {
                size = size
                    .checked_add(
                        self.chunks
                            .get(&chunk_ref.chunk_id)
                            .map(|chunk| chunk.logical_len)
                            .ok_or_else(|| unknown_chunk(&chunk_ref.chunk_id))?,
                    )
                    .ok_or_else(|| resource("ContentObject logical size overflow"))?;
            }
            if size > self.effective.max_single_entry_logical_bytes {
                return Err(Diagnostic::new(
                    OutcomeClass::PolicyRefused,
                    ReasonCode::ResourceLimit,
                    "Entry logical size exceeds the bound in force",
                ));
            }
            self.total_logical = self
                .total_logical
                .checked_add(size)
                .ok_or_else(|| resource("total logical size overflow"))?;
            if self.total_logical > self.effective.total_logical_bytes {
                return Err(Diagnostic::new(
                    OutcomeClass::PolicyRefused,
                    ReasonCode::ResourceLimit,
                    "STREAM total logical bytes exceed the bound in force",
                ));
            }
        }
        self.entries.push(entry);
        if u64::try_from(self.entries.len()).unwrap_or(u64::MAX) > self.effective.entry_count {
            return Err(Diagnostic::new(
                OutcomeClass::PolicyRefused,
                ReasonCode::ResourceLimit,
                "STREAM entry count exceeds the bound in force",
            ));
        }
        Ok(())
    }
}

impl<R: Read> Scanner<R> {
    /// Validates the trailer, the final identities, and the exact total length.
    fn finish(mut self, footer: StreamFooter) -> Result<SequentialArchive> {
        if !self.source.at_end()? {
            return Err(Diagnostic::new(
                OutcomeClass::Corrupt,
                ReasonCode::IncorrectTotalLength,
                "trailing bytes follow the fixed STREAM footer",
            ));
        }
        for required in self.required_tags() {
            if !self.seen_singletons.contains(&required.wire_id()) {
                return Err(structure(format!(
                    "STREAM body is missing its {required:?} item"
                )));
            }
        }
        let descriptor_body = self
            .descriptor_body
            .take()
            .ok_or_else(|| structure("STREAM body is missing its DESCRIPTOR item"))?;
        let fidelity = self
            .fidelity
            .take()
            .ok_or_else(|| structure("STREAM body is missing its FIDELITY item"))?;
        let (descriptor_offset, descriptor_len) = self
            .descriptor_location
            .ok_or_else(|| structure("STREAM DESCRIPTOR item has no recorded location"))?;

        if footer.total_len != self.source.consumed {
            return Err(Diagnostic::new(
                if footer.total_len > self.source.consumed {
                    OutcomeClass::Truncated
                } else {
                    OutcomeClass::Corrupt
                },
                ReasonCode::IncorrectTotalLength,
                format!(
                    "declared {} bytes, read {}",
                    footer.total_len, self.source.consumed
                ),
            ));
        }
        if footer.body_len != self.source.body_len {
            return Err(structure(format!(
                "footer declares a {}-byte STREAM_BODY but {} bytes were read",
                footer.body_len, self.source.body_len
            )));
        }
        if Digest::from_bytes(self.source.body.clone().finalize().into()) != footer.body_digest {
            return Err(integrity(
                ReasonCode::SectionDigestMismatch,
                "STREAM_BODY digest mismatch",
            ));
        }
        if sha256_exact(&self.preamble_bytes) != footer.preamble_digest {
            return Err(integrity(
                ReasonCode::FooterBindingMismatch,
                "footer preamble binding mismatch",
            ));
        }
        if footer.descriptor_offset != descriptor_offset || footer.descriptor_len != descriptor_len
        {
            return Err(structure(
                "footer descriptor locator does not match the emitted DESCRIPTOR item",
            ));
        }
        if descriptor_body.namespace != FORMAT_NAMESPACE
            || descriptor_body.identity_profile != 1
            || descriptor_body.digest_algorithm != 1
        {
            return Err(Diagnostic::new(
                OutcomeClass::Unsupported,
                ReasonCode::UnsupportedRequiredFeature,
                "unsupported descriptor namespace, identity profile, or digest algorithm",
            ));
        }

        let entry_count =
            u64::try_from(self.entries.len()).map_err(|_| resource("entry count exceeds u64"))?;
        let chunk_count =
            u64::try_from(self.chunks.len()).map_err(|_| resource("Chunk count exceeds u64"))?;
        if footer.entry_count != entry_count
            || footer.chunk_count != chunk_count
            || footer.total_logical != self.total_logical
        {
            return Err(structure(
                "footer actual totals disagree with the records that were read",
            ));
        }

        if self.limits.content == StreamContentPolicy::Retain {
            for (chunk_id, chunk) in &mut self.chunks {
                chunk.plaintext = self.staging.read(chunk_id)?.into_boxed_slice();
            }
        }

        let entry_set = EntrySet::from_canonical(std::mem::take(&mut self.entries))?;
        let descriptor = ArchiveDescriptor {
            format_major: self.preamble.version.major,
            format_minor: self.preamble.version.minor,
            format_namespace: descriptor_body.namespace,
            features: self.preamble.features,
            layout: Layout::Stream,
            role: ArchiveRole::Complete,
            budget_declared: self.preamble.budget_declared,
            stream_dedup_window: self.preamble.stream_dedup_window,
            budget: self.preamble.budget,
            decode: self.preamble.decode,
            identity_profile: IdentityProfile::IdentityV1,
            digest_algorithm: DigestAlgorithm::Sha256,
            planner_id: descriptor_body.planner_id,
            chunker_id: descriptor_body.chunker_id,
            lai: descriptor_body.lai,
            pcr: descriptor_body.pcr,
            aux: descriptor_body.aux,
            pci: None,
        };
        let locators = std::mem::take(&mut self.locators);
        let archive = Archive {
            descriptor,
            entry_set,
            content_store: ContentStore {
                objects: std::mem::take(&mut self.objects),
                chunks: std::mem::take(&mut self.chunks),
                dictionaries: std::mem::take(&mut self.dictionaries),
                reconstruction_data: std::mem::take(&mut self.reconstruction_data),
                reconstruction_fallbacks: std::mem::take(&mut self.reconstruction_fallbacks),
                reconstruction_regions: std::mem::take(&mut self.reconstruction_regions),
                reconstruction_audits: std::mem::take(&mut self.reconstruction_audits),
                chunk_groups: std::mem::take(&mut self.chunk_groups),
                physical_order: std::mem::take(&mut self.physical_order).into_boxed_slice(),
            },
            transform_plans: std::mem::take(&mut self.plans).into_boxed_slice(),
            fidelity,
            index: Index {
                present: false,
                valid: false,
                chunks: locators.clone(),
                status: "not applicable; STREAM layout carries no Index".to_owned(),
            },
        };
        archive.validate_without_retained_plaintext()?;
        validate_feature_model(&archive)?;
        self.validate_region_ownership(&archive)?;

        let expected_decode = aggregate_archive_decode_requirements(
            &archive.transform_plans,
            &archive.content_store.dictionaries,
            &archive.content_store.chunk_groups,
        )?;
        if archive.descriptor.decode != expected_decode {
            return Err(structure(
                "declared decode requirements disagree with the recorded plans",
            ));
        }
        if archive.total_logical_size()? != self.total_logical {
            return Err(structure(
                "accumulated logical size disagrees with the assembled model",
            ));
        }
        if self.preamble.budget_declared {
            self.validate_declared_budget(&archive, &locators)?;
        }

        let stored_entries = archive
            .entry_set
            .entries()
            .iter()
            .map(crate::eam::Entry::identity)
            .collect::<Vec<_>>();
        let stored_lai = archive.descriptor.lai;
        let stored_pcr = archive.descriptor.pcr;
        let stored_aux = archive.descriptor.aux;
        let (mut canonical, roots) = native_identities_from_verified(&archive)?;
        for (stored, recomputed) in stored_entries
            .iter()
            .zip(canonical.entry_set.entries().iter().map(Entry::identity))
        {
            if stored.identity_digest != recomputed.identity_digest {
                return Err(integrity(
                    ReasonCode::EntryIdentityMismatch,
                    "serialized Entry identity does not match its canonical fields",
                ));
            }
            if stored.aux_digest != recomputed.aux_digest {
                return Err(integrity(
                    ReasonCode::EntryAuxMismatch,
                    "serialized Entry auxiliary digest does not match its metadata",
                ));
            }
        }
        if stored_lai != roots.lai.0 {
            return Err(integrity(ReasonCode::LaiMismatch, "LAI mismatch"));
        }
        if stored_pcr != roots.pcr.0 {
            return Err(integrity(ReasonCode::PcrMismatch, "PCR mismatch"));
        }
        if stored_aux != roots.aux.0 {
            return Err(integrity(ReasonCode::AuxMismatch, "AUX mismatch"));
        }

        let pci = PhysicalContainerIdentity(Digest::from_bytes(
            self.source.container.clone().finalize().into(),
        ));
        canonical.descriptor.pci = Some(pci.0);
        canonical.index = archive.index.clone();
        let identities = roots.with_pci(pci);
        let report = StreamReport {
            dedup_window: self.preamble.stream_dedup_window,
            budget_declared: self.preamble.budget_declared,
            actual_entry_count: entry_count,
            actual_total_logical_bytes: self.total_logical,
            actual_chunk_count: chunk_count,
            chunk_frames: self.chunk_frames,
            manifest_records: self.manifest_records,
            body_len: footer.body_len,
            total_len: footer.total_len,
            peak_retained_chunks: self.peak_retained_chunks,
            peak_resident_staging_bytes: self.staging.peak_resident_bytes(),
            spilled_staging_bytes: self.staging.spilled_bytes(),
            access: StreamAccessProfile::SEQUENTIAL,
            plaintext_retained: self.limits.content == StreamContentPolicy::Retain,
        };
        let staged = match self.limits.content {
            StreamContentPolicy::Verify => None,
            StreamContentPolicy::Stage | StreamContentPolicy::Retain => Some(StagedChunks {
                staging: self.staging,
            }),
        };
        Ok(SequentialArchive {
            opened: OpenedArchive {
                archive: canonical,
                report: VerificationReport {
                    canonical_encoding: true,
                    container_structure: true,
                    section_integrity: true,
                    semantic_invariants: true,
                    chunk_integrity: true,
                    dictionary_integrity: true,
                    reconstruction_integrity: true,
                    chunk_group_integrity: true,
                    access_costs: true,
                    content_integrity: true,
                    entry_identities: true,
                    lai: true,
                    pcr: true,
                    aux: true,
                    pci_computed: true,
                    index_status: IndexStatus::NotApplicableStream,
                    index_reason: None,
                    identities,
                },
            },
            stream: report,
            staged,
        })
    }

    fn required_tags(&self) -> Vec<StreamItemTag> {
        let mut required = vec![StreamItemTag::TransformPlans];
        if self.extended {
            required.push(StreamItemTag::Dictionaries);
            required.push(StreamItemTag::ChunkGroups);
        }
        if self.reconstructive {
            required.push(StreamItemTag::ReconstructionData);
        }
        if self.whole_object {
            required.push(StreamItemTag::ReconstructionRegions);
        }
        required.push(StreamItemTag::Fidelity);
        required.push(StreamItemTag::Descriptor);
        required
    }

    fn validate_region_ownership(&self, archive: &Archive) -> Result<()> {
        let expected = super::container::region_owned_chunk_ids(
            &archive.content_store.objects,
            &archive.content_store.reconstruction_regions,
        )?;
        if expected != self.region_owned_declared {
            return Err(Diagnostic::new(
                OutcomeClass::Nonconforming,
                ReasonCode::UnknownReconstructionRegion,
                "region-owned Chunk declarations do not match ReconstructionRegion ranges",
            ));
        }
        Ok(())
    }

    fn validate_declared_budget(
        &self,
        archive: &Archive,
        locators: &BTreeMap<Digest, ChunkLocation>,
    ) -> Result<()> {
        let budget = archive.descriptor.budget;
        let mut max_single = 0_u64;
        for object in archive.content_store.objects.values() {
            let size = object.chunks.iter().try_fold(0_u64, |total, chunk_ref| {
                total
                    .checked_add(
                        archive
                            .content_store
                            .chunks
                            .get(&chunk_ref.chunk_id)
                            .map(|chunk| chunk.logical_len)
                            .ok_or_else(|| unknown_chunk(&chunk_ref.chunk_id))?,
                    )
                    .ok_or_else(|| resource("ContentObject logical size overflow"))
            })?;
            max_single = max_single.max(size);
        }
        let metadata_bytes = self
            .manifest_payload_bytes
            .checked_add(self.supporting_metadata_bytes)
            .ok_or_else(|| resource("metadata byte total exceeds u64"))?;
        let path_depth = archive
            .entry_set
            .entries()
            .iter()
            .map(|entry| u64::try_from(entry.path().depth()).unwrap_or(u64::MAX))
            .max()
            .unwrap_or(0);
        let expansion = super::container::maximum_expansion_ratio(archive, locators)?;
        let entry_count = u64::try_from(archive.entry_set.len())
            .map_err(|_| resource("entry count exceeds u64"))?;
        let chunk_count = u64::try_from(archive.content_store.chunks.len())
            .map_err(|_| resource("Chunk count exceeds u64"))?;
        if entry_count > budget.entry_count
            || self.total_logical > budget.total_logical_bytes
            || max_single > budget.max_single_entry_logical_bytes
            || chunk_count > budget.chunk_count
            || path_depth > budget.max_path_depth
            || metadata_bytes > budget.max_metadata_bytes
            || expansion > budget.max_expansion_ratio_milli
            || budget.max_key_derivation_cost != 0
        {
            return Err(Diagnostic::new(
                OutcomeClass::Corrupt,
                ReasonCode::ResourceLimit,
                "decoded actuals exceed the archive's declared bounds",
            ));
        }
        Ok(())
    }
}

fn be64(bytes: &[u8]) -> Result<u64> {
    Ok(u64::from_be_bytes(bytes.try_into().map_err(|_| {
        structure("expected exactly eight big-endian bytes")
    })?))
}

fn digest32(bytes: &[u8]) -> Result<Digest> {
    Ok(Digest::from_bytes(bytes.try_into().map_err(|_| {
        noncanonical("digest must contain exactly 32 bytes")
    })?))
}

fn unknown_chunk(chunk_id: &Digest) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::Nonconforming,
        ReasonCode::UnknownChunk,
        chunk_id.to_string(),
    )
}

fn truncated(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(OutcomeClass::Truncated, ReasonCode::TruncatedStream, detail)
}

fn noncanonical(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::Nonconforming,
        ReasonCode::NoncanonicalEncoding,
        detail,
    )
}

fn structure(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(OutcomeClass::Corrupt, ReasonCode::SectionStructure, detail)
}

fn integrity(code: ReasonCode, detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(OutcomeClass::Corrupt, code, detail)
}

fn resource(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::PolicyRefused,
        ReasonCode::ResourceLimit,
        detail,
    )
}

fn io(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(OutcomeClass::PolicyRefused, ReasonCode::Io, detail)
}

#[cfg(test)]
mod tests {
    use super::{
        StreamContentPolicy, StreamItemTag, StreamWindow, StreamWriteOptions,
        bootstrap_sequential_limits, encode_stream, open_stream_with_limits,
    };
    use crate::codec::{lz4_plan, lzma2_plan, store_plan, zstd_plan};
    use crate::eam::{
        Archive, ArchiveDescriptor, ArchiveRole, ContentRef, ContentStore, DecodeRequirements,
        Digest, DigestAlgorithm, Entry, EntryData, EntryIdentity, EntrySet, FeatureSet,
        FidelityReport, IdentityProfile, Index, Layout, LogicalPath, MetadataSet, ResourceBudget,
        TransformPlan,
    };
    use crate::ecf::{FEATURE_CODEC_TRANSFORM_V1, WriteOptions, encode, open};
    use crate::identity::build_content;
    use std::collections::BTreeMap;

    /// STORE, Zstandard, LZ4, and LZMA2 all reach the same model under either
    /// layout. Plans are constructed directly so every codec is exercised
    /// regardless of what a cost-driven planner would have chosen.
    #[test]
    fn every_registered_codec_encodes_identically_under_both_layouts() {
        let plans = vec![
            store_plan(),
            zstd_plan(3).unwrap(),
            lz4_plan(Box::default()).unwrap(),
            lzma2_plan(4, 1024 * 1024, Box::default()).unwrap(),
        ];
        let payloads: Vec<Vec<u8>> = (0..plans.len())
            .map(|index| {
                (0..40_000_u32)
                    .map(|value| {
                        (value.wrapping_mul(2_654_435_761).wrapping_add(index as u32) >> 11) as u8
                    })
                    .collect()
            })
            .collect();

        let mut entries = Vec::new();
        let mut objects = BTreeMap::new();
        let mut chunks = BTreeMap::new();
        for (index, (plan, payload)) in plans.iter().zip(&payloads).enumerate() {
            let (object, object_chunks) =
                build_content(payload, payload.len(), plan.plan_id).unwrap();
            entries.push(Entry::new(
                LogicalPath::from_utf8(&[format!("payload-{index}.bin")]).unwrap(),
                EntryData::File {
                    content: ContentRef::Internal(object.logical_digest),
                },
                MetadataSet::default(),
                EntryIdentity::default(),
            ));
            objects.insert(object.logical_digest, object);
            chunks.extend(object_chunks);
        }

        let archive = Archive {
            descriptor: ArchiveDescriptor {
                format_major: 0,
                format_minor: 1,
                format_namespace: crate::ecf::FORMAT_NAMESPACE.to_owned(),
                features: FeatureSet {
                    incompat: FEATURE_CODEC_TRANSFORM_V1,
                    read_only_compat: 0,
                    compat: 0,
                },
                layout: Layout::Indexed,
                role: ArchiveRole::Complete,
                budget_declared: true,
                stream_dedup_window: 0,
                budget: ResourceBudget::default(),
                decode: DecodeRequirements::default(),
                identity_profile: IdentityProfile::IdentityV1,
                digest_algorithm: DigestAlgorithm::Sha256,
                planner_id: "codec-coverage-test".to_owned(),
                chunker_id: "whole-object/test-v1".to_owned(),
                lai: Digest::ZERO,
                pcr: Digest::ZERO,
                aux: Digest::ZERO,
                pci: None,
            },
            entry_set: EntrySet::new(entries).unwrap(),
            content_store: ContentStore {
                physical_order: chunks
                    .keys()
                    .copied()
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
                objects,
                chunks,
                ..ContentStore::default()
            },
            transform_plans: plans.into_boxed_slice(),
            fidelity: FidelityReport {
                platform: "test".to_owned(),
                ..FidelityReport::default()
            },
            index: Index::default(),
        };

        let indexed = encode(&archive, WriteOptions::default()).unwrap();
        let mut bytes = Vec::new();
        let summary = encode_stream(
            &archive,
            StreamWriteOptions {
                window: StreamWindow::Auto,
                budget_declared: true,
            },
            &mut bytes,
        )
        .unwrap();

        assert_eq!(indexed.identities.lai, summary.identities.lai);
        assert_eq!(indexed.identities.pcr, summary.identities.pcr);
        assert_eq!(indexed.identities.aux, summary.identities.aux);
        assert_ne!(indexed.identities.pci, summary.identities.pci);
        assert_eq!(summary.dedup_window, 0);

        let opened = open(&indexed.bytes).unwrap();
        let sequential = open_stream_with_limits(
            bytes.as_slice(),
            super::SequentialLimits {
                content: StreamContentPolicy::Retain,
                ..bootstrap_sequential_limits()
            },
        )
        .unwrap();
        assert_eq!(
            opened.archive.content_store.chunks,
            sequential.opened.archive.content_store.chunks
        );
        assert_eq!(
            opened.archive.content_store.objects,
            sequential.opened.archive.content_store.objects
        );
        assert_eq!(
            opened.archive.entry_set,
            sequential.opened.archive.entry_set
        );

        let codecs = sequential
            .opened
            .archive
            .transform_plans
            .iter()
            .map(|plan: &TransformPlan| plan.codec.clone())
            .collect::<Vec<_>>();
        for expected in ["store/v1", "zstandard/v1", "lz4/v1", "lzma2/v1"] {
            assert!(codecs.iter().any(|codec| codec == expected), "{expected}");
        }
    }

    #[test]
    fn item_tags_round_trip_through_their_wire_values() {
        for tag in [
            StreamItemTag::TransformPlans,
            StreamItemTag::Dictionaries,
            StreamItemTag::ChunkGroups,
            StreamItemTag::ReconstructionData,
            StreamItemTag::ReconstructionRegions,
            StreamItemTag::ChunkFrame,
            StreamItemTag::ManifestRecord,
            StreamItemTag::Fidelity,
            StreamItemTag::Descriptor,
        ] {
            assert_eq!(StreamItemTag::from_wire(tag.wire_id()).unwrap(), tag);
        }
        assert!(StreamItemTag::from_wire(0).is_err());
        assert!(StreamItemTag::from_wire(10).is_err());
    }

    #[test]
    fn chunk_frames_and_manifest_records_share_one_interleaved_stage() {
        assert_eq!(
            StreamItemTag::ChunkFrame.stage(),
            StreamItemTag::ManifestRecord.stage()
        );
        assert!(StreamItemTag::TransformPlans.stage() < StreamItemTag::ChunkFrame.stage());
        assert!(StreamItemTag::Descriptor.stage() > StreamItemTag::Fidelity.stage());
        assert!(!StreamItemTag::ChunkFrame.is_singleton());
        assert!(StreamItemTag::Descriptor.is_singleton());
    }

    #[test]
    fn explicit_stream_packing_defaults_to_a_zero_window() {
        assert_eq!(
            StreamWriteOptions::default().window,
            StreamWindow::Ceiling(0)
        );
        assert!(StreamWriteOptions::default().budget_declared);
    }
}
