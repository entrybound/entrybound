#!/usr/bin/env bash
# Build the entrybound-backed chunk-length dumper and cross-check the Python gear-norm-v1 reference.
#   wsl.exe -d Ubuntu -- bash /mnt/d/Projects/entrybound/entrybound/research/corpus/generators/g2-generated/cdc_verify.sh \
#       [--out research/corpus/generators/g2-generated/cdc_verify/results-<name>.json] PATH...
# Builds in /root/eb-research/scratch/g2-cdc-verify (outside git) with the workspace Cargo.lock.
# Never pass held-out paths (cdc_verify.py refuses them).
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(cd "$HERE/../../../.." && pwd)"
WORK=/root/eb-research/scratch/g2-cdc-verify
export CARGO_TARGET_DIR="$WORK/target"
export PATH="/root/.cargo/bin:$PATH"
export RUSTUP_TOOLCHAIN=1.98.1  # the WSL rustup default is an older 1.75 toolchain; the crate needs edition 2024
mkdir -p "$WORK/src"
sed "s|ENTRYBOUND_CRATE_PATH|$REPO/crates/entrybound|" "$HERE/cdc_verify/Cargo.toml" > "$WORK/Cargo.toml.new"
cmp -s "$WORK/Cargo.toml.new" "$WORK/Cargo.toml" 2>/dev/null || mv "$WORK/Cargo.toml.new" "$WORK/Cargo.toml"
rm -f "$WORK/Cargo.toml.new"
cp "$HERE/cdc_verify/src/main.rs" "$WORK/src/main.rs"
[[ -f "$WORK/Cargo.lock" ]] || cp "$REPO/Cargo.lock" "$WORK/Cargo.lock"
cargo build --release --quiet --manifest-path "$WORK/Cargo.toml" >&2
cd "$REPO"
exec /root/eb-research/venv/bin/python -B "$HERE/cdc_verify.py" --rust-bin "$CARGO_TARGET_DIR/release/ebrc-g2-cdc-verify" "$@"
