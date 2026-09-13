#!/usr/bin/env python3
"""Fingerprint the pinned ubuntu:24.04 Docker container. Runs inside WSL Ubuntu.

Invoke through the wrapper:
  wsl.exe -d Ubuntu -- bash /mnt/d/Projects/entrybound/entrybound/research/tools/env/capture_docker.sh

Every step is idempotent and fails loudly on a digest or hash mismatch:
 1. Ensure the base image ubuntu@<index digest> is present (pull by digest if not)
    and that the local image really carries that digest.
 2. Fetch the raw OCI index and linux/amd64 manifest from the registry, check that
    they hash to the pinned digests, and record index/manifest/config/layer
    digests and sizes as download provenance (skipped when already recorded).
 3. Build the fingerprint image from docker/ubuntu-24.04-env.Dockerfile, skipped when
    an image with the same recipe label already exists.
 4. Extract the apt .deb provenance (URL, size, SHA-256) from the image and compare
    it with research/environment/provenance/docker-ubuntu-24.04.downloads.json.
 5. Run capture_env.py inside the container (network off, repo mounted read-only)
    and store the verified fingerprint in research/environment/.

Standard library only (Python 3.12 in WSL). The docker CLI here is Docker
Desktop's docker.exe reached through WSL interop, so bind-mount sources are
translated with `wslpath -w`.
"""

from __future__ import annotations

import datetime as _dt
import hashlib
import io
import json
import re
import shutil
import subprocess
import sys
import tarfile
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]
sys.dont_write_bytecode = True  # keep __pycache__ out of the repository tree
sys.path.insert(0, str(HERE))
import capture_env as ce  # noqa: E402

ENV_NAME = "docker-ubuntu-24.04"
PARENT_NAME = "windows-host"

BASE_REPOSITORY = "docker.io/library/ubuntu"
BASE_TAG = "24.04"
BASE_INDEX_DIGEST = "sha256:224a1869083a311ef3f13648a154ba79832fbef6364d31493642ca03082da254"
BASE_AMD64_MANIFEST_DIGEST = "sha256:a61567bd31828687156d735ea8eb01ba4e37636e225dd6a48ba94136a70d9d61"
BASE_PINNED = f"{BASE_REPOSITORY}:{BASE_TAG}@{BASE_INDEX_DIGEST}"
BASE_BY_DIGEST = f"ubuntu@{BASE_INDEX_DIGEST}"
REGISTRY_API = "https://registry-1.docker.io/v2/library/ubuntu"

APT_SNAPSHOT = "20260912T000000Z"
PACKAGES = ["python3"]
DERIVED_TAG = "entrybound-research/env-ubuntu-24.04:snapshot-20260912"
DOCKERFILE = HERE / "docker" / "ubuntu-24.04-env.Dockerfile"
TRANSPORT_CA = Path("/etc/ssl/certs/ca-certificates.crt")
RUN_FLAGS = "rm;network=none;platform=linux/amd64;bind=repo:/repo:ro"

OUT_DIR = REPO / "research" / "environment"
PROVENANCE = OUT_DIR / "provenance" / f"{ENV_NAME}.downloads.json"
PROVENANCE_SCHEMA = "entrybound.research.download-provenance/1"
LABEL_RECIPE = "org.entrybound.research.recipe-sha256"
LABEL_CA = "org.entrybound.research.transport-ca-sha256"


def fail(msg: str) -> "NoReturn":  # noqa: F821
    print(f"ERROR: {msg}", file=sys.stderr)
    raise SystemExit(2)


DOCKER = shutil.which("docker")


def docker(*args, input_bytes=None, timeout=900, check=True):
    if not DOCKER:
        fail("docker CLI not found on PATH")
    proc = subprocess.run(
        [DOCKER, *args],
        input=input_bytes,
        stdin=None if input_bytes is not None else subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        timeout=timeout,
    )
    if check and proc.returncode != 0:
        tail = proc.stderr.decode("utf-8", "replace").strip()[-3000:]
        fail(f"docker {' '.join(args[:2])} failed (rc={proc.returncode}):\n{tail}")
    return proc


def sha256_hex(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def verified_raw(ref: str, digest: str) -> bytes:
    raw = docker("buildx", "imagetools", "inspect", "--raw", ref, timeout=300).stdout
    for candidate in (raw, raw.rstrip(b"\n"), raw.rstrip(b"\r\n")):
        if "sha256:" + sha256_hex(candidate) == digest:
            return candidate
    fail(f"HASH MISMATCH: raw manifest for {ref} hashes to sha256:{sha256_hex(raw)}, expected {digest}")


def ensure_base_image():
    if docker("image", "inspect", BASE_BY_DIGEST, check=False).returncode != 0:
        print(f"pulling {BASE_PINNED}", file=sys.stderr)
        docker("pull", "--platform", "linux/amd64", BASE_BY_DIGEST, timeout=1800)
    info = json.loads(docker("image", "inspect", BASE_BY_DIGEST).stdout)[0]
    digests = info.get("RepoDigests") or []
    if not any(d.endswith("@" + BASE_INDEX_DIGEST) for d in digests) and info.get("Id") != BASE_INDEX_DIGEST:
        fail(f"local image for {BASE_BY_DIGEST} does not carry the pinned digest (RepoDigests={digests})")
    return info


def oci_downloads():
    index_raw = verified_raw(BASE_BY_DIGEST, BASE_INDEX_DIGEST)
    index = json.loads(index_raw)
    amd64 = [
        m for m in index.get("manifests", [])
        if (m.get("platform") or {}).get("os") == "linux" and (m.get("platform") or {}).get("architecture") == "amd64"
    ]
    if [m.get("digest") for m in amd64] != [BASE_AMD64_MANIFEST_DIGEST]:
        fail(f"HASH MISMATCH: index lists linux/amd64 manifests {[m.get('digest') for m in amd64]}, "
             f"expected [{BASE_AMD64_MANIFEST_DIGEST}]")
    manifest_raw = verified_raw(f"ubuntu@{BASE_AMD64_MANIFEST_DIGEST}", BASE_AMD64_MANIFEST_DIGEST)
    manifest = json.loads(manifest_raw)
    items = [
        {"kind": "oci-image-index", "url": f"{REGISTRY_API}/manifests/{BASE_INDEX_DIGEST}",
         "reference": BASE_PINNED, "media_type": index.get("mediaType"),
         "size_bytes": len(index_raw), "sha256": BASE_INDEX_DIGEST[7:]},
        {"kind": "oci-image-manifest", "platform": "linux/amd64",
         "url": f"{REGISTRY_API}/manifests/{BASE_AMD64_MANIFEST_DIGEST}", "media_type": manifest.get("mediaType"),
         "size_bytes": len(manifest_raw), "sha256": BASE_AMD64_MANIFEST_DIGEST[7:]},
    ]
    cfg = manifest.get("config") or {}
    items.append({"kind": "oci-image-config", "url": f"{REGISTRY_API}/blobs/{cfg.get('digest')}",
                  "media_type": cfg.get("mediaType"), "size_bytes": cfg.get("size"),
                  "sha256": str(cfg.get("digest", ""))[7:]})
    for layer in manifest.get("layers") or []:
        items.append({"kind": "oci-image-layer", "url": f"{REGISTRY_API}/blobs/{layer.get('digest')}",
                      "media_type": layer.get("mediaType"), "size_bytes": layer.get("size"),
                      "sha256": str(layer.get("digest", ""))[7:]})
    return items


def recipe():
    dockerfile_lf = DOCKERFILE.read_bytes().replace(b"\r\n", b"\n")
    text = dockerfile_lf.decode("utf-8")
    expected = {"BASE_IMAGE": BASE_PINNED, "APT_SNAPSHOT": APT_SNAPSHOT, "PACKAGES": " ".join(PACKAGES)}
    for arg, value in expected.items():
        m = re.search(rf"^ARG {arg}=(.*)$", text, re.M)
        if not m or m.group(1).strip() != value:
            fail(f"{DOCKERFILE.name}: ARG {arg} default {m.group(1).strip() if m else None!r} != pinned {value!r}")
    spec = {
        "dockerfile_sha256_lf": sha256_hex(dockerfile_lf),
        "base_image": BASE_PINNED,
        "platform": "linux/amd64",
        "apt_snapshot": APT_SNAPSHOT,
        "packages": PACKAGES,
    }
    return dockerfile_lf, spec, sha256_hex(ce.canonical_json(spec))


def image_labels(tag):
    proc = docker("image", "inspect", tag, check=False)
    if proc.returncode != 0:
        return None
    info = json.loads(proc.stdout)[0]
    return (info.get("Config") or {}).get("Labels") or {}, info.get("Id")


def ensure_derived_image(dockerfile_lf: bytes, recipe_sha: str):
    found = image_labels(DERIVED_TAG)
    if found and found[0].get(LABEL_RECIPE) == recipe_sha:
        print(f"skip build: {DERIVED_TAG} already built from recipe {recipe_sha[:12]}", file=sys.stderr)
        return found[1], found[0].get(LABEL_CA)
    if not TRANSPORT_CA.is_file():
        fail(f"{TRANSPORT_CA} missing (install ca-certificates in WSL)")
    ca = TRANSPORT_CA.read_bytes()
    buf = io.BytesIO()
    with tarfile.open(fileobj=buf, mode="w", format=tarfile.PAX_FORMAT) as tf:
        for name, data in (("Dockerfile", dockerfile_lf), ("ca-certificates.crt", ca)):
            ti = tarfile.TarInfo(name)
            ti.size, ti.mtime, ti.mode = len(data), 0, 0o644
            tf.addfile(ti, io.BytesIO(data))
    print(f"building {DERIVED_TAG} (recipe {recipe_sha[:12]}, apt snapshot {APT_SNAPSHOT})", file=sys.stderr)
    proc = docker(
        "build", "--platform", "linux/amd64", "--progress", "plain",
        "--build-arg", f"BASE_IMAGE={BASE_PINNED}",
        "--build-arg", f"APT_SNAPSHOT={APT_SNAPSHOT}",
        "--build-arg", f"PACKAGES={' '.join(PACKAGES)}",
        "--label", f"{LABEL_RECIPE}={recipe_sha}",
        "--label", f"{LABEL_CA}={sha256_hex(ca)}",
        "-t", DERIVED_TAG, "-",
        input_bytes=buf.getvalue(), timeout=3600,
    )
    sys.stderr.write(proc.stderr.decode("utf-8", "replace")[-2000:] + "\n")
    found = image_labels(DERIVED_TAG)
    if not found or found[0].get(LABEL_RECIPE) != recipe_sha:
        fail("built image does not carry the expected recipe label")
    return found[1], found[0].get(LABEL_CA)


def image_file(path: str) -> str:
    return docker("run", "--rm", "--network", "none", "--entrypoint", "cat", DERIVED_TAG, path).stdout.decode("utf-8")


def deb_downloads():
    uris = {}
    for line in image_file("/eb-provenance/apt-print-uris.txt").splitlines():
        m = re.match(r"^'([^']+)'\s+(\S+)\s+(\d+)\s+(\S+?):(\S+)$", line.strip())
        if m:
            uris[m.group(2)] = {"url": m.group(1), "size": int(m.group(3)), "index_hash": f"{m.group(4)}:{m.group(5)}"}
    debs = []
    for line in image_file("/eb-provenance/apt-debs.tsv").splitlines():
        if not line.strip():
            continue
        name, size, digest = line.split("\t")
        u = uris.get(name)
        if not u:
            fail(f"downloaded {name} not listed by apt --print-uris")
        if u["size"] != int(size):
            fail(f"SIZE MISMATCH for {name}: index says {u['size']}, file has {size}")
        if not u["url"].startswith(f"https://snapshot.ubuntu.com/ubuntu/{APT_SNAPSHOT}/"):
            fail(f"{name} was not fetched from the pinned snapshot: {u['url']}")
        debs.append({"kind": "deb", "url": u["url"], "filename": name, "size_bytes": int(size),
                     "sha256": digest, "apt_index_hash": u["index_hash"]})
    if set(uris) != {d["filename"] for d in debs}:
        fail(f"apt --print-uris / downloaded set differ: {sorted(set(uris) ^ {d['filename'] for d in debs})}")
    installed = [ln.split("\t") for ln in image_file("/eb-provenance/dpkg-installed.tsv").splitlines() if ln.strip()]
    return debs, installed


def reconcile_provenance(downloads, ca_sha):
    fresh_map = {d["url"]: (d["sha256"], d["size_bytes"]) for d in downloads}
    if PROVENANCE.exists():
        old = json.loads(PROVENANCE.read_text(encoding="utf-8"))
        old_map = {d["url"]: (d["sha256"], d["size_bytes"]) for d in old.get("downloads", [])}
        if old_map != fresh_map:
            diff = sorted(set(old_map.items()) ^ set(fresh_map.items()))
            fail(f"HASH MISMATCH against {PROVENANCE}:\n" + "\n".join(map(str, diff)))
        print(f"provenance unchanged: {PROVENANCE.name} ({len(fresh_map)} downloads)", file=sys.stderr)
        return
    doc = {
        "schema": PROVENANCE_SCHEMA,
        "environment_name": ENV_NAME,
        "retrieval_date": _dt.date.today().isoformat(),
        "retrieved_by": "research/tools/env/capture_docker.py",
        "downloads": sorted(downloads, key=lambda d: (d["kind"], d["url"])),
        "transport_ca_bundle": {
            "source": "WSL Ubuntu /etc/ssl/certs/ca-certificates.crt (build-time bind mount only)",
            "sha256": ca_sha,
            "note": "TLS transport only; .deb integrity is enforced by apt InRelease signatures "
                    "(ubuntu-archive-keyring from the pinned base image) and the SHA-256 values above.",
        },
    }
    PROVENANCE.parent.mkdir(parents=True, exist_ok=True)
    PROVENANCE.write_text(json.dumps(doc, indent=2) + "\n", encoding="utf-8", newline="\n")
    print(f"provenance written: {PROVENANCE}", file=sys.stderr)


def container_clock_skew():
    before = time.time()
    out = docker("run", "--rm", "--network", "none", "--entrypoint", "date", DERIVED_TAG, "-u", "+%s.%N").stdout
    after = time.time()
    try:
        return round(float(out.decode().strip()) - (before + after) / 2, 1), round(after - before, 1)
    except ValueError:
        return None, None


def main():
    ensure_base_image()
    oci = None
    if PROVENANCE.exists():
        recorded = json.loads(PROVENANCE.read_text(encoding="utf-8")).get("downloads", [])
        oci = [d for d in recorded if d["kind"].startswith("oci-")]
        if not any(d["kind"] == "oci-image-index" and d["sha256"] == BASE_INDEX_DIGEST[7:] for d in oci):
            oci = None
    if oci is None:
        oci = oci_downloads()
    dockerfile_lf, recipe_spec, recipe_sha = recipe()
    image_id, ca_sha = ensure_derived_image(dockerfile_lf, recipe_sha)
    debs, installed = deb_downloads()
    reconcile_provenance(oci + debs, ca_sha)

    parent = ce.lookup_field(OUT_DIR, PARENT_NAME, "platform_id")
    win_repo = subprocess.run(["wslpath", "-w", str(REPO)], capture_output=True, text=True, check=True).stdout.strip()
    run_args = [
        "run", "--rm", "--network", "none", "--platform", "linux/amd64",
        "--mount", f"type=bind,source={win_repo},target=/repo,readonly",
        DERIVED_TAG,
        "python3", "/repo/research/tools/env/capture_env.py", "capture",
        "--name", ENV_NAME, "--repo", "/repo", "--work-dir", "/repo", "--work-dir", "/tmp",
        "--parent-platform-id", parent,
        "--container-base-image", BASE_PINNED,
        "--container-platform-manifest", BASE_AMD64_MANIFEST_DIGEST,
        "--container-apt-snapshot", APT_SNAPSHOT,
        "--container-packages", ",".join(PACKAGES),
        "--container-recipe-sha256", recipe_sha,
        "--container-run-flags", RUN_FLAGS,
        "--stdout",
    ]
    proc = docker(*run_args, timeout=900)
    sys.stderr.write(proc.stderr.decode("utf-8", "replace"))
    doc = json.loads(proc.stdout.decode("utf-8"))
    skew, spread = container_clock_skew()
    doc["capture"]["docker_driver"] = ce.normalize({
        "driver": "research/tools/env/capture_docker.py",
        "derived_image_tag": DERIVED_TAG,
        "derived_image_id": image_id,
        "recipe": recipe_spec,
        "python3_package": next((f"{p}={v}" for p, v, _ in installed if p == "python3"), None),
        "container_clock_minus_wsl_seconds": skew,
        "clock_probe_roundtrip_seconds": spread,
        "docker_run_args": " ".join(run_args[:9]),
    })
    env_id, status = ce.store_document(doc, ENV_NAME, OUT_DIR)
    print(f"{ENV_NAME}: env_id={env_id} platform_id={doc['platform_id']} [{status}]", file=sys.stderr)
    for obs in doc["capture"].get("observations", []):
        print(f"  observation: {obs}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except ce.FingerprintError as exc:
        fail(str(exc))
