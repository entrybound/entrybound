//! Metadata-first, range-backed access to Complete INDEXED archives.

use std::collections::{BTreeMap, BTreeSet};

use super::container::{
    FOOTER_MAGIC, chunk_frame_header_len, decode_frame_payload, decode_preamble,
    enforce_chunk_bounds, has_codec_transform_feature, has_cross_file_feature,
    has_reconstructive_feature, has_whole_object_feature, parse_chunk_frame_header,
    physical_prefix_from_slices, reconstruct_region_members, section_kind,
};
use super::records::{
    decode_chunk_groups, decode_descriptor, decode_dictionaries, decode_index, decode_manifest,
    decode_reconstruction_regions, decode_reconstruction_section, decode_transform_plans,
    decode_transform_plans_v2, decode_transform_plans_v3,
};
use super::{FOOTER_LEN, FORMAT_NAMESPACE, PREAMBLE_LEN, SECTION_HEADER_LEN, SectionKind};
use crate::codec::{PlanMode, aggregate_archive_decode_requirements, plan_mode, validate_plans};
use crate::diagnostics::{Diagnostic, OutcomeClass, ReasonCode, Result};
use crate::eam::{
    ArchiveDescriptor, ArchiveRole, Chunk, ChunkGroup, ChunkLocation, ContentObject, ContentRef,
    Dictionary, Digest, DigestAlgorithm, EntryData, EntrySet, IdentityProfile, Layout,
    ReconstructionData, ReconstructionRegion, TransformPlan,
};
use crate::identity::{chunk_root_from_leaves, sha256_exact, verify_metadata_lai};
use crate::random_access::{
    AccessPurpose, AccessTraceEntry, RandomAccessPolicy, RandomReadSource, RangeSession,
    SourceRevision,
};

const SECTION_MAGIC: [u8; 4] = *b"EBS1";

/// Session-local status of the non-authoritative Index cache.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RandomAccessIndexStatus {
    PresentValid,
    RebuiltAbsent,
    RebuiltInvalid,
}

/// Precise identity status for a partial access operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IdentityVerificationStatus {
    Verified,
    NotRequested,
    DeclaredNotFullyVerified,
    NotComputed,
}

/// Metadata obtained without decoding every physical Chunk.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RandomAccessMetadata {
    pub descriptor: ArchiveDescriptor,
    pub entries: EntrySet,
    pub content_objects: BTreeMap<Digest, ContentObject>,
    pub source_length: u64,
    pub source_revision: SourceRevision,
    pub section_count: u64,
    pub section_directory: Box<[RandomAccessSection]>,
    pub encrypted_segment_count: Option<u64>,
    pub encrypted: bool,
}

/// Public physical directory information authenticated or structurally checked
/// during metadata-first opening.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RandomAccessSection {
    pub kind: String,
    pub offset: u64,
    pub payload_length: u64,
}

/// Explicit proof boundary for one range-backed operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RandomAccessVerificationReport {
    pub source_revision_stable: bool,
    pub preamble_footer_verified: bool,
    pub section_structure_verified: bool,
    pub semantic_metadata_sections_verified: bool,
    pub index_status: RandomAccessIndexStatus,
    pub requested_path: Option<String>,
    pub content_object_digest_verified: bool,
    pub chunk_count_verified: u64,
    pub dependency_chunk_count: u64,
    pub dictionaries_verified: bool,
    pub groups_verified: bool,
    pub reconstruction_verified: bool,
    pub bytes_fetched: u64,
    pub range_request_count: u64,
    pub lai: IdentityVerificationStatus,
    pub aux: IdentityVerificationStatus,
    pub pcr: IdentityVerificationStatus,
    pub pci: IdentityVerificationStatus,
    pub whole_archive_verified: bool,
    pub access_trace: Box<[AccessTraceEntry]>,
}

/// One completely verified logical file and its deliberately partial report.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RandomAccessRead {
    pub bytes: Box<[u8]>,
    pub report: RandomAccessVerificationReport,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SectionDirectoryEntry {
    kind: SectionKind,
    header_offset: u64,
    payload_offset: u64,
    payload_len: u64,
    payload_digest: Digest,
}

#[derive(Clone, Copy, Debug)]
struct Footer {
    offset: u64,
    descriptor_offset: u64,
    descriptor_len: u64,
    manifest_offset: u64,
    manifest_len: u64,
    entry_count: u64,
    total_logical: u64,
}

#[derive(Clone, Copy, Debug)]
struct FrameLocator {
    offset: u64,
    stored_len: u64,
}

/// Open INDEXED metadata without decoding unrelated Chunk payloads.
pub struct RandomAccessArchive {
    session: RangeSession,
    metadata: RandomAccessMetadata,
    by_kind: BTreeMap<u8, SectionDirectoryEntry>,
    index: BTreeMap<Digest, FrameLocator>,
    index_status: RandomAccessIndexStatus,
    plans: Option<Box<[TransformPlan]>>,
    dictionaries: Option<BTreeMap<Digest, Dictionary>>,
    groups: Option<BTreeMap<Digest, ChunkGroup>>,
    reconstruction_data: Option<BTreeMap<Digest, ReconstructionData>>,
    regions: Option<BTreeMap<Digest, ReconstructionRegion>>,
    scanned_headers: Option<BTreeMap<Digest, (FrameLocator, super::container::ChunkFrameHeader)>>,
    extended: bool,
    reconstructive: bool,
    whole_object: bool,
}

/// Opens a Complete INDEXED archive over any revision-aware random source.
pub fn open_indexed_random(
    source: impl RandomReadSource + 'static,
    policy: RandomAccessPolicy,
) -> Result<RandomAccessArchive> {
    RandomAccessArchive::open(Box::new(source), policy)
}

impl RandomAccessArchive {
    fn open(source: Box<dyn RandomReadSource>, policy: RandomAccessPolicy) -> Result<Self> {
        let mut session = RangeSession::new(source, policy)?;
        let source_length = session.len();
        if source_length < PREAMBLE_LEN + FOOTER_LEN {
            return Err(Diagnostic::new(
                OutcomeClass::Truncated,
                ReasonCode::TruncatedFooter,
                "source is too short to contain an INDEXED footer",
            ));
        }
        let footer_offset = source_length - FOOTER_LEN;
        let footer_bytes = session.read(footer_offset, FOOTER_LEN, AccessPurpose::Footer)?;
        let footer = parse_footer(&footer_bytes, footer_offset, source_length)?;
        let preamble_bytes = session.read(0, PREAMBLE_LEN, AccessPurpose::Preamble)?;
        let preamble = decode_preamble(&preamble_bytes)?;
        if preamble.layout != Layout::Indexed {
            return Err(Diagnostic::new(
                OutcomeClass::Unsupported,
                ReasonCode::RandomAccessNotIndexed,
                "random access is defined only for INDEXED layout",
            ));
        }
        if preamble.features.incompat & crate::crypto::FEATURE_ENCRYPTED_INDEXED_V1 != 0 {
            return Err(Diagnostic::new(
                OutcomeClass::Unsupported,
                ReasonCode::CryptoNoMatchingRecipient,
                "encrypted random access requires an explicit crypto unlock option",
            ));
        }
        if sha256_exact(&preamble_bytes).as_bytes() != &footer_bytes[64..96] {
            return Err(integrity(
                ReasonCode::FooterBindingMismatch,
                "footer preamble binding mismatch",
            ));
        }
        if preamble.footer_hint != footer_offset {
            return Err(structure(
                "preamble footer hint does not locate the fixed footer",
            ));
        }
        super::enforce_caller_policy(preamble.budget, session.policy().resource_policy)?;
        super::enforce_decode_policy(preamble.decode, session.policy().decode_policy)?;
        let extended = has_cross_file_feature(preamble.features);
        let reconstructive = has_reconstructive_feature(preamble.features);
        let whole_object = has_whole_object_feature(preamble.features);
        let (sections, by_kind) = walk_sections(
            &mut session,
            footer.offset,
            extended,
            reconstructive,
            whole_object,
        )?;
        let descriptor_section = required_kind(&by_kind, SectionKind::Descriptor)?;
        let manifest_section = required_kind(&by_kind, SectionKind::ManifestRecords)?;
        if descriptor_section.header_offset != footer.descriptor_offset
            || descriptor_section.payload_len + SECTION_HEADER_LEN != footer.descriptor_len
            || manifest_section.header_offset != footer.manifest_offset
            || manifest_section.payload_len + SECTION_HEADER_LEN != footer.manifest_len
        {
            return Err(structure(
                "footer Descriptor/Manifest locators disagree with section framing",
            ));
        }
        let descriptor_payload =
            read_section(&mut session, descriptor_section, AccessPurpose::Descriptor)?;
        let descriptor_body = decode_descriptor(&descriptor_payload)?;
        if descriptor_body.declarations.is_some()
            || descriptor_body.namespace != FORMAT_NAMESPACE
            || descriptor_body.identity_profile != 1
            || descriptor_body.digest_algorithm != 1
        {
            return Err(Diagnostic::new(
                OutcomeClass::Unsupported,
                ReasonCode::UnsupportedRequiredFeature,
                "random unencrypted reader requires canonical Descriptor v1",
            ));
        }
        let manifest_payload =
            read_section(&mut session, manifest_section, AccessPurpose::Manifest)?;
        let (entries, content_objects) = decode_manifest(&manifest_payload)?;
        if u64::try_from(entries.len()).unwrap_or(u64::MAX) != footer.entry_count {
            return Err(structure("footer entry count disagrees with Manifest"));
        }
        verify_manifest_references(&entries, &content_objects)?;
        verify_metadata_lai(&entries, footer.total_logical, descriptor_body.lai)?;
        let descriptor = ArchiveDescriptor {
            format_major: preamble.version.major,
            format_minor: preamble.version.minor,
            format_namespace: descriptor_body.namespace,
            features: preamble.features,
            layout: Layout::Indexed,
            role: ArchiveRole::Complete,
            budget_declared: preamble.budget_declared,
            stream_dedup_window: preamble.stream_dedup_window,
            budget: preamble.budget,
            decode: preamble.decode,
            identity_profile: IdentityProfile::IdentityV1,
            digest_algorithm: DigestAlgorithm::Sha256,
            planner_id: descriptor_body.planner_id,
            chunker_id: descriptor_body.chunker_id,
            lai: descriptor_body.lai,
            pcr: descriptor_body.pcr,
            aux: descriptor_body.aux,
            pci: None,
        };
        let index_section = find_kind(&by_kind, SectionKind::Index);
        let (index, index_status) = match index_section {
            None => (BTreeMap::new(), RandomAccessIndexStatus::RebuiltAbsent),
            Some(section) => match read_section(&mut session, section, AccessPurpose::Index)
                .and_then(|bytes| decode_index(&bytes))
            {
                Ok(index)
                    if validate_index_extents(
                        &index,
                        required_kind(&by_kind, SectionKind::ChunkData)?,
                    )
                    .is_ok() =>
                {
                    (
                        index
                            .into_iter()
                            .map(|(digest, location)| {
                                (
                                    digest,
                                    FrameLocator {
                                        offset: location.offset,
                                        stored_len: location.stored_len,
                                    },
                                )
                            })
                            .collect(),
                        RandomAccessIndexStatus::PresentValid,
                    )
                }
                _ => (BTreeMap::new(), RandomAccessIndexStatus::RebuiltInvalid),
            },
        };
        let section_count = u64::try_from(sections.len())
            .map_err(|_| access_policy("section count exceeds u64"))?;
        session.check_stable()?;
        let metadata = RandomAccessMetadata {
            descriptor,
            entries,
            content_objects,
            source_length,
            source_revision: session.initial_revision().clone(),
            section_count,
            section_directory: sections
                .values()
                .map(|section| RandomAccessSection {
                    kind: format!("{:?}", section.kind),
                    offset: section.header_offset,
                    payload_length: section.payload_len,
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            encrypted_segment_count: None,
            encrypted: false,
        };
        Ok(Self {
            session,
            metadata,
            by_kind,
            index,
            index_status,
            plans: None,
            dictionaries: None,
            groups: None,
            reconstruction_data: None,
            regions: None,
            scanned_headers: None,
            extended,
            reconstructive,
            whole_object,
        })
    }

    #[must_use]
    pub fn metadata(&self) -> &RandomAccessMetadata {
        &self.metadata
    }

    /// Returns metadata-only access accounting without claiming payload or
    /// whole-archive verification.
    pub fn metadata_report(&self) -> Result<RandomAccessVerificationReport> {
        self.session.check_stable()?;
        Ok(self.report(None, false, 0, 0, false, false, false))
    }

    /// Reads and verifies one complete regular-file ContentObject.
    pub fn read_entry(&mut self, path: &crate::eam::LogicalPath) -> Result<RandomAccessRead> {
        let entry = self
            .metadata
            .entries
            .entries()
            .iter()
            .find(|entry| entry.path() == path)
            .ok_or_else(|| {
                Diagnostic::new(
                    OutcomeClass::Nonconforming,
                    ReasonCode::RandomAccessEntryNotFound,
                    path.to_string(),
                )
            })?;
        let content_id = match entry.data() {
            EntryData::File {
                content: ContentRef::Internal(digest),
            } => *digest,
            EntryData::Directory | EntryData::Symlink { .. } | EntryData::ReparsePoint { .. } => {
                return Err(Diagnostic::new(
                    OutcomeClass::Unsupported,
                    ReasonCode::RandomAccessEntryNotFile,
                    path.to_string(),
                ));
            }
        };
        let sparse_map = entry.metadata().sparse_map().cloned();
        let object = self
            .metadata
            .content_objects
            .get(&content_id)
            .cloned()
            .ok_or_else(|| dependency("Entry references an unknown ContentObject"))?;
        if object.logical_digest != content_id {
            return Err(dependency(
                "ContentObject map key and logical digest disagree",
            ));
        }
        self.load_decode_metadata()?;
        if self.index_status != RandomAccessIndexStatus::PresentValid {
            self.scan_all_chunk_headers()?;
        }
        let requested = object
            .chunks
            .iter()
            .map(|reference| reference.chunk_id)
            .collect::<BTreeSet<_>>();
        let mut closure = requested.clone();
        let mut reconstruction_needed = false;
        if self.whole_object {
            let regions = self
                .regions
                .as_ref()
                .expect("loaded with whole-object feature");
            for region in regions
                .values()
                .filter(|value| value.content_object == content_id)
            {
                reconstruction_needed = true;
                let start = usize::try_from(region.start_chunk_index)
                    .map_err(|_| dependency("region start exceeds usize"))?;
                let count = usize::try_from(region.chunk_count)
                    .map_err(|_| dependency("region count exceeds usize"))?;
                let members = object
                    .chunks
                    .get(start..start.saturating_add(count))
                    .ok_or_else(|| dependency("region range exceeds ContentObject"))?;
                closure.extend(members.iter().map(|value| value.chunk_id));
            }
        }
        let dependency_limit = self.session.policy().max_dependency_chunks;
        if u64::try_from(closure.len()).unwrap_or(u64::MAX) > dependency_limit {
            return Err(access_policy(
                "requested Chunk closure exceeds caller policy",
            ));
        }
        let mut pending = closure.iter().copied().collect::<Vec<_>>();
        while let Some(chunk_id) = pending.pop() {
            let header = self.read_chunk_header(chunk_id)?;
            if header.region_owned {
                continue;
            }
            let plan = self
                .plans
                .as_ref()
                .expect("decode metadata loaded")
                .iter()
                .find(|plan| plan.plan_id == header.plan_ref)
                .ok_or_else(|| dependency(format!("unknown TransformPlan {}", header.plan_ref)))?;
            reconstruction_needed |= plan
                .transforms
                .iter()
                .any(|step| step.reconstruction_ref.is_some());
            for prerequisite in self.group_prerequisites(chunk_id, &header)? {
                if closure.insert(prerequisite) {
                    pending.push(prerequisite);
                    if u64::try_from(closure.len()).unwrap_or(u64::MAX) > dependency_limit {
                        return Err(access_policy(
                            "lookback dependency closure exceeds caller policy",
                        ));
                    }
                }
            }
        }
        let dependency_count =
            u64::try_from(closure.len().saturating_sub(requested.len())).unwrap_or(u64::MAX);
        let mut payload_ranges = Vec::new();
        for chunk_id in &closure {
            let header = self.read_chunk_header(*chunk_id)?;
            if !header.region_owned {
                let locator = self.index[chunk_id];
                payload_ranges.push((
                    locator.offset + chunk_frame_header_len(self.extended),
                    header.stored_len,
                    if requested.contains(chunk_id) {
                        AccessPurpose::Chunk
                    } else {
                        AccessPurpose::Lookback
                    },
                ));
            }
        }
        self.session.prefetch(&payload_ranges)?;
        let mut decoded = BTreeMap::<Digest, Chunk>::new();
        for chunk_id in closure {
            if decoded.contains_key(&chunk_id) {
                continue;
            }
            let header = self.read_chunk_header(chunk_id)?;
            if header.region_owned {
                decoded.insert(
                    chunk_id,
                    Chunk {
                        chunk_id,
                        logical_len: header.logical_len,
                        plan_ref: header.plan_ref,
                        group_ref: None,
                        plaintext: Box::new([]),
                    },
                );
                continue;
            }
            let prerequisites = self.group_prerequisites(chunk_id, &header)?;
            for prerequisite in prerequisites {
                self.decode_one(prerequisite, &mut decoded)?;
            }
            self.decode_one(chunk_id, &mut decoded)?;
        }
        if reconstruction_needed {
            let plans = self
                .plans
                .as_ref()
                .expect("decode metadata loaded")
                .iter()
                .map(|plan| (plan.plan_id, plan))
                .collect::<BTreeMap<_, _>>();
            let regions = self.regions.as_ref().expect("regions loaded");
            let requested_region_bytes = regions
                .values()
                .filter(|value| value.content_object == content_id)
                .try_fold(0_u64, |total, region| {
                    total
                        .checked_add(region.logical_bytes)
                        .ok_or_else(|| access_policy("region logical byte total overflow"))
                })?;
            if requested_region_bytes > self.session.policy().max_decoded_logical_bytes {
                return Err(access_policy(
                    "aggregate region access exceeds caller decoded-byte policy",
                ));
            }
            for region in regions
                .values()
                .filter(|value| value.content_object == content_id)
            {
                if region.access.logical_bytes != region.logical_bytes
                    || region.access.logical_chunks != region.chunk_count
                    || region.access.worst_reconstructed_bytes != region.logical_bytes
                {
                    return Err(Diagnostic::new(
                        OutcomeClass::Corrupt,
                        ReasonCode::InvalidRegionAccess,
                        region.region_id.to_string(),
                    ));
                }
                if region.logical_bytes > self.session.policy().max_decoded_logical_bytes {
                    return Err(access_policy(
                        "region access exceeds caller decoded-byte policy",
                    ));
                }
                let start = usize::try_from(region.start_chunk_index)
                    .map_err(|_| dependency("region start exceeds usize"))?;
                let end = start
                    .checked_add(
                        usize::try_from(region.chunk_count)
                            .map_err(|_| dependency("region count exceeds usize"))?,
                    )
                    .ok_or_else(|| dependency("region range overflow"))?;
                let member_lengths = object
                    .chunks
                    .get(start..end)
                    .ok_or_else(|| dependency("region range exceeds ContentObject"))?
                    .iter()
                    .map(|reference| {
                        decoded
                            .get(&reference.chunk_id)
                            .map(|chunk| chunk.logical_len)
                            .ok_or_else(|| dependency("region member header is unavailable"))
                    })
                    .collect::<Result<Vec<_>>>()?;
                for (chunk_id, bytes) in
                    reconstruct_region_members(region, &object, &plans, &member_lengths)?
                {
                    decoded
                        .get_mut(&chunk_id)
                        .ok_or_else(|| dependency("reconstructed member was not declared"))?
                        .plaintext = bytes.into_boxed_slice();
                }
            }
        }
        let mut output = Vec::new();
        let mut leaves = Vec::with_capacity(object.chunks.len());
        for reference in &object.chunks {
            let chunk = decoded
                .get(&reference.chunk_id)
                .ok_or_else(|| dependency("requested Chunk was not decoded"))?;
            if sha256_exact(&chunk.plaintext) != chunk.chunk_id {
                return Err(integrity(
                    ReasonCode::ChunkDigestMismatch,
                    chunk.chunk_id.to_string(),
                ));
            }
            leaves.push((chunk.chunk_id, chunk.logical_len));
            output.extend_from_slice(&chunk.plaintext);
            if u64::try_from(output.len()).unwrap_or(u64::MAX)
                > self.session.policy().max_decoded_logical_bytes
            {
                return Err(access_policy("decoded logical bytes exceed caller policy"));
            }
        }
        if chunk_root_from_leaves(&leaves) != object.chunk_root {
            return Err(integrity(
                ReasonCode::ChunkRootMismatch,
                object.logical_digest.to_string(),
            ));
        }
        if sha256_exact(&output) != object.logical_digest {
            return Err(integrity(
                ReasonCode::ContentDigestMismatch,
                object.logical_digest.to_string(),
            ));
        }
        if let Some(map) = sparse_map {
            map.validate_plaintext(&output)?;
        }
        self.session.check_stable()?;
        let report = self.report(
            Some(path.to_string()),
            true,
            u64::try_from(object.chunks.len()).unwrap_or(u64::MAX),
            dependency_count,
            self.extended,
            self.extended,
            reconstruction_needed,
        );
        Ok(RandomAccessRead {
            bytes: output.into_boxed_slice(),
            report,
        })
    }

    fn load_decode_metadata(&mut self) -> Result<()> {
        if self.plans.is_some() {
            return Ok(());
        }
        let plans_section = required_kind(&self.by_kind, SectionKind::TransformPlans)?;
        let plans_bytes =
            read_section(&mut self.session, plans_section, AccessPurpose::Descriptor)?;
        let plans = if self.whole_object {
            decode_transform_plans_v3(&plans_bytes)?
        } else if self.reconstructive {
            decode_transform_plans_v2(&plans_bytes)?
        } else {
            decode_transform_plans(
                &plans_bytes,
                has_codec_transform_feature(self.metadata.descriptor.features),
            )?
        };
        validate_plans(&plans)?;
        let dictionaries = if self.extended {
            let section = required_kind(&self.by_kind, SectionKind::Dictionaries)?;
            decode_dictionaries(&read_section(
                &mut self.session,
                section,
                AccessPurpose::Dictionary,
            )?)?
        } else {
            BTreeMap::new()
        };
        let groups = if self.extended {
            let section = required_kind(&self.by_kind, SectionKind::ChunkGroups)?;
            decode_chunk_groups(&read_section(
                &mut self.session,
                section,
                AccessPurpose::Lookback,
            )?)?
        } else {
            BTreeMap::new()
        };
        let reconstruction_data = if self.reconstructive {
            let section = required_kind(&self.by_kind, SectionKind::ReconstructionData)?;
            decode_reconstruction_section(&read_section(
                &mut self.session,
                section,
                AccessPurpose::Reconstruction,
            )?)?
            .0
        } else {
            BTreeMap::new()
        };
        let regions = if self.whole_object {
            let section = required_kind(&self.by_kind, SectionKind::ReconstructionRegions)?;
            decode_reconstruction_regions(&read_section(
                &mut self.session,
                section,
                AccessPurpose::Reconstruction,
            )?)?
            .0
        } else {
            BTreeMap::new()
        };
        let actual_decode = aggregate_archive_decode_requirements(&plans, &dictionaries, &groups)?;
        if actual_decode != self.metadata.descriptor.decode {
            return Err(structure(
                "Descriptor decode requirements disagree with authenticated dependencies",
            ));
        }
        super::enforce_decode_policy(actual_decode, self.session.policy().decode_policy)?;
        self.plans = Some(plans);
        self.dictionaries = Some(dictionaries);
        self.groups = Some(groups);
        self.reconstruction_data = Some(reconstruction_data);
        self.regions = Some(regions);
        Ok(())
    }

    fn scan_all_chunk_headers(&mut self) -> Result<()> {
        if self.scanned_headers.is_some() {
            return Ok(());
        }
        let section = required_kind(&self.by_kind, SectionKind::ChunkData)?;
        let header_len = chunk_frame_header_len(self.extended);
        let end = section
            .payload_offset
            .checked_add(section.payload_len)
            .ok_or_else(|| structure("CHUNK_DATA extent overflow"))?;
        let mut cursor = section.payload_offset;
        let mut headers = BTreeMap::new();
        let mut count = 0_u64;
        let mut previous = None;
        while cursor < end {
            count = count
                .checked_add(1)
                .ok_or_else(|| access_policy("Chunk frame count overflow"))?;
            if count > self.session.policy().max_chunk_frames_scanned {
                return Err(access_policy("Chunk-header scan exceeds caller policy"));
            }
            let bytes = self
                .session
                .read(cursor, header_len, AccessPurpose::ChunkHeader)?;
            let header = parse_chunk_frame_header(&bytes, self.extended, self.whole_object)?;
            enforce_chunk_bounds(&header, self.metadata.descriptor.budget)?;
            if !self.extended && previous.is_some_and(|digest| digest >= header.chunk_id) {
                return Err(structure("historical Chunk frames are not digest ordered"));
            }
            previous = Some(header.chunk_id);
            let frame_len = header_len
                .checked_add(header.stored_len)
                .ok_or_else(|| structure("Chunk frame length overflow"))?;
            let next = cursor
                .checked_add(frame_len)
                .ok_or_else(|| structure("Chunk frame extent overflow"))?;
            if next > end || headers.contains_key(&header.chunk_id) {
                return Err(structure("Chunk frame is duplicate or exceeds CHUNK_DATA"));
            }
            headers.insert(
                header.chunk_id,
                (
                    FrameLocator {
                        offset: cursor,
                        stored_len: header.stored_len,
                    },
                    header,
                ),
            );
            cursor = next;
        }
        if cursor != end || count != self.metadata.descriptor.budget.chunk_count {
            return Err(structure("CHUNK_DATA frame count/extent mismatch"));
        }
        self.index = headers
            .iter()
            .map(|(digest, (locator, _))| (*digest, *locator))
            .collect();
        self.scanned_headers = Some(headers);
        Ok(())
    }

    fn read_chunk_header(
        &mut self,
        chunk_id: Digest,
    ) -> Result<super::container::ChunkFrameHeader> {
        if let Some(headers) = &self.scanned_headers {
            return headers
                .get(&chunk_id)
                .map(|(_, header)| *header)
                .ok_or_else(|| dependency(format!("unknown Chunk {chunk_id}")));
        }
        let locator = self
            .index
            .get(&chunk_id)
            .copied()
            .ok_or_else(|| dependency(format!("Index lacks requested Chunk {chunk_id}")))?;
        let header_len = chunk_frame_header_len(self.extended);
        let bytes = self
            .session
            .read(locator.offset, header_len, AccessPurpose::ChunkHeader)?;
        match parse_chunk_frame_header(&bytes, self.extended, self.whole_object) {
            Ok(header)
                if header.chunk_id == chunk_id && header.stored_len == locator.stored_len =>
            {
                enforce_chunk_bounds(&header, self.metadata.descriptor.budget)?;
                Ok(header)
            }
            _ => {
                self.index_status = RandomAccessIndexStatus::RebuiltInvalid;
                self.scan_all_chunk_headers()?;
                self.scanned_headers
                    .as_ref()
                    .and_then(|headers| headers.get(&chunk_id))
                    .map(|(_, header)| *header)
                    .ok_or_else(|| dependency(format!("unknown Chunk {chunk_id}")))
            }
        }
    }

    fn group_prerequisites(
        &mut self,
        chunk_id: Digest,
        header: &super::container::ChunkFrameHeader,
    ) -> Result<Vec<Digest>> {
        let plans = self.plans.as_ref().expect("decode metadata loaded");
        let plan = plans
            .iter()
            .find(|plan| plan.plan_id == header.plan_ref)
            .ok_or_else(|| dependency(format!("unknown TransformPlan {}", header.plan_ref)))?;
        let PlanMode::Prefix { lookback } = plan_mode(plan)? else {
            return Ok(Vec::new());
        };
        let group_id = header
            .group_ref
            .ok_or_else(|| dependency("prefix-coded Chunk lacks group_ref"))?;
        let group = self
            .groups
            .as_ref()
            .expect("decode metadata loaded")
            .get(&group_id)
            .ok_or_else(|| dependency(format!("unknown ChunkGroup {group_id}")))?
            .clone();
        if group.max_lookback != lookback {
            return Err(dependency("ChunkGroup and TransformPlan lookback disagree"));
        }
        let mut ordered = self
            .index
            .iter()
            .map(|(digest, locator)| (locator.offset, *digest))
            .collect::<Vec<_>>();
        ordered.sort();
        let position = ordered
            .iter()
            .position(|(_, digest)| *digest == chunk_id)
            .ok_or_else(|| dependency("requested Chunk has no physical position"))?;
        let first = position.saturating_sub(usize::try_from(lookback).unwrap_or(usize::MAX));
        let mut output = Vec::new();
        let mut bytes = 0_u64;
        for (_, predecessor) in &ordered[first..position] {
            let predecessor_header = self.read_chunk_header(*predecessor)?;
            if predecessor_header.group_ref == Some(group_id) {
                bytes = bytes
                    .checked_add(predecessor_header.logical_len)
                    .ok_or_else(|| dependency("lookback bytes overflow"))?;
                output.push(*predecessor);
            }
        }
        if bytes > group.max_preceding_bytes {
            return Err(Diagnostic::new(
                OutcomeClass::Corrupt,
                ReasonCode::AccessCostMismatch,
                group_id.to_string(),
            ));
        }
        Ok(output)
    }

    fn decode_one(
        &mut self,
        chunk_id: Digest,
        decoded: &mut BTreeMap<Digest, Chunk>,
    ) -> Result<()> {
        if decoded.contains_key(&chunk_id) {
            return Ok(());
        }
        let header = self.read_chunk_header(chunk_id)?;
        if header.region_owned {
            return Err(dependency(
                "region-owned Chunk cannot be decoded as ordinary payload",
            ));
        }
        let locator = self
            .index
            .get(&chunk_id)
            .copied()
            .ok_or_else(|| dependency(format!("unknown Chunk {chunk_id}")))?;
        let header_len = chunk_frame_header_len(self.extended);
        let stored_offset = locator
            .offset
            .checked_add(header_len)
            .ok_or_else(|| dependency("Chunk payload offset overflow"))?;
        let purpose = if header.group_ref.is_some() {
            AccessPurpose::Lookback
        } else {
            AccessPurpose::Chunk
        };
        let stored = self
            .session
            .read(stored_offset, header.stored_len, purpose)?;
        let plan = self
            .plans
            .as_ref()
            .expect("decode metadata loaded")
            .iter()
            .find(|plan| plan.plan_id == header.plan_ref)
            .ok_or_else(|| dependency(format!("unknown TransformPlan {}", header.plan_ref)))?
            .clone();
        let prefix = if let PlanMode::Prefix { lookback } = plan_mode(&plan)? {
            let group = header
                .group_ref
                .ok_or_else(|| dependency("prefix-coded Chunk lacks group"))?;
            let prerequisites = self.group_prerequisites(chunk_id, &header)?;
            for prerequisite in &prerequisites {
                if !decoded.contains_key(prerequisite) {
                    self.decode_one(*prerequisite, decoded)?;
                }
            }
            let slices = prerequisites
                .iter()
                .map(|digest| decoded[digest].plaintext.as_ref())
                .collect::<Vec<_>>();
            let declared = self
                .groups
                .as_ref()
                .expect("metadata loaded")
                .get(&group)
                .ok_or_else(|| dependency("unknown ChunkGroup"))?;
            if declared.max_lookback != lookback {
                return Err(dependency("lookback declaration mismatch"));
            }
            Some(physical_prefix_from_slices(&slices, lookback)?)
        } else {
            None
        };
        let plaintext = decode_frame_payload(
            &plan,
            &stored,
            header.logical_len,
            self.dictionaries.as_ref().expect("metadata loaded"),
            self.reconstruction_data.as_ref().expect("metadata loaded"),
            prefix.as_deref(),
        )?;
        if u64::try_from(plaintext.len()).unwrap_or(u64::MAX) != header.logical_len
            || sha256_exact(&plaintext) != chunk_id
        {
            return Err(integrity(
                ReasonCode::ChunkDigestMismatch,
                chunk_id.to_string(),
            ));
        }
        let retained = decoded.values().try_fold(0_u64, |total, chunk| {
            total
                .checked_add(u64::try_from(chunk.plaintext.len()).unwrap_or(u64::MAX))
                .ok_or_else(|| access_policy("decoded dependency byte total overflow"))
        })?;
        if retained.saturating_add(header.logical_len)
            > self.session.policy().max_decoded_logical_bytes
        {
            return Err(access_policy(
                "decoded dependency bytes exceed caller policy",
            ));
        }
        decoded.insert(
            chunk_id,
            Chunk {
                chunk_id,
                logical_len: header.logical_len,
                plan_ref: header.plan_ref,
                group_ref: header.group_ref,
                plaintext: plaintext.into_boxed_slice(),
            },
        );
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn report(
        &self,
        requested_path: Option<String>,
        content_verified: bool,
        chunks: u64,
        dependencies: u64,
        dictionaries: bool,
        groups: bool,
        reconstruction: bool,
    ) -> RandomAccessVerificationReport {
        RandomAccessVerificationReport {
            source_revision_stable: true,
            preamble_footer_verified: true,
            section_structure_verified: true,
            semantic_metadata_sections_verified: true,
            index_status: self.index_status,
            requested_path,
            content_object_digest_verified: content_verified,
            chunk_count_verified: chunks,
            dependency_chunk_count: dependencies,
            dictionaries_verified: dictionaries,
            groups_verified: groups,
            reconstruction_verified: reconstruction,
            bytes_fetched: self.session.bytes_fetched(),
            range_request_count: self.session.range_requests(),
            lai: IdentityVerificationStatus::Verified,
            aux: IdentityVerificationStatus::NotRequested,
            pcr: IdentityVerificationStatus::DeclaredNotFullyVerified,
            pci: IdentityVerificationStatus::NotComputed,
            whole_archive_verified: false,
            access_trace: self.session.trace().to_vec().into_boxed_slice(),
        }
    }
}

fn parse_footer(bytes: &[u8], offset: u64, source_length: u64) -> Result<Footer> {
    if bytes.len() != usize::try_from(FOOTER_LEN).unwrap_or(128) || bytes[..8] != FOOTER_MAGIC {
        return Err(Diagnostic::new(
            OutcomeClass::Truncated,
            ReasonCode::TruncatedFooter,
            "fixed INDEXED footer magic is absent",
        ));
    }
    let declared = be_u64(&bytes[8..16])?;
    if declared != source_length {
        return Err(Diagnostic::new(
            if declared > source_length {
                OutcomeClass::Truncated
            } else {
                OutcomeClass::Corrupt
            },
            ReasonCode::IncorrectTotalLength,
            format!("declared {declared} bytes, source has {source_length}"),
        ));
    }
    if bytes[96..].iter().any(|byte| *byte != 0) {
        return Err(structure("footer reserved bytes are nonzero"));
    }
    Ok(Footer {
        offset,
        descriptor_offset: be_u64(&bytes[16..24])?,
        descriptor_len: be_u64(&bytes[24..32])?,
        manifest_offset: be_u64(&bytes[32..40])?,
        manifest_len: be_u64(&bytes[40..48])?,
        entry_count: be_u64(&bytes[48..56])?,
        total_logical: be_u64(&bytes[56..64])?,
    })
}

fn walk_sections(
    session: &mut RangeSession,
    footer_offset: u64,
    extended: bool,
    reconstructive: bool,
    whole_object: bool,
) -> Result<(
    BTreeMap<u16, SectionDirectoryEntry>,
    BTreeMap<u8, SectionDirectoryEntry>,
)> {
    let mut cursor = PREAMBLE_LEN;
    let mut expected_id = 1_u16;
    let mut sections = BTreeMap::new();
    let mut by_kind = BTreeMap::new();
    while cursor < footer_offset {
        if u64::try_from(sections.len()).unwrap_or(u64::MAX) >= session.policy().max_section_count {
            return Err(access_policy(
                "section-directory walk exceeds caller policy",
            ));
        }
        if footer_offset - cursor < SECTION_HEADER_LEN {
            return Err(structure("section header is truncated"));
        }
        let header = session.read(cursor, SECTION_HEADER_LEN, AccessPurpose::SectionHeader)?;
        if header[..4] != SECTION_MAGIC
            || be_u16(&header[6..8])? != 1
            || header[8..16].iter().any(|byte| *byte != 0)
            || header[56..64].iter().any(|byte| *byte != 0)
        {
            return Err(structure("section header is malformed or noncanonical"));
        }
        let id = be_u16(&header[4..6])?;
        if id != expected_id {
            return Err(structure("section IDs are not canonical and contiguous"));
        }
        let kind = section_kind(id, extended, reconstructive, whole_object)?;
        let payload_len = be_u64(&header[16..24])?;
        let next = cursor
            .checked_add(SECTION_HEADER_LEN)
            .and_then(|value| value.checked_add(payload_len))
            .ok_or_else(|| structure("section extent overflow"))?;
        if next > footer_offset {
            return Err(structure("section payload exceeds footer boundary"));
        }
        let digest = Digest::from_bytes(
            header[24..56]
                .try_into()
                .map_err(|_| structure("section digest has wrong width"))?,
        );
        let entry = SectionDirectoryEntry {
            kind,
            header_offset: cursor,
            payload_offset: cursor + SECTION_HEADER_LEN,
            payload_len,
            payload_digest: digest,
        };
        sections.insert(id, entry);
        by_kind.insert(kind_key(kind), entry);
        cursor = next;
        expected_id = expected_id
            .checked_add(1)
            .ok_or_else(|| structure("section ID overflow"))?;
    }
    if cursor != footer_offset {
        return Err(structure("section directory does not terminate at footer"));
    }
    for required in [
        SectionKind::Descriptor,
        SectionKind::TransformPlans,
        SectionKind::ChunkData,
        SectionKind::ManifestRecords,
        SectionKind::Fidelity,
    ] {
        let _ = required_kind(&by_kind, required)?;
    }
    if extended {
        let _ = required_kind(&by_kind, SectionKind::Dictionaries)?;
        let _ = required_kind(&by_kind, SectionKind::ChunkGroups)?;
    }
    if reconstructive {
        let _ = required_kind(&by_kind, SectionKind::ReconstructionData)?;
    }
    if whole_object {
        let _ = required_kind(&by_kind, SectionKind::ReconstructionRegions)?;
    }
    Ok((sections, by_kind))
}

fn read_section(
    session: &mut RangeSession,
    section: SectionDirectoryEntry,
    purpose: AccessPurpose,
) -> Result<Vec<u8>> {
    let bytes = session.read(section.payload_offset, section.payload_len, purpose)?;
    if sha256_exact(&bytes) != section.payload_digest {
        return Err(integrity(
            ReasonCode::SectionDigestMismatch,
            format!("{:?} section digest mismatch", section.kind),
        ));
    }
    Ok(bytes)
}

fn validate_index_extents(
    index: &BTreeMap<Digest, ChunkLocation>,
    chunk_section: SectionDirectoryEntry,
) -> Result<()> {
    let start = chunk_section.payload_offset;
    let end = start
        .checked_add(chunk_section.payload_len)
        .ok_or_else(|| structure("CHUNK_DATA extent overflow"))?;
    let mut offsets = BTreeSet::new();
    for location in index.values() {
        if location.offset < start || location.offset >= end || !offsets.insert(location.offset) {
            return Err(structure(
                "Index locator is outside CHUNK_DATA or duplicated",
            ));
        }
    }
    Ok(())
}

fn verify_manifest_references(
    entries: &EntrySet,
    objects: &BTreeMap<Digest, ContentObject>,
) -> Result<()> {
    for entry in entries.entries() {
        if let EntryData::File {
            content: ContentRef::Internal(digest),
        } = entry.data()
            && !objects.contains_key(digest)
        {
            return Err(dependency(format!(
                "Entry references unknown ContentObject {digest}"
            )));
        }
    }
    Ok(())
}

fn required_kind(
    sections: &BTreeMap<u8, SectionDirectoryEntry>,
    kind: SectionKind,
) -> Result<SectionDirectoryEntry> {
    find_kind(sections, kind).ok_or_else(|| structure(format!("missing {kind:?} section")))
}

fn find_kind(
    sections: &BTreeMap<u8, SectionDirectoryEntry>,
    kind: SectionKind,
) -> Option<SectionDirectoryEntry> {
    sections.get(&kind_key(kind)).copied()
}

const fn kind_key(kind: SectionKind) -> u8 {
    match kind {
        SectionKind::Descriptor => 1,
        SectionKind::TransformPlans => 2,
        SectionKind::Dictionaries => 3,
        SectionKind::ChunkGroups => 4,
        SectionKind::ReconstructionData => 5,
        SectionKind::ReconstructionRegions => 6,
        SectionKind::ChunkData => 7,
        SectionKind::ManifestRecords => 8,
        SectionKind::Fidelity => 9,
        SectionKind::Index => 10,
    }
}

fn be_u16(bytes: &[u8]) -> Result<u16> {
    Ok(u16::from_be_bytes(
        bytes.try_into().map_err(|_| structure("expected u16"))?,
    ))
}

fn be_u64(bytes: &[u8]) -> Result<u64> {
    Ok(u64::from_be_bytes(
        bytes.try_into().map_err(|_| structure("expected u64"))?,
    ))
}

fn structure(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(OutcomeClass::Corrupt, ReasonCode::SectionStructure, detail)
}

fn integrity(code: ReasonCode, detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(OutcomeClass::Corrupt, code, detail)
}

fn dependency(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::Corrupt,
        ReasonCode::RandomAccessDependencyInvalid,
        detail,
    )
}

fn access_policy(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::PolicyRefused,
        ReasonCode::RandomAccessPolicyRefused,
        detail,
    )
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{RandomAccessIndexStatus, open_indexed_random};
    use crate::archive::{PackOptions, plan_directory};
    use crate::eam::LogicalPath;
    use crate::ecf::{WriteOptions, encode};
    use crate::random_access::{MemoryRandomReadSource, RandomAccessPolicy};

    struct TestDir(PathBuf);

    impl TestDir {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "entrybound-random-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            std::fs::create_dir(&path).unwrap();
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn encoded_fixture(include_index: bool) -> (TestDir, Vec<u8>) {
        let temp = TestDir::new();
        std::fs::write(temp.path().join("alpha.txt"), b"random access alpha").unwrap();
        std::fs::write(temp.path().join("unrelated.txt"), vec![7_u8; 128 * 1024]).unwrap();
        let archive = plan_directory(temp.path(), PackOptions::default()).unwrap();
        let encoded = encode(&archive, WriteOptions { include_index }).unwrap();
        (temp, encoded.bytes)
    }

    #[test]
    fn memory_random_access_reads_one_verified_entry() {
        let (_temp, bytes) = encoded_fixture(true);
        let source = MemoryRandomReadSource::new(bytes);
        let mut archive = open_indexed_random(source, RandomAccessPolicy::default()).unwrap();
        let path = LogicalPath::from_utf8(["alpha.txt"]).unwrap();
        let read = archive.read_entry(&path).unwrap();
        assert_eq!(&*read.bytes, b"random access alpha");
        assert!(!read.report.whole_archive_verified);
        assert_eq!(
            read.report.index_status,
            RandomAccessIndexStatus::PresentValid
        );
        assert!(read.report.bytes_fetched < archive.metadata().source_length);
    }

    #[test]
    fn missing_index_rebuilds_from_headers_only() {
        let (_temp, bytes) = encoded_fixture(false);
        let source = MemoryRandomReadSource::new(bytes);
        let mut archive = open_indexed_random(source, RandomAccessPolicy::default()).unwrap();
        let path = LogicalPath::from_utf8(["alpha.txt"]).unwrap();
        let read = archive.read_entry(&path).unwrap();
        assert_eq!(
            read.report.index_status,
            RandomAccessIndexStatus::RebuiltAbsent
        );
        assert_eq!(&*read.bytes, b"random access alpha");
    }

    #[test]
    fn corrupt_index_is_rebuilt_without_trusting_its_section_digest() {
        let (_temp, mut bytes) = encoded_fixture(true);
        let probe = open_indexed_random(
            MemoryRandomReadSource::new(bytes.clone()),
            RandomAccessPolicy::default(),
        )
        .unwrap();
        let index = super::find_kind(&probe.by_kind, crate::ecf::SectionKind::Index).unwrap();
        bytes[usize::try_from(index.payload_offset).unwrap()] ^= 0x01;
        let mut archive = open_indexed_random(
            MemoryRandomReadSource::new(bytes),
            RandomAccessPolicy::default(),
        )
        .unwrap();
        let path = LogicalPath::from_utf8(["alpha.txt"]).unwrap();
        let read = archive.read_entry(&path).unwrap();
        assert_eq!(
            read.report.index_status,
            RandomAccessIndexStatus::RebuiltInvalid
        );
        assert_eq!(&*read.bytes, b"random access alpha");
    }

    #[test]
    fn unread_corrupt_chunk_does_not_create_a_whole_archive_claim() {
        let (temp, bytes) = encoded_fixture(true);
        let planned = plan_directory(temp.path(), PackOptions::default()).unwrap();
        let unrelated = planned
            .entry_set
            .entries()
            .iter()
            .find(|entry| entry.path().to_string() == "unrelated.txt")
            .and_then(|entry| match entry.data() {
                crate::eam::EntryData::File {
                    content: crate::eam::ContentRef::Internal(content),
                } => planned.content_store.objects.get(content),
                crate::eam::EntryData::Directory
                | crate::eam::EntryData::Symlink { .. }
                | crate::eam::EntryData::ReparsePoint { .. } => None,
            })
            .unwrap()
            .chunks[0]
            .chunk_id;
        let probe = open_indexed_random(
            MemoryRandomReadSource::new(bytes.clone()),
            RandomAccessPolicy::default(),
        )
        .unwrap();
        let locator = probe.index[&unrelated];
        let mut corrupted = bytes;
        let payload =
            usize::try_from(locator.offset + super::chunk_frame_header_len(probe.extended))
                .unwrap();
        corrupted[payload] ^= 0x80;
        let mut archive = open_indexed_random(
            MemoryRandomReadSource::new(corrupted),
            RandomAccessPolicy::default(),
        )
        .unwrap();
        let read = archive
            .read_entry(&LogicalPath::from_utf8(["alpha.txt"]).unwrap())
            .unwrap();
        assert_eq!(&*read.bytes, b"random access alpha");
        assert!(!read.report.whole_archive_verified);
        assert_eq!(
            read.report.pci,
            super::IdentityVerificationStatus::NotComputed
        );
    }

    #[test]
    fn coalescing_is_only_an_access_optimization() {
        let (_temp, bytes) = encoded_fixture(true);
        let path = LogicalPath::from_utf8(["alpha.txt"]).unwrap();
        let mut coalesced = open_indexed_random(
            MemoryRandomReadSource::new(bytes.clone()),
            RandomAccessPolicy::default(),
        )
        .unwrap();
        let mut uncoalesced = open_indexed_random(
            MemoryRandomReadSource::new(bytes),
            RandomAccessPolicy {
                coalesce_gap_bytes: 0,
                ..RandomAccessPolicy::default()
            },
        )
        .unwrap();
        assert_eq!(
            coalesced.read_entry(&path).unwrap().bytes,
            uncoalesced.read_entry(&path).unwrap().bytes
        );
    }
}
