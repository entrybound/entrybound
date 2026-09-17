#!/usr/bin/env python3
"""Capability probes: for one representative config per incumbent family (plus
Entrybound), create+extract the fixture tree (make_fixtures.py) and record what
survived: non-UTF-8/Unicode paths, mtime/atime/ctime/birth precision,
permissions, ownership, symlinks, hardlinks, POSIX ACLs, xattrs, and sparse
files. Must run as root in WSL Ubuntu on ext4 (research/PROGRESS.md).

Writes research/baselines/probes/capability-probe-results.json (raw, per-config
detail) which research/baselines/build_capability_matrix.py turns into
research/baselines/capability-matrix.csv alongside the DOC-sourced rows for
capabilities that cannot be probed by round-tripping this fixture (encryption,
authentication, streaming, random access -- see that script).

Usage: run_capability_probes.py [--out-json PATH] [--fixture DIR] [--scratch DIR]
"""
from __future__ import annotations

import argparse
import json
import os
import shutil
import stat
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import make_fixtures  # noqa: E402

REPO_WSL = "/mnt/d/Projects/entrybound/entrybound"
EBOUND = "/root/eb-research/target/baseline/release/ebound"


def sh(cmd: str, timeout=600):
    # errors="replace": tool output can echo a fixture filename containing a
    # deliberately non-UTF-8 byte (the non_utf8_paths probe fixture); strict
    # decoding would crash the probe itself rather than record a real result.
    return subprocess.run(["bash", "-o", "pipefail", "-c", cmd], capture_output=True, text=True,
                           errors="replace", timeout=timeout)


# name -> (create_cmd(src, art), extract_cmd(art, dest), family)
def commands(src: str, art: str, dest: str):
    mk = f"mkdir -p {dest} && "
    return {
        "zip-9": (f"cd {src} && zip -9 -r -X -y -q {art} .",
                  mk + f"unzip -q -o {art} -d {dest}"),
        "7z-mx9-solid": (f"cd {src} && 7z a -t7z -mx=9 -snl -bd -y {art} . >/dev/null",
                         mk + f"7z x -y -o{dest} {art} >/dev/null"),
        "tar-gzip-9": (f"tar -cf - -C {src} . | gzip -9 > {art}",
                       mk + f"tar xzf {art} -C {dest}"),
        "tar-zstd-19": (f"tar -cf - -C {src} . | zstd -19 -q > {art}",
                        mk + f"tar --zstd -xf {art} -C {dest}"),
        "tar-xz-9e": (f"tar -cf - -C {src} . | xz -9 -e -q > {art}",
                      mk + f"tar xJf {art} -C {dest}"),
        "tar-lz4": (f"tar -cf - -C {src} . | lz4 -q > {art}",
                    mk + f"tar xf {art} -I lz4 -C {dest}"),
        "tar-brotli-q11": (f"tar -cf - -C {src} . | brotli -q 11 -c > {art}",
                           mk + f"brotli -d -c {art} | tar xf - -C {dest}"),
        "squashfs-zstd": (f"rm -f {art}; mksquashfs {src} {art} -comp zstd -no-progress -no-exports >/dev/null",
                          f"unsquashfs -f -d {dest} {art} >/dev/null"),
        "wim-lzms-solid": (f"rm -f {art}; wimcapture {src} {art} --compress=LZMS --solid >/dev/null",
                           mk + f"wimapply {art} 1 {dest}"),
        "borg": (f"rm -rf {art}; borg init --encryption=none {art} >/dev/null && borg create {art}::a {src} >/dev/null",
                 mk + f"cd {dest} && borg extract {art}::a"),
        "restic": (f"rm -rf {art}; restic init --repo {art} -q && restic backup {src} --repo {art} -q --host probe",
                   mk + f"restic restore latest --repo {art} --target {dest} -q"),
        "dwarfs": (f"rm -f {art}; mkdwarfs -i {src} -o {art} -l9 >/dev/null 2>&1",
                   mk + f"dwarfsextract -i {art} -o {dest}"),
        "zpaq": (f"rm -f {art}; zpaq a {art} {src} -m5 -t1 >/dev/null",
                 mk + f"zpaq x {art} -to {dest} -f >/dev/null"),
        "entrybound-ebound": (f"{EBOUND} pack {src} {art} --profile balanced >/dev/null",
                              mk + f"{EBOUND} convert {art} {dest}/rt.tar --to tar --allow-lossy >/dev/null && "
                              f"tar xf {dest}/rt.tar -C {dest}"),
    }


RESTIC_ENV = {"RESTIC_PASSWORD": "ebr-baseline-research-only"}


def rebase(item_path: Path, dest: Path) -> Path:
    want = item_path.name
    cur = dest
    for _ in range(24):
        try:
            entries = [e for e in os.listdir(cur) if not e.startswith(".")]
        except OSError:
            break
        if len(entries) == 1 and (cur / entries[0]).is_dir():
            cur = cur / entries[0]
            if cur.name == want:
                return cur
            continue
        break
    return dest


def lstat_or_none(p: Path):
    try:
        return os.lstat(p)
    except OSError:
        return None


def check_path(dest_root: Path, rel: str, expect_content: bytes | None = None):
    p = dest_root / rel
    st = lstat_or_none(p)
    if st is None:
        return {"present": False}
    out = {"present": True}
    if expect_content is not None and stat.S_ISREG(st.st_mode):
        try:
            out["content_ok"] = p.read_bytes() == expect_content
        except OSError as e:
            out["content_ok"] = False
            out["error"] = str(e)
    return out


def probe_one(name: str, create_cmd: str, extract_cmd: str, fixture: Path, scratch: Path) -> dict:
    art = scratch / f"{name}.art"
    dest = scratch / f"{name}.dest"
    if art.exists():
        if art.is_dir():
            shutil.rmtree(art)
        else:
            art.unlink()
    shutil.rmtree(dest, ignore_errors=True)
    env = dict(os.environ)
    if name == "restic":
        env.update(RESTIC_ENV)

    r = {"config": name}
    cr = subprocess.run(["bash", "-o", "pipefail", "-c", create_cmd], capture_output=True, text=True,
                        errors="replace", env=env, timeout=1200)
    r["create_exit"] = cr.returncode
    if cr.returncode != 0:
        r["create_stderr"] = cr.stderr[-2000:]
        return r
    er = subprocess.run(["bash", "-o", "pipefail", "-c", extract_cmd], capture_output=True, text=True,
                        errors="replace", env=env, timeout=1200)
    r["extract_exit"] = er.returncode
    if er.returncode != 0:
        # Non-fatal: e.g. 7z/wimlib refuse an absolute-path symlink that no longer
        # resolves inside the extraction root ("dangerous link path") but still
        # extract everything else. Record the warning and inspect what landed.
        r["extract_stderr"] = er.stderr[-2000:]
        r["extract_nonzero_but_continuing"] = True

    root = rebase(fixture, dest)
    r["extract_root"] = str(root)

    # unicode path
    uni = [d for d in os.listdir(root) if d.startswith("unicode-")] if root.is_dir() else []
    r["unicode_dir_present"] = bool(uni)
    if uni:
        files = os.listdir(root / uni[0])
        r["unicode_file_present"] = any("caf" in f for f in files)

    # non-utf8 path: scan raw bytes of directory entries
    nonutf8_present = False
    try:
        for entry in os.listdir(os.fsencode(root)):
            if b"nonutf8-" in entry:
                nonutf8_present = True
    except OSError:
        pass
    r["nonutf8_present"] = nonutf8_present

    # permissions
    perm_res = {}
    for fn, want_mode in [("perms/mode644.txt", 0o644), ("perms/mode600.txt", 0o600),
                          ("perms/mode755.sh", 0o755), ("perms/mode700dir", 0o700)]:
        st = lstat_or_none(root / fn)
        if st is None:
            perm_res[fn] = None
        else:
            perm_res[fn] = {"got": oct(stat.S_IMODE(st.st_mode)), "want": oct(want_mode), "match": stat.S_IMODE(st.st_mode) == want_mode}
    r["permissions"] = perm_res

    # ownership
    st = lstat_or_none(root / "ownership/owned-by-1000.txt")
    r["ownership"] = None if st is None else {"uid": st.st_uid, "gid": st.st_gid, "match": (st.st_uid, st.st_gid) == (1000, 1000)}

    # symlinks
    sym_res = {}
    for name_, want_target in [("symlinks/rel-link.txt", "target.txt"),
                                ("symlinks/dangling-link.txt", "does-not-exist.txt")]:
        p = root / name_
        st = lstat_or_none(p)
        if st is None:
            sym_res[name_] = {"present": False}
        elif stat.S_ISLNK(st.st_mode):
            sym_res[name_] = {"present": True, "is_symlink": True, "target": os.readlink(p), "target_match": os.readlink(p) == want_target}
        else:
            sym_res[name_] = {"present": True, "is_symlink": False}
    r["symlinks"] = sym_res

    # hardlinks
    p1, p2 = root / "hardlinks/primary.txt", root / "hardlinks/secondary.txt"
    s1, s2 = lstat_or_none(p1), lstat_or_none(p2)
    if s1 and s2:
        r["hardlinks"] = {"present": True, "same_inode": s1.st_ino == s2.st_ino, "nlink1": s1.st_nlink, "nlink2": s2.st_nlink}
    else:
        r["hardlinks"] = {"present": False}

    # ACL
    acl_p = root / "acl/acl-file.txt"
    if acl_p.exists():
        acl_out = sh(f"getfacl -p {acl_p} 2>&1")
        r["acl"] = {"present": True, "has_entry": "user:1234:rwx" in acl_out.stdout}
    else:
        r["acl"] = {"present": False}

    # xattr
    xattr_p = root / "xattr/xattr-file.txt"
    if xattr_p.exists():
        xattr_out = sh(f"getfattr -n user.ebr_probe --only-values {xattr_p} 2>&1")
        r["xattr"] = {"present": True, "value": xattr_out.stdout.strip(), "match": xattr_out.stdout.strip() == "hello_xattr"}
    else:
        r["xattr"] = {"present": False}

    # sparse
    sp = root / "sparse/sparse.bin"
    st = lstat_or_none(sp)
    if st is not None:
        apparent = st.st_size
        allocated = st.st_blocks * 512
        r["sparse"] = {"present": True, "apparent_bytes": apparent, "allocated_bytes": allocated,
                        "still_sparse": allocated < apparent * 0.5}
    else:
        r["sparse"] = {"present": False}

    # timestamps: compare a plain perms file's mtime/atime to the fixed values;
    # ctime cannot be probed (it is always the extraction moment, not restorable
    # archive data) and is reported as such by the caller, not measured here.
    tf = root / "perms/mode644.txt"
    st = lstat_or_none(tf)
    if st is not None:
        r["mtime"] = {"got": int(st.st_mtime), "want": make_fixtures.FIXED_MTIME, "match": int(st.st_mtime) == make_fixtures.FIXED_MTIME}
        r["atime"] = {"got": int(st.st_atime), "want": make_fixtures.FIXED_ATIME, "match": int(st.st_atime) == make_fixtures.FIXED_ATIME}
        birth = sh(f"stat -c '%W' {tf}")
        try:
            birth_val = int(birth.stdout.strip())
        except ValueError:
            birth_val = None
        r["birthtime_raw"] = birth_val  # 0 or None commonly means unsupported/unset on this fs+tool
    return r


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--out-json", default=f"{REPO_WSL}/research/baselines/probes/capability-probe-results.json")
    ap.add_argument("--fixture", default="/root/eb-research/cache/baselines/fixture")
    ap.add_argument("--scratch", default="/root/eb-research/scratch/baselines/capability")
    ap.add_argument("--only", help="comma-separated subset of config names")
    args = ap.parse_args()

    fixture = Path(args.fixture)
    fixture_safe = Path(str(args.fixture) + "-utf8-safe")
    fixture_lenient = Path(str(args.fixture) + "-lenient")  # no non-utf8 path, no ACL fixture
    fixture_maxlenient = Path(str(args.fixture) + "-maxlenient")  # + no symlinks
    scratch = Path(args.scratch)
    scratch.mkdir(parents=True, exist_ok=True)
    make_fixtures.build(fixture, include_nonutf8=True, include_acl=True, include_symlinks=True)
    make_fixtures.build(fixture_safe, include_nonutf8=False, include_acl=True, include_symlinks=True)
    make_fixtures.build(fixture_lenient, include_nonutf8=False, include_acl=False, include_symlinks=True)
    make_fixtures.build(fixture_maxlenient, include_nonutf8=False, include_acl=False, include_symlinks=False)

    def strict_refusal_probe(cmd: str) -> dict:
        cp = subprocess.run(["bash", "-c", cmd], capture_output=True, text=True, errors="replace", timeout=180)
        return {"exit_code": cp.returncode, "refused": cp.returncode != 0, "stderr": cp.stderr.strip()[-500:]}

    # wimlib and Entrybound both refuse their WHOLE input tree outright on a
    # non-UTF-8 filename; Entrybound additionally refuses a tree containing an
    # ACL whose mask disagrees with its reported POSIX mode bits
    # (EB_EAM_INVALID_ACL). Each refusal IS the probe result for that capability,
    # captured directly here against the fixture that actually triggers it, before
    # falling back to a fixture the tool accepts to probe its other capabilities.
    strict_findings = {
        "wim-lzms-solid": {
            "nonutf8": strict_refusal_probe(f"wimcapture {fixture} {scratch}/wim-nonutf8-check.wim --compress=LZMS --solid"),
        },
        "entrybound-ebound": {
            "nonutf8": strict_refusal_probe(f"{EBOUND} pack {fixture} {scratch}/entrybound-nonutf8-check.eb --profile balanced"),
            "acl_mode_mismatch": strict_refusal_probe(f"{EBOUND} pack {fixture_safe} {scratch}/entrybound-acl-check.eb --profile balanced"),
            "symlink_export": strict_refusal_probe(
                f"{EBOUND} pack {fixture_lenient} {scratch}/entrybound-symlink-check.eb --profile balanced >/dev/null && "
                f"{EBOUND} convert {scratch}/entrybound-symlink-check.eb {scratch}/entrybound-symlink-check.tar --to tar --allow-lossy"
            ),
        },
    }
    fixture_for = {"wim-lzms-solid": fixture_safe, "entrybound-ebound": fixture_maxlenient}

    cmds = commands(str(fixture), "{art}", "{dest}")
    for cfg, fx in fixture_for.items():
        cmds[cfg] = tuple(c.replace(str(fixture), str(fx)) for c in cmds[cfg])

    only = set(args.only.split(",")) if args.only else None
    results = {}
    for name, (create_tmpl, extract_tmpl) in cmds.items():
        if only and name not in only:
            continue
        use_fixture = fixture_for.get(name, fixture)
        art = str(scratch / f"{name}.art")
        dest = str(scratch / f"{name}.dest")
        create_cmd = create_tmpl.replace("{art}", art).replace("{dest}", dest)
        extract_cmd = extract_tmpl.replace("{art}", art).replace("{dest}", dest)
        print(f"probing {name} ...", file=sys.stderr)
        try:
            results[name] = probe_one(name, create_cmd, extract_cmd, use_fixture, scratch)
        except Exception as exc:  # noqa: BLE001
            results[name] = {"config": name, "exception": str(exc)}
        if name in strict_findings:
            results[name]["strict_refusal_findings"] = strict_findings[name]
            if "nonutf8" in strict_findings[name]:
                results[name]["nonutf8_present"] = False  # refused before creating any archive
        print(json.dumps(results[name])[:300], file=sys.stderr)

    Path(args.out_json).write_text(json.dumps(results, indent=2, sort_keys=True), encoding="utf-8")
    print(f"wrote {args.out_json}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
