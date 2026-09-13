#!/usr/bin/env bash
# Build/derive script (g4-binary F16): convert a disk image between formats with qemu-img.
#
#   bash convert_image.sh --out <staging> --seed N --params <JSON>
# params: exactly one source selector:
#           from_item + src   (derive: file <src> inside the from_items dependency)
#           input             (build: a declared download input)
#         src_format (raw|qcow2|vmdk|...), dst (output file name), dst_format (raw|qcow2|vmdk),
#         options (qemu-img -o string, optional)
# Output: <out>/<dst>.  qemu-img convert runs single-coroutine (-m 1) without out-of-order writes;
# raw output is sparse and additionally normalized with fallocate --dig-holes; the result is checked
# with qemu-img compare against the source.
set -euo pipefail
OUT= PARAMS='{}'
while [[ $# -gt 0 ]]; do
  case $1 in
    --out) OUT=$2; shift 2 ;;
    --seed) shift 2 ;;
    --params) PARAMS=$2; shift 2 ;;
    *) echo "unknown argument $1" >&2; exit 2 ;;
  esac
done
[[ -n $OUT && -d $OUT ]] || { echo "--out must be an existing directory" >&2; exit 2; }
if jq -e 'has("from_item")' <<<"$PARAMS" >/dev/null; then
  base=$(jq -er --arg k "$(jq -er .from_item <<<"$PARAMS")" '.[$k]' <<<"$EB_FROM_JSON")
  SRC="$base/$(jq -er .src <<<"$PARAMS")"
else
  SRC=$(jq -er --arg k "$(jq -er .input <<<"$PARAMS")" '.[$k]' <<<"$EB_INPUTS_JSON")
fi
[[ -f $SRC ]] || { echo "source image $SRC missing" >&2; exit 1; }
SF=$(jq -er .src_format <<<"$PARAMS")
DF=$(jq -er .dst_format <<<"$PARAMS")
DST="$OUT/$(jq -er .dst <<<"$PARAMS")"
OPTS=$(jq -r '.options // empty' <<<"$PARAMS")
args=(convert -p -m 1 -f "$SF" -O "$DF")
[[ -n $OPTS ]] && args+=(-o "$OPTS")
[[ $DF == raw ]] && args+=(-S 4k)
qemu-img "${args[@]}" "$SRC" "$DST" </dev/null | tr '\r' '\n' | tail -1
[[ $DF == raw ]] && fallocate --dig-holes "$DST"
qemu-img compare -f "$SF" -F "$DF" "$SRC" "$DST"
chmod 0644 "$DST"
{
  echo "source $(basename "$SRC") format $SF sha256 $(sha256sum "$SRC" | cut -d' ' -f1)"
  echo "output $(basename "$DST") format $DF options ${OPTS:-default}"
  qemu-img --version | head -1
} > "$OUT/CONVERTINFO.txt"
chmod 0644 "$OUT/CONVERTINFO.txt"
echo "convert_image: done"
