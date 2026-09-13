#!/usr/bin/env python3
"""Maximal-entropy files from a seeded CSPRNG (family F20, corpus group g2-generated).  GENERATED DATA.

Stands in for /dev/urandom output reproducibly: bytes are SHAKE-256(domain || seed || file index ||
block counter) in 1 MiB blocks (a standard XOF used in counter mode).

Modes (params.mode):
  files   boundary-sizes/: one file for each size in a fixed list of edge sizes (0, 1, powers of two
          +-1, 4 KiB block edges, and min/target/max +-1 of every gear-norm-v1 v2 policy, 16 MiB+1);
          log-uniform/: `count` files whose sizes are 2^U(0, log2(max_mib MiB)), fanned out into 16 dirs.
  single  one file of size_mib MiB (large incompressible object).
params: mode, count, max_mib, size_mib
"""

import argparse
import hashlib
import json
import math
import os
import random
import struct
import sys

MiB = 1 << 20
KiB = 1 << 10
EPOCH = int(os.environ.get("SOURCE_DATE_EPOCH", "1767225600"))
DOMAIN = b"ebrc-g2-f20-csprng"


def write_random(path, seed, index, size):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "wb") as f:
        block = 0
        left = size
        while left:
            take = min(left, MiB)
            f.write(hashlib.shake_256(DOMAIN + b"\0" + struct.pack("<QQQ", seed, index, block)).digest(take))
            left -= take
            block += 1
    os.chmod(path, 0o644)


def boundary_sizes():
    s = {0, 1, 2, 3, 7, 8, 15, 16, 31, 32, 63, 64, 127, 128, 255, 256, 511, 512, 1023, 1024, 4095, 4096, 4097,
         8191, 8192, 8193, 65535, 65536, 65537, MiB - 1, MiB, MiB + 1, 16 * MiB + 1}
    for mn, tg, mx in ((512 * KiB, 2 * MiB, 8 * MiB), (128 * KiB, 512 * KiB, 2 * MiB), (64 * KiB, 256 * KiB, MiB),
                       (32 * KiB, 128 * KiB, 512 * KiB)):
        for v in (mn, tg, mx):
            s.update((v - 1, v, v + 1))
    return sorted(s)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True)
    ap.add_argument("--seed", type=int, required=True)
    ap.add_argument("--params", default="{}")
    args = ap.parse_args()
    p = json.loads(args.params)
    rng = random.Random(args.seed)
    out = args.out
    total = 0
    idx = 0
    if p.get("mode") == "files":
        for size in boundary_sizes():
            write_random(os.path.join(out, "boundary-sizes", f"size-{size:09d}.bin"), args.seed, idx, size)
            idx += 1
            total += size
        lmax = math.log2(int(p.get("max_mib", 16)) * MiB)
        for i in range(int(p.get("count", 300))):
            size = int(2 ** rng.uniform(0, lmax))
            write_random(os.path.join(out, "log-uniform", f"{i % 16:x}", f"r{i:05d}.bin"), args.seed, idx, size)
            idx += 1
            total += size
    elif p.get("mode") == "single":
        size = int(p["size_mib"]) * MiB
        write_random(os.path.join(out, f"random-{p['size_mib']}m.bin"), args.seed, idx, size)
        total = size
    else:
        raise SystemExit("csprng_files: params.mode must be files or single")
    paths = []
    for r, dn, fn in os.walk(out):
        for n in fn + dn:
            paths.append(os.path.relpath(os.path.join(r, n), out))
    for i, rel in enumerate(sorted(paths, key=lambda s: (-s.count(os.sep), s))):
        os.utime(os.path.join(out, rel), ns=((EPOCH - 17 * i) * 10**9,) * 2)
    print(json.dumps({"files": idx if idx else 1, "bytes": total}), file=sys.stderr)


if __name__ == "__main__":
    main()
