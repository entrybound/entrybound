#!/usr/bin/env python3
"""Compute C1-C3 inputs for §7; never self-assign a candidate's final tier.

The manifest names two pinned source trees and checked-in census/audit snapshots.
Missing dependency assurance data makes C3 incomplete and blocks a tier assessment.
No held-out paths are accepted. C4-C7 require an independent assessor.
"""
from __future__ import annotations

import argparse
import difflib
import hashlib
import json
import re
from pathlib import Path

WORDS = re.compile(r"[\w]+(?:[-'][\w]+)*", re.UNICODE)
TREE = re.compile(r"^([A-Za-z0-9_-]+) v([0-9][^\s]*)")
REGISTRY_KINDS = ("wire_record_types", "fields", "enum_values", "feature_bits",
                  "reason_codes", "registry_identifiers", "identity_participation",
                  "decode_behaviours")
SOURCE_ROLES = ("writer", "full_reader", "minimal_reader", "tests")


def sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def inside(root: Path, ref: str) -> Path:
    if not isinstance(ref, str) or not ref or "heldout" in ref.casefold():
        raise ValueError(f"unsafe or held-out reference: {ref!r}")
    rel = Path(ref)
    if rel.is_absolute() or ".." in rel.parts:
        raise ValueError(f"reference leaves source tree: {ref!r}")
    path = (root / rel).resolve()
    if not path.is_relative_to(root.resolve()):
        raise ValueError(f"reference leaves source tree: {ref!r}")
    return path


def text_or_empty(root: Path, ref: str) -> str:
    path = inside(root, ref)
    return path.read_text(encoding="utf-8") if path.is_file() else ""


def changed_words(before: str, after: str) -> dict:
    a, b = WORDS.findall(before), WORDS.findall(after)
    opcodes = difflib.SequenceMatcher(None, a, b, autojunk=False).get_opcodes()
    added = sum(j2-j1 for kind, _, _, j1, j2 in opcodes if kind in ("insert", "replace"))
    removed = sum(i2-i1 for kind, i1, i2, _, _ in opcodes if kind in ("delete", "replace"))
    return {"baseline_words": len(a), "candidate_words": len(b),
            "added_or_changed_words": added, "removed_or_changed_words": removed}


def code_lines(text: str) -> int:
    """Count Rust lines with code, excluding blank and comment-only lines.

    Inline comments do not erase a code line. Block-comment state spans lines;
    string literals containing comment markers are not mistaken for comments.
    """
    count = 0
    block = False
    for line in text.splitlines():
        i, code, string = 0, False, None
        escaped = False
        while i < len(line):
            c = line[i]
            pair = line[i:i+2]
            if block:
                if pair == "*/":
                    block = False
                    i += 2
                else:
                    i += 1
                continue
            if string:
                code = True
                if escaped:
                    escaped = False
                elif c == "\\":
                    escaped = True
                elif c == string:
                    string = None
                i += 1
                continue
            if pair == "//":
                break
            if pair == "/*":
                block = True
                i += 2
                continue
            if c in ('"', "'"):
                string = c
                code = True
            elif not c.isspace():
                code = True
            i += 1
        count += int(code)
    return count


def read_json(root: Path, ref: str) -> dict:
    path = inside(root, ref)
    if not path.is_file():
        raise ValueError(f"required snapshot missing: {path}")
    obj = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(obj, dict):
        raise ValueError(f"snapshot must be object: {path}")
    return obj


def registry_delta(base: dict, candidate: dict) -> dict:
    result = {}
    for kind in REGISTRY_KINDS:
        a, b = base.get(kind), candidate.get(kind)
        if not isinstance(a, list) or not isinstance(b, list) or len(a) != len(set(a)) or len(b) != len(set(b)):
            raise ValueError(f"registry census {kind}: unique lists required in both snapshots")
        result[kind] = {"added": sorted(set(b)-set(a)), "removed": sorted(set(a)-set(b)),
                        "added_count": len(set(b)-set(a))}
    return result


def tree_packages(text: str) -> set[str]:
    result = set()
    for line in text.splitlines():
        match = TREE.match(line.strip())
        if match:
            result.add(match.group(1) + "@" + match.group(2))
    return result


def direct_normal(metadata: dict) -> set[str]:
    packages = {p["id"]: p for p in metadata.get("packages", [])}
    nodes = {n["id"]: n for n in (metadata.get("resolve") or {}).get("nodes", [])}
    result = set()
    for member in metadata.get("workspace_members", []):
        node = nodes.get(member, {})
        for dep in node.get("deps", []):
            if not any(k.get("kind") in (None, "normal") for k in dep.get("dep_kinds", [])):
                continue
            package = packages.get(dep.get("pkg"))
            if package:
                result.add(package["name"] + "@" + package["version"])
    return result


def compute(manifest: dict, baseline: Path, candidate: Path) -> dict:
    if manifest.get("schema") != "entrybound.cost-input-manifest.v1":
        raise ValueError("cost manifest schema mismatch")
    if not manifest.get("candidate_id") or not manifest.get("baseline_commit_sha") or not manifest.get("candidate_commit_sha"):
        raise ValueError("candidate and full source commits required")
    if not all(re.fullmatch(r"[0-9a-f]{40}", manifest[x]) for x in ("baseline_commit_sha", "candidate_commit_sha")):
        raise ValueError("full source commit SHAs required")
    normative = manifest.get("normative_files")
    roles = manifest.get("source_roles")
    if not isinstance(normative, list) or not normative or len(normative) != len(set(normative)) or not isinstance(roles, dict):
        raise ValueError("normative_files and source_roles required")
    if set(roles) != set(SOURCE_ROLES):
        raise ValueError("source_roles must enumerate writer/full_reader/minimal_reader/tests")
    c1_files = {ref: changed_words(text_or_empty(baseline, ref), text_or_empty(candidate, ref)) for ref in normative}
    census_ref = manifest.get("registry_census_ref")
    if not census_ref:
        raise ValueError("registry census reference required")
    c1_registry = registry_delta(read_json(baseline, census_ref), read_json(candidate, census_ref))
    c2 = {}
    for role, refs in roles.items():
        if not isinstance(refs, list) or len(refs) != len(set(refs)):
            raise ValueError(f"{role}: unique path list required")
        base_lines = sum(code_lines(text_or_empty(baseline, ref)) for ref in refs)
        candidate_lines = sum(code_lines(text_or_empty(candidate, ref)) for ref in refs)
        c2[role] = {"baseline_loc": base_lines, "candidate_loc": candidate_lines,
                    "delta_loc": candidate_lines-base_lines, "paths": refs}
    retained = manifest.get("retained_decode_paths", [])
    if not isinstance(retained, list) or len(retained) != len(set(retained)):
        raise ValueError("retained_decode_paths must be a unique list")
    c2["retained_decode_surface"] = {"candidate_loc": sum(code_lines(text_or_empty(candidate, ref)) for ref in retained),
                                      "paths": retained,
                                      "interpretation": "enumerated reachable reader files; independent reachability review required"}
    tree_ref, meta_ref, audit_ref = (manifest.get(x) for x in ("cargo_tree_ref", "cargo_metadata_ref", "dependency_audit_ref"))
    if not tree_ref or not meta_ref or not audit_ref:
        raise ValueError("cargo tree, metadata and dependency audit snapshots required")
    base_tree = tree_packages(text_or_empty(baseline, tree_ref))
    candidate_tree = tree_packages(text_or_empty(candidate, tree_ref))
    if not base_tree or not candidate_tree:
        raise ValueError("cargo tree normal/target-all snapshots empty")
    base_meta = read_json(baseline, meta_ref)
    candidate_meta = read_json(candidate, meta_ref)
    added = sorted(candidate_tree-base_tree)
    audit = read_json(candidate, audit_ref)
    entries = audit.get("packages", {})
    if not isinstance(entries, dict):
        raise ValueError("dependency audit packages must be a mapping")
    needed = set(added) | (direct_normal(candidate_meta)-direct_normal(base_meta))
    missing = sorted(needed-set(entries))
    required_fields = {"unsafe_blocks", "unsafe_functions", "native_source_bytes", "spdx_license",
                       "last_release_date", "publisher_count", "open_advisories", "msrv"}
    incomplete = [name for name in needed if name in entries and not required_fields <= set(entries[name])]
    incomplete += [name for name in needed if name in entries and required_fields <= set(entries[name])
                   and (any(type(entries[name][k]) is not int or entries[name][k] < 0
                            for k in ("unsafe_blocks", "unsafe_functions", "native_source_bytes", "publisher_count", "open_advisories"))
                        or not all(isinstance(entries[name][k], str) and entries[name][k]
                                   for k in ("spdx_license", "last_release_date", "msrv")))]
    c3 = {"normal_target_all_packages_added": added,
          "normal_target_all_packages_removed": sorted(base_tree-candidate_tree),
          "direct_normal_packages_added": sorted(direct_normal(candidate_meta)-direct_normal(base_meta)),
          "dependency_audit": {name: entries[name] for name in sorted(needed) if name in entries},
          "advisory_snapshot_sha256": audit.get("advisory_snapshot_sha256"),
          "missing_audit_entries": missing, "incomplete_audit_entries": sorted(incomplete),
          "complete": not missing and not incomplete and bool(re.fullmatch(r"[0-9a-f]{64}", str(audit.get("advisory_snapshot_sha256", ""))))}
    all_refs = sorted(set(normative) | {census_ref, tree_ref, meta_ref} |
                      {ref for refs in roles.values() for ref in refs} | set(retained))
    source_hashes = {
        "baseline": {ref: (sha(path) if (path := inside(baseline, ref)).is_file() else None) for ref in all_refs},
        "candidate": {ref: (sha(path) if (path := inside(candidate, ref)).is_file() else None) for ref in all_refs+[audit_ref]},
    }
    return {"schema": "entrybound.cost-inputs.v1", "candidate_id": manifest["candidate_id"],
            "baseline_commit_sha": manifest["baseline_commit_sha"],
            "candidate_commit_sha": manifest["candidate_commit_sha"],
            "C1": {"normative_files": c1_files, "normative_words_added_or_changed": sum(x["added_or_changed_words"] for x in c1_files.values()),
                   "registry": c1_registry}, "C2": c2, "C3": c3,
            "input_sha256": source_hashes,
            "manifest_sha256": hashlib.sha256(json.dumps(manifest, sort_keys=True, separators=(",", ":")).encode()).hexdigest(),
            "tier_status": "AWAITING_INDEPENDENT_C4_C7_ASSESSMENT" if c3["complete"] else "C3_INCOMPLETE_NO_TIER"}


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--manifest", required=True, type=Path)
    ap.add_argument("--baseline-root", required=True, type=Path)
    ap.add_argument("--candidate-root", required=True, type=Path)
    ap.add_argument("--out", required=True, type=Path)
    args = ap.parse_args()
    manifest = json.loads(args.manifest.read_text(encoding="utf-8"))
    result = compute(manifest, args.baseline_root.resolve(), args.candidate_root.resolve())
    args.out.write_text(json.dumps(result, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(json.dumps({"candidate_id": result["candidate_id"], "tier_status": result["tier_status"],
                      "out": str(args.out)}, sort_keys=True))
    return 0 if result["C3"]["complete"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
