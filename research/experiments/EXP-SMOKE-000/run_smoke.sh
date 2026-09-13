#!/usr/bin/env bash
# End-to-end smoke of the ebr pipeline inside WSL:
#   setup venv -> validate -> run (skipped if a verified run exists) -> verify-raw
#   -> normalize -> report -> regenerate normalize/report and require byte-identical output.
# Usage: wsl.exe -d Ubuntu -- bash /mnt/d/Projects/entrybound/entrybound/research/experiments/EXP-SMOKE-000/run_smoke.sh [--force-run]
set -euo pipefail

REPO=/mnt/d/Projects/entrybound/entrybound
TOOLS="$REPO/research/tools"
SPEC=research/experiments/EXP-SMOKE-000/spec.yaml
EXP=EXP-SMOKE-000
PY=/root/eb-research/venv/bin/python

bash "$TOOLS/setup_venv.sh"
export PYTHONPATH="$TOOLS"
export PYTHONDONTWRITEBYTECODE=1
cd "$REPO"

"$PY" -m ebr validate --spec "$SPEC"
"$PY" -m ebr plan --spec "$SPEC"

have_runs=$(ls research/raw/$EXP/*.meta.json 2>/dev/null | wc -l || true)
if [[ "${1:-}" == "--force-run" || "$have_runs" == "0" ]]; then
  "$PY" -m ebr run --spec "$SPEC"
else
  echo "run_smoke: $have_runs existing run(s); skipping run (pass --force-run to add one)"
fi

"$PY" -m ebr verify-raw --spec "$SPEC" --refingerprint
"$PY" -m ebr normalize --spec "$SPEC" >/dev/null
"$PY" -m ebr report --spec "$SPEC" >/dev/null

snapshot() {
  (cd "$REPO/research" && find "normalized/$EXP" "reports/tables/$EXP" "reports/figures/$EXP" -type f -print0 \
     | sort -z | xargs -0 sha256sum)
}
first=$(snapshot)
"$PY" -m ebr normalize --spec "$SPEC" >/dev/null
"$PY" -m ebr report --spec "$SPEC" >/dev/null
second=$(snapshot)
if [[ "$first" != "$second" ]]; then
  echo "run_smoke: DETERMINISM FAILURE: regenerated outputs differ" >&2
  diff <(echo "$first") <(echo "$second") >&2 || true
  exit 1
fi
echo "run_smoke: regenerated normalized/report outputs are byte-identical ($(echo "$first" | wc -l) files)"

# Semantic checks on the summary (all samples ok, round trips pass, deterministic gzip output).
"$PY" - <<'PY'
import json, sys
s = json.load(open("research/normalized/EXP-SMOKE-000/summary.json"))
c = s["counts"]
assert c["measured"] > 0 and c["failed"] == 0 and c["invalid"] == 0, c
for key, digests in s["string_metrics"]["output_sha256"].items():
    assert len(digests) == 1, f"non-deterministic gzip output for {key}: {digests}"
for env, cands in s["headline"].items():
    for cand, metrics in cands.items():
        assert metrics["roundtrip_ok"]["median"] == 1, (cand, metrics["roundtrip_ok"])
print("run_smoke: semantic checks passed:", json.dumps(c))
PY
