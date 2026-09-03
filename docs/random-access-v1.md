# Verified random access v1

Status: implemented for Complete `INDEXED` archives. This is an access API and
verification doctrine, not a new ECF wire feature.

## Boundary

`open`, `open_with_policy`, and `verify` retain their historical whole-archive
meaning. `open_indexed_random` and `open_indexed_random_encrypted` are separate
APIs over `RandomReadSource`; they do not buffer the source or call the full
reader. `STREAM` is rejected because its declared capability remains a complete
sequential pass.

A successful `read_entry` proves the requested regular file, not the archive as
a whole. Its `RandomAccessVerificationReport` always has
`whole_archive_verified=false` and `PCI=NotComputed`. PCR remains
`DeclaredNotFullyVerified`, because unread physical records can change or be
corrupt without affecting the requested object proof.

## Source and session

`RandomReadSource` exposes exact length, exact positioned reads, and a
`SourceRevision`. Implementations exist for immutable memory, one held local
file handle, and HTTP(S). Every offset and extent is checked before allocation.
A session pins its initial revision, maintains a bounded range cache, accounts
all reads, and rechecks detectable revision state before returning metadata or
file content.

The default caller-owned `RandomAccessPolicy` permits at most:

- 64 MiB in one range, 512 MiB fetched, and 100,000 source range requests;
- 128 MiB of fetched metadata and 64 section headers;
- 4,000,000 scanned Chunk-frame headers and 65,536 dependency Chunks;
- 8 GiB of decoded requested logical data and 128 MiB of cached ranges;
- 4,000,000 encrypted SegmentHeaders and 100,000 trace entries.

It also carries the ordinary `ResourceBudget` and `DecodeRequirements` caller
policies. Archive declarations can only narrow work; they cannot raise caller
limits. Applications should choose smaller values appropriate to their trust
and latency budget.

## Unencrypted opening

Opening fetches the fixed footer and preamble, checks their binding and total
length, then walks only 64-byte section headers to create a session-local
directory. Section order, extent, overlap, required cardinality, layout, role,
and features are checked without fetching unrelated section payloads.

Descriptor and Manifest are fetched and verified by section SHA-256 before
parsing. Entry/ContentObject references and metadata-derived LAI are verified.
TransformPlans, Dictionaries, ChunkGroups, ReconstructionData, and
ReconstructionRegions are fetched only for an entry read; their aggregate
decoder declaration is independently checked.

## Index doctrine and dependencies

The Index remains a non-authoritative cache. A valid Index supplies candidate
Chunk-frame offsets. The selected frame header must independently name the
expected digest and agree on stored length and physical declarations. If the
Index is absent, fails its digest/canonical parse, has impossible extents, or
points at the wrong frame, locators are rebuilt by reading only Chunk-frame
headers and skipping stored payloads. Rebuild work is bounded and reported as
`RebuiltAbsent` or `RebuiltInvalid`.

Before ordinary payload ranges are prefetched, the reader computes transitive
lookback closure from verified frame headers and physical order. It validates
group identity, maximum lookback, and preceding bytes. A reconstruction region
uses the complete declared region and dependencies; access declarations and
every reconstructed member digest are checked by the ordinary implementation.

Every Chunk is accepted only after canonical frame parsing, bounded codec
decode, inverse transforms/reconstruction, exact logical length, and SHA-256
identity verification. Requested Chunks are concatenated in authoritative
ContentObject order; Chunk root, logical size, and complete ContentObject digest
must match before bytes are returned.

## Encrypted INDEXED

Encrypted opening fetches public preamble/footer v2, CryptoEnvelope, and public
SegmentHeaders first. Crypto/KDF policies are enforced before unlock. The AFK
candidate is accepted only after key commitment and envelope MAC checks.
CONTROL segments are authenticated; Descriptor v2's private resource/decode
declarations are enforced before dependent payload decoding. Manifest,
TransformPlans, ChunkGroups, Fidelity, EncryptedIndex, and terminal
`ArchiveFinalV1` bindings are authenticated.

The encrypted Index maps a Chunk to a candidate PAYLOAD segment and remains a
cache. A selected segment passes SegmentHeader/counter checks, exact AEAD AD,
AES-256-GCM-SIV authentication, fragment reassembly, object digest, `EBPO`
dispatch, and Chunk-frame checks. Invalid or absent indices are rebuilt through
bounded authenticated PAYLOAD-object scanning. Archive-ID-bound AD prevents
cross-archive segment use.

Encrypted Chunk headers live inside authenticated private objects. Lookback
discovery can therefore authenticate the requested Chunk object before learning
its private group reference; the complete bounded dependency closure is still
verified before requested file plaintext is exposed. No duplicate public group
authority or new wire feature was added.

The terminal CONTROL segment and `ArchiveFinalV1` are authenticated and checked
against Descriptor/Manifest object IDs, recipient-set digest, and footer core.
The complete segment-sequence digest is not claimed without reading all
segments.

## Reports and trace

Reports distinguish verified metadata, one verified ContentObject, dependency
counts, reconstruction/group/dictionary checks, Index status, source stability,
fetched bytes, request count, and identity status. The bounded optional trace
labels footer, preamble, section headers, Descriptor, Manifest, Index, Chunk,
dictionary, lookback, reconstruction, encrypted CONTROL, and encrypted PAYLOAD
ranges plus cache hit/miss. Trace/cache state never affects identity.

`ebound read` stages a complete verified file before exclusive output creation
or stdout emission. Remote `list` and `inspect` label results as range-backed
metadata views and never print a whole-archive verification claim.

Entry v2 metadata is part of the same verified Manifest lookup. Remote list and
inspect can therefore expose Symlink kind/target summary without unrelated
payload ranges. `read` remains a regular-file operation and rejects Directory
and Symlink Entries. When a requested File carries a sparse map, its verified
logical bytes are checked against every declared hole before return.
