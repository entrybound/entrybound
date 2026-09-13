#!/usr/bin/env bash
# Authoring aid: inspect only the build-system section of the QuickJS release Makefile so that
# build_c_multibuild.sh can override optimisation/debug flags.  Prints Makefile lines only.
set -euo pipefail
d=/root/eb-research/scratch/g4-binary-authoring/quickjs-probe
mkdir -p "$d"
cd "$d"
[[ -f q.tar.xz ]] || curl -fsSL -o q.tar.xz https://bellard.org/quickjs/quickjs-2026-06-04.tar.xz
tar -xJf q.tar.xz --wildcards '*/Makefile' -O | sed -n '20,60p;74,180p;205,230p;290,330p'
