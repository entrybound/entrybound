//! Planner candidate traces, exhaustive search, and regret research, via
//! `entrybound`'s default-off `research-internals` feature
//! (`entrybound::research::planner`).
//!
//! `research/harness/research-internals.md` states the contract this crate
//! exists to check: "a research harness must check that
//! `PlannedAssignment::differences(&planned)` is empty against an Archive
//! planned by the real planner before it trusts an enumeration." That check
//! is [`check_enumeration_matches_real_planner`]. It reuses the same minimal
//! unplanned-`Archive` construction `crates/entrybound/tests/research_internals.rs`
//! (the Internals agent's own proof) already uses, over public
//! `entrybound::eam`/`entrybound::identity` items -- not a copy of that
//! test file, but the same, intentionally minimal, bootstrap-archive shape.
//!
//! "Regret" here means exactly what the enumeration's own `exclusion` field
//! records for every non-selected candidate (`InsufficientGain`,
//! `NotCheaperThanIncumbent`, `Superseded`; see
//! `research-internals.md`'s "Candidate enumeration" table) -- the cost gap
//! between a candidate and the one actually selected. [`candidate_regret`]
//! computes it directly from one chunk's trace.

pub mod exhaustive;
pub mod strategies;

use entrybound::chunker::ChunkingParameters;
use entrybound::diagnostics::Diagnostic;
use entrybound::eam::{
    Archive, ArchiveDescriptor, ArchiveRole, ContentRef, ContentStore, DecodeRequirements, Digest,
    DigestAlgorithm, Entry, EntryData, EntryIdentity, EntrySet, FeatureSet, FidelityReport,
    IdentityProfile, Index, Layout, LogicalPath, MetadataSet, ResourceBudget,
};
use entrybound::identity::{build_content, build_content_from_ranges};
use entrybound::planner::{CompressionProfile, UNPLANNED_PLAN_ID};
use entrybound::research::codec;
use entrybound::research::planner::{ArchiveTrace, ChunkTrace, PlannerVersion, trace_archive};
use serde::Serialize;
use std::collections::BTreeMap;

/// One named, fixed-size-chunked input for [`archive_for`].
pub struct NamedInput {
    pub name: &'static str,
    pub bytes: Vec<u8>,
    pub chunk_size: usize,
}

/// Builds a minimal unplanned `Archive` from in-memory inputs: the same
/// bootstrap shape `entrybound`'s own research-internals test suite uses to
/// drive the planner research API (`ArchiveDescriptor.planner_id:
/// "unplanned"`, `transform_plans: [store_plan()]`, one `Entry::File` per
/// input). It is not what `entrybound::archive::plan_directory` builds from
/// a real filesystem tree (that path also runs the real planner before
/// returning); this is deliberately the *pre-planning* state
/// `PlannerVersion::plan`/`trace_archive` both expect.
pub fn archive_for(inputs: &[NamedInput]) -> Result<Archive, Diagnostic> {
    let mut built = Vec::with_capacity(inputs.len());
    for input in inputs {
        built.push((
            input.name,
            build_content(&input.bytes, input.chunk_size, UNPLANNED_PLAN_ID)?,
        ));
    }
    assemble_unplanned(built)
}

/// The Chunk ranges production's own content-defined chunker assigns to one
/// object's bytes under `profile`: `chunker::select_parameters` over the
/// profile's `chunking_candidates()`, then `chunker::chunk_ranges` -- the
/// same two calls `entrybound::archive::plan_directory` makes for a
/// single-object item.
pub fn production_chunk_ranges(
    bytes: &[u8],
    profile: CompressionProfile,
) -> Result<(ChunkingParameters, Vec<std::ops::Range<usize>>), Diagnostic> {
    if bytes.is_empty() {
        return Ok((profile.chunking_candidates()[0], Vec::new()));
    }
    let refs: [&[u8]; 1] = [bytes];
    let evaluation = entrybound::chunker::select_parameters(&refs, profile.chunking_candidates())?;
    let ranges = entrybound::chunker::chunk_ranges(bytes, evaluation.parameters)?
        .iter()
        .map(|range| range.start..range.end)
        .collect();
    Ok((evaluation.parameters, ranges))
}

/// [`archive_for`] for one object split at explicit Chunk ranges (typically
/// [`production_chunk_ranges`]), so planner research sees the Chunks
/// production would plan rather than one whole-file Chunk.
pub fn archive_for_ranges(
    name: &'static str,
    bytes: &[u8],
    ranges: &[std::ops::Range<usize>],
) -> Result<Archive, Diagnostic> {
    let built = vec![(
        name,
        build_content_from_ranges(bytes, ranges, UNPLANNED_PLAN_ID)?,
    )];
    assemble_unplanned(built)
}

type BuiltContent = (
    entrybound::eam::ContentObject,
    BTreeMap<Digest, entrybound::eam::Chunk>,
);

fn assemble_unplanned(built: Vec<(&'static str, BuiltContent)>) -> Result<Archive, Diagnostic> {
    let mut objects = BTreeMap::new();
    let mut chunks = BTreeMap::new();
    let mut entries = Vec::new();
    for (name, (object, object_chunks)) in built {
        let input_name = name;
        entries.push(Entry::new(
            LogicalPath::from_utf8([input_name])?,
            EntryData::File {
                content: ContentRef::Internal(object.logical_digest),
            },
            MetadataSet::default(),
            EntryIdentity::default(),
        ));
        objects.insert(object.logical_digest, object);
        chunks.extend(object_chunks);
    }
    Ok(Archive {
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
            chunker_id: "ebr-planner-fixed/v1".to_owned(),
            lai: Digest::ZERO,
            pcr: Digest::ZERO,
            aux: Digest::ZERO,
            pci: None,
        },
        entry_set: EntrySet::new(entries)?,
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
    })
}

/// Whether the research candidate enumeration reproduces the real planner's
/// choices for one unplanned archive, at one profile and planner
/// generation. An empty `differences` means it does.
#[derive(Debug, Clone, Serialize)]
pub struct DriftReport {
    pub profile: &'static str,
    pub version: &'static str,
    pub differences: Vec<String>,
}

impl DriftReport {
    #[must_use]
    pub fn matches(&self) -> bool {
        self.differences.is_empty()
    }
}

fn version_label(version: PlannerVersion) -> &'static str {
    match version {
        PlannerVersion::V1 => "v1",
        PlannerVersion::V2 => "v2",
        PlannerVersion::V3 => "v3",
        PlannerVersion::V4 => "v4",
        PlannerVersion::V5 => "v5",
        PlannerVersion::V6 => "v6",
    }
}

/// Runs both the real planner (`PlannerVersion::plan`, on a clone) and the
/// research enumeration (`trace_archive`) over the SAME unplanned archive,
/// and reports any difference between them.
pub fn check_enumeration_matches_real_planner(
    unplanned: &Archive,
    profile: CompressionProfile,
    version: PlannerVersion,
) -> Result<DriftReport, Diagnostic> {
    let trace: ArchiveTrace = trace_archive(unplanned, profile, version)?;

    let mut planned = unplanned.clone();
    version.plan(&mut planned, profile)?;

    let differences = trace.assignment.differences(&planned);
    Ok(DriftReport {
        profile: profile.as_str(),
        version: version_label(version),
        differences,
    })
}

/// The gap, in the enumeration's own `complete_cost` units, between one
/// candidate and the candidate the Chunk stage actually selected. Zero for
/// the selected candidate itself. This is "regret" in the research sense:
/// how much worse an alternative choice would have been.
#[must_use]
pub fn candidate_regret(trace: &ChunkTrace) -> Vec<i128> {
    let selected_cost = trace
        .candidates
        .get(trace.selected)
        .map(|c| i128::from(c.complete_cost))
        .unwrap_or(0);
    trace
        .candidates
        .iter()
        .map(|c| i128::from(c.complete_cost) - selected_cost)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use entrybound::research::planner::trace_chunk;

    fn text_bytes(prefix: &str, len: usize) -> Vec<u8> {
        prefix
            .bytes()
            .chain(b"the quick brown fox jumps over the lazy dog; entrybound research harness planner test; ".iter().copied())
            .cycle()
            .take(len)
            .collect()
    }

    fn sample_inputs() -> Vec<NamedInput> {
        // Distinct content per input (the `a`/`b` prefix), so the two
        // entries land in distinct, content-addressed chunks rather than
        // deduplicating to one -- that dedup is correct system behavior, not
        // a bug, but it would make this fixture useless for tests that want
        // to exercise more than one chunk.
        vec![
            NamedInput {
                name: "a.txt",
                bytes: text_bytes("a", 20 * 1024),
                chunk_size: 20 * 1024,
            },
            NamedInput {
                name: "b.txt",
                bytes: text_bytes("b", 20 * 1024),
                chunk_size: 20 * 1024,
            },
        ]
    }

    #[test]
    fn archive_for_builds_one_chunk_per_input() {
        let archive = archive_for(&sample_inputs()).unwrap();
        assert_eq!(archive.content_store.chunks.len(), 2);
        assert_eq!(archive.entry_set.entries().len(), 2);
        assert_eq!(archive.descriptor.planner_id, "unplanned");
    }

    #[test]
    fn enumeration_matches_the_real_planner_for_every_generation_and_profile() {
        let unplanned = archive_for(&sample_inputs()).unwrap();
        for profile in [
            CompressionProfile::Fast,
            CompressionProfile::Balanced,
            CompressionProfile::Dense,
            CompressionProfile::Extreme,
        ] {
            for version in PlannerVersion::ALL {
                let report =
                    check_enumeration_matches_real_planner(&unplanned, profile, version).unwrap();
                assert!(
                    report.matches(),
                    "enumeration drifted from the real planner for {profile:?}/{version:?}: {:?}",
                    report.differences
                );
            }
        }
    }

    #[test]
    fn candidate_regret_is_zero_at_the_selected_candidate() {
        let bytes = text_bytes("regret", 20 * 1024);
        let trace = trace_chunk(CompressionProfile::Balanced, PlannerVersion::V6, &bytes).unwrap();
        let regret = candidate_regret(&trace);
        assert_eq!(regret[trace.selected], 0);
    }

    /// A deterministic xorshift64* stream: unlike `entrybound`'s own private
    /// `deterministic_noise` test helper (not exposed to this crate), this is
    /// this crate's own, but built the same way -- a fixed seed, no OS
    /// randomness, so every run of every test using it sees identical bytes.
    fn generated_noise(seed: u64, len: usize) -> Vec<u8> {
        let mut state = seed | 1;
        (0..len)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                state as u8
            })
            .collect()
    }

    /// A handful of small, varied, synthetic inputs -- repetitive text,
    /// pseudo-random noise, a sparse mostly-zero buffer, and a little-endian
    /// numeric sequence (the shape `select_v4_plan`'s own transform-candidate
    /// tests use to trigger delta/shuffle transforms). None of these are
    /// recognizable DEFLATE/gzip streams, so no v5 reconstruction candidate
    /// can win over them -- see `exhaustive.rs`'s module docs' "Scope".
    fn generated_inputs() -> Vec<(&'static str, Vec<u8>)> {
        let sparse = {
            let mut bytes = vec![0_u8; 8 * 1024];
            for (index, byte) in bytes.iter_mut().enumerate().step_by(97) {
                *byte = (index % 251) as u8;
            }
            bytes
        };
        let numeric = (0_u32..2_048).flat_map(u32::to_le_bytes).collect();
        vec![
            ("repeating_text", text_bytes("gen", 8 * 1024)),
            ("noise", generated_noise(0x1234_5678_9abc_def1, 8 * 1024)),
            ("sparse_zeros", sparse),
            ("numeric_sequence", numeric),
        ]
    }

    #[test]
    fn trace_reconstruction_selects_exactly_what_plan_archive_v6_selected_on_generated_inputs() {
        for (name, bytes) in generated_inputs() {
            let chunk_size = bytes.len().max(1);
            let inputs = [NamedInput {
                name: "item",
                bytes: bytes.clone(),
                chunk_size,
            }];
            let unplanned = archive_for(&inputs).unwrap();
            let report = check_enumeration_matches_real_planner(
                &unplanned,
                CompressionProfile::Extreme,
                PlannerVersion::V6,
            )
            .unwrap();
            assert!(
                report.matches(),
                "{name}: trace reconstruction drifted from plan_archive_v6: {:?}",
                report.differences
            );
        }
    }

    #[test]
    fn exhaustive_search_never_exceeds_the_production_choice() {
        use crate::exhaustive::{SearchCaps, chunk_regret, search_chunk};
        use entrybound::research::planner::CandidateStage;

        for (name, bytes) in generated_inputs() {
            // `search_chunk` depends only on the plaintext, not on
            // profile/version, so it is computed once per input and reused
            // for every regret check below.
            let search = search_chunk(&bytes, &SearchCaps::full()).unwrap();
            for profile in [
                CompressionProfile::Fast,
                CompressionProfile::Balanced,
                CompressionProfile::Dense,
                CompressionProfile::Extreme,
            ] {
                for version in PlannerVersion::ALL {
                    let trace = trace_chunk(profile, version, &bytes).unwrap();
                    let selected = &trace.candidates[trace.selected];
                    // Out of this module's declared scope (see
                    // `exhaustive.rs`'s module docs' "Scope"); none of this
                    // test's generated inputs are DEFLATE/gzip streams, so
                    // this should never actually fire -- it is a guard, not
                    // a skip.
                    assert_ne!(
                        selected.stage,
                        CandidateStage::V5Reconstruction,
                        "{name}/{profile:?}/{version:?}: selected a reconstructive candidate, \
                         out of this test's scope"
                    );
                    let regret = chunk_regret(trace.selected_plan(), &bytes, &search).unwrap();
                    assert!(
                        regret >= 0,
                        "{name}/{profile:?}/{version:?}: regret={regret} \
                         (production complete_cost recomputed under v5_independent_cost minus \
                         this crate's exhaustive optimum {}), selected plan {:?}",
                        search.optimal_cost,
                        trace.selected_plan()
                    );
                }
            }
        }
    }
}
