#!/usr/bin/env bash
# Authoring wrapper: run author_sources.py inside WSL with the research venv (see its docstring).
exec /root/eb-research/venv/bin/python -B "$(dirname "$0")/author_sources.py" "$@"
