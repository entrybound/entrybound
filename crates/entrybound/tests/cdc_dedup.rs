use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use entrybound::archive::{
    ExtractionPolicy, PackOptions, bootstrap_resource_policy, explain, inspect, list,
    pack_directory, unpack,
};
use entrybound::chunker::{
    BALANCED_V2, ChunkingParameters, DENSE_V2, chunk_ranges, select_parameters,
};
use entrybound::diagnostics::{OutcomeClass, ReasonCode};
use entrybound::eam::{
    Archive, ArchiveDescriptor, ArchiveRole, ContentRef, ContentStore, DecodeRequirements, Digest,
    DigestAlgorithm, Entry, EntryData, EntryIdentity, EntrySet, FeatureSet, FidelityReport,
    IdentityProfile, Index, Layout, LogicalPath, MetadataSet, ResourceBudget, TransformPlan,
};
use entrybound::ecf::{
    FOOTER_LEN, PREAMBLE_LEN, SECTION_HEADER_LEN, WriteOptions, encode, open, open_with_policy,
    verify,
};
use entrybound::identity::{
    BOOTSTRAP_CHUNK_SIZE, STORE_CODEC_IDENTIFIER, STORE_PLAN_ID, STORE_PLAN_IDENTIFIER,
    apply_native_identities, build_content, build_content_from_ranges, sha256_exact,
};
use entrybound::planner::{CompressionProfile, UNPLANNED_PLAN_ID, plan_archive};

static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

#[test]
fn cdc_resynchronizes_after_an_insertion_that_shifts_fixed_boundaries() {
    let original = deterministic_noise(12 * 1024 * 1024);
    let mut inserted = Vec::with_capacity(original.len() + 4096);
    inserted.extend_from_slice(&original[..64 * 1024]);
    inserted.extend_from_slice(&vec![0xa5; 4096]);
    inserted.extend_from_slice(&original[64 * 1024..]);

    let fixed_original = fixed_chunk_ids(&original, BALANCED_V2.target_size);
    let fixed_inserted = fixed_chunk_ids(&inserted, BALANCED_V2.target_size);
    let cdc_original = cdc_chunk_ids(&original, BALANCED_V2);
    let cdc_inserted = cdc_chunk_ids(&inserted, BALANCED_V2);
    let fixed_matches = intersection_count(&fixed_original, &fixed_inserted);
    let cdc_matches = intersection_count(&cdc_original, &cdc_inserted);

    assert!(
        cdc_matches >= 3,
        "CDC did not recover substantial downstream chunks"
    );
    assert!(cdc_matches > fixed_matches);
}

#[test]
fn chunking_changes_pcr_but_not_content_or_logical_identity() {
    let content = deterministic_noise(5 * 1024 * 1024 + 137);
    let fixed = build_content(&content, BOOTSTRAP_CHUNK_SIZE, STORE_PLAN_ID).unwrap();
    let balanced_ranges = ranges(&content, BALANCED_V2);
    let dense_ranges = ranges(&content, DENSE_V2);
    let balanced = build_content_from_ranges(&content, &balanced_ranges, STORE_PLAN_ID).unwrap();
    let dense = build_content_from_ranges(&content, &dense_ranges, STORE_PLAN_ID).unwrap();

    let fixed_archive = archive_with_content(fixed, "fixed-1mib/v1", "balanced-v1");
    let balanced_archive = archive_with_content(
        balanced,
        BALANCED_V2.chunker_id,
        CompressionProfile::Balanced.planner_id(),
    );
    let dense_archive = archive_with_content(
        dense,
        DENSE_V2.chunker_id,
        CompressionProfile::Dense.planner_id(),
    );
    let fixed_roots = apply_native_identities(&fixed_archive).unwrap().1;
    let balanced_roots = apply_native_identities(&balanced_archive).unwrap().1;
    let dense_roots = apply_native_identities(&dense_archive).unwrap().1;

    assert_eq!(
        fixed_archive.content_store.objects.keys().next(),
        balanced_archive.content_store.objects.keys().next()
    );
    assert_eq!(fixed_roots.lai, balanced_roots.lai);
    assert_eq!(balanced_roots.lai, dense_roots.lai);
    assert_eq!(fixed_roots.aux, balanced_roots.aux);
    assert_eq!(balanced_roots.aux, dense_roots.aux);
    assert_ne!(fixed_roots.pcr, balanced_roots.pcr);
    assert_ne!(balanced_roots.pcr, dense_roots.pcr);

    let fixed_bytes = encode(&fixed_archive, WriteOptions::default()).unwrap();
    let balanced_bytes = encode(&balanced_archive, WriteOptions::default()).unwrap();
    assert_ne!(fixed_bytes.identities.pci, balanced_bytes.identities.pci);
}

#[test]
fn archive_wide_dedup_is_exact_deterministic_and_extracts_all_references() {
    let fixture = Fixture::new("archive-wide");
    let source = fixture.path.join("source");
    fs::create_dir(&source).unwrap();
    let base = deterministic_noise(3 * 1024 * 1024);
    let mut inserted = Vec::with_capacity(base.len() + 8192);
    inserted.extend_from_slice(&base[..96 * 1024]);
    inserted.extend_from_slice(&vec![0x3c; 8192]);
    inserted.extend_from_slice(&base[96 * 1024..]);
    fs::write(source.join("base-a.bin"), &base).unwrap();
    fs::write(source.join("base-b.bin"), &base).unwrap();
    fs::write(source.join("inserted.bin"), &inserted).unwrap();
    fs::write(source.join("repeated.bin"), vec![0_u8; 4 * 1024 * 1024]).unwrap();
    fs::write(source.join("compressible.txt"), vec![b'e'; 1024 * 1024]).unwrap();
    for index in 0..24 {
        fs::write(
            source.join(format!("small-{index:02}.txt")),
            b"same small content",
        )
        .unwrap();
    }

    let first = pack_directory(&source, PackOptions::default()).unwrap();
    let second = pack_directory(&source, PackOptions::default()).unwrap();
    assert_eq!(first.bytes, second.bytes);
    let opened = open(&first.bytes).unwrap();
    verify(&first.bytes).unwrap();
    assert_eq!(opened.archive.descriptor.planner_id, "balanced-v5");
    assert_eq!(opened.archive.descriptor.chunker_id, BALANCED_V2.chunker_id);

    let view = inspect(&opened).unwrap();
    assert_eq!(
        view.chunks.unique_chunk_count,
        opened.archive.descriptor.budget.chunk_count
    );
    assert!(view.chunks.logical_chunk_references > view.chunks.unique_chunk_count);
    assert!(view.chunks.deduplicated_bytes > base.len() as u64);
    assert!(view.chunks.maximum_chunk_bytes <= BALANCED_V2.maximum_size as u64);
    assert!(
        view.codec_usage
            .iter()
            .any(|usage| usage.codec == "store/v1")
    );
    assert!(
        view.codec_usage
            .iter()
            .any(|usage| usage.codec == "zstandard/v1")
    );
    let explanation = explain(&opened).unwrap();
    assert_eq!(explanation.chunks, view.chunks);
    assert!(explanation.chunks.dedup_ratio_milli > 1_000);
    assert!(explanation.physical_savings_bytes > 0);

    let base_object = object_for_path(&opened.archive, "base-a.bin");
    let duplicate_object = object_for_path(&opened.archive, "base-b.bin");
    assert_eq!(base_object, duplicate_object);
    assert!(base_object.chunks.len() > 2);
    let repeated_object = object_for_path(&opened.archive, "repeated.bin");
    assert!(
        repeated_object
            .chunks
            .iter()
            .map(|reference| reference.chunk_id)
            .collect::<BTreeSet<_>>()
            .len()
            < repeated_object.chunks.len()
    );

    let mut restrictive = bootstrap_resource_policy();
    restrictive.total_logical_bytes = view.chunks.unique_plaintext_bytes;
    let error = open_with_policy(&first.bytes, restrictive).unwrap_err();
    assert_eq!(error.class(), OutcomeClass::PolicyRefused);
    assert_eq!(error.code(), ReasonCode::ResourceLimit);

    let destination = fixture.path.join("restored");
    unpack(&first.bytes, &destination, ExtractionPolicy::default()).unwrap();
    assert_tree_files_equal(&source, &destination);

    assert_shared_store_chunk_corruption_is_detected(&first.bytes, &opened.archive);
}

#[test]
fn denser_chunking_is_selected_only_when_exact_dedup_beats_overhead() {
    let base = deterministic_noise(4 * 1024 * 1024);
    let mut variants = Vec::new();
    for insertion in [32 * 1024, 64 * 1024, 96 * 1024, 128 * 1024] {
        let mut variant = Vec::with_capacity(base.len() + insertion);
        variant.extend_from_slice(&base[..48 * 1024]);
        variant.extend_from_slice(&vec![0x7d; insertion]);
        variant.extend_from_slice(&base[48 * 1024..]);
        variants.push(variant);
    }
    let contents = std::iter::once(base.as_slice())
        .chain(variants.iter().map(Vec::as_slice))
        .collect::<Vec<_>>();
    let selected = select_parameters(&contents, &[BALANCED_V2, DENSE_V2]).unwrap();
    let balanced = entrybound::chunker::evaluate(&contents, BALANCED_V2).unwrap();
    let dense = entrybound::chunker::evaluate(&contents, DENSE_V2).unwrap();
    assert_eq!(selected.parameters, DENSE_V2);
    assert!(dense.estimated_cost_bytes < balanced.estimated_cost_bytes);
}

#[test]
fn historical_fixed_chunk_v1_archive_still_opens_and_verifies() {
    let fixture = Fixture::new("fixed-v1");
    let content = deterministic_noise(BOOTSTRAP_CHUNK_SIZE * 2 + 17);
    let (object, chunks) =
        build_content(&content, BOOTSTRAP_CHUNK_SIZE, UNPLANNED_PLAN_ID).unwrap();
    let mut archive = archive_with_content(
        (object, chunks),
        "fixed-1mib/v1",
        CompressionProfile::Balanced.planner_v1_id(),
    );
    archive.transform_plans = Box::default();
    plan_archive(&mut archive, CompressionProfile::Balanced).unwrap();
    assert_eq!(archive.descriptor.planner_id, "balanced-v1");
    let encoded = encode(&archive, WriteOptions::default()).unwrap();
    let opened = open(&encoded.bytes).unwrap();
    assert_eq!(opened.archive.descriptor.chunker_id, "fixed-1mib/v1");
    assert_eq!(opened.archive.descriptor.planner_id, "balanced-v1");
    verify(&encoded.bytes).unwrap();
    assert_eq!(inspect(&opened).unwrap().chunker_id, "fixed-1mib/v1");
    assert_eq!(list(&opened.archive).unwrap()[0].path, "file");
    let reconstructed = opened
        .archive
        .content_store
        .objects
        .values()
        .next()
        .unwrap()
        .chunks
        .iter()
        .flat_map(|reference| {
            opened.archive.content_store.chunks[&reference.chunk_id]
                .plaintext
                .iter()
                .copied()
        })
        .collect::<Vec<_>>();
    assert_eq!(reconstructed, content);
    let destination = fixture.path.join("restored");
    unpack(&encoded.bytes, &destination, ExtractionPolicy::default()).unwrap();
    assert_eq!(fs::read(destination.join("file")).unwrap(), content);
}

fn archive_with_content(
    (object, chunks): (
        entrybound::eam::ContentObject,
        BTreeMap<Digest, entrybound::eam::Chunk>,
    ),
    chunker_id: &str,
    planner_id: &str,
) -> Archive {
    let logical_digest = object.logical_digest;
    Archive {
        descriptor: ArchiveDescriptor {
            format_major: 0,
            format_minor: 1,
            format_namespace: entrybound::ecf::FORMAT_NAMESPACE.to_owned(),
            features: FeatureSet::default(),
            layout: Layout::Indexed,
            role: ArchiveRole::Complete,
            budget_declared: true,
            budget: ResourceBudget::default(),
            decode: DecodeRequirements::default(),
            identity_profile: IdentityProfile::IdentityV1,
            digest_algorithm: DigestAlgorithm::Sha256,
            planner_id: planner_id.to_owned(),
            chunker_id: chunker_id.to_owned(),
            lai: Digest::ZERO,
            pcr: Digest::ZERO,
            aux: Digest::ZERO,
            pci: None,
        },
        entry_set: EntrySet::new(vec![Entry::new(
            LogicalPath::from_utf8(["file"]).unwrap(),
            EntryData::File {
                content: ContentRef::Internal(logical_digest),
            },
            MetadataSet::default(),
            EntryIdentity::default(),
        )])
        .unwrap(),
        content_store: ContentStore {
            physical_order: chunks
                .keys()
                .copied()
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            objects: BTreeMap::from([(logical_digest, object)]),
            chunks,
            dictionaries: BTreeMap::new(),
            reconstruction_data: BTreeMap::new(),
            chunk_groups: BTreeMap::new(),
        },
        transform_plans: vec![TransformPlan {
            plan_id: STORE_PLAN_ID,
            identifier: STORE_PLAN_IDENTIFIER.to_owned(),
            transforms: Box::default(),
            codec: STORE_CODEC_IDENTIFIER.to_owned(),
            codec_params: Box::default(),
            dictionary: None,
            decode: DecodeRequirements::default(),
        }]
        .into_boxed_slice(),
        fidelity: FidelityReport::default(),
        index: Index::default(),
    }
}

fn ranges(content: &[u8], parameters: ChunkingParameters) -> Vec<Range<usize>> {
    chunk_ranges(content, parameters)
        .unwrap()
        .iter()
        .map(|range| range.start..range.end)
        .collect()
}

fn fixed_chunk_ids(bytes: &[u8], size: usize) -> BTreeSet<Digest> {
    bytes.chunks(size).map(sha256_exact).collect()
}

fn cdc_chunk_ids(bytes: &[u8], parameters: ChunkingParameters) -> BTreeSet<Digest> {
    chunk_ranges(bytes, parameters)
        .unwrap()
        .iter()
        .map(|range| sha256_exact(&bytes[range.start..range.end]))
        .collect()
}

fn intersection_count(left: &BTreeSet<Digest>, right: &BTreeSet<Digest>) -> usize {
    left.intersection(right).count()
}

fn object_for_path<'a>(archive: &'a Archive, path: &str) -> &'a entrybound::eam::ContentObject {
    let entry = archive
        .entry_set
        .entries()
        .iter()
        .find(|entry| entry.path().to_string() == path)
        .unwrap();
    let EntryData::File {
        content: ContentRef::Internal(digest),
    } = entry.data()
    else {
        panic!("expected file")
    };
    &archive.content_store.objects[&digest]
}

fn assert_shared_store_chunk_corruption_is_detected(bytes: &[u8], archive: &Archive) {
    let plan_codecs = archive
        .transform_plans
        .iter()
        .map(|plan| (plan.plan_id, plan.codec.as_str()))
        .collect::<BTreeMap<_, _>>();
    let chunk_id = archive
        .content_store
        .chunks
        .values()
        .find(|chunk| plan_codecs[&chunk.plan_ref] == "store/v1")
        .unwrap()
        .chunk_id;
    let reference_count = archive
        .entry_set
        .entries()
        .iter()
        .filter_map(|entry| match entry.data() {
            EntryData::File {
                content: ContentRef::Internal(digest),
            } => Some(&archive.content_store.objects[&digest]),
            EntryData::Directory => None,
        })
        .flat_map(|object| object.chunks.iter())
        .filter(|reference| reference.chunk_id == chunk_id)
        .count();
    assert!(reference_count >= 2);

    let mut corrupt = bytes.to_vec();
    let location = archive.index.chunks[&chunk_id];
    let extended = archive.descriptor.features.incompat
        & entrybound::ecf::FEATURE_CROSS_FILE_COMPRESSION_V1
        != 0;
    let frame_header = if extended { 96 } else { 64 };
    corrupt[usize::try_from(location.offset).unwrap() + frame_header] ^= 1;
    rehash_section(&mut corrupt, if extended { 5 } else { 3 });
    let error = verify(&corrupt).unwrap_err();
    assert_eq!(error.code(), ReasonCode::ChunkDigestMismatch);
    assert!(error.detail().contains(&chunk_id.to_string()));
}

fn assert_tree_files_equal(source: &Path, destination: &Path) {
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            assert_eq!(
                fs::read(entry.path()).unwrap(),
                fs::read(destination.join(entry.file_name())).unwrap()
            );
        }
    }
}

fn deterministic_noise(len: usize) -> Vec<u8> {
    let mut state = 0x1f83_d9ab_fb41_bd6b_u64;
    (0..len)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state as u8
        })
        .collect()
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

fn rehash_section(bytes: &mut [u8], section: u16) {
    let (header, payload) = locate_section(bytes, section);
    let digest = sha256_exact(&bytes[payload]);
    bytes[header.start + 24..header.start + 56].copy_from_slice(digest.as_bytes());
}

struct Fixture {
    path: PathBuf,
}

impl Fixture {
    fn new(label: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "entrybound-cdc-{label}-{}-{}",
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
