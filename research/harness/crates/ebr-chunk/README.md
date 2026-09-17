# ebr-chunk

Chunking/dedup research: production `gear-norm-v1` (`entrybound::chunker`)
alongside eight research-only alternative content-defined chunking (CDC)
implementations, all runnable through the same `--algorithm` flag and
measured by the same four subcommands. Nothing here is wired into any
production code path.

## Algorithms

| `--algorithm` | Construction | Citation |
|---|---|---|
| `gear-norm-v1` | production (unmodified passthrough to `entrybound::chunker::chunk_ranges`) | -- |
| `fixed-size` | byte-count baseline, no content dependence | -- |
| `fastcdc-2016` | Gear hash, sub-minimum cut-point skipping, two-level normalized chunking | Xia et al., *FastCDC*, USENIX ATC 2016 |
| `fastcdc-2020` | same rule as `fastcdc-2016`, restructured to process two bytes per iteration for speed (same cut points) | Xia et al., *FastCDC 2.0*, IEEE TPDS 2020 |
| `rabin` | Rabin-style polynomial rolling hash over a sliding window (GF(2) reduction) | Rabin (1981); Broder (1993); CDC use per Muthitacharoen et al. (LBFS), SOSP 2001 |
| `buzhash` | cyclic-polynomial (rotate-and-XOR) rolling hash over a sliding window | Broder (1993) |
| `ae` | Asymmetric Extremum: cut after `window` bytes pass without a new running-maximum, no hashing | Zhang et al., *AE*, IEEE INFOCOM 2015 |
| `ram` | bounded-window backward-looking local-maximum rule (monotonic-deque sliding-window max), no hashing | Zhang et al., *RAM*, 2021 (see `src/algorithms/ram.rs` for exactly what is and is not reproduced) |
| `tttd` | Two Thresholds, Two Divisors: a Rabin-fingerprint rolling hash with a backup checkpoint so pathological runs don't all pile up at `max_size` | Eshghi & Tang, HP Labs HPL-2005-30 (2005) |

Every algorithm's module doc comment (`src/algorithms/<name>.rs`) cites its
paper in full and documents its exact construction, including where this
crate's implementation is a direct, verified port (`fastcdc2016`/
`fastcdc2020`, cross-checked in tests against the `fastcdc` crate pinned
exactly as a dev-dependency) versus this crate's own best-effort
reproduction of a published rule (`ae`/`ram`/`tttd`), and any place a
constant is calibrated empirically rather than taken from the paper (`ae`/
`ram`'s `window` default -- see `src/algorithms/ae.rs`).

Every algorithm produces a stable, self-describing algorithm id embedding
its parameters, mirroring production's `chunker_id` convention, e.g.
`fastcdc-2016/min-131072/avg-524288/max-2097152/nc-2` or
`rabin-cdc-v1/min-131072/target-524288/max-2097152/window-48`.

## Shared parameters

Every algorithm is sized from the *same* production preset
(`--profile fast|balanced|dense|extreme`, default `balanced` -- production's
frozen `FAST_V2`/`BALANCED_V2`/`DENSE_V2`/`EXTREME_V2` min/target/max), so
runs across algorithms at the same profile are directly comparable. Two
optional flags override an algorithm's own extra parameter:

- `--window <bytes>` -- `rabin` (default 48), `buzhash` (default 64), `ae`
  and `ram` (default equal to the profile's target size; see
  `src/algorithms/ae.rs` for how that default was calibrated).
- `--normalization <0-3>` -- `fastcdc-2016`/`fastcdc-2020` (default 2).

`fixed-size` ignores both and uses the profile's target size as its exact
chunk size. `tttd` always uses `rabin`'s default 48-byte window internally
(not currently overridable from the CLI; see `Params::from_profile` if that
needs to change).

## Subcommands

```text
ebr-chunk <boundaries|dedup|shift-stability|random-access> \
          --item-path <file> --experiment-id <id> --env-name <name> \
          --algorithm <name> [--profile fast|balanced|dense|extreme] \
          [--window N] [--normalization 0-3] \
          [--run-id <id>] [--out <path>] [subcommand-specific flags]
```

`--item-path` is a *file* (chunking operates on one content object's
plaintext), read fully into memory -- a production API constraint, not a
choice this binary makes.

### `boundaries`

Chunk-size histogram, quantiles (p50/p90/p99), count, and a throughput
sample.

- `--buckets <n>` (default 10): equal-width histogram bucket count.

The throughput field is named
`throughput_bytes_per_sec_smoke_non_decision_grade` on purpose: it is a
single in-process wall-clock pass with no quiet-machine guard and no
repetitions, so it is a smoke measurement only, never decision-grade timing
(this harness's shared rule).

### `dedup`

Deduplication ratio and estimated overhead over one item or several
versions of it, using production's own physical-cost constants
(`entrybound::chunker::{CHUNK_FRAME_OVERHEAD_BYTES,
INDEX_ENTRY_OVERHEAD_BYTES, CHUNK_REFERENCE_OVERHEAD_BYTES}`) so an
alternative algorithm's estimated cost is directly comparable to
production's own `ChunkingEvaluation`.

- `--versions <path1,path2,...>` (optional): additional files, each treated
  as another version of the same logical item (`--item-path` is always
  version one). Omit it to measure within-item dedup only.

### `shift-stability`

Applies three independent edits to `--item-path` -- an insertion, a
deletion, and an overwrite, each at its own seeded random offset -- and
reports, per edit: how many chunk boundaries survived (with insertions/
deletions correctly compared *shifted* by the edit's length delta, not
literally, and a boundary inside the edited region never counted
preserved) and how many bytes would need rewriting under content-addressed
dedup (chunks whose content hash does not appear anywhere in the original
chunk set).

- `--seed <u64>` (default 1)
- `--insert-bytes <n>` (default 32; `0` skips the insertion scenario)
- `--delete-bytes <n>` (default 32; `0` or larger than the input skips it)
- `--overwrite-bytes <n>` (default 32; `0` or larger than the input skips it)

This is the subcommand that shows CDC's headline property directly: run
`--algorithm fixed-size` and `--algorithm buzhash` (or any other CDC
algorithm) over the same item and compare `preserved_boundary_fraction`
across their edits -- fixed-size should be near zero for insert/delete
(every boundary past the edit shifts), while a real CDC algorithm should
stay well above it.

### `random-access`

Samples random logical byte ranges and measures random-access
amplification: the ratio of physical bytes that must be read (every chunk
overlapping the requested range, in full) to the bytes actually requested.

- `--range-bytes <n>` (default 4096): width of each sampled logical range.
- `--samples <n>` (default 20)
- `--seed <u64>` (default 1)

## Output schema

Every subcommand appends one `ebr.harness.raw.v1` row via
`ebr_common::results::ResultWriter` (see `research/harness/README.md`'s
"Output schema" for the envelope); `payload` is that subcommand's own
result struct serialized as JSON (`BoundariesResult`, `DedupResult`,
`ShiftStabilityResult`, `RandomAccessResult` -- see each module's doc
comments in `src/boundaries.rs`, `src/dedup.rs`, `src/shift_stability.rs`,
`src/random_access.rs` for every field).

## Example `ebr` runner spec command templates

See `research/harness/README.md`'s "How binaries plug into `ebr` runner
specs" for the placeholder contract (`{item_path}`, `{candidate}`, ...) and
why `--out` should be omitted when driving this binary from a spec (so its
one JSON line lands on stdout for the runner's `stdout_json` metric
source).

```yaml
# Chunk-size distribution for one candidate algorithm across a size sweep.
command: >-
  /root/eb-research/target/harness/release/ebr-chunk boundaries
  --item-path {item_path} --experiment-id EXP-CHUNK-BOUNDARIES-001
  --env-name wsl-ubuntu --algorithm {candidate} --profile {profile}
metrics:
  - {name: chunk_count, source: stdout_json, key: payload.chunk_count}
  - {name: mean_chunk_bytes, source: stdout_json, key: payload.mean_chunk_bytes}
  - {name: p99_chunk_bytes, source: stdout_json, key: payload.p99_chunk_bytes}
```

```yaml
# Cross-version dedup ratio for a candidate algorithm, given a second
# version of the same logical item pre-staged next to it.
command: >-
  /root/eb-research/target/harness/release/ebr-chunk dedup
  --item-path {item_path} --experiment-id EXP-CHUNK-DEDUP-001
  --env-name wsl-ubuntu --algorithm {candidate} --profile balanced
  --versions {scratch}/version-2.bin
metrics:
  - {name: dedup_ratio, source: stdout_json, key: payload.dedup_ratio}
  - {name: estimated_cost_bytes, source: stdout_json, key: payload.estimated_cost_bytes}
```

```yaml
# Shift-stability: fixed-size versus a CDC algorithm, same edit sizes.
command: >-
  /root/eb-research/target/harness/release/ebr-chunk shift-stability
  --item-path {item_path} --experiment-id EXP-CHUNK-SHIFT-001
  --env-name wsl-ubuntu --algorithm {candidate} --profile balanced
  --seed {seed} --insert-bytes 64 --delete-bytes 64 --overwrite-bytes 64
metrics:
  - {name: insert_preserved_fraction, source: stdout_json, key: 'payload.edits[0].preserved_boundary_fraction'}
```

```yaml
# Random-access amplification at a fixed logical range width.
command: >-
  /root/eb-research/target/harness/release/ebr-chunk random-access
  --item-path {item_path} --experiment-id EXP-CHUNK-RANDOMACCESS-001
  --env-name wsl-ubuntu --algorithm {candidate} --profile {profile}
  --range-bytes 4096 --samples 50 --seed {seed}
metrics:
  - {name: mean_amplification, source: stdout_json, key: payload.mean_amplification}
```

## What it provides (library API)

- `production_chunk_ranges` -- a thin re-export of
  `entrybound::chunker::chunk_ranges`, so research code has one place to
  call into production chunking.
- `algorithms::Algorithm` -- the dispatch enum every subcommand uses:
  `Algorithm::from_profile(name, profile, window, normalization)` builds
  one from CLI-shaped inputs, `.chunk_ranges(data)` runs it, and
  `.algorithm_id()` names it. See `src/algorithms/mod.rs`.
- `boundaries::measure`, `dedup::measure`, `shift_stability::measure`,
  `random_access::measure` -- the library functions behind each
  subcommand, independently usable from other research code or tests.
- `SeededRng` -- the small deterministic PRNG `shift_stability` and
  `random_access` share for seeded offsets/filler bytes.

## Non-goals (for now)

- No encrypted-boundary chunking (`entrybound::chunker::chunk_ranges_encrypted`
  / `EncryptedBoundaryKey`) yet -- production-only research on that path is
  future work.
- `tttd`'s window is not yet exposed as a CLI override (it always uses
  `rabin::DEFAULT_WINDOW`); the `Params` field exists for when that's
  needed.
- `dedup`'s cross-version measurement treats every version as an
  independent whole file; it does not yet report *per-version* dedup
  contribution (how much version 2 added on top of version 1 specifically)
  versus the aggregate.

## Testing

```sh
# WSL (pinned toolchain, this agent's own target dir):
CARGO_TARGET_DIR=/root/eb-research/target/b2-chunk /root/.cargo/bin/cargo +1.98.1 test -p ebr-chunk

# Windows:
$env:CARGO_TARGET_DIR = "D:/eb-research/target/b2-chunk-win"
& "$env:USERPROFILE\.cargo\bin\cargo.exe" +1.98.1 test -p ebr-chunk
```

All tests generate their own pseudo-random byte buffers in memory; nothing
in this crate reads `research/corpus` or `/root/eb-research` unless a
caller points `--item-path` there. Coverage includes, per algorithm:
boundary determinism, min/max enforcement, empty-input handling, and the
"a small edit only desynchronizes nearby boundaries" property that
distinguishes CDC from fixed-size chunking (`fixed_size`'s own test shows
the contrasting negative case). `fastcdc2016`/`fastcdc2020` additionally
cross-check every production profile x normalization level against the
pinned `fastcdc` reference crate, byte-for-byte; `fastcdc2020` additionally
asserts it produces identical boundaries to `fastcdc2016` for the same
parameters (the 2020 paper's own claim -- faster, not different). `rabin`
additionally verifies its incremental rolling state against a
brute-force-recomputed fingerprint of the trailing window at several
positions. `lib.rs` verifies `gear-norm-v1` routed through `Algorithm`
matches calling `entrybound::chunker::chunk_ranges` directly (the property
that actually matters for every subcommand: dispatching through this
crate's uniform interface must never change what production computes).
