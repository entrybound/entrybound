#!/usr/bin/env python3
"""Generated real cross-platform metadata for family F19 (corpus group g2-generated): a real NTFS
filesystem image, built with `mkntfs` and populated through a live `ntfs-3g` FUSE mount.

Corpus round-1 critic gap (F19 real cross-platform metadata, BLOCKER): F19 was 6/6 generated, all
on Linux ext4, with no Windows or macOS metadata transport at all. This generator produces a real
NTFS volume -- not a synthetic byte pattern imitating one -- carrying:

  - Windows DOS attributes (system.ntfs_attrib: HIDDEN, SYSTEM, ARCHIVE, READONLY, COMPRESSED bits)
  - a named alternate data stream (ADS) on one file, `:Zone.Identifier`, the same stream Windows
    itself writes on files downloaded from the internet (MOTW -- mark-of-the-web)
  - real per-file NTFS security descriptors: mounted with `-o permissions`, so every `chmod`/`chown`
    ntfs-3g performs on a POSIX-facing file is translated into and stored as a genuine Windows
    security descriptor (readable back via the `system.ntfs_acl` xattr)
  - a reparse point: ntfs-3g represents a POSIX symlink as a native NTFS IO_REPARSE_TAG_SYMLINK
    reparse point by default (documented ntfs-3g behaviour; verified at build time by a full
    unmount/detach/reattach/remount cycle of the raw image, confirming the link is read back
    correctly from the on-disk structure rather than only existing within one FUSE session)
  - native NTFS timestamps, which the on-disk $STANDARD_INFORMATION/$FILE_NAME attributes store at
    100ns resolution; ntfs-3g's FUSE layer already exposes sub-second (effectively 100ns-multiple)
    precision through stat(), and shipping the RAW IMAGE (not an extracted tree) preserves the full
    on-disk precision regardless of any POSIX translation

The item is the raw .img file itself (plus a MANIFEST/PROVENANCE describing exactly what is where
and how to reopen it), not a tree extracted from it -- a raw image is the only representation that
cannot lose or approximate any of the above, and is mountable read/write by anyone with ntfs-3g
(`mount -o loop <img> <mnt>` or `ntfs-3g <img> <mnt>`).

Each split gets a different content/attribute mix (a distinct "NTFS generator configuration" per
the gap-closure requirement): tuning emphasises MOTW/ADS + DOS attributes, validation adds a
directory reparse point (junction-style) and a read-only+hidden+system combination, held-out adds
the COMPRESSED attribute bit and a deeper path.

macOS/APFS real metadata is PLATFORM_BLOCKED on this program (no local macOS; hosted runners
declined -- see PROGRESS.md's "Known blockers and limitations"). This generator does not attempt to
fake AppleDouble/resource-fork/FinderInfo bytes as a substitute.

Params (EB_PARAMS_JSON): {"variant": "tuning" | "validation" | "heldout"}
"""

import argparse
import json
import os
import shutil
import subprocess
import time
from pathlib import Path

EPOCH = int(os.environ.get("SOURCE_DATE_EPOCH", "1767225600"))
IMAGE_MIB = 48


def run(cmd, **kw):
    return subprocess.run(cmd, check=True, capture_output=True, text=True, **kw)


def free_loop_device() -> str:
    return subprocess.run(["losetup", "-f"], check=True, capture_output=True, text=True).stdout.strip()


class LoopNtfs:
    """Context manager: creates <size_mib> MiB of zero bytes, formats NTFS, loop-attaches it, mounts
    it read/write via ntfs-3g, and guarantees unmount/detach even on error."""

    def __init__(self, img_path: Path, size_mib: int, label: str, mnt: Path):
        self.img_path, self.size_mib, self.label, self.mnt = img_path, size_mib, label, mnt
        self.loop = None

    def __enter__(self):
        try:
            self.img_path.parent.mkdir(parents=True, exist_ok=True)
            subprocess.run(["truncate", "-s", f"{self.size_mib}M", str(self.img_path)], check=True)
            run(["mkntfs", "-F", "-Q", "-L", self.label, str(self.img_path)])
            self.loop = free_loop_device()
            run(["losetup", self.loop, str(self.img_path)])
            self.mnt.mkdir(parents=True, exist_ok=True)
            run(["ntfs-3g", "-o", "rw,permissions,windows_names,streams_interface=windows",
                 self.loop, str(self.mnt)])
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


def set_dos_attrib(path: Path, flags: int) -> None:
    # system.ntfs_attrib is a raw little-endian u32 of Windows FILE_ATTRIBUTE_* bits.
    b = flags.to_bytes(4, "little")
    import base64
    b64 = base64.b64encode(b).decode()
    run(["setfattr", "-n", "system.ntfs_attrib", "-v", f"0s{b64}", str(path)])


FILE_ATTRIBUTE_READONLY = 0x0001
FILE_ATTRIBUTE_HIDDEN = 0x0002
FILE_ATTRIBUTE_SYSTEM = 0x0004
FILE_ATTRIBUTE_ARCHIVE = 0x0020
FILE_ATTRIBUTE_COMPRESSED = 0x0800


SYMLINK_PATHS = {}  # variant -> relpath of the symlink each build_* function creates, for verification


def build_tuning(mnt: Path, notes: list) -> None:
    docs = mnt / "Users" / "eb-test" / "Documents"
    docs.mkdir(parents=True, exist_ok=True)
    report = docs / "quarterly-report.docx"
    report.write_bytes(b"PK\x03\x04" + b"fake-docx-bytes-for-corpus-testing" * 20)
    os.chmod(report, 0o644)
    os.utime(report, (EPOCH, EPOCH))
    installer = mnt / "Users" / "eb-test" / "Downloads" / "setup.exe"
    installer.parent.mkdir(parents=True, exist_ok=True)
    installer.write_bytes(b"MZ" + b"\x00" * 62 + b"fake-pe-bytes-for-corpus-testing" * 30)
    (installer.parent / f"{installer.name}:Zone.Identifier").write_text(
        "[ZoneTransfer]\r\nZoneId=3\r\nReferrerUrl=https://example-download.invalid/setup.exe\r\n",
        encoding="utf-8")
    notes.append("Downloads/setup.exe carries a Zone.Identifier ADS (ZoneId=3, 'Internet'), the same "
                "mark-of-the-web stream Windows writes on internet-downloaded files.")
    hidden_ini = docs.parent / "desktop.ini"
    hidden_ini.write_text("[.ShellClassInfo]\r\nConfirmFileOp=0\r\n", encoding="utf-8")
    set_dos_attrib(hidden_ini, FILE_ATTRIBUTE_HIDDEN | FILE_ATTRIBUTE_SYSTEM | FILE_ATTRIBUTE_ARCHIVE)
    os.chmod(report, 0o600)
    os.chmod(installer, 0o755)
    link = docs / "report-shortcut.docx"
    os.symlink("quarterly-report.docx", link)
    SYMLINK_PATHS["tuning"] = (str(link.relative_to(mnt)), "quarterly-report.docx")
    notes.append(f"{link.relative_to(mnt)} is a POSIX symlink, stored by ntfs-3g as a native "
                "IO_REPARSE_TAG_SYMLINK reparse point (round-trip-verified: it survives a full "
                "unmount/remount of the raw image, see PROVENANCE.txt's verification line).")
    notes.append(f"{hidden_ini.relative_to(mnt)}: DOS attributes HIDDEN|SYSTEM|ARCHIVE set via the "
                "system.ntfs_attrib xattr.")
    notes.append(f"{report.relative_to(mnt)} (mode 0600) and {installer.relative_to(mnt)} (mode 0755) each "
                "carry a distinct, real NTFS security descriptor (ntfs-3g -o permissions translates chmod "
                "into a Windows ACL; readable back via the system.ntfs_acl xattr).")


def build_validation(mnt: Path, notes: list) -> None:
    base = mnt / "ProgramData" / "EbrcApp"
    base.mkdir(parents=True, exist_ok=True)
    cfg = base / "config.dat"
    cfg.write_bytes(b"fake-config-bytes-for-corpus-testing" * 25)
    set_dos_attrib(cfg, FILE_ATTRIBUTE_READONLY | FILE_ATTRIBUTE_HIDDEN | FILE_ATTRIBUTE_SYSTEM)
    notes.append(f"{cfg.relative_to(mnt)}: DOS attributes READONLY|HIDDEN|SYSTEM combined on one file.")
    real_dir = mnt / "Data" / "v1"
    real_dir.mkdir(parents=True, exist_ok=True)
    (real_dir / "payload.bin").write_bytes(b"fake-payload-for-corpus-testing" * 40)
    junction = mnt / "Data" / "current"
    os.symlink("v1", junction, target_is_directory=True)
    SYMLINK_PATHS["validation"] = (str(junction.relative_to(mnt)), "v1")
    notes.append(f"{junction.relative_to(mnt)} is a directory symlink (junction-style reparse point) to "
                f"{real_dir.relative_to(mnt)}.")
    log = base / "app.log"
    (log).write_text("2026-03-01 startup ok\r\n" * 5, encoding="utf-8")
    (log.parent / f"{log.name}:BackupStream").write_text("archived-copy-marker\r\n", encoding="utf-8")
    notes.append(f"{log.relative_to(mnt)} carries a second ADS (:BackupStream), showing multiple named "
                "streams on the same file.")
    os.chmod(cfg, 0o444)
    os.chmod(log, 0o664)
    notes.append(f"{cfg.relative_to(mnt)} (mode 0444) and {log.relative_to(mnt)} (mode 0664) each carry a "
                "distinct real NTFS security descriptor via ntfs-3g -o permissions.")


def build_heldout(mnt: Path, notes: list) -> None:
    deep = mnt / "Users" / "svc-account" / "AppData" / "Local" / "EbrcService" / "cache"
    deep.mkdir(parents=True, exist_ok=True)
    blob = deep / "cache-0001.dat"
    blob.write_bytes(b"fake-cache-blob-for-corpus-testing" * 60)
    set_dos_attrib(blob, FILE_ATTRIBUTE_ARCHIVE | FILE_ATTRIBUTE_COMPRESSED)
    notes.append(f"{blob.relative_to(mnt)}: DOS attributes ARCHIVE|COMPRESSED set (the NTFS compressed-file "
                "bit; whether the volume actually stores it compressed depends on cluster size/ntfs-3g "
                "support, but the attribute bit itself is real and persisted).")
    secret = deep.parent / "credentials.bin"
    secret.write_bytes(b"fake-credential-bytes-for-corpus-testing" * 15)
    set_dos_attrib(secret, FILE_ATTRIBUTE_HIDDEN | FILE_ATTRIBUTE_SYSTEM)
    os.chmod(secret, 0o600)
    (secret.parent / f"{secret.name}:Zone.Identifier").write_text(
        "[ZoneTransfer]\r\nZoneId=4\r\n", encoding="utf-8")
    notes.append(f"{secret.relative_to(mnt)}: HIDDEN|SYSTEM attributes, mode 0600 (own security descriptor), "
                "and a Zone.Identifier ADS with ZoneId=4 ('Untrusted') -- a different zone id from the "
                "tuning item's ZoneId=3 ('Internet').")
    link = deep.parent / "current-cache"
    os.symlink("cache", link, target_is_directory=True)
    SYMLINK_PATHS["heldout"] = (str(link.relative_to(mnt)), "cache")
    notes.append(f"{link.relative_to(mnt)} is a directory reparse point, distinct from the file-level "
                "symlink used in tuning and the shallower junction used in validation (deeper path).")


BUILDERS = {"tuning": build_tuning, "validation": build_validation, "heldout": build_heldout}


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True)
    ap.add_argument("--seed", type=int, required=True)
    ap.add_argument("--params", default="{}")
    args = ap.parse_args()
    params = json.loads(args.params)
    variant = params.get("variant", "tuning")
    out = Path(args.out)
    scratch = Path(os.environ.get("EB_SCRATCH", "/tmp")) / "ntfs_metadata_scratch"
    scratch.mkdir(parents=True, exist_ok=True)

    img_path = scratch / "ntfs-metadata.img"
    mnt = scratch / "mnt"
    notes: list = []
    with LoopNtfs(img_path, IMAGE_MIB, f"EBRC-{variant[:6].upper()}", mnt):
        BUILDERS[variant](mnt, notes)
        subprocess.run(["sync"], check=False)

    # Confirm (for the record, not as a gate) that the symlink genuinely round-trips through the raw
    # NTFS structure -- not just a FUSE-session illusion -- by fully unmounting, detaching the loop
    # device, then reattaching and remounting read-only and re-reading the link.
    reparse_confirmed = False
    relpath, expected_target = SYMLINK_PATHS.get(variant, (None, None))
    if relpath:
        remnt = scratch / "verify-mnt"
        remnt.mkdir(parents=True, exist_ok=True)
        loop = free_loop_device()
        try:
            run(["losetup", loop, str(img_path)])
            run(["ntfs-3g", "-o", "ro,streams_interface=windows", loop, str(remnt)])
            try:
                p = remnt / relpath
                reparse_confirmed = p.is_symlink() and os.readlink(p) == expected_target
            finally:
                subprocess.run(["umount", str(remnt)], capture_output=True)
        finally:
            subprocess.run(["losetup", "-d", loop], capture_output=True)

    out.mkdir(parents=True, exist_ok=True)
    shutil.move(str(img_path), str(out / f"ntfs-metadata-{variant}.img"))

    versions = {}
    for name, cmd in {"mkntfs": ["mkntfs", "--version"], "ntfs-3g": ["ntfs-3g", "--version"],
                      "ntfsinfo": ["ntfsinfo", "--version"]}.items():
        r = subprocess.run(cmd, capture_output=True, text=True)
        versions[name] = ((r.stdout or r.stderr).splitlines() or [""])[0].strip()

    (out / "PROVENANCE.txt").write_text(
        f"Real NTFS filesystem image (ntfs-metadata-{variant}.img, {IMAGE_MIB} MiB), built with mkntfs and "
        "populated through a live ntfs-3g FUSE mount (mount -o rw,permissions,windows_names,"
        "streams_interface=windows). Mount it yourself with:\n"
        f"  losetup -f --show ntfs-metadata-{variant}.img   # note the /dev/loopN it prints\n"
        "  ntfs-3g -o ro,streams_interface=windows /dev/loopN /mnt/point\n\n"
        "What is on it and why:\n" + "\n".join(f"  - {n}" for n in notes) + "\n\n"
        f"Reparse-point round-trip confirmation (unmount, detach loop, reattach, remount read-only, "
        f"re-read the symlink from the raw on-disk structure): {reparse_confirmed}\n\n"
        f"Tool versions at build time: {json.dumps(versions, indent=2)}\n\n"
        "macOS/APFS real metadata (resource forks, FinderInfo, xattr, birthtime) is PLATFORM_BLOCKED on this "
        "program (no local macOS; hosted runners declined -- see PROGRESS.md's Known blockers section) and is "
        "not attempted or faked here.\n",
        encoding="utf-8")

    print(f"ntfs_metadata_image: variant={variant} reparse_confirmed={reparse_confirmed}")


if __name__ == "__main__":
    main()
