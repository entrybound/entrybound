"""Storage pre-run check and shard hygiene for experiment drivers (design review DR1-05; amendment PA-05).

The WSL distro disk (ext4 inside a sparse VHDX on D:) reports free space that can exceed the physical free
space of D: (the 2026-09-16 storage incident mode). Every experiment driver that writes scratch or retained
outputs calls this before each shard:

  python research/tools/experiments/storage_preflight.py --experiment EXP-SCALE-007 --scratch-cap-gb 150

It refuses (exit 5) when   scratch_cap_gb + floor_gb > physical free space of D:,
where floor_gb defaults to 100 GB (research/orchestration/disk_alarm.py alarms below 100 GB; disk_guard.py keeps
75 GB). The usable budget is min(D: physical free, WSL-visible free) minus the floor. It also reports when the
WSL-visible free space exceeds D: physical free (VHDX overcommit): the WSL number is then not a capacity.

Between shards a driver deletes shard scratch and runs  --trim  (inside WSL as root: fstrim -v /), which returns
freed ext4 blocks to the sparse VHDX so D: space is actually released.

Works on the Windows host (queries WSL through wsl.exe) and inside WSL (reads /mnt/d). Stdlib only.
"""

from __future__ import annotations

import argparse
import json
import os
import platform
import shutil
import subprocess
import sys

GB = 1024 ** 3


def d_free_bytes():
    if platform.system() == "Windows":
        return shutil.disk_usage("D:/").free
    return shutil.disk_usage("/mnt/d").free


def wsl_free_bytes():
    if platform.system() == "Windows":
        out = subprocess.run(["wsl.exe", "-d", "Ubuntu", "--", "df", "-B1", "--output=avail", "/"],
                             capture_output=True, text=True, check=True).stdout.split()
        return int(out[-1])
    return shutil.disk_usage("/").free


def trim():
    if platform.system() == "Windows":
        cmd = ["wsl.exe", "-d", "Ubuntu", "-u", "root", "--", "fstrim", "-v", "/"]
    else:
        cmd = ["fstrim", "-v", "/"]
    return subprocess.run(cmd, capture_output=True, text=True)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--experiment", required=True)
    ap.add_argument("--scratch-cap-gb", type=float, required=True)
    ap.add_argument("--floor-gb", type=float, default=100.0)
    ap.add_argument("--trim", action="store_true", help="run fstrim on the WSL root after a shard's scratch was deleted")
    ap.add_argument("--json", action="store_true")
    args = ap.parse_args()

    if args.trim:
        r = trim()
        sys.stdout.write(r.stdout)
        sys.stderr.write(r.stderr)
        if r.returncode != 0:
            return r.returncode

    d_free = d_free_bytes()
    w_free = wsl_free_bytes()
    usable = min(d_free, w_free) - args.floor_gb * GB
    need = args.scratch_cap_gb * GB
    res = {
        "experiment": args.experiment,
        "d_physical_free_gb": round(d_free / GB, 1),
        "wsl_visible_free_gb": round(w_free / GB, 1),
        "vhdx_overcommit": w_free > d_free,
        "floor_gb": args.floor_gb,
        "usable_gb": round(usable / GB, 1),
        "scratch_cap_gb": args.scratch_cap_gb,
        "decision": "ALLOW" if need <= usable else "REFUSE",
    }
    if args.json:
        print(json.dumps(res, indent=2, sort_keys=True))
    else:
        for k, v in res.items():
            print(f"{k}={v}")
        if res["vhdx_overcommit"]:
            print("NOTE WSL-visible free space exceeds D: physical free space; only D: is a capacity")
    return 0 if res["decision"] == "ALLOW" else 5


if __name__ == "__main__":
    sys.exit(main())
