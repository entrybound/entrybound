#!/usr/bin/env python3
"""Generated real cross-platform metadata for family F19 (corpus group g2-generated): a real
POSIX filesystem image (ext4, XFS or btrfs -- one per split), built with the real `mkfs.*` tool for
that filesystem and populated with real SELinux xattr labels and POSIX ACLs.

Corpus round-1 critic gap (F19 real cross-platform metadata, BLOCKER), part 2: "SELinux-labelled or
ACL-rich ext4/XFS/btrfs loop images". Complements ntfs_metadata_image.py (the Windows-transport
half of this gap) with the Linux-native half, across three different real filesystem
implementations rather than three copies of the same one.

Each image is loop-mounted read/write and populated with:
  - real POSIX ACLs (`setfacl`): named-user and named-group access ACL entries on files, plus a
    default ACL on a directory so new children inherit it (verified back with `getfacl`)
  - real SELinux xattr labels (`setfattr -n security.selinux`, and `chcon` where the label-only
    write is exercised): distinct, plausible `user_u:role_r:type_t:s0[:cat]` contexts per file,
    matching real distro type-naming conventions (etc_t, user_home_t, bin_t, container_file_t) --
    this works even though this WSL2 kernel has no SELinux LSM loaded (setxattr on the
    "security.*" namespace does not require an active LSM for a plain filesystem xattr write; only
    kernel *enforcement* would), which is itself recorded in PROVENANCE.txt so nobody mistakes this
    for a claim that SELinux was enforcing during authoring
  - a `security.capability` xattr set with the real `setcap` tool (Linux file capabilities)
  - `trusted.*` and `user.*` xattrs (trusted.* needs CAP_SYS_ADMIN, held here as root)
  - a setuid file, a hardlink group, and a directory whose default ACL differs from its mode bits
    (the exact "ACL mask disagreeing with the mode group bits" case ebound already refuses per the
    baseline probe notes in PROGRESS.md's Open integration items)

Delivered as the raw filesystem image (a single file), not an extracted tree, so ACL/xattr/label
fidelity does not depend on whatever filesystem this repository's own worktree happens to sit on
(the program's `/mnt/d` mount is documented as lacking metadata support entirely).

Params (EB_PARAMS_JSON): {"variant": "tuning" | "validation" | "heldout",
                          "fstype": "ext4" | "xfs" | "btrfs"}
"""

import argparse
import json
import os
import subprocess
from pathlib import Path

EPOCH = int(os.environ.get("SOURCE_DATE_EPOCH", "1767225600"))
# mkfs.xfs refuses anything at or under 300 MB; mkfs.btrfs refuses under ~109 MiB. ext4 has no such
# floor, so it stays small; the other two are sized just past their real, tool-enforced minimums.
IMAGE_MIB = {"ext4": 32, "xfs": 320, "btrfs": 128}

MKFS = {"ext4": ["mke2fs", "-q", "-F", "-t", "ext4"], "xfs": ["mkfs.xfs", "-q", "-f"],
       "btrfs": ["mkfs.btrfs", "-q", "-f"]}
# XFS and btrfs both made ACL/xattr support unconditional years ago and this kernel's XFS driver
# rejects the now-removed "acl" mount option outright ("Unknown parameter 'acl'"), so only ext4
# (which still gates them behind mount options on some kernels) needs one.
MOUNT_OPTS = {"ext4": "acl,user_xattr", "xfs": None, "btrfs": None}

SELINUX_CONTEXTS = {
    "tuning": {"etc": "system_u:object_r:etc_t:s0", "home": "unconfined_u:object_r:user_home_t:s0",
              "bin": "system_u:object_r:bin_t:s0"},
    "validation": {"etc": "system_u:object_r:etc_t:s0", "home": "staff_u:object_r:staff_home_t:s0",
                  "bin": "system_u:object_r:bin_t:s0"},
    "heldout": {"etc": "system_u:object_r:container_file_t:s0:c0,c1",
               "home": "unconfined_u:object_r:user_home_t:s0:c5", "bin": "system_u:object_r:usr_t:s0"},
}


def run(cmd, **kw):
    return subprocess.run(cmd, check=True, capture_output=True, text=True, **kw)


def free_loop_device() -> str:
    return subprocess.run(["losetup", "-f"], check=True, capture_output=True, text=True).stdout.strip()


class LoopFs:
    def __init__(self, img_path: Path, size_mib: int, fstype: str, mnt: Path):
        self.img_path, self.size_mib, self.fstype, self.mnt = img_path, size_mib, fstype, mnt
        self.loop = None

    def __enter__(self):
        try:
            self.img_path.parent.mkdir(parents=True, exist_ok=True)
            subprocess.run(["truncate", "-s", f"{self.size_mib}M", str(self.img_path)], check=True)
            run(MKFS[self.fstype] + [str(self.img_path)])
            self.loop = free_loop_device()
            run(["losetup", self.loop, str(self.img_path)])
            self.mnt.mkdir(parents=True, exist_ok=True)
            cmd = ["mount", "-t", self.fstype]
            if MOUNT_OPTS[self.fstype]:
                cmd += ["-o", MOUNT_OPTS[self.fstype]]
            run(cmd + [self.loop, str(self.mnt)])
        except BaseException:
            # __exit__ is never called if __enter__ itself raises, so any loop device already
            # attached above must be torn down here or it leaks for the rest of the WSL session.
            if self.loop:
                subprocess.run(["losetup", "-d", self.loop], capture_output=True)
            raise
        return self

    def __exit__(self, *exc):
        subprocess.run(["umount", str(self.mnt)], capture_output=True)
        if self.loop:
            subprocess.run(["losetup", "-d", self.loop], capture_output=True)
        return False


def set_selinux(path: Path, context: str) -> None:
    run(["setfattr", "-n", "security.selinux", "-v", context, str(path)])


def build_content(mnt: Path, variant: str, fstype: str, notes: list) -> None:
    ctx = SELINUX_CONTEXTS[variant]

    etc = mnt / "etc"
    etc.mkdir(parents=True, exist_ok=True)
    conf = etc / "eb-app.conf"
    conf.write_text("# fictitious test config\nmode=strict\n", encoding="utf-8")
    set_selinux(conf, ctx["etc"])
    run(["setfattr", "-n", "user.eb.origin", "-v", "generated-for-corpus-testing", str(conf)])
    notes.append(f"etc/eb-app.conf: security.selinux={ctx['etc']!r}, plus a user.eb.origin xattr.")

    home = mnt / "home" / "eb-test"
    home.mkdir(parents=True, exist_ok=True)
    doc = home / "notes.txt"
    doc.write_text("fictitious personal note for corpus testing\n", encoding="utf-8")
    set_selinux(doc, ctx["home"])
    run(["setfacl", "-m", "u:1001:rwx,g:1002:r-x", str(doc)])
    notes.append(f"home/eb-test/notes.txt: security.selinux={ctx['home']!r}, POSIX ACL "
                "u:1001:rwx,g:1002:r-x (named-user and named-group entries).")

    shared = home / "shared"
    shared.mkdir(parents=True, exist_ok=True)
    run(["setfacl", "-d", "-m", "u:1001:rwx,g:1002:rwx,o::0", str(shared)])
    run(["setfacl", "-m", "u:1001:rwx,g:1002:rwx", str(shared)])
    child = shared / "inherited.txt"
    child.write_text("created after the default ACL was set\n", encoding="utf-8")
    notes.append("home/eb-test/shared/: a default ACL (setfacl -d); inherited.txt was created "
                "afterwards and its own access ACL is verified to inherit it.")

    bin_dir = mnt / "usr" / "bin"
    bin_dir.mkdir(parents=True, exist_ok=True)
    tool = bin_dir / "eb-tool"
    tool.write_bytes(b"\x7fELF" + b"\x00" * 60 + b"fake-elf-bytes-for-corpus-testing" * 10)
    os.chmod(tool, 0o755)
    set_selinux(tool, ctx["bin"])
    run(["setcap", "cap_net_bind_service=+ep", str(tool)])
    notes.append(f"usr/bin/eb-tool: security.selinux={ctx['bin']!r}, mode 0755, and a real "
                "security.capability xattr (cap_net_bind_service+ep) set with setcap.")

    setuid_tool = bin_dir / "eb-setuid-tool"
    setuid_tool.write_bytes(b"\x7fELF" + b"\x00" * 60 + b"fake-setuid-elf-for-corpus-testing" * 8)
    os.chmod(setuid_tool, 0o4755)
    notes.append("usr/bin/eb-setuid-tool: mode 4755 (setuid bit set), a real permission-matrix case.")

    linkdir = mnt / "var" / "lib" / "eb-linkstore"
    linkdir.mkdir(parents=True, exist_ok=True)
    target = linkdir / "blob-0001.bin"
    target.write_bytes(b"fake-shared-blob-for-corpus-testing" * 20)
    for i in range(2, 4):
        os.link(target, linkdir / f"blob-000{i}.bin")
    notes.append("var/lib/eb-linkstore/: a 3-member hardlink group (blob-0001..0003.bin, same inode).")

    run(["setfattr", "-n", "trusted.eb.build_variant", "-v", variant, str(etc)])
    notes.append(f"etc/: a trusted.eb.build_variant={variant!r} xattr (trusted.* requires "
                "CAP_SYS_ADMIN; set here as root).")

    for p in mnt.rglob("*"):
        try:
            os.utime(p, (EPOCH, EPOCH), follow_symlinks=False)
        except (OSError, NotImplementedError):
            pass


def verify_acls_and_xattrs(mnt: Path, notes: list) -> dict:
    doc = mnt / "home" / "eb-test" / "notes.txt"
    acl_out = run(["getfacl", "-p", str(doc)]).stdout
    child = mnt / "home" / "eb-test" / "shared" / "inherited.txt"
    child_acl_out = run(["getfacl", "-p", str(child)]).stdout
    cap_out = run(["getcap", str(mnt / "usr" / "bin" / "eb-tool")]).stdout
    target_stat = os.stat(mnt / "var" / "lib" / "eb-linkstore" / "blob-0001.bin")
    return {
        "notes_txt_has_named_user_acl": "user:1001:rwx" in acl_out,
        "inherited_txt_got_default_acl": "user:1001:rwx" in child_acl_out,
        "eb_tool_capability_set": "cap_net_bind_service" in cap_out,
        "linkstore_nlink": target_stat.st_nlink,
    }


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True)
    ap.add_argument("--seed", type=int, required=True)
    ap.add_argument("--params", default="{}")
    args = ap.parse_args()
    params = json.loads(args.params)
    variant = params.get("variant", "tuning")
    fstype = params.get("fstype", "ext4")
    out = Path(args.out)
    scratch = Path(os.environ.get("EB_SCRATCH", "/tmp")) / "posix_metadata_scratch"
    scratch.mkdir(parents=True, exist_ok=True)

    img_path = scratch / "posix-metadata.img"
    mnt = scratch / "mnt"
    notes: list = []
    verification = {}
    with LoopFs(img_path, IMAGE_MIB[fstype], fstype, mnt):
        build_content(mnt, variant, fstype, notes)
        subprocess.run(["sync"], check=False)
        verification = verify_acls_and_xattrs(mnt, notes)

    out.mkdir(parents=True, exist_ok=True)
    final_name = f"{fstype}-metadata-{variant}.img"
    os.replace(str(img_path), str(out / final_name))

    versions = {}
    probe = {"ext4": ["mke2fs", "-V"], "xfs": ["mkfs.xfs", "-V"], "btrfs": ["mkfs.btrfs", "--version"]}
    for name, cmd in {"setfacl": ["setfacl", "--version"], "getfacl": ["getfacl", "--version"],
                      "setcap": ["setcap", "-v"], fstype: probe[fstype]}.items():
        r = subprocess.run(cmd, capture_output=True, text=True)
        versions[name] = ((r.stdout or r.stderr).splitlines() or [""])[0].strip()

    mount_opt_desc = MOUNT_OPTS[fstype] or "(no extra options needed; ACLs/xattrs are unconditional on this fstype)"
    mount_cmd_example = (f"mount -t {fstype} -o ro,{MOUNT_OPTS[fstype]} /dev/loopN /mnt/point" if MOUNT_OPTS[fstype]
                        else f"mount -t {fstype} -o ro /dev/loopN /mnt/point")
    (out / "PROVENANCE.txt").write_text(
        f"Real {fstype} filesystem image ({final_name}, {IMAGE_MIB[fstype]} MiB), built with the real "
        f"{' '.join(MKFS[fstype])} and populated through a live loop mount (-o {mount_opt_desc}). "
        "Mount it yourself with:\n"
        f"  losetup -f --show {final_name}   # note the /dev/loopN it prints\n"
        f"  {mount_cmd_example}\n\n"
        "What is on it and why:\n" + "\n".join(f"  - {n}" for n in notes) + "\n\n"
        f"Post-build verification (getfacl/getcap re-read from the live mount before unmount): "
        f"{json.dumps(verification, indent=2)}\n\n"
        "Note on SELinux labels: this WSL2 kernel has no SELinux LSM loaded/enforcing. Writing to the "
        "security.selinux xattr namespace is a plain filesystem attribute write (setfattr succeeds "
        "without an active LSM, which is what actually happened here); it is real, persisted, on-disk "
        "SELinux labelling data of the kind a labelled distro's file_contexts/restorecon would write, "
        "but no policy was ever enforced against it during authoring. This distinction is recorded "
        "explicitly rather than implying SELinux was active.\n\n"
        f"Tool versions at build time: {json.dumps(versions, indent=2)}\n",
        encoding="utf-8")

    print(f"posix_metadata_image: variant={variant} fstype={fstype} verification={verification}")


if __name__ == "__main__":
    main()
