# ebr-chunk

Chunking/dedup research: production `gear-norm-v1` (`entrybound::chunker`)
compared against research-only alternative content-defined chunking (CDC),
on the same input.

## What it provides

- `production_chunk_ranges` -- a thin re-export of
  `entrybound::chunker::chunk_ranges`, so research code has one place to call
  into production chunking.
- `alt_cdc` -- a Buzhash-windowed rolling-hash CDC implementation, deliberately
  a different construction from production's gear hash (fixed window +
  rotate/XOR, versus gear's whole-run shift/add). Not wired into any
  production code path. Its test suite includes the property that actually
  distinguishes CDC from fixed-size chunking: inserting a few bytes near the
  start of a buffer only desynchronizes *nearby* chunk boundaries, not every
  boundary after the insertion point.
- `compare(data, production_parameters)` -- runs both chunkers over the same
  bytes at matching min/target/max sizes
  (`alt_cdc::BuzhashParameters::matching`) and returns each one's
  `ChunkerSummary` (chunk count, min/max/mean chunk size, and a
  dedup-after-chunking count via SHA-256 of each chunk's bytes).

## Non-goals (for now)

- No encrypted-boundary chunking (`entrybound::chunker::chunk_ranges_encrypted`
  / `EncryptedBoundaryKey`) yet -- production-only research on that path is
  future work.
- `alt_cdc` is one alternative, not an exhaustive CDC survey. Adding a second
  (e.g. a modulus-based Rabin fingerprint, or a plain fixed-size baseline for
  contrast) is a natural next step and does not require changing this
  module's shape.
- `compare`'s dedup count hashes whole chunks with SHA-256
  (`ebr_common::hash::sha256_hex`); it does not yet measure cross-item
  (corpus-wide) dedup, only within one input.

## The `ebr-chunk` binary

```text
ebr-chunk --item-path <file> --experiment-id <id> --env-name <name> \
          [--profile fast|balanced|dense|extreme] [--run-id <id>] [--out <path>]
```

`--item-path` is a *file* (chunking operates on one content object's
plaintext), read fully into memory -- chunking needs the whole slice, which is
a production API constraint, not a choice this binary makes. `--profile`
selects one of `entrybound::chunker`'s frozen presets (`FAST_V2`,
`BALANCED_V2`, `DENSE_V2`, `EXTREME_V2`); default `balanced`. See
`research/harness/README.md` for the shared flag contract.

## Testing

```sh
cargo +1.98.1 test -p ebr-chunk
```

All tests generate their own pseudo-random byte buffers in memory; nothing in
this crate reads `research/corpus` or `/root/eb-research` unless a caller
points `--item-path` there.
