#!/usr/bin/env python3
"""Provenance-level lockfile audit for g1-code family F03 (dependency/vendor trees).

Gap F03 (corpus round-1 critic, MAJOR): split labels for F03 pass, but the *content*
of several vendor trees leaks across splits (they draw from overlapping sets of
third-party packages). This script gives a cheap, reproducible, pre-registered way
to measure that overlap from each item's own lockfile -- Cargo.lock (cargo), go.sum
(go modules), package-lock.json (npm) or a hash-locked pip requirements file -- by
computing each item's declared "name@version" dependency set and the pairwise
overlap between items. This is a provenance/name-identity measure (does item A
declare the same package versions as item B), not a byte-level measure (how many
bytes of A's vendored sources are byte-identical to B's) -- the two are correlated
but not the same; the byte-level figures already quoted for this gap (e.g. "bat
shares 14.7% (79 MiB) with ripgrep's vendor tree") come from corpus fingerprinting,
not from this tool. This tool answers the complementary, cheaper question at the
package-identity level, which is what a "shared_packages" experiment covariate
needs to key on.

Held-out guard: by default this script only reads lockfiles for items whose split is
"tuning" or "validation". A held-out F03 item's Cargo.lock/go.sum/package-lock.json
IS the held-out project's own content (this is not the corpus's synthesized pip
lock case -- see NOTE below), so reading it before the design freeze would leak
held-out information exactly like reading the vendored source would. Held-out items
are only ever processed with --include-heldout, which additionally requires
--unlock-heldout <design-freeze-commit> and passes corpuslib's two-key
check_heldout_unlock -- so this script structurally cannot be pointed at a held-out
project's lockfile before the freeze, even by operator mistake. Pre-freeze runs
(this one included) report held-out items only as "not measured (pre-freeze)".

NOTE on the two pip items: f03-tuning-pypi-scientific-site-packages and
f03-heldout-pypi-web-site-packages are NOT "some upstream project's own lockfile":
their lock files (research/corpus/generators/g1-code/locks/*.txt) are curated by
this repository itself (see authoring/make_pip_lock.py), not fetched from a
held-out project's repository. Reading the *tuning* one is therefore always fine.
Per the held-out discipline above, the *heldout* one is still gated the same as
every other held-out item's lockfile (no exception is made just because the file
happens to already be readable from git) -- it is measured only after
--unlock-heldout, at which point it can be compared against the tuning stack to
close out the covariate exactly as pre-registered.

Every non-pip lockfile is fetched fresh from raw.githubusercontent.com at the exact
commit already pinned in research/corpus/sources/g1-code.json (read live from that
file, never hardcoded here, so this script can't drift out of sync with the pins)
and cached under <data-root>/cache/f03-audit/. Only the single lockfile blob is
fetched, never the repository itself.

Usage:
  run.sh f03_lockfile_audit                       # tuning+validation only (default, safe)
  run.sh f03_lockfile_audit --include-heldout \
      --unlock-heldout <design-freeze-commit>      # post-freeze: adds the heldout F03 items
  run.sh f03_lockfile_audit --offline              # cache-only, no network
"""
from __future__ import annotations

import argparse
import json
import re
import sys
import tomllib
import urllib.request
from pathlib import Path

TOOLS_DIR = Path(__file__).resolve().parents[3] / "tools"
sys.path.insert(0, str(TOOLS_DIR))
import corpuslib as cl  # noqa: E402

REPO_ROOT = cl.REPO_ROOT
SOURCES_PATH = REPO_ROOT / "research" / "corpus" / "sources" / "g1-code.json"
USER_AGENT = "entrybound-research-corpus-f03-lockfile-audit/1"
SHARED_THRESHOLD = 0.05  # gap's own bar: "sharing more than 5% of lock entries"

# item_id -> ecosystem. Every F03 vendor/dependency-tree item in g1-code.json.
REGISTRY = {
    "f03-tuning-ripgrep-cargo-vendor": "cargo",
    "f03-tuning-typescript-npm-node-modules": "npm",
    "f03-tuning-pypi-scientific-site-packages": "pip",
    "f03-tuning-fzf-go-mod-vendor": "go",
    "f03-validation-bat-cargo-vendor": "cargo",
    "f03-validation-bootstrap-npm-node-modules": "npm",
    "f03-validation-yq-go-mod-vendor": "go",
    "f03-heldout-alacritty-cargo-vendor": "cargo",
    "f03-heldout-pdfjs-npm-node-modules": "npm",
    "f03-heldout-pypi-web-site-packages": "pip",
    "f03-heldout-direnv-go-mod-vendor": "go",
}


def load_items() -> dict:
    doc = cl.load_json(SOURCES_PATH)
    return {it["item_id"]: it for it in doc["items"]}


def _raw_url(repo: str, commit: str, path: str) -> str:
    # https://github.com/OWNER/REPO -> https://raw.githubusercontent.com/OWNER/REPO/COMMIT/PATH
    owner_repo = repo.removeprefix("https://github.com/").rstrip("/")
    return f"https://raw.githubusercontent.com/{owner_repo}/{commit}/{path}"


def _git_block(item: dict, items: dict) -> dict:
    """Some F03 items embed recipe.git directly; others (e.g. ripgrep-cargo-vendor)
    derive their source from an F01 from_items dependency that has it instead."""
    r = item["recipe"]
    if "git" in r:
        return r["git"]
    for dep_id in r.get("from_items", []) or []:
        dep = items[dep_id]
        if "git" in dep["recipe"]:
            return dep["recipe"]["git"]
    raise cl.CorpusError(f"{item['item_id']}: no recipe.git, directly or via from_items")


def lockfile_url(item: dict, ecosystem: str, items: dict) -> str:
    if ecosystem == "cargo":
        g = _git_block(item, items)
        return _raw_url(g["repo"], g["commit"], "Cargo.lock")
    if ecosystem == "go":
        g = _git_block(item, items)
        return _raw_url(g["repo"], g["commit"], "go.sum")
    if ecosystem == "npm":
        r = item["recipe"]
        for inp in r["inputs"]:
            if inp["name"] == "package-lock":
                return inp["url"]
        raise cl.CorpusError(f"{item['item_id']}: no 'package-lock' input found")
    raise ValueError(ecosystem)


def pip_lock_path(item: dict) -> Path:
    keys = list(item["recipe"]["generator"]["params"]["files_sha256"])
    assert len(keys) == 1, keys
    return REPO_ROOT / keys[0]


def fetch_cached(url: str, cache_dir: Path, offline: bool) -> bytes:
    cache_dir.mkdir(parents=True, exist_ok=True)
    key = cl.sha256_bytes(url.encode())
    blob = cache_dir / key
    if blob.is_file():
        return blob.read_bytes()
    if offline:
        raise cl.CorpusError(f"not cached and --offline: {url}")
    req = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
    with urllib.request.urlopen(req, timeout=60) as resp:
        data = resp.read()
    blob.write_bytes(data)
    (cache_dir / (key + ".url")).write_text(url + "\n", encoding="utf-8")
    return data


# ---------------------------------------------------------------------------
# parsers: each returns a set of "name@version" strings


def parse_cargo_lock(text: str) -> set[str]:
    doc = tomllib.loads(text)
    out = set()
    for pkg in doc.get("package", []):
        name, version = pkg.get("name"), pkg.get("version")
        if name and version:
            out.add(f"{name}@{version}")
    return out


_GO_SUM_RE = re.compile(r"^(\S+)\s+(\S+?)(?:/go\.mod)?\s+h1:")


def parse_go_sum(text: str) -> set[str]:
    out = set()
    for line in text.splitlines():
        m = _GO_SUM_RE.match(line)
        if m:
            out.add(f"{m.group(1)}@{m.group(2)}")
    return out


def parse_package_lock_json(text: str) -> set[str]:
    doc = json.loads(text)
    out = set()
    packages = doc.get("packages")
    if isinstance(packages, dict):  # lockfileVersion 2/3
        for path, meta in packages.items():
            if not path or not isinstance(meta, dict):
                continue  # "" is the root package itself
            version = meta.get("version")
            name = meta.get("name") or path.rsplit("node_modules/", 1)[-1]
            if version:
                out.add(f"{name}@{version}")
        if out:
            return out

    def walk(deps: dict):
        for name, meta in (deps or {}).items():
            version = meta.get("version")
            if version:
                out.add(f"{name}@{version}")
            walk(meta.get("dependencies"))

    walk(doc.get("dependencies"))  # lockfileVersion 1 fallback
    return out


_PIP_RE = re.compile(r"^([A-Za-z0-9][A-Za-z0-9_.-]*)==([A-Za-z0-9_.!+-]+)")


def parse_pip_lock(text: str) -> set[str]:
    out = set()
    for line in text.splitlines():
        line = line.strip()
        if not line or line.startswith("#") or line.startswith("--"):
            continue
        m = _PIP_RE.match(line)
        if m:
            out.add(f"{m.group(1).lower()}@{m.group(2)}")
    return out


PARSERS = {"cargo": parse_cargo_lock, "go": parse_go_sum, "npm": parse_package_lock_json, "pip": parse_pip_lock}


def dependency_set(item_id: str, ecosystem: str, items: dict, cache_dir: Path, offline: bool) -> set[str]:
    item = items[item_id]
    if ecosystem == "pip":
        text = pip_lock_path(item).read_text(encoding="utf-8")
    else:
        url = lockfile_url(item, ecosystem, items)
        text = fetch_cached(url, cache_dir, offline).decode("utf-8", "replace")
    return PARSERS[ecosystem](text)


def pair_stats(a_id: str, a: set[str], b_id: str, b: set[str]) -> dict:
    shared = a & b
    union = a | b
    return {
        "a": a_id, "b": b_id,
        "a_count": len(a), "b_count": len(b), "shared_count": len(shared),
        "shared_fraction_of_a": round(len(shared) / len(a), 4) if a else 0.0,
        "shared_fraction_of_b": round(len(shared) / len(b), 4) if b else 0.0,
        "jaccard": round(len(shared) / len(union), 4) if union else 0.0,
        "over_threshold": (len(shared) / len(a) > SHARED_THRESHOLD if a else False)
                          or (len(shared) / len(b) > SHARED_THRESHOLD if b else False),
        "sample_shared_entries": sorted(shared)[:8],
    }


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--data-root", default=None)
    ap.add_argument("--include-heldout", action="store_true")
    ap.add_argument("--unlock-heldout", default=None, metavar="DESIGN_FREEZE_COMMIT")
    ap.add_argument("--offline", action="store_true")
    ap.add_argument("--out", default=str(REPO_ROOT / "research" / "corpus" / "generators" / "g1-code" /
                                        "authoring" / "f03_lockfile_audit_report.json"))
    args = ap.parse_args(argv)

    layout = cl.Layout(data_root=args.data_root)
    cache_dir = layout.cache_dir / "f03-audit"
    items = load_items()

    heldout_ok = False
    if args.include_heldout:
        cl.check_heldout_unlock(layout, args.unlock_heldout)  # raises HeldoutAccessError if not unlocked
        heldout_ok = True

    sets: dict[str, set[str]] = {}
    skipped: dict[str, str] = {}
    for item_id, eco in sorted(REGISTRY.items()):
        item = items[item_id]
        if item["split"] == "heldout" and not heldout_ok:
            skipped[item_id] = "not measured (pre-freeze): this item's lockfile is a held-out project's own content"
            continue
        sets[item_id] = dependency_set(item_id, eco, items, cache_dir, args.offline)

    measured = sorted(sets)
    pairs = []
    for i, a_id in enumerate(measured):
        for b_id in measured[i + 1:]:
            if items[a_id]["independence_group"] == items[b_id]["independence_group"]:
                continue  # same upstream project; overlap is expected/uninteresting
            pairs.append(pair_stats(a_id, sets[a_id], b_id, sets[b_id]))
    pairs.sort(key=lambda p: -max(p["shared_fraction_of_a"], p["shared_fraction_of_b"]))

    report = {
        "schema": "ebrc-f03-lockfile-audit-v1",
        "method": "pairwise intersection of per-item {name@version} sets parsed from Cargo.lock / go.sum / "
                  "package-lock.json / hash-locked pip requirements; a provenance/name-identity measure, not "
                  "a byte-level content measure",
        "shared_threshold": SHARED_THRESHOLD,
        "sources_file": cl.repo_rel(SOURCES_PATH),
        "items": {
            iid: {"split": items[iid]["split"], "independence_group": items[iid]["independence_group"],
                  "ecosystem": REGISTRY[iid], "dependency_count": len(sets[iid]) if iid in sets else None,
                  "status": "measured" if iid in sets else skipped.get(iid, "skipped")}
            for iid in sorted(REGISTRY)
        },
        "pairs": pairs,
        "over_threshold_pairs": [p for p in pairs if p["over_threshold"]],
    }
    out_path = Path(args.out)
    cl.write_json_atomic(out_path, report)
    print(f"measured {len(sets)}/{len(REGISTRY)} items ({len(skipped)} held-out skipped pre-freeze); "
          f"{len(pairs)} cross-group pairs, {len(report['over_threshold_pairs'])} over "
          f"{SHARED_THRESHOLD:.0%} threshold")
    for p in report["over_threshold_pairs"]:
        print(f"  {p['a']} <-> {p['b']}: {p['shared_count']} shared "
              f"(of a={p['a_count']}, b={p['b_count']}; {p['shared_fraction_of_a']:.1%} of a, "
              f"{p['shared_fraction_of_b']:.1%} of b, jaccard={p['jaccard']:.3f})")
    print(f"report written to {cl.repo_rel(out_path)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
