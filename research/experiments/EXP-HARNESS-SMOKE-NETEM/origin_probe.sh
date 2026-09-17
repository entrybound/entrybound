#!/usr/bin/env bash
# Helper invoked by this experiment's spec.yaml `command`: runs
# research/harness/crates/ebr-netem's `ebr-origin` binary (the "one binary"
# smoke target for the ebr-netem crate) against a tiny directory holding the
# spec's generated item, fetches that item back over one real HTTP GET, and
# prints one JSON line the spec's stdout_json metrics read.
#
# ebr-origin/ebr-netem-proxy serve forever rather than completing like the
# other harness binaries (ebr-pack/-chunk/-codec/-planner/-access) that print
# one row and exit, so they cannot be an ebr spec `command` on their own; this
# script is the minimal wiring that starts one, does one real request against
# it, and stops it -- see research/harness/README.md's known limitations.
#
# Usage: origin_probe.sh <item_path> <scratch_dir>
set -uo pipefail

ITEM_PATH="$1"
SCRATCH="$2"
ORIGIN_BIN=/root/eb-research/target/harness/release/ebr-origin
ROOT="$SCRATCH/root"
LOG="$SCRATCH/origin.log"
FETCHED="$SCRATCH/fetched.bin"

mkdir -p "$ROOT"
cp "$ITEM_PATH" "$ROOT/payload.bin"

"$ORIGIN_BIN" --root "$ROOT" --listen 127.0.0.1:0 --env-id adhoc >"$LOG" 2>&1 &
PID=$!
trap 'kill "$PID" 2>/dev/null; wait "$PID" 2>/dev/null' EXIT

ADDR=""
for _ in $(seq 1 100); do
    ADDR=$(grep -o 'listening on [^ ]*' "$LOG" 2>/dev/null | awk '{print $3}')
    [[ -n "$ADDR" ]] && break
    sleep 0.05
done
if [[ -z "$ADDR" ]]; then
    echo "origin_probe: ebr-origin never printed a listen address" >&2
    cat "$LOG" >&2
    printf '{"schema":"ebr.harness.smoke.v1","experiment_id":"EXP-HARNESS-SMOKE-NETEM","payload":{"binary":"ebr-origin","started":false,"roundtrip_ok":0}}\n'
    exit 1
fi

curl -sS -o "$FETCHED" "http://$ADDR/payload.bin"
CURL_RC=$?

roundtrip_ok=0
fetched_bytes=0
if [[ $CURL_RC -eq 0 ]] && cmp -s "$FETCHED" "$ITEM_PATH"; then
    roundtrip_ok=1
    fetched_bytes=$(stat -c%s "$FETCHED")
fi

# roundtrip_ok is 1/0 (not JSON true/false): the spec reads it as a numeric
# stdout_json metric (family correctness), matching how other ebr specs
# (e.g. EXP-SMOKE-000's `check`-sourced roundtrip_ok) represent a pass/fail
# check as 1/0 rather than a JSON boolean.
printf '{"schema":"ebr.harness.smoke.v1","experiment_id":"EXP-HARNESS-SMOKE-NETEM","payload":{"binary":"ebr-origin","started":true,"curl_exit_code":%d,"fetched_bytes":%d,"roundtrip_ok":%d}}\n' \
    "$CURL_RC" "$fetched_bytes" "$roundtrip_ok"

if [[ "$roundtrip_ok" != "1" ]]; then
    exit 1
fi
