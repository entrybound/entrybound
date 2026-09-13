#!/usr/bin/env python3
"""Pack a corpus item into a sparse ext4 filesystem image (family F15, corpus group g2-generated).
DERIVED-FROM-REAL when the source item is real.

Used by `derive` items: the single from_items dependency (EB_FROM_JSON) is packed with the real
e2fsprogs tools into a raw image file created with ftruncate, so the image is sparse exactly where
mke2fs/e2fsprogs leave blocks unwritten:

  truncate -> mke2fs -t ext4 -d <source> (fixed UUID, hash_seed, E2FSPROGS_FAKE_TIME,
              lazy_itable_init=0, lazy_journal_init=0, nodiscard, root_owner=0:0)
           -> debugfs -w: set atime/ctime of every inode to SOURCE_DATE_EPOCH
              (mke2fs -d copies the source's atime/ctime, which are not reproducible)
           -> e2fsck -fn must report a clean filesystem

Reproducibility was checked with e2fsprogs 1.47.0: identical image bytes across runs and across
source trees on ext4 and tmpfs.  Paths containing '"', '\\' or newlines are refused (debugfs quoting).

params: name (image file name), image_mib (int), block_size (int, default 4096), inode_size (int,
        default 256), journal (bool, default true), label (str)
The UUID and hash seed are derived from --seed.
"""

import argparse
import hashlib
import json
import os
import subprocess
import sys
import uuid

EPOCH = int(os.environ.get("SOURCE_DATE_EPOCH", "1767225600"))


def run(cmd, **kw):
    r = subprocess.run(cmd, capture_output=True, **kw)
    if r.returncode != 0:
        sys.stderr.write(r.stdout.decode(errors="replace") + r.stderr.decode(errors="replace"))
        raise SystemExit(f"ext4_image: {cmd[0]} failed with exit {r.returncode}")
    return r


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True)
    ap.add_argument("--seed", type=int, required=True)
    ap.add_argument("--params", default="{}")
    args = ap.parse_args()
    p = json.loads(args.params)
    deps = json.loads(os.environ.get("EB_FROM_JSON", "{}"))
    if len(deps) != 1:
        raise SystemExit("ext4_image: exactly one from_items dependency is required")
    src = next(iter(deps.values()))
    name = p.get("name", "rootfs.ext4")
    size = int(p["image_mib"]) << 20
    bs = int(p.get("block_size", 4096))
    isz = int(p.get("inode_size", 256))
    label = str(p.get("label", "ebrc"))[:16]
    h = hashlib.sha256(b"ebrc-g2-f15-ext4|" + str(args.seed).encode()).digest()
    fs_uuid = str(uuid.UUID(bytes=h[:16]))
    hash_seed = str(uuid.UUID(bytes=h[16:32]))

    paths = []
    for root, dnames, fnames in os.walk(src):
        dnames.sort()
        for n in sorted(fnames + dnames):
            rel = os.path.relpath(os.path.join(root, n), src)
            if any(c in rel for c in ('"', "\\", "\n", "\r")):
                raise SystemExit(f"ext4_image: path not representable in debugfs commands: {rel!r}")
            paths.append("/" + rel)
    paths.sort()

    img = os.path.join(args.out, name)
    with open(img, "wb") as f:
        f.truncate(size)
    env = dict(os.environ, E2FSPROGS_FAKE_TIME=str(EPOCH), MKE2FS_CONFIG="/etc/mke2fs.conf")
    ext_opts = f"hash_seed={hash_seed},root_owner=0:0,lazy_itable_init=0,lazy_journal_init=0,nodiscard"
    cmd = ["mke2fs", "-q", "-F", "-t", "ext4", "-b", str(bs), "-I", str(isz), "-L", label, "-U", fs_uuid,
           "-E", ext_opts]
    if not p.get("journal", True):
        cmd += ["-O", "^has_journal"]
    cmd += ["-d", src, img]
    run(cmd, env=env)
    cmds = []
    for pth in ["/", "/lost+found"] + paths:
        cmds.append(f'sif "{pth}" atime {EPOCH}')
        cmds.append(f'sif "{pth}" ctime {EPOCH}')
    cmdfile = os.path.join(os.environ.get("EB_SCRATCH", "/tmp"), "debugfs-times.cmds")
    with open(cmdfile, "w", encoding="utf-8") as f:
        f.write("\n".join(cmds) + "\n")
    r = run(["debugfs", "-w", "-f", cmdfile, img], env=env)
    errs = [ln for ln in r.stderr.decode(errors="replace").splitlines() if ln and not ln.startswith("debugfs ")]
    if errs:
        sys.stderr.write("\n".join(errs[:20]) + "\n")
        raise SystemExit("ext4_image: debugfs reported errors")
    run(["e2fsck", "-fn", img], env=env)
    st = os.stat(img)
    if st.st_blocks * 512 >= st.st_size:
        raise SystemExit("ext4_image: image is not sparse")
    os.chmod(img, 0o644)
    os.utime(img, ns=(EPOCH * 10**9, EPOCH * 10**9))
    print(json.dumps({"image": name, "apparent": st.st_size, "allocated": st.st_blocks * 512, "paths": len(paths),
                      "mke2fs": run(["mke2fs", "-V"], env=env).stderr.decode().splitlines()[0]}), file=sys.stderr)


if __name__ == "__main__":
    main()
