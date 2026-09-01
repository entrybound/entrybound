#!/usr/bin/env python3
"""Development-only strict-tar differential probe.

This program generates its inputs and asks the local Python tarfile and system
tar implementations to list/read them. Production Entrybound never executes a
legacy runtime. Run with `python tools/tar-strict/probe.py` and compare the JSON
with observed-outcomes-v1.json before intentionally updating that evidence.
"""

from __future__ import annotations

import hashlib
import io
import json
import pathlib
import platform
import subprocess
import tarfile
import tempfile


def octal(field: memoryview, value: int) -> None:
    text = f"{value:o}".encode()
    field[:] = b"0" * len(field)
    field[len(field) - 1 - len(text) : len(field) - 1] = text
    field[-1] = 0


def member(name: bytes, content: bytes) -> bytes:
    header = bytearray(512)
    header[: len(name)] = name
    view = memoryview(header)
    octal(view[100:108], 0o100644)
    octal(view[108:116], 0)
    octal(view[116:124], 0)
    octal(view[124:136], len(content))
    octal(view[136:148], 1_700_000_000)
    header[148:156] = b"        "
    header[156] = ord("0")
    header[257:263] = b"ustar\0"
    header[263:265] = b"00"
    header[148:156] = f"{sum(header):06o}\0 ".encode()
    padding = (-len(content)) % 512
    return bytes(header) + content + b"\0" * padding


def cases() -> dict[str, bytes]:
    end = b"\0" * 1024
    ordinary = member(b"file", b"ordinary") + end
    duplicate = member(b"dup", b"first") + member(b"dup", b"second") + end
    traversal = member(b"../outside", b"unsafe") + end
    bad_checksum = bytearray(ordinary)
    bad_checksum[0] ^= 1
    return {
        "ordinary": ordinary,
        "duplicate-name": duplicate,
        "path-traversal": traversal,
        "single-zero-end-block": ordinary[:-512],
        "nonzero-after-end": ordinary + b"BAD",
        "bad-checksum": bytes(bad_checksum),
    }


def probe_python(data: bytes) -> dict[str, object]:
    try:
        with tarfile.open(fileobj=io.BytesIO(data), mode="r:") as archive:
            result = []
            for item in archive.getmembers():
                content = archive.extractfile(item) if item.isfile() else None
                payload = content.read() if content is not None else b""
                result.append(
                    {
                        "name": item.name,
                        "size": item.size,
                        "sha256": hashlib.sha256(payload).hexdigest()
                        if content is not None
                        else None,
                    }
                )
            return {"outcome": "accepted", "entries": result}
    except Exception as error:  # exact runtime behavior is the evidence
        return {"outcome": "refused", "error": type(error).__name__}


def probe_system_tar(path: pathlib.Path) -> dict[str, object]:
    process = subprocess.run(
        ["tar", "-tf", str(path)], capture_output=True, text=True, check=False
    )
    return {
        "outcome": "accepted" if process.returncode == 0 else "refused",
        "returncode": process.returncode,
        "listing": process.stdout.splitlines(),
        "stderr": process.stderr.strip(),
    }


def main() -> None:
    try:
        version = subprocess.run(
            ["tar", "--version"], capture_output=True, text=True, check=False
        ).stdout.splitlines()[0]
    except (FileNotFoundError, IndexError):
        version = "unavailable"
    output: dict[str, object] = {
        "schema": "entrybound/tar-differential-evidence/v1",
        "platform": platform.platform(),
        "runtimes": {
            "python-tarfile": platform.python_version(),
            "system-tar": version,
        },
        "cases": {},
    }
    with tempfile.TemporaryDirectory(
        prefix="entrybound-tar-probe-", dir=pathlib.Path(__file__).parent
    ) as directory:
        root = pathlib.Path(directory)
        for name, data in cases().items():
            path = root / f"{name}.tar"
            path.write_bytes(data)
            output["cases"][name] = {
                "source_sha256": hashlib.sha256(data).hexdigest(),
                "python-tarfile": probe_python(data),
                "system-tar": probe_system_tar(path),
            }
    print(json.dumps(output, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
