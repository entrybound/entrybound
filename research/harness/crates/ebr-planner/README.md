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
  checks all 6 generations x 4 profiles = 24 combinations.
- `candidate_regret(chunk_trace)` -- the cost gap, in the enumeration's own
  `complete_cost` units, between each candidate and the one actually
  selected. Zero at the selected candidate; positive everywhere the real
  planner passed over a more expensive alternative. This is "regret" in the
  research sense used by `research/PROGRESS.md`'s decision-method work, not a
  new metric this crate invents.

## Non-goals (for now)

- `archive_for` only builds single-chunk-per-input, fixed-chunk-size
  archives (one `Entry::File` per named input, `chunk_size` equal to the
  whole input). It does not yet drive multi-chunk objects, cohorts across
  many similar objects at corpus scale, or JPEG region candidates
  (`entrybound::research::planner::trace_jpeg_object`) -- all reachable
  through the same `entrybound::research::planner` surface, just not wired
  up here yet.
- No exhaustive *search* over planner parameters (chunk sizes, dictionary
  construction choices, ...) yet -- today this crate only replays the
  enumeration production already performs, at production's frozen presets.
  "Exhaustive search" in the crate description names the destination, not
  something implemented yet.

## The `ebr-planner` binary

```text
ebr-planner --item-path <file> --experiment-id <id> --env-name <name> \
            [--profile fast|balanced|dense|extreme] [--run-id <id>] [--out <path>]
```

Reads `--item-path` fully into memory as a single-object, single-chunk
archive input and appends one `ebr.harness.raw.v1` JSONL row per planner
generation (`PlannerVersion::ALL`, six rows total), each carrying a
`DriftReport`. See `research/harness/README.md` for the shared flag
contract.

## Testing

```sh
cargo +1.98.1 test -p ebr-planner
```

Tests build their own small in-memory archives; nothing in this crate reads
`research/corpus` or `/root/eb-research` unless a caller points `--item-path`
there. The drift-matching test takes a few seconds (it plans and enumerates
24 profile/generation combinations), which is expected -- see the module
docs' "Limitations" note inherited from `research-internals.md`: "the
enumeration re-encodes every candidate, so it costs about as much as
planning itself."
