"""Emit one line per workflow agent completion, for orchestrator checkpoint commits.

Polls workflow journal.jsonl files, joins `started` (agentId -> label) with
`result` entries, and prints `DONE <run> <label>` for results that appear after
the watcher starts. Unknown entry types are printed as `EVENT <run> <type>`.

Usage: python watch_journals.py <journal_dir> [<journal_dir> ...]
"""

import json
import os
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


def main(dirs):
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
                    print(f"DONE {run} {label} {ok}", flush=True)
                elif kind not in ("started", "launched"):
                    print(f"EVENT {run} {kind}", flush=True)
            seen[directory] = len(entries)
        time.sleep(POLL_SECONDS)


if __name__ == "__main__":
    main(sys.argv[1:])
