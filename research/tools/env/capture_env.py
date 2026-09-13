#!/usr/bin/env python3
"""Entrybound research environment fingerprint.

Standard library only. Runs on Windows CPython 3.13 and Linux CPython 3.12
(WSL Ubuntu 24.04 and the pinned ubuntu:24.04 container).

Subcommands
-----------
capture  Probe the current environment and emit a fingerprint JSON document.
         With --out-dir the fingerprint is stored as <out-dir>/<env_id>.json
         and <out-dir>/index.json is updated (name -> env_id). Idempotent: an
         existing file whose stable section matches is left untouched.
store    Verify a fingerprint produced elsewhere (e.g. inside a container) and
         store it exactly like `capture --out-dir`.
verify   Re-hash every stored fingerprint and check index.json. Exit 1 on any
         mismatch.
lookup   Print one id field of a named environment from index.json.

Identity
--------
Every fingerprint has a `stable` section (hashed) and a `capture` section
(not hashed: timestamps, load, free memory, power source, worktree dirtiness,
observations). Three ids are derived from `stable`:

  env_id               sha256(canonical_json(stable))
  env_id_without_repo  sha256(canonical_json(stable minus "repo"))
  platform_id          sha256(canonical_json(stable["platform"]))

canonical_json = json.dumps(sort_keys=True, separators=(",", ":"),
ensure_ascii=False, allow_nan=False) encoded as UTF-8. Floats are forbidden in
`stable` so the encoding is identical across Python versions and platforms.
"""

from __future__ import annotations

import argparse
import configparser
import datetime as _dt
import glob
import hashlib
import json
import locale
import os
import platform
import re
import shutil
import struct
import subprocess
import sys
import uuid
from collections import Counter
from pathlib import Path

SCHEMA = "entrybound.research.env-fingerprint/1"
INDEX_SCHEMA = "entrybound.research.env-index/1"
ID_ALGORITHM = (
    "env_id = sha256(canonical_json(stable)); "
    "env_id_without_repo = sha256(canonical_json(stable minus 'repo')); "
    "platform_id = sha256(canonical_json(stable.platform)); "
    "canonical_json = json.dumps(sort_keys=True, separators=(',', ':'), "
    "ensure_ascii=False, allow_nan=False).encode('utf-8')"
)

IS_WINDOWS = os.name == "nt"
IS_LINUX = sys.platform.startswith("linux")
DEFAULT_TIMEOUT = 60

# Observations are human-readable findings gathered during a capture. They are
# stored in the unhashed capture section so wording changes never move ids.
OBSERVATIONS: list[str] = []


class FingerprintError(RuntimeError):
    """Raised on integrity failures (hash mismatch, inconsistent index)."""


# --------------------------------------------------------------------------
# Generic helpers
# --------------------------------------------------------------------------


def canonical_json(obj) -> bytes:
    return json.dumps(
        obj, sort_keys=True, separators=(",", ":"), ensure_ascii=False, allow_nan=False
    ).encode("utf-8")


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_file(path) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as fh:
        for chunk in iter(lambda: fh.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def read_text(path, default=None):
    try:
        return Path(path).read_text(encoding="utf-8", errors="replace")
    except OSError:
        return default


def read_first_line(path, default=None):
    text = read_text(path)
    if text is None:
        return default
    return text.strip()


def _decode(data: bytes) -> str:
    if not data:
        return ""
    # wsl.exe and a few Windows tools emit UTF-16LE without a BOM.
    if data[:2] == b"\xff\xfe" or (len(data) >= 4 and data[1] == 0 and data[3] == 0 and data[0] != 0):
        try:
            text = data.decode("utf-16-le")
        except UnicodeDecodeError:
            text = data.decode("utf-8", "replace")
    else:
        text = data.decode("utf-8", "replace")
    return text.lstrip("\ufeff").replace("\r\n", "\n").replace("\r", "\n")


def run(argv, cwd=None, timeout=DEFAULT_TIMEOUT, env=None, stdin_bytes=None):
    """Run a command, never raising. Returns a small dict."""
    kwargs = {}
    if IS_WINDOWS:
        kwargs["creationflags"] = 0x08000000  # CREATE_NO_WINDOW
    try:
        proc = subprocess.run(
            [str(a) for a in argv],
            cwd=cwd,
            env=env,
            input=stdin_bytes,
            stdin=None if stdin_bytes is not None else subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=timeout,
            **kwargs,
        )
    except FileNotFoundError:
        return {"ok": False, "rc": None, "stdout": "", "stderr": "", "error": "not-found"}
    except subprocess.TimeoutExpired:
        return {"ok": False, "rc": None, "stdout": "", "stderr": "", "error": f"timeout after {timeout}s"}
    except OSError as exc:
        return {"ok": False, "rc": None, "stdout": "", "stderr": "", "error": f"{type(exc).__name__}: {exc}"}
    return {
        "ok": proc.returncode == 0,
        "rc": proc.returncode,
        "stdout": _decode(proc.stdout),
        "stderr": _decode(proc.stderr),
        "error": None,
    }


def parse_cpulist(text):
    ids = []
    if not text:
        return ids
    for part in text.strip().split(","):
        part = part.strip()
        if not part:
            continue
        if "-" in part:
            lo, hi = part.split("-", 1)
            ids.extend(range(int(lo), int(hi) + 1))
        else:
            ids.append(int(part))
    return sorted(set(ids))


def compress_ranges(ids):
    ids = sorted(set(ids))
    if not ids:
        return ""
    out = []
    start = prev = ids[0]
    for i in ids[1:]:
        if i == prev + 1:
            prev = i
            continue
        out.append(f"{start}-{prev}" if start != prev else f"{start}")
        start = prev = i
    out.append(f"{start}-{prev}" if start != prev else f"{start}")
    return ",".join(out)


def nearest_existing(path: str) -> str:
    p = os.path.abspath(path)
    while not os.path.exists(p):
        parent = os.path.dirname(p)
        if parent == p:
            break
        p = parent
    return p


# --------------------------------------------------------------------------
# Privacy: strip user-profile names from every recorded string
# --------------------------------------------------------------------------

_USER_PATTERNS = [
    # C:\Users\name\...  or C:/Users/name/...
    re.compile(r"(?i)(\b[A-Z]:[\\/]+Users[\\/]+)(?!Public\b|Default\b|<user>)([^\\/:*?\"<>|\r\n]+)"),
    # /mnt/c/Users/name, /run/desktop/mnt/host/c/Users/name, \\wsl$ style paths
    re.compile(r"(?i)((?:^|[\\/])[a-z][\\/]Users[\\/])(?!Public\b|Default\b|<user>)([^\\/:\r\n]+)"),
    re.compile(r"(/home/)(?!<user>)([^/:\s\"']+)"),
]


def _current_user_names():
    names = set()
    for var in ("USERNAME", "USER", "LOGNAME"):
        v = os.environ.get(var)
        if v:
            names.add(v)
    try:
        names.add(Path.home().name)
    except (RuntimeError, OSError):
        pass
    return {n for n in names if len(n) >= 3 and n.lower() not in {"root", "user", "admin", "system"}}


_USER_NAMES = _current_user_names()


def redact(text: str) -> str:
    for pat in _USER_PATTERNS:
        text = pat.sub(lambda m: m.group(1) + "<user>", text)
    for name in _USER_NAMES:
        text = re.sub(r"(?<![A-Za-z0-9])" + re.escape(name) + r"(?![A-Za-z0-9])", "<user>", text)
    return text


def normalize(obj):
    """Deep-sort dict keys and redact every string (keys included)."""
    if isinstance(obj, dict):
        return {redact(str(k)): normalize(obj[k]) for k in sorted(obj, key=lambda x: str(x))}
    if isinstance(obj, (list, tuple)):
        return [normalize(v) for v in obj]
    if isinstance(obj, str):
        return redact(obj)
    return obj


def assert_no_floats(obj, where="stable"):
    if isinstance(obj, float):
        raise FingerprintError(f"float value in hashed section at {where}")
    if isinstance(obj, dict):
        for k, v in obj.items():
            assert_no_floats(v, f"{where}.{k}")
    elif isinstance(obj, list):
        for i, v in enumerate(obj):
            assert_no_floats(v, f"{where}[{i}]")


# --------------------------------------------------------------------------
# Tools on PATH
# --------------------------------------------------------------------------

_7Z = r"7-Zip(?:\s+\[\d+\]|\s+\([a-z]\))?\s+(\d+\.\d+)"

# (name, version arguments, regex whose group 1 is the version)
TOOL_SPECS = [
    ("tar", ["--version"], r"(?:\(GNU tar\)|bsdtar)\s+(\S+)"),
    ("bsdtar", ["--version"], r"bsdtar\s+(\S+)"),
    ("7z", [], _7Z),
    ("7zz", [], _7Z),
    ("7za", [], _7Z),
    ("7zr", [], _7Z),
    ("zstd", ["-V"], r"\bv(\d+\.\d+\.\d+)"),
    ("xz", ["--version"], r"xz \(XZ Utils\) (\S+)"),
    ("gzip", ["--version"], r"^gzip (\S+)"),
    ("pigz", ["--version"], r"^pigz (\S+)"),
    ("lz4", ["--version"], r"\bv(\d+\.\d+\.\d+)"),
    ("lzip", ["--version"], r"^lzip (\S+)"),
    ("brotli", ["--version"], r"^brotli (\S+)"),
    ("pixz", ["-h"], None),  # no version flag; Debian package version is recorded instead
    ("par2", ["-V"], r"par2cmdline version (\S+)"),
    # `zip -v` with redirected stdin/stdout compresses stdin instead of printing
    # the version screen, so parse the help banner instead.
    ("zip", ["-h"], r"^Zip (\d\S*)"),
    ("unzip", ["-v"], r"^UnZip (\S+)"),
    ("bzip2", ["--help"], r"Version (\d[^,\s]*)"),
    ("cpio", ["--version"], r"cpio \(GNU cpio\) (\S+)"),
    ("mksquashfs", ["-version"], r"mksquashfs version (\S+)"),
    ("hyperfine", ["--version"], r"^hyperfine (\S+)"),
    ("sqlite3", ["--version"], r"^(\d+\.\d+\.\d+)"),
    ("jq", ["--version"], r"^jq-(\S+)"),
    ("python", ["--version"], r"^Python (\S+)"),
    ("python3", ["--version"], r"^Python (\S+)"),
    ("java", ["-version"], r'version "([^"]+)"'),
    ("docker", ["--version"], r"version ([^,\s]+)"),
    ("git", ["--version"], r"git version (\S+)"),
    ("rustc", ["-V"], r"^rustc (\S+)"),
    ("cargo", ["-V"], r"^cargo (\S+)"),
    ("rustup", ["--version"], r"^rustup (\S+)"),
]

REQUIRED_BASELINE_TOOLS = [
    "tar", "bsdtar", "7z", "7zz", "zstd", "xz", "gzip", "pigz", "lz4", "lzip",
    "brotli", "pixz", "par2", "zip", "unzip", "java", "docker",
]


def _is_app_execution_alias(path: str) -> bool:
    if not IS_WINDOWS:
        return False
    try:
        st = os.lstat(path)
    except OSError:
        return False
    return getattr(st, "st_reparse_tag", 0) == 0x8000001B


def dpkg_package_for(paths):
    if not IS_LINUX or not shutil.which("dpkg-query"):
        return None
    seen = []
    for p in paths:
        if not p or p in seen:
            continue
        seen.append(p)
        candidates = [p]
        if p.startswith("/usr/bin/") or p.startswith("/usr/sbin/"):
            candidates.append(p[4:])
        for cand in candidates:
            res = run(["dpkg-query", "-S", cand])
            if not res["ok"]:
                continue
            for line in res["stdout"].splitlines():
                if line.startswith("diversion ") or ": " not in line:
                    continue
                pkg = line.split(": ", 1)[0].split(",")[0].strip()
                meta = run(["dpkg-query", "-W", "-f=${Package}\t${Version}\t${Architecture}", pkg])
                if meta["ok"] and meta["stdout"].count("\t") == 2:
                    name, version, arch = meta["stdout"].strip().split("\t")
                    return {"name": name, "version": version, "architecture": arch}
                return {"name": pkg}
    return None


def probe_tool(name, args, pattern, cwd):
    path = shutil.which(name)
    if not path:
        return {"found": False}
    rec = {"found": True, "path": path}
    real = os.path.realpath(path)
    if os.path.normcase(real) != os.path.normcase(path):
        rec["resolved_path"] = real
    if _is_app_execution_alias(path):
        rec["app_execution_alias"] = True
        rec["executed"] = False
        return rec
    try:
        rec["size_bytes"] = os.stat(real).st_size
        rec["sha256"] = sha256_file(real)
    except OSError as exc:
        rec["hash_error"] = f"{type(exc).__name__}: {exc.strerror or exc}"
    res = run([path, *args], cwd=cwd)
    rec["exit_code"] = res["rc"]
    if res["error"]:
        rec["error"] = res["error"]
    text = res["stdout"] + "\n" + res["stderr"]
    if pattern:
        m = re.search(pattern, text, re.M)
        if m:
            rec["version"] = m.group(1)
            line_start = text.rfind("\n", 0, m.start()) + 1
            line_end = text.find("\n", m.end())
            rec["version_line"] = text[line_start: line_end if line_end >= 0 else None].strip()[:200]
    pkg = dpkg_package_for([real, path])
    if pkg:
        rec["package"] = pkg
    return rec


def tools_info(cwd):
    tools = {}
    for name, args, pattern in TOOL_SPECS:
        tools[name] = probe_tool(name, args, pattern, cwd)
    missing = [t for t in REQUIRED_BASELINE_TOOLS if not tools.get(t, {}).get("found")]
    if missing:
        OBSERVATIONS.append("baseline tools not found on PATH: " + ", ".join(missing))
    return tools


# --------------------------------------------------------------------------
# Rust toolchain
# --------------------------------------------------------------------------


def _rustc_vv(res):
    if not res["ok"]:
        return {"error": res["error"] or (res["stderr"].strip().splitlines() or ["rc=%s" % res["rc"]])[-1][:300]}
    out = {}
    lines = res["stdout"].strip().splitlines()
    if lines:
        out["version_line"] = lines[0].strip()
    for line in lines[1:]:
        k, sep, v = line.partition(":")
        if sep:
            out[k.strip().replace(" ", "_").replace("-", "_")] = v.strip()
    return out


def _first_line(res):
    if not res["ok"]:
        return {"error": res["error"] or (res["stderr"].strip().splitlines() or ["rc=%s" % res["rc"]])[-1][:300]}
    lines = res["stdout"].strip().splitlines()
    return {"version_line": lines[0].strip() if lines else ""}


def rust_info(repo_cwd):
    exe = ".exe" if IS_WINDOWS else ""
    cargo_home = Path(os.environ.get("CARGO_HOME") or (Path.home() / ".cargo"))
    cargo_bin = cargo_home / "bin"
    rustup_on_path = shutil.which("rustup")
    rustup = rustup_on_path
    if not rustup and (cargo_bin / f"rustup{exe}").is_file():
        rustup = str(cargo_bin / f"rustup{exe}")
    info = {
        "cargo_home_bin": str(cargo_bin),
        "cargo_home_bin_on_path": any(
            os.path.normcase(os.path.abspath(p)) == os.path.normcase(os.path.abspath(str(cargo_bin)))
            for p in os.environ.get("PATH", "").split(os.pathsep) if p
        ),
        "rustup": None,
    }
    path_rustc = shutil.which("rustc")
    info["path_rustc"] = {"path": path_rustc, **_first_line(run([path_rustc, "-V"], cwd=repo_cwd))} if path_rustc else None
    if not rustup:
        return info
    bindir = Path(rustup).parent
    ver = run([rustup, "--version"], cwd=repo_cwd)
    info["rustup"] = {"path": rustup, "found_on_path": bool(rustup_on_path), **_first_line(ver)}
    active = run([rustup, "show", "active-toolchain"], cwd=repo_cwd)
    info["active_toolchain_in_repo"] = active["stdout"].strip() if active["ok"] else {"error": active["stderr"].strip()[:300]}
    listing = run([rustup, "toolchain", "list"], cwd=repo_cwd)
    lines = [ln.strip() for ln in listing["stdout"].splitlines() if ln.strip()] if listing["ok"] else []
    info["installed_toolchains"] = lines
    info["repo_proxy"] = {
        "rustc": _rustc_vv(run([bindir / f"rustc{exe}", "-Vv"], cwd=repo_cwd)),
        "cargo": _first_line(run([bindir / f"cargo{exe}", "-V"], cwd=repo_cwd)),
    }
    per_tc = {}
    for ln in lines:
        tc = ln.split()[0]
        per_tc[tc] = {
            "rustc": _rustc_vv(run([rustup, "run", tc, "rustc", "-Vv"], cwd=repo_cwd)),
            "cargo": _first_line(run([rustup, "run", tc, "cargo", "-V"], cwd=repo_cwd)),
        }
    info["toolchains"] = per_tc
    repo_release = info["repo_proxy"]["rustc"].get("release")
    if path_rustc and repo_release:
        path_line = (info["path_rustc"] or {}).get("version_line", "")
        if repo_release not in path_line:
            OBSERVATIONS.append(
                f"`rustc` resolved from PATH ({path_rustc}: {path_line or 'error'}) differs from the repo toolchain "
                f"({repo_release}); invoke the rustup proxy in {cargo_bin} or source ~/.cargo/env"
            )
    elif not path_rustc and repo_release:
        OBSERVATIONS.append(f"rustc is not on PATH; repo toolchain {repo_release} is reachable only via {cargo_bin}")
    return info


# --------------------------------------------------------------------------
# Repository state
# --------------------------------------------------------------------------


def git_head(repo: Path):
    dotgit = repo / ".git"
    if dotgit.is_file():
        content = read_text(dotgit, "").strip()
        if not content.startswith("gitdir:"):
            return {"error": ".git file without gitdir"}
        gitdir = (repo / content[len("gitdir:"):].strip()).resolve()
    elif dotgit.is_dir():
        gitdir = dotgit
    else:
        return {"error": "no .git"}
    commondir = gitdir
    if (gitdir / "commondir").is_file():
        commondir = (gitdir / read_text(gitdir / "commondir", "").strip()).resolve()
    head = read_text(gitdir / "HEAD", "").strip()
    if not head.startswith("ref:"):
        return {"head_ref": None, "head_sha": head or None}
    ref = head[len("ref:"):].strip()
    for base in (gitdir, commondir):
        f = base / ref
        if f.is_file():
            return {"head_ref": ref, "head_sha": read_text(f, "").strip()}
    packed = read_text(commondir / "packed-refs", "") or ""
    for line in packed.splitlines():
        if line.startswith("#") or line.startswith("^"):
            continue
        parts = line.split()
        if len(parts) == 2 and parts[1] == ref:
            return {"head_ref": ref, "head_sha": parts[0]}
    return {"head_ref": ref, "head_sha": None, "error": "ref not resolvable"}


def _file_digest(path: Path):
    try:
        data = path.read_bytes()
    except OSError as exc:
        return {"present": False, "error": exc.strerror or str(exc)}
    return {
        "present": True,
        "bytes": len(data),
        "sha256": sha256_bytes(data),
        "sha256_lf_normalized": sha256_bytes(data.replace(b"\r\n", b"\n")),
        "contains_crlf": b"\r\n" in data,
    }


def repo_info(repo: Path | None):
    if repo is None:
        return None, None
    stable = {"path": str(repo), "exists": repo.is_dir()}
    capture = {}
    if not repo.is_dir():
        return stable, capture
    stable["git"] = git_head(repo)
    stable["cargo_lock"] = _file_digest(repo / "Cargo.lock")
    stable["rust_toolchain_toml"] = _file_digest(repo / "rust-toolchain.toml")
    git = shutil.which("git")
    if git:
        base = [git, "-c", "safe.directory=*", "-C", str(repo)]
        rev = run(base + ["rev-parse", "HEAD"], timeout=120)
        if rev["ok"]:
            capture["git_rev_parse_head"] = rev["stdout"].strip()
            if rev["stdout"].strip() != stable["git"].get("head_sha"):
                raise FingerprintError(
                    f"git rev-parse HEAD ({rev['stdout'].strip()}) disagrees with .git reader "
                    f"({stable['git'].get('head_sha')})"
                )
        status = run(base + ["status", "--porcelain=v1", "--untracked-files=no"], timeout=300)
        if status["ok"]:
            capture["tracked_paths_modified"] = len([ln for ln in status["stdout"].splitlines() if ln.strip()])
        else:
            capture["git_status_error"] = (status["error"] or status["stderr"].strip())[:300]
    else:
        capture["git_cli"] = "not found; HEAD read directly from .git"
    return stable, capture


# --------------------------------------------------------------------------
# Environment variables that change tool behaviour (values recorded)
# --------------------------------------------------------------------------

TOOL_ENV_VARS = [
    "TAR_OPTIONS", "GZIP", "GZIP_OPT", "XZ_OPT", "XZ_DEFAULTS", "ZSTD_CLEVEL", "ZSTD_NBTHREADS",
    "BZIP", "BZIP2", "LZ4", "ZIPOPT", "UNZIP", "UNZIPOPT", "BROTLI", "SOURCE_DATE_EPOCH", "TZ",
    "RUSTFLAGS", "RUSTDOCFLAGS", "RUSTUP_TOOLCHAIN", "RUSTUP_HOME", "CARGO_HOME", "CARGO_TARGET_DIR",
    "CARGO_BUILD_TARGET", "CARGO_BUILD_JOBS", "CARGO_INCREMENTAL", "CARGO_ENCODED_RUSTFLAGS",
    "PYTHONHASHSEED", "PYTHONUTF8", "JAVA_TOOL_OPTIONS", "_JAVA_OPTIONS", "DOCKER_HOST", "DOCKER_CONTEXT",
    "DOCKER_BUILDKIT", "DOCKER_DEFAULT_PLATFORM", "LANG", "LC_ALL", "LC_CTYPE", "LC_COLLATE", "WSLENV",
]


def process_info():
    info = {
        "python": {
            "implementation": platform.python_implementation(),
            "version": platform.python_version(),
            "build": list(platform.python_build()),
            "compiler": platform.python_compiler(),
            "executable": sys.executable,
            "executable_sha256": sha256_file(sys.executable) if sys.executable and os.path.isfile(sys.executable) else None,
            "bits": struct.calcsize("P") * 8,
        },
        "encoding": {
            "preferred": locale.getpreferredencoding(False),
            "filesystem": sys.getfilesystemencoding(),
            "stdout": getattr(sys.stdout, "encoding", None),
        },
        "tool_env": {k: os.environ[k] for k in TOOL_ENV_VARS if k in os.environ},
        "cargo_profile_env": sorted(k for k in os.environ if k.startswith("CARGO_PROFILE_")),
    }
    if os.name == "posix":
        mask = os.umask(0)
        os.umask(mask)
        info["umask"] = f"{mask:04o}"
        try:
            import resource

            limits = {}
            for name in ("RLIMIT_NOFILE", "RLIMIT_NPROC", "RLIMIT_STACK", "RLIMIT_AS", "RLIMIT_FSIZE", "RLIMIT_MEMLOCK"):
                if hasattr(resource, name):
                    soft, hard = resource.getrlimit(getattr(resource, name))
                    limits[name] = [
                        "unlimited" if soft == resource.RLIM_INFINITY else soft,
                        "unlimited" if hard == resource.RLIM_INFINITY else hard,
                    ]
            info["rlimits"] = limits
        except ImportError:
            pass
        info["euid"] = os.geteuid()
    return info


# --------------------------------------------------------------------------
# Docker engine
# --------------------------------------------------------------------------


def _json_or_none(text):
    text = text.strip()
    if not text:
        return None
    try:
        return json.loads(text.splitlines()[0]) if text.startswith("{") else None
    except json.JSONDecodeError:
        try:
            return json.loads(text)
        except json.JSONDecodeError:
            return None


def docker_info(cwd):
    exe = shutil.which("docker")
    if not exe:
        return {"cli_found": False}, {}
    stable = {"cli_found": True}
    capture = {}
    ver = run([exe, "version", "--format", "{{json .}}"], cwd=cwd, timeout=120)
    vj = _json_or_none(ver["stdout"])
    if vj:
        c = vj.get("Client") or {}
        stable["client"] = {k: c.get(k) for k in ("Version", "ApiVersion", "GitCommit", "GoVersion", "Os", "Arch", "Context")}
        s = vj.get("Server")
        if s:
            stable["server"] = {
                "platform_name": (s.get("Platform") or {}).get("Name"),
                **{k: s.get(k) for k in ("Version", "ApiVersion", "MinAPIVersion", "Os", "Arch", "KernelVersion", "GitCommit")},
                "components": sorted(
                    ({"name": comp.get("Name"), "version": comp.get("Version")} for comp in (s.get("Components") or [])),
                    key=lambda d: str(d["name"]),
                ),
            }
    if not ver["ok"]:
        stable["server_available"] = False
        capture["docker_version_error"] = (ver["error"] or ver["stderr"].strip())[:400]
        return stable, capture
    stable["server_available"] = True
    inf = run([exe, "info", "--format", "{{json .}}"], cwd=cwd, timeout=120)
    ij = _json_or_none(inf["stdout"])
    if ij:
        keep = (
            "ServerVersion", "OperatingSystem", "OSType", "OSVersion", "Architecture", "KernelVersion", "NCPU",
            "MemTotal", "Driver", "CgroupDriver", "CgroupVersion", "DefaultRuntime", "SecurityOptions",
            "Isolation", "InitBinary", "LiveRestoreEnabled", "ExperimentalBuild", "DockerRootDir", "Labels",
            "IndexServerAddress",
        )
        eng = {k: ij.get(k) for k in keep}
        eng["DriverStatus"] = ij.get("DriverStatus")
        eng["Runtimes"] = sorted((ij.get("Runtimes") or {}).keys())
        eng["ContainerdCommit"] = (ij.get("ContainerdCommit") or {}).get("ID")
        eng["RuncCommit"] = (ij.get("RuncCommit") or {}).get("ID")
        eng["InitCommit"] = (ij.get("InitCommit") or {}).get("ID")
        plugins = ((ij.get("ClientInfo") or {}).get("Plugins")) or []
        eng["cli_plugins"] = sorted(
            ({"name": p.get("Name"), "version": p.get("Version")} for p in plugins), key=lambda d: str(d["name"])
        )
        stable["engine"] = eng
        # Docker Desktop serves `docker info` from a cache (SystemTime stays
        # frozen across calls), so these counts are indicative only.
        capture["docker_runtime_cached_info"] = {
            k: ij.get(k) for k in ("Containers", "ContainersRunning", "Images")
        }
        if ij.get("Warnings"):
            capture["docker_warnings"] = ij.get("Warnings")
    else:
        capture["docker_info_error"] = (inf["error"] or inf["stderr"].strip())[:400]
    return stable, capture


# --------------------------------------------------------------------------
# Linux platform
# --------------------------------------------------------------------------

KEY_CPU_FLAGS = [
    "hypervisor", "ht", "hybrid_cpu", "constant_tsc", "nonstop_tsc", "tsc_reliable", "aes", "pclmulqdq",
    "vpclmulqdq", "vaes", "sha_ni", "sse4_2", "popcnt", "avx", "avx2", "avx512f", "avx_vnni", "bmi1", "bmi2",
    "gfni", "erms", "fsrm",
]


def _os_release():
    out = {}
    for line in (read_text("/etc/os-release", "") or "").splitlines():
        k, sep, v = line.partition("=")
        if sep:
            out[k.strip()] = v.strip().strip('"')
    return out


def linux_cpu():
    text = read_text("/proc/cpuinfo", "") or ""
    procs = []
    for block in text.split("\n\n"):
        if not block.strip():
            continue
        d = {}
        for line in block.splitlines():
            k, sep, v = line.partition(":")
            if sep:
                d[k.strip()] = v.strip()
        procs.append(d)
    first = procs[0] if procs else {}
    flags = sorted(set(first.get("flags", "").split()))
    cpu_dirs = sorted(
        (d for d in glob.glob("/sys/devices/system/cpu/cpu[0-9]*") if os.path.basename(d)[3:].isdigit()),
        key=lambda d: int(os.path.basename(d)[3:]),
    )
    cores = {}
    packages = set()
    for d in cpu_dirs:
        n = int(os.path.basename(d)[3:])
        pkg = read_first_line(f"{d}/topology/physical_package_id")
        core = read_first_line(f"{d}/topology/core_id")
        if pkg is None or core is None:
            continue
        packages.add(pkg)
        cores.setdefault((pkg, core), []).append(n)
    threads_per_core = Counter(len(v) for v in cores.values())
    hybrid = {}
    for pmu in ("cpu_core", "cpu_atom"):
        cpus = read_first_line(f"/sys/devices/{pmu}/cpus")
        if cpus is not None:
            hybrid[pmu] = compress_ranges(parse_cpulist(cpus))
    caches = []
    for idx in sorted(glob.glob("/sys/devices/system/cpu/cpu0/cache/index[0-9]*")):
        caches.append({
            "level": read_first_line(f"{idx}/level"),
            "type": read_first_line(f"{idx}/type"),
            "size": read_first_line(f"{idx}/size"),
            "shared_cpu_list": read_first_line(f"{idx}/shared_cpu_list"),
        })
    vulns = {}
    for f in sorted(glob.glob("/sys/devices/system/cpu/vulnerabilities/*")):
        vulns[os.path.basename(f)] = read_first_line(f)
    try:
        affinity = len(os.sched_getaffinity(0))
    except (AttributeError, OSError):
        affinity = None
    return {
        "vendor_id": first.get("vendor_id"),
        "model_name": sorted({p.get("model name", "") for p in procs}),
        "family": first.get("cpu family"),
        "model": first.get("model"),
        "stepping": first.get("stepping"),
        "microcode": sorted({p.get("microcode", "") for p in procs}),
        "flags_count": len(flags),
        "flags_sha256": sha256_bytes(" ".join(flags).encode()),
        "selected_flags": {f: (f in flags) for f in KEY_CPU_FLAGS},
        "logical_cpus": os.cpu_count(),
        "online": read_first_line("/sys/devices/system/cpu/online"),
        "sched_affinity_count": affinity,
        "topology_as_presented": {
            "packages": len(packages),
            "cores": len(cores),
            "threads_per_core_histogram": {str(k): v for k, v in sorted(threads_per_core.items())},
        },
        "hybrid_pmu_cpus": hybrid or None,
        "hybrid_core_labels_available": bool(hybrid),
        "cpufreq_available": os.path.isdir("/sys/devices/system/cpu/cpu0/cpufreq"),
        "caches_cpu0": caches,
        "vulnerabilities": vulns,
    }


def _cgroup_values():
    rel = None
    for line in (read_text("/proc/self/cgroup", "") or "").splitlines():
        if line.startswith("0::"):
            rel = line[3:].strip()
    out = {"version": 2 if os.path.exists("/sys/fs/cgroup/cgroup.controllers") else 1}
    if rel is not None and out["version"] == 2:
        base = "/sys/fs/cgroup" + (rel if rel != "/" else "")
        for f in ("memory.max", "memory.swap.max", "cpu.max", "cpuset.cpus.effective", "pids.max"):
            out[f] = read_first_line(os.path.join(base, f))
    return out, rel


def linux_platform(kind, parent_platform_id):
    osr = _os_release()
    meminfo = {}
    for line in (read_text("/proc/meminfo", "") or "").splitlines():
        k, sep, v = line.partition(":")
        if sep:
            parts = v.split()
            if parts and parts[0].isdigit():
                meminfo[k.strip()] = int(parts[0]) * (1024 if len(parts) > 1 and parts[1] == "kB" else 1)
    cpu = linux_cpu()
    release = platform.release()
    cgroup, cgroup_rel = _cgroup_values()
    interop = any(os.path.exists(f"/proc/sys/fs/binfmt_misc/{n}") for n in ("WSLInterop", "WSLInterop-late"))
    virt = {
        "cpu_hypervisor_flag": cpu["selected_flags"].get("hypervisor"),
        "kernel_is_microsoft_wsl2": "microsoft-standard-wsl2" in release.lower(),
        "systemd_detect_virt": None,
        "dmi_sys_vendor": read_first_line("/sys/class/dmi/id/sys_vendor"),
        "dmi_product_name": read_first_line("/sys/class/dmi/id/product_name"),
        "container": {
            "dockerenv": os.path.exists("/.dockerenv"),
            "containerenv": os.path.exists("/run/.containerenv"),
            "cgroup_namespace_root": cgroup_rel == "/",
        },
    }
    sdv = shutil.which("systemd-detect-virt")
    if sdv:
        r = run([sdv])
        virt["systemd_detect_virt"] = r["stdout"].strip() or f"rc={r['rc']}"
    if virt["kernel_is_microsoft_wsl2"]:
        wsl = {
            "distro_name": os.environ.get("WSL_DISTRO_NAME"),
            "interop_enabled": interop,
            "wslg_present": os.path.isdir("/mnt/wslg"),
        }
        conf_text = read_text("/etc/wsl.conf")
        if conf_text is not None:
            cp = configparser.ConfigParser(interpolation=None)
            try:
                cp.read_string(conf_text)
                wsl["wsl_conf"] = {s: dict(cp.items(s)) for s in cp.sections()}
            except configparser.Error as exc:
                wsl["wsl_conf"] = {"parse_error": str(exc)[:200]}
        if interop and kind != "container":
            wsl["wsl_version"] = wsl_version()
        virt["wsl"] = wsl
    plat = {
        "os": {
            "family": "linux",
            "pretty_name": osr.get("PRETTY_NAME"),
            "id": osr.get("ID"),
            "version_id": osr.get("VERSION_ID"),
            "version_codename": osr.get("VERSION_CODENAME"),
            "libc": "-".join(x for x in platform.libc_ver() if x),
        },
        "kernel": {
            "release": release,
            "version": platform.version(),
            "machine": platform.machine(),
            "proc_version": (read_text("/proc/version", "") or "").strip(),
            "clocksource": read_first_line("/sys/devices/system/clocksource/clocksource0/current_clocksource"),
            "transparent_hugepage": read_first_line("/sys/kernel/mm/transparent_hugepage/enabled"),
        },
        "virtualization": virt,
        "cpu": cpu,
        "memory": {
            "mem_total_bytes": meminfo.get("MemTotal"),
            "swap_total_bytes": meminfo.get("SwapTotal"),
            "hugepagesize_bytes": meminfo.get("Hugepagesize"),
            "cgroup": cgroup,
        },
    }
    if parent_platform_id:
        plat["parent"] = {"relation": "hosted-by", "platform_id": parent_platform_id}
    if kind == "container":
        pk = run(["dpkg-query", "-W", "-f=${Package}=${Version}\n"]) if shutil.which("dpkg-query") else None
        if pk and pk["ok"]:
            lines = sorted(ln for ln in pk["stdout"].splitlines() if ln.strip())
            plat["os"]["dpkg_installed"] = {"count": len(lines), "sha256": sha256_bytes(("\n".join(lines) + "\n").encode())}
    topo = cpu["topology_as_presented"]
    if not cpu["hybrid_core_labels_available"]:
        OBSERVATIONS.append(
            f"guest presents {cpu['logical_cpus']} logical CPUs as {topo['cores']} cores "
            f"(threads/core histogram {topo['threads_per_core_histogram']}) with no P/E-core labels "
            "(/sys/devices/cpu_core and cpu_atom absent); this topology is synthetic under Hyper-V"
        )
    capture = {
        "memory_available_bytes": meminfo.get("MemAvailable"),
        "cgroup_path": cgroup_rel,
        "pid1_comm": read_first_line("/proc/1/comm"),
    }
    try:
        capture["load_average"] = list(os.getloadavg())
    except OSError:
        pass
    up = read_first_line("/proc/uptime")
    if up:
        capture["uptime_seconds"] = int(float(up.split()[0]))
    return plat, capture


def linux_filesystems(paths):
    rows = []
    for line in (read_text("/proc/self/mountinfo", "") or "").splitlines():
        parts = line.split(" ")
        try:
            sep = parts.index("-")
        except ValueError:
            continue
        unesc = lambda s: re.sub(r"\\([0-7]{3})", lambda m: chr(int(m.group(1), 8)), s)  # noqa: E731
        rows.append({
            "mount_point": unesc(parts[4]),
            "mount_options": parts[5],
            "fstype": parts[sep + 1],
            "source": unesc(parts[sep + 2]),
            "super_options": parts[sep + 3] if len(parts) > sep + 3 else "",
        })
    stable, capture = [], []
    for p in paths:
        existing = nearest_existing(p)
        real = os.path.realpath(existing)
        best = None
        for i, r in enumerate(rows):
            mp = r["mount_point"]
            if real == mp or mp == "/" or real.startswith(mp.rstrip("/") + "/"):
                if best is None or len(mp) >= len(best[1]["mount_point"]):
                    best = (i, r)
        entry = {"path": p}
        cap = {"path": p, "exists": os.path.exists(p), "resolved_via": existing if existing != p else None}
        if best:
            r = best[1]
            # Drop per-boot / per-container values (9p channel fds, overlay
            # snapshot directories) so the hashed options stay stable.
            opts = [
                o for o in r["super_options"].split(",")
                if o and not re.match(r"^(rfd|wfd|lowerdir|upperdir|workdir|index|metacopy)=", o)
            ]
            entry.update({
                "mount_point": r["mount_point"],
                "fstype": r["fstype"],
                "mount_options": r["mount_options"],
                "super_options": ",".join(opts),
            })
            cap["mount_source"] = r["source"]
            if r["fstype"] == "9p" and "aname=drvfs" in r["super_options"] and "metadata" not in r["super_options"]:
                OBSERVATIONS.append(
                    f"{p} is on WSL drvfs (9p) without the 'metadata' option: chmod/chown/mode bits and "
                    "POSIX ownership are not persisted there; use ext4 (/root/eb-research) for metadata fidelity tests"
                )
        try:
            sv = os.statvfs(existing)
            entry["block_size"] = sv.f_bsize
            entry["name_max"] = sv.f_namemax
            cap["total_bytes"] = sv.f_blocks * sv.f_frsize
            cap["free_bytes"] = sv.f_bavail * sv.f_frsize
        except OSError:
            pass
        stable.append(entry)
        capture.append(cap)
    return stable, capture


# --------------------------------------------------------------------------
# Windows platform
# --------------------------------------------------------------------------

POWER_SUBGROUP_PROCESSOR = "54533251-82be-4824-96c1-47b60b740d00"
POWER_SETTINGS = [
    ("PROCTHROTTLEMIN", "893dee8e-2bef-41e0-89c6-b55d0929964c"),
    ("PROCTHROTTLEMIN1", "893dee8e-2bef-41e0-89c6-b55d0929964d"),
    ("PROCTHROTTLEMAX", "bc5038f7-23e0-4960-96da-33abaf5935ec"),
    ("PROCTHROTTLEMAX1", "bc5038f7-23e0-4960-96da-33abaf5935ed"),
    ("PERFBOOSTMODE", "be337238-0d82-4146-a960-4f3749d470c7"),
    ("PERFBOOSTPOL", "45bcc044-d885-43e2-8605-ee0ec6e96b59"),
    ("PERFEPP", "36687f9e-e3a5-4dbf-b1dc-15eb381c6863"),
    ("PERFEPP1", "36687f9e-e3a5-4dbf-b1dc-15eb381c6864"),
    ("HETEROPOLICY", "7f2f5cfa-f10c-4823-b5e1-e93ae85f46b5"),
    ("SCHEDPOLICY", "93b8b6dc-0698-4d1c-9ee4-0644e900c85d"),
    ("CPMINCORES", "0cc5b647-c1df-4637-891a-dec35c318583"),
    ("CPMAXCORES", "ea062031-0e34-4ff1-9b6d-eb1059334028"),
    ("SYSCOOLPOL", "94d3a615-a899-4ac5-ae2b-e4d8f634367f"),
]
OVERLAY_LABELS = {
    "00000000-0000-0000-0000-000000000000": "Balanced (no overlay)",
    "961cc777-2547-4f9d-8174-7d86181b8a7a": "Best power efficiency",
    "3af9b8d9-7c97-431d-ad78-34a8bfea439f": "Better performance",
    "ded574b5-45a0-4f42-8737-46345c09c238": "Best performance",
}
PC_SYSTEM_TYPES = {0: "Unspecified", 1: "Desktop", 2: "Mobile", 3: "Workstation", 4: "Enterprise Server",
                   5: "SOHO Server", 6: "Appliance PC", 7: "Performance Server", 8: "Maximum"}
VOLUME_FLAGS = [
    (0x00000001, "CASE_SENSITIVE_SEARCH"), (0x00000002, "CASE_PRESERVED_NAMES"), (0x00000004, "UNICODE_ON_DISK"),
    (0x00000008, "PERSISTENT_ACLS"), (0x00000010, "FILE_COMPRESSION"), (0x00000040, "SPARSE_FILES"),
    (0x00000080, "REPARSE_POINTS"), (0x00008000, "VOLUME_IS_COMPRESSED"), (0x00010000, "OBJECT_IDS"),
    (0x00020000, "ENCRYPTION"), (0x00040000, "NAMED_STREAMS"), (0x00080000, "READ_ONLY_VOLUME"),
    (0x00400000, "HARD_LINKS"), (0x00800000, "EXTENDED_ATTRIBUTES"), (0x02000000, "USN_JOURNAL"),
    (0x08000000, "BLOCK_REFCOUNTING"), (0x20000000, "DAX_VOLUME"),
]


def _reg_values(key_path, names):
    import winreg

    out = {}
    try:
        with winreg.OpenKey(winreg.HKEY_LOCAL_MACHINE, key_path) as k:
            for n in names:
                try:
                    out[n] = winreg.QueryValueEx(k, n)[0]
                except OSError:
                    out[n] = None
    except OSError as exc:
        out["_error"] = str(exc)
    return out


def wsl_version():
    exe = shutil.which("wsl.exe") or shutil.which("wsl")
    if not exe:
        return None
    r = run([exe, "--version"], timeout=60)
    if not r["ok"]:
        return {"error": (r["error"] or r["stderr"].strip() or f"rc={r['rc']}")[:200]}
    out = {}
    for line in r["stdout"].splitlines():
        k, sep, v = line.partition(":")
        if sep and v.strip():
            out[k.strip()] = v.strip()
    return out


_CIM_SCRIPT = r"""
$ErrorActionPreference = 'Stop'
$o = [ordered]@{}
try { $p = Get-CimInstance Win32_Processor | Select-Object -First 1
  $o.processor = [ordered]@{ Name=$p.Name; Manufacturer=$p.Manufacturer; NumberOfCores=$p.NumberOfCores;
    NumberOfEnabledCore=$p.NumberOfEnabledCore; NumberOfLogicalProcessors=$p.NumberOfLogicalProcessors;
    ThreadCount=$p.ThreadCount; MaxClockSpeed=$p.MaxClockSpeed; L2CacheSize=$p.L2CacheSize; L3CacheSize=$p.L3CacheSize;
    VirtualizationFirmwareEnabled=$p.VirtualizationFirmwareEnabled } } catch { $o.processor_error = $_.Exception.Message }
try { $cs = Get-CimInstance Win32_ComputerSystem
  $o.computer_system = [ordered]@{ HypervisorPresent=$cs.HypervisorPresent; PCSystemType=[int]$cs.PCSystemType;
    Manufacturer=$cs.Manufacturer; Model=$cs.Model; NumberOfProcessors=$cs.NumberOfProcessors;
    TotalPhysicalMemory=[string]$cs.TotalPhysicalMemory } } catch { $o.computer_system_error = $_.Exception.Message }
try { $b = Get-CimInstance Win32_BIOS
  $o.bios = [ordered]@{ Manufacturer=$b.Manufacturer; SMBIOSBIOSVersion=$b.SMBIOSBIOSVersion;
    ReleaseDate=$(if ($b.ReleaseDate) { $b.ReleaseDate.ToString('yyyy-MM-dd') } else { $null }) } } catch { $o.bios_error = $_.Exception.Message }
try { $mods = @(Get-CimInstance Win32_PhysicalMemory)
  $o.memory_modules = @($mods | ForEach-Object { [ordered]@{ Capacity=[string]$_.Capacity; Speed=$_.Speed;
    ConfiguredClockSpeed=$_.ConfiguredClockSpeed; SMBIOSMemoryType=$_.SMBIOSMemoryType; FormFactor=$_.FormFactor } }) } catch { $o.memory_modules_error = $_.Exception.Message }
try { $o.battery_count = @(Get-CimInstance Win32_Battery).Count } catch { $o.battery_error = $_.Exception.Message }
try { $dg = Get-CimInstance -Namespace root\Microsoft\Windows\DeviceGuard -ClassName Win32_DeviceGuard
  $o.device_guard = [ordered]@{ VirtualizationBasedSecurityStatus=$dg.VirtualizationBasedSecurityStatus;
    SecurityServicesRunning=@($dg.SecurityServicesRunning); SecurityServicesConfigured=@($dg.SecurityServicesConfigured) } } catch { $o.device_guard_error = $_.Exception.Message }
try { $mp = Get-MpComputerStatus
  $o.defender = [ordered]@{ AMRunningMode=[string]$mp.AMRunningMode; AntivirusEnabled=$mp.AntivirusEnabled;
    RealTimeProtectionEnabled=$mp.RealTimeProtectionEnabled; IsTamperProtected=$mp.IsTamperProtected } } catch { $o.defender_error = $_.Exception.Message }
$o | ConvertTo-Json -Depth 6 -Compress
"""


def windows_cim():
    import base64

    ps = shutil.which("powershell.exe") or shutil.which("pwsh.exe")
    if not ps:
        return {"error": "no PowerShell"}
    enc = base64.b64encode(_CIM_SCRIPT.encode("utf-16-le")).decode("ascii")
    r = run([ps, "-NoProfile", "-NonInteractive", "-EncodedCommand", enc], timeout=180)
    try:
        return json.loads(r["stdout"].strip())
    except json.JSONDecodeError:
        return {"error": (r["error"] or r["stderr"].strip() or "unparseable output")[:400]}


def windows_topology():
    import ctypes

    k32 = ctypes.WinDLL("kernel32", use_last_error=True)
    k32.GetLogicalProcessorInformationEx.argtypes = [ctypes.c_int, ctypes.c_void_p, ctypes.POINTER(ctypes.c_ulong)]
    k32.GetLogicalProcessorInformationEx.restype = ctypes.c_int
    size = ctypes.c_ulong(0)
    k32.GetLogicalProcessorInformationEx(0xFFFF, None, ctypes.byref(size))
    if size.value == 0:
        return {"error": f"GetLogicalProcessorInformationEx size query failed ({ctypes.get_last_error()})"}
    buf = ctypes.create_string_buffer(size.value)
    if not k32.GetLogicalProcessorInformationEx(0xFFFF, buf, ctypes.byref(size)):
        return {"error": f"GetLogicalProcessorInformationEx failed ({ctypes.get_last_error()})"}
    data = buf.raw[: size.value]

    def masks(body_off, count_off, first_mask_off):
        count = struct.unpack_from("<H", data, body_off + count_off)[0] or 1
        ids = []
        for i in range(count):
            mask, group = struct.unpack_from("<QH", data, body_off + first_mask_off + 16 * i)
            ids.extend(group * 64 + bit for bit in range(64) if (mask >> bit) & 1)
        return ids

    cores, packages, numa, caches, groups = [], [], [], [], []
    off = 0
    while off + 8 <= len(data):
        rel, rec_size = struct.unpack_from("<II", data, off)
        if rec_size == 0:
            break
        body = off + 8
        if rel in (0, 3):
            flags, eff = struct.unpack_from("<BB", data, body)
            entry = {"smt": bool(flags & 1), "efficiency_class": eff, "lps": masks(body, 22, 24)}
            (cores if rel == 0 else packages).append(entry)
        elif rel == 1:
            node = struct.unpack_from("<I", data, body)[0]
            numa.append({"node": node, "lps": masks(body, 22, 24)})
        elif rel == 2:
            level, assoc, line_size, cache_size, ctype = struct.unpack_from("<BBHII", data, body)
            caches.append({
                "level": level, "associativity": assoc, "line_size": line_size, "size_bytes": cache_size,
                "type": {0: "Unified", 1: "Instruction", 2: "Data", 3: "Trace"}.get(ctype, str(ctype)),
                "lps": masks(body, 30, 32),
            })
        elif rel == 4:
            max_groups, active_groups = struct.unpack_from("<HH", data, body)
            for g in range(active_groups):
                max_lp, active_lp = struct.unpack_from("<BB", data, body + 24 + 48 * g)
                groups.append({"group": g, "maximum_processors": max_lp, "active_processors": active_lp})
        off += rec_size

    classes = sorted({c["efficiency_class"] for c in cores})
    by_class = []
    for ec in sorted(classes, reverse=True):
        members = [c for c in cores if c["efficiency_class"] == ec]
        lps = sorted(i for c in members for i in c["lps"])
        if len(classes) > 1:
            label = "P-core" if ec == max(classes) else ("E-core" if ec == min(classes) else f"class-{ec}")
        else:
            label = "homogeneous"
        # map each core's L2 cache size (first L2 cache covering its first logical processor)
        l2 = sorted({
            ca["size_bytes"] for c in members for ca in caches
            if ca["level"] == 2 and c["lps"] and c["lps"][0] in ca["lps"]
        })
        by_class.append({
            "efficiency_class": ec,
            "label": label,
            "cores": len(members),
            "smt_cores": sum(1 for c in members if c["smt"]),
            "logical_processors": len(lps),
            "logical_processor_ids": compress_ranges(lps),
            "l2_cache_sizes_bytes": l2,
        })
    cache_summary = Counter(
        (ca["level"], ca["type"], ca["size_bytes"], ca["associativity"], ca["line_size"], len(ca["lps"])) for ca in caches
    )
    return {
        "source": "GetLogicalProcessorInformationEx(RelationAll)",
        "packages": len(packages),
        "numa_nodes": len(numa),
        "processor_groups": groups,
        "physical_cores": len(cores),
        "logical_processors": sum(len(c["lps"]) for c in cores),
        "hybrid": len(classes) > 1,
        "efficiency_classes": by_class,
        "caches": [
            {"level": k[0], "type": k[1], "size_bytes": k[2], "associativity": k[3], "line_size": k[4],
             "logical_processors_per_instance": k[5], "instances": n}
            for k, n in sorted(cache_summary.items())
        ],
    }


def windows_power():
    import ctypes
    import winreg

    class GUID(ctypes.Structure):
        _fields_ = [("Data1", ctypes.c_ulong), ("Data2", ctypes.c_ushort), ("Data3", ctypes.c_ushort),
                    ("Data4", ctypes.c_ubyte * 8)]

    def g(s):
        return GUID.from_buffer_copy(uuid.UUID(s).bytes_le)

    def gs(guid):
        return str(uuid.UUID(bytes_le=bytes(guid)))

    out = {}
    try:
        pp = ctypes.WinDLL("powrprof")
        k32 = ctypes.WinDLL("kernel32")
        PG = ctypes.POINTER(GUID)
        pp.PowerGetActiveScheme.argtypes = [ctypes.c_void_p, ctypes.POINTER(PG)]
        pp.PowerGetActiveScheme.restype = ctypes.c_ulong
        pp.PowerReadFriendlyName.argtypes = [ctypes.c_void_p, PG, PG, PG, ctypes.c_void_p, ctypes.POINTER(ctypes.c_ulong)]
        pp.PowerReadFriendlyName.restype = ctypes.c_ulong
        for fn in ("PowerReadACValueIndex", "PowerReadDCValueIndex"):
            getattr(pp, fn).argtypes = [ctypes.c_void_p, PG, PG, PG, ctypes.POINTER(ctypes.c_ulong)]
            getattr(pp, fn).restype = ctypes.c_ulong
        k32.LocalFree.argtypes = [ctypes.c_void_p]
        active = PG()
        rc = pp.PowerGetActiveScheme(None, ctypes.byref(active))
        if rc != 0:
            raise OSError(f"PowerGetActiveScheme rc={rc}")
        scheme = GUID.from_buffer_copy(bytes(active.contents))
        k32.LocalFree(ctypes.cast(active, ctypes.c_void_p))

        def friendly(sub=None, setting=None):
            n = ctypes.c_ulong(0)
            subp = ctypes.byref(sub) if sub is not None else None
            setp = ctypes.byref(setting) if setting is not None else None
            if pp.PowerReadFriendlyName(None, ctypes.byref(scheme), subp, setp, None, ctypes.byref(n)) != 0 or not n.value:
                return None
            b = ctypes.create_string_buffer(n.value)
            if pp.PowerReadFriendlyName(None, ctypes.byref(scheme), subp, setp, b, ctypes.byref(n)) != 0:
                return None
            return b.raw[: n.value].decode("utf-16-le", "replace").rstrip("\x00")

        out["active_scheme"] = {"guid": gs(scheme), "name": friendly()}
        sub = g(POWER_SUBGROUP_PROCESSOR)
        settings = {}
        for alias, sguid in POWER_SETTINGS:
            st = g(sguid)
            ac, dc = ctypes.c_ulong(0), ctypes.c_ulong(0)
            rc_ac = pp.PowerReadACValueIndex(None, ctypes.byref(scheme), ctypes.byref(sub), ctypes.byref(st), ctypes.byref(ac))
            rc_dc = pp.PowerReadDCValueIndex(None, ctypes.byref(scheme), ctypes.byref(sub), ctypes.byref(st), ctypes.byref(dc))
            settings[alias] = {
                "guid": sguid,
                "name": friendly(sub, st),
                "ac": ac.value if rc_ac == 0 else f"error {rc_ac}",
                "dc": dc.value if rc_dc == 0 else f"error {rc_dc}",
            }
        out["processor_settings"] = settings
    except (OSError, AttributeError) as exc:
        out["api_error"] = str(exc)[:300]
    r = run(["powercfg", "/getactivescheme"])
    m = re.search(r"([0-9a-fA-F-]{36})\s+\((.*)\)", r["stdout"])
    out["powercfg_getactivescheme"] = {"guid": m.group(1).lower(), "name": m.group(2)} if m else {"error": (r["error"] or r["stdout"].strip())[:200]}
    try:
        with winreg.OpenKey(winreg.HKEY_LOCAL_MACHINE, r"SYSTEM\CurrentControlSet\Control\Power\User\PowerSchemes") as k:
            ov = {}
            for name, key in (("ac", "ActiveOverlayAcPowerScheme"), ("dc", "ActiveOverlayDcPowerScheme")):
                try:
                    val = str(winreg.QueryValueEx(k, key)[0]).lower()
                    ov[name] = {"guid": val, "label": OVERLAY_LABELS.get(val, "unknown")}
                except OSError:
                    ov[name] = None
            out["power_mode_overlay"] = ov
    except OSError as exc:
        out["power_mode_overlay"] = {"error": str(exc)[:200]}
    name = (out.get("active_scheme") or out.get("powercfg_getactivescheme") or {}).get("name")
    if name:
        OBSERVATIONS.append(
            f"Windows power scheme at capture: {name!r}; AC overlay "
            f"{((out.get('power_mode_overlay') or {}).get('ac') or {}).get('label')}"
        )
    return out


def windows_memory():
    import ctypes

    class MEMORYSTATUSEX(ctypes.Structure):
        _fields_ = [("dwLength", ctypes.c_ulong), ("dwMemoryLoad", ctypes.c_ulong),
                    ("ullTotalPhys", ctypes.c_ulonglong), ("ullAvailPhys", ctypes.c_ulonglong),
                    ("ullTotalPageFile", ctypes.c_ulonglong), ("ullAvailPageFile", ctypes.c_ulonglong),
                    ("ullTotalVirtual", ctypes.c_ulonglong), ("ullAvailVirtual", ctypes.c_ulonglong),
                    ("ullAvailExtendedVirtual", ctypes.c_ulonglong)]

    k32 = ctypes.WinDLL("kernel32")
    ms = MEMORYSTATUSEX()
    ms.dwLength = ctypes.sizeof(MEMORYSTATUSEX)
    k32.GlobalMemoryStatusEx(ctypes.byref(ms))
    installed = ctypes.c_ulonglong(0)
    ok = k32.GetPhysicallyInstalledSystemMemory(ctypes.byref(installed))
    stable = {
        "total_physical_bytes": ms.ullTotalPhys,
        "installed_physical_bytes": installed.value * 1024 if ok else None,
    }
    capture = {
        "memory_available_bytes": ms.ullAvailPhys,
        "memory_load_percent": ms.dwMemoryLoad,
        "commit_limit_bytes": ms.ullTotalPageFile,
    }
    return stable, capture


def windows_power_status():
    import ctypes

    class SPS(ctypes.Structure):
        _fields_ = [("ACLineStatus", ctypes.c_ubyte), ("BatteryFlag", ctypes.c_ubyte),
                    ("BatteryLifePercent", ctypes.c_ubyte), ("SystemStatusFlag", ctypes.c_ubyte),
                    ("BatteryLifeTime", ctypes.c_ulong), ("BatteryFullLifeTime", ctypes.c_ulong)]

    s = SPS()
    if not ctypes.WinDLL("kernel32").GetSystemPowerStatus(ctypes.byref(s)):
        return None
    return {
        "ac_line_status": {0: "offline", 1: "online"}.get(s.ACLineStatus, "unknown"),
        "battery_percent": None if s.BatteryLifePercent == 255 else s.BatteryLifePercent,
        "battery_saver_on": bool(s.SystemStatusFlag & 1),
    }


def windows_platform(parent_platform_id):
    import ctypes

    cv = _reg_values(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion",
                     ["ProductName", "DisplayVersion", "CurrentBuild", "UBR", "EditionID", "InstallationType"])
    try:
        build = int(cv.get("CurrentBuild") or 0)
    except ValueError:
        build = 0
    cpu_reg = _reg_values(r"HARDWARE\DESCRIPTION\System\CentralProcessor\0",
                          ["ProcessorNameString", "Identifier", "VendorIdentifier", "~MHz", "Update Revision"])
    upd = cpu_reg.get("Update Revision")
    microcode = None
    if isinstance(upd, (bytes, bytearray)) and len(upd) >= 4:
        raw = bytes(upd)
        microcode = hex(int.from_bytes(raw[4:8] if len(raw) >= 8 else raw[:4], "little"))
    cim = windows_cim()
    topo = windows_topology()
    mem_stable, mem_capture = windows_memory()
    cs = cim.get("computer_system") or {}
    dg = cim.get("device_guard") or {}
    proc = cim.get("processor") or {}
    plat = {
        "os": {
            "family": "windows",
            "marketing_name": ("Windows 11" if build >= 22000 else "Windows 10") if build else None,
            "product_name_registry": cv.get("ProductName"),
            "edition_id": cv.get("EditionID"),
            "display_version": cv.get("DisplayVersion"),
            "installation_type": cv.get("InstallationType"),
            "build": build,
            "ubr": cv.get("UBR"),
            "version": f"10.0.{build}.{cv.get('UBR')}",
            "architecture": platform.machine(),
        },
        "kernel": {"nt_version": f"10.0.{build}.{cv.get('UBR')}", "platform_version": platform.version()},
        "virtualization": {
            "hypervisor_present": cs.get("HypervisorPresent"),
            "vbs_status": dg.get("VirtualizationBasedSecurityStatus"),
            "vbs_status_label": {0: "off", 1: "enabled-not-running", 2: "running"}.get(dg.get("VirtualizationBasedSecurityStatus")),
            "security_services_running": dg.get("SecurityServicesRunning"),
            "security_services_configured": dg.get("SecurityServicesConfigured"),
            "virtualization_firmware_enabled": proc.get("VirtualizationFirmwareEnabled"),
            "wsl": wsl_version(),
        },
        "cpu": {
            "model_name": cpu_reg.get("ProcessorNameString", "").strip() if cpu_reg.get("ProcessorNameString") else None,
            "identifier": cpu_reg.get("Identifier"),
            "vendor_id": cpu_reg.get("VendorIdentifier"),
            "microcode_revision_registry": microcode,
            "logical_cpus": os.cpu_count(),
            "physical_cores": topo.get("physical_cores"),
            "p_cores": sum(c["cores"] for c in topo.get("efficiency_classes", []) if c["label"] == "P-core"),
            "e_cores": sum(c["cores"] for c in topo.get("efficiency_classes", []) if c["label"] == "E-core"),
            "topology": topo,
            "cim_win32_processor": proc or cim.get("processor_error"),
        },
        "memory": {
            **mem_stable,
            "modules": cim.get("memory_modules") or cim.get("memory_modules_error"),
        },
        "power": windows_power(),
        "hardware": {
            "pc_system_type": cs.get("PCSystemType"),
            "pc_system_type_label": PC_SYSTEM_TYPES.get(cs.get("PCSystemType")),
            "manufacturer": cs.get("Manufacturer"),
            "model": cs.get("Model"),
            "bios": cim.get("bios") or cim.get("bios_error"),
            "battery_count": cim.get("battery_count"),
        },
        "security": {"defender": cim.get("defender") or cim.get("defender_error")},
    }
    if parent_platform_id:
        plat["parent"] = {"relation": "hosted-by", "platform_id": parent_platform_id}
    for cls in topo.get("efficiency_classes", []):
        OBSERVATIONS.append(
            f"{cls['label']} (EfficiencyClass {cls['efficiency_class']}): {cls['cores']} cores, "
            f"{cls['logical_processors']} logical processors, ids {cls['logical_processor_ids']}"
        )
    if (cim.get("defender") or {}).get("RealTimeProtectionEnabled"):
        OBSERVATIONS.append("Microsoft Defender real-time protection is on; Windows-side file I/O timings include AV scanning")
    if cs.get("HypervisorPresent"):
        OBSERVATIONS.append("Hyper-V hypervisor is present (VBS/WSL2): the Windows host itself runs as the root partition")
    capture = dict(mem_capture)
    capture["power_status"] = windows_power_status()
    capture["cpu_registry_mhz"] = cpu_reg.get("~MHz")
    try:
        k32 = ctypes.WinDLL("kernel32")
        k32.GetTickCount64.restype = ctypes.c_ulonglong
        capture["uptime_seconds"] = int(k32.GetTickCount64() // 1000)
    except OSError:
        pass
    return plat, capture


def windows_filesystems(paths):
    import ctypes

    k32 = ctypes.WinDLL("kernel32", use_last_error=True)
    stable, capture = [], []
    for p in paths:
        existing = nearest_existing(p)
        entry = {"path": p}
        cap = {"path": p, "exists": os.path.exists(p), "resolved_via": existing if existing != os.path.abspath(p) else None}
        root = ctypes.create_unicode_buffer(262)
        if k32.GetVolumePathNameW(ctypes.c_wchar_p(existing), root, 262):
            fsname = ctypes.create_unicode_buffer(262)
            maxcomp, flags, serial = ctypes.c_ulong(0), ctypes.c_ulong(0), ctypes.c_ulong(0)
            ok = k32.GetVolumeInformationW(ctypes.c_wchar_p(root.value), None, 0, ctypes.byref(serial),
                                           ctypes.byref(maxcomp), ctypes.byref(flags), fsname, 262)
            entry["volume_root"] = root.value
            entry["drive_type"] = {0: "unknown", 1: "no-root", 2: "removable", 3: "fixed", 4: "remote",
                                   5: "cdrom", 6: "ramdisk"}.get(k32.GetDriveTypeW(ctypes.c_wchar_p(root.value)))
            if ok:
                entry["fstype"] = fsname.value
                entry["max_component_length"] = maxcomp.value
                entry["flags"] = [n for bit, n in VOLUME_FLAGS if flags.value & bit]
            else:
                entry["error"] = f"GetVolumeInformationW failed ({ctypes.get_last_error()})"
            free, total = ctypes.c_ulonglong(0), ctypes.c_ulonglong(0)
            if k32.GetDiskFreeSpaceExW(ctypes.c_wchar_p(root.value), ctypes.byref(free), ctypes.byref(total), None):
                cap["free_bytes"] = free.value
                cap["total_bytes"] = total.value
        stable.append(entry)
        capture.append(cap)
    return stable, capture


# --------------------------------------------------------------------------
# Capture
# --------------------------------------------------------------------------


def detect_kind():
    if IS_WINDOWS:
        return "windows-host"
    if os.path.exists("/.dockerenv") or os.path.exists("/run/.containerenv"):
        return "container"
    if "microsoft-standard-wsl2" in platform.release().lower():
        return "wsl"
    return "linux"


def compute_ids(stable):
    assert_no_floats(stable)
    without_repo = {k: v for k, v in stable.items() if k != "repo"}
    return {
        "env_id": sha256_bytes(canonical_json(stable)),
        "env_id_without_repo": sha256_bytes(canonical_json(without_repo)),
        "platform_id": sha256_bytes(canonical_json(stable["platform"])),
    }


def capture(args):
    OBSERVATIONS.clear()
    kind = detect_kind()
    repo = Path(args.repo).resolve() if args.repo else None
    cwd = str(repo) if repo and repo.is_dir() else os.getcwd()
    parent = args.parent_platform_id
    if args.parent_from_index:
        if not args.out_dir:
            raise FingerprintError("--parent-from-index requires --out-dir")
        parent = lookup_field(Path(args.out_dir), args.parent_from_index, "platform_id")
    if parent and not re.fullmatch(r"[0-9a-f]{64}", parent):
        raise FingerprintError(f"invalid parent platform id: {parent!r}")

    work_dirs = [os.path.abspath(p) for p in (args.work_dir or [])]
    if IS_WINDOWS:
        plat, plat_cap = windows_platform(parent)
        fs_stable, fs_cap = windows_filesystems(work_dirs)
    else:
        plat, plat_cap = linux_platform(kind, parent)
        fs_stable, fs_cap = linux_filesystems(work_dirs)

    repo_stable, repo_cap = repo_info(repo)
    docker_stable, docker_cap = docker_info(cwd)
    stable = {
        "schema": SCHEMA,
        "kind": kind,
        "platform": plat,
        "filesystems": fs_stable,
        "process": process_info(),
        "rust": rust_info(cwd),
        "tools": tools_info(cwd),
        "docker": docker_stable,
        "repo": repo_stable,
    }
    container = {}
    for key in ("base_image", "platform_manifest", "apt_snapshot", "packages", "recipe_sha256", "run_flags"):
        val = getattr(args, f"container_{key}")
        if val:
            container[key] = val
    if container:
        stable["container"] = container

    now = _dt.datetime.now(_dt.timezone.utc).replace(microsecond=0)
    cap = {
        "name": args.name,
        "captured_at_utc": now.isoformat().replace("+00:00", "Z"),
        "capture_tool": {"file": "research/tools/env/capture_env.py", "sha256": sha256_file(__file__)},
        "platform": plat_cap,
        "filesystems": fs_cap,
        "repo_worktree": repo_cap,
        **docker_cap,
    }
    for item in args.capture_note or []:
        k, sep, v = item.partition("=")
        if not sep:
            raise FingerprintError(f"--capture-note expects KEY=VALUE, got {item!r}")
        cap.setdefault("notes", {})[k] = v
    stable = normalize(stable)
    ids = compute_ids(stable)
    cap["observations"] = sorted(set(OBSERVATIONS))
    doc = {
        "schema": SCHEMA,
        **ids,
        "id_algorithm": ID_ALGORITHM,
        "stable": stable,
        "capture": normalize(cap),
    }
    return doc


# --------------------------------------------------------------------------
# Store / index / verify
# --------------------------------------------------------------------------


def verify_document(doc, expected_env_id=None):
    if doc.get("schema") != SCHEMA:
        raise FingerprintError(f"unexpected schema {doc.get('schema')!r}")
    stable = doc.get("stable")
    if not isinstance(stable, dict) or "platform" not in stable:
        raise FingerprintError("fingerprint without stable.platform")
    ids = compute_ids(stable)
    for k, v in ids.items():
        if doc.get(k) != v:
            raise FingerprintError(f"HASH MISMATCH: recorded {k}={doc.get(k)} but stable section hashes to {v}")
    if expected_env_id and ids["env_id"] != expected_env_id:
        raise FingerprintError(f"HASH MISMATCH: file named {expected_env_id} contains env_id {ids['env_id']}")
    return ids


def _write_json(path: Path, obj):
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = path.with_suffix(path.suffix + ".tmp")
    tmp.write_text(json.dumps(obj, indent=2, ensure_ascii=False) + "\n", encoding="utf-8", newline="\n")
    os.replace(tmp, path)


def load_index(out_dir: Path):
    path = out_dir / "index.json"
    if not path.exists():
        return {"schema": INDEX_SCHEMA, "environments": {}, "history": {}}
    idx = json.loads(path.read_text(encoding="utf-8"))
    if idx.get("schema") != INDEX_SCHEMA:
        raise FingerprintError(f"{path}: unexpected schema {idx.get('schema')!r}")
    idx.setdefault("environments", {})
    idx.setdefault("history", {})
    return idx


def store_document(doc, name, out_dir: Path):
    if not re.fullmatch(r"[a-z0-9][a-z0-9._-]*", name or ""):
        raise FingerprintError(f"invalid environment name {name!r}")
    ids = verify_document(doc)
    env_id = ids["env_id"]
    path = out_dir / f"{env_id}.json"
    if path.exists():
        existing = json.loads(path.read_text(encoding="utf-8"))
        verify_document(existing, expected_env_id=env_id)
        if canonical_json(existing["stable"]) != canonical_json(doc["stable"]):
            raise FingerprintError(f"HASH COLLISION? {path} has a different stable section")
        status = "unchanged (fingerprint matches; existing file kept)"
    else:
        _write_json(path, doc)
        status = "written"
    idx = load_index(out_dir)
    before = canonical_json(idx)
    idx["environments"][name] = env_id
    hist = idx["history"].setdefault(name, [])
    if env_id not in hist:
        hist.append(env_id)
    idx["environments"] = dict(sorted(idx["environments"].items()))
    idx["history"] = dict(sorted(idx["history"].items()))
    if canonical_json(idx) != before:
        _write_json(out_dir / "index.json", idx)
    return env_id, status


def lookup_field(out_dir: Path, name, field):
    idx = load_index(out_dir)
    env_id = idx["environments"].get(name)
    if not env_id:
        raise FingerprintError(f"environment {name!r} not in {out_dir / 'index.json'}")
    doc = json.loads((out_dir / f"{env_id}.json").read_text(encoding="utf-8"))
    ids = verify_document(doc, expected_env_id=env_id)
    if field not in ids:
        raise FingerprintError(f"unknown id field {field!r}")
    return ids[field]


def verify_dir(out_dir: Path):
    problems = []
    docs = {}
    for f in sorted(out_dir.glob("*.json")):
        if f.name == "index.json":
            continue
        if not re.fullmatch(r"[0-9a-f]{64}\.json", f.name):
            continue
        try:
            doc = json.loads(f.read_text(encoding="utf-8"))
            docs[f.stem] = verify_document(doc, expected_env_id=f.stem)
        except (FingerprintError, json.JSONDecodeError, KeyError) as exc:
            problems.append(f"{f.name}: {exc}")
    try:
        idx = load_index(out_dir)
        for name, env_id in idx["environments"].items():
            if env_id not in docs:
                problems.append(f"index: {name} -> {env_id} has no valid fingerprint file")
            if env_id not in idx["history"].get(name, []):
                problems.append(f"index: {name} current id missing from history")
        for name, ids in idx["history"].items():
            for env_id in ids:
                if env_id not in docs:
                    problems.append(f"index history: {name} -> {env_id} has no valid fingerprint file")
    except (FingerprintError, json.JSONDecodeError) as exc:
        problems.append(f"index.json: {exc}")
    return docs, problems


# --------------------------------------------------------------------------
# CLI
# --------------------------------------------------------------------------


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)

    c = sub.add_parser("capture", help="probe this environment")
    c.add_argument("--name", required=True, help="index name, e.g. windows-host, wsl-ubuntu")
    c.add_argument("--repo", help="repository root (git HEAD, Cargo.lock, rust-toolchain.toml)")
    c.add_argument("--work-dir", action="append", help="working directory whose filesystem is recorded (repeatable)")
    c.add_argument("--out-dir", help="store <env_id>.json and update index.json here")
    c.add_argument("--stdout", action="store_true", help="print the fingerprint JSON to stdout")
    c.add_argument("--parent-platform-id", help="platform_id of the physical host this environment runs on")
    c.add_argument("--parent-from-index", metavar="NAME", help="read the parent platform_id from index.json in --out-dir")
    c.add_argument("--container-base-image")
    c.add_argument("--container-platform-manifest")
    c.add_argument("--container-apt-snapshot")
    c.add_argument("--container-packages")
    c.add_argument("--container-recipe-sha256")
    c.add_argument("--container-run-flags")
    c.add_argument("--capture-note", action="append", help="KEY=VALUE recorded in the unhashed capture section")

    s = sub.add_parser("store", help="verify and store a fingerprint JSON file")
    s.add_argument("--input", required=True, help="fingerprint JSON path, or - for stdin")
    s.add_argument("--name", required=True)
    s.add_argument("--out-dir", required=True)

    v = sub.add_parser("verify", help="re-hash stored fingerprints and check index.json")
    v.add_argument("--out-dir", required=True)

    lk = sub.add_parser("lookup", help="print an id of a named environment")
    lk.add_argument("--out-dir", required=True)
    lk.add_argument("--name", required=True)
    lk.add_argument("--field", default="env_id", choices=["env_id", "env_id_without_repo", "platform_id"])

    args = ap.parse_args(argv)
    try:
        if args.cmd == "capture":
            doc = capture(args)
            if args.out_dir:
                env_id, status = store_document(doc, args.name, Path(args.out_dir))
                print(f"{args.name}: env_id={env_id} platform_id={doc['platform_id']} [{status}]", file=sys.stderr)
                for obs in doc["capture"]["observations"]:
                    print(f"  observation: {obs}", file=sys.stderr)
            if args.stdout or not args.out_dir:
                sys.stdout.write(json.dumps(doc, indent=2, ensure_ascii=False) + "\n")
        elif args.cmd == "store":
            raw = sys.stdin.read() if args.input == "-" else Path(args.input).read_text(encoding="utf-8")
            doc = json.loads(raw)
            env_id, status = store_document(doc, args.name, Path(args.out_dir))
            print(f"{args.name}: env_id={env_id} platform_id={doc['platform_id']} [{status}]", file=sys.stderr)
        elif args.cmd == "verify":
            docs, problems = verify_dir(Path(args.out_dir))
            for p in problems:
                print(f"FAIL {p}", file=sys.stderr)
            print(f"verified {len(docs)} fingerprint(s); {len(problems)} problem(s)", file=sys.stderr)
            return 1 if problems else 0
        elif args.cmd == "lookup":
            print(lookup_field(Path(args.out_dir), args.name, args.field))
    except FingerprintError as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main())
