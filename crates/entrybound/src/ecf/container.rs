use std::collections::{BTreeMap, BTreeSet};

use super::records::{
    DescriptorBody, DescriptorDeclarations, decode_chunk_groups, decode_conversion,
    decode_descriptor, decode_dictionaries, decode_fidelity, decode_index,
    decode_legacy_preservation, decode_manifest, decode_reconstruction_regions,
    decode_reconstruction_section, decode_transform_plans, decode_transform_plans_v2,
    decode_transform_plans_v3, encode_chunk_groups, encode_conversion, encode_descriptor,
    encode_dictionaries, encode_fidelity, encode_index, encode_legacy_preservation,
    encode_manifest, encode_reconstruction_regions, encode_reconstruction_section,
    encode_transform_plans, encode_transform_plans_v2, encode_transform_plans_v3,
};
use super::{
    CHUNK_FRAME_HEADER_LEN, CHUNK_FRAME_V2_HEADER_LEN, FEATURE_CODEC_TRANSFORM_V1,
    FEATURE_CONVERSION_PROVENANCE_V1, FEATURE_CROSS_FILE_COMPRESSION_V1,
    FEATURE_LEGACY_PRESERVATION_V1, FEATURE_RECONSTRUCTIVE_TRANSFORM_V1, FEATURE_STREAM_LAYOUT_V1,
    FEATURE_WHOLE_OBJECT_RECONSTRUCTION_V1, FOOTER_LEN, FORMAT_NAMESPACE, FormatVersion, MAGIC,
    PREAMBLE_LEN, SECTION_HEADER_LEN, SUPPORTED_INCOMPAT_FEATURES, SectionKind,
};
use crate::codec::{
    PlanMode, aggregate_archive_decode_requirements, decode_payload,
    decode_payload_with_dictionary, decode_payload_with_prefix, decode_payload_with_reconstruction,
    encode_payload, encode_payload_with_dictionary, encode_payload_with_prefix,
    encode_payload_with_reconstruction, plan_mode, validate_plans,
};
use crate::diagnostics::{Diagnostic, OutcomeClass, ReasonCode, Result};
use crate::eam::{
    Archive, ArchiveDescriptor, ArchiveRole, Chunk, ChunkGroup, ChunkLocation, ContentStore,
    DecodeRequirements, Dictionary, Digest, DigestAlgorithm, FeatureSet, IdentityProfile, Index,
    Layout, ReconstructionData, ResourceBudget, TransformPlan,
};
use crate::identity::{
    IdentitySet, apply_native_identities, physical_container_identity, sha256_exact,
};

const SECTION_MAGIC: [u8; 4] = *b"EBS1";
pub(super) const CHUNK_MAGIC: [u8; 4] = *b"EBCH";
pub(super) const FOOTER_MAGIC: [u8; 8] = [0x8e, b'E', b'B', b'F', b'\r', b'\n', 0x1a, b'\n'];
pub(super) const SECTION_VERSION: u16 = 1;

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
    /// STREAM layout has no Index by design. Chunk-frame locators are recorded
    /// while scanning, but no Index item exists and none is expected.
    NotApplicableStream,
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
    pub reconstruction_integrity: bool,
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
    let reconstructive = has_reconstructive_feature(archive.descriptor.features);
    let whole_object = has_whole_object_feature(archive.descriptor.features);
    for plan in &archive.transform_plans {
        let required = crate::codec::required_features(plan)?;
        if required & !archive.descriptor.features.incompat != 0 {
            return Err(Diagnostic::new(
                OutcomeClass::Unsupported,
                ReasonCode::UnsupportedRequiredFeature,
                format!(
                    "TransformPlan {} requires undeclared incompat feature bits {:016x}",
                    plan.plan_id,
                    required & !archive.descriptor.features.incompat
                ),
            ));
        }
    }
    let transform_steps = has_codec_transform_feature(archive.descriptor.features);
    let plans_payload = if whole_object {
        encode_transform_plans_v3(&archive.transform_plans)?
    } else if reconstructive {
        encode_transform_plans_v2(&archive.transform_plans)?
    } else {
        encode_transform_plans(&archive.transform_plans, transform_steps)?
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
    let (chunk_payload, relative_index) = encode_chunks(&archive, extended, whole_object)?;
    normalize_descriptor(
        &mut archive,
        Some(&relative_index),
        PhysicalDeclaration::INDEXED,
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
        declarations: None,
    })?;
    let manifest_payload = encode_manifest(&archive.entry_set, &archive.content_store.objects)?;
    let mut fidelity_payload = encode_fidelity(&archive.fidelity)?;
    if let Some(conversion) = &archive.conversion {
        fidelity_payload.extend_from_slice(&encode_conversion(conversion)?);
    }
    if let Some(preservation) = &archive.preservation {
        fidelity_payload.extend_from_slice(&encode_legacy_preservation(preservation)?);
    }

    let mut body = Vec::new();
    let descriptor = append_section(
        &mut body,
        SectionKind::Descriptor,
        &descriptor_payload,
        extended,
        reconstructive,
        whole_object,
    )?;
    append_section(
        &mut body,
        SectionKind::TransformPlans,
        &plans_payload,
        extended,
        reconstructive,
        whole_object,
    )?;
    if extended {
        append_section(
            &mut body,
            SectionKind::Dictionaries,
            &dictionaries_payload,
            extended,
            reconstructive,
            whole_object,
        )?;
        append_section(
            &mut body,
            SectionKind::ChunkGroups,
            &groups_payload,
            extended,
            reconstructive,
            whole_object,
        )?;
    }
    if reconstructive {
        append_section(
            &mut body,
            SectionKind::ReconstructionData,
            &reconstruction_payload,
            extended,
            reconstructive,
            whole_object,
        )?;
    }
    if whole_object {
        append_section(
            &mut body,
            SectionKind::ReconstructionRegions,
            &regions_payload,
            extended,
            reconstructive,
            whole_object,
        )?;
    }
    let chunk_section = append_section(
        &mut body,
        SectionKind::ChunkData,
        &chunk_payload,
        extended,
        reconstructive,
        whole_object,
    )?;
    let manifest = append_section(
        &mut body,
        SectionKind::ManifestRecords,
        &manifest_payload,
        extended,
        reconstructive,
        whole_object,
    )?;
    append_section(
        &mut body,
        SectionKind::Fidelity,
        &fidelity_payload,
        extended,
        reconstructive,
        whole_object,
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
        append_section(
            &mut body,
            SectionKind::Index,
            &index_payload,
            extended,
            reconstructive,
            whole_object,
        )?;
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

/// Canonical private pieces consumed by the encrypted INDEXED writer.
///
/// These are the same record/frame encoders as unencrypted INDEXED. Crypto
/// framing changes their visibility and physical organization, not EAM facts.
pub(crate) struct EncryptedPlainParts {
    pub(crate) archive: Archive,
    pub(crate) roots: crate::identity::NativeRoots,
    pub(crate) descriptor: Vec<u8>,
    pub(crate) transform_plans: Vec<Vec<u8>>,
    pub(crate) dictionaries: Vec<Vec<u8>>,
    pub(crate) chunk_groups: Vec<Vec<u8>>,
    pub(crate) reconstruction_data: Vec<Vec<u8>>,
    pub(crate) reconstruction_regions: Vec<Vec<u8>>,
    pub(crate) chunk_frames: Vec<Vec<u8>>,
    pub(crate) manifest: Vec<Vec<u8>>,
    pub(crate) fidelity: Vec<u8>,
}

/// Produces the authoritative canonical records/frames for crypto-v1.
pub(crate) fn prepare_encrypted_plain_parts(input: &Archive) -> Result<EncryptedPlainParts> {
    prepare_encrypted_plain_parts_version(input, 2)
}

/// Reproduces the pre-correction encrypted form only for generated
/// compatibility tests. No production writer can select Descriptor v1.
#[cfg(test)]
pub(crate) fn prepare_legacy_encrypted_plain_parts(input: &Archive) -> Result<EncryptedPlainParts> {
    prepare_encrypted_plain_parts_version(input, 1)
}

fn prepare_encrypted_plain_parts_version(
    input: &Archive,
    descriptor_version: u16,
) -> Result<EncryptedPlainParts> {
    if input.conversion.is_some() {
        return Err(Diagnostic::new(
            OutcomeClass::Unsupported,
            ReasonCode::UnsupportedRequiredFeature,
            "crypto-v1 does not yet carry conversion provenance; convert to unencrypted ECF",
        ));
    }
    input.validate()?;
    validate_plans(&input.transform_plans)?;
    validate_feature_model(input)?;
    let (mut archive, roots) = apply_native_identities(input)?;
    let extended = has_cross_file_feature(archive.descriptor.features);
    let reconstructive = has_reconstructive_feature(archive.descriptor.features);
    let whole_object = has_whole_object_feature(archive.descriptor.features);
    for plan in &archive.transform_plans {
        let required = crate::codec::required_features(plan)?;
        if required & !archive.descriptor.features.incompat != 0 {
            return Err(Diagnostic::new(
                OutcomeClass::Unsupported,
                ReasonCode::UnsupportedRequiredFeature,
                format!(
                    "TransformPlan {} requires undeclared features",
                    plan.plan_id
                ),
            ));
        }
    }
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
    let (chunks_payload, relative_index) = encode_chunks(&archive, extended, whole_object)?;
    normalize_descriptor(
        &mut archive,
        Some(&relative_index),
        PhysicalDeclaration::INDEXED,
    )?;
    let descriptor = encode_descriptor(&DescriptorBody {
        namespace: FORMAT_NAMESPACE.to_owned(),
        identity_profile: 1,
        digest_algorithm: 1,
        planner_id: archive.descriptor.planner_id.clone(),
        chunker_id: archive.descriptor.chunker_id.clone(),
        lai: roots.lai.0,
        pcr: roots.pcr.0,
        aux: roots.aux.0,
        declarations: match descriptor_version {
            1 => None,
            2 => Some(DescriptorDeclarations {
                decode: archive.descriptor.decode,
                budget: archive.descriptor.budget,
            }),
            _ => return Err(noncanonical("unsupported encrypted Descriptor version")),
        },
    })?;
    let manifest_payload = encode_manifest(&archive.entry_set, &archive.content_store.objects)?;
    let fidelity = encode_fidelity(&archive.fidelity)?;
    let header_len = usize::try_from(chunk_frame_header_len(extended))
        .map_err(|_| resource("Chunk frame header exceeds usize"))?;
    let mut chunk_frames = Vec::new();
    let mut cursor = 0usize;
    while cursor < chunks_payload.len() {
        let header_end = cursor
            .checked_add(header_len)
            .ok_or_else(|| resource("Chunk frame header offset overflow"))?;
        let header_bytes = chunks_payload
            .get(cursor..header_end)
            .ok_or_else(|| structure("Chunk frame header is truncated"))?;
        let header = parse_chunk_frame_header(header_bytes, extended, whole_object)?;
        let end = header_end
            .checked_add(
                usize::try_from(header.stored_len)
                    .map_err(|_| resource("Chunk frame payload exceeds usize"))?,
            )
            .ok_or_else(|| resource("Chunk frame extent overflow"))?;
        chunk_frames.push(
            chunks_payload
                .get(cursor..end)
                .ok_or_else(|| structure("Chunk frame payload is truncated"))?
                .to_vec(),
        );
        cursor = end;
    }
    Ok(EncryptedPlainParts {
        archive,
        roots,
        descriptor,
        transform_plans: split_canonical_records(&plans_payload)?,
        dictionaries: split_canonical_records(&dictionaries_payload)?,
        chunk_groups: split_canonical_records(&groups_payload)?,
        reconstruction_data: split_canonical_records(&reconstruction_payload)?,
        reconstruction_regions: split_canonical_records(&regions_payload)?,
        chunk_frames,
        manifest: split_canonical_records(&manifest_payload)?,
        fidelity,
    })
}

fn split_canonical_records(mut input: &[u8]) -> Result<Vec<Vec<u8>>> {
    let mut records = Vec::new();
    while !input.is_empty() {
        let (_, consumed) = crate::canonical::decode_record(input)?;
        records.push(input[..consumed].to_vec());
        input = &input[consumed..];
    }
    Ok(records)
}

pub(crate) struct EncryptedDecodedParts {
    pub(crate) ordinary_features: FeatureSet,
    pub(crate) descriptor_version: u16,
    pub(crate) descriptor: Vec<u8>,
    pub(crate) transform_plans: Vec<Vec<u8>>,
    pub(crate) dictionaries: Vec<Vec<u8>>,
    pub(crate) chunk_groups: Vec<Vec<u8>>,
    pub(crate) reconstruction_data: Vec<Vec<u8>>,
    pub(crate) reconstruction_regions: Vec<Vec<u8>>,
    pub(crate) chunk_frames: Vec<Vec<u8>>,
    pub(crate) manifest: Vec<Vec<u8>>,
    pub(crate) fidelity: Vec<u8>,
    pub(crate) entry_count: u64,
    pub(crate) total_logical: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct PrivateDescriptorDeclaration {
    pub(crate) version: u16,
    pub(crate) decode: Option<DecodeRequirements>,
    pub(crate) budget: Option<ResourceBudget>,
}

/// Parses only the authenticated Descriptor declaration surface needed by the
/// crypto reader before it continues through dependent private objects.
pub(crate) fn private_descriptor_declaration(bytes: &[u8]) -> Result<PrivateDescriptorDeclaration> {
    let body = decode_descriptor(bytes)?;
    Ok(match body.declarations {
        Some(declarations) => PrivateDescriptorDeclaration {
            version: 2,
            decode: Some(declarations.decode),
            budget: Some(declarations.budget),
        },
        None => PrivateDescriptorDeclaration {
            version: 1,
            decode: None,
            budget: None,
        },
    })
}

/// Re-enters the ordinary ECF semantic reader after crypto authentication.
///
/// The temporary container exists only in memory. Every semantic record and
/// Chunk frame is the exact authenticated canonical byte sequence from EBPO;
/// this avoids a second EAM parser or serialization-shaped model.
pub(crate) fn open_encrypted_plain_parts(
    parts: EncryptedDecodedParts,
    policy: ResourceBudget,
    decode_policy: DecodeRequirements,
    encrypted_pci: crate::identity::PhysicalContainerIdentity,
) -> Result<OpenedArchive> {
    let descriptor_body = decode_descriptor(&parts.descriptor)?;
    let declarations = descriptor_body.declarations;
    let decoded_descriptor_version = if declarations.is_some() { 2 } else { 1 };
    if decoded_descriptor_version != parts.descriptor_version {
        return Err(noncanonical(
            "authenticated Descriptor version disagrees with its crypto dispatch",
        ));
    }
    if descriptor_body.namespace != FORMAT_NAMESPACE
        || descriptor_body.identity_profile != 1
        || descriptor_body.digest_algorithm != 1
    {
        return Err(Diagnostic::new(
            OutcomeClass::Unsupported,
            ReasonCode::UnsupportedRequiredFeature,
            "unsupported encrypted Descriptor namespace, identity profile, or digest algorithm",
        ));
    }
    if let Some(declarations) = declarations {
        enforce_caller_policy(declarations.budget, policy)?;
        enforce_decode_policy(declarations.decode, decode_policy)?;
    }
    let extended = has_cross_file_feature(parts.ordinary_features);
    let reconstructive = has_reconstructive_feature(parts.ordinary_features);
    let whole_object = has_whole_object_feature(parts.ordinary_features);
    let plans_payload = concat_records(&parts.transform_plans)?;
    let dictionaries_payload = concat_records(&parts.dictionaries)?;
    let groups_payload = concat_records(&parts.chunk_groups)?;
    let reconstruction_payload = concat_records(&parts.reconstruction_data)?;
    let regions_payload = concat_records(&parts.reconstruction_regions)?;
    let chunks_payload = concat_records(&parts.chunk_frames)?;
    let manifest_payload = concat_records(&parts.manifest)?;

    let plans = if whole_object {
        decode_transform_plans_v3(&plans_payload)?
    } else if reconstructive {
        decode_transform_plans_v2(&plans_payload)?
    } else {
        decode_transform_plans(
            &plans_payload,
            has_codec_transform_feature(parts.ordinary_features),
        )?
    };
    let dictionaries = if extended {
        decode_dictionaries(&dictionaries_payload)?
    } else {
        BTreeMap::new()
    };
    let groups = if extended {
        decode_chunk_groups(&groups_payload)?
    } else {
        BTreeMap::new()
    };
    let decode = aggregate_archive_decode_requirements(&plans, &dictionaries, &groups)?;
    if declarations.is_some_and(|value| value.decode != decode) {
        return Err(Diagnostic::new(
            OutcomeClass::Corrupt,
            ReasonCode::ResourceLimit,
            "Descriptor v2 DecodeRequirements disagree with authenticated decoder structures",
        ));
    }
    enforce_decode_policy(decode, decode_policy)?;
    let descriptor_budget = declarations.map_or(
        ResourceBudget {
            max_key_derivation_cost: 0,
            ..policy
        },
        |value| value.budget,
    );
    let descriptor = ArchiveDescriptor {
        format_major: FormatVersion::BOOTSTRAP.major,
        format_minor: FormatVersion::BOOTSTRAP.minor,
        format_namespace: descriptor_body.namespace,
        features: parts.ordinary_features,
        layout: Layout::Indexed,
        role: ArchiveRole::Complete,
        budget_declared: true,
        stream_dedup_window: 0,
        budget: descriptor_budget,
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
    let mut body = Vec::new();
    let descriptor_location = append_section(
        &mut body,
        SectionKind::Descriptor,
        &parts.descriptor,
        extended,
        reconstructive,
        whole_object,
    )?;
    append_section(
        &mut body,
        SectionKind::TransformPlans,
        &plans_payload,
        extended,
        reconstructive,
        whole_object,
    )?;
    if extended {
        append_section(
            &mut body,
            SectionKind::Dictionaries,
            &dictionaries_payload,
            extended,
            reconstructive,
            whole_object,
        )?;
        append_section(
            &mut body,
            SectionKind::ChunkGroups,
            &groups_payload,
            extended,
            reconstructive,
            whole_object,
        )?;
    }
    if reconstructive {
        append_section(
            &mut body,
            SectionKind::ReconstructionData,
            &reconstruction_payload,
            extended,
            reconstructive,
            whole_object,
        )?;
    }
    if whole_object {
        append_section(
            &mut body,
            SectionKind::ReconstructionRegions,
            &regions_payload,
            extended,
            reconstructive,
            whole_object,
        )?;
    }
    append_section(
        &mut body,
        SectionKind::ChunkData,
        &chunks_payload,
        extended,
        reconstructive,
        whole_object,
    )?;
    let manifest_location = append_section(
        &mut body,
        SectionKind::ManifestRecords,
        &manifest_payload,
        extended,
        reconstructive,
        whole_object,
    )?;
    append_section(
        &mut body,
        SectionKind::Fidelity,
        &parts.fidelity,
        extended,
        reconstructive,
        whole_object,
    )?;
    let footer_offset = PREAMBLE_LEN
        .checked_add(u64_len(&body)?)
        .ok_or_else(|| resource("temporary authenticated container length overflow"))?;
    let total_len = footer_offset
        .checked_add(FOOTER_LEN)
        .ok_or_else(|| resource("temporary authenticated container length overflow"))?;
    let preamble = encode_preamble(&descriptor, footer_offset)?;
    let footer = encode_footer(
        total_len,
        absolute_section(descriptor_location),
        absolute_section(manifest_location),
        parts.entry_count,
        parts.total_logical,
        sha256_exact(&preamble),
    );
    let mut bytes = preamble;
    bytes.extend_from_slice(&body);
    bytes.extend_from_slice(&footer);
    let mut opened =
        open_with_limits_internal(&bytes, policy, decode_policy, declarations.is_some())?;
    let exact_budget = derived_budget(&opened.archive, &opened.archive.index.chunks)?;
    enforce_caller_policy(exact_budget, policy)?;
    if declarations.is_none() {
        opened.archive.descriptor.budget_declared = false;
        opened.archive.descriptor.budget = exact_budget;
    }
    opened.archive.descriptor.pci = Some(encrypted_pci.0);
    opened.report.identities.pci = encrypted_pci;
    Ok(opened)
}

fn concat_records(records: &[Vec<u8>]) -> Result<Vec<u8>> {
    let total = records.iter().try_fold(0usize, |total, record| {
        total
            .checked_add(record.len())
            .ok_or_else(|| resource("authenticated private collection exceeds usize"))
    })?;
    let mut bytes = Vec::with_capacity(total);
    for record in records {
        bytes.extend_from_slice(record);
    }
    Ok(bytes)
}

pub(super) fn decode_auxiliary_payload(
    payload: &[u8],
    conversion_feature: bool,
    preservation_feature: bool,
) -> Result<(
    crate::eam::FidelityReport,
    Option<crate::eam::ConversionProvenance>,
    Option<crate::eam::LegacyPreservation>,
)> {
    let (_, consumed) = crate::canonical::decode_record(payload)?;
    let fidelity = decode_fidelity(&payload[..consumed])?;
    let mut trailing = &payload[consumed..];
    let conversion = if conversion_feature {
        if trailing.is_empty() {
            return Err(noncanonical(
                "conversion-provenance-v1 requires one type-28 record",
            ));
        }
        let (_, conversion_len) = crate::canonical::decode_record(trailing)?;
        let value = decode_conversion(&trailing[..conversion_len])?;
        trailing = &trailing[conversion_len..];
        Some(value)
    } else {
        if !trailing.is_empty() {
            return Err(noncanonical(
                "Fidelity payload has an undeclared trailing semantic record",
            ));
        }
        None
    };
    let preservation = if preservation_feature {
        if !conversion_feature || trailing.is_empty() {
            return Err(noncanonical(
                "legacy-preservation-v1 requires conversion provenance and one type-30 record",
            ));
        }
        let value = decode_legacy_preservation(trailing)?;
        trailing = &[];
        Some(value)
    } else {
        None
    };
    if !trailing.is_empty() {
        return Err(noncanonical(
            "Fidelity payload has an undeclared trailing semantic record",
        ));
    }
    Ok((fidelity, conversion, preservation))
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
    open_with_limits_internal(bytes, policy, decode_policy, false)
}

fn open_with_limits_internal(
    bytes: &[u8],
    policy: ResourceBudget,
    decode_policy: DecodeRequirements,
    allow_private_descriptor_v2: bool,
) -> Result<OpenedArchive> {
    let preamble = decode_preamble(bytes)?;
    if preamble.layout != Layout::Indexed {
        return Err(Diagnostic::new(
            OutcomeClass::Unsupported,
            ReasonCode::UnsupportedRequiredFeature,
            "this archive uses STREAM layout; open it with the sequential reader",
        ));
    }
    enforce_caller_policy(preamble.budget, policy)?;
    enforce_decode_policy(preamble.decode, decode_policy)?;
    let footer = decode_footer(bytes, &preamble)?;
    if preamble.footer_hint != footer.offset {
        return Err(noncanonical(
            "canonical INDEXED preamble footer hint must identify the fixed footer",
        ));
    }
    let extended = has_cross_file_feature(preamble.features);
    let reconstructive = has_reconstructive_feature(preamble.features);
    let whole_object = has_whole_object_feature(preamble.features);
    let sections = decode_sections(bytes, &footer, extended, reconstructive, whole_object)?;

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
    let (reconstruction_data, reconstruction_fallbacks) = if reconstructive {
        decode_reconstruction_section(
            required_section(&sections, SectionKind::ReconstructionData)?.payload,
        )?
    } else {
        (BTreeMap::new(), BTreeMap::new())
    };
    let (reconstruction_regions, reconstruction_audits) = if whole_object {
        decode_reconstruction_regions(
            required_section(&sections, SectionKind::ReconstructionRegions)?.payload,
        )?
    } else {
        (BTreeMap::new(), BTreeMap::new())
    };

    if descriptor_section.location != footer.descriptor
        || manifest_section.location != footer.manifest
    {
        return Err(structure(
            "footer authoritative locators do not match section framing",
        ));
    }

    let descriptor_body = decode_descriptor(descriptor_section.payload)?;
    if descriptor_body.declarations.is_some() != allow_private_descriptor_v2 {
        return Err(noncanonical(if allow_private_descriptor_v2 {
            "corrected encrypted archive requires Descriptor v2"
        } else {
            "Descriptor v2 is private to corrected encrypted INDEXED archives"
        }));
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
    let plans = if whole_object {
        decode_transform_plans_v3(plans_section.payload)?
    } else if reconstructive {
        decode_transform_plans_v2(plans_section.payload)?
    } else {
        decode_transform_plans(
            plans_section.payload,
            has_codec_transform_feature(preamble.features),
        )?
    };
    let (entries, objects) = decode_manifest(manifest_section.payload)?;
    let (chunks, physical_order, rebuilt_index) = decode_chunks(
        chunks_section.payload,
        chunks_section
            .location
            .offset
            .checked_add(SECTION_HEADER_LEN)
            .ok_or_else(|| resource("chunk payload offset overflow"))?,
        DecodeDependencies {
            plans: &plans,
            dictionaries: &dictionaries,
            reconstruction_data: &reconstruction_data,
            groups: &chunk_groups,
        },
        ChunkDecodeModel {
            declared_budget: preamble.budget,
            extended,
            whole_object,
            objects: &objects,
            regions: &reconstruction_regions,
        },
    )?;
    let (fidelity, conversion, preservation) = decode_auxiliary_payload(
        fidelity_section.payload,
        preamble.features.incompat & FEATURE_CONVERSION_PROVENANCE_V1 != 0,
        preamble.features.incompat & FEATURE_LEGACY_PRESERVATION_V1 != 0,
    )?;

    let index_section = sections
        .iter()
        .find(|section| section.kind == SectionKind::Index);
    let (index, index_status) = validate_or_rebuild_index(index_section, &rebuilt_index);
    let (budget_declared, budget, decode) = descriptor_body.declarations.map_or(
        (preamble.budget_declared, preamble.budget, preamble.decode),
        |declarations| (true, declarations.budget, declarations.decode),
    );
    let descriptor = ArchiveDescriptor {
        format_major: preamble.version.major,
        format_minor: preamble.version.minor,
        format_namespace: descriptor_body.namespace,
        features: preamble.features,
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
    let archive = Archive {
        descriptor,
        entry_set: entries,
        content_store: ContentStore {
            objects,
            chunks,
            dictionaries,
            reconstruction_data,
            reconstruction_fallbacks,
            reconstruction_regions,
            reconstruction_audits,
            chunk_groups,
            physical_order,
        },
        transform_plans: plans,
        fidelity,
        conversion,
        preservation,
        index,
    };
    archive.validate()?;
    validate_actuals(&archive, &footer, &rebuilt_index)?;

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
            reconstruction_integrity: true,
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
                IndexStatus::PresentValid | IndexStatus::NotApplicableStream => None,
                IndexStatus::RebuiltAbsent => Some(ReasonCode::IndexAbsentRebuilt),
                IndexStatus::RebuiltInvalid => Some(ReasonCode::IndexInvalidRebuilt),
            },
            identities,
        },
    })
}

/// Reads only the fixed preamble to learn which layout a container uses.
///
/// A consumer must choose its reader from the archive's declared layout, never
/// from whether the source it happened to receive is seekable.
pub fn peek_layout(bytes: &[u8]) -> Result<Layout> {
    Ok(decode_preamble(bytes)?.layout)
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

pub(crate) fn enforce_caller_policy(
    declared: ResourceBudget,
    policy: ResourceBudget,
) -> Result<()> {
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

pub(crate) fn enforce_decode_policy(
    declared: DecodeRequirements,
    policy: DecodeRequirements,
) -> Result<()> {
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

pub(super) fn has_cross_file_feature(features: FeatureSet) -> bool {
    features.incompat & FEATURE_CROSS_FILE_COMPRESSION_V1 != 0
}

pub(super) fn has_codec_transform_feature(features: FeatureSet) -> bool {
    features.incompat & FEATURE_CODEC_TRANSFORM_V1 != 0
}

pub(super) fn has_reconstructive_feature(features: FeatureSet) -> bool {
    features.incompat & FEATURE_RECONSTRUCTIVE_TRANSFORM_V1 != 0
}

pub(super) fn has_whole_object_feature(features: FeatureSet) -> bool {
    features.incompat & FEATURE_WHOLE_OBJECT_RECONSTRUCTION_V1 != 0
}

pub(super) fn validate_feature_model(archive: &Archive) -> Result<()> {
    if archive.descriptor.features.incompat & !SUPPORTED_INCOMPAT_FEATURES != 0 {
        return Err(Diagnostic::new(
            OutcomeClass::Unsupported,
            ReasonCode::UnsupportedRequiredFeature,
            "archive declares unsupported required feature bits",
        ));
    }
    if archive.descriptor.features.incompat & crate::crypto::CRYPTO_FEATURES != 0 {
        return Err(Diagnostic::new(
            OutcomeClass::Nonconforming,
            ReasonCode::UnsupportedRequiredFeature,
            "crypto incompatibility features are not part of an unencrypted EAM declaration",
        ));
    }
    let declares_stream = archive.descriptor.features.incompat & FEATURE_STREAM_LAYOUT_V1 != 0;
    if declares_stream != (archive.descriptor.layout == Layout::Stream) {
        return Err(Diagnostic::new(
            OutcomeClass::Nonconforming,
            ReasonCode::UnsupportedRequiredFeature,
            "stream-layout-v1 must be declared exactly when the layout is STREAM",
        ));
    }
    let extended = has_cross_file_feature(archive.descriptor.features);
    let reconstructive = has_reconstructive_feature(archive.descriptor.features);
    let whole_object = has_whole_object_feature(archive.descriptor.features);
    if whole_object
        && archive.descriptor.features.incompat
            & (FEATURE_CROSS_FILE_COMPRESSION_V1
                | FEATURE_CODEC_TRANSFORM_V1
                | FEATURE_RECONSTRUCTIVE_TRANSFORM_V1)
            != (FEATURE_CROSS_FILE_COMPRESSION_V1
                | FEATURE_CODEC_TRANSFORM_V1
                | FEATURE_RECONSTRUCTIVE_TRANSFORM_V1)
    {
        return Err(Diagnostic::new(
            OutcomeClass::Unsupported,
            ReasonCode::UnsupportedRequiredFeature,
            "whole-object-reconstruction-v1 requires cross-file, codec-transform, and reconstructive-transform v1",
        ));
    }
    if reconstructive
        && archive.descriptor.features.incompat
            & (FEATURE_CROSS_FILE_COMPRESSION_V1 | FEATURE_CODEC_TRANSFORM_V1)
            != (FEATURE_CROSS_FILE_COMPRESSION_V1 | FEATURE_CODEC_TRANSFORM_V1)
    {
        return Err(Diagnostic::new(
            OutcomeClass::Unsupported,
            ReasonCode::UnsupportedRequiredFeature,
            "reconstructive-transform-v1 requires cross-file-compression-v1 and codec-transform-v1",
        ));
    }
    if !reconstructive
        && (!archive.content_store.reconstruction_data.is_empty()
            || !archive.content_store.reconstruction_fallbacks.is_empty())
    {
        return Err(Diagnostic::new(
            OutcomeClass::Unsupported,
            ReasonCode::UnsupportedRequiredFeature,
            "ReconstructionData and ReconstructionFallback require reconstructive-transform-v1",
        ));
    }
    if !whole_object
        && (!archive.content_store.reconstruction_regions.is_empty()
            || !archive.content_store.reconstruction_audits.is_empty())
    {
        return Err(Diagnostic::new(
            OutcomeClass::Unsupported,
            ReasonCode::UnsupportedRequiredFeature,
            "ReconstructionRegion and v2 audit records require whole-object-reconstruction-v1",
        ));
    }
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
        && archive.descriptor.layout == Layout::Indexed
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

/// Physical declarations a writer fixes before emitting the preamble.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct PhysicalDeclaration {
    pub(super) layout: Layout,
    pub(super) budget_declared: bool,
    pub(super) stream_dedup_window: u64,
}

impl PhysicalDeclaration {
    pub(super) const INDEXED: Self = Self {
        layout: Layout::Indexed,
        budget_declared: true,
        stream_dedup_window: 0,
    };
}

/// Normalizes the descriptor a writer is about to emit.
///
/// `relative_index` supplies the stored lengths used to derive the declared
/// resource budget. A producer that declares no pre-payload budget passes
/// `None`; the descriptor then carries a zero budget and the reader bounds the
/// pass with its own policy instead.
pub(super) fn normalize_descriptor(
    archive: &mut Archive,
    relative_index: Option<&BTreeMap<Digest, ChunkLocation>>,
    declaration: PhysicalDeclaration,
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
    archive.descriptor.layout = declaration.layout;
    archive.descriptor.role = ArchiveRole::Complete;
    archive.descriptor.budget_declared = declaration.budget_declared;
    archive.descriptor.stream_dedup_window = declaration.stream_dedup_window;
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
    archive.descriptor.budget = match relative_index {
        Some(index) => derived_budget(archive, index)?,
        None => ResourceBudget::default(),
    };
    Ok(())
}

pub(super) fn derived_budget(
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
    let mut metadata_bound = u64_len(&encode_manifest(
        &archive.entry_set,
        &archive.content_store.objects,
    )?)?;
    if let Some(conversion) = &archive.conversion {
        metadata_bound = metadata_bound
            .checked_add(u64_len(&encode_conversion(conversion)?)?)
            .ok_or_else(|| resource("metadata and conversion-provenance budget overflow"))?;
    }
    if let Some(preservation) = &archive.preservation {
        metadata_bound = metadata_bound
            .checked_add(u64_len(&encode_legacy_preservation(preservation)?)?)
            .ok_or_else(|| resource("metadata and legacy-preservation budget overflow"))?;
    }
    if has_reconstructive_feature(archive.descriptor.features) {
        metadata_bound = metadata_bound
            .checked_add(u64_len(&encode_reconstruction_section(
                &archive.content_store.reconstruction_data,
                &archive.content_store.reconstruction_fallbacks,
            )?)?)
            .ok_or_else(|| resource("metadata and reconstruction budget overflow"))?;
    }
    if has_whole_object_feature(archive.descriptor.features) {
        metadata_bound = metadata_bound
            .checked_add(u64_len(&encode_reconstruction_regions(
                &archive.content_store.reconstruction_regions,
                &archive.content_store.reconstruction_audits,
            )?)?)
            .ok_or_else(|| resource("metadata and region budget overflow"))?;
    }
    Ok(ResourceBudget {
        entry_count: u64::try_from(archive.entry_set.len())
            .map_err(|_| resource("entry count exceeds u64"))?,
        total_logical_bytes: archive.total_logical_size()?,
        max_single_entry_logical_bytes: max_single,
        max_expansion_ratio_milli: maximum_expansion_ratio(archive, relative_index)?,
        chunk_count: u64::try_from(archive.content_store.chunks.len())
            .map_err(|_| resource("Chunk count exceeds u64"))?,
        max_path_depth: u64::try_from(max_path_depth)
            .map_err(|_| resource("path depth exceeds u64"))?,
        max_metadata_bytes: metadata_bound,
        max_key_derivation_cost: 0,
    })
}

pub(super) fn maximum_expansion_ratio(
    archive: &Archive,
    index: &BTreeMap<Digest, ChunkLocation>,
) -> Result<u64> {
    let mut maximum = 1_000_u64;
    let region_owned = region_owned_chunk_ids(
        &archive.content_store.objects,
        &archive.content_store.reconstruction_regions,
    )?;
    for (chunk_id, chunk) in &archive.content_store.chunks {
        if region_owned.contains(chunk_id) {
            continue;
        }
        let stored_len = index
            .get(chunk_id)
            .ok_or_else(|| structure("encoded Chunk is absent from the rebuilt Index"))?
            .stored_len;
        maximum = maximum.max(expansion_ratio(chunk.logical_len, stored_len)?);
    }
    for region in archive.content_store.reconstruction_regions.values() {
        maximum = maximum.max(expansion_ratio(
            region.logical_bytes,
            u64::try_from(region.representation.len())
                .map_err(|_| resource("region representation exceeds u64"))?,
        )?);
    }
    Ok(maximum)
}

pub(super) fn expansion_ratio(logical_len: u64, stored_len: u64) -> Result<u64> {
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

/// Encodes CHUNK_DATA frames one at a time in the archive's physical order.
///
/// Both layouts drive their physical Chunk bytes through this single encoder,
/// so a frame's stored bytes are identical whether it lands in an INDEXED
/// CHUNK_DATA section or a STREAM `CHUNK_FRAME` item. Bounded-lookback groups
/// depend on frame order, so callers must present chunks in `physical_order`.
pub(super) struct ChunkFrameEncoder<'a> {
    archive: &'a Archive,
    plans: BTreeMap<u64, &'a TransformPlan>,
    region_owned: BTreeSet<Digest>,
    group_history: BTreeMap<Digest, Vec<Digest>>,
    extended: bool,
    whole_object: bool,
}

impl<'a> ChunkFrameEncoder<'a> {
    pub(super) fn new(archive: &'a Archive, extended: bool, whole_object: bool) -> Result<Self> {
        let region_owned = region_owned_chunk_ids(
            &archive.content_store.objects,
            &archive.content_store.reconstruction_regions,
        )?;
        if whole_object {
            verify_region_representations(archive)?;
        }
        Ok(Self {
            archive,
            plans: archive
                .transform_plans
                .iter()
                .map(|plan| (plan.plan_id, plan))
                .collect(),
            region_owned,
            group_history: BTreeMap::new(),
            extended,
            whole_object,
        })
    }

    /// Encodes the next frame and advances bounded-lookback group history.
    pub(super) fn encode_next(&mut self, chunk_id: &Digest) -> Result<Vec<u8>> {
        let chunk = self
            .archive
            .content_store
            .chunks
            .get(chunk_id)
            .ok_or_else(|| {
                Diagnostic::new(
                    OutcomeClass::Nonconforming,
                    ReasonCode::UnknownChunk,
                    chunk_id.to_string(),
                )
            })?;
        if chunk.logical_len != u64_len(&chunk.plaintext)? {
            return Err(structure("Chunk has an inconsistent logical length"));
        }
        let is_region_owned = self.region_owned.contains(chunk_id);
        let stored = if is_region_owned {
            if !self.whole_object
                || chunk.plan_ref != crate::jpeg_reconstruction::REGION_MEMBER_PLAN_REF
                || chunk.group_ref.is_some()
            {
                return Err(Diagnostic::new(
                    OutcomeClass::Nonconforming,
                    ReasonCode::InvalidReconstructionRegion,
                    format!("Chunk {} has conflicting region ownership", chunk.chunk_id),
                ));
            }
            Vec::new()
        } else {
            self.encode_stored_payload(chunk)?
        };
        let mut frame = Vec::with_capacity(stored.len() + 96);
        frame.extend_from_slice(&CHUNK_MAGIC);
        frame.extend_from_slice(
            &(if self.whole_object {
                3_u16
            } else if self.extended {
                2_u16
            } else {
                SECTION_VERSION
            })
            .to_be_bytes(),
        );
        frame.extend_from_slice(&u16::from(is_region_owned).to_be_bytes());
        frame.extend_from_slice(&u64_len(&stored)?.to_be_bytes());
        frame.extend_from_slice(chunk.chunk_id.as_bytes());
        frame.extend_from_slice(&chunk.logical_len.to_be_bytes());
        frame.extend_from_slice(
            &(if is_region_owned {
                crate::jpeg_reconstruction::REGION_MEMBER_PLAN_REF
            } else {
                chunk.plan_ref
            })
            .to_be_bytes(),
        );
        if self.extended {
            frame.extend_from_slice(chunk.group_ref.unwrap_or(Digest::ZERO).as_bytes());
        }
        frame.extend_from_slice(&stored);
        if !is_region_owned && let Some(group_id) = chunk.group_ref {
            self.group_history
                .entry(group_id)
                .or_default()
                .push(chunk.chunk_id);
        }
        Ok(frame)
    }

    /// Returns the header width this encoder produces for every frame.
    pub(super) fn header_len(&self) -> u64 {
        if self.extended {
            CHUNK_FRAME_V2_HEADER_LEN
        } else {
            CHUNK_FRAME_HEADER_LEN
        }
    }

    fn encode_stored_payload(&mut self, chunk: &Chunk) -> Result<Vec<u8>> {
        let plan = self.plans.get(&chunk.plan_ref).ok_or_else(|| {
            Diagnostic::new(
                OutcomeClass::Unsupported,
                ReasonCode::UnknownTransformPlan,
                format!("Chunk {} uses plan {}", chunk.chunk_id, chunk.plan_ref),
            )
        })?;
        match plan_mode(plan)? {
            PlanMode::Independent if chunk.group_ref.is_none() => {
                if plan
                    .transforms
                    .iter()
                    .any(|step| step.reconstruction_ref.is_some())
                {
                    encode_payload_with_reconstruction(
                        plan,
                        &chunk.plaintext,
                        &self.archive.content_store.reconstruction_data,
                    )
                } else {
                    encode_payload(plan, &chunk.plaintext)
                }
            }
            PlanMode::Dictionary(dictionary_id) if chunk.group_ref.is_none() => {
                let dictionary = self
                    .archive
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
                encode_payload_with_dictionary(plan, &chunk.plaintext, dictionary)
            }
            PlanMode::Prefix { lookback } => {
                let group_id = chunk.group_ref.ok_or_else(|| {
                    Diagnostic::new(
                        OutcomeClass::Nonconforming,
                        ReasonCode::InvalidGroupReference,
                        format!("prefix Chunk {} has no group_ref", chunk.chunk_id),
                    )
                })?;
                let group = self
                    .archive
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
                    &self.archive.content_store.chunks,
                    self.group_history.entry(group_id).or_default(),
                    lookback,
                )?;
                encode_payload_with_prefix(plan, &chunk.plaintext, &prefix)
            }
            _ => Err(Diagnostic::new(
                OutcomeClass::Nonconforming,
                ReasonCode::InvalidGroupReference,
                format!(
                    "Chunk {} has a group incompatible with its plan",
                    chunk.chunk_id
                ),
            )),
        }
    }
}

fn encode_chunks(
    archive: &Archive,
    extended: bool,
    whole_object: bool,
) -> Result<(Vec<u8>, BTreeMap<Digest, ChunkLocation>)> {
    let mut encoder = ChunkFrameEncoder::new(archive, extended, whole_object)?;
    let header_len = encoder.header_len();
    let mut payload = Vec::new();
    let mut index = BTreeMap::new();
    for chunk_id in &archive.content_store.physical_order {
        let offset = u64_len(&payload)?;
        let frame = encoder.encode_next(chunk_id)?;
        let stored_len = u64_len(&frame)?
            .checked_sub(header_len)
            .ok_or_else(|| structure("Chunk frame is shorter than its header"))?;
        payload.extend_from_slice(&frame);
        index.insert(*chunk_id, ChunkLocation { offset, stored_len });
    }
    Ok((payload, index))
}

#[derive(Clone, Copy)]
pub(super) struct EncodedChunkFrame<'a> {
    pub(super) offset: u64,
    pub(super) chunk_id: Digest,
    pub(super) logical_len: u64,
    pub(super) plan_ref: u64,
    pub(super) group_ref: Option<Digest>,
    pub(super) stored: &'a [u8],
    pub(super) region_owned: bool,
}

type DecodedChunks = (
    BTreeMap<Digest, Chunk>,
    Box<[Digest]>,
    BTreeMap<Digest, ChunkLocation>,
);

#[derive(Clone, Copy)]
pub(super) struct DecodeDependencies<'a> {
    pub(super) plans: &'a [TransformPlan],
    pub(super) dictionaries: &'a BTreeMap<Digest, Dictionary>,
    pub(super) reconstruction_data: &'a BTreeMap<Digest, ReconstructionData>,
    pub(super) groups: &'a BTreeMap<Digest, ChunkGroup>,
}

#[derive(Clone, Copy)]
struct ChunkDecodeModel<'a> {
    declared_budget: ResourceBudget,
    extended: bool,
    whole_object: bool,
    objects: &'a BTreeMap<Digest, crate::eam::ContentObject>,
    regions: &'a BTreeMap<Digest, crate::eam::ReconstructionRegion>,
}

/// The authoritative fields a Chunk frame header declares exactly once.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct ChunkFrameHeader {
    pub(super) stored_len: u64,
    pub(super) chunk_id: Digest,
    pub(super) logical_len: u64,
    pub(super) plan_ref: u64,
    pub(super) group_ref: Option<Digest>,
    pub(super) region_owned: bool,
}

/// Returns the Chunk frame header width selected by the archive's features.
pub(super) const fn chunk_frame_header_len(extended: bool) -> u64 {
    if extended {
        CHUNK_FRAME_V2_HEADER_LEN
    } else {
        CHUNK_FRAME_HEADER_LEN
    }
}

/// Parses one Chunk frame header. Both layouts share this single authority so
/// they can never disagree about what a frame declares.
pub(super) fn parse_chunk_frame_header(
    header: &[u8],
    extended: bool,
    whole_object: bool,
) -> Result<ChunkFrameHeader> {
    if header[..4] != CHUNK_MAGIC {
        return Err(structure("Chunk frame magic mismatch"));
    }
    let expected_version = if whole_object {
        3
    } else if extended {
        2
    } else {
        SECTION_VERSION
    };
    let flags = u16::from_be_bytes(exact(&header[6..8])?);
    let region_owned = whole_object && flags == 1;
    if u16::from_be_bytes(exact(&header[4..6])?) != expected_version
        || (!whole_object && flags != 0)
        || (whole_object && flags > 1)
    {
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
    if region_owned
        && (stored_len != 0
            || plan_ref != crate::jpeg_reconstruction::REGION_MEMBER_PLAN_REF
            || group_ref.is_some())
    {
        return Err(noncanonical(
            "region-owned Chunk declaration has payload, plan, or group",
        ));
    }
    Ok(ChunkFrameHeader {
        stored_len,
        chunk_id,
        logical_len,
        plan_ref,
        group_ref,
        region_owned,
    })
}

/// Enforces the declared per-Chunk resource bounds shared by both layouts.
pub(super) fn enforce_chunk_bounds(
    header: &ChunkFrameHeader,
    declared_budget: ResourceBudget,
) -> Result<()> {
    if header.logical_len > declared_budget.max_single_entry_logical_bytes {
        return Err(Diagnostic::new(
            OutcomeClass::Corrupt,
            ReasonCode::ResourceLimit,
            format!(
                "Chunk {} exceeds the archive's declared logical bound",
                header.chunk_id
            ),
        ));
    }
    if !header.region_owned
        && expansion_ratio(header.logical_len, header.stored_len)?
            > declared_budget.max_expansion_ratio_milli
    {
        return Err(Diagnostic::new(
            OutcomeClass::Corrupt,
            ReasonCode::ResourceLimit,
            format!(
                "Chunk {} exceeds the archive's declared expansion bound",
                header.chunk_id
            ),
        ));
    }
    Ok(())
}

fn decode_chunks(
    payload: &[u8],
    absolute_payload_offset: u64,
    dependencies: DecodeDependencies<'_>,
    model: ChunkDecodeModel<'_>,
) -> Result<DecodedChunks> {
    let ChunkDecodeModel {
        declared_budget,
        extended,
        whole_object,
        objects,
        regions,
    } = model;
    let plans = dependencies
        .plans
        .iter()
        .map(|plan| (plan.plan_id, plan))
        .collect::<BTreeMap<_, _>>();
    let mut frames = Vec::<EncodedChunkFrame<'_>>::new();
    let mut seen = BTreeSet::new();
    let mut cursor = 0_usize;
    let mut previous = None;
    while cursor < payload.len() {
        let header_len = usize::try_from(chunk_frame_header_len(extended)).unwrap_or(if extended {
            96
        } else {
            64
        });
        if payload.len() - cursor < header_len {
            return Err(structure("Chunk frame header is truncated"));
        }
        let header = &payload[cursor..cursor + header_len];
        let ChunkFrameHeader {
            stored_len,
            chunk_id,
            logical_len,
            plan_ref,
            group_ref,
            region_owned,
        } = parse_chunk_frame_header(header, extended, whole_object)?;
        if !seen.insert(chunk_id) {
            return Err(noncanonical("Chunk frame IDs must be unique"));
        }
        if !extended && previous.is_some_and(|digest| digest >= chunk_id) {
            return Err(noncanonical(
                "Chunk frames must be uniquely ordered by chunk ID",
            ));
        }
        previous = Some(chunk_id);
        if !region_owned {
            plans.get(&plan_ref).ok_or_else(|| {
                Diagnostic::new(
                    OutcomeClass::Unsupported,
                    ReasonCode::UnknownTransformPlan,
                    format!("Chunk {chunk_id} uses plan {plan_ref}"),
                )
            })?;
        }
        enforce_chunk_bounds(
            &ChunkFrameHeader {
                stored_len,
                chunk_id,
                logical_len,
                plan_ref,
                group_ref,
                region_owned,
            },
            declared_budget,
        )?;
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
            region_owned,
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
    let ordinary_frames = frames
        .iter()
        .copied()
        .filter(|frame| !frame.region_owned)
        .collect::<Vec<_>>();
    validate_frame_dependencies(
        &ordinary_frames,
        &plans,
        dependencies.dictionaries,
        dependencies.groups,
    )?;

    let mut chunks = BTreeMap::new();
    let mut index = BTreeMap::new();
    let mut group_history = BTreeMap::<Digest, Vec<Digest>>::new();
    for (position, frame) in frames.iter().enumerate() {
        if frame.region_owned {
            index.insert(
                frame.chunk_id,
                ChunkLocation {
                    offset: frame.offset,
                    stored_len: 0,
                },
            );
            chunks.insert(
                frame.chunk_id,
                Chunk {
                    chunk_id: frame.chunk_id,
                    logical_len: frame.logical_len,
                    plan_ref: crate::jpeg_reconstruction::REGION_MEMBER_PLAN_REF,
                    group_ref: None,
                    plaintext: Box::default(),
                },
            );
            continue;
        }
        let plan = plans[&frame.plan_ref];
        let prefix = match plan_mode(plan)? {
            PlanMode::Prefix { lookback } => {
                let group_id = frame.group_ref.ok_or_else(|| {
                    Diagnostic::new(
                        OutcomeClass::Nonconforming,
                        ReasonCode::InvalidGroupReference,
                        frame.chunk_id.to_string(),
                    )
                })?;
                Some(physical_prefix(
                    &chunks,
                    group_history.entry(group_id).or_default(),
                    lookback,
                )?)
            }
            PlanMode::Independent | PlanMode::Dictionary(_) => None,
        };
        let decoded = decode_frame_payload(
            plan,
            frame.stored,
            frame.logical_len,
            dependencies.dictionaries,
            dependencies.reconstruction_data,
            prefix.as_deref(),
        );
        let plaintext = match decoded {
            Ok(plaintext) => plaintext,
            Err(error) if is_group_prerequisite(&frames, position, dependencies.groups) => {
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
            let code = if plan
                .transforms
                .iter()
                .any(|step| step.reconstruction_ref.is_some())
            {
                ReasonCode::ReconstructedDigestMismatch
            } else if is_group_prerequisite(&frames, position, dependencies.groups) {
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
    if whole_object {
        reconstruct_regions(&mut chunks, objects, regions, dependencies.plans, &frames)?;
    }
    let physical_order = frames
        .iter()
        .map(|frame| frame.chunk_id)
        .collect::<Vec<_>>()
        .into_boxed_slice();
    Ok((chunks, physical_order, index))
}

pub(super) fn region_owned_chunk_ids(
    objects: &BTreeMap<Digest, crate::eam::ContentObject>,
    regions: &BTreeMap<Digest, crate::eam::ReconstructionRegion>,
) -> Result<BTreeSet<Digest>> {
    let mut owned = BTreeSet::new();
    for region in regions.values() {
        let object = objects.get(&region.content_object).ok_or_else(|| {
            Diagnostic::new(
                OutcomeClass::Nonconforming,
                ReasonCode::UnknownContentObject,
                region.content_object.to_string(),
            )
        })?;
        let start = usize::try_from(region.start_chunk_index)
            .map_err(|_| structure("ReconstructionRegion start exceeds usize"))?;
        let count = usize::try_from(region.chunk_count)
            .map_err(|_| structure("ReconstructionRegion count exceeds usize"))?;
        let end = start
            .checked_add(count)
            .filter(|end| *end <= object.chunks.len())
            .ok_or_else(|| {
                Diagnostic::new(
                    OutcomeClass::Nonconforming,
                    ReasonCode::InvalidReconstructionRegion,
                    region.region_id.to_string(),
                )
            })?;
        for chunk_ref in &object.chunks[start..end] {
            owned.insert(chunk_ref.chunk_id);
        }
    }
    Ok(owned)
}

pub(super) fn verify_region_representations(archive: &Archive) -> Result<()> {
    let plans = archive
        .transform_plans
        .iter()
        .map(|plan| (plan.plan_id, plan))
        .collect::<BTreeMap<_, _>>();
    for region in archive.content_store.reconstruction_regions.values() {
        let plan = plans.get(&region.plan_ref).ok_or_else(|| {
            Diagnostic::new(
                OutcomeClass::Unsupported,
                ReasonCode::UnknownTransformPlan,
                region.plan_ref.to_string(),
            )
        })?;
        let transformed = crate::codec::decode_transformed_payload(
            plan,
            &region.representation,
            region.transformed_bytes,
        )
        .map_err(|error| {
            Diagnostic::new(
                OutcomeClass::Corrupt,
                ReasonCode::MalformedReconstructionPayload,
                format!("region {}: {}", region.region_id, error.detail()),
            )
        })?;
        let reconstructed = crate::transform::inverse_pipeline(&plan.transforms, &transformed)?;
        let original = region_original_bytes(archive, region)?;
        if sha256_exact(&original) != sha256_exact(&reconstructed) || original != reconstructed {
            return Err(integrity(
                ReasonCode::RegionMemberDigestMismatch,
                format!(
                    "writer rejected region {} exact round trip",
                    region.region_id
                ),
            ));
        }
    }
    Ok(())
}

pub(super) fn region_original_bytes(
    archive: &Archive,
    region: &crate::eam::ReconstructionRegion,
) -> Result<Vec<u8>> {
    let object = &archive.content_store.objects[&region.content_object];
    let start = usize::try_from(region.start_chunk_index)
        .map_err(|_| structure("ReconstructionRegion start exceeds usize"))?;
    let end = start
        .checked_add(
            usize::try_from(region.chunk_count)
                .map_err(|_| structure("ReconstructionRegion count exceeds usize"))?,
        )
        .ok_or_else(|| structure("ReconstructionRegion range overflows"))?;
    let mut bytes = Vec::with_capacity(
        usize::try_from(region.logical_bytes)
            .map_err(|_| resource("ReconstructionRegion exceeds usize"))?,
    );
    for chunk_ref in &object.chunks[start..end] {
        bytes.extend_from_slice(&archive.content_store.chunks[&chunk_ref.chunk_id].plaintext);
    }
    Ok(bytes)
}

pub(super) fn reconstruct_regions(
    chunks: &mut BTreeMap<Digest, Chunk>,
    objects: &BTreeMap<Digest, crate::eam::ContentObject>,
    regions: &BTreeMap<Digest, crate::eam::ReconstructionRegion>,
    plans: &[TransformPlan],
    frames: &[EncodedChunkFrame<'_>],
) -> Result<()> {
    let plans = plans
        .iter()
        .map(|plan| (plan.plan_id, plan))
        .collect::<BTreeMap<_, _>>();
    let declared_region_chunks = frames
        .iter()
        .filter(|frame| frame.region_owned)
        .map(|frame| frame.chunk_id)
        .collect::<BTreeSet<_>>();
    let expected_region_chunks = region_owned_chunk_ids(objects, regions)?;
    if declared_region_chunks != expected_region_chunks {
        return Err(Diagnostic::new(
            OutcomeClass::Nonconforming,
            ReasonCode::UnknownReconstructionRegion,
            "region-owned Chunk declarations do not match ReconstructionRegion ranges",
        ));
    }
    for region in regions.values() {
        let object = objects.get(&region.content_object).ok_or_else(|| {
            Diagnostic::new(
                OutcomeClass::Nonconforming,
                ReasonCode::UnknownContentObject,
                region.content_object.to_string(),
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
        let member_lengths = object
            .chunks
            .get(start..end)
            .ok_or_else(|| {
                Diagnostic::new(
                    OutcomeClass::Nonconforming,
                    ReasonCode::InvalidReconstructionRegion,
                    region.region_id.to_string(),
                )
            })?
            .iter()
            .map(|chunk_ref| {
                chunks
                    .get(&chunk_ref.chunk_id)
                    .map(|chunk| chunk.logical_len)
                    .ok_or_else(|| {
                        Diagnostic::new(
                            OutcomeClass::Nonconforming,
                            ReasonCode::UnknownChunk,
                            chunk_ref.chunk_id.to_string(),
                        )
                    })
            })
            .collect::<Result<Vec<_>>>()?;
        for (chunk_id, plaintext) in
            reconstruct_region_members(region, object, &plans, &member_lengths)?
        {
            let chunk = chunks.get_mut(&chunk_id).ok_or_else(|| {
                Diagnostic::new(
                    OutcomeClass::Nonconforming,
                    ReasonCode::UnknownChunk,
                    chunk_id.to_string(),
                )
            })?;
            if !chunk.plaintext.is_empty() && chunk.plaintext.as_ref() != plaintext.as_slice() {
                return Err(integrity(
                    ReasonCode::RegionMemberDigestMismatch,
                    format!("region {} repeated Chunk {chunk_id}", region.region_id),
                ));
            }
            chunk.plaintext = plaintext.into_boxed_slice();
        }
    }
    Ok(())
}

/// Rebuilds one region's member plaintexts and verifies every member digest.
///
/// This is the single whole-object reconstruction authority. The INDEXED reader
/// calls it after decoding CHUNK_DATA; the sequential reader calls it when the
/// region's ContentObject record arrives, which is the first moment a
/// sequential reader knows which Chunks the region owns. `member_lengths` are
/// the authoritative logical lengths declared by the region-owned Chunk frames.
pub(super) fn reconstruct_region_members(
    region: &crate::eam::ReconstructionRegion,
    object: &crate::eam::ContentObject,
    plans: &BTreeMap<u64, &TransformPlan>,
    member_lengths: &[u64],
) -> Result<Vec<(Digest, Vec<u8>)>> {
    if region.representation.is_empty()
        || region.representation.len() > crate::jpeg_reconstruction::MAX_JXL_BYTES
        || region.transformed_bytes
            > u64::try_from(crate::jpeg_reconstruction::MAX_JXL_BYTES).unwrap_or(u64::MAX)
        || region.logical_bytes
            > u64::try_from(crate::jpeg_reconstruction::MAX_JPEG_BYTES).unwrap_or(u64::MAX)
        || region.chunk_count > crate::jpeg_reconstruction::MAX_REGION_CHUNKS
    {
        return Err(Diagnostic::new(
            OutcomeClass::PolicyRefused,
            ReasonCode::ResourceLimit,
            format!(
                "region {} exceeds JPEG reconstruction bounds",
                region.region_id
            ),
        ));
    }
    let plan = plans.get(&region.plan_ref).ok_or_else(|| {
        Diagnostic::new(
            OutcomeClass::Unsupported,
            ReasonCode::UnknownTransformPlan,
            region.plan_ref.to_string(),
        )
    })?;
    let transformed = crate::codec::decode_transformed_payload(
        plan,
        &region.representation,
        region.transformed_bytes,
    )
    .map_err(|error| {
        Diagnostic::new(
            OutcomeClass::Corrupt,
            ReasonCode::MalformedReconstructionPayload,
            format!("region {}: {}", region.region_id, error.detail()),
        )
    })?;
    let reconstructed = crate::transform::inverse_pipeline(&plan.transforms, &transformed)?;
    if u64::try_from(reconstructed.len()).unwrap_or(u64::MAX) != region.logical_bytes {
        return Err(integrity(
            ReasonCode::ReconstructedLengthMismatch,
            format!("region {} reconstructed length", region.region_id),
        ));
    }
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
            region.region_id.to_string(),
        )
    })?;
    if members.len() != member_lengths.len() {
        return Err(integrity(
            ReasonCode::ReconstructedLengthMismatch,
            format!("region {} member count", region.region_id),
        ));
    }
    let mut cursor = 0_usize;
    let mut rebuilt = Vec::with_capacity(members.len());
    for (chunk_ref, logical_len) in members.iter().zip(member_lengths) {
        let length =
            usize::try_from(*logical_len).map_err(|_| resource("region member exceeds usize"))?;
        let next = cursor
            .checked_add(length)
            .filter(|next| *next <= reconstructed.len())
            .ok_or_else(|| {
                integrity(
                    ReasonCode::ReconstructedLengthMismatch,
                    format!("region {} member boundaries", region.region_id),
                )
            })?;
        let plaintext = &reconstructed[cursor..next];
        if sha256_exact(plaintext) != chunk_ref.chunk_id {
            return Err(integrity(
                ReasonCode::RegionMemberDigestMismatch,
                format!("region {} Chunk {}", region.region_id, chunk_ref.chunk_id),
            ));
        }
        rebuilt.push((chunk_ref.chunk_id, plaintext.to_vec()));
        cursor = next;
    }
    if cursor != reconstructed.len() {
        return Err(integrity(
            ReasonCode::ReconstructedLengthMismatch,
            format!("region {} trailing reconstructed bytes", region.region_id),
        ));
    }
    Ok(rebuilt)
}

pub(super) fn validate_frame_dependencies(
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

pub(super) fn physical_prefix(
    chunks: &BTreeMap<Digest, Chunk>,
    history: &[Digest],
    lookback: u32,
) -> Result<Vec<u8>> {
    let plaintexts = history
        .iter()
        .map(|chunk_id| chunks[chunk_id].plaintext.as_ref())
        .collect::<Vec<_>>();
    physical_prefix_from_slices(&plaintexts, lookback)
}

/// Builds the bounded-lookback prefix from already decoded preceding members.
///
/// Both layouts derive their prefix bytes here, so a prefix-coded Chunk decodes
/// identically whether its predecessors came from a CHUNK_DATA section or from
/// the sequential reader's bounded group history.
pub(super) fn physical_prefix_from_slices(history: &[&[u8]], lookback: u32) -> Result<Vec<u8>> {
    let lookback = usize::try_from(lookback).map_err(|_| resource("lookback exceeds usize"))?;
    let first = history.len().saturating_sub(lookback);
    let preceding = &history[first..];
    let total = preceding.iter().try_fold(0_usize, |total, plaintext| {
        total
            .checked_add(plaintext.len())
            .ok_or_else(|| resource("prefix length exceeds usize"))
    })?;
    let retained =
        total.min(usize::try_from(crate::codec::ZSTD_WINDOW_BYTES).unwrap_or(usize::MAX));
    let mut skip = total - retained;
    let mut prefix = Vec::with_capacity(retained);
    for plaintext in preceding {
        if skip >= plaintext.len() {
            skip -= plaintext.len();
            continue;
        }
        prefix.extend_from_slice(&plaintext[skip..]);
        skip = 0;
    }
    Ok(prefix)
}

/// Decodes one Chunk frame's stored bytes back to plaintext.
///
/// This is the single decode authority shared by the INDEXED section reader and
/// the sequential STREAM reader. `prefix` carries the bounded-lookback history
/// for prefix-coded plans and must be `None` for every other plan mode.
pub(super) fn decode_frame_payload(
    plan: &TransformPlan,
    stored: &[u8],
    logical_len: u64,
    dictionaries: &BTreeMap<Digest, Dictionary>,
    reconstruction_data: &BTreeMap<Digest, ReconstructionData>,
    prefix: Option<&[u8]>,
) -> Result<Vec<u8>> {
    match plan_mode(plan)? {
        PlanMode::Independent => {
            if plan
                .transforms
                .iter()
                .any(|step| step.reconstruction_ref.is_some())
            {
                decode_payload_with_reconstruction(plan, stored, logical_len, reconstruction_data)
            } else {
                decode_payload(plan, stored, logical_len)
            }
        }
        PlanMode::Dictionary(dictionary_id) => {
            let dictionary = dictionaries.get(&dictionary_id).ok_or_else(|| {
                Diagnostic::new(
                    OutcomeClass::Nonconforming,
                    ReasonCode::UnknownDictionary,
                    dictionary_id.to_string(),
                )
            })?;
            decode_payload_with_dictionary(plan, stored, logical_len, dictionary)
        }
        PlanMode::Prefix { .. } => {
            let prefix = prefix.ok_or_else(|| {
                Diagnostic::new(
                    OutcomeClass::Nonconforming,
                    ReasonCode::InvalidGroupReference,
                    "a prefix-coded Chunk requires bounded-lookback history",
                )
            })?;
            decode_payload_with_prefix(plan, stored, logical_len, prefix)
        }
    }
}

pub(super) fn is_group_prerequisite(
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
pub(super) struct SectionLocation {
    pub(super) offset: u64,
    pub(super) len: u64,
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
    reconstructive: bool,
    whole_object: bool,
) -> Result<SectionLocation> {
    let offset = u64_len(body)?;
    body.extend_from_slice(&SECTION_MAGIC);
    body.extend_from_slice(
        &section_id(kind, extended, reconstructive, whole_object)?.to_be_bytes(),
    );
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

/// Serializes the fixed 256-byte preamble for either layout.
///
/// `footer_offset` is the advisory INDEXED footer hint. A STREAM writer cannot
/// know its own final extent before emitting the body, so it passes zero and
/// the hint field stays zero; the STREAM footer is self-locating at EOF.
pub(super) fn encode_preamble(
    descriptor: &ArchiveDescriptor,
    footer_offset: u64,
) -> Result<Vec<u8>> {
    if descriptor.layout == Layout::Indexed && descriptor.stream_dedup_window != 0 {
        return Err(noncanonical(
            "INDEXED preamble must declare a zero STREAM dedup window",
        ));
    }
    if descriptor.layout == Layout::Stream && footer_offset != 0 {
        return Err(noncanonical(
            "STREAM preamble must not declare an advisory footer offset",
        ));
    }
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
    bytes[72] = descriptor.layout.wire_id();
    bytes[73] = 1;
    bytes[74] = u8::from(descriptor.budget_declared);
    bytes[76..84].copy_from_slice(&descriptor.decode.window_bytes.to_be_bytes());
    bytes[84..92].copy_from_slice(&descriptor.decode.working_set_bytes.to_be_bytes());
    bytes[92..96].copy_from_slice(&descriptor.decode.flags.to_be_bytes());
    encode_budget(&mut bytes[96..160], descriptor.budget);
    bytes[160..168].copy_from_slice(&descriptor.stream_dedup_window.to_be_bytes());
    bytes[176..184].copy_from_slice(&footer_offset.to_be_bytes());
    Ok(bytes)
}

pub(super) fn encode_budget(bytes: &mut [u8], budget: ResourceBudget) {
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
pub(super) struct Preamble {
    pub(super) version: FormatVersion,
    pub(super) features: FeatureSet,
    pub(super) layout: Layout,
    pub(super) budget_declared: bool,
    pub(super) budget: ResourceBudget,
    pub(super) decode: DecodeRequirements,
    pub(super) stream_dedup_window: u64,
    pub(super) footer_hint: u64,
}

pub(super) fn decode_preamble(bytes: &[u8]) -> Result<Preamble> {
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
    if features.incompat & crate::crypto::FEATURE_PRIVATE_RESOURCE_DECLARATION_V1 != 0
        && features.incompat & crate::crypto::FEATURE_ENCRYPTED_INDEXED_V1 == 0
    {
        return Err(Diagnostic::new(
            OutcomeClass::Nonconforming,
            ReasonCode::UnsupportedRequiredFeature,
            "private-resource-declaration-v1 requires encrypted-indexed-v1",
        ));
    }
    let layout = match preamble[72] {
        1 => Layout::Indexed,
        2 => Layout::Stream,
        other => {
            return Err(Diagnostic::new(
                OutcomeClass::Unsupported,
                ReasonCode::UnsupportedRequiredFeature,
                format!("unsupported ECF layout discriminant {other}"),
            ));
        }
    };
    if preamble[73] != 1 {
        return Err(Diagnostic::new(
            OutcomeClass::Unsupported,
            ReasonCode::UnsupportedRequiredFeature,
            "only Complete bootstrap archives are supported",
        ));
    }
    if layout == Layout::Stream && features.incompat & FEATURE_STREAM_LAYOUT_V1 == 0 {
        return Err(Diagnostic::new(
            OutcomeClass::Nonconforming,
            ReasonCode::UnsupportedRequiredFeature,
            "STREAM layout must declare required feature stream-layout-v1",
        ));
    }
    if layout == Layout::Indexed && features.incompat & FEATURE_STREAM_LAYOUT_V1 != 0 {
        return Err(Diagnostic::new(
            OutcomeClass::Nonconforming,
            ReasonCode::UnsupportedRequiredFeature,
            "stream-layout-v1 is declared but the layout is not STREAM",
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
    let stream_dedup_window = u64::from_be_bytes(exact(&preamble[160..168])?);
    if preamble[168..176].iter().any(|byte| *byte != 0) {
        return Err(noncanonical("the hostility summary field must be zero"));
    }
    let footer_hint = u64::from_be_bytes(exact(&preamble[176..184])?);
    match layout {
        Layout::Indexed => {
            if stream_dedup_window != 0 {
                return Err(noncanonical(
                    "the STREAM dedup window must be zero in INDEXED layout",
                ));
            }
        }
        Layout::Stream => {
            if footer_hint != 0 {
                return Err(noncanonical(
                    "STREAM layout has no advisory footer-offset hint",
                ));
            }
        }
    }
    Ok(Preamble {
        version,
        features,
        layout,
        budget_declared: preamble[74] == 1,
        budget,
        decode: DecodeRequirements {
            window_bytes: u64::from_be_bytes(exact(&preamble[76..84])?),
            working_set_bytes: u64::from_be_bytes(exact(&preamble[84..92])?),
            flags: u32::from_be_bytes(exact(&preamble[92..96])?),
        },
        stream_dedup_window,
        footer_hint,
    })
}

pub(super) fn decode_budget(bytes: &[u8]) -> Result<ResourceBudget> {
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
    reconstructive: bool,
    whole_object: bool,
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
        let kind = section_kind(id, extended, reconstructive, whole_object)?;
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
    let required_end = if whole_object {
        10
    } else if reconstructive {
        9
    } else if extended {
        8
    } else {
        6
    };
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
    let actual = derived_budget(archive, rebuilt_index)?;
    let decode = aggregate_archive_decode_requirements(
        &archive.transform_plans,
        &archive.content_store.dictionaries,
        &archive.content_store.chunk_groups,
    )?;
    if actual.entry_count > budget.entry_count
        || actual.total_logical_bytes > budget.total_logical_bytes
        || actual.max_single_entry_logical_bytes > budget.max_single_entry_logical_bytes
        || actual.max_expansion_ratio_milli > budget.max_expansion_ratio_milli
        || actual.chunk_count > budget.chunk_count
        || actual.max_path_depth > budget.max_path_depth
        || actual.max_metadata_bytes > budget.max_metadata_bytes
        || actual.max_key_derivation_cost > budget.max_key_derivation_cost
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

fn section_id(
    kind: SectionKind,
    extended: bool,
    reconstructive: bool,
    whole_object: bool,
) -> Result<u16> {
    match (extended, reconstructive, whole_object, kind) {
        (_, _, _, SectionKind::Descriptor) => Ok(1),
        (_, _, _, SectionKind::TransformPlans) => Ok(2),
        (true, true, true, SectionKind::Dictionaries) => Ok(3),
        (true, true, true, SectionKind::ChunkGroups) => Ok(4),
        (true, true, true, SectionKind::ReconstructionData) => Ok(5),
        (true, true, true, SectionKind::ReconstructionRegions) => Ok(6),
        (true, true, true, SectionKind::ChunkData) => Ok(7),
        (true, true, true, SectionKind::ManifestRecords) => Ok(8),
        (true, true, true, SectionKind::Fidelity) => Ok(9),
        (true, true, true, SectionKind::Index) => Ok(10),
        (true, true, false, SectionKind::Dictionaries) => Ok(3),
        (true, true, false, SectionKind::ChunkGroups) => Ok(4),
        (true, true, false, SectionKind::ReconstructionData) => Ok(5),
        (true, true, false, SectionKind::ChunkData) => Ok(6),
        (true, true, false, SectionKind::ManifestRecords) => Ok(7),
        (true, true, false, SectionKind::Fidelity) => Ok(8),
        (true, true, false, SectionKind::Index) => Ok(9),
        (false, false, false, SectionKind::ChunkData) => Ok(3),
        (false, false, false, SectionKind::ManifestRecords) => Ok(4),
        (false, false, false, SectionKind::Fidelity) => Ok(5),
        (false, false, false, SectionKind::Index) => Ok(6),
        (true, false, false, SectionKind::Dictionaries) => Ok(3),
        (true, false, false, SectionKind::ChunkGroups) => Ok(4),
        (true, false, false, SectionKind::ChunkData) => Ok(5),
        (true, false, false, SectionKind::ManifestRecords) => Ok(6),
        (true, false, false, SectionKind::Fidelity) => Ok(7),
        (true, false, false, SectionKind::Index) => Ok(8),
        _ => Err(structure(
            "section kind does not belong to selected feature schema",
        )),
    }
}

fn section_kind(
    value: u16,
    extended: bool,
    reconstructive: bool,
    whole_object: bool,
) -> Result<SectionKind> {
    match (extended, reconstructive, whole_object, value) {
        (_, _, _, 1) => Ok(SectionKind::Descriptor),
        (_, _, _, 2) => Ok(SectionKind::TransformPlans),
        (true, true, true, 3) => Ok(SectionKind::Dictionaries),
        (true, true, true, 4) => Ok(SectionKind::ChunkGroups),
        (true, true, true, 5) => Ok(SectionKind::ReconstructionData),
        (true, true, true, 6) => Ok(SectionKind::ReconstructionRegions),
        (true, true, true, 7) => Ok(SectionKind::ChunkData),
        (true, true, true, 8) => Ok(SectionKind::ManifestRecords),
        (true, true, true, 9) => Ok(SectionKind::Fidelity),
        (true, true, true, 10) => Ok(SectionKind::Index),
        (true, true, false, 3) => Ok(SectionKind::Dictionaries),
        (true, true, false, 4) => Ok(SectionKind::ChunkGroups),
        (true, true, false, 5) => Ok(SectionKind::ReconstructionData),
        (true, true, false, 6) => Ok(SectionKind::ChunkData),
        (true, true, false, 7) => Ok(SectionKind::ManifestRecords),
        (true, true, false, 8) => Ok(SectionKind::Fidelity),
        (true, true, false, 9) => Ok(SectionKind::Index),
        (false, false, false, 3) => Ok(SectionKind::ChunkData),
        (false, false, false, 4) => Ok(SectionKind::ManifestRecords),
        (false, false, false, 5) => Ok(SectionKind::Fidelity),
        (false, false, false, 6) => Ok(SectionKind::Index),
        (true, false, false, 3) => Ok(SectionKind::Dictionaries),
        (true, false, false, 4) => Ok(SectionKind::ChunkGroups),
        (true, false, false, 5) => Ok(SectionKind::ChunkData),
        (true, false, false, 6) => Ok(SectionKind::ManifestRecords),
        (true, false, false, 7) => Ok(SectionKind::Fidelity),
        (true, false, false, 8) => Ok(SectionKind::Index),
        _ => Err(Diagnostic::new(
            OutcomeClass::Unsupported,
            ReasonCode::UnsupportedRequiredFeature,
            format!("unknown section type {value}"),
        )),
    }
}

pub(super) fn digest(bytes: &[u8]) -> Result<Digest> {
    Ok(Digest::from_bytes(bytes.try_into().map_err(|_| {
        noncanonical("digest must contain exactly 32 bytes")
    })?))
}

pub(super) fn exact<const N: usize>(bytes: &[u8]) -> Result<[u8; N]> {
    bytes
        .try_into()
        .map_err(|_| structure(format!("expected exactly {N} bytes")))
}

pub(super) fn u64_len<T: AsRef<[u8]> + ?Sized>(value: &T) -> Result<u64> {
    u64::try_from(value.as_ref().len()).map_err(|_| resource("byte length exceeds u64"))
}

pub(super) fn noncanonical(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::Nonconforming,
        ReasonCode::NoncanonicalEncoding,
        detail,
    )
}

pub(super) fn structure(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(OutcomeClass::Corrupt, ReasonCode::SectionStructure, detail)
}

pub(super) fn integrity(code: ReasonCode, detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(OutcomeClass::Corrupt, code, detail)
}

pub(super) fn resource(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::PolicyRefused,
        ReasonCode::ResourceLimit,
        detail,
    )
}
