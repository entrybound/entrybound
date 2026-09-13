#!/usr/bin/env bash
# Wrapper: run discover.py inside WSL with the research venv.  Authoring aid only.
exec /root/eb-research/venv/bin/python -B "$(dirname "$0")/discover.py" "$@"
