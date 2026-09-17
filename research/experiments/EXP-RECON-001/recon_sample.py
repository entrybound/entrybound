#!/usr/bin/env python3
"""EXP-RECON-001 per-sample wrapper (stdlib only).

Packs one corpus item with the production planner (the v6 generation that
`ebound pack` always uses), then records, without interpreting anything:

* the pack outcome (ok / refused with reason code / crashed);
* artifact bytes, identities and required feature bits;
* declared decode requirements (window, working set, flags);
* reconstruction selections, audit/fallback reasons and access declarations
  (`ebound inspect --json --reconstruction`, unencrypted cells only);
* the replayed/derived savings lines of `ebound explain` (descriptive only);
* exact section byte accounting and counts (`ebr-inspect-bytes`);
* an exact round trip: every regular file's SHA-256 and every symlink target
  of the unpacked tree compared with the source item.

The LAST stdout line is one JSON object (read by the ebr runner's
`stdout_json` metrics). Child process output never reaches stdout. The wrapper
exits 0 whenever it measured an outcome (including product refusals and
crashes, which are recorded as data) and exits 2 only for its own usage or
environment errors, so the runner marks those samples failed.

Held-out guard: refuses any item or scratch path under the held-out roots
before reading it (defence in depth; the runner and every harness binary
guard as well).
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import stat
import subprocess
import sys
from pathlib import Path

SCHEMA = "ebr.recon.sample.v1"
HELDOUT_ROOTS = ("/root/eb-research/heldout", "/root/eb-research/fingerprints/heldout")
REASON_RE = re.compile(r"\b(EB_[A-Z0-9_]+)\b")
KV_RE = re.compile(r"([a-z][a-z0-9-]*)=(-?\d+)")


def die(msg: str) -> None:
    print(f"recon_sample: {msg}", file=sys.stderr)
    sys.exit(2)


def guard_not_heldout(path: str) -> None:
    extra = [os.environ.get("EB_HELDOUT_ROOT")] if os.environ.get("EB_HELDOUT_ROOT") else []
    real = os.path.realpath(path)
    lexical = os.path.normpath(os.path.abspath(path))
    for root in list(HELDOUT_ROOTS) + extra:
        root = os.path.normpath(root)
        for p in (real, lexical):
            if p == root or p.startswith(root + os.sep):
                die(f"refusing held-out path {path!r}")


def sha256_file(path: str) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as fh:
        for block in iter(lambda: fh.read(1 << 20), b""):
            h.update(block)
    return h.hexdigest()


def run(cmd, log_path: Path, timeout=None):
    """Run a child; stdout/stderr go to a log file, never to our stdout."""
    try:
        proc = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=timeout)
    except FileNotFoundError:
        die(f"missing binary {cmd[0]}")
    out = proc.stdout.decode("utf-8", "replace")
    err = proc.stderr.decode("utf-8", "replace")
    with open(log_path, "a", encoding="utf-8") as fh:
        fh.write(f"$ {' '.join(map(str, cmd))}\n[exit {proc.returncode}]\n--- stdout ---\n{out[-20000:]}\n--- stderr ---\n{err[-20000:]}\n")
    return proc.returncode, out, err


def last_json_line(text: str):
    for line in reversed(text.strip().splitlines()):
        line = line.strip()
        if line.startswith("{"):
            try:
                return json.loads(line)
            except ValueError:
                return None
    return None


def outcome(rc: int, err: str):
    crashed = rc < 0 or "panicked" in err or "RUST_BACKTRACE" in err
    m = REASON_RE.search(err)
    return crashed, (m.group(1) if m else None)


def tree_digest_map(root: bytes):
    """Map relative path -> ('f', sha256) | ('l', target hex) | ('o', mode)."""
    out = {}
    for dirpath, dirnames, filenames in os.walk(root, followlinks=False):
        for name in list(dirnames) + filenames:
            full = os.path.join(dirpath, name)
            rel = os.path.relpath(full, root)
            st = os.lstat(full)
            if stat.S_ISLNK(st.st_mode):
                out[rel] = ("l", os.readlink(full).hex())
            elif stat.S_ISREG(st.st_mode):
                h = hashlib.sha256()
                with open(full, "rb") as fh:
                    for block in iter(lambda: fh.read(1 << 20), b""):
                        h.update(block)
                out[rel] = ("f", h.hexdigest())
            elif stat.S_ISDIR(st.st_mode):
                continue
            else:
                out[rel] = ("o", stat.S_IFMT(st.st_mode))
    return out


def compare_trees(src: str, dst: str):
    s = tree_digest_map(os.fsencode(src))
    d = tree_digest_map(os.fsencode(dst))
    compared = mismatches = special = 0
    examples = []
    for rel, (kind, value) in sorted(s.items()):
        if kind == "o":
            special += 1
            continue
        compared += 1
        if d.get(rel) != (kind, value):
            mismatches += 1
            if len(examples) < 20:
                examples.append({"path_hex": rel.hex(), "expected": [kind, value], "found": d.get(rel)})
    extra = sorted(set(d) - set(s))
    return {
        "roundtrip_files_compared": compared,
        "roundtrip_mismatches": mismatches,
        "roundtrip_special_skipped": special,
        "roundtrip_extra_paths": len(extra),
        "mismatch_examples": examples,
    }


def parse_explain(text: str):
    res = {}
    for line in text.splitlines():
        if line.startswith("[NOT_RECORDED] replayed reconstructive comparison:"):
            kv = dict(KV_RE.findall(line))
            res["deflate_replayed_chunks"] = int(kv.get("chunks", 0))
            res["deflate_gross_savings_bytes"] = int(kv.get("gross-savings", 0))
            res["deflate_side_data_overhead_bytes"] = int(kv.get("reconstruction-data-overhead", 0))
            res["deflate_net_savings_bytes"] = int(kv.get("net-savings", 0))
        elif line.startswith("[AUDIT] reconstructive fallbacks:"):
            kv = dict(KV_RE.findall(line))
            res["deflate_fallback_chunks"] = int(kv.get("chunks", 0))
            res["deflate_fallback_unrecognized_or_failed"] = int(kv.get("unrecognized-or-verification-failed", 0))
            res["deflate_fallback_cost_rejected"] = int(kv.get("complete-cost-rejected", 0))
        elif line.startswith("[DERIVED] JPEG reconstruction:"):
            kv = dict(KV_RE.findall(line))
            res["jpeg_gross_savings_bytes"] = int(kv.get("gross-savings", 0))
            res["jpeg_representation_bytes"] = int(kv.get("representation", 0))
            res["jpeg_region_overhead_bytes"] = int(kv.get("region-overhead", 0))
            res["jpeg_net_savings_bytes"] = int(kv.get("net-savings", 0))
            for key in ("not-recognized", "unsupported", "exact-verification-failed",
                        "complete-cost-rejected", "dedup-conflict", "resource-policy-excluded"):
                res["jpeg_fallback_" + key.replace("-", "_")] = int(kv.get(key, 0))
    return res


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--item-path", required=True)
    ap.add_argument("--item-id", required=True)
    ap.add_argument("--profile", required=True, choices=["fast", "balanced", "dense", "extreme"])
    ap.add_argument("--layout", required=True, choices=["indexed", "stream"])
    ap.add_argument("--encrypt", required=True, choices=["true", "false"])
    ap.add_argument("--scratch", required=True)
    ap.add_argument("--bin-dir", required=True)
    ap.add_argument("--detail-dir", required=True)
    ap.add_argument("--rep", required=True)
    ap.add_argument("--experiment-id", default="EXP-RECON-001")
    ap.add_argument("--env-name", default="wsl-ubuntu")
    a = ap.parse_args()

    for p in (a.item_path, a.scratch, a.detail_dir):
        guard_not_heldout(p)
    if not os.path.isdir(a.item_path):
        die(f"item path is not a directory: {a.item_path}")
    encrypted = a.encrypt == "true"
    if encrypted and a.layout != "indexed":
        die("encrypted cells are INDEXED only (production has no encrypted STREAM)")
    bins = {n: os.path.join(a.bin_dir, n) for n in ("ebound", "ebr-pack", "ebr-unpack", "ebr-inspect-bytes")}
    for n, p in bins.items():
        if not os.access(p, os.X_OK):
            die(f"missing executable {p}")

    cell = f"{a.profile}-{a.layout}-{'enc' if encrypted else 'plain'}"
    detail_dir = Path(a.detail_dir) / a.item_id
    detail_dir.mkdir(parents=True, exist_ok=True)
    log = detail_dir / f"{cell}-rep{a.rep}.log"
    log.write_text("", encoding="utf-8")
    scratch = Path(a.scratch)
    scratch.mkdir(parents=True, exist_ok=True)
    arch = scratch / "a.eb"
    pw = Path(str(arch) + ".testpassword")
    result = {
        "schema": SCHEMA, "item_id": a.item_id, "cell": cell, "profile": a.profile, "layout": a.layout,
        "encrypted": encrypted, "rep": a.rep,
        "binary_sha256": {n: sha256_file(p) for n, p in bins.items()},
    }
    detail = {"cell": cell}

    # 1. pack
    if encrypted:
        rc, out, err = run([bins["ebr-pack"], "--item-path", a.item_path, "--experiment-id", a.experiment_id,
                            "--env-name", a.env_name, "--profile", a.profile, "--layout", "indexed",
                            "--encrypt", "true", "--archive-out", str(arch)], log)
    else:
        rc, out, err = run([bins["ebound"], "pack", a.item_path, str(arch), "--profile", a.profile,
                            "--layout", a.layout], log)
    crashed, reason = outcome(rc, err)
    result.update({"pack_exit": rc, "pack_crashed": int(crashed), "pack_refused": int(rc != 0 and not crashed),
                   "pack_reason_code": reason})
    if rc != 0 or not arch.is_file():
        detail["pack_stderr_tail"] = err[-4000:]
        finish(result, detail, detail_dir, cell, a.rep)
        return
    result["artifact_bytes"] = arch.stat().st_size
    result["artifact_sha256"] = None if encrypted else sha256_file(str(arch))

    # 2. exact byte accounting + counts (harness build; bytes proven identical to ebound, harness review R1-06)
    cmd = [bins["ebr-inspect-bytes"], "--archive-path", str(arch), "--experiment-id", a.experiment_id,
           "--env-name", a.env_name, "--layout", a.layout]
    if encrypted:
        cmd += ["--encrypted", "true", "--unlock-password-file", str(pw)]
    rc_b, out_b, err_b = run(cmd, log)
    row = last_json_line(out_b) or {}
    payload = row.get("payload", {}) if isinstance(row, dict) else {}
    ba = payload.get("byte_accounting", {}) or {}
    counts = payload.get("counts", {}) or {}
    recon_counts = counts.get("reconstruction", {}) or {}
    result.update({
        "inspect_bytes_exit": rc_b,
        "byte_accounting_granular": ba.get("granular"),
        "byte_accounting_balanced": ba.get("balanced"),
        "reconstruction_section_bytes": ba.get("reconstruction_bytes") if ba.get("granular") else None,
        "metadata_bytes": ba.get("metadata_bytes") if ba.get("granular") else None,
        "index_bytes": ba.get("index_bytes") if ba.get("granular") else None,
        "dictionary_bytes": ba.get("dictionary_bytes") if ba.get("granular") else None,
        "chunk_payload_bytes": ba.get("chunk_payload_bytes") if ba.get("granular") else None,
        "planner_id": counts.get("planner_id"),
        "chunker_id": counts.get("chunker_id"),
        "recon_transform_object_count": recon_counts.get("object_count"),
        "recon_transform_object_bytes": recon_counts.get("object_bytes"),
        "recon_transform_chunk_count": recon_counts.get("chunk_count"),
        "whole_object_region_count": counts.get("whole_object_region_count"),
        "unique_chunk_count": (counts.get("chunks") or {}).get("unique_chunk_count"),
    })
    detail["chunk_payload_bytes_by_transform"] = ba.get("chunk_payload_bytes_by_transform")
    detail["inspect_bytes_stderr_tail"] = err_b[-2000:]

    # 3. production inspection and explanation (unencrypted: CLI password entry is interactive only)
    if not encrypted:
        rc_i, out_i, err_i = run([bins["ebound"], "inspect", str(arch), "--json", "--reconstruction"], log)
        info = last_json_line(out_i) or {}
        feats = ((info.get("archive") or {}).get("features") or {})
        res = info.get("resources") or {}
        rec = info.get("reconstruction") or {}
        ids = info.get("identities") or {}
        audits = info.get("reconstruction_audits") or []
        by_reason = {}
        for au in audits:
            key = f"{au.get('transform')}|{au.get('reason')}"
            by_reason[key] = by_reason.get(key, 0) + 1
        incompat = feats.get("incompat")
        result.update({
            "inspect_exit": rc_i,
            "verification_status": ids.get("status"),
            "lai": ids.get("lai"), "aux": ids.get("aux"), "pcr": ids.get("pcr"), "pci": ids.get("pci"),
            "feature_incompat": incompat,
            "feature_0x4_deflate_reconstruction": None if incompat is None else int(bool(incompat & 0x4)),
            "feature_0x8_whole_object_reconstruction": None if incompat is None else int(bool(incompat & 0x8)),
            "declared_window_bytes": res.get("window_bytes"),
            "declared_working_set_bytes": res.get("working_set_bytes"),
            "declared_decode_flags": res.get("decode_flags"),
            "recon_data_objects": rec.get("data_objects"),
            "recon_data_bytes": rec.get("data_bytes"),
            "recon_regions": rec.get("regions"),
            "jpeg_regions": rec.get("jpeg_regions"),
            "worst_access_chunks": rec.get("worst_access_chunks"),
            "worst_access_bytes": rec.get("worst_access_bytes"),
            "audit_records": len(audits),
        })
        detail["audits_by_transform_reason"] = by_reason
        detail["regions"] = info.get("reconstruction_regions")
        rc_e, out_e, err_e = run([bins["ebound"], "explain", str(arch)], log)
        result["explain_exit"] = rc_e
        result.update(parse_explain(out_e))
        result["determinism_key"] = result["artifact_sha256"]
    else:
        result["determinism_key"] = None  # ciphertext differs by design (HC-13(b)); LAI checked in EXP-RECON-004

    # 4. exact round trip
    dest = scratch / "unpacked"
    if encrypted:
        rc_u, out_u, err_u = run([bins["ebr-unpack"], "--archive-path", str(arch), "--experiment-id", a.experiment_id,
                                  "--env-name", a.env_name, "--layout", "indexed", "--encrypted", "true",
                                  "--unlock-password-file", str(pw), "--scratch-dir", str(dest),
                                  "--keep-scratch", "true"], log)
    else:
        rc_u, out_u, err_u = run([bins["ebound"], "unpack", str(arch), str(dest), "--symlinks", "all"], log)
    crashed_u, reason_u = outcome(rc_u, err_u)
    result.update({"unpack_exit": rc_u, "unpack_crashed": int(crashed_u), "unpack_reason_code": reason_u})
    if rc_u == 0 and dest.is_dir():
        cmp_res = compare_trees(a.item_path, str(dest))
        detail["mismatch_examples"] = cmp_res.pop("mismatch_examples")
        result.update(cmp_res)
        result["roundtrip_evaluated"] = 1
    else:
        result["roundtrip_evaluated"] = 0
        detail["unpack_stderr_tail"] = err_u[-4000:]
    finish(result, detail, detail_dir, cell, a.rep)


def finish(result, detail, detail_dir: Path, cell: str, rep: str) -> None:
    detail["result"] = result
    blob = json.dumps(detail, sort_keys=True).encode("utf-8")
    path = detail_dir / f"{cell}-rep{rep}.detail.json"
    path.write_bytes(blob)
    result["detail_sha256"] = hashlib.sha256(blob).hexdigest()
    sys.stdout.write(json.dumps(result, sort_keys=True) + "\n")


if __name__ == "__main__":
    main()
