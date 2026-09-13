"""Raw JSONL(.gz) writing, reading and verification.

Each run writes ``research/raw/<experiment_id>/<run_id>.jsonl`` (or ``.jsonl.gz``)
with one canonical-JSON row per sample and a sidecar ``<run_id>.meta.json``.
Rows are hash-chained: ``row_sha256`` = SHA-256 of the canonical row without
``row_sha256``; ``prev_row_sha256`` links to the previous row (64 zeros first).
On completion the meta file records the raw file's SHA-256 and byte count.
"""

from __future__ import annotations

import gzip
import hashlib
import json
import math
import os
import re
from pathlib import Path
from typing import Any, Dict, Iterator, List, Optional, Tuple

from . import META_SCHEMA, RAW_SCHEMA

ZERO_DIGEST = "0" * 64
HEX64 = re.compile(r"^[0-9a-f]{64}$")

REQUIRED_ROW_KEYS = (
    "schema", "experiment_id", "run_id", "seq", "phase", "candidate", "item_id", "split", "repetition",
    "command", "exit_code", "ok", "valid", "stdout_sha256", "stderr_sha256", "metrics", "timestamps",
    "env_id", "git_sha", "item_fingerprint", "timing", "row_sha256", "prev_row_sha256",
)


def _clean(obj: Any) -> Any:
    if isinstance(obj, float):
        return obj if math.isfinite(obj) else None
    if isinstance(obj, dict):
        return {str(k): _clean(v) for k, v in obj.items()}
    if isinstance(obj, (list, tuple)):
        return [_clean(v) for v in obj]
    if isinstance(obj, Path):
        return obj.as_posix()
    return obj


def canonical(obj: Any) -> bytes:
    return json.dumps(_clean(obj), sort_keys=True, separators=(",", ":"), ensure_ascii=False, allow_nan=False).encode("utf-8")


def row_digest(row: Dict[str, Any]) -> str:
    body = {k: v for k, v in row.items() if k != "row_sha256"}
    return hashlib.sha256(canonical(body)).hexdigest()


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as fh:
        for chunk in iter(lambda: fh.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def raw_path(raw_dir: Path, run_id: str, compression: str) -> Path:
    return raw_dir / (f"{run_id}.jsonl.gz" if compression == "gzip" else f"{run_id}.jsonl")


def meta_path(raw_dir: Path, run_id: str) -> Path:
    return raw_dir / f"{run_id}.meta.json"


class RawWriter:
    def __init__(self, path: Path):
        self.path = path
        self.prev = ZERO_DIGEST
        self.count = 0
        if path.exists():
            raise FileExistsError(f"raw file {path} already exists (run ids must be unique)")
        path.parent.mkdir(parents=True, exist_ok=True)
        self.gz = path.suffix == ".gz"

    def append(self, row: Dict[str, Any]) -> Dict[str, Any]:
        row = _clean(dict(row))
        row["schema"] = RAW_SCHEMA
        row["prev_row_sha256"] = self.prev
        row.pop("row_sha256", None)
        row["row_sha256"] = row_digest(row)
        line = canonical(row) + b"\n"
        if self.gz:
            # one gzip member per row (valid multi-member gzip); mtime=0 for reproducibility
            with open(self.path, "ab") as raw:
                with gzip.GzipFile(filename="", fileobj=raw, mode="wb", mtime=0) as gz:
                    gz.write(line)
                raw.flush()
                os.fsync(raw.fileno())
        else:
            with open(self.path, "ab") as fh:
                fh.write(line)
                fh.flush()
                os.fsync(fh.fileno())
        self.prev = row["row_sha256"]
        self.count += 1
        return row


def read_rows(path: Path) -> Iterator[Dict[str, Any]]:
    opener = gzip.open if path.suffix == ".gz" else open
    with opener(path, "rb") as fh:
        for lineno, line in enumerate(fh, 1):
            if not line.strip():
                continue
            try:
                yield json.loads(line)
            except ValueError as exc:
                raise ValueError(f"{path}:{lineno}: invalid JSON: {exc}") from exc


def write_json(path: Path, data: Dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = path.with_name(path.name + ".tmp")
    with open(tmp, "w", encoding="utf-8", newline="\n") as fh:
        json.dump(_clean(data), fh, indent=2, sort_keys=True, ensure_ascii=False)
        fh.write("\n")
    os.replace(tmp, path)


def list_runs(raw_dir: Path) -> List[str]:
    if not raw_dir.is_dir():
        return []
    return sorted(p.name[: -len(".meta.json")] for p in raw_dir.glob("*.meta.json"))


def load_meta(raw_dir: Path, run_id: str) -> Dict[str, Any]:
    with open(meta_path(raw_dir, run_id), encoding="utf-8") as fh:
        return json.load(fh)


class VerifyReport:
    def __init__(self, run_id: str):
        self.run_id = run_id
        self.errors: List[str] = []
        self.warnings: List[str] = []
        self.rows = 0

    @property
    def ok(self) -> bool:
        return not self.errors

    def to_dict(self) -> Dict[str, Any]:
        return {"run_id": self.run_id, "ok": self.ok, "rows": self.rows, "errors": self.errors, "warnings": self.warnings}


def verify_run(raw_dir: Path, run_id: str, *, allow_incomplete: bool = False, design_freeze_sha: Optional[str] = None,
               refingerprint=None) -> VerifyReport:
    rep = VerifyReport(run_id)
    try:
        meta = load_meta(raw_dir, run_id)
    except (OSError, ValueError) as exc:
        rep.errors.append(f"meta unreadable: {exc}")
        return rep
    if meta.get("schema") != META_SCHEMA:
        rep.errors.append(f"meta schema {meta.get('schema')!r} != {META_SCHEMA}")
    status = meta.get("status")
    if status != "complete":
        (rep.warnings if allow_incomplete else rep.errors).append(f"run status is {status!r}")
    path = raw_dir / meta.get("raw_file", "")
    if not meta.get("raw_file") or not path.is_file():
        if meta.get("recorded_samples", 0) == 0 and str(status).startswith(("aborted", "interrupted", "error")):
            return rep
        rep.errors.append(f"raw file missing: {path}")
        return rep
    if status == "complete":
        actual = sha256_file(path)
        if actual != meta.get("raw_sha256"):
            rep.errors.append(f"raw sha256 mismatch: file {actual} != meta {meta.get('raw_sha256')}")
        if path.stat().st_size != meta.get("raw_bytes"):
            rep.errors.append(f"raw byte count mismatch: {path.stat().st_size} != {meta.get('raw_bytes')}")

    plan = {p["seq"]: p for p in meta.get("plan", []) if p.get("seq") is not None}
    prev = ZERO_DIGEST
    fps: Dict[str, str] = {}
    expected_seq = 0
    try:
        for row in read_rows(path):
            rep.rows += 1
            where = f"row seq={row.get('seq')}"
            missing = [k for k in REQUIRED_ROW_KEYS if k not in row]
            if missing:
                rep.errors.append(f"{where}: missing keys {missing}")
                continue
            if row["schema"] != RAW_SCHEMA:
                rep.errors.append(f"{where}: schema {row['schema']}")
            if row["row_sha256"] != row_digest(row):
                rep.errors.append(f"{where}: row_sha256 does not match content (tampered or corrupt)")
            if row["prev_row_sha256"] != prev:
                rep.errors.append(f"{where}: hash chain broken")
            prev = row["row_sha256"]
            if row["experiment_id"] != meta.get("experiment_id") or row["run_id"] != run_id:
                rep.errors.append(f"{where}: experiment/run id mismatch")
            if row["seq"] != expected_seq:
                rep.errors.append(f"{where}: expected seq {expected_seq}")
            expected_seq = row["seq"] + 1
            p = plan.get(row["seq"])
            if p is None:
                rep.errors.append(f"{where}: not in plan")
            elif (p["phase"], p["candidate"], p["item_key"], p["repetition"]) != (
                row["phase"], row["candidate"], f"{row['split']}/{row['item_id']}", row["repetition"]
            ):
                rep.errors.append(f"{where}: does not match plan entry {p}")
            for k in ("stdout_sha256", "stderr_sha256"):
                if not (isinstance(row[k], str) and HEX64.match(row[k])):
                    rep.errors.append(f"{where}: {k} not a sha256 hex digest")
            if row["env_id"] != meta.get("env_id"):
                rep.errors.append(f"{where}: env_id differs from meta")
            if row["git_sha"] != meta.get("git_sha"):
                rep.errors.append(f"{where}: git_sha differs from meta")
            key = f"{row['split']}/{row['item_id']}"
            fp = (row.get("item_fingerprint") or {}).get("sha256")
            if not fp:
                rep.errors.append(f"{where}: item_fingerprint missing sha256")
            elif fps.setdefault(key, fp) != fp:
                rep.errors.append(f"{where}: item fingerprint changed within run for {key}")
            if row["split"] == "heldout":
                unlock = (row.get("heldout") or {}).get("unlock_sha")
                if design_freeze_sha is None or unlock != design_freeze_sha:
                    rep.errors.append(f"{where}: held-out row without a matching design-freeze unlock")
            t = row["timing"]
            if t.get("decision_grade"):
                if not t.get("guard_pre") or not t.get("guard_post"):
                    rep.errors.append(f"{where}: decision-grade timing row lacks guard readings")
                if row["valid"] and not (t["guard_pre"].get("ok") and t["guard_post"].get("ok")):
                    rep.errors.append(f"{where}: valid timing row with failing guard")
    except ValueError as exc:
        rep.errors.append(str(exc))
    if status == "complete" and rep.rows != meta.get("recorded_samples"):
        rep.errors.append(f"row count {rep.rows} != meta recorded_samples {meta.get('recorded_samples')}")
    if status == "complete" and rep.rows != len(plan):
        rep.errors.append(f"row count {rep.rows} != planned samples {len(plan)}")
    if meta.get("git_sha") is None:
        rep.warnings.append("git_sha unknown")
    elif meta.get("git_dirty"):
        rep.warnings.append("git worktree had tracked modifications at run time")
    if not (meta.get("env") or {}).get("registered"):
        rep.warnings.append(f"env_id {meta.get('env_id')} is not registered under research/environment")
    for w in (meta.get("env") or {}).get("warnings") or []:
        rep.warnings.append(f"env: {w}")
    if refingerprint is not None:
        for key, fp in sorted(fps.items()):
            if key.startswith("heldout/"):
                continue
            current = refingerprint(key)
            if current is None:
                rep.warnings.append(f"cannot re-fingerprint {key} (item not present)")
            elif current != fp:
                rep.errors.append(f"corpus item {key} changed since run: {current} != {fp}")
    return rep
