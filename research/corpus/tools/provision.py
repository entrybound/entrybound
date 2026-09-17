#!/usr/bin/env python3
"""Materialize Entrybound research corpus items from research/corpus/sources/*.json.

For each selected item (plus its from_items dependencies):
  1. acquire inputs into the content-addressed cache (downloads verified against
     declared SHA-256, or TOFU: first retrieval is pinned in research/corpus/pins/<id>.json),
     git archives (commit-pinned) and docker exports (digest-pinned);
  2. run recipe steps and the generator/build script in a staging directory with a
     sanitized environment (umask 022, LC_ALL=C.UTF-8, TZ=UTC, fixed SOURCE_DATE_EPOCH);
  3. fingerprint the staging tree (fingerprint.py), enforce the scale tier and the
     output pin (declared hex or TOFU), then atomically move it to
     /root/eb-research/corpus/<split>/<id> or /root/eb-research/heldout/<id>;
  4. write research/corpus/fingerprints/<id>.json (held-out: hashes/bytes/counts only).

Rerunnable: an item whose materialization key (recipe + script hashes + dependency
hashes) is unchanged and whose on-disk tree still matches its recorded fingerprint is
skipped.  Any pinned/recorded hash mismatch fails loudly (exit 1).

Usage (inside WSL, via research/corpus/tools/run.sh):
  run.sh provision --check                 validate definitions only
  run.sh provision --family F04 --jobs 4   materialize one family
  run.sh provision --item my-item --rebuild
"""

from __future__ import annotations

import argparse
import contextlib
import hashlib
import json
import os
import re
import secrets
import shutil
import subprocess
import sys
import threading
import time
import traceback
import urllib.parse
import urllib.request
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
sys.dont_write_bytecode = True
import corpuslib as cl  # noqa: E402
import fingerprint as fpm  # noqa: E402

SAFE_PATH = "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"
SOURCE_DATE_EPOCH = "1767225600"  # 2026-01-01T00:00:00Z, fixed for reproducible generators/builds
USER_AGENT = "entrybound-research-corpus-provision/1"
# Wikimedia's User-Agent policy (meta.wikimedia.org/wiki/User-Agent_policy) asks bots to identify
# themselves with a project URL and purpose; generic/undescriptive UAs are throttled more
# aggressively. Used only for commons.wikimedia.org / upload.wikimedia.org requests.
WIKIMEDIA_USER_AGENT = ("EntryboundResearchCorpus/1.0 "
                        "(https://github.com/entrybound/entrybound; research corpus provisioning; "
                        "polite, rate-limited, contact via repository issues) Python-urllib/" + sys.version.split()[0])
_print_lock = threading.Lock()
_max_retry_after = 900  # seconds; cap so a single throttle wait cannot stall a run indefinitely


def log(msg: str) -> None:
    with _print_lock:
        print(msg, flush=True)


# ---------------------------------------------------------------------------
# environment / versions

_env_cache: dict = {}


def environment_versions(used: set) -> dict:
    out = {"python": sys.version.split()[0], "python_executable": sys.executable, "kernel": os.uname().release}
    probes = {
        "tar": ["tar", "--version"], "bsdtar": ["bsdtar", "--version"], "git": ["git", "--version"],
        "cp": ["cp", "--version"], "gzip": ["gzip", "--version"], "xz": ["xz", "--version"],
        "zstd": ["zstd", "--version"], "bzip2": ["bzip2", "--help"], "lzip": ["lzip", "--version"],
        "lz4": ["lz4", "--version"], "brotli": ["brotli", "--version"], "7z": ["7z"],
    }
    for name in sorted(used):
        if name == "docker":
            continue
        if name in probes:
            if name not in _env_cache:
                _env_cache[name] = cl.command_version(probes[name])
            out[name] = _env_cache[name]
    return out


# ---------------------------------------------------------------------------
# context


class Ctx:
    def __init__(self, item, layout, args, deps):
        self.item = item
        self.iid = item["item_id"]
        self.layout = layout
        self.args = args
        self.deps = deps  # item_id -> (record, path)
        token = f"{os.getpid()}.{secrets.token_hex(4)}"
        self.staging = layout.staging_dir / f"{self.iid}.{token}"
        self.scratch = layout.scratch_dir / f"{self.iid}.{token}"
        self.inputs: dict[str, Path] = {}
        self.input_filenames: dict[str, str] = {}
        self.provenance: dict = {}
        self.used_tools: set = set()
        self.pins = cl.load_json_if_exists(layout.pins_path(self.iid)) or {
            "format": cl.PINS_FORMAT, "item_id": self.iid, "inputs": {}, "output": None, "history": []}
        self.pins_dirty = False
        layout.log_dir.mkdir(parents=True, exist_ok=True)
        self.log_path = layout.log_dir / f"{self.iid}.log"
        self.logf = open(self.log_path, "ab")
        self.logf.write(f"\n==== {cl.utc_now()} provision {self.iid} pid {os.getpid()} ====\n".encode())
        self.logf.flush()

    def close(self):
        self.logf.close()

    def env(self, extra: dict | None = None) -> dict:
        venv_bin = str(Path(sys.executable).parent)
        path = venv_bin + ":" + ("/root/.cargo/bin:" if Path("/root/.cargo/bin").is_dir() else "") + SAFE_PATH
        home = self.scratch / "home"
        home.mkdir(parents=True, exist_ok=True)
        env = {
            "PATH": path, "HOME": str(home), "LC_ALL": "C.UTF-8", "LANG": "C.UTF-8", "TZ": "UTC",
            "SOURCE_DATE_EPOCH": SOURCE_DATE_EPOCH, "PYTHONHASHSEED": "0", "PYTHONDONTWRITEBYTECODE": "1",
            "GIT_TERMINAL_PROMPT": "0", "GIT_CONFIG_NOSYSTEM": "1", "TMPDIR": str(self.scratch),
        }
        for k in ("RUSTUP_HOME", "CARGO_HOME"):
            default = "/root/.rustup" if k == "RUSTUP_HOME" else "/root/.cargo"
            if os.environ.get(k) or Path(default).is_dir():
                env[k] = os.environ.get(k) or default
        for k in ("http_proxy", "https_proxy", "HTTP_PROXY", "HTTPS_PROXY", "no_proxy", "NO_PROXY"):
            if k in os.environ:
                env[k] = os.environ[k]
        if extra:
            env.update(extra)
        return env

    def run(self, cmd, *, cwd=None, stdout=None, extra_env=None, what=None):
        cmd = [str(c) if isinstance(c, Path) else c for c in cmd]
        self.logf.write(("$ " + " ".join(repr(c) if isinstance(c, bytes) else c for c in cmd) + "\n").encode())
        self.logf.flush()
        r = subprocess.run(cmd, cwd=str(cwd or self.scratch), env=self.env(extra_env),
                           stdout=stdout if stdout is not None else self.logf, stderr=self.logf,
                           stdin=subprocess.DEVNULL)
        self.logf.flush()
        if r.returncode != 0:
            raise cl.CorpusError(f"{self.iid}: {what or cmd[0]} failed (exit {r.returncode}); "
                                 f"see {self.log_path}\n{tail(self.log_path)}")
        return r

    def capture(self, cmd, what=None) -> str:
        with open(self.scratch / "capture.out", "w+b") as f:
            self.run(cmd, stdout=f, what=what)
            f.seek(0)
            return f.read().decode("utf-8", "replace").strip()


def tail(path, n=30) -> str:
    try:
        with open(path, "rb") as f:
            f.seek(0, 2)
            size = f.tell()
            f.seek(max(0, size - 16384))
            lines = f.read().decode("utf-8", "replace").splitlines()
        return "\n".join("    " + ln for ln in lines[-n:])
    except OSError:
        return ""


# ---------------------------------------------------------------------------
# acquisition


def _verified_blob(blob: Path, expected_size: int | None, reverify: bool, filename: str | None = None) -> bool:
    if not blob.is_file():
        return False
    marker = blob.with_name(blob.name + ".verified.json")
    st = blob.stat()
    m = cl.load_json_if_exists(marker)
    if not reverify and m and m.get("size") == st.st_size and m.get("mtime_ns") == st.st_mtime_ns:
        ok = True
    else:
        digest, _ = cl.sha256_file(blob)
        if digest != blob.name:
            raise cl.HashMismatch(f"HASH MISMATCH: cached blob {blob} has sha256 {digest}")
        cl.write_json_atomic(marker, {"size": st.st_size, "mtime_ns": st.st_mtime_ns, "verified_utc": cl.utc_now()})
        ok = True
    if ok and expected_size is not None and st.st_size != expected_size:
        raise cl.HashMismatch(f"SIZE MISMATCH: cached blob {blob} is {st.st_size} bytes, expected {expected_size}")
    # Cheap and unconditional (runs even on the .verified.json marker fast path above):
    # the blob's own sha256 filename can never catch a blob that was corrupted before
    # that hash was taken, e.g. an all-zero download that a flaky upstream/proxy served
    # once with a plausible size (the historical Wikimedia pageviews blob). A magic-byte
    # check is independent of the blob's self-derived cache key.
    if ok and filename:
        cl.check_magic_bytes(blob, filename)
    return ok


def _retry_after_seconds(headers, default: float) -> float:
    """Parse a Retry-After header (delay-seconds or HTTP-date form); fall back to `default`."""
    raw = ""
    with contextlib.suppress(Exception):
        raw = (headers or {}).get("Retry-After", "") if hasattr(headers, "get") else ""
    raw = str(raw).strip()
    if not raw:
        return default
    if raw.isdigit():
        return min(float(raw), _max_retry_after)
    with contextlib.suppress(Exception):
        import email.utils
        dt = email.utils.parsedate_to_datetime(raw)
        if dt is not None:
            now = _dt_now_utc()
            secs = (dt - now).total_seconds()
            if secs > 0:
                return min(secs, _max_retry_after)
    return default


def _dt_now_utc():
    import datetime
    return datetime.datetime.now(datetime.timezone.utc)


def fetch_url(ctx: Ctx, url: str, expected_sha: str | None, expected_size: int | None,
              headers: dict | None = None) -> dict:
    cache = ctx.layout.cache_dir
    blobs = cache / "sha256"
    index = cache / "by-url"
    tmpdir = cache / "tmp"
    for d in (blobs, index, tmpdir):
        d.mkdir(parents=True, exist_ok=True)
    url_key = cl.sha256_bytes(url.encode("utf-8"))
    sniff_name = os.path.basename(urllib.parse.unquote(urllib.parse.urlparse(url).path))
    with cl.file_lock(ctx.layout.lock_path("download-" + url_key[:32])):
        idx = cl.load_json_if_exists(index / f"{url_key}.json")
        if expected_sha:
            blob = blobs / expected_sha
            if _verified_blob(blob, expected_size, ctx.args.reverify_cache, sniff_name):
                info = dict(idx) if idx and idx.get("sha256") == expected_sha else {
                    "url": url, "final_url": None, "sha256": expected_sha, "size": blob.stat().st_size,
                    "retrieved_utc": None}
                info["cached"] = True
                info["path"] = str(blob)
                return info
        elif idx:
            blob = blobs / idx["sha256"]
            if _verified_blob(blob, expected_size, ctx.args.reverify_cache, sniff_name):
                info = dict(idx)
                info["cached"] = True
                info["path"] = str(blob)
                return info
        if ctx.args.offline:
            raise cl.CorpusError(f"{ctx.iid}: {url} not in cache and --offline given")
        req_headers = {"User-Agent": USER_AGENT}
        if headers:
            req_headers.update(headers)
        last_err = None
        attempt = 0
        throttle_retries = 0
        max_throttle_retries = 20  # 429/503 waits are expected under polite crawling, not failures
        max_attempts = 4  # for genuine network/HTTP errors
        while True:
            attempt += 1
            tmp = tmpdir / f"{url_key}.{os.getpid()}.{secrets.token_hex(4)}.part"
            try:
                log(f"[{ctx.iid}] download {url} (attempt {attempt})")
                req = urllib.request.Request(url, headers=req_headers)
                h = hashlib.sha256()
                size = 0
                with urllib.request.urlopen(req, timeout=120) as resp, open(tmp, "wb") as out:
                    final_url = resp.geturl()
                    while True:
                        b = resp.read(cl.MiB)
                        if not b:
                            break
                        h.update(b)
                        out.write(b)
                        size += len(b)
                digest = h.hexdigest()
                break
            except urllib.error.HTTPError as e:
                with contextlib.suppress(OSError):
                    tmp.unlink()
                if e.code in (429, 503) and throttle_retries < max_throttle_retries:
                    throttle_retries += 1
                    wait = _retry_after_seconds(e.headers, default=min(30 * throttle_retries, _max_retry_after))
                    log(f"[{ctx.iid}] HTTP {e.code} for {url}; honoring Retry-After, sleeping {wait:.0f}s "
                        f"(throttle retry {throttle_retries}/{max_throttle_retries})")
                    time.sleep(wait)
                    continue
                last_err = e
                if attempt >= max_attempts:
                    raise cl.CorpusError(f"{ctx.iid}: download failed for {url}: {last_err}")
                time.sleep(2 * attempt)
            except Exception as e:  # network errors: retry
                last_err = e
                with contextlib.suppress(OSError):
                    tmp.unlink()
                if attempt >= max_attempts:
                    raise cl.CorpusError(f"{ctx.iid}: download failed for {url}: {last_err}")
                time.sleep(2 * attempt)
        if expected_size is not None and size != expected_size:
            tmp.unlink()
            raise cl.HashMismatch(f"SIZE MISMATCH for {url}: got {size} bytes, expected {expected_size}")
        if expected_sha and digest != expected_sha:
            tmp.unlink()
            raise cl.HashMismatch(f"HASH MISMATCH for {url}: got sha256 {digest}, pinned {expected_sha}")
        try:
            cl.check_magic_bytes(tmp, sniff_name)
        except cl.HashMismatch:
            tmp.unlink()
            raise
        blob = blobs / digest
        os.replace(tmp, blob)
        st = blob.stat()
        cl.write_json_atomic(blob.with_name(blob.name + ".verified.json"),
                             {"size": st.st_size, "mtime_ns": st.st_mtime_ns, "verified_utc": cl.utc_now()})
        info = {"url": url, "final_url": final_url, "sha256": digest, "size": size, "retrieved_utc": cl.utc_now(),
                "retrieval_date": time.strftime("%Y-%m-%d")}
        cl.write_json_atomic(index / f"{url_key}.json", info)
        info = dict(info, cached=False, path=str(blob))
        return info


# ---------------------------------------------------------------------------
# upstream-published digest cross-check (see task_bb8ca4e8 integrity follow-up,
# PROGRESS.md "the corpus cache's self-consistency check cannot detect a corrupt
# cached blob that matches its own SHA-256-named path")
#
# The download cache's own bookkeeping (blob named after its content's SHA-256, plus
# TOFU pinning on first retrieval) can never catch a blob that was corrupted before
# that hash was ever taken: whatever bytes arrived, the cache faithfully remembers
# them as "correct" forever after. Where the source host independently publishes a
# digest for the file -- computed by them, at their end, out of band from our
# download -- cross-checking our cached bytes against it closes that gap, for
# TOFU and declared inputs alike, on cache hits as well as fresh downloads.


def _fetch_manifest_text(ctx: "Ctx", url: str, max_bytes: int = 8 * cl.MiB) -> str:
    """Fetch a small text manifest (e.g. a checksums file or a dumpstatus.json) with
    the same polite User-Agent and basic retry as fetch_url, but outside the
    content-addressed blob cache: the manifest itself is not corpus content, just a
    control-plane input to the check below, and dump-status pages can be regenerated
    upstream so are not worth pinning."""
    if ctx.args.offline:
        raise cl.CorpusError(f"{ctx.iid}: {url} (upstream digest manifest) not fetchable with --offline")
    last_err = None
    for attempt in range(1, 4):
        try:
            req = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
            with urllib.request.urlopen(req, timeout=60) as resp:
                data = resp.read(max_bytes + 1)
            if len(data) > max_bytes:
                raise cl.CorpusError(f"{ctx.iid}: upstream digest manifest {url} exceeds {max_bytes} bytes")
            return data.decode("utf-8", "replace")
        except cl.CorpusError:
            raise
        except Exception as e:
            last_err = e
            time.sleep(2 * attempt)
    raise cl.CorpusError(f"{ctx.iid}: could not fetch upstream digest manifest {url}: {last_err}")


def _parse_wikimedia_dumpstatus(text: str, filename: str) -> dict | None:
    """dumpstatus.json: {"jobs": {"<job>": {"files": {"<filename>": {"sha1":, "md5":, "size":, ...}}}}}."""
    data = json.loads(text)
    for job in (data.get("jobs") or {}).values():
        entry = (job.get("files") or {}).get(filename)
        if entry:
            return entry
    return None


def _parse_checksum_file(text: str, filename: str) -> str | None:
    """GNU coreutils `sha256sum`/`sha1sum`/`md5sum` output ("<hex>  <filename>" or
    "<hex> *<filename>" per line) or BSD-style ("SHA256 (<filename>) = <hex>")."""
    for line in text.splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        m = re.match(r"^([0-9a-fA-F]{32,64})\s+[* ]?(.+)$", line)
        if m and os.path.basename(m.group(2).strip()) == filename:
            return m.group(1).lower()
        m = re.match(r"^\w+\s*\(([^)]+)\)\s*=\s*([0-9a-fA-F]{32,64})$", line)
        if m and os.path.basename(m.group(1).strip()) == filename:
            return m.group(2).lower()
    return None


def verify_upstream_digest(ctx: "Ctx", spec: dict, blob: Path, filename: str) -> dict:
    """Cross-check a cached/downloaded blob against an independently published
    digest. Raises cl.HashMismatch on a mismatch; returns a small provenance record
    on success (or on "not found in manifest", which is reported, not raised, since a
    manifest can legitimately omit a file the recipe still needs)."""
    manifest_url = spec["manifest_url"]
    algo = spec["algo"]
    want_name = spec.get("file") or filename
    text = _fetch_manifest_text(ctx, manifest_url)
    if spec["format"] == "wikimedia-dumpstatus":
        entry = _parse_wikimedia_dumpstatus(text, want_name)
        published = entry.get(algo) if entry else None
        published_size = entry.get("size") if entry else None
    elif spec["format"] == "checksum-file":
        published = _parse_checksum_file(text, want_name)
        published_size = None
    else:
        raise cl.CorpusError(f"{ctx.iid}: unknown upstream_digest.format {spec['format']!r}")
    if published is None:
        log(f"[{ctx.iid}] WARNING: upstream_digest manifest {manifest_url} has no entry for {want_name}; "
            f"cannot cross-check (not treated as a failure, since a dump's file list can shrink)")
        return {"manifest_url": manifest_url, "algo": algo, "checked": False}
    if published_size is not None and published_size != blob.stat().st_size:
        raise cl.HashMismatch(f"UPSTREAM SIZE MISMATCH: {ctx.iid} input {want_name}: cached blob is "
                              f"{blob.stat().st_size} bytes, {manifest_url} publishes {published_size}")
    got = cl.hash_file_algo(blob, algo)
    if got != published.lower():
        raise cl.HashMismatch(f"UPSTREAM DIGEST MISMATCH: {ctx.iid} input {want_name}: cached blob "
                              f"{algo} is {got}, {manifest_url} publishes {published}; the cached copy "
                              f"is corrupt even though it matches its own SHA-256-named cache path")
    return {"manifest_url": manifest_url, "algo": algo, "value": got, "checked": True}


def acquire_inputs(ctx: Ctx) -> None:
    prov = []
    for inp in ctx.item["recipe"].get("inputs", []) or []:
        name, url, declared = inp["name"], inp["url"], inp["sha256"]
        pinned = None
        if declared == "TOFU":
            p = ctx.pins["inputs"].get(name)
            if p and p.get("url") == url:
                pinned = p["sha256"]
        expected = declared if declared != "TOFU" else pinned
        size = inp.get("size")
        if size is None and declared == "TOFU" and pinned:
            size = ctx.pins["inputs"][name].get("size")
        info = fetch_url(ctx, url, expected, size)
        if declared == "TOFU" and not pinned:
            old = ctx.pins["inputs"].get(name)
            if old:
                ctx.pins["history"].append({"replaced_utc": cl.utc_now(), "input": name, "previous": old})
            ctx.pins["inputs"][name] = {"url": url, "sha256": info["sha256"], "size": info["size"],
                                        "retrieved_utc": info.get("retrieved_utc") or cl.utc_now(),
                                        "retrieval_date": info.get("retrieval_date") or time.strftime("%Y-%m-%d")}
            ctx.pins_dirty = True
            pin_status = "tofu-first-retrieval"
            log(f"[{ctx.iid}] TOFU pinned input {name}: sha256 {info['sha256']} ({info['size']} bytes)")
        else:
            pin_status = "declared" if declared != "TOFU" else "tofu-pinned"
        filename = inp.get("filename") or os.path.basename(urllib.parse.unquote(urllib.parse.urlparse(url).path)) or name
        if not cl.SAFE_BASENAME_RE.match(filename):
            filename = name
        ctx.inputs[name] = Path(info["path"])
        ctx.input_filenames[name] = filename
        prov_entry = {"name": name, "url": url, "final_url": info.get("final_url"), "sha256": info["sha256"],
                     "size": info["size"], "retrieved_utc": info.get("retrieved_utc"),
                     "retrieval_date": info.get("retrieval_date"), "pin": pin_status,
                     "filename": filename}
        upstream = inp.get("upstream_digest")
        if upstream:
            # Runs on every acquisition, cache hit or fresh download alike: this is
            # independent of (and does not shortcut through) the self-consistency
            # ".verified.json" marker, so a blob that was corrupted before its own
            # sha256 was ever taken still gets caught here.
            prov_entry["upstream_digest"] = verify_upstream_digest(ctx, upstream, ctx.inputs[name], filename)
        prov.append(prov_entry)
    if prov:
        ctx.provenance["inputs"] = prov


def acquire_git_full(ctx: Ctx) -> None:
    """git.history == "full": cache a full-history bare mirror once, bundle it
    (`git bundle create --all`, SHA-256 recorded for provenance/caching), then
    materialize the item by cloning from that cached bundle and checking out the
    pinned commit -- so the item's tree is a real working copy with a complete
    `.git` (refs, packed objects, reflog), not just a commit snapshot. Output is
    always unpinned (recipe.output_pin: null is enforced by validate_item): the
    .git/index and any reflog timestamps are not byte-reproducible across clones."""
    g = ctx.item["recipe"]["git"]
    ctx.used_tools.add("git")
    key = cl.sha256_bytes(cl._norm_git_repo(g["repo"]).encode())[:24]
    mirror = ctx.layout.cache_dir / "git-full" / f"{key}.git"
    mirror.parent.mkdir(parents=True, exist_ok=True)
    bundles_dir = ctx.layout.cache_dir / "git-full-bundles"
    bundles_dir.mkdir(parents=True, exist_ok=True)
    with cl.file_lock(ctx.layout.lock_path("git-full-" + key)):
        if not (mirror / "HEAD").exists():
            if ctx.args.offline:
                raise cl.CorpusError(f"{ctx.iid}: full mirror of {g['repo']} not cached and --offline given")
            log(f"[{ctx.iid}] git clone --mirror {g['repo']} (full history)")
            ctx.run(["git", "clone", "--mirror", "--", g["repo"], mirror], what="git clone --mirror")
        have = subprocess.run(["git", "-C", mirror, "cat-file", "-e", f"{g['commit']}^{{commit}}"],
                              capture_output=True, env=ctx.env()).returncode == 0
        if not have:
            if ctx.args.offline:
                raise cl.CorpusError(f"{ctx.iid}: commit {g['commit']} not cached and --offline given")
            log(f"[{ctx.iid}] git fetch {g['repo']} into full mirror (updating cached history)")
            ctx.run(["git", "-C", mirror, "fetch", "--tags", "--prune", "origin", "+refs/*:refs/*"],
                    what="git fetch --mirror-update")
            if subprocess.run(["git", "-C", mirror, "cat-file", "-e", f"{g['commit']}^{{commit}}"],
                              capture_output=True, env=ctx.env()).returncode != 0:
                raise cl.HashMismatch(f"{ctx.iid}: commit {g['commit']} not present in full mirror of {g['repo']}")
        bundle_tmp = ctx.scratch / "repo.bundle.tmp"
        ctx.run(["git", "-C", mirror, "bundle", "create", bundle_tmp, "--all"], what="git bundle create")
        bundle_sha, bundle_size = cl.sha256_file(bundle_tmp)
        bundle = bundles_dir / f"{key}.{bundle_sha}.bundle"
        if not bundle.exists():
            os.replace(bundle_tmp, bundle)
        else:
            bundle_tmp.unlink()
    ctx.run(["git", "clone", "--", bundle, ctx.staging], what="git clone (from cached bundle)")
    ctx.run(["git", "-C", ctx.staging, "checkout", "--quiet", g["commit"]], what="git checkout")
    ctx.run(["git", "-C", ctx.staging, "remote", "remove", "origin"], what="git remote remove origin")
    # the clone/checkout above leave the local cache's bundle path in .git/logs/* (reflog messages)
    # and possibly stale remote-tracking cruft; expire+gc so the materialized .git carries no trace
    # of this machine's cache layout (the item is meant to look like an ordinary repository clone).
    ctx.run(["git", "-C", ctx.staging, "reflog", "expire", "--expire=now", "--expire-unreachable=now", "--all"],
            what="git reflog expire")
    ctx.run(["git", "-C", ctx.staging, "gc", "--prune=now", "--quiet"], what="git gc")
    ctx.provenance["git"] = {
        "repo": g["repo"], "commit": g["commit"], "ref": g.get("ref"), "history": "full",
        "mirror_bundle_sha256": bundle_sha, "mirror_bundle_bytes": bundle_size,
        "git_version": cl.command_version(["git", "--version"]),
        "notes": "materialized tree is a full working clone (with .git: refs, packed objects, reflog) "
                 "checked out at the pinned commit; cloned from a cached `git bundle create --all` of a "
                 "full-history bare mirror of the repository, not from a shallow archive",
    }


def acquire_git(ctx: Ctx) -> None:
    g = ctx.item["recipe"]["git"]
    if g.get("history") == "full":
        acquire_git_full(ctx)
        return
    ctx.used_tools.add("git")
    key = cl.sha256_bytes(cl._norm_git_repo(g["repo"]).encode())[:24]
    gitdir = ctx.layout.cache_dir / "git" / f"{key}.git"
    gitdir.parent.mkdir(parents=True, exist_ok=True)
    tar_path = ctx.scratch / "git-archive.tar"
    with cl.file_lock(ctx.layout.lock_path("git-" + key)):
        if not (gitdir / "HEAD").exists():
            ctx.run(["git", "init", "--bare", "-q", gitdir], what="git init")
        have = subprocess.run(["git", "-C", gitdir, "cat-file", "-e", f"{g['commit']}^{{commit}}"],
                              capture_output=True, env=ctx.env()).returncode == 0
        if not have:
            if ctx.args.offline:
                raise cl.CorpusError(f"{ctx.iid}: commit {g['commit']} not cached and --offline given")
            log(f"[{ctx.iid}] git fetch {g['repo']} {g['commit']}")
            r = subprocess.run(["git", "-C", gitdir, "fetch", "--depth", "1", "--no-tags", g["repo"], g["commit"]],
                               stdout=ctx.logf, stderr=ctx.logf, env=ctx.env())
            if r.returncode != 0 and g.get("ref"):
                ctx.run(["git", "-C", gitdir, "fetch", "--depth", "1", "--no-tags", g["repo"], g["ref"]],
                        what="git fetch ref")
            elif r.returncode != 0:
                raise cl.CorpusError(f"{ctx.iid}: git fetch of commit failed; add recipe.git.ref. {tail(ctx.log_path)}")
            if subprocess.run(["git", "-C", gitdir, "cat-file", "-e", f"{g['commit']}^{{commit}}"],
                              capture_output=True, env=ctx.env()).returncode != 0:
                raise cl.HashMismatch(f"{ctx.iid}: commit {g['commit']} not present after fetching {g['repo']}")
        cmd = ["git", "-C", gitdir, "-c", "tar.umask=0022", "archive", "--format=tar", "-o", tar_path, g["commit"]]
        if g.get("subpaths"):
            cmd += ["--"] + list(g["subpaths"])
        ctx.run(cmd, what="git archive")
    digest, size = cl.sha256_file(tar_path)
    ctx.inputs["git-archive"] = tar_path
    ctx.input_filenames["git-archive"] = "git-archive.tar"
    ctx.provenance["git"] = {"repo": g["repo"], "commit": g["commit"], "ref": g.get("ref"),
                             "subpaths": g.get("subpaths"), "archive_sha256": digest, "archive_bytes": size,
                             "archive_tar_umask": "0022", "git_version": cl.command_version(["git", "--version"])}


def acquire_docker(ctx: Ctx) -> None:
    d = ctx.item["recipe"]["docker"]
    exe = shutil.which("docker") or shutil.which("docker.exe")
    if not exe:
        raise cl.CorpusError(f"{ctx.iid}: docker CLI not found")
    platform = d.get("platform", "linux/amd64")
    image = d["image"]
    tar_path = ctx.scratch / "docker-rootfs.tar"
    with cl.file_lock(ctx.layout.lock_path("docker-" + cl.sha256_bytes(image.encode())[:24])):
        if not ctx.args.offline:
            ctx.run([exe, "pull", "--platform", platform, image], what="docker pull")
        cid = ctx.capture([exe, "create", "--platform", platform, image, "/ebrc-noop"], what="docker create")
        cid = cid.splitlines()[-1].strip()
        try:
            with open(tar_path, "wb") as f:
                ctx.run([exe, "export", cid], stdout=f, what="docker export")
        finally:
            subprocess.run([exe, "rm", "-f", cid], capture_output=True)
        inspect = ctx.capture([exe, "image", "inspect", "--format", "{{.Id}} {{.Os}}/{{.Architecture}}", image],
                              what="docker inspect")
    digest, size = cl.sha256_file(tar_path)
    ctx.inputs["docker-rootfs"] = tar_path
    ctx.input_filenames["docker-rootfs"] = "docker-rootfs.tar"
    ctx.provenance["docker"] = {
        "image": image, "platform": platform, "emulated": not platform.startswith("linux/amd64")
        and not platform.startswith("linux/386"),
        "image_inspect": inspect, "export_tar_sha256": digest, "export_tar_bytes": size,
        "docker_version": cl.command_version([exe, "version", "--format",
                                              "client {{.Client.Version}} server {{.Server.Version}}"]),
    }


# ---------------------------------------------------------------------------
# steps


def sniff(path: Path) -> str:
    with open(path, "rb") as f:
        head = f.read(512)
    if head.startswith(b"\x1f\x8b"):
        return "gz"
    if head.startswith(b"\xfd7zXZ\x00"):
        return "xz"
    if head.startswith(b"\x28\xb5\x2f\xfd"):
        return "zst"
    if head.startswith(b"BZh"):
        return "bz2"
    if head.startswith(b"LZIP"):
        return "lz"
    if head.startswith(b"\x04\x22\x4d\x18"):
        return "lz4"
    if len(head) >= 262 and head[257:262] == b"ustar":
        return "tar"
    if head.startswith(b"PK\x03\x04") or head.startswith(b"PK\x05\x06"):
        return "zip"
    if head.startswith(b"7z\xbc\xaf\x27\x1c"):
        return "7z"
    return "unknown"


def dest_of(ctx: Ctx, rel: str | None) -> Path:
    rel = rel or ""
    if not cl.safe_relpath(rel):
        raise cl.CorpusError(f"{ctx.iid}: unsafe dest {rel!r}")
    return ctx.staging if rel in ("", ".") else ctx.staging / rel


def step_extract(ctx: Ctx, st: dict) -> None:
    src = ctx.inputs[st["input"]]
    fmt = st.get("format", "auto")
    if fmt == "auto":
        s = sniff(src)
        fmt = {"gz": "tar", "xz": "tar", "zst": "tar", "bz2": "tar", "lz": "tar", "tar": "tar",
               "zip": "zip", "7z": "7z"}.get(s)
        if not fmt:
            raise cl.CorpusError(f"{ctx.iid}: cannot detect archive format of input {st['input']} ({s})")
    dest = dest_of(ctx, st.get("dest"))
    dest.mkdir(parents=True, exist_ok=True)
    strip = st.get("strip_components", 0)
    if fmt == "tar":
        ctx.used_tools.add("tar")
        cmd = ["tar", "-x", "-f", src, "-C", dest, "--numeric-owner", "--same-owner", "--same-permissions",
               "--xattrs", "--xattrs-include=*", "--acls", "--delay-directory-restore"]
        if strip:
            cmd.append(f"--strip-components={strip}")
    else:
        ctx.used_tools.add("bsdtar")
        cmd = ["bsdtar", "-x", "-p", "--numeric-owner", "-f", src, "-C", dest]
        if strip:
            cmd += ["--strip-components", str(strip)]
    ctx.run(cmd, what=f"extract {st['input']}")


def step_decompress(ctx: Ctx, st: dict) -> None:
    src = ctx.inputs[st["input"]]
    fmt = st.get("format", "auto")
    if fmt == "auto":
        fmt = sniff(src)
    tools = {"gz": ["gzip", "-dc"], "xz": ["xz", "-dc", "-T1"], "zst": ["zstd", "-dcq"], "bz2": ["bzip2", "-dc"],
             "lz": ["lzip", "-dc"], "lz4": ["lz4", "-dc"], "br": ["brotli", "-dc"]}
    if fmt not in tools:
        raise cl.CorpusError(f"{ctx.iid}: cannot decompress input {st['input']} (format {fmt})")
    ctx.used_tools.add({"gz": "gzip", "zst": "zstd", "lz": "lzip"}.get(fmt, fmt))
    dest = dest_of(ctx, st["dest"])
    dest.parent.mkdir(parents=True, exist_ok=True)
    with open(dest, "wb") as out:
        ctx.run(tools[fmt] + [src], stdout=out, what=f"decompress {st['input']}")
    os.chmod(dest, 0o644)


def step_copy(ctx: Ctx, st: dict) -> None:
    src = ctx.inputs[st["input"]]
    dest = dest_of(ctx, st.get("dest") or ctx.input_filenames[st["input"]])
    dest.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(src, dest)
    os.chmod(dest, int(st.get("mode", "0644"), 8))


def step_copy_item(ctx: Ctx, st: dict) -> None:
    ctx.used_tools.add("cp")
    _, base = ctx.deps[st["from_item"]]
    src = base if st.get("src", "") in ("", ".") else base / st["src"]
    if not (src.exists() or src.is_symlink()):
        raise cl.CorpusError(f"{ctx.iid}: copy-item source {src} missing")
    dest = dest_of(ctx, st.get("dest"))
    if src.is_dir() and not src.is_symlink():
        dest.mkdir(parents=True, exist_ok=True)
        ctx.run(["cp", "-a", "--reflink=never", f"{src}/.", f"{dest}/"], what="copy-item")
    else:
        dest.parent.mkdir(parents=True, exist_ok=True)
        ctx.run(["cp", "-a", "--reflink=never", src, dest], what="copy-item")


def step_remove(ctx: Ctx, st: dict) -> None:
    for rel in st["paths"]:
        p = dest_of(ctx, rel)
        if p.is_symlink() or p.is_file() or (p.exists() and not p.is_dir()):
            p.unlink()
        elif p.is_dir():
            shutil.rmtree(p)
        else:
            raise cl.CorpusError(f"{ctx.iid}: remove: {rel} does not exist (recipe drift?)")


def step_run(ctx: Ctx, spec: dict) -> dict:
    script = cl.REPO_ROOT / spec["script"]
    interp = {"python": sys.executable, "bash": "/bin/bash", "sh": "/bin/sh"}[spec.get("interpreter", "python")]
    params = spec.get("params", {}) or {}
    seed = spec.get("seed", 0)
    params_json = cl.canonical_json(params).decode("utf-8")
    extra = {
        "EB_OUT": str(ctx.staging), "EB_SEED": str(seed), "EB_PARAMS_JSON": params_json, "EB_ITEM_ID": ctx.iid,
        "EB_FAMILY": ctx.item["family"], "EB_SPLIT": ctx.item["split"], "EB_SCALE": ctx.item["scale"],
        "EB_SCRATCH": str(ctx.scratch), "EB_REPO": str(cl.REPO_ROOT), "EB_DATA_ROOT": str(ctx.layout.data_root),
        "EB_CACHE": str(ctx.layout.cache_dir),
        "EB_INPUTS_JSON": cl.canonical_json({k: str(v) for k, v in ctx.inputs.items()}).decode(),
        "EB_FROM_JSON": cl.canonical_json({k: str(p) for k, (_, p) in ctx.deps.items()}).decode(),
    }
    for k, v in ctx.inputs.items():
        extra["EB_INPUT_" + k.upper().replace("-", "_")] = str(v)
    for k, (_, p) in ctx.deps.items():
        extra["EB_FROM_" + k.upper().replace("-", "_")] = str(p)
    log(f"[{ctx.iid}] run {spec['script']} seed={seed}")
    ctx.run([interp, script, "--out", ctx.staging, "--seed", str(seed), "--params", params_json],
            cwd=ctx.scratch, extra_env=extra, what=f"script {spec['script']}")
    return {"script": spec["script"], "script_sha256": cl.sha256_file(script)[0], "interpreter": spec.get(
        "interpreter", "python"), "interpreter_path": interp, "seed": seed, "params": params}


def default_steps(ctx: Ctx) -> list:
    kind = ctx.item["kind"]
    if kind == "download":
        return [{"op": "copy", "input": inp["name"]} for inp in ctx.item["recipe"]["inputs"]]
    if kind == "git-archive":
        if (ctx.item["recipe"].get("git") or {}).get("history") == "full":
            return []  # acquire_git_full() already populated ctx.staging directly (a real clone)
        return [{"op": "extract", "input": "git-archive", "format": "tar"}]
    if kind == "docker-export":
        return [{"op": "extract", "input": "docker-rootfs", "format": "tar"}]
    return []


def materialize(ctx: Ctx) -> None:
    ctx.staging.mkdir(parents=True)
    os.chmod(ctx.staging, 0o755)
    ctx.scratch.mkdir(parents=True)
    r = ctx.item["recipe"]
    if r.get("inputs"):
        acquire_inputs(ctx)
    if r.get("git"):
        acquire_git(ctx)
    if r.get("docker"):
        acquire_docker(ctx)
    if ctx.deps:
        ctx.provenance["from_items"] = [
            {"item_id": k, "logical_tree_sha256": rec["fingerprint"]["logical_tree_sha256"]}
            for k, (rec, _) in sorted(ctx.deps.items())]
    steps = r.get("steps") or default_steps(ctx)
    runs = []
    for st in steps:
        op = st["op"]
        if op == "extract":
            step_extract(ctx, st)
        elif op == "decompress":
            step_decompress(ctx, st)
        elif op == "copy":
            step_copy(ctx, st)
        elif op == "copy-item":
            step_copy_item(ctx, st)
        elif op == "remove":
            step_remove(ctx, st)
        elif op == "run":
            runs.append(step_run(ctx, st))
    ctx.provenance["steps"] = steps
    if r.get("generator"):
        ctx.provenance["generator"] = step_run(ctx, r["generator"])
    if runs:
        ctx.provenance["run_steps"] = runs
    os.chmod(ctx.staging, 0o755)


# ---------------------------------------------------------------------------
# per item


def build_record(item, mk, fp_full, ctx_prov, env, pin_info, scale_check, materialized_utc, layout):
    heldout = item["split"] == "heldout"
    return {
        "format": cl.RECORD_FORMAT,
        "corpus_version": cl.CORPUS_VERSION,
        "item_id": item["item_id"],
        "family": item["family"],
        "split": item["split"],
        "scale": item["scale"],
        "kind": item["kind"],
        "real_or_generated": item["real_or_generated"],
        "independence_group": item["independence_group"],
        "source_file": item["_source_file"],
        "item_def_sha256": cl.item_def_sha256(item),
        "materialization_key": mk["sha256"],
        "materialized_utc": materialized_utc,
        "materialized_relpath": str(layout.item_path(item, allow_heldout=True).relative_to(layout.data_root)),
        "output_pin": pin_info,
        "scale_check": scale_check,
        "provenance": ctx_prov,
        "environment": env,
        "fingerprint": fpm.heldout_safe(fp_full) if heldout else fp_full,
        "fingerprint_view": "heldout-safe (hashes, bytes, basic counts)" if heldout else "full",
    }


def refresh_metadata(record: dict, item: dict) -> dict | None:
    """Definition changed only in non-recipe fields: rewrite descriptive fields."""
    new = dict(record)
    for k in ("family", "scale", "real_or_generated", "independence_group"):
        new[k] = item[k]
    new["source_file"] = item["_source_file"]
    new["item_def_sha256"] = cl.item_def_sha256(item)
    if record.get("scale") != item["scale"]:
        nbytes = record["fingerprint"]["bytes"]
        limit = cl.SCALES[item["scale"]]
        idx = cl.SCALE_ORDER.index(item["scale"])
        smaller = cl.SCALES[cl.SCALE_ORDER[idx - 1]] if idx > 0 else None
        if nbytes > limit:
            raise cl.CorpusError(f"{item['item_id']}: {nbytes} bytes exceeds scale tier {item['scale']} limit {limit}")
        new["scale_check"] = {"tier": item["scale"], "tier_limit_bytes": limit, "bytes": nbytes, "ok": True,
                              "fits_smaller_tier": smaller is not None and nbytes <= smaller}
    return new if new != record else None


def check_output_pin(item, mk_sha, logical, pins) -> tuple[dict, bool]:
    policy = cl.effective_output_pin(item)
    now = cl.utc_now()
    if policy is None:
        return {"policy": None, "status": "unpinned"}, False
    if policy != "TOFU":
        if logical != policy:
            raise cl.HashMismatch(f"HASH MISMATCH: {item['item_id']} logical_tree_sha256 {logical} "
                                  f"!= declared output_pin {policy}")
        return {"policy": "declared", "status": "matched-declared", "logical_tree_sha256": policy}, False
    op = pins.get("output")
    if op and op.get("materialization_key") == mk_sha:
        if logical != op["logical_tree_sha256"]:
            raise cl.HashMismatch(f"HASH MISMATCH: {item['item_id']} logical_tree_sha256 {logical} "
                                  f"!= TOFU pin {op['logical_tree_sha256']} (pinned {op.get('pinned_utc')}); "
                                  f"the recipe is not reproducible or an input drifted")
        return {"policy": "TOFU", "status": "matched-tofu", "logical_tree_sha256": logical,
                "pinned_utc": op.get("pinned_utc")}, False
    if op:
        pins["history"].append({"replaced_utc": now, "output": op})
    pins["output"] = {"materialization_key": mk_sha, "logical_tree_sha256": logical, "pinned_utc": now}
    return {"policy": "TOFU", "status": "tofu-first-pin", "logical_tree_sha256": logical, "pinned_utc": now}, True


def provision_item(item: dict, items: dict, layout: cl.Layout, args) -> str:
    iid = item["item_id"]
    heldout = item["split"] == "heldout"
    with cl.file_lock(layout.lock_path("item-" + iid)):
        deps = {}
        for d in item["recipe"].get("from_items", []) or []:
            rec = cl.load_json_if_exists(layout.record_path(d))
            dpath = layout.item_path(items[d], allow_heldout=True)
            if not rec or not dpath.is_dir():
                raise cl.CorpusError(f"{iid}: dependency {d} is not materialized")
            deps[d] = (rec, dpath)
        mk = cl.materialization_key(item, {k: rec["fingerprint"]["logical_tree_sha256"] for k, (rec, _) in deps.items()})
        target = layout.item_path(item, allow_heldout=True)
        record_path = layout.record_path(iid)
        record = cl.load_json_if_exists(record_path)
        pins = cl.load_json_if_exists(layout.pins_path(iid)) or {
            "format": cl.PINS_FORMAT, "item_id": iid, "inputs": {}, "output": None, "history": []}

        # ---- existing materialization -------------------------------------------------
        if target.is_dir() and record and record.get("materialization_key") == mk["sha256"] \
                and record.get("split") == item["split"] and not args.rebuild:
            rec_logical = record["fingerprint"]["logical_tree_sha256"]
            policy = cl.effective_output_pin(item)
            if policy not in (None, "TOFU") and policy != rec_logical:
                raise cl.HashMismatch(f"HASH MISMATCH: {iid} recorded logical_tree_sha256 {rec_logical} "
                                      f"!= declared output_pin {policy}")
            op = pins.get("output")
            if policy == "TOFU" and op and op.get("materialization_key") == mk["sha256"] \
                    and op["logical_tree_sha256"] != rec_logical:
                raise cl.HashMismatch(f"HASH MISMATCH: {iid} record {rec_logical} != TOFU pin {op['logical_tree_sha256']}")
            verify = args.verify
            if verify == "auto":
                verify = "full" if record["fingerprint"]["bytes"] <= cl.SCALES["medium"] else "quick"
            state = cl.load_json_if_exists(layout.state_path(iid))
            status = None
            if verify == "quick" and state and state.get("materialization_key") == mk["sha256"] \
                    and state.get("logical_tree_sha256") == rec_logical:
                if fpm.stat_digest(fpm.scan(target)) == state.get("stat_digest"):
                    status = "up-to-date (quick stat check)"
            if status is None:
                fp = fpm.fingerprint_path(target, jobs=args.hash_jobs, with_stat_digest=True)
                if fp["logical_tree_sha256"] != rec_logical:
                    raise cl.HashMismatch(
                        f"HASH MISMATCH: materialized {target} has logical_tree_sha256 {fp['logical_tree_sha256']} "
                        f"but {cl.repo_rel(record_path)} records {rec_logical}. The tree was modified or the "
                        f"record is stale; investigate, then rerun with --rebuild")
                cl.write_json_atomic(layout.state_path(iid), {
                    "item_id": iid, "materialization_key": mk["sha256"], "logical_tree_sha256": rec_logical,
                    "tree_sha256": fp["tree_sha256"], "stat_digest": fp["_stat_digest"], "verified_utc": cl.utc_now()})
                note = "" if fp["tree_sha256"] == record["fingerprint"]["tree_sha256"] else \
                    " (note: tree_sha256 differs: st_blocks allocation changed)"
                status = "up-to-date (full fingerprint)" + note
            if policy == "TOFU" and not op:
                pins["output"] = {"materialization_key": mk["sha256"], "logical_tree_sha256": rec_logical,
                                  "pinned_utc": record.get("materialized_utc") or cl.utc_now()}
                cl.write_json_atomic(layout.pins_path(iid), pins)
            refreshed = refresh_metadata(record, item)
            if refreshed:
                cl.write_json_atomic(record_path, refreshed)
                status += "; record metadata refreshed"
            return status

        if args.dry_run:
            return "would materialize"
        if target.exists() and not record:
            log(f"[{iid}] existing directory without a fingerprint record: rematerializing")
        elif record and record.get("materialization_key") != mk["sha256"] and not args.rebuild:
            log(f"[{iid}] recipe/scripts/dependencies changed: rematerializing")

        # ---- materialize ------------------------------------------------------------------
        for d in (layout.staging_dir, layout.scratch_dir, target.parent, layout.lines_path(item).parent):
            d.mkdir(parents=True, exist_ok=True)
        if heldout:
            os.chmod(layout.heldout_root, 0o700)
            os.chmod(layout.lines_path(item).parent, 0o700)
        ctx = Ctx(item, layout, args, deps)
        ctx.pins = pins
        lines_tmp = layout.lines_path(item).with_name(f".{iid}.{os.getpid()}.lines.gz.tmp")
        ok = False
        try:
            materialize(ctx)
            fp = fpm.fingerprint_path(ctx.staging, jobs=args.hash_jobs, lines_out=str(lines_tmp), with_stat_digest=True)
            limit = cl.SCALES[item["scale"]]
            idx = cl.SCALE_ORDER.index(item["scale"])
            smaller = cl.SCALES[cl.SCALE_ORDER[idx - 1]] if idx > 0 else None
            scale_check = {"tier": item["scale"], "tier_limit_bytes": limit, "bytes": fp["bytes"],
                           "ok": fp["bytes"] <= limit,
                           "fits_smaller_tier": smaller is not None and fp["bytes"] <= smaller}
            if not scale_check["ok"]:
                raise cl.CorpusError(f"{iid}: {fp['bytes']} bytes exceeds scale tier {item['scale']} "
                                     f"limit {limit}")
            if scale_check["fits_smaller_tier"]:
                log(f"[{iid}] WARNING: {fp['bytes']} bytes fits a smaller tier than {item['scale']}")
            if record and record.get("materialization_key") == mk["sha256"] and cl.effective_output_pin(item) is None \
                    and record["fingerprint"]["logical_tree_sha256"] != fp["logical_tree_sha256"]:
                log(f"[{iid}] note: unpinned output differs from previous record (non-reproducible kind)")
            pin_info, pin_changed = check_output_pin(item, mk["sha256"], fp["logical_tree_sha256"], pins)
            if pin_info["status"] == "tofu-first-pin":
                log(f"[{iid}] TOFU pinned output logical_tree_sha256 {fp['logical_tree_sha256']}")
            # swap into place
            old = None
            if target.exists() or target.is_symlink():
                old = layout.staging_dir / f"{iid}.old.{os.getpid()}.{secrets.token_hex(4)}"
                os.rename(target, old)
            os.rename(ctx.staging, target)
            if old:
                shutil.rmtree(old)
            os.replace(lines_tmp, layout.lines_path(item))
            if heldout:
                os.chmod(layout.lines_path(item), 0o600)
            now = cl.utc_now()
            env = environment_versions(ctx.used_tools)
            env["tool_sha256"] = cl.tool_digests()
            rec = build_record(item, mk, {k: v for k, v in fp.items() if not k.startswith("_")},
                               ctx.provenance, env, pin_info, scale_check, now, layout)
            cl.write_json_atomic(layout.state_path(iid), {
                "item_id": iid, "materialization_key": mk["sha256"], "logical_tree_sha256": fp["logical_tree_sha256"],
                "tree_sha256": fp["tree_sha256"], "stat_digest": fp["_stat_digest"], "verified_utc": now})
            if ctx.pins_dirty or pin_changed:
                cl.write_json_atomic(layout.pins_path(iid), pins)
            cl.write_json_atomic(record_path, rec)
            ok = True
            return f"materialized logical_tree_sha256={fp['logical_tree_sha256']} bytes={fp['bytes']}"
        except cl.HashMismatch:
            if ctx.staging.exists():
                layout.quarantine_dir.mkdir(parents=True, exist_ok=True)
                q = layout.quarantine_dir / ctx.staging.name
                os.rename(ctx.staging, q)
                log(f"[{iid}] staging tree kept for inspection at {q}")
            raise
        finally:
            ctx.close()
            with contextlib.suppress(OSError):
                lines_tmp.unlink()
            if ctx.scratch.exists():
                shutil.rmtree(ctx.scratch, ignore_errors=True)
            if not ok and ctx.staging.exists() and not args.keep_failed:
                shutil.rmtree(ctx.staging, ignore_errors=True)


# ---------------------------------------------------------------------------
# main


def split_csv(values):
    out = set()
    for v in values or []:
        out.update(x.strip() for x in v.split(",") if x.strip())
    return out


def find_orphans(layout: cl.Layout, items: dict) -> list:
    orphans = []
    for split in cl.SPLITS:
        root = layout.split_root(split)
        if not root.is_dir():
            continue
        for p in sorted(root.iterdir()):
            it = items.get(p.name)
            if not it or it["split"] != split:
                orphans.append(str(p))
    return orphans


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--corpus-dir", help="default research/corpus (env EB_CORPUS_DIR)")
    ap.add_argument("--data-root", help="default /root/eb-research (env EB_DATA_ROOT)")
    ap.add_argument("--family", action="append", help="F01..F20 (repeatable / comma separated)")
    ap.add_argument("--group", action="append", help="independence_group filter")
    ap.add_argument("--item", action="append", help="item_id filter")
    ap.add_argument("--split", action="append", help="tuning|validation|heldout filter")
    ap.add_argument("--jobs", type=int, default=2, help="items materialized concurrently (default 2)")
    ap.add_argument("--hash-jobs", type=int, default=4, help="content hashing threads per item (default 4)")
    ap.add_argument("--verify", choices=("auto", "full", "quick"), default="auto",
                    help="check of already-materialized items: full re-fingerprint, quick lstat digest, "
                         "auto = full up to 512 MiB else quick")
    ap.add_argument("--rebuild", action="store_true", help="rematerialize selected items even if up to date")
    ap.add_argument("--check", action="store_true", help="validate source definitions and exit")
    ap.add_argument("--list", action="store_true", help="list items with materialization status and exit")
    ap.add_argument("--dry-run", action="store_true", help="report what would be materialized")
    ap.add_argument("--offline", action="store_true", help="never touch the network (cache only)")
    ap.add_argument("--reverify-cache", action="store_true", help="rehash cached download blobs")
    ap.add_argument("--keep-failed", action="store_true", help="keep staging dirs of failed items")
    args = ap.parse_args(argv)

    cl.require_linux()
    os.umask(0o022)
    layout = cl.Layout(args.corpus_dir, args.data_root)
    items, errors, warnings = cl.load_sources(layout)
    for w in warnings:
        log(f"WARNING: {w}")
    if errors:
        for e in errors:
            log(f"ERROR: {e}")
        log(f"{len(errors)} source definition error(s)")
        return 2
    if args.check or args.list:
        for iid in sorted(items, key=lambda k: (items[k]["family"], items[k]["split"], k)):
            it = items[iid]
            rec = cl.load_json_if_exists(layout.record_path(iid))
            status = "recorded" if rec else "not-materialized"
            log(f"{it['family']} {it['split']:<10} {it['scale']:<6} {it['kind']:<13} {iid}  [{status}]")
        for o in find_orphans(layout, items):
            log(f"WARNING: orphan directory not defined by any source (or split changed): {o}")
        log(f"OK: {len(items)} item(s) in {len(list(layout.sources_dir.glob('*.json')))} source file(s)")
        return 0

    fams, groups, ids, splits = split_csv(args.family), split_csv(args.group), split_csv(args.item), split_csv(args.split)
    unknown = ids - set(items)
    if unknown:
        log(f"ERROR: unknown item(s): {sorted(unknown)}")
        return 2
    selected = {iid for iid, it in items.items()
                if (not fams or it["family"] in fams) and (not groups or it["independence_group"] in groups)
                and (not ids or iid in ids) and (not splits or it["split"] in splits)}
    frontier = list(selected)
    while frontier:
        n = frontier.pop()
        for d in (items[n]["recipe"].get("from_items") or []):
            if d not in selected:
                selected.add(d)
                frontier.append(d)
                log(f"[{d}] included as dependency of {n}")
    if not selected:
        log("nothing selected")
        return 0
    failed, done = set(), {}
    for level in cl.dependency_order(items, selected):
        with ThreadPoolExecutor(max_workers=max(1, args.jobs)) as ex:
            futs = {}
            for iid in level:
                bad = [d for d in (items[iid]["recipe"].get("from_items") or []) if d in failed]
                if bad:
                    failed.add(iid)
                    log(f"[{iid}] SKIPPED: dependency failed: {bad}")
                    continue
                futs[ex.submit(provision_item, items[iid], items, layout, args)] = iid
            for fut in as_completed(futs):
                iid = futs[fut]
                try:
                    done[iid] = fut.result()
                    log(f"[{iid}] {done[iid]}")
                except cl.HashMismatch as e:
                    failed.add(iid)
                    log(f"[{iid}] FAILED (HASH MISMATCH): {e}")
                except Exception as e:  # noqa: BLE001
                    failed.add(iid)
                    log(f"[{iid}] FAILED: {e}")
                    if not isinstance(e, cl.CorpusError):
                        log(traceback.format_exc())
    log(f"provision: {len(done)} ok, {len(failed)} failed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
