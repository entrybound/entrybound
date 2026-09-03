# POSIX filesystem fidelity v1

Status: implemented for Unix capture/restoration, with explicit capability
reporting elsewhere. Archive validity and extraction safety are separate.

## Capture

Traversal uses held capability directories and no-follow type/metadata calls.
A source symlink is captured as an Entry and is never traversed or opened as
its target. Unix target bytes come from the native `OsStr` representation.
Regular files and directories capture executable, nanosecond mtime, mode
`& 0o7777`, numeric uid/gid, and accessible xattrs. Xattr path operations are
no-follow; an enumerated attribute disappearing or failing unexpectedly is a
source-instability/capture failure, never silent omission.

Unix `(device,inode)` pairs are temporary traversal evidence for multiply
linked regular files. Group IDs are finalized only after canonical paths and
ContentObject digests are known; inode values never enter output bytes or an
identity root. Every alias is stability-checked and inode-scoped metadata must
agree.

Linux sparse discovery uses `SEEK_DATA`/`SEEK_HOLE`. It records returned data
extents rather than guessing from zero runs. Unsupported filesystem behavior
omits the sparse map and adds a typed FidelityReport limitation; logical bytes
are always captured. Other platforms likewise report unavailable POSIX classes
instead of fabricating values.

Production POSIX dependencies are narrowly scoped: `xattr 1.6.1`
(MIT/Apache-2.0) provides no-follow xattr operations, and `rustix 1.1.4`
(Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT) provides safe Unix
`fchown` and Linux sparse seeks. Entrybound adds no unsafe code.

## Extraction policy

ExtractionPolicy has independent caller-owned choices:

- symlinks: `Refuse`, `Safe` (default), or `All`;
- ownership: `Ignore` (default) or `Restore`;
- xattrs: `Ignore` (default) or `Restore`;
- sparse: `Logical` (default) or `Restore`.

Safe symlinks require a relative target whose lexical resolution from the
link's parent stays beneath the extraction root. Absolute, drive/rooted, and
escaping targets are refused without rewriting the archived target. `All` is
the explicit opt-in for exact absolute/escaping targets.

The extractor fully verifies the archive before creating final objects. It
then creates directories and hardlink representatives, writes file content or
declared sparse data extents, creates remaining hardlinks, applies directory
metadata from deepest to shallowest, and creates symlinks last. Consequently an
archive-created symlink can never redirect a later extraction write.

Ownership precedes final permission bits because chown may clear setuid/setgid.
Xattrs are applied through file descriptors or no-follow symlink operations.
Timestamps follow operations that would otherwise mutate them. Restrictive
directory permissions are delayed until descendants exist. Sparse Restore
truncates to the full logical size and writes only declared data extents;
Logical writes the complete byte sequence. Every skipped or failed metadata
operation appears in ExtractionReport rather than being described as restored.

Symlink ownership and no-follow symlink timestamp restoration are currently
reported capture-only where the portable safe API cannot guarantee them.
Ownership restoration can require privilege. Windows accepts only UTF-8 link
targets and may require platform symlink privilege; POSIX_BYTES capture and
full POSIX restoration are Unix-only.
