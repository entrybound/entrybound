#!/usr/bin/env bash
# g5-media toolchain setup (WSL Ubuntu 24.04, root). Idempotent; rerun safely.
#
#   wsl.exe -d Ubuntu -- bash /mnt/d/Projects/entrybound/entrybound/research/corpus/generators/g5-media/setup_tools.sh
#
# 1. Installs the Ubuntu-packaged encoders/inspectors used by the g5-media build scripts.
# 2. Builds mozjpeg from its upstream GitHub tag archive (sha256-verified against MOZ_SHA below)
#    into /root/eb-research/tools/mozjpeg-<ver> (outside git, static, not on PATH).
#
# The build scripts check exact tool versions (params "require_tools"), so a changed toolchain
# fails loudly instead of silently changing corpus bytes.
set -euo pipefail
export DEBIAN_FRONTEND=noninteractive

PKGS=(libjpeg-turbo-progs imagemagick libimage-exiftool-perl webp gifsicle libtiff-tools ffmpeg flac lame
      vorbis-tools opus-tools qpdf poppler-utils nasm libheif-examples libheif-plugin-x265 libavif-bin libjxl-tools icc-profiles-free)
missing=()
for p in "${PKGS[@]}"; do
  if ! dpkg-query -W -f='${Status}' "$p" 2>/dev/null | grep -q "install ok installed"; then
    missing+=("$p")
  fi
done
if ((${#missing[@]})); then
  echo "installing: ${missing[*]}"
  apt-get -o DPkg::Lock::Timeout=1800 update -qq
  apt-get -o DPkg::Lock::Timeout=1800 install -y -qq --no-install-recommends "${missing[@]}"
fi

MOZ_VER=4.1.5
MOZ_URL="https://github.com/mozilla/mozjpeg/archive/refs/tags/v${MOZ_VER}.tar.gz"
# retrieved 2026-09-12 (UTC 2026-09-13), 2313651 bytes
MOZ_SHA="9fcbb7171f6ac383f5b391175d6fb3acde5e64c4c4727274eade84ed0998fcc1"
PREFIX=/root/eb-research/tools/mozjpeg-${MOZ_VER}
SRC=/root/eb-research/tools/src
if [[ ! -x "$PREFIX/bin/cjpeg" ]]; then
  mkdir -p "$SRC"
  tgz="$SRC/mozjpeg-${MOZ_VER}.tar.gz"
  if [[ ! -f "$tgz" ]]; then
    curl -fsSL -o "$tgz.part" "$MOZ_URL"
    mv -f "$tgz.part" "$tgz"
  fi
  got=$(sha256sum "$tgz" | cut -d' ' -f1)
  echo "mozjpeg source url=$MOZ_URL sha256=$got size=$(stat -c %s "$tgz") retrieved=$(date -u -r "$tgz" +%F)"
  if [[ "$got" != "$MOZ_SHA" ]]; then
    echo "HASH MISMATCH: mozjpeg source $got != $MOZ_SHA" >&2
    exit 1
  fi
  b="$SRC/mozjpeg-${MOZ_VER}-build"
  rm -rf "$b"
  mkdir -p "$b"
  tar -xzf "$tgz" -C "$b" --strip-components=1
  cmake -S "$b" -B "$b/build" -DCMAKE_INSTALL_PREFIX="$PREFIX" -DENABLE_SHARED=FALSE -DPNG_SUPPORTED=TRUE \
        -DWITH_ARITH_ENC=TRUE -DWITH_ARITH_DEC=TRUE -DCMAKE_BUILD_TYPE=Release >"$b.cmake.log" 2>&1
  cmake --build "$b/build" -j 8 >"$b.build.log" 2>&1
  cmake --install "$b/build" >"$b.install.log" 2>&1
fi
echo "mozjpeg: $("$PREFIX/bin/cjpeg" -version 2>&1 | head -1)"
echo "cjpeg:   $(cjpeg -version 2>&1 | head -1)"
echo "magick:  $(convert -version | head -1)"
echo "exiftool: $(exiftool -ver)"
echo "cwebp:   $(cwebp -version)"
echo "ffmpeg:  $(ffmpeg -version | head -1)"
echo "flac:    $(flac --version)"
echo "lame:    $(lame --version | head -1)"
echo "oggenc:  $(oggenc --version)"
echo "opusenc: $(opusenc --version | head -1)"
echo "gifsicle: $(gifsicle --version | head -1)"
echo "tiffcp:  $(tiffcp 2>&1 | head -1)"
echo "qpdf:    $(qpdf --version | head -1)"
echo "avifenc: $(avifenc --version 2>&1 | head -1)"
echo "heif-enc: $(heif-enc --version 2>&1 | head -1)"
echo "cjxl:    $(cjxl --version 2>&1 | head -1)"
