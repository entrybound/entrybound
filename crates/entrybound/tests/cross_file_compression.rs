use std::fs;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use entrybound::archive::{
    ExtractionPolicy, PackOptions, bootstrap_resource_policy, explain, inspect, pack_directory,
    unpack,
};
use entrybound::diagnostics::{OutcomeClass, ReasonCode};
use entrybound::eam::{DecodeRequirements, Digest, FeatureSet};
use entrybound::ecf::{
    FOOTER_LEN, PREAMBLE_LEN, SECTION_HEADER_LEN, WriteOptions, encode, open, open_with_limits,
    verify,
};
use entrybound::identity::sha256_exact;
use entrybound::planner::{
    CompressionProfile, UNPLANNED_PLAN_ID, plan_archive_v2, plan_archive_v3,
};

static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

#[test]
fn balanced_uses_one_shared_dictionary_and_keeps_independent_access() {
    let fixture = Fixture::new("dictionary");
    let source = fixture.path.join("source");
    write_similar_source(&source, 24, 64 * 1024, 0x6a09_e667_f3bc_c909);
    fs::write(
        source.join("unrelated.bin"),
        noise(64 * 1024, 0x510e_527f_ade6_82d1),
    )
    .unwrap();

    let first = pack(&source, CompressionProfile::Balanced);
    let second = pack(&source, CompressionProfile::Balanced);
    assert_eq!(first.bytes, second.bytes);
    assert_eq!(first.identities, second.identities);
    let opened = open(&first.bytes).unwrap();
    verify(&first.bytes).unwrap();
    let view = inspect(&opened).unwrap();
    assert_eq!(view.planner_id, "balanced-v6");
    assert_eq!(view.cross_file.dictionary_count, 1);
    assert!(view.cross_file.dictionary_bytes <= 8 * 1024);
    assert!(view.cross_file.dictionary_backed_chunks >= 8);
    assert_eq!(view.cross_file.chunk_group_count, 0);
    assert_eq!(view.cross_file.maximum_lookback, 0);
    assert!(view.cross_file.every_chunk_independently_decodable);
    assert!(
        opened
            .archive
            .content_store
            .dictionaries
            .values()
            .all(|dictionary| sha256_exact(&dictionary.bytes) == dictionary.dictionary_id)
    );
    let explanation = explain(&opened).unwrap();
    assert!(explanation.shared_dictionary_savings_bytes > 0);
    assert_eq!(
        explanation.physical_savings_bytes,
        explanation.ordinary_codec_savings_bytes
            + explanation.shared_dictionary_savings_bytes
            + explanation.bounded_lookback_savings_bytes
            + explanation.structural_transform_savings_bytes
    );
    assert_eq!(
        explanation.dictionary_storage_bytes,
        view.cross_file.dictionary_bytes
    );
    assert_eq!(explanation.bounded_lookback_savings_bytes, 0);

    let destination = fixture.path.join("restored");
    unpack(&first.bytes, &destination, ExtractionPolicy::default()).unwrap();
    assert_file_trees_equal(&source, &destination);

    let mut missing = first.archive.clone();
    missing.content_store.dictionaries.clear();
    assert_eq!(
        encode(&missing, WriteOptions::default())
            .unwrap_err()
            .code(),
        ReasonCode::UnknownDictionary
    );
    let mut unsupported = first.archive.clone();
    unsupported
        .content_store
        .dictionaries
        .values_mut()
        .next()
        .unwrap()
        .format = "unknown-dictionary-format/v1".to_owned();
    assert_eq!(
        encode(&unsupported, WriteOptions::default())
            .unwrap_err()
            .code(),
        ReasonCode::UnsupportedDictionaryFormat
    );
    assert_dictionary_corruption_fails(&first.bytes);
}

#[test]
fn dense_and_extreme_select_only_bounded_lookback_and_preserve_identity() {
    let fixture = Fixture::new("lookback");
    let source = fixture.path.join("source");
    write_similar_source(&source, 12, 64 * 1024, 0xbb67_ae85_84ca_a73b);
    fs::write(
        source.join("independent.bin"),
        noise(64 * 1024, 0xa54f_f53a_5f1d_36f1),
    )
    .unwrap();

    let balanced = pack(&source, CompressionProfile::Balanced);
    let dense = pack(&source, CompressionProfile::Dense);
    let extreme = pack(&source, CompressionProfile::Extreme);
    for encoded in [&dense, &extreme] {
        let opened = open(&encoded.bytes).unwrap();
        verify(&encoded.bytes).unwrap();
        let view = inspect(&opened).unwrap();
        assert!(view.cross_file.chunk_group_count >= 1);
        assert!(view.cross_file.maximum_lookback > 0);
        assert!(
            view.cross_file.maximum_lookback <= if view.planner_id == "dense-v6" { 4 } else { 8 }
        );
        assert_eq!(
            view.cross_file.worst_random_access_chunks,
            view.cross_file.maximum_lookback
        );
        assert!(view.cross_file.worst_random_access_bytes > 0);
        assert!(!view.cross_file.every_chunk_independently_decodable);
        let explanation = explain(&opened).unwrap();
        assert!(explanation.bounded_lookback_savings_bytes > 0);
        assert_eq!(
            explanation.physical_savings_bytes,
            explanation.ordinary_codec_savings_bytes
                + explanation.shared_dictionary_savings_bytes
                + explanation.bounded_lookback_savings_bytes
                + explanation.structural_transform_savings_bytes
        );
        assert!(
            opened
                .archive
                .content_store
                .chunks
                .values()
                .any(|chunk| chunk.group_ref.is_none())
        );
    }
    assert_eq!(balanced.identities.lai, dense.identities.lai);
    assert_eq!(dense.identities.lai, extreme.identities.lai);
    assert_eq!(balanced.identities.aux, dense.identities.aux);
    assert_eq!(dense.identities.aux, extreme.identities.aux);
    assert_eq!(
        balanced.archive.descriptor.chunker_id,
        dense.archive.descriptor.chunker_id
    );
    assert_eq!(balanced.identities.pcr, dense.identities.pcr);
    assert_eq!(
        balanced
            .archive
            .content_store
            .objects
            .keys()
            .collect::<Vec<_>>(),
        dense
            .archive
            .content_store
            .objects
            .keys()
            .collect::<Vec<_>>()
    );
    assert_ne!(balanced.identities.pci, dense.identities.pci);
    let required = dense.archive.descriptor.decode;
    assert!(required.working_set_bytes > 4 * 1024 * 1024);
    let refusal = open_with_limits(
        &dense.bytes,
        bootstrap_resource_policy(),
        DecodeRequirements {
            window_bytes: required.window_bytes,
            working_set_bytes: required.working_set_bytes - 1,
            flags: required.flags,
        },
    )
    .unwrap_err();
    assert_eq!(refusal.class(), OutcomeClass::PolicyRefused);
    assert_eq!(refusal.code(), ReasonCode::ResourceLimit);

    let destination = fixture.path.join("restored");
    unpack(&dense.bytes, &destination, ExtractionPolicy::default()).unwrap();
    assert_file_trees_equal(&source, &destination);

    let mut bad_reference = dense.archive.clone();
    let grouped = bad_reference
        .content_store
        .chunks
        .values_mut()
        .find(|chunk| chunk.group_ref.is_some())
        .unwrap();
    grouped.group_ref = Some(Digest::ZERO);
    assert_eq!(
        encode(&bad_reference, WriteOptions::default())
            .unwrap_err()
            .code(),
        ReasonCode::InvalidGroupReference
    );
    let mut bad_access = dense.archive.clone();
    bad_access
        .content_store
        .chunk_groups
        .values_mut()
        .next()
        .unwrap()
        .max_preceding_bytes += 1;
    assert_eq!(
        encode(&bad_access, WriteOptions::default())
            .unwrap_err()
            .code(),
        ReasonCode::AccessCostMismatch
    );
    let mut bad_order = dense.archive.clone();
    let group_id = *bad_order.content_store.chunk_groups.keys().next().unwrap();
    let independent_position = bad_order
        .content_store
        .physical_order
        .iter()
        .position(|chunk_id| bad_order.content_store.chunks[chunk_id].group_ref.is_none())
        .unwrap();
    let mut order = bad_order.content_store.physical_order.to_vec();
    let independent = order.remove(independent_position);
    let first_group_position = order
        .iter()
        .position(|chunk_id| bad_order.content_store.chunks[chunk_id].group_ref == Some(group_id))
        .unwrap();
    order.insert(first_group_position + 1, independent);
    bad_order.content_store.physical_order = order.into_boxed_slice();
    assert_eq!(
        encode(&bad_order, WriteOptions::default())
            .unwrap_err()
            .code(),
        ReasonCode::InvalidGroupOrdering
    );
    assert_prerequisite_corruption_fails(&dense.bytes, &dense.archive);
}

#[test]
fn small_cohort_stays_independent_and_v2_v3_remain_readable() {
    let fixture = Fixture::new("independent-v2");
    let source = fixture.path.join("source");
    fs::create_dir(&source).unwrap();
    let base = noise(64, 0x3c6e_f372_fe94_f82b);
    for index in 0..8 {
        let mut bytes = base.clone();
        bytes[63] ^= u8::try_from(index + 1).unwrap();
        fs::write(source.join(format!("small-{index}.bin")), bytes).unwrap();
    }
    let v3 = pack(&source, CompressionProfile::Balanced);
    let opened = open(&v3.bytes).unwrap();
    assert!(opened.archive.content_store.dictionaries.is_empty());
    assert!(opened.archive.content_store.chunk_groups.is_empty());
    let explanation = explain(&opened).unwrap();
    assert!(explanation.independent_similarity_cohort_count >= 1);
    assert!(explanation.independent_cohort_reason.is_some());
    let dense = pack(&source, CompressionProfile::Dense);
    assert!(dense.archive.content_store.chunk_groups.is_empty());
    assert!(dense.archive.content_store.dictionaries.is_empty());

    let mut historical_v3 = v3.archive.clone();
    reset_planning(&mut historical_v3);
    plan_archive_v3(&mut historical_v3, CompressionProfile::Balanced).unwrap();
    let encoded = encode(&historical_v3, WriteOptions::default()).unwrap();
    let reopened = open(&encoded.bytes).unwrap();
    assert_eq!(reopened.archive.descriptor.planner_id, "balanced-v3");
    assert_eq!(
        reopened.archive.descriptor.features.incompat,
        entrybound::ecf::FEATURE_CROSS_FILE_COMPRESSION_V1
    );
    verify(&encoded.bytes).unwrap();

    let mut v2 = v3.archive.clone();
    reset_planning(&mut v2);
    plan_archive_v2(&mut v2, CompressionProfile::Balanced).unwrap();
    let encoded = encode(&v2, WriteOptions::default()).unwrap();
    let reopened = open(&encoded.bytes).unwrap();
    assert_eq!(reopened.archive.descriptor.planner_id, "balanced-v2");
    assert_eq!(reopened.archive.descriptor.features.incompat, 0);
    verify(&encoded.bytes).unwrap();
}

fn reset_planning(archive: &mut entrybound::eam::Archive) {
    archive.descriptor.features = FeatureSet::default();
    archive.content_store.dictionaries.clear();
    archive.content_store.chunk_groups.clear();
    archive.content_store.physical_order = archive
        .content_store
        .chunks
        .keys()
        .copied()
        .collect::<Vec<_>>()
        .into_boxed_slice();
    for chunk in archive.content_store.chunks.values_mut() {
        chunk.plan_ref = UNPLANNED_PLAN_ID;
        chunk.group_ref = None;
    }
    archive.transform_plans = Box::default();
}

fn pack(source: &Path, profile: CompressionProfile) -> entrybound::ecf::EncodedArchive {
    pack_directory(
        source,
        PackOptions {
            profile,
            ..PackOptions::default()
        },
    )
    .unwrap()
}

fn write_similar_source(source: &Path, count: usize, len: usize, seed: u64) {
    fs::create_dir(source).unwrap();
    let base = noise(len, seed);
    for index in 0..count {
        let mut bytes = base.clone();
        let first = 128 + (index * 977) % (len.saturating_sub(512).max(1));
        for (offset, byte) in bytes[first..(first + 256).min(len)].iter_mut().enumerate() {
            *byte = (index as u8).wrapping_mul(31).wrapping_add(offset as u8);
        }
        fs::write(source.join(format!("sample-{index:03}.bin")), bytes).unwrap();
    }
}

fn noise(len: usize, mut state: u64) -> Vec<u8> {
    (0..len)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state as u8
        })
        .collect()
}

fn assert_dictionary_corruption_fails(bytes: &[u8]) {
    let mut corrupt = bytes.to_vec();
    let (header, payload) = locate_section(&corrupt, 3);
    let last = payload.end - 1;
    corrupt[last] ^= 1;
    rehash_section(&mut corrupt, header, payload);
    assert_eq!(
        verify(&corrupt).unwrap_err().code(),
        ReasonCode::DictionaryDigestMismatch
    );
}

fn assert_prerequisite_corruption_fails(bytes: &[u8], archive: &entrybound::eam::Archive) {
    let group_id = archive
        .content_store
        .chunk_groups
        .keys()
        .next()
        .copied()
        .unwrap();
    let chunk_id = archive
        .content_store
        .physical_order
        .iter()
        .copied()
        .find(|chunk_id| archive.content_store.chunks[chunk_id].group_ref == Some(group_id))
        .unwrap();
    let mut corrupt = bytes.to_vec();
    let location = archive.index.chunks[&chunk_id];
    corrupt[usize::try_from(location.offset).unwrap() + 96] ^= 1;
    let (header, payload) = locate_section(&corrupt, chunk_section_kind(&corrupt));
    rehash_section(&mut corrupt, header, payload);
    assert_eq!(
        verify(&corrupt).unwrap_err().code(),
        ReasonCode::PrerequisiteChunkCorrupt
    );
}

fn locate_section(bytes: &[u8], wanted: u16) -> (Range<usize>, Range<usize>) {
    let mut cursor = PREAMBLE_LEN as usize;
    let footer = bytes.len() - FOOTER_LEN as usize;
    while cursor < footer {
        let id = u16::from_be_bytes(bytes[cursor + 4..cursor + 6].try_into().unwrap());
        let payload_len = usize::try_from(u64::from_be_bytes(
            bytes[cursor + 16..cursor + 24].try_into().unwrap(),
        ))
        .unwrap();
        let payload_start = cursor + SECTION_HEADER_LEN as usize;
        let payload_end = payload_start + payload_len;
        if id == wanted {
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

fn rehash_section(bytes: &mut [u8], header: Range<usize>, payload: Range<usize>) {
    let digest = sha256_exact(&bytes[payload]);
    bytes[header.start + 24..header.start + 56].copy_from_slice(digest.as_bytes());
}

fn assert_file_trees_equal(source: &Path, destination: &Path) {
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        assert_eq!(
            fs::read(entry.path()).unwrap(),
            fs::read(destination.join(entry.file_name())).unwrap()
        );
    }
}

struct Fixture {
    path: PathBuf,
}

impl Fixture {
    fn new(label: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "entrybound-cross-file-{label}-{}-{}",
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
