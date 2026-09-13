import hashlib
import json

import pytest

from conftest import linux_only, make_spec
from ebr.normalize import NormalizeError, normalize
from ebr.outputs import read_csv
from ebr.report import report
from ebr.run import RunOptions, run_experiment


def _tree_hashes(*roots):
    out = {}
    for root in roots:
        for p in sorted(root.rglob("*")):
            if p.is_file():
                out[str(p)] = hashlib.sha256(p.read_bytes()).hexdigest()
    return out


@pytest.fixture
def completed(spec_dict, layout, tmp_path):
    if not __import__("sys").platform.startswith("linux"):
        pytest.skip("linux")
    spec_dict["analysis"]["report_metrics"] = ["output_bytes", "max_rss_bytes", "wall_s"]
    spec_dict["analysis"]["sensitivity"] = {"weights": {"output_bytes": 0.6, "max_rss_bytes": 0.4}, "dirichlet_samples": 200}
    spec_dict["metrics"].append({"name": "out_sha", "source": "path_sha256", "path": "{scratch}/out", "family": "correctness"})
    s = make_spec(spec_dict, tmp_path)
    out = run_experiment(s, layout, RunOptions(quiet=True, scratch_root=str(tmp_path / "scratch")))
    return s, out


@linux_only
def test_normalize_without_thresholds_reports_unset(completed, layout):
    s, out = completed
    summary = normalize(s, layout, command="python -m ebr normalize --spec spec.yaml")
    nd = layout.normalized_dir(s.experiment_id)
    for name in ("samples.csv", "failures.csv", "per_item.csv", "per_candidate.csv", "paired_ratios.csv", "pareto.csv", "sensitivity.csv", "summary.json"):
        assert (nd / name).is_file(), name
    header, samples = read_csv(nd / "samples.csv")
    assert header[0] == "generated-by: python -m ebr normalize --spec spec.yaml"
    assert any(h.startswith("input: ") and out.raw_file.name in h for h in header)
    assert len(samples) == 6 and set(samples["phase"]) == {"measure"}
    assert summary["counts"] == {"rows": 8, "warmup_rows_excluded": 2, "measured": 6, "valid": 6, "invalid": 0, "failed": 0}
    _, pr = read_csv(nd / "paired_ratios.csv")
    ob = pr[pr.metric == "output_bytes"].iloc[0]
    assert ob.candidate == "head" and ob.median_ratio == pytest.approx(1000 / 65536)
    assert ob.verdict == "threshold_unset:bootstrap.confidence"
    _, pc = read_csv(nd / "per_candidate.csv")
    assert pc[pc.metric == "output_bytes"]["ci_status"].iloc[0] == "threshold_unset:bootstrap.confidence"
    assert "bootstrap.confidence unset: confidence intervals and verdicts omitted" in summary["warnings"]
    assert summary["string_metrics"]["out_sha"]
    assert all(len(v) == 1 for v in summary["string_metrics"]["out_sha"].values())
    _, pa = read_csv(nd / "pareto.csv")
    assert set(pa["candidate"]) == {"copy", "head"}
    assert "unset (strict Pareto" in pa["epsilon"].iloc[0]


@linux_only
def test_normalize_with_override_and_report_deterministic(completed, layout, spec_dict, tmp_path):
    s, out = completed
    raw = dict(s.raw)
    raw["thresholds_override"] = {
        "bootstrap": {"confidence": 0.9},
        "families": {
            "size": {"practical_significance_band": {"kind": "ratio", "lower": 0.99, "upper": 1.01}},
            "time_wall": {"practical_significance_band": {"kind": "ratio", "lower": 0.99, "upper": 1.01}},
        },
    }
    from ebr.spec import parse_spec

    s2 = parse_spec(raw, path=s.path, sha256=s.sha256)
    summary = normalize(s2, layout, command="python -m ebr normalize --spec spec.yaml")
    verdicts = {(v["candidate"], v["metric"]): v["verdict"] for v in summary["paired_ratio_verdicts"]}
    assert verdicts[("head", "output_bytes")] == "distinguishable_below"
    assert verdicts[("head", "wall_s")].startswith("not_decision_grade(informational:")
    assert summary["thresholds_source"].endswith("spec-override (non-decision)")
    assert summary["pareto_frontier"]
    assert summary["sensitivity"][0]["base_winner"] == "head"

    written = report(s2, layout, command="python -m ebr report --spec spec.yaml")
    figs = layout.figures_dir(s.experiment_id)
    tables = layout.tables_dir(s.experiment_id)
    assert (figs / "output_bytes_by_candidate.png").is_file()
    assert (figs / "output_bytes_by_candidate.csv").is_file()
    assert (figs / "paired_ratio_output_bytes.png").is_file()
    (figs / "stale.png").write_bytes(b"old")
    report(s2, layout, command="python -m ebr report --spec spec.yaml")
    assert not (figs / "stale.png").exists()
    assert (figs / "pareto_output_bytes_vs_max_rss_bytes.png").is_file()
    png = (figs / "output_bytes_by_candidate.png").read_bytes()
    assert png.startswith(b"\x89PNG") and b"generated-by: python -m ebr report --spec spec.yaml" in png
    header, fig_csv = read_csv(figs / "output_bytes_by_candidate.csv")
    assert header[0].startswith("generated-by: python -m ebr report")
    assert set(fig_csv["candidate"]) == {"copy", "head"}
    md = (tables / "paired_ratios.md").read_text()
    assert md.startswith("<!--\ngenerated-by: python -m ebr report --spec spec.yaml")
    assert "distinguishable_below" in md
    assert "figures/output_bytes_by_candidate.png" in written

    before = _tree_hashes(layout.normalized_dir(s.experiment_id), figs, tables)
    normalize(s2, layout, command="python -m ebr normalize --spec spec.yaml")
    report(s2, layout, command="python -m ebr report --spec spec.yaml")
    after = _tree_hashes(layout.normalized_dir(s.experiment_id), figs, tables)
    assert before == after


@linux_only
def test_multi_item_pairing_by_item(spec_dict, layout, tmp_path):
    spec_dict["corpus"] = {"generated": [
        {"item_id": "r1", "generator": "random-v1", "bytes": 20000, "seed": 1, "family": "random"},
        {"item_id": "t1", "generator": "text-v1", "bytes": 30000, "seed": 2, "family": "text"},
        {"item_id": "z1", "generator": "zeros-v1", "bytes": 40000, "seed": 0, "family": "zeros"},
    ]}
    spec_dict["repetitions"] = 2
    spec_dict["warmups"] = 0
    spec_dict["analysis"]["pairing"] = "item"
    spec_dict["thresholds_override"] = {"bootstrap": {"confidence": 0.9}}
    s = make_spec(spec_dict, tmp_path)
    run_experiment(s, layout, RunOptions(quiet=True, scratch_root=str(tmp_path / "scratch")))
    summary = normalize(s, layout)
    nd = layout.normalized_dir(s.experiment_id)
    _, pc = read_csv(nd / "per_candidate.csv")
    ob = pc[(pc.metric == "output_bytes") & (pc.candidate == "copy")].iloc[0]
    assert ob.unit_of_analysis == "item_median" and ob.n_units == 3 and ob.n_items == 3
    assert ob["median"] == 30000 and ob.ci_status.startswith("ok:")
    _, pi = read_csv(nd / "per_item.csv")
    assert len(pi[(pi.metric == "output_bytes")]) == 6 and set(pi["n"]) == {2}
    _, pr = read_csv(nd / "paired_ratios.csv")
    r = pr[(pr.metric == "output_bytes")].iloc[0]
    assert r.n_pairs == 3 and r.verdict == "threshold_unset:families.size.practical_significance_band"
    written = report(s, layout)
    _, fig = read_csv(layout.figures_dir(s.experiment_id) / "output_bytes_by_candidate.csv")
    assert len(fig) == 6  # per-item medians, not samples
    assert summary["counts"]["measured"] == 12


@linux_only
def test_normalize_refuses_tampered_raw(completed, layout):
    s, out = completed
    data = out.raw_file.read_bytes()
    out.raw_file.write_bytes(data.replace(b'"exit_code":0', b'"exit_code":1', 1))
    with pytest.raises(NormalizeError, match="verify-raw failed"):
        normalize(s, layout)


@linux_only
def test_cli_end_to_end(completed, layout, capsys):
    from ebr.cli import main

    s, out = completed
    args = ["--repo-root", str(layout.repo_root), "--data-root", str(layout.data_root)]
    assert main(args + ["verify-raw", "--spec", s.path]) == 0
    assert json.loads(capsys.readouterr().out.splitlines()[0])["ok"] is True
    assert main(args + ["normalize", "--spec", s.path]) == 0
    capsys.readouterr()
    assert main(args + ["report", "--spec", s.path]) == 0
    capsys.readouterr()
    header, _ = read_csv(layout.normalized_dir(s.experiment_id) / "per_candidate.csv")
    assert header[0].startswith("generated-by: python -m ebr normalize --spec ")
    assert main(args + ["validate", "--spec", s.path]) == 0
    bad = layout.repo_root / "bad.yaml"
    bad.write_text("experiment_id: x\n")
    assert main(args + ["validate", "--spec", str(bad)]) == 2
