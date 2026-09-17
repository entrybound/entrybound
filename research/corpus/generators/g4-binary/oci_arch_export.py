#!/usr/bin/env python3
"""Run step: export a single-architecture rootfs from a public OCI/Docker registry image, without
a Docker daemon (g4-binary F09 architecture-skew gap closure, 2026-09-17).

Contract: python oci_arch_export.py --out <staging> --seed N --params <JSON>

Uses `skopeo copy` (a plain read-only HTTPS registry client; no daemon, no privileged operations
beyond what any pull needs) to fetch the exact per-architecture manifest of a multi-arch image
into a scratch OCI layout, then extracts its layers (in order, applying OCI/Docker whiteout
semantics: `.wh.<name>` deletes `<name>` from the accumulated tree so far; `.wh..wh..opq` clears a
directory's prior contents) with GNU tar (`--xattrs`) so that xattrs such as
`security.capability` on setuid-adjacent binaries are preserved, matching what the existing
docker-export items (which do use a local Docker daemon) already do for other architectures.

params:
  repo:             registry repository, e.g. "ubuntu" or "fedora" (resolved against Docker Hub's
                     library/ namespace exactly like `docker pull <repo>`)
  manifest_digest:   the platform-specific manifest digest from the image's multi-arch index
                     (sha256:<hex>; not the index digest) -- this is what pins the content
  platform:          "linux/arm64" | "linux/riscv64" | ... (recorded for provenance only; the
                     manifest_digest is what is actually fetched)
The seed is unused (network content is fixed by the pinned digest).
"""

import argparse
import json
import os
import subprocess
import sys
from pathlib import Path

WHITEOUT_PREFIX = ".wh."
OPAQUE_WHITEOUT = ".wh..wh..opq"


def sh(argv, **kw):
    print("+ " + " ".join(str(a) for a in argv), flush=True)
    subprocess.run(argv, check=True, **kw)


def read_json(path):
    with open(path, "r", encoding="utf-8") as f:
        return json.load(f)


def blob_path(oci_dir: Path, digest: str) -> Path:
    algo, hexd = digest.split(":", 1)
    return oci_dir / "blobs" / algo / hexd


def list_tar_entries(layer_path: Path, decompress_cmd):
    r = subprocess.run(decompress_cmd + [str(layer_path)], stdout=subprocess.PIPE, check=True)
    names = subprocess.run(["tar", "-tf", "-"], input=r.stdout, stdout=subprocess.PIPE,
                            check=True).stdout.decode("utf-8", "surrogateescape").splitlines()
    return names


def apply_whiteouts(rootfs: Path, entries: list[str]) -> list[str]:
    """Remove whiteout targets from the accumulated rootfs; return tar --exclude patterns for the
    whiteout marker entries themselves (which must not be materialized)."""
    excludes = []
    for name in entries:
        base = os.path.basename(name.rstrip("/"))
        if not base.startswith(WHITEOUT_PREFIX):
            continue
        excludes.append(name)
        d = os.path.dirname(name.rstrip("/"))
        if base == OPAQUE_WHITEOUT:
            target = rootfs / d if d else rootfs
            if target.is_dir():
                for child in list(target.iterdir()):
                    sh(["rm", "-rf", "--", str(child)])
        else:
            victim = base[len(WHITEOUT_PREFIX):]
            target = (rootfs / d / victim) if d else (rootfs / victim)
            if target.exists() or target.is_symlink():
                sh(["rm", "-rf", "--", str(target)])
    return excludes


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True)
    ap.add_argument("--seed", type=int, default=0)
    ap.add_argument("--params", default="{}")
    args = ap.parse_args()
    params = json.loads(args.params)
    repo = params["repo"]
    digest = params["manifest_digest"]
    if not digest.startswith("sha256:"):
        sys.exit(f"oci_arch_export: manifest_digest must be sha256:<hex>, got {digest!r}")

    scratch = Path(os.environ["EB_SCRATCH"]) / "oci_arch_export"
    subprocess.run(["rm", "-rf", str(scratch)], check=True)
    scratch.mkdir(parents=True)
    oci_dir = scratch / "layout"
    ref = f"docker://{repo}@{digest}"
    sh(["skopeo", "copy", "--retry-times", "5", ref, f"oci:{oci_dir}:ref"])

    index = read_json(oci_dir / "index.json")
    man_digest = index["manifests"][0]["digest"]
    manifest = read_json(blob_path(oci_dir, man_digest))

    rootfs = Path(args.out)
    rootfs.mkdir(parents=True, exist_ok=True)
    for layer in manifest["layers"]:
        media = layer["mediaType"]
        path = blob_path(oci_dir, layer["digest"])
        if media.endswith("+gzip"):
            decompress = ["gzip", "-dc"]
        elif media.endswith("+zstd"):
            decompress = ["zstd", "-dc"]
        elif media.endswith(".tar"):
            decompress = ["cat"]
        else:
            sys.exit(f"oci_arch_export: unsupported layer mediaType {media!r}")
        entries = list_tar_entries(path, decompress)
        excludes = apply_whiteouts(rootfs, entries)
        tar_argv = ["tar", "--xattrs", "--xattrs-include=*", "-xf", "-", "-C", str(rootfs)]
        for e in excludes:
            tar_argv += ["--exclude", e]
        r = subprocess.run(decompress + [str(path)], stdout=subprocess.PIPE, check=True)
        subprocess.run(tar_argv, input=r.stdout, check=True)

    subprocess.run(["rm", "-rf", str(scratch)], check=True)
    print(f"oci_arch_export: extracted {repo}@{digest} ({len(manifest['layers'])} layer(s)) -> {rootfs}")


if __name__ == "__main__":
    main()
