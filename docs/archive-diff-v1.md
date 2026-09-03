# Archive diff v1

Status: implemented as `entrybound/archive-diff-v1`.

Archive diff preserves the four identity layers:

| tier | comparison |
| --- | --- |
| SEMANTIC | LAI-bearing paths, kinds, identity metadata, ContentObject digests |
| AUXILIARY | non-LAI metadata, FidelityReport, conversion and preservation evidence |
| PHYSICAL | PCR-bearing chunking, order, plans, dictionaries, groups, reconstruction |
| CONTAINER | layout, Index, feature/framing/security state, exact PCI |

The summary always reports LAI, AUX, PCR, and PCI as `SAME`, `DIFFERENT`,
`NOT_VERIFIED`, or (for PCI) `NOT_COMPUTED`. Digest equality does not suppress
the ordered detail records requested by the caller. File-content detail is
digest and size only; preserved foreign source bytes are never dumped.

`--json` emits compact deterministic UTF-8 JSON with schema
`entrybound/archive-diff-v1`. Records are ordered by tier, subject, and field.

## Verification scope

Local unencrypted INDEXED/STREAM inputs are fully opened. Encrypted INDEXED
inputs require side-specific unlock material before any private tier is
compared. Recipient-only changes therefore remain container/addressing changes,
not semantic changes.
`--public` is the explicit locked-encrypted mode: it compares only approved
public crypto framing and reports every native identity as `NOT_VERIFIED`.

When either side is an HTTP(S) URL, both sides use the revision-pinned
RandomReadSource metadata path. Complete verified Manifest/ContentObject
metadata permits semantic comparison, but unread payload means PCR is
`NOT_VERIFIED` and PCI is `NOT_COMPUTED`. The report includes transferred bytes
and request counts. STREAM URLs are not presented as random-access sources.

Under `posix-metadata-v1`, Symlink kind/target and `core.executable` changes are
SEMANTIC. Mode, uid/gid, hardlink group, xattrs, and sparse-map changes are
AUXILIARY. Equal ContentObject digests never imply hardlink topology, and a
hardlink-only or sparse-only change is not PHYSICAL unless the recorded native
plan independently changed.
