#!/usr/bin/env python3
"""Print exactly one JSON line describing a just-created baseline artifact.

Usage: finalize_artifact.py <artifact_path>

Used as the last step of every baseline candidate's create command so that
ebr's stdout_json metrics (artifact_sha256, artifact_tree_sha256, artifact_bytes,
artifact_kind) can read the run's own last stdout line (see ebr.run._json_key /
res.last_stdout_line). artifact_tree_sha256 (content_tree_sha256) is computed
uniformly for both single-file artifacts and directory artifacts (borg/restic
repositories), which is what makes determinism comparable across both shapes.
"""
from __future__ import annotations

import hashlib
import json
import os
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from fpshim import fingerprint_path  # noqa: E402


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as fh:
        for chunk in iter(lambda: fh.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def dir_bytes(path: Path) -> int:
    total = 0
    for root, _dirs, files in os.walk(path):
        for f in files:
            fp = os.path.join(root, f)
            try:
                total += os.lstat(fp).st_size
            except OSError:
                pass
    return total


def main(argv=None) -> int:
    argv = argv if argv is not None else sys.argv[1:]
    if len(argv) != 1:
        print(json.dumps({"error": "usage: finalize_artifact.py <artifact_path>"}))
        return 2
    p = Path(argv[0])
    if not p.exists():
        print(json.dumps({"error": f"artifact not found: {p}", "artifact_bytes": None, "artifact_kind": None,
                           "artifact_sha256": None, "artifact_tree_sha256": None}))
        return 1
    is_file = p.is_file()
    kind = "file" if is_file else "dir"
    artifact_sha256 = sha256_file(p) if is_file else None
    artifact_bytes = p.stat().st_size if is_file else dir_bytes(p)
    fp = fingerprint_path(p)
    out = {
        "artifact_kind": kind,
        "artifact_bytes": artifact_bytes,
        "artifact_sha256": artifact_sha256,
        "artifact_tree_sha256": fp["content_tree_sha256"],
        "artifact_logical_tree_sha256": fp["logical_tree_sha256"],
    }
    print(json.dumps(out, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
