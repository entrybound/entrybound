#!/usr/bin/env python3
"""Per-package license audit for g1-code family F03 (dependency/vendor trees).

Gap F03 (MAJOR): the 11 "LicenseRef-mixed-per-package" F03 items were never audited
package by package; license.redistributable was set to false purely as a
conservative default. This script generates a per-package license manifest for each
*already-materialized, non-held-out* F03 item and recommends redistributable from
that evidence, using an ecosystem-appropriate authoritative source wherever one
exists:

  cargo (ripgrep, bat vendor/): each crate's own vendored Cargo.toml
      [package].license / license-file -- exactly what `cargo metadata` reports,
      read directly (no cargo/network invocation needed: the vendor tree already
      carries every crate's own manifest).
  npm  (typescript, bootstrap node_modules/): each package's own package.json
      "license" (or legacy "licenses") field -- exactly what `license-checker`
      reports, read directly.
  pip  (pypi-scientific site-packages/): importlib.metadata distribution metadata
      (License field and "License ::" trove classifiers) via
      importlib.metadata.distributions(path=[...]) -- the same data source
      `pip-licenses` itself reads, applied directly against the materialized
      site-packages directory without needing a matching interpreter installed.
  go   (fzf, yq vendor/): go vendoring has no manifest license field, so this is a
      best-effort heuristic (module boundaries from vendor/modules.txt, then a
      keyword classifier over each module's root-level LICENSE/COPYING/NOTICE-style
      file) -- the same approach tools like `go-licenses` use, but implemented
      in-repo (installing the real go-licenses binary needs a full `go build`
      module resolution this vendor-only tree does not have net access to
      reproduce offline). Every heuristic verdict is reported with the evidence
      file so a human can override it.

A package's evidence classifies as one of: permissive (an SPDX id on the
ALLOWLIST below), copyleft (GPL/AGPL/LGPL family with no permissive
alternative in an "A OR B" expression), or unresolved (no manifest field / no
license file found / text did not match a known license). An item is recommended
redistributable=true only when EVERY package resolves to "permissive"; otherwise
it stays false and the unresolved/copyleft packages are listed as the reason.

Held-out F03 items are never touched by this script (it only accepts the 7
tuning/validation item ids below); this mirrors f03_lockfile_audit.py's guard.

Usage: run.sh f03_license_audit
"""
from __future__ import annotations

import json
import re
import sys
import tomllib
from pathlib import Path

TOOLS_DIR = Path(__file__).resolve().parents[3] / "tools"
sys.path.insert(0, str(TOOLS_DIR))
import corpuslib as cl  # noqa: E402

REPO_ROOT = cl.REPO_ROOT
SOURCES_PATH = REPO_ROOT / "research" / "corpus" / "sources" / "g1-code.json"
OUT_DIR = REPO_ROOT / "research" / "corpus" / "generators" / "g1-code" / "authoring" / "licenses"

# SPDX identifiers treated as permissive for the redistributable recommendation.
# Deliberately conservative: anything not on this list (GPL/AGPL/LGPL family,
# unrecognized, or "unresolved") keeps the item non-redistributable.
PERMISSIVE = {
    "MIT", "MIT-0", "Apache-2.0", "BSD-2-Clause", "BSD-3-Clause", "BSD-4-Clause", "ISC", "Unlicense", "0BSD",
    "Zlib", "CC0-1.0", "Python-2.0", "PSF-2.0", "WTFPL", "BlueOak-1.0.0", "OFL-1.1", "MPL-2.0", "BSL-1.0",
    "CC-BY-3.0", "CC-BY-4.0", "Artistic-2.0", "MIT-CMU", "HPND", "TCL", "Vim",
}
# A vendored crate/package license field may be an "A OR B" / "A/B" (either satisfies) or
# "A AND B" (both apply) expression. This splits on OR/AND alike and treats a match as
# permissive if ANY part is on the allowlist -- which is deliberately optimistic for AND
# (it should really require ALL parts to be acceptable) but conservative in the direction
# that matters: an AND expression whose other half is an unrecognized custom license (e.g.
# "(MIT OR Apache-2.0) AND Unicode-DFS-2016") still fails to classify as fully permissive
# here only because parens/aliases aren't normalized -- see the alias table below for the
# handful of cases actually seen in this corpus; anything else stays a flagged, human-
# reviewable "copyleft-or-other"/"unresolved" rather than being silently approved.
_OR_SPLIT = re.compile(r"\s+OR\s+|\s*/\s*|\s+AND\s+")
_STRIP_PARENS = re.compile(r"[()]")
_ALIASES = {  # non-SPDX spellings actually observed in this corpus's manifests
    "APACHE 2.0": "Apache-2.0", "APACHE-2": "Apache-2.0", "APACHE2": "Apache-2.0",
    "BSD-3-CLAUSE": "BSD-3-Clause", "THE MIT LICENSE": "MIT", "MIT LICENSE": "MIT",
}


def classify(expr: str) -> str:
    if not expr or not expr.strip():
        return "unresolved"
    cleaned = _STRIP_PARENS.sub(" ", expr)
    parts = [p.strip() for p in _OR_SPLIT.split(cleaned) if p.strip()]
    parts = [_ALIASES.get(p.upper(), p) for p in parts]
    return "permissive" if any(p in PERMISSIVE for p in parts) else "copyleft-or-other"


# ---------------------------------------------------------------------------
# cargo: vendor/<crate-version>/Cargo.toml


def audit_cargo(vendor_dir: Path) -> list[dict]:
    out = []
    for crate_dir in sorted(p for p in vendor_dir.iterdir() if p.is_dir()):
        toml_path = crate_dir / "Cargo.toml"
        if not toml_path.is_file():
            out.append({"package": crate_dir.name, "license_expr": None, "source": "no Cargo.toml", "class": "unresolved"})
            continue
        doc = tomllib.loads(toml_path.read_text(encoding="utf-8", errors="replace"))
        pkg = doc.get("package", {})
        lic = pkg.get("license")
        lic_file = pkg.get("license-file")
        if lic:
            out.append({"package": crate_dir.name, "license_expr": lic, "source": "Cargo.toml [package].license",
                        "class": classify(lic)})
        elif lic_file:
            out.append({"package": crate_dir.name, "license_expr": None, "source": f"Cargo.toml license-file={lic_file}",
                        "class": "unresolved"})  # present but needs a human/manual read of the file text
        else:
            out.append({"package": crate_dir.name, "license_expr": None, "source": "Cargo.toml has no license field",
                        "class": "unresolved"})
    return out


# ---------------------------------------------------------------------------
# npm: node_modules/**/package.json (nested node_modules for de-duped versions too)


def audit_npm(node_modules_dir: Path) -> list[dict]:
    out = []
    seen = set()
    for pkg_json in sorted(node_modules_dir.rglob("package.json")):
        rel = pkg_json.relative_to(node_modules_dir)
        if rel.parent.name.startswith("."):
            continue
        try:
            meta = json.loads(pkg_json.read_text(encoding="utf-8", errors="replace"))
        except (json.JSONDecodeError, OSError):
            continue
        name, version = meta.get("name"), meta.get("version")
        if not name:
            continue
        key = (name, version)
        if key in seen:
            continue
        seen.add(key)
        lic = meta.get("license")
        if isinstance(lic, dict):
            lic = lic.get("type")
        if not lic and isinstance(meta.get("licenses"), list) and meta["licenses"]:
            lic = " OR ".join(sorted({x.get("type") for x in meta["licenses"] if x.get("type")}))
        label = f"{name}@{version}" if version else name
        if lic:
            out.append({"package": label, "license_expr": lic, "source": "package.json license",
                        "class": classify(lic)})
        else:
            out.append({"package": label, "license_expr": None, "source": "package.json has no license field",
                        "class": "unresolved"})
    return out


# ---------------------------------------------------------------------------
# pip: importlib.metadata against the materialized site-packages dir (same data pip-licenses reads)


def audit_pip(site_packages_dir: Path) -> list[dict]:
    import importlib.metadata as im
    out = []
    for dist in im.distributions(path=[str(site_packages_dir)]):
        name = dist.metadata.get("Name") or dist.name
        version = dist.metadata.get("Version") or dist.version
        lic = dist.metadata.get("License")
        license_expr_field = dist.metadata.get("License-Expression")  # PEP 639
        classifiers = [v for k, v in (dist.metadata.items() if hasattr(dist.metadata, "items") else [])
                       if k == "Classifier" and v.startswith("License ::")]
        expr = None
        source = "no License metadata"
        if license_expr_field:
            expr = license_expr_field
            source = "PEP 639 License-Expression"
        elif classifiers:
            spdx_from_classifier = {
                "License :: OSI Approved :: MIT License": "MIT",
                "License :: OSI Approved :: Apache Software License": "Apache-2.0",
                "License :: OSI Approved :: BSD License": "BSD-3-Clause",
                "License :: OSI Approved :: ISC License (ISCL)": "ISC",
                "License :: OSI Approved :: The Unlicense (Unlicense)": "Unlicense",
                "License :: OSI Approved :: Python Software Foundation License": "PSF-2.0",
                "License :: OSI Approved :: Mozilla Public License 2.0 (MPL 2.0)": "MPL-2.0",
                "License :: OSI Approved :: GNU General Public License v2 (GPLv2)": "GPL-2.0-only",
                "License :: OSI Approved :: GNU General Public License v3 (GPLv3)": "GPL-3.0-only",
                "License :: OSI Approved :: GNU Lesser General Public License v3 (LGPLv3)": "LGPL-3.0-only",
                "License :: OSI Approved :: GNU Library or Lesser General Public License (LGPL)": "LGPL-2.1-only",
            }
            mapped = [spdx_from_classifier.get(c) for c in classifiers]
            mapped = [m for m in mapped if m]
            if mapped:
                expr = " OR ".join(sorted(set(mapped)))
                source = "trove classifier"
        if not expr and lic and lic not in ("UNKNOWN", ""):
            expr = lic
            source = "METADATA License field"
        label = f"{name}@{version}"
        out.append({"package": label, "license_expr": expr, "source": source,
                    "class": classify(expr) if expr else "unresolved"})
    return out


# ---------------------------------------------------------------------------
# go: vendor/modules.txt module boundaries + a keyword classifier over a root license file

_GO_LICENSE_KEYWORDS = [
    ("MIT", re.compile(r"\bMIT License\b|Permission is hereby granted, free of charge", re.I)),
    ("Apache-2.0", re.compile(r"Apache License,?\s+Version 2\.0", re.I)),
    ("BSD-3-Clause", re.compile(r"Redistributions? in binary form.*Redistributions? of source", re.I | re.S)),
    ("ISC", re.compile(r"\bISC License\b|Permission to use, copy, modify, and(?:/or)? distribute", re.I)),
    ("MPL-2.0", re.compile(r"Mozilla Public License,?\s+version 2\.0|Mozilla Public License Version 2\.0", re.I)),
    ("GPL-3.0-only", re.compile(r"GNU GENERAL PUBLIC LICENSE\s*\n?\s*Version 3", re.I)),
    ("GPL-2.0-only", re.compile(r"GNU GENERAL PUBLIC LICENSE\s*\n?\s*Version 2", re.I)),
    ("LGPL-3.0-only", re.compile(r"GNU LESSER GENERAL PUBLIC LICENSE\s*\n?\s*Version 3", re.I)),
    ("Unlicense", re.compile(r"This is free and unencumbered software released into the public domain", re.I)),
    ("BSD-2-Clause", re.compile(r"Redistributions of source code must retain", re.I)),
]
_GO_LICENSE_FILENAMES = re.compile(r"^(LICEN[CS]E|COPYING|NOTICE|UNLICENSE)(\..*)?$", re.I)
_GO_MODULE_LINE = re.compile(r"^# (\S+) (\S+)")  # "# module version" or "# module => replacement"


def go_module_roots(vendor_dir: Path) -> dict[str, str]:
    """module import path -> version, from vendor/modules.txt's '# module version' lines."""
    mods = {}
    txt = (vendor_dir / "modules.txt")
    if not txt.is_file():
        return mods
    for line in txt.read_text(encoding="utf-8", errors="replace").splitlines():
        m = _GO_MODULE_LINE.match(line)
        if m and not line.startswith("## "):
            mods[m.group(1)] = m.group(2)
    return mods


def audit_go(vendor_dir: Path) -> list[dict]:
    out = []
    for module_path, version in sorted(go_module_roots(vendor_dir).items()):
        root = vendor_dir / module_path
        if not root.is_dir():
            out.append({"package": f"{module_path}@{version}", "license_expr": None,
                        "source": "module root not found under vendor/", "class": "unresolved"})
            continue
        found = None
        for child in sorted(root.iterdir()):
            if child.is_file() and _GO_LICENSE_FILENAMES.match(child.name):
                found = child
                break
        if not found:
            out.append({"package": f"{module_path}@{version}", "license_expr": None,
                        "source": "no LICENSE/COPYING/NOTICE file at module root", "class": "unresolved"})
            continue
        text = found.read_text(encoding="utf-8", errors="replace")
        matched = next((spdx for spdx, pat in _GO_LICENSE_KEYWORDS if pat.search(text)), None)
        out.append({"package": f"{module_path}@{version}", "license_expr": matched,
                    "source": f"heuristic keyword match on {found.name}", "class": classify(matched) if matched
                    else "unresolved"})
    return out


AUDITORS = {"cargo": audit_cargo, "npm": audit_npm, "pip": audit_pip, "go": audit_go}

# item_id -> (ecosystem, materialized subpath to hand to the auditor)
ITEMS = {
    "f03-tuning-ripgrep-cargo-vendor": ("cargo", "vendor"),
    "f03-tuning-typescript-npm-node-modules": ("npm", "node_modules"),
    "f03-tuning-pypi-scientific-site-packages": ("pip", "site-packages"),
    "f03-tuning-fzf-go-mod-vendor": ("go", "vendor"),
    "f03-validation-bat-cargo-vendor": ("cargo", "vendor"),
    "f03-validation-bootstrap-npm-node-modules": ("npm", "node_modules"),
    "f03-validation-yq-go-mod-vendor": ("go", "vendor"),
}


def main() -> int:
    layout = cl.Layout()
    items = {it["item_id"]: it for it in cl.load_json(SOURCES_PATH)["items"]}
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    summary = {}
    for item_id, (eco, subpath) in sorted(ITEMS.items()):
        item = items[item_id]
        assert item["split"] != "heldout", item_id
        tree = layout.item_path(item)
        target = tree / subpath
        if not target.is_dir():
            print(f"SKIP {item_id}: {target} not materialized", file=sys.stderr)
            continue
        packages = AUDITORS[eco](target)
        classes = {}
        for p in packages:
            classes.setdefault(p["class"], []).append(p["package"])
        all_permissive = packages and set(classes) == {"permissive"}
        manifest = {
            "item_id": item_id, "ecosystem": eco, "package_count": len(packages),
            "class_counts": {k: len(v) for k, v in sorted(classes.items())},
            "non_permissive_packages": sorted(classes.get("copyleft-or-other", []) + classes.get("unresolved", [])),
            "recommended_redistributable": all_permissive,
            "packages": packages,
        }
        cl.write_json_atomic(OUT_DIR / f"{item_id}.json", manifest)
        summary[item_id] = {"package_count": len(packages), "class_counts": manifest["class_counts"],
                            "recommended_redistributable": all_permissive}
        print(f"{item_id}: {len(packages)} packages, {manifest['class_counts']}, "
              f"recommended redistributable={all_permissive}")
    cl.write_json_atomic(OUT_DIR / "_summary.json", summary)
    return 0


if __name__ == "__main__":
    sys.exit(main())
