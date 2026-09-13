#!/usr/bin/env python3
"""Non-JPEG image container/codec variants from lossless images (g5-media, F13 "other media").

ebrc build script (research/corpus/methodology.md 5.4):
    python image_codecs.py --out <staging> --seed N --params <canonical JSON>
Inputs: EB_INPUTS_JSON {input name -> verified cached blob (PNG/TIFF)}.  Output is a pure function
of (script, seed, params, input bytes, exact tool versions checked via params.require_tools).
Self-contained on purpose (no import of jpeg_matrix.py) so the materialization key covers every
line of code that shapes the bytes.

Real photo folders increasingly hold WebP/AVIF/HEIC/JPEG XL, PNG/TIFF/GIF exports and JPEG XL
"lossless JPEG transcodes"; public-domain real samples of AVIF/HEIC/JXL are scarce, so these are
encoded from real lossless sources and labelled derived-from-real.

params
  inputs        optional list of input names (default all, sorted)
  tiles         optional {"count": n, "size": [w, h], "reduce": [..]} (as jpeg_matrix.py)
  reduce        optional int box-downscale when tiles is absent
  max_inputs    optional int
  formats       list of format ids (FORMATS below)
  big_formats_max_inputs  optional int: uncompressed formats (bmp, tiff-none) only for the first n images
  require_tools {tool: expected version substring}: cwebp avifenc heif-enc cjxl imagemagick pillow ffmpeg tiffcp cjpeg
  max_bytes     fail if output exceeds this
"""

from __future__ import annotations

import argparse
import json
import os
import random
import subprocess
from pathlib import Path

from PIL import Image

EPOCH = int(os.environ.get("SOURCE_DATE_EPOCH", "1767225600"))
SCRATCH = Path(os.environ.get("EB_SCRATCH", "/tmp"))
Image.MAX_IMAGE_PIXELS = 400_000_000
BIG = {"bmp", "tiff-none"}


def _out(cmd):
    r = subprocess.run(cmd, capture_output=True, text=True, errors="replace")
    return (r.stdout + r.stderr).strip()


def tool_version(tool):
    cmds = {
        "cwebp": ["cwebp", "-version"], "avifenc": ["avifenc", "--version"], "heif-enc": ["heif-enc", "--version"],
        "cjxl": ["cjxl", "--version"], "imagemagick": ["convert", "-version"], "ffmpeg": ["ffmpeg", "-hide_banner", "-version"],
        "tiffcp": ["tiffcp"], "cjpeg": ["cjpeg", "-version"],
    }
    if tool == "pillow":
        return f"Pillow {Image.__version__}"
    if tool in cmds:
        return (_out(cmds[tool]).splitlines() or [""])[0]
    if tool.startswith("pkg:"):
        return tool + " " + _out(["dpkg-query", "-W", "-f=${Version}", tool[4:]])
    raise SystemExit(f"unknown tool {tool}")


def check_tools(req):
    for tool, expect in sorted(req.items()):
        got = tool_version(tool)
        print(f"tool {tool}: {got}")
        if expect not in got:
            raise SystemExit(f"TOOL VERSION MISMATCH: {tool} is '{got}', expected to contain '{expect}'")


def run(cmd):
    env = dict(os.environ, MAGICK_THREAD_LIMIT="1", OMP_NUM_THREADS="1")
    r = subprocess.run(cmd, capture_output=True, env=env)
    if r.returncode != 0:
        raise SystemExit(f"command failed ({r.returncode}): {' '.join(map(str, cmd))}\n{r.stderr.decode(errors='replace')[-2000:]}")


def load_rgb(path):
    im = Image.open(path)
    im.load()
    if im.mode in ("I;16", "I;16B", "I"):
        im = im.point(lambda v: v / 256).convert("L")
    return im.convert("RGB")


def prepare(params, seed):
    inputs = json.loads(os.environ["EB_INPUTS_JSON"])
    names = params.get("inputs") or sorted(inputs)
    tiles = params.get("tiles")
    out = []
    for name in names:
        im = load_rgb(inputs[name])
        if tiles:
            rng = random.Random(f"{seed}:{name}:codec-tiles")
            tw, th = tiles["size"]
            reduces = tiles.get("reduce", [1])
            for t in range(tiles["count"]):
                r = reduces[t % len(reduces)]
                w, h = (tw, th) if t % 2 == 0 else (th, tw)
                cw, ch = min(w * r, im.width), min(h * r, im.height)
                x, y = rng.randrange(0, im.width - cw + 1), rng.randrange(0, im.height - ch + 1)
                tile = im.crop((x, y, x + cw, y + ch))
                out.append((f"{name}-t{t}", tile.reduce(r) if r > 1 else tile))
        else:
            r = int(params.get("reduce", 1))
            out.append((name, im.reduce(r) if r > 1 else im))
    if params.get("max_inputs"):
        out = out[: int(params["max_inputs"])]
    res = []
    for name, im in out:
        png = SCRATCH / f"{name}.png"
        ppm = SCRATCH / f"{name}.ppm"
        im.save(png, format="PNG", compress_level=1)
        im.save(ppm, format="PPM")
        res.append({"name": name, "im": im, "png": png, "ppm": ppm})
    return res


FORMATS = {
    # id: (dir ext, builder)
    "webp-q75": "webp", "webp-q90-sharpyuv": "webp", "webp-lossless": "webp",
    "avif-q60-420": "avif", "avif-q80-444": "avif",
    "heic-q50": "heic", "heic-q80": "heic",
    "jxl-d1": "jxl", "jxl-lossless": "jxl", "jxl-from-jpeg": "jxl",
    "jp2-rate20": "jp2",
    "png-pil-opt": "png", "png-im-interlaced": "png", "png-pal256": "png",
    "tiff-lzw": "tif", "tiff-zip": "tif", "tiff-jpeg": "tif", "tiff-none": "tif",
    "gif-256": "gif", "bmp": "bmp",
}


def build(fmt, img, out):
    png, ppm, im = img["png"], img["ppm"], img["im"]
    if fmt == "webp-q75":
        run(["cwebp", "-quiet", "-q", "75", "-m", "4", "-metadata", "none", str(png), "-o", str(out)])
    elif fmt == "webp-q90-sharpyuv":
        run(["cwebp", "-quiet", "-q", "90", "-m", "6", "-sharp_yuv", "-metadata", "none", str(png), "-o", str(out)])
    elif fmt == "webp-lossless":
        run(["cwebp", "-quiet", "-lossless", "-z", "6", "-metadata", "none", str(png), "-o", str(out)])
    elif fmt == "avif-q60-420":
        run(["avifenc", "-j", "1", "--codec", "aom", "-s", "6", "-q", "60", "-y", "420", "-d", "8", "--ignore-exif", "--ignore-xmp", str(png), str(out)])
    elif fmt == "avif-q80-444":
        run(["avifenc", "-j", "1", "--codec", "aom", "-s", "6", "-q", "80", "-y", "444", "-d", "8", "--ignore-exif", "--ignore-xmp", str(png), str(out)])
    elif fmt in ("heic-q50", "heic-q80"):
        run(["heif-enc", "-q", fmt.split("-q")[1], "--no-alpha", "-o", str(out), str(png)])
    elif fmt == "jxl-d1":
        run(["cjxl", str(png), str(out), "-d", "1.0", "-e", "7", "--num_threads=0", "--quiet"])
    elif fmt == "jxl-lossless":
        run(["cjxl", str(png), str(out), "-d", "0", "-e", "3", "--num_threads=0", "--quiet"])
    elif fmt == "jxl-from-jpeg":
        jpg = SCRATCH / f"{img['name']}-q88.jpg"
        run(["cjpeg", "-quality", "88", "-outfile", str(jpg), str(ppm)])
        run(["cjxl", str(jpg), str(out), "--lossless_jpeg=1", "-e", "7", "--num_threads=0", "--quiet"])
    elif fmt == "jp2-rate20":
        run(["ffmpeg", "-nostdin", "-hide_banner", "-loglevel", "error", "-threads", "1", "-i", str(ppm), "-frames:v", "1",
             "-c:v", "libopenjpeg", "-compression_level", "20", "-f", "image2", "-update", "1", "-y", str(out)])
    elif fmt == "png-pil-opt":
        im.save(out, format="PNG", optimize=True)
    elif fmt == "png-im-interlaced":
        run(["convert", str(ppm), "-interlace", "PNG", "-define", "png:exclude-chunks=date,time", "-strip", f"PNG:{out}"])
    elif fmt == "png-pal256":
        im.quantize(colors=256, method=Image.Quantize.MEDIANCUT, dither=Image.Dither.FLOYDSTEINBERG).save(out, format="PNG", optimize=True)
    elif fmt in ("tiff-lzw", "tiff-zip", "tiff-jpeg", "tiff-none"):
        raw = SCRATCH / f"{img['name']}-raw.tif"
        if not raw.exists():
            im.save(raw, format="TIFF")
        comp = {"tiff-lzw": ["-c", "lzw:2"], "tiff-zip": ["-c", "zip:2"], "tiff-jpeg": ["-c", "jpeg:85", "-r", "64"],
                "tiff-none": ["-c", "none"]}[fmt]
        run(["tiffcp"] + comp + [str(raw), str(out)])
    elif fmt == "gif-256":
        run(["convert", str(ppm), "-dither", "FloydSteinberg", "-colors", "256", f"GIF:{out}"])
    elif fmt == "bmp":
        im.save(out, format="BMP")
    else:
        raise SystemExit(f"unknown format {fmt}")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True)
    ap.add_argument("--seed", type=int, required=True)
    ap.add_argument("--params", default="{}")
    args = ap.parse_args()
    params = json.loads(args.params)
    check_tools(params.get("require_tools", {}))
    outdir = Path(args.out)
    imgs = prepare(params, args.seed)
    print(f"prepared {len(imgs)} images: " + ", ".join(f"{i['name']}={i['im'].width}x{i['im'].height}" for i in imgs))
    big_n = params.get("big_formats_max_inputs")
    for fmt in params["formats"]:
        d = outdir / fmt
        d.mkdir(exist_ok=True)
        for k, img in enumerate(imgs):
            if fmt in BIG and big_n is not None and k >= big_n:
                continue
            build(fmt, img, d / f"{img['name']}.{FORMATS[fmt]}")
    total = 0
    for dirpath, dirnames, filenames in os.walk(outdir, topdown=False):
        for fn in filenames:
            p = os.path.join(dirpath, fn)
            os.chmod(p, 0o644)
            os.utime(p, (EPOCH, EPOCH))
            total += os.path.getsize(p)
        for dn in dirnames:
            p = os.path.join(dirpath, dn)
            os.chmod(p, 0o755)
            os.utime(p, (EPOCH, EPOCH))
    print(f"output bytes {total}")
    if params.get("max_bytes") and total > params["max_bytes"]:
        raise SystemExit(f"output {total} bytes exceeds max_bytes {params['max_bytes']}")


if __name__ == "__main__":
    main()
