"""Statistics for ebr: robust descriptives, bootstrap CIs, noise rule, Pareto, sensitivity.

Conventions
-----------
* Percentiles use numpy ``method="linear"`` (Hyndman-Fan type 7).
* ``mad`` is the raw median absolute deviation (no 1.4826 scaling) unless ``scale=True``.
* Bootstrap: percentile intervals from ``Generator(PCG64(seed))`` index resampling,
  at least 10000 resamples (enforced), computed in chunks. Same seed + data =>
  identical interval.
* Ratios are candidate / baseline. "Paired" means resampling whole pairs.
* No decision threshold has a default here: bands, epsilons and confidence levels
  are arguments (sourced from research/methods/thresholds.json by callers).
"""

from __future__ import annotations

import itertools
import math
from dataclasses import asdict, dataclass
from typing import Callable, Dict, List, Mapping, Optional, Sequence, Tuple

import numpy as np
from scipy import stats as _sps

from .thresholds import MIN_BOOTSTRAP_RESAMPLES

PERCENTILE_METHOD = "linear"


def _arr(x: Sequence[float]) -> np.ndarray:
    a = np.asarray(x, dtype=float)
    if a.ndim != 1:
        raise ValueError("expected a 1-D sequence")
    if a.size == 0:
        raise ValueError("empty sample")
    if not np.all(np.isfinite(a)):
        raise ValueError("sample contains NaN/inf")
    return a


# ----------------------------------------------------------------------------
# descriptives


def median(x: Sequence[float]) -> float:
    return float(np.median(_arr(x)))


def percentile(x: Sequence[float], q: float) -> float:
    return float(np.percentile(_arr(x), q, method=PERCENTILE_METHOD))


def p90(x: Sequence[float]) -> float:
    return percentile(x, 90)


def p95(x: Sequence[float]) -> float:
    return percentile(x, 95)


def iqr(x: Sequence[float]) -> float:
    a = _arr(x)
    q75, q25 = np.percentile(a, [75, 25], method=PERCENTILE_METHOD)
    return float(q75 - q25)


def mad(x: Sequence[float], scale: bool = False) -> float:
    a = _arr(x)
    m = np.median(np.abs(a - np.median(a)))
    return float(m * 1.4826 if scale else m)


def geometric_mean(x: Sequence[float]) -> float:
    a = _arr(x)
    if np.any(a <= 0):
        raise ValueError("geometric mean requires strictly positive values")
    return float(np.exp(np.mean(np.log(a))))


def describe(x: Sequence[float]) -> Dict[str, float]:
    a = _arr(x)
    out = {
        "n": int(a.size),
        "median": median(a),
        "p90": p90(a),
        "p95": p95(a),
        "iqr": iqr(a),
        "mad": mad(a),
        "min": float(a.min()),
        "max": float(a.max()),
        "mean": float(a.mean()),
    }
    out["geomean"] = geometric_mean(a) if np.all(a > 0) else None
    return out


# ----------------------------------------------------------------------------
# bootstrap


@dataclass(frozen=True)
class CI:
    estimate: float
    lower: float
    upper: float
    confidence: float
    resamples: int
    seed: int
    method: str
    n: int

    def to_dict(self) -> Dict[str, float]:
        return asdict(self)


def _check_boot(confidence: float, resamples: int) -> None:
    if not (0.0 < confidence < 1.0):
        raise ValueError("confidence must be in (0, 1)")
    if resamples < MIN_BOOTSTRAP_RESAMPLES:
        raise ValueError(f"resamples must be >= {MIN_BOOTSTRAP_RESAMPLES}")


def _boot_stat(n: int, stat: Callable[[np.ndarray], np.ndarray], resamples: int, seed: int, chunk: int = 2000) -> np.ndarray:
    """Apply ``stat`` to index matrices of shape (chunk, n); returns all bootstrap replicates."""
    rng = np.random.Generator(np.random.PCG64(seed))
    out = np.empty(resamples, dtype=float)
    done = 0
    while done < resamples:
        k = min(chunk, resamples - done)
        idx = rng.integers(0, n, size=(k, n))
        out[done : done + k] = stat(idx)
        done += k
    return out


def _interval(reps: np.ndarray, confidence: float) -> Tuple[float, float]:
    alpha = (1.0 - confidence) / 2.0
    lo, hi = np.quantile(reps, [alpha, 1.0 - alpha], method=PERCENTILE_METHOD)
    return float(lo), float(hi)


def bootstrap_median_ci(x: Sequence[float], *, confidence: float, seed: int, resamples: int = MIN_BOOTSTRAP_RESAMPLES) -> CI:
    _check_boot(confidence, resamples)
    a = _arr(x)
    if a.size < 2:
        raise ValueError("bootstrap CI needs n >= 2")
    reps = _boot_stat(a.size, lambda idx: np.median(a[idx], axis=1), resamples, seed)
    lo, hi = _interval(reps, confidence)
    return CI(float(np.median(a)), lo, hi, confidence, resamples, seed, "percentile-bootstrap:median", int(a.size))


def bootstrap_paired_ratio_ci(
    candidate: Sequence[float],
    baseline: Sequence[float],
    *,
    confidence: float,
    seed: int,
    resamples: int = MIN_BOOTSTRAP_RESAMPLES,
    statistic: str = "median_of_ratios",
) -> CI:
    """CI for a paired ratio candidate/baseline, resampling pairs.

    ``median_of_ratios``: median_i(c_i / b_i).  ``ratio_of_medians``: median(c)/median(b)
    with pairs resampled jointly.  ``geomean_of_ratios``: exp(mean(log(c_i/b_i))).
    """
    _check_boot(confidence, resamples)
    c = _arr(candidate)
    b = _arr(baseline)
    if c.size != b.size:
        raise ValueError("paired samples must have equal length")
    if c.size < 2:
        raise ValueError("bootstrap CI needs n >= 2 pairs")
    if np.any(b <= 0) or np.any(c < 0):
        raise ValueError("ratios require baseline > 0 and candidate >= 0")
    if statistic == "median_of_ratios":
        r = c / b
        est = float(np.median(r))
        reps = _boot_stat(c.size, lambda idx: np.median(r[idx], axis=1), resamples, seed)
    elif statistic == "ratio_of_medians":
        est = float(np.median(c) / np.median(b))
        reps = _boot_stat(c.size, lambda idx: np.median(c[idx], axis=1) / np.median(b[idx], axis=1), resamples, seed)
    elif statistic == "geomean_of_ratios":
        if np.any(c <= 0):
            raise ValueError("geomean_of_ratios requires candidate > 0")
        lr = np.log(c / b)
        est = float(np.exp(lr.mean()))
        reps = _boot_stat(c.size, lambda idx: np.exp(lr[idx].mean(axis=1)), resamples, seed)
    else:
        raise ValueError(f"unknown statistic {statistic}")
    lo, hi = _interval(reps, confidence)
    return CI(est, lo, hi, confidence, resamples, seed, f"paired-percentile-bootstrap:{statistic}", int(c.size))


def bootstrap_paired_difference_ci(
    candidate: Sequence[float], baseline: Sequence[float], *, confidence: float, seed: int, resamples: int = MIN_BOOTSTRAP_RESAMPLES
) -> CI:
    """CI for the median paired difference candidate - baseline."""
    _check_boot(confidence, resamples)
    c, b = _arr(candidate), _arr(baseline)
    if c.size != b.size or c.size < 2:
        raise ValueError("need equal-length paired samples with n >= 2")
    d = c - b
    reps = _boot_stat(d.size, lambda idx: np.median(d[idx], axis=1), resamples, seed)
    lo, hi = _interval(reps, confidence)
    return CI(float(np.median(d)), lo, hi, confidence, resamples, seed, "paired-percentile-bootstrap:median_difference", int(d.size))


# ----------------------------------------------------------------------------
# noise rule


@dataclass(frozen=True)
class NoiseVerdict:
    distinguishable: bool
    direction: Optional[str]  # "below" | "above" | None
    ci_lower: float
    ci_upper: float
    band_lower: float
    band_upper: float

    @property
    def label(self) -> str:
        if not self.distinguishable:
            return "indistinguishable"
        return f"distinguishable_{self.direction}"


def noise_rule(ci_lower: float, ci_upper: float, band: Tuple[float, float]) -> NoiseVerdict:
    """Distinguishable only if the whole CI lies outside the practical-significance band.

    A CI that overlaps the band at any point (including touching an edge) is
    indistinguishable.  ``direction`` says which side of the band the CI lies on.
    """
    lo, hi = float(band[0]), float(band[1])
    if lo > hi:
        raise ValueError("band lower > upper")
    if ci_lower > ci_upper:
        raise ValueError("ci lower > upper")
    if ci_upper < lo:
        return NoiseVerdict(True, "below", ci_lower, ci_upper, lo, hi)
    if ci_lower > hi:
        return NoiseVerdict(True, "above", ci_lower, ci_upper, lo, hi)
    return NoiseVerdict(False, None, ci_lower, ci_upper, lo, hi)


# ----------------------------------------------------------------------------
# epsilon-Pareto


def _modes(mode, k: int) -> List[str]:
    modes = [mode] * k if isinstance(mode, str) else list(mode)
    if len(modes) != k or any(m not in ("absolute", "relative") for m in modes):
        raise ValueError("mode must be 'absolute'/'relative' or one such value per objective")
    return modes


def _oriented(values: np.ndarray, directions: Sequence[str], modes: Sequence[str]) -> np.ndarray:
    """Transform so that every objective is minimised; relative objectives work in log space."""
    v = values.astype(float).copy()
    for j, d in enumerate(directions):
        if d not in ("minimize", "maximize"):
            raise ValueError(f"direction must be minimize/maximize, got {d}")
        if modes[j] == "relative":
            if np.any(v[:, j] <= 0):
                raise ValueError("relative epsilon requires strictly positive objective values")
            v[:, j] = np.log(v[:, j])
        if d == "maximize":
            v[:, j] = -v[:, j]
    return v


def _eps(epsilon, k: int, modes: Sequence[str]) -> np.ndarray:
    e = np.full(k, float(epsilon)) if np.isscalar(epsilon) else np.asarray(epsilon, dtype=float)
    if e.shape != (k,) or np.any(e < 0):
        raise ValueError("epsilon must be a non-negative scalar or one value per objective")
    return np.array([np.log1p(e[j]) if modes[j] == "relative" else e[j] for j in range(k)])


def eps_dominates(a: Sequence[float], b: Sequence[float], directions: Sequence[str], epsilon=0.0, mode="absolute") -> bool:
    """True if ``a`` epsilon-dominates ``b``.

    a dominates b iff for every objective a is not worse than b by more than eps_j,
    and for at least one objective a is better than b by more than eps_j.
    With epsilon = 0 this is standard Pareto dominance.  ``mode="relative"``
    (scalar or per objective) compares log values with eps_j = log(1 + epsilon_j).
    """
    k = len(directions)
    modes = _modes(mode, k)
    pts = _oriented(np.array([a, b], dtype=float), directions, modes)
    e = _eps(epsilon, pts.shape[1], modes)
    diff = pts[0] - pts[1]  # negative = a better
    not_worse = np.all(diff <= e)
    better = np.any(diff < -e)
    return bool(not_worse and better)


def pareto_frontier(points: Sequence[Sequence[float]], directions: Sequence[str], epsilon=0.0, mode="absolute") -> List[int]:
    """Indices of points not epsilon-dominated by any other point (stable order).

    With epsilon > 0 the frontier keeps points that are not *distinguishably*
    dominated (it can grow relative to the strict frontier) and drops points whose
    only advantage is within tolerance (it can also shrink).
    """
    pts = np.asarray(points, dtype=float)
    if pts.ndim != 2 or pts.shape[1] != len(directions):
        raise ValueError("points must be (n, k) with k == len(directions)")
    front = []
    for i in range(len(pts)):
        if not any(eps_dominates(pts[j], pts[i], directions, epsilon, mode) for j in range(len(pts)) if j != i):
            front.append(i)
    return front


def dominance_matrix(points: Sequence[Sequence[float]], directions: Sequence[str], epsilon=0.0, mode="absolute") -> np.ndarray:
    pts = np.asarray(points, dtype=float)
    n = len(pts)
    m = np.zeros((n, n), dtype=bool)
    for i, j in itertools.permutations(range(n), 2):
        m[i, j] = eps_dominates(pts[i], pts[j], directions, epsilon, mode)
    return m


# ----------------------------------------------------------------------------
# rank agreement


def kendall_tau(x: Sequence[float], y: Sequence[float]) -> Tuple[float, float]:
    """Kendall tau-b and two-sided p-value (scipy)."""
    res = _sps.kendalltau(np.asarray(x, dtype=float), np.asarray(y, dtype=float), variant="b")
    return float(res.statistic), float(res.pvalue)


def kendall_tau_rankings(order_a: Sequence[str], order_b: Sequence[str]) -> float:
    """Kendall tau-b between two orderings (best first) of the same item names."""
    if sorted(order_a) != sorted(order_b):
        raise ValueError("rankings must contain the same names")
    pos_b = {name: i for i, name in enumerate(order_b)}
    return kendall_tau(list(range(len(order_a))), [pos_b[n] for n in order_a])[0]


def ranks(values: Sequence[float], direction: str = "minimize") -> List[float]:
    a = np.asarray(values, dtype=float)
    return list(_sps.rankdata(a if direction == "minimize" else -a, method="average"))


# ----------------------------------------------------------------------------
# weight-perturbation sensitivity


GRID_FACTORS = (0.5, 0.75, 1.0, 1.25, 1.5)


def normalize_scores(matrix: Sequence[Sequence[float]], directions: Sequence[str]) -> np.ndarray:
    """Min-max normalise columns to [0, 1] with 1 = best (constant columns -> 1)."""
    m = np.asarray(matrix, dtype=float)
    out = np.empty_like(m)
    for j, d in enumerate(directions):
        col = m[:, j]
        lo, hi = col.min(), col.max()
        if hi == lo:
            out[:, j] = 1.0
        else:
            s = (col - lo) / (hi - lo)
            out[:, j] = s if d == "maximize" else 1.0 - s
    return out


def _winners(scores: np.ndarray, weights: np.ndarray, aggregate: str) -> np.ndarray:
    """scores (n_cand, k) in [0,1] higher-better; weights (n_w, k) rows sum to 1 -> winner index per row."""
    if aggregate == "sum":
        util = weights @ scores.T
    elif aggregate == "geometric":
        util = weights @ np.log(np.clip(scores, 1e-12, None)).T
    else:
        raise ValueError("aggregate must be sum or geometric")
    return np.argmax(util, axis=1), util


@dataclass
class SensitivityResult:
    base_winner: int
    grid_mode: str
    grid_count: int
    grid_fraction_preserving: float
    dirichlet_count: int
    dirichlet_fraction_preserving: Optional[float]
    winner_counts_grid: Dict[int, int]
    winner_counts_dirichlet: Dict[int, int]
    dirichlet_seed: Optional[int]
    dirichlet_concentration: Optional[float]

    def to_dict(self):
        return asdict(self)


def weight_sensitivity(
    scores: Sequence[Sequence[float]],
    base_weights: Sequence[float],
    *,
    grid_mode: str = "full",
    factors: Sequence[float] = GRID_FACTORS,
    dirichlet_samples: int = 0,
    dirichlet_concentration: Optional[float] = None,
    seed: Optional[int] = None,
    aggregate: str = "sum",
    tie_tolerance: float = 1e-12,
) -> SensitivityResult:
    """Fraction of weight perturbations that preserve the base-weight winner.

    ``scores``: (candidates, criteria), higher is better (see :func:`normalize_scores`).
    Grid: each weight multiplied by a factor in ``factors`` (default ±25 %/±50 %),
    ``grid_mode="full"`` = all combinations, ``"one_at_a_time"`` = vary one weight;
    weights are renormalised to sum 1.  Dirichlet: ``Dirichlet(c * k * w_base)`` if a
    concentration ``c`` is given (mean = base weights; c = 1 with equal base weights
    is the flat Dirichlet), else the flat ``Dirichlet(1,...,1)``.
    A perturbation preserves the winner when the base winner's utility is within
    ``tie_tolerance`` of the maximum.
    """
    s = np.asarray(scores, dtype=float)
    w0 = np.asarray(base_weights, dtype=float)
    if s.ndim != 2 or w0.shape != (s.shape[1],):
        raise ValueError("scores must be (n, k) and base_weights length k")
    if np.any(w0 < 0) or w0.sum() <= 0:
        raise ValueError("weights must be non-negative with positive sum")
    w0 = w0 / w0.sum()
    k = s.shape[1]
    base_w, base_util = _winners(s, w0[None, :], aggregate)
    base = int(base_w[0])

    if grid_mode == "full":
        combos = np.array(list(itertools.product(factors, repeat=k)), dtype=float)
    elif grid_mode == "one_at_a_time":
        rows = []
        for j in range(k):
            for f in factors:
                r = np.ones(k)
                r[j] = f
                rows.append(r)
        combos = np.unique(np.array(rows), axis=0)
    else:
        raise ValueError("grid_mode must be full or one_at_a_time")
    gw = combos * w0[None, :]
    gw = gw[gw.sum(axis=1) > 0]
    gw = gw / gw.sum(axis=1, keepdims=True)

    def preserved(weights: np.ndarray) -> Tuple[float, Dict[int, int]]:
        win, util = _winners(s, weights, aggregate)
        keep = util[:, base] >= util.max(axis=1) - tie_tolerance
        counts: Dict[int, int] = {}
        for x in win:
            counts[int(x)] = counts.get(int(x), 0) + 1
        return float(keep.mean()), counts

    g_frac, g_counts = preserved(gw)
    d_frac, d_counts = None, {}
    if dirichlet_samples:
        if seed is None:
            raise ValueError("Dirichlet sampling requires a seed")
        rng = np.random.Generator(np.random.PCG64(seed))
        alpha = np.ones(k) if dirichlet_concentration is None else dirichlet_concentration * w0 * k
        if np.any(alpha <= 0):
            raise ValueError("Dirichlet alpha must be positive (zero base weight with concentration)")
        dw = rng.dirichlet(alpha, size=int(dirichlet_samples))
        d_frac, d_counts = preserved(dw)
    return SensitivityResult(base, grid_mode, int(len(gw)), g_frac, int(dirichlet_samples), d_frac, g_counts, d_counts, seed, dirichlet_concentration)


def safe_float(x) -> Optional[float]:
    try:
        f = float(x)
    except (TypeError, ValueError):
        return None
    return f if math.isfinite(f) else None
