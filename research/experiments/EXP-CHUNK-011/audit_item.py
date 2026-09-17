#!/usr/bin/env python3
"""EXP-CHUNK-011 per-item chunker_id producer audit (production CLI + harness encrypted pack).

For one corpus item, one operation and one profile, produce an archive by that
operation and compare its recorded chunker identifier and identities with a
direct ``ebound pack --profile P`` of the same input (the reference producer):

  pack           direct pack (reference against itself: grammar check only)
  replan-same    pack P, then ``ebound repack --profile P``      (current-v6 replan)
  replan-cross   pack balanced (or fast when P is balanced), then ``repack --profile P``
  retain         pack P, then ``ebound repack`` without --profile (boundaries retained)
  convert-tar    deterministic pax tar of the item, then ``ebound convert --profile P``
  encrypted-pack ``ebr-pack --encrypt true --profile P`` (harness; production library code)

Prints ONE JSON line ``{"schema": "ebr.exp-chunk-011.audit.v1", "payload": {...}}``.
Operation refusals (non-zero exit of the audited operation) are recorded as
outcomes (``op_exit_code``, ``op_reason``), not as wrapper failures; the wrapper
itself exits non-zero only on its own errors. Standard library only.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import shutil
import subprocess
from pathlib import Path

SCHEMA = "ebr.exp-chunk-011.audit.v1"
# docs/chunking-v1.md and docs/crypto-suite-v1.md chunker_id grammars (frozen).
GRAMMAR = {
    "plain": re.compile(r"^gear-norm-v1/min-(\d+)/target-(\d+)/max-(\d+)$"),
    "secret-table": re.compile(r"^gear-norm-secret-table-v1/min-(\d+)/target-(\d+)/max-(\d+)$"),
}


PARAMS = re.compile(r"/min-(\d+)/target-(\d+)/max-(\d+)$")


def run(cmd: list[str]) -> subprocess.CompletedProcess:
    return subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False)


def inspect(ebound: str, archive: Path) -> dict:
    text = run([ebound, "inspect", str(archive)])
    js = run([ebound, "inspect", str(archive), "--json"])
    out = {"chunker_id": None, "planner_id": None, "lai": None, "aux": None, "pcr": None}
    if text.returncode == 0:
        for line in text.stdout.decode("utf-8", "replace").splitlines():
            if line.startswith("chunker: "):
                out["chunker_id"] = line[len("chunker: "):].strip()
            elif line.startswith("planner: "):
                out["planner_id"] = line[len("planner: "):].strip()
    if js.returncode == 0:
        ident = json.loads(js.stdout.decode("utf-8")).get("identities", {})
        out.update({k: ident.get(k) for k in ("lai", "aux", "pcr")})
    return out


def grammar_ok(chunker_id: str | None, kind: str) -> bool:
    if not chunker_id:
        return False
    m = GRAMMAR[kind].match(chunker_id)
    if not m:
        return False
    mn, tg, mx = (int(x) for x in m.groups())
    return 0 < mn <= tg <= mx and tg & (tg - 1) == 0


def first_line(b: bytes) -> str:
    s = b.decode("utf-8", "replace").strip().splitlines()
    return s[0][:300] if s else ""


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--item-path", required=True)
    ap.add_argument("--scratch", required=True)
    ap.add_argument("--op", required=True,
                    choices=["pack", "replan-same", "replan-cross", "retain", "convert-tar", "encrypted-pack"])
    ap.add_argument("--profile", required=True, choices=["fast", "balanced", "dense", "extreme"])
    ap.add_argument("--ebound", default="/root/eb-research/target/exp-chunk-prod/release/ebound")
    ap.add_argument("--ebr-pack", default="/root/eb-research/target/exp-chunk/release/ebr-pack")
    a = ap.parse_args()

    item = Path(a.item_path)
    s = Path(a.scratch) / "exp-chunk-011"
    if s.exists():
        shutil.rmtree(s)
    s.mkdir(parents=True)
    ref_path = s / "ref.eb"
    payload: dict = {"op": a.op, "profile": a.profile}

    ref = run([a.ebound, "pack", str(item), str(ref_path), "--profile", a.profile, "--layout", "indexed"])
    payload["reference_exit_code"] = ref.returncode
    if ref.returncode != 0:
        payload["reference_reason"] = first_line(ref.stderr)
        payload["applicable"] = False
    else:
        r = inspect(a.ebound, ref_path)
        payload.update({f"reference_{k}": v for k, v in r.items()})
        payload["reference_chunker_grammar_ok"] = grammar_ok(r["chunker_id"], "plain")
        op_path = s / "op.eb"
        op = None
        kind = "plain"
        if a.op == "pack":
            op = ref
            shutil.copyfile(ref_path, op_path)
        elif a.op in ("replan-same", "retain"):
            cmd = [a.ebound, "repack", str(ref_path), str(op_path)]
            if a.op == "replan-same":
                cmd += ["--profile", a.profile]
            op = run(cmd)
        elif a.op == "replan-cross":
            src_profile = "fast" if a.profile == "balanced" else "balanced"
            src = s / "src.eb"
            sp = run([a.ebound, "pack", str(item), str(src), "--profile", src_profile, "--layout", "indexed"])
            op = sp if sp.returncode != 0 else run([a.ebound, "repack", str(src), str(op_path), "--profile", a.profile])
            payload["replan_source_profile"] = src_profile
        elif a.op == "convert-tar":
            tar = s / "item.tar"
            src_dir = item if item.is_dir() else item.parent
            members = sorted(os.listdir(item)) if item.is_dir() else [item.name]
            t = run(["tar", "--sort=name", "--mtime=@0", "--owner=0", "--group=0", "--numeric-owner",
                     "--format=pax", "--pax-option=exthdr.name=%d/PaxHeaders/%f,delete=atime,delete=ctime",
                     "-C", str(src_dir), "-cf", str(tar), *members])
            op = t if t.returncode != 0 else run([a.ebound, "convert", str(tar), str(op_path), "--from", "tar",
                                                   "--profile", a.profile])
        elif a.op == "encrypted-pack":
            kind = "secret-table"
            op = run([a.ebr_pack, "--item-path", str(item), "--experiment-id", "EXP-CHUNK-011",
                      "--env-name", "wsl-ubuntu", "--profile", a.profile, "--layout", "indexed",
                      "--encrypt", "true", "--archive-out", str(op_path)])
        payload["op_exit_code"] = op.returncode
        payload["op_reason"] = first_line(op.stderr) if op.returncode != 0 else ""
        payload["applicable"] = op.returncode == 0
        if op.returncode == 0:
            if a.op == "encrypted-pack":
                lines = [ln for ln in op.stdout.decode("utf-8").splitlines() if ln.strip()]
                counts = json.loads(lines[-1])["payload"].get("counts", {})
                o = {"chunker_id": counts.get("chunker_id"), "planner_id": counts.get("planner_id"),
                     "lai": None, "aux": None, "pcr": None}
            else:
                o = inspect(a.ebound, op_path)
            payload.update({f"op_{k}": v for k, v in o.items()})
            payload["op_chunker_grammar_ok"] = grammar_ok(o["chunker_id"], kind)
            ref_params = PARAMS.search(r["chunker_id"] or "")
            op_params = PARAMS.search(o["chunker_id"] or "")
            payload["chunker_params_equal"] = bool(ref_params and op_params and ref_params.groups() == op_params.groups())
            payload["chunker_id_equal"] = (o["chunker_id"] == r["chunker_id"]) if kind == "plain" else None
            for k in ("lai", "aux", "pcr"):
                payload[f"{k}_equal"] = (o[k] == r[k]) if (o[k] is not None and r[k] is not None) else None

    for key in [k for k in payload if k.endswith(("_equal", "_ok")) or k == "applicable"]:
        v = payload[key]
        payload[f"{key}_n"] = None if v is None else int(bool(v))
    canonical = json.dumps(payload, sort_keys=True, separators=(",", ":")).encode()
    payload["result_sha256"] = hashlib.sha256(canonical).hexdigest()
    print(json.dumps({"schema": SCHEMA, "payload": payload}, sort_keys=True))
    shutil.rmtree(s, ignore_errors=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
