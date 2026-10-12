"""Independent empty bootstrap identity transcript oracle, docs/format-v0.md.

Test-only reference. Never imports Entrybound or derives a golden from a round
trip. Optional .eb inputs verify exact-byte PCI with Python's SHA-256 backend.
"""
import argparse
import hashlib
import itertools
import json
import re
from pathlib import Path


def u64(value):
    return value.to_bytes(8, "big")


def digest(domain, fields):
    domain = domain.encode("ascii")
    transcript = b"Entrybound hash v1\x00" + u64(len(domain)) + domain + u64(len(fields))
    transcript += b"".join(u64(len(field)) + field for field in fields)
    return hashlib.sha256(transcript).digest()


def empty_roots():
    manifest = digest("manifest/empty", [])
    # Complete role=1, empty EntrySet and ContentStore, fixed-1mib/v1;
    # empty captured/unavailable/degraded/filesystem lists, platform="test".
    fidelity_bytes = u64(0) * 3 + u64(4) + b"test" + u64(0)
    return {
        "lai": digest("lai/v1", [b"sha256", manifest, b"identity/v1", b"\x01", u64(0), u64(0)]).hex(),
        "pcr": digest("pcr/v1", [b"sha256", u64(0), u64(0), b"fixed-1mib/v1"]).hex(),
        "aux": digest("aux/v1", [b"sha256", digest("aux-manifest/empty", []), digest("fidelity/v1", [fidelity_bytes]), digest("conversion/absent/v1", [])]).hex(),
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("files", nargs="*", type=Path)
    parser.add_argument("--matrix-log", type=Path)
    parser.add_argument("--fixture-root", type=Path)
    args = parser.parse_args()
    result = {"empty_roots": empty_roots(), "pci": []}
    expected = {}
    if args.matrix_log:
        if not args.fixture_root:
            parser.error("--matrix-log requires --fixture-root")
        text = args.matrix_log.read_text(encoding="utf-8")
        root = args.fixture_root.resolve(strict=True)
        cases = re.findall(r"MATRIX_CASE (\d+) profile=(\w+) padding=(\w+) boundary=(\w+) index=(true|false) password=(true|false) files=14 exact_bytes=true metadata=true wrong_key_refused=true truncation_refused=true", text)
        wanted = set(itertools.product(["Fast", "Balanced", "Dense", "Extreme"], ["None", "Bucketed", "Maximum"], ["SecretGearTable", "PhteAes128"], ["false", "true"], ["false", "true"]))
        if len(cases) != 96 or {int(c[0]) for c in cases} != set(range(96)) or {tuple(c[1:]) for c in cases} != wanted:
            raise ValueError("matrix membership must contain every exact configuration once")
        for name, value in re.findall(r"MATRIX_PCI (.+) ([0-9a-f]{64})", text):
            path = Path(name)
            if path.is_symlink() or not path.resolve(strict=True).is_relative_to(root):
                raise ValueError("PCI file outside the declared generated fixture root")
            if path in expected:
                raise ValueError("duplicate PCI row")
            expected[path] = value
        if len(expected) != 98:
            raise ValueError("expected 96 encrypted and two empty plaintext PCI rows")
        result["configuration_matrix"] = {"cases": 96, "files_per_case": 14, "membership_exact": True}
    for path in [*args.files, *expected]:
        hasher = hashlib.sha256()
        size = 0
        with path.open("rb") as handle:
            while block := handle.read(1 << 20):
                hasher.update(block)
                size += len(block)
        value = hasher.hexdigest()
        if path in expected and value != expected[path]:
            raise ValueError("independent PCI calculation differs: " + str(path))
        result["pci"].append({"path": str(path), "bytes": size, "pci": value, "expected_pci": expected.get(path)})
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
