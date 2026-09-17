#!/usr/bin/env python3
"""Generated structurally-adversarial archives for family F20 (corpus group g2-generated).
GENERATED DATA. Held-out only: a structurally distinct generator from malicious_archives.py (used
for tuning/validation) -- different container formats and different tricks within the zip/tar ones
it shares, so held-out is not just a reseed of the tuning/validation construction.

Produces:
  zip_reserved_names.zip     entries named after Windows-reserved device names (CON, NUL, AUX,
                              COM1, LPT1) and a zip "alternate data stream"-style name containing a
                              colon (name:stream) -- extractor/platform-specific edge cases distinct
                              from malicious_archives.py's path-traversal focus
  zip_symlink_loop.zip       two symlink entries that point to each other (a -> b, b -> a): a
                              self-referential loop rather than an outward escape
  zip64_duplicate_names.zip  duplicate-name entries (like malicious_archives.py's, but built with
                              Zip64 extra fields present) so a parser that only checks "is this
                              Zip64" behaves differently than one that also checks Zip64 field
                              consistency
  tar_pax_traversal.tar      a PAX (not USTAR) member whose *extended attribute* path.name escapes
                              the base directory even though the ustar-compatible name field alone
                              looks safe -- a different traversal vector from malicious_archives.py
                              (which uses the plain USTAR name field directly)
  malformed_7z_header.7z     a syntactically-invalid 7z file: correct 6-byte signature and version,
                              but a NextHeaderOffset/NextHeaderSize pointing past end-of-file
  malformed_rar4_header.rar  a syntactically-invalid RAR file: correct RAR4 signature, a MAIN_HEAD
                              block whose declared HEAD_SIZE extends past end-of-file

Params (EB_PARAMS_JSON): {"marker": <str>}
"""

import argparse
import io
import json
import os
import struct
import tarfile
import time
import zlib
from pathlib import Path

EPOCH = int(os.environ.get("SOURCE_DATE_EPOCH", "1767225600"))


def dos_datetime(epoch: int) -> tuple:
    t = time.gmtime(epoch)
    return ((t.tm_hour << 11) | (t.tm_min << 5) | (t.tm_sec // 2),
            (((t.tm_year - 1980) & 0x7F) << 9) | (t.tm_mon << 5) | t.tm_mday)


def zip_entry(name, data, *, external_attr=0o644 << 16, symlink_target=None, zip64=False):
    body = symlink_target.encode() if symlink_target is not None else data
    ext = external_attr if symlink_target is None else (0o120777 << 16)
    return {"name": name, "data": body, "external_attr": ext, "crc": zlib.crc32(body) & 0xFFFFFFFF,
           "zip64": zip64}


def write_zip(path: Path, entries: list) -> None:
    dos_time, dos_date = dos_datetime(EPOCH)
    buf, central = bytearray(), bytearray()
    for e in entries:
        name_b = e["name"].encode()
        offset = len(buf)
        extra = b""
        version_needed = 20
        if e["zip64"]:
            # Zip64 extra field claiming 8-byte sizes, even though this entry is tiny -- a parser
            # that trusts "has a Zip64 extra field" as a signal without checking the sizes it
            # actually carries (or its declared length) can be confused by this.
            extra = struct.pack("<HHQQ", 0x0001, 16, len(e["data"]), len(e["data"]))
            version_needed = 45
        buf += struct.pack("<IHHHHHIIIHH", 0x04034B50, version_needed, 0, 0, dos_time, dos_date,
                          e["crc"], len(e["data"]), len(e["data"]), len(name_b), len(extra))
        buf += name_b + extra + e["data"]
        central += struct.pack("<IHHHHHHIIIHHHHHII", 0x02014B50, version_needed, version_needed, 0,
                              0, dos_time, dos_date, e["crc"], len(e["data"]), len(e["data"]),
                              len(name_b), len(extra), 0, 0, 0, e["external_attr"], offset)
        central += name_b + extra
    cd_offset = len(buf)
    eocd = struct.pack("<IHHHHIIH", 0x06054B50, 0, 0, len(entries), len(entries), len(central),
                       cd_offset, 0)
    path.write_bytes(bytes(buf) + bytes(central) + eocd)


def build_zip_reserved_names(out: Path, marker: str) -> None:
    names = ["CON", "NUL", "AUX", "COM1", "LPT1", "notes.txt:hidden-stream"]
    entries = [zip_entry(n, f"{marker}:{n}\n".encode()) for n in names]
    write_zip(out / "zip_reserved_names.zip", entries)


def build_zip_symlink_loop(out: Path, marker: str) -> None:
    entries = [zip_entry("a", b"", symlink_target="b"), zip_entry("b", b"", symlink_target="a")]
    write_zip(out / "zip_symlink_loop.zip", entries)


def build_zip64_duplicate_names(out: Path, marker: str) -> None:
    entries = [
        zip_entry("payload.bin", f"{marker}:first (zip64 extra)\n".encode(), zip64=True),
        zip_entry("payload.bin", f"{marker}:second (zip64 extra), same name\n".encode(), zip64=True),
    ]
    write_zip(out / "zip64_duplicate_names.zip", entries)


def build_tar_pax_traversal(out: Path, marker: str) -> None:
    with tarfile.open(out / "tar_pax_traversal.tar", "w", format=tarfile.PAX_FORMAT) as tf:
        ti = tarfile.TarInfo(name="safe-looking-name.txt")
        ti.mtime = EPOCH
        data = f"{marker}:pax path override escapes via extended attribute\n".encode()
        ti.size = len(data)
        ti.pax_headers = {"path": "../../../tmp/eb-f20-pax-escape.txt"}
        tf.addfile(ti, io.BytesIO(data))


def build_malformed_7z(out: Path) -> None:
    # Real 7z layout: 6-byte signature, 2-byte version, 4-byte StartHeaderCRC, then a 20-byte
    # StartHeader (NextHeaderOffset u64, NextHeaderSize u64, NextHeaderCRC u32). We set
    # NextHeaderOffset/Size to point well past the (tiny) file we actually write.
    sig = b"7z\xbc\xaf\x27\x1c" + bytes([0, 4])
    next_header_offset = 1 << 20  # 1 MiB past a file that will be a few dozen bytes long
    next_header_size = 64
    start_header = struct.pack("<QQI", next_header_offset, next_header_size, 0)
    start_header_crc = zlib.crc32(start_header) & 0xFFFFFFFF
    (out / "malformed_7z_header.7z").write_bytes(sig + struct.pack("<I", start_header_crc) + start_header)


def build_malformed_rar4(out: Path) -> None:
    # RAR4 signature, then a MAIN_HEAD block (HEAD_CRC u16, HEAD_TYPE u8=0x73, HEAD_FLAGS u16,
    # HEAD_SIZE u16) whose HEAD_SIZE claims far more bytes than actually follow.
    sig = b"Rar!\x1a\x07\x00"
    head_size = 0x7FFF  # declares ~32 KiB of header; file ends a few bytes later
    main_head = struct.pack("<HBHH", 0, 0x73, 0, head_size)
    (out / "malformed_rar4_header.rar").write_bytes(sig + main_head)


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True)
    ap.add_argument("--seed", type=int, required=True)
    ap.add_argument("--params", default="{}")
    args = ap.parse_args()
    params = json.loads(args.params)
    out = Path(args.out)
    marker = params.get("marker", "eb-f20-heldout")

    build_zip_reserved_names(out, marker)
    build_zip_symlink_loop(out, marker)
    build_zip64_duplicate_names(out, marker)
    build_tar_pax_traversal(out, marker)
    build_malformed_7z(out)
    build_malformed_rar4(out)


if __name__ == "__main__":
    main()
