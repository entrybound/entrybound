# ebr-access

Random/remote access and streaming/memory measurements, over entrybound's
public production API. This crate never changes production semantics and
touches only entrybound's public surface (`entrybound::ecf`,
`entrybound::archive`, `entrybound::random_access`,
`entrybound::legacy::export`), plus the default-off `research-internals`
feature declared on its `entrybound` dependency (not used by any code in
this crate today -- see `research/harness/README.md`'s "Dependency on
production").

## What it provides

Three independent pieces, each with its own module doc comment covering the
details this README only summarizes:

### `measure_random_access` (`src/random_access.rs`)

INDEXED metadata-first random access
(`entrybound::ecf::open_indexed_random`), generic over any
`entrybound::random_access::RandomReadSource` -- `MemoryRandomReadSource`,
`LocalFileRandomReadSource`, or `HttpRangeSource` all work through the same
function. Over one opened archive, in order:

1. **first-entry read** -- a dedicated `read_entry` call for the first
   entry, taken before anything else touches the session, so its latency is
   the coldest the session can offer. (INDEXED `read_entry` always returns a
   whole `ContentObject`'s plaintext at once, so "time to first plaintext
   byte" and "time to the complete first entry" are the same measurement
   against today's production API.)
2. **a seeded random sample** of entries (`RandomAccessConfig::sample_size`,
   `seed`), reproducible across runs.
3. **eager full consumption** -- every requested entry, once, in order.

Every read carries its own timing and byte accounting
(`RandomReadMeasurement`) plus its full `RandomAccessVerificationReport`
flags (`VerificationFlags`). **Byte counts are per-read deltas** (harness
review round 1, finding R1-02): the report's `bytes_fetched` and
`range_request_count` are *session-cumulative* counters, so each read's
`bytes_fetched_from_source`/`range_request_count` is the difference from the
session counters before that read, `metadata_open_bytes_fetched` is the
metadata-first open's own cost, and `session_total_bytes_fetched` equals the
open plus every read's delta (unit-tested). Earlier revisions reported the
cumulative counters as per-read costs and summed them, overstating
`eager_total_bytes_fetched` roughly quadratically in the number of reads;
`VerificationFlags` still carries the raw cumulative values, labeled as
such. `--source-kind local-file` reads a file this process wrote moments
earlier, so its page cache is hot (`source_page_cache` says so in the row). **Random byte-range reads within large entries**:
whenever a read returns a plaintext at or above
`large_entry_threshold_bytes`, `byte_ranges_per_large_entry` seeded random
`(offset, len)` slices are additionally cut from it and timed
(`ByteRangeReadMeasurement`) -- since `read_entry` has no sub-object range
API, the slice itself is pure in-memory work; the accompanying
`RandomReadMeasurement` is what a caller should read to learn what a "just
this byte range" request truly costs a range-backed source today.
`RandomAccessConfig::policy` is a full caller-owned
`entrybound::random_access::RandomAccessPolicy`, so every knob it exposes
(`coalesce_gap_bytes`, `max_dependency_chunks`,
`max_individual_range_bytes`, `max_total_bytes_fetched`, ...) is in play,
not just the defaults.

### `measure_stream` (`src/stream.rs`)

STREAM sequential `encode -> open -> verify -> list -> unpack`
(`entrybound::ecf::{encode_stream, open_stream, verify_stream}`,
`entrybound::archive::{list, unpack_stream}`), the layout meant for a source
that cannot seek. `open_stream` is STREAM's only verification pass
(`StreamAccessProfile::listing_requires_full_scan` is always `true`), so
`list` here runs cheaply on top of that already-open model rather than
pretending STREAM has a cheap first listing. Every pass's `StreamReport`
memory/scratch fields (`peak_retained_chunks`,
`peak_resident_staging_bytes`, `spilled_staging_bytes`) are surfaced
(`StreamPassStats`) for the `open` and `unpack` passes.

### The streaming memory probe (`src/probe.rs`, re-exported at the crate root)

`run_probe` drives a synthetic, seed-derived, generated-on-the-fly input
(`SyntheticGenerator`, bounded-memory regardless of size) through
`pack -> verify -> unpack` (and, opt-in, `repack`/`export`), sampling this
process's current resident memory on a background thread at a fixed
interval throughout each stage, with a phase-scoped high-water mark per
stage (`memory`, `ebr_common::measure::ScopedMemory`). **What it already found**:
`entrybound::archive::plan_directory` holds every Chunk's plaintext in
memory for as long as the returned `Archive` is alive
(`entrybound::eam::Chunk::plaintext`), regardless of which layout is
encoded afterward -- so `pack` costs at least the input's total size in
process memory today, streaming sink or not. `probe.rs`'s module doc
explains this (and the "cannot avoid storing the synthetic input on scratch
disk first" constraint, and why `repack`/`export` are opt-in) in full.

**Stage memory (harness review round 1, finding R1-05).** Earlier revisions
sampled `getrusage`'s process-lifetime peak and kept the planned `Archive`
(every Chunk's plaintext) alive through `verify` and `unpack`, so every stage
reported `pack`'s peak and bounded streaming memory could never be observed.
Now each stage resets the Linux high-water mark at its start, `repack`/
`export` run right after `pack` and the archive is dropped before `verify`/
`unpack`, and with `--probe-isolate true` (the binary's default) `verify`
and `unpack` each run in a fresh child process (`--mode probe-stage`), whose
lifetime peak is also reported. Streaming-memory claims must come from
isolated runs.

Both `measure_random_access` and `measure_stream` attach phase-scoped
memory (`access_memory`; `encode_memory`/`open_memory`/`verify_memory`/
`unpack_memory`) rather than the process-lifetime peak, which in `roundtrip`
mode always includes planning and encoding the archive being read.

## Non-goals (for now)

- Random access here can run over `LocalFileRandomReadSource` or
  `HttpRangeSource` as well as `MemoryRandomReadSource`
  (`--source-kind local-file|http`), but this binary has no HTTP server of
  its own: `--source-kind http` requires `--source-url` to already serve
  these exact `--item-path` encoded bytes with byte-range support.
  `ebr-netem` (built separately) is the network emulation proxy/origin a
  later experiment would put in front of a real remote random-access run to
  get range-request-count-under-latency numbers.
- The probe's own tests only ever request a few hundred KiB -- enough to
  prove the mechanism, not to produce decision-grade 1-64 GiB scaling
  numbers (see `src/probe.rs`'s module docs, "Actually running this at
  1-64 GiB"). Every wall-clock number this crate has produced so far is
  non-decision-grade smoke timing.
- No `publish` stage in the probe: entrybound's public API has no operation
  named or shaped like a "publish" step beyond `encode`/`encode_stream`
  (which `pack` already measures), so the probe does not invent one.

## The `ebr-access` binary

Two modes, chosen with `--mode` (default `roundtrip`):

```text
ebr-access --mode roundtrip --item-path <dir> --experiment-id <id> --env-name <name> \
           [--profile fast|balanced|dense|extreme] \
           [--source-kind memory|local-file|http] [--source-url <url>] \
           [--sample-size <n>] [--sample-seed <n>] \
           [--large-entry-threshold-bytes <n>] [--byte-ranges-per-large-entry <n>] \
           [--byte-range-len <n>] \
           [--coalesce-gap-bytes <n>] [--max-dependency-chunks <n>] \
           [--max-total-bytes-fetched <n>] [--max-individual-range-bytes <n>] \
           [--run-id <id>] [--out <path>]

ebr-access --mode probe --experiment-id <id> --env-name <name> \
           [--item-path <label>] [--profile fast|balanced|dense|extreme] \
           [--probe-total-bytes <n>] [--probe-seed <n>] \
           [--probe-sample-interval-ms <n>] \
           [--probe-stages pack,verify,unpack,repack,export] \
           [--probe-scratch-root <dir>] [--probe-isolate true|false] \
           [--run-id <id>] [--out <path>]
```

`roundtrip` plans and encodes `--item-path` once, then runs
`measure_random_access` over every regular-file entry and `measure_stream`
over the same planned archive. `probe` runs the streaming memory probe over
a synthetic generated input instead of a real `--item-path` (`--item-path`
is accepted there only as a label for the JSONL row's `item_id`; it is
never read from disk in `probe` mode). Flags not listed here (`--out`,
`--run-id`, `--experiment-id`, `--env-name`) follow
`research/harness/README.md`'s shared contract. Every numeric flag has a
default matching `RandomAccessConfig::default()` / `ProbeConfig::default()`
if omitted.

### Example `ebr` runner spec command templates

`roundtrip`, sweeping `--source-kind` as a candidate parameter:

```yaml
command: >-
  /root/eb-research/target/harness/release/ebr-access
  --mode roundtrip --item-path {item_path} --experiment-id EXP-ACCESS-RANDOM-001
  --env-name wsl-ubuntu --profile {candidate}
  --source-kind {params.source_kind} --sample-size 32 --sample-seed {seed}
```

`roundtrip`, sweeping `RandomAccessPolicy`'s `coalesce_gap_bytes` to see its
effect on `range_request_count`:

```yaml
command: >-
  /root/eb-research/target/harness/release/ebr-access
  --mode roundtrip --item-path {item_path} --experiment-id EXP-ACCESS-COALESCE-001
  --env-name wsl-ubuntu --profile balanced
  --coalesce-gap-bytes {params.coalesce_gap_bytes}
```

`probe`, sweeping input size as a candidate parameter (`{params.total_bytes}`
in bytes -- keep this at smoke scale, e.g. `1048576`-`67108864`, unless the
spec has deliberately reserved a machine and disk budget for a decision-grade
1-64 GiB run; see `src/probe.rs`'s module docs):

```yaml
command: >-
  /root/eb-research/target/harness/release/ebr-access
  --mode probe --experiment-id EXP-ACCESS-PROBE-001 --env-name wsl-ubuntu
  --profile fast --probe-total-bytes {params.total_bytes}
  --probe-sample-interval-ms 50 --probe-stages pack,verify,unpack
```

Standalone, manual runs (outside any `ebr` spec):

```sh
CARGO_TARGET_DIR=/root/eb-research/target/harness \
  /root/.cargo/bin/cargo +1.98.1 run -p ebr-access --release -- \
  --mode roundtrip --item-path /root/eb-research/corpus/tuning/f01-tuning-ripgrep-14-1-1-git \
  --experiment-id EXP-ACCESS-SMOKE --env-name wsl-ubuntu

CARGO_TARGET_DIR=/root/eb-research/target/harness \
  /root/.cargo/bin/cargo +1.98.1 run -p ebr-access --release -- \
  --mode probe --experiment-id EXP-ACCESS-PROBE-SMOKE --env-name wsl-ubuntu \
  --probe-total-bytes 8388608 --probe-stages pack,verify,unpack,repack,export
```

## Testing

```sh
cargo +1.98.1 test -p ebr-access
```

Tests build their own small scratch input trees (`random_access.rs`,
`stream.rs`) or synthetic byte streams (`probe.rs`) under
`std::env::temp_dir()` and remove them afterward; nothing in this crate
reads `research/corpus` or `/root/eb-research` unless a caller points
`--item-path`/`--source-url` there.
