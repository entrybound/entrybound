"""Stage repository scripts, helpers and inputs referenced by timed commands onto ext4 (design review DR1-16; PA-16).

decision-method.md section 4.1 keeps WSL timing off drvfs/9P. A timed command that loads a wrapper script, a
Python helper or a JSON input from /mnt/d crosses into the Windows host (with Defender scanning D:) inside the
timed sample. Timing specs therefore reference staged copies:

    /root/eb-research/stage/<EXP-ID>/repo/<repository-relative path>

This tool (run inside WSL) finds every such reference in a spec's commands and platform.requires, copies the
repository file to the staged path (a directory reference such as a PYTHONPATH entry stages every file below it;
a {placeholder} in a path stages every file matching it), records SHA-256 per file in
/root/eb-research/stage/<EXP-ID>/stage-manifest.json, and with --verify re-hashes the staged copies against the
repository and the manifest immediately before a timing window (exit 6 on any mismatch). A run driver copies the
manifest next to the raw run (research/raw/<EXP-ID>/stage-manifest-<spec>-<utc>.json) so meta can cite it.

Usage (WSL, repository root):
  /root/eb-research/venv/bin/python research/tools/experiments/stage_tools.py --spec research/experiments/EXP-REMOTE-001/spec.yaml
  /root/eb-research/venv/bin/python research/tools/experiments/stage_tools.py --spec ... --verify
  /root/eb-research/venv/bin/python research/tools/experiments/stage_tools.py --spec ... --dry-run
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import shutil
import sys
from datetime import datetime, timezone
from pathlib import Path

import yaml

ROOT = Path(__file__).resolve().parents[3]
STAGE_ROOT = Path("/root/eb-research/stage")
REF_RX = re.compile(r"/root/eb-research/stage/(?P<exp>EXP-[A-Z]+-[0-9]+)/repo/(?P<rel>[A-Za-z0-9_./\-{}]+)")


def sha256(p: Path) -> str:
    h = hashlib.sha256()
    with open(p, "rb") as fh:
        for block in iter(lambda: fh.read(1 << 20), b""):
            h.update(block)
    return h.hexdigest()


def references(doc):
    texts = [str(doc.get("command", ""))]
    for c in doc.get("candidates") or []:
        if isinstance(c, dict):
            texts.append(str(c.get("command", "")))
            texts.extend(str(v) for v in (c.get("params") or {}).values())
    texts.extend(str(x) for x in ((doc.get("platform") or {}).get("requires") or []))
    texts.extend(str(v) for v in (doc.get("env") or {}).values())
    refs = set()
    for t in texts:
        for m in REF_RX.finditer(t):
            refs.add((m.group("exp"), m.group("rel").rstrip(".,;")))
    return sorted(refs)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--spec", required=True)
    ap.add_argument("--verify", action="store_true")
    ap.add_argument("--dry-run", action="store_true")
    args = ap.parse_args()
    doc = yaml.safe_load(Path(args.spec).read_text(encoding="utf-8"))
    refs = references(doc)
    if not refs:
        print("no staged references in spec")
        return 0
    exps = {e for e, _ in refs}
    if len(exps) != 1 or not str(doc.get("experiment_id", "")).startswith(next(iter(exps))):
        print(f"staged references must use the spec's own experiment id; found {sorted(exps)}", file=sys.stderr)
        return 2
    exp = exps.pop()
    manifest_path = STAGE_ROOT / exp / "stage-manifest.json"
    old = json.loads(manifest_path.read_text(encoding="utf-8")) if manifest_path.is_file() else {"files": {}}
    files = dict(old.get("files", {}))
    bad = 0
    expanded = []
    for _, rel in refs:
        pattern = re.sub(r"\{[^}]*\}", "*", rel)
        if pattern != rel:
            expanded.extend(sorted(x.relative_to(ROOT).as_posix() for x in ROOT.glob(pattern) if x.is_file()))
        elif (ROOT / rel).is_dir():
            expanded.extend(sorted(x.relative_to(ROOT).as_posix() for x in (ROOT / rel).rglob("*")
                                   if x.is_file() and "__pycache__" not in x.parts))
        else:
            expanded.append(rel)
    for rel in expanded:
        src = ROOT / rel
        dst = STAGE_ROOT / exp / "repo" / rel
        if not src.is_file():
            print(f"MISSING_SOURCE {rel}", file=sys.stderr)
            bad += 1
            continue
        want = sha256(src)
        if args.verify:
            have = sha256(dst) if dst.is_file() else None
            rec = files.get(rel)
            ok = have == want and rec == want
            print(f"{'OK' if ok else 'MISMATCH'} {rel} repo={want[:12]} staged={(have or 'absent')[:12]} manifest={(rec or 'absent')[:12]}")
            bad += 0 if ok else 1
            continue
        print(f"STAGE {rel} -> {dst} sha256={want}")
        if not args.dry_run:
            dst.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(src, dst)
            if os.access(src, os.X_OK) or rel.endswith(".sh"):
                dst.chmod(0o755)
            files[rel] = want
    if not args.verify and not args.dry_run:
        manifest_path.parent.mkdir(parents=True, exist_ok=True)
        manifest_path.write_text(json.dumps({
            "schema": "entrybound.research.stage-manifest/1",
            "experiment_id": exp,
            "updated_utc": datetime.now(timezone.utc).isoformat(timespec="seconds"),
            "files": dict(sorted(files.items())),
        }, indent=2) + "\n", encoding="utf-8")
    return 6 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
