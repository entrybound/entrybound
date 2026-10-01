"""Regression cases for the C03 embedded-invariant descriptive undercount."""
import importlib.util
from pathlib import Path
import unittest


MODULE_PATH = Path(__file__).resolve().parents[1] / "objective_screen.py"
SPEC = importlib.util.spec_from_file_location("objective_screen", MODULE_PATH)
screen = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(screen)


class DecisionInvariantCountsTests(unittest.TestCase):
    def test_embedded_references_are_counted(self):
        counts = screen.decision_invariant_counts([
            {"hard_constraints": ["I15 unique deterministic interpretation", "Preserve I31."]},
            {"hard_constraints": ["I15", "I1 / I20 declared bounds"]},
        ])
        self.assertEqual(dict(counts), {15: 2, 31: 1, 1: 1, 20: 1})

    def test_duplicates_count_once_per_decision(self):
        counts = screen.decision_invariant_counts([
            {"hard_constraints": ["I17", "I17 and I17 bounds", "I17", "I29"]},
            {"hard_constraints": ["I17 and I29"]},
        ])
        self.assertEqual(dict(counts), {17: 2, 29: 2})

    def test_invalid_tokens_and_aliases_do_not_count(self):
        counts = screen.decision_invariant_counts([
            {"hard_constraints": ["I0 I32 I100 XI1 I17_suffix i17 T1 P1 C1 E1"]},
            {"hard_constraints": []},
            {},
        ])
        self.assertEqual(dict(counts), {})

    def test_method_word_boundaries(self):
        counts = screen.decision_invariant_counts([
            {"hard_constraints": ["(I1), I9; I10/I19 and I30-I31"]},
        ])
        self.assertEqual(set(counts), {1, 9, 10, 19, 30, 31})
        self.assertTrue(all(count == 1 for count in counts.values()))


if __name__ == "__main__":
    unittest.main()
