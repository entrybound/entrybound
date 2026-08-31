use aes_gcm_siv::{
    Aes256GcmSiv, Nonce,
    aead::{Aead, KeyInit, Payload},
};
use argon2::{Algorithm, Argon2, ParamsBuilder, Version};
use hkdf::Hkdf;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use x_wing::{
    DecapsulationKey,
    kem::{Decapsulate as _, Decapsulator as _, KeyExport as _},
};

fn hex(value: &str) -> Vec<u8> {
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

#[test]
fn aes_256_gcm_siv_rfc_8452_vector() {
    let key = hex("0100000000000000000000000000000000000000000000000000000000000000");
    let nonce: [u8; 12] = hex("030000000000000000000000").try_into().unwrap();
    let output = Aes256GcmSiv::new_from_slice(&key)
        .unwrap()
        .encrypt(&Nonce::from(nonce), Payload { msg: &[], aad: &[] })
        .unwrap();
    assert_eq!(output, hex("07f5f4169bbf55a8400cd47ea6fd400f"));
}

#[test]
fn hkdf_sha256_rfc_5869_case_1() {
    let ikm = vec![0x0b; 22];
    let salt = hex("000102030405060708090a0b0c");
    let info = hex("f0f1f2f3f4f5f6f7f8f9");
    let (prk, hkdf) = Hkdf::<Sha256>::extract(Some(&salt), &ikm);
    assert_eq!(
        prk.as_slice(),
        hex("077709362c2e32df0ddc3f0dc47bba6390b6c73bb50f9c3122ec844ad7c2b3e5")
    );
    let mut okm = [0_u8; 42];
    hkdf.expand(&info, &mut okm).unwrap();
    assert_eq!(
        okm.as_slice(),
        hex("3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf34007208d5b887185865")
    );
}

#[test]
fn hmac_sha256_rfc_4231_case_1() {
    let mut mac = Hmac::<Sha256>::new_from_slice(&[0x0b; 20]).unwrap();
    mac.update(b"Hi There");
    assert_eq!(
        mac.finalize().into_bytes().as_slice(),
        hex("b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7")
    );
}

#[test]
fn argon2id_rfc_9106_vector() {
    let mut builder = ParamsBuilder::new();
    builder
        .m_cost(32)
        .t_cost(3)
        .p_cost(4)
        .output_len(32)
        .data(argon2::AssociatedData::new(&[0x04; 12]).unwrap());
    let params = builder.build().unwrap();
    let mut output = [0_u8; 32];
    Argon2::new_with_secret(&[0x03; 8], Algorithm::Argon2id, Version::V0x13, params)
        .unwrap()
        .hash_password_into(&[0x01; 32], &[0x02; 16], &mut output)
        .unwrap();
    assert_eq!(
        output.as_slice(),
        hex("0d640df58d78766c08c037a34a8b53c9d01ef0452d75b65eb52520e96b01e659")
    );
}

#[test]
fn every_authoritative_xwing_draft10_kat() {
    let source = include_str!("data/xwing-draft10.json");
    let seeds = json_hex_fields(source, "seed");
    let eseeds = json_hex_fields(source, "eseed");
    let secrets = json_hex_fields(source, "ss");
    let public_keys = json_hex_fields(source, "pk");
    let ciphertexts = json_hex_fields(source, "ct");
    assert_eq!(seeds.len(), 3);
    for index in 0..seeds.len() {
        let seed: [u8; 32] = seeds[index].as_slice().try_into().unwrap();
        let secret_key = DecapsulationKey::from(seed);
        let public_key = secret_key.encapsulation_key();
        assert_eq!(public_key.to_bytes().as_slice(), public_keys[index]);
        let randomness =
            ml_kem::array::ArrayN::<u8, 64>::try_from(eseeds[index].as_slice()).unwrap();
        let (ciphertext, shared) = public_key.encapsulate_deterministic(&randomness);
        assert_eq!(ciphertext.as_slice(), ciphertexts[index]);
        assert_eq!(shared.as_slice(), secrets[index]);
        assert_eq!(
            secret_key.decapsulate(&ciphertext).as_slice(),
            secrets[index]
        );
    }
}

fn json_hex_fields(source: &str, key: &str) -> Vec<Vec<u8>> {
    let prefix = format!("\"{key}\": \"");
    source
        .lines()
        .filter_map(|line| {
            let value = line.trim().strip_prefix(&prefix)?;
            let value = value.strip_suffix(',').unwrap_or(value);
            Some(hex(value.strip_suffix('"').unwrap()))
        })
        .collect()
}
