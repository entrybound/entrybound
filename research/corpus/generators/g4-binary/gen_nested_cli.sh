#!/usr/bin/env bash
# Generator (g4-binary F14 held-out): nested archives built with command-line archivers.  GENERATED DATA.
#
#   bash gen_nested_cli.sh --out <staging> --seed N --params <JSON>     params: {"profile": "cli-v1"}
#
# Deliberately a different generator (and toolchain) from gen_nested_archives.py (tuning):
#   pkg_1.0-1_amd64.deb      ar(debian-binary, control.tar.zst, data.tar.xz(man/*.gz, share/assets.zip))
#   initrd.cpio.gz           gzip(cpio newc(lib/modules/*.ko.zst, etc/rootfs.squashfs(xz)))
#   bundle.tar.zst           zstd(tar(inner.7z(deep.zip(leaf.tar.lz4))))                      4 levels
#   logs.zip                 zip(day-*.log.bz2, old/archive.tar.lz)
#   firmware.squashfs        squashfs-xz(payload.tar.gz, update.cpio)
#   objects/libdemo.a        ar of generated relocatable-object-like blobs
# Pseudo-random bytes come from AES-128-CTR keyed by sha256(seed:label) (openssl), text from an awk LCG.
# Fixed mtimes (SOURCE_DATE_EPOCH), owners 0:0, sorted member order, single-threaded compressors.
set -euo pipefail
OUT= SEED= PARAMS='{}'
while [[ $# -gt 0 ]]; do
  case $1 in
    --out) OUT=$2; shift 2 ;;
    --seed) SEED=$2; shift 2 ;;
    --params) PARAMS=$2; shift 2 ;;
    *) echo "unknown argument $1" >&2; exit 2 ;;
  esac
done
[[ $(jq -r '.profile // "cli-v1"' <<<"$PARAMS") == cli-v1 ]] || { echo "unknown profile" >&2; exit 2; }
for t in openssl tar cpio ar zip 7z mksquashfs gzip xz zstd bzip2 lz4 lzip awk; do
  command -v "$t" >/dev/null || { echo "missing tool $t" >&2; exit 1; }
done
EPOCH=${SOURCE_DATE_EPOCH:-1767225600}
W="$EB_SCRATCH/gen"
rm -rf "$W"; mkdir -p "$W"
umask 022

rand() {  # rand <label> <bytes>
  local key
  key=$(printf '%s:%s' "$SEED" "$1" | sha256sum | cut -c1-32)
  head -c "$2" /dev/zero | openssl enc -aes-128-ctr -nosalt -K "$key" -iv 00000000000000000000000000000000
}
text() {  # text <label> <approx bytes>
  local s
  s=$(printf '%s:%s' "$SEED" "$1" | sha256sum | cut -c1-8)
  awk -v seed=$((16#$s)) -v n="$2" 'BEGIN {
    split("kernel module driver firmware update package archive stream block index header payload manifest checksum signature release build", w, " ");
    x = seed; t = 0; line = "";
    while (t < n) { x = (1103515245 * x + 12345) % 2147483648; word = w[int(x / 65536) % 16 + 1];
      line = line word " "; t += length(word) + 1;
      if (length(line) > 70) { print line; line = "" } }
    print line }'
}
stamp() { find "$1" -exec touch -h -d "@$EPOCH" {} +; }
tarc() {  # tarc <dir> <member...>  -> stdout (deterministic GNU tar)
  local d=$1; shift
  tar -C "$d" --sort=name --mtime="@$EPOCH" --owner=0 --group=0 --numeric-owner --format=gnu -cf - "$@"
}

# ---- 1. deb-like package
P="$W/deb"; mkdir -p "$P/control" "$P/data/usr/share/man/man1" "$P/data/usr/share/demo"
printf '2.0\n' > "$P/debian-binary"
printf 'Package: demo\nVersion: 1.0-1\nArchitecture: amd64\nDescription: generated\n' > "$P/control/control"
text md5 4000 > "$P/control/md5sums"
for i in 1 2 3 4 5 6; do text "man$i" 20000 | gzip -n -9 > "$P/data/usr/share/man/man1/tool$i.1.gz"; done
mkdir -p "$P/assets"
for i in 1 2 3 4 5 6 7 8; do rand "asset$i" $((30000 + i * 7000)) > "$P/assets/blob$i.bin"; text "assettxt$i" 15000 > "$P/assets/note$i.txt"; done
stamp "$P/assets"
(cd "$P/assets" && LC_ALL=C ls | LC_ALL=C sort | zip -X -D -q -9 -@ "$P/data/usr/share/demo/assets.zip")
stamp "$P"
tarc "$P/control" . | zstd -q -19 -T1 > "$P/control.tar.zst"
tarc "$P/data" . | xz -6 -T1 > "$P/data.tar.xz"
stamp "$P"
(cd "$P" && ar rcD "$OUT/pkg_1.0-1_amd64.deb" debian-binary control.tar.zst data.tar.xz)

# ---- 2. initrd-like cpio.gz with zstd modules and a squashfs image
I="$W/initrd"; mkdir -p "$I/root/lib/modules" "$I/root/etc" "$I/sq"
for i in $(seq 1 12); do rand "ko$i" $((20000 + i * 3000)) | cat - <(head -c 40000 /dev/zero) | zstd -q -9 -T1 > "$I/root/lib/modules/mod$i.ko.zst"; done
for i in 1 2 3 4 5; do text "sqtxt$i" 60000 > "$I/sq/doc$i.txt"; rand "sqbin$i" 50000 > "$I/sq/data$i.bin"; done
stamp "$I/sq"
SOURCE_DATE_EPOCH=$EPOCH mksquashfs "$I/sq" "$I/root/etc/rootfs.squashfs" -comp xz -all-root \
  -no-xattrs -processors 1 -noappend -quiet >/dev/null
stamp "$I/root"
(cd "$I/root" && find . -mindepth 1 | LC_ALL=C sort | cpio -o -H newc --reproducible -R 0:0 --quiet) | gzip -n -9 > "$OUT/initrd.cpio.gz"

# ---- 3. zstd(tar(7z(zip(tar.lz4))))  four levels
B="$W/bundle"; mkdir -p "$B/leaf" "$B/l2" "$B/l3" "$B/l4"
for i in $(seq 1 10); do text "leaf$i" 25000 > "$B/leaf/leaf$i.txt"; done
rand leafbin 200000 > "$B/leaf/leaf.bin"
stamp "$B/leaf"
tarc "$B/leaf" . | lz4 -q -9 > "$B/l2/leaf.tar.lz4"
text l2readme 3000 > "$B/l2/README"
stamp "$B/l2"
(cd "$B/l2" && zip -X -D -q -6 "$B/l3/deep.zip" README leaf.tar.lz4)
rand l3extra 100000 > "$B/l3/extra.bin"
stamp "$B/l3"
(cd "$B/l3" && 7z a -bd -bso0 -mx=5 -mmt=1 -mtc=off -mta=off "$B/l4/inner.7z" deep.zip extra.bin)
text manifest 2000 > "$B/l4/MANIFEST"
stamp "$B/l4"
tarc "$B/l4" . | zstd -q -12 -T1 > "$OUT/bundle.tar.zst"

# ---- 4. zip of bz2 logs and a tar.lz
L="$W/logs"; mkdir -p "$L/z/old" "$L/t"
for d in 01 02 03 04 05 06 07; do text "log$d" 120000 | bzip2 -9 > "$L/z/day-$d.log.bz2"; done
for i in 1 2 3; do text "old$i" 80000 > "$L/t/old$i.log"; done
stamp "$L/t"
tarc "$L/t" . | lzip -9 > "$L/z/old/archive.tar.lz"
stamp "$L/z"
(cd "$L/z" && find . -type f | LC_ALL=C sort | sed 's#^\./##' | zip -X -D -q -0 -@ "$OUT/logs.zip")

# ---- 5. squashfs holding a tar.gz and a cpio
F="$W/fw"; mkdir -p "$F/img" "$F/p" "$F/c"
for i in 1 2 3 4; do rand "fwp$i" 120000 > "$F/p/part$i.bin"; head -c 60000 /dev/zero > "$F/p/pad$i.bin"; done
stamp "$F/p"
tarc "$F/p" . | gzip -n -6 > "$F/img/payload.tar.gz"
for i in 1 2 3; do text "fwc$i" 40000 > "$F/c/cfg$i.conf"; done
stamp "$F/c"
(cd "$F/c" && find . -mindepth 1 | LC_ALL=C sort | cpio -o -H newc --reproducible -R 0:0 --quiet) > "$F/img/update.cpio"
stamp "$F/img"
SOURCE_DATE_EPOCH=$EPOCH mksquashfs "$F/img" "$OUT/firmware.squashfs" -comp xz -all-root \
  -no-xattrs -processors 1 -noappend -quiet >/dev/null

# ---- 6. static-library-like ar archive
A="$W/ar"; mkdir -p "$A" "$OUT/objects"
for i in $(seq 1 20); do { printf '\177ELF\002\001\001'; head -c 57 /dev/zero; rand "obj$i" $((4000 + i * 900)); head -c 3000 /dev/zero; } > "$A/obj$(printf %02d "$i").o"; done
stamp "$A"
(cd "$A" && ar rcD "$OUT/objects/libdemo.a" $(LC_ALL=C ls *.o))

find "$OUT" -type f -exec chmod 0644 {} +
find "$OUT" -type d -exec chmod 0755 {} +
echo "gen_nested_cli: done"
