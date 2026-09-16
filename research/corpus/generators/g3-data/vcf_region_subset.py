#!/usr/bin/env python3
"""Decompress a BGZF/gzip VCF and keep a contiguous genomic-position prefix (F07 run step).

A full chromosome VCF can decompress to far more than the 8 GiB large-tier cap (1000 Genomes
Project phase 3 chr22, for instance, decompresses from 205,612,353 B to 11,212,370,718 B). VCF
records are sorted by position, so a byte-bounded prefix of the data lines (everything after the
header) is exactly a contiguous genomic region starting at the first variant -- a real region
subset, not a synthetic one, produced with the stdlib gzip module only (no bcftools/tabix
required; BGZF is valid concatenated-member gzip, which gzip.open decompresses in full).

ebrc script contract:
    python vcf_region_subset.py --out <staging dir> --seed <int> --params <canonical JSON>
params:
  input          declared recipe input name holding the source .vcf.gz (default "vcf")
  output         output file name (default "<input stem>.region-subset.vcf")
  target_bytes   stop once the uncompressed prefix reaches this many bytes (default 4 GiB),
                 rounded down to the last complete data line
"""

import argparse
import gzip
import json
import os
import sys
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

    src = env_input_path(params.get("input", "vcf"))
    target_bytes = int(params.get("target_bytes", 4 * (1 << 30)))
    output_name = params.get("output") or (src.stem.split(".vcf")[0] + ".region-subset.vcf")
    dest = out / output_name

    header_lines = 0
    data_lines = 0
    first_pos = last_pos = None
    contig = None
    written = 0
    with gzip.open(src, "rb") as fin, open(dest, "wb") as fout:
        for raw in fin:
            if raw.startswith(b"#"):
                fout.write(raw)
                written += len(raw)
                header_lines += 1
                continue
            if written >= target_bytes:
                break
            fout.write(raw)
            written += len(raw)
            data_lines += 1
            fields = raw.split(b"\t", 2)
            if len(fields) >= 2:
                if contig is None:
                    contig = fields[0].decode("ascii", "replace")
                    first_pos = fields[1].decode("ascii", "replace")
                last_pos = fields[1].decode("ascii", "replace")

    (out / "region_subset_summary.json").write_text(json.dumps({
        "source_file": src.name, "header_lines": header_lines, "data_lines": data_lines,
        "contig": contig, "first_pos": first_pos, "last_pos": last_pos,
        "output_bytes": written, "target_bytes": target_bytes,
        "note": "prefix of the position-sorted VCF body: a real contiguous region from the start "
                "of the contig, truncated at the target byte budget (last complete record kept).",
    }, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"wrote {data_lines} data lines ({written} bytes), region {contig}:{first_pos}-{last_pos}",
          file=sys.stderr)
    if data_lines == 0:
        raise SystemExit("no data lines written")
    return 0


if __name__ == "__main__":
    sys.exit(main())
