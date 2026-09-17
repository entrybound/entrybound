#!/usr/bin/env bash
# Detached, agent-free continuation of the EXP-BASE-SIZE incumbent size/determinism pass.
# Runs the small tier (all tuning+validation small items) and then a seeded medium sample,
# timing disabled. Logs go outside git; raw rows land in research/raw/EXP-BASE-SIZE via ebr.
#
# Launch (from Windows):
#   wsl.exe -d Ubuntu -- bash -lc 'setsid nohup bash /mnt/d/Projects/entrybound/entrybound/research/baselines/run_size_pass.sh >/dev/null 2>&1 &'
# Progress: /root/eb-research/logs/baselines/size-pass.log ; completion marker: size-pass.done / size-pass.failed
set -uo pipefail
REPO=/mnt/d/Projects/entrybound/entrybound
PY=/root/eb-research/venv/bin/python
LOG_DIR=/root/eb-research/logs/baselines
STAMP=$(date -u +%Y%m%dT%H%M%SZ)
mkdir -p "$LOG_DIR"
rm -f "$LOG_DIR/size-pass.done" "$LOG_DIR/size-pass.failed"
exec >>"$LOG_DIR/size-pass.log" 2>&1
cd "$REPO"
export PYTHONPATH=research/tools

step() {
  echo "[$(date -u +%FT%TZ)] $*"
  "$@"
  local rc=$?
  echo "[$(date -u +%FT%TZ)] exit=$rc: $*"
  return $rc
}

free_d_gb=$(df --output=avail -BG / | tail -1 | tr -dc '0-9')
if [ "${free_d_gb:-0}" -lt 100 ]; then
  echo "refusing to start: WSL root (hosted on D:) has ${free_d_gb}G free (<100G)"
  touch "$LOG_DIR/size-pass.failed"
  exit 1
fi

step $PY research/baselines/run_baselines.py spec --scales small \
  --out research/baselines/specs/EXP-BASE-SIZE-small.yaml || { touch "$LOG_DIR/size-pass.failed"; exit 1; }
step $PY research/baselines/run_baselines.py run --spec research/baselines/specs/EXP-BASE-SIZE-small.yaml \
  --gzip --run-id "size-small-$STAMP" || { touch "$LOG_DIR/size-pass.failed"; exit 1; }
step $PY research/baselines/run_baselines.py spec --scales medium --medium-sample 20 \
  --out research/baselines/specs/EXP-BASE-SIZE-medium-sample.yaml || { touch "$LOG_DIR/size-pass.failed"; exit 1; }
step $PY research/baselines/run_baselines.py run --spec research/baselines/specs/EXP-BASE-SIZE-medium-sample.yaml \
  --gzip --run-id "size-medium-$STAMP" || { touch "$LOG_DIR/size-pass.failed"; exit 1; }
touch "$LOG_DIR/size-pass.done"
