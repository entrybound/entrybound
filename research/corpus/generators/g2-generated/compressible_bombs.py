#!/usr/bin/env python3
"""Highly compressible and decompression-bomb-like inputs (family F20, corpus group g2-generated).
GENERATED DATA.

Modes (params.mode):
  dense       Plain files that compress extremely well or defeat codec windows (tuning):
                zeros (dense, allocated), single-byte runs, ASCII runs, a counter-incrementing log,
                random periods just beyond common match windows repeated several times
                (32 KiB + 1 > deflate window; 2 MiB + 1 > zstd -3 window; 4 MiB + 1 > brotli default;
                8 MiB + 1 > xz -6 dictionary / zstd -19 window), zeros with one random byte per 4 KiB block,
                zeros with 1 KiB of random bytes every MiB, a 1 MiB random block repeated block-aligned.
              params: scale (float)
  compressed  Small compressed containers with huge expansion ratios (validation):
                gzip of 4 GiB zeros, xz of 8 GiB zeros, bzip2 of 8 GiB zeros, zstd -19 of 16 GiB zeros,
                triple-nested gzip of 1 GiB zeros, a flat zip (16 x 256 MiB zero entries), a nested zip
                (8 stored copies of an inner zip with 16 x 64 MiB zero entries), a tar.zst of 16 x 1 GiB
                zero members, and a POSIX tar with a 1 TiB sparse member (GNU tar --sparse).
              params: gz_gib, xz_gib, bz2_gib, zst_gib, nested_gib, zip_entries, zip_entry_mib, tar_members
Compression uses Python's zlib/lzma/bz2/zipfile/gzip with fixed headers (mtime 0, no names), `zstd -T1`
and GNU tar with fixed owners/mtimes; tool versions are printed to the provision log.  All outputs are
deterministic for fixed tool versions (pins detect drift).
"""

import argparse
import bz2
import gzip
import hashlib
import io
import json
import lzma
import os
import random
import struct
import subprocess
import sys
import zipfile
import zlib

MiB = 1 << 20
KiB = 1 << 10
GiB = 1 << 30
EPOCH = int(os.environ.get("SOURCE_DATE_EPOCH", "1767225600"))
ZCHUNK = bytes(16 * MiB)


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


def zeros_stream(total):
    left = total
    while left:
        take = min(left, len(ZCHUNK))
        yield ZCHUNK if take == len(ZCHUNK) else bytes(take)
        left -= take


def out_file(out, rel):
    path = os.path.join(out, rel)
    os.makedirs(os.path.dirname(path), exist_ok=True)
    return path


def dense(out, rng, rnd, p):
    s = float(p.get("scale", 1.0))
    sizes = {}

    def put(rel, chunks):
        path = out_file(out, rel)
        n = 0
        with open(path, "wb") as f:
            for c in chunks:
                f.write(c)
                n += len(c)
        os.chmod(path, 0o644)
        sizes[rel] = n

    put("runs/zeros-dense.bin", zeros_stream(int(128 * MiB * s)))
    put("runs/byte-ff.bin", [b"\xff" * int(32 * MiB * s)])
    put("runs/ascii-A.txt", [b"A" * int(32 * MiB * s)])

    def periodic(period, total):
        unit = rnd(period)
        left = total
        while left:
            take = min(left, period)
            yield unit[:take]
            left -= take

    put("periods/period-32k+1.bin", periodic(32 * KiB + 1, int(16 * MiB * s)))
    put("periods/period-2m+1.bin", periodic(2 * MiB + 1, 4 * (2 * MiB + 1)))
    put("periods/period-4m+1.bin", periodic(4 * MiB + 1, 3 * (4 * MiB + 1)))
    put("periods/period-8m+1.bin", periodic(8 * MiB + 1, 3 * (8 * MiB + 1)))

    def counter_log(total):
        n, i, buf = 0, 0, []
        while n < total:
            ln = (f"2026-01-01T00:{(i // 60) % 60:02d}:{i % 60:02d}.{i % 1000:03d}Z INFO worker-3 "
                  f"request_id={i:012d} status=200 bytes=5120 duration_ms=12\n").encode()
            buf.append(ln)
            n += len(ln)
            i += 1
            if len(buf) >= 65536:
                yield b"".join(buf)
                buf = []
        if buf:
            yield b"".join(buf)

    put("text/counter-log.log", counter_log(int(32 * MiB * s)))

    def sparse_bytes(total, every, width):
        for off in range(0, total, every):
            blk = bytearray(min(every, total - off))
            pos = rng.randrange(0, max(1, len(blk) - width))
            blk[pos:pos + width] = rnd(min(width, len(blk)))
            yield bytes(blk)

    put("near-zero/one-random-byte-per-4k.bin", sparse_bytes(int(64 * MiB * s), 4 * KiB, 1))
    put("near-zero/1k-random-every-1m.bin", sparse_bytes(int(64 * MiB * s), MiB, KiB))
    block = rnd(MiB)
    put("blocks/1m-random-block-repeated.bin", (block for _ in range(int(64 * s))))
    return sizes


def compressed(out, rng, rnd, p):
    sizes = {}
    info = {}

    def done(rel, expanded):
        path = os.path.join(out, rel)
        os.chmod(path, 0o644)
        sizes[rel] = os.path.getsize(path)
        info[rel] = expanded

    gz_n = int(p.get("gz_gib", 4)) * GiB
    rel = "zeros-%dg.gz" % (gz_n // GiB)
    with open(out_file(out, rel), "wb") as raw:
        with gzip.GzipFile(filename="", mode="wb", fileobj=raw, compresslevel=9, mtime=0) as g:
            for c in zeros_stream(gz_n):
                g.write(c)
    done(rel, gz_n)

    xz_n = int(p.get("xz_gib", 8)) * GiB
    rel = "zeros-%dg.xz" % (xz_n // GiB)
    comp = lzma.LZMACompressor(format=lzma.FORMAT_XZ, check=lzma.CHECK_CRC64, preset=6)
    with open(out_file(out, rel), "wb") as f:
        for c in zeros_stream(xz_n):
            f.write(comp.compress(c))
        f.write(comp.flush())
    done(rel, xz_n)

    bz_n = int(p.get("bz2_gib", 8)) * GiB
    rel = "zeros-%dg.bz2" % (bz_n // GiB)
    comp = bz2.BZ2Compressor(9)
    with open(out_file(out, rel), "wb") as f:
        for c in zeros_stream(bz_n):
            f.write(comp.compress(c))
        f.write(comp.flush())
    done(rel, bz_n)

    zst_n = int(p.get("zst_gib", 16)) * GiB
    rel = "zeros-%dg.zst" % (zst_n // GiB)
    with open(out_file(out, rel), "wb") as f:
        pr = subprocess.Popen(["zstd", "-19", "-T1", "-q", "-c", "--no-progress"], stdin=subprocess.PIPE, stdout=f)
        for c in zeros_stream(zst_n):
            pr.stdin.write(c)
        pr.stdin.close()
        if pr.wait() != 0:
            raise SystemExit("zstd failed")
    done(rel, zst_n)

    nest_n = int(p.get("nested_gib", 1)) * GiB
    rel = "zeros-%dg.gz.gz.gz" % (nest_n // GiB)
    level1 = io.BytesIO()
    with gzip.GzipFile(filename="", mode="wb", fileobj=level1, compresslevel=9, mtime=0) as g:
        for c in zeros_stream(nest_n):
            g.write(c)
    data = level1.getvalue()
    for _ in range(2):
        data = gzip.compress(data, compresslevel=9, mtime=0)
    with open(out_file(out, rel), "wb") as f:
        f.write(data)
    done(rel, nest_n)

    def zinfo(name):
        zi = zipfile.ZipInfo(name, date_time=(2026, 1, 1, 0, 0, 0))
        zi.compress_type = zipfile.ZIP_DEFLATED
        zi.create_system = 3
        zi.external_attr = 0o100644 << 16
        return zi

    entries = int(p.get("zip_entries", 16))
    ent_n = int(p.get("zip_entry_mib", 256)) * MiB
    rel = f"zip-flat-{entries}x{ent_n // MiB}m.zip"
    with zipfile.ZipFile(out_file(out, rel), "w", compresslevel=9, allowZip64=True) as z:
        for i in range(entries):
            with z.open(zinfo(f"zeros-{i:02d}.bin"), "w", force_zip64=True) as w:
                for c in zeros_stream(ent_n):
                    w.write(c)
    done(rel, entries * ent_n)

    inner = io.BytesIO()
    with zipfile.ZipFile(inner, "w", compresslevel=9, allowZip64=True) as z:
        for i in range(16):
            with z.open(zinfo(f"inner-{i:02d}.bin"), "w", force_zip64=True) as w:
                for c in zeros_stream(64 * MiB):
                    w.write(c)
    inner_bytes = inner.getvalue()
    rel = "zip-nested-8x16x64m.zip"
    with zipfile.ZipFile(out_file(out, rel), "w", compresslevel=9, allowZip64=True) as z:
        for i in range(8):
            z.writestr(zinfo(f"layer-{i}.zip"), inner_bytes)
    done(rel, 8 * 16 * 64 * MiB)

    members = int(p.get("tar_members", 16))
    rel = f"tar-zst-{members}x1g.tar.zst"
    with open(out_file(out, rel), "wb") as f:
        pr = subprocess.Popen(["zstd", "-19", "-T1", "-q", "-c", "--no-progress"], stdin=subprocess.PIPE, stdout=f)
        import tarfile
        with tarfile.open(fileobj=pr.stdin, mode="w|", format=tarfile.PAX_FORMAT) as tf:
            for i in range(members):
                ti = tarfile.TarInfo(f"zeros/member-{i:02d}.bin")
                ti.size = GiB
                ti.mtime = EPOCH
                ti.mode = 0o644
                ti.uid = ti.gid = 0
                ti.uname = ti.gname = ""
                tf.addfile(ti, ZeroReader(GiB))
        pr.stdin.close()
        if pr.wait() != 0:
            raise SystemExit("zstd failed")
    done(rel, members * GiB)

    scratch = os.environ.get("EB_SCRATCH") or out
    sp_dir = os.path.join(scratch, "sparse-src")
    os.makedirs(sp_dir, exist_ok=True)
    sp = os.path.join(sp_dir, "disk-1t.img")
    with open(sp, "wb") as f:
        f.seek((1 << 40) - 4096)
        f.write(rnd(4096))
        f.truncate(1 << 40)
    with open(sp, "r+b") as f:
        f.seek(0)
        f.write(b"EBRC-SPARSE-1T\n")
    os.utime(sp, ns=(EPOCH * 10**9,) * 2)
    rel = "tar-sparse-1t-member.tar"
    subprocess.run(["tar", "--create", "--sparse", "--format=posix", "--numeric-owner", "--owner=0", "--group=0",
                    f"--mtime=@{EPOCH}", "--mode=0644", "--sort=name",
                    "--pax-option=exthdr.name=%d/PaxHeaders/%f,delete=atime,delete=ctime",
                    "-C", sp_dir, "-f", os.path.join(out, rel), "disk-1t.img"], check=True)
    os.unlink(sp)
    done(rel, 1 << 40)
    return {"sizes": sizes, "expanded_bytes": info}


class ZeroReader:
    def __init__(self, n):
        self.left = n

    def read(self, k=-1):
        if self.left <= 0:
            return b""
        k = self.left if k is None or k < 0 else min(k, self.left)
        k = min(k, len(ZCHUNK))
        self.left -= k
        return ZCHUNK[:k] if k != len(ZCHUNK) else ZCHUNK


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True)
    ap.add_argument("--seed", type=int, required=True)
    ap.add_argument("--params", default="{}")
    args = ap.parse_args()
    p = json.loads(args.params)
    rng = random.Random(args.seed)
    rnd = Shake(args.seed, b"ebrc-g2-f20-bombs-" + p.get("mode", "").encode())
    versions = {"zlib": zlib.ZLIB_RUNTIME_VERSION, "lzma": lzma.LZMA_VERSION if hasattr(lzma, "LZMA_VERSION") else None,
                "zstd": subprocess.run(["zstd", "-V"], capture_output=True, text=True).stdout.strip(),
                "tar": subprocess.run(["tar", "--version"], capture_output=True, text=True).stdout.splitlines()[0]}
    if p.get("mode") == "dense":
        res = dense(args.out, rng, rnd, p)
    elif p.get("mode") == "compressed":
        res = compressed(args.out, rng, rnd, p)
    else:
        raise SystemExit("compressible_bombs: params.mode must be dense or compressed")
    paths = []
    for r, dn, fn in os.walk(args.out):
        for n in fn + dn:
            paths.append(os.path.relpath(os.path.join(r, n), args.out))
    for i, rel in enumerate(sorted(paths, key=lambda s: (-s.count(os.sep), s))):
        os.utime(os.path.join(args.out, rel), ns=((EPOCH - 7 * i) * 10**9,) * 2)
    print(json.dumps({"tool_versions": versions, "result": res}, sort_keys=True), file=sys.stderr)


if __name__ == "__main__":
    main()
