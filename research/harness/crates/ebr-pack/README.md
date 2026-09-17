# ebr-pack

End-to-end archive measurements using entrybound's **public** production API:
`plan -> encode -> open -> verify -> unpack -> random read`, with byte
accounting at every stage.

## What it measures

`measure_roundtrip(input_dir, profile)` runs, over one directory tree:

1. **plan** -- `entrybound::archive::plan_directory` (builds and plans an EAM `Archive` from the filesystem, without encoding it).
2. **encode** -- `entrybound::ecf::encode` (canonical native ECF bytes).
3. **open** -- `entrybound::ecf::open` (full open + verify of the encoded bytes).
4. **verify** -- `entrybound::ecf::verify` (the standalone verification report).
5. **unpack** -- `entrybound::archive::unpack` into a scratch directory this crate creates and removes itself.
6. **random read** -- `entrybound::ecf::open_indexed_random` + `RandomAccessArchive::read_entry` over an in-memory source (`entrybound::random_access::MemoryRandomReadSource`), reading back the first regular file.

Every stage's wall-clock time (`ebr_common::timing::Stopwatch`, monotonic) and
relevant byte counts are returned in `RoundtripMeasurement`. Input logical
bytes and file count come from `ebr_common::walk::walk_tree`, not from
entrybound (so the input-side accounting is measured independently of the
library under test).

This is a first cut: encrypted archives, STREAM layout, and repack/diff are
all also public API (`entrybound::crypto`,
`entrybound::ecf::{encode_stream, open_stream, verify_stream}`,
`entrybound::archive::{prepare_repack, archive_diff}`) that a later
experiment can add a measurement path for.

## The `research-internals` feature

`Cargo.toml` enables `entrybound`'s `research-internals` feature (see
`research/harness/research-internals.md`) so later measurement code added to
this crate can reach `entrybound::research` without a dependency change.
`src/lib.rs` does not use it yet -- it only calls the public API listed
above.

## The `ebr-pack` binary

```text
ebr-pack --item-path <dir> --experiment-id <id> --env-name <name> \
         [--profile fast|balanced|dense|extreme] [--run-id <id>] [--out <path>]
```

- `--item-path`: a directory to pack (a corpus item's materialized path, or
  any other directory).
- `--experiment-id`: free-form label, becomes the JSONL row's `experiment_id`.
- `--env-name`: one of `research/environment/index.json`'s recorded names
  (`windows-host`, `wsl-ubuntu`, `docker-ubuntu-24.04`); resolved to that
  environment's current `env_id`.
- `--profile`: default `balanced`.
- `--run-id`: default a freshly generated id (`ebr_common::timing::generate_run_id`).
- `--out`: append the JSONL row to this file (parent directories created as
  needed); default stdout.

This is exactly the shape an `ebr` runner spec's command template fills in
(see the top-level `research/harness/README.md`).

Example:

```sh
CARGO_TARGET_DIR=/root/eb-research/target/harness \
  /root/.cargo/bin/cargo +1.98.1 run -p ebr-pack --release -- \
  --item-path /root/eb-research/corpus/tuning/f01-tuning-ripgrep-14-1-1-git \
  --experiment-id EXP-PACK-SMOKE --env-name wsl-ubuntu
```

## Testing

```sh
cargo +1.98.1 test -p ebr-pack
```

Tests build their own small scratch input trees under `std::env::temp_dir()`
and remove them afterward; nothing in this crate reads `research/corpus` or
`/root/eb-research` unless a caller points `--item-path` there.
