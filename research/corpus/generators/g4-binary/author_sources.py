#!/usr/bin/env python3
"""Authoring aid for research/corpus/sources/g4-binary.json (families F09 F10 F11 F14 F16).

NOT used by provision.  Run once (inside WSL) while authoring:
    /root/eb-research/venv/bin/python -B author_sources.py [--plan] [--write]
It resolves package-index selections (Ubuntu/Debian Packages, Fedora primary.xml, Arch package API,
PyPI JSON, Maven metadata, F-Droid index-v2, Alpine APKINDEX, OpenWrt sha256sums) into pinned
input URLs with published SHA-256 digests (or "TOFU" where upstream publishes none) and writes the
sources JSON.  The committed JSON is authoritative; re-running this script later may resolve newer
versions and must not silently replace a committed definition (compare, then decide).
Retrieval date of all resolutions: 2026-09-12.

Upstream split plan (no upstream repository/URL/docker repo crosses splits, and held-out groups are
never used in tuning/validation by this group; see notes in the JSON):
  tuning:     Ubuntu 24.04, Alpine 3.24, Silesia corpus (public benchmark), linux-firmware, fd-find (Rust),
              PyPI binary wheels, Apache Maven, Jenkins, generated (py) nested archives / redundant blobs
  validation: Fedora 44, Debian 13 (edk2 firmware, nocloud image), CPython Windows embeddable, Git for Windows,
              Maven Central JVM jars, Apache Spark (pyspark sdist), generated office documents, generated FAT image
  heldout:    AlmaLinux 10, PowerShell, uutils coreutils (macOS), OpenWrt, QuickJS, Arch Linux packages,
              X.org release tarballs, F-Droid APKs, Firefox, FreeBSD 15.1, generated (cli) nested archives,
              generated GPT disk
"""

from __future__ import annotations

import argparse
import gzip
import hashlib
import io
import json
import lzma
import os
import re
import subprocess
import sys
import tarfile
import urllib.parse
import urllib.request
import xml.etree.ElementTree as ET
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[3]
OUT_JSON = REPO / "research" / "corpus" / "sources" / "g4-binary.json"
CACHE = Path("/root/eb-research/scratch/g4-binary-authoring")
UA = {"User-Agent": "entrybound-research-corpus-authoring/1"}
GEN = "research/corpus/generators/g4-binary"
SAFE = re.compile(r"[^A-Za-z0-9._+-]")


# ---------------------------------------------------------------------------------------------
# http helpers with an authoring cache

def fetch(url: str, binary: bool = True, cache: bool = True):
    CACHE.mkdir(parents=True, exist_ok=True)
    p = CACHE / hashlib.sha256(url.encode()).hexdigest()
    if cache and p.exists():
        data = p.read_bytes()
    else:
        req = urllib.request.Request(url, headers=UA)
        with urllib.request.urlopen(req, timeout=300) as r:
            data = r.read()
        if cache:
            p.write_bytes(data)
    return data if binary else data.decode("utf-8", "replace")


def head_size(url: str):
    key = CACHE / ("head-" + hashlib.sha256(url.encode()).hexdigest())
    if key.exists():
        v = key.read_text()
        return int(v) if v.lstrip("-").isdigit() else None
    req = urllib.request.Request(url, headers=UA, method="HEAD")
    try:
        with urllib.request.urlopen(req, timeout=120) as r:
            n = int(r.headers.get("Content-Length") or -1)
    except Exception:  # noqa: BLE001
        n = -1
    CACHE.mkdir(parents=True, exist_ok=True)
    key.write_text(str(n))
    return n if n >= 0 else None


def exists(url: str) -> bool:
    return head_size(url) is not None


def inp(name, url, sha256="TOFU", size=None, filename=None, notes=None):
    name = re.sub(r"[^a-z0-9_-]", "-", name.lower()).strip("-")[:64]
    d = {"name": name, "url": url, "sha256": sha256}
    if size is not None and size >= 0:
        d["size"] = size
    base = os.path.basename(urllib.parse.unquote(urllib.parse.urlparse(url).path))
    fn = filename or (SAFE.sub("_", base) if SAFE.search(base) else None)
    if fn:
        d["filename"] = fn
    if notes:
        d["notes"] = notes
    return d


def dedupe_names(inputs):
    seen = {}
    for d in inputs:
        n = d["name"]
        if n in seen:
            seen[n] += 1
            d["name"] = f"{n[:60]}-{seen[n]}"
        else:
            seen[n] = 0
    return inputs


# ---------------------------------------------------------------------------------------------
# resolvers

def parse_deb822(text):
    out = {}
    for block in text.split("\n\n"):
        rec, key = {}, None
        for ln in block.splitlines():
            if ln.startswith((" ", "\t")):
                continue
            if ":" in ln:
                key, val = ln.split(":", 1)
                rec[key] = val.strip()
        if "Package" in rec:
            out[rec["Package"]] = rec
    return out


def deb_packages(names, index_url, base_url, decompress):
    idx = parse_deb822(decompress(fetch(index_url)).decode())
    res = []
    for n in names:
        p = idx.get(n)
        if not p:
            raise SystemExit(f"package {n} not in {index_url}")
        res.append(inp(n, base_url + p["Filename"], p["SHA256"], int(p["Size"]),
                       notes=f"{n} {p['Version']} (SHA-256 from {index_url})"))
    return res


def ubuntu_debs(names):
    return deb_packages(names, "https://archive.ubuntu.com/ubuntu/dists/noble/main/binary-amd64/Packages.gz",
                        "https://archive.ubuntu.com/ubuntu/", gzip.decompress)


def debian_debs(names):
    return deb_packages(names, "https://deb.debian.org/debian/dists/trixie/main/binary-amd64/Packages.xz",
                        "https://deb.debian.org/debian/", lzma.decompress)


def fedora_rpms(names, release=44):
    base = f"https://dl.fedoraproject.org/pub/fedora/linux/releases/{release}/Everything/x86_64/os/"
    repomd = ET.fromstring(fetch(base + "repodata/repomd.xml"))
    ns = {"r": "http://linux.duke.edu/metadata/repo"}
    href = None
    for data in repomd.findall("r:data", ns):
        if data.get("type") == "primary":
            href = data.find("r:location", ns).get("href")
    raw = fetch(base + href)
    if href.endswith(".zst"):
        raw = subprocess.run(["zstd", "-dcq"], input=raw, capture_output=True, check=True).stdout
    elif href.endswith(".gz"):
        raw = gzip.decompress(raw)
    elif href.endswith(".xz"):
        raw = lzma.decompress(raw)
    cache = CACHE / f"fedora{release}-primary-index.json"
    if cache.exists():
        pk = json.loads(cache.read_text())
    else:
        pk = {}
        c = "{http://linux.duke.edu/metadata/common}"
        for _, el in ET.iterparse(io.BytesIO(raw)):
            if el.tag == c + "package":
                name = el.find(c + "name").text
                arch = el.find(c + "arch").text
                if arch in ("x86_64", "noarch"):
                    v = el.find(c + "version")
                    pk[name] = {"href": el.find(c + "location").get("href"),
                                "sha256": el.find(c + "checksum").text,
                                "size": int(el.find(c + "size").get("package")),
                                "evr": f"{v.get('epoch')}:{v.get('ver')}-{v.get('rel')}", "arch": arch}
                el.clear()
        cache.write_text(json.dumps(pk))
    res = []
    for n in names:
        p = pk.get(n)
        if not p:
            raise SystemExit(f"fedora package {n} not found")
        res.append(inp(n, base + p["href"], p["sha256"], p["size"],
                       notes=f"{n} {p['evr']}.{p['arch']} (SHA-256 from Fedora {release} primary.xml)"))
    return res


def arch_pkgs(names):
    res = []
    for n in names:
        d = json.loads(fetch(f"https://archlinux.org/packages/search/json/?name={urllib.parse.quote(n)}", False))
        r = [x for x in d["results"] if x["pkgname"] == n and x["arch"] in ("x86_64", "any")]
        if not r:
            raise SystemExit(f"arch package {n} not found")
        r = r[0]
        url = f"https://archive.archlinux.org/packages/{n[0]}/{n}/{urllib.parse.quote(r['filename'])}"
        res.append(inp(n, url, "TOFU", r["compressed_size"],
                       notes=f"{n} {r['epoch']}:{r['pkgver']}-{r['pkgrel']} ({r['repo']}); Arch publishes signatures "
                             f"only, so SHA-256 is TOFU-pinned"))
    return res


def pypi_wheels(names):
    res = []
    for n in names:
        d = json.loads(fetch(f"https://pypi.org/pypi/{n}/json", False))
        files = [u for u in d["urls"] if u["packagetype"] == "bdist_wheel" and "x86_64" in u["filename"]
                 and "manylinux" in u["filename"] and "musllinux" not in u["filename"]]
        pref = [f for f in files if "-cp312-cp312-" in f["filename"]] or \
               [f for f in files if "abi3" in f["filename"]] or [f for f in files if "-py3-none-" in f["filename"]]
        if not pref:
            raise SystemExit(f"no cp312 manylinux x86_64 wheel for {n}: {[f['filename'] for f in files]}")
        f = sorted(pref, key=lambda f: f["filename"])[-1]
        res.append(inp(n, f["url"], f["digests"]["sha256"], f["size"],
                       notes=f"{n} {d['info']['version']} {f['filename']} (SHA-256 from PyPI JSON API)"))
    return res


def maven_jars(coords):
    res = []
    for coord in coords:
        g, a = coord.split(":")[:2]
        base = f"https://repo1.maven.org/maven2/{g.replace('.', '/')}/{a}/"
        meta = ET.fromstring(fetch(base + "maven-metadata.xml"))
        ver = coord.split(":")[2] if coord.count(":") == 2 else meta.find("versioning/release").text
        url = f"{base}{ver}/{a}-{ver}.jar"
        sha = "TOFU"
        try:
            s = fetch(url + ".sha256", False).strip().split()[0]
            if re.fullmatch(r"[0-9a-f]{64}", s):
                sha = s
        except Exception:  # noqa: BLE001
            pass
        res.append(inp(a, url, sha, head_size(url),
                       notes=f"{g}:{a}:{ver}" + (" (SHA-256 from Maven Central .sha256)" if sha != "TOFU" else
                                                  " (Maven Central publishes SHA-1 only; TOFU)")))
    return res


def fdroid_apks(pkgs):
    idx = json.loads(fetch("https://f-droid.org/repo/index-v2.json", False))
    res = []
    for pkg in pkgs:
        p = idx["packages"].get(pkg)
        if not p:
            raise SystemExit(f"f-droid package {pkg} not found")
        vers = list(p["versions"].values())
        best = max(vers, key=lambda v: v["manifest"]["versionCode"])
        f = best["file"]
        lic = p["metadata"].get("license", "?")
        res.append(inp(pkg.replace(".", "-"), "https://f-droid.org/repo" + f["name"], f["sha256"], f["size"],
                       notes=f"{pkg} {best['manifest'].get('versionName')} (versionCode "
                             f"{best['manifest']['versionCode']}), license {lic} (SHA-256 from F-Droid index-v2)"))
    return res


def alpine_apks(names, branch="v3.24", repo="main"):
    raw = fetch(f"https://dl-cdn.alpinelinux.org/alpine/{branch}/{repo}/x86_64/APKINDEX.tar.gz")
    with tarfile.open(fileobj=io.BytesIO(raw)) as tf:
        txt = tf.extractfile("APKINDEX").read().decode()
    idx = {}
    for block in txt.strip().split("\n\n"):
        r = dict(ln.split(":", 1) for ln in block.splitlines() if ":" in ln)
        idx[r["P"]] = r
    res = []
    for n in names:
        r = idx.get(n)
        if not r:
            raise SystemExit(f"alpine package {n} not found")
        url = f"https://dl-cdn.alpinelinux.org/alpine/{branch}/{repo}/x86_64/{n}-{r['V']}.apk"
        res.append(inp(n, url, "TOFU", int(r["S"]),
                       notes=f"{n} {r['V']} license {r.get('L')} (APKINDEX has SHA-1 of the control segment only; TOFU)"))
    return res


def openwrt_images(release, target, names):
    base = f"https://downloads.openwrt.org/releases/{release}/targets/{target}/"
    sums = {}
    for ln in fetch(base + "sha256sums", False).splitlines():
        h, f = ln.split(" *", 1)
        sums[f] = h
    res = []
    for n in names:
        fn = f"openwrt-{release}-{target.replace('/', '-')}-{n}"
        if fn not in sums:
            raise SystemExit(f"openwrt image {fn} not found")
        short = n.replace("tplink_", "").replace("-squashfs-", "-")
        res.append(inp(short, base + fn, sums[fn], head_size(base + fn),
                       notes=f"SHA-256 from {base}sha256sums"))
    return res


def xorg_tarballs(paths):
    res = []
    for p in paths:
        url = "https://www.x.org/releases/individual/" + p
        if head_size(url) is None:
            raise SystemExit(f"x.org tarball missing: {url}")
        res.append(inp(os.path.basename(p).replace(".tar", "-tar"), url, "TOFU", head_size(url),
                       notes="X.org publishes checksums in release announcements only; TOFU"))
    return res


def gh_asset(repo, tag, name):
    d = json.loads(fetch(f"https://api.github.com/repos/{repo}/releases/tags/{tag}", False))
    for a in d["assets"]:
        if a["name"] == name:
            dig = (a.get("digest") or "").removeprefix("sha256:")
            return inp(name.lower(), a["browser_download_url"], dig if len(dig) == 64 else "TOFU", a["size"],
                       notes=f"GitHub release {repo}@{tag}" + (" (SHA-256 from GitHub asset digest)" if dig else ""))
    raise SystemExit(f"asset {name} not in {repo}@{tag}")


def sums_lookup(url, fname, algo_style="gnu"):
    text = fetch(url, False)
    for ln in text.splitlines():
        if algo_style == "bsd":
            m = re.match(r"SHA256 \((.+)\) = ([0-9a-f]{64})", ln)
            if m and m.group(1) == fname:
                return m.group(2)
        else:
            parts = ln.split()
            if len(parts) >= 2 and parts[1].lstrip("*") == fname and re.fullmatch(r"[0-9a-f]{64}", parts[0]):
                return parts[0]
    raise SystemExit(f"{fname} not in {url}")


# ---------------------------------------------------------------------------------------------
# licenses

def lic(name, redistributable=True, attribution="", notes=""):
    return {"spdx_or_name": name, "redistributable": redistributable, "attribution": attribution, "notes": notes}


CC0_GEN = lic("CC0-1.0", True, "Entrybound research (generated data)",
              "Generated by a script in research/corpus/generators/g4-binary; no third-party content.")


# ---------------------------------------------------------------------------------------------
# item definitions

def items():
    out = []

    def add(**kw):
        out.append(kw)

    docker_prune = f"{GEN}/prune_tree.py"
    unpack = f"{GEN}/unpack.py"

    # ======================= F09 executable/binary objects =======================
    add(item_id="f09-ubuntu-noble-usr-binaries", family="F09", split="tuning", scale="medium",
        kind="docker-export", real_or_generated="real",
        recipe={"docker": {"image": "ubuntu:24.04@sha256:224a1869083a311ef3f13648a154ba79832fbef6364d31493642ca03082da254",
                           "platform": "linux/amd64"},
                "steps": [{"op": "extract", "input": "docker-rootfs", "format": "tar"},
                          {"op": "run", "script": docker_prune,
                           "params": {"keep": ["bin", "sbin", "lib", "lib64", "usr/bin", "usr/sbin", "usr/lib",
                                               "usr/lib64", "usr/libexec"],
                                      "require": ["usr/bin", "usr/lib"]}}],
                "output_pin": "TOFU",
                "notes": "Official Docker Hub ubuntu:24.04 index digest resolved 2026-09-12; exported rootfs pruned to "
                         "executable/library directories (usrmerge symlinks bin/sbin/lib/lib64 kept)."},
        license=lic("Ubuntu archive packages (GPL-2.0/GPL-3.0/LGPL-2.1/MIT/BSD and others)", True,
                    "Canonical Ltd. and the respective upstream authors",
                    "Per-package copyright files live in /usr/share/doc of the image (pruned here); corresponding "
                    "source is available from the Ubuntu archive."),
        independence_group="ubuntu-noble",
        description="ELF executables and shared libraries (/usr/bin, /usr/sbin, /usr/lib, /usr/libexec) of the official "
                    "Ubuntu 24.04 container image (glibc, amd64).",
        tags=["elf", "glibc", "container-rootfs"])
    add(item_id="f09-alpine-324-rootfs-binaries", family="F09", split="tuning", scale="small",
        kind="docker-export", real_or_generated="real",
        recipe={"docker": {"image": "alpine:3.24@sha256:28bd5fe8b56d1bd048e5babf5b10710ebe0bae67db86916198a6eec434943f8b",
                           "platform": "linux/amd64"},
                "steps": [{"op": "extract", "input": "docker-rootfs", "format": "tar"},
                          {"op": "run", "script": docker_prune,
                           "params": {"keep": ["bin", "sbin", "lib", "usr/bin", "usr/sbin", "usr/lib", "usr/libexec"],
                                      "require": ["usr/lib"]}}],
                "output_pin": "TOFU",
                "notes": "Official Docker Hub alpine:3.24 index digest resolved 2026-09-12 (busybox + musl, many symlinks)."},
        license=lic("Alpine packages (GPL-2.0-only busybox, MIT musl, OpenSSL Apache-2.0, others)", True,
                    "Alpine Linux developers and upstream authors", "Source available from Alpine aports."),
        independence_group="alpine-linux",
        description="musl/busybox ELF binaries and libraries (/bin, /sbin, /lib, /usr/bin, /usr/lib) of the official "
                    "Alpine 3.24 container image.",
        tags=["elf", "musl", "symlink-heavy"])
    add(item_id="f09-silesia-mozilla-ooffice", family="F09", split="tuning", scale="medium",
        kind="download", real_or_generated="real",
        recipe={"inputs": [inp("mozilla-bz2", "https://sun.aei.polsl.pl/~sdeor/corpus/mozilla.bz2", "TOFU", 17914392),
                           inp("ooffice-bz2", "https://sun.aei.polsl.pl/~sdeor/corpus/ooffice.bz2", "TOFU", 2862526)],
                "steps": [{"op": "decompress", "input": "mozilla-bz2", "dest": "mozilla", "format": "bz2"},
                          {"op": "decompress", "input": "ooffice-bz2", "dest": "ooffice", "format": "bz2"}],
                "notes": "Individual Silesia files from the corpus home page (not silesia.zip). samba (source code tar) "
                         "was deliberately not used: it is text, not a binary object (belongs to F01)."},
        license=lic("Silesia corpus (mozilla: Mozilla 1.0 binaries MPL-1.1/NPL-1.1; ooffice: OpenOffice.org 1.01 DLL LGPL-2.1/SISSL-1.1)",
                    True, "Sebastian Deorowicz, Silesian University of Technology (corpus); Mozilla; Sun Microsystems",
                    "Freely distributed research corpus; upstream licenses permit redistribution."),
        independence_group="silesia-corpus",
        description="Silesia corpus 'mozilla' (tarred executables of Mozilla 1.0, Tru64 UNIX edition) and 'ooffice' "
                    "(a DLL from OpenOffice.org 1.01), bzip2-decompressed as published.",
        tags=["public-benchmark", "elf", "pe", "silesia"])
    add(item_id="f09-cpython-3147-embed-win-amd64", family="F09", split="validation", scale="medium",
        kind="download", real_or_generated="real",
        recipe={"inputs": [inp("python-embed", "https://www.python.org/ftp/python/3.14.7/python-3.14.7-embed-amd64.zip",
                               "TOFU", 12673227, notes="python.org publishes Sigstore bundles, not SHA-256 lists")],
                "steps": [{"op": "extract", "input": "python-embed", "format": "zip"},
                          {"op": "remove", "paths": ["python314.zip"]}],
                "notes": "python314.zip (compressed stdlib bytecode) removed so the item is PE binaries only."},
        license=lic("PSF-2.0 (bundled: OpenSSL Apache-2.0, libffi MIT, SQLite blessing, Tcl/Tk-style, zlib)", True,
                    "Python Software Foundation", "LICENSE.txt is kept in the item."),
        independence_group="cpython",
        description="Windows x64 PE binaries (python.exe, python314.dll, *.pyd, vcruntime) from the official CPython "
                    "3.14.7 embeddable package.",
        tags=["pe", "windows"])
    add(item_id="f09-fedora-44-usr-binaries", family="F09", split="validation", scale="medium",
        kind="docker-export", real_or_generated="real",
        recipe={"docker": {"image": "fedora:44@sha256:43b29f65a41eb9c35e1cd5323e3bdf3b655c2357a9f4f1ff2f9c2798e5045d80",
                           "platform": "linux/amd64"},
                "steps": [{"op": "extract", "input": "docker-rootfs", "format": "tar"},
                          {"op": "run", "script": docker_prune,
                           "params": {"keep": ["bin", "sbin", "lib", "lib64", "usr/bin", "usr/sbin", "usr/lib",
                                               "usr/lib64", "usr/libexec"],
                                      "require": ["usr/bin", "usr/lib64"]}}],
                "output_pin": "TOFU",
                "notes": "Official Docker Hub fedora:44 index digest resolved 2026-09-12."},
        license=lic("Fedora packages (GPL/LGPL/MIT/BSD and others)", True, "Fedora Project contributors",
                    "Corresponding source available from Fedora (SRPMs)."),
        independence_group="fedora",
        description="ELF executables and shared libraries of the official Fedora 44 container image (glibc, lib64 layout).",
        tags=["elf", "glibc", "container-rootfs"])
    add(item_id="f09-git-for-windows-portable-2550", family="F09", split="validation", scale="medium",
        kind="download", real_or_generated="real",
        recipe={"inputs": [gh_asset("git-for-windows/git", "v2.55.0.windows.5", "PortableGit-2.55.0.5-64-bit.7z.exe")],
                "steps": [{"op": "run", "script": unpack,
                           "params": {"input": "portablegit-2-55-0-5-64-bit-7z-exe", "format": "7z", "dest": "",
                                      "include": ["mingw64/bin/**", "mingw64/libexec/**", "usr/bin/**", "usr/lib/**"],
                                      "normalize_modes": True}}],
                "notes": "7-Zip self-extracting archive unpacked with p7zip; pruned to binary directories. "
                         "Git's libexec/git-core holds many byte-identical copies of git.exe (hardlinks on Windows)."},
        license=lic("GPL-2.0-only (Git) plus MSYS2/MinGW-w64 components (GPL, LGPL, BSD, MIT, Perl Artistic)", True,
                    "Git for Windows project, MSYS2, MinGW-w64", "Sources published by the Git for Windows project."),
        independence_group="git-for-windows",
        description="Windows x64 PE executables and DLLs (MSYS2 runtime, MinGW-w64 Git, Perl DLLs) from the official "
                    "PortableGit 2.55.0.5 release.",
        tags=["pe", "windows", "duplicate-executables"])
    add(item_id="f09-heldout-almalinux-10-usr-binaries", family="F09", split="heldout", scale="medium",
        kind="docker-export", real_or_generated="real",
        recipe={"docker": {"image": "almalinux:10@sha256:957738702313e6ee452cdb17bc1431542c467be9a4e2f4da3b8e551b0ebb9677",
                           "platform": "linux/amd64"},
                "steps": [{"op": "extract", "input": "docker-rootfs", "format": "tar"},
                          {"op": "run", "script": docker_prune,
                           "params": {"keep": ["bin", "sbin", "lib", "lib64", "usr/bin", "usr/sbin", "usr/lib",
                                               "usr/lib64", "usr/libexec"],
                                      "require": ["usr/bin"]}}],
                "output_pin": "TOFU",
                "notes": "Official Docker Hub almalinux:10 index digest resolved 2026-09-12."},
        license=lic("AlmaLinux packages (GPL/LGPL/MIT/BSD and others)", True, "AlmaLinux OS Foundation and upstream authors",
                    "Corresponding source available from AlmaLinux vault."),
        independence_group="almalinux",
        description="Held-out: executable/library directories exported from the official AlmaLinux 10 container image "
                    "(pinned digest).",
        tags=["elf"])
    add(item_id="f09-heldout-powershell-766-win-x64", family="F09", split="heldout", scale="medium",
        kind="download", real_or_generated="real",
        recipe={"inputs": [gh_asset("PowerShell/PowerShell", "v7.6.6", "PowerShell-7.6.6-win-x64.zip")],
                "steps": [{"op": "extract", "input": "powershell-7-6-6-win-x64-zip", "format": "zip"}]},
        license=lic("MIT (PowerShell, .NET runtime) with third-party notices", True, "Microsoft Corporation",
                    "ThirdPartyNotices.txt included in the release."),
        independence_group="powershell",
        description="Held-out: official PowerShell 7.6.6 win-x64 release zip, extracted (GitHub release asset).",
        tags=["pe", "windows"])
    add(item_id="f09-heldout-uutils-coreutils-0110-macos", family="F09", split="heldout", scale="medium",
        kind="download", real_or_generated="real",
        recipe={"inputs": [gh_asset("uutils/coreutils", "0.11.0", "coreutils-0.11.0-x86_64-apple-darwin.tar.gz"),
                           gh_asset("uutils/coreutils", "0.11.0", "coreutils-0.11.0-aarch64-apple-darwin.tar.gz")],
                "steps": [{"op": "extract", "input": "coreutils-0-11-0-x86_64-apple-darwin-tar-gz", "dest": "x86_64-apple-darwin"},
                          {"op": "extract", "input": "coreutils-0-11-0-aarch64-apple-darwin-tar-gz", "dest": "aarch64-apple-darwin"}]},
        license=lic("MIT", True, "uutils developers", ""),
        independence_group="uutils-coreutils",
        description="Held-out: official uutils coreutils 0.11.0 macOS release archives (x86_64 and aarch64), extracted "
                    "(GitHub release assets).",
        tags=["macho"])

    ninja = ["ninja-linux.zip", "ninja-linux-aarch64.zip", "ninja-mac.zip", "ninja-win.zip", "ninja-winarm64.zip"]
    add(item_id="f09-heldout-ninja-1132-multiplatform", family="F09", split="heldout", scale="small",
        kind="download", real_or_generated="real",
        recipe={"inputs": [gh_asset("ninja-build/ninja", "v1.13.2", n) for n in ninja],
                "steps": [{"op": "extract", "input": n.lower().replace(".", "-"), "format": "zip",
                           "dest": n.removesuffix(".zip")} for n in ninja]},
        license=lic("Apache-2.0", True, "Ninja authors", ""),
        independence_group="ninja-build",
        description="Held-out: official Ninja 1.13.2 release binaries for Linux x86_64/aarch64, macOS and Windows "
                    "x64/arm64 (GitHub release assets), extracted.",
        tags=["elf", "macho", "pe"])

    # ======================= F10 highly redundant binaries =======================
    add(item_id="f10-rust-fd-find-1050-multiprofile", family="F10", split="tuning", scale="medium",
        kind="build", real_or_generated="derived-from-real",
        recipe={"inputs": [inp("crate", "https://static.crates.io/crates/fd-find/fd-find-10.5.0.crate",
                               "f35becce49c551724fd26d9824ec01d6578868cecbcc4e3d060658f9f8ad1e4d", 137149,
                               notes="SHA-256 from the crates.io API (fd-find 10.5.0)")],
                "generator": {"script": f"{GEN}/build_rust_multiprofile.sh", "interpreter": "bash", "seed": 0,
                              "params": {"bin": "fd", "toolchain": "stable",
                                         "profiles": ["dev", "dev-debug1-opt1", "release-debuginfo", "release-nostrip"],
                                         "keep_rlibs_profile": "dev"}},
                "output_pin": "TOFU",
                "notes": "Dependencies fetched from crates.io with `cargo fetch --locked` (checksums verified by the "
                         "published Cargo.lock). Paths remapped so the output is location independent."},
        license=lic("MIT OR Apache-2.0 (fd) plus dependency licenses (MIT/Apache-2.0/Unicode-3.0; jemalloc BSD-2-Clause)",
                    True, "fd authors (David Peter et al.) and crate authors", ""),
        independence_group="rust-fd-find",
        description="The same Rust program (fd-find 10.5.0) built with four profiles (debug, debug1+opt1, "
                    "release+debuginfo, release unstripped) plus the debug-profile .rlib static libraries: DWARF-heavy, "
                    "mutually near-duplicate binaries.",
        tags=["elf", "debuginfo", "static-library", "multi-build"])
    add(item_id="f10-linux-firmware-20260910-subset", family="F10", split="tuning", scale="medium",
        kind="download", real_or_generated="real",
        recipe={"inputs": [inp("firmware", "https://cdn.kernel.org/pub/linux/kernel/firmware/linux-firmware-20260910.tar.xz",
                               sums_lookup("https://cdn.kernel.org/pub/linux/kernel/firmware/sha256sums.asc",
                                           "linux-firmware-20260910.tar.xz"), 656677324,
                               notes="SHA-256 from kernel.org sha256sums.asc")],
                "steps": [{"op": "run", "script": unpack,
                           "params": {"input": "firmware", "format": "tar", "dest": "", "strip_components": 1,
                                      "members": ["linux-firmware-20260910/WHENCE", "linux-firmware-20260910/amdgpu/*",
                                                  "linux-firmware-20260910/i915/*", "linux-firmware-20260910/xe/*",
                                                  "linux-firmware-20260910/intel/iwlwifi/*",
                                                  "linux-firmware-20260910/rtw89/*", "linux-firmware-20260910/radeon/*"]}}],
                "notes": "Subset chosen for families of near-duplicate blobs (successive firmware API versions, per-SKU "
                         "variants) and sized to the medium tier from the tarball index (~473 MiB). The first "
                         "attempt used a top-level iwlwifi-* pattern, which no longer exists (moved to intel/iwlwifi/)."},
        license=lic("linux-firmware (per-file licenses listed in WHENCE; redistribution permitted, modification often not)",
                    True, "Firmware vendors (AMD, Intel, Realtek, MediaTek, Qualcomm, NVIDIA) via linux-firmware",
                    "WHENCE is kept in the item; only unmodified redistribution is permitted for most blobs."),
        independence_group="linux-firmware",
        description="Subset of linux-firmware 20260910 (amdgpu, i915, xe, intel/iwlwifi, rtw89, radeon plus WHENCE): "
                    "families of versioned and per-SKU near-duplicate firmware images.",
        tags=["firmware", "near-duplicate"])
    add(item_id="f10-gen-redundant-binary-blobs", family="F10", split="tuning", scale="small",
        kind="generate", real_or_generated="generated",
        recipe={"generator": {"script": f"{GEN}/gen_redundant_blobs.py", "seed": 1009,
                              "params": {"profile": "mixed-v1"}}},
        license=CC0_GEN, independence_group="gen-g4-redundant-blobs",
        description="Generated: zero-filled and 0xFF-padded flash-like images, fixed-size record streams, periodic "
                    "patterns, shifted repeats and near-duplicate variants (synthetic worst/best cases for dedup).",
        tags=["generated", "zeros", "padding", "records"])
    add(item_id="f10-debian-13-edk2-firmware-images", family="F10", split="validation", scale="medium",
        kind="download", real_or_generated="real",
        recipe={"inputs": debian_debs(["ovmf", "qemu-efi-aarch64"]),
                "steps": [{"op": "run", "script": unpack,
                           "params": {"input": "ovmf", "format": "deb", "dest": "ovmf"}},
                          {"op": "run", "script": unpack,
                           "params": {"input": "qemu-efi-aarch64", "format": "deb", "dest": "qemu-efi-aarch64"}}],
                "notes": "Debian package contents unpacked with dpkg-deb -x; AAVMF/QEMU_EFI flash images are padded "
                         "to fixed flash sizes."},
        license=lic("BSD-2-Clause-Patent (edk2) with bundled OpenSSL (Apache-2.0) and others", True,
                    "TianoCore edk2 contributors; Debian packagers", "usr/share/doc/*/copyright kept in the item."),
        independence_group="debian-trixie",
        description="UEFI firmware volumes from Debian 13 ovmf and qemu-efi-aarch64 packages (OVMF/AAVMF code and vars "
                    "images in several flash sizes and secure-boot variants).",
        tags=["firmware", "padding", "near-duplicate"])
    add(item_id="f10-fedora-44-static-libraries", family="F10", split="validation", scale="medium",
        kind="download", real_or_generated="real",
        recipe={"inputs": fedora_rpms(["glibc-static", "libstdc++-static", "zlib-ng-compat-static", "ghc-base-devel",
                                       "mingw64-crt", "mingw64-winpthreads-static", "libgfortran-static", "glib2-static",
                                       "qt6-qtbase-static"]),
                "steps": [{"op": "run", "script": unpack,
                           "params": {"input": "*", "format": "rpm", "dest": "", "include": ["**/*.a"],
                                      "prune_empty_dirs": True}}],
                "notes": "Static archives (.a) extracted from Fedora 44 release RPMs with bsdtar."},
        license=lic("Mixed: LGPL-2.1-or-later (glibc), GPL-3.0-with-GCC-exception (libstdc++), Apache-2.0 WITH LLVM-exception, "
                    "Zlib, BSD-3-Clause (GHC base)", True, "Upstream projects; Fedora packagers", ""),
        independence_group="fedora",
        description="Static libraries (.a archives of ELF objects with symbol tables) from Fedora 44 glibc-static, "
                    "libstdc++-static, llvm-static, zlib-ng-compat-static and ghc-base-devel.",
        tags=["static-library", "elf"])
    openwrt_devices = [
        "tplink_tl-wdr3500-v1", "tplink_tl-wdr3600-v1", "tplink_tl-wdr4300-v1", "tplink_tl-wdr4300-v1-il",
        "tplink_tl-wdr4310-v1", "tplink_tl-wr1043nd-v2", "tplink_tl-wr1043nd-v3", "tplink_tl-wr1043nd-v4",
        "tplink_tl-wr1043n-v5", "tplink_tl-wr1045nd-v2", "tplink_tl-wr2543-v1", "tplink_tl-wr842n-v3",
        "tplink_tl-wr810n-v1", "tplink_cpe210-v1", "tplink_cpe210-v2", "tplink_cpe210-v3", "tplink_cpe220-v2",
        "tplink_cpe220-v3", "tplink_cpe510-v1", "tplink_cpe510-v2", "tplink_cpe510-v3", "tplink_cpe605-v1",
        "tplink_cpe610-v1", "tplink_cpe610-v2", "tplink_wbs210-v1", "tplink_wbs210-v2", "tplink_wbs510-v1",
        "tplink_wbs510-v2"]
    add(item_id="f10-heldout-openwrt-25125-ath79-images", family="F10", split="heldout", scale="medium",
        kind="download", real_or_generated="real",
        recipe={"inputs": openwrt_images("25.12.5", "ath79/generic",
                                         [d + "-squashfs-sysupgrade.bin" for d in openwrt_devices]
                                         + [d + "-squashfs-factory.bin" for d in openwrt_devices[13:19]])},
        license=lic("GPL-2.0-only (OpenWrt firmware: Linux, busybox, others)", True, "OpenWrt project",
                    "Source available from the OpenWrt project."),
        independence_group="openwrt",
        description="Held-out: official OpenWrt 25.12.5 ath79/generic firmware images for TP-Link devices (sysupgrade and "
                    "factory), SHA-256 pinned from the release sha256sums.",
        tags=["firmware"])
    add(item_id="f10-heldout-quickjs-20260604-multibuild", family="F10", split="heldout", scale="medium",
        kind="build", real_or_generated="derived-from-real",
        recipe={"inputs": [inp("quickjs", "https://bellard.org/quickjs/quickjs-2026-06-04.tar.xz", "TOFU", 621500)],
                "generator": {"script": f"{GEN}/build_c_multibuild.sh", "interpreter": "bash", "seed": 0,
                              "params": {"variants": {"O0-g3": "-O0 -g3", "O2-g": "-O2 -g", "Os-g": "-Os -g",
                                                      "O2-nodebug": "-O2"},
                                         "targets": ["qjs", "libquickjs.a"]}},
                "output_pin": "TOFU"},
        license=lic("MIT", True, "Fabrice Bellard and Charlie Gordon", ""),
        independence_group="quickjs",
        description="Held-out: QuickJS 2026-06-04 built from the release tarball with four C compiler flag sets.",
        tags=["elf", "multi-build"])

    rpi_commit = "b76effded7afe5ed92070ff169e0109a8487fb82"  # tag 1.20260907
    rpi_files = ["start4.elf", "start4x.elf", "start4cd.elf", "start4db.elf", "fixup4.dat", "fixup4x.dat",
                 "fixup4cd.dat", "fixup4db.dat", "LICENCE.broadcom"]
    add(item_id="f10-heldout-raspberrypi-firmware-pi4-variants", family="F10", split="heldout", scale="small",
        kind="download", real_or_generated="real",
        recipe={"inputs": [inp(f, f"https://raw.githubusercontent.com/raspberrypi/firmware/{rpi_commit}/boot/{f}", "TOFU",
                               head_size(f"https://raw.githubusercontent.com/raspberrypi/firmware/{rpi_commit}/boot/{f}"),
                               notes="raspberrypi/firmware tag 1.20260907; GitHub raw file at the tagged commit (TOFU)")
                           for f in rpi_files]},
        license=lic("Broadcom binary firmware licence (boot/LICENCE.broadcom: redistribution in binary form without modification permitted)",
                    True, "Broadcom Corporation / Raspberry Pi Ltd", "LICENCE.broadcom is included in the item."),
        independence_group="raspberrypi-firmware",
        description="Held-out: Raspberry Pi 4 GPU firmware variants (start4/start4x/start4cd/start4db.elf with fixup files) "
                    "from the raspberrypi/firmware repository at tag 1.20260907.",
        tags=["firmware", "near-duplicate"])

    # ======================= F11 already-compressed files =======================
    add(item_id="f11-ubuntu-noble-debs", family="F11", split="tuning", scale="medium",
        kind="download", real_or_generated="real",
        recipe={"inputs": ubuntu_debs(["openjdk-21-jre-headless", "libllvm18", "dotnet-runtime-8.0", "containerd",
                                       "libreoffice-core", "coreutils", "bash", "vim-runtime", "perl-modules-5.38",
                                       "libc6", "git", "openssh-server", "systemd", "locales", "libicu74",
                                       "binutils-x86-64-linux-gnu", "fonts-dejavu-core", "tzdata"]),
                "notes": "Release-pocket (noble) versions, which stay in the pool for the life of the release."},
        license=lic("Ubuntu main archive packages (DFSG-free; per-package debian/copyright)", True,
                    "Canonical Ltd., Debian and upstream authors", ""),
        independence_group="ubuntu-noble",
        description="Ubuntu 24.04 .deb packages as published (ar archives with zstd-compressed control/data tarballs).",
        tags=["deb", "zstd"])
    add(item_id="f11-pypi-binary-wheels-cp312", family="F11", split="tuning", scale="medium",
        kind="download", real_or_generated="real",
        recipe={"inputs": pypi_wheels(["pyarrow", "opencv-python-headless", "onnxruntime", "duckdb", "llvmlite", "grpcio",
                                       "lxml", "shapely", "h5py"])},
        license=lic("Mixed permissive (Apache-2.0, MIT, BSD-3-Clause) per project", True, "Respective PyPI projects",
                    "Wheels bundle third-party native libraries with their own notices."),
        independence_group="pypi-binary-wheels",
        description="CPython 3.12 manylinux x86_64 binary wheels (deflate zip archives) of nine native-extension projects.",
        tags=["wheel", "zip", "deflate"])
    add(item_id="f11-alpine-324-apks", family="F11", split="tuning", scale="small",
        kind="download", real_or_generated="real",
        recipe={"inputs": alpine_apks(["busybox", "musl", "libcrypto3", "libssl3", "zlib", "zstd-libs", "xz-libs",
                                       "libcurl", "git", "sqlite-libs", "ncurses-terminfo-base", "readline", "bash",
                                       "openssh-client-default", "pcre2", "jq"]),
                "notes": "Alpine 3.24 main; superseded package builds are removed from the CDN, so the TOFU pin plus "
                         "local cache is the durable reference."},
        license=lic("Alpine packages (GPL-2.0, MIT, Apache-2.0, BSD and others)", True, "Alpine Linux developers", ""),
        independence_group="alpine-linux",
        description="Alpine Linux 3.24 .apk packages as published (concatenated gzip streams: signature, control, data).",
        tags=["apk", "gzip"])
    add(item_id="f11-fedora-44-rpms", family="F11", split="validation", scale="medium",
        kind="download", real_or_generated="real",
        recipe={"inputs": fedora_rpms(["gcc", "gcc-c++", "kernel-core", "kernel-modules-core", "python3-libs", "qt6-qtbase",
                                       "emacs-common", "webkitgtk6.0", "R-core", "php-cli", "poppler", "texlive-base"])},
        license=lic("Fedora packages (GPL/LGPL/MIT/BSD and others)", True, "Fedora Project contributors", ""),
        independence_group="fedora",
        description="Fedora 44 release RPMs as published (RPM lead/header plus zstd-compressed cpio payload).",
        tags=["rpm", "zstd"])
    add(item_id="f11-maven-central-jvm-jars", family="F11", split="validation", scale="medium",
        kind="download", real_or_generated="real",
        recipe={"inputs": maven_jars(["org.jetbrains.kotlin:kotlin-compiler-embeddable", "org.python:jython-standalone",
                                      "com.h2database:h2", "org.clojure:clojure", "org.apache.lucene:lucene-core",
                                      "org.eclipse.jdt:ecj"])},
        license=lic("Mixed: Apache-2.0 (Kotlin, Lucene), PSF-2.0 (Jython), MPL-2.0/EPL-1.0 (H2), EPL-1.0 (Clojure), EPL-2.0 (ecj)",
                    True, "JetBrains, Jython project, H2 Group, Rich Hickey, Apache Software Foundation, Eclipse Foundation", ""),
        independence_group="maven-central-jvm-jars",
        description="JVM .jar files (deflate zip) from Maven Central: Kotlin compiler, Jython standalone, H2, Clojure, "
                    "Lucene core, Eclipse JDT compiler.",
        tags=["jar", "zip", "deflate"])
    add(item_id="f11-heldout-archlinux-pkgs", family="F11", split="heldout", scale="medium",
        kind="download", real_or_generated="real",
        recipe={"inputs": arch_pkgs(["boost-libs", "ruby", "nodejs", "gtk4", "gimp", "inkscape", "mesa",
                                     "ghostscript", "erlang", "kicad", "texinfo"])},
        license=lic("Arch Linux packages (GPL/LGPL/MIT/BSD and others)", True, "Arch Linux packagers and upstream authors", ""),
        independence_group="archlinux",
        description="Held-out: Arch Linux x86_64 .pkg.tar.zst packages from the Arch Linux Archive (TOFU-pinned).",
        tags=["zstd", "tar"])
    add(item_id="f11-heldout-xorg-release-tarballs", family="F11", split="heldout", scale="medium",
        kind="download", real_or_generated="real",
        recipe={"inputs": xorg_tarballs([
            "xserver/xorg-server-21.1.24.tar.xz", "xserver/xorg-server-21.1.24.tar.gz", "xserver/xorg-server-1.20.11.tar.bz2",
            "xserver/xorg-server-1.10.6.tar.gz", "lib/libX11-1.8.13.tar.xz", "lib/libX11-1.8.13.tar.gz",
            "lib/libX11-1.7.2.tar.bz2", "lib/libXt-1.3.1.tar.xz", "lib/libXaw-1.0.16.tar.xz", "lib/libxcb-1.17.0.tar.xz",
            "lib/libXfont2-2.0.9.tar.xz", "app/twm-1.0.13.1.tar.xz", "font/font-misc-misc-1.1.3.tar.xz",
            "data/xkeyboard-config/xkeyboard-config-2.48.tar.xz", "data/xkeyboard-config/xkeyboard-config-2.34.tar.bz2"])},
        license=lic("MIT/X11 and related permissive licenses", True, "X.Org Foundation and contributors", ""),
        independence_group="xorg",
        description="Held-out: X.Org source release tarballs spanning gzip, bzip2 and xz compression.",
        tags=["gzip", "bzip2", "xz", "tar"])

    add(item_id="f11-heldout-7zip-2603-release-archives", family="F11", split="heldout", scale="small",
        kind="download", real_or_generated="real",
        recipe={"inputs": [gh_asset("ip7z/7zip", "26.03", n) for n in
                           ["7z2603-src.7z", "7z2603-src.tar.xz", "lzma2603.7z", "7z2603-extra.7z",
                            "7z2603-linux-x64.tar.xz", "7z2603-mac.tar.xz"]]},
        license=lic("LGPL-2.1-or-later with BSD-3-Clause parts and unRAR restriction (7-Zip); public domain (LZMA SDK)",
                    True, "Igor Pavlov", ""),
        independence_group="7-zip",
        description="Held-out: official 7-Zip 26.03 release archives (.7z and .tar.xz) as published on the ip7z/7zip "
                    "GitHub release.",
        tags=["7z", "xz"])

    # ======================= F14 archive-inside-archive =======================
    add(item_id="f14-apache-maven-3916-bin-tarball", family="F14", split="tuning", scale="small",
        kind="download", real_or_generated="real",
        recipe={"inputs": [inp("maven", "https://archive.apache.org/dist/maven/maven-3/3.9.16/binaries/apache-maven-3.9.16-bin.tar.gz",
                               "TOFU", 9278065, notes="Apache publishes SHA-512 "
                               "(831a8591fe20c8243b1dbe7d71e3244f31d1665b0804b2e825e38cbbe5ce0cafb8338851f90780735568773e0a6cd07bbec107cda0b896b008b861075358b6f6)")]},
        license=lic("Apache-2.0 (with bundled third-party jars under their licenses)", True, "Apache Software Foundation", ""),
        independence_group="apache-maven",
        description="Apache Maven 3.9.16 binary distribution kept as published: tar.gz containing lib/*.jar (zip) archives.",
        tags=["tar.gz", "jar", "nesting-2"])
    add(item_id="f14-jenkins-25683-war", family="F14", split="tuning", scale="medium",
        kind="download", real_or_generated="real",
        recipe={"inputs": [inp("jenkins-war", "https://get.jenkins.io/war-stable/2.568.3/jenkins.war",
                               "ccdbfdceade83489e34285a4d57c12134ffea0a09ca20062282e580cbfa5f09e", 101130898,
                               notes="SHA-256 from jenkins.war.sha256; get.jenkins.io redirects to a mirror")]},
        license=lic("MIT (Jenkins core) with bundled third-party libraries", True, "Jenkins project", ""),
        independence_group="jenkins",
        description="Jenkins 2.568.3 LTS WAR: zip containing WEB-INF/lib/*.jar and detached-plugin .hpi archives that "
                    "themselves contain jars (three archive levels).",
        tags=["war", "jar", "hpi", "nesting-3"])
    add(item_id="f14-gen-nested-archives-py", family="F14", split="tuning", scale="small",
        kind="generate", real_or_generated="generated",
        recipe={"generator": {"script": f"{GEN}/gen_nested_archives.py", "seed": 1409, "params": {"profile": "mixed-v1"}}},
        license=CC0_GEN, independence_group="gen-g4-nested-archives-py",
        description="Generated: zip-of-zips (stored/deflated), tar containing tar.gz, three-level tar.xz/zip/tar.gz "
                    "nesting, jar-like fat archives with stored nested jars, and archives mixed into a plain tree.",
        tags=["generated", "zip", "tar", "nesting-3"])
    add(item_id="f14-pyspark-420-sdist", family="F14", split="validation", scale="medium",
        kind="download", real_or_generated="real",
        recipe={"inputs": [inp("pyspark", "https://files.pythonhosted.org/packages/c3/33/c987434f5d50aa802779a004ca0fd45ee4350caab50554ad7283d5a22b50/pyspark-4.2.0.tar.gz",
                               "5ad689d53570ee1674193fd4f9bda065f0db3be9363a27d2a3406cc457b70b61", 450129423,
                               notes="SHA-256 from the PyPI JSON API")]},
        license=lic("Apache-2.0 (Spark) with bundled jars under their own licenses", True, "Apache Software Foundation", ""),
        independence_group="apache-spark",
        description="PySpark 4.2.0 source distribution as published: tar.gz containing ~hundreds of dependency jars.",
        tags=["tar.gz", "jar", "nesting-2"])
    add(item_id="f14-gen-office-documents", family="F14", split="validation", scale="small",
        kind="generate", real_or_generated="generated",
        recipe={"generator": {"script": f"{GEN}/gen_office_docs.py", "seed": 1419, "params": {"profile": "office-v1"}}},
        license=CC0_GEN, independence_group="gen-g4-office-docs",
        description="Generated: OOXML (docx/xlsx/pptx) and ODF (odt/ods) packages written directly as zip containers, "
                    "including embedded workbooks/documents inside documents and an EPUB.",
        tags=["generated", "ooxml", "odf", "zip"])
    add(item_id="f14-heldout-fdroid-apks", family="F14", split="heldout", scale="medium",
        kind="download", real_or_generated="real",
        recipe={"inputs": fdroid_apks(["org.schabi.newpipe", "de.danoeh.antennapod", "org.wikipedia",
                                       "org.kde.kdeconnect_tp", "com.nextcloud.client", "org.fdroid.fdroid",
                                       "org.videolan.vlc"])},
        license=lic("Mixed FOSS licenses per app (GPL-3.0, Apache-2.0, MPL-2.0, AGPL-3.0)", True, "Respective app authors via F-Droid", ""),
        independence_group="f-droid-apps",
        description="Held-out: Android APKs from the main F-Droid repository (SHA-256 from index-v2).",
        tags=["apk", "zip"])
    add(item_id="f14-heldout-firefox-15501-linux-tarball", family="F14", split="heldout", scale="medium",
        kind="download", real_or_generated="real",
        recipe={"inputs": [inp("firefox", "https://ftp.mozilla.org/pub/firefox/releases/155.0.1/linux-x86_64/en-US/firefox-155.0.1.tar.xz",
                               "642ab731354a5ca790b894d4556dfb5028c61d0c24eb10d10e10a111a69c89bf", 87372544,
                               notes="SHA-256 from the release SHA256SUMS")]},
        license=lic("MPL-2.0 (Mozilla trademark policy applies to the unmodified binary)", True, "Mozilla Foundation", ""),
        independence_group="mozilla-firefox",
        description="Held-out: official Firefox 155.0.1 Linux x86_64 release tarball as published.",
        tags=["tar.xz", "zip"])
    add(item_id="f14-heldout-gen-nested-archives-cli", family="F14", split="heldout", scale="small",
        kind="generate", real_or_generated="generated",
        recipe={"generator": {"script": f"{GEN}/gen_nested_cli.sh", "interpreter": "bash", "seed": 1499,
                              "params": {"profile": "cli-v1"}}},
        license=CC0_GEN, independence_group="gen-g4-nested-archives-cli",
        description="Held-out generated (different generator from the tuning item): nested archives built with command-line "
                    "tools (tar, cpio, ar, zip, 7z, squashfs with gzip/xz/zstd/lz4/bzip2).",
        tags=["generated"])

    # ======================= F16 VM/disk-like images =======================
    add(item_id="f16-alpine-3241-minirootfs-ext4-raw", family="F16", split="tuning", scale="medium",
        kind="build", real_or_generated="derived-from-real",
        recipe={"inputs": [inp("minirootfs", "https://dl-cdn.alpinelinux.org/alpine/v3.24/releases/x86_64/alpine-minirootfs-3.24.1-x86_64.tar.gz",
                               "41f73e3cf5fa919b8aa5ca6b30dc48f0da2720776d7423e2a7748211456fe081", 3698422,
                               notes="SHA-256 from latest-releases.yaml")],
                "generator": {"script": f"{GEN}/build_ext4_image.sh", "interpreter": "bash", "seed": 0,
                              "params": {"image": "alpine-3.24.1-x86_64-ext4.img", "size_mib": 256, "label": "alpine-root",
                                         "uuid": "6c3b3c1e-4f0a-4b8e-9a53-2f5d0e1a4b16", "block_size": 4096}},
                "output_pin": "TOFU",
                "notes": "mke2fs -d populate; E2FSPROGS_FAKE_TIME, fixed UUID/hash seed; zero blocks punched to holes."},
        license=lic("Alpine packages (GPL-2.0-only busybox, MIT musl, others)", True, "Alpine Linux developers", ""),
        independence_group="alpine-linux",
        description="Raw sparse 256 MiB ext4 filesystem image populated with the Alpine 3.24.1 minirootfs.",
        tags=["raw", "ext4", "sparse"])
    add(item_id="f16-alpine-3241-minirootfs-ext4-qcow2", family="F16", split="tuning", scale="small",
        kind="derive", real_or_generated="derived-from-real",
        recipe={"from_items": ["f16-alpine-3241-minirootfs-ext4-raw"],
                "generator": {"script": f"{GEN}/convert_image.sh", "interpreter": "bash", "seed": 0,
                              "params": {"from_item": "f16-alpine-3241-minirootfs-ext4-raw",
                                         "src": "alpine-3.24.1-x86_64-ext4.img", "src_format": "raw",
                                         "dst": "alpine-3.24.1-x86_64-ext4.qcow2", "dst_format": "qcow2",
                                         "options": "compat=1.1,cluster_size=65536,lazy_refcounts=off"}}},
        license=lic("Alpine packages (GPL-2.0-only busybox, MIT musl, others)", True, "Alpine Linux developers", ""),
        independence_group="alpine-linux",
        description="The same Alpine ext4 image converted to qcow2 (uncompressed clusters) for raw-vs-qcow2 comparison.",
        tags=["qcow2", "ext4"])
    add(item_id="f16-ubuntu-noble-cloudimg-20260911-raw", family="F16", split="tuning", scale="large",
        kind="build", real_or_generated="derived-from-real",
        recipe={"inputs": [inp("cloudimg", "https://cloud-images.ubuntu.com/releases/noble/release-20260911/ubuntu-24.04-server-cloudimg-amd64.img",
                               sums_lookup("https://cloud-images.ubuntu.com/releases/noble/release-20260911/SHA256SUMS",
                                           "ubuntu-24.04-server-cloudimg-amd64.img"), 625256960,
                               notes="qcow2 image; SHA-256 from the release SHA256SUMS")],
                "generator": {"script": f"{GEN}/convert_image.sh", "interpreter": "bash", "seed": 0,
                              "params": {"input": "cloudimg", "src_format": "qcow2",
                                         "dst": "ubuntu-24.04-server-cloudimg-amd64.raw", "dst_format": "raw"}},
                "output_pin": "TOFU"},
        license=lic("Ubuntu cloud image (per-package licenses; Canonical IP rights policy permits unmodified redistribution)",
                    True, "Canonical Ltd.", "Converted qcow2->raw without modifying guest content."),
        independence_group="ubuntu-noble",
        description="Official Ubuntu 24.04 server cloud image (release 20260911) converted from qcow2 to a sparse raw "
                    "disk (GPT, ext4 root, ESP, BIOS boot).",
        tags=["raw", "sparse", "gpt", "ext4"])
    add(item_id="f16-debian-13-nocloud-20260831-qcow2", family="F16", split="validation", scale="medium",
        kind="download", real_or_generated="real",
        recipe={"inputs": [inp("nocloud", "https://cloud.debian.org/images/cloud/trixie/20260831-2587/debian-13-nocloud-amd64-20260831-2587.qcow2",
                               "TOFU", 406716416, notes="Debian publishes SHA-512 e4f716b1fb48be24085c0907bd1a0a31f03b7bf2adfbd46d9f39595a225dc38741a4b5d79910e61fa1d885ac043e5ea0663fe805f26944e1dc1211a3206022c2")]},
        license=lic("Debian image (DFSG-free packages, per-package copyright)", True, "Debian Cloud Team", ""),
        independence_group="debian-trixie",
        description="Official Debian 13 nocloud amd64 qcow2 image (build 20260831-2587) as published.",
        tags=["qcow2"])
    add(item_id="f16-gen-fat32-image", family="F16", split="validation", scale="medium",
        kind="generate", real_or_generated="generated",
        recipe={"generator": {"script": f"{GEN}/gen_fat_image.py", "seed": 1609,
                              "params": {"size_mib": 64, "fat": 32, "volume_id": "1f16c0de", "label": "EBFAT32"}},
                "output_pin": "TOFU"},
        license=CC0_GEN, independence_group="gen-g4-fat-image",
        description="Generated: 64 MiB FAT32 filesystem image (mkfs.vfat + mtools) holding a generated EFI-like tree "
                    "with long names, small text files and larger binaries.",
        tags=["generated", "fat32", "raw"])
    add(item_id="f16-heldout-almalinux-10-genericcloud-qcow2", family="F16", split="heldout", scale="medium",
        kind="download", real_or_generated="real",
        recipe={"inputs": [inp("genericcloud", "https://repo.almalinux.org/almalinux/10/cloud/x86_64/images/AlmaLinux-10-GenericCloud-10.2-20260817.0.x86_64.qcow2",
                               "bc59485c4828861a15887e30ff1bb913f0f16202fd7286208518f4814da1e10a", 496631808,
                               notes="SHA-256 from the images CHECKSUM file")]},
        license=lic("AlmaLinux image (per-package licenses)", True, "AlmaLinux OS Foundation", ""),
        independence_group="almalinux",
        description="Held-out: official AlmaLinux 10.2 GenericCloud x86_64 qcow2 image (20260817) as published.",
        tags=["qcow2"])
    add(item_id="f16-heldout-freebsd-151-ufs-vmdk", family="F16", split="heldout", scale="large",
        kind="download", real_or_generated="real",
        recipe={"inputs": [inp("vmdk-xz", "https://download.freebsd.org/releases/VM-IMAGES/15.1-RELEASE/amd64/Latest/FreeBSD-15.1-RELEASE-amd64-ufs.vmdk.xz",
                               sums_lookup("https://download.freebsd.org/releases/VM-IMAGES/15.1-RELEASE/amd64/Latest/CHECKSUM.SHA256",
                                           "FreeBSD-15.1-RELEASE-amd64-ufs.vmdk.xz", "bsd"), 665305636,
                               notes="SHA-256 from CHECKSUM.SHA256")],
                "steps": [{"op": "decompress", "input": "vmdk-xz", "dest": "FreeBSD-15.1-RELEASE-amd64-ufs.vmdk", "format": "xz"}]},
        license=lic("BSD-2-Clause (FreeBSD) with third-party components", True, "The FreeBSD Project", ""),
        independence_group="freebsd",
        description="Held-out: official FreeBSD 15.1-RELEASE amd64 UFS VM image in VMDK format (xz-decompressed as published).",
        tags=["vmdk", "ufs"])
    add(item_id="f16-heldout-gen-gpt-disk", family="F16", split="heldout", scale="medium",
        kind="generate", real_or_generated="generated",
        recipe={"generator": {"script": f"{GEN}/gen_gpt_disk.py", "seed": 1699,
                              "params": {"size_mib": 192, "esp_mib": 48,
                                         "disk_guid": "a3c1e7d2-5b4f-4e61-8d2a-9f0b1c2d3e4f"}},
                "output_pin": "TOFU"},
        license=CC0_GEN, independence_group="gen-g4-gpt-disk",
        description="Held-out generated (different generator from the FAT item): GPT-partitioned raw disk with a FAT ESP "
                    "and an ext4 partition holding generated content.",
        tags=["generated"])
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--write", action="store_true")
    ap.add_argument("--plan", action="store_true")
    args = ap.parse_args()
    its = items()
    for it in its:
        r = it["recipe"]
        if r.get("inputs"):
            dedupe_names(r["inputs"])
    # wildcard unpack input "*" means "every declared input" -> expand to explicit run steps
    for it in its:
        r = it["recipe"]
        steps = []
        for st in r.get("steps", []) or []:
            if st.get("op") == "run" and st.get("params", {}).get("input") == "*":
                for d in r["inputs"]:
                    steps.append(dict(st, params=dict(st["params"], input=d["name"])))
            else:
                steps.append(st)
        if steps:
            r["steps"] = steps
    if args.plan:
        tot = 0
        for it in its:
            s = sum(d.get("size") or 0 for d in it["recipe"].get("inputs", []))
            tot += s
            print(f"{it['family']} {it['split']:<10} {it['scale']:<6} {it['item_id']:<48} inputs={len(it['recipe'].get('inputs', []))} "
                  f"{s / 2**20:9.1f} MiB")
            for d in it["recipe"].get("inputs", []):
                print(f"      {d['name']:<44} {(d.get('size') or 0) / 2**20:8.1f} MiB {d['sha256'][:12]} {d['url']}")
        print(f"total declared download bytes: {tot / 2**30:.2f} GiB")
    doc = {"schema": "ebrc-sources-v1",
           "notes": {"group": "g4-binary", "families": ["F09", "F10", "F11", "F14", "F16"],
                     "authored": "2026-09-12 with research/corpus/generators/g4-binary/author_sources.py (resolutions "
                                 "of package indexes on that date); this JSON is authoritative",
                     "split_plan": __doc__.split("Upstream split plan", 1)[1].strip()},
           "items": its}
    if args.write:
        OUT_JSON.parent.mkdir(parents=True, exist_ok=True)
        OUT_JSON.write_text(json.dumps(doc, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
        print(f"wrote {OUT_JSON} ({len(its)} items)")


if __name__ == "__main__":
    main()
