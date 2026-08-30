//! Non-production generator for crypto wire specification vectors.
//!
//! Every input in this program is public and fixed. Do not use this code as an
//! encryption API or copy its deterministic inputs into production code.

use aes_gcm_siv::{
    Aes256GcmSiv, Nonce,
    aead::{Aead, KeyInit, Payload},
};
use argon2::{Algorithm, Argon2, Params, Version};
use hkdf::Hkdf;
use sha2::{Digest, Sha256};

const FORMAT_NAMESPACE: &[u8] = b"ecf/bootstrap-v1";
const XWING_PARAMETERS: &[u8] = b"xwing-mlkem768-x25519-sha3-256/draft-10";

fn t1(label: &str, fields: &[&[u8]]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&u16::try_from(label.len()).unwrap().to_be_bytes());
    out.extend_from_slice(label.as_bytes());
    out.extend_from_slice(&u16::try_from(fields.len()).unwrap().to_be_bytes());
    for (index, value) in fields.iter().enumerate() {
        out.extend_from_slice(&u16::try_from(index + 1).unwrap().to_be_bytes());
        out.extend_from_slice(&u64::try_from(value.len()).unwrap().to_be_bytes());
        out.extend_from_slice(value);
    }
    out
}

fn sha256(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

fn hkdf_extract(salt: &[u8], ikm: &[u8]) -> [u8; 32] {
    let (prk, _) = Hkdf::<Sha256>::extract(Some(salt), ikm);
    prk.into()
}

fn hkdf_expand(prk: &[u8; 32], info: &[u8]) -> [u8; 32] {
    let hkdf = Hkdf::<Sha256>::from_prk(prk).unwrap();
    let mut out = [0_u8; 32];
    hkdf.expand(info, &mut out).unwrap();
    out
}

fn method_context(
    stanza_type: u16,
    protection_class: u8,
    stanza_id: &[u8; 16],
    parameters: &[u8],
    encapsulation: &[u8],
) -> Vec<u8> {
    let stanza_version = 1_u16.to_be_bytes();
    let stanza_type = stanza_type.to_be_bytes();
    let protection_class = [protection_class];
    let hint = [0_u8; 16];
    t1(
        "entrybound/recipient-method-context/v1",
        &[
            &stanza_version,
            &stanza_type,
            &protection_class,
            stanza_id,
            &hint,
            parameters,
            encapsulation,
        ],
    )
}

fn wrap_key(
    archive_id: &[u8; 32],
    method_secret: &[u8; 32],
    stanza_type: u16,
    protection_class: u8,
    stanza_id: &[u8; 16],
    method_context_digest: &[u8; 32],
) -> ([u8; 32], [u8; 32]) {
    let prk = hkdf_extract(archive_id, method_secret);
    let suite = 1_u16.to_be_bytes();
    let stanza_type = stanza_type.to_be_bytes();
    let protection_class = [protection_class];
    let info = t1(
        "entrybound/recipient-wrap-key/v1",
        &[
            &suite,
            &stanza_type,
            &protection_class,
            stanza_id,
            method_context_digest,
        ],
    );
    (prk, hkdf_expand(&prk, &info))
}

fn wrap_ad(
    archive_id: &[u8; 32],
    stanza_type: u16,
    protection_class: u8,
    stanza_id: &[u8; 16],
    parameters: &[u8],
    encapsulation: &[u8],
    nonce: &[u8; 12],
) -> Vec<u8> {
    let format_major = 0_u16.to_be_bytes();
    let format_minor = 1_u16.to_be_bytes();
    let crypto_version = 1_u16.to_be_bytes();
    let suite = 1_u16.to_be_bytes();
    let stanza_version = 1_u16.to_be_bytes();
    let stanza_type = stanza_type.to_be_bytes();
    let protection_class = [protection_class];
    let hint = [0_u8; 16];
    t1(
        "entrybound/recipient-wrap-ad/v1",
        &[
            FORMAT_NAMESPACE,
            &format_major,
            &format_minor,
            &crypto_version,
            &suite,
            archive_id,
            &stanza_version,
            &stanza_type,
            &protection_class,
            stanza_id,
            &hint,
            parameters,
            encapsulation,
            nonce,
        ],
    )
}

fn wrap_afk(key: &[u8; 32], nonce: &[u8; 12], ad: &[u8], afk: &[u8; 32]) -> Vec<u8> {
    let nonce = Nonce::from(*nonce);
    Aes256GcmSiv::new_from_slice(key)
        .unwrap()
        .encrypt(&nonce, Payload { msg: afk, aad: ad })
        .unwrap()
}

fn a2id_parameters(salt: &[u8; 16]) -> Vec<u8> {
    let mut out = b"A2ID".to_vec();
    out.extend_from_slice(&19_u32.to_be_bytes());
    out.extend_from_slice(&262_144_u32.to_be_bytes());
    out.extend_from_slice(&3_u32.to_be_bytes());
    out.extend_from_slice(&4_u32.to_be_bytes());
    out.extend_from_slice(salt);
    out
}

fn field(tag: u16, field_type: u8, value: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&tag.to_be_bytes());
    out.push(field_type);
    out.push(0);
    out.extend_from_slice(&u64::try_from(value.len()).unwrap().to_be_bytes());
    out.extend_from_slice(value);
    out
}

fn recipient_directory_entry(id: [u8; 16], fingerprint: [u8; 32], label: &str) -> Vec<u8> {
    let mut payload = Vec::new();
    payload.extend_from_slice(&field(1, 7, &id));
    payload.extend_from_slice(&field(2, 2, &1_u16.to_be_bytes()));
    payload.extend_from_slice(&field(3, 7, &fingerprint));
    payload.extend_from_slice(&field(4, 8, label.as_bytes()));
    let mut out = Vec::new();
    out.extend_from_slice(&22_u16.to_be_bytes());
    out.extend_from_slice(&1_u16.to_be_bytes());
    out.extend_from_slice(&0_u32.to_be_bytes());
    out.extend_from_slice(&u64::try_from(payload.len()).unwrap().to_be_bytes());
    out.extend_from_slice(&payload);
    out
}

fn sequence(kind: u16, items: &[Vec<u8>]) -> Vec<u8> {
    let mut out = b"EBCS".to_vec();
    out.extend_from_slice(&1_u16.to_be_bytes());
    out.extend_from_slice(&kind.to_be_bytes());
    out.extend_from_slice(&0_u32.to_be_bytes());
    out.extend_from_slice(&u64::try_from(items.len()).unwrap().to_be_bytes());
    for item in items {
        out.extend_from_slice(&u64::try_from(item.len()).unwrap().to_be_bytes());
        out.extend_from_slice(item);
    }
    out
}

fn private_object(kind: u16, payload: &[u8]) -> Vec<u8> {
    let mut out = b"EBPO".to_vec();
    out.extend_from_slice(&1_u16.to_be_bytes());
    out.extend_from_slice(&kind.to_be_bytes());
    out.extend_from_slice(&0_u32.to_be_bytes());
    out.extend_from_slice(payload);
    out
}

fn emit(name: &str, value: &[u8]) {
    println!("{name}={}", hex(value));
}

fn emit_sha(name: &str, value: &[u8]) {
    emit(name, &sha256(value));
}

fn hex(value: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(value.len() * 2);
    for byte in value {
        out.push(char::from(HEX[usize::from(byte >> 4)]));
        out.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    out
}

fn main() {
    let archive_id: [u8; 32] = core::array::from_fn(|i| u8::try_from(i).unwrap());
    let afk: [u8; 32] = core::array::from_fn(|i| 0x20 + u8::try_from(i).unwrap());

    let v5_secret: [u8; 32] = core::array::from_fn(|i| 0xb0 + u8::try_from(i).unwrap());
    let v5_id: [u8; 16] = core::array::from_fn(|i| 0x40 + u8::try_from(i).unwrap());
    let v5_nonce: [u8; 12] = core::array::from_fn(|i| 0x50 + u8::try_from(i).unwrap());
    let v5_encapsulation: Vec<u8> = (0..1120).map(|i| (i % 256) as u8).collect();
    let v5_context = method_context(1, 1, &v5_id, XWING_PARAMETERS, &v5_encapsulation);
    let v5_context_digest = sha256(&v5_context);
    let (v5_prk, v5_key) = wrap_key(&archive_id, &v5_secret, 1, 1, &v5_id, &v5_context_digest);
    let v5_ad = wrap_ad(
        &archive_id,
        1,
        1,
        &v5_id,
        XWING_PARAMETERS,
        &v5_encapsulation,
        &v5_nonce,
    );
    emit("V5_METHOD_CONTEXT", &v5_context);
    emit("V5_METHOD_CONTEXT_SHA256", &v5_context_digest);
    emit_sha("V5_ENCAPSULATION_SHA256", &v5_encapsulation);
    emit("V5_WRAP_PRK", &v5_prk);
    emit("V5_WRAP_KEY", &v5_key);
    emit("V5_WRAP_NONCE", &v5_nonce);
    emit("V5_AFK", &afk);
    emit("V5_WRAP_AD", &v5_ad);
    emit_sha("V5_WRAP_AD_SHA256", &v5_ad);
    emit(
        "V5_WRAPPED_AFK",
        &wrap_afk(&v5_key, &v5_nonce, &v5_ad, &afk),
    );

    let v6_salt: [u8; 16] = core::array::from_fn(|i| 0x60 + u8::try_from(i).unwrap());
    let v6_parameters = a2id_parameters(&v6_salt);
    let params = Params::new(262_144, 3, 4, Some(32)).unwrap();
    let mut v6_secret = [0_u8; 32];
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
        .hash_password_into(b"correct horse battery staple", &v6_salt, &mut v6_secret)
        .unwrap();
    let v6_id: [u8; 16] = core::array::from_fn(|i| 0x70 + u8::try_from(i).unwrap());
    let v6_nonce: [u8; 12] = core::array::from_fn(|i| 0x80 + u8::try_from(i).unwrap());
    let v6_context = method_context(2, 2, &v6_id, &v6_parameters, &[]);
    let v6_context_digest = sha256(&v6_context);
    let (v6_prk, v6_key) = wrap_key(&archive_id, &v6_secret, 2, 2, &v6_id, &v6_context_digest);
    let v6_ad = wrap_ad(&archive_id, 2, 2, &v6_id, &v6_parameters, &[], &v6_nonce);
    emit("V6_A2ID", &v6_parameters);
    emit("V6_ARGON2ID_OUTPUT", &v6_secret);
    emit("V6_METHOD_CONTEXT", &v6_context);
    emit("V6_METHOD_CONTEXT_SHA256", &v6_context_digest);
    emit("V6_WRAP_PRK", &v6_prk);
    emit("V6_WRAP_KEY", &v6_key);
    emit("V6_WRAP_NONCE", &v6_nonce);
    emit("V6_AFK", &afk);
    emit("V6_WRAP_AD", &v6_ad);
    emit_sha("V6_WRAP_AD_SHA256", &v6_ad);
    emit(
        "V6_WRAPPED_AFK",
        &wrap_afk(&v6_key, &v6_nonce, &v6_ad, &afk),
    );

    let entry_a = recipient_directory_entry(
        core::array::from_fn(|i| u8::try_from(i).unwrap()),
        core::array::from_fn(|i| 0x20 + u8::try_from(i).unwrap()),
        "alice",
    );
    let entry_b = recipient_directory_entry(
        core::array::from_fn(|i| 0x10 + u8::try_from(i).unwrap()),
        core::array::from_fn(|i| 0x40 + u8::try_from(i).unwrap()),
        "",
    );
    let empty = sequence(1, &[]);
    let one = sequence(8, std::slice::from_ref(&entry_a));
    let multi = sequence(8, &[entry_a.clone(), entry_b.clone()]);
    let private = private_object(3, &multi);
    emit("S1_EMPTY", &empty);
    emit_sha("S1_EMPTY_SHA256", &empty);
    emit("S2_ONE", &one);
    emit_sha("S2_ONE_SHA256", &one);
    emit("S3_MULTI", &multi);
    emit_sha("S3_MULTI_SHA256", &multi);
    emit("S4_PRIVATE_CONTROL_OBJECT", &private);
    emit_sha("S4_PRIVATE_CONTROL_OBJECT_SHA256", &private);
    let reversed = sequence(8, &[entry_b.clone(), entry_a.clone()]);
    let duplicate = sequence(8, &[entry_a.clone(), entry_a]);
    let mut truncated = one;
    truncated.pop();
    emit("S5_OUT_OF_ORDER_INVALID", &reversed);
    emit_sha("S5_OUT_OF_ORDER_INVALID_SHA256", &reversed);
    emit("S6_DUPLICATE_INVALID", &duplicate);
    emit_sha("S6_DUPLICATE_INVALID_SHA256", &duplicate);
    emit("S7_TRUNCATED_INVALID", &truncated);
    emit_sha("S7_TRUNCATED_INVALID_SHA256", &truncated);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corrected_wrap_and_sequence_vectors_are_stable() {
        let archive_id: [u8; 32] = core::array::from_fn(|i| u8::try_from(i).unwrap());
        let afk: [u8; 32] = core::array::from_fn(|i| 0x20 + u8::try_from(i).unwrap());

        let v5_secret: [u8; 32] = core::array::from_fn(|i| 0xb0 + u8::try_from(i).unwrap());
        let v5_id: [u8; 16] = core::array::from_fn(|i| 0x40 + u8::try_from(i).unwrap());
        let v5_nonce: [u8; 12] = core::array::from_fn(|i| 0x50 + u8::try_from(i).unwrap());
        let encapsulation: Vec<u8> = (0..1120).map(|i| (i % 256) as u8).collect();
        let context = method_context(1, 1, &v5_id, XWING_PARAMETERS, &encapsulation);
        let digest = sha256(&context);
        let (_, key) = wrap_key(&archive_id, &v5_secret, 1, 1, &v5_id, &digest);
        let ad = wrap_ad(
            &archive_id,
            1,
            1,
            &v5_id,
            XWING_PARAMETERS,
            &encapsulation,
            &v5_nonce,
        );
        assert_eq!(
            hex(&sha256(&ad)),
            "12334742b8d3e5457ae34a7c72ae30c126dd506587a681f6a24b5c6c2fe171a0"
        );
        assert_eq!(
            hex(&wrap_afk(&key, &v5_nonce, &ad, &afk)),
            "db49535140afd435d92500c4eaf6e77f72bcf0fc469aa36c76775a44503ba19d192b1705f345960f1a72c4cec9541603"
        );

        let v6_salt: [u8; 16] = core::array::from_fn(|i| 0x60 + u8::try_from(i).unwrap());
        let parameters = a2id_parameters(&v6_salt);
        let mut secret = [0_u8; 32];
        Argon2::new(
            Algorithm::Argon2id,
            Version::V0x13,
            Params::new(262_144, 3, 4, Some(32)).unwrap(),
        )
        .hash_password_into(b"correct horse battery staple", &v6_salt, &mut secret)
        .unwrap();
        let v6_id: [u8; 16] = core::array::from_fn(|i| 0x70 + u8::try_from(i).unwrap());
        let v6_nonce: [u8; 12] = core::array::from_fn(|i| 0x80 + u8::try_from(i).unwrap());
        let digest = sha256(&method_context(2, 2, &v6_id, &parameters, &[]));
        let (_, key) = wrap_key(&archive_id, &secret, 2, 2, &v6_id, &digest);
        let ad = wrap_ad(&archive_id, 2, 2, &v6_id, &parameters, &[], &v6_nonce);
        assert_eq!(
            hex(&sha256(&ad)),
            "56e84a074663fc268c5f5298cbdad6f7cc84731814c6113e136f484588c105df"
        );
        assert_eq!(
            hex(&wrap_afk(&key, &v6_nonce, &ad, &afk)),
            "e183d98d5694ee3dc723e9d43cdd56d92889035d3f7c11e9492236f43115fe20a53c6725b498e1288e8fb12aae7ff573"
        );

        let empty = sequence(1, &[]);
        assert_eq!(
            hex(&sha256(&empty)),
            "6d025124515cac937fc17001ee46b15c11ee51ace780180438b4fb65f5e1666f"
        );
    }
}
