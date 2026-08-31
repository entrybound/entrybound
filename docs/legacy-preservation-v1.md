# Legacy preservation v1

Status: implemented for ZIP import.

Preservation is a compatibility projection plus forensic evidence:

```text
ZIP bytes -> immutable LOM -> named compatibility profile -> EAM
          \_______________________________________________/
                    auxiliary preservation evidence
```

`--preserve` therefore requires an exact `--compat=zip/<runtime>@<version>`.
The profile decides the one coherent EAM projection. Preservation never makes
foreign declarations into EAM authority.

## Preserved authority

The preservation object contains:

- the exact source ZIP byte sequence and its SHA-256;
- every archive- and entry-scoped LOM field in parser order;
- raw and interpreted values, authority, source range, and validity state;
- every classified conflict and policy-independent resolution;
- every selected compatibility resolution;
- source format and preservation-format version.

The exact snapshot is the byte authority for unknown structures, gaps, padding,
comments, original ordering, and future exact-source recovery. Structured LOM
records make evidence searchable and permit a future alternate projection
without reparsing the native archive.

## Native wire

Required incompatibility feature `0x4000` is
`legacy-preservation-v1`. It requires `conversion-provenance-v1` (`0x2000`).
The feature adds these version-1 canonical records after type 28 in the single
Fidelity/auxiliary payload:

| Type | Record | Canonical role |
|---:|---|---|
| 30 | LegacyPreservation | format, source format/digest/bytes, ordered observation/conflict/resolution streams |
| 31 | PreservedObservation | scope, subject/observation ordinals, field, authority, raw/interpreted values, location, validity |
| 32 | PreservedConflict | ordinal, field, authority/value/location streams, class, optional LOM resolution |
| 33 | PreservedAuthority | format, structure, instance |
| 34 | PreservedValue | explicit value kind and exactly one typed value |
| 35 | PreservedLocation | source offset and length |
| 36 | PreservedLOMResolution | action and optional selected authority |

Observation order is `(scope, subject_ordinal, observation_ordinal)` and conflict
order is the explicit ordinal. Selected type-29 compatibility resolutions remain
unique canonical lexical order. Duplicate/out-of-order evidence, source-digest
mismatch, a preservation record without both feature bits, or trailing records
are nonconforming. Historical type-28 strict records are unchanged.

All nested collections in tags 5-7 and conflict tags 3-5 are raw concatenations
of complete canonical records; each inner 16-byte record header is the sole item
length and type dispatch. Empty collections are zero bytes. Nesting is closed by
the outer record tag: observation streams allow only type 31, conflict streams
only type 32, authority streams only type 33, value streams only type 34,
location streams only type 35, and selected-resolution streams only type 29.
Unknown types and trailing bytes fail closed. Type 34 tag 1 selects bytes(1),
u64(2), i64(3), UTF-8(4), or bool(5), and tag 2 must have that exact canonical
field type. Observation scope is archive(0) or entry(1); validity is valid(1),
invalid(2), or uninterpreted(3).

INDEXED and STREAM encode the same object. Readers that do not implement
`0x4000` fail closed instead of discarding evidence.

## Identity and recovery

Preservation is auxiliary:

- LAI is unchanged for the same EAM projection;
- PCR is unchanged for the same chunk organization;
- AUX changes because exact source and LOM evidence are hashed under the
  `legacy-preservation/v1` and `aux-preservation/v1` domains;
- PCI normally changes with the added records.

`legacy::zip::recover_preserved_source` returns the exact snapshot only after
archive validation and SHA-256 agreement with both the preservation object and
ConversionProvenance. It is source recovery, not ZIP export.

## Resource limits

The caller bounds observations globally and per subject, conflicts,
resolutions, encoded conversion/preservation bytes, and exact source bytes.
Limits are checked while parsing/building evidence and again before native
planning. Preserve defaults are finite; absence of a caller override never
means unlimited evidence.

The CLI's experimental defaults are 8,000,000 observations, 4,096 observations
per subject, 1,000,000 conflicts, 1,000,000 resolutions, 256 MiB total encoded
conversion/preservation evidence, and 4 GiB exact-source bytes. The archive,
entry, decompression, and expansion limits in `ZipImportPolicy` also apply.
