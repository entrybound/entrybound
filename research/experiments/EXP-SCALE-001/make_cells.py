#!/usr/bin/env python3
"""Generate cells.json and cells.jsonl for EXP-SCALE-001 (design artifact; deterministic).

Run from the repository root:  python research/experiments/EXP-SCALE-001/make_cells.py
The cell table mirrors protocol.md "Corpus selectors"; the operation table mirrors
"Candidates". expected_tree_sha256 stays null until the first materialization, whose
values are committed before stage 1 (protocol.md, driver contract step 2).
"""
import json
import os

HERE = os.path.dirname(os.path.abspath(__file__))
GIB = 1 << 30
MIB = 1 << 20
CAP24 = 24 * GIB
CAP2 = 2 * GIB

SHAPES = [
    # id, synth args, logical bytes, entries, caps, stage
    ("b1g", {"shape": "files", "files": 64, "total_bytes": 1 * GIB, "content": "half", "seed": 1301}, 1 * GIB, 64, [CAP24], 1),
    ("b4g", {"shape": "files", "files": 64, "total_bytes": 4 * GIB, "content": "half", "seed": 1302}, 4 * GIB, 64, [CAP24, CAP2], 1),
    ("b16g", {"shape": "files", "files": 64, "total_bytes": 16 * GIB, "content": "half", "seed": 1303}, 16 * GIB, 64, [CAP24, CAP2], 1),
    ("b64g", {"shape": "files", "files": 64, "total_bytes": (1 << 36) - (1 << 24), "content": "half", "seed": 1304}, (1 << 36) - (1 << 24), 64, [CAP24, CAP2], 2),
    ("e1g", {"shape": "files", "files": 1, "total_bytes": 1 * GIB, "content": "half", "seed": 1311}, 1 * GIB, 1, [CAP24], 1),
    ("e4g", {"shape": "files", "files": 1, "total_bytes": 4 * GIB, "content": "half", "seed": 1312}, 4 * GIB, 1, [CAP24, CAP2], 1),
    ("e16g", {"shape": "files", "files": 1, "total_bytes": (1 << 34) - (1 << 24), "content": "half", "seed": 1313}, (1 << 34) - (1 << 24), 1, [CAP24, CAP2], 1),
    ("n1e3", {"shape": "entries", "entries": 1000, "entry_bytes": 4096, "content": "half", "seed": 1321}, 1000 * 4096, 1000, [CAP24], 1),
    ("n1e4", {"shape": "entries", "entries": 10000, "entry_bytes": 4096, "content": "half", "seed": 1322}, 10000 * 4096, 10000, [CAP24], 1),
    ("n1e5", {"shape": "entries", "entries": 100000, "entry_bytes": 4096, "content": "half", "seed": 1323}, 100000 * 4096, 100000, [CAP24, CAP2], 1),
    ("n999k", {"shape": "entries", "entries": 999000, "entry_bytes": 4096, "content": "half", "seed": 1324}, 999000 * 4096, 999000, [CAP24, CAP2], 1),
    ("s1g", {"shape": "sparse", "total_bytes": 1 * GIB, "island_bytes": MIB, "island_period": 64 * MIB, "content": "half", "seed": 1331}, 1 * GIB, 1, [CAP24], 1),
    ("s16g", {"shape": "sparse", "total_bytes": (1 << 34) - (1 << 24), "island_bytes": MIB, "island_period": 64 * MIB, "content": "half", "seed": 1332}, (1 << 34) - (1 << 24), 1, [CAP24, CAP2], 1),
    ("d1g", {"shape": "dedup", "files": 256, "file_bytes": 4 * MIB, "unique": 16, "content": "half", "seed": 1341}, 256 * 4 * MIB, 256, [CAP24], 1),
    ("d16g", {"shape": "dedup", "files": 4096, "file_bytes": 4 * MIB, "unique": 256, "content": "half", "seed": 1342}, 4096 * 4 * MIB, 4096, [CAP24, CAP2], 1),
    ("i4g", {"shape": "files", "files": 64, "total_bytes": 4 * GIB, "content": "incompressible", "seed": 1351}, 4 * GIB, 64, [CAP24, CAP2], 1),
]

EB = "/root/eb-research/target/scale-prod/release/ebound"

# name: (argv template, stdin, stdout, prerequisites, applicability)
OPS = {
    "pack-indexed-fast": ([EB, "pack", "{input}", "{dst}/out.eb", "--profile", "fast"], None, None, [], "all"),
    "pack-indexed-balanced": ([EB, "pack", "{input}", "{dst}/out.eb", "--profile", "balanced"], None, None, [], "all"),
    "pack-indexed-dense": ([EB, "pack", "{input}", "{dst}/out.eb", "--profile", "dense"], None, None, [], "logical<=4GiB&entries<=1e5"),
    "pack-indexed-extreme": ([EB, "pack", "{input}", "{dst}/out.eb", "--profile", "extreme"], None, None, [], "logical<=1GiB&entries<=1e4"),
    "pack-stream-file": ([EB, "pack", "{input}", "{dst}/out.ebs", "--layout", "stream", "--stream-window", "auto", "--profile", "balanced"], None, None, [], "all"),
    "pack-stream-stdout": ([EB, "pack", "{input}", "-", "--layout", "stream", "--stream-window", "auto", "--profile", "balanced"], None, "{dst}/out.ebs", [], "all"),
    "pack-stream-stdout-window0": ([EB, "pack", "{input}", "-", "--layout", "stream", "--profile", "balanced"], None, "{dst}/out.ebs", [], "all"),
    "pack-indexed-stdout": ([EB, "pack", "{input}", "-", "--layout", "indexed", "--profile", "balanced"], None, "{dst}/out.eb", [], "all"),
    "unpack-indexed": ([EB, "unpack", "{archive_indexed}", "{dst}/tree"], None, None, ["archive_indexed"], "all"),
    "unpack-stream-file": ([EB, "unpack", "{archive_stream}", "{dst}/tree"], None, None, ["archive_stream"], "all"),
    "unpack-stream-stdin": ([EB, "unpack", "-", "{dst}/tree"], "{archive_stream}", None, ["archive_stream"], "all"),
    "verify-indexed": ([EB, "verify", "{archive_indexed}"], None, None, ["archive_indexed"], "all"),
    "verify-stream-stdin": ([EB, "verify", "-"], "{archive_stream}", None, ["archive_stream"], "all"),
    "list-indexed": ([EB, "list", "{archive_indexed}"], None, None, ["archive_indexed"], "all"),
    "list-stream-stdin": ([EB, "list", "-"], "{archive_stream}", None, ["archive_stream"], "all"),
    "inspect-indexed": ([EB, "inspect", "{archive_indexed}", "--json"], None, None, ["archive_indexed"], "all"),
    "explain-indexed": ([EB, "explain", "{archive_indexed}"], None, None, ["archive_indexed"], "all"),
    "read-entry-indexed": ([EB, "read", "{archive_indexed}", "{entry_path}", "--output", "{dst}/entry"], None, None, ["archive_indexed"], "all"),
    "repack-representation": ([EB, "repack", "{archive_indexed}", "{dst}/out.eb"], None, None, ["archive_indexed"], "all"),
    "repack-replan-fast": ([EB, "repack", "{archive_indexed}", "{dst}/out.eb", "--profile", "fast"], None, None, ["archive_indexed"], "all"),
    "repack-to-stream": ([EB, "repack", "{archive_indexed}", "{dst}/out.ebs", "--layout", "stream", "--stream-window", "auto"], None, None, ["archive_indexed"], "all"),
    "repack-to-indexed": ([EB, "repack", "{archive_stream}", "{dst}/out.eb", "--layout", "indexed"], None, None, ["archive_stream"], "all"),
    "sign-detached-indexed": ([EB, "sign", "{archive_indexed}", "--signing-key", "{signing_key}", "--detached", "{dst}/out.ebsig"], None, None, ["archive_indexed", "signing_key"], "all"),
    "inc-tar-zstd-create": (["bash", "-c", "tar -C \"$1\" -cf - . | zstd -T1 -3 -q -o \"$2\"", "sh", "{input}", "{dst}/out.tar.zst"], None, None, [], "all"),
    "inc-tar-zstd-extract": (["bash", "-c", "mkdir -p \"$2\" && zstd -dc -q \"$1\" | tar -C \"$2\" -xf -", "sh", "{incumbent_tar_zst}", "{dst}/tree"], None, None, ["incumbent_tar_zst"], "all"),
}

PREREQS = {
    "archive_indexed": [EB, "pack", "{input}", "{cache}/A.eb", "--profile", "balanced"],
    "archive_stream": [EB, "pack", "{input}", "{cache}/A.ebs", "--layout", "stream", "--stream-window", "auto", "--profile", "balanced"],
    "signing_key": [EB, "key", "generate-signing", "{cache}/signing.key"],
    "incumbent_tar_zst": ["bash", "-c", "tar -C \"$1\" -cf - . | zstd -T1 -3 -q -o \"$2\"", "sh", "{input}", "{cache}/A.tar.zst"],
}


def main() -> None:
    cells = []
    rows = []
    for cid, synth, logical, entries, caps, stage in SHAPES:
        for cap in caps:
            tag = "cap24" if cap == CAP24 else "cap2"
            item_id = f"{cid}-{tag}"
            cells.append({
                "item_id": item_id, "shape_id": cid, "stage": stage, "cap_bytes": cap,
                "synth": synth, "logical_bytes": logical, "entries": entries,
                "declared_disk_peak_bytes": 4 * logical + 3 * GIB,
                "expected_tree_sha256": None,
            })
            if stage == 1:
                rows.append({"item_id": item_id, "split": "synthetic", "family": f"SYN-{cid[0].upper()}",
                             "path": "research/experiments/EXP-SCALE-001/cells.json"})
    doc = {
        "schema": "ebr.scale.cells.v1", "experiment_id": "EXP-SCALE-001",
        "binary": EB, "binary_commit": None,
        "env": {"TMPDIR": "{tmp}", "LANG": "C.UTF-8", "TZ": "UTC"},
        "cells": cells,
        "operations": {k: {"argv": v[0], "stdin": v[1], "stdout": v[2], "prerequisites": v[3], "applicable": v[4]} for k, v in OPS.items()},
        "prerequisites": PREREQS,
    }
    with open(os.path.join(HERE, "cells.json"), "w", encoding="utf-8", newline="\n") as fh:
        json.dump(doc, fh, indent=1, sort_keys=True)
        fh.write("\n")
    with open(os.path.join(HERE, "cells.jsonl"), "w", encoding="utf-8", newline="\n") as fh:
        for r in rows:
            fh.write(json.dumps(r, sort_keys=True) + "\n")


if __name__ == "__main__":
    main()
