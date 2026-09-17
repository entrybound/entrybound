#!/usr/bin/env python3
"""Deterministic, bounded-memory synthetic input trees for the scale domain (EXP-SCALE-*).

Research-only instrument. Standard library only (Python >= 3.10). Content is a function of
(shape parameters, seed) alone and uses only FIPS 202 SHAKE256, so it is independent of
Python, OpenSSL and platform versions. Memory use is O(block size) regardless of tree size
(the tree digest is streamed in generation order, which equals byte-sorted path order by
construction).

Shapes
  files    --total-bytes N --files F           F equal files (last takes the remainder),
                                               256 files per directory d%05d/f%07d.bin
  entries  --entries N --entry-bytes S         N files of S bytes, 1000 per leaf directory,
           [--long-names]                      a%04d/b%04d/e%08d.dat (with --long-names the leaf
                                               directory is b%04d- plus 96 'x', so every relative
                                               path exceeds 100 bytes and needs a pax path record)
  sparse   --total-bytes L --island-bytes D    one sparse file of logical size L with a data
           --island-period P                   island of D bytes every P bytes (rest holes)
  dedup    --files F --file-bytes S --unique U F files of S bytes whose content cycles over U
                                               distinct seeds (exact whole-file duplicates)

Content models (--content)
  half            SHAKE256 bytes mapped onto a 16-symbol ASCII alphabet (about 4 bits/byte,
                  roughly 2:1 for zstd-class codecs, no exact duplicate blocks)
  incompressible  raw SHAKE256 bytes
  zeros           zero bytes

Commands
  materialize --out DIR ...   create DIR (must not exist); writes DIR.synth.json beside it
  digest DIR                  recompute the tree digest by reading DIR (streaming)

Tree digest (ebr.scale.tree.v1): SHA-256 over lines "<relpath>\\t<logical size>\\t<sha256>\\n"
for every regular file, in byte-sorted relpath order. Directory mtimes are not part of it.
All files get mtime 946684800 (2000-01-01T00:00:00Z) and mode 0644; directories 0755.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import sys
import time

BLOCK = 1 << 20
TAG = b"ebr-scale-synth-v1"
FIXED_MTIME = 946684800
ALPHA16 = bytes.maketrans(bytes(range(256)), bytes(b"etaoinshrdlu \n.,"[i % 16] for i in range(256)))


def content_block(seed: int, file_key: int, block_index: int, n: int, model: str) -> bytes:
    if model == "zeros":
        return bytes(n)
    raw = hashlib.shake_256(TAG + seed.to_bytes(8, "little") + file_key.to_bytes(8, "little")
                            + block_index.to_bytes(8, "little")).digest(n)
    if model == "incompressible":
        return raw
    if model == "half":
        return raw.translate(ALPHA16)
    raise SystemExit(f"unknown content model {model}")


def write_file(path: str, size: int, seed: int, file_key: int, model: str) -> str:
    h = hashlib.sha256()
    with open(path, "wb") as fh:
        remaining, bi = size, 0
        while remaining > 0:
            n = min(BLOCK, remaining)
            b = content_block(seed, file_key, bi, n, model)
            fh.write(b)
            h.update(b)
            remaining -= n
            bi += 1
    os.chmod(path, 0o644)
    os.utime(path, (FIXED_MTIME, FIXED_MTIME))
    return h.hexdigest()


def write_sparse(path: str, logical: int, island: int, period: int, seed: int, model: str) -> str:
    h = hashlib.sha256()
    zero_block = bytes(BLOCK)
    with open(path, "wb") as fh:
        fh.truncate(logical)
        pos, isl = 0, 0
        while pos < logical:
            n = min(island, logical - pos)
            fh.seek(pos)
            written = 0
            while written < n:
                m = min(BLOCK, n - written)
                b = content_block(seed, isl, written // BLOCK, m, model)
                fh.write(b)
                h.update(b)
                written += m
            gap = min(period, logical - pos) - n
            while gap > 0:  # hash the hole as zeros without writing it
                m = min(BLOCK, gap)
                h.update(zero_block[:m])
                gap -= m
            pos += period
            isl += 1
    os.chmod(path, 0o644)
    os.utime(path, (FIXED_MTIME, FIXED_MTIME))
    return h.hexdigest()


def materialize(a: argparse.Namespace) -> dict:
    out = a.out
    if os.path.exists(out):
        raise SystemExit(f"{out} exists; refusing to overwrite")
    os.makedirs(out)
    tree = hashlib.sha256()
    files = 0
    logical = 0
    t0 = time.time()

    def emit(rel: str, size: int, digest: str) -> None:
        nonlocal files, logical
        tree.update(f"{rel}\t{size}\t{digest}\n".encode())
        files += 1
        logical += size

    if a.shape == "files":
        per = a.total_bytes // a.files
        for i in range(a.files):
            d = f"d{i // 256:05d}"
            os.makedirs(os.path.join(out, d), exist_ok=True)
            size = per + (a.total_bytes - per * a.files if i == a.files - 1 else 0)
            rel = f"{d}/f{i:07d}.bin"
            emit(rel, size, write_file(os.path.join(out, rel), size, a.seed, i, a.content))
    elif a.shape == "entries":
        for i in range(a.entries):
            d = f"a{i // 1000000:04d}/b{i // 1000:04d}" + ("-" + "x" * 96 if a.long_names else "")
            if i % 1000 == 0:
                os.makedirs(os.path.join(out, d), exist_ok=True)
            rel = f"{d}/e{i:08d}.dat"
            emit(rel, a.entry_bytes, write_file(os.path.join(out, rel), a.entry_bytes, a.seed, i, a.content))
    elif a.shape == "sparse":
        rel = "sparse.img"
        emit(rel, a.total_bytes, write_sparse(os.path.join(out, rel), a.total_bytes, a.island_bytes,
                                              a.island_period, a.seed, a.content))
    elif a.shape == "dedup":
        for i in range(a.files):
            d = f"d{i // 256:05d}"
            os.makedirs(os.path.join(out, d), exist_ok=True)
            rel = f"{d}/f{i:07d}.bin"
            emit(rel, a.file_bytes, write_file(os.path.join(out, rel), a.file_bytes, a.seed, i % a.unique, a.content))
    else:
        raise SystemExit(f"unknown shape {a.shape}")

    for root, dirs, _ in os.walk(out):
        for d in dirs:
            os.chmod(os.path.join(root, d), 0o755)
    params = {k: v for k, v in vars(a).items() if k not in ("cmd", "func", "out")}
    rec = {"schema": "ebr.scale.tree.v1", "params": params, "files": files, "logical_bytes": logical,
           "tree_sha256": tree.hexdigest(), "generator_sha256": _self_sha256(), "generate_s": round(time.time() - t0, 3)}
    with open(out.rstrip("/") + ".synth.json", "w", encoding="utf-8") as fh:
        json.dump(rec, fh, sort_keys=True)
        fh.write("\n")
    return rec


def digest(path: str) -> dict:
    entries = []
    for root, dirs, fnames in os.walk(path):
        dirs.sort()
        for f in fnames:
            full = os.path.join(root, f)
            entries.append(os.path.relpath(full, path).replace(os.sep, "/").encode())
    entries.sort()
    tree = hashlib.sha256()
    total = 0
    for rel in entries:
        full = os.path.join(path, rel.decode())
        h = hashlib.sha256()
        size = 0
        with open(full, "rb") as fh:
            while True:
                b = fh.read(BLOCK)
                if not b:
                    break
                h.update(b)
                size += len(b)
        tree.update(rel + f"\t{size}\t{h.hexdigest()}\n".encode())
        total += size
    return {"schema": "ebr.scale.tree.v1", "files": len(entries), "logical_bytes": total, "tree_sha256": tree.hexdigest()}


def _self_sha256() -> str:
    with open(os.path.abspath(__file__), "rb") as fh:
        return hashlib.sha256(fh.read()).hexdigest()


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)
    m = sub.add_parser("materialize")
    m.add_argument("--out", required=True)
    m.add_argument("--shape", required=True, choices=["files", "entries", "sparse", "dedup"])
    m.add_argument("--content", default="half", choices=["half", "incompressible", "zeros"])
    m.add_argument("--seed", type=int, default=20260917)
    m.add_argument("--total-bytes", type=int, default=0)
    m.add_argument("--files", type=int, default=1)
    m.add_argument("--entries", type=int, default=0)
    m.add_argument("--entry-bytes", type=int, default=4096)
    m.add_argument("--long-names", action="store_true")
    m.add_argument("--island-bytes", type=int, default=1 << 20)
    m.add_argument("--island-period", type=int, default=64 << 20)
    m.add_argument("--file-bytes", type=int, default=1 << 20)
    m.add_argument("--unique", type=int, default=1)
    d = sub.add_parser("digest")
    d.add_argument("path")
    a = ap.parse_args(argv)
    rec = materialize(a) if a.cmd == "materialize" else digest(a.path)
    print(json.dumps(rec, sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main())
