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
- the thin `entrybound-cli` binary package;
- typed EAM foundations for directories, regular files, ContentObjects,
  content-addressed Chunks, TransformPlans, metadata, FidelityReport, and a
  non-authoritative Index;
- structural UTF-8 LogicalPaths with canonical ordering, explicit-ancestor,
  duplicate-path, `.` and `..`, and file-prefix invariant checks;
- distinct LAI, PCR, AUX, and PCI types, stable diagnostic classes/reason
  codes, and a caller-owned extraction-policy boundary.

This language-standardization slice does not yet serialize or open `.eb`
archives. The CLI entry point exists and reports unfinished archive commands as
unsupported instead of pretending they work.

## Deliberately not implemented

The native writer, reader, pack, unpack, list, inspect, and verify operations
are not implemented yet. There is also no legacy ZIP/tar/7z import, STREAM
layout, encryption, signing, advanced compression, intelligent planning,
content-defined chunking, symlink or special-file support, hardlink metadata,
ACLs, xattrs, platform-specific metadata, mounting, recovery, language
bindings, Go compatibility layer, or FFI.

## Build and use

Install a stable Rust toolchain with rustfmt and Clippy. The workspace declares
its toolchain and minimum supported Rust version.

```sh
cargo build -p entrybound-cli
cargo run -p entrybound-cli -- --help
```

## Targeted checks

```sh
cargo fmt --all --check
cargo check -p entrybound -p entrybound-cli
cargo clippy -p entrybound -p entrybound-cli --all-targets -- -D warnings
cargo test -p entrybound -p entrybound-cli
```

See [the bootstrap format note](docs/format-v0.md) for the canonical encoding
and identity choices, and [CONTRIBUTING.md](CONTRIBUTING.md) for development
conventions.
