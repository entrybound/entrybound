#!/usr/bin/env python3
"""Reconstruct a subset of several Linux stable releases side by side (F18 near-duplicate trees).

Run as a recipe `run` step of a download item whose inputs are the base release tarball
(input name "base") and the official cumulative stable patches against it (input names
"patch-<N>", e.g. patch-6.6.30.xz is applied to 6.6 to give 6.6.30).

params:
  base_topdir: top-level directory inside the base tarball, e.g. "linux-6.6"
  subset:      list of top-level paths to keep, e.g. ["fs", "mm", "include"]
  snapshots:   list of stable numbers; 0 means the base release itself
  name_format: output directory name format, e.g. "linux-6.6.{n}" (n=0 uses base_topdir)
Output: <out>/<name>/<subset paths> for every snapshot.  Every snapshot is a byte-exact subset
of the corresponding upstream stable tree: patches are applied with `git apply` restricted to the
subset paths, which fails loudly if any hunk does not apply.
"""

import argparse
import json
import os
import shutil
import subprocess
from pathlib import Path


def run(argv, **kw):
    print("+ " + " ".join(str(a) for a in argv), flush=True)
    subprocess.run([str(a) for a in argv], check=True, stdin=subprocess.DEVNULL, **kw)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True)
    ap.add_argument("--seed", type=int, default=0)
    ap.add_argument("--params", required=True)
    args = ap.parse_args()
    p = json.loads(args.params)
    out = Path(args.out)
    scratch = Path(os.environ["EB_SCRATCH"])
    inputs = json.loads(os.environ["EB_INPUTS_JSON"])
    top = p["base_topdir"]
    subset = sorted(p["subset"])
    base = scratch / "base"
    base.mkdir()
    run(["tar", "-x", "-f", inputs["base"], "-C", base, "--numeric-owner", "--same-owner", "--same-permissions",
         "--strip-components=1"] + [f"{top}/{s}" for s in subset])
    missing = [s for s in subset if not (base / s).exists()]
    if missing:
        raise SystemExit(f"subset paths missing from base tarball: {missing}")
    patchdir = scratch / "patches"
    patchdir.mkdir()
    for n in p["snapshots"]:
        name = top if n == 0 else p["name_format"].format(n=n)
        dest = out / name
        dest.mkdir(parents=True)
        run(["cp", "-a", "--reflink=never", f"{base}/.", f"{dest}/"])
        if n == 0:
            continue
        inp = f"patch-{n}"
        patch = patchdir / f"{inp}.diff"
        with open(patch, "wb") as f:
            run(["xz", "-dc", inputs[inp]], stdout=f)
        includes = []
        for s in subset:
            includes += [f"--include={s}", f"--include={s}/*"]
        env = dict(os.environ, GIT_CONFIG_NOSYSTEM="1", GIT_CONFIG_GLOBAL="/dev/null",
                   GIT_CEILING_DIRECTORIES=str(out))
        # git apply outside a repository behaves like patch(1) but understands git-style
        # renames/modes and refuses fuzzy application (any failing hunk aborts).
        run(["git", "apply", "--whitespace=nowarn", *includes, patch], cwd=dest, env=env)
        patch.unlink()
    shutil.rmtree(base)


if __name__ == "__main__":
    main()
