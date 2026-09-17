# ebr-planner

Planner candidate traces, exhaustive search, and regret research, via
`entrybound`'s default-off `research-internals` feature
(`entrybound::research::planner`).

## What it provides

- `archive_for(inputs)` -- builds a minimal *unplanned* `Archive` from
  in-memory named inputs. This is the same bootstrap-archive shape
  `crates/entrybound/tests/research_internals.rs` (the Internals agent's own
  proof) uses to drive the planner research API, built from public
  `entrybound::eam`/`entrybound::identity` items -- not a copy of that test
  file, just the same minimal construction.
- `check_enumeration_matches_real_planner(unplanned, profile, version)` --
  runs both the real planner (`PlannerVersion::plan`, on a clone) and the
  research candidate enumeration (`entrybound::research::planner::trace_archive`)
  over the *same* unplanned archive, and reports
  `PlannedAssignment::differences`. `research/harness/research-internals.md`
  states the exact contract this checks: "a research harness must check that
  [this] is empty against an Archive planned by the real planner before it
  trusts an enumeration." An empty `differences` list is the expected
  result for every planner generation and profile -- this crate's own test
  (`enumeration_matches_the_real_planner_for_every_generation_and_profile`)
  checks all 6 generations x 4 profiles = 24 combinations, and
  `trace_reconstruction_selects_exactly_what_plan_archive_v6_selected_on_generated_inputs`
  repeats the `PlannerVersion::V6` check over several generated inputs
  (repetitive text, pseudo-random noise, a sparse buffer, a little-endian
  numeric sequence).
- `candidate_regret(chunk_trace)` -- the cost gap, in the enumeration's own
  `complete_cost` units, between each candidate and the one actually
  selected. Zero at the selected candidate; positive everywhere the real
  planner passed over a more expensive alternative. This is "regret" in the
  research sense used by `research/PROGRESS.md`'s decision-method work, not a
  new metric this crate invents.
- `exhaustive` module -- a widened, capped candidate search (every codec x
  transform x level, and every dictionary/lookback level) that finds the true
  optimum over a strict superset of what production ever tries, so its
  regret against production's real choice is always `>= 0`. See "Exhaustive
  search and regret" below.
- `strategies` module -- three deterministic, research-only alternative
  selectors compared against that optimum: a pruned greedy walk, a partition
  dynamic program over lookback groups, and a frozen-threshold decision
  tree. See "Alternative selection strategies" below.

## Exhaustive search and regret (`exhaustive` module)

`entrybound::research::planner::trace_chunk`/`trace_cohort` enumerate exactly
the candidates production's own frozen, profile-restricted lists build --
they never invent a candidate production would not have tried. `exhaustive`
is the complement: a search space that is a strict *superset* of every
candidate any `CompressionProfile`/`PlannerVersion` combination ever tries,
costed with the exact same production accounting primitives
(`v5_independent_cost`, `encoded_transform_plan_v2_len`,
`encoded_transform_plan_len`, `encoded_dictionary_len`,
`encoded_chunk_group_len`, the `*_SECTION_OVERHEAD_BYTES` constants). Because
production's real choice is always one exact member of this wider universe
(chunk-level: every `SUPPORTED_LEVELS` Zstandard level and every
`SUPPORTED_LZMA2_CONFIGURATIONS` LZMA2 configuration, crossed with every
`TransformKind`; cohort-level: every `SUPPORTED_LEVELS` dictionary/lookback
level crossed with every `SUPPORTED_LOOKBACKS` lookback distance), this
module's optimum can never cost more than production's choice -- except a v5
DEFLATE-reconstruction selection, which is a different shape and out of
scope (see the module's doc comment's "Scope" section for exactly why, and
which of this crate's tests guard against it silently passing anyway).

- `SearchCaps`/`CohortSearchCaps` -- explicit, recorded bounds (`::full()`
  gives the whole universe described above plus a hard `max_candidates` cap);
  every search takes one, and every JSONL row this crate writes carries it,
  so nothing is silently bounded.
- `search_chunk(plaintext, &caps)` -> `ChunkExhaustiveSearch` -- every
  (codec, transform) shape `caps` describes, each candidate's
  `payload_bytes`/`plan_record_bytes`/`complete_cost`, and the cheapest one
  found.
- `chunk_regret(selected_plan, plaintext, &search)` -- production's real
  choice, recosted with the same formula the search uses (so the comparison
  holds regardless of which internal formula production used to pick it),
  minus the search's optimum. `exhaustive_search_never_exceeds_the_production_choice`
  checks this is `>= 0` across 4 generated inputs x 4 profiles x 6 planner
  generations = 96 combinations.
- `search_cohort(archive, cohort, profile, planner_id, plans, &caps)` ->
  `CohortExhaustiveSearch` -- the independent baseline plus every dictionary
  and lookback candidate `caps` describes, over one `SimilarityCohort` and
  the archive/plan-table state `entrybound::research::planner::trace_cohort`
  itself expects (for example `ChunkStageTrace::{archive, plans}`).
  `cohort_regret(&real_trace, &search)` compares it against a real
  `trace_cohort` run the same way `chunk_regret` does for chunks; this
  crate's own test also asserts the two independent-cost formulas agree
  exactly, not just that one bounds the other.

## Alternative selection strategies (`strategies` module)

Three deterministic strategies, each compared against `exhaustive`'s optimum
but never fed back into production:

- `select_greedy(plaintext, &caps, patience)` -- walks
  `exhaustive::candidate_shapes`'s fixed, cheap-first order (STORE, LZ4,
  ascending Zstandard levels, LZMA2 configurations, then each transform
  crossed with those same codecs) and stops as soon as `patience` candidates
  in a row fail to improve on the best seen. `GreedyResult::evaluated` records
  how many shapes it actually costed, so a JSONL row shows the pruning
  directly (a smoke run over repetitive text found the true optimum -- the
  same 265-byte `Zstd { level: 9 }` candidate `search_chunk` found -- after
  costing only 9 of the 52 shapes).
- `select_group_lookback(archive, chunk_ids, &caps, max_group_len)` -- a
  genuine dynamic program, distinct from production's own mechanism
  (production forms at most one lookback group per whole similarity cohort).
  This DP instead chooses, for an ordered sequence of chunks, where to cut it
  into contiguous segments -- each either independent or one lookback group
  at its own `(level, lookback)` -- to minimize total cost, by the standard
  partition recurrence over an explicit, capped `max_group_len`. It reuses
  production's own `cohort_prefix` so a segment's cost is exactly what
  production would charge for that grouping; only the choice of *where to
  group* is this module's own alternative. `naive_independent_cost` is the
  "every chunk on its own" baseline the DP can never exceed (that partition
  is one the DP itself considers).
- `select_by_tree(analysis, logical_len)` -- a deterministic, frozen-threshold
  decision tree over only integer (or integer-derived boolean) features from
  `entrybound::planner::analyze`'s `ContentAnalysis` (`distinct_symbols`,
  `maximum_symbol_frequency_ppm`, `adjacent_repeat_ppm`, `likely_compressible`,
  `empty`) plus `logical_len`. No search at all -- the same inputs always
  yield the same choice. The thresholds were set by inspecting `search_chunk`
  traces over this crate's own generated tuning fixtures, not retrained
  against the real corpus; promoting them (or this strategy) beyond research
  use needs a Decision Ledger entry like everything else this crate builds.

## Non-goals (for now)

- `archive_for` only builds single-chunk-per-input, fixed-chunk-size
  archives (one `Entry::File` per named input, `chunk_size` equal to the
  whole input). It does not yet drive multi-chunk objects or JPEG region
  candidates (`entrybound::research::planner::trace_jpeg_object`) -- both
  reachable through the same `entrybound::research::planner` surface, just
  not wired up here yet.
- `search_cohort`/`select_group_lookback` are library-level: they take an
  `Archive` and a chunk/cohort list a caller already built (this crate's own
  tests do, via `archive_for` plus a manually specified `SimilarityCohort` or
  chunk-id list), rather than the real `entrybound::similarity::cluster`
  threshold. The `ebr-planner` binary below only drives the chunk-level
  pieces (one item per invocation maps naturally onto them); a corpus-scale
  cohort runner, wiring `cluster()`'s real thresholds and multiple
  `--item-path`s per invocation, is future work.
- The candidate-regret CSV covers chunk-level candidates only (see below);
  cohort-level dictionary/lookback results are JSONL/library-only for now.

## The `ebr-planner` binary

```text
ebr-planner --item-path <file> --experiment-id <id> --env-name <name> \
            [--profile fast|balanced|dense|extreme] [--run-id <id>] [--out <path>] \
            [--patience <n>] [--csv-out <path>]
```

Reads `--item-path` fully into memory as a single-object, single-chunk
archive input and appends `ebr.harness.raw.v1` JSONL rows (to `--out`, or
stdout if omitted). Every row's `payload.kind` says which of these it is:

| `kind` | Count per invocation | What it carries |
|---|---|---|
| (none -- a `DriftReport`) | 6 (one per `PlannerVersion`) | unchanged original behavior: `PlannedAssignment::differences` against the real planner |
| `exhaustive_search` | 1 | the full `ChunkExhaustiveSearch` (every candidate `SearchCaps::full()` describes, `caps`, `optimal`, `truncated`) |
| `exhaustive_regret` | 6 (one per `PlannerVersion`) | that generation's real selected candidate's stage and cost, the exhaustive optimum, and `regret` |
| `greedy` | 1 | `select_greedy`'s chosen candidate, how many shapes it evaluated, and `patience` |
| `tree` | 1 | `select_by_tree`'s choice, its cost, and the exhaustive optimum |

That is 15 JSONL rows for one invocation -- **this binary was already
multi-row-per-invocation before this crate added the last 8**, so
`research/harness/README.md`'s "`--out` left unset -> exactly one JSON line"
note does not apply to `ebr-planner`; drive it with `--out` to a per-run file
(or per-`{rep}`/`{seed}` file) rather than relying on `stdout_json`
extraction.

`--patience` (default `3`) is `select_greedy`'s pruning patience.
`--csv-out <path>`, if given, appends one candidate-regret CSV row per
`exhaustive_search` candidate (`item_id, candidate_index, codec, transform,
payload_bytes, plan_record_bytes, complete_cost, is_optimal,
regret_vs_optimal`), writing the header first if the file did not already
exist -- so pointing every item in an experiment at the same `--csv-out` path
builds one combined candidate-regret table.

See `research/harness/README.md` for the shared flag contract (placeholders,
`--profile`/`--env-name` conventions, the `ebr.harness.raw.v1` schema).

### Example `ebr` runner spec command templates

Emit every row (drift, exhaustive search, regret, greedy, tree) to one JSONL
file per item, plus a combined candidate-regret CSV, over the `extreme`
profile:

```yaml
command: >-
  /root/eb-research/target/harness/release/ebr-planner
  --item-path {item_path} --experiment-id EXP-PLANNER-REGRET-001
  --env-name wsl-ubuntu --profile {candidate}
  --out /root/eb-research/raw/EXP-PLANNER-REGRET-001/{item_id}.jsonl
  --csv-out /root/eb-research/raw/EXP-PLANNER-REGRET-001/candidate-regret.csv
```

The same shape, sweeping the profile itself as the candidate dimension and
tuning the greedy strategy's pruning patience:

```yaml
candidates:
  - {name: fast, params: {profile: fast, patience: 1}}
  - {name: balanced, params: {profile: balanced, patience: 3}}
  - {name: dense, params: {profile: dense, patience: 3}}
  - {name: extreme, params: {profile: extreme, patience: 5}}
command: >-
  /root/eb-research/target/harness/release/ebr-planner
  --item-path {item_path} --experiment-id EXP-PLANNER-GREEDY-PATIENCE-001
  --env-name wsl-ubuntu --profile {profile} --patience {patience}
  --out /root/eb-research/raw/EXP-PLANNER-GREEDY-PATIENCE-001/{item_id}-{candidate}.jsonl
```

## Testing

```sh
cargo +1.98.1 test -p ebr-planner
```

Tests build their own small in-memory archives; nothing in this crate reads
`research/corpus` or `/root/eb-research` unless a caller points `--item-path`
there. The full suite (15 tests) runs in well under 10 seconds. The
drift-matching and exhaustive-regret tests re-plan/re-search several
profile/generation/input combinations each, which is expected -- see the
module docs' "Limitations" note inherited from `research-internals.md`: "the
enumeration re-encodes every candidate, so it costs about as much as
planning itself," and `exhaustive`'s search costs more still (a strict
superset of candidates). `search_chunk` does not depend on `profile`/
`version`, so tests that check regret across every generation compute it
once per input and reuse it, rather than re-running the search per
generation.
