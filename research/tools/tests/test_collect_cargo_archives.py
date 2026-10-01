"""Generated Cargo locks/archives only; never reads an existing corpus or cache."""

import gzip
import errno
import hashlib
import importlib.util
import io
import json
import os
import stat
import sys
import tarfile
from pathlib import Path

import pytest

if sys.platform != "linux":
    pytest.skip("offline Cargo collector requires Linux/WSL", allow_module_level=True)

SCRIPT = Path(__file__).resolve().parents[2] / "corpus/generators/g1-code/collect_cargo_archives.py"
spec = importlib.util.spec_from_file_location("collect_cargo_archives_test", SCRIPT)
collector = importlib.util.module_from_spec(spec)
spec.loader.exec_module(collector)

MIT = b"""MIT License
Copyright (c) 2026 Synthetic fixture authors
Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the Software), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies.
The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.
THE SOFTWARE IS PROVIDED AS IS, WITHOUT WARRANTY OF ANY KIND.
"""
UNICODE = b"""UNICODE, INC. LICENSE AGREEMENT - DATA FILES AND SOFTWARE
Copyright (c) Synthetic fixture authors
Permission is hereby granted, free of charge, to any person obtaining a copy
of the Unicode data files and any associated documentation to use the data files,
provided that this copyright and permission notice is retained.
"""
APACHE = b"""Apache License
Version 2.0, January 2004
TERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION
Synthetic fixture: retain copyright, license and NOTICE attribution.
"""


def sha(data):
    return hashlib.sha256(data).hexdigest()


def archive_bytes(name, version, files, extras=(), first_extras=()):
    buffer = io.BytesIO()
    with tarfile.open(fileobj=buffer, mode="w", format=tarfile.USTAR_FORMAT) as archive:
        for info, data in first_extras:
            archive.addfile(info, io.BytesIO(data) if data is not None else None)
        for relative, content in files.items():
            info = tarfile.TarInfo(f"{name}-{version}/{relative}")
            info.size, info.mode, info.mtime = len(content), 0o644, 17
            archive.addfile(info, io.BytesIO(content))
        for info, data in extras:
            archive.addfile(info, io.BytesIO(data) if data is not None else None)
    return gzip.compress(buffer.getvalue(), mtime=0)


class SyntheticInputs:
    def __init__(self, root):
        root.chmod(0o700)
        self.root, self.lock, self.cache, self.out = root, root / "Cargo.lock", root / "cache", root / "out"
        self.shard = self.cache / "index.crates.io-synthetic"
        self.shard.mkdir(parents=True)
        self.packages = [
            {"name": "alpha-one", "version": "1.0.0", "license": "MIT"},
            {"name": "zeta-two", "version": "1.2.3+metadata", "license": "(MIT OR Apache-2.0) AND Unicode-DFS-2016"},
        ]
        self.files = []
        for package in self.packages:
            files = {"Cargo.toml": self.metadata(package), "LICENSE-MIT": MIT,
                     "src/data.bin": bytes(range(256)) * 3,
                     "third-party/NOTICE": b"Synthetic third-party notice: preserve this exact original."}
            if "Unicode" in package["license"]:
                files["LICENSE-UNICODE"] = UNICODE
            self.files.append(files)
        for index in range(2):
            self.rebuild(index)

    @staticmethod
    def metadata(package):
        return (f'[package]\nname = "{package["name"]}"\nversion = "{package["version"]}"\n'
                f'license = "{package["license"]}"\n').encode()

    def path(self, index):
        package = self.packages[index]
        return self.shard / f'{package["name"]}-{package["version"]}.crate'

    def rebuild(self, index, extras=(), payload=None):
        package = self.packages[index]
        payload = archive_bytes(package["name"], package["version"], self.files[index], extras) if payload is None else payload
        self.path(index).write_bytes(payload)
        self.packages[index]["checksum"] = sha(payload)
        if all("checksum" in package for package in self.packages):
            self.write_lock()

    def write_lock(self, extra=""):
        text = "version = 3\n"
        for package in reversed(self.packages):
            text += ('\n[[package]]\n' + ''.join(f'{key} = "{package[key]}"\n' for key in ("name", "version"))
                     + f'source = "{package.get("source", collector.REGISTRY)}"\n'
                     + f'checksum = "{package["checksum"]}"\n')
        self.lock.write_text(text + extra, encoding="utf-8")

    def args(self, **updates):
        args = dict(lock=self.lock, lock_sha256=sha(self.lock.read_bytes()), cache=self.cache,
                    scratch_root=self.root, out=self.out,
                    source_commit="1" * 40, source_split="tuning", expected_count=len(self.packages))
        args.update(updates)
        return args


@pytest.fixture
def inputs(tmp_path, monkeypatch):
    monkeypatch.delenv("EB_SPLIT", raising=False)
    return SyntheticInputs(tmp_path)


def assert_no_output(inputs):
    assert not inputs.out.exists()
    assert not list(inputs.root.glob(".cargo-originals-*"))


def test_original_bytes_notices_and_deterministic_publication(inputs):
    lock_before = inputs.lock.read_bytes()
    archive_before = {inputs.path(i).name: inputs.path(i).read_bytes() for i in range(2)}
    first = collector.collect(**inputs.args())
    second_out = inputs.root / "second"
    second = collector.collect(**inputs.args(out=second_out))
    assert first == second
    assert [p["name"] for p in first["packages"]] == ["alpha-one", "zeta-two"]
    assert first["archive_bytes"] == sum(map(len, archive_before.values()))
    assert first["lock_sha256"] == sha(lock_before)
    assert first["tool_sha256"] == sha(SCRIPT.read_bytes())
    assert (inputs.out / "Cargo.lock").read_bytes() == lock_before
    manifest = json.loads((inputs.out / "provenance-notices.json").read_text())
    assert manifest == first
    for row in manifest["packages"]:
        assert (inputs.out / row["archive_path"]).read_bytes() == archive_before[Path(row["archive_path"]).name]
        assert row["published_url"].startswith("https://static.crates.io/crates/")
        notice = next(n for n in row["license_notices"] if n["path"] == "third-party/NOTICE")
        assert notice["sha256"] == sha(inputs.files[0]["third-party/NOTICE"])
    assert manifest["packages"][1]["selected_license_text_clauses"] == ["MIT", "Unicode-DFS-2016"]
    for path in [inputs.out, *inputs.out.rglob("*")]:
        relative = path.relative_to(inputs.out)
        twin = second_out / relative
        assert path.stat().st_mtime == collector.FIXED_MTIME
        assert stat.S_IMODE(path.stat().st_mode) == (0o755 if path.is_dir() else 0o644)
        if path.is_file():
            assert path.read_bytes() == twin.read_bytes()
    assert inputs.lock.read_bytes() == lock_before
    assert {inputs.path(i).name: inputs.path(i).read_bytes() for i in range(2)} == archive_before


def test_existing_empty_caller_output_is_refused(inputs):
    inputs.out.mkdir()
    with pytest.raises(collector.Refusal, match="already exists"):
        collector.collect(**inputs.args())
    assert not list(inputs.out.iterdir())


@pytest.mark.parametrize("failure", ["missing", "duplicate", "checksum", "lock-pin", "bad-lock", "bad-registry",
                                    "duplicate-identity", "unsafe-name", "unsafe-version", "bad-checksum", "count"])
def test_lock_cache_refusals_before_output(inputs, failure, monkeypatch):
    overrides = {}
    if failure == "missing":
        inputs.path(0).unlink()
    elif failure == "duplicate":
        duplicate = inputs.cache / "second-registry"
        duplicate.mkdir()
        (duplicate / inputs.path(0).name).write_bytes(inputs.path(0).read_bytes())
    elif failure == "checksum":
        inputs.path(0).write_bytes(b"tampered compressed original")
    elif failure == "lock-pin":
        overrides["lock_sha256"] = "0" * 64
    elif failure == "bad-lock":
        inputs.lock.write_text("package = 7", encoding="utf-8")
    elif failure == "bad-registry":
        inputs.packages[0]["source"] = "registry+https://example.invalid/index"
        inputs.write_lock()
    elif failure == "duplicate-identity":
        inputs.packages.append(dict(inputs.packages[0]))
        inputs.write_lock()
    elif failure == "unsafe-name":
        inputs.packages[0]["name"] = "../outside"
        inputs.write_lock()
    elif failure == "unsafe-version":
        inputs.packages[0]["version"] = "1.0.0/../outside"
        inputs.write_lock()
    elif failure == "bad-checksum":
        inputs.packages[0]["checksum"] = "not-a-checksum"
        inputs.write_lock()
    else:
        overrides["expected_count"] = 3
    monkeypatch.setattr(collector, "publish", lambda *args: pytest.fail("validation reached publication"))
    with pytest.raises(collector.Refusal):
        collector.collect(**inputs.args(**overrides))
    assert_no_output(inputs)


@pytest.mark.parametrize("failure", ["bad-gzip", "gzip-crc", "gzip-deflate", "bad-tar", "traversal", "absolute", "backslash",
                                    "symlink", "hardlink", "fifo", "duplicate-member", "path-alias", "file-trailing-slash", "wrong-name", "wrong-version",
                                    "missing-metadata", "bad-metadata", "missing-license", "missing-and", "unknown-license",
                                    "missing-legacy-clause", "trailing-data"])
def test_archive_license_refusals_before_output(inputs, failure, monkeypatch):
    extras, payload = [], None
    if failure == "bad-gzip":
        payload = b"not gzip"
    elif failure == "gzip-crc":
        payload = bytearray(inputs.path(0).read_bytes())
        payload[-8] ^= 1
        payload = bytes(payload)
    elif failure == "gzip-deflate":
        payload = bytearray(inputs.path(0).read_bytes())
        payload[10] = 255  # Reserved deflate block type: raises zlib.error.
        payload = bytes(payload)
    elif failure == "bad-tar":
        payload = gzip.compress(b"not a tar stream", mtime=0)
    elif failure in ("traversal", "absolute", "backslash", "symlink", "hardlink", "fifo", "duplicate-member", "path-alias", "file-trailing-slash"):
        prefix = "alpha-one-1.0.0/"
        name = {"traversal": prefix + "../escape", "absolute": "/absolute/escape",
                "backslash": prefix + "path\\escape", "duplicate-member": prefix + "LICENSE-MIT",
                "path-alias": prefix + "src//data.bin", "file-trailing-slash": prefix + "src/new-file/"}.get(failure, prefix + "link")
        info = tarfile.TarInfo(name)
        if failure in ("symlink", "hardlink", "fifo"):
            info.type = {"symlink": tarfile.SYMTYPE, "hardlink": tarfile.LNKTYPE, "fifo": tarfile.FIFOTYPE}[failure]
            info.linkname = "../../forbidden"
            extras.append((info, None))
        else:
            info.size = 3
            extras.append((info, b"bad"))
    elif failure in ("wrong-name", "wrong-version"):
        package = dict(inputs.packages[0])
        package["name" if failure == "wrong-name" else "version"] = "wrong" if failure == "wrong-name" else "9.0.0"
        inputs.files[0]["Cargo.toml"] = inputs.metadata(package)
    elif failure == "missing-metadata":
        del inputs.files[0]["Cargo.toml"]
    elif failure == "bad-metadata":
        inputs.files[0]["Cargo.toml"] = b"[package"
    elif failure == "missing-license":
        del inputs.files[0]["LICENSE-MIT"]
    elif failure == "missing-and":
        del inputs.files[1]["LICENSE-UNICODE"]
    elif failure in ("unknown-license", "missing-legacy-clause"):
        inputs.packages[0]["license"] = "MIT AND Mystery" if failure == "unknown-license" else "MIT/Apache-2.0"
        inputs.files[0]["Cargo.toml"] = inputs.metadata(inputs.packages[0])
    else:
        payload = gzip.compress(gzip.decompress(inputs.path(0).read_bytes()) + b"hidden uninspected bytes", mtime=0)
    index = 1 if failure == "missing-and" else 0
    inputs.rebuild(index, extras=extras, payload=payload)
    monkeypatch.setattr(collector, "publish", lambda *args: pytest.fail("validation reached publication"))
    with pytest.raises(collector.Refusal):
        collector.collect(**inputs.args())
    assert_no_output(inputs)


def test_or_branch_and_legacy_both_clause_evidence(inputs):
    inputs.packages[0]["license"] = "MIT OR Apache-2.0"
    inputs.files[0]["Cargo.toml"] = inputs.metadata(inputs.packages[0])
    del inputs.files[0]["LICENSE-MIT"]
    inputs.files[0]["LICENSE-APACHE"] = APACHE
    inputs.rebuild(0)
    manifest = collector.collect(**inputs.args())
    assert manifest["packages"][0]["selected_license_text_clauses"] == ["Apache-2.0"]


def test_conventional_directory_trailing_slash_is_permitted(inputs):
    root = tarfile.TarInfo("alpha-one-1.0.0/")
    root.type = tarfile.DIRTYPE
    directory = tarfile.TarInfo("alpha-one-1.0.0/src/")
    directory.type = tarfile.DIRTYPE
    inputs.rebuild(0, extras=[(root, None), (directory, None)])
    assert collector.collect(**inputs.args())["archive_count"] == 2


@pytest.mark.parametrize("member,order", [("root", "before"), ("root", "after"),
                                        ("ancestor", "before"), ("ancestor", "after")])
def test_regular_root_or_ancestor_conflicts_refuse_before_output(inputs, monkeypatch, member, order):
    name = "alpha-one-1.0.0" + ("/src" if member == "ancestor" else "")
    info = tarfile.TarInfo(name)
    info.size = 3
    extra = [(info, b"bad")]
    payload = archive_bytes("alpha-one", "1.0.0", inputs.files[0],
                            extras=extra if order == "after" else (),
                            first_extras=extra if order == "before" else ())
    inputs.rebuild(0, payload=payload)
    monkeypatch.setattr(collector, "publish", lambda *args: pytest.fail("hierarchy conflict reached publication"))
    with pytest.raises(collector.Refusal, match="hierarchy conflict"):
        collector.collect(**inputs.args())
    assert_no_output(inputs)


@pytest.mark.parametrize("boundary", ["complete-output", "raise-limit", "tar-expansion", "notice", "members"])
def test_bounds_refuse_before_output(inputs, boundary, monkeypatch):
    overrides = {}
    if boundary == "complete-output":
        overrides["max_bytes"] = sum(inputs.path(i).stat().st_size for i in range(2)) + inputs.lock.stat().st_size
    elif boundary == "raise-limit":
        overrides["max_bytes"] = collector.MAX_OUTPUT_BYTES + 1
    elif boundary == "tar-expansion":
        monkeypatch.setattr(collector, "MAX_TAR_BYTES", 512)
    elif boundary == "notice":
        monkeypatch.setattr(collector, "MAX_NOTICE_BYTES", 16)
    else:
        monkeypatch.setattr(collector, "MAX_MEMBERS", 2)
    with pytest.raises(collector.Refusal):
        collector.collect(**inputs.args(**overrides))
    assert_no_output(inputs)


@pytest.mark.parametrize("argument", ["lock", "cache", "scratch_root", "out"])
def test_protected_lexical_paths_are_refused_before_any_open(inputs, argument, monkeypatch):
    args = inputs.args(**{argument: inputs.root / "heldout" / "unobserved"})
    monkeypatch.setattr(collector.os, "open", lambda *args, **kwargs: pytest.fail("protected path caused filesystem open"))
    with pytest.raises(collector.Refusal, match="protected"):
        collector.collect(**args)


@pytest.mark.parametrize("kind", ["archive", "cache-ancestor", "output", "lock"])
def test_symlinks_are_not_followed(inputs, kind):
    args = inputs.args()
    if kind == "archive":
        target = inputs.root / "unread-target"
        inputs.path(0).rename(target)
        inputs.path(0).symlink_to(target)
    elif kind == "cache-ancestor":
        target = inputs.root / "unread-cache"
        inputs.cache.rename(target)
        inputs.cache.symlink_to(target, target_is_directory=True)
    elif kind == "output":
        target = inputs.root / "untouched-output"
        target.mkdir()
        inputs.out.symlink_to(target, target_is_directory=True)
    else:
        target = inputs.root / "unread-lock"
        inputs.lock.rename(target)
        inputs.lock.symlink_to(target)
    with pytest.raises((collector.Refusal, OSError)):
        collector.collect(**args)
    assert not list(inputs.root.glob(".cargo-originals-*"))
    if kind == "output":
        assert not list(target.iterdir())
    else:
        assert_no_output(inputs)


def test_nonempty_output_is_not_changed(inputs):
    inputs.out.mkdir()
    sentinel = inputs.out / "existing"
    sentinel.write_bytes(b"caller-owned")
    with pytest.raises(collector.Refusal, match="already exists"):
        collector.collect(**inputs.args())
    assert list(inputs.out.iterdir()) == [sentinel]
    assert sentinel.read_bytes() == b"caller-owned"
    assert not list(inputs.root.glob(".cargo-originals-*"))


def test_write_failure_cleans_only_owned_stage(inputs, monkeypatch):
    original = collector.write_file
    writes = []

    def fail_second(parent_fd, name, data):
        writes.append(name)
        if len(writes) == 2:
            raise OSError("synthetic disk full")
        return original(parent_fd, name, data)

    monkeypatch.setattr(collector, "write_file", fail_second)
    with pytest.raises(OSError, match="disk full"):
        collector.collect(**inputs.args())
    assert len(writes) == 2
    assert_no_output(inputs)


@pytest.mark.parametrize("failure", ["io", "restrictive-umask"])
def test_first_stage_open_failure_cleans_owned_empty_directory(inputs, monkeypatch, failure):
    original = collector.os.open
    attempted = []

    def fail_stage_open(path, flags, *args, **kwargs):
        if path == inputs.out.name and flags & os.O_DIRECTORY and "dir_fd" in kwargs:
            attempted.append(path)
            raise OSError(errno.EACCES if failure == "restrictive-umask" else errno.EIO,
                          "synthetic first stage open failure")
        return original(path, flags, *args, **kwargs)

    args = inputs.args()
    monkeypatch.setattr(collector.os, "open", fail_stage_open)
    previous_umask = os.umask(0o777) if failure == "restrictive-umask" else None
    try:
        with pytest.raises(OSError, match="first stage open failure"):
            collector.collect(**args)
    finally:
        if previous_umask is not None:
            os.umask(previous_umask)
    assert attempted == [inputs.out.name]
    assert_no_output(inputs)


@pytest.mark.parametrize("failure", ["public-mode", "owner", "outside-child"])
def test_trusted_scratch_requirements(inputs, failure, monkeypatch):
    args = inputs.args()
    if failure == "public-mode":
        inputs.root.chmod(0o755)
    elif failure == "owner":
        monkeypatch.setattr(collector.os, "geteuid", lambda: os.stat(inputs.root).st_uid + 1)
    else:
        args["out"] = inputs.root / "nested" / "out"
    with pytest.raises(collector.Refusal):
        collector.collect(**args)
    assert_no_output(inputs)


@pytest.mark.parametrize("replacement", ["directory", "symlink", "moved-only"])
def test_relocated_stage_or_replacement_is_never_removed(inputs, monkeypatch, replacement):
    moved = inputs.root / "moved-owned-output"
    sentinel = inputs.root / "unrelated"
    sentinel.mkdir()
    (sentinel / "preserve").write_bytes(b"unrelated caller data")

    def relocate_then_fail(*args):
        inputs.out.rename(moved)
        if replacement == "directory":
            inputs.out.mkdir()
            (inputs.out / "preserve").write_bytes(b"replacement")
        elif replacement == "symlink":
            inputs.out.symlink_to(sentinel, target_is_directory=True)
        raise OSError("synthetic write failure after same-UID relocation")

    monkeypatch.setattr(collector, "write_file", relocate_then_fail)
    with pytest.raises(collector.Refusal, match="cleanup refused"):
        collector.collect(**inputs.args())
    assert moved.is_dir()
    assert (sentinel / "preserve").read_bytes() == b"unrelated caller data"
    if replacement == "directory":
        assert (inputs.out / "preserve").read_bytes() == b"replacement"
    elif replacement == "symlink":
        assert inputs.out.is_symlink()


def test_cli_aggregate_success_and_refusal(inputs, capsys):
    args = inputs.args()
    cli = [part for key, value in args.items() for part in ("--" + key.replace("_", "-"), str(value))]
    assert collector.main(cli) == 0
    assert capsys.readouterr().out.startswith("OK: 2 original archive(s)")
    inputs.out.rename(inputs.root / "completed")
    inputs.path(0).unlink()
    assert collector.main(cli) == 2
    assert "missing or duplicate" in capsys.readouterr().err
    assert_no_output(inputs)
