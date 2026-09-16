#!/usr/bin/env python3
"""Derive a logrotate-style rotated .gz directory from a same-split real log (F05 run step).

Reads a prefix (bounded by max_bytes, on a line boundary) of a file inside an already-
materialized same-split dependency (recipe.from_items), then rewrites that prefix in place as a
classic logrotate series: the current segment keeps the original file name uncompressed, and
older segments become "<name>.1.gz" (most recent rotation) through "<name>.N.gz" (oldest), each
independently gzipped -- matching what `logrotate` with `compress` and no `delaycompress` leaves
on disk. The split is a byte-range partition of real log content (nothing is invented), so
real_or_generated="derived-from-real". Reading directly from the dependency's materialized path
(EB_FROM_<NAME>, always set by provision.py for every recipe.from_items entry) rather than via a
preceding copy-item step lets this script cap the prefix before touching disk, independent of how
large the source log actually is (e.g. HDFS.log is 1.58 GB; the item stays small/medium tier).

ebrc script contract:
    python logrotate_derive.py --out <staging dir> --seed <int> --params <canonical JSON>
params:
  from_item    item_id of the recipe.from_items dependency to read from (required)
  src          path of the source log file, relative to that dependency's materialized tree
  dest         output file name inside --out (default: basename of src)
  max_bytes    cap the prefix read from src, rounded down to a whole line (default 12,000,000)
  segments     number of rotated pieces, including the current one (default 6)
"""

import argparse
import gzip
import json
import os
import sys
from pathlib import Path


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True)
    ap.add_argument("--seed", type=int, default=0)
    ap.add_argument("--params", default="{}")
    args = ap.parse_args()
    params = json.loads(args.params)
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)

    from_item = params["from_item"]
    src_rel = params["src"]
    var = "EB_FROM_" + from_item.upper().replace("-", "_")
    from_root = os.environ.get(var)
    if not from_root:
        raise SystemExit(f"dependency {from_item!r} not available (env {var} unset)")
    src = Path(from_root) / src_rel
    if not src.is_file():
        raise SystemExit(f"source log {src} not found")

    max_bytes = int(params.get("max_bytes", 12_000_000))
    segments = int(params.get("segments", 6))
    if segments < 2:
        raise SystemExit("segments must be >= 2")
    dest_name = params.get("dest") or src.name
    target = out / dest_name

    lines = []
    total = 0
    with open(src, "rb") as f:
        for line in f:
            if total + len(line) > max_bytes:
                break
            lines.append(line)
            total += len(line)
    n = len(lines)
    if n < segments:
        raise SystemExit(f"{src}: only {n} lines in the first {max_bytes} bytes, need >= {segments}")
    base = n // segments
    bounds = [base * i for i in range(segments)] + [n]

    # oldest segment (index 0) -> highest rotation number, matching logrotate's convention.
    for seg in range(segments):
        chunk = lines[bounds[seg]:bounds[seg + 1]]
        age = segments - 1 - seg  # 0 = current, 1..segments-1 = .1.gz .. .(segments-1).gz
        if age == 0:
            with open(target, "wb") as f:
                f.writelines(chunk)
        else:
            rotated = target.with_name(f"{target.name}.{age}.gz")
            with gzip.GzipFile(filename="", mode="wb", fileobj=open(rotated, "wb"), mtime=0) as gz:
                gz.writelines(chunk)

    (out / f"{target.name}.rotation_summary.json").write_text(json.dumps({
        "source_item": from_item, "source_path": src_rel, "prefix_bytes": total, "max_bytes": max_bytes,
        "lines": n, "segments": segments, "segment_line_bounds": bounds,
    }, indent=2) + "\n", encoding="utf-8")
    print(f"rotated {dest_name}: {n} lines ({total} bytes) from {from_item}:{src_rel} into {segments} segments",
          file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
