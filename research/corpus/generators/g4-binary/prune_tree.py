#!/usr/bin/env python3
"""Run step: prune the staging tree to a set of kept relative paths (g4-binary).

Contract: python prune_tree.py --out <staging> --seed N --params <JSON>
params:
  keep:    [relative paths]  kept entirely (files, symlinks or whole directory subtrees)
  require: [relative paths]  must exist (lstat) before pruning, else fail loudly (upstream drift)
Everything else is removed.  Ancestor directories of kept paths are kept (with their own metadata),
symlinks are never followed.  Deterministic; no content is modified.
"""

import argparse
import json
import os
import shutil
import sys


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True)
    ap.add_argument("--seed", type=int, default=0)
    ap.add_argument("--params", default="{}")
    a = ap.parse_args()
    p = json.loads(a.params)
    keep = sorted(set(k.strip("/") for k in p.get("keep", [])))
    if not keep:
        sys.exit("prune_tree: params.keep is empty")
    for r in p.get("require", []):
        if not os.path.lexists(os.path.join(a.out, r)):
            sys.exit(f"prune_tree: required path {r!r} missing (upstream layout changed?)")

    removed = 0

    def walk(rel):
        nonlocal removed
        base = os.path.join(a.out, rel) if rel else a.out
        for name in sorted(os.listdir(base)):
            crel = f"{rel}/{name}" if rel else name
            full = os.path.join(base, name)
            if crel in keep:
                continue
            is_dir = os.path.isdir(full) and not os.path.islink(full)
            if is_dir and any(k.startswith(crel + "/") for k in keep):
                walk(crel)
                continue
            if is_dir:
                shutil.rmtree(full)
            else:
                os.unlink(full)
            removed += 1

    walk("")
    print(f"prune_tree: kept {len(keep)} prefixes, removed {removed} top-level entries/subtrees")


if __name__ == "__main__":
    main()
