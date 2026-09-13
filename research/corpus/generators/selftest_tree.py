#!/usr/bin/env python3
"""Tiny deterministic metadata-rich tree for the corpus tool self-test.

Generator contract (all ebrc generators): invoked by provision.py as
    <interpreter> <script> --out <empty staging dir> --seed <int> --params <canonical JSON>
with EB_* environment variables (EB_SCRATCH, EB_INPUTS_JSON, EB_FROM_JSON, ...), cwd = scratch,
umask 022, LC_ALL=C.UTF-8, TZ=UTC, SOURCE_DATE_EPOCH fixed.  Output must be a pure function
of (script, seed, params, inputs).

params: files (int, default 12), max_size (int bytes, default 20000), special (bool, default true)
Produces: nested dirs, text/random/zero/duplicate files, varied modes, and when special is true
a hardlink pair, relative/absolute/dangling symlinks, a sparse file, a FIFO, a non-UTF-8 name,
a case-colliding pair, a user.* xattr, a POSIX ACL (if setfacl exists), a setgid directory and
fixed mtimes.
"""

import argparse
import json
import os
import random
import shutil
import subprocess

WORDS = ("archive entry bound chunk index manifest stream codec block extent header footer "
         "delta object tree path mode owner group xattr sparse hole link").split()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True)
    ap.add_argument("--seed", type=int, required=True)
    ap.add_argument("--params", default="{}")
    args = ap.parse_args()
    p = json.loads(args.params)
    rng = random.Random(args.seed)
    out = args.out
    n_files = int(p.get("files", 12))
    max_size = int(p.get("max_size", 20000))
    special = bool(p.get("special", True))

    dirs = ["src", "src/lib", "data", "data/deep/er/still", "empty-dir"]
    for d in dirs:
        os.makedirs(os.path.join(out, d), mode=0o755, exist_ok=True)
    written = []
    for i in range(n_files):
        d = rng.choice(dirs[:4])
        name = f"file{i:03d}" + rng.choice([".txt", ".bin", ".c", ".json", ""])
        kind = rng.choice(["text", "random", "zeros", "dup"])
        size = rng.randrange(0, max_size)
        if kind == "text":
            words = []
            total = 0
            while total < size:
                w = rng.choice(WORDS)
                words.append(w)
                total += len(w) + 1
            data = (" ".join(words) + "\n").encode()
        elif kind == "random":
            data = rng.randbytes(size)
        elif kind == "zeros":
            data = bytes(size)
        else:
            data = written[rng.randrange(len(written))][1] if written else b"first\n"
        path = os.path.join(out, d, name)
        with open(path, "wb") as f:
            f.write(data)
        os.chmod(path, rng.choice([0o644, 0o600, 0o755]))
        written.append((path, data))
    with open(os.path.join(out, "empty.txt"), "wb"):
        pass

    if special:
        os.link(written[0][0], os.path.join(out, "data", "hardlink-a"))
        os.link(written[0][0], os.path.join(out, "src", "hardlink-b"))
        os.symlink("../src/lib", os.path.join(out, "data", "link-rel"))
        os.symlink("/etc/hostname", os.path.join(out, "data", "link-abs"))
        os.symlink("does-not-exist", os.path.join(out, "data", "link-dangling"))
        with open(os.path.join(out, "data", "sparse.img"), "wb") as f:
            f.seek(1 << 20)
            f.write(b"x" * 100)
            f.truncate(3 << 20)
        os.mkfifo(os.path.join(out, "data", "fifo"), 0o644)
        with open(os.path.join(os.fsencode(out), b"data", b"name-\xff\xfe.bin"), "wb") as f:
            f.write(b"non-utf8 name\n")
        for n in ("Readme", "README"):
            with open(os.path.join(out, n), "w") as f:
                f.write(n + "\n")
        target = os.path.join(out, "src", "lib")
        os.setxattr(os.path.join(out, "README"), "user.ebrc.note", b"selftest xattr value")
        if shutil.which("setfacl"):
            subprocess.run(["setfacl", "-m", "u:1234:rx", target], check=True)
            subprocess.run(["setfacl", "-d", "-m", "g:2345:r", target], check=True)
        shared = os.path.join(out, "shared")
        os.mkdir(shared)
        os.chmod(shared, 0o2775)
        os.chown(os.path.join(out, "README"), 1000, 1000)

    # deterministic mtimes, children before parents
    paths = []
    for root, dnames, fnames in os.walk(out, topdown=False):
        for n in sorted(fnames) + sorted(dnames):
            paths.append(os.path.join(root, n))
    paths.sort(key=lambda s: (-s.count(os.sep), s))
    base = 1_700_000_000_000_000_000
    for i, path in enumerate(paths):
        t = base + i * 1_000_000_123
        os.utime(path, ns=(t, t), follow_symlinks=False)


if __name__ == "__main__":
    main()
