#!/usr/bin/env python3
"""Generated duplicate-tree layouts for family F17 (corpus group g2-generated).  GENERATED DATA.

A base project tree is generated (mixed text/code, random "compressed" assets, small binaries,
empty files, a few symlinks) and then duplicated inside the item in several ways, each controlled by
params:

  exact_copies     list of relative destinations receiving `cp -a` copies (content + metadata identical)
  renamed_copies   list of destinations receiving copies whose files keep their bytes but get new
                   mtimes (as `cp -r` without -p would) and normalised modes
  vendored_subtree {"count": N, "min_files": M}: a subtree of the base with >= M files is copied into
                   N different vendor paths (e.g. third_party/<lib>, node_modules/<pkg>/node_modules/...)
  nested_copy      bool: a copy of the whole base placed inside itself (backup-inside-project)
  renamed_files    int: files duplicated under different names in other directories
  base_files       int: files in the base tree; base_dir: where the base lives (default "project")

Exact duplication is byte-for-byte; no copy is hardlinked or reflinked (separate inodes), so the
scale tier counts every copy.  mtimes are set deterministically.
"""

import argparse
import hashlib
import json
import math
import os
import random
import shutil
import struct
import subprocess
import sys

EPOCH = int(os.environ.get("SOURCE_DATE_EPOCH", "1767225600"))
MiB = 1 << 20


class Shake:
    def __init__(self, seed, domain):
        self.prefix = domain + b"\0" + struct.pack("<Q", seed & (2**64 - 1))
        self.counter = 0

    def __call__(self, n):
        out = bytearray()
        while len(out) < n:
            take = min(n - len(out), 4 * MiB)
            out += hashlib.shake_256(self.prefix + struct.pack("<Q", self.counter)).digest(take)
            self.counter += 1
        return bytes(out)


SYLL = "ka ri to mo na lu be si ve ro da ne xi pa qu fe zo li ma te ho gu ja ce wi yo ba de fi ga hi ko".split()


def words(rng, n):
    s = set()
    while len(s) < n:
        s.add("".join(rng.choice(SYLL) for _ in range(rng.choice((2, 3, 3, 4)))))
    return sorted(s)


def gen_base(base, rng, rnd, n_files, vocab):
    lines = [" ".join(rng.choice(vocab) for _ in range(rng.randrange(2, 14))) for _ in range(20000)]
    dirs = ["."]
    for i in range(max(4, n_files // 25)):
        parent = rng.choice(dirs)
        d = os.path.normpath(os.path.join(parent, rng.choice(vocab)))
        if d.count("/") < 7 and d not in dirs:
            dirs.append(d)
    for d in dirs:
        os.makedirs(os.path.join(base, d), exist_ok=True)
    made = []
    for i in range(n_files):
        d = dirs[min(len(dirs) - 1, int(rng.paretovariate(0.9)) - 1)] if rng.random() < 0.3 else rng.choice(dirs)
        kind = rng.random()
        if kind < 0.55:
            ext = rng.choice((".c", ".h", ".py", ".js", ".go", ".md", ".txt", ".json", ".yaml"))
            size = max(0, int(math.exp(7.2 + 1.3 * rng.gauss(0, 1))))
            buf, cur = [], 0
            while cur < size:
                ln = lines[rng.randrange(len(lines))] + "\n"
                buf.append(ln)
                cur += len(ln)
            data = "".join(buf).encode()
        elif kind < 0.80:
            ext = rng.choice((".png", ".jpg", ".gz", ".woff2", ".zip"))
            data = rnd(min(8 * MiB, max(64, int(math.exp(9.5 + 1.5 * rng.gauss(0, 1))))))
        elif kind < 0.95:
            ext = rng.choice((".o", ".so", ".bin", ".class", ".pyc"))
            unit = rnd(rng.choice((64, 256, 1024)))
            size = min(4 * MiB, max(64, int(math.exp(9.0 + 1.2 * rng.gauss(0, 1)))))
            data = bytearray()
            while len(data) < size:
                data += unit if rng.random() < 0.6 else rnd(len(unit))
            data = bytes(data[:size])
        else:
            ext = rng.choice(("", ".keep", ".gitkeep"))
            data = b""
        name = f"{rng.choice(vocab)}{i}{ext}"
        path = os.path.join(base, d, name)
        with open(path, "wb") as f:
            f.write(data)
        os.chmod(path, 0o755 if ext in (".so", "") and data and rng.random() < 0.5 else 0o644)
        made.append(os.path.relpath(path, base))
    for i in range(min(20, len(made) // 50)):
        target = rng.choice(made)
        link = os.path.join(base, os.path.dirname(target), f"link-{i}")
        if not os.path.lexists(link):
            os.symlink(os.path.basename(target), link)
    return made


def set_times(root, rng, base_t):
    entries = []
    for r, dn, fn in os.walk(root):
        for n in fn + dn:
            entries.append(os.path.relpath(os.path.join(r, n), root))
    entries.sort(key=lambda s: (-s.count(os.sep), s))
    for rel in entries:
        h = int.from_bytes(hashlib.sha256(rel.encode("utf-8", "surrogateescape")).digest()[:4], "little")
        t = (base_t - h % (86400 * 365)) * 10**9 + h % 10**9
        os.utime(os.path.join(root, rel), ns=(t, t), follow_symlinks=False)


def cp_a(src, dst):
    os.makedirs(os.path.dirname(dst) or ".", exist_ok=True)
    subprocess.run(["cp", "-a", "--reflink=never", src, dst], check=True)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True)
    ap.add_argument("--seed", type=int, required=True)
    ap.add_argument("--params", default="{}")
    args = ap.parse_args()
    p = json.loads(args.params)
    rng = random.Random(args.seed)
    rnd = Shake(args.seed, b"ebrc-g2-f17-duptree")
    vocab = words(rng, 4000)
    out = args.out
    base_rel = p.get("base_dir", "project")
    base = os.path.join(out, base_rel)
    os.makedirs(base)
    made = gen_base(base, rng, rnd, int(p.get("base_files", 1500)), vocab)
    set_times(base, rng, EPOCH - 86400 * 20)
    os.utime(base, ns=(EPOCH * 10**9 - 5, EPOCH * 10**9 - 5))
    summary = {"base_files": len(made)}

    # file-level duplicates under new names (done before copies so they propagate)
    for i in range(int(p.get("renamed_files", 0))):
        src = rng.choice(made)
        dst_dir = os.path.join(base, os.path.dirname(rng.choice(made)))
        dst = os.path.join(dst_dir, f"copy-of-{i}-{os.path.basename(src)}")
        if not os.path.lexists(dst) and not os.path.islink(os.path.join(base, src)):
            shutil.copyfile(os.path.join(base, src), dst)
            st = os.lstat(os.path.join(base, src))
            os.chmod(dst, st.st_mode & 0o7777)
            os.utime(dst, ns=(st.st_mtime_ns, st.st_mtime_ns))

    if p.get("vendored_subtree"):
        vs = p["vendored_subtree"]
        counts = {}
        for rel in made:
            parts = rel.split("/")
            for k in range(1, len(parts)):
                counts["/".join(parts[:k])] = counts.get("/".join(parts[:k]), 0) + 1
        cands = sorted(d for d, c in counts.items() if c >= int(vs.get("min_files", 20)))
        if not cands:
            raise SystemExit("duplicate_tree: no subtree large enough to vendor")
        sub = rng.choice(cands)
        lib = os.path.basename(sub)
        dests = []
        for i in range(int(vs.get("count", 4))):
            style = i % 4
            host = f"app-{vocab[i * 7 % len(vocab)]}"
            if style == 0:
                d = f"workspace/{host}/third_party/{lib}"
            elif style == 1:
                d = f"workspace/{host}/node_modules/{lib}"
            elif style == 2:
                d = f"workspace/{host}/node_modules/{vocab[i]}/node_modules/{lib}"
            else:
                d = f"workspace/{host}/vendor/github.com/example/{lib}"
            cp_a(os.path.join(base, sub), os.path.join(out, d))
            dests.append(d)
        summary["vendored"] = {"subtree": sub, "copies": dests}

    for d in p.get("exact_copies", []):
        cp_a(base, os.path.join(out, d))
    for j, d in enumerate(p.get("renamed_copies", [])):
        dst = os.path.join(out, d)
        cp_a(base, dst)
        for r, dn, fn in os.walk(dst):
            for n in fn:
                fp = os.path.join(r, n)
                if not os.path.islink(fp):
                    os.chmod(fp, 0o644)
        set_times(dst, rng, EPOCH - 3600 * (j + 1))
    if p.get("nested_copy"):
        tmp = os.path.join(out, ".nested-copy-tmp")
        cp_a(base, tmp)  # cp refuses to copy a directory into itself: stage outside, then move in
        os.makedirs(os.path.join(base, "backup"), exist_ok=True)
        os.rename(tmp, os.path.join(base, "backup", "project-before-refactor"))
        set_times(os.path.join(base, "backup"), rng, EPOCH - 86400)
    # directory mtimes are perturbed by copies: normalise all directories deterministically
    dirs = []
    for r, dn, fn in os.walk(out):
        for n in dn:
            dirs.append(os.path.relpath(os.path.join(r, n), out))
    for rel in sorted(dirs, key=lambda s: (-s.count(os.sep), s)):
        h = int.from_bytes(hashlib.sha256(rel.encode()).digest()[:4], "little")
        os.utime(os.path.join(out, rel), ns=((EPOCH - h % 86400) * 10**9,) * 2, follow_symlinks=False)
    summary["copies"] = {"exact": p.get("exact_copies", []), "renamed": p.get("renamed_copies", []),
                         "nested": bool(p.get("nested_copy"))}
    print(json.dumps(summary, sort_keys=True), file=sys.stderr)


if __name__ == "__main__":
    main()
