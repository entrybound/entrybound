#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
/root/eb-research/venv/bin/python -B upstreams.py
/root/eb-research/venv/bin/python -B prefetch.py --jobs 6 "$@"
