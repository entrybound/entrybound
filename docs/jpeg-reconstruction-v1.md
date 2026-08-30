# JPEG whole-object reconstruction (`jpeg-jxl-reconstruct/v1`)

Status: experimental frozen implementation note for the v6 planners.

## Dependency decision

Entrybound uses the pure-Rust `jixel` 0.2.19 encoder and `jxl-oxide` 0.12.6
decoder/reconstructor, both with default features disabled. `jixel` is licensed
BSD-3-Clause OR Apache-2.0; `jxl-oxide` is MIT OR Apache-2.0. No GPL component,
native library, subprocess, or external command is part of core decoding.
`jixel` raises the workspace MSRV to Rust 1.94. Frozen v6 encoding pins one
encoder thread and requests JPEG reconstruction data explicitly. `jxl-oxide`
uses an allocation tracker with a 256 MiB limit.

The dependency boundary is governed by the transform registry. Archive strings
can select only the registered `jpeg-jxl-reconstruct/v1` behavior. Its canonical
parameter bytes are:

```text
JJ01\0jixel-0.2.19\0jxl-oxide-0.12.6\0threads=1
```

## Whole-object model

A `ReconstructionRegion` is one physical representation of a contiguous range
of one authoritative ContentObject Chunk sequence:

```text
region_id
content_object
start_chunk_index
chunk_count
transform_plan
logical_bytes
transformed_bytes
ordinary_physical_bytes
region_overhead_bytes
declared access cost
encoded representation
```

Membership exists only as `content_object + start_chunk_index + chunk_count`.
There is no serialized member list. ContentObject Chunk order remains the only
logical order. Version-3 CHUNK_DATA frames retain each original Chunk digest and
logical length, set the region-owned flag, and contain no independent payload or
plan. The reader reconstructs the complete region, slices it using those
authoritative logical lengths, and verifies every original SHA-256 Chunk digest.

Regions cannot overlap. A Chunk cannot have both an independent physical
payload and region ownership. Multiple Entries may refer to the same
ContentObject without conflict. V6 conservatively rejects a region when any
member Chunk is also referenced by a distinct ContentObject, or when a member
already needs a Dictionary or ChunkGroup. This preserves one authoritative
physical representation for every exact plaintext Chunk.

## Recognition and mandatory verification

Recognition is byte-based. The supplied complete ContentObject must have a
bounded conventional JPEG marker structure, dimensions no greater than
100,000,000 pixels, and no more than 64 MiB of original bytes. Entrybound does
not inspect the filename extension, decode pixels, or re-encode JPEG from pixel
samples.

Every candidate executes:

```text
original JPEG
  -> jixel lossless JPEG-aware JPEG XL representation
  -> jxl-oxide JPEG bitstream reconstruction
  -> recreated JPEG
```

Both SHA-256 equality and direct byte equality are mandatory. Failure at either
check rejects the candidate before planning, and the ECF writer repeats the
complete inverse/equality check before committing container bytes. There is no
opt-out. The JPEG XL representation is self-contained, so the TransformStep has
no ReconstructionData reference and no redundant side-data object is emitted.

The tested selected subset includes deterministic baseline sequential JPEGs.
APP/COM/EXIF/ICC marker arrangements are attempted when the pinned libraries
accept them and are eligible only if the mandatory exact check succeeds. The
generated progressive corpus uses the pure-Rust, MIT-or-Apache-2.0
`jpeg-encoder` crate only as test tooling. A known valid progressive producer
output exposes an internal failure in the pinned JPEG/JPEG XL forward stack,
so v1 rejects progressive SOF markers before entering that dependency and
falls back safely. Progressive JPEG is therefore not part of the frozen v1
selected subset. Dependency calls are additionally unwind-confined. Some
otherwise legal producer/marker combinations are likewise rejected by v1 and
fall back to ordinary v5 physical encoding. Malformed, truncated, or unsupported
inputs also fall back; Entrybound never weakens exactness.

Reconstruction is ContentObject-bounded. V6 does not scan nested JPEG streams or
span more than one ContentObject.

## Frozen v6 profiles

All v6 profiles retain their corresponding frozen v5 CDC, exact-dedup,
independent codec/transform, similarity, Dictionary, and ChunkGroup behavior.
The JPEG-specific ordered post-transform codec candidates are:

| Planner | Region eligibility | JPEG XL post-codec candidates |
|---|---|---|
| `fast-v6` | disabled | none |
| `balanced-v6` | exactly one logical Chunk; lookback remains zero | STORE, LZ4, Zstandard levels 3 and 5 |
| `dense-v6` | complete eligible ContentObject, at most 4096 Chunks | STORE, Zstandard 9/15, LZMA2 preset 6 with 4 MiB dictionary |
| `extreme-v6` | complete eligible ContentObject, at most 4096 Chunks | STORE, LZ4, Zstandard 15/19, LZMA2 preset 6/4 MiB and preset 9/8 MiB |

Candidate ordering breaks equal complete costs in favor of the earlier/simpler
representation. `balanced-v6` cannot select a multi-Chunk region, preserving
independent-Chunk access by contract. Dense and extreme may select a whole-file
region, but their exact access cost is declared.

## Complete-cost rule

The best post-codec representation is charged for its stored bytes, canonical
TransformPlan v3 bytes when newly required, its ReconstructionRegion record,
and the region section header when first required. Existing Chunk frame and
Index costs are common to both alternatives because original Chunk declarations
remain present; the planner cancels those equal terms rather than hiding them.
The comparison baseline is the actual ordinary v5 payload set for the unique
member Chunks. A region must save strictly more than both 256 bytes and 2% of
that baseline. The ordinary representation wins ties.

Persisted audit records target a Chunk, ContentObject, or Region explicitly.
V6 records one of: not recognized, implementation unsupported, exact
verification failed, complete cost did not win, dedup/physical conflict, or
resource-policy exclusion. Audits are physical planning evidence, never EAM
semantic authority.

## Access and resource bounds

Every region declares logical bytes, logical Chunk count, and the worst bytes
that must be reconstructed to access one member. V1 reconstructs the complete
region, so worst reconstructed bytes equal region logical bytes. `inspect`
reports the largest region, worst bytes/Chunks, and whether all content remains
independently Chunk-decodable.

Frozen implementation limits are:

- original/reconstructed JPEG: 64 MiB;
- JPEG XL representation and transformed intermediate: 64 MiB;
- region membership: 4096 logical Chunks;
- image dimensions: 100,000,000 pixels;
- reconstructed-to-stored expansion: 64:1;
- JPEG/JPEG XL working set: 256 MiB;
- CLI aggregate decoder working-set policy: 384 MiB.

Sizes and caller decode policy are checked before expensive decoding where
available. The JXL allocation tracker and bounded reconstruction writer enforce
the implementation limits during attacker-facing parsing.

## Format evolution

Required incompatibility feature bit `0x8`
(`whole-object-reconstruction-v1`) requires bits `0x1`, `0x2`, and `0x4`. It
selects TransformStep v3 records, adds the authoritative
RECONSTRUCTION_REGIONS section, and selects CHUNK_DATA frame version 3.
Historical v1-v5 records are not reinterpreted. Readers that do not implement
bit `0x8` fail closed.

The new canonical record types are:

- type 17: TransformStep v3, fields transform identifier (1), parameters (2),
  optional ReconstructionData reference (3);
- type 18: ReconstructionRegion, fields listed in the whole-object model in
  order (1-13);
- type 19: ReconstructionAudit v2, target kind (1), target digest (2),
  transform identifier (3), reason (4).

The v6 extended section order is DESCRIPTOR, TRANSFORM_PLANS, DICTIONARIES,
CHUNK_GROUPS, RECONSTRUCTION_DATA, RECONSTRUCTION_REGIONS, CHUNK_DATA,
MANIFEST_RECORDS, FIDELITY, and optional INDEX.

## Identity

JPEG XL bytes, region identity/configuration, access declarations, and audits
are physical facts. Original Chunk digests, ContentObject logical digest, LAI,
and AUX are unchanged. PCR is unchanged when the logical Chunk sequence is
unchanged under the current PCR definition. PCI changes whenever exact
container bytes change. A region is not verified until every reconstructed
original member Chunk digest succeeds.
