#!/usr/bin/env bash
# Host prerequisites for the g3-data corpus group (F05 logs/text, F06 structured text,
# F07 numeric arrays, F08 databases).  Run once inside WSL Ubuntu before provisioning:
#
#   wsl.exe -d Ubuntu -- bash /mnt/d/Projects/entrybound/entrybound/research/corpus/generators/g3-data/setup_prereqs.sh
#
# Idempotent: installs only missing Ubuntu packages, then prints the tool versions the
# g3-data recipes depend on.  The versions feed into TOFU output pins (for example
# osmium writes its version into the OSM XML header), so a changed version fails loudly
# at the pin check rather than silently producing different bytes.
set -euo pipefail

need=()
command -v osmium >/dev/null 2>&1 || need+=(osmium-tool)
command -v bzip2  >/dev/null 2>&1 || need+=(bzip2)
command -v unzip  >/dev/null 2>&1 || need+=(unzip)
if ((${#need[@]})); then
  echo "setup_prereqs: apt-get install ${need[*]}"
  export DEBIAN_FRONTEND=noninteractive
  apt-get update -qq
  apt-get install -y -qq "${need[@]}"
fi

VENV=/root/eb-research/venv
echo "osmium:  $(osmium --version 2>/dev/null | head -1)"
echo "libosmium: $(osmium --version 2>/dev/null | grep -i libosmium | head -1)"
echo "osmium-tool deb: $(dpkg -s osmium-tool | awk '/^Version/{print $2}')"
echo "bzip2:   $(bzip2 --help 2>&1 | head -1)"
echo "docker:  $(docker version --format 'client {{.Client.Version}} server {{.Server.Version}}' 2>/dev/null || echo missing)"
"$VENV/bin/python" - <<'PY'
import sqlite3, sys
import numpy, scipy
print("venv python:", sys.version.split()[0])
print("numpy:", numpy.__version__, "scipy:", scipy.__version__)
print("sqlite (venv python):", sqlite3.sqlite_version)
con = sqlite3.connect(":memory:")
opts = [r[0] for r in con.execute("PRAGMA compile_options")]
print("sqlite FTS5:", "ENABLE_FTS5" in opts, "RTREE:", "ENABLE_RTREE" in opts, "JSON:", con.execute("select json('{}')").fetchone()[0] == "{}")
PY
