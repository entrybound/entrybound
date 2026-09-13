#!/usr/bin/env python3
"""Prune an extracted container root filesystem down to selected subtrees (corpus group g2-generated).

Used as the generator of docker-export items: provision.py first extracts the digest-pinned
image export into the staging directory, then this script deletes everything that is not one
of the `keep` paths or an ancestor directory of one.  Kept subtrees are untouched (content,
modes, owners, xattrs, hardlinks, symlinks and mtimes exactly as exported); ancestor
directories keep their exported metadata.

params:
  keep                      list of relative paths that must exist in the export (required).
                            No component of a keep path may be a symlink.
  drop_docker_placeholders  bool, default true: remove the empty placeholder files that
                            `docker export` adds (.dockerenv, etc/hostname, etc/hosts,
                            etc/resolv.conf) when they are empty regular files.

Fails loudly if a keep path is missing or traverses a symlink.  Output is a pure function of
the export tar and params.
"""

import argparse
import json
import os
import shutil
import stat
import sys

PLACEHOLDERS = (".dockerenv", "etc/hostname", "etc/hosts", "etc/resolv.conf")


def safe_rel(rel: str) -> bool:
    if not rel or rel.startswith("/") or "\\" in rel or "\x00" in rel:
        return False
    return all(p not in ("", ".", "..") for p in rel.split("/"))


def remove(path: bytes) -> None:
    st = os.lstat(path)
    if stat.S_ISDIR(st.st_mode):
        # make every directory traversable/removable even if exported with mode 0000
        for root, dirs, _files in os.walk(path):
            for d in dirs:
                p = os.path.join(root, d)
                if not os.path.islink(p):
                    os.chmod(p, 0o700)
        os.chmod(path, 0o700)
        shutil.rmtree(path)
    else:
        os.unlink(path)


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True)
    ap.add_argument("--seed", type=int, required=True)
    ap.add_argument("--params", default="{}")
    args = ap.parse_args()
    params = json.loads(args.params)
    keep = params.get("keep")
    if not isinstance(keep, list) or not keep or not all(isinstance(k, str) and safe_rel(k) for k in keep):
        raise SystemExit("subtree_extract: params.keep must be a non-empty list of safe relative paths")
    out = os.fsencode(args.out)
    keep_b = sorted({os.fsencode(k) for k in keep})

    for k in keep_b:
        parts = k.split(b"/")
        for i in range(1, len(parts) + 1):
            p = os.path.join(out, *parts[:i])
            try:
                st = os.lstat(p)
            except FileNotFoundError:
                raise SystemExit(f"subtree_extract: keep path {k.decode(errors='replace')} missing in export")
            if stat.S_ISLNK(st.st_mode):
                raise SystemExit(f"subtree_extract: keep path {k.decode(errors='replace')} traverses symlink "
                                 f"{b'/'.join(parts[:i]).decode(errors='replace')}")

    ancestors = set()
    for k in keep_b:
        parts = k.split(b"/")
        for i in range(1, len(parts)):
            ancestors.add(b"/".join(parts[:i]))
    keep_set = set(keep_b)

    removed = 0

    def prune(rel_dir: bytes) -> None:
        nonlocal removed
        full_dir = os.path.join(out, rel_dir) if rel_dir else out
        dst = os.lstat(full_dir)
        for name in sorted(os.listdir(full_dir)):
            rel = rel_dir + b"/" + name if rel_dir else name
            if rel in keep_set:
                continue
            if rel in ancestors:
                prune(rel)
                continue
            remove(os.path.join(out, rel))
            removed += 1
        # keep the exported directory mtime (only the extended digest sees it)
        os.utime(full_dir, ns=(dst.st_atime_ns, dst.st_mtime_ns), follow_symlinks=False)

    prune(b"")

    dropped = []
    if params.get("drop_docker_placeholders", True):
        for ph in PLACEHOLDERS:
            p = os.path.join(out, os.fsencode(ph))
            try:
                st = os.lstat(p)
            except FileNotFoundError:
                continue
            if stat.S_ISREG(st.st_mode) and st.st_size == 0:
                # restore parent mtime so dropping a placeholder does not perturb it
                parent = os.path.dirname(p)
                pst = os.lstat(parent)
                os.unlink(p)
                os.utime(parent, ns=(pst.st_atime_ns, pst.st_mtime_ns), follow_symlinks=False)
                dropped.append(ph)
    print(json.dumps({"kept": [k.decode() for k in keep_b], "removed_top_entries": removed,
                      "dropped_placeholders": dropped}), file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
