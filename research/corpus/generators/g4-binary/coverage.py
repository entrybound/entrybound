#!/usr/bin/env python3
"""Report g4-binary coverage (family x split x scale), bytes and pin status from the committed
sources and fingerprint records (hashes/bytes/counts only; no item content is read)."""

import json
import sys
from collections import defaultdict
from pathlib import Path

CORPUS = Path(__file__).resolve().parents[2]
src = json.loads((CORPUS / "sources" / "g4-binary.json").read_text(encoding="utf-8"))
cells = defaultdict(list)
groups = defaultdict(set)
total = alloc = 0
missing = []
print(f"{'item':<50} {'fam':<4} {'split':<10} {'scale':<6} {'bytes':>14} {'alloc':>14} pin")
for it in src["items"]:
    rec_p = CORPUS / "fingerprints" / f"{it['item_id']}.json"
    rec = json.loads(rec_p.read_text(encoding="utf-8")) if rec_p.exists() else None
    key = (it["family"], it["split"], it["scale"])
    cells[key].append(it["item_id"])
    groups[(it["family"], it["split"])].add(it["independence_group"])
    if not rec:
        missing.append(it["item_id"])
        print(f"{it['item_id']:<50} {it['family']:<4} {it['split']:<10} {it['scale']:<6} {'-':>14} {'-':>14} NOT MATERIALIZED")
        continue
    fp = rec["fingerprint"]
    b = fp["bytes"]
    a = fp.get("allocated_bytes", 0) or 0
    total += b
    alloc += a
    warn = " (fits smaller tier)" if rec["scale_check"].get("fits_smaller_tier") else ""
    print(f"{it['item_id']:<50} {it['family']:<4} {it['split']:<10} {it['scale']:<6} {b:>14,} {a:>14,} "
          f"{rec['output_pin']['status']}{warn}")
print(f"\ntotal apparent bytes {total:,} ({total / 2**30:.2f} GiB); allocated {alloc:,} ({alloc / 2**30:.2f} GiB)")
print("\ncoverage (family: split -> scale counts [independence groups])")
for fam in ("F09", "F10", "F11", "F14", "F16"):
    parts = []
    for split in ("tuning", "validation", "heldout"):
        sc = {s: len(cells[(fam, split, s)]) for s in ("small", "medium", "large") if cells[(fam, split, s)]}
        parts.append(f"{split}: {sc} groups={len(groups[(fam, split)])}")
    print(f"  {fam}: " + " | ".join(parts))
if missing:
    print("\nnot materialized:", ", ".join(missing))
    sys.exit(1)
