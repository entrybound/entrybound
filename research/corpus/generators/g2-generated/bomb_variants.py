#!/usr/bin/env python3
"""Bomb-like and near-bomb inputs, held-out construction (family F20, corpus group g2-generated).
GENERATED DATA.  A different generator from compressible_bombs.py (tuning/validation): other codecs and
container shapes, and entry-count rather than byte-count blow-ups.

  compressed/brotli-zeros-2g.br              brotli -q 5 -w 24 of 2 GiB zeros
  compressed/lz4-zeros-1g.lz4                lz4 -9 frame of 1 GiB zeros
  compressed/lzip-zeros-1g.lz                lzip -6 of 1 GiB zeros
  compressed/xz-multiblock-zeros-4g.xz       xz -6 -T1 --block-size=64MiB of 4 GiB zeros
  compressed/gzip-multimember-4096x1m.gz     4096 concatenated gzip members of 1 MiB zeros
  entries/zip-70000-empty-entries.zip        70000 empty stored entries (ZIP64 end of central directory)
  entries/tar-200000-empty-members.tar.gz    200000 empty ustar members, gzip -6
  dense/zeros-with-0.1pct-random-bytes.bin   128 MiB zeros with ~0.1% bytes replaced by random bytes
  dense/ab-pattern.txt                       64 MiB of "AB"
  dense/paragraph-repeat-mutations.txt       a 1 KiB paragraph repeated, one random byte change per ~50 KiB
params: scale (float, default 1.0; scales the dense files and the zero-stream sizes)
"""

import argparse
import gzip
import hashlib
import io
import json
import os
import random
import struct
import subprocess
import sys
import tarfile
import zipfile

MiB = 1 << 20
GiB = 1 << 30
EPOCH = int(os.environ.get("SOURCE_DATE_EPOCH", "1767225600"))
Z = bytes(16 * MiB)


class Shake:
    def __init__(self, seed, domain):
        self.prefix = domain + b"\0" + struct.pack("<Q", seed & (2**64 - 1))
        self.counter = 0

    def __call__(self, n):
        out = bytearray()
        while len(out) < n:
            take = min(n - len(out), 4 * MiB)
            out += hashlib.shake_256(self.prefix + struct.pack("<Q", self.counter)).digest(take)
            self.counter += 1
        return bytes(out)


def path_for(out, rel):
    p = os.path.join(out, rel)
    os.makedirs(os.path.dirname(p), exist_ok=True)
    return p


def pipe_zeros(cmd, total, dest):
    with open(dest, "wb") as f:
        pr = subprocess.Popen(cmd, stdin=subprocess.PIPE, stdout=f)
        left = total
        while left:
            k = min(left, len(Z))
            pr.stdin.write(Z if k == len(Z) else bytes(k))
            left -= k
        pr.stdin.close()
        if pr.wait() != 0:
            raise SystemExit(f"{cmd[0]} failed")
    os.chmod(dest, 0o644)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True)
    ap.add_argument("--seed", type=int, required=True)
    ap.add_argument("--params", default="{}")
    args = ap.parse_args()
    p = json.loads(args.params)
    s = float(p.get("scale", 1.0))
    rng = random.Random(args.seed)
    rnd = Shake(args.seed, b"ebrc-g2-f20-bomb-variants")
    out = args.out
    versions = {}
    for tool, cmd in (("brotli", ["brotli", "--version"]), ("lz4", ["lz4", "--version"]), ("lzip", ["lzip", "--version"]),
                      ("xz", ["xz", "--version"])):
        r = subprocess.run(cmd, capture_output=True, text=True)
        versions[tool] = (r.stdout or r.stderr).strip().splitlines()[0]

    pipe_zeros(["brotli", "-q", "5", "-w", "24", "-c"], int(2 * GiB * s), path_for(out, "compressed/brotli-zeros-2g.br"))
    pipe_zeros(["lz4", "-9", "-q", "-c"], int(1 * GiB * s), path_for(out, "compressed/lz4-zeros-1g.lz4"))
    pipe_zeros(["lzip", "-6", "-q", "-c"], int(1 * GiB * s), path_for(out, "compressed/lzip-zeros-1g.lz"))
    pipe_zeros(["xz", "-6", "-T1", "--block-size=64MiB", "-q", "-c"], int(4 * GiB * s),
               path_for(out, "compressed/xz-multiblock-zeros-4g.xz"))
    member = gzip.compress(bytes(MiB), compresslevel=9, mtime=0)
    with open(path_for(out, "compressed/gzip-multimember-4096x1m.gz"), "wb") as f:
        for _ in range(4096):
            f.write(member)

    zp = path_for(out, "entries/zip-70000-empty-entries.zip")
    with zipfile.ZipFile(zp, "w", compression=zipfile.ZIP_STORED, allowZip64=True) as z:
        for i in range(70000):
            zi = zipfile.ZipInfo(f"e/{i // 1000:02d}/{i:05d}", date_time=(2026, 1, 1, 0, 0, 0))
            zi.create_system = 3
            zi.external_attr = 0o100644 << 16
            z.writestr(zi, b"")
    raw = io.BytesIO()
    with tarfile.open(fileobj=raw, mode="w", format=tarfile.USTAR_FORMAT) as tf:
        for i in range(200000):
            ti = tarfile.TarInfo(f"m/{i // 5000:02d}/{i:06d}")
            ti.mtime = EPOCH
            ti.mode = 0o644
            ti.uname = ti.gname = ""
            tf.addfile(ti, io.BytesIO(b""))
    with open(path_for(out, "entries/tar-200000-empty-members.tar.gz"), "wb") as f:
        f.write(gzip.compress(raw.getvalue(), compresslevel=6, mtime=0))

    n = int(128 * MiB * s)
    with open(path_for(out, "dense/zeros-with-0.1pct-random-bytes.bin"), "wb") as f:
        for off in range(0, n, 4 * MiB):
            blk = bytearray(min(4 * MiB, n - off))
            flips = rng.sample(range(len(blk)), len(blk) // 1000)
            vals = rnd(len(flips))
            for k, pos in enumerate(flips):
                blk[pos] = vals[k] | 1
            f.write(blk)
    with open(path_for(out, "dense/ab-pattern.txt"), "wb") as f:
        f.write(b"AB" * int(32 * MiB * s))
    para = (b"Lorem-free filler paragraph for compression tests: the quick archive packs every entry, "
            b"indexes each chunk, and verifies the manifest before writing the footer. ") * 8
    para = para[:1024]
    assert len(para) == 1024
    n = int(64 * MiB * s)
    buf = bytearray(para * (n // 1024 + 1))[:n]
    pos = 0
    while True:
        pos += rng.randrange(20000, 80000)
        if pos >= n:
            break
        buf[pos] = rnd(1)[0]
    with open(path_for(out, "dense/paragraph-repeat-mutations.txt"), "wb") as f:
        f.write(buf)
    paths = []
    for r, dn, fn in os.walk(out):
        for x in fn + dn:
            paths.append(os.path.relpath(os.path.join(r, x), out))
    for i, rel in enumerate(sorted(paths, key=lambda q: (-q.count(os.sep), q))):
        fp = os.path.join(out, rel)
        if os.path.isfile(fp):
            os.chmod(fp, 0o644)
        os.utime(fp, ns=((EPOCH - 13 * i) * 10**9,) * 2)
    print(json.dumps({"tool_versions": versions}), file=sys.stderr)


if __name__ == "__main__":
    main()
