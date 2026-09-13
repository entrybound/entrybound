#!/usr/bin/env python3
"""CDC-adversarial inputs for the frozen gear-norm-v1 chunker (family F20, corpus group g2-generated).
GENERATED DATA.  Requires numpy (the shared hash-locked research venv).

gear-norm-v1 (docs/chunking-v1.md, crates/entrybound/src/chunker.rs): hash = (hash << 1) + GEAR[byte]
(mod 2^64), GEAR[i] = splitmix64(0x6a09e667f3bcc909 + i); for target 2^b a cut is taken when
hash & (2^(b+1)-1) == 0 for lengths in [min, target), when hash & (2^(b-1)-1) == 0 for lengths in
[target, max), and forced at max.  Because of the left shift, the low k bits of the hash depend only on
the last k bytes, and every policy has min >= 32 KiB, so a cut decision at a position depends only on
the 16..22 bytes ending there.  The v2 policies (min/target/max):
    fast-v2 512K/2M/8M (masks 22/20 bits)   balanced-v2 128K/512K/2M (19/18)
    dense-v2 64K/256K/1M (19/17)            extreme-v2 32K/128K/512K (18/16)

Constructions (all high-entropy: SHAKE-256 counter-mode bytes, perturbed only where stated):
  max-forcing   Every position whose low 16 hash bits would be zero is repaired by changing one byte in
                the 16-byte window (the smallest mask of any policy is 16 bits, so no policy ever finds a
                content-defined boundary): every chunk is exactly `max`, i.e. CDC degenerates to fixed-size
                chunking.  Variants: one byte inserted at the front (zero chunk dedup for every policy)
                and one byte edited in the middle.
  min-forcing   A 22-byte trigger whose low 22 hash bits are zero (built bit by bit; the trigger alone
                determines those bits) ends at every multiple of 32 KiB.  Every policy's minimum is a
                power-of-two multiple of 32 KiB, so every chunk is exactly `min` for all four policies,
                maximising chunk count and per-chunk overhead.  Variant: one byte inserted at the front.
  controls      Unmodified random data and its one-byte-insert twin (CDC resynchronises).
  dedup-evading A 4 MiB max-forcing blob copied into 8 files after max-forcing prefixes whose lengths
                have pairwise distinct residues modulo 512 KiB: 8 exact copies, zero duplicate chunks
                for every policy.  The same geometry over unmodified random data is the control.
  boundary-lengths  Max-forcing files of length min-1, min, min+1, max-1, max, max+1 for each policy.

After generation every file is chunked with a vectorised reference implementation of gear-norm-v1 and
the intended properties are asserted (failure = non-zero exit).

params: size_mib (int, default 32), dedup_blob_mib (int, 4), dedup_copies (int, 8)
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
    raise SystemExit("gear_extremes: numpy is required (run research/tools/setup_venv.sh)")

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


def maxforced(rnd, n):
    b = bytearray(rnd(n))
    repair(b)
    return b


def write(out, rel, data):
    path = os.path.join(out, rel)
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "wb") as f:
        f.write(data)
    os.chmod(path, 0o644)
    return path


def lengths_all(data):
    pos, lv = candidates(bytes(data))
    return {p[0]: chunk_lengths(len(data), pos, lv, p) for p in POLICIES}


def chunk_digests(data, lengths):
    out, off = [], 0
    for ln in lengths:
        out.append(hashlib.sha256(data[off:off + ln]).digest())
        off += ln
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True)
    ap.add_argument("--seed", type=int, required=True)
    ap.add_argument("--params", default="{}")
    args = ap.parse_args()
    p = json.loads(args.params)
    rng = random.Random(args.seed)
    rnd = Shake(args.seed, b"ebrc-g2-f20-gear-extremes")
    out = args.out
    n = int(p.get("size_mib", 32)) * MiB
    report = {}

    def check_all_equal(name, data, which):
        ls = lengths_all(data)
        for pol in POLICIES:
            want = {"max": pol[3], "min": pol[1]}[which]
            body = ls[pol[0]][:-1]
            if any(x != want for x in body) or ls[pol[0]][-1] > want:
                raise SystemExit(f"{name}: {pol[0]} expected all chunks == {which} ({want}); got "
                                 f"{sorted(set(ls[pol[0]]))[:6]}")
        report[name] = {k: len(v) for k, v in ls.items()}
        return ls

    # max-forcing and its variants
    a = maxforced(rnd, n)
    write(out, "max-forcing/random-maxforced.bin", a)
    check_all_equal("max-forcing/random-maxforced.bin", a, "max")
    ins = bytearray(rnd(1)) + a
    repair(ins, start=0, end=64)
    write(out, "max-forcing/random-maxforced.insert1-front.bin", ins)
    ls_ins = check_all_equal("max-forcing/random-maxforced.insert1-front.bin", ins, "max")
    ls_a = lengths_all(a)
    for pol in POLICIES:
        shared = set(chunk_digests(a, ls_a[pol[0]])) & set(chunk_digests(ins, ls_ins[pol[0]]))
        if shared:
            raise SystemExit(f"insert1-front: {pol[0]} unexpectedly shares {len(shared)} chunks")
    ed = bytearray(a)
    mid = n // 2 + 12345
    ed[mid] ^= 0x5A
    repair(ed, start=mid, end=mid + 16)
    write(out, "max-forcing/random-maxforced.edit1-middle.bin", ed)
    check_all_equal("max-forcing/random-maxforced.edit1-middle.bin", ed, "max")

    # min-forcing
    m = bytearray(rnd(n))
    for end in range(32 * KiB, n + 1, 32 * KiB):
        m[end - WINDOW:end] = trigger(rng, 0)
    write(out, "min-forcing/trigger-every-32k.bin", m)
    check_all_equal("min-forcing/trigger-every-32k.bin", m, "min")
    mi = bytearray(rnd(1)) + m
    write(out, "min-forcing/trigger-every-32k.insert1-front.bin", mi)
    ls = lengths_all(mi)
    report["min-forcing/trigger-every-32k.insert1-front.bin"] = {k: len(v) for k, v in ls.items()}

    # controls
    c = rnd(n)
    write(out, "controls/random.bin", c)
    write(out, "controls/random.insert1-front.bin", rnd(1) + c)

    # dedup-evading copies of a max-forcing blob, and the natural-random control
    blob_n = int(p.get("dedup_blob_mib", 4)) * MiB
    copies = int(p.get("dedup_copies", 8))
    blob = bytes(maxforced(rnd, blob_n))
    cblob = rnd(blob_n)
    residues = set()
    geo = []
    while len(geo) < copies:
        pre = rng.randrange(64 * KiB, 6 * MiB)
        if pre % (512 * KiB) in residues:
            continue
        residues.add(pre % (512 * KiB))
        geo.append((pre, rng.randrange(256 * KiB, 3 * MiB)))
    hostile_digests = {pol[0]: [] for pol in POLICIES}
    control_digests = {pol[0]: [] for pol in POLICIES}
    for k, (pre, suf) in enumerate(geo):
        data = bytearray(rnd(pre)) + blob + bytearray(rnd(suf))
        prot = [(pre, pre + blob_n)]
        repair(data, protected=prot)
        if bytes(data[pre:pre + blob_n]) != blob:
            raise SystemExit("dedup-evading: blob copy was modified by repair")
        write(out, f"dedup-evading/file-{k}.bin", data)
        ls = check_all_equal(f"dedup-evading/file-{k}.bin", data, "max")
        for pol in POLICIES:
            hostile_digests[pol[0]].extend(chunk_digests(data, ls[pol[0]]))
        cdata = rnd(pre) + cblob + rnd(suf)
        write(out, f"dedup-control/file-{k}.bin", cdata)
        cls = lengths_all(cdata)
        for pol in POLICIES:
            control_digests[pol[0]].extend(chunk_digests(cdata, cls[pol[0]]))
    for pol in POLICIES:
        d = hostile_digests[pol[0]]
        if len(d) != len(set(d)):
            raise SystemExit(f"dedup-evading: {pol[0]} found duplicate chunks")
    control_dups = {pol[0]: len(control_digests[pol[0]]) - len(set(control_digests[pol[0]])) for pol in POLICIES}
    if control_dups["extreme-v2"] == 0:
        raise SystemExit("dedup-control: expected CDC to find duplicate chunks in the natural-random control")
    report["dedup-evading"] = {"copies": copies, "prefix_residues_mod_512k": sorted(residues), "duplicate_chunks": 0,
                               "control_duplicate_chunks": control_dups}

    # boundary lengths
    for pol in POLICIES:
        for base, tag in ((pol[1], "min"), (pol[3], "max")):
            for delta in (-1, 0, 1):
                data = maxforced(rnd, base + delta)
                write(out, f"boundary-lengths/{pol[0]}-{tag}{delta:+d}.bin", data)

    paths = []
    for r, dn, fn in os.walk(out):
        for x in fn + dn:
            paths.append(os.path.relpath(os.path.join(r, x), out))
    for i, rel in enumerate(sorted(paths, key=lambda s: (-s.count(os.sep), s))):
        os.utime(os.path.join(out, rel), ns=((EPOCH - 60 * i) * 10**9,) * 2, follow_symlinks=False)
    print(json.dumps({"chunk_counts": report}, sort_keys=True), file=sys.stderr)


if __name__ == "__main__":
    main()
