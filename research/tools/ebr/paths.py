"""Repository layout, data roots, and read-only git queries."""

from __future__ import annotations

import os
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path, PurePosixPath
from typing import Optional

IS_WINDOWS = sys.platform.startswith("win")
IS_LINUX = sys.platform.startswith("linux")


def _detect_repo_root() -> Path:
    env = os.environ.get("EB_REPO_ROOT")
    if env:
        return Path(env).resolve()
    # research/tools/ebr/paths.py -> repo root is parents[3]
    here = Path(__file__).resolve()
    cand = here.parents[3]
    if (cand / "research").is_dir():
        return cand
    raise RuntimeError("cannot locate repository root; set EB_REPO_ROOT")


def default_data_root() -> Optional[Path]:
    """Large-data root outside git (WSL ext4). ``EB_RESEARCH_ROOT`` overrides."""
    env = os.environ.get("EB_RESEARCH_ROOT")
    if env:
        return Path(env)
    if IS_LINUX:
        return Path("/root/eb-research")
    return None


@dataclass(frozen=True)
class Layout:
    repo_root: Path
    data_root: Optional[Path]

    @classmethod
    def detect(cls, repo_root: Optional[str] = None, data_root: Optional[str] = None) -> "Layout":
        rr = Path(repo_root).resolve() if repo_root else _detect_repo_root()
        dr = Path(data_root) if data_root else default_data_root()
        return cls(rr, dr)

    # --- repository (git-tracked, small) ---
    @property
    def research(self) -> Path:
        return self.repo_root / "research"

    def raw_dir(self, experiment_id: str) -> Path:
        return self.research / "raw" / experiment_id

    def normalized_dir(self, experiment_id: str) -> Path:
        return self.research / "normalized" / experiment_id

    def figures_dir(self, experiment_id: str) -> Path:
        return self.research / "reports" / "figures" / experiment_id

    def tables_dir(self, experiment_id: str) -> Path:
        return self.research / "reports" / "tables" / experiment_id

    @property
    def thresholds_path(self) -> Path:
        return self.research / "methods" / "thresholds.json"

    @property
    def design_freeze_path(self) -> Path:
        return self.research / "decisions" / "design-freeze.json"

    @property
    def environment_dir(self) -> Path:
        return self.research / "environment"

    # --- data root (outside git) ---
    def _need_data_root(self) -> Path:
        if self.data_root is None:
            raise RuntimeError("no data root on this platform; set EB_RESEARCH_ROOT")
        return self.data_root

    @property
    def corpus_root(self) -> Path:
        env = os.environ.get("EB_CORPUS_ROOT")
        return Path(env) if env else self._need_data_root() / "corpus"

    @property
    def heldout_root(self) -> Path:
        env = os.environ.get("EB_HELDOUT_ROOT")
        return Path(env) if env else self._need_data_root() / "heldout"

    @property
    def generated_root(self) -> Path:
        return self._need_data_root() / "generated"

    @property
    def scratch_root(self) -> Path:
        env = os.environ.get("EB_SCRATCH_ROOT")
        if env:
            return Path(env)
        if self.data_root is not None:
            return self.data_root / "scratch" / "ebr"
        import tempfile

        return Path(tempfile.gettempdir()) / "eb-research-scratch" / "ebr"

    @property
    def cache_dir(self) -> Optional[Path]:
        if self.data_root is None:
            return None
        return self.data_root / "cache" / "ebr"

    def rel(self, path: os.PathLike | str) -> str:
        """Repo-relative POSIX path when inside the repo, else absolute POSIX path."""
        p = Path(path).resolve()
        try:
            return str(PurePosixPath(*p.relative_to(self.repo_root).parts))
        except ValueError:
            return p.as_posix()


def _git(repo: Path, *args: str, timeout: float = 60.0) -> Optional[str]:
    try:
        out = subprocess.run(
            ["git", "-c", "safe.directory=*", "-C", str(repo), *args],
            capture_output=True,
            text=True,
            timeout=timeout,
        )
    except (OSError, subprocess.TimeoutExpired):
        return None
    if out.returncode != 0:
        return None
    return out.stdout


def git_sha(repo: Path) -> Optional[str]:
    out = _git(repo, "rev-parse", "HEAD")
    if out:
        return out.strip()
    # Fallback: read .git directly (read-only).
    head = repo / ".git" / "HEAD"
    try:
        ref = head.read_text().strip()
    except OSError:
        return None
    if not ref.startswith("ref:"):
        return ref
    name = ref.split(None, 1)[1]
    loose = repo / ".git" / name
    if loose.is_file():
        return loose.read_text().strip()
    packed = repo / ".git" / "packed-refs"
    if packed.is_file():
        for line in packed.read_text().splitlines():
            parts = line.split()
            if len(parts) == 2 and parts[1] == name:
                return parts[0]
    return None


def git_dirty(repo: Path) -> Optional[bool]:
    """True when tracked files differ from HEAD (untracked files ignored)."""
    out = _git(repo, "status", "--porcelain", "--untracked-files=no")
    if out is None:
        return None
    return bool(out.strip())
