#!/usr/bin/env python3
"""Extract a name-filtered subset of a zip archive's members (F05 run step).

Used when the full archive decompresses past the corpus's 8 GiB large-tier scale cap (e.g. the
Backblaze Drive Stats Q1 2025 zip is 1,020,483,699 B compressed but 10,891,076,206 B
uncompressed across 90 daily CSVs). Keeping a prefix-filtered subset of members is a real,
unmodified slice of the published data (real_or_generated stays "real"), not a derive.

ebrc script contract:
    python extract_zip_subset.py --out <staging dir> --seed <int> --params <canonical JSON>
params:
  input      declared recipe input name holding the zip archive (default "zip")
  prefixes   list of substrings; a member is kept if its name contains any of them
             (default: keep everything, i.e. equivalent to a full extract)
  strip_components   leading path components to drop from each kept member's output path
"""

import argparse
import json
import os
import sys
import zipfile
from pathlib import Path


def env_input_path(name: str) -> Path:
    var = "EB_INPUT_" + name.upper().replace("-", "_")
    p = os.environ.get(var)
    if not p or not Path(p).is_file():
        raise SystemExit(f"input {name!r} not available (env {var} = {p!r})")
    return Path(p)


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True)
    ap.add_argument("--seed", type=int, default=0)
    ap.add_argument("--params", default="{}")
    args = ap.parse_args()
    params = json.loads(args.params)
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)

    src = env_input_path(params.get("input", "zip"))
    prefixes = params.get("prefixes")
    strip = int(params.get("strip_components", 0))

    kept = 0
    kept_bytes = 0
    total_members = 0
    with zipfile.ZipFile(src) as z:
        for info in z.infolist():
            total_members += 1
            if info.is_dir():
                continue
            name = info.filename
            if prefixes is not None and not any(p in name for p in prefixes):
                continue
            parts = Path(name).parts
            rel = Path(*parts[strip:]) if strip and len(parts) > strip else Path(name)
            dest = out / rel
            dest.parent.mkdir(parents=True, exist_ok=True)
            with z.open(info) as fsrc, open(dest, "wb") as fdst:
                while True:
                    chunk = fsrc.read(1 << 20)
                    if not chunk:
                        break
                    fdst.write(chunk)
            os.chmod(dest, 0o644)
            kept += 1
            kept_bytes += info.file_size

    (out / "extract_subset_summary.json").write_text(json.dumps({
        "source_archive": src.name, "total_members_in_archive": total_members,
        "members_kept": kept, "kept_bytes": kept_bytes, "prefixes": prefixes,
    }, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"kept {kept}/{total_members} members ({kept_bytes} bytes)", file=sys.stderr)
    if kept == 0:
        raise SystemExit("no members matched the prefix filter")
    return 0


if __name__ == "__main__":
    main()
