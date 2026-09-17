#!/usr/bin/env python3
"""EXP-CHUNK-001 per-item screen wrapper around the existing ``ebr-chunk`` binary.

Runs, for one corpus item (a file or a directory) and one CDC candidate
(algorithm + production preset + optional algorithm flags):

1. ``ebr-chunk dedup`` over every *distinct* non-empty regular file of the item
   (identical files are passed once, mirroring ``entrybound::chunker::evaluate``,
   which counts identical ContentObjects once), giving archive-wide exact-chunk
   dedup within the item;
2. ``ebr-chunk boundaries`` over every distinct file (chunk-size distribution);
3. ``ebr-chunk shift-stability`` over the first ``--shift-prefix-bytes`` of the
   largest distinct file, for ``--shift-seeds`` seeds (insert/delete/overwrite);
4. ``ebr-chunk random-access`` over the largest distinct file.

It prints exactly ONE JSON line (the ebr runner reads only the last stdout
line): ``{"schema": "ebr.exp-chunk-001.screen.v1", "payload": {...}}``.
``payload.deterministic_sha256`` hashes the canonical JSON of every
deterministic field (throughput smoke fields and run ids excluded), so the
runner's repeat-hash check (decision-method.md section 4.13) can compare
repetitions.

Symlinks with safe names are staged under ``--scratch`` so file paths never
reach ``ebr-chunk --versions`` (comma-separated) with a comma in them.
Standard library only. Not decision-grade timing: no timing fields are used.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import statistics
import subprocess
import sys
from pathlib import Path

SCHEMA = "ebr.exp-chunk-001.screen.v1"


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as fh:
        for block in iter(lambda: fh.read(1 << 20), b""):
            h.update(block)
    return h.hexdigest()


def regular_files(root: Path) -> list[tuple[bytes, Path, int]]:
    if root.is_file():
        return [(os.fsencode(root.name), root, root.stat().st_size)]
    out = []
    for dirpath, dirnames, filenames in os.walk(root, followlinks=False):
        dirnames.sort(key=os.fsencode)
        for name in filenames:
            p = Path(dirpath) / name
            st = p.lstat()
            if not p.is_symlink() and os.path.isfile(p):
                rel = os.fsencode(os.path.relpath(p, root))
                out.append((rel, p, st.st_size))
    out.sort(key=lambda t: t[0])
    return out


def run_chunk(ebr_chunk: str, sub: str, item: Path, common: list[str], extra: list[str]) -> dict:
    cmd = [ebr_chunk, sub, "--item-path", str(item), *common, *extra]
    proc = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False)
    if proc.returncode != 0:
        sys.stderr.write(proc.stderr.decode("utf-8", "replace"))
        raise SystemExit(f"ebr-chunk {sub} failed with exit {proc.returncode}")
    lines = [ln for ln in proc.stdout.decode("utf-8").splitlines() if ln.strip()]
    return json.loads(lines[-1])["payload"]


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--item-path", required=True)
    ap.add_argument("--scratch", required=True)
    ap.add_argument("--ebr-chunk", default="/root/eb-research/target/harness/release/ebr-chunk")
    ap.add_argument("--experiment-id", default="EXP-CHUNK-001")
    ap.add_argument("--env-name", default="wsl-ubuntu")
    ap.add_argument("--algorithm", required=True)
    ap.add_argument("--profile", required=True, choices=["fast", "balanced", "dense", "extreme"])
    ap.add_argument("--normalization", default="none")
    ap.add_argument("--value-width", default="none")
    ap.add_argument("--seed", type=int, default=1)
    ap.add_argument("--shift-seeds", type=int, default=4)
    ap.add_argument("--shift-prefix-bytes", type=int, default=16 * 1024 * 1024)
    ap.add_argument("--edit-bytes", type=int, default=64)
    ap.add_argument("--ra-range-bytes", type=int, default=4096)
    ap.add_argument("--ra-samples", type=int, default=200)
    a = ap.parse_args()

    item = Path(a.item_path)
    scratch = Path(a.scratch) / "exp-chunk-001-stage"
    scratch.mkdir(parents=True, exist_ok=True)

    common = ["--experiment-id", a.experiment_id, "--env-name", a.env_name,
              "--algorithm", a.algorithm, "--profile", a.profile]
    if a.normalization != "none":
        common += ["--normalization", a.normalization]
    if a.value_width != "none":
        common += ["--value-width", a.value_width]

    files = regular_files(item)
    total_logical = sum(size for _, _, size in files)
    distinct: list[tuple[str, Path, int]] = []
    seen: set[str] = set()
    for _, path, size in files:
        if size == 0:
            continue
        digest = sha256_file(path)
        if digest in seen:
            continue
        seen.add(digest)
        distinct.append((digest, path, size))

    payload: dict = {
        "algorithm": a.algorithm, "profile": a.profile,
        "normalization": a.normalization, "value_width": a.value_width,
        "file_count": len(files), "distinct_nonempty_file_count": len(distinct),
        "total_logical_bytes": total_logical,
    }
    if not distinct:
        payload.update({"algorithm_id": None, "dedup_stored_fraction": None})
    else:
        staged = []
        for i, (_, path, _) in enumerate(distinct):
            link = scratch / f"f{i:05d}"
            if link.is_symlink() or link.exists():
                link.unlink()
            os.symlink(os.path.abspath(path), link)
            staged.append(link)

        dd = run_chunk(a.ebr_chunk, "dedup", staged[0], common,
                       ["--versions", ",".join(str(s) for s in staged[1:])] if len(staged) > 1 else [])
        payload["algorithm_id"] = dd["algorithm_id"]
        payload["distinct_logical_bytes"] = dd["total_logical_bytes"]
        payload["unique_chunk_count"] = dd["unique_chunk_count"]
        payload["unique_chunk_bytes"] = dd["unique_chunk_bytes"]
        payload["manifest_chunk_references"] = dd["manifest_chunk_references"]
        payload["mean_chunk_bytes"] = dd["mean_chunk_bytes"]
        payload["modelled_cost_bytes_frozen_constants"] = dd["estimated_cost_bytes"]
        payload["dedup_stored_fraction"] = dd["unique_chunk_bytes"] / total_logical

        counts, p99s, maxes, mins = [], [], [], []
        for link in staged:
            b = run_chunk(a.ebr_chunk, "boundaries", link, common, ["--buckets", "16"])
            counts.append(b["chunk_count"])
            p99s.append(b["p99_chunk_bytes"])
            maxes.append(b["max_chunk_bytes"])
            mins.append(b["min_chunk_bytes"])
        payload["chunk_count"] = sum(counts)
        payload["max_file_p99_chunk_bytes"] = max(p99s)
        payload["max_chunk_bytes"] = max(maxes)

        largest = max(distinct, key=lambda t: (t[2], t[0]))[1]
        prefix = scratch / "shift-prefix.bin"
        with open(largest, "rb") as src, open(prefix, "wb") as dst:
            dst.write(src.read(a.shift_prefix_bytes))
        per_kind: dict[str, list[float]] = {}
        changed: dict[str, list[int]] = {}
        for s in range(1, a.shift_seeds + 1):
            sh = run_chunk(a.ebr_chunk, "shift-stability", prefix, common,
                           ["--seed", str(a.seed * 1000 + s), "--insert-bytes", str(a.edit_bytes),
                            "--delete-bytes", str(a.edit_bytes), "--overwrite-bytes", str(a.edit_bytes)])
            for e in sh["edits"]:
                per_kind.setdefault(e["kind"], []).append(e["preserved_boundary_fraction"])
                changed.setdefault(e["kind"], []).append(e["changed_bytes"])
        for kind, vals in sorted(per_kind.items()):
            payload[f"shift_{kind}_preserved_boundary_fraction_median"] = statistics.median(vals)
            payload[f"shift_{kind}_changed_bytes_median"] = statistics.median(changed[kind])

        ra = run_chunk(a.ebr_chunk, "random-access", largest, common,
                       ["--range-bytes", str(a.ra_range_bytes), "--samples", str(a.ra_samples),
                        "--seed", str(a.seed)])
        payload["ra_4k_mean_amplification"] = ra["mean_amplification"]
        payload["ra_4k_max_amplification"] = ra["max_amplification"]

        for link in staged:
            link.unlink()
        prefix.unlink()

    canonical = json.dumps(payload, sort_keys=True, separators=(",", ":")).encode()
    payload["deterministic_sha256"] = hashlib.sha256(canonical).hexdigest()
    print(json.dumps({"schema": SCHEMA, "payload": payload}, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
