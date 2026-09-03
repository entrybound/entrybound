# Canonical security and platform metadata v1

Status: implemented canonical model and wire. Native capture/restoration support
is capability-dependent as specified in
[platform-fidelity-v1.md](platform-fidelity-v1.md).

Required incompatibility bit `0x10000` is
`platform-security-metadata-v1`. It requires `0x8000`
(`posix-metadata-v1`) and is set exactly when an Entry uses Entry-v3 or a
MetadataSet contains a name 9–15. Feature-absent archives retain Entry-v1/v2,
MetadataItem-v1/v2, and all historical identity bytes unchanged.

## Entry-v3 and reparse objects

Entry remains record type 3. Version 3 retains Entry-v2 tags 1–8 and adds
optional tag 9, a bytes field containing exactly one canonical type-43
WindowsReparsePointV1 record. Kind IDs are 1 Directory, 2 File, 3 Symlink, and
4 ReparsePoint. Directory, File, and Symlink cardinality is unchanged from
Entry-v2. ReparsePoint has no content digest or symlink fields and requires tag
9. Entry-v3 must contain a ReparsePoint or metadata name 9–15; it is not an
alternate encoding for an older Entry.

WindowsReparsePointV1 is record type 43/version 1:

| Tag | Type | Meaning |
|---:|---|---|
| 1 | u32 | nonzero reparse tag |
| 2 | bytes | exact reparse data, at most 16,376 bytes |

Opaque ReparsePoint tag/data and native Symlink logical target affect LAI.
For a recognized Windows symbolic link, the portable logical target belongs in
the Symlink Entry and the exact original type-43 object belongs in metadata
name 13. The latter is AUX-only. Unknown reparse objects are never coerced to
File or Directory.

## MetadataItem-v3

MetadataItem remains type 8. Every item nested in Entry-v3 uses record version
3, including retained names 1–8. Required tags are name (1/u8), criticality
(2/u8, zero Optional), and restorability (3/u8, 1 Restorable or 2
CaptureOnly). Exactly one value field is allowed:

| ID | Name | Canonical value |
|---:|---|---|
| 1 | `core.executable` | tag 4 bool |
| 2 | `core.mtime` | tag 5 canonical Timestamp |
| 3 | `posix.mode` | tag 6 u32 |
| 4 | `posix.uid` | tag 6 u32 |
| 5 | `posix.gid` | tag 6 u32 |
| 6 | `posix.hardlink-group` | tag 7 bytes[32] |
| 7 | `posix.xattrs` | tag 8 XAttrV1 sequence |
| 8 | `posix.sparse-map` | tag 9 SparseMapV1 |
| 9 | `security.acls` | tag 10 AclV1 sequence |
| 10 | `windows.security-descriptor` | tag 11 WindowsSecurityDescriptorV1 |
| 11 | `windows.file-attributes` | tag 6 u32 |
| 12 | `windows.creation-time` | tag 5 canonical Timestamp |
| 13 | `windows.reparse-original` | tag 12 WindowsReparsePointV1 |
| 14 | `macos.flags` | tag 6 u32 |
| 15 | `macos.birthtime` | tag 5 canonical Timestamp |

Metadata names are unique and numerically ordered. `macos.birthtime` is
CaptureOnly in v1. The other new items are Restorable when platform and caller
policy permit it.

## ACL wire and semantics

AclV1 is type 40/version 1: dialect tag 1/u8, scope tag 2/u8, and tag 3 as an
ordered sequence of type-41 AclEntryV1 records. Dialects are POSIX1E=1 and
NFS4=2. Scopes are ACCESS=1 and DEFAULT=2. POSIX1E supports both scopes;
DEFAULT requires a Directory. NFS4 supports ACCESS only. A present
`security.acls` item contains one to three ACLs; each ACL contains 1–65,536
ACEs and `(dialect,scope)` is unique.

AclEntryV1 fields are type tag 1/u8, principal tag 2/u8, optional numeric ID
tag 3/u32, optional UUID tag 4/bytes[16], permissions tag 5/u32, and flags tag
6/u32. Types are ALLOW=1, DENY=2, AUDIT=3, and ALARM=4. Principals are:

| ID | Principal | Qualifier |
|---:|---|---|
| 1 | USER_OBJ | none |
| 2 | USER | numeric uid tag 3 |
| 3 | GROUP_OBJ | none |
| 4 | GROUP | numeric gid tag 3 |
| 5 | MASK | none |
| 6 | OTHER | none |
| 7 | OWNER@ | none |
| 8 | GROUP@ | none |
| 9 | EVERYONE@ | none |
| 10 | UUID | tag 4, exactly 16 bytes |

POSIX1E entries are ALLOW, have zero flags, and use READ=0x1, WRITE=0x2,
EXECUTE=0x4 only. Canonical order is USER_OBJ, named USER by uid, GROUP_OBJ,
named GROUP by gid, MASK, OTHER. USER_OBJ/GROUP_OBJ/OTHER occur exactly once;
MASK occurs exactly once iff named entries exist.

NFS4 preserves semantically significant ACE order. The accepted permission
registry is the RFC/NFSv4 `ACE4_*` mask `0x001f01ff`: read/write/append data,
read/write named attributes, execute, delete child, read/write attributes,
delete, read/write ACL, write owner, and synchronize. Accepted inheritance and
ACE flags are mask `0x000000ff`: file/directory inherit, no-propagate,
inherit-only, successful/failed-access audit, identifier-group, and inherited.
Unknown bits fail closed.

A POSIX1E ACCESS ACL requires `posix.mode`; USER_OBJ, effective group class
(MASK when present, otherwise GROUP_OBJ), and OTHER must equal its owner,
group, and other mode classes. `core.executable` continues to agree with mode.
No authority is silently regenerated from another.

## Windows descriptor and attributes

WindowsSecurityDescriptorV1 is type 42/version 1 with exact validated
self-relative bytes in tag 1. Maximum length is 1 MiB. Validation covers
revision/reserved/control framing, ACL-presence flags, aligned bounded component
offsets, SID framing, ACL revisions/length/count, the v1 ACE type registry,
object-ACE flags, embedded SIDs, ACE lengths, overlap, and trailing bytes. ACE
and DACL/SACL order is preserved. No POSIX/NFS4 translation occurs.

`windows.file-attributes` accepts mask `0x005af9a7`. Its named bits are
READONLY (`0x1`), HIDDEN (`0x2`), SYSTEM (`0x4`), ARCHIVE (`0x20`), NORMAL
(`0x80`), TEMPORARY (`0x100`), COMPRESSED (`0x800`), OFFLINE (`0x1000`),
NOT_CONTENT_INDEXED (`0x2000`), ENCRYPTED (`0x4000`), INTEGRITY_STREAM
(`0x8000`), NO_SCRUB_DATA (`0x20000`), PINNED (`0x80000`), UNPINNED
(`0x100000`), and RECALL_ON_DATA_ACCESS (`0x400000`). NORMAL is accepted only
alone. DEVICE, VIRTUAL, and the internal-only EA/RECALL_ON_OPEN bit are not
accepted. Directory (`0x10`), sparse
(`0x200`), and reparse (`0x400`) bits are deliberately excluded because Entry
kind, SparseMapV1, and WindowsReparsePointV1 are their sole authorities.
`windows.creation-time` is a signed-seconds Timestamp with source precision.

## macOS fields and identity

`macos.flags` accepts the frozen Darwin UF/SF registry mask `0x40bf80ef`:
UF_NODUMP, UF_IMMUTABLE, UF_APPEND, UF_OPAQUE, UF_COMPRESSED, UF_TRACKED,
UF_DATAVAULT, UF_HIDDEN, SF_ARCHIVED, SF_IMMUTABLE, SF_APPEND, SF_RESTRICTED,
SF_NOUNLINK, SF_SNAPSHOT, SF_FIRMLINK, and SF_DATALESS. Unknown bits fail
closed.
`macos.birthtime` is a nanosecond Timestamp. FinderInfo, ResourceFork, and
quarantine data remain exact xattrs and are not duplicated.

ACLs, Windows descriptors/attributes/times/original reparse data, macOS
flags/birthtime, and retained xattrs contribute to AUX only. They never change
PCR when physical planning is held fixed. Opaque ReparsePoint tag/data and
Symlink targets contribute to LAI. Generic inspection exposes counts,
lengths, flags, and SHA-256 summaries—not raw principals, descriptors, or
security-sensitive attribute values.

Canonical vectors are in
[security-metadata-v1-vectors.txt](security-metadata-v1-vectors.txt).
