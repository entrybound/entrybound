#!/usr/bin/env python3
"""Collect pinned original crates.io archives offline; never build or fetch packages.

Linux/WSL only. All input, archive, license and size validation precedes output
creation. The caller must exclusively control private D-backed WSL scratch,
supply an unrestricted lock and use an absent output within that scratch.
"""

from __future__ import annotations

import argparse
import contextlib
import errno
import gzip
import hashlib
import io
import json
import os
import re
import shutil
import stat
import sys
import tarfile
import tomllib
import zlib
from pathlib import Path, PurePosixPath

MAX_OUTPUT_BYTES = 64 * 1024 * 1024
MAX_LOCK_BYTES = 1024 * 1024
MAX_TAR_BYTES = 128 * 1024 * 1024
MAX_NOTICE_BYTES = 2 * 1024 * 1024
MAX_ALL_NOTICE_BYTES = 8 * 1024 * 1024
MAX_MEMBERS = 100_000
FIXED_MTIME = 1767225600
REGISTRY = "registry+https://github.com/rust-lang/crates.io-index"
NAME_RE = re.compile(r"^[a-z0-9][a-z0-9_-]{0,63}$")
VERSION_RE = re.compile(r"^[0-9]+\.[0-9]+\.[0-9]+(?:[-+][0-9A-Za-z]+(?:[.-][0-9A-Za-z]+)*)*$")
HEX64_RE = re.compile(r"^[0-9a-f]{64}$")
NOTICE_RE = re.compile(r"^(?:license|licence|copying|copyright|notice|unlicense)", re.I)


class Refusal(Exception):
    """Invalid or unsupported input; no acquisition fallback is allowed."""


def digest(data):
    return hashlib.sha256(data).hexdigest()


def canonical(value):
    return (json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True) + "\n").encode()


def safe_path(value):
    path = Path(value)
    # Reject before filesystem inspection, including the standard relocated
    # heldout component. This is a refusal, never an authorization mechanism.
    if any(part in ("..",) or part.casefold() == "heldout" for part in path.parts):
        raise Refusal("protected or traversing paths are not permitted")
    return Path(os.path.abspath(path))


@contextlib.contextmanager
def directory_fd(path):
    path = safe_path(path)
    fd = os.open(path.anchor, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
    try:
        for part in path.parts[1:]:
            child = os.open(part, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW, dir_fd=fd)
            os.close(fd)
            fd = child
        yield fd
    finally:
        os.close(fd)


def read_fd(fd, limit):
    info = os.fstat(fd)
    if not stat.S_ISREG(info.st_mode) or info.st_size > limit:
        raise Refusal("input must be a regular file within its byte limit")
    chunks, size = [], 0
    while chunk := os.read(fd, min(1024 * 1024, limit - size + 1)):
        size += len(chunk)
        if size > limit:
            raise Refusal("input grew beyond its byte limit")
        chunks.append(chunk)
    return b"".join(chunks)


def read_regular(path, limit):
    path = safe_path(path)
    with directory_fd(path.parent) as parent:
        fd = os.open(path.name, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK, dir_fd=parent)
        try:
            return read_fd(fd, limit)
        finally:
            os.close(fd)


def registry_packages(lock_bytes, expected_count):
    try:
        doc = tomllib.loads(lock_bytes.decode("utf-8"))
    except (UnicodeError, tomllib.TOMLDecodeError) as exc:
        raise Refusal("malformed Cargo.lock") from exc
    if not isinstance(doc.get("package"), list):
        raise Refusal("Cargo.lock must contain a package list")
    packages, seen = [], set()
    for package in doc["package"]:
        if not isinstance(package, dict):
            raise Refusal("Cargo.lock package must be an object")
        source = package.get("source")
        if source is not None and not isinstance(source, str):
            raise Refusal("Cargo.lock source must be a string")
        if not source or not source.startswith("registry+"):
            continue
        name, version, checksum = (package.get(key) for key in ("name", "version", "checksum"))
        if source != REGISTRY:
            raise Refusal("only the pinned crates.io registry is supported")
        if not isinstance(name, str) or not NAME_RE.fullmatch(name):
            raise Refusal("unsafe or unsupported registry package name")
        if not isinstance(version, str) or not VERSION_RE.fullmatch(version):
            raise Refusal("unsafe or unsupported registry package version")
        if not isinstance(checksum, str) or not HEX64_RE.fullmatch(checksum):
            raise Refusal("registry package requires a lowercase SHA256 checksum")
        if (name, version) in seen:
            raise Refusal("duplicate registry package identity")
        seen.add((name, version))
        packages.append({"name": name, "version": version, "checksum": checksum, "source": source})
    if len(packages) != expected_count:
        raise Refusal("registry package count does not match the caller pin")
    return sorted(packages, key=lambda p: (p["name"], p["version"]))


def cached_archive(cache_fd, filename, limit):
    matches = []
    for shard in sorted(os.listdir(cache_fd)):
        info = os.stat(shard, dir_fd=cache_fd, follow_symlinks=False)
        if stat.S_ISLNK(info.st_mode):
            raise Refusal("cache registry directories must not be symlinks")
        if not stat.S_ISDIR(info.st_mode):
            continue
        shard_fd = os.open(shard, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW, dir_fd=cache_fd)
        try:
            try:
                os.stat(filename, dir_fd=shard_fd, follow_symlinks=False)
            except FileNotFoundError:
                continue
            matches.append(shard)
        finally:
            os.close(shard_fd)
    if len(matches) != 1:
        raise Refusal("missing or duplicate cached registry archive")
    shard_fd = os.open(matches[0], os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW, dir_fd=cache_fd)
    try:
        fd = os.open(filename, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK, dir_fd=shard_fd)
        try:
            return read_fd(fd, limit)
        finally:
            os.close(fd)
    finally:
        os.close(shard_fd)


def license_markers(text):
    text = " ".join(text.casefold().split())
    tests = {
        "MIT": "permission is hereby granted, free of charge" in text
               and "copyright notice and this permission notice" in text,
        "Apache-2.0": "apache license" in text and "version 2.0" in text
                      and "terms and conditions" in text,
        "Unlicense": "this is free and unencumbered software released into the public domain" in text,
        "BSD-3-Clause": "redistribution and use in source and binary forms" in text
                        and "neither the name" in text and "redistributions of source code must retain" in text,
        "BSL-1.0": "boost software license - version 1.0" in text
                   and "must be included in all copies of the software" in text,
        "Unicode-DFS-2016": "unicode" in text and "permission is hereby granted" in text
                            and "data files" in text and "copyright" in text,
    }
    return sorted(key for key, present in tests.items() if present)


def selected_license_clauses(expression, available):
    # These are the exact declarations of the pinned source population. Unknown
    # declarations require a reviewed policy; a keyword is not a license grant.
    alternatives = {
        "MIT": [("MIT",)],
        "MIT OR Apache-2.0": [("MIT",), ("Apache-2.0",)],
        "Unlicense OR MIT": [("MIT",), ("Unlicense",)],
        "MIT/Apache-2.0": [("MIT", "Apache-2.0")],
        "Unlicense/MIT": [("Unlicense", "MIT")],
        "Apache-2.0 OR BSL-1.0": [("BSL-1.0",), ("Apache-2.0",)],
        "(Apache-2.0 OR MIT) AND BSD-3-Clause": [("MIT", "BSD-3-Clause"), ("Apache-2.0", "BSD-3-Clause")],
        "(MIT OR Apache-2.0) AND Unicode-DFS-2016": [("MIT", "Unicode-DFS-2016"),
                                                  ("Apache-2.0", "Unicode-DFS-2016")],
    }
    if not isinstance(expression, str) or expression not in alternatives:
        raise Refusal("missing or unsupported package license expression")
    for clauses in alternatives[expression]:
        if set(clauses) <= available:
            return list(clauses)
    raise Refusal("required package license text evidence is missing")


def inspect_archive(payload, package):
    expanded = io.BytesIO()
    try:
        with gzip.GzipFile(fileobj=io.BytesIO(payload)) as compressed:
            while chunk := compressed.read(1024 * 1024):
                if expanded.tell() + len(chunk) > MAX_TAR_BYTES:
                    raise Refusal("archive expansion exceeds its byte limit")
                expanded.write(chunk)
        expanded.seek(0)
        with tarfile.open(fileobj=expanded, mode="r:") as archive:
            members = []
            for member in archive:
                if len(members) >= MAX_MEMBERS:
                    raise Refusal("archive has too many members")
                members.append(member)
            prefix = package["name"] + "-" + package["version"]
            seen, notices, available, metadata, notice_bytes = set(), [], set(), None, 0
            file_paths, directory_paths = set(), {prefix}
            for member in members:
                # A directory may have one conventional trailing slash. Every
                # other component must be canonical: raw aliases must not
                # bypass duplicate detection after POSIX path normalization.
                name = member.name[:-1] if member.isdir() and member.name.endswith("/") else member.name
                parts = PurePosixPath(name).parts
                if not name or "\\" in name or "\x00" in name or name.startswith("/") \
                        or any(part in ("", ".", "..") for part in name.split("/")) \
                        or not parts or parts[0] != prefix:
                    raise Refusal("unsafe or wrong-root archive member")
                if name in seen:
                    raise Refusal("duplicate archive member")
                seen.add(name)
                if not (member.isfile() or member.isdir()):
                    raise Refusal("archive links and special files are not permitted")
                ancestors = {"/".join(parts[:i]) for i in range(1, len(parts))}
                if ancestors & file_paths or (member.isfile() and name in directory_paths):
                    raise Refusal("archive file/directory hierarchy conflict")
                directory_paths.update(ancestors)
                (file_paths if member.isfile() else directory_paths).add(name)
                relative = name[len(prefix) + 1:]
                if member.isfile() and relative == "Cargo.toml":
                    if member.size > MAX_LOCK_BYTES:
                        raise Refusal("archive Cargo.toml exceeds its byte limit")
                    metadata = archive.extractfile(member).read()
                if member.isfile() and NOTICE_RE.match(PurePosixPath(relative).name):
                    if member.size > MAX_NOTICE_BYTES or notice_bytes + member.size > MAX_ALL_NOTICE_BYTES:
                        raise Refusal("archive license/notice bytes exceed their limit")
                    data = archive.extractfile(member).read()
                    notice_bytes += len(data)
                    markers = license_markers(data.decode("utf-8", errors="replace"))
                    notices.append({"path": relative, "bytes": len(data), "sha256": digest(data),
                                    "text_markers": markers})
                    if "/" not in relative:
                        available.update(markers)
            # Cargo archives may have zero tar padding; hidden trailing bytes
            # after the parsed member set are not accepted as inspected content.
            expanded.seek(archive.offset)
            while chunk := expanded.read(1024 * 1024):
                if any(chunk):
                    raise Refusal("nonzero trailing tar data")
    except (gzip.BadGzipFile, EOFError, tarfile.TarError, zlib.error, OSError) as exc:
        raise Refusal("malformed gzip/tar registry archive") from exc
    if metadata is None:
        raise Refusal("archive Cargo.toml is missing")
    try:
        document = tomllib.loads(metadata.decode("utf-8"))
    except (UnicodeError, tomllib.TOMLDecodeError) as exc:
        raise Refusal("malformed archive Cargo.toml") from exc
    meta = document.get("package")
    if not isinstance(meta, dict) or meta.get("name") != package["name"] or meta.get("version") != package["version"]:
        raise Refusal("archive package name/version does not match Cargo.lock")
    clauses = selected_license_clauses(meta.get("license"), available)
    return {"cargo_toml_sha256": digest(metadata), "license_expression": meta["license"],
            "selected_license_text_clauses": clauses, "license_notices": sorted(notices, key=lambda n: n["path"])}


def output_available(parent_fd, name):
    try:
        os.stat(name, dir_fd=parent_fd, follow_symlinks=False)
    except FileNotFoundError:
        return
    raise Refusal("output already exists; an absent caller output is required")


def private_scratch(fd):
    info = os.fstat(fd)
    if info.st_uid != os.geteuid() or stat.S_IMODE(info.st_mode) != 0o700:
        raise Refusal("scratch root must be caller-owned with mode 0700")


def cleanup_owned(parent_fd, name, identity):
    # No-follow blocks links, not same-UID directory relocation. Never delete
    # a replacement entry or assume a moved stage remains at the caller path.
    try:
        info = os.stat(name, dir_fd=parent_fd, follow_symlinks=False)
    except FileNotFoundError as exc:
        raise Refusal("owned output moved; cleanup refused") from exc
    if not stat.S_ISDIR(info.st_mode) or (info.st_dev, info.st_ino, info.st_uid) != identity:
        raise Refusal("owned output identity changed; cleanup refused")
    # An unopened stage is empty. rmdir also handles a restrictive umask that
    # prevents opening it; removing an entry requires its parent's permissions.
    try:
        os.rmdir(name, dir_fd=parent_fd)
        return
    except OSError as exc:
        if exc.errno not in (errno.ENOTEMPTY, errno.EEXIST):
            raise
    shutil.rmtree(name, dir_fd=parent_fd)


def write_file(parent_fd, name, data):
    fd = os.open(name, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o644, dir_fd=parent_fd)
    try:
        with os.fdopen(fd, "wb", closefd=False) as stream:
            stream.write(data)
            stream.flush()
            os.fsync(fd)
        os.fchmod(fd, 0o644)
        os.utime(fd, (FIXED_MTIME, FIXED_MTIME))
    finally:
        os.close(fd)


def publish(parent_fd, output_name, archives, lock_bytes, manifest_bytes):
    output_available(parent_fd, output_name)
    # mkdir is exclusive: it never reuses or replaces a caller directory. The
    # private parent must remain exclusively controlled throughout the call.
    os.mkdir(output_name, 0o700, dir_fd=parent_fd)
    complete, identity = False, None
    try:
        info = os.stat(output_name, dir_fd=parent_fd, follow_symlinks=False)
        if not stat.S_ISDIR(info.st_mode) or info.st_uid != os.geteuid():
            raise Refusal("new output ownership changed; cleanup refused")
        identity = (info.st_dev, info.st_ino, info.st_uid)
        stage_fd = os.open(output_name, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW, dir_fd=parent_fd)
        try:
            info = os.fstat(stage_fd)
            if (info.st_dev, info.st_ino, info.st_uid) != identity:
                raise Refusal("new output identity changed; cleanup refused")
            os.mkdir("packages", 0o755, dir_fd=stage_fd)
            packages_fd = os.open("packages", os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW, dir_fd=stage_fd)
            try:
                for name, payload in archives:
                    write_file(packages_fd, name, payload)
                os.fchmod(packages_fd, 0o755)
                os.utime(packages_fd, (FIXED_MTIME, FIXED_MTIME))
            finally:
                os.close(packages_fd)
            write_file(stage_fd, "Cargo.lock", lock_bytes)
            write_file(stage_fd, "provenance-notices.json", manifest_bytes)
            os.fchmod(stage_fd, 0o755)
            os.utime(stage_fd, (FIXED_MTIME, FIXED_MTIME))
        finally:
            os.close(stage_fd)
        complete = True
    finally:
        if not complete and identity is not None:
            cleanup_owned(parent_fd, output_name, identity)


def collect(*, lock, lock_sha256, cache, scratch_root, out, source_commit, source_split, expected_count,
            max_bytes=MAX_OUTPUT_BYTES):
    if sys.platform != "linux":
        raise Refusal("collector requires Linux/WSL")
    if source_split not in ("tuning", "validation") or os.environ.get("EB_SPLIT", "tuning") not in ("tuning", "validation"):
        raise Refusal("only unrestricted tuning/validation inputs are permitted")
    if not isinstance(max_bytes, int) or isinstance(max_bytes, bool) or not 0 < max_bytes <= MAX_OUTPUT_BYTES:
        raise Refusal("output byte limit must be positive and at most 64 MiB")
    if not isinstance(expected_count, int) or isinstance(expected_count, bool) or expected_count < 1:
        raise Refusal("expected registry package count must be positive")
    if not isinstance(lock_sha256, str) or not HEX64_RE.fullmatch(lock_sha256):
        raise Refusal("expected Cargo.lock SHA256 is required")
    if not isinstance(source_commit, str) or not re.fullmatch(r"[0-9a-f]{40}", source_commit):
        raise Refusal("source commit must be a full lowercase 40-hex pin")
    lock, cache, scratch_root, out = (safe_path(path) for path in (lock, cache, scratch_root, out))
    if out.parent != scratch_root:
        raise Refusal("output must be a direct child of the private scratch root")
    lock_bytes = read_regular(lock, MAX_LOCK_BYTES)
    if digest(lock_bytes) != lock_sha256:
        raise Refusal("Cargo.lock checksum does not match the caller pin")
    packages = registry_packages(lock_bytes, expected_count)
    archives, rows, archive_bytes, all_notices = [], [], 0, 0
    with directory_fd(scratch_root) as parent_fd, directory_fd(cache) as cache_fd:
        private_scratch(parent_fd)
        output_available(parent_fd, out.name)
        for package in packages:
            filename = package["name"] + "-" + package["version"] + ".crate"
            payload = cached_archive(cache_fd, filename, max_bytes - archive_bytes)
            if digest(payload) != package["checksum"]:
                raise Refusal("cached archive checksum does not match Cargo.lock")
            notice_metadata = inspect_archive(payload, package)
            all_notices += sum(n["bytes"] for n in notice_metadata["license_notices"])
            if all_notices > MAX_ALL_NOTICE_BYTES:
                raise Refusal("collection license/notice bytes exceed their limit")
            archive_bytes += len(payload)
            archives.append((filename, payload))
            rows.append({"name": package["name"], "version": package["version"],
                         "registry_source": package["source"], "archive_path": "packages/" + filename,
                         "published_url": "https://static.crates.io/crates/" + package["name"] + "/" + filename,
                         "archive_sha256": digest(payload), "archive_bytes": len(payload), **notice_metadata})
        manifest = {"schema": "ebrc-cargo-originals-v1", "source_split": source_split,
                    "source_commit": source_commit, "lock_sha256": lock_sha256,
                    "tool_sha256": digest(Path(__file__).read_bytes()), "python_version": sys.version.split()[0],
                    "archive_count": len(rows), "archive_bytes": archive_bytes,
                    "license_evidence_scope": "embedded text hashes; no redistribution or corpus acceptance claim",
                    "packages": rows}
        manifest_bytes = canonical(manifest)
        if archive_bytes + len(lock_bytes) + len(manifest_bytes) > max_bytes:
            raise Refusal("complete output exceeds the caller byte limit")
        publish(parent_fd, out.name, archives, lock_bytes, manifest_bytes)
    return manifest


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("lock", "lock-sha256", "cache", "scratch-root", "out", "source-commit"):
        parser.add_argument("--" + name, required=True)
    parser.add_argument("--source-split", required=True, choices=("tuning", "validation"))
    parser.add_argument("--expected-count", required=True, type=int)
    parser.add_argument("--max-bytes", type=int, default=MAX_OUTPUT_BYTES)
    args = parser.parse_args(argv)
    try:
        result = collect(**vars(args))
    except (Refusal, OSError) as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 2
    print(f"OK: {result['archive_count']} original archive(s), {result['archive_bytes']} archive bytes")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
