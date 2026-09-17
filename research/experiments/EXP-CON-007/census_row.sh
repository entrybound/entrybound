#!/usr/bin/env bash
# EXP-CON-007 census row: build one (item, cell) archive set and print ONE JSON line (the ebr stdout_json
# source reads the last stdout line). Pre-registered design artifact.
#
#   census_row.sh --item-path DIR --out FILE --mode pack|convert-strict|convert-preserve
#                 [--profile P] [--layout indexed|stream] [--encrypt none|password]
#                 --ebound PATH --inspect-bytes PATH [--ebr-pack PATH] [--compat ID] [--max-inputs N] --env-name NAME
#
# pack: plain cells are packed by the production CLI (ebound-prod). The production CLI reads passwords only from
#   the controlling terminal, so encrypted cells are packed by the harness `ebr-pack --encrypt true` (fresh test
#   password in a sidecar), whose plain-archive output is SHA-256 identical to the CLI (harness-cli-parity.sh);
#   encrypted bytes are randomized by design and only their public feature words and byte accounting are recorded.
# convert-*: every ZIP-family member file of the item (*.zip, *.jar, *.war, *.whl, *.apk; sorted; at most
#   --max-inputs) is imported with `ebound convert` (strict, or --preserve --compat=<ID>); the row aggregates
#   the feature words, outcomes and byte accounting over the imported members.
set -euo pipefail
ITEM= OUT= MODE=pack PROFILE=balanced LAYOUT=indexed ENC=none EBOUND= IB= ENV=wsl-ubuntu
EBR_PACK=/root/eb-research/target/harness/release/ebr-pack
COMPAT=zip/python-zipfile@3.13.5
MAX_INPUTS=20
while [ $# -gt 0 ]; do
  case "$1" in
    --item-path) ITEM=$2; shift 2 ;;
    --out) OUT=$2; shift 2 ;;
    --mode) MODE=$2; shift 2 ;;
    --profile) PROFILE=$2; shift 2 ;;
    --layout) LAYOUT=$2; shift 2 ;;
    --encrypt) ENC=$2; shift 2 ;;
    --ebound) EBOUND=$2; shift 2 ;;
    --inspect-bytes) IB=$2; shift 2 ;;
    --ebr-pack) EBR_PACK=$2; shift 2 ;;
    --compat) COMPAT=$2; shift 2 ;;
    --max-inputs) MAX_INPUTS=$2; shift 2 ;;
    --env-name) ENV=$2; shift 2 ;;
    *) echo "unknown argument $1" >&2; exit 2 ;;
  esac
done
DIR=$(dirname "$OUT")

# Emit the census object for one archive ($1) packed with layout $2 and encryption $3 to stdout.
row_for_archive() {
  local a=$1 lay=$2 enc=$3 d
  d=$(dirname "$a")
  "$EBOUND" inspect "$a" --json > "$d/inspect.json" 2> "$d/inspect.stderr" || echo '{}' > "$d/inspect.json"
  local ib_args=(--archive-path "$a" --experiment-id EXP-CON-007 --env-name "$ENV" --layout "$lay")
  if [ "$enc" != none ]; then
    ib_args=(--archive-path "$a" --experiment-id EXP-CON-007 --env-name "$ENV" --layout indexed --encrypted true
             --unlock-password-file "$a.testpassword")
  fi
  "$IB" "${ib_args[@]}" 2> "$d/ib.stderr" | tail -n 1 > "$d/ib.json" || echo '{}' > "$d/ib.json"
  [ -s "$d/ib.json" ] || echo '{}' > "$d/ib.json"
  local pre_hex
  pre_hex=$(od -An -tx1 -j 16 -N 24 "$a" | tr -d ' \n')
  jq -c --slurpfile ib "$d/ib.json" --arg pre "$pre_hex" '{
    pack_outcome: "OK",
    incompat: (try .archive.features.incompat catch null),
    read_only_compat: (try .archive.features.read_only_compat catch null),
    compat: (try .archive.features.compat catch null),
    preamble_feature_words_hex: $pre,
    reconstruction_audit_count: (try (.reconstruction_audits | length) catch null),
    entry_count: (try .resources.entry_count catch null),
    chunk_count: (try .resources.chunk_count catch null),
    planner_id: ($ib[0].payload.counts.planner_id // null),
    artifact_bytes: ($ib[0].payload.artifact_bytes // null),
    reconstruction_bytes: ($ib[0].payload.byte_accounting.reconstruction_bytes // null),
    manifest_bytes: ($ib[0].payload.byte_accounting.manifest_bytes // null),
    metadata_bytes: ($ib[0].payload.byte_accounting.metadata_bytes // null),
    other_section_bytes: ($ib[0].payload.byte_accounting.other_section_bytes // null),
    index_bytes: ($ib[0].payload.byte_accounting.index_bytes // null)
  }' "$d/inspect.json"
}

first_code() { grep -o 'EB_[A-Z0-9_]*' "$1" | head -n 1 || true; }

if [ "$MODE" = pack ]; then
  PACK_ERR="$DIR/pack.stderr"
  outcome=OK
  if [ "$ENC" = none ]; then
    "$EBOUND" pack "$ITEM" "$OUT" --profile "$PROFILE" --layout "$LAYOUT" > "$DIR/pack.stdout" 2> "$PACK_ERR" || outcome=FAILED
  else
    "$EBR_PACK" --item-path "$ITEM" --experiment-id EXP-CON-007 --env-name "$ENV" --profile "$PROFILE" \
      --layout indexed --encrypt true --archive-out "$OUT" > "$DIR/pack.stdout" 2> "$PACK_ERR" || outcome=FAILED
  fi
  if [ "$outcome" != OK ]; then
    code=$(first_code "$PACK_ERR"); jq -cn --arg o "${code:-PACK_FAILED}" '{pack_outcome: $o}'
    exit 0
  fi
  lay=$LAYOUT; [ "$ENC" != none ] && lay=indexed
  row_for_archive "$OUT" "$lay" "$ENC"
  exit 0
fi

# convert modes
mapfile -t INPUTS < <(find "$ITEM" -type f \( -iname '*.zip' -o -iname '*.jar' -o -iname '*.war' -o -iname '*.whl' -o -iname '*.apk' \) -print | LC_ALL=C sort | head -n "$MAX_INPUTS")
if [ "${#INPUTS[@]}" -eq 0 ]; then
  jq -cn '{pack_outcome: "NO_INPUTS", converted: 0, refused: 0}'
  exit 0
fi
ROWS="$DIR/convert-rows.jsonl"; : > "$ROWS"
i=0
for f in "${INPUTS[@]}"; do
  i=$((i + 1)); sub="$DIR/c$i"; mkdir -p "$sub"; a="$sub/a.eb"
  if [ "$MODE" = convert-strict ]; then
    args=(convert "$f" "$a" --strict --layout "$LAYOUT" --profile "$PROFILE")
  else
    args=(convert "$f" "$a" --preserve "--compat=$COMPAT" --layout "$LAYOUT" --profile "$PROFILE")
  fi
  if "$EBOUND" "${args[@]}" > "$sub/convert.stdout" 2> "$sub/convert.stderr"; then
    row_for_archive "$a" "$LAYOUT" none >> "$ROWS"
  else
    code=$(first_code "$sub/convert.stderr"); jq -cn --arg o "${code:-CONVERT_FAILED}" '{pack_outcome: $o}' >> "$ROWS"
  fi
  rm -f "$a"
done
jq -cs '{
  pack_outcome: (if any(.[]; .pack_outcome == "OK") then "OK" else "ALL_REFUSED" end),
  converted: (map(select(.pack_outcome == "OK")) | length),
  refused: (map(select(.pack_outcome != "OK")) | length),
  refusal_codes: (map(select(.pack_outcome != "OK") | .pack_outcome) | unique | join(",")),
  incompat_values: (map(select(.pack_outcome == "OK") | (.incompat // 0) | tostring) | unique | join(",")),
  read_only_compat_values: (map(select(.pack_outcome == "OK") | (.read_only_compat // 0) | tostring) | unique | join(",")),
  compat_values: (map(select(.pack_outcome == "OK") | (.compat // 0) | tostring) | unique | join(",")),
  artifact_bytes: (map(.artifact_bytes // 0) | add),
  metadata_bytes: (map(.metadata_bytes // 0) | add),
  other_section_bytes: (map(.other_section_bytes // 0) | add),
  manifest_bytes: (map(.manifest_bytes // 0) | add),
  reconstruction_bytes: (map(.reconstruction_bytes // 0) | add),
  entry_count: (map(.entry_count // 0) | add)
}' "$ROWS"
