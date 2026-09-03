# Native POSIX metadata v1

Status: implemented. This document freezes the semantic and canonical wire
contract selected by required incompatibility feature `0x8000`
(`posix-metadata-v1`). The feature is present exactly when an archive contains
a Symlink Entry or any metadata name 3–8 below. Feature-absent archives retain
the historical Entry and MetadataItem version-1 bytes and identity results.

## Entry model and identity

The closed native Entry kinds are Directory, File, and Symlink. A File has one
internal ContentObject digest. A Directory has no content. A Symlink has no
ContentObject and carries one exact `LinkTarget`:

| Encoding | ID | Canonical condition |
|---|---:|---|
| UTF8 | 1 | target bytes are valid UTF-8 |
| POSIX_BYTES | 2 | target bytes are not valid UTF-8 |

Targets are nonempty-or-empty opaque link targets, bounded to 1 MiB and may not
contain NUL. They are not LogicalPaths: `/`, `..`, and absolute targets are
valid archive semantics. Extraction policy separately decides whether creating
the link is safe. UTF-8 bytes encoded as POSIX_BYTES are noncanonical.

Path, kind, file ContentObject digest, Symlink target encoding/bytes, and
`core.executable` contribute to LAI. `core.mtime`, mode, ownership, hardlink
topology, xattrs, and sparse layout contribute to AUX. None of the new metadata
enters PCR solely by existing. Exact container serialization remains PCI.

## Entry record v2

Entry remains canonical record type 3. Version 2 has these strictly increasing
fields:

| Tag | Type | Cardinality and meaning |
|---:|---|---|
| 1 | sequence | one or more canonical PathComponent records |
| 2 | u8 | explicit kind: 1 Directory, 2 File, 3 Symlink |
| 3 | bytes[32] | required only for File: ContentObject logical digest |
| 4 | u8 | required only for Symlink: LinkTarget encoding |
| 5 | bytes | required only for Symlink: exact target bytes |
| 6 | sequence | canonical MetadataItem-v2 records |
| 7 | bytes[32] | Entry identity digest |
| 8 | bytes[32] | Entry AUX digest |

Directory omits tags 3–5; File has tag 3 and omits 4–5; Symlink omits 3 and has
4–5. No kind is inferred from absence. Entry v2 must contain a Symlink or a
metadata-v2 name 3–8, so an old Entry cannot acquire a second canonical form.
Entry v1 remains byte-for-byte unchanged.

## MetadataItem v2

MetadataItem remains type 8. Every item nested in Entry v2 uses record version
2, including historical names 1 and 2. Tags 1–3 are required: name `u8`,
criticality `u8=0` (Optional), and restorability `u8=1` (Restorable). Exactly
one typed value field is then present:

| Name ID | Name | Typed value |
|---:|---|---|
| 1 | `core.executable` | tag 4 bool |
| 2 | `core.mtime` | tag 5 bytes containing one canonical Timestamp type-9 record |
| 3 | `posix.mode` | tag 6 u32, only bits `0o7777` |
| 4 | `posix.uid` | tag 6 u32 |
| 5 | `posix.gid` | tag 6 u32 |
| 6 | `posix.hardlink-group` | tag 7 bytes[32] |
| 7 | `posix.xattrs` | tag 8 sequence of XAttrV1 records |
| 8 | `posix.sparse-map` | tag 9 bytes containing one SparseMapV1 record |

MetadataSet is uniquely keyed and ordered by numeric name ID. If mode and
executable are both present, executable must equal `(mode & 0o111) != 0`.
File-type bits are never stored in mode.

## Hardlinks

Hardlinks are ordinary File Entries that reference the same ContentObject.
They additionally carry one group digest; equal files without it are not
hardlinks. Every group has at least two members, identical inode-scoped
metadata, and one membership. Its inode-independent ID is the Entrybound
structured hash with domain `entrybound/hardlink-group/v1` over the shared
ContentObject digest and a count-prefixed, length-framed, canonically sorted
LogicalPath sequence. Readers recompute the digest from the complete EntrySet.
Device and inode numbers are traversal evidence only and are never serialized.

## Xattrs

XAttrV1 is record type 37/version 1: tag 1 exact name bytes, tag 2 exact value
bytes. Names are nonempty, NUL-free, at most 255 bytes, sorted by raw bytes, and
unique. Values are at most 16 MiB; an Entry has at most 4096 xattrs. Empty and
binary values are valid. Generic inspection reports only name, length, and
SHA-256. Feature `0x10000` adds a higher-level canonical ACL model. On Linux,
recognized `system.posix_acl_access` and `system.posix_acl_default` values are
projected there and are not duplicated in `posix.xattrs`; all other accessible
xattrs remain exact. Feature-absent archives retain the v2 interpretation.

## Sparse maps

SparseMapV1 is record type 38/version 1: tag 1 logical size `u64`, tag 2 an
ordered sequence of SparseExtentV1. SparseExtentV1 is type 39/version 1 with
offset `u64` tag 1 and nonzero length `u64` tag 2. Extents are in range,
strictly ordered, nonoverlapping, and nonadjacent (adjacent extents must be
merged). Bytes outside extents are holes and must be zero in the authoritative
ContentObject. A sparse map is valid only on a File and has at most 1,000,000
extents. The ContentObject always remains the sole full logical-byte authority.

## Bounds and integration

Fixed per-object bounds above combine with caller-owned
`ResourceBudget.max_metadata_bytes`, entry count, logical-byte, and decoder
limits. Canonical sequence framing and checked arithmetic apply before semantic
construction. INDEXED, STREAM, encrypted private manifests, metadata-first
random access, representation-only/replanned repack, diff, inspect, and explain
all use the same Entry/Metadata decoder. Encrypted archives reveal none of this
private manifest material before authentication. Entry-v3 and MetadataItem-v3
extend this model without redefining it; see
[security-metadata-v1.md](security-metadata-v1.md).

Deterministic Entry vectors, including the historical v1 bytes, are in
[posix-metadata-v1-vectors.txt](posix-metadata-v1-vectors.txt).
