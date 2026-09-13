#!/usr/bin/env python3
"""Assemble the corpus manifest, statistics, coverage table and held-out lock.

Writes (under research/corpus/):
  manifest.json       corpus_version ebrc-2026.09-v1; every defined item with provenance,
                      license, fingerprint reference and hashes, split/family/scale, status;
                      family x split x scale coverage; split audit; manifest_sha256
  statistics.json     per-item statistics for tuning+validation (from stats/<id>.json),
                      family x split aggregates; held-out = fingerprint-only with note
  coverage.md         the coverage table rendered as Markdown
  heldout-lock.json   held-out item ids and tree hashes (status draft|frozen)

manifest_sha256 = SHA-256 of the canonical JSON (sorted keys, no whitespace, UTF-8) of
manifest.json with the "manifest_sha256" key removed.  Outputs contain no timestamps
except the freeze metadata, so re-assembling an unchanged corpus is byte-identical.

Held-out lock:  --freeze sets status "frozen" (requires every held-out item materialized).
Once frozen, any change of the held-out set fails loudly unless --relock --reason "..." is
given; the previous set is kept in "history".
--verify-heldout re-fingerprints held-out items (hashes only) against the lock.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
sys.dont_write_bytecode = True
import corpuslib as cl  # noqa: E402
import fingerprint as fpm  # noqa: E402

MANIFEST_FORMAT = "ebrc-manifest-v1"
STATISTICS_FORMAT = "ebrc-statistics-v1"
LOCK_FORMAT = "ebrc-heldout-lock-v1"
HELDOUT_NOTE = ("Held-out items expose fingerprints only (hashes, bytes, basic counts). Content statistics are "
                "deferred until a recorded design-freeze commit unlocks them (see methodology.md).")


def provenance_summary(record: dict | None, item: dict) -> dict:
    r = item["recipe"]
    out: dict = {}
    if r.get("inputs"):
        prov = {p["name"]: p for p in (record or {}).get("provenance", {}).get("inputs", [])}
        out["inputs"] = [{"name": i["name"], "url": i["url"], "declared_sha256": i["sha256"],
                          "sha256": prov.get(i["name"], {}).get("sha256"), "size": prov.get(i["name"], {}).get("size"),
                          "retrieved_utc": prov.get(i["name"], {}).get("retrieved_utc")} for i in r["inputs"]]
    if r.get("git"):
        g = (record or {}).get("provenance", {}).get("git", {})
        out["git"] = {"repo": r["git"]["repo"], "commit": r["git"]["commit"], "subpaths": r["git"].get("subpaths"),
                      "archive_sha256": g.get("archive_sha256")}
    if r.get("docker"):
        out["docker"] = {"image": r["docker"]["image"], "platform": r["docker"].get("platform", "linux/amd64")}
    if r.get("from_items"):
        out["from_items"] = list(r["from_items"])
    scripts = []
    if r.get("generator"):
        scripts.append(("generator", r["generator"]))
    for st in r.get("steps", []) or []:
        if st.get("op") == "run":
            scripts.append(("run", st))
    if scripts:
        out["scripts"] = [{"role": role, "script": s["script"], "sha256": cl.sha256_file(cl.REPO_ROOT / s["script"])[0],
                           "interpreter": s.get("interpreter", "python"), "seed": s.get("seed"),
                           "params": s.get("params", {})} for role, s in scripts]
    if r.get("steps"):
        out["steps"] = [st["op"] for st in r["steps"]]
    return out


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--corpus-dir")
    ap.add_argument("--data-root")
    ap.add_argument("--require-all", action="store_true", help="fail if any item is not materialized/current")
    ap.add_argument("--require-stats", action="store_true", help="fail if tuning/validation stats are missing/stale")
    ap.add_argument("--freeze", action="store_true", help="freeze the held-out set")
    ap.add_argument("--relock", action="store_true", help="allow changing a frozen held-out set")
    ap.add_argument("--reason", help="required with --relock")
    ap.add_argument("--verify-heldout", action="store_true", help="re-fingerprint held-out items against the lock")
    ap.add_argument("--hash-jobs", type=int, default=4)
    args = ap.parse_args(argv)
    cl.require_linux()
    layout = cl.Layout(args.corpus_dir, args.data_root)
    items, errors, warnings = cl.load_sources(layout)
    if errors:
        for e in errors:
            print(f"ERROR: {e}")
        return 2
    lock_path = layout.corpus_dir / "heldout-lock.json"

    if args.verify_heldout:
        lock = cl.load_json_if_exists(lock_path)
        if not lock:
            print("ERROR: no heldout-lock.json")
            return 2
        bad = 0
        for ent in lock["items"]:
            item = items.get(ent["item_id"])
            path = layout.item_path(item, allow_heldout=True) if item else None
            if not path or not path.is_dir():
                print(f"[{ent['item_id']}] MISSING")
                bad += 1
                continue
            fp = fpm.heldout_safe(fpm.fingerprint_path(path, jobs=args.hash_jobs))
            ok = fp["logical_tree_sha256"] == ent["logical_tree_sha256"]
            print(f"[{ent['item_id']}] {'OK' if ok else 'HASH MISMATCH'} logical_tree_sha256 {fp['logical_tree_sha256']}")
            bad += 0 if ok else 1
        return 1 if bad else 0

    # ---- per item status -------------------------------------------------------------
    records: dict = {}
    for iid in sorted(items):
        records[iid] = cl.load_json_if_exists(layout.record_path(iid))
    problems = []
    manifest_items = []
    for iid in sorted(items, key=lambda k: (items[k]["family"], cl.SPLITS.index(items[k]["split"]),
                                            cl.SCALE_ORDER.index(items[k]["scale"]), k)):
        it = items[iid]
        rec = records[iid]
        status = "ok"
        if not rec:
            status = "not-materialized"
        else:
            deps = {d: (records.get(d) or {}).get("fingerprint", {}).get("logical_tree_sha256")
                    for d in it["recipe"].get("from_items", []) or []}
            if any(v is None for v in deps.values()):
                status = "stale"
            else:
                mk = cl.materialization_key(it, deps)["sha256"]
                if rec.get("materialization_key") != mk or rec.get("split") != it["split"]:
                    status = "stale"
                elif rec.get("item_def_sha256") != cl.item_def_sha256(it):
                    status = "stale-metadata"
            pins = cl.load_json_if_exists(layout.pins_path(iid)) or {}
            for inp in it["recipe"].get("inputs", []) or []:
                if inp["sha256"] == "TOFU" and inp["name"] not in (pins.get("inputs") or {}):
                    problems.append(f"{iid}: TOFU input {inp['name']} has no pin in {cl.repo_rel(layout.pins_path(iid))}")
            if it["split"] == "heldout" and "structure" in rec.get("fingerprint", {}):
                problems.append(f"{iid}: held-out record exposes structure statistics")
        if status != "ok" and args.require_all:
            problems.append(f"{iid}: {status}")
        fp = (rec or {}).get("fingerprint", {})
        entry = {
            "item_id": iid, "family": it["family"], "family_name": cl.FAMILIES[it["family"]],
            "split": it["split"], "scale": it["scale"], "kind": it["kind"],
            "real_or_generated": it["real_or_generated"], "independence_group": it["independence_group"],
            "description": it["description"], "license": it["license"],
            "source_file": it["_source_file"], "item_def_sha256": cl.item_def_sha256(it),
            "status": status,
            "provenance": provenance_summary(rec, it),
            "fingerprint_ref": cl.repo_rel(layout.record_path(iid)) if rec else None,
            "fingerprint_record_sha256": cl.sha256_bytes(cl.canonical_json(rec)) if rec else None,
            "materialization_key": (rec or {}).get("materialization_key"),
            "output_pin": (rec or {}).get("output_pin"),
            "tree_sha256": fp.get("tree_sha256"),
            "logical_tree_sha256": fp.get("logical_tree_sha256"),
            "content_tree_sha256": fp.get("content_tree_sha256"),
            "bytes": fp.get("bytes"),
            "file_count": (fp.get("counts") or {}).get("files"),
            "materialized_relpath": (rec or {}).get("materialized_relpath"),
        }
        manifest_items.append(entry)

    # ---- coverage --------------------------------------------------------------------
    table = {}
    for fam in cl.FAMILIES:
        table[fam] = {s: {sc: {"items": 0, "materialized": 0, "bytes": 0} for sc in cl.SCALE_ORDER} for s in cl.SPLITS}
    for e in manifest_items:
        cell = table[e["family"]][e["split"]][e["scale"]]
        cell["items"] += 1
        if e["status"] in ("ok", "stale-metadata"):
            cell["materialized"] += 1
            cell["bytes"] += e["bytes"] or 0
    gaps = [f"{fam}/{s}" for fam in cl.FAMILIES for s in cl.SPLITS
            if sum(table[fam][s][sc]["items"] for sc in cl.SCALE_ORDER) == 0]
    by_rog = {}
    for e in manifest_items:
        by_rog[e["real_or_generated"]] = by_rog.get(e["real_or_generated"], 0) + 1
    complete = all(e["status"] == "ok" for e in manifest_items)

    manifest = {
        "manifest_format": MANIFEST_FORMAT,
        "corpus_version": cl.CORPUS_VERSION,
        "families": cl.FAMILIES,
        "splits": list(cl.SPLITS),
        "scale_tiers": {k: {"max_bytes": v} for k, v in cl.SCALES.items()},
        "fingerprint_format": fpm.FORMAT,
        "tool_sha256": cl.tool_digests(),
        "complete": complete,
        "item_count": len(manifest_items),
        "items_by_real_or_generated": dict(sorted(by_rog.items())),
        "items": manifest_items,
        "coverage": {"family_split_scale": table, "empty_family_splits": gaps},
        "split_audit": {"errors": [], "warnings": warnings,
                        "rule": "within a family an independence_group may appear in one split only; across "
                                "families a group or upstream (URL, SHA-256, git repo, docker repository) shared "
                                "with the held-out split is an error; derive items must stay in their sources' split"},
    }
    manifest["manifest_sha256"] = cl.sha256_bytes(cl.canonical_json(manifest))

    # ---- statistics --------------------------------------------------------------------
    stats_items, stats_problems = {}, []
    agg: dict = {}
    for e in manifest_items:
        if e["split"] == "heldout":
            continue
        st = cl.load_json_if_exists(layout.stats_path(e["item_id"]))
        if e["status"] not in ("ok", "stale-metadata"):
            continue
        if not st:
            stats_problems.append(f"{e['item_id']}: stats missing")
            continue
        if st.get("logical_tree_sha256") != e["logical_tree_sha256"]:
            stats_problems.append(f"{e['item_id']}: stats stale (logical hash differs)")
            continue
        stats_items[e["item_id"]] = st
        key = f"{e['family']}/{e['split']}"
        a = agg.setdefault(key, {"items": 0, "bytes": 0, "unique_file_inodes": 0, "zstd_in": 0, "zstd_out": 0,
                                 "xz_in": 0, "xz_out": 0, "exact_dup_bytes": 0, "exact_nonempty_bytes": 0,
                                 "block_dup_bytes": 0, "block_bytes": 0, "entropy_weighted": 0.0})
        a["items"] += 1
        a["bytes"] += st["totals"]["bytes"]
        a["unique_file_inodes"] += st["totals"]["unique_file_inodes"]
        z, x = st["compressibility"].get("zstd -3"), st["compressibility"].get("xz -6")
        if z:
            a["zstd_in"] += z["input_bytes"]
            a["zstd_out"] += z["output_bytes"]
        if x:
            a["xz_in"] += x["input_bytes"]
            a["xz_out"] += x["output_bytes"]
        a["exact_dup_bytes"] += st["exact_duplicates"]["duplicate_bytes"]
        a["exact_nonempty_bytes"] += st["exact_duplicates"]["nonempty_bytes"]
        a["block_dup_bytes"] += st["block_duplicates"]["duplicate_bytes"]
        a["block_bytes"] += st["block_duplicates"]["bytes"]
        a["entropy_weighted"] += (st["entropy"]["stream_order0_bits_per_byte"] or 0.0) * st["totals"]["bytes"]
    aggregates = {}
    for key in sorted(agg):
        a = agg[key]
        aggregates[key] = {
            "items": a["items"], "bytes": a["bytes"], "unique_file_inodes": a["unique_file_inodes"],
            "zstd_-3_ratio": round(a["zstd_out"] / a["zstd_in"], 6) if a["zstd_in"] else None,
            "xz_-6_ratio": round(a["xz_out"] / a["xz_in"], 6) if a["xz_in"] else None,
            "exact_duplicate_byte_ratio": round(a["exact_dup_bytes"] / a["exact_nonempty_bytes"], 6)
            if a["exact_nonempty_bytes"] else 0.0,
            "block_duplicate_byte_ratio": round(a["block_dup_bytes"] / a["block_bytes"], 6) if a["block_bytes"] else 0.0,
            "byte_weighted_stream_order0_entropy": round(a["entropy_weighted"] / a["bytes"], 6) if a["bytes"] else None,
            "note": "ratios are byte-weighted over items (sum of outputs / sum of inputs); indicators only",
        }
    heldout_fp = {e["item_id"]: {"family": e["family"], "scale": e["scale"], "status": e["status"],
                                 "bytes": e["bytes"], "file_count": e["file_count"],
                                 "logical_tree_sha256": e["logical_tree_sha256"]}
                  for e in manifest_items if e["split"] == "heldout"}
    statistics = {
        "statistics_format": STATISTICS_FORMAT,
        "corpus_version": cl.CORPUS_VERSION,
        "manifest_sha256": manifest["manifest_sha256"],
        "items": stats_items,
        "missing_or_stale": stats_problems,
        "aggregates_by_family_split": aggregates,
        "heldout": {"note": HELDOUT_NOTE, "items": heldout_fp},
    }

    # ---- coverage.md -------------------------------------------------------------------
    hdr = "| Family | " + " | ".join(f"{s[0].upper()}:{sc[0].upper()}" for s in cl.SPLITS for sc in cl.SCALE_ORDER) + " |"
    sep = "|---|" + "---:|" * (len(cl.SPLITS) * len(cl.SCALE_ORDER))
    rows = [f"# Corpus coverage ({cl.CORPUS_VERSION})", "",
            f"manifest_sha256 `{manifest['manifest_sha256']}`", "",
            "Cells show materialized/defined items. Columns: split (T=tuning, V=validation, H=heldout) : "
            "scale (S<=16 MiB, M<=512 MiB, L<=8 GiB).", "", hdr, sep]
    for fam, name in cl.FAMILIES.items():
        cells = []
        for s in cl.SPLITS:
            for sc in cl.SCALE_ORDER:
                c = table[fam][s][sc]
                cells.append(f"{c['materialized']}/{c['items']}" if c["items"] else "-")
        rows.append(f"| {fam} {name} | " + " | ".join(cells) + " |")
    rows += ["", f"Empty family/split combinations: {len(gaps)}" + (": " + ", ".join(gaps) if gaps else ""), ""]
    coverage_md = "\n".join(rows)

    # ---- held-out lock -----------------------------------------------------------------
    held = [e for e in manifest_items if e["split"] == "heldout"]
    lock_items = [{"item_id": e["item_id"], "family": e["family"], "scale": e["scale"],
                   "independence_group": e["independence_group"], "status": e["status"],
                   "logical_tree_sha256": e["logical_tree_sha256"], "tree_sha256": e["tree_sha256"],
                   "content_tree_sha256": e["content_tree_sha256"], "bytes": e["bytes"],
                   "file_count": e["file_count"]} for e in held]
    set_lines = "".join(f"{e['item_id']}\t{e['logical_tree_sha256']}\n" for e in lock_items)
    set_sha = cl.sha256_bytes(set_lines.encode("utf-8"))
    old = cl.load_json_if_exists(lock_path)
    lock = {"lock_format": LOCK_FORMAT, "corpus_version": cl.CORPUS_VERSION, "status": "draft",
            "heldout_set_sha256": set_sha,
            "heldout_set_rule": "SHA-256 over lines '<item_id>\\t<logical_tree_sha256>\\n' sorted by item_id",
            "manifest_sha256": manifest["manifest_sha256"], "items": lock_items, "history": []}
    if old:
        lock["history"] = old.get("history", [])
    if old and old.get("status") == "frozen":
        if old.get("heldout_set_sha256") != set_sha:
            if not args.relock:
                print(f"ERROR: HELD-OUT SET CHANGED: frozen {old.get('heldout_set_sha256')} != current {set_sha}. "
                      f"Refusing to write; use --relock --reason '...' only with an explicit decision record.")
                return 1
            if not args.reason:
                print("ERROR: --relock requires --reason")
                return 2
            lock["history"].append({"previous_heldout_set_sha256": old.get("heldout_set_sha256"),
                                    "previous_frozen_utc": old.get("frozen_utc"),
                                    "previous_frozen_git_head": old.get("frozen_git_head"),
                                    "relocked_utc": cl.utc_now(), "reason": args.reason,
                                    "previous_item_ids": [i["item_id"] for i in old.get("items", [])]})
            lock["status"] = "draft"
        else:
            for k in ("status", "frozen_utc", "frozen_git_head"):
                lock[k] = old.get(k)
    if args.freeze:
        missing = [e["item_id"] for e in held if e["status"] != "ok"]
        if not held or missing:
            print(f"ERROR: cannot freeze: {'no held-out items' if not held else 'not current: ' + ', '.join(missing)}")
            return 1
        if lock.get("status") != "frozen":
            lock["status"] = "frozen"
            lock["frozen_utc"] = cl.utc_now()
            lock["frozen_git_head"] = cl.git_head()

    if args.require_stats and stats_problems:
        problems += stats_problems
    cl.write_json_atomic(layout.corpus_dir / "manifest.json", manifest)
    cl.write_json_atomic(layout.corpus_dir / "statistics.json", statistics)
    cl.write_text_atomic(layout.corpus_dir / "coverage.md", coverage_md)
    cl.write_json_atomic(lock_path, lock)
    for w in warnings:
        print(f"WARNING: {w}")
    print(f"manifest: {len(manifest_items)} items, complete={complete}, manifest_sha256={manifest['manifest_sha256']}")
    print(f"statistics: {len(stats_items)} items with stats, {len(stats_problems)} missing/stale")
    print(f"heldout-lock: {len(lock_items)} items, status={lock['status']}, heldout_set_sha256={set_sha}")
    if problems:
        for p in problems:
            print(f"ERROR: {p}")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
