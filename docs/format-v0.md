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

Two physical layouts are implemented, both role `Complete`, unencrypted, and
with only directory and regular-file entries. They encode the same EAM: LAI,
PCR, and AUX are identical across layouts and only PCI differs.

`INDEXED` is the random-access layout and consists of:

1. a fixed 256-byte preamble beginning with `8E 45 42 31 0D 0A 1A 0A`;
2. checksum-protected authoritative sections plus an optional `INDEX`;
3. a fixed 128-byte footer with total length, actual totals, authoritative
   descriptor/manifest locators, and a preamble digest.

Section headers declare their payload length once and hash the payload. Chunk
frames declare stored length once; manifests do not repeat it. The Index only
caches chunk-frame locators and is ignored and rebuilt when absent or invalid.
No semantic Entry field appears in the Index.

`STREAM` is the sequential layout selected by required incompatibility feature
bit `0x10` (`stream-layout-v1`) and layout discriminant `2`. It replaces the
section sequence with a single tagged `STREAM_BODY`, carries no Index, is
written without `Seek` and read without `Seek`, and declares a
`stream_dedup_window` bounding how far a sequential reference may depend on an
already emitted unique Chunk. Its Chunk frames are byte-identical to INDEXED
frames; its manifest records are the same canonical records, emitted
individually after the physical data they describe. The complete construction is
documented in [stream-layout-v1.md](stream-layout-v1.md).

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
Required incompatibility feature bit `0x8`
(`whole-object-reconstruction-v1`) requires bits `0x1`, `0x2`, and `0x4`,
selects TransformStep v3 and Chunk frame v3, and adds the canonical
RECONSTRUCTION_REGIONS section. See
[jpeg-reconstruction-v1.md](jpeg-reconstruction-v1.md).
Required incompatibility feature bit `0x10` (`stream-layout-v1`) selects the
sequential STREAM layout. It requires no other bit and changes no record schema;
it changes only the container's physical organization and access capability. It
is declared exactly when the preamble's layout discriminant is `2`. See
[stream-layout-v1.md](stream-layout-v1.md).

Cryptographic architecture is frozen but deliberately not implemented. The
reserved required incompatibility bits are `0x20` (`encrypted-indexed-v1`),
`0x40` (`payload-suite-v1`), `0x80` (`recipient-xwing-v1`), `0x100`
(`recipient-password-v1`), `0x200` (`signature-ed25519-v1`), `0x400`
(`crypto-padding-v1`), and `0x800` (`keyed-boundary-phte-v1`). Current readers
continue to reject them as unsupported. Their future canonical records,
feature constraints, and footer v2 are frozen in
[crypto-wire-v1.md](crypto-wire-v1.md); primitive and security rules are in
[crypto-suite-v1.md](crypto-suite-v1.md).

The crypto-v1 wire correction reserves two self-identifying grammars inside
authenticated encrypted objects: `EBPO` version 1 dispatches one canonical
record, Chunk frame, or sequence payload; `EBCS` version 1 carries one of nine
explicitly typed private collections. Crypto record type 22 is
`RecipientDirectoryEntryV1` and type 27 is `EncryptedIndexEntryV1`. These
assignments remain unimplemented and therefore unsupported by current readers;
their exact bytes, limits, and collection ordering are normative only in
[crypto-wire-v1.md](crypto-wire-v1.md).

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
| 72 | layout `u8` (`1` INDEXED, `2` STREAM), role `u8`, budget-declared `bool`, reserved `u8` |
| 76 | aggregate decode window `u64`, working set `u64`, flags `u32` |
| 96 | eight ResourceBudget `u64` values in specification order |
| 160 | STREAM dedup window `u64`; zero in INDEXED |
| 168 | hostility summary `u64`; always zero in this version |
| 176 | advisory footer-offset hint `u64`; zero in STREAM |

Each section starts with a 64-byte header:

```text
"EBS1" | section_type:u16 | version:u16 | flags:u32 | reserved:u32
payload_length:u64 | sha256(payload):[u8;32] | reserved:[u8;8]
```

Sections occur exactly once and in this order: DESCRIPTOR (1), TRANSFORM_PLANS
(2), CHUNK_DATA (3), MANIFEST_RECORDS (4), FIDELITY (5), and optionally INDEX
(6). Unknown, missing, duplicate, or reordered authoritative sections are not
canonical. Sections exist only in INDEXED layout; STREAM replaces them with
tagged items.

Required incompatibility feature bit `0x1` (`cross-file-compression-v1`)
selects the extended schema: DESCRIPTOR (1), TRANSFORM_PLANS (2), DICTIONARIES
(3), CHUNK_GROUPS (4), CHUNK_DATA (5), MANIFEST_RECORDS (6), FIDELITY (7), and
optionally INDEX (8). Both new authoritative sections occur exactly once even
when empty. The four currently recognized incompatibility bits are `0x1`,
`0x2`, `0x4`, `0x8`, and `0x10`; readers reject every other unknown required
bit, including the reserved but unimplemented crypto bits above. `0x2` changes only the
TransformPlan field-3 item schema and may be combined with `0x1`, as v4 does.

With `0x4`, the canonical extended schema is DESCRIPTOR (1), TRANSFORM_PLANS
(2), DICTIONARIES (3), CHUNK_GROUPS (4), RECONSTRUCTION_DATA (5), CHUNK_DATA
(6), MANIFEST_RECORDS (7), FIDELITY (8), and optional INDEX (9). The new
authoritative physical section occurs exactly once, including when empty.

With `0x8`, the canonical extended schema is DESCRIPTOR (1), TRANSFORM_PLANS
(2), DICTIONARIES (3), CHUNK_GROUPS (4), RECONSTRUCTION_DATA (5),
RECONSTRUCTION_REGIONS (6), CHUNK_DATA (7), MANIFEST_RECORDS (8), FIDELITY
(9), and optional INDEX (10). Both reconstruction sections occur exactly once,
including when empty.

CHUNK_DATA is a sequence of digest-ordered plan-driven frames:

```text
"EBCH" | version:u16 | flags:u16 | stored_length:u64
chunk_id:[u8;32] | logical_length:u64 | plan_ref:u64 | stored_bytes
```

Under `cross-file-compression-v1`, version-2 frames append
`group_ref:[u8;32]` before stored bytes. Zero means no group. This is the sole
membership authority; ChunkGroup records do not contain member lists. Frame
order is the authority for preceding-Chunk dependencies.

Under `whole-object-reconstruction-v1`, frame version is 3. Flag bit 0 marks a
region-owned Chunk declaration. Such a frame has zero stored length, plan ref
zero, and no group; its digest and logical length remain the authoritative
original Chunk declaration. All other flag bits are reserved and zero.

The 128-byte INDEXED footer begins with `8E 45 42 46 0D 0A 1A 0A`, then contains
total container length; absolute offset/length pairs for DESCRIPTOR and
MANIFEST_RECORDS; actual entry count and total logical bytes; SHA-256 of the
entire preamble; and 32 zero reserved bytes. The STREAM footer keeps the same
magic and width but declares the DESCRIPTOR item locator, the `STREAM_BODY`
extent, final actual Chunk, entry, and logical-byte totals, the preamble digest,
and the `STREAM_BODY` digest; see
[stream-layout-v1.md](stream-layout-v1.md).

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
| TransformStep v3 | 17 | transform identifier(1), canonical parameters(2), optional ReconstructionData reference(3) |
| ReconstructionRegion | 18 | region identity(1), ContentObject(2), start Chunk index(3), Chunk count(4), plan ref(5), logical bytes(6), transformed bytes(7), access logical bytes(8), access Chunks(9), worst reconstructed bytes(10), encoded representation(11), ordinary physical bytes(12), region overhead bytes(13) |
| ReconstructionAudit v2 | 19 | target kind(1), target digest(2), transform identifier(3), reason(4) |

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
With `whole-object-reconstruction-v1`, TransformPlan sequences contain type-17
steps. A self-contained reconstructive step omits field 3. Type-18 region
records precede type-19 audits in RECONSTRUCTION_REGIONS; both sets are
canonical and uniquely ordered. Region membership is not repeated as a list.

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

## Layout equivalence

A validated EAM encoded under both layouts produces identical LAI, PCR, and AUX
and different PCI. None of the three logical identities binds the layout, the
Chunk frame order, the Index, or the codec choice, so a physical reorganization
cannot move them. STREAM selects an object-major frame order and therefore has a
different `ContentStore::physical_order` than the INDEXED encoding of the same
model; that field is physical only and participates in no identity.

## Frozen cryptographic extension (not implemented)

Crypto v1 is INDEXED-only and uses one non-negotiated PayloadSuite:
AES-256-GCM-SIV, HKDF-SHA-256, HMAC-SHA-256, and SHA-256. A random 32-byte
archive file key feeds a labeled key hierarchy. A separate HMAC commitment is
verified before a candidate file key is accepted. Record/segment associated
data, mandatory segment endings, and an encrypted terminal archive-final record
bind order, truncation, identities, recipient set, and the fixed footer.

Normal public-key recipients use the draft-10 X-Wing construction
(ML-KEM-768 + X25519); password-only archives use Argon2id and cannot mix with
hybrid recipients. Names, metadata, manifest, Index, identities, decode budgets,
and embedded signatures are encrypted. Only format/crypto discovery, the
CryptoEnvelope, recipient method framing, padded ciphertext framing, and the
fixed footer remain public. Encrypted STREAM is deferred and must fail closed.

Default encrypted creation uses authenticated quarter-octave record padding and
a secret-derived Gear table as defense in depth. A separately declared PHTE +
AES-128 keyed-boundary mode supplies the stronger published keyed-CDC
construction. Neither mode claims to hide total archive size or access patterns.
Exact dedup remains plaintext-SHA-256 equality within one archive/file-key
domain only; convergent or cross-tenant encryption is forbidden.

Ed25519 signatures preserve independent content (`LAI`, `AUX`, identity
profile, format), physical (`PCR`), and addressing (suite, recipient-set digest,
commitment, archive ID) bindings. The precise transcript encodings, feature
assignments, limits, vectors, and reason codes are normative in
[crypto-wire-v1.md](crypto-wire-v1.md). The threat model and review record are
[crypto-threat-model-v1.md](crypto-threat-model-v1.md) and
[crypto-review-v1.md](crypto-review-v1.md). Reserving these bytes does not make
the current Rust implementation cryptographically capable.

## Index handling

The Index exists only in INDEXED layout. STREAM has none by design, and readers
report it as not applicable rather than absent or invalid.

Index entries contain only physical Chunk-frame locators. The reader always
rebuilds the authoritative locator map by scanning CHUNK_DATA. A present Index
is used only if its section digest, canonical encoding, and complete locator map
match the rebuilt map. Otherwise the reader reports
`EB_ECF_INDEX_INVALID_REBUILT`; an absent Index reports
`EB_ECF_INDEX_ABSENT_REBUILT`. Neither outcome changes EAM interpretation.
