"""Mechanical metadata scan for the Phase C experiment pre-registration design review (round 1).

Regenerates research/experiments/_review/design-review-round1-metadata.csv, the per-experiment
flag table that research/experiments/design-review-round1.md cites. It reads only committed files
under research/ and never prints corpus item identifiers.

Held-out discipline: the corpus manifest is filtered to split in {tuning, validation} in memory
before any use; no held-out row, path or statistic is written or printed. (The one-off held-out
identifier check reported in the review is deliberately NOT part of this script, so that running it
does not expose a session to held-out identities.)

Usage (Windows host, repository root; add --mde for the synthetic MDE feasibility table):
  C:/Python313/python.exe research/experiments/_review/design_review_round1_scan.py

Requires PyYAML. Output columns are documented in FLAG_DOC below.
"""

from __future__ import annotations

import csv
import glob
import json
import os
import re
import sys

REPO = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", ".."))
R = os.path.join(REPO, "research")
EXP = os.path.join(R, "experiments")
OUT = os.path.join(EXP, "_review", "design-review-round1-metadata.csv")

FLAG_DOC = {
    "READY_UNBUILT": "status READY but tooling_to_build names repository paths that do not exist yet",
    "TIMING_FLAG_MISMATCH": "index requires_timing disagrees with the committed specs (timing spec present but flag False)",
    "TIMING_NO_AFFINITY": "a spec with timing: true has no platform.cpu_affinity",
    "GUARD_LOOSE": "timing spec guard looser than decision-method 4.4 (loadavg > 1.0 + |affinity|, other CPU > 3.0, or max_wait_s < 300)",
    "TIMED_CMD_ON_MNT": "a timing spec's command reads a script or helper from /mnt/* (drvfs/9P) inside the timed sample",
    "SPEC_MIXES_SPLITS": "one spec selects both tuning and validation",
    "AUTO_BLOCKS_EMPTY": "protocol.md contains empty generated <!-- BEGIN AUTO:... --> blocks",
    "SPEC_REF_MISSING": "protocol references spec-*.yaml files that do not exist and whose mention is not marked as generated later (heuristic)",
    "BELOW_MIN_SUPPORT": "an explicit item list in some spec has < 10 independence groups or < 5 families in a split (informational: only matters for banded corpus-level verdicts)",
    "DISK_OVER_HEADROOM": "disk_gb_estimate > 250 (D: headroom after disk_guard's 75 GB floor is about 350 GB at review time)",
    "NO_METRIC_CANONICAL": "no spec metric (or its '__stratum' prefix) is a canonical Appendix A.2 name",
}


def load_yaml_docs(path):
    import yaml  # noqa: PLC0415

    try:
        with open(path, encoding="utf-8") as fh:
            return [d for d in yaml.safe_load_all(fh) if isinstance(d, dict)]
    except Exception:  # malformed specs are reported by ebr validate, not here
        return []


def _t_cdf(x: float, df: float) -> float:
    """Student t CDF by Simpson integration of the density (adequate for a method check)."""
    import math  # noqa: PLC0415

    if x < 0:
        return 1.0 - _t_cdf(-x, df)
    c = math.exp(math.lgamma((df + 1) / 2) - math.lgamma(df / 2)) / math.sqrt(df * math.pi)
    n = 4000
    h = x / n
    s = 0.0
    for i in range(n + 1):
        t = i * h
        w = 1 if i in (0, n) else (4 if i % 2 else 2)
        s += w * c * (1 + t * t / df) ** (-(df + 1) / 2)
    return 0.5 + s * h / 3


def _t_quantile(p: float, df: float) -> float:
    lo, hi = 0.0, 50.0
    for _ in range(60):
        mid = (lo + hi) / 2
        if _t_cdf(mid, df) < p:
            lo = mid
        else:
            hi = mid
    return (lo + hi) / 2


def mde_check() -> int:
    """Synthetic MDE feasibility check (decision-method 4.10/4.12 formulas; no experiment data).

    Uses only the validation split's independence-group counts per family (bookkeeping fields),
    equal family weights, a common within-family between-group SD `sd` of paired log ratios, a
    one-sided superiority alpha of 0.005 (k_sup = 1) and power 0.80. Prints MDE in band units for
    T-01 (r = 0.01), T-04 (r = 0.05) and T-06 (r = 0.10).
    """
    import math  # noqa: PLC0415

    manifest = json.load(open(os.path.join(R, "corpus", "manifest.json"), encoding="utf-8"))
    groups: dict = {}
    for it in manifest["items"]:
        if it.get("split") == "validation":
            groups.setdefault(it["family"], set()).add(it["independence_group"])
    n = {f: len(g) for f, g in groups.items()}
    pooled_df = sum(k - 1 for k in n.values() if k >= 2)
    w = 1.0 / len(n)
    print("validation groups per family:", dict(sorted(n.items())), "total", sum(n.values()))
    print("sd_log,df,se_log,mde_bands_T01,mde_bands_T04,mde_bands_T06")
    for sd in (0.01, 0.02, 0.03, 0.05, 0.10):
        parts = [(w * w * sd * sd / k, (k - 1) if k >= 2 else pooled_df) for k in n.values()]
        v = sum(p for p, _ in parts)
        df = v * v / sum(p * p / d for p, d in parts)
        se = math.sqrt(v)
        mult = _t_quantile(0.995, df) + _t_quantile(0.80, df)
        bands = [mult * se / math.log1p(r) for r in (0.01, 0.05, 0.10)]
        print(f"{sd:.2f},{df:.1f},{se:.4f},{bands[0]:.2f},{bands[1]:.2f},{bands[2]:.2f}")
    return 0


def main() -> int:
    if "--mde" in sys.argv[1:]:
        return mde_check()
    thresholds = json.load(open(os.path.join(R, "methods", "thresholds.json"), encoding="utf-8"))
    canon = set(thresholds["metric_thresholds"])
    manifest = json.load(open(os.path.join(R, "corpus", "manifest.json"), encoding="utf-8"))
    visible = {
        it["item_id"]: (it["split"], it["family"], it["independence_group"])
        for it in manifest["items"]
        if it.get("split") in ("tuning", "validation")
    }
    index_rows = list(csv.DictReader(open(os.path.join(EXP, "index.csv"), encoding="utf-8")))

    out_rows = []
    for row in index_rows:
        e = row["experiment_id"]
        edir = os.path.join(EXP, e)
        flags = []
        specs = sorted(set(glob.glob(os.path.join(edir, "**", "*.yaml"), recursive=True)))
        docs = []
        for sp in specs:
            for d in load_yaml_docs(sp):
                if d.get("schema") == "ebr.spec.v1":
                    docs.append((sp, d))

        # READY with unbuilt named tooling
        if row["status"] == "READY":
            missing = []
            for p in re.findall(r"(research/[\w./\-{},]+)", row["tooling_to_build"]):
                p = p.rstrip(";,.)")
                cands = [p]
                if "{" in p and "}" in p:
                    pre, rest = p.split("{", 1)
                    inner, suf = rest.split("}", 1)
                    cands = [pre + i + suf for i in inner.split(",")]
                for c in cands:
                    if not os.path.exists(os.path.join(REPO, c)):
                        missing.append(c)
            if missing:
                flags.append("READY_UNBUILT")

        any_timing = False
        canon_hits = 0
        for sp, d in docs:
            timing = bool(d.get("timing"))
            any_timing |= timing
            for m in d.get("metrics", []) or []:
                name = str(m.get("name", ""))
                if name in canon or name.split("__")[0] in canon:
                    canon_hits += 1
            plat = d.get("platform") or {}
            guard = d.get("guard") or {}
            corpus = d.get("corpus") or {}
            splits = corpus.get("splits") or []
            if "tuning" in splits and "validation" in splits:
                flags.append("SPEC_MIXES_SPLITS")
            if timing:
                aff = plat.get("cpu_affinity")
                if not aff:
                    flags.append("TIMING_NO_AFFINITY")
                lav = guard.get("max_loadavg_1m")
                if (
                    (aff and lav is not None and float(lav) > 1.0 + len(aff) + 1e-9)
                    or float(guard.get("max_other_cpu_percent", 0) or 0) > 3.0
                    or float(guard.get("max_wait_s", 300) or 0) < 300
                ):
                    flags.append("GUARD_LOOSE")
                cmd = str(d.get("command", "")) + " ".join(
                    str(c.get("command", "")) for c in (d.get("candidates") or []) if isinstance(c, dict)
                )
                if "/mnt/" in cmd:
                    flags.append("TIMED_CMD_ON_MNT")
            ids = [i for i in (corpus.get("item_ids") or []) if i in visible]
            by_split = {}
            for i in ids:
                s, fam, grp = visible[i]
                by_split.setdefault(s, set()).add((fam, grp))
            for s, groups in by_split.items():
                if len(groups) < 10 or len({f for f, _ in groups}) < 5:
                    flags.append("BELOW_MIN_SUPPORT")
        if docs and canon_hits == 0:
            flags.append("NO_METRIC_CANONICAL")
        if any_timing and row["requires_timing"] == "False":
            flags.append("TIMING_FLAG_MISMATCH")

        proto_path = os.path.join(edir, "protocol.md")
        text = open(proto_path, encoding="utf-8").read() if os.path.exists(proto_path) else ""
        blocks = re.findall(r"<!-- BEGIN AUTO:(\w+) -->(.*?)<!-- END AUTO:\1 -->", text, re.S)
        if blocks and all(not body.strip() for _, body in blocks):
            flags.append("AUTO_BLOCKS_EMPTY")
        for n in set(re.findall(r"\b(spec-[A-Za-z0-9_.\-]+?\.yaml)\b", text)):
            if glob.glob(os.path.join(edir, "**", n), recursive=True):
                continue
            ctx = " ".join(m.group(0) for m in re.finditer(r".{0,120}" + re.escape(n) + r".{0,60}", text))
            if not re.search(r"generat|written at|derived|after|to be written|make_|design freeze|Commit A|later", ctx, re.I):
                flags.append("SPEC_REF_MISSING")
                break
        try:
            if float(row["disk_gb_estimate"]) > 250:
                flags.append("DISK_OVER_HEADROOM")
        except ValueError:
            pass

        out_rows.append(
            {
                "experiment_id": e,
                "domain": row["domain"],
                "status": row["status"],
                "requires_timing": row["requires_timing"],
                "decision_count": row["decision_count"],
                "compute_hours_estimate": row["compute_hours_estimate"],
                "disk_gb_estimate": row["disk_gb_estimate"],
                "spec_documents": len(docs),
                "flags": ";".join(sorted(set(flags))),
            }
        )

    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    with open(OUT, "w", encoding="utf-8", newline="\n") as fh:
        fh.write("# generated by research/experiments/_review/design_review_round1_scan.py; flag meanings:\n")
        for k, v in FLAG_DOC.items():
            fh.write(f"# {k}: {v}\n")
        w = csv.DictWriter(fh, fieldnames=list(out_rows[0].keys()), lineterminator="\n")
        w.writeheader()
        w.writerows(out_rows)

    counts = {}
    for r in out_rows:
        for f in filter(None, r["flags"].split(";")):
            counts[f] = counts.get(f, 0) + 1
    print(f"experiments={len(out_rows)}")
    for k in FLAG_DOC:
        print(f"{k}={counts.get(k, 0)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
