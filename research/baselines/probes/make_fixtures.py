#!/usr/bin/env python3
"""Build the capability-probe fixture tree at <out> (default
/root/eb-research/cache/baselines/fixture), covering every capability
research/baselines/capability-matrix.csv checks by PROBE: non-UTF-8 and
Unicode paths, mtime/atime precision, permissions, ownership, symlinks,
hardlinks, POSIX ACLs, xattrs, and sparse files. Must run as root on ext4
(WSL Ubuntu), per research/PROGRESS.md.

Usage: make_fixtures.py [<out_dir>]
"""
from __future__ import annotations

import os
import shutil
import subprocess
import sys
import time
from pathlib import Path

FIXED_MTIME = 1_700_000_000  # 2023-11-14T22:13:20Z; distinguishable from "now"
FIXED_ATIME = 1_650_000_000  # different from mtime, so the two are distinguishable


def sh(cmd: str) -> subprocess.CompletedProcess:
    return subprocess.run(["bash", "-c", cmd], capture_output=True, text=True)


def build(out: Path, include_nonutf8: bool = True, include_acl: bool = True, include_symlinks: bool = True) -> None:
    if out.exists():
        shutil.rmtree(out)
    out.mkdir(parents=True)

    # 1. Unicode path (multi-script + combining marks + emoji).
    uni_dir = out / "unicode-日本語-\U0001f600"
    uni_dir.mkdir()
    (uni_dir / "café-résumé.txt").write_text("unicode content\n", encoding="utf-8")

    # 2. Non-UTF-8 path: a byte sequence that is not valid UTF-8 (Latin-1 0xE9 'e-acute'
    # alone, invalid as UTF-8 start byte in this position) -- ext4 stores filenames as
    # opaque byte strings, so this is a legal (if unusual) filename on Linux. Some tools
    # (Entrybound's ebound pack) refuse the whole input tree outright when this file is
    # present (EB_EAM_INVALID_PATH_COMPONENT), so include_nonutf8=False builds a fixture
    # usable to probe such a tool's OTHER capabilities.
    if include_nonutf8:
        bad_name = os.fsencode(str(out)) + b"/nonutf8-\xe9\xff-name.bin"
        with open(bad_name, "wb") as fh:
            fh.write(b"non-utf8 filename content\n")

    # 3. Permissions: a range of modes including exec and no-access-to-others.
    perms_dir = out / "perms"
    perms_dir.mkdir()
    for name, mode in [("mode644.txt", 0o644), ("mode600.txt", 0o600), ("mode755.sh", 0o755), ("mode700dir", 0o700)]:
        p = perms_dir / name
        if name.endswith("dir"):
            p.mkdir()
        else:
            p.write_text(f"perm test {name}\n", encoding="utf-8")
        os.chmod(p, mode)

    # 4. Ownership: chown to a non-root uid/gid (root can chown to any id, even
    # nonexistent-in-passwd, since ext4 stores raw numeric uid/gid).
    own_dir = out / "ownership"
    own_dir.mkdir()
    owned = own_dir / "owned-by-1000.txt"
    owned.write_text("ownership test\n", encoding="utf-8")
    os.chown(owned, 1000, 1000)

    # 5. Symlinks: relative + absolute + dangling. include_symlinks=False builds a
    # fixture for a tool whose export path refuses the ENTRY KIND outright (seen
    # with Entrybound's tar export: "the frozen target profile has no native
    # symlink representation" -- refusing the whole conversion transactionally,
    # not just the one entry), so that tool's other capabilities can still be probed.
    if include_symlinks:
        sym_dir = out / "symlinks"
        sym_dir.mkdir()
        (sym_dir / "target.txt").write_text("symlink target\n", encoding="utf-8")
        os.symlink("target.txt", sym_dir / "rel-link.txt")
        os.symlink(str((sym_dir / "target.txt").resolve()), sym_dir / "abs-link.txt")
        os.symlink("does-not-exist.txt", sym_dir / "dangling-link.txt")

    # 6. Hardlinks: two names, one inode.
    hl_dir = out / "hardlinks"
    hl_dir.mkdir()
    primary = hl_dir / "primary.txt"
    primary.write_text("hardlink test\n", encoding="utf-8")
    os.link(primary, hl_dir / "secondary.txt")

    # 7. POSIX ACLs (setfacl grants uid 1234 rwx beyond the owner/group/other bits).
    # Some strict validators (Entrybound's ebound pack, EB_EAM_INVALID_ACL) reject a
    # tree where the ACL mask and the reported group mode bits disagree, which
    # setfacl's own mask recompute can produce; include_acl=False skips this fixture
    # so such a tool's OTHER capabilities can still be probed.
    if include_acl:
        acl_dir = out / "acl"
        acl_dir.mkdir()
        acl_file = acl_dir / "acl-file.txt"
        acl_file.write_text("acl test\n", encoding="utf-8")
        acl_r = sh(f"setfacl -m u:1234:rwx {acl_file}")
    else:
        acl_r = subprocess.CompletedProcess(args=[], returncode=0, stdout="", stderr="")

    # 8. Extended attributes (user.* namespace, portable across POSIX xattr tools).
    xattr_dir = out / "xattr"
    xattr_dir.mkdir()
    xattr_file = xattr_dir / "xattr-file.txt"
    xattr_file.write_text("xattr test\n", encoding="utf-8")
    xattr_r = sh(f"setfattr -n user.ebr_probe -v hello_xattr {xattr_file}")

    # 9. Sparse file: 8 MiB logical, one real 4 KiB block in the middle, rest a hole.
    sparse_dir = out / "sparse"
    sparse_dir.mkdir()
    sparse_file = sparse_dir / "sparse.bin"
    sz = 8 * 1024 * 1024
    with open(sparse_file, "wb") as fh:
        fh.truncate(sz)
    sh(f"dd if=/dev/urandom of={sparse_file} bs=4096 count=1 seek=512 conv=notrunc status=none")
    sparse_r = sh(f"fallocate --dig-holes {sparse_file}")

    # 10. mtime/atime: set to fixed, distinguishable values on every regular file.
    for root, dirs, files in os.walk(out):
        for f in files:
            p = os.path.join(root, f)
            try:
                os.utime(p, (FIXED_ATIME, FIXED_MTIME), follow_symlinks=False)
            except (NotImplementedError, OSError):
                pass

    report = {
        "acl_setfacl_ok": acl_r.returncode == 0,
        "acl_setfacl_stderr": acl_r.stderr.strip(),
        "xattr_setfattr_ok": xattr_r.returncode == 0,
        "xattr_setfattr_stderr": xattr_r.stderr.strip(),
        "sparse_fallocate_ok": sparse_r.returncode == 0,
        "sparse_fallocate_stderr": sparse_r.stderr.strip(),
    }
    print(report)


if __name__ == "__main__":
    out_dir = Path(sys.argv[1]) if len(sys.argv) > 1 else Path("/root/eb-research/cache/baselines/fixture")
    build(out_dir)
    print(f"fixture built at {out_dir}")
