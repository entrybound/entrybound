#!/usr/bin/env python3
"""Canonical tree fingerprint for a corpus item (format ebrc-fp-v1).

Walks a file or directory WITHOUT following symlinks and emits one canonical
ASCII line per object plus one line per hardlink group, then hashes the
bytewise-sorted lines.  Four digests are produced from the same walk:

  content_tree_sha256   p t s c l g                (portable: survives metadata-lossy copies)
  logical_tree_sha256   p t s c m o l g x r d      (all identity metadata, filesystem-allocation independent;
                                                     used for pins and idempotence)
  tree_sha256           p t s c m o l g x r d b    (adds raw st_blocks: bound to the materializing filesystem)
  extended_tree_sha256  p t s c m o l g x r d b mt (adds mtime_ns; informational)

Object line fields (tab separated, in this order, each "key=value"):
  p  relative path, raw bytes, lowercase hex ("" for the item root)
  t  type: file | dir | symlink | fifo | other-socket | other-chardev | other-blockdev | other-unknown
  s  size: st_size for files, len(target) for symlinks, "-" otherwise
  c  SHA-256 of content for regular files, "-" otherwise
  m  permission bits incl. setuid/setgid/sticky, 4-digit octal
  o  uid:gid (numeric)
  l  symlink target raw bytes hex, "-" otherwise
  g  hardlink group id "h<N>" when >=2 paths inside the item share an inode, "-" otherwise
  x  xattrs: ","-joined "<name hex>:<sha256 of value>" sorted by name bytes, "-" if none
  r  rdev "major:minor" for char/block devices, "-" otherwise
  d  data-extent map from SEEK_DATA/SEEK_HOLE for regular files: "none" (size 0), "full",
     "hole" (no data), "e:<start>-<end>;..." (<=4 extents) or "h:<sha256 of that list>"; "-" otherwise
  b  st_blocks (512-byte units) for regular files, "-" otherwise
  mt mtime_ns ("-" for the item root, whose mtime is not part of the item)
Hardlink group lines:  g=h<N>  t=hardlink-group  n=<paths in item>  p=<lexicographically first path hex>
Group ids are assigned in bytewise order of each group's first path.  Each digest is
SHA-256 over the concatenation of (line + "\\n") for all lines sorted bytewise.

Also reports bytes, file/dir/symlink/hardlink counts, max depth and path-length stats.
"""

from __future__ import annotations

import argparse
import errno
import gzip
import hashlib
import json
import os
import stat
import sys
from concurrent.futures import ThreadPoolExecutor

FORMAT = "ebrc-fp-v1"
CHUNK = 1 << 20  # multiple of 4096 so stats.py block analysis stays aligned


class TreeChanged(Exception):
    pass


class Entry:
    __slots__ = ("rel", "st", "kind", "target", "xattrs", "depth")

    def __init__(self, rel, st, kind, target, xattrs, depth):
        self.rel = rel          # bytes
        self.st = st            # os.stat_result (lstat)
        self.kind = kind        # str
        self.target = target    # bytes | None
        self.xattrs = xattrs    # tuple[(name_bytes, sha256hex, value_len)]
        self.depth = depth      # int


def type_of(mode: int) -> str:
    if stat.S_ISREG(mode):
        return "file"
    if stat.S_ISDIR(mode):
        return "dir"
    if stat.S_ISLNK(mode):
        return "symlink"
    if stat.S_ISFIFO(mode):
        return "fifo"
    if stat.S_ISSOCK(mode):
        return "other-socket"
    if stat.S_ISCHR(mode):
        return "other-chardev"
    if stat.S_ISBLK(mode):
        return "other-blockdev"
    return "other-unknown"


def read_xattrs(path: bytes) -> tuple:
    try:
        names = os.listxattr(path, follow_symlinks=False)
    except OSError as e:
        if e.errno in (errno.ENOTSUP, errno.EOPNOTSUPP):
            return ()
        raise
    out = []
    for name in names:
        value = os.getxattr(path, name, follow_symlinks=False)
        out.append((os.fsencode(name), hashlib.sha256(value).hexdigest(), len(value)))
    out.sort(key=lambda t: t[0])
    return tuple(out)


def join(root: bytes, rel: bytes) -> bytes:
    return root + b"/" + rel if rel else root


class Scan:
    def __init__(self, root: bytes, entries: list, root_type: str):
        self.root = root
        self.entries = entries
        self.root_type = root_type


def scan(root) -> Scan:
    """Metadata-only walk (lstat, readlink, xattrs).  Never follows symlinks and
    refuses to cross into another filesystem."""
    root = os.fsencode(root)
    if len(root) > 1:
        root = root.rstrip(b"/")
    st = os.lstat(root)
    kind = type_of(st.st_mode)
    if kind not in ("file", "dir"):
        raise SystemExit(f"fingerprint root must be a regular file or directory, got {kind}")
    entries = [Entry(b"", st, kind, None, read_xattrs(root), 0)]
    dev = st.st_dev
    if kind == "dir":
        stack = [(b"", 0)]
        while stack:
            rel_dir, depth = stack.pop()
            with os.scandir(join(root, rel_dir)) as it:
                children = [(d.name, d.stat(follow_symlinks=False)) for d in it]
            for name, cst in children:
                rel = rel_dir + b"/" + name if rel_dir else name
                full = join(root, rel)
                if cst.st_dev != dev:
                    raise SystemExit(f"refusing to cross filesystem boundary at {full!r}")
                ck = type_of(cst.st_mode)
                target = os.readlink(full) if ck == "symlink" else None
                entries.append(Entry(rel, cst, ck, target, read_xattrs(full), depth + 1))
                if ck == "dir":
                    stack.append((rel, depth + 1))
    entries.sort(key=lambda e: e.rel)
    return Scan(root, entries, kind)


def data_extents(fd: int, size: int) -> list:
    if size == 0:
        return []
    ext = []
    off = 0
    while off < size:
        try:
            d = os.lseek(fd, off, os.SEEK_DATA)
        except OSError as e:
            if e.errno == errno.ENXIO:
                break
            if e.errno in (errno.EINVAL, errno.ENOTSUP, errno.EOPNOTSUPP):
                ext = [(0, size)]
                break
            raise
        if d >= size:
            break
        h = min(os.lseek(fd, d, os.SEEK_HOLE), size)
        ext.append((d, h))
        off = h
    os.lseek(fd, 0, os.SEEK_SET)
    return ext


def extents_desc(ext: list, size: int) -> str:
    if size == 0:
        return "none"
    if not ext:
        return "hole"
    if ext == [(0, size)]:
        return "full"
    listing = ";".join(f"{a}-{b}" for a, b in ext)
    if len(ext) <= 4:
        return "e:" + listing
    return "h:" + hashlib.sha256(listing.encode("ascii")).hexdigest()


def hash_file(full: bytes, st, sink=None, chunk: int = CHUNK):
    """SHA-256 + extent map of a regular file.  `sink(memoryview)` receives every
    chunk (all chunks except the last are exactly `chunk` bytes)."""
    fd = os.open(full, os.O_RDONLY | os.O_NOFOLLOW | os.O_CLOEXEC)
    try:
        fst = os.fstat(fd)
        if (fst.st_ino, fst.st_dev, fst.st_size) != (st.st_ino, st.st_dev, st.st_size) or not stat.S_ISREG(fst.st_mode):
            raise TreeChanged(f"file changed during fingerprint: {full!r}")
        ext = data_extents(fd, st.st_size)
        h = hashlib.sha256()
        buf = bytearray(chunk)
        mv = memoryview(buf)
        total = 0
        with open(fd, "rb", buffering=0, closefd=False) as f:
            while True:
                n = 0
                while n < chunk:
                    r = f.readinto(mv[n:])
                    if not r:
                        break
                    n += r
                if n == 0:
                    break
                piece = mv[:n]
                h.update(piece)
                if sink is not None:
                    sink(piece)
                total += n
                if n < chunk:
                    break
        if total != st.st_size:
            raise TreeChanged(f"size changed during fingerprint: {full!r}")
        return h.hexdigest(), ext
    finally:
        os.close(fd)


def unique_files(sc: Scan) -> list:
    """First path (bytewise) of each regular-file inode."""
    seen = set()
    out = []
    for e in sc.entries:
        if e.kind == "file":
            key = (e.st.st_dev, e.st.st_ino)
            if key not in seen:
                seen.add(key)
                out.append(e)
    return out


def hash_contents(sc: Scan, jobs: int = 4) -> dict:
    files = unique_files(sc)

    def work(e):
        return (e.st.st_dev, e.st.st_ino), hash_file(join(sc.root, e.rel), e.st)

    if jobs <= 1:
        return dict(work(e) for e in files)
    files = sorted(files, key=lambda e: -e.st.st_size)
    with ThreadPoolExecutor(max_workers=jobs) as ex:
        return dict(ex.map(work, files))


def hardlink_groups(sc: Scan) -> dict:
    by_inode: dict = {}
    for e in sc.entries:  # entries are sorted by rel
        if e.kind != "dir" and e.st.st_nlink > 1:
            by_inode.setdefault((e.st.st_dev, e.st.st_ino), []).append(e.rel)
    groups = [paths for paths in by_inode.values() if len(paths) >= 2]
    groups.sort(key=lambda p: p[0])
    out = {}
    for i, paths in enumerate(groups):
        gid = f"h{i}"
        for rel in paths:
            out[rel] = (gid, len(paths), paths[0])
    return out


def stat_digest(sc: Scan) -> str:
    """Cheap change detector (not portable): lstat fields incl. inode and ctime."""
    h = hashlib.sha256()
    for e in sc.entries:
        s = e.st
        # the item root's times/size change when provision renames staging into place: excluded
        times = "-\t-" if e.rel == b"" else f"{s.st_mtime_ns}\t{s.st_ctime_ns}"
        size = "-" if e.kind == "dir" else str(s.st_size)
        h.update(f"{e.rel.hex()}\t{s.st_mode}\t{size}\t{times}\t{s.st_ino}\t"
                 f"{s.st_nlink}\t{s.st_uid}\t{s.st_gid}\t{s.st_blocks}\n".encode("ascii"))
    return h.hexdigest()


def _pct(sorted_vals, q):
    if not sorted_vals:
        return 0
    idx = min(len(sorted_vals) - 1, max(0, int(round(q * (len(sorted_vals) - 1)))))
    return sorted_vals[idx]


def finalize(sc: Scan, contents: dict, lines_out: str | None = None) -> dict:
    groups = hardlink_groups(sc)
    hashers = {k: hashlib.sha256() for k in ("content", "logical", "tree", "extended")}
    records = []  # (sort_key, content_line, logical_line, tree_suffix, ext_suffix)

    for e in sc.entries:
        s = e.st
        pfx = f"p={e.rel.hex()}\t"
        size = "-"
        csum = "-"
        dmap = "-"
        blocks = "-"
        if e.kind == "file":
            csum, ext = contents[(s.st_dev, s.st_ino)]
            size = str(s.st_size)
            dmap = extents_desc(ext, s.st_size)
            blocks = str(s.st_blocks)
        elif e.kind == "symlink":
            size = str(len(e.target))
        target = e.target.hex() if e.target is not None else "-"
        g = groups.get(e.rel)
        gid = g[0] if g else "-"
        xs = ",".join(f"{n.hex()}:{d}" for n, d, _ in e.xattrs) if e.xattrs else "-"
        rdev = f"{os.major(s.st_rdev)}:{os.minor(s.st_rdev)}" if e.kind in ("other-chardev", "other-blockdev") else "-"
        mode = f"{stat.S_IMODE(s.st_mode):04o}"
        own = f"{s.st_uid}:{s.st_gid}"
        mtime = "-" if e.rel == b"" else str(s.st_mtime_ns)
        content_line = f"{pfx}t={e.kind}\ts={size}\tc={csum}\tl={target}\tg={gid}"
        logical_line = (f"{pfx}t={e.kind}\ts={size}\tc={csum}\tm={mode}\to={own}\tl={target}\tg={gid}"
                        f"\tx={xs}\tr={rdev}\td={dmap}")
        records.append((pfx, content_line, logical_line, f"\tb={blocks}", f"\tmt={mtime}"))
    seen_groups = set()
    extra_paths = 0
    for rel, (gid, n, first) in groups.items():
        if gid in seen_groups:
            continue
        seen_groups.add(gid)
        extra_paths += n - 1
        line = f"g={gid}\tt=hardlink-group\tn={n}\tp={first.hex()}"
        records.append((f"g={gid}\t", line, line, "", ""))
    records.sort(key=lambda r: r[0])

    gz = None
    if lines_out:
        raw = open(lines_out, "wb")
        gz = gzip.GzipFile(filename="", mode="wb", fileobj=raw, mtime=0)
    try:
        for _, cl, ll, bsuf, tsuf in records:
            hashers["content"].update((cl + "\n").encode("ascii"))
            hashers["logical"].update((ll + "\n").encode("ascii"))
            hashers["tree"].update((ll + bsuf + "\n").encode("ascii"))
            full = (ll + bsuf + tsuf + "\n").encode("ascii")
            hashers["extended"].update(full)
            if gz is not None:
                gz.write(full)
    finally:
        if gz is not None:
            gz.close()
            raw.close()

    # summary --------------------------------------------------------------
    counts = {"objects": len(sc.entries), "files": 0, "unique_file_inodes": 0, "dirs": 0, "symlinks": 0,
              "hardlink_groups": len(seen_groups), "hardlink_extra_paths": 0, "fifos": 0, "other": 0}
    bytes_unique = bytes_all = alloc = 0
    seen_inodes = set()
    path_lens = []
    max_depth = 0
    max_name = 0
    for e in sc.entries:
        if e.kind == "file":
            counts["files"] += 1
            bytes_all += e.st.st_size
            key = (e.st.st_dev, e.st.st_ino)
            if key not in seen_inodes:
                seen_inodes.add(key)
                bytes_unique += e.st.st_size
                alloc += e.st.st_blocks * 512
        elif e.kind == "dir":
            if e.rel != b"":
                counts["dirs"] += 1
        elif e.kind == "symlink":
            counts["symlinks"] += 1
        elif e.kind == "fifo":
            counts["fifos"] += 1
        else:
            counts["other"] += 1
        if e.rel:
            path_lens.append(len(e.rel))
            max_name = max(max_name, len(e.rel.rsplit(b"/", 1)[-1]))
            max_depth = max(max_depth, e.depth)
    counts["unique_file_inodes"] = len(seen_inodes)
    counts["hardlink_extra_paths"] = extra_paths
    path_lens.sort()
    return {
        "format": FORMAT,
        "root_type": sc.root_type,
        "tree_sha256": hashers["tree"].hexdigest(),
        "logical_tree_sha256": hashers["logical"].hexdigest(),
        "content_tree_sha256": hashers["content"].hexdigest(),
        "extended_tree_sha256": hashers["extended"].hexdigest(),
        "bytes": bytes_unique,
        "bytes_all_paths": bytes_all,
        "allocated_bytes": alloc,
        "counts": counts,
        "structure": {
            "max_depth": max_depth,
            "max_name_length_bytes": max_name,
            "path_length_bytes": {
                "min": path_lens[0] if path_lens else 0,
                "max": path_lens[-1] if path_lens else 0,
                "mean": round(sum(path_lens) / len(path_lens), 3) if path_lens else 0,
                "p50": _pct(path_lens, 0.50),
                "p95": _pct(path_lens, 0.95),
            },
        },
    }


HELDOUT_SAFE_KEYS = ("format", "root_type", "tree_sha256", "logical_tree_sha256", "content_tree_sha256",
                     "extended_tree_sha256", "bytes")
HELDOUT_SAFE_COUNTS = ("objects", "files", "dirs", "symlinks", "hardlink_groups")


def heldout_safe(fp: dict) -> dict:
    """Fingerprint view allowed for held-out items before design freeze:
    hashes, bytes and basic counts only."""
    out = {k: fp[k] for k in HELDOUT_SAFE_KEYS}
    out["counts"] = {k: fp["counts"][k] for k in HELDOUT_SAFE_COUNTS}
    return out


def fingerprint_path(path, jobs: int = 4, lines_out: str | None = None, with_stat_digest: bool = False) -> dict:
    sc = scan(path)
    contents = hash_contents(sc, jobs=jobs)
    fp = finalize(sc, contents, lines_out=lines_out)
    if with_stat_digest:
        fp["_stat_digest"] = stat_digest(sc)
    return fp


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("path")
    ap.add_argument("--jobs", type=int, default=4, help="content hashing threads (default 4)")
    ap.add_argument("--lines-out", help="write gzip'd extended canonical lines here")
    ap.add_argument("--heldout-safe", action="store_true", help="emit only hashes, bytes and basic counts")
    args = ap.parse_args(argv)
    sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
    from corpuslib import Layout, is_heldout_path  # noqa: E402
    fp = fingerprint_path(args.path, jobs=args.jobs, lines_out=args.lines_out)
    if args.heldout_safe or is_heldout_path(args.path, Layout()):
        fp = heldout_safe(fp)
    json.dump(fp, sys.stdout, indent=2)
    sys.stdout.write("\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
