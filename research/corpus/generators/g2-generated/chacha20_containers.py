#!/usr/bin/env python3
"""ChaCha20 keystream and encrypted-volume-like inputs, held-out construction (family F20, corpus group
g2-generated).  GENERATED DATA.  A different CSPRNG, size distribution and container layout from the
tuning/validation generators (csprng_files.py, aes_ctr_ciphertext.py).

Keystream bytes come from `openssl enc -chacha20` (key and 16-byte IV = 32-bit little-endian block counter ||
96-bit nonce, derived from --seed with SHA-256).  Before use, the first 3 blocks from OpenSSL are compared with
an independent pure-Python RFC 8439 ChaCha20 block function; any difference fails.

Modes (params.mode):
  files   pareto/: `count` files with Pareto(alpha=1.1, xm bytes) sizes capped at cap_mib, fanned out by
          first key byte;  sealed/: `sealed` files with a generic header (magic "EBRCSEAL", 12-byte
          nonce, 64-byte random wrapped key), keystream body and a 16-byte random tag.
  volume  volume.img: an encrypted-volume-like image: a 2 MiB header area (text metadata block, 8 random
          key slots, padding with random bytes) followed by volume_mib MiB of ChaCha20 keystream (a distinct
          nonce per 4 GiB segment); plus backup.tar.sealed of backup_mib MiB.
params: mode, count, xm, cap_mib, sealed, volume_mib, backup_mib
"""

import argparse
import hashlib
import json
import os
import random
import struct
import subprocess
import sys

MiB = 1 << 20
EPOCH = int(os.environ.get("SOURCE_DATE_EPOCH", "1767225600"))
ZERO = bytes(4 * MiB)


def rotl(v, c):
    return ((v << c) & 0xFFFFFFFF) | (v >> (32 - c))


def chacha_block(key, counter, nonce):
    const = [0x61707865, 0x3320646E, 0x79622D32, 0x6B206574]
    st = const + list(struct.unpack("<8I", key)) + [counter] + list(struct.unpack("<3I", nonce))
    x = st[:]

    def qr(a, b, c, d):
        x[a] = (x[a] + x[b]) & 0xFFFFFFFF; x[d] = rotl(x[d] ^ x[a], 16)  # noqa: E702
        x[c] = (x[c] + x[d]) & 0xFFFFFFFF; x[b] = rotl(x[b] ^ x[c], 12)  # noqa: E702
        x[a] = (x[a] + x[b]) & 0xFFFFFFFF; x[d] = rotl(x[d] ^ x[a], 8)  # noqa: E702
        x[c] = (x[c] + x[d]) & 0xFFFFFFFF; x[b] = rotl(x[b] ^ x[c], 7)  # noqa: E702

    for _ in range(10):
        qr(0, 4, 8, 12); qr(1, 5, 9, 13); qr(2, 6, 10, 14); qr(3, 7, 11, 15)  # noqa: E702
        qr(0, 5, 10, 15); qr(1, 6, 11, 12); qr(2, 7, 8, 13); qr(3, 4, 9, 14)  # noqa: E702
    return struct.pack("<16I", *[(x[i] + st[i]) & 0xFFFFFFFF for i in range(16)])


class ChaCha:
    def __init__(self, seed, label):
        h = hashlib.sha256(b"ebrc-g2-f20-chacha20|" + str(seed).encode() + b"|" + label.encode()).digest()
        self.key = h
        self.nonce = hashlib.sha256(h).digest()[:12]

    def iv(self, counter=0, nonce=None):
        return struct.pack("<I", counter) + (nonce or self.nonce)

    def stream_to(self, f, n, nonce=None):
        pr = subprocess.Popen(["openssl", "enc", "-chacha20", "-K", self.key.hex(), "-iv", self.iv(0, nonce).hex()],
                              stdin=subprocess.PIPE, stdout=f)
        left = n
        while left:
            k = min(left, len(ZERO))
            pr.stdin.write(ZERO if k == len(ZERO) else bytes(k))
            left -= k
        pr.stdin.close()
        if pr.wait() != 0:
            raise SystemExit("openssl chacha20 failed")

    def bytes(self, n, nonce=None):
        r = subprocess.run(["openssl", "enc", "-chacha20", "-K", self.key.hex(), "-iv", self.iv(0, nonce).hex()],
                           input=bytes(n), capture_output=True)
        if r.returncode != 0 or len(r.stdout) != n:
            raise SystemExit("openssl chacha20 failed")
        return r.stdout


def self_test():
    c = ChaCha(0, "kat")
    got = c.bytes(192)
    want = b"".join(chacha_block(c.key, k, c.nonce) for k in range(3))
    if got != want:
        raise SystemExit("ChaCha20 cross-check failed: openssl output differs from the RFC 8439 block function")


def write_path(out, rel):
    p = os.path.join(out, rel)
    os.makedirs(os.path.dirname(p), exist_ok=True)
    return p


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True)
    ap.add_argument("--seed", type=int, required=True)
    ap.add_argument("--params", default="{}")
    args = ap.parse_args()
    p = json.loads(args.params)
    self_test()
    rng = random.Random(args.seed)
    out = args.out
    total = 0
    if p.get("mode") == "files":
        cap = int(p.get("cap_mib", 64)) * MiB
        for i in range(int(p.get("count", 1200))):
            size = min(cap, int(int(p.get("xm", 16384)) * rng.paretovariate(1.1)))
            c = ChaCha(args.seed, f"pareto-{i}")
            path = write_path(out, f"pareto/{c.key[0]:02x}/obj-{i:05d}.bin")
            with open(path, "wb") as f:
                c.stream_to(f, size)
            total += size
        for i in range(int(p.get("sealed", 200))):
            size = min(cap, int(2048 * rng.paretovariate(1.3)))
            c = ChaCha(args.seed, f"sealed-{i}")
            path = write_path(out, f"sealed/item-{i:04d}.sealed")
            with open(path, "wb") as f:
                f.write(b"EBRCSEAL" + struct.pack(">BBH", 1, 2, 0) + c.nonce + c.bytes(64, b"\1" * 12))
                f.flush()
                c.stream_to(f, size)
                f.seek(0, 2)
                f.write(hashlib.sha256(c.key + b"tag").digest()[:16])
            total += size + 100
    elif p.get("mode") == "volume":
        vol = int(p.get("volume_mib", 1024)) * MiB
        c = ChaCha(args.seed, "volume")
        path = write_path(out, "volume.img")
        with open(path, "wb") as f:
            meta = json.dumps({"format": "ebrc-volume", "version": 2, "cipher": "chacha20", "sector_size": 4096,
                               "segments": vol // (4096 << 20) + 1, "uuid": hashlib.sha256(c.key).hexdigest()[:32],
                               "created": EPOCH}, sort_keys=True).encode()
            hdr = bytearray(b"EBRCVOL\0" + struct.pack(">IQ", len(meta), vol) + meta)
            hdr += bytes(4096 - len(hdr) % 4096)
            for slot in range(8):
                hdr += struct.pack(">I", slot) + c.bytes(508, struct.pack(">I", slot) + b"\2" * 8)
            hdr += c.bytes(2 * MiB - len(hdr), b"\3" * 12)
            f.write(hdr)
            seg = 1 << 32  # 2^20 sectors of 4 KiB
            off = 0
            while off < vol:
                n = min(seg, vol - off)
                f.flush()
                f.seek(0, 2)
                c.stream_to(f, n, struct.pack(">Q", off // seg) + b"\4" * 4)
                off += n
        total += vol + 2 * MiB
        bk = int(p.get("backup_mib", 512)) * MiB
        c2 = ChaCha(args.seed, "backup")
        path = write_path(out, "backup.tar.sealed")
        with open(path, "wb") as f:
            f.write(b"EBRCSEAL" + struct.pack(">BBH", 1, 1, 0) + c2.nonce + c2.bytes(64, b"\5" * 12))
            f.flush()
            c2.stream_to(f, bk)
            f.seek(0, 2)
            f.write(hashlib.sha256(c2.key + b"tag").digest()[:16])
        total += bk
    else:
        raise SystemExit("chacha20_containers: params.mode must be files or volume")
    paths = []
    for r, dn, fn in os.walk(out):
        for x in fn + dn:
            paths.append(os.path.relpath(os.path.join(r, x), out))
    for i, rel in enumerate(sorted(paths, key=lambda q: (-q.count(os.sep), q))):
        fp = os.path.join(out, rel)
        if os.path.isfile(fp):
            os.chmod(fp, 0o644)
        os.utime(fp, ns=((EPOCH - 41 * i) * 10**9,) * 2)
    print(json.dumps({"bytes": total, "openssl": subprocess.run(["openssl", "version"], capture_output=True,
                                                                  text=True).stdout.strip()}), file=sys.stderr)


if __name__ == "__main__":
    main()
