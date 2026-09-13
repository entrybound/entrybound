#!/usr/bin/env bash
# Follow the newest g4-binary provision log, printing only result lines (for monitoring).
log=$(ls -t /root/eb-research/logs/g4-binary/provision-*.log 2>/dev/null | head -1)
[[ -n $log ]] || { echo "no log"; exit 1; }
tail -n +1 -F "$log" | grep --line-buffered -E "materialized logical|up-to-date|FAILED|HASH MISMATCH|WARNING|provision: |Traceback|rror"
