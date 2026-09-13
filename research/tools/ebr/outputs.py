"""Generated-output helpers: provenance headers, deterministic CSV/JSON/Markdown writing."""

from __future__ import annotations

import hashlib
import io
import json
import os
import shlex
from pathlib import Path
from typing import Any, Dict, List, Optional, Sequence, Tuple

import pandas as pd

from . import __version__
from .paths import Layout, git_sha

FLOAT_FORMAT = "%.10g"
HEADER_PREFIX = "# "


def command_line(subcommand: str, layout: Layout, spec_path: Optional[str], extra: Sequence[str] = ()) -> str:
    parts = ["python", "-m", "ebr", subcommand]
    if spec_path:
        parts += ["--spec", layout.rel(spec_path)]
    parts += list(extra)
    return " ".join(shlex.quote(p) for p in parts)


def provenance(layout: Layout, command: str, experiment_id: str, inputs: Sequence[Tuple[str, str]]) -> List[str]:
    """Header lines (without comment prefix). ``inputs`` = [(label, sha256)]."""
    lines = [
        f"generated-by: {command}",
        f"tool: ebr {__version__} (research/tools/ebr)",
        f"tool-git-sha: {git_sha(layout.repo_root) or 'unknown'}",
        f"experiment: {experiment_id}",
    ]
    for label, digest in inputs:
        lines.append(f"input: {label} sha256={digest}")
    lines.append("do-not-edit: regenerate with the generated-by command; no number here is hand-entered")
    return lines


def _atomic_write_bytes(path: Path, data: bytes) -> str:
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = path.with_name(path.name + ".tmp")
    with open(tmp, "wb") as fh:
        fh.write(data)
    os.replace(tmp, path)
    return hashlib.sha256(data).hexdigest()


def write_csv(path: Path, df: pd.DataFrame, header: Sequence[str]) -> str:
    buf = io.StringIO()
    for line in header:
        buf.write(HEADER_PREFIX + line.replace("\n", " ") + "\n")
    df.to_csv(buf, index=False, lineterminator="\n", float_format=FLOAT_FORMAT)
    return _atomic_write_bytes(path, buf.getvalue().encode("utf-8"))


def read_csv(path: Path) -> Tuple[List[str], pd.DataFrame]:
    header: List[str] = []
    with open(path, encoding="utf-8") as fh:
        for line in fh:
            if line.startswith(HEADER_PREFIX):
                header.append(line[len(HEADER_PREFIX):].rstrip("\n"))
            else:
                break
    df = pd.read_csv(path, skiprows=len(header), keep_default_na=True)
    return header, df


def write_json(path: Path, data: Dict[str, Any]) -> str:
    text = json.dumps(data, indent=2, sort_keys=True, ensure_ascii=False, allow_nan=False) + "\n"
    return _atomic_write_bytes(path, text.encode("utf-8"))


def write_text(path: Path, text: str) -> str:
    return _atomic_write_bytes(path, text.encode("utf-8"))


GENERATED_SUFFIXES = (".csv", ".json", ".md", ".png", ".tmp")


def clear_generated(directory: Path) -> None:
    """Remove stale generated files (directories under normalized/ and reports/*/<exp> are ebr-owned)."""
    if not directory.is_dir():
        return
    for p in directory.iterdir():
        if p.is_file() and p.suffix in GENERATED_SUFFIXES:
            p.unlink()


def sha256_path(path: Path) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as fh:
        for chunk in iter(lambda: fh.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()
