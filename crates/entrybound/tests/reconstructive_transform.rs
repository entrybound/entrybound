use std::collections::BTreeMap;
use std::io::Write as _;

use entrybound::diagnostics::ReasonCode;
use entrybound::eam::{
    Archive, ArchiveDescriptor, ArchiveRole, ContentRef, ContentStore, DecodeRequirements, Digest,
    DigestAlgorithm, Entry, EntryData, EntryIdentity, EntrySet, FeatureSet, FidelityReport,
    IdentityProfile, Index, Layout, LogicalPath, MetadataSet, ReconstructionFallbackReason,
    ResourceBudget, TransformPlan,
};
use entrybound::ecf::{WriteOptions, encode, open, verify};
use entrybound::identity::{
    STORE_CODEC_IDENTIFIER, STORE_PLAN_ID, STORE_PLAN_IDENTIFIER, build_content,
};
use entrybound::planner::{
    CompressionProfile, UNPLANNED_PLAN_ID, plan_archive_v4, plan_archive_v5,
};
use flate2::{Compression, write::GzEncoder};

#[test]
fn v5_selected_reconstruction_round_trips_and_preserves_logical_identity() {
    let original = gzip_fixture();
    let mut v5 = archive_for(&original);
    let report = plan_archive_v5(&mut v5, CompressionProfile::Dense).unwrap();
    assert!(report.reconstructive_chunks > 0);
    assert!(!v5.content_store.reconstruction_data.is_empty());
    assert!(v5.transform_plans.iter().any(|plan| {
        plan.transforms
            .iter()
            .any(|step| step.transform_id == "deflate-reconstruct/v1")
    }));

    let first = encode(&v5, WriteOptions::default()).unwrap();
    let second = encode(&v5, WriteOptions::default()).unwrap();
    assert_eq!(first.bytes, second.bytes);
    let opened = open(&first.bytes).unwrap();
    verify(&first.bytes).unwrap();
    assert_eq!(only_plaintext(&opened.archive), original.as_slice());
    assert!(opened.report.reconstruction_integrity);

    let mut v4 = archive_for(&original);
    plan_archive_v4(&mut v4, CompressionProfile::Dense).unwrap();
    let historical = encode(&v4, WriteOptions::default()).unwrap();
    let reopened_v4 = open(&historical.bytes).unwrap();
    assert_eq!(first.identities.lai, historical.identities.lai);
    assert_eq!(first.identities.aux, historical.identities.aux);
    assert_eq!(first.identities.pcr, historical.identities.pcr);
    assert_ne!(first.identities.pci, historical.identities.pci);
    assert_eq!(only_plaintext(&reopened_v4.archive), original.as_slice());
}

#[test]
fn missing_corrupt_or_undeclared_reconstruction_fails_typed() {
    let original = gzip_fixture();
    let mut archive = archive_for(&original);
    plan_archive_v5(&mut archive, CompressionProfile::Dense).unwrap();

    let mut missing = archive.clone();
    missing.content_store.reconstruction_data.clear();
    assert_eq!(
        encode(&missing, WriteOptions::default())
            .unwrap_err()
            .code(),
        ReasonCode::UnknownReconstructionData
    );

    let mut corrupt = archive.clone();
    let data = corrupt
        .content_store
        .reconstruction_data
        .values_mut()
        .next()
        .unwrap();
    data.bytes[0] ^= 1;
    assert_eq!(
        encode(&corrupt, WriteOptions::default())
            .unwrap_err()
            .code(),
        ReasonCode::ReconstructionDataDigestMismatch
    );

    let mut undeclared = archive;
    undeclared.descriptor.features.incompat &=
        !entrybound::ecf::FEATURE_RECONSTRUCTIVE_TRANSFORM_V1;
    assert_eq!(
        encode(&undeclared, WriteOptions::default())
            .unwrap_err()
            .code(),
        ReasonCode::UnsupportedRequiredFeature
    );
}

#[test]
fn reconstruction_fallback_reason_is_canonical_and_explainable() {
    let original = vec![0xff; 8 * 1024];
    let mut archive = archive_for(&original);
    plan_archive_v5(&mut archive, CompressionProfile::Balanced).unwrap();

    let chunk_id = *archive.content_store.chunks.keys().next().unwrap();
    assert_eq!(
        archive.content_store.reconstruction_fallbacks[&chunk_id],
        ReconstructionFallbackReason::UnrecognizedOrVerificationFailed
    );

    let encoded = encode(&archive, WriteOptions::default()).unwrap();
    let opened = open(&encoded.bytes).unwrap();
    assert_eq!(
        opened.archive.content_store.reconstruction_fallbacks,
        archive.content_store.reconstruction_fallbacks
    );
    let explanation = entrybound::archive::explain(&opened).unwrap();
    assert_eq!(explanation.reconstructive_fallback_chunk_count, 1);
    assert_eq!(
        explanation.reconstructive_fallback_reason.as_deref(),
        Some("unrecognized-or-verification-failed=1, complete-cost-rejected=0")
    );
}

fn gzip_fixture() -> Vec<u8> {
    let source = (0..20_000)
        .flat_map(|index| {
            format!(
                "row={index:06};value={:08x};category={}\n",
                index * 17,
                index % 31
            )
            .into_bytes()
        })
        .collect::<Vec<_>>();
    let mut encoder = GzEncoder::new(Vec::new(), Compression::new(6));
    encoder.write_all(&source).unwrap();
    encoder.finish().unwrap()
}

fn archive_for(bytes: &[u8]) -> Archive {
    let (object, chunks) = build_content(bytes, bytes.len().max(1), UNPLANNED_PLAN_ID).unwrap();
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
            chunker_id: "test-single-chunk/v1".to_owned(),
            lai: Digest::ZERO,
            pcr: Digest::ZERO,
            aux: Digest::ZERO,
            pci: None,
        },
        entry_set: EntrySet::new(vec![Entry::new(
            LogicalPath::from_utf8(["payload.gz"]).unwrap(),
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
            reconstruction_fallbacks: BTreeMap::new(),
            reconstruction_regions: BTreeMap::new(),
            reconstruction_audits: BTreeMap::new(),
            chunk_groups: BTreeMap::new(),
        },
        transform_plans: vec![store_plan()].into_boxed_slice(),
        fidelity: FidelityReport::default(),
        index: Index::default(),
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

fn only_plaintext(archive: &Archive) -> &[u8] {
    archive
        .content_store
        .chunks
        .values()
        .next()
        .unwrap()
        .plaintext
        .as_ref()
}
