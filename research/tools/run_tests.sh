#!/usr/bin/env bash
# Run the ebr pytest suite inside WSL with the pinned venv.
# Usage: wsl.exe -d Ubuntu -- bash /mnt/d/Projects/entrybound/entrybound/research/tools/run_tests.sh [pytest args]
set -euo pipefail
TOOLS=/mnt/d/Projects/entrybound/entrybound/research/tools
bash "$TOOLS/setup_venv.sh" >/dev/null
export PYTHONPATH="$TOOLS"
export PYTHONDONTWRITEBYTECODE=1
cd "$TOOLS"
exec /root/eb-research/venv/bin/python -m pytest -p no:cacheprovider -q tests "$@"
