//! Entrybound crypto-v1 primitives, recipient envelope, and encrypted INDEXED API.
//!
//! This module implements only the frozen suite described by the repository's
//! crypto-v1 documents. Algorithm choice is not archive- or caller-negotiable.

mod container;
mod wire;

use std::path::Path;

use aes_gcm_siv::{
    Aes256GcmSiv, Nonce,
    aead::{Aead, KeyInit, Payload},
};
use argon2::{Algorithm, Argon2, Params, Version};
use hkdf::Hkdf;
use hmac::{Hmac, KeyInit as HmacKeyInit, Mac};
use sha2::{Digest as _, Sha256};
use x_wing::{
    DecapsulationKey, EncapsulationKey, XWingKem,
    kem::{Decapsulator as _, Kem as _, KeyExport as _},
};
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

use crate::diagnostics::{Diagnostic, OutcomeClass, ReasonCode, Result};
use crate::{archive::PackOptions, chunker::EncryptedBoundaryKey};

pub use container::{
    CryptoInspection, EncryptedArchive, EncryptedOpenOptions, EncryptedWriteOptions,
    PublicCryptoInspection, encrypt_archive, inspect_encrypted, open_encrypted, verify_encrypted,
};

pub const FEATURE_ENCRYPTED_INDEXED_V1: u64 = 0x20;
pub const FEATURE_PAYLOAD_SUITE_V1: u64 = 0x40;
pub const FEATURE_XWING_RECIPIENT: u64 = 0x80;
pub const FEATURE_PASSWORD_RECIPIENT: u64 = 0x100;
pub const FEATURE_PADDING: u64 = 0x400;
pub const FEATURE_STRONG_BOUNDARY: u64 = 0x800;
pub const CRYPTO_FEATURES: u64 = FEATURE_ENCRYPTED_INDEXED_V1
    | FEATURE_PAYLOAD_SUITE_V1
    | FEATURE_XWING_RECIPIENT
    | FEATURE_PASSWORD_RECIPIENT
    | FEATURE_PADDING
    | FEATURE_STRONG_BOUNDARY;

const XWING_PUBLIC_KEY_LEN: usize = 1_216;
const XWING_SECRET_KEY_LEN: usize = 32;
const LOCAL_KEY_HEADER_LEN: usize = 12;
const LOCAL_KEY_MAGIC: &[u8; 4] = b"EBK1";

type HmacSha256 = Hmac<Sha256>;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(u8)]
pub enum PaddingMode {
    None = 0,
    #[default]
    Bucketed = 1,
    Maximum = 2,
}

impl TryFrom<u8> for PaddingMode {
    type Error = Diagnostic;

    fn try_from(value: u8) -> Result<Self> {
        match value {
            0 => Ok(Self::None),
            1 => Ok(Self::Bucketed),
            2 => Ok(Self::Maximum),
            _ => Err(Diagnostic::new(
                OutcomeClass::Nonconforming,
                ReasonCode::CryptoPaddingInvalid,
                "unknown encrypted padding mode",
            )),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(u8)]
pub enum BoundaryMode {
    #[default]
    SecretGearTable = 1,
    PhteAes128 = 2,
}

impl TryFrom<u8> for BoundaryMode {
    type Error = Diagnostic;

    fn try_from(value: u8) -> Result<Self> {
        match value {
            1 => Ok(Self::SecretGearTable),
            2 => Ok(Self::PhteAes128),
            _ => Err(Diagnostic::new(
                OutcomeClass::Unsupported,
                ReasonCode::CryptoBoundaryModeUnsupported,
                "unknown encrypted chunk-boundary mode",
            )),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
enum ProtectionPolicy {
    HybridOnly = 1,
    PasswordOnly = 2,
}

/// Caller-owned upper bounds for attacker-controlled crypto work.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CryptoPolicy {
    pub max_stanzas: u32,
    pub max_stanza_bytes: u64,
    pub max_envelope_bytes: u64,
    pub max_identity_attempts: u32,
    pub max_argon2_memory_kib: u32,
    pub max_argon2_passes: u32,
    pub max_argon2_parallelism: u32,
    pub max_segments: u64,
    pub max_messages_per_segment: u32,
    pub max_ciphertext_record_bytes: u64,
    pub max_private_record_bytes: u64,
    pub max_working_memory_bytes: u64,
}

impl Default for CryptoPolicy {
    fn default() -> Self {
        Self {
            max_stanzas: 1_024,
            max_stanza_bytes: 65_536,
            max_envelope_bytes: 16 << 20,
            max_identity_attempts: 4_096,
            max_argon2_memory_kib: 1_048_576,
            max_argon2_passes: 10,
            max_argon2_parallelism: 16,
            max_segments: 1_000_000,
            max_messages_per_segment: (1 << 20) - 1,
            max_ciphertext_record_bytes: (64 << 20) + 16,
            max_private_record_bytes: 64 << 20,
            max_working_memory_bytes: 1 << 30,
        }
    }
}

/// Public X-Wing draft-10 recipient material.
#[derive(Clone, Eq, PartialEq)]
pub struct XWingRecipient {
    bytes: [u8; XWING_PUBLIC_KEY_LEN],
    label: String,
}

impl std::fmt::Debug for XWingRecipient {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("XWingRecipient")
            .field("fingerprint", &hex(&self.fingerprint()))
            .field("label", &self.label)
            .finish()
    }
}

impl XWingRecipient {
    pub fn from_bytes(bytes: &[u8], label: impl Into<String>) -> Result<Self> {
        let bytes: [u8; XWING_PUBLIC_KEY_LEN] = bytes.try_into().map_err(|_| {
            Diagnostic::new(
                OutcomeClass::Nonconforming,
                ReasonCode::CryptoRecipientStanzaInvalid,
                "X-Wing recipient key must contain exactly 1216 bytes",
            )
        })?;
        EncapsulationKey::try_from(bytes.as_slice()).map_err(|_| {
            Diagnostic::new(
                OutcomeClass::Nonconforming,
                ReasonCode::CryptoRecipientStanzaInvalid,
                "X-Wing recipient key is malformed",
            )
        })?;
        let label = label.into();
        if label.len() > 1_024 {
            return Err(resource_refused("recipient label exceeds 1024 bytes"));
        }
        Ok(Self { bytes, label })
    }

    #[must_use]
    pub fn bytes(&self) -> &[u8; XWING_PUBLIC_KEY_LEN] {
        &self.bytes
    }

    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }

    #[must_use]
    pub fn fingerprint(&self) -> [u8; 32] {
        recipient_fingerprint(&self.bytes).expect("fixed recipient transcript")
    }

    pub fn encode_file(&self) -> Result<Vec<u8>> {
        encode_local_key(1, &self.bytes)
    }

    pub fn read_file(path: &Path) -> Result<Self> {
        let bytes = std::fs::read(path).map_err(|error| io(path, "read recipient", error))?;
        let payload = decode_local_key(&bytes, 1, XWING_PUBLIC_KEY_LEN)?;
        Self::from_bytes(payload, "")
    }
}

/// Secret X-Wing draft-10 identity material. Debug output never exposes it.
pub struct XWingIdentity {
    secret: DecapsulationKey,
}

impl std::fmt::Debug for XWingIdentity {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("XWingIdentity(REDACTED)")
    }
}

impl XWingIdentity {
    pub fn generate() -> Result<(Self, XWingRecipient)> {
        let (secret, public) = XWingKem::generate_keypair();
        let public_bytes: [u8; XWING_PUBLIC_KEY_LEN] =
            public.to_bytes().as_slice().try_into().map_err(|_| {
                resource_refused("X-Wing implementation returned a noncanonical public key")
            })?;
        Ok((
            Self { secret },
            XWingRecipient {
                bytes: public_bytes,
                label: String::new(),
            },
        ))
    }

    pub fn from_seed(seed: [u8; XWING_SECRET_KEY_LEN]) -> Self {
        Self {
            secret: DecapsulationKey::from(seed),
        }
    }

    #[must_use]
    pub fn recipient(&self) -> XWingRecipient {
        let public = self.secret.encapsulation_key().to_bytes();
        XWingRecipient {
            bytes: public
                .as_slice()
                .try_into()
                .expect("X-Wing public key size"),
            label: String::new(),
        }
    }

    pub fn encode_file(&self) -> Result<Zeroizing<Vec<u8>>> {
        Ok(Zeroizing::new(encode_local_key(2, self.secret.as_bytes())?))
    }

    pub fn read_file(path: &Path) -> Result<Self> {
        let bytes =
            Zeroizing::new(std::fs::read(path).map_err(|error| io(path, "read identity", error))?);
        let payload = decode_local_key(&bytes, 2, XWING_SECRET_KEY_LEN)?;
        Ok(Self::from_seed(
            payload.try_into().expect("validated key size"),
        ))
    }
}

/// Unlock material. Password bytes are borrowed so the caller retains and can
/// zeroize its input buffer.
#[derive(Clone, Copy)]
pub enum Unlock<'a> {
    Identity(&'a XWingIdentity),
    Password(&'a [u8]),
}

struct Secret32([u8; 32]);

impl Drop for Secret32 {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

#[derive(Zeroize, ZeroizeOnDrop)]
struct KeyHierarchy {
    root_prk: [u8; 32],
    commitment: [u8; 32],
    envelope_mac: [u8; 32],
    control_segment: [u8; 32],
    payload_segment: [u8; 32],
    default_boundary: [u8; 32],
    strong_poly: [u8; 16],
    strong_prf: [u8; 16],
}

impl KeyHierarchy {
    fn derive(afk: &[u8; 32], archive_id: &[u8; 32]) -> Result<Self> {
        let major = 0_u16.to_be_bytes();
        let minor = 1_u16.to_be_bytes();
        let crypto = wire::CRYPTO_VERSION.to_be_bytes();
        let suite = wire::PAYLOAD_SUITE_V1.to_be_bytes();
        let root_context = wire::t1(
            "entrybound/root-salt/v1",
            &[
                wire::FORMAT_NAMESPACE,
                &major,
                &minor,
                &crypto,
                &suite,
                archive_id,
            ],
        )?;
        let root_salt: [u8; 32] = Sha256::digest(root_context).into();
        let (mut prk, _) = Hkdf::<Sha256>::extract(Some(&root_salt), afk);
        let mut root_prk = [0; 32];
        root_prk.copy_from_slice(&prk);
        prk.zeroize();
        Ok(Self {
            commitment: derive(&root_prk, "commitment-key", &[])?,
            envelope_mac: derive(&root_prk, "envelope-mac-key", &[])?,
            control_segment: derive(&root_prk, "control-segment-root", &[])?,
            payload_segment: derive(&root_prk, "payload-segment-root", &[])?,
            default_boundary: derive(&root_prk, "default-boundary-table-key", &[])?,
            strong_poly: derive(&root_prk, "strong-boundary-poly-key", &[])?,
            strong_prf: derive(&root_prk, "strong-boundary-prf-key", &[])?,
            root_prk,
        })
    }

    fn boundary_key(&self, mode: BoundaryMode) -> Result<(EncryptedBoundaryKey, &'static str)> {
        match mode {
            BoundaryMode::SecretGearTable => {
                let mut table = Box::new([0_u64; 256]);
                for (index, value) in table.iter_mut().enumerate() {
                    let index = u16::try_from(index)
                        .expect("the secret Gear table contains 256 values")
                        .to_be_bytes();
                    let transcript = wire::t1("entrybound/gear-table/v1", &[&index])?;
                    let digest = hmac(&self.default_boundary, &transcript)?;
                    *value = u64::from_be_bytes(digest[..8].try_into().unwrap());
                }
                Ok((
                    EncryptedBoundaryKey::SecretGearTable(table),
                    "gear-norm-secret-table-v1",
                ))
            }
            BoundaryMode::PhteAes128 => {
                let modulus = (1_u128 << 127) - 1;
                let mut counter = 0_u32;
                let polynomial = loop {
                    let transcript = wire::t1(
                        "entrybound/phte-poly-candidate/v1",
                        &[&counter.to_be_bytes()],
                    )?;
                    let digest = hmac(&self.strong_poly, &transcript)?;
                    let mut candidate = [0_u8; 16];
                    candidate.copy_from_slice(&digest[..16]);
                    candidate[0] &= 0x7f;
                    let candidate = u128::from_be_bytes(candidate);
                    if candidate != modulus {
                        break candidate;
                    }
                    counter = counter.checked_add(1).ok_or_else(|| {
                        crypto_integrity(
                            ReasonCode::CryptoBoundaryModeUnsupported,
                            "PHTE polynomial rejection sampling exhausted u32",
                        )
                    })?;
                };
                Ok((
                    EncryptedBoundaryKey::PhteAes128 {
                        polynomial,
                        aes_key: self.strong_prf,
                    },
                    "phte-aes128-norm-v1",
                ))
            }
        }
    }
}

/// Captures a filesystem directory with the encrypted keyed CDC policy and
/// writes one authenticated, metadata-private INDEXED archive.
pub fn pack_directory_encrypted(
    input: &Path,
    pack: PackOptions,
    options: EncryptedWriteOptions<'_>,
) -> Result<EncryptedArchive> {
    container::validate_write_options(&options)?;
    let afk = Secret32(random::<32>()?);
    let archive_id = random::<32>()?;
    let keys = KeyHierarchy::derive(&afk.0, &archive_id)?;
    let (boundary, chunker_prefix) = keys.boundary_key(options.boundary)?;
    let mut archive =
        crate::archive::plan_directory_encrypted(input, pack, &boundary, chunker_prefix)?;
    archive.descriptor.planner_id = match pack.profile {
        crate::planner::CompressionProfile::Fast => "fast-enc-v1",
        crate::planner::CompressionProfile::Balanced => "balanced-enc-v1",
        crate::planner::CompressionProfile::Dense => "dense-enc-v1",
        crate::planner::CompressionProfile::Extreme => "extreme-enc-v1",
    }
    .to_owned();
    let parts = crate::ecf::prepare_encrypted_plain_parts(&archive)?;
    container::encrypt_with_file_key(parts, &afk.0, archive_id, options)
}

fn derive<const N: usize>(root_prk: &[u8; 32], purpose: &str, context: &[u8]) -> Result<[u8; N]> {
    let info = wire::t1("entrybound/derive/v1", &[purpose.as_bytes(), context])?;
    let hkdf = Hkdf::<Sha256>::from_prk(root_prk)
        .map_err(|_| crypto_integrity(ReasonCode::CryptoSuiteUnsupported, "invalid root PRK"))?;
    let mut output = [0; N];
    hkdf.expand(&info, &mut output).map_err(|_| {
        crypto_integrity(
            ReasonCode::CryptoSuiteUnsupported,
            "HKDF output length is invalid",
        )
    })?;
    Ok(output)
}

fn commitment(
    keys: &KeyHierarchy,
    archive_id: &[u8; 32],
    padding: PaddingMode,
    boundary: BoundaryMode,
) -> Result<[u8; 32]> {
    let major = 0_u16.to_be_bytes();
    let minor = 1_u16.to_be_bytes();
    let crypto = wire::CRYPTO_VERSION.to_be_bytes();
    let suite = wire::PAYLOAD_SUITE_V1.to_be_bytes();
    let layout = [1_u8];
    let padding = [padding as u8];
    let boundary = [boundary as u8];
    let context = wire::t1(
        "entrybound/key-commitment/v1",
        &[
            wire::FORMAT_NAMESPACE,
            &major,
            &minor,
            &crypto,
            &suite,
            archive_id,
            &layout,
            &padding,
            &boundary,
        ],
    )?;
    hmac(&keys.commitment, &context)
}

fn public_crypto_context(
    archive_id: &[u8; 32],
    features: u64,
    padding: PaddingMode,
    boundary: BoundaryMode,
) -> Result<Vec<u8>> {
    let major = 0_u16.to_be_bytes();
    let minor = 1_u16.to_be_bytes();
    let crypto = wire::CRYPTO_VERSION.to_be_bytes();
    let suite = wire::PAYLOAD_SUITE_V1.to_be_bytes();
    let layout = [1_u8];
    let role = [1_u8];
    let features = features.to_be_bytes();
    let padding = [padding as u8];
    let boundary = [boundary as u8];
    wire::t1(
        "entrybound/public-crypto-context/v1",
        &[
            wire::FORMAT_NAMESPACE,
            &major,
            &minor,
            &crypto,
            &suite,
            archive_id,
            &layout,
            &role,
            &features,
            &padding,
            &boundary,
        ],
    )
}

fn envelope_mac(
    keys: &KeyHierarchy,
    public_context: &[u8],
    commitment: &[u8; 32],
    policy: ProtectionPolicy,
    stanzas: &[wire::RecipientStanza],
) -> Result<[u8; 32]> {
    let stanza_bytes = wire::encode_stanza_sequence(stanzas)?;
    let policy_byte = [policy as u8];
    let core = wire::t1(
        "entrybound/envelope-core/v1",
        &[public_context, commitment, &policy_byte, &stanza_bytes],
    )?;
    let transcript = wire::t1("entrybound/envelope-mac/v1", &[public_context, &core])?;
    hmac(&keys.envelope_mac, &transcript)
}

fn recipient_fingerprint(public_key: &[u8]) -> Result<[u8; 32]> {
    let kind = 1_u16.to_be_bytes();
    Ok(Sha256::digest(wire::t1(
        "entrybound/recipient-public-key/v1",
        &[&kind, public_key],
    )?)
    .into())
}

fn method_context(stanza: &wire::RecipientStanza) -> Result<Vec<u8>> {
    let version = 1_u16.to_be_bytes();
    let kind = stanza.stanza_type.to_be_bytes();
    let class = [stanza.protection_class];
    wire::t1(
        "entrybound/recipient-method-context/v1",
        &[
            &version,
            &kind,
            &class,
            &stanza.stanza_id,
            &stanza.recipient_hint,
            &stanza.method_parameters,
            &stanza.encapsulation,
        ],
    )
}

fn wrap_key(
    archive_id: &[u8; 32],
    method_secret: &[u8; 32],
    stanza: &wire::RecipientStanza,
) -> Result<Secret32> {
    let context_digest: [u8; 32] = Sha256::digest(method_context(stanza)?).into();
    let (mut prk, _) = Hkdf::<Sha256>::extract(Some(archive_id), method_secret);
    let suite = wire::PAYLOAD_SUITE_V1.to_be_bytes();
    let kind = stanza.stanza_type.to_be_bytes();
    let class = [stanza.protection_class];
    let info = wire::t1(
        "entrybound/recipient-wrap-key/v1",
        &[&suite, &kind, &class, &stanza.stanza_id, &context_digest],
    )?;
    let hkdf = Hkdf::<Sha256>::from_prk(prk.as_ref()).map_err(|_| {
        crypto_integrity(
            ReasonCode::CryptoRecipientStanzaInvalid,
            "invalid recipient wrap PRK",
        )
    })?;
    prk.zeroize();
    let mut key = [0; 32];
    hkdf.expand(&info, &mut key).map_err(|_| {
        crypto_integrity(
            ReasonCode::CryptoRecipientStanzaInvalid,
            "recipient wrap key failed",
        )
    })?;
    Ok(Secret32(key))
}

fn wrap_ad(archive_id: &[u8; 32], stanza: &wire::RecipientStanza) -> Result<Vec<u8>> {
    let major = 0_u16.to_be_bytes();
    let minor = 1_u16.to_be_bytes();
    let crypto = wire::CRYPTO_VERSION.to_be_bytes();
    let suite = wire::PAYLOAD_SUITE_V1.to_be_bytes();
    let stanza_version = 1_u16.to_be_bytes();
    let kind = stanza.stanza_type.to_be_bytes();
    let class = [stanza.protection_class];
    wire::t1(
        "entrybound/recipient-wrap-ad/v1",
        &[
            wire::FORMAT_NAMESPACE,
            &major,
            &minor,
            &crypto,
            &suite,
            archive_id,
            &stanza_version,
            &kind,
            &class,
            &stanza.stanza_id,
            &stanza.recipient_hint,
            &stanza.method_parameters,
            &stanza.encapsulation,
            &stanza.wrap_nonce,
        ],
    )
}

fn seal_afk(
    archive_id: &[u8; 32],
    method_secret: &[u8; 32],
    stanza: &wire::RecipientStanza,
    afk: &[u8; 32],
) -> Result<[u8; 48]> {
    let key = wrap_key(archive_id, method_secret, stanza)?;
    let ad = wrap_ad(archive_id, stanza)?;
    let ciphertext = aead_seal(&key.0, &stanza.wrap_nonce, &ad, afk)?;
    ciphertext.try_into().map_err(|_| {
        crypto_integrity(
            ReasonCode::CryptoRecipientStanzaInvalid,
            "wrapped AFK length mismatch",
        )
    })
}

fn open_afk(
    archive_id: &[u8; 32],
    method_secret: &[u8; 32],
    stanza: &wire::RecipientStanza,
) -> Result<Secret32> {
    let key = wrap_key(archive_id, method_secret, stanza)?;
    let ad = wrap_ad(archive_id, stanza)?;
    let plaintext = aead_open(&key.0, &stanza.wrap_nonce, &ad, &stanza.wrapped_afk)
        .map_err(|_| no_recipient())?;
    let bytes: [u8; 32] = plaintext.try_into().map_err(|_| no_recipient())?;
    Ok(Secret32(bytes))
}

fn aead_seal(key: &[u8; 32], nonce: &[u8; 12], ad: &[u8], bytes: &[u8]) -> Result<Vec<u8>> {
    Aes256GcmSiv::new_from_slice(key)
        .map_err(|_| crypto_integrity(ReasonCode::CryptoSuiteUnsupported, "invalid AEAD key"))?
        .encrypt(
            &Nonce::from(*nonce),
            Payload {
                msg: bytes,
                aad: ad,
            },
        )
        .map_err(|_| crypto_integrity(ReasonCode::CryptoAeadAuthFailed, "AEAD encryption failed"))
}

fn aead_open(key: &[u8; 32], nonce: &[u8; 12], ad: &[u8], bytes: &[u8]) -> Result<Vec<u8>> {
    Aes256GcmSiv::new_from_slice(key)
        .map_err(|_| crypto_integrity(ReasonCode::CryptoSuiteUnsupported, "invalid AEAD key"))?
        .decrypt(
            &Nonce::from(*nonce),
            Payload {
                msg: bytes,
                aad: ad,
            },
        )
        .map_err(|_| {
            crypto_integrity(
                ReasonCode::CryptoAeadAuthFailed,
                "encrypted record authentication failed",
            )
        })
}

fn hmac(key: &[u8], bytes: &[u8]) -> Result<[u8; 32]> {
    let mut mac = <HmacSha256 as HmacKeyInit>::new_from_slice(key)
        .map_err(|_| crypto_integrity(ReasonCode::CryptoSuiteUnsupported, "invalid HMAC key"))?;
    mac.update(bytes);
    Ok(mac.finalize().into_bytes().into())
}

fn a2id(salt: &[u8; 16], memory: u32, passes: u32, parallelism: u32) -> Vec<u8> {
    let mut bytes = b"A2ID".to_vec();
    bytes.extend_from_slice(&19_u32.to_be_bytes());
    bytes.extend_from_slice(&memory.to_be_bytes());
    bytes.extend_from_slice(&passes.to_be_bytes());
    bytes.extend_from_slice(&parallelism.to_be_bytes());
    bytes.extend_from_slice(salt);
    bytes
}

fn parse_a2id(bytes: &[u8], policy: CryptoPolicy) -> Result<([u8; 16], Params)> {
    if bytes.len() != wire::PASSWORD_METHOD_LEN || &bytes[..4] != b"A2ID" {
        return Err(stanza_invalid(
            "password stanza has malformed A2ID parameters",
        ));
    }
    let version = u32::from_be_bytes(bytes[4..8].try_into().unwrap());
    let memory = u32::from_be_bytes(bytes[8..12].try_into().unwrap());
    let passes = u32::from_be_bytes(bytes[12..16].try_into().unwrap());
    let parallelism = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
    let salt = bytes[20..36].try_into().unwrap();
    if version != 19
        || !(65_536..=1_048_576).contains(&memory)
        || !(3..=10).contains(&passes)
        || !(1..=16).contains(&parallelism)
        || memory < 8 * parallelism
        || memory % (4 * parallelism) != 0
    {
        return Err(stanza_invalid(
            "password KDF parameters violate the frozen Argon2id bounds",
        ));
    }
    if memory > policy.max_argon2_memory_kib
        || passes > policy.max_argon2_passes
        || parallelism > policy.max_argon2_parallelism
    {
        return Err(Diagnostic::new(
            OutcomeClass::PolicyRefused,
            ReasonCode::CryptoPasswordKdfPolicyRefused,
            "password KDF parameters exceed the caller policy",
        ));
    }
    let params = Params::new(memory, passes, parallelism, Some(32))
        .map_err(|_| stanza_invalid("password KDF parameters are invalid"))?;
    Ok((salt, params))
}

fn password_secret(password: &[u8], parameters: &[u8], policy: CryptoPolicy) -> Result<Secret32> {
    let (salt, params) = parse_a2id(parameters, policy)?;
    let mut output = [0; 32];
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
        .hash_password_into(password, &salt, &mut output)
        .map_err(|_| no_recipient())?;
    Ok(Secret32(output))
}

fn random<const N: usize>() -> Result<[u8; N]> {
    let mut bytes = [0; N];
    getrandom::fill(&mut bytes).map_err(|_| resource_refused("operating-system CSPRNG failed"))?;
    Ok(bytes)
}

fn encode_local_key(kind: u16, payload: &[u8]) -> Result<Vec<u8>> {
    let mut bytes = LOCAL_KEY_MAGIC.to_vec();
    bytes.extend_from_slice(&1_u16.to_be_bytes());
    bytes.extend_from_slice(&kind.to_be_bytes());
    bytes.extend_from_slice(
        &u32::try_from(payload.len())
            .map_err(|_| resource_refused("local key material exceeds u32"))?
            .to_be_bytes(),
    );
    bytes.extend_from_slice(payload);
    Ok(bytes)
}

fn decode_local_key(bytes: &[u8], kind: u16, len: usize) -> Result<&[u8]> {
    if bytes.len() != LOCAL_KEY_HEADER_LEN + len
        || &bytes[..4] != LOCAL_KEY_MAGIC
        || u16::from_be_bytes(bytes[4..6].try_into().unwrap()) != 1
        || u16::from_be_bytes(bytes[6..8].try_into().unwrap()) != kind
        || u32::from_be_bytes(bytes[8..12].try_into().unwrap()) as usize != len
    {
        return Err(stanza_invalid(
            "local key file has invalid type, version, or length",
        ));
    }
    Ok(&bytes[LOCAL_KEY_HEADER_LEN..])
}

fn no_recipient() -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::PolicyRefused,
        ReasonCode::CryptoNoMatchingRecipient,
        "no supplied credential unlocked the encrypted archive",
    )
}

fn stanza_invalid(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::Nonconforming,
        ReasonCode::CryptoRecipientStanzaInvalid,
        detail,
    )
}

fn crypto_integrity(code: ReasonCode, detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(OutcomeClass::Corrupt, code, detail)
}

fn resource_refused(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::PolicyRefused,
        ReasonCode::CryptoResourcePolicyRefused,
        detail,
    )
}

fn io(path: &Path, operation: &str, error: std::io::Error) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::PolicyRefused,
        ReasonCode::Io,
        format!("cannot {operation} '{}': {error}", path.display()),
    )
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha3::Sha3_256;

    fn decode_hex(value: &str) -> Vec<u8> {
        value
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }

    fn vector(name: &str) -> &'static str {
        include_str!("../../../../docs/crypto-wire-v1-vectors.txt")
            .lines()
            .find_map(|line| {
                line.strip_prefix(name)
                    .and_then(|value| value.strip_prefix('='))
            })
            .unwrap_or_else(|| panic!("missing crypto vector {name}"))
    }

    #[test]
    fn corrected_v5_and_v6_wrapping_vectors() {
        let archive_id: [u8; 32] = core::array::from_fn(|i| i as u8);
        let afk: [u8; 32] = core::array::from_fn(|i| 0x20 + i as u8);
        let mut hybrid = wire::RecipientStanza {
            stanza_type: 1,
            protection_class: 1,
            stanza_id: core::array::from_fn(|i| 0x40 + i as u8),
            recipient_hint: [0; 16],
            method_parameters: wire::XWING_METHOD.to_vec(),
            encapsulation: (0..1120).map(|i| (i % 256) as u8).collect(),
            wrap_nonce: core::array::from_fn(|i| 0x50 + i as u8),
            wrapped_afk: [0; 48],
        };
        let secret: [u8; 32] = core::array::from_fn(|i| 0xb0 + i as u8);
        let hybrid_context = method_context(&hybrid).unwrap();
        assert_eq!(hex(&hybrid_context), vector("V5_METHOD_CONTEXT"));
        assert_eq!(
            hex(&Sha256::digest(&hybrid_context)),
            vector("V5_METHOD_CONTEXT_SHA256")
        );
        let (hybrid_prk, _) = Hkdf::<Sha256>::extract(Some(&archive_id), &secret);
        assert_eq!(hex(hybrid_prk.as_ref()), vector("V5_WRAP_PRK"));
        assert_eq!(
            hex(&wrap_key(&archive_id, &secret, &hybrid).unwrap().0),
            vector("V5_WRAP_KEY")
        );
        assert_eq!(
            hex(&wrap_ad(&archive_id, &hybrid).unwrap()),
            vector("V5_WRAP_AD")
        );
        assert_eq!(
            hex(&Sha256::digest(wrap_ad(&archive_id, &hybrid).unwrap())),
            vector("V5_WRAP_AD_SHA256")
        );
        hybrid.wrapped_afk = seal_afk(&archive_id, &secret, &hybrid, &afk).unwrap();
        assert_eq!(hex(&hybrid.wrapped_afk), vector("V5_WRAPPED_AFK"));

        let parameters =
            decode_hex("4132494400000013000400000000000300000004606162636465666768696a6b6c6d6e6f");
        let secret = password_secret(
            b"correct horse battery staple",
            &parameters,
            CryptoPolicy::default(),
        )
        .unwrap();
        assert_eq!(hex(&secret.0), vector("V6_ARGON2ID_OUTPUT"));
        let mut password = wire::RecipientStanza {
            stanza_type: 2,
            protection_class: 2,
            stanza_id: core::array::from_fn(|i| 0x70 + i as u8),
            recipient_hint: [0; 16],
            method_parameters: parameters,
            encapsulation: vec![],
            wrap_nonce: core::array::from_fn(|i| 0x80 + i as u8),
            wrapped_afk: [0; 48],
        };
        let password_context = method_context(&password).unwrap();
        assert_eq!(hex(&password_context), vector("V6_METHOD_CONTEXT"));
        assert_eq!(
            hex(&Sha256::digest(&password_context)),
            vector("V6_METHOD_CONTEXT_SHA256")
        );
        let (password_prk, _) = Hkdf::<Sha256>::extract(Some(&archive_id), &secret.0);
        assert_eq!(hex(password_prk.as_ref()), vector("V6_WRAP_PRK"));
        assert_eq!(
            hex(&wrap_key(&archive_id, &secret.0, &password).unwrap().0),
            vector("V6_WRAP_KEY")
        );
        assert_eq!(
            hex(&wrap_ad(&archive_id, &password).unwrap()),
            vector("V6_WRAP_AD")
        );
        assert_eq!(
            hex(&Sha256::digest(wrap_ad(&archive_id, &password).unwrap())),
            vector("V6_WRAP_AD_SHA256")
        );
        password.wrapped_afk = seal_afk(&archive_id, &secret.0, &password, &afk).unwrap();
        assert_eq!(hex(&password.wrapped_afk), vector("V6_WRAPPED_AFK"));
    }

    #[test]
    fn frozen_v1_to_v4_construction_vectors() {
        let archive_id: [u8; 32] = core::array::from_fn(|i| i as u8);
        let afk: [u8; 32] = core::array::from_fn(|i| 0x20 + i as u8);
        let keys = KeyHierarchy::derive(&afk, &archive_id).unwrap();
        assert_eq!(
            hex(&keys.root_prk),
            "47fe02a0218467666c70f5aca6c8fc01e67b3b75ceec9c405498c750745b2f0e"
        );
        assert_eq!(
            hex(&keys.commitment),
            "8ebf3989bc145eea11a334641019095bec09fa264f55d3f4365e899a49d60cdb"
        );
        assert_eq!(
            hex(&keys.envelope_mac),
            "65479e34546cacf6083804c20f1b3bc0df802c926f37684c4069cf19bed2689e"
        );
        assert_eq!(
            hex(&keys.control_segment),
            "73026777ba7515733ec28c7f69050cb11372a73d06ccd50cfeeb01faf0ef6d0c"
        );
        assert_eq!(
            hex(&keys.payload_segment),
            "ebe5cab2cbf934472ab7f96785b8ab4468c7f67118d8d3b511e1aa3ecdb3cbf0"
        );
        assert_eq!(
            hex(&keys.default_boundary),
            "2659b0e17cf2ee880df551bf040cb58f147471d615acb46268fede0c3c723cbc"
        );
        assert_eq!(hex(&keys.strong_poly), "035551a60590525f9529aab1ce233e41");
        assert_eq!(hex(&keys.strong_prf), "f35f2c09669f609e858fc8ff02d81150");
        assert_eq!(
            hex(&commitment(
                &keys,
                &archive_id,
                PaddingMode::Bucketed,
                BoundaryMode::SecretGearTable
            )
            .unwrap()),
            "16bb3e788dce7f99545ae0fc098ccf2c8c0087b8cf539095af977b30c3c7dcfc"
        );

        let salt: [u8; 16] = core::array::from_fn(|i| 0xa0 + i as u8);
        let segment = container::segment_key(&keys, &archive_id, 2, 7, &salt).unwrap();
        assert_eq!(
            hex(&segment.0),
            "896e85aced3989236dff9b96e3f08ea37456b61c720de93b82db7391b16243bc"
        );
        assert_eq!(
            container::data_nonce(3),
            decode_hex("000000000000000000000003").as_slice()
        );
        assert_eq!(
            container::end_nonce(4),
            decode_hex("ffffffff0000000000000004").as_slice()
        );
        let header = decode_hex(
            "45425347000102000000000000000007a0a1a2a3a4a5a6a7a8a9aaabacadaeaf0000000400000000000000000000000000000000000042300000000000000000",
        );
        let protected =
            decode_hex("4542433100010100000000000000000700000000000000030000000000001010");
        assert_eq!(
            hex(&Sha256::digest(
                container::record_ad(&archive_id, &header, &protected).unwrap()
            )),
            "cf1e885dfd548e088f5405eb8c9a44d4ba1573dabce0743a4fbaadcad8db841a"
        );

        let raw_a = [1_u8, 2, 3];
        let raw_b = [0xa0_u8, 0xa1];
        let mut keyed = [&raw_a[..], &raw_b[..]].map(|raw| {
            let digest: [u8; 32] =
                Sha256::digest(wire::t1("entrybound/recipient-stanza-sort/v1", &[raw]).unwrap())
                    .into();
            (digest, raw)
        });
        keyed.sort_by_key(|item| item.0);
        let mut sequence = (keyed.len() as u64).to_be_bytes().to_vec();
        for (_, raw) in keyed {
            sequence.extend_from_slice(&(raw.len() as u64).to_be_bytes());
            sequence.extend_from_slice(raw);
        }
        assert_eq!(
            hex(&Sha256::digest(
                wire::t1("entrybound/recipient-set/v1", &[&sequence]).unwrap()
            )),
            "d25792586b6102e7d8d19e4dd96cbbef18a409f05f0dc921166a5f0607bbc61c"
        );

        let mut combiner = (0_u8..32).collect::<Vec<_>>();
        combiner.extend(0x20_u8..0x40);
        combiner.extend(0x40_u8..0x60);
        combiner.extend(0x60_u8..0x80);
        combiner.extend_from_slice(br"\.//^\");
        assert_eq!(
            hex(&Sha3_256::digest(combiner)),
            "0acca09f2fb739bc89668dbcd01ae5aebf9b72c6fe013297e3baa96854468491"
        );
    }

    #[test]
    fn local_key_files_are_typed_and_secret_debug_is_redacted() {
        let (identity, recipient) = XWingIdentity::generate().unwrap();
        assert_eq!(
            XWingRecipient::from_bytes(recipient.bytes(), "").unwrap(),
            recipient
        );
        assert_eq!(format!("{identity:?}"), "XWingIdentity(REDACTED)");
        assert_eq!(identity.encode_file().unwrap().len(), 44);
        assert_eq!(recipient.encode_file().unwrap().len(), 1228);
        assert_eq!(
            XWingRecipient::from_bytes(&[0; XWING_PUBLIC_KEY_LEN - 1], "")
                .unwrap_err()
                .code(),
            ReasonCode::CryptoRecipientStanzaInvalid
        );
    }

    #[test]
    fn encrypted_boundary_derivation_and_phte_ranges_are_frozen() {
        let archive_id: [u8; 32] = core::array::from_fn(|i| i as u8);
        let afk: [u8; 32] = core::array::from_fn(|i| 0x20 + i as u8);
        let keys = KeyHierarchy::derive(&afk, &archive_id).unwrap();
        let (gear, prefix) = keys.boundary_key(BoundaryMode::SecretGearTable).unwrap();
        assert_eq!(prefix, "gear-norm-secret-table-v1");
        let EncryptedBoundaryKey::SecretGearTable(table) = gear else {
            panic!("wrong encrypted boundary type")
        };
        assert_eq!(
            &table[..4],
            &[
                0xc182_6e0c_5914_ee82,
                0xb32f_0b50_6e61_c623,
                0x8cba_ac85_8e6a_87db,
                0x658e_5b86_cbfe_65c4,
            ]
        );

        let (phte, prefix) = keys.boundary_key(BoundaryMode::PhteAes128).unwrap();
        assert_eq!(prefix, "phte-aes128-norm-v1");
        let EncryptedBoundaryKey::PhteAes128 {
            polynomial,
            aes_key,
        } = phte
        else {
            panic!("wrong encrypted boundary type")
        };
        assert_eq!(polynomial, 0x10a7_9729_d273_d8af_c608_51e9_8b49_1a65);
        assert_eq!(aes_key, keys.strong_prf);
        let parameters = crate::chunker::ChunkingParameters {
            chunker_id: "phte-test-vector",
            minimum_size: 16,
            target_size: 32,
            maximum_size: 64,
        };
        let input = (0..256).map(|value| value as u8).collect::<Vec<_>>();
        let ranges = crate::chunker::chunk_ranges_encrypted(
            &input,
            parameters,
            &EncryptedBoundaryKey::PhteAes128 {
                polynomial,
                aes_key,
            },
        )
        .unwrap();
        let ends = ranges.iter().map(|range| range.end).collect::<Vec<_>>();
        assert_eq!(ends, [34, 71, 93, 153, 217, 256]);
    }

    #[test]
    fn argon2_policy_is_checked_before_kdf_work() {
        let parameters = a2id(&[0x60; 16], 262_144, 3, 4);
        let policy = CryptoPolicy {
            max_argon2_memory_kib: 65_536,
            ..CryptoPolicy::default()
        };
        let Err(error) = password_secret(b"not evaluated", &parameters, policy) else {
            panic!("over-policy Argon2 parameters were accepted")
        };
        assert_eq!(error.code(), ReasonCode::CryptoPasswordKdfPolicyRefused);

        let malformed = a2id(&[0x60; 16], 32_768, 3, 4);
        let Err(error) = password_secret(b"not evaluated", &malformed, CryptoPolicy::default())
        else {
            panic!("malformed Argon2 parameters were accepted")
        };
        assert_eq!(error.code(), ReasonCode::CryptoRecipientStanzaInvalid);
    }
}
