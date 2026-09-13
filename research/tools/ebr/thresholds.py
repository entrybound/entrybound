"""Machine-readable thresholds (research/methods/thresholds.json).

research/decision-method.md is authoritative for every value. The JSON file is a
mechanical transcription of it; ``null`` means "not yet set", and ebr refuses to
emit any verdict that depends on an unset value (it reports ``threshold_unset``
instead). ebr never supplies a numeric default for a decision threshold.

Specs without ``decision_ids`` (smoke / exploratory) may carry a
``thresholds_override`` block; outputs derived from it are labelled
``thresholds_source: spec-override (non-decision)``. Specs attached to decisions
must not override.
"""

from __future__ import annotations

import copy
import json
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Dict, Optional, Tuple

from . import THRESHOLDS_SCHEMA

#: Metric families. Every spec metric declares one; thresholds.json has one key each.
METRIC_FAMILIES: Dict[str, str] = {
    "time_wall": "Wall-clock elapsed time of a sample",
    "time_cpu": "User+system CPU time of a sample (process tree)",
    "memory_peak": "Peak resident set size (Linux) / peak working set (Windows)",
    "scratch_peak": "Peak bytes occupied in the per-sample scratch directory",
    "size": "Output size in bytes and size ratios",
    "throughput": "Derived rates such as input bytes per second",
    "io": "Filesystem block input/output counts",
    "correctness": "Pass/fail checks such as round-trip equality",
    "other": "Custom metrics not covered by another family",
}

#: Families whose values are timing measurements (only decision-grade under --timing).
TIMING_FAMILIES = frozenset({"time_wall", "time_cpu", "throughput"})

BAND_KINDS = ("ratio", "absolute")
EPSILON_KINDS = ("relative", "absolute")

#: Minimum bootstrap resamples required by the research plan (not a decision threshold).
MIN_BOOTSTRAP_RESAMPLES = 10000


class ThresholdUnset(LookupError):
    """A verdict needed a threshold that is null in thresholds.json."""


def template() -> Dict[str, Any]:
    fams = {}
    for name, desc in METRIC_FAMILIES.items():
        fams[name] = {
            "description": desc,
            "practical_significance_band": {"kind": None, "lower": None, "upper": None},
            "pareto_epsilon": {"kind": None, "value": None},
            "decision_method_section": None,
        }
    return {
        "schema": THRESHOLDS_SCHEMA,
        "status": "template",
        "authority": (
            "research/decision-method.md is authoritative. Every non-null value here must be "
            "transcribed from it and cite the section in decision_method_section. null = unset; "
            "ebr refuses verdicts that need an unset value and never substitutes a default."
        ),
        "semantics": {
            "practical_significance_band": (
                "kind 'ratio': candidate/baseline paired median ratios inside [lower, upper] are "
                "practically equivalent; kind 'absolute': differences (candidate - baseline, metric "
                "units) inside [lower, upper]. A difference is distinguishable only if the whole "
                "bootstrap CI lies outside the band."
            ),
            "pareto_epsilon": (
                "kind 'relative': fractional tolerance (a is not worse than b if a <= b*(1+value) "
                "for minimised metrics, compared in log space); kind 'absolute': metric units."
            ),
        },
        "families": fams,
        "bootstrap": {
            "confidence": None,
            "resamples": None,
            "decision_method_section": None,
            "note": "ebr enforces resamples >= 10000 regardless of this value.",
        },
        "quiet_machine_guard": {
            "max_loadavg_1m": None,
            "max_other_cpu_percent": None,
            "decision_method_section": None,
            "note": "Required before any timing=true run; ebr refuses timing runs while unset.",
        },
        "sensitivity": {
            "min_fraction_preserving_winner": None,
            "dirichlet_samples": None,
            "dirichlet_concentration": None,
            "decision_method_section": None,
        },
    }


def _deep_merge(base: Dict[str, Any], over: Dict[str, Any]) -> Dict[str, Any]:
    out = copy.deepcopy(base)
    for k, v in over.items():
        if isinstance(v, dict) and isinstance(out.get(k), dict):
            out[k] = _deep_merge(out[k], v)
        else:
            out[k] = copy.deepcopy(v)
    return out


@dataclass
class Thresholds:
    data: Dict[str, Any]
    path: Optional[str]
    source: str  # "thresholds.json", "thresholds.json+spec-override (non-decision)", "template (file missing)"
    warnings: list = field(default_factory=list)

    @classmethod
    def load(cls, path: Path, override: Optional[Dict[str, Any]] = None, decision_ids=()) -> "Thresholds":
        warnings = []
        if path.is_file():
            with open(path, encoding="utf-8") as fh:
                data = json.load(fh)
            if data.get("schema") != THRESHOLDS_SCHEMA:
                raise ValueError(f"{path}: schema must be {THRESHOLDS_SCHEMA}")
            source = "thresholds.json"
        else:
            data = template()
            source = "template (thresholds.json missing)"
            warnings.append(f"{path} missing; all thresholds unset")
        validate(data)
        if override:
            if decision_ids:
                raise ValueError(
                    "thresholds_override is only allowed in specs without decision_ids; "
                    "decision experiments must use research/methods/thresholds.json"
                )
            data = _deep_merge(data, override)
            validate(data)
            source = source + "+spec-override (non-decision)"
        return cls(data, str(path), source, warnings)

    def band(self, family: str) -> Tuple[str, float, float]:
        b = self.data["families"][family]["practical_significance_band"]
        if b.get("kind") is None or b.get("lower") is None or b.get("upper") is None:
            raise ThresholdUnset(f"families.{family}.practical_significance_band")
        return b["kind"], float(b["lower"]), float(b["upper"])

    def epsilon(self, family: str) -> Tuple[str, float]:
        e = self.data["families"][family]["pareto_epsilon"]
        if e.get("kind") is None or e.get("value") is None:
            raise ThresholdUnset(f"families.{family}.pareto_epsilon")
        return e["kind"], float(e["value"])

    def bootstrap_confidence(self) -> float:
        c = self.data.get("bootstrap", {}).get("confidence")
        if c is None:
            raise ThresholdUnset("bootstrap.confidence")
        return float(c)

    def bootstrap_resamples(self) -> int:
        r = self.data.get("bootstrap", {}).get("resamples")
        if r is None:
            return MIN_BOOTSTRAP_RESAMPLES
        return max(int(r), MIN_BOOTSTRAP_RESAMPLES)

    def guard(self) -> Dict[str, Optional[float]]:
        g = self.data.get("quiet_machine_guard", {})
        return {"max_loadavg_1m": g.get("max_loadavg_1m"), "max_other_cpu_percent": g.get("max_other_cpu_percent")}

    def sensitivity(self) -> Dict[str, Any]:
        return dict(self.data.get("sensitivity", {}))


def validate(data: Dict[str, Any]) -> None:
    fams = data.get("families")
    if not isinstance(fams, dict):
        raise ValueError("thresholds: 'families' must be an object")
    missing = set(METRIC_FAMILIES) - set(fams)
    if missing:
        raise ValueError(f"thresholds: missing families {sorted(missing)}")
    unknown = set(fams) - set(METRIC_FAMILIES)
    if unknown:
        raise ValueError(f"thresholds: unknown families {sorted(unknown)}")
    for name, f in fams.items():
        b = f.get("practical_significance_band", {})
        if b.get("kind") not in (None,) + BAND_KINDS:
            raise ValueError(f"thresholds: families.{name}.practical_significance_band.kind invalid")
        if b.get("lower") is not None and b.get("upper") is not None and float(b["lower"]) > float(b["upper"]):
            raise ValueError(f"thresholds: families.{name} band lower > upper")
        e = f.get("pareto_epsilon", {})
        if e.get("kind") not in (None,) + EPSILON_KINDS:
            raise ValueError(f"thresholds: families.{name}.pareto_epsilon.kind invalid")
        if e.get("value") is not None and float(e["value"]) < 0:
            raise ValueError(f"thresholds: families.{name}.pareto_epsilon.value negative")
    conf = data.get("bootstrap", {}).get("confidence")
    if conf is not None and not (0.0 < float(conf) < 1.0):
        raise ValueError("thresholds: bootstrap.confidence must be in (0,1)")
