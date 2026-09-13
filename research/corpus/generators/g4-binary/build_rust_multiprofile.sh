#!/usr/bin/env bash
# Build script (kind build, g4-binary F10): compile one Rust binary crate with several cargo profiles.
#
#   bash build_rust_multiprofile.sh --out <staging> --seed N --params <JSON>
# params: bin (binary target), toolchain (rustup toolchain name), profiles ([dev | dev-debug1-opt1 |
#         release-debuginfo | release-nostrip | release]), keep_rlibs_profile (optional profile whose
#         target/<dir>/deps/*.rlib are kept)
# input:  "crate" = the .crate tarball from static.crates.io (must contain Cargo.lock).
# Dependencies are fetched once with `cargo fetch --locked` (crates.io checksums from Cargo.lock) into
# $EB_CACHE/g4-binary/cargo-home.  Absolute build paths are remapped (--remap-path-prefix,
# -ffile-prefix-map) so outputs do not depend on the scratch location; CARGO_INCREMENTAL=0.
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
BIN=$(jq -er .bin <<<"$PARAMS")
TC=$(jq -er .toolchain <<<"$PARAMS")
mapfile -t PROFILES < <(jq -er '.profiles[]' <<<"$PARAMS")
RLIBS=$(jq -r '.keep_rlibs_profile // empty' <<<"$PARAMS")
CRATE=$(jq -er .crate <<<"$EB_INPUTS_JSON")

export CARGO_HOME="$EB_CACHE/g4-binary/cargo-home"
mkdir -p "$CARGO_HOME"
SRC="$EB_SCRATCH/src"
TGT="$EB_SCRATCH/target"
mkdir -p "$SRC" "$TGT"
tar -xzf "$CRATE" -C "$SRC" --strip-components=1 --no-same-owner
cd "$SRC"
[[ -f Cargo.lock ]] || { echo "crate has no Cargo.lock; refusing unlocked build" >&2; exit 1; }

CARGO=(rustup run "$TC" cargo)
export CARGO_TARGET_DIR="$TGT" CARGO_INCREMENTAL=0 CARGO_TERM_COLOR=never CARGO_TERM_PROGRESS_WHEN=never
export RUSTFLAGS="--remap-path-prefix=$SRC=/build/src --remap-path-prefix=$CARGO_HOME=/cargo --remap-path-prefix=$TGT=/build/target"
export CFLAGS="-ffile-prefix-map=$SRC=/build/src -ffile-prefix-map=$CARGO_HOME=/cargo -ffile-prefix-map=$TGT=/build/target"
export CXXFLAGS="$CFLAGS"

(
  flock 9
  "${CARGO[@]}" fetch --locked
) 9>"$CARGO_HOME/.g4-fetch.lock"

PROFILE_CFG=(
  --config 'profile.dev-debug1-opt1.inherits="dev"' --config 'profile.dev-debug1-opt1.opt-level=1'
  --config 'profile.dev-debug1-opt1.debug=1'
  --config 'profile.release-debuginfo.inherits="release"' --config 'profile.release-debuginfo.debug=2'
  --config 'profile.release-debuginfo.strip="none"'
  --config 'profile.release-nostrip.inherits="release"' --config 'profile.release-nostrip.debug=0'
  --config 'profile.release-nostrip.strip="none"'
)
profdir() { case $1 in dev) echo debug ;; *) echo "$1" ;; esac; }

for prof in "${PROFILES[@]}"; do
  echo "== cargo build --profile $prof"
  "${CARGO[@]}" build --frozen --profile "$prof" --bin "$BIN" "${PROFILE_CFG[@]}" -j 12
  install -D -m 0755 "$TGT/$(profdir "$prof")/$BIN" "$OUT/profiles/$prof/$BIN"
done
if [[ -n $RLIBS ]]; then
  mkdir -p "$OUT/rlibs/$RLIBS"
  find "$TGT/$(profdir "$RLIBS")/deps" -maxdepth 1 -name '*.rlib' -print0 | sort -z |
    xargs -0 -I{} install -m 0644 {} "$OUT/rlibs/$RLIBS/"
fi

{
  echo "crate_sha256 $(sha256sum "$CRATE" | cut -d' ' -f1)"
  echo "bin $BIN"
  echo "profiles ${PROFILES[*]}"
  echo "keep_rlibs_profile ${RLIBS:-none}"
  echo "rustflags --remap-path-prefix=<src>=/build/src --remap-path-prefix=<cargo_home>=/cargo --remap-path-prefix=<target>=/build/target"
  echo "--- rustc -vV"
  "${CARGO[@]/cargo/rustc}" -vV
  echo "--- cargo -V"
  "${CARGO[@]}" -V
  echo "--- cc --version"
  cc --version | head -1
} > "$OUT/BUILDINFO.txt"
chmod 0644 "$OUT/BUILDINFO.txt"
find "$OUT" -type d -exec chmod 0755 {} +
echo "build_rust_multiprofile: done"
