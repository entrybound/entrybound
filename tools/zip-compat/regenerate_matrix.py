#!/usr/bin/env python3
"""Regenerate the checked ZIP runtime outcome matrix.

This development tool intentionally executes foreign runtimes. The Entrybound
library and CLI do not import or invoke it.
"""

from __future__ import annotations

import hashlib
import json
import pathlib
import platform
import shutil
import subprocess
import sys
import tempfile

from generate_corpus import cases


ROOT = pathlib.Path(__file__).resolve().parents[2]
TOOLS = pathlib.Path(__file__).resolve().parent
CORPUS = ROOT / "target" / "zip-compat-corpus"
JAVA_CLASSES = ROOT / "target" / "zip-compat-probe"
OUTPUT = TOOLS / "observed-outcomes-v1.json"


def run(command: list[str], *, cwd: pathlib.Path | None = None) -> subprocess.CompletedProcess[str]:
    return subprocess.run(command, cwd=cwd, text=True, capture_output=True, check=False)


def json_probe(command: list[str]) -> dict[str, object]:
    result = run(command)
    if result.returncode == 0:
        return json.loads(result.stdout)
    return {
        "process_error": result.stderr.strip().splitlines()[-1] if result.stderr.strip() else "nonzero exit",
        "exit_code": result.returncode,
    }


def probe_libarchive(archive: pathlib.Path) -> dict[str, object]:
    listing = run(["tar", "-tf", str(archive)])
    result: dict[str, object] = {
        "runtime": "zip/libarchive-bsdtar@3.8.8",
        "listing": listing.stdout.splitlines() if listing.returncode == 0 else [],
    }
    if listing.returncode != 0:
        result["listing_error"] = listing.stderr.strip().splitlines()[-1]
        return result
    selected = []
    for name in dict.fromkeys(result["listing"]):
        content = subprocess.run(
            ["tar", "-xOf", str(archive), str(name)],
            capture_output=True,
            check=False,
        )
        if content.returncode == 0:
            selected.append(
                {
                    "name": name,
                    "length": len(content.stdout),
                    "sha256": hashlib.sha256(content.stdout).hexdigest(),
                }
            )
        else:
            selected.append({"name": name, "error": content.stderr.decode(errors="replace").strip().splitlines()[-1]})
    result["selected"] = selected
    return result


def main() -> None:
    python_version = f"{sys.version_info.major}.{sys.version_info.minor}.{sys.version_info.micro}"
    if python_version != "3.13.5":
        raise SystemExit(f"matrix v1 requires CPython 3.13.5, found {python_version}")
    java_version = run(["java", "-version"]).stderr.splitlines()[0]
    if '"21.0.12.1"' not in java_version:
        raise SystemExit(f"matrix v1 requires Temurin/OpenJDK 21.0.12.1, found {java_version}")
    tar_version = run(["tar", "--version"]).stdout.splitlines()[0]
    if "bsdtar 3.8.8" not in tar_version:
        raise SystemExit(f"matrix v1 requires bsdtar/libarchive 3.8.8, found {tar_version}")

    CORPUS.mkdir(parents=True, exist_ok=True)
    for name, data in cases().items():
        (CORPUS / f"{name}.zip").write_bytes(data)
    JAVA_CLASSES.mkdir(parents=True, exist_ok=True)
    compiled = run(["javac", "-d", str(JAVA_CLASSES), str(TOOLS / "ZipProbe.java")])
    if compiled.returncode != 0:
        raise SystemExit(compiled.stderr)

    matrix: dict[str, object] = {
        "matrix_version": "zip-observed-outcomes/v1",
        "platform": platform.platform(),
        "runtimes": [
            {"profile_id": "zip/python-zipfile@3.13.5", "version": sys.version.splitlines()[0]},
            {"profile_id": "zip/java-zipfile@21.0.12.1", "version": java_version},
            {"profile_id": "zip/java-zipinputstream@21.0.12.1", "version": java_version},
            {"profile_id": "zip/libarchive-bsdtar@3.8.8", "version": tar_version},
        ],
        "cases": {},
    }
    for name in sorted(cases()):
        archive = CORPUS / f"{name}.zip"
        matrix["cases"][name] = {
            "source_sha256": hashlib.sha256(archive.read_bytes()).hexdigest(),
            "python_zipfile": json_probe([sys.executable, str(TOOLS / "probe_python.py"), str(archive)]),
            "java_zipfile": json_probe(["java", "-cp", str(JAVA_CLASSES), "ZipProbe", "zipfile", str(archive)]),
            "java_zipinputstream": json_probe(["java", "-cp", str(JAVA_CLASSES), "ZipProbe", "zipinputstream", str(archive)]),
            "libarchive_bsdtar": probe_libarchive(archive),
        }
    OUTPUT.write_text(json.dumps(matrix, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(OUTPUT)


if __name__ == "__main__":
    main()
