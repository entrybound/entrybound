"""Command line: python -m ebr {run,normalize,report,verify-raw,validate,plan} --spec PATH.

Exit codes: 0 ok; 1 samples failed / verification failed; 2 spec or usage error;
3 refused (held-out lock, quiet-machine guard, timing flags, platform).
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import List, Optional

from .corpus import FingerprintCache
from .outputs import command_line
from .paths import Layout
from .rawio import list_runs, verify_run
from .spec import HeldoutLocked, SpecError, design_freeze_sha, load_spec


def _cpus(text: str) -> List[int]:
    out: List[int] = []
    for part in text.split(","):
        if "-" in part:
            a, b = part.split("-", 1)
            out.extend(range(int(a), int(b) + 1))
        elif part.strip():
            out.append(int(part))
    return out


def build_parser() -> argparse.ArgumentParser:
    ap = argparse.ArgumentParser(prog="python -m ebr", description="Entrybound research experiment runner")
    ap.add_argument("--repo-root", help="repository root (default: auto-detect / EB_REPO_ROOT)")
    ap.add_argument("--data-root", help="large-data root (default: /root/eb-research on Linux / EB_RESEARCH_ROOT)")
    sub = ap.add_subparsers(dest="cmd", required=True)

    def spec_arg(p):
        p.add_argument("--spec", required=True, help="experiment spec (YAML or JSON)")

    p = sub.add_parser("run", help="execute the experiment and append raw JSONL")
    spec_arg(p)
    p.add_argument("--timing", action="store_true", help="collect decision-grade timing under the quiet-machine guard")
    p.add_argument("--allow-untimed", action="store_true", help="run a timing:true spec without the guard (samples not decision-grade)")
    p.add_argument("--run-id")
    p.add_argument("--gzip", action="store_true", help="write raw rows as .jsonl.gz")
    p.add_argument("--cpus", type=_cpus, help="pin samples to CPUs, e.g. 2,3 or 4-7")
    p.add_argument("--keep-scratch", action="store_true")
    p.add_argument("--scratch-root")
    p.add_argument("--max-load", type=float, dest="max_load", help="override guard max 1-minute load average")
    p.add_argument("--max-other-cpu", type=float, dest="max_other_cpu", help="override guard max other-process CPU percent")
    p.add_argument("--dry-run", action="store_true", help="resolve items and print the run order only")
    p.add_argument("--quiet", action="store_true")

    p = sub.add_parser("verify-raw", help="verify raw JSONL hashes, chain, plan and provenance")
    spec_arg(p)
    p.add_argument("--run-id", action="append")
    p.add_argument("--allow-incomplete", action="store_true")
    p.add_argument("--refingerprint", action="store_true", help="re-hash corpus items and compare with recorded fingerprints")

    p = sub.add_parser("normalize", help="raw -> normalized CSV + summary JSON")
    spec_arg(p)
    p.add_argument("--run-id", action="append")
    p.add_argument("--include-incomplete", action="store_true")

    p = sub.add_parser("report", help="normalized -> markdown tables + PNG figures")
    spec_arg(p)

    p = sub.add_parser("validate", help="validate a spec (no execution)")
    spec_arg(p)

    p = sub.add_parser("plan", help="alias for run --dry-run")
    spec_arg(p)
    return ap


def main(argv: Optional[List[str]] = None) -> int:
    argv = list(sys.argv[1:] if argv is None else argv)
    args = build_parser().parse_args(argv)
    layout = Layout.detect(args.repo_root, args.data_root)
    try:
        spec = load_spec(args.spec)
    except (SpecError, OSError, ValueError) as exc:
        print(f"ebr: spec error: {exc}", file=sys.stderr)
        return 2

    try:
        if args.cmd == "validate":
            print(json.dumps({"experiment_id": spec.experiment_id, "valid": True, "spec_sha256": spec.sha256}))
            return 0

        if args.cmd in ("run", "plan"):
            from .run import RunOptions, RunRefused, run_experiment

            dry = args.cmd == "plan" or args.dry_run
            opts = RunOptions(
                timing=getattr(args, "timing", False),
                allow_untimed=getattr(args, "allow_untimed", False),
                run_id=getattr(args, "run_id", None),
                compression="gzip" if getattr(args, "gzip", False) else None,
                cpus=getattr(args, "cpus", None),
                keep_scratch=getattr(args, "keep_scratch", False),
                scratch_root=getattr(args, "scratch_root", None),
                max_loadavg_1m=getattr(args, "max_load", None),
                max_other_cpu_percent=getattr(args, "max_other_cpu", None),
                dry_run=dry,
                argv=["python", "-m", "ebr"] + [layout.rel(a) if a == args.spec else a for a in argv],
                quiet=getattr(args, "quiet", False),
            )
            try:
                out = run_experiment(spec, layout, opts)
            except (RunRefused, HeldoutLocked) as exc:
                print(f"ebr: refused: {exc}", file=sys.stderr)
                return 3
            if dry:
                for e in out.plan:
                    print(f"{e.order_index}\t{e.phase}\tround={e.round}\t{e.candidate}\t{e.item_key}\trep={e.repetition}\tseq={e.seq}")
                return 0
            print(json.dumps({"run_id": out.run_id, "status": out.status, "raw_file": layout.rel(out.raw_file),
                              "recorded": out.recorded, "failed": out.failed, "invalid": out.invalid}))
            return 0 if out.failed == 0 else 1

        if args.cmd == "verify-raw":
            raw_dir = layout.raw_dir(spec.experiment_id)
            runs = args.run_id or list_runs(raw_dir)
            if not runs:
                print(f"ebr: no runs under {raw_dir}", file=sys.stderr)
                return 1
            freeze = design_freeze_sha(layout.design_freeze_path)
            refp = None
            if args.refingerprint:
                refp = _refingerprinter(spec, layout, raw_dir)
            ok = True
            for rid in runs:
                rep = verify_run(raw_dir, rid, allow_incomplete=args.allow_incomplete, design_freeze_sha=freeze, refingerprint=refp)
                print(json.dumps(rep.to_dict()))
                ok = ok and rep.ok
            return 0 if ok else 1

        if args.cmd == "normalize":
            from .normalize import NormalizeError, normalize

            extra = []
            for rid in args.run_id or []:
                extra += ["--run-id", rid]
            if args.include_incomplete:
                extra.append("--include-incomplete")
            try:
                summary = normalize(spec, layout, args.run_id, command_line("normalize", layout, args.spec, extra), args.include_incomplete)
            except NormalizeError as exc:
                print(f"ebr: normalize failed: {exc}", file=sys.stderr)
                return 1
            print(json.dumps({"experiment_id": spec.experiment_id, "outputs": summary["outputs"], "warnings": summary["warnings"]}, indent=2))
            return 0

        if args.cmd == "report":
            from .report import ReportError, report

            try:
                written = report(spec, layout, command_line("report", layout, args.spec))
            except ReportError as exc:
                print(f"ebr: report failed: {exc}", file=sys.stderr)
                return 1
            print(json.dumps({"experiment_id": spec.experiment_id, "written": written}, indent=2))
            return 0
    except SpecError as exc:
        print(f"ebr: spec error: {exc}", file=sys.stderr)
        return 2
    except HeldoutLocked as exc:
        print(f"ebr: refused: {exc}", file=sys.stderr)
        return 3
    return 2


def _refingerprinter(spec, layout: Layout, raw_dir: Path):
    from .rawio import load_meta

    cache = FingerprintCache(None)
    paths = {}
    for rid in list_runs(raw_dir):
        for it in load_meta(raw_dir, rid).get("items", []):
            paths[it["item_key"]] = Path(it["path"])

    def refp(key: str):
        p = paths.get(key)
        if p is None or not p.exists():
            return None
        return cache.get(p)["sha256"]

    return refp
