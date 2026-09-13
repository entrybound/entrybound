import hashlib
import os

import pytest

from conftest import make_spec
from ebr.corpus import FingerprintCache, compute_fingerprint, generate_bytes, materialize_generated, prepare_items, resolve_items
from ebr.spec import GeneratedItem, SpecError, parse_spec

SMOKE_SHA = "007dc5aa7af2aba9ed0b571263101a6008fcb13a3a128633f9c2b123f8731048"


def test_generators_deterministic():
    t = generate_bytes("text-v1", 1 << 20, 0)
    assert len(t) == 1 << 20
    assert hashlib.sha256(t).hexdigest() == SMOKE_SHA
    assert generate_bytes("random-v1", 1000, 1) == generate_bytes("random-v1", 1000, 1)
    assert generate_bytes("random-v1", 1000, 1) != generate_bytes("random-v1", 1000, 2)
    assert generate_bytes("zeros-v1", 10, 0) == bytes(10)
    assert generate_bytes("random-v1", 0, 0) == b""


def test_materialize_idempotent_and_loud(layout):
    g = GeneratedItem("x", "random-v1", 4096, 3, None, None)
    p = materialize_generated(g, "EXP-T", layout)
    first = p.stat().st_mtime_ns
    assert materialize_generated(g, "EXP-T", layout) == p
    assert p.stat().st_mtime_ns == first
    p.write_bytes(b"tampered")
    with pytest.raises(RuntimeError, match="exists with sha256"):
        materialize_generated(g, "EXP-T", layout)
    bad = GeneratedItem("y", "random-v1", 4096, 3, None, "0" * 64)
    with pytest.raises(RuntimeError, match="spec expects"):
        materialize_generated(bad, "EXP-T", layout)


def test_tree_fingerprint(tmp_path):
    root = tmp_path / "item"
    (root / "sub").mkdir(parents=True)
    (root / "a.txt").write_bytes(b"alpha")
    (root / "sub" / "b.bin").write_bytes(b"\x00" * 100)
    fp1 = compute_fingerprint(root)
    assert fp1["type"] == "tree" and fp1["files"] == 2 and fp1["bytes"] == 105 and fp1["dirs"] == 1
    assert compute_fingerprint(root) == fp1
    (root / "sub" / "b.bin").write_bytes(b"\x00" * 99 + b"\x01")
    fp2 = compute_fingerprint(root)
    assert fp2["sha256"] != fp1["sha256"] and fp2["bytes"] == 105
    (root / "sub" / "b.bin").rename(root / "sub" / "c.bin")
    assert compute_fingerprint(root)["sha256"] not in (fp1["sha256"], fp2["sha256"])
    f = compute_fingerprint(root / "a.txt")
    assert f == {"version": "tree-sha256-v1", "type": "file", "sha256": hashlib.sha256(b"alpha").hexdigest(), "bytes": 5, "files": 1}


def test_fingerprint_cache_invalidates(tmp_path):
    item = tmp_path / "f"
    item.write_bytes(b"one")
    cache = FingerprintCache(tmp_path / "cache" / "fp.sqlite")
    a = cache.get(item)
    assert cache.get(item) == a
    item.write_bytes(b"two!")
    os.utime(item, ns=(1, 1))
    b = cache.get(item)
    assert b["sha256"] == hashlib.sha256(b"two!").hexdigest()


def test_split_selection_and_missing_items(spec_dict, layout):
    dev = layout.corpus_root / "dev"
    dev.mkdir()
    (dev / "one").write_bytes(b"1")
    (dev / "two").mkdir()
    (dev / "two" / "x").write_bytes(b"22")
    (dev / ".hidden").write_bytes(b"")
    spec_dict["corpus"] = {"splits": ["dev"]}
    s = parse_spec(spec_dict)
    items = resolve_items(s, layout, heldout_ok=False)
    assert [i.item_id for i in items] == ["one", "two"]
    fps = prepare_items(s, layout, items, FingerprintCache(None))
    assert fps["dev/two"]["bytes"] == 2
    spec_dict["corpus"] = {"splits": ["dev"], "item_ids": ["one", "nope"]}
    with pytest.raises(SpecError, match="not found"):
        resolve_items(parse_spec(spec_dict), layout, heldout_ok=False)
