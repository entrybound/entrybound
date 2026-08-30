# Reconstructive transform v1

Status: experimental frozen implementation note for the v5 planners.

## Dependency and safety decision

Entrybound pins Microsoft `preflate-rs` 0.7.6 for bit-exact DEFLATE
reconstruction. The crate is Apache-2.0, forbids unsafe Rust, exposes exact
stream recreation, and accepts explicit plaintext and hash-chain bounds. It
requires Rust 1.89, so Entrybound's MSRV is 1.89. `crc32fast` 1.5.0 validates
gzip payload checksums; `flate2` 1.1.9 is test-only and generates fixtures.

Entrybound does not implement a DEFLATE parser. Archive strings dispatch only
through the governed transform registry.

## Reversibility classes and pipeline rule

`delta8/v1` and `byte-shuffle/v1` are Structural: they accept every byte
string, preserve length, and are invertible by construction.

`deflate-reconstruct/v1` is Reconstructive: it is format-specific, changes
length, and requires side data. A v5 pipeline may have at most one
reconstructive step, it must be first in forward order, and zero or more
Structural steps may follow. The codec remains last. Inverse decoding runs the
codec, reverses Structural steps, reconstructs the original DEFLATE bytes, and
then verifies the original plaintext Chunk length and SHA-256 digest.

## Eligibility and exact verification

V1 attempts only a complete Chunk containing one of:

- raw DEFLATE that preflate consumes completely and recreates exactly;
- zlib with DEFLATE method, a valid header, no preset dictionary, and a valid
  Adler-32 trailer;
- one gzip member with a bounded valid header, DEFLATE method, and valid CRC-32
  and ISIZE trailer.

It does not scan PNG, ZIP, arbitrary embedded regions, concatenated gzip
members, or streams crossing Chunk boundaries. A failed or ambiguous attempt
falls back to the ordinary v4-compatible candidates.

For every candidate the writer performs:

```text
original -> preflate plaintext + corrections -> recreated
```

and requires both `SHA256(original) == SHA256(recreated)` and byte-for-byte
equality. The native ECF writer repeats this operation against recorded side
data, so a caller cannot commit an unverified reconstructive EAM. This check
cannot be disabled.

## ReconstructionData

ReconstructionData is physical, digest-addressed by SHA-256 of its exact bytes,
stored once, and referenced by TransformStep v2. It records format
`preflate-rs-0.7.6/deflate-reconstruct-v1` and the intermediate length.

Its exact bytes are:

```text
"ERD1" | wrapper:u8 | reserved:[u8;3]
prefix_length:u32be | correction_length:u64be | suffix_length:u32be
prefix | preflate_corrections | suffix
```

Wrapper values are raw=0, zlib=1, gzip=2. Prefix and suffix preserve the exact
container bytes; the correction stream recreates the exact raw DEFLATE body.

Feature bit `0x4` requires bits `0x1` and `0x2`, adds canonical record type 14
and TransformStep v2 record type 15, and selects the section sequence documented
in `format-v0.md`. Historical type-13 steps and v1–v4 sections are unchanged.
The same section may contain canonical type-16 ReconstructionFallback audit
records after all ReconstructionData objects. They associate a Chunk digest
with either `unrecognized-or-verification-failed` or `complete-cost-rejected`.
These records are non-authoritative physical planning evidence, contain no
alternate Chunk meaning, and participate only in section integrity and PCI.

## Frozen v5 planning

All v5 profiles retain their corresponding frozen v4 candidates, CDC,
deduplication, similarity, Dictionary, and bounded-group behavior.

| Planner | Recognition | Reconstructive candidates |
|---|---|---|
| `fast-v5` | none | none; lookback remains zero |
| `balanced-v5` | complete bounded streams, max chain 512 | reconstruction → Zstandard levels 3 and 5; lookback zero |
| `dense-v5` | complete bounded streams, max chain 2048 | reconstruction → Zstandard 9; LZMA2 preset 6 / 4 MiB; delta8 → Zstandard 9 |
| `extreme-v5` | complete bounded streams, max chain 4096 | Zstandard 15/19; LZMA2 preset 9 / 8 MiB; delta8 → Zstandard 15; byte-shuffle 2/4/8 → LZMA2 preset 9 / 8 MiB |

A candidate cost is encoded payload + canonical TransformPlan v2 + canonical
ReconstructionData record + 64 bytes of section framing. It must beat the best
ordinary v4 representation by strictly more than both 256 bytes and 2%.
Ordinary encoding wins ties. Existing cohort planning then compares its full
Dictionary/lookback cost against this independent result.

`explain` reads the persisted type-16 records to report every non-selected v5
attempt as a fallback, split between recognition/verification failures and
complete-cost rejections. It also reports selected gross payload savings,
ReconstructionData overhead, and net savings.

## Resource limits and identity

V1 limits original input and ReconstructionData to 16 MiB each, intermediate
plaintext to 64 MiB, expansion to 64x, and declares an additional 80 MiB
working set on every selected reconstructive plan. The caller's archive and
decode policies are checked before payload decoding. Lengths are checked before
allocation where available, wrapper parsing is bounded, and malformed state
uses reconstructive-specific typed diagnostics.

The original pre-transform bytes remain Chunk plaintext identity.
ReconstructionData, transform parameters, and codecs do not enter
ContentObject identity, LAI, AUX, or PCR. With unchanged chunking and metadata,
only PCI changes when this physical representation is selected.
