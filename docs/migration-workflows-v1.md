# Migration workflows v1

Status: implemented as entrybound/migration-report-v1.

## One semantic source

ebound publish establishes one verified EAM and derives every requested
artifact from it:

    verified/opened .eb ─┐
                         ├→ one EAM → native .eb / ZIP / tar / compressed tar
    directory plan once ─┘

An existing encrypted .eb is unlocked and authenticated once. A directory is
captured and planned once. Target command-line order, native layout, native
compression profile, source ciphertext, and source physical Chunk order cannot
influence deterministic legacy bytes.

## Aggregate preflight

Before creating an output:

1. verify/open or plan the source;
2. canonicalize target requests by exact profile ID;
3. run each existing export analyzer;
4. fully encode each non-refused target;
5. verify each transport wrapper;
6. strict-reimport every legacy artifact;
7. determine all final names and collision state;
8. reject the complete set if any target is REFUSED or unapproved LOSSY.

Duplicate target requests collapse. A dry run performs all these steps,
including target encoding and strict re-import, and prints the canonical
migration report without writing files.

## Canonical names

For base name release, default output names are:

    release.eb
    release.zip
    release.tar
    release.tar.gz
    release.tar.zst
    release.tar.xz
    release.tar.bz2
    release.migration.json

The base-name option changes the stem only and cannot contain path separators,
colon, NUL, dot, or dot-dot. Extensions are naming conveniences; exact target
profiles remain the behavioral authority.

For a directory source, native output encodes the same EAM using the selected
INDEXED/STREAM layout and compression profile. For an existing .eb, native
output makes an exact verified-byte copy. If its requested final path is the
source path, Entrybound records a verified source-in-place relation and does
not rewrite it.

## Transaction

Every prepared artifact is written and synced to a private temporary sibling.
Only after all artifacts validate are exclusive hard links created at final
names. An existing final name always refuses the transaction. A failure during
the commit removes every final name created by the transaction and every
temporary. The canonical MigrationReport, when requested, is a member of the
same transaction.

Ordinary filesystems do not provide a portable single syscall that makes
several sibling names visible simultaneously. Entrybound guarantees failure
atomicity and no overwrite: after a reported failure it leaves no new final
artifact, and it never alters a pre-existing destination.

## MigrationReport v1

Canonical JSON records:

- source LAI, AUX, PCR, encryption and signature summary;
- conversion/preservation evidence presence;
- canonically ordered requested target profiles;
- each LOSSLESS/LOSSY/REFUSED result and structured issues;
- whether lossy output was approved;
- path, length, SHA-256, strict-reimport status, and re-import LAI;
- relation to the native artifact;
- native artifact path/hash/relation where requested;
- sidecar relationship where applicable;
- overall READY, LOSSY_APPROVAL_REQUIRED, REFUSED, PUBLISHED, or FAILED state.

For a LOSSLESS legacy target, a successful report requires reimport_lai equal
to source_lai. The report relates semantically equivalent artifacts without
claiming equivalent bytes or physical identity.

Encrypted and signed source state is explicit. Legacy targets are recorded as
unencrypted and as not embedding Entrybound signatures; that transition is not
classified as semantic LOSSY. The native .eb remains the rich artifact.

## Sidecars

    ebound sidecar release.zip
    ebound sidecar archive.tar.zst
    ebound sidecar archive.7z

The default output appends .eb to the complete source name. The original is
opened as a stable snapshot and never modified. Existing legacy import policy
is reused:

- strict for all supported formats by default;
- exact versioned ZIP compatibility and preservation policies when requested;
- no invented tar/7z compatibility mode.

Entrybound encodes the imported EAM, reopens and verifies the sidecar, and
requires its authenticated ConversionProvenance source digest to equal SHA-256
of the exact legacy input before publication. ZIP preservation retains its
existing exact-source and LOM evidence. A sidecar MigrationReport records the
legacy format/digest, import mode/profile, conflicts/resolutions, sidecar
LAI/AUX/PCR, sidecar hash, preservation status, and verification result.
