#!/usr/bin/env python3
"""Generated hardlink-farm trees for family F19 (corpus group g2-generated).  GENERATED DATA.

Models (params.model):
  rsnapshot  Rotating backup snapshots made the way `rsync --link-dest` / rsnapshot make them: the
             oldest snapshot is a full copy of a synthetic home/project dataset (varied owners and
             modes); every newer snapshot hardlinks unchanged files to the previous snapshot, writes
             changed files as new inodes, adds and deletes files, renames the occasional directory
             (files stay hardlinked under the new path) and breaks links on metadata-only changes
             (same bytes, new inode, different mode/owner).  params: base_files, snapshots,
             change_fraction.
  linkstore  (held-out construction) Nix-store-like tree after `nix-store --optimise`: read-only
             package directories named <base32 hash>-<name>-<version> (dirs 0555, files 0444/0555,
             mtime 1), several versions per package sharing most files, identical files replaced by
             hardlinks into /nix/store/.links/<base32 sha256>, .drv text files, and profile/gcroot
             symlink chains.  params: packages, versions_max.

Content bytes come from SHAKE-256 counter mode (random assets) and a seeded word model (text).
"""

import argparse
import hashlib
import json
import math
import os
import random
import struct
import sys

EPOCH = int(os.environ.get("SOURCE_DATE_EPOCH", "1767225600"))
MiB = 1 << 20
B32 = "0123456789abcdfghijklmnpqrsvwxyz"  # nix base32 alphabet


class Shake:
    def __init__(self, seed, domain):
        self.prefix = domain + b"\0" + struct.pack("<Q", seed & (2**64 - 1))
        self.counter = 0

    def __call__(self, n):
        out = bytearray()
        while len(out) < n:
            take = min(n - len(out), 4 * MiB)
            out += hashlib.shake_256(self.prefix + struct.pack("<Q", self.counter)).digest(take)
            self.counter += 1
        return bytes(out)


SYLL = "ka ri to mo na lu be si ve ro da ne xi pa qu fe zo li ma te ho gu ja ce wi yo ba de fi ga hi ko".split()


class Words:
    def __init__(self, rng):
        self.rng = rng
        self.w = sorted({"".join(rng.choice(SYLL) for _ in range(rng.choice((2, 3, 3, 4)))) for _ in range(5000)})
        self.lines = [(" ".join(rng.choice(self.w) for _ in range(rng.randrange(3, 16))) + "\n").encode()
                      for _ in range(8192)]

    def text(self, n):
        out = bytearray()
        r = self.rng.randrange
        while len(out) < n:
            out += self.lines[r(8192)]
        return bytes(out[:n])


def b32(digest, n):
    v = int.from_bytes(digest, "little")
    return "".join(B32[(v >> (5 * i)) & 31] for i in range(n))


def lognorm(rng, mu, sigma, lo, hi):
    return max(lo, min(hi, int(math.exp(mu + sigma * rng.gauss(0, 1)))))


def write(path, data, mode, owner=None, mtime_ns=None):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "wb") as f:
        f.write(data)
    if owner:
        os.chown(path, *owner)
    os.chmod(path, mode)
    if mtime_ns is not None:
        os.utime(path, ns=(mtime_ns, mtime_ns))


# ---------------------------------------------------------------------------------------------

def model_rsnapshot(out, rng, rnd, words, p):
    n_files = int(p.get("base_files", 2500))
    snaps = int(p.get("snapshots", 10))
    change = float(p.get("change_fraction", 0.03))
    owners = [(1000, 1000), (1000, 1000), (1000, 100), (1001, 1001), (0, 0), (33, 33)]
    dirs = ["home/alice/projects", "home/alice/Documents", "home/alice/.config", "home/bob/mail", "home/bob/photos",
            "srv/www/site", "srv/www/site/assets", "etc/app", "var/log/app", "home/alice/projects/src",
            "home/alice/projects/src/core", "home/alice/projects/tests", "home/bob/code/tool", "srv/db-dumps"]
    state = {}  # rel -> dict(content=bytes|None(key), mode, owner, mtime, gen)

    def new_content(rel):
        if rel.endswith((".jpg", ".png", ".gz", ".bin")):
            return rnd(lognorm(rng, 10.5, 1.4, 256, 8 * MiB))
        return words.text(lognorm(rng, 8.8, 1.3, 1, 2 * MiB))

    for i in range(n_files):
        d = rng.choice(dirs)
        ext = rng.choice((".txt", ".md", ".py", ".c", ".log", ".conf", ".jpg", ".png", ".gz", ".bin", ".json"))
        rel = f"{d}/{rng.choice(words.w)}-{i}{ext}"
        state[rel] = {"data": new_content(rel), "mode": rng.choice((0o644, 0o644, 0o600, 0o640, 0o755, 0o444)),
                      "owner": rng.choice(owners), "mtime": (EPOCH - 86400 * 400 - rng.randrange(86400 * 1000)) * 10**9}
    names = [f"weekly.{k}" for k in range(2, -1, -1)] + [f"daily.{k}" for k in range(snaps - 4, -1, -1)]
    names = names[-snaps:] if len(names) > snaps else names
    prev = None
    inode_of = {}  # rel -> path in previous snapshot (for linking)
    stats = {"snapshots": len(names), "links": 0, "new_inodes": 0, "broken_links_metadata": 0}
    for si, snap in enumerate(names):
        t_snap = EPOCH - 86400 * (len(names) - si) * 3
        root = os.path.join(out, "backups", snap)
        changed = set()
        meta_changed = set()
        if si > 0:
            rels = sorted(state)
            for rel in rng.sample(rels, int(len(rels) * change)):
                if rel.endswith(".log"):
                    state[rel]["data"] = state[rel]["data"] + words.text(rng.randrange(100, 20000))
                else:
                    state[rel]["data"] = new_content(rel)
                state[rel]["mtime"] = (t_snap - rng.randrange(86400)) * 10**9
                changed.add(rel)
            for rel in rng.sample(rels, int(len(rels) * change / 6)):
                if rel not in changed:
                    state[rel]["mode"] = rng.choice((0o600, 0o640, 0o664, 0o755))
                    meta_changed.add(rel)
            for rel in rng.sample(rels, int(len(rels) * change / 3)):
                state.pop(rel, None)
                changed.discard(rel)
            for k in range(int(n_files * change / 3)):
                d = rng.choice(dirs)
                rel = f"{d}/{rng.choice(words.w)}-s{si}-{k}.txt"
                state[rel] = {"data": new_content(rel), "mode": 0o644, "owner": rng.choice(owners),
                              "mtime": (t_snap - rng.randrange(86400)) * 10**9}
                changed.add(rel)
            if rng.random() < 0.4:
                old_dir = rng.choice(dirs[:9])
                new_dir = old_dir + "-renamed-" + str(si)
                for rel in [r for r in sorted(state) if r.startswith(old_dir + "/") and r.count("/") == old_dir.count("/") + 1]:
                    nrel = new_dir + rel[len(old_dir):]
                    state[nrel] = state.pop(rel)
                    if rel in inode_of:
                        inode_of[nrel] = inode_of.pop(rel)
                    if rel in changed:
                        changed.discard(rel)
                        changed.add(nrel)
                    if rel in meta_changed:
                        meta_changed.discard(rel)
                        meta_changed.add(nrel)
                dirs[dirs.index(old_dir)] = new_dir
        new_inode_of = {}
        for rel in sorted(state):
            path = os.path.join(root, rel)
            os.makedirs(os.path.dirname(path), exist_ok=True)
            st = state[rel]
            if prev is not None and rel in inode_of and rel not in changed and rel not in meta_changed:
                os.link(inode_of[rel], path)
                stats["links"] += 1
            else:
                write(path, st["data"], st["mode"], st["owner"], st["mtime"])
                stats["new_inodes"] += 1
                if rel in meta_changed:
                    stats["broken_links_metadata"] += 1
            new_inode_of[rel] = path
        inode_of = new_inode_of
        prev = root
        dpaths = []
        for r, dn, fn in os.walk(root):
            for n in dn:
                dpaths.append(os.path.join(r, n))
        for dp in sorted(dpaths, key=lambda s: (-s.count("/"), s)):
            os.utime(dp, ns=(t_snap * 10**9,) * 2)
        os.utime(root, ns=(t_snap * 10**9,) * 2)
    with open(os.path.join(out, "backups", "rsnapshot.conf"), "w") as f:
        f.write("config_version\t1.2\nsnapshot_root\t/backups/\nretain\tdaily\t7\nretain\tweekly\t3\n"
                "link_dest\t1\nbackup\t/home/\tlocalhost/\nbackup\t/srv/\tlocalhost/\n")
    os.utime(os.path.join(out, "backups", "rsnapshot.conf"), ns=(EPOCH * 10**9,) * 2)
    os.utime(os.path.join(out, "backups"), ns=(EPOCH * 10**9,) * 2)
    return stats


# ---------------------------------------------------------------------------------------------

def model_linkstore(out, rng, rnd, words, p):
    n_pk = int(p.get("packages", 70))
    vmax = int(p.get("versions_max", 6))
    store = os.path.join(out, "nix", "store")
    links = os.path.join(store, ".links")
    os.makedirs(links)
    link_by_hash = {}
    stats = {"packages": 0, "files": 0, "linked": 0, "unique": 0}
    licenses = [words.text(lognorm(rng, 8.5, 0.5, 800, 40000)) for _ in range(8)]
    one = 10**9  # nix store mtime: 1 second after the epoch

    def add_file(pkgdir, rel, data, exe):
        path = os.path.join(pkgdir, rel)
        os.makedirs(os.path.dirname(path), exist_ok=True)
        h = hashlib.sha256(data + (b"x" if exe else b"-")).digest()
        key = b32(h, 52)
        stats["files"] += 1
        if key in link_by_hash:
            os.link(link_by_hash[key], path)
            stats["linked"] += 1
            return
        write(path, data, 0o555 if exe else 0o444, (0, 0), one)
        lp = os.path.join(links, key)
        os.link(path, lp)
        link_by_hash[key] = lp
        stats["unique"] += 1

    drvs = []
    for pk in range(n_pk):
        name = rng.choice(words.w) + ("-" + rng.choice(words.w) if rng.random() < 0.3 else "")
        base_files = {}
        nfiles = lognorm(rng, 3.6, 0.9, 3, 400)
        for k in range(nfiles):
            kind = rng.random()
            if kind < 0.15:
                rel, data, exe = f"bin/{name}{'-' + str(k) if k else ''}", b"\x7fELF\x02\x01\x01" + rnd(lognorm(rng, 11, 1.2, 2000, 6 * MiB)), True
            elif kind < 0.35:
                rel, data, exe = f"lib/lib{name}{k}.so.{rng.randrange(1, 9)}", b"\x7fELF" + rnd(lognorm(rng, 11.5, 1.3, 4000, 8 * MiB)), True
            elif kind < 0.55:
                rel, data, exe = f"share/man/man{rng.choice((1, 3, 5, 8))}/{name}-{k}.gz", rnd(lognorm(rng, 7.5, 0.8, 200, 60000)), False
            elif kind < 0.75:
                rel, data, exe = f"share/doc/{name}/{rng.choice(words.w)}-{k}.md", words.text(lognorm(rng, 8, 1.1, 100, 300000)), False
            elif kind < 0.9:
                rel, data, exe = f"share/locale/{rng.choice(('de', 'fr', 'ja', 'pt_BR', 'zh_CN'))}/LC_MESSAGES/{name}-{k}.mo", rnd(lognorm(rng, 9, 0.8, 500, 200000)), False
            else:
                rel, data, exe = f"include/{name}/{rng.choice(words.w)}-{k}.h", words.text(lognorm(rng, 8.3, 1.0, 100, 100000)), False
            base_files[rel] = (data, exe)
        base_files[f"share/licenses/{name}/COPYING"] = (licenses[rng.randrange(len(licenses))], False)
        versions = rng.randrange(1, vmax + 1)
        major, minor = rng.randrange(0, 6), rng.randrange(0, 20)
        prev_store = None
        for v in range(versions):
            ver = f"{major}.{minor + v}.{rng.randrange(10)}"
            if v:
                for rel in sorted(base_files):
                    if rng.random() < rng.uniform(0.05, 0.3):
                        data, exe = base_files[rel]
                        if rel.startswith("share/doc"):
                            data = data + words.text(rng.randrange(50, 3000))
                        else:
                            data = data[: len(data) // 2] + rnd(len(data) - len(data) // 2)
                        base_files[rel] = (data, exe)
            digest = hashlib.sha256(f"{name}-{ver}-{pk}".encode() + rnd(8)).digest()
            pkgdir = os.path.join(store, f"{b32(digest, 32)}-{name}-{ver}")
            for rel in sorted(base_files):
                add_file(pkgdir, rel, *base_files[rel])
            if prev_store:
                os.symlink(prev_store, os.path.join(pkgdir, "share", "doc", name, "previous-version")) \
                    if os.path.isdir(os.path.join(pkgdir, "share", "doc", name)) else None
            prev_store = "/nix/store/" + os.path.basename(pkgdir)
            drv = f"Derive([(\"out\",\"{prev_store}\",\"\",\"\")],[],[],\"x86_64-linux\",\"/bin/sh\",[\"-e\",\"builder.sh\"]," \
                  f"[(\"name\",\"{name}-{ver}\"),(\"version\",\"{ver}\")])"
            dpath = os.path.join(store, f"{b32(hashlib.sha256(drv.encode()).digest(), 32)}-{name}-{ver}.drv")
            write(dpath, drv.encode(), 0o444, (0, 0), one)
            drvs.append(dpath)
            stats["packages"] += 1
    # profiles and gcroots
    pk_dirs = sorted(d for d in os.listdir(store) if not d.startswith(".") and not d.endswith(".drv"))
    prof = os.path.join(out, "nix", "var", "nix", "profiles")
    os.makedirs(os.path.join(prof, "per-user", "root"))
    for g in range(1, 13):
        env = os.path.join(store, f"{b32(hashlib.sha256(b'env%d' % g).digest(), 32)}-user-environment")
        os.makedirs(os.path.join(env, "bin"))
        for d in rng.sample(pk_dirs, min(12, len(pk_dirs))):
            bindir = os.path.join(store, d, "bin")
            if os.path.isdir(bindir):
                for b in sorted(os.listdir(bindir)):
                    lp = os.path.join(env, "bin", b)
                    if not os.path.lexists(lp):
                        os.symlink(f"/nix/store/{d}/bin/{b}", lp)
        os.symlink(f"/nix/store/{os.path.basename(env)}", os.path.join(prof, "per-user", "root", f"profile-{g}-link"))
    os.symlink(f"profile-12-link", os.path.join(prof, "per-user", "root", "profile"))
    os.symlink("/nix/var/nix/profiles/per-user/root/profile", os.path.join(prof, "default"))
    gc = os.path.join(out, "nix", "var", "nix", "gcroots", "auto")
    os.makedirs(gc)
    for i, d in enumerate(rng.sample(pk_dirs, min(40, len(pk_dirs)))):
        os.symlink(f"/nix/store/{d}", os.path.join(gc, b32(hashlib.sha256(d.encode()).digest(), 32)))
    # read-only store directories, mtime 1 (children first)
    all_dirs = []
    for r, dn, fn in os.walk(store):
        for n in dn:
            all_dirs.append(os.path.join(r, n))
    for dp in sorted(all_dirs, key=lambda s: (-s.count("/"), s)):
        if os.path.basename(dp) != ".links":
            os.chmod(dp, 0o555)
        os.utime(dp, ns=(one, one))
    for r, dn, fn in os.walk(out):
        for n in fn + dn:
            fp = os.path.join(r, n)
            if os.path.islink(fp):
                os.utime(fp, ns=(one, one), follow_symlinks=False)
    for dp in (os.path.join(out, "nix", "var", "nix", "gcroots", "auto"), os.path.join(out, "nix", "var", "nix", "gcroots"),
               os.path.join(prof, "per-user", "root"), os.path.join(prof, "per-user"), prof,
               os.path.join(out, "nix", "var", "nix"), os.path.join(out, "nix", "var"), store, os.path.join(out, "nix")):
        os.utime(dp, ns=(EPOCH * 10**9,) * 2)
    os.chmod(store, 0o1775)
    os.chown(store, 0, 30000)
    return stats


MODELS = {"rsnapshot": model_rsnapshot, "linkstore": model_linkstore}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True)
    ap.add_argument("--seed", type=int, required=True)
    ap.add_argument("--params", default="{}")
    args = ap.parse_args()
    p = json.loads(args.params)
    if os.geteuid() != 0:
        raise SystemExit("hardlink_farm: must run as root (chown)")
    model = p.get("model")
    if model not in MODELS:
        raise SystemExit(f"hardlink_farm: params.model must be one of {sorted(MODELS)}")
    rng = random.Random(args.seed)
    rnd = Shake(args.seed, b"ebrc-g2-f19-" + model.encode())
    words = Words(random.Random(args.seed + 1))
    stats = MODELS[model](args.out, rng, rnd, words, p)
    print(json.dumps(stats, sort_keys=True), file=sys.stderr)


if __name__ == "__main__":
    main()
