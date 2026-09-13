#!/usr/bin/env python3
"""Cross-check the vectorised gear-norm-v1 reference chunker embedded in the g2-generated F20 generators
against the real entrybound::chunker implementation (Rust, built by cdc_verify.sh).

usage (inside WSL, via cdc_verify.sh):
  cdc_verify.py --rust-bin BIN [--out results.json] PATH...   (files or directories; held-out paths refused)

For every regular file and every v2 policy, compares the full chunk-length sequence produced by
entrybound::chunker::chunk_ranges with the Python reference in gear_extremes.py (the reference is
kept byte-identical in entropy_sparse_dup.py and gear_straddle.py; this script checks that too).
Exit status 1 on any mismatch.  Records only chunk counts and length run-length summaries.
"""

import argparse
import hashlib
import importlib.util
import json
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "..", "..", "tools"))
import corpuslib  # noqa: E402

BEGIN = "# gear-norm-v1 reference (kept byte-identical"
END = "# ---------------------------------------------------------------------------------------------\n\n\nclass Shake"


def reference_block(path):
    s = open(path, encoding="utf-8").read()
    i = s.index(BEGIN)
    j = s.index("# ----", s.index("def trigger(", i))
    return s[i:j]


def rle(lengths):
    out = []
    for x in lengths:
        if out and out[-1][0] == x:
            out[-1][1] += 1
        else:
            out.append([x, 1])
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--rust-bin", required=True)
    ap.add_argument("--out")
    ap.add_argument("paths", nargs="+")
    args = ap.parse_args()
    blocks = {n: reference_block(os.path.join(HERE, n)) for n in ("gear_extremes.py", "entropy_sparse_dup.py", "gear_straddle.py")
              if os.path.exists(os.path.join(HERE, n))}
    digests = {n: hashlib.sha256(b.encode()).hexdigest() for n, b in blocks.items()}
    if len(set(digests.values())) != 1:
        raise SystemExit(f"reference chunker blocks differ between generators: {digests}")
    spec = importlib.util.spec_from_file_location("gear_extremes", os.path.join(HERE, "gear_extremes.py"))
    ref = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(ref)

    files = []
    for p in args.paths:
        corpuslib.assert_not_heldout(p)
        if os.path.isdir(p):
            for r, dn, fn in os.walk(p):
                dn.sort()
                for n in sorted(fn):
                    fp = os.path.join(r, n)
                    if os.path.isfile(fp) and not os.path.islink(fp):
                        files.append(fp)
        else:
            files.append(p)
    results = {"reference_block_sha256": next(iter(digests.values())), "files": [], "mismatches": 0}
    batch = 16
    for k in range(0, len(files), batch):
        group = files[k:k + batch]
        r = subprocess.run([args.rust_bin] + group, capture_output=True, text=True, check=True)
        rust = {}
        for line in r.stdout.splitlines():
            o = json.loads(line)
            rust[(o["file"], o["policy"])] = o
        for fp in group:
            with open(fp, "rb") as f:
                data = f.read()
            pos, lv = ref.candidates(data)
            entry = {"file": fp, "bytes": len(data), "policies": {}}
            for pol in ref.POLICIES:
                py = rle(ref.chunk_lengths(len(data), pos, lv, pol)) if data else []
                ru = rust[(fp, pol[0])]["rle"]
                ok = [list(x) for x in ru] == py
                entry["policies"][pol[0]] = {"chunks": sum(c for _, c in py), "match": ok,
                                             "distinct_lengths": len({x for x, _ in py}),
                                             "rle_head": py[:3]}
                if not ok:
                    results["mismatches"] += 1
                    print(f"MISMATCH {fp} {pol[0]}: rust {ru[:4]} python {py[:4]}", file=sys.stderr)
            results["files"].append(entry)
    if args.out:
        corpuslib.write_json_atomic(args.out, results)
    print(json.dumps({"files": len(files), "mismatches": results["mismatches"]}))
    return 1 if results["mismatches"] else 0


if __name__ == "__main__":
    sys.exit(main())
