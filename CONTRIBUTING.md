# Contributing to Entrybound

Entrybound is at the experimental format-bootstrap stage. Changes to native
semantics or wire encoding must be checked against the Product Architecture
Specification and accompanied by focused conformance tests.

## Development

Rust is Entrybound's primary implementation language. The stable
toolchain is declared in `rust-toolchain.toml` and includes rustfmt and Clippy.
The workspace MSRV is Rust 1.94, required by the pinned pure-Rust JPEG/JPEG XL
transcode dependency.

```sh
cargo fmt --all --check
cargo check -p entrybound -p entrybound-cli
cargo clippy -p entrybound -p entrybound-cli --all-targets -- -D warnings
cargo test -p entrybound
cargo run -p entrybound-cli --bin ebound -- --help
```

The core crate uses RustCrypto's `sha2` for SHA-256, `cap-std` for
capability-relative filesystem traversal, and `zstd` 0.13.3 for the operational
Zstandard codec. The `zstd` crate is MIT licensed; its bindings use the
redistributable upstream Zstandard implementation, and default optional
features are disabled. Additional dependencies require a concrete format or
implementation need. Keep the CLI crate thin: semantic rules belong in
`entrybound::eam`, format rules in `entrybound::ecf`, identity rules in
`entrybound::identity`, creation-time codec selection in
`entrybound::planner`, operational codec dispatch below ECF, and filesystem
workflows and caller-owned resource policy in `entrybound::archive`.
Deterministic similarity feature extraction belongs in
`entrybound::similarity`; Dictionary/ChunkGroup candidate selection remains in
the creation-only planner.

The primary executable is `ebound`. The `entrybound` executable is a thin
compatibility alias; both call the shared implementation in the
`entrybound-cli` package library. Do not duplicate command logic between them.

Filesystem tests must use generated trees. Extraction changes need collision,
containment, preflight-integrity, and resource-policy tests; do not add opaque
archive fixtures or path-string shortcuts.

Compression planner behavior is frozen by planner ID. Do not change the
candidate levels, probe, cost rule, or parameter encoding of a `*-v1` policy;
introduce a new planner version. Decoder behavior must depend only on recorded
TransformPlans, never on the creation profile.

Chunker behavior is likewise frozen by `chunker_id`. `gear-norm-v1` includes
its table-generation formula, boundary masks, and min/target/max parameters.
Never change those in place. Creation-time chunking belongs in
`entrybound::chunker`; EAM and ECF consume only resulting ordered Chunk
references. Exact dedup equality is the plaintext SHA-256 Chunk identity.

Planner v3 also freezes `bottom-k-shingle-v1`, per-profile cohort bounds,
dictionary sample ordering/caps, trainer construction identifiers, dictionary
and lookback candidate levels, the strict cohort gain rule, and bounded access
calculation. Changing any of those requires a new planner/similarity version.
ChunkGroup membership authority belongs only on `Chunk.group_ref`; never add a
duplicated authoritative member list.

Planner v4 additionally freezes the closed codec/transform registries, codec
parameters, structural-transform definitions, eligibility probe, complete-cost
rule, and ordered candidate sets in `docs/codec-transform-v1.md`.

Planner v5 freezes the Structural/Reconstructive registry classes, preflate
version and bounds, DEFLATE wrapper eligibility, mandatory dual round-trip
check, ReconstructionData encoding, complete-cost margin, and candidate sets
in `docs/reconstructive-transform-v1.md`.

Planner v6 freezes the whole-ContentObject ReconstructionRegion model, the
`jixel`/`jxl-oxide` versions and single-thread encoder setting, conservative
dedup eligibility, mandatory exact round-trip check, access declarations,
complete-cost margin, and candidate sets in `docs/jpeg-reconstruction-v1.md`.

`stream-layout-v1` freezes the tagged `STREAM_BODY` item tags, their canonical
order, the record-item and Chunk-frame framing, the STREAM footer field layout,
and the stream dedup-window definition in `docs/stream-layout-v1.md`. Changing
any of them requires a new layout feature bit, not an edit in place.

Layout is a physical and access-capability choice only. A change that would make
INDEXED and STREAM disagree about entries, ContentObjects, Chunks, declared
bounds, LAI, PCR, or AUX is a bug in the change, not a property of the layout;
add the equivalence assertion to `tests/stream_layout.rs` rather than relaxing
it. Only PCI and `ContentStore::physical_order` may differ between layouts.

Keep single-authority framing. A Chunk frame's own `stored_length` is the sole
declaration of its extent; never add a second length to the enclosing item or a
trailing record. Semantic facts are declared exactly once, and STREAM has no
Index to reconcile.

A sequential writer must be usable with a sink that implements `Write` and not
`Seek`, and a sequential reader with a source that implements `Read` and not
`Seek`. Keep those bounds narrow so the type system, not a comment, enforces
the guarantee. Never make a sequential reader read its whole source into memory
and delegate to the INDEXED reader, and never expose an API that looks
random-access while hiding a full scan.

Sequential extraction stages decoded content in bounded, spilling storage and
materializes only after the whole archive has verified. Do not write unverified
content into a caller's destination, and do not require the archive plaintext in
RAM.

Crypto v1 is governed by `docs/crypto-suite-v1.md` and
`docs/crypto-wire-v1.md`. Do not alter T1 fields, HKDF labels, recipient
wrapping, segment nonces, associated data, padding buckets, boundary
derivations, or feature/record assignments to fit an implementation. The
`entrybound::crypto` module owns this closed suite and uses pinned RustCrypto,
X-Wing, Argon2, and zeroization dependencies; it must not gain locally written
cryptographic primitives or user-selectable algorithms. Permanent RFC,
draft-10, V1-V7, and RFC 8032 vectors are the external-conformance gate.

Signature records are canonical type 26 and frozen by the three independent
CONTENT/PHYSICAL/ADDRESSING transcripts. Never substitute PCI for PCR or merge
cryptographic validity with binding freshness. Embedded signatures belong only
to encrypted EBCS kind 7; unencrypted archives use exact detached `.ebsig`
records. Timestamp verification is offline, SHA-256/Ed25519-only, bounded to
64 KiB DER, and requires caller-provided trust anchors.

Adding a recipient must preserve AFK/archive ID and unchanged PAYLOAD ciphertext
where ordinal-compatible; it makes ADDRESSING signatures stale without touching
CONTENT/PHYSICAL. Removing a recipient or changing a password is never a stanza
edit: it rotates AFK/archive ID, re-runs keyed chunking/planning, and self-verifies
a complete replacement before filesystem replacement. Never silently delete or
resign stale historical signatures.

Canonical Descriptor v1 is permanently type 1/version 1 with tags 1-8.
Corrected encrypted INDEXED archives alone use Descriptor type 1/version 2,
required tags 1-19, and incompatibility feature `0x1000`. Do not add a second
resource/decode record or expose Descriptor-v2 values through public crypto
framing. The reader must authenticate and caller-check Descriptor v2 before
dependent private objects, then independently derive decoder requirements and
actual archive resource usage. Legacy encrypted Descriptor-v1 input is
compatibility-only and must remain explicitly labeled as lacking a producer
declaration; writers may not emit it.

Never print or derive `Debug`/`Display` for AFKs, identity seeds, method
secrets, passwords, or derived keys. Check attacker-controlled KDF, stanza,
segment, record, and allocation bounds before expensive work. No plaintext
from an encrypted archive may reach the filesystem until the full archive has
authenticated and passed ordinary EAM/identity verification. Encrypted STREAM
must fail before any output is emitted.

Generated conformance inputs belong in tests as format-building code, not as
opaque binary fixtures. Each negative case must assert a stable reason code.

Legacy adapters belong under `entrybound::legacy` and emit format-neutral LOM
evidence before any EAM projection. Never make a central directory, runtime
quirk, tar header, or other foreign declaration directly authoritative EAM
state. Parser authorities remain independent through reconciliation; strict
mode refuses Divergence and Irreconcilable evidence. Automatic Omission and
Refinement decisions must be canonical conversion provenance, contribute only
to AUX, and must not change LAI/PCR. ZIP tests build bytes programmatically and
must enforce caller-owned compressed/uncompressed/expansion limits.

ZIP strict, compatibility, and preservation policies must all consume the same
immutable `ZipObservation`. A compatibility ID always includes the exact probed
runtime version; behavior changes require a new ID and regenerated checked
matrix. Reference Python/Java/libarchive programs belong only in
`tools/zip-compat`, never in production. Compatibility never bypasses path,
extent, collision, special-file, or resource safety. Preservation feature
`0x4000` requires conversion feature `0x2000`, keeps type 28 unchanged, and
stores exact source plus ordered LOM evidence in canonical types 30-36. It may
change AUX/PCI, never LAI/PCR for the same EAM projection.
