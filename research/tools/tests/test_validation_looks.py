import copy
import hashlib
import json
import os
import shutil
import stat
import subprocess
import uuid
from pathlib import Path

import pytest

from experiments import validation_looks as vl


SPEC = (
    "schema: ebr.spec.v1\nexperiment_id: EXP-TEST-001\n"
    "decision_ids: [{decision}]\ncandidates:\n  - name: {candidate}\n"
    "corpus:\n  splits: [validation]\nstatus: look-{number}\n"
)


@pytest.fixture
def tmp_path(monkeypatch):
    base = Path(__file__).resolve().parents[3] / "target" / "validation-look-fixtures"
    root = base / uuid.uuid4().hex
    root.mkdir(parents=True)
    git_dir = root / "gitdata"
    subprocess.run(["git", "init", "--bare", "-q", str(git_dir)], cwd=root, check=True)
    monkeypatch.setenv("GIT_DIR", str(git_dir))
    monkeypatch.setenv("GIT_WORK_TREE", str(root))
    subprocess.run(["git", "config", "core.bare", "false"], cwd=root, check=True)
    try:
        yield root
    finally:
        assert root.resolve().is_relative_to(base.resolve())
        def writable_then_remove(function, path, _error):
            os.chmod(path, stat.S_IWRITE)
            function(path)
        shutil.rmtree(root, onerror=writable_then_remove)


def fixture(tmp_path):
    root = tmp_path
    exp = root / "research/experiments/EXP-TEST-001"
    exp.mkdir(parents=True)
    d1 = "DEC-CMP-001"
    d2 = "DEC-CMP-002"
    ledger = {
        d1: {"decision_id": d1, "cluster": "compression",
             "candidate_set": [{"candidate_id": "a", "description": "A"}],
             "primary_metrics_by_od": {"OD-05": "artifact_bytes"},
             "metric_registration_blockers_by_od": {}},
        d2: {"decision_id": d2, "cluster": "compression",
             "candidate_set": [{"candidate_id": "b", "description": "B"}],
             "primary_metrics_by_od": {"OD-05": "artifact_bytes"},
             "metric_registration_blockers_by_od": {}},
    }
    plan = {(d, n): {"owning_experiment": "EXP-TEST-001", "participating_experiments": "",
                     "mde_record_path": f"research/normalized/EXP-TEST-001/decision/{d}-look{n}-mde.json"}
            for d in ledger for n in (1, 2)}
    for p in plan.values():
        mde = root / p["mde_record_path"]
        mde.parent.mkdir(parents=True, exist_ok=True)
        mde.write_text('{"status":"SYNTHETIC_PASS"}')
    (root / "research/decision-method.md").write_bytes(b"# Synthetic governing method\n")
    (exp / "record.md").write_bytes(b"# EXP-TEST-001\n## Pre-registration\nSynthetic fixture.\n")
    for decision, candidate in ((d1, "a"), (d2, "b")):
        for number in (1, 2):
            (exp / f"spec-{decision}-look{number}.yaml").write_bytes(
                SPEC.format(decision=decision, candidate=candidate, number=number).encode("utf-8"))
    subprocess.run(["git", "-c", "user.name=Fixture", "-c", "user.email=fixture@example.invalid",
                    "add", "research/decision-method.md", "research/experiments/EXP-TEST-001"],
                   cwd=root, check=True)
    subprocess.run(["git", "-c", "user.name=Fixture", "-c", "user.email=fixture@example.invalid",
                    "commit", "-qm", "synthetic method and pre-registration"], cwd=root, check=True)
    return root, ledger, plan


def registration(root, ledger, plan, decision, number, look_id, item, group):
    attestation = root / f"research/corpus/validation-attestation-{look_id}.json"
    attestation.parent.mkdir(parents=True, exist_ok=True)
    attestation.write_text(vl.canonical({"schema": "entrybound.validation-split-attestation.v1",
                                         "split": "validation", "items": {item: group}}))
    mde = root / plan[(decision, number)]["mde_record_path"]
    spec_ref = f"research/experiments/EXP-TEST-001/spec-{decision}-look{number}.yaml"
    spec_bytes = (root / spec_ref).read_bytes()
    commit = subprocess.run(["git", "rev-parse", "HEAD"], cwd=root, check=True,
                            capture_output=True, text=True).stdout.strip()
    row = {
        "schema": "entrybound.validation-look.v1", "event_type": "REGISTERED",
        "look_id": look_id, "look_number": number, "look_type": "CONFIRMATORY",
        "decision_ids": [decision], "experiment_id": "EXP-TEST-001",
        "participating_experiments": [], "candidate_ids": {decision: [ledger[decision]["candidate_set"][0]["candidate_id"]]},
        "primary_metrics": {decision: {"OD-05": "artifact_bytes"}},
        "validation_item_groups": {item: group},
        "validation_split_evidence_ref": str(attestation.relative_to(root)).replace("\\", "/"),
        "validation_split_evidence_sha256": hashlib.sha256(attestation.read_bytes()).hexdigest(),
        "spec_ref": spec_ref,
        "spec_sha256": hashlib.sha256(spec_bytes).hexdigest(),
        "mde_refs": {decision: plan[(decision, number)]["mde_record_path"]},
        "mde_sha256_by_decision": {decision: hashlib.sha256(mde.read_bytes()).hexdigest()},
        "method_commit_sha": commit, "source_commit_sha": commit,
        "registered_at_utc": "2026-10-01T00:00:00Z", "change_ref": "change.md" if number == 2 else "",
        "related_prior_look_ids": [], "charged_prior_look_ids": [],
    }
    return vl.enrich_registration(row, ledger)


def test_plan_is_not_execution(tmp_path):
    root, ledger, plan = fixture(tmp_path)
    assert vl.audit([], plan, ledger, root) == {
        "registered_looks": 0, "executed_looks": 0, "registered_unexecuted": 0,
        "confirmatory_registered": 0, "tuning_only_registered": 0}


def test_register_and_execution_are_distinct(tmp_path):
    root, ledger, plan = fixture(tmp_path)
    first = registration(root, ledger, plan, "DEC-CMP-001", 1, "VL-000001", "v1", "g1")
    vl.validate_registration(first, [], plan, ledger, root, check_current_files=True)
    assert vl.audit([first], plan, ledger, root)["registered_unexecuted"] == 1
    execution = {"schema": "entrybound.validation-look.v1", "event_type": "EXECUTED",
                 "look_id": "VL-000001", "executed_at_utc": "2026-10-01T01:00:00Z",
                 "raw_result_refs": ["research/raw/EXP-TEST-001/result.json"],
                 "raw_result_sha256": {"research/raw/EXP-TEST-001/result.json": "b" * 64},
                 "raw_meta_sha256": {"research/raw/EXP-TEST-001/result.json": "c" * 64}}
    execution["raw_result_refs"] = ["research/raw/EXP-TEST-001/result.jsonl"]
    execution["raw_result_sha256"] = {execution["raw_result_refs"][0]: "b" * 64}
    execution["raw_meta_sha256"] = {execution["raw_result_refs"][0]: "c" * 64}
    with pytest.raises(ValueError, match="raw result missing"):
        vl.validate_execution(execution, {"VL-000001": first}, set(), root, check_current_files=True)
    raw = root / execution["raw_result_refs"][0]
    raw.parent.mkdir(parents=True)
    raw.write_text("{}")
    execution["raw_result_sha256"][execution["raw_result_refs"][0]] = hashlib.sha256(raw.read_bytes()).hexdigest()
    meta = raw.with_name("result.meta.json")
    meta.write_text(vl.canonical({"run_id": "result", "raw_file": raw.name,
                                  "raw_sha256": execution["raw_result_sha256"][execution["raw_result_refs"][0]],
                                  "status": "complete", "experiment_id": first["experiment_id"],
                                  "decision_ids": first["decision_ids"], "spec_path": first["spec_ref"],
                                  "spec_sha256": first["spec_sha256"],
                                  "spec": {"candidates": [{"name": "a"}], "corpus": {"splits": ["validation"]}},
                                  "items": [{"item_id": "v1", "split": "validation"}],
                                  "started_utc": "2026-10-01T00:30:00Z",
                                  "git_sha": first["source_commit_sha"], "git_dirty": False}))
    execution["raw_meta_sha256"][execution["raw_result_refs"][0]] = hashlib.sha256(meta.read_bytes()).hexdigest()
    vl.validate_execution(execution, {"VL-000001": first}, set(), root, check_current_files=True)
    assert vl.audit([first, execution], plan, ledger, root)["executed_looks"] == 1
    original_meta = meta.read_text(encoding="utf-8")
    for change, message in ((lambda value: value["spec"]["candidates"].append({"name": "invented"}), "candidates"),
                            (lambda value: value.update(spec_sha256="0" * 64), "spec/decisions"),
                            (lambda value: value["items"][0].update(split="tuning"), "validation items")):
        changed = json.loads(original_meta)
        change(changed)
        meta.write_text(vl.canonical(changed), encoding="utf-8")
        execution["raw_meta_sha256"][execution["raw_result_refs"][0]] = hashlib.sha256(meta.read_bytes()).hexdigest()
        with pytest.raises(ValueError, match=message):
            vl.validate_execution(execution, {"VL-000001": first}, set(), root, check_current_files=True)


def test_owner_and_second_look_group_are_enforced(tmp_path):
    root, ledger, plan = fixture(tmp_path)
    first = registration(root, ledger, plan, "DEC-CMP-001", 1, "VL-000001", "v1", "g1")
    wrong_owner = copy.deepcopy(first)
    wrong_owner["experiment_id"] = "EXP-OTHER-001"
    with pytest.raises(ValueError, match="not planned owner"):
        vl.validate_registration(wrong_owner, [], plan, ledger, root)
    second = registration(root, ledger, plan, "DEC-CMP-001", 2, "VL-000002", "v2", "g1")
    second["related_prior_look_ids"] = ["VL-000001"]
    with pytest.raises(ValueError, match="reuses an independence group"):
        vl.validate_registration(second, [first], plan, ledger, root)


def test_cross_decision_candidate_change_is_charged(tmp_path):
    root, ledger, plan = fixture(tmp_path)
    first = registration(root, ledger, plan, "DEC-CMP-001", 1, "VL-000001", "v1", "g1")
    ledger["DEC-CMP-002"]["candidate_set"] = [{"candidate_id": "b", "description": "B revised"}]
    second = registration(root, ledger, plan, "DEC-CMP-002", 1, "VL-000002", "v2", "g2")
    second["related_prior_look_ids"] = ["VL-000001"]
    with pytest.raises(ValueError, match="not charged exactly"):
        vl.validate_registration(second, [first], plan, ledger, root)
    second["charged_prior_look_ids"] = ["VL-000001"]
    vl.validate_registration(second, [first], plan, ledger, root)


def test_no_primary_assignment_cannot_invent_registration_metric(tmp_path):
    root, ledger, plan = fixture(tmp_path)
    decision = "DEC-CMP-001"
    ledger[decision]["primary_metrics_by_od"] = {}
    row = registration(root, ledger, plan, decision, 1, "VL-000001", "v1", "g1")
    with pytest.raises(ValueError, match="primary metrics differ from ledger assignment"):
        vl.validate_registration(row, [], plan, ledger, root)
    row["primary_metrics"][decision] = {}
    vl.validate_registration(row, [], plan, ledger, root)


def test_unresolved_metric_registration_blocks_even_non_graded_look(tmp_path):
    root, ledger, plan = fixture(tmp_path)
    decision = "DEC-CMP-001"
    ledger[decision]["primary_metrics_by_od"] = {}
    ledger[decision]["metric_registration_blockers_by_od"] = {
        "OD-05": "Chosen-file recovery lacks an exact selection instrument, role and band."
    }
    row = registration(root, ledger, plan, decision, 1, "VL-000001", "v1", "g1")
    row["primary_metrics"][decision] = {}
    with pytest.raises(ValueError, match="missing or unresolved canonical metric registration"):
        vl.validate_registration(row, [], plan, ledger, root)
    del ledger[decision]["metric_registration_blockers_by_od"]
    with pytest.raises(ValueError, match="missing or unresolved canonical metric registration"):
        vl.validate_registration(row, [], plan, ledger, root)


def test_registration_rejects_nonexistent_or_uncommitted_source(tmp_path):
    root, ledger, plan = fixture(tmp_path)
    row = registration(root, ledger, plan, "DEC-CMP-001", 1, "VL-000001", "v1", "g1")
    row["source_commit_sha"] = "0" * 40
    with pytest.raises(ValueError, match="does not precede|absent from Git commit"):
        vl.validate_registration(row, [], plan, ledger, root)
