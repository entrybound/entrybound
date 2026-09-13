#!/usr/bin/env bash
# Provision g1-code items (F01, F02, F03, F18) through the corpus framework; idempotent.
#   provision_g1.sh [extra provision args, e.g. --split tuning --item <id>]
# Build/vendor recipes need the apt packages of setup_build_deps.sh; it is run first (no-op when present).
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(cd "$HERE/../../../.." && pwd)"
bash "$HERE/setup_build_deps.sh" >/dev/null
mkdir -p /root/eb-research/logs
log=/root/eb-research/logs/g1-code-provision-$(date -u +%Y%m%dT%H%M%SZ)-$$.log
echo "log: $log"
cd "$REPO"
set +e
bash research/corpus/tools/run.sh provision --family F01,F02,F03,F18 "$@" 2>&1 | tee "$log"
rc=${PIPESTATUS[0]}
exit "$rc"
