"""Shared helpers for the Entrybound research corpus (ebrc) tools.

Stdlib only.  Used by fingerprint.py, provision.py, stats.py and assemble.py, and
importable by experiment runners for the held-out guard (``assert_not_heldout``)
and item path resolution (``Layout.item_path``).
"""

from __future__ import annotations

import contextlib
import datetime as _dt
import fcntl
import hashlib
import json
import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path, PurePosixPath

CORPUS_VERSION = "ebrc-2026.09-v1"
SOURCES_SCHEMA = "ebrc-sources-v1"
RECORD_FORMAT = "ebrc-fingerprint-record-v1"
PINS_FORMAT = "ebrc-pins-v1"

MiB = 1 << 20
GiB = 1 << 30

TOOLS_DIR = Path(__file__).resolve().parent
REPO_ROOT = TOOLS_DIR.parents[2]
DEFAULT_CORPUS_DIR = REPO_ROOT / "research" / "corpus"
DEFAULT_DATA_ROOT = Path("/root/eb-research")

FAMILIES = {
    "F01": "source-code repositories",
    "F02": "build trees",
    "F03": "dependency/vendor trees",
    "F04": "many-small-file trees",
    "F05": "logs/text",
    "F06": "JSON/XML/CSV/structured text",
    "F07": "scientific/numeric arrays",
    "F08": "databases",
    "F09": "executable/binary objects",
    "F10": "highly redundant binaries",
    "F11": "already-compressed files",
    "F12": "JPEG/images",
    "F13": "other media",
    "F14": "archive-inside-archive",
    "F15": "sparse files",
    "F16": "VM/disk-like images",
    "F17": "duplicate trees",
    "F18": "near-duplicate/versioned trees",
    "F19": "metadata-heavy filesystem trees",
    "F20": "adversarial/high-entropy inputs",
}
SPLITS = ("tuning", "validation", "heldout")
SCALES = {"small": 16 * MiB, "medium": 512 * MiB, "large": 8 * GiB}
SCALE_ORDER = ("small", "medium", "large")
KINDS = ("download", "generate", "derive", "docker-export", "git-archive", "build")
REAL_OR_GENERATED = ("real", "generated", "derived-from-real")
INTERPRETERS = ("python", "bash", "sh")
STEP_OPS = ("extract", "decompress", "copy", "copy-item", "remove", "run")

# Default output pin policy per kind: "TOFU" pins the first materialized
# logical_tree_sha256; None records it without pinning (non-reproducible outputs).
DEFAULT_OUTPUT_PIN = {
    "download": "TOFU",
    "generate": "TOFU",
    "derive": "TOFU",
    "git-archive": "TOFU",
    "docker-export": None,
    "build": None,
}

KEBAB_RE = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*$")
HEX64_RE = re.compile(r"^[0-9a-f]{64}$")
HEX40_RE = re.compile(r"^[0-9a-f]{40}$")
INPUT_NAME_RE = re.compile(r"^[a-z0-9][a-z0-9_-]{0,63}$")
DOCKER_IMAGE_RE = re.compile(r"^[a-z0-9][a-z0-9._/-]*(?::[A-Za-z0-9_.-]+)?@sha256:[0-9a-f]{64}$")
SAFE_BASENAME_RE = re.compile(r"^[A-Za-z0-9._+-]{1,200}$")


class CorpusError(Exception):
    """Generic loud failure."""


class HashMismatch(CorpusError):
    """A pinned or recorded hash did not match."""


class HeldoutAccessError(CorpusError):
    """Attempted content access to the held-out split without an unlock."""


# ---------------------------------------------------------------------------
# small utilities


def utc_now() -> str:
    return _dt.datetime.now(_dt.timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")


def canonical_json(obj) -> bytes:
    return json.dumps(obj, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode("utf-8")


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_file(path, chunk: int = MiB) -> tuple[str, int]:
    h = hashlib.sha256()
    n = 0
    with open(path, "rb") as f:
        while True:
            b = f.read(chunk)
            if not b:
                break
            h.update(b)
            n += len(b)
    return h.hexdigest(), n


def write_json_atomic(path, obj, pretty: bool = True) -> None:
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    data = (json.dumps(obj, indent=2, ensure_ascii=False) + "\n") if pretty else canonical_json(obj).decode("utf-8")
    fd, tmp = tempfile.mkstemp(prefix="." + path.name + ".", suffix=".tmp", dir=str(path.parent))
    try:
        with os.fdopen(fd, "w", encoding="utf-8", newline="\n") as f:
            f.write(data)
        os.replace(tmp, path)
    except BaseException:
        with contextlib.suppress(OSError):
            os.unlink(tmp)
        raise


def write_text_atomic(path, text: str) -> None:
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, tmp = tempfile.mkstemp(prefix="." + path.name + ".", suffix=".tmp", dir=str(path.parent))
    try:
        with os.fdopen(fd, "w", encoding="utf-8", newline="\n") as f:
            f.write(text)
        os.replace(tmp, path)
    except BaseException:
        with contextlib.suppress(OSError):
            os.unlink(tmp)
        raise


def load_json(path):
    with open(path, "r", encoding="utf-8") as f:
        return json.load(f)


def load_json_if_exists(path):
    try:
        return load_json(path)
    except FileNotFoundError:
        return None


@contextlib.contextmanager
def file_lock(path):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    with open(path, "a+") as f:
        fcntl.flock(f.fileno(), fcntl.LOCK_EX)
        try:
            yield
        finally:
            fcntl.flock(f.fileno(), fcntl.LOCK_UN)


def tool_digests(names=("corpuslib.py", "fingerprint.py", "provision.py", "stats.py", "assemble.py")) -> dict:
    out = {}
    for n in names:
        p = TOOLS_DIR / n
        if p.exists():
            out[n] = sha256_file(p)[0]
    return out


def command_version(cmd, lines: int = 1) -> str | None:
    try:
        r = subprocess.run(cmd, capture_output=True, text=True, timeout=60)
    except (OSError, subprocess.TimeoutExpired):
        return None
    text = (r.stdout or r.stderr or "").strip().splitlines()
    return " | ".join(text[:lines]) if text else None


def git_head(repo: Path = REPO_ROOT) -> str | None:
    """Read-only: current HEAD commit of the repository (None if unavailable)."""
    try:
        r = subprocess.run(["git", "-c", "safe.directory=*", "-C", str(repo), "rev-parse", "HEAD"],
                           capture_output=True, text=True, timeout=60)
    except OSError:
        return None
    return r.stdout.strip() if r.returncode == 0 else None


def require_linux() -> None:
    if not sys.platform.startswith("linux"):
        raise SystemExit("ebrc tools must run inside WSL Ubuntu (Linux); see research/corpus/tools/run.sh")


def repo_rel(path) -> str:
    try:
        return PurePosixPath(Path(path).resolve().relative_to(REPO_ROOT)).as_posix()
    except ValueError:
        return str(path)


def safe_relpath(rel: str, allow_empty: bool = True) -> bool:
    if rel in ("", "."):
        return allow_empty
    if rel.startswith("/") or "\\" in rel or "\x00" in rel:
        return False
    parts = rel.split("/")
    return all(p not in ("", ".", "..") for p in parts)


# ---------------------------------------------------------------------------
# layout


class Layout:
    """All on-disk locations.  Overridable for sandboxes (selftest) via args or
    EB_CORPUS_DIR / EB_DATA_ROOT."""

    def __init__(self, corpus_dir=None, data_root=None):
        self.corpus_dir = Path(corpus_dir or os.environ.get("EB_CORPUS_DIR") or DEFAULT_CORPUS_DIR).resolve()
        self.data_root = Path(data_root or os.environ.get("EB_DATA_ROOT") or DEFAULT_DATA_ROOT)

    # git-tracked (small) ------------------------------------------------
    @property
    def sources_dir(self) -> Path:
        return self.corpus_dir / "sources"

    @property
    def fingerprints_dir(self) -> Path:
        return self.corpus_dir / "fingerprints"

    @property
    def pins_dir(self) -> Path:
        return self.corpus_dir / "pins"

    @property
    def stats_dir(self) -> Path:
        return self.corpus_dir / "stats"

    def record_path(self, item_id: str) -> Path:
        return self.fingerprints_dir / f"{item_id}.json"

    def pins_path(self, item_id: str) -> Path:
        return self.pins_dir / f"{item_id}.json"

    def stats_path(self, item_id: str) -> Path:
        return self.stats_dir / f"{item_id}.json"

    # outside git (large) ------------------------------------------------
    @property
    def heldout_root(self) -> Path:
        return self.data_root / "heldout"

    def split_root(self, split: str) -> Path:
        return self.heldout_root if split == "heldout" else self.data_root / "corpus" / split

    def item_path(self, item_or_id, split: str | None = None, allow_heldout: bool = False) -> Path:
        if isinstance(item_or_id, dict):
            item_id, split = item_or_id["item_id"], item_or_id["split"]
        else:
            item_id = item_or_id
        if split not in SPLITS:
            raise CorpusError(f"unknown split {split!r} for {item_id}")
        if split == "heldout" and not allow_heldout:
            raise HeldoutAccessError(f"{item_id} is a held-out item; its path is not available to this caller")
        return self.split_root(split) / item_id

    def lines_path(self, item: dict) -> Path:
        return self.data_root / "fingerprints" / item["split"] / f"{item['item_id']}.lines.gz"

    def state_path(self, item_id: str) -> Path:
        return self.data_root / "state" / f"{item_id}.json"

    def lock_path(self, name: str) -> Path:
        return self.data_root / "locks" / f"{name}.lock"

    @property
    def cache_dir(self) -> Path:
        return self.data_root / "cache"

    @property
    def staging_dir(self) -> Path:
        return self.data_root / "staging"

    @property
    def scratch_dir(self) -> Path:
        return self.data_root / "scratch"

    @property
    def quarantine_dir(self) -> Path:
        return self.data_root / "quarantine"

    @property
    def log_dir(self) -> Path:
        return self.data_root / "logs" / "provision"


# ---------------------------------------------------------------------------
# held-out guard


def is_heldout_path(path, layout: Layout | None = None) -> bool:
    layout = layout or Layout()
    try:
        real = Path(os.path.realpath(path))
        root = Path(os.path.realpath(layout.heldout_root))
    except OSError:
        return False
    return real == root or root in real.parents


def assert_not_heldout(path, layout: Layout | None = None) -> None:
    """Experiment/tuning code must call this before reading any corpus path."""
    if is_heldout_path(path, layout):
        raise HeldoutAccessError(f"refusing to read held-out path {path}")


def check_heldout_unlock(layout: Layout, commit: str | None) -> str:
    """Two-key unlock for content access to held-out items.

    Requires BOTH a CLI-supplied design-freeze commit AND a committed
    <corpus_dir>/heldout-unlock.json naming the same commit, which must exist
    in git history and be an ancestor of HEAD.  Returns the commit on success."""
    if not commit:
        raise HeldoutAccessError("held-out content access requires --unlock-heldout <design-freeze-commit>")
    if not HEX40_RE.match(commit):
        raise HeldoutAccessError("--unlock-heldout needs a full 40-hex commit id")
    unlock = load_json_if_exists(layout.corpus_dir / "heldout-unlock.json")
    if not unlock or unlock.get("design_freeze_commit") != commit:
        raise HeldoutAccessError(
            f"{layout.corpus_dir / 'heldout-unlock.json'} missing or does not name design_freeze_commit {commit}")
    lock = load_json_if_exists(layout.corpus_dir / "heldout-lock.json")
    if not lock or lock.get("status") != "frozen":
        raise HeldoutAccessError("heldout-lock.json is not frozen; unlock refused")
    git = ["git", "-c", "safe.directory=*", "-C", str(REPO_ROOT)]
    r1 = subprocess.run(git + ["cat-file", "-e", f"{commit}^{{commit}}"], capture_output=True)
    r2 = subprocess.run(git + ["merge-base", "--is-ancestor", commit, "HEAD"], capture_output=True)
    if r1.returncode != 0 or r2.returncode != 0:
        raise HeldoutAccessError(f"design-freeze commit {commit} is not an ancestor of HEAD")
    return commit


# ---------------------------------------------------------------------------
# source definitions

ITEM_KEYS_REQUIRED = ("item_id", "family", "split", "scale", "kind", "real_or_generated", "recipe",
                      "license", "independence_group", "description")
ITEM_KEYS_OPTIONAL = ("notes", "tags")
RECIPE_KEYS = ("inputs", "git", "docker", "from_items", "generator", "steps", "output_pin", "notes")
LICENSE_KEYS = ("spdx_or_name", "redistributable", "attribution", "notes")


def _url_ok(url: str) -> bool:
    """https only.  file:// is accepted solely inside selftest.sh sandboxes."""
    if url.startswith("https://"):
        return True
    return url.startswith("file:///") and os.environ.get("EB_SELFTEST_ALLOW_FILE_URLS") == "1"


def _check_script(obj: dict, where: str, errors: list, *, need_seed: bool) -> None:
    script = obj.get("script")
    if not isinstance(script, str) or not safe_relpath(script, allow_empty=False) or not script.startswith("research/"):
        errors.append(f"{where}: script must be a repo-relative path under research/ (got {script!r})")
    elif not (REPO_ROOT / script).is_file():
        errors.append(f"{where}: script {script} does not exist")
    interp = obj.get("interpreter", "python")
    if interp not in INTERPRETERS:
        errors.append(f"{where}: interpreter must be one of {INTERPRETERS}")
    seed = obj.get("seed")
    if (need_seed or seed is not None) and (not isinstance(seed, int) or isinstance(seed, bool)):
        errors.append(f"{where}: seed must be an integer")
    if "params" in obj and not isinstance(obj["params"], dict):
        errors.append(f"{where}: params must be an object")
    allowed = {"op", "script", "interpreter", "seed", "params", "notes"}
    extra = set(obj) - allowed
    if extra:
        errors.append(f"{where}: unknown keys {sorted(extra)}")


def validate_item(item: dict, where: str, errors: list, warnings: list) -> None:
    if not isinstance(item, dict):
        errors.append(f"{where}: item must be an object")
        return
    for k in ITEM_KEYS_REQUIRED:
        if k not in item:
            errors.append(f"{where}: missing key {k}")
    extra = set(item) - set(ITEM_KEYS_REQUIRED) - set(ITEM_KEYS_OPTIONAL)
    if extra:
        errors.append(f"{where}: unknown keys {sorted(extra)}")
    iid = item.get("item_id")
    if not isinstance(iid, str) or not KEBAB_RE.match(iid) or len(iid) > 80:
        errors.append(f"{where}: item_id must be kebab-case [a-z0-9-], <=80 chars (got {iid!r})")
    if item.get("family") not in FAMILIES:
        errors.append(f"{where}: family must be one of F01..F20")
    if item.get("split") not in SPLITS:
        errors.append(f"{where}: split must be one of {SPLITS}")
    if item.get("scale") not in SCALES:
        errors.append(f"{where}: scale must be one of {tuple(SCALES)}")
    kind = item.get("kind")
    if kind not in KINDS:
        errors.append(f"{where}: kind must be one of {KINDS}")
    rog = item.get("real_or_generated")
    if rog not in REAL_OR_GENERATED:
        errors.append(f"{where}: real_or_generated must be one of {REAL_OR_GENERATED}")
    grp = item.get("independence_group")
    if not isinstance(grp, str) or not KEBAB_RE.match(grp):
        errors.append(f"{where}: independence_group must be kebab-case")
    if not isinstance(item.get("description"), str) or not item.get("description", "").strip():
        errors.append(f"{where}: description must be a non-empty string")
    lic = item.get("license")
    if not isinstance(lic, dict):
        errors.append(f"{where}: license must be an object")
    else:
        for k in LICENSE_KEYS:
            if k not in lic:
                errors.append(f"{where}: license.{k} missing")
        if set(lic) - set(LICENSE_KEYS):
            errors.append(f"{where}: license has unknown keys {sorted(set(lic) - set(LICENSE_KEYS))}")
        if not isinstance(lic.get("spdx_or_name"), str) or not lic.get("spdx_or_name", "").strip():
            errors.append(f"{where}: license.spdx_or_name must be a non-empty string")
        if not isinstance(lic.get("redistributable"), bool):
            errors.append(f"{where}: license.redistributable must be true/false")
        for k in ("attribution", "notes"):
            if k in lic and not isinstance(lic[k], str):
                errors.append(f"{where}: license.{k} must be a string")

    recipe = item.get("recipe")
    if not isinstance(recipe, dict):
        errors.append(f"{where}: recipe must be an object")
        return
    extra = set(recipe) - set(RECIPE_KEYS)
    if extra:
        errors.append(f"{where}: recipe has unknown keys {sorted(extra)}")
    input_names = set()
    for i, inp in enumerate(recipe.get("inputs", []) or []):
        w = f"{where}: recipe.inputs[{i}]"
        if not isinstance(inp, dict):
            errors.append(f"{w} must be an object")
            continue
        extra = set(inp) - {"name", "url", "sha256", "size", "filename", "notes"}
        if extra:
            errors.append(f"{w}: unknown keys {sorted(extra)}")
        name = inp.get("name")
        if not isinstance(name, str) or not INPUT_NAME_RE.match(name):
            errors.append(f"{w}: name must match {INPUT_NAME_RE.pattern}")
        elif name in input_names or name in ("git-archive", "docker-rootfs"):
            errors.append(f"{w}: duplicate/reserved input name {name}")
        else:
            input_names.add(name)
        url = inp.get("url")
        if not isinstance(url, str) or not _url_ok(url):
            errors.append(f"{w}: url must be https://")
        sha = inp.get("sha256")
        if not (sha == "TOFU" or (isinstance(sha, str) and HEX64_RE.match(sha))):
            errors.append(f"{w}: sha256 must be 64 lowercase hex or \"TOFU\"")
        if "size" in inp and (not isinstance(inp["size"], int) or isinstance(inp["size"], bool) or inp["size"] < 0):
            errors.append(f"{w}: size must be a non-negative integer")
        if "filename" in inp and (not isinstance(inp["filename"], str) or not SAFE_BASENAME_RE.match(inp["filename"])):
            errors.append(f"{w}: filename must be a safe basename")
    git = recipe.get("git")
    if git is not None:
        if not isinstance(git, dict):
            errors.append(f"{where}: recipe.git must be an object")
        else:
            extra = set(git) - {"repo", "commit", "ref", "subpaths", "history", "notes"}
            if extra:
                errors.append(f"{where}: recipe.git unknown keys {sorted(extra)}")
            if not isinstance(git.get("repo"), str) or not _url_ok(git["repo"]):
                errors.append(f"{where}: recipe.git.repo must be an https:// URL")
            if not isinstance(git.get("commit"), str) or not HEX40_RE.match(git["commit"]):
                errors.append(f"{where}: recipe.git.commit must be a full 40-hex commit id")
            for sp in git.get("subpaths", []) or []:
                if not isinstance(sp, str) or not safe_relpath(sp, allow_empty=False):
                    errors.append(f"{where}: recipe.git.subpaths entries must be safe relative paths")
            history = git.get("history", "archive")
            if history not in ("archive", "full"):
                errors.append(f"{where}: recipe.git.history must be \"archive\" or \"full\"")
            if history == "full":
                if git.get("subpaths"):
                    errors.append(f"{where}: recipe.git.subpaths is not supported with history=full "
                                  f"(a full clone carries the whole repository)")
                # a full clone's .git/index and working-tree stat metadata are not
                # reproducible across clones/machines, so the output must be unpinned.
                pin = recipe.get("output_pin", DEFAULT_OUTPUT_PIN.get(kind))
                if pin is not None:
                    errors.append(f"{where}: recipe.git.history=full requires recipe.output_pin: null "
                                  f"(.git/index stat data is not reproducible)")
    docker = recipe.get("docker")
    if docker is not None:
        if not isinstance(docker, dict):
            errors.append(f"{where}: recipe.docker must be an object")
        else:
            extra = set(docker) - {"image", "platform", "notes"}
            if extra:
                errors.append(f"{where}: recipe.docker unknown keys {sorted(extra)}")
            if not isinstance(docker.get("image"), str) or not DOCKER_IMAGE_RE.match(docker["image"]):
                errors.append(f"{where}: recipe.docker.image must be pinned by digest: name[:tag]@sha256:<64hex>")
            if docker.get("platform", "linux/amd64") not in ("linux/amd64", "linux/arm64", "linux/arm/v7", "linux/386"):
                errors.append(f"{where}: recipe.docker.platform unsupported")
    from_items = recipe.get("from_items", []) or []
    if not isinstance(from_items, list) or not all(isinstance(x, str) and KEBAB_RE.match(x) for x in from_items):
        errors.append(f"{where}: recipe.from_items must be a list of item ids")
        from_items = []
    gen = recipe.get("generator")
    if gen is not None:
        if not isinstance(gen, dict):
            errors.append(f"{where}: recipe.generator must be an object")
        else:
            _check_script(gen, f"{where}: recipe.generator", errors, need_seed=True)
    steps = recipe.get("steps", []) or []
    if not isinstance(steps, list):
        errors.append(f"{where}: recipe.steps must be a list")
        steps = []
    for i, st in enumerate(steps):
        w = f"{where}: recipe.steps[{i}]"
        if not isinstance(st, dict) or st.get("op") not in STEP_OPS:
            errors.append(f"{w}: op must be one of {STEP_OPS}")
            continue
        op = st["op"]
        allowed = {
            "extract": {"op", "input", "dest", "format", "strip_components", "notes"},
            "decompress": {"op", "input", "dest", "format", "notes"},
            "copy": {"op", "input", "dest", "mode", "notes"},
            "copy-item": {"op", "from_item", "src", "dest", "notes"},
            "remove": {"op", "paths", "notes"},
            "run": {"op", "script", "interpreter", "seed", "params", "notes"},
        }[op]
        if op == "run":
            _check_script(st, w, errors, need_seed=False)
            continue
        extra = set(st) - allowed
        if extra:
            errors.append(f"{w}: unknown keys {sorted(extra)}")
        if op in ("extract", "decompress", "copy"):
            inp = st.get("input")
            valid_inputs = set(input_names)
            if git is not None:
                valid_inputs.add("git-archive")
            if docker is not None:
                valid_inputs.add("docker-rootfs")
            if inp not in valid_inputs:
                errors.append(f"{w}: input {inp!r} is not a declared input")
        if "dest" in st and (not isinstance(st["dest"], str) or not safe_relpath(st["dest"])):
            errors.append(f"{w}: dest must be a safe relative path")
        if op == "extract":
            if st.get("format", "auto") not in ("auto", "tar", "zip", "7z"):
                errors.append(f"{w}: format must be auto|tar|zip|7z")
            sc = st.get("strip_components", 0)
            if not isinstance(sc, int) or isinstance(sc, bool) or sc < 0:
                errors.append(f"{w}: strip_components must be a non-negative integer")
        if op == "decompress":
            if st.get("format", "auto") not in ("auto", "gz", "xz", "zst", "bz2", "lz", "lz4", "br"):
                errors.append(f"{w}: format must be auto|gz|xz|zst|bz2|lz|lz4|br")
            if not st.get("dest"):
                errors.append(f"{w}: decompress needs dest")
        if op == "copy" and "mode" in st and (not isinstance(st["mode"], str) or not re.match(r"^0?[0-7]{3,4}$", st["mode"])):
            errors.append(f"{w}: mode must be an octal string like \"0644\"")
        if op == "copy-item":
            if st.get("from_item") not in from_items:
                errors.append(f"{w}: from_item must be listed in recipe.from_items")
            if "src" in st and (not isinstance(st["src"], str) or not safe_relpath(st["src"])):
                errors.append(f"{w}: src must be a safe relative path")
        if op == "remove":
            paths = st.get("paths")
            if not isinstance(paths, list) or not paths or not all(isinstance(p, str) and safe_relpath(p, allow_empty=False) for p in paths):
                errors.append(f"{w}: paths must be a non-empty list of safe relative paths")
    pin = recipe.get("output_pin", DEFAULT_OUTPUT_PIN.get(kind))
    if not (pin is None or pin == "TOFU" or (isinstance(pin, str) and HEX64_RE.match(pin))):
        errors.append(f"{where}: recipe.output_pin must be null, \"TOFU\" or 64 hex")

    # kind-specific structure
    has_inputs = bool(recipe.get("inputs"))
    if kind == "download":
        if not has_inputs:
            errors.append(f"{where}: kind download requires recipe.inputs")
        if git is not None or docker is not None:
            errors.append(f"{where}: kind download must not use git/docker")
    elif kind == "generate":
        if gen is None:
            errors.append(f"{where}: kind generate requires recipe.generator")
        if has_inputs or git is not None or docker is not None or from_items:
            errors.append(f"{where}: kind generate must not have inputs/git/docker/from_items (use build or derive)")
        if rog != "generated":
            errors.append(f"{where}: kind generate requires real_or_generated=generated")
    elif kind == "derive":
        if not from_items:
            errors.append(f"{where}: kind derive requires recipe.from_items")
        if gen is None and not steps:
            errors.append(f"{where}: kind derive requires recipe.generator or recipe.steps")
        if rog == "real":
            errors.append(f"{where}: kind derive cannot be real_or_generated=real")
    elif kind == "docker-export":
        if docker is None:
            errors.append(f"{where}: kind docker-export requires recipe.docker")
    elif kind == "git-archive":
        if git is None:
            errors.append(f"{where}: kind git-archive requires recipe.git")
    elif kind == "build":
        if gen is None:
            errors.append(f"{where}: kind build requires recipe.generator (the build script)")
    if docker is not None and kind != "docker-export":
        errors.append(f"{where}: recipe.docker only valid for kind docker-export")
    if git is not None and kind not in ("git-archive", "build"):
        errors.append(f"{where}: recipe.git only valid for kind git-archive or build")
    if rog == "generated" and kind in ("download", "git-archive", "docker-export"):
        warnings.append(f"{where}: real_or_generated=generated for kind {kind} is unusual")


def _norm_git_repo(url: str) -> str:
    u = url.lower().rstrip("/")
    return u[:-4] if u.endswith(".git") else u


def upstream_keys(item: dict) -> set:
    r = item.get("recipe") or {}
    keys = set()
    for inp in r.get("inputs", []) or []:
        if isinstance(inp, dict):
            if isinstance(inp.get("url"), str):
                keys.add("url:" + inp["url"])
            if isinstance(inp.get("sha256"), str) and HEX64_RE.match(inp["sha256"]):
                keys.add("sha256:" + inp["sha256"])
    git = r.get("git")
    if isinstance(git, dict) and isinstance(git.get("repo"), str):
        keys.add("git:" + _norm_git_repo(git["repo"]))
    docker = r.get("docker")
    if isinstance(docker, dict) and isinstance(docker.get("image"), str):
        name = docker["image"].split("@", 1)[0]
        name = name.rsplit(":", 1)[0] if ":" in name.split("/")[-1] else name
        keys.add("docker:" + name)
    return keys


def load_sources(layout: Layout):
    """Load and validate every sources/*.json.  Returns (items, errors, warnings);
    items maps item_id -> item dict with an added "_source_file" key."""
    errors, warnings = [], []
    items: dict[str, dict] = {}
    files = sorted(layout.sources_dir.glob("*.json")) if layout.sources_dir.is_dir() else []
    for path in files:
        rel = repo_rel(path)
        try:
            doc = load_json(path)
        except (OSError, ValueError) as e:
            errors.append(f"{rel}: cannot parse JSON: {e}")
            continue
        if not isinstance(doc, dict) or doc.get("schema") != SOURCES_SCHEMA or not isinstance(doc.get("items"), list):
            errors.append(f"{rel}: top level must be {{\"schema\": \"{SOURCES_SCHEMA}\", \"items\": [...]}}")
            continue
        extra = set(doc) - {"schema", "items", "notes"}
        if extra:
            errors.append(f"{rel}: unknown top-level keys {sorted(extra)}")
        for i, item in enumerate(doc["items"]):
            where = f"{rel}: items[{i}]" + (f" ({item.get('item_id')})" if isinstance(item, dict) else "")
            validate_item(item, where, errors, warnings)
            if not isinstance(item, dict) or not isinstance(item.get("item_id"), str):
                continue
            iid = item["item_id"]
            if iid in items:
                errors.append(f"{where}: duplicate item_id {iid} (also in {items[iid]['_source_file']})")
                continue
            it = dict(item)
            it["_source_file"] = rel
            items[iid] = it

    # cross-item checks -----------------------------------------------------
    for iid, it in items.items():
        r = it.get("recipe") or {}
        for fid in r.get("from_items", []) or []:
            if fid not in items:
                errors.append(f"{iid}: from_items references unknown item {fid}")
            elif items[fid].get("split") != it.get("split"):
                errors.append(f"{iid}: derives from {fid} in split {items[fid].get('split')} but is in split "
                              f"{it.get('split')} (cross-split derivation leaks upstream content)")
            elif items[fid].get("independence_group") != it.get("independence_group"):
                warnings.append(f"{iid}: independence_group {it.get('independence_group')} differs from source item "
                                f"{fid} ({items[fid].get('independence_group')})")
    # cycles
    state = {}

    def visit(n, stack):
        if state.get(n) == 1:
            errors.append(f"dependency cycle: {' -> '.join(stack + [n])}")
            return
        if state.get(n) == 2 or n not in items:
            return
        state[n] = 1
        for d in (items[n].get("recipe") or {}).get("from_items", []) or []:
            visit(d, stack + [n])
        state[n] = 2

    for n in items:
        visit(n, [])

    # group/split discipline
    fam_group_splits: dict[tuple, set] = {}
    group_splits: dict[str, dict] = {}
    for iid, it in items.items():
        g, f, s = it.get("independence_group"), it.get("family"), it.get("split")
        fam_group_splits.setdefault((f, g), set()).add(s)
        group_splits.setdefault(g, {}).setdefault(s, set()).add(f"{iid}({f})")
    for (f, g), splits in sorted(fam_group_splits.items(), key=lambda kv: (str(kv[0][0]), str(kv[0][1]))):
        if len(splits) > 1:
            errors.append(f"family {f}: independence_group {g} spans splits {sorted(splits)}")
    for g, by_split in sorted(group_splits.items(), key=lambda kv: str(kv[0])):
        if len(by_split) > 1:
            msg = f"independence_group {g} spans splits across families: " + "; ".join(
                f"{s}: {sorted(v)}" for s, v in sorted(by_split.items()))
            if "heldout" in by_split:
                errors.append(msg)
            else:
                warnings.append(msg)
    key_splits: dict[str, dict] = {}
    for iid, it in items.items():
        for k in upstream_keys(it):
            key_splits.setdefault(k, {}).setdefault(it.get("split"), set()).add(iid)
    for k, by_split in sorted(key_splits.items()):
        if len(by_split) > 1:
            errors.append(f"upstream {k} is used by items in different splits: " + "; ".join(
                f"{s}: {sorted(v)}" for s, v in sorted(by_split.items())))
    return items, errors, warnings


def public_item(item: dict) -> dict:
    return {k: v for k, v in item.items() if not k.startswith("_")}


def item_def_sha256(item: dict) -> str:
    return sha256_bytes(canonical_json(public_item(item)))


def effective_output_pin(item: dict):
    r = item.get("recipe") or {}
    return r["output_pin"] if "output_pin" in r else DEFAULT_OUTPUT_PIN.get(item.get("kind"))


def recipe_scripts(item: dict) -> list[str]:
    r = item.get("recipe") or {}
    out = []
    if isinstance(r.get("generator"), dict):
        out.append(r["generator"]["script"])
    for st in r.get("steps", []) or []:
        if isinstance(st, dict) and st.get("op") == "run":
            out.append(st["script"])
    return out


def materialization_key(item: dict, dep_logical: dict) -> dict:
    """Everything that determines materialized content.  Changes force a rebuild.
    Returns {"sha256": ..., "basis": {...}}."""
    scripts = {s: sha256_file(REPO_ROOT / s)[0] for s in sorted(set(recipe_scripts(item)))}
    basis = {
        "kind": item["kind"],
        "recipe": {k: v for k, v in item["recipe"].items() if k not in ("notes", "output_pin")},
        "script_sha256": scripts,
        "from_items_logical_tree_sha256": {k: dep_logical[k] for k in sorted(dep_logical)},
        "provision_semantics": "ebrc-provision-v1",
    }
    return {"sha256": sha256_bytes(canonical_json(basis)), "basis": basis}


def dependency_order(items: dict, selected: set) -> list[list[str]]:
    """Topological levels of selected items (dependencies must already be in selected)."""
    level: dict[str, int] = {}

    def lv(n):
        if n in level:
            return level[n]
        deps = [d for d in (items[n].get("recipe") or {}).get("from_items", []) or [] if d in selected]
        level[n] = 0 if not deps else 1 + max(lv(d) for d in deps)
        return level[n]

    for n in selected:
        lv(n)
    out: list[list[str]] = []
    for n in sorted(selected):
        while len(out) <= level[n]:
            out.append([])
        out[level[n]].append(n)
    return out
