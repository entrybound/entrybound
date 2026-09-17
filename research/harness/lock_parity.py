"""Check that the research harness resolves entrybound's dependency graph to
the same crate versions production does.

research/harness is its own Cargo workspace with its own Cargo.lock, so
nothing forces the transitive dependencies of crates/entrybound (zstd-sys,
zstd-safe, cc, smallvec, ...) to resolve to the versions the production
workspace's Cargo.lock pins. A drift there means harness "production"
measurements run a different dependency build than the shipped CLI, which
the research-internals identity proof (run in the production workspace)
does not cover. See harness-review-round1.md finding R1-06.

Usage (Python >= 3.11, standard library only):

  python research/harness/lock_parity.py            # report; exit 1 on drift
  python research/harness/lock_parity.py --commands # also print cargo update commands

The comparison walks the normal-dependency closure of the `entrybound`
package in each lock file. In the production lock, entrybound's entry also
lists its dev-dependencies (it is a workspace member there), so crates named
only under crates/entrybound/Cargo.toml [dev-dependencies] are excluded
from the production walk's roots. A package reached in both closures must
resolve to the same version set; a package reached in only one closure is
reported for information (it cannot change entrybound's own code paths
unless a feature edge is involved) but is not a failure.
"""

import argparse
import pathlib
import sys
import tomllib

REPO = pathlib.Path(__file__).resolve().parents[2]
PROD_LOCK = REPO / "Cargo.lock"
HARNESS_LOCK = REPO / "research" / "harness" / "Cargo.lock"
ENTRYBOUND_TOML = REPO / "crates" / "entrybound" / "Cargo.toml"


def load_lock(path):
    data = tomllib.loads(path.read_text(encoding="utf-8"))
    by_name = {}
    for package in data["package"]:
        by_name.setdefault(package["name"], []).append(package)
    return by_name


def resolve_ref(by_name, ref):
    parts = ref.split(" ")
    name = parts[0]
    candidates = by_name.get(name, [])
    if len(parts) >= 2:
        candidates = [p for p in candidates if p["version"] == parts[1]]
    if len(candidates) != 1:
        raise SystemExit(f"cannot resolve lock reference {ref!r}: {len(candidates)} matches")
    return candidates[0]


def closure(by_name, root_name, skip_root_deps=frozenset()):
    roots = [p for p in by_name.get(root_name, []) if "source" not in p]
    if len(roots) != 1:
        raise SystemExit(f"expected exactly one path package named {root_name}")
    seen = {}
    stack = [
        resolve_ref(by_name, ref)
        for ref in roots[0].get("dependencies", [])
        if ref.split(" ")[0] not in skip_root_deps
    ]
    while stack:
        package = stack.pop()
        key = (package["name"], package["version"])
        if key in seen:
            continue
        seen[key] = package
        for ref in package.get("dependencies", []):
            stack.append(resolve_ref(by_name, ref))
    versions = {}
    for name, version in seen:
        versions.setdefault(name, set()).add(version)
    return versions


def dev_only_dependencies():
    manifest = tomllib.loads(ENTRYBOUND_TOML.read_text(encoding="utf-8"))
    normal = set(manifest.get("dependencies", {}))
    for target in manifest.get("target", {}).values():
        normal |= set(target.get("dependencies", {}))
    dev = set(manifest.get("dev-dependencies", {}))
    return frozenset(dev - normal)


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--commands", action="store_true", help="print cargo update --precise commands")
    args = parser.parse_args()

    production = closure(load_lock(PROD_LOCK), "entrybound", dev_only_dependencies())
    harness = closure(load_lock(HARNESS_LOCK), "entrybound")

    drift = []
    for name in sorted(set(production) & set(harness)):
        if production[name] != harness[name]:
            drift.append((name, sorted(production[name]), sorted(harness[name])))
    only_production = sorted(set(production) - set(harness))
    only_harness = sorted(set(harness) - set(production))

    print(f"entrybound normal-dependency closure: production {len(production)} crates, harness {len(harness)} crates")
    if only_production:
        print("reached only in the production closure (informational): " + ", ".join(only_production))
    if only_harness:
        print("reached only in the harness closure (informational): " + ", ".join(only_harness))
    for name, prod_versions, harness_versions in drift:
        print(f"DRIFT {name}: production {prod_versions} harness {harness_versions}")
        if args.commands and len(prod_versions) == 1 and len(harness_versions) == 1:
            print(
                f"  cargo +1.98.1 update --manifest-path research/harness/Cargo.toml "
                f"-p {name}@{harness_versions[0]} --precise {prod_versions[0]}"
            )
    print("RESULT " + ("PASS" if not drift else f"FAIL ({len(drift)} drifted crates)"))
    return 0 if not drift else 1


if __name__ == "__main__":
    sys.exit(main())
