import json
import shutil
import uuid
from pathlib import Path

import pytest

from methods import cost_inputs


@pytest.fixture
def source_roots():
    base = Path(__file__).resolve().parents[3] / "target" / "cost-input-fixtures"
    root = base / uuid.uuid4().hex
    baseline, candidate = root / "baseline", root / "candidate"
    baseline.mkdir(parents=True)
    candidate.mkdir()
    try:
        yield baseline, candidate
    finally:
        assert root.resolve().is_relative_to(base.resolve())
        shutil.rmtree(root)


def put(root, ref, content):
    path = root / ref
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content, encoding="utf-8")


def fixture(source_roots):
    baseline, candidate = source_roots
    put(baseline, "docs/spec.md", "alpha beta\n")
    put(candidate, "docs/spec.md", "alpha gamma delta\n")
    old = {kind: [] for kind in cost_inputs.REGISTRY_KINDS}
    new = {**old, "reason_codes": ["EB_REASON_NEW"]}
    put(baseline, "registry.json", json.dumps(old))
    put(candidate, "registry.json", json.dumps(new))
    put(baseline, "src/writer.rs", "// comment\nfn old() {}\n")
    put(candidate, "src/writer.rs", "// comment\nfn old() {}\nfn new() {}\n")
    put(candidate, "src/reader.rs", "/* comment */\nfn read() {}\n")
    put(baseline, "tree.txt", "root v0.1.0\nfoo v1.0.0\n")
    put(candidate, "tree.txt", "root v0.1.0\nfoo v1.0.0\nbar v2.0.0\n")
    def meta(with_bar):
        packages = [{"id": "root", "name": "root", "version": "0.1.0"},
                    {"id": "foo", "name": "foo", "version": "1.0.0"}]
        deps = [{"pkg": "foo", "dep_kinds": [{"kind": None}]}]
        if with_bar:
            packages.append({"id": "bar", "name": "bar", "version": "2.0.0"})
            deps.append({"pkg": "bar", "dep_kinds": [{"kind": None}]})
        return {"packages": packages, "workspace_members": ["root"],
                "resolve": {"nodes": [{"id": "root", "deps": deps}]}}
    put(baseline, "meta.json", json.dumps(meta(False)))
    put(candidate, "meta.json", json.dumps(meta(True)))
    audit = {"advisory_snapshot_sha256": "a" * 64,
             "packages": {"bar@2.0.0": {"unsafe_blocks": 0, "unsafe_functions": 0,
                                         "native_source_bytes": 0, "spdx_license": "MIT",
                                         "last_release_date": "2026-01-01", "publisher_count": 2,
                                         "open_advisories": 0, "msrv": "1.70"}}}
    put(candidate, "audit.json", json.dumps(audit))
    manifest = {"schema": "entrybound.cost-input-manifest.v1", "candidate_id": "candidate-a",
                "baseline_commit_sha": "a" * 40, "candidate_commit_sha": "b" * 40,
                "normative_files": ["docs/spec.md"], "registry_census_ref": "registry.json",
                "source_roles": {"writer": ["src/writer.rs"], "full_reader": ["src/reader.rs"],
                                 "minimal_reader": [], "tests": []},
                "retained_decode_paths": ["src/reader.rs"], "cargo_tree_ref": "tree.txt",
                "cargo_metadata_ref": "meta.json", "dependency_audit_ref": "audit.json"}
    return baseline, candidate, manifest


def test_c1_c2_c3_are_measured_and_not_a_tier(source_roots):
    baseline, candidate, manifest = fixture(source_roots)
    result = cost_inputs.compute(manifest, baseline, candidate)
    assert result["C1"]["normative_words_added_or_changed"] == 2
    assert result["C1"]["registry"]["reason_codes"]["added_count"] == 1
    assert result["C2"]["writer"]["delta_loc"] == 1
    assert result["C3"]["direct_normal_packages_added"] == ["bar@2.0.0"]
    assert result["C3"]["complete"] is True
    assert result["tier_status"] == "AWAITING_INDEPENDENT_C4_C7_ASSESSMENT"


def test_missing_dependency_audit_blocks_tier(source_roots):
    baseline, candidate, manifest = fixture(source_roots)
    put(candidate, "audit.json", json.dumps({"advisory_snapshot_sha256": "a" * 64, "packages": {}}))
    result = cost_inputs.compute(manifest, baseline, candidate)
    assert result["C3"]["complete"] is False
    assert result["C3"]["missing_audit_entries"] == ["bar@2.0.0"]
    assert result["tier_status"] == "C3_INCOMPLETE_NO_TIER"


def test_heldout_path_refused(source_roots):
    baseline, candidate, manifest = fixture(source_roots)
    manifest["normative_files"] = ["research/heldout/x"]
    with pytest.raises(ValueError, match="held-out"):
        cost_inputs.compute(manifest, baseline, candidate)
