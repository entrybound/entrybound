# Cross-file compression intelligence v1

Status: frozen creation-time behavior for planner v3. Readers decode only the
Dictionary, TransformPlan, ChunkGroup, and physical Chunk order recorded in an
archive; they never run similarity analysis or the planner.

## Similarity fingerprint and cohorts

Similarity runs after CDC and exact SHA-256 deduplication, over unique plaintext
Chunks in digest order. `bottom-k-shingle-v1` hashes overlapping 32-byte
shingles at a 16-byte stride with FNV-1a-64 using offset
`cbf29ce484222325` and prime `00000100000001b3`. At most 4,096 shingles per
Chunk are scanned. The lexicographically lowest unique hash values form the
fixed-size sketch. All arithmetic and comparisons are integer-only.

Chunks are assigned deterministically to a bounded leader cohort when their
sketch intersection reaches the profile threshold. Candidate leaders are found
through sketch-value buckets; the greatest intersection wins and cohort digest
breaks ties. SHA-256, not a similarity fingerprint, remains the sole authority
for exact equality. Similarity is neither stored as identity nor needed to
decode.

| Planner | Sketch / threshold | Cohort min / max | Candidate cap |
|---|---|---|---|
| `fast-v3` | disabled | none | 0 |
| `balanced-v3` | 8 / 500 per thousand | 8 / 32 | 64 |
| `dense-v3` | 16 / 375 per thousand | 4 / 64 | 128 |
| `extreme-v3` | 32 / 250 per thousand | 3 / 128 | 256 |

The v3 profiles retain exactly the v2 CDC candidate policies and the frozen v1
independent STORE/Zstandard selection behavior. They add these cohort searches:

| Planner | Dictionary levels | Training cap | Lookback candidates / levels |
|---|---|---|---|
| `fast-v3` | none | none | none |
| `balanced-v3` | 3, 5 | 16 samples, 16 KiB each, 8 KiB dictionary | none |
| `dense-v3` | 5, 9, 15 | 32 samples, 32 KiB each, 16 KiB dictionary | 1, 2, 4 / levels 5, 9 |
| `extreme-v3` | 9, 15, 19 | 64 samples, 64 KiB each, 32 KiB dictionary | 1, 2, 4, 8 / levels 9, 15, 19 |

`balanced-v3` therefore has lookback zero by contract and every Chunk is
independently decodable given its optional Dictionary.

## Shared Dictionaries

A Dictionary is a first-class physical object with SHA-256 identity over its
exact bytes, codec `zstandard/v1`, format `zstd-trained/v1`, and a frozen
construction identifier. Samples are selected in Chunk-digest order and
truncated to the profile cap before training through pinned `zstd` 0.13.3
(upstream Zstandard 1.5.7). Dictionary bytes are stored once and referenced by
TransformPlans; they are never duplicated in a plan.

For every cohort, the planner first computes the best existing independent
payload cost. A dictionary candidate includes all encoded cohort payloads, the
exact canonical Dictionary record length, the 64-byte section header, and the
exact canonical TransformPlan record length. It must beat the independent
payload by more than 128 bytes. A simpler independent representation wins all
ties. Training failure is a normal rejected candidate, not an archive error.

## Bounded ChunkGroups and lookback

Dense and extreme may instead select a Zstandard reference-prefix plan. A
ChunkGroup stores only its stable identifier, `max_lookback`, and exact worst
case `max_preceding_bytes`. Membership authority exists only in each Chunk's
`group_ref`. Consecutive CHUNK_DATA frame order is the authority for dependency
order; ChunkGroup never duplicates a member list.

To decode a dependent Chunk, the reader decodes up to `max_lookback` preceding
Chunks in the same contiguous group, concatenates their plaintext in physical
order, and provides the final 1 MiB as the Zstandard reference prefix. The first
Chunk has an empty prefix. This uses the safe Zstandard reference-prefix API;
there is no hidden persistent compressor state and no unsafe Rust in
Entrybound.

The planner charges encoded payloads, the exact plan and group record lengths,
and the 64-byte group-section header. It uses the same strict 128-byte cohort
gain rule. The declared access bytes equal the maximum sum of the full logical
lengths of the preceding required Chunks, because those Chunks must themselves
be decoded. Open and verify recompute this value, enforce contiguity and bounds,
and aggregate it into the archive decoder working-set requirement.

## Physical order and identity

Accepted similarity cohorts are physically contiguous in cohort-ID order;
remaining unique Chunks follow in digest order. ContentObject Chunk-reference
order and canonical Entry order never change. Physical reordering, dictionaries,
codec parameters, and group layout do not enter ContentObject logical digests,
LAI, AUX, or the current PCR construction. With unchanged boundaries and
metadata those identities remain unchanged; exact-byte PCI may change.

## ECF extension and compatibility

The experimental namespace remains `ecf/bootstrap-v1`, extended through
required incompatibility feature bit `0x0000_0000_0000_0001`, named
`cross-file-compression-v1`. An extended archive uses canonical section order:

```text
DESCRIPTOR, TRANSFORM_PLANS, DICTIONARIES, CHUNK_GROUPS,
CHUNK_DATA, MANIFEST_RECORDS, FIDELITY, optional INDEX
```

Dictionary record type 11 contains identity, codec, format, construction, and
bytes. ChunkGroup record type 12 contains identifier, max lookback, and max
preceding bytes. Extended version-2 Chunk frames append the sole 32-byte
`group_ref` authority to the historical header. A zero digest means no group.

New readers retain the historical section schema and version-1 Chunk frames
for `*-v1`/`*-v2` archives. Historical readers see the unknown required feature
and fail closed. Missing, duplicate, corrupt, unsupported, or mismatched
dictionaries and invalid group references, ordering, lookback, or access costs
produce stable typed diagnostics.

This version does not add content-specific parsers, additional codecs,
unbounded solid compression, cross-archive dictionaries, STREAM layout,
encryption, or remote retrieval.
