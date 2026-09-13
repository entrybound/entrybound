"""Single-command execution with resource measurement.

Linux: ``[taskset -c CPUS] /usr/bin/time -v -o FILE <argv>``; the runner reaps the
``time`` process with ``os.wait4`` for microsecond user/sys rusage, parses the
``-v`` report for max RSS / faults / context switches / FS in-out, and measures
wall time with ``perf_counter_ns`` around spawn→reap.

Windows: process created suspended inside a Job Object (exact tree CPU and I/O),
peak working set sampled with psutil across the process tree, CPU affinity
applied through psutil to the runner (inherited by the child) for the sample.

Both: a poller thread tracks the per-sample scratch directory's peak size and
enforces the optional timeout. stdout/stderr go to files (outside scratch) and
are recorded as SHA-256 digests + byte counts.
"""

from __future__ import annotations

import hashlib
import os
import shlex
import shutil
import signal
import subprocess
import sys
import threading
import time
from dataclasses import dataclass, field
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Dict, List, Optional, Sequence, Union

import psutil

from .guard import CpuWindow
from .timev import TimeVParseError, exit_code_from, parse_time_v

IS_WINDOWS = sys.platform.startswith("win")
IS_LINUX = sys.platform.startswith("linux")
GNU_TIME = "/usr/bin/time"
STDERR_EXCERPT_BYTES = 2048


def utcnow() -> str:
    return datetime.now(timezone.utc).isoformat(timespec="microseconds")


# ----------------------------------------------------------------------------
# rendering


def quote_for(shell: str, value: str) -> str:
    if shell in ("bash", "sh"):
        return shlex.quote(value)
    if shell == "cmd":
        return subprocess.list2cmdline([value])
    if shell == "pwsh":
        return "'" + value.replace("'", "''") + "'"
    raise ValueError(shell)


def render(template: Union[str, Sequence[str]], ctx: Dict[str, Any], shell: str) -> Dict[str, Any]:
    """Return {'display': str, 'argv': list|None, 'shell_string': str|None}."""
    if isinstance(template, str):
        s = template.format(**{k: quote_for(shell, str(v)) for k, v in ctx.items()})
        if shell == "bash":
            argv = ["bash", "-o", "pipefail", "-c", s]
        elif shell == "sh":
            argv = ["sh", "-c", s]
        elif shell == "pwsh":
            argv = ["pwsh", "-NoProfile", "-NonInteractive", "-Command", s]
        else:  # cmd
            return {"display": s, "argv": None, "shell_string": s}
        return {"display": s, "argv": argv, "shell_string": None}
    argv = [part.format(**{k: str(v) for k, v in ctx.items()}) for part in template]
    return {"display": subprocess.list2cmdline(argv) if IS_WINDOWS else shlex.join(argv), "argv": argv, "shell_string": None}


# ----------------------------------------------------------------------------
# helpers


def dir_usage(path: Path) -> tuple:
    """(apparent_bytes, allocated_bytes|None) for a file or directory tree; tolerant of races."""
    apparent = 0
    alloc: Optional[int] = 0 if not IS_WINDOWS else None
    stack = [str(path)]
    while stack:
        cur = stack.pop()
        try:
            with os.scandir(cur) as it:
                for de in it:
                    try:
                        st = de.stat(follow_symlinks=False)
                    except OSError:
                        continue
                    if de.is_dir(follow_symlinks=False):
                        stack.append(de.path)
                    else:
                        apparent += st.st_size
                        if alloc is not None:
                            alloc += getattr(st, "st_blocks", 0) * 512
        except (FileNotFoundError, NotADirectoryError, PermissionError):
            continue
    return apparent, alloc


def path_size(path: Path) -> Optional[int]:
    if not path.exists():
        return None
    if path.is_file():
        return path.stat().st_size
    return dir_usage(path)[0]


def digest_file(path: Path) -> tuple:
    h = hashlib.sha256()
    n = 0
    with open(path, "rb") as fh:
        for chunk in iter(lambda: fh.read(1 << 20), b""):
            h.update(chunk)
            n += len(chunk)
    return h.hexdigest(), n


@dataclass
class ExecResult:
    exit_code: Optional[int]
    timed_out: bool
    start_utc: str
    end_utc: str
    wall_s: float
    resource: Dict[str, Any]
    scratch: Dict[str, Any]
    measurement: Dict[str, Any]
    cpu_window: Dict[str, Any]
    stdout_sha256: str
    stdout_bytes: int
    stderr_sha256: str
    stderr_bytes: int
    stderr_excerpt: Optional[str]
    last_stdout_line: Optional[str]
    errors: List[str] = field(default_factory=list)


class _Poller(threading.Thread):
    def __init__(self, scratch: Path, interval: float, timeout: Optional[float], kill, track_tree_pid: Optional[int]):
        super().__init__(daemon=True)
        self.scratch = scratch
        self.interval = interval
        self.timeout = timeout
        self.kill = kill
        self.stop_evt = threading.Event()
        self.peak_apparent = 0
        self.peak_alloc: Optional[int] = None if IS_WINDOWS else 0
        self.polls = 0
        self.timed_out = False
        self.t0 = time.perf_counter()
        self.tree_pid = track_tree_pid
        self.peak_wset: Dict[int, int] = {}
        self.peak_tree_rss_sum = 0

    def _sample_tree(self) -> None:
        try:
            root = psutil.Process(self.tree_pid)
            procs = [root] + root.children(recursive=True)
        except psutil.Error:
            return
        total = 0
        for p in procs:
            try:
                mi = p.memory_info()
            except psutil.Error:
                continue
            peak = getattr(mi, "peak_wset", None) or mi.rss
            if peak > self.peak_wset.get(p.pid, 0):
                self.peak_wset[p.pid] = peak
            total += mi.rss
        self.peak_tree_rss_sum = max(self.peak_tree_rss_sum, total)

    def run(self) -> None:
        while not self.stop_evt.is_set():
            a, b = dir_usage(self.scratch)
            self.polls += 1
            self.peak_apparent = max(self.peak_apparent, a)
            if b is not None and self.peak_alloc is not None:
                self.peak_alloc = max(self.peak_alloc, b)
            if self.tree_pid is not None:
                self._sample_tree()
            if self.timeout is not None and time.perf_counter() - self.t0 > self.timeout and not self.timed_out:
                self.timed_out = True
                self.kill()
            self.stop_evt.wait(self.interval)

    def stop(self) -> None:
        self.stop_evt.set()
        self.join()


def _finish_io(io_dir: Path, errors: List[str]) -> Dict[str, Any]:
    so, se = io_dir / "stdout", io_dir / "stderr"
    so_d, so_n = digest_file(so)
    se_d, se_n = digest_file(se)
    last_line = None
    with open(so, "rb") as fh:
        if so_n:
            fh.seek(max(0, so_n - 65536))
            tail = fh.read().decode("utf-8", "replace").splitlines()
            tail = [t for t in tail if t.strip()]
            last_line = tail[-1] if tail else None
    with open(se, "rb") as fh:
        excerpt = fh.read(STDERR_EXCERPT_BYTES).decode("utf-8", "replace")
    return {
        "stdout_sha256": so_d,
        "stdout_bytes": so_n,
        "stderr_sha256": se_d,
        "stderr_bytes": se_n,
        "stderr_excerpt_all": excerpt,
        "last_stdout_line": last_line,
    }


def execute(
    rendered: Dict[str, Any],
    *,
    cwd: Optional[str],
    env: Dict[str, str],
    scratch: Path,
    io_dir: Path,
    poll_interval_s: float,
    timeout_s: Optional[float],
    cpu_affinity: Optional[List[int]],
    measure: bool = True,
) -> ExecResult:
    io_dir.mkdir(parents=True, exist_ok=True)
    if IS_WINDOWS:
        return _execute_windows(rendered, cwd, env, scratch, io_dir, poll_interval_s, timeout_s, cpu_affinity)
    return _execute_posix(rendered, cwd, env, scratch, io_dir, poll_interval_s, timeout_s, cpu_affinity, measure)


def _execute_posix(rendered, cwd, env, scratch, io_dir, poll, timeout, affinity, measure) -> ExecResult:
    errors: List[str] = []
    argv = list(rendered["argv"])
    tv_path = io_dir / "timev.txt"
    if tv_path.exists():
        tv_path.unlink()
    use_time = measure and IS_LINUX and os.access(GNU_TIME, os.X_OK)
    full = ([GNU_TIME, "-v", "-o", str(tv_path)] if use_time else []) + argv
    if affinity:
        taskset = shutil.which("taskset")
        if not taskset:
            raise RuntimeError("cpu_affinity requested but taskset is not installed")
        full = [taskset, "-c", ",".join(str(c) for c in affinity)] + full
    with open(io_dir / "stdout", "wb") as so, open(io_dir / "stderr", "wb") as se:
        win = CpuWindow.start()
        start_utc = utcnow()
        t0 = time.perf_counter_ns()
        p = subprocess.Popen(full, cwd=cwd, env=env, stdout=so, stderr=se, stdin=subprocess.DEVNULL, start_new_session=True)

        def kill() -> None:
            try:
                os.killpg(p.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass

        poller = _Poller(scratch, poll, timeout, kill, None)
        poller.start()
        while True:
            try:
                _, status, ru = os.wait4(p.pid, 0)
                break
            except InterruptedError:
                continue
        t1 = time.perf_counter_ns()
        end_utc = utcnow()
        p.returncode = os.waitstatus_to_exitcode(status)
        poller.stop()
        # kill stray background descendants left in the session
        try:
            os.killpg(p.pid, signal.SIGKILL)
        except (ProcessLookupError, PermissionError):
            pass
    wall = (t1 - t0) / 1e9
    tv: Dict[str, Any] = {}
    exit_code: Optional[int] = p.returncode
    if use_time:
        try:
            tv = parse_time_v(tv_path.read_text(encoding="utf-8", errors="replace"))
            exit_code = exit_code_from(tv)
        except (OSError, TimeVParseError) as exc:
            errors.append(f"time -v parse failed: {exc}")
    cpu = win.stop(tree_cpu_s=None)
    final_a, final_b = dir_usage(scratch)
    io = _finish_io(io_dir, errors)
    resource = {
        "wall_s": wall,
        "user_s": ru.ru_utime,
        "sys_s": ru.ru_stime,
        "cpu_s": ru.ru_utime + ru.ru_stime,
        "max_rss_bytes": tv["max_rss_kib"] * 1024 if "max_rss_kib" in tv else ru.ru_maxrss * 1024,
        "fs_inputs": tv.get("fs_inputs", ru.ru_inblock),
        "fs_outputs": tv.get("fs_outputs", ru.ru_oublock),
        "major_faults": tv.get("major_faults", ru.ru_majflt),
        "minor_faults": tv.get("minor_faults", ru.ru_minflt),
        "voluntary_ctx": tv.get("voluntary_ctx", ru.ru_nvcsw),
        "involuntary_ctx": tv.get("involuntary_ctx", ru.ru_nivcsw),
        "timev_elapsed_s": tv.get("elapsed_s"),
    }
    measurement = {
        "method": "gnu-time-v+wait4" if use_time else "wait4",
        "wall_clock": "perf_counter_ns spawn->reap (includes time/shell startup)",
        "timev": tv or None,
        "rusage": {
            "ru_utime": ru.ru_utime, "ru_stime": ru.ru_stime, "ru_maxrss_kib": ru.ru_maxrss,
            "ru_inblock": ru.ru_inblock, "ru_oublock": ru.ru_oublock,
            "ru_majflt": ru.ru_majflt, "ru_minflt": ru.ru_minflt,
            "ru_nvcsw": ru.ru_nvcsw, "ru_nivcsw": ru.ru_nivcsw,
            "note": "includes the /usr/bin/time wrapper itself" if use_time else None,
        },
        "argv": full,
        "cpu_affinity": affinity,
        "scratch_polls": poller.polls,
    }
    scratch_m = {
        "scratch_peak_bytes": max(poller.peak_apparent, final_a),
        "scratch_peak_alloc_bytes": None if final_b is None else max(poller.peak_alloc or 0, final_b),
        "scratch_final_bytes": final_a,
    }
    excerpt = io.pop("stderr_excerpt_all")
    return ExecResult(
        exit_code=None if poller.timed_out else exit_code,
        timed_out=poller.timed_out,
        start_utc=start_utc,
        end_utc=end_utc,
        wall_s=wall,
        resource=resource,
        scratch=scratch_m,
        measurement=measurement,
        cpu_window=cpu,
        stderr_excerpt=excerpt if (exit_code != 0 or poller.timed_out) else None,
        errors=errors,
        **io,
    )


def _execute_windows(rendered, cwd, env, scratch, io_dir, poll, timeout, affinity) -> ExecResult:
    from . import winjob

    errors: List[str] = []
    me = psutil.Process()
    old_aff = None
    if affinity:
        old_aff = me.cpu_affinity()
        me.cpu_affinity(list(affinity))
    job = winjob.Job()
    try:
        with open(io_dir / "stdout", "wb") as so, open(io_dir / "stderr", "wb") as se:
            win = CpuWindow.start()
            start_utc = utcnow()
            t0 = time.perf_counter_ns()
            flags = winjob.CREATE_SUSPENDED | subprocess.CREATE_NEW_PROCESS_GROUP
            if rendered["argv"] is not None:
                p = subprocess.Popen(rendered["argv"], cwd=cwd, env=env, stdout=so, stderr=se, stdin=subprocess.DEVNULL, creationflags=flags)
            else:
                p = subprocess.Popen(rendered["shell_string"], shell=True, cwd=cwd, env=env, stdout=so, stderr=se, stdin=subprocess.DEVNULL, creationflags=flags)
            handle = int(p._handle)  # noqa: SLF001 - documented CPython attribute on Windows
            try:
                job.assign_and_resume(handle)
            except OSError as exc:
                p.kill()
                p.wait()
                raise RuntimeError(f"cannot place sample process in a job object: {exc}") from exc
            poller = _Poller(scratch, poll, timeout, lambda: job.terminate(1), p.pid)
            poller.start()
            p.wait()
            t1 = time.perf_counter_ns()
            end_utc = utcnow()
            poller.stop()
            acct = job.accounting()
            child_peak = winjob.peak_working_set(handle)
    finally:
        job.close()
        if old_aff is not None:
            me.cpu_affinity(old_aff)
    wall = (t1 - t0) / 1e9
    cpu = win.stop(tree_cpu_s=acct["user_s"] + acct["sys_s"])
    peak_sampled = max(poller.peak_wset.values(), default=0)
    final_a, _ = dir_usage(scratch)
    io = _finish_io(io_dir, errors)
    resource = {
        "wall_s": wall,
        "user_s": acct["user_s"],
        "sys_s": acct["sys_s"],
        "cpu_s": acct["user_s"] + acct["sys_s"],
        "max_rss_bytes": max(peak_sampled, child_peak or 0),
        "tree_peak_rss_sum_bytes": poller.peak_tree_rss_sum,
        "io_read_bytes": acct["io_read_bytes"],
        "io_write_bytes": acct["io_write_bytes"],
        "major_faults": None,
        "minor_faults": acct["page_faults"],
    }
    measurement = {
        "method": "win-job+psutil-peak-wset",
        "wall_clock": "perf_counter_ns spawn->exit (includes suspended-create/job assignment)",
        "job": acct,
        "child_peak_working_set_bytes": child_peak,
        "sampled_peak_working_set_by_pid": {str(k): v for k, v in sorted(poller.peak_wset.items())},
        "argv": rendered["argv"] if rendered["argv"] is not None else rendered["shell_string"],
        "cpu_affinity": affinity,
        "scratch_polls": poller.polls,
        "note": "max_rss_bytes = max(per-process sampled peak_wset, direct child PeakWorkingSetSize)",
    }
    exit_code = p.returncode
    excerpt = io.pop("stderr_excerpt_all")
    return ExecResult(
        exit_code=None if poller.timed_out else exit_code,
        timed_out=poller.timed_out,
        start_utc=start_utc,
        end_utc=end_utc,
        wall_s=wall,
        resource=resource,
        scratch={"scratch_peak_bytes": max(poller.peak_apparent, final_a), "scratch_peak_alloc_bytes": None, "scratch_final_bytes": final_a},
        measurement=measurement,
        cpu_window=cpu,
        stderr_excerpt=excerpt if (exit_code != 0 or poller.timed_out) else None,
        errors=errors,
        **io,
    )


def run_check(rendered: Dict[str, Any], *, cwd, env, timeout_s) -> Dict[str, Any]:
    """Unmeasured helper command (e.g. round-trip verification)."""
    try:
        if rendered["argv"] is not None:
            cp = subprocess.run(rendered["argv"], cwd=cwd, env=env, capture_output=True, timeout=timeout_s, stdin=subprocess.DEVNULL)
        else:
            cp = subprocess.run(rendered["shell_string"], shell=True, cwd=cwd, env=env, capture_output=True, timeout=timeout_s, stdin=subprocess.DEVNULL)
    except subprocess.TimeoutExpired:
        return {"exit_code": None, "timed_out": True, "stdout_sha256": None, "stderr_sha256": None}
    return {
        "exit_code": cp.returncode,
        "timed_out": False,
        "stdout_sha256": hashlib.sha256(cp.stdout).hexdigest(),
        "stderr_sha256": hashlib.sha256(cp.stderr).hexdigest(),
        "stderr_excerpt": cp.stderr[:STDERR_EXCERPT_BYTES].decode("utf-8", "replace") if cp.returncode != 0 else None,
    }
