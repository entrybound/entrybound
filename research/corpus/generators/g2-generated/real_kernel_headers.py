#!/usr/bin/env python3
"""Real duplicate-tree layout for family F17 (corpus group g2-generated): unpack kernel
header/devel packages for two ABI versions of the same distribution side by side, so the
item is a real (not generated) near-duplicate tree -- most files are byte-identical between
the two versions (Kconfig, Makefiles, most headers), while a minority differ (version.h,
autoconf.h, a handful of headers touched by that point release/erratum).

Inputs are downloaded package files (declared in the item's recipe.inputs; provision.py
verifies their SHA-256 against the source-of-truth JSON, or TOFU-pins first retrieval).

Params (EB_PARAMS_JSON):
  distro   "ubuntu-deb" | "rpm"
  label_a, label_b   subdirectory names for the two version trees (e.g. "6.8.0-31",
                     "6.19.10-300.fc44")
  inputs_a, inputs_b   lists of EB_INPUT_<NAME> keys (uppercased, '-' -> '_') to unpack into
                       dir_a / dir_b respectively, in order.

Ubuntu: each input is a .deb, unpacked with `dpkg-deb -x` (data members only; no scripts run).
RPM (Fedora / AlmaLinux): each input is an .rpm; converted with `rpm2cpio` and extracted with
`cpio -idm --no-absolute-filenames` (payload only; no scriptlets run). Both are read-only,
non-executing unpacks -- no package is installed or its scripts invoked.
"""

import argparse
import json
import os
import subprocess
from pathlib import Path


def env_input(name: str) -> str:
    key = "EB_INPUT_" + name.upper().replace("-", "_")
    return os.environ[key]


def unpack_deb(deb_path: str, dest: Path) -> None:
    subprocess.run(["dpkg-deb", "-x", deb_path, str(dest)], check=True)


def unpack_rpm(rpm_path: str, dest: Path, scratch: Path) -> None:
    cpio_path = scratch / (Path(rpm_path).name + ".cpio")
    with open(cpio_path, "wb") as f:
        subprocess.run(["rpm2cpio", rpm_path], stdout=f, check=True)
    dest.mkdir(parents=True, exist_ok=True)
    with open(cpio_path, "rb") as f:
        subprocess.run(["cpio", "-idm", "--no-absolute-filenames", "--quiet"], cwd=dest, stdin=f, check=True)
    cpio_path.unlink()


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True)
    ap.add_argument("--seed", type=int, required=True)
    ap.add_argument("--params", default="{}")
    args = ap.parse_args()
    params = json.loads(args.params)
    out = Path(args.out)
    scratch = Path(os.environ.get("EB_SCRATCH", "/tmp"))
    scratch.mkdir(parents=True, exist_ok=True)

    distro = params["distro"]
    dir_a = out / params["label_a"]
    dir_b = out / params["label_b"]
    dir_a.mkdir(parents=True, exist_ok=True)
    dir_b.mkdir(parents=True, exist_ok=True)

    if distro == "ubuntu-deb":
        for name in params["inputs_a"]:
            unpack_deb(env_input(name), dir_a)
        for name in params["inputs_b"]:
            unpack_deb(env_input(name), dir_b)
    elif distro == "rpm":
        for name in params["inputs_a"]:
            unpack_rpm(env_input(name), dir_a, scratch)
        for name in params["inputs_b"]:
            unpack_rpm(env_input(name), dir_b, scratch)
    else:
        raise SystemExit(f"unknown distro {distro!r}")

    # Deterministic mtimes on the unpacked trees (package payloads already carry the
    # packager's recorded mtimes; leave file content untouched, only normalise directory
    # mtimes bumped by extraction order so the tree hash is stable across reruns).
    epoch = int(os.environ.get("SOURCE_DATE_EPOCH", "1767225600"))
    for root in (dir_a, dir_b):
        for dirpath, dirnames, _filenames in os.walk(root):
            os.utime(dirpath, (epoch, epoch))


if __name__ == "__main__":
    main()
