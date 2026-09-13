#!/usr/bin/env python3
"""g1-code build/vendor driver: materialize a real build tree (F02) or dependency tree (F03).

Invoked by provision.py (kind build) as
    python build_tree.py --out <staging> --seed N --params <canonical JSON>

The work happens in a fixed per-item directory, $EB_DATA_ROOT/build/g1-code/<item_id>/, so that
absolute paths recorded inside build trees (depfiles, cargo fingerprints, Makefiles, config.log)
are the same on every rebuild; provision's per-item lock serializes access.  Package-manager
caches live in $EB_CACHE/g1-code/ (downloads there are verified by the lockfile hashes: Cargo.lock
checksums, package-lock integrity, pip --require-hashes).

params (all strings may use {work} {src} {build} {jobs} {cache} {repo} {python} placeholders):
  source:   {"from_item": id} | {"input": name, "strip_components": n} | {"git_archive": true}
            | {"files": [{"input": name, "dest": relpath}]} | {"none": true}   -> populates {src}
  expect:   [[argv, expected_prefix_of_first_output_line], ...]    toolchain pins (fail loudly)
  files_sha256: {repo-relative path: sha256}                         data files read by commands
  env:      {NAME: value}
  jobs:     int (default 16)
  commands: [{"argv": [...], "cwd": "{src}"}, ...]                  run in order, must succeed
  capture:  [{"from": "{build}", "to": "build"}, ...]                moved into the item root
            ("to": "" moves the children of "from" into the root)
The seed is unused (builds are not randomized).  Output is recorded unpinned (output_pin null)
unless the item definition declares otherwise.
"""

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path


def sub(value, ctx):
    if isinstance(value, str):
        for k, v in ctx.items():
            value = value.replace("{" + k + "}", str(v))
        return value
    if isinstance(value, list):
        return [sub(v, ctx) for v in value]
    if isinstance(value, dict):
        return {k: sub(v, ctx) for k, v in value.items()}
    return value


def run(argv, cwd, env):
    print("+ (cd %s && %s)" % (cwd, " ".join(argv)), flush=True)
    r = subprocess.run(argv, cwd=cwd, env=env, stdin=subprocess.DEVNULL)
    if r.returncode != 0:
        raise SystemExit(f"build_tree: command failed (exit {r.returncode}): {argv}")


def sha256_file(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True)
    ap.add_argument("--seed", type=int, required=True)
    ap.add_argument("--params", required=True)
    args = ap.parse_args()
    params = json.loads(args.params)
    out = Path(args.out)
    item_id = os.environ["EB_ITEM_ID"]
    data_root = Path(os.environ["EB_DATA_ROOT"])
    work = data_root / "build" / "g1-code" / item_id
    cache = Path(os.environ["EB_CACHE"]) / "g1-code"
    ctx = {"work": work, "src": work / "src", "build": work / "build", "jobs": int(params.get("jobs", 16)),
           "cache": cache, "repo": os.environ["EB_REPO"], "python": sys.executable}
    env = dict(os.environ)
    env.update(sub(params.get("env", {}), ctx))

    for rel, want in sorted(params.get("files_sha256", {}).items()):
        got = sha256_file(Path(os.environ["EB_REPO"]) / rel)
        if got != want:
            raise SystemExit(f"HASH MISMATCH: {rel} sha256 {got} != params.files_sha256 {want}")
    for argv, prefix in params.get("expect", []):
        argv = sub(argv, ctx)
        r = subprocess.run(argv, env=env, capture_output=True, text=True)
        line = ((r.stdout or r.stderr).strip().splitlines() or [""])[0]
        print(f"toolchain: {' '.join(argv)} -> {line}", flush=True)
        if r.returncode != 0 or not line.startswith(prefix):
            raise SystemExit(f"toolchain pin mismatch: {' '.join(argv)} gave {line!r}, expected prefix {prefix!r}")

    if work.exists():
        shutil.rmtree(work)
    (work / "src").mkdir(parents=True)
    (work / "build").mkdir()
    cache.mkdir(parents=True, exist_ok=True)
    inputs = json.loads(os.environ.get("EB_INPUTS_JSON", "{}"))
    froms = json.loads(os.environ.get("EB_FROM_JSON", "{}"))
    src = params["source"]
    tar_flags = ["--numeric-owner", "--same-owner", "--same-permissions", "--delay-directory-restore"]
    if "from_item" in src:
        run(["cp", "-a", "--reflink=never", froms[src["from_item"]] + "/.", str(work / "src") + "/"], work, env)
    elif "input" in src:
        cmd = ["tar", "-x", "-f", inputs[src["input"]], "-C", str(work / "src")] + tar_flags
        if src.get("strip_components"):
            cmd.append(f"--strip-components={src['strip_components']}")
        run(cmd, work, env)
    elif src.get("git_archive"):
        run(["tar", "-x", "-f", inputs["git-archive"], "-C", str(work / "src")] + tar_flags, work, env)
    elif "files" in src:
        for f in src["files"]:
            dest = work / "src" / f["dest"]
            dest.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(inputs[f["input"]], dest)
            os.chmod(dest, 0o644)
    elif src.get("none"):
        pass
    else:
        raise SystemExit("build_tree: params.source must name from_item, input, git_archive, files or none")

    for c in params["commands"]:
        run(sub(c["argv"], ctx), sub(c.get("cwd", "{src}"), ctx), env)

    for cap in params["capture"]:
        frm = Path(sub(cap["from"], ctx))
        to = sub(cap.get("to", ""), ctx)
        if not frm.is_dir():
            raise SystemExit(f"build_tree: capture source {frm} missing")
        if to:
            dest = out / to
            dest.parent.mkdir(parents=True, exist_ok=True)
            os.rename(frm, dest)
        else:
            for child in sorted(frm.iterdir()):
                os.rename(child, out / child.name)
    os.chmod(out, 0o755)
    shutil.rmtree(work)


if __name__ == "__main__":
    main()
