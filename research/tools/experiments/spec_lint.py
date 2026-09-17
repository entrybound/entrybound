"""Design-level lint for experiment ebr specs (design review DR1-07, DR1-09, DR1-16; amendments PA-07, PA-09, PA-16).

ebr validate checks schema validity only. This lint adds the pre-registration rules that the Phase C design
revision made binding for every spec under research/experiments/EXP-*/ (decision-method section 14 item 3 lists
the runner-side spec lint; these rules are its design-side counterpart until the runner implements them):

  E-MNT        a spec with timing: true invokes a script, helper, input or binary under /mnt/ (drvfs/9P), or a
               repository-relative research/ path, in its command, candidate params, env or cwd; stage it to ext4
               with research/tools/experiments/stage_tools.py and set cwd to the staged repo (PA-16)
  E-AFFINITY   a spec with timing: true has no platform.cpu_affinity (sections 4.2/4.3)
  E-GUARD      a timing spec's guard is looser than section 4.4: max_loadavg_1m > 1.0 + |cpu_affinity|,
               max_other_cpu_percent > 3.0 or max_wait_s < 300; a declared deviation in notes
               ("DEVIATION-GUARD: <reason>") downgrades this to W-GUARD-DEVIATION
  E-SPLITS     a spec selects both tuning and validation while reporting a banded or diagnostic canonical
               metric (a graded comparison may not run on validation outside a registered look; PA-09)
  E-LOOKSPEC   a spec selects validation with banded or diagnostic canonical metrics but its status is not
               look-1 or look-2 (a look spec derived from the tuning record with candidates restricted to W and S)
  W-VALSTATUS  a spec selects validation with only binary, descriptive or non-canonical metrics and its status is
               not hc-screen, census or look-N (label it so the look registry can tell screens from looks)
  E-HELDOUT    a spec selects the heldout split (held-out specs are generated only by EXP-EVAL-012 at Commit A)

Usage:  python research/tools/experiments/spec_lint.py [--json] [EXP-ID ...]
Exit 1 if any E-* finding. Requires PyYAML.
"""

from __future__ import annotations

import argparse
import glob
import json
import os
import re
import sys
from pathlib import Path

import yaml

ROOT = Path(__file__).resolve().parents[3]
EXP = ROOT / "research" / "experiments"
THRESHOLDS = ROOT / "research" / "methods" / "thresholds.json"
REPO_RELATIVE_RX = re.compile(r"(?<![\w/.-])research/")


def canonical_roles():
    t = json.loads(THRESHOLDS.read_text(encoding="utf-8"))
    return {k: v.get("role") for k, v in t["metric_thresholds"].items()}


def spec_files(ids):
    pats = [str(EXP / (i or "EXP-*") / "**" / "*.yaml") for i in (ids or [None])]
    out = set()
    for p in pats:
        out.update(glob.glob(p, recursive=True))
    return sorted(out)


def lint_doc(path, d, roles):
    f = []
    rel = os.path.relpath(path, ROOT).replace(os.sep, "/")
    timing = bool(d.get("timing"))
    plat = d.get("platform") or {}
    guard = d.get("guard") or {}
    corpus = d.get("corpus") or {}
    splits = corpus.get("splits") or []
    status = str(d.get("status", ""))
    notes = str(d.get("notes", ""))
    metrics = d.get("metrics") or []
    graded = []
    for m in metrics:
        name = str(m.get("name", ""))
        base = name.split("__")[0]
        role = roles.get(name) or roles.get(base)
        if role in ("banded", "diagnostic"):
            graded.append(name)
    cmds = [str(d.get("command", ""))] + [str(c.get("command", "")) for c in (d.get("candidates") or []) if isinstance(c, dict)]
    cmds += [str(v) for c in (d.get("candidates") or []) if isinstance(c, dict) for v in (c.get("params") or {}).values()]
    cmds += [str(v) for v in (d.get("env") or {}).values()] + [str(d.get("cwd", ""))]
    if timing:
        if any("/mnt/" in c for c in cmds):
            f.append(("E-MNT", "timed command references /mnt/"))
        elif any(REPO_RELATIVE_RX.search(c) for c in cmds):
            f.append(("E-MNT", "timed command references a repository-relative research/ path (the WSL cwd is the /mnt/d checkout)"))
        aff = plat.get("cpu_affinity")
        if not aff:
            f.append(("E-AFFINITY", "timing spec without platform.cpu_affinity"))
        loose = []
        lav = guard.get("max_loadavg_1m")
        if aff and lav is not None and float(lav) > 1.0 + len(aff) + 1e-9:
            loose.append(f"max_loadavg_1m {lav} > {1.0 + len(aff)}")
        if float(guard.get("max_other_cpu_percent", 0) or 0) > 3.0:
            loose.append("max_other_cpu_percent > 3.0")
        if float(guard.get("max_wait_s", 300) or 0) < 300:
            loose.append("max_wait_s < 300")
        if loose:
            code = "W-GUARD-DEVIATION" if "DEVIATION-GUARD:" in notes else "E-GUARD"
            f.append((code, "; ".join(loose)))
    if "heldout" in splits:
        f.append(("E-HELDOUT", "heldout split selected"))
    if "validation" in splits:
        if graded:
            if "tuning" in splits:
                f.append(("E-SPLITS", f"tuning and validation with graded metrics {graded}"))
            elif not re.fullmatch(r"look-[12]", status):
                f.append(("E-LOOKSPEC", f"validation with graded metrics {graded} but status {status!r}"))
        elif not (status in ("hc-screen", "census") or re.fullmatch(r"look-[12]", status)):
            f.append(("W-VALSTATUS", f"validation screen with status {status!r}"))
    return [(rel, d.get("experiment_id"), c, m) for c, m in f]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("ids", nargs="*")
    ap.add_argument("--json", action="store_true")
    args = ap.parse_args()
    roles = canonical_roles()
    findings = []
    for p in spec_files(args.ids):
        try:
            with open(p, encoding="utf-8") as fh:
                docs = [x for x in yaml.safe_load_all(fh) if isinstance(x, dict)]
        except Exception as exc:  # noqa: BLE001
            findings.append((p, None, "E-YAML", str(exc)[:200]))
            continue
        for d in docs:
            if d.get("schema") == "ebr.spec.v1":
                findings.extend(lint_doc(p, d, roles))
    if args.json:
        print(json.dumps([dict(zip(("spec", "experiment_id", "code", "detail"), x)) for x in findings], indent=2))
    else:
        for x in findings:
            print("\t".join(str(v) for v in x))
        errs = sum(1 for x in findings if x[2].startswith("E-"))
        print(f"findings={len(findings)} errors={errs}")
    return 1 if any(x[2].startswith("E-") for x in findings) else 0


if __name__ == "__main__":
    sys.exit(main())
