#!/usr/bin/env bash
# Harness review round 1 end-to-end smoke check (WSL). Builds the harness in
# release mode and exercises every binary once on tiny generated inputs,
# asserting the fields and behaviors the round-1 fixes introduced (see
# research/harness/harness-review-round1.md). Every number this prints is
# NON-DECISION-GRADE smoke output: no quiet-machine guard, one repetition.
#
# Never reads held-out content: the refusal checks pass a nonexistent child of
# the held-out root, which the guard refuses lexically before any filesystem
# access.
#
# Usage: wsl.exe -d Ubuntu -- bash /mnt/d/Projects/entrybound/entrybound/research/harness/review-round1-smoke.sh
set -euo pipefail

HARNESS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/root/eb-research/target/b2-review}"
BIN="$CARGO_TARGET_DIR/release"
WORK="/root/eb-research/scratch/b2-review-smoke"
ENV_NAME="wsl-ubuntu"
EXP="EXP-HARNESS-REVIEW-R1-SMOKE"
HELDOUT_PROBE="/root/eb-research/heldout/__ebr_guard_selftest_nonexistent__"

bash "$HARNESS_DIR/build.sh" build --workspace --release >&2

rm -rf "$WORK"
mkdir -p "$WORK/tree/sub"
python3 - "$WORK" <<'PY'
import gzip, os, random, sys
work = sys.argv[1]
rng = random.Random(20260917)
with open(f"{work}/tree/log.txt", "w") as f:
    for i in range(40000):
        f.write(f"2026-09-17T00:{i % 60:02d}:00Z worker={i % 13} status=ok latency_ms={(i * 7) % 911}\n")
with open(f"{work}/tree/sub/noise.bin", "wb") as f:
    f.write(bytes(rng.getrandbits(8) for _ in range(3 * 1024 * 1024)))
with gzip.open(f"{work}/tree/sub/rows.csv.gz", "wb", compresslevel=6) as f:
    f.write("".join(f"{i},{i * 17},{i % 31}\n" for i in range(50000)).encode())
with open(f"{work}/planner-item.bin", "wb") as f:
    f.write(open(f"{work}/tree/log.txt", "rb").read() * 2)
PY

fail() { echo "SMOKE FAIL: $*" >&2; exit 1; }
check() { # check <json-file> <jq-expression-that-must-be-true> <label>
    jq -e "$2" "$1" >/dev/null || fail "$3 ($2)"
    echo "ok: $3"
}

# --- ebr-pack INDEXED: scoped memory, named buckets, independent cross-check
"$BIN/ebr-pack" --item-path "$WORK/tree" --experiment-id "$EXP" --env-name "$ENV_NAME" \
    --profile balanced --layout indexed --archive-out "$WORK/a.eb" > "$WORK/pack-indexed.json"
check "$WORK/pack-indexed.json" '.payload.pack_memory.scoped == true' "pack_memory is phase-scoped"
check "$WORK/pack-indexed.json" '.payload.byte_accounting.named_bucket_total_bytes == .payload.artifact_bytes' "named buckets sum to artifact size"
check "$WORK/pack-indexed.json" '.payload.byte_accounting_cross_check.matched == true' "frame walk matches inspection"
check "$WORK/pack-indexed.json" '.payload.phases.chunking_pass_seconds == null' "no harness chunking pass by default"
jq -c '{pack_memory: .payload.pack_memory, lifetime: .payload.process_lifetime_peak_rss_bytes, input_logical_bytes: .payload.input_logical_bytes}' "$WORK/pack-indexed.json"

# --- ebr-pack STREAM with --stream-window auto
"$BIN/ebr-pack" --item-path "$WORK/tree" --experiment-id "$EXP" --env-name "$ENV_NAME" \
    --profile balanced --layout stream --stream-window auto --archive-out "$WORK/a.ebs" > "$WORK/pack-stream.json"
check "$WORK/pack-stream.json" '.payload.verified == true' "STREAM pack with --stream-window auto verifies"

# --- ebr-inspect-bytes and ebr-unpack
"$BIN/ebr-inspect-bytes" --archive-path "$WORK/a.eb" --experiment-id "$EXP" --env-name "$ENV_NAME" > "$WORK/inspect.json"
check "$WORK/inspect.json" '.payload.byte_accounting_cross_check.matched == true' "inspect-bytes cross-check"
"$BIN/ebr-unpack" --archive-path "$WORK/a.ebs" --layout stream --experiment-id "$EXP" --env-name "$ENV_NAME" \
    --scratch-dir "$WORK/unpacked" > "$WORK/unpack.json"
check "$WORK/unpack.json" '.payload.verified == true and .payload.stream_staging != null and .payload.scratch.peak_total_bytes >= .payload.scratch.peak_bytes' "STREAM unpack reports staging and total scratch"

# --- ebr-access roundtrip (per-read deltas) and isolated probe
"$BIN/ebr-access" --mode roundtrip --item-path "$WORK/tree" --experiment-id "$EXP" --env-name "$ENV_NAME" \
    --sample-size 0 > "$WORK/access.json"
check "$WORK/access.json" '(.payload.random_access.metadata_open_bytes_fetched + ([.payload.random_access.first_entry_read.bytes_fetched_from_source] | add) + ([.payload.random_access.eager_reads[].bytes_fetched_from_source] | add)) == .payload.random_access.session_total_bytes_fetched' "random-access deltas sum to the session total"
"$BIN/ebr-access" --mode probe --experiment-id "$EXP" --env-name "$ENV_NAME" \
    --probe-total-bytes 33554432 --probe-sample-interval-ms 5 --probe-scratch-root "$WORK" > "$WORK/probe.json"
check "$WORK/probe.json" '[.payload.probe.stages[] | select(.stage == "verify" or .stage == "unpack") | .isolated] == [true, true]' "verify/unpack stages ran isolated"
jq -c '[.payload.probe.stages[] | {stage, isolated, peak_delta: .memory.peak_delta_bytes, lifetime: .process_lifetime_peak_rss_bytes}]' "$WORK/probe.json"

# --- ebr-chunk: calibrated rolling-hash mean, RAM paper rule id
"$BIN/ebr-chunk" dedup --item-path "$WORK/tree/sub/noise.bin" --experiment-id "$EXP" --env-name "$ENV_NAME" \
    --algorithm rabin --profile extreme > "$WORK/chunk-rabin.json"
check "$WORK/chunk-rabin.json" '(.payload.algorithm_id | contains("cut-mod-")) and .payload.mean_chunk_bytes > 0' "rabin mean-matched calibration and mean_chunk_bytes"
"$BIN/ebr-chunk" boundaries --item-path "$WORK/tree/sub/noise.bin" --experiment-id "$EXP" --env-name "$ENV_NAME" \
    --algorithm ram --profile extreme > "$WORK/chunk-ram.json"
check "$WORK/chunk-ram.json" '.payload.algorithm_id | startswith("ram-widodo2017-v2/")' "RAM implements the Widodo et al. rule"

# --- ebr-codec: one candidate per process with resources reported
"$BIN/ebr-codec" object-matrix --item-path "$WORK/tree/log.txt" --experiment-id "$EXP" --env-name "$ENV_NAME" \
    --candidate "ppmd7(external,7zip-level=9" --out "$WORK/codec.jsonl"
jq -s -e 'length == 1 and .[0].payload.threads == 1 and .[0].payload.window_bytes != null and .[0].payload.decode_memory.scoped == true' "$WORK/codec.jsonl" >/dev/null \
    || fail "codec candidate filter and resource fields"
echo "ok: codec candidate filter and resource fields"

# --- ebr-planner: production-chunked item, archive-level bounds
"$BIN/ebr-planner" --item-path "$WORK/planner-item.bin" --experiment-id "$EXP" --env-name "$ENV_NAME" \
    --profile balanced --max-chunks 2 --out "$WORK/planner.jsonl"
jq -s -e '([.[] | select(.payload.kind == "chunking")][0].payload | .chunk_count > 1 and .searched_chunk_count == 2 and .truncated) and ([.[] | select(.payload.kind == "archive_regret")] | length == 6) and ([.[] | select(.payload.kind == "archive_regret") | .payload.bounds.regret_upper_bound >= .payload.bounds.regret_lower_bound] | all) and ([.[] | select(.payload.differences != null) | (.payload.differences | length) == 0] | all)' "$WORK/planner.jsonl" >/dev/null \
    || fail "planner chunking, drift, and archive regret bounds"
echo "ok: planner chunking, drift, and archive regret bounds"

# --- held-out refusal in every binary, including with EB_DATA_ROOT overridden
expect_refusal() {
    local label="$1"; shift
    if out="$("$@" 2>&1)"; then fail "$label accepted a held-out path"; fi
    grep -q "refusing to read held-out path" <<<"$out" || fail "$label failed for another reason: $out"
    echo "ok: $label refuses a held-out path"
}
expect_refusal ebr-pack "$BIN/ebr-pack" --item-path "$HELDOUT_PROBE" --experiment-id "$EXP" --env-name "$ENV_NAME"
expect_refusal ebr-unpack "$BIN/ebr-unpack" --archive-path "$HELDOUT_PROBE" --experiment-id "$EXP" --env-name "$ENV_NAME"
expect_refusal ebr-verify "$BIN/ebr-verify" --archive-path "$HELDOUT_PROBE" --experiment-id "$EXP" --env-name "$ENV_NAME"
expect_refusal ebr-inspect-bytes "$BIN/ebr-inspect-bytes" --archive-path "$HELDOUT_PROBE" --experiment-id "$EXP" --env-name "$ENV_NAME"
expect_refusal ebr-access "$BIN/ebr-access" --mode roundtrip --item-path "$HELDOUT_PROBE" --experiment-id "$EXP" --env-name "$ENV_NAME"
expect_refusal ebr-chunk "$BIN/ebr-chunk" dedup --item-path "$WORK/tree/log.txt" --versions "$HELDOUT_PROBE" --experiment-id "$EXP" --env-name "$ENV_NAME"
expect_refusal ebr-codec "$BIN/ebr-codec" object-matrix --item-path "$HELDOUT_PROBE" --experiment-id "$EXP" --env-name "$ENV_NAME"
expect_refusal ebr-planner "$BIN/ebr-planner" --item-path "$HELDOUT_PROBE" --experiment-id "$EXP" --env-name "$ENV_NAME"
expect_refusal ebr-origin "$BIN/ebr-origin" --root "$HELDOUT_PROBE"
expect_refusal "ebr-pack with EB_DATA_ROOT overridden" env EB_DATA_ROOT="$WORK" "$BIN/ebr-pack" --item-path "$HELDOUT_PROBE" --experiment-id "$EXP" --env-name "$ENV_NAME"
expect_refusal "ebr-pack via a ../ path" "$BIN/ebr-pack" --item-path "/root/eb-research/corpus/../heldout/__ebr_guard_selftest_nonexistent__" --experiment-id "$EXP" --env-name "$ENV_NAME"

rm -rf "$WORK"
echo "SMOKE PASS (non-decision-grade)"
