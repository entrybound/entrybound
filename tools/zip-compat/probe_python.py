#!/usr/bin/env python3
"""Emit deterministic JSON observations for CPython's zipfile runtime."""

from __future__ import annotations

import hashlib
import json
import pathlib
import sys
import zipfile


def error_name(error: BaseException) -> str:
    return f"{type(error).__module__}.{type(error).__name__}"


def main() -> None:
    archive = pathlib.Path(sys.argv[1])
    result: dict[str, object] = {"runtime": f"zip/python-zipfile@{sys.version_info.major}.{sys.version_info.minor}.{sys.version_info.micro}"}
    try:
        with zipfile.ZipFile(archive) as source:
            infos = source.infolist()
            result["listing"] = [info.filename for info in infos]
            selected = []
            for name in dict.fromkeys(info.filename for info in infos):
                try:
                    info = source.getinfo(name)
                    data = source.read(info)
                    selected.append({"name": name, "length": len(data), "sha256": hashlib.sha256(data).hexdigest()})
                except Exception as error:  # probe must record runtime behavior
                    selected.append({"name": name, "error": error_name(error)})
            result["selected"] = selected
    except Exception as error:
        result["listing_error"] = error_name(error)
    print(json.dumps(result, sort_keys=True, separators=(",", ":")))


if __name__ == "__main__":
    main()
