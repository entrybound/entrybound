#!/usr/bin/env python3
"""Generator (g4-binary F14 tuning): nested archives built with the Python standard library.  GENERATED DATA.

Contract: python gen_nested_archives.py --out <staging> --seed N --params <JSON>
params: profile ("mixed-v1")

Layout (~12 MiB):
  plain/                         ordinary files next to the archives (text, JSON, random, zeros)
  zip-of-zips-stored.zip         stored outer zip holding 8 deflated inner zips
  zip-of-zips-deflated.zip       deflated outer zip holding 6 stored inner zips
  tar-of-targz.tar               ustar holding 5 .tar.gz members
  nested3.tar.xz                 tar.xz -> inner.zip -> innermost.tar.gz -> files      (3 archive levels)
  app-fat.jar                    jar-like zip: META-INF/MANIFEST.MF, classes/*.class-like blobs,
                                 BOOT-INF/lib/*.jar stored uncompressed (Spring-Boot style)
  pkg-1.0-py3-none-any.whl       wheel-like zip containing a data zip and a .tar.bz2
  deep/a/b/c/bundle.tgz          tar.gz holding a zip holding a tar (3 levels) placed deep in the tree
  mixed.zip                      zip whose members are .gz, .bz2, .xz single-file streams and a tar
All timestamps fixed (zip 2026-01-01 00:00:00, tar/gzip mtime 1767225600 / 0), entries sorted,
ownership 0:0; content from random.Random(seed).
"""

import argparse
import bz2
import gzip
import io
import json
import lzma
import os
import random
import tarfile
import zipfile

EPOCH = 1767225600
ZDATE = (2026, 1, 1, 0, 0, 0)
WORDS = ("archive entry bound chunk index manifest stream codec block extent header footer delta object "
         "tree path mode owner group sparse hole link planner random access dedup window table offset").split()


class Gen:
    def __init__(self, seed):
        self.rng = random.Random(seed)

    def text(self, n):
        out, total = [], 0
        while total < n:
            w = self.rng.choice(WORDS)
            out.append(w)
            total += len(w) + 1
            if self.rng.random() < 0.08:
                out.append("\n")
        return (" ".join(out)[:n] + "\n").encode()

    def json_doc(self, n_items):
        items = [{"id": i, "name": self.rng.choice(WORDS), "size": self.rng.randrange(10**6),
                  "tags": self.rng.sample(WORDS, 3)} for i in range(n_items)]
        return json.dumps({"items": items}, indent=1, sort_keys=True).encode()

    def blob(self, n):
        return self.rng.randbytes(n)

    def files(self, count, max_size, prefix="f"):
        out = []
        for i in range(count):
            kind = self.rng.choice(("text", "text", "json", "blob", "zeros"))
            size = self.rng.randrange(64, max_size)
            if kind == "text":
                data, ext = self.text(size), ".txt"
            elif kind == "json":
                data, ext = self.json_doc(max(1, size // 120)), ".json"
            elif kind == "blob":
                data, ext = self.blob(size), ".bin"
            else:
                data, ext = bytes(size), ".dat"
            out.append((f"{prefix}{i:03d}{ext}", data))
        return out


def zip_bytes(entries, method=zipfile.ZIP_DEFLATED):
    bio = io.BytesIO()
    with zipfile.ZipFile(bio, "w") as z:
        for name, data in sorted(entries, key=lambda e: e[0]):
            zi = zipfile.ZipInfo(name, date_time=ZDATE)
            zi.compress_type = method
            zi.external_attr = 0o100644 << 16
            zi.create_system = 3
            z.writestr(zi, data)
    return bio.getvalue()


def tar_bytes(entries, mode="w"):
    bio = io.BytesIO()
    with tarfile.open(fileobj=bio, mode=mode, format=tarfile.USTAR_FORMAT) as t:
        for name, data in sorted(entries, key=lambda e: e[0]):
            ti = tarfile.TarInfo(name)
            ti.size = len(data)
            ti.mtime = EPOCH
            ti.mode = 0o644
            ti.uid = ti.gid = 0
            ti.uname = ti.gname = "root"
            t.addfile(ti, io.BytesIO(data))
    return bio.getvalue()


def gz(data):
    return gzip.compress(data, compresslevel=6, mtime=0)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True)
    ap.add_argument("--seed", type=int, required=True)
    ap.add_argument("--params", default="{}")
    a = ap.parse_args()
    p = json.loads(a.params)
    if p.get("profile", "mixed-v1") != "mixed-v1":
        raise SystemExit("unknown profile")
    g = Gen(a.seed)
    out = a.out

    def write(rel, data):
        path = os.path.join(out, rel)
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, "wb") as f:
            f.write(data)
        os.chmod(path, 0o644)

    for name, data in g.files(30, 60000, "plain"):
        write(f"plain/{name}", data)

    inner = [(f"inner{i}.zip", zip_bytes(g.files(20, 40000, f"i{i}_"))) for i in range(8)]
    write("zip-of-zips-stored.zip", zip_bytes(inner, zipfile.ZIP_STORED))
    inner = [(f"stored{i}.zip", zip_bytes(g.files(15, 50000, f"s{i}_"), zipfile.ZIP_STORED)) for i in range(6)]
    write("zip-of-zips-deflated.zip", zip_bytes(inner, zipfile.ZIP_DEFLATED))

    members = [(f"part{i}.tar.gz", gz(tar_bytes([(f"part{i}/{n}", d) for n, d in g.files(25, 40000)])))
               for i in range(5)]
    write("tar-of-targz.tar", tar_bytes(members))

    innermost = gz(tar_bytes([(f"leaf/{n}", d) for n, d in g.files(30, 30000, "leaf")]))
    inner_zip = zip_bytes([("innermost.tar.gz", innermost), ("README.txt", g.text(2000))], zipfile.ZIP_STORED)
    write("nested3.tar.xz", lzma.compress(tar_bytes([("level1/inner.zip", inner_zip), ("level1/notes.txt", g.text(5000))]),
                                          preset=6))

    jars = [(f"BOOT-INF/lib/dep{i}-1.{i}.jar",
             zip_bytes([("META-INF/MANIFEST.MF", b"Manifest-Version: 1.0\r\nCreated-By: ebrc-gen\r\n\r\n")]
                       + [(f"com/example/dep{i}/C{j}.class", b"\xca\xfe\xba\xbe\x00\x00\x00\x41" + g.blob(g.rng.randrange(300, 4000)))
                          for j in range(40)]))
            for i in range(10)]
    classes = [(f"BOOT-INF/classes/com/example/app/K{j}.class", b"\xca\xfe\xba\xbe\x00\x00\x00\x41" + g.blob(2000))
               for j in range(30)]
    manifest = [("META-INF/MANIFEST.MF", b"Manifest-Version: 1.0\r\nMain-Class: org.example.Launcher\r\n\r\n")]
    bio = io.BytesIO()
    with zipfile.ZipFile(bio, "w") as z:
        for name, data in manifest + sorted(classes) + sorted(jars):
            zi = zipfile.ZipInfo(name, date_time=ZDATE)
            zi.compress_type = zipfile.ZIP_STORED if name.endswith(".jar") else zipfile.ZIP_DEFLATED
            zi.external_attr = 0o100644 << 16
            zi.create_system = 3
            z.writestr(zi, data)
    write("app-fat.jar", bio.getvalue())

    whl = [("pkg/__init__.py", b"__version__ = '1.0'\n"),
           ("pkg/data/resources.zip", zip_bytes(g.files(20, 20000, "res"))),
           ("pkg/data/fixtures.tar.bz2", bz2.compress(tar_bytes(g.files(10, 30000, "fx")), 9)),
           ("pkg-1.0.dist-info/METADATA", b"Metadata-Version: 2.1\nName: pkg\nVersion: 1.0\n"),
           ("pkg-1.0.dist-info/WHEEL", b"Wheel-Version: 1.0\nGenerator: ebrc-gen\nRoot-Is-Purelib: true\nTag: py3-none-any\n")]
    write("pkg-1.0-py3-none-any.whl", zip_bytes(whl))

    lvl3 = tar_bytes([(f"x/{n}", d) for n, d in g.files(12, 20000, "deep")])
    lvl2 = zip_bytes([("level3.tar", lvl3)])
    write("deep/a/b/c/bundle.tgz", gz(tar_bytes([("bundle/level2.zip", lvl2)])))

    mixed = [("one.txt.gz", gz(g.text(40000))), ("two.bin.bz2", bz2.compress(g.blob(20000) + bytes(20000), 9)),
             ("three.json.xz", lzma.compress(g.json_doc(300))), ("four.tar", tar_bytes(g.files(8, 20000, "m")))]
    write("mixed.zip", zip_bytes(mixed, zipfile.ZIP_STORED))


if __name__ == "__main__":
    main()
