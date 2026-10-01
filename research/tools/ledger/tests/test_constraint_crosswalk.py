"""Focused exact-source checks for the B01 crosswalk and K01 proposal."""

from __future__ import annotations

import csv
import hashlib
import json
import subprocess
import sys
import unittest
from pathlib import Path


REPO = Path(__file__).resolve().parents[4]
sys.path.insert(0, str(REPO / "research/tools/ledger"))

import constraint_crosswalk as cc  # noqa: E402


class CrosswalkTests(unittest.TestCase):
    def test_exact_source_coverage_and_regeneration(self) -> None:
        ledger = REPO / "research/decision-ledger.jsonl"
        output = REPO / "research/methods/constraint-crosswalk.csv"
        source = cc.source_rows(ledger)
        generated = cc.build(ledger, output)
        cc.validate(generated)
        self.assertEqual(len(source), 989)
        self.assertEqual([r["constraint_string"] for r in generated], source)
        self.assertEqual(output.read_text(encoding="utf-8"), cc.render(generated))
        with output.open(encoding="utf-8", newline="") as handle:
            self.assertEqual(list(csv.DictReader(handle)), generated)
        self.assertTrue(all(r["string_sha256"] == hashlib.sha256(
            r["constraint_string"].encode("utf-8")).hexdigest() for r in generated))

    def test_reviewer_signoff_survives_only_identical_disposition(self) -> None:
        row = cc.build(REPO / "research/decision-ledger.jsonl")[0]
        old = {**row, "reviewer": "independent-review-ref"}
        self.assertEqual(cc.carried_reviewer(old, row), "independent-review-ref")
        changed = {**row, "rationale": row["rationale"] + " changed"}
        self.assertEqual(cc.carried_reviewer(old, changed), "")

    def test_known_semantic_routes(self) -> None:
        self.assertEqual(cc.classify("I24 physical chunk boundaries must not leak plaintext structure in encrypted archives")[:2],
                         ("HC-xx", "HC-14"))
        self.assertEqual(cc.classify("CLI exclusive-create no-overwrite (entrybound-cli/src/lib.rs:4952-4964)")[:2],
                         ("HC-xx", "HC-18"))
        self.assertEqual(cc.classify("SPEC §24 item 5: no Entrybound-defined transport protocol")[:2],
                         ("NON_GOAL", "F-25"))
        self.assertEqual(cc.classify("SPEC §20.7 no in-archive decompressor VM")[:2],
                         ("F-xx", "F-30"))
        self.assertEqual(cc.classify("PROGRESS standing decision 2026-09-12: local and Docker only")[:2],
                         ("PROGRAM", ""))
        self.assertEqual(cc.classify("SPEC §6.2 balanced is default (frozen, open to evidence)")[:2],
                         ("DECISION_INPUT", ""))

    def test_adversarial_exact_source_corrections(self) -> None:
        cases = {
            "Exclusive create, no overwrite (docs/stream-layout-v1.md L383-385)": ("HC-xx", "HC-18"),
            "SPEC §9.10 hybrid PQ KEM mandatory from v1": ("F-xx", "F-12"),
            "crypto-v1 wire frozen: CONTENT binding mandatory": ("F-xx", "F-13"),
            "docs/crypto-threat-model-v1.md L105-107 removing a recipient rotates the file key and re-encrypts": ("F-xx", "F-14"),
            "compromised host is a non-goal (docs/crypto-threat-model-v1.md L209)": ("DECISION_INPUT", ""),
            "Standing constraint: platform experiments local/WSL/Docker only; macOS/APFS PLATFORM_BLOCKED": ("PROGRAM", ""),
            "model decision determinism-guarantee-scope (output must not depend on unrecorded ambient state)": ("HC-xx", "HC-11;HC-13"),
            "unknown TLV fields rejected (docs/format-v0.md §Decisions L24-29)": ("HC-xx", "HC-04"),
            "SPEC §19.4 object shape and three normative behaviours not deferred": ("HC-xx", "HC-05;HC-15"),
            "SPEC §9.6 L1125 timestamps recommended": ("DECISION_INPUT", ""),
            "docs/legacy-export-v1.md L51-52 legacy targets never embed Entrybound signatures": ("F-xx", "F-22"),
            "SPEC §4.9 L478 (uncompressed default; compact-manifest incompat)": ("HC-xx", "HC-04"),
            "SPEC §5.8 registries with reserved ranges": ("HC-xx", "HC-04;HC-16"),
        }
        self.assertEqual(len(cc.EXACT_ROUTES), 70)
        self.assertTrue(set(cc.EXACT_ROUTES) <= set(cc.source_rows(REPO / "research/decision-ledger.jsonl")))
        for source, expected in cases.items():
            with self.subTest(source=source):
                self.assertEqual(cc.classify(source)[:2], expected)

    def test_rejects_hash_or_screening_class_corruption(self) -> None:
        row = dict(zip(cc.FIELDS, (
            "sample", "0" * 64, "PROGRAM", "", "governance", cc.CLASSIFIER, "")))
        with self.assertRaisesRegex(ValueError, "incorrect string digest"):
            cc.validate([row])
        row["string_sha256"] = hashlib.sha256(b"sample").hexdigest()
        row["target"] = "HC-01"
        with self.assertRaisesRegex(ValueError, "nonscreening class"):
            cc.validate([row])

    def test_assignment_proposal_covers_exact_ledger_without_signoff(self) -> None:
        proposal_raw = [line.rstrip(b"\r\n") for line in (
            REPO / "research/methods/decision-assignments-proposal.jsonl").read_bytes().splitlines()
            if line.strip()]
        proposal = [json.loads(line) for line in proposal_raw]
        current_bytes = (REPO / "research/decision-ledger.jsonl").read_bytes()
        current = [json.loads(line) for line in current_bytes.splitlines() if line.strip()]
        source_digests = {row["source_ledger_sha256"] for row in proposal}
        self.assertEqual(len(source_digests), 1)
        # The proposal pins the governing v1 rows. The live ledger is now v2,
        # so compare with the recorded method commit when live bytes differ.
        # Before migration, those original v1 bytes are the live ledger.
        if hashlib.sha256(current_bytes).hexdigest() in source_digests:
            governing_bytes = current_bytes
        else:
            governing_commits = {row["method_commit_sha"] for row in current}
            self.assertEqual(len(governing_commits), 1)
            governing_commit = governing_commits.pop()
            self.assertRegex(governing_commit, r"^[0-9a-f]{40}$")
            governing_bytes = subprocess.run(
                ["git", "show", f"{governing_commit}:research/decision-ledger.jsonl"],
                cwd=REPO, check=True, capture_output=True,
            ).stdout
        self.assertEqual(source_digests, {hashlib.sha256(governing_bytes).hexdigest()})
        ledger_raw = [line.rstrip(b"\r\n") for line in governing_bytes.splitlines() if line.strip()]
        self.assertEqual(len(proposal), len(ledger_raw))
        self.assertEqual(len(current), len(ledger_raw))
        self.assertEqual(len(proposal), 596)
        review = [json.loads(line) for line in (
            REPO / "research/methods/decision-assignments-independent-review.jsonl").read_text(
                encoding="utf-8").splitlines() if line]
        self.assertEqual(len(review), len(proposal))
        for assignment, assignment_raw, reviewed, raw, migrated in zip(
                proposal, proposal_raw, review, ledger_raw, current):
            source = json.loads(raw)
            self.assertEqual(assignment["decision_id"], source["decision_id"])
            self.assertEqual(migrated["decision_id"], source["decision_id"])
            self.assertEqual(assignment["source_row_sha256"], hashlib.sha256(raw).hexdigest())
            self.assertEqual(reviewed["decision_id"], assignment["decision_id"])
            self.assertEqual(reviewed["source_proposal_row_sha256"],
                             hashlib.sha256(assignment_raw).hexdigest())
            for field, value in source.items():
                with self.subTest(decision=source["decision_id"], field=field):
                    if field == "history":
                        self.assertEqual(migrated[field][:len(value)], value)
                    else:
                        self.assertEqual(migrated[field], value)
            self.assertEqual(assignment["assignment_status"], "PROPOSED_UNAPPROVED")
            self.assertEqual(assignment["assignment_review_ref"], "")
            if source["decision_id"] in {"DEC-ECO-077", "DEC-ECO-080", "DEC-ECO-081", "DEC-ECO-082"}:
                self.assertEqual(assignment["od_ids"], [])
                self.assertEqual(assignment["od_requirement_screen_refs"], {})
                self.assertEqual(assignment["od_assignment_rationale"], {})
                self.assertEqual(assignment["primary_metrics_by_od"], {})
                self.assertEqual(assignment["no_primary_metric_reason_by_od"], {})
                self.assertIn("no product OD applies", assignment["od_applicability_rationale"])
            else:
                self.assertTrue(assignment["od_ids"])
                self.assertEqual(assignment["od_applicability_rationale"], "")
            self.assertTrue(set(assignment["primary_metrics_by_od"]).issubset(assignment["od_ids"]))
            self.assertTrue(set(assignment["hc_ids"]).isdisjoint(assignment["freeze_ids_from_crosswalk"]))
            if source["blocker_class"] == "HUMAN_PARTICIPANTS":
                self.assertEqual(assignment["decision_type"], "HUMAN-FACING")
            if source["blocker_class"] == "EXTERNAL_REVIEW":
                self.assertEqual(assignment["decision_type"], "EXTERNAL")

    def test_multi_od_capability_and_export_routes_remain_visible(self) -> None:
        proposal = {r["decision_id"]: r for r in (
            json.loads(line) for line in (
                REPO / "research/methods/decision-assignments-proposal.jsonl").read_text(
                    encoding="utf-8").splitlines() if line)}
        access = proposal["DEC-ACC-002"]
        self.assertIn("OD-11", access["od_ids"])
        self.assertGreater(len(access["od_ids"]), 4)
        export = proposal["DEC-ACC-013"]
        self.assertNotIn("OD-10", export["od_ids"])
        self.assertEqual(export["primary_metrics_by_od"]["OD-19"], "export_acceptance_fraction")
        self.assertNotIn("OD-09", export["primary_metrics_by_od"])
        self.assertNotIn("OD-18", proposal["DEC-CMP-045"]["od_ids"])
        self.assertEqual(proposal["DEC-CRY-099"]["od_ids"], ["OD-15"])
        self.assertIn("OD-15", proposal["DEC-CRY-095"]["od_ids"])
        self.assertEqual(proposal["DEC-ECO-036"]["decision_type"], "HUMAN-FACING")
        self.assertIn("EXTERNAL", proposal["DEC-ECO-036"]["evidence_route_types"])
        self.assertIn("OD-25", proposal["DEC-ECO-036"]["od_ids"])
        self.assertEqual(proposal["DEC-ECO-026"]["decision_type"], "EXTERNAL")
        for decision in ("DEC-CMP-017", "DEC-CMP-041"):
            self.assertEqual(proposal[decision]["decision_type"], "EMPIRICAL")

    def test_reviewed_source_specific_route_proposals(self) -> None:
        proposal = {r["decision_id"]: r for r in (
            json.loads(line) for line in (
                REPO / "research/methods/decision-assignments-proposal.jsonl").read_text(
                    encoding="utf-8").splitlines() if line)}
        for decision, ods in {
            "DEC-CMP-004": {"OD-05", "OD-06"},
            "DEC-CMP-027": {"OD-05", "OD-08", "OD-10", "OD-12", "OD-13", "OD-14"},
            "DEC-CRY-081": {"OD-08", "OD-10", "OD-14", "OD-15", "OD-16"},
            "DEC-ECO-041": {"OD-07", "OD-08", "OD-22", "OD-24"},
            "DEC-LEG-047": {"OD-17", "OD-19", "OD-22", "OD-24"},
            "DEC-LEG-108": {"OD-08", "OD-09", "OD-19"},
            "DEC-MOD-006": {"OD-05", "OD-06", "OD-07", "OD-08", "OD-09", "OD-21"},
        }.items():
            with self.subTest(decision=decision):
                self.assertEqual(set(proposal[decision]["od_ids"]), ods)
        self.assertEqual(proposal["DEC-ECO-053"]["evidence_route_types"],
                         ["HUMAN-FACING", "EXTERNAL", "EMPIRICAL"])
        self.assertEqual(proposal["DEC-LEG-047"]["evidence_route_types"],
                         ["EXTERNAL", "EMPIRICAL"])
        self.assertEqual(proposal["DEC-PLT-054"]["evidence_route_types"],
                         ["EMPIRICAL", "FORMAL"])
        for decision in ("DEC-ACC-027", "DEC-ACC-035", "DEC-CRY-061"):
            self.assertNotIn("EMPIRICAL", proposal[decision]["evidence_route_types"])
            self.assertEqual(proposal[decision]["primary_metrics_by_od"], {})
            self.assertEqual(set(proposal[decision]["no_primary_metric_reason_by_od"]),
                             set(proposal[decision]["od_ids"]))
        remote_calibration = proposal["DEC-ACC-063"]
        self.assertEqual(remote_calibration["primary_metrics_by_od"], {})
        self.assertIn("§4.15", remote_calibration["no_primary_metric_reason_by_od"]["OD-12"])


if __name__ == "__main__":
    unittest.main()
