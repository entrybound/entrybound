//! Encrypted INDEXED container integration.

use std::collections::BTreeSet;

use sha2::{Digest as _, Sha256};
use subtle::ConstantTimeEq as _;
use x_wing::{
    Ciphertext, EncapsulationKey,
    kem::{Decapsulate as _, Encapsulate as _},
};
use zeroize::Zeroize;

use super::{
    AddressingBinding, BoundaryMode, CryptoPolicy, KeyHierarchy, PaddingMode, ProtectionPolicy,
    SignatureRecord, Unlock, XWingRecipient, a2id, aead_open, aead_seal, commitment,
    crypto_integrity, envelope_mac, no_recipient, open_afk, password_secret, public_crypto_context,
    random, resource_refused, seal_afk, stanza_invalid, wire,
};
use crate::archive::{ArchiveInspection, inspect};
use crate::canonical::{RecordBuilder, decode_record};
use crate::diagnostics::{Diagnostic, OutcomeClass, ReasonCode, Result};
use crate::eam::{Archive, DecodeRequirements, FeatureSet, ResourceBudget};
use crate::ecf::{
    EncryptedDecodedParts, EncryptedPlainParts, MAGIC, PREAMBLE_LEN, SECTION_HEADER_LEN,
    prepare_encrypted_plain_parts, private_descriptor_declaration,
};
use crate::ecf::{OpenedArchive, VerificationReport};
use crate::identity::{IdentitySet, physical_container_identity, sha256_exact};
use crate::planner::CompressionProfile;

const SECTION_MAGIC: &[u8; 4] = b"EBS1";
const SECTION_ENVELOPE: u16 = 32;
const SECTION_SEGMENTS: u16 = 33;
const FOOTER_MAGIC: [u8; 8] = [0x8e, b'E', b'B', b'F', b'\r', b'\n', 0x1a, b'\n'];
const ENCRYPTED_FOOTER_LEN: usize = 192;
const SEGMENT_HEADER_LEN: usize = 64;
const PROTECTED_HEADER_LEN: usize = 32;
const SEGMENT_CONTROL: u8 = 1;
const SEGMENT_PAYLOAD: u8 = 2;
const RECORD_DATA: u8 = 1;
const RECORD_END: u8 = 2;
const MAX_DATA_MESSAGES: usize = (1 << 20) - 1;
const MAX_SEGMENT_PRIVATE: u64 = 1 << 30;
const MAX_CONTROL_PRIVATE: usize = 1 << 20;
const MAX_PAYLOAD_PRIVATE: usize = 64 << 20;

#[derive(Clone, Eq, PartialEq)]
pub struct EncryptedWriteOptions<'a> {
    pub recipients: &'a [XWingRecipient],
    pub password: Option<&'a [u8]>,
    pub padding: PaddingMode,
    pub boundary: BoundaryMode,
    pub include_index: bool,
    pub embedded_signatures: &'a [SignatureRecord],
}

impl Default for EncryptedWriteOptions<'_> {
    fn default() -> Self {
        Self {
            recipients: &[],
            password: None,
            padding: PaddingMode::Bucketed,
            boundary: BoundaryMode::SecretGearTable,
            include_index: true,
            embedded_signatures: &[],
        }
    }
}

#[derive(Clone, Copy)]
pub struct EncryptedOpenOptions<'a> {
    pub unlock: Option<Unlock<'a>>,
    pub crypto_policy: CryptoPolicy,
    pub resource_policy: ResourceBudget,
    pub decode_policy: DecodeRequirements,
}

impl<'a> EncryptedOpenOptions<'a> {
    #[must_use]
    pub fn new(unlock: Option<Unlock<'a>>) -> Self {
        Self {
            unlock,
            crypto_policy: CryptoPolicy::default(),
            resource_policy: crate::archive::bootstrap_resource_policy(),
            decode_policy: crate::archive::bootstrap_decode_policy(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicCryptoInspection {
    pub encrypted: bool,
    pub payload_suite: &'static str,
    pub recipient_count: u64,
    pub recipient_types: Vec<&'static str>,
    pub padding: PaddingMode,
    pub boundary: BoundaryMode,
    pub segment_count: Option<u64>,
    pub total_container_bytes: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CryptoInspection {
    pub public: PublicCryptoInspection,
    pub authenticated: Option<ArchiveInspection>,
    pub authenticated_descriptor: Option<AuthenticatedDescriptorInspection>,
}

/// Authenticated status of the sole producer resource declaration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AuthenticatedDescriptorInspection {
    pub record_version: u16,
    pub producer_declaration_present: bool,
    pub independently_validated: bool,
    pub declared_budget: Option<ResourceBudget>,
    pub declared_decode: Option<DecodeRequirements>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EncryptedArchive {
    pub bytes: Vec<u8>,
    pub archive: Archive,
    pub identities: IdentitySet,
    pub public: PublicCryptoInspection,
}

/// Authenticated private recipient-directory metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecipientDirectoryEntry {
    pub stanza_id: [u8; 16],
    pub stanza_type: u16,
    pub fingerprint: [u8; 32],
    pub label: String,
}

/// Encrypted archive state whose private signature/addressing data has been
/// authenticated. No file key is exposed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthenticatedEncryptedArchive {
    pub opened: OpenedArchive,
    pub embedded_signatures: Vec<SignatureRecord>,
    pub recipient_directory: Vec<RecipientDirectoryEntry>,
    pub addressing: AddressingBinding,
}

#[derive(Clone)]
struct PlainObject {
    class: u8,
    bytes: Vec<u8>,
}

struct BuiltSegment {
    bytes: Vec<u8>,
    digest: [u8; 32],
}

struct ParsedFooter {
    total_len: u64,
    envelope_offset: u64,
    envelope_len: u64,
    segments_offset: u64,
    segments_len: u64,
    terminal_offset: u64,
    archive_final_offset: u64,
    preamble_digest: [u8; 32],
    segments_digest: [u8; 32],
    public_context_digest: [u8; 32],
}

struct ArchiveFinal {
    segment_count: u64,
    entry_count: u64,
    total_logical: u64,
    chunk_count: u64,
    lai: [u8; 32],
    pcr: [u8; 32],
    aux: [u8; 32],
    recipient_set: [u8; 32],
    segment_sequence: [u8; 32],
    footer_core: [u8; 32],
    descriptor_id: [u8; 32],
    manifest_id: [u8; 32],
}

/// Encrypts one already-planned EAM as a crypto-v1 INDEXED archive.
pub fn encrypt_archive(
    archive: &Archive,
    options: EncryptedWriteOptions<'_>,
) -> Result<EncryptedArchive> {
    validate_write_options(&options)?;
    let mut encrypted_plan = archive.clone();
    encrypted_plan.descriptor.planner_id =
        encrypted_planner_id(&encrypted_plan.descriptor.planner_id).to_owned();
    let parts = prepare_encrypted_plain_parts(&encrypted_plan)?;
    let afk = super::Secret32(random::<32>()?);
    let archive_id = random::<32>()?;
    encrypt_with_file_key(parts, &afk.0, archive_id, options)
}

pub(super) fn encrypt_with_file_key(
    parts: EncryptedPlainParts,
    afk: &[u8; 32],
    archive_id: [u8; 32],
    options: EncryptedWriteOptions<'_>,
) -> Result<EncryptedArchive> {
    validate_write_options(&options)?;
    let (policy, stanzas, directory) = build_recipients(afk, &archive_id, &options)?;
    encrypt_with_material(
        parts, afk, archive_id, options, policy, stanzas, directory, None,
    )
}

#[derive(Clone)]
struct ReusableSegment {
    class: u8,
    bytes: Vec<u8>,
    digest: [u8; 32],
}

#[allow(clippy::too_many_arguments)]
fn encrypt_with_material(
    mut parts: EncryptedPlainParts,
    afk: &[u8; 32],
    archive_id: [u8; 32],
    options: EncryptedWriteOptions<'_>,
    policy: ProtectionPolicy,
    mut stanzas: Vec<wire::RecipientStanza>,
    directory: Vec<Vec<u8>>,
    reusable_payload: Option<&std::collections::BTreeMap<u64, ReusableSegment>>,
) -> Result<EncryptedArchive> {
    let descriptor_declaration = private_descriptor_declaration(&parts.descriptor)?;
    let keys = KeyHierarchy::derive(afk, &archive_id)?;
    stanzas.sort_by(|left, right| {
        left.sort_key()
            .expect("canonical stanza")
            .cmp(&right.sort_key().expect("canonical stanza"))
    });
    let ordinary_features = parts.archive.descriptor.features.incompat;
    let recipient_feature = match policy {
        ProtectionPolicy::HybridOnly => super::FEATURE_XWING_RECIPIENT,
        ProtectionPolicy::PasswordOnly => super::FEATURE_PASSWORD_RECIPIENT,
    };
    let mut features = ordinary_features
        | super::FEATURE_ENCRYPTED_INDEXED_V1
        | super::FEATURE_PAYLOAD_SUITE_V1
        | super::FEATURE_PADDING
        | recipient_feature;
    if options.boundary == BoundaryMode::PhteAes128 {
        features |= super::FEATURE_STRONG_BOUNDARY;
    }
    if !options.embedded_signatures.is_empty() {
        features |= super::FEATURE_SIGNATURE_ED25519_V1;
    }
    match descriptor_declaration.version {
        1 => {}
        2 => features |= super::FEATURE_PRIVATE_RESOURCE_DECLARATION_V1,
        _ => {
            return Err(wire::private_invalid(
                "unsupported encrypted Descriptor version",
            ));
        }
    }
    let public_context =
        public_crypto_context(&archive_id, features, options.padding, options.boundary)?;
    let stored_commitment = commitment(&keys, &archive_id, options.padding, options.boundary)?;
    let mac = envelope_mac(&keys, &public_context, &stored_commitment, policy, &stanzas)?;
    let envelope = wire::CryptoEnvelope {
        archive_id,
        commitment: stored_commitment,
        protection_policy: policy as u8,
        padding_mode: options.padding as u8,
        boundary_mode: options.boundary as u8,
        stanzas,
        envelope_mac: mac,
    };
    let envelope_section = section(SECTION_ENVELOPE, &envelope.encode()?)?;

    let descriptor = wire::private_object(wire::PRIVATE_OBJECT_RECORD, &parts.descriptor)?;
    let manifest = collection_object(wire::COLLECTION_MANIFEST, &parts.manifest)?;
    let descriptor_id = wire::encrypted_object_id(&descriptor)?;
    let manifest_id = wire::encrypted_object_id(&manifest)?;
    let mut objects = vec![PlainObject {
        class: SEGMENT_CONTROL,
        bytes: descriptor,
    }];
    objects.push(PlainObject {
        class: SEGMENT_CONTROL,
        bytes: collection_object(wire::COLLECTION_TRANSFORM_PLANS, &parts.transform_plans)?,
    });
    if ordinary_features & crate::ecf::FEATURE_CROSS_FILE_COMPRESSION_V1 != 0 {
        objects.push(PlainObject {
            class: SEGMENT_CONTROL,
            bytes: collection_object(wire::COLLECTION_CHUNK_GROUPS, &parts.chunk_groups)?,
        });
    }
    if policy == ProtectionPolicy::HybridOnly {
        objects.push(PlainObject {
            class: SEGMENT_CONTROL,
            bytes: collection_object(wire::COLLECTION_RECIPIENT_DIRECTORY, &directory)?,
        });
    }
    if ordinary_features & crate::ecf::FEATURE_CROSS_FILE_COMPRESSION_V1 != 0 {
        objects.push(PlainObject {
            class: SEGMENT_PAYLOAD,
            bytes: collection_object(wire::COLLECTION_DICTIONARIES, &parts.dictionaries)?,
        });
    }
    if ordinary_features & crate::ecf::FEATURE_RECONSTRUCTIVE_TRANSFORM_V1 != 0 {
        objects.push(PlainObject {
            class: SEGMENT_PAYLOAD,
            bytes: collection_object(
                wire::COLLECTION_RECONSTRUCTION_DATA,
                &parts.reconstruction_data,
            )?,
        });
    }
    if ordinary_features & crate::ecf::FEATURE_WHOLE_OBJECT_RECONSTRUCTION_V1 != 0 {
        objects.push(PlainObject {
            class: SEGMENT_PAYLOAD,
            bytes: collection_object(
                wire::COLLECTION_RECONSTRUCTION_REGIONS,
                &parts.reconstruction_regions,
            )?,
        });
    }
    let mut encrypted_index = Vec::new();
    for frame in parts.chunk_frames {
        let chunk_id: [u8; 32] = frame
            .get(16..48)
            .ok_or_else(|| wire::private_invalid("Chunk frame is too short for its digest"))?
            .try_into()
            .unwrap();
        let object = wire::private_object(wire::PRIVATE_OBJECT_CHUNK, &frame)?;
        let segment_ordinal = objects.len() as u64;
        let fragment_count =
            u32::try_from(fragment_objects(SEGMENT_PAYLOAD, std::slice::from_ref(&object))?.len())
                .map_err(|_| resource_refused("encrypted Chunk fragment count exceeds u32"))?;
        encrypted_index.push(encode_encrypted_index_entry(
            &chunk_id,
            segment_ordinal,
            fragment_count,
        )?);
        objects.push(PlainObject {
            class: SEGMENT_PAYLOAD,
            bytes: object,
        });
    }
    objects.push(PlainObject {
        class: SEGMENT_CONTROL,
        bytes: manifest,
    });
    objects.push(PlainObject {
        class: SEGMENT_CONTROL,
        bytes: wire::private_object(wire::PRIVATE_OBJECT_RECORD, &parts.fidelity)?,
    });
    if !options.embedded_signatures.is_empty() {
        let mut signatures = options
            .embedded_signatures
            .iter()
            .map(SignatureRecord::encode)
            .collect::<Result<Vec<_>>>()?;
        signatures.sort();
        signatures.dedup();
        if signatures.len() != options.embedded_signatures.len() {
            return Err(wire::private_invalid(
                "embedded signatures contain an exact duplicate",
            ));
        }
        objects.push(PlainObject {
            class: SEGMENT_CONTROL,
            bytes: collection_object(wire::COLLECTION_SIGNATURES, &signatures)?,
        });
    }
    if options.include_index {
        encrypted_index.sort();
        objects.push(PlainObject {
            class: SEGMENT_CONTROL,
            bytes: collection_object(wire::COLLECTION_INDEX, &encrypted_index)?,
        });
    }

    let mut segments = Vec::new();
    let mut segment_digests = Vec::new();
    let mut segment_salts = BTreeSet::new();
    if let Some(reusable) = reusable_payload {
        for value in reusable.values() {
            let salt: [u8; 16] = value
                .bytes
                .get(16..32)
                .ok_or_else(|| segment_invalid("reusable segment header is truncated"))?
                .try_into()
                .unwrap();
            if !segment_salts.insert((value.class, salt)) {
                return Err(segment_invalid("reusable segment salt is duplicated"));
            }
        }
    }
    for (ordinal, object) in objects.into_iter().enumerate() {
        if object.class == SEGMENT_PAYLOAD
            && let Some(reused) = reusable_payload.and_then(|values| values.get(&(ordinal as u64)))
        {
            if reused.class != object.class {
                return Err(segment_invalid("reusable payload segment class mismatch"));
            }
            segments.extend_from_slice(&reused.bytes);
            segment_digests.push(reused.digest);
            continue;
        }
        let built = build_unique_segment(
            object.class,
            ordinal as u64,
            &[object.bytes],
            &archive_id,
            &keys,
            options.padding,
            &mut segment_salts,
        )?;
        segments.extend_from_slice(&built.bytes);
        segment_digests.push(built.digest);
    }
    let terminal_ordinal = segment_digests.len() as u64;
    let recipient_sequence = wire::encode_stanza_sequence(&envelope.stanzas)?;
    let recipient_set: [u8; 32] = Sha256::digest(wire::t1(
        "entrybound/recipient-set/v1",
        &[&recipient_sequence],
    )?)
    .into();
    let segment_sequence = segment_sequence_digest(&segment_digests)?;
    let mut final_value = ArchiveFinal {
        segment_count: terminal_ordinal + 1,
        entry_count: parts.archive.entry_set.len() as u64,
        total_logical: parts.archive.total_logical_size()?,
        chunk_count: parts.archive.content_store.chunks.len() as u64,
        lai: *parts.roots.lai.0.as_bytes(),
        pcr: *parts.roots.pcr.0.as_bytes(),
        aux: *parts.roots.aux.0.as_bytes(),
        recipient_set,
        segment_sequence,
        footer_core: [0; 32],
        descriptor_id,
        manifest_id,
    };
    let dummy_object = wire::private_object(
        wire::PRIVATE_OBJECT_RECORD,
        &encode_archive_final(&final_value)?,
    )?;
    let terminal_extent =
        predict_segment_extent(SEGMENT_CONTROL, &[dummy_object.len()], options.padding)?;
    let segments_payload_len = segments.len() as u64 + terminal_extent;
    let envelope_offset = PREAMBLE_LEN;
    let envelope_len = envelope_section.len() as u64;
    let segments_offset = envelope_offset + envelope_len;
    let segments_len = SECTION_HEADER_LEN + segments_payload_len;
    let terminal_offset = segments_offset + SECTION_HEADER_LEN + segments.len() as u64;
    let archive_final_offset = terminal_offset + SEGMENT_HEADER_LEN as u64;
    let total_len = segments_offset + segments_len + ENCRYPTED_FOOTER_LEN as u64;
    let preamble = encrypted_preamble(features)?;
    let context_digest: [u8; 32] = Sha256::digest(&public_context).into();
    let core = footer_core(
        total_len,
        envelope_offset,
        envelope_len,
        segments_offset,
        segments_len,
        terminal_offset,
        archive_final_offset,
        sha256_exact(&preamble).as_bytes(),
        &context_digest,
    )?;
    final_value.footer_core = Sha256::digest(&core).into();
    let final_object = wire::private_object(
        wire::PRIVATE_OBJECT_RECORD,
        &encode_archive_final(&final_value)?,
    )?;
    let terminal = build_unique_segment(
        SEGMENT_CONTROL,
        terminal_ordinal,
        &[final_object],
        &archive_id,
        &keys,
        options.padding,
        &mut segment_salts,
    )?;
    if terminal.bytes.len() as u64 != terminal_extent {
        return Err(segment_invalid(
            "terminal segment extent changed after footer binding",
        ));
    }
    segments.extend_from_slice(&terminal.bytes);
    let segments_section = section(SECTION_SEGMENTS, &segments)?;
    let footer = encrypted_footer(
        total_len,
        envelope_offset,
        envelope_len,
        segments_offset,
        segments_len,
        terminal_offset,
        archive_final_offset,
        &preamble,
        &segments,
        &public_context,
    );
    let mut bytes = preamble;
    bytes.extend_from_slice(&envelope_section);
    bytes.extend_from_slice(&segments_section);
    bytes.extend_from_slice(&footer);
    if bytes.len() as u64 != total_len {
        return Err(segment_invalid("encrypted total length mismatch"));
    }
    let pci = physical_container_identity(&bytes);
    let identities = parts.roots.with_pci(pci);
    parts.archive.descriptor.pci = Some(pci.0);
    let public = public_inspection(&envelope, bytes.len() as u64, Some(terminal_ordinal + 1))?;
    Ok(EncryptedArchive {
        bytes,
        archive: parts.archive,
        identities,
        public,
    })
}

pub(super) fn validate_write_options(options: &EncryptedWriteOptions<'_>) -> Result<()> {
    for signature in options.embedded_signatures {
        signature.encode()?;
    }
    if options.password.is_some() == !options.recipients.is_empty() {
        return Err(Diagnostic::new(
            OutcomeClass::PolicyRefused,
            ReasonCode::CryptoRecipientPolicyInvalid,
            "encrypted creation requires recipients or one password, never both",
        ));
    }
    if options.recipients.len() > 1_024 {
        return Err(resource_refused("too many recipients"));
    }
    let mut seen = BTreeSet::new();
    for recipient in options.recipients {
        if !seen.insert(recipient.fingerprint()) {
            return Err(Diagnostic::new(
                OutcomeClass::PolicyRefused,
                ReasonCode::CryptoRecipientPolicyInvalid,
                "duplicate input recipient",
            ));
        }
    }
    Ok(())
}

fn build_recipients(
    afk: &[u8; 32],
    archive_id: &[u8; 32],
    options: &EncryptedWriteOptions<'_>,
) -> Result<(ProtectionPolicy, Vec<wire::RecipientStanza>, Vec<Vec<u8>>)> {
    if let Some(password) = options.password {
        let parameters = a2id(&random()?, 262_144, 3, 4);
        let secret = password_secret(password, &parameters, CryptoPolicy::default())?;
        let mut stanza = wire::RecipientStanza {
            stanza_type: 2,
            protection_class: 2,
            stanza_id: random()?,
            recipient_hint: [0; 16],
            method_parameters: parameters,
            encapsulation: Vec::new(),
            wrap_nonce: random()?,
            wrapped_afk: [0; 48],
        };
        stanza.wrapped_afk = seal_afk(archive_id, &secret.0, &stanza, afk)?;
        return Ok((ProtectionPolicy::PasswordOnly, vec![stanza], Vec::new()));
    }
    let mut stanzas = Vec::new();
    let mut directory = Vec::new();
    for recipient in options.recipients {
        let public = EncapsulationKey::try_from(recipient.bytes().as_slice())
            .map_err(|_| stanza_invalid("malformed X-Wing recipient key"))?;
        let (ciphertext, shared) = public.encapsulate();
        let mut stanza = wire::RecipientStanza {
            stanza_type: 1,
            protection_class: 1,
            stanza_id: random()?,
            recipient_hint: [0; 16],
            method_parameters: wire::XWING_METHOD.to_vec(),
            encapsulation: ciphertext.as_slice().to_vec(),
            wrap_nonce: random()?,
            wrapped_afk: [0; 48],
        };
        let shared = super::Secret32(
            shared
                .as_slice()
                .try_into()
                .expect("X-Wing shared secret length"),
        );
        stanza.wrapped_afk = seal_afk(archive_id, &shared.0, &stanza, afk)?;
        directory.push(encode_recipient_directory(
            &stanza.stanza_id,
            &recipient.fingerprint(),
            recipient.label(),
        )?);
        stanzas.push(stanza);
    }
    directory.sort_by(|left, right| left[28..44].cmp(&right[28..44]));
    Ok((ProtectionPolicy::HybridOnly, stanzas, directory))
}

/// Opens and fully authenticates/decrypts an encrypted INDEXED archive.
pub fn open_encrypted(bytes: &[u8], options: EncryptedOpenOptions<'_>) -> Result<OpenedArchive> {
    Ok(open_encrypted_authenticated(bytes, options)?.opened)
}

/// Opens an encrypted archive and returns only authenticated private
/// signature/addressing metadata alongside the ordinary verified EAM.
pub fn open_encrypted_authenticated(
    bytes: &[u8],
    options: EncryptedOpenOptions<'_>,
) -> Result<AuthenticatedEncryptedArchive> {
    let parsed = parse_public(bytes, options.crypto_policy)?;
    let unlock = options.unlock.ok_or_else(no_recipient)?;
    let (afk, keys) = unlock_envelope(
        &parsed.envelope,
        parsed.features,
        unlock,
        options.crypto_policy,
    )?;
    let pci = physical_container_identity(bytes);
    let decoded = decrypt_segments(
        parsed.segments,
        &parsed.envelope,
        &keys,
        &parsed.footer,
        parsed.features,
        options,
    )?;
    drop(afk);
    let addressing = AddressingBinding {
        payload_suite_id: wire::PAYLOAD_SUITE_V1,
        recipient_set_digest: recipient_set_digest(&parsed.envelope.stanzas)?,
        commitment: parsed.envelope.commitment,
        archive_id: parsed.envelope.archive_id,
    };
    let opened = crate::ecf::open_encrypted_plain_parts(
        decoded.parts,
        options.resource_policy,
        options.decode_policy,
        pci,
    )?;
    Ok(AuthenticatedEncryptedArchive {
        opened,
        embedded_signatures: decoded.signatures,
        recipient_directory: decoded.recipient_directory,
        addressing,
    })
}

pub fn verify_encrypted(
    bytes: &[u8],
    options: EncryptedOpenOptions<'_>,
) -> Result<VerificationReport> {
    Ok(open_encrypted(bytes, options)?.report)
}

struct MutationState {
    opened: OpenedArchive,
    afk: super::Secret32,
    archive_id: [u8; 32],
    envelope: wire::CryptoEnvelope,
    signatures: Vec<SignatureRecord>,
    recipient_directory: Vec<RecipientDirectoryEntry>,
    payload_segments: std::collections::BTreeMap<u64, ReusableSegment>,
    include_index: bool,
}

fn open_for_mutation(bytes: &[u8], options: EncryptedOpenOptions<'_>) -> Result<MutationState> {
    let parsed = parse_public(bytes, options.crypto_policy)?;
    let unlock = options.unlock.ok_or_else(no_recipient)?;
    let (afk, keys) = unlock_envelope(
        &parsed.envelope,
        parsed.features,
        unlock,
        options.crypto_policy,
    )?;
    let pci = physical_container_identity(bytes);
    let decoded = decrypt_segments(
        parsed.segments,
        &parsed.envelope,
        &keys,
        &parsed.footer,
        parsed.features,
        options,
    )?;
    let include_index = decoded.index_present;
    let opened = crate::ecf::open_encrypted_plain_parts(
        decoded.parts,
        options.resource_policy,
        options.decode_policy,
        pci,
    )?;
    Ok(MutationState {
        opened,
        afk,
        archive_id: parsed.envelope.archive_id,
        envelope: parsed.envelope,
        signatures: decoded.signatures,
        recipient_directory: decoded.recipient_directory,
        payload_segments: decoded.payload_segments,
        include_index,
    })
}

/// Embeds a signature by re-authenticating CONTROL/footer data while retaining
/// the AFK, archive ID, and ordinal-compatible PAYLOAD ciphertext.
pub fn embed_signature(
    bytes: &[u8],
    options: EncryptedOpenOptions<'_>,
    signature: SignatureRecord,
) -> Result<EncryptedArchive> {
    let mut state = open_for_mutation(bytes, options)?;
    if state
        .signatures
        .iter()
        .any(|existing| existing == &signature)
    {
        return Err(wire::private_invalid(
            "embedded signature is an exact duplicate",
        ));
    }
    state.signatures.push(signature);
    state
        .signatures
        .sort_by_key(|value| value.encode().expect("validated signature"));
    let directory = state
        .recipient_directory
        .iter()
        .map(encode_recipient_directory_entry)
        .collect::<Result<Vec<_>>>()?;
    let parts = prepare_encrypted_plain_parts(&state.opened.archive)?;
    let padding = PaddingMode::try_from(state.envelope.padding_mode)?;
    let boundary = BoundaryMode::try_from(state.envelope.boundary_mode)?;
    let write = EncryptedWriteOptions {
        recipients: &[],
        password: None,
        padding,
        boundary,
        include_index: state.include_index,
        embedded_signatures: &state.signatures,
    };
    let policy = protection_policy(state.envelope.protection_policy)?;
    let output = encrypt_with_material(
        parts,
        &state.afk.0,
        state.archive_id,
        write,
        policy,
        state.envelope.stanzas,
        directory,
        Some(&state.payload_segments),
    )?;
    verify_mutation_output(&output.bytes, &state.afk.0)?;
    Ok(output)
}

/// Adds one X-Wing recipient while preserving the existing AFK, archive ID,
/// keyed chunking, and ordinal-compatible PAYLOAD ciphertext.
pub fn add_recipient(
    bytes: &[u8],
    options: EncryptedOpenOptions<'_>,
    recipient: &XWingRecipient,
) -> Result<EncryptedArchive> {
    let mut state = open_for_mutation(bytes, options)?;
    if state.envelope.protection_policy != ProtectionPolicy::HybridOnly as u8 {
        return Err(Diagnostic::new(
            OutcomeClass::PolicyRefused,
            ReasonCode::CryptoRecipientPolicyInvalid,
            "recipient addition is available only for HYBRID_ONLY archives",
        ));
    }
    if state
        .recipient_directory
        .iter()
        .any(|entry| entry.fingerprint == recipient.fingerprint())
    {
        return Err(Diagnostic::new(
            OutcomeClass::PolicyRefused,
            ReasonCode::CryptoRecipientPolicyInvalid,
            "recipient public key is already present",
        ));
    }
    let singleton = [recipient.clone()];
    let temporary = EncryptedWriteOptions {
        recipients: &singleton,
        password: None,
        padding: PaddingMode::try_from(state.envelope.padding_mode)?,
        boundary: BoundaryMode::try_from(state.envelope.boundary_mode)?,
        include_index: state.include_index,
        embedded_signatures: &[],
    };
    let (_, new_stanzas, new_directory) =
        build_recipients(&state.afk.0, &state.archive_id, &temporary)?;
    state.envelope.stanzas.extend(new_stanzas);
    let mut directory = state
        .recipient_directory
        .iter()
        .map(encode_recipient_directory_entry)
        .collect::<Result<Vec<_>>>()?;
    directory.extend(new_directory);
    directory.sort_by(|left, right| left[28..44].cmp(&right[28..44]));
    let parts = prepare_encrypted_plain_parts(&state.opened.archive)?;
    let write = EncryptedWriteOptions {
        recipients: &[],
        password: None,
        padding: temporary.padding,
        boundary: temporary.boundary,
        include_index: state.include_index,
        embedded_signatures: &state.signatures,
    };
    let output = encrypt_with_material(
        parts,
        &state.afk.0,
        state.archive_id,
        write,
        ProtectionPolicy::HybridOnly,
        state.envelope.stanzas,
        directory,
        Some(&state.payload_segments),
    )?;
    verify_mutation_output(&output.bytes, &state.afk.0)?;
    Ok(output)
}

/// Removes recipients only by creating a fresh encryption epoch. `retained`
/// must name every recipient that should unlock the replacement.
pub fn reencrypt_recipients(
    bytes: &[u8],
    options: EncryptedOpenOptions<'_>,
    retained: &[XWingRecipient],
) -> Result<EncryptedArchive> {
    let state = open_for_mutation(bytes, options)?;
    if state.envelope.protection_policy != ProtectionPolicy::HybridOnly as u8 || retained.is_empty()
    {
        return Err(Diagnostic::new(
            OutcomeClass::PolicyRefused,
            ReasonCode::CryptoRecipientPolicyInvalid,
            "recipient removal requires a HYBRID_ONLY archive and at least one retained recipient",
        ));
    }
    let current = state
        .recipient_directory
        .iter()
        .map(|entry| entry.fingerprint)
        .collect::<BTreeSet<_>>();
    let retained_set = retained
        .iter()
        .map(XWingRecipient::fingerprint)
        .collect::<BTreeSet<_>>();
    if retained_set.len() != retained.len()
        || !retained_set.is_subset(&current)
        || retained_set.len() >= current.len()
    {
        return Err(Diagnostic::new(
            OutcomeClass::PolicyRefused,
            ReasonCode::CryptoRecipientPolicyInvalid,
            "retained public keys must be a unique proper subset of the authenticated directory",
        ));
    }
    rotate_encryption_epoch(state, retained, None)
}

/// Replaces a password through fresh-AFK/archive-ID full re-encryption.
pub fn change_password(
    bytes: &[u8],
    options: EncryptedOpenOptions<'_>,
    new_password: &[u8],
) -> Result<EncryptedArchive> {
    let state = open_for_mutation(bytes, options)?;
    if state.envelope.protection_policy != ProtectionPolicy::PasswordOnly as u8
        || new_password.is_empty()
    {
        return Err(Diagnostic::new(
            OutcomeClass::PolicyRefused,
            ReasonCode::CryptoRecipientPolicyInvalid,
            "password rotation requires a PASSWORD_ONLY archive and a nonempty new password",
        ));
    }
    rotate_encryption_epoch(state, &[], Some(new_password))
}

fn rotate_encryption_epoch(
    state: MutationState,
    recipients: &[XWingRecipient],
    password: Option<&[u8]>,
) -> Result<EncryptedArchive> {
    let afk = super::Secret32(random::<32>()?);
    let archive_id = random::<32>()?;
    let keys = KeyHierarchy::derive(&afk.0, &archive_id)?;
    let boundary_mode = BoundaryMode::try_from(state.envelope.boundary_mode)?;
    let (boundary, chunker_prefix) = keys.boundary_key(boundary_mode)?;
    let profile = encrypted_profile(&state.opened.archive.descriptor.planner_id)?;
    let mut archive = crate::archive::replan_archive_encrypted(
        &state.opened.archive,
        profile,
        &boundary,
        chunker_prefix,
    )?;
    archive.descriptor.planner_id = match profile {
        CompressionProfile::Fast => "fast-enc-v1",
        CompressionProfile::Balanced => "balanced-enc-v1",
        CompressionProfile::Dense => "dense-enc-v1",
        CompressionProfile::Extreme => "extreme-enc-v1",
    }
    .to_owned();
    let parts = prepare_encrypted_plain_parts(&archive)?;
    let output = encrypt_with_file_key(
        parts,
        &afk.0,
        archive_id,
        EncryptedWriteOptions {
            recipients,
            password,
            padding: PaddingMode::try_from(state.envelope.padding_mode)?,
            boundary: boundary_mode,
            include_index: state.include_index,
            embedded_signatures: &state.signatures,
        },
    )?;
    verify_mutation_output(&output.bytes, &afk.0)?;
    Ok(output)
}

/// Mutations know the newly installed AFK, so they can authenticate and
/// structurally verify the complete replacement before the caller replaces a
/// filesystem object—even when the retained recipients' private identities
/// are intentionally unavailable to the mutating caller.
fn verify_mutation_output(bytes: &[u8], afk: &[u8; 32]) -> Result<()> {
    let options = EncryptedOpenOptions::new(None);
    let parsed = parse_public(bytes, options.crypto_policy)?;
    let keys = KeyHierarchy::derive(afk, &parsed.envelope.archive_id)?;
    let padding = PaddingMode::try_from(parsed.envelope.padding_mode)?;
    let boundary = BoundaryMode::try_from(parsed.envelope.boundary_mode)?;
    let expected_commitment = commitment(&keys, &parsed.envelope.archive_id, padding, boundary)?;
    if !bool::from(expected_commitment.ct_eq(&parsed.envelope.commitment)) {
        return Err(crypto_integrity(
            ReasonCode::CryptoKeyCommitmentFailed,
            "replacement archive key commitment did not self-verify",
        ));
    }
    let public_context = public_crypto_context(
        &parsed.envelope.archive_id,
        parsed.features,
        padding,
        boundary,
    )?;
    let expected_mac = envelope_mac(
        &keys,
        &public_context,
        &parsed.envelope.commitment,
        protection_policy(parsed.envelope.protection_policy)?,
        &parsed.envelope.stanzas,
    )?;
    if !bool::from(expected_mac.ct_eq(&parsed.envelope.envelope_mac)) {
        return Err(crypto_integrity(
            ReasonCode::CryptoEnvelopeAuthFailed,
            "replacement archive envelope did not self-verify",
        ));
    }
    let pci = physical_container_identity(bytes);
    let decoded = decrypt_segments(
        parsed.segments,
        &parsed.envelope,
        &keys,
        &parsed.footer,
        parsed.features,
        options,
    )?;
    crate::ecf::open_encrypted_plain_parts(
        decoded.parts,
        options.resource_policy,
        options.decode_policy,
        pci,
    )?;
    Ok(())
}

fn encrypted_profile(planner_id: &str) -> Result<CompressionProfile> {
    if planner_id.starts_with("fast-") {
        Ok(CompressionProfile::Fast)
    } else if planner_id.starts_with("balanced-") {
        Ok(CompressionProfile::Balanced)
    } else if planner_id.starts_with("dense-") {
        Ok(CompressionProfile::Dense)
    } else if planner_id.starts_with("extreme-") {
        Ok(CompressionProfile::Extreme)
    } else {
        Err(Diagnostic::new(
            OutcomeClass::Unsupported,
            ReasonCode::UnsupportedRequiredFeature,
            "cannot map encrypted planner ID to a frozen creation profile",
        ))
    }
}

pub fn inspect_encrypted(
    bytes: &[u8],
    unlock: Option<Unlock<'_>>,
    policy: CryptoPolicy,
) -> Result<CryptoInspection> {
    let parsed = parse_public(bytes, policy)?;
    let public = public_inspection(
        &parsed.envelope,
        bytes.len() as u64,
        Some(parsed.segment_count),
    )?;
    let authenticated = if let Some(unlock) = unlock {
        let mut options = EncryptedOpenOptions::new(Some(unlock));
        options.crypto_policy = policy;
        let opened = open_encrypted(bytes, options)?;
        Some(inspect(&opened)?)
    } else {
        None
    };
    let authenticated_descriptor = authenticated.as_ref().map(|view| {
        let present = parsed.features & super::FEATURE_PRIVATE_RESOURCE_DECLARATION_V1 != 0;
        AuthenticatedDescriptorInspection {
            record_version: if present { 2 } else { 1 },
            producer_declaration_present: present,
            independently_validated: true,
            declared_budget: present.then_some(view.declared_resources),
            declared_decode: present.then_some(view.decode_requirements),
        }
    });
    Ok(CryptoInspection {
        public,
        authenticated,
        authenticated_descriptor,
    })
}

struct PublicParsed<'a> {
    features: u64,
    envelope: wire::CryptoEnvelope,
    segments: &'a [u8],
    segment_count: u64,
    footer: ParsedFooter,
}

fn parse_public(bytes: &[u8], policy: CryptoPolicy) -> Result<PublicParsed<'_>> {
    if bytes.len() < PREAMBLE_LEN as usize + ENCRYPTED_FOOTER_LEN {
        return Err(truncated("encrypted archive is shorter than fixed framing"));
    }
    let preamble = &bytes[..PREAMBLE_LEN as usize];
    if preamble[..8] != MAGIC {
        return Err(Diagnostic::new(
            OutcomeClass::Unsupported,
            ReasonCode::BadMagic,
            "Entrybound magic mismatch",
        ));
    }
    if u16::from_be_bytes(preamble[8..10].try_into().unwrap()) != 0
        || u16::from_be_bytes(preamble[10..12].try_into().unwrap()) != 1
        || u32::from_be_bytes(preamble[12..16].try_into().unwrap()) != PREAMBLE_LEN as u32
    {
        return Err(Diagnostic::new(
            OutcomeClass::Unsupported,
            ReasonCode::UnsupportedVersion,
            "unsupported encrypted format version",
        ));
    }
    let features = u64::from_be_bytes(preamble[16..24].try_into().unwrap());
    let required = super::FEATURE_ENCRYPTED_INDEXED_V1
        | super::FEATURE_PAYLOAD_SUITE_V1
        | super::FEATURE_PADDING;
    if features & required != required
        || features & crate::ecf::FEATURE_STREAM_LAYOUT_V1 != 0
        || features & !crate::ecf::SUPPORTED_INCOMPAT_FEATURES != 0
    {
        return Err(Diagnostic::new(
            OutcomeClass::Unsupported,
            ReasonCode::CryptoSuiteUnsupported,
            "invalid encrypted INDEXED feature set",
        ));
    }
    if sha256_exact(&preamble[16..40]).as_bytes() != &preamble[40..72]
        || preamble[24..40].iter().any(|byte| *byte != 0)
        || preamble[72] != 1
        || preamble[73] != 1
        || preamble[74] != 0
        || preamble[75..].iter().any(|byte| *byte != 0)
    {
        return Err(Diagnostic::new(
            OutcomeClass::Nonconforming,
            ReasonCode::NoncanonicalEncoding,
            "encrypted preamble violates zero sentinels",
        ));
    }
    let footer = parse_footer(bytes)?;
    if footer.total_len != bytes.len() as u64 {
        return Err(Diagnostic::new(
            OutcomeClass::Corrupt,
            ReasonCode::IncorrectTotalLength,
            "encrypted footer total length disagrees with EOF",
        ));
    }
    if footer.preamble_digest != *sha256_exact(preamble).as_bytes() {
        return Err(crypto_integrity(
            ReasonCode::FooterBindingMismatch,
            "encrypted footer/preamble binding mismatch",
        ));
    }
    let expected_segments_offset = footer
        .envelope_offset
        .checked_add(footer.envelope_len)
        .ok_or_else(|| resource_refused("encrypted envelope extent overflow"))?;
    let expected_footer_offset = footer
        .segments_offset
        .checked_add(footer.segments_len)
        .ok_or_else(|| resource_refused("encrypted segments extent overflow"))?;
    if footer.envelope_offset != PREAMBLE_LEN
        || footer.segments_offset != expected_segments_offset
        || expected_footer_offset != bytes.len() as u64 - ENCRYPTED_FOOTER_LEN as u64
    {
        return Err(Diagnostic::new(
            OutcomeClass::Nonconforming,
            ReasonCode::SectionStructure,
            "encrypted sections are not contiguous and canonical",
        ));
    }
    let (envelope_payload, envelope_complete) =
        parse_section(bytes, footer.envelope_offset, SECTION_ENVELOPE)?;
    if envelope_complete != footer.envelope_len
        || envelope_payload.len() as u64 > policy.max_envelope_bytes
    {
        return Err(resource_refused(
            "CryptoEnvelope extent exceeds policy or footer",
        ));
    }
    let envelope = wire::CryptoEnvelope::decode(envelope_payload)?;
    if envelope.stanzas.len() as u32 > policy.max_stanzas {
        return Err(resource_refused("recipient count exceeds caller policy"));
    }
    for stanza in &envelope.stanzas {
        if stanza.encode()?.len() as u64 > policy.max_stanza_bytes {
            return Err(resource_refused("recipient stanza exceeds caller policy"));
        }
    }
    let (segments, segments_complete) =
        parse_section(bytes, footer.segments_offset, SECTION_SEGMENTS)?;
    if segments_complete != footer.segments_len
        || *sha256_exact(segments).as_bytes() != footer.segments_digest
    {
        return Err(crypto_integrity(
            ReasonCode::SectionDigestMismatch,
            "encrypted segments digest mismatch",
        ));
    }
    let segment_count = scan_public_segments(segments, policy)?;
    let padding = PaddingMode::try_from(envelope.padding_mode)?;
    let boundary = BoundaryMode::try_from(envelope.boundary_mode)?;
    let context = public_crypto_context(&envelope.archive_id, features, padding, boundary)?;
    if !constant_time_eq(&Sha256::digest(&context), &footer.public_context_digest) {
        return Err(crypto_integrity(
            ReasonCode::FooterBindingMismatch,
            "public crypto context digest mismatch",
        ));
    }
    validate_envelope_policy(&envelope, features)?;
    Ok(PublicParsed {
        features,
        envelope,
        segments,
        segment_count,
        footer,
    })
}

fn scan_public_segments(bytes: &[u8], policy: CryptoPolicy) -> Result<u64> {
    let mut cursor = 0usize;
    let mut ordinal = 0u64;
    while cursor < bytes.len() {
        if ordinal >= policy.max_segments {
            return Err(resource_refused("encrypted segment count exceeds policy"));
        }
        let (extent, _, _, count, _) = parse_segment_header(&bytes[cursor..], ordinal)?;
        if count > policy.max_messages_per_segment {
            return Err(resource_refused("segment DATA count exceeds caller policy"));
        }
        if extent as u64 > policy.max_working_memory_bytes {
            return Err(resource_refused(
                "segment extent exceeds crypto working-memory policy",
            ));
        }
        cursor = cursor
            .checked_add(extent)
            .ok_or_else(|| segment_invalid("segment extent overflow"))?;
        if cursor > bytes.len() {
            return Err(truncated("segment extent is truncated"));
        }
        ordinal += 1;
    }
    Ok(ordinal)
}

fn unlock_envelope(
    envelope: &wire::CryptoEnvelope,
    features: u64,
    unlock: Unlock<'_>,
    policy: CryptoPolicy,
) -> Result<(super::Secret32, KeyHierarchy)> {
    let padding = PaddingMode::try_from(envelope.padding_mode)?;
    let boundary = BoundaryMode::try_from(envelope.boundary_mode)?;
    let context = public_crypto_context(&envelope.archive_id, features, padding, boundary)?;
    let protection = match envelope.protection_policy {
        1 => ProtectionPolicy::HybridOnly,
        2 => ProtectionPolicy::PasswordOnly,
        _ => {
            return Err(Diagnostic::new(
                OutcomeClass::Nonconforming,
                ReasonCode::CryptoRecipientPolicyInvalid,
                "unknown recipient policy",
            ));
        }
    };
    let mut attempts = 0u32;
    for stanza in &envelope.stanzas {
        let method_secret = match (&unlock, stanza.stanza_type) {
            (Unlock::Identity(identity), 1) => {
                let mut ciphertext = Ciphertext::default();
                if stanza.encapsulation.len() != ciphertext.len() {
                    return Err(stanza_invalid("X-Wing encapsulation must be 1120 bytes"));
                }
                ciphertext.copy_from_slice(&stanza.encapsulation);
                let shared = identity.secret.decapsulate(&ciphertext);
                Some(super::Secret32(
                    shared
                        .as_slice()
                        .try_into()
                        .expect("X-Wing shared secret length"),
                ))
            }
            (Unlock::Password(password), 2) => Some(password_secret(
                password,
                &stanza.method_parameters,
                policy,
            )?),
            _ => None,
        };
        let Some(method_secret) = method_secret else {
            continue;
        };
        attempts += 1;
        if attempts > policy.max_identity_attempts {
            return Err(resource_refused(
                "recipient identity attempt limit exceeded",
            ));
        }
        let candidate = match open_afk(&envelope.archive_id, &method_secret.0, stanza) {
            Ok(value) => value,
            Err(_) => continue,
        };
        let keys = KeyHierarchy::derive(&candidate.0, &envelope.archive_id)?;
        let expected_commitment = commitment(&keys, &envelope.archive_id, padding, boundary)?;
        if !constant_time_eq(&expected_commitment, &envelope.commitment) {
            return Err(crypto_integrity(
                ReasonCode::CryptoKeyCommitmentFailed,
                "candidate file-key commitment failed",
            ));
        }
        let expected_mac = envelope_mac(
            &keys,
            &context,
            &envelope.commitment,
            protection,
            &envelope.stanzas,
        )?;
        if !constant_time_eq(&expected_mac, &envelope.envelope_mac) {
            return Err(crypto_integrity(
                ReasonCode::CryptoEnvelopeAuthFailed,
                "CryptoEnvelope authentication failed",
            ));
        }
        return Ok((candidate, keys));
    }
    Err(no_recipient())
}

struct DecryptedPrivate {
    parts: EncryptedDecodedParts,
    signatures: Vec<SignatureRecord>,
    recipient_directory: Vec<RecipientDirectoryEntry>,
    payload_segments: std::collections::BTreeMap<u64, ReusableSegment>,
    index_present: bool,
}

fn decrypt_segments(
    bytes: &[u8],
    envelope: &wire::CryptoEnvelope,
    keys: &KeyHierarchy,
    footer: &ParsedFooter,
    features: u64,
    options: EncryptedOpenOptions<'_>,
) -> Result<DecryptedPrivate> {
    let policy = options.crypto_policy;
    let resource_policy = options.resource_policy;
    let decode_policy = options.decode_policy;
    let padding = PaddingMode::try_from(envelope.padding_mode)?;
    let mut cursor = 0usize;
    let mut ordinal = 0u64;
    let mut completed = Vec::new();
    let mut payload_segments = std::collections::BTreeMap::new();
    let mut segment_salts = BTreeSet::new();
    let mut collector = ObjectCollector::default();
    let mut archive_final = None;
    let mut actual_terminal_offset = None;
    while cursor < bytes.len() {
        let segment_start = cursor;
        if ordinal >= policy.max_segments {
            return Err(resource_refused("encrypted segment count exceeds policy"));
        }
        let (segment_len, class, salt, count, header) =
            parse_segment_header(&bytes[cursor..], ordinal)?;
        if !segment_salts.insert((class, salt)) {
            return Err(segment_invalid("segment salt is reused in one key domain"));
        }
        if count > policy.max_messages_per_segment {
            return Err(resource_refused("segment DATA count exceeds caller policy"));
        }
        if segment_len as u64 > policy.max_working_memory_bytes {
            return Err(resource_refused(
                "segment extent exceeds crypto working-memory policy",
            ));
        }
        let end = cursor
            .checked_add(segment_len)
            .ok_or_else(|| segment_invalid("segment extent overflow"))?;
        let segment = bytes
            .get(cursor..end)
            .ok_or_else(|| truncated("segment extent is truncated"))?;
        let key = segment_key(keys, &envelope.archive_id, class, ordinal, &salt)?;
        let decrypt_context = RecordDecryptContext {
            archive_id: &envelope.archive_id,
            segment_header: header,
            padding,
            segment_class: class,
        };
        let mut record_cursor = SEGMENT_HEADER_LEN;
        let mut exact_data = Vec::new();
        let mut private_total = 0u64;
        let mut ciphertext_total = 0u64;
        let mut completed_objects = 0usize;
        for counter in 0..count {
            let (protected, ciphertext, next) = parse_protected(
                segment,
                record_cursor,
                RECORD_DATA,
                ordinal,
                u64::from(counter),
                policy,
            )?;
            let private = decrypt_record(&key, &decrypt_context, protected, ciphertext, false)?;
            if private.len() as u64 > policy.max_private_record_bytes {
                return Err(resource_refused("private record exceeds caller policy"));
            }
            private_total = private_total
                .checked_add(private.len() as u64)
                .ok_or_else(|| segment_invalid("private total overflow"))?;
            ciphertext_total = ciphertext_total
                .checked_add(ciphertext.len() as u64)
                .ok_or_else(|| segment_invalid("ciphertext total overflow"))?;
            exact_data.extend_from_slice(protected);
            exact_data.extend_from_slice(ciphertext);
            if let Some(object) = collector.push(wire::decode_private_fragment(&private)?)? {
                completed_objects += 1;
                if let Some(final_value) =
                    collector.dispatch(object, features, resource_policy, decode_policy)?
                    && archive_final.replace(final_value).is_some()
                {
                    return Err(segment_invalid("duplicate ArchiveFinal"));
                }
            }
            record_cursor = next;
        }
        let (end_header, end_ciphertext, next) = parse_protected(
            segment,
            record_cursor,
            RECORD_END,
            ordinal,
            u64::from(count),
            policy,
        )?;
        if next != segment.len() {
            return Err(segment_invalid("bytes follow segment END"));
        }
        let end_private = decrypt_record(&key, &decrypt_context, end_header, end_ciphertext, true)?;
        let data_digest: [u8; 32] = Sha256::digest(wire::t1(
            "entrybound/segment-data/v1",
            &[header, &data_sequence(count, &exact_data)],
        )?)
        .into();
        validate_segment_end(
            &end_private,
            ordinal,
            class,
            count,
            private_total,
            ciphertext_total,
            &data_digest,
        )?;
        let digest: [u8; 32] = Sha256::digest(wire::t1(
            "entrybound/segment-digest/v1",
            &[header, &exact_data, end_header, end_ciphertext],
        )?)
        .into();
        completed.push(digest);
        if class == SEGMENT_PAYLOAD {
            payload_segments.insert(
                ordinal,
                ReusableSegment {
                    class,
                    bytes: segment.to_vec(),
                    digest,
                },
            );
        }
        cursor = end;
        ordinal += 1;
        if cursor == bytes.len() {
            if class != SEGMENT_CONTROL || count != 1 || completed_objects != 1 {
                return Err(segment_invalid("terminal segment shape is invalid"));
            }
            actual_terminal_offset = Some(
                footer
                    .segments_offset
                    .checked_add(SECTION_HEADER_LEN)
                    .and_then(|value| value.checked_add(segment_start as u64))
                    .ok_or_else(|| segment_invalid("terminal segment locator overflow"))?,
            );
        }
    }
    if collector.partial.is_some() {
        return Err(truncated("private fragments are incomplete"));
    }
    let final_value = archive_final.ok_or_else(|| truncated("ArchiveFinal is missing"))?;
    if final_value.segment_count != ordinal || completed.len() != ordinal as usize {
        return Err(segment_invalid("ArchiveFinal segment count mismatch"));
    }
    let actual_terminal_offset =
        actual_terminal_offset.ok_or_else(|| truncated("terminal segment is missing"))?;
    if footer.terminal_offset != actual_terminal_offset
        || footer.archive_final_offset
            != actual_terminal_offset
                .checked_add(SEGMENT_HEADER_LEN as u64)
                .ok_or_else(|| segment_invalid("ArchiveFinal locator overflow"))?
    {
        return Err(segment_invalid(
            "footer terminal or ArchiveFinal locator is not canonical",
        ));
    }
    let prior = segment_sequence_digest(&completed[..completed.len() - 1])?;
    if !constant_time_eq(&prior, &final_value.segment_sequence) {
        return Err(segment_invalid("ordered segment-sequence digest mismatch"));
    }
    let boundary = BoundaryMode::try_from(envelope.boundary_mode)?;
    let context = public_crypto_context(&envelope.archive_id, features, padding, boundary)?;
    let context_digest: [u8; 32] = Sha256::digest(context).into();
    let core = footer_core(
        footer.total_len,
        footer.envelope_offset,
        footer.envelope_len,
        footer.segments_offset,
        footer.segments_len,
        footer.terminal_offset,
        footer.archive_final_offset,
        &footer.preamble_digest,
        &context_digest,
    )?;
    if !constant_time_eq(&Sha256::digest(core), &final_value.footer_core) {
        return Err(crypto_integrity(
            ReasonCode::FooterBindingMismatch,
            "authenticated footer-core mismatch",
        ));
    }
    let stanza_sequence = wire::encode_stanza_sequence(&envelope.stanzas)?;
    let set_digest: [u8; 32] = Sha256::digest(wire::t1(
        "entrybound/recipient-set/v1",
        &[&stanza_sequence],
    )?)
    .into();
    if !constant_time_eq(&set_digest, &final_value.recipient_set)
        || collector.descriptor_id != Some(final_value.descriptor_id)
        || collector.manifest_id != Some(final_value.manifest_id)
    {
        return Err(crypto_integrity(
            ReasonCode::CryptoEnvelopeAuthFailed,
            "ArchiveFinal recipient/object binding mismatch",
        ));
    }
    collector.validate_recipient_directory(&envelope.stanzas)?;
    let signature_present = collector.signatures.is_some();
    if signature_present != (features & super::FEATURE_SIGNATURE_ED25519_V1 != 0) {
        return Err(wire::private_invalid(
            "embedded-signature feature and collection presence disagree",
        ));
    }
    let signatures = collector
        .signatures
        .as_deref()
        .unwrap_or_default()
        .iter()
        .map(|bytes| SignatureRecord::decode(bytes))
        .collect::<Result<Vec<_>>>()?;
    let recipient_directory = collector.decode_recipient_directory()?;
    let index_present = collector.index_present;
    let decoded = collector.finish(
        FeatureSet {
            incompat: features & !super::CRYPTO_FEATURES,
            read_only_compat: 0,
            compat: 0,
        },
        final_value.entry_count,
        final_value.total_logical,
    )?;
    let roots = decode_descriptor_roots(&decoded.descriptor)?;
    if !constant_time_eq(&roots.0, &final_value.lai)
        || !constant_time_eq(&roots.1, &final_value.pcr)
        || !constant_time_eq(&roots.2, &final_value.aux)
    {
        return Err(segment_invalid(
            "ArchiveFinal identities disagree with Descriptor",
        ));
    }
    if decoded.chunk_frames.len() as u64 != final_value.chunk_count {
        return Err(segment_invalid("ArchiveFinal unique Chunk count mismatch"));
    }
    Ok(DecryptedPrivate {
        parts: decoded,
        signatures,
        recipient_directory,
        payload_segments,
        index_present,
    })
}

#[derive(Default)]
struct ObjectCollector {
    partial: Option<PartialObject>,
    descriptor: Option<Vec<u8>>,
    descriptor_id: Option<[u8; 32]>,
    descriptor_version: Option<u16>,
    plans: Option<Vec<Vec<u8>>>,
    dictionaries: Option<Vec<Vec<u8>>>,
    groups: Option<Vec<Vec<u8>>>,
    reconstruction_data: Option<Vec<Vec<u8>>>,
    reconstruction_regions: Option<Vec<Vec<u8>>>,
    chunks: Vec<Vec<u8>>,
    manifest: Option<Vec<Vec<u8>>>,
    manifest_id: Option<[u8; 32]>,
    fidelity: Option<Vec<u8>>,
    recipient_directory: Option<Vec<Vec<u8>>>,
    signatures: Option<Vec<Vec<u8>>>,
    index_present: bool,
    last_rank: u8,
}

struct PartialObject {
    id: [u8; 32],
    total: u64,
    count: u32,
    next_index: u32,
    bytes: Vec<u8>,
}

impl ObjectCollector {
    fn push(&mut self, fragment: wire::PrivateFragment) -> Result<Option<Vec<u8>>> {
        if fragment.index == 0 {
            if self.partial.is_some() || fragment.offset != 0 {
                return Err(wire::private_invalid(
                    "fragment sequence overlaps or starts late",
                ));
            }
            self.partial = Some(PartialObject {
                id: fragment.object_id,
                total: fragment.total_len,
                count: fragment.count,
                next_index: 0,
                bytes: Vec::with_capacity(
                    usize::try_from(fragment.total_len)
                        .map_err(|_| resource_refused("encrypted object exceeds usize"))?,
                ),
            });
        }
        let partial = self
            .partial
            .as_mut()
            .ok_or_else(|| wire::private_invalid("orphan noninitial fragment"))?;
        if fragment.object_id != partial.id
            || fragment.total_len != partial.total
            || fragment.count != partial.count
            || fragment.index != partial.next_index
            || fragment.offset != partial.bytes.len() as u64
        {
            return Err(wire::private_invalid(
                "fragment sequence is not contiguous and exact",
            ));
        }
        partial.bytes.extend_from_slice(&fragment.bytes);
        partial.next_index += 1;
        if partial.next_index == partial.count {
            let complete = self.partial.take().expect("present partial object");
            if complete.bytes.len() as u64 != complete.total
                || wire::encrypted_object_id(&complete.bytes)? != complete.id
            {
                return Err(crypto_integrity(
                    ReasonCode::CryptoPrivateObjectInvalid,
                    "encrypted object identity mismatch",
                ));
            }
            return Ok(Some(complete.bytes));
        }
        Ok(None)
    }

    fn dispatch(
        &mut self,
        object: Vec<u8>,
        features: u64,
        resource_policy: ResourceBudget,
        decode_policy: DecodeRequirements,
    ) -> Result<Option<ArchiveFinal>> {
        let id = wire::encrypted_object_id(&object)?;
        let (kind, payload) = wire::decode_private_object(&object)?;
        match kind {
            wire::PRIVATE_OBJECT_RECORD => match wire::record_kind(payload)? {
                1 => {
                    self.rank(1)?;
                    let declaration = private_descriptor_declaration(payload)?;
                    let feature_present =
                        features & super::FEATURE_PRIVATE_RESOURCE_DECLARATION_V1 != 0;
                    if feature_present != (declaration.version == 2) {
                        return Err(wire::private_invalid(
                            "Descriptor version and private-resource-declaration-v1 feature disagree",
                        ));
                    }
                    if let (Some(budget), Some(decode)) = (declaration.budget, declaration.decode) {
                        crate::ecf::enforce_caller_policy(budget, resource_policy)?;
                        crate::ecf::enforce_decode_policy(decode, decode_policy)?;
                    }
                    set_once(&mut self.descriptor, payload.to_vec(), "Descriptor")?;
                    self.descriptor_id = Some(id);
                    self.descriptor_version = Some(declaration.version);
                }
                5 => {
                    self.rank(10)?;
                    set_once(&mut self.fidelity, payload.to_vec(), "Fidelity")?;
                }
                wire::RECORD_ARCHIVE_FINAL => {
                    self.rank(13)?;
                    return Ok(Some(decode_archive_final(payload)?));
                }
                _ => return Err(wire::private_invalid("forbidden singleton private record")),
            },
            wire::PRIVATE_OBJECT_CHUNK => {
                self.rank(8)?;
                if payload.len() < 4 || &payload[..4] != b"EBCH" {
                    return Err(wire::private_invalid(
                        "CHUNK_FRAME EBPO payload has wrong magic",
                    ));
                }
                self.chunks.push(payload.to_vec());
            }
            wire::PRIVATE_OBJECT_SEQUENCE => {
                let (collection, items) = wire::decode_sequence_container(payload)?;
                match collection {
                    wire::COLLECTION_TRANSFORM_PLANS => {
                        self.rank(2)?;
                        set_once(&mut self.plans, items, "TransformPlans")?;
                    }
                    wire::COLLECTION_CHUNK_GROUPS => {
                        self.rank(3)?;
                        set_once(&mut self.groups, items, "ChunkGroups")?;
                    }
                    wire::COLLECTION_RECIPIENT_DIRECTORY => {
                        self.rank(4)?;
                        set_once(&mut self.recipient_directory, items, "RecipientDirectory")?;
                    }
                    wire::COLLECTION_DICTIONARIES => {
                        self.rank(5)?;
                        set_once(&mut self.dictionaries, items, "Dictionaries")?;
                    }
                    wire::COLLECTION_RECONSTRUCTION_DATA => {
                        self.rank(6)?;
                        set_once(&mut self.reconstruction_data, items, "ReconstructionData")?;
                    }
                    wire::COLLECTION_RECONSTRUCTION_REGIONS => {
                        self.rank(7)?;
                        set_once(
                            &mut self.reconstruction_regions,
                            items,
                            "ReconstructionRegions",
                        )?;
                    }
                    wire::COLLECTION_MANIFEST => {
                        self.rank(9)?;
                        set_once(&mut self.manifest, items, "Manifest")?;
                        self.manifest_id = Some(id);
                    }
                    wire::COLLECTION_SIGNATURES => {
                        self.rank(11)?;
                        for item in &items {
                            SignatureRecord::decode(item)?;
                        }
                        set_once(&mut self.signatures, items, "EmbeddedSignatures")?;
                    }
                    wire::COLLECTION_INDEX => {
                        self.rank(12)?;
                        if self.index_present {
                            return Err(wire::private_invalid("duplicate encrypted Index object"));
                        }
                        self.index_present = true;
                    }
                    _ => return Err(wire::private_invalid("unsupported private EBCS collection")),
                }
            }
            _ => unreachable!(),
        }
        Ok(None)
    }

    fn validate_recipient_directory(&self, stanzas: &[wire::RecipientStanza]) -> Result<()> {
        let hybrid = stanzas.iter().all(|stanza| stanza.protection_class == 1);
        if !hybrid {
            if self.recipient_directory.is_some() {
                return Err(stanza_invalid(
                    "password archive contains a recipient directory",
                ));
            }
            return Ok(());
        }
        let expected = stanzas
            .iter()
            .filter(|stanza| stanza.stanza_type == 1)
            .map(|stanza| stanza.stanza_id)
            .collect::<BTreeSet<_>>();
        if expected.is_empty() {
            if self.recipient_directory.is_some() {
                return Err(wire::private_invalid(
                    "recipient directory has no supported public-key stanza",
                ));
            }
            return Ok(());
        }
        let directory = self.recipient_directory.as_ref().ok_or_else(|| {
            wire::private_invalid("hybrid archive is missing its private recipient directory")
        })?;
        if directory.len() != expected.len() {
            return Err(wire::private_invalid(
                "recipient directory and supported stanza counts disagree",
            ));
        }
        let mut found = BTreeSet::new();
        for bytes in directory {
            let (record, consumed) = decode_record(bytes)?;
            if consumed != bytes.len() || record.kind != wire::RECORD_RECIPIENT_DIRECTORY {
                return Err(wire::private_invalid(
                    "recipient directory contains a wrong record",
                ));
            }
            record.expect_tags(&[1, 2, 3, 4], &[])?;
            let id: [u8; 16] = exact(record.field(1)?.as_bytes()?)?;
            if record.field(2)?.as_u16()? != 1
                || record.field(3)?.as_bytes()?.len() != 32
                || record.field(4)?.as_utf8()?.len() > 1_024
                || !found.insert(id)
            {
                return Err(wire::private_invalid(
                    "recipient directory entry is malformed or duplicate",
                ));
            }
        }
        if found != expected {
            return Err(wire::private_invalid(
                "recipient directory IDs do not match authenticated stanzas",
            ));
        }
        Ok(())
    }

    fn decode_recipient_directory(&self) -> Result<Vec<RecipientDirectoryEntry>> {
        self.recipient_directory
            .as_deref()
            .unwrap_or_default()
            .iter()
            .map(|bytes| {
                let (record, consumed) = decode_record(bytes)?;
                if consumed != bytes.len() || record.kind != wire::RECORD_RECIPIENT_DIRECTORY {
                    return Err(wire::private_invalid(
                        "recipient directory contains a wrong record",
                    ));
                }
                record.expect_tags(&[1, 2, 3, 4], &[])?;
                Ok(RecipientDirectoryEntry {
                    stanza_id: exact(record.field(1)?.as_bytes()?)?,
                    stanza_type: record.field(2)?.as_u16()?,
                    fingerprint: exact(record.field(3)?.as_bytes()?)?,
                    label: record.field(4)?.as_utf8()?.to_owned(),
                })
            })
            .collect()
    }

    fn rank(&mut self, rank: u8) -> Result<()> {
        if self.descriptor.is_none() && rank != 1 {
            return Err(wire::private_invalid(
                "authenticated Descriptor must be the first private object",
            ));
        }
        if rank < self.last_rank || (rank == self.last_rank && rank != 8) {
            return Err(wire::private_invalid(
                "private objects are duplicated or out of order",
            ));
        }
        self.last_rank = rank;
        Ok(())
    }

    fn finish(
        self,
        ordinary_features: FeatureSet,
        entry_count: u64,
        total_logical: u64,
    ) -> Result<EncryptedDecodedParts> {
        Ok(EncryptedDecodedParts {
            ordinary_features,
            descriptor_version: self
                .descriptor_version
                .ok_or_else(|| wire::private_invalid("missing Descriptor version"))?,
            descriptor: self
                .descriptor
                .ok_or_else(|| wire::private_invalid("missing Descriptor"))?,
            transform_plans: self
                .plans
                .ok_or_else(|| wire::private_invalid("missing TransformPlans"))?,
            dictionaries: self.dictionaries.unwrap_or_default(),
            chunk_groups: self.groups.unwrap_or_default(),
            reconstruction_data: self.reconstruction_data.unwrap_or_default(),
            reconstruction_regions: self.reconstruction_regions.unwrap_or_default(),
            chunk_frames: self.chunks,
            manifest: self
                .manifest
                .ok_or_else(|| wire::private_invalid("missing Manifest"))?,
            fidelity: self
                .fidelity
                .ok_or_else(|| wire::private_invalid("missing Fidelity"))?,
            entry_count,
            total_logical,
        })
    }
}

fn set_once<T>(slot: &mut Option<T>, value: T, name: &str) -> Result<()> {
    if slot.replace(value).is_some() {
        return Err(wire::private_invalid(format!(
            "duplicate encrypted {name} object"
        )));
    }
    Ok(())
}

fn collection_object(kind: u16, records: &[Vec<u8>]) -> Result<Vec<u8>> {
    wire::private_object(
        wire::PRIVATE_OBJECT_SEQUENCE,
        &wire::sequence_container(kind, records)?,
    )
}

fn encode_encrypted_index_entry(
    chunk_id: &[u8; 32],
    segment_ordinal: u64,
    fragment_count: u32,
) -> Result<Vec<u8>> {
    let mut record = RecordBuilder::new(wire::RECORD_ENCRYPTED_INDEX);
    record
        .bytes(1, chunk_id)?
        .u64(2, segment_ordinal)?
        .u64(3, 0)?
        .u32(4, fragment_count)?;
    record.finish()
}

struct FragmentRef<'a> {
    id: [u8; 32],
    total: u64,
    index: u32,
    count: u32,
    offset: u64,
    bytes: &'a [u8],
}

fn fragment_objects<'a>(class: u8, objects: &'a [Vec<u8>]) -> Result<Vec<FragmentRef<'a>>> {
    let maximum = if class == SEGMENT_CONTROL {
        MAX_CONTROL_PRIVATE
    } else {
        MAX_PAYLOAD_PRIVATE
    };
    let capacity = maximum - 512;
    let mut fragments = Vec::new();
    for object in objects {
        if object.is_empty() || object.len() > 1 << 30 {
            return Err(wire::private_invalid(
                "encrypted object size is outside v1 bounds",
            ));
        }
        let count = object.len().div_ceil(capacity);
        let id = wire::encrypted_object_id(object)?;
        for (index, bytes) in object.chunks(capacity).enumerate() {
            fragments.push(FragmentRef {
                id,
                total: object.len() as u64,
                index: index as u32,
                count: count as u32,
                offset: (index * capacity) as u64,
                bytes,
            });
        }
    }
    Ok(fragments)
}

fn build_segment(
    class: u8,
    ordinal: u64,
    objects: &[Vec<u8>],
    archive_id: &[u8; 32],
    keys: &KeyHierarchy,
    padding: PaddingMode,
) -> Result<BuiltSegment> {
    let fragments = fragment_objects(class, objects)?;
    if fragments.len() > MAX_DATA_MESSAGES {
        return Err(resource_refused("too many DATA records"));
    }
    let salt = random::<16>()?;
    let predicted = predict_segment_extent_from_fragments(class, &fragments, padding)?;
    let header = segment_header(class, ordinal, &salt, fragments.len() as u32, predicted)?;
    let key = segment_key(keys, archive_id, class, ordinal, &salt)?;
    let mut data = Vec::new();
    let mut unpadded = 0u64;
    let mut ciphertext_total = 0u64;
    for (counter, fragment) in fragments.iter().enumerate() {
        let private = wire::encode_private_fragment(
            &fragment.id,
            fragment.total,
            fragment.index,
            fragment.count,
            fragment.offset,
            fragment.bytes,
        )?;
        unpadded = unpadded
            .checked_add(private.len() as u64)
            .ok_or_else(|| resource_refused("segment private total overflow"))?;
        let padded = pad_private(&private, class, padding)?;
        let protected = protected_header(
            RECORD_DATA,
            ordinal,
            counter as u64,
            (padded.len() + 16) as u64,
        )?;
        let ciphertext = aead_seal(
            &key.0,
            &data_nonce(counter as u64),
            &record_ad(archive_id, &header, &protected)?,
            &padded,
        )?;
        ciphertext_total += ciphertext.len() as u64;
        data.extend_from_slice(&protected);
        data.extend_from_slice(&ciphertext);
    }
    if unpadded > MAX_SEGMENT_PRIVATE {
        return Err(resource_refused("segment exceeds 1 GiB"));
    }
    let data_digest: [u8; 32] = Sha256::digest(wire::t1(
        "entrybound/segment-data/v1",
        &[&header, &data_sequence(fragments.len() as u32, &data)],
    )?)
    .into();
    let end_record = encode_segment_end(
        ordinal,
        class,
        fragments.len() as u32,
        unpadded,
        ciphertext_total,
        &data_digest,
    )?;
    let padded_end = pad_private(&end_record, SEGMENT_CONTROL, padding)?;
    let end_header = protected_header(
        RECORD_END,
        ordinal,
        fragments.len() as u64,
        (padded_end.len() + 16) as u64,
    )?;
    let end_ciphertext = aead_seal(
        &key.0,
        &end_nonce(fragments.len() as u64),
        &record_ad(archive_id, &header, &end_header)?,
        &padded_end,
    )?;
    let digest: [u8; 32] = Sha256::digest(wire::t1(
        "entrybound/segment-digest/v1",
        &[&header, &data, &end_header, &end_ciphertext],
    )?)
    .into();
    let mut bytes = header;
    bytes.extend_from_slice(&data);
    bytes.extend_from_slice(&end_header);
    bytes.extend_from_slice(&end_ciphertext);
    if bytes.len() as u64 != predicted {
        return Err(segment_invalid("segment extent mismatch"));
    }
    Ok(BuiltSegment { bytes, digest })
}

fn build_unique_segment(
    class: u8,
    ordinal: u64,
    objects: &[Vec<u8>],
    archive_id: &[u8; 32],
    keys: &KeyHierarchy,
    padding: PaddingMode,
    used: &mut BTreeSet<(u8, [u8; 16])>,
) -> Result<BuiltSegment> {
    for _ in 0..16 {
        let built = build_segment(class, ordinal, objects, archive_id, keys, padding)?;
        let salt: [u8; 16] = built.bytes[16..32].try_into().unwrap();
        if used.insert((class, salt)) {
            return Ok(built);
        }
    }
    Err(resource_refused(
        "OS randomness repeatedly produced a duplicate segment salt",
    ))
}

fn predict_segment_extent(
    class: u8,
    object_lengths: &[usize],
    padding: PaddingMode,
) -> Result<u64> {
    let objects = object_lengths
        .iter()
        .map(|len| vec![0; *len])
        .collect::<Vec<_>>();
    predict_segment_extent_from_fragments(class, &fragment_objects(class, &objects)?, padding)
}

fn predict_segment_extent_from_fragments(
    class: u8,
    fragments: &[FragmentRef<'_>],
    padding: PaddingMode,
) -> Result<u64> {
    let mut length = SEGMENT_HEADER_LEN as u64;
    for fragment in fragments {
        let record = wire::encode_private_fragment(
            &fragment.id,
            fragment.total,
            fragment.index,
            fragment.count,
            fragment.offset,
            fragment.bytes,
        )?;
        length = length
            .checked_add(
                PROTECTED_HEADER_LEN as u64
                    + padded_len(record.len() + 8, class, padding)? as u64
                    + 16,
            )
            .ok_or_else(|| resource_refused("segment extent overflow"))?;
    }
    let end = encode_segment_end(0, class, fragments.len() as u32, 0, 0, &[0; 32])?;
    length
        .checked_add(
            PROTECTED_HEADER_LEN as u64
                + padded_len(end.len() + 8, SEGMENT_CONTROL, padding)? as u64
                + 16,
        )
        .ok_or_else(|| resource_refused("segment extent overflow"))
}

fn segment_header(
    class: u8,
    ordinal: u64,
    salt: &[u8; 16],
    count: u32,
    extent: u64,
) -> Result<Vec<u8>> {
    if !matches!(class, SEGMENT_CONTROL | SEGMENT_PAYLOAD) || count as usize > MAX_DATA_MESSAGES {
        return Err(segment_invalid("segment class or DATA count is invalid"));
    }
    let mut bytes = b"EBSG".to_vec();
    bytes.extend_from_slice(&1_u16.to_be_bytes());
    bytes.push(class);
    bytes.push(0);
    bytes.extend_from_slice(&ordinal.to_be_bytes());
    bytes.extend_from_slice(salt);
    bytes.extend_from_slice(&count.to_be_bytes());
    bytes.extend_from_slice(&0_u32.to_be_bytes());
    bytes.extend_from_slice(&0_u64.to_be_bytes());
    bytes.extend_from_slice(&extent.to_be_bytes());
    bytes.extend_from_slice(&[0; 8]);
    Ok(bytes)
}

type ParsedSegmentHeader<'a> = (usize, u8, [u8; 16], u32, &'a [u8]);

fn parse_segment_header(input: &[u8], ordinal: u64) -> Result<ParsedSegmentHeader<'_>> {
    let header = input
        .get(..SEGMENT_HEADER_LEN)
        .ok_or_else(|| truncated("segment header is truncated"))?;
    if &header[..4] != b"EBSG"
        || u16::from_be_bytes(header[4..6].try_into().unwrap()) != 1
        || !matches!(header[6], SEGMENT_CONTROL | SEGMENT_PAYLOAD)
        || header[7] != 0
        || u64::from_be_bytes(header[8..16].try_into().unwrap()) != ordinal
        || header[36..48].iter().any(|byte| *byte != 0)
        || header[56..64].iter().any(|byte| *byte != 0)
    {
        return Err(segment_invalid(
            "SegmentHeader is noncanonical or reordered",
        ));
    }
    let count = u32::from_be_bytes(header[32..36].try_into().unwrap());
    if count as usize > MAX_DATA_MESSAGES {
        return Err(segment_invalid("too many DATA records"));
    }
    let len = usize::try_from(u64::from_be_bytes(header[48..56].try_into().unwrap()))
        .map_err(|_| resource_refused("segment extent exceeds usize"))?;
    if len < SEGMENT_HEADER_LEN + PROTECTED_HEADER_LEN + 16 {
        return Err(segment_invalid("segment extent is too small"));
    }
    Ok((
        len,
        header[6],
        header[16..32].try_into().unwrap(),
        count,
        header,
    ))
}

fn protected_header(class: u8, ordinal: u64, counter: u64, ciphertext_len: u64) -> Result<Vec<u8>> {
    if !matches!(class, RECORD_DATA | RECORD_END) || ciphertext_len < 16 {
        return Err(segment_invalid("protected record class/length is invalid"));
    }
    let mut bytes = b"EBC1".to_vec();
    bytes.extend_from_slice(&1_u16.to_be_bytes());
    bytes.push(class);
    bytes.push(0);
    bytes.extend_from_slice(&ordinal.to_be_bytes());
    bytes.extend_from_slice(&counter.to_be_bytes());
    bytes.extend_from_slice(&ciphertext_len.to_be_bytes());
    Ok(bytes)
}

fn parse_protected(
    segment: &[u8],
    cursor: usize,
    class: u8,
    ordinal: u64,
    counter: u64,
    policy: CryptoPolicy,
) -> Result<(&[u8], &[u8], usize)> {
    let header_end = cursor
        .checked_add(PROTECTED_HEADER_LEN)
        .ok_or_else(|| segment_invalid("protected header offset overflow"))?;
    let header = segment
        .get(cursor..header_end)
        .ok_or_else(|| truncated("protected record header is truncated"))?;
    if &header[..4] != b"EBC1"
        || u16::from_be_bytes(header[4..6].try_into().unwrap()) != 1
        || header[6] != class
        || header[7] != 0
        || u64::from_be_bytes(header[8..16].try_into().unwrap()) != ordinal
        || u64::from_be_bytes(header[16..24].try_into().unwrap()) != counter
    {
        return Err(segment_invalid(
            "ProtectedRecordHeader is noncanonical or reordered",
        ));
    }
    let len = u64::from_be_bytes(header[24..32].try_into().unwrap());
    if len < 16 || len > policy.max_ciphertext_record_bytes {
        return Err(resource_refused(
            "protected ciphertext length exceeds policy",
        ));
    }
    let end = header_end
        .checked_add(
            usize::try_from(len)
                .map_err(|_| resource_refused("ciphertext length exceeds usize"))?,
        )
        .ok_or_else(|| segment_invalid("ciphertext extent overflow"))?;
    let ciphertext = segment
        .get(header_end..end)
        .ok_or_else(|| truncated("protected ciphertext is truncated"))?;
    Ok((header, ciphertext, end))
}

pub(super) fn segment_key(
    keys: &KeyHierarchy,
    archive_id: &[u8; 32],
    class: u8,
    ordinal: u64,
    salt: &[u8; 16],
) -> Result<super::Secret32> {
    use hkdf::Hkdf;
    let root = if class == SEGMENT_CONTROL {
        &keys.control_segment
    } else {
        &keys.payload_segment
    };
    let (mut prk, _) = Hkdf::<Sha256>::extract(Some(salt), root);
    let suite = wire::PAYLOAD_SUITE_V1.to_be_bytes();
    let class_bytes = [class];
    let ordinal_bytes = ordinal.to_be_bytes();
    let info = wire::t1(
        "entrybound/segment-key/v1",
        &[archive_id, &suite, &class_bytes, &ordinal_bytes],
    )?;
    let hkdf = Hkdf::<Sha256>::from_prk(prk.as_ref())
        .map_err(|_| segment_invalid("segment PRK is invalid"))?;
    prk.zeroize();
    let mut output = [0; 32];
    hkdf.expand(&info, &mut output)
        .map_err(|_| segment_invalid("segment key derivation failed"))?;
    Ok(super::Secret32(output))
}

pub(super) fn record_ad(
    archive_id: &[u8; 32],
    segment: &[u8],
    protected: &[u8],
) -> Result<Vec<u8>> {
    let crypto = wire::CRYPTO_VERSION.to_be_bytes();
    let suite = wire::PAYLOAD_SUITE_V1.to_be_bytes();
    wire::t1(
        "entrybound/aead-record/v1",
        &[
            wire::FORMAT_NAMESPACE,
            &crypto,
            &suite,
            archive_id,
            segment,
            protected,
        ],
    )
}

pub(super) fn data_nonce(counter: u64) -> [u8; 12] {
    let mut nonce = [0; 12];
    nonce[4..].copy_from_slice(&counter.to_be_bytes());
    nonce
}

pub(super) fn end_nonce(count: u64) -> [u8; 12] {
    let mut nonce = [0; 12];
    nonce[..4].fill(0xff);
    nonce[4..].copy_from_slice(&count.to_be_bytes());
    nonce
}

struct RecordDecryptContext<'a> {
    archive_id: &'a [u8; 32],
    segment_header: &'a [u8],
    padding: PaddingMode,
    segment_class: u8,
}

fn decrypt_record(
    key: &super::Secret32,
    context: &RecordDecryptContext<'_>,
    protected: &[u8],
    ciphertext: &[u8],
    end: bool,
) -> Result<Vec<u8>> {
    let counter = u64::from_be_bytes(protected[16..24].try_into().unwrap());
    let nonce = if end {
        end_nonce(counter)
    } else {
        data_nonce(counter)
    };
    let plaintext = aead_open(
        &key.0,
        &nonce,
        &record_ad(context.archive_id, context.segment_header, protected)?,
        ciphertext,
    )?;
    if plaintext.len() < 8 {
        return Err(crypto_nonconforming(
            ReasonCode::CryptoPaddingInvalid,
            "authenticated private record is too short",
        ));
    }
    let private_len = usize::try_from(u64::from_be_bytes(plaintext[..8].try_into().unwrap()))
        .map_err(|_| resource_refused("private length exceeds usize"))?;
    let private_end = 8usize
        .checked_add(private_len)
        .ok_or_else(|| resource_refused("private length overflow"))?;
    let private = plaintext.get(8..private_end).ok_or_else(|| {
        crypto_nonconforming(
            ReasonCode::CryptoPaddingInvalid,
            "private length exceeds authenticated bytes",
        )
    })?;
    let class = if end {
        SEGMENT_CONTROL
    } else {
        context.segment_class
    };
    if plaintext.len() != padded_len(8 + private_len, class, context.padding)? {
        return Err(crypto_nonconforming(
            ReasonCode::CryptoPaddingInvalid,
            "authenticated padding does not match its declared mode",
        ));
    }
    Ok(private.to_vec())
}

fn pad_private(private: &[u8], class: u8, mode: PaddingMode) -> Result<Vec<u8>> {
    let unpadded = 8usize
        .checked_add(private.len())
        .ok_or_else(|| resource_refused("private record length overflow"))?;
    let target = padded_len(unpadded, class, mode)?;
    let mut bytes = Vec::with_capacity(target);
    bytes.extend_from_slice(&(private.len() as u64).to_be_bytes());
    bytes.extend_from_slice(private);
    if bytes.len() < target {
        let mut padding = vec![0; target - bytes.len()];
        getrandom::fill(&mut padding)
            .map_err(|_| resource_refused("OS CSPRNG failed while padding"))?;
        bytes.extend_from_slice(&padding);
    }
    Ok(bytes)
}

fn padded_len(unpadded: usize, class: u8, mode: PaddingMode) -> Result<usize> {
    let maximum = if class == SEGMENT_CONTROL {
        MAX_CONTROL_PRIVATE
    } else {
        MAX_PAYLOAD_PRIVATE
    };
    if unpadded > maximum {
        return Err(resource_refused("private record exceeds class capacity"));
    }
    match mode {
        PaddingMode::None => Ok(unpadded),
        PaddingMode::Maximum => Ok(maximum),
        PaddingMode::Bucketed => bucket_lengths(class)
            .into_iter()
            .find(|bucket| *bucket >= unpadded)
            .ok_or_else(|| resource_refused("private record has no permitted bucket")),
    }
}

fn bucket_lengths(class: u8) -> Vec<usize> {
    let (first, last): (u32, u32) = if class == SEGMENT_CONTROL {
        (8, 20)
    } else {
        (12, 26)
    };
    let mut values = BTreeSet::new();
    for power in first..=last {
        values.insert(1usize << power);
        values.insert(5 * (1usize << (power - 2)));
        values.insert(3 * (1usize << (power - 1)));
        values.insert(7 * (1usize << (power - 2)));
    }
    let minimum = 1usize << first;
    let maximum = 1usize << last;
    values
        .into_iter()
        .filter(|value| *value >= minimum && *value <= maximum)
        .collect()
}

fn data_sequence(count: u32, exact_records: &[u8]) -> Vec<u8> {
    let mut output = u64::from(count).to_be_bytes().to_vec();
    let mut cursor = 0usize;
    while cursor < exact_records.len() {
        let cipher_len =
            u64::from_be_bytes(exact_records[cursor + 24..cursor + 32].try_into().unwrap())
                as usize;
        let extent = PROTECTED_HEADER_LEN + cipher_len;
        output.extend_from_slice(&(extent as u64).to_be_bytes());
        output.extend_from_slice(&exact_records[cursor..cursor + extent]);
        cursor += extent;
    }
    output
}

fn encode_segment_end(
    ordinal: u64,
    class: u8,
    count: u32,
    private_bytes: u64,
    ciphertext_bytes: u64,
    digest: &[u8; 32],
) -> Result<Vec<u8>> {
    let mut record = RecordBuilder::new(wire::RECORD_SEGMENT_END);
    record
        .u64(1, ordinal)?
        .u8(2, class)?
        .u32(3, count)?
        .u64(4, private_bytes)?
        .u64(5, ciphertext_bytes)?
        .bytes(6, digest)?;
    record.finish()
}

fn validate_segment_end(
    input: &[u8],
    ordinal: u64,
    class: u8,
    count: u32,
    private_bytes: u64,
    ciphertext_bytes: u64,
    digest: &[u8; 32],
) -> Result<()> {
    let (record, consumed) = decode_record(input)?;
    if consumed != input.len() || record.kind != wire::RECORD_SEGMENT_END {
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
        return Err(segment_invalid(
            "SegmentEnd authenticated totals/digest mismatch",
        ));
    }
    Ok(())
}

fn encode_archive_final(value: &ArchiveFinal) -> Result<Vec<u8>> {
    let mut record = RecordBuilder::new(wire::RECORD_ARCHIVE_FINAL);
    record
        .u64(1, value.segment_count)?
        .u64(2, value.entry_count)?
        .u64(3, value.total_logical)?
        .u64(4, value.chunk_count)?
        .bytes(5, &value.lai)?
        .bytes(6, &value.pcr)?
        .bytes(7, &value.aux)?
        .bytes(8, &value.recipient_set)?
        .bytes(9, &value.segment_sequence)?
        .bytes(10, &value.footer_core)?
        .bytes(11, &value.descriptor_id)?
        .bytes(12, &value.manifest_id)?;
    record.finish()
}

fn decode_archive_final(input: &[u8]) -> Result<ArchiveFinal> {
    let (record, consumed) = decode_record(input)?;
    if consumed != input.len() || record.kind != wire::RECORD_ARCHIVE_FINAL {
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
        segment_sequence: exact(record.field(9)?.as_bytes()?)?,
        footer_core: exact(record.field(10)?.as_bytes()?)?,
        descriptor_id: exact(record.field(11)?.as_bytes()?)?,
        manifest_id: exact(record.field(12)?.as_bytes()?)?,
    })
}

fn segment_sequence_digest(digests: &[[u8; 32]]) -> Result<[u8; 32]> {
    let mut sequence = Vec::new();
    for digest in digests {
        sequence.extend_from_slice(&32_u64.to_be_bytes());
        sequence.extend_from_slice(digest);
    }
    Ok(Sha256::digest(wire::t1(
        "entrybound/segment-sequence/v1",
        &[&(digests.len() as u64).to_be_bytes(), &sequence],
    )?)
    .into())
}

fn encrypted_preamble(features: u64) -> Result<Vec<u8>> {
    let mut bytes = vec![0; PREAMBLE_LEN as usize];
    bytes[..8].copy_from_slice(&MAGIC);
    bytes[8..10].copy_from_slice(&0_u16.to_be_bytes());
    bytes[10..12].copy_from_slice(&1_u16.to_be_bytes());
    bytes[12..16].copy_from_slice(&(PREAMBLE_LEN as u32).to_be_bytes());
    bytes[16..24].copy_from_slice(&features.to_be_bytes());
    let feature_digest = sha256_exact(&bytes[16..40]);
    bytes[40..72].copy_from_slice(feature_digest.as_bytes());
    bytes[72] = 1;
    bytes[73] = 1;
    bytes[74] = 0;
    Ok(bytes)
}

fn section(kind: u16, payload: &[u8]) -> Result<Vec<u8>> {
    let mut bytes = SECTION_MAGIC.to_vec();
    bytes.extend_from_slice(&kind.to_be_bytes());
    bytes.extend_from_slice(&1_u16.to_be_bytes());
    bytes.extend_from_slice(&[0; 8]);
    bytes.extend_from_slice(&(payload.len() as u64).to_be_bytes());
    bytes.extend_from_slice(sha256_exact(payload).as_bytes());
    bytes.extend_from_slice(&[0; 8]);
    bytes.extend_from_slice(payload);
    Ok(bytes)
}

fn parse_section(bytes: &[u8], offset: u64, kind: u16) -> Result<(&[u8], u64)> {
    let start =
        usize::try_from(offset).map_err(|_| resource_refused("section offset exceeds usize"))?;
    let header_end = start
        .checked_add(SECTION_HEADER_LEN as usize)
        .ok_or_else(|| resource_refused("section header offset overflow"))?;
    let header = bytes
        .get(start..header_end)
        .ok_or_else(|| truncated("encrypted section header is truncated"))?;
    if &header[..4] != SECTION_MAGIC
        || u16::from_be_bytes(header[4..6].try_into().unwrap()) != kind
        || u16::from_be_bytes(header[6..8].try_into().unwrap()) != 1
        || header[8..16].iter().any(|byte| *byte != 0)
        || header[56..64].iter().any(|byte| *byte != 0)
    {
        return Err(Diagnostic::new(
            OutcomeClass::Nonconforming,
            ReasonCode::SectionStructure,
            "encrypted section header is noncanonical",
        ));
    }
    let payload_len = usize::try_from(u64::from_be_bytes(header[16..24].try_into().unwrap()))
        .map_err(|_| resource_refused("section payload exceeds usize"))?;
    let end = header_end
        .checked_add(payload_len)
        .ok_or_else(|| resource_refused("section extent overflow"))?;
    let payload = bytes
        .get(header_end..end)
        .ok_or_else(|| truncated("encrypted section payload is truncated"))?;
    if sha256_exact(payload).as_bytes() != &header[24..56] {
        return Err(crypto_integrity(
            ReasonCode::SectionDigestMismatch,
            "encrypted section digest mismatch",
        ));
    }
    Ok((payload, (SECTION_HEADER_LEN as usize + payload_len) as u64))
}

#[allow(clippy::too_many_arguments)]
fn footer_core(
    total: u64,
    envelope_offset: u64,
    envelope_len: u64,
    segments_offset: u64,
    segments_len: u64,
    terminal_offset: u64,
    final_offset: u64,
    preamble_digest: &[u8; 32],
    context_digest: &[u8; 32],
) -> Result<Vec<u8>> {
    wire::t1(
        "entrybound/encrypted-footer-core/v1",
        &[
            &2_u16.to_be_bytes(),
            &total.to_be_bytes(),
            &envelope_offset.to_be_bytes(),
            &envelope_len.to_be_bytes(),
            &segments_offset.to_be_bytes(),
            &segments_len.to_be_bytes(),
            &terminal_offset.to_be_bytes(),
            &final_offset.to_be_bytes(),
            preamble_digest,
            context_digest,
        ],
    )
}

#[allow(clippy::too_many_arguments)]
fn encrypted_footer(
    total: u64,
    envelope_offset: u64,
    envelope_len: u64,
    segments_offset: u64,
    segments_len: u64,
    terminal_offset: u64,
    final_offset: u64,
    preamble: &[u8],
    segments: &[u8],
    public_context: &[u8],
) -> Vec<u8> {
    let mut footer = Vec::with_capacity(ENCRYPTED_FOOTER_LEN);
    footer.extend_from_slice(&FOOTER_MAGIC);
    footer.extend_from_slice(&2_u16.to_be_bytes());
    footer.extend_from_slice(&(ENCRYPTED_FOOTER_LEN as u16).to_be_bytes());
    footer.extend_from_slice(&0_u32.to_be_bytes());
    for value in [
        total,
        envelope_offset,
        envelope_len,
        segments_offset,
        segments_len,
        terminal_offset,
        final_offset,
    ] {
        footer.extend_from_slice(&value.to_be_bytes());
    }
    footer.extend_from_slice(sha256_exact(preamble).as_bytes());
    footer.extend_from_slice(sha256_exact(segments).as_bytes());
    footer.extend_from_slice(&Sha256::digest(public_context));
    footer.extend_from_slice(&[0; 24]);
    footer
}

fn parse_footer(bytes: &[u8]) -> Result<ParsedFooter> {
    let start = bytes
        .len()
        .checked_sub(ENCRYPTED_FOOTER_LEN)
        .ok_or_else(|| truncated("encrypted footer is truncated"))?;
    let footer = &bytes[start..];
    if footer[..8] != FOOTER_MAGIC {
        return Err(Diagnostic::new(
            OutcomeClass::Truncated,
            ReasonCode::TruncatedFooter,
            "encrypted footer magic is missing",
        ));
    }
    if u16::from_be_bytes(footer[8..10].try_into().unwrap()) != 2
        || u16::from_be_bytes(footer[10..12].try_into().unwrap()) != ENCRYPTED_FOOTER_LEN as u16
        || footer[12..16].iter().any(|byte| *byte != 0)
        || footer[168..].iter().any(|byte| *byte != 0)
    {
        return Err(Diagnostic::new(
            OutcomeClass::Nonconforming,
            ReasonCode::NoncanonicalEncoding,
            "encrypted footer is noncanonical",
        ));
    }
    Ok(ParsedFooter {
        total_len: be64(footer, 16),
        envelope_offset: be64(footer, 24),
        envelope_len: be64(footer, 32),
        segments_offset: be64(footer, 40),
        segments_len: be64(footer, 48),
        terminal_offset: be64(footer, 56),
        archive_final_offset: be64(footer, 64),
        preamble_digest: footer[72..104].try_into().unwrap(),
        segments_digest: footer[104..136].try_into().unwrap(),
        public_context_digest: footer[136..168].try_into().unwrap(),
    })
}

fn validate_envelope_policy(envelope: &wire::CryptoEnvelope, features: u64) -> Result<()> {
    match envelope.protection_policy {
        1 if !envelope.stanzas.is_empty()
            && envelope
                .stanzas
                .iter()
                .all(|value| value.stanza_type != 2 && value.protection_class == 1)
            && features & super::FEATURE_XWING_RECIPIENT != 0
            && features & super::FEATURE_PASSWORD_RECIPIENT == 0 => {}
        2 if envelope.stanzas.len() == 1
            && envelope.stanzas[0].stanza_type == 2
            && envelope.stanzas[0].protection_class == 2
            && features & super::FEATURE_PASSWORD_RECIPIENT != 0
            && features & super::FEATURE_XWING_RECIPIENT == 0 => {}
        _ => {
            return Err(Diagnostic::new(
                OutcomeClass::Nonconforming,
                ReasonCode::CryptoRecipientPolicyInvalid,
                "recipient policy, stanzas, and feature bits disagree",
            ));
        }
    }
    for stanza in &envelope.stanzas {
        match stanza.stanza_type {
            1 if stanza.method_parameters == wire::XWING_METHOD
                && stanza.encapsulation.len() == 1120
                && stanza.protection_class == 1 => {}
            2 if stanza.method_parameters.len() == wire::PASSWORD_METHOD_LEN
                && stanza.encapsulation.is_empty()
                && stanza.protection_class == 2 => {}
            1 | 2 => {
                return Err(stanza_invalid(
                    "known recipient stanza has invalid parameters",
                ));
            }
            _ => {}
        }
    }
    Ok(())
}

fn public_inspection(
    envelope: &wire::CryptoEnvelope,
    total: u64,
    segments: Option<u64>,
) -> Result<PublicCryptoInspection> {
    Ok(PublicCryptoInspection {
        encrypted: true,
        payload_suite: "entrybound-payload-suite-v1",
        recipient_count: envelope.stanzas.len() as u64,
        recipient_types: envelope
            .stanzas
            .iter()
            .map(|value| match value.stanza_type {
                1 => "xwing-draft-10",
                2 => "password-argon2id",
                _ => "unknown",
            })
            .collect(),
        padding: PaddingMode::try_from(envelope.padding_mode)?,
        boundary: BoundaryMode::try_from(envelope.boundary_mode)?,
        segment_count: segments,
        total_container_bytes: total,
    })
}

fn recipient_set_digest(stanzas: &[wire::RecipientStanza]) -> Result<[u8; 32]> {
    let sequence = wire::encode_stanza_sequence(stanzas)?;
    Ok(Sha256::digest(wire::t1("entrybound/recipient-set/v1", &[&sequence])?).into())
}

fn protection_policy(value: u8) -> Result<ProtectionPolicy> {
    match value {
        1 => Ok(ProtectionPolicy::HybridOnly),
        2 => Ok(ProtectionPolicy::PasswordOnly),
        _ => Err(Diagnostic::new(
            OutcomeClass::Nonconforming,
            ReasonCode::CryptoRecipientPolicyInvalid,
            "unknown recipient protection policy",
        )),
    }
}

fn encode_recipient_directory(
    id: &[u8; 16],
    fingerprint: &[u8; 32],
    label: &str,
) -> Result<Vec<u8>> {
    let mut record = RecordBuilder::new(wire::RECORD_RECIPIENT_DIRECTORY);
    record
        .bytes(1, id)?
        .u16(2, 1)?
        .bytes(3, fingerprint)?
        .utf8(4, label)?;
    record.finish()
}

fn encode_recipient_directory_entry(value: &RecipientDirectoryEntry) -> Result<Vec<u8>> {
    if value.stanza_type != 1 {
        return Err(wire::private_invalid(
            "recipient directory contains an unsupported stanza type",
        ));
    }
    encode_recipient_directory(&value.stanza_id, &value.fingerprint, &value.label)
}

fn decode_descriptor_roots(bytes: &[u8]) -> Result<([u8; 32], [u8; 32], [u8; 32])> {
    let (record, consumed) = decode_record(bytes)?;
    if consumed != bytes.len() || record.kind != 1 {
        return Err(wire::private_invalid("encrypted Descriptor is not type 1"));
    }
    match record.version {
        1 => record.expect_versioned_tags(1, &[1, 2, 3, 4, 5, 6, 7, 8], &[])?,
        2 => record.expect_versioned_tags(
            2,
            &[
                1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19,
            ],
            &[],
        )?,
        _ => {
            return Err(wire::private_invalid(
                "unsupported encrypted Descriptor version",
            ));
        }
    }
    Ok((
        exact(record.field(6)?.as_bytes()?)?,
        exact(record.field(7)?.as_bytes()?)?,
        exact(record.field(8)?.as_bytes()?)?,
    ))
}

fn encrypted_planner_id(id: &str) -> &str {
    if id.starts_with("fast-") {
        "fast-enc-v1"
    } else if id.starts_with("balanced-") {
        "balanced-enc-v1"
    } else if id.starts_with("dense-") {
        "dense-enc-v1"
    } else if id.starts_with("extreme-") {
        "extreme-enc-v1"
    } else {
        id
    }
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    left.len() == right.len() && bool::from(left.ct_eq(right))
}

fn be64(bytes: &[u8], offset: usize) -> u64 {
    u64::from_be_bytes(bytes[offset..offset + 8].try_into().unwrap())
}

fn exact<const N: usize>(bytes: &[u8]) -> Result<[u8; N]> {
    bytes
        .try_into()
        .map_err(|_| wire::private_invalid(format!("expected {N} bytes")))
}

fn segment_invalid(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::Nonconforming,
        ReasonCode::CryptoSegmentStructureInvalid,
        detail,
    )
}

fn crypto_nonconforming(code: ReasonCode, detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(OutcomeClass::Nonconforming, code, detail)
}

fn truncated(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(OutcomeClass::Truncated, ReasonCode::TruncatedStream, detail)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    use super::*;
    use crate::archive::{PackOptions, plan_directory};
    use crate::crypto::{SigningKey, current_bindings, sign_archive};

    struct Fixture {
        root: PathBuf,
        source: PathBuf,
    }

    impl Fixture {
        fn new(name: &str) -> Self {
            let root = std::env::temp_dir().join(format!(
                "entrybound-descriptor-v2-{}-{name}",
                std::process::id()
            ));
            let _ = std::fs::remove_dir_all(&root);
            let source = root.join("source");
            std::fs::create_dir_all(&source).unwrap();
            std::fs::write(source.join("data.bin"), vec![b'A'; 2 * 1024 * 1024 + 31]).unwrap();
            Self { root, source }
        }

        fn archive(&self) -> Archive {
            plan_directory(&self.source, PackOptions::default()).unwrap()
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    fn encrypted_from_parts(
        parts: EncryptedPlainParts,
        recipient: &XWingRecipient,
    ) -> EncryptedArchive {
        encrypt_with_file_key(
            parts,
            &[0x41; 32],
            [0x52; 32],
            EncryptedWriteOptions {
                recipients: std::slice::from_ref(recipient),
                padding: PaddingMode::None,
                ..EncryptedWriteOptions::default()
            },
        )
        .unwrap()
    }

    fn open_identity(
        bytes: &[u8],
        identity: &super::super::XWingIdentity,
    ) -> Result<OpenedArchive> {
        open_encrypted(
            bytes,
            EncryptedOpenOptions::new(Some(Unlock::Identity(identity))),
        )
    }

    fn descriptor_value_offset(bytes: &[u8], wanted: u16) -> usize {
        let mut cursor = crate::canonical::RECORD_HEADER_LEN;
        while cursor < bytes.len() {
            let tag = u16::from_be_bytes(bytes[cursor..cursor + 2].try_into().unwrap());
            let len = u64::from_be_bytes(bytes[cursor + 4..cursor + 12].try_into().unwrap());
            if tag == wanted {
                return cursor + 12;
            }
            cursor += 12 + usize::try_from(len).unwrap();
        }
        panic!("Descriptor field {wanted} is absent");
    }

    #[test]
    fn padding_and_nonce_rules_are_frozen() {
        assert_eq!(
            padded_len(256, SEGMENT_CONTROL, PaddingMode::Bucketed).unwrap(),
            256
        );
        assert_eq!(
            padded_len(257, SEGMENT_CONTROL, PaddingMode::Bucketed).unwrap(),
            320
        );
        assert_eq!(
            padded_len(1, SEGMENT_PAYLOAD, PaddingMode::Bucketed).unwrap(),
            4096
        );
        assert_eq!(&data_nonce(7)[..4], &[0; 4]);
        assert_eq!(&end_nonce(7)[..4], &[0xff; 4]);
    }

    #[test]
    fn corrected_writer_emits_v2_feature_and_legacy_v1_remains_readable() {
        let fixture = Fixture::new("compatibility");
        let archive = fixture.archive();
        let (identity, recipient) = super::super::XWingIdentity::generate().unwrap();

        let corrected =
            encrypted_from_parts(prepare_encrypted_plain_parts(&archive).unwrap(), &recipient);
        let corrected_features = u64::from_be_bytes(corrected.bytes[16..24].try_into().unwrap());
        assert_ne!(
            corrected_features & super::super::FEATURE_PRIVATE_RESOURCE_DECLARATION_V1,
            0
        );
        let corrected_open = open_identity(&corrected.bytes, &identity).unwrap();
        assert!(corrected_open.archive.descriptor.budget_declared);
        assert_eq!(
            corrected_open.archive.descriptor.budget,
            corrected.archive.descriptor.budget
        );
        assert_eq!(
            corrected_open.archive.descriptor.decode,
            corrected.archive.descriptor.decode
        );
        let corrected_inspection = inspect_encrypted(
            &corrected.bytes,
            Some(Unlock::Identity(&identity)),
            CryptoPolicy::default(),
        )
        .unwrap()
        .authenticated_descriptor
        .unwrap();
        assert_eq!(corrected_inspection.record_version, 2);
        assert!(corrected_inspection.producer_declaration_present);
        assert!(corrected_inspection.independently_validated);

        let mut resource_refusal = EncryptedOpenOptions::new(Some(Unlock::Identity(&identity)));
        resource_refusal.resource_policy.entry_count = 0;
        let error = open_encrypted(&corrected.bytes, resource_refusal).unwrap_err();
        assert_eq!(error.class(), OutcomeClass::PolicyRefused);
        assert_eq!(error.code(), ReasonCode::ResourceLimit);

        let mut decode_refusal = EncryptedOpenOptions::new(Some(Unlock::Identity(&identity)));
        decode_refusal.decode_policy.window_bytes = 0;
        let error = open_encrypted(&corrected.bytes, decode_refusal).unwrap_err();
        assert_eq!(error.class(), OutcomeClass::PolicyRefused);
        assert_eq!(error.code(), ReasonCode::ResourceLimit);

        let legacy = encrypted_from_parts(
            crate::ecf::prepare_legacy_encrypted_plain_parts(&archive).unwrap(),
            &recipient,
        );
        let legacy_features = u64::from_be_bytes(legacy.bytes[16..24].try_into().unwrap());
        assert_eq!(
            legacy_features & super::super::FEATURE_PRIVATE_RESOURCE_DECLARATION_V1,
            0
        );
        let legacy_open = open_identity(&legacy.bytes, &identity).unwrap();
        assert!(!legacy_open.archive.descriptor.budget_declared);
        let legacy_inspection = inspect_encrypted(
            &legacy.bytes,
            Some(Unlock::Identity(&identity)),
            CryptoPolicy::default(),
        )
        .unwrap()
        .authenticated_descriptor
        .unwrap();
        assert_eq!(legacy_inspection.record_version, 1);
        assert!(!legacy_inspection.producer_declaration_present);
        assert!(legacy_inspection.declared_budget.is_none());
        assert!(legacy_inspection.declared_decode.is_none());
    }

    #[test]
    fn descriptor_feature_mismatch_duplicate_and_missing_fail_closed() {
        let fixture = Fixture::new("dispatch-negative");
        let archive = fixture.archive();
        let policy = crate::archive::bootstrap_resource_policy();
        let decode_policy = crate::archive::bootstrap_decode_policy();
        let v1 = crate::ecf::prepare_legacy_encrypted_plain_parts(&archive).unwrap();
        let v2 = prepare_encrypted_plain_parts(&archive).unwrap();
        let v1_object = wire::private_object(wire::PRIVATE_OBJECT_RECORD, &v1.descriptor).unwrap();
        let v2_object = wire::private_object(wire::PRIVATE_OBJECT_RECORD, &v2.descriptor).unwrap();

        let error = ObjectCollector::default()
            .dispatch(
                v1_object,
                super::super::FEATURE_PRIVATE_RESOURCE_DECLARATION_V1,
                policy,
                decode_policy,
            )
            .err()
            .expect("feature/version mismatch must fail");
        assert_eq!(error.code(), ReasonCode::CryptoPrivateObjectInvalid);

        let error = ObjectCollector::default()
            .dispatch(v2_object.clone(), 0, policy, decode_policy)
            .err()
            .expect("feature/version mismatch must fail");
        assert_eq!(error.code(), ReasonCode::CryptoPrivateObjectInvalid);

        let mut duplicate = ObjectCollector::default();
        duplicate
            .dispatch(
                v2_object.clone(),
                super::super::FEATURE_PRIVATE_RESOURCE_DECLARATION_V1,
                policy,
                decode_policy,
            )
            .unwrap();
        let error = duplicate
            .dispatch(
                v2_object,
                super::super::FEATURE_PRIVATE_RESOURCE_DECLARATION_V1,
                policy,
                decode_policy,
            )
            .err()
            .expect("duplicate Descriptor must fail");
        assert_eq!(error.code(), ReasonCode::CryptoPrivateObjectInvalid);

        let plans =
            collection_object(wire::COLLECTION_TRANSFORM_PLANS, &v2.transform_plans).unwrap();
        let error = ObjectCollector::default()
            .dispatch(
                plans,
                super::super::FEATURE_PRIVATE_RESOURCE_DECLARATION_V1,
                policy,
                decode_policy,
            )
            .err()
            .expect("missing Descriptor must fail");
        assert_eq!(error.code(), ReasonCode::CryptoPrivateObjectInvalid);
    }

    #[test]
    fn authenticated_underdeclared_budget_and_decode_are_rejected() {
        let fixture = Fixture::new("underdeclared");
        let archive = fixture.archive();
        let (identity, recipient) = super::super::XWingIdentity::generate().unwrap();

        for tag in [12_u16, 13, 16] {
            let mut parts = prepare_encrypted_plain_parts(&archive).unwrap();
            let value = descriptor_value_offset(&parts.descriptor, tag);
            parts.descriptor[value..value + 8].copy_from_slice(&0_u64.to_be_bytes());
            let encrypted = encrypted_from_parts(parts, &recipient);
            assert_eq!(
                open_identity(&encrypted.bytes, &identity)
                    .unwrap_err()
                    .code(),
                ReasonCode::ResourceLimit,
                "underdeclared Descriptor tag {tag} must fail"
            );
        }

        let mut parts = prepare_encrypted_plain_parts(&archive).unwrap();
        let declaration = private_descriptor_declaration(&parts.descriptor).unwrap();
        assert!(declaration.decode.unwrap().window_bytes > 0);
        let window = descriptor_value_offset(&parts.descriptor, 9);
        parts.descriptor[window..window + 8].copy_from_slice(&0_u64.to_be_bytes());
        let encrypted = encrypted_from_parts(parts, &recipient);
        assert_eq!(
            open_identity(&encrypted.bytes, &identity)
                .unwrap_err()
                .code(),
            ReasonCode::ResourceLimit
        );
    }

    #[test]
    fn recipient_addition_reuses_bulk_payload_ciphertext() {
        let fixture = Fixture::new("recipient-add-payload-preservation");
        let archive = fixture.archive();
        let (first_identity, first_recipient) = super::super::XWingIdentity::generate().unwrap();
        let (second_identity, second_recipient) = super::super::XWingIdentity::generate().unwrap();
        let initial = encrypted_from_parts(
            prepare_encrypted_plain_parts(&archive).unwrap(),
            &first_recipient,
        );
        let before = open_for_mutation(
            &initial.bytes,
            EncryptedOpenOptions::new(Some(Unlock::Identity(&first_identity))),
        )
        .unwrap();
        let authenticated = open_encrypted_authenticated(
            &initial.bytes,
            EncryptedOpenOptions::new(Some(Unlock::Identity(&first_identity))),
        )
        .unwrap();
        let bindings =
            current_bindings(&authenticated.opened, Some(authenticated.addressing)).unwrap();
        let signature =
            sign_archive(&bindings, &SigningKey::from_seed([0x71; 32]), true, true).unwrap();
        let signed = embed_signature(
            &initial.bytes,
            EncryptedOpenOptions::new(Some(Unlock::Identity(&first_identity))),
            signature,
        )
        .unwrap();
        let after_embed = open_for_mutation(
            &signed.bytes,
            EncryptedOpenOptions::new(Some(Unlock::Identity(&first_identity))),
        )
        .unwrap();
        assert_reused_payload(&before.payload_segments, &after_embed.payload_segments);

        let added = add_recipient(
            &signed.bytes,
            EncryptedOpenOptions::new(Some(Unlock::Identity(&first_identity))),
            &second_recipient,
        )
        .unwrap();
        let after = open_for_mutation(
            &added.bytes,
            EncryptedOpenOptions::new(Some(Unlock::Identity(&second_identity))),
        )
        .unwrap();
        assert_reused_payload(&after_embed.payload_segments, &after.payload_segments);
    }

    fn assert_reused_payload(
        expected: &BTreeMap<u64, ReusableSegment>,
        actual: &BTreeMap<u64, ReusableSegment>,
    ) {
        assert_eq!(expected.len(), actual.len());
        for (ordinal, expected) in expected {
            let actual = actual.get(ordinal).unwrap();
            assert_eq!(actual.class, expected.class);
            assert_eq!(actual.bytes, expected.bytes);
            assert_eq!(actual.digest, expected.digest);
        }
    }
}
