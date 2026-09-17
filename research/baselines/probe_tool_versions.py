#!/usr/bin/env python3
"""Probe exact installed versions of every tool research/baselines/configs.json
pins, and write research/baselines/tool-versions.json. Run inside WSL Ubuntu
(the tools are Linux binaries); see README.md for the invocation.
"""
from __future__ import annotations

import datetime
import json
import re
import shutil
import subprocess
import sys
from pathlib import Path

TOOLS = {
    "zip": {"cmd": ["zip", "-v"], "pkg": "apt: zip"},
    "unzip": {"cmd": ["unzip", "-v"], "pkg": "apt: unzip"},
    "7z": {"cmd": ["7z"], "pkg": "apt: 7zip (7-Zip 23.01+dfsg)"},
    "tar": {"cmd": ["tar", "--version"], "pkg": "apt: tar (GNU tar)"},
    "gzip": {"cmd": ["gzip", "--version"], "pkg": "apt: gzip"},
    "pigz": {"cmd": ["pigz", "--version"], "pkg": "apt: pigz"},
    "zstd": {"cmd": ["zstd", "--version"], "pkg": "apt: zstd"},
    "xz": {"cmd": ["xz", "--version"], "pkg": "apt: xz-utils"},
    "pixz": {"cmd": ["pixz", "-h"], "pkg": "apt: pixz"},
    "lz4": {"cmd": ["lz4", "--version"], "pkg": "apt: lz4"},
    "brotli": {"cmd": ["brotli", "--version"], "pkg": "apt: brotli"},
    "mksquashfs": {"cmd": ["mksquashfs", "-version"], "pkg": "apt: squashfs-tools"},
    "unsquashfs": {"cmd": ["unsquashfs", "-version"], "pkg": "apt: squashfs-tools"},
    "wimcapture": {"cmd": ["wimcapture", "--version"], "pkg": "apt: wimtools (wimlib)"},
    "wimapply": {"cmd": ["wimapply", "--version"], "pkg": "apt: wimtools (wimlib)"},
    "borg": {"cmd": ["borg", "--version"], "pkg": "apt: borgbackup"},
    "restic": {"cmd": ["restic", "version"], "pkg": "apt: restic"},
    "zpaq": {"cmd": ["zpaq"], "pkg": "apt: zpaq"},
    "mkdwarfs": {"cmd": ["mkdwarfs", "--help"], "pkg": "GitHub release mhx/dwarfs v0.15.7 (dwarfs-universal, static x86_64 Linux binary)"},
    "dwarfsextract": {"cmd": ["dwarfsextract", "--help"], "pkg": "GitHub release mhx/dwarfs v0.15.7 (dwarfs-universal)"},
    "python3": {"cmd": ["python3", "--version"], "pkg": "system"},
    "git": {"cmd": ["git", "--version"], "pkg": "apt: git"},
}

VERSION_RE = re.compile(r"(\d+\.\d+(?:\.\d+)?(?:[.\-+][A-Za-z0-9]+)*)")
DWARFS_VERSION_RE = re.compile(r"\(v(\d+\.\d+\.\d+(?:-[A-Za-z0-9]+)?)")

DPKG_PACKAGE = {
    "zip": "zip", "unzip": "unzip", "7z": "7zip", "tar": "tar", "gzip": "gzip", "pigz": "pigz",
    "zstd": "zstd", "xz": "xz-utils", "pixz": "pixz", "lz4": "lz4", "brotli": "brotli",
    "mksquashfs": "squashfs-tools", "unsquashfs": "squashfs-tools", "wimcapture": "wimtools",
    "wimapply": "wimtools", "borg": "borgbackup", "restic": "restic", "zpaq": "zpaq", "git": "git",
}


def dpkg_version(pkg: str) -> str | None:
    try:
        cp = subprocess.run(["dpkg-query", "-W", "-f=${Version}", pkg], capture_output=True, text=True, timeout=10)
        v = cp.stdout.strip()
        return v or None
    except Exception:  # noqa: BLE001
        return None


def which(name: str) -> str | None:
    p = shutil.which(name)
    if p:
        return p
    for extra in ("/root/eb-research/tools/dwarfs",):
        cand = Path(extra) / name
        if cand.exists():
            return str(cand)
    return None


def probe(name: str, spec: dict) -> dict:
    path = which(name)
    if not path:
        return {"installed": False, "path": None, "version_raw": None, "version": None, "package": spec["pkg"]}
    try:
        cp = subprocess.run(spec["cmd"], capture_output=True, text=True, timeout=20)
        text = (cp.stdout or "") + (cp.stderr or "")
    except Exception as exc:  # noqa: BLE001
        text = f"<probe failed: {exc}>"
    first_lines = "\n".join(text.splitlines()[:8])
    m = DWARFS_VERSION_RE.search(text) or VERSION_RE.search(first_lines)
    dpkg_pkg = DPKG_PACKAGE.get(name)
    return {
        "installed": True,
        "path": path,
        "version_raw": first_lines.strip(),
        "version": m.group(1) if m else None,
        "dpkg_version": dpkg_version(dpkg_pkg) if dpkg_pkg else None,
        "package": spec["pkg"],
    }


def main() -> int:
    out = {
        "schema": "ebr.baseline-tool-versions.v1",
        "probed_at_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(timespec="seconds"),
        "probed_on": "WSL Ubuntu 24.04 (root), per research/PROGRESS.md",
        "tools": {name: probe(name, spec) for name, spec in TOOLS.items()},
    }
    dest = Path(__file__).resolve().parent / "tool-versions.json"
    dest.write_text(json.dumps(out, indent=2, sort_keys=False) + "\n", encoding="utf-8")
    missing = [n for n, v in out["tools"].items() if not v["installed"]]
    print(f"wrote {dest}")
    if missing:
        print(f"missing: {missing}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
