#!/usr/bin/env python3
"""Run step: unpack one declared input into the staging tree with optional member filtering (g4-binary).

Contract: python unpack.py --out <staging> --seed N --params <JSON>; the input path comes from
EB_INPUTS_JSON (set by provision.py).
params:
  input:            declared input name (required)
  format:           7z | deb | rpm | tar | zip
  dest:             relative destination directory inside the item ("" = root)
  strip_components: int (tar only)
  members:          tar only: shell-glob member patterns passed to GNU tar --wildcards (after the archive's
                    own leading directory; use "*/x/*" style patterns)
  include:          globs over extracted relative paths ("**" crosses directories); files/symlinks that match
                    none are deleted
  exclude:          globs; matching files/symlinks are deleted
  prune_empty_dirs: remove directories left empty by filtering (default true when include/exclude given)
  normalize_modes:  set regular files to 0644 (0755 if the source had any x bit or name ends .exe/.dll)
                    and directories to 0755 (for formats without POSIX modes, e.g. 7z from Windows)
Fails loudly when an extractor fails or when filtering leaves nothing.
"""

import argparse
import json
import os
import re
import shutil
import stat
import subprocess
import sys
import tempfile


def glob_to_re(g):
    out, i = "", 0
    while i < len(g):
        c = g[i]
        if g.startswith("**/", i):
            out += "(?:.*/)?"
            i += 3
            continue
        if g.startswith("**", i):
            out += ".*"
            i += 2
            continue
        out += {"*": "[^/]*", "?": "[^/]"}.get(c, re.escape(c))
        i += 1
    return re.compile("^" + out + "$")


def run(cmd, **kw):
    print("+ " + " ".join(cmd), flush=True)
    r = subprocess.run(cmd, **kw)
    if r.returncode != 0:
        sys.exit(f"unpack: command failed ({r.returncode}): {' '.join(cmd)}")
    return r


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True)
    ap.add_argument("--seed", type=int, default=0)
    ap.add_argument("--params", default="{}")
    a = ap.parse_args()
    p = json.loads(a.params)
    inputs = json.loads(os.environ.get("EB_INPUTS_JSON", "{}"))
    name = p["input"]
    if name not in inputs:
        sys.exit(f"unpack: input {name!r} not declared (have {sorted(inputs)})")
    src = inputs[name]
    fmt = p["format"]
    dest_rel = p.get("dest", "")
    if dest_rel.startswith("/") or ".." in dest_rel.split("/"):
        sys.exit("unpack: unsafe dest")
    dest = os.path.join(a.out, dest_rel) if dest_rel else a.out
    os.makedirs(dest, exist_ok=True)

    if fmt == "tar":
        cmd = ["tar", "-x", "-f", src, "-C", dest, "--numeric-owner", "--same-owner", "--same-permissions",
               "--delay-directory-restore"]
        if p.get("strip_components"):
            cmd.append(f"--strip-components={int(p['strip_components'])}")
        if p.get("members"):
            cmd += ["--wildcards", "--"] + list(p["members"])
        run(cmd)
    elif fmt == "7z":
        tmp = tempfile.mkdtemp(prefix="unpack7z.", dir=os.environ.get("EB_SCRATCH"))
        run(["7z", "x", "-y", "-bd", "-bso0", f"-o{tmp}", src])
        for n in sorted(os.listdir(tmp)):
            shutil.move(os.path.join(tmp, n), os.path.join(dest, n))
        shutil.rmtree(tmp)
    elif fmt == "zip":
        run(["bsdtar", "-x", "-p", "--numeric-owner", "-f", src, "-C", dest])
    elif fmt == "deb":
        run(["dpkg-deb", "-x", src, dest])
    elif fmt == "rpm":
        run(["bsdtar", "-x", "-p", "--numeric-owner", "-f", src, "-C", dest])
    else:
        sys.exit(f"unpack: unknown format {fmt}")

    inc = [glob_to_re(g) for g in p.get("include", [])]
    exc = [glob_to_re(g) for g in p.get("exclude", [])]
    filtering = bool(inc or exc)
    removed = kept = 0
    if filtering:
        for root, dirs, files in os.walk(dest, topdown=True):
            dirs.sort()
            for fn in sorted(files) + [d for d in dirs if os.path.islink(os.path.join(root, d))]:
                full = os.path.join(root, fn)
                rel = os.path.relpath(full, dest)
                ok = (not inc or any(r.match(rel) for r in inc)) and not any(r.match(rel) for r in exc)
                if ok:
                    kept += 1
                else:
                    os.unlink(full)
                    removed += 1
        if kept == 0:
            sys.exit("unpack: filters removed every file (upstream layout changed?)")
    if p.get("prune_empty_dirs", filtering):
        for root, dirs, files in os.walk(dest, topdown=False):
            for d in dirs:
                full = os.path.join(root, d)
                if not os.path.islink(full) and not os.listdir(full):
                    os.rmdir(full)
    if p.get("normalize_modes"):
        for root, dirs, files in os.walk(dest):
            for d in dirs:
                full = os.path.join(root, d)
                if not os.path.islink(full):
                    os.chmod(full, 0o755)
            for fn in files:
                full = os.path.join(root, fn)
                st = os.lstat(full)
                if stat.S_ISREG(st.st_mode):
                    x = st.st_mode & 0o111 or fn.lower().endswith((".exe", ".dll"))
                    os.chmod(full, 0o755 if x else 0o644)
        os.chmod(dest, 0o755)
    print(f"unpack: {name} ({fmt}) -> {dest_rel or '.'}; kept {kept} removed {removed} (filtering={filtering})")


if __name__ == "__main__":
    main()
