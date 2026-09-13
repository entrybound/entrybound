#!/usr/bin/env python3
"""JPEG producer/mode matrix and malformed-JPEG edge cases from lossless images (g5-media, F12).

ebrc build script (see research/corpus/methodology.md 5.4):
    python jpeg_matrix.py --out <staging> --seed N --params <canonical JSON>
Inputs arrive as EB_INPUTS_JSON {input name -> verified cached blob}.  Output is a pure function of
(script bytes, seed, params, input bytes, and the exact encoder versions checked via
params.require_tools -- a different toolchain fails loudly instead of changing bytes silently).

Why: JPEG-reconstruction eligibility (e.g. lossless re-encoding of the entropy-coded data) depends
on the producer (libjpeg-turbo, mozjpeg, ImageMagick, Pillow, FFmpeg, jpegtran transforms,
exiftool rewrites), coding mode (baseline/progressive/arithmetic, restart intervals, subsampling),
colour model (YCbCr/gray/CMYK/YCCK), metadata layout and damage (truncation, corruption, trailing
data).  All outputs are labelled derived-from-real; malformed files are deliberate.

params
  mode            "matrix" | "edge"
  inputs          optional list of input names (default: all, sorted)
  tiles           optional {"count": n, "size": [w, h], "reduce": [1, 2]}: crop n tiles per input
                  (random positions from the seed; every other tile portrait; reduce = box downscale)
  reduce          optional int: box-downscale whole inputs (used when tiles is absent)
  max_inputs      optional int: use only the first n prepared images
  variants        (matrix) list of variant ids, grammar below
  transform_base  (matrix) variant id used as the source of jpegtran/exiftool transforms
  edge_cases      (edge) list of case ids (see EDGE_CASES)
  require_tools   {tool: expected version substring}; tools: cjpeg, mozjpeg, imagemagick, pillow,
                  ffmpeg, exiftool, libturbojpeg, icc-profiles-free
  max_bytes       fail if the output exceeds this many bytes

variant grammar (matrix):
  <enc>-q<Q>-<sub>[-<opt>...]  enc: ljt (libjpeg-turbo cjpeg), moz (mozjpeg cjpeg), im (ImageMagick),
                               pil (Pillow), ff (FFmpeg mjpeg; Q = qscale 2..31)
                               sub: 444 422 420 440 411 gray
                               opt: opt prog arith rst1r rst4b dctfloat dctfast smooth20 icc baseline
                                    ssim notrellis fastcrush interlace
  ycck-q<Q>-<sub>              TurboJPEG API, CMYK input -> YCCK JPEG with Adobe marker
  cmyk-im-q<Q> / cmyk-pil-q<Q> CMYK JPEG (Adobe APP14) from ImageMagick / Pillow
  jt-prog jt-opt jt-arith jt-rst2 jt-rot90 jt-crop jt-gray   libjpeg-turbo jpegtran on transform_base
  mozjt                        mozjpeg jpegtran defaults on transform_base
  exif-heavy                   exiftool: EXIF+GPS+IPTC+XMP+ICC+thumbnail on transform_base
  exif-camera                  exif-heavy with the JFIF APP0 removed (camera-style APP1-first layout)
"""

from __future__ import annotations

import argparse
import ctypes
import io
import json
import os
import random
import shutil
import struct
import subprocess
import sys
import zipfile
from pathlib import Path

from PIL import Image, features

LJT_BIN = "/usr/bin"
MOZ_BIN = "/root/eb-research/tools/mozjpeg-4.1.5/bin"
ICC_SRGB = "/usr/share/color/icc/sRGB.icc"
ICC_LARGE = "/usr/share/color/icc/ITULab.icc"  # 431 KB: forces a multi-chunk APP2 ICC profile
EPOCH = int(os.environ.get("SOURCE_DATE_EPOCH", "1767225600"))
SCRATCH = Path(os.environ.get("EB_SCRATCH", "/tmp"))
Image.MAX_IMAGE_PIXELS = 400_000_000


# ---------------------------------------------------------------------------
# tool checks


def _cmd_out(cmd):
    r = subprocess.run(cmd, capture_output=True, text=True, errors="replace")
    return (r.stdout + r.stderr).strip()


def tool_version(tool):
    if tool == "cjpeg":
        return _cmd_out([f"{LJT_BIN}/cjpeg", "-version"]).splitlines()[0]
    if tool == "mozjpeg":
        return _cmd_out([f"{MOZ_BIN}/cjpeg", "-version"]).splitlines()[0]
    if tool == "imagemagick":
        return _cmd_out(["convert", "-version"]).splitlines()[0]
    if tool == "pillow":
        return f"Pillow {Image.__version__} libjpeg-turbo {features.version('libjpeg_turbo')}"
    if tool == "ffmpeg":
        return _cmd_out(["ffmpeg", "-hide_banner", "-version"]).splitlines()[0]
    if tool == "exiftool":
        return "exiftool " + _cmd_out(["exiftool", "-ver"])
    if tool in ("libturbojpeg", "icc-profiles-free"):
        return f"{tool} " + _cmd_out(["dpkg-query", "-W", "-f=${Version}", tool])
    raise SystemExit(f"unknown tool {tool}")


def check_tools(req):
    for tool, expect in sorted(req.items()):
        got = tool_version(tool)
        print(f"tool {tool}: {got}")
        if expect not in got:
            raise SystemExit(f"TOOL VERSION MISMATCH: {tool} is '{got}', expected to contain '{expect}'")


def run(cmd, stdout=None, env=None):
    e = dict(os.environ, MAGICK_THREAD_LIMIT="1", OMP_NUM_THREADS="1")
    if env:
        e.update(env)
    r = subprocess.run(cmd, stdout=stdout if stdout is not None else subprocess.PIPE, stderr=subprocess.PIPE, env=e)
    if r.returncode != 0:
        raise SystemExit(f"command failed ({r.returncode}): {' '.join(map(str, cmd))}\n{r.stderr.decode(errors='replace')[-2000:]}")
    return r


# ---------------------------------------------------------------------------
# image preparation


def load_rgb(path):
    im = Image.open(path)
    im.load()
    if im.mode in ("I;16", "I;16B", "I"):
        im = im.point(lambda v: v / 256).convert("L")
    if im.mode not in ("RGB", "L"):
        im = im.convert("RGB")
    if im.mode == "L":
        im = im.convert("RGB")
    return im


def prepare_images(params, seed):
    inputs = json.loads(os.environ["EB_INPUTS_JSON"])
    names = params.get("inputs") or sorted(inputs)
    tiles = params.get("tiles")
    out = []
    for name in names:
        im = load_rgb(inputs[name])
        if tiles:
            rng = random.Random(f"{seed}:{name}:tiles")
            tw, th = tiles["size"]
            reduces = tiles.get("reduce", [1])
            for t in range(tiles["count"]):
                r = reduces[t % len(reduces)]
                w, h = (tw, th) if t % 2 == 0 else (th, tw)
                cw, ch = min(w * r, im.width), min(h * r, im.height)
                x = rng.randrange(0, im.width - cw + 1)
                y = rng.randrange(0, im.height - ch + 1)
                tile = im.crop((x, y, x + cw, y + ch))
                if r > 1:
                    tile = tile.reduce(r)
                out.append((f"{name}-t{t}", tile))
        else:
            r = int(params.get("reduce", 1))
            out.append((name, im.reduce(r) if r > 1 else im))
    if params.get("max_inputs"):
        out = out[: int(params["max_inputs"])]
    prepared = []
    for name, im in out:
        ppm = SCRATCH / f"src-{name}.ppm"
        im.save(ppm, format="PPM")
        prepared.append({"name": name, "im": im, "ppm": ppm})
    return prepared


# ---------------------------------------------------------------------------
# encoders

SAMPLE = {"444": "1x1", "422": "2x1", "420": "2x2", "440": "1x2", "411": "4x1"}
IM_SAMPLE = {"444": "1x1", "422": "2x1", "420": "2x2", "440": "1x2", "411": "4x1"}
PIL_SUB = {"444": 0, "422": 1, "420": 2}
FF_PIX = {"444": "yuvj444p", "422": "yuvj422p", "420": "yuvj420p", "440": "yuvj440p", "411": "yuvj411p", "gray": "gray"}
TJ_SAMP = {"444": 0, "422": 1, "420": 2, "gray": 3, "440": 4, "411": 5}


def parse_variant(vid):
    parts = vid.split("-")
    return parts[0], int(parts[1][1:]), parts[2], parts[3:]


def enc_cjpeg(binary, img, out, q, sub, opts, moz=False):
    cmd = [binary, "-quality", str(q)]
    if sub == "gray":
        cmd.append("-grayscale")
    else:
        cmd += ["-sample", SAMPLE[sub]]
    flag = {"opt": ["-optimize"], "prog": ["-progressive"], "arith": ["-arithmetic"], "rst1r": ["-restart", "1"],
            "rst4b": ["-restart", "4B"], "dctfloat": ["-dct", "float"], "dctfast": ["-dct", "fast"],
            "smooth20": ["-smooth", "20"], "icc": ["-icc", ICC_SRGB], "baseline": ["-baseline"],
            "ssim": ["-tune-ssim"], "notrellis": ["-notrellis"], "fastcrush": ["-fastcrush"]}
    for o in opts:
        if o not in flag or (not moz and o in ("baseline", "ssim", "notrellis", "fastcrush")):
            raise SystemExit(f"unsupported cjpeg option {o}")
        cmd += flag[o]
    cmd += ["-outfile", str(out), str(img["ppm"])]
    run(cmd)


def enc_im(img, out, q, sub, opts):
    cmd = ["convert", str(img["ppm"]), "-quality", str(q)]
    if sub == "gray":
        cmd += ["-colorspace", "Gray"]
    else:
        cmd += ["-sampling-factor", IM_SAMPLE[sub]]
    for o in opts:
        if o == "interlace":
            cmd += ["-interlace", "JPEG"]
        elif o == "opt":
            cmd += ["-define", "jpeg:optimize-coding=true"]
        else:
            raise SystemExit(f"unsupported im option {o}")
    run(cmd + [f"JPEG:{out}"])


def enc_pil(img, out, q, sub, opts):
    kw = {"quality": q}
    im = img["im"]
    if sub == "gray":
        im = im.convert("L")
    else:
        kw["subsampling"] = PIL_SUB[sub]
    for o in opts:
        if o == "opt":
            kw["optimize"] = True
        elif o == "prog":
            kw["progressive"] = True
        else:
            raise SystemExit(f"unsupported pil option {o}")
    im.save(out, format="JPEG", **kw)


def enc_ff(img, out, q, sub, opts):
    cmd = ["ffmpeg", "-nostdin", "-hide_banner", "-loglevel", "error", "-threads", "1", "-i", str(img["ppm"]),
           "-frames:v", "1", "-c:v", "mjpeg", "-pix_fmt", FF_PIX[sub], "-q:v", str(q)]
    for o in opts:
        if o == "opt":
            cmd += ["-huffman", "optimal"]
        else:
            raise SystemExit(f"unsupported ff option {o}")
    if "opt" not in opts:
        cmd += ["-huffman", "default"]
    run(cmd + ["-f", "image2", "-update", "1", "-y", str(out)])


def enc_ycck(img, out, q, sub):
    tj = ctypes.CDLL("libturbojpeg.so.0")
    tj.tjInitCompress.restype = ctypes.c_void_p
    tj.tjCompress2.argtypes = [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_int, ctypes.c_int, ctypes.c_int, ctypes.c_int,
                               ctypes.POINTER(ctypes.POINTER(ctypes.c_ubyte)), ctypes.POINTER(ctypes.c_ulong),
                               ctypes.c_int, ctypes.c_int, ctypes.c_int]
    tj.tjFree.argtypes = [ctypes.c_void_p]
    tj.tjDestroy.argtypes = [ctypes.c_void_p]
    cmyk = img["im"].convert("CMYK")
    data = cmyk.tobytes()
    h = tj.tjInitCompress()
    buf = ctypes.POINTER(ctypes.c_ubyte)()
    size = ctypes.c_ulong(0)
    rc = tj.tjCompress2(h, data, cmyk.width, 0, cmyk.height, 11, ctypes.byref(buf), ctypes.byref(size), TJ_SAMP[sub], q, 0)
    if rc != 0:
        raise SystemExit("tjCompress2 (CMYK/YCCK) failed")
    Path(out).write_bytes(ctypes.string_at(buf, size.value))
    tj.tjFree(buf)
    tj.tjDestroy(h)


def enc_cmyk(tool, img, out, q):
    if tool == "im":
        run(["convert", str(img["ppm"]), "-colorspace", "CMYK", "-quality", str(q), f"JPEG:{out}"])
    else:
        img["im"].convert("CMYK").save(out, format="JPEG", quality=q)


def jpegtran(binary, src, out, args):
    run([binary] + args + ["-outfile", str(out), str(src)])


EXIF_ARGS = [
    "-n", "-Make=EB Research Camera Co.", "-Model=EBRC-G5 Mark II", "-Software=ebrc g5-media exif-heavy 1",
    "-Artist=Entrybound research corpus (synthetic metadata)", "-Copyright=Synthetic metadata; image derived from public source",
    "-DateTimeOriginal=2025:06:01 12:34:56", "-CreateDate=2025:06:01 12:34:56", "-ModifyDate=2025:06:02 08:00:00",
    "-SubSecTimeOriginal=123", "-OffsetTimeOriginal=+00:00", "-ExposureTime=0.004", "-FNumber=5.6", "-ISO=200",
    "-FocalLength=35", "-FocalLengthIn35mmFormat=52", "-ExposureProgram=3", "-MeteringMode=5", "-Flash=16",
    "-WhiteBalance=0", "-LensMake=EB Optics", "-LensModel=EB 24-70mm F2.8", "-BodySerialNumber=G5000001",
    "-GPSLatitude=29.559", "-GPSLatitudeRef=N", "-GPSLongitude=95.089", "-GPSLongitudeRef=W", "-GPSAltitude=12.5",
    "-GPSAltitudeRef=0", "-GPSDateStamp=2025:06:01", "-GPSTimeStamp=12:34:56", "-Orientation=1", "-ResolutionUnit=2",
    "-XResolution=300", "-YResolution=300", "-YCbCrPositioning=1", "-ColorSpace=1",
    "-IPTC:By-line=EBRC", "-IPTC:City=Houston", "-IPTC:Country-PrimaryLocationName=USA",
    "-XMP-dc:Creator=EBRC", "-XMP-xmp:Rating=3", "-XMP-photoshop:Headline=Synthetic metadata block",
]


def exif_heavy(src, out, name, thumb_src_im):
    thumb = SCRATCH / f"thumb-{name}.jpg"
    t = thumb_src_im.copy()
    t.thumbnail((160, 160), Image.Resampling.BOX)
    t.save(thumb, format="JPEG", quality=80)
    desc = ("Synthetic long description for metadata-heavy JPEG testing. " * 40).strip()
    kws = [f"-IPTC:Keywords=kw{i:03d}" for i in range(60)] + [f"-XMP-dc:Subject=subject{i:03d}" for i in range(60)]
    run(["exiftool", "-q", "-q"] + EXIF_ARGS + kws + [
        f"-ImageDescription={desc[:1000]}", f"-XMP-dc:Description={desc}", f"-IPTC:Caption-Abstract={desc[:1800]}",
        f"-UserComment={desc[:800]}", f"-ICC_Profile<={ICC_SRGB}", f"-ThumbnailImage<={thumb}", "-o", str(out), str(src)])


# ---------------------------------------------------------------------------
# JPEG marker surgery


def segments(data):
    """Return (list of (marker, start, end) up to and including SOS header, scan_start)."""
    if data[:2] != b"\xff\xd8":
        raise ValueError("no SOI")
    pos, segs = 2, []
    while pos < len(data):
        if data[pos] != 0xFF:
            raise ValueError(f"expected marker at {pos}")
        while data[pos] == 0xFF:
            pos += 1
        m = data[pos]
        pos += 1
        if m in (0xD8, 0x01) or 0xD0 <= m <= 0xD7:
            continue
        length = struct.unpack(">H", data[pos:pos + 2])[0]
        segs.append((m, pos - 2, pos + length))
        pos += length
        if m == 0xDA:
            return segs, pos
    raise ValueError("no SOS")


def strip_jfif(data):
    segs, _ = segments(data)
    out, last = bytearray(data[:2]), 2
    for m, s, e in segs:
        if m == 0xE0 and data[s + 4:s + 9] == b"JFIF\x00":
            out += data[last:s]
            last = e
    out += data[last:]
    return bytes(out)


def scan_bounds(data):
    _, scan_start = segments(data)
    eoi = data.rfind(b"\xff\xd9")
    return scan_start, (eoi if eoi > scan_start else len(data))


# ---------------------------------------------------------------------------
# matrix mode


def build_variant(vid, img, out, cache):
    if vid.startswith(("ljt-", "moz-", "im-", "pil-", "ff-")):
        enc, q, sub, opts = parse_variant(vid)
        if enc == "ljt":
            enc_cjpeg(f"{LJT_BIN}/cjpeg", img, out, q, sub, opts)
        elif enc == "moz":
            enc_cjpeg(f"{MOZ_BIN}/cjpeg", img, out, q, sub, opts, moz=True)
        elif enc == "im":
            enc_im(img, out, q, sub, opts)
        elif enc == "pil":
            enc_pil(img, out, q, sub, opts)
        else:
            enc_ff(img, out, q, sub, opts)
    elif vid.startswith("ycck-"):
        _, q, sub, _ = parse_variant(vid)
        enc_ycck(img, out, q, sub)
    elif vid.startswith("cmyk-"):
        tool, q = vid.split("-")[1], int(vid.split("-")[2][1:])
        enc_cmyk(tool, img, out, q)
    else:
        base = cache["base"]
        im = img["im"]
        tj = {"jt-prog": ["-copy", "all", "-progressive"], "jt-opt": ["-copy", "all", "-optimize"],
              "jt-arith": ["-copy", "all", "-arithmetic"], "jt-rst2": ["-copy", "all", "-restart", "2"],
              "jt-rot90": ["-copy", "all", "-rotate", "90", "-trim"], "jt-gray": ["-copy", "all", "-grayscale"],
              "jt-crop": ["-copy", "all", "-crop", f"{im.width * 3 // 4}x{im.height * 3 // 4}+{im.width // 8}+{im.height // 8}"]}
        if vid in tj:
            jpegtran(f"{LJT_BIN}/jpegtran", base, out, tj[vid])
        elif vid == "mozjt":
            jpegtran(f"{MOZ_BIN}/jpegtran", base, out, ["-copy", "all"])
        elif vid == "exif-heavy":
            exif_heavy(base, out, img["name"], im)
        elif vid == "exif-camera":
            tmp = SCRATCH / f"exifcam-{img['name']}.jpg"
            exif_heavy(base, tmp, img["name"], im)
            Path(out).write_bytes(strip_jfif(tmp.read_bytes()))
        else:
            raise SystemExit(f"unknown variant {vid}")


def mode_matrix(outdir, imgs, params):
    variants = params["variants"]
    base_id = params.get("transform_base", "ljt-q90-420")
    for img in imgs:
        base = SCRATCH / f"base-{img['name']}.jpg"
        build_variant(base_id, img, base, {})
        cache = {"base": base}
        for vid in variants:
            d = outdir / vid
            d.mkdir(exist_ok=True)
            build_variant(vid, img, d / f"{img['name']}.jpg", cache)


# ---------------------------------------------------------------------------
# edge mode

SCAN_NONINTERLEAVED = "0;\n1;\n2;\n"
SCAN_PROG_CUSTOM = "0 1 2: 0 0 0 2;\n0: 1 9 0 2;\n2: 1 63 0 1;\n1: 1 63 0 1;\n0: 10 63 0 2;\n0 1 2: 0 0 2 1;\n0: 1 63 2 1;\n0 1 2: 0 0 1 0;\n1: 1 63 1 0;\n2: 1 63 1 0;\n0: 1 63 1 0;\n"


def edge_case(case, img, out, seed):
    rng = random.Random(f"{seed}:{img['name']}:{case}")
    name = img["name"]
    base = SCRATCH / f"edge-base-{name}.jpg"
    prog = SCRATCH / f"edge-prog-{name}.jpg"
    rst = SCRATCH / f"edge-rst-{name}.jpg"
    if not base.exists():
        enc_cjpeg(f"{LJT_BIN}/cjpeg", img, base, 85, "420", [])
        enc_cjpeg(f"{LJT_BIN}/cjpeg", img, prog, 85, "420", ["prog"])
        enc_cjpeg(f"{LJT_BIN}/cjpeg", img, rst, 85, "420", ["rst1r"])
    b = base.read_bytes()
    s0, s1 = scan_bounds(b)

    def write(data):
        Path(out).write_bytes(data)

    if case.startswith("trunc-") and case.endswith("pct"):
        src = prog.read_bytes() if "-prog-" in case else b
        pct = int(case.split("-")[-1][:-3])
        write(src[: len(src) * pct // 100])
    elif case == "trunc-no-eoi":
        write(b[:-2])
    elif case == "trunc-header":
        segs, _ = segments(b)
        sof = [e for m, s, e in segs if m == 0xC0][0]
        write(b[: sof + 10])
    elif case == "trunc-after-ff":
        i = b.find(b"\xff\x00", s0 + (s1 - s0) // 2)
        write(b[: i + 1])
    elif case.startswith("corrupt-bitflip-"):
        n = int(case.split("-")[-1])
        d = bytearray(b)
        for _ in range(n):
            p = rng.randrange(s0, s1)
            d[p] ^= 1 << rng.randrange(8)
        write(bytes(d))
    elif case == "corrupt-zero-4k":
        d = bytearray(b)
        p = s0 + (s1 - s0) // 3
        d[p:p + 4096] = bytes(min(4096, s1 - p))
        write(bytes(d))
    elif case == "corrupt-random-block-4k":
        d = bytearray(b)
        p = s0 + (s1 - s0) // 2
        d[p:p + 4096] = rng.randbytes(min(4096, s1 - p))
        write(bytes(d))
    elif case == "corrupt-rst-missing":
        r = rst.read_bytes()
        rs0, _ = scan_bounds(r)
        idx = [i for i in range(rs0, len(r) - 1) if r[i] == 0xFF and 0xD0 <= r[i + 1] <= 0xD7]
        k = idx[len(idx) // 2]
        write(r[:k] + r[k + 2:])
    elif case == "corrupt-rst-order":
        r = bytearray(rst.read_bytes())
        rs0, _ = scan_bounds(bytes(r))
        idx = [i for i in range(rs0, len(r) - 1) if r[i] == 0xFF and 0xD0 <= r[i + 1] <= 0xD7]
        a, c = idx[len(idx) // 3], idx[2 * len(idx) // 3]
        r[a + 1], r[c + 1] = r[c + 1], r[a + 1]
        write(bytes(r))
    elif case == "corrupt-dht":
        d = bytearray(b)
        segs, _ = segments(b)
        s = [s for m, s, e in segs if m == 0xC4][0]
        for i in range(s + 5, s + 21):
            d[i] = rng.randrange(256)
        write(bytes(d))
    elif case == "corrupt-sof-height-zero":
        d = bytearray(b)
        segs, _ = segments(b)
        s = [s for m, s, e in segs if m == 0xC0][0]
        d[s + 5:s + 7] = b"\x00\x00"
        write(bytes(d))
    elif case == "corrupt-early-eoi":
        p = s0 + (s1 - s0) // 2
        write(b[:p] + b"\xff\xd9" + b[p:])
    elif case == "trail-random-4k":
        write(b + rng.randbytes(4096))
    elif case == "trail-zeros-64k":
        write(b + bytes(65536))
    elif case == "trail-text":
        write(b + b"\nTRAILER: data appended after EOI by a careless tool\n" * 8)
    elif case == "trail-zip":
        bio = io.BytesIO()
        with zipfile.ZipFile(bio, "w", zipfile.ZIP_DEFLATED) as z:
            zi = zipfile.ZipInfo("hidden/readme.txt", date_time=(2026, 1, 1, 0, 0, 0))
            zi.external_attr = 0o644 << 16
            z.writestr(zi, "polyglot payload\n" * 200)
        write(b + bio.getvalue())
    elif case == "trail-jpeg-concat":
        small = SCRATCH / f"edge-small-{name}.jpg"
        t = img["im"].copy()
        t.thumbnail((256, 256), Image.Resampling.BOX)
        t.save(small, format="JPEG", quality=70)
        write(b + small.read_bytes())
    elif case == "lead-junk-512":
        write(rng.randbytes(512) + b)
    elif case == "pad-ff-fill":
        segs, _ = segments(b)
        d, last = bytearray(), 0
        for m, s, e in segs:
            d += b[last:s] + b"\xff" * 7
            last = s
        write(bytes(d + b[last:]))
    elif case == "multi-com":
        coms = b"".join(b"\xff\xfe" + struct.pack(">H", 2 + len(t)) + t
                        for t in (f"comment segment {i:03d} ".encode() * 3 for i in range(120)))
        write(b[:2] + coms + b[2:])
    elif case == "huge-app-tiny-image":
        tiny = SCRATCH / f"tiny-{name}.ppm"
        img["im"].resize((16, 16), Image.Resampling.BOX).save(tiny, format="PPM")
        t = SCRATCH / f"tiny-{name}.jpg"
        run([f"{LJT_BIN}/cjpeg", "-quality", "90", "-outfile", str(t), str(tiny)])
        tb = t.read_bytes()
        app13 = b"Photoshop 3.0\x00" + rng.randbytes(60000)
        com = b"x" * 65000
        write(tb[:2] + b"\xff\xed" + struct.pack(">H", 2 + len(app13)) + app13 + b"\xff\xfe" + struct.pack(">H", 2 + len(com)) + com + tb[2:])
    elif case == "app-after-dqt":
        segs, _ = segments(b)
        app = [(s, e) for m, s, e in segs if m == 0xE0][0]
        dqt_end = [e for m, s, e in segs if m == 0xDB][-1]
        rest = b[:app[0]] + b[app[1]:dqt_end]
        write(rest + b[app[0]:app[1]] + b[dqt_end:])
    elif case.startswith("dims-"):
        w, h = map(int, case.split("-")[1].split("x"))
        p = SCRATCH / f"dims-{name}-{w}x{h}.ppm"
        img["im"].resize((w, h), Image.Resampling.BOX).save(p, format="PPM")
        run([f"{LJT_BIN}/cjpeg", "-quality", "85", "-sample", "1x1" if w % 2 else "2x2", "-outfile", str(out), str(p)])
    elif case == "gray-ljt":
        enc_cjpeg(f"{LJT_BIN}/cjpeg", img, out, 85, "gray", [])
    elif case == "gray-im":
        enc_im(img, out, 85, "gray", [])
    elif case == "cmyk-im":
        enc_cmyk("im", img, out, 85)
    elif case == "cmyk-pil":
        enc_cmyk("pil", img, out, 88)
    elif case == "ycck-tj":
        enc_ycck(img, out, 85, "420")
    elif case == "cmyk-im-large-icc":
        tmp = SCRATCH / f"cmyk-{name}.jpg"
        enc_cmyk("im", img, tmp, 85)
        run(["exiftool", "-q", "-q", f"-ICC_Profile<={ICC_LARGE}", "-o", str(out), str(tmp)])
    elif case == "scans-noninterleaved":
        sf = SCRATCH / "scans-nonint.txt"
        sf.write_text(SCAN_NONINTERLEAVED)
        run([f"{LJT_BIN}/cjpeg", "-quality", "85", "-scans", str(sf), "-outfile", str(out), str(img["ppm"])])
    elif case == "scans-progressive-custom":
        sf = SCRATCH / "scans-prog.txt"
        sf.write_text(SCAN_PROG_CUSTOM)
        run([f"{LJT_BIN}/cjpeg", "-quality", "85", "-scans", str(sf), "-outfile", str(out), str(img["ppm"])])
    elif case == "arith-sequential":
        enc_cjpeg(f"{LJT_BIN}/cjpeg", img, out, 85, "420", ["arith"])
    elif case == "arith-progressive":
        enc_cjpeg(f"{LJT_BIN}/cjpeg", img, out, 85, "420", ["arith", "prog"])
    elif case == "exif-orientation-6":
        run(["exiftool", "-q", "-q", "-n", "-Orientation=6", "-o", str(out), str(base)])
    elif case == "exif-thumb-large":
        th = SCRATCH / f"bigthumb-{name}.jpg"
        img["im"].save(th, format="JPEG", quality=95)
        if th.stat().st_size > 60000:
            t2 = img["im"].copy()
            t2.thumbnail((320, 320), Image.Resampling.BOX)
            t2.save(th, format="JPEG", quality=95)
        run(["exiftool", "-q", "-q", f"-ThumbnailImage<={th}", "-o", str(out), str(base)])
    elif case == "exif-heavy":
        exif_heavy(base, out, name, img["im"])
    elif case == "exif-camera":
        tmp = SCRATCH / f"edge-exifcam-{name}.jpg"
        exif_heavy(base, tmp, name, img["im"])
        write(strip_jfif(tmp.read_bytes()))
    elif case == "q100-444-optimized":
        enc_cjpeg(f"{LJT_BIN}/cjpeg", img, out, 100, "444", ["opt"])
    elif case == "q1-420":
        enc_cjpeg(f"{LJT_BIN}/cjpeg", img, out, 1, "420", [])
    elif case == "jt-rot90-perfect-or-trim":
        jpegtran(f"{LJT_BIN}/jpegtran", base, out, ["-copy", "all", "-rotate", "90", "-trim"])
    elif case == "duplicate-identical":
        write(b)
    else:
        raise SystemExit(f"unknown edge case {case}")


def mode_edge(outdir, imgs, params, seed):
    for case in params["edge_cases"]:
        d = outdir / case
        d.mkdir(exist_ok=True)
        for img in imgs:
            edge_case(case, img, d / f"{img['name']}.jpg", seed)


# ---------------------------------------------------------------------------


def normalize_tree(root):
    total = 0
    for dirpath, dirnames, filenames in os.walk(root, topdown=False):
        for fn in filenames:
            p = os.path.join(dirpath, fn)
            os.chmod(p, 0o644)
            total += os.path.getsize(p)
            os.utime(p, (EPOCH, EPOCH))
        for dn in dirnames:
            p = os.path.join(dirpath, dn)
            os.chmod(p, 0o755)
            os.utime(p, (EPOCH, EPOCH))
    return total


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True)
    ap.add_argument("--seed", type=int, required=True)
    ap.add_argument("--params", default="{}")
    args = ap.parse_args()
    params = json.loads(args.params)
    check_tools(params.get("require_tools", {}))
    outdir = Path(args.out)
    imgs = prepare_images(params, args.seed)
    print(f"prepared {len(imgs)} images: " + ", ".join(f"{i['name']}={i['im'].width}x{i['im'].height}" for i in imgs))
    if params["mode"] == "matrix":
        mode_matrix(outdir, imgs, params)
    elif params["mode"] == "edge":
        mode_edge(outdir, imgs, params, args.seed)
    else:
        raise SystemExit(f"unknown mode {params['mode']}")
    total = normalize_tree(outdir)
    print(f"output bytes {total}")
    if params.get("max_bytes") and total > params["max_bytes"]:
        raise SystemExit(f"output {total} bytes exceeds max_bytes {params['max_bytes']}")


if __name__ == "__main__":
    main()
