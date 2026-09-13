#!/usr/bin/env bash
# Install Ubuntu 24.04 packages needed by the g1-code build/vendor recipes (idempotent).
# Build trees are recorded as unpinned (output_pin null); the package set below determines which
# optional modules get compiled (e.g. CPython _ssl/_sqlite3), so it is listed here explicitly.
set -euo pipefail
PKGS=(bison flex bc libelf-dev libssl-dev zlib1g-dev libffi-dev libbz2-dev liblzma-dev libsqlite3-dev
      libreadline-dev uuid-dev libncurses-dev libgdbm-dev libgdbm-compat-dev libicu-dev libzstd-dev
      libexpat1-dev pkg-config patch rsync)
missing=()
for p in "${PKGS[@]}"; do dpkg-query -W -f='${Status}' "$p" 2>/dev/null | grep -q 'install ok installed' || missing+=("$p"); done
if ((${#missing[@]})); then
  export DEBIAN_FRONTEND=noninteractive
  apt-get -o DPkg::Lock::Timeout=900 update -q
  apt-get -o DPkg::Lock::Timeout=900 install -y -q --no-install-recommends "${missing[@]}"
fi
for p in "${PKGS[@]}"; do printf '%s\t%s\n' "$p" "$(dpkg-query -W -f='${Version}' "$p")"; done
