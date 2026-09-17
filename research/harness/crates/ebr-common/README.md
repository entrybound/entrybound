# ebr-common

Shared library for every other crate in the Entrybound research harness
(`research/harness`, Phase B2). Nothing in this crate is a stable API and
nothing in it changes production behavior; it only reads already-committed
research data and measures the calling process.

## What it provides

| Module | Purpose |
|---|---|
| `manifest` | Reads `research/corpus/manifest.json` (`Manifest::load_default`), when present. Tolerant of unknown fields (`extra`). |
| `heldout` | `HeldoutGuard`: refuses any path under the held-out root (`$EB_DATA_ROOT/heldout`, default `/root/eb-research/heldout`) unless `EB_HELDOUT_UNLOCK` matches `research/decisions/design-freeze.json` (keys `design_freeze_sha`, `commit_sha`, or `sha`). That file does not exist yet, so unlocking is currently always impossible -- fail closed by default. See the module docs for exactly how this differs from `research/corpus/tools/corpuslib.py`'s `assert_not_heldout`/`check_heldout_unlock`, which this is *equivalent in effect to*, not a port of. |
| `walk` | `walk_tree` lists a corpus item's files/dirs/symlinks (never reading content); `read_tree_in_memory` additionally reads every regular file's bytes, subject to an explicit byte budget. |
| `results` | `ResultWriter`: appends `ebr.harness.raw.v1` JSONL rows (`schema`, `experiment_id`, `run_id`, `env_id`, `seq`, `timestamp_utc`, `item_id`, `payload`). See the module docs for how this schema differs from the Python baselines' `ebr.raw.v1`. |
| `runenv` | `EnvIndex::load` reads `research/environment/index.json` to resolve an environment name to its `env_id`; `RunContext` bundles `experiment_id`/`run_id`/`env_id`. |
| `measure` | `peak_memory()`: this process's own peak resident memory. True OS high-water mark on Linux (`getrusage`); current-only on Windows today (see module docs for why -- no safe crate reaches `PeakWorkingSetSize` without an `unsafe` call at the site, and this workspace forbids `unsafe_code`). |
| `timing` | `Stopwatch` (wraps `Instant`, the only clock used for durations), `now_utc_rfc3339()`, `generate_run_id()`. |
| `hash` | `sha256_hex`/`sha256_file`, using the same exact `sha2 = "=0.11.0"` pin as `crates/entrybound`. |

Top-level `discover_repo_root(start)` walks up from a `CARGO_MANIFEST_DIR` to
find the entrybound repo root, for binaries that need to locate
`research/corpus`, `research/environment`, or `research/decisions` at
runtime.

## Non-goals

- This crate does not itself implement the two-key, git-ancestry-checked
  `check_heldout_unlock` ceremony `corpuslib.py` uses for actual held-out
  *content* access ahead of Phase D. A caller that needs those stronger
  guarantees should still shell out to `corpuslib.py`.
- This crate does not read or write `research/corpus/fingerprints/*.json`,
  `research/corpus/stats/*.json`, or any provisioning/materialization logic.
  Those stay in `research/corpus/tools/*.py`; `manifest.rs` only reads the
  already-assembled `manifest.json`.
- `results.rs`'s JSONL schema (`ebr.harness.raw.v1`) is a first cut for
  in-process Rust measurements and is not byte-identical with the Python
  baselines' `ebr.raw.v1` (`research/raw/*/*.jsonl`), which additionally
  hash-chains rows. See the module docs.

## Testing

```sh
cargo +1.98.1 test -p ebr-common
```

All tests are self-contained (they use `std::env::temp_dir()` scratch
directories they create and remove themselves) and never touch
`/root/eb-research` or any corpus content.
