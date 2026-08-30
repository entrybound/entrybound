# Entrybound

Entrybound is an archive system built around one canonical semantic model: an
archive has one interpretation of its entries, content, and metadata, while
physical layout and transforms remain replaceable implementation choices.

The native `.eb` format and this implementation are **experimental and not yet
stable**. Do not use this version as a long-term archival or production format.

## Rust implementation status

Rust is the primary implementation language. The Cargo workspace currently
establishes:

- the `entrybound` library package, with separate EAM, ECF, identity,
  diagnostics, and archive-operation modules;
- the thin `entrybound-cli` package, producing the canonical `ebound`
  executable and an `entrybound` compatibility alias;
- typed EAM foundations for directories, regular files, ContentObjects,
  content-addressed Chunks, TransformPlans, metadata, FidelityReport, and a
  non-authoritative Index;
- structural UTF-8 LogicalPaths with canonical ordering, explicit-ancestor,
  duplicate-path, `.` and `..`, and file-prefix invariant checks;
- distinct LAI, PCR, AUX, and PCI types, stable diagnostic classes/reason
  codes, and a caller-owned extraction-policy boundary.
- deterministic canonical serialization of validated in-memory EAM into
  unencrypted Complete INDEXED `.eb` bytes;
- native reopen and verification covering canonical encoding, container and
  section structure, plaintext content, Entry identities, LAI, PCR, AUX, and
  exact-byte PCI;
- optional Index validation and rebuilding from authoritative CHUNK_DATA.
- deterministic creation-time compression planning with historical v1–v5
  compatibility and current `fast-v6`, `balanced-v6`, `dense-v6`, and
  `extreme-v6` policies;
- normalized Gear-hash content-defined chunking with versioned min/target/max
  parameters and archive-wide exact plaintext Chunk deduplication;
- governed codec/transform registries and operational per-Chunk STORE,
  Zstandard, LZ4, and raw LZMA2 TransformPlans, with declared and caller-enforced
  decoder-memory requirements;
- first-class ordered `delta8/v1` and `byte-shuffle/v1` structural-transform
  pipelines whose recorded parameters fully determine inverse decoding;
- bounded, bit-exact `deflate-reconstruct/v1` pipelines for eligible complete
  raw DEFLATE, zlib, and single-member gzip Chunks, with digest-addressed
  ReconstructionData and mandatory writer-side byte equality verification;
- whole-ContentObject `ReconstructionRegion` representations and opportunistic
  bit-exact `jpeg-jxl-reconstruct/v1`, with mandatory writer revalidation and
  declared whole-region access cost;
- deterministic binary similarity cohorts, cost-qualified shared Zstandard
  dictionaries, and explicitly bounded ChunkGroups for dense/extreme packing;
- deterministic filesystem packing for UTF-8 directory and regular-file
  trees, including bounded same-handle source-change detection;
- capability-relative, component-at-a-time extraction with exclusive file and
  directory creation, collision refusal, and pre-materialization verification;
- working `pack`, `unpack`, `list`, `inspect`, `verify`, and compression
  `explain` CLI commands;
- capture and restoration of `core.mtime` and, on Unix, `core.executable`, with
  an in-band FidelityReport for metadata the bootstrap does not preserve;
- caller-owned resource limits enforced against the archive declaration before
  authoritative records are decoded.

## Native bootstrap workflow

```sh
ebound pack ./example example.eb
ebound pack ./example example-fast.eb --profile fast
ebound pack ./example example-dense.eb --profile dense
ebound pack ./example example-extreme.eb --profile extreme
ebound verify example.eb
ebound list example.eb
ebound inspect example.eb
ebound explain example.eb
ebound unpack example.eb ./restored
```

`pack` uses `<input-name>.eb` when its output is omitted. `unpack` uses the
archive path without its extension when its destination is omitted. Archive
output files and extracted entries are created exclusively; existing objects
are refused. `balanced` is the default creation profile. Profiles affect
packing only; every archive records the TransformPlans needed for
self-describing decompression.

## Deliberately not implemented

This slice uses deterministic normalized content-defined chunking, stores each
exact plaintext Chunk once, and measures complete-cost candidates across
STORE, Zstandard, LZ4, raw LZMA2, structural pipelines, verified DEFLATE
reconstruction, opportunistic JPEG/JPEG XL whole-object reconstruction, shared
Zstandard dictionaries, and bounded Zstandard lookback.
It writes only unencrypted Complete INDEXED archives. It
supports UTF-8 directory names, directories, and regular files, and deliberately
rejects symlinks and special files. There is no legacy ZIP/tar/7z import,
STREAM layout, encryption, signing, lossy image recompression, guaranteed
support for every JPEG producer/marker combination, embedded-stream scanning,
unbounded solid compression, hardlink
metadata, ACLs, xattrs, ownership, platform-specific extended metadata,
mounting, recovery, language bindings, Go compatibility layer, or FFI.

Extraction is rooted in a held capability directory and resolves every
LogicalPath component relative to that handle. The current implementation uses
`cap-std`'s sandboxed filesystem API on Linux, macOS, FreeBSD, and Windows and
reports this confinement mode. Only collision policy `Refuse` is implemented.

The CLI's explicit bootstrap resource defaults are 1,000,000 entries, 64 GiB
total logical bytes, 16 GiB per file, 4,000,000 chunks, path depth 1,024, and
1 GiB of manifest/metadata bytes. Decoder policy permits an 8 MiB codec window
and 384 MiB aggregate working set, including LZMA2, stored-dictionary,
bounded-group access, the bounded 80 MiB DEFLATE reconstruction working set,
and the bounded 256 MiB JPEG/JPEG XL reconstruction working set. These are compatibility
limits, not claims that every machine can safely process archives of those
sizes; embedders can and should supply narrower caller-owned limits.

## Build and use

Install a stable Rust toolchain with rustfmt and Clippy. The workspace declares
its toolchain and minimum supported Rust version.

```sh
cargo build -p entrybound-cli --bin ebound
cargo run -p entrybound-cli --bin ebound -- --help
cargo install --path crates/entrybound-cli --bin ebound
```

`entrybound` is also built as a compatibility alias backed by the same CLI
implementation. New scripts and documentation should use `ebound`.

## Targeted checks

```sh
cargo fmt --all --check
cargo check -p entrybound -p entrybound-cli
cargo clippy -p entrybound -p entrybound-cli --all-targets -- -D warnings
cargo test -p entrybound -p entrybound-cli
```

See [the bootstrap format note](docs/format-v0.md) for the canonical encoding
and identity choices, [the filesystem bootstrap note](docs/filesystem-bootstrap.md)
for capture, confinement, and policy behavior,
[the planner-v1 note](docs/planner-v1.md) for frozen profiles and the
minimum-gain rule, [the CDC/deduplication note](docs/chunking-v1.md) for v2
chunking policies and exact physical sharing,
[the cross-file compression note](docs/cross-file-compression-v1.md) for v3
similarity, Dictionary, and bounded ChunkGroup behavior,
[the codec/transform note](docs/codec-transform-v1.md) for v4 registries,
candidate sets, wire feature, and exact reversible transforms, and
[the reconstructive-transform note](docs/reconstructive-transform-v1.md) for
v5 DEFLATE recognition, exact reconstruction, resource limits, and wire
evolution, [the JPEG reconstruction note](docs/jpeg-reconstruction-v1.md) for
v6 whole-object regions, eligibility, byte-exact JPEG reconstruction, and
random-access costs, and
[CONTRIBUTING.md](CONTRIBUTING.md) for development conventions.
