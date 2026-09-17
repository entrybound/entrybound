"""Refuse to proceed when research storage would consume the C: drive.

Checks (Windows host):
  1. C: free space is at least --min-c-free-gb (default 25 GB).
  2. The WSL distro that hosts /root/eb-research (default Ubuntu) has its
     BasePath on D:, read from HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Lxss.
  3. No Docker Desktop data disk (docker_data.vhdx) exists under C:.

Exit 0 when all checks pass, 1 otherwise (each failure printed). Workflows and
long-running experiments must run this before launching and between stages.

Usage: python research/orchestration/disk_guard.py [--min-c-free-gb 25] [--distro Ubuntu]
"""

import argparse
import os
import pathlib
import shutil
import sys


def distro_base_paths():
    import winreg

    paths = {}
    root = r"Software\Microsoft\Windows\CurrentVersion\Lxss"
    with winreg.OpenKey(winreg.HKEY_CURRENT_USER, root) as key:
        index = 0
        while True:
            try:
                sub = winreg.EnumKey(key, index)
            except OSError:
                break
            index += 1
            with winreg.OpenKey(key, sub) as distro:
                try:
                    name = winreg.QueryValueEx(distro, "DistributionName")[0]
                    base = winreg.QueryValueEx(distro, "BasePath")[0]
                except OSError:
                    continue
                paths[name] = base
    return paths


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--min-c-free-gb", type=float, default=25.0)
    parser.add_argument("--distro", default="Ubuntu")
    args = parser.parse_args()
    failures = []

    free_gb = shutil.disk_usage("C:\\").free / 1024**3
    print(f"C: free {free_gb:.1f} GB (minimum {args.min_c_free_gb} GB)")
    if free_gb < args.min_c_free_gb:
        failures.append(f"C: free space {free_gb:.1f} GB is below {args.min_c_free_gb} GB")

    base = distro_base_paths().get(args.distro)
    print(f"WSL distro {args.distro} BasePath: {base}")
    if base is None:
        failures.append(f"WSL distro {args.distro} is not registered")
    elif not base.replace("\\\\?\\", "").upper().startswith("D:"):
        failures.append(f"WSL distro {args.distro} disk is not on D: ({base})")

    docker_c = pathlib.Path(os.environ.get("LOCALAPPDATA", r"C:\Users\Default\AppData\Local")) / "Docker" / "wsl" / "disk" / "docker_data.vhdx"
    if docker_c.exists():
        failures.append(f"Docker data disk is on C: ({docker_c}, {docker_c.stat().st_size / 1024**3:.1f} GB)")

    for failure in failures:
        print(f"DISK GUARD FAILURE: {failure}")
    sys.exit(1 if failures else 0)


if __name__ == "__main__":
    main()
