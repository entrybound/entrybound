#!/usr/bin/env python3
"""Authoring helper: emit research/corpus/sources/g1-code.json (families F01, F02, F03, F18).

Reads upstreams.py (selections) and downloads.lock.json (SHA-256 + size of every download,
cross-checked by prefetch.py against upstream-published checksums where available).
Re-running it is deterministic; the JSON file is the committed artefact.

    python make_sources.py            (Windows or WSL; stdlib only)
"""

import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import upstreams as U  # noqa: E402

REPO = HERE.parents[4]
LOCK = json.loads((HERE / "downloads.lock.json").read_text(encoding="utf-8"))
GEN = "research/corpus/generators/g1-code"
BUILD = f"{GEN}/build_tree.py"

RUSTC = "rustc 1.98.1 (48a229cea 2026-09-01)"
CARGO = "cargo 1.98.1"
GCC = "13.3.0"
NODE, NPM = "v22.23.2", "10.9.8"
PY, PIP = "Python 3.12.3", "pip 24.0 "
JOBS = 16

items = []


def lic(spdx, redistributable, attribution, notes=""):
    return {"spdx_or_name": spdx, "redistributable": redistributable, "attribution": attribution, "notes": notes}


def inp(name, url, filename=None, notes=None):
    rec = LOCK[url]
    d = {"name": name, "url": url, "sha256": rec["sha256"], "size": rec["size"]}
    if filename:
        d["filename"] = filename
    check = rec.get("published_check", "")
    if notes or check:
        d["notes"] = "; ".join(x for x in (notes, f"published checksum: {check}") if x)
    return d


def add(item_id, family, split, scale, kind, rog, recipe, license_, group, description, notes=None, tags=None):
    it = {"item_id": item_id, "family": family, "split": split, "scale": scale, "kind": kind,
          "real_or_generated": rog, "recipe": recipe, "license": license_, "independence_group": group,
          "description": description}
    if notes:
        it["notes"] = notes
    if tags:
        it["tags"] = tags
    items.append(it)
    return item_id


def git(key):
    repo, commit, ref = U.GIT[key]
    return {"repo": repo, "commit": commit, "ref": ref}


def gcc_expect():
    return [[["gcc", "-dumpfullversion"], GCC], [["make", "--version"], "GNU Make 4.3"]]


def rust_env():
    return {"RUSTUP_TOOLCHAIN": "stable", "CARGO_HOME": "{cache}/cargo-home", "CARGO_TERM_COLOR": "never",
            "CARGO_TERM_PROGRESS_WHEN": "never"}


def rust_expect():
    return [[["rustc", "--version"], RUSTC], [["cargo", "--version"], CARGO]]


def npm_env():
    return {"npm_config_cache": "{cache}/npm-cache", "npm_config_update_notifier": "false",
            "npm_config_fund": "false", "npm_config_audit": "false", "npm_config_progress": "false"}


# licenses ------------------------------------------------------------------------------------
L_ZSTD = lic("BSD-3-Clause OR GPL-2.0-only", True, "Meta Platforms, Inc. and affiliates; Zstandard contributors",
             "dual licensed; contrib/ and tests/ files carry their own headers")
L_RG = lic("Unlicense OR MIT", True, "Andrew Gallant and ripgrep contributors")
L_CURL = lic("curl", True, "Daniel Stenberg and curl contributors", "curl license (MIT/X derivative); see COPYING")
L_LLVM = lic("Apache-2.0 WITH LLVM-exception", True, "LLVM Project contributors",
             "a few subprojects/test inputs carry other permissive licenses; see the per-directory LICENSE.TXT files")
L_SQLITE = lic("blessing", True, "D. Richard Hipp and SQLite contributors", "public domain dedication (SQLite blessing)")
L_LZ4 = lic("BSD-2-Clause AND GPL-2.0-only", True, "Yann Collet and LZ4 contributors",
            "lib/ is BSD-2-Clause; programs/, tests/ and examples/ are GPL-2.0-only")
L_CPY = lic("PSF-2.0", True, "Python Software Foundation and CPython contributors",
            "includes third-party code under compatible permissive licenses; see LICENSE")
L_REDIS = lic("BSD-3-Clause", True, "Redis Ltd., Salvatore Sanfilippo and contributors",
              "the 7.2.x branch is BSD-3-Clause (the RSALv2/SSPLv1 relicensing applies to 7.4+; checked COPYING at "
              "7.2.16); bundled deps: jemalloc BSD-2-Clause, Lua MIT, hiredis BSD-3-Clause, linenoise BSD-2-Clause")
L_CJSON = lic("MIT", True, "Dave Gamble and cJSON contributors", "tests/unity is MIT (ThrowTheSwitch)")
L_MUSL = lic("MIT", True, "Rich Felker and musl contributors", "see COPYRIGHT")
L_GO = lic("BSD-3-Clause", True, "The Go Authors", "vendored third-party modules under permissive licenses")
L_LINUX = lic("GPL-2.0-only WITH Linux-syscall-note", True, "Linus Torvalds and Linux kernel contributors",
              "individual files carry SPDX identifiers (some dual-licensed); see COPYING and LICENSES/")
L_PG = lic("PostgreSQL", True, "PostgreSQL Global Development Group", "see COPYRIGHT")
L_LUA = lic("MIT", True, "Lua.org, PUC-Rio")
L_MIXED = lic("LicenseRef-mixed-per-package", False, "authors of each vendored/installed package",
              "aggregate of third-party packages under their own licenses (overwhelmingly permissive OSS, some copyleft "
              "possible); not audited package by package, so treated as non-redistributable: only hashes are committed")


def build_lic(base, extra):
    return lic(base["spdx_or_name"], base["redistributable"], base["attribution"],
               (base["notes"] + "; " if base["notes"] else "") + extra)


def build_recipe(params, inputs=None, from_items=None, git_=None, output_pin=None):
    r = {}
    if inputs:
        r["inputs"] = inputs
    if git_:
        r["git"] = git_
    if from_items:
        r["from_items"] = from_items
    r["generator"] = {"script": BUILD, "interpreter": "python", "seed": 0, "params": params}
    if output_pin is not None:
        r["output_pin"] = output_pin
    return r


# ============================================================================================
# F01 source-code repositories
# ============================================================================================
F01_TUN_ZSTD = add("f01-tuning-zstd-v1-5-7-git", "F01", "tuning", "small", "git-archive", "real",
                   {"git": git("zstd")}, L_ZSTD, "zstd",
                   "git archive of facebook/zstd at tag v1.5.7 (C library, CLI, tests, contrib, docs)")
F01_TUN_RG = add("f01-tuning-ripgrep-14-1-1-git", "F01", "tuning", "small", "git-archive", "real",
                 {"git": git("ripgrep")}, L_RG, "ripgrep",
                 "git archive of BurntSushi/ripgrep at tag 14.1.1 (Rust workspace with Cargo.lock)")
add("f01-tuning-curl-8-19-0-git", "F01", "tuning", "medium", "git-archive", "real", {"git": git("curl")}, L_CURL,
    "curl", "git archive of curl/curl at tag curl-8_19_0 (C library/tool, large test-case corpus, docs)")
add("f01-tuning-llvm-project-20-1-8-git", "F01", "tuning", "large", "git-archive", "real", {"git": git("llvm")},
    L_LLVM, "llvm-project",
    "git archive of the llvm/llvm-project monorepo at tag llvmorg-20.1.8 (LLVM, Clang, LLD, libc++, MLIR, tests)")

F01_VAL_REDIS = add("f01-validation-redis-7-2-16-git", "F01", "validation", "small", "git-archive", "real",
                    {"git": git("redis")}, L_REDIS, "redis",
                    "git archive of redis/redis at tag 7.2.16 (C server with bundled deps/ and Tcl test suite)")
F01_VAL_CPY = add("f01-validation-cpython-3-13-15-git", "F01", "validation", "medium", "git-archive", "real",
                  {"git": git("cpython")}, L_CPY, "cpython",
                  "git archive of python/cpython at tag v3.13.15 (C interpreter, stdlib, tests, docs)")

MUSL125 = "https://musl.libc.org/releases/musl-1.2.5.tar.gz"
LINUX_TAR = f"https://cdn.kernel.org/pub/linux/kernel/v6.x/linux-{U.LINUX_BASE}.tar.xz"
GO_TAR = f"https://go.dev/dl/{U.GO_SRC[0]}.src.tar.gz"
PG_TAR = f"https://ftp.postgresql.org/pub/source/v{U.POSTGRES}/postgresql-{U.POSTGRES}.tar.bz2"

add("f01-heldout-musl-1-2-5-src", "F01", "heldout", "small", "download", "real",
    {"inputs": [inp("musl", MUSL125)], "steps": [{"op": "extract", "input": "musl", "strip_components": 1}]},
    L_MUSL, "musl", "musl libc 1.2.5 release source tarball from musl.libc.org, extracted")
add("f01-heldout-go-1-25-0-src", "F01", "heldout", "medium", "download", "real",
    {"inputs": [inp("go-src", GO_TAR)], "steps": [{"op": "extract", "input": "go-src", "strip_components": 1}]},
    L_GO, "golang-go", "Go 1.25.0 source release tarball from go.dev/dl (toolchain and standard library), extracted")
add("f01-heldout-linux-6-6-src", "F01", "heldout", "large", "download", "real",
    {"inputs": [inp("linux", LINUX_TAR)], "steps": [{"op": "extract", "input": "linux", "strip_components": 1}]},
    L_LINUX, "linux-kernel", "Linux 6.6 release tarball from cdn.kernel.org, extracted (full source tree)")

# ============================================================================================
# F02 build trees
# ============================================================================================
add("f02-tuning-ripgrep-cargo-target", "F02", "tuning", "large", "build", "real",
    build_recipe({
        "source": {"from_item": F01_TUN_RG}, "env": dict(rust_env(), CARGO_TARGET_DIR="{build}/target"),
        "expect": rust_expect(), "jobs": JOBS,
        "commands": [{"argv": ["cargo", "fetch", "--locked"]},
                     {"argv": ["cargo", "build", "--locked", "-j", "{jobs}"]},
                     {"argv": ["cargo", "test", "--locked", "--no-run", "-j", "{jobs}"]},
                     {"argv": ["cargo", "build", "--locked", "--release", "-j", "{jobs}"]}],
        "capture": [{"from": "{build}/target", "to": "target"}]}, from_items=[F01_TUN_RG]),
    build_lic(L_RG, "build outputs; binaries statically include Rust std (MIT OR Apache-2.0) and the "
                    "crates.io dependencies of Cargo.lock (permissive)"),
    "ripgrep",
    "cargo target/ directory after `cargo build`, `cargo test --no-run` and `cargo build --release` of ripgrep 14.1.1 "
    "(rustc 1.98.1, Cargo.lock-pinned crates): debug+release profiles, incremental caches, rlibs, depfiles",
    notes="crates fetched from crates.io by cargo and verified against Cargo.lock checksums; unpinned output "
          "(build kind); built in fixed dir /root/eb-research/build/g1-code/<item_id>",
    tags=["toolchain-dependent", "network-fetch-lockfile-verified"])
add("f02-tuning-zstd-cmake-build", "F02", "tuning", "medium", "build", "real",
    build_recipe({
        "source": {"from_item": F01_TUN_ZSTD}, "env": {"CC": "gcc", "CXX": "g++"},
        "expect": gcc_expect() + [[["cmake", "--version"], "cmake version 3.28.3"]], "jobs": JOBS,
        "commands": [{"argv": ["cmake", "-S", "{src}/build/cmake", "-B", "{build}", "-G", "Unix Makefiles",
                               "-DCMAKE_BUILD_TYPE=RelWithDebInfo", "-DZSTD_BUILD_TESTS=ON", "-DZSTD_BUILD_CONTRIB=ON",
                               "-DZSTD_BUILD_STATIC=ON", "-DZSTD_BUILD_SHARED=ON", "-DZSTD_MULTITHREAD_SUPPORT=ON"]},
                     {"argv": ["cmake", "--build", "{build}", "-j", "{jobs}"]}],
        "capture": [{"from": "{build}", "to": "build"}]}, from_items=[F01_TUN_ZSTD]),
    build_lic(L_ZSTD, "build outputs (objects, static/shared libraries, programs, tests, contrib tools)"), "zstd",
    "CMake (Unix Makefiles) RelWithDebInfo build directory of zstd v1.5.7 with programs, tests and contrib enabled "
    "(gcc 13.3.0)",
    notes="unpinned output (build kind)", tags=["toolchain-dependent"])

LZ4_110 = "https://codeload.github.com/lz4/lz4/tar.gz/refs/tags/v1.10.0"
add("f02-tuning-lz4-intree-make-build", "F02", "tuning", "small", "build", "real",
    build_recipe({
        "source": {"input": "lz4", "strip_components": 1}, "env": {"CC": "gcc"}, "expect": gcc_expect(),
        "jobs": JOBS,
        "commands": [{"argv": ["make", "-j", "{jobs}"], "cwd": "{src}"}],
        "capture": [{"from": "{src}", "to": ""}]}, inputs=[inp("lz4", LZ4_110)]),
    build_lic(L_LZ4, "build outputs"), "lz4",
    "in-tree `make` (default target: lib-release + lz4-release) of the lz4 v1.10.0 tag archive: the working tree "
    "after the build (library objects, static and shared liblz4, lz4 CLI); gcc 13.3.0",
    notes="in-tree build, so the tree also contains the lz4 sources; `make all` (adds tests/examples) measured "
          "17.4 MiB on the first attempt and was replaced by the default target to keep a small-tier F02 item; "
          "unpinned output (build kind)",
    tags=["toolchain-dependent", "in-tree-build"])

CJSON_1719 = "https://codeload.github.com/DaveGamble/cJSON/tar.gz/refs/tags/v1.7.19"
add("f02-validation-cjson-cmake-build", "F02", "validation", "small", "build", "real",
    build_recipe({
        "source": {"input": "cjson", "strip_components": 1}, "env": {"CC": "gcc"},
        "expect": gcc_expect() + [[["cmake", "--version"], "cmake version 3.28.3"]], "jobs": JOBS,
        "commands": [{"argv": ["cmake", "-S", "{src}", "-B", "{build}", "-G", "Unix Makefiles",
                               "-DCMAKE_BUILD_TYPE=Debug", "-DENABLE_CJSON_TEST=ON", "-DENABLE_CJSON_UTILS=ON",
                               "-DBUILD_SHARED_AND_STATIC_LIBS=ON"]},
                     {"argv": ["cmake", "--build", "{build}", "-j", "{jobs}"]}],
        "capture": [{"from": "{build}", "to": "build"}]}, inputs=[inp("cjson", CJSON_1719)]),
    build_lic(L_CJSON, "build outputs"), "cjson",
    "CMake (Unix Makefiles) Debug build directory of the cJSON v1.7.19 tag archive with cJSON_Utils and the unit tests "
    "enabled; gcc 13.3.0",
    notes="unpinned output (build kind)", tags=["toolchain-dependent"])

add("f02-validation-cpython-build", "F02", "validation", "medium", "build", "real",
    build_recipe({
        "source": {"from_item": F01_VAL_CPY}, "env": {"CC": "gcc"}, "expect": gcc_expect(), "jobs": JOBS,
        "commands": [{"argv": ["{src}/configure", "--prefix=/opt/ebrc/python3.13", "--with-ensurepip=no"],
                      "cwd": "{build}"},
                     {"argv": ["make", "-j", "{jobs}"], "cwd": "{build}"}],
        "capture": [{"from": "{build}", "to": "build"}]}, from_items=[F01_VAL_CPY]),
    build_lic(L_CPY, "build outputs; extension modules link against Ubuntu 24.04 system libraries"), "cpython",
    "out-of-tree `configure && make` build directory of CPython 3.13.15 (objects, libpython3.13.a, python binary, "
    "shared extension modules, generated sources); gcc 13.3.0 with the dev packages of setup_build_deps.sh",
    notes="optional modules depend on installed -dev packages (research/corpus/generators/g1-code/setup_build_deps.sh);"
          " unpinned output (build kind)", tags=["toolchain-dependent", "system-libraries"])
add("f02-validation-redis-intree-build", "F02", "validation", "medium", "build", "real",
    build_recipe({
        "source": {"from_item": F01_VAL_REDIS}, "env": {"CC": "gcc"}, "expect": gcc_expect(), "jobs": JOBS,
        "commands": [{"argv": ["make", "-j", "{jobs}", "BUILD_TLS=no", "V=1"], "cwd": "{src}"}],
        "capture": [{"from": "{src}", "to": ""}]}, from_items=[F01_VAL_REDIS]),
    build_lic(L_REDIS, "build outputs"), "redis",
    "in-tree `make` of redis 7.2.16: the whole working tree after the build (sources interleaved with objects, "
    "bundled jemalloc/lua/hiredis builds, redis-server/cli/benchmark binaries); gcc 13.3.0",
    notes="in-tree build, so the tree also contains the F01 validation redis sources; unpinned output (build kind)",
    tags=["toolchain-dependent", "in-tree-build"])

add("f02-heldout-postgresql-17-6-build", "F02", "heldout", "medium", "build", "real",
    build_recipe({
        "source": {"input": "postgresql", "strip_components": 1}, "env": {"CC": "gcc"},
        "expect": gcc_expect(), "jobs": JOBS,
        "commands": [{"argv": ["{src}/configure", "--prefix=/opt/ebrc/pgsql", "--enable-debug", "--with-openssl",
                               "--with-icu"], "cwd": "{build}"},
                     {"argv": ["make", "-j", "{jobs}", "world-bin"], "cwd": "{build}"}],
        "capture": [{"from": "{build}", "to": "build"}]}, inputs=[inp("postgresql", PG_TAR)]),
    build_lic(L_PG, "build outputs"), "postgresql",
    "out-of-tree (VPATH) `configure --enable-debug && make world-bin` build directory of the PostgreSQL 17.6 release "
    "tarball; gcc 13.3.0",
    notes="unpinned output (build kind)", tags=["toolchain-dependent", "system-libraries"])
add("f02-heldout-musl-1-2-5-build", "F02", "heldout", "small", "build", "real",
    build_recipe({
        "source": {"input": "musl", "strip_components": 1}, "env": {"CC": "gcc"}, "expect": gcc_expect(),
        "jobs": JOBS,
        "commands": [{"argv": ["{src}/configure", "--prefix=/opt/ebrc/musl"], "cwd": "{build}"},
                     {"argv": ["make", "-j", "{jobs}"], "cwd": "{build}"}],
        "capture": [{"from": "{build}", "to": "build"}]}, inputs=[inp("musl", MUSL125)]),
    build_lic(L_MUSL, "build outputs"), "musl",
    "out-of-tree `configure && make` build directory of the musl 1.2.5 release tarball; gcc 13.3.0",
    notes="unpinned output (build kind)", tags=["toolchain-dependent"])
add("f02-heldout-linux-6-6-defconfig-build", "F02", "heldout", "large", "build", "real",
    build_recipe({
        "source": {"input": "linux", "strip_components": 1},
        "env": {"KBUILD_BUILD_TIMESTAMP": "Thu Jan  1 00:00:00 UTC 2026", "KBUILD_BUILD_USER": "ebrc",
                "KBUILD_BUILD_HOST": "ebrc"},
        "expect": gcc_expect(), "jobs": JOBS,
        "commands": [{"argv": ["make", "-C", "{src}", "O={build}", "x86_64_defconfig"]},
                     {"argv": ["make", "-C", "{src}", "O={build}", "-j", "{jobs}", "all"]}],
        "capture": [{"from": "{build}", "to": "build"}]}, inputs=[inp("linux", LINUX_TAR)]),
    build_lic(L_LINUX, "build outputs (objects, vmlinux, bzImage, modules)"), "linux-kernel",
    "out-of-tree (O=) `make x86_64_defconfig && make all` build directory of the Linux 6.6 release tarball; gcc 13.3.0",
    notes="unpinned output (build kind)", tags=["toolchain-dependent"])

# ============================================================================================
# F03 dependency/vendor trees
# ============================================================================================
add("f03-tuning-ripgrep-cargo-vendor", "F03", "tuning", "medium", "build", "real",
    build_recipe({
        "source": {"from_item": F01_TUN_RG}, "env": rust_env(), "expect": rust_expect(),
        "commands": [{"argv": ["cargo", "vendor", "--locked", "--versioned-dirs", "{build}/vendor"]}],
        "capture": [{"from": "{build}/vendor", "to": "vendor"}]}, from_items=[F01_TUN_RG], output_pin="TOFU"),
    L_MIXED, "ripgrep",
    "`cargo vendor --locked --versioned-dirs` of ripgrep 14.1.1's Cargo.lock: vendor/ with one directory per crate "
    "(sources, .cargo-checksum.json)",
    notes="crates downloaded from crates.io by cargo 1.98.1 and verified against Cargo.lock checksums; TOFU-pinned "
          "output (deterministic)", tags=["network-fetch-lockfile-verified"])
TS = U.TYPESCRIPT_COMMIT[0]
add("f03-tuning-typescript-npm-node-modules", "F03", "tuning", "medium", "build", "real",
    build_recipe({
        "source": {"files": [{"input": "package-json", "dest": "package.json"},
                             {"input": "package-lock", "dest": "package-lock.json"}]},
        "env": npm_env(), "expect": [[["node", "--version"], NODE], [["npm", "--version"], NPM]],
        "commands": [{"argv": ["npm", "ci", "--ignore-scripts", "--no-audit", "--no-fund", "--loglevel=warn"]}],
        "capture": [{"from": "{src}/node_modules", "to": "node_modules"}]},
        inputs=[inp("package-json", U.gh_raw("microsoft/TypeScript", TS, "package.json")),
                inp("package-lock", U.gh_raw("microsoft/TypeScript", TS, "package-lock.json"))], output_pin="TOFU"),
    L_MIXED, "microsoft-typescript",
    "node_modules/ produced by `npm ci --ignore-scripts` from microsoft/TypeScript v5.9.3 package.json + "
    "package-lock.json (the TypeScript compiler repository's development dependencies)",
    notes="packages downloaded from registry.npmjs.org by npm 10.9.8 (node v22.23.2) and verified against lockfile "
          "integrity hashes; install scripts not run; TOFU-pinned output",
    tags=["network-fetch-lockfile-verified"])
LOCK_SCI = "research/corpus/generators/g1-code/locks/pypi-scientific-stack.txt"
LOCK_WEB = "research/corpus/generators/g1-code/locks/pypi-web-service-stack.txt"


def pip_params(lock_rel, lock_sha):
    return {"source": {"none": True}, "files_sha256": {lock_rel: lock_sha},
            "expect": [[["{python}", "--version"], PY], [["{python}", "-m", "pip", "--version"], PIP]],
            "commands": [{"argv": ["{python}", "-m", "pip", "install", "--isolated", "--no-deps", "--require-hashes",
                                   "--only-binary=:all:", "--disable-pip-version-check", "--no-warn-script-location",
                                   "--cache-dir", "{cache}/pip-cache", "--target", "{build}/site-packages",
                                   "-r", "{repo}/" + lock_rel]}],
            "capture": [{"from": "{build}/site-packages", "to": "site-packages"}]}


def sha256_of(rel):
    import hashlib
    return hashlib.sha256((REPO / rel).read_bytes()).hexdigest()


add("f03-tuning-pypi-scientific-site-packages", "F03", "tuning", "medium", "build", "real",
    build_recipe(pip_params(LOCK_SCI, sha256_of(LOCK_SCI))), L_MIXED, "pypi-scientific-stack",
    "`pip install --target` (CPython 3.12, manylinux x86_64 wheels, with __pycache__) of a hash-locked scientific "
    "Python stack: numpy, scipy, pandas, matplotlib, scikit-learn, sympy, pillow, networkx, seaborn and dependencies "
    "(22 wheels)",
    notes="lock generated by authoring/make_pip_lock.py from PyPI on 2026-09-12; wheels verified by --require-hashes; "
          "top-level selection curated (not a single upstream project's lock); unpinned output (pyc marshal output "
          "is not guaranteed byte-stable)", tags=["network-fetch-lockfile-verified", "curated-selection"])

add("f03-validation-bat-cargo-vendor", "F03", "validation", "large", "build", "real",
    build_recipe({
        "source": {"git_archive": True}, "env": rust_env(), "expect": rust_expect(),
        "commands": [{"argv": ["cargo", "vendor", "--locked", "--versioned-dirs", "{build}/vendor"]}],
        "capture": [{"from": "{build}/vendor", "to": "vendor"}]}, git_=git("bat"), output_pin="TOFU"),
    L_MIXED, "sharkdp-bat",
    "`cargo vendor --locked --versioned-dirs` of sharkdp/bat v0.25.0's Cargo.lock (git archive at the tag)",
    notes="crates downloaded from crates.io by cargo 1.98.1 and verified against Cargo.lock checksums; TOFU-pinned",
    tags=["network-fetch-lockfile-verified"])
BS = U.BOOTSTRAP_COMMIT[0]
add("f03-validation-bootstrap-npm-node-modules", "F03", "validation", "medium", "build", "real",
    build_recipe({
        "source": {"files": [{"input": "package-json", "dest": "package.json"},
                             {"input": "package-lock", "dest": "package-lock.json"}]},
        "env": npm_env(), "expect": [[["node", "--version"], NODE], [["npm", "--version"], NPM]],
        "commands": [{"argv": ["npm", "ci", "--ignore-scripts", "--no-audit", "--no-fund", "--loglevel=warn"]}],
        "capture": [{"from": "{src}/node_modules", "to": "node_modules"}]},
        inputs=[inp("package-json", U.gh_raw("twbs/bootstrap", BS, "package.json")),
                inp("package-lock", U.gh_raw("twbs/bootstrap", BS, "package-lock.json"))], output_pin="TOFU"),
    L_MIXED, "twbs-bootstrap",
    "node_modules/ produced by `npm ci --ignore-scripts` from twbs/bootstrap v5.3.8 package.json + package-lock.json",
    notes="packages from registry.npmjs.org verified against lockfile integrity hashes (npm 10.9.8, node v22.23.2); "
          "install scripts not run; TOFU-pinned", tags=["network-fetch-lockfile-verified"])

add("f03-heldout-alacritty-cargo-vendor", "F03", "heldout", "medium", "build", "real",
    build_recipe({
        "source": {"git_archive": True}, "env": rust_env(), "expect": rust_expect(),
        "commands": [{"argv": ["cargo", "vendor", "--locked", "--versioned-dirs", "{build}/vendor"]}],
        "capture": [{"from": "{build}/vendor", "to": "vendor"}]}, git_=git("alacritty"), output_pin="TOFU"),
    L_MIXED, "alacritty",
    "`cargo vendor --locked --versioned-dirs` of alacritty/alacritty v0.15.1's Cargo.lock (git archive at the tag)",
    notes="crates from crates.io verified against Cargo.lock checksums (cargo 1.98.1); TOFU-pinned",
    tags=["network-fetch-lockfile-verified"])
PJ = U.PDFJS_COMMIT[0]
add("f03-heldout-pdfjs-npm-node-modules", "F03", "heldout", "medium", "build", "real",
    build_recipe({
        "source": {"files": [{"input": "package-json", "dest": "package.json"},
                             {"input": "package-lock", "dest": "package-lock.json"}]},
        "env": npm_env(), "expect": [[["node", "--version"], NODE], [["npm", "--version"], NPM]],
        "commands": [{"argv": ["npm", "ci", "--ignore-scripts", "--no-audit", "--no-fund", "--loglevel=warn"]}],
        "capture": [{"from": "{src}/node_modules", "to": "node_modules"}]},
        inputs=[inp("package-json", U.gh_raw("mozilla/pdf.js", PJ, "package.json")),
                inp("package-lock", U.gh_raw("mozilla/pdf.js", PJ, "package-lock.json"))], output_pin="TOFU"),
    L_MIXED, "mozilla-pdfjs",
    "node_modules/ produced by `npm ci --ignore-scripts` from mozilla/pdf.js v4.10.38 package.json + package-lock.json",
    notes="packages from registry.npmjs.org verified against lockfile integrity hashes (npm 10.9.8, node v22.23.2); "
          "install scripts not run; TOFU-pinned", tags=["network-fetch-lockfile-verified"])
add("f03-heldout-pypi-web-site-packages", "F03", "heldout", "medium", "build", "real",
    build_recipe(pip_params(LOCK_WEB, sha256_of(LOCK_WEB))), L_MIXED, "pypi-web-service-stack",
    "`pip install --target` (CPython 3.12, manylinux x86_64 wheels, with __pycache__) of a hash-locked PyPI "
    "web-service requirement set (61 wheels; lock file research/corpus/generators/g1-code/locks/"
    "pypi-web-service-stack.txt)",
    notes="lock generated by authoring/make_pip_lock.py from PyPI on 2026-09-12; wheels verified by --require-hashes; "
          "curated top-level selection; shares a few ubiquitous small wheels (packaging, six, python-dateutil) with the "
          "tuning scientific stack; unpinned output", tags=["network-fetch-lockfile-verified", "curated-selection"])

L_MIT_GO = "MIT"


def go_vendor(item_id, split, key, group, project, version, attribution, scale="small"):
    add(item_id, "F03", split, scale, "build", "real",
        build_recipe({
            "source": {"git_archive": True},
            "env": {"GOTOOLCHAIN": "local", "GOPATH": "{cache}/gopath", "GOMODCACHE": "{cache}/go-mod-cache",
                    "GOCACHE": "{cache}/go-build-cache", "GOPROXY": "https://proxy.golang.org",
                    "GOSUMDB": "sum.golang.org", "GONOSUMDB": "", "GOPRIVATE": "",
                    "GOTELEMETRY": "off"},
            "expect": [[["go", "version"], "go version go1.22.2 linux/amd64"]],
            "commands": [{"argv": ["go", "mod", "vendor", "-o", "{build}/vendor"]}],
            "capture": [{"from": "{build}/vendor", "to": "vendor"}]}, git_=git(key), output_pin="TOFU"),
        L_MIXED, group,
        f"`go mod vendor` of {project} {version} (git archive at the tag): vendor/ with modules.txt and the imported "
        f"packages of every dependency module",
        notes=f"project itself is {L_MIT_GO} ({attribution}); modules downloaded via proxy.golang.org by Go 1.22.2 "
              "(Ubuntu golang-1.22, GOTOOLCHAIN=local) and verified against go.sum and sum.golang.org; TOFU-pinned",
        tags=["network-fetch-lockfile-verified", "go-modules"])


go_vendor("f03-tuning-fzf-go-mod-vendor", "tuning", "fzf", "junegunn-fzf", "junegunn/fzf", "0.46.1",
          "Junegunn Choi")
go_vendor("f03-validation-yq-go-mod-vendor", "validation", "yq", "mikefarah-yq", "mikefarah/yq", "v4.40.5",
          "Mike Farah", scale="medium")
go_vendor("f03-heldout-direnv-go-mod-vendor", "heldout", "direnv", "direnv", "direnv/direnv", "v2.34.0",
          "direnv contributors")

# ============================================================================================
# F18 near-duplicate / versioned trees
# ============================================================================================


def multi_extract(prefix, urls_names):
    inputs, steps = [], []
    for name, url, dest, strip in urls_names:
        inputs.append(inp(name, url))
        st = {"op": "extract", "input": name}
        if dest:
            st["dest"] = dest
        if strip:
            st["strip_components"] = strip
        steps.append(st)
    return {"inputs": inputs, "steps": steps}


add("f18-tuning-zstd-releases-1-5-x", "F18", "tuning", "medium", "download", "real",
    multi_extract("zstd", [(f"v{v.replace('.', '_')}",
                            f"https://github.com/facebook/zstd/releases/download/v{v}/zstd-{v}.tar.gz", None, 0)
                           for v in U.ZSTD_RELEASES]),
    L_ZSTD, "zstd",
    "seven successive zstd release source tarballs (1.5.0, 1.5.1, 1.5.2, 1.5.4, 1.5.5, 1.5.6, 1.5.7) extracted side "
    "by side as zstd-<version>/ (upstream skipped 1.5.3)")
add("f18-tuning-sqlite-amalgamation-series", "F18", "tuning", "medium", "download", "real",
    multi_extract("sqlite", [(f"sqlite-{n}", f"https://www.sqlite.org/{y}/sqlite-amalgamation-{n}.zip", None, 0)
                             for y, n in U.SQLITE_AMALGAMATIONS]),
    L_SQLITE, "sqlite",
    "fourteen successive SQLite amalgamation zips (3.45.0 through 3.50.0) extracted side by side as "
    "sqlite-amalgamation-<n>/ (few large, nearly identical C files per version)",
    notes="sqlite.org publishes SHA3-256 only for current downloads; SHA-256 recorded from the 2026-09-12 retrieval")
add("f18-tuning-lz4-releases-1-9-to-1-10", "F18", "tuning", "small", "download", "real",
    multi_extract("lz4", [(f"lz4-{t[1:].replace('.', '_')}", f"https://codeload.github.com/lz4/lz4/tar.gz/refs/tags/{t}",
                           None, 0) for t in U.LZ4_TAGS]),
    L_LZ4, "lz4",
    "six successive lz4 release tag archives (v1.9.0, v1.9.1, v1.9.2, v1.9.3, v1.9.4, v1.10.0) from GitHub codeload, "
    "extracted side by side as lz4-<version>/",
    notes="GitHub-generated tag archives; SHA-256 recorded from the 2026-09-12 retrieval")
add("f18-tuning-ripgrep-commit-snapshots", "F18", "tuning", "medium", "download", "real",
    multi_extract("rg", [(f"c{i:02d}", f"https://codeload.github.com/BurntSushi/ripgrep/tar.gz/{c}",
                          f"{i:02d}-{c[:12]}", 1) for i, c in enumerate(U.RIPGREP_COMMITS, 1)]),
    L_RG, "ripgrep",
    "sixteen consecutive first-parent commits of BurntSushi/ripgrep (2057023dc5eb..e50df40a1967, ending at tag "
    "14.1.0) as full source snapshots NN-<sha12>/ (small edit distance history)",
    notes="GitHub codeload commit archives; SHA-256 recorded from the 2026-09-12 retrieval")

add("f18-tuning-curl-8-x-release-series", "F18", "tuning", "large", "download", "real",
    multi_extract("curl", [(f"curl-{v.replace('.', '_')}", f"https://curl.se/download/curl-{v}.tar.xz", None, 0)
                           for v in U.CURL_RELEASES]),
    L_CURL, "curl",
    f"thirty successive curl release tarballs ({U.CURL_RELEASES[0]} through {U.CURL_RELEASES[-1]}, every 8.x release "
    "including patch releases) from curl.se, extracted side by side as curl-<version>/",
    notes="curl.se publishes GPG signatures, not SHA-256; SHA-256 recorded from the 2026-09-12 retrieval")

add("f18-validation-cpython-3-12-point-releases", "F18", "validation", "large", "download", "real",
    multi_extract("py", [(f"python-{v.replace('.', '_')}", f"https://www.python.org/ftp/python/{v}/Python-{v}.tar.xz",
                          None, 0) for v in U.PYTHON312]),
    L_CPY, "cpython",
    "six successive CPython point-release source tarballs (3.12.0 through 3.12.5) from python.org, extracted side by "
    "side as Python-<version>/")
add("f18-validation-redis-7-2-point-releases", "F18", "validation", "medium", "download", "real",
    multi_extract("redis", [(f"redis-{v.replace('.', '_')}", f"https://download.redis.io/releases/redis-{v}.tar.gz",
                             None, 0) for v in U.REDIS72]),
    L_REDIS, "redis",
    "ten successive redis release tarballs (7.2.0 through 7.2.9) from download.redis.io, extracted side by side as "
    "redis-<version>/")
add("f18-validation-cjson-releases", "F18", "validation", "small", "download", "real",
    multi_extract("cjson", [(f"cjson-{t[1:].replace('.', '_')}",
                             f"https://codeload.github.com/DaveGamble/cJSON/tar.gz/refs/tags/{t}", None, 0)
                            for t in U.CJSON_TAGS]),
    L_CJSON, "cjson",
    "eight successive cJSON release tag archives (v1.7.12 through v1.7.19) from GitHub codeload, extracted side by "
    "side as cJSON-<version>/", notes="GitHub-generated tag archives; SHA-256 recorded from the 2026-09-12 retrieval")

add("f18-heldout-musl-releases-1-2-x", "F18", "heldout", "medium", "download", "real",
    multi_extract("musl", [(f"musl-{v.replace('.', '_')}", f"https://musl.libc.org/releases/musl-{v}.tar.gz", None, 0)
                           for v in U.MUSL_RELEASES]),
    L_MUSL, "musl",
    "six successive musl release tarballs (1.2.0 through 1.2.5) from musl.libc.org, extracted side by side",
    notes="musl publishes GPG signatures, not SHA-256; SHA-256 recorded from the 2026-09-12 retrieval")
add("f18-heldout-lua-5-4-releases", "F18", "heldout", "small", "download", "real",
    multi_extract("lua", [(f"lua-{v.replace('.', '_')}", f"https://www.lua.org/ftp/lua-{v}.tar.gz", None, 0)
                          for v in U.LUA_RELEASES]),
    L_LUA, "lua",
    "nine successive Lua release tarballs (5.4.0 through 5.4.8) from lua.org, extracted side by side")
lin_inputs = [inp("base", LINUX_TAR)] + [
    inp(f"patch-{n}", f"https://cdn.kernel.org/pub/linux/kernel/v6.x/patch-{U.LINUX_BASE}.{n}.xz")
    for n in U.LINUX_PATCHES]
LINUX_SUBSET = ["block", "crypto", "fs", "include", "init", "io_uring", "ipc", "kernel", "lib", "mm", "net",
                "security", "virt"]
add("f18-heldout-linux-6-6-stable-subset-series", "F18", "heldout", "large", "download", "real",
    {"inputs": lin_inputs,
     "steps": [{"op": "run", "script": f"{GEN}/linux_stable_series.py", "interpreter": "python", "seed": 0,
                "params": {"base_topdir": f"linux-{U.LINUX_BASE}", "subset": LINUX_SUBSET,
                           "snapshots": [0] + U.LINUX_PATCHES, "name_format": f"linux-{U.LINUX_BASE}.{{n}}"}}]},
    L_LINUX, "linux-kernel",
    "Linux 6.6, 6.6.30, 6.6.60 and 6.6.90 side by side, restricted to the core top-level directories "
    f"({', '.join(LINUX_SUBSET)}); stable trees reconstructed from the 6.6 tarball plus the official cumulative "
    "patch-6.6.N.xz files from cdn.kernel.org",
    notes="byte-exact subsets of the upstream stable trees (git apply, no fuzz)")

doc = {
    "schema": "ebrc-sources-v1",
    "notes": ("g1-code group: F01 source-code repositories, F02 build trees, F03 dependency/vendor trees, F18 "
              "near-duplicate/versioned trees. Generated by research/corpus/generators/g1-code/authoring/"
              "make_sources.py from upstreams.py and downloads.lock.json (retrieval date 2026-09-12). Split upstreams "
              "are disjoint: tuning = zstd, ripgrep, curl, llvm-project, sqlite, lz4, TypeScript devDeps, PyPI "
              "scientific stack, fzf; validation = CPython, redis, bat, bootstrap, cJSON, yq; heldout = Linux 6.6, Go, "
              "musl, PostgreSQL, Lua, alacritty, pdf.js, PyPI web-service stack, direnv. Build and vendor recipes need "
              "setup_build_deps.sh (apt) and the pinned toolchains asserted in their params. Vendor trees from "
              "different splits unavoidably share some ubiquitous third-party packages (e.g. common crates or npm "
              "packages)."),
    "items": items,
}
out = REPO / "research" / "corpus" / "sources" / "g1-code.json"
out.parent.mkdir(parents=True, exist_ok=True)
out.write_text(json.dumps(doc, indent=2, ensure_ascii=False) + "\n", encoding="utf-8", newline="\n")
print(f"wrote {out} with {len(items)} items")
