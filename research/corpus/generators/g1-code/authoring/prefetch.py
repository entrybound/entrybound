#!/usr/bin/env python3
"""Authoring helper for the g1-code source definitions (not used by provision.py).

Downloads every URL listed in downloads.json through provision.fetch_url, so the blobs land in
the provision content-addressed cache (/root/eb-research/cache/sha256 + by-url index) with the
same provenance format, then cross-checks each SHA-256 against the checksum the upstream project
publishes where one exists.  Writes downloads.lock.json next to this script:
    {url: {sha256, size, final_url, retrieved_utc, retrieval_date, published_check}}
The declared sha256/size values in sources/g1-code.json are copied from that file by
make_sources.py.  A published-checksum mismatch aborts loudly.

Only hashes and sizes are computed here, so running it over held-out inputs is allowed.

    run inside WSL:  /root/eb-research/venv/bin/python prefetch.py [--jobs 6] [--only SUBSTR]
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
import threading
import urllib.request
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path

HERE = Path(__file__).resolve().parent
TOOLS = HERE.parents[2] / "tools"
sys.path.insert(0, str(TOOLS))
import corpuslib as cl  # noqa: E402
import provision  # noqa: E402

UA = {"User-Agent": "entrybound-research-corpus-provision/1"}
_text_cache: dict[str, str] = {}
_lock = threading.Lock()


def get_text(url: str) -> str:
    with _lock:
        if url in _text_cache:
            return _text_cache[url]
    with urllib.request.urlopen(urllib.request.Request(url, headers=UA), timeout=120) as r:
        text = r.read().decode("utf-8", "replace")
    with _lock:
        _text_cache[url] = text
    return text


def published_sha256(entry: dict) -> tuple[str | None, str]:
    pub = entry.get("published")
    if not pub:
        return None, "none-published"
    kind = pub["kind"]
    name = entry["url"].rstrip("/").rsplit("/", 1)[-1]
    if kind == "declared":
        return pub["sha256"], f"declared by {pub['source']}"
    text = get_text(pub["url"])
    if kind == "sha256-file":
        m = re.findall(r"\b[0-9a-f]{64}\b", text)
        if len(set(m)) != 1:
            raise SystemExit(f"cannot parse {pub['url']}")
        return m[0], pub["url"]
    if kind in ("sha256sums", "redis-hashes"):
        for line in text.splitlines():
            toks = line.split()
            if name in toks:
                hx = [t for t in toks if re.fullmatch(r"[0-9a-f]{64}", t)]
                if hx:
                    return hx[0], pub["url"]
        raise SystemExit(f"{name} not listed in {pub['url']}")
    if kind == "lua-ftp":
        m = re.search(r'HREF="' + re.escape(name) + r'".*?CLASS="sum">([0-9a-f]{64})<', text, re.S)
        if not m:
            raise SystemExit(f"{name} not listed in {pub['url']}")
        return m.group(1), pub["url"]
    if kind == "python-api":
        rel = json.loads(text)
        if len(rel) != 1:
            raise SystemExit(f"python.org API: expected one release for {pub['url']}")
        rid = rel[0]["resource_uri"].rstrip("/").rsplit("/", 1)[-1]
        furl = f"https://www.python.org/api/v2/downloads/release_file/?release={rid}"
        for f in json.loads(get_text(furl)):
            if f.get("url", "").rstrip("/").rsplit("/", 1)[-1] == name:
                if f.get("sha256_sum"):
                    return f["sha256_sum"], furl
                return None, f"md5:{f.get('md5_sum')}"
        return None, "python.org API: file not listed"
    raise SystemExit(f"unknown published kind {kind}")


class _Args:
    reverify_cache = False
    offline = False


class _Ctx:
    def __init__(self, layout):
        self.layout = layout
        self.args = _Args()
        self.iid = "g1-code-prefetch"


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--jobs", type=int, default=6)
    ap.add_argument("--only", default=None)
    args = ap.parse_args()
    spec = cl.load_json(HERE / "downloads.json")
    lock_path = HERE / "downloads.lock.json"
    lock = cl.load_json_if_exists(lock_path) or {}
    layout = cl.Layout()
    ctx = _Ctx(layout)
    todo = [e for e in spec["downloads"] if not args.only or args.only in e["url"]]

    def one(e):
        prev = lock.get(e["url"])
        info = provision.fetch_url(ctx, e["url"], prev["sha256"] if prev else None, prev["size"] if prev else None)
        pub, src = published_sha256(e)
        if src.startswith("md5:"):
            h = hashlib.md5()
            with open(info["path"], "rb") as fh:
                for chunk in iter(lambda: fh.read(1 << 20), b""):
                    h.update(chunk)
            if h.hexdigest() != src[4:]:
                raise cl.HashMismatch(f"HASH MISMATCH: {e['url']} md5 {h.hexdigest()} != python.org API md5 {src[4:]}")
            src = "match (md5 only; python.org API publishes no sha256): https://www.python.org/api/v2/downloads/"
        if pub and pub != info["sha256"]:
            raise cl.HashMismatch(f"HASH MISMATCH: {e['url']} sha256 {info['sha256']} != published {pub} ({src})")
        check = f"match: {src}" if pub else src
        rec = {"sha256": info["sha256"], "size": info["size"], "final_url": (prev or {}).get("final_url") or info.get("final_url"),
               "retrieved_utc": (prev or {}).get("retrieved_utc") or info.get("retrieved_utc"),
               "retrieval_date": (prev or {}).get("retrieval_date") or info.get("retrieval_date"),
               "published_check": check}
        if rec["final_url"] and "?" in rec["final_url"]:
            # GitHub release assets redirect to short-lived signed URLs; drop the query string both
            # here and in the provision by-url index (it is copied into committed provenance).
            rec["final_url"] = rec["final_url"].split("?", 1)[0]
            idx_path = layout.cache_dir / "by-url" / f"{cl.sha256_bytes(e['url'].encode('utf-8'))}.json"
            idx = cl.load_json_if_exists(idx_path)
            if idx and idx.get("final_url") and "?" in idx["final_url"]:
                idx["final_url"] = idx["final_url"].split("?", 1)[0]
                cl.write_json_atomic(idx_path, idx)
        return e["url"], rec

    failed = 0
    with ThreadPoolExecutor(max_workers=args.jobs) as ex:
        futs = [ex.submit(one, e) for e in todo]
        for f in as_completed(futs):
            try:
                url, rec = f.result()
            except Exception as exc:  # noqa: BLE001
                failed += 1
                print(f"FAILED: {exc}", flush=True)
                continue
            lock[url] = rec
            print(f"{rec['sha256']} {rec['size']:>11} {url} [{rec['published_check']}]", flush=True)
    cl.write_json_atomic(lock_path, dict(sorted(lock.items())))
    print(f"prefetch: {len(todo) - failed} ok, {failed} failed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
