# Platform fidelity v1

Status: canonical platform/security objects are fully readable and writable.
Capture and restoration are enabled only where the build has a safe,
no-follow, exact platform boundary. Entrybound itself retains
`unsafe_code = "forbid"`.

## Capture

Linux reads `system.posix_acl_access` and directory
`system.posix_acl_default` through `xattr 1.6.1` no-follow operations, parses
the kernel ACL xattr format independently, and emits canonical POSIX1E ACLs.
Those two xattrs are not duplicated in `posix.xattrs`. Other accessible xattrs
remain exact. The existing capability traversal, source stability checks,
hardlink consistency, and sparse discovery continue to apply.

Windows capture uses safe `cap-std 4.0.3` metadata for stable file attributes
and 100-nanosecond creation time. Directory/sparse/reparse authority bits are
excluded from the attribute metadata item. Traversal checks no-follow metadata
and refuses every reparse source before opening or descending through it when
exact tag/payload capture is unavailable.

The audited safe wrappers evaluated for Windows do not expose both exact
self-relative security-descriptor bytes and exact opaque reparse buffers with
the required no-follow get/set behavior. Consequently this build records
`windows.security-descriptor` and `windows.reparse-original` as unavailable in
FidelityReport and refuses reparse capture. It does not substitute a lossy ACL
view or local unsafe FFI. Canonical descriptors/reparse objects received from
an archive remain fully parsed, validated, identity-bound, inspectable, and
preserved by repack.

macOS uses safe no-follow standard metadata to capture `st_flags`, birthtime,
POSIX metadata, and xattrs (including FinderInfo/ResourceFork when accessible).
There is no audited exact macOS ACL-to-frozen-NFS4 adapter in this build, so
`security.acls` is reported unavailable rather than partially translated.

Hardlink aliases must have identical inode-scoped metadata under the existing
capture comparison. Any disagreement is source instability. Per-object limits
(three ACLs, 65,536 ACEs, 1 MiB descriptors, 16,376-byte reparse buffers) and
aggregate `ResourceBudget.max_metadata_bytes` are checked before acceptance.

## Extraction policy and ordering

ExtractionPolicy adds independent controls:

- ACL: Ignore (default) or Restore;
- Windows security: Ignore (default) or Restore;
- reparse: Refuse (default), KnownSafe, or All;
- platform metadata: Ignore (default) or Restore.

CLI spellings are `--acls`, `--windows-security`, `--reparse`, and
`--platform-metadata`. Ignored, unavailable, privilege-denied, and partially
restored classes are always included in ExtractionReport.

Materialization remains verified-before-write. Ordinary directories and files
are created first, then hardlinks. Ownership and ordinary xattrs precede ACLs;
ACLs precede final mode; timestamps are last. Directory metadata is applied
deepest-first after descendants exist. Symlinks remain last. Reparse objects
would also be last, but exact reparse restoration is refused in this build
because there is no audited safe exact-set API. Thus archive-created namespace
objects cannot redirect later extraction writes.

Linux restores POSIX1E ACLs through file-descriptor xattr operations. NFS4 ACL
restoration on Linux is reported incompatible. Windows descriptor restoration,
opaque reparse restoration, macOS ACL restoration, flags, and birthtime are
reported unavailable when requested; no result claims full fidelity for a
subset. SACL absence caused by capture privilege is represented by
FidelityReport rather than guessed.

## Tooling and portability

INDEXED, STREAM, encrypted private manifests, random metadata reads, and repack
share the versioned Entry/Metadata decoder. Diff classifies ReparsePoint
changes as SEMANTIC and ACL/platform metadata changes as AUXILIARY. Inspect and
explain summarize security material unless an explicit future detailed
security surface is selected.

Frozen `zip/portable-v1` and `tar/pax-v1` bytes are unchanged. Their analyzer
reports ACLs, descriptors, flags, birthtime, and other unsupported platform
metadata as LOSSY; opaque ReparsePoint is REFUSED. A logical Symlink with only
Windows original reparse metadata remains subject to the profile's existing
Symlink support plus an explicit AUX-loss issue.
