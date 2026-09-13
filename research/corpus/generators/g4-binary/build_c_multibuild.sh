#!/usr/bin/env bash
# Build script (kind build, g4-binary F10): build a small C project (QuickJS release tarball layout)
# several times with different optimisation/debug flag sets.
#
#   bash build_c_multibuild.sh --out <staging> --seed N --params <JSON>
# params: variants {name: "extra CFLAGS"}, targets [make targets copied to <out>/<variant>/]
# input:  "quickjs" = release tarball (.tar.xz).  Each variant builds in its own copy of the source;
# -ffile-prefix-map maps that copy to /build/quickjs so outputs are location independent.
# GNU ar on Ubuntu runs in deterministic mode by default.
set -euo pipefail
OUT= PARAMS='{}'
while [[ $# -gt 0 ]]; do
  case $1 in
    --out) OUT=$2; shift 2 ;;
    --seed) shift 2 ;;
    --params) PARAMS=$2; shift 2 ;;
    *) echo "unknown argument $1" >&2; exit 2 ;;
  esac
done
[[ -n $OUT && -d $OUT ]] || { echo "--out must be an existing directory" >&2; exit 2; }
TARBALL=$(jq -er .quickjs <<<"$EB_INPUTS_JSON")
mapfile -t TARGETS < <(jq -er '.targets[]' <<<"$PARAMS")
mapfile -t VARIANTS < <(jq -er '.variants | keys[]' <<<"$PARAMS")

SRC0="$EB_SCRATCH/src0"
mkdir -p "$SRC0"
tar -xJf "$TARBALL" -C "$SRC0" --strip-components=1 --no-same-owner
[[ -f $SRC0/Makefile ]] || { echo "Makefile missing" >&2; exit 1; }

for v in "${VARIANTS[@]}"; do
  flags=$(jq -er --arg v "$v" '.variants[$v]' <<<"$PARAMS")
  b="$EB_SCRATCH/build-$v"
  rm -rf "$b"
  cp -a "$SRC0" "$b"
  echo "== variant $v: $flags"
  make -C "$b" -j8 CONFIG_LTO= \
    "CFLAGS_OPT=\$(CFLAGS) $flags -ffile-prefix-map=$b=/build/quickjs" \
    "CFLAGS_NOLTO=\$(CFLAGS) $flags -ffile-prefix-map=$b=/build/quickjs" \
    "LDFLAGS=-g" "${TARGETS[@]}"
  for t in "${TARGETS[@]}"; do
    mode=0644; [[ -x "$b/$t" && $t != *.a ]] && mode=0755
    install -D -m "$mode" "$b/$t" "$OUT/$v/$t"
  done
done
{
  echo "tarball_sha256 $(sha256sum "$TARBALL" | cut -d' ' -f1)"
  echo "targets ${TARGETS[*]}"
  for v in "${VARIANTS[@]}"; do echo "variant $v: $(jq -r --arg v "$v" '.variants[$v]' <<<"$PARAMS")"; done
  echo "--- cc --version"; cc --version | head -1
  echo "--- ar --version"; ar --version | head -1
} > "$OUT/BUILDINFO.txt"
chmod 0644 "$OUT/BUILDINFO.txt"
find "$OUT" -type d -exec chmod 0755 {} +
echo "build_c_multibuild: done"
