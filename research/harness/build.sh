#!/usr/bin/env bash
# Builds (and by default tests) the research harness workspace on WSL, with
# the pinned research Rust toolchain (1.98.1) and its own target directory,
# kept out of both git and the production workspace's target/.
#
# Usage:
#   research/harness/build.sh [cargo-args...]
#
# With no arguments, runs `cargo +1.98.1 test --workspace`. Any arguments
# given replace that default entirely and are passed to cargo as-is, e.g.:
#   research/harness/build.sh build --release
#   research/harness/build.sh clippy --workspace --all-targets
#   research/harness/build.sh fmt --all -- --check
#
# CARGO_TARGET_DIR defaults to /root/eb-research/target/harness: a shared,
# stable location for this one workspace, not the per-agent
# /root/eb-research/target/b2-<label> convention other Phase B2 agents use
# while several agents build concurrently. Set CARGO_TARGET_DIR yourself
# before calling this script if you are building alongside another agent and
# want to avoid lock contention on the shared default.
#
# CARGO_BIN defaults to /root/.cargo/bin/cargo (the rustup proxy; plain
# `cargo`/`rustc` on WSL's PATH resolve to Ubuntu's packaged 1.75.0, not the
# pinned research toolchain -- see research/PROGRESS.md).
set -euo pipefail

HARNESS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CARGO_BIN="${CARGO_BIN:-/root/.cargo/bin/cargo}"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/root/eb-research/target/harness}"

if [ "$#" -eq 0 ]; then
    set -- test --workspace
fi

echo "harness dir:      $HARNESS_DIR" >&2
echo "CARGO_TARGET_DIR: $CARGO_TARGET_DIR" >&2
echo "toolchain:        1.98.1 ($CARGO_BIN +1.98.1)" >&2
echo "command:          cargo +1.98.1 $*" >&2

cd "$HARNESS_DIR"
exec "$CARGO_BIN" +1.98.1 "$@"
