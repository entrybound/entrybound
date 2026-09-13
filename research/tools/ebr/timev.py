"""Parser for GNU ``/usr/bin/time -v`` output."""

from __future__ import annotations

import re
from typing import Any, Dict, Optional

_FIELDS = {
    "Command being timed": ("command", str),
    "User time (seconds)": ("user_s", float),
    "System time (seconds)": ("sys_s", float),
    "Percent of CPU this job got": ("cpu_percent", "pct"),
    "Elapsed (wall clock) time (h:mm:ss or m:ss)": ("elapsed_s", "clock"),
    "Average shared text size (kbytes)": ("avg_shared_text_kib", int),
    "Average unshared data size (kbytes)": ("avg_unshared_data_kib", int),
    "Average stack size (kbytes)": ("avg_stack_kib", int),
    "Average total size (kbytes)": ("avg_total_kib", int),
    "Maximum resident set size (kbytes)": ("max_rss_kib", int),
    "Average resident set size (kbytes)": ("avg_rss_kib", int),
    "Major (requiring I/O) page faults": ("major_faults", int),
    "Minor (reclaiming a frame) page faults": ("minor_faults", int),
    "Voluntary context switches": ("voluntary_ctx", int),
    "Involuntary context switches": ("involuntary_ctx", int),
    "Swaps": ("swaps", int),
    "File system inputs": ("fs_inputs", int),
    "File system outputs": ("fs_outputs", int),
    "Socket messages sent": ("socket_sent", int),
    "Socket messages received": ("socket_received", int),
    "Signals delivered": ("signals", int),
    "Page size (bytes)": ("page_size", int),
    "Exit status": ("exit_status", int),
}


def parse_clock(text: str) -> float:
    """Parse ``h:mm:ss`` or ``m:ss.ss`` into seconds."""
    parts = text.strip().split(":")
    if not 2 <= len(parts) <= 3 or not all(parts):
        raise ValueError(f"bad elapsed time {text!r}")
    secs = 0.0
    for p in parts:
        secs = secs * 60 + float(p)
    return secs


class TimeVParseError(ValueError):
    pass


def parse_time_v(text: str) -> Dict[str, Any]:
    """Parse the ``-v`` report. Raises TimeVParseError if mandatory fields are missing."""
    out: Dict[str, Any] = {}
    for raw_line in text.splitlines():
        line = raw_line.strip()
        if not line:
            continue
        m = re.match(r"^Command (?:exited with non-zero status|terminated by signal) (\d+)$", line)
        if m:
            if "signal" in line:
                out["terminated_by_signal"] = int(m.group(1))
            else:
                out["nonzero_status"] = int(m.group(1))
            continue
        # split on the LAST ": " (the command line itself may contain ": ")
        if line.startswith("Command being timed:"):
            val = line[len("Command being timed:"):].strip()
            if len(val) >= 2 and val[0] == '"' and val[-1] == '"':
                val = val[1:-1]
            out["command"] = val
            continue
        if ": " not in line:
            continue
        key, val = line.rsplit(": ", 1)
        spec = _FIELDS.get(key.strip())
        if spec is None:
            continue
        name, conv = spec
        val = val.strip()
        try:
            if conv == "pct":
                out[name] = None if val in ("?", "?%") else float(val.rstrip("%"))
            elif conv == "clock":
                out[name] = parse_clock(val)
            else:
                out[name] = conv(val)
        except ValueError as exc:
            raise TimeVParseError(f"cannot parse {key!r}: {val!r}") from exc
    for required in ("user_s", "sys_s", "elapsed_s", "max_rss_kib", "exit_status"):
        if required not in out:
            raise TimeVParseError(f"missing field {required} in /usr/bin/time -v output")
    return out


def exit_code_from(parsed: Dict[str, Any]) -> Optional[int]:
    if "terminated_by_signal" in parsed:
        return -int(parsed["terminated_by_signal"])
    return parsed.get("exit_status")
