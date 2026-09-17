#!/usr/bin/env python3
"""Round-trip verification for one baseline archive/repository, used as the
`roundtrip_ok` ebr "check" metric in research/baselines/specs/EXP-BASE-SIZE*.yaml.

Extracts <artifact> (per <format_tag>'s incumbent tool) into <dest>, fingerprints
both <item_path> and <dest> with the corpus framework's own fingerprint_path
(content_tree_sha256 / logical_tree_sha256 -- the same definitions
research/corpus/manifest.json's pins use), writes a JSON report to <report>, and
exits 0 iff content_tree_sha256 matches (the portable, format-agnostic
round-trip bar -- the only thing this "check" metric gates, since ebr has no
concept of a non-gating check and several legitimate incumbents do not
reproduce every metadata field logical_tree_sha256 checks).

Every sample (match or not) is appended as one NDJSON record to --detail-log,
including whether logical_tree_sha256 also matched (informational metadata
fidelity) and, on a content mismatch, a plain path-level diff (added/removed/
changed relative paths, independent of the canonical-line format) explaining
exactly what differed.

Usage:
  roundtrip_check.py <item_path> <artifact> <format_tag> <dest> <report>
                      [--detail-log PATH] [--candidate NAME] [--item-id ID] [--rep N]
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import shlex
import shutil
import stat
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from fpshim import fingerprint_path  # noqa: E402


def sh(cmd: str, timeout: int = 1800) -> None:
    cp = subprocess.run(["bash", "-o", "pipefail", "-c", cmd], capture_output=True, timeout=timeout)
    if cp.returncode != 0:
        raise RuntimeError(
            f"extract command failed (exit {cp.returncode}): {cmd}\n"
            f"stdout: {cp.stdout[-2000:].decode('utf-8', 'replace')}\n"
            f"stderr: {cp.stderr[-2000:].decode('utf-8', 'replace')}"
        )


def extract(format_tag: str, artifact: Path, dest: Path) -> None:
    dest.mkdir(parents=True, exist_ok=True)
    a = shlex.quote(str(artifact))
    d = shlex.quote(str(dest))
    if format_tag == "zip":
        sh(f"unzip -q -o {a} -d {d}")
    elif format_tag == "sevenzip":
        sh(f"7z x -y -o{d} {a} >/dev/null")
    elif format_tag == "tar+gzip":
        sh(f"tar xzf {a} -C {d}")
    elif format_tag == "tar+zstd":
        sh(f"tar --zstd -xf {a} -C {d}")
    elif format_tag == "tar+xz":
        # plain xz and pixz-produced streams are both valid xz; tar -J handles both.
        sh(f"tar xJf {a} -C {d}")
    elif format_tag == "tar+lz4":
        sh(f"tar xf {a} -I lz4 -C {d}")
    elif format_tag == "tar+brotli":
        # GNU tar 1.35's --use-compress-program mis-tokenizes a value containing
        # "-d" (misparsed as tar's own -d/--compare flag: "command already set
        # when parsing -d"); pipe brotli's decompression instead.
        sh(f"brotli -d -c {a} | tar xf - -C {d}")
    elif format_tag == "squashfs":
        # unsquashfs creates <dest> itself and refuses an existing non-empty one without -f.
        shutil.rmtree(dest, ignore_errors=True)
        sh(f"unsquashfs -f -d {d} {a}")
    elif format_tag == "wim":
        sh(f"wimapply {a} 1 {d}")
    elif format_tag == "borg":
        sh(f"cd {d} && borg extract {a}::onlyarchive")
        # borg extract recreates the full absolute path of item_path under dest;
        # descend to the matching leaf so dest directly mirrors item_path's tree.
    elif format_tag == "restic":
        sh(f"restic restore latest --repo {a} --target {d} -q")
    elif format_tag == "dwarfs":
        sh(f"dwarfsextract -i {a} -o {d}")
    elif format_tag == "zpaq":
        sh(f"zpaq x {a} -to {d} -f")
    else:
        raise ValueError(f"unknown format_tag {format_tag!r}")


def _rebase_to_match(item_path: Path, extracted_root: Path) -> Path:
    """borg/restic/some tar layouts can recreate item_path's absolute directory
    chain under extracted_root (e.g. .../extracted/root/path/to/item). If
    extracted_root itself doesn't look like item_path's content but contains
    exactly one path that, walked down, matches item_path's basename chain,
    descend into it. Falls back to extracted_root unchanged."""
    want_name = item_path.name
    cur = extracted_root
    seen = {cur}
    for _ in range(24):
        entries = [e for e in os.listdir(cur) if not e.startswith(".")]
        if len(entries) == 1:
            cand = cur / entries[0]
            if cand.is_dir():
                if cand.name == want_name:
                    return cand
                cur = cand
                if cur in seen:
                    break
                seen.add(cur)
                continue
        break
    return extracted_root


def _walk_signature(root: Path):
    """Plain (path,type,size,content-sha256|symlink-target) signature, independent
    of fingerprint.py's canonical line format, used only to explain a mismatch."""
    out = {}
    root_b = os.fsencode(root)
    stack = [b""]
    while stack:
        rel = stack.pop()
        full = os.path.join(root_b, rel) if rel else root_b
        try:
            entries = list(os.scandir(full))
        except OSError as e:
            out[rel.decode("utf-8", "replace") or "."] = {"error": str(e)}
            continue
        for de in entries:
            r = os.path.join(rel, de.name) if rel else de.name
            rp = r.replace(b"\\", b"/").decode("utf-8", "replace")
            st = de.stat(follow_symlinks=False)
            if stat.S_ISDIR(st.st_mode):
                out[rp] = {"type": "dir"}
                stack.append(r)
            elif stat.S_ISLNK(st.st_mode):
                out[rp] = {"type": "symlink", "target": os.readlink(os.path.join(root_b, r)).decode("utf-8", "replace")}
            elif stat.S_ISREG(st.st_mode):
                h = hashlib.sha256()
                with open(os.path.join(root_b, r), "rb") as fh:
                    for chunk in iter(lambda: fh.read(1 << 20), b""):
                        h.update(chunk)
                out[rp] = {"type": "file", "size": st.st_size, "sha256": h.hexdigest()}
            else:
                out[rp] = {"type": "other"}
    return out


def diff_trees(item_path: Path, extracted: Path) -> dict:
    a = _walk_signature(item_path)
    b = _walk_signature(extracted)
    only_source = sorted(set(a) - set(b))
    only_extracted = sorted(set(b) - set(a))
    changed = []
    for k in sorted(set(a) & set(b)):
        if a[k] != b[k]:
            changed.append({"path": k, "source": a[k], "extracted": b[k]})
    return {
        "only_in_source": only_source[:200],
        "only_in_extracted": only_extracted[:200],
        "changed": changed[:200],
        "only_in_source_count": len(only_source),
        "only_in_extracted_count": len(only_extracted),
        "changed_count": len(changed),
    }


def main(argv=None) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("item_path")
    ap.add_argument("artifact")
    ap.add_argument("format_tag")
    ap.add_argument("dest")
    ap.add_argument("report")
    ap.add_argument("--detail-log", help="NDJSON log appended for every sample (match or not), not just mismatches")
    ap.add_argument("--candidate", default="")
    ap.add_argument("--item-id", default="")
    ap.add_argument("--rep", default="")
    args = ap.parse_args(argv)

    item_path = Path(args.item_path)
    artifact = Path(args.artifact)
    dest = Path(args.dest)
    report_path = Path(args.report)
    shutil.rmtree(dest, ignore_errors=True)

    try:
        extract(args.format_tag, artifact, dest)
    except Exception as exc:  # noqa: BLE001 - report the failure, don't crash the sample loop
        report_path.write_text(json.dumps({"error": str(exc), "content_match": False, "logical_match": False}), encoding="utf-8")
        if args.detail_log:
            os.makedirs(os.path.dirname(args.detail_log) or ".", exist_ok=True)
            with open(args.detail_log, "a", encoding="utf-8") as fh:
                fh.write(json.dumps({
                    "candidate": args.candidate, "item_id": args.item_id, "rep": args.rep,
                    "format_tag": args.format_tag, "content_match": False, "logical_match": False,
                    "extract_error": str(exc)[:1000],
                }, sort_keys=True) + "\n")
        print(json.dumps({"roundtrip_error": str(exc)[:500]}))
        return 1

    # Safe for every format: it only descends when the extraction root has exactly
    # one child (the shape produced by tools that store an absolute-path chain,
    # e.g. borg/zpaq), and is a no-op when children already sit directly at dest.
    extract_root = _rebase_to_match(item_path, dest) if item_path.is_dir() else dest

    src_fp = fingerprint_path(item_path)
    dst_fp = fingerprint_path(extract_root)
    content_match = src_fp["content_tree_sha256"] == dst_fp["content_tree_sha256"]
    logical_match = src_fp["logical_tree_sha256"] == dst_fp["logical_tree_sha256"]

    report = {
        "format_tag": args.format_tag,
        "content_match": content_match,
        "logical_match": logical_match,
        "source_content_tree_sha256": src_fp["content_tree_sha256"],
        "extracted_content_tree_sha256": dst_fp["content_tree_sha256"],
        "source_logical_tree_sha256": src_fp["logical_tree_sha256"],
        "extracted_logical_tree_sha256": dst_fp["logical_tree_sha256"],
        "source_bytes": src_fp["bytes"],
        "extracted_bytes": dst_fp["bytes"],
    }
    if not content_match:
        report["diff"] = diff_trees(item_path, extract_root)

    if args.detail_log:
        rec = {
            "candidate": args.candidate,
            "item_id": args.item_id,
            "rep": args.rep,
            "format_tag": args.format_tag,
            "content_match": content_match,
            "logical_match": logical_match,
            "source_content_tree_sha256": report["source_content_tree_sha256"],
            "extracted_content_tree_sha256": report["extracted_content_tree_sha256"],
            "source_logical_tree_sha256": report["source_logical_tree_sha256"],
            "extracted_logical_tree_sha256": report["extracted_logical_tree_sha256"],
        }
        if not content_match:
            rec["diff"] = report["diff"]
        os.makedirs(os.path.dirname(args.detail_log) or ".", exist_ok=True)
        with open(args.detail_log, "a", encoding="utf-8") as fh:
            fh.write(json.dumps(rec, sort_keys=True) + "\n")

    report_path.write_text(json.dumps(report, sort_keys=True), encoding="utf-8")
    print(json.dumps({"roundtrip_content_match": content_match, "roundtrip_logical_match": logical_match}))
    return 0 if content_match else 1


if __name__ == "__main__":
    raise SystemExit(main())
