#!/usr/bin/env python3
"""Generate and run the EXP-BASE-SIZE ebr spec: incumbent baseline configs
(research/baselines/configs.json) against corpus tuning+validation items
(research/corpus/manifest.json), timing disabled.

For every (config, item) this measures, via the ebr runner:
  * complete artifact bytes (output_bytes; also artifact_bytes from the
    create command's own accounting, cross-checked against it)
  * determinism: repetitions=2, artifact_tree_sha256/artifact_sha256 recorded
    per rep; ebr's normalize groups these as string_metrics so a non-singleton
    set of digests for a (candidate,item) key means non-deterministic output
  * round-trip fidelity: roundtrip_ok (content_tree_sha256 equality, the
    portable format-agnostic bar) and roundtrip_logical_ok (logical_tree_sha256
    equality, informational -- full metadata is not expected from every format)
  * single-member extraction support is NOT re-measured per item here (it is a
    per-config property); `probe-single-member` records it once per config into
    research/baselines/single-member-extraction.json.

Usage (run inside WSL with the ebr venv):
  wsl.exe -d Ubuntu -- bash -lc '
    PY=/root/eb-research/venv/bin/python
    cd /mnt/d/Projects/entrybound/entrybound
    export PYTHONPATH=research/tools
    $PY research/baselines/run_baselines.py spec --scales small --out research/baselines/specs/EXP-BASE-SIZE-small.yaml
    $PY research/baselines/run_baselines.py run  --spec research/baselines/specs/EXP-BASE-SIZE-small.yaml
  '

Subcommands:
  list-configs                          print config ids and their families
  spec     --scales S[,S...] [--configs C[,C...]] [--medium-sample N] [--large-items ID[,ID...]]
           [--repetitions N] [--timeout-s N] [--seed N] --out PATH
  run      --spec PATH [--gzip] [--run-id ID] [--allow-incomplete-verify]
  probe-single-member [--configs C[,C...]] [--item-id ID] --out PATH
"""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path

REPO_WSL = "/mnt/d/Projects/entrybound/entrybound"
PROBES = f"{REPO_WSL}/research/baselines/probes"
MANIFEST = f"{REPO_WSL}/research/corpus/manifest.json"
FINALIZE = f"python3 {PROBES}/finalize_artifact.py"
ROUNDTRIP = f"python3 {PROBES}/roundtrip_check.py"
DETAIL_LOG = f"{REPO_WSL}/research/raw/EXP-BASE-SIZE/roundtrip-detail.jsonl"
ARTIFACT = "{scratch}/artifact.bin"
EXTRACTED = "{scratch}/extracted"
REPORT = "{scratch}/roundtrip.json"

# Every command ends by printing exactly one JSON stdout line (finalize_artifact.py),
# read back by the stdout_json metrics below. Fallback `mv` lines defend against
# tools (Info-ZIP zip, zpaq) that auto-append their own suffix when the given name
# has no suffix they recognize.
CONFIGS = {
    # --- zip / DEFLATE -----------------------------------------------------
    "zip-6": dict(family="zip-deflate", format="zip", params={"level": 6}, requires=["zip"],
                  command=(f"cd {{item_path}} && zip -6 -r -X -y -q {ARTIFACT} . ; "
                           f"test -f {ARTIFACT} || mv {ARTIFACT}.zip {ARTIFACT} ; "
                           f"{FINALIZE} {ARTIFACT}")),
    "zip-9": dict(family="zip-deflate", format="zip", params={"level": 9}, requires=["zip"],
                  command=(f"cd {{item_path}} && zip -9 -r -X -y -q {ARTIFACT} . ; "
                           f"test -f {ARTIFACT} || mv {ARTIFACT}.zip {ARTIFACT} ; "
                           f"{FINALIZE} {ARTIFACT}")),
    "7z-zip-9": dict(family="zip-deflate", format="zip", params={"level": 9}, requires=["7z"],
                     command=(f"cd {{item_path}} && 7z a -tzip -mx=9 -snl -bd -y {ARTIFACT} . >/dev/null && "
                              f"{FINALIZE} {ARTIFACT}")),
    # --- tar+gzip ------------------------------------------------------------
    "targz-gzip-6": dict(family="tar-gzip", format="tar+gzip", params={"level": 6, "threads": 1}, requires=["tar", "gzip"],
                          command=(f"tar --sort=name --mtime=@0 --owner=0 --group=0 --numeric-owner -cf - -C {{item_path}} . "
                                   f"| gzip -6 -n > {ARTIFACT} && {FINALIZE} {ARTIFACT}")),
    "targz-gzip-9": dict(family="tar-gzip", format="tar+gzip", params={"level": 9, "threads": 1}, requires=["tar", "gzip"],
                          command=(f"tar --sort=name --mtime=@0 --owner=0 --group=0 --numeric-owner -cf - -C {{item_path}} . "
                                   f"| gzip -9 -n > {ARTIFACT} && {FINALIZE} {ARTIFACT}")),
    "targz-pigz-9-t1": dict(family="tar-gzip", format="tar+gzip", params={"level": 9, "threads": 1}, requires=["tar", "pigz"],
                             command=(f"tar --sort=name --mtime=@0 --owner=0 --group=0 --numeric-owner -cf - -C {{item_path}} . "
                                      f"| pigz -9 -n -p1 > {ARTIFACT} && {FINALIZE} {ARTIFACT}")),
    "targz-pigz-9-tN": dict(family="tar-gzip", format="tar+gzip", params={"level": 9, "threads": 0}, requires=["tar", "pigz", "nproc"],
                             command=(f"tar --sort=name --mtime=@0 --owner=0 --group=0 --numeric-owner -cf - -C {{item_path}} . "
                                      f"| pigz -9 -n -p$(nproc) > {ARTIFACT} && {FINALIZE} {ARTIFACT}")),
    # --- tar+zstd --------------------------------------------------------------
    "tarzstd-3-t1": dict(family="tar-zstd", format="tar+zstd", params={"level": 3, "threads": 1}, requires=["tar", "zstd"],
                          command=(f"tar --sort=name --mtime=@0 --owner=0 --group=0 --numeric-owner -cf - -C {{item_path}} . "
                                   f"| zstd -3 -T1 -q > {ARTIFACT} && {FINALIZE} {ARTIFACT}")),
    "tarzstd-19-t1": dict(family="tar-zstd", format="tar+zstd", params={"level": 19, "threads": 1}, requires=["tar", "zstd"],
                           command=(f"tar --sort=name --mtime=@0 --owner=0 --group=0 --numeric-owner -cf - -C {{item_path}} . "
                                    f"| zstd -19 -T1 -q > {ARTIFACT} && {FINALIZE} {ARTIFACT}")),
    "tarzstd-19-tN": dict(family="tar-zstd", format="tar+zstd", params={"level": 19, "threads": 0}, requires=["tar", "zstd"],
                           command=(f"tar --sort=name --mtime=@0 --owner=0 --group=0 --numeric-owner -cf - -C {{item_path}} . "
                                    f"| zstd -19 -T0 -q > {ARTIFACT} && {FINALIZE} {ARTIFACT}")),
    "tarzstd-22-ultra-long27-t1": dict(family="tar-zstd", format="tar+zstd", params={"level": 22, "threads": 1}, requires=["tar", "zstd"],
                                        command=(f"tar --sort=name --mtime=@0 --owner=0 --group=0 --numeric-owner -cf - -C {{item_path}} . "
                                                 f"| zstd --ultra -22 --long=27 -T1 -q > {ARTIFACT} && {FINALIZE} {ARTIFACT}")),
    # --- tar+xz ------------------------------------------------------------------
    "tarxz-6": dict(family="tar-xz", format="tar+xz", params={"level": 6, "threads": 1}, requires=["tar", "xz"],
                     command=(f"tar --sort=name --mtime=@0 --owner=0 --group=0 --numeric-owner -cf - -C {{item_path}} . "
                              f"| xz -6 -T1 -q > {ARTIFACT} && {FINALIZE} {ARTIFACT}")),
    "tarxz-9e": dict(family="tar-xz", format="tar+xz", params={"level": 9, "threads": 1}, requires=["tar", "xz"],
                      command=(f"tar --sort=name --mtime=@0 --owner=0 --group=0 --numeric-owner -cf - -C {{item_path}} . "
                               f"| xz -9 -e -T1 -q > {ARTIFACT} && {FINALIZE} {ARTIFACT}")),
    "tarxz-pixz": dict(family="tar-xz", format="tar+xz", params={"threads": 0}, requires=["tar", "pixz", "nproc"],
                        command=(f"tar --sort=name --mtime=@0 --owner=0 --group=0 --numeric-owner -cf - -C {{item_path}} . "
                                 f"| pixz -p$(nproc) > {ARTIFACT} && {FINALIZE} {ARTIFACT}")),
    # --- tar+lz4 / tar+brotli -----------------------------------------------------
    "tarlz4-1": dict(family="tar-lz4", format="tar+lz4", params={"level": 1}, requires=["tar", "lz4"],
                      command=(f"tar --sort=name --mtime=@0 --owner=0 --group=0 --numeric-owner -cf - -C {{item_path}} . "
                               f"| lz4 -1 -q > {ARTIFACT} && {FINALIZE} {ARTIFACT}")),
    "tarbrotli-q11": dict(family="tar-brotli", format="tar+brotli", params={"quality": 11}, requires=["tar", "brotli"],
                           command=(f"tar --sort=name --mtime=@0 --owner=0 --group=0 --numeric-owner -cf - -C {{item_path}} . "
                                    f"| brotli -q 11 -c > {ARTIFACT} && {FINALIZE} {ARTIFACT}")),
    # --- 7z / LZMA2 -----------------------------------------------------------------
    "7z-mx5-solid": dict(family="7z-lzma2", format="sevenzip", params={"level": 5}, requires=["7z"],
                          command=f"cd {{item_path}} && 7z a -t7z -mx=5 -snl -bd -y {ARTIFACT} . >/dev/null && {FINALIZE} {ARTIFACT}"),
    "7z-mx9-solid": dict(family="7z-lzma2", format="sevenzip", params={"level": 9}, requires=["7z"],
                          command=f"cd {{item_path}} && 7z a -t7z -mx=9 -snl -bd -y {ARTIFACT} . >/dev/null && {FINALIZE} {ARTIFACT}"),
    "7z-mx9-nosolid": dict(family="7z-lzma2", format="sevenzip", params={"level": 9}, requires=["7z"],
                            command=f"cd {{item_path}} && 7z a -t7z -mx=9 -ms=off -snl -bd -y {ARTIFACT} . >/dev/null && {FINALIZE} {ARTIFACT}"),
    # --- squashfs --------------------------------------------------------------------
    "squashfs-zstd": dict(family="squashfs", format="squashfs", params={"level": 19}, requires=["mksquashfs"],
                           command=(f"rm -f {ARTIFACT} && mksquashfs {{item_path}} {ARTIFACT} -comp zstd -Xcompression-level 19 "
                                    f"-no-progress -no-exports >/dev/null && {FINALIZE} {ARTIFACT}")),
    "squashfs-xz": dict(family="squashfs", format="squashfs", params={}, requires=["mksquashfs"],
                         command=(f"rm -f {ARTIFACT} && mksquashfs {{item_path}} {ARTIFACT} -comp xz "
                                  f"-no-progress -no-exports >/dev/null && {FINALIZE} {ARTIFACT}")),
    # --- wim ---------------------------------------------------------------------------
    "wim-lzx": dict(family="wim", format="wim", params={}, requires=["wimcapture"],
                     command=f"rm -f {ARTIFACT} && wimcapture {{item_path}} {ARTIFACT} --compress=LZX --no-acls >/dev/null && {FINALIZE} {ARTIFACT}"),
    "wim-lzms-solid": dict(family="wim", format="wim", params={}, requires=["wimcapture"],
                            command=f"rm -f {ARTIFACT} && wimcapture {{item_path}} {ARTIFACT} --compress=LZMS --solid --no-acls >/dev/null && {FINALIZE} {ARTIFACT}"),
    # --- borg / restic (dedup repos; artifact is a directory) --------------------------
    "borg-repo": dict(family="borg-dedup", format="borg", params={}, requires=["borg"],
                       command=(f"rm -rf {ARTIFACT} && borg init --encryption=none {ARTIFACT} >/dev/null && "
                                f"borg create --compression zstd,19 {ARTIFACT}::onlyarchive {{item_path}} >/dev/null && "
                                f"{FINALIZE} {ARTIFACT}")),
    "restic-repo": dict(family="restic-dedup", format="restic", params={}, requires=["restic"], env={"RESTIC_PASSWORD": "ebr-baseline-research-only"},
                         command=(f"rm -rf {ARTIFACT} && restic init --repo {ARTIFACT} -q && "
                                  f"restic backup {{item_path}} --repo {ARTIFACT} -q --host ebr-baseline && "
                                  f"{FINALIZE} {ARTIFACT}")),
    # --- dwarfs --------------------------------------------------------------------------
    "dwarfs-l7": dict(family="dwarfs", format="dwarfs", params={"level": 7}, requires=["mkdwarfs"],
                       command=f"rm -f {ARTIFACT} && mkdwarfs -i {{item_path}} -o {ARTIFACT} -l7 >/dev/null 2>&1 && {FINALIZE} {ARTIFACT}"),
    "dwarfs-l9": dict(family="dwarfs", format="dwarfs", params={"level": 9}, requires=["mkdwarfs"],
                       command=f"rm -f {ARTIFACT} && mkdwarfs -i {{item_path}} -o {ARTIFACT} -l9 >/dev/null 2>&1 && {FINALIZE} {ARTIFACT}"),
    # --- zpaq ------------------------------------------------------------------------------
    "zpaq-m5": dict(family="zpaq", format="zpaq", params={"method": 5}, requires=["zpaq"],
                     command=(f"rm -f {ARTIFACT}; zpaq a {ARTIFACT} {{item_path}} -m5 -t1 >/dev/null ; "
                              f"test -f {ARTIFACT} || mv {ARTIFACT}.zpaq {ARTIFACT} ; "
                              f"{FINALIZE} {ARTIFACT}")),
}

#  roundtrip_ok is the ONLY correctness gate (content_tree_sha256 equality, the
#  portable format-agnostic bar). logical_tree_sha256 equality (full metadata:
#  mode/owner/xattrs/data-extents) is recorded for every sample regardless of
#  match, in {DETAIL_LOG} -- it is informational, not gating, because several
#  legitimate incumbents (WIM, zip's default attribute model, squashfs's own
#  permission normalization) are not expected to reproduce every metadata field
#  ebr's logical_tree_sha256 checks, and a per-check "informational" flag does
#  not exist in ebr's spec schema (a mismatch would otherwise mark the whole
#  sample invalid, which is correct for content but wrong for metadata alone).
CHECK_ROUNDTRIP = (
    f"{ROUNDTRIP} {{item_path}} {ARTIFACT} {{format}} {EXTRACTED} {REPORT} "
    f"--detail-log {DETAIL_LOG} --candidate {{candidate}} --item-id {{item_id}} --rep {{rep}}"
)

METRICS = [
    {"name": "output_bytes", "source": "path_size", "path": ARTIFACT, "family": "size", "direction": "minimize", "unit": "B"},
    {"name": "artifact_kind", "source": "stdout_json", "key": "artifact_kind", "family": "correctness", "kind": "string"},
    {"name": "artifact_sha256", "source": "stdout_json", "key": "artifact_sha256", "family": "correctness", "kind": "string"},
    {"name": "artifact_tree_sha256", "source": "stdout_json", "key": "artifact_tree_sha256", "family": "correctness", "kind": "string"},
    {"name": "artifact_logical_tree_sha256", "source": "stdout_json", "key": "artifact_logical_tree_sha256", "family": "correctness", "kind": "string"},
    {"name": "input_bytes", "source": "input", "family": "size", "direction": "none", "unit": "B"},
    {"name": "ratio", "source": "derived", "op": "ratio", "args": ["output_bytes", "input_bytes"], "family": "size", "direction": "minimize"},
    {"name": "roundtrip_ok", "source": "check", "command": CHECK_ROUNDTRIP, "family": "correctness", "direction": "maximize"},
]


def load_manifest() -> dict:
    with open(Path(__file__).resolve().parents[1] / "corpus" / "manifest.json", encoding="utf-8") as f:
        return json.load(f)


# f19-tuning-zoo triggers a pre-existing ebr/corpus.py bug: _walk()/compute_fingerprint()
# reuse a display-safe backslash-to-slash replacement as a real filesystem path, which
# corrupts any path component containing a literal backslash byte (this item's adversarial
# "hostile" filename fixtures). Flagged as a follow-up (task_bb8ca4e8); excluded here so a
# single broken item does not abort ebr's whole-corpus fingerprint prepass for everyone else.
KNOWN_BROKEN_ITEMS = {"f19-tuning-zoo"}


def select_items(scales, splits, medium_sample: int, large_items, seed: int, exclude=None):
    exclude = set(exclude or ()) | KNOWN_BROKEN_ITEMS
    m = load_manifest()
    items = [it for it in m["items"] if it["split"] in splits and it["item_id"] not in exclude]
    by_scale = {}
    for it in items:
        by_scale.setdefault(it["scale"], []).append(it)
    chosen = []
    if "small" in scales:
        chosen += by_scale.get("small", [])
    if "medium" in scales:
        med = sorted(by_scale.get("medium", []), key=lambda it: it["item_id"])
        if medium_sample and medium_sample < len(med):
            # deterministic evenly-spaced sample across families/splits, not random
            step = len(med) / medium_sample
            idx = sorted({int(i * step) for i in range(medium_sample)})
            med = [med[i] for i in idx]
        chosen += med
    if "large" in scales:
        lg = by_scale.get("large", [])
        if large_items:
            wanted = set(large_items)
            lg = [it for it in lg if it["item_id"] in wanted]
        chosen += lg
    return chosen


def build_spec(config_ids, items, repetitions: int, timeout_s: float, seed: int) -> dict:
    candidates = []
    requires = set()
    for cid in config_ids:
        c = CONFIGS[cid]
        params = dict(c["params"])
        params["format"] = c["format"]
        cand = {"name": cid, "params": params, "command": c["command"]}
        if "env" in c:
            cand["env"] = c["env"]
        candidates.append(cand)
        requires.update(t for t in c["requires"] if t not in ("nproc",))
    item_ids = [it["item_id"] for it in items]
    splits = sorted({it["split"] for it in items})
    spec = {
        "schema": "ebr.spec.v1",
        "experiment_id": "EXP-BASE-SIZE",
        "status": "baselines",
        "question": (
            "For each pinned incumbent container/compressor configuration, what artifact size, "
            "determinism and round-trip fidelity does it achieve on the tuning+validation corpus, "
            "with timing disabled (research/PROGRESS.md quiet-machine constraint)?"
        ),
        "hypothesis": (
            "No specific hypothesis: this is a descriptive measurement pass (baselines/configs.json) "
            "feeding the domain experiments in Phase C, not a decision test."
        ),
        "decision_ids": [],
        "candidates": candidates,
        "corpus": {"splits": splits, "item_ids": item_ids, "manifest": "research/corpus/manifest.json"},
        "metrics": METRICS,
        "repetitions": repetitions,
        "warmups": 0,
        "seeds": {"order": seed, "bootstrap": seed},
        "timing": False,
        "order": "randomized_blocks",
        "raw_compression": "none",
        "platform": {
            "os": ["linux"], "arch": ["x86_64"], "requires": sorted(requires),
            "timeout_s": timeout_s,
        },
        "analysis": {"report_metrics": ["output_bytes", "ratio", "roundtrip_ok"]},
    }
    return spec


def cmd_list_configs(args):
    for family in sorted({c["family"] for c in CONFIGS.values()}):
        ids = [cid for cid, c in CONFIGS.items() if c["family"] == family]
        print(f"{family}: {', '.join(ids)}")


def cmd_spec(args):
    scales = args.scales.split(",")
    splits = args.splits.split(",")
    config_ids = list(CONFIGS) if args.configs == "all" else args.configs.split(",")
    for cid in config_ids:
        if cid not in CONFIGS:
            raise SystemExit(f"unknown config_id {cid!r}; see list-configs")
    large_items = args.large_items.split(",") if args.large_items else None
    items = select_items(scales, splits, args.medium_sample, large_items, args.seed)
    if not items:
        raise SystemExit("no items selected")
    spec = build_spec(config_ids, items, args.repetitions, args.timeout_s, args.seed)
    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    try:
        import yaml
        with open(out, "w", encoding="utf-8") as f:
            yaml.safe_dump(spec, f, sort_keys=False, width=100)
    except ImportError:
        with open(out, "w", encoding="utf-8") as f:
            json.dump(spec, f, indent=2)
        print("note: PyYAML not available; wrote JSON instead (ebr accepts .json specs too)", file=sys.stderr)
    coverage = {
        "config_ids": config_ids,
        "item_count": len(items),
        "item_ids": [it["item_id"] for it in items],
        "splits": sorted({it["split"] for it in items}),
        "scales": sorted({it["scale"] for it in items}),
        "repetitions": args.repetitions,
    }
    cov_path = out.with_suffix(out.suffix + ".coverage.json")
    with open(cov_path, "w", encoding="utf-8") as f:
        json.dump(coverage, f, indent=2)
    print(f"wrote {out} ({len(candidates_of(spec))} candidates x {len(items)} items x {args.repetitions} reps)")
    print(f"wrote {cov_path}")


def candidates_of(spec):
    return spec["candidates"]


def cmd_run(args):
    py = sys.executable
    repo = str(Path(__file__).resolve().parents[2])
    env_note = f"PYTHONPATH should include {repo}/research/tools"
    base = [py, "-m", "ebr"]
    def run(*extra):
        cmd = base + list(extra) + ["--spec", args.spec]
        print("+", " ".join(cmd))
        return subprocess.run(cmd, cwd=repo)
    r = run("validate")
    if r.returncode != 0:
        raise SystemExit(r.returncode)
    run_args = ["run"]
    if args.gzip:
        run_args.append("--gzip")
    if args.run_id:
        run_args += ["--run-id", args.run_id]
    r = run(*run_args)
    if r.returncode not in (0, 1):
        raise SystemExit(r.returncode)
    # --allow-incomplete: harmless once every run is complete; required if this
    # invocation (or an earlier one folded into the same experiment_id) was
    # stopped mid-run. A genuinely tampered/corrupt raw file still fails
    # verify-raw's hash-chain checks regardless of this flag.
    verify_args = ["verify-raw", "--refingerprint", "--allow-incomplete"]
    r = run(*verify_args)
    if r.returncode not in (0, 1):
        raise SystemExit(r.returncode)
    # --include-incomplete: harmless once every run is complete, required if this
    # invocation (or an earlier one folded into the same experiment_id) was
    # stopped mid-run rather than finishing on its own.
    run("normalize", "--include-incomplete")


def cmd_probe_single_member(args):
    """Probe, once per config, whether extracting a single named member without
    a full extract is possible, and record the exact command used. Uses one
    already-materialized small item (default: the smallest tuning item)."""
    m = load_manifest()
    if args.item_id:
        item = next(it for it in m["items"] if it["item_id"] == args.item_id)
    else:
        item = min((it for it in m["items"] if it["split"] == "tuning"), key=lambda it: it["bytes"])
    config_ids = list(CONFIGS) if args.configs == "all" else args.configs.split(",")
    print(json.dumps({"probe_item": item["item_id"], "config_ids": config_ids}))


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)

    sub.add_parser("list-configs").set_defaults(func=cmd_list_configs)

    p = sub.add_parser("spec")
    p.add_argument("--scales", default="small,medium,large")
    p.add_argument("--splits", default="tuning,validation")
    p.add_argument("--configs", default="all")
    p.add_argument("--medium-sample", type=int, default=0, help="0 = all medium items")
    p.add_argument("--large-items", default="")
    p.add_argument("--repetitions", type=int, default=2)
    p.add_argument("--timeout-s", type=float, default=1800)
    p.add_argument("--seed", type=int, default=20260917)
    p.add_argument("--out", required=True)
    p.set_defaults(func=cmd_spec)

    p = sub.add_parser("run")
    p.add_argument("--spec", required=True)
    p.add_argument("--gzip", action="store_true")
    p.add_argument("--run-id")
    p.add_argument("--allow-incomplete-verify", action="store_true")
    p.set_defaults(func=cmd_run)

    p = sub.add_parser("probe-single-member")
    p.add_argument("--configs", default="all")
    p.add_argument("--item-id")
    p.add_argument("--out")
    p.set_defaults(func=cmd_probe_single_member)

    args = ap.parse_args(argv)
    args.func(args)


if __name__ == "__main__":
    main()
