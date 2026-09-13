#!/usr/bin/env bash
# Authoring aid (tuning input only): sum member sizes of linux-firmware directories (two levels) so the
# F10 tuning subset stays inside the medium tier.  Reads the tar index, not file contents.
set -euo pipefail
blob=/root/eb-research/cache/sha256/f3937ca282ba256242e2b6dbe523df8a80007d29ffd61f56d270190865492ea8
[[ -f $blob ]] || { echo "firmware tarball not cached yet"; exit 0; }
idx=/root/eb-research/scratch/g4-binary-authoring/firmware-list.txt
[[ -s $idx ]] || tar -tvJf "$blob" > "$idx"
awk '{ size=$3; path=$6; sub(/^[^\/]*\//, "", path); n=split(path, a, "/");
       key=(n>2 ? a[1] "/" a[2] : (n==2 ? a[1] "/" : path)); s[key]+=size; c[key]++ }
     END { for (k in s) printf "%d\t%d\t%s\n", s[k], c[k], k }' "$idx" | sort -rn |
  grep -E "^[0-9]+\s+[0-9]+\s+(${1:-intel|amdgpu|i915|xe|mediatek|ath1|rtw|brcm|radeon|nvidia|rtl)}" | head -60
