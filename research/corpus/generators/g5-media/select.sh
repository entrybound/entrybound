#!/usr/bin/env bash
# Wrapper: run the g5-media source selection helper inside WSL with the ebrc venv.
#   wsl.exe -d Ubuntu -- bash /mnt/d/Projects/entrybound/entrybound/research/corpus/generators/g5-media/select.sh <select|fetch|emit|all> [--pool P] [--refresh P]
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
umask 022
export PYTHONDONTWRITEBYTECODE=1
exec /root/eb-research/venv/bin/python -B "$HERE/select_sources.py" "$@"
