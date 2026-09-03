# Native filesystem bootstrap

This note scopes the first filesystem workflow layered on
`ecf/bootstrap-v1`. It does not change the wire format or EAM semantics.

Packing accepts one caller-selected directory and traverses it through held
capability-directory handles. Directory entries are sorted before traversal;
the resulting Entries are then ordered and validated by `EntrySet`. A regular
file is opened once per attempt, its metadata and bytes come from that handle,
and length, mtime, and executable status are checked again on the same handle.
Two fresh-handle retries follow the initial attempt. Persistent change returns
`EB_INPUT_SOURCE_UNSTABLE`. Symlink metadata and exact target bytes are read
without following the link, and traversal never descends through a symlink.
Devices, FIFOs, sockets, and other unsupported objects are rejected.

The bootstrap captures `core.mtime`; supported Unix systems additionally
capture `core.executable`, `posix.mode`, numeric uid/gid, xattrs, and hardlink
topology. Linux uses `SEEK_DATA`/`SEEK_HOLE` to capture sparse layout without
inferring holes from zero-filled content. Platform capabilities that cannot be
observed reliably are declared unavailable in FidelityReport rather than
fabricated. ACLs, special-file semantics, and other platform-specific metadata
remain unavailable. The normative feature and policy are defined in
[posix-metadata-v1.md](posix-metadata-v1.md) and
[filesystem-fidelity-v1.md](filesystem-fidelity-v1.md).
New files use normalized Gear-hash content-defined chunking. The default
`balanced-v6` policy uses 128 KiB minimum, 512 KiB target, and 2 MiB maximum
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
directories and representative files exclusively, then hardlink siblings,
then symbolic links last. The default `Safe` symlink policy accepts only
relative targets whose lexical resolution stays beneath the root; `Refuse` and
explicit `All` are also available. Ownership, xattr, and sparse restoration
are caller-controlled. Ownership precedes final mode, directory metadata is
applied after descendants, and timestamps are last. Any metadata that cannot
be restored is returned in `ExtractionReport`.

The CLI bootstrap policy permits at most 1,000,000 entries, 64 GiB total
logical bytes, 16 GiB for one file, 4,000,000 distinct Chunks, path depth 1,024,
and 1 GiB for the manifest/metadata bound. A declaration above caller policy is
`POLICY_REFUSED`; decoded actuals above the archive's own declaration are
corruption. Applications embedding the library should choose limits suitable
for their environment. Zstandard archives additionally declare a 1 MiB decoder
window or up to an 8 MiB LZMA2 dictionary and an aggregate working-set
requirement covering codec state, stored Dictionaries, and maximum bounded-group
access. The CLI permits up to 384 MiB so a declared v6 JPEG reconstruction
working set can be honored.
The bootstrap caller policy enforces both before section decoding and can be
narrowed by an embedding application.
