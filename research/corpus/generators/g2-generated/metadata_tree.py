#!/usr/bin/env python3
"""Generated metadata-heavy filesystem trees for family F19 (corpus group g2-generated).  GENERATED DATA.

Must run as root on a Linux filesystem with user/trusted xattrs and POSIX ACLs (ext4 in WSL).
Every metadata operation is checked; an unsupported feature fails loudly instead of silently
producing a different tree.

Profiles (params.profile):
  zoo     broad coverage: exhaustive file permission matrix (all 4096 mode values incl. setuid/setgid/
          sticky), directory mode variants, varied uids/gids (0 .. 4294967294), user.*/trusted.*/
          security.capability/security.selinux xattrs, access and default POSIX ACLs, hardlink groups
          (incl. one group of 1000 links), symlinks (relative, absolute, dangling, loops, self-loop,
          40-link chain, 4095-byte target, non-UTF-8 target, escaping the root), FIFOs, char/block
          device nodes, a unix socket, empty files and empty directory forests, deep nesting,
          Unicode (NFC/NFD pairs, CJK, RTL, emoji/ZWJ, confusables), Windows-hostile and
          non-UTF-8 names, case-colliding names, 255-byte names, extreme mtimes (1901 .. 2446).
  deep    depth and width: a 1000-level one-character directory chain, a chain of 200-byte names close
          to PATH_MAX, a 20000-entry directory, default-ACL inheritance through a subtree (ACLs
          applied by the kernel at create time), symlinks climbing hundreds of levels, hardlinks
          between the deepest and shallowest directories, xattrs on directories, script families
          (Devanagari, Thai, Georgian, decomposed Hangul, bidi controls, homoglyphs).
  matrix  (held-out construction) owner x group x mode x ACL-variant cross product, xattr value-size
          ladder up to the ext4 single-block limit and 255-byte xattr names, names drawn from random
          Unicode code points and random invalid byte strings, an alternatives-style two-level symlink
          farm, a bin/sbin/usr-bin style hardlink farm, timestamps uniformly random over the ext4 range,
          device nodes with random major/minor numbers.

params: profile, scale (float, multiplies counts; default 1.0)
"""

import argparse
import hashlib
import json
import os
import random
import socket
import stat
import struct
import sys
import unicodedata

EPOCH = int(os.environ.get("SOURCE_DATE_EPOCH", "1767225600"))
ACL_UNDEF = 0xFFFFFFFF
USER_OBJ, USER, GROUP_OBJ, GROUP, MASK, OTHER = 0x01, 0x02, 0x04, 0x08, 0x10, 0x20
UIDS = [0, 1, 2, 33, 101, 999, 1000, 1001, 65533, 65534, 65535, 100000, 1 << 20, 2**31 - 1, 2**31, 4294967294]
EXT4_MIN_T = -(2**31)
EXT4_MAX_T = 15032385535  # 2446-05-10, the ext4 extra-epoch limit


def mkdirs(path: bytes) -> None:
    """Iterative os.makedirs (os.makedirs recurses once per level; the deep profile has >1000 levels)."""
    if os.path.isdir(path):
        return
    parts = path.split(b"/")
    cur = b"/" if path.startswith(b"/") else b""
    for part in parts:
        if not part:
            continue
        cur = os.path.join(cur, part) if cur else part
        try:
            os.mkdir(cur)
        except FileExistsError:
            pass


def acl_blob(entries):
    ents = sorted(entries, key=lambda e: (e[0], e[2]))
    return struct.pack("<I", 2) + b"".join(struct.pack("<HHI", t, perm, i) for t, perm, i in ents)


def acl(user_obj=7, group_obj=5, other=5, users=(), groups=(), mask=None):
    e = [(USER_OBJ, user_obj, ACL_UNDEF), (GROUP_OBJ, group_obj, ACL_UNDEF), (OTHER, other, ACL_UNDEF)]
    e += [(USER, perm, uid) for uid, perm in users]
    e += [(GROUP, perm, gid) for gid, perm in groups]
    if users or groups or mask is not None:
        e.append((MASK, 7 if mask is None else mask, ACL_UNDEF))
    return acl_blob(e)


def fscap(permitted, effective=True):
    return struct.pack("<IIIII", 0x02000000 | (1 if effective else 0), permitted & 0xFFFFFFFF, 0, permitted >> 32, 0)


class Tree:
    def __init__(self, out, rng):
        self.out = os.fsencode(out)
        self.rng = rng
        self.meta = []      # deferred (path, kind, value)
        self.times = []     # (path, t_ns)
        self.count = {"files": 0, "dirs": 0, "symlinks": 0, "hardlinks": 0, "fifos": 0, "devices": 0, "sockets": 0,
                      "xattrs": 0, "acls": 0}

    def p(self, rel):
        rel = rel if isinstance(rel, bytes) else rel.encode("utf-8")
        return os.path.join(self.out, rel)

    def dir(self, rel, mode=0o755, owner=None):
        path = self.p(rel)
        mkdirs(path)
        self.count["dirs"] += 1
        self.meta.append((path, "mode", mode))
        if owner:
            self.meta.append((path, "owner", owner))
        return path

    def file(self, rel, data=b"", mode=0o644, owner=None):
        path = self.p(rel)
        mkdirs(os.path.dirname(path))
        with open(path, "xb") as f:
            f.write(data)
        self.count["files"] += 1
        self.meta.append((path, "mode", mode))
        if owner:
            self.meta.append((path, "owner", owner))
        return path

    def symlink(self, rel, target, owner=None):
        path = self.p(rel)
        mkdirs(os.path.dirname(path))
        os.symlink(target if isinstance(target, bytes) else target.encode("utf-8"), path)
        self.count["symlinks"] += 1
        if owner:
            self.meta.append((path, "owner", owner))
        return path

    def link(self, existing_rel, rel):
        path = self.p(rel)
        mkdirs(os.path.dirname(path))
        os.link(self.p(existing_rel), path, follow_symlinks=False)
        self.count["hardlinks"] += 1
        return path

    def fifo(self, rel, mode=0o644):
        path = self.p(rel)
        mkdirs(os.path.dirname(path))
        os.mkfifo(path, 0o600)
        self.count["fifos"] += 1
        self.meta.append((path, "mode", mode))
        return path

    def device(self, rel, block, major, minor, mode=0o660):
        path = self.p(rel)
        mkdirs(os.path.dirname(path))
        os.mknod(path, (stat.S_IFBLK if block else stat.S_IFCHR) | 0o600, os.makedev(major, minor))
        self.count["devices"] += 1
        self.meta.append((path, "mode", mode))
        return path

    def sock(self, rel):
        path = self.p(rel)
        mkdirs(os.path.dirname(path))
        cwd = os.getcwd()
        os.chdir(os.path.dirname(path))
        try:
            s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
            s.bind(os.path.basename(path))
            s.close()
        finally:
            os.chdir(cwd)
        self.count["sockets"] += 1
        self.meta.append((path, "mode", 0o755))
        return path

    def xattr(self, path, name, value):
        self.meta.append((path, "xattr", (name, value)))

    def acl_access(self, path, blob):
        self.meta.append((path, "xattr", ("system.posix_acl_access", blob)))

    def acl_default(self, path, blob):
        self.meta.append((path, "xattr", ("system.posix_acl_default", blob)))

    def mtime(self, path, t_sec, ns=0):
        self.times.append((path, t_sec * 10**9 + ns))

    def apply(self):
        # order: owners, then modes (chown clears setuid/setgid), then xattrs/ACLs (ACLs rewrite group bits)
        for path, kind, val in self.meta:
            if kind == "owner":
                os.lchown(path, *val)
        for path, kind, val in self.meta:
            if kind == "mode":
                if not os.path.islink(path):
                    os.chmod(path, val)
        for path, kind, val in self.meta:
            if kind == "xattr":
                name, value = val
                try:
                    os.setxattr(path, name, value, follow_symlinks=False)
                except OSError as e:
                    raise SystemExit(f"setxattr {name} on {path!r} failed: {e}")
                self.count["acls" if name.startswith("system.posix_acl") else "xattrs"] += 1
        explicit = {}
        for path, t in self.times:
            explicit[path] = t
        all_paths = []
        for root, dnames, fnames in os.walk(self.out):
            for n in fnames + dnames:
                all_paths.append(os.path.join(root, n))
        all_paths.sort(key=lambda s: (-s.count(b"/"), s))
        for path in all_paths:
            rel = os.path.relpath(path, self.out)
            h = int.from_bytes(hashlib.sha256(rel).digest()[:8], "little")
            t = explicit.get(path, (EPOCH - h % (86400 * 3650)) * 10**9 + h % 10**9)
            os.utime(path, ns=(t, t), follow_symlinks=False)


# ---------------------------------------------------------------------------------------------

UNICODE_NAMES = [
    unicodedata.normalize("NFC", "café"), unicodedata.normalize("NFD", "café"),
    unicodedata.normalize("NFC", "Ångström"), unicodedata.normalize("NFD", "Ångström"),
    "日本語のファイル名.txt", "中文文件名", "한국어 파일", unicodedata.normalize("NFD", "한국어"),
    "עברית", "العربية.txt", "emoji-😀-🚀", "zwj-👩‍💻-family-👨‍👩‍👧", "combining-é̂̃",
    "fullwidth-ＡＢＣ", "cyrillic-аpple", "latin-apple", "ohm-Ω", "omega-Ω", "ligature-ﬁle",
    "rtl-override-‮gnp.exe", "zero-width-​space", "nbsp name", "bom-﻿name", "private-use-",
    "astral-\U0001d518\U0001d52b\U0001d526", "noncharacter-￾", "tamil-தமிழ்", "greek-ΑΒΓ", "math-∀x∈ℝ",
]
HOSTILE_NAMES = [" leading-space", "trailing-space ", "trailing-dot.", "..dots..", "-leading-dash", "--help", "*star*",
                 "question?", "colon:name", "pipe|name", "lt<gt>", 'quote"double', "single'quote", "back\\slash",
                 "tab\tname", "newline\nname", "cr\rname", "CON", "nul.txt", "Aux", "COM1.log", "LPT9", "con.",
                 "PRN .txt", "~lock", "#autosave#", ".hidden", "...", "$dollar", "%percent%", "semi;colon",
                 "amp&ersand", "[brackets]", "{braces}", "percent%2Fslash", "esc\x1b[31mred", "bell\x07",
                 "del\x7fchar", "ctl\x01\x02\x03"]
NONUTF8_NAMES = [b"latin1-caf\xe9", b"invalid-\xff\xfe", b"overlong-\xc0\xaf", b"surrogate-\xed\xa0\x80",
                 b"truncated-\xe6\x97", b"cp1252-\x93quoted\x94", b"shiftjis-\x93\xfa\x96\x7b", b"lone-cont-\x80\x81"]
CASE_SETS = [["Makefile", "makefile", "MAKEFILE"], ["a.TXT", "a.txt", "A.txt"], ["straße", "STRASSE", "strasse"],
             ["ǅ", "Ǆ", "ǆ"], ["ı", "I", "i", "İ"]]


def profile_zoo(t: Tree, rng: random.Random, scale: float):
    # permission matrix
    for m in range(0o10000):
        t.file(f"modes/files/m{m:04o}", b"%04o\n" % m, mode=m)
    for m in (0o000, 0o100, 0o111, 0o222, 0o300, 0o444, 0o500, 0o555, 0o700, 0o711, 0o750, 0o755, 0o775, 0o777,
              0o1777, 0o1755, 0o2755, 0o2775, 0o2770, 0o3777, 0o4755, 0o6755, 0o7777, 0o1000):
        d = t.dir(f"modes/dirs/d{m:04o}", mode=m)
        t.file(f"modes/dirs/d{m:04o}/inside", b"x")
    # owners
    for i, uid in enumerate(UIDS):
        for j, gid in enumerate(UIDS[:: 3]):
            t.file(f"owners/u{uid}-g{gid}", b"", mode=0o640, owner=(uid, gid))
        t.dir(f"owners/dir-u{uid}", owner=(uid, UIDS[-1 - i]))
    # names
    for n in UNICODE_NAMES:
        t.file("names/unicode/" + n, n.encode("utf-8"))
    for n in HOSTILE_NAMES:
        t.file("names/hostile/" + n, b"hostile\n")
    for n in NONUTF8_NAMES:
        t.file(b"names/non-utf8/" + n, n)
    for k, grp in enumerate(CASE_SETS):
        for n in grp:
            t.file(f"names/case-{k}/{n}", n.encode())
    t.file("names/long/" + "a" * 255, b"255 ascii")
    t.file("names/long/" + "語" * 85, b"255 bytes of 3-byte utf-8")
    t.file("names/long/" + "😀" * 63 + "ab", b"254 bytes of 4-byte utf-8")
    t.dir("names/long/" + "d" * 255)
    t.file("names/long/" + "d" * 255 + "/" + "f" * 255, b"510-byte path component pair")
    # xattrs
    for i in range(60):
        path = t.file(f"xattrs/file-{i:02d}", bytes([i]) * rng.randrange(0, 64))
        for k in range(rng.randrange(1, 8)):
            size = rng.choice((0, 1, 16, 100, 255, 256, 1000))
            t.xattr(path, f"user.ebrc.k{k}", bytes(rng.randrange(256) for _ in range(size)))
    p = t.file("xattrs/many-small-names", b"many")
    for k in range(80):
        t.xattr(p, f"user.n{k:02d}", b"v%d" % k)
    t.xattr(t.file("xattrs/one-3500-byte-value", b"big"), "user.big", bytes(rng.randrange(256) for _ in range(3500)))
    t.xattr(t.file("xattrs/empty-value", b""), "user.empty", b"")
    t.xattr(t.file("xattrs/nul-bytes", b"nul"), "user.nul", b"\0a\0b\0")
    t.xattr(t.file("xattrs/utf8-name", b"u"), "user.ünïcödé.名前", "värde".encode())
    t.xattr(t.dir("xattrs/on-dir"), "user.dir.purpose", b"xattr on a directory")
    t.xattr(t.file("xattrs/trusted", b"t"), "trusted.overlay.opaque", b"y")
    lnk = t.symlink("xattrs/symlink-with-trusted", "trusted")
    t.xattr(lnk, "trusted.ebrc.on-symlink", b"symlinks cannot carry user.* xattrs")
    t.xattr(t.file("xattrs/capability-net-raw", b"\x7fELF", mode=0o755), "security.capability", fscap(1 << 13))
    t.xattr(t.file("xattrs/capability-multi", b"\x7fELF", mode=0o750), "security.capability",
            fscap((1 << 10) | (1 << 12) | (1 << 21) | (1 << 38)))
    t.xattr(t.file("xattrs/selinux-label", b"s"), "security.selinux", b"system_u:object_r:etc_t:s0\0")
    # ACLs
    for i in range(40):
        path = t.file(f"acls/file-{i:02d}", b"acl", mode=0o640)
        users = tuple((uid, rng.randrange(8)) for uid in sorted(rng.sample(UIDS[1:], rng.randrange(0, 4))))
        groups = tuple((gid, rng.randrange(8)) for gid in sorted(rng.sample(UIDS[1:], rng.randrange(0, 3))))
        t.acl_access(path, acl(rng.randrange(8), rng.randrange(8), rng.randrange(8), users, groups,
                               rng.randrange(8) if users or groups else None))
    d = t.dir("acls/default-acl-dir", mode=0o2770)
    t.file("acls/default-acl-dir/existing-before", b"created before the default ACL")
    t.acl_default(d, acl(7, 5, 0, ((1000, 7), (1001, 5)), ((100, 5),), 7))
    t.acl_access(d, acl(7, 7, 0, ((1000, 7),), ((100, 5),), 7))
    t.acl_default(t.dir("acls/default-only-base"), acl(6, 4, 4))
    # hardlinks
    for g in range(60):
        src = f"hardlinks/groups/g{g:02d}-a"
        t.file(src, b"hardlink group %d\n" % g * rng.randrange(1, 50), mode=rng.choice((0o644, 0o755, 0o4755)))
        for k in range(rng.randrange(1, 10)):
            t.link(src, f"hardlinks/groups/{rng.choice(('x', 'y', 'z'))}/g{g:02d}-link{k}")
    t.file("hardlinks/farm/original", b"one inode, 1000 names\n")
    for k in range(int(999 * scale)):
        t.link("hardlinks/farm/original", f"hardlinks/farm/n{k:04d}")
    xl = t.file("hardlinks/with-xattr-and-acl", b"shared metadata")
    t.xattr(xl, "user.shared", b"all links see this")
    t.acl_access(xl, acl(6, 4, 0, ((1000, 6),), (), 6))
    t.link("hardlinks/with-xattr-and-acl", "hardlinks/elsewhere/deep/er/alias")
    # symlinks
    t.file("symlinks/target.txt", b"target\n")
    t.dir("symlinks/target-dir")
    for name, target in [("rel", "target.txt"), ("rel-up", "../symlinks/target.txt"), ("abs", "/etc/passwd"),
                         ("abs-missing", "/nonexistent/deep/path"), ("dangling", "missing"), ("loop-a", "loop-b"),
                         ("loop-b", "loop-a"), ("self", "self"), ("dir", "target-dir"), ("dir-slash", "target-dir/"),
                         ("dot", "."), ("dotdot", ".."), ("escape", "../../../../../../../../outside-the-item"),
                         ("unicode-target", "日本語/😀"), ("space target", " spaced target ")]:
        t.symlink(f"symlinks/{name}", target)
    t.symlink("symlinks/non-utf8-target", b"caf\xe9/\xff")
    t.symlink("symlinks/long-target", "x" * 4095)
    for k in range(40):
        t.symlink(f"symlinks/chain/l{k:02d}", f"l{k + 1:02d}" if k < 39 else "../target.txt")
    t.symlink("symlinks/owned-link", "target.txt", owner=(1000, 1000))
    # special files
    t.fifo("special/fifo", 0o620)
    t.fifo("special/fifo-world", 0o666)
    t.device("special/null-like", False, 1, 3, 0o666)
    t.device("special/tty-like", False, 4, 64, 0o620)
    t.device("special/loop0-like", True, 7, 0, 0o660)
    t.device("special/sda1-like", True, 8, 1, 0o660)
    t.sock("special/control.sock")
    # empties and nesting
    for i in range(int(1500 * scale)):
        t.file(f"empty/files/e{i:04d}", b"")
    for i in range(int(40 * scale)):
        for j in range(20):
            t.dir(f"empty/dirs/a{i:02d}/b{j:02d}")
    chain = "nest/" + "/".join(f"level{k:03d}" for k in range(120))
    t.dir(chain)
    t.file(chain + "/bottom.txt", b"120 levels down\n")
    # timestamps
    for name, sec, ns in [("t-1901", EXT4_MIN_T, 0), ("t-minus-1", -1, 999999999), ("t-epoch", 0, 0),
                          ("t-one-ns", 0, 1), ("t-2038-max32", 2**31 - 1, 999999999), ("t-2038-plus1", 2**31, 0),
                          ("t-2106", 2**32, 123456789), ("t-2446-max", EXT4_MAX_T, 999999999),
                          ("t-source-date-epoch", EPOCH, 0), ("t-negative-ns", -86400 * 365 * 30, 500000000)]:
        t.mtime(t.file(f"times/{name}", name.encode()), sec, ns)
    t.mtime(t.dir("times/dir-in-1970"), 3600)


SCRIPTS = ["देवनागरी", "ภาษาไทย", "ქართული", "ᄒᆞᆫ글", "بِسْم", "‫embedded-rtl‬",
           "Ꭰꮳꮃ", "ⲁⲛⲟⲕ", "𐌰𐌱𐌲", "ምሳሌ", "ཀཁག", "ᚠᚢᚦ", "Ⓐⓑⓒ", "ǅungla"]


def profile_deep(t: Tree, rng: random.Random, scale: float):
    depth = int(1000 * min(1.0, scale))
    chain = "/".join("d" for _ in range(depth))
    t.dir("chain1/" + chain)
    t.file("chain1/" + chain + "/deepest", b"1000 levels\n")
    t.file("chain1/shallow", b"shallow\n")
    t.link("chain1/" + chain + "/deepest", "chain1/hardlink-to-deepest")
    t.symlink("chain1/" + chain + "/up-to-top", "/".join(".." for _ in range(depth)) + "/shallow")
    name200 = lambda k: (f"{k:02d}-" + "long-component-" * 13)[:200]  # noqa: E731
    long_chain = "/".join(name200(k) for k in range(18))
    t.dir("chain2/" + long_chain)
    t.file("chain2/" + long_chain + "/end", b"close to PATH_MAX\n")
    for i in range(int(20000 * scale)):
        t.file(f"wide/{SCRIPTS[i % len(SCRIPTS)]}-{i:05d}", b"")
    top = t.dir("inherit", mode=0o2775)
    t.acl_default(top, acl(7, 7, 5, ((1000, 7), (2000, 5)), ((3000, 7),), 7))
    # default ACLs must exist before children are created for inheritance: apply immediately
    os.chmod(top, 0o2775)
    os.setxattr(top, "system.posix_acl_default", acl(7, 7, 5, ((1000, 7), (2000, 5)), ((3000, 7),), 7))
    t.meta = [m for m in t.meta if not (m[0] == top and m[1] == "xattr")]
    t.count["acls"] += 1
    for i in range(int(400 * scale)):
        d = "inherit/" + "/".join(f"s{rng.randrange(6)}" for _ in range(rng.randrange(1, 6)))
        path = t.p(d)
        mkdirs(path)
        with open(os.path.join(path, f"f{i}".encode()), "ab") as f:
            f.write(b"inherited acl\n")
    for i in range(300):
        d = t.dir(f"dirxattrs/{SCRIPTS[i % len(SCRIPTS)]}/{i:03d}")
        t.xattr(d, "user.index", str(i).encode())
        if i % 3 == 0:
            t.xattr(d, "user.script", SCRIPTS[i % len(SCRIPTS)].encode())
    for i in range(200):
        t.symlink(f"links/up{i:03d}", "../" * (i + 1) + "target")
    t.file("links/target", b"")
    for i, s in enumerate(SCRIPTS):
        t.file(f"scripts/{s}/{unicodedata.normalize('NFD', s)}", s.encode())
        t.file(f"scripts/{s}/{unicodedata.normalize('NFKC', s)}-nfkc", s.encode())
    t.file("homoglyphs/paypal", b"latin")
    t.file("homoglyphs/pаypаl", b"cyrillic a")
    t.file("homoglyphs/рaypal", b"cyrillic er")
    for i, uid in enumerate(UIDS):
        t.file(f"owners/{i:02d}", b"", owner=(uid, UIDS[(i * 5) % len(UIDS)]), mode=0o600 + i)


def random_unicode_name(rng, n):
    out = []
    while len(out) < n:
        plane = rng.choice((0x0000, 0x0000, 0x0000, 0x10000, 0x20000))
        cp = plane + rng.randrange(0x20, 0xFFFE)
        if 0xD800 <= cp <= 0xDFFF or cp == 0x2F or (cp & 0xFFFE) == 0xFFFE or 0xFDD0 <= cp <= 0xFDEF:
            continue
        out.append(chr(cp))
    s = "".join(out)
    while len(s.encode("utf-8")) > 240:
        s = s[:-1]
    return s


def profile_matrix(t: Tree, rng: random.Random, scale: float):
    modes = [0o400, 0o440, 0o444, 0o600, 0o640, 0o644, 0o660, 0o664, 0o666, 0o700, 0o750, 0o755, 0o770, 0o775,
             0o777, 0o4755, 0o2755, 0o6755, 0o1777, 0o4711, 0o2711, 0o000, 0o010, 0o001, 0o4000, 0o2000, 0o1000,
             0o7000, 0o4644, 0o2644, 0o0204, 0o0402]
    acl_variants = [None, ("u",), ("g",), ("ug",), ("mask0",), ("wide",)]
    uids = [0, 7, 1000, 1500, 60000, 65534, 2**31 - 7, 4000000000]
    gids = [0, 5, 100, 1000, 65534, 3000000000]
    n = 0
    for uid in uids:
        for gid in gids:
            for m in modes:
                v = acl_variants[n % len(acl_variants)]
                path = t.file(f"matrix/u{uid}/g{gid}/m{m:04o}-{'none' if v is None else v[0]}", b"", mode=m,
                              owner=(uid, gid))
                if v == ("u",):
                    t.acl_access(path, acl(6, 4, 0, ((uid + 1, 6),), (), 6))
                elif v == ("g",):
                    t.acl_access(path, acl(6, 4, 0, (), ((gid + 1, 4),), 4))
                elif v == ("ug",):
                    t.acl_access(path, acl(7, 5, 0, ((1, 7), (uid + 2, 5)), ((gid + 3, 1),), 5))
                elif v == ("mask0",):
                    t.acl_access(path, acl(7, 7, 7, ((42, 7),), ((43, 7),), 0))
                elif v == ("wide",):
                    t.acl_access(path, acl(7, 5, 1, tuple((10000 + k, k % 8) for k in range(8)),
                                           tuple((20000 + k, (k * 3) % 8) for k in range(8)), 7))
                n += 1
        t.acl_default(t.dir(f"matrix/u{uid}", owner=(uid, 0)), acl(7, 5, 5, ((uid + 9, 7),), (), 7))
    ladder = [0, 1, 2, 63, 64, 127, 128, 255, 256, 511, 512, 1023, 1024, 2047, 2048, 3000, 3700]
    for size in ladder:
        t.xattr(t.file(f"xattr-ladder/v{size:04d}", b""), "user.v", bytes(rng.randrange(256) for _ in range(size)))
    t.xattr(t.file("xattr-ladder/name-255", b""), "user." + "n" * 250, b"max-length name")
    path = t.file("xattr-ladder/mixed-namespaces", b"")
    t.xattr(path, "user.a", b"1")
    t.xattr(path, "trusted.b", b"2")
    t.xattr(path, "security.c", b"3")
    for i in range(int(3000 * scale)):
        t.file(b"random-names/" + random_unicode_name(rng, rng.randrange(1, 40)).encode("utf-8") + b".%d" % i, b"")
    for i in range(int(300 * scale)):
        raw = bytes(rng.randrange(1, 256) for _ in range(rng.randrange(1, 60))).replace(b"/", b"_")
        t.file(b"random-bytes/" + raw + b"-%d" % i, raw)
    for i in range(int(300 * scale)):
        cmd = f"tool{i:03d}"
        t.file(f"alt/usr/lib/{cmd}-impl{i % 3}", b"#!/bin/sh\n", mode=0o755)
        t.symlink(f"alt/etc/alternatives/{cmd}", f"/usr/lib/{cmd}-impl{i % 3}")
        t.symlink(f"alt/usr/bin/{cmd}", f"/etc/alternatives/{cmd}")
    for i in range(int(2000 * scale)):
        name = f"prog{i:04d}"
        t.file(f"farm/usr/bin/{name}", b"\x7fELF" + bytes(rng.randrange(256) for _ in range(rng.randrange(16, 400))),
               mode=rng.choice((0o755, 0o755, 0o4755, 0o2755, 0o555)))
        t.link(f"farm/usr/bin/{name}", f"farm/bin/{name}")
        t.link(f"farm/usr/bin/{name}", f"farm/sbin/{name}")
    for i in range(int(2000 * scale)):
        p = t.file(f"times/r{i:04d}", b"")
        t.mtime(p, rng.randrange(EXT4_MIN_T, EXT4_MAX_T), rng.randrange(10**9))
    for i in range(64):
        t.device(f"dev/c{i:02d}", False, rng.randrange(1, 4095), rng.randrange(0, 1 << 20), rng.choice((0o600, 0o660, 0o666)))
        t.device(f"dev/b{i:02d}", True, rng.randrange(1, 4095), rng.randrange(0, 1 << 20), 0o660)
    for i in range(16):
        t.fifo(f"dev/fifo{i:02d}", rng.choice((0o600, 0o622, 0o666)))


PROFILES = {"zoo": profile_zoo, "deep": profile_deep, "matrix": profile_matrix}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True)
    ap.add_argument("--seed", type=int, required=True)
    ap.add_argument("--params", default="{}")
    args = ap.parse_args()
    p = json.loads(args.params)
    if os.geteuid() != 0:
        raise SystemExit("metadata_tree: must run as root (chown, mknod, trusted.* xattrs)")
    prof = p.get("profile")
    if prof not in PROFILES:
        raise SystemExit(f"metadata_tree: params.profile must be one of {sorted(PROFILES)}")
    rng = random.Random(args.seed)
    t = Tree(args.out, rng)
    PROFILES[prof](t, rng, float(p.get("scale", 1.0)))
    t.apply()
    print(json.dumps(t.count, sort_keys=True), file=sys.stderr)


if __name__ == "__main__":
    main()
