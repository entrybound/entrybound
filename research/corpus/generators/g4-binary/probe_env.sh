#!/usr/bin/env bash
# Capability / upstream-availability probe for the g4-binary corpus group (no timing, no content reads).
# Usage: probe_env.sh [section...]
set -uo pipefail
C="curl -fsSL --max-time 60"
sec() { echo; echo "== $*"; }
want() { [[ $# -eq 0 ]] && return 0; return 1; }
S=("$@")
has() { [[ ${#S[@]} -eq 0 ]] && return 0; for s in "${S[@]}"; do [[ $s == "$1" ]] && return 0; done; return 1; }

if has tools; then
  sec tools
  for t in mkfs.ext4 mke2fs mkfs.vfat mcopy mmd qemu-img mkfs.xfs mkfs.btrfs mksquashfs 7z 7zz bsdtar zip unzip rustc cargo gcc make ar file rpm2cpio cpio dpkg-deb; do printf '%-12s %s\n' "$t" "$(command -v $t || echo MISSING)"; done
  ls /root/.cargo/bin 2>/dev/null | head; /root/.cargo/bin/rustc --version 2>/dev/null; /root/.cargo/bin/cargo --version 2>/dev/null
  mke2fs -V 2>&1 | head -1
  dpkg -l e2fsprogs dosfstools mtools qemu-utils 2>/dev/null | tail -n +6
fi
if has python; then
  sec python-embed
  $C https://www.python.org/ftp/python/ | grep -oE 'href="3\.1[34]\.[0-9]+/"' | sort -V | tail -4
fi
if has gfw; then
  sec git-for-windows
  $C https://api.github.com/repos/git-for-windows/git/releases/latest | grep -E '"(tag_name|browser_download_url)"' | grep -E 'tag_name|PortableGit|MinGit-[0-9.]+-64-bit.zip' | head
fi
if has pwsh; then
  sec powershell
  $C https://api.github.com/repos/PowerShell/PowerShell/releases/latest | grep -E '"(tag_name|browser_download_url)"' | grep -E 'tag_name|win-x64.zip' | head
fi
if has curlwin; then
  sec curl-windows
  $C https://curl.se/windows/ | grep -oE 'dl-[0-9_]+/curl-[0-9.]+_[0-9]+-win64-mingw.zip' | sort -u | head
fi
if has silesia; then
  sec silesia
  curl -sSI --max-time 60 https://sun.aei.polsl.pl//~sdeor/corpus/silesia.zip | head -5
fi
if has firmware; then
  sec linux-firmware
  $C https://cdn.kernel.org/pub/linux/kernel/firmware/ | grep -oE 'linux-firmware-2026[0-9]+\.tar\.xz' | sort -u | tail -3
  $C https://cdn.kernel.org/pub/linux/kernel/firmware/ | grep -E 'linux-firmware-2026' | tail -3
fi
if has alpine; then
  sec alpine
  $C https://dl-cdn.alpinelinux.org/alpine/ | grep -oE 'v3\.[0-9]+' | sort -Vu | tail -3
  $C https://dl-cdn.alpinelinux.org/alpine/latest-stable/releases/x86_64/latest-releases.yaml | grep -E 'file:|sha256' | head -30
  $C https://dl-cdn.alpinelinux.org/alpine/latest-stable/releases/cloud/ 2>/dev/null | grep -oE 'nocloud_alpine-[^"]+x86_64-bios[^"]*qcow2' | sort -u | tail -4
fi
if has docker; then
  sec docker
  for img in ubuntu:24.04 alpine:3.22 fedora:42 almalinux:9; do echo "$img"; done
fi
if has openwrt; then
  sec openwrt
  $C https://downloads.openwrt.org/releases/ | grep -oE '2[0-9]\.[0-9]+\.[0-9]+/' | sort -Vu | tail -3
fi
if has quickjs; then
  sec quickjs
  $C https://bellard.org/quickjs/ | grep -oE 'quickjs-20[0-9-]+\.tar\.xz' | sort -u | tail -3
fi
if has jenkins; then
  sec jenkins
  $C https://get.jenkins.io/war-stable/ | grep -oE '2\.[0-9]+\.[0-9]+/' | sort -Vu | tail -2
fi
if has gradle; then
  sec gradle
  $C https://services.gradle.org/versions/current | head -c 600; echo
fi
if has maven; then
  sec maven
  $C https://dlcdn.apache.org/maven/maven-3/ | grep -oE '3\.9\.[0-9]+/' | sort -Vu | tail -2
fi
if has pyspark; then
  sec pyspark
  $C https://pypi.org/pypi/pyspark/json | python3 -c 'import json,sys; d=json.load(sys.stdin); print(d["info"]["version"]); [print(u["filename"],u["size"],u["digests"]["sha256"],u["url"]) for u in d["urls"]]'
fi
if has debiancloud; then
  sec debian-cloud
  $C https://cloud.debian.org/images/cloud/trixie/ | grep -oE '20[0-9]{6}-[0-9]+/' | sort -u | tail -3
fi
if has ubuntucloud; then
  sec ubuntu-cloud
  $C https://cloud-images.ubuntu.com/releases/noble/ | grep -oE 'release-20[0-9]+(\.[0-9]+)?/' | sort -u | tail -3
fi
if has alma; then
  sec almalinux-cloud
  $C https://repo.almalinux.org/almalinux/9/cloud/x86_64/images/ | grep -oE 'AlmaLinux-9-GenericCloud-[0-9.]+-[0-9]+\.x86_64\.qcow2' | sort -u | tail -3
  $C https://repo.almalinux.org/almalinux/10/cloud/x86_64/images/ | grep -oE 'AlmaLinux-10-GenericCloud-[0-9.]+-[0-9]+\.x86_64\.qcow2' | sort -u | tail -3
fi
if has freebsd; then
  sec freebsd
  $C https://download.freebsd.org/releases/VM-IMAGES/ | grep -oE '1[0-9]\.[0-9]-RELEASE' | sort -Vu | tail -3
fi
if has fdroid; then
  sec fdroid
  curl -sSI --max-time 60 https://f-droid.org/repo/index-v2.json | head -5
fi
if has arch; then
  sec arch-archive
  curl -sSI --max-time 60 https://archive.archlinux.org/packages/z/zstd/ | head -3
fi
