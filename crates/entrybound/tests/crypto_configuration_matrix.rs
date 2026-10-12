//! Generated conformance inputs; test keys have no use outside these fixtures.
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use entrybound::archive::{ExtractionPolicy, PackOptions, unpack_opened};
use entrybound::crypto::{
    BoundaryMode, EncryptedOpenOptions, EncryptedWriteOptions, PaddingMode, Unlock, XWingIdentity,
    open_encrypted, pack_directory_encrypted,
};
use entrybound::diagnostics::ReasonCode;
use entrybound::eam::{
    Archive, ArchiveDescriptor, ArchiveRole, ContentStore, DecodeRequirements, Digest,
    DigestAlgorithm, EntrySet, FeatureSet, FidelityReport, IdentityProfile, Index, Layout,
    ResourceBudget, TransformPlan,
};
use entrybound::ecf::{WriteOptions, encode, open};
use entrybound::identity::{STORE_CODEC_IDENTIFIER, STORE_PLAN_ID, STORE_PLAN_IDENTIFIER};
use entrybound::planner::CompressionProfile;

const LENGTHS: [usize; 14] = [
    0, 1, 15, 16, 17, 255, 256, 257, 4095, 4096, 4097, 65535, 65536, 65537,
];
static NEXT: AtomicUsize = AtomicUsize::new(0);

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "entrybound-configuration-matrix-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        fs::create_dir(path.join("source")).unwrap();
        for length in LENGTHS {
            fs::write(
                path.join("source").join(format!("length-{length}.bin")),
                payload(length),
            )
            .unwrap();
        }
        Self(path)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if !std::thread::panicking()
            && std::env::var_os("ENTRYBOUND_RETAIN_MATRIX_FIXTURES").is_none()
        {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }
}

fn payload(length: usize) -> Vec<u8> {
    (0..length)
        .map(|i| ((i * 73 + i / 11 + 19) & 255) as u8)
        .collect()
}

#[test]
fn every_crypto_creation_configuration_reconstructs_boundary_files_and_metadata() {
    let fixture = Fixture::new();
    let source = fixture.0.join("source");
    let identity = XWingIdentity::from_seed([0x21; 32]);
    let recipient = identity.recipient();
    let wrong_identity = XWingIdentity::from_seed([0x22; 32]);
    let mut cases = 0;
    for profile in [
        CompressionProfile::Fast,
        CompressionProfile::Balanced,
        CompressionProfile::Dense,
        CompressionProfile::Extreme,
    ] {
        for padding in [
            PaddingMode::None,
            PaddingMode::Bucketed,
            PaddingMode::Maximum,
        ] {
            for boundary in [BoundaryMode::SecretGearTable, BoundaryMode::PhteAes128] {
                for include_index in [false, true] {
                    for password in [false, true] {
                        let recipients = if password {
                            &[][..]
                        } else {
                            std::slice::from_ref(&recipient)
                        };
                        let encrypted = pack_directory_encrypted(
                            &source,
                            PackOptions {
                                profile,
                                ..PackOptions::default()
                            },
                            EncryptedWriteOptions {
                                recipients,
                                password: password
                                    .then_some(&b"configuration matrix test password"[..]),
                                padding,
                                boundary,
                                include_index,
                                ..EncryptedWriteOptions::default()
                            },
                        )
                        .unwrap();
                        assert_eq!(encrypted.public.padding, padding);
                        assert_eq!(encrypted.public.boundary, boundary);
                        let unlock = if password {
                            Unlock::Password(b"configuration matrix test password")
                        } else {
                            Unlock::Identity(&identity)
                        };
                        let opened = open_encrypted(
                            &encrypted.bytes,
                            EncryptedOpenOptions::new(Some(unlock)),
                        )
                        .unwrap();
                        assert_eq!(opened.report.identities, encrypted.identities);
                        assert_eq!(opened.archive.entry_set, encrypted.archive.entry_set);
                        assert_eq!(opened.archive.fidelity, encrypted.archive.fidelity);
                        assert_eq!(
                            opened.archive.content_store.objects,
                            encrypted.archive.content_store.objects
                        );
                        let destination = fixture.0.join(format!("restored-{cases}"));
                        unpack_opened(&opened, &destination, ExtractionPolicy::default()).unwrap();
                        for length in LENGTHS {
                            let name = format!("length-{length}.bin");
                            assert_eq!(fs::read(destination.join(&name)).unwrap(), payload(length));
                            assert_eq!(
                                fs::metadata(destination.join(&name))
                                    .unwrap()
                                    .modified()
                                    .unwrap(),
                                fs::metadata(source.join(&name))
                                    .unwrap()
                                    .modified()
                                    .unwrap()
                            );
                            #[cfg(unix)]
                            {
                                use std::os::unix::fs::PermissionsExt as _;
                                assert_eq!(
                                    fs::metadata(destination.join(&name))
                                        .unwrap()
                                        .permissions()
                                        .mode()
                                        & 0o777,
                                    fs::metadata(source.join(&name))
                                        .unwrap()
                                        .permissions()
                                        .mode()
                                        & 0o777
                                );
                            }
                        }
                        let wrong = if password {
                            Unlock::Password(b"incorrect configuration matrix password")
                        } else {
                            Unlock::Identity(&wrong_identity)
                        };
                        assert_eq!(
                            open_encrypted(
                                &encrypted.bytes,
                                EncryptedOpenOptions::new(Some(wrong))
                            )
                            .unwrap_err()
                            .code(),
                            ReasonCode::CryptoNoMatchingRecipient
                        );
                        let truncated = &encrypted.bytes[..encrypted.bytes.len() - 1];
                        assert!(matches!(
                            open_encrypted(truncated, EncryptedOpenOptions::new(Some(unlock)))
                                .unwrap_err()
                                .code(),
                            ReasonCode::TruncatedFooter | ReasonCode::IncorrectTotalLength
                        ));
                        if std::env::var_os("ENTRYBOUND_RETAIN_MATRIX_FIXTURES").is_some() {
                            let path = fixture.0.join(format!("ciphertext-{cases}.eb"));
                            fs::write(&path, &encrypted.bytes).unwrap();
                            println!(
                                "MATRIX_PCI {} {}",
                                path.display(),
                                encrypted.identities.pci.0
                            );
                        }
                        println!(
                            "MATRIX_CASE {cases} profile={profile:?} padding={padding:?} boundary={boundary:?} index={include_index} password={password} files={} exact_bytes=true metadata=true wrong_key_refused=true truncation_refused=true",
                            LENGTHS.len()
                        );
                        cases += 1;
                    }
                }
            }
        }
    }
    assert_eq!(cases, 96);
}

#[test]
fn all_padding_and_boundary_discriminants_follow_the_closed_registry() {
    for value in 0..=u8::MAX {
        if value <= 2 {
            assert_eq!(PaddingMode::try_from(value).unwrap() as u8, value);
        } else {
            assert_eq!(
                PaddingMode::try_from(value).unwrap_err().code(),
                ReasonCode::CryptoPaddingInvalid
            );
        }
        if matches!(value, 1 | 2) {
            assert_eq!(BoundaryMode::try_from(value).unwrap() as u8, value);
        } else {
            assert_eq!(
                BoundaryMode::try_from(value).unwrap_err().code(),
                ReasonCode::CryptoBoundaryModeUnsupported
            );
        }
    }
}

#[test]
fn empty_native_roots_match_independent_specification_goldens() {
    // Recompute these using tools/qualification/identity_reference.py.
    // The reference uses Python hashlib and explicit transcript lengths,
    // never Entrybound's encoder, reader or identity implementation.
    let archive = Archive {
        descriptor: ArchiveDescriptor {
            format_major: 0,
            format_minor: 1,
            format_namespace: "ecf/bootstrap-v1".into(),
            features: FeatureSet::default(),
            layout: Layout::Indexed,
            role: ArchiveRole::Complete,
            budget_declared: true,
            stream_dedup_window: 0,
            budget: ResourceBudget::default(),
            decode: DecodeRequirements::default(),
            identity_profile: IdentityProfile::IdentityV1,
            digest_algorithm: DigestAlgorithm::Sha256,
            planner_id: STORE_PLAN_IDENTIFIER.into(),
            chunker_id: "fixed-1mib/v1".into(),
            lai: Digest::ZERO,
            pcr: Digest::ZERO,
            aux: Digest::ZERO,
            pci: None,
        },
        entry_set: EntrySet::default(),
        content_store: ContentStore::default(),
        transform_plans: vec![TransformPlan {
            plan_id: STORE_PLAN_ID,
            identifier: STORE_PLAN_IDENTIFIER.into(),
            transforms: Box::default(),
            codec: STORE_CODEC_IDENTIFIER.into(),
            codec_params: Box::default(),
            dictionary: None,
            decode: DecodeRequirements::default(),
        }]
        .into_boxed_slice(),
        fidelity: FidelityReport {
            platform: "test".into(),
            ..FidelityReport::default()
        },
        conversion: None,
        preservation: None,
        index: Index::default(),
    };
    for include_index in [false, true] {
        let first = encode(&archive, WriteOptions { include_index }).unwrap();
        let second = encode(&archive, WriteOptions { include_index }).unwrap();
        assert_eq!(first.bytes, second.bytes);
        let opened = open(&first.bytes).unwrap();
        assert_eq!(
            opened.report.identities.lai.0.to_string(),
            "73ea379fc6f10b4ed514cbbcdb6a12889dcf721ef0e0b173431ef6c3f52c790f"
        );
        assert_eq!(
            opened.report.identities.pcr.0.to_string(),
            "01a1b9720e640af1d3fcce9532379ba50b53afedb8149419cf4f4de0a027b66e"
        );
        assert_eq!(
            opened.report.identities.aux.0.to_string(),
            "597430db8ce6fcc3d4b30863d37f5ecd81783443a55810b02d3e5405be77b177"
        );
        if std::env::var_os("ENTRYBOUND_RETAIN_MATRIX_FIXTURES").is_some() {
            let fixture = Fixture::new();
            let path = fixture.0.join("empty.eb");
            fs::write(&path, &first.bytes).unwrap();
            println!(
                "MATRIX_PCI {} {}",
                path.display(),
                opened.report.identities.pci.0
            );
        }
    }
}
