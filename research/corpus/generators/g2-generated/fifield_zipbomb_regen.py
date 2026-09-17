#!/usr/bin/env python3
"""Build script (F20 structurally-adversarial archives, corpus group g2-generated): regenerate
David Fifield's overlapping-entry zip bombs from his public-domain generator script, rather than
vendoring pre-made bomb files (REQ-ECO-0174: regenerate, never vendor a zip bomb's compressed
bytes -- the technique, not one frozen artifact, is what the corpus should carry).

The generator (bamsoftware.com/hacks/zipbomb/zipbomb-20210121.zip, declared as recipe.inputs) is a
small Python 3 script implementing the "quoted overlapping" and "full overlapping" local-file-header
trick (many directory entries' data ranges overlap in the archive, and DEFLATE-quoted extra fields
let a single compressed block satisfy zip readers that trust either the local header or the central
directory) -- the same construction behind well-known bombs such as 42.zip and bamsoftware's own
zbxl.zip/zbsm.zip, at a much smaller, corpus-appropriate scale here.

Produces (sizes are the Makefile's own example parameters, scaled down where noted):
  overlap.zip      --mode=full_overlap, 26 single-letter entries, ~11 KiB
  quoted_small.zip  --mode=quoted_overlap, 250 entries, ~42 KiB, ~5.46 GiB claimed uncompressed
  quoted_extra.zip  --mode=quoted_overlap --extra=9999, 252 entries with extra-field quoting

Params (EB_PARAMS_JSON): {"zip_input": "<EB_INPUT_ key>"}
"""

import json
import os
import subprocess
import zipfile
from pathlib import Path


def main() -> None:
    params = json.loads(os.environ["EB_PARAMS_JSON"])
    out = Path(os.environ["EB_OUT"])
    scratch = Path(os.environ.get("EB_SCRATCH", "/tmp"))
    scratch.mkdir(parents=True, exist_ok=True)

    zip_key = "EB_INPUT_" + params["zip_input"].upper().replace("-", "_")
    src_zip = os.environ[zip_key]
    with zipfile.ZipFile(src_zip) as zf:
        zf.extractall(scratch)
    tool = next(scratch.glob("zipbomb-*/zipbomb"))
    tool.chmod(0o755)

    runs = [
        ("overlap.zip", ["--mode=full_overlap", "--alphabet", "ABCDEFGHIJKLMNOPQRSTUVWXYZ",
                        "--num-files=26", "--compressed-size=10000"]),
        ("quoted_small.zip", ["--mode=quoted_overlap", "--num-files=250", "--compressed-size=21179"]),
        ("quoted_extra.zip", ["--mode=quoted_overlap", "--num-files=252", "--compressed-size=21260",
                              "--extra=9999"]),
    ]
    for name, args in runs:
        dst = out / name
        with open(dst, "wb") as f:
            subprocess.run(["python3", str(tool), *args], stdout=f, check=True, cwd=scratch)
        os.chmod(dst, 0o644)

    (out / "PROVENANCE.txt").write_text(
        "Regenerated (not vendored) from David Fifield's public-domain zipbomb generator\n"
        "(https://www.bamsoftware.com/hacks/zipbomb/, zipbomb-20210121.zip). Each file here is a\n"
        "fresh run of that script with the parameters noted in each Makefile-style invocation below;\n"
        "no pre-made bomb bytes were downloaded or copied.\n\n"
        "overlap.zip:      zipbomb --mode=full_overlap --alphabet ABCDEFGHIJKLMNOPQRSTUVWXYZ "
        "--num-files=26 --compressed-size=10000\n"
        "quoted_small.zip: zipbomb --mode=quoted_overlap --num-files=250 --compressed-size=21179\n"
        "quoted_extra.zip: zipbomb --mode=quoted_overlap --num-files=252 --compressed-size=21260 "
        "--extra=9999\n",
        encoding="utf-8")
    os.chmod(out / "PROVENANCE.txt", 0o644)
    print("fifield_zipbomb_regen: done")


if __name__ == "__main__":
    main()
