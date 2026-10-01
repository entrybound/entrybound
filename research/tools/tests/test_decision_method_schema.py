import copy
import csv
import hashlib
import json
import shutil
import subprocess
import uuid
from pathlib import Path

import pytest

from ledger import assemble, validate_decisions
from experiments import spec_lint


ROOT = Path(__file__).resolve().parents[3]


@pytest.fixture
def crosswalk_path():
    base = ROOT / "target" / "decision-method-fixtures"
    folder = base / uuid.uuid4().hex
    folder.mkdir(parents=True)
    try:
        yield folder / "crosswalk.csv"
    finally:
        assert folder.resolve().is_relative_to(base.resolve())
        shutil.rmtree(folder)


def v2_row():
    row = json.loads((ROOT / "research/decision-ledger.jsonl").read_text(encoding="utf-8").splitlines()[0])
    row.update(copy.deepcopy(assemble.DECISION_METHOD_DEFAULTS))
    return row


def test_generated_schema_matches_checked_in_file():
    _, generated = assemble.build_schemas()
    checked = json.loads((ROOT / "research/tools/ledger/decision-ledger.schema.json").read_text(encoding="utf-8"))
    assert generated == checked


def test_v2_placeholders_and_carry_forward():
    _, schema = assemble.build_schemas()
    row = v2_row()
    assert assemble.schema_validate(row, schema) == []
    previous = {"hc_ids": ["HC-01"], "od_ids": ["OD-05"], "decision_type": "EMPIRICAL",
                "evidence_route_types": ["EMPIRICAL"], "method_commit_sha": "a" * 40}
    assemble.preserve_method_fields(row, previous)
    assert {k: row[k] for k in previous} == previous
    assert assemble.schema_validate(row, schema) == []


def test_unregistered_primary_metric_rejected():
    _, schema = assemble.build_schemas()
    row = v2_row()
    row["od_ids"] = ["OD-05"]
    row["primary_metrics_by_od"] = {"OD-05": "unregistered_metric"}
    errors = validate_decisions.validate_row(row, schema, {}, {}, {}, ROOT, gate_a=False)
    assert any("unregistered primary metric" in e for e in errors)


def test_decided_cannot_bypass_method_gates():
    _, schema = assemble.build_schemas()
    row = v2_row()
    row["status"] = "DECIDED"
    errors = validate_decisions.validate_row(row, schema, {}, {}, {}, ROOT, gate_a=False)
    assert any("G-A method assignment incomplete" in e for e in errors)
    assert any("computed cost tier" in e for e in errors)
    assert any("history lacks R0-R10" in e for e in errors)


def test_bare_invariant_uses_governed_appendix_a_map():
    _, schema = assemble.build_schemas()
    row = v2_row()
    row["hard_constraints"] = ["I1"]
    row["hc_ids"] = ["HC-01"]
    errors = validate_decisions.validate_row(row, schema, {}, {}, {}, ROOT, gate_a=True)
    assert not any("uncrosswalked hard constraint" in error for error in errors)
    assert not any("hc_ids omit crosswalk" in error for error in errors)


def test_crosswalk_rejects_unregistered_appendix_b_freeze(crosswalk_path):
    constraint = "synthetic freeze claim"
    with crosswalk_path.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=["constraint_string", "string_sha256", "class",
                                                    "target", "rationale", "classifier", "reviewer"])
        writer.writeheader()
        writer.writerow({"constraint_string": constraint,
                         "string_sha256": hashlib.sha256(constraint.encode()).hexdigest(),
                         "class": "F-xx", "target": "F-99", "rationale": "synthetic negative fixture",
                         "classifier": "producer", "reviewer": "reviewer"})
    errors = []
    validate_decisions.load_crosswalk(crosswalk_path, errors, True,
                                      validate_decisions.registered_freezes(ROOT))
    assert any("unregistered objective Appendix B freeze" in error for error in errors)


def test_zero_hc_needs_reviewed_applicability_rationale():
    _, schema = assemble.build_schemas()
    row = v2_row()
    row["hard_constraints"] = []
    errors = validate_decisions.validate_row(row, schema, {}, {}, {}, ROOT, gate_a=True)
    assert any("zero applicable HC requires reviewed applicability rationale" in error for error in errors)
    row["hc_applicability_rationale"] = "No registered HC applies to this scoped question; independent review required."
    errors = validate_decisions.validate_row(row, schema, {}, {}, {}, ROOT, gate_a=True)
    assert not any("zero applicable HC requires reviewed applicability rationale" in error for error in errors)


def test_gate_a_rejects_self_review_and_unbound_review_file():
    _, schema = assemble.build_schemas()
    row = v2_row()
    row["hard_constraints"] = []
    row["hc_applicability_rationale"] = "No registered HC applies to this synthetic scoped question."
    row["assigning_session"] = "assignment-producer"
    row["assignment_reviewer_session"] = "assignment-producer"
    row["assignment_review_ref"] = "research/decision-method.md"
    row["assignment_review_sha256"] = hashlib.sha256((ROOT / row["assignment_review_ref"]).read_bytes()).hexdigest()
    errors = validate_decisions.validate_row(row, schema, {}, {}, {}, ROOT, gate_a=True)
    assert any("independent assignment reviewer session missing" in error for error in errors)
    row["assignment_reviewer_session"] = "independent-reviewer"
    errors = validate_decisions.validate_row(row, schema, {}, {}, {}, ROOT, gate_a=True)
    assert any("assignment review is not valid JSONL" in error for error in errors)


def test_review_digest_binds_every_governed_assignment_field(crosswalk_path, monkeypatch):
    root = crosswalk_path.parent
    ref = validate_decisions.ASSIGNMENT_REVIEW_REF
    review = root / ref
    review.parent.mkdir(parents=True, exist_ok=True)
    row = v2_row()
    row.update({"hc_ids": ["HC-01"], "hc_applicability_rationale": "",
                "od_ids": ["OD-05"], "od_applicability_rationale": "", "decision_type": "EMPIRICAL",
                "evidence_route_types": ["EMPIRICAL"],
                "primary_metrics_by_od": {"OD-05": "artifact_bytes"},
                "no_primary_metric_reason_by_od": {}, "primary_metric_justifications": {},
                "assigning_session": "producer", "assignment_reviewer_session": "reviewer",
                "assignment_review_ref": ref})
    record = {"schema": "entrybound.assignment-review.v1", "decision_id": row["decision_id"],
              "reviewer_session": "reviewer", "review_status": "ACCEPTED_PROSPECTIVE_ASSIGNMENT",
              "reviewed_assignment": {field: row[field] for field in validate_decisions.ASSIGNMENT_REVIEW_FIELDS},
              "reviewed_assignment_sha256": validate_decisions.reviewed_assignment_digest(row)}
    review.write_text(json.dumps(record) + "\n", encoding="utf-8")
    row["assignment_review_sha256"] = hashlib.sha256(review.read_bytes()).hexdigest()
    monkeypatch.setattr(validate_decisions, "checked_pinned_ref", lambda *_args, **_kwargs: None)
    errors = []
    validate_decisions.checked_assignment_review(root, row, errors, row["decision_id"])
    assert errors == []
    missing_tenth = copy.deepcopy(record)
    del missing_tenth["reviewed_assignment"]["metric_registration_blockers_by_od"]
    review.write_text(json.dumps(missing_tenth) + "\n", encoding="utf-8")
    errors = []
    validate_decisions.checked_assignment_review(root, row, errors, row["decision_id"])
    assert any("review payload differs" in error for error in errors)
    review.write_text(json.dumps(record) + "\n", encoding="utf-8")
    mutations = {
        "hc_ids": [],
        "hc_applicability_rationale": "Changed applicability",
        "od_ids": ["OD-06"],
        "od_applicability_rationale": "Changed OD applicability exception.",
        "decision_type": "FORMAL",
        "evidence_route_types": ["EMPIRICAL", "FORMAL"],
        "primary_metrics_by_od": {},
        "no_primary_metric_reason_by_od": {"OD-05": "Changed no-primary reason with enough detail."},
        "primary_metric_justifications": {"OD-05": "Changed primary metric justification."},
        "metric_registration_blockers_by_od": {
            "OD-05": "Missing exact selection instrument, role and band for this criterion."},
    }
    for field, changed in mutations.items():
        mutated = copy.deepcopy(row)
        mutated[field] = changed
        errors = []
        validate_decisions.checked_assignment_review(root, mutated, errors, row["decision_id"])
        assert any("review digest differs" in error for error in errors), field


def test_gate_a_rejects_omitted_od_metric_disposition():
    _, schema = assemble.build_schemas()
    row = v2_row()
    row["od_ids"] = ["OD-05"]
    errors = validate_decisions.validate_row(row, schema, {}, {}, {}, ROOT, gate_a=True)
    assert any("every OD needs primary metric, reviewed no-primary rationale, or reviewed metric-registration blocker" in error for error in errors)


def test_reviewed_metric_blocker_is_a_disjoint_prospective_disposition():
    registry = json.loads((ROOT / "research/methods/thresholds.json").read_text(encoding="utf-8"))["metric_thresholds"]
    row = v2_row()
    row["od_ids"] = ["OD-05"]
    row["metric_registration_blockers_by_od"] = {
        "OD-05": "File-recovery preference lacks an exact selection instrument, role and band."
    }
    errors = []
    validate_decisions.metric_check(row, registry, errors, "fixture", True)
    assert errors == []
    row["primary_metrics_by_od"] = {"OD-05": "artifact_bytes"}
    errors = []
    validate_decisions.metric_check(row, registry, errors, "fixture", True)
    assert any("overlapping primary, no-primary, or metric-registration blocker" in e for e in errors)
    row["primary_metrics_by_od"] = {}
    row["metric_registration_blockers_by_od"] = {"OD-06": "No exact selection instrument, role and band are registered."}
    errors = []
    validate_decisions.metric_check(row, registry, errors, "fixture", True)
    assert any("outside od_ids" in e for e in errors)
    row["metric_registration_blockers_by_od"] = {"OD-05": "missing metric"}
    errors = []
    validate_decisions.metric_check(row, registry, errors, "fixture", True)
    assert any("lacks exact missing instrument/role/band detail" in e for e in errors)


def test_decided_rejects_unresolved_metric_registration_blocker():
    _, schema = assemble.build_schemas()
    row = v2_row()
    row["status"] = "DECIDED"
    row["od_ids"] = ["OD-05"]
    row["metric_registration_blockers_by_od"] = {
        "OD-05": "Chosen-plaintext recovery lacks an exact selection instrument, role and band."
    }
    errors = validate_decisions.validate_row(row, schema, {}, {}, {}, ROOT, gate_a=False)
    assert any("DECIDED retains unresolved canonical metric registration" in e for e in errors)


def test_decision_bearing_prereg_lint_requires_exact_review_and_no_metric_blocker(monkeypatch):
    _, schema = assemble.build_schemas()
    registry = json.loads((ROOT / "research/methods/thresholds.json").read_text(encoding="utf-8"))["metric_thresholds"]
    row = v2_row()
    decision = row["decision_id"]
    spec = {"schema": "ebr.spec.v1", "experiment_id": "EXP-TEST-001", "decision_ids": [decision]}
    path = ROOT / "research/experiments/EXP-TEST-001/spec.yaml"
    errors = spec_lint.assignment_findings(path, spec, {}, schema, registry, ROOT)
    assert any("live decision assignment missing" in error[3] for error in errors)
    row["decision_type"] = "EMPIRICAL"
    row["evidence_route_types"] = ["EMPIRICAL"]
    row["od_ids"] = ["OD-05"]
    row["primary_metrics_by_od"] = {"OD-05": "artifact_bytes"}
    errors = spec_lint.assignment_findings(path, spec, {decision: row}, schema, registry, ROOT)
    assert any("exact reviewed assignment invalid" in error[3] for error in errors)
    monkeypatch.setattr(spec_lint.validate_decisions, "checked_assignment_review",
                        lambda *_args: None)
    row["primary_metrics_by_od"] = {}
    row["metric_registration_blockers_by_od"] = {
        "OD-05": "The chosen-file criterion lacks an exact selection instrument, role and band."
    }
    errors = spec_lint.assignment_findings(path, spec, {decision: row}, schema, registry, ROOT)
    assert any("unresolved canonical metric registration" in error[3] for error in errors)


def test_empty_product_od_is_limited_to_reviewed_method_admission():
    _, schema = assemble.build_schemas()
    row = v2_row()
    row["od_applicability_rationale"] = "Governed method admission does not select product utility."
    errors = validate_decisions.validate_row(row, schema, {}, {}, {}, ROOT, gate_a=True)
    assert any("empty product OD requires reviewed method-admission exception" in error for error in errors)
    row["decision_id"] = "DEC-ECO-077"
    errors = validate_decisions.validate_row(row, schema, {}, {}, {}, ROOT, gate_a=True)
    assert not any("empty product OD requires reviewed method-admission exception" in error for error in errors)
    row["od_ids"] = ["OD-05"]
    errors = validate_decisions.validate_row(row, schema, {}, {}, {}, ROOT, gate_a=True)
    assert any("OD applicability exception cannot accompany assigned product ODs" in error for error in errors)


def test_public_heldout_reports_allowed_but_payload_paths_refused():
    safe = "research/reports/heldout-validation.json"
    assert validate_decisions.source_ref(ROOT, safe) == (ROOT / safe).resolve()
    assert validate_decisions.source_ref(ROOT, "research/corpus/heldout/items/private-sample") is None
    assert validate_decisions.source_ref(ROOT, "research/corpus/fingerprints/heldout/item.json") is None
    assert validate_decisions.source_ref(ROOT, "../../root/eb-research/heldout/item") is None


def test_evidence_ref_requires_existing_committed_matching_hash():
    ref = "research/methods/thresholds.json"
    digest = hashlib.sha256((ROOT / ref).read_bytes()).hexdigest()
    row = {"evidence_sha256": {ref: digest}}
    errors = []
    validate_decisions.checked_pinned_ref(ROOT, row, ref, errors, "fixture")
    assert errors == []
    row["evidence_sha256"][ref] = "0" * 64
    validate_decisions.checked_pinned_ref(ROOT, row, ref, errors, "fixture")
    assert any("SHA-256 mismatch" in error for error in errors)
    validate_decisions.checked_pinned_ref(ROOT, row, "research/experiments/EXP-TEST-999/raw.json", errors, "fixture")
    assert any("missing or unsafe evidence reference" in error for error in errors)


def test_primary_metric_roles_and_no_primary_disposition():
    registry = json.loads((ROOT / "research/methods/thresholds.json").read_text(encoding="utf-8"))["metric_thresholds"]
    row = v2_row()
    row["od_ids"] = ["OD-05"]
    row["primary_metrics_by_od"] = {"OD-05": "dedup_stored_fraction"}
    errors = []
    validate_decisions.metric_check(row, registry, errors, "fixture", True)
    assert any("named component justification" in error for error in errors)
    row["primary_metric_justifications"] = {
        "OD-05": "M05.4 dedup_stored_fraction is the exact component this decision selects."
    }
    errors = []
    validate_decisions.metric_check(row, registry, errors, "fixture", True)
    assert errors == []
    row["primary_metrics_by_od"] = {}
    row["no_primary_metric_reason_by_od"] = {"OD-05": "Binary floor only; no graded selection metric applies."}
    errors = []
    validate_decisions.metric_check(row, registry, errors, "fixture", True)
    assert errors == []
    row["primary_metrics_by_od"] = {"OD-05": "roundtrip_mismatches"}
    errors = []
    validate_decisions.metric_check(row, registry, errors, "fixture", True)
    assert any("overlapping primary, no-primary, or metric-registration blocker" in error for error in errors)
    assert any("non-primary role" in error for error in errors)


@pytest.mark.parametrize("role", ["binary", "descriptive", "cost_input", "heuristic",
                                  "secondary_only", "sensitivity_arm"])
def test_nonselecting_metric_roles_cannot_be_primary(role):
    registry = json.loads((ROOT / "research/methods/thresholds.json").read_text(encoding="utf-8"))["metric_thresholds"]
    name = next(name for name, detail in registry.items() if detail["role"] == role)
    row = v2_row()
    row["od_ids"] = ["OD-05"]
    row["primary_metrics_by_od"] = {"OD-05": name}
    errors = []
    validate_decisions.metric_check(row, registry, errors, "fixture", True)
    assert any("non-primary role" in error for error in errors)


def test_mixed_routes_cannot_drop_empirical_formal_or_external_evidence():
    _, schema = assemble.build_schemas()
    row = v2_row()
    method_commit = subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT, capture_output=True,
                                   text=True, check=True).stdout.strip()
    row.update({"status": "DECIDED", "blocker_class": "NONE", "hc_applicability_rationale": "Synthetic zero-HC fixture only.",
                "decision_type": "EMPIRICAL", "evidence_route_types": ["EMPIRICAL", "FORMAL", "EXTERNAL"],
                "method_commit_sha": method_commit,
                "gate_audit_refs": ["research/decision-method.md"]})
    row["history"].append({"date": "2026-10-01", "change": "synthetic route-negative fixture",
                           "method_commit_sha": method_commit,
                           "rule_outcomes": {f"R{i}": "PASS" for i in range(11)},
                           "deviations": [], "evidence_refs": ["research/decision-method.md"],
                           "completeness_critic_ref": "research/decision-method.md"})
    errors = validate_decisions.validate_row(row, schema, {}, {}, {}, ROOT, gate_a=False)
    assert any("empirical held-out/freeze evidence missing" in error for error in errors)
    assert any("FORMAL counterexample search/reviewer missing" in error for error in errors)
    assert any("missing or unsafe evidence reference" in error for error in errors)  # EXTERNAL review
    assert any("gate audit must use the §8.6 decision path" in error for error in errors)
