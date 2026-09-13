"""Corpus selection, deterministic generated items, and item fingerprints.

Layout (outside git): ``<data_root>/corpus/<split>/<item_id>`` (file or directory),
held-out items at ``<data_root>/heldout/<item_id>``, generated items at
``<data_root>/generated/<experiment_id>/<item_id>``.

The held-out directory is never listed, stat'ed or hashed here unless the spec's
held-out guard has been passed (see :func:`ebr.spec.check_heldout`).
"""

from __future__ import annotations

import csv
import hashlib
import json
import os
import sqlite3
import stat
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Dict, Iterable, List, Optional

from .paths import Layout
from .spec import HELDOUT_SPLIT, GeneratedItem, HeldoutLocked, Spec, SpecError

FINGERPRINT_VERSION = "tree-sha256-v1"

_WORDS = (
    "archive entry bound stream block codec frame index chunk table header footer record "
    "metadata extent sparse link path mode owner group time size hash digest verify extract "
    "create append delta dictionary window match literal offset length token symbol state "
    "the of and to in is that for with as on by from at this be are was it or an which "
    "data file tree node leaf root page map list set key value order level ratio rate"
).split()


@dataclass
class Item:
    item_id: str
    split: str
    family: Optional[str]
    path: Path
    generated: bool = False

    @property
    def is_heldout(self) -> bool:
        return self.split == HELDOUT_SPLIT


# ----------------------------------------------------------------------------
# generated items


def _counter_stream(tag: bytes, seed: int) -> Iterable[bytes]:
    i = 0
    s = seed.to_bytes(8, "little")
    while True:
        yield hashlib.sha256(tag + s + i.to_bytes(8, "little")).digest()
        i += 1


def generate_bytes(generator: str, nbytes: int, seed: int) -> bytes:
    """Deterministic, library-version-independent synthetic content."""
    if generator == "zeros-v1":
        return bytes(nbytes)
    if generator == "random-v1":
        out = bytearray()
        for block in _counter_stream(b"ebr-gen-random-v1", seed):
            out += block
            if len(out) >= nbytes:
                break
        return bytes(out[:nbytes])
    if generator == "text-v1":
        out = bytearray()
        nwords = 0
        for block in _counter_stream(b"ebr-gen-text-v1", seed):
            for b in block:
                out += _WORDS[b % len(_WORDS)].encode("ascii")
                nwords += 1
                if nwords % 13 == 0:
                    out += b".\n"
                else:
                    out += b" "
            if len(out) >= nbytes:
                break
        return bytes(out[:nbytes])
    raise SpecError(f"unknown generator {generator}")


def materialize_generated(g: GeneratedItem, experiment_id: str, layout: Layout) -> Path:
    """Create (or verify) a generated item; idempotent; fails loudly on mismatch."""
    root = layout.generated_root / experiment_id
    root.mkdir(parents=True, exist_ok=True)
    path = root / g.item_id
    data = generate_bytes(g.generator, g.bytes, g.seed)
    digest = hashlib.sha256(data).hexdigest()
    if g.sha256 is not None and digest != g.sha256:
        raise RuntimeError(
            f"generated item {g.item_id}: generator produced sha256 {digest}, spec expects {g.sha256}"
        )
    if path.exists():
        existing = sha256_file(path)
        if existing != digest:
            raise RuntimeError(f"generated item {path} exists with sha256 {existing}, expected {digest}")
        return path
    tmp = path.with_name(path.name + ".tmp")
    with open(tmp, "wb") as fh:
        fh.write(data)
        fh.flush()
        os.fsync(fh.fileno())
    os.replace(tmp, path)
    return path


# ----------------------------------------------------------------------------
# fingerprints


def sha256_file(path: os.PathLike | str) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as fh:
        for chunk in iter(lambda: fh.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def _walk(root: Path) -> List[tuple]:
    """Sorted (relpath_bytes, lstat) for every entry below root (not following symlinks)."""
    entries = []
    stack = [b""]
    root_b = os.fsencode(root)
    while stack:
        rel = stack.pop()
        full = os.path.join(root_b, rel) if rel else root_b
        with os.scandir(full) as it:
            for de in it:
                r = os.path.join(rel, de.name) if rel else de.name
                st = de.stat(follow_symlinks=False)
                entries.append((r.replace(b"\\", b"/"), st))
                if stat.S_ISDIR(st.st_mode):
                    stack.append(r)
    entries.sort(key=lambda e: e[0])
    return entries


def _stat_signature(path: Path) -> str:
    st = path.lstat()
    h = hashlib.sha256()
    h.update(f"{st.st_mode}:{st.st_size}:{st.st_mtime_ns}:{st.st_ino}".encode())
    if stat.S_ISDIR(st.st_mode):
        for rel, est in _walk(path):
            h.update(rel + f"\0{est.st_mode}:{est.st_size}:{est.st_mtime_ns}:{est.st_ino}\n".encode())
    return h.hexdigest()


def compute_fingerprint(path: Path) -> Dict[str, Any]:
    st = path.lstat()
    if stat.S_ISREG(st.st_mode):
        return {"version": FINGERPRINT_VERSION, "type": "file", "sha256": sha256_file(path), "bytes": st.st_size, "files": 1}
    if not stat.S_ISDIR(st.st_mode):
        raise RuntimeError(f"corpus item {path} is neither a regular file nor a directory")
    h = hashlib.sha256()
    nbytes = files = dirs = links = 0
    for rel, est in _walk(path):
        full = os.path.join(os.fsencode(path), rel)
        if stat.S_ISREG(est.st_mode):
            d = sha256_file(full)
            h.update(b"F\0" + rel + b"\0" + str(est.st_size).encode() + b"\0" + d.encode() + b"\n")
            nbytes += est.st_size
            files += 1
        elif stat.S_ISDIR(est.st_mode):
            h.update(b"D\0" + rel + b"\n")
            dirs += 1
        elif stat.S_ISLNK(est.st_mode):
            h.update(b"L\0" + rel + b"\0" + os.readlink(full) + b"\n")
            links += 1
        else:
            h.update(b"O\0" + rel + b"\0" + str(stat.S_IFMT(est.st_mode)).encode() + b"\n")
    return {
        "version": FINGERPRINT_VERSION,
        "type": "tree",
        "sha256": h.hexdigest(),
        "bytes": nbytes,
        "files": files,
        "dirs": dirs,
        "symlinks": links,
    }


class FingerprintCache:
    """sqlite cache keyed on (path, stat signature); disabled when path is None."""

    def __init__(self, db_path: Optional[Path]):
        self.db_path = None if os.environ.get("EB_NO_FINGERPRINT_CACHE") else db_path

    def get(self, path: Path) -> Dict[str, Any]:
        if self.db_path is None:
            return compute_fingerprint(path)
        self.db_path.parent.mkdir(parents=True, exist_ok=True)
        sig = _stat_signature(path)
        key = str(path.resolve())
        con = sqlite3.connect(str(self.db_path), timeout=60)
        try:
            con.execute("CREATE TABLE IF NOT EXISTS fp (path TEXT PRIMARY KEY, sig TEXT, version TEXT, json TEXT)")
            row = con.execute("SELECT sig, version, json FROM fp WHERE path=?", (key,)).fetchone()
            if row and row[0] == sig and row[1] == FINGERPRINT_VERSION:
                return json.loads(row[2])
            fp = compute_fingerprint(path)
            with con:
                con.execute("REPLACE INTO fp VALUES (?,?,?,?)", (key, sig, FINGERPRINT_VERSION, json.dumps(fp)))
            return fp
        finally:
            con.close()


# ----------------------------------------------------------------------------
# selection


def _load_manifest(path: Path) -> List[Dict[str, Any]]:
    text = path.read_text(encoding="utf-8")
    if path.suffix == ".jsonl":
        rows = [json.loads(line) for line in text.splitlines() if line.strip()]
    elif path.suffix == ".json":
        data = json.loads(text)
        rows = data["items"] if isinstance(data, dict) else data
    elif path.suffix == ".csv":
        rows = list(csv.DictReader(line for line in text.splitlines() if not line.startswith("#")))
    else:
        raise SpecError(f"manifest {path}: use .json, .jsonl or .csv")
    for r in rows:
        if "item_id" not in r or "split" not in r:
            raise SpecError(f"manifest {path}: every row needs item_id and split")
    return rows


def _inside(child: Path, parent: Path) -> bool:
    try:
        child.resolve().relative_to(parent.resolve())
        return True
    except (ValueError, OSError):
        return False


def resolve_items(spec: Spec, layout: Layout, heldout_ok: bool) -> List[Item]:
    """Resolve the spec's corpus selector to concrete items (sorted by split, item_id)."""
    sel = spec.corpus
    if sel.wants_heldout and not heldout_ok:
        raise HeldoutLocked(f"{spec.experiment_id}: held-out split selected without unlock")
    corpus_root = Path(sel.root) if sel.root else None
    items: Dict[tuple, Item] = {}

    manifest_rows = None
    if sel.manifest:
        manifest_rows = _load_manifest(Path(sel.manifest))

    def split_dir(split: str) -> Path:
        if split == HELDOUT_SPLIT:
            return layout.heldout_root
        return (corpus_root or layout.corpus_root) / split

    if manifest_rows is not None:
        for r in manifest_rows:
            split = r["split"]
            if sel.splits and split not in sel.splits:
                continue
            if split == HELDOUT_SPLIT and not heldout_ok:
                continue  # never touch held-out rows without unlock
            if sel.item_ids and r["item_id"] not in sel.item_ids:
                continue
            fam = r.get("family") or None
            if sel.families and fam not in sel.families:
                continue
            p = Path(r["path"]) if r.get("path") else split_dir(split) / r["item_id"]
            items[(split, r["item_id"])] = Item(r["item_id"], split, fam, p)
    else:
        for split in sel.splits:
            d = split_dir(split)
            if not d.is_dir():
                raise SpecError(f"corpus split directory {d} does not exist")
            names = sorted(os.listdir(d))
            for name in names:
                if name.startswith(".") or name.endswith((".tmp", ".part")):
                    continue
                if sel.item_ids and name not in sel.item_ids:
                    continue
                items[(split, name)] = Item(name, split, None, d / name)
        if sel.item_ids:
            found = {k[1] for k in items}
            missing = [i for i in sel.item_ids if i not in found]
            if missing:
                raise SpecError(f"corpus item_ids not found in splits {sel.splits}: {missing}")

    for g in sel.generated:
        path = layout.generated_root / spec.experiment_id / g.item_id
        items[("generated", g.item_id)] = Item(g.item_id, "generated", g.family, path, generated=True)

    out = sorted(items.values(), key=lambda it: (it.split, it.item_id))
    if not out:
        raise SpecError(f"{spec.experiment_id}: corpus selector matched no items")
    if not heldout_ok and layout.data_root is not None:
        hroot = layout.heldout_root
        for it in out:
            if _inside(it.path, hroot):
                raise HeldoutLocked(f"item {it.item_id} resolves inside the held-out root {hroot}")
    return out


def prepare_items(spec: Spec, layout: Layout, items: List[Item], cache: FingerprintCache) -> Dict[str, Dict[str, Any]]:
    """Materialize generated items and fingerprint every item. Keyed by ``split/item_id``."""
    gens = {g.item_id: g for g in spec.corpus.generated}
    fps: Dict[str, Dict[str, Any]] = {}
    for it in items:
        if it.generated:
            materialize_generated(gens[it.item_id], spec.experiment_id, layout)
        if not it.path.exists():
            raise SpecError(f"corpus item {it.item_id} missing at {it.path}")
        fps[f"{it.split}/{it.item_id}"] = cache.get(it.path)
    return fps
