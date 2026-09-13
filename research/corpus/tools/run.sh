#!/usr/bin/env bash
# Entry point for the corpus tools inside WSL Ubuntu.
#
#   wsl.exe -d Ubuntu -- bash /mnt/d/Projects/entrybound/entrybound/research/corpus/tools/run.sh <tool> [args...]
#
# tools: provision | stats | assemble | fingerprint | selftest
# Ensures the data-root layout and /root/eb-research/venv exist (numpy for stats comes from the
# shared hash-locked research/tools/setup_venv.sh).  Idempotent.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(cd "$HERE/../../.." && pwd)"
SHARED=/root/eb-research
VENV="$SHARED/venv"
DATA="${EB_DATA_ROOT:-$SHARED}"

usage() { echo "usage: run.sh {provision|stats|assemble|fingerprint|selftest} [args...]" >&2; exit 2; }
[[ $# -ge 1 ]] || usage
tool=$1; shift

umask 022
mkdir -p "$SHARED/cache" "$DATA/corpus/tuning" "$DATA/corpus/validation" "$DATA/heldout" "$DATA/cache" "$DATA/locks"
chmod 700 "$DATA/heldout"

if [[ ! -x "$VENV/bin/python" ]]; then
  exec 9>"$SHARED/venv.lock"
  flock 9
  [[ -x "$VENV/bin/python" ]] || python3 -m venv "$VENV"
  exec 9>&-
fi
if [[ "$tool" == "stats" || "$tool" == "selftest" ]] && ! "$VENV/bin/python" -c 'import numpy' 2>/dev/null; then
  if [[ -f "$REPO/research/tools/setup_venv.sh" ]]; then
    bash "$REPO/research/tools/setup_venv.sh"
  else
    echo "run.sh: numpy missing in $VENV and research/tools/setup_venv.sh not found" >&2
    exit 2
  fi
fi

export PYTHONDONTWRITEBYTECODE=1
case "$tool" in
  provision|stats|assemble|fingerprint) exec "$VENV/bin/python" -B "$HERE/$tool.py" "$@" ;;
  selftest) exec bash "$HERE/selftest.sh" "$@" ;;
  *) usage ;;
esac
