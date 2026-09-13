#!/usr/bin/env bash
# Provision (or verify) every item defined in research/corpus/sources/g3-data.json.
#
#   wsl.exe -d Ubuntu -- bash /mnt/d/Projects/entrybound/entrybound/research/corpus/generators/g3-data/provision_g3.sh [provision args...]
#
# Selects the g3-data items by id (so items of other source files are untouched), runs
# setup_prereqs.sh first, and passes any extra arguments (--jobs, --split, --rebuild,
# --verify, --offline, --dry-run, ...) through to tools/run.sh provision.  Idempotent:
# provision skips items whose materialization key and fingerprint are unchanged and fails
# loudly (exit 1) on any hash mismatch.
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(cd "$HERE/../../../.." && pwd)"
SRC="$REPO/research/corpus/sources/g3-data.json"
bash "$HERE/setup_prereqs.sh" >/dev/null
FILTER="${G3_ITEM_FILTER:-}"
ids=$(python3 - "$SRC" "$FILTER" <<'PY'
import json, re, sys
items = json.load(open(sys.argv[1]))["items"]
pat = re.compile(sys.argv[2]) if sys.argv[2] else None
print(",".join(i["item_id"] for i in items if not pat or pat.search(i["item_id"])))
PY
)
[[ -n "$ids" ]] || { echo "provision_g3: no items selected" >&2; exit 2; }
exec bash "$REPO/research/corpus/tools/run.sh" provision --item "$ids" "$@"
