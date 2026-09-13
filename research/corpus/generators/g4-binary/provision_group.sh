#!/usr/bin/env bash
# Provision every g4-binary family (F09 F10 F11 F14 F16) with the corpus framework and keep a log.
#   bash provision_group.sh [extra provision args...]      e.g. --split tuning  --rebuild  --item <id>
# Idempotent: already-materialized items are verified, not rebuilt (see provision.py).
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RUN="$HERE/../../tools/run.sh"
LOG_DIR=/root/eb-research/logs/g4-binary
mkdir -p "$LOG_DIR"
log="$LOG_DIR/provision-$(date -u +%Y%m%dT%H%M%SZ).log"
echo "log: $log"
if [[ " $* " == *" --item "* ]]; then
  bash "$RUN" provision --jobs 3 --hash-jobs 4 "$@" 2>&1 | tee "$log"
else
  bash "$RUN" provision --family F09,F10,F11,F14,F16 --jobs 3 --hash-jobs 4 "$@" 2>&1 | tee "$log"
fi
exit "${PIPESTATUS[0]}"
