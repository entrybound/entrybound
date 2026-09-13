#!/usr/bin/env python3
"""CDC-adversarial inputs for gear-norm-v1, held-out construction (family F20, corpus group g2-generated).
GENERATED DATA.  Requires numpy.  A different generator from gear_extremes.py (tuning): it targets the
mask transitions and near-misses instead of pure min/max forcing.

Triggers are 22-byte strings whose low 22 hash bits equal a chosen residue (independent of preceding bytes);
filler is SHAKE-256 random data "repaired" so that no other position has zero low-16 hash bits.  Trigger
strings are rejected if a window lying entirely inside them would have zero low-16 bits.

  late-flood-20bit.bin      residue 2^20 every 4096 bytes: matches every late mask and every early mask
                            except fast-v2's 22-bit one -> fast-v2 cuts exactly at target (2 MiB), the other
                            policies exactly at min.
  near-miss-15bit.bin       residue 2^15 every 997 bytes: low 15 bits zero, bit 15 set -> no v2 policy
                            (masks >= 16 bits) ever cuts; every chunk is max although a 15-bit mask would cut
                            everywhere.  Plus a one-byte-insert twin (zero shared chunks).
  target-minus-one.bin      residue 0 every 524287 bytes -> balanced-v2 chunks are exactly target-1.
  alternating-min-max.bin   residue 0 at the end of every other chunk of the sequence 32 KiB, 512 KiB, ...
                            -> extreme-v2 alternates min and max chunks.
  trigger-bursts.bin        unmodified random data with 2 MiB bursts of residue-0 triggers every 32 KiB at
                            every 8 MiB boundary (mixed natural and forced boundaries).
  shifted-dups/file-NN.bin  a 1.5 MiB max-forcing blob copied into 16 max-forcing files at offsets with
                            distinct residues modulo 512 KiB (no duplicate chunks under any policy).
Designed properties are asserted with the reference chunker; other policies' chunk counts are logged.

params: size_mib (int, default 32)
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
    raise SystemExit("gear_straddle: numpy is required (run research/tools/setup_venv.sh)")

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


def clean_trigger(rng, residue):
    """A trigger whose internal fully-contained 16-byte windows (ending at offsets 15..20) are non-zero."""
    while True:
        t = trigger(rng, residue)
        if all(l16_at(t, k) != 0 for k in range(15, WINDOW - 1)):
            return t


def build(rnd, rng, n, ends, residue):
    """Random filler of length n with triggers ending at each position in `ends` (exclusive end offsets)."""
    data = bytearray(rnd(n))
    prot, keep = [], []
    for e in ends:
        if e - WINDOW < 0 or e > n:
            continue
        data[e - WINDOW:e] = clean_trigger(rng, residue)
        prot.append((e - WINDOW, e))
        keep.append(e - 1)
    repair(data, protected=prot, keep=keep)
    return data


def lengths_all(data):
    pos, lv = candidates(bytes(data))
    return {p[0]: chunk_lengths(len(data), pos, lv, p) for p in POLICIES}


def expect(name, cond, detail):
    if not cond:
        raise SystemExit(f"{name}: designed property violated: {detail}")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True)
    ap.add_argument("--seed", type=int, required=True)
    ap.add_argument("--params", default="{}")
    args = ap.parse_args()
    p = json.loads(args.params)
    rng = random.Random(args.seed)
    rnd = Shake(args.seed, b"ebrc-g2-f20-gear-straddle")
    n = int(p.get("size_mib", 32)) * MiB
    out = args.out
    report = {}
    pol = {x[0]: x for x in POLICIES}

    def save(rel, data):
        path = os.path.join(out, rel)
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, "wb") as f:
            f.write(data)
        os.chmod(path, 0o644)
        ls = lengths_all(data)
        report[rel] = {k: {"chunks": len(v), "distinct_lengths": len(set(v))} for k, v in ls.items()}
        return ls

    d = build(rnd, rng, n, range(4096, n + 1, 4096), 1 << 20)
    ls = save("late-flood-20bit.bin", d)
    expect("late-flood", all(x == pol["fast-v2"][2] for x in ls["fast-v2"][:-1]), "fast-v2 chunks != target")
    for k in ("balanced-v2", "dense-v2", "extreme-v2"):
        expect("late-flood", all(x == pol[k][1] for x in ls[k][:-1]), f"{k} chunks != min")

    d = build(rnd, rng, n, range(997, n + 1, 997), 1 << 15)
    ls = save("near-miss-15bit.bin", d)
    for k, v in ls.items():
        expect("near-miss", all(x == pol[k][3] for x in v[:-1]), f"{k} not max-forced")
    d2 = bytearray(rnd(1)) + d
    ls2 = save("near-miss-15bit.insert1-front.bin", d2)
    for k in ls:
        a, o = set(), 0
        for x in ls[k]:
            a.add(hashlib.sha256(d[o:o + x]).digest())
            o += x
        b, o = set(), 0
        for x in ls2[k]:
            b.add(hashlib.sha256(d2[o:o + x]).digest())
            o += x
        expect("near-miss insert", not (a & b), f"{k} shares chunks")

    t1 = pol["balanced-v2"][2] - 1
    d = build(rnd, rng, n, range(t1, n + 1, t1), 0)
    ls = save("target-minus-one.bin", d)
    expect("target-minus-one", all(x == t1 for x in ls["balanced-v2"][:-1]), "balanced-v2 chunks != target-1")

    ends, pos_ = [], 0
    ext = pol["extreme-v2"]
    while pos_ < n:
        pos_ += ext[1]
        ends.append(pos_)
        pos_ += ext[3]
    d = build(rnd, rng, n, [e for e in ends if e <= n], 0)
    ls = save("alternating-min-max.bin", d)
    body = ls["extreme-v2"][:-1]
    expect("alternating", all(x == (ext[1] if i % 2 == 0 else ext[3]) for i, x in enumerate(body)), "extreme-v2 pattern")

    d = bytearray(rnd(n))
    for base in range(0, n, 8 * MiB):
        for e in range(base + 32 * KiB, min(n, base + 2 * MiB) + 1, 32 * KiB):
            d[e - WINDOW:e] = trigger(rng, 0)
    save("trigger-bursts.bin", d)

    blob = bytearray(rnd(1536 * KiB))
    repair(blob)
    blob = bytes(blob)
    residues = set()
    digests = {k: [] for k in pol}
    i = 0
    while i < 16:
        pre = rng.randrange(64 * KiB, 3 * MiB)
        if pre % (512 * KiB) in residues:
            continue
        residues.add(pre % (512 * KiB))
        data = bytearray(rnd(pre)) + blob + bytearray(rnd(rng.randrange(64 * KiB, 2 * MiB)))
        repair(data, protected=[(pre, pre + len(blob))])
        expect("shifted-dups", bytes(data[pre:pre + len(blob)]) == blob, "blob modified")
        ls = save(f"shifted-dups/file-{i:02d}.bin", data)
        for k, v in ls.items():
            expect("shifted-dups", all(x == pol[k][3] for x in v[:-1]), f"{k} not max-forced")
            o = 0
            for x in v:
                digests[k].append(hashlib.sha256(data[o:o + x]).digest())
                o += x
        i += 1
    for k, v in digests.items():
        expect("shifted-dups", len(v) == len(set(v)), f"{k} duplicate chunks")

    paths = []
    for r, dn, fn in os.walk(out):
        for x in fn + dn:
            paths.append(os.path.relpath(os.path.join(r, x), out))
    for j, rel in enumerate(sorted(paths, key=lambda s: (-s.count(os.sep), s))):
        os.utime(os.path.join(out, rel), ns=((EPOCH - 91 * j) * 10**9,) * 2)
    print(json.dumps({"files": len(report)}), file=sys.stderr)


if __name__ == "__main__":
    main()
