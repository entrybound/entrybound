#!/usr/bin/env python3
"""Generator (g4-binary F16 validation): FAT32 filesystem image with generated content.  GENERATED DATA.

Contract: python gen_fat_image.py --out <staging> --seed N --params <JSON>
params: size_mib (int), fat (12|16|32), volume_id (8 hex), label (<=11 chars)

Builds <out>/fat<fat>-<size_mib>m.img without mounting: mkfs.vfat with fixed volume id and
--invariant, then mtools (mmd/mcopy -m) with SOURCE_DATE_EPOCH and host mtimes fixed, entries added in
sorted order.  Content: an EFI-system-partition-like tree (EFI/BOOT/*.EFI pseudo-PE blobs, loader
entries, vendor configs), a directory of many small files with long/Unicode names, and a few multi-MiB
files (random, zero-padded, repeated).  Zero clusters are punched to holes (fallocate --dig-holes).
"""

import argparse
import json
import os
import random
import shutil
import subprocess
import sys

WORDS = "boot loader entry kernel initrd options root quiet splash timeout default console efi vendor".split()


def run(cmd, env=None):
    r = subprocess.run(cmd, env=env, capture_output=True, text=True)
    if r.returncode != 0:
        sys.exit(f"gen_fat_image: {' '.join(cmd)} failed: {r.stderr.strip()[:2000]}")
    return r.stdout


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True)
    ap.add_argument("--seed", type=int, required=True)
    ap.add_argument("--params", default="{}")
    a = ap.parse_args()
    p = json.loads(a.params)
    size_mib, fat = int(p["size_mib"]), int(p["fat"])
    epoch = int(os.environ.get("SOURCE_DATE_EPOCH", "1767225600"))
    rng = random.Random(a.seed)
    scratch = os.environ.get("EB_SCRATCH") or a.out + ".scratch"
    tree = os.path.join(scratch, "fat-tree")
    shutil.rmtree(tree, ignore_errors=True)

    def put(rel, data):
        path = os.path.join(tree, rel)
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, "wb") as f:
            f.write(data)

    def text(n):
        s, out = 0, []
        while s < n:
            w = rng.choice(WORDS)
            out.append(w)
            s += len(w) + 1
        return (" ".join(out) + "\n").encode()

    def pe_like(n):
        hdr = b"MZ" + bytes(58) + (128).to_bytes(4, "little") + bytes(64) + b"PE\x00\x00"
        body = bytearray()
        while len(body) < n:
            body += rng.randbytes(rng.randrange(2000, 20000)) + bytes(rng.randrange(0, 8000))
        return hdr + bytes(body[:n])

    put("EFI/BOOT/BOOTX64.EFI", pe_like(1_200_000))
    put("EFI/BOOT/mmx64.efi", pe_like(850_000))
    for v in ("debian", "fedora", "ubuntu"):
        put(f"EFI/{v}/grubx64.efi", pe_like(rng.randrange(1_500_000, 2_600_000)))
        put(f"EFI/{v}/shimx64.efi", pe_like(950_000))
        put(f"EFI/{v}/grub.cfg", text(rng.randrange(400, 4000)))
        put(f"EFI/{v}/BOOTX64.CSV", "shimx64.efi,{},,This is the boot entry\n".format(v).encode("utf-16"))
    for i in range(24):
        put(f"loader/entries/entry-{i:02d}.conf", text(rng.randrange(120, 600)))
    for i in range(400):
        name = f"small-files/Long File Name Number {i:04d} " + rng.choice(["résumé", "naïve", "日本語", "plain", "Ωmega"]) + ".txt"
        put(name, text(rng.randrange(10, 3000)))
    put("big/random-6m.bin", rng.randbytes(6 << 20))
    put("big/zero-padded-8m.bin", rng.randbytes(1 << 20) + bytes(7 << 20))
    chunk = rng.randbytes(256 << 10)
    put("big/repeated-4m.bin", chunk * 16)
    put("big/firmware-ff-3m.bin", rng.randbytes(700_000) + b"\xff" * (3 << 20) )

    for root, dirs, files in os.walk(tree):
        for n in dirs + files:
            os.utime(os.path.join(root, n), (epoch, epoch))

    img = os.path.join(a.out, f"fat{fat}-{size_mib}m.img")
    with open(img, "wb") as f:
        f.truncate(size_mib << 20)
    env = dict(os.environ, SOURCE_DATE_EPOCH=str(epoch), MTOOLS_SKIP_CHECK="1", TZ="UTC")
    help_text = subprocess.run(["mkfs.vfat", "--help"], capture_output=True, text=True).stdout + \
        subprocess.run(["mkfs.vfat", "--help"], capture_output=True, text=True).stderr
    inv = ["--invariant"] if "--invariant" in help_text else []
    run(["mkfs.vfat", "-F", str(fat), "-i", p["volume_id"], "-n", p["label"], "-S", "512", *inv, img], env)
    entries = []
    for root, dirs, files in os.walk(tree):
        dirs.sort()
        rel = os.path.relpath(root, tree)
        if rel != ".":
            entries.append(("d", rel))
        for fn in sorted(files):
            entries.append(("f", os.path.normpath(os.path.join(rel, fn))))
    for kind, rel in entries:
        target = "::/" + rel.replace(os.sep, "/")
        if kind == "d":
            run(["mmd", "-i", img, target], env)
        else:
            run(["mcopy", "-i", img, "-m", "-Q", os.path.join(tree, rel), target], env)
    run(["fsck.vfat", "-n", img], env)
    run(["fallocate", "--dig-holes", img], env)
    os.chmod(img, 0o644)
    shutil.rmtree(tree, ignore_errors=True)


if __name__ == "__main__":
    main()
