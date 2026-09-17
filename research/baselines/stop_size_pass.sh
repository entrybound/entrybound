#!/usr/bin/env bash
# Stop the detached EXP-BASE-SIZE size pass cleanly (launcher, ebr runner, and in-flight tool).
# Rows already written remain in research/raw/EXP-BASE-SIZE; rerunning run_size_pass.sh resumes with a new run id.
set -u
LOG=/root/eb-research/logs/baselines/size-pass.log
for pattern in "run_size_pass.sh" "run_baselines.py run" "-m ebr run"; do
  pkill -TERM -f -- "$pattern" 2>/dev/null
done
sleep 5
# In-flight measured tool processes started under the ebr scratch directory.
pkill -TERM -f -- "/root/eb-research/scratch/ebr/EXP-BASE-SIZE" 2>/dev/null
sleep 3
pkill -KILL -f -- "/root/eb-research/scratch/ebr/EXP-BASE-SIZE" 2>/dev/null
remaining=$(pgrep -af -- "run_size_pass.sh|run_baselines.py|-m ebr run|scratch/ebr/EXP-BASE-SIZE" | grep -v stop_size_pass | wc -l)
echo "[$(date -u +%FT%TZ)] stop_size_pass.sh: remaining matching processes=$remaining" | tee -a "$LOG"
touch /root/eb-research/logs/baselines/size-pass.paused
