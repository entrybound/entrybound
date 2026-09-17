"""Projected MDE entry gate for size and timing campaigns (design review DR1-06; program amendment PA-06).

decision-method.md section 4.12: before a validation look, every confirmatory comparison's
MDE = (t_{df,1-alpha} + t_{df,0.80}) * SE, in log units, must be at most 1 band unit, where SE is the
section 4.10 corpus-level (Welch-Satterthwaite) standard error computed with the TUNING group-level
variance of the same comparison and the VALIDATION group structure. PA-06 makes a projected version of
this check an entry gate BEFORE a campaign's full tuning run, using a cheap tuning proxy of the
candidate effect, so a campaign does not spend its budget before learning that no look can pass.

Modes
  --items CSV      item-level values of one comparison on tuning: columns item_id and log_ratio (or ratio,
                   converted with ln). Items are mapped to (family, independence_group) through
                   research/corpus/manifest.json restricted to split == tuning; group value = mean of item
                   log ratios (section 4.9 L2). Within-family between-group variance per family; families with
                   fewer than 2 tuning groups use the pooled within-family variance.
  --synthetic-sd S one common within-family between-group SD (log units) for every applicable family
                   (reproduces the review's table; no experiment data).

Common options
  --band-r R | --band-a A     relative band half-width r (band unit ln(1+r)) or absolute threshold a
  --alpha A                   one-sided alpha (default 0.005: superiority with k_sup = 1, or non-inferiority)
  --k-sup K                   superiority metrics declared for the pair; alpha becomes 0.005 / K
  --families F01,F02,...      applicable families (default: every family with validation groups)
  --weights equal|PATH        equal family weights, or a JSON {family: weight} (e.g. workload-mix.json)
  --gate G                    pass threshold in band units (default 1.0; held-out projections use 2.0)

Exit status 0 when the projected MDE is at most the gate, 4 when it fails, 2 on input errors.
Held-out discipline: only split == tuning and split == validation manifest rows are read; held-out rows are
dropped in memory before any use and nothing about them is printed. Stdlib only.
"""

from __future__ import annotations

import argparse
import csv
import json
import math
import sys
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
MANIFEST = ROOT / "research" / "corpus" / "manifest.json"


def t_cdf(x: float, df: float) -> float:
    if x < 0:
        return 1.0 - t_cdf(-x, df)
    c = math.exp(math.lgamma((df + 1) / 2) - math.lgamma(df / 2)) / math.sqrt(df * math.pi)
    n = 4000
    h = x / n
    s = 0.0
    for i in range(n + 1):
        t = i * h
        w = 1 if i in (0, n) else (4 if i % 2 else 2)
        s += w * c * (1 + t * t / df) ** (-(df + 1) / 2)
    return 0.5 + s * h / 3


def t_quantile(p: float, df: float) -> float:
    lo, hi = 0.0, 60.0
    for _ in range(70):
        mid = (lo + hi) / 2
        if t_cdf(mid, df) < p:
            lo = mid
        else:
            hi = mid
    return (lo + hi) / 2


def visible_manifest():
    doc = json.loads(MANIFEST.read_text(encoding="utf-8"))
    out = {}
    for it in doc["items"]:
        if it.get("split") in ("tuning", "validation"):
            out[it["item_id"]] = (it["split"], it["family"], it["independence_group"])
    return out


def validation_groups(vis):
    groups = defaultdict(set)
    for split, fam, grp in vis.values():
        if split == "validation":
            groups[fam].add(grp)
    return {f: len(g) for f, g in groups.items()}


def tuning_variances(path, vis):
    by_group = defaultdict(list)
    with open(path, encoding="utf-8", newline="") as fh:
        for r in csv.DictReader(line for line in fh if not line.startswith("#")):
            iid = r["item_id"]
            if iid not in vis or vis[iid][0] != "tuning":
                raise SystemExit(f"item {iid!r} is not a tuning item of the manifest (only tuning items may feed the projection)")
            v = float(r["log_ratio"]) if r.get("log_ratio") not in (None, "") else math.log(float(r["ratio"]))
            _, fam, grp = vis[iid]
            by_group[(fam, grp)].append(v)
    fam_groups = defaultdict(list)
    for (fam, _), vals in by_group.items():
        fam_groups[fam].append(sum(vals) / len(vals))
    var, pooled_ss, pooled_df = {}, 0.0, 0
    for fam, gv in fam_groups.items():
        if len(gv) >= 2:
            m = sum(gv) / len(gv)
            s2 = sum((x - m) ** 2 for x in gv) / (len(gv) - 1)
            var[fam] = s2
            pooled_ss += s2 * (len(gv) - 1)
            pooled_df += len(gv) - 1
    pooled = pooled_ss / pooled_df if pooled_df else None
    return var, pooled, {f: len(g) for f, g in fam_groups.items()}


def project(n_val, variances, pooled, weights, alpha, gate, band_unit):
    pooled_df = sum(k - 1 for k in n_val.values() if k >= 2)
    parts = []
    for fam, k in n_val.items():
        s2 = variances.get(fam, pooled)
        if s2 is None:
            raise SystemExit(f"family {fam}: no tuning variance and no pooled variance available")
        v = weights[fam] ** 2 * s2 / k
        df = (k - 1) if k >= 2 else pooled_df
        parts.append((fam, v, df))
    V = sum(v for _, v, _ in parts)
    df = V * V / sum(v * v / d for _, v, d in parts if d > 0)
    se = math.sqrt(V)
    mult = t_quantile(1 - alpha, df) + t_quantile(0.80, df)
    mde_bands = mult * se / band_unit
    return {"validation_groups": n_val, "df": round(df, 2), "se_log": se, "mde_log": mult * se,
            "mde_band_units": mde_bands, "gate_band_units": gate, "pass": mde_bands <= gate,
            "alpha_one_sided": alpha}


def main():
    ap = argparse.ArgumentParser()
    src = ap.add_mutually_exclusive_group(required=True)
    src.add_argument("--items")
    src.add_argument("--synthetic-sd", type=float)
    band = ap.add_mutually_exclusive_group(required=True)
    band.add_argument("--band-r", type=float)
    band.add_argument("--band-a", type=float)
    ap.add_argument("--alpha", type=float, default=0.005)
    ap.add_argument("--k-sup", type=int, default=1)
    ap.add_argument("--families", default="")
    ap.add_argument("--weights", default="equal")
    ap.add_argument("--gate", type=float, default=1.0)
    ap.add_argument("--json", action="store_true")
    args = ap.parse_args()

    vis = visible_manifest()
    n_val = validation_groups(vis)
    if args.families:
        fams = [f for f in args.families.split(",") if f]
        missing = [f for f in fams if f not in n_val]
        if missing:
            print(f"families without validation groups: {missing}", file=sys.stderr)
            return 2
        n_val = {f: n_val[f] for f in fams}
    if args.weights == "equal":
        weights = {f: 1.0 / len(n_val) for f in n_val}
    else:
        raw = json.loads(Path(args.weights).read_text(encoding="utf-8"))
        raw = raw.get("family_weights", raw)
        tot = sum(float(raw[f]) for f in n_val)
        weights = {f: float(raw[f]) / tot for f in n_val}
    band_unit = math.log1p(args.band_r) if args.band_r is not None else args.band_a
    alpha = args.alpha / max(1, args.k_sup) if args.k_sup > 1 else args.alpha

    if args.synthetic_sd is not None:
        variances, pooled, tuning_groups = {f: args.synthetic_sd ** 2 for f in n_val}, args.synthetic_sd ** 2, {}
        label = "SYNTHETIC (not decision-relevant)"
    else:
        variances, pooled, tuning_groups = tuning_variances(args.items, vis)
        label = "PROJECTION from tuning proxy (entry gate PA-06; not a section 4.12 look gate)"
    res = project(n_val, variances, pooled, weights, alpha, args.gate, band_unit)
    res.update({"label": label, "tuning_groups_used": tuning_groups})
    if args.json:
        print(json.dumps(res, indent=2, sort_keys=True))
    else:
        print(label)
        print(f"df={res['df']} se_log={res['se_log']:.5f} mde_band_units={res['mde_band_units']:.3f} gate={args.gate} -> {'PASS' if res['pass'] else 'FAIL'}")
    return 0 if res["pass"] else 4


if __name__ == "__main__":
    sys.exit(main())
