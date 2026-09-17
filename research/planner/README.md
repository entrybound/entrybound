# Planner domain (program §9, §10): shared experiment conventions

Phase C-design output of the planner domain. It holds the definitions that
several `research/experiments/EXP-PLANNER-*/protocol.md` files share, so each
protocol can cite them instead of restating them. The authoritative method is
`research/decision-method.md`, which governs here as everywhere. This file
never restates a threshold: it cites `research/methods/thresholds.json` keys
and decision-method sections.

Status: **design, not a pre-registration record.** A `record.md` (§12.2) for
each experiment is committed only after gate G-A holds (§0.4). No decision-relevant
result exists at the commit that adds this file.

## 1. Experiment map

| ID | Title | Status | Decisions (primary first) |
|---|---|---|---|
| EXP-PLANNER-001 | Status-quo profile census: sizes, section bytes, plan usage, feature bits, identities | READY | DEC-CMP-036, DEC-CMP-034, DEC-CMP-035, DEC-CON-010, DEC-MOD-039 |
| EXP-PLANNER-002 | Exhaustive planner oracle traces and status-quo regret by workload class | NEEDS_TOOLING | DEC-CMP-032, DEC-CMP-034, DEC-CMP-009 |
| EXP-PLANNER-003 | Search strategies and deterministic work budgets | NEEDS_TOOLING | DEC-CMP-032, DEC-CMP-033 |
| EXP-PLANNER-004 | Candidate tables, gating probe, gain margins, dictionary acceptance and stage conflicts | NEEDS_TOOLING | DEC-CMP-034, DEC-CMP-012, DEC-CMP-030, DEC-CMP-020, DEC-CMP-039, DEC-CMP-009 |
| EXP-PLANNER-005 | Chunking-policy selection rule, parameter granularity and per-profile chunk lists | NEEDS_TOOLING | DEC-CMP-004, DEC-CMP-003, DEC-CMP-002, DEC-MOD-039 |
| EXP-PLANNER-006 | Profile parameter response surface and fitted thirteen-parameter profiles | NEEDS_TOOLING | DEC-CMP-035, DEC-CMP-045, DEC-CMP-016, DEC-CMP-006, DEC-CMP-013, DEC-CMP-019, DEC-CRY-028, DEC-ACC-003, DEC-ACC-029 |
| EXP-PLANNER-007 | Profile set, default profile and profile timing campaign | NEEDS_TOOLING | DEC-CMP-036, DEC-CMP-035, DEC-CMP-034, DEC-CMP-032, DEC-CMP-033 |
| EXP-PLANNER-008 | Deterministic decode-cost model for `min_decode_rate` | NEEDS_TOOLING | DEC-CMP-015 |
| EXP-PLANNER-009 | Encoder output identity across versions, CPU dispatch paths and emulated ARM64 | NEEDS_TOOLING | DEC-CMP-031, DEC-CMP-019, DEC-CMP-040, DEC-CMP-029 |
| EXP-PLANNER-010 | Planner provenance: replan, historical generations, encrypted planner IDs, explain and decision logs | NEEDS_TOOLING | DEC-CMP-040, DEC-CMP-025, DEC-CMP-022, DEC-CMP-029, DEC-ACC-012, DEC-INT-046 |
| EXP-PLANNER-011 | Cross-architecture and constrained-device replication | BLOCKED | DEC-CMP-015, DEC-CMP-036, DEC-CMP-031, DEC-CMP-016, DEC-CMP-019 |
| EXP-PLANNER-012 | Planner-domain held-out confirmation (Phase D placeholder) | BLOCKED | every planner decision that reaches a provisional selection |

Uncovered section-9/10 decisions and their evidence routes:
`research/experiments/_index/planner-uncovered.jsonl`.

Execution order (§13 "deterministic metrics precede timing"): 001 → 002 →
{003, 004, 005, 006 replay arms} → {005, 006 re-plan arms, 009, 010} → joint
validation look (§5 below) → 007 and 008 timing windows → 012 (Phase D).
011 runs whenever its platforms become available.

## 2. Build identity (binding for every planner experiment)

1. Toolchain `1.98.1`, invoked by explicit path (`/root/.cargo/bin/cargo +1.98.1`,
   `%USERPROFILE%\.cargo\bin\cargo.exe +1.98.1`), per `research/PROGRESS.md`.
2. The harness is rebuilt at the experiment's pre-registration commit into
   `/root/eb-research/target/planner-harness` (WSL) and
   `D:/eb-research/target/planner-harness-win` (Windows):
   `CARGO_TARGET_DIR=/root/eb-research/target/planner-harness bash research/harness/build.sh build --release`.
3. Production `ebound` is rebuilt from the production workspace at the same
   commit, without `research-internals`, into
   `/root/eb-research/target/planner-cli` (WSL) and `D:/eb-research/target/planner-cli-win`.
4. Before every campaign and after any `Cargo.lock` change:
   `python research/harness/lock_parity.py` and
   `bash research/harness/harness-cli-parity.sh` both print `RESULT PASS`
   (harness review use constraint 1).
5. **Design-smoke finding (non-decision-grade, 2026-09-17).** The binaries at
   the default `/root/eb-research/target/harness/release/` predate the harness
   review fix commit `5bd33f8`: that `ebr-planner` still searched each input as
   one whole-file Chunk and emitted no `chunking`/`archive_regret` rows
   (finding R1-07). No planner experiment may use that directory.
6. Status quo (R0 item 1) is production behaviour at the research baseline
   `9e44608`. EXP-PLANNER-001 carries a baseline-identity arm that proves the
   pre-registration build reproduces it byte for byte; if it does not, every
   planner experiment stops (HC-16, objective MVT-16(d)).

## 3. PSUB-k derived sub-items

Exhaustive and multi-arm experiments cannot afford whole large items. A
PSUB-k sub-item is derived from one tuning or validation item (never held-out)
by `research/planner/tools/make_psub.py` (to be written, TP-15):

1. If the item's logical bytes are at most k MiB, the sub-item is the whole item.
2. Otherwise regular files are ordered by `SHA-256(item_id ‖ 0x00 ‖ relpath_without_first_component)`,
   ties broken by the full relative path bytes. Dropping the first component
   keeps corresponding files of duplicate and versioned trees (F17, F18)
   adjacent in the order, so their cross-file structure survives sampling.
3. Files are taken in that order while the cumulative size stays at most k MiB.
   The first file that does not fit is included as one contiguous slice of
   the remaining length, at offset
   `(u64 from the first 8 bytes of SHA-256(item_id ‖ relpath)) mod (size − length + 1)`,
   so the offset is fixed without looking at content.
4. Directories, symlinks and metadata of included files are copied with
   `cp -a` semantics into `/root/eb-research/derived/planner/psub-<k>/<split>/<item_id>`.
5. Every sub-item gets a fingerprint (`research/corpus/tools/fingerprint.py`)
   and a row in `research/planner/derived/psub-<k>.json` (ebr manifest shape:
   `item_id` = `<item_id>--psub<k>`, `split`, `family`, `independence_group`,
   `scale` of the parent, `path`, `logical_tree_sha256`, `parent_logical_tree_sha256`).
6. Sub-items inherit the parent's independence group. Scale-tier strata use the
   **parent's** tier, reported with the realized sub-item bytes.

k values in use: PSUB-256 (EXP-PLANNER-002, 003, 004, 005, and 006 for fast
and balanced), PSUB-64 (007 dense/extreme encode timing on TS-1).

**G1-PSUB-64.** The PSUB-64 sub-item of the first tuning (or validation) item of
each independence group in `SHA-256("G1" ‖ item_id)` order: one sub-item per
group, every group represented (`make_psub.py --one-per-group --group-key G1`).
Used by EXP-PLANNER-006 (dense and extreme), 009 and 010. Manifests:
`research/planner/derived/g1-psub-64.json`, and `g1-psub-64-small.json` (parent
scale tier small only) for emulated and encrypted arms.

## 4. TS-1 timing subset

`research/planner/tools/make_ts1.py` (TP-16) selects, before any timing result:

- per family F01-F20, up to 2 tuning items from distinct independence groups,
  taken in `SHA-256("TS-1" ‖ item_id)` order among small- and medium-tier items;
  families lacking such items contribute what they have;
- per family with a large-tier tuning item, the first large item in the same
  order, used only for `fast` and `balanced` encode timing and for decode timing
  of every profile;
- dense and extreme **encode** timing uses the PSUB-64 sub-items of the TS-1
  items (large-tier encode guards for dense and extreme are therefore reported
  as a limitation, not claimed).

The same rule, with `"TS-1-V"` as the key prefix, selects the validation timing
subset for the joint look. Outputs: `research/planner/derived/ts1.json`,
`ts1-psub-64.json`, `ts1-timing-union.json` (both, for pre-packing),
`ts1-validation.json`, and `ts1-validation-windows.json` (the same items staged to
local NTFS by TP-19).

## 5. Joint validation look plan (§5.2)

Every planner decision shares the ledger cluster `compression`, a candidate, or
the primary metric `artifact_bytes` with at least one other planner decision, so
they are **related** under §5.2, and a candidate change after one decision's look
spends a look of the others. Therefore:

1. All tuning work of EXP-PLANNER-002 to 006, 009 and 010 finishes, and each
   decision's W and S are frozen and written into its `record.md`.
2. The §4.12 MDE gate is computed for every confirmatory comparison from
   tuning group-level variance and the validation group structure (via the
   §14 item 2 decision tooling). A comparison failing it is redesigned or its
   decision stays `INSUFFICIENT_EVIDENCE`.
3. One look, **planner look 1**, is appended to
   `research/decisions/validation-looks.jsonl` covering all planner decisions
   at once, then the validation runs of 001-010 execute.
4. Look 2 is reserved. It may use only new validation items from new
   independence groups, committed and fingerprinted before use (§5.2).
5. Cross-domain: decisions this domain shares with crossfile, codecs,
   reconstruction, scale, remote, crypto, integrity and container experiments
   are related through shared candidates. The look is registered only after
   those domains' records for the same decisions have frozen their W and S,
   or the look counts against them too.

## 6. Decision frames

- **Profile-scope decisions** (DEC-CMP-002, -003, -004, -006, -009 arm, -012,
  -013, -016, -020 arm, -030, -032, -033, -034, -035, -039, -045 and the
  DEC-ACC-029, DEC-ACC-003 and DEC-CRY-028 ratio arms) use the constrained form
  of §2.0 and Appendix B.1: `artifact_bytes` is the only superiority metric;
  `encode_wall_s` (at the profile's `metric_thresholds.encode_wall_s.relative_by_profile`
  band), `decode_wall_s`, `alloc_peak_bytes` and `access_granularity_bytes`
  are non-inferiority guards; the declared profile bounds screen at R2 once
  DEC-CMP-035 fixes them, and until then are reported, not screened (R2 last
  bullet). No weights are used. Scopes are the four profiles (R9 partition 1),
  and a universal claim is the same candidate selected in every profile scope.
- **The default-profile question** (DEC-CMP-036 part (c)) is universal scope
  and uses the band-unit utility of §6.2 with equal base OD weights (§6.3) over
  the declared OD set.
- **Decision types and `od_ids`/`hc_ids` in these protocols are proposals.**
  §1.2 requires an independent session to assign `decision_type`, and §14
  item 12 generates `od_ids`/`hc_ids`. If the assigned sets differ, the
  record is amended before execution.
- **Regret** is always a paired `artifact_bytes` comparison against the
  exhaustive oracle candidate, in continuous band units of T-01 (§6.2), using
  archive-level bounds (`archive_regret`) or per-chunk rows with
  `in_scope == true` only (harness use constraint 6).

## 7. Modelled versus materialized bytes

Replays over EXP-PLANNER-002 traces yield **modelled** `artifact_bytes`:
the status-quo archive's exact bytes minus its traced Chunk-stage, cohort-stage
and region-stage costs, plus the alternative's costs, with each distinct
TransformPlan record charged once. Modelled values become decision-grade only
through reconciliation by `ebr-planner materialize` (TP-02):

- a seeded 10% sample of all (item, profile, variant) tuning cells
  (seed `260917992`), and **every** W and S cell at validation, is
  materialized;
- `reconciliation_delta_bytes` must be exactly 0 in every materialized cell
  (binary, tolerance 0);
- one non-zero delta makes every modelled value of that variant family
  informational until the replay model is fixed and the whole reconciliation
  sample is re-run. Materialized values then replace modelled ones.

Variants that cannot be materialized with production codecs (for example a
Zstandard window above the frozen 1 MiB, or external codecs) are labelled
`modelled-only`; they may inform R6 minimum-gain eliminations of the
*modelled candidate* (an upper bound failing the minimum gain eliminates it)
but can never select it.

## 8. Gates and shared prerequisites

| Prerequisite | Needed by | Source |
|---|---|---|
| G-A: constraint crosswalk, ledger schema v2 (`hc_ids`, `od_ids`, `decision_type`), validation-look registry | committing any `record.md` | §0.4, §14 items 10, 12, 14 |
| G-B: `thresholds.json` test update; decision-analysis tooling; frozen held-out lock; `workload-mix.json` | first decision-relevant result | §14 items 1, 2, 5, 13 |
| Runner additions (cooldown, canary, covariates, guard accounting, spec lint, write accounting) | any timing in 003, 004, 006, 007, 008, 010 | §14 item 3 |
| Calibration `EXP-CAL-AA-<env>`, `EXP-CAL-AAP-<env>`, `EXP-CAL-KE-<env>`, `EXP-CAL-BG-<env>` for `wsl-ubuntu` and `windows-host`, families `time_wall`, `time_cpu`, `memory_peak` | timing and memory verdicts | §4.7 |
| Counting allocator with site attribution | `alloc_peak_bytes` | §14 item 17 |
| Cost-accounting scripts C1-C3 and the C4-C7 assessor template | computed tiers before the look | §14 item 7 |
| Rule simulation | before the first validation look | §14 item 9 |
| Harness use constraints 1-7 | every run | `research/harness/harness-review-round1.md` |
| `disk_guard.py` pass | every launch | PROGRESS standing decision 2026-09-16 |

## 9. Tooling contracts (to be written; names are binding)

| ID | Artifact | Contract |
|---|---|---|
| TP-00 | `research/experiments/EXP-PLANNER-001/census_sample.py` | **Written at design.** Wraps `ebr-pack`, `ebound inspect --json`, `ebr-unpack` and `research/corpus/tools/fingerprint.py`; prints one JSON line `ebr.planner.census.v1` |
| TP-01 | `ebr-planner trace-tree` (new `research/harness/crates/ebr-planner/src/tree.rs`) | `--item-path <dir> --experiment-id --env-name --profiles fast,balanced,dense,extreme --caps full --external-candidates brotli-q11-w24,bzip2-9,ppmd-o6-m16 --sampled-costs 16x4096 --cache-dir /root/eb-research/cache/planner-oracle --out-dir <dir>`. `--sampled-costs` adds, per Chunk and candidate, the complete cost on the deterministic 16-window sample used by successive halving (EXP-PLANNER-003). Captures the tree with production capture, runs `trace_archive(archive, profile, V6)` per profile, `search_chunk(SearchCaps::full())` per unique Chunk digest per realized chunking (cache keyed by SHA-256 and caps hash), `search_cohort` with every `SUPPORTED_LEVELS` × `SUPPORTED_LOOKBACKS`, `trace_jpeg_object` per ContentObject. Requires `PlannedAssignment::differences` empty. Writes gzip JSONL shards `chunks`, `cohorts`, `objects`, `refs`; stdout JSON `ebr.planner.trace-summary.v1` with `drift_differences`, `archive_regret.<profile>.{lower,upper,selected_cost}`, `work.{candidate_evaluations,bytes_examined}`, `shards_sha256` |
| TP-02 | `ebr-planner materialize` | `--item-path --assignment <jsonl> --profile --layout indexed --experiment-id --env-name --archive-out [--modelled-only-ok true]` (the flag admits payloads from `ebr-codec` external or non-frozen codec parameters and marks the row `modelled_only`); encodes the explicit assignment via research plan constructors and `ecf::encode`; stdout `ebr.planner.materialize.v1`: `artifact_bytes`, `byte_accounting`, `verified`, `roundtrip_content_match`, `modelled_artifact_bytes`, `reconciliation_delta_bytes`, `pci` |
| TP-03 | `research/planner/tools/replay.py` | Deterministic, stdlib-only replays and fits over TP-01 shards: `strategies`, `budgets`, `tables`, `gating`, `margins`, `dictionary-rule`, `conflicts`, `decision-log-size`, `params`, `fit-tree`, `fit-tables`, `fit-constants`, `fit-tnb`; writes CSV with provenance header under `research/normalized/<EXP>/replay/` and assignment files for TP-02 |
| TP-04 | `ebr-planner chunking-matrix` | `--item-path --chunking <FAST_V2\|BALANCED_V2\|DENSE_V2\|EXTREME_V2\|custom:min/target/max>... --codec-stage <profile> --granularity archive\|per-object\|per-category\|per-size-class --categoriser signature-v0`; stdout `ebr.planner.chunking-matrix.v1`: `artifact_bytes`, `proxy_cost_status_quo`, `proxy_cost_corrected`, `unique_chunk_count`, `logical_chunk_references`, `max_object_chunk_count`, `cross_category_duplicate_bytes`, `pcr`, `research_only_multi_chunker` |
| TP-05 | `ebr-planner categorise` | Integer byte-signature categoriser `signature-v0` (ELF/PE/Mach-O with architecture, gzip/zlib/ZIP/PNG/JPEG, safetensors/GGUF, integer-probe text vs structured-numeric vs incompressible); per-object labels; no filename or extension input |
| TP-06 | `ebr-planner param-plan` | `--item-path --base-profile --layout --experiment-id --env-name --set <param>=<value>` (repeatable; the thirteen SPEC §6.0a parameters plus `planning_window_bytes`, `cross_file_scope=none\|owner\|archive`, `ordering=digest\|path\|type-path`, `supplied_dictionary=<file>`, `adaptive=declared\|host`, `forbid_store=true`, `eligibility=mirror-validation`, `zstd_window_log=<n>[+long]`, `stage_order=cohorts-first\|region-first\|joint\|cohorts-first-split-audit`, `record_encoder_versions=true`, `profile_definition=<form>`) `[--fitted-profiles <json>] [--no-verify true] [--identity-summary true] --archive-out`; stdout `ebr.planner.param-plan.v1`: `artifact_bytes`, `access_granularity_bytes`, `declared_decode_memory_bytes`, `declared_window_bytes`, `features.incompat`, `work.*`, `unsupported_parameters` (must be empty in decision cells), `verified`, `roundtrip_content_match` |
| TP-07 | `ebr-planner time-select` | In-process monotonic timing of selection plus encode for one strategy, table, gate or margin variant; stdout `encode_wall_s`, `encode_cpu_s` (thread CPU), `work.*` |
| TP-08 | counting allocator in `ebr-common` plus `ebr-unpack --alloc-peak true` | decode `alloc_peak_bytes` (§14 item 17; shared with other domains) |
| TP-09 | `ebr-codec decode-matrix` | In-process decode timing per (Chunk, frozen codec configuration, transform) in batches of at least 1,000 operations or 50 ms; stdout per-config `decode_wall_s`, `decode_throughput_bps`, `decoded_bytes` |
| TP-10 | `research/planner/tools/encoder-drift/` | `make_projects.py` generates one Cargo project per encoder version set; driver `encoder-drift` hashes each (Chunk, configuration) output; `run_matrix.sh` runs native x86-64, `qemu-x86_64 -cpu {Nehalem,Haswell,max}` and `qemu-aarch64` user mode |
| TP-11 | environment installs (downloads approved) | `apt-get install qemu-user-static gcc-aarch64-linux-gnu`; `rustup target add aarch64-unknown-linux-gnu --toolchain 1.98.1`; Windows MSVC build of TP-10 |
| TP-12 | `ebr-planner pack-generation` and `ebr-planner replan` | pack with `PlannerVersion::{V2..V6}`; replan with policy `current\|pinned\|source\|current-report` (`--ebound` for the production repack path, `--encrypt true` with generated test credentials, `--report-planner-ids-only true`); stdout `pcr_equal`, `pci_equal`, `artifact_bytes`, `source_planner_id`, `result_planner_id` |
| TP-13 | `research/planner/tools/explain_fidelity.py` | Runs `ebound explain` on census archives and scores agreement with TP-01 traces |
| TP-14 | `research/planner/tools/historical_api_census.py`, `research/planner/tools/planner_surface_loc.py` | Static census of historical creation API call sites and retained planner writer LoC (checked-in counter, §7.1 C2) |
| TP-15 | `research/planner/tools/make_psub.py` | §3 above |
| TP-16 | `research/planner/tools/make_ts1.py` | §4 above |
| TP-17 | `ebr run --shard i/K` (runner addition) | Deterministic specs only (refused with `timing: true`); lets K runs of one spec execute concurrently; memory-aware launcher `research/planner/tools/shard_launch.py` caps concurrent item bytes. Throughput only, never a prerequisite for correctness |
| TP-18 | `research/planner/tools/cgroup_decode.sh` | Runs a decode under a cgroup v2 `memory.max` in WSL (root) and reports the outcome class and `alloc_peak_bytes` |
| TP-19 | `research/planner/tools/stage_windows_items.py` | Copies selected tuning/validation items (never held-out) to `D:/eb-research/...` local NTFS outside the repository and writes a Windows manifest with fingerprints |
| TP-20 | `research/planner/tools/make_timing_spec.py` | Generates look-1 timing specs from a template, a frozen W ∪ S list in a `record.md`, and a validation subset manifest; the generated spec is committed before the look |
| TP-21 | `research/planner/tools/prepack.py` | Applicability-filtered, untimed pre-pack or decode-input wrapper for EXP-PLANNER-007 (prints `artifact_bytes`, `artifact_sha256`, `applicable`) |
| TP-22 | `research/planner/tools/guidance_census.py` | Mechanical census of `ebound pack --help`: each profile's stated use case and bounds (binary per profile) |
| TP-23 | `research/planner/tools/make_decode_bundles.py`, `research/planner/tools/decode_cost_model.py` | Chunk bundles per (family, category) from TP-01 traces; fits the reference decode table and evaluates candidate models (Kendall τ-b, floor honouring) |

## 10. Held-out identity exposure of the designing session (L7 disclosure)

The session that wrote these designs read `research/PROGRESS.md` as
instructed. Its change log names the upstream sources of held-out items in
families F01, F04, F12, F13, F15, F16, F17, F19 and F20. The session opened no
held-out content, listing, statistic or path; it filtered
`research/corpus/manifest.json` to the tuning and validation splits
programmatically. Under §5.4 item 3 and §5.7 L7 this is an identity
exposure: before any of these designs is used as a `record.md`, a session
without that exposure reviews the candidates and analyses for decisions whose
applicable families include the exposed ones (all planner decisions), or
the exposure is recorded against them in the L7 audit record (§5.8 h).
The identifiers are not repeated here.

## 11. Seeds

Experiment EXP-PLANNER-0NN uses `order = 260917000 + 10·NN + 1`,
`bootstrap = … + 2`, `command = … + 3` (for example EXP-PLANNER-001:
260917011, 260917012, 260917013; EXP-PLANNER-012: 260917121, 260917122,
260917123). Cross-experiment seeds: reconciliation sample `260917992`,
learned-selector cross-validation folds `260917993`, selection-stability
bootstrap `260917994`.
