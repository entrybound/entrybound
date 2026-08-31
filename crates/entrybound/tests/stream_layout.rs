//! STREAM layout: identity equivalence with INDEXED, sequential guarantees,
//! and adversarial framing.
//!
//! Every fixture here is generated from real content and encoded by the real
//! writer. There are no opaque fixture archives.

use std::collections::BTreeMap;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use entrybound::archive::{
    ExtractionPolicy, PackOptions, bootstrap_resource_policy, inspect, list, pack_directory,
    unpack, unpack_stream,
};
use entrybound::diagnostics::{OutcomeClass, ReasonCode};
use entrybound::eam::{
    Archive, ArchiveDescriptor, ArchiveRole, ContentRef, ContentStore, DecodeRequirements, Digest,
    DigestAlgorithm, Entry, EntryData, EntryIdentity, EntrySet, FeatureSet, FidelityReport,
    IdentityProfile, Index, Layout, LogicalPath, MetadataSet, ResourceBudget, TransformPlan,
};
use entrybound::ecf::{
    CHUNK_FRAME_HEADER_LEN, CHUNK_FRAME_V2_HEADER_LEN, FEATURE_STREAM_LAYOUT_V1, IndexStatus,
    PREAMBLE_LEN, STREAM_FOOTER_LEN, STREAM_ITEM_HEADER_LEN, STREAM_ITEM_MAGIC, SequentialLimits,
    StagingLimits, StreamContentPolicy, StreamItemTag, StreamWindow, StreamWriteOptions,
    StreamWriteSummary, bootstrap_sequential_limits, encode_stream, open, open_stream_with_limits,
    verify_stream_with_limits,
};
use entrybound::identity::{
    STORE_CODEC_IDENTIFIER, STORE_PLAN_ID, STORE_PLAN_IDENTIFIER, build_content_from_ranges,
    sha256_exact,
};
use entrybound::planner::CompressionProfile;

static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

// ---------------------------------------------------------------------------
// Sinks and sources that prove the access guarantees at the type level
// ---------------------------------------------------------------------------

/// A sink that implements `Write` and nothing else.
///
/// The STREAM writer is generic over `W: Write`, so a writer that ever needed
/// to seek could not be instantiated with this type at all.
#[derive(Debug, Default)]
struct WriteOnlySink {
    bytes: Vec<u8>,
    writes: usize,
    flushes: usize,
}

impl Write for WriteOnlySink {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.writes += 1;
        self.bytes.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.flushes += 1;
        Ok(())
    }
}

/// A source that implements `Read` and nothing else, and that hands back at
/// most `step` bytes per call, the way a pipe under load behaves.
#[derive(Debug)]
struct DripSource<'a> {
    bytes: &'a [u8],
    cursor: usize,
    step: usize,
    reads: usize,
}

impl<'a> DripSource<'a> {
    const fn new(bytes: &'a [u8], step: usize) -> Self {
        Self {
            bytes,
            cursor: 0,
            step,
            reads: 0,
        }
    }
}

impl Read for DripSource<'_> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.reads += 1;
        let remaining = self.bytes.len() - self.cursor;
        let count = remaining.min(self.step).min(buf.len());
        buf[..count].copy_from_slice(&self.bytes[self.cursor..self.cursor + count]);
        self.cursor += count;
        Ok(count)
    }
}

// ---------------------------------------------------------------------------
// Layout equivalence
// ---------------------------------------------------------------------------

#[test]
fn every_profile_preserves_logical_identity_across_both_layouts() {
    let fixture = Fixture::new("equivalence");
    let source = fixture.path.join("source");
    write_rich_source(&source);

    let mut observed_codecs = Vec::new();
    for profile in [
        CompressionProfile::Fast,
        CompressionProfile::Balanced,
        CompressionProfile::Dense,
        CompressionProfile::Extreme,
    ] {
        let indexed = pack(&source, profile);
        let mut sink = WriteOnlySink::default();
        let stream = encode_stream(&indexed.archive, auto_window(), &mut sink).unwrap();
        assert!(sink.writes > 0);

        // The central invariant: the layouts never disagree about meaning.
        assert_eq!(indexed.identities.lai, stream.identities.lai);
        assert_eq!(indexed.identities.aux, stream.identities.aux);
        assert_eq!(indexed.identities.pcr, stream.identities.pcr);
        assert_ne!(indexed.identities.pci, stream.identities.pci);
        assert_ne!(indexed.bytes, sink.bytes);

        let opened_indexed = open(&indexed.bytes).unwrap();
        let sequential = open_stream_with_limits(
            DripSource::new(&sink.bytes, 4096),
            retaining_limits(bootstrap_sequential_limits()),
        )
        .unwrap();
        let streamed = &sequential.opened.archive;

        assert_eq!(streamed.descriptor.layout, Layout::Stream);
        assert_eq!(opened_indexed.archive.descriptor.layout, Layout::Indexed);
        assert_eq!(opened_indexed.archive.entry_set, streamed.entry_set);
        assert_eq!(
            opened_indexed.archive.content_store.objects,
            streamed.content_store.objects
        );
        assert_eq!(
            opened_indexed.archive.content_store.chunks,
            streamed.content_store.chunks
        );
        assert_eq!(
            opened_indexed.archive.content_store.dictionaries,
            streamed.content_store.dictionaries
        );
        assert_eq!(
            opened_indexed.archive.content_store.chunk_groups,
            streamed.content_store.chunk_groups
        );
        assert_eq!(
            opened_indexed.archive.content_store.reconstruction_data,
            streamed.content_store.reconstruction_data
        );
        assert_eq!(
            opened_indexed.archive.content_store.reconstruction_regions,
            streamed.content_store.reconstruction_regions
        );
        assert_eq!(
            opened_indexed.archive.transform_plans,
            streamed.transform_plans
        );
        assert_eq!(opened_indexed.archive.fidelity, streamed.fidelity);
        assert_eq!(
            list(&opened_indexed.archive).unwrap(),
            list(streamed).unwrap()
        );

        // Physical organization is exactly what the layouts are allowed to
        // differ in, and it is the only structural difference.
        assert_eq!(
            opened_indexed
                .archive
                .content_store
                .physical_order
                .iter()
                .copied()
                .collect::<std::collections::BTreeSet<_>>(),
            streamed
                .content_store
                .physical_order
                .iter()
                .copied()
                .collect::<std::collections::BTreeSet<_>>()
        );

        let view = inspect(&sequential.opened).unwrap();
        assert_eq!(view.layout, Layout::Stream);
        assert_eq!(view.index_status, IndexStatus::NotApplicableStream);
        assert!(!view.index_applicable);
        assert!(!view.random_entry_lookup);
        assert!(view.stream_layout_feature_present);
        assert!(view.budget_declared);
        assert_eq!(view.stream_dedup_window, stream.dedup_window);
        observed_codecs.extend(view.codec_usage.iter().map(|usage| usage.codec.clone()));

        let indexed_out = fixture.path.join(format!("indexed-{profile:?}"));
        let stream_out = fixture.path.join(format!("stream-{profile:?}"));
        unpack(&indexed.bytes, &indexed_out, ExtractionPolicy::default()).unwrap();
        unpack_stream(
            DripSource::new(&sink.bytes, 7),
            &stream_out,
            ExtractionPolicy::default(),
            bootstrap_sequential_limits(),
        )
        .unwrap();
        assert_trees_equal(&source, &indexed_out);
        assert_trees_equal(&source, &stream_out);
    }

    observed_codecs.sort();
    observed_codecs.dedup();
    assert!(observed_codecs.iter().any(|codec| codec == "store/v1"));
    assert!(observed_codecs.iter().any(|codec| codec == "zstandard/v1"));
}

#[test]
fn shared_dictionaries_survive_the_sequential_layout() {
    let fixture = Fixture::new("dictionary");
    let source = fixture.path.join("source");
    write_similar_source(&source, 24, 64 * 1024, 0x6a09_e667_f3bc_c909);
    fs::write(
        source.join("unrelated.bin"),
        noise(64 * 1024, 0x510e_527f_ade6_82d1),
    )
    .unwrap();

    let indexed = pack(&source, CompressionProfile::Balanced);
    assert!(!indexed.archive.content_store.dictionaries.is_empty());
    let (bytes, summary) = encode_to_vec(&indexed.archive, auto_window());
    assert_eq!(indexed.identities.lai, summary.identities.lai);
    assert_eq!(indexed.identities.pcr, summary.identities.pcr);

    let sequential = open_stream_with_limits(
        DripSource::new(&bytes, 64),
        retaining_limits(bootstrap_sequential_limits()),
    )
    .unwrap();
    assert_eq!(
        sequential.opened.archive.content_store.dictionaries,
        indexed.archive.content_store.dictionaries
    );
    let restored = fixture.path.join("restored");
    unpack_stream(
        DripSource::new(&bytes, 4096),
        &restored,
        ExtractionPolicy::default(),
        bootstrap_sequential_limits(),
    )
    .unwrap();
    assert_trees_equal(&source, &restored);
}

#[test]
fn bounded_lookback_groups_survive_the_sequential_layout() {
    let fixture = Fixture::new("lookback");
    let source = fixture.path.join("source");
    write_similar_source(&source, 12, 64 * 1024, 0xbb67_ae85_84ca_a73b);
    fs::write(
        source.join("independent.bin"),
        noise(64 * 1024, 0xa54f_f53a_5f1d_36f1),
    )
    .unwrap();

    let indexed = pack(&source, CompressionProfile::Dense);
    assert!(!indexed.archive.content_store.chunk_groups.is_empty());
    let (bytes, summary) = encode_to_vec(&indexed.archive, auto_window());
    assert_eq!(indexed.identities.lai, summary.identities.lai);
    assert_eq!(indexed.identities.pcr, summary.identities.pcr);
    assert_ne!(indexed.identities.pci, summary.identities.pci);

    let sequential = open_stream_with_limits(
        DripSource::new(&bytes, 1),
        retaining_limits(bootstrap_sequential_limits()),
    )
    .unwrap();
    assert_eq!(
        sequential.opened.archive.content_store.chunk_groups,
        indexed.archive.content_store.chunk_groups
    );
    assert_eq!(
        sequential.opened.archive.content_store.chunks,
        indexed.archive.content_store.chunks
    );
    let restored = fixture.path.join("restored");
    unpack_stream(
        DripSource::new(&bytes, 4096),
        &restored,
        ExtractionPolicy::default(),
        bootstrap_sequential_limits(),
    )
    .unwrap();
    assert_trees_equal(&source, &restored);
}

// ---------------------------------------------------------------------------
// Stream dedup window
// ---------------------------------------------------------------------------

#[test]
fn a_window_of_zero_is_valid_when_no_object_shares_an_earlier_chunk() {
    let fixture = Fixture::new("window-zero");
    let source = fixture.path.join("source");
    fs::create_dir_all(source.join("nested")).unwrap();
    fs::write(source.join("alpha.bin"), noise(96 * 1024, 0x1111)).unwrap();
    fs::write(source.join("beta.bin"), noise(96 * 1024, 0x2222)).unwrap();
    fs::write(source.join("nested/gamma.txt"), b"gamma text\n").unwrap();
    fs::write(source.join("empty"), []).unwrap();

    let indexed = pack(&source, CompressionProfile::Balanced);
    let (bytes, summary) = encode_to_vec(&indexed.archive, StreamWriteOptions::default());
    assert_eq!(summary.dedup_window, 0);

    let sequential =
        open_stream_with_limits(DripSource::new(&bytes, 1), bootstrap_sequential_limits()).unwrap();
    assert_eq!(sequential.stream.dedup_window, 0);
    assert_eq!(sequential.opened.archive.descriptor.stream_dedup_window, 0);
}

#[test]
fn a_plan_needing_history_fails_a_zero_window_and_names_the_requirement() {
    let archive = shared_reference_archive();
    let auto = encode_to_vec(&archive, auto_window()).1;
    assert!(
        auto.dedup_window > 0,
        "the fixture must create a cross-object historical dependency"
    );

    let mut refused = WriteOnlySink::default();
    let error = encode_stream(&archive, StreamWriteOptions::default(), &mut refused).unwrap_err();
    assert_eq!(error.code(), ReasonCode::StreamWindowExceeded);
    assert_eq!(error.class(), OutcomeClass::PolicyRefused);
    assert!(error.detail().contains(&auto.dedup_window.to_string()));

    // An explicit ceiling at or above the requirement is accepted, and the
    // archive declares the exact minimum its organization needs.
    let exact = encode_to_vec(
        &archive,
        StreamWriteOptions {
            window: StreamWindow::Ceiling(auto.dedup_window),
            ..StreamWriteOptions::default()
        },
    );
    assert_eq!(exact.1.dedup_window, auto.dedup_window);

    let mut too_small = WriteOnlySink::default();
    assert_eq!(
        encode_stream(
            &archive,
            StreamWriteOptions {
                window: StreamWindow::Ceiling(auto.dedup_window - 1),
                ..StreamWriteOptions::default()
            },
            &mut too_small,
        )
        .unwrap_err()
        .code(),
        ReasonCode::StreamWindowExceeded
    );
}

#[test]
fn a_reader_rejects_a_body_that_exceeds_its_declared_window() {
    let archive = shared_reference_archive();
    let (bytes, summary) = encode_to_vec(&archive, auto_window());
    assert!(summary.dedup_window > 0);

    // Re-declare a zero window without touching the body, and repair the
    // footer binding so the window rule is the only thing left to fail.
    let mut tampered = bytes.clone();
    let preamble_len = usize::try_from(PREAMBLE_LEN).unwrap();
    tampered[160..168].copy_from_slice(&0_u64.to_be_bytes());
    let digest = sha256_exact(&tampered[..preamble_len]);
    let footer_start = tampered.len() - usize::try_from(STREAM_FOOTER_LEN).unwrap();
    tampered[footer_start + 64..footer_start + 96].copy_from_slice(digest.as_bytes());

    let error = open_stream_with_limits(
        DripSource::new(&tampered, 512),
        bootstrap_sequential_limits(),
    )
    .unwrap_err();
    assert_eq!(error.code(), ReasonCode::StreamWindowExceeded);
    assert_eq!(error.class(), OutcomeClass::Nonconforming);
}

// ---------------------------------------------------------------------------
// Truncation, framing, and ordering
// ---------------------------------------------------------------------------

#[test]
fn truncation_is_distinguishable_from_corruption() {
    let bytes = sample_stream("truncation");

    for cut in [
        0,
        8,
        usize::try_from(PREAMBLE_LEN).unwrap() - 1,
        usize::try_from(PREAMBLE_LEN).unwrap(),
        usize::try_from(PREAMBLE_LEN).unwrap() + 8,
        bytes.len() / 2,
        bytes.len() - usize::try_from(STREAM_FOOTER_LEN).unwrap(),
        bytes.len() - 40,
        bytes.len() - 1,
    ] {
        let error = open_stream_with_limits(
            DripSource::new(&bytes[..cut], 3),
            bootstrap_sequential_limits(),
        )
        .unwrap_err();
        assert_eq!(
            error.class(),
            OutcomeClass::Truncated,
            "cut at {cut} reported {error}"
        );
        assert_eq!(error.code(), ReasonCode::TruncatedStream);
    }

    // Trailing bytes after a complete footer are corruption, not truncation.
    let mut extended = bytes.clone();
    extended.push(0);
    let error = open_stream_with_limits(
        DripSource::new(&extended, 64),
        bootstrap_sequential_limits(),
    )
    .unwrap_err();
    assert_eq!(error.class(), OutcomeClass::Corrupt);
    assert_eq!(error.code(), ReasonCode::IncorrectTotalLength);
}

#[test]
fn an_unknown_item_tag_is_refused_rather_than_skipped() {
    let bytes = sample_stream("unknown-tag");
    let items = walk_items(&bytes);
    let first = items[0].offset;
    let mut tampered = bytes.clone();
    tampered[first + 4..first + 6].copy_from_slice(&0xfff0_u16.to_be_bytes());
    let error = open_stream_with_limits(
        DripSource::new(&tampered, 97),
        bootstrap_sequential_limits(),
    )
    .unwrap_err();
    assert_eq!(error.class(), OutcomeClass::Unsupported);
    assert_eq!(error.code(), ReasonCode::UnsupportedRequiredFeature);

    // A byte pattern that is neither an item header nor the footer is corrupt.
    let mut broken = bytes.clone();
    broken[first] ^= 0xff;
    let error =
        open_stream_with_limits(DripSource::new(&broken, 97), bootstrap_sequential_limits())
            .unwrap_err();
    assert_eq!(error.code(), ReasonCode::StreamItemOrdering);
}

#[test]
fn a_manifest_record_cannot_precede_the_data_it_describes() {
    let bytes = sample_stream("forward-reference");
    let items = walk_items(&bytes);
    let first_frame = items
        .iter()
        .find(|item| item.tag == StreamItemTag::ChunkFrame.wire_id())
        .copied()
        .expect("the body must contain a chunk frame");
    // An empty ContentObject legitimately precedes every frame, so pick the
    // first record that actually depends on physical data.
    let record = items
        .iter()
        .find(|item| {
            item.tag == StreamItemTag::ManifestRecord.wire_id() && item.offset > first_frame.offset
        })
        .copied()
        .expect("the body must contain a manifest record that follows its data");

    let mut reordered = Vec::with_capacity(bytes.len());
    reordered.extend_from_slice(&bytes[..first_frame.offset]);
    reordered.extend_from_slice(&bytes[record.offset..record.offset + record.len]);
    reordered.extend_from_slice(&bytes[first_frame.offset..record.offset]);
    reordered.extend_from_slice(&bytes[record.offset + record.len..]);
    assert_eq!(reordered.len(), bytes.len());

    let error = open_stream_with_limits(
        DripSource::new(&reordered, 256),
        bootstrap_sequential_limits(),
    )
    .unwrap_err();
    assert_eq!(error.code(), ReasonCode::StreamForwardReference);
    assert_eq!(error.class(), OutcomeClass::Nonconforming);
}

#[test]
fn a_duplicated_semantic_item_is_refused() {
    let bytes = sample_stream("duplicate-item");
    let items = walk_items(&bytes);
    let plans = items
        .iter()
        .find(|item| item.tag == StreamItemTag::TransformPlans.wire_id())
        .expect("the body must declare its TransformPlans");

    let mut duplicated = Vec::with_capacity(bytes.len() + plans.len);
    duplicated.extend_from_slice(&bytes[..plans.offset + plans.len]);
    duplicated.extend_from_slice(&bytes[plans.offset..plans.offset + plans.len]);
    duplicated.extend_from_slice(&bytes[plans.offset + plans.len..]);

    let error = open_stream_with_limits(
        DripSource::new(&duplicated, 1024),
        bootstrap_sequential_limits(),
    )
    .unwrap_err();
    assert_eq!(error.code(), ReasonCode::DuplicateSemanticDeclaration);
}

#[test]
fn a_corrupted_chunk_payload_is_reported_as_corruption() {
    let bytes = sample_stream("corrupt-chunk");
    let items = walk_items(&bytes);
    let frame = items
        .iter()
        .find(|item| item.tag == StreamItemTag::ChunkFrame.wire_id() && item.payload_len > 0)
        .expect("the body must contain a Chunk frame with stored bytes");

    let mut tampered = bytes.clone();
    let last = frame.offset + frame.len - 1;
    tampered[last] ^= 0xff;
    let error = open_stream_with_limits(
        DripSource::new(&tampered, 333),
        bootstrap_sequential_limits(),
    )
    .unwrap_err();
    assert_eq!(error.class(), OutcomeClass::Corrupt);
}

#[test]
fn a_corrupted_supporting_record_is_caught_by_its_own_digest() {
    let fixture = Fixture::new("corrupt-support");
    let source = fixture.path.join("source");
    write_similar_source(&source, 24, 64 * 1024, 0x6a09_e667_f3bc_c909);
    fs::write(
        source.join("unrelated.bin"),
        noise(64 * 1024, 0x510e_527f_ade6_82d1),
    )
    .unwrap();
    let indexed = pack(&source, CompressionProfile::Balanced);
    assert!(!indexed.archive.content_store.dictionaries.is_empty());
    let (bytes, _) = encode_to_vec(&indexed.archive, auto_window());

    let items = walk_items(&bytes);
    let dictionaries = items
        .iter()
        .find(|item| item.tag == StreamItemTag::Dictionaries.wire_id())
        .expect("the body must declare its Dictionaries");
    assert!(dictionaries.payload_len > 0);

    let mut tampered = bytes.clone();
    let target = dictionaries.offset + dictionaries.len - 1;
    tampered[target] ^= 0xff;
    let error = open_stream_with_limits(
        DripSource::new(&tampered, 999),
        bootstrap_sequential_limits(),
    )
    .unwrap_err();
    assert_eq!(error.code(), ReasonCode::SectionDigestMismatch);
    assert_eq!(error.class(), OutcomeClass::Corrupt);
}

// ---------------------------------------------------------------------------
// Resource policy, budget declaration, and staging
// ---------------------------------------------------------------------------

#[test]
fn a_caller_policy_refusal_stops_the_pass_before_it_completes() {
    let bytes = sample_stream("policy-refusal");
    let mut narrow = bootstrap_sequential_limits();
    narrow.max_container_bytes = u64::try_from(bytes.len()).unwrap() / 2;
    let error = open_stream_with_limits(DripSource::new(&bytes, 4096), narrow).unwrap_err();
    assert_eq!(error.class(), OutcomeClass::PolicyRefused);
    assert_eq!(error.code(), ReasonCode::ResourceLimit);

    let mut tiny_budget = bootstrap_sequential_limits();
    tiny_budget.budget = ResourceBudget {
        chunk_count: 1,
        ..bootstrap_resource_policy()
    };
    let error = open_stream_with_limits(DripSource::new(&bytes, 4096), tiny_budget).unwrap_err();
    assert_eq!(error.class(), OutcomeClass::PolicyRefused);
    assert_eq!(error.code(), ReasonCode::ResourceLimit);
}

#[test]
fn an_undeclared_budget_is_bounded_by_caller_policy_and_reported_honestly() {
    let fixture = Fixture::new("undeclared-budget");
    let source = fixture.path.join("source");
    write_rich_source(&source);
    let indexed = pack(&source, CompressionProfile::Balanced);

    let undeclared = StreamWriteOptions {
        window: StreamWindow::Auto,
        budget_declared: false,
    };
    let (bytes, summary) = encode_to_vec(&indexed.archive, undeclared);
    assert!(!summary.budget_declared);
    assert_eq!(summary.archive.descriptor.budget, ResourceBudget::default());
    assert_eq!(indexed.identities.lai, summary.identities.lai);
    assert_eq!(indexed.identities.pcr, summary.identities.pcr);

    let sequential =
        open_stream_with_limits(DripSource::new(&bytes, 512), bootstrap_sequential_limits())
            .unwrap();
    assert!(!sequential.stream.budget_declared);
    assert!(!sequential.opened.archive.descriptor.budget_declared);
    // Final actual totals still come from the footer.
    assert_eq!(
        sequential.stream.actual_entry_count,
        u64::try_from(indexed.archive.entry_set.len()).unwrap()
    );
    assert_eq!(
        sequential.stream.actual_chunk_count,
        u64::try_from(indexed.archive.content_store.chunks.len()).unwrap()
    );
    let view = inspect(&sequential.opened).unwrap();
    assert!(!view.budget_declared);

    // Absence of a declaration is never a claim of unlimited resources.
    let mut narrow = bootstrap_sequential_limits();
    narrow.budget = ResourceBudget {
        chunk_count: 2,
        ..bootstrap_resource_policy()
    };
    assert_eq!(
        open_stream_with_limits(DripSource::new(&bytes, 512), narrow)
            .unwrap_err()
            .code(),
        ReasonCode::ResourceLimit
    );
}

#[test]
fn staging_spills_within_its_memory_bound_and_leaves_nothing_behind() {
    let fixture = Fixture::new("staging");
    let source = fixture.path.join("source");
    write_similar_source(&source, 8, 96 * 1024, 0x3141_5926_5358_9793);
    let indexed = pack(&source, CompressionProfile::Balanced);
    let (bytes, _) = encode_to_vec(&indexed.archive, auto_window());

    let before = staging_files();
    let limits = SequentialLimits {
        staging: StagingLimits {
            memory_bytes: 8 * 1024,
            total_bytes: 64 * 1024 * 1024,
        },
        ..bootstrap_sequential_limits()
    };
    let restored = fixture.path.join("restored");
    let (report, stream) = unpack_stream(
        DripSource::new(&bytes, 4096),
        &restored,
        ExtractionPolicy::default(),
        limits,
    )
    .unwrap();
    assert!(report.entries_created > 0);
    assert!(stream.spilled_staging_bytes > 0, "the fixture must spill");
    assert!(stream.peak_resident_staging_bytes <= 8 * 1024 + 96 * 1024);
    assert_trees_equal(&source, &restored);
    assert_eq!(staging_files(), before);

    // A failure part-way through must clean up its temporary storage too.
    let truncated = &bytes[..bytes.len() - 64];
    let failed = fixture.path.join("failed");
    let error = unpack_stream(
        DripSource::new(truncated, 4096),
        &failed,
        ExtractionPolicy::default(),
        limits,
    )
    .unwrap_err();
    assert_eq!(error.class(), OutcomeClass::Truncated);
    assert_eq!(staging_files(), before);
    assert!(
        !failed.exists(),
        "a failed sequential pass must not create destination objects"
    );
}

#[test]
fn unverified_content_never_reaches_the_destination() {
    let fixture = Fixture::new("safe-extraction");
    let source = fixture.path.join("source");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("first.txt"), vec![b'a'; 64 * 1024]).unwrap();
    fs::write(source.join("second.txt"), vec![b'b'; 64 * 1024]).unwrap();
    let indexed = pack(&source, CompressionProfile::Balanced);
    let (bytes, _) = encode_to_vec(&indexed.archive, auto_window());

    // Corrupt the very last Chunk frame. Everything before it is valid, so a
    // careless extractor would already have written the earlier file.
    let items = walk_items(&bytes);
    let frame = items
        .iter()
        .rfind(|item| item.tag == StreamItemTag::ChunkFrame.wire_id() && item.payload_len > 0)
        .expect("the fixture must contain Chunk frames with stored bytes");
    let mut tampered = bytes.clone();
    tampered[frame.offset + frame.len - 1] ^= 0xff;

    let destination = fixture.path.join("destination");
    let error = unpack_stream(
        DripSource::new(&tampered, 4096),
        &destination,
        ExtractionPolicy::default(),
        bootstrap_sequential_limits(),
    )
    .unwrap_err();
    assert_eq!(error.class(), OutcomeClass::Corrupt);
    assert!(
        !destination.exists(),
        "no destination object may be created from unverified content"
    );

    // Existing objects are never replaced, even by a valid archive.
    let occupied = fixture.path.join("occupied");
    fs::create_dir(&occupied).unwrap();
    fs::write(occupied.join("first.txt"), b"existing").unwrap();
    let error = unpack_stream(
        DripSource::new(&bytes, 4096),
        &occupied,
        ExtractionPolicy::default(),
        bootstrap_sequential_limits(),
    )
    .unwrap_err();
    assert_eq!(error.code(), ReasonCode::ExtractionCollision);
    assert_eq!(fs::read(occupied.join("first.txt")).unwrap(), b"existing");
}

// ---------------------------------------------------------------------------
// Backward compatibility
// ---------------------------------------------------------------------------

#[test]
fn historical_indexed_archives_are_unchanged_and_readers_fail_closed() {
    let fixture = Fixture::new("compatibility");
    let source = fixture.path.join("source");
    write_rich_source(&source);
    let indexed = pack(&source, CompressionProfile::Balanced);

    // An INDEXED preamble still declares layout 1, no STREAM feature bit, and a
    // zero STREAM/hostility region.
    assert_eq!(indexed.bytes[72], 1);
    assert_eq!(
        indexed.archive.descriptor.features.incompat & FEATURE_STREAM_LAYOUT_V1,
        0
    );
    assert!(indexed.bytes[160..176].iter().all(|byte| *byte == 0));
    assert_eq!(indexed.archive.descriptor.stream_dedup_window, 0);
    assert_eq!(indexed.archive.descriptor.layout, Layout::Indexed);
    open(&indexed.bytes).unwrap();

    // A STREAM archive declares layout 2 and the required feature bit, and the
    // random-access reader refuses it rather than reinterpreting its bytes.
    let (bytes, summary) = encode_to_vec(&indexed.archive, auto_window());
    assert_eq!(bytes[72], 2);
    assert_ne!(
        summary.archive.descriptor.features.incompat & FEATURE_STREAM_LAYOUT_V1,
        0
    );
    let error = open(&bytes).unwrap_err();
    assert_eq!(error.class(), OutcomeClass::Unsupported);
    assert_eq!(error.code(), ReasonCode::UnsupportedRequiredFeature);

    // The sequential reader likewise refuses an INDEXED container.
    let error = open_stream_with_limits(
        DripSource::new(&indexed.bytes, 128),
        bootstrap_sequential_limits(),
    )
    .unwrap_err();
    assert_eq!(error.code(), ReasonCode::UnsupportedRequiredFeature);
}

#[test]
fn the_writer_is_deterministic_and_verification_is_repeatable() {
    let fixture = Fixture::new("determinism");
    let source = fixture.path.join("source");
    write_rich_source(&source);
    let indexed = pack(&source, CompressionProfile::Balanced);

    let (first, first_summary) = encode_to_vec(&indexed.archive, auto_window());
    let (second, second_summary) = encode_to_vec(&indexed.archive, auto_window());
    assert_eq!(first, second);
    assert_eq!(first_summary.identities, second_summary.identities);
    assert_eq!(first_summary.total_len, u64::try_from(first.len()).unwrap());
    assert_eq!(
        first_summary.body_len,
        first_summary.total_len - PREAMBLE_LEN - STREAM_FOOTER_LEN
    );

    let report =
        verify_stream_with_limits(DripSource::new(&first, 5), bootstrap_sequential_limits())
            .unwrap();
    assert!(report.canonical_encoding);
    assert!(report.chunk_integrity);
    assert!(report.content_integrity);
    assert!(report.lai && report.pcr && report.aux);
    assert!(report.pci_computed);
    assert_eq!(report.index_status, IndexStatus::NotApplicableStream);
    assert_eq!(report.identities, first_summary.identities);
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const fn auto_window() -> StreamWriteOptions {
    StreamWriteOptions {
        window: StreamWindow::Auto,
        budget_declared: true,
    }
}

const fn retaining_limits(limits: SequentialLimits) -> SequentialLimits {
    SequentialLimits {
        content: StreamContentPolicy::Retain,
        ..limits
    }
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

fn encode_to_vec(
    archive: &entrybound::eam::Archive,
    options: StreamWriteOptions,
) -> (Vec<u8>, StreamWriteSummary) {
    let mut sink = WriteOnlySink::default();
    let summary = encode_stream(archive, options, &mut sink).unwrap();
    (sink.bytes, summary)
}

fn sample_stream(name: &str) -> Vec<u8> {
    let fixture = Fixture::new(name);
    let source = fixture.path.join("source");
    write_rich_source(&source);
    let indexed = pack(&source, CompressionProfile::Balanced);
    encode_to_vec(&indexed.archive, auto_window()).0
}

/// One tagged item as it appears in a `STREAM_BODY`.
#[derive(Clone, Copy, Debug)]
struct Item {
    tag: u16,
    offset: usize,
    len: usize,
    payload_len: usize,
}

/// Walks the tagged body using only the documented framing rules.
fn walk_items(bytes: &[u8]) -> Vec<Item> {
    let header_len = usize::try_from(STREAM_ITEM_HEADER_LEN).unwrap();
    let mut cursor = usize::try_from(PREAMBLE_LEN).unwrap();
    let footer_start = bytes.len() - usize::try_from(STREAM_FOOTER_LEN).unwrap();
    let extended_frames = u64::from_be_bytes(bytes[16..24].try_into().unwrap()) & 0x1 != 0;
    let frame_header = usize::try_from(if extended_frames {
        CHUNK_FRAME_V2_HEADER_LEN
    } else {
        CHUNK_FRAME_HEADER_LEN
    })
    .unwrap();
    let mut items = Vec::new();
    while cursor < footer_start {
        assert_eq!(bytes[cursor..cursor + 4], STREAM_ITEM_MAGIC);
        let tag = u16::from_be_bytes(bytes[cursor + 4..cursor + 6].try_into().unwrap());
        let (payload_len, len) = if tag == StreamItemTag::ChunkFrame.wire_id() {
            let stored = usize::try_from(u64::from_be_bytes(
                bytes[cursor + header_len + 8..cursor + header_len + 16]
                    .try_into()
                    .unwrap(),
            ))
            .unwrap();
            (stored, header_len + frame_header + stored)
        } else {
            let declared = usize::try_from(u64::from_be_bytes(
                bytes[cursor + header_len..cursor + header_len + 8]
                    .try_into()
                    .unwrap(),
            ))
            .unwrap();
            (declared, header_len + 40 + declared)
        };
        items.push(Item {
            tag,
            offset: cursor,
            len,
            payload_len,
        });
        cursor += len;
    }
    assert_eq!(cursor, footer_start);
    items
}

fn staging_files() -> Vec<PathBuf> {
    let prefix = format!("entrybound-stage-{}-", std::process::id());
    let mut found = fs::read_dir(std::env::temp_dir())
        .map(|entries| {
            entries
                .filter_map(std::result::Result::ok)
                .map(|entry| entry.path())
                .filter(|path| {
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(|name| name.starts_with(&prefix))
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    found.sort();
    found
}

fn write_rich_source(source: &Path) {
    fs::create_dir_all(source.join("nested/deeper")).unwrap();
    fs::create_dir_all(source.join("empty-dir")).unwrap();
    fs::write(source.join("nested/hello.txt"), b"hello from the stream\n").unwrap();
    fs::write(
        source.join("nested/deeper/notes.md"),
        "# notes\n\nrepeated line\n".repeat(400),
    )
    .unwrap();
    fs::write(source.join("compressible.bin"), vec![b'x'; 192 * 1024]).unwrap();
    fs::write(source.join("structured.bin"), structured(128 * 1024)).unwrap();
    fs::write(source.join("incompressible.bin"), noise(128 * 1024, 0xdead)).unwrap();
    fs::write(source.join("empty"), []).unwrap();
}

/// Two logical objects that share one exact Chunk and each own one unique
/// Chunk. Constructing the ranges directly freezes the intended physical
/// dependency instead of relying on incidental CDC boundaries.
fn shared_reference_archive() -> Archive {
    let shared = structured(4096);
    let head = noise(1024, 0x0bad_cafe);
    let tail = noise(1024, 0x0fee_1900);

    let mut first_bytes = head.clone();
    first_bytes.extend_from_slice(&shared);
    let (first, first_chunks) = build_content_from_ranges(
        &first_bytes,
        &[0..head.len(), head.len()..first_bytes.len()],
        STORE_PLAN_ID,
    )
    .unwrap();

    let mut second_bytes = shared;
    second_bytes.extend_from_slice(&tail);
    let shared_len = second_bytes.len() - tail.len();
    let (second, second_chunks) = build_content_from_ranges(
        &second_bytes,
        &[0..shared_len, shared_len..second_bytes.len()],
        STORE_PLAN_ID,
    )
    .unwrap();

    let entries = EntrySet::new(vec![
        Entry::new(
            LogicalPath::from_utf8(["first.bin"]).unwrap(),
            EntryData::File {
                content: ContentRef::Internal(first.logical_digest),
            },
            MetadataSet::default(),
            EntryIdentity::default(),
        ),
        Entry::new(
            LogicalPath::from_utf8(["second.bin"]).unwrap(),
            EntryData::File {
                content: ContentRef::Internal(second.logical_digest),
            },
            MetadataSet::default(),
            EntryIdentity::default(),
        ),
    ])
    .unwrap();
    let objects = BTreeMap::from([
        (first.logical_digest, first),
        (second.logical_digest, second),
    ]);
    let mut chunks = first_chunks;
    chunks.extend(second_chunks);

    Archive {
        descriptor: ArchiveDescriptor {
            format_major: 0,
            format_minor: 1,
            format_namespace: entrybound::ecf::FORMAT_NAMESPACE.to_owned(),
            features: FeatureSet::default(),
            layout: Layout::Indexed,
            role: ArchiveRole::Complete,
            budget_declared: true,
            stream_dedup_window: 0,
            budget: ResourceBudget::default(),
            decode: DecodeRequirements::default(),
            identity_profile: IdentityProfile::IdentityV1,
            digest_algorithm: DigestAlgorithm::Sha256,
            planner_id: "stream-window-test/v1".to_owned(),
            chunker_id: "explicit-ranges/test-v1".to_owned(),
            lai: Digest::ZERO,
            pcr: Digest::ZERO,
            aux: Digest::ZERO,
            pci: None,
        },
        entry_set: entries,
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
        fidelity: FidelityReport {
            platform: "test".to_owned(),
            ..FidelityReport::default()
        },
        conversion: None,
        preservation: None,
        index: Index::default(),
    }
}

fn write_similar_source(source: &Path, count: usize, len: usize, seed: u64) {
    fs::create_dir_all(source).unwrap();
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

fn structured(len: usize) -> Vec<u8> {
    (0..len)
        .map(|index| ((index % 251) as u8).wrapping_add((index / 251) as u8))
        .collect()
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

fn assert_trees_equal(left: &Path, right: &Path) {
    assert_eq!(collect_tree(left), collect_tree(right));
}

fn collect_tree(root: &Path) -> BTreeMap<String, Option<Vec<u8>>> {
    let mut found = BTreeMap::new();
    let mut pending = vec![(root.to_path_buf(), String::new())];
    while let Some((directory, prefix)) = pending.pop() {
        let mut entries = fs::read_dir(&directory)
            .unwrap()
            .map(std::result::Result::unwrap)
            .collect::<Vec<_>>();
        entries.sort_by_key(std::fs::DirEntry::file_name);
        for entry in entries {
            let name = entry.file_name().to_string_lossy().into_owned();
            let key = if prefix.is_empty() {
                name
            } else {
                format!("{prefix}/{name}")
            };
            if entry.file_type().unwrap().is_dir() {
                found.insert(key.clone(), None);
                pending.push((entry.path(), key));
            } else {
                found.insert(key, Some(fs::read(entry.path()).unwrap()));
            }
        }
    }
    found
}

struct Fixture {
    path: PathBuf,
}

impl Fixture {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "entrybound-stream-{name}-{}-{}",
            std::process::id(),
            NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
