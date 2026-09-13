"""Environment identification (env_id) against the research/environment registry.

The registry (see research/environment/README.md, tooling in research/tools/env/) holds
content-addressed fingerprints ``<env_id>.json`` (schema
``entrybound.research.env-fingerprint/1``) and ``index.json`` mapping environment
names (``windows-host``, ``wsl-ubuntu``, ``docker-ubuntu-24.04``) to their current
env_id.

Resolution order:
1. ``EB_ENV_ID``: an env_id that must exist as ``<env_id>.json`` (or
   ``EB_ENV_ALLOW_UNREGISTERED=1``).
2. ``EB_ENV_NAME``: a name looked up in ``index.json``.
3. Auto-detect the running kind (``windows-host``, ``wsl``, ``container``, ``linux``)
   and pick the unique index entry whose fingerprint has ``stable.kind`` equal to it.
4. Otherwise ``unregistered-<platform>-<probe12>`` with ``registered: false``.

A registered fingerprint is integrity-checked (``env_id == sha256(canonical_json(stable))``).
Because env_id covers the repository commit, a fingerprint captured at a different
commit than the current HEAD is still recorded but flagged in ``warnings``; its
``env_id_without_repo`` and ``platform_id`` remain valid for grouping. Recapture with
``research/tools/env/capture_all.ps1``.
"""

from __future__ import annotations

import hashlib
import json
import os
import platform
import sys
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

from .paths import Layout, git_sha

INDEX_SCHEMA = "entrybound.research.env-index/1"
FINGERPRINT_SCHEMA = "entrybound.research.env-fingerprint/1"


def _read(path: str) -> Optional[str]:
    try:
        with open(path, encoding="utf-8", errors="replace") as fh:
            return fh.read()
    except OSError:
        return None


def detect_kind() -> str:
    if sys.platform.startswith("win"):
        return "windows-host"
    if sys.platform.startswith("linux"):
        if os.path.exists("/.dockerenv") or os.path.exists("/run/.containerenv"):
            return "container"
        if "microsoft" in (_read("/proc/version") or "").lower():
            return "wsl"
        return "linux"
    return sys.platform


def probe() -> Dict[str, Any]:
    info: Dict[str, Any] = {
        "kind": detect_kind(),
        "system": platform.system().lower(),
        "kernel": platform.release(),
        "machine": platform.machine().lower(),
        "logical_cpus": os.cpu_count(),
        "python": platform.python_version(),
    }
    cpuinfo = _read("/proc/cpuinfo")
    model = None
    if cpuinfo:
        for line in cpuinfo.splitlines():
            if line.lower().startswith("model name"):
                model = line.split(":", 1)[1].strip()
                break
    info["cpu_model"] = model or platform.processor() or None
    try:
        import psutil

        info["mem_total_bytes"] = int(psutil.virtual_memory().total)
    except Exception:  # pragma: no cover
        info["mem_total_bytes"] = None
    return info


def probe_fingerprint(info: Dict[str, Any]) -> str:
    stable = {k: info.get(k) for k in ("kind", "system", "kernel", "machine", "logical_cpus", "cpu_model")}
    mem = info.get("mem_total_bytes")
    stable["mem_total_gib"] = None if mem is None else round(mem / (1 << 30))
    return hashlib.sha256(json.dumps(stable, sort_keys=True).encode()).hexdigest()


def _canonical(obj: Any) -> bytes:
    return json.dumps(obj, sort_keys=True, separators=(",", ":"), ensure_ascii=False, allow_nan=False).encode("utf-8")


def load_fingerprint(env_dir: Path, env_id: str) -> Dict[str, Any]:
    path = env_dir / f"{env_id}.json"
    doc = json.loads(path.read_text(encoding="utf-8"))
    if doc.get("schema") != FINGERPRINT_SCHEMA:
        raise RuntimeError(f"{path}: unexpected schema {doc.get('schema')!r}")
    if doc.get("env_id") != env_id:
        raise RuntimeError(f"{path}: env_id field {doc.get('env_id')!r} does not match file name")
    stable = doc.get("stable")
    if hashlib.sha256(_canonical(stable)).hexdigest() != env_id:
        raise RuntimeError(f"{path}: env_id does not hash-match its stable section (corrupt or edited)")
    return doc


def _index(env_dir: Path) -> Optional[Dict[str, Any]]:
    p = env_dir / "index.json"
    if not p.is_file():
        return None
    idx = json.loads(p.read_text(encoding="utf-8"))
    if idx.get("schema") != INDEX_SCHEMA:
        raise RuntimeError(f"{p}: unexpected schema {idx.get('schema')!r}")
    return idx


def _details_for(doc: Dict[str, Any], name: Optional[str], source: str, repo_root: Path, info: Dict[str, Any]) -> Dict[str, Any]:
    stable = doc["stable"]
    warnings: List[str] = []
    fp_sha = ((stable.get("repo") or {}).get("git") or {}).get("head_sha")
    head = git_sha(repo_root)
    if fp_sha and head and fp_sha != head:
        warnings.append(
            f"environment fingerprint captured at repo commit {fp_sha[:12]}, current HEAD {head[:12]}: "
            "env_id is stale for this commit; group by env_id_without_repo/platform_id or recapture"
        )
    if stable.get("kind") != info["kind"]:
        warnings.append(f"fingerprint kind {stable.get('kind')!r} != detected kind {info['kind']!r}")
    if info["kind"] in ("wsl", "linux", "container"):
        kernel_blob = json.dumps((stable.get("platform") or {}).get("kernel"))
        if info["kernel"] not in kernel_blob:
            warnings.append(f"running kernel {info['kernel']} not found in fingerprint: environment may have changed")
    return {
        "registered": True,
        "source": source,
        "env_name": name,
        "env_id_without_repo": doc.get("env_id_without_repo"),
        "platform_id": doc.get("platform_id"),
        "fingerprint_repo_sha": fp_sha,
        "warnings": warnings,
        "probe": info,
        "probe_fingerprint": probe_fingerprint(info),
    }


def resolve_env_id(layout: Layout, info: Optional[Dict[str, Any]] = None) -> Tuple[str, Dict[str, Any]]:
    """Return (env_id, details)."""
    info = info or probe()
    env_dir = layout.environment_dir
    idx = _index(env_dir) if env_dir.is_dir() else None

    forced = os.environ.get("EB_ENV_ID")
    if forced:
        if (env_dir / f"{forced}.json").is_file():
            doc = load_fingerprint(env_dir, forced)
            name = next((n for n, e in (idx or {}).get("environments", {}).items() if e == forced), None)
            return forced, _details_for(doc, name, "EB_ENV_ID", layout.repo_root, info)
        if os.environ.get("EB_ENV_ALLOW_UNREGISTERED") != "1":
            raise RuntimeError(f"EB_ENV_ID={forced} has no fingerprint under {env_dir}")
        return forced, {"registered": False, "source": "EB_ENV_ID (unregistered)", "warnings": [], "probe": info,
                        "probe_fingerprint": probe_fingerprint(info)}

    if idx is not None:
        envs = idx.get("environments", {})
        name = os.environ.get("EB_ENV_NAME")
        source = "EB_ENV_NAME"
        if name is None:
            matches = []
            for n, e in sorted(envs.items()):
                try:
                    kind = json.loads((env_dir / f"{e}.json").read_text(encoding="utf-8"))["stable"].get("kind")
                except (OSError, ValueError, KeyError):
                    continue
                if kind == info["kind"]:
                    matches.append(n)
            if len(matches) == 1:
                name, source = matches[0], "research/environment/index.json (kind match)"
            elif len(matches) > 1:
                raise RuntimeError(f"several environments of kind {info['kind']!r} in index.json {matches}; set EB_ENV_NAME")
        if name is not None:
            if name not in envs:
                raise RuntimeError(f"EB_ENV_NAME={name} not in {env_dir / 'index.json'}")
            env_id = envs[name]
            return env_id, _details_for(load_fingerprint(env_dir, env_id), name, source, layout.repo_root, info)

    fp = probe_fingerprint(info)
    return f"unregistered-{info['kind']}-{fp[:12]}", {
        "registered": False, "source": "unregistered", "warnings": [], "probe": info, "probe_fingerprint": fp}
