# The `research-internals` cargo feature

`crates/entrybound` declares a default-off cargo feature, `research-internals`.
When enabled, it exposes read-only research access to internal planner, codec,
transform, reconstruction, similarity, and ECF entry points under
`entrybound::research`. It exists so the research harness (`research/harness`,
Phase B2) can measure every candidate the production planner considers and
reproduce exact codec output, without forking production code and without
changing what production does.

The feature is research-only. No production build, CLI build, or release
enables it, and nothing in it is a stable API.

## Enabling it

```toml
# In a research harness crate (its own workspace):
entrybound = { path = "../../crates/entrybound", features = ["research-internals"] }
```

```sh
cargo test -p entrybound --features research-internals
cargo build -p entrybound-cli --features entrybound/research-internals   # identity checks only
```

`crates/entrybound/Cargo.toml` declares `research-internals = []`. The feature
has no dependencies, and the default feature set is unchanged (empty).
`entrybound-cli` does not enable it.

## The no-behavior-change rule

Enabling or disabling the feature must never change archive bytes, planner
decisions, diagnostics, or any other production behavior. The implementation
follows these constraints:

1. **Additive only.** Each production file gains one
   `#[cfg(feature = "research-internals")] pub mod research { ... }` child
   module, placed after all production items and before `#[cfg(test)] mod tests`.
   No production line is edited, moved, or removed, so production line numbers
   and panic locations do not change. `lib.rs` gains one gated re-export module.
2. **No hooks.** No production function body contains research code, callbacks,
   counters, or cfg branches. A child module can call its parent's private items,
   so every wrapper just forwards to the private production function.
3. **Thin wrappers.** Wrappers pass arguments and results through unchanged.
   Private result types are copied field by field into public mirror types
   (`codec::research::PlanMode`, `planner::research::CohortSelection`,
   `jpeg_reconstruction::research::{VerifiedJpegRepresentation, JpegAttemptFailure}`,
   `reconstruction::research::ReconstructCandidate`). There is one deliberate
   addition: `ecf::research::parse_chunk_frame_header` refuses a short slice
   before calling the internal parser, which assumes a complete header.
4. **The enumeration mirrors loops, not costs.** The candidate enumeration
   re-runs each production selection loop outside the production function
   bodies. It calls the same private candidate-list, encode, record-length, and
   qualification functions the planner calls, and records every intermediate
   value. It never re-implements a cost formula or a threshold.
5. **Drift is detected, not assumed away.** A research harness must check that
   `PlannedAssignment::differences(&planned)` is empty against an Archive planned
   by the real planner before it trusts an enumeration. The feature's tests
   enforce this for every planner generation and profile.

These constraints are enforced mechanically:

| Check | Where |
|---|---|
| Stripping every gated block (and the `[features]` table) leaves each production file byte-identical to the base commit, and no other production path changed | `internals_identity_tools.py additive-check`, run by both identity scripts |
| CLI archives are SHA-256 identical with and without the feature, for all four profiles in INDEXED and STREAM layouts over a fixed generated tree | `internals-identity-check.sh` (WSL, primary) and `internals-identity-check.ps1` (Windows) |
| The `entrybound` and `entrybound-cli` test suites run with and without the feature; `research_internals` passes and the failing-test sets are identical (empty on Windows) | same scripts |
| Clippy findings (all targets, lints reported) are identical with and without the feature, `-D warnings` results are identical both ways, and `cargo fmt --check` passes | same scripts |
| The enumeration's selected candidates equal the real planners' choices, and research codec output equals the stored payload bytes | `crates/entrybound/tests/research_internals.rs` |

Results are recorded in [`internals-identity-check.md`](internals-identity-check.md),
which the scripts generate.

**Pre-existing failures.** Two failures exist on `dev` without this feature,
and the scripts report them as `PRE-EXISTING` rather than hiding them:

- On the pinned research toolchain (1.98.1),
  `cargo clippy --workspace --all-targets -- -D warnings` fails. Clippy 1.98
  lints `clippy::chunks_exact_to_as_chunks` and `clippy::manual_is_multiple_of`
  fire in untouched code: `ecf/container.rs`, `legacy/sevenz.rs`,
  `crypto/signature.rs`, `crypto/mod.rs`, `tests/crypto_primitives.rs`, and on
  Linux `archive/filesystem.rs`. Fixing them would edit production code, which
  this feature must not do.
- On WSL, `cross_file_compression::small_cohort_stays_independent_and_v2_v3_remain_readable`
  fails. It expects `FEATURE_PLATFORM_SECURITY_METADATA_V1`, which a pack of the
  WSL temporary directory does not set. The test passes on Windows, and it also
  fails in WSL on a pristine `git archive` export of the base commit.

A gate is `PRE-EXISTING` only when the command fails identically with and
without the feature, and the additive-only audit has proved that the source
compiled without the feature is byte-identical to the base commit. The complete
comparison gates still have to pass: the clippy findings with lints reported
across all targets, and the failing-test sets.

## Exposed items

All paths are under `entrybound::research`. Each module re-exports
`crate::<module>::research`.

| Module | Items |
|---|---|
| `planner` | Constants: `MINIMUM_ZSTD_INPUT_BYTES`, `PROBE_BYTES`, `MINIMUM_COHORT_GAIN_BYTES`, `DICTIONARY_SECTION_OVERHEAD_BYTES`, `CHUNK_GROUP_SECTION_OVERHEAD_BYTES`, `RECONSTRUCTION_SECTION_OVERHEAD_BYTES`, `MINIMUM_RECONSTRUCTION_GAIN_BYTES`, `MINIMUM_RECONSTRUCTION_GAIN_BASIS_POINTS`. Candidate lists: `levels` (v1–v3), `dictionary_levels`, `lookback_levels`, `v4_candidates`, `reconstruction_candidates`, `reconstruction_max_chain`, `jpeg_region_candidates`. Costs: `complete_candidate_cost` (v4), `v5_independent_cost`, `v6_ordinary_payload_cost`, `independent_encoded_len`. Qualification rules: `qualifies_v4`, `qualifies_reconstruction`, `qualifies_against_independent`. Selection: `select_v4_plan`, `select_cohort_plan` returning `CohortSelection`. Cohort helpers: `train_cohort_dictionary`, `cohort_prefix`, `maximum_preceding_bytes` (lookback prefix sizing), `chunk_group_id`, `chunk_object_owners`. The candidate enumeration is described in the next section. |
| `codec` | Constants: codec identifiers, `ZSTD_WINDOW_LOG`, `ZSTD_WINDOW_BYTES`, `ZSTD_WORKING_SET_BYTES`, `SUPPORTED_LEVELS`, `SUPPORTED_LOOKBACKS`, `SUPPORTED_LZMA2_CONFIGURATIONS`, `ZSTD_DICTIONARY_FORMAT`, `ZSTD_DICTIONARY_CONSTRUCTION_PREFIX`, `SUPPORTED_DICTIONARY_CONSTRUCTIONS`. Plan constructors: `store_plan`, `zstd_plan`, `zstd_dictionary_plan`, `zstd_prefix_plan`, `zstd_transformed_plan`, `lz4_plan`, `lzma2_plan`, `with_pipeline`, `without_transforms`. Validation: `validate_plan`, `validate_plans`, `required_features`, `plan_mode` returning `PlanMode`. Decode requirements: `aggregate_decode_requirements`, `aggregate_archive_decode_requirements`, `zstd_decode_requirements`. Payloads: `encode_payload`, `encode_transformed_payload`, `encode_payload_with_{dictionary,prefix,reconstruction}`, `decode_payload`, `decode_transformed_payload`, `decode_payload_with_{dictionary,prefix,reconstruction}`. Dictionaries: `train_dictionary`, `validate_dictionary`. |
| `transform` | `DELTA8_ID`, `BYTE_SHUFFLE_ID`. Step constructors: `delta8_step`, `byte_shuffle_step`, `deflate_reconstruct_step`, `jpeg_reconstruct_step`. Pipelines: `validate_pipeline`, `required_features`, `forward_pipeline`, `forward_pipeline_with_reconstruction`, `inverse_pipeline`, `inverse_pipeline_with_reconstruction`, `intermediate_len`. Helpers: `display_step`, `is_reconstructive`, `is_whole_object_reconstructive`. |
| `reconstruction` (DEFLATE) | Limits: `MAX_RECONSTRUCTION_INPUT_BYTES`, `MAX_RECONSTRUCTION_DATA_BYTES`, `MAX_INTERMEDIATE_BYTES`, `RECONSTRUCTION_WORKING_SET_BYTES`, `MAX_EXPANSION_RATIO`. Identifiers: `DEFLATE_RECONSTRUCT_ID`, `DEFLATE_RECONSTRUCTION_FORMAT`. Functions: `parameters`, `validate_parameters`, `try_forward` returning `ReconstructCandidate`, `verified_forward`, `inverse`, `validate_data`. |
| `jpeg_reconstruction` | Limits: `MAX_JPEG_BYTES`, `MAX_JXL_BYTES`, `MAX_JPEG_PIXELS`, `JPEG_WORKING_SET_BYTES`, `MAX_REGION_CHUNKS`, `MAX_REGION_EXPANSION_RATIO`. Constants: `JPEG_RECONSTRUCT_ID`, `JPEG_RECONSTRUCT_PARAMETERS`, `REGION_MEMBER_PLAN_REF`. Functions: `verified_forward` returning `VerifiedJpegRepresentation` or `JpegAttemptFailure`, `inverse`, `validate_parameters`, `region_identity`. |
| `similarity` | `SHINGLE_BYTES`, `SHINGLE_STRIDE`, `MAX_SCANNED_SHINGLES`. Functions: `fingerprint` (bottom-k sketch items), `intersection_count`, `fnv1a`, `cohort_id`. |
| `ecf` | Record lengths: `encoded_transform_plan_len`, `encoded_transform_plan_v2_len`, `encoded_transform_plan_v3_len`, `encoded_dictionary_len`, `encoded_chunk_group_len`, `encoded_reconstruction_data_len`, `encoded_reconstruction_region_len`, `encoded_legacy_evidence_len`. Lookback: `physical_prefix_from_slices`. Frames: `chunk_frame_header_len`, `parse_chunk_frame_header` returning `ChunkFrameHeader`, `decode_frame_payload`. |

## Candidate enumeration (`entrybound::research::planner`)

For planner-optimality research the module can return every candidate the
production planner evaluates, with its cost components, qualification result,
and exclusion reason, plus the candidate the production rule selects.

| Function | Input | Output |
|---|---|---|
| `trace_chunk(profile, version, plaintext)` | One Chunk and a `PlannerVersion` (`V1`..`V6`) | `ChunkTrace`: the probe result, every Chunk-stage candidate in evaluation order, the v5 DEFLATE attempt, and `selected` (the index of the plan the Chunk stage assigns) |
| `trace_chunk_stage(archive, profile, version)` | An unplanned or planned Archive | `ChunkStageTrace`: a trace for every Chunk, plus the exact Archive state and TransformPlan table the cohort stage receives |
| `trace_cohort(archive, cohort, profile, stage_planner_id, plans)` | One `SimilarityCohort` and the cohort-stage state | `CohortTrace`: the independent cost split into components, the trained dictionary, every dictionary and lookback candidate, and the selected index (`None` keeps the independent assignment) |
| `trace_jpeg_object(archive, object, profile, plans, emitted_region_section)` | One ContentObject and the post-v5 state | `ObjectTrace`: the eligibility outcome, the ordinary cost, every region candidate, and the selected region or the audit reason |
| `trace_archive(archive, profile, version)` | A whole Archive (not mutated) | `ArchiveTrace`: all of the above, plus `PlannedAssignment`, the full physical planning state the rules imply |

Per-candidate fields:

- `payload_bytes`: codec output length. For cohorts this is summed over the members.
- `plan_record_bytes`: the TransformPlan record the stage charges. This is zero
  for v1–v3, the v1 record for v4 and cohort candidates, the v2 record for v5,
  and the v3 record for JPEG regions (zero when the plan is already recorded).
- `side_data_bytes`: ReconstructionData, Dictionary, or ChunkGroup record bytes
  plus section overhead. JPEG candidates split this into `region_record_bytes`
  and `section_header_bytes`.
- `complete_cost`: the value the production comparison uses.
- `qualified`: the stage's minimum-gain rule. `qualification_baseline` records
  what the rule compared against.
- `selected`: true for the production choice.
- `exclusion`: why a candidate is not the choice. `InsufficientGain` means it
  failed the gain rule. `NotCheaperThanIncumbent` means it qualified but did not
  strictly beat an earlier candidate (earlier candidates win ties).
  `Superseded` means a later candidate replaced it. `CarriedToNextStage` marks
  the v4 winner that became the v5 baseline.

`PlannerVersion::plan` runs the real `plan_archive*` function of that
generation, and `PlannedAssignment::differences(&planned)` lists every mismatch
against its result (`plan_ref`/`group_ref` per Chunk, TransformPlans,
dictionaries, ChunkGroups, ReconstructionData, fallbacks, regions, audits, and
physical order). The tests require an empty difference list for all six
generations under all four profiles. They also require that each final Chunk
assignment rebuilt from the selected candidates alone equals the real planner's
assignment, and that `select_cohort_plan` and `plan_archive_v6` agree with the
cohort and object traces.

Limitations:

- The enumeration re-encodes every candidate, so it costs about as much as
  planning itself. It is single-threaded, like the planner.
- `PlannedAssignment` does not compare the derived descriptor feature bits or
  decode requirements.
- `ChunkTrace.selected` is the Chunk-stage choice. A cohort or JPEG region can
  override it later; the cohort and object traces record that.
- For JPEG regions, production applies the gain rule only to the cheapest
  candidate. `qualified` on the other region candidates is informational.
- CDC parameter selection (`chunker::select_parameters`) is already public and
  is not traced here.

## Stability and promotion

Nothing under `entrybound::research` is a stable or supported API. Names,
signatures, and trace structures may change whenever the production planner
changes, and the tests fail if a mirror drifts. **An accepted Decision Ledger
entry (`research/decision-ledger.jsonl`) is required before any research API
is promoted to a stable production API, or before any research finding changes
production planner behavior.** That promotion is a separate, reviewed
production change with its own compatibility and format analysis. It does not
happen by removing the `cfg` gate.
