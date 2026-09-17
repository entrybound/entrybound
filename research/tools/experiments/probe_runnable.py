"""Probe environment prerequisites that decide whether an experiment is runnable now (design review DR1-13; PA-13).

Writes research/experiments/_program/runnable-probe.json, a dated snapshot that build_index.py joins with each
index-fragment row's optional `requires_env` list to compute the `runnable_now` column. The snapshot is committed so
that index generation stays deterministic; re-probe after installing packages or changing the host.

Probe keys (a fragment's requires_env must use these names):
  wsl-qemu-user-aarch64   qemu-aarch64 (user mode) on PATH in WSL
  wsl-qemu-user-x86_64    qemu-x86_64 (user mode) on PATH in WSL
  wsl-qemu-system-x86_64  qemu-system-x86_64 on PATH in WSL
  wsl-kvm                 /dev/kvm present in WSL and kvm_intel nested=Y
  wsl-gcc-aarch64         aarch64-linux-gnu-gcc on PATH in WSL
  wsl-podman              podman on PATH in WSL
  docker-desktop          docker CLI reaches a running engine (Windows)
  wsl-proverif            proverif on PATH in WSL
  wsl-tamarin             tamarin-prover on PATH in WSL
  wsl-par2                par2 on PATH in WSL
  wsl-7z                  7z on PATH in WSL
  wsl-sch-netem           CONFIG_NET_SCH_NETEM in the WSL kernel config
  wsl-dm-flakey           CONFIG_DM_FLAKEY in the WSL kernel config
  wsl-ebr-venv            /root/eb-research/venv/bin/python imports ebr dependencies (yaml)
  windows-edge            Microsoft Edge executable present on the Windows host
  macos                   always false on this host (PLATFORM_BLOCKED)
  native-arm64            always false on this host (PLATFORM_BLOCKED)
  human-participants      always false (no participants available to the program)
  external-reviewers      always false (no independent external reviewers engaged)

Usage (Windows host): C:/Python313/python.exe research/tools/experiments/probe_runnable.py
Read-only: it runs `command -v`, reads /proc and /sys, and asks `docker info`; it installs or changes nothing.
"""

from __future__ import annotations

import json
import os
import subprocess
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
OUT = ROOT / "research" / "experiments" / "_program" / "runnable-probe.json"

WSL_CHECKS = {
    "wsl-qemu-user-aarch64": "command -v qemu-aarch64",
    "wsl-qemu-user-x86_64": "command -v qemu-x86_64",
    "wsl-qemu-system-x86_64": "command -v qemu-system-x86_64",
    "wsl-kvm": "test -e /dev/kvm && grep -qx Y /sys/module/kvm_intel/parameters/nested",
    "wsl-gcc-aarch64": "command -v aarch64-linux-gnu-gcc",
    "wsl-podman": "command -v podman",
    "wsl-proverif": "command -v proverif",
    "wsl-tamarin": "command -v tamarin-prover",
    "wsl-par2": "command -v par2",
    "wsl-7z": "command -v 7z",
    "wsl-sch-netem": "zcat /proc/config.gz | grep -Eq '^CONFIG_NET_SCH_NETEM=(y|m)'",
    "wsl-dm-flakey": "zcat /proc/config.gz | grep -Eq '^CONFIG_DM_FLAKEY=(y|m)'",
    "wsl-ebr-venv": "/root/eb-research/venv/bin/python -c 'import yaml'",
}


# A probe that succeeds can still be unusable by program policy. build_index.py uses `effective`.
POLICY_OVERRIDES = {
    "docker-desktop": (
        "Phase C-design rules state Docker is unavailable, and PROGRESS (storage incident 2026-09-16) defers every "
        "Docker-dependent step until the owner verifies Docker Desktop starts cleanly with its relocated disks; an "
        "engine answering `docker info` at probe time does not change experiment status until that verification is recorded"
    ),
}


def wsl(cmd):
    try:
        r = subprocess.run(["wsl.exe", "-d", "Ubuntu", "--", "bash", "-lc", cmd + " >/dev/null 2>&1"],
                           capture_output=True, timeout=60)
        return r.returncode == 0
    except Exception:  # noqa: BLE001
        return False


def docker_ok():
    try:
        r = subprocess.run(["docker", "info", "--format", "{{.ServerVersion}}"], capture_output=True, timeout=30)
        return r.returncode == 0
    except Exception:  # noqa: BLE001
        return False


def edge_ok():
    for base in (os.environ.get("ProgramFiles(x86)", ""), os.environ.get("ProgramFiles", "")):
        if base and Path(base, "Microsoft", "Edge", "Application", "msedge.exe").is_file():
            return True
    return False


def main():
    probes = {k: wsl(v) for k, v in WSL_CHECKS.items()}
    probes["docker-desktop"] = docker_ok()
    probes["windows-edge"] = edge_ok()
    probes.update({"macos": False, "native-arm64": False, "human-participants": False, "external-reviewers": False})
    doc = {
        "schema": "entrybound.research.runnable-probe/1",
        "probed_utc": datetime.now(timezone.utc).isoformat(timespec="seconds"),
        "generator": "research/tools/experiments/probe_runnable.py",
        "probes": dict(sorted(probes.items())),
        "policy_overrides": POLICY_OVERRIDES,
        "effective": {k: (v and k not in POLICY_OVERRIDES) for k, v in sorted(probes.items())},
    }
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(json.dumps(doc, indent=2) + "\n", encoding="utf-8", newline="\n")
    for k, v in doc["probes"].items():
        print(f"{k}={'yes' if v else 'no'}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
