# `ebr-crossfile`: tooling design for the cross-file experiments (NOT BUILT)

Status: **design only, pre-registered with Phase C-design** (`research/experiments/EXP-CROSSFILE-*`).
No code exists. Every experiment whose index row lists `tooling_to_build` containing
`ebr-crossfile` is `NEEDS_TOOLING` until this crate exists, passes the validity controls in
§4, and a harness review (like `harness-review-round1.md`) has checked it.

It is a new member crate of the research harness workspace:
`research/harness/crates/ebr-crossfile` (add it to `research/harness/Cargo.toml` `members`),
binary `ebr-crossfile`, depending on `entrybound` by path with `research-internals`
(`research/harness/research-internals.md`), `ebr-common`, and `ebr-pack` (for
`ebr_pack::bytes` exact byte accounting). Workspace lints forbid local `unsafe`.

## 1. Shared contract

- Flags: `--item-path`, `--item-id`, `--experiment-id`, `--env-name`, `--run-id`, `--out`
  follow `research/harness/README.md`. With `--out` omitted the binary prints exactly one
  `ebr.harness.raw.v1` JSON line whose `payload` carries the fields named below, so ebr
  specs read them with `source: stdout_json, key: payload.<field>`.
- Every input path passes `ebr_common::heldout::assert_inputs_not_heldout` first.
- Chunking and the Chunk stage are always production: `entrybound::research::planner::
  trace_chunk_stage(archive, profile, PlannerVersion::V6)` over the archive
  `entrybound::archive::plan_directory` captures (planner v6 runs the frozen v5 Chunk and
  cohort stages). Research candidates replace only the stage the experiment varies.
- Cohort-stage memoisation: `select_cohort_plan` results are cached under
  `--memo-dir` keyed by SHA-256 over (profile, stage planner id, ordered cohort Chunk
  digests, SHA-256 of the canonical TransformPlan table, candidate rule id). The memo
  directory is per repetition (`.../rep{rep}`) so the second repetition of a deterministic
  metric recomputes everything and the repeat-hash check (decision-method §4.13) is real.
- Counting allocator: a pinned third-party `GlobalAlloc` wrapper crate (local `unsafe` is
  forbidden) providing current and peak allocated bytes with scoped reset; this is the
  in-process `alloc_peak_bytes` oracle of decision-method §14 item 17 for these binaries.
  Allocation-site attribution is out of scope here and remains §14 item 17's gate.
- Access sample: the (file, offset, length) sample for the strata {1 B, 4 KiB, 1 MiB,
  whole entry} is exactly the procedure of
  `research/experiments/EXP-CROSSFILE-005/solid_access.py` (`sample_plan`, seed =
  first 8 bytes of SHA-256(`EXP-CROSSFILE access-sample v1\0` || item_id)), so Entrybound
  and incumbent amplification are computed on identical byte ranges. A Rust port must
  reproduce the Python sample exactly (unit test against the script's output).

## 2. Subcommands

| Subcommand | Varies | Output payload fields (canonical names where they exist) |
|---|---|---|
| `similarity` | cohort formation method and parameters (`--method`, `--method-params`); planning and encoding otherwise production | `artifact_bytes`, `dictionary_bytes`, `side_data_bytes`, `cohort_count`, `cohort_chunks`, `cohorts_dictionary_selected`, `cohorts_lookback_selected`, `cohorts_rejected`, `rejected_candidate_encodes`, Stage-A quality fields when `--ground-truth exact|sampled`: `pair_recall_j250`, `pair_recall_j375`, `pair_recall_j500`, `pair_precision_j250`, `pair_precision_j375`, `pair_precision_j500`, `sketch_retention_insert_1b`, `sketch_retention_insert_16b`, `sketch_retention_insert_256b`; `drift_ok` (status-quo method only: `PlannedAssignment::differences` empty and archive SHA-256 equals `plan_archive_v6`+`ecf::encode` output), `roundtrip_ok`, `archive_sha256` |
| `order` | physical base order and cohort placement (`--order`), clustering mode per profile | `artifact_bytes`, `stream_artifact_bytes`, `stream_window_required_bytes`, `archive_sha256`, `roundtrip_ok`; with `--remote-tasks true`: per task `http_requests_<task>`, `http_bytes_<task>`, and `remote_task_latency_s_<task>_<profile>` modelled per decision-method §4.15 (slow start on and off) |
| `dict-sweep` | dictionary construction, size, samples, trainer, scope, acceptance rule (`--rule`, `--construction`) | `artifact_bytes`, `dictionary_bytes`, `dictionaries_accepted`, `dictionary_chunks`, `net_savings_bytes`, `accept_flips_margin_x0_5`, `accept_flips_margin_x2`, `http_bytes_cold_member_read`, `http_requests_cold_member_read`, `alloc_peak_bytes_member_read`, `declared_working_set_bytes`, `declared_tightness_ratio_working_set`, `roundtrip_ok`, `archive_sha256`, `encodable_by_production` |
| `topology` | lookback depth, prefix window, dependency topology, group caps, bounded-solid blocks, declaration model (`--topology`, `--declaration`) | `artifact_bytes`, `artifact_bytes_source` (`encoded` or `modelled`), `side_data_bytes`, `access_granularity_bytes`, `access_amplification_1b`, `access_amplification_4k`, `access_amplification_1m`, `access_amplification_entry`, `closure_chunks_max`, `closure_chunks_mean`, `closure_decoded_bytes_max`, `closure_decoded_bytes_mean`, `declared_preceding_bytes_max`, `declared_tightness_ratio_access`, `declared_cost_accuracy`, `blast_logical_bytes_p95_payload`, `blast_logical_bytes_max_payload`, `blast_entries_p95_payload`, `blast_logical_bytes_p95_dictionary`, `workspan_speedup_bound_n{2,4,8,16}`, `http_bytes_random_entry`, `http_requests_random_entry`, `remote_random_entry_latency_s_<profile>` (modelled), `localization_recall_<definition>`, `localization_precision_<definition>`, `cost_model_tightness_<model>`, `cost_model_accuracy_<model>`, `roundtrip_ok` |
| `access` | nothing (measures production archives written by the subcommands above or by `ebound pack`) | per source kind (`memory`, `local-file`, `http` via a child `ebr-origin`): `decoded_logical_bytes_per_read` (p50, p95, max), `dependency_chunk_count_per_read` (p50, p95, max), `bytes_fetched_by_purpose` (Chunk, Dictionary, Lookback, Index, Manifest, ...), `http_requests_per_read`, `http_bytes_per_read`, `alloc_peak_bytes_per_read`, declared values (`working_set_bytes`, `max_preceding_bytes`) and each cost model's prediction, `declared_cost_accuracy`, `declared_tightness_ratio_*` |
| `inject` | seeded corruption (`--stratum payload|index|metadata|dictionary|integrity`, `--events N`, `--class byte|512b|64k|1m|truncate`) | per event and aggregated: `blast_logical_bytes_p95`, `blast_logical_bytes_max`, `blast_entries_p95` per stratum, `detection_rate` (must be 1), `localization_recall`, `localization_precision` of `ebound verify`'s affected-entry report against the read-every-entry oracle, `analytic_model_agreement` (exact set equality with `topology`'s closure prediction) |
| `decode-bench` | nothing in the format; decoder variant (`--decoder production|iterative-bounded|dag-parallel`), thread count | in-process batch timers (>= 1,000 operations per batch, >= 40 sampled per round): `first_entry_latency_s`, `random_entry_latency_s` (p50, p95), `decode_wall_s`, `decode_cpu_s`, `alloc_peak_bytes`; `dag-parallel` also `decode_wall_s_n{1,2,4,8}` for `parallel_speedup` |
| `plan-cost` | planner stage cost; `--hostile-series` generator | in-process stage timers and allocator peaks: `similarity_cpu_s`, `training_cpu_s`, `cohort_search_cpu_s`, `ordering_cpu_s`, `region_stage_cpu_s`, `encode_cpu_s` (sum), `alloc_peak_bytes`; with `--hostile-series`: per element count 2^10..2^20 the CPU seconds for the fit of objective MVT-06(d) |
| `region-conflict` | order and conflict rule of the v6 whole-object region stage versus the cohort stage (`--rule`) | `artifact_bytes`, `regions_selected`, `regions_suppressed_dedup`, `regions_suppressed_cohort_dictionary`, `regions_suppressed_cohort_group`, `audit_reason_specificity_fraction`, `roundtrip_ok`, `archive_sha256` |

Additional flags and subcommands the specs use:

| Item | Meaning |
|---|---|
| `--synthetic-from-item-id true` (`topology`, `access`, `inject`) | When the item id starts with `xf004-long-group-<n>`, ignore `--item-path` (a 1-byte placeholder generated item) and build the stress tree: n near-identical 64 KiB files with seeded 1% byte edits (seed from the spec's generated-item seed), sized so dense and extreme form one maximal cohort |
| `dict-sweep --adversarial-set v1` | The 8 hostile supplied-Dictionary inputs of EXP-CROSSFILE-003 §4, through `entrybound::research::codec::validate_dictionary` and the research embed path; prints `adversarial_true_refusal_fraction`, `refusal_alloc_peak_bytes`, `refusal_bytes_read`, `adversarial_outcomes_sha256` |
| `access --build ebr-pack|topology|dict-sweep --build-args <config>` | How the measured archive is produced (production-encodable configurations only; refuses others) |
| `decode-bench --prepare` | Untimed pass that writes `/root/eb-research/scratch/EXP-CROSSFILE-008/<item>/<config>.eb` and records SHA-256 |
| `decode-bench --speedup-pair k --pin-base-cpu 2` | Within one sample, a T(1) batch pinned to vCPU 2 and a T(k) batch pinned to 2..k+1 in seeded order; prints `decode_wall_s_n1`, `decode_wall_s_nk`, `parallel_speedup`, `parallel_efficiency`, `output_identity_ok` |
| `plan-cost --hostile-series shared-shingle|colliding-bucket --points 10..20 --runs-per-point 3` | MVT-06(d) series in-process; prints the fitted exponent, its 0.99 interval, CPU at 2^20 and `mvt_06d_violation` |
| `determinism-cell --vectors <dir> --profiles balanced,dense,extreme --archive-dir <dir>` | EXP-CROSSFILE-010 cell: train every vector file, pack the input at each profile with `sq-abs128`, print `dictionary_vector_set_sha256`, `archive_set_sha256`, `crossfile_physical_set_sha256`, `vectors_trained`, `artifact_bytes_total`; built per libzstd release under `research/harness/zstd-drift/` |
| `similarity|dict-sweep --emit-vectors <dir>` | Writes each attempted training sample set as a vector file (EXP-CROSSFILE-010 input) |

## 3. Research encoders for configurations production cannot write

- Chained prefix, window `w`, lookback `k` (including `k` in {16, 32} and `w` != 1 MiB),
  leader-only star, restart interval, group-length cap, anchor (non-transitive) topology:
  Zstandard reference-prefix compression through the pinned `zstd = "=0.13.3"` crate
  (same libzstd as production), with the prefix built exactly as
  `entrybound::research::ecf::physical_prefix_from_slices` builds it for the registered
  case, generalised to `w`. Frame parameters otherwise equal `zstd_prefix_plan`.
- Long-distance matching: the same prefix API with `enableLongDistanceMatching` and
  `windowLog` in {23, 27}; the declared decoder window equals `2^windowLog`.
- Bounded-solid blocks: one Zstandard frame per contiguous run of cohort members whose
  plaintext sum is at most `B`; a byte is decodable after decoding its block prefix.
- Records these configurations would need are **modelled**, not encoded:
  `side_data_bytes` = production `encoded_chunk_group_len` / `encoded_transform_plan_len`
  over a hypothetical parameter encoding with the same field widths, plus one 64-byte
  section header; bounded-solid blocks are charged 64 bytes of section header plus 48
  bytes per block (an assumption recorded here before any result; the experiments run a
  sensitivity arm at 0 and 96 bytes per block). Payload bytes are always real encoder
  output. `artifact_bytes_source = modelled` marks every such value.
- Dictionary constructions beyond production's nine: COVER and fastCOVER with parameter
  optimisation (`ZDICT_optimizeTrainFromBuffer_cover`/`_fastCover`) and the legacy
  trainer through `zstd-sys` experimental bindings in this crate only; raw-content
  dictionaries; LZMA2 preset dictionary and LZ4 block dictionary for DEC-CMP-014 arms,
  with the dependency versions pinned in `Cargo.toml` and recorded in every row.
  Which algorithm `ZDICT_trainFromBuffer` runs at libzstd 1.5.7 is established by citing
  `lib/dictBuilder/zdict.c` at the pinned `zstd-sys` source (decision-method §9.3), not by
  recollection.

## 4. Validity controls (each run fails closed)

1. **Production reproduction.** For every configuration production can write (status-quo
   similarity, digest order, frozen dictionary rules, registered lookbacks at 1 MiB), the
   subcommand's archive SHA-256 equals the archive `ebr-pack --profile <p>` writes for the
   same item, and `PlannedAssignment::differences` is empty. A mismatch invalidates the run.
2. **Round trip.** Production-encodable archives: `entrybound::ecf::open` + verification +
   every entry byte-equal to the source tree. Research-only topologies: every Chunk decoded
   through the research decoder along its dependency DAG equals its plaintext SHA-256.
   Failures are HC-02 violation records (objective §2.0 rule 2), never averaged.
3. **Byte accounting.** `ebr_pack::bytes` named buckets sum exactly to the file size for
   encoded archives; for modelled values, payload + modelled records reconcile to
   `artifact_bytes` exactly.
4. **Analytic versus measured.** The `topology` closure, amplification and payload
   blast-radius predictions for production-encodable configurations must equal `access`
   and `inject` measurements exactly (EXP-CROSSFILE-006); until they do, `modelled`
   values for research-only topologies are not decision-grade.
5. **Sample identity.** The Rust access sample equals `solid_access.py`'s sample for the
   same item (unit test over three generated trees).
6. **Spec-key conformance.** The `stdout_json` keys the EXP-CROSSFILE specs read are normative
   (the tables above are summaries). A crate test loads every
   `research/experiments/EXP-CROSSFILE-*/spec*.yaml`, runs each referenced subcommand on a tiny
   generated tree, and asserts that every `payload.*` key the spec reads is present and typed as
   the spec's metric kind; a missing key fails the build of the campaign, not the run.
7. `lock_parity.py` and `harness-cli-parity.sh` print `RESULT PASS` before any campaign
   (harness review use constraint 1). Timing comparisons against incumbents use
   production-built `ebound` or first show timing parity (use constraint 2).

## 5. Estimated build effort

About 4-6k lines of Rust plus tests; judgment-level agent work for the similarity methods
(MinHash-LSH, SimHash, TLSH, Nilsimsa ordering, Finesse and N-transform super-features,
content-defined sampling) whose reference definitions must be cited to primary sources
(papers, DwarFS v0.15.7 and TLSH reference source at pinned commits), and scripted work for
the rest. A harness review round is required before any C-exec use.
