# ebr-codec

Codec/transform/reconstruction research. Evaluates, over the same input:

- **Production codecs and plans exactly**, reached through `entrybound`'s
  default-off `research-internals` feature
  (`entrybound::research::{codec, transform, reconstruction,
  jpeg_reconstruction}`): STORE, Zstandard at every supported level,
  dictionary mode, prefix (bounded-lookback) mode, LZ4, raw LZMA2 at all
  three production presets, the `delta8`/`byte-shuffle` structural
  pipelines, DEFLATE-reconstruction, and JPEG-to-JPEG-XL whole-object
  reconstruction. See `research/harness/research-internals.md` for the
  identity guarantees this relies on: nothing here changes production
  behavior or archive bytes.
- **Candidate external codecs not in production**, each pinned exactly in
  `Cargo.toml` (matching production's own pin where the crate is also a
  production dependency): brotli, bzip2, DEFLATE/zlib at every level, xz with
  BCJ x86/ARM64 filters, LZ4 HC levels, Zstandard long-distance-matching
  variants, and PPMd7.
- **Structural transform candidates beyond production's set**: BCJ
  x86/ARM64/RISC-V (standalone, not inside an xz container), float/integer
  delta-of-delta, and byte-plane splitting at widths production's own
  `byte-shuffle` refuses (anything other than 2, 4, or 8).

## Module map

| Module | What it wraps |
|---|---|
| `production` | `roundtrip_store`, `roundtrip_zstd`, `roundtrip_zstd_dictionary`, `roundtrip_zstd_prefix`, `roundtrip_zstd_delta8`, `roundtrip_zstd_byte_shuffle`, `roundtrip_lz4`, `roundtrip_lzma2`, `roundtrip_deflate_reconstruct`, `roundtrip_jpeg_reconstruct` -- each builds a `TransformPlan` exactly the way production's planner would, validates it, and round-trips through the exact `encode_payload`/`decode_payload` (or the dictionary/prefix/reconstruction variant) production uses. |
| `external` | `roundtrip_zstd_crate`, `roundtrip_zstd_ldm`, `roundtrip_brotli`, `roundtrip_bzip2`, `roundtrip_deflate`, `roundtrip_zlib`, `roundtrip_xz` (optional BCJ filter), `roundtrip_lz4_hc`, `roundtrip_ppmd`. |
| `transform` | `bcj_forward`/`bcj_inverse` (x86/ARM64/RISC-V), `delta_of_delta_forward`/`_inverse`, `byte_plane_split_forward`/`_inverse`. Pure structural transforms, exactly invertible over arbitrary bytes; no codec attached. |
| `matrix` | `candidates(plaintext)` -- every production and external codec at a representative parameter set, over one buffer. `chunk_matrix(parameters, plaintext)` -- splits into chunks with production's real `chunk_ranges` and runs `candidates` per chunk. `transform_false_positive_analysis(plaintext)` -- applies every structural transform (production's `byte-shuffle` plus this crate's own), compresses with zstd level 3 before and after, and flags a transform as a false positive when it does not reduce the compressed size. |
| `lib.rs` | `compare_zstd_level` -- the original, narrower production-vs-external zstd comparison this crate started from; kept for the `compare` subcommand and its own test. |

Every `RoundtripResult`/`ExternalCodecResult` carries `encode_seconds`/
`decode_seconds`: a single in-process wall-clock sample
(`ebr_common::timing::Stopwatch`, so a monotonic clock) around that one
call. **This is a smoke timing, not decision-grade** (no quiet-machine
guard, no repetition, no warmup) -- see the Phase B2 rule in
`research/PROGRESS.md`. It exists so a matrix row is not silent about cost
entirely, not to support a timing claim.

## Why these external/structural candidates

- **brotli** (`brotli = "9.0.0"`, pure Rust, default features): production
  has no brotli codec at all; this is a from-scratch candidate.
- **bzip2** (`=0.6.1`) and **DEFLATE/zlib** (`flate2 =1.1.9`): both already
  production dependencies (bzip2 for legacy `.tar.bz2`/`.7z` reading, flate2
  for DEFLATE-reconstruction fixtures), but neither is used as an *archive
  codec* in production -- this crate pins the identical version so a size
  comparison is not confounded by a different build of the same algorithm.
- **xz with BCJ x86/ARM64** and the harness's own standalone BCJ transforms
  (`lzma-rust2 = "0.20.0"`, same exact pin and feature set --
  `std`/`encoder`/`xz` -- as `crates/entrybound`'s own dependency): production's
  `lzma2_plan` stores raw LZMA2 at three fixed `(preset, dictionary)` pairs,
  no xz container, no filter chain at all. `lzma-rust2` is the same crate
  production already trusts for LZMA2, and it happens to also implement
  BCJ x86/ARM/ARM64/ARM-Thumb/PowerPC/SPARC/IA-64/RISC-V filters
  (`lzma_rust2::filter::bcj::{BcjReader, BcjWriter}`) and a full `.xz`
  container with a configurable pre-filter chain (`XzOptions`,
  `prepend_pre_filter`) -- both reachable without adding a second LZMA
  binding.
- **LZ4 HC levels** (`lz4 = "1.28.1"`, a `liblz4` binding via `lz4-sys`):
  `lz4_flex` (`=0.14.0`, already a production dependency, backing
  `entrybound::codec::lz4_plan`) is fast-only and has no high-compression
  mode. Only a `liblz4` binding reaches HC.
- **Zstandard long-distance matching** (the pinned `zstd = "0.13.3"` crate
  the crate already depended on for the original zstd-vs-zstd comparison):
  production's own `encode_zstd` (`crates/entrybound/src/codec.rs`) always
  calls `long_distance_matching(false)` and ties the window log to the
  level. `roundtrip_zstd_ldm` calls `zstd::bulk::Compressor` directly with
  LDM on and an explicit window log -- a parameter combination
  `TransformPlan` cannot express at all.
- **PPMd7** (`ppmd-rust = "1.5.0"`): production has no PPMd codec.
  Surveyed alternatives on crates.io: `ppmd` (version `0.0.0`, a reserved
  placeholder, not usable), `ppmd-sys` (raw FFI bindings, an `unsafe`
  surface this crate would have to wrap itself), `ppmd-core` and
  `omnizip-ppmd` (much lower adoption, no recent activity signal at the time
  of this survey). `ppmd-rust` implements PPMd7 (PPMdH, the variant 7-Zip's
  `.7z` PPMd uses) and PPMd8, is actively maintained (a release the same
  week this crate was written), and its public API
  (`Ppmd7Encoder`/`Ppmd7Decoder`, `Read`/`Write`-based) needs no `unsafe` at
  the call site -- satisfying this workspace's `unsafe_code = "forbid"`
  lint without needing an allow. (Its own internals use `unsafe` for the
  PPM model's arena allocator, same as most PPMd ports; that is a property
  of the dependency, not of any code in this workspace.)

## The `ebr-codec` binary

```text
ebr-codec <subcommand> --item-path <file> --experiment-id <id> --env-name <name> \
          [--level <i32>] [--profile <fast|balanced|dense|extreme>] \
          [--run-id <id>] [--out <path>]
```

The subcommand is the first positional argument. `compare` is assumed when
it is missing (so any existing invocation from before this crate had
subcommands keeps working unchanged).

| Subcommand | What it runs | Rows written |
|---|---|---|
| `compare` (default) | `compare_zstd_level` at `--level` (default `3`, must be one of `entrybound::research::codec::SUPPORTED_LEVELS`: `1, 3, 5, 9, 15, 19, 22`) | one |
| `object-matrix` | `matrix::candidates` over the whole item | one per candidate |
| `chunk-matrix` | splits the item with `matrix::chunking_parameters_for_profile(--profile)` (default `balanced`), then `matrix::candidates` per chunk | one per (chunk, candidate) |
| `transform-false-positives` | `matrix::transform_false_positive_analysis` over the whole item | one per structural transform |

Every row is one `ebr.harness.raw.v1` JSONL line (see
`research/harness/README.md`'s "Output schema"); `chunk-matrix`'s `item_id`
is `<item-path>#chunk=<index>`.

### Example `ebr` runner spec command templates

`object-matrix`/`chunk-matrix`/`transform-false-positives` all write *more
than one row per invocation*, unlike most binaries in this workspace --
`research/harness/README.md`'s guidance to omit `--out` so a spec's
`stdout_json` metric source can read a single line's `payload` does **not**
apply to them. Point `--out` at a scratch file and post-process the JSONL
instead:

```yaml
# Per-chunk matrix at the balanced profile.
command: >-
  /root/eb-research/target/harness/release/ebr-codec
  chunk-matrix --item-path {item_path} --experiment-id EXP-CODEC-001
  --env-name wsl-ubuntu --profile balanced --out {scratch}/chunk-matrix.jsonl
```

```yaml
# Whole-object matrix.
command: >-
  /root/eb-research/target/harness/release/ebr-codec
  object-matrix --item-path {item_path} --experiment-id EXP-CODEC-002
  --env-name wsl-ubuntu --out {scratch}/object-matrix.jsonl
```

```yaml
# Structural-transform false-positive scan.
command: >-
  /root/eb-research/target/harness/release/ebr-codec
  transform-false-positives --item-path {item_path} --experiment-id EXP-CODEC-003
  --env-name wsl-ubuntu --out {scratch}/transform-false-positives.jsonl
```

`compare` is the one subcommand that still fits the single-row
`stdout_json` convention directly (`--out` omitted):

```yaml
command: >-
  /root/eb-research/target/harness/release/ebr-codec
  compare --item-path {item_path} --experiment-id EXP-CODEC-000
  --env-name wsl-ubuntu --level {candidate}
```

## Non-goals (for now)

- The matrix's parameter lists (`matrix.rs`'s `MATRIX_*` constants) are a
  deliberately small, representative subset per codec (e.g. 3 of zstd's 7
  `SUPPORTED_LEVELS`), not an exhaustive sweep -- a full sweep over a real
  corpus item would make `chunk-matrix` impractically slow. Widening a list
  is a one-line change if a later experiment needs it.
- Dictionary and prefix modes are exercised in `production.rs`'s own unit
  tests but are **not** included in `matrix::candidates`: they need a
  second, related sample (a trained dictionary, or a preceding cohort
  member's bytes), which does not fit a single-input matrix row.
- The false-positive scan's reference codec is fixed at zstd level 3 (a
  fast, representative default); it does not sweep the reference codec
  itself.
- No PPMd8 (only PPMd7/PPMdH, matching 7-Zip's variant), and no LZMA (only
  raw LZMA2 in production and xz-in-LZMA2 in `external.rs`) as a bare
  filter-less stream.

## Testing

```sh
cargo +1.98.1 test -p ebr-codec
```

All unit tests use small in-memory buffers built from a repeating string or
a deterministic xorshift generator; nothing in this crate reads
`research/corpus` or `/root/eb-research` unless a caller points
`--item-path` there.

`tests/production_archive_equality.rs` is the "production-plan byte
equality with archive payloads" check: it packs a real fixture tree with
production's own (non-research) `entrybound::archive::pack_directory`
across all four `CompressionProfile`s, then for every `PlanMode::Independent`
chunk, re-encodes that chunk's plaintext through
`entrybound::research::codec::encode_payload` with the exact `TransformPlan`
the real packer assigned, and asserts the result is byte-identical to the
archive's own stored bytes at that chunk's `Index` location. Two findings
from writing this test, useful to anyone else navigating
`entrybound::eam::{Archive, Chunk, Index}` from a research crate:

- **`Chunk::plan_ref` is a `TransformPlan::plan_id` to look up, not a
  positional index** into `archive.transform_plans` (confirmed against
  `entrybound::ecf::container`'s own `plans.get(&chunk.plan_ref)` lookup,
  keyed by `plan_id`).
- **`Index`'s `ChunkLocation::offset` is the Chunk frame's absolute *header*
  start, not its stored-payload start**; `stored_len` is the payload-only
  length *after* that header. The header length is constant for one whole
  archive (`entrybound::research::ecf::chunk_frame_header_len(extended)`,
  where `extended`/`whole_object` are read from two archive-wide feature
  bits -- `entrybound::ecf::FEATURE_CROSS_FILE_COMPRESSION_V1` and
  `FEATURE_WHOLE_OBJECT_RECONSTRUCTION_V1` -- on `archive.descriptor.features.incompat`,
  both plain public constants, not research-gated).

Round-trip coverage per candidate:

- `production.rs`: every production path above, including both
  reconstruction candidates against real generated zlib/DEFLATE streams
  (and a documented-ineligible case for non-DEFLATE input).
- `external.rs`: every external codec at 3+ representative
  levels/qualities/presets each.
- `transform.rs`: every structural transform round-trips over pseudo-random
  bytes at several lengths (including lengths that are not a whole number
  of the transform's lane/element width, to exercise tail handling), plus a
  cross-check that this crate's width-2/4/8 byte-plane split is
  byte-for-byte identical to production's own `byte-shuffle/v1`
  (`entrybound::research::transform::byte_shuffle_step` +
  `forward_pipeline`) -- not a reimplementation that could drift unnoticed,
  the same algorithm, widened.
- `matrix.rs`: `candidates` round-trips (or is typed-ineligible) on every
  entry over a representative buffer; the false-positive scan is checked
  both ways -- BCJ on plain (non-executable) text is flagged as a false
  positive, and second-order delta on a linear ramp is not.

## Build and lint

```sh
# WSL (pinned toolchain, dedicated target dir per the Phase B2 shared rules):
CARGO_TARGET_DIR=/root/eb-research/target/b2-codec /root/.cargo/bin/cargo +1.98.1 test -p ebr-codec
CARGO_TARGET_DIR=/root/eb-research/target/b2-codec /root/.cargo/bin/cargo +1.98.1 clippy -p ebr-codec --all-targets -- -D warnings
CARGO_TARGET_DIR=/root/eb-research/target/b2-codec /root/.cargo/bin/cargo +1.98.1 fmt -p ebr-codec -- --check

# Windows:
$env:CARGO_TARGET_DIR = "D:/eb-research/target/b2-codec"
& "$env:USERPROFILE\.cargo\bin\cargo.exe" +1.98.1 test -p ebr-codec --manifest-path D:/Projects/entrybound/entrybound/research/harness/Cargo.toml
```

Both hosts: 28 tests pass (27 unit tests in `src/lib.rs`, including the
crate's original `compares_production_and_external_zstd_on_the_same_input`,
plus the 1 integration test above), clippy is clean at `-D warnings`, and
`cargo fmt --check` passes.
