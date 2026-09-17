#!/usr/bin/env python3
"""EXP-PLANNER-001 per-sample driver (planner tooling TP-00).

One ebr sample = one (item, profile, layout) pack with production code, then:

1. ``ebr-pack`` (harness build, parity-checked against ``ebound``) packs the item
   and reports exact section byte accounting, plan/codec counts and planner id;
2. ``ebound inspect --json`` (production CLI build) reports the descriptor
   feature bits and the LAI/AUX/PCR/PCI identities of the same archive file;
3. ``ebr-unpack`` extracts the archive into the sample scratch directory;
4. ``research/corpus/tools/fingerprint.py`` hashes the extraction, which is
   compared with the item's pinned ``content_tree_sha256`` from the corpus
   manifest (or with a fresh fingerprint of the input when the item is not in
   the manifest, e.g. a generated smoke tree).

It prints exactly one JSON line (schema ``ebr.planner.census.v1``) as the last
stdout line, which the ebr runner's ``stdout_json`` metrics read, and creates
``<scratch>/roundtrip.ok`` only when the extracted content matches. Exit status
is non-zero when packing, inspection or extraction fails, so the runner records
a failed sample instead of a silently partial one.

Optional ``--baseline-ebound`` packs the same item with an ``ebound`` built at
the research baseline 9e44608 (INDEXED layout only) and reports whether its
archive SHA-256 equals this sample's archive (status-quo identity arm, HC-16
MVT-16(d)).

Held-out safety: the harness binaries refuse held-out paths before reading, and
this script refuses any manifest row whose split is ``heldout``.
"""
from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import os
import subprocess
import sys
from pathlib import Path

SCHEMA = "ebr.planner.census.v1"
REPO = Path(__file__).resolve().parents[3]


def _fingerprint_module():
    src = REPO / "research" / "corpus" / "tools" / "fingerprint.py"
    spec = importlib.util.spec_from_file_location("planner_census_fingerprint", src)
    mod = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = mod
    spec.loader.exec_module(mod)
    return mod


def _run(argv, stdout_path: Path | None = None) -> str:
    with open(stdout_path, "wb") if stdout_path else open(os.devnull, "wb") as sink:
        proc = subprocess.run(argv, stdout=subprocess.PIPE if stdout_path is None else sink,
                              stderr=subprocess.PIPE)
    if proc.returncode != 0:
        tail = proc.stderr.decode("utf-8", "replace")[-2000:]
        raise RuntimeError(f"{Path(argv[0]).name} exited {proc.returncode}: {tail}")
    return proc.stdout.decode("utf-8", "replace") if stdout_path is None else ""


def _last_json_line(path: Path) -> dict:
    lines = [ln for ln in path.read_text(encoding="utf-8").splitlines() if ln.strip()]
    if not lines:
        raise RuntimeError(f"{path} is empty")
    return json.loads(lines[-1])


def _sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as fh:
        for block in iter(lambda: fh.read(1 << 20), b""):
            h.update(block)
    return h.hexdigest()


def _expected_content(manifest: str | None, item_id: str):
    if not manifest:
        return None
    data = json.loads(Path(manifest).read_text(encoding="utf-8"))
    rows = data["items"] if isinstance(data, dict) else data
    for row in rows:
        if row.get("item_id") == item_id:
            if row.get("split") == "heldout":
                raise RuntimeError("refusing a held-out manifest row")
            return row.get("content_tree_sha256"), row.get("logical_tree_sha256")
    return None


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--item-path", required=True)
    ap.add_argument("--item-id", required=True)
    ap.add_argument("--scratch", required=True)
    ap.add_argument("--profile", required=True, choices=["fast", "balanced", "dense", "extreme"])
    ap.add_argument("--layout", required=True, choices=["indexed", "stream"])
    ap.add_argument("--harness-bin", required=True, help="directory holding ebr-pack and ebr-unpack")
    ap.add_argument("--ebound", required=True, help="production ebound binary (no research-internals)")
    ap.add_argument("--baseline-ebound", help="ebound built at 9e44608 for the identity arm (indexed only)")
    ap.add_argument("--manifest", help="corpus manifest.json holding the item's pinned fingerprints")
    ap.add_argument("--experiment-id", default="EXP-PLANNER-001")
    ap.add_argument("--env-name", default="wsl-ubuntu")
    ap.add_argument("--fingerprint-jobs", type=int, default=4)
    args = ap.parse_args(argv)

    scratch = Path(args.scratch)
    scratch.mkdir(parents=True, exist_ok=True)
    archive = scratch / "archive.eb"
    pack_row = scratch / "pack.jsonl"
    inspect_out = scratch / "inspect.json"
    extracted = scratch / "extracted"
    ok_flag = scratch / "roundtrip.ok"
    if ok_flag.exists():
        ok_flag.unlink()
    hb = Path(args.harness_bin)

    pack_cmd = [str(hb / "ebr-pack"), "--item-path", args.item_path, "--experiment-id", args.experiment_id,
                "--env-name", args.env_name, "--profile", args.profile, "--layout", args.layout,
                "--archive-out", str(archive), "--out", str(pack_row)]
    if args.layout == "stream":
        pack_cmd += ["--stream-window", "auto"]
    _run(pack_cmd)
    pack = _last_json_line(pack_row)["payload"]
    ba = pack["byte_accounting"]
    counts = pack["counts"]

    _run([args.ebound, "inspect", "--json", str(archive)], stdout_path=inspect_out)
    inspection = _last_json_line(inspect_out)
    features = (inspection.get("archive") or {}).get("features") or {}
    identities = inspection.get("identities") or {}

    _run([str(hb / "ebr-unpack"), "--archive-path", str(archive), "--experiment-id", args.experiment_id,
          "--env-name", args.env_name, "--layout", args.layout, "--scratch-dir", str(extracted),
          "--keep-scratch", "true", "--out", str(scratch / "unpack.jsonl")])

    fp = _fingerprint_module()
    got = fp.fingerprint_path(str(extracted), jobs=args.fingerprint_jobs)
    expected = _expected_content(args.manifest, args.item_id)
    if expected is None or expected[0] is None:
        src = fp.fingerprint_path(args.item_path, jobs=args.fingerprint_jobs)
        expected = (src["content_tree_sha256"], src["logical_tree_sha256"])
    content_match = got["content_tree_sha256"] == expected[0]
    logical_match = got["logical_tree_sha256"] == expected[1]
    if content_match:
        ok_flag.write_text("ok\n", encoding="ascii")

    baseline_identical = None
    if args.baseline_ebound and args.layout == "indexed":
        base_archive = scratch / "baseline.eb"
        _run([args.baseline_ebound, "pack", args.item_path, str(base_archive), "--layout", "indexed",
              "--profile", args.profile])
        baseline_identical = _sha256_file(base_archive) == _sha256_file(archive)
        base_archive.unlink()

    artifact_bytes = pack["artifact_bytes"]
    chunks = counts["chunks"]
    logical = counts["total_logical_bytes"]
    row = {
        "schema": SCHEMA,
        "item_id": args.item_id,
        "profile": args.profile,
        "layout": args.layout,
        "artifact_bytes": artifact_bytes,
        "artifact_sha256": _sha256_file(archive),
        "byte_accounting_granular": ba["granular"],
        "chunk_payload_bytes": ba["chunk_payload_bytes"],
        "metadata_bytes": artifact_bytes - ba["chunk_payload_bytes"] if ba["granular"] else None,
        "metadata_bytes_per_entry": ((artifact_bytes - ba["chunk_payload_bytes"]) / counts["entry_count"]
                                     if ba["granular"] and counts["entry_count"] else None),
        "index_bytes": ba["index_bytes"] if ba["granular"] else None,
        "dictionary_bytes": ba["dictionary_bytes"] if ba["granular"] else None,
        "side_data_bytes": ba["chunk_group_bytes"] + ba["reconstruction_bytes"] if ba["granular"] else None,
        "transform_plan_bytes": ba["transform_plan_bytes"] if ba["granular"] else None,
        "chunk_payload_bytes_by_codec": ba.get("chunk_payload_bytes_by_codec"),
        "chunk_payload_bytes_by_transform": ba.get("chunk_payload_bytes_by_transform"),
        "planner_id": counts["planner_id"],
        "chunker_id": counts["chunker_id"],
        "entry_count": counts["entry_count"],
        "total_logical_bytes": logical,
        "unique_chunk_count": chunks["unique_chunk_count"],
        "logical_chunk_references": chunks["logical_chunk_references"],
        "unique_plaintext_bytes": chunks["unique_plaintext_bytes"],
        "dedup_stored_fraction": chunks["unique_plaintext_bytes"] / logical if logical else None,
        "codec_usage": {u["codec"]: {"chunks": u["chunk_count"], "logical_bytes": u["logical_bytes"],
                                      "stored_bytes": u["stored_bytes"]} for u in counts["codec_usage"]},
        "plan_count": len(counts["plans"]),
        "dictionary_count": counts["cross_file"]["dictionary_count"],
        "chunk_group_count": counts["cross_file"]["chunk_group_count"],
        "reconstruction_object_count": counts["reconstruction"]["object_count"],
        "whole_object_region_count": counts["whole_object_region_count"],
        "features": {"incompat": features.get("incompat"), "read_only_compat": features.get("read_only_compat"),
                     "compat": features.get("compat")},
        "identities": {k: identities.get(k) for k in ("lai", "aux", "pcr", "pci")},
        "verified": pack["verified"],
        "roundtrip_content_match": content_match,
        "roundtrip_logical_match": logical_match,
        "roundtrip_mismatches": 0 if content_match else 1,
        "baseline_9e44608_identical": baseline_identical,
        "build_note": pack.get("build_note"),
    }
    print(json.dumps(row, sort_keys=True))
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as exc:  # noqa: BLE001 - one failed sample must not look like a partial success
        print(json.dumps({"schema": SCHEMA, "error": str(exc)[:1000]}))
        sys.exit(1)
