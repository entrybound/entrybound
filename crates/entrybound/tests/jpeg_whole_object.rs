use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use entrybound::archive::{
    ExtractionPolicy, PackOptions, bootstrap_resource_policy, explain, inspect, pack_directory,
    unpack,
};
use entrybound::eam::{
    Archive, ArchiveDescriptor, ArchiveRole, ContentRef, ContentStore, DecodeRequirements, Digest,
    DigestAlgorithm, Entry, EntryData, EntryIdentity, EntrySet, FeatureSet, FidelityReport,
    IdentityProfile, Index, Layout, LogicalPath, MetadataSet, ResourceBudget, TransformPlan,
};
use entrybound::ecf::{WriteOptions, encode, open, open_with_limits, verify};
use entrybound::identity::{
    STORE_CODEC_IDENTIFIER, STORE_PLAN_ID, STORE_PLAN_IDENTIFIER, build_content, sha256_exact,
};
use entrybound::planner::{CompressionProfile, plan_archive_v5, plan_archive_v6};
use image::codecs::jpeg::JpegEncoder;
use image::{ExtendedColorType, ImageEncoder};

static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

#[test]
fn selected_multi_chunk_jpeg_region_is_exact_deterministic_and_identity_neutral() {
    let fixture = Fixture::new("multi-chunk");
    let original = generated_tiled_jpeg(1_024, 1_024, 96);
    let mut baseline = archive_for(&original, 64 * 1024);
    let shared_object = *baseline.content_store.objects.keys().next().unwrap();
    baseline.entry_set = EntrySet::new(vec![
        file_entry("image.jpg", shared_object),
        file_entry("copy.jpg", shared_object),
    ])
    .unwrap();
    let image_object = baseline
        .entry_set
        .entries()
        .iter()
        .find_map(|entry| match entry.data() {
            EntryData::File {
                content: ContentRef::Internal(digest),
            } => Some(digest),
            EntryData::Directory => None,
        })
        .unwrap();
    assert!(baseline.content_store.objects[&image_object].chunks.len() > 1);

    let mut ordinary = baseline.clone();
    plan_archive_v5(&mut ordinary, CompressionProfile::Dense).unwrap();
    let ordinary_encoded = encode(&ordinary, WriteOptions::default()).unwrap();

    let mut reconstructed = baseline;
    plan_archive_v6(&mut reconstructed, CompressionProfile::Dense).unwrap();
    assert_eq!(reconstructed.descriptor.planner_id, "dense-v6");
    assert_eq!(
        reconstructed.content_store.reconstruction_regions.len(),
        1,
        "audits={:?}",
        reconstructed.content_store.reconstruction_audits
    );
    let region = reconstructed
        .content_store
        .reconstruction_regions
        .values()
        .next()
        .unwrap();
    assert!(region.chunk_count > 1);
    assert!(region.access.worst_reconstructed_bytes == region.logical_bytes);

    let mut undeclared = reconstructed.clone();
    undeclared.descriptor.features.incompat &=
        !entrybound::ecf::FEATURE_WHOLE_OBJECT_RECONSTRUCTION_V1;
    assert_eq!(
        encode(&undeclared, WriteOptions::default())
            .unwrap_err()
            .code(),
        entrybound::diagnostics::ReasonCode::UnsupportedRequiredFeature
    );

    let mut missing = reconstructed.clone();
    missing.content_store.reconstruction_regions.clear();
    assert_eq!(
        encode(&missing, WriteOptions::default())
            .unwrap_err()
            .code(),
        entrybound::diagnostics::ReasonCode::UnknownReconstructionRegion,
        "ECF-REGION-MISSING-001"
    );

    let mut bad_access = reconstructed.clone();
    let (old_id, mut bad_region) = bad_access
        .content_store
        .reconstruction_regions
        .pop_first()
        .unwrap();
    bad_region.access.worst_reconstructed_bytes -= 1;
    bad_region.region_id = region_identity_for_test(&bad_region);
    bad_access
        .content_store
        .reconstruction_regions
        .insert(bad_region.region_id, bad_region);
    assert_ne!(
        old_id,
        *bad_access
            .content_store
            .reconstruction_regions
            .keys()
            .next()
            .unwrap()
    );
    assert_eq!(
        encode(&bad_access, WriteOptions::default())
            .unwrap_err()
            .code(),
        entrybound::diagnostics::ReasonCode::InvalidRegionAccess,
        "ECF-REGION-ACCESS-001"
    );

    let mut outside = reconstructed.clone();
    let (_, mut outside_region) = outside
        .content_store
        .reconstruction_regions
        .pop_first()
        .unwrap();
    outside_region.start_chunk_index = u64::try_from(
        outside.content_store.objects[&outside_region.content_object]
            .chunks
            .len(),
    )
    .unwrap();
    outside_region.chunk_count = 1;
    outside_region.region_id = region_identity_for_test(&outside_region);
    outside
        .content_store
        .reconstruction_regions
        .insert(outside_region.region_id, outside_region);
    assert_eq!(
        encode(&outside, WriteOptions::default())
            .unwrap_err()
            .code(),
        entrybound::diagnostics::ReasonCode::InvalidReconstructionRegion,
        "ECF-REGION-RANGE-001"
    );

    let mut overlap = reconstructed.clone();
    let mut second_region = overlap
        .content_store
        .reconstruction_regions
        .values()
        .next()
        .unwrap()
        .clone();
    second_region.chunk_count = 1;
    let first_chunk =
        overlap.content_store.objects[&second_region.content_object].chunks[0].chunk_id;
    second_region.logical_bytes = overlap.content_store.chunks[&first_chunk].logical_len;
    second_region.access.logical_bytes = second_region.logical_bytes;
    second_region.access.logical_chunks = 1;
    second_region.access.worst_reconstructed_bytes = second_region.logical_bytes;
    second_region.region_id = region_identity_for_test(&second_region);
    overlap
        .content_store
        .reconstruction_regions
        .insert(second_region.region_id, second_region);
    assert_eq!(
        encode(&overlap, WriteOptions::default())
            .unwrap_err()
            .code(),
        entrybound::diagnostics::ReasonCode::OverlappingReconstructionRegion,
        "ECF-REGION-OVERLAP-001"
    );

    let mut invalid_candidate = reconstructed.clone();
    let (_, mut invalid_region) = invalid_candidate
        .content_store
        .reconstruction_regions
        .pop_first()
        .unwrap();
    invalid_region.representation[0] ^= 0xff;
    invalid_region.region_id = region_identity_for_test(&invalid_region);
    invalid_candidate
        .content_store
        .reconstruction_regions
        .insert(invalid_region.region_id, invalid_region);
    assert_eq!(
        encode(&invalid_candidate, WriteOptions::default())
            .unwrap_err()
            .code(),
        entrybound::diagnostics::ReasonCode::MalformedReconstructionPayload,
        "ECF-REGION-WRITER-REVERIFY-001: an invalid selected candidate is never committed"
    );

    let first = encode(&reconstructed, WriteOptions::default()).unwrap();
    let second = encode(&reconstructed, WriteOptions::default()).unwrap();
    assert_eq!(first.bytes, second.bytes);
    assert_eq!(first.identities.lai, ordinary_encoded.identities.lai);
    assert_eq!(first.identities.aux, ordinary_encoded.identities.aux);
    assert_eq!(first.identities.pcr, ordinary_encoded.identities.pcr);
    assert_ne!(first.identities.pci, ordinary_encoded.identities.pci);

    let resource_error = open_with_limits(
        &first.bytes,
        bootstrap_resource_policy(),
        DecodeRequirements {
            window_bytes: 8 * 1024 * 1024,
            working_set_bytes: 128 * 1024 * 1024,
            flags: 0,
        },
    )
    .unwrap_err();
    assert_eq!(
        resource_error.code(),
        entrybound::diagnostics::ReasonCode::ResourceLimit
    );

    verify(&first.bytes).unwrap();
    let opened = open(&first.bytes).unwrap();
    let view = inspect(&opened).unwrap();
    assert_eq!(view.whole_object.jpeg_region_count, 1);
    assert!(!view.whole_object.every_chunk_independently_decodable);
    assert!(view.whole_object.worst_access_chunks > 1);
    let explanation = explain(&opened).unwrap();
    assert!(explanation.jpeg_reconstructive_gross_savings_bytes > 0);
    assert!(explanation.jpeg_reconstructive_net_savings_bytes > 0);
    assert!(explanation.jpeg_representation_bytes > 0);
    assert!(
        explanation
            .representative_pipelines
            .iter()
            .any(|pipeline| pipeline.contains("jpeg-jxl-reconstruct/v1"))
    );

    let destination = fixture.path.join("restored");
    unpack(&first.bytes, &destination, ExtractionPolicy::default()).unwrap();
    assert_eq!(fs::read(destination.join("image.jpg")).unwrap(), original);
    assert_eq!(
        fs::read(destination.join("copy.jpg")).unwrap(),
        fs::read(destination.join("image.jpg")).unwrap()
    );
}

#[test]
fn balanced_retains_independent_access_and_small_jpeg_falls_back() {
    let fixture = Fixture::new("balanced");
    let source = fixture.path.join("source");
    fs::create_dir(&source).unwrap();
    fs::write(source.join("small.jpeg"), generated_noise_jpeg(32, 32, 80)).unwrap();
    let encoded = pack_directory(&source, PackOptions::default()).unwrap();
    let opened = open(&encoded.bytes).unwrap();
    assert!(
        opened
            .archive
            .content_store
            .reconstruction_regions
            .is_empty()
    );
    assert!(
        !opened
            .archive
            .content_store
            .reconstruction_audits
            .is_empty()
    );
    assert!(
        inspect(&opened)
            .unwrap()
            .whole_object
            .every_chunk_independently_decodable
    );
    assert!(explain(&opened).unwrap().jpeg_fallback_reason.is_some());
}

#[test]
fn a_chunk_shared_by_distinct_content_objects_makes_regions_ineligible() {
    let jpeg = generated_tiled_jpeg(384, 256, 90);
    let mut archive = archive_for(&jpeg, 16 * 1024);
    let original_digest = *archive.content_store.objects.keys().next().unwrap();
    let shared_chunk = archive.content_store.objects[&original_digest].chunks[0].chunk_id;
    let shared_plaintext = archive.content_store.chunks[&shared_chunk]
        .plaintext
        .clone();
    let (partial, _) = build_content(&shared_plaintext, shared_plaintext.len(), 0).unwrap();
    let partial_digest = partial.logical_digest;
    assert_ne!(partial_digest, original_digest);
    archive
        .content_store
        .objects
        .insert(partial_digest, partial);
    archive.entry_set = EntrySet::new(vec![
        file_entry("image.jpg", original_digest),
        file_entry("fragment.bin", partial_digest),
    ])
    .unwrap();

    plan_archive_v6(&mut archive, CompressionProfile::Dense).unwrap();
    assert!(archive.content_store.reconstruction_regions.is_empty());
    assert!(
        archive
            .content_store
            .reconstruction_audits
            .values()
            .any(|audit| {
                audit.reason == entrybound::eam::ReconstructionAuditReason::RegionDedupConflict
            })
    );
}

fn generated_noise_jpeg(width: u32, height: u32, quality: u8) -> Vec<u8> {
    let mut state = 0x4d59_5df4_d0f3_3173_u64;
    let mut pixels = Vec::with_capacity((width as usize) * (height as usize) * 3);
    for _ in 0..u64::from(width) * u64::from(height) * 3 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        pixels.push((state >> 24) as u8);
    }
    let mut jpeg = Vec::new();
    JpegEncoder::new_with_quality(&mut jpeg, quality)
        .write_image(&pixels, width, height, ExtendedColorType::Rgb8)
        .unwrap();
    jpeg
}

fn generated_tiled_jpeg(width: u32, height: u32, quality: u8) -> Vec<u8> {
    let mut tile = [0_u8; 64 * 64 * 3];
    let mut state = 0x6a09_e667_f3bc_c909_u64;
    for value in &mut tile {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        *value = (state >> 29) as u8;
    }
    let mut pixels = Vec::with_capacity((width as usize) * (height as usize) * 3);
    for y in 0..height as usize {
        for x in 0..width as usize {
            let offset = ((y % 64) * 64 + (x % 64)) * 3;
            pixels.extend_from_slice(&tile[offset..offset + 3]);
        }
    }
    let mut jpeg = Vec::new();
    JpegEncoder::new_with_quality(&mut jpeg, quality)
        .write_image(&pixels, width, height, ExtendedColorType::Rgb8)
        .unwrap();
    jpeg
}

fn archive_for(bytes: &[u8], chunk_size: usize) -> Archive {
    let (object, chunks) = build_content(bytes, chunk_size, 0).unwrap();
    let logical_digest = object.logical_digest;
    Archive {
        descriptor: ArchiveDescriptor {
            format_major: 0,
            format_minor: 1,
            format_namespace: "ecf/bootstrap-v1".to_owned(),
            features: FeatureSet::default(),
            layout: Layout::Indexed,
            role: ArchiveRole::Complete,
            budget_declared: true,
            budget: ResourceBudget::default(),
            decode: DecodeRequirements::default(),
            identity_profile: IdentityProfile::IdentityV1,
            digest_algorithm: DigestAlgorithm::Sha256,
            planner_id: "unplanned".to_owned(),
            chunker_id: "test-fixed-64k/v1".to_owned(),
            lai: Digest::ZERO,
            pcr: Digest::ZERO,
            aux: Digest::ZERO,
            pci: None,
        },
        entry_set: EntrySet::new(vec![file_entry("image.jpg", logical_digest)]).unwrap(),
        content_store: ContentStore {
            physical_order: chunks
                .keys()
                .copied()
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            objects: BTreeMap::from([(logical_digest, object)]),
            chunks,
            ..ContentStore::default()
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

fn file_entry(path: &str, content: Digest) -> Entry {
    Entry::new(
        LogicalPath::from_utf8([path]).unwrap(),
        EntryData::File {
            content: ContentRef::Internal(content),
        },
        MetadataSet::default(),
        EntryIdentity::default(),
    )
}

fn region_identity_for_test(region: &entrybound::eam::ReconstructionRegion) -> Digest {
    let representation_digest = sha256_exact(&region.representation);
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"entrybound/reconstruction-region/jpeg-jxl/v1\0");
    bytes.extend_from_slice(region.content_object.as_bytes());
    bytes.extend_from_slice(&region.start_chunk_index.to_be_bytes());
    bytes.extend_from_slice(&region.chunk_count.to_be_bytes());
    bytes.extend_from_slice(&region.plan_ref.to_be_bytes());
    bytes.extend_from_slice(&region.logical_bytes.to_be_bytes());
    bytes.extend_from_slice(&region.transformed_bytes.to_be_bytes());
    bytes.extend_from_slice(&region.ordinary_physical_bytes.to_be_bytes());
    bytes.extend_from_slice(&region.region_overhead_bytes.to_be_bytes());
    bytes.extend_from_slice(&region.access.logical_bytes.to_be_bytes());
    bytes.extend_from_slice(&region.access.logical_chunks.to_be_bytes());
    bytes.extend_from_slice(&region.access.worst_reconstructed_bytes.to_be_bytes());
    bytes.extend_from_slice(representation_digest.as_bytes());
    sha256_exact(&bytes)
}

struct Fixture {
    path: PathBuf,
}

impl Fixture {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "entrybound-jpeg-{name}-{}-{}",
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
