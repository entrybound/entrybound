"""Single source of truth for the g1-code upstream selections (imported by make_sources.py and
used to emit downloads.json for prefetch.py).  Authoring-time only.

Selection rationale: every split draws from a disjoint set of upstream projects so that no
repository, URL or pinned hash crosses splits (corpuslib.load_sources enforces this).
  tuning:     zstd, ripgrep, curl, llvm-project, sqlite, lz4, TypeScript(npm), PyPI scientific stack, fzf(go)
  validation: CPython, redis, bat(cargo), bootstrap(npm), cJSON, yq(go)
  heldout:    Linux 6.6, Go, musl, PostgreSQL, Lua, alacritty(cargo), pdf.js(npm), PyPI web stack, direnv(go)
"""

RETRIEVAL_DATE = "2026-09-12"

# ---- git tags resolved with `git ls-remote --tags` on 2026-09-12 (peeled commit ids) ----------
GIT = {
    "zstd": ("https://github.com/facebook/zstd", "f8745da6ff1ad1e7bab384bd1f9d742439278e99", "v1.5.7"),
    "ripgrep": ("https://github.com/BurntSushi/ripgrep", "4649aa9700619f94cf9c66876e9549d83420e16c", "14.1.1"),
    "curl": ("https://github.com/curl/curl", "8c908d2d0a6d32abdedda2c52e90bd56ec76c24d", "curl-8_19_0"),
    "llvm": ("https://github.com/llvm/llvm-project", "87f0227cb60147a26a1eeb4fb06e3b505e9c7261", "llvmorg-20.1.8"),
    "cpython": ("https://github.com/python/cpython", "4061bc4c35f7c26f25264666d4ba083b93d2f6f9", "v3.13.15"),
    "redis": ("https://github.com/redis/redis", "335554f18caf7bbf6b0ac2b3548133d750f00a1b", "7.2.16"),
    "bat": ("https://github.com/sharkdp/bat", "25f4f96ea3afb6fe44552f3b38ed8b1540ffa1b3", "v0.25.0"),
    "alacritty": ("https://github.com/alacritty/alacritty", "0c405d53e74ace2980fc5e6c6d5b710c144bc075", "v0.15.1"),
    # Go module projects for small `go mod vendor` trees (go directive <= 1.22, built with GOTOOLCHAIN=local)
    "fzf": ("https://github.com/junegunn/fzf", "3c0a6304756e890e0a605b742943a9bb8e1d2247", "0.46.1"),
    "yq": ("https://github.com/mikefarah/yq", "dd648994340a5d03225d97abf19c9bf1086c3f07", "v4.40.5"),
    "direnv": ("https://github.com/direnv/direnv", "b2f5e9f205c43670cc948c5ee77a06077a493b2f", "v2.34.0"),
}

ZSTD_RELEASES = ["1.5.0", "1.5.1", "1.5.2", "1.5.4", "1.5.5", "1.5.6", "1.5.7"]  # upstream skipped 1.5.3
SQLITE_AMALGAMATIONS = [  # (sqlite.org year dir, version number)
    ("2024", "3450000"), ("2024", "3450100"), ("2024", "3450200"), ("2024", "3450300"),
    ("2024", "3460000"), ("2024", "3460100"), ("2024", "3470000"), ("2024", "3470100"),
    ("2024", "3470200"), ("2025", "3480000"), ("2025", "3490000"), ("2025", "3490100"),
    ("2025", "3490200"), ("2025", "3500000"),
]
LZ4_TAGS = ["v1.9.0", "v1.9.1", "v1.9.2", "v1.9.3", "v1.9.4", "v1.10.0"]
# 16 consecutive first-parent commits of BurntSushi/ripgrep ending at tag 14.1.0
RIPGREP_COMMITS = [
    "2057023dc5eb2b2ab28d03d2b902f3b57ce93165", "8e8fc9c5039663bf0145d7ae292c4214da957d4f",
    "6c2a550e1ed190351707dbcb28d5085a89ac0710", "827082a33ae7d86846a103ef14fb9ff297587049",
    "5dec4b8e3755bcc0be27fb43bd20880d6da1c42b", "23af5fb043f8e625294e210446090f17dbd7e3bf",
    "e0a85678e112759f56f9e07d04a29f2eed6eb348", "67dd809a8097923a721305ce0632a9fb9f19cb57",
    "b9c774937fc285e6668be9823c4bf231a61fc4a8", "f02a50a69da131269fe0d8460e08b1fceea04ca9",
    "c8e4a84519ede79b330f9a9e2b072110d5bd0c8c", "6e9141a9ca39a7ffbb25bf0817d21895e486aacc",
    "2c3897585d3a6d0709954bc389bed3d957878048", "44aa5a417d396ec75613c876e0eb5e4ba2f03b1d",
    "1fa76d2a42f05509a01c3e50a9424d16cfa8726d", "e50df40a1967708b9781486b1c017e48040bceb0",
]
TYPESCRIPT_COMMIT = ("c63de15a992d37f0d6cec03ac7631872838602cb", "v5.9.3")
CURL_RELEASES = ["8.0.0", "8.0.1", "8.1.0", "8.1.1", "8.1.2", "8.2.0", "8.2.1", "8.3.0", "8.4.0", "8.5.0", "8.6.0",
                 "8.7.0", "8.7.1", "8.8.0", "8.9.0", "8.9.1", "8.10.0", "8.10.1", "8.11.0", "8.11.1", "8.12.0",
                 "8.12.1", "8.13.0", "8.14.0", "8.14.1", "8.15.0", "8.16.0", "8.17.0", "8.18.0", "8.19.0"]

PYTHON312 = ["3.12.0", "3.12.1", "3.12.2", "3.12.3", "3.12.4", "3.12.5"]
REDIS72 = [f"7.2.{i}" for i in range(10)]
CJSON_TAGS = ["v1.7.12", "v1.7.13", "v1.7.14", "v1.7.15", "v1.7.16", "v1.7.17", "v1.7.18", "v1.7.19"]
BOOTSTRAP_COMMIT = ("25aa8cc0b32f0d1a54be575347e6d84b70b1acd7", "v5.3.8")

LINUX_BASE = "6.6"
LINUX_PATCHES = [30, 60, 90]  # stable patches are cumulative against 6.6
GO_SRC = ("go1.25.0", "4bd01e91297207bfa450ea40d4d5a93b1b531a5e438473b2a06e18e077227225", 31974753)
MUSL_RELEASES = ["1.2.0", "1.2.1", "1.2.2", "1.2.3", "1.2.4", "1.2.5"]
LUA_RELEASES = ["5.4.0", "5.4.1", "5.4.2", "5.4.3", "5.4.4", "5.4.5", "5.4.6", "5.4.7", "5.4.8"]
POSTGRES = "17.6"
PDFJS_COMMIT = ("f9bea397f817100a186d0ced2aa3d4b85813b637", "v4.10.38")


def gh_raw(org_repo, commit, path):
    return f"https://raw.githubusercontent.com/{org_repo}/{commit}/{path}"


def urls():
    """Yield (split, url, published-checksum spec or None)."""
    kernel_sums = {"kind": "sha256sums", "url": "https://cdn.kernel.org/pub/linux/kernel/v6.x/sha256sums.asc"}
    for v in ZSTD_RELEASES:
        u = f"https://github.com/facebook/zstd/releases/download/v{v}/zstd-{v}.tar.gz"
        yield "tuning", u, {"kind": "sha256-file", "url": u + ".sha256"}
    for y, n in SQLITE_AMALGAMATIONS:
        yield "tuning", f"https://www.sqlite.org/{y}/sqlite-amalgamation-{n}.zip", None
    for t in LZ4_TAGS:
        yield "tuning", f"https://codeload.github.com/lz4/lz4/tar.gz/refs/tags/{t}", None
    for c in RIPGREP_COMMITS:
        yield "tuning", f"https://codeload.github.com/BurntSushi/ripgrep/tar.gz/{c}", None
    for f in ("package.json", "package-lock.json"):
        yield "tuning", gh_raw("microsoft/TypeScript", TYPESCRIPT_COMMIT[0], f), None
    for v in CURL_RELEASES:
        yield "tuning", f"https://curl.se/download/curl-{v}.tar.xz", None
    for v in PYTHON312:
        yield "validation", f"https://www.python.org/ftp/python/{v}/Python-{v}.tar.xz", {
            "kind": "python-api", "url": f"https://www.python.org/api/v2/downloads/release/?name=Python%20{v}"}
    for v in REDIS72:
        yield "validation", f"https://download.redis.io/releases/redis-{v}.tar.gz", {
            "kind": "redis-hashes", "url": "https://raw.githubusercontent.com/redis/redis-hashes/master/README"}
    for t in CJSON_TAGS:
        yield "validation", f"https://codeload.github.com/DaveGamble/cJSON/tar.gz/refs/tags/{t}", None
    for f in ("package.json", "package-lock.json"):
        yield "validation", gh_raw("twbs/bootstrap", BOOTSTRAP_COMMIT[0], f), None
    yield "heldout", f"https://cdn.kernel.org/pub/linux/kernel/v6.x/linux-{LINUX_BASE}.tar.xz", kernel_sums
    for n in LINUX_PATCHES:
        yield "heldout", f"https://cdn.kernel.org/pub/linux/kernel/v6.x/patch-{LINUX_BASE}.{n}.xz", kernel_sums
    yield "heldout", f"https://go.dev/dl/{GO_SRC[0]}.src.tar.gz", {
        "kind": "declared", "sha256": GO_SRC[1], "source": "https://go.dev/dl/?mode=json&include=all"}
    for v in MUSL_RELEASES:
        yield "heldout", f"https://musl.libc.org/releases/musl-{v}.tar.gz", None
    for v in LUA_RELEASES:
        yield "heldout", f"https://www.lua.org/ftp/lua-{v}.tar.gz", {"kind": "lua-ftp", "url": "https://www.lua.org/ftp/"}
    u = f"https://ftp.postgresql.org/pub/source/v{POSTGRES}/postgresql-{POSTGRES}.tar.bz2"
    yield "heldout", u, {"kind": "sha256-file", "url": u + ".sha256"}
    for f in ("package.json", "package-lock.json"):
        yield "heldout", gh_raw("mozilla/pdf.js", PDFJS_COMMIT[0], f), None


if __name__ == "__main__":
    import json
    import sys
    from pathlib import Path
    out = {"notes": "generated by upstreams.py; consumed by prefetch.py",
           "downloads": [{"split": s, "url": u, **({"published": p} if p else {})} for s, u, p in urls()]}
    Path(__file__).with_name("downloads.json").write_text(json.dumps(out, indent=2) + "\n", encoding="utf-8")
    print(f"{len(out['downloads'])} downloads", file=sys.stderr)
