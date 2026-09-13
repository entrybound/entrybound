#!/usr/bin/env bash
# Development check for g4-binary scripts: run a generator twice in a provision-like sanitized
# environment (two different scratch paths) and compare output digests (content, modes, sparse map).
#   bash test_generators.sh <name> [<name>...]   names: blobs nested office cli fat gpt ext4 qcow2 prune
# Uses /root/eb-research/scratch/g4-binary-test; never touches corpus or held-out directories.
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
T=/root/eb-research/scratch/g4-binary-test
PY=/root/eb-research/venv/bin/python
mkdir -p "$T/inputs"
umask 022

digest() {  # digest <dir>: sha256 over (path, type, mode, size, sha256, extent map)
  (cd "$1" && find . -mindepth 1 -printf '%P\t%y\t%m\t%s\n' | LC_ALL=C sort | while IFS=$'\t' read -r pth typ mode size; do
     if [[ $typ == f ]]; then
       h=$(sha256sum "$pth" | cut -c1-16)
       ext=$($PY -c 'import os,sys
fd=os.open(sys.argv[1],os.O_RDONLY); n=os.fstat(fd).st_size; o=0; ex=[]
while o<n:
  try: d=os.lseek(fd,o,os.SEEK_DATA)
  except OSError: break
  h=os.lseek(fd,d,os.SEEK_HOLE); ex.append((d,h)); o=h
print(len(ex), sum(b-a for a,b in ex))' "$pth")
       printf '%s\t%s\t%s\t%s\t%s\t%s\n' "$pth" "$typ" "$mode" "$size" "$h" "$ext"
     else
       printf '%s\t%s\t%s\n' "$pth" "$typ" "$mode"
     fi
   done)
}

runone() {  # runone <label> <interp> <script> <seed> <params> [inputs-json] [from-json]
  local label=$1 interp=$2 script=$3 seed=$4 params=$5 inputs=${6:-'{}'} from=${7:-'{}'}
  for r in a b; do
    local out="$T/$label-$r/out" scr="$T/$label-$r/scratch-$RANDOM$RANDOM"
    rm -rf "$T/$label-$r"; mkdir -p "$out" "$scr"
    env -i PATH=/root/eb-research/venv/bin:/root/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin \
      HOME="$scr" LC_ALL=C.UTF-8 LANG=C.UTF-8 TZ=UTC SOURCE_DATE_EPOCH=1767225600 PYTHONHASHSEED=0 TMPDIR="$scr" \
      RUSTUP_HOME=/root/.rustup EB_SCRATCH="$scr" EB_CACHE=/root/eb-research/cache EB_INPUTS_JSON="$inputs" EB_FROM_JSON="$from" \
      bash -c "cd '$scr' && $interp '$script' --out '$out' --seed $seed --params '$params'" > "$T/$label-$r.log" 2>&1 \
      || { echo "FAIL $label run $r"; tail -30 "$T/$label-$r.log"; return 1; }
    digest "$out" > "$T/$label-$r.digest"
  done
  if cmp -s "$T/$label-a.digest" "$T/$label-b.digest"; then
    echo "OK   $label deterministic: $(wc -l < "$T/$label-a.digest") entries, $(du -sb --apparent-size "$T/$label-a/out" | cut -f1) bytes apparent, $(du -sb "$T/$label-a/out" | cut -f1) allocated"
  else
    echo "DIFF $label"; diff "$T/$label-a.digest" "$T/$label-b.digest" | head -20
  fi
}

fetch() {  # fetch <url> <file>
  [[ -f "$T/inputs/$2" ]] || curl -fsSL -o "$T/inputs/$2" "$1"
  echo "$T/inputs/$2"
}

for name in "$@"; do
  case $name in
    blobs) runone blobs "$PY" "$HERE/gen_redundant_blobs.py" 1009 '{"profile":"mixed-v1"}' ;;
    nested) runone nested "$PY" "$HERE/gen_nested_archives.py" 1409 '{"profile":"mixed-v1"}' ;;
    office) runone office "$PY" "$HERE/gen_office_docs.py" 1419 '{"profile":"office-v1"}' ;;
    cli) runone cli bash "$HERE/gen_nested_cli.sh" 1499 '{"profile":"cli-v1"}' ;;
    fat) runone fat "$PY" "$HERE/gen_fat_image.py" 1609 '{"fat":32,"label":"EBFAT32","size_mib":64,"volume_id":"1f16c0de"}' ;;
    gpt) runone gpt "$PY" "$HERE/gen_gpt_disk.py" 1699 '{"disk_guid":"a3c1e7d2-5b4f-4e61-8d2a-9f0b1c2d3e4f","esp_mib":48,"size_mib":192}' ;;
    ext4)
      m=$(fetch https://dl-cdn.alpinelinux.org/alpine/v3.24/releases/x86_64/alpine-minirootfs-3.24.1-x86_64.tar.gz mini.tar.gz)
      runone ext4 bash "$HERE/build_ext4_image.sh" 0 '{"block_size":4096,"image":"a.img","label":"alpine-root","size_mib":256,"uuid":"6c3b3c1e-4f0a-4b8e-9a53-2f5d0e1a4b16"}' "{\"minirootfs\":\"$m\"}" ;;
    qcow2)
      runone qcow2 bash "$HERE/convert_image.sh" 0 '{"dst":"a.qcow2","dst_format":"qcow2","from_item":"x","options":"compat=1.1,cluster_size=65536,lazy_refcounts=off","src":"a.img","src_format":"raw"}' '{}' "{\"x\":\"$T/ext4-a/out\"}" ;;
    prune)
      mkdir -p "$T/prune-src/usr/bin" "$T/prune-src/usr/lib" "$T/prune-src/etc" && echo x > "$T/prune-src/usr/bin/a" && ln -sf usr/bin "$T/prune-src/bin" && echo y > "$T/prune-src/etc/z"
      rm -rf "$T/prune-out"; cp -a "$T/prune-src" "$T/prune-out"
      $PY "$HERE/prune_tree.py" --out "$T/prune-out" --seed 0 --params '{"keep":["bin","usr/bin","usr/lib"],"require":["usr/bin"]}'
      find "$T/prune-out" | sort ;;
    *) echo "unknown $name" ;;
  esac
done
