"""Import research/corpus/tools/fingerprint.py by path, regardless of cwd.

Reused so that baseline round-trip verification uses the exact same
content_tree_sha256 / logical_tree_sha256 definitions as research/corpus's own
pinned manifest fingerprints (research/corpus/manifest.json), rather than a
second, possibly-diverging implementation.
"""
from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

_MODULE = None


def _research_dir() -> Path:
    # research/baselines/probes/fpshim.py -> research
    return Path(__file__).resolve().parents[2]


def fingerprint_module():
    global _MODULE
    if _MODULE is None:
        src = _research_dir() / "corpus" / "tools" / "fingerprint.py"
        spec = importlib.util.spec_from_file_location("baseline_probes._fingerprint", src)
        mod = importlib.util.module_from_spec(spec)
        sys.modules[spec.name] = mod
        spec.loader.exec_module(mod)
        _MODULE = mod
    return _MODULE


def fingerprint_path(path, jobs: int = 4):
    return fingerprint_module().fingerprint_path(path, jobs=jobs)
