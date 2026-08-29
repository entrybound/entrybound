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

The planned bootstrap layout is `INDEXED`, role `Complete`, unencrypted, with only
directory and regular-file entries. It consists of:

1. a fixed 256-byte preamble beginning with `8E 45 42 31 0D 0A 1A 0A`;
2. checksum-protected `DESCRIPTOR`, `TRANSFORM_PLANS`, `CHUNK_DATA`,
   `MANIFEST_RECORDS`, `FIDELITY`, and optional `INDEX` sections;
3. a fixed 128-byte footer with total length, actual totals, authoritative
   descriptor/manifest locators, and a preamble digest.

Section headers declare their payload length once and hash the payload. Chunk
frames declare stored length once; manifests do not repeat it. The Index only
caches chunk-frame locators and is ignored and rebuilt when absent or invalid.
No semantic Entry field appears in the Index.

Chunk frames carry the authoritative Chunk fields (`chunk_id`, `logical_len`,
and `plan_ref`) plus stored bytes. ContentObjects carry ordered Chunk
references. Entry records carry the sole path/kind/content/metadata authority.

## Transform and chunking

`bootstrap-store-v1` will be a real TransformPlan record using the registered
local `store/v1` codec and no transforms. The deterministic bootstrap chunker
will use fixed 1 MiB plaintext chunks. These are implementation choices for the
first vertical slice, not future format doctrine.

## Digests

The bootstrap format uses SHA-256. It is universally specified, widely
implemented, dependency-free in the minimal reader, and suitable for
independent implementations. A tree-native digest would be attractive for a
future version, but SHA-256 keeps the planned first security-sensitive
implementation small and auditable. The algorithm name is domain-separated and
bound into LAI, PCR, and AUX descriptors, so migration is unambiguous.

All structured hashes use distinct ASCII domain labels and length-prefixed
fields. Merkle leaves and interior nodes will be separately domain-separated;
the canonical tree splits at the largest power of two below the leaf count and
is never padded. PCI is SHA-256 over every exact container byte and will be
computed on open; it is not embedded, avoiding a self-referential digest.
