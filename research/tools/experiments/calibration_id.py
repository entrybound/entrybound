"""Calibration identity for timing and memory calibration validity (design review DR1-04).

decision-method.md section 4.7 says calibration "is repeated whenever env_id changes", but env_id
(research/tools/ebr/envinfo.py) hashes the repository commit and, on Windows, Docker Desktop client,
engine and plugin versions. Read literally, every research checkpoint would invalidate calibration.
This tool defines the key the Phase C experiment designs use instead (program amendment PA-04 in
research/experiments/_program/design-revision-round1.md, filed for the round-2 method review):

  calibration_id = sha256(canonical JSON of {environment kind, platform_id, trigger values, runner hooks tree})

The trigger values are exactly the fields whose change plausibly moves timing or memory noise:
OS build and kernel, WSL kernel and WSL version, CPU model and microcode, BIOS, power scheme, overlay and
processor power settings, Defender state (and platform/engine versions when captured), VBS state, WSL
memory and vCPU presentation, the pinned research Rust toolchain, the Python runtime, and the git tree
object of the runner (research/tools/ebr), which carries the timing hooks. Repository commit, Docker
versions, installed tool inventories and paths are deliberately excluded.

Usage:
  python research/tools/experiments/calibration_id.py --env windows-host        # print id and triggers
  python research/tools/experiments/calibration_id.py --env wsl-ubuntu --json
  python research/tools/experiments/calibration_id.py --env wsl-ubuntu --require-calibration --families time_wall,time_cpu
      exit 0 only if research/experiments/_program/calibration-registry.jsonl holds a PASS record for this
      calibration_id covering every requested family (the guarded launcher run_guarded.py calls this for
      every spec with timing: true).

Stdlib only. Missing trigger fields are reported (they must be added to the environment capture before a
calibration campaign; a missing field is hashed as the literal "NOT_CAPTURED").
"""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
ENV_DIR = ROOT / "research" / "environment"
REGISTRY = ROOT / "research" / "experiments" / "_program" / "calibration-registry.jsonl"

TRIGGERS = {
    "windows-host": [
        "platform/os/version",
        "platform/kernel/nt_version",
        "platform/cpu/model_name",
        "platform/cpu/microcode_revision_registry",
        "platform/hardware/bios/SMBIOSBIOSVersion",
        "platform/hardware/bios/ReleaseDate",
        "platform/power/active_scheme/guid",
        "platform/power/power_mode_overlay/ac/guid",
        "platform/power/processor_settings",
        "platform/security/defender/RealTimeProtectionEnabled",
        "platform/security/defender/AMProductVersion",
        "platform/security/defender/AMEngineVersion",
        "platform/virtualization/vbs_status",
        "platform/virtualization/hypervisor_present",
        "platform/virtualization/wsl/Kernel version",
        "platform/virtualization/wsl/WSL version",
        "platform/memory/total_physical_bytes",
        "process/python/version",
        "rust/toolchains/1.98.1-x86_64-pc-windows-msvc/rustc/version_line",
    ],
    "wsl-ubuntu": [
        "platform/parent/platform_id",
        "platform/kernel/release",
        "platform/kernel/version",
        "platform/kernel/clocksource",
        "platform/kernel/transparent_hugepage",
        "platform/virtualization/wsl/wsl_version/WSL version",
        "platform/virtualization/wsl/wsl_version/Kernel version",
        "platform/virtualization/wsl/wsl_version/Windows version",
        "platform/cpu/model_name",
        "platform/cpu/microcode",
        "platform/cpu/logical_cpus",
        "platform/cpu/flags_sha256",
        "platform/memory/mem_total_bytes",
        "platform/memory/swap_total_bytes",
        "process/python/version",
        "rust/toolchains/1.98.1-x86_64-unknown-linux-gnu/rustc/version_line",
    ],
}
# Nested-KVM guest environments (PA-14) register their own trigger list when captured; until then they
# have no calibration identity and their timing is informational.


def dig(obj, path):
    cur = obj
    for part in path.split("/"):
        if isinstance(cur, dict) and part in cur:
            cur = cur[part]
        else:
            return None
    return cur


def load_fingerprint(env_name):
    index = json.loads((ENV_DIR / "index.json").read_text(encoding="utf-8"))
    env_id = index["environments"][env_name]
    return json.loads((ENV_DIR / f"{env_id}.json").read_text(encoding="utf-8"))


def runner_tree():
    try:
        out = subprocess.run(["git", "-C", str(ROOT), "rev-parse", "HEAD:research/tools/ebr"],
                             capture_output=True, text=True, check=True)
        return out.stdout.strip()
    except Exception:  # noqa: BLE001 - reported, not fatal
        return "NOT_AVAILABLE"


def compute(env_name):
    if env_name not in TRIGGERS:
        raise SystemExit(f"no calibration trigger list for {env_name!r}; known: {sorted(TRIGGERS)}")
    doc = load_fingerprint(env_name)
    stable = doc.get("stable", {})
    values, missing = {}, []
    for path in TRIGGERS[env_name]:
        v = dig(stable, path)
        if v is None:
            missing.append(path)
            v = "NOT_CAPTURED"
        values[path] = v
    key = {
        "schema": "entrybound.research.calibration-id/1",
        "environment": env_name,
        "platform_id": doc.get("platform_id"),
        "triggers": values,
        "runner_hooks_tree": runner_tree(),
    }
    canon = json.dumps(key, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode("utf-8")
    return hashlib.sha256(canon).hexdigest(), key, missing, doc.get("env_id")


def require(cal_id, families):
    if not REGISTRY.is_file():
        return False, "calibration registry absent"
    have = set()
    for line in REGISTRY.read_text(encoding="utf-8").splitlines():
        if not line.strip():
            continue
        rec = json.loads(line)
        if rec.get("calibration_id") == cal_id and rec.get("result") == "PASS":
            have.update(rec.get("families", []))
    lacking = [f for f in families if f not in have]
    return (not lacking), ("ok" if not lacking else f"no PASS calibration for families {lacking}")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--env", required=True)
    ap.add_argument("--json", action="store_true")
    ap.add_argument("--require-calibration", action="store_true")
    ap.add_argument("--families", default="")
    args = ap.parse_args()
    cal_id, key, missing, env_id = compute(args.env)
    if args.json:
        print(json.dumps({"calibration_id": cal_id, "env_id": env_id, "key": key, "missing_triggers": missing}, indent=2, sort_keys=True))
    else:
        print(f"calibration_id={cal_id}")
        print(f"env_id={env_id} (not used for calibration validity)")
        for m in missing:
            print(f"MISSING_TRIGGER {m}")
    if args.require_calibration:
        fams = [f for f in args.families.split(",") if f]
        ok, why = require(cal_id, fams)
        print(("CALIBRATION_OK " if ok else "CALIBRATION_MISSING ") + why, file=sys.stderr)
        return 0 if ok else 3
    return 0


if __name__ == "__main__":
    sys.exit(main())
