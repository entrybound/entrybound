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
  unencrypted Complete `.eb` bytes in two physical layouts, random-access
  `INDEXED` and sequential `STREAM`, which encode the same archive model;
- native reopen and verification covering canonical encoding, container and
  section structure, plaintext content, Entry identities, LAI, PCR, AUX, and
  exact-byte PCI;
- optional Index validation and rebuilding from authoritative CHUNK_DATA.
- a sequential `stream-layout-v1` writer that needs only `std::io::Write` and a
  sequential reader that needs only `std::io::Read`, with tagged single-authority
  framing, a declared and enforced stream deduplication window, and a
  self-locating footer that keeps truncation distinguishable from corruption;
- bounded, spilling staging for sequential extraction, so unverified content
  never reaches a destination path and archive plaintext is never required in
  RAM;
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
- working `pack`, `unpack`, `list`, `inspect`, `verify`, compression `explain`,
  strict ZIP `convert`, `sign`, and authenticated `key` CLI commands;
- a format-neutral Legacy Observation Model, independent ZIP local/central/
  descriptor evidence, strict ZIP32/ZIP64 STORE/DEFLATE reconciliation, and an
  AUX-bound in-band conversion provenance record;
- capture and restoration of `core.mtime` and, on Unix, `core.executable`, with
  an in-band FidelityReport for metadata the bootstrap does not preserve;
- caller-owned resource limits enforced against the archive declaration before
  authoritative records are decoded.
- metadata-private crypto-v1 `INDEXED` archives using the frozen
  AES-256-GCM-SIV payload suite, X-Wing draft-10 hybrid recipients or a sole
  Argon2id password recipient, authenticated segments, keyed encrypted CDC,
  and authenticated record padding;
- public-only encrypted inspection plus authenticated verify and unpack paths;
  extraction still materializes nothing until the complete archive verifies.
- pure Ed25519 CONTENT/PHYSICAL/ADDRESSING signatures, exact detached `.ebsig`
  records, encrypted embedded signatures, and offline RFC 3161 verification
  against caller-provided trust anchors;
- authenticated recipient listing and hybrid-recipient addition without AFK
  rotation, plus recipient removal and password changes through verified
  fresh-AFK full re-encryption.

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

Strict ZIP import produces an ordinary native archive after independently
checking the foreign authorities, decompressed length, and CRC-32:

```sh
ebound convert input.zip converted.eb --strict
ebound verify converted.eb
ebound inspect converted.eb
ebound unpack converted.eb ./restored-zip
```

Encrypted crypto-v1 archives are `INDEXED` only. Recipient and identity files
use the small experimental `EBK1` local-key wrapper; this is local tooling and
is not part of the `.eb` wire format. The implemented key commands are limited
to authenticated listing, hybrid addition, fresh-key hybrid removal, and
fresh-key password rotation; they are not a general key-management system.

```sh
ebound pack ./example private.eb --recipient recipient.ebk
ebound inspect private.eb --crypto
ebound verify private.eb --identity identity.ebk
ebound unpack private.eb ./restored-private --identity identity.ebk

# Passwords are read from the controlling terminal and never from argv.
ebound pack ./example private-password.eb --password
ebound unpack private-password.eb ./restored-password --password
```

Signatures always bind CONTENT and default to every binding available:
CONTENT+PHYSICAL for unencrypted archives, plus ADDRESSING for authenticated
encrypted archives. Unencrypted archives use detached signatures because the current unencrypted ECF has no
normative embedded-signature placement. Encrypted archives may embed signatures
after successful unlock. Timestamp tokens are supplied externally and verified
offline; Entrybound does not contact a TSA.

```sh
ebound key generate-signing signer.key
ebound sign example.eb --signing-key signer.key --detached example.ebsig --bind-physical
ebound verify example.eb --signature example.ebsig --require-content-signature

ebound sign private.eb --signing-key signer.key --embed \
  --identity identity.ebk --bind-physical --bind-addressing
ebound verify private.eb --identity identity.ebk --signatures

ebound key list private.eb --identity identity.ebk
ebound key add private.eb --identity identity.ebk --recipient colleague.ebk
# Removal requires every retained public key and performs complete re-encryption.
ebound key remove private.eb --identity identity.ebk --retain colleague.ebk
ebound key change-password private-password.eb --password
```

Repeated `--recipient` options create one hybrid archive for multiple X-Wing
recipients. `--pad bucketed` is the default; `--pad none` deliberately leaks
exact protected-record sizes and is reported as a privacy warning, while
`--pad max` uses the frozen maximum record class. Encrypted creation uses a
file-key-derived secret Gear table by default; `--chunk-boundary keyed-prf`
selects the stronger PHTE/AES-128 mode. Encryption is intentionally
nondeterministic even when logical identity is unchanged.

Sequential workflows write and read the same archive model without seeking:

```sh
ebound pack ./example - --layout stream > example-stream.eb
ebound pack ./example example-stream.eb --layout stream --stream-window auto
ebound verify - < example-stream.eb
ebound list - < example-stream.eb
ebound inspect - < example-stream.eb
ebound unpack - ./restored-stream < example-stream.eb
ebound pack ./example - --layout stream | ebound verify -
```

An output of `-` writes archive bytes to standard output and defaults to
`--layout stream`; a regular file output defaults to `--layout indexed`. When
archive bytes go to standard output, every status line goes to standard error.
`--stream-window` declares how far a sequential reference may reach back to an
already emitted Chunk; the default of `0` refuses to create any cross-object
historical dependency, and `auto` accepts and declares whatever the sequential
organization requires. STREAM archives keep the `.eb` extension, carry no Index,
and cannot resolve one entry without a complete sequential pass.

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
It writes unencrypted Complete archives in INDEXED or STREAM layout and
encrypted Complete archives in INDEXED layout. It supports UTF-8 directory
names, directories, and regular files, and deliberately rejects symlinks and
special files. Strict ZIP import supports single-disk ZIP32/ZIP64 STORE and
DEFLATE, but there is no ZIP export, runtime-compatibility mode, preservation
mode, encrypted ZIP, tar/7z import, encrypted STREAM layout,
online TSA request support, general PKI/keychain integration, classical-only
recipients, remote range access, general
`repack`, lossy image recompression, guaranteed
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
and the bounded 256 MiB JPEG/JPEG XL reconstruction working set. Crypto policy
additionally caps recipient/envelope sizes, identity attempts, Argon2 work,
segment/message counts, ciphertext/private-record sizes, and aggregate crypto
working memory before attacker-controlled work. These are compatibility
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
random-access costs, [the STREAM layout note](docs/stream-layout-v1.md) for the
sequential wire shape, tagged items, dedup window, budget declaration, staging
extraction, access complexity, and identity equivalence with INDEXED,
[the crypto threat model](docs/crypto-threat-model-v1.md),
[suite freeze](docs/crypto-suite-v1.md),
[wire freeze](docs/crypto-wire-v1.md),
[security review](docs/crypto-review-v1.md), and
[crypto implementation note](docs/crypto-implementation-v1.md) for the
operational encrypted-INDEXED/signature subset, and the
[signing and key-management note](docs/signing-key-management-v1.md) for binding
freshness, timestamp trust, and recipient mutation architecture, and
[Legacy Observation Model note](docs/legacy-observation-model-v1.md) and
[strict ZIP import note](docs/zip-import-v1.md) for foreign evidence,
reconciliation, bomb limits, and auxiliary conversion provenance, and
[CONTRIBUTING.md](CONTRIBUTING.md) for development conventions.
