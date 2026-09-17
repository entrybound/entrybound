#!/usr/bin/env python3
"""EXP-CROSSFILE-009 collector: host-independent fingerprint of an archive's cross-file physical plan.

Pre-registered with research/experiments/EXP-CROSSFILE-009/protocol.md. Stdlib only.

Runs the production CLI's own inspection (`ebound inspect <archive> --json --plans`
and `--chunks`) and prints ONE JSON line with SHA-256 digests over canonical JSON of:

  plans_sha256        every TransformPlan (identifier, codec, parameters, dictionary
                      reference, declared window/working set/flags), sorted by id
  dictionary_ids      sorted distinct Dictionary references (a Dictionary id is the
                      SHA-256 of its exact bytes, so equal ids mean equal bytes)
  assignment_sha256   every Chunk record (digest, physical ordinal, logical bytes,
                      plan identifier, group, stored bytes), sorted by physical ordinal
  crossfile_physical_sha256
                      digest over the three values above

These exclude the host-dependent identities and metadata that findings F-0001 and
F-0002 already explain (AUX/LAI/PCI and platform feature bits), so a cross-host
difference in this fingerprint is a difference in dictionaries, plans or Chunk
assignment, which is what DEC-CMP-021 asks about. PCR and PCI are also reported.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys


def canon_sha(obj) -> str:
    return hashlib.sha256(json.dumps(obj, sort_keys=True, separators=(",", ":")).encode()).hexdigest()


def inspect(ebound: str, archive: str, view: str) -> dict:
    out = subprocess.run([ebound, "inspect", archive, "--json", view], check=True, stdout=subprocess.PIPE).stdout
    return json.loads(out)


def main(argv=None) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--ebound", required=True)
    ap.add_argument("--archive", required=True)
    a = ap.parse_args(argv)

    plans_view = inspect(a.ebound, a.archive, "--plans")
    chunks_view = inspect(a.ebound, a.archive, "--chunks")
    plans = sorted(plans_view.get("plans", []), key=lambda p: p["id"])
    by_id = {p["id"]: p for p in plans}
    dictionaries = sorted({p["dictionary"] for p in plans if p.get("dictionary")})
    records = sorted(chunks_view.get("chunk_records", []), key=lambda r: r["physical_ordinal"])
    assignment = [
        {
            "digest": r["digest"],
            "physical_ordinal": r["physical_ordinal"],
            "logical_bytes": r["logical_bytes"],
            "plan": by_id.get(r["plan_id"], {}).get("identifier", f"unknown:{r['plan_id']}"),
            "plan_params": by_id.get(r["plan_id"], {}).get("codec_parameters_hex"),
            "dictionary": by_id.get(r["plan_id"], {}).get("dictionary"),
            "group": r.get("group"),
            "stored_bytes": r["stored_bytes"],
        }
        for r in records
    ]
    plans_sha = canon_sha(plans)
    assignment_sha = canon_sha(assignment)
    result = {
        "schema": "ebr.crossfile.physical-fingerprint.v1",
        "archive": a.archive,
        "plan_count": len(plans),
        "dictionary_count": len(dictionaries),
        "dictionary_ids": dictionaries,
        "dictionary_chunk_count": sum(1 for r in assignment if r["dictionary"]),
        "group_count": len({r["group"] for r in assignment if r["group"]}),
        "grouped_chunk_count": sum(1 for r in assignment if r["group"]),
        "chunk_count": len(assignment),
        "plans_sha256": plans_sha,
        "assignment_sha256": assignment_sha,
        "crossfile_physical_sha256": canon_sha([plans_sha, dictionaries, assignment_sha]),
        "pcr": chunks_view.get("identities", {}).get("pcr"),
        "pci": chunks_view.get("identities", {}).get("pci"),
        "working_set_bytes": chunks_view.get("resources", {}).get("working_set_bytes"),
    }
    print(json.dumps(result, sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main())
