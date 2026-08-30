# Native filesystem bootstrap

This note scopes the first filesystem workflow layered on
`ecf/bootstrap-v1`. It does not change the wire format or EAM semantics.

Packing accepts one caller-selected directory and traverses it through held
capability-directory handles. Directory entries are sorted before traversal;
the resulting Entries are then ordered and validated by `EntrySet`. A regular
file is opened once per attempt, its metadata and bytes come from that handle,
and length, mtime, and executable status are checked again on the same handle.
Two fresh-handle retries follow the initial attempt. Persistent change returns
`EB_INPUT_SOURCE_UNSTABLE`. Symlinks and all non-file/non-directory objects are
rejected without traversal.

The bootstrap captures `core.mtime` and captures `core.executable` on Unix.
Platforms without a portable executable bit declare that class unavailable.
The FidelityReport also declares ACLs, xattrs, ownership, hardlink identity,
platform-specific metadata, and symlink/special-file semantics unavailable.
New files use normalized Gear-hash content-defined chunking. The default
`balanced-v4` policy uses 128 KiB minimum, 512 KiB target, and 2 MiB maximum
Chunks. Exact SHA-256 Chunk deduplication is archive-wide, after which the
creation-only planner performs deterministic similarity analysis and compares
independent multi-codec/structural pipelines with complete-cost shared-dictionary candidates.
Dense and extreme may also select bounded ChunkGroups; balanced never uses
lookback. Each unique plaintext Chunk is encoded once. The reader uses only
recorded plans, Dictionaries, groups, and Chunk references. Historical
`fixed-1mib/v1` and v2 archives remain readable. Policies are
documented in [chunking-v1.md](chunking-v1.md) and
[planner-v1.md](planner-v1.md), with cross-file behavior in
[cross-file-compression-v1.md](cross-file-compression-v1.md).

Extraction fully opens, verifies, and enforces caller resource policy before it
creates the destination root. It holds that root as a `cap-std` `Dir`, resolves
each validated UTF-8 LogicalPath component relative to the held handle, creates
directories explicitly, and creates files exclusively. The only collision
policy is `Refuse`. File metadata is applied after content; directory metadata
is applied in reverse Entry order after the tree exists. Any metadata that the
platform cannot restore is returned in `ExtractionReport`.

The CLI bootstrap policy permits at most 1,000,000 entries, 64 GiB total
logical bytes, 16 GiB for one file, 4,000,000 distinct Chunks, path depth 1,024,
and 1 GiB for the manifest/metadata bound. A declaration above caller policy is
`POLICY_REFUSED`; decoded actuals above the archive's own declaration are
corruption. Applications embedding the library should choose limits suitable
for their environment. Zstandard archives additionally declare a 1 MiB decoder
window or up to an 8 MiB LZMA2 dictionary and an aggregate working-set
requirement covering codec state, stored Dictionaries, and maximum bounded-group
access. The CLI permits up to 128 MiB.
The bootstrap caller policy enforces both before section decoding and can be
narrowed by an embedding application.
