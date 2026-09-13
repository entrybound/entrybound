#!/usr/bin/env python3
"""Upstream discovery helper for the g4-binary corpus group (run manually while authoring
sources/g4-binary.json; not used by provision).  Prints candidate URLs with published sizes and
hashes.  Usage: discover.py <section> [...]"""

import json
import re
import subprocess
import sys
import urllib.request

UA = {"User-Agent": "entrybound-research-corpus-discovery/1"}


def get(url, binary=False):
    req = urllib.request.Request(url, headers=UA)
    with urllib.request.urlopen(req, timeout=120) as r:
        data = r.read()
    return data if binary else data.decode("utf-8", "replace")


def head(url):
    req = urllib.request.Request(url, headers=UA, method="HEAD")
    try:
        with urllib.request.urlopen(req, timeout=120) as r:
            return int(r.headers.get("Content-Length") or -1), r.geturl()
    except Exception as e:  # noqa: BLE001
        return f"ERR {e}", url


def show(url):
    size, final = head(url)
    print(f"  {url}\n    size={size} final={final if final != url else '='}")


def sec(name):
    print(f"\n== {name}", flush=True)


def gh_assets(repo, pattern, tag=None):
    api = f"https://api.github.com/repos/{repo}/releases/" + (f"tags/{tag}" if tag else "latest")
    d = json.loads(get(api))
    print(f"  tag {d['tag_name']}")
    for a in d["assets"]:
        if re.search(pattern, a["name"]):
            print(f"  {a['browser_download_url']}\n    size={a['size']} digest={a.get('digest')}")


def docker_digest(image):
    r = subprocess.run(["docker", "buildx", "imagetools", "inspect", image], capture_output=True, text=True)
    lines = [ln for ln in r.stdout.splitlines() if ln.startswith(("Name:", "Digest:"))]
    print(f"  {image}: {' '.join(lines)} {r.stderr.strip()[:200]}")


def main():
    for s in sys.argv[1:]:
        if s == "docker":
            sec("docker")
            for img in ("ubuntu:24.04", "alpine:3.24", "fedora:44", "fedora:43", "almalinux:10", "almalinux:9"):
                docker_digest(img)
        elif s == "github":
            sec("github")
            gh_assets("git-for-windows/git", r"^PortableGit-.*-64-bit\.7z\.exe$")
            gh_assets("PowerShell/PowerShell", r"win-x64\.zip$")
        elif s == "curlwin":
            sec("curl windows")
            html = get("https://curl.se/windows/")
            for m in sorted(set(re.findall(r'href="([^"]*win64-mingw\.zip)"', html)))[:6]:
                print("  ", m)
        elif s == "python":
            sec("python embed")
            for v in ("3.14.7", "3.13.12", "3.13.13"):
                show(f"https://www.python.org/ftp/python/{v}/python-{v}-embed-amd64.zip")
        elif s == "silesia":
            sec("silesia")
            show("https://sun.aei.polsl.pl//~sdeor/corpus/silesia.zip")
            show("https://sun.aei.polsl.pl/~sdeor/corpus/mozilla.bz2")
            show("https://sun.aei.polsl.pl/~sdeor/corpus/ooffice.bz2")
        elif s == "clouds":
            sec("ubuntu cloud")
            html = get("https://cloud-images.ubuntu.com/releases/noble/release-20260911/")
            for m in sorted(set(re.findall(r'href="(ubuntu-24\.04-server-cloudimg-amd64[^"]*)"', html))):
                print("  ", m)
            print(get("https://cloud-images.ubuntu.com/releases/noble/release-20260911/SHA256SUMS")[:1500])
            sec("debian cloud")
            base = "https://cloud.debian.org/images/cloud/trixie/20260831-2587/"
            html = get(base)
            for m in sorted(set(re.findall(r'href="(debian-13-nocloud-amd64[^"]*)"', html))):
                print("  ", m)
            print(get(base + "SHA512SUMS")[:3000])
            sec("almalinux cloud")
            for rel in ("10", "9"):
                base = f"https://repo.almalinux.org/almalinux/{rel}/cloud/x86_64/images/"
                html = get(base)
                for m in sorted(set(re.findall(r'href="(AlmaLinux-[^"]*GenericCloud[^"]*x86_64[^"]*)"', html))):
                    print("  ", base + m)
                try:
                    print(get(base + "CHECKSUM")[:1500])
                except Exception as e:  # noqa: BLE001
                    print("  CHECKSUM err", e)
            sec("freebsd")
            base = "https://download.freebsd.org/releases/VM-IMAGES/15.1-RELEASE/amd64/Latest/"
            html = get(base)
            for m in sorted(set(re.findall(r'href="(FreeBSD[^"]*)"', html))):
                print("  ", base + m)
            try:
                print(get(base + "CHECKSUM.SHA256")[:3000])
            except Exception as e:  # noqa: BLE001
                print("  CHECKSUM err", e)
        elif s == "firefox":
            sec("firefox")
            d = json.loads(get("https://product-details.mozilla.org/1.0/firefox_versions.json"))
            v = d["LATEST_FIREFOX_VERSION"]
            print("  latest", v, "esr", d.get("FIREFOX_ESR"))
            html = get(f"https://ftp.mozilla.org/pub/firefox/releases/{v}/linux-x86_64/en-US/")
            print("  ", sorted(set(re.findall(r'href="[^"]*/(firefox-[^"/]+)"', html))))
            sums = get(f"https://ftp.mozilla.org/pub/firefox/releases/{v}/SHA256SUMS")
            for ln in sums.splitlines():
                if "linux-x86_64/en-US/firefox" in ln:
                    print("  ", ln)
        elif s == "openwrt":
            sec("openwrt ath79 generic 25.12.5")
            base = "https://downloads.openwrt.org/releases/25.12.5/targets/ath79/generic/"
            sums = get(base + "sha256sums")
            n = 0
            for ln in sums.splitlines():
                if "sysupgrade" in ln or "factory" in ln:
                    n += 1
            print("  images:", n)
            print("\n".join(sums.splitlines()[:40]))
        elif s == "quickjs":
            sec("quickjs")
            show("https://bellard.org/quickjs/quickjs-2026-06-04.tar.xz")
        elif s == "firmware":
            sec("linux-firmware")
            show("https://cdn.kernel.org/pub/linux/kernel/firmware/linux-firmware-20260910.tar.xz")
            try:
                print(get("https://cdn.kernel.org/pub/linux/kernel/firmware/sha256sums.asc")[-2500:])
            except Exception as e:  # noqa: BLE001
                print("  sums err", e)
        elif s == "alpine":
            sec("alpine")
            show("https://dl-cdn.alpinelinux.org/alpine/v3.24/releases/x86_64/alpine-minirootfs-3.24.1-x86_64.tar.gz")
        elif s == "misc":
            sec("misc")
            show("https://get.jenkins.io/war-stable/2.568.3/jenkins.war")
            print("  ", get("https://get.jenkins.io/war-stable/2.568.3/jenkins.war.sha256")[:200])
            show("https://archive.apache.org/dist/maven/maven-3/3.9.16/binaries/apache-maven-3.9.16-bin.tar.gz")
            print("  ", get("https://archive.apache.org/dist/maven/maven-3/3.9.16/binaries/apache-maven-3.9.16-bin.tar.gz.sha512")[:200])
        elif s == "sizes":
            sec("sizes")
            for u in (
                "https://cloud-images.ubuntu.com/releases/noble/release-20260911/ubuntu-24.04-server-cloudimg-amd64.img",
                "https://cloud-images.ubuntu.com/releases/noble/release-20260911/ubuntu-24.04-server-cloudimg-amd64.vmdk",
                "https://cloud.debian.org/images/cloud/trixie/20260831-2587/debian-13-nocloud-amd64-20260831-2587.qcow2",
                "https://cloud.debian.org/images/cloud/trixie/20260831-2587/debian-13-nocloud-amd64-20260831-2587.raw",
                "https://cloud.debian.org/images/cloud/trixie/20260831-2587/debian-13-nocloud-amd64-20260831-2587.tar.xz",
                "https://repo.almalinux.org/almalinux/10/cloud/x86_64/images/AlmaLinux-10-GenericCloud-10.2-20260817.0.x86_64.qcow2",
                "https://download.freebsd.org/releases/VM-IMAGES/15.1-RELEASE/amd64/Latest/FreeBSD-15.1-RELEASE-amd64-ufs.vmdk.xz",
                "https://download.freebsd.org/releases/VM-IMAGES/15.1-RELEASE/amd64/Latest/FreeBSD-15.1-RELEASE-amd64-ufs.qcow2.xz",
                "https://download.freebsd.org/releases/VM-IMAGES/15.1-RELEASE/amd64/Latest/FreeBSD-15.1-RELEASE-amd64-ufs.raw.xz",
                "https://download.freebsd.org/releases/VM-IMAGES/15.1-RELEASE/amd64/Latest/FreeBSD-15.1-RELEASE-amd64-ufs.vhd.xz",
                "https://ftp.mozilla.org/pub/firefox/releases/155.0.1/linux-x86_64/en-US/firefox-155.0.1.tar.xz",
                "https://curl.se/windows/dl-8.22.0_1/curl-8.22.0_1-win64-mingw.zip",
            ):
                show(u)
            print(get("https://cloud-images.ubuntu.com/releases/noble/release-20260911/SHA256SUMS").count("\n"))
            for ln in get("https://cloud-images.ubuntu.com/releases/noble/release-20260911/SHA256SUMS").splitlines():
                if "amd64.img" in ln or "amd64.vmdk" in ln:
                    print("  ", ln)
            for ln in get("https://cloud.debian.org/images/cloud/trixie/20260831-2587/SHA512SUMS").splitlines():
                if "nocloud-amd64" in ln:
                    print("  ", ln)
            try:
                print(get("https://curl.se/windows/dl-8.22.0_1/hashes.txt")[:1500])
            except Exception as e:  # noqa: BLE001
                print("  curl hashes err", e)
        elif s == "openwrt-list":
            sec("openwrt ath79 generic listing")
            base = "https://downloads.openwrt.org/releases/25.12.5/targets/ath79/generic/"
            html = get(base)
            print(html[2000:3500])
            sums = get(base + "sha256sums")
            names = [ln.split("*", 1)[1] for ln in sums.splitlines() if "tplink" in ln and "squashfs-sysupgrade" in ln]
            print("  tplink sysupgrade count", len(names))
            for n in names:
                print("  ", n)
        elif s == "pkgindex":
            import gzip
            import lzma
            sec("ubuntu noble main Packages")
            idx = gzip.decompress(get("https://archive.ubuntu.com/ubuntu/dists/noble/main/binary-amd64/Packages.gz", True)).decode()
            pk = parse_deb822(idx)
            big = sorted(pk.values(), key=lambda p: -int(p["Size"]))[:60]
            for p in big:
                print(f"  {p['Package']:40s} {int(p['Size'])/1e6:8.1f}MB {p['Filename']}")
            sec("debian trixie main Packages (selected)")
            idx = lzma.decompress(get("https://deb.debian.org/debian/dists/trixie/main/binary-amd64/Packages.xz", True)).decode()
            pk = parse_deb822(idx)
            for name in ("ovmf", "ovmf-legacy", "libc6-dbg", "qemu-efi-aarch64", "seabios", "ipxe-qemu", "grub-efi-amd64-bin",
                         "shim-signed", "u-boot-qemu", "opensbi", "libstdc++6-14-dbg", "valgrind-dbg", "firmware-linux-free",
                         "sgabios", "qemu-system-data"):
                p = pk.get(name)
                if p:
                    print(f"  {name:28s} {p['Version']:28s} {int(p['Size'])/1e6:7.2f}MB inst={p.get('Installed-Size')}KB {p['Filename']}")
                else:
                    print(f"  {name}: missing")
            try:
                idx = lzma.decompress(get("https://deb.debian.org/debian-debug/dists/trixie-debug/main/binary-amd64/Packages.xz", True)).decode()
                pk = parse_deb822(idx)
                big = sorted(pk.values(), key=lambda p: -int(p["Size"]))[:30]
                print("  -- debian-debug largest")
                for p in big:
                    print(f"  {p['Package']:40s} {int(p['Size'])/1e6:8.1f}MB {p['Filename']}")
            except Exception as e:  # noqa: BLE001
                print("  debug archive err", e)
        elif s == "misc2":
            sec("crates fd-find")
            d = json.loads(get("https://crates.io/api/v1/crates/fd-find"))
            for v in d["versions"][:3]:
                print("  ", v["num"], v["checksum"], v.get("crate_size"), v.get("rust_version"))
            sec("alpine v3.24 main APKINDEX")
            import tarfile
            import io
            raw = get("https://dl-cdn.alpinelinux.org/alpine/v3.24/main/x86_64/APKINDEX.tar.gz", True)
            with tarfile.open(fileobj=io.BytesIO(raw)) as tf:
                txt = tf.extractfile("APKINDEX").read().decode()
            recs = []
            for block in txt.strip().split("\n\n"):
                r = dict(ln.split(":", 1) for ln in block.splitlines() if ":" in ln)
                recs.append(r)
            recs.sort(key=lambda r: -int(r.get("S", 0)))
            for r in recs[:40]:
                print(f"  {r['P']:36s} {r['V']:20s} {int(r['S'])/1e6:7.2f}MB")
            sec("xorg individual")
            for sub in ("xserver", "lib"):
                html = get(f"https://www.x.org/releases/individual/{sub}/")
                names = re.findall(r'href="([^"]+\.tar\.(?:gz|bz2|xz))"', html)
                print(f"  {sub}: {len(names)} tarballs; sample", names[-8:])
        elif s == "xorg":
            sec("xorg listings")
            for sub, pat in (("xserver", r"xorg-server-[0-9.]+\.tar\.(gz|bz2|xz)"), ("lib", r"libX11-[0-9.]+\.tar\.(gz|bz2|xz)"),
                             ("lib", r"libXt-[0-9.]+\.tar\.(gz|bz2|xz)"), ("lib", r"libxcb-[0-9.]+\.tar\.(gz|bz2|xz)"),
                             ("app", r"xterm-[0-9]+\.tar\.(gz|bz2|xz)"), ("font", r"font-misc-misc-[0-9.]+\.tar\.(gz|bz2|xz)"),
                             ("lib", r"libXaw-[0-9.]+\.tar\.(gz|bz2|xz)"), ("data/xkeyboard-config", r"xkeyboard-config-[0-9.]+\.tar\.(gz|bz2|xz)"),
                             ("app", r"twm-[0-9.]+\.tar\.(gz|bz2|xz)"), ("lib", r"libXfont2?-[0-9.]+\.tar\.(gz|bz2|xz)")):
                html = get(f"https://www.x.org/releases/individual/{sub}/")
                names = sorted(set(m.group(0) for m in re.finditer(pat, html)))
                print(f"  {sub}: {names}")
        elif s == "xzsize":
            sec("xz uncompressed sizes (index read via HTTP range)")
            for u in sys.argv[sys.argv.index("xzsize") + 1:]:
                print("  ", u, xz_uncompressed_size(u))
            return
        else:
            print("unknown section", s)


def xz_uncompressed_size(url):
    """Read the xz stream footer + index with HTTP Range requests (single-stream files)."""
    size, _ = head(url)
    req = urllib.request.Request(url, headers=dict(UA, Range=f"bytes={size - 12}-{size - 1}"))
    footer = urllib.request.urlopen(req, timeout=120).read()
    assert footer[-2:] == b"YZ", footer
    backward = (int.from_bytes(footer[4:8], "little") + 1) * 4
    start = size - 12 - backward
    req = urllib.request.Request(url, headers=dict(UA, Range=f"bytes={start}-{size - 13}"))
    idx = urllib.request.urlopen(req, timeout=120).read()
    assert idx[0] == 0, "not an index"
    pos = 1

    def varint():
        nonlocal pos
        v, shift = 0, 0
        while True:
            b = idx[pos]
            pos += 1
            v |= (b & 0x7F) << shift
            shift += 7
            if not b & 0x80:
                return v
    n = varint()
    total = 0
    for _ in range(n):
        varint()
        total += varint()
    return f"records={n} uncompressed={total} ({total / 2**30:.2f} GiB)"


def parse_deb822(text):
    out = {}
    for block in text.split("\n\n"):
        rec = {}
        key = None
        for ln in block.splitlines():
            if ln.startswith((" ", "\t")) and key:
                continue
            if ":" in ln:
                key, val = ln.split(":", 1)
                rec[key] = val.strip()
        if "Package" in rec:
            out[rec["Package"]] = rec
    return out


if False:
    pass


if __name__ == "__main__":
    main()
