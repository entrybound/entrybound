#!/usr/bin/env python3
"""Helper (g4-binary F16): rewrite image files so their sparse layout is a pure function of content.

    python sparsify.py <file> [<file> ...]

Why: mke2fs, qemu-img and similar tools may leave *unwritten* (preallocated, zero-reading) extents
in the host file.  ext4 reports those as holes or as data for SEEK_DATA/SEEK_HOLE depending on page
cache state, so the corpus fingerprint's extent map (field d) would change after the file is read.
`fallocate --dig-holes` does not help because it skips ranges SEEK_DATA already calls holes.
This helper reads every 4 KiB block through the page cache (unwritten ranges read as zeros), writes
only non-zero blocks into a fresh file of the same size (all other ranges become real holes) and
atomically replaces the original, keeping its permission bits.  The result has no unwritten extents
and its data/hole map depends only on which 4 KiB blocks are non-zero.
"""

import os
import stat
import sys

BLOCK = 4096


def sparsify(path: str) -> tuple[int, int]:
    st = os.stat(path)
    size = st.st_size
    tmp = path + ".sparsify.tmp"
    written = 0
    zero = bytes(BLOCK)
    with open(path, "rb", buffering=0) as src, open(tmp, "wb", buffering=0) as dst:
        off = 0
        while off < size:
            chunk = src.read(min(64 * BLOCK * 256, size - off))  # 256 MiB windows
            if not chunk:
                raise SystemExit(f"sparsify: short read in {path} at {off}")
            for i in range(0, len(chunk), BLOCK):
                blk = chunk[i:i + BLOCK]
                if blk != zero[:len(blk)]:
                    dst.seek(off + i)
                    dst.write(blk)
                    written += len(blk)
            off += len(chunk)
        dst.truncate(size)
        os.fsync(dst.fileno())
    os.chmod(tmp, stat.S_IMODE(st.st_mode))
    os.replace(tmp, path)
    return size, written


def main():
    if len(sys.argv) < 2:
        raise SystemExit(__doc__)
    for p in sys.argv[1:]:
        size, written = sparsify(p)
        print(f"sparsify: {os.path.basename(p)} size={size} nonzero_block_bytes={written}")


if __name__ == "__main__":
    main()
