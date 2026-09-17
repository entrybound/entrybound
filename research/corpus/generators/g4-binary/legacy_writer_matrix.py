#!/usr/bin/env python3
"""Build step: write a real same-split source tree through a broad matrix of legacy archive
writers (g4-binary F14 gap closure, 2026-09-17: 7z filter/method/solid/encryption diversity, ZIP
dialects including ZIP64/ZipCrypto/AES/CP437/Shift-JIS/data-descriptors, tar dialects (v7, ustar,
oldgnu, gnu, pax, with GNU/SCHILY xattr+ACL and sparse-member extensions), cpio, WIM, CAB, and (via
WSL-to-Windows interop, no Docker/VM) genuine Windows-host bsdtar and PowerShell Compress-Archive
output).

Contract: python legacy_writer_matrix.py --out <staging> --seed N --params <JSON>

params:
  source_item:  item_id of a same-split real source tree, supplied via recipe.from_items; its
                materialized path is read from EB_FROM_<ITEM_ID> (uppercased, '-' -> '_')
  password:     shared demo password used by every encrypted variant below (recorded verbatim in
                the corpus item's license/description notes, exactly like the F20 private-vault
                items already do)
  windows:      true on a Windows host (WSL interop can reach real Windows binaries); false skips
                the Windows-writer section (used for the one held-out variant that instead only
                repeats the WSL-side tools, see the item definitions)

Every sub-step is independent and wrapped so one tool's failure (missing tool, an upstream tar
dialect rejecting a long path, etc.) is recorded in _generation_manifest.json rather than aborting
the whole item; the manifest lists every produced file, the tool invocation and its status.
The seed is unused (every writer is deterministic given the source tree and params).
"""

import argparse
import gzip
import io
import json
import os
import shutil
import stat
import struct
import subprocess
import sys
import tarfile
import zipfile
from pathlib import Path

WIN_TAR = "/mnt/c/Windows/System32/tar.exe"
WIN_POWERSHELL = "/mnt/c/Windows/System32/WindowsPowerShell/v1.0/powershell.exe"


class Manifest:
    def __init__(self):
        self.entries = []

    def ok(self, path, tool, note=""):
        self.entries.append({"path": path, "tool": tool, "status": "ok", "note": note})
        print(f"[ok] {path}  ({tool})", flush=True)

    def skip(self, path, tool, reason):
        self.entries.append({"path": path, "tool": tool, "status": "skipped", "note": reason})
        print(f"[skip] {path}  ({tool}): {reason}", flush=True)

    def write(self, out: Path):
        (out / "_generation_manifest.json").write_text(
            json.dumps({"schema": "ebrc-legacy-writer-matrix-v1", "entries": self.entries}, indent=2) + "\n")


def run(argv, m: Manifest, out_rel, cwd=None, input=None, stdout_path=None):
    """Run argv; on failure record a skip and return False; on success record ok and return True."""
    try:
        if stdout_path:
            with open(stdout_path, "wb") as f:
                r = subprocess.run(argv, cwd=cwd, input=input, stdout=f, stderr=subprocess.PIPE, timeout=900)
        else:
            r = subprocess.run(argv, cwd=cwd, input=input, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                timeout=900)
        if r.returncode != 0:
            err = (r.stderr or b"").decode("utf-8", "replace")[-800:]
            m.skip(out_rel, argv[0], f"exit {r.returncode}: {err}")
            return False
        m.ok(out_rel, argv[0])
        return True
    except (OSError, subprocess.TimeoutExpired) as e:
        m.skip(out_rel, argv[0], str(e))
        return False


def copy_source(src: Path, work: Path) -> Path:
    """A writable scratch copy: the from_items source tree is another item's materialized,
    fingerprint-referenced content and must never be mutated in place (xattrs/ACLs get applied
    below for the tar extended-attribute variants)."""
    dest = work / "src"
    subprocess.run(["cp", "-a", "--reflink=never", f"{src}/.", f"{dest}/"], check=True)
    return dest


# --------------------------------------------------------------------------------------------
# 7z matrix

def do_7z(src: Path, out: Path, password: str, m: Manifest) -> None:
    d = out / "7z"
    d.mkdir(parents=True, exist_ok=True)
    variants = [
        (["7z", "a", "-t7z", "-m0=LZMA", "-ms=off"], "lzma-nonsolid.7z", "LZMA, non-solid"),
        (["7z", "a", "-t7z", "-m0=LZMA2", "-ms=on"], "lzma2-solid.7z", "LZMA2, solid"),
        (["7z", "a", "-t7z", "-m0=LZMA2", "-mf=BCJ", "-ms=on"], "lzma2-bcj-solid.7z", "LZMA2+BCJ x86 filter, solid"),
        (["7z", "a", "-t7z", "-m0=BCJ2", "-m1=LZMA2", "-m2=LZMA2", "-m3=LZMA2"], "lzma2-bcj2.7z",
         "LZMA2 with the 4-stream BCJ2 x86 filter"),
        (["7z", "a", "-t7z", "-m0=PPMd", "-ms=on"], "ppmd-solid.7z", "PPMd, solid"),
        (["7z", "a", "-t7z", "-m0=PPMd", "-ms=off"], "ppmd-nonsolid.7z", "PPMd, non-solid"),
        (["7z", "a", "-t7z", "-m0=BZip2", "-ms=off"], "bzip2-nonsolid.7z", "BZip2, non-solid (BZip2 has its own "
         "blocking, so 7-Zip solid blocks add little; shown non-solid)"),
        (["7z", "a", "-t7z", "-m0=Deflate", "-ms=off"], "deflate.7z", "Deflate (7z container, non-LZMA method)"),
        (["7z", "a", "-t7z", "-m0=LZMA2", "-mhe=on", f"-p{password}"], "encrypted-headers.7z",
         f"LZMA2 with encrypted headers (-mhe=on); password {password!r} (recorded here and in the item notes)"),
        (["7z", "a", "-t7z", "-m0=LZMA2", "-v1m"], "multivolume.7z", "1 MiB multi-volume LZMA2 (produces "
         ".7z.001, .7z.002, ...)"),
    ]
    for argv, name, note in variants:
        target = d / name
        full = argv + [str(target), "src"]
        if run(full, m, f"7z/{name}", cwd=str(src.parent)):
            m.entries[-1]["note"] = note


# --------------------------------------------------------------------------------------------
# zip matrix

def do_zip(src: Path, out: Path, password: str, m: Manifest) -> None:
    d = out / "zip"
    d.mkdir(parents=True, exist_ok=True)
    work = src.parent

    def zip_cmd(argv, name, note, env=None):
        target = d / name
        full = ["zip"] + argv + [str(target), "-r", "src"]
        if run(full, m, f"zip/{name}", cwd=str(work)):
            m.entries[-1]["note"] = note

    zip_cmd(["-0", "-X"], "store-nocompress.zip", "compression level 0 (store)")
    zip_cmd(["-9", "-X"], "deflate-max.zip", "compression level 9 (max deflate)")
    zip_cmd(["-Z", "bzip2", "-X"], "bzip2-method.zip", "Info-Zip BZip2 compression method (method 12)")
    zip_cmd(["-y", "-X"], "symlinks-as-is.zip", "-y: store symlinks as symlinks rather than following them")
    zip_cmd(["-P", password, "-X"], "zipcrypto.zip", f"ZipCrypto traditional encryption; password {password!r}")

    # WinZip AES-256 via 7-Zip's zip writer (Info-Zip has no AES support)
    target = d / "winzip-aes256.zip"
    run(["7z", "a", "-tzip", "-mem=AES256", f"-p{password}", str(target), "src"], m, "zip/winzip-aes256.zip",
        cwd=str(work))
    if m.entries and m.entries[-1]["status"] == "ok":
        m.entries[-1]["note"] = f"WinZip AES-256 encryption (7-Zip zip writer); password {password!r}"

    # CP437 / Shift-JIS non-UTF-8 names: run under the POSIX/C locale so Info-Zip does not set the
    # UTF-8 language-encoding flag (general-purpose bit 11) for names outside ASCII.
    for charset, sample_name, note in (
            ("cp437", "caf\xe9-r\xe9sum\xe9.txt".encode("cp437"), "a CP437-only-representable name (accented "
             "Latin letters), written with the UTF-8 flag clear"),
            ("shiftjis", "\u30c6\u30b9\u30c8\u30d5\u30a1\u30a4\u30eb.txt".encode("shift_jis"),
             "a Shift-JIS-only-representable name (Japanese katakana), written with the UTF-8 flag clear")):
        namedir = work / f"{charset}-src"
        if namedir.exists():
            shutil.rmtree(namedir)
        namedir.mkdir(parents=True)
        # sample_name is a raw legacy-encoded byte string, which pathlib's `/` cannot join; use
        # os.path.join (bytes-safe) and a plain fd instead.
        raw_path = os.path.join(os.fsencode(namedir), sample_name)
        with open(raw_path, "wb") as f:
            f.write(f"{charset} legacy-encoding filename test\n".encode())
        target = d / f"{charset}-name.zip"
        env = dict(os.environ, LANG="C", LC_ALL="C")
        try:
            r = subprocess.run(["zip", "-X", "-q", "-r", str(target), f"{charset}-src"], cwd=str(work), env=env,
                                stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=120)
            if r.returncode == 0:
                with zipfile.ZipFile(target) as zf:
                    flagged_utf8 = any(i.flag_bits & 0x800 for i in zf.infolist())
                if flagged_utf8:
                    m.skip(f"zip/{charset}-name.zip", "zip", "wrote it, but the UTF-8 flag ended up set anyway")
                else:
                    m.ok(f"zip/{charset}-name.zip", "zip")
                    m.entries[-1]["note"] = note
            else:
                m.skip(f"zip/{charset}-name.zip", "zip", (r.stderr or b"").decode("utf-8", "replace")[-400:])
        except (OSError, subprocess.TimeoutExpired) as e:
            m.skip(f"zip/{charset}-name.zip", "zip", str(e))

    # ZIP64: forced by having more than 65535 entries (small files; Python's zipfile writes the
    # ZIP64 end-of-central-directory record automatically once the entry count crosses 65535).
    try:
        target = d / "zip64-many-entries.zip"
        with zipfile.ZipFile(target, "w", zipfile.ZIP_STORED) as zf:
            for i in range(70000):
                zf.writestr(f"entries/f{i:06d}.txt", b"x")
        m.ok("zip/zip64-many-entries.zip", "python-zipfile")
        m.entries[-1]["note"] = "70000 entries (>65535), forcing a ZIP64 end-of-central-directory record"
    except Exception as e:  # noqa: BLE001
        m.skip("zip/zip64-many-entries.zip", "python-zipfile", str(e))

    # ZIP64: a single entry whose logical (uncompressed) size exceeds 4 GiB. A sparse all-zero
    # file keeps this cheap (near-zero real disk use, trivially fast to deflate).
    try:
        big = work / "bigzero.bin"
        with open(big, "wb") as f:
            f.truncate(4 * 1024 ** 3 + 300 * 1024 ** 2)
        target = d / "zip64-large-logical-size.zip"
        with zipfile.ZipFile(target, "w", zipfile.ZIP_DEFLATED, compresslevel=1) as zf:
            zf.write(big, "bigzero.bin")
        big.unlink()
        m.ok("zip/zip64-large-logical-size.zip", "python-zipfile")
        m.entries[-1]["note"] = "one entry with >4 GiB uncompressed size (sparse all-zero content: compresses to " \
                                 "a few MiB), forcing a ZIP64 local/central-directory extra field"
    except Exception as e:  # noqa: BLE001
        m.skip("zip/zip64-large-logical-size.zip", "python-zipfile", str(e))

    # Data descriptors: zipfile only omits the local-header size/CRC (general-purpose bit 3, a
    # trailing data-descriptor record instead) when its output stream is not seekable, which is
    # what a real streaming writer (a pipe, a socket) forces; Info-Zip's own CLI always buffers
    # each entry first even when writing to a pipe, so it never needs one.
    class _NoSeek:
        def __init__(self, fp):
            self._fp = fp

        def write(self, b):
            return self._fp.write(b)

        def flush(self):
            return self._fp.flush()

    try:
        target = d / "data-descriptors.zip"
        with open(target, "wb") as raw:
            with zipfile.ZipFile(_NoSeek(raw), "w", zipfile.ZIP_DEFLATED) as zf:
                for p in sorted(src.rglob("*"))[:200]:
                    if p.is_file():
                        with zf.open(str(p.relative_to(src)), "w") as e:
                            e.write(p.read_bytes())
        with zipfile.ZipFile(target) as zf:
            data = target.read_bytes()
            has_dd = b"PK\x03\x04" in data and struct.unpack("<H", data[6:8])[0] & 0x08
        m.ok("zip/data-descriptors.zip", "python-zipfile")
        m.entries[-1]["note"] = f"streamed to a non-seekable sink, forcing data-descriptor records " \
                                 f"(first local header bit 3 set: {bool(has_dd)})"
    except Exception as e:  # noqa: BLE001
        m.skip("zip/data-descriptors.zip", "python-zipfile", str(e))


# --------------------------------------------------------------------------------------------
# tar dialects (GNU tar + bsdtar), cpio, WIM, CAB

def do_tar(src_ro: Path, work_src: Path, out: Path, m: Manifest) -> None:
    d = out / "tar"
    d.mkdir(parents=True, exist_ok=True)
    for fmt in ("v7", "ustar", "oldgnu", "gnu", "pax", "posix"):
        target = d / f"{fmt}.tar"
        run(["tar", f"--format={fmt}", "-cf", str(target), "src"], m, f"tar/{fmt}.tar", cwd=str(work_src.parent))

    run(["tar", "--format=gnu", "--sparse", "-cf", str(d / "gnu-sparse.tar"), "src"], m, "tar/gnu-sparse.tar",
        cwd=str(work_src.parent))
    if m.entries[-1]["status"] == "ok":
        m.entries[-1]["note"] = "GNU tar --sparse: the sparse member below is stored as a GNU sparse header " \
                                 "(data blocks + hole map), not its full apparent size"

    # xattrs + ACLs (GNU tar's --acls emits SCHILY.acl.access/default pax records, the same
    # convention schily's `star` uses, so this exercises that extension without needing star
    # itself).
    sample = next((p for p in work_src.rglob("*") if p.is_file()), None)
    xattr_ok = acl_ok = False
    if sample is not None:
        xattr_ok = subprocess.run(["setfattr", "-n", "user.ebrc_test", "-v", "legacy-writer-matrix", str(sample)],
                                   capture_output=True).returncode == 0
        acl_ok = subprocess.run(["setfacl", "-m", "u:1000:rwx", str(sample)], capture_output=True).returncode == 0
    target = d / "pax-xattrs-acls.tar"
    if run(["tar", "--format=pax", "--xattrs", "--xattrs-include=*", "--acls", "-cf", str(target), "src"], m,
           "tar/pax-xattrs-acls.tar", cwd=str(work_src.parent)):
        m.entries[-1]["note"] = f"pax format with GNU --xattrs/--acls (SCHILY.xattr.*/SCHILY.acl.* pax records); " \
                                 f"xattr set: {xattr_ok}, ACL set: {acl_ok}"

    # bsdtar (libarchive) pax writer -- a different implementation from GNU tar's.
    run(["bsdtar", "--format=pax", "-cf", str(d / "bsdtar-pax.tar"), "src"], m, "tar/bsdtar-pax.tar",
        cwd=str(work_src.parent))

    # cpio newc (SVR4) and odc (old portable ASCII)
    cd = out / "cpio"
    cd.mkdir(parents=True, exist_ok=True)
    for fmt, name in (("newc", "newc.cpio"), ("odc", "odc.cpio")):
        try:
            files = subprocess.run(["find", "src", "-type", "f", "-o", "-type", "l"], cwd=str(work_src.parent),
                                    capture_output=True, check=True).stdout
            with open(cd / name, "wb") as f:
                r = subprocess.run(["cpio", "-o", "-H", fmt], cwd=str(work_src.parent), input=files, stdout=f,
                                    stderr=subprocess.PIPE, timeout=300)
            if r.returncode == 0:
                m.ok(f"cpio/{name}", "cpio")
            else:
                m.skip(f"cpio/{name}", "cpio", (r.stderr or b"").decode("utf-8", "replace")[-400:])
        except (OSError, subprocess.TimeoutExpired) as e:
            m.skip(f"cpio/{name}", "cpio", str(e))

    # wimlib: three compression methods plus a solid (LZMS, always solid) variant
    wd = out / "wim"
    wd.mkdir(parents=True, exist_ok=True)
    for method, name in (("XPRESS", "xpress.wim"), ("LZX", "lzx.wim"), ("LZMS", "lzms-solid.wim")):
        run(["wimlib-imagex", "capture", "src", str(wd / name), f"--compress={method}"], m, f"wim/{name}",
            cwd=str(work_src.parent))

    # gcab CAB (a handful of representative files, not the whole tree: CAB is FAT-era and this is
    # meant to exercise the format, not act as a full backup)
    cabfiles = sorted(p for p in work_src.rglob("*") if p.is_file())[:40]
    if cabfiles:
        cd2 = out / "cab"
        cd2.mkdir(parents=True, exist_ok=True)
        run(["gcab", "-c", str(cd2 / "sample.cab")] + [str(p) for p in cabfiles], m, "cab/sample.cab")


# --------------------------------------------------------------------------------------------
# Windows-host writers (WSL-to-Windows interop; no Docker/VM; output kept on the D: drive only)

def _win_path(posix_path: Path) -> str:
    """The real Windows-style path (D:\\...) for a location under /mnt/d, for arguments handed to
    a native Windows process: WSL interop does not translate POSIX-style arguments on its own."""
    r = subprocess.run(["wslpath", "-w", str(posix_path)], capture_output=True, text=True, check=True)
    return r.stdout.strip()


def do_windows(work_src: Path, out: Path, m: Manifest) -> None:
    if not (Path(WIN_TAR).exists() and Path(WIN_POWERSHELL).exists()):
        m.skip("windows/*", "n/a", "Windows host tools not reachable from this WSL session")
        return
    # Per-item subdirectory: provision.py can run several build items concurrently (--jobs), and a
    # shared fixed path here raced between them (one item's rmtree/copy stepping on another's
    # concurrently-written files -- "OSError: Directory not empty" was observed running all three
    # F14 items at once).
    item_id = os.environ.get("EB_ITEM_ID", "item")
    win_scratch = Path("/mnt/d/eb-research/scratch-legacy-writer-matrix") / item_id
    if win_scratch.exists():
        shutil.rmtree(win_scratch)
    win_scratch.mkdir(parents=True)
    subprocess.run(["cp", "-a", "--reflink=never", str(work_src), str(win_scratch / "src")], check=True)
    # Symlinks on a 9p/drvfs-mounted Linux tree are not something the native Windows tools below
    # can stat reliably (bsdtar.exe and Compress-Archive both error out on them); symlink fidelity
    # is already covered by the WSL-side zip -y variant, so drop them here.
    subprocess.run(["find", str(win_scratch / "src"), "-type", "l", "-delete"], check=True)
    win_scratch_w = _win_path(win_scratch)

    d = out / "windows"
    d.mkdir(parents=True, exist_ok=True)

    def finish(produced: Path, dest_name: str, note: str) -> None:
        if not produced.exists():
            m.entries[-1] = {"path": f"windows/{dest_name}", "tool": m.entries[-1]["tool"], "status": "skipped",
                              "note": "the writer reported success but produced no output file"}
            return
        shutil.copyfile(produced, d / dest_name)
        m.entries[-1]["note"] = note

    tar_zip = win_scratch / "bsdtar-win.zip"
    if run([WIN_TAR, "-a", "-cf", win_scratch_w + "\\bsdtar-win.zip", "-C", win_scratch_w, "src"], m,
           "windows/bsdtar-win.zip"):
        finish(tar_zip, "bsdtar-win.zip", "written by the real C:/Windows/System32/tar.exe (bsdtar), auto zip format")

    tar_ustar = win_scratch / "bsdtar-win-ustar.tar"
    if run([WIN_TAR, "--format=ustar", "-cf", win_scratch_w + "\\bsdtar-win-ustar.tar", "-C", win_scratch_w, "src"],
           m, "windows/bsdtar-win-ustar.tar"):
        finish(tar_ustar, "bsdtar-win-ustar.tar", "written by the real C:/Windows/System32/tar.exe (bsdtar), ustar")

    ps_zip = win_scratch / "compress-archive.zip"
    ps_cmd = (f"Compress-Archive -Path '{win_scratch_w}\\src' -DestinationPath '{win_scratch_w}"
              f"\\compress-archive.zip' -CompressionLevel Optimal -Force")
    if run([WIN_POWERSHELL, "-NoProfile", "-NonInteractive", "-Command", ps_cmd], m, "windows/compress-archive.zip"):
        finish(ps_zip, "compress-archive.zip", "written by real Windows PowerShell 5.1 Compress-Archive (.NET "
               "ZipFile), deflate zip, on the Windows host (not WSL)")

    shutil.rmtree(win_scratch, ignore_errors=True)


# --------------------------------------------------------------------------------------------
# a format-faithful macOS Finder/ditto-style zip: __MACOSX/._<name> AppleDouble sidecars next to
# each visible entry. A real publicly downloaded macOS-authored zip with __MACOSX entries could
# not be located within this session's budget (every GitHub Actions-built macOS release .zip
# tried explicitly strips resource forks); this is GENERATED, not downloaded, and is labelled as
# such in the item's real_or_generated/description fields. The AppleDouble binary layout (magic,
# version, the Finder Info + Resource Fork entry descriptors) follows RFC 1740 and matches what
# ditto/Finder Compress actually write, so the *format* is faithful even though the bytes are not
# from a real capture.

def _appledouble_bytes(finder_info: bytes) -> bytes:
    magic = 0x00051607
    version = 0x00020000
    entries = [(9, finder_info)]  # entry id 9 = Finder Info
    header = struct.pack(">II16xH", magic, version, len(entries))
    body = b""
    offset = len(header) + len(entries) * 12
    descs = b""
    for eid, data in entries:
        descs += struct.pack(">III", eid, offset, len(data))
        body += data
        offset += len(data)
    return header + descs + body


def do_macos_zip(work_src: Path, out: Path, m: Manifest) -> None:
    d = out / "macos-zip"
    d.mkdir(parents=True, exist_ok=True)
    stage = work_src.parent / "macos-stage"
    if stage.exists():
        shutil.rmtree(stage)
    stage.mkdir(parents=True)
    visible = sorted(p for p in work_src.rglob("*") if p.is_file())[:25]
    macosx_root = stage / "__MACOSX" / "src"
    macosx_root.mkdir(parents=True)
    (stage / "src").mkdir()
    finder_info = bytes(32)  # zeroed FinderInfo (type/creator/flags all default): what ditto
    # emits for a plain file with no custom Finder attributes, which is the common case.
    for p in visible:
        rel = p.relative_to(work_src)
        dst = stage / "src" / rel
        dst.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(p, dst)
        ad_dir = macosx_root / rel.parent
        ad_dir.mkdir(parents=True, exist_ok=True)
        (ad_dir / f"._{rel.name}").write_bytes(_appledouble_bytes(finder_info))
    target = d / "macosx-appledouble.zip"
    if run(["zip", "-X", "-q", "-r", str(target), "__MACOSX", "src"], m, "macos-zip/macosx-appledouble.zip",
           cwd=str(stage)):
        m.entries[-1]["note"] = "GENERATED (format-faithful, not a real download; see module docstring): " \
                                 f"{len(visible)} visible files each paired with a __MACOSX/._<name> AppleDouble " \
                                 "sidecar (RFC 1740 layout, Finder Info entry)"
    shutil.rmtree(stage, ignore_errors=True)


# --------------------------------------------------------------------------------------------

def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True)
    ap.add_argument("--seed", type=int, default=0)
    ap.add_argument("--params", default="{}")
    args = ap.parse_args()
    p = json.loads(args.params)
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)

    env_name = "EB_FROM_" + p["source_item"].upper().replace("-", "_")
    src = Path(os.environ[env_name])
    password = p["password"]

    scratch = Path(os.environ["EB_SCRATCH"]) / "legacy_writer_matrix"
    if scratch.exists():
        shutil.rmtree(scratch)
    scratch.mkdir(parents=True)
    work_src = copy_source(src, scratch)

    m = Manifest()
    do_7z(work_src, out, password, m)
    do_zip(work_src, out, password, m)
    do_tar(src, work_src, out, m)
    do_macos_zip(work_src, out, m)
    if p.get("windows"):
        do_windows(work_src, out, m)
    else:
        m.skip("windows/*", "n/a", "windows param is false for this split's variant")

    m.write(out)
    shutil.rmtree(scratch, ignore_errors=True)
    n_ok = sum(1 for e in m.entries if e["status"] == "ok")
    n_skip = sum(1 for e in m.entries if e["status"] == "skipped")
    print(f"legacy_writer_matrix: {n_ok} ok, {n_skip} skipped (see _generation_manifest.json)")


if __name__ == "__main__":
    main()
