#!/usr/bin/env bash
# Resolve and hash-lock the ebr Python dependencies (one-time; rerun with --relock).
# Writes research/tools/requirements.txt (exact pins + sha256) and
# research/tools/requirements-downloads.json (URL, size, sha256, retrieval date).
# Wheels are cached outside git at /root/eb-research/cache/wheels/ebr.
set -euo pipefail

HERE=/mnt/d/Projects/entrybound/entrybound/research/tools
ROOT=/root/eb-research
VENV="$ROOT/venv"
WHEELS="$ROOT/cache/wheels/ebr"
REQ="$HERE/requirements.txt"
MAN="$HERE/requirements-downloads.json"
RETRIEVED=2026-09-12
TOP=(numpy scipy matplotlib pandas pyyaml psutil pytest)

if [[ -s "$REQ" && "${1:-}" != "--relock" ]]; then
  echo "lock_requirements: $REQ already exists; pass --relock to re-resolve"
  exit 0
fi

mkdir -p "$ROOT/cache" "$WHEELS"
exec 9>"$ROOT/venv.lock"
flock 9

if [[ ! -x "$VENV/bin/python" ]]; then
  python3 -m venv "$VENV"
fi

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
"$VENV/bin/python" -m pip install --disable-pip-version-check --dry-run --ignore-installed \
  --only-binary=:all: --report "$TMP/report.json" "${TOP[@]}" >/dev/null
# Download exactly what the resolver selected.
python3 - "$TMP/report.json" > "$TMP/urls.txt" <<'PY'
import json, sys
for inst in json.load(open(sys.argv[1]))["install"]:
    print(inst["download_info"]["url"])
PY
mkdir -p "$TMP/w"
while read -r url; do
  fname=$(python3 -c 'import sys,os,urllib.parse as u; print(u.unquote(os.path.basename(u.urlparse(sys.argv[1]).path)))' "$url")
  if [[ -f "$WHEELS/$fname" ]]; then
    cp "$WHEELS/$fname" "$TMP/w/$fname"
  else
    curl -fsSL --retry 3 -o "$TMP/w/$fname" "$url"
  fi
done < "$TMP/urls.txt"

python3 "$HERE/lockgen.py" --report "$TMP/report.json" --wheel-dir "$TMP/w" \
  --requirements "$REQ" --manifest "$MAN" --retrieved "$RETRIEVED" --top "${TOP[@]}"
cp --update=none "$TMP"/w/* "$WHEELS/"
echo "lock_requirements: wrote $REQ and $MAN"
