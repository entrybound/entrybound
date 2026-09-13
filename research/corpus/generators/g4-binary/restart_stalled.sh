#!/usr/bin/env bash
# Operational helper: stop a g4-binary provision run whose download connection is stalled and remove
# only that run's temporary files (partial download, staging and scratch dirs of the named item).
#   bash restart_stalled.sh <pid> <item_id> <download-part-glob-prefix>
set -euo pipefail
pid=$1 item=$2 part=$3
cmd=$(tr '\0' ' ' < "/proc/$pid/cmdline" || true)
[[ $cmd == *provision.py* ]] || { echo "pid $pid is not provision.py: $cmd" >&2; exit 1; }
kill -9 "$pid"
sleep 1
rm -f /root/eb-research/cache/tmp/"$part"*.part
rm -rf /root/eb-research/staging/"$item".* /root/eb-research/scratch/"$item".*
echo "stopped $pid; cleaned temporary files for $item"
