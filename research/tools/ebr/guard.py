"""Quiet-machine guard and CPU accounting for timing runs.

``other-process CPU`` = (system-wide busy CPU time - this runner's own CPU time -
CPU time of the measured process tree) over a window, as a percentage of all
logical CPUs. On WSL2 the system view is the Linux VM: Windows host activity is
visible only indirectly (steal time counts as busy). Both limits come from
thresholds.json ``quiet_machine_guard`` unless overridden by the spec ``guard``
block or CLI flags; with timing enabled and a required limit unset, the runner
refuses to start.
"""

from __future__ import annotations

import os
import sys
import time
from dataclasses import asdict, dataclass, field
from typing import Any, Dict, List, Optional

import psutil


def system_busy_seconds() -> float:
    ct = psutil.cpu_times()
    d = ct._asdict()
    total = sum(d.values())
    if sys.platform.startswith("linux"):
        total -= d.get("guest", 0.0) + d.get("guest_nice", 0.0)
        idle = d.get("idle", 0.0) + d.get("iowait", 0.0)
    else:
        idle = d.get("idle", 0.0)
    return total - idle


def self_cpu_seconds() -> float:
    t = psutil.Process().cpu_times()
    return float(t.user + t.system)


def children_cpu_seconds() -> float:
    """CPU of terminated, reaped children (POSIX); 0 on Windows (job accounting used instead)."""
    if sys.platform.startswith("win"):
        return 0.0
    t = os.times()
    return float(t.children_user + t.children_system)


def loadavg_1m() -> Optional[float]:
    if hasattr(os, "getloadavg") and not sys.platform.startswith("win"):
        return float(os.getloadavg()[0])
    return None


@dataclass
class CpuWindow:
    """Snapshot pair around a sample; ``tree_cpu_s`` supplied by the caller at stop."""

    t0: float = 0.0
    busy0: float = 0.0
    self0: float = 0.0
    children0: float = 0.0

    @classmethod
    def start(cls) -> "CpuWindow":
        return cls(time.perf_counter(), system_busy_seconds(), self_cpu_seconds(), children_cpu_seconds())

    def stop(self, tree_cpu_s: Optional[float]) -> Dict[str, Any]:
        wall = time.perf_counter() - self.t0
        busy = system_busy_seconds() - self.busy0
        own = self_cpu_seconds() - self.self0
        children = children_cpu_seconds() - self.children0
        tree = tree_cpu_s if tree_cpu_s is not None else children
        # On POSIX the reaped tree is already inside `children`; do not subtract twice.
        subtract = own + (children if not sys.platform.startswith("win") else tree)
        other = max(0.0, busy - subtract)
        ncpu = psutil.cpu_count(logical=True) or 1
        pct = 100.0 * other / (wall * ncpu) if wall > 0 else None
        return {
            "window_s": wall,
            "system_busy_s": busy,
            "runner_cpu_s": own,
            "tree_cpu_s": tree,
            "other_cpu_s": other,
            "other_cpu_percent": pct,
            "logical_cpus": ncpu,
        }


@dataclass
class GuardReading:
    ok: bool
    loadavg_1m: Optional[float]
    other_cpu_percent: Optional[float]
    reasons: List[str] = field(default_factory=list)
    utc: Optional[str] = None

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


@dataclass
class QuietGuard:
    max_loadavg_1m: Optional[float]
    max_other_cpu_percent: Optional[float]
    sample_interval_s: float = 1.0
    sources: Dict[str, str] = field(default_factory=dict)

    def missing_limits(self) -> List[str]:
        missing = []
        if self.max_other_cpu_percent is None:
            missing.append("max_other_cpu_percent")
        if self.max_loadavg_1m is None and loadavg_1m() is not None:
            missing.append("max_loadavg_1m")
        return missing

    def evaluate(self, load: Optional[float], other_pct: Optional[float]) -> GuardReading:
        reasons = []
        if self.max_loadavg_1m is not None and load is not None and load > self.max_loadavg_1m:
            reasons.append(f"loadavg_1m {load:.2f} > {self.max_loadavg_1m}")
        if self.max_other_cpu_percent is not None and other_pct is not None and other_pct > self.max_other_cpu_percent:
            reasons.append(f"other_cpu_percent {other_pct:.2f} > {self.max_other_cpu_percent}")
        from datetime import datetime, timezone

        return GuardReading(not reasons, load, other_pct, reasons, datetime.now(timezone.utc).isoformat())

    def check(self) -> GuardReading:
        """Idle-window check: measures other-process CPU for ``sample_interval_s``."""
        win = CpuWindow.start()
        time.sleep(self.sample_interval_s)
        acct = win.stop(tree_cpu_s=0.0)
        return self.evaluate(loadavg_1m(), acct["other_cpu_percent"])

    def config(self) -> Dict[str, Any]:
        return {
            "max_loadavg_1m": self.max_loadavg_1m,
            "max_other_cpu_percent": self.max_other_cpu_percent,
            "sample_interval_s": self.sample_interval_s,
            "sources": self.sources,
            "scope": "wsl2-vm" if _is_wsl() else sys.platform,
        }


def _is_wsl() -> bool:
    try:
        with open("/proc/version") as fh:
            return "microsoft" in fh.read().lower()
    except OSError:
        return False
