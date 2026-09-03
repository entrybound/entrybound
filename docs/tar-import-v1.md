# Strict tar-family import v1

Status: implemented as `tar-strict/v1` under `entrybound::legacy`. The adapter
produces format-neutral Legacy Observation Model (LOM) evidence; it does not
make tar fields part of EAM.

## Structure and authorities

The parser walks exact 512-byte records with checked extent arithmetic. It
retains each complete header block plus separate observations for `name`,
`mode`, `uid`, `gid`, `size`, `mtime`, checksum, `typeflag`, `linkname`,
magic/version, `uname`, `gname`, device numbers, ustar `prefix`, payload, and
padding. Header checksum is verified using the POSIX space-filled checksum
field; both historically valid unsigned and signed-byte sums are accepted.

Supported structural dialects are POSIX ustar, POSIX pax global/per-entry
extended headers, GNU long-name/long-link records, and structurally valid GNU
base-256 numeric fields. Pax records use their decimal length as framing and
must be exact UTF-8 `key=value\n` records. Unknown pax keys remain observations
and are reported as unavailable fidelity.

Authorities stay separate until strict reconciliation:

1. the base header, including the ustar `prefix/name` composition;
2. a persistent global pax claim;
3. the next-entry local pax claim;
4. the next-entry GNU long-name or long-link claim.

The pax specification's local-over-global and pax-over-base rules are recorded
as `Refinement` resolutions. A GNU long name and a selected pax `path` that
disagree are `Divergence` and strict import refuses them. Repeated incompatible
pax claims, malformed extension chains, or an extension without a target fail
closed. The same immutable observation is intended for future versioned tar
compatibility profiles.

## Projection and safety

Regular files (`0`, NUL, and compatible regular variants), directories (`5`),
symbolic links (`2`), and hard links (`1`) project into EAM. A symlink target is
selected by the normative pax `linkpath`, compatible GNU long-link, or base
header claim and is retained as exact LinkTarget bytes; it is never parsed as a
LogicalPath. A hardlink target is a safe in-archive LogicalPath, must resolve to
a regular file, and becomes an ordinary File Entry sharing that ContentObject
plus deterministic `posix.hardlink-group` metadata. Link payloads must be empty
and conflicting target authorities fail strict import. Character/block devices,
FIFOs, GNU sparse entries, and other special types remain observed and refused.

Resolved paths must be UTF-8 Entrybound `LogicalPath` values. Absolute paths,
Windows-rooted or backslash paths, NUL, empty components, `.`/`..`, duplicates,
file-as-ancestor relationships, and collisions are refused rather than
normalized. Missing ancestor directories may be synthesized only when no
foreign observation contradicts directory kind; every synthesis is an
`Omission` resolution in conversion provenance.

The native POSIX metadata mapping is:

- any regular execute bit maps to `core.executable`;
- tar seconds or pax fractional `mtime` maps to `core.mtime`. Native precision
  classes exactly represent 0, 2, 6, 7, or 9 fractional digits; other pax
  granularities retain the exact value using the next suitable class and add
  an explicit fidelity limitation rather than silently claiming exact source
  precision;
- permission/special bits map to `posix.mode`, and numeric uid/gid map to
  `posix.uid`/`posix.gid`;
- hardlink membership maps to the canonical native group described above.

User/group names, device numbers, access or change times,
ACL/xattr/security/vendor pax namespaces, and unknown pax keys remain auxiliary
observations and explicit `FidelityReport` losses. Tar sparse declarations are
still refused rather than converted to native sparse metadata because GNU/PAX
sparse authority has not been frozen for this strict adapter.

Two all-zero blocks terminate the logical archive. Additional trailing zero
blocks are accepted and observed. Nonzero file padding or any nonzero byte
after the terminator is invalid.

## Resource policy and identity

Caller-owned limits bound source bytes, entry count, single and aggregate file
bytes, names, extension bytes, pax records, observations per subject and in
aggregate, conflicts, and resolutions. All payload and padded extents are
checked before slicing or allocating.

Successful import records source format `tar` (or a layered `tar+...` format),
adapter ID, exact outer SHA-256, observations, resolutions, synthesized
ancestors, and fidelity losses in canonical ConversionProvenance type 28. This
evidence changes AUX only. It does not enter LAI or PCR, and tar payload bytes
are replanned by the ordinary native CDC/dedup/compression pipeline.

```sh
ebound convert archive.tar archive.eb --strict
ebound convert archive.tar archive-stream.eb --strict --layout stream
ebound convert archive.tar ignored.eb --strict --dry-run
```

Tar compatibility profiles, tar preservation, special files, and sparse-file
projection are not defined by v1. Frozen tar export remains a separate target
profile and is not broadened by native link support.

## Differential development evidence

`tools/tar-strict/probe.py` regenerates a small non-production behavior matrix
against Python `tarfile` 3.13.5 and bsdtar/libarchive 3.8.8 on the probed
Windows host. The checked `observed-outcomes-v1.json` records that both tools
list duplicate and traversal names, accept one-zero-block termination, and
ignore nonzero bytes after a valid terminator; strict Entrybound deliberately
refuses each case under native uniqueness, path, and framing invariants. All
three reject a bad header checksum. GNU tar was unavailable on this host, so no
GNU runtime-compatibility rule is inferred. These observations are evidence
for a future versioned compatibility profile, never production authority.
