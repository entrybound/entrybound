//! Authenticated, range-backed access to crypto-v1 INDEXED archives.

use std::collections::{BTreeMap, BTreeSet};

use sha2::{Digest as _, Sha256};
use subtle::ConstantTimeEq as _;

use super::{
    BoundaryMode, EncryptedOpenOptions, KeyHierarchy, PaddingMode, aead_open,
    public_crypto_context, wire,
};
use crate::canonical::decode_record;
use crate::codec::{PlanMode, aggregate_archive_decode_requirements, plan_mode, validate_plans};
use crate::diagnostics::{Diagnostic, OutcomeClass, ReasonCode, Result};
use crate::eam::{
    ArchiveDescriptor, ArchiveRole, Chunk, ChunkGroup, ContentObject, ContentRef, Dictionary,
    Digest, DigestAlgorithm, EntryData, EntrySet, FeatureSet, IdentityProfile, Layout,
    ReconstructionData, ReconstructionRegion, ResourceBudget, TransformPlan,
};
use crate::ecf::container::{
    ChunkFrameHeader, chunk_frame_header_len, decode_frame_payload, decode_preamble,
    enforce_chunk_bounds, has_codec_transform_feature, has_cross_file_feature,
    has_reconstructive_feature, has_whole_object_feature, parse_chunk_frame_header,
    physical_prefix_from_slices, reconstruct_region_members,
};
use crate::ecf::records::{
    decode_chunk_groups, decode_descriptor, decode_dictionaries, decode_fidelity, decode_manifest,
    decode_reconstruction_regions, decode_reconstruction_section, decode_transform_plans,
    decode_transform_plans_v2, decode_transform_plans_v3,
};
use crate::ecf::{
    IdentityVerificationStatus, RandomAccessIndexStatus, RandomAccessMetadata, RandomAccessRead,
    RandomAccessSection, RandomAccessVerificationReport,
};
use crate::identity::{
    chunk_root_from_leaves, sha256_exact, verify_metadata_aux, verify_metadata_lai,
};
use crate::random_access::{
    AccessPurpose, AccessTraceEntry, RandomAccessPolicy, RandomReadSource, RangeSession,
    SourceRevision,
};

const ENCRYPTED_FOOTER_LEN: u64 = 192;
const SECTION_HEADER_LEN: u64 = 64;
const SEGMENT_HEADER_LEN: u64 = 64;
const PROTECTED_HEADER_LEN: u64 = 32;
const SEGMENT_CONTROL: u8 = 1;
const SEGMENT_PAYLOAD: u8 = 2;
const RECORD_DATA: u8 = 1;
const RECORD_END: u8 = 2;

#[derive(Clone, Copy)]
struct Footer {
    total_len: u64,
    envelope_offset: u64,
    envelope_len: u64,
    segments_offset: u64,
    segments_len: u64,
    terminal_offset: u64,
    archive_final_offset: u64,
    preamble_digest: [u8; 32],
    public_context_digest: [u8; 32],
}

#[derive(Clone)]
struct SegmentLocator {
    offset: u64,
    extent: u64,
    class: u8,
    salt: [u8; 16],
    count: u32,
    header: [u8; 64],
}

struct DecryptedObject {
    bytes: Vec<u8>,
    object_id: [u8; 32],
}

#[derive(Clone, Copy)]
struct EncryptedChunkLocator {
    segment_ordinal: u64,
    fragment_count: u32,
}

#[derive(Default)]
struct ControlObjects {
    descriptor: Option<Vec<u8>>,
    descriptor_id: Option<[u8; 32]>,
    plans: Option<Vec<Vec<u8>>>,
    groups: Option<Vec<Vec<u8>>>,
    manifest: Option<Vec<Vec<u8>>>,
    manifest_id: Option<[u8; 32]>,
    fidelity: Option<Vec<u8>>,
    index: Option<Vec<Vec<u8>>>,
    index_invalid: bool,
    final_value: Option<ArchiveFinal>,
}

struct SupportObjects {
    dictionaries: BTreeMap<Digest, Dictionary>,
    reconstruction_data: BTreeMap<Digest, ReconstructionData>,
    regions: BTreeMap<Digest, ReconstructionRegion>,
}

#[derive(Clone, Copy)]
struct ArchiveFinal {
    segment_count: u64,
    entry_count: u64,
    total_logical: u64,
    chunk_count: u64,
    lai: [u8; 32],
    pcr: [u8; 32],
    aux: [u8; 32],
    recipient_set: [u8; 32],
    footer_core: [u8; 32],
    descriptor_id: [u8; 32],
    manifest_id: [u8; 32],
}

/// Public, range-backed inspection of encrypted framing. This does not unlock
/// or authenticate private archive semantics.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EncryptedRandomPublicInspection {
    pub public: super::PublicCryptoInspection,
    pub source_revision: SourceRevision,
    pub required_features: u64,
    pub bytes_fetched: u64,
    pub range_request_count: u64,
    pub access_trace: Box<[AccessTraceEntry]>,
    pub whole_archive_verified: bool,
}

/// An unlocked encrypted INDEXED archive whose unrelated PAYLOAD segments have
/// not been read.
pub struct EncryptedRandomAccessArchive {
    session: RangeSession,
    metadata: RandomAccessMetadata,
    envelope: wire::CryptoEnvelope,
    keys: KeyHierarchy,
    padding: PaddingMode,
    crypto_policy: super::CryptoPolicy,
    segments: Vec<SegmentLocator>,
    index: BTreeMap<Digest, EncryptedChunkLocator>,
    index_status: RandomAccessIndexStatus,
    plans: Box<[TransformPlan]>,
    groups: BTreeMap<Digest, ChunkGroup>,
    support: Option<SupportObjects>,
    chunk_frames: BTreeMap<Digest, Vec<u8>>,
    extended: bool,
    whole_object: bool,
}

/// Opens encrypted INDEXED metadata through authenticated CONTROL records.
pub fn open_indexed_random_encrypted(
    source: impl RandomReadSource + 'static,
    policy: RandomAccessPolicy,
    options: EncryptedOpenOptions<'_>,
) -> Result<EncryptedRandomAccessArchive> {
    EncryptedRandomAccessArchive::open(Box::new(source), policy, options)
}

/// Inspects only public crypto-v1 framing over bounded byte ranges.
pub fn inspect_indexed_random_encrypted_public(
    source: impl RandomReadSource + 'static,
    policy: RandomAccessPolicy,
    crypto_limits: super::CryptoPolicy,
) -> Result<EncryptedRandomPublicInspection> {
    let mut session = RangeSession::new(Box::new(source), policy)?;
    if session.policy().max_section_count < 2 {
        return Err(access_policy(
            "encrypted INDEXED framing requires two physical sections",
        ));
    }
    if session.len() < crate::ecf::PREAMBLE_LEN + ENCRYPTED_FOOTER_LEN {
        return Err(truncated("encrypted INDEXED footer is missing"));
    }
    let footer_offset = session.len() - ENCRYPTED_FOOTER_LEN;
    let footer_bytes = session.read(footer_offset, ENCRYPTED_FOOTER_LEN, AccessPurpose::Footer)?;
    let footer = parse_footer(&footer_bytes, session.len())?;
    let preamble_bytes = session.read(0, crate::ecf::PREAMBLE_LEN, AccessPurpose::Preamble)?;
    let preamble = decode_preamble(&preamble_bytes)?;
    if preamble.layout != Layout::Indexed
        || preamble.features.incompat & super::FEATURE_ENCRYPTED_INDEXED_V1 == 0
        || preamble.features.incompat & super::FEATURE_PAYLOAD_SUITE_V1 == 0
    {
        return Err(Diagnostic::new(
            OutcomeClass::Unsupported,
            ReasonCode::RandomAccessNotIndexed,
            "source is not a crypto-v1 INDEXED archive",
        ));
    }
    if footer.total_len != session.len()
        || footer.preamble_digest != *sha256_exact(&preamble_bytes).as_bytes()
    {
        return Err(crypto_integrity(
            ReasonCode::FooterBindingMismatch,
            "encrypted footer length/preamble binding mismatch",
        ));
    }
    if footer.envelope_offset != crate::ecf::PREAMBLE_LEN
        || footer.envelope_offset.checked_add(footer.envelope_len) != Some(footer.segments_offset)
        || footer.segments_offset.checked_add(footer.segments_len) != Some(footer_offset)
    {
        return Err(segment_invalid(
            "encrypted section extents are not canonical",
        ));
    }
    if footer.envelope_len > crypto_limits.max_envelope_bytes {
        return Err(crypto_policy("public CryptoEnvelope exceeds caller policy"));
    }
    let envelope = read_envelope(&mut session, footer.envelope_offset, footer.envelope_len)?;
    if envelope.stanzas.len() as u32 > crypto_limits.max_stanzas {
        return Err(crypto_policy("public CryptoEnvelope exceeds caller policy"));
    }
    super::container::validate_envelope_policy(&envelope, preamble.features.incompat)?;
    let padding = PaddingMode::try_from(envelope.padding_mode)?;
    let boundary = BoundaryMode::try_from(envelope.boundary_mode)?;
    let context = public_crypto_context(
        &envelope.archive_id,
        preamble.features.incompat,
        padding,
        boundary,
    )?;
    if !constant_time_eq(&Sha256::digest(&context), &footer.public_context_digest) {
        return Err(crypto_integrity(
            ReasonCode::FooterBindingMismatch,
            "encrypted public-context/footer binding mismatch",
        ));
    }
    let options = EncryptedOpenOptions {
        unlock: None,
        crypto_policy: crypto_limits,
        resource_policy: session.policy().resource_policy,
        decode_policy: session.policy().decode_policy,
    };
    let segments = walk_segments(&mut session, &footer, options)?;
    session.check_stable()?;
    Ok(EncryptedRandomPublicInspection {
        public: super::container::public_inspection(
            &envelope,
            session.len(),
            Some(u64::try_from(segments.len()).unwrap_or(u64::MAX)),
        )?,
        source_revision: session.initial_revision().clone(),
        required_features: preamble.features.incompat,
        bytes_fetched: session.bytes_fetched(),
        range_request_count: session.range_requests(),
        access_trace: session.trace().to_vec().into_boxed_slice(),
        whole_archive_verified: false,
    })
}

impl EncryptedRandomAccessArchive {
    fn open(
        source: Box<dyn RandomReadSource>,
        policy: RandomAccessPolicy,
        options: EncryptedOpenOptions<'_>,
    ) -> Result<Self> {
        let mut session = RangeSession::new(source, policy)?;
        if session.policy().max_section_count < 2 {
            return Err(access_policy(
                "encrypted INDEXED framing requires two physical sections",
            ));
        }
        if session.len() < crate::ecf::PREAMBLE_LEN + ENCRYPTED_FOOTER_LEN {
            return Err(truncated("encrypted INDEXED footer is missing"));
        }
        let footer_offset = session.len() - ENCRYPTED_FOOTER_LEN;
        let footer_bytes =
            session.read(footer_offset, ENCRYPTED_FOOTER_LEN, AccessPurpose::Footer)?;
        let footer = parse_footer(&footer_bytes, session.len())?;
        let preamble_bytes = session.read(0, crate::ecf::PREAMBLE_LEN, AccessPurpose::Preamble)?;
        let preamble = decode_preamble(&preamble_bytes)?;
        if preamble.layout != Layout::Indexed
            || preamble.features.incompat & super::FEATURE_ENCRYPTED_INDEXED_V1 == 0
            || preamble.features.incompat & super::FEATURE_PAYLOAD_SUITE_V1 == 0
        {
            return Err(Diagnostic::new(
                OutcomeClass::Unsupported,
                ReasonCode::RandomAccessNotIndexed,
                "source is not a crypto-v1 INDEXED archive",
            ));
        }
        if footer.total_len != session.len()
            || footer.preamble_digest != *sha256_exact(&preamble_bytes).as_bytes()
        {
            return Err(crypto_integrity(
                ReasonCode::FooterBindingMismatch,
                "encrypted footer length/preamble binding mismatch",
            ));
        }
        if footer.envelope_offset != crate::ecf::PREAMBLE_LEN
            || footer.envelope_offset.checked_add(footer.envelope_len)
                != Some(footer.segments_offset)
            || footer.segments_offset.checked_add(footer.segments_len) != Some(footer_offset)
        {
            return Err(segment_invalid(
                "encrypted section extents are not canonical",
            ));
        }
        if footer.envelope_len > options.crypto_policy.max_envelope_bytes {
            return Err(crypto_policy("CryptoEnvelope exceeds caller policy"));
        }
        let envelope = read_envelope(&mut session, footer.envelope_offset, footer.envelope_len)?;
        if envelope.stanzas.len() as u32 > options.crypto_policy.max_stanzas {
            return Err(crypto_policy(
                "recipient stanza count exceeds caller policy",
            ));
        }
        for stanza in &envelope.stanzas {
            if u64::try_from(stanza.encode()?.len()).unwrap_or(u64::MAX)
                > options.crypto_policy.max_stanza_bytes
            {
                return Err(crypto_policy("recipient stanza exceeds caller policy"));
            }
        }
        super::container::validate_envelope_policy(&envelope, preamble.features.incompat)?;
        let padding = PaddingMode::try_from(envelope.padding_mode)?;
        let boundary = BoundaryMode::try_from(envelope.boundary_mode)?;
        let context = public_crypto_context(
            &envelope.archive_id,
            preamble.features.incompat,
            padding,
            boundary,
        )?;
        if !constant_time_eq(&Sha256::digest(&context), &footer.public_context_digest) {
            return Err(crypto_integrity(
                ReasonCode::FooterBindingMismatch,
                "public crypto context digest mismatch",
            ));
        }
        let unlock = options.unlock.ok_or_else(super::no_recipient)?;
        let (afk, keys) = super::container::unlock_envelope(
            &envelope,
            preamble.features.incompat,
            unlock,
            options.crypto_policy,
        )?;
        drop(afk);
        let segments = walk_segments(&mut session, &footer, options)?;
        let mut controls = ControlObjects::default();
        let mut control_private_bytes = 0_u64;
        for (ordinal, locator) in segments.iter().enumerate() {
            if locator.class != SEGMENT_CONTROL {
                continue;
            }
            let object = decrypt_segment(
                &mut session,
                locator,
                u64::try_from(ordinal).unwrap_or(u64::MAX),
                &envelope,
                &keys,
                padding,
                options,
                AccessPurpose::EncryptedControl,
            )?;
            control_private_bytes = control_private_bytes
                .checked_add(u64::try_from(object.bytes.len()).unwrap_or(u64::MAX))
                .ok_or_else(|| crypto_policy("CONTROL private byte total overflow"))?;
            if control_private_bytes > options.crypto_policy.max_working_memory_bytes {
                return Err(crypto_policy(
                    "aggregate CONTROL private bytes exceed caller policy",
                ));
            }
            dispatch_control(&mut controls, object, preamble.features.incompat)?;
            if ordinal == 0 {
                let descriptor = controls
                    .descriptor
                    .as_deref()
                    .ok_or_else(|| private_invalid("Descriptor is not the first private object"))?;
                let body = decode_descriptor(descriptor)?;
                let corrected = preamble.features.incompat
                    & super::FEATURE_PRIVATE_RESOURCE_DECLARATION_V1
                    != 0;
                if corrected != body.declarations.is_some() {
                    return Err(private_invalid(
                        "Descriptor version and private-resource feature disagree",
                    ));
                }
                if let Some(declarations) = body.declarations {
                    crate::ecf::enforce_caller_policy(
                        declarations.budget,
                        options.resource_policy,
                    )?;
                    crate::ecf::enforce_decode_policy(declarations.decode, options.decode_policy)?;
                }
            }
        }
        let final_value = controls
            .final_value
            .ok_or_else(|| truncated("authenticated ArchiveFinal is missing"))?;
        validate_terminal_bindings(
            &controls,
            &final_value,
            &footer,
            &envelope,
            preamble.features.incompat,
            &context,
            &segments,
        )?;
        let descriptor_bytes = controls
            .descriptor
            .as_deref()
            .ok_or_else(|| private_invalid("encrypted Descriptor is missing"))?;
        let descriptor_body = decode_descriptor(descriptor_bytes)?;
        let manifest_items = controls
            .manifest
            .as_deref()
            .ok_or_else(|| private_invalid("encrypted Manifest is missing"))?;
        let manifest_bytes = concatenate(manifest_items);
        let (entries, content_objects) = decode_manifest(&manifest_bytes)?;
        let fidelity = decode_fidelity(
            controls
                .fidelity
                .as_deref()
                .ok_or_else(|| private_invalid("encrypted Fidelity is missing"))?,
        )?;
        verify_metadata_lai(
            &entries,
            final_value.total_logical,
            Digest::from_bytes(final_value.lai),
        )?;
        verify_metadata_aux(&entries, &fidelity, Digest::from_bytes(final_value.aux))?;
        if final_value.entry_count != u64::try_from(entries.len()).unwrap_or(u64::MAX)
            || descriptor_body.lai != Digest::from_bytes(final_value.lai)
            || descriptor_body.pcr != Digest::from_bytes(final_value.pcr)
            || descriptor_body.aux != Digest::from_bytes(final_value.aux)
        {
            return Err(segment_invalid(
                "ArchiveFinal counts/identities disagree with authenticated metadata",
            ));
        }
        verify_manifest_references(&entries, &content_objects)?;
        let ordinary_features = FeatureSet {
            incompat: preamble.features.incompat & !super::CRYPTO_FEATURES,
            read_only_compat: preamble.features.read_only_compat,
            compat: preamble.features.compat,
        };
        let extended = has_cross_file_feature(ordinary_features);
        let reconstructive = has_reconstructive_feature(ordinary_features);
        let whole_object = has_whole_object_feature(ordinary_features);
        let plan_items = controls
            .plans
            .as_deref()
            .ok_or_else(|| private_invalid("encrypted TransformPlans are missing"))?;
        let plan_bytes = concatenate(plan_items);
        let plans = if whole_object {
            decode_transform_plans_v3(&plan_bytes)?
        } else if reconstructive {
            decode_transform_plans_v2(&plan_bytes)?
        } else {
            decode_transform_plans(&plan_bytes, has_codec_transform_feature(ordinary_features))?
        };
        validate_plans(&plans)?;
        let groups = controls.groups.as_deref().map_or_else(
            || Ok(BTreeMap::new()),
            |items| decode_chunk_groups(&concatenate(items)),
        )?;
        let (budget_declared, budget, decode) = descriptor_body.declarations.map_or(
            (
                false,
                ResourceBudget::default(),
                crate::eam::DecodeRequirements::default(),
            ),
            |value| (true, value.budget, value.decode),
        );
        let descriptor = ArchiveDescriptor {
            format_major: preamble.version.major,
            format_minor: preamble.version.minor,
            format_namespace: descriptor_body.namespace,
            features: ordinary_features,
            layout: Layout::Indexed,
            role: ArchiveRole::Complete,
            budget_declared,
            stream_dedup_window: 0,
            budget,
            decode,
            identity_profile: IdentityProfile::IdentityV1,
            digest_algorithm: DigestAlgorithm::Sha256,
            planner_id: descriptor_body.planner_id,
            chunker_id: descriptor_body.chunker_id,
            lai: descriptor_body.lai,
            pcr: descriptor_body.pcr,
            aux: descriptor_body.aux,
            pci: None,
        };
        let (index, index_status) = if controls.index_invalid {
            (BTreeMap::new(), RandomAccessIndexStatus::RebuiltInvalid)
        } else if let Some(items) = controls.index {
            match decode_encrypted_index(&items, &segments, final_value.chunk_count) {
                Ok(value) => (value, RandomAccessIndexStatus::PresentValid),
                Err(_) => (BTreeMap::new(), RandomAccessIndexStatus::RebuiltInvalid),
            }
        } else {
            (BTreeMap::new(), RandomAccessIndexStatus::RebuiltAbsent)
        };
        let metadata = RandomAccessMetadata {
            descriptor,
            entries,
            content_objects,
            source_length: session.len(),
            source_revision: session.initial_revision().clone(),
            section_count: 2,
            section_directory: vec![
                RandomAccessSection {
                    kind: "CryptoEnvelope".to_owned(),
                    offset: footer.envelope_offset,
                    payload_length: footer.envelope_len - SECTION_HEADER_LEN,
                },
                RandomAccessSection {
                    kind: "EncryptedSegments".to_owned(),
                    offset: footer.segments_offset,
                    payload_length: footer.segments_len - SECTION_HEADER_LEN,
                },
            ]
            .into_boxed_slice(),
            encrypted_segment_count: Some(u64::try_from(segments.len()).unwrap_or(u64::MAX)),
            encrypted: true,
        };
        session.check_stable()?;
        Ok(Self {
            session,
            metadata,
            envelope,
            keys,
            padding,
            crypto_policy: options.crypto_policy,
            segments,
            index,
            index_status,
            plans,
            groups,
            support: None,
            chunk_frames: BTreeMap::new(),
            extended,
            whole_object,
        })
    }

    #[must_use]
    pub fn metadata(&self) -> &RandomAccessMetadata {
        &self.metadata
    }

    pub fn metadata_report(&self) -> Result<RandomAccessVerificationReport> {
        self.session.check_stable()?;
        Ok(self.report(None, false, 0, 0, false, false, false))
    }

    pub fn read_entry(&mut self, path: &crate::eam::LogicalPath) -> Result<RandomAccessRead> {
        self.load_support_and_index_fallback()?;
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
                content: ContentRef::Internal(value),
            } => *value,
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
            .ok_or_else(|| dependency("Entry references unknown ContentObject"))?;
        let requested = object
            .chunks
            .iter()
            .map(|value| value.chunk_id)
            .collect::<BTreeSet<_>>();
        if u64::try_from(requested.len()).unwrap_or(u64::MAX)
            > self.session.policy().max_dependency_chunks
        {
            return Err(access_policy(
                "requested Chunk closure exceeds caller policy",
            ));
        }
        let mut decoded = BTreeMap::<Digest, Chunk>::new();
        let mut dependencies = 0_u64;
        let mut reconstruction_verified = false;
        for chunk_id in requested {
            let header = self.chunk_header(chunk_id)?;
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
            let plan = self
                .plans
                .iter()
                .find(|plan| plan.plan_id == header.plan_ref)
                .ok_or_else(|| dependency(format!("unknown TransformPlan {}", header.plan_ref)))?;
            reconstruction_verified |= plan
                .transforms
                .iter()
                .any(|step| step.reconstruction_ref.is_some());
            let predecessors = self.group_prerequisites(chunk_id, &header)?;
            dependencies = dependencies
                .checked_add(u64::try_from(predecessors.len()).unwrap_or(u64::MAX))
                .ok_or_else(|| access_policy("dependency count overflow"))?;
            if dependencies > self.session.policy().max_dependency_chunks {
                return Err(access_policy("lookback closure exceeds caller policy"));
            }
            for predecessor in predecessors {
                self.decode_chunk(predecessor, &mut decoded)?;
            }
            self.decode_chunk(chunk_id, &mut decoded)?;
        }
        if self.whole_object {
            let owned_plans = self.plans.to_vec();
            let regions = self
                .support
                .as_ref()
                .expect("support loaded")
                .regions
                .values()
                .filter(|value| value.content_object == content_id)
                .cloned()
                .collect::<Vec<_>>();
            let requested_region_bytes = regions.iter().try_fold(0_u64, |total, region| {
                total
                    .checked_add(region.logical_bytes)
                    .ok_or_else(|| access_policy("region logical byte total overflow"))
            })?;
            if requested_region_bytes > self.session.policy().max_decoded_logical_bytes {
                return Err(access_policy(
                    "aggregate region access exceeds caller decoded-byte policy",
                ));
            }
            for region in &regions {
                reconstruction_verified = true;
                if region.access.logical_bytes != region.logical_bytes
                    || region.access.logical_chunks != region.chunk_count
                    || region.access.worst_reconstructed_bytes != region.logical_bytes
                {
                    return Err(crypto_integrity(
                        ReasonCode::InvalidRegionAccess,
                        region.region_id.to_string(),
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
                let members = object
                    .chunks
                    .get(start..end)
                    .ok_or_else(|| dependency("region exceeds ContentObject"))?;
                for member in members {
                    if let std::collections::btree_map::Entry::Vacant(entry) =
                        decoded.entry(member.chunk_id)
                    {
                        let header = self.chunk_header(member.chunk_id)?;
                        entry.insert(Chunk {
                            chunk_id: member.chunk_id,
                            logical_len: header.logical_len,
                            plan_ref: header.plan_ref,
                            group_ref: header.group_ref,
                            plaintext: Box::new([]),
                        });
                    }
                }
                let lengths = members
                    .iter()
                    .map(|member| Ok(decoded[&member.chunk_id].logical_len))
                    .collect::<Result<Vec<_>>>()?;
                let plans = owned_plans
                    .iter()
                    .map(|plan| (plan.plan_id, plan))
                    .collect::<BTreeMap<_, _>>();
                for (chunk_id, bytes) in
                    reconstruct_region_members(region, &object, &plans, &lengths)?
                {
                    decoded
                        .get_mut(&chunk_id)
                        .expect("declared member")
                        .plaintext = bytes.into_boxed_slice();
                }
            }
        }
        let mut output = Vec::new();
        let mut leaves = Vec::new();
        for reference in &object.chunks {
            let chunk = decoded
                .get(&reference.chunk_id)
                .ok_or_else(|| dependency("requested Chunk was not decoded"))?;
            if sha256_exact(&chunk.plaintext) != chunk.chunk_id {
                return Err(crypto_integrity(
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
            return Err(crypto_integrity(
                ReasonCode::ChunkRootMismatch,
                object.logical_digest.to_string(),
            ));
        }
        if sha256_exact(&output) != object.logical_digest {
            return Err(crypto_integrity(
                ReasonCode::ContentDigestMismatch,
                object.logical_digest.to_string(),
            ));
        }
        if let Some(map) = sparse_map {
            map.validate_plaintext(&output)?;
        }
        self.session.check_stable()?;
        Ok(RandomAccessRead {
            bytes: output.into_boxed_slice(),
            report: self.report(
                Some(path.to_string()),
                true,
                u64::try_from(object.chunks.len()).unwrap_or(u64::MAX),
                dependencies,
                self.extended,
                self.extended,
                reconstruction_verified,
            ),
        })
    }

    fn load_support_and_index_fallback(&mut self) -> Result<()> {
        if self.support.is_some() {
            return Ok(());
        }
        let known_chunk_segments = self
            .index
            .values()
            .map(|value| value.segment_ordinal)
            .collect::<BTreeSet<_>>();
        let mut dictionary_items = None;
        let mut reconstruction_items = None;
        let mut region_items = None;
        let mut discovered = BTreeMap::new();
        let mut retained_private_bytes = 0_u64;
        for ordinal in 0..self.segments.len() {
            let ordinal_u64 = u64::try_from(ordinal).unwrap_or(u64::MAX);
            if self.segments[ordinal].class != SEGMENT_PAYLOAD {
                continue;
            }
            if self.index_status == RandomAccessIndexStatus::PresentValid
                && known_chunk_segments.contains(&ordinal_u64)
            {
                continue;
            }
            let locator = self.segments[ordinal].clone();
            let resource_policy = self.session.policy().resource_policy;
            let decode_policy = self.session.policy().decode_policy;
            let object = decrypt_segment(
                &mut self.session,
                &locator,
                ordinal_u64,
                &self.envelope,
                &self.keys,
                self.padding,
                EncryptedOpenOptions {
                    unlock: None,
                    crypto_policy: self.crypto_policy,
                    resource_policy,
                    decode_policy,
                },
                AccessPurpose::EncryptedPayload,
            )?;
            retained_private_bytes = retained_private_bytes
                .checked_add(u64::try_from(object.bytes.len()).unwrap_or(u64::MAX))
                .ok_or_else(|| crypto_policy("PAYLOAD private byte total overflow"))?;
            if retained_private_bytes > self.crypto_policy.max_working_memory_bytes {
                return Err(crypto_policy(
                    "retained PAYLOAD objects exceed caller crypto memory policy",
                ));
            }
            let (kind, payload) = wire::decode_private_object(&object.bytes)?;
            match kind {
                wire::PRIVATE_OBJECT_CHUNK => {
                    if u64::try_from(discovered.len()).unwrap_or(u64::MAX)
                        >= self.session.policy().max_chunk_frames_scanned
                    {
                        return Err(access_policy(
                            "encrypted Chunk-object scan exceeds caller policy",
                        ));
                    }
                    let header_len = usize::try_from(chunk_frame_header_len(self.extended))
                        .map_err(|_| access_policy("Chunk header length exceeds usize"))?;
                    let header = parse_chunk_frame_header(
                        payload
                            .get(..header_len)
                            .ok_or_else(|| truncated("encrypted Chunk frame is truncated"))?,
                        self.extended,
                        self.whole_object,
                    )?;
                    discovered.insert(
                        header.chunk_id,
                        EncryptedChunkLocator {
                            segment_ordinal: ordinal_u64,
                            fragment_count: locator.count,
                        },
                    );
                    self.chunk_frames.insert(header.chunk_id, payload.to_vec());
                    if self.index_status == RandomAccessIndexStatus::PresentValid {
                        self.index_status = RandomAccessIndexStatus::RebuiltInvalid;
                    }
                }
                wire::PRIVATE_OBJECT_SEQUENCE => {
                    let (collection, items) = wire::decode_sequence_container(payload)?;
                    match collection {
                        wire::COLLECTION_DICTIONARIES => {
                            set_once(&mut dictionary_items, items, "Dictionaries")?
                        }
                        wire::COLLECTION_RECONSTRUCTION_DATA => {
                            set_once(&mut reconstruction_items, items, "ReconstructionData")?
                        }
                        wire::COLLECTION_RECONSTRUCTION_REGIONS => {
                            set_once(&mut region_items, items, "ReconstructionRegions")?
                        }
                        _ => return Err(private_invalid("unexpected PAYLOAD collection")),
                    }
                }
                _ => return Err(private_invalid("unexpected PAYLOAD private object")),
            }
        }
        if self.index_status != RandomAccessIndexStatus::PresentValid {
            self.index = discovered;
            if u64::try_from(self.index.len()).unwrap_or(u64::MAX)
                != self.metadata.descriptor.budget.chunk_count
                && self.metadata.descriptor.budget_declared
            {
                return Err(segment_invalid("rebuilt encrypted Chunk count mismatch"));
            }
        }
        let dictionaries = dictionary_items.map_or_else(
            || Ok(BTreeMap::new()),
            |items| decode_dictionaries(&concatenate(&items)),
        )?;
        let reconstruction_data = reconstruction_items.map_or_else(
            || Ok(BTreeMap::new()),
            |items| decode_reconstruction_section(&concatenate(&items)).map(|value| value.0),
        )?;
        let regions = region_items.map_or_else(
            || Ok(BTreeMap::new()),
            |items| decode_reconstruction_regions(&concatenate(&items)).map(|value| value.0),
        )?;
        let actual_decode =
            aggregate_archive_decode_requirements(&self.plans, &dictionaries, &self.groups)?;
        if self.metadata.descriptor.budget_declared
            && actual_decode != self.metadata.descriptor.decode
        {
            return Err(private_invalid(
                "Descriptor decode declaration disagrees with authenticated dependencies",
            ));
        }
        crate::ecf::enforce_decode_policy(actual_decode, self.session.policy().decode_policy)?;
        self.support = Some(SupportObjects {
            dictionaries,
            reconstruction_data,
            regions,
        });
        Ok(())
    }

    fn chunk_frame(&mut self, chunk_id: Digest) -> Result<Vec<u8>> {
        if let Some(frame) = self.chunk_frames.get(&chunk_id) {
            return Ok(frame.clone());
        }
        let locator = self
            .index
            .get(&chunk_id)
            .copied()
            .ok_or_else(|| dependency(format!("encrypted Index lacks Chunk {chunk_id}")))?;
        let segment_index = usize::try_from(locator.segment_ordinal)
            .map_err(|_| dependency("segment ordinal exceeds usize"))?;
        let segment = self
            .segments
            .get(segment_index)
            .cloned()
            .ok_or_else(|| dependency("encrypted Index references absent segment"))?;
        if segment.class != SEGMENT_PAYLOAD || segment.count != locator.fragment_count {
            self.index_status = RandomAccessIndexStatus::RebuiltInvalid;
            self.support = None;
            self.load_support_and_index_fallback()?;
            return self
                .chunk_frames
                .get(&chunk_id)
                .cloned()
                .ok_or_else(|| dependency(format!("unknown Chunk {chunk_id}")));
        }
        let resource_policy = self.session.policy().resource_policy;
        let decode_policy = self.session.policy().decode_policy;
        let object = decrypt_segment(
            &mut self.session,
            &segment,
            locator.segment_ordinal,
            &self.envelope,
            &self.keys,
            self.padding,
            EncryptedOpenOptions {
                unlock: None,
                crypto_policy: self.crypto_policy,
                resource_policy,
                decode_policy,
            },
            AccessPurpose::EncryptedPayload,
        )?;
        let (kind, payload) = wire::decode_private_object(&object.bytes)?;
        if kind != wire::PRIVATE_OBJECT_CHUNK {
            self.index_status = RandomAccessIndexStatus::RebuiltInvalid;
            self.support = None;
            self.load_support_and_index_fallback()?;
            return self
                .chunk_frames
                .get(&chunk_id)
                .cloned()
                .ok_or_else(|| dependency(format!("unknown Chunk {chunk_id}")));
        }
        let header_len = usize::try_from(chunk_frame_header_len(self.extended))
            .map_err(|_| access_policy("Chunk header exceeds usize"))?;
        let header = parse_chunk_frame_header(
            payload
                .get(..header_len)
                .ok_or_else(|| truncated("encrypted Chunk frame is truncated"))?,
            self.extended,
            self.whole_object,
        )?;
        if header.chunk_id != chunk_id {
            self.index_status = RandomAccessIndexStatus::RebuiltInvalid;
            self.support = None;
            self.load_support_and_index_fallback()?;
            return self
                .chunk_frames
                .get(&chunk_id)
                .cloned()
                .ok_or_else(|| dependency(format!("unknown Chunk {chunk_id}")));
        }
        let frame = payload.to_vec();
        self.chunk_frames.insert(chunk_id, frame.clone());
        Ok(frame)
    }

    fn chunk_header(&mut self, chunk_id: Digest) -> Result<ChunkFrameHeader> {
        let frame = self.chunk_frame(chunk_id)?;
        let header_len = usize::try_from(chunk_frame_header_len(self.extended))
            .map_err(|_| access_policy("Chunk header exceeds usize"))?;
        let header =
            parse_chunk_frame_header(&frame[..header_len], self.extended, self.whole_object)?;
        let budget = if self.metadata.descriptor.budget_declared {
            self.metadata.descriptor.budget
        } else {
            self.session.policy().resource_policy
        };
        enforce_chunk_bounds(&header, budget)?;
        if frame.len()
            != header_len
                .checked_add(
                    usize::try_from(header.stored_len)
                        .map_err(|_| access_policy("Chunk stored length exceeds usize"))?,
                )
                .ok_or_else(|| dependency("Chunk frame extent overflow"))?
        {
            return Err(dependency("encrypted Chunk frame stored length mismatch"));
        }
        Ok(header)
    }

    fn group_prerequisites(
        &mut self,
        chunk_id: Digest,
        header: &ChunkFrameHeader,
    ) -> Result<Vec<Digest>> {
        let plan = self
            .plans
            .iter()
            .find(|plan| plan.plan_id == header.plan_ref)
            .ok_or_else(|| dependency("Chunk references unknown TransformPlan"))?;
        let PlanMode::Prefix { lookback } = plan_mode(plan)? else {
            return Ok(Vec::new());
        };
        let group_id = header
            .group_ref
            .ok_or_else(|| dependency("prefix-coded Chunk lacks group_ref"))?;
        let group = self
            .groups
            .get(&group_id)
            .ok_or_else(|| dependency("prefix-coded Chunk references unknown group"))?
            .clone();
        if group.max_lookback != lookback {
            return Err(dependency("group lookback and plan disagree"));
        }
        let mut physical = self
            .index
            .iter()
            .map(|(digest, locator)| (locator.segment_ordinal, *digest))
            .collect::<Vec<_>>();
        physical.sort();
        let position = physical
            .iter()
            .position(|(_, digest)| *digest == chunk_id)
            .ok_or_else(|| dependency("Chunk has no encrypted physical position"))?;
        let first = position.saturating_sub(usize::try_from(lookback).unwrap_or(usize::MAX));
        let mut predecessors = Vec::new();
        let mut logical_bytes = 0_u64;
        for (_, predecessor) in &physical[first..position] {
            let predecessor_header = self.chunk_header(*predecessor)?;
            if predecessor_header.group_ref == Some(group_id) {
                logical_bytes = logical_bytes
                    .checked_add(predecessor_header.logical_len)
                    .ok_or_else(|| dependency("lookback byte count overflow"))?;
                predecessors.push(*predecessor);
            }
        }
        if logical_bytes > group.max_preceding_bytes {
            return Err(crypto_integrity(
                ReasonCode::AccessCostMismatch,
                group_id.to_string(),
            ));
        }
        Ok(predecessors)
    }

    fn decode_chunk(
        &mut self,
        chunk_id: Digest,
        decoded: &mut BTreeMap<Digest, Chunk>,
    ) -> Result<()> {
        if decoded.contains_key(&chunk_id) {
            return Ok(());
        }
        let frame = self.chunk_frame(chunk_id)?;
        let header_len = usize::try_from(chunk_frame_header_len(self.extended))
            .map_err(|_| access_policy("Chunk header exceeds usize"))?;
        let header =
            parse_chunk_frame_header(&frame[..header_len], self.extended, self.whole_object)?;
        if header.region_owned {
            return Err(dependency(
                "region member has no independent representation",
            ));
        }
        let plan = self
            .plans
            .iter()
            .find(|plan| plan.plan_id == header.plan_ref)
            .ok_or_else(|| dependency("Chunk references unknown TransformPlan"))?
            .clone();
        let prefix = if let PlanMode::Prefix { lookback } = plan_mode(&plan)? {
            let predecessors = self.group_prerequisites(chunk_id, &header)?;
            for predecessor in &predecessors {
                if !decoded.contains_key(predecessor) {
                    self.decode_chunk(*predecessor, decoded)?;
                }
            }
            let slices = predecessors
                .iter()
                .map(|digest| decoded[digest].plaintext.as_ref())
                .collect::<Vec<_>>();
            Some(physical_prefix_from_slices(&slices, lookback)?)
        } else {
            None
        };
        let support = self.support.as_ref().expect("support metadata loaded");
        let plaintext = decode_frame_payload(
            &plan,
            &frame[header_len..],
            header.logical_len,
            &support.dictionaries,
            &support.reconstruction_data,
            prefix.as_deref(),
        )?;
        if sha256_exact(&plaintext) != chunk_id {
            return Err(crypto_integrity(
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
            aux: IdentityVerificationStatus::Verified,
            pcr: IdentityVerificationStatus::DeclaredNotFullyVerified,
            pci: IdentityVerificationStatus::NotComputed,
            whole_archive_verified: false,
            access_trace: self.session.trace().to_vec().into_boxed_slice(),
        }
    }
}

fn read_envelope(
    session: &mut RangeSession,
    offset: u64,
    extent: u64,
) -> Result<wire::CryptoEnvelope> {
    if extent < SECTION_HEADER_LEN || extent > session.policy().max_metadata_bytes {
        return Err(crypto_policy("CryptoEnvelope extent exceeds caller policy"));
    }
    let header = session.read(offset, SECTION_HEADER_LEN, AccessPurpose::SectionHeader)?;
    validate_section_header(&header, 32)?;
    let payload_len = be_u64(&header[16..24])?;
    if SECTION_HEADER_LEN.checked_add(payload_len) != Some(extent) {
        return Err(segment_invalid(
            "CryptoEnvelope footer/header extent mismatch",
        ));
    }
    let payload = session.read(
        offset + SECTION_HEADER_LEN,
        payload_len,
        AccessPurpose::EncryptedControl,
    )?;
    if sha256_exact(&payload).as_bytes() != &header[24..56] {
        return Err(crypto_integrity(
            ReasonCode::SectionDigestMismatch,
            "CryptoEnvelope section digest mismatch",
        ));
    }
    wire::CryptoEnvelope::decode(&payload)
}

fn walk_segments(
    session: &mut RangeSession,
    footer: &Footer,
    options: EncryptedOpenOptions<'_>,
) -> Result<Vec<SegmentLocator>> {
    let section = session.read(
        footer.segments_offset,
        SECTION_HEADER_LEN,
        AccessPurpose::SectionHeader,
    )?;
    validate_section_header(&section, 33)?;
    let payload_len = be_u64(&section[16..24])?;
    if SECTION_HEADER_LEN.checked_add(payload_len) != Some(footer.segments_len) {
        return Err(segment_invalid("ENCRYPTED_SEGMENTS extent mismatch"));
    }
    let end = footer
        .segments_offset
        .checked_add(footer.segments_len)
        .ok_or_else(|| segment_invalid("ENCRYPTED_SEGMENTS extent overflow"))?;
    let mut cursor = footer.segments_offset + SECTION_HEADER_LEN;
    let mut locators = Vec::new();
    let mut salts = BTreeSet::new();
    while cursor < end {
        let ordinal = u64::try_from(locators.len()).unwrap_or(u64::MAX);
        if ordinal >= options.crypto_policy.max_segments
            || ordinal >= session.policy().max_encrypted_segment_header_walk
        {
            return Err(access_policy(
                "encrypted SegmentHeader walk exceeds caller policy",
            ));
        }
        let bytes = session.read(cursor, SEGMENT_HEADER_LEN, AccessPurpose::SectionHeader)?;
        let header: [u8; 64] = bytes
            .try_into()
            .map_err(|_| truncated("SegmentHeader is truncated"))?;
        let (class, salt, count, extent) = parse_segment_header(&header, ordinal)?;
        if count > options.crypto_policy.max_messages_per_segment
            || extent > options.crypto_policy.max_working_memory_bytes
        {
            return Err(crypto_policy(
                "encrypted segment declaration exceeds caller policy",
            ));
        }
        if !salts.insert((class, salt)) {
            return Err(segment_invalid("segment salt is reused in one key domain"));
        }
        let next = cursor
            .checked_add(extent)
            .ok_or_else(|| segment_invalid("segment extent overflow"))?;
        if next > end {
            return Err(truncated("segment extent exceeds ENCRYPTED_SEGMENTS"));
        }
        locators.push(SegmentLocator {
            offset: cursor,
            extent,
            class,
            salt,
            count,
            header,
        });
        cursor = next;
    }
    if cursor != end || locators.is_empty() {
        return Err(truncated("encrypted segment sequence is incomplete"));
    }
    let terminal = locators.last().expect("not empty");
    if terminal.offset != footer.terminal_offset
        || footer.archive_final_offset != terminal.offset + SEGMENT_HEADER_LEN
        || terminal.class != SEGMENT_CONTROL
        || terminal.count != 1
    {
        return Err(segment_invalid(
            "terminal segment/footer locators are invalid",
        ));
    }
    Ok(locators)
}

#[allow(clippy::too_many_arguments)]
fn decrypt_segment(
    session: &mut RangeSession,
    locator: &SegmentLocator,
    ordinal: u64,
    envelope: &wire::CryptoEnvelope,
    keys: &KeyHierarchy,
    padding: PaddingMode,
    options: EncryptedOpenOptions<'_>,
    purpose: AccessPurpose,
) -> Result<DecryptedObject> {
    let bytes = session.read(locator.offset, locator.extent, purpose)?;
    if bytes.get(..64) != Some(locator.header.as_slice()) {
        return Err(segment_invalid(
            "fetched SegmentHeader changed within the session",
        ));
    }
    let key = super::container::segment_key(
        keys,
        &envelope.archive_id,
        locator.class,
        ordinal,
        &locator.salt,
    )?;
    let mut cursor = usize::try_from(SEGMENT_HEADER_LEN).unwrap_or(64);
    let mut exact_data = Vec::new();
    let mut private_total = 0_u64;
    let mut ciphertext_total = 0_u64;
    let mut partial_id = None;
    let mut partial_total = 0_u64;
    let mut partial_count = 0_u32;
    let mut partial_next = 0_u32;
    let mut object = Vec::new();
    for counter in 0..locator.count {
        let (protected, ciphertext, next) = parse_protected(
            &bytes,
            cursor,
            RECORD_DATA,
            ordinal,
            u64::from(counter),
            options,
        )?;
        let private = decrypt_private_record(
            &key.0,
            &envelope.archive_id,
            &locator.header,
            protected,
            ciphertext,
            locator.class,
            padding,
            false,
        )?;
        if u64::try_from(private.len()).unwrap_or(u64::MAX)
            > options.crypto_policy.max_private_record_bytes
        {
            return Err(crypto_policy(
                "protected private record exceeds caller policy",
            ));
        }
        private_total = private_total
            .checked_add(u64::try_from(private.len()).unwrap_or(u64::MAX))
            .ok_or_else(|| segment_invalid("private byte total overflow"))?;
        ciphertext_total = ciphertext_total
            .checked_add(u64::try_from(ciphertext.len()).unwrap_or(u64::MAX))
            .ok_or_else(|| segment_invalid("ciphertext byte total overflow"))?;
        exact_data.extend_from_slice(protected);
        exact_data.extend_from_slice(ciphertext);
        let fragment = wire::decode_private_fragment(&private)?;
        if counter == 0 {
            if fragment.index != 0 || fragment.offset != 0 {
                return Err(private_invalid("first object fragment is not initial"));
            }
            partial_id = Some(fragment.object_id);
            partial_total = fragment.total_len;
            partial_count = fragment.count;
            if fragment.total_len > options.crypto_policy.max_working_memory_bytes {
                return Err(crypto_policy(
                    "encrypted object exceeds caller crypto memory policy",
                ));
            }
            object.reserve(
                usize::try_from(fragment.total_len)
                    .map_err(|_| crypto_policy("encrypted object exceeds usize"))?,
            );
        }
        if Some(fragment.object_id) != partial_id
            || fragment.total_len != partial_total
            || fragment.count != partial_count
            || fragment.index != partial_next
            || fragment.offset != u64::try_from(object.len()).unwrap_or(u64::MAX)
        {
            return Err(private_invalid(
                "private fragments are not contiguous and exact",
            ));
        }
        object.extend_from_slice(&fragment.bytes);
        partial_next += 1;
        cursor = next;
    }
    if partial_next != partial_count
        || u64::try_from(object.len()).unwrap_or(u64::MAX) != partial_total
    {
        return Err(truncated("encrypted object fragments are incomplete"));
    }
    let (end_header, end_ciphertext, next) = parse_protected(
        &bytes,
        cursor,
        RECORD_END,
        ordinal,
        u64::from(locator.count),
        options,
    )?;
    if next != bytes.len() {
        return Err(segment_invalid("bytes follow authenticated segment END"));
    }
    let end_private = decrypt_private_record(
        &key.0,
        &envelope.archive_id,
        &locator.header,
        end_header,
        end_ciphertext,
        locator.class,
        padding,
        true,
    )?;
    if u64::try_from(end_private.len()).unwrap_or(u64::MAX)
        > options.crypto_policy.max_private_record_bytes
    {
        return Err(crypto_policy(
            "segment END private record exceeds caller policy",
        ));
    }
    let data_digest: [u8; 32] = Sha256::digest(wire::t1(
        "entrybound/segment-data/v1",
        &[&locator.header, &data_sequence(locator.count, &exact_data)?],
    )?)
    .into();
    validate_segment_end(
        &end_private,
        ordinal,
        locator.class,
        locator.count,
        private_total,
        ciphertext_total,
        &data_digest,
    )?;
    let object_id = partial_id.ok_or_else(|| truncated("segment has no private object"))?;
    if wire::encrypted_object_id(&object)? != object_id {
        return Err(crypto_integrity(
            ReasonCode::CryptoPrivateObjectInvalid,
            "encrypted object identity mismatch",
        ));
    }
    Ok(DecryptedObject {
        bytes: object,
        object_id,
    })
}

fn dispatch_control(
    controls: &mut ControlObjects,
    object: DecryptedObject,
    features: u64,
) -> Result<()> {
    let (kind, payload) = wire::decode_private_object(&object.bytes)?;
    match kind {
        wire::PRIVATE_OBJECT_RECORD => match wire::record_kind(payload)? {
            1 => {
                set_once(&mut controls.descriptor, payload.to_vec(), "Descriptor")?;
                controls.descriptor_id = Some(object.object_id);
            }
            5 => set_once(&mut controls.fidelity, payload.to_vec(), "Fidelity")?,
            wire::RECORD_ARCHIVE_FINAL => {
                set_once(
                    &mut controls.final_value,
                    decode_archive_final(payload)?,
                    "ArchiveFinal",
                )?;
            }
            _ => return Err(private_invalid("forbidden singleton CONTROL object")),
        },
        wire::PRIVATE_OBJECT_SEQUENCE => {
            let peek_index = payload.len() >= 8
                && &payload[..4] == b"EBCS"
                && u16::from_be_bytes(payload[6..8].try_into().unwrap()) == wire::COLLECTION_INDEX;
            let decoded = wire::decode_sequence_container(payload);
            let (collection, items) = match decoded {
                Ok(value) => value,
                Err(_) if peek_index => {
                    controls.index_invalid = true;
                    return Ok(());
                }
                Err(error) => return Err(error),
            };
            match collection {
                wire::COLLECTION_TRANSFORM_PLANS => {
                    set_once(&mut controls.plans, items, "TransformPlans")?
                }
                wire::COLLECTION_CHUNK_GROUPS => {
                    set_once(&mut controls.groups, items, "ChunkGroups")?
                }
                wire::COLLECTION_MANIFEST => {
                    set_once(&mut controls.manifest, items, "Manifest")?;
                    controls.manifest_id = Some(object.object_id);
                }
                wire::COLLECTION_INDEX => set_once(&mut controls.index, items, "Index")?,
                wire::COLLECTION_RECIPIENT_DIRECTORY | wire::COLLECTION_SIGNATURES => {}
                _ => {
                    return Err(private_invalid(
                        "PAYLOAD collection appeared in CONTROL segment",
                    ));
                }
            }
        }
        _ => return Err(private_invalid("Chunk object appeared in CONTROL segment")),
    }
    if features & super::FEATURE_SIGNATURE_ED25519_V1 == 0 {
        // Signature collection parsing above remains authenticated but is not
        // needed for random file retrieval.
    }
    Ok(())
}

fn validate_terminal_bindings(
    controls: &ControlObjects,
    final_value: &ArchiveFinal,
    footer: &Footer,
    envelope: &wire::CryptoEnvelope,
    features: u64,
    public_context: &[u8],
    segments: &[SegmentLocator],
) -> Result<()> {
    if final_value.segment_count != u64::try_from(segments.len()).unwrap_or(u64::MAX)
        || controls.descriptor_id != Some(final_value.descriptor_id)
        || controls.manifest_id != Some(final_value.manifest_id)
    {
        return Err(segment_invalid(
            "ArchiveFinal object/count binding mismatch",
        ));
    }
    let recipient_sequence = wire::encode_stanza_sequence(&envelope.stanzas)?;
    let recipient_set: [u8; 32] = Sha256::digest(wire::t1(
        "entrybound/recipient-set/v1",
        &[&recipient_sequence],
    )?)
    .into();
    if !constant_time_eq(&recipient_set, &final_value.recipient_set) {
        return Err(crypto_integrity(
            ReasonCode::CryptoEnvelopeAuthFailed,
            "ArchiveFinal recipient-set binding mismatch",
        ));
    }
    let context_digest: [u8; 32] = Sha256::digest(public_context).into();
    let core = wire::t1(
        "entrybound/encrypted-footer-core/v1",
        &[
            &2_u16.to_be_bytes(),
            &footer.total_len.to_be_bytes(),
            &footer.envelope_offset.to_be_bytes(),
            &footer.envelope_len.to_be_bytes(),
            &footer.segments_offset.to_be_bytes(),
            &footer.segments_len.to_be_bytes(),
            &footer.terminal_offset.to_be_bytes(),
            &footer.archive_final_offset.to_be_bytes(),
            &footer.preamble_digest,
            &context_digest,
        ],
    )?;
    if !constant_time_eq(&Sha256::digest(core), &final_value.footer_core) {
        return Err(crypto_integrity(
            ReasonCode::FooterBindingMismatch,
            "authenticated footer-core mismatch",
        ));
    }
    if features & super::FEATURE_PRIVATE_RESOURCE_DECLARATION_V1 == 0 {
        // Accepted only as the explicitly supported pre-correction form.
    }
    Ok(())
}

fn decode_encrypted_index(
    items: &[Vec<u8>],
    segments: &[SegmentLocator],
    expected_count: u64,
) -> Result<BTreeMap<Digest, EncryptedChunkLocator>> {
    let mut output = BTreeMap::new();
    for item in items {
        let (record, consumed) = decode_record(item)?;
        if consumed != item.len() || record.kind != wire::RECORD_ENCRYPTED_INDEX {
            return Err(private_invalid("encrypted Index contains a wrong record"));
        }
        record.expect_tags(&[1, 2, 3, 4], &[])?;
        let digest = Digest::from_bytes(
            record
                .field(1)?
                .as_bytes()?
                .try_into()
                .map_err(|_| private_invalid("encrypted Index digest length"))?,
        );
        let ordinal = record.field(2)?.as_u64()?;
        let first_counter = record.field(3)?.as_u64()?;
        let fragments = record.field(4)?.as_u32()?;
        let segment = segments
            .get(
                usize::try_from(ordinal)
                    .map_err(|_| private_invalid("Index ordinal exceeds usize"))?,
            )
            .ok_or_else(|| private_invalid("Index references absent segment"))?;
        if first_counter != 0
            || fragments == 0
            || segment.class != SEGMENT_PAYLOAD
            || segment.count != fragments
            || output
                .insert(
                    digest,
                    EncryptedChunkLocator {
                        segment_ordinal: ordinal,
                        fragment_count: fragments,
                    },
                )
                .is_some()
        {
            return Err(private_invalid(
                "encrypted Index locator is invalid or duplicate",
            ));
        }
    }
    if u64::try_from(output.len()).unwrap_or(u64::MAX) != expected_count {
        return Err(private_invalid("encrypted Index Chunk count mismatch"));
    }
    Ok(output)
}

fn parse_footer(bytes: &[u8], source_len: u64) -> Result<Footer> {
    if bytes.len() != usize::try_from(ENCRYPTED_FOOTER_LEN).unwrap_or(192)
        || &bytes[..8] != b"\x8eEBF\r\n\x1a\n"
    {
        return Err(truncated("encrypted footer magic is missing"));
    }
    if be_u16(&bytes[8..10])? != 2
        || be_u16(&bytes[10..12])? != 192
        || bytes[12..16].iter().any(|byte| *byte != 0)
        || bytes[168..].iter().any(|byte| *byte != 0)
    {
        return Err(private_invalid("encrypted footer is noncanonical"));
    }
    let total_len = be_u64(&bytes[16..24])?;
    if total_len != source_len {
        return Err(Diagnostic::new(
            if total_len > source_len {
                OutcomeClass::Truncated
            } else {
                OutcomeClass::Corrupt
            },
            ReasonCode::IncorrectTotalLength,
            "encrypted footer total length mismatch",
        ));
    }
    Ok(Footer {
        total_len,
        envelope_offset: be_u64(&bytes[24..32])?,
        envelope_len: be_u64(&bytes[32..40])?,
        segments_offset: be_u64(&bytes[40..48])?,
        segments_len: be_u64(&bytes[48..56])?,
        terminal_offset: be_u64(&bytes[56..64])?,
        archive_final_offset: be_u64(&bytes[64..72])?,
        preamble_digest: bytes[72..104].try_into().unwrap(),
        public_context_digest: bytes[136..168].try_into().unwrap(),
    })
}

fn validate_section_header(bytes: &[u8], kind: u16) -> Result<()> {
    if bytes.len() != 64
        || &bytes[..4] != b"EBS1"
        || be_u16(&bytes[4..6])? != kind
        || be_u16(&bytes[6..8])? != 1
        || bytes[8..16].iter().any(|byte| *byte != 0)
        || bytes[56..64].iter().any(|byte| *byte != 0)
    {
        return Err(segment_invalid("encrypted section header is noncanonical"));
    }
    Ok(())
}

fn parse_segment_header(header: &[u8; 64], ordinal: u64) -> Result<(u8, [u8; 16], u32, u64)> {
    if &header[..4] != b"EBSG"
        || be_u16(&header[4..6])? != 1
        || !matches!(header[6], SEGMENT_CONTROL | SEGMENT_PAYLOAD)
        || header[7] != 0
        || be_u64(&header[8..16])? != ordinal
        || header[36..48].iter().any(|byte| *byte != 0)
        || header[56..64].iter().any(|byte| *byte != 0)
    {
        return Err(segment_invalid(
            "SegmentHeader is noncanonical or reordered",
        ));
    }
    let count = u32::from_be_bytes(header[32..36].try_into().unwrap());
    let extent = be_u64(&header[48..56])?;
    if count > (1 << 20) - 1 || extent < SEGMENT_HEADER_LEN + PROTECTED_HEADER_LEN + 16 {
        return Err(segment_invalid("SegmentHeader count/extent is invalid"));
    }
    Ok((header[6], header[16..32].try_into().unwrap(), count, extent))
}

fn parse_protected<'a>(
    segment: &'a [u8],
    cursor: usize,
    class: u8,
    ordinal: u64,
    counter: u64,
    options: EncryptedOpenOptions<'_>,
) -> Result<(&'a [u8], &'a [u8], usize)> {
    let header_end = cursor
        .checked_add(usize::try_from(PROTECTED_HEADER_LEN).unwrap_or(32))
        .ok_or_else(|| segment_invalid("protected header offset overflow"))?;
    let header = segment
        .get(cursor..header_end)
        .ok_or_else(|| truncated("protected header is truncated"))?;
    if &header[..4] != b"EBC1"
        || be_u16(&header[4..6])? != 1
        || header[6] != class
        || header[7] != 0
        || be_u64(&header[8..16])? != ordinal
        || be_u64(&header[16..24])? != counter
    {
        return Err(segment_invalid(
            "protected record is reordered or noncanonical",
        ));
    }
    let len = be_u64(&header[24..32])?;
    if len < 16 || len > options.crypto_policy.max_ciphertext_record_bytes {
        return Err(crypto_policy("protected ciphertext exceeds caller policy"));
    }
    let end = header_end
        .checked_add(usize::try_from(len).map_err(|_| crypto_policy("ciphertext exceeds usize"))?)
        .ok_or_else(|| segment_invalid("ciphertext extent overflow"))?;
    let ciphertext = segment
        .get(header_end..end)
        .ok_or_else(|| truncated("protected ciphertext is truncated"))?;
    Ok((header, ciphertext, end))
}

#[allow(clippy::too_many_arguments)]
fn decrypt_private_record(
    key: &[u8; 32],
    archive_id: &[u8; 32],
    segment_header: &[u8],
    protected: &[u8],
    ciphertext: &[u8],
    segment_class: u8,
    padding: PaddingMode,
    end: bool,
) -> Result<Vec<u8>> {
    let counter = be_u64(&protected[16..24])?;
    let nonce = if end {
        super::container::end_nonce(counter)
    } else {
        super::container::data_nonce(counter)
    };
    let plaintext = aead_open(
        key,
        &nonce,
        &super::container::record_ad(archive_id, segment_header, protected)?,
        ciphertext,
    )?;
    if plaintext.len() < 8 {
        return Err(private_invalid("authenticated private record is too short"));
    }
    let private_len = usize::try_from(be_u64(&plaintext[..8])?)
        .map_err(|_| crypto_policy("private length exceeds usize"))?;
    let private_end = 8_usize
        .checked_add(private_len)
        .ok_or_else(|| crypto_policy("private length overflow"))?;
    let private = plaintext
        .get(8..private_end)
        .ok_or_else(|| private_invalid("private length exceeds authenticated plaintext"))?;
    let class = if end { SEGMENT_CONTROL } else { segment_class };
    if plaintext.len() != padded_len(8 + private_len, class, padding)? {
        return Err(Diagnostic::new(
            OutcomeClass::Nonconforming,
            ReasonCode::CryptoPaddingInvalid,
            "authenticated padding is noncanonical",
        ));
    }
    Ok(private.to_vec())
}

fn padded_len(unpadded: usize, class: u8, mode: PaddingMode) -> Result<usize> {
    let maximum = if class == SEGMENT_CONTROL {
        1 << 20
    } else {
        64 << 20
    };
    if unpadded > maximum {
        return Err(crypto_policy("private record exceeds class capacity"));
    }
    match mode {
        PaddingMode::None => Ok(unpadded),
        PaddingMode::Maximum => Ok(maximum),
        PaddingMode::Bucketed => {
            let (first, last) = if class == SEGMENT_CONTROL {
                (8_u32, 20_u32)
            } else {
                (12_u32, 26_u32)
            };
            let mut buckets = BTreeSet::new();
            for power in first..=last {
                buckets.insert(1_usize << power);
                buckets.insert(5 * (1_usize << (power - 2)));
                buckets.insert(3 * (1_usize << (power - 1)));
                buckets.insert(7 * (1_usize << (power - 2)));
            }
            buckets
                .into_iter()
                .filter(|value| *value >= 1_usize << first && *value <= 1_usize << last)
                .find(|value| *value >= unpadded)
                .ok_or_else(|| crypto_policy("private record has no padding bucket"))
        }
    }
}

fn data_sequence(count: u32, exact_records: &[u8]) -> Result<Vec<u8>> {
    let mut output = u64::from(count).to_be_bytes().to_vec();
    let mut cursor = 0_usize;
    while cursor < exact_records.len() {
        let header = exact_records
            .get(cursor..cursor + 32)
            .ok_or_else(|| segment_invalid("DATA transcript header is truncated"))?;
        let cipher_len = usize::try_from(be_u64(&header[24..32])?)
            .map_err(|_| crypto_policy("DATA transcript length exceeds usize"))?;
        let extent = 32_usize
            .checked_add(cipher_len)
            .ok_or_else(|| segment_invalid("DATA transcript extent overflow"))?;
        let record = exact_records
            .get(cursor..cursor + extent)
            .ok_or_else(|| segment_invalid("DATA transcript record is truncated"))?;
        output.extend_from_slice(&u64::try_from(extent).unwrap_or(u64::MAX).to_be_bytes());
        output.extend_from_slice(record);
        cursor += extent;
    }
    Ok(output)
}

fn validate_segment_end(
    bytes: &[u8],
    ordinal: u64,
    class: u8,
    count: u32,
    private_bytes: u64,
    ciphertext_bytes: u64,
    digest: &[u8; 32],
) -> Result<()> {
    let (record, consumed) = decode_record(bytes)?;
    if consumed != bytes.len() || record.kind != wire::RECORD_SEGMENT_END {
        return Err(segment_invalid("END plaintext is not SegmentEndV1"));
    }
    record.expect_tags(&[1, 2, 3, 4, 5, 6], &[])?;
    if record.field(1)?.as_u64()? != ordinal
        || record.field(2)?.as_u8()? != class
        || record.field(3)?.as_u32()? != count
        || record.field(4)?.as_u64()? != private_bytes
        || record.field(5)?.as_u64()? != ciphertext_bytes
        || !constant_time_eq(record.field(6)?.as_bytes()?, digest)
    {
        return Err(segment_invalid("SegmentEnd totals/digest mismatch"));
    }
    Ok(())
}

fn decode_archive_final(bytes: &[u8]) -> Result<ArchiveFinal> {
    let (record, consumed) = decode_record(bytes)?;
    if consumed != bytes.len() || record.kind != wire::RECORD_ARCHIVE_FINAL {
        return Err(segment_invalid("terminal object is not ArchiveFinalV1"));
    }
    record.expect_tags(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12], &[])?;
    Ok(ArchiveFinal {
        segment_count: record.field(1)?.as_u64()?,
        entry_count: record.field(2)?.as_u64()?,
        total_logical: record.field(3)?.as_u64()?,
        chunk_count: record.field(4)?.as_u64()?,
        lai: exact(record.field(5)?.as_bytes()?)?,
        pcr: exact(record.field(6)?.as_bytes()?)?,
        aux: exact(record.field(7)?.as_bytes()?)?,
        recipient_set: exact(record.field(8)?.as_bytes()?)?,
        footer_core: exact(record.field(10)?.as_bytes()?)?,
        descriptor_id: exact(record.field(11)?.as_bytes()?)?,
        manifest_id: exact(record.field(12)?.as_bytes()?)?,
    })
}

fn concatenate(items: &[Vec<u8>]) -> Vec<u8> {
    let total = items.iter().map(Vec::len).sum();
    let mut output = Vec::with_capacity(total);
    for item in items {
        output.extend_from_slice(item);
    }
    output
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

fn set_once<T>(slot: &mut Option<T>, value: T, name: &str) -> Result<()> {
    if slot.replace(value).is_some() {
        return Err(private_invalid(format!(
            "duplicate encrypted {name} object"
        )));
    }
    Ok(())
}

fn be_u16(bytes: &[u8]) -> Result<u16> {
    Ok(u16::from_be_bytes(
        bytes
            .try_into()
            .map_err(|_| segment_invalid("expected u16"))?,
    ))
}

fn be_u64(bytes: &[u8]) -> Result<u64> {
    Ok(u64::from_be_bytes(
        bytes
            .try_into()
            .map_err(|_| segment_invalid("expected u64"))?,
    ))
}

fn exact<const N: usize>(bytes: &[u8]) -> Result<[u8; N]> {
    bytes
        .try_into()
        .map_err(|_| private_invalid(format!("expected {N} bytes")))
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    left.len() == right.len() && bool::from(left.ct_eq(right))
}

fn segment_invalid(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::Nonconforming,
        ReasonCode::CryptoSegmentStructureInvalid,
        detail,
    )
}

fn private_invalid(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::Nonconforming,
        ReasonCode::CryptoPrivateObjectInvalid,
        detail,
    )
}

fn crypto_integrity(code: ReasonCode, detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(OutcomeClass::Corrupt, code, detail)
}

fn crypto_policy(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::PolicyRefused,
        ReasonCode::CryptoResourcePolicyRefused,
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

fn dependency(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::Corrupt,
        ReasonCode::RandomAccessDependencyInvalid,
        detail,
    )
}

fn truncated(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(OutcomeClass::Truncated, ReasonCode::TruncatedStream, detail)
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{PROTECTED_HEADER_LEN, SEGMENT_HEADER_LEN, open_indexed_random_encrypted};
    use crate::archive::{PackOptions, plan_directory};
    use crate::crypto::{
        EncryptedOpenOptions, EncryptedWriteOptions, Unlock, XWingIdentity, encrypt_archive,
    };
    use crate::eam::{ContentRef, EntryData, LogicalPath};
    use crate::random_access::{MemoryRandomReadSource, RandomAccessPolicy};

    struct TestDir(PathBuf);

    impl TestDir {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "entrybound-crypto-random-{}-{}",
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

    #[test]
    fn hybrid_random_access_authenticates_control_terminal_and_requested_payload() {
        let directory = TestDir::new();
        std::fs::write(
            directory.path().join("wanted.bin"),
            b"authenticated range bytes",
        )
        .unwrap();
        std::fs::write(
            directory.path().join("unread.bin"),
            vec![91_u8; 2 * 1024 * 1024],
        )
        .unwrap();
        let archive = plan_directory(directory.path(), PackOptions::default()).unwrap();
        let (identity, recipient) = XWingIdentity::generate().unwrap();
        let recipients = [recipient];
        let encrypted = encrypt_archive(
            &archive,
            EncryptedWriteOptions {
                recipients: &recipients,
                ..EncryptedWriteOptions::default()
            },
        )
        .unwrap();
        let source = MemoryRandomReadSource::new(encrypted.bytes);
        let mut opened = open_indexed_random_encrypted(
            source,
            RandomAccessPolicy::default(),
            EncryptedOpenOptions::new(Some(Unlock::Identity(&identity))),
        )
        .unwrap();
        let path = LogicalPath::from_utf8(["wanted.bin"]).unwrap();
        let read = opened.read_entry(&path).unwrap();
        assert_eq!(&*read.bytes, b"authenticated range bytes");
        assert!(read.report.content_object_digest_verified);
        assert!(!read.report.whole_archive_verified);
        assert!(read.report.bytes_fetched < opened.metadata().source_length);
    }

    #[test]
    fn encrypted_missing_index_rebuilds_authenticated_payload_objects() {
        let directory = TestDir::new();
        std::fs::write(directory.path().join("wanted.bin"), b"encrypted fallback").unwrap();
        let archive = plan_directory(directory.path(), PackOptions::default()).unwrap();
        let (identity, recipient) = XWingIdentity::generate().unwrap();
        let encrypted = encrypt_archive(
            &archive,
            EncryptedWriteOptions {
                recipients: &[recipient],
                include_index: false,
                ..EncryptedWriteOptions::default()
            },
        )
        .unwrap();
        let mut opened = open_indexed_random_encrypted(
            MemoryRandomReadSource::new(encrypted.bytes),
            RandomAccessPolicy::default(),
            EncryptedOpenOptions::new(Some(Unlock::Identity(&identity))),
        )
        .unwrap();
        let read = opened
            .read_entry(&LogicalPath::from_utf8(["wanted.bin"]).unwrap())
            .unwrap();
        assert_eq!(&*read.bytes, b"encrypted fallback");
        assert_eq!(
            read.report.index_status,
            crate::ecf::RandomAccessIndexStatus::RebuiltAbsent
        );
    }

    #[test]
    fn password_random_access_uses_the_existing_policy_checked_unlock() {
        let directory = TestDir::new();
        std::fs::write(directory.path().join("wanted.bin"), b"password range bytes").unwrap();
        let archive = plan_directory(directory.path(), PackOptions::default()).unwrap();
        let password = b"high entropy password for range testing";
        let encrypted = encrypt_archive(
            &archive,
            EncryptedWriteOptions {
                password: Some(password),
                ..EncryptedWriteOptions::default()
            },
        )
        .unwrap();
        let mut opened = open_indexed_random_encrypted(
            MemoryRandomReadSource::new(encrypted.bytes),
            RandomAccessPolicy::default(),
            EncryptedOpenOptions::new(Some(Unlock::Password(password))),
        )
        .unwrap();
        let read = opened
            .read_entry(&LogicalPath::from_utf8(["wanted.bin"]).unwrap())
            .unwrap();
        assert_eq!(&*read.bytes, b"password range bytes");
        assert!(!read.report.whole_archive_verified);
    }

    #[test]
    fn unread_encrypted_payload_corruption_does_not_become_a_whole_archive_claim() {
        let directory = TestDir::new();
        std::fs::write(
            directory.path().join("wanted.bin"),
            b"authenticated range bytes",
        )
        .unwrap();
        std::fs::write(
            directory.path().join("unread.bin"),
            vec![17_u8; 1024 * 1024],
        )
        .unwrap();
        let archive = plan_directory(directory.path(), PackOptions::default()).unwrap();
        let unread = archive
            .entry_set
            .entries()
            .iter()
            .find(|entry| entry.path().to_string() == "unread.bin")
            .and_then(|entry| match entry.data() {
                EntryData::File {
                    content: ContentRef::Internal(content),
                } => archive.content_store.objects.get(content),
                EntryData::Directory
                | EntryData::Symlink { .. }
                | EntryData::ReparsePoint { .. } => None,
            })
            .unwrap()
            .chunks[0]
            .chunk_id;
        let (identity, recipient) = XWingIdentity::generate().unwrap();
        let encrypted = encrypt_archive(
            &archive,
            EncryptedWriteOptions {
                recipients: &[recipient],
                ..EncryptedWriteOptions::default()
            },
        )
        .unwrap();
        let probe = open_indexed_random_encrypted(
            MemoryRandomReadSource::new(encrypted.bytes.clone()),
            RandomAccessPolicy::default(),
            EncryptedOpenOptions::new(Some(Unlock::Identity(&identity))),
        )
        .unwrap();
        let ordinal = probe.index[&unread].segment_ordinal;
        let segment = &probe.segments[usize::try_from(ordinal).unwrap()];
        let mutation =
            usize::try_from(segment.offset + SEGMENT_HEADER_LEN + PROTECTED_HEADER_LEN).unwrap();
        let mut corrupted = encrypted.bytes;
        corrupted[mutation] ^= 0x40;
        let mut opened = open_indexed_random_encrypted(
            MemoryRandomReadSource::new(corrupted),
            RandomAccessPolicy::default(),
            EncryptedOpenOptions::new(Some(Unlock::Identity(&identity))),
        )
        .unwrap();
        let read = opened
            .read_entry(&LogicalPath::from_utf8(["wanted.bin"]).unwrap())
            .unwrap();
        assert_eq!(&*read.bytes, b"authenticated range bytes");
        assert!(!read.report.whole_archive_verified);
        assert_eq!(
            read.report.pci,
            crate::ecf::IdentityVerificationStatus::NotComputed
        );
    }
}
