#!/usr/bin/env python3
"""Build step: materialize an OCI image layout directory (g4-binary F16 gap closure, 2026-09-17).

Contract: python build_oci_layout.py --out <staging> --seed N --params <JSON>

No Docker daemon is used anywhere; `skopeo` is a plain read-only HTTPS registry client.

Two modes:

mode "copy": a straight `skopeo copy` of a real, digest-pinned public image into an OCI image
  layout at the item root (oci-layout file, index.json, blobs/sha256/*).  params:
    repo:        Docker Hub library repo, e.g. "ubuntu" or "fedora"
    ref_digest:  sha256:<hex> -- an INDEX digest (multi_arch true) or a single-platform manifest
                 digest (multi_arch false); this is what pins the content
    multi_arch:  true copies the whole index (every listed platform's manifest+layers: this is
                 what gives a real multi-arch OCI index); false copies one platform's image
    compress:    "gzip" (as published) or "zstd" (skopeo re-encodes each layer's compression;
                 content is unchanged, only the on-disk representation is)

mode "whiteout-derive": start from a real single-platform image (by digest, via `skopeo copy`),
  then append one synthetic layer that (a) writes one new small file and (b) whiteouts one
  existing path from the base layer(s), producing a genuine 2-layer OCI image with real base
  content plus an OCI/Docker-style opaque/whiteout entry.  params:
    repo, ref_digest:  the real single-platform base image (as in "copy" mode, multi_arch false)
    add_path:          repo-relative path (inside the image) for the new file the extra layer adds
    add_text:          its (small, deterministic) text content
    whiteout_path:     an existing path (from the base layer) that the extra layer deletes

Deterministic: every byte written (blob digests, index.json, timestamps embedded in the synthetic
layer/config) is fixed by the pinned input digest(s) plus these params, so the output can be
TOFU-pinned.
"""

import argparse
import gzip
import hashlib
import io
import json
import os
import shutil
import subprocess
import sys
import tarfile
from pathlib import Path

FIXED_MTIME = 1735689600  # 2025-01-01T00:00:00Z; used only for the synthetic whiteout layer


def sh(argv, **kw):
    print("+ " + " ".join(str(a) for a in argv), flush=True)
    subprocess.run(argv, check=True, **kw)


def read_json(p):
    with open(p, "r", encoding="utf-8") as f:
        return json.load(f)


def write_json(p, obj):
    p.parent.mkdir(parents=True, exist_ok=True)
    data = json.dumps(obj, separators=(",", ":"), sort_keys=True).encode("utf-8")
    p.write_bytes(data)
    return data


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def blob_path(root: Path, digest: str) -> Path:
    algo, hexd = digest.split(":", 1)
    return root / "blobs" / algo / hexd


def put_blob(root: Path, data: bytes) -> tuple[str, int]:
    digest = "sha256:" + sha256_bytes(data)
    p = blob_path(root, digest)
    p.parent.mkdir(parents=True, exist_ok=True)
    if not p.exists():
        p.write_bytes(data)
    return digest, len(data)


def skopeo_copy(repo: str, ref_digest: str, dest: Path, multi_arch: bool, compress: str) -> None:
    dest.mkdir(parents=True, exist_ok=True)
    argv = ["skopeo", "copy", "--retry-times", "5"]
    if multi_arch:
        argv += ["--multi-arch", "all"]
    if compress == "zstd":
        argv += ["--dest-compress-format", "zstd"]
    elif compress != "gzip":
        sys.exit(f"build_oci_layout: unsupported compress {compress!r}")
    argv += [f"docker://{repo}@{ref_digest}", f"oci:{dest}:ref"]
    sh(argv)
    write_json(dest / "oci-layout", {"imageLayoutVersion": "1.0.0"})


def mode_copy(out: Path, p: dict) -> None:
    skopeo_copy(p["repo"], p["ref_digest"], out, bool(p.get("multi_arch", False)), p.get("compress", "gzip"))


def mode_whiteout_derive(out: Path, scratch: Path, p: dict) -> None:
    base = scratch / "base"
    if base.exists():
        shutil.rmtree(base)
    skopeo_copy(p["repo"], p["ref_digest"], base, multi_arch=False, compress="gzip")
    base_index = read_json(base / "index.json")
    base_man_digest = base_index["manifests"][0]["digest"]
    base_manifest = read_json(blob_path(base, base_man_digest))
    base_config_digest = base_manifest["config"]["digest"]
    base_config = read_json(blob_path(base, base_config_digest))

    out.mkdir(parents=True, exist_ok=True)
    # Copy every blob the base image references (config + all its layers) into the new layout
    # unmodified, so the new image's lower layers are byte-identical to the real base image.
    for layer in base_manifest["layers"]:
        src = blob_path(base, layer["digest"])
        dst = blob_path(out, layer["digest"])
        dst.parent.mkdir(parents=True, exist_ok=True)
        if not dst.exists():
            shutil.copyfile(src, dst)

    # Build the extra layer: one added file, one whiteout of an existing base path.
    add_path = p["add_path"].strip("/")
    whiteout_path = p["whiteout_path"].strip("/")
    wdir, wname = os.path.split(whiteout_path)
    wh_entry_name = (f"{wdir}/.wh.{wname}" if wdir else f".wh.{wname}")
    buf = io.BytesIO()
    with tarfile.open(fileobj=buf, mode="w", format=tarfile.PAX_FORMAT) as tf:
        data = p["add_text"].encode("utf-8")
        ti = tarfile.TarInfo(add_path)
        ti.size = len(data)
        ti.mtime = FIXED_MTIME
        ti.mode = 0o644
        ti.uid = ti.gid = 0
        tf.addfile(ti, io.BytesIO(data))
        wi = tarfile.TarInfo(wh_entry_name)
        wi.size = 0
        wi.mtime = FIXED_MTIME
        wi.mode = 0o644
        wi.uid = wi.gid = 0
        tf.addfile(wi, io.BytesIO(b""))
    tar_bytes = buf.getvalue()
    gz_buf = io.BytesIO()
    with gzip.GzipFile(fileobj=gz_buf, mode="wb", mtime=FIXED_MTIME) as gz:
        gz.write(tar_bytes)
    layer_data = gz_buf.getvalue()
    layer_digest, layer_size = put_blob(out, layer_data)
    diff_id = "sha256:" + sha256_bytes(tar_bytes)

    new_config = dict(base_config)
    rootfs = dict(new_config.get("rootfs", {"type": "layers", "diff_ids": []}))
    rootfs["diff_ids"] = list(rootfs.get("diff_ids", [])) + [diff_id]
    new_config["rootfs"] = rootfs
    new_config["history"] = list(new_config.get("history", [])) + [
        {"created": "2026-09-17T00:00:00Z",
         "comment": f"entrybound-research: synthetic layer adding {add_path} and whiting out {whiteout_path}"}]
    config_bytes = write_json(scratch / "new_config.json", new_config)
    config_digest, config_size = put_blob(out, config_bytes)

    new_manifest = {
        "schemaVersion": 2,
        "mediaType": "application/vnd.oci.image.manifest.v1+json",
        "config": {"mediaType": "application/vnd.oci.image.config.v1+json", "size": config_size,
                   "digest": config_digest},
        "layers": base_manifest["layers"] + [
            {"mediaType": "application/vnd.oci.image.layer.v1.tar+gzip", "size": layer_size,
             "digest": layer_digest}],
    }
    manifest_bytes = write_json(scratch / "new_manifest.json", new_manifest)
    manifest_digest, manifest_size = put_blob(out, manifest_bytes)

    index = {"schemaVersion": 2, "mediaType": "application/vnd.oci.image.index.v1+json",
             "manifests": [{"mediaType": "application/vnd.oci.image.manifest.v1+json",
                            "size": manifest_size, "digest": manifest_digest,
                            "annotations": {"org.opencontainers.image.ref.name": "ref"}}]}
    write_json(out / "index.json", index)
    write_json(out / "oci-layout", {"imageLayoutVersion": "1.0.0"})
    print(f"build_oci_layout: whiteout-derive base={base_man_digest} -> {manifest_digest} "
          f"(2 layers, added {add_path}, whited out {whiteout_path})")


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True)
    ap.add_argument("--seed", type=int, default=0)
    ap.add_argument("--params", default="{}")
    args = ap.parse_args()
    p = json.loads(args.params)
    out = Path(args.out)
    scratch = Path(os.environ["EB_SCRATCH"]) / "build_oci_layout"
    if scratch.exists():
        shutil.rmtree(scratch)
    scratch.mkdir(parents=True)
    mode = p.get("mode", "copy")
    if mode == "copy":
        mode_copy(out, p)
    elif mode == "whiteout-derive":
        mode_whiteout_derive(out, scratch, p)
    else:
        sys.exit(f"build_oci_layout: unknown mode {mode!r}")
    shutil.rmtree(scratch, ignore_errors=True)


if __name__ == "__main__":
    main()
