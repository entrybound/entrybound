# POSIX filesystem fidelity v1

Status: implemented for Unix capture/restoration, with explicit capability
reporting elsewhere. Archive validity and extraction safety are separate.

## Capture

Traversal enumerates held capability directories and inspects source entry
types without following links. An inspected source symlink is represented as
an Entry. Unix target bytes come from the native `OsStr` representation.
Regular files and directories capture executable, nanosecond mtime, mode
`& 0o7777`, numeric uid/gid, and accessible xattrs. Xattr path operations are
no-follow at the final path component; an enumerated attribute disappearing or failing unexpectedly is a
source-instability/capture failure, never silent omission.

Capture does not yet provide a complete concurrent-source guarantee. Initial
type inspection and opening a regular file/directory are separate operations;
ACL/xattr/platform capture still resolves ancestor pathnames. A concurrent
source replacement can therefore redirect an initial open or metadata read.
Later held-handle stability checks do not prove the initial no-follow identity
or bind every metadata read. These capture adapters require further repair and
native race qualification before that stronger guarantee can be advertised.

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
- special permissions: `Ignore` (default) or `Restore`;
- privileged xattrs: `Ignore` (default) or `Restore`;
- sparse: `Logical` (default) or `Restore`.

Canonical ACL and Windows/macOS policy controls extend this list in
[platform-fidelity-v1.md](platform-fidelity-v1.md).

Safe symlinks require a relative target whose effective resolution through
archived links stays beneath the extraction root. Parent traversals are
evaluated after each link expansion. Cycles and chains exceeding 40 expansions
are refused, as are preexisting link/reparse components on the resolved target.
Absolute, drive/rooted, and
escaping targets are refused without rewriting the archived target. `All` is
the explicit opt-in for exact absolute/escaping targets.

The extractor fully verifies the archive before creating final objects. It
then creates directories and hardlink representatives, writes file content or
declared sparse data extents, creates remaining hardlinks and symlinks, then
applies final directory metadata from deepest to shallowest through retained
directory handles. Consequently an
archive-created symlink can never redirect a later extraction write.

Ownership precedes final permission bits because chown may clear setuid/setgid.
Setuid, setgid and sticky bits are stripped unless the caller independently
selects `SpecialPermissionsPolicy::Restore` (`--restore-special-permissions`).
Ownership restoration alone does not authorize them. Ordinary xattr restoration
allows the `user.` namespace; other namespaces require the independent
`PrivilegedXAttrPolicy::Restore` (`--restore-privileged-xattrs`) as well as
`XAttrPolicy::Restore`. ACL xattrs always use the canonical ACL policy, never
the ordinary xattr bypass. Archive facts and their identities remain unchanged.
Regular-file and directory xattrs are applied through held file descriptors.
Unix files and directories are created with restrictive `0600`/`0700` modes,
before content writes. Linux inherited access/default ACLs are removed from newly owned objects and
read back before descendants are created; an existing destination root's ACLs
are preserved. Final mode precedes final ACL restoration, since chmod changes
the POSIX ACL mask. Both mode and ACL are read back; a conflicting requested
mode/ACL or a failed verification is a reported fidelity failure.
Timestamps follow operations that would otherwise mutate them. Restrictive
directory permissions are delayed until descendants exist. Sparse Restore
truncates to the full logical size and writes only declared data extents;
Logical writes the complete byte sequence. Every skipped or failed metadata
operation appears in ExtractionReport rather than being described as restored.

Symlink ownership, xattrs, mode and no-follow timestamp restoration are currently
reported capture-only where the portable safe API cannot guarantee them.
No symlink metadata setter re-resolves an ambient destination pathname.
Created parent directories and hardlink representatives retain their handles;
metadata operations use the opened object rather than reopening its name.
Linux hardlink creation uses the held representative's procfs fd link; missing
procfs or a changed representative fails safely. Windows holds extraction handles
without delete sharing and verifies representative and alias identity. On other
platforms this build has no safe held-source hardlink operation: archives with
hardlink groups are refused before destination materialization. A detected race
refuses extraction and preserves
names that may now belong to another actor.

Two concurrent destination cases remain unqualified: substituting an ordinary
directory between directory creation and opening, and moving a held Unix
descendant outside the destination before a later write. Retaining a handle
does not prevent those relocations.
These are open implementation defects against the kernel-confinement contract;
the existing handle and identity checks do not qualify those cases as safe.
The current extractor therefore returns `ConfinementMode::WeakerReported`,
including on platforms whose initial directory acquisition has not been
qualified. Capability-relative lookup alone does not justify reporting the
full output boundary as `KernelEnforced`. This reporting correction does not
satisfy the outstanding confinement qualification.

Without an archived POSIX mode, `core.executable` controls regular-file execute
permission. It does not remove directory search permission: directories from
non-POSIX hosts retain their restrictive initial owner access. An archived
POSIX directory mode is restored subject to the explicit metadata policy.

Ownership restoration can require privilege. Windows accepts only UTF-8 link
targets and chooses the native file/directory link operation from the
resolved archived target's kind. Unknown target
kinds are refused before final output; successful link creation still requires
the platform's symlink privilege. POSIX_BYTES capture and
full POSIX restoration are Unix-only.
