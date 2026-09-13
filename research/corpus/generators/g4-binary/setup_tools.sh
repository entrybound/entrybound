#!/usr/bin/env bash
# Install the Ubuntu packages the g4-binary build scripts need (idempotent).
#   dosfstools  mkfs.vfat (FAT images)          mtools     mcopy/mmd (populate FAT without mounting)
#   qemu-utils  qemu-img (raw <-> qcow2/vmdk)    squashfs-tools, e2fsprogs, xfsprogs, btrfs-progs: already present
#   p7zip-full  7z (7z/SFX creation for nested archives)   zip/unzip
set -euo pipefail
need=()
for p in dosfstools mtools qemu-utils p7zip-full zip unzip cpio xz-utils zstd lzip bzip2 file; do
  if ! dpkg-query -W -f='${Status}' "$p" 2>/dev/null | grep -q "install ok installed"; then need+=("$p"); fi
done
if ((${#need[@]})); then
  echo "installing: ${need[*]}"
  export DEBIAN_FRONTEND=noninteractive
  apt-get -o DPkg::Lock::Timeout=900 update -qq
  apt-get -o DPkg::Lock::Timeout=900 install -y -qq --no-install-recommends "${need[@]}"
fi
for t in mkfs.vfat mcopy mmd qemu-img 7z zip mke2fs mksquashfs mkfs.xfs; do printf '%-10s %s\n' "$t" "$(command -v $t)"; done
qemu-img --version | head -1
mkfs.vfat --help 2>&1 | head -1 || true
mtools --version 2>&1 | head -1 || true
echo "== rust"
ls -la /root/.cargo/bin | head -30
/root/.cargo/bin/rustup toolchain list 2>&1 || true
/root/.cargo/bin/rustup show active-toolchain 2>&1 || true
