#!/usr/bin/env python3
"""Build/derive script (F15, corpus group g2-generated): convert a real VM/cloud disk image
(vmdk or qcow2) to a sparse raw disk with qemu-img, then -- only when the disk's own protective
MBR proves it is safe -- shrink the raw file's declared/logical size down to the disk's original
real capacity.

Some published cloud/VM image artifacts (this OVA's VMDK, and this qcow2) declare a nominal
virtual disk size (here: 10 GiB) larger than the GPT partition table they actually contain, which
was originally created for a smaller disk (here: ~3.5 GiB) and then repackaged onto a bigger
nominal disk for template convenience -- `fdisk`/GPT reports this as a "GPT PMBR size mismatch" and
"backup GPT table is not on the end of the device". Every byte beyond the disk's *original*
declared size (read from the protective MBR's partition-record LBA count, the same field `fdisk`
reads to print that mismatch) is pure unpartitioned zero padding: this script proves that with
SEEK_DATA (no data extent may start at or beyond that boundary) before truncating, so the shrink is
provably lossless and, as a side effect, restores GPT/PMBR consistency (the backup header ends up
exactly where the PMBR already expected it). This keeps the corpus item's *logical* size (what the
scale-tier check measures) equal to the disk's real content instead of its incidental template
padding, while every real partition and every real byte is preserved unchanged (verified by an
exact SHA-256 match of the retained region before/after truncation).

A third mode, "gpt-partition", handles the opposite case: a real disk whose GPT partitions
genuinely fill the whole nominal size (no PMBR mismatch, so no lossless whole-disk shrink is
possible -- confirmed here for the AlmaLinux GenericCloud image: its xfs root partition's own
superblock block count exactly equals its partition size). There the *disk* doesn't fit the scale
tier, but one real partition does: the disk is loop-mounted (kernel partition scanning via
`losetup -fP`) and that single real partition's block device is converted straight to sparse raw
with qemu-img, which is exactly as real (same filesystem, same bytes) as converting the whole disk
would be, just scoped to a byte range the tier ceiling can hold.

Params (EB_PARAMS_JSON):
  mode          "ova-vmdk"        -- EB_INPUT_<ova_input> is an .ova (tar of .ovf/.mf/.vmdk);
                                     vmdk_member names the .vmdk entry inside it.
                "from-item-qcow2" -- from_item + src_relpath locate an already-provisioned item's
                                     qcow2 file (EB_FROM_<FROM_ITEM>).
                "gpt-partition"   -- from_item + src_relpath as above; partition_label (GPT
                                     PARTLABEL, e.g. "boot") selects which partition to extract.
  dst           output raw file name, written to EB_OUT/<dst>
  src_format    "vmdk" | "qcow2"
  allow_shrink  bool, default true; if the PMBR declares a size and no data extent exists beyond
                it, truncate to that size (see docstring). If false, keeps the full converted size.
"""

import json
import os
import struct
import subprocess
import tarfile
import time
from pathlib import Path


def sha256_range(path: Path, start: int, length: int) -> str:
    import hashlib
    h = hashlib.sha256()
    with open(path, "rb") as f:
        f.seek(start)
        remaining = length
        while remaining > 0:
            chunk = f.read(min(remaining, 8 << 20))
            if not chunk:
                break
            h.update(chunk)
            remaining -= len(chunk)
    return h.hexdigest()


def pmbr_declared_bytes(path: Path) -> int | None:
    """Parse the protective MBR's partition-record LBA count (offset 446+8, 4 bytes LE); the
    disk's PMBR-declared total size is (lba_count + 1) * 512 (LBA 0 is the MBR sector itself).
    Returns None if this doesn't look like a GPT protective MBR (no 0xEE partition type)."""
    with open(path, "rb") as f:
        mbr = f.read(512)
    if len(mbr) < 512 or mbr[510:512] != b"\x55\xaa":
        return None
    entry = mbr[446:462]
    ptype = entry[4]
    if ptype != 0xEE:
        return None
    (lba_count,) = struct.unpack_from("<I", entry, 12)
    return (lba_count + 1) * 512


def first_data_extent_at_or_after(path: Path, offset: int, size: int) -> int | None:
    fd = os.open(path, os.O_RDONLY)
    try:
        try:
            d = os.lseek(fd, offset, os.SEEK_DATA)
        except OSError as e:
            if e.errno == 6:  # ENXIO: no more data past offset
                return None
            raise
        return d if d < size else None
    finally:
        os.close(fd)


def main() -> None:
    params = json.loads(os.environ["EB_PARAMS_JSON"])
    out = Path(os.environ["EB_OUT"])
    scratch = Path(os.environ.get("EB_SCRATCH", "/tmp"))
    scratch.mkdir(parents=True, exist_ok=True)

    mode = params["mode"]
    src_format = params["src_format"]
    dst = out / params["dst"]
    allow_shrink = params.get("allow_shrink", True)

    if mode == "gpt-partition":
        from_key = "EB_FROM_" + params["from_item"].upper().replace("-", "_")
        base = Path(os.environ[from_key])
        src = base / params["src_relpath"]
        src_sha256 = subprocess.run(["sha256sum", str(src)], capture_output=True, text=True,
                                    check=True).stdout.split()[0]
        full_raw = scratch / "full-disk.raw"
        subprocess.run(["qemu-img", "convert", "-p", "-m", "1", "-f", src_format, "-O", "raw", str(src),
                       str(full_raw)], check=True, stdin=subprocess.DEVNULL)
        loop = subprocess.run(["losetup", "-fP", "--show", str(full_raw)], capture_output=True, text=True,
                              check=True).stdout.strip()
        try:
            time.sleep(0.5)  # let the kernel finish scanning/creating partition device nodes
            label = params["partition_label"]
            blkid = subprocess.run(["blkid"], capture_output=True, text=True, check=True).stdout
            part_dev = None
            for line in blkid.splitlines():
                if line.startswith(loop + "p") and f'PARTLABEL="{label}"' in line:
                    part_dev = line.split(":")[0]
                    break
            if part_dev is None:
                raise SystemExit(f"no partition with PARTLABEL={label!r} found under {loop}")
            part_fstype = None
            for kv in blkid.splitlines():
                if kv.startswith(part_dev + ":"):
                    for tok in kv.split():
                        if tok.startswith('TYPE="'):
                            part_fstype = tok.split('"')[1]
            subprocess.run(["qemu-img", "convert", "-p", "-m", "1", "-f", "raw", "-O", "raw", "-S", "4k", part_dev,
                           str(dst)], check=True, stdin=subprocess.DEVNULL)
        finally:
            subprocess.run(["losetup", "-d", loop], check=False)
        full_raw.unlink(missing_ok=True)
        subprocess.run(["fallocate", "--dig-holes", str(dst)], check=True)
        subprocess.run(["sync", str(dst)], check=True)
        os.chmod(dst, 0o644)
        size = dst.stat().st_size
        data = first_data_extent_at_or_after(dst, 0, size)
        qemu_version = subprocess.run(["qemu-img", "--version"], capture_output=True, text=True, check=True) \
            .stdout.splitlines()[0]
        info = (
            f"source {params['src_relpath']} format {src_format} sha256 {src_sha256}\n"
            f"partition PARTLABEL={label} fstype={part_fstype} extracted whole (kernel loop partition scan), "
            f"because the source disk's GPT genuinely fills its nominal size (no PMBR/GPT size mismatch) so no "
            f"lossless whole-disk shrink was possible; this one real partition is used instead.\n"
            f"output {dst.name} format raw options -S 4k (fallocate --dig-holes applied), size {size} bytes, "
            f"first data extent at {data}\n"
            f"{qemu_version}\n"
        )
        (out / "CONVERTINFO.txt").write_text(info, encoding="utf-8")
        os.chmod(out / "CONVERTINFO.txt", 0o644)
        print(f"shrink_real_disk_image: gpt-partition {label} ({part_fstype}) -> {dst.name} ({size} bytes)")
        return

    if mode == "ova-vmdk":
        ova_key = "EB_INPUT_" + params["ova_input"].upper().replace("-", "_")
        ova_path = os.environ[ova_key]
        member = params["vmdk_member"]
        with tarfile.open(ova_path, "r:") as tf:
            tf.extract(member, path=scratch, filter="data")
        src = scratch / member
        src_desc = f"{member} (extracted from {os.path.basename(ova_path)})"
    elif mode == "from-item-qcow2":
        from_key = "EB_FROM_" + params["from_item"].upper().replace("-", "_")
        base = Path(os.environ[from_key])
        src = base / params["src_relpath"]
        src_desc = params["src_relpath"]
    else:
        raise SystemExit(f"unknown mode {mode!r}")

    src_sha256 = subprocess.run(["sha256sum", str(src)], capture_output=True, text=True, check=True).stdout.split()[0]

    subprocess.run(
        ["qemu-img", "convert", "-p", "-m", "1", "-f", src_format, "-O", "raw", "-S", "4k", str(src), str(dst)],
        check=True, stdin=subprocess.DEVNULL)
    full_size = dst.stat().st_size

    declared = pmbr_declared_bytes(dst) if allow_shrink else None
    action = "kept at full converted size (no shrink)"
    shrunk_to = full_size
    if declared is not None and declared < full_size:
        first_data = first_data_extent_at_or_after(dst, declared, full_size)
        if first_data is None:
            pre_hash = sha256_range(dst, 0, declared)
            os.truncate(dst, declared)
            post_hash = sha256_range(dst, 0, declared)
            if pre_hash != post_hash:
                raise SystemExit("shrink verification failed: retained-region hash changed across truncation")
            shrunk_to = declared
            action = (f"shrunk {full_size} -> {declared} bytes (PMBR-declared original disk size; "
                      f"SEEK_DATA found no data at/after offset {declared} before truncating; "
                      f"retained-region SHA-256 identical before/after: {post_hash})")
        else:
            action = (f"PMBR declares {declared} bytes but a data extent starts at {first_data} "
                      f"(< full size {full_size}); kept at full converted size to avoid data loss")

    subprocess.run(["fallocate", "--dig-holes", str(dst)], check=True)
    subprocess.run(["sync", str(dst)], check=True)
    os.chmod(dst, 0o644)

    qemu_version = subprocess.run(["qemu-img", "--version"], capture_output=True, text=True, check=True) \
        .stdout.splitlines()[0]
    info = (
        f"source {src_desc} format {src_format} sha256 {src_sha256}\n"
        f"output {dst.name} format raw options -S 4k (fallocate --dig-holes applied), size {shrunk_to} bytes\n"
        f"shrink: {action}\n"
        f"{qemu_version}\n"
    )
    (out / "CONVERTINFO.txt").write_text(info, encoding="utf-8")
    os.chmod(out / "CONVERTINFO.txt", 0o644)
    print(f"shrink_real_disk_image: done ({action})")


if __name__ == "__main__":
    main()
