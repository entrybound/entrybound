# Native repack v1

Status: implemented for verified unencrypted Complete INDEXED and STREAM
archives. No wire feature is added.

## Modes

`representation-only` is selected when `--profile` is absent. It retains the
authoritative ContentObject Chunk-reference sequences and every recorded
TransformPlan, Dictionary, ChunkGroup, ReconstructionData object,
ReconstructionRegion, reconstruction audit, and physical Chunk order. Only the
requested layout, STREAM declaration, and reconstructible INDEXED Index change.
The output must have the same LAI, AUX, and PCR.

Supplying `--profile fast|balanced|dense|extreme` selects `replan`. Every source
ContentObject is first reconstructed and digest verified in memory. The current
v6 chunker/planner is then run over those bytes. Entries, semantic metadata,
FidelityReport, ConversionProvenance, and LegacyPreservation are copied as
authoritative EAM data; old physical plans are not. LAI and AUX must remain
equal, while PCR may change.

## Layout and Index

INDEXED-to-STREAM uses the native sequential writer and enforces the requested
finite `--stream-window`; `auto` records the minimum required window.
STREAM-to-INDEXED constructs the ordinary non-authoritative Index.
`--index preserve` is the default (and means present when the source was
STREAM), while `present` and `absent` are valid only for INDEXED output.

## Verification and publication

Preparation is direct EAM-to-ECF; no temporary extraction directory exists.
The resulting bytes are reopened with the appropriate full reader and the
mode-specific identity invariants are checked before publication. The CLI
writes a private sibling, flushes it, and publishes through an exclusive link;
an existing destination is never overwritten. `--dry-run` performs planning,
encoding, reopening, and identity comparison but creates no output.

For a canonical same-representation rewrite, byte equality is reported when it
actually occurs. A historical representation that canonical current writers do
not reproduce is reported as “semantic/physical equivalent; container
rewritten.” Encrypted repack is refused rather than silently decrypting.

Representation-only repack copies Entry v2 and every POSIX metadata record at
the EAM level and requires LAI/AUX/PCR equality. Replanning preserves the same
Entries, link targets, hardlink groups, xattrs, and sparse maps while rebuilding
only physical content representation; it requires LAI/AUX equality.

Entry-v3 and MetadataItem-v3 platform/security records follow the same rule.
Opaque ReparsePoint semantics preserve LAI; ACLs, Windows security/platform
metadata, original reparse bytes, and macOS metadata preserve AUX exactly.
