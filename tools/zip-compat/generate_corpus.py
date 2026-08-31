#!/usr/bin/env python3
"""Generate the small adversarial ZIP corpus used by compatibility probes.

This is development tooling, never production parser code. Every case is built
from explicit ZIP records so the intended disagreement is reviewable.
"""

from __future__ import annotations

import argparse
import binascii
import pathlib
import struct
import zlib


LOCAL = 0x04034B50
CENTRAL = 0x02014B50
EOCD = 0x06054B50
DESCRIPTOR = 0x08074B50


def raw_deflate(data: bytes) -> bytes:
    compressor = zlib.compressobj(6, zlib.DEFLATED, -15)
    return compressor.compress(data) + compressor.flush()


def one_entry(
    *,
    local_name: bytes = b"a.txt",
    central_name: bytes | None = None,
    content: bytes = b"payload",
    method: int = 0,
    local_method: int | None = None,
    flags: int = 0,
    local_crc: int | None = None,
    central_crc: int | None = None,
    local_compressed_size: int | None = None,
    local_uncompressed_size: int | None = None,
    central_compressed_size: int | None = None,
    central_uncompressed_size: int | None = None,
    descriptor: tuple[int, int, int] | None = None,
    local_extra: bytes = b"",
    central_extra: bytes | None = None,
    external: int = 0,
    trailing_compressed: bytes = b"",
) -> bytes:
    central_name = local_name if central_name is None else central_name
    central_extra = local_extra if central_extra is None else central_extra
    encoded = raw_deflate(content) if method == 8 else content
    encoded += trailing_compressed
    crc = binascii.crc32(content) & 0xFFFFFFFF
    local_crc = crc if local_crc is None else local_crc
    central_crc = crc if central_crc is None else central_crc
    local_method = method if local_method is None else local_method
    lc = len(encoded) if local_compressed_size is None else local_compressed_size
    lu = len(content) if local_uncompressed_size is None else local_uncompressed_size
    cc = len(encoded) if central_compressed_size is None else central_compressed_size
    cu = len(content) if central_uncompressed_size is None else central_uncompressed_size
    if descriptor is not None:
        flags |= 0x0008
        local_crc = local_compressed_size = local_uncompressed_size = 0
        lc = lu = 0

    local = struct.pack(
        "<IHHHHHIIIHH",
        LOCAL,
        20,
        flags,
        local_method,
        0,
        0,
        local_crc,
        lc,
        lu,
        len(local_name),
        len(local_extra),
    ) + local_name + local_extra + encoded
    if descriptor is not None:
        local += struct.pack("<IIII", DESCRIPTOR, *descriptor)

    central = struct.pack(
        "<IHHHHHHIIIHHHHHII",
        CENTRAL,
        0x0314,
        20,
        flags,
        method,
        0,
        0,
        central_crc,
        cc,
        cu,
        len(central_name),
        len(central_extra),
        0,
        0,
        0,
        external,
        0,
    ) + central_name + central_extra
    eocd = struct.pack("<IHHHHIIH", EOCD, 0, 0, 1, 1, len(central), len(local), 0)
    return local + central + eocd


def duplicate_names() -> bytes:
    first = one_entry(local_name=b"dup", content=b"first")
    second = one_entry(local_name=b"dup", content=b"second")
    # Rebuild from each local/central record with corrected offsets.
    first_cd = first.find(struct.pack("<I", CENTRAL))
    second_cd = second.find(struct.pack("<I", CENTRAL))
    local1 = first[:first_cd]
    central1 = bytearray(first[first_cd:-22])
    local2 = second[:second_cd]
    central2 = bytearray(second[second_cd:-22])
    central2[42:46] = struct.pack("<I", len(local1))
    central = bytes(central1) + bytes(central2)
    eocd = struct.pack(
        "<IHHHHIIH", EOCD, 0, 0, 2, 2, len(central), len(local1) + len(local2), 0
    )
    return local1 + local2 + central + eocd


def unicode_extra(primary: bytes, value: bytes) -> bytes:
    body = b"\x01" + struct.pack("<I", binascii.crc32(primary) & 0xFFFFFFFF) + value
    return struct.pack("<HH", 0x7075, len(body)) + body


def cases() -> dict[str, bytes]:
    payload = b"payload"
    crc = binascii.crc32(payload) & 0xFFFFFFFF
    return {
        "ordinary-store": one_entry(),
        "ordinary-deflate": one_entry(method=8, content=b"compressible " * 20),
        "local-central-name": one_entry(local_name=b"local", central_name=b"central"),
        "local-central-method": one_entry(method=0, local_method=8),
        "local-crc": one_entry(local_crc=crc ^ 1),
        "central-crc": one_entry(central_crc=crc ^ 1),
        "local-size-short": one_entry(local_uncompressed_size=len(payload) - 1),
        "central-size-short": one_entry(central_uncompressed_size=len(payload) - 1),
        "descriptor-crc": one_entry(descriptor=(crc ^ 1, len(payload), len(payload))),
        "duplicate-name": duplicate_names(),
        "unsafe-parent": one_entry(local_name=b"../escape"),
        "unicode-conflict": one_entry(
            local_name=b"primary",
            flags=0x0800,
            local_extra=unicode_extra(b"primary", b"different"),
        ),
        "directory-type": one_entry(
            local_name=b"plain",
            content=b"",
            external=0o040755 << 16,
        ),
        "deflate-trailing": one_entry(
            method=8,
            content=b"deflate-data" * 20,
            trailing_compressed=b"TRAILING",
        ),
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("output", type=pathlib.Path)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    for name, data in cases().items():
        (args.output / f"{name}.zip").write_bytes(data)


if __name__ == "__main__":
    main()
