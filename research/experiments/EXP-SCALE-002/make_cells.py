#!/usr/bin/env python3
"""Generate cells.json and cells.jsonl for EXP-SCALE-002 (design artifact; deterministic).

Run from the repository root:  python research/experiments/EXP-SCALE-002/make_cells.py
Mirrors protocol.md "Candidates" and "Corpus selectors". Same driver contract as
EXP-SCALE-001 (research/tools/scale/scale_cell.py).
"""
import json
import os

HERE = os.path.dirname(os.path.abspath(__file__))
GIB, MIB = 1 << 30, 1 << 20
CAP24, CAP2 = 24 * GIB, 2 * GIB
EB = "/root/eb-research/target/scale-prod/release/ebound"

IMPORTS = ["import-tar", "import-tar-gz", "import-tar-zst", "import-tar-xz", "import-zip", "import-7z", "import-tar-stdin"]
EXPORTS = ["export-zip-file", "export-zip-stdout", "export-tar-file", "export-tar-gz-file", "export-tar-zst-file",
           "export-tar-zst-stdout", "export-zip-dryrun", "publish-three-targets"]
INCS = ["inc-bsdtar-zip-create", "inc-7z-extract"]
SCALING_OPS = IMPORTS + EXPORTS + INCS


def files(n, total, seed, content="half"):
    return {"shape": "files", "files": n, "total_bytes": total, "content": content, "seed": seed}


def entries(n, size, seed, long_names=False):
    d = {"shape": "entries", "entries": n, "entry_bytes": size, "content": "half", "seed": seed}
    if long_names:
        d["long_names"] = True  # driver renames into >100-byte relative paths before tar creation
    return d


CELLS = [
    # id, synth, logical, entries, caps, ops, source overrides
    ("b1g", files(64, 1 * GIB, 1301), 1 * GIB, 64, [CAP24], SCALING_OPS, {}),
    ("b4g", files(64, 4 * GIB, 1302), 4 * GIB, 64, [CAP24, CAP2], SCALING_OPS, {}),
    ("b16g", files(64, 16 * GIB, 1303), 16 * GIB, 64, [CAP24, CAP2], SCALING_OPS, {}),
    ("n1e5", entries(100000, 4096, 1323), 100000 * 4096, 100000, [CAP24, CAP2], SCALING_OPS, {}),
    ("n999k", entries(999000, 4096, 1324), 999000 * 4096, 999000, [CAP24, CAP2], SCALING_OPS, {}),
    ("gz-2p32m1", files(1, (1 << 32) - 1, 1401), (1 << 32) - 1, 1, [CAP24], ["import-gzip-member"], {}),
    ("gz-2p32", files(1, 1 << 32, 1402), 1 << 32, 1, [CAP24], ["import-gzip-member"], {}),
    ("gz-2p32p1", files(1, (1 << 32) + 1, 1403), (1 << 32) + 1, 1, [CAP24], ["import-gzip-member"], {}),
    ("gz-5g", files(1, 5 * GIB, 1404), 5 * GIB, 1, [CAP24], ["import-gzip-member", "export-tar-gz-file"], {}),
    ("zip-65535", entries(65535, 64, 1411), 65535 * 64, 65535, [CAP24], ["import-zip", "export-zip-file"], {}),
    ("zip-65536", entries(65536, 64, 1412), 65536 * 64, 65536, [CAP24], ["import-zip", "export-zip-file", "export-zip-stdout"], {}),
    ("zip64-member", files(1, (1 << 32) + 1, 1413), (1 << 32) + 1, 1, [CAP24], ["import-zip", "export-zip-file"], {}),
    ("tar-8g-member", files(1, (1 << 33) + 1, 1414), (1 << 33) + 1, 1, [CAP24], ["import-tar", "import-tar-gnu", "export-tar-file"], {}),
    ("zero-1g", files(1, 1 * GIB, 1415, content="zeros"), 1 * GIB, 1, [CAP24], ["export-zip-file", "export-tar-gz-file"], {}),
    ("member-5g", files(1, 5 * GIB, 1416), 5 * GIB, 1, [CAP24], ["export-tar-zst-file", "export-zip-file"], {}),
    ("tar-500k", entries(500000, 64, 1421), 500000 * 64, 500000, [CAP24], ["import-tar"], {}),
    ("tar-500k-pax", entries(500000, 64, 1422, long_names=True), 500000 * 64, 500000, [CAP24], ["import-tar"], {}),
    ("tar-999k", entries(999000, 64, 1423), 999000 * 64, 999000, [CAP24], ["import-tar", "import-zip", "export-zip-file", "export-tar-file"], {}),
    ("7z-solid-4g", files(64, 4 * GIB, 1302), 4 * GIB, 64, [CAP24], ["import-7z", "inc-7z-extract"], {"A.7z": "-mx=5 -ms=on"}),
    ("7z-solid-16g", files(64, 16 * GIB, 1303), 16 * GIB, 64, [CAP24], ["import-7z"], {"A.7z": "-mx=5 -ms=on"}),
    ("7z-ultra-1536m", files(64, 1 * GIB, 1301), 1 * GIB, 64, [CAP24], ["import-7z"], {"A.7z": "-mx=9 -md=1536m -ms=on"}),
    ("zst-long31", files(64, 4 * GIB, 1302), 4 * GIB, 64, [CAP24], ["import-tar-zst"], {"A.tar.zst": "--long=31 -19 -T1"}),
]

OPS = {
    "import-tar": [EB, "convert", "{A.tar}", "{dst}/out.eb", "--from", "tar"],
    "import-tar-gnu": [EB, "convert", "{A-gnu.tar}", "{dst}/out.eb", "--from", "tar"],
    "import-tar-gz": [EB, "convert", "{A.tar.gz}", "{dst}/out.eb", "--from", "tar.gz"],
    "import-tar-zst": [EB, "convert", "{A.tar.zst}", "{dst}/out.eb", "--from", "tar.zst"],
    "import-tar-xz": [EB, "convert", "{A.tar.xz}", "{dst}/out.eb", "--from", "tar.xz"],
    "import-zip": [EB, "convert", "{A.zip}", "{dst}/out.eb", "--from", "zip"],
    "import-7z": [EB, "convert", "{A.7z}", "{dst}/out.eb", "--from", "7z"],
    "import-gzip-member": [EB, "convert", "{A.gz}", "{dst}/out.eb", "--from", "gzip", "--entry-name", "payload.bin"],
    "import-tar-stdin": {"argv": [EB, "convert", "-", "{dst}/out.eb", "--from", "tar"], "stdin": "{A.tar}"},
    "export-zip-file": [EB, "convert", "{A.eb}", "{dst}/out.zip", "--to", "zip", "--allow-lossy", "--receipt", "{dst}/receipt.json"],
    "export-zip-stdout": {"argv": [EB, "convert", "{A.eb}", "-", "--to", "zip", "--allow-lossy"], "stdout": "{dst}/out.zip"},
    "export-tar-file": [EB, "convert", "{A.eb}", "{dst}/out.tar", "--to", "tar", "--allow-lossy"],
    "export-tar-gz-file": [EB, "convert", "{A.eb}", "{dst}/out.tar.gz", "--to", "tar.gz", "--allow-lossy"],
    "export-tar-zst-file": [EB, "convert", "{A.eb}", "{dst}/out.tar.zst", "--to", "tar.zst", "--allow-lossy"],
    "export-tar-zst-stdout": {"argv": [EB, "convert", "{A.eb}", "-", "--to", "tar.zst", "--allow-lossy"], "stdout": "{dst}/out.tar.zst"},
    "export-zip-dryrun": [EB, "convert", "{A.eb}", "{dst}/out.zip", "--to", "zip", "--allow-lossy", "--dry-run"],
    "publish-three-targets": [EB, "publish", "{A.eb}", "--output-dir", "{dst}/pub", "--native", "--target", "zip",
                              "--target", "tar.zst", "--base-name", "release", "--allow-lossy", "--report", "{dst}/pub/report.json"],
    "inc-bsdtar-zip-create": ["bsdtar", "-C", "{input}", "--format", "zip", "-cf", "{dst}/out.zip", "."],
    "inc-7z-extract": ["7z", "x", "-y", "-o{dst}/tree", "{A.7z}"],
}

TAR = "tar -C \"$1\" --format=pax --sort=name --mtime=@946684800 --owner=0 --group=0 --numeric-owner -cf - ."
PREREQS = {
    "A.eb": [EB, "pack", "{input}", "{cache}/A.eb", "--profile", "balanced"],
    "A.tar": ["bash", "-c", TAR + " > \"$2\"", "sh", "{input}", "{cache}/A.tar"],
    "A-gnu.tar": ["bash", "-c", TAR.replace("--format=pax", "--format=gnu") + " > \"$2\"", "sh", "{input}", "{cache}/A-gnu.tar"],
    "A.tar.gz": ["bash", "-c", TAR + " | gzip -6 -n > \"$2\"", "sh", "{input}", "{cache}/A.tar.gz"],
    "A.tar.zst": ["bash", "-c", TAR + " | zstd {flags} -q -o \"$2\"", "sh", "{input}", "{cache}/A.tar.zst"],
    "A.tar.xz": ["bash", "-c", TAR + " | xz -T1 -6 > \"$2\"", "sh", "{input}", "{cache}/A.tar.xz"],
    "A.zip": ["bash", "-c", "cd \"$1\" && zip -q -r -X \"$2\" .", "sh", "{input}", "{cache}/A.zip"],
    "A.7z": ["bash", "-c", "cd \"$1\" && 7z a -t7z {flags} \"$2\" ./* > /dev/null", "sh", "{input}", "{cache}/A.7z"],
    "A.gz": ["bash", "-c", "gzip -6 -n -c \"$(find \"$1\" -type f | head -1)\" > \"$2\"", "sh", "{input}", "{cache}/A.gz"],
}
DEFAULT_FLAGS = {"A.tar.zst": "-T1 -3", "A.7z": "-mx=5 -ms=on"}


def op_needs(argv) -> list:
    text = json.dumps(argv)
    return [p for p in PREREQS if "{" + p + "}" in text]


def main() -> None:
    cells, rows = [], []
    for cid, synth, logical, n, caps, ops, overrides in CELLS:
        flags = dict(DEFAULT_FLAGS)
        flags.update(overrides)
        for cap in caps:
            tag = "cap24" if cap == CAP24 else "cap2"
            item_id = f"{cid}-{tag}"
            cells.append({"item_id": item_id, "shape_id": cid, "cap_bytes": cap, "synth": synth,
                          "logical_bytes": logical, "entries": n, "applicable_ops": ops,
                          "source_flags": flags, "declared_disk_peak_bytes": 8 * logical + 3 * GIB,
                          "expected_tree_sha256": None})
            rows.append({"item_id": item_id, "split": "synthetic", "family": "SYN-LEGACY",
                         "path": "research/experiments/EXP-SCALE-002/cells.json"})
    operations = {}
    for name, v in OPS.items():
        d = v if isinstance(v, dict) else {"argv": v}
        operations[name] = {"argv": d["argv"], "stdin": d.get("stdin"), "stdout": d.get("stdout"),
                            "prerequisites": op_needs(d)}
    doc = {"schema": "ebr.scale.cells.v1", "experiment_id": "EXP-SCALE-002", "binary": EB, "binary_commit": None,
           "env": {"TMPDIR": "{tmp}", "LANG": "C.UTF-8", "TZ": "UTC"}, "cells": cells,
           "operations": operations, "prerequisites": PREREQS}
    with open(os.path.join(HERE, "cells.json"), "w", encoding="utf-8", newline="\n") as fh:
        json.dump(doc, fh, indent=1, sort_keys=True)
        fh.write("\n")
    with open(os.path.join(HERE, "cells.jsonl"), "w", encoding="utf-8", newline="\n") as fh:
        for r in rows:
            fh.write(json.dumps(r, sort_keys=True) + "\n")


if __name__ == "__main__":
    main()
