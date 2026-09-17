#!/usr/bin/env python3
"""EXP-CROSSFILE-005 collector: exact access granularity of block-bounded solid incumbents.

Pre-registered with research/experiments/EXP-CROSSFILE-005/protocol.md. Stdlib only.

For one (incumbent artifact, source tree) pair, prints ONE JSON line:

  artifact_bytes            size of the artifact file
  access_granularity_bytes  worst case, over every byte of every non-empty regular
                            file, of the decoded (uncompressed) bytes a reader must
                            produce to obtain that one byte
  access_amplification_*    decoded bytes / requested bytes, mean over a seeded
                            sample, per range stratum {1b, 4k, 1m, entry}
                            (objective M10.2; strata are never pooled)
  granularity_source        "listing" when derived exactly from the archive's own
                            block listing, "declared_bound" when only a configured
                            block size bounds it (squashfs, dwarfs)

Decoded-byte model per format (what a sequential decoder must emit to reach the
last requested byte, starting from the start of the independently decodable unit
that contains the first requested byte):

  sevenzip   unit = 7z folder ("Block = N" in `7z l -slt`); files are laid out in
             listing order inside their folder.
  tar+xz     unit = xz block (`xz --robot --list -vv`, uncompressed offsets); member
             data offsets from the decompressed tar stream (tarfile offset_data).
  tar+zstd   unit = the single zstd frame (the incumbent config writes one frame).
  squashfs   unit = data block of --block-bytes (tail fragments are <= one block):
             declared bound, not exact.
  dwarfs     unit = filesystem block of --block-bytes; chunk placement is not
             listed, so only the 1 B stratum and the worst case are bounded.

The sample is a pure function of item_id (SHA-256 seed), so every candidate of
EXP-CROSSFILE-004/-005/-006 samples the same (file, offset) pairs.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import random
import stat
import subprocess
import sys
import tarfile
from bisect import bisect_right
from typing import Dict, List, Optional, Tuple

SCHEMA = "ebr.crossfile.solid-access.v1"
SAMPLE_DOMAIN = b"EXP-CROSSFILE access-sample v1\x00"
STRATA = {"1b": 1, "4k": 4096, "1m": 1048576}


def walk_files(root: str) -> List[Tuple[str, int]]:
    """Sorted (posix relpath, size) of non-empty regular files; symlinks are not followed."""
    out: List[Tuple[str, int]] = []
    root_b = os.fsencode(root)
    for dirpath, dirnames, filenames in os.walk(root_b, followlinks=False):
        dirnames.sort()
        for name in filenames:
            full = os.path.join(dirpath, name)
            st = os.lstat(full)
            if stat.S_ISREG(st.st_mode) and st.st_size > 0:
                rel = os.path.relpath(full, root_b).replace(os.sep.encode(), b"/")
                out.append((os.fsdecode(rel), st.st_size))
    out.sort(key=lambda t: os.fsencode(t[0]))
    return out


def sample_plan(item_id: str, files: List[Tuple[str, int]], n: int):
    seed = int.from_bytes(hashlib.sha256(SAMPLE_DOMAIN + item_id.encode("utf-8")).digest()[:8], "big")
    rng = random.Random(seed)
    plan: Dict[str, List[Tuple[str, int, int]]] = {}
    for label, width in STRATA.items():
        weights = [max(0, size - width + 1) for _, size in files]
        total = sum(weights)
        picks: List[Tuple[str, int, int]] = []
        if total > 0:
            cum = []
            acc = 0
            for w in weights:
                acc += w
                cum.append(acc)
            for _ in range(n):
                x = rng.randrange(total)
                i = bisect_right(cum, x)
                start = x - (cum[i] - weights[i])
                picks.append((files[i][0], start, width))
        plan[label] = picks
    if files:
        idx = list(range(len(files))) if len(files) <= n else [rng.randrange(len(files)) for _ in range(n)]
        plan["entry"] = [(files[i][0], 0, files[i][1]) for i in idx]
    else:
        plan["entry"] = []
    return seed, plan


def run(cmd: List[str], stdin=None) -> bytes:
    return subprocess.run(cmd, check=True, stdout=subprocess.PIPE, stdin=stdin).stdout


def sevenzip_units(artifact: str) -> Dict[str, Tuple[int, int]]:
    """path -> (offset of file data within its folder, folder uncompressed size)."""
    text = run(["7z", "l", "-slt", artifact]).decode("utf-8", "surrogateescape")
    records = []
    started = False
    cur: Dict[str, str] = {}
    for line in text.splitlines():
        if line.startswith("----------"):
            started = True
            continue
        if not started:
            continue
        if not line.strip():
            if cur:
                records.append(cur)
                cur = {}
            continue
        if " = " in line:
            k, v = line.split(" = ", 1)
            cur[k] = v
    if cur:
        records.append(cur)
    fill: Dict[str, int] = {}
    placed: List[Tuple[str, str, int]] = []
    for r in records:
        if r.get("Folder") == "+" or "Path" not in r:
            continue
        size = int(r.get("Size") or 0)
        block = r.get("Block", "")
        if size == 0 or block == "":
            continue
        off = fill.get(block, 0)
        placed.append((r["Path"].replace("\\", "/"), block, off))
        fill[block] = off + size
    return {path: (off, fill[block]) for path, block, off in placed}


def tar_offsets(stream: bytes) -> Dict[str, int]:
    import io

    out: Dict[str, int] = {}
    with tarfile.open(fileobj=io.BytesIO(stream), mode="r|") as tf:
        for m in tf:
            if m.isreg() and m.size > 0:
                name = m.name[2:] if m.name.startswith("./") else m.name
                out[name] = m.offset_data
    return out


def xz_blocks(artifact: str) -> List[Tuple[int, int]]:
    text = run(["xz", "--robot", "--list", "-vv", artifact]).decode()
    blocks = []
    for line in text.splitlines():
        cols = line.split("\t")
        if cols and cols[0] == "block":
            blocks.append((int(cols[5]), int(cols[7])))
    blocks.sort()
    return blocks


def main(argv: Optional[List[str]] = None) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--format", required=True, choices=["sevenzip", "tar+xz", "tar+zstd", "squashfs", "dwarfs"])
    ap.add_argument("--artifact", required=True)
    ap.add_argument("--item-path", required=True)
    ap.add_argument("--item-id", required=True)
    ap.add_argument("--block-bytes", type=int, default=0, help="configured unit size (squashfs, dwarfs)")
    ap.add_argument("--samples", type=int, default=1000)
    a = ap.parse_args(argv)

    files = walk_files(a.item_path)
    sizes = dict(files)
    seed, plan = sample_plan(a.item_id, files, a.samples)
    source = "listing"

    if a.format == "sevenzip":
        units = sevenzip_units(a.artifact)

        def decoded(path: str, off: int, length: int) -> int:
            base, _ = units[path]
            return base + off + length

        worst = max((units[p][0] + s for p, s in files), default=0)
    elif a.format in ("tar+xz", "tar+zstd"):
        tool = ["xz", "-dc", a.artifact] if a.format == "tar+xz" else ["zstd", "-dcq", a.artifact]
        offs = tar_offsets(run(tool))
        if a.format == "tar+xz":
            blocks = xz_blocks(a.artifact)
            starts = [b[0] for b in blocks]
        else:
            starts = [0]

        def unit_start(u: int) -> int:
            return starts[max(0, bisect_right(starts, u) - 1)]

        def decoded(path: str, off: int, length: int) -> int:
            u = offs[path] + off
            return u + length - unit_start(u)

        worst = 0
        for p, s in files:
            last = offs[p] + s - 1
            worst = max(worst, last + 1 - unit_start(last))
    else:
        if a.block_bytes <= 0:
            ap.error("--block-bytes is required for squashfs and dwarfs")
        source = "declared_bound"
        b = a.block_bytes

        def decoded(path: str, off: int, length: int) -> int:
            first = off // b
            last = (off + length - 1) // b
            return (last - first + 1) * b

        worst = b

    result = {
        "schema": SCHEMA,
        "format": a.format,
        "item_id": a.item_id,
        "artifact_bytes": os.path.getsize(a.artifact),
        "files": len(files),
        "logical_bytes": sum(sizes.values()),
        "sample_seed": seed,
        "granularity_source": source,
        "access_granularity_bytes": worst,
    }
    for label, picks in plan.items():
        key = f"access_amplification_{label}"
        if not picks or (a.format == "dwarfs" and label != "1b"):
            result[key] = None
            result[f"{key}_n"] = 0
            continue
        ratios = [decoded(p, off, ln) / ln for p, off, ln in picks]
        result[key] = sum(ratios) / len(ratios)
        result[f"{key}_n"] = len(ratios)
    print(json.dumps(result, sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main())
