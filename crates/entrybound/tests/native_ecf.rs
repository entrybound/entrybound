use std::collections::BTreeMap;
use std::ops::Range;

use entrybound::diagnostics::{OutcomeClass, ReasonCode};
use entrybound::eam::{
    Archive, ArchiveDescriptor, ArchiveRole, ContentRef, ContentStore, DecodeRequirements, Digest,
    DigestAlgorithm, Entry, EntryData, EntryIdentity, EntrySet, FeatureSet, FidelityReport,
    IdentityProfile, Index, Layout, LogicalPath, MetadataItem, MetadataSet, ResourceBudget,
    Timestamp, TimestampPrecision, TransformPlan,
};
use entrybound::ecf::{
    FOOTER_LEN, IndexStatus, PREAMBLE_LEN, SECTION_HEADER_LEN, WriteOptions, encode, open, verify,
};
use entrybound::identity::{
    BOOTSTRAP_CHUNK_SIZE, STORE_CODEC_IDENTIFIER, STORE_PLAN_ID, STORE_PLAN_IDENTIFIER,
    build_content, sha256_exact,
};

#[test]
fn empty_archive_round_trips() {
    assert_round_trip(empty_archive());
}

#[test]
fn complete_in_memory_fixture_round_trips_and_is_deterministic() {
    let archive = complete_fixture(17, true, BOOTSTRAP_CHUNK_SIZE, "fixed-1mib/v1");
    let first = encode(&archive, WriteOptions::default()).unwrap();
    let second = encode(&archive, WriteOptions::default()).unwrap();
    assert_eq!(first.bytes, second.bytes);
    assert_eq!(first.identities, second.identities);

    let opened = open(&first.bytes).unwrap();
    assert_eq!(opened.archive, first.archive);
    assert_eq!(opened.report.identities, first.identities);
    assert_eq!(opened.report.index_status, IndexStatus::PresentValid);
    assert!(opened.report.canonical_encoding);
    assert!(opened.report.chunk_integrity);
    assert!(opened.report.pci_computed);
    assert_eq!(opened.archive.content_store.objects.len(), 5);
    assert_eq!(
        opened
            .archive
            .entry_set
            .entries()
            .iter()
            .filter(|entry| matches!(entry.data(), EntryData::File { .. }))
            .count(),
        6
    );
}

#[test]
fn logical_and_auxiliary_identity_change_for_the_right_reasons() {
    let baseline = encode(
        &complete_fixture(17, false, BOOTSTRAP_CHUNK_SIZE, "fixed-1mib/v1"),
        WriteOptions::default(),
    )
    .unwrap();
    let mtime = encode(
        &complete_fixture(18, false, BOOTSTRAP_CHUNK_SIZE, "fixed-1mib/v1"),
        WriteOptions::default(),
    )
    .unwrap();
    assert_eq!(baseline.identities.lai, mtime.identities.lai);
    assert_ne!(baseline.identities.aux, mtime.identities.aux);

    let executable = encode(
        &complete_fixture(17, true, BOOTSTRAP_CHUNK_SIZE, "fixed-1mib/v1"),
        WriteOptions::default(),
    )
    .unwrap();
    assert_ne!(baseline.identities.lai, executable.identities.lai);

    let mut changed = complete_fixture(17, false, BOOTSTRAP_CHUNK_SIZE, "fixed-1mib/v1");
    replace_text_content(&mut changed, b"different text");
    let changed = encode(&changed, WriteOptions::default()).unwrap();
    assert_ne!(baseline.identities.lai, changed.identities.lai);
}

#[test]
fn rechunking_preserves_logical_identity_and_changes_pcr() {
    let one_mib = encode(
        &complete_fixture(17, true, BOOTSTRAP_CHUNK_SIZE, "fixed-1mib/v1"),
        WriteOptions::default(),
    )
    .unwrap();
    let half_mib = encode(
        &complete_fixture(17, true, BOOTSTRAP_CHUNK_SIZE / 2, "fixed-512k/test-v1"),
        WriteOptions::default(),
    )
    .unwrap();
    assert_eq!(one_mib.identities.lai, half_mib.identities.lai);
    assert_eq!(one_mib.identities.aux, half_mib.identities.aux);
    assert_ne!(one_mib.identities.pcr, half_mib.identities.pcr);
}

#[test]
fn index_is_optional_and_never_changes_semantics() {
    let archive = complete_fixture(17, true, BOOTSTRAP_CHUNK_SIZE, "fixed-1mib/v1");
    let indexed = encode(&archive, WriteOptions::default()).unwrap();
    let no_index = encode(
        &archive,
        WriteOptions {
            include_index: false,
        },
    )
    .unwrap();
    let indexed_opened = open(&indexed.bytes).unwrap();
    let no_index_opened = open(&no_index.bytes).unwrap();
    assert_eq!(
        no_index_opened.report.index_status,
        IndexStatus::RebuiltAbsent
    );
    assert_eq!(
        no_index_opened.report.index_reason,
        Some(ReasonCode::IndexAbsentRebuilt)
    );
    assert_eq!(indexed.identities.lai, no_index.identities.lai);
    assert_eq!(indexed.identities.pcr, no_index.identities.pcr);
    assert_eq!(indexed.identities.aux, no_index.identities.aux);
    assert_ne!(indexed.identities.pci, no_index.identities.pci);
    assert_semantic_entries_equal(&indexed_opened.archive, &no_index_opened.archive);

    let mut misleading = indexed.bytes.clone();
    mutate_index_offset(&mut misleading);
    let misleading_opened = open(&misleading).unwrap();
    assert_eq!(
        misleading_opened.report.index_status,
        IndexStatus::RebuiltInvalid
    );
    assert_eq!(
        misleading_opened.report.index_reason,
        Some(ReasonCode::IndexInvalidRebuilt)
    );
    assert_eq!(
        indexed.identities.lai,
        misleading_opened.report.identities.lai
    );
    assert_eq!(
        indexed.identities.pcr,
        misleading_opened.report.identities.pcr
    );
    assert_eq!(
        indexed.identities.aux,
        misleading_opened.report.identities.aux
    );
    assert_semantic_entries_equal(&indexed_opened.archive, &misleading_opened.archive);
}

#[test]
fn generated_native_conformance_corpus() {
    let valid = encode(
        &complete_fixture(17, true, BOOTSTRAP_CHUNK_SIZE, "fixed-1mib/v1"),
        WriteOptions::default(),
    )
    .unwrap()
    .bytes;

    let cases = [
        invalid_case(
            "ECF-F001-bad-magic",
            &valid,
            ReasonCode::BadMagic,
            |bytes| {
                bytes[0] ^= 0x01;
            },
        ),
        invalid_case(
            "ECF-F002-unsupported-version",
            &valid,
            ReasonCode::UnsupportedVersion,
            |bytes| bytes[8..10].copy_from_slice(&1_u16.to_be_bytes()),
        ),
        invalid_case(
            "ECF-F003-required-feature",
            &valid,
            ReasonCode::UnsupportedRequiredFeature,
            |bytes| {
                bytes[16..24].copy_from_slice(&(1_u64 << 63).to_be_bytes());
                let checksum = sha256_exact(&bytes[16..40]);
                bytes[40..72].copy_from_slice(checksum.as_bytes());
                let preamble_digest = sha256_exact(&bytes[..PREAMBLE_LEN as usize]);
                let footer = bytes.len() - FOOTER_LEN as usize;
                bytes[footer + 64..footer + 96].copy_from_slice(preamble_digest.as_bytes());
            },
        ),
        invalid_case(
            "ECF-F004-incorrect-total-length",
            &valid,
            ReasonCode::IncorrectTotalLength,
            |bytes| {
                let footer = bytes.len() - usize::try_from(FOOTER_LEN).unwrap();
                let wrong = u64::try_from(bytes.len()).unwrap() + 1;
                bytes[footer + 8..footer + 16].copy_from_slice(&wrong.to_be_bytes());
            },
        ),
        invalid_case(
            "ECF-F005-malformed-record-length",
            &valid,
            ReasonCode::SectionStructure,
            |bytes| {
                let (_, payload) = locate_section(bytes, 1);
                bytes[payload.start + 8..payload.start + 16]
                    .copy_from_slice(&u64::MAX.to_be_bytes());
                rehash_section(bytes, 1);
            },
        ),
        invalid_case(
            "ECF-F006-out-of-order-fields",
            &valid,
            ReasonCode::NoncanonicalEncoding,
            |bytes| {
                let (_, payload) = locate_section(bytes, 1);
                let first = field_header(bytes, payload.start, 1);
                let second = field_header(bytes, payload.start, 2);
                bytes[first..first + 2].copy_from_slice(&2_u16.to_be_bytes());
                bytes[second..second + 2].copy_from_slice(&1_u16.to_be_bytes());
                rehash_section(bytes, 1);
            },
        ),
        invalid_case(
            "ECF-F007-duplicate-field",
            &valid,
            ReasonCode::DuplicateSemanticDeclaration,
            |bytes| {
                let (_, payload) = locate_section(bytes, 1);
                let second = field_header(bytes, payload.start, 2);
                bytes[second..second + 2].copy_from_slice(&1_u16.to_be_bytes());
                rehash_section(bytes, 1);
            },
        ),
        invalid_case(
            "ECF-F008-unknown-field",
            &valid,
            ReasonCode::NoncanonicalEncoding,
            |bytes| {
                let (_, payload) = locate_section(bytes, 1);
                let eighth = field_header(bytes, payload.start, 8);
                bytes[eighth..eighth + 2].copy_from_slice(&9_u16.to_be_bytes());
                rehash_section(bytes, 1);
            },
        ),
        invalid_case(
            "ECF-F009-section-digest",
            &valid,
            ReasonCode::SectionDigestMismatch,
            |bytes| {
                let (_, payload) = locate_section(bytes, 5);
                bytes[payload.start] ^= 1;
            },
        ),
        invalid_case(
            "ECF-F010-chunk-digest",
            &valid,
            ReasonCode::ChunkDigestMismatch,
            |bytes| {
                let (_, payload) = locate_section(bytes, 3);
                bytes[payload.start + 16] ^= 1;
                rehash_section(bytes, 3);
            },
        ),
        descriptor_digest_case("ECF-F011-lai", &valid, 6, ReasonCode::LaiMismatch),
        descriptor_digest_case("ECF-F012-pcr", &valid, 7, ReasonCode::PcrMismatch),
        descriptor_digest_case("ECF-F013-aux", &valid, 8, ReasonCode::AuxMismatch),
        manifest_entry_digest_case(
            "ECF-F014-entry-identity",
            &valid,
            6,
            ReasonCode::EntryIdentityMismatch,
        ),
        manifest_entry_digest_case(
            "ECF-F015-entry-aux",
            &valid,
            7,
            ReasonCode::EntryAuxMismatch,
        ),
    ];

    for case in cases {
        let error = open(&case.bytes).unwrap_err();
        assert_eq!(error.code(), case.expected, "{}", case.id);
    }

    let truncated = &valid[..valid.len() - 1];
    let error = open(truncated).unwrap_err();
    assert_eq!(error.code(), ReasonCode::TruncatedFooter);

    let mut invalid_index = valid;
    mutate_index_offset(&mut invalid_index);
    let report = verify(&invalid_index).unwrap();
    assert_eq!(report.index_reason, Some(ReasonCode::IndexInvalidRebuilt));
}

fn assert_round_trip(archive: Archive) {
    let encoded = encode(&archive, WriteOptions::default()).unwrap();
    let opened = open(&encoded.bytes).unwrap();
    assert_eq!(opened.archive, encoded.archive);
    assert_eq!(verify(&encoded.bytes).unwrap(), opened.report);
}

fn empty_archive() -> Archive {
    Archive {
        descriptor: descriptor("fixed-1mib/v1"),
        entry_set: EntrySet::default(),
        content_store: ContentStore::default(),
        transform_plans: vec![store_plan()].into_boxed_slice(),
        fidelity: FidelityReport {
            platform: "test".to_owned(),
            ..FidelityReport::default()
        },
        conversion: None,
        preservation: None,
        index: Index::default(),
    }
}

fn complete_fixture(
    mtime_seconds: i64,
    executable: bool,
    chunk_size: usize,
    chunker_id: &str,
) -> Archive {
    let timestamp =
        Timestamp::new(mtime_seconds, 123, TimestampPrecision::Nanosecond, true).unwrap();
    let mut objects = BTreeMap::new();
    let mut chunks = BTreeMap::new();
    let mut entries = vec![
        directory(&["bin"], timestamp),
        directory(&["nested"], timestamp),
        directory(&["nested", "deep"], timestamp),
        directory(&["empty-dir"], timestamp),
    ];
    add_file(
        &mut entries,
        &mut objects,
        &mut chunks,
        &["empty"],
        &[],
        false,
        timestamp,
        chunk_size,
    );
    add_file(
        &mut entries,
        &mut objects,
        &mut chunks,
        &["nested", "hello.txt"],
        b"hello Entrybound\n",
        false,
        timestamp,
        chunk_size,
    );
    add_file(
        &mut entries,
        &mut objects,
        &mut chunks,
        &["repeat-a"],
        b"same bytes",
        false,
        timestamp,
        chunk_size,
    );
    add_file(
        &mut entries,
        &mut objects,
        &mut chunks,
        &["repeat-b"],
        b"same bytes",
        false,
        timestamp,
        chunk_size,
    );
    add_file(
        &mut entries,
        &mut objects,
        &mut chunks,
        &["binary"],
        &[0, 255, 17, 128, 0, 42],
        false,
        timestamp,
        chunk_size,
    );
    let large = (0..BOOTSTRAP_CHUNK_SIZE + 137)
        .map(|index| u8::try_from(index % 251).unwrap())
        .collect::<Vec<_>>();
    add_file(
        &mut entries,
        &mut objects,
        &mut chunks,
        &["bin", "tool"],
        &large,
        executable,
        timestamp,
        chunk_size,
    );
    Archive {
        descriptor: descriptor(chunker_id),
        entry_set: EntrySet::new(entries).unwrap(),
        content_store: ContentStore {
            physical_order: chunks
                .keys()
                .copied()
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            objects,
            chunks,
            dictionaries: BTreeMap::new(),
            reconstruction_data: BTreeMap::new(),
            reconstruction_fallbacks: BTreeMap::new(),
            reconstruction_regions: BTreeMap::new(),
            reconstruction_audits: BTreeMap::new(),
            chunk_groups: BTreeMap::new(),
        },
        transform_plans: vec![store_plan()].into_boxed_slice(),
        fidelity: FidelityReport {
            captured: vec!["core.executable".to_owned(), "core.mtime".to_owned()]
                .into_boxed_slice(),
            platform: "test".to_owned(),
            ..FidelityReport::default()
        },
        conversion: None,
        preservation: None,
        index: Index::default(),
    }
}

fn descriptor(chunker_id: &str) -> ArchiveDescriptor {
    ArchiveDescriptor {
        format_major: 0,
        format_minor: 1,
        format_namespace: "ecf/bootstrap-v1".to_owned(),
        features: FeatureSet::default(),
        layout: Layout::Indexed,
        role: ArchiveRole::Complete,
        budget_declared: true,
        stream_dedup_window: 0,
        budget: ResourceBudget::default(),
        decode: DecodeRequirements::default(),
        identity_profile: IdentityProfile::IdentityV1,
        digest_algorithm: DigestAlgorithm::Sha256,
        planner_id: STORE_PLAN_IDENTIFIER.to_owned(),
        chunker_id: chunker_id.to_owned(),
        lai: Digest::ZERO,
        pcr: Digest::ZERO,
        aux: Digest::ZERO,
        pci: None,
    }
}

fn store_plan() -> TransformPlan {
    TransformPlan {
        plan_id: STORE_PLAN_ID,
        identifier: STORE_PLAN_IDENTIFIER.to_owned(),
        transforms: Box::default(),
        codec: STORE_CODEC_IDENTIFIER.to_owned(),
        codec_params: Box::default(),
        dictionary: None,
        decode: DecodeRequirements::default(),
    }
}

fn directory(path: &[&str], timestamp: Timestamp) -> Entry {
    Entry::new(
        LogicalPath::from_utf8(path).unwrap(),
        EntryData::Directory,
        MetadataSet::new(vec![MetadataItem::mtime(timestamp)]).unwrap(),
        EntryIdentity::default(),
    )
}

#[allow(clippy::too_many_arguments)]
fn add_file(
    entries: &mut Vec<Entry>,
    objects: &mut BTreeMap<Digest, entrybound::eam::ContentObject>,
    chunks: &mut BTreeMap<Digest, entrybound::eam::Chunk>,
    path: &[&str],
    content: &[u8],
    executable: bool,
    timestamp: Timestamp,
    chunk_size: usize,
) {
    let (object, object_chunks) = build_content(content, chunk_size, STORE_PLAN_ID).unwrap();
    let digest = object.logical_digest;
    objects.entry(digest).or_insert(object);
    for (chunk_id, chunk) in object_chunks {
        chunks.entry(chunk_id).or_insert(chunk);
    }
    entries.push(Entry::new(
        LogicalPath::from_utf8(path).unwrap(),
        EntryData::File {
            content: ContentRef::Internal(digest),
        },
        MetadataSet::new(vec![
            MetadataItem::executable(executable),
            MetadataItem::mtime(timestamp),
        ])
        .unwrap(),
        EntryIdentity::default(),
    ));
}

fn replace_text_content(archive: &mut Archive, replacement: &[u8]) {
    let path = LogicalPath::from_utf8(["nested", "hello.txt"]).unwrap();
    let timestamp = archive
        .entry_set
        .entries()
        .iter()
        .find(|entry| entry.path() == &path)
        .unwrap()
        .metadata()
        .mtime()
        .unwrap();
    let mut entries = archive.entry_set.entries().to_vec();
    let index = entries
        .iter()
        .position(|entry| entry.path() == &path)
        .unwrap();
    let (object, chunks) = build_content(replacement, BOOTSTRAP_CHUNK_SIZE, STORE_PLAN_ID).unwrap();
    let digest = object.logical_digest;
    archive.content_store.objects.insert(digest, object);
    archive.content_store.chunks.extend(chunks);
    archive.content_store.physical_order = archive
        .content_store
        .chunks
        .keys()
        .copied()
        .collect::<Vec<_>>()
        .into_boxed_slice();
    entries[index] = Entry::new(
        path,
        EntryData::File {
            content: ContentRef::Internal(digest),
        },
        MetadataSet::new(vec![
            MetadataItem::executable(false),
            MetadataItem::mtime(timestamp),
        ])
        .unwrap(),
        EntryIdentity::default(),
    );
    archive.entry_set = EntrySet::from_canonical(entries).unwrap();
}

fn assert_semantic_entries_equal(left: &Archive, right: &Archive) {
    assert_eq!(left.entry_set, right.entry_set);
    assert_eq!(left.content_store, right.content_store);
    assert_eq!(left.transform_plans, right.transform_plans);
    assert_eq!(left.fidelity, right.fidelity);
}

struct InvalidCase {
    id: &'static str,
    bytes: Vec<u8>,
    expected: ReasonCode,
}

fn invalid_case(
    id: &'static str,
    valid: &[u8],
    expected: ReasonCode,
    mutate: impl FnOnce(&mut Vec<u8>),
) -> InvalidCase {
    let mut bytes = valid.to_vec();
    mutate(&mut bytes);
    InvalidCase {
        id,
        bytes,
        expected,
    }
}

fn descriptor_digest_case(
    id: &'static str,
    valid: &[u8],
    tag: u16,
    expected: ReasonCode,
) -> InvalidCase {
    invalid_case(id, valid, expected, |bytes| {
        let (_, payload) = locate_section(bytes, 1);
        let value = field_value(bytes, payload.start, tag);
        bytes[value.start] ^= 1;
        rehash_section(bytes, 1);
    })
}

fn manifest_entry_digest_case(
    id: &'static str,
    valid: &[u8],
    tag: u16,
    expected: ReasonCode,
) -> InvalidCase {
    invalid_case(id, valid, expected, |bytes| {
        let (_, payload) = locate_section(bytes, 4);
        let value = field_value(bytes, payload.start, tag);
        bytes[value.start] ^= 1;
        rehash_section(bytes, 4);
    })
}

fn mutate_index_offset(bytes: &mut [u8]) {
    let (_, payload) = locate_section(bytes, 6);
    let value = field_value(bytes, payload.start, 2);
    let current = u64::from_be_bytes(bytes[value.clone()].try_into().unwrap());
    bytes[value].copy_from_slice(&(current + 1).to_be_bytes());
    rehash_section(bytes, 6);
}

fn locate_section(bytes: &[u8], wanted: u16) -> (Range<usize>, Range<usize>) {
    let mut cursor = usize::try_from(PREAMBLE_LEN).unwrap();
    let footer = bytes.len() - usize::try_from(FOOTER_LEN).unwrap();
    while cursor < footer {
        assert_eq!(&bytes[cursor..cursor + 4], b"EBS1");
        let kind = u16::from_be_bytes(bytes[cursor + 4..cursor + 6].try_into().unwrap());
        let payload_len = usize::try_from(u64::from_be_bytes(
            bytes[cursor + 16..cursor + 24].try_into().unwrap(),
        ))
        .unwrap();
        let payload_start = cursor + usize::try_from(SECTION_HEADER_LEN).unwrap();
        let payload_end = payload_start + payload_len;
        if kind == wanted {
            return (cursor..payload_start, payload_start..payload_end);
        }
        cursor = payload_end;
    }
    panic!("section {wanted} not found");
}

fn rehash_section(bytes: &mut [u8], section: u16) {
    let (header, payload) = locate_section(bytes, section);
    let digest = sha256_exact(&bytes[payload]);
    bytes[header.start + 24..header.start + 56].copy_from_slice(digest.as_bytes());
}

fn field_header(bytes: &[u8], record_start: usize, wanted: u16) -> usize {
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
        if tag == wanted {
            return cursor;
        }
        let len = usize::try_from(u64::from_be_bytes(
            bytes[cursor + 4..cursor + 12].try_into().unwrap(),
        ))
        .unwrap();
        cursor += 12 + len;
    }
    panic!("field {wanted} not found");
}

fn field_value(bytes: &[u8], record_start: usize, wanted: u16) -> Range<usize> {
    let header = field_header(bytes, record_start, wanted);
    let len = usize::try_from(u64::from_be_bytes(
        bytes[header + 4..header + 12].try_into().unwrap(),
    ))
    .unwrap();
    header + 12..header + 12 + len
}

#[test]
fn diagnostic_classes_remain_distinguishable() {
    let encoded = encode(&empty_archive(), WriteOptions::default()).unwrap();
    let mut unsupported = encoded.bytes.clone();
    unsupported[8..10].copy_from_slice(&1_u16.to_be_bytes());
    assert_eq!(
        open(&unsupported).unwrap_err().class(),
        OutcomeClass::Unsupported
    );

    let truncated = &encoded.bytes[..encoded.bytes.len() - 1];
    assert_eq!(
        open(truncated).unwrap_err().class(),
        OutcomeClass::Truncated
    );
}
