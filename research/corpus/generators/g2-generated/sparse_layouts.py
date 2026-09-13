#!/usr/bin/env python3
"""Generated sparse-file layouts for family F15 (corpus group g2-generated).  GENERATED DATA.

Every file is created with pwrite() at explicit offsets plus ftruncate() (never fallocate: unwritten
extents are reported by SEEK_DATA inconsistently depending on page-cache state, which would make
the fingerprint extent map unstable).  After each file is closed the script re-opens it and checks
with SEEK_DATA/SEEK_HOLE that the data extents are exactly the 4 KiB block-aligned union of what was
written; any difference (e.g. a filesystem without hole support) fails loudly.

Models (params.model):
  vm-raw        Raw VM disk image: protective MBR + GPT (valid CRCs, backup GPT at the end), BIOS-boot
                partition, FAT32-like ESP with two FAT copies and boot files, ext4-like root partition
                with superblock + backups (sparse_super groups), flex_bg bitmaps, partially written
                inode tables, a zero-filled (allocated) journal with a few transaction blocks, file
                data extents (text, random/compressed, duplicated files, zero-written files) and
                stale data from deleted files.  params: disk_bytes, fill_fraction.
  db-prealloc   Database/queue preallocation: Kafka-like log segments with fixed-size sparse
                .index/.timeindex files, bbolt-like page file with scattered pages, InnoDB-like
                page-compressed tablespace (punch-hole per 16 KiB page), LMDB-like WRITEMAP data file
                truncated to the map size.  params: scale (float).
  edge-cases    Small boundary cases: all-hole, first/last byte only, alternating 4 KiB data/hole,
                1-byte islands at block edges, written zeros (no holes), sizes not block multiples,
                empty file, many tiny sparse files.  Total apparent size <= 16 MiB.
  fragmented    Many small extents: BitTorrent-like partially downloaded files (random piece bitmaps
                at several piece sizes), a regular 4 KiB-every-64 KiB lattice, a Bernoulli block
                pattern and a log with random hole runs.  params: scale (float).
  huge-hole     Huge holes / single extents: multi-GiB images with one data extent at the end, at the
                start or in the middle, an all-hole file, and journal-like 8 MiB fixed-size files.
                params: scale (float).
"""

import argparse
import errno
import hashlib
import json
import os
import random
import struct
import sys
import uuid
import zlib

BLOCK = 4096
MiB = 1 << 20
GiB = 1 << 30
EPOCH = int(os.environ.get("SOURCE_DATE_EPOCH", "1767225600"))


class Shake:
    def __init__(self, seed: int, domain: bytes):
        self.prefix = domain + b"\0" + struct.pack("<Q", seed & (2**64 - 1))
        self.counter = 0

    def __call__(self, n: int) -> bytes:
        out = bytearray()
        while len(out) < n:
            take = min(n - len(out), 4 * MiB)
            out += hashlib.shake_256(self.prefix + struct.pack("<Q", self.counter)).digest(take)
            self.counter += 1
        return bytes(out)


class Text:
    """Cheap pseudo-text (log/prose-like, compressible ~3-4x)."""

    SYLL = "ka ri to mo na lu be si ve ro da ne xi pa qu fe zo li ma te ho gu ja ce wi yo ba de fi ga".split()

    def __init__(self, rng: random.Random):
        self.rng = rng
        words = sorted({"".join(rng.choice(self.SYLL) for _ in range(rng.choice((1, 2, 3)))) for _ in range(3000)})
        lines = []
        for i in range(4096):
            kind = rng.random()
            if kind < 0.5:
                ln = f"{EPOCH - rng.randrange(10**6)} {rng.choice(('INFO', 'WARN', 'DEBUG', 'ERROR'))} " \
                     f"[{rng.choice(words)}] " + " ".join(rng.choice(words) for _ in range(rng.randrange(3, 14)))
            else:
                ln = " ".join(rng.choice(words) for _ in range(rng.randrange(4, 20))).capitalize() + "."
            lines.append((ln + "\n").encode())
        self.lines = lines

    def __call__(self, n: int) -> bytes:
        out = bytearray()
        lines = self.lines
        r = self.rng.randrange
        while len(out) < n:
            out += lines[r(4096)]
        return bytes(out[:n])


def data_extents(path: str, size: int) -> list:
    fd = os.open(path, os.O_RDONLY)
    try:
        ext = []
        off = 0
        while off < size:
            try:
                d = os.lseek(fd, off, os.SEEK_DATA)
            except OSError as e:
                if e.errno == errno.ENXIO:
                    break
                raise
            if d >= size:
                break
            h = min(os.lseek(fd, d, os.SEEK_HOLE), size)
            ext.append((d, h))
            off = h
        return ext
    finally:
        os.close(fd)


def expected_extents(writes: list, size: int) -> list:
    blocks = sorted((s // BLOCK * BLOCK, min(-(-e // BLOCK) * BLOCK, size)) for s, e in writes if e > s)
    merged = []
    for s, e in blocks:
        if merged and s <= merged[-1][1]:
            merged[-1] = (merged[-1][0], max(merged[-1][1], e))
        else:
            merged.append((s, e))
    return merged


class SparseFile:
    registry = []

    def __init__(self, path: str, size: int, mode: int = 0o644):
        os.makedirs(os.path.dirname(path), exist_ok=True)
        self.path, self.size, self.mode = path, size, mode
        self.fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600)
        self.writes = []

    def put(self, off: int, data: bytes) -> None:
        if not data:
            return
        if off < 0 or off + len(data) > self.size:
            raise SystemExit(f"write outside file bounds: {self.path} off={off} len={len(data)} size={self.size}")
        mv = memoryview(data)
        done = 0
        while done < len(data):
            done += os.pwrite(self.fd, mv[done:], off + done)
        self.writes.append((off, off + len(data)))

    def close(self) -> dict:
        os.ftruncate(self.fd, self.size)
        os.fsync(self.fd)
        os.close(self.fd)
        os.chmod(self.path, self.mode)
        exp = expected_extents(self.writes, self.size)
        got = data_extents(self.path, self.size)
        if got != exp:
            raise SystemExit(f"SEEK_DATA/SEEK_HOLE verification failed for {self.path}: expected {len(exp)} extents "
                             f"{exp[:4]}..., got {len(got)} {got[:4]}... (filesystem without hole support?)")
        data_bytes = sum(e - s for s, e in got)
        info = {"path": self.path, "size": self.size, "extents": len(got), "data_bytes": data_bytes}
        SparseFile.registry.append(info)
        return info


def aligned(x: int, a: int = BLOCK) -> int:
    return x // a * a


# ---------------------------------------------------------------------------------------------
# vm-raw

def guid_le(s: str) -> bytes:
    return uuid.UUID(s).bytes_le


def gpt_header(my_lba, alt_lba, first, last, disk_guid, entries_lba, entries_crc):
    hdr = bytearray(struct.pack("<8sIIIIQQQQ16sQIII", b"EFI PART", 0x00010000, 92, 0, 0, my_lba, alt_lba, first, last,
                                disk_guid, entries_lba, 128, 128, entries_crc))
    crc = zlib.crc32(bytes(hdr)) & 0xFFFFFFFF
    hdr[16:20] = struct.pack("<I", crc)
    return bytes(hdr) + bytes(512 - 92)


def model_vm_raw(out, rng, rnd, text, p):
    disk = aligned(int(p.get("disk_bytes", 3 * GiB)), MiB)
    fill = float(p.get("fill_fraction", 0.06))
    sf = SparseFile(os.path.join(out, p.get("name", "vm-disk0.raw")), disk)
    lbas = disk // 512
    parts = [
        (1 * MiB, 2 * MiB, "21686148-6449-6E6F-744E-656564454649", "BIOS boot partition"),
        (2 * MiB, 130 * MiB, "C12A7328-F81F-11D2-BA4B-00A0C93EC93B", "EFI System Partition"),
        (130 * MiB, disk - MiB, "0FC63DAF-8483-4772-8E79-3D69D8477DE4", "Linux filesystem"),
    ]
    # protective MBR
    mbr = bytearray(512)
    mbr[0:440] = rnd(440)
    mbr[440:444] = rnd(4)
    mbr[446:462] = struct.pack("<B3sB3sII", 0, b"\x00\x02\x00", 0xEE, b"\xff\xff\xff", 1, min(lbas - 1, 0xFFFFFFFF))
    mbr[510:512] = b"\x55\xaa"
    entries = bytearray(128 * 128)
    for i, (s, e, tg, name) in enumerate(parts):
        entries[i * 128:(i + 1) * 128] = guid_le(tg) + rnd(16) + struct.pack("<QQQ", s // 512, e // 512 - 1, 0) + \
            name.encode("utf-16-le").ljust(72, b"\0")
    ecrc = zlib.crc32(bytes(entries)) & 0xFFFFFFFF
    disk_guid = rnd(16)
    sf.put(0, bytes(mbr) + gpt_header(1, lbas - 1, 34, lbas - 34, disk_guid, 2, ecrc) + bytes(entries))
    sf.put((lbas - 33) * 512, bytes(entries) + gpt_header(lbas - 1, 1, 34, lbas - 34, disk_guid, lbas - 33, ecrc))
    # BIOS boot partition: core image
    sf.put(parts[0][0], b"\xeb\x63\x90" + rnd(48 * 1024 - 3))

    # ESP (FAT32-like)
    esp = parts[1][0]
    esp_sectors = (parts[1][1] - esp) // 512
    spc, reserved, nfats = 8, 32, 2
    fat_sectors = (esp_sectors // spc * 4 + 511) // 512
    bs = bytearray(512)
    bs[0:3] = b"\xeb\x58\x90"
    bs[3:11] = b"mkfs.fat"
    struct.pack_into("<HBHBHHBHHHII", bs, 11, 512, spc, reserved, nfats, 0, 0, 0xF8, 0, 63, 255, 0, esp_sectors)
    struct.pack_into("<IHHIHH", bs, 36, fat_sectors, 0, 0, 2, 1, 6)
    bs[66] = 0x29
    bs[67:71] = rnd(4)
    bs[71:82] = b"EFI        "
    bs[82:90] = b"FAT32   "
    bs[90:510] = rnd(420)
    bs[510:512] = b"\x55\xaa"
    fsinfo = bytearray(512)
    fsinfo[0:4] = b"RRaA"
    fsinfo[484:488] = b"rrAa"
    struct.pack_into("<II", fsinfo, 488, esp_sectors // spc - 400, 400)
    fsinfo[510:512] = b"\x55\xaa"
    sf.put(esp, bytes(bs) + bytes(fsinfo))
    sf.put(esp + 6 * 512, bytes(bs) + bytes(fsinfo))
    data_start = esp + (reserved + nfats * fat_sectors) * 512
    cluster = spc * 512
    files = [("BOOTX64.EFI", rnd(1_300_000)), ("grubx64.efi", rnd(2_400_000)), ("mmx64.efi", rnd(850_000)),
             ("grub.cfg", text(4200)), ("BOOTX64.CSV", "shimx64.efi,shim,,This is the boot entry\n".encode("utf-16-le"))]
    fat = [0x0FFFFFF8, 0x0FFFFFFF, 0x0FFFFFFF]
    next_cluster = 3
    dirent = bytearray()
    for name, data in files:
        n_cl = max(1, -(-len(data) // cluster))
        first = next_cluster
        for k in range(n_cl):
            fat.append(first + k + 1 if k < n_cl - 1 else 0x0FFFFFFF)
        next_cluster += n_cl
        sf.put(data_start + (first - 2) * cluster, b"MZ" + data[2:] if name.endswith((".EFI", ".efi")) else data)
        base, _, ext = name.partition(".")
        dirent += struct.pack("<11sBBBHHHHHHHI", (base[:8].upper().ljust(8) + ext[:3].upper().ljust(3)).encode(),
                              0x20, 0, 0, 0, 0x5a21, 0x5a21, first >> 16, 0, 0x5a21, first & 0xFFFF, len(data))
    sf.put(data_start, bytes(dirent).ljust(cluster, b"\0"))
    fat_bytes = b"".join(struct.pack("<I", x) for x in fat)
    for i in range(nfats):
        sf.put(esp + (reserved + i * fat_sectors) * 512, fat_bytes)

    # ext4-like root partition
    root, root_end = parts[2][0], parts[2][1]
    blocks = (root_end - root) // BLOCK
    bpg = 32768
    groups = -(-blocks // bpg)
    inodes_per_group = 8192
    itable_blocks = inodes_per_group * 256 // BLOCK
    sb = bytearray(1024)
    struct.pack_into("<IIIIIIIIIIIIIHHHHHHIIII", sb, 0, inodes_per_group * groups, blocks, blocks // 20,
                     int(blocks * (1 - fill)), inodes_per_group * groups - 900, 0, 2, 2, bpg, bpg, inodes_per_group,
                     EPOCH, EPOCH, 3, 0xFFFF, 0xEF53, 1, 1, 0, EPOCH, 0, 0, 1)
    sb[0x68:0x78] = rnd(16)
    sb[0x78:0x88] = b"rootfs".ljust(16, b"\0")
    sb[0x88:0xC8] = b"/".ljust(64, b"\0")

    def superblock_block(group):
        blk = bytearray(BLOCK)
        off = 1024 if group == 0 else 0
        s = bytearray(sb)
        struct.pack_into("<H", s, 0x5A, group)
        blk[off:off + 1024] = s
        return bytes(blk)

    gdt = bytearray()
    flex = 16
    for g in range(groups):
        flex_base = (g // flex) * flex * bpg
        bb = flex_base + 1 + 256 + (g % flex)
        ib = bb + flex
        it = flex_base + 1 + 256 + 2 * flex + (g % flex) * itable_blocks
        gdt += struct.pack("<IIIHHHHIHHHH", bb, ib, it, rng.randrange(bpg), rng.randrange(inodes_per_group),
                           rng.randrange(40), 0x0004, 0, rng.randrange(1 << 16), rng.randrange(1 << 16), 0, 0)
        gdt += bytes(64 - 32)
    backup_groups = {0, 1} | {3 ** k for k in range(1, 8)} | {5 ** k for k in range(1, 6)} | {7 ** k for k in range(1, 5)}
    for g in range(groups):
        if g in backup_groups:
            sf.put(root + g * bpg * BLOCK, superblock_block(g) + bytes(gdt).ljust(-(-len(gdt) // BLOCK) * BLOCK, b"\0"))
    # bitmaps and inode tables (flex groups): bitmaps written for every group, inode tables only
    # partially initialised (lazy itable init) except the first two groups, which are zeroed.
    reserved = [(g * bpg, g * bpg + 2) for g in backup_groups if g < groups]
    for g in range(groups):
        flex_base = (g // flex) * flex * bpg
        bb = flex_base + 1 + 256 + (g % flex)
        ib = bb + flex
        it = flex_base + 1 + 256 + 2 * flex + (g % flex) * itable_blocks
        usage = max(0.0, fill * 2.2 * (1 - g / max(1, groups)))
        bitmap = bytearray(BLOCK)
        nbits = int(bpg * usage)
        bitmap[: nbits // 8] = b"\xff" * (nbits // 8)
        sf.put(root + bb * BLOCK, bytes(bitmap))
        ibitmap = bytearray(BLOCK)
        nset = rng.randrange(0, 256)
        ibitmap[:nset] = b"\xff" * nset
        sf.put(root + ib * BLOCK, bytes(ibitmap))
        reserved.append((bb, bb + 1))
        reserved.append((ib, ib + 1))
        if g < 2:
            sf.put(root + it * BLOCK, bytes(itable_blocks * BLOCK))
        n_inodes = int(inodes_per_group * usage * 0.3)
        recs = bytearray()
        for i in range(n_inodes):
            mode = rng.choice((0o100644, 0o100755, 0o40755, 0o120777))
            recs += struct.pack("<HHIIIIIHHII", mode, 0, rng.randrange(1 << 24), EPOCH - rng.randrange(10**7), EPOCH,
                                EPOCH - rng.randrange(10**7), 0, 0, 1, rng.randrange(1 << 16), 0x80000)
            recs += struct.pack("<HHHHI", 0xF30A, 1, 4, 0, 0) + rnd(8) + bytes(256 - 36 - 12 - 8)
        if recs:
            sf.put(root + it * BLOCK, bytes(recs).ljust(-(-len(recs) // BLOCK) * BLOCK, b"\0"))
        reserved.append((it, it + itable_blocks))
    # journal: 64 MiB in the middle group, zero-filled (allocated) with a few transactions
    jgroup = groups // 2
    jstart = jgroup * bpg + 1024
    jblocks = 64 * MiB // BLOCK
    sf.put(root + jstart * BLOCK, bytes(jblocks * BLOCK))
    jsb = struct.pack(">IIIIIII", 0xC03B3998, 4, 0, BLOCK, jblocks, 1, 1) + rnd(16)
    sf.put(root + jstart * BLOCK, jsb)
    for tx in range(24):
        blk = struct.pack(">III", 0xC03B3998, 1, tx + 100) + rnd(40)
        sf.put(root + (jstart + 1 + tx * 9) * BLOCK, blk)
        for k in range(1, 8):
            sf.put(root + (jstart + 1 + tx * 9 + k) * BLOCK, rnd(BLOCK) if k % 3 else text(BLOCK))
    reserved.append((jstart, jstart + jblocks))
    # file data
    target = int(blocks * BLOCK * fill)
    written = 0
    previous = []
    cursor = {}
    attempts = 0
    while written < target and attempts < 200000:
        attempts += 1
        size = max(BLOCK, min(64 * MiB, int(rng.lognormvariate(11.0, 1.8))))
        g = min(groups - 1, int(abs(rng.gauss(0, groups * 0.35))))
        start_blk = cursor.get(g, g * bpg + (1 + 256 + 2 * flex + flex * itable_blocks if g % flex == 0 else 64))
        if start_blk >= min((g + 1) * bpg, blocks):
            if all(cursor.get(x, 0) >= min((x + 1) * bpg, blocks) for x in range(groups)):
                break
            continue
        nblk = -(-size // BLOCK)
        clash = next((r for r in reserved if r[0] < start_blk + nblk and start_blk < r[1]), None)
        if clash is not None and clash[0] > start_blk:
            nblk = clash[0] - start_blk  # shorten the extent to the free gap before the reserved area
            size = nblk * BLOCK
        elif clash is not None:
            cursor[g] = clash[1]
            continue
        if start_blk + nblk > min((g + 1) * bpg, blocks):
            cursor[g] = (g + 1) * bpg  # group full
            continue
        cursor[g] = start_blk + nblk + rng.choice((0, 0, 0, 1, 2, 8, 64))
        kind = rng.random()
        if kind < 0.35:
            data = text(size)
        elif kind < 0.72:
            data = rnd(size)
        elif kind < 0.84 and previous:
            data = previous[rng.randrange(len(previous))][:size]
            size = len(data)
        elif kind < 0.90:
            data = bytes(size)  # zero-written file content: allocated zeros
        else:
            unit = rnd(rng.choice((64, 256, 1024)))
            data = (unit * (size // len(unit) + 1))[:size]
        if len(previous) < 64 and size <= 8 * MiB and rng.random() < 0.3:
            previous.append(data)
        sf.put(root + start_blk * BLOCK, data)
        written += len(data)
    return [sf.close()]


# ---------------------------------------------------------------------------------------------
# db-prealloc

def model_db_prealloc(out, rng, rnd, text, p):
    scale = float(p.get("scale", 1.0))
    infos = []
    # Kafka-like topic partitions
    for part in range(3):
        d = os.path.join(out, "kafka-logs", f"events-{part}")
        base = 0
        nseg = 3
        for seg in range(nseg):
            active = seg == nseg - 1
            log_size = int(rng.uniform(4, 12) * MiB * scale) if not active else int(rng.uniform(1, 3) * MiB * scale)
            name = f"{base:020d}"
            # .log: dense record batches
            sf = SparseFile(os.path.join(d, name + ".log"), log_size)
            off = 0
            nrec = 0
            while off < log_size:
                payload = text(rng.randrange(200, 2000))
                batch = struct.pack(">QIiBI", base + nrec, len(payload) + 49, 0, 2, zlib.crc32(payload)) + \
                    struct.pack(">HIQQqhii", 0, 0, EPOCH * 1000 - rng.randrange(10**8), EPOCH * 1000, -1, -1, -1, 1) + payload
                batch = batch[: log_size - off]
                sf.put(off, batch)
                off += len(batch)
                nrec += 1
            infos.append(sf.close())
            entries = log_size // 4096
            idx_size = 10 * MiB if active else entries * 8
            tidx_size = 10 * MiB if active else entries * 12
            if idx_size:
                sf = SparseFile(os.path.join(d, name + ".index"), max(idx_size, 8))
                sf.put(0, b"".join(struct.pack(">II", i * 7, i * 4096) for i in range(entries)))
                infos.append(sf.close())
                sf = SparseFile(os.path.join(d, name + ".timeindex"), max(tidx_size, 12))
                sf.put(0, b"".join(struct.pack(">QI", EPOCH * 1000 + i * 37, i * 7) for i in range(entries)))
                infos.append(sf.close())
            base += nrec
        with open(os.path.join(d, "leader-epoch-checkpoint"), "w") as f:
            f.write("0\n1\n0 0\n")
        with open(os.path.join(d, "partition.metadata"), "w") as f:
            f.write(f"version: 0\ntopic_id: {uuid.UUID(bytes=rnd(16))}\n")
    # bbolt-like page file: grown by ftruncate, pages written at scattered page ids
    size = int(96 * MiB * scale) // BLOCK * BLOCK
    sf = SparseFile(os.path.join(out, "bolt", "state.db"), size)
    npages = size // BLOCK
    for pg in (0, 1):
        sf.put(pg * BLOCK, struct.pack("<QHHIIIQQQQQ", pg, 0x04, 0, 0, 0xED0CDAED, 2, BLOCK, 0, 3, 4, npages) + rnd(64))
    live = sorted(rng.sample(range(2, npages), int(npages * 0.06)))
    for pg in live:
        kind = rng.choice((0x01, 0x02, 0x02, 0x10))
        body = text(rng.randrange(800, BLOCK - 16)) if kind == 0x02 else rnd(rng.randrange(64, BLOCK - 16))
        sf.put(pg * BLOCK, struct.pack("<QHHI", pg, kind, rng.randrange(200), 0) + body)
    infos.append(sf.close())
    # InnoDB-like page-compressed tablespace: each 16 KiB page holds a compressed payload, the
    # remainder of the page is punched out (a hole) -> one extent per page
    page = 16 * 1024
    size = int(48 * MiB * scale) // page * page
    sf = SparseFile(os.path.join(out, "mysql", "shop", "orders.ibd"), size)
    for pg in range(size // page):
        if pg > 4 and rng.random() < 0.12:
            continue  # never-written page inside the extended tablespace
        raw = text(page - 38)
        comp = zlib.compress(raw, rng.choice((1, 6)))[: page - 38 - 8]
        hdr = struct.pack(">IIIIQHQI", zlib.crc32(comp), pg, max(0, pg - 1), pg + 1, 10**9 + pg * 977, 34354, 0, 1)
        sf.put(pg * page, hdr + comp)
    infos.append(sf.close())
    # LMDB-like data file with MDB_WRITEMAP: truncated to the map size, used prefix only
    size = int(160 * MiB * scale) // BLOCK * BLOCK
    sf = SparseFile(os.path.join(out, "lmdb", "data.mdb"), size)
    used = int(size * 0.08) // BLOCK
    for pg in range(used):
        if pg < 2:
            sf.put(pg * BLOCK, struct.pack("<QHHIQ", pg, 0x08, 0, 0xBEEFC0DE, size) + rnd(128))
        elif rng.random() < 0.9:
            sf.put(pg * BLOCK, struct.pack("<QHHI", pg, rng.choice((1, 2, 4)), 0, 0) + text(BLOCK - 16))
    infos.append(sf.close())
    lock = SparseFile(os.path.join(out, "lmdb", "lock.mdb"), 8192)
    lock.put(0, struct.pack("<IIQQ", 0xBEEFC0DE, 2, 0, 1) + bytes(40))
    infos.append(lock.close())
    return infos


# ---------------------------------------------------------------------------------------------
# edge-cases

def model_edge_cases(out, rng, rnd, text, p):
    infos = []

    def mk(name, size, puts):
        sf = SparseFile(os.path.join(out, name), size)
        for off, data in puts:
            sf.put(off, data)
        infos.append(sf.close())

    mk("all-hole-1m.bin", MiB, [])
    mk("empty.bin", 0, [])
    mk("last-byte-only-1m.bin", MiB, [(MiB - 1, b"\x01")])
    mk("first-byte-only-1m.bin", MiB, [(0, b"\x01")])
    mk("hole-then-partial-tail.bin", MiB + 1, [(MiB, b"Z")])
    mk("alternating-4k-data-first.bin", MiB, [(o, rnd(BLOCK)) for o in range(0, MiB, 2 * BLOCK)])
    mk("alternating-4k-hole-first.bin", MiB, [(o, rnd(BLOCK)) for o in range(BLOCK, MiB, 2 * BLOCK)])
    mk("byte-islands-at-block-edges.bin", MiB, [(o, bytes([rng.randrange(1, 256)])) for o in
                                                  (4095, 4096, 12287, 16384, 65535, 65537, 262143, 524288, 1048575)])
    mk("written-zeros-no-holes-1m.bin", MiB, [(0, bytes(MiB))])
    mk("zero-block-inside-data.bin", 12 * BLOCK, [(0, rnd(3 * BLOCK)), (3 * BLOCK, bytes(BLOCK)), (4 * BLOCK, rnd(3 * BLOCK)),
                                                  (9 * BLOCK, rnd(BLOCK))])
    mk("one-hole-in-the-middle.bin", 7 * BLOCK, [(0, rnd(3 * BLOCK)), (4 * BLOCK, rnd(3 * BLOCK))])
    mk("odd-size-with-mid-data.bin", 1_000_001, [(700_000, text(3000))])
    mk("tiny-4097-data-at-4096.bin", 4097, [(4096, b"!")])
    mk("partial-block-write-straddle.bin", 64 * BLOCK, [(BLOCK * 10 - 7, rnd(14)), (BLOCK * 30 + 1, rnd(BLOCK * 2 - 2))])
    mk("head-and-tail-1m.bin", MiB, [(0, text(6000)), (MiB - 5000, text(5000))])
    mk("hole-at-start-data-at-end-2m.bin", 2 * MiB, [(2 * MiB - 64 * 1024, rnd(64 * 1024))])
    for i in range(48):
        size = rng.choice((BLOCK * 3, 40 * 1024, 64 * 1024 + 1, 20000))
        off = rng.randrange(0, size - 1)
        mk(f"many/sparse-{i:02d}.dat", size, [(off, rnd(min(rng.randrange(1, 3000), size - off)))])
    total = sum(i["size"] for i in infos)
    if total > 16 * MiB:
        raise SystemExit(f"edge-cases total apparent size {total} exceeds 16 MiB")
    return infos


# ---------------------------------------------------------------------------------------------
# fragmented

def model_fragmented(out, rng, rnd, text, p):
    scale = float(p.get("scale", 1.0))
    infos = []
    for name, size, piece, frac in (("torrent/linux-dvd-partial.iso", 128 * MiB, 256 * 1024, 0.4),
                                    ("torrent/dataset-partial.tar", 64 * MiB, 16 * 1024, 0.25),
                                    ("torrent/video-partial.mkv", 48 * MiB, MiB, 0.6)):
        size = int(size * scale) // piece * piece
        sf = SparseFile(os.path.join(out, name), size)
        for i in range(size // piece):
            if rng.random() < frac:
                sf.put(i * piece, rnd(piece) if "tar" not in name else text(piece))
        infos.append(sf.close())
    size = int(64 * MiB * scale) // (64 * 1024) * (64 * 1024)
    sf = SparseFile(os.path.join(out, "lattice-4k-every-64k.bin"), size)
    for o in range(0, size, 64 * 1024):
        sf.put(o, rnd(BLOCK))
    infos.append(sf.close())
    size = int(48 * MiB * scale) // BLOCK * BLOCK
    sf = SparseFile(os.path.join(out, "bernoulli-blocks-p030.bin"), size)
    for b in range(size // BLOCK):
        if rng.random() < 0.3:
            sf.put(b * BLOCK, rnd(BLOCK))
    infos.append(sf.close())
    size = int(64 * MiB * scale) // BLOCK * BLOCK
    sf = SparseFile(os.path.join(out, "log-with-hole-runs.log"), size)
    o = 0
    while o < size:
        run = min(size - o, BLOCK * rng.randrange(1, 64))
        if rng.random() < 0.55:
            sf.put(o, text(run))
        o += run + BLOCK * int(rng.paretovariate(1.2))
    infos.append(sf.close())
    return infos


# ---------------------------------------------------------------------------------------------
# huge-hole

def model_huge_hole(out, rng, rnd, text, p):
    scale = float(p.get("scale", 1.0))
    infos = []
    s = aligned(int(2 * GiB * scale), MiB)
    sf = SparseFile(os.path.join(out, "images/tail-extent.img"), s)
    sf.put(s - MiB, rnd(MiB))
    infos.append(sf.close())
    s = aligned(int(1 * GiB * scale), MiB)
    sf = SparseFile(os.path.join(out, "images/head-extent.img"), s)
    sf.put(0, b"QFI\xfb" + rnd(64 * 1024 - 4))
    infos.append(sf.close())
    s = aligned(int(768 * MiB * scale), MiB)
    sf = SparseFile(os.path.join(out, "images/middle-island.img"), s)
    sf.put(s // 2, text(256 * 1024))
    sf.put(s - 1, b"\n")
    infos.append(sf.close())
    s = aligned(int(512 * MiB * scale), MiB)
    infos.append(SparseFile(os.path.join(out, "images/all-hole.img"), s).close())
    for i in range(10):
        s = 8 * MiB
        sf = SparseFile(os.path.join(out, "journal", f"system@{uuid.UUID(bytes=rnd(16)).hex}-{i:016x}.journal"), s)
        used = rng.randrange(200 * 1024, 1200 * 1024)
        sf.put(0, b"LPKSHHRH" + rnd(232))
        sf.put(BLOCK, text(used))
        if rng.random() < 0.5:
            sf.put(s - 2 * BLOCK, rnd(BLOCK))  # tail hash-table page
        infos.append(sf.close())
    return infos


MODELS = {"vm-raw": model_vm_raw, "db-prealloc": model_db_prealloc, "edge-cases": model_edge_cases,
          "fragmented": model_fragmented, "huge-hole": model_huge_hole}


def set_mtimes(out: str, rng: random.Random) -> None:
    paths = []
    for root, dnames, fnames in os.walk(out):
        for n in fnames + dnames:
            paths.append(os.path.relpath(os.path.join(root, n), out))
    paths.sort(key=lambda s: (-s.count(os.sep), s))
    for i, rel in enumerate(paths):
        t = (EPOCH - 86400 * 3 - i * 61) * 10**9 + (i * 7919) % 10**9
        os.utime(os.path.join(out, rel), ns=(t, t), follow_symlinks=False)


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True)
    ap.add_argument("--seed", type=int, required=True)
    ap.add_argument("--params", default="{}")
    args = ap.parse_args()
    p = json.loads(args.params)
    model = p.get("model")
    if model not in MODELS:
        raise SystemExit(f"sparse_layouts: params.model must be one of {sorted(MODELS)}")
    rng = random.Random(args.seed)
    rnd = Shake(args.seed, b"ebrc-g2-f15-" + model.encode())
    text = Text(random.Random(args.seed ^ 0x5DEECE66D))
    MODELS[model](args.out, rng, rnd, text, p)
    set_mtimes(args.out, rng)
    summary = {"files": len(SparseFile.registry), "apparent": sum(i["size"] for i in SparseFile.registry),
               "data": sum(i["data_bytes"] for i in SparseFile.registry),
               "extents": sum(i["extents"] for i in SparseFile.registry)}
    print(json.dumps(summary, sort_keys=True), file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
