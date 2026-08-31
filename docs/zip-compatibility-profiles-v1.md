# ZIP compatibility profiles v1

Status: implemented and frozen. Compatibility is a reconciliation policy over
the same immutable `ZipObservation` used by strict import. Production code does
not execute a legacy runtime and never reparses source bytes for a profile.

## Frozen profile IDs

The initial matrix was measured on Windows x86-64. The exact identifiers and
probe versions are:

| Profile ID | Observed implementation |
|---|---|
| `zip/python-zipfile@3.13.5` | CPython `zipfile` 3.13.5 |
| `zip/java-zipfile@21.0.12.1` | Eclipse Temurin/OpenJDK `java.util.zip.ZipFile` 21.0.12.1 |
| `zip/java-zipinputstream@21.0.12.1` | Eclipse Temurin/OpenJDK `java.util.zip.ZipInputStream` 21.0.12.1 |
| `zip/libarchive-bsdtar@3.8.8` | libarchive `bsdtar` 3.8.8 |

Unversioned aliases are forbidden. A behavioral change requires a new ID even
when the upstream project retains its name.

## Observed-outcome matrix

`tools/zip-compat/observed-outcomes-v1.json` is the checked result. The
reproducible harness in `tools/zip-compat/` generates adversarial ZIP bytes and
executes the four reference readers. `regenerate_matrix.py` refuses a runtime
whose exact version differs from the frozen version.

Each case records listing, selected path, readable content length and SHA-256,
and the observed error where applicable. It includes local/central filename,
method, size and CRC disagreement; descriptor disagreement; duplicate names;
Unicode-path disagreement; directory attributes; and trailing DEFLATE bytes.

The matrix is evidence for these generalized rules:

### Python zipfile 3.13.5

- central-directory name, method, sizes, and CRC are selected;
- a local/central name mismatch is refused during member access;
- a CRC-bound Info-ZIP Unicode Path is preferred;
- descriptor claims are not semantic authority when central claims exist;
- the last duplicate member is projected;
- unused bytes after a completed DEFLATE stream are accepted.

### Java ZipFile 21.0.12.1

- central-directory name, method, and compressed extent are selected;
- primary name bytes are used rather than the Unicode Path extra;
- the complete decoded stream is the content result even when the declared
  uncompressed size is shorter;
- CRC disagreement is not enforced by the observed access path;
- the last duplicate member is projected;
- unused bytes after a completed DEFLATE stream are accepted.

### Java ZipInputStream 21.0.12.1

- local-header name, method, sizes, and CRC are selected;
- a data descriptor becomes the size/CRC authority when present;
- central-directory claims and Unicode Path extras do not choose the result;
- members are processed in local order and the last duplicate is projected;
- trailing bytes in the selected DEFLATE extent are refused.

### libarchive bsdtar 3.8.8

- local-header name is selected and a valid Unicode Path extra is preferred;
- local/central method, size, and CRC contradictions are refused;
- external file-type attributes participate in directory interpretation;
- trailing bytes in a DEFLATE extent are refused;
- duplicate extraction is modeled as the final member occupying the path. The
  probe's `-xOf` concatenation is retained as a matrix note and is not used as
  extraction semantics.

Every compatibility resolution is stored in `ConversionProvenance`, including
the profile ID, rule, chosen observation, and rejected observations.

## Safety overrides

Compatibility is semantic, not vulnerability emulation. Every profile refuses
unsafe/rooted/NUL paths, path traversal, special files unsupported by EAM,
overlapping extents, reads beyond an observed compressed extent, integer
overflow, resource-limit violations, and extraction collisions. A refusal is
reported as “compatibility interpretation blocked by Entrybound safety
invariant”; the result is never silently sanitized and called compatible.

## Limits and known unmodeled behavior

Profiles cover only structures emitted by strict ZIP parser v1: single-disk
ZIP32/ZIP64 STORE/DEFLATE, bounded extras, and supported descriptors. Encrypted,
multi-disk, unsupported-method, symlink, device, and structurally ambiguous ZIP
input remains unsupported. The profiles are corpus-backed models, not claims to
reproduce every undocumented behavior of those runtimes.

To add a profile, add generated distinguishing cases, run a pinned probe,
review its safety implications, check the matrix, implement explicit rules,
and assign a new exact-version ID. Production code must never spawn the probe.
