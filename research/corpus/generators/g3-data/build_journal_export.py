#!/usr/bin/env python3
"""Generate a native binary systemd-journald journal file (F05, generated).

Produces a deterministic stream of synthetic log entries in the systemd "journal export"
text format (the same format `journalctl -o export` emits), then feeds that stream through the
real `systemd-journal-remote` binary (Ubuntu package `systemd-journal-remote`) to obtain a
genuine binary .journal file (the journald on-disk format: JSON-like framed records, an mmap'd
hash-table index, optional per-field compression) rather than a hand-rolled imitation.

Not byte-reproducible: systemd-journal-remote stamps every output file with a random 128-bit
file ID (`sd_id128_randomize`) and its own internal sequence/hash-table sizing, so two runs
with byte-identical input produce different journal bytes.  The item declares
recipe.output_pin: null for this reason (see build_postgres_pgbench.sh / build_mariadb_test_db.sh
for the same pattern with Docker-built database directories).  The *logical* entry content
(messages, priorities, units, timestamps, boot/machine IDs) is a pure function of --seed/--params.

Extent-map settling (2026-09-17, task_bb8ca4e8 integrity follow-up): on this ext4/WSL2 host,
system.journal's SEEK_DATA/SEEK_HOLE extent map is not immediately settled when
systemd-journal-remote exits -- the same allocation-timing defect class already worked around
in the g4-binary VM-image generators (see PROGRESS.md 2026-09-16T21:30Z), one layer deeper than
the documented tree_sha256/st_blocks issue. `fallocate --dig-holes` + fsync/sync below are best-
effort hardening (content_tree_sha256, which excludes the extent map, is unaffected either way),
but empirically the map can still shift for several seconds after this script returns and
provision.py takes its first fingerprint. If `provision.py --verify` reports a HASH MISMATCH for
this item with an unchanged content_tree_sha256, re-fingerprint the materialized directory a few
times a few seconds apart (research/corpus/tools/fingerprint.py) to find the settled value, then
correct the recorded fingerprint to that value -- do not assume corruption or re-run --rebuild
(which would also stamp a new random file ID, changing real content for no reason).

ebrc script contract:
    python build_journal_export.py --out <staging dir> --seed <int> --params <canonical JSON>
params:
  boots       number of synthetic "boot sessions" (default 3)
  entries_per_boot   log lines per boot (default 4000)
  base_epoch  synthetic __REALTIME_TIMESTAMP base, seconds (default 1767225600 = 2026-01-01Z)
  file_name   output journal file name (default "system.journal")

Requires /usr/lib/systemd/systemd-journal-remote (apt package systemd-journal-remote).
"""

import argparse
import json
import os
import random
import shutil
import subprocess
import sys
import uuid
from pathlib import Path

UNITS = [
    ("sshd.service", "sshd"), ("cron.service", "cron"), ("nginx.service", "nginx"),
    ("systemd-networkd.service", "systemd-networkd"), ("docker.service", "dockerd"),
    ("app-checkout.service", "checkout"), ("app-payments.service", "payments"),
    ("postgresql.service", "postgres"), ("systemd-logind.service", "systemd-logind"),
    ("kernel", "kernel"),
]
MESSAGES = {
    "sshd": ["Accepted publickey for deploy from 10.{0}.{1}.{2} port {3} ssh2",
             "Disconnected from user deploy 10.{0}.{1}.{2} port {3}",
             "pam_unix(sshd:session): session opened for user deploy(uid=1000) by (uid=0)",
             "pam_unix(sshd:session): session closed for user deploy"],
    "cron": ["(root) CMD (   cd / && run-parts --report /etc/cron.hourly)",
             "pam_unix(cron:session): session opened for user root(uid=0) by (uid=0)"],
    "nginx": ["{0}.{1}.{2}.{3} - - [request] \"GET /healthz HTTP/1.1\" 200 2",
               "worker process {0} exited on signal 15", "start worker processes"],
    "systemd-networkd": ["eth0: Gained carrier", "eth0: Lost carrier", "eth0: DHCPv4 address 10.{0}.{1}.{2}/24"],
    "dockerd": ["API listen on /run/docker.sock", "Container {0:08x} started", "Container {0:08x} stopped"],
    "checkout": ["order {0} placed total_cents={1}", "order {0} payment_pending", "GET /cart 200 {1}ms"],
    "payments": ["charge {0} succeeded amount_cents={1}", "charge {0} declined reason=insufficient_funds"],
    "postgres": ["checkpoint starting: time", "checkpoint complete: wrote {0} buffers",
                 "duration: {0}.{1} ms  statement: SELECT 1"],
    "systemd-logind": ["New session {0} of user deploy.", "Session {0} logged out."],
    "kernel": ["TCP: request_sock_TCP: Possible SYN flooding on port {0}. Sending cookies.",
               "eth0: renamed from veth{0:04x}", "audit: type=1400 audit({0}.{1}:{2})"],
}


def gen_stream(rng: random.Random, boots: int, entries_per_boot: int, base_epoch: int):
    machine_id = uuid.UUID(int=rng.getrandbits(128), version=4).hex
    t = base_epoch
    for b in range(boots):
        boot_id = uuid.UUID(int=rng.getrandbits(128), version=4).hex
        t += rng.randint(60, 3600)
        for i in range(entries_per_boot):
            unit, comm = UNITS[rng.randrange(len(UNITS))]
            templates = MESSAGES[comm]
            tmpl = templates[rng.randrange(len(templates))]
            args = [rng.randrange(256) for _ in range(4)]
            msg = tmpl.format(*args)
            pid = rng.randrange(300, 65000)
            prio = rng.choices([3, 4, 6, 6, 6, 7], weights=[1, 3, 40, 40, 40, 16])[0]
            t += rng.randint(0, 4) + rng.random()
            realtime_us = int(t * 1_000_000)
            monotonic_us = int((t - base_epoch) * 1_000_000) & 0xFFFFFFFFFFFFFFFF
            fields = {
                "__REALTIME_TIMESTAMP": str(realtime_us),
                "__MONOTONIC_TIMESTAMP": str(monotonic_us),
                "_BOOT_ID": boot_id,
                "_MACHINE_ID": machine_id,
                "_HOSTNAME": "ebrc-gen-host",
                "_TRANSPORT": "syslog" if comm != "kernel" else "kernel",
                "PRIORITY": str(prio),
                "SYSLOG_FACILITY": "3",
                "SYSLOG_IDENTIFIER": comm,
                "_COMM": comm,
                "_PID": str(pid),
                "MESSAGE": msg,
            }
            if unit != "kernel":
                fields["UNIT"] = unit
                fields["_SYSTEMD_UNIT"] = unit
            for k, v in fields.items():
                yield f"{k}={v}\n"
            yield "\n"


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True)
    ap.add_argument("--seed", type=int, default=0)
    ap.add_argument("--params", default="{}")
    args = ap.parse_args()
    params = json.loads(args.params)
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)

    boots = int(params.get("boots", 3))
    entries_per_boot = int(params.get("entries_per_boot", 4000))
    base_epoch = int(params.get("base_epoch", 1767225600))
    file_name = params.get("file_name", "system.journal")

    exe = "/usr/lib/systemd/systemd-journal-remote"
    if not Path(exe).is_file():
        alt = shutil.which("systemd-journal-remote")
        if alt:
            exe = alt
        else:
            raise SystemExit("systemd-journal-remote not found; apt-get install systemd-journal-remote")

    scratch = Path(os.environ.get("EB_SCRATCH", "/tmp")) / "journal-export"
    scratch.mkdir(parents=True, exist_ok=True)
    export_path = scratch / "entries.export"
    rng = random.Random(args.seed)
    n = 0
    with open(export_path, "w", encoding="utf-8") as f:
        for line in gen_stream(rng, boots, entries_per_boot, base_epoch):
            f.write(line)
            if line == "\n":
                n += 1

    journal_path = out / file_name
    if journal_path.exists():
        journal_path.unlink()
    r = subprocess.run([exe, "--output", str(journal_path), "--split-mode=none", str(export_path)],
                       stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    log = r.stdout.decode("utf-8", "replace")
    print(log, file=sys.stderr)
    if r.returncode != 0 or not journal_path.is_file():
        raise SystemExit(f"systemd-journal-remote failed (exit {r.returncode})")
    if "Finishing after writing" not in log:
        raise SystemExit("systemd-journal-remote did not report entries written")
    written = int(log.rsplit("Finishing after writing", 1)[1].split()[0])
    if written != n:
        raise SystemExit(f"wrote {written} entries, expected {n}")
    os.chmod(journal_path, 0o644)

    (out / "generator_summary.json").write_text(json.dumps({
        "entries": n, "boots": boots, "entries_per_boot": entries_per_boot, "base_epoch": base_epoch,
        "note": "logical content is a pure function of seed/params; the .journal file's internal "
                "file ID and hash-table sizing are randomized per run by systemd-journal-remote "
                "(recipe.output_pin is null for this item).",
    }, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    shutil.rmtree(scratch, ignore_errors=True)

    # Settle the extent map before the corpus framework fingerprints the staging directory.
    # systemd-journal-remote writes system.journal in a way that leaves ext4/WSL2 delayed
    # allocation not yet flushed for a window after the process exits, so a SEEK_DATA/SEEK_HOLE
    # scan taken immediately can see a different (still content-identical) extent layout than one
    # taken moments later -- the same class of allocation-timing defect already worked around for
    # the g4-binary VM-image generators (fallocate --dig-holes + trailing sync). Force every
    # regular file's data to be durably allocated on disk, then fsync the directory entries, so
    # fingerprint.py's logical_tree_sha256 (which includes the extent map) is stable the first
    # time it is computed.
    for f in (journal_path, out / "generator_summary.json"):
        subprocess.run(["fallocate", "--dig-holes", str(f)], check=True)
        fd = os.open(f, os.O_RDONLY)
        try:
            os.fsync(fd)
        finally:
            os.close(fd)
    dfd = os.open(out, os.O_RDONLY)
    try:
        os.fsync(dfd)
    finally:
        os.close(dfd)
    subprocess.run(["sync"], check=True)
    return 0


if __name__ == "__main__":
    sys.exit(main())
