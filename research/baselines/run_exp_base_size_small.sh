#!/usr/bin/env bash
# Executes the EXP-BASE-SIZE small-scale baseline pass end to end (run -> verify-raw
# -refingerprint -> normalize). Safe to re-invoke: `ebr run` appends a new run_id;
# rerun verify-raw/normalize afterwards to fold it in.
set -uo pipefail
cd /mnt/d/Projects/entrybound/entrybound
export PYTHONPATH=/mnt/d/Projects/entrybound/entrybound/research/tools
export PATH=/root/eb-research/tools/dwarfs:$PATH
PY=/root/eb-research/venv/bin/python
$PY research/baselines/run_baselines.py run --spec research/baselines/specs/EXP-BASE-SIZE-small.yaml
echo "RUN_BASELINES_EXIT=$?"
