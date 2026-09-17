#!/usr/bin/env python3
"""Generate cells.json, cells.jsonl and cells-win.jsonl for EXP-SCALE-004 (design artifact).

Run from the repository root: python research/experiments/EXP-SCALE-004/make_cells.py
"""
import json
import os

HERE = os.path.dirname(os.path.abspath(__file__))
GIB = 1 << 30

KILL_OPS = ["unpack-stream", "unpack-indexed", "pack-indexed", "pack-stream-stdout", "repack", "export-zip", "publish"]
IO_KINDS = ["eacces", "erofs", "epipe", "emfile", "enospc"]
SCENARIOS_LINUX = (
    ["ttff-unpack-stream-stdin", "ttff-unpack-indexed", "ttff-inc-zstd-tar", "diskfull-tmp", "diskfull-dst", "diskfull-pack"]
    + [f"kill-{op}" for op in KILL_OPS]
    + ["corrupt-last-chunk-stream", "corrupt-last-chunk-indexed", "exclusive-create",
       "boundary-entries", "boundary-single-entry", "boundary-logical"]
    + [f"sibling-rename-{fs}" for fs in ("ext4", "xfs", "btrfs")]
    + ["spill-permissions"] + [f"io-fault-{k}" for k in IO_KINDS] + ["nonseekable-census"]
)
SCENARIOS_WIN = ["sibling-rename-ntfs", "spill-permissions-windows", "kill-unpack-stream", "exclusive-create"]

TTFF = ["ttff-unpack-stream-stdin", "ttff-unpack-indexed", "ttff-inc-zstd-tar"]
GENERIC = (["diskfull-tmp", "diskfull-dst", "diskfull-pack"] + [f"kill-{op}" for op in KILL_OPS]
           + ["corrupt-last-chunk-stream", "corrupt-last-chunk-indexed", "exclusive-create"]
           + [f"sibling-rename-{fs}" for fs in ("ext4", "xfs", "btrfs")] + ["spill-permissions"]
           + [f"io-fault-{k}" for k in IO_KINDS] + ["nonseekable-census"])


def files(n, total, seed, content="half"):
    return {"shape": "files", "files": n, "total_bytes": total, "content": content, "seed": seed}


def entries(n, size, seed):
    return {"shape": "entries", "entries": n, "entry_bytes": size, "content": "half", "seed": seed}


CELLS = [
    ("b1g", files(64, 1 * GIB, 1301), TTFF + GENERIC, ["linux", "windows"]),
    ("b4g", files(64, 4 * GIB, 1302), TTFF + ["spill-permissions", "kill-unpack-stream", "diskfull-tmp"], ["linux", "windows"]),
    ("b16g", files(64, 16 * GIB, 1303), TTFF, ["linux"]),
    ("e4g", files(1, 4 * GIB, 1312), TTFF + ["diskfull-tmp", "kill-unpack-stream"], ["linux"]),
    ("n1e5", entries(100000, 4096, 1323), TTFF + ["kill-unpack-stream", "kill-pack-indexed", "nonseekable-census"], ["linux", "windows"]),
    ("entries-1000000", entries(1000000, 64, 1431), ["boundary-entries"], ["linux"]),
    ("entries-1000001", entries(1000001, 64, 1432), ["boundary-entries"], ["linux"]),
    ("single-2p34", files(1, 1 << 34, 1433), ["boundary-single-entry"], ["linux"]),
    ("single-2p34p1", files(1, (1 << 34) + 1, 1434), ["boundary-single-entry"], ["linux"]),
    ("logical-2p36p2p20", files(64, (1 << 36) + (1 << 20), 1435), ["boundary-logical"], ["linux"]),
]


def main() -> None:
    cells = []
    rows_linux, rows_win = [], []
    for cid, synth, scenarios, hosts in CELLS:
        cells.append({"item_id": cid, "synth": synth, "applicable_scenarios": scenarios, "hosts": hosts,
                      "kill_points_per_rep": 25, "expected_tree_sha256": None})
        row = {"item_id": cid, "split": "synthetic", "family": "SYN-FAULT"}
        if "linux" in hosts:
            rows_linux.append(dict(row, path="research/experiments/EXP-SCALE-004/cells.json"))
        if "windows" in hosts:
            rows_win.append(dict(row, path="research/experiments/EXP-SCALE-004/cells.json"))
    doc = {"schema": "ebr.scale.faultcells.v1", "experiment_id": "EXP-SCALE-004",
           "binary_linux": "/root/eb-research/target/scale-prod/release/ebound",
           "binary_windows": "D:/eb-research/target/scale-prod-win/release/ebound.exe",
           "binary_commit": None, "caps": {"default": 24 * GIB, "cap2": 2 * GIB},
           "scenarios_linux": SCENARIOS_LINUX, "scenarios_windows": SCENARIOS_WIN,
           "rename_retry_limit": 5, "cells": cells}
    with open(os.path.join(HERE, "cells.json"), "w", encoding="utf-8", newline="\n") as fh:
        json.dump(doc, fh, indent=1, sort_keys=True)
        fh.write("\n")
    for name, rows in (("cells.jsonl", rows_linux), ("cells-win.jsonl", rows_win)):
        with open(os.path.join(HERE, name), "w", encoding="utf-8", newline="\n") as fh:
            for r in rows:
                fh.write(json.dumps(r, sort_keys=True) + "\n")


if __name__ == "__main__":
    main()
