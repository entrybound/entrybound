#!/usr/bin/env python3
"""Phone-style HEIC/HEVC matrix (g5-media, F13 "other media").

ebrc build script (research/corpus/methodology.md 5.4):
    python phone_matrix.py --out <staging> --seed N --params <canonical JSON>
Inputs: EB_INPUTS_JSON {input name -> verified cached blob (TIFF/PNG)}. Output is a pure function
of (script, seed, params, input bytes, exact tool versions checked via params.require_tools).

Why this script exists: every other F13 production pipeline in the corpus (Blender-rendered open
movies, LibriVox audiobook masters, Musopen/Prelinger archive.org transcodes, scanner-to-PDF
government documents) repeats across splits (round-1 corpus critique, F13 "pipeline repetition").
This adds a genuinely different, currently-absent pipeline: modern smartphone camera output --
HEIC stills (libheif + x265 intra, as iOS/many Android cameras save by default) and a short HEVC
clip (libx265, Main profile, "hvc1" tag, as iOS records) tagged with the "pipeline:phone-x265-heif"
covariate. No new upstream download is needed: it re-encodes the *same-split* lossless TIFF
sources already selected for that split (fsa for validation, highsmith for held-out), so the
independence group is unchanged (reuses g5-loc-fsa-owi-color-tiff / g5-loc-highsmith) and no new
independence is claimed for it.

The still photograph is real; the "video" is not (nothing was filmed) -- it is a deterministic,
seeded pan/zoom path rendered over that real still with ffmpeg's zoompan filter, then HEVC-encoded.
Labelled real_or_generated "generated" for that reason, even though its only pixel source is real.

Self-contained on purpose (no import of jpeg_matrix.py/image_codecs.py) so the materialization key
covers every line of code that shapes the bytes.

params
  inputs          optional list of input names (default all, sorted)
  long_edge       resize each source (box downscale, aspect preserved) so its longer side is at
                   most this many px (default 4032, a common phone main-camera output resolution)
                   -- the underlying master scans are much higher resolution than any phone photo,
                   so this keeps HEIC/HEVC output phone-scale in both bytes and encode time.
  heic_qualities  list of heif-enc -q values (int, phone capture quality range)
  heic_thumb_at   index into heic_qualities whose output also embeds an EXIF-scale thumbnail
                   (phones do); omit for none
  video           {"seconds": n, "fps": n, "crf": n, "keyint": n,
                    "resolutions": [[w, h], ...]}  one clip per resolution (landscape/portrait)
  require_tools   {tool: expected version substring}: ffmpeg heif-enc pillow
  max_bytes       fail if output exceeds this
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

# Deterministic zoompan presets (ffmpeg Ken-Burns-style pan/zoom over a static still); one is
# chosen per (seed, input name, resolution) so different images/orientations get different motion
# without any of it depending on wall-clock or unseeded randomness.
ZOOMPAN_PRESETS = [
    "z='min(zoom+0.0020,1.25)':x=0:y=0",
    "z='min(zoom+0.0020,1.25)':x='iw-iw/zoom':y=0",
    "z='min(zoom+0.0015,1.20)':x=0:y='ih-ih/zoom'",
    "z='min(zoom+0.0015,1.20)':x='(iw-iw/zoom)/2':y='(ih-ih/zoom)/2'",
    "z='if(lte(zoom,1.001),1.28,max(1.001,zoom-0.0022))':x=0:y=0",
]


def _out(cmd):
    r = subprocess.run(cmd, capture_output=True, text=True, errors="replace")
    return (r.stdout + r.stderr).strip()


def tool_version(tool):
    if tool == "pillow":
        return f"Pillow {Image.__version__}"
    if tool == "ffmpeg":
        return (_out(["ffmpeg", "-hide_banner", "-version"]).splitlines() or [""])[0]
    if tool == "heif-enc":
        return (_out(["heif-enc", "--version"]).splitlines() or [""])[0]
    if tool.startswith("pkg:"):
        return tool + " " + _out(["dpkg-query", "-W", "-f=${Version}", tool[4:]])
    raise SystemExit(f"unknown tool {tool}")


def check_tools(req):
    for tool, expect in sorted(req.items()):
        got = tool_version(tool)
        print(f"tool {tool}: {got}")
        if expect not in got:
            raise SystemExit(f"TOOL VERSION MISMATCH: {tool} is '{got}', expected to contain '{expect}'")


def run(cmd, what):
    env = dict(os.environ, OMP_NUM_THREADS="1")
    r = subprocess.run(cmd, capture_output=True, env=env)
    if r.returncode != 0:
        raise SystemExit(f"{what} failed ({r.returncode}): {' '.join(map(str, cmd))}\n{r.stderr.decode(errors='replace')[-2000:]}")


def load_phone_rgb(path, long_edge):
    im = Image.open(path)
    im.load()
    if im.mode in ("I;16", "I;16B", "I"):
        im = im.point(lambda v: v / 256).convert("L")
    im = im.convert("RGB")
    w, h = im.size
    if max(w, h) > long_edge:
        scale = long_edge / max(w, h)
        im = im.resize((max(1, round(w * scale)), max(1, round(h * scale))), Image.LANCZOS)
    return im


def build_heic(png, out, quality, thumb):
    cmd = ["heif-enc", "-q", str(quality), "--no-alpha"]
    if thumb:
        cmd += ["-t", "240"]
    cmd += ["-o", str(out), str(png)]
    run(cmd, f"heif-enc q={quality}")


def build_hevc(png, out, w, h, seconds, fps, crf, keyint, preset_expr):
    frames = seconds * fps
    vf = (f"scale={w}:{h}:force_original_aspect_ratio=increase,crop={w}:{h},"
          f"zoompan={preset_expr}:d={frames}:s={w}x{h}:fps={fps},format=yuv420p")
    run(["ffmpeg", "-y", "-hide_banner", "-loglevel", "error", "-loop", "1", "-i", str(png),
         "-vf", vf, "-frames:v", str(frames), "-r", str(fps),
         "-c:v", "libx265", "-preset", "medium", "-crf", str(crf), "-profile:v", "main",
         "-tag:v", "hvc1", "-x265-params", f"keyint={keyint}:min-keyint={max(1, keyint // 2)}:log-level=error",
         "-movflags", "+faststart", "-an", str(out)], "ffmpeg hevc encode")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True)
    ap.add_argument("--seed", type=int, required=True)
    ap.add_argument("--params", default="{}")
    args = ap.parse_args()
    params = json.loads(args.params)
    check_tools(params.get("require_tools", {}))

    inputs = json.loads(os.environ["EB_INPUTS_JSON"])
    names = params.get("inputs") or sorted(inputs)
    long_edge = int(params.get("long_edge", 4032))
    heic_qualities = params.get("heic_qualities", [35, 55])
    heic_thumb_at = params.get("heic_thumb_at")
    video = params["video"]

    outdir = Path(args.out)
    (outdir / "heic").mkdir(parents=True, exist_ok=True)
    (outdir / "video").mkdir(parents=True, exist_ok=True)

    for name in names:
        rng = random.Random(f"{args.seed}:{name}:phone-matrix")
        im = load_phone_rgb(inputs[name], long_edge)
        png = SCRATCH / f"{name}-phone.png"
        im.save(png, format="PNG", compress_level=1)
        print(f"{name}: phone-scale {im.width}x{im.height}")

        for qi, q in enumerate(heic_qualities):
            out = outdir / "heic" / f"{name}-q{q}.heic"
            build_heic(png, out, q, thumb=(qi == heic_thumb_at))

        for w, h in video["resolutions"]:
            orient = "portrait" if h > w else "landscape"
            preset = ZOOMPAN_PRESETS[rng.randrange(len(ZOOMPAN_PRESETS))]
            out = outdir / "video" / f"{name}-{orient}-{w}x{h}.mp4"
            build_hevc(png, out, w, h, video["seconds"], video["fps"], video["crf"], video["keyint"], preset)

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
