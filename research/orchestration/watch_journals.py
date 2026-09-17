"""Emit one line per workflow agent completion, for orchestrator checkpoint commits.

Polls workflow journal.jsonl files, joins `started` (agentId -> label) with
`result` entries, and prints `DONE <run> <label>` for results that appear after
the watcher starts. Unknown entry types are printed as `EVENT <run> <type>`.

Usage: python watch_journals.py <journal_dir> [<journal_dir> ...]
"""

import json
import os
import shutil
import sys
import time

POLL_SECONDS = 20


def read_entries(path):
    try:
        with open(path, encoding="utf-8") as handle:
            lines = handle.readlines()
    except OSError:
        return []
    entries = []
    for line in lines:
        line = line.strip()
        if not line:
            continue
        try:
            entries.append(json.loads(line))
        except json.JSONDecodeError:
            continue  # partially written trailing line
    return entries


DISK_LIMITS_GB = {"C:\\": 50.0, "D:\\": 100.0}
# EB_WATCH_QUIET=1 reports only failures, empty results, and disk alarms; workflow
# completion already arrives as its own notification, so per-agent DONE lines are noise
# that costs orchestrator turns.
QUIET = os.environ.get("EB_WATCH_QUIET") == "1"


def check_disks(warned):
    """Print one DISK LOW line per drive when free space drops below its limit."""
    for drive, limit in DISK_LIMITS_GB.items():
        try:
            free_gb = shutil.disk_usage(drive).free / 1024**3
        except OSError:
            continue
        if free_gb < limit and drive not in warned:
            print(f"DISK LOW {drive} free={free_gb:.1f}GB below {limit}GB", flush=True)
            warned.add(drive)
        elif free_gb >= limit * 1.2:
            warned.discard(drive)


def check_markers(seen_markers):
    """Report completion-marker files (EB_WATCH_MARKERS, ';'-separated paths) once when they appear."""
    for path in filter(None, os.environ.get("EB_WATCH_MARKERS", "").split(";")):
        if path not in seen_markers and os.path.exists(path):
            print(f"MARKER {path}", flush=True)
            seen_markers.add(path)


def main(dirs):
    warned = set()
    seen_markers = set()
    seen = {}
    for directory in dirs:
        seen[directory] = len(read_entries(os.path.join(directory, "journal.jsonl")))
    labels = {}
    while True:
        for directory in dirs:
            run = os.path.basename(directory.rstrip("/\\"))
            entries = read_entries(os.path.join(directory, "journal.jsonl"))
            for entry in entries:
                if entry.get("type") == "started":
                    labels[entry.get("agentId")] = entry.get("label", "?")
            for entry in entries[seen[directory]:]:
                kind = entry.get("type")
                if kind == "result":
                    label = labels.get(entry.get("agentId"), entry.get("agentId"))
                    ok = "null" if entry.get("result") is None else "ok"
                    if not QUIET or ok == "null":
                        print(f"DONE {run} {label} {ok}", flush=True)
                elif kind not in ("started", "launched"):
                    print(f"EVENT {run} {kind}", flush=True)
            seen[directory] = len(entries)
        check_disks(warned)
        check_markers(seen_markers)
        time.sleep(POLL_SECONDS)


if __name__ == "__main__":
    main(sys.argv[1:])
