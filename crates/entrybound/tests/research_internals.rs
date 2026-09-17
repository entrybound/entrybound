#![cfg(feature = "research-internals")]
//! The default-off `research-internals` surface must reproduce production
//! planner choices exactly and expose the exact codec bytes archives store.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write as _;

use entrybound::eam::{
    Archive, ArchiveDescriptor, ArchiveRole, ContentRef, ContentStore, DecodeRequirements, Digest,
    DigestAlgorithm, Entry, EntryData, EntryIdentity, EntrySet, FeatureSet, FidelityReport,
    IdentityProfile, Index, Layout, LogicalPath, MetadataSet, ReconstructionAuditReason,
    ReconstructionAuditTarget, ResourceBudget, TransformPlan,
};
use entrybound::ecf::{
    FEATURE_CROSS_FILE_COMPRESSION_V1, FEATURE_WHOLE_OBJECT_RECONSTRUCTION_V1, WriteOptions,
    encode, open,
};
use entrybound::identity::build_content;
use entrybound::planner::{CompressionProfile, UNPLANNED_PLAN_ID};
use entrybound::research::codec::{self, PlanMode};
use entrybound::research::planner::{
    ArchiveTrace, CandidateStage, ChunkTrace, CohortSelection, ExclusionReason, PlannerVersion,
    trace_archive, trace_chunk, trace_chunk_stage,
};
use entrybound::research::{
    ecf as research_ecf, jpeg_reconstruction, planner, reconstruction, similarity, transform,
};
use flate2::Compression;
use flate2::write::GzEncoder;
use image::codecs::jpeg::JpegEncoder;
use image::{ExtendedColorType, ImageEncoder};

struct Input {
    name: &'static str,
    bytes: Vec<u8>,
    chunk_size: usize,
}

#[test]
fn enumeration_reproduces_fast_planner_choices() {
    assert_enumeration_reproduces(CompressionProfile::Fast);
}

#[test]
fn enumeration_reproduces_balanced_planner_choices() {
    assert_enumeration_reproduces(CompressionProfile::Balanced);
}

#[test]
fn enumeration_reproduces_dense_planner_choices() {
    assert_enumeration_reproduces(CompressionProfile::Dense);
}

#[test]
fn enumeration_reproduces_extreme_planner_choices() {
    assert_enumeration_reproduces(CompressionProfile::Extreme);
}

#[test]
fn chunk_enumeration_selects_the_plan_real_planners_assign_to_a_lone_chunk() {
    let gzip = gzip_bytes(4_000);
    let jpeg = noise_jpeg(256, 192, 85);
    let kinds = [
        ("text.txt", text_bytes(20 * 1024)),
        ("counters.bin", numeric_bytes(20 * 1024)),
        ("zeros.bin", vec![0; 20 * 1024]),
        ("random.bin", noise_bytes(20 * 1024, 0x3c6e_f372_fe94_f82b)),
        ("tiny.bin", b"below the Zstandard input floor".to_vec()),
        ("rows.csv.gz", gzip),
        ("photo.jpg", jpeg),
    ];
    let mut regions = 0_usize;
    for (name, bytes) in kinds {
        let archive = archive_for(&[Input {
            name,
            chunk_size: bytes.len(),
            bytes: bytes.clone(),
        }]);
        assert_eq!(archive.content_store.chunks.len(), 1);
        let chunk_id = *archive.content_store.chunks.keys().next().unwrap();
        for profile in profiles() {
            for version in PlannerVersion::ALL {
                let trace = trace_chunk(profile, version, &bytes).unwrap();
                assert_eq!(trace.chunk_id, chunk_id);
                assert_eq!(
                    trace
                        .candidates
                        .iter()
                        .filter(|candidate| candidate.selected)
                        .count(),
                    1
                );
                let mut planned = archive.clone();
                version.plan(&mut planned, profile).unwrap();
                let planned_chunk = &planned.content_store.chunks[&chunk_id];
                if version == PlannerVersion::V6 {
                    regions += assert_object_enumeration_matches(&archive, &planned, profile);
                }
                if planned_chunk.plan_ref == jpeg_reconstruction::REGION_MEMBER_PLAN_REF {
                    assert_eq!(version, PlannerVersion::V6);
                    continue;
                }
                let plan = planned
                    .transform_plans
                    .iter()
                    .find(|plan| plan.plan_id == planned_chunk.plan_ref)
                    .unwrap();
                assert_eq!(
                    trace.selected_plan(),
                    plan,
                    "{name} {profile:?} {version:?}: enumeration selected a different plan"
                );
                assert_eq!(
                    planned
                        .content_store
                        .reconstruction_fallbacks
                        .get(&chunk_id)
                        .copied(),
                    trace
                        .reconstruction
                        .as_ref()
                        .and_then(|attempt| attempt.fallback)
                );
            }
        }
    }
    assert!(regions > 0, "no lone JPEG Chunk selected a region");
}

/// Checks `trace_jpeg_object` against `plan_archive_v6` for a one-object Archive
/// and returns the number of selected regions.
fn assert_object_enumeration_matches(
    archive: &Archive,
    planned_v6: &Archive,
    profile: CompressionProfile,
) -> usize {
    let mut v5 = archive.clone();
    PlannerVersion::V5.plan(&mut v5, profile).unwrap();
    let plans = v5
        .transform_plans
        .iter()
        .map(|plan| (plan.plan_id, plan.clone()))
        .collect::<BTreeMap<_, _>>();
    let object = v5.content_store.objects.values().next().unwrap();
    let trace = planner::trace_jpeg_object(&v5, object, profile, &plans, false).unwrap();
    let target = ReconstructionAuditTarget::ContentObject(object.logical_digest);
    assert_eq!(
        trace.audit,
        planned_v6
            .content_store
            .reconstruction_audits
            .get(&target)
            .map(|audit| audit.reason)
    );
    match &trace.region {
        Some(region) => {
            assert_eq!(
                planned_v6
                    .content_store
                    .reconstruction_regions
                    .values()
                    .collect::<Vec<_>>(),
                vec![region]
            );
            let selected = &trace.candidates[trace.selected.unwrap()];
            assert!(selected.selected && selected.qualified);
            assert!(
                planned_v6
                    .transform_plans
                    .iter()
                    .any(|plan| plan == &selected.plan)
            );
            1
        }
        None => {
            assert!(planned_v6.content_store.reconstruction_regions.is_empty());
            assert!(trace.candidates.iter().all(|candidate| !candidate.selected));
            0
        }
    }
}

#[test]
fn research_codec_round_trip_equals_stored_balanced_payloads() {
    assert_codec_matches_stored_payloads(CompressionProfile::Balanced);
}

#[test]
fn research_codec_round_trip_equals_stored_dense_payloads() {
    assert_codec_matches_stored_payloads(CompressionProfile::Dense);
}

#[test]
fn research_codec_round_trip_equals_stored_fast_and_extreme_payloads() {
    assert_codec_matches_stored_payloads(CompressionProfile::Fast);
    assert_codec_matches_stored_payloads(CompressionProfile::Extreme);
}

#[test]
fn wrappers_forward_to_production_internals() {
    let similar = similar_records(3);
    let left = similarity::fingerprint(&similar[0], 16);
    let right = similarity::fingerprint(&similar[1], 16);
    assert_eq!(left.len(), 16);
    assert!(left.windows(2).all(|pair| pair[0] < pair[1]));
    assert_eq!(similarity::intersection_count(&left, &left), left.len());
    assert!(similarity::intersection_count(&left, &right) > 0);

    let numeric = numeric_bytes(4096);
    for steps in [
        vec![transform::delta8_step()],
        vec![transform::byte_shuffle_step(4).unwrap()],
    ] {
        let forward = transform::forward_pipeline(&steps, &numeric).unwrap();
        assert_ne!(forward, numeric);
        assert_eq!(
            transform::inverse_pipeline(&steps, &forward).unwrap(),
            numeric
        );
    }

    let gzip = gzip_bytes(2_000);
    let candidate = reconstruction::try_forward(&gzip, 2_048)
        .unwrap()
        .expect("complete gzip stream is eligible");
    assert_eq!(
        reconstruction::inverse(&candidate.intermediate, &candidate.data).unwrap(),
        gzip
    );
    assert_eq!(
        reconstruction::verified_forward(&gzip, 2_048, &candidate.data).unwrap(),
        candidate.intermediate
    );

    let jpeg = baseline_jpeg(96, 64);
    let verified = jpeg_reconstruction::verified_forward(&jpeg).unwrap();
    assert_eq!((verified.width, verified.height), (96, 64));
    assert_eq!(jpeg_reconstruction::inverse(&verified.bytes).unwrap(), jpeg);
    assert_eq!(
        jpeg_reconstruction::verified_forward(&progressive_jpeg(96, 64)).unwrap_err(),
        jpeg_reconstruction::JpegAttemptFailure::Unsupported
    );

    let prefix_plan = codec::zstd_prefix_plan(9, 2).unwrap();
    assert_eq!(
        codec::plan_mode(&prefix_plan).unwrap(),
        PlanMode::Prefix { lookback: 2 }
    );
    let history = [
        similar[0].as_slice(),
        similar[1].as_slice(),
        similar[2].as_slice(),
    ];
    let prefix = research_ecf::physical_prefix_from_slices(&history, 2).unwrap();
    assert_eq!(
        prefix,
        [similar[1].as_slice(), similar[2].as_slice()].concat()
    );

    for profile in profiles() {
        let text = text_bytes(24 * 1024);
        let v1 = trace_chunk(profile, PlannerVersion::V1, &text).unwrap();
        assert_eq!(
            usize::try_from(v1.candidates[v1.selected].payload_bytes).unwrap(),
            planner::independent_encoded_len(profile, profile.planner_v1_id(), &text).unwrap()
        );
        let v4 = trace_chunk(profile, PlannerVersion::V4, &text).unwrap();
        assert_eq!(
            v4.selected_plan(),
            &planner::select_v4_plan(profile, &text).unwrap()
        );
        assert_eq!(
            usize::try_from(v4.candidates[v4.selected].payload_bytes).unwrap(),
            planner::independent_encoded_len(profile, profile.planner_v4_id(), &text).unwrap()
        );
    }
}

fn assert_enumeration_reproduces(profile: CompressionProfile) {
    let archive = archive_for(&inputs());
    let mut coverage = Coverage::default();
    let mut v5_chunks = Vec::new();
    for version in PlannerVersion::ALL {
        let trace = trace_archive(&archive, profile, version)
            .unwrap_or_else(|error| panic!("{profile:?} {version:?} trace failed: {error:?}"));
        let mut planned = archive.clone();
        version.plan(&mut planned, profile).unwrap();
        let differences = trace.assignment.differences(&planned);
        assert!(
            differences.is_empty(),
            "{profile:?} {version:?} enumeration differs from the real planner: {differences:#?}"
        );
        assert_selected_candidates_are_planner_choices(&trace, &planned);
        assert_chunk_traces_are_consistent(&archive, &trace, &mut coverage);
        if version == PlannerVersion::V5 {
            v5_chunks = trace.chunks.clone();
        } else if version == PlannerVersion::V6 {
            // v6 runs the frozen v5 Chunk stage unchanged.
            assert_eq!(strip_version(&trace.chunks), strip_version(&v5_chunks));
        }
        assert_cohort_traces_reproduce_select_cohort_plan(&archive, &trace, &mut coverage);
        for object in &trace.objects {
            match (&object.region, object.audit) {
                (Some(region), None) => {
                    coverage.jpeg_region = true;
                    assert_eq!(
                        planned
                            .content_store
                            .reconstruction_regions
                            .get(&region.region_id),
                        Some(region)
                    );
                    let selected = object.selected.unwrap();
                    assert!(object.candidates[selected].qualified);
                    assert_eq!(
                        region.region_overhead_bytes
                            + object.candidates[selected].representation_bytes,
                        object.candidates[selected].complete_cost
                    );
                }
                (None, Some(reason)) => {
                    coverage.audits.insert(format!("{reason:?}"));
                }
                other => {
                    panic!("object trace has neither exactly a region nor an audit: {other:?}")
                }
            }
        }
    }
    match profile {
        CompressionProfile::Fast => {
            assert!(coverage.transformed_or_codec_choice, "{profile:?} coverage");
        }
        CompressionProfile::Balanced | CompressionProfile::Dense | CompressionProfile::Extreme => {
            // The generated inputs must actually exercise every enumerated stage.
            assert!(
                coverage.deflate_eligible,
                "{profile:?} saw no eligible DEFLATE stream"
            );
            if profile != CompressionProfile::Balanced {
                assert!(
                    coverage.deflate_reconstruction,
                    "{profile:?} selected no DEFLATE plan"
                );
            }
            assert!(
                coverage.cross_file_cohort,
                "{profile:?} selected no cross-file cohort plan"
            );
            if profile == CompressionProfile::Balanced {
                assert!(
                    coverage.dictionary_cohort,
                    "Balanced selected no dictionary"
                );
            } else {
                assert!(
                    coverage.lookback_cohort,
                    "{profile:?} selected no lookback group"
                );
            }
            assert!(coverage.jpeg_region, "{profile:?} selected no JPEG region");
            for reason in [
                ReconstructionAuditReason::Unsupported,
                ReconstructionAuditReason::NotRecognized,
            ] {
                assert!(
                    coverage.audits.contains(&format!("{reason:?}")),
                    "{profile:?} recorded audits {:?}",
                    coverage.audits
                );
            }
        }
    }
}

#[derive(Default)]
struct Coverage {
    transformed_or_codec_choice: bool,
    deflate_eligible: bool,
    deflate_reconstruction: bool,
    cross_file_cohort: bool,
    dictionary_cohort: bool,
    lookback_cohort: bool,
    jpeg_region: bool,
    audits: BTreeSet<String>,
}

fn strip_version(chunks: &[ChunkTrace]) -> Vec<ChunkTrace> {
    chunks
        .iter()
        .cloned()
        .map(|mut chunk| {
            chunk.version = PlannerVersion::V5;
            chunk
        })
        .collect()
}

/// Rebuilds each Chunk's final assignment from the selected candidates alone and
/// checks it against the Archive the real planner produced.
fn assert_selected_candidates_are_planner_choices(trace: &ArchiveTrace, planned: &Archive) {
    let planned_plans = planned
        .transform_plans
        .iter()
        .map(|plan| (plan.plan_id, plan))
        .collect::<BTreeMap<_, _>>();
    let mut expected = BTreeMap::<Digest, (Option<&TransformPlan>, Option<Digest>)>::new();
    for chunk in &trace.chunks {
        expected.insert(chunk.chunk_id, (Some(chunk.selected_plan()), None));
    }
    for cohort in &trace.cohorts {
        let Some(index) = cohort.selected else {
            continue;
        };
        let candidate = &cohort.candidates[index];
        assert!(candidate.selected);
        for chunk_id in &cohort.cohort.chunks {
            expected.insert(
                *chunk_id,
                (
                    Some(&candidate.plan),
                    candidate.group.as_ref().map(|group| group.group_id),
                ),
            );
        }
    }
    for object in &trace.objects {
        if let (Some(region), Some(index)) = (&object.region, object.selected) {
            assert_eq!(
                planned_plans.get(&region.plan_ref).copied(),
                Some(&object.candidates[index].plan)
            );
            for chunk_ref in &planned.content_store.objects[&object.content_object].chunks {
                expected.insert(chunk_ref.chunk_id, (None, None));
            }
        }
    }
    assert_eq!(expected.len(), planned.content_store.chunks.len());
    for (chunk_id, chunk) in &planned.content_store.chunks {
        let (plan, group_ref) = expected[chunk_id];
        match plan {
            Some(plan) => {
                assert_eq!(
                    planned_plans.get(&chunk.plan_ref).copied(),
                    Some(plan),
                    "{:?} {:?} chunk {chunk_id}",
                    trace.profile,
                    trace.version
                );
            }
            None => assert_eq!(chunk.plan_ref, jpeg_reconstruction::REGION_MEMBER_PLAN_REF),
        }
        assert_eq!(chunk.group_ref, group_ref);
    }
}

fn assert_chunk_traces_are_consistent(
    archive: &Archive,
    trace: &ArchiveTrace,
    coverage: &mut Coverage,
) {
    let profile = trace.profile;
    let version = trace.version;
    assert_eq!(trace.chunks.len(), archive.content_store.chunks.len());
    for chunk_trace in &trace.chunks {
        let plaintext = &archive.content_store.chunks[&chunk_trace.chunk_id].plaintext;
        assert_eq!(
            chunk_trace
                .candidates
                .iter()
                .filter(|candidate| candidate.selected)
                .count(),
            1
        );
        for (index, candidate) in chunk_trace.candidates.iter().enumerate() {
            assert_eq!(candidate.selected, index == chunk_trace.selected);
            assert_eq!(candidate.exclusion.is_none(), candidate.selected);
        }
        let selected = chunk_trace.selected_plan();
        if selected.codec != entrybound::identity::STORE_CODEC_IDENTIFIER {
            coverage.transformed_or_codec_choice = true;
        }
        if selected
            .transforms
            .iter()
            .any(|step| step.reconstruction_ref.is_some())
        {
            coverage.deflate_reconstruction = true;
        }
        if chunk_trace
            .reconstruction
            .as_ref()
            .is_some_and(|attempt| attempt.data.is_some())
        {
            coverage.deflate_eligible = true;
        }

        if matches!(
            version,
            PlannerVersion::V4 | PlannerVersion::V5 | PlannerVersion::V6
        ) {
            let v4_winner = chunk_trace
                .candidates
                .iter()
                .filter(|candidate| {
                    matches!(
                        candidate.stage,
                        CandidateStage::V4Ordinary | CandidateStage::V4Transformed
                    ) && matches!(
                        candidate.exclusion,
                        None | Some(ExclusionReason::CarriedToNextStage)
                    )
                })
                .map(|candidate| &candidate.plan)
                .collect::<Vec<_>>();
            assert_eq!(
                v4_winner,
                vec![&planner::select_v4_plan(profile, plaintext).unwrap()],
                "{profile:?} {version:?} chunk {}",
                chunk_trace.chunk_id
            );
        }

        if matches!(
            version,
            PlannerVersion::V1 | PlannerVersion::V2 | PlannerVersion::V3
        ) {
            let selected = &chunk_trace.candidates[chunk_trace.selected];
            assert_eq!(
                usize::try_from(selected.payload_bytes).unwrap(),
                planner::independent_encoded_len(profile, version.planner_id(profile), plaintext)
                    .unwrap()
            );
            for candidate in &chunk_trace.candidates {
                if candidate.stage == CandidateStage::IndependentZstandard {
                    assert_eq!(
                        candidate.qualified,
                        entrybound::planner::zstandard_wins(
                            chunk_trace.logical_len,
                            usize::try_from(candidate.payload_bytes).unwrap()
                        )
                        .unwrap()
                    );
                }
            }
        }

        // Cost components are exactly the production cost functions.
        if version == PlannerVersion::V5 {
            let data = chunk_trace
                .reconstruction
                .as_ref()
                .and_then(|attempt| attempt.data.clone());
            for candidate in &chunk_trace.candidates {
                match candidate.stage {
                    CandidateStage::V4Ordinary | CandidateStage::V4Transformed => {
                        assert_eq!(
                            candidate.complete_cost,
                            planner::complete_candidate_cost(&candidate.plan, plaintext).unwrap()
                        );
                        assert_eq!(
                            candidate.plan_record_bytes,
                            research_ecf::encoded_transform_plan_len(&candidate.plan).unwrap()
                        );
                    }
                    CandidateStage::V5Baseline => assert_eq!(
                        candidate.complete_cost,
                        planner::v5_independent_cost(&candidate.plan, plaintext, None).unwrap()
                    ),
                    CandidateStage::V5Reconstruction => {
                        let data = data.as_ref().unwrap();
                        let values = BTreeMap::from([(data.reconstruction_id, data.clone())]);
                        assert_eq!(
                            candidate.complete_cost,
                            planner::v5_independent_cost(
                                &candidate.plan,
                                plaintext,
                                Some((data, &values))
                            )
                            .unwrap()
                        );
                    }
                    CandidateStage::IndependentZstandard | CandidateStage::IndependentStore => {
                        panic!("v5 trace contains a v1-v3 candidate")
                    }
                }
                assert_eq!(
                    candidate.complete_cost,
                    candidate.payload_bytes
                        + candidate.plan_record_bytes
                        + candidate.side_data_bytes
                );
            }
        }
    }
}

fn assert_cohort_traces_reproduce_select_cohort_plan(
    archive: &Archive,
    trace: &ArchiveTrace,
    coverage: &mut Coverage,
) {
    if matches!(trace.version, PlannerVersion::V1 | PlannerVersion::V2) {
        assert!(trace.cohorts.is_empty());
        return;
    }
    // The exact state production hands to select_cohort_plan: Chunk-stage
    // plan_refs, the Chunk-stage plan table, and Chunk-stage ReconstructionData.
    let stage = trace_chunk_stage(archive, trace.profile, trace.version).unwrap();
    assert_eq!(stage.chunks, trace.chunks);
    let stage_planner_id = trace.version.stage_planner_id(trace.profile);
    for cohort_trace in &trace.cohorts {
        let production = planner::select_cohort_plan(
            &stage.archive,
            &cohort_trace.cohort,
            trace.profile,
            stage_planner_id,
            &stage.plans,
        )
        .unwrap();
        assert_eq!(production, cohort_trace.selection());
        if trace.version == PlannerVersion::V5 {
            assert_eq!(
                &planner::trace_cohort(
                    &stage.archive,
                    &cohort_trace.cohort,
                    trace.profile,
                    stage_planner_id,
                    &stage.plans,
                )
                .unwrap(),
                cohort_trace
            );
        }
        let independent = cohort_trace.independent;
        assert_eq!(
            independent.complete_cost,
            independent.payload_bytes + independent.plan_record_bytes + independent.side_data_bytes
        );
        for (index, candidate) in cohort_trace.candidates.iter().enumerate() {
            assert_eq!(candidate.selected, cohort_trace.selected == Some(index));
            assert_eq!(candidate.exclusion.is_none(), candidate.selected);
            assert_eq!(
                candidate.complete_cost,
                candidate.payload_bytes + candidate.plan_record_bytes + candidate.side_data_bytes
            );
            assert_eq!(
                candidate.qualified,
                planner::qualifies_against_independent(
                    candidate.complete_cost,
                    independent.complete_cost
                )
                .unwrap()
            );
        }
        match production {
            CohortSelection::Independent => continue,
            CohortSelection::Dictionary { .. } => coverage.dictionary_cohort = true,
            CohortSelection::Lookback { .. } => coverage.lookback_cohort = true,
        }
        coverage.cross_file_cohort = true;
        let selected = &cohort_trace.candidates[cohort_trace.selected.unwrap()];
        assert!(selected.qualified);
        assert!(
            selected.complete_cost + planner::MINIMUM_COHORT_GAIN_BYTES < independent.complete_cost
        );
    }
}

fn assert_codec_matches_stored_payloads(profile: CompressionProfile) {
    let mut archive = archive_for(&inputs());
    PlannerVersion::V6.plan(&mut archive, profile).unwrap();
    let encoded = encode(&archive, WriteOptions::default()).unwrap();
    let bytes = &encoded.bytes;
    let opened = open(bytes).unwrap();
    let features = opened.archive.descriptor.features.incompat;
    let extended = features & FEATURE_CROSS_FILE_COMPRESSION_V1 != 0;
    let whole_object = features & FEATURE_WHOLE_OBJECT_RECONSTRUCTION_V1 != 0;
    let header_len = usize::try_from(research_ecf::chunk_frame_header_len(extended)).unwrap();
    let plans = archive
        .transform_plans
        .iter()
        .map(|plan| (plan.plan_id, plan))
        .collect::<BTreeMap<_, _>>();
    let mut group_history = BTreeMap::<Digest, Vec<&[u8]>>::new();
    let mut compared = 0_usize;

    for chunk_id in &opened.archive.content_store.physical_order {
        let chunk = &archive.content_store.chunks[chunk_id];
        let location = opened.archive.index.chunks[chunk_id];
        let offset = usize::try_from(location.offset).unwrap();
        let header = research_ecf::parse_chunk_frame_header(
            &bytes[offset..offset + header_len],
            extended,
            whole_object,
        )
        .unwrap();
        assert_eq!(header.chunk_id, *chunk_id);
        assert_eq!(header.stored_len, location.stored_len);
        assert_eq!(header.logical_len, chunk.logical_len);
        assert_eq!(header.plan_ref, chunk.plan_ref);
        assert_eq!(header.group_ref, chunk.group_ref);
        let stored_start = offset + header_len;
        let stored =
            &bytes[stored_start..stored_start + usize::try_from(location.stored_len).unwrap()];
        if header.region_owned {
            assert!(stored.is_empty());
            continue;
        }

        let plan = plans[&chunk.plan_ref];
        let reconstructive = plan
            .transforms
            .iter()
            .any(|step| step.reconstruction_ref.is_some());
        let mut prefix = None;
        let expected = match codec::plan_mode(plan).unwrap() {
            PlanMode::Independent if reconstructive => {
                let values = &archive.content_store.reconstruction_data;
                assert_eq!(
                    codec::decode_payload_with_reconstruction(
                        plan,
                        stored,
                        chunk.logical_len,
                        values
                    )
                    .unwrap(),
                    chunk.plaintext.as_ref()
                );
                codec::encode_payload_with_reconstruction(plan, &chunk.plaintext, values).unwrap()
            }
            PlanMode::Independent => {
                assert_eq!(
                    codec::decode_payload(plan, stored, chunk.logical_len).unwrap(),
                    chunk.plaintext.as_ref()
                );
                codec::encode_payload(plan, &chunk.plaintext).unwrap()
            }
            PlanMode::Dictionary(dictionary_id) => {
                let dictionary = &archive.content_store.dictionaries[&dictionary_id];
                codec::validate_dictionary(dictionary).unwrap();
                assert_eq!(
                    codec::decode_payload_with_dictionary(
                        plan,
                        stored,
                        chunk.logical_len,
                        dictionary
                    )
                    .unwrap(),
                    chunk.plaintext.as_ref()
                );
                codec::encode_payload_with_dictionary(plan, &chunk.plaintext, dictionary).unwrap()
            }
            PlanMode::Prefix { lookback } => {
                let group_id = chunk.group_ref.unwrap();
                let history = group_history.entry(group_id).or_default();
                let bounded = research_ecf::physical_prefix_from_slices(history, lookback).unwrap();
                assert_eq!(
                    codec::decode_payload_with_prefix(plan, stored, chunk.logical_len, &bounded)
                        .unwrap(),
                    chunk.plaintext.as_ref()
                );
                let encoded =
                    codec::encode_payload_with_prefix(plan, &chunk.plaintext, &bounded).unwrap();
                history.push(&chunk.plaintext);
                prefix = Some(bounded);
                encoded
            }
        };
        assert_eq!(
            expected, stored,
            "{profile:?}: research codec output differs from stored payload of Chunk {chunk_id}"
        );
        assert_eq!(
            research_ecf::decode_frame_payload(
                plan,
                stored,
                chunk.logical_len,
                &archive.content_store.dictionaries,
                &archive.content_store.reconstruction_data,
                prefix.as_deref(),
            )
            .unwrap(),
            chunk.plaintext.as_ref()
        );
        compared += 1;
    }
    assert!(compared > 0);

    for region in opened.archive.content_store.reconstruction_regions.values() {
        let plan = plans[&region.plan_ref];
        let object = &archive.content_store.objects[&region.content_object];
        let original = object
            .chunks
            .iter()
            .flat_map(|chunk_ref| {
                archive.content_store.chunks[&chunk_ref.chunk_id]
                    .plaintext
                    .iter()
                    .copied()
            })
            .collect::<Vec<_>>();
        let jxl = codec::decode_transformed_payload(
            plan,
            &region.representation,
            region.transformed_bytes,
        )
        .unwrap();
        assert_eq!(jpeg_reconstruction::inverse(&jxl).unwrap(), original);
        assert_eq!(
            codec::encode_transformed_payload(plan, &jxl).unwrap(),
            region.representation.as_ref()
        );
        assert_eq!(
            jpeg_reconstruction::verified_forward(&original)
                .unwrap()
                .bytes,
            jxl
        );
        assert_eq!(
            jpeg_reconstruction::region_identity(region),
            region.region_id
        );
    }
    if profile != CompressionProfile::Fast {
        assert!(
            !opened
                .archive
                .content_store
                .reconstruction_regions
                .is_empty()
        );
    }
}

fn profiles() -> [CompressionProfile; 4] {
    [
        CompressionProfile::Fast,
        CompressionProfile::Balanced,
        CompressionProfile::Dense,
        CompressionProfile::Extreme,
    ]
}

fn inputs() -> Vec<Input> {
    let mut inputs = vec![
        Input {
            name: "text.txt",
            bytes: text_bytes(40 * 1024),
            chunk_size: 16 * 1024,
        },
        Input {
            name: "counters.bin",
            bytes: numeric_bytes(40 * 1024),
            chunk_size: 16 * 1024,
        },
        Input {
            name: "zeros.bin",
            bytes: vec![0; 36 * 1024],
            chunk_size: 16 * 1024,
        },
        Input {
            name: "random.bin",
            bytes: noise_bytes(24 * 1024, 0x4d59_5df4_d0f3_3173),
            chunk_size: 16 * 1024,
        },
    ];
    let gzip = gzip_bytes(4_000);
    inputs.push(Input {
        name: "rows.csv.gz",
        chunk_size: gzip.len(),
        bytes: gzip,
    });
    // One single-Chunk JPEG (eligible under Balanced) and one multi-Chunk JPEG
    // (Dense/Extreme only), so the second region sees an emitted region section.
    let jpeg = noise_jpeg(256, 192, 85);
    inputs.push(Input {
        name: "single-chunk.jpg",
        chunk_size: jpeg.len(),
        bytes: jpeg,
    });
    inputs.push(Input {
        name: "multi-chunk.jpg",
        bytes: noise_jpeg(288, 224, 80),
        chunk_size: 24 * 1024,
    });
    let progressive = progressive_jpeg(128, 96);
    inputs.push(Input {
        name: "progressive.jpg",
        chunk_size: progressive.len(),
        bytes: progressive,
    });
    for (index, bytes) in similar_records(10).into_iter().enumerate() {
        inputs.push(Input {
            name: Box::leak(format!("records-{index:02}.txt").into_boxed_str()),
            chunk_size: bytes.len(),
            bytes,
        });
    }
    // Incompressible samples sharing one base only a trained dictionary can
    // exploit, so Balanced (dictionary-only cohorts) also selects a cohort plan.
    for (index, bytes) in shared_noise_samples(12, 16 * 1024).into_iter().enumerate() {
        inputs.push(Input {
            name: Box::leak(format!("sample-{index:02}.bin").into_boxed_str()),
            chunk_size: bytes.len(),
            bytes,
        });
    }
    inputs
}

fn archive_for(inputs: &[Input]) -> Archive {
    let mut objects = BTreeMap::new();
    let mut chunks = BTreeMap::new();
    let mut entries = Vec::new();
    for input in inputs {
        let (object, object_chunks) =
            build_content(&input.bytes, input.chunk_size, UNPLANNED_PLAN_ID).unwrap();
        entries.push(Entry::new(
            LogicalPath::from_utf8([input.name]).unwrap(),
            EntryData::File {
                content: ContentRef::Internal(object.logical_digest),
            },
            MetadataSet::default(),
            EntryIdentity::default(),
        ));
        assert!(objects.insert(object.logical_digest, object).is_none());
        chunks.extend(object_chunks);
    }
    Archive {
        descriptor: ArchiveDescriptor {
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
            planner_id: "unplanned".to_owned(),
            chunker_id: "test-fixed-research/v1".to_owned(),
            lai: Digest::ZERO,
            pcr: Digest::ZERO,
            aux: Digest::ZERO,
            pci: None,
        },
        entry_set: EntrySet::new(entries).unwrap(),
        content_store: ContentStore {
            physical_order: chunks
                .keys()
                .copied()
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            objects,
            chunks,
            ..ContentStore::default()
        },
        transform_plans: vec![codec::store_plan()].into_boxed_slice(),
        fidelity: FidelityReport::default(),
        conversion: None,
        preservation: None,
        index: Index::default(),
    }
}

fn text_bytes(len: usize) -> Vec<u8> {
    let mut bytes = (0..)
        .flat_map(|index: u32| {
            format!(
                "{index:06} {} account={:05} status={}\n",
                ["alpha", "beta", "gamma", "delta"][(index % 4) as usize],
                index.wrapping_mul(7919) % 50_000,
                if index.is_multiple_of(3) {
                    "open"
                } else {
                    "closed"
                }
            )
            .into_bytes()
        })
        .take(len)
        .collect::<Vec<_>>();
    bytes.truncate(len);
    bytes
}

fn numeric_bytes(len: usize) -> Vec<u8> {
    (0_u32..)
        .flat_map(|value| (value * 3).to_le_bytes())
        .take(len)
        .collect()
}

fn noise_bytes(len: usize, seed: u64) -> Vec<u8> {
    let mut state = seed;
    (0..len)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state >> 24) as u8
        })
        .collect()
}

fn gzip_bytes(rows: usize) -> Vec<u8> {
    let source = (0..rows)
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

fn shared_noise_samples(count: usize, len: usize) -> Vec<Vec<u8>> {
    let base = noise_bytes(len, 0xbb67_ae85_84ca_a73b);
    (0..count)
        .map(|index| {
            let mut bytes = base.clone();
            let first = 128 + (index * 977) % (len - 512);
            for (offset, byte) in bytes[first..first + 256].iter_mut().enumerate() {
                *byte = (index as u8).wrapping_mul(31).wrapping_add(offset as u8);
            }
            bytes
        })
        .collect()
}

fn similar_records(count: usize) -> Vec<Vec<u8>> {
    (0..count)
        .map(|index| {
            let header = format!("record={index:04}\nkind=entrybound-research-internals\n");
            let rows = (0..160)
                .map(|row| format!("row={row:04},field=shared-value,status=active,owner=ops\n"))
                .collect::<String>();
            format!("{header}{rows}tail={index:04}\n").into_bytes()
        })
        .collect()
}

fn pixels(width: u32, height: u32) -> Vec<u8> {
    (0..height)
        .flat_map(|y| {
            (0..width).flat_map(move |x| {
                [
                    x.wrapping_mul(13).wrapping_add(y * 3) as u8,
                    y.wrapping_mul(17).wrapping_add(x * 5) as u8,
                    x.wrapping_mul(7).wrapping_add(y * 11) as u8,
                ]
            })
        })
        .collect()
}

fn baseline_jpeg(width: u32, height: u32) -> Vec<u8> {
    let mut jpeg = Vec::new();
    JpegEncoder::new_with_quality(&mut jpeg, 84)
        .write_image(
            &pixels(width, height),
            width,
            height,
            ExtendedColorType::Rgb8,
        )
        .unwrap();
    jpeg
}

fn noise_jpeg(width: u32, height: u32, quality: u8) -> Vec<u8> {
    let image = noise_bytes(
        (width as usize) * (height as usize) * 3,
        0x6a09_e667_f3bc_c909,
    );
    let mut jpeg = Vec::new();
    JpegEncoder::new_with_quality(&mut jpeg, quality)
        .write_image(&image, width, height, ExtendedColorType::Rgb8)
        .unwrap();
    jpeg
}

fn progressive_jpeg(width: u16, height: u16) -> Vec<u8> {
    let mut jpeg = Vec::new();
    let mut encoder = jpeg_encoder::Encoder::new(&mut jpeg, 86);
    encoder.set_progressive(true);
    encoder
        .encode(
            &pixels(u32::from(width), u32::from(height)),
            width,
            height,
            jpeg_encoder::ColorType::Rgb,
        )
        .unwrap();
    jpeg
}
