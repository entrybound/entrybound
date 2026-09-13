#!/usr/bin/env python3
"""Generated many-small-file trees for family F04 (corpus group g2-generated).  GENERATED DATA.

Models (params.model):
  sourcelike  Project/source-tree-like hierarchy: Zipf-weighted directory fan-out, lognormal file
              sizes (median ~1.5 KiB), synthetic code/doc/config text built from a seeded token
              model with a few shared licence headers, valid tiny PNG icons, random fixture blobs,
              ~3% empty files (__init__.py, .gitkeep).
  maildir     Maildir++ mail store: per-user cur/new/tmp folders and IMAP subfolders, RFC 5322
              style synthetic messages (example.* domains, invented names, generated prose),
              occasional base64 attachments of random bytes, dovecot-like index/uidlist files.
  objstore    Content-addressed stores: git loose objects (real object format: zlib-deflated
              "<type> <len>\\0payload" named by SHA-1 in 256 fan-out dirs; blobs/trees/commits),
              plus a ccache-like two-level hashed cache of small binary result files.

Common params: files (int, approximate number of regular files).
Seeded by --seed through random.Random (structure) and SHAKE-256 counter mode (bytes).
All names/addresses are invented; no real personal data.  mtimes are set deterministically.
"""

import argparse
import hashlib
import json
import math
import os
import random
import struct
import sys
import zlib
from email.utils import formatdate

EPOCH = int(os.environ.get("SOURCE_DATE_EPOCH", "1767225600"))


class Shake:
    """Seeded CSPRNG: SHAKE-256(domain | seed | counter) blocks."""

    def __init__(self, seed: int, domain: bytes):
        self.prefix = domain + b"\0" + struct.pack("<Q", seed & (2**64 - 1))
        self.counter = 0

    def bytes(self, n: int) -> bytes:
        out = bytearray()
        while len(out) < n:
            take = min(n - len(out), 1 << 20)
            out += hashlib.shake_256(self.prefix + struct.pack("<Q", self.counter)).digest(take)
            self.counter += 1
        return bytes(out)


SYLL = ("ka ri to mo na lu be si ve ro da ne xi pa qu fe zo li ma te ho gu ja ce wi yo ba "
        "de fi ga hi ko la me ni po ra se ti vu wa xe yu ze").split()
DIR_WORDS = ("src lib include tests test docs doc internal util utils core api v2 fixtures examples "
             "scripts tools cmd pkg vendor assets static templates config migrations bench data "
             "i18n locale proto schema build ci contrib extra plugins drivers net io fs mem sync").split()


def make_words(rng: random.Random, n: int) -> list:
    words = set()
    while len(words) < n:
        k = rng.choice((2, 2, 3, 3, 4))
        words.add("".join(rng.choice(SYLL) for _ in range(k)))
    return sorted(words)


class Mtimes:
    def __init__(self):
        self.items = []

    def add(self, path, t):
        self.items.append((path, t))

    def apply(self, out):
        files = [(p, t) for p, t in self.items]
        for p, t in files:
            os.utime(p, ns=(t, t), follow_symlinks=False)
        dirs = []
        for root, dnames, _ in os.walk(out):
            for d in dnames:
                dirs.append(os.path.relpath(os.path.join(root, d), out))
        dirs.sort(key=lambda s: (-s.count(os.sep), s))
        for d in dirs:
            t = (EPOCH - 86400 * 30 - (hash_str(d) % (86400 * 400))) * 10**9
            os.utime(os.path.join(out, d), ns=(t, t), follow_symlinks=False)


def hash_str(s: str) -> int:
    return int.from_bytes(hashlib.sha256(s.encode("utf-8", "surrogateescape")).digest()[:8], "little")


def lognormal_size(rng, median, sigma, lo, hi):
    v = int(math.exp(math.log(median) + sigma * rng.gauss(0.0, 1.0)))
    return max(lo, min(hi, v))


def png_bytes(rng: random.Random, shake: Shake, w: int, h: int) -> bytes:
    def chunk(tag, data):
        return struct.pack(">I", len(data)) + tag + data + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)
    palette_n = rng.choice((2, 4, 16))
    palette = shake.bytes(3 * palette_n)
    raw = bytearray()
    bits = {2: 1, 4: 2, 16: 4}[palette_n]
    row_bytes = (w * bits + 7) // 8
    for _ in range(h):
        raw.append(0)
        raw += bytes(rng.randrange(256) for _ in range(row_bytes))
    ihdr = struct.pack(">IIBBBBB", w, h, bits, 3, 0, 0, 0)
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", ihdr) + chunk(b"PLTE", palette) + \
        chunk(b"IDAT", zlib.compress(bytes(raw), 9)) + chunk(b"IEND", b"")


# ---------------------------------------------------------------------------------------------
# sourcelike

LICENSE_HEADERS = [
    "// SPDX-License-Identifier: MIT\n// Copyright (c) {y} Example Contributors\n\n",
    "/*\n * SPDX-License-Identifier: Apache-2.0\n * Copyright {y} The Example Authors.\n */\n\n",
    "# SPDX-License-Identifier: BSD-3-Clause\n# Copyright (c) {y}, Example Project developers\n\n",
    "// SPDX-License-Identifier: MPL-2.0\n// This file is part of the example suite ({y}).\n\n",
    "/* SPDX-License-Identifier: GPL-2.0-or-later */\n/* (c) {y} example.org hackers */\n\n",
    "# SPDX-License-Identifier: ISC\n# Generated fixture header {y}\n\n",
]

LANGS = {
    "c": ([".c", ".h"], 0.18), "py": ([".py"], 0.16), "js": ([".js", ".ts"], 0.14), "rs": ([".rs"], 0.08),
    "go": ([".go"], 0.07), "java": ([".java"], 0.05), "doc": ([".md", ".rst", ".txt"], 0.10),
    "conf": ([".json", ".yaml", ".toml", ".xml", ".ini"], 0.12), "bin": ([".png", ".bin", ".dat", ".ico"], 0.07),
    "empty": ([".gitkeep", "__init__.py", ".keep"], 0.03),
}


def line_pool(rng: random.Random, words: list, lang: str, n: int) -> list:
    w = lambda: rng.choice(words)  # noqa: E731
    lines = []
    for _ in range(n):
        ind = "    " * rng.choice((0, 1, 1, 2, 2, 3))
        r = rng.random()
        if lang == "c":
            opts = [f"{ind}static int {w()}_{w()}(struct {w()} *{w()}, size_t {w()});",
                    f"{ind}if ({w()}->{w()} == {rng.randrange(256)}) {{", f"{ind}}}",
                    f"{ind}{w()}_{w()}({w()}, {w()}, {rng.randrange(1 << 16)});",
                    f"#define {w().upper()}_{w().upper()} 0x{rng.randrange(1 << 16):04x}",
                    f"{ind}/* {w()} {w()} {w()} {w()} */", f"#include <{w()}/{w()}.h>", ""]
        elif lang == "py":
            opts = [f"{ind}def {w()}_{w()}(self, {w()}, {w()}=None):", f"{ind}return {w()}.{w()}({w()})",
                    f"{ind}if {w()} is not None and {w()} > {rng.randrange(100)}:",
                    f"{ind}{w()} = {w()}.get(\"{w()}\", {rng.randrange(1000)})", f"import {w()}.{w()}",
                    f"{ind}# {w()} {w()} {w()}", f"class {w().capitalize()}{w().capitalize()}:", ""]
        elif lang == "js":
            opts = [f"{ind}export function {w()}{w().capitalize()}({w()}, {w()}) {{",
                    f"{ind}const {w()} = await {w()}.{w()}({rng.randrange(100)});", f"{ind}}}",
                    f"import {{ {w()} }} from './{w()}/{w()}.js';", f"{ind}// {w()} {w()} {w()}",
                    f"{ind}if (!{w()}) throw new Error('{w()} {w()}');", ""]
        elif lang == "rs":
            opts = [f"{ind}pub fn {w()}_{w()}(&self, {w()}: &[u8]) -> Result<{w().capitalize()}> {{",
                    f"{ind}let {w()} = {w()}.{w()}()?;", f"{ind}}}", f"use crate::{w()}::{w().capitalize()};",
                    f"{ind}/// {w()} {w()} {w()} {w()}", f"{ind}match {w()} {{", ""]
        elif lang == "go":
            opts = [f"func ({w()[0]} *{w().capitalize()}) {w().capitalize()}({w()} int) error {{",
                    f"{ind}if err := {w()}.{w().capitalize()}(); err != nil {{", f"{ind}return err", f"{ind}}}",
                    f"{ind}// {w()} {w()} {w()}", ""]
        elif lang == "java":
            opts = [f"{ind}public {w().capitalize()} get{w().capitalize()}() {{",
                    f"{ind}private final {w().capitalize()} {w()} = new {w().capitalize()}();",
                    f"{ind}return this.{w()};", f"{ind}}}", f"import org.example.{w()}.{w().capitalize()};", ""]
        elif lang == "doc":
            n = rng.randrange(4, 16)
            opts = [" ".join(w() for _ in range(n)).capitalize() + ".", f"## {w().capitalize()} {w()}",
                    f"- `{w()}`: {w()} {w()} {w()}", ""]
        else:  # conf
            opts = [f"{ind}\"{w()}\": \"{w()}-{rng.randrange(1000)}\",", f"{ind}{w()}: {rng.randrange(10000)}",
                    f"{ind}{w()}_{w()} = \"{w()}\"", f"{ind}<{w()} id=\"{rng.randrange(1 << 20)}\">{w()}</{w()}>",
                    f"[{w()}.{w()}]", ""]
        lines.append(opts[min(len(opts) - 1, int(r * len(opts)))])
    return lines


def gen_sourcelike(out, rng, shake, p, mt):
    n_files = int(p.get("files", 3000))
    words = make_words(rng, 6000)
    pools = {lang: line_pool(rng, words, lang, 20000) for lang in ("c", "py", "js", "rs", "go", "java", "doc", "conf")}
    # directory hierarchy
    dirs = []
    n_projects = max(1, n_files // 700)
    for pi in range(n_projects):
        proj = f"{rng.choice(words)}-{rng.choice(words)}"
        stack = [(proj, 0)]
        while stack:
            d, depth = stack.pop()
            dirs.append(d)
            if depth < 7:
                for _ in range(min(8, int(rng.paretovariate(1.6)) - (1 if depth > 2 else 0))):
                    name = rng.choice(DIR_WORDS) if rng.random() < 0.6 else rng.choice(words)
                    stack.append((f"{d}/{name}", depth + 1))
    dirs = sorted(set(dirs))
    cum = []
    acc = 0.0
    for d in dirs:
        acc += 1.0 / (1 + (hash_str(d) % 97)) ** 0.8
        cum.append(acc)
    for d in dirs:
        os.makedirs(os.path.join(out, d), exist_ok=True)
    langs = list(LANGS)
    lang_w = [LANGS[k][1] for k in langs]
    used = set()
    total = 0
    for i in range(n_files):
        d = rng.choices(dirs, cum_weights=cum)[0]
        lang = rng.choices(langs, lang_w)[0]
        exts = LANGS[lang][0]
        if lang == "empty":
            name = rng.choice(exts)
        else:
            name = rng.choice(words) + ("_" + rng.choice(words) if rng.random() < 0.4 else "") + rng.choice(exts)
        rel = f"{d}/{name}"
        k = 1
        while rel in used:
            base, dot, ext = name.rpartition(".")
            rel = f"{d}/{base or ext}{k}{dot}{ext if base else ''}"
            k += 1
        used.add(rel)
        path = os.path.join(out, rel)
        if lang == "empty":
            data = b""
        elif lang == "bin":
            if rel.endswith((".png", ".ico")):
                s = rng.choice((8, 16, 16, 24, 32, 48))
                data = png_bytes(rng, shake, s, s)
            else:
                data = shake.bytes(lognormal_size(rng, 900, 1.5, 1, 256 * 1024))
        else:
            size = lognormal_size(rng, 1500, 1.35, 16, 400 * 1024)
            pool = pools[lang]
            parts = []
            if lang not in ("doc", "conf") and rng.random() < 0.7:
                parts.append(LICENSE_HEADERS[rng.randrange(len(LICENSE_HEADERS))].format(y=rng.randrange(2008, 2026)))
            cur = sum(len(x) for x in parts)
            while cur < size:
                ln = pool[rng.randrange(len(pool))] + "\n"
                parts.append(ln)
                cur += len(ln)
            data = "".join(parts).encode("utf-8")
        with open(path, "wb") as f:
            f.write(data)
        total += len(data)
        mode = 0o755 if (lang == "py" and rng.random() < 0.05) or rel.endswith(".sh") else 0o644
        os.chmod(path, mode)
        mt.add(path, (EPOCH - rng.randrange(86400 * 900)) * 10**9 + rng.randrange(10**9))
    return {"files": n_files, "dirs": len(dirs), "bytes": total}


# ---------------------------------------------------------------------------------------------
# maildir

def gen_maildir(out, rng, shake, p, mt):
    n_msgs = int(p.get("files", 20000))
    n_users = int(p.get("users", 12))
    words = make_words(rng, 8000)
    first = sorted({w.capitalize() for w in words[:600]})
    domains = [f"{rng.choice(words)}.example.{tld}" for tld in ("com", "org", "net") for _ in range(6)]
    folders = ["", ".Sent", ".Drafts", ".Trash", ".Archive", ".Archive.2024", ".Archive.2025", ".Lists",
               ".Lists.dev", ".Lists.announce", ".Junk"]
    total = 0
    count = 0
    per_user = max(1, n_msgs // n_users)
    for u in range(n_users):
        user = f"{rng.choice(first).lower()}{u}"
        base = os.path.join(out, "Maildir", user)
        ufolders = folders[: rng.randrange(4, len(folders) + 1)]
        for fo in ufolders:
            for sub in ("cur", "new", "tmp"):
                os.makedirs(os.path.join(base, fo, sub) if fo else os.path.join(base, sub), exist_ok=True)
        uid_lines = [f"3 V{EPOCH - u * 1000} N{per_user + 1} G{hashlib.md5(user.encode()).hexdigest()}"]
        with open(os.path.join(base, "subscriptions"), "w") as f:
            f.write("".join(fo.lstrip(".") + "\n" for fo in ufolders if fo))
        mt.add(os.path.join(base, "subscriptions"), (EPOCH - 86400) * 10**9)
        for m in range(per_user):
            fo = rng.choices(ufolders, [8] + [1] * (len(ufolders) - 1))[0]
            sub = "cur" if rng.random() < 0.93 else "new"
            t = EPOCH - rng.randrange(86400 * 1500)
            sender = f"{rng.choice(first)} {rng.choice(first)}"
            addr = f"{sender.split()[0].lower()}.{sender.split()[1].lower()}@{rng.choice(domains)}"
            subj = " ".join(rng.choice(words) for _ in range(rng.randrange(2, 9))).capitalize()
            mid = f"<{shake.bytes(8).hex()}.{t}@{rng.choice(domains)}>"
            body_len = lognormal_size(rng, 1800, 1.1, 40, 200000)
            body = []
            cur = 0
            while cur < body_len:
                sent = " ".join(rng.choice(words) for _ in range(rng.randrange(5, 18))).capitalize() + "."
                if rng.random() < 0.08:
                    sent = "> " + sent
                body.append(sent + ("\n\n" if rng.random() < 0.15 else "\n"))
                cur += len(sent) + 1
            hdr = [f"Return-Path: <{addr}>"]
            for hop in range(rng.randrange(1, 5)):
                hdr.append(f"Received: from mx{hop}.{rng.choice(domains)} (mx{hop}.{rng.choice(domains)} "
                           f"[192.0.2.{rng.randrange(1, 255)}])\n\tby mail.example.net with ESMTPS id "
                           f"{shake.bytes(6).hex().upper()}; {formatdate(t - hop * 3, usegmt=True)}")
            hdr += [f"Message-ID: {mid}", f"Date: {formatdate(t, usegmt=True)}", f"From: {sender} <{addr}>", f"To: {user}@mail.example.net",
                    f"Subject: {subj}", "MIME-Version: 1.0"]
            text = "".join(body)
            if rng.random() < 0.12:
                boundary = "=_" + shake.bytes(12).hex()
                att = shake.bytes(lognormal_size(rng, 12000, 1.0, 300, 400000))
                b64 = __import__("base64").encodebytes(att).decode()
                hdr.append(f"Content-Type: multipart/mixed; boundary=\"{boundary}\"")
                msg = "\n".join(hdr) + f"\n\n--{boundary}\nContent-Type: text/plain; charset=utf-8\n\n{text}\n" \
                    f"--{boundary}\nContent-Type: application/octet-stream; name=\"{rng.choice(words)}.bin\"\n" \
                    f"Content-Transfer-Encoding: base64\n\n{b64}--{boundary}--\n"
            else:
                hdr.append("Content-Type: text/plain; charset=utf-8")
                msg = "\n".join(hdr) + "\n\n" + text
            data = msg.replace("\n", "\r\n" if rng.random() < 0.1 else "\n").encode("utf-8")
            flags = "".join(sorted(set(rng.sample("DFPRT", rng.randrange(0, 3))) | {"S"}))
            name = f"{t}.M{rng.randrange(10**6)}P{rng.randrange(2, 65536)}Q{m}.mail.example.net,S={len(data)}," \
                   f"W={len(data) + msg.count(chr(10))}"
            if sub == "cur":
                name += f":2,{flags}"
            path = os.path.join(base, fo, sub, name) if fo else os.path.join(base, sub, name)
            with open(path, "wb") as f:
                f.write(data)
            os.chmod(path, 0o600)
            mt.add(path, t * 10**9)
            uid_lines.append(f"{m + 1} :{name.split(':')[0]}")
            total += len(data)
            count += 1
        with open(os.path.join(base, "dovecot-uidlist"), "w") as f:
            f.write("\n".join(uid_lines) + "\n")
        idx = shake.bytes(rng.randrange(2000, 60000))
        with open(os.path.join(base, "dovecot.index.cache"), "wb") as f:
            f.write(b"\x08\x02\x00\x00" + idx)
        for extra in ("dovecot-uidlist", "dovecot.index.cache"):
            mt.add(os.path.join(base, extra), (EPOCH - 3600) * 10**9)
    return {"messages": count, "users": n_users, "bytes": total}


# ---------------------------------------------------------------------------------------------
# objstore

def gen_objstore(out, rng, shake, p, mt):
    n = int(p.get("files", 20000))
    ccache_frac = float(p.get("ccache_fraction", 0.35))
    words = make_words(rng, 5000)
    n_git = int(n * (1 - ccache_frac))
    n_cc = n - n_git
    git = os.path.join(out, "repo.git")
    os.makedirs(os.path.join(git, "objects", "info"), exist_ok=True)
    os.makedirs(os.path.join(git, "objects", "pack"), exist_ok=True)
    os.makedirs(os.path.join(git, "refs", "heads"), exist_ok=True)
    os.makedirs(os.path.join(git, "refs", "tags"), exist_ok=True)
    total = 0
    blobs = []

    def put(kind: bytes, payload: bytes) -> bytes:
        nonlocal total
        raw = kind + b" " + str(len(payload)).encode() + b"\0" + payload
        sha = hashlib.sha1(raw).digest()
        hx = sha.hex()
        d = os.path.join(git, "objects", hx[:2])
        os.makedirs(d, exist_ok=True)
        path = os.path.join(d, hx[2:])
        if not os.path.exists(path):
            data = zlib.compress(raw, 6)
            with open(path, "wb") as f:
                f.write(data)
            os.chmod(path, 0o444)
            mt.add(path, (EPOCH - rng.randrange(86400 * 700)) * 10**9)
            total += len(data)
        return sha

    made = 0
    commits = []
    parent = None
    while made < n_git:
        # a commit: a handful of new blobs, one or two trees, one commit
        entries = []
        for _ in range(rng.randrange(1, 12)):
            if blobs and rng.random() < 0.1:
                sha, name = blobs[rng.randrange(len(blobs))]
            else:
                if rng.random() < 0.85:
                    size = lognormal_size(rng, 900, 1.3, 1, 60000)
                    lines = []
                    cur = 0
                    while cur < size:
                        ln = " ".join(rng.choice(words) for _ in range(rng.randrange(1, 12))) + "\n"
                        lines.append(ln)
                        cur += len(ln)
                    payload = "".join(lines).encode()
                else:
                    payload = shake.bytes(lognormal_size(rng, 2000, 1.2, 1, 80000))
                name = rng.choice(words) + rng.choice((".c", ".h", ".py", ".md", ".txt", ".json", ""))
                sha = put(b"blob", payload)
                made += 1
                blobs.append((sha, name))
            entries.append((rng.choice((b"100644", b"100644", b"100755")), name.encode(), sha))
        seen = set()
        tree = b""
        for mode, name, sha in sorted(entries, key=lambda e: e[1]):
            if name in seen:
                continue
            seen.add(name)
            tree += mode + b" " + name + b"\0" + sha
        tsha = put(b"tree", tree)
        made += 1
        t = EPOCH - 86400 * 700 + len(commits) * 977
        who = f"{rng.choice(words).capitalize()} {rng.choice(words).capitalize()} <{rng.choice(words)}@example.org>"
        body = f"tree {tsha.hex()}\n" + (f"parent {parent.hex()}\n" if parent else "") + \
            f"author {who} {t} +0000\ncommitter {who} {t} +0000\n\n" + \
            " ".join(rng.choice(words) for _ in range(rng.randrange(3, 12))).capitalize() + "\n"
        parent = put(b"commit", body.encode())
        made += 1
        commits.append(parent)
    refs = {"HEAD": b"ref: refs/heads/main\n", "refs/heads/main": parent.hex().encode() + b"\n",
            "description": b"Unnamed repository; edit this file 'description' to name the repository.\n",
            "config": b"[core]\n\trepositoryformatversion = 0\n\tfilemode = true\n\tbare = true\n"}
    for i in range(0, len(commits), max(1, len(commits) // 40)):
        refs[f"refs/tags/v{i // max(1, len(commits) // 40)}.0"] = commits[i].hex().encode() + b"\n"
    for rel, data in refs.items():
        path = os.path.join(git, rel)
        with open(path, "wb") as f:
            f.write(data)
        mt.add(path, EPOCH * 10**9)
        total += len(data)
    # ccache-like
    cc = os.path.join(out, "ccache")
    for i in range(n_cc):
        key = shake.bytes(16).hex()
        d = os.path.join(cc, key[0], key[1])
        os.makedirs(d, exist_ok=True)
        size = lognormal_size(rng, 3500, 1.4, 60, 2 * 1024 * 1024)
        body = shake.bytes(size - 20)
        data = b"cCrS" + struct.pack("<BBHIQ", 1, 2, 0, rng.randrange(1 << 32), size) + body
        path = os.path.join(d, key[2:] + rng.choice(("R", "R", "R", "M")))
        with open(path, "wb") as f:
            f.write(data)
        mt.add(path, (EPOCH - rng.randrange(86400 * 60)) * 10**9)
        total += len(data)
    for a in "0123456789abcdef":
        path = os.path.join(cc, a, "stats")
        if os.path.isdir(os.path.dirname(path)):
            with open(path, "w") as f:
                f.write("".join(f"{rng.randrange(100000)}\n" for _ in range(40)))
            mt.add(path, EPOCH * 10**9)
    return {"git_objects": made, "ccache_files": n_cc, "bytes": total}


MODELS = {"sourcelike": gen_sourcelike, "maildir": gen_maildir, "objstore": gen_objstore}


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True)
    ap.add_argument("--seed", type=int, required=True)
    ap.add_argument("--params", default="{}")
    args = ap.parse_args()
    p = json.loads(args.params)
    model = p.get("model")
    if model not in MODELS:
        raise SystemExit(f"small_file_tree: params.model must be one of {sorted(MODELS)}")
    rng = random.Random(args.seed)
    shake = Shake(args.seed, b"ebrc-g2-f04-" + model.encode())
    mt = Mtimes()
    summary = MODELS[model](args.out, rng, shake, p, mt)
    mt.apply(args.out)
    print(json.dumps(summary, sort_keys=True), file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
