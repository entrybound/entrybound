use std::fs;
use std::ops::Range;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use entrybound::archive::{
    CollisionPolicy, ExtractionPolicy, PackOptions, bootstrap_decode_policy,
    bootstrap_resource_policy, explain, inspect, pack_directory, unpack, unpack_stream,
};
use entrybound::diagnostics::{OutcomeClass, ReasonCode};
use entrybound::eam::DecodeRequirements;
use entrybound::ecf::{
    FOOTER_LEN, PREAMBLE_LEN, SECTION_HEADER_LEN, StreamWindow, StreamWriteOptions,
    bootstrap_sequential_limits, encode_stream, open, open_with_limits, verify,
};
use entrybound::identity::{BOOTSTRAP_CHUNK_SIZE, sha256_exact};
use entrybound::planner::CompressionProfile;

static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

#[test]
fn planner_selects_codecs_per_chunk_and_round_trips_exactly() {
    let fixture = Fixture::new("selection");
    let source = fixture.path.join("source");
    fs::create_dir(&source).unwrap();
    fs::write(source.join("repetitive.txt"), vec![b'a'; 256 * 1024]).unwrap();
    fs::write(source.join("zeroes.bin"), vec![0_u8; 96 * 1024]).unwrap();
    fs::write(source.join("noise.bin"), deterministic_noise(256 * 1024)).unwrap();
    fs::write(source.join("tiny"), b"small").unwrap();

    let mut mixed = vec![b'x'; BOOTSTRAP_CHUNK_SIZE];
    mixed.extend_from_slice(&deterministic_noise(BOOTSTRAP_CHUNK_SIZE));
    fs::write(source.join("mixed.bin"), &mixed).unwrap();

    let encoded = pack(&source, CompressionProfile::Balanced);
    let opened = open(&encoded.bytes).unwrap();
    let view = inspect(&opened).unwrap();
    let store = view
        .codec_usage
        .iter()
        .find(|usage| usage.codec == "store/v1")
        .unwrap();
    let zstandard = view
        .codec_usage
        .iter()
        .find(|usage| usage.codec == "zstandard/v1")
        .unwrap();
    assert!(store.chunk_count >= 2);
    assert!(zstandard.chunk_count >= 3);
    assert!(zstandard.stored_bytes < zstandard.logical_bytes);

    let mixed_entry = opened
        .archive
        .entry_set
        .entries()
        .iter()
        .find(|entry| entry.path().to_string() == "mixed.bin")
        .unwrap();
    let entrybound::eam::EntryData::File {
        content: entrybound::eam::ContentRef::Internal(object_id),
    } = mixed_entry.data()
    else {
        panic!("mixed fixture was not a file")
    };
    let object = &opened.archive.content_store.objects[&object_id];
    let selected = object
        .chunks
        .iter()
        .map(|reference| {
            let plan_id = opened.archive.content_store.chunks[&reference.chunk_id].plan_ref;
            opened
                .archive
                .transform_plans
                .iter()
                .find(|plan| plan.plan_id == plan_id)
                .unwrap()
                .codec
                .as_str()
        })
        .collect::<Vec<_>>();
    assert!(selected.contains(&"zstandard/v1"));
    assert!(selected.contains(&"store/v1"));

    verify(&encoded.bytes).unwrap();
    let destination = fixture.path.join("restored");
    unpack(&encoded.bytes, &destination, ExtractionPolicy::default()).unwrap();
    for name in [
        "repetitive.txt",
        "zeroes.bin",
        "noise.bin",
        "tiny",
        "mixed.bin",
    ] {
        assert_eq!(
            fs::read(source.join(name)).unwrap(),
            fs::read(destination.join(name)).unwrap()
        );
    }

    let explanation = explain(&opened).unwrap();
    assert_eq!(explanation.planner_id, "balanced-v6");
    assert!(explanation.physical_savings_bytes > 0);
}

#[test]
fn planning_and_native_encoding_are_deterministic() {
    let fixture = Fixture::new("determinism");
    let source = fixture.path.join("source");
    fs::create_dir(&source).unwrap();
    fs::write(source.join("compressible"), vec![b'q'; 300 * 1024]).unwrap();
    fs::write(source.join("noise"), deterministic_noise(200 * 1024)).unwrap();

    for profile in [
        CompressionProfile::Fast,
        CompressionProfile::Balanced,
        CompressionProfile::Dense,
        CompressionProfile::Extreme,
    ] {
        let first = pack(&source, profile);
        let second = pack(&source, profile);
        assert_eq!(first.bytes, second.bytes, "{}", profile.planner_id());
        assert_eq!(first.identities, second.identities);
        assert_eq!(first.archive.descriptor.planner_id, profile.planner_id());
        assert!(
            profile
                .chunking_candidates()
                .iter()
                .any(|candidate| candidate.chunker_id == first.archive.descriptor.chunker_id)
        );
    }
}

#[test]
fn v4_transform_pipeline_is_self_describing_across_ecf_and_unpack() {
    let fixture = Fixture::new("transform-pipeline");
    let source = fixture.path.join("source");
    fs::create_dir(&source).unwrap();
    let numeric = (0_u32..65_536)
        .flat_map(u32::to_le_bytes)
        .collect::<Vec<_>>();
    fs::write(source.join("numeric.bin"), &numeric).unwrap();

    let first = pack(&source, CompressionProfile::Dense);
    let second = pack(&source, CompressionProfile::Dense);
    assert_eq!(first.bytes, second.bytes);
    let opened = open(&first.bytes).unwrap();
    let view = inspect(&opened).unwrap();
    assert_ne!(
        opened.archive.descriptor.features.incompat & entrybound::ecf::FEATURE_CODEC_TRANSFORM_V1,
        0
    );
    assert!(view.transformed_chunk_count > 0);
    assert!(!view.transform_usage.is_empty());
    assert!(view.plans.iter().any(|plan| !plan.transforms.is_empty()));
    let mut missing_feature = first.archive.clone();
    missing_feature.descriptor.features.incompat &= !entrybound::ecf::FEATURE_CODEC_TRANSFORM_V1;
    assert_eq!(
        entrybound::ecf::encode(&missing_feature, entrybound::ecf::WriteOptions::default())
            .unwrap_err()
            .code(),
        ReasonCode::UnsupportedRequiredFeature
    );
    let mut unknown_transform = first.archive.clone();
    unknown_transform
        .transform_plans
        .iter_mut()
        .find(|plan| !plan.transforms.is_empty())
        .unwrap()
        .transforms[0]
        .transform_id = "unknown-transform/v1".to_owned();
    assert_eq!(
        entrybound::ecf::encode(&unknown_transform, entrybound::ecf::WriteOptions::default())
            .unwrap_err()
            .code(),
        ReasonCode::UnknownTransform
    );
    verify(&first.bytes).unwrap();

    let destination = fixture.path.join("restored");
    unpack(&first.bytes, &destination, ExtractionPolicy::default()).unwrap();
    assert_eq!(fs::read(destination.join("numeric.bin")).unwrap(), numeric);

    let mut stream = Vec::new();
    let stream_summary = encode_stream(
        &first.archive,
        StreamWriteOptions {
            window: StreamWindow::Auto,
            budget_declared: true,
        },
        &mut stream,
    )
    .unwrap();
    assert_eq!(first.identities.lai, stream_summary.identities.lai);
    assert_eq!(first.identities.aux, stream_summary.identities.aux);
    assert_eq!(first.identities.pcr, stream_summary.identities.pcr);
    let stream_destination = fixture.path.join("stream-restored");
    unpack_stream(
        stream.as_slice(),
        &stream_destination,
        ExtractionPolicy::default(),
        bootstrap_sequential_limits(),
    )
    .unwrap();
    assert_eq!(
        fs::read(stream_destination.join("numeric.bin")).unwrap(),
        numeric
    );

    let explanation = explain(&opened).unwrap();
    assert!(explanation.transformed_chunk_count > 0);
    assert!(!explanation.representative_pipelines.is_empty());
}

#[test]
fn creation_profile_is_physically_separate_from_logical_identity() {
    let fixture = Fixture::new("identity");
    let source = fixture.path.join("source");
    fs::create_dir(&source).unwrap();
    fs::write(source.join("content"), vec![b'z'; 512 * 1024]).unwrap();

    let fast = pack(&source, CompressionProfile::Fast);
    let extreme = pack(&source, CompressionProfile::Extreme);
    assert_eq!(fast.identities.lai, extreme.identities.lai);
    assert_eq!(fast.identities.aux, extreme.identities.aux);
    assert_eq!(
        fast.archive
            .content_store
            .objects
            .keys()
            .collect::<Vec<_>>(),
        extreme
            .archive
            .content_store
            .objects
            .keys()
            .collect::<Vec<_>>()
    );
    if fast.archive.descriptor.chunker_id == extreme.archive.descriptor.chunker_id {
        assert_eq!(fast.identities.pcr, extreme.identities.pcr);
    } else {
        assert_ne!(fast.identities.pcr, extreme.identities.pcr);
    }
    assert_ne!(fast.identities.pci, extreme.identities.pci);
    assert_ne!(fast.bytes, extreme.bytes);
}

#[test]
fn decoder_requirements_are_refused_before_decompression() {
    let fixture = Fixture::new("decode-policy");
    let source = fixture.path.join("source");
    fs::create_dir(&source).unwrap();
    fs::write(source.join("content"), vec![0_u8; 128 * 1024]).unwrap();
    let encoded = pack(&source, CompressionProfile::Balanced);
    assert_eq!(encoded.archive.descriptor.decode.window_bytes, 1024 * 1024);
    assert_eq!(
        encoded.archive.descriptor.decode.working_set_bytes,
        4 * 1024 * 1024
    );
    assert!(
        encoded.archive.descriptor.decode.working_set_bytes
            <= bootstrap_decode_policy().working_set_bytes
    );

    let error = open_with_limits(
        &encoded.bytes,
        bootstrap_resource_policy(),
        DecodeRequirements::default(),
    )
    .unwrap_err();
    assert_eq!(error.class(), OutcomeClass::PolicyRefused);
    assert_eq!(error.code(), ReasonCode::ResourceLimit);

    let destination = fixture.path.join("refused");
    let error = unpack(
        &encoded.bytes,
        &destination,
        ExtractionPolicy::new_with_decode(
            CollisionPolicy::Refuse,
            bootstrap_resource_policy(),
            DecodeRequirements::default(),
        ),
    )
    .unwrap_err();
    assert_eq!(error.class(), OutcomeClass::PolicyRefused);
    assert!(!destination.exists());

    let mut underdeclared = encoded.bytes.clone();
    underdeclared[120..128].copy_from_slice(&1_000_u64.to_be_bytes());
    rehash_preamble(&mut underdeclared);
    let error = open(&underdeclared).unwrap_err();
    assert_eq!(error.class(), OutcomeClass::Corrupt);
    assert_eq!(error.code(), ReasonCode::ResourceLimit);
}

#[test]
fn compressed_payload_and_plan_corruption_have_typed_failures() {
    let fixture = Fixture::new("corruption");
    let source = fixture.path.join("source");
    fs::create_dir(&source).unwrap();
    fs::write(source.join("content"), vec![b'r'; 128 * 1024]).unwrap();
    let encoded = pack(&source, CompressionProfile::Balanced);
    let chunk_section = chunk_section_kind(&encoded.bytes);

    let mut damaged_payload = encoded.bytes.clone();
    let (_, chunk_data) = locate_section(&damaged_payload, chunk_section);
    damaged_payload[chunk_data.start + 96] ^= 0xff;
    rehash_section(&mut damaged_payload, chunk_section);
    let error = verify(&damaged_payload).unwrap_err();
    assert_eq!(error.code(), ReasonCode::DecompressionFailed);

    let mut wrong_logical_length = encoded.bytes.clone();
    let (_, chunk_data) = locate_section(&wrong_logical_length, chunk_section);
    let declared = u64::from_be_bytes(
        wrong_logical_length[chunk_data.start + 48..chunk_data.start + 56]
            .try_into()
            .unwrap(),
    );
    wrong_logical_length[chunk_data.start + 48..chunk_data.start + 56]
        .copy_from_slice(&(declared + 1).to_be_bytes());
    wrong_logical_length[112..120].copy_from_slice(&(declared + 1).to_be_bytes());
    let expansion = u64::from_be_bytes(wrong_logical_length[120..128].try_into().unwrap());
    wrong_logical_length[120..128].copy_from_slice(&(expansion + 1_000).to_be_bytes());
    rehash_preamble(&mut wrong_logical_length);
    rehash_section(&mut wrong_logical_length, chunk_section);
    let error = verify(&wrong_logical_length).unwrap_err();
    assert_eq!(error.code(), ReasonCode::DecompressedLengthMismatch);

    let mut unknown_codec = encoded.bytes.clone();
    let (_, plans) = locate_section(&unknown_codec, 2);
    let second_record = next_record(&unknown_codec, plans.start);
    let codec = field_value(&unknown_codec, second_record, 4);
    assert_eq!(codec.len(), 12);
    unknown_codec[codec].copy_from_slice(b"mysteryxx/v1");
    rehash_section(&mut unknown_codec, 2);
    let error = verify(&unknown_codec).unwrap_err();
    assert_eq!(error.class(), OutcomeClass::Unsupported);
    assert_eq!(error.code(), ReasonCode::UnknownCodec);

    let mut invalid_parameters = encoded.bytes.clone();
    let (_, plans) = locate_section(&invalid_parameters, 2);
    let second_record = next_record(&invalid_parameters, plans.start);
    let parameters = field_value(&invalid_parameters, second_record, 5);
    invalid_parameters[parameters.start] ^= 1;
    rehash_section(&mut invalid_parameters, 2);
    let error = verify(&invalid_parameters).unwrap_err();
    assert_eq!(error.code(), ReasonCode::InvalidCodecParameters);
}

fn pack(source: &std::path::Path, profile: CompressionProfile) -> entrybound::ecf::EncodedArchive {
    pack_directory(
        source,
        PackOptions {
            profile,
            ..PackOptions::default()
        },
    )
    .unwrap()
}

fn deterministic_noise(len: usize) -> Vec<u8> {
    let mut state = 0x6a09_e667_f3bc_c909_u64;
    (0..len)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state as u8
        })
        .collect()
}

fn next_record(bytes: &[u8], start: usize) -> usize {
    let payload_len = usize::try_from(u64::from_be_bytes(
        bytes[start + 8..start + 16].try_into().unwrap(),
    ))
    .unwrap();
    start + 16 + payload_len
}

fn field_value(bytes: &[u8], record_start: usize, wanted: u16) -> Range<usize> {
    let payload_len = usize::try_from(u64::from_be_bytes(
        bytes[record_start + 8..record_start + 16]
            .try_into()
            .unwrap(),
    ))
    .unwrap();
    let mut cursor = record_start + 16;
    let end = cursor + payload_len;
    while cursor < end {
        let tag = u16::from_be_bytes(bytes[cursor..cursor + 2].try_into().unwrap());
        let len = usize::try_from(u64::from_be_bytes(
            bytes[cursor + 4..cursor + 12].try_into().unwrap(),
        ))
        .unwrap();
        if tag == wanted {
            return cursor + 12..cursor + 12 + len;
        }
        cursor += 12 + len;
    }
    panic!("field not found")
}

fn locate_section(bytes: &[u8], wanted: u16) -> (Range<usize>, Range<usize>) {
    let mut cursor = PREAMBLE_LEN as usize;
    let footer = bytes.len() - FOOTER_LEN as usize;
    while cursor < footer {
        let kind = u16::from_be_bytes(bytes[cursor + 4..cursor + 6].try_into().unwrap());
        let payload_len = usize::try_from(u64::from_be_bytes(
            bytes[cursor + 16..cursor + 24].try_into().unwrap(),
        ))
        .unwrap();
        let payload_start = cursor + SECTION_HEADER_LEN as usize;
        let payload_end = payload_start + payload_len;
        if kind == wanted {
            return (cursor..payload_start, payload_start..payload_end);
        }
        cursor = payload_end;
    }
    panic!("section not found")
}

fn chunk_section_kind(bytes: &[u8]) -> u16 {
    let incompat = u64::from_be_bytes(bytes[16..24].try_into().unwrap());
    if incompat & entrybound::ecf::FEATURE_WHOLE_OBJECT_RECONSTRUCTION_V1 != 0 {
        7
    } else if incompat & entrybound::ecf::FEATURE_RECONSTRUCTIVE_TRANSFORM_V1 != 0 {
        6
    } else if incompat & entrybound::ecf::FEATURE_CROSS_FILE_COMPRESSION_V1 != 0 {
        5
    } else {
        3
    }
}

fn rehash_section(bytes: &mut [u8], section: u16) {
    let (header, payload) = locate_section(bytes, section);
    let digest = sha256_exact(&bytes[payload]);
    bytes[header.start + 24..header.start + 56].copy_from_slice(digest.as_bytes());
}

fn rehash_preamble(bytes: &mut [u8]) {
    let digest = sha256_exact(&bytes[..PREAMBLE_LEN as usize]);
    let footer = bytes.len() - FOOTER_LEN as usize;
    bytes[footer + 64..footer + 96].copy_from_slice(digest.as_bytes());
}

struct Fixture {
    path: PathBuf,
}

impl Fixture {
    fn new(label: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "entrybound-compression-{label}-{}-{}",
            std::process::id(),
            NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self { path }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
