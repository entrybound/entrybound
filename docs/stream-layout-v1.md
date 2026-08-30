# Native STREAM layout (`stream-layout-v1`)

Status: experimental implementation note for the second physical ECF layout. It
is not a frozen Entrybound v1 specification.

STREAM is a second physical organization of the same Entrybound Archive Model.
It writes to a sink that implements only `std::io::Write` and reads from a
source that implements only `std::io::Read`.

The central invariant is that **STREAM and INDEXED differ only in physical
organization and access capability. They never disagree about what an archive
means.** Encoding one validated EAM under both layouts yields identical LAI,
PCR, and AUX. Only PCI differs, because PCI is the digest of the exact
container bytes.

## Required incompatibility feature

Bit `0x10` (`stream-layout-v1`) is a required incompatibility feature. A reader
that does not implement it must refuse the archive rather than attempt to read
STREAM bytes as INDEXED sections. The preamble also declares layout
discriminant `2`, so a historical reader fails closed twice over: once on the
unknown required bit and once on the unknown layout.

The bit and the layout are declared together or not at all. A container that
declares one without the other is not canonical.

STREAM composes with the existing required bits and adds no ordering
constraints of its own beyond theirs. It does not require any of them.

STREAM archives keep the `.eb` extension. The layout is a property of the
container, not of its name.

## Preamble

STREAM uses the same fixed 256-byte preamble at the same offsets as INDEXED.
Three fields differ in how they are used:

| Offset | Field | STREAM |
|---:|---|---|
| 72 | layout | `2` |
| 160 | STREAM dedup window | the declared window (see below) |
| 176 | advisory footer-offset hint | must be zero |

The footer hint is zero because a sequential writer cannot know its own final
extent before emitting the body, and the STREAM footer is self-locating at EOF.
The hostility summary at offset 168 remains zero, exactly as in INDEXED.

Everything else — magic, version, the three feature bitmaps and their checksum,
archive role, `BudgetDeclared`, decode requirements, and the eight
`ResourceBudget` values — is declared identically in both layouts.

## Body shape

```text
preamble (256 bytes)
STREAM_BODY
  TRANSFORM_PLANS
  [DICTIONARIES] [CHUNK_GROUPS]            with cross-file-compression-v1
  [RECONSTRUCTION_DATA]                    with reconstructive-transform-v1
  [RECONSTRUCTION_REGIONS]                 with whole-object-reconstruction-v1
  ( CHUNK_FRAME* MANIFEST_RECORD )*        ContentObject records follow data
  MANIFEST_RECORD*                         Entry records
  FIDELITY
  DESCRIPTOR
footer (128 bytes, self-locating at EOF)
```

The body carries no length at its beginning. Its final extent is established by
the footer.

## Tagged items

Every body item begins with a fixed 16-byte header, so a reader never has to
guess whether the next bytes are physical content or a semantic record.

```text
"EBI1" | tag:u16 | version:u16 | flags:u32 | reserved:u32
```

`version` is `1`; `flags` and `reserved` are zero. The frozen tag values are:

| Tag | Item | Body | Occurs |
|---:|---|---|---|
| 1 | `TRANSFORM_PLANS` | record | exactly once, always |
| 2 | `DICTIONARIES` | record | exactly once with `0x1` |
| 3 | `CHUNK_GROUPS` | record | exactly once with `0x1` |
| 4 | `RECONSTRUCTION_DATA` | record | exactly once with `0x4` |
| 5 | `RECONSTRUCTION_REGIONS` | record | exactly once with `0x8` |
| 6 | `CHUNK_FRAME` | physical | once per unique Chunk |
| 7 | `MANIFEST_RECORD` | record | once per Entry and ContentObject |
| 8 | `FIDELITY` | record | exactly once |
| 9 | `DESCRIPTOR` | record | exactly once |

Unknown tags are refused, never skipped.

### Record items

A record item declares its own payload length and payload digest once:

```text
item header | payload_length:u64 | sha256(payload):[u8;32] | payload
```

Each record item's payload is byte-identical to the corresponding INDEXED
section payload, except `MANIFEST_RECORD`, which carries exactly one canonical
record — one type-3 Entry or one type-4 ContentObject. Concatenating a STREAM
body's `MANIFEST_RECORD` payloads in body order yields exactly the INDEXED
`MANIFEST_RECORDS` section payload.

### Chunk frame items

A `CHUNK_FRAME` item's body is an ordinary `EBCH` frame, byte-identical to the
frame the INDEXED writer would place in `CHUNK_DATA` for the same Chunk:

```text
item header | "EBCH" | version:u16 | flags:u16 | stored_length:u64
            | chunk_id:[u8;32] | logical_length:u64 | plan_ref:u64
            | [group_ref:[u8;32]] | stored_bytes
```

The frame's own `stored_length` is the sole authority for its extent. The item
does not repeat it, and no trailing record repeats it either. There is no
ZIP-style forward header plus trailing duplicate anywhere in the layout.

Frame version and flags follow the same feature-selected rules as INDEXED:
version 3 with `0x8`, version 2 with `0x1`, otherwise version 1; flag bit 0
marks a region-owned Chunk declaration with zero stored length.

## Ordering rules

1. Supporting items come first, in tag order, each exactly once, with the
   required set determined by the declared features.
2. `CHUNK_FRAME` and `MANIFEST_RECORD` items interleave. A ContentObject record
   may appear only after every Chunk frame it references.
3. Entry records follow every ContentObject record. No `CHUNK_FRAME` and no
   ContentObject record may follow the first Entry record.
4. `FIDELITY` follows the manifest records; `DESCRIPTOR` is the last item.
5. ContentObject records are uniquely ordered by logical digest; Entry records
   are in canonical LogicalPath order. Chunk frame identities are unique.

A reader enforces this order as it reads and refuses a body that violates it.
Chunk frames are **not** digest ordered in STREAM; the canonical STREAM order is
object-major, described next.

## Physical organization

INDEXED emits `physical_order` as the planner produced it and relies on the
Index or a `CHUNK_DATA` scan for lookup. STREAM has neither, so it selects its
own deterministic sequential organization:

- ContentObjects are visited in canonical logical-digest order.
- For each, its not-yet-emitted Chunks are emitted, then its Manifest record.
- A bounded-lookback ChunkGroup is emitted as one contiguous run at the point
  its first member is needed, preserving the group's relative order from
  `physical_order` and therefore its exact stored bytes.
- Any Chunk no ContentObject references is emitted after the last object record.

This is the "physical organization" the two layouts are permitted to differ in.
No semantic fact changes: the Chunk set, the ContentObject set, every Entry,
every declared bound, and all three logical identities are unchanged. The
bounded-lookback prefix a decoder reconstructs is derived from the same
preceding members in the same order, so prefix-coded Chunks decode to the same
plaintext from the same stored bytes.

## Stream dedup window

Exact deduplication is not weakened for STREAM. A Chunk still appears exactly
once, and a later ContentObject still references it by identity. What STREAM
adds is a declaration of how far back such a reference may reach, so a reader
can size its retained state before it starts.

Let `frame_index(c)` be a Chunk's frame ordinal and `S(o)` the number of frames
emitted before ContentObject `o`'s own run began. A reference from `o` to a
Chunk with `frame_index(c) < S(o)` is a **historical cross-object deduplication
reference** with distance `S(o) − frame_index(c)`.

`stream_dedup_window` is the largest such distance in the archive, and zero when
there are none. Objects that become ready at the same frame count form one run
and share a single `S(o)`.

A reader therefore retains the Chunks of the run it is currently reading plus at
most `window` older Chunks. Both parts are bounded: the run is bounded by the
declared `max_single_entry_logical_bytes` and `chunk_count`, and the historical
part by the declared window. Bounded-lookback group history is a separate,
smaller cache bounded by each group's own declared `max_lookback` and
`max_preceding_bytes`.

### Producer policy

```text
--stream-window <n>     refuse an organization that needs more than n
--stream-window auto    accept whatever the organization needs
```

The default for explicit STREAM packing is `0`, which refuses to create any
cross-object historical dependency. `--stream-window` is a **ceiling the
producer commits to**; the archive always declares the exact minimum its
organization requires, so `--stream-window 8` on an archive that needs 1
declares 1.

If the planned archive cannot satisfy the requested ceiling, packing fails with
`EB_ECF_STREAM_WINDOW_EXCEEDED` and names the window actually required. It never
silently raises the window, and it never rewrites the plan's semantics to fit.

Shared dictionaries do not by themselves create historical references, because a
dictionary is not a Chunk. Bounded-lookback ChunkGroups and files that share
content usually do, so those archives normally need `auto` or an explicit
non-zero ceiling.

### Reader enforcement

A reader tracks the same run boundaries and refuses any reference that reaches
further back than the declared window, with
`EB_ECF_STREAM_WINDOW_EXCEEDED`. A reference to a Chunk whose frame has not been
seen at all is `EB_ECF_STREAM_FORWARD_REFERENCE`. The reader validates the
declared window as an upper bound; it does not require the producer to have
declared the tightest possible value.

## Footer and descriptor placement

The fixed 128-byte trailer keeps the INDEXED footer magic and width and remains
self-locating: a reader that reaches EOF finds it in the last 128 bytes.

| Offset | Field |
|---:|---|
| 0 | footer magic `8E 45 42 46 0D 0A 1A 0A` |
| 8 | intended total container length |
| 16 | `DESCRIPTOR` item absolute offset |
| 24 | `DESCRIPTOR` item length |
| 32 | `STREAM_BODY` length |
| 40 | final actual unique Chunk count |
| 48 | final actual entry count |
| 56 | final actual total logical bytes |
| 64 | SHA-256 of the entire preamble |
| 96 | SHA-256 of the entire `STREAM_BODY` |

The descriptor is footer-located because the final identity roots and totals are
naturally known only at the end. It carries the same fields as the INDEXED
DESCRIPTOR section: namespace, identity profile, digest algorithm, planner ID,
chunker ID, LAI, PCR, and AUX.

Nothing already emitted is ever revisited. A STREAM writer is therefore valid
against a sink that implements `Write` and not `Seek`, which is exactly how the
implementation is typed.

### Truncation versus corruption

Truncation stays distinguishable from corruption:

- A source that ends inside the preamble, inside an item, or before the fixed
  footer returns `TRUNCATED EB_ECF_TRUNCATED_STREAM`.
- A complete footer whose declared total length exceeds the bytes actually read
  is `TRUNCATED`; a declared length below the bytes read, or bytes trailing a
  complete footer, is `CORRUPT EB_ECF_INCORRECT_TOTAL_LENGTH`.
- A byte pattern at an item boundary that is neither an item header nor the
  footer magic is `CORRUPT EB_ECF_STREAM_ITEM_ORDERING`.

## Supporting decoder data before use

A sequential reader must know how to decode a payload before it meets it, so
every dependency is declared ahead of the first item that needs it:
TransformPlans, Dictionaries, ChunkGroups, ReconstructionData, and
ReconstructionRegions all precede the first `CHUNK_FRAME`. Discovering a codec,
dictionary, group, or reconstruction method never requires seeking backwards.

Whole-object reconstruction regions are the one case that needs care. A region
declares its representation up front, but which Chunks it owns is derived from
its ContentObject's chunk range, and that record follows the data by design. The
reader therefore holds the region's representation and reconstructs its members
at the moment the ContentObject record arrives — the first point at which the
membership is known — and verifies every member Chunk against its declared
digest before staging it. The region remains one authoritative physical
representation, exactly as in INDEXED.

## Budget declaration

`BudgetDeclared = true` is the normal case. Directory packing has already
planned the whole archive, so it knows its totals and declares them. Such a
writer encodes its frames once to measure their stored lengths before emitting
the preamble; that costs memory proportional to the encoded archive but never a
seek.

`BudgetDeclared = false` is fully supported for producers that cannot measure
their output before emitting it. In that case:

- the preamble's eight `ResourceBudget` values are all zero, and a non-zero
  value there is not canonical;
- the caller's own policy alone bounds decoding during the pass, and it is
  enforced incrementally as entries, Chunks, and bytes arrive;
- the final actual totals are reported in the footer;
- `inspect` reports that the producer did not declare a pre-payload budget.

Absence of a declaration never means unlimited resources. There is no CLI option
for it, because no current producer needs one.

## Sequential writer

```rust
pub fn encode_stream<W: Write>(
    input: &Archive,
    options: StreamWriteOptions,
    sink: W,
) -> Result<StreamWriteSummary>
```

The writer validates the EAM first, never seeks, emits deterministic bytes,
enforces the requested window, computes the footer and the identity roots while
writing, and returns the final identities and statistics. `Seek` is absent from
the bound, so a writer that needed it could not compile.

The INDEXED encoder remains available unchanged as `encode`.

## Sequential reader

```rust
pub fn open_stream_with_limits<R: Read>(
    source: R,
    limits: SequentialLimits,
) -> Result<SequentialArchive>
```

The pass reads once, from front to back. It validates framing, enforces the
caller's resource policy as it goes, decodes and verifies each physical item on
arrival, tracks bounded dependencies, accumulates semantic manifest data,
recomputes the identity roots incrementally, and validates the descriptor and
footer at EOF. It does not read the archive into a `Vec<u8>` and call the
INDEXED reader, and it never holds the whole archive in memory.

`SequentialLimits` carries the caller's `ResourceBudget`, `DecodeRequirements`,
staging limits, a largest-single-item bound, a largest-total-container bound,
and the content policy:

| `StreamContentPolicy` | Retains |
|---|---|
| `Verify` | only the declared window; enough for verify, list, inspect |
| `Stage` | every Chunk, in bounded staging; used by extraction |
| `Retain` | every Chunk, placed back into the returned model; used by `explain` |

### Honest capability

`StreamAccessProfile` states what the source actually offers rather than
implying more:

- `random_entry_lookup: false` — always, for STREAM.
- `entry_cursor: Sequential`.
- `listing_requires_full_scan: true`.
- `source_replayable: false` — the pass consumes its source.

Random entry lookup is unavailable until the stream has been scanned or
repacked. There is no API that looks random-access while hiding an O(n) scan,
and STREAM is never reported as random-access merely because the source happened
to be a seekable file: the reader is chosen from the archive's declared layout.

### Access complexity

| Operation | INDEXED | STREAM |
|---|---|---|
| verify | O(n) | O(n), one pass |
| list | O(entries) after open | O(n) full scan |
| inspect | O(n) | O(n) full scan |
| one entry by path | O(1) locate, O(chunks) decode | O(n) scan |
| extract all | O(n) | O(n) plus staging |

## Staging and safe extraction

Manifest records follow the data they describe, so a sequential extractor learns
a Chunk's semantic destination only after the Chunk itself. Extraction therefore
stages decoded content internally and creates nothing under the caller's
destination until the archive has fully verified.

- Unverified data never becomes a final extracted file. Materialization begins
  only after framing, every Chunk digest, the EAM invariants, the identity
  roots, the footer binding, and the exact total length have all been
  established.
- Staging is bounded in memory. It keeps a resident working set up to
  `StagingLimits::memory_bytes` and spills the remainder to a private temporary
  file; `total_bytes` bounds the whole store. The archive plaintext is never
  required in RAM.
- The temporary file is the reader's own scratch space, created with
  `create_new`, never reachable through a caller-supplied path. Seeking inside a
  file this process created is unrelated to the archive's no-seek guarantee.
- Staging is released when the pass ends, including on failure and truncation.
- The destination stays capability-confined and existing objects are never
  overwritten: extraction uses the same policy as INDEXED and refuses
  collisions.

## Verification

`ebound verify` on a STREAM archive validates the preamble, the feature model,
tagged item framing and order, stored lengths, supporting-record integrity, the
stream dedup-window constraints, Dictionaries and ChunkGroups, reconstruction
dependencies and regions, plaintext Chunk digests, the EAM invariants, LAI, PCR,
AUX, the footer binding, the exact total length, and PCI.

## Identity equivalence with INDEXED

Encoding the same planned EAM under both layouts gives:

```text
INDEXED LAI == STREAM LAI
INDEXED AUX == STREAM AUX
INDEXED PCR == STREAM PCR      when chunk organization is unchanged
INDEXED PCI != STREAM PCI      the container bytes differ
```

LAI binds the Entry manifest root, entry count, and total logical size. PCR
binds the digest-ordered `(logical_digest, chunk_root)` list, the unique Chunk
count, and the chunker ID. AUX binds the Entry-AUX root, the FidelityReport, and
the explicit absent-ConversionRecord digest. None of them binds the layout, the
frame order, the Index, or the codec choice, which is why a physical
reorganization cannot move them.

Opening either encoding produces the same entries, ContentObjects, Chunks,
Dictionaries, ChunkGroups, ReconstructionData, and ReconstructionRegions, and
extracting either produces identical file bytes. `ContentStore::physical_order`
differs by design and is physical only.

## Limitations

- No encrypted STREAM layout. STREAM archives are unencrypted, as INDEXED
  archives are in this bootstrap.
- No random entry access, no partial extraction, and no repack command. A STREAM
  archive must be scanned to answer any per-entry question.
- The source is consumed by the pass. Answering a second question about a piped
  archive requires piping it again.
- `explain` needs retained plaintext, because it re-derives the physical
  alternatives the planner weighed. On a STREAM source it runs under the
  retaining content policy, whose memory profile matches opening the INDEXED
  encoding of the same archive.
- The writer takes an in-memory `Archive`, so its own memory is already
  proportional to the archive it is asked to encode. The format requires no
  seek; the current writer API is not itself an incremental producer.
- No cryptographic recipients or signatures, no remote range access, no legacy
  adapters, no new codecs or transforms, and no mounting.
