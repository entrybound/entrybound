"""Synthetic negative admission cases; no held-out files or live decisions."""

from __future__ import annotations

import copy
import json
import sys
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[4]
sys.path.insert(0, str(ROOT / "research/tools/methods"))
import gate_admission as gate  # noqa: E402


class GateAdmissionTests(unittest.TestCase):
    def test_public_register_covers_every_method_item(self) -> None:
        register = json.loads((ROOT / gate.REGISTER).read_text(encoding="utf-8"))
        self.assertEqual([row["number"] for row in register["items"]], list(range(1, 20)))
        for row in register["items"]:
            self.assertIn(row["status"], {"OPEN", "PASS"})
            if row["status"] == "OPEN":
                self.assertTrue(row["open_reason"])
                self.assertIsNone(row["evidence"])
            else:
                self.assertIsInstance(row["evidence"], dict)
        self.assertEqual(register["method_sha256"], gate.sha((ROOT / gate.METHOD).read_bytes()))

    def test_negative_phase_fixtures_block_exact_missing_items(self) -> None:
        fixtures = json.loads((Path(__file__).parent / "fixtures/admission-negative.json").read_text(encoding="utf-8"))
        for case in fixtures:
            with self.subTest(case=case["name"]):
                states = {n: {"status": "PASS"} for n in gate.ITEMS}
                for number in case["open_items"]:
                    states[number] = {"status": "OPEN"}
                result = gate.admission(case["event"], states,
                                        measurement=case["measurement"],
                                        decision_id=case["decision_id"],
                                        routes=set(case["routes"]),
                                        has_validation_looks=case["has_validation_looks"])
                self.assertEqual(result["status"], "BLOCKED")
                self.assertEqual(result["blocked_items"], case["expected_blocked"])

    def test_future_item_cannot_pass_on_bare_declaration(self) -> None:
        self.assertIn("verifier not implemented", " ".join(gate.check_receipt(ROOT, 2, {}, {})))
        register = json.loads((ROOT / gate.REGISTER).read_text(encoding="utf-8"))
        forged = copy.deepcopy(register)
        forged["items"][1]["status"] = "PASS"
        forged["items"][1]["open_reason"] = ""
        forged["items"][1]["evidence"] = {"artifact_exists": True}
        result = gate.evaluate(ROOT, forged, run_live=False)
        self.assertEqual(result["items"][2]["status"], "OPEN")
        self.assertIn("verifier not implemented", " ".join(result["items"][2]["reasons"]))

    def test_reviewed_no_primary_od_cannot_gain_grading_or_utility(self) -> None:
        ledger = {"DEC-ECO-080": {"primary_metrics_by_od": {},
                                  "no_primary_metric_reason_by_od": {"OD-20": "Governed outcome disclosure."}}}
        claims = {"DEC-ECO-080": {"graded": True,
                                   "primary_metrics_by_od": {"OD-20": "artifact_bytes"},
                                   "utility_od_ids": ["OD-20"], "non_graded_reason": ""}}
        errors = gate.graded_route_errors(ledger, claims)
        self.assertTrue(any("no-primary disposition" in error for error in errors))
        self.assertTrue(any("utility weight" in error for error in errors))
        all_pass = {n: {"status": "PASS"} for n in gate.ITEMS}
        result = gate.admission("first_result", all_pass, measurement="other",
                                ledger_assignments=ledger, metric_claims=claims)
        self.assertEqual(result["blocked_items"], [])
        self.assertEqual(result["status"], "BLOCKED")
        self.assertTrue(result["metric_errors"])
        missing = gate.admission("first_result", all_pass, measurement="other")
        self.assertEqual(missing["status"], "BLOCKED")
        self.assertTrue(any("not supplied" in error for error in missing["metric_errors"]))
        apparent_valid = gate.admission(
            "first_result", all_pass, measurement="other",
            ledger_assignments={"DEC-SYNTH": {"primary_metrics_by_od": {"OD-05": "artifact_bytes"},
                                              "no_primary_metric_reason_by_od": {}}},
            metric_claims={"DEC-SYNTH": {"graded": True,
                                          "primary_metrics_by_od": {"OD-05": "artifact_bytes"},
                                          "utility_od_ids": ["OD-05"], "non_graded_reason": ""}})
        self.assertEqual(apparent_valid["blocked_items"], [])
        self.assertEqual(apparent_valid["status"], "BLOCKED")
        self.assertTrue(any("verifier is not implemented" in error
                            for error in apparent_valid["metric_errors"]))
        malformed = {"DEC-SYNTH": {"graded": True,
                                    "primary_metrics_by_od": {"OD-05": "artifact_bytes"},
                                    "utility_od_ids": [{"OD-05": 1}], "non_graded_reason": ""}}
        self.assertTrue(any("types invalid" in error for error in
                            gate.graded_route_errors({"DEC-SYNTH": {}}, malformed)))

    def test_unresolved_metric_registration_blocks_results_and_decided(self) -> None:
        row = {"primary_metrics_by_od": {}, "no_primary_metric_reason_by_od": {},
               "metric_registration_blockers_by_od": {
                   "OD-10": "Question-specific access-cost prediction instrument lacks a registered role and band."}}
        ledger = {"DEC-SYNTH": row}
        claim = {"DEC-SYNTH": {"graded": False, "primary_metrics_by_od": {},
                                "utility_od_ids": [], "non_graded_reason": "Calibration diagnostic only."}}
        all_pass = {n: {"status": "PASS"} for n in gate.ITEMS}
        self.assertTrue(any("unresolved canonical metric registration" in error
                            for error in gate.graded_route_errors(ledger, claim)))
        result = gate.admission("first_result", all_pass, measurement="other",
                                ledger_assignments=ledger, metric_claims=claim)
        self.assertEqual(result["status"], "BLOCKED")
        self.assertTrue(any("unresolved canonical metric registration" in error
                            for error in result["metric_errors"]))
        decided = gate.admission("decided", all_pass, measurement="other",
                                 decision_id="DEC-SYNTH", routes={"FORMAL"},
                                 ledger_assignments=ledger)
        self.assertEqual(decided["status"], "BLOCKED")
        self.assertTrue(any("unresolved canonical metric registration" in error
                            for error in decided["metric_errors"]))
        experiment = gate.admission("every_experiment", all_pass, decision_ids=True,
                                    ledger_assignments=ledger)
        self.assertEqual(experiment["status"], "BLOCKED")
        self.assertTrue(any("unresolved canonical metric registration" in error
                            for error in experiment["metric_errors"]))
        self.assertTrue(any("verifier is not implemented" in error
                            for error in experiment["metric_errors"]))
        omitted_ids = gate.admission("every_experiment", all_pass)
        self.assertEqual(omitted_ids["status"], "BLOCKED")
        self.assertTrue(any("verifier is not implemented" in error
                            for error in omitted_ids["metric_errors"]))
        initial = gate.admission("record_admission", all_pass, decision_ids=True,
                                 ledger_assignments=ledger)
        self.assertEqual(initial["status"], "BLOCKED")
        self.assertTrue(any("unresolved canonical metric registration" in error
                            for error in initial["metric_errors"]))
        self.assertTrue(any("verifier is not implemented" in error
                            for error in initial["metric_errors"]))

    def test_admission_timing_and_conditional_prerequisites(self) -> None:
        self.assertEqual(gate.required_items("record_admission"), gate.GA)
        self.assertNotIn(9, gate.required_items("record_admission"))
        self.assertEqual(gate.required_items("first_validation", measurement="other") & {7, 9}, {7, 9})
        self.assertEqual(gate.required_items("first_result", measurement="timing") & {3, 4}, {3, 4})
        self.assertEqual(gate.required_items("first_result", measurement="memory") & {3, 4}, {4})
        self.assertEqual(gate.required_items("first_od16_result", measurement="other") & {16}, {16})
        self.assertEqual(gate.required_items("first_hc06_record") & {17}, {17})
        self.assertEqual(gate.required_items("first_remote_result", measurement="other") & {18}, {18})
        self.assertEqual(gate.required_items("first_cc_outcome", measurement="other") & {19}, {19})
        self.assertEqual(gate.required_items("every_experiment") & gate.GA, set())
        self.assertEqual(gate.required_items("every_experiment", decision_ids=True) & gate.GA, gate.GA)

    def test_private_and_heldout_refs_are_never_read(self) -> None:
        self.assertIsNone(gate.safe_path(ROOT, "research/corpus/heldout/items.json"))
        self.assertIsNone(gate.safe_path(ROOT, "research/corpus/fingerprints/heldout/manifest.json"))
        self.assertIsNone(gate.safe_path(ROOT, "research/corpus/items/heldout/sample.json"))
        self.assertIsNone(gate.safe_path(ROOT, "../outside.json"))
        self.assertIsNone(gate.safe_path(ROOT, ".git/private-note.json"))
        self.assertEqual(gate.safe_path(ROOT, "research/corpus/heldout-lock.json"),
                         (ROOT / "research/corpus/heldout-lock.json").resolve())
        self.assertEqual(gate.safe_path(ROOT, "research/methods/heldout-review.md"),
                         (ROOT / "research/methods/heldout-review.md").resolve())

    def test_decided_gate_helper_is_nonrecursive_and_open(self) -> None:
        state = gate.gate_d_status(ROOT)
        self.assertEqual(state["status"], "OPEN")
        self.assertTrue(state["reasons"])


if __name__ == "__main__":
    unittest.main()
