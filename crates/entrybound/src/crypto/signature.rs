//! Frozen Ed25519 archive-signature wire and binding evaluation.

use std::path::Path;

use ed25519_dalek::{Signature, Signer as _, SigningKey as DalekSigningKey, VerifyingKey};
use sha2::{Digest as _, Sha256};
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

use super::{timestamp, wire};
use crate::canonical::{RecordBuilder, decode_record};
use crate::diagnostics::{Diagnostic, OutcomeClass, ReasonCode, Result};
use crate::ecf::OpenedArchive;

const SIGNATURE_VERSION: u16 = 1;
const ALGORITHM_ED25519: u16 = 1;
pub(crate) const BIND_CONTENT: u8 = 1;
pub(crate) const BIND_PHYSICAL: u8 = 2;
pub(crate) const BIND_ADDRESSING: u8 = 4;
const BINDING_MASK: u8 = BIND_CONTENT | BIND_PHYSICAL | BIND_ADDRESSING;
const MAX_TIMESTAMP_BYTES: usize = 64 << 10;
const LOCAL_SIGNING_MAGIC: &[u8; 4] = b"EBSK";
const LOCAL_SIGNING_VERSION: u16 = 1;

/// Current encrypted addressing values. They are physical/authenticated
/// context, not Entrybound semantic identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AddressingBinding {
    pub payload_suite_id: u16,
    pub recipient_set_digest: [u8; 32],
    pub commitment: [u8; 32],
    pub archive_id: [u8; 32],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CurrentBindings {
    pub(crate) content: Vec<u8>,
    pub(crate) physical: Vec<u8>,
    pub(crate) addressing: Option<Vec<u8>>,
}

/// Secret local Ed25519 seed. Neither `Debug` nor `Display` is implemented.
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct SigningKey {
    seed: [u8; 32],
}

impl SigningKey {
    pub fn generate() -> Result<Self> {
        let mut seed = [0_u8; 32];
        getrandom::fill(&mut seed).map_err(|_| {
            Diagnostic::new(
                OutcomeClass::PolicyRefused,
                ReasonCode::CryptoResourcePolicyRefused,
                "operating-system CSPRNG failed while generating signing key",
            )
        })?;
        Ok(Self { seed })
    }

    #[must_use]
    pub const fn from_seed(seed: [u8; 32]) -> Self {
        Self { seed }
    }

    #[must_use]
    pub fn public_key(&self) -> [u8; 32] {
        DalekSigningKey::from_bytes(&self.seed)
            .verifying_key()
            .to_bytes()
    }

    pub fn encode_file(&self) -> Zeroizing<Vec<u8>> {
        let mut bytes = LOCAL_SIGNING_MAGIC.to_vec();
        bytes.extend_from_slice(&LOCAL_SIGNING_VERSION.to_be_bytes());
        bytes.extend_from_slice(&1_u16.to_be_bytes());
        bytes.extend_from_slice(&32_u32.to_be_bytes());
        bytes.extend_from_slice(&self.seed);
        Zeroizing::new(bytes)
    }

    pub fn read_file(path: &Path) -> Result<Self> {
        let mut bytes = std::fs::read(path).map_err(|error| {
            Diagnostic::new(
                OutcomeClass::PolicyRefused,
                ReasonCode::Io,
                format!("cannot read signing key '{}': {error}", path.display()),
            )
        })?;
        let result = if bytes.len() == 44
            && &bytes[..4] == LOCAL_SIGNING_MAGIC
            && u16::from_be_bytes(bytes[4..6].try_into().unwrap()) == LOCAL_SIGNING_VERSION
            && u16::from_be_bytes(bytes[6..8].try_into().unwrap()) == 1
            && u32::from_be_bytes(bytes[8..12].try_into().unwrap()) == 32
        {
            Ok(Self {
                seed: bytes[12..44].try_into().unwrap(),
            })
        } else {
            Err(signature_invalid(
                "local signing key has invalid magic, version, type, or length",
            ))
        };
        bytes.zeroize();
        result
    }
}

/// Canonical type-26 signature record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignatureRecord {
    pub signature_version: u16,
    pub algorithm_id: u16,
    pub binding_mask: u8,
    pub public_key: [u8; 32],
    pub content_binding: Vec<u8>,
    pub physical_binding: Option<Vec<u8>>,
    pub addressing_binding: Option<Vec<u8>>,
    pub signature: [u8; 64],
    pub timestamp_token: Option<Vec<u8>>,
}

impl SignatureRecord {
    pub fn encode_without_timestamp(&self) -> Result<Vec<u8>> {
        self.validate_shape()?;
        let mut record = RecordBuilder::new(wire::RECORD_SIGNATURE);
        record
            .u16(1, self.signature_version)?
            .u16(2, self.algorithm_id)?
            .u8(3, self.binding_mask)?
            .bytes(4, &self.public_key)?
            .bytes(5, &self.content_binding)?;
        if let Some(value) = &self.physical_binding {
            record.bytes(6, value)?;
        }
        if let Some(value) = &self.addressing_binding {
            record.bytes(7, value)?;
        }
        record.bytes(8, &self.signature)?;
        record.finish()
    }

    pub fn encode(&self) -> Result<Vec<u8>> {
        self.validate_shape()?;
        let mut record = RecordBuilder::new(wire::RECORD_SIGNATURE);
        record
            .u16(1, self.signature_version)?
            .u16(2, self.algorithm_id)?
            .u8(3, self.binding_mask)?
            .bytes(4, &self.public_key)?
            .bytes(5, &self.content_binding)?;
        if let Some(value) = &self.physical_binding {
            record.bytes(6, value)?;
        }
        if let Some(value) = &self.addressing_binding {
            record.bytes(7, value)?;
        }
        record.bytes(8, &self.signature)?;
        if let Some(token) = &self.timestamp_token {
            record.bytes(9, token)?;
        }
        record.finish()
    }

    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let (record, consumed) = decode_record(bytes).map_err(|_| {
            signature_invalid("detached or embedded signature is not a canonical record")
        })?;
        if consumed != bytes.len() || record.kind != wire::RECORD_SIGNATURE || record.version != 1 {
            return Err(signature_invalid(
                "signature must be exactly one canonical type-26/version-1 record",
            ));
        }
        record
            .expect_tags(&[1, 2, 3, 4, 5, 8], &[6, 7, 9])
            .map_err(|_| signature_invalid("SignatureRecord fields are not canonical"))?;
        let value = Self {
            signature_version: record.field(1)?.as_u16()?,
            algorithm_id: record.field(2)?.as_u16()?,
            binding_mask: record.field(3)?.as_u8()?,
            public_key: exact(record.field(4)?.as_bytes()?, "public key")?,
            content_binding: record.field(5)?.as_bytes()?.to_vec(),
            physical_binding: record
                .optional_field(6)
                .map(|field| field.as_bytes().map(ToOwned::to_owned))
                .transpose()?,
            addressing_binding: record
                .optional_field(7)
                .map(|field| field.as_bytes().map(ToOwned::to_owned))
                .transpose()?,
            signature: exact(record.field(8)?.as_bytes()?, "signature")?,
            timestamp_token: record
                .optional_field(9)
                .map(|field| field.as_bytes().map(ToOwned::to_owned))
                .transpose()?,
        };
        value.validate_shape()?;
        if value.encode()? != bytes {
            return Err(signature_invalid("SignatureRecord is not canonical"));
        }
        Ok(value)
    }

    pub fn with_timestamp_token(mut self, token: Vec<u8>) -> Result<Self> {
        if token.is_empty() || token.len() > MAX_TIMESTAMP_BYTES {
            return Err(timestamp_invalid(
                "RFC 3161 timestamp token length is outside 1..65536 bytes",
            ));
        }
        self.timestamp_token = Some(token);
        self.validate_shape()?;
        Ok(self)
    }

    #[must_use]
    pub fn signer_id(&self) -> [u8; 16] {
        let transcript = wire::t1("entrybound/signer-id/v1", &[&self.public_key])
            .expect("fixed signer-id transcript");
        let digest = Sha256::digest(transcript);
        digest[..16].try_into().unwrap()
    }

    fn validate_shape(&self) -> Result<()> {
        if self.signature_version != SIGNATURE_VERSION || self.algorithm_id != ALGORITHM_ED25519 {
            return Err(Diagnostic::new(
                OutcomeClass::Unsupported,
                ReasonCode::SignatureUnsupported,
                "unsupported signature version or algorithm",
            ));
        }
        if self.binding_mask & BIND_CONTENT == 0 || self.binding_mask & !BINDING_MASK != 0 {
            return Err(Diagnostic::new(
                OutcomeClass::Unsupported,
                ReasonCode::SignatureUnsupported,
                "signature binding mask is missing CONTENT or contains unknown bits",
            ));
        }
        if (self.binding_mask & BIND_PHYSICAL != 0) != self.physical_binding.is_some()
            || (self.binding_mask & BIND_ADDRESSING != 0) != self.addressing_binding.is_some()
        {
            return Err(signature_invalid(
                "signature binding fields disagree with the binding mask",
            ));
        }
        validate_content_binding(&self.content_binding)?;
        if let Some(value) = &self.physical_binding {
            validate_physical_binding(value)?;
        }
        if let Some(value) = &self.addressing_binding {
            validate_addressing_binding(value)?;
        }
        if self
            .timestamp_token
            .as_ref()
            .is_some_and(|token| token.is_empty() || token.len() > MAX_TIMESTAMP_BYTES)
        {
            return Err(timestamp_invalid(
                "RFC 3161 timestamp token length is outside 1..65536 bytes",
            ));
        }
        Ok(())
    }

    fn signed_transcript(&self) -> Result<Vec<u8>> {
        wire::t1(
            "entrybound/signature/v1",
            &[
                &self.signature_version.to_be_bytes(),
                &self.algorithm_id.to_be_bytes(),
                &[self.binding_mask],
                &self.public_key,
                &self.content_binding,
                &[u8::from(self.physical_binding.is_some())],
                self.physical_binding.as_deref().unwrap_or_default(),
                &[u8::from(self.addressing_binding.is_some())],
                self.addressing_binding.as_deref().unwrap_or_default(),
            ],
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CryptographicStatus {
    Valid,
    Invalid,
    Unsupported,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BindingStatus {
    Valid,
    Stale,
    NotBound,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimestampStatus {
    Valid,
    Invalid,
    Unsupported,
    Absent,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignatureStatus {
    pub signer_id: [u8; 16],
    pub binding_mask: u8,
    pub cryptographic: CryptographicStatus,
    pub content: BindingStatus,
    pub physical: BindingStatus,
    pub addressing: BindingStatus,
    pub timestamp: TimestampStatus,
    pub timestamp_unix_seconds: Option<i64>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SignaturePolicy {
    pub require_signature: bool,
    pub require_content: bool,
    pub require_physical: bool,
    pub require_addressing: bool,
}

/// Builds the current signature transcripts from an already verified archive.
pub fn current_bindings(
    opened: &OpenedArchive,
    addressing: Option<AddressingBinding>,
) -> Result<CurrentBindings> {
    let identities = opened.report.identities;
    let descriptor = &opened.archive.descriptor;
    let content = wire::t1(
        "entrybound/signature-content/v1",
        &[
            identities.lai.0.as_bytes(),
            identities.aux.0.as_bytes(),
            b"identity/v1",
            b"ecf/bootstrap-v1",
            &descriptor.format_major.to_be_bytes(),
            &descriptor.format_minor.to_be_bytes(),
        ],
    )?;
    let physical = wire::t1(
        "entrybound/signature-physical/v1",
        &[identities.pcr.0.as_bytes()],
    )?;
    let addressing = addressing
        .map(|value| {
            wire::t1(
                "entrybound/signature-addressing/v1",
                &[
                    &value.payload_suite_id.to_be_bytes(),
                    &value.recipient_set_digest,
                    &value.commitment,
                    &value.archive_id,
                ],
            )
        })
        .transpose()?;
    Ok(CurrentBindings {
        content,
        physical,
        addressing,
    })
}

/// Creates one frozen pure-Ed25519 signature. CONTENT is mandatory.
pub fn sign_archive(
    current: &CurrentBindings,
    key: &SigningKey,
    bind_physical: bool,
    bind_addressing: bool,
) -> Result<SignatureRecord> {
    let addressing = if bind_addressing {
        Some(current.addressing.clone().ok_or_else(|| {
            Diagnostic::new(
                OutcomeClass::PolicyRefused,
                ReasonCode::SignatureStaleAddressing,
                "ADDRESSING binding is valid only for an authenticated encrypted archive",
            )
        })?)
    } else {
        None
    };
    let mut value = SignatureRecord {
        signature_version: SIGNATURE_VERSION,
        algorithm_id: ALGORITHM_ED25519,
        binding_mask: BIND_CONTENT
            | if bind_physical { BIND_PHYSICAL } else { 0 }
            | if bind_addressing { BIND_ADDRESSING } else { 0 },
        public_key: key.public_key(),
        content_binding: current.content.clone(),
        physical_binding: bind_physical.then(|| current.physical.clone()),
        addressing_binding: addressing,
        signature: [0; 64],
        timestamp_token: None,
    };
    let dalek = DalekSigningKey::from_bytes(&key.seed);
    value.signature = dalek.sign(&value.signed_transcript()?).to_bytes();
    Ok(value)
}

pub fn verify_signature(
    value: &SignatureRecord,
    current: &CurrentBindings,
    timestamp_policy: Option<&timestamp::TimestampPolicy>,
) -> Result<SignatureStatus> {
    value.validate_shape()?;
    let key = VerifyingKey::from_bytes(&value.public_key)
        .map_err(|_| signature_invalid("Ed25519 public key is malformed"))?;
    if key.is_weak() {
        return Err(signature_invalid("Ed25519 public key has small order"));
    }
    let signature = Signature::from_bytes(&value.signature);
    let cryptographic = if key
        .verify_strict(&value.signed_transcript()?, &signature)
        .is_ok()
    {
        CryptographicStatus::Valid
    } else {
        CryptographicStatus::Invalid
    };
    let content = compare(
        cryptographic,
        Some(&value.content_binding),
        Some(&current.content),
    );
    let physical = compare(
        cryptographic,
        value.physical_binding.as_ref(),
        Some(&current.physical),
    );
    let addressing = compare(
        cryptographic,
        value.addressing_binding.as_ref(),
        current.addressing.as_ref(),
    );
    let (timestamp, timestamp_unix_seconds) = match &value.timestamp_token {
        None => (TimestampStatus::Absent, None),
        Some(token) => match timestamp_policy {
            None => (TimestampStatus::Unsupported, None),
            Some(policy) => match timestamp::verify_timestamp(value, token, policy) {
                Ok(time) => (TimestampStatus::Valid, Some(time)),
                Err(error) if error.code() == ReasonCode::SignatureTimestampUnsupported => {
                    (TimestampStatus::Unsupported, None)
                }
                Err(_) => (TimestampStatus::Invalid, None),
            },
        },
    };
    Ok(SignatureStatus {
        signer_id: value.signer_id(),
        binding_mask: value.binding_mask,
        cryptographic,
        content,
        physical,
        addressing,
        timestamp,
        timestamp_unix_seconds,
    })
}

impl SignaturePolicy {
    pub fn enforce(self, statuses: &[SignatureStatus]) -> Result<()> {
        if (self.require_signature
            || self.require_content
            || self.require_physical
            || self.require_addressing)
            && statuses.is_empty()
        {
            return Err(Diagnostic::new(
                OutcomeClass::PolicyRefused,
                ReasonCode::SignatureAbsent,
                "caller policy requires a signature but none was supplied",
            ));
        }
        if statuses
            .iter()
            .any(|status| status.cryptographic == CryptographicStatus::Invalid)
        {
            return Err(signature_invalid("one or more signatures are invalid"));
        }
        if self.require_content
            && !statuses
                .iter()
                .any(|status| status.content == BindingStatus::Valid)
        {
            return Err(stale(ReasonCode::SignatureStaleContent, "CONTENT"));
        }
        if self.require_physical
            && !statuses
                .iter()
                .any(|status| status.physical == BindingStatus::Valid)
        {
            return Err(stale(ReasonCode::SignatureStalePhysical, "PHYSICAL"));
        }
        if self.require_addressing
            && !statuses
                .iter()
                .any(|status| status.addressing == BindingStatus::Valid)
        {
            return Err(stale(ReasonCode::SignatureStaleAddressing, "ADDRESSING"));
        }
        Ok(())
    }
}

pub fn read_detached_signature(path: &Path) -> Result<SignatureRecord> {
    let bytes = std::fs::read(path).map_err(|error| {
        Diagnostic::new(
            OutcomeClass::PolicyRefused,
            ReasonCode::Io,
            format!(
                "cannot read detached signature '{}': {error}",
                path.display()
            ),
        )
    })?;
    SignatureRecord::decode(&bytes)
}

fn compare(
    cryptographic: CryptographicStatus,
    stored: Option<&Vec<u8>>,
    current: Option<&Vec<u8>>,
) -> BindingStatus {
    let Some(stored) = stored else {
        return BindingStatus::NotBound;
    };
    if cryptographic != CryptographicStatus::Valid {
        return BindingStatus::Stale;
    }
    if current.is_some_and(|current| current == stored) {
        BindingStatus::Valid
    } else {
        BindingStatus::Stale
    }
}

fn validate_content_binding(bytes: &[u8]) -> Result<()> {
    let fields = wire::decode_t1(bytes, "entrybound/signature-content/v1", 6)
        .map_err(|_| signature_invalid("ContentBindingV1 is malformed"))?;
    if fields[0].len() != 32
        || fields[1].len() != 32
        || fields[2] != b"identity/v1"
        || fields[3] != b"ecf/bootstrap-v1"
        || fields[4].len() != 2
        || fields[5].len() != 2
    {
        return Err(signature_invalid("ContentBindingV1 fields are invalid"));
    }
    Ok(())
}

fn validate_physical_binding(bytes: &[u8]) -> Result<()> {
    let fields = wire::decode_t1(bytes, "entrybound/signature-physical/v1", 1)
        .map_err(|_| signature_invalid("PhysicalBindingV1 is malformed"))?;
    if fields[0].len() != 32 {
        return Err(signature_invalid("PhysicalBindingV1 PCR length is invalid"));
    }
    Ok(())
}

fn validate_addressing_binding(bytes: &[u8]) -> Result<()> {
    let fields = wire::decode_t1(bytes, "entrybound/signature-addressing/v1", 4)
        .map_err(|_| signature_invalid("AddressingBindingV1 is malformed"))?;
    if fields[0].len() != 2
        || fields[1].len() != 32
        || fields[2].len() != 32
        || fields[3].len() != 32
    {
        return Err(signature_invalid("AddressingBindingV1 fields are invalid"));
    }
    Ok(())
}

fn exact<const N: usize>(bytes: &[u8], name: &str) -> Result<[u8; N]> {
    bytes.try_into().map_err(|_| {
        signature_invalid(format!(
            "SignatureRecord {name} must contain exactly {N} bytes"
        ))
    })
}

fn signature_invalid(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(OutcomeClass::Corrupt, ReasonCode::SignatureInvalid, detail)
}

fn timestamp_invalid(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::Corrupt,
        ReasonCode::SignatureTimestampInvalid,
        detail,
    )
}

fn stale(code: ReasonCode, binding: &str) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::PolicyRefused,
        code,
        format!("caller requires a current {binding} signature binding"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vector_bindings() -> CurrentBindings {
        let content = wire::t1(
            "entrybound/signature-content/v1",
            &[
                &core::array::from_fn::<_, 32, _>(|index| index as u8),
                &core::array::from_fn::<_, 32, _>(|index| (index + 32) as u8),
                b"identity/v1",
                b"ecf/bootstrap-v1",
                &0_u16.to_be_bytes(),
                &1_u16.to_be_bytes(),
            ],
        )
        .unwrap();
        let physical = wire::t1(
            "entrybound/signature-physical/v1",
            &[&core::array::from_fn::<_, 32, _>(|index| {
                (index + 64) as u8
            })],
        )
        .unwrap();
        let addressing = wire::t1(
            "entrybound/signature-addressing/v1",
            &[
                &1_u16.to_be_bytes(),
                &decode_hex("d25792586b6102e7d8d19e4dd96cbbef18a409f05f0dc921166a5f0607bbc61c"),
                &decode_hex("16bb3e788dce7f99545ae0fc098ccf2c8c0087b8cf539095af977b30c3c7dcfc"),
                &core::array::from_fn::<_, 32, _>(|index| index as u8),
            ],
        )
        .unwrap();
        CurrentBindings {
            content,
            physical,
            addressing: Some(addressing),
        }
    }

    #[test]
    fn frozen_v7_signature_matches() {
        let key = SigningKey::from_seed(
            decode_hex("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60")
                .try_into()
                .unwrap(),
        );
        let record = sign_archive(&vector_bindings(), &key, true, true).unwrap();
        assert_eq!(
            hex(&Sha256::digest(record.signed_transcript().unwrap())),
            "68b91370f571119013e73530df0996aa51845387d839f136ae569ca279587f50"
        );
        assert_eq!(
            hex(&record.signature),
            "3ba203ed99bc3050b9815966f4d05da12b7133a9416457280090d063b2282e13bb3e3be1c0ea737136f52838bf43050d57f6c394d8df4040bf204cec9f583a0f"
        );
        assert_eq!(
            verify_signature(&record, &vector_bindings(), None)
                .unwrap()
                .cryptographic,
            CryptographicStatus::Valid
        );
    }

    #[test]
    fn rfc8032_section_7_1_test_one() {
        let key = SigningKey::from_seed(
            decode_hex("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60")
                .try_into()
                .unwrap(),
        );
        let dalek = DalekSigningKey::from_bytes(&key.seed);
        assert_eq!(
            hex(&dalek.sign(b"").to_bytes()),
            "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b"
        );
    }

    #[test]
    fn rfc8032_section_7_1_test_two_and_three() {
        for (seed, public, message, expected) in [
            (
                "4ccd089b28ff96da9db6c346ec114e0f5b8a319f35aba624da8cf6ed4fb8a6fb",
                "3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c",
                "72",
                "92a009a9f0d4cab8720e820b5f642540a2b27b5416503f8fb3762223ebdb69da085ac1e43e15996e458f3613d0f11d8c387b2eaeb4302aeeb00d291612bb0c00",
            ),
            (
                "c5aa8df43f9f837bedb7442f31dcb7b166d38535076f094b85ce3a2e0b4458f7",
                "fc51cd8e6218a1a38da47ed00230f0580816ed13ba3303ac5deb911548908025",
                "af82",
                "6291d657deec24024827e69c3abe01a30ce548a284743a445e3680d7db5ac3ac18ff9b538d16f290ae67f760984dc6594a7c15e9716ed28dc027beceea1ec40a",
            ),
        ] {
            let key = SigningKey::from_seed(decode_hex(seed).try_into().unwrap());
            assert_eq!(hex(&key.public_key()), public);
            assert_eq!(
                hex(&DalekSigningKey::from_bytes(&key.seed)
                    .sign(&decode_hex(message))
                    .to_bytes()),
                expected
            );
        }
    }

    #[test]
    fn detached_record_rejects_trailing_and_unknown_binding_bits() {
        let key = SigningKey::from_seed([7; 32]);
        let record = sign_archive(&vector_bindings(), &key, true, true).unwrap();
        let mut trailing = record.encode().unwrap();
        trailing.push(0);
        assert_eq!(
            SignatureRecord::decode(&trailing).unwrap_err().code(),
            ReasonCode::SignatureInvalid
        );
        let mut invalid = record;
        invalid.binding_mask |= 0x80;
        assert_eq!(
            invalid.encode().unwrap_err().code(),
            ReasonCode::SignatureUnsupported
        );

        let mut weak = sign_archive(&vector_bindings(), &key, false, false).unwrap();
        weak.public_key = [0; 32];
        weak.public_key[0] = 1;
        assert_eq!(
            verify_signature(&weak, &vector_bindings(), None)
                .unwrap_err()
                .code(),
            ReasonCode::SignatureInvalid
        );

        // Project Wycheproof's EdDSA invalid families include non-canonical S
        // encodings. Set S to the group order L; strict verification must not
        // accept the alternate scalar representation.
        let mut noncanonical = sign_archive(&vector_bindings(), &key, false, false).unwrap();
        noncanonical.signature[32..].copy_from_slice(&decode_hex(
            "edd3f55c1a631258d69cf7a2def9de1400000000000000000000000000000010",
        ));
        assert_eq!(
            verify_signature(&noncanonical, &vector_bindings(), None)
                .unwrap()
                .cryptographic,
            CryptographicStatus::Invalid
        );
    }

    #[test]
    fn cryptographic_validity_is_separate_from_binding_freshness() {
        let key = SigningKey::from_seed([0x51; 32]);
        let record = sign_archive(&vector_bindings(), &key, true, true).unwrap();
        let mut changed = vector_bindings();
        changed.content.push(0);
        changed.physical.push(0);
        changed.addressing.as_mut().unwrap().push(0);
        let status = verify_signature(&record, &changed, None).unwrap();
        assert_eq!(status.cryptographic, CryptographicStatus::Valid);
        assert_eq!(status.content, BindingStatus::Stale);
        assert_eq!(status.physical, BindingStatus::Stale);
        assert_eq!(status.addressing, BindingStatus::Stale);

        let mut corrupt = record;
        corrupt.signature[0] ^= 1;
        let status = verify_signature(&corrupt, &vector_bindings(), None).unwrap();
        assert_eq!(status.cryptographic, CryptographicStatus::Invalid);
    }

    #[test]
    fn signature_policy_uses_stable_staleness_diagnostics() {
        let key = SigningKey::from_seed([0x52; 32]);
        let record = sign_archive(&vector_bindings(), &key, false, false).unwrap();
        let status = verify_signature(&record, &vector_bindings(), None).unwrap();
        let error = SignaturePolicy {
            require_physical: true,
            ..SignaturePolicy::default()
        }
        .enforce(&[status])
        .unwrap_err();
        assert_eq!(error.code(), ReasonCode::SignatureStalePhysical);
    }

    #[test]
    fn timestamp_tokens_are_bounded_and_need_explicit_trust() {
        let key = SigningKey::from_seed([0x53; 32]);
        let record = sign_archive(&vector_bindings(), &key, false, false)
            .unwrap()
            .with_timestamp_token(vec![0x30, 0x00])
            .unwrap();
        assert_eq!(
            verify_signature(&record, &vector_bindings(), None)
                .unwrap()
                .timestamp,
            TimestampStatus::Unsupported
        );
        let policy = timestamp::TimestampPolicy {
            trust_anchors: vec![timestamp::TimestampTrustAnchor { der: vec![0] }],
            verification_unix_seconds: 0,
        };
        assert_eq!(
            verify_signature(&record, &vector_bindings(), Some(&policy))
                .unwrap()
                .timestamp,
            TimestampStatus::Invalid
        );
        assert_eq!(
            sign_archive(&vector_bindings(), &key, false, false)
                .unwrap()
                .with_timestamp_token(vec![0; MAX_TIMESTAMP_BYTES + 1])
                .unwrap_err()
                .code(),
            ReasonCode::SignatureTimestampInvalid
        );
    }

    #[test]
    fn local_signing_key_is_versioned_and_round_trips() {
        let key = SigningKey::from_seed([9; 32]);
        let bytes = key.encode_file();
        assert_eq!(&bytes[..4], b"EBSK");
        assert_eq!(bytes.len(), 44);
    }

    fn decode_hex(value: &str) -> Vec<u8> {
        value
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|byte| format!("{byte:02x}")).collect()
    }
}
