use std::collections::{BTreeMap, BTreeSet};

use super::records::{
    DescriptorBody, decode_chunk_groups, decode_descriptor, decode_dictionaries, decode_fidelity,
    decode_index, decode_manifest, decode_transform_plans, encode_chunk_groups, encode_descriptor,
    encode_dictionaries, encode_fidelity, encode_index, encode_manifest, encode_transform_plans,
};
use super::{
    CHUNK_FRAME_HEADER_LEN, CHUNK_FRAME_V2_HEADER_LEN, FEATURE_CROSS_FILE_COMPRESSION_V1,
    FOOTER_LEN, FORMAT_NAMESPACE, FormatVersion, MAGIC, PREAMBLE_LEN, SECTION_HEADER_LEN,
    SUPPORTED_INCOMPAT_FEATURES, SectionKind,
};
use crate::codec::{
    PlanMode, aggregate_archive_decode_requirements, decode_payload,
    decode_payload_with_dictionary, decode_payload_with_prefix, encode_payload,
    encode_payload_with_dictionary, encode_payload_with_prefix, plan_mode, validate_plans,
};
use crate::diagnostics::{Diagnostic, OutcomeClass, ReasonCode, Result};
use crate::eam::{
    Archive, ArchiveDescriptor, ArchiveRole, Chunk, ChunkGroup, ChunkLocation, ContentStore,
    DecodeRequirements, Dictionary, Digest, DigestAlgorithm, FeatureSet, IdentityProfile, Index,
    Layout, ResourceBudget, TransformPlan,
};
use crate::identity::{
    IdentitySet, apply_native_identities, physical_container_identity, sha256_exact,
};

const SECTION_MAGIC: [u8; 4] = *b"EBS1";
const CHUNK_MAGIC: [u8; 4] = *b"EBCH";
const FOOTER_MAGIC: [u8; 8] = [0x8e, b'E', b'B', b'F', b'\r', b'\n', 0x1a, b'\n'];
const SECTION_VERSION: u16 = 1;

/// Writer options that affect only reconstructible physical data.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WriteOptions {
    pub include_index: bool,
}

impl Default for WriteOptions {
    fn default() -> Self {
        Self {
            include_index: true,
        }
    }
}

/// Result of deterministic native serialization.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EncodedArchive {
    pub bytes: Vec<u8>,
    pub archive: Archive,
    pub identities: IdentitySet,
}

/// How the reader handled the optional non-authoritative Index.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IndexStatus {
    PresentValid,
    RebuiltAbsent,
    RebuiltInvalid,
}

/// Explicit checklist produced by native verification.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationReport {
    pub canonical_encoding: bool,
    pub container_structure: bool,
    pub section_integrity: bool,
    pub semantic_invariants: bool,
    pub chunk_integrity: bool,
    pub dictionary_integrity: bool,
    pub chunk_group_integrity: bool,
    pub access_costs: bool,
    pub content_integrity: bool,
    pub entry_identities: bool,
    pub lai: bool,
    pub pcr: bool,
    pub aux: bool,
    pub pci_computed: bool,
    pub index_status: IndexStatus,
    pub index_reason: Option<ReasonCode>,
    pub identities: IdentitySet,
}

/// An opened, verified archive.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpenedArchive {
    pub archive: Archive,
    pub report: VerificationReport,
}

/// Serializes a validated EAM as canonical unencrypted Complete INDEXED ECF.
pub fn encode(input: &Archive, options: WriteOptions) -> Result<EncodedArchive> {
    input.validate()?;
    validate_plans(&input.transform_plans)?;
    validate_feature_model(input)?;
    let (mut archive, roots) = apply_native_identities(input)?;
    let extended = has_cross_file_feature(archive.descriptor.features);
    let plans_payload = encode_transform_plans(&archive.transform_plans)?;
    let dictionaries_payload = encode_dictionaries(&archive.content_store.dictionaries)?;
    let groups_payload = encode_chunk_groups(&archive.content_store.chunk_groups)?;
    let (chunk_payload, relative_index) = encode_chunks(&archive, extended)?;
    normalize_descriptor(&mut archive, &relative_index)?;

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
    let manifest_payload = encode_manifest(&archive.entry_set, &archive.content_store.objects)?;
    let fidelity_payload = encode_fidelity(&archive.fidelity)?;

    let mut body = Vec::new();
    let descriptor = append_section(
        &mut body,
        SectionKind::Descriptor,
        &descriptor_payload,
        extended,
    )?;
    append_section(
        &mut body,
        SectionKind::TransformPlans,
        &plans_payload,
        extended,
    )?;
    if extended {
        append_section(
            &mut body,
            SectionKind::Dictionaries,
            &dictionaries_payload,
            extended,
        )?;
        append_section(
            &mut body,
            SectionKind::ChunkGroups,
            &groups_payload,
            extended,
        )?;
    }
    let chunk_section =
        append_section(&mut body, SectionKind::ChunkData, &chunk_payload, extended)?;
    let manifest = append_section(
        &mut body,
        SectionKind::ManifestRecords,
        &manifest_payload,
        extended,
    )?;
    append_section(
        &mut body,
        SectionKind::Fidelity,
        &fidelity_payload,
        extended,
    )?;

    let payload_base = PREAMBLE_LEN
        .checked_add(chunk_section.offset)
        .and_then(|value| value.checked_add(SECTION_HEADER_LEN))
        .ok_or_else(|| resource("chunk section offset overflow"))?;
    let authoritative_index = relative_index
        .into_iter()
        .map(|(digest, location)| {
            Ok((
                digest,
                ChunkLocation {
                    offset: payload_base
                        .checked_add(location.offset)
                        .ok_or_else(|| resource("chunk frame offset overflow"))?,
                    stored_len: location.stored_len,
                },
            ))
        })
        .collect::<Result<BTreeMap<_, _>>>()?;

    if options.include_index {
        let index_payload = encode_index(&authoritative_index)?;
        append_section(&mut body, SectionKind::Index, &index_payload, extended)?;
    }

    let footer_offset = PREAMBLE_LEN
        .checked_add(u64_len(&body)?)
        .ok_or_else(|| resource("footer offset overflow"))?;
    let total_len = footer_offset
        .checked_add(FOOTER_LEN)
        .ok_or_else(|| resource("container length overflow"))?;
    let preamble = encode_preamble(&archive.descriptor, footer_offset)?;
    let footer = encode_footer(
        total_len,
        absolute_section(descriptor),
        absolute_section(manifest),
        u64::try_from(archive.entry_set.len()).map_err(|_| resource("entry count exceeds u64"))?,
        archive.total_logical_size()?,
        sha256_exact(&preamble),
    );

    let capacity = usize::try_from(total_len).map_err(|_| resource("container exceeds usize"))?;
    let mut bytes = Vec::with_capacity(capacity);
    bytes.extend_from_slice(&preamble);
    bytes.extend_from_slice(&body);
    bytes.extend_from_slice(&footer);
    if u64_len(&bytes)? != total_len {
        return Err(structure(
            "writer produced an inconsistent container length",
        ));
    }

    let pci = physical_container_identity(&bytes);
    let identities = roots.with_pci(pci);
    archive.descriptor.pci = Some(pci.0);
    archive.index = if options.include_index {
        Index {
            present: true,
            valid: true,
            chunks: authoritative_index,
            status: "present and valid".to_owned(),
        }
    } else {
        Index {
            present: false,
            valid: false,
            chunks: authoritative_index,
            status: "absent; rebuilt from CHUNK_DATA".to_owned(),
        }
    };
    Ok(EncodedArchive {
        bytes,
        archive,
        identities,
    })
}

/// Opens and fully verifies canonical bootstrap ECF bytes.
pub fn open(bytes: &[u8]) -> Result<OpenedArchive> {
    open_with_limits(
        bytes,
        crate::archive::bootstrap_resource_policy(),
        crate::archive::bootstrap_decode_policy(),
    )
}

/// Opens and fully verifies bytes while enforcing caller-owned resource limits.
pub fn open_with_policy(bytes: &[u8], policy: ResourceBudget) -> Result<OpenedArchive> {
    open_with_limits(bytes, policy, crate::archive::bootstrap_decode_policy())
}

/// Opens and verifies bytes under explicit size and decoder-memory limits.
pub fn open_with_limits(
    bytes: &[u8],
    policy: ResourceBudget,
    decode_policy: DecodeRequirements,
) -> Result<OpenedArchive> {
    let preamble = decode_preamble(bytes)?;
    enforce_caller_policy(preamble.budget, policy)?;
    enforce_decode_policy(preamble.decode, decode_policy)?;
    let footer = decode_footer(bytes, &preamble)?;
    if preamble.footer_hint != footer.offset {
        return Err(noncanonical(
            "canonical INDEXED preamble footer hint must identify the fixed footer",
        ));
    }
    let extended = has_cross_file_feature(preamble.features);
    let sections = decode_sections(bytes, &footer, extended)?;

    let descriptor_section = required_section(&sections, SectionKind::Descriptor)?;
    let plans_section = required_section(&sections, SectionKind::TransformPlans)?;
    let chunks_section = required_section(&sections, SectionKind::ChunkData)?;
    let manifest_section = required_section(&sections, SectionKind::ManifestRecords)?;
    let fidelity_section = required_section(&sections, SectionKind::Fidelity)?;
    let dictionaries = if extended {
        decode_dictionaries(required_section(&sections, SectionKind::Dictionaries)?.payload)?
    } else {
        BTreeMap::new()
    };
    let chunk_groups = if extended {
        decode_chunk_groups(required_section(&sections, SectionKind::ChunkGroups)?.payload)?
    } else {
        BTreeMap::new()
    };

    if descriptor_section.location != footer.descriptor
        || manifest_section.location != footer.manifest
    {
        return Err(structure(
            "footer authoritative locators do not match section framing",
        ));
    }

    let descriptor_body = decode_descriptor(descriptor_section.payload)?;
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
    let plans = decode_transform_plans(plans_section.payload)?;
    let (chunks, physical_order, rebuilt_index) = decode_chunks(
        chunks_section.payload,
        chunks_section
            .location
            .offset
            .checked_add(SECTION_HEADER_LEN)
            .ok_or_else(|| resource("chunk payload offset overflow"))?,
        &plans,
        &dictionaries,
        &chunk_groups,
        preamble.budget,
        extended,
    )?;
    let (entries, objects) = decode_manifest(manifest_section.payload)?;
    let fidelity = decode_fidelity(fidelity_section.payload)?;

    let index_section = sections
        .iter()
        .find(|section| section.kind == SectionKind::Index);
    let (index, index_status) = validate_or_rebuild_index(index_section, &rebuilt_index);
    let descriptor = ArchiveDescriptor {
        format_major: preamble.version.major,
        format_minor: preamble.version.minor,
        format_namespace: descriptor_body.namespace,
        features: preamble.features,
        layout: Layout::Indexed,
        role: ArchiveRole::Complete,
        budget_declared: preamble.budget_declared,
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
    let archive = Archive {
        descriptor,
        entry_set: entries,
        content_store: ContentStore {
            objects,
            chunks,
            dictionaries,
            chunk_groups,
            physical_order,
        },
        transform_plans: plans,
        fidelity,
        index,
    };
    archive.validate()?;
    validate_actuals(
        &archive,
        &footer,
        u64_len(manifest_section.payload)?,
        &rebuilt_index,
    )?;

    let stored_entries = archive
        .entry_set
        .entries()
        .iter()
        .map(|entry| entry.identity())
        .collect::<Vec<_>>();
    let stored_lai = archive.descriptor.lai;
    let stored_pcr = archive.descriptor.pcr;
    let stored_aux = archive.descriptor.aux;
    let (mut canonical, roots) = apply_native_identities(&archive)?;
    for (stored, recomputed) in stored_entries.iter().zip(
        canonical
            .entry_set
            .entries()
            .iter()
            .map(|entry| entry.identity()),
    ) {
        if stored.identity_digest != recomputed.identity_digest {
            return Err(Diagnostic::new(
                OutcomeClass::Corrupt,
                ReasonCode::EntryIdentityMismatch,
                "serialized Entry identity does not match its canonical fields",
            ));
        }
        if stored.aux_digest != recomputed.aux_digest {
            return Err(Diagnostic::new(
                OutcomeClass::Corrupt,
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

    let pci = physical_container_identity(bytes);
    canonical.descriptor.pci = Some(pci.0);
    canonical.index = archive.index;
    let identities = roots.with_pci(pci);
    Ok(OpenedArchive {
        archive: canonical,
        report: VerificationReport {
            canonical_encoding: true,
            container_structure: true,
            section_integrity: true,
            semantic_invariants: true,
            chunk_integrity: true,
            dictionary_integrity: true,
            chunk_group_integrity: true,
            access_costs: true,
            content_integrity: true,
            entry_identities: true,
            lai: true,
            pcr: true,
            aux: true,
            pci_computed: true,
            index_status,
            index_reason: match index_status {
                IndexStatus::PresentValid => None,
                IndexStatus::RebuiltAbsent => Some(ReasonCode::IndexAbsentRebuilt),
                IndexStatus::RebuiltInvalid => Some(ReasonCode::IndexInvalidRebuilt),
            },
            identities,
        },
    })
}

/// Verifies native bytes without hiding which guarantees were checked.
pub fn verify(bytes: &[u8]) -> Result<VerificationReport> {
    Ok(open(bytes)?.report)
}

/// Verifies native bytes under explicit caller-owned resource limits.
pub fn verify_with_policy(bytes: &[u8], policy: ResourceBudget) -> Result<VerificationReport> {
    Ok(open_with_policy(bytes, policy)?.report)
}

/// Verifies native bytes under explicit size and decoder-memory limits.
pub fn verify_with_limits(
    bytes: &[u8],
    policy: ResourceBudget,
    decode_policy: DecodeRequirements,
) -> Result<VerificationReport> {
    Ok(open_with_limits(bytes, policy, decode_policy)?.report)
}

fn enforce_caller_policy(declared: ResourceBudget, policy: ResourceBudget) -> Result<()> {
    let exceeded = declared.entry_count > policy.entry_count
        || declared.total_logical_bytes > policy.total_logical_bytes
        || declared.max_single_entry_logical_bytes > policy.max_single_entry_logical_bytes
        || declared.max_expansion_ratio_milli > policy.max_expansion_ratio_milli
        || declared.chunk_count > policy.chunk_count
        || declared.max_path_depth > policy.max_path_depth
        || declared.max_metadata_bytes > policy.max_metadata_bytes
        || declared.max_key_derivation_cost > policy.max_key_derivation_cost;
    if exceeded {
        return Err(Diagnostic::new(
            OutcomeClass::PolicyRefused,
            ReasonCode::ResourceLimit,
            "archive resource declaration exceeds caller policy",
        ));
    }
    Ok(())
}

fn enforce_decode_policy(declared: DecodeRequirements, policy: DecodeRequirements) -> Result<()> {
    if declared.window_bytes > policy.window_bytes
        || declared.working_set_bytes > policy.working_set_bytes
        || declared.flags & !policy.flags != 0
    {
        return Err(Diagnostic::new(
            OutcomeClass::PolicyRefused,
            ReasonCode::ResourceLimit,
            "archive decoder requirements exceed caller policy",
        ));
    }
    Ok(())
}

fn has_cross_file_feature(features: FeatureSet) -> bool {
    features.incompat & FEATURE_CROSS_FILE_COMPRESSION_V1 != 0
}

fn validate_feature_model(archive: &Archive) -> Result<()> {
    if archive.descriptor.features.incompat & !SUPPORTED_INCOMPAT_FEATURES != 0 {
        return Err(Diagnostic::new(
            OutcomeClass::Unsupported,
            ReasonCode::UnsupportedRequiredFeature,
            "archive declares unsupported required feature bits",
        ));
    }
    let extended = has_cross_file_feature(archive.descriptor.features);
    for dictionary in archive.content_store.dictionaries.values() {
        crate::codec::validate_dictionary(dictionary)?;
    }
    let dependent_plan = archive
        .transform_plans
        .iter()
        .try_fold(false, |found, plan| {
            Ok::<_, Diagnostic>(found || plan_mode(plan)? != PlanMode::Independent)
        })?;
    let dependent_model = !archive.content_store.dictionaries.is_empty()
        || !archive.content_store.chunk_groups.is_empty()
        || archive
            .content_store
            .chunks
            .values()
            .any(|chunk| chunk.group_ref.is_some())
        || dependent_plan;
    if dependent_model && !extended {
        return Err(Diagnostic::new(
            OutcomeClass::Unsupported,
            ReasonCode::UnsupportedRequiredFeature,
            "Dictionary or ChunkGroup behavior requires cross-file-compression-v1",
        ));
    }
    if !extended
        && archive.content_store.physical_order.as_ref()
            != archive
                .content_store
                .chunks
                .keys()
                .copied()
                .collect::<Vec<_>>()
                .as_slice()
    {
        return Err(noncanonical(
            "historical Chunk frames must remain digest ordered",
        ));
    }
    Ok(())
}

fn normalize_descriptor(
    archive: &mut Archive,
    relative_index: &BTreeMap<Digest, ChunkLocation>,
) -> Result<()> {
    archive.descriptor.format_major = FormatVersion::BOOTSTRAP.major;
    archive.descriptor.format_minor = FormatVersion::BOOTSTRAP.minor;
    archive.descriptor.format_namespace = FORMAT_NAMESPACE.to_owned();
    if archive.descriptor.features.incompat & !SUPPORTED_INCOMPAT_FEATURES != 0 {
        return Err(Diagnostic::new(
            OutcomeClass::Unsupported,
            ReasonCode::UnsupportedRequiredFeature,
            format!(
                "unknown incompat feature bits {:016x}",
                archive.descriptor.features.incompat & !SUPPORTED_INCOMPAT_FEATURES
            ),
        ));
    }
    archive.descriptor.layout = Layout::Indexed;
    archive.descriptor.role = ArchiveRole::Complete;
    archive.descriptor.budget_declared = true;
    archive.descriptor.decode = aggregate_archive_decode_requirements(
        &archive.transform_plans,
        &archive.content_store.dictionaries,
        &archive.content_store.chunk_groups,
    )?;
    archive.descriptor.identity_profile = IdentityProfile::IdentityV1;
    archive.descriptor.digest_algorithm = DigestAlgorithm::Sha256;
    if archive.descriptor.planner_id.is_empty() {
        return Err(noncanonical("descriptor planner_id must be non-empty"));
    }
    if archive.descriptor.chunker_id.is_empty() {
        return Err(noncanonical("descriptor chunker_id must be non-empty"));
    }
    archive.descriptor.budget = derived_budget(archive, relative_index)?;
    Ok(())
}

fn derived_budget(
    archive: &Archive,
    relative_index: &BTreeMap<Digest, ChunkLocation>,
) -> Result<ResourceBudget> {
    let mut max_single = 0_u64;
    for object in archive.content_store.objects.values() {
        let size = object.chunks.iter().try_fold(0_u64, |total, chunk_ref| {
            let chunk = archive
                .content_store
                .chunks
                .get(&chunk_ref.chunk_id)
                .ok_or_else(|| structure("ContentObject references an unknown Chunk"))?;
            total
                .checked_add(chunk.logical_len)
                .ok_or_else(|| resource("ContentObject logical size overflow"))
        })?;
        max_single = max_single.max(size);
    }
    let max_path_depth = archive
        .entry_set
        .entries()
        .iter()
        .map(|entry| entry.path().depth())
        .max()
        .unwrap_or(0);
    let metadata_bound = u64_len(&encode_manifest(
        &archive.entry_set,
        &archive.content_store.objects,
    )?)?;
    Ok(ResourceBudget {
        entry_count: u64::try_from(archive.entry_set.len())
            .map_err(|_| resource("entry count exceeds u64"))?,
        total_logical_bytes: archive.total_logical_size()?,
        max_single_entry_logical_bytes: max_single,
        max_expansion_ratio_milli: maximum_expansion_ratio(
            &archive.content_store.chunks,
            relative_index,
        )?,
        chunk_count: u64::try_from(archive.content_store.chunks.len())
            .map_err(|_| resource("Chunk count exceeds u64"))?,
        max_path_depth: u64::try_from(max_path_depth)
            .map_err(|_| resource("path depth exceeds u64"))?,
        max_metadata_bytes: metadata_bound,
        max_key_derivation_cost: 0,
    })
}

fn maximum_expansion_ratio(
    chunks: &BTreeMap<Digest, Chunk>,
    index: &BTreeMap<Digest, ChunkLocation>,
) -> Result<u64> {
    let mut maximum = 1_000_u64;
    for (chunk_id, chunk) in chunks {
        let stored_len = index
            .get(chunk_id)
            .ok_or_else(|| structure("encoded Chunk is absent from the rebuilt Index"))?
            .stored_len;
        maximum = maximum.max(expansion_ratio(chunk.logical_len, stored_len)?);
    }
    Ok(maximum)
}

fn expansion_ratio(logical_len: u64, stored_len: u64) -> Result<u64> {
    if stored_len == 0 {
        return if logical_len == 0 {
            Ok(0)
        } else {
            Err(structure("non-empty Chunk has a zero stored length"))
        };
    }
    let numerator = u128::from(logical_len)
        .checked_mul(1_000)
        .and_then(|value| value.checked_add(u128::from(stored_len) - 1))
        .ok_or_else(|| resource("expansion-ratio calculation overflow"))?;
    u64::try_from(numerator / u128::from(stored_len))
        .map_err(|_| resource("expansion ratio exceeds u64"))
}

fn encode_chunks(
    archive: &Archive,
    extended: bool,
) -> Result<(Vec<u8>, BTreeMap<Digest, ChunkLocation>)> {
    let plans = archive
        .transform_plans
        .iter()
        .map(|plan| (plan.plan_id, plan))
        .collect::<BTreeMap<_, _>>();
    let mut payload = Vec::new();
    let mut index = BTreeMap::new();
    let mut group_history = BTreeMap::<Digest, Vec<Digest>>::new();
    for chunk_id in &archive.content_store.physical_order {
        let chunk = archive.content_store.chunks.get(chunk_id).ok_or_else(|| {
            Diagnostic::new(
                OutcomeClass::Nonconforming,
                ReasonCode::UnknownChunk,
                chunk_id.to_string(),
            )
        })?;
        if chunk.logical_len != u64_len(&chunk.plaintext)? {
            return Err(structure("Chunk has an inconsistent logical length"));
        }
        let plan = plans.get(&chunk.plan_ref).ok_or_else(|| {
            Diagnostic::new(
                OutcomeClass::Unsupported,
                ReasonCode::UnknownTransformPlan,
                format!("Chunk {} uses plan {}", chunk.chunk_id, chunk.plan_ref),
            )
        })?;
        let stored = match plan_mode(plan)? {
            PlanMode::Independent if chunk.group_ref.is_none() => {
                encode_payload(plan, &chunk.plaintext)?
            }
            PlanMode::Dictionary(dictionary_id) if chunk.group_ref.is_none() => {
                let dictionary = archive
                    .content_store
                    .dictionaries
                    .get(&dictionary_id)
                    .ok_or_else(|| {
                        Diagnostic::new(
                            OutcomeClass::Nonconforming,
                            ReasonCode::UnknownDictionary,
                            dictionary_id.to_string(),
                        )
                    })?;
                encode_payload_with_dictionary(plan, &chunk.plaintext, dictionary)?
            }
            PlanMode::Prefix { lookback } => {
                let group_id = chunk.group_ref.ok_or_else(|| {
                    Diagnostic::new(
                        OutcomeClass::Nonconforming,
                        ReasonCode::InvalidGroupReference,
                        format!("prefix Chunk {} has no group_ref", chunk.chunk_id),
                    )
                })?;
                let group = archive
                    .content_store
                    .chunk_groups
                    .get(&group_id)
                    .ok_or_else(|| {
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
                let prefix = physical_prefix(
                    &archive.content_store.chunks,
                    group_history.entry(group_id).or_default(),
                    lookback,
                )?;
                encode_payload_with_prefix(plan, &chunk.plaintext, &prefix)?
            }
            _ => {
                return Err(Diagnostic::new(
                    OutcomeClass::Nonconforming,
                    ReasonCode::InvalidGroupReference,
                    format!(
                        "Chunk {} has a group incompatible with its plan",
                        chunk.chunk_id
                    ),
                ));
            }
        };
        let stored_len = u64_len(&stored)?;
        let offset = u64_len(&payload)?;
        payload.extend_from_slice(&CHUNK_MAGIC);
        payload.extend_from_slice(&(if extended { 2_u16 } else { SECTION_VERSION }).to_be_bytes());
        payload.extend_from_slice(&0_u16.to_be_bytes());
        payload.extend_from_slice(&stored_len.to_be_bytes());
        payload.extend_from_slice(chunk.chunk_id.as_bytes());
        payload.extend_from_slice(&chunk.logical_len.to_be_bytes());
        payload.extend_from_slice(&chunk.plan_ref.to_be_bytes());
        if extended {
            payload.extend_from_slice(chunk.group_ref.unwrap_or(Digest::ZERO).as_bytes());
        }
        payload.extend_from_slice(&stored);
        index.insert(chunk.chunk_id, ChunkLocation { offset, stored_len });
        if let Some(group_id) = chunk.group_ref {
            group_history
                .entry(group_id)
                .or_default()
                .push(chunk.chunk_id);
        }
    }
    Ok((payload, index))
}

#[derive(Clone, Copy)]
struct EncodedChunkFrame<'a> {
    offset: u64,
    chunk_id: Digest,
    logical_len: u64,
    plan_ref: u64,
    group_ref: Option<Digest>,
    stored: &'a [u8],
}

type DecodedChunks = (
    BTreeMap<Digest, Chunk>,
    Box<[Digest]>,
    BTreeMap<Digest, ChunkLocation>,
);

fn decode_chunks(
    payload: &[u8],
    absolute_payload_offset: u64,
    plans: &[TransformPlan],
    dictionaries: &BTreeMap<Digest, Dictionary>,
    groups: &BTreeMap<Digest, ChunkGroup>,
    declared_budget: ResourceBudget,
    extended: bool,
) -> Result<DecodedChunks> {
    let plans = plans
        .iter()
        .map(|plan| (plan.plan_id, plan))
        .collect::<BTreeMap<_, _>>();
    let mut frames = Vec::<EncodedChunkFrame<'_>>::new();
    let mut seen = BTreeSet::new();
    let mut cursor = 0_usize;
    let mut previous = None;
    while cursor < payload.len() {
        let header_len = usize::try_from(if extended {
            CHUNK_FRAME_V2_HEADER_LEN
        } else {
            CHUNK_FRAME_HEADER_LEN
        })
        .unwrap_or(if extended { 96 } else { 64 });
        if payload.len() - cursor < header_len {
            return Err(structure("Chunk frame header is truncated"));
        }
        let header = &payload[cursor..cursor + header_len];
        if header[..4] != CHUNK_MAGIC {
            return Err(structure("Chunk frame magic mismatch"));
        }
        let expected_version = if extended { 2 } else { SECTION_VERSION };
        if u16::from_be_bytes(exact(&header[4..6])?) != expected_version || header[6..8] != [0; 2] {
            return Err(noncanonical(
                "Chunk frame version or flags are not canonical",
            ));
        }
        let stored_len = u64::from_be_bytes(exact(&header[8..16])?);
        let chunk_id = digest(&header[16..48])?;
        let logical_len = u64::from_be_bytes(exact(&header[48..56])?);
        let plan_ref = u64::from_be_bytes(exact(&header[56..64])?);
        let group_ref = if extended {
            let value = digest(&header[64..96])?;
            (value != Digest::ZERO).then_some(value)
        } else {
            None
        };
        if !seen.insert(chunk_id) {
            return Err(noncanonical("Chunk frame IDs must be unique"));
        }
        if !extended && previous.is_some_and(|digest| digest >= chunk_id) {
            return Err(noncanonical(
                "Chunk frames must be uniquely ordered by chunk ID",
            ));
        }
        previous = Some(chunk_id);
        plans.get(&plan_ref).ok_or_else(|| {
            Diagnostic::new(
                OutcomeClass::Unsupported,
                ReasonCode::UnknownTransformPlan,
                format!("Chunk {chunk_id} uses plan {plan_ref}"),
            )
        })?;
        if logical_len > declared_budget.max_single_entry_logical_bytes {
            return Err(Diagnostic::new(
                OutcomeClass::Corrupt,
                ReasonCode::ResourceLimit,
                format!("Chunk {chunk_id} exceeds the archive's declared logical bound"),
            ));
        }
        if expansion_ratio(logical_len, stored_len)? > declared_budget.max_expansion_ratio_milli {
            return Err(Diagnostic::new(
                OutcomeClass::Corrupt,
                ReasonCode::ResourceLimit,
                format!("Chunk {chunk_id} exceeds the archive's declared expansion bound"),
            ));
        }
        let data_start = cursor
            .checked_add(header_len)
            .ok_or_else(|| resource("Chunk frame offset overflow"))?;
        let data_end = data_start
            .checked_add(usize::try_from(stored_len).map_err(|_| resource("Chunk too large"))?)
            .ok_or_else(|| resource("Chunk data length overflow"))?;
        if data_end > payload.len() {
            return Err(structure("Chunk stored length exceeds CHUNK_DATA"));
        }
        let frame_offset =
            u64::try_from(cursor).map_err(|_| resource("Chunk offset exceeds u64"))?;
        frames.push(EncodedChunkFrame {
            offset: absolute_payload_offset
                .checked_add(frame_offset)
                .ok_or_else(|| resource("absolute Chunk offset overflow"))?,
            chunk_id,
            logical_len,
            plan_ref,
            group_ref,
            stored: &payload[data_start..data_end],
        });
        cursor = data_end;
        if u64::try_from(frames.len()).unwrap_or(u64::MAX) > declared_budget.chunk_count {
            return Err(Diagnostic::new(
                OutcomeClass::Corrupt,
                ReasonCode::ResourceLimit,
                "decoded Chunk count exceeds the archive declaration",
            ));
        }
    }
    validate_frame_dependencies(&frames, &plans, dictionaries, groups)?;

    let mut chunks = BTreeMap::new();
    let mut index = BTreeMap::new();
    let mut group_history = BTreeMap::<Digest, Vec<Digest>>::new();
    for (position, frame) in frames.iter().enumerate() {
        let plan = plans[&frame.plan_ref];
        let decoded = match plan_mode(plan)? {
            PlanMode::Independent => decode_payload(plan, frame.stored, frame.logical_len),
            PlanMode::Dictionary(dictionary_id) => {
                let dictionary = dictionaries.get(&dictionary_id).ok_or_else(|| {
                    Diagnostic::new(
                        OutcomeClass::Nonconforming,
                        ReasonCode::UnknownDictionary,
                        dictionary_id.to_string(),
                    )
                })?;
                decode_payload_with_dictionary(plan, frame.stored, frame.logical_len, dictionary)
            }
            PlanMode::Prefix { lookback } => {
                let group_id = frame.group_ref.ok_or_else(|| {
                    Diagnostic::new(
                        OutcomeClass::Nonconforming,
                        ReasonCode::InvalidGroupReference,
                        frame.chunk_id.to_string(),
                    )
                })?;
                let prefix = physical_prefix(
                    &chunks,
                    group_history.entry(group_id).or_default(),
                    lookback,
                )?;
                decode_payload_with_prefix(plan, frame.stored, frame.logical_len, &prefix)
            }
        };
        let plaintext = match decoded {
            Ok(plaintext) => plaintext,
            Err(error) if is_group_prerequisite(&frames, position, groups) => {
                return Err(Diagnostic::new(
                    OutcomeClass::Corrupt,
                    ReasonCode::PrerequisiteChunkCorrupt,
                    format!(
                        "Chunk {} is required by a later group member: {} {}",
                        frame.chunk_id,
                        error.code().as_str(),
                        error.detail()
                    ),
                ));
            }
            Err(error) => return Err(error),
        };
        if sha256_exact(&plaintext) != frame.chunk_id {
            let code = if is_group_prerequisite(&frames, position, groups) {
                ReasonCode::PrerequisiteChunkCorrupt
            } else {
                ReasonCode::ChunkDigestMismatch
            };
            return Err(integrity(code, format!("Chunk {}", frame.chunk_id)));
        }
        index.insert(
            frame.chunk_id,
            ChunkLocation {
                offset: frame.offset,
                stored_len: u64::try_from(frame.stored.len())
                    .map_err(|_| resource("stored Chunk length exceeds u64"))?,
            },
        );
        chunks.insert(
            frame.chunk_id,
            Chunk {
                chunk_id: frame.chunk_id,
                logical_len: frame.logical_len,
                plan_ref: frame.plan_ref,
                group_ref: frame.group_ref,
                plaintext: plaintext.into_boxed_slice(),
            },
        );
        if let Some(group_id) = frame.group_ref {
            group_history
                .entry(group_id)
                .or_default()
                .push(frame.chunk_id);
        }
    }
    let physical_order = frames
        .iter()
        .map(|frame| frame.chunk_id)
        .collect::<Vec<_>>()
        .into_boxed_slice();
    Ok((chunks, physical_order, index))
}

fn validate_frame_dependencies(
    frames: &[EncodedChunkFrame<'_>],
    plans: &BTreeMap<u64, &TransformPlan>,
    dictionaries: &BTreeMap<Digest, Dictionary>,
    groups: &BTreeMap<Digest, ChunkGroup>,
) -> Result<()> {
    let mut positions = BTreeMap::<Digest, Vec<usize>>::new();
    for (position, frame) in frames.iter().enumerate() {
        match plan_mode(plans[&frame.plan_ref])? {
            PlanMode::Independent if frame.group_ref.is_none() => {}
            PlanMode::Dictionary(dictionary_id) if frame.group_ref.is_none() => {
                if !dictionaries.contains_key(&dictionary_id) {
                    return Err(Diagnostic::new(
                        OutcomeClass::Nonconforming,
                        ReasonCode::UnknownDictionary,
                        dictionary_id.to_string(),
                    ));
                }
            }
            PlanMode::Prefix { lookback } => {
                let group_id = frame.group_ref.ok_or_else(|| {
                    Diagnostic::new(
                        OutcomeClass::Nonconforming,
                        ReasonCode::InvalidGroupReference,
                        frame.chunk_id.to_string(),
                    )
                })?;
                let group = groups.get(&group_id).ok_or_else(|| {
                    Diagnostic::new(
                        OutcomeClass::Nonconforming,
                        ReasonCode::InvalidGroupReference,
                        group_id.to_string(),
                    )
                })?;
                if lookback != group.max_lookback {
                    return Err(Diagnostic::new(
                        OutcomeClass::Nonconforming,
                        ReasonCode::LookbackViolation,
                        format!("ChunkGroup {group_id} and TransformPlan disagree"),
                    ));
                }
                positions.entry(group_id).or_default().push(position);
            }
            _ => {
                return Err(Diagnostic::new(
                    OutcomeClass::Nonconforming,
                    ReasonCode::InvalidGroupReference,
                    frame.chunk_id.to_string(),
                ));
            }
        }
    }
    if positions.len() != groups.len() {
        return Err(Diagnostic::new(
            OutcomeClass::Nonconforming,
            ReasonCode::InvalidGroupReference,
            "every ChunkGroup must have prefix-coded members",
        ));
    }
    for (group_id, member_positions) in positions {
        if member_positions.len() < 2
            || member_positions
                .windows(2)
                .any(|pair| pair[1] != pair[0] + 1)
        {
            return Err(Diagnostic::new(
                OutcomeClass::Nonconforming,
                ReasonCode::InvalidGroupOrdering,
                group_id.to_string(),
            ));
        }
        let group = &groups[&group_id];
        let lookback = usize::try_from(group.max_lookback)
            .map_err(|_| resource("group lookback exceeds usize"))?;
        let mut maximum = 0_u64;
        for member_index in 0..member_positions.len() {
            let first = member_index.saturating_sub(lookback);
            let bytes = member_positions[first..member_index].iter().try_fold(
                0_u64,
                |total, position| {
                    total
                        .checked_add(frames[*position].logical_len)
                        .ok_or_else(|| resource("group access bytes exceed u64"))
                },
            )?;
            maximum = maximum.max(bytes);
        }
        if maximum != group.max_preceding_bytes {
            return Err(Diagnostic::new(
                OutcomeClass::Corrupt,
                ReasonCode::AccessCostMismatch,
                format!("ChunkGroup {group_id} access declaration mismatch"),
            ));
        }
    }
    Ok(())
}

fn physical_prefix(
    chunks: &BTreeMap<Digest, Chunk>,
    history: &[Digest],
    lookback: u32,
) -> Result<Vec<u8>> {
    let lookback = usize::try_from(lookback).map_err(|_| resource("lookback exceeds usize"))?;
    let first = history.len().saturating_sub(lookback);
    let preceding = &history[first..];
    let total = preceding.iter().try_fold(0_usize, |total, chunk_id| {
        total
            .checked_add(chunks[chunk_id].plaintext.len())
            .ok_or_else(|| resource("prefix length exceeds usize"))
    })?;
    let retained =
        total.min(usize::try_from(crate::codec::ZSTD_WINDOW_BYTES).unwrap_or(usize::MAX));
    let mut skip = total - retained;
    let mut prefix = Vec::with_capacity(retained);
    for chunk_id in preceding {
        let plaintext = &chunks[chunk_id].plaintext;
        if skip >= plaintext.len() {
            skip -= plaintext.len();
            continue;
        }
        prefix.extend_from_slice(&plaintext[skip..]);
        skip = 0;
    }
    Ok(prefix)
}

fn is_group_prerequisite(
    frames: &[EncodedChunkFrame<'_>],
    position: usize,
    groups: &BTreeMap<Digest, ChunkGroup>,
) -> bool {
    let Some(group_id) = frames[position].group_ref else {
        return false;
    };
    let lookback = usize::try_from(groups[&group_id].max_lookback).unwrap_or(0);
    frames[position + 1..frames.len().min(position.saturating_add(lookback + 1))]
        .iter()
        .any(|frame| frame.group_ref == Some(group_id))
}

fn validate_or_rebuild_index(
    section: Option<&SectionView<'_>>,
    rebuilt: &BTreeMap<Digest, ChunkLocation>,
) -> (Index, IndexStatus) {
    match section {
        None => (
            Index {
                present: false,
                valid: false,
                chunks: rebuilt.clone(),
                status: "absent; rebuilt from CHUNK_DATA".to_owned(),
            },
            IndexStatus::RebuiltAbsent,
        ),
        Some(section) if !section.digest_valid => invalid_index(rebuilt, "digest mismatch"),
        Some(section) => match decode_index(section.payload) {
            Ok(index) if index == *rebuilt => (
                Index {
                    present: true,
                    valid: true,
                    chunks: rebuilt.clone(),
                    status: "present and valid".to_owned(),
                },
                IndexStatus::PresentValid,
            ),
            Ok(_) => invalid_index(rebuilt, "locator mismatch"),
            Err(_) => invalid_index(rebuilt, "noncanonical or malformed"),
        },
    }
}

fn invalid_index(rebuilt: &BTreeMap<Digest, ChunkLocation>, reason: &str) -> (Index, IndexStatus) {
    (
        Index {
            present: true,
            valid: false,
            chunks: rebuilt.clone(),
            status: format!("invalid ({reason}); rebuilt from CHUNK_DATA"),
        },
        IndexStatus::RebuiltInvalid,
    )
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SectionLocation {
    offset: u64,
    len: u64,
}

fn absolute_section(location: SectionLocation) -> SectionLocation {
    SectionLocation {
        offset: PREAMBLE_LEN + location.offset,
        len: location.len,
    }
}

fn append_section(
    body: &mut Vec<u8>,
    kind: SectionKind,
    payload: &[u8],
    extended: bool,
) -> Result<SectionLocation> {
    let offset = u64_len(body)?;
    body.extend_from_slice(&SECTION_MAGIC);
    body.extend_from_slice(&section_id(kind, extended)?.to_be_bytes());
    body.extend_from_slice(&SECTION_VERSION.to_be_bytes());
    body.extend_from_slice(&0_u32.to_be_bytes());
    body.extend_from_slice(&0_u32.to_be_bytes());
    body.extend_from_slice(&u64_len(payload)?.to_be_bytes());
    body.extend_from_slice(sha256_exact(payload).as_bytes());
    body.extend_from_slice(&[0; 8]);
    body.extend_from_slice(payload);
    Ok(SectionLocation {
        offset,
        len: SECTION_HEADER_LEN
            .checked_add(u64_len(payload)?)
            .ok_or_else(|| resource("section length overflow"))?,
    })
}

fn encode_preamble(descriptor: &ArchiveDescriptor, footer_offset: u64) -> Result<Vec<u8>> {
    let mut bytes = vec![0_u8; usize::try_from(PREAMBLE_LEN).unwrap_or(256)];
    bytes[0..8].copy_from_slice(&MAGIC);
    bytes[8..10].copy_from_slice(&descriptor.format_major.to_be_bytes());
    bytes[10..12].copy_from_slice(&descriptor.format_minor.to_be_bytes());
    bytes[12..16].copy_from_slice(&u32::try_from(PREAMBLE_LEN).unwrap_or(256).to_be_bytes());
    bytes[16..24].copy_from_slice(&descriptor.features.incompat.to_be_bytes());
    bytes[24..32].copy_from_slice(&descriptor.features.read_only_compat.to_be_bytes());
    bytes[32..40].copy_from_slice(&descriptor.features.compat.to_be_bytes());
    let checksum = sha256_exact(&bytes[16..40]);
    bytes[40..72].copy_from_slice(checksum.as_bytes());
    bytes[72] = 1;
    bytes[73] = 1;
    bytes[74] = u8::from(descriptor.budget_declared);
    bytes[76..84].copy_from_slice(&descriptor.decode.window_bytes.to_be_bytes());
    bytes[84..92].copy_from_slice(&descriptor.decode.working_set_bytes.to_be_bytes());
    bytes[92..96].copy_from_slice(&descriptor.decode.flags.to_be_bytes());
    encode_budget(&mut bytes[96..160], descriptor.budget);
    bytes[176..184].copy_from_slice(&footer_offset.to_be_bytes());
    Ok(bytes)
}

fn encode_budget(bytes: &mut [u8], budget: ResourceBudget) {
    let values = [
        budget.entry_count,
        budget.total_logical_bytes,
        budget.max_single_entry_logical_bytes,
        budget.max_expansion_ratio_milli,
        budget.chunk_count,
        budget.max_path_depth,
        budget.max_metadata_bytes,
        budget.max_key_derivation_cost,
    ];
    for (slot, value) in bytes.chunks_exact_mut(8).zip(values) {
        slot.copy_from_slice(&value.to_be_bytes());
    }
}

#[derive(Clone, Copy, Debug)]
struct Preamble {
    version: FormatVersion,
    features: FeatureSet,
    budget_declared: bool,
    budget: ResourceBudget,
    decode: DecodeRequirements,
    footer_hint: u64,
}

fn decode_preamble(bytes: &[u8]) -> Result<Preamble> {
    if bytes.len() < 8 || bytes[..8] != MAGIC {
        return Err(Diagnostic::new(
            OutcomeClass::Nonconforming,
            ReasonCode::BadMagic,
            "Entrybound magic mismatch",
        ));
    }
    if bytes.len() < usize::try_from(PREAMBLE_LEN).unwrap_or(256) {
        return Err(Diagnostic::new(
            OutcomeClass::Truncated,
            ReasonCode::TruncatedFooter,
            "archive ends inside its fixed preamble",
        ));
    }
    let preamble = &bytes[..usize::try_from(PREAMBLE_LEN).unwrap_or(256)];
    let version = FormatVersion {
        major: u16::from_be_bytes(exact(&preamble[8..10])?),
        minor: u16::from_be_bytes(exact(&preamble[10..12])?),
    };
    if version != FormatVersion::BOOTSTRAP {
        return Err(Diagnostic::new(
            OutcomeClass::Unsupported,
            ReasonCode::UnsupportedVersion,
            format!(
                "unsupported ECF version {}.{}",
                version.major, version.minor
            ),
        ));
    }
    if u32::from_be_bytes(exact(&preamble[12..16])?) != u32::try_from(PREAMBLE_LEN).unwrap_or(256) {
        return Err(noncanonical("preamble length is not canonical"));
    }
    let features = FeatureSet {
        incompat: u64::from_be_bytes(exact(&preamble[16..24])?),
        read_only_compat: u64::from_be_bytes(exact(&preamble[24..32])?),
        compat: u64::from_be_bytes(exact(&preamble[32..40])?),
    };
    if sha256_exact(&preamble[16..40]).as_bytes() != &preamble[40..72] {
        return Err(integrity(
            ReasonCode::SectionDigestMismatch,
            "feature bitmap checksum mismatch",
        ));
    }
    if features.incompat & !SUPPORTED_INCOMPAT_FEATURES != 0 {
        return Err(Diagnostic::new(
            OutcomeClass::Unsupported,
            ReasonCode::UnsupportedRequiredFeature,
            format!(
                "unknown incompat feature bits {:016x}",
                features.incompat & !SUPPORTED_INCOMPAT_FEATURES
            ),
        ));
    }
    if preamble[72] != 1 || preamble[73] != 1 {
        return Err(Diagnostic::new(
            OutcomeClass::Unsupported,
            ReasonCode::UnsupportedRequiredFeature,
            "only Complete INDEXED bootstrap archives are supported",
        ));
    }
    if !matches!(preamble[74], 0 | 1)
        || preamble[75] != 0
        || preamble[184..].iter().any(|b| *b != 0)
    {
        return Err(noncanonical(
            "preamble booleans and reserved bytes are not canonical",
        ));
    }
    let budget = decode_budget(&preamble[96..160])?;
    if preamble[160..176].iter().any(|byte| *byte != 0) {
        return Err(noncanonical(
            "STREAM and hostility fields must be zero in INDEXED bootstrap",
        ));
    }
    Ok(Preamble {
        version,
        features,
        budget_declared: preamble[74] == 1,
        budget,
        decode: DecodeRequirements {
            window_bytes: u64::from_be_bytes(exact(&preamble[76..84])?),
            working_set_bytes: u64::from_be_bytes(exact(&preamble[84..92])?),
            flags: u32::from_be_bytes(exact(&preamble[92..96])?),
        },
        footer_hint: u64::from_be_bytes(exact(&preamble[176..184])?),
    })
}

fn decode_budget(bytes: &[u8]) -> Result<ResourceBudget> {
    let mut values = [0_u64; 8];
    for (value, slot) in values.iter_mut().zip(bytes.chunks_exact(8)) {
        *value = u64::from_be_bytes(exact(slot)?);
    }
    Ok(ResourceBudget {
        entry_count: values[0],
        total_logical_bytes: values[1],
        max_single_entry_logical_bytes: values[2],
        max_expansion_ratio_milli: values[3],
        chunk_count: values[4],
        max_path_depth: values[5],
        max_metadata_bytes: values[6],
        max_key_derivation_cost: values[7],
    })
}

fn encode_footer(
    total_len: u64,
    descriptor: SectionLocation,
    manifest: SectionLocation,
    entry_count: u64,
    total_logical: u64,
    preamble_digest: Digest,
) -> Vec<u8> {
    let mut bytes = vec![0_u8; usize::try_from(FOOTER_LEN).unwrap_or(128)];
    bytes[0..8].copy_from_slice(&FOOTER_MAGIC);
    bytes[8..16].copy_from_slice(&total_len.to_be_bytes());
    bytes[16..24].copy_from_slice(&descriptor.offset.to_be_bytes());
    bytes[24..32].copy_from_slice(&descriptor.len.to_be_bytes());
    bytes[32..40].copy_from_slice(&manifest.offset.to_be_bytes());
    bytes[40..48].copy_from_slice(&manifest.len.to_be_bytes());
    bytes[48..56].copy_from_slice(&entry_count.to_be_bytes());
    bytes[56..64].copy_from_slice(&total_logical.to_be_bytes());
    bytes[64..96].copy_from_slice(preamble_digest.as_bytes());
    bytes
}

#[derive(Clone, Copy, Debug)]
struct Footer {
    offset: u64,
    descriptor: SectionLocation,
    manifest: SectionLocation,
    entry_count: u64,
    total_logical: u64,
}

fn decode_footer(bytes: &[u8], _preamble: &Preamble) -> Result<Footer> {
    let minimum = PREAMBLE_LEN
        .checked_add(FOOTER_LEN)
        .ok_or_else(|| resource("minimum container length overflow"))?;
    if u64_len(bytes)? < minimum {
        return Err(Diagnostic::new(
            OutcomeClass::Truncated,
            ReasonCode::TruncatedFooter,
            "archive is too short to contain the fixed footer",
        ));
    }
    let footer_offset = bytes.len() - usize::try_from(FOOTER_LEN).unwrap_or(128);
    let footer = &bytes[footer_offset..];
    if footer[..8] != FOOTER_MAGIC {
        return Err(Diagnostic::new(
            OutcomeClass::Truncated,
            ReasonCode::TruncatedFooter,
            "fixed footer magic is absent",
        ));
    }
    let declared = u64::from_be_bytes(exact(&footer[8..16])?);
    let actual = u64_len(bytes)?;
    if declared != actual {
        return Err(Diagnostic::new(
            if declared > actual {
                OutcomeClass::Truncated
            } else {
                OutcomeClass::Corrupt
            },
            ReasonCode::IncorrectTotalLength,
            format!("declared {declared} bytes, found {actual}"),
        ));
    }
    if sha256_exact(&bytes[..usize::try_from(PREAMBLE_LEN).unwrap_or(256)]).as_bytes()
        != &footer[64..96]
    {
        return Err(integrity(
            ReasonCode::FooterBindingMismatch,
            "footer preamble binding mismatch",
        ));
    }
    if footer[96..].iter().any(|byte| *byte != 0) {
        return Err(noncanonical("footer reserved bytes must be zero"));
    }
    Ok(Footer {
        offset: u64::try_from(footer_offset).map_err(|_| resource("footer offset exceeds u64"))?,
        descriptor: SectionLocation {
            offset: u64::from_be_bytes(exact(&footer[16..24])?),
            len: u64::from_be_bytes(exact(&footer[24..32])?),
        },
        manifest: SectionLocation {
            offset: u64::from_be_bytes(exact(&footer[32..40])?),
            len: u64::from_be_bytes(exact(&footer[40..48])?),
        },
        entry_count: u64::from_be_bytes(exact(&footer[48..56])?),
        total_logical: u64::from_be_bytes(exact(&footer[56..64])?),
    })
}

#[derive(Clone, Copy, Debug)]
struct SectionView<'a> {
    kind: SectionKind,
    location: SectionLocation,
    payload: &'a [u8],
    digest_valid: bool,
}

fn decode_sections<'a>(
    bytes: &'a [u8],
    footer: &Footer,
    extended: bool,
) -> Result<Vec<SectionView<'a>>> {
    let mut sections = Vec::new();
    let mut cursor = usize::try_from(PREAMBLE_LEN).unwrap_or(256);
    let footer_offset =
        usize::try_from(footer.offset).map_err(|_| resource("footer offset too large"))?;
    let mut expected_id = 1_u16;
    let mut seen = BTreeSet::new();
    while cursor < footer_offset {
        let header_len = usize::try_from(SECTION_HEADER_LEN).unwrap_or(64);
        if footer_offset - cursor < header_len {
            return Err(structure("section header is truncated"));
        }
        let header = &bytes[cursor..cursor + header_len];
        if header[..4] != SECTION_MAGIC {
            return Err(structure("section magic mismatch"));
        }
        let id = u16::from_be_bytes(exact(&header[4..6])?);
        let kind = section_kind(id, extended)?;
        if id != expected_id {
            return Err(noncanonical(
                "sections are missing, duplicated, or out of canonical order",
            ));
        }
        expected_id += 1;
        if !seen.insert(id) {
            return Err(Diagnostic::new(
                OutcomeClass::Nonconforming,
                ReasonCode::DuplicateSemanticDeclaration,
                "duplicate section declaration",
            ));
        }
        if u16::from_be_bytes(exact(&header[6..8])?) != SECTION_VERSION
            || header[8..16] != [0; 8]
            || header[56..64] != [0; 8]
        {
            return Err(noncanonical(
                "section version, flags, or reserved bytes are invalid",
            ));
        }
        let payload_len = usize::try_from(u64::from_be_bytes(exact(&header[16..24])?))
            .map_err(|_| resource("section length exceeds usize"))?;
        let payload_start = cursor + header_len;
        let payload_end = payload_start
            .checked_add(payload_len)
            .ok_or_else(|| resource("section length overflow"))?;
        if payload_end > footer_offset {
            return Err(structure("section payload extends into the footer"));
        }
        let payload = &bytes[payload_start..payload_end];
        let digest_valid = sha256_exact(payload).as_bytes() == &header[24..56];
        if !digest_valid && kind != SectionKind::Index {
            return Err(integrity(
                ReasonCode::SectionDigestMismatch,
                format!("{:?} section", kind),
            ));
        }
        sections.push(SectionView {
            kind,
            location: SectionLocation {
                offset: u64::try_from(cursor)
                    .map_err(|_| resource("section offset exceeds u64"))?,
                len: SECTION_HEADER_LEN
                    .checked_add(
                        u64::try_from(payload_len).map_err(|_| resource("section too large"))?,
                    )
                    .ok_or_else(|| resource("section length overflow"))?,
            },
            payload,
            digest_valid,
        });
        cursor = payload_end;
    }
    let required_end = if extended { 8 } else { 6 };
    if expected_id != required_end && expected_id != required_end + 1 {
        return Err(structure("required bootstrap sections are missing"));
    }
    Ok(sections)
}

fn required_section<'a>(
    sections: &[SectionView<'a>],
    kind: SectionKind,
) -> Result<SectionView<'a>> {
    sections
        .iter()
        .copied()
        .find(|section| section.kind == kind)
        .ok_or_else(|| structure(format!("missing required {:?} section", kind)))
}

fn validate_actuals(
    archive: &Archive,
    footer: &Footer,
    manifest_len: u64,
    rebuilt_index: &BTreeMap<Digest, ChunkLocation>,
) -> Result<()> {
    let entry_count =
        u64::try_from(archive.entry_set.len()).map_err(|_| resource("entry count exceeds u64"))?;
    let total_logical = archive.total_logical_size()?;
    if footer.entry_count != entry_count || footer.total_logical != total_logical {
        return Err(structure(
            "footer actual totals disagree with authoritative records",
        ));
    }
    let budget = archive.descriptor.budget;
    let max_single = archive
        .content_store
        .objects
        .values()
        .map(|object| {
            object.chunks.iter().try_fold(0_u64, |total, chunk_ref| {
                let chunk = archive
                    .content_store
                    .chunks
                    .get(&chunk_ref.chunk_id)
                    .ok_or_else(|| structure("ContentObject references an unknown Chunk"))?;
                total
                    .checked_add(chunk.logical_len)
                    .ok_or_else(|| resource("ContentObject logical size overflow"))
            })
        })
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .max()
        .unwrap_or(0);
    let expansion = maximum_expansion_ratio(&archive.content_store.chunks, rebuilt_index)?;
    let decode = aggregate_archive_decode_requirements(
        &archive.transform_plans,
        &archive.content_store.dictionaries,
        &archive.content_store.chunk_groups,
    )?;
    if entry_count > budget.entry_count
        || total_logical > budget.total_logical_bytes
        || max_single > budget.max_single_entry_logical_bytes
        || u64::try_from(archive.content_store.chunks.len()).unwrap_or(u64::MAX)
            > budget.chunk_count
        || archive
            .entry_set
            .entries()
            .iter()
            .map(|entry| u64::try_from(entry.path().depth()).unwrap_or(u64::MAX))
            .max()
            .unwrap_or(0)
            > budget.max_path_depth
        || manifest_len > budget.max_metadata_bytes
        || expansion > budget.max_expansion_ratio_milli
        || budget.max_key_derivation_cost != 0
        || archive.descriptor.decode != decode
    {
        return Err(Diagnostic::new(
            OutcomeClass::Corrupt,
            ReasonCode::ResourceLimit,
            "decoded actuals exceed declared archive bounds",
        ));
    }
    Ok(())
}

fn section_id(kind: SectionKind, extended: bool) -> Result<u16> {
    match (extended, kind) {
        (_, SectionKind::Descriptor) => Ok(1),
        (_, SectionKind::TransformPlans) => Ok(2),
        (false, SectionKind::ChunkData) => Ok(3),
        (false, SectionKind::ManifestRecords) => Ok(4),
        (false, SectionKind::Fidelity) => Ok(5),
        (false, SectionKind::Index) => Ok(6),
        (true, SectionKind::Dictionaries) => Ok(3),
        (true, SectionKind::ChunkGroups) => Ok(4),
        (true, SectionKind::ChunkData) => Ok(5),
        (true, SectionKind::ManifestRecords) => Ok(6),
        (true, SectionKind::Fidelity) => Ok(7),
        (true, SectionKind::Index) => Ok(8),
        _ => Err(structure(
            "section kind does not belong to selected feature schema",
        )),
    }
}

fn section_kind(value: u16, extended: bool) -> Result<SectionKind> {
    match (extended, value) {
        (_, 1) => Ok(SectionKind::Descriptor),
        (_, 2) => Ok(SectionKind::TransformPlans),
        (false, 3) => Ok(SectionKind::ChunkData),
        (false, 4) => Ok(SectionKind::ManifestRecords),
        (false, 5) => Ok(SectionKind::Fidelity),
        (false, 6) => Ok(SectionKind::Index),
        (true, 3) => Ok(SectionKind::Dictionaries),
        (true, 4) => Ok(SectionKind::ChunkGroups),
        (true, 5) => Ok(SectionKind::ChunkData),
        (true, 6) => Ok(SectionKind::ManifestRecords),
        (true, 7) => Ok(SectionKind::Fidelity),
        (true, 8) => Ok(SectionKind::Index),
        _ => Err(Diagnostic::new(
            OutcomeClass::Unsupported,
            ReasonCode::UnsupportedRequiredFeature,
            format!("unknown section type {value}"),
        )),
    }
}

fn digest(bytes: &[u8]) -> Result<Digest> {
    Ok(Digest::from_bytes(bytes.try_into().map_err(|_| {
        noncanonical("digest must contain exactly 32 bytes")
    })?))
}

fn exact<const N: usize>(bytes: &[u8]) -> Result<[u8; N]> {
    bytes
        .try_into()
        .map_err(|_| structure(format!("expected exactly {N} bytes")))
}

fn u64_len<T: AsRef<[u8]> + ?Sized>(value: &T) -> Result<u64> {
    u64::try_from(value.as_ref().len()).map_err(|_| resource("byte length exceeds u64"))
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
