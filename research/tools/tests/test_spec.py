import json

import pytest

from conftest import make_spec
from ebr.corpus import resolve_items
from ebr.spec import HeldoutLocked, SpecError, check_heldout, heldout_unlocked, load_spec, parse_spec
from ebr.thresholds import Thresholds, ThresholdUnset, template

SHA = "9e44608e5a3ed44fb8f2c4cc5423a6f69b0d0155"


def test_valid_spec_parses(spec_dict):
    s = parse_spec(spec_dict)
    assert s.experiment_id == "EXP-TEST-001"
    assert [c.name for c in s.candidates] == ["copy", "head"]
    assert s.metric("ratio").args == ["output_bytes", "input_bytes"]
    assert s.analysis.pairing == "item_repetition"


def test_yaml_and_json_equivalent(spec_dict, tmp_path):
    y = make_spec(spec_dict, tmp_path)
    jp = tmp_path / "spec.json"
    jp.write_text(json.dumps(spec_dict))
    j = load_spec(jp)
    assert y.raw == j.raw
    assert y.sha256 != j.sha256  # file digests differ by encoding


@pytest.mark.parametrize("field", ["experiment_id", "question", "hypothesis", "candidates", "corpus", "metrics", "repetitions", "seeds", "timing"])
def test_missing_required(spec_dict, field):
    del spec_dict[field]
    with pytest.raises(SpecError):
        parse_spec(spec_dict)


def test_unknown_top_level_field(spec_dict):
    spec_dict["timming"] = True
    with pytest.raises(SpecError, match="unknown top-level"):
        parse_spec(spec_dict)


def test_unknown_placeholder(spec_dict):
    spec_dict["candidates"][0]["command"] = "cat {item_path} > {scrach}/out"
    with pytest.raises(SpecError, match="unknown placeholder"):
        parse_spec(spec_dict)


def test_param_collision(spec_dict):
    spec_dict["candidates"][0]["params"] = {"scratch": 1}
    with pytest.raises(SpecError, match="collides"):
        parse_spec(spec_dict)


def test_timing_must_be_bool(spec_dict):
    spec_dict["timing"] = "yes"
    with pytest.raises(SpecError):
        parse_spec(spec_dict)


def test_unknown_family(spec_dict):
    spec_dict["metrics"][0]["family"] = "speed"
    with pytest.raises(SpecError, match="family"):
        parse_spec(spec_dict)


def test_derived_requires_known_args(spec_dict):
    spec_dict["metrics"][4]["args"] = ["output_bytes", "nope"]
    with pytest.raises(SpecError):
        parse_spec(spec_dict)


def test_thresholds_override_rejected_for_decisions(spec_dict):
    spec_dict["decision_ids"] = ["DEC-1"]
    spec_dict["thresholds_override"] = {"bootstrap": {"confidence": 0.9}}
    with pytest.raises(SpecError, match="thresholds_override"):
        parse_spec(spec_dict)


def test_threshold_template_all_null(layout):
    t = Thresholds.load(layout.thresholds_path)
    with pytest.raises(ThresholdUnset):
        t.bootstrap_confidence()
    with pytest.raises(ThresholdUnset):
        t.band("size")
    with pytest.raises(ThresholdUnset):
        t.epsilon("time_wall")
    assert t.bootstrap_resamples() == 10000
    assert t.guard() == {"max_loadavg_1m": None, "max_other_cpu_percent": None}


def test_threshold_override_merges(layout):
    t = Thresholds.load(layout.thresholds_path, {"families": {"size": {"practical_significance_band": {"kind": "ratio", "lower": 0.9, "upper": 1.1}}}})
    assert t.band("size") == ("ratio", 0.9, 1.1)
    assert "spec-override" in t.source
    with pytest.raises(ValueError):
        Thresholds.load(layout.thresholds_path, {"bootstrap": {"confidence": 0.9}}, decision_ids=["D"])


def test_repo_thresholds_file_is_template():
    from pathlib import Path

    p = Path(__file__).resolve().parents[2] / "methods" / "thresholds.json"
    data = json.loads(p.read_text())
    assert data["status"] == "template"
    assert "decision-method.md" in data["authority"]
    for fam in data["families"].values():
        assert fam["practical_significance_band"] == {"kind": None, "lower": None, "upper": None}
        assert fam["pareto_epsilon"] == {"kind": None, "value": None}
    assert set(data["families"]) == set(template()["families"])


# ---- held-out guard -----------------------------------------------------------------


def _heldout_spec(spec_dict):
    spec_dict["corpus"] = {"splits": ["heldout"]}
    return parse_spec(spec_dict)


def test_heldout_refused_without_design_freeze(spec_dict, layout):
    s = _heldout_spec(spec_dict)
    with pytest.raises(HeldoutLocked, match="does not exist"):
        check_heldout(s, layout.design_freeze_path, env={"EB_HELDOUT_UNLOCK": SHA})


def test_heldout_refused_without_token(spec_dict, layout):
    layout.design_freeze_path.write_text(json.dumps({"design_freeze_sha": SHA}))
    s = _heldout_spec(spec_dict)
    with pytest.raises(HeldoutLocked, match="not set"):
        check_heldout(s, layout.design_freeze_path, env={})


def test_heldout_refused_wrong_token(spec_dict, layout):
    layout.design_freeze_path.write_text(json.dumps({"design_freeze_sha": SHA}))
    s = _heldout_spec(spec_dict)
    with pytest.raises(HeldoutLocked, match="does not equal"):
        check_heldout(s, layout.design_freeze_path, env={"EB_HELDOUT_UNLOCK": "0" * 40})


def test_heldout_unlocked_with_matching_token(spec_dict, layout):
    layout.design_freeze_path.write_text(json.dumps({"commit_sha": SHA}))
    s = _heldout_spec(spec_dict)
    assert check_heldout(s, layout.design_freeze_path, env={"EB_HELDOUT_UNLOCK": SHA.upper()}) is True
    assert heldout_unlocked(layout.design_freeze_path, {"EB_HELDOUT_UNLOCK": SHA})


def test_non_heldout_spec_not_affected(spec_dict, layout):
    assert check_heldout(parse_spec(spec_dict), layout.design_freeze_path, env={}) is False


def test_resolve_items_never_lists_heldout_when_locked(spec_dict, layout):
    (layout.heldout_root / "secret-item").write_text("x")
    s = _heldout_spec(spec_dict)
    with pytest.raises(HeldoutLocked):
        resolve_items(s, layout, heldout_ok=False)


def test_symlink_into_heldout_refused(spec_dict, layout):
    (layout.heldout_root / "secret-item").write_text("x")
    dev = layout.corpus_root / "dev"
    dev.mkdir()
    try:
        (dev / "sneaky").symlink_to(layout.heldout_root / "secret-item")
    except (OSError, NotImplementedError):
        pytest.skip("symlinks unavailable")
    spec_dict["corpus"] = {"splits": ["dev"]}
    with pytest.raises(HeldoutLocked, match="inside the held-out root"):
        resolve_items(parse_spec(spec_dict), layout, heldout_ok=False)


def test_manifest_family_selection(spec_dict, layout, tmp_path):
    dev = layout.corpus_root / "dev"
    dev.mkdir()
    for name in ("a", "b", "c"):
        (dev / name).write_text(name)
    man = tmp_path / "manifest.jsonl"
    man.write_text("\n".join(json.dumps(r) for r in [
        {"item_id": "a", "split": "dev", "family": "text"},
        {"item_id": "b", "split": "dev", "family": "binary"},
        {"item_id": "c", "split": "dev", "family": "text"},
        {"item_id": "z", "split": "heldout", "family": "text"},
    ]))
    spec_dict["corpus"] = {"manifest": str(man), "families": ["text"]}
    items = resolve_items(parse_spec(spec_dict), layout, heldout_ok=False)
    assert [(i.split, i.item_id) for i in items] == [("dev", "a"), ("dev", "c")]


def test_families_without_manifest_rejected(spec_dict):
    spec_dict["corpus"] = {"splits": ["dev"], "families": ["text"]}
    with pytest.raises(SpecError, match="manifest"):
        parse_spec(spec_dict)


def test_reads_corpus_framework_manifest_via_materialized_relpath(spec_dict, layout, tmp_path):
    """corpus.manifest can point straight at research/corpus/tools' generated
    research/corpus/manifest.json: item paths come from materialized_relpath
    (relative to the data root), the same layout provision.py writes into it."""
    (layout.data_root / "corpus" / "tuning").mkdir(parents=True)
    (layout.data_root / "corpus" / "tuning" / "a").write_text("a")
    (layout.heldout_root / "z").write_text("z")
    man = tmp_path / "manifest.json"
    man.write_text(json.dumps({
        "manifest_format": "ebrc-manifest-v1",
        "items": [
            {"item_id": "a", "family": "F01", "split": "tuning", "status": "ok",
             "materialized_relpath": "corpus/tuning/a"},
            {"item_id": "z", "family": "F01", "split": "heldout", "status": "ok",
             "materialized_relpath": "heldout/z"},
        ],
    }))
    spec_dict["corpus"] = {"manifest": str(man)}
    items = resolve_items(parse_spec(spec_dict), layout, heldout_ok=False)
    assert [(i.split, i.item_id) for i in items] == [("tuning", "a")]
    assert items[0].path == layout.data_root / "corpus" / "tuning" / "a"


def test_heldout_refusal_delegates_to_corpuslib(spec_dict, layout):
    """The path-membership check in resolve_items() is corpuslib.assert_not_heldout(),
    not a second reimplementation (PROGRESS.md integration item)."""
    from ebr import corpus as corpus_mod

    cl = corpus_mod._corpuslib()
    assert cl.__name__ == "ebr._corpuslib"
    assert cl.assert_not_heldout.__module__ == "ebr._corpuslib"
    assert callable(cl.assert_not_heldout) and issubclass(cl.HeldoutAccessError, Exception)
    # and it is actually consulted: a manifest "path" override into the held-out root is refused.
    man_dir = layout.data_root / "man"
    man_dir.mkdir()
    (layout.heldout_root / "secret").write_text("x")
    man = man_dir / "manifest.jsonl"
    man.write_text(json.dumps({"item_id": "secret", "split": "dev", "path": str(layout.heldout_root / "secret")}))
    spec_dict["corpus"] = {"manifest": str(man)}
    with pytest.raises(HeldoutLocked, match="inside the held-out root"):
        resolve_items(parse_spec(spec_dict), layout, heldout_ok=False)
