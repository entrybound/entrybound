#!/usr/bin/env python3
"""EXP-CROSSFILE-013 collector: one (item, source physical configuration, export target) case.

Pre-registered with research/experiments/EXP-CROSSFILE-013/protocol.md. Stdlib only.

Packs the item with the production CLI under one source configuration, exports the
native archive to one legacy target with explicit lossy approval, and prints ONE JSON
line. The analysis groups rows by (item, target) and requires `export_sha256` and
`outcome` to be identical across every source configuration (objective MVT-13(d);
DEC-LEG-016), and reports source cross-file structure (dictionaries, groups) so a
case where the source actually used ChunkGroups or Dictionaries is identifiable.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys


def sha256_file(path: str) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as fh:
        for block in iter(lambda: fh.read(1 << 20), b""):
            h.update(block)
    return h.hexdigest()


def main(argv=None) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--ebound", required=True)
    ap.add_argument("--item-path", required=True)
    ap.add_argument("--scratch", required=True)
    ap.add_argument("--profile", required=True, choices=["fast", "balanced", "dense", "extreme"])
    ap.add_argument("--layout", default="indexed", choices=["indexed", "stream"])
    ap.add_argument("--stream-window", default="auto")
    ap.add_argument("--target", required=True, choices=["zip", "tar", "tar.gz", "tar.zst", "tar.xz", "tar.bz2"])
    a = ap.parse_args(argv)

    os.makedirs(a.scratch, exist_ok=True)
    native = os.path.join(a.scratch, "source.eb")
    export = os.path.join(a.scratch, "export." + a.target)
    receipt = os.path.join(a.scratch, "receipt.bin")
    pack_cmd = [a.ebound, "pack", a.item_path, native, "--layout", a.layout, "--profile", a.profile]
    if a.layout == "stream":
        pack_cmd += ["--stream-window", a.stream_window]
    result = {"schema": "ebr.crossfile.export-case.v1", "profile": a.profile, "layout": a.layout, "target": a.target}
    pack = subprocess.run(pack_cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    result["pack_exit"] = pack.returncode
    if pack.returncode != 0:
        result["pack_reason"] = (pack.stderr.decode("utf-8", "replace").strip().splitlines() or [""])[0][:300]
        result["outcome"] = "SOURCE_REFUSED"
        print(json.dumps(result, sort_keys=True))
        return 0
    text = pack.stdout.decode("utf-8", "replace")
    for key in ("PCR", "planner"):
        m = re.search(rf"^{key} (\S+)$", text, re.M)
        result[key.lower()] = m.group(1) if m else None
    chunks = subprocess.run([a.ebound, "inspect", native, "--json", "--chunks"], stdout=subprocess.PIPE)
    plans = subprocess.run([a.ebound, "inspect", native, "--json", "--plans"], stdout=subprocess.PIPE)
    if chunks.returncode == 0 and plans.returncode == 0 and a.layout == "indexed":
        records = json.loads(chunks.stdout).get("chunk_records", [])
        plan_list = json.loads(plans.stdout).get("plans", [])
        dict_plans = {p["id"] for p in plan_list if p.get("dictionary")}
        result["source_group_count"] = len({r["group"] for r in records if r.get("group")})
        result["source_grouped_chunks"] = sum(1 for r in records if r.get("group"))
        result["source_dictionary_chunks"] = sum(1 for r in records if r["plan_id"] in dict_plans)
    result["source_bytes"] = os.path.getsize(native)
    conv = subprocess.run(
        [a.ebound, "convert", native, export, "--to", a.target, "--allow-lossy", "--receipt", receipt],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    result["export_exit"] = conv.returncode
    out = conv.stdout.decode("utf-8", "replace")
    m = re.search(r"^outcome: (\S+)$", out, re.M)
    result["outcome"] = m.group(1) if m else ("REFUSED" if conv.returncode != 0 else None)
    if conv.returncode != 0:
        result["export_reason"] = (conv.stderr.decode("utf-8", "replace").strip().splitlines() or [""])[0][:300]
    result["issue_lines"] = sum(1 for line in out.splitlines() if line.startswith("issue "))
    result["issues_sha256"] = hashlib.sha256(
        "\n".join(sorted(line for line in out.splitlines() if line.startswith("issue "))).encode()
    ).hexdigest()
    if os.path.exists(export):
        result["export_bytes"] = os.path.getsize(export)
        result["export_sha256"] = sha256_file(export)
    if os.path.exists(receipt):
        result["receipt_bytes"] = os.path.getsize(receipt)
        result["receipt_sha256"] = sha256_file(receipt)
    print(json.dumps(result, sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main())
