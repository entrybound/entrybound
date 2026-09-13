#!/usr/bin/env bash
# Convert a downloaded OpenStreetMap PBF extract to OSM XML with osmium-tool (F06 run step).
#
# ebrc script contract:
#   bash osm_pbf_to_xml.sh --out <staging dir> --seed <int> --params <canonical JSON>
# params: {"input": input name (default "pbf"), "output": file name (default "extract.osm")}
# The input path comes from EB_INPUT_<NAME>.  Requires osmium-tool (setup_prereqs.sh);
# osmium writes its version into the XML <osm generator=...> attribute, so a different
# osmium version changes the bytes and fails the item's TOFU output pin loudly.
set -euo pipefail
OUT="" PARAMS="{}"
while [[ $# -gt 0 ]]; do
  case "$1" in
    --out) OUT=$2; shift 2 ;;
    --seed) shift 2 ;;
    --params) PARAMS=$2; shift 2 ;;
    *) echo "unknown arg $1" >&2; exit 2 ;;
  esac
done
command -v osmium >/dev/null || { echo "osmium not found: run research/corpus/generators/g3-data/setup_prereqs.sh" >&2; exit 2; }
NAME=$(python3 -c 'import json,sys; print(json.loads(sys.argv[1]).get("input","pbf"))' "$PARAMS")
OUTPUT=$(python3 -c 'import json,sys; print(json.loads(sys.argv[1]).get("output","extract.osm"))' "$PARAMS")
VAR="EB_INPUT_$(echo "$NAME" | tr 'a-z-' 'A-Z_')"
SRC="${!VAR:-}"
[[ -f "$SRC" ]] || { echo "input $NAME not available ($VAR)" >&2; exit 2; }
osmium --version
# cached inputs are named by SHA-256 without an extension: give the input format explicitly
INFO="${EB_SCRATCH:-/tmp}/osmium-fileinfo.txt"
osmium fileinfo -e -F pbf "$SRC" > "$INFO"
cat "$INFO"
osmium cat --no-progress -F pbf -f osm -o "$OUT/$OUTPUT" "$SRC"
chmod 0644 "$OUT/$OUTPUT"
