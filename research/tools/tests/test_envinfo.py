import hashlib
import json
from pathlib import Path

import pytest

from ebr.envinfo import detect_kind, probe, resolve_env_id
from ebr.paths import Layout


def _canon(obj):
    return json.dumps(obj, sort_keys=True, separators=(",", ":"), ensure_ascii=False, allow_nan=False).encode()


def _register(env_dir: Path, name: str, stable: dict) -> str:
    env_id = hashlib.sha256(_canon(stable)).hexdigest()
    doc = {"schema": "entrybound.research.env-fingerprint/1", "env_id": env_id, "env_id_without_repo": "w" * 64,
           "platform_id": "p" * 64, "stable": stable, "capture": {}}
    (env_dir / f"{env_id}.json").write_text(json.dumps(doc))
    idx_path = env_dir / "index.json"
    idx = json.loads(idx_path.read_text()) if idx_path.exists() else {"schema": "entrybound.research.env-index/1", "environments": {}, "history": {}}
    idx["environments"][name] = env_id
    idx["history"].setdefault(name, []).append(env_id)
    idx_path.write_text(json.dumps(idx))
    return env_id


def _stable(kind, head="1" * 40):
    import platform

    return {"schema": "entrybound.research.env-fingerprint/1", "kind": kind,
            "platform": {"kernel": {"release": platform.release()}}, "repo": {"git": {"head_sha": head}}}


def test_unregistered_fallback(layout):
    env_id, d = resolve_env_id(layout)
    assert env_id.startswith(f"unregistered-{detect_kind()}-")
    assert d["registered"] is False


def test_resolves_by_kind_and_checks_integrity(layout, monkeypatch):
    env_dir = layout.environment_dir
    mine = _register(env_dir, "this-machine", _stable(detect_kind()))
    _register(env_dir, "other", _stable("some-other-kind"))
    env_id, d = resolve_env_id(layout)
    assert env_id == mine and d["registered"] and d["env_name"] == "this-machine"
    assert d["platform_id"] == "p" * 64
    assert d["warnings"] == []  # tmp repo has no git HEAD, kernel matches

    monkeypatch.setenv("EB_ENV_NAME", "other")
    env_id2, d2 = resolve_env_id(layout)
    assert env_id2 != mine and any("kind" in w for w in d2["warnings"])
    monkeypatch.delenv("EB_ENV_NAME")

    doc_path = env_dir / f"{mine}.json"
    doc = json.loads(doc_path.read_text())
    doc["stable"]["kind"] = detect_kind()
    doc["stable"]["extra"] = 1
    doc_path.write_text(json.dumps(doc))
    with pytest.raises(RuntimeError, match="hash-match"):
        resolve_env_id(layout)


def test_ambiguous_kind_requires_name(layout):
    _register(layout.environment_dir, "a", _stable(detect_kind(), "2" * 40))
    _register(layout.environment_dir, "b", _stable(detect_kind(), "3" * 40))
    with pytest.raises(RuntimeError, match="EB_ENV_NAME"):
        resolve_env_id(layout)


def test_forced_env_id(layout, monkeypatch):
    monkeypatch.setenv("EB_ENV_ID", "f" * 64)
    with pytest.raises(RuntimeError, match="no fingerprint"):
        resolve_env_id(layout)
    monkeypatch.setenv("EB_ENV_ALLOW_UNREGISTERED", "1")
    env_id, d = resolve_env_id(layout)
    assert env_id == "f" * 64 and not d["registered"]


def test_real_registry_resolves_if_present():
    repo = Path(__file__).resolve().parents[3]
    if not (repo / "research" / "environment" / "index.json").is_file():
        pytest.skip("no environment registry")
    layout = Layout(repo, None)
    idx = json.loads((repo / "research" / "environment" / "index.json").read_text())
    try:
        env_id, d = resolve_env_id(layout)
    except RuntimeError as exc:
        pytest.skip(f"registry not resolvable here: {exc}")
    if d["registered"]:
        assert env_id in idx["environments"].values()
    assert probe()["kind"] == detect_kind()
