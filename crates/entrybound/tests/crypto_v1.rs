use std::path::PathBuf;

use entrybound::archive::{PackOptions, plan_directory};
use entrybound::crypto::{
    BoundaryMode, CryptoPolicy, EncryptedOpenOptions, EncryptedWriteOptions,
    FEATURE_PRIVATE_RESOURCE_DECLARATION_V1, PaddingMode, Unlock, XWingIdentity, XWingRecipient,
    encrypt_archive, inspect_encrypted, open_encrypted, pack_directory_encrypted,
};
use entrybound::diagnostics::ReasonCode;
use entrybound::ecf::{WriteOptions, encode};
use sha2::{Digest as _, Sha256};

struct Fixture {
    root: PathBuf,
    source: PathBuf,
}

impl Fixture {
    fn new(name: &str) -> Self {
        let root = temporary(name);
        let _ = std::fs::remove_dir_all(&root);
        let source = root.join("source");
        std::fs::create_dir_all(source.join("nested/short")).unwrap();
        std::fs::write(source.join("nested/data.bin"), b"private entrybound bytes").unwrap();
        std::fs::write(
            source.join("nested/short/a.txt"),
            b"repeated repeated repeated",
        )
        .unwrap();
        Self { root, source }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

#[test]
fn encrypted_hybrid_indexed_round_trip_is_metadata_private() {
    let fixture = Fixture::new("crypto-hybrid");
    let archive = plan_directory(&fixture.source, PackOptions::default()).unwrap();
    let (identity, recipient) = XWingIdentity::generate().unwrap();
    let encrypted = encrypt_archive(
        &archive,
        EncryptedWriteOptions {
            recipients: std::slice::from_ref(&recipient),
            ..EncryptedWriteOptions::default()
        },
    )
    .unwrap();
    assert!(!contains(&encrypted.bytes, b"nested/data.bin"));
    assert!(!contains(&encrypted.bytes, b"private entrybound bytes"));
    assert_ne!(
        u64::from_be_bytes(encrypted.bytes[16..24].try_into().unwrap())
            & FEATURE_PRIVATE_RESOURCE_DECLARATION_V1,
        0
    );
    let public = inspect_encrypted(&encrypted.bytes, None, CryptoPolicy::default()).unwrap();
    assert!(public.authenticated.is_none());
    assert!(public.authenticated_descriptor.is_none());
    assert_eq!(public.public.recipient_count, 1);

    let private = inspect_encrypted(
        &encrypted.bytes,
        Some(Unlock::Identity(&identity)),
        CryptoPolicy::default(),
    )
    .unwrap();
    let descriptor = private.authenticated_descriptor.unwrap();
    assert_eq!(descriptor.record_version, 2);
    assert!(descriptor.producer_declaration_present);
    assert!(descriptor.independently_validated);
    assert_eq!(
        descriptor.declared_budget,
        Some(encrypted.archive.descriptor.budget)
    );
    assert_eq!(
        descriptor.declared_decode,
        Some(encrypted.archive.descriptor.decode)
    );

    let opened = open_identity(&encrypted.bytes, &identity).unwrap();
    assert!(opened.archive.descriptor.budget_declared);
    assert_eq!(opened.archive.entry_set.len(), archive.entry_set.len());
    assert_eq!(opened.report.identities.lai, encrypted.identities.lai);
    assert_eq!(opened.report.identities.pcr, encrypted.identities.pcr);
    assert_eq!(opened.report.identities.aux, encrypted.identities.aux);
}

#[test]
fn private_resource_declaration_feature_is_forbidden_on_unencrypted_archives() {
    let fixture = Fixture::new("crypto-private-resource-feature");
    let mut archive = plan_directory(&fixture.source, PackOptions::default()).unwrap();
    archive.descriptor.features.incompat |= FEATURE_PRIVATE_RESOURCE_DECLARATION_V1;
    assert_eq!(
        encode(&archive, WriteOptions::default())
            .unwrap_err()
            .code(),
        ReasonCode::UnsupportedRequiredFeature
    );
}

#[test]
fn multiple_hybrid_recipients_unlock_and_wrong_identity_is_indistinguishable() {
    let fixture = Fixture::new("crypto-multiple");
    let archive = plan_directory(&fixture.source, PackOptions::default()).unwrap();
    let (first_identity, first_recipient) = XWingIdentity::generate().unwrap();
    let (second_identity, second_recipient) = XWingIdentity::generate().unwrap();
    let (wrong_identity, _) = XWingIdentity::generate().unwrap();
    let first_recipient =
        XWingRecipient::from_bytes(first_recipient.bytes(), "a much longer label").unwrap();
    let second_recipient = XWingRecipient::from_bytes(second_recipient.bytes(), "b").unwrap();
    let recipients = [second_recipient, first_recipient];
    let encrypted = encrypt_archive(
        &archive,
        EncryptedWriteOptions {
            recipients: &recipients,
            ..EncryptedWriteOptions::default()
        },
    )
    .unwrap();
    for identity in [&first_identity, &second_identity] {
        open_identity(&encrypted.bytes, identity).unwrap();
    }
    let error = open_identity(&encrypted.bytes, &wrong_identity).unwrap_err();
    assert_eq!(error.code(), ReasonCode::CryptoNoMatchingRecipient);

    let duplicate = encrypt_archive(
        &archive,
        EncryptedWriteOptions {
            recipients: &[recipients[0].clone(), recipients[0].clone()],
            ..EncryptedWriteOptions::default()
        },
    )
    .unwrap_err();
    assert_eq!(duplicate.code(), ReasonCode::CryptoRecipientPolicyInvalid);
}

#[test]
fn password_archive_unlocks_and_wrong_password_does_not_materialize() {
    let fixture = Fixture::new("crypto-password");
    let archive = plan_directory(&fixture.source, PackOptions::default()).unwrap();
    let encrypted = encrypt_archive(
        &archive,
        EncryptedWriteOptions {
            password: Some(b"a long high entropy archive password"),
            ..EncryptedWriteOptions::default()
        },
    )
    .unwrap();
    open_encrypted(
        &encrypted.bytes,
        EncryptedOpenOptions::new(Some(Unlock::Password(
            b"a long high entropy archive password",
        ))),
    )
    .unwrap();
    let destination = fixture.root.join("must-not-exist");
    let error = open_encrypted(
        &encrypted.bytes,
        EncryptedOpenOptions::new(Some(Unlock::Password(b"wrong password"))),
    )
    .unwrap_err();
    assert_eq!(error.code(), ReasonCode::CryptoNoMatchingRecipient);
    assert!(!destination.exists());
}

#[test]
fn encrypted_filesystem_pack_uses_secret_chunking_and_enc_planner_ids() {
    let fixture = Fixture::new("crypto-keyed-cdc");
    std::fs::write(
        fixture.source.join("large.bin"),
        deterministic_bytes(2 * 1024 * 1024 + 37),
    )
    .unwrap();
    let (identity, recipient) = XWingIdentity::generate().unwrap();
    let options = EncryptedWriteOptions {
        recipients: std::slice::from_ref(&recipient),
        ..EncryptedWriteOptions::default()
    };
    let first =
        pack_directory_encrypted(&fixture.source, PackOptions::default(), options.clone()).unwrap();
    let second =
        pack_directory_encrypted(&fixture.source, PackOptions::default(), options).unwrap();
    assert_eq!(first.archive.descriptor.planner_id, "balanced-enc-v1");
    assert!(
        first
            .archive
            .descriptor
            .chunker_id
            .starts_with("gear-norm-secret-table-v1/")
    );
    assert_eq!(first.identities.lai, second.identities.lai);
    assert_eq!(first.identities.aux, second.identities.aux);
    assert_ne!(first.identities.pci, second.identities.pci);
    open_identity(&first.bytes, &identity).unwrap();

    std::fs::write(fixture.source.join("large.bin"), deterministic_bytes(256)).unwrap();
    let strong = pack_directory_encrypted(
        &fixture.source,
        PackOptions {
            profile: entrybound::planner::CompressionProfile::Fast,
            ..PackOptions::default()
        },
        EncryptedWriteOptions {
            recipients: std::slice::from_ref(&recipient),
            boundary: BoundaryMode::PhteAes128,
            ..EncryptedWriteOptions::default()
        },
    )
    .unwrap();
    assert_eq!(strong.archive.descriptor.planner_id, "fast-enc-v1");
    assert!(
        strong
            .archive
            .descriptor
            .chunker_id
            .starts_with("phte-aes128-norm-v1/")
    );
}

#[test]
fn every_padding_mode_round_trips() {
    let fixture = Fixture::new("crypto-padding");
    std::fs::remove_dir_all(fixture.source.join("nested")).unwrap();
    let archive = plan_directory(&fixture.source, PackOptions::default()).unwrap();
    let (identity, recipient) = XWingIdentity::generate().unwrap();
    for padding in [
        PaddingMode::None,
        PaddingMode::Bucketed,
        PaddingMode::Maximum,
    ] {
        let encrypted = encrypt_archive(
            &archive,
            EncryptedWriteOptions {
                recipients: std::slice::from_ref(&recipient),
                padding,
                ..EncryptedWriteOptions::default()
            },
        )
        .unwrap();
        assert_eq!(encrypted.public.padding, padding);
        open_identity(&encrypted.bytes, &identity).unwrap();
    }
}

#[test]
fn commitment_envelope_ciphertext_footer_and_truncation_tampering_fail_closed() {
    let fixture = Fixture::new("crypto-tamper");
    let archive = plan_directory(&fixture.source, PackOptions::default()).unwrap();
    let (identity, recipient) = XWingIdentity::generate().unwrap();
    let encrypted = encrypt_archive(
        &archive,
        EncryptedWriteOptions {
            recipients: std::slice::from_ref(&recipient),
            ..EncryptedWriteOptions::default()
        },
    )
    .unwrap();

    let mut commitment = encrypted.bytes.clone();
    mutate_envelope_field(&mut commitment, 4);
    assert_eq!(
        open_identity(&commitment, &identity).unwrap_err().code(),
        ReasonCode::CryptoKeyCommitmentFailed
    );

    let mut envelope_mac = encrypted.bytes.clone();
    mutate_envelope_field(&mut envelope_mac, 9);
    assert_eq!(
        open_identity(&envelope_mac, &identity).unwrap_err().code(),
        ReasonCode::CryptoEnvelopeAuthFailed
    );

    let mut ciphertext = encrypted.bytes.clone();
    mutate_first_ciphertext_and_rehash(&mut ciphertext);
    assert_eq!(
        open_identity(&ciphertext, &identity).unwrap_err().code(),
        ReasonCode::CryptoAeadAuthFailed
    );

    let mut footer = encrypted.bytes.clone();
    let footer_start = footer.len() - 192;
    footer[footer_start + 23] ^= 1;
    assert!(open_identity(&footer, &identity).is_err());

    for remove in [1, 191, 193] {
        let truncated = &encrypted.bytes[..encrypted.bytes.len() - remove];
        let error = open_identity(truncated, &identity).unwrap_err();
        assert!(matches!(
            error.code(),
            ReasonCode::TruncatedFooter | ReasonCode::IncorrectTotalLength
        ));
    }
}

#[test]
fn recipient_and_segment_reordering_splicing_and_policy_limits_fail_closed() {
    let fixture = Fixture::new("crypto-adversarial-order");
    let archive = plan_directory(&fixture.source, PackOptions::default()).unwrap();
    let (first_identity, first_recipient) = XWingIdentity::generate().unwrap();
    let (second_identity, second_recipient) = XWingIdentity::generate().unwrap();
    let recipients = [first_recipient, second_recipient];
    let encrypted = encrypt_archive(
        &archive,
        EncryptedWriteOptions {
            recipients: &recipients,
            ..EncryptedWriteOptions::default()
        },
    )
    .unwrap();

    let mut reordered_stanzas = encrypted.bytes.clone();
    exchange_first_two_stanzas(&mut reordered_stanzas, false);
    assert_eq!(
        open_identity(&reordered_stanzas, &first_identity)
            .unwrap_err()
            .code(),
        ReasonCode::CryptoRecipientStanzaInvalid
    );

    let mut duplicate_stanza = encrypted.bytes.clone();
    exchange_first_two_stanzas(&mut duplicate_stanza, true);
    assert_eq!(
        open_identity(&duplicate_stanza, &first_identity)
            .unwrap_err()
            .code(),
        ReasonCode::CryptoRecipientStanzaInvalid
    );

    let mut unknown_stanza = encrypt_archive(
        &archive,
        EncryptedWriteOptions {
            recipients: std::slice::from_ref(&recipients[0]),
            ..EncryptedWriteOptions::default()
        },
    )
    .unwrap()
    .bytes;
    set_first_stanza_type(&mut unknown_stanza, 0x7777);
    assert_eq!(
        open_identity(&unknown_stanza, &first_identity)
            .unwrap_err()
            .code(),
        ReasonCode::CryptoNoMatchingRecipient
    );

    let mut malformed_encapsulation = encrypt_archive(
        &archive,
        EncryptedWriteOptions {
            recipients: std::slice::from_ref(&recipients[0]),
            ..EncryptedWriteOptions::default()
        },
    )
    .unwrap()
    .bytes;
    mutate_first_stanza_field_byte(&mut malformed_encapsulation, 7);
    assert_eq!(
        open_identity(&malformed_encapsulation, &first_identity)
            .unwrap_err()
            .code(),
        ReasonCode::CryptoNoMatchingRecipient
    );

    let mut reordered_segments = encrypted.bytes.clone();
    reorder_first_two_segments(&mut reordered_segments);
    assert_eq!(
        open_identity(&reordered_segments, &first_identity)
            .unwrap_err()
            .code(),
        ReasonCode::CryptoSegmentStructureInvalid
    );

    let second_archive = encrypt_archive(
        &archive,
        EncryptedWriteOptions {
            recipients: &recipients,
            ..EncryptedWriteOptions::default()
        },
    )
    .unwrap();
    let mut spliced = encrypted.bytes.clone();
    splice_first_segment(&mut spliced, &second_archive.bytes);
    assert_eq!(
        open_identity(&spliced, &first_identity).unwrap_err().code(),
        ReasonCode::CryptoAeadAuthFailed
    );

    let mut end_mutation = encrypted.bytes.clone();
    mutate_first_segment_end(&mut end_mutation);
    assert_eq!(
        open_identity(&end_mutation, &first_identity)
            .unwrap_err()
            .code(),
        ReasonCode::CryptoAeadAuthFailed
    );

    let mut locator = encrypted.bytes.clone();
    let footer = locator.len() - 192;
    locator[footer + 63] ^= 1;
    assert_eq!(
        open_identity(&locator, &first_identity).unwrap_err().code(),
        ReasonCode::CryptoSegmentStructureInvalid
    );

    let mut options = EncryptedOpenOptions::new(Some(Unlock::Identity(&second_identity)));
    options.crypto_policy = CryptoPolicy {
        max_segments: 1,
        ..CryptoPolicy::default()
    };
    assert_eq!(
        open_encrypted(&encrypted.bytes, options)
            .unwrap_err()
            .code(),
        ReasonCode::CryptoResourcePolicyRefused
    );
}

#[test]
fn unknown_required_crypto_feature_fails_before_recipient_work() {
    let fixture = Fixture::new("crypto-unknown-feature");
    let archive = plan_directory(&fixture.source, PackOptions::default()).unwrap();
    let (identity, recipient) = XWingIdentity::generate().unwrap();
    let mut bytes = encrypt_archive(
        &archive,
        EncryptedWriteOptions {
            recipients: std::slice::from_ref(&recipient),
            ..EncryptedWriteOptions::default()
        },
    )
    .unwrap()
    .bytes;
    let features = u64::from_be_bytes(bytes[16..24].try_into().unwrap()) | (1 << 63);
    bytes[16..24].copy_from_slice(&features.to_be_bytes());
    let error = open_identity(&bytes, &identity).unwrap_err();
    assert_eq!(error.code(), ReasonCode::CryptoSuiteUnsupported);
}

fn open_identity(
    bytes: &[u8],
    identity: &XWingIdentity,
) -> entrybound::diagnostics::Result<entrybound::ecf::OpenedArchive> {
    open_encrypted(
        bytes,
        EncryptedOpenOptions::new(Some(Unlock::Identity(identity))),
    )
}

fn mutate_envelope_field(bytes: &mut [u8], wanted: u16) {
    let section = 256usize;
    let payload_start = section + 64;
    let payload_len =
        u64::from_be_bytes(bytes[section + 16..section + 24].try_into().unwrap()) as usize;
    let mut cursor = payload_start + 16;
    let end = payload_start + payload_len;
    while cursor < end {
        let tag = u16::from_be_bytes(bytes[cursor..cursor + 2].try_into().unwrap());
        let len = u64::from_be_bytes(bytes[cursor + 4..cursor + 12].try_into().unwrap()) as usize;
        if tag == wanted {
            bytes[cursor + 12] ^= 1;
            let digest: [u8; 32] = Sha256::digest(&bytes[payload_start..end]).into();
            bytes[section + 24..section + 56].copy_from_slice(&digest);
            return;
        }
        cursor += 12 + len;
    }
    panic!("envelope field {wanted} was not found");
}

fn mutate_first_ciphertext_and_rehash(bytes: &mut [u8]) {
    let footer_start = bytes.len() - 192;
    let segments_section = u64::from_be_bytes(
        bytes[footer_start + 40..footer_start + 48]
            .try_into()
            .unwrap(),
    ) as usize;
    let payload_start = segments_section + 64;
    let payload_len = u64::from_be_bytes(
        bytes[segments_section + 16..segments_section + 24]
            .try_into()
            .unwrap(),
    ) as usize;
    bytes[payload_start + 64 + 32] ^= 1;
    let digest: [u8; 32] =
        Sha256::digest(&bytes[payload_start..payload_start + payload_len]).into();
    bytes[segments_section + 24..segments_section + 56].copy_from_slice(&digest);
    bytes[footer_start + 104..footer_start + 136].copy_from_slice(&digest);
}

fn exchange_first_two_stanzas(bytes: &mut [u8], duplicate: bool) {
    let (start, len) = envelope_field(bytes, 8);
    let sequence = &mut bytes[start..start + len];
    assert_eq!(u64::from_be_bytes(sequence[..8].try_into().unwrap()), 2);
    let first_len = u64::from_be_bytes(sequence[8..16].try_into().unwrap()) as usize;
    let first = 8..16 + first_len;
    let second_start = first.end;
    let second_len =
        u64::from_be_bytes(sequence[second_start..second_start + 8].try_into().unwrap()) as usize;
    let second = second_start..second_start + 8 + second_len;
    assert_eq!(first.len(), second.len());
    let first_bytes = sequence[first.clone()].to_vec();
    let second_bytes = sequence[second.clone()].to_vec();
    sequence[first].copy_from_slice(if duplicate {
        &first_bytes
    } else {
        &second_bytes
    });
    sequence[second].copy_from_slice(&first_bytes);
    rehash_envelope(bytes);
}

fn set_first_stanza_type(bytes: &mut [u8], stanza_type: u16) {
    let (sequence_start, sequence_len) = envelope_field(bytes, 8);
    let sequence = &mut bytes[sequence_start..sequence_start + sequence_len];
    assert_eq!(u64::from_be_bytes(sequence[..8].try_into().unwrap()), 1);
    let record_len = u64::from_be_bytes(sequence[8..16].try_into().unwrap()) as usize;
    let record = &mut sequence[16..16 + record_len];
    let mut cursor = 16usize;
    while cursor < record.len() {
        let tag = u16::from_be_bytes(record[cursor..cursor + 2].try_into().unwrap());
        let len = u64::from_be_bytes(record[cursor + 4..cursor + 12].try_into().unwrap()) as usize;
        if tag == 2 {
            assert_eq!(len, 2);
            record[cursor + 12..cursor + 14].copy_from_slice(&stanza_type.to_be_bytes());
            rehash_envelope(bytes);
            return;
        }
        cursor += 12 + len;
    }
    panic!("stanza type was not found");
}

fn mutate_first_stanza_field_byte(bytes: &mut [u8], wanted: u16) {
    let (sequence_start, sequence_len) = envelope_field(bytes, 8);
    let sequence = &mut bytes[sequence_start..sequence_start + sequence_len];
    assert_eq!(u64::from_be_bytes(sequence[..8].try_into().unwrap()), 1);
    let record_len = u64::from_be_bytes(sequence[8..16].try_into().unwrap()) as usize;
    let record = &mut sequence[16..16 + record_len];
    let mut cursor = 16usize;
    while cursor < record.len() {
        let tag = u16::from_be_bytes(record[cursor..cursor + 2].try_into().unwrap());
        let len = u64::from_be_bytes(record[cursor + 4..cursor + 12].try_into().unwrap()) as usize;
        if tag == wanted {
            assert!(len > 0);
            record[cursor + 12] ^= 1;
            rehash_envelope(bytes);
            return;
        }
        cursor += 12 + len;
    }
    panic!("stanza field {wanted} was not found");
}

fn envelope_field(bytes: &[u8], wanted: u16) -> (usize, usize) {
    let payload_start = 256 + 64;
    let payload_len = u64::from_be_bytes(bytes[272..280].try_into().unwrap()) as usize;
    let mut cursor = payload_start + 16;
    let end = payload_start + payload_len;
    while cursor < end {
        let tag = u16::from_be_bytes(bytes[cursor..cursor + 2].try_into().unwrap());
        let len = u64::from_be_bytes(bytes[cursor + 4..cursor + 12].try_into().unwrap()) as usize;
        if tag == wanted {
            return (cursor + 12, len);
        }
        cursor += 12 + len;
    }
    panic!("envelope field {wanted} was not found");
}

fn rehash_envelope(bytes: &mut [u8]) {
    let payload_start = 256 + 64;
    let payload_len = u64::from_be_bytes(bytes[272..280].try_into().unwrap()) as usize;
    let digest: [u8; 32] =
        Sha256::digest(&bytes[payload_start..payload_start + payload_len]).into();
    bytes[280..312].copy_from_slice(&digest);
}

fn segment_ranges(bytes: &[u8]) -> Vec<std::ops::Range<usize>> {
    let footer = bytes.len() - 192;
    let section = u64::from_be_bytes(bytes[footer + 40..footer + 48].try_into().unwrap()) as usize;
    let payload_len =
        u64::from_be_bytes(bytes[section + 16..section + 24].try_into().unwrap()) as usize;
    let end = section + 64 + payload_len;
    let mut cursor = section + 64;
    let mut ranges = Vec::new();
    while cursor < end {
        let extent =
            u64::from_be_bytes(bytes[cursor + 48..cursor + 56].try_into().unwrap()) as usize;
        ranges.push(cursor..cursor + extent);
        cursor += extent;
    }
    assert_eq!(cursor, end);
    ranges
}

fn reorder_first_two_segments(bytes: &mut [u8]) {
    let ranges = segment_ranges(bytes);
    assert!(ranges.len() >= 2);
    let first = bytes[ranges[0].clone()].to_vec();
    let second = bytes[ranges[1].clone()].to_vec();
    let start = ranges[0].start;
    bytes[start..start + second.len()].copy_from_slice(&second);
    bytes[start + second.len()..start + second.len() + first.len()].copy_from_slice(&first);
    rehash_segments(bytes);
}

fn splice_first_segment(target: &mut [u8], source: &[u8]) {
    let target_ranges = segment_ranges(target);
    let source_ranges = segment_ranges(source);
    assert_eq!(target_ranges[0].len(), source_ranges[0].len());
    target[target_ranges[0].clone()].copy_from_slice(&source[source_ranges[0].clone()]);
    rehash_segments(target);
}

fn mutate_first_segment_end(bytes: &mut [u8]) {
    let first = segment_ranges(bytes)[0].clone();
    bytes[first.end - 1] ^= 1;
    rehash_segments(bytes);
}

fn rehash_segments(bytes: &mut [u8]) {
    let footer = bytes.len() - 192;
    let section = u64::from_be_bytes(bytes[footer + 40..footer + 48].try_into().unwrap()) as usize;
    let payload_len =
        u64::from_be_bytes(bytes[section + 16..section + 24].try_into().unwrap()) as usize;
    let payload_start = section + 64;
    let digest: [u8; 32] =
        Sha256::digest(&bytes[payload_start..payload_start + payload_len]).into();
    bytes[section + 24..section + 56].copy_from_slice(&digest);
    bytes[footer + 104..footer + 136].copy_from_slice(&digest);
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

fn deterministic_bytes(len: usize) -> Vec<u8> {
    let mut state = 0x1234_5678_9abc_def0_u64;
    (0..len)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state as u8
        })
        .collect()
}

fn temporary(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("entrybound-{}-{name}", std::process::id()))
}
