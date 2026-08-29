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
Files use fixed 1 MiB chunks and the existing `bootstrap-store-v1`
TransformPlan.

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
for their environment.
