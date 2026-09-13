"""g5-media item table: turns selection/*.json into ebrc source definitions (used by select_sources.py emit).

Families: F12 (JPEG/images) and F13 (other media).  Every item lists its independence group; no
group is shared across splits within a family, and upstream files are split-exclusive.
"""

from __future__ import annotations

import re

MiB = 1 << 20
GEN = "research/corpus/generators/g5-media"

NOTES = (
    "g5-media: F12 JPEG/images and F13 other media. Emitted by research/corpus/generators/g5-media/select_sources.py "
    "(emit stage) from selection/*.json; do not hand-edit -- change g5_items.py or the selection and re-emit. "
    "Downloads retrieved 2026-09-12 local date (UTC timestamps may read 2026-09-13); every input declares sha256+size. "
    "Build items need the toolchain from generators/g5-media/setup_tools.sh (Ubuntu 24.04 packages + mozjpeg 4.1.5 "
    "built from source); scripts verify exact tool versions via params.require_tools and fail loudly on drift."
)

TOOLS_JPEG = {
    "cjpeg": "libjpeg-turbo version 2.1.5 (build 20240408)",
    "mozjpeg": "mozjpeg version 4.1.5",
    "imagemagick": "ImageMagick 6.9.12-98 Q16",
    "pillow": "Pillow 12.3.0 libjpeg-turbo 3.1.4.1",
    "ffmpeg": "ffmpeg version 6.1.1-3ubuntu5",
    "exiftool": "exiftool 12.76",
    "libturbojpeg": "libturbojpeg 1:2.1.5-2ubuntu2",
    "icc-profiles-free": "icc-profiles-free 2.0.1+dfsg-1.1",
}
TOOLS_CODECS = {
    "cwebp": "1.3.2", "avifenc": "Version: 1.0.4 (dav1d [dec]:1.4.1, libgav1 [dec]:0.18.0, aom [enc/dec]:v3.8.2",
    "heif-enc": "1.17.6", "cjxl": "cjxl v0.7.0", "imagemagick": "ImageMagick 6.9.12-98 Q16", "pillow": "Pillow 12.3.0",
    "ffmpeg": "ffmpeg version 6.1.1-3ubuntu5", "tiffcp": "LIBTIFF, Version 4.5.1", "cjpeg": "libjpeg-turbo version 2.1.5 (build 20240408)",
    "pkg:libheif-plugin-x265": "1.17.6-1ubuntu4.8", "pkg:libx265-199": "3.5-2build1", "pkg:libjxl0.7": "0.7.0-10.2ubuntu6.1",
    "pkg:libwebp7": "1.3.2-0.4build3", "pkg:libopenjp2-7": "2.5.0-2ubuntu0.5",
}

# ---------------------------------------------------------------------------
# licences

LIC = {
    "nasa-ivl": {"spdx_or_name": "LicenseRef-NASA-media-guidelines (US Government work, public domain in the US)", "redistributable": True,
                 "attribution": "NASA; photographer credit per input notes",
                 "notes": "NASA still images are generally not subject to US copyright (NASA Images and Media Usage Guidelines); "
                          "NASA insignia/identifiers must not imply endorsement. Selection excluded records whose credits name "
                          "commercial or non-US-agency parties, non-NASA astronauts or Roscosmos photographers."},
    "commons-cc0": {"spdx_or_name": "CC0-1.0", "redistributable": True,
                    "attribution": "Wikimedia Commons contributors; author per input notes (not required by CC0)",
                    "notes": "Licence of each file read from Commons extmetadata (LicenseShortName) at selection time; source page URL in input notes."},
    "commons-usgov": {"spdx_or_name": "LicenseRef-PD-USGov (US federal government work, public domain)", "redistributable": True,
                      "attribution": "US federal agencies (FWS, NPS, Navy, Army, Air Force, USDA, NOAA, USGS) via Wikimedia Commons; per input notes",
                      "notes": "Files tagged with PD-USGov-* templates on Commons and 'Public domain' licence metadata; NASA-credited files excluded (NASA is a tuning group)."},
    "kodak": {"spdx_or_name": "LicenseRef-Kodak-PhotoCD-PCD0992", "redistributable": False,
              "attribution": "Eastman Kodak Company (Kodak Lossless True Color Image Suite); PNG conversion Rich Franzen, r0k.us",
              "notes": "Host states the images were released by Kodak for unrestricted usage, but no formal licence text exists; "
                       "treated as not redistributable (only hashes committed). Encoded derivatives inherit this status."},
    "fsa": {"spdx_or_name": "LicenseRef-Public-Domain (Library of Congress FSA/OWI; no known restrictions)", "redistributable": True,
            "attribution": "Library of Congress, Prints & Photographs Division, FSA/OWI Collection; photographers per input notes",
            "notes": "US Government (Farm Security Administration / Office of War Information) photographs 1939-1945, retrieved from Wikimedia Commons (LoC TIFF uploads). Derivatives inherit public-domain status."},
    "highsmith": {"spdx_or_name": "LicenseRef-Public-Domain (Carol M. Highsmith Archive, Library of Congress)", "redistributable": True,
                  "attribution": "Carol M. Highsmith Archive, Library of Congress, Prints and Photographs Division",
                  "notes": "Highsmith donated her archive to the public; LoC states no known restrictions. TIFFs retrieved from Wikimedia Commons LoC uploads."},
    "bbb": {"spdx_or_name": "CC-BY-3.0", "redistributable": True, "attribution": "(c) copyright 2008, Blender Foundation / www.bigbuckbunny.org",
            "notes": "Big Buck Bunny open movie (download.blender.org; one Wikimedia Commons server transcode of the same film)."},
    "sintel": {"spdx_or_name": "CC-BY-3.0", "redistributable": True, "attribution": "(c) copyright Blender Foundation | durian.blender.org",
               "notes": "Sintel trailer renditions and the music+effects FLAC stem."},
    "tos": {"spdx_or_name": "CC-BY-3.0", "redistributable": True, "attribution": "(CC) Blender Foundation | mango.blender.org",
            "notes": "Tears of Steel film is CC BY 3.0; its original soundtrack files are CC BY-ND 3.0 ((C) Joram Letwory) -- verbatim redistribution allowed."},
    "ed": {"spdx_or_name": "CC-BY-2.5", "redistributable": True,
           "attribution": "(c) copyright 2006, Blender Foundation / Netherlands Media Art Institute / www.elephantsdream.org", "notes": "Elephants Dream open movie."},
    "prelinger": {"spdx_or_name": "LicenseRef-Public-Domain (Prelinger Archives)", "redistributable": True, "attribution": "Prelinger Archives (Internet Archive)",
                  "notes": "Prelinger Archives films marked public domain on the Internet Archive; MPEG-4 and Ogg files are archive.org derivatives."},
    "librivox": {"spdx_or_name": "LicenseRef-Public-Domain (LibriVox; PDM-1.0)", "redistributable": True, "attribution": "LibriVox volunteers",
                 "notes": "LibriVox recordings are dedicated to the public domain; files hosted by the Internet Archive (LibriVox's distribution host), MP3/Ogg/M4B derivatives made by archive.org."},
    "musopen-dvd": {"spdx_or_name": "LicenseRef-Public-Domain (PDM-1.0)", "redistributable": True, "attribution": "Musopen (www.musopen.org)",
                    "notes": "Musopen Kickstarter lossless DVD compilation (Internet Archive item musopen-dvd-in-lossless-flac-format)."},
    "musopen-chopin": {"spdx_or_name": "CC0-1.0", "redistributable": True, "attribution": "Musopen (www.musopen.org)",
                       "notes": "Musopen Chopin complete works FLAC (Internet Archive item musopen-chopin-complete-works-flac)."},
    "ntrs": {"spdx_or_name": "LicenseRef-US-Government-Work (NTRS GOV_PUBLIC_USE_PERMITTED)", "redistributable": True, "attribution": "NASA",
             "notes": "Only NTRS records with copyright determination GOV_PUBLIC_USE_PERMITTED were selected."},
    "fedreg": {"spdx_or_name": "LicenseRef-US-Government-Work (Federal Register)", "redistributable": True,
               "attribution": "Office of the Federal Register, National Archives and Records Administration (govinfo.gov)",
               "notes": "Federal Register documents are US Government works; public domain in the US."},
    "usgs": {"spdx_or_name": "LicenseRef-US-Government-Work (USGS)", "redistributable": True, "attribution": "U.S. Geological Survey",
             "notes": "USGS-authored publications are public domain; individual figures credited to third parties may carry other terms."},
}

# ---------------------------------------------------------------------------
# helpers


def slug(s, n=40):
    return re.sub(r"[^a-z0-9]+", "-", s.lower()).strip("-")[:n].strip("-") or "x"


def input_notes(e):
    lic = e.get("license") or {}
    parts = [lic.get("spdx_or_name", "")]
    if lic.get("attribution"):
        parts.append("author: " + lic["attribution"])
    if lic.get("source_page"):
        parts.append(lic["source_page"])
    return "; ".join(p for p in parts if p)[:600]


class MissingPool(Exception):
    pass


def pool_entries(sel, pool):
    if pool not in sel or any(not e.get("sha256") for e in sel[pool]["entries"]):
        raise MissingPool(pool)
    return sel[pool]["entries"]


def entries_for(sel, pool, alloc):
    es = [e for e in pool_entries(sel, pool) if e.get("alloc") == alloc]
    if not es:
        raise SystemExit(f"no entries allocated to {alloc} in pool {pool}")
    return es


def input_name(e):
    return slug(e["key"], 64)


def make_inputs(entries, with_notes=True):
    inputs = []
    names = [input_name(e) for e in entries]
    if len(set(names)) != len(names):
        raise SystemExit(f"input name collision among {names}")
    for name, e in zip(names, entries):
        inp = {"name": name, "url": e["url"], "sha256": e["sha256"], "size": e["size"]}
        if with_notes:
            inp["notes"] = input_notes(e)
        inputs.append(inp)
    return inputs


def download_item(item_id, family, split, scale, group, entries, lic, description, tags=None, extract_zip=False, notes=None):
    entries = sorted(entries, key=lambda e: ((e.get("dir") or ""), e["name"], e["key"]))
    inputs = make_inputs(entries)
    steps, dests = [], set()
    for inp, e in zip(inputs, entries):
        base = f"{e['dir']}/{e['name']}" if e.get("dir") else e["name"]
        if extract_zip and e["url"].endswith(".zip"):
            steps.append({"op": "extract", "input": inp["name"], "format": "zip", **({"dest": e["dir"]} if e.get("dir") else {})})
            continue
        dest = base
        k = 1
        while dest.lower() in dests:
            stem, dot, ext = base.rpartition(".")
            dest = f"{stem} ({k}).{ext}" if dot else f"{base} ({k})"
            k += 1
        dests.add(dest.lower())
        steps.append({"op": "copy", "input": inp["name"], "dest": dest})
    item = {
        "item_id": item_id, "family": family, "split": split, "scale": scale, "kind": "download",
        "real_or_generated": "real",
        "recipe": {"inputs": inputs, "steps": steps, "output_pin": "TOFU"},
        "license": lic, "independence_group": group, "description": description,
    }
    if tags:
        item["tags"] = tags
    if notes:
        item["notes"] = notes
    return item


def build_item(item_id, family, split, scale, group, entries, lic, description, script, seed, params, tags=None, notes=None):
    entries = sorted(entries, key=lambda e: e["key"])
    inputs = make_inputs(entries)
    # stable, meaningful image names for the scripts: map input name -> short name via params.inputs order
    item = {
        "item_id": item_id, "family": family, "split": split, "scale": scale, "kind": "build",
        "real_or_generated": "derived-from-real",
        "recipe": {"inputs": inputs, "generator": {"script": f"{GEN}/{script}", "interpreter": "python", "seed": seed, "params": params},
                   "output_pin": "TOFU",
                   "notes": "Encoded with pinned local toolchain (setup_tools.sh); build kind because bytes depend on encoder versions, "
                            "pinned TOFU and guarded by params.require_tools."},
        "license": lic, "independence_group": group, "description": description,
    }
    if tags:
        item["tags"] = tags
    if notes:
        item["notes"] = notes
    return item


# ---------------------------------------------------------------------------
# JPEG matrix definitions

V_TUNING = [
    "ljt-q50-420", "ljt-q75-420", "ljt-q90-420", "ljt-q95-444", "ljt-q100-444", "ljt-q85-422", "ljt-q80-440", "ljt-q70-411",
    "ljt-q85-420-opt", "ljt-q85-420-prog", "ljt-q92-444-prog", "ljt-q85-420-arith", "ljt-q85-420-arith-prog",
    "ljt-q85-420-rst1r", "ljt-q85-420-rst4b", "ljt-q85-420-dctfloat", "ljt-q85-420-dctfast", "ljt-q85-gray",
    "ljt-q85-420-smooth20", "ljt-q85-420-icc",
    "moz-q75-420", "moz-q90-420-baseline", "moz-q85-444-ssim", "moz-q60-420-notrellis-fastcrush", "moz-q80-420-arith",
    "im-q82-420", "im-q92-422-interlace", "im-q75-444-opt",
    "pil-q75-420", "pil-q95-444-opt", "pil-q80-420-prog",
    "ff-q2-420-opt", "ff-q6-444", "ff-q4-422",
    "ycck-q85-420", "cmyk-im-q85", "cmyk-pil-q88",
    "jt-prog", "jt-opt", "jt-arith", "jt-rst2", "jt-rot90", "jt-crop", "jt-gray", "mozjt", "exif-heavy", "exif-camera",
]
V_VALIDATION = [
    "ljt-q60-420", "ljt-q78-420", "ljt-q88-420", "ljt-q93-444", "ljt-q98-444", "ljt-q82-422", "ljt-q76-440", "ljt-q65-411",
    "ljt-q88-420-opt", "ljt-q88-420-prog", "ljt-q90-444-prog", "ljt-q80-420-arith", "ljt-q86-420-arith-prog",
    "ljt-q88-420-rst1r", "ljt-q88-420-rst4b", "ljt-q88-420-dctfloat", "ljt-q84-420-dctfast", "ljt-q80-gray",
    "ljt-q88-420-icc",
    "moz-q70-420", "moz-q88-420-baseline", "moz-q92-444-ssim", "moz-q65-420-notrellis-fastcrush", "moz-q84-420-arith",
    "im-q85-420", "im-q90-422-interlace",
    "pil-q80-420", "pil-q92-444-opt", "pil-q85-420-prog",
    "ff-q3-420-opt", "ff-q5-444",
    "ycck-q88-420", "cmyk-im-q80", "cmyk-pil-q90",
    "jt-prog", "jt-opt", "jt-arith", "jt-rst2", "jt-rot90", "jt-crop", "jt-gray", "mozjt", "exif-heavy", "exif-camera",
]
V_HELDOUT = [
    "ljt-q55-420", "ljt-q72-420", "ljt-q87-420", "ljt-q94-444", "ljt-q99-444", "ljt-q83-422", "ljt-q79-440", "ljt-q68-411",
    "ljt-q87-420-opt", "ljt-q87-420-prog", "ljt-q91-444-prog", "ljt-q83-420-arith", "ljt-q89-420-arith-prog",
    "ljt-q87-420-rst1r", "ljt-q87-420-rst4b", "ljt-q87-420-dctfloat", "ljt-q81-420-dctfast", "ljt-q86-gray",
    "ljt-q87-420-smooth20", "ljt-q87-420-icc",
    "moz-q72-420", "moz-q86-420-baseline", "moz-q89-444-ssim", "moz-q58-420-notrellis-fastcrush", "moz-q82-420-arith",
    "im-q80-420", "im-q93-422-interlace", "im-q78-444-opt",
    "pil-q70-420", "pil-q93-444-opt", "pil-q82-420-prog",
    "ff-q3-420", "ff-q7-444-opt",
    "ycck-q82-444", "cmyk-im-q88", "cmyk-pil-q84",
    "jt-prog", "jt-opt", "jt-arith", "jt-rst2", "jt-rot90", "jt-crop", "jt-gray", "mozjt", "exif-heavy", "exif-camera",
]
EDGE_COMMON = [
    "trunc-no-eoi", "trunc-header", "trunc-after-ff", "corrupt-zero-4k", "corrupt-random-block-4k",
    "corrupt-rst-missing", "corrupt-rst-order", "corrupt-dht", "corrupt-sof-height-zero", "corrupt-early-eoi",
    "trail-random-4k", "trail-zeros-64k", "trail-text", "trail-zip", "trail-jpeg-concat", "lead-junk-512", "pad-ff-fill",
    "multi-com", "huge-app-tiny-image", "app-after-dqt", "gray-ljt", "gray-im", "cmyk-im", "cmyk-pil", "ycck-tj",
    "cmyk-im-large-icc", "scans-noninterleaved", "scans-progressive-custom", "arith-sequential", "arith-progressive",
    "exif-orientation-6", "exif-thumb-large", "exif-heavy", "exif-camera", "q100-444-optimized", "q1-420",
    "jt-rot90-perfect-or-trim", "duplicate-identical",
]
EDGE_TUNING = ["trunc-10pct", "trunc-50pct", "trunc-90pct", "trunc-99pct", "trunc-prog-50pct", "corrupt-bitflip-1",
               "corrupt-bitflip-64", "dims-1x1", "dims-17x9", "dims-4097x3"] + EDGE_COMMON
EDGE_VALIDATION = ["trunc-25pct", "trunc-75pct", "trunc-95pct", "trunc-prog-30pct", "corrupt-bitflip-4",
                   "corrupt-bitflip-32", "dims-2x2", "dims-33x17", "dims-3x2049"] + EDGE_COMMON
EDGE_HELDOUT = ["trunc-33pct", "trunc-67pct", "trunc-97pct", "trunc-prog-80pct", "corrupt-bitflip-2",
                "corrupt-bitflip-16", "dims-1x7", "dims-15x31", "dims-2047x5"] + EDGE_COMMON

CODEC_FORMATS = ["webp-q75", "webp-q90-sharpyuv", "webp-lossless", "avif-q60-420", "avif-q80-444", "heic-q50", "heic-q80",
                 "jxl-d1", "jxl-lossless", "jxl-from-jpeg", "jp2-rate20", "png-pil-opt", "png-im-interlaced", "png-pal256",
                 "tiff-lzw", "tiff-zip", "tiff-jpeg", "tiff-none", "gif-256", "bmp"]


def lossless_names(entries):
    return [input_name(e) for e in sorted(entries, key=lambda e: e["key"])]


# ---------------------------------------------------------------------------


def build_items(sel, partial=False):
    items, skipped = [], []

    def A(fn):
        try:
            items.append(fn())
        except MissingPool as e:
            if not partial:
                raise SystemExit(f"pool {e} missing or not fetched")
            skipped.append(str(e))

    def lazy(pool):
        class _L:
            def __iter__(self):
                return iter(pool_entries(sel, pool))

            def __getitem__(self, k):
                return pool_entries(sel, pool)[k]

            def __len__(self):
                return len(pool_entries(sel, pool))
        return _L()

    kodak = lazy("kodak-png")
    fsa = lazy("fsa-owi-color-tiff")
    hs = lazy("highsmith-tiff")

    # ======================== F12 tuning ========================
    A(lambda: build_item("f12-tuning-kodak-jpeg-matrix-medium", "F12", "tuning", "medium", "g5-kodak-photocd-suite", kodak, LIC["kodak"],
                 "JPEG producer/mode matrix: all 24 Kodak PhotoCD PNGs encoded by libjpeg-turbo 2.1.5 (quality 50-100, 4:4:4/4:2:2/"
                 "4:2:0/4:4:0/4:1:1/gray, optimized Huffman, progressive, arithmetic, restart intervals, float/fast DCT, smoothing, "
                 "ICC), mozjpeg 4.1.5 (trellis/progressive/baseline/tune-ssim/arithmetic), ImageMagick 6.9.12, Pillow 12.3, FFmpeg "
                 "6.1 mjpeg, TurboJPEG YCCK, CMYK JPEGs, jpegtran lossless transforms (progressive/optimize/arithmetic/restart/"
                 "rotate/crop/gray, mozjpeg jpegtran) and exiftool metadata-heavy rewrites; <variant>/<image>.jpg.",
                 "jpeg_matrix.py", 1201,
                 {"mode": "matrix", "inputs": lossless_names(kodak), "transform_base": "ljt-q90-420", "variants": V_TUNING,
                  "require_tools": TOOLS_JPEG, "max_bytes": 500 * MiB},
                 tags=["public-benchmark", "jpeg-reconstruction", "encoder-matrix"]))
    A(lambda: build_item("f12-tuning-kodak-jpeg-edge-cases-small", "F12", "tuning", "small", "g5-kodak-photocd-suite",
                 [e for e in kodak if e["key"] in ("kodim03", "kodim08", "kodim15", "kodim20", "kodim23")],
                 LIC["kodak"],
                 "Deliberately unusual or malformed JPEGs from 5 Kodak images (box-downscaled 2x): truncations (10-99%, progressive, "
                 "no EOI, inside header), bit flips, zeroed/random blocks, broken restart markers, corrupt DHT/SOF, early EOI, "
                 "trailing random/zeros/text/ZIP/second JPEG, leading junk, 0xFF fill padding, 120 COM segments, 60 KB APP13 on a "
                 "16x16 image, APP after DQT, odd dimensions, gray/CMYK/YCCK, 431 KB multi-chunk ICC, non-interleaved and custom "
                 "progressive scan scripts, arithmetic coding, EXIF orientation/large thumbnail/heavy metadata, camera-style APP1-first "
                 "layout, q1 (extended 16-bit tables) and q100, duplicate file.",
                 "jpeg_matrix.py", 1202,
                 {"mode": "edge", "reduce": 2, "edge_cases": EDGE_TUNING, "require_tools": TOOLS_JPEG, "max_bytes": 15 * MiB},
                 tags=["public-benchmark", "jpeg-reconstruction", "malformed"]))
    for scale in ("small", "medium", "large"):
        iid = f"f12-tuning-nasa-ivl-photos-{scale}"
        A(lambda iid=iid, scale=scale: download_item(iid, "F12", "tuning", scale, "g5-nasa-ivl-photos", entries_for(sel, "nasa-ivl-photos", iid), LIC["nasa-ivl"],
                        f"Real camera-produced JPEG originals (~orig.jpg) from the NASA Image and Video Library, photographs by NASA center "
                        f"photographers 2012-2025 stratified by camera make/model, software and coding process (Nikon, Canon, Sony, "
                        f"Hasselblad, iPhone; Adobe-processed and in-camera); laid out as <center>/<year>/<nasa_id>.jpg ({scale} tier).",
                        tags=["jpeg-reconstruction", "camera-jpeg"]))

    # ======================== F12 validation ========================
    for scale in ("small", "medium"):
        iid = f"f12-validation-commons-cc0-photos-{scale}"
        A(lambda iid=iid, scale=scale: download_item(iid, "F12", "validation", scale, "g5-commons-cc0-photos", entries_for(sel, "commons-cc0-photos", iid),
                        LIC["commons-cc0"],
                        f"Real camera JPEGs from Wikimedia Commons Quality/Featured pictures released under CC0, EXIF Make present, at most two "
                        f"files per uploader, round-robin across camera makes; original Commons file names under photos/ ({scale} tier).",
                        tags=["jpeg-reconstruction", "camera-jpeg"]))
    A(lambda: build_item("f12-validation-fsa-jpeg-matrix-medium", "F12", "validation", "medium", "g5-loc-fsa-owi-color", fsa, LIC["fsa"],
                 "JPEG producer/mode matrix (different quality settings than tuning) over 15 tiles (1152x768/768x1152, native and "
                 "2x box-downscaled) cropped from 5 Library of Congress FSA/OWI colour transparency scans (lossless TIFF, 1939-1945).",
                 "jpeg_matrix.py", 2201,
                 {"mode": "matrix", "inputs": lossless_names(fsa), "tiles": {"count": 3, "size": [1152, 768], "reduce": [1, 2]},
                  "transform_base": "ljt-q92-420", "variants": V_VALIDATION, "require_tools": TOOLS_JPEG, "max_bytes": 500 * MiB},
                 tags=["jpeg-reconstruction", "encoder-matrix"]))
    A(lambda: build_item("f12-validation-fsa-jpeg-edge-cases-small", "F12", "validation", "small", "g5-loc-fsa-owi-color", fsa, LIC["fsa"],
                 "Unusual/malformed JPEG edge cases (as the tuning edge-case item, with different truncation points, flip counts and "
                 "dimensions) from one 768x512 region (box-downscaled 2x to 384x256) of each of 5 FSA/OWI colour transparency scans.",
                 "jpeg_matrix.py", 2202,
                 {"mode": "edge", "inputs": lossless_names(fsa), "tiles": {"count": 1, "size": [384, 256], "reduce": [2]},
                  "edge_cases": EDGE_VALIDATION, "require_tools": TOOLS_JPEG, "max_bytes": 15 * MiB},
                 tags=["jpeg-reconstruction", "malformed"]))

    # ======================== F12 held-out ========================
    for scale in ("small", "medium", "large"):
        iid = f"f12-heldout-usgov-photos-{scale}"
        A(lambda iid=iid, scale=scale: download_item(iid, "F12", "heldout", scale, "g5-commons-usgov-photos", entries_for(sel, "commons-usgov-photos", iid),
                        LIC["commons-usgov"],
                        f"Real JPEG files from Wikimedia Commons tagged PD-USGov-* for eight non-NASA US federal agencies, EXIF Make present, "
                        f"round-robin across agencies; stored as <agency>/<Commons file name> ({scale} tier). Provenance-only description.",
                        tags=["jpeg-reconstruction", "camera-jpeg"]))
    A(lambda: build_item("f12-heldout-highsmith-jpeg-matrix-medium", "F12", "heldout", "medium", "g5-loc-highsmith", hs, LIC["highsmith"],
                 "JPEG producer/mode matrix (held-out quality settings) over tiles cropped from 5 Carol M. Highsmith Archive TIFFs "
                 "(Library of Congress, via Wikimedia Commons). Provenance-only description.",
                 "jpeg_matrix.py", 3201,
                 {"mode": "matrix", "inputs": lossless_names(hs), "tiles": {"count": 3, "size": [1152, 768], "reduce": [2, 1]},
                  "transform_base": "ljt-q89-420", "variants": V_HELDOUT, "require_tools": TOOLS_JPEG, "max_bytes": 500 * MiB},
                 tags=["jpeg-reconstruction", "encoder-matrix"]))
    A(lambda: build_item("f12-heldout-highsmith-jpeg-edge-cases-small", "F12", "heldout", "small", "g5-loc-highsmith", hs, LIC["highsmith"],
                 "Unusual/malformed JPEG edge cases (held-out truncation points, flip counts and dimensions) from one downscaled tile "
                 "of each of 5 Carol M. Highsmith Archive TIFFs. Provenance-only description.",
                 "jpeg_matrix.py", 3202,
                 {"mode": "edge", "inputs": lossless_names(hs), "tiles": {"count": 1, "size": [384, 256], "reduce": [3]},
                  "edge_cases": EDGE_HELDOUT, "require_tools": TOOLS_JPEG, "max_bytes": 15 * MiB},
                 tags=["jpeg-reconstruction", "malformed"]))

    # ======================== F13 tuning ========================
    A(lambda: download_item("f13-tuning-bbb-video-medium", "F13", "tuning", "medium", "g5-blender-bbb",
                    entries_for(sel, "blender", "f13-tuning-bbb-video-medium"), LIC["bbb"],
                    "Big Buck Bunny open movie renditions from download.blender.org (stored ZIPs extracted): 320x180 H.264/AAC MP4, "
                    "480p Ogg Theora/Vorbis, 480p AVI (MPEG-4 Part 2 + MP3).",
                    tags=["public-benchmark", "video"], extract_zip=True))
    A(lambda: download_item("f13-tuning-bbb-video-large", "F13", "tuning", "large", "g5-blender-bbb",
                    entries_for(sel, "blender", "f13-tuning-bbb-video-large"), LIC["bbb"],
                    "Big Buck Bunny renditions: 720p H.264 QuickTime MOV, 640x360 M4V, 2013 'sunflower' 1080p30 H.264 MP4 (deflated in "
                    "its ZIP), and the Wikimedia Commons 480p VP9 WebM server transcode.",
                    tags=["public-benchmark", "video"], extract_zip=True,
                    notes="The Commons transcode is regenerated by Wikimedia occasionally; its declared sha256 detects drift."))
    A(lambda: download_item("f13-tuning-prelinger-video-medium", "F13", "tuning", "medium", "g5-prelinger-archives",
                    entries_for(sel, "prelinger-video", "f13-tuning-prelinger-video-medium"), LIC["prelinger"],
                    "Public-domain Prelinger Archives films (Internet Archive): per film the archive.org MPEG-4 (H.264/AAC) and Ogg "
                    "Theora/Vorbis derivatives, stored as <identifier>/<file>.", tags=["video"],
                    notes="Replaces a planned Wikimedia Commons CC0 WebM set: upload.wikimedia.org returned HTTP 429 (Retry-After 600) "
                          "for bulk original-file downloads on 2026-09-12."))
    A(lambda: download_item("f13-tuning-librivox-audiobook-medium", "F13", "tuning", "medium", "g5-librivox-1900-last-president",
                    entries_for(sel, "librivox", "f13-tuning-librivox-audiobook-medium"), LIC["librivox"],
                    "LibriVox audiobook '1900; or, The Last President' (Ingersoll Lockwood): 128 kbps MP3 chapter files, Ogg Vorbis "
                    "chapter files and the M4B (AAC) audiobook from the Internet Archive item 1900lastpresident_1904_librivox.",
                    tags=["audio"]))
    A(lambda: download_item("f13-tuning-musopen-flac-medium", "F13", "tuning", "medium", "g5-musopen-kickstarter-flac",
                    entries_for(sel, "musopen-flac", "f13-tuning-musopen-flac-medium"), LIC["musopen-dvd"],
                    "Real FLAC orchestral recordings (Borodin, In the Steppes of Central Asia; Brahms Symphony No. 1 mvts II-III) from "
                    "the Musopen Kickstarter lossless compilation.", tags=["audio", "lossless-audio"]))
    A(lambda: download_item("f13-tuning-nasa-ntrs-pdf-small", "F13", "tuning", "small", "g5-nasa-ntrs-pdf",
                    entries_for(sel, "nasa-ntrs-pdf", "f13-tuning-nasa-ntrs-pdf-small"), LIC["ntrs"],
                    "Real PDF technical reports (NASA TM/TP/CP/CR, 2019-2025) from the NASA Technical Reports Server, public-use permitted.",
                    tags=["pdf"]))
    A(lambda: download_item("f13-tuning-commons-media-small", "F13", "tuning", "small", "g5-commons-media-set-a",
                    entries_for(sel, "commons-media", "f13-tuning-commons-media-small"), LIC["commons-cc0"],
                    "Mixed real CC0 media from Wikimedia Commons (set a): PNG, GIF, WebP, TIFF images; Ogg, FLAC, WAV, MP3 audio; a WebM "
                    "video; each file from a distinct uploader not used by sets b/c; <type>/<Commons file name>.",
                    tags=["mixed-media"]))
    A(lambda: build_item("f13-tuning-kodak-image-codecs-small", "F13", "tuning", "small", "g5-kodak-photocd-suite",
                 [e for e in kodak if e["key"] in ("kodim05", "kodim19")],
                 LIC["kodak"],
                 "Two Kodak PhotoCD images encoded to non-JPEG image containers: WebP (lossy, sharp-YUV, lossless), AVIF (aom 4:2:0/4:4:4), "
                 "HEIC (x265), JPEG XL (lossy, lossless, lossless JPEG transcode), JPEG 2000, PNG (optimized, Adam7, 256-colour palette), "
                 "TIFF (LZW, Deflate, JPEG-in-TIFF, uncompressed), GIF and BMP.",
                 "image_codecs.py", 1301,
                 {"formats": CODEC_FORMATS, "big_formats_max_inputs": 1, "require_tools": TOOLS_CODECS, "max_bytes": 15 * MiB},
                 tags=["public-benchmark", "image-codecs"]))

    # ======================== F13 validation ========================
    A(lambda: download_item("f13-validation-sintel-media-medium", "F13", "validation", "medium", "g5-blender-sintel",
                    entries_for(sel, "blender", "f13-validation-sintel-media-medium"), LIC["sintel"],
                    "Sintel (Blender open movie) trailer renditions: H.264 MP4 480p/720p/1080p, Ogg Theora 480p/720p, DivX Plus HD MKV "
                    "480p/720p, plus the film's 5.1 music+effects stem as FLAC.", tags=["public-benchmark", "video", "lossless-audio"]))
    A(lambda: download_item("f13-validation-librivox-audiobook-medium", "F13", "validation", "medium", "g5-librivox-tenn-sf-stories",
                    entries_for(sel, "librivox", "f13-validation-librivox-audiobook-medium"), LIC["librivox"],
                    "LibriVox audiobook '3 SF Stories by William Tenn': VBR MP3 and Ogg Vorbis files from the Internet Archive item "
                    "3sfstoriesbywilliamtenn_1910_librivox.", tags=["audio"]))
    A(lambda: download_item("f13-validation-federal-register-pdf-small", "F13", "validation", "small", "g5-federal-register-pdf",
                    entries_for(sel, "federal-register-pdf", "f13-validation-federal-register-pdf-small"), LIC["fedreg"],
                    "Real PDF Federal Register documents (rules, proposed rules, notices, presidential documents) from three 2025 issue "
                    "dates, as published on govinfo.gov; <date>/<type>/<document number>.pdf.", tags=["pdf"]))
    A(lambda: download_item("f13-validation-commons-media-small", "F13", "validation", "small", "g5-commons-media-set-b",
                    entries_for(sel, "commons-media", "f13-validation-commons-media-small"), LIC["commons-cc0"],
                    "Mixed real CC0 media from Wikimedia Commons (set b, uploaders disjoint from sets a/c): PNG, GIF, WebP, TIFF, Ogg, FLAC, "
                    "WAV, MP3, WebM.", tags=["mixed-media"]))
    A(lambda: build_item("f13-validation-fsa-image-codecs-small", "F13", "validation", "small", "g5-loc-fsa-owi-color", sorted(fsa, key=lambda e: e["key"])[:2],
                 LIC["fsa"],
                 "One 1536x1024 region (box-downscaled 2x to 768x512) from each of two FSA/OWI colour transparency scans encoded to WebP/AVIF/HEIC/"
                 "JPEG XL/JPEG 2000/PNG/TIFF/GIF/BMP variants.",
                 "image_codecs.py", 2301,
                 {"tiles": {"count": 1, "size": [768, 512], "reduce": [2]}, "formats": CODEC_FORMATS, "big_formats_max_inputs": 1,
                  "require_tools": TOOLS_CODECS, "max_bytes": 15 * MiB},
                 tags=["image-codecs"]))

    # ======================== F13 held-out ========================
    A(lambda: download_item("f13-heldout-tos-media-large", "F13", "heldout", "large", "g5-blender-tears-of-steel",
                    entries_for(sel, "blender", "f13-heldout-tos-media-large"), LIC["tos"],
                    "Tears of Steel (Blender open movie) files from download.blender.org: 1080p WebM (stored ZIP extracted) and the DVD "
                    "stereo mix AIFF. Provenance-only description.", tags=["public-benchmark", "video"], extract_zip=True))
    A(lambda: download_item("f13-heldout-elephants-dream-media-medium", "F13", "heldout", "medium", "g5-blender-elephants-dream",
                    entries_for(sel, "blender", "f13-heldout-elephants-dream-media-medium"), LIC["ed"],
                    "Elephants Dream (Blender open movie) files from download.blender.org: 480p and 720p H.264/AAC QuickTime MOVs and "
                    "the 5.1 music+effects AC-3 stream. Provenance-only description.", tags=["video"]))
    A(lambda: download_item("f13-heldout-librivox-audiobook-medium", "F13", "heldout", "medium", "g5-librivox-reynolds-sf-stories",
                    entries_for(sel, "librivox", "f13-heldout-librivox-audiobook-medium"), LIC["librivox"],
                    "LibriVox audiobook files (128 kbps MP3 and M4B) from the Internet Archive item 4sfstoriesbymackreynolds_2105_librivox. "
                    "Provenance-only description.", tags=["audio"]))
    A(lambda: download_item("f13-heldout-musopen-chopin-flac-medium", "F13", "heldout", "medium", "g5-musopen-chopin-flac",
                    entries_for(sel, "musopen-flac", "f13-heldout-musopen-chopin-flac-medium"), LIC["musopen-chopin"],
                    "FLAC files from the Internet Archive item musopen-chopin-complete-works-flac (five pieces). Provenance-only description.",
                    tags=["audio", "lossless-audio"]))
    A(lambda: download_item("f13-heldout-usgs-pdf-small", "F13", "heldout", "small", "g5-usgs-pubs-pdf",
                    entries_for(sel, "usgs-pdf", "f13-heldout-usgs-pdf-small"), LIC["usgs"],
                    "PDF reports from the USGS Publications Warehouse numbered series (2020-2023). Provenance-only description.", tags=["pdf"]))
    A(lambda: download_item("f13-heldout-commons-media-small", "F13", "heldout", "small", "g5-commons-media-set-c",
                    entries_for(sel, "commons-media", "f13-heldout-commons-media-small"), LIC["commons-cc0"],
                    "Mixed CC0 media files from Wikimedia Commons (set c, uploaders disjoint from sets a/b). Provenance-only description.",
                    tags=["mixed-media"]))
    A(lambda: build_item("f13-heldout-highsmith-image-codecs-small", "F13", "heldout", "small", "g5-loc-highsmith", sorted(hs, key=lambda e: e["key"])[:2],
                 LIC["highsmith"],
                 "Tiles from two Carol M. Highsmith Archive TIFFs encoded to non-JPEG image container variants. Provenance-only description.",
                 "image_codecs.py", 3301,
                 {"tiles": {"count": 1, "size": [768, 512], "reduce": [3]}, "formats": CODEC_FORMATS, "big_formats_max_inputs": 1,
                  "require_tools": TOOLS_CODECS, "max_bytes": 15 * MiB},
                 tags=["image-codecs"]))
    if skipped:
        print(f"partial emit: skipped items needing pools {sorted(set(skipped))}")
    return items
