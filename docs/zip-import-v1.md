# Strict ZIP import v1

Status: implemented as `zip-strict/v1` and exposed by `ebound convert`.

## Supported structure

The production adapter parses ZIP bytes directly and independently observes:

- EOCD and, when required, the ZIP64 locator and ZIP64 EOCD;
- every central-directory entry;
- every referenced local file header and exact file-data extent;
- signed or unsigned ZIP32/ZIP64 data descriptors;
- archive and entry comments;
- all extra fields as raw framed evidence.

Ordinary single-disk ZIP32 and ZIP64 archives using STORE (method 0) or raw
DEFLATE (method 8) are supported. Multi-disk/spanned archives, encrypted ZIP,
unknown methods, overlapping extents, malformed/truncated framing, unsupported
special-file kinds, and arithmetic outside caller policy are refused. No entry
is skipped.

Central-directory, local-header, and data-descriptor values are distinct LOM
authorities. The adapter compares filename bytes, general flags, method, CRC,
compressed/uncompressed sizes, DOS timestamps, extras, attributes, disk, and
offset evidence. The central directory is not declared correct by convention.

## Extras, names, and paths

Every extra field is length-checked and retained. The adapter interprets ZIP64
extended information (`0001`), extended timestamps (`5455`), Info-ZIP Unicode
Path (`7075`), Unicode Comment (`6375`), and NTFS timestamps (`000a`). Unknown
fields and metadata outside the EAM subset remain named FidelityReport losses.

With general-purpose UTF-8 bit 11 set, primary name bytes must be valid UTF-8.
A Unicode Path value is accepted only when version 1, valid UTF-8, and its CRC
binds the exact primary name bytes. It is a Refinement when compatible and a
strict refusal when contradictory. Without either mechanism, ZIP's CP437
mapping is deterministic. Raw filename bytes remain evidence in every case.

Resolved names are split only on `/`. Strict v1 refuses NUL, absolute/rooted or
drive-qualified names, backslashes, empty internal components, `.`, and `..`.
It never normalizes an unsafe path. A trailing `/` is solely the directory
marker and is removed before constructing `LogicalPath`.

Name markers, DOS directory attributes, and Unix mode type bits are independent
kind observations. Compatible claims resolve to Directory or regular File.
Type divergence and Unix symlink/device/FIFO/socket claims are refused.

## Strict reconciliation

Strict v1 automatically resolves only compatible Omission and validated
Refinement. It refuses every Divergence affecting name/path, content method,
CRC, size, timestamps, or kind, and every Irreconcilable structure. It also
refuses duplicate logical paths and file-as-ancestor conflicts.

ZIPs often omit directory entries. The reconciler deterministically synthesizes
each missing ancestor only when no observation declares that path as a file and
there is no ancestor metadata conflict. Each synthesis is an Omission resolution
in conversion provenance; EAM still contains an explicit Directory Entry.

For every file, the resolver takes the exact structurally established compressed
extent, decodes STORE or DEFLATE under caller limits, requires the exact
reconciled uncompressed length, and independently computes and checks CRC-32.
Only verified plaintext crosses into the ordinary Entrybound CDC, deduplication,
planner, transform, and native writer pipeline. Source DEFLATE is not preserved
as an Entrybound physical plan.

## Resource policy

`ZipImportPolicy` is caller-owned and independently bounds archive bytes, entry
count, central-directory bytes, aggregate extra bytes, names/comments,
compressed and uncompressed entry bytes, total uncompressed bytes, and expansion
ratio. Every offset/length operation is checked before slicing or allocation.
Actual decoded bytes, not only declared sizes, are constrained. The CLI uses
explicit generous experimental defaults; embedders should choose narrower
limits.

## Metadata and fidelity

The initial projection records `core.executable` from compatible Unix mode
evidence and `core.mtime` only from unambiguous CRC/structure-bound UTC extended
or NTFS timestamps. DOS local time has no UTC offset, so its precision/leak is
retained as evidence and reported unavailable rather than guessed. Comments,
unknown extras, ownership/ACL/xattr-like platform attributes, and other
unsupported auxiliary classes are reported in FidelityReport. An ambiguous
semantic claim is never downgraded to a fidelity warning.

## CLI and limits

```text
ebound convert input.zip output.eb --strict
ebound convert input.zip output-stream.eb --strict --layout stream
ebound convert input.zip output.eb --strict --profile dense
```

Strict is the v1 default. Detection uses valid ZIP end/record structure rather
than the filename extension; `--from zip` is an explicit assertion. Output is a
normal unencrypted native `.eb`, so ordinary verify/list/inspect/unpack apply.
Encrypted conversion, ZIP export, compatibility-runtime profiles, preservation
mode, encrypted ZIP, and symlink/special-file EAM support are not implemented.

## Differential evidence

The generated adversarial builders are intentionally independent from the
production parser. Mainstream readers commonly privilege central-directory
values and differ on duplicate names, local/central mismatches, unsafe paths,
descriptor ambiguity, and malformed extras. Those observations motivate future
`--compat=<runtime>` profiles but are never authority for strict v1. Differential
results belong in test/research evidence and cannot silently change this policy.

The development corpus records the following qualitative differences (tool
versions are deliberately not made policy inputs):

| Generated case | Python `zipfile` | Java `ZipFile` / .NET `ZipArchive` | strict v1 |
|---|---|---|---|
| ordinary STORE/DEFLATE | lists and reads | lists and reads | imports after independent CRC/size verification |
| local/central filename mismatch | central name is listed; content read may reject the local-name mismatch | central-directory naming is generally exposed, with validation differing by runtime/version | Divergence; refuse |
| duplicate filename | duplicate entries can be listed; name lookup behavior is API-dependent | duplicate enumeration/name lookup differs by API | duplicate LogicalPath; refuse |
| `../` or rooted name | name is commonly exposed; extraction safety is caller/API dependent | normalization/extraction behavior differs | unsafe path; refuse without normalization |
| malformed extra/forged offset | errors occur at different scan/read phases | errors occur at different scan/read phases | bounded structural failure before EAM |

This matrix is evidence for later compatibility profiles, not a promise to
reproduce any listed runtime. Generated byte constructors and strict assertions
live with the adapter tests rather than as opaque fixtures.
