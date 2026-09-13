"""Windows measurement path (job object + psutil peak working set). Skipped elsewhere."""

import sys

import pytest

from ebr.rawio import read_rows, verify_run
from ebr.run import RunOptions, run_experiment
from ebr.spec import parse_spec

pytestmark = pytest.mark.skipif(not sys.platform.startswith("win"), reason="Windows only")

ALLOC = "import sys,time; b=bytearray(64*1024*1024); b[::4096]=b'x'*len(b[::4096]); open(sys.argv[1],'wb').write(b'y'*300000); time.sleep(0.3)"


def _spec(extra=None):
    d = {
        "experiment_id": "EXP-WIN-001",
        "question": "q",
        "hypothesis": "h",
        "candidates": [
            {"name": "alloc", "command": [sys.executable, "-c", ALLOC, "{scratch}/f"]},
            {"name": "shell", "command": "echo hello> {scratch}\\f"},
        ],
        "shell": "cmd",
        "corpus": {"generated": [{"item_id": "z", "generator": "zeros-v1", "bytes": 10}]},
        "metrics": [
            {"name": "max_rss_bytes", "source": "resource", "family": "memory_peak", "direction": "minimize"},
            {"name": "cpu_s", "source": "resource", "family": "time_cpu", "direction": "minimize"},
            {"name": "scratch_peak_bytes", "source": "scratch", "family": "scratch_peak", "direction": "minimize"},
        ],
        "repetitions": 1,
        "warmups": 0,
        "seeds": 1,
        "timing": False,
        "platform": {"os": ["windows"], "timeout_s": 30, "poll_interval_s": 0.02},
    }
    d.update(extra or {})
    return parse_spec(d)


def test_windows_job_measurement(layout, tmp_path):
    s = _spec()
    out = run_experiment(s, layout, RunOptions(quiet=True, scratch_root=str(tmp_path / "s"), cpus=[0]))
    rows = {r["candidate"]: r for r in read_rows(out.raw_file)}
    a = rows["alloc"]
    assert a["exit_code"] == 0, a["stderr_excerpt"]
    assert a["measurement"]["method"] == "win-job+psutil-peak-wset"
    assert a["metrics"]["max_rss_bytes"] >= 64 * 1024 * 1024
    assert a["metrics"]["scratch_peak_bytes"] >= 300000
    assert a["measurement"]["job"]["total_processes"] >= 1
    assert a["measurement"]["job"]["io_write_bytes"] >= 300000
    assert a["cpu_affinity"] == [0]
    assert rows["shell"]["exit_code"] == 0
    assert verify_run(layout.raw_dir(s.experiment_id), out.run_id).ok


def test_windows_timeout_kills_job(layout, tmp_path):
    s = _spec({"candidates": [{"name": "slow", "command": [sys.executable, "-c", "import time; time.sleep(20)"]}],
               "platform": {"os": ["windows"], "timeout_s": 1.0}})
    out = run_experiment(s, layout, RunOptions(quiet=True, scratch_root=str(tmp_path / "s")))
    row = next(read_rows(out.raw_file))
    assert row["timed_out"] and not row["ok"]
    assert row["resource"]["wall_s"] < 10
