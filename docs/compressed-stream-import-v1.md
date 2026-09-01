# Compressed stream import v1

Status: implemented strict transport adapters for gzip, Zstandard, XZ, and
bzip2. These adapters are composed with the one tar observer; there are no
separate `tar.gz`, `tar.zst`, `tar.xz`, or `tar.bz2` parsers.

## Layered observation

For a wrapped tar source, the outer transport is LOM layer 0 and decoded tar is
layer 1. Evidence offsets are relative to the byte space named by each layer's
authority. Conversion provenance retains the exact outer source digest, layer
order, wrapper member count, SHA-256 of the exact decoded child, wrapper
integrity result, and tar reconciliation decisions. Wrapper filename/comment
or frame fields never become Entry paths or EAM metadata.

The processing sequence is:

```text
outer bytes -> bounded transport observation and verified decode
            -> decoded-child SHA-256 and structural tar detection
            -> the same tar observer/resolver used for bare tar
            -> ordinary native planning and ECF
```

## Implemented transports

- **gzip / RFC 1952** uses pinned `flate2 1.1.9`. Every concatenated member has
  independently parsed flags, MTIME, XFL, OS, FEXTRA, FNAME, FCOMMENT and
  FHCRC evidence. Reserved flags, header CRC, DEFLATE completion, CRC-32 and
  ISIZE are checked before accepting member output.
- **Zstandard** uses the existing pinned `zstd 0.13.3` stack. Standard and
  concatenated frames are decoded one at a time; frame checksums are enforced.
  Skippable frames are retained as explicit observations. Frames requiring an
  external dictionary are refused. Declared content size and decoder window
  are policy-checked before decode.
- **XZ** enables the `xz` feature of the existing pure-Rust, MIT/Apache-2.0
  `lzma-rust2 0.20.0`. It accepts standard concatenated streams and legal
  stream padding, validates stream/block checks, and applies a caller memory
  limit to dictionary/filter allocation. Unsupported filters/checks fail.
- **bzip2** uses pinned `bzip2 0.6.1` (MIT/Apache-2.0), whose default backend is
  the pure-Rust `libbz2-rs-sys`; Entrybound adds no unsafe code. Concatenated
  members are decoded independently, member CRC/combined CRC failures are
  rejected, and the block-size-derived working set is policy checked.

## Limits and projections

`WrapperImportPolicy` bounds compressed source bytes, member/frame/stream
count, wrapper metadata, decoded-child and aggregate decoded bytes, expansion
ratio, and decoder window/dictionary memory. Limits apply during decoding, not
only after a `Vec` has grown. A wrapped tar must also satisfy every inner
`TarImportPolicy` bound.

If the verified decoded child is structurally tar, it is always sent to the
tar adapter. Otherwise conversion is permitted only with an explicit safe
`--entry-name`; that path becomes one native regular file and its missing
ancestors are synthesized explicitly. FNAME, the input filename, and other
wrapper metadata are never implicit path authorities.

```sh
ebound convert archive.tar.gz archive.eb --strict
ebound convert archive.tar.zst archive.eb --strict
ebound convert archive.tar.xz archive.eb --strict
ebound convert archive.tar.bz2 archive.eb --strict

ebound convert payload.zst payload.eb --from zstd \
  --entry-name payload.bin --strict
```

Byte/structure detection is authoritative; an explicit `--from` must agree
with actual framing. Accepted values are `tar`, `gzip`, `zstd`, `xz`, `bzip2`,
`tar.gz`, `tar.zst`, `tar.xz`, and `tar.bz2`. Compatibility and preservation
remain ZIP-only until versioned runtime matrices exist for these formats.
