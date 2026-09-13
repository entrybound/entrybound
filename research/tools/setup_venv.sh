#!/usr/bin/env bash
# Install the hash-locked ebr requirements into /root/eb-research/venv.
# Idempotent: skips when the venv already records the same requirements.txt digest
# and `pip check` passes. Fails loudly on any hash mismatch (pip --require-hashes).
set -euo pipefail

HERE=/mnt/d/Projects/entrybound/entrybound/research/tools
ROOT=/root/eb-research
VENV="$ROOT/venv"
WHEELS="$ROOT/cache/wheels/ebr"
REQ="$HERE/requirements.txt"
MAN="$HERE/requirements-downloads.json"
STAMP="$VENV/.ebr-requirements.sha256"

[[ -s "$REQ" ]] || { echo "setup_venv: missing $REQ (run lock_requirements.sh)" >&2; exit 2; }
mkdir -p "$ROOT/cache" "$WHEELS"
exec 9>"$ROOT/venv.lock"
flock 9

[[ -x "$VENV/bin/python" ]] || python3 -m venv "$VENV"

want=$(sha256sum "$REQ" | cut -d' ' -f1)
if [[ -f "$STAMP" && "$(cat "$STAMP")" == "$want" ]] && "$VENV/bin/python" -m pip check >/dev/null 2>&1 \
   && "$VENV/bin/python" -c 'import numpy, scipy, matplotlib, pandas, yaml, psutil, pytest' 2>/dev/null; then
  echo "setup_venv: up to date ($want)"
  exit 0
fi

# Verify / fetch cached wheels against the manifest before installing.
python3 - "$MAN" "$WHEELS" <<'PY'
import hashlib, json, os, sys, urllib.request
man, wheels = sys.argv[1], sys.argv[2]
for f in json.load(open(man))["files"]:
    path = os.path.join(wheels, f["filename"])
    if not os.path.isfile(path):
        print("fetch", f["url"])
        tmp = path + ".part"
        urllib.request.urlretrieve(f["url"], tmp)
        os.replace(tmp, path)
    h = hashlib.sha256(open(path, "rb").read()).hexdigest()
    size = os.path.getsize(path)
    if h != f["sha256"] or size != f["size_bytes"]:
        sys.exit(f"HASH/SIZE MISMATCH for {path}: {h} {size} != {f['sha256']} {f['size_bytes']}")
print("setup_venv: cached wheels verified")
PY

"$VENV/bin/python" -m pip install --disable-pip-version-check --no-index --find-links "$WHEELS" \
  --require-hashes --no-deps -r "$REQ"
"$VENV/bin/python" -m pip check
echo "$want" > "$STAMP"
echo "setup_venv: installed ($want)"
