#!/usr/bin/env bash
# memcap_run.sh -- run one command inside a fresh cgroup-v1 memory cgroup (WSL2 Ubuntu,
# root) and print exactly one JSON object describing its resource outcome.
#
# Scale-domain shared instrument (research/experiments/EXP-SCALE-*). Research-only.
#
# Usage:
#   memcap_run.sh --label ID [--cap BYTES|none] [--watch-mount DIR]... [--stdin FILE]
#                 [--stdout FILE] [--stderr FILE] [--cwd DIR] -- COMMAND [ARGS...]
#
# Semantics (pre-registered in EXP-SCALE-001 protocol.md section "Environment"):
#   * A cgroup /sys/fs/cgroup/memory/ebr-scale/<label>-<pid> is created. With --cap BYTES,
#     memory.limit_in_bytes AND memory.memsw.limit_in_bytes are both set to BYTES, so the
#     command cannot escape the cap through swap. With --cap none no limit is written
#     (accounting only). cgroup-v1 accounting includes page cache; the cap therefore bounds
#     anon + page cache, and page cache is reclaimable (see threats to validity).
#   * The command runs under GNU time -v inside the cgroup. max_rss_kib is GNU time's report
#     for the measured child (harness review R1-16: never wait4 of the runner).
#   * OOM kills are read from memory.oom_control (oom_kill counter) and from the signal GNU
#     time reports. When "signal" is non-null the command was killed and GNU time's
#     "exit_status" (reported as 0 in that case) must be ignored; drivers classify the sample
#     as KILLED_BY_SIGNAL / OOM_KILLED from (signal, oom_kill), never from exit_status.
#     wrapper_status is the status of the measuring subshell (128+signal when killed). The wrapper itself exits 0 whenever it could measure, whatever the command
#     did, so an ebr sample stays valid and the outcome is data. It exits 70 only for
#     infrastructure failures (cgroup cannot be created, GNU time missing).
#   * --watch-mount DIR (repeatable): the block device behind DIR (a dedicated loop mount) is
#     synced before and after; the delta of sectors written (/sys/block/<dev>/stat field 7)
#     x 512 is reported as device_write_bytes. This is the tolerance-0 write-accounting
#     primitive of decision-method.md section 4.14 for scratch and streaming claims.
set -u

LABEL=""
CAP="none"
STDIN_FILE="/dev/null"
STDOUT_FILE="/dev/null"
STDERR_FILE=""
CWD=""
WATCH=()
while [ $# -gt 0 ]; do
  case "$1" in
    --label) LABEL="$2"; shift 2 ;;
    --cap) CAP="$2"; shift 2 ;;
    --watch-mount) WATCH+=("$2"); shift 2 ;;
    --stdin) STDIN_FILE="$2"; shift 2 ;;
    --stdout) STDOUT_FILE="$2"; shift 2 ;;
    --stderr) STDERR_FILE="$2"; shift 2 ;;
    --cwd) CWD="$2"; shift 2 ;;
    --) shift; break ;;
    *) echo "memcap_run.sh: unknown argument $1" >&2; exit 64 ;;
  esac
done
[ -n "$LABEL" ] || { echo "memcap_run.sh: --label required" >&2; exit 64; }
[ $# -gt 0 ] || { echo "memcap_run.sh: no command" >&2; exit 64; }
[ -x /usr/bin/time ] || { echo "memcap_run.sh: /usr/bin/time (GNU time) missing" >&2; exit 70; }

ROOT=/sys/fs/cgroup/memory/ebr-scale
CG="$ROOT/${LABEL//[^A-Za-z0-9._-]/_}-$$"
mkdir -p "$ROOT" && mkdir "$CG" || { echo "memcap_run.sh: cannot create $CG" >&2; exit 70; }
cleanup() { rmdir "$CG" 2>/dev/null || true; }
trap cleanup EXIT

if [ "$CAP" != "none" ]; then
  echo "$CAP" > "$CG/memory.limit_in_bytes" || { echo "memcap_run.sh: cannot set limit" >&2; exit 70; }
  echo "$CAP" > "$CG/memory.memsw.limit_in_bytes" || { echo "memcap_run.sh: cannot set memsw limit" >&2; exit 70; }
fi

dev_writes() {  # $1 = mount dir -> sectors written on its backing block device
  local src dev
  src=$(findmnt -no SOURCE --target "$1" 2>/dev/null | head -1)
  dev=$(basename "$src")
  if [ -r "/sys/block/$dev/stat" ]; then awk '{print $7}' "/sys/block/$dev/stat"; else echo "null"; fi
}
declare -A W0
for m in "${WATCH[@]}"; do sync -f "$m" 2>/dev/null; W0["$m"]=$(dev_writes "$m"); done

TV=$(mktemp /tmp/memcap-timev.XXXXXX)
ERRF="${STDERR_FILE:-$(mktemp /tmp/memcap-stderr.XXXXXX)}"
T0=$(date +%s.%N)
(
  [ -n "$CWD" ] && cd "$CWD"
  echo $BASHPID > "$CG/cgroup.procs"
  exec /usr/bin/time -v -o "$TV" "$@" < "$STDIN_FILE" > "$STDOUT_FILE" 2> "$ERRF"
)
WRAP_STATUS=$?
T1=$(date +%s.%N)

WJSON=""
for m in "${WATCH[@]}"; do
  sync -f "$m" 2>/dev/null
  w1=$(dev_writes "$m"); w0=${W0["$m"]}
  if [ "$w0" = "null" ] || [ "$w1" = "null" ]; then v=null; else v=$(( (w1 - w0) * 512 )); fi
  WJSON="$WJSON${WJSON:+,}\"$m\":$v"
done

maxrss=$(awk -F': ' '/Maximum resident set size/ {print $2}' "$TV")
exitst=$(awk -F': ' '/Exit status/ {print $2}' "$TV")
sig=$(sed -n 's/.*Command terminated by signal \([0-9]*\).*/\1/p' "$TV" | head -1)
ucpu=$(awk -F': ' '/User time \(seconds\)/ {print $2}' "$TV")
scpu=$(awk -F': ' '/System time \(seconds\)/ {print $2}' "$TV")
maxuse=$(cat "$CG/memory.max_usage_in_bytes")
maxmemsw=$(cat "$CG/memory.memsw.max_usage_in_bytes" 2>/dev/null || echo null)
failcnt=$(cat "$CG/memory.failcnt")
oomkill=$(awk '/^oom_kill / {print $2}' "$CG/memory.oom_control")
stderr_tail=$(tail -c 2000 "$ERRF" | python3 -c 'import json,sys; print(json.dumps(sys.stdin.buffer.read().decode("utf-8","replace")))')
[ -z "$STDERR_FILE" ] && rm -f "$ERRF"
rm -f "$TV"

printf '{"schema":"ebr.scale.memcap.v1","label":"%s","cap_bytes":%s,"wrapper_status":%s,"exit_status":%s,"signal":%s,"max_rss_kib":%s,"user_s":%s,"sys_s":%s,"elapsed_s":%s,"cgroup_max_usage_bytes":%s,"cgroup_memsw_max_usage_bytes":%s,"cgroup_failcnt":%s,"oom_kill":%s,"device_write_bytes":{%s},"stderr_tail":%s}\n' \
  "$LABEL" "$( [ "$CAP" = none ] && echo null || echo "$CAP")" "$WRAP_STATUS" "${exitst:-null}" "${sig:-null}" \
  "${maxrss:-null}" "${ucpu:-null}" "${scpu:-null}" "$(awk -v a="$T0" -v b="$T1" 'BEGIN { printf "%.6f", b - a }')" "$maxuse" "$maxmemsw" "$failcnt" "${oomkill:-null}" \
  "$WJSON" "$stderr_tail"
exit 0
