#!/usr/bin/env python3
"""Generator (g4-binary F10): synthetic highly redundant binary files.  GENERATED DATA.

Contract: python gen_redundant_blobs.py --out <staging> --seed N --params <JSON>
params: profile ("mixed-v1"), scale (float multiplier on sizes, default 1.0)

Files (sizes at scale 1.0, ~12.5 MiB total):
  zeros-1m.bin                 all-zero
  flash-nor-2m.bin             header + pseudo-compressed kernel + 0xFF erase-block padding + rootfs + 0xFF tail
  flash-nor-2m-variant.bin     same layout, kernel differs by a few hundred bytes, rootfs shifted by 1 byte
  records-fixed-3m.bin         256-byte records: incrementing id/timestamp, mostly constant fields
  records-fixed-1m-other.bin   same schema, different id range and one changed field
  periodic-prime-1m.bin        random 4093-byte period repeated (not aligned to 4 KiB)
  shifted-repeats-2m.bin       128 KiB random chunk repeated 16x with random-length inserts between copies
  sparse-dense-1m.bin          zero background with small random islands (dense file, no holes)
  bitmap-runs-1m.bin           run-length structured 0x00/0xFF/0x55/0xAA bitmap
  utf16-string-table-512k.bin  repeated UTF-16LE identifier strings with small counters
All bytes derive from random.Random(seed); output is a pure function of (script, seed, params).
"""

import argparse
import json
import os
import random
import struct


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True)
    ap.add_argument("--seed", type=int, required=True)
    ap.add_argument("--params", default="{}")
    a = ap.parse_args()
    p = json.loads(a.params)
    if p.get("profile", "mixed-v1") != "mixed-v1":
        raise SystemExit("unknown profile")
    k = float(p.get("scale", 1.0))
    rng = random.Random(a.seed)
    KiB, MiB = 1024, 1024 * 1024

    def sz(n):
        return max(4096, int(n * k))

    def write(name, data):
        path = os.path.join(a.out, name)
        with open(path, "wb") as f:
            f.write(data)
        os.chmod(path, 0o644)

    write("zeros-1m.bin", bytes(sz(1 * MiB)))

    def flash(kernel, rootfs, total, erase=64 * KiB, shift=0):
        hdr = b"EBFW" + struct.pack("<IIII", 1, len(kernel), len(rootfs), 0xDEADBEEF) + b"\x00" * 44
        img = bytearray(b"\xff" * total)
        img[0:len(hdr)] = hdr
        img[256:256 + len(kernel)] = kernel
        off = ((256 + len(kernel) + erase - 1) // erase) * erase + shift
        img[off:off + len(rootfs)] = rootfs
        return bytes(img)

    kernel = rng.randbytes(sz(300 * KiB))
    rootfs = rng.randbytes(sz(500 * KiB))
    write("flash-nor-2m.bin", flash(kernel, rootfs, sz(2 * MiB)))
    k2 = bytearray(kernel)
    for _ in range(300):
        k2[rng.randrange(len(k2))] = rng.randrange(256)
    write("flash-nor-2m-variant.bin", flash(bytes(k2), rootfs, sz(2 * MiB), shift=1))

    def records(n_bytes, id0, flag):
        out = bytearray()
        ts = 1_700_000_000
        i = 0
        while len(out) < n_bytes:
            ts += rng.choice((1, 1, 1, 2, 5))
            body = struct.pack("<QQIHH", id0 + i, ts, flag, i % 7, 0) + b"STATUS=OK;REGION=eu-west-1;" + b"\x00" * 16
            payload = rng.randbytes(8) if i % 50 == 0 else b"\x00" * 8
            rec = (body + payload).ljust(256, b"\x20")
            out += rec
            i += 1
        return bytes(out[:n_bytes])

    write("records-fixed-3m.bin", records(sz(3 * MiB), 1, 0x11))
    write("records-fixed-1m-other.bin", records(sz(1 * MiB), 900_000, 0x12))

    period = rng.randbytes(4093)
    n = sz(1 * MiB)
    write("periodic-prime-1m.bin", (period * (n // len(period) + 1))[:n])

    chunk = rng.randbytes(sz(128 * KiB))
    buf = bytearray()
    while len(buf) < sz(2 * MiB):
        buf += chunk
        buf += rng.randbytes(rng.randrange(1, 3000))
    write("shifted-repeats-2m.bin", bytes(buf[:sz(2 * MiB)]))

    n = sz(1 * MiB)
    img = bytearray(n)
    for _ in range(40):
        off = rng.randrange(n - 2048)
        ln = rng.randrange(16, 2048)
        img[off:off + ln] = rng.randbytes(ln)
    write("sparse-dense-1m.bin", bytes(img))

    n = sz(1 * MiB)
    bm = bytearray()
    while len(bm) < n:
        bm += bytes([rng.choice((0x00, 0xFF, 0x55, 0xAA))]) * rng.choice((1, 3, 17, 255, 4096, 30000))
    write("bitmap-runs-1m.bin", bytes(bm[:n]))

    names = [f"com.example.module{i}.ClassName{j}" for i in range(12) for j in range(9)]
    n = sz(512 * KiB)
    st = bytearray()
    c = 0
    while len(st) < n:
        s = f"{rng.choice(names)}#{c % 97}".encode("utf-16-le")
        st += struct.pack("<H", len(s) // 2) + s
        c += 1
    write("utf16-string-table-512k.bin", bytes(st[:n]))


if __name__ == "__main__":
    main()
