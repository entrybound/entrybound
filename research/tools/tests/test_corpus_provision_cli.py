"""Corpus CLI safety checks using generated definitions and guarded layouts only."""

import copy
import hashlib
import importlib.util
import json
import os
import subprocess
import sys
from pathlib import Path

import pytest

if sys.platform != "linux":
    pytest.skip("corpus provisioning tools require Linux/WSL", allow_module_level=True)


TOOLS = Path(__file__).resolve().parents[2] / "corpus" / "tools"
spec = importlib.util.spec_from_file_location("corpus_provision_cli_test", TOOLS / "provision.py")
provision = importlib.util.module_from_spec(spec)
spec.loader.exec_module(provision)


def download(iid, family, split, group):
    return {
        "item_id": iid, "family": family, "split": split, "scale": "small",
        "kind": "download", "real_or_generated": "real", "independence_group": group,
        "description": "Synthetic definition; never acquired",
        "license": {"spdx_or_name": "MIT", "redistributable": True,
                    "attribution": "Synthetic test", "notes": ""},
        "recipe": {"inputs": [{"name": "blob", "url": f"https://example.invalid/{iid}.blob",
                               "sha256": hashlib.sha256(iid.encode()).hexdigest()}]},
    }


def derived(iid, source):
    item = download(iid, source["family"], source["split"], source["independence_group"])
    item.update(kind="derive", real_or_generated="derived-from-real")
    item["recipe"] = {"from_items": [source["item_id"]],
                      "steps": [{"op": "copy-item", "from_item": source["item_id"]}]}
    return item


class GuardedLayout:
    """Only generated source definitions and explicitly selected records exist."""

    def __init__(self, root):
        self.sources_dir = root / "sources"
        self.sources_dir.mkdir()
        self._records_dir = root / "records"
        self._records_dir.mkdir()
        self.allowed_records = set()
        self.record_reads = []

    def write_sources(self, items):
        (self.sources_dir / "synthetic.json").write_text(
            json.dumps({"schema": provision.cl.SOURCES_SCHEMA, "items": items}), encoding="utf-8")

    def record_path(self, iid):
        assert iid in self.allowed_records, f"unselected record requested: {iid}"
        self.record_reads.append(iid)
        return self._records_dir / f"{iid}.json"

    def __getattr__(self, name):
        raise AssertionError(f"payload or unapproved layout access: {name}")


def forbidden(*args, **kwargs):
    raise AssertionError("payload, orphan, fingerprint or provisioning access")


@pytest.fixture
def catalog(tmp_path, monkeypatch):
    layout = GuardedLayout(tmp_path)
    beta = download("beta-validation", "F02", "validation", "beta")
    items = [download("alpha-tuning", "F01", "tuning", "alpha"), beta,
             download("gamma-heldout", "F03", "heldout", "gamma"),
             download("other-tuning", "F02", "tuning", "other"),
             derived("derived-validation", beta)]
    layout.write_sources(items)
    monkeypatch.setattr(provision.cl, "Layout", lambda *args: layout)
    monkeypatch.setattr(provision.cl, "require_linux", lambda: None)
    monkeypatch.setattr(provision, "find_orphans", forbidden)
    monkeypatch.setattr(provision.fpm, "fingerprint_path", forbidden)
    monkeypatch.setattr(provision, "provision_item", forbidden)
    monkeypatch.setattr(provision.cl, "load_json_if_exists", forbidden)
    return layout, items


@pytest.mark.parametrize("args", [["--check"], ["--check", "--list"],
                                  ["--check", "--item", "gamma-heldout", "--split", "heldout"]])
def test_check_is_count_only_and_never_reads_records_or_payload(catalog, capsys, args):
    layout, items = catalog
    assert provision.main(args) == 0
    output = capsys.readouterr().out
    assert output.strip() == "OK: 5 item(s) in 1 source file(s)"
    assert layout.record_reads == []
    assert all(it["item_id"] not in output for it in items)


@pytest.mark.parametrize("args, selected", [
    ([], {"alpha-tuning", "beta-validation", "other-tuning", "derived-validation"}),
    (["--family", "F02", "--group", "beta", "--split", "validation"],
     {"beta-validation", "derived-validation"}),
    (["--family", " F01,F02 ", "--family", "F03", "--group", "alpha,beta", "--group", "gamma",
      "--item", "alpha-tuning,gamma-heldout", "--item", "beta-validation",
      "--split", "tuning", "--split", "validation,heldout"],
     {"alpha-tuning", "beta-validation", "gamma-heldout"}),
    (["--item", "gamma-heldout"], set()),
    (["--split", "heldout"], {"gamma-heldout"}),
    (["--item", "derived-validation"], {"derived-validation"}),
    (["--split", "tuning", "--family", "F02"], {"other-tuning"}),
])
def test_list_reads_only_filtered_records_without_payload_or_dependencies(catalog, capsys, args, selected):
    layout, items = catalog
    layout.allowed_records = selected
    if selected:
        iid = sorted(selected)[0]
        (layout._records_dir / f"{iid}.json").write_text('{"synthetic": true}', encoding="utf-8")
    assert provision.main(["--list", *args]) == 0
    output = capsys.readouterr().out
    assert set(layout.record_reads) == selected
    assert len(layout.record_reads) == len(selected)
    assert f"OK: {len(selected)} item(s) selected" in output
    assert all((it["item_id"] in output) == (it["item_id"] in selected) for it in items)
    assert "not-materialized" not in output
    if selected:
        assert "[recorded]" in output
    if len(selected) > 1:
        assert "[not-recorded]" in output


@pytest.mark.parametrize("mode", ["--check", "--list"])
@pytest.mark.parametrize("errors", [[], ["synthetic private-id /protected/path https://private.invalid/"]])
def test_metadata_source_diagnostics_are_aggregate(catalog, monkeypatch, capsys, mode, errors):
    layout, _ = catalog
    items, real_errors, _ = provision.cl.load_sources(layout)
    assert not real_errors
    warning = "synthetic private-id private-group /protected/path https://private.invalid/"
    monkeypatch.setattr(provision.cl, "load_sources", lambda _: (items, errors, [warning]))
    args = [mode] if mode == "--check" else [mode, "--family", "F20"]
    assert provision.main(args) == (2 if errors else 0)
    output = capsys.readouterr().out
    assert "1 source definition warning(s)" in output
    assert "source definition error(s)" in output if errors else "OK:" in output
    assert all(secret not in output for secret in ("private-id", "private-group", "/protected/path",
                                                   "https://private.invalid/"))
    assert layout.record_reads == []


@pytest.mark.parametrize("mode", ["--check", "--list"])
@pytest.mark.parametrize("invalid", ["missing", "cycle", "cross-split", "upstream", "malformed"])
def test_metadata_preserves_real_source_validation_refusals(catalog, capsys, mode, invalid):
    layout, original = catalog
    items = copy.deepcopy(original)
    if invalid == "missing":
        items[-1]["recipe"]["from_items"] = ["missing-private-id"]
        items[-1]["recipe"]["steps"][0]["from_item"] = "missing-private-id"
    elif invalid == "cycle":
        items[1] = derived("beta-validation", items[-1])
    elif invalid == "cross-split":
        items[-1] = derived("derived-validation", items[2])
        items[-1]["split"] = "validation"
    elif invalid == "upstream":
        items[2]["recipe"]["inputs"] = copy.deepcopy(items[0]["recipe"]["inputs"])
    layout.write_sources(items)
    if invalid == "malformed":
        (layout.sources_dir / "synthetic.json").write_text('{"private-id":', encoding="utf-8")
    assert provision.main([mode]) == 2
    output = capsys.readouterr().out
    assert "source definition error(s)" in output
    assert layout.record_reads == []
    assert all(it["item_id"] not in output for it in original)
    assert "missing-private-id" not in output
    assert str(layout.sources_dir) not in output


def test_list_unknown_ids_are_aggregate(catalog, capsys):
    layout, _ = catalog
    assert provision.main(["--list", "--item", "unknown-private-id,another-private-id"]) == 2
    assert capsys.readouterr().out.strip() == "ERROR: 2 unknown item(s)"
    assert not layout.record_reads


@pytest.mark.parametrize("mode", ["--check", "--list", "ordinary"])
@pytest.mark.parametrize("field, value", [
    ("independence_group", []),
    ("from_items", 7), ("from_items", None), ("from_items", 0), ("from_items", False),
    ("from_items", {}), ("from_items", "typed-private-id"), ("from_items", [["typed-private-id"]]),
    ("inputs", 7), ("inputs", None), ("subpaths", 7), ("subpaths", None),
    ("family", []), ("scale", []), ("kind", []), ("step_input", []), ("steps", False),
])
def test_invalid_definition_types_fail_before_graph_or_dispatch(catalog, monkeypatch, capsys, mode, field, value):
    layout, original = catalog
    items = copy.deepcopy(original)
    if field == "from_items":
        items[-1]["recipe"][field] = value
    elif field in ("inputs", "steps"):
        items[0]["recipe"][field] = value
    elif field == "subpaths":
        items[0]["kind"] = "git-archive"
        items[0]["recipe"] = {"git": {"repo": "https://private.invalid/typed-private-id.git",
                                     "commit": "1" * 40, "subpaths": value}}
    elif field == "step_input":
        items[0]["recipe"]["steps"] = [{"op": "copy", "input": value}]
    else:
        items[0][field] = value
    layout.write_sources(items)
    monkeypatch.setattr(provision.cl, "upstream_keys", forbidden)
    monkeypatch.setattr(provision.cl, "dependency_order", forbidden)
    args = ["--offline"] if mode == "ordinary" else [mode]
    assert provision.main(args) == 2
    captured = capsys.readouterr()
    assert "source definition error(s)" in captured.out
    assert "Traceback" not in captured.out + captured.err
    assert not layout.record_reads
    if mode == "ordinary":
        assert "ERROR:" in captured.out
        assert str(layout.sources_dir) in captured.out
    else:
        assert "ERROR:" not in captured.out
        assert all(it["item_id"] not in captured.out for it in original)
        assert all(secret not in captured.out + captured.err for secret in
                   (str(layout.sources_dir), "typed-private-id", "https://private.invalid/"))


@pytest.mark.parametrize("contents", ["{}", "not valid json"])
def test_list_record_presence_does_not_parse_selected_contents(catalog, capsys, contents):
    layout, _ = catalog
    layout.allowed_records = {"alpha-tuning"}
    (layout._records_dir / "alpha-tuning.json").write_text(contents, encoding="utf-8")
    assert provision.main(["--list", "--item", "alpha-tuning"]) == 0
    assert "[recorded]" in capsys.readouterr().out
    assert layout.record_reads == ["alpha-tuning"]


@pytest.mark.parametrize("entry", ["symlink", "directory", "inaccessible"])
def test_list_refuses_nonregular_or_inaccessible_selected_records_without_following(catalog, monkeypatch,
                                                                                 capsys, entry):
    layout, _ = catalog
    layout.allowed_records = {"alpha-tuning"}
    record = layout._records_dir / "alpha-tuning.json"
    if entry == "symlink":
        target = layout._records_dir / "forbidden-target"
        record.symlink_to(target)
        original_stat = Path.stat

        def no_target_stat(path, *args, **kwargs):
            assert path != target, "record target inspected"
            assert path != record or kwargs.get("follow_symlinks") is False, "record symlink followed"
            return original_stat(path, *args, **kwargs)

        monkeypatch.setattr(Path, "stat", no_target_stat)
    elif entry == "directory":
        record.mkdir()
    else:
        original_lstat = Path.lstat

        def inaccessible(path, *args, **kwargs):
            if path == record:
                raise PermissionError("synthetic private path")
            return original_lstat(path, *args, **kwargs)

        monkeypatch.setattr(Path, "lstat", inaccessible)
    assert provision.main(["--list", "--item", "alpha-tuning"]) == 2
    captured = capsys.readouterr()
    assert captured.out.strip() == "ERROR: 1 selected metadata record(s) could not be inspected"
    assert not captured.err
    assert layout.record_reads == ["alpha-tuning"]


def test_materialization_keeps_dependency_closure_and_offline_options(catalog, monkeypatch, capsys):
    layout, _ = catalog
    calls = []

    def fake_provision(item, all_items, passed_layout, args):
        calls.append(item["item_id"])
        assert passed_layout is layout
        assert args.offline and args.reverify_cache and args.verify == "full"
        assert args.jobs == 1 and args.hash_jobs == 3
        return "synthetic dispatch only"

    monkeypatch.setattr(provision, "provision_item", fake_provision)
    assert provision.main(["--item", "derived-validation", "--offline", "--reverify-cache",
                           "--verify", "full", "--jobs", "1", "--hash-jobs", "3"]) == 0
    assert calls == ["beta-validation", "derived-validation"]
    assert "included as dependency" in capsys.readouterr().out
    assert not layout.record_reads


def test_materialization_default_selection_and_detailed_errors_are_preserved(catalog, monkeypatch, capsys):
    layout, items = catalog
    calls = []
    monkeypatch.setattr(provision, "provision_item", lambda it, *_: calls.append(it["item_id"]) or "stub")
    assert provision.main(["--jobs", "1"]) == 0
    assert set(calls) == {it["item_id"] for it in items}
    assert calls.index("beta-validation") < calls.index("derived-validation")
    capsys.readouterr()
    monkeypatch.setattr(provision.cl, "load_sources", lambda _: ({}, ["synthetic detailed error"],
                                                                ["synthetic detailed warning"]))
    calls.clear()
    assert provision.main(["--offline"]) == 2
    output = capsys.readouterr().out
    assert "WARNING: synthetic detailed warning" in output
    assert "ERROR: synthetic detailed error" in output
    assert not calls and not layout.record_reads


@pytest.fixture
def wrapper(tmp_path):
    """Redirect the copied wrapper's fixed shared root to a generated sandbox."""
    tools = tmp_path / "repo" / "research" / "corpus" / "tools"
    tools.mkdir(parents=True)
    shared = tmp_path / "shared"
    guard_bin = tmp_path / "guard-bin"
    guard_bin.mkdir()
    trace = tmp_path / "setup-trace"
    script = (TOOLS / "run.sh").read_text(encoding="utf-8")
    assert script.count("SHARED=/root/eb-research") == 1
    (tools / "run.sh").write_text(script.replace("SHARED=/root/eb-research", f"SHARED='{shared}'"),
                                  encoding="utf-8")
    (tools / "provision.py").write_text(
        "import json, os, sys\nprint(json.dumps({'args': sys.argv[1:], "
        "'no_bytecode': os.environ.get('PYTHONDONTWRITEBYTECODE')}))\n", encoding="utf-8")
    for name in ("mkdir", "chmod", "flock"):
        guard = guard_bin / name
        guard.write_text(f"#!/bin/sh\nprintf '%s\\n' '{name}' >> '{trace}'\n"
                         'test "${ALLOW_SETUP:-}" = yes || exit 97\n', encoding="utf-8")
        guard.chmod(0o755)
    python = guard_bin / "python3"
    python.write_text(f"#!/bin/sh\nexec '{sys.executable}' \"$@\"\n", encoding="utf-8")
    python.chmod(0o755)
    env = dict(os.environ, PATH=f"{guard_bin}:/usr/bin:/bin", EB_DATA_ROOT=str(tmp_path / "payload"))
    return tools, shared, python, trace, env


@pytest.mark.parametrize("flag", ["--check", "--list", "--ch", "--che", "--chec", "--l", "--li", "--lis",
                                  "-h", "--he", "--hel", "--help"])
@pytest.mark.parametrize("existing_venv", [False, True])
def test_wrapper_metadata_dispatch_never_sets_up_payload_or_venv(wrapper, flag, existing_venv):
    tools, shared, python, trace, env = wrapper
    if existing_venv:
        venv_bin = shared / "venv" / "bin"
        venv_bin.mkdir(parents=True)
        (venv_bin / "python").symlink_to(python)
    result = subprocess.run(["bash", str(tools / "run.sh"), "provision", "--family", "F02", flag],
                            env=env, text=True, capture_output=True)
    assert result.returncode == 0, result.stderr
    assert json.loads(result.stdout) == {"args": ["--family", "F02", flag], "no_bytecode": "1"}
    assert not trace.exists()
    assert not Path(env["EB_DATA_ROOT"]).exists()
    assert shared.exists() == existing_venv


@pytest.mark.parametrize("flag", ["-h", "--he", "--hel", "--help"])
def test_provision_help_never_loads_definitions_or_layout(monkeypatch, capsys, flag):
    monkeypatch.setattr(provision.cl, "Layout", forbidden)
    monkeypatch.setattr(provision.cl, "load_sources", forbidden)
    monkeypatch.setattr(provision.cl, "require_linux", forbidden)
    with pytest.raises(SystemExit) as exited:
        provision.main([flag])
    assert exited.value.code == 0
    assert "usage:" in capsys.readouterr().out


def test_wrapper_materialization_retains_setup(wrapper):
    tools, shared, python, trace, env = wrapper
    venv_bin = shared / "venv" / "bin"
    venv_bin.mkdir(parents=True)
    (venv_bin / "python").symlink_to(python)
    env["ALLOW_SETUP"] = "yes"
    result = subprocess.run(["bash", str(tools / "run.sh"), "provision", "--offline"],
                            env=env, text=True, capture_output=True)
    assert result.returncode == 0, result.stderr
    assert trace.read_text().splitlines() == ["mkdir", "chmod"]
    assert json.loads(result.stdout)["args"] == ["--offline"]
