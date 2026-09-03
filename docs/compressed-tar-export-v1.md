# Compressed tar export v1

Status: implemented and frozen.

## Composition and outcomes

The four exact profiles are tar.gz/pax-v1, tar.zst/pax-v1, tar.xz/pax-v1,
and tar.bz2/pax-v1. Each is the composition:

    verified EAM
    → tar/pax-v1 analysis and bytes
    → deterministic transport wrapper

There is no separate representability analyzer for a wrapper. The wrapped
profile inherits the exact tar/pax-v1 issue list and
LOSSLESS/LOSSY/REFUSED outcome. Compression is physical transport and never
introduces a semantic-loss issue.

## Frozen wrapper parameters

tar.gz/pax-v1 is one RFC 1952 member. The payload is raw DEFLATE level 6. The
header has CM=8, FLG=0, MTIME=0, XFL=0, OS=255, and no extra field, filename,
comment, or header CRC. Its trailer contains CRC-32 of the exact tar bytes and
their length modulo 2^32.

tar.zst/pax-v1 is one standard Zstandard frame produced by pinned zstd 0.13.3
at level 9. Content size and checksum are present; dictionary ID is absent; no
dictionary, long-distance matching, or multithread-dependent framing is used.
The pledged source size is the exact tar length.

tar.xz/pax-v1 is one XZ stream produced by pinned lzma-rust2 0.20.0.
XzOptions preset 6 freezes the LZMA2 configuration, one block is emitted, and
the integrity check is CRC64.

tar.bz2/pax-v1 is one bzip2 stream produced by pinned bzip2 0.6.1 with block
level 9.

None of the profiles records source filenames, host identity, current time,
random bytes, or thread-count-dependent encoder state. The same tar/pax-v1
bytes and profile produce byte-identical wrapped bytes.

## Verification

Encoder completion is insufficient for publication. Entrybound passes every
generated wrapper through its existing strict transport reader and requires:

    wrapper member/frame/stream count == 1
    decoded bytes == exact tar/pax-v1 bytes

It then composes the corresponding strict transport import with the strict tar
import. A LOSSLESS artifact is publishable only when strict re-import LAI
equals source LAI. An accepted LOSSY artifact must differ only where its typed
export issues predict.

## Receipt

Bare ZIP and tar continue using the byte-compatible
entrybound/export-receipt-v1. A wrapped target uses
entrybound/export-receipt-v2. Version 2 adds:

- semantic target (tar/pax-v1);
- transport target (gzip-v1, zstd-v1, xz-v1, or bzip2-v1);
- inner tar byte length and SHA-256;
- strict-reimport validation state and re-import LAI.

The final target length and SHA-256 always describe the wrapped artifact.
Receipt v2 does not reinterpret or invalidate receipt v1.
