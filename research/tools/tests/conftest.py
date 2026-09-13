import copy
import json
import os
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from ebr.paths import Layout  # noqa: E402
from ebr.spec import parse_spec  # noqa: E402
from ebr.thresholds import template  # noqa: E402

IS_LINUX = sys.platform.startswith("linux")
linux_only = pytest.mark.skipif(not IS_LINUX, reason="requires Linux (/usr/bin/time, bash)")


@pytest.fixture(autouse=True)
def _isolate_env(monkeypatch):
    for var in ("EB_REPO_ROOT", "EB_RESEARCH_ROOT", "EB_CORPUS_ROOT", "EB_HELDOUT_ROOT", "EB_SCRATCH_ROOT",
                "EB_HELDOUT_UNLOCK", "EB_ENV_ID", "EB_ENV_ALLOW_UNREGISTERED"):
        monkeypatch.delenv(var, raising=False)


@pytest.fixture
def layout(tmp_path) -> Layout:
    repo = tmp_path / "repo"
    for d in ("research/methods", "research/decisions", "research/environment", "research/raw"):
        (repo / d).mkdir(parents=True)
    (repo / "research/methods/thresholds.json").write_text(json.dumps(template(), indent=2))
    data = tmp_path / "data"
    (data / "corpus").mkdir(parents=True)
    (data / "heldout").mkdir(parents=True)
    return Layout(repo, data)


BASE_SPEC = {
    "schema": "ebr.spec.v1",
    "experiment_id": "EXP-TEST-001",
    "question": "q",
    "hypothesis": "h",
    "decision_ids": [],
    "candidates": [
        {"name": "copy", "params": {"n": 0}, "command": "cat {item_path} > {scratch}/out"},
        {"name": "head", "params": {"n": 1000}, "command": "head -c {n} {item_path} > {scratch}/out"},
    ],
    "corpus": {"generated": [{"item_id": "rnd-64k", "generator": "random-v1", "bytes": 65536, "seed": 7}]},
    "metrics": [
        {"name": "wall_s", "source": "resource", "family": "time_wall", "direction": "minimize", "unit": "s"},
        {"name": "max_rss_bytes", "source": "resource", "family": "memory_peak", "direction": "minimize"},
        {"name": "scratch_peak_bytes", "source": "scratch", "family": "scratch_peak", "direction": "minimize"},
        {"name": "output_bytes", "source": "path_size", "path": "{scratch}/out", "family": "size", "direction": "minimize"},
        {"name": "ratio", "source": "derived", "op": "ratio", "args": ["output_bytes", "input_bytes"], "family": "size", "direction": "minimize"},
        {"name": "nonempty", "source": "check", "command": "test -s {scratch}/out", "family": "correctness", "direction": "maximize"},
    ],
    "repetitions": 3,
    "warmups": 1,
    "seeds": {"order": 11, "bootstrap": 12},
    "timing": False,
    "platform": {"os": ["linux"], "poll_interval_s": 0.01, "timeout_s": 30},
    "analysis": {"baseline": "copy", "pairing": "item_repetition", "pareto_objectives": ["output_bytes", "max_rss_bytes"]},
}


@pytest.fixture
def spec_dict():
    return copy.deepcopy(BASE_SPEC)


def make_spec(d, tmp_path=None):
    if tmp_path is not None:
        import yaml

        p = Path(tmp_path) / "spec.yaml"
        p.write_text(yaml.safe_dump(d, sort_keys=False))
        from ebr.spec import load_spec

        return load_spec(p)
    return parse_spec(d)
