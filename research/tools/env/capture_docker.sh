#!/usr/bin/env bash
# Capture the pinned ubuntu:24.04 Docker container fingerprint (see capture_docker.py).
#
# Invoke from Windows after capture of windows-host:
#   wsl.exe -d Ubuntu -- bash /mnt/d/Projects/entrybound/entrybound/research/tools/env/capture_docker.sh
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec /usr/bin/python3 "$HERE/capture_docker.py" "$@"
