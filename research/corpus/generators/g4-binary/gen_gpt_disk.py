#!/usr/bin/env python3
"""Generator (g4-binary F16 held-out): GPT-partitioned raw disk image with generated content.  GENERATED DATA.

Different generator from gen_fat_image.py (validation).  Contract:
    python gen_gpt_disk.py --out <staging> --seed N --params <JSON>
params: size_mib (disk), esp_mib (EFI system partition), disk_guid

Layout of <out>/disk-gpt-<size_mib>m.raw (sfdisk script with fixed disk and partition GUIDs):
  p1  BIOS boot  (1 MiB)   pseudo-random boot-loader-like blob with zero tail
  p2  ESP  FAT16/32 (esp_mib) built with mkfs.fat --invariant + mtools at a partition offset
  p3  Linux root ext4 (rest) built with mke2fs -d from a generated tree; inode c/a/crtimes normalized via debugfs
Partitions are written block-wise skipping all-zero 4 KiB blocks, then fallocate --dig-holes.
The generated root tree mixes /usr-like binaries (pseudo-ELF with zero runs), logs, configs, a
duplicate directory and a large preallocated-looking zero file.
"""

import argparse
import json
import os
import random
import shutil
import subprocess
import sys
import uuid

SECTOR = 512
MiB = 1 << 20


def run(cmd, env=None, stdin=None):
    r = subprocess.run(cmd, env=env, input=stdin, capture_output=True, text=True)
    if r.returncode != 0:
        sys.exit(f"gen_gpt_disk: {' '.join(cmd)} failed ({r.returncode}): {r.stderr.strip()[:2000]}")
    return r.stdout


def copy_into(disk, part_img, offset):
    with open(part_img, "rb") as src, open(disk, "r+b") as dst:
        pos = 0
        while True:
            blk = src.read(4096)
            if not blk:
                break
            if blk.count(0) != len(blk):
                dst.seek(offset + pos)
                dst.write(blk)
            pos += len(blk)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True)
    ap.add_argument("--seed", type=int, required=True)
    ap.add_argument("--params", default="{}")
    a = ap.parse_args()
    p = json.loads(a.params)
    size_mib, esp_mib = int(p["size_mib"]), int(p["esp_mib"])
    disk_guid = str(uuid.UUID(p["disk_guid"]))
    epoch = int(os.environ.get("SOURCE_DATE_EPOCH", "1767225600"))
    rng = random.Random(a.seed)
    scratch = os.environ.get("EB_SCRATCH") or (a.out + ".scratch")
    work = os.path.join(scratch, "gptwork")
    shutil.rmtree(work, ignore_errors=True)
    os.makedirs(work)
    env = dict(os.environ, SOURCE_DATE_EPOCH=str(epoch), E2FSPROGS_FAKE_TIME=str(epoch), MTOOLS_SKIP_CHECK="1", TZ="UTC")

    def guid(label):
        return str(uuid.UUID(bytes=random.Random(f"{a.seed}:{label}").randbytes(16), version=4))

    # ---- generated content trees
    def pseudo_elf(n):
        out = bytearray(b"\x7fELF\x02\x01\x01" + bytes(9))
        while len(out) < n:
            out += rng.randbytes(rng.randrange(500, 6000)) + bytes(rng.randrange(0, 3000))
        return bytes(out[:n])

    def lines(n):
        out, t = [], 0
        while t < n:
            ln = f"2026-01-{rng.randrange(1, 29):02d}T{rng.randrange(24):02d}:{rng.randrange(60):02d}:00Z host{rng.randrange(8)} " \
                 f"svc[{rng.randrange(3000)}]: " + rng.choice(["started", "stopped", "reload ok", "timeout after 30s",
                                                                  "disk /dev/vda3 at 81%", "checksum verified"]) + "\n"
            out.append(ln)
            t += len(ln)
        return "".join(out).encode()

    root = os.path.join(work, "root")
    esp = os.path.join(work, "esp")

    def put(base, rel, data, mode=0o644):
        path = os.path.join(base, rel)
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, "wb") as f:
            f.write(data)
        os.chmod(path, mode)

    for i in range(60):
        put(root, f"usr/bin/tool{i:02d}", pseudo_elf(rng.randrange(20_000, 400_000)), 0o755)
    for i in range(30):
        put(root, f"usr/lib/libgen{i:02d}.so.1", pseudo_elf(rng.randrange(50_000, 900_000)))
    for i in range(20):
        put(root, f"var/log/svc{i:02d}.log", lines(rng.randrange(5_000, 300_000)))
    for i in range(80):
        put(root, f"etc/conf.d/unit{i:03d}.conf", f"[unit]\nname=unit{i}\nrestart={rng.choice(['always', 'no'])}\n".encode())
    put(root, "var/lib/prealloc.img", bytes(24 * MiB))
    put(root, "srv/data/random.bin", rng.randbytes(12 * MiB))
    shutil.copytree(os.path.join(root, "usr/lib"), os.path.join(root, "backup/usr-lib-copy"))
    os.symlink("../usr/bin/tool00", os.path.join(root, "backup/tool-link"))
    put(esp, "EFI/BOOT/BOOTX64.EFI", b"MZ" + rng.randbytes(700_000))
    put(esp, "EFI/gen/grub.cfg", b"set timeout=3\nmenuentry 'gen' { linux /vmlinuz root=PARTUUID=" + guid("root").encode() + b" }\n")
    put(esp, "vmlinuz", rng.randbytes(9 * MiB))
    put(esp, "initrd.img", rng.randbytes(4 * MiB) + bytes(2 * MiB))
    for base in (root, esp):
        for dp, dns, fns in os.walk(base):
            for n in dns + fns:
                os.utime(os.path.join(dp, n), (epoch, epoch), follow_symlinks=False)
        os.utime(base, (epoch, epoch))

    # ---- partition table
    disk = os.path.join(a.out, f"disk-gpt-{size_mib}m.raw")
    with open(disk, "wb") as f:
        f.truncate(size_mib * MiB)
    bios_start, bios_sectors = 2048, 2048
    esp_start, esp_sectors = bios_start + bios_sectors, esp_mib * MiB // SECTOR
    root_start = esp_start + esp_sectors
    last_usable = size_mib * MiB // SECTOR - 34
    root_sectors = ((last_usable - root_start + 1) // 2048) * 2048
    script = (f"label: gpt\nlabel-id: {disk_guid}\nunit: sectors\nfirst-lba: 2048\n"
              f"start={bios_start}, size={bios_sectors}, type=21686148-6449-6E6F-744E-656564454649, uuid={guid('bios')}, name=\"bios\"\n"
              f"start={esp_start}, size={esp_sectors}, type=C12A7328-F81F-11D2-BA4B-00A0C93EC93B, uuid={guid('esp')}, name=\"esp\"\n"
              f"start={root_start}, size={root_sectors}, type=4F68BCE3-E8CD-4DB1-96E7-FBCAF984B709, uuid={guid('root')}, name=\"root\"\n")
    run(["sfdisk", "--no-reread", "--no-tell-kernel", "-q", disk], env, stdin=script)

    # p1 BIOS boot
    with open(disk, "r+b") as f:
        f.seek(bios_start * SECTOR)
        f.write(rng.randbytes(300_000))

    # p2 ESP
    esp_img = os.path.join(work, "esp.img")
    with open(esp_img, "wb") as f:
        f.truncate(esp_sectors * SECTOR)
    fatbits = "32" if esp_mib >= 64 else "16"
    run(["mkfs.fat", "-F", fatbits, "-i", guid("espvol")[:8], "-n", "GENESP", "--invariant", esp_img], env)
    for dp, dns, fns in sorted(os.walk(esp)):
        rel = os.path.relpath(dp, esp)
        if rel != ".":
            run(["mmd", "-i", esp_img, "::/" + rel], env)
        for fn in sorted(fns):
            r = os.path.normpath(os.path.join(rel, fn))
            run(["mcopy", "-i", esp_img, "-m", "-Q", os.path.join(esp, r), "::/" + r], env)
    run(["fsck.fat", "-n", esp_img], env)
    copy_into(disk, esp_img, esp_start * SECTOR)

    # p3 root ext4
    root_img = os.path.join(work, "root.img")
    with open(root_img, "wb") as f:
        f.truncate(root_sectors * SECTOR)
    fsuuid = guid("rootfs")
    run(["mke2fs", "-q", "-F", "-t", "ext4", "-b", "4096", "-I", "256", "-L", "genroot", "-U", fsuuid,
         "-E", f"hash_seed={fsuuid},lazy_itable_init=0,lazy_journal_init=0,nodiscard,root_owner=0:0",
         "-d", root, root_img], env)
    inodes = {2, 11}
    stack = ["/"]
    while stack:
        d = stack.pop()
        listing = run(["debugfs", "-R", f"ls -p {d}", root_img], env)
        for ln in listing.splitlines():
            parts = ln.split("/")
            if len(parts) < 7 or not parts[1].isdigit() or parts[5] in (".", ".."):
                continue
            inodes.add(int(parts[1]))
            if parts[2].startswith("04"):
                stack.append(d.rstrip("/") + "/" + parts[5])
    cmds = "".join(f"sif <{i}> {fld} @{epoch}\n" for i in sorted(inodes) for fld in ("ctime", "atime", "crtime"))
    cmdfile = os.path.join(work, "debugfs.cmds")
    with open(cmdfile, "w") as f:
        f.write(cmds)
    run(["debugfs", "-w", "-f", cmdfile, root_img], env)
    run(["e2fsck", "-fn", root_img], env)
    copy_into(disk, root_img, root_start * SECTOR)

    run(["sfdisk", "--verify", disk], env)
    run(["fallocate", "--dig-holes", disk], env)
    os.chmod(disk, 0o644)
    shutil.rmtree(work, ignore_errors=True)


if __name__ == "__main__":
    main()
