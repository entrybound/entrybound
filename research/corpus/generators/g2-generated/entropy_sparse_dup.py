#!/usr/bin/env python3
"""Maximal-entropy files with sparse exact duplication (family F20, corpus group g2-generated).
GENERATED DATA.  Requires numpy (the shared hash-locked research venv).

Two variants with identical duplication geometry:
  cdc-friendly/  `files` files of `file_mib` MiB of SHAKE-256 random bytes; a pool of `blobs` random blobs
                 (sizes from blob_kib) is copied 2..max_copies times each into random files at random,
                 non-overlapping offsets.  Duplicated bytes are a few percent of the total.  Content-defined
                 chunking re-synchronises inside the copies and finds duplicate chunks.
  cdc-hostile/   the same geometry, but base data and blobs are gear-norm-v1 max-forcing (no position has
                 zero low-16 hash bits, repaired outside the protected blob copies) and the copies of each
                 blob sit at offsets with pairwise distinct residues modulo 512 KiB: every v2 policy cuts
                 only at multiples of `max`, so no chunk is duplicated although the blobs are.
Both variants are checked with the reference chunker (duplicate chunks > 0 for friendly/extreme-v2,
= 0 for hostile/all policies; hostile files must consist of max-size chunks only).

params: files (24), file_mib (8), blobs (12), blob_kib ([64, 256, 1024, 4096]), max_copies (6)
"""

import argparse
import bisect
import hashlib
import json
import os
import random
import struct
import sys

try:
    import numpy as np
except ImportError:  # pragma: no cover
    raise SystemExit("entropy_sparse_dup: numpy is required (run research/tools/setup_venv.sh)")

MiB = 1 << 20
KiB = 1 << 10
EPOCH = int(os.environ.get("SOURCE_DATE_EPOCH", "1767225600"))

# ---------------------------------------------------------------------------------------------
# gear-norm-v1 reference (kept byte-identical in gear_extremes.py, entropy_sparse_dup.py, gear_straddle.py)
M64 = (1 << 64) - 1


def splitmix64(x):
    x = (x + 0x9E3779B97F4A7C15) & M64
    x = ((x ^ (x >> 30)) * 0xBF58476D1CE4E5B9) & M64
    x = ((x ^ (x >> 27)) * 0x94D049BB133111EB) & M64
    return x ^ (x >> 31)


GEAR = [splitmix64((0x6A09E667F3BCC909 + i) & M64) for i in range(256)]
T32 = np.array([g & 0xFFFFFFFF for g in GEAR], dtype=np.uint32)
G16 = [g & 0xFFFF for g in GEAR]
BIT0_BYTES = ([c for c in range(256) if GEAR[c] & 1 == 0], [c for c in range(256) if GEAR[c] & 1 == 1])
POLICIES = (("fast-v2", 512 * KiB, 2 * MiB, 8 * MiB), ("balanced-v2", 128 * KiB, 512 * KiB, 2 * MiB),
            ("dense-v2", 64 * KiB, 256 * KiB, 1 * MiB), ("extreme-v2", 32 * KiB, 128 * KiB, 512 * KiB))
WINDOW = 22


def low_hash(buf) -> "np.ndarray":
    """L[p] = sum_{i<22} GEAR[b[p-i]] << i mod 2^32 (bytes before the buffer count as absent).
    Bits 0..21 equal the chunker hash bits whenever p is >= 21 bytes past the chunk start."""
    a = np.frombuffer(buf, dtype=np.uint8)
    n = a.size
    out = np.empty(n, dtype=np.uint32)
    step = 8 * MiB
    for s in range(0, n, step):
        e = min(n, s + step)
        lo = max(0, s - (WINDOW - 1))
        t = T32[a[lo:e]]
        acc = t.copy()
        for i in range(1, WINDOW):
            acc[i:] += t[:-i] << np.uint32(i)
        out[s:e] = acc[s - lo:]
    return out


def candidates(buf, bits=16):
    L = low_hash(buf)
    pos = np.flatnonzero((L & np.uint32((1 << bits) - 1)) == 0).astype(np.int64)
    return pos, L[pos]


def chunk_lengths(n, pos, lv, policy):
    _, mn, tg, mx = policy
    b = tg.bit_length() - 1
    early = pos[(lv & np.uint32((1 << (b + 1)) - 1)) == 0]
    late = pos[(lv & np.uint32((1 << (b - 1)) - 1)) == 0]
    out = []
    start = 0
    while start < n:
        limit = min(start + mx, n)
        end = None
        lo, hi = start + mn - 1, min(start + tg - 2, limit - 1)
        if lo <= hi:
            i = int(np.searchsorted(early, lo))
            if i < early.size and early[i] <= hi:
                end = int(early[i]) + 1
        if end is None:
            j = int(np.searchsorted(late, start + tg - 1))
            if j < late.size and late[j] <= limit - 1:
                end = int(late[j]) + 1
        if end is None:
            end = limit
        out.append(end - start)
        start = end
    return out


def l16_at(b, r):
    v = 0
    for i in range(16):
        q = r - i
        if q < 0:
            break
        v += G16[b[q]] << i
    return v & 0xFFFF


def repair(buf: bytearray, protected=(), start=0, end=None, keep=()):
    """Remove every zero of the low 16 hash bits in positions [start, end) (except positions in `keep`) by
    changing the newest unprotected byte of the 16-byte window.  Deterministic."""
    end = len(buf) if end is None else end
    keep = set(keep)
    prot = sorted(protected)
    prot_starts = [s for s, _ in prot]

    def is_prot(q):
        i = bisect.bisect_right(prot_starts, q) - 1
        return i >= 0 and q < prot[i][1]

    for _ in range(8):
        L = low_hash(bytes(buf[max(0, start - 16):end]))
        zeros = np.flatnonzero((L & np.uint32(0xFFFF)) == 0) + max(0, start - 16)
        zeros = [int(z) for z in zeros if z >= start and z >= WINDOW - 1 and int(z) not in keep]
        if not zeros:
            return
        for p in zeros:
            if l16_at(buf, p) != 0:
                continue
            q = next((x for x in range(p, max(-1, p - 16), -1) if not is_prot(x)), None)
            if q is None:
                raise SystemExit(f"repair: zero at {p} lies entirely inside protected bytes")
            orig = buf[q]
            for k in range(1, 256):
                buf[q] = (orig + k) & 0xFF
                if all(l16_at(buf, r) != 0 or r in keep for r in range(max(q, WINDOW - 1), min(len(buf), q + 16))):
                    break
            else:
                raise SystemExit(f"repair: no byte value clears position {p}")
    raise SystemExit("repair: did not converge")


def trigger(rng, residue, bits=WINDOW):
    """Bytes s[0..bits) with sum GEAR[s[bits-1-i]] << i == residue (mod 2^bits)."""
    s = [0] * bits
    acc = 0
    for i in range(bits):
        want = ((residue >> i) ^ (acc >> i)) & 1
        choices = BIT0_BYTES[want]
        c = choices[rng.randrange(len(choices))]
        s[bits - 1 - i] = c
        acc = (acc + (GEAR[c] << i)) & ((1 << bits) - 1)
    assert acc == residue & ((1 << bits) - 1)
    return bytes(s)
# ---------------------------------------------------------------------------------------------


class Shake:
    def __init__(self, seed, domain):
        self.prefix = domain + b"\0" + struct.pack("<Q", seed & M64)
        self.counter = 0

    def __call__(self, n):
        out = bytearray()
        while len(out) < n:
            take = min(n - len(out), 4 * MiB)
            out += hashlib.shake_256(self.prefix + struct.pack("<Q", self.counter)).digest(take)
            self.counter += 1
        return bytes(out)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True)
    ap.add_argument("--seed", type=int, required=True)
    ap.add_argument("--params", default="{}")
    args = ap.parse_args()
    p = json.loads(args.params)
    rng = random.Random(args.seed)
    rnd = Shake(args.seed, b"ebrc-g2-f20-sparse-dup")
    n_files = int(p.get("files", 24))
    fsize = int(p.get("file_mib", 8)) * MiB
    n_blobs = int(p.get("blobs", 12))
    sizes = [int(x) * KiB for x in p.get("blob_kib", [64, 256, 1024, 4096])]
    max_copies = int(p.get("max_copies", 6))
    gap = 64

    placements = {i: [] for i in range(n_files)}  # file -> [(offset, blob)]
    blob_sizes = []
    for b in range(n_blobs):
        size = sizes[b % len(sizes)]
        blob_sizes.append(size)
        copies = rng.randrange(2, max_copies + 1)
        residues = set()
        placed = 0
        tries = 0
        while placed < copies and tries < 10000:
            tries += 1
            fi = rng.randrange(n_files)
            off = rng.randrange(gap, fsize - size - gap)
            if off % (512 * KiB) in residues:
                continue
            if any(off < o + blob_sizes[bb] + gap and o < off + size + gap for o, bb in placements[fi]):
                continue
            placements[fi].append((off, b))
            residues.add(off % (512 * KiB))
            placed += 1
        if placed < 2:
            raise SystemExit("entropy_sparse_dup: could not place blob copies")
    friendly_blobs = [rnd(s) for s in blob_sizes]
    hostile_blobs = []
    for s in blob_sizes:
        hb = bytearray(rnd(s))
        repair(hb)
        hostile_blobs.append(bytes(hb))

    report = {"duplicated_bytes": sum(blob_sizes[b] for f in placements.values() for _, b in f),
              "total_bytes": 2 * n_files * fsize}
    digests = {"cdc-friendly": {pol[0]: [] for pol in POLICIES}, "cdc-hostile": {pol[0]: [] for pol in POLICIES}}
    for variant, blobs in (("cdc-friendly", friendly_blobs), ("cdc-hostile", hostile_blobs)):
        for fi in range(n_files):
            data = bytearray(rnd(fsize))
            prot = []
            for off, b in sorted(placements[fi]):
                data[off:off + blob_sizes[b]] = blobs[b]
                prot.append((off, off + blob_sizes[b]))
            if variant == "cdc-hostile":
                repair(data, protected=prot)
                for off, b in placements[fi]:
                    if bytes(data[off:off + blob_sizes[b]]) != blobs[b]:
                        raise SystemExit("cdc-hostile: repair modified a blob copy")
            path = os.path.join(args.out, variant, f"file-{fi:02d}.bin")
            os.makedirs(os.path.dirname(path), exist_ok=True)
            with open(path, "wb") as f:
                f.write(data)
            os.chmod(path, 0o644)
            pos, lv = candidates(bytes(data))
            for pol in POLICIES:
                ls = chunk_lengths(len(data), pos, lv, pol)
                if variant == "cdc-hostile" and (any(x != pol[3] for x in ls[:-1]) or ls[-1] > pol[3]):
                    raise SystemExit(f"cdc-hostile/file-{fi:02d}: {pol[0]} is not max-forced")
                o = 0
                for ln in ls:
                    digests[variant][pol[0]].append(hashlib.sha256(data[o:o + ln]).digest())
                    o += ln
    dups = {v: {k: len(d) - len(set(d)) for k, d in pd.items()} for v, pd in digests.items()}
    if any(dups["cdc-hostile"].values()):
        raise SystemExit(f"cdc-hostile: duplicate chunks found {dups['cdc-hostile']}")
    if dups["cdc-friendly"]["extreme-v2"] == 0:
        raise SystemExit("cdc-friendly: expected duplicate chunks")
    report["duplicate_chunks"] = dups
    paths = []
    for r, dn, fn in os.walk(args.out):
        for n in fn + dn:
            paths.append(os.path.relpath(os.path.join(r, n), args.out))
    for i, rel in enumerate(sorted(paths, key=lambda s: (-s.count(os.sep), s))):
        os.utime(os.path.join(args.out, rel), ns=((EPOCH - 3 * i) * 10**9,) * 2)
    print(json.dumps(report, sort_keys=True), file=sys.stderr)


if __name__ == "__main__":
    main()
