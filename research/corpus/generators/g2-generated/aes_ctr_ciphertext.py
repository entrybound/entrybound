#!/usr/bin/env python3
"""AES-256-CTR ciphertext inputs (family F20, corpus group g2-generated).  GENERATED DATA.

Keys and IVs are derived from --seed with SHAKE-256; encryption is done by `openssl enc -aes-256-ctr -nosalt`
after a known-answer test (NIST SP 800-38A F.5.5, CTR-AES256, two blocks).  Plaintexts are synthetic
(structured text/log/record data from a seeded generator), never real data.

  vault/backup-vN.bin.enc            4 near-duplicate plaintext versions (v1..v3 apply insertions,
                                     deletions and in-place edits to the previous version), each encrypted
                                     under the same key with a fresh IV: no ciphertext sharing at all.
  vault-nonce-reuse/backup-vN.bin.enc the same plaintexts under the same key AND the same IV (a crypto misuse
                                     seen in the wild): ciphertext bytes are shared wherever plaintext
                                     bytes are identical at the same offset, so parts are deduplicable.
  keystream/aes-ctr-keystream.bin    pure keystream (encryption of zeros).
  containers/                        generic encrypted-container files: magic "EBRCENC1", version, 16-byte
                                     salt, 16-byte IV, big-endian length, ciphertext of a small synthetic
                                     plaintext, 32-byte random tag; plus "Salted__" + 8-byte salt style files.
params: plaintext_mib (int, 24), keystream_mib (int, 64), containers (int, 300)
"""

import argparse
import hashlib
import json
import math
import os
import random
import struct
import subprocess
import sys

MiB = 1 << 20
EPOCH = int(os.environ.get("SOURCE_DATE_EPOCH", "1767225600"))


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


def aes_ctr(key: bytes, iv: bytes, data: bytes) -> bytes:
    r = subprocess.run(["openssl", "enc", "-aes-256-ctr", "-nosalt", "-K", key.hex(), "-iv", iv.hex()],
                       input=data, capture_output=True)
    if r.returncode != 0:
        raise SystemExit("openssl enc failed: " + r.stderr.decode(errors="replace"))
    if len(r.stdout) != len(data):
        raise SystemExit("openssl enc returned a different length")
    return r.stdout


def aes_ctr_to_file(key, iv, chunks, path):
    with open(path, "wb") as f:
        pr = subprocess.Popen(["openssl", "enc", "-aes-256-ctr", "-nosalt", "-K", key.hex(), "-iv", iv.hex()],
                              stdin=subprocess.PIPE, stdout=f)
        n = 0
        for c in chunks:
            pr.stdin.write(c)
            n += len(c)
        pr.stdin.close()
        if pr.wait() != 0:
            raise SystemExit("openssl enc failed")
    if os.path.getsize(path) != n:
        raise SystemExit("openssl enc output length mismatch")
    os.chmod(path, 0o644)


def kat():
    key = bytes.fromhex("603deb1015ca71be2b73aef0857d77811f352c073b6108d72d9810a30914dff4")
    iv = bytes.fromhex("f0f1f2f3f4f5f6f7f8f9fafbfcfdfeff")
    pt = bytes.fromhex("6bc1bee22e409f96e93d7e117393172aae2d8a571e03ac9c9eb76fac45af8e51")
    ct = bytes.fromhex("601ec313775789a5b7a7f504bbf3d228f443e3ca4d62b59aca84e990cacaf5c5")
    if aes_ctr(key, iv, pt) != ct:
        raise SystemExit("AES-256-CTR known-answer test failed (NIST SP 800-38A F.5.5)")


def plaintext(rng, rnd, n):
    words = sorted({"".join(rng.choice("abcdefghijklmnopqrstuvwxyz") for _ in range(rng.randrange(3, 9))) for _ in range(4000)})
    parts, total, i = [], 0, 0
    while total < n:
        kind = rng.random()
        if kind < 0.5:
            hdr = f"./data/{rng.choice(words)}/{rng.choice(words)}-{i}.log".encode().ljust(100, b"\0") + \
                struct.pack(">12sQ", b"0000644\0", rng.randrange(1 << 30)) + bytes(404)
            body = "".join(f"{EPOCH + i * 3 + k} {rng.choice(('INFO', 'WARN', 'ERROR'))} {rng.choice(words)} "
                           f"{' '.join(rng.choice(words) for _ in range(rng.randrange(3, 12)))}\n"
                           for k in range(rng.randrange(20, 400))).encode()
            blk = hdr + body + bytes((-len(body)) % 512)
        elif kind < 0.8:
            recs = b"".join(struct.pack("<IIq16s", i, rng.randrange(1 << 20), EPOCH - rng.randrange(10**7),
                                        rng.choice(words).encode()) for _ in range(rng.randrange(100, 3000)))
            blk = b"RECS" + struct.pack("<I", len(recs)) + recs
        else:
            blk = b"BLOB" + rnd(rng.randrange(512, 200000))
        parts.append(blk)
        total += len(blk)
        i += 1
    return b"".join(parts)[:n]


def evolve(rng, rnd, data):
    b = bytearray(data)
    for _ in range(rng.randrange(3, 8)):
        op = rng.random()
        pos = rng.randrange(len(b))
        if op < 0.35:
            b[pos:pos] = rnd(rng.randrange(1, 64 * 1024))
        elif op < 0.6:
            del b[pos:pos + rng.randrange(1, 64 * 1024)]
        else:
            n = rng.randrange(1, 256 * 1024)
            b[pos:pos + n] = rnd(min(n, len(b) - pos))
    return bytes(b)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True)
    ap.add_argument("--seed", type=int, required=True)
    ap.add_argument("--params", default="{}")
    args = ap.parse_args()
    p = json.loads(args.params)
    kat()
    rng = random.Random(args.seed)
    rnd = Shake(args.seed, b"ebrc-g2-f20-aes-ctr")
    keys = Shake(args.seed, b"ebrc-g2-f20-aes-ctr-keys")
    out = args.out
    key = keys(32)
    fixed_iv = keys(16)
    n = int(p.get("plaintext_mib", 24)) * MiB
    versions = [plaintext(rng, rnd, n)]
    for _ in range(3):
        versions.append(evolve(rng, rnd, versions[-1]))
    for v, pt in enumerate(versions):
        for sub, iv in (("vault", keys(16)), ("vault-nonce-reuse", fixed_iv)):
            path = os.path.join(out, sub, f"backup-v{v}.bin.enc")
            os.makedirs(os.path.dirname(path), exist_ok=True)
            with open(path, "wb") as f:
                f.write(aes_ctr(key, iv, pt))
            os.chmod(path, 0o644)
    ks = int(p.get("keystream_mib", 64)) * MiB
    os.makedirs(os.path.join(out, "keystream"), exist_ok=True)
    zero = bytes(4 * MiB)
    aes_ctr_to_file(keys(32), keys(16), (zero[: min(4 * MiB, ks - o)] for o in range(0, ks, 4 * MiB)),
                    os.path.join(out, "keystream", "aes-ctr-keystream.bin"))
    os.makedirs(os.path.join(out, "containers"), exist_ok=True)
    total_c = 0
    for i in range(int(p.get("containers", 300))):
        size = max(16, min(4 * MiB, int(math.exp(9.5 + 1.6 * rng.gauss(0, 1)))))
        pt = plaintext(rng, rnd, size)
        salt, iv = rnd(16), rnd(16)
        k = hashlib.pbkdf2_hmac("sha256", b"corpus-passphrase-%d" % (i % 7), salt, 1000, 32)
        if i % 4 == 3:
            data = b"Salted__" + salt[:8] + aes_ctr(k, iv, pt)
            name = f"archive-{i:04d}.bin.enc"
        else:
            data = b"EBRCENC1" + struct.pack(">HH", 1, 1) + salt + iv + struct.pack(">Q", len(pt)) + aes_ctr(k, iv, pt) + rnd(32)
            name = f"object-{i:04d}.ebrcenc"
        with open(os.path.join(out, "containers", name), "wb") as f:
            f.write(data)
        os.chmod(os.path.join(out, "containers", name), 0o644)
        total_c += len(data)
    paths = []
    for r, dn, fn in os.walk(out):
        for x in fn + dn:
            paths.append(os.path.relpath(os.path.join(r, x), out))
    for i, rel in enumerate(sorted(paths, key=lambda q: (-q.count(os.sep), q))):
        os.utime(os.path.join(out, rel), ns=((EPOCH - 29 * i) * 10**9,) * 2)
    print(json.dumps({"openssl": subprocess.run(["openssl", "version"], capture_output=True, text=True).stdout.strip(),
                      "plaintext_versions": [len(v) for v in versions], "containers_bytes": total_c}), file=sys.stderr)


if __name__ == "__main__":
    main()
