#!/usr/bin/env python3
"""EXP-INT-009 driver: identity reproduction and binding staleness under rewrite operations.

One invocation = one ebr sample for one (item, profile, repetition). It packs the item with
the production `ebound` CLI, applies the pre-registered rewrite operations of
research/experiments/EXP-INT-009/protocol.md section 6, records LAI/AUX/PCR/PCI after each
operation (`ebound inspect --json`), verifies a detached signature against every output,
checks a par2 recovery set bound by file hash, optionally runs the rollback arm on a version
series, writes every observation to a gzip JSON detail file, and prints exactly one JSON line
(schema ebr.int.driver.v1) whose `payload` the ebr spec reads with `stdout_json` metrics.

Standard library only. Works on Linux (WSL) and Windows (pass ebound.exe). It never reads a
held-out path: it refuses any --item-path containing a path component named `heldout`.

Exit status: 0 when every command it needed ran (including a recorded pack refusal of the
item, reported as payload.refused = true); 2 on usage errors or a missing tool.
"""
from __future__ import annotations

import argparse
import gzip
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import time
from pathlib import Path

SCHEMA = "ebr.int.driver.v1"
EXPERIMENT_ID = "EXP-INT-009"
IDS = ("lai", "aux", "pcr", "pci")

# Pre-registered expectations per operation (protocol section 6). "same" means the identity
# must equal the one of the reference archive p0; "report" means it is recorded but not scored.
EXPECTED = {
    "pack-again": {"lai": "same", "aux": "same", "pcr": "same", "pci": "same"},
    "repack-preserve": {"lai": "same", "aux": "same", "pcr": "same", "pci": "report"},
    "repack-index-absent": {"lai": "same", "aux": "same", "pcr": "same", "pci": "changed"},
    "repack-index-present-from-absent": {"lai": "same", "aux": "same", "pcr": "same", "pci": "report"},
    "repack-layout-stream": {"lai": "same", "aux": "same", "pcr": "same", "pci": "changed"},
    "repack-layout-indexed-from-stream": {"lai": "same", "aux": "same", "pcr": "same", "pci": "report"},
    "repack-profile-other": {"lai": "same", "aux": "same", "pcr": "report", "pci": "report"},
    "replan-from-extract": {"lai": "same", "aux": "report", "pcr": "same", "pci": "report"},
}
OTHER_PROFILE = {"fast": "extreme", "balanced": "extreme", "dense": "fast", "extreme": "fast"}
SIG_RE = re.compile(r"^signature\s+(.*)$", re.M)
TRUNC = 4096


def refuse_heldout(path: str) -> None:
    parts = {p.lower() for p in Path(path).parts}
    if "heldout" in parts:
        print(f"refusing to read held-out path: {path}", file=sys.stderr)
        sys.exit(2)


def run(cmd, cwd=None, timeout=7200):
    t0 = time.monotonic()
    try:
        p = subprocess.run([str(c) for c in cmd], cwd=cwd, capture_output=True, text=True,
                           encoding="utf-8", errors="replace", timeout=timeout)
        rc, out, err = p.returncode, p.stdout, p.stderr
    except subprocess.TimeoutExpired as exc:
        rc, out, err = -9, exc.stdout or "", f"timeout after {timeout}s"
    return {
        "argv": [str(c) for c in cmd],
        "exit": rc,
        "stdout": (out or "")[:TRUNC],
        "stderr": (err or "")[:TRUNC],
        "wall_s": round(time.monotonic() - t0, 6),
    }


def identities(ebound, archive, log):
    t0 = time.monotonic()
    try:
        cp = subprocess.run([str(ebound), "inspect", str(archive), "--json"], capture_output=True,
                            text=True, encoding="utf-8", errors="replace", timeout=7200)
        rc, out, err = cp.returncode, cp.stdout, cp.stderr
    except subprocess.TimeoutExpired:
        rc, out, err = -9, "", "timeout"
    log.append({"step": "inspect", "archive": Path(archive).name, "exit": rc, "stderr": (err or "")[:TRUNC],
                "wall_s": round(time.monotonic() - t0, 6)})
    if rc != 0:
        return None
    try:
        ids = json.loads(out)["identities"]
    except (ValueError, KeyError):
        return None
    return {k: ids.get(k) for k in IDS} | {"status": ids.get("status")}


def parse_signature(text):
    m = SIG_RE.search(text or "")
    if not m:
        return None
    return dict(tok.split("=", 1) for tok in m.group(1).split() if "=" in tok)


def pattern(ref, other):
    if ref is None or other is None:
        return None
    return {k: ("same" if ref[k] == other[k] else "changed") for k in IDS}


def conformance(op, pat):
    exp = EXPECTED.get(op)
    if exp is None or pat is None:
        return 0, 0
    scored = matched = 0
    for k, want in exp.items():
        if want == "report":
            continue
        scored += 1
        matched += int(pat[k] == want)
    return scored, matched


def version_dirs(item_path: Path):
    dirs = sorted([d for d in item_path.iterdir() if d.is_dir()], key=lambda d: os.fsencode(d.name))
    files = [f for f in item_path.iterdir() if not f.is_dir()]
    return dirs if len(dirs) >= 2 and not files else None


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--ebound", required=True)
    ap.add_argument("--par2", default=shutil.which("par2") or "")
    ap.add_argument("--item-path", required=True)
    ap.add_argument("--item-id", required=True)
    ap.add_argument("--profile", required=True, choices=sorted(OTHER_PROFILE))
    ap.add_argument("--work-dir", required=True)
    ap.add_argument("--detail-root", required=True)
    ap.add_argument("--candidate", required=True)
    ap.add_argument("--env-name", required=True)
    ap.add_argument("--rep", required=True)
    ap.add_argument("--series", choices=("auto", "off"), default="auto")
    args = ap.parse_args(argv)

    refuse_heldout(args.item_path)
    ebound = Path(args.ebound)
    if not ebound.is_file():
        print(f"missing ebound: {ebound}", file=sys.stderr)
        return 2
    item = Path(args.item_path)
    work = Path(args.work_dir) / "int009"
    if work.exists():
        shutil.rmtree(work)
    work.mkdir(parents=True)
    log: list = []
    detail = {"schema": "ebr.int.detail.v1", "experiment_id": EXPERIMENT_ID, "item_id": args.item_id,
              "candidate": args.candidate, "env_name": args.env_name, "rep": args.rep,
              "ebound_sha256": hashlib.sha256(ebound.read_bytes()).hexdigest(), "ops": {}, "log": log}
    payload = {"refused": False}

    p0 = work / "p0.eb"
    r = run([ebound, "pack", item, p0, "--layout", "indexed", "--profile", args.profile])
    log.append({"step": "pack-p0", **r})
    if r["exit"] != 0:
        payload.update({"refused": True, "refusal": (r["stderr"] or r["stdout"]).strip().splitlines()[:1]})
        return emit(args, detail, payload)
    ref = identities(ebound, p0, log)
    detail["reference"] = ref

    # Detached signature over p0 with the physical binding (addressing is encrypted-only).
    sk, sig = work / "signing.key", work / "p0.ebsig"
    log.append({"step": "keygen", **run([ebound, "key", "generate-signing", sk])})
    rs = run([ebound, "sign", p0, "--signing-key", sk, "--detached", sig, "--bind-physical"])
    log.append({"step": "sign-p0", **rs})
    have_sig = rs["exit"] == 0 and sig.is_file()

    # par2 recovery set bound by file name and hash (candidate par-native-file-hash).
    par_dir = work / "par"
    have_par = False
    if args.par2:
        par_dir.mkdir()
        shutil.copyfile(p0, par_dir / "archive.eb")
        rp = run([args.par2, "create", "-q", "-t1", "-r5", "-n1", "prot.par2", "archive.eb"], cwd=par_dir)
        log.append({"step": "par2-create", **rp})
        have_par = rp["exit"] == 0

    outputs = {}

    def op(name, cmd, out):
        rr = run(cmd)
        log.append({"step": name, **rr})
        outputs[name] = out if rr["exit"] == 0 and out.exists() else None

    other = OTHER_PROFILE[args.profile]
    op("pack-again", [ebound, "pack", item, work / "p0b.eb", "--layout", "indexed", "--profile", args.profile], work / "p0b.eb")
    op("repack-preserve", [ebound, "repack", p0, work / "o1.eb"], work / "o1.eb")
    op("repack-index-absent", [ebound, "repack", p0, work / "o2.eb", "--index", "absent"], work / "o2.eb")
    if outputs["repack-index-absent"]:
        op("repack-index-present-from-absent", [ebound, "repack", work / "o2.eb", work / "o3.eb", "--index", "present"], work / "o3.eb")
    op("repack-layout-stream", [ebound, "repack", p0, work / "o4.eb", "--layout", "stream", "--stream-window", "auto"], work / "o4.eb")
    if outputs["repack-layout-stream"]:
        op("repack-layout-indexed-from-stream", [ebound, "repack", work / "o4.eb", work / "o4b.eb", "--layout", "indexed"], work / "o4b.eb")
    op("repack-profile-other", [ebound, "repack", p0, work / "o5.eb", "--profile", other], work / "o5.eb")
    xdir = work / "extract"
    ru = run([ebound, "unpack", p0, xdir])
    log.append({"step": "unpack-p0", **ru})
    if ru["exit"] == 0:
        op("replan-from-extract", [ebound, "pack", xdir, work / "o6.eb", "--layout", "indexed", "--profile", args.profile], work / "o6.eb")

    scored_total = matched_total = 0
    binding = {c: {"benign_ops": 0, "still_matches": 0, "reported_stale": 0}
               for c in ("bind-pci", "bind-section-chunk-digests", "bind-lai-plus-signature",
                         "filename-convention-only", "receipt-or-detached-signature",
                         "par-native-file-hash", "wrapper-index-artifact")}
    for name, out in outputs.items():
        rec = {"output_present": out is not None}
        if out is not None:
            ids = identities(ebound, out, log)
            pat = pattern(ref, ids)
            s, m = conformance(name, pat)
            scored_total += s
            matched_total += m
            rec.update({"identities": ids, "pattern": pat, "expectation_cells": s, "expectation_matched": m})
            if have_sig:
                rv = run([ebound, "verify", out, "--signature", sig])
                log.append({"step": f"verify-signature-{name}", **{k: rv[k] for k in ("exit", "stderr", "wall_s")}})
                rec["signature"] = parse_signature(rv["stdout"])
                rec["signature_verify_exit"] = rv["exit"]
            if have_par:
                pd = work / f"par-{name}"
                shutil.copytree(par_dir, pd)
                shutil.copyfile(out, pd / "archive.eb")
                rv = run([args.par2, "verify", "-q", "-t1", "prot.par2"], cwd=pd)
                rec["par2_verify_exit"] = rv["exit"]
                shutil.rmtree(pd)
            if pat is not None:
                sigd = rec.get("signature") or {}
                matches = {
                    "bind-pci": pat["pci"] == "same",
                    "bind-section-chunk-digests": pat["pcr"] == "same",
                    "bind-lai-plus-signature": pat["lai"] == "same" and sigd.get("cryptographic") == "VALID",
                    "filename-convention-only": True,
                    "receipt-or-detached-signature": sigd.get("physical") == "VALID",
                    "par-native-file-hash": rec.get("par2_verify_exit") == 0,
                    "wrapper-index-artifact": pat["pci"] == "same",
                }
                stale_reported = {
                    "receipt-or-detached-signature": sigd.get("physical") == "STALE",
                    "par-native-file-hash": rec.get("par2_verify_exit") not in (None, 0),
                }
                for c, ok in matches.items():
                    binding[c]["benign_ops"] += 1
                    binding[c]["still_matches"] += int(bool(ok))
                    binding[c]["reported_stale"] += int(bool(stale_reported.get(c, False)) and not ok)
        detail["ops"][name] = rec

    rollback = rollback_arm(args, ebound, item, work, sk, log) if args.series == "auto" else None
    detail["rollback"] = rollback
    detail["binding"] = binding

    n_ops = len(outputs)
    payload.update({
        "ops_total": n_ops,
        "ops_failed": sum(1 for v in outputs.values() if v is None),
        "expectation_cells": scored_total,
        "expectation_mismatches": scored_total - matched_total,
        "migration_identity_conformance_fraction": (matched_total / scored_total) if scored_total else None,
        "pack_again_pci_same": (detail["ops"].get("pack-again", {}).get("pattern") or {}).get("pci") == "same",
        "replan_pcr_same": (detail["ops"].get("replan-from-extract", {}).get("pattern") or {}).get("pcr") == "same",
        "replan_pci_same": (detail["ops"].get("replan-from-extract", {}).get("pattern") or {}).get("pci") == "same",
        "replan_wall_s": next((e["wall_s"] for e in log if e.get("step") == "replan-from-extract"), None),
        "binding_survival_fraction": {c: (v["still_matches"] / v["benign_ops"]) if v["benign_ops"] else None
                                      for c, v in binding.items()},
        "rollback_undetected": (rollback or {}).get("undetected"),
    })
    return emit(args, detail, payload)


def rollback_arm(args, ebound, item, work, sk, log):
    versions = version_dirs(item)
    if not versions:
        return None
    old, new = versions[-2], versions[-1]
    res = {"old_version": old.name, "new_version": new.name, "version_order": "bytewise-name (protocol C3 fallback)"}
    arcs = {}
    for tag, src in (("old", old), ("new", new)):
        a = work / f"rb-{tag}.eb"
        r = run([ebound, "pack", src, a, "--layout", "indexed", "--profile", args.profile])
        log.append({"step": f"rollback-pack-{tag}", **{k: r[k] for k in ("exit", "stderr", "wall_s")}})
        if r["exit"] != 0:
            res["refused"] = True
            return res
        s = work / f"rb-{tag}.ebsig"
        r = run([ebound, "sign", a, "--signing-key", sk, "--detached", s, "--bind-physical"])
        log.append({"step": f"rollback-sign-{tag}", **{k: r[k] for k in ("exit", "stderr", "wall_s")}})
        arcs[tag] = (a, s, identities(ebound, a, log))
    (old_a, old_s, old_id), (_, _, new_id) = arcs["old"], arcs["new"]
    rv = run([ebound, "verify", old_a, "--signature", old_s])
    sigd = parse_signature(rv["stdout"]) or {}
    # The attacker serves the old, validly signed archive to a caller expecting the new one.
    detected = {
        "status-quo-none": rv["exit"] != 0 or sigd.get("cryptographic") != "VALID",
        "expected-pci-option": old_id is not None and new_id is not None and old_id["pci"] != new_id["pci"],
        "expected-addressing-signature": None,  # addressing binding exists only for encrypted archives
        "min-timestamp-policy": None,  # MODEL: no timestamp tokens in this arm (protocol section 3)
        "signed-version-counter-new-version": None,  # MODEL
        "tuf-style-external-metadata": None,  # MODEL
        "self-contained-freshness-claim": rv["exit"] != 0,
    }
    res.update({"old_signature": sigd, "detected": detected,
                "undetected": {k: (None if v is None else int(not v)) for k, v in detected.items()}})
    return res


def deterministic_view(detail):
    """The subset of the detail that must be identical across repetitions (repeat-hash, §4.13).

    Excludes wall times, command text and the signer key id (a fresh signing key is generated
    per repetition); keeps identities, change patterns, signature and binding statuses, par2
    outcomes and rollback outcomes.
    """
    def sig(d):
        return None if d is None else {k: v for k, v in sorted(d.items()) if k != "signer"}
    ops = {}
    for name, rec in sorted(detail.get("ops", {}).items()):
        ids = rec.get("identities")
        if ids is not None:
            # Values of cells pre-registered as "report" may legitimately depend on the run (for
            # example AUX after unpack restores directory times); only their pattern is hashed.
            exp = EXPECTED.get(name, {})
            ids = {k: v for k, v in ids.items() if exp.get(k, "same") != "report"}
        ops[name] = {"output_present": rec.get("output_present"), "identities": ids,
                     "pattern": rec.get("pattern"), "signature": sig(rec.get("signature")),
                     "signature_verify_exit": rec.get("signature_verify_exit"),
                     "par2_verify_exit": rec.get("par2_verify_exit")}
    rb = detail.get("rollback")
    if rb:
        rb = {k: (sig(v) if k == "old_signature" else v) for k, v in rb.items()}
    return {"reference": detail.get("reference"), "ops": ops, "rollback": rb, "binding": detail.get("binding")}


def emit(args, detail, payload) -> int:
    root = Path(args.detail_root) / args.candidate / args.item_id
    root.mkdir(parents=True, exist_ok=True)
    path = root / f"rep{args.rep}.json.gz"
    raw = json.dumps(detail, sort_keys=True, separators=(",", ":")).encode("utf-8")
    with open(path, "wb") as fh, gzip.GzipFile(fileobj=fh, mode="wb", mtime=0) as gz:
        gz.write(raw)
    det = json.dumps(deterministic_view(detail), sort_keys=True, separators=(",", ":")).encode("utf-8")
    payload["detail_sha256"] = hashlib.sha256(det).hexdigest()
    payload["detail_file_sha256"] = hashlib.sha256(path.read_bytes()).hexdigest()
    payload["detail_path"] = str(path)
    print(json.dumps({"schema": SCHEMA, "experiment_id": EXPERIMENT_ID, "item_id": args.item_id,
                      "candidate": args.candidate, "payload": payload}, sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main())
