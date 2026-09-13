#!/usr/bin/env bash
# Build script (kind build, g4-binary F16): raw sparse ext4 image populated from a root filesystem tarball.
#
#   bash build_ext4_image.sh --out <staging> --seed N --params <JSON>
# params: image (file name), size_mib, label, uuid (also used as directory hash seed), block_size
# input:  "minirootfs" = root filesystem tarball (gzip tar)
# Reproducibility: mke2fs -d (no mount, no root privileges needed beyond ownership), fixed UUID/hash seed,
# E2FSPROGS_FAKE_TIME=SOURCE_DATE_EPOCH, atimes normalized before population, every inode's
# ctime/atime/crtime then set to SOURCE_DATE_EPOCH with debugfs (mtimes from the tarball are kept),
# e2fsck -fn verification, and zero blocks punched to holes (fallocate --dig-holes) so the sparse layout
# is a function of content.
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
IMG_NAME=$(jq -er .image <<<"$PARAMS")
SIZE=$(jq -er .size_mib <<<"$PARAMS")
LABEL=$(jq -er .label <<<"$PARAMS")
UUID=$(jq -er .uuid <<<"$PARAMS")
BS=$(jq -er '.block_size // 4096' <<<"$PARAMS")
SRC=$(jq -er .minirootfs <<<"$EB_INPUTS_JSON")
EPOCH=${SOURCE_DATE_EPOCH:?}

R="$EB_SCRATCH/rootfs"
rm -rf "$R"; mkdir -p "$R"
tar -xzf "$SRC" -C "$R" --numeric-owner --same-owner --same-permissions
chmod 0755 "$R"
find "$R" -exec touch -h -a -d "@$EPOCH" {} +

IMG="$OUT/$IMG_NAME"
truncate -s "${SIZE}M" "$IMG"
E2FSPROGS_FAKE_TIME=$EPOCH mke2fs -q -F -t ext4 -b "$BS" -I 256 -L "$LABEL" -U "$UUID" \
  -E "hash_seed=$UUID,lazy_itable_init=0,lazy_journal_init=0,nodiscard,root_owner=0:0" \
  -d "$R" "$IMG"

# normalize inode change/access/creation times of every allocated inode
CMDS="$EB_SCRATCH/debugfs.cmds"
# enumerate linked inodes by walking the directory tree (debugfs "ls -p": /ino/mode/uid/gid/name/size/)
walk() {
  local dir=$1
  debugfs -R "ls -p $dir" "$IMG" 2>/dev/null | while IFS=/ read -r _ ino mode uid gid name size _; do
    [[ -z $ino || $name == "." || $name == ".." ]] && continue
    echo "$ino"
    if [[ ${mode:0:2} == 04 ]]; then
      walk "${dir%/}/$name"
    fi
  done
}
{ echo 2; echo 11; walk /; } | sort -un | while read -r ino; do
  for f in ctime atime crtime; do echo "sif <$ino> $f @$EPOCH"; done
done > "$CMDS"
E2FSPROGS_FAKE_TIME=$EPOCH debugfs -w -f "$CMDS" "$IMG" >/dev/null 2>"$EB_SCRATCH/debugfs.err" || { cat "$EB_SCRATCH/debugfs.err" >&2; exit 1; }
if grep -v '^debugfs ' "$EB_SCRATCH/debugfs.err" | grep -q .; then cat "$EB_SCRATCH/debugfs.err" >&2; exit 1; fi
E2FSPROGS_FAKE_TIME=$EPOCH e2fsck -fn "$IMG" >/dev/null
fallocate --dig-holes "$IMG"
chmod 0644 "$IMG"
{
  echo "source_sha256 $(sha256sum "$SRC" | cut -d' ' -f1)"
  echo "image $IMG_NAME size_mib $SIZE block_size $BS label $LABEL uuid $UUID"
  mke2fs -V 2>&1 | head -1
} > "$OUT/BUILDINFO.txt"
chmod 0644 "$OUT/BUILDINFO.txt"
echo "build_ext4_image: done"
