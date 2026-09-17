#!/usr/bin/env bash
# Harness review round 1, finding R1-06: proves that `ebr-pack` (harness
# workspace, research/harness/Cargo.lock, research-internals enabled) writes
# byte-identical archives to the production `ebound pack` CLI (production
# workspace, root Cargo.lock, default features) for the same fixed tree, all
# four profiles, INDEXED and STREAM (`--stream-window auto`). Run
# `python3 research/harness/lock_parity.py` first; this script checks the
# consequence, not the lock files.
#
# Usage: wsl.exe -d Ubuntu -- bash /mnt/d/Projects/entrybound/entrybound/research/harness/harness-cli-parity.sh
set -euo pipefail

HARNESS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(cd "$HARNESS_DIR/../.." && pwd)"
CARGO=(/root/.cargo/bin/cargo +1.98.1)
HARNESS_TARGET="${HARNESS_TARGET:-/root/eb-research/target/b2-review}"
CLI_TARGET="${CLI_TARGET:-/root/eb-research/target/b2-review-cli}"
WORK="/root/eb-research/scratch/b2-review-cli-parity"

python3 -B "$HARNESS_DIR/lock_parity.py"

(cd "$REPO" && CARGO_TARGET_DIR="$CLI_TARGET" "${CARGO[@]}" build --locked --release -p entrybound-cli --bin ebound >&2)
CARGO_TARGET_DIR="$HARNESS_TARGET" bash "$HARNESS_DIR/build.sh" build --locked --release -p ebr-pack >&2

rm -rf "$WORK"
mkdir -p "$WORK/out"
python3 -B "$HARNESS_DIR/internals_identity_tools.py" tree "$WORK/tree" >&2

status=0
for profile in fast balanced dense extreme; do
    for layout in indexed stream; do
        window=()
        [ "$layout" = stream ] && window=(--stream-window auto)
        cli_out="$WORK/out/cli-$profile-$layout.eb"
        harness_out="$WORK/out/harness-$profile-$layout.eb"
        "$CLI_TARGET/release/ebound" pack "$WORK/tree" "$cli_out" --layout "$layout" "${window[@]}" --profile "$profile" >/dev/null
        "$HARNESS_TARGET/release/ebr-pack" --item-path "$WORK/tree" --experiment-id EXP-HARNESS-CLI-PARITY \
            --env-name wsl-ubuntu --profile "$profile" --layout "$layout" "${window[@]}" \
            --archive-out "$harness_out" >/dev/null
        cli_sha="$(sha256sum "$cli_out" | cut -d' ' -f1)"
        harness_sha="$(sha256sum "$harness_out" | cut -d' ' -f1)"
        if [ "$cli_sha" = "$harness_sha" ]; then
            echo "IDENTICAL $profile $layout $cli_sha"
        else
            echo "DIFFERENT $profile $layout cli=$cli_sha harness=$harness_sha"
            status=1
        fi
    done
done
rm -rf "$WORK"
if [ "$status" -eq 0 ]; then echo "RESULT PASS"; else echo "RESULT FAIL"; fi
exit "$status"
