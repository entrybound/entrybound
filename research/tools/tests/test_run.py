import gzip
import json
from collections import Counter

import pytest

from conftest import linux_only, make_spec
from ebr.corpus import Item
from ebr.rawio import list_runs, load_meta, read_rows, verify_run
from ebr.run import RunOptions, RunRefused, build_plan, run_experiment
from ebr.spec import HeldoutLocked, SpecError, parse_spec


def _items(n):
    from pathlib import Path

    return [Item(f"i{k}", "dev", None, Path(f"/nonexistent/i{k}")) for k in range(n)]


def test_plan_balanced_interleaved_and_seeded(spec_dict):
    spec_dict["candidates"].append({"name": "third", "command": "true"})
    spec_dict["repetitions"] = 4
    spec_dict["warmups"] = 2
    s = parse_spec(spec_dict)
    items = _items(3)
    plan = build_plan(s, items)
    assert len(plan) == (4 + 2) * 3 * 3
    assert [e.phase for e in plan[:18]] == ["warmup"] * 18
    for rnd in range(6):
        pairs = Counter((e.candidate, e.item_key) for e in plan if e.round == rnd)
        assert len(pairs) == 9 and set(pairs.values()) == {1}
    # randomized_blocks keeps each item's candidates contiguous within a round
    for rnd in range(6):
        keys = [e.item_key for e in plan if e.round == rnd]
        blocks = [keys[i:i + 3] for i in range(0, 9, 3)]
        assert all(len(set(b)) == 1 for b in blocks)
    assert [e.__dict__ for e in build_plan(s, items)] == [e.__dict__ for e in plan]
    spec_dict["seeds"]["order"] = 99
    other = build_plan(parse_spec(spec_dict), items)
    assert [(e.candidate, e.item_key) for e in other] != [(e.candidate, e.item_key) for e in plan]
    # candidate order within an item block varies across rounds (drift decorrelation)
    orders = {tuple(e.candidate for e in plan if e.round == r and e.item_key == "dev/i0") for r in range(6)}
    assert len(orders) > 1
    seqs = [e.seq for e in plan]
    assert seqs == list(range(len(plan)))


def test_plan_without_recorded_warmups(spec_dict):
    spec_dict["record_warmups"] = False
    s = parse_spec(spec_dict)
    plan = build_plan(s, _items(1))
    assert [e.seq for e in plan if e.phase == "warmup"] == [None, None]
    assert [e.seq for e in plan if e.phase == "measure"] == list(range(6))


def _opts(**kw):
    kw.setdefault("quiet", True)
    return RunOptions(**kw)


@linux_only
def test_end_to_end_run_and_verify(spec_dict, layout, tmp_path):
    s = make_spec(spec_dict, tmp_path)
    out = run_experiment(s, layout, _opts(scratch_root=str(tmp_path / "scratch")))
    assert out.status == "complete" and out.recorded == 8 and out.failed == 0
    raw_dir = layout.raw_dir("EXP-TEST-001")
    assert list_runs(raw_dir) == [out.run_id]
    rows = list(read_rows(out.raw_file))
    assert len(rows) == 8
    by = {(r["candidate"], r["phase"], r["repetition"]): r for r in rows}
    copy = by[("copy", "measure", 0)]
    head = by[("head", "measure", 0)]
    assert copy["metrics"]["output_bytes"] == 65536 and head["metrics"]["output_bytes"] == 1000
    assert copy["metrics"]["ratio"] == 1.0
    assert copy["metrics"]["nonempty"] == 1
    assert copy["metrics"]["max_rss_bytes"] > 0
    assert copy["metrics"]["scratch_peak_bytes"] >= 65536
    assert copy["measurement"]["method"] == "gnu-time-v+wait4"
    assert copy["measurement"]["timev"]["exit_status"] == 0
    assert copy["exit_code"] == 0 and copy["valid"] and copy["ok"]
    assert copy["timing"]["decision_grade"] is False
    assert len(copy["stdout_sha256"]) == 64
    assert copy["item_fingerprint"]["bytes"] == 65536
    assert copy["env_id"].startswith("unregistered-")
    rep = verify_run(raw_dir, out.run_id)
    assert rep.ok, rep.errors
    assert any("not registered" in w for w in rep.warnings)
    # no scratch left behind
    assert not any((tmp_path / "scratch").rglob("out"))

    # tamper with one metric value -> row digest mismatch and file hash mismatch
    text = out.raw_file.read_text()
    out.raw_file.write_text(text.replace('"output_bytes":1000', '"output_bytes":999', 1))
    rep2 = verify_run(raw_dir, out.run_id)
    assert not rep2.ok
    assert any("row_sha256" in e for e in rep2.errors)
    assert any("raw sha256 mismatch" in e for e in rep2.errors)
    # dropping a row breaks the chain / plan count
    lines = text.splitlines(keepends=True)
    out.raw_file.write_text("".join(lines[:3] + lines[4:]))
    rep3 = verify_run(raw_dir, out.run_id)
    assert any("hash chain" in e for e in rep3.errors)


@linux_only
def test_gzip_raw_and_failures(spec_dict, layout, tmp_path):
    spec_dict["candidates"][1]["command"] = "exit 7"
    spec_dict["warmups"] = 0
    spec_dict["repetitions"] = 1
    s = parse_spec(spec_dict)
    out = run_experiment(s, layout, _opts(compression="gzip", scratch_root=str(tmp_path / "s")))
    assert out.raw_file.name.endswith(".jsonl.gz")
    with gzip.open(out.raw_file, "rt") as fh:
        rows = [json.loads(l) for l in fh]
    bad = [r for r in rows if r["candidate"] == "head"][0]
    assert bad["exit_code"] == 7 and not bad["ok"] and not bad["valid"]
    assert "command_or_check_failed" in bad["invalid_reasons"]
    assert out.failed == 1
    assert verify_run(layout.raw_dir(s.experiment_id), out.run_id).ok


@linux_only
def test_scratch_peak_timeout_stdout_json_and_env(spec_dict, layout, tmp_path, monkeypatch):
    monkeypatch.setenv("EB_HELDOUT_UNLOCK", "deadbeef")
    spec_dict["candidates"] = [
        {"name": "peak", "command": "head -c 300000 /dev/zero > {scratch}/big && sleep 0.4 && rm {scratch}/big && echo '{{\"a\": {{\"b\": 5}}}}' && touch {scratch}/out && test -z \"${{EB_HELDOUT_UNLOCK:-}}\""},
        {"name": "slow", "command": "sleep 5; touch {scratch}/out"},
    ]
    spec_dict["metrics"] = [
        {"name": "scratch_peak_bytes", "source": "scratch", "family": "scratch_peak", "direction": "minimize"},
        {"name": "scratch_final_bytes", "source": "scratch", "family": "scratch_peak"},
        {"name": "b", "source": "stdout_json", "key": "a.b", "family": "other"},
    ]
    spec_dict["analysis"] = {}
    spec_dict["warmups"] = 0
    spec_dict["repetitions"] = 1
    spec_dict["platform"]["timeout_s"] = 1.0
    s = parse_spec(spec_dict)
    out = run_experiment(s, layout, _opts(scratch_root=str(tmp_path / "s")))
    rows = {r["candidate"]: r for r in read_rows(out.raw_file)}
    peak = rows["peak"]
    assert peak["exit_code"] == 0, peak["stderr_excerpt"]
    assert peak["metrics"]["scratch_peak_bytes"] >= 300000
    assert peak["metrics"]["scratch_final_bytes"] == 0
    assert peak["metrics"]["b"] == 5
    slow = rows["slow"]
    assert slow["timed_out"] and slow["exit_code"] is None and "timeout" in slow["invalid_reasons"]
    assert slow["resource"]["wall_s"] < 4


@linux_only
def test_cpu_affinity_uses_taskset(spec_dict, layout, tmp_path):
    spec_dict["warmups"] = 0
    spec_dict["repetitions"] = 1
    s = parse_spec(spec_dict)
    out = run_experiment(s, layout, _opts(cpus=[0], scratch_root=str(tmp_path / "s")))
    row = next(read_rows(out.raw_file))
    assert row["measurement"]["argv"][0].endswith("taskset")
    assert row["cpu_affinity"] == [0]
    with pytest.raises(RunRefused, match="out of range"):
        run_experiment(s, layout, _opts(cpus=[100000]))


def test_timing_flags_and_guard_refusals(spec_dict, layout, tmp_path):
    spec_dict["timing"] = True
    s = parse_spec(spec_dict)
    spec_dict["timing"] = False
    s_untimed = parse_spec(spec_dict)
    if s.platform.os != ["linux"] or not __import__("sys").platform.startswith("linux"):
        pytest.skip("linux spec")
    with pytest.raises(RunRefused, match="--timing"):
        run_experiment(s, layout, _opts())
    with pytest.raises(SpecError, match="timing: false"):
        run_experiment(s_untimed, layout, _opts(timing=True))
    with pytest.raises(RunRefused, match="guard limits unset"):
        run_experiment(s, layout, _opts(timing=True))
    spec_dict["timing"] = True
    spec_dict["guard"] = {"sample_interval_s": 0.05}
    s2 = parse_spec(spec_dict)
    with pytest.raises(RunRefused, match="not quiet"):
        run_experiment(s2, layout, _opts(timing=True, max_loadavg_1m=-1.0, max_other_cpu_percent=100.0))
    assert list_runs(layout.raw_dir(s.experiment_id)) == []  # refused runs write nothing


@linux_only
def test_timing_run_with_permissive_guard_is_decision_grade(spec_dict, layout, tmp_path):
    spec_dict["timing"] = True
    spec_dict["guard"] = {"sample_interval_s": 0.05}
    spec_dict["warmups"] = 0
    spec_dict["repetitions"] = 1
    s = parse_spec(spec_dict)
    out = run_experiment(s, layout, _opts(timing=True, max_loadavg_1m=1e9, max_other_cpu_percent=100.0, scratch_root=str(tmp_path / "s")))
    rows = list(read_rows(out.raw_file))
    assert all(r["timing"]["decision_grade"] for r in rows)
    assert all(r["timing"]["guard_pre"]["ok"] and r["timing"]["guard_post"]["ok"] for r in rows)
    meta = load_meta(layout.raw_dir(s.experiment_id), out.run_id)
    assert meta["timing"]["guard"]["sources"] == {"max_loadavg_1m": "cli", "max_other_cpu_percent": "cli"}
    assert verify_run(layout.raw_dir(s.experiment_id), out.run_id).ok


def test_heldout_run_refused(spec_dict, layout):
    spec_dict["corpus"] = {"splits": ["heldout"]}
    s = parse_spec(spec_dict)
    if not __import__("sys").platform.startswith("linux"):
        pytest.skip("linux spec")
    with pytest.raises(HeldoutLocked):
        run_experiment(s, layout, _opts())


def test_hardcoded_heldout_path_refused(spec_dict, layout):
    if not __import__("sys").platform.startswith("linux"):
        pytest.skip("linux spec")
    spec_dict["candidates"][0]["command"] = f"cat {layout.heldout_root.as_posix()}/x > {{scratch}}/out"
    with pytest.raises(HeldoutLocked, match="references the held-out root"):
        run_experiment(parse_spec(spec_dict), layout, _opts())


def test_platform_mismatch_refused(spec_dict, layout):
    spec_dict["platform"]["os"] = ["darwin"]
    with pytest.raises(RunRefused, match="requires os"):
        run_experiment(parse_spec(spec_dict), layout, _opts())


def test_scratch_inside_repo_refused(spec_dict, layout):
    import sys

    if not sys.platform.startswith("linux"):
        pytest.skip("linux spec")
    with pytest.raises(RunRefused, match="inside the repository"):
        run_experiment(parse_spec(spec_dict), layout, _opts(scratch_root=str(layout.repo_root / "scratch")))
