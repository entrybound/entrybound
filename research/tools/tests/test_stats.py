import numpy as np
import pytest

from ebr import stats


def test_descriptives_known_values():
    x = list(range(1, 11))
    d = stats.describe(x)
    assert d["median"] == 5.5
    assert d["p90"] == pytest.approx(9.1)
    assert d["p95"] == pytest.approx(9.55)
    assert d["iqr"] == pytest.approx(4.5)
    assert d["mad"] == pytest.approx(2.5)
    assert stats.mad(x, scale=True) == pytest.approx(2.5 * 1.4826)
    assert d["n"] == 10 and d["min"] == 1 and d["max"] == 10


def test_descriptives_reject_bad_input():
    with pytest.raises(ValueError):
        stats.median([])
    with pytest.raises(ValueError):
        stats.median([1.0, float("nan")])


def test_geometric_mean():
    assert stats.geometric_mean([1, 10, 100]) == pytest.approx(10)
    with pytest.raises(ValueError):
        stats.geometric_mean([1, 0])
    assert stats.describe([-1, 1])["geomean"] is None


def test_bootstrap_median_seeded_and_enforces_resamples():
    rng = np.random.default_rng(1)
    x = rng.normal(0, 1, 60)
    a = stats.bootstrap_median_ci(x, confidence=0.95, seed=5)
    b = stats.bootstrap_median_ci(x, confidence=0.95, seed=5)
    c = stats.bootstrap_median_ci(x, confidence=0.95, seed=6)
    assert a == b
    assert (a.lower, a.upper) != (c.lower, c.upper)
    assert a.resamples == 10000
    assert a.lower <= a.estimate <= a.upper
    assert a.lower < 0 < a.upper  # true median 0 inside (seeded data)
    with pytest.raises(ValueError, match="10000"):
        stats.bootstrap_median_ci(x, confidence=0.95, seed=5, resamples=9999)
    with pytest.raises(ValueError):
        stats.bootstrap_median_ci([1.0], confidence=0.95, seed=5)
    with pytest.raises(ValueError):
        stats.bootstrap_median_ci(x, confidence=1.0, seed=5)


def test_bootstrap_wider_at_higher_confidence():
    x = np.random.default_rng(3).lognormal(0, 0.5, 40)
    lo = stats.bootstrap_median_ci(x, confidence=0.8, seed=1)
    hi = stats.bootstrap_median_ci(x, confidence=0.99, seed=1)
    assert hi.lower <= lo.lower and hi.upper >= lo.upper


def test_paired_ratio_ci():
    rng = np.random.default_rng(2)
    base = rng.uniform(1, 2, 30)
    exact = stats.bootstrap_paired_ratio_ci(2 * base, base, confidence=0.95, seed=1)
    assert exact.estimate == pytest.approx(2) and exact.lower == pytest.approx(2) and exact.upper == pytest.approx(2)
    noisy = 1.5 * base * rng.lognormal(0, 0.05, 30)
    for statistic in ("median_of_ratios", "ratio_of_medians", "geomean_of_ratios"):
        ci = stats.bootstrap_paired_ratio_ci(noisy, base, confidence=0.95, seed=1, statistic=statistic)
        assert 1.3 < ci.lower <= 1.5 <= ci.upper < 1.7, statistic
    with pytest.raises(ValueError):
        stats.bootstrap_paired_ratio_ci([1, 2], [1, 2, 3], confidence=0.95, seed=1)
    with pytest.raises(ValueError):
        stats.bootstrap_paired_ratio_ci([1, 2], [0, 2], confidence=0.95, seed=1)


def test_paired_difference_ci():
    b = np.arange(1.0, 21.0)
    ci = stats.bootstrap_paired_difference_ci(b + 3, b, confidence=0.9, seed=4)
    assert ci.estimate == pytest.approx(3) and ci.lower == pytest.approx(3) and ci.upper == pytest.approx(3)


@pytest.mark.parametrize(
    "ci,band,expected",
    [
        ((0.90, 0.95), (0.97, 1.03), "distinguishable_below"),
        ((1.04, 1.10), (0.97, 1.03), "distinguishable_above"),
        ((0.96, 0.99), (0.97, 1.03), "indistinguishable"),
        ((0.95, 0.97), (0.97, 1.03), "indistinguishable"),  # touching the band edge
        ((0.90, 1.10), (0.97, 1.03), "indistinguishable"),  # straddles the band
    ],
)
def test_noise_rule(ci, band, expected):
    assert stats.noise_rule(ci[0], ci[1], band).label == expected


def test_noise_rule_validates():
    with pytest.raises(ValueError):
        stats.noise_rule(1, 0, (0.9, 1.1))
    with pytest.raises(ValueError):
        stats.noise_rule(0, 1, (1.1, 0.9))


def test_pareto_dominance_basic():
    mm = ["minimize", "minimize"]
    assert stats.eps_dominates((1, 1), (2, 2), mm)
    assert not stats.eps_dominates((2, 2), (1, 1), mm)
    assert not stats.eps_dominates((1, 3), (2, 2), mm)
    assert not stats.eps_dominates((1, 1), (1, 1), mm)  # equal points do not dominate
    assert stats.eps_dominates((1, 2), (1, 3), mm)
    assert stats.eps_dominates((5, 1), (4, 2), ["maximize", "minimize"])


def test_epsilon_dominance_absolute_and_relative():
    mm = ["minimize", "minimize"]
    a, b = (1.0, 2.05), (1.5, 2.0)
    assert not stats.eps_dominates(a, b, mm)  # worse on objective 2 without tolerance
    assert stats.eps_dominates(a, b, mm, epsilon=0.1)
    # a must beat b by MORE than epsilon somewhere
    assert not stats.eps_dominates((1.45, 2.0), (1.5, 2.0), mm, epsilon=0.1)
    # relative: 2% worse allowed with 5% tolerance; 20% better counts
    assert stats.eps_dominates((80.0, 102.0), (100.0, 100.0), mm, epsilon=0.05, mode="relative")
    assert not stats.eps_dominates((80.0, 110.0), (100.0, 100.0), mm, epsilon=0.05, mode="relative")
    # per-objective modes
    assert stats.eps_dominates((80.0, 2.05), (100.0, 2.0), mm, epsilon=[0.05, 0.1], mode=["relative", "absolute"])
    with pytest.raises(ValueError):
        stats.eps_dominates((0.0, 1.0), (1.0, 1.0), mm, epsilon=0.1, mode="relative")


def test_pareto_frontier():
    mm = ["minimize", "minimize"]
    pts = [(1, 5), (2, 3), (2.05, 3.05), (4, 1), (5, 5), (2, 3)]
    # strict: (2.05, 3.05) is dominated by (2, 3); duplicates do not dominate each other
    assert stats.pareto_frontier(pts, mm) == [0, 1, 3, 5]
    # epsilon 0.1: (2.05, 3.05) is not distinguishably worse than (2, 3) -> kept
    assert stats.pareto_frontier(pts, mm, epsilon=0.1) == [0, 1, 2, 3, 5]
    # epsilon can also remove a point: (1.5, 2.0) is only 0.05 better on obj 2 than (1.0, 2.05)
    pts2 = [(1.0, 2.05), (1.5, 2.0)]
    assert stats.pareto_frontier(pts2, mm) == [0, 1]
    assert stats.pareto_frontier(pts2, mm, epsilon=0.1) == [0]
    m = stats.dominance_matrix(pts, mm)
    assert m[1, 4] and not m[4, 1]


def test_kendall_tau():
    assert stats.kendall_tau([1, 2, 3, 4], [1, 2, 3, 4])[0] == pytest.approx(1)
    assert stats.kendall_tau([1, 2, 3, 4], [4, 3, 2, 1])[0] == pytest.approx(-1)
    assert stats.kendall_tau_rankings(["a", "b", "c"], ["a", "b", "c"]) == pytest.approx(1)
    assert stats.kendall_tau_rankings(["a", "b", "c", "d"], ["b", "a", "c", "d"]) == pytest.approx(2 / 3)
    with pytest.raises(ValueError):
        stats.kendall_tau_rankings(["a"], ["b"])


def test_normalize_scores():
    s = stats.normalize_scores([[10, 1], [20, 3], [30, 3]], ["minimize", "maximize"])
    assert s.tolist() == [[1.0, 0.0], [0.5, 1.0], [0.0, 1.0]]


def test_weight_sensitivity_dominant_winner_is_stable():
    scores = stats.normalize_scores([[1, 1], [2, 2], [3, 3]], ["minimize", "minimize"])
    res = stats.weight_sensitivity(scores, [0.5, 0.5], dirichlet_samples=500, seed=1)
    assert res.base_winner == 0
    assert res.grid_count == 25
    assert res.grid_fraction_preserving == 1.0
    assert res.dirichlet_fraction_preserving == 1.0


def test_weight_sensitivity_tradeoff():
    # candidate 0 fast but big, candidate 1 small but slow, near-balanced under equal weights
    scores = stats.normalize_scores([[1.0, 100.0], [1.9, 50.0]], ["minimize", "minimize"])
    res = stats.weight_sensitivity(scores, [0.55, 0.45], grid_mode="one_at_a_time", dirichlet_samples=2000, seed=3)
    assert res.grid_count == 9
    assert 0.0 < res.grid_fraction_preserving < 1.0
    assert 0.0 < res.dirichlet_fraction_preserving < 1.0
    again = stats.weight_sensitivity(scores, [0.55, 0.45], grid_mode="one_at_a_time", dirichlet_samples=2000, seed=3)
    assert again == res
    centered = stats.weight_sensitivity(scores, [0.55, 0.45], dirichlet_samples=2000, dirichlet_concentration=50.0, seed=3)
    assert centered.dirichlet_fraction_preserving > res.dirichlet_fraction_preserving
    with pytest.raises(ValueError):
        stats.weight_sensitivity(scores, [0.5, 0.5], dirichlet_samples=10)
