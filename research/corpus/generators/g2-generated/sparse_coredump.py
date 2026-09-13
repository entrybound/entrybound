#!/usr/bin/env python3
"""Generated sparse process core dumps and fixed-size segment files for family F15 (held-out
generator, corpus group g2-generated).  GENERATED DATA.

A separate generator from sparse_layouts.py so that held-out sparse items do not share a layout model
with tuning/validation items.

ELF64 x86-64 core files (ET_CORE) laid out the way the Linux core writer does: ELF header, program
headers, a PT_NOTE segment (NT_PRSTATUS, NT_PRPSINFO, NT_SIGINFO, NT_AUXV, NT_FILE, NT_FPREGSET,
NT_X86_XSTATE with synthetic register contents), then PT_LOAD segments at page-aligned file offsets.
Pages that are entirely zero are skipped with lseek (holes), exactly as dump_skip() does:
  * file-backed r-x mappings: only the ELF header page is dumped (coredump_filter bit 4), memsz > filesz;
  * rw-p data/bss and the heap: allocator-like chunks with pointer-sized fields, strings and freed
    (zeroed) chunks;
  * thread stacks (stack_mib): only the top 32-512 KiB touched, the rest holes;
  * anonymous arena mappings: a random fraction of touched pages;
  * guard pages: memsz only.
Fixed-size write-ahead-log segment files: created by ftruncate() to the segment size, records written
from the start, the unused tail left as a hole.

params: cores (int), heap_mib (int), arenas (int), arena_mib (int), threads (int), stack_mib (int),
        wal_segments (int), wal_segment_mib (int)
Each file's data extents are verified with SEEK_DATA/SEEK_HOLE against the pages written.
"""

import argparse
import errno
import hashlib
import json
import os
import random
import struct
import sys

PAGE = 4096
MiB = 1 << 20
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


def data_extents(path, size):
    fd = os.open(path, os.O_RDONLY)
    try:
        ext, off = [], 0
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


class PageWriter:
    """Writes buffers at page granularity, skipping all-zero pages (dump_skip semantics)."""

    def __init__(self, path, mode=0o600):
        os.makedirs(os.path.dirname(path), exist_ok=True)
        self.path = path
        self.mode = mode
        self.fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600)
        self.data_pages = set()
        self.partial = []
        self.zero = bytes(PAGE)

    def write_pages(self, off, buf):
        assert off % PAGE == 0
        for i in range(0, len(buf), PAGE):
            pg = buf[i:i + PAGE]
            if pg == self.zero[:len(pg)]:
                continue
            os.pwrite(self.fd, pg, off + i)
            self.data_pages.add((off + i) // PAGE)

    def write_raw(self, off, buf):
        os.pwrite(self.fd, buf, off)
        for pgno in range(off // PAGE, (off + len(buf) + PAGE - 1) // PAGE):
            self.data_pages.add(pgno)

    def close(self, size):
        os.ftruncate(self.fd, size)
        os.fsync(self.fd)
        os.close(self.fd)
        os.chmod(self.path, self.mode)
        exp = []
        for pgno in sorted(self.data_pages):
            s, e = pgno * PAGE, min((pgno + 1) * PAGE, size)
            if s >= size:
                continue
            if exp and exp[-1][1] == s:
                exp[-1] = (exp[-1][0], e)
            else:
                exp.append((s, e))
        got = data_extents(self.path, size)
        if got != exp:
            raise SystemExit(f"SEEK_DATA/SEEK_HOLE verification failed for {self.path}: expected {len(exp)} extents, "
                             f"got {len(got)}")
        return {"size": size, "extents": len(got), "data": sum(e - s for s, e in got)}


def note(name: bytes, ntype: int, desc: bytes) -> bytes:
    pad = lambda b: b + bytes((4 - len(b) % 4) % 4)  # noqa: E731
    return struct.pack("<III", len(name) + 1, len(desc), ntype) + pad(name + b"\0") + pad(desc)


def heap_bytes(rng, rnd, size, base_ptr):
    buf = bytearray(size)
    off = 16
    strings = [b"GET /api/v1/items?id=", b"connection reset by peer", b"worker-", b"cache miss for key ",
               b"application/json", b"Content-Length: ", b"/var/lib/service/state/"]
    while off < size - 64:
        csize = min(size - off - 16, max(32, int(rng.paretovariate(1.3) * 48)) // 16 * 16)
        struct.pack_into("<QQ", buf, off, 0, csize | 1)
        body = off + 16
        kind = rng.random()
        if kind < 0.25:
            pass  # freed/never-touched chunk: zeros (whole zero pages become holes)
        elif kind < 0.55:
            for q in range(body, body + csize - 8, 8):
                if rng.random() < 0.6:
                    struct.pack_into("<Q", buf, q, base_ptr + rng.randrange(0, size) // 16 * 16)
        elif kind < 0.8:
            s = rng.choice(strings) + str(rng.randrange(10**9)).encode()
            buf[body:body + min(csize, len(s))] = s[:csize]
        else:
            n = min(csize, 4096)
            buf[body:body + n] = rnd(n)
        off = body + csize
    return bytes(buf)


def write_core(path, rng, rnd, p):
    heap = p.get("heap_mib", 32) * MiB
    arenas = p.get("arenas", 3)
    arena_sz = p.get("arena_mib", 32) * MiB
    threads = p.get("threads", 4)
    stack = int(p.get("stack_mib", 8)) * MiB
    libs = ["/usr/bin/ebrc-worker", "/usr/lib/x86_64-linux-gnu/libc.so.6", "/usr/lib/x86_64-linux-gnu/libm.so.6",
            "/usr/lib/x86_64-linux-gnu/libssl.so.3", "/usr/lib/x86_64-linux-gnu/libz.so.1",
            "/usr/lib/x86_64-linux-gnu/ld-linux-x86-64.so.2"]
    maps = []  # (vaddr, memsz, flags, kind, fname)
    va = 0x555555554000
    for i, lib in enumerate(libs):
        if i == 1:
            va = 0x7F0000000000 + rng.randrange(1 << 20) * PAGE
        text = rng.randrange(16, 512) * PAGE
        maps.append((va, text, 5, "elfhdr", lib))
        va += text
        maps.append((va, rng.randrange(1, 16) * PAGE, 4, "file-ro", lib))
        va += maps[-1][1]
        maps.append((va, rng.randrange(2, 64) * PAGE, 6, "data", lib))
        va += maps[-1][1]
        if i == 0:
            maps.append((0x555556000000, heap, 6, "heap", "[heap]"))
            va = 0x7F0000000000
    va = 0x7E0000000000
    for a in range(arenas):
        maps.append((va, PAGE, 0, "guard", None))
        va += PAGE
        maps.append((va, arena_sz, 6, "arena", None))
        va += arena_sz
    for t in range(threads):
        maps.append((va, PAGE, 0, "guard", None))
        va += PAGE
        maps.append((va, stack, 6, "stack", None))
        va += stack
    maps.append((0x7FFFFFFDE000, 132 * PAGE, 6, "stack", "[stack]"))
    maps.append((0x7FFFFFFFD000, 2 * PAGE, 5, "vdso", "[vdso]"))

    notes = bytearray()
    prstatus = bytearray(336)
    struct.pack_into("<iii", prstatus, 0, 11, 0, 0)
    struct.pack_into("<I", prstatus, 32, rng.randrange(1000, 60000))
    prstatus[112:112 + 27 * 8] = b"".join(struct.pack("<Q", rng.randrange(1 << 47)) for _ in range(27))
    notes += note(b"CORE", 1, bytes(prstatus))
    psinfo = bytearray(136)
    psinfo[0] = ord("R")
    psinfo[40:56] = b"ebrc-worker".ljust(16, b"\0")
    psinfo[56:136] = b"/usr/bin/ebrc-worker --config /etc/ebrc/worker.toml".ljust(80, b"\0")
    notes += note(b"CORE", 3, bytes(psinfo))
    notes += note(b"CORE", 0x53494749, struct.pack("<iii", 11, 0, 1) + rnd(116))
    notes += note(b"CORE", 6, b"".join(struct.pack("<QQ", k, rng.randrange(1 << 40)) for k in
                                       (33, 16, 6, 17, 3, 4, 5, 7, 8, 9, 11, 12, 13, 14, 23, 25, 31, 0)))
    nt_file = [(m[0], m[0] + m[1], rng.randrange(64)) for m in maps if m[4] and not m[4].startswith("[")]
    strs = b"".join(m[4].encode() + b"\0" for m in maps if m[4] and not m[4].startswith("["))
    notes += note(b"CORE", 0x46494C45, struct.pack("<QQ", len(nt_file), PAGE) +
                  b"".join(struct.pack("<QQQ", *e) for e in nt_file) + strs)
    notes += note(b"CORE", 2, rnd(512))
    notes += note(b"LINUX", 0x202, rnd(2696))

    phnum = 1 + len(maps)
    hdr_len = 64 + 56 * phnum
    note_off = hdr_len
    seg_off = -(-(note_off + len(notes)) // PAGE) * PAGE
    phdrs = [struct.pack("<IIQQQQQQ", 4, 0, note_off, 0, 0, len(notes), 0, 0)]
    layout = []
    off = seg_off
    for vaddr, memsz, flags, kind, fname in maps:
        if kind in ("guard", "file-ro"):
            filesz = 0
        elif kind == "elfhdr":
            filesz = PAGE
        else:
            filesz = memsz
        phdrs.append(struct.pack("<IIQQQQQQ", 1, flags, off, vaddr, 0, filesz, memsz, PAGE))
        layout.append((off, filesz, kind, vaddr))
        off += filesz
    ehdr = b"\x7fELF\x02\x01\x01" + bytes(9) + struct.pack("<HHIQQQIHHHHHH", 4, 62, 1, 0, 64, 0, 0, 64, 56,
                                                           min(phnum, 0xFFFF), 64, 0, 0)
    w = PageWriter(path)
    w.write_raw(0, ehdr + b"".join(phdrs) + bytes(note_off - hdr_len) + bytes(notes))
    for foff, filesz, kind, vaddr in layout:
        if filesz == 0:
            continue
        if kind == "elfhdr":
            buf = b"\x7fELF\x02\x01\x01" + bytes(9) + struct.pack("<HH", 3, 62) + rnd(PAGE - 20)
        elif kind == "heap":
            buf = heap_bytes(rng, rnd, filesz, vaddr)
        elif kind == "data":
            buf = bytearray(filesz)
            for q in range(0, filesz, PAGE):
                if rng.random() < 0.7:
                    buf[q:q + PAGE] = rnd(PAGE) if rng.random() < 0.3 else \
                        b"".join(struct.pack("<Q", vaddr + rng.randrange(1 << 20)) for _ in range(PAGE // 8))
            buf = bytes(buf)
        elif kind == "arena":
            buf = bytearray(filesz)
            frac = rng.uniform(0.05, 0.35)
            for q in range(0, filesz, PAGE):
                if rng.random() < frac:
                    buf[q:q + PAGE] = heap_bytes(rng, rnd, PAGE, vaddr + q)
            buf = bytes(buf)
        elif kind == "stack":
            buf = bytearray(filesz)
            used = min(filesz, rng.randrange(8, 128) * PAGE)
            buf[filesz - used:] = b"".join(struct.pack("<Q", rng.choice((0, rng.randrange(1 << 47), vaddr + rng.randrange(filesz))))
                                           for _ in range(used // 8))
            buf = bytes(buf)
        else:  # vdso
            buf = b"\x7fELF" + rnd(filesz - 4)
        w.write_pages(foff, buf)
    return w.close(off)


def write_wal(path, rng, rnd, seg_size, fill):
    w = PageWriter(path, 0o644)
    used = int(seg_size * fill) // PAGE * PAGE
    buf = bytearray()
    words = [b"series", b"sample", b"tombstone", b"exemplar", b"metadata", b"http_requests_total",
             b"node_cpu_seconds_total", b"job=\"api\"", b"instance=\"10.0.0.", b"le=\"0.25\""]
    while len(buf) < used:
        rec = b" ".join(rng.choice(words) for _ in range(rng.randrange(2, 9))) + struct.pack("<qd", EPOCH * 1000, rng.random())
        buf += struct.pack("<BH", rng.choice((1, 2, 3, 4)), len(rec)) + struct.pack("<I", hashlib.sha256(rec).digest()[0]) + rec
        if len(buf) // PAGE != (len(buf) - len(rec)) // PAGE and rng.random() < 0.3:
            buf += bytes((-len(buf)) % PAGE)  # page padding
    w.write_raw(0, bytes(buf[:used])) if used else None
    return w.close(seg_size)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True)
    ap.add_argument("--seed", type=int, required=True)
    ap.add_argument("--params", default="{}")
    args = ap.parse_args()
    p = json.loads(args.params)
    rng = random.Random(args.seed)
    rnd = Shake(args.seed, b"ebrc-g2-f15-coredump")
    infos = []
    for i in range(int(p.get("cores", 3))):
        pid = rng.randrange(1000, 400000)
        path = os.path.join(args.out, "var/lib/systemd/coredump",
                            f"core.ebrc-worker.0.{hashlib.sha256(rnd(16)).hexdigest()[:32]}.{pid}.{EPOCH - i * 86400}000000")
        infos.append(write_core(path, rng, rnd, p))
    for i in range(int(p.get("wal_segments", 2))):
        seg = int(p.get("wal_segment_mib", 64)) * MiB
        fill = 1.0 if i < int(p.get("wal_segments", 2)) - 1 else rng.uniform(0.05, 0.4)
        infos.append(write_wal(os.path.join(args.out, "tsdb/wal", f"{i:08d}"), rng, rnd, seg, fill))
    paths = []
    for root, dnames, fnames in os.walk(args.out):
        for n in fnames + dnames:
            paths.append(os.path.relpath(os.path.join(root, n), args.out))
    for i, rel in enumerate(sorted(paths, key=lambda s: (-s.count(os.sep), s))):
        t = (EPOCH - 3600 * (i + 1)) * 10**9
        os.utime(os.path.join(args.out, rel), ns=(t, t), follow_symlinks=False)
    print(json.dumps({"files": len(infos), "apparent": sum(x["size"] for x in infos),
                      "data": sum(x["data"] for x in infos), "extents": sum(x["extents"] for x in infos)}), file=sys.stderr)


if __name__ == "__main__":
    main()
