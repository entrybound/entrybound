#!/usr/bin/env bash
# Show the tail of the newest g4-binary provision log (no content reads).
log=$(ls -t /root/eb-research/logs/g4-binary/provision-*.log 2>/dev/null | head -1)
[[ -n $log ]] || { echo "no log"; exit 0; }
echo "$log"
grep -vE 'download .* \(attempt' "$log" | tail -n "${1:-40}"
