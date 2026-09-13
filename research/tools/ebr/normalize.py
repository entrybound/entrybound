"""Raw JSONL -> research/normalized/<experiment_id>/*.csv + summary.json.

Only verified runs are consumed (verify-raw must pass). Aggregation units follow
``analysis.pairing``: ``item`` aggregates repetitions to per-item medians first
and treats items as the resampling unit; ``item_repetition`` uses individual
samples paired by (run, item, repetition). Timing-family values from runs
without an effective ``--timing`` guard are reported but flagged
``decision_grade=false`` and never receive a verdict.

Outputs (all with a provenance header; deterministic for identical inputs):
  samples.csv        one row per measured sample (warmups excluded)
  failures.csv       samples that failed or were invalid, with reasons
  per_item.csv       descriptives per (env, candidate, item, metric)
  per_candidate.csv  descriptives + bootstrap CI per (env, candidate, metric)
  paired_ratios.csv  candidate/baseline paired ratios, CI, noise-rule verdict
  pareto.csv         epsilon-Pareto frontier over analysis.pareto_objectives
  sensitivity.csv    weight-perturbation sensitivity (analysis.sensitivity)
  summary.json       inputs, counts, headline numbers, verdicts, warnings
"""

from __future__ import annotations

from pathlib import Path
from typing import Any, Dict, List, Optional, Sequence

import numpy as np
import pandas as pd

from . import SUMMARY_SCHEMA, stats
from .outputs import clear_generated, command_line, provenance, write_csv, write_json
from .paths import Layout
from .rawio import list_runs, load_meta, read_rows, verify_run
from .spec import Spec, design_freeze_sha
from .thresholds import TIMING_FAMILIES, Thresholds, ThresholdUnset


class NormalizeError(RuntimeError):
    pass


def _fmt(x: Any) -> Any:
    if x is None:
        return None
    if isinstance(x, (np.floating, float)):
        return None if not np.isfinite(x) else float(x)
    if isinstance(x, (np.integer,)):
        return int(x)
    return x


def load_samples(spec: Spec, layout: Layout, run_ids: Optional[Sequence[str]] = None, include_incomplete: bool = False):
    raw_dir = layout.raw_dir(spec.experiment_id)
    runs = list(run_ids) if run_ids else list_runs(raw_dir)
    if not runs:
        raise NormalizeError(f"no runs under {raw_dir}")
    freeze = design_freeze_sha(layout.design_freeze_path)
    inputs, rows, warnings = [], [], []
    for rid in runs:
        rep = verify_run(raw_dir, rid, allow_incomplete=include_incomplete, design_freeze_sha=freeze)
        if not rep.ok:
            raise NormalizeError(f"verify-raw failed for run {rid}: {rep.errors}")
        meta = load_meta(raw_dir, rid)
        if meta.get("status") != "complete" and not include_incomplete:
            continue
        warnings.extend(f"{rid}: {w}" for w in rep.warnings)
        if meta.get("spec_sha256") != (spec.sha256 if spec.path else meta.get("spec_sha256")):
            warnings.append(f"{rid}: recorded with a different spec revision (sha256 {meta.get('spec_sha256')})")
        path = raw_dir / meta["raw_file"]
        if path.is_file():
            rows.extend(read_rows(path))
        inputs.append(
            {
                "run_id": rid,
                "raw_file": layout.rel(path),
                "raw_sha256": meta.get("raw_sha256"),
                "status": meta.get("status"),
                "env_id": meta.get("env_id"),
                "git_sha": meta.get("git_sha"),
                "git_dirty": meta.get("git_dirty"),
                "timing_effective": (meta.get("timing") or {}).get("effective"),
                "rows": rep.rows,
            }
        )
    if not inputs:
        raise NormalizeError("no complete runs to normalize")
    return inputs, rows, warnings


def samples_frame(spec: Spec, rows: List[Dict[str, Any]]) -> pd.DataFrame:
    recs = []
    for r in rows:
        rec = {
            "run_id": r["run_id"],
            "seq": r["seq"],
            "phase": r["phase"],
            "env_id": r["env_id"],
            "git_sha": r["git_sha"],
            "candidate": r["candidate"],
            "item_id": r["item_id"],
            "split": r["split"],
            "family": r.get("family"),
            "item_key": f"{r['split']}/{r['item_id']}",
            "round": r.get("round"),
            "repetition": r["repetition"],
            "exit_code": r["exit_code"],
            "ok": bool(r["ok"]),
            "valid": bool(r["valid"]),
            "timing_effective": bool(r["timing"].get("effective")),
            "decision_grade_timing": bool(r["timing"].get("decision_grade")),
            "invalid_reasons": ";".join(r.get("invalid_reasons") or []),
            "item_sha256": (r.get("item_fingerprint") or {}).get("sha256"),
        }
        for m in spec.metrics:
            rec[m.name] = r["metrics"].get(m.name)
        recs.append(rec)
    cols = ["run_id", "seq", "phase", "env_id", "git_sha", "candidate", "item_id", "split", "family", "item_key", "round",
            "repetition", "exit_code", "ok", "valid", "timing_effective", "decision_grade_timing", "invalid_reasons",
            "item_sha256"] + [m.name for m in spec.metrics]
    df = pd.DataFrame.from_records(recs, columns=cols)
    cand_order = {c.name: i for i, c in enumerate(spec.candidates)}
    df["_c"] = df["candidate"].map(cand_order)
    df = df.sort_values(["env_id", "_c", "item_key", "run_id", "repetition", "seq"], kind="mergesort").drop(columns="_c")
    return df.reset_index(drop=True)


def _numeric_metrics(spec: Spec):
    return [m for m in spec.metrics if m.kind == "numeric"]


def _units(df: pd.DataFrame, metric: str, pairing: str) -> pd.DataFrame:
    """Aggregation units: per-item medians (pairing=item) or samples (item_repetition)."""
    d = df[["env_id", "candidate", "item_key", "run_id", "repetition", metric]].dropna(subset=[metric])
    d = d.assign(**{metric: d[metric].astype(float)})
    if pairing == "item":
        return d.groupby(["env_id", "candidate", "item_key"], as_index=False, sort=True)[metric].median()
    return d


def normalize(
    spec: Spec,
    layout: Layout,
    run_ids: Optional[Sequence[str]] = None,
    command: Optional[str] = None,
    include_incomplete: bool = False,
) -> Dict[str, Any]:
    out_dir = layout.normalized_dir(spec.experiment_id)
    command = command or command_line("normalize", layout, spec.path)
    inputs, rows, warnings = load_samples(spec, layout, run_ids, include_incomplete)
    thresholds = Thresholds.load(layout.thresholds_path, spec.thresholds_override, spec.decision_ids)
    warnings.extend(thresholds.warnings)
    header = provenance(layout, command, spec.experiment_id, [(i["raw_file"], i["raw_sha256"]) for i in inputs])
    seed = spec.seeds.bootstrap
    resamples = thresholds.bootstrap_resamples()
    try:
        confidence: Optional[float] = thresholds.bootstrap_confidence()
    except ThresholdUnset:
        confidence = None
        warnings.append("bootstrap.confidence unset: confidence intervals and verdicts omitted")

    all_df = samples_frame(spec, rows)
    measured = all_df[all_df["phase"] == "measure"].reset_index(drop=True)
    used = measured[measured["valid"]].reset_index(drop=True)
    failures = measured[~measured["valid"]][["run_id", "seq", "candidate", "item_key", "repetition", "exit_code", "ok", "invalid_reasons"]]
    pairing = spec.analysis.pairing
    fam = {m.name: m.family for m in spec.metrics}

    outputs: Dict[str, str] = {}
    clear_generated(out_dir)
    outputs["samples.csv"] = write_csv(out_dir / "samples.csv", measured, header)
    outputs["failures.csv"] = write_csv(out_dir / "failures.csv", failures.reset_index(drop=True), header)

    # decision-grade status per (env, candidate): all used samples decision-grade?
    dg = used.groupby(["env_id", "candidate"])["decision_grade_timing"].all().to_dict() if len(used) else {}

    def decision_grade(env: str, cands: Sequence[str], metric: str) -> bool:
        if fam[metric] not in TIMING_FAMILIES:
            return True
        return all(dg.get((env, c), False) for c in cands)

    # ---- per_item ---------------------------------------------------------------
    per_item_recs = []
    for m in _numeric_metrics(spec):
        d = used.dropna(subset=[m.name])
        for (env, cand, item), g in d.groupby(["env_id", "candidate", "item_key"], sort=True):
            desc = stats.describe(g[m.name].astype(float).to_numpy())
            per_item_recs.append({"env_id": env, "candidate": cand, "item_key": item, "metric": m.name, "family": m.family,
                                  "unit": m.unit, "decision_grade": decision_grade(env, [cand], m.name), **desc})
    per_item = pd.DataFrame(per_item_recs, columns=["env_id", "candidate", "item_key", "metric", "family", "unit", "decision_grade",
                                                     "n", "median", "p90", "p95", "iqr", "mad", "min", "max", "mean", "geomean"])
    per_item = _order(per_item, spec, ["env_id", "_c", "metric", "item_key"])
    outputs["per_item.csv"] = write_csv(out_dir / "per_item.csv", per_item, header)

    # ---- per_candidate ------------------------------------------------------------
    pc_recs = []
    for m in _numeric_metrics(spec):
        units = _units(used, m.name, pairing)
        for (env, cand), g in units.groupby(["env_id", "candidate"], sort=True):
            vals = g[m.name].to_numpy()
            desc = stats.describe(vals)
            rec = {"env_id": env, "candidate": cand, "metric": m.name, "family": m.family, "unit": m.unit,
                   "unit_of_analysis": "item_median" if pairing == "item" else "sample",
                   "decision_grade": decision_grade(env, [cand], m.name), "n_units": desc["n"],
                   "n_items": int(used[(used.env_id == env) & (used.candidate == cand)]["item_key"].nunique()),
                   "median": desc["median"], "p90": desc["p90"], "p95": desc["p95"], "iqr": desc["iqr"], "mad": desc["mad"],
                   "geomean": desc["geomean"], "ci_lower": None, "ci_upper": None, "ci_confidence": confidence,
                   "ci_status": None}
            if confidence is None:
                rec["ci_status"] = "threshold_unset:bootstrap.confidence"
            elif desc["n"] < 2:
                rec["ci_status"] = "insufficient_units"
            else:
                ci = stats.bootstrap_median_ci(vals, confidence=confidence, seed=seed, resamples=resamples)
                rec.update(ci_lower=ci.lower, ci_upper=ci.upper, ci_status=f"ok:{ci.method}:resamples={ci.resamples}:seed={seed}")
            pc_recs.append(rec)
    per_cand = pd.DataFrame(pc_recs, columns=["env_id", "candidate", "metric", "family", "unit", "unit_of_analysis", "decision_grade",
                                               "n_units", "n_items", "median", "p90", "p95", "iqr", "mad", "geomean", "ci_lower",
                                               "ci_upper", "ci_confidence", "ci_status"])
    per_cand = _order(per_cand, spec, ["env_id", "metric", "_c"])
    outputs["per_candidate.csv"] = write_csv(out_dir / "per_candidate.csv", per_cand, header)

    # ---- paired ratios --------------------------------------------------------------
    ratio_recs = []
    base = spec.analysis.baseline
    if base is not None:
        for m in _numeric_metrics(spec):
            units = _units(used, m.name, pairing)
            keys = ["env_id", "item_key"] if pairing == "item" else ["env_id", "run_id", "item_key", "repetition"]
            b = units[units.candidate == base].drop(columns="candidate").rename(columns={m.name: "base"})
            for cand in [c.name for c in spec.candidates if c.name != base]:
                c = units[units.candidate == cand].drop(columns="candidate").rename(columns={m.name: "cand"})
                pairs = c.merge(b, on=keys, how="inner")
                for env, g in pairs.groupby("env_id", sort=True):
                    ratio_recs.append(_ratio_record(env, cand, base, m, g, thresholds, confidence, seed, resamples,
                                                    decision_grade(env, [cand, base], m.name), pairing))
    ratios = pd.DataFrame(ratio_recs, columns=["env_id", "candidate", "baseline", "metric", "family", "unit", "pairing",
                                                "decision_grade", "n_pairs", "median_ratio", "geomean_ratio", "ci_lower",
                                                "ci_upper", "ci_confidence", "band_kind", "band_lower", "band_upper", "verdict"])
    ratios = _order(ratios, spec, ["env_id", "metric", "_c"])
    outputs["paired_ratios.csv"] = write_csv(out_dir / "paired_ratios.csv", ratios, header)

    # ---- pareto ----------------------------------------------------------------------
    pareto_recs = []
    objectives = spec.analysis.pareto_objectives
    eps_status = {}
    if objectives:
        epsilons, modes = [], []
        for o in objectives:
            try:
                kind, val = thresholds.epsilon(fam[o])
                epsilons.append(val)
                modes.append(kind)
                eps_status[o] = f"{kind}:{val}"
            except ThresholdUnset:
                epsilons.append(0.0)
                modes.append("absolute")
                eps_status[o] = "unset (strict Pareto, epsilon=0)"
        for env in sorted(per_cand["env_id"].unique()):
            pcs = per_cand[(per_cand.env_id == env) & (per_cand.metric.isin(objectives))]
            wide = pcs.pivot(index="candidate", columns="metric", values="median")
            wide = wide.dropna()
            if wide.empty:
                continue
            order = [c.name for c in spec.candidates if c.name in wide.index]
            wide = wide.loc[order, objectives]
            dirs = [spec.metric(o).direction for o in objectives]
            try:
                front = set(stats.pareto_frontier(wide.to_numpy(), dirs, epsilons, modes))
                dom = stats.dominance_matrix(wide.to_numpy(), dirs, epsilons, modes)
                status = "ok"
            except ValueError as exc:
                front, dom, status = set(), None, f"error:{exc}"
            dg_all = all(decision_grade(env, order, o) for o in objectives)
            for i, cand in enumerate(order):
                rec = {"env_id": env, "candidate": cand, "on_frontier": i in front,
                       "dominated_by": ";".join(order[j] for j in range(len(order)) if dom is not None and dom[j, i]),
                       "decision_grade": dg_all, "status": status,
                       "epsilon": ";".join(f"{o}={eps_status[o]}" for o in objectives)}
                for o in objectives:
                    rec[o] = wide.loc[cand, o]
                pareto_recs.append(rec)
    pareto = pd.DataFrame(pareto_recs, columns=["env_id", "candidate", *objectives, "on_frontier", "dominated_by", "decision_grade", "status", "epsilon"])
    outputs["pareto.csv"] = write_csv(out_dir / "pareto.csv", pareto, header)

    # ---- sensitivity --------------------------------------------------------------------
    sens_recs = []
    sens_cfg = spec.analysis.sensitivity
    if sens_cfg:
        tsens = thresholds.sensitivity()
        weights = sens_cfg["weights"]
        crit = list(weights)
        dsamples = sens_cfg.get("dirichlet_samples", tsens.get("dirichlet_samples"))
        dconc = sens_cfg.get("dirichlet_concentration", tsens.get("dirichlet_concentration"))
        for env in sorted(per_cand["env_id"].unique()):
            pcs = per_cand[(per_cand.env_id == env) & (per_cand.metric.isin(crit))]
            wide = pcs.pivot(index="candidate", columns="metric", values="median").dropna()
            order = [c.name for c in spec.candidates if c.name in wide.index]
            if len(order) < 2 or set(crit) - set(wide.columns):
                continue
            wide = wide.loc[order, crit]
            scores = stats.normalize_scores(wide.to_numpy(), [spec.metric(c).direction for c in crit])
            res = stats.weight_sensitivity(scores, [float(weights[c]) for c in crit], grid_mode="full",
                                           dirichlet_samples=int(dsamples or 0), dirichlet_concentration=dconc,
                                           seed=seed if dsamples else None)
            sens_recs.append({
                "env_id": env, "criteria": ";".join(f"{c}={weights[c]}" for c in crit), "base_winner": order[res.base_winner],
                "grid_mode": res.grid_mode, "grid_count": res.grid_count, "grid_fraction_preserving": res.grid_fraction_preserving,
                "dirichlet_count": res.dirichlet_count, "dirichlet_fraction_preserving": res.dirichlet_fraction_preserving,
                "dirichlet_concentration": dconc, "seed": seed if dsamples else None,
                "grid_winners": ";".join(f"{order[k]}={v}" for k, v in sorted(res.winner_counts_grid.items())),
                "dirichlet_winners": ";".join(f"{order[k]}={v}" for k, v in sorted(res.winner_counts_dirichlet.items())),
                "decision_grade": all(decision_grade(env, order, c) for c in crit),
            })
    sens = pd.DataFrame(sens_recs, columns=["env_id", "criteria", "base_winner", "grid_mode", "grid_count", "grid_fraction_preserving",
                                             "dirichlet_count", "dirichlet_fraction_preserving", "dirichlet_concentration", "seed",
                                             "grid_winners", "dirichlet_winners", "decision_grade"])
    outputs["sensitivity.csv"] = write_csv(out_dir / "sensitivity.csv", sens, header)

    # ---- summary ----------------------------------------------------------------------
    headline: Dict[str, Any] = {}
    for rec in pc_recs:
        headline.setdefault(rec["env_id"], {}).setdefault(rec["candidate"], {})[rec["metric"]] = {
            "median": _fmt(rec["median"]), "ci_lower": _fmt(rec["ci_lower"]), "ci_upper": _fmt(rec["ci_upper"]),
            "decision_grade": rec["decision_grade"]}
    string_metrics = {}
    for m in spec.metrics:
        if m.kind == "string":
            string_metrics[m.name] = {
                f"{env}|{cand}|{item}": sorted(set(g[m.name].dropna().astype(str)))
                for (env, cand, item), g in used.groupby(["env_id", "candidate", "item_key"], sort=True)
            }
    summary = {
        "schema": SUMMARY_SCHEMA,
        "experiment_id": spec.experiment_id,
        "generated_by": command,
        "provenance": header,
        "spec_path": layout.rel(spec.path) if spec.path else None,
        "spec_sha256": spec.sha256,
        "decision_ids": spec.decision_ids,
        "inputs": inputs,
        "env_ids": sorted(all_df["env_id"].unique().tolist()),
        "counts": {
            "rows": int(len(all_df)),
            "warmup_rows_excluded": int((all_df["phase"] == "warmup").sum()),
            "measured": int(len(measured)),
            "valid": int(len(used)),
            "invalid": int((~measured["valid"]).sum()),
            "failed": int((~measured["ok"]).sum()),
        },
        "timing": {
            "any_effective": bool(measured["timing_effective"].any()) if len(measured) else False,
            "all_decision_grade": bool(measured["decision_grade_timing"].all()) if len(measured) else False,
            "timing_families": sorted(TIMING_FAMILIES),
            "note": "timing-family numbers without decision_grade are informational only",
        },
        "analysis": {"pairing": pairing, "baseline": base, "bootstrap_confidence": confidence,
                     "bootstrap_resamples": resamples, "bootstrap_seed": seed, "pareto_epsilon": eps_status},
        "thresholds_source": thresholds.source,
        "thresholds_status": thresholds.data.get("status"),
        "headline": headline,
        "string_metrics": string_metrics,
        "paired_ratio_verdicts": [
            {k: _fmt(r[k]) for k in ("env_id", "candidate", "baseline", "metric", "median_ratio", "ci_lower", "ci_upper", "verdict")}
            for r in ratio_recs
        ],
        "pareto_frontier": {env: sorted(g[g.on_frontier]["candidate"].tolist()) for env, g in pareto.groupby("env_id")} if len(pareto) else {},
        "sensitivity": [{k: _fmt(v) for k, v in r.items()} for r in sens_recs],
        "outputs": {f"{layout.rel(out_dir / k)}": v for k, v in sorted(outputs.items())},
        "warnings": sorted(set(warnings)),
    }
    write_json(out_dir / "summary.json", summary)
    return summary


def _order(df: pd.DataFrame, spec: Spec, keys: List[str]) -> pd.DataFrame:
    if df.empty:
        return df
    cand_order = {c.name: i for i, c in enumerate(spec.candidates)}
    metric_order = {m.name: i for i, m in enumerate(spec.metrics)}
    df = df.assign(_c=df["candidate"].map(cand_order))
    if "metric" in df.columns:
        df = df.assign(_m=df["metric"].map(metric_order))
        keys = ["_m" if k == "metric" else k for k in keys]
    df = df.sort_values(keys, kind="mergesort")
    return df.drop(columns=[c for c in ("_c", "_m") if c in df.columns]).reset_index(drop=True)


def _ratio_record(env, cand, base, m, g, thresholds: Thresholds, confidence, seed, resamples, dgrade, pairing) -> Dict[str, Any]:
    rec: Dict[str, Any] = {"env_id": env, "candidate": cand, "baseline": base, "metric": m.name, "family": m.family,
                           "unit": m.unit, "pairing": pairing, "decision_grade": dgrade, "n_pairs": int(len(g)),
                           "median_ratio": None, "geomean_ratio": None, "ci_lower": None, "ci_upper": None,
                           "ci_confidence": confidence, "band_kind": None, "band_lower": None, "band_upper": None, "verdict": None}
    cv, bv = g["cand"].to_numpy(float), g["base"].to_numpy(float)
    if len(g) == 0:
        rec["verdict"] = "no_pairs"
        return rec
    if np.any(bv <= 0) or np.any(cv < 0):
        rec["verdict"] = "ratio_undefined:nonpositive_baseline"
        return rec
    r = cv / bv
    rec["median_ratio"] = float(np.median(r))
    rec["geomean_ratio"] = float(np.exp(np.mean(np.log(r)))) if np.all(r > 0) else None
    try:
        kind, lo, hi = thresholds.band(m.family)
        rec.update(band_kind=kind, band_lower=lo, band_upper=hi)
    except ThresholdUnset:
        kind = None
    if confidence is None:
        rec["verdict"] = "threshold_unset:bootstrap.confidence"
        return rec
    if len(g) < 2:
        rec["verdict"] = "insufficient_pairs"
        return rec
    ci = stats.bootstrap_paired_ratio_ci(cv, bv, confidence=confidence, seed=seed, resamples=resamples)
    rec.update(ci_lower=ci.lower, ci_upper=ci.upper)
    if kind is None:
        label = f"threshold_unset:families.{m.family}.practical_significance_band"
    elif kind == "ratio":
        label = stats.noise_rule(ci.lower, ci.upper, (lo, hi)).label
    else:
        dci = stats.bootstrap_paired_difference_ci(cv, bv, confidence=confidence, seed=seed, resamples=resamples)
        label = stats.noise_rule(dci.lower, dci.upper, (lo, hi)).label + ":absolute_difference_ci"
    rec["verdict"] = label if dgrade else f"not_decision_grade(informational:{label})"
    return rec
