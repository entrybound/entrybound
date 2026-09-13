"""Markdown tables and matplotlib figures from normalized CSVs.

Outputs
  research/reports/tables/<experiment_id>/{per_candidate,paired_ratios,pareto,sensitivity,index}.md
  research/reports/figures/<experiment_id>/<name>.png + <name>.csv (the exact plotted data)

Every file carries the generating command line: Markdown in a leading HTML
comment, CSV in ``# `` header lines, PNG in its ``Comment``/``Source`` text chunks.
PNGs are written without a timestamp/software chunk so reruns are byte-stable for
a pinned matplotlib.
"""

from __future__ import annotations

from pathlib import Path
from typing import Any, Dict, List, Optional, Sequence

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt  # noqa: E402
import numpy as np  # noqa: E402
import pandas as pd  # noqa: E402

from .outputs import clear_generated, command_line, provenance, read_csv, sha256_path, write_csv, write_text  # noqa: E402
from .paths import Layout  # noqa: E402
from .spec import Spec  # noqa: E402
from .thresholds import TIMING_FAMILIES  # noqa: E402

# Reference light palette (dataviz skill defaults), fixed roles.
SURFACE = "#fcfcfb"
TEXT_PRIMARY = "#0b0b0b"
TEXT_SECONDARY = "#52514e"
GRID = "#e4e3de"
SERIES_1 = "#2a78d6"  # blue
SERIES_2 = "#eb6834"  # orange
NEUTRAL_MARK = "#9a9993"
BAND_FILL = "#dcdbd5"


class ReportError(RuntimeError):
    pass


def _style() -> Dict[str, Any]:
    return {
        "figure.facecolor": SURFACE,
        "axes.facecolor": SURFACE,
        "savefig.facecolor": SURFACE,
        "axes.edgecolor": GRID,
        "axes.labelcolor": TEXT_SECONDARY,
        "axes.titlecolor": TEXT_PRIMARY,
        "axes.titlesize": 11,
        "axes.titleweight": "bold",
        "axes.labelsize": 9,
        "axes.spines.top": False,
        "axes.spines.right": False,
        "axes.grid": True,
        "grid.color": GRID,
        "grid.linewidth": 0.8,
        "xtick.color": TEXT_SECONDARY,
        "ytick.color": TEXT_SECONDARY,
        "xtick.labelsize": 8,
        "ytick.labelsize": 8,
        "legend.frameon": False,
        "legend.fontsize": 8,
        "font.family": "DejaVu Sans",
        "svg.hashsalt": "ebr",
    }


def _fmt(v: Any, digits: int = 4) -> str:
    if v is None or (isinstance(v, float) and not np.isfinite(v)):
        return ""
    if isinstance(v, (bool, np.bool_)):
        return "yes" if v else "no"
    if isinstance(v, (int, np.integer)):
        return str(int(v))
    if isinstance(v, (float, np.floating)):
        if float(v).is_integer() and abs(v) < 1e15:
            return str(int(v))
        if 1e4 <= abs(v) < 1e15:
            return f"{float(v):.0f}"
        return f"{float(v):.{digits}g}"
    return str(v).replace("|", "\\|")


def markdown_table(df: pd.DataFrame, columns: Optional[Sequence[str]] = None) -> str:
    cols = list(columns or df.columns)
    lines = ["| " + " | ".join(cols) + " |", "|" + "|".join("---" for _ in cols) + "|"]
    for _, r in df.iterrows():
        vals = [None if (isinstance(r[c], float) and np.isnan(r[c])) else r[c] for c in cols]
        lines.append("| " + " | ".join(_fmt(v) for v in vals) + " |")
    return "\n".join(lines) + "\n"


def _md_header(lines: Sequence[str]) -> str:
    # CommonMark HTML block (type 2) ends only at "-->", so "--" in command lines is safe.
    body = "\n".join(line.replace("-->", "- ->") for line in lines)
    return f"<!--\n{body}\n-->\n"


def _save_png(fig, path: Path, command: str, source_csv: str) -> str:
    path.parent.mkdir(parents=True, exist_ok=True)
    fig.savefig(path, dpi=150, bbox_inches="tight",
                metadata={"Software": None, "Comment": f"generated-by: {command}", "Source": source_csv})
    plt.close(fig)
    return sha256_path(path)


def report(spec: Spec, layout: Layout, command: Optional[str] = None) -> Dict[str, str]:
    norm = layout.normalized_dir(spec.experiment_id)
    if not (norm / "summary.json").is_file():
        raise ReportError(f"{norm}/summary.json missing; run `python -m ebr normalize` first")
    command = command or command_line("report", layout, spec.path)
    tables_dir = layout.tables_dir(spec.experiment_id)
    fig_dir = layout.figures_dir(spec.experiment_id)
    clear_generated(tables_dir)
    clear_generated(fig_dir)
    written: Dict[str, str] = {}

    def src(name: str):
        p = norm / name
        _, df = read_csv(p)
        return p, df

    def hdr(sources: Sequence[Path]) -> List[str]:
        return provenance(layout, command, spec.experiment_id, [(layout.rel(p), sha256_path(p)) for p in sources])

    pc_path, pc = src("per_candidate.csv")
    pr_path, pr = src("paired_ratios.csv")
    pa_path, pa = src("pareto.csv")
    se_path, se = src("sensitivity.csv")
    s_path, samples = src("samples.csv")
    cand_order = [c.name for c in spec.candidates]
    metric_names = spec.analysis.report_metrics or [m.name for m in spec.metrics if m.kind == "numeric"]

    # ---- tables -----------------------------------------------------------------
    t = pc[pc.metric.isin(metric_names)]
    md = _md_header(hdr([pc_path])) + f"# {spec.experiment_id}: per-candidate summary\n\n"
    md += ("Unit of analysis per `analysis.pairing`; CI = percentile bootstrap of the median. "
           "Timing-family rows with decision_grade = no are informational only.\n\n")
    md += markdown_table(t, ["env_id", "metric", "candidate", "unit", "n_units", "median", "p90", "iqr", "mad", "ci_lower", "ci_upper", "decision_grade", "ci_status"])
    written["tables/per_candidate.md"] = write_text(tables_dir / "per_candidate.md", md)

    md = _md_header(hdr([pr_path])) + f"# {spec.experiment_id}: paired ratios vs baseline\n\n"
    md += "Ratio = candidate / baseline. Verdict applies the noise rule (CI must lie wholly outside the band).\n\n"
    md += markdown_table(pr[pr.metric.isin(metric_names)], ["env_id", "metric", "candidate", "baseline", "n_pairs", "median_ratio", "ci_lower", "ci_upper", "band_lower", "band_upper", "verdict"]) if len(pr) else "_No baseline configured._\n"
    written["tables/paired_ratios.md"] = write_text(tables_dir / "paired_ratios.md", md)

    md = _md_header(hdr([pa_path])) + f"# {spec.experiment_id}: Pareto frontier\n\n"
    md += markdown_table(pa) if len(pa) else "_No pareto_objectives configured._\n"
    written["tables/pareto.md"] = write_text(tables_dir / "pareto.md", md)

    md = _md_header(hdr([se_path])) + f"# {spec.experiment_id}: weight sensitivity\n\n"
    md += markdown_table(se) if len(se) else "_No analysis.sensitivity configured._\n"
    written["tables/sensitivity.md"] = write_text(tables_dir / "sensitivity.md", md)

    # ---- figures ------------------------------------------------------------------
    figure_names: List[str] = []
    with plt.rc_context(_style()):
        valid = samples[samples["valid"].astype(bool)]
        for metric in metric_names:
            m = spec.metric(metric)
            d = valid.dropna(subset=[metric])
            if d.empty:
                continue
            for env, g in d.groupby("env_id", sort=True):
                n_items = g["item_key"].nunique()
                if n_items > 1:
                    plot = g.groupby(["candidate", "item_key"], as_index=False)[metric].median()
                    plot["repetition"] = ""
                    what = "per-item medians"
                else:
                    plot = g[["candidate", "item_key", "repetition", metric]].copy()
                    what = "samples"
                plot = plot.rename(columns={metric: "value"})
                plot["_c"] = plot["candidate"].map({c: i for i, c in enumerate(cand_order)})
                plot = plot.sort_values(["_c", "item_key", "repetition"], kind="mergesort").drop(columns="_c")
                meds = plot.groupby("candidate")["value"].median()
                plot["candidate_median"] = plot["candidate"].map(meds)
                plot.insert(0, "env_id", env)
                plot.insert(1, "metric", metric)
                name = f"{metric}_by_candidate" + ("" if d["env_id"].nunique() == 1 else f"_{env}")
                csv_path = fig_dir / f"{name}.csv"
                written[f"figures/{name}.csv"] = write_csv(csv_path, plot.reset_index(drop=True), hdr([s_path]))
                cands = [c for c in cand_order if c in set(plot["candidate"])]
                fig, ax = plt.subplots(figsize=(6.4, 0.55 * len(cands) + 1.4))
                rng = np.random.Generator(np.random.PCG64(0))
                for i, c in enumerate(cands):
                    vals = plot[plot.candidate == c]["value"].to_numpy(float)
                    jitter = (rng.random(len(vals)) - 0.5) * 0.25
                    ax.scatter(vals, np.full(len(vals), i) + jitter, s=22, color=SERIES_1, alpha=0.85,
                               edgecolors=SURFACE, linewidths=0.8, zorder=3)
                    ax.plot([meds[c], meds[c]], [i - 0.3, i + 0.3], color=TEXT_PRIMARY, linewidth=2, zorder=4,
                            solid_capstyle="round")
                ax.set_yticks(range(len(cands)), cands)
                ax.set_ylim(-0.6, len(cands) - 0.4)
                ax.invert_yaxis()
                ax.grid(axis="y", visible=False)
                ax.set_xlabel(f"{metric}" + (f" ({m.unit})" if m.unit else ""))
                dg = "" if m.family not in TIMING_FAMILIES or bool(g["decision_grade_timing"].all()) else "\ninformational only: not decision-grade timing"
                ax.set_title(f"{spec.experiment_id}: {metric}\n{what}; bar = median{dg}", loc="left")
                written[f"figures/{name}.png"] = _save_png(fig, fig_dir / f"{name}.png", command, layout.rel(csv_path))
                figure_names.append(name)

        if len(pr):
            for metric in metric_names:
                g_all = pr[(pr.metric == metric) & pr.ci_lower.notna()]
                if g_all.empty:
                    continue
                for env, g in g_all.groupby("env_id", sort=True):
                    name = f"paired_ratio_{metric}" + ("" if pr["env_id"].nunique() == 1 else f"_{env}")
                    plot = g[["env_id", "metric", "candidate", "baseline", "n_pairs", "median_ratio", "ci_lower", "ci_upper", "band_lower", "band_upper", "verdict"]].reset_index(drop=True)
                    csv_path = fig_dir / f"{name}.csv"
                    written[f"figures/{name}.csv"] = write_csv(csv_path, plot, hdr([pr_path]))
                    fig, ax = plt.subplots(figsize=(6.4, 0.55 * len(plot) + 1.4))
                    if plot["band_lower"].notna().all():
                        ax.axvspan(float(plot["band_lower"].iloc[0]), float(plot["band_upper"].iloc[0]), facecolor=BAND_FILL,
                                   edgecolor=NEUTRAL_MARK, linewidth=0.6, zorder=1, label="practical-significance band")
                    ax.axvline(1.0, color=NEUTRAL_MARK, linewidth=1, zorder=2)
                    for i, r in plot.iterrows():
                        ax.plot([r.ci_lower, r.ci_upper], [i, i], color=SERIES_1, linewidth=2, solid_capstyle="round", zorder=3)
                        ax.scatter([r.median_ratio], [i], s=36, color=SERIES_1, edgecolors=SURFACE, linewidths=1.2, zorder=4)
                    ax.set_yticks(range(len(plot)), plot["candidate"])
                    ax.set_ylim(-0.6, len(plot) - 0.4)
                    ax.invert_yaxis()
                    ax.grid(axis="y", visible=False)
                    base = plot["baseline"].iloc[0]
                    ax.set_xlabel(f"{metric} ratio vs {base} (median, CI)")
                    note = "\ninformational only: not decision-grade timing" if plot["verdict"].astype(str).str.startswith("not_decision_grade").any() else ""
                    ax.set_title(f"{spec.experiment_id}: {metric} paired ratio vs {base}{note}", loc="left")
                    if plot["band_lower"].notna().all():
                        ax.legend(loc="lower right")
                    written[f"figures/{name}.png"] = _save_png(fig, fig_dir / f"{name}.png", command, layout.rel(csv_path))
                    figure_names.append(name)

        objectives = spec.analysis.pareto_objectives
        if len(pa) and len(objectives) >= 2:
            xo, yo = objectives[0], objectives[1]
            for env, g in pa.groupby("env_id", sort=True):
                name = f"pareto_{xo}_vs_{yo}" + ("" if pa["env_id"].nunique() == 1 else f"_{env}")
                plot = g[["env_id", "candidate", xo, yo, "on_frontier", "dominated_by", "epsilon"]].reset_index(drop=True)
                csv_path = fig_dir / f"{name}.csv"
                written[f"figures/{name}.csv"] = write_csv(csv_path, plot, hdr([pa_path]))
                fig, ax = plt.subplots(figsize=(6.4, 4.4))
                front = plot[plot.on_frontier.astype(bool)]
                dom = plot[~plot.on_frontier.astype(bool)]
                if len(dom):
                    ax.scatter(dom[xo], dom[yo], s=40, color=NEUTRAL_MARK, edgecolors=SURFACE, linewidths=1.5, zorder=3, label="dominated")
                ax.scatter(front[xo], front[yo], s=48, color=SERIES_1, edgecolors=SURFACE, linewidths=1.5, zorder=4, label="frontier")
                if len(plot) <= 12:
                    for _, r in plot.iterrows():
                        ax.annotate(r.candidate, (r[xo], r[yo]), textcoords="offset points", xytext=(6, 4), fontsize=8, color=TEXT_SECONDARY)
                ux, uy = spec.metric(xo).unit, spec.metric(yo).unit
                ax.set_xlabel(f"{xo}" + (f" ({ux})" if ux else "") + f" - {spec.metric(xo).direction}")
                ax.set_ylabel(f"{yo}" + (f" ({uy})" if uy else "") + f" - {spec.metric(yo).direction}")
                eps_note = str(plot["epsilon"].iloc[0]).replace("unset (strict Pareto, epsilon=0)", "unset/strict") if len(plot) else ""
                ax.set_title(f"{spec.experiment_id}: Pareto frontier (candidate medians)\nepsilon: {eps_note}", loc="left", fontsize=10)
                if len(dom):
                    ax.legend(loc="best")
                written[f"figures/{name}.png"] = _save_png(fig, fig_dir / f"{name}.png", command, layout.rel(csv_path))
                figure_names.append(name)

    idx = _md_header(provenance(layout, command, spec.experiment_id, [(layout.rel(norm / "summary.json"), sha256_path(norm / "summary.json"))]))
    idx += f"# {spec.experiment_id} generated report index\n\n"
    idx += f"Question: {spec.question}\n\nHypothesis: {spec.hypothesis}\n\n"
    idx += "Tables: [per candidate](per_candidate.md), [paired ratios](paired_ratios.md), [Pareto](pareto.md), [sensitivity](sensitivity.md)\n\n"
    idx += "Figures (PNG + plotted CSV):\n\n"
    for n in figure_names:
        rel = f"../../figures/{spec.experiment_id}/{n}"
        idx += f"- [{n}.png]({rel}.png) ([csv]({rel}.csv))\n"
    written["tables/index.md"] = write_text(tables_dir / "index.md", idx)
    return written
