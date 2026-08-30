# Bootstrap ECF encoding (`ecf/bootstrap-v1`)

Status: experimental implementation note. This is the initial native subset,
not a frozen Entrybound v1 specification.

## Decisions

The bootstrap uses a small canonical binary encoding rather than a
runtime-defined serializer. Every canonical record has a 16-byte header:

```text
record_type:u16 | version:u16 | flags:u32 | payload_length:u64 | payload
```

Record payloads are ordered TLV fields:

```text
tag:u16 | type:u8 | flags:u8 | value_length:u64 | value
```

Tags are strictly increasing, flags and reserved bytes are zero, integers are
big-endian, booleans are exactly `00` or `01`, strings are UTF-8, and sequences
have an explicit 64-bit item count followed by length-delimited items. Maps are
not used. Readers reject non-minimal, out-of-order, duplicate, unknown, or
ill-typed fields. This makes the encoding deterministic, byte-inspectable,
unambiguous, stream-emittable, and implementable without language-specific
serialization defaults. Records and the namespace `ecf/bootstrap-v1` are
versioned so later format revisions can add records without changing these
bytes' interpretation.

All lengths, offsets, and counts are unsigned 64-bit values. Repeated values
are bounded before allocation.

## Container

The implemented bootstrap layout is `INDEXED`, role `Complete`, unencrypted,
with only directory and regular-file entries. It consists of:

1. a fixed 256-byte preamble beginning with `8E 45 42 31 0D 0A 1A 0A`;
2. checksum-protected authoritative sections plus an optional `INDEX`;
3. a fixed 128-byte footer with total length, actual totals, authoritative
   descriptor/manifest locators, and a preamble digest.

Section headers declare their payload length once and hash the payload. Chunk
frames declare stored length once; manifests do not repeat it. The Index only
caches chunk-frame locators and is ignored and rebuilt when absent or invalid.
No semantic Entry field appears in the Index.

Chunk frames carry the authoritative Chunk fields (`chunk_id`, `logical_len`,
`plan_ref`, and, when the extension requires it, `group_ref`) plus stored
bytes. ContentObjects carry ordered Chunk
references. Entry records carry the sole path/kind/content/metadata authority.

## Transform and chunking

`bootstrap-store-v1` remains a byte-preserving TransformPlan using `store/v1`.
Planner v1 may also record `zstandard/v1` plans whose identifiers are
`zstandard-v1-level-{level}-window-20`. Their closed 12-byte parameter value is
`"ZP01" | level:i32be | window_log:u8 | checksum:u8 |
content_size:u8 | dictionary_id:u8`. Planner v1 requires window log 20,
checksum 0, content size 1, dictionary ID 0, no dictionary, and no preceding
transforms. Readers reject any other parameter shape rather than consulting
codec defaults.

The Rust writer uses `zstd` 0.13.3 with optional default features disabled and
sets all represented encoder parameters explicitly. Zstandard plans declare a
1 MiB decoder window and 4 MiB working set. New filesystem archives use
versioned normalized CDC while historical archives retain `fixed-1mib/v1`.
Ordered Chunk references already represent arbitrary boundaries, so CDC and
deduplication require no wire change. Codec selection and chunking are physical
choices, not Entry semantics. See [planner-v1.md](planner-v1.md) and
[chunking-v1.md](chunking-v1.md). Planner v3 may record canonical
dictionary-backed `ZD01` or bounded-prefix `ZX01` parameter values while
retaining codec identifier `zstandard/v1`; the complete frozen construction is
documented in [cross-file-compression-v1.md](cross-file-compression-v1.md).
Required incompatibility feature bit `0x2` (`codec-transform-v1`) enables
first-class TransformStep records plus registered `lz4/v1` and `lzma2/v1`
plans. It does not alter frozen v1-v3 plan interpretation. See
[codec-transform-v1.md](codec-transform-v1.md).
Required incompatibility feature bit `0x4` (`reconstructive-transform-v1`)
requires bits `0x1` and `0x2`, selects TransformStep v2 records, and adds the
RECONSTRUCTION_DATA section. See
[reconstructive-transform-v1.md](reconstructive-transform-v1.md).

## Digests

The bootstrap format uses SHA-256 through the RustCrypto `sha2` crate. SHA-256
is universally specified, widely implemented, and suitable for independent
implementations. A tree-native digest would be attractive for a future version,
but SHA-256 keeps the first implementation small and auditable. The algorithm
name is domain-separated and bound into LAI, PCR, and AUX descriptors, so
migration is unambiguous.

All structured hashes use distinct ASCII domain labels and length-prefixed
fields. Merkle leaves and interior nodes are separately domain-separated; the
canonical tree splits at the largest power of two below the leaf count and is
never padded. PCI is SHA-256 over every exact container byte and is computed on
open; it is not embedded, avoiding a self-referential digest.

## Exact bootstrap framing

The 256-byte preamble has these fixed offsets. Unlisted bytes are reserved and
must be zero.

| Offset | Value |
|---:|---|
| 0 | 8-byte Entrybound magic |
| 8 | major `u16`, minor `u16`, preamble length `u32` |
| 16 | incompat, read-only-compatible, and compatible `u64` feature bitmaps |
| 40 | SHA-256 of the 24 feature-bitmap bytes |
| 72 | layout `u8`, role `u8`, budget-declared `bool`, reserved `u8` |
| 76 | aggregate decode window `u64`, working set `u64`, flags `u32` |
| 96 | eight ResourceBudget `u64` values in specification order |
| 160 | STREAM dedup window `u64` and hostility summary `u64`; both zero here |
| 176 | advisory footer-offset hint `u64` |

Each section starts with a 64-byte header:

```text
"EBS1" | section_type:u16 | version:u16 | flags:u32 | reserved:u32
payload_length:u64 | sha256(payload):[u8;32] | reserved:[u8;8]
```

Sections occur exactly once and in this order: DESCRIPTOR (1), TRANSFORM_PLANS
(2), CHUNK_DATA (3), MANIFEST_RECORDS (4), FIDELITY (5), and optionally INDEX
(6). Unknown, missing, duplicate, or reordered authoritative sections are not
canonical.

Required incompatibility feature bit `0x1` (`cross-file-compression-v1`)
selects the extended schema: DESCRIPTOR (1), TRANSFORM_PLANS (2), DICTIONARIES
(3), CHUNK_GROUPS (4), CHUNK_DATA (5), MANIFEST_RECORDS (6), FIDELITY (7), and
optionally INDEX (8). Both new authoritative sections occur exactly once even
when empty. The three currently recognized incompatibility bits are `0x1`,
`0x2`, and `0x4`; readers reject every other unknown required bit. `0x2` changes only the
TransformPlan field-3 item schema and may be combined with `0x1`, as v4 does.

With `0x4`, the canonical extended schema is DESCRIPTOR (1), TRANSFORM_PLANS
(2), DICTIONARIES (3), CHUNK_GROUPS (4), RECONSTRUCTION_DATA (5), CHUNK_DATA
(6), MANIFEST_RECORDS (7), FIDELITY (8), and optional INDEX (9). The new
authoritative physical section occurs exactly once, including when empty.

CHUNK_DATA is a sequence of digest-ordered plan-driven frames:

```text
"EBCH" | version:u16 | flags:u16 | stored_length:u64
chunk_id:[u8;32] | logical_length:u64 | plan_ref:u64 | stored_bytes
```

Under `cross-file-compression-v1`, version-2 frames append
`group_ref:[u8;32]` before stored bytes. Zero means no group. This is the sole
membership authority; ChunkGroup records do not contain member lists. Frame
order is the authority for preceding-Chunk dependencies.

The 128-byte footer begins with `8E 45 42 46 0D 0A 1A 0A`, then contains total
container length; absolute offset/length pairs for DESCRIPTOR and
MANIFEST_RECORDS; actual entry count and total logical bytes; SHA-256 of the
entire preamble; and 32 zero reserved bytes.

## Canonical records

Record type and strictly increasing field tags are:

| Record | Type | Fields |
|---|---:|---|
| Descriptor | 1 | namespace(1), identity profile(2), digest algorithm(3), planner ID(4), chunker ID(5), LAI(6), PCR(7), AUX(8) |
| TransformPlan | 2 | plan ID(1), identifier(2), transform sequence(3), codec(4), parameters(5), optional dictionary(6), decode window(7), working set(8), flags(9) |
| Entry | 3 | LogicalPath components(1), kind(2), ContentRef kind(3), optional logical digest(4), MetadataSet(5), identity digest(6), auxiliary digest(7) |
| ContentObject | 4 | logical digest(1), chunk root(2), ordered Chunk references(3) |
| FidelityReport | 5 | captured(1), unavailable(2), degraded(3), platform(4), filesystem declarations(5) |
| Index entry | 6 | Chunk digest(1), absolute frame offset(2), stored length(3) |
| PathComponent | 7 | encoding(1), bytes(2) |
| MetadataItem | 8 | name(1), criticality(2), restorability(3), boolean(4) or timestamp(5) |
| Timestamp | 9 | signed seconds(1), nanoseconds(2), source precision(3), restorable(4) |
| Fidelity issue | 10 | class(1), reason(2), optional entry scope(3) |
| Dictionary | 11 | dictionary digest(1), codec(2), format(3), construction(4), exact bytes(5) |
| ChunkGroup | 12 | group ID(1), maximum lookback Chunks(2), maximum preceding logical bytes(3) |
| TransformStep | 13 | transform identifier(1), canonical parameter bytes(2) |
| ReconstructionData | 14 | exact-byte digest(1), format/version(2), intermediate length(3), exact reconstruction bytes(4) |
| TransformStep v2 | 15 | transform identifier(1), canonical parameter bytes(2), optional reconstruction reference(3) |
| ReconstructionFallback | 16 | Chunk digest(1), fallback reason enum(2) |

Without `codec-transform-v1`, TransformPlan field 3 remains the historical
empty sequence. With the feature, every sequence item is a version-1 type-13
TransformStep record. Non-empty legacy string placeholders were never emitted
by frozen planners and are rejected rather than reinterpreted.
With `reconstructive-transform-v1`, every TransformPlan field-3 item is a
type-15 TransformStep v2 record. Type 13 is never reinterpreted. Structural
  steps omit field 3; the sole reconstructive step must be first and carries one
  digest reference to a type-14 ReconstructionData object.
  Type-16 records follow all type-14 records in the RECONSTRUCTION_DATA section,
  are ordered by Chunk digest, and are non-authoritative creation-audit data.
  Reason 1 means recognition or mandatory exact verification did not qualify;
  reason 2 means a verified candidate did not win the complete-cost comparison.

Sequences contain `count:u64`, then repeated `item_length:u64 | item`. Entries
are in canonical LogicalPath order. ContentObjects, Chunks, TransformPlans, and
Index entries are ordered by their identifiers. Set-like string and fidelity
lists are sorted and unique. The reader re-encodes authoritative records and
rejects any representation that is not byte-canonical.

## Hash construction

Chunk and ContentObject logical digests are plain SHA-256 over exact plaintext
bytes. Section digests, the feature checksum, preamble binding, and PCI are
plain SHA-256 over the exact bytes named by those fields.

A structured hash is:

```text
SHA256(
  "Entrybound hash v1\\0" ||
  u64be(domain_length) || domain ||
  u64be(field_count) ||
  each(u64be(field_length) || field)
)
```

The domains used are `chunk-tree/{leaf,empty,node}`,
`entry/{identity,aux}/v1`, `manifest/{leaf,empty,node}`, `lai/v1`, `pcr/v1`,
`aux-manifest/{leaf,empty,node}`, `fidelity/v1`, `conversion/absent/v1`, and
`aux/v1`.

- Entry identity binds identity profile, component bytes and encodings, kind,
  ContentRef kind, logical digest, and `core.executable`.
- Entry AUX binds every remaining implemented metadata item (`core.mtime`).
- LAI binds SHA-256, manifest root, `identity/v1`, Complete role, entry count,
  and total logical size.
- PCR binds SHA-256, the digest-ordered `(logical_digest, chunk_root)` list,
  unique physical Chunk count, and chunker ID. Transform plans are deliberately
  excluded so recompression with unchanged chunking preserves PCR.
- AUX binds SHA-256, the Entry-AUX Merkle root, FidelityReport digest, and the
  explicit absent-ConversionRecord digest.
- PCI is SHA-256 over every exact `.eb` byte and therefore changes when an Index
  is added, removed, or repaired even though LAI, PCR, and AUX do not.

## Index handling

Index entries contain only physical Chunk-frame locators. The reader always
rebuilds the authoritative locator map by scanning CHUNK_DATA. A present Index
is used only if its section digest, canonical encoding, and complete locator map
match the rebuilt map. Otherwise the reader reports
`EB_ECF_INDEX_INVALID_REBUILT`; an absent Index reports
`EB_ECF_INDEX_ABSENT_REBUILT`. Neither outcome changes EAM interpretation.
