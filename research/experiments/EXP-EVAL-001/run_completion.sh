#!/usr/bin/env bash
# EXP-EVAL-001 detached, agent-free runner (protocol.md "Commands").
# Timing is disabled for every spec, so shards of one group run in parallel.
#
# Launch from Windows (after research/orchestration/disk_guard.py passes):
#   wsl.exe -d Ubuntu -- bash -lc 'setsid nohup bash /mnt/d/Projects/entrybound/entrybound/research/experiments/EXP-EVAL-001/run_completion.sh >/dev/null 2>&1 &'
# Progress: /root/eb-research/logs/EXP-EVAL-001/*.log ; markers: done / failed / waiting in the same directory.
set -uo pipefail
REPO=/mnt/d/Projects/entrybound/entrybound
PY=/root/eb-research/venv/bin/python
LOG=/root/eb-research/logs/EXP-EVAL-001
BASE_LOG=/root/eb-research/logs/baselines
mkdir -p "$LOG"
rm -f "$LOG/done" "$LOG/failed"
exec >>"$LOG/driver.log" 2>&1
cd "$REPO"
export PYTHONPATH=research/tools
export PATH=/root/eb-research/tools/dwarfs:$PATH
note() { echo "[$(date -u +%FT%TZ)] $*"; }

# Pre-registered precondition: the running EXP-BASE-SIZE medium sample finishes first,
# so the two passes never overlap on the same (config, item) pairs or compete for scratch.
while [ ! -e "$BASE_LOG/size-pass.done" ] && [ ! -e "$BASE_LOG/size-pass.failed" ]; do
  touch "$LOG/waiting"; sleep 600
done
rm -f "$LOG/waiting"
[ -e "$BASE_LOG/size-pass.failed" ] && note "EXP-BASE-SIZE reported failure; continuing (its gaps are reported, not re-run here)"

guard() {  # $1 = minimum free GiB on the WSL root, $2 = minimum MemAvailable GiB
  local free_gb mem_gb
  free_gb=$(df --output=avail -BG / | tail -1 | tr -dc '0-9')
  mem_gb=$(awk '/MemAvailable/ {print int($2/1048576)}' /proc/meminfo)
  [ "${free_gb:-0}" -ge "$1" ] && [ "${mem_gb:-0}" -ge "$2" ]
}

run_spec() {  # $1 spec path
  local spec=$1 id
  id=$(basename "$spec" .yaml)
  note "start $id"
  $PY -m ebr validate --spec "$spec" >>"$LOG/$id.log" 2>&1 || return 2
  $PY -m ebr run --gzip --spec "$spec" >>"$LOG/$id.log" 2>&1
  local rc=$?
  $PY -m ebr verify-raw --refingerprint --allow-incomplete --spec "$spec" >>"$LOG/$id.log" 2>&1
  $PY -m ebr normalize --include-incomplete --spec "$spec" >>"$LOG/$id.log" 2>&1
  note "end $id run_exit=$rc"
  [ "$rc" -le 1 ]
}

run_group() {  # $1 group name, $2 min free GiB, $3 min MemAvailable GiB
  local pids=() s
  for s in research/experiments/EXP-EVAL-001/shards/EXP-EVAL-001-"$1"-s*.yaml; do
    until guard "$2" "$3"; do note "guard wait before $s"; sleep 300; done
    run_spec "$s" & pids+=($!)
    sleep 60
  done
  local ok=0 p
  for p in "${pids[@]}"; do wait "$p" || ok=1; done
  return $ok
}

guard 100 8 || { note "refusing to start: <100 GiB free or <8 GiB MemAvailable"; touch "$LOG/failed"; exit 1; }
status=0
run_spec research/experiments/EXP-EVAL-001/spec.yaml || status=1
run_group medlo 100 8 || status=1
run_group medhi 100 10 || status=1
run_group large 120 12 || status=1
if [ $status -eq 0 ]; then touch "$LOG/done"; else touch "$LOG/failed"; fi
note "finished status=$status"
