//! Offline RFC 3161 verification for the narrow v1 profile: SHA-256 imprint
//! and Ed25519 CMS/certificate signatures. No network service is consulted.

use cms::cert::CertificateChoices;
use cms::content_info::ContentInfo;
use cms::signed_data::{SignedData, SignerIdentifier};
use der::asn1::{GeneralizedTime, ObjectIdentifier, OctetString, Uint};
use der::{Decode, Encode, Sequence};
use ed25519_dalek::{Signature, VerifyingKey};
use sha2::{Digest as _, Sha256};
use x509_cert::Certificate;
use x509_cert::ext::Extensions;
use x509_cert::ext::pkix::constraints::BasicConstraints;
use x509_cert::ext::pkix::name::GeneralName;
use x509_cert::ext::pkix::{ExtendedKeyUsage, KeyUsage};

use super::signature::SignatureRecord;
use crate::diagnostics::{Diagnostic, OutcomeClass, ReasonCode, Result};

const OID_SIGNED_DATA: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.113549.1.7.2");
const OID_TST_INFO: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.113549.1.9.16.1.4");
const OID_SHA256: ObjectIdentifier = ObjectIdentifier::new_unwrap("2.16.840.1.101.3.4.2.1");
const OID_ED25519: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.3.101.112");
const OID_CONTENT_TYPE: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.113549.1.9.3");
const OID_MESSAGE_DIGEST: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.113549.1.9.4");
const OID_TIMESTAMPING_EKU: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.3.6.1.5.5.7.3.8");
const MAX_CHAIN_DEPTH: usize = 16;

#[derive(Clone, Debug, Eq, PartialEq, Sequence)]
struct MessageImprint {
    hash_algorithm: spki::AlgorithmIdentifierOwned,
    hashed_message: OctetString,
}

#[derive(Clone, Debug, Eq, PartialEq, Sequence)]
struct Accuracy {
    seconds: Option<u32>,
    #[asn1(context_specific = "0", tag_mode = "IMPLICIT", optional = "true")]
    millis: Option<u16>,
    #[asn1(context_specific = "1", tag_mode = "IMPLICIT", optional = "true")]
    micros: Option<u16>,
}

#[derive(Clone, Debug, Eq, PartialEq, Sequence)]
struct TstInfo {
    version: u32,
    policy: ObjectIdentifier,
    message_imprint: MessageImprint,
    serial_number: Uint,
    gen_time: GeneralizedTime,
    accuracy: Option<Accuracy>,
    #[asn1(default = "Default::default")]
    ordering: bool,
    nonce: Option<Uint>,
    #[asn1(context_specific = "0", tag_mode = "EXPLICIT", optional = "true")]
    tsa: Option<GeneralName>,
    #[asn1(context_specific = "1", tag_mode = "IMPLICIT", optional = "true")]
    extensions: Option<Extensions>,
}

/// Caller-provided DER trust anchor.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimestampTrustAnchor {
    pub der: Vec<u8>,
}

/// Offline trust and wall-clock policy. Certificate validity is evaluated at
/// the token's generation time; generation after this caller time is refused.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimestampPolicy {
    pub trust_anchors: Vec<TimestampTrustAnchor>,
    pub verification_unix_seconds: i64,
}

pub(super) fn verify_timestamp(
    signature_record: &SignatureRecord,
    token: &[u8],
    policy: &TimestampPolicy,
) -> Result<i64> {
    if token.is_empty() || token.len() > 64 << 10 {
        return Err(invalid("RFC 3161 token length is outside v1 bounds"));
    }
    if policy.trust_anchors.is_empty() {
        return Err(invalid(
            "timestamp verification requires a caller trust anchor",
        ));
    }
    let content =
        ContentInfo::from_der(token).map_err(|_| invalid("timestamp CMS DER is malformed"))?;
    if content.content_type != OID_SIGNED_DATA {
        return Err(unsupported("timestamp token is not CMS SignedData"));
    }
    let signed = content
        .content
        .decode_as::<SignedData>()
        .map_err(|_| invalid("timestamp SignedData is malformed"))?;
    if signed.digest_algorithms.len() != 1
        || signed
            .digest_algorithms
            .get(0)
            .is_none_or(|algorithm| !is_algorithm(algorithm, OID_SHA256))
        || signed.encap_content_info.econtent_type != OID_TST_INFO
        || signed.signer_infos.0.len() != 1
    {
        return Err(unsupported(
            "timestamp must use one SHA-256 digest and one id-ct-TSTInfo signer",
        ));
    }
    let econtent = signed
        .encap_content_info
        .econtent
        .as_ref()
        .ok_or_else(|| invalid("timestamp TSTInfo content is absent"))?
        .decode_as::<OctetString>()
        .map_err(|_| invalid("timestamp TSTInfo wrapper is malformed"))?;
    let tst = TstInfo::from_der(econtent.as_bytes())
        .map_err(|_| invalid("timestamp TSTInfo is malformed"))?;
    if tst.version != 1 || !is_algorithm(&tst.message_imprint.hash_algorithm, OID_SHA256) {
        return Err(unsupported(
            "timestamp TSTInfo version or message-imprint algorithm is unsupported",
        ));
    }
    let imprint = Sha256::digest(signature_record.encode_without_timestamp()?);
    if tst.message_imprint.hashed_message.as_bytes() != imprint.as_slice() {
        return Err(invalid(
            "timestamp message imprint does not match SignatureRecord",
        ));
    }
    let gen_time = i64::try_from(tst.gen_time.to_unix_duration().as_secs())
        .map_err(|_| invalid("timestamp generation time exceeds i64"))?;
    if gen_time > policy.verification_unix_seconds {
        return Err(invalid("timestamp generation time is in the future"));
    }

    let signer = signed
        .signer_infos
        .0
        .get(0)
        .expect("validated one timestamp signer");
    if !is_algorithm(&signer.digest_alg, OID_SHA256)
        || !is_algorithm(&signer.signature_algorithm, OID_ED25519)
        || signer.unsigned_attrs.is_some()
    {
        return Err(unsupported(
            "timestamp signer uses an unsupported digest/signature/attribute profile",
        ));
    }
    let attributes = signer
        .signed_attrs
        .as_ref()
        .ok_or_else(|| invalid("timestamp signed attributes are absent"))?;
    validate_signed_attributes(attributes, econtent.as_bytes())?;

    let certificates = signed
        .certificates
        .as_ref()
        .ok_or_else(|| invalid("timestamp certificate chain is absent"))?
        .0
        .iter()
        .filter_map(|choice| match choice {
            CertificateChoices::Certificate(certificate) => Some(certificate.clone()),
            CertificateChoices::Other(_) => None,
        })
        .collect::<Vec<_>>();
    let signer_certificate = certificates
        .iter()
        .find(|certificate| signer_matches(certificate, &signer.sid))
        .ok_or_else(|| invalid("timestamp signer certificate is absent"))?;
    validate_timestamp_eku(signer_certificate)?;
    validate_time(signer_certificate, gen_time)?;
    let signed_attributes = attributes
        .to_der()
        .map_err(|_| invalid("timestamp signed attributes cannot be canonicalized"))?;
    let signature_bytes: [u8; 64] = signer
        .signature
        .as_bytes()
        .try_into()
        .map_err(|_| invalid("timestamp Ed25519 signature length is invalid"))?;
    ed25519_public_key(signer_certificate)?
        .verify_strict(&signed_attributes, &Signature::from_bytes(&signature_bytes))
        .map_err(|_| invalid("timestamp CMS Ed25519 signature is invalid"))?;
    validate_chain(
        signer_certificate,
        &certificates,
        &policy.trust_anchors,
        gen_time,
    )?;
    Ok(gen_time)
}

fn validate_signed_attributes(
    attributes: &cms::signed_data::SignedAttributes,
    econtent: &[u8],
) -> Result<()> {
    let mut content_type_seen = false;
    let mut message_digest_seen = false;
    for attribute in attributes.iter() {
        if attribute.oid == OID_CONTENT_TYPE {
            if content_type_seen || attribute.values.len() != 1 {
                return Err(invalid("timestamp content-type attribute is duplicate"));
            }
            let value = attribute
                .values
                .get(0)
                .expect("validated one value")
                .decode_as::<ObjectIdentifier>()
                .map_err(|_| invalid("timestamp content-type attribute is malformed"))?;
            if value != OID_TST_INFO {
                return Err(invalid("timestamp content-type disagrees with TSTInfo"));
            }
            content_type_seen = true;
        } else if attribute.oid == OID_MESSAGE_DIGEST {
            if message_digest_seen || attribute.values.len() != 1 {
                return Err(invalid("timestamp message-digest attribute is duplicate"));
            }
            let value = attribute
                .values
                .get(0)
                .expect("validated one value")
                .decode_as::<OctetString>()
                .map_err(|_| invalid("timestamp message-digest attribute is malformed"))?;
            if value.as_bytes() != Sha256::digest(econtent).as_slice() {
                return Err(invalid("timestamp CMS message digest is invalid"));
            }
            message_digest_seen = true;
        }
    }
    if !content_type_seen || !message_digest_seen {
        return Err(invalid("timestamp required signed attributes are absent"));
    }
    Ok(())
}

fn validate_chain(
    signer: &Certificate,
    embedded: &[Certificate],
    anchors: &[TimestampTrustAnchor],
    time: i64,
) -> Result<()> {
    let anchors = anchors
        .iter()
        .map(|anchor| {
            Certificate::from_der(&anchor.der)
                .map(|certificate| (anchor.der.clone(), certificate))
                .map_err(|_| invalid("timestamp trust anchor is malformed DER"))
        })
        .collect::<Result<Vec<_>>>()?;
    let mut current = signer.clone();
    let mut seen = Vec::<Vec<u8>>::new();
    for _ in 0..MAX_CHAIN_DEPTH {
        let current_der = current
            .to_der()
            .map_err(|_| invalid("timestamp certificate is not canonical DER"))?;
        if anchors.iter().any(|(der, _)| der == &current_der) {
            return Ok(());
        }
        if seen.iter().any(|value| value == &current_der) {
            return Err(invalid("timestamp certificate chain contains a cycle"));
        }
        seen.push(current_der);
        let issuer = embedded
            .iter()
            .chain(anchors.iter().map(|(_, certificate)| certificate))
            .find(|candidate| {
                candidate.tbs_certificate().subject() == current.tbs_certificate().issuer()
            })
            .ok_or_else(|| invalid("timestamp chain does not reach a trust anchor"))?;
        validate_ca(issuer)?;
        validate_time(issuer, time)?;
        verify_certificate(&current, issuer)?;
        current = issuer.clone();
    }
    Err(invalid(
        "timestamp certificate chain exceeds v1 depth bound",
    ))
}

fn verify_certificate(certificate: &Certificate, issuer: &Certificate) -> Result<()> {
    if !is_algorithm(certificate.signature_algorithm(), OID_ED25519)
        || !is_algorithm(certificate.tbs_certificate().signature(), OID_ED25519)
    {
        return Err(unsupported(
            "timestamp chain contains a non-Ed25519 certificate signature",
        ));
    }
    let signature: [u8; 64] = certificate
        .signature()
        .raw_bytes()
        .try_into()
        .map_err(|_| invalid("timestamp certificate signature length is invalid"))?;
    let tbs = certificate
        .tbs_certificate()
        .to_der()
        .map_err(|_| invalid("timestamp certificate TBS cannot be encoded"))?;
    ed25519_public_key(issuer)?
        .verify_strict(&tbs, &Signature::from_bytes(&signature))
        .map_err(|_| invalid("timestamp certificate-chain signature is invalid"))
}

fn ed25519_public_key(certificate: &Certificate) -> Result<VerifyingKey> {
    let spki = certificate.tbs_certificate().subject_public_key_info();
    if !is_algorithm(&spki.algorithm, OID_ED25519) {
        return Err(unsupported("timestamp certificate key is not Ed25519"));
    }
    let bytes: [u8; 32] = spki
        .subject_public_key
        .raw_bytes()
        .try_into()
        .map_err(|_| invalid("timestamp Ed25519 public-key length is invalid"))?;
    let key = VerifyingKey::from_bytes(&bytes)
        .map_err(|_| invalid("timestamp Ed25519 public key is malformed"))?;
    if key.is_weak() {
        return Err(invalid("timestamp Ed25519 public key has small order"));
    }
    Ok(key)
}

fn signer_matches(certificate: &Certificate, sid: &SignerIdentifier) -> bool {
    match sid {
        SignerIdentifier::IssuerAndSerialNumber(value) => {
            certificate.tbs_certificate().issuer() == &value.issuer
                && certificate.tbs_certificate().serial_number() == &value.serial_number
        }
        SignerIdentifier::SubjectKeyIdentifier(value) => certificate
            .tbs_certificate()
            .get_extension::<x509_cert::ext::pkix::SubjectKeyIdentifier>()
            .ok()
            .flatten()
            .is_some_and(|(_, identifier)| identifier == *value),
    }
}

fn validate_timestamp_eku(certificate: &Certificate) -> Result<()> {
    let (critical, eku) = certificate
        .tbs_certificate()
        .get_extension::<ExtendedKeyUsage>()
        .map_err(|_| invalid("timestamp ExtendedKeyUsage is malformed"))?
        .ok_or_else(|| invalid("timestamp signer lacks ExtendedKeyUsage"))?;
    if !critical || eku.0.as_slice() != [OID_TIMESTAMPING_EKU] {
        return Err(invalid(
            "timestamp ExtendedKeyUsage must be critical and solely timeStamping",
        ));
    }
    let usage = certificate
        .tbs_certificate()
        .get_extension::<KeyUsage>()
        .map_err(|_| invalid("timestamp KeyUsage is malformed"))?;
    if usage.is_some_and(|(_, value)| !value.digital_signature()) {
        return Err(invalid("timestamp KeyUsage forbids digitalSignature"));
    }
    Ok(())
}

fn validate_ca(certificate: &Certificate) -> Result<()> {
    let (_, constraints) = certificate
        .tbs_certificate()
        .get_extension::<BasicConstraints>()
        .map_err(|_| invalid("timestamp CA BasicConstraints is malformed"))?
        .ok_or_else(|| invalid("timestamp issuer lacks BasicConstraints"))?;
    if !constraints.ca {
        return Err(invalid("timestamp issuer is not a CA"));
    }
    let usage = certificate
        .tbs_certificate()
        .get_extension::<KeyUsage>()
        .map_err(|_| invalid("timestamp CA KeyUsage is malformed"))?;
    if usage.is_some_and(|(_, value)| !value.key_cert_sign()) {
        return Err(invalid("timestamp issuer KeyUsage forbids keyCertSign"));
    }
    Ok(())
}

fn validate_time(certificate: &Certificate, time: i64) -> Result<()> {
    let validity = certificate.tbs_certificate().validity();
    let not_before = i64::try_from(validity.not_before.to_unix_duration().as_secs())
        .map_err(|_| invalid("certificate notBefore exceeds i64"))?;
    let not_after = i64::try_from(validity.not_after.to_unix_duration().as_secs())
        .map_err(|_| invalid("certificate notAfter exceeds i64"))?;
    if time < not_before || time > not_after {
        return Err(invalid(
            "timestamp generation time is outside certificate validity",
        ));
    }
    Ok(())
}

fn is_algorithm(value: &spki::AlgorithmIdentifierOwned, oid: ObjectIdentifier) -> bool {
    value.oid == oid && value.parameters.is_none()
}

fn invalid(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::Corrupt,
        ReasonCode::SignatureTimestampInvalid,
        detail,
    )
}

fn unsupported(detail: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        OutcomeClass::Unsupported,
        ReasonCode::SignatureTimestampUnsupported,
        detail,
    )
}

#[cfg(test)]
mod tests {
    use std::str::FromStr as _;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use cms::cert::{CertificateChoices, IssuerAndSerialNumber};
    use cms::content_info::CmsVersion;
    use cms::signed_data::{
        CertificateSet, DigestAlgorithmIdentifiers, EncapsulatedContentInfo, SignerInfo,
        SignerInfos,
    };
    use der::asn1::{ObjectIdentifier, OctetString, SetOfVec};
    use der::{Any, Decode, Encode, Tag};
    use ed25519_dalek::{Signer as _, SigningKey};
    use x509_cert::attr::Attribute;
    use x509_cert::builder::profile::BuilderProfile;
    use x509_cert::builder::{Builder, CertificateBuilder};
    use x509_cert::certificate::TbsCertificate;
    use x509_cert::ext::Extension;
    use x509_cert::name::Name;
    use x509_cert::serial_number::SerialNumber;
    use x509_cert::time::Validity;
    use x509_cert::{SubjectPublicKeyInfo, spki::SubjectPublicKeyInfoRef};

    use super::*;
    use crate::crypto::{CurrentBindings, sign_archive};

    #[derive(Clone)]
    struct TimestampProfile(Name);

    impl BuilderProfile for TimestampProfile {
        fn get_issuer(&self, _subject: &Name) -> Name {
            self.0.clone()
        }

        fn get_subject(&self) -> Name {
            self.0.clone()
        }

        fn build_extensions(
            &self,
            _spk: SubjectPublicKeyInfoRef<'_>,
            _issuer_spk: SubjectPublicKeyInfoRef<'_>,
            _tbs: &TbsCertificate,
        ) -> x509_cert::builder::Result<Vec<Extension>> {
            let eku = ExtendedKeyUsage(vec![OID_TIMESTAMPING_EKU]);
            Ok(vec![Extension {
                extn_id: ObjectIdentifier::new_unwrap("2.5.29.37"),
                critical: true,
                extn_value: OctetString::new(eku.to_der()?)?,
            }])
        }
    }

    #[test]
    fn generated_ed25519_rfc3161_token_verifies_offline() {
        let record = signature_record();
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let tsa = SigningKey::from_bytes(&[0x61; 32]);
        let name = Name::from_str("CN=Entrybound test TSA").unwrap();
        let spki = SubjectPublicKeyInfo::from_key(&tsa.verifying_key()).unwrap();
        let certificate = CertificateBuilder::new(
            TimestampProfile(name),
            SerialNumber::from(7_u32),
            Validity::from_now(Duration::from_secs(3_600)).unwrap(),
            spki,
        )
        .unwrap()
        .build::<_, ed25519_dalek::Signature>(&tsa)
        .unwrap();

        let imprint = Sha256::digest(record.encode_without_timestamp().unwrap());
        let tst = TstInfo {
            version: 1,
            policy: ObjectIdentifier::new_unwrap("1.3.6.1.4.1.57264.1"),
            message_imprint: MessageImprint {
                hash_algorithm: algorithm(OID_SHA256),
                hashed_message: OctetString::new(imprint.as_slice()).unwrap(),
            },
            serial_number: Uint::new(&[1]).unwrap(),
            gen_time: GeneralizedTime::from_unix_duration(now).unwrap(),
            accuracy: None,
            ordering: false,
            nonce: None,
            tsa: None,
            extensions: None,
        };
        let tst_der = tst.to_der().unwrap();
        let content = EncapsulatedContentInfo {
            econtent_type: OID_TST_INFO,
            econtent: Some(Any::new(Tag::OctetString, tst_der.clone()).unwrap()),
        };
        let attributes = x509_cert::attr::Attributes::try_from(vec![
            attribute_oid(OID_CONTENT_TYPE, OID_TST_INFO),
            attribute_octets(OID_MESSAGE_DIGEST, &Sha256::digest(&tst_der)),
        ])
        .unwrap();
        let signature = tsa.sign(&attributes.to_der().unwrap());
        let signer = SignerInfo {
            version: CmsVersion::V1,
            sid: SignerIdentifier::IssuerAndSerialNumber(IssuerAndSerialNumber {
                issuer: certificate.tbs_certificate().issuer().clone(),
                serial_number: certificate.tbs_certificate().serial_number().clone(),
            }),
            digest_alg: algorithm(OID_SHA256),
            signed_attrs: Some(attributes),
            signature_algorithm: algorithm(OID_ED25519),
            signature: OctetString::new(signature.to_bytes()).unwrap(),
            unsigned_attrs: None,
        };
        let signed = SignedData {
            version: CmsVersion::V3,
            digest_algorithms: DigestAlgorithmIdentifiers::try_from(vec![algorithm(OID_SHA256)])
                .unwrap(),
            encap_content_info: content,
            certificates: Some(
                CertificateSet::try_from(vec![CertificateChoices::Certificate(
                    certificate.clone(),
                )])
                .unwrap(),
            ),
            crls: None,
            signer_infos: SignerInfos::try_from(vec![signer]).unwrap(),
        };
        let signed_der = signed.to_der().unwrap();
        let token = ContentInfo {
            content_type: OID_SIGNED_DATA,
            content: Any::from_der(&signed_der).unwrap(),
        }
        .to_der()
        .unwrap();
        let result = verify_timestamp(
            &record,
            &token,
            &TimestampPolicy {
                trust_anchors: vec![TimestampTrustAnchor {
                    der: certificate.to_der().unwrap(),
                }],
                verification_unix_seconds: i64::try_from(now.as_secs() + 1).unwrap(),
            },
        )
        .unwrap();
        assert_eq!(result, i64::try_from(now.as_secs()).unwrap());

        let mut changed_record = record.clone();
        changed_record.signature[0] ^= 1;
        assert_eq!(
            verify_timestamp(
                &changed_record,
                &token,
                &TimestampPolicy {
                    trust_anchors: vec![TimestampTrustAnchor {
                        der: certificate.to_der().unwrap(),
                    }],
                    verification_unix_seconds: i64::try_from(now.as_secs() + 1).unwrap(),
                },
            )
            .unwrap_err()
            .code(),
            ReasonCode::SignatureTimestampInvalid
        );
        assert_eq!(
            verify_timestamp(
                &record,
                &token,
                &TimestampPolicy {
                    trust_anchors: vec![TimestampTrustAnchor { der: vec![0] }],
                    verification_unix_seconds: i64::try_from(now.as_secs() + 1).unwrap(),
                },
            )
            .unwrap_err()
            .code(),
            ReasonCode::SignatureTimestampInvalid
        );
    }

    fn algorithm(oid: ObjectIdentifier) -> spki::AlgorithmIdentifierOwned {
        spki::AlgorithmIdentifierOwned {
            oid,
            parameters: None,
        }
    }

    fn attribute_oid(oid: ObjectIdentifier, value: ObjectIdentifier) -> Attribute {
        let mut values = SetOfVec::new();
        values
            .insert(Any::from_der(&value.to_der().unwrap()).unwrap())
            .unwrap();
        Attribute { oid, values }
    }

    fn attribute_octets(oid: ObjectIdentifier, value: &[u8]) -> Attribute {
        let octets = OctetString::new(value).unwrap();
        let mut values = SetOfVec::new();
        values
            .insert(Any::from_der(&octets.to_der().unwrap()).unwrap())
            .unwrap();
        Attribute { oid, values }
    }

    fn signature_record() -> SignatureRecord {
        let bindings = CurrentBindings {
            content: crate::crypto::wire::t1(
                "entrybound/signature-content/v1",
                &[
                    &[0; 32],
                    &[1; 32],
                    b"identity/v1",
                    b"ecf/bootstrap-v1",
                    &[0, 0],
                    &[0, 1],
                ],
            )
            .unwrap(),
            physical: crate::crypto::wire::t1("entrybound/signature-physical/v1", &[&[2; 32]])
                .unwrap(),
            addressing: Some(
                crate::crypto::wire::t1(
                    "entrybound/signature-addressing/v1",
                    &[&[0, 1], &[3; 32], &[4; 32], &[5; 32]],
                )
                .unwrap(),
            ),
        };
        sign_archive(
            &bindings,
            &crate::crypto::SigningKey::from_seed([4; 32]),
            true,
            true,
        )
        .unwrap()
    }
}
