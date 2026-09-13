#!/usr/bin/env python3
"""g5-media (F12 JPEG/images, F13 other media) source selection helper.

This is a one-time *selection* tool, not a provision script.  It is kept in the repository so
the choice of upstream files is documented and repeatable; the committed outputs are the
authority:

  research/corpus/generators/g5-media/selection/<pool>.json   chosen files + license + API facts
  research/corpus/sources/g5-media.json                        ebrc source definitions

Stages (each idempotent; run inside WSL with the ebrc venv python):
  select  Query public APIs with fixed rules (NASA Image and Video Library, Wikimedia Commons,
          Internet Archive (LibriVox / Musopen), NASA NTRS, Federal Register, USGS Pubs, fixed
          Blender open-movie and Kodak URLs).  A pool whose selection file exists is NOT queried
          again (use --refresh POOL to redo one), so reruns are stable even though the APIs change.
          Selection uses only provenance metadata (license, author, EXIF producer tags, byte
          sizes) -- never content statistics -- so it is safe for held-out pools.
  fetch   Download every chosen URL into the ebrc content-addressed cache through
          provision.fetch_url (identical code path and provenance index as provisioning),
          verify API-declared size / SHA-1 / MD5, record SHA-256.  Already-recorded entries are
          re-checked against the cache only.
  emit    Write research/corpus/sources/g5-media.json with declared sha256 + size per input.

Usage:
  wsl.exe -d Ubuntu -- bash /mnt/d/Projects/entrybound/entrybound/research/corpus/generators/g5-media/select.sh [select|fetch|emit|all] [--pool P] [--refresh P]
"""

from __future__ import annotations

import argparse
import hashlib
import html
import json
import os
import random
import re
import sys
import threading
import time
import urllib.error
import urllib.parse
import urllib.request
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[3]
sys.path.insert(0, str(REPO / "research" / "corpus" / "tools"))
sys.dont_write_bytecode = True
import corpuslib as cl  # noqa: E402
import provision  # noqa: E402

SEL_DIR = HERE / "selection"
SOURCES_OUT = REPO / "research" / "corpus" / "sources" / "g5-media.json"
UA = provision.USER_AGENT
SELECTION_SEED = 20260912
MiB = 1 << 20
_lock = threading.Lock()


def log(msg):
    with _lock:
        print(msg, flush=True)


# ---------------------------------------------------------------------------
# HTTP helpers


def http(url, data=None, headers=None, method=None, timeout=120, tries=8):
    hdrs = {"User-Agent": UA}
    if headers:
        hdrs.update(headers)
    last = None
    for attempt in range(1, tries + 1):
        try:
            req = urllib.request.Request(url, data=data, headers=hdrs, method=method)
            with urllib.request.urlopen(req, timeout=timeout) as r:
                return r.status, dict(r.headers), (r.read() if method != "HEAD" else b""), r.geturl()
        except urllib.error.HTTPError as e:
            if e.code in (404, 410, 403):
                return e.code, dict(e.headers or {}), b"", url
            last = e
            if e.code in (429, 503):
                retry_after = (e.headers or {}).get("Retry-After", "")
                wait = int(retry_after) if retry_after.isdigit() else 30 * attempt
                log(f"  HTTP {e.code} from {urllib.parse.urlparse(url).hostname}; sleeping {wait}s")
                time.sleep(wait)
                continue
        except Exception as e:  # noqa: BLE001
            last = e
        time.sleep(2 * attempt)
    raise RuntimeError(f"HTTP failed for {url}: {last}")


def get_json(url, data=None, headers=None):
    st, _, body, _ = http(url, data=data, headers=headers)
    if st != 200:
        raise RuntimeError(f"HTTP {st} for {url}")
    return json.loads(body)


def head(url):
    st, h, _, final = http(url, method="HEAD")
    h = {k.lower(): v for k, v in h.items()}
    size = int(h["content-length"]) if st == 200 and "content-length" in h else None
    return st, size, h.get("etag", "").strip('"'), final


def strip_html(s):
    return html.unescape(re.sub(r"<[^>]+>", "", s or "")).strip()


def safe_name(name):
    name = re.sub(r"[\x00-\x1f\x7f/\\]", "_", name).strip()
    return name[:180] if len(name.encode()) <= 200 else name.encode()[:190].decode("utf-8", "ignore")


# ---------------------------------------------------------------------------
# Wikimedia Commons

COMMONS_API = "https://commons.wikimedia.org/w/api.php"
EXT_FILTER = "LicenseShortName|License|Artist|Credit|UsageTerms|LicenseUrl|AttributionRequired|Copyrighted|DateTimeOriginal"
BLOCK_ORIGIN = re.compile(r"NASA|JPL|Jet Propulsion|Blender|LibriVox|Musopen|Kodak|Library of Congress|Highsmith|"
                          r"Farm Security|Office of War Information|Big Buck Bunny|Sintel|Tears of Steel|Elephants Dream",
                          re.I)


_api_lock = threading.Lock()
_api_last = [0.0]
COMMONS_API_SPACING = 20.0  # seconds; Wikimedia rate-limits anonymous API clients per IP (HTTP 429)


def commons_search(search, want, gsrlimit=50, max_pages=None):
    """Return page dicts (formatversion=2) in search-rank order with imageinfo + common metadata.
    Requests are serialized and spaced (COMMONS_API_SPACING) to stay under Wikimedia's API limits."""
    out, offset = [], 0
    max_pages = max_pages or max(1, -(-want // gsrlimit))
    for _ in range(max_pages):
        params = {
            "action": "query", "format": "json", "formatversion": "2", "generator": "search",
            "gsrnamespace": "6", "gsrsearch": search, "gsrlimit": str(gsrlimit), "gsroffset": str(offset),
            "prop": "imageinfo", "iiprop": "url|size|sha1|mime|extmetadata|commonmetadata|user|timestamp",
            "iiextmetadatafilter": EXT_FILTER,
        }
        with _api_lock:
            wait = _api_last[0] + COMMONS_API_SPACING - time.time()
            if wait > 0:
                time.sleep(wait)
            d = get_json(COMMONS_API + "?" + urllib.parse.urlencode(params))
            _api_last[0] = time.time()
        pages = sorted((d.get("query") or {}).get("pages", []), key=lambda p: p.get("index", 0))
        out.extend(p for p in pages if p.get("imageinfo"))
        if len(out) >= want or "continue" not in d:
            break
        offset = d["continue"]["gsroffset"]
    return out


def commons_entry(p):
    ii = p["imageinfo"][0]
    em = {k: (v.get("value") if isinstance(v, dict) else v) for k, v in (ii.get("extmetadata") or {}).items()}
    md = {m["name"]: m["value"] for m in (ii.get("commonmetadata") or []) if isinstance(m, dict) and "name" in m}
    title = p["title"][5:] if p["title"].startswith("File:") else p["title"]
    lic = strip_html(em.get("LicenseShortName"))
    artist = strip_html(em.get("Artist")) or ii.get("user", "")
    credit = strip_html(em.get("Credit"))
    return {
        "key": f"commons-{p['pageid']}",
        "url": ii["url"].split("?")[0],
        "name": safe_name(title),
        "api_size": ii["size"],
        "api_sha1": ii.get("sha1"),
        "mime": ii.get("mime"),
        "license": {"spdx_or_name": lic, "url": em.get("LicenseUrl") or "", "usage_terms": strip_html(em.get("UsageTerms")),
                    "attribution": artist[:200], "credit": credit[:200], "source_page": ii.get("descriptionurl", "")},
        "uploader": ii.get("user", ""),
        "meta": {k: str(md[k])[:80] for k in ("Make", "Model", "Software") if k in md},
        "width": ii.get("width"), "height": ii.get("height"),
    }


def lic_ok(e, allowed):
    s = (e["license"]["spdx_or_name"] + " " + e["license"]["usage_terms"]).lower()
    return any(a in s for a in allowed)


PD_TERMS = ("cc0", "public domain")


# ---------------------------------------------------------------------------
# allocation helpers


def round_robin(entries, keyf):
    buckets, order = {}, []
    for e in entries:
        k = keyf(e)
        if k not in buckets:
            buckets[k] = []
            order.append(k)
        buckets[k].append(e)
    out = []
    while any(buckets.values()):
        for k in order:
            if buckets[k]:
                out.append(buckets[k].pop(0))
    return out


def allocate(entries, plan, sizef=lambda e: e["api_size"]):
    """plan: list of (item_id, budget_bytes, max_file_bytes, min_total_bytes).  Greedy in order."""
    used = set()
    for item_id, budget, max_file, min_total in plan:
        total = 0
        for e in entries:
            if e["key"] in used or sizef(e) is None or sizef(e) > max_file:
                continue
            if total + sizef(e) > budget:
                continue
            e["alloc"] = item_id
            used.add(e["key"])
            total += sizef(e)
        if total < min_total:
            raise RuntimeError(f"allocation for {item_id} reached only {total} bytes (< {min_total})")
        log(f"  allocated {item_id}: {sum(1 for e in entries if e.get('alloc') == item_id)} files, {total} bytes")
    return [e for e in entries if e.get("alloc")]


# ---------------------------------------------------------------------------
# pools

POOLS = {}


def pool(name):
    def deco(f):
        POOLS[name] = f
        return f
    return deco


NASA_BLOCK = re.compile(r"SpaceX|Boeing|Getty|Reuters|AP Photo|Associated Press|Lockheed|Northrop|Blue Origin|Axiom|"
                        r"Sierra Space|Airbus|Bigelow|Roscosmos|GCTC|courtesy|Gerst|Kuipers|Pesquet|Cristoforetti|Parmitano|Nespoli|"
                        r"Vittori|Hadfield|Wakata|Noguchi|Hoshide|Furukawa|Kanai|Zelentsov|Sinyak", re.I)
NASA_BLOCK_AGENCY = re.compile(r"(?<![A-Za-z])(ESA|JAXA|CSA|DLR|CNES|ISRO)(?![A-Za-z])")


def third_party(text):
    return bool(NASA_BLOCK.search(text) or NASA_BLOCK_AGENCY.search(text))


@pool("nasa-ivl-photos")
def pool_nasa_ivl_photos():
    """F12 tuning: NASA Image and Video Library photographs from NASA centers, camera-produced JPEG
    originals (~orig.jpg) with EXIF producer tags; stratified by (Make, Model, Software, coding)."""
    centers = ["JSC", "KSC", "HQ", "GRC", "MSFC", "AFRC", "LARC", "GSFC", "ARC", "SSC"]
    cands = {}
    for c in centers:
        for y in range(2012, 2026):
            q = urllib.parse.urlencode({"media_type": "image", "center": c, "year_start": y, "year_end": y,
                                        "page_size": 100, "page": 1})
            try:
                d = get_json("https://images-api.nasa.gov/search?" + q)
            except RuntimeError as e:
                log(f"  nasa search {c} {y}: {e}")
                continue
            for it in d["collection"]["items"]:
                dd = it["data"][0]
                nid = dd.get("nasa_id", "")
                if not re.match(r"^[A-Za-z0-9_.-]+$", nid) or nid.upper().startswith("PIA"):
                    continue
                cands[nid] = {"nasa_id": nid, "center": dd.get("center", c), "date": dd.get("date_created", ""),
                              "photographer": dd.get("photographer", "") or "", "secondary_creator": dd.get("secondary_creator", "") or "",
                              "title": (dd.get("title") or "")[:120], "description_has_credit": third_party(dd.get("description") or "")}
            time.sleep(0.2)
    ids = sorted(cands)
    random.Random(SELECTION_SEED).shuffle(ids)
    log(f"  nasa: {len(ids)} candidate ids")
    picked = []

    def probe(nid):
        c = cands[nid]
        try:
            md = get_json(f"https://images-assets.nasa.gov/image/{nid}/metadata.json")
        except Exception:  # noqa: BLE001
            return None
        make, model = md.get("EXIF:Make"), md.get("EXIF:Model")
        if not make or md.get("File:FileType") != "JPEG":
            return None
        credit = " ".join(str(md.get(k, "")) for k in ("EXIF:Copyright", "EXIF:Artist", "XMP:Rights", "IPTC:CopyrightNotice",
                                                        "XMP:Credit", "IPTC:Credit", "Photoshop:Credit"))
        who = f"{c['photographer']} {c['secondary_creator']} {credit}"
        if third_party(who) or c["description_has_credit"]:
            return None
        if "NASA" not in who.upper() and not c["photographer"]:
            return None
        url = f"https://images-assets.nasa.gov/image/{nid}/{nid}~orig.jpg"
        st, size, etag, _ = head(url)
        if st != 200 or size is None or not (400_000 <= size <= 25 * MiB):
            return None
        year = (c["date"] or "0000")[:4]
        return {
            "key": f"nasa-{nid}", "url": url, "name": f"{nid}.jpg", "dir": f"{c['center']}/{year}",
            "api_size": size, "api_md5": etag if re.match(r"^[0-9a-f]{32}$", etag or "") else None,
            "license": {"spdx_or_name": "NASA media (US Government work, public domain in the US)",
                        "attribution": f"NASA/{c['photographer'] or c['center']}".strip(),
                        "source_page": f"https://images.nasa.gov/details/{nid}"},
            "meta": {"Make": str(make)[:40], "Model": str(model)[:40], "Software": str(md.get("EXIF:Software", ""))[:60],
                     "EncodingProcess": md.get("File:EncodingProcess", ""), "center": c["center"], "date": c["date"][:10]},
        }

    with ThreadPoolExecutor(max_workers=6) as ex:
        for batch_start in range(0, min(len(ids), 1400), 120):
            batch = ids[batch_start:batch_start + 120]
            for fut in as_completed([ex.submit(probe, n) for n in batch]):
                r = fut.result()
                if r:
                    picked.append(r)
            if len(picked) >= 420:
                break
    picked.sort(key=lambda e: e["key"])
    random.Random(SELECTION_SEED + 1).shuffle(picked)

    def strat(e):
        sw = re.split(r"[ /(]", e["meta"]["Software"] or "none")[0].lower()
        return (e["meta"]["Make"].lower(), e["meta"]["Model"].lower(), sw, e["meta"]["EncodingProcess"])

    ordered = round_robin(picked, strat)
    return allocate(ordered, [
        ("f12-tuning-nasa-ivl-photos-small", 15 * MiB, 3 * MiB, 8 * MiB),
        ("f12-tuning-nasa-ivl-photos-medium", 480 * MiB, 25 * MiB, 300 * MiB),
        ("f12-tuning-nasa-ivl-photos-large", 900 * MiB, 25 * MiB, 600 * MiB),
    ])


def commons_photo_pool(searches, allowed_lic, per_uploader, exclude_origin=True, need_make=True,
                       min_size=700_000, max_size=15 * MiB, want=500):
    seen, out = set(), []
    for label, s in searches:
        for p in commons_search(s, want=want):
            e = commons_entry(p)
            if e["key"] in seen:
                continue
            seen.add(e["key"])
            if e["mime"] != "image/jpeg" or not (min_size <= e["api_size"] <= max_size):
                continue
            if not lic_ok(e, allowed_lic) or (need_make and not e["meta"].get("Make")):
                continue
            if exclude_origin and BLOCK_ORIGIN.search(" ".join([e["license"]["attribution"], e["license"]["credit"], e["name"]])):
                continue
            e["group_label"] = label
            out.append(e)
    counts, res = {}, []
    for e in out:
        u = e["uploader"] or e["license"]["attribution"]
        if counts.get(u, 0) >= per_uploader:
            continue
        counts[u] = counts.get(u, 0) + 1
        res.append(e)
    return res


@pool("commons-cc0-photos")
def pool_commons_cc0_photos():
    """F12 validation: Wikimedia Commons Quality/Featured photographs released under CC0, camera JPEGs
    with EXIF Make, at most 2 files per uploader, stratified by camera make."""
    ents = commons_photo_pool([
        ("qi", "filemime:image/jpeg incategory:Quality_images haswbstatement:P275=Q6938433 filesize:700,15000"),
        ("fp", "filemime:image/jpeg incategory:Featured_pictures_on_Wikimedia_Commons haswbstatement:P275=Q6938433 filesize:700,15000"),
    ], ("cc0",), per_uploader=2, want=150)
    for e in ents:
        e["dir"] = "photos"
    ordered = round_robin(ents, lambda e: e["meta"].get("Make", "").lower())
    return allocate(ordered, [
        ("f12-validation-commons-cc0-photos-small", 15 * MiB, 3 * MiB, 8 * MiB),
        ("f12-validation-commons-cc0-photos-medium", 300 * MiB, 15 * MiB, 150 * MiB),
    ])


USGOV_TEMPLATES = ["PD-USGov-FWS", "PD-USGov-NPS", "PD-USGov-Military-Navy", "PD-USGov-Military-Army",
                   "PD-USGov-Military-Air Force", "PD-USGov-USDA", "PD-USGov-NOAA", "PD-USGov-USGS"]


@pool("commons-usgov-photos")
def pool_commons_usgov_photos():
    """F12 held-out: Wikimedia Commons photographs by US federal agencies other than NASA (PD-USGov-*
    templates), camera JPEGs with EXIF Make, round-robin across agencies."""
    searches = [(t, f'filemime:image/jpeg hastemplate:"{t}" filesize:900,15000') for t in USGOV_TEMPLATES]
    ents = commons_photo_pool(searches, PD_TERMS, per_uploader=40, want=100)
    for e in ents:
        e["dir"] = re.sub(r"[^A-Za-z0-9]+", "-", e["group_label"].replace("PD-USGov-", "")).strip("-")
    random.Random(SELECTION_SEED + 2).shuffle(ents)
    ordered = round_robin(ents, lambda e: e["group_label"])
    return allocate(ordered, [
        ("f12-heldout-usgov-photos-small", 15 * MiB, 3 * MiB, 8 * MiB),
        ("f12-heldout-usgov-photos-medium", 300 * MiB, 15 * MiB, 150 * MiB),
        ("f12-heldout-usgov-photos-large", 850 * MiB, 15 * MiB, 540 * MiB),
    ])


@pool("kodak-png")
def pool_kodak():
    """F12/F13 tuning lossless source: Kodak Lossless True Color Image Suite (24 PNG, PhotoCD PCD0992)."""
    out = []
    for i in range(1, 25):
        url = f"https://r0k.us/graphics/kodak/kodak/kodim{i:02d}.png"
        st, size, _, _ = head(url)
        if st != 200:
            raise RuntimeError(f"kodak {url}: HTTP {st}")
        out.append({"key": f"kodim{i:02d}", "url": url, "name": f"kodim{i:02d}.png", "api_size": size,
                    "license": {"spdx_or_name": "Kodak PhotoCD PCD0992 test images (released by Eastman Kodak for unrestricted usage, per r0k.us)",
                                "attribution": "Eastman Kodak Company; PNG conversion by Rich Franzen (r0k.us)",
                                "source_page": "https://r0k.us/graphics/kodak/"}, "alloc": "kodak"})
    return out


def commons_tiff_pool(search, n, max_size):
    ents = []
    for p in commons_search(search, want=50):
        e = commons_entry(p)
        if e["mime"] != "image/tiff" or e["api_size"] > max_size or e["api_size"] < 20 * MiB:
            continue
        if not lic_ok(e, PD_TERMS + ("no known restrictions", "pd-")):
            continue
        ents.append(e)
    ents.sort(key=lambda e: e["key"])
    random.Random(SELECTION_SEED + 3).shuffle(ents)
    return ents[:n]


@pool("fsa-owi-color-tiff")
def pool_fsa():
    """F12/F13 validation lossless source: Library of Congress FSA/OWI color transparency scans (TIFF,
    1939-1945, public domain) via Wikimedia Commons."""
    ents = commons_tiff_pool('filemime:image/tiff insource:"fsac" filesize:20000,70000', 5, 70 * MiB)
    for e in ents:
        e["alloc"] = "fsa"
    return ents


@pool("highsmith-tiff")
def pool_highsmith():
    """F12/F13 held-out lossless source: Carol M. Highsmith Archive 8-bit master TIFFs ('u' masters) from the
    Library of Congress (tile.loc.gov), taken directly from LoC because Wikimedia rate-limits original-file
    downloads.  Seeded random pick among collection result pages; masters of 40-95 MB; item rights advisory must
    read 'No known restrictions on publication'."""
    cands = set()
    for sp in (2, 7, 13, 21, 34):
        d = get_json(f"https://www.loc.gov/collections/carol-m-highsmith/?fo=json&c=100&sp={sp}")
        for r in d.get("results", []):
            m = re.search(r"service:pnp:highsm:(\d+):(\d+)", " ".join(r.get("image_url") or []))
            if m and str(r.get("id", "")).startswith("http"):
                cands.add((r["id"], m.group(1), m.group(2)))
        time.sleep(6)
    cands = sorted(cands)
    random.Random(SELECTION_SEED + 7).shuffle(cands)
    out = []
    for page, a, b in cands:
        if len(out) >= 5:
            break
        url = f"https://tile.loc.gov/storage-services/master/pnp/highsm/{a}/{b}u.tif"
        st, size, _, _ = head(url)
        if st != 200 or size is None or not (40 * MiB <= size <= 95 * MiB):
            continue
        item = get_json(page.rstrip("/") + "/?fo=json")
        time.sleep(4)
        it = item.get("item", {})
        rights = " ".join(it.get("rights_advisory") or []) if isinstance(it.get("rights_advisory"), list) else str(it.get("rights_advisory", ""))
        if "no known restrictions" not in rights.lower():
            continue
        out.append({"key": f"loc-highsm-{b}", "url": url, "name": f"highsm-{b}u.tif", "api_size": size, "alloc": "highsmith",
                    "title": str(it.get("title", ""))[:160],
                    "license": {"spdx_or_name": "Public domain (LoC: " + rights.strip()[:80] + ")",
                                "attribution": "Carol M. Highsmith Archive, Library of Congress, Prints and Photographs Division",
                                "source_page": page}})
    if len(out) < 5:
        raise RuntimeError("highsmith: fewer than 5 masters found")
    return out


COMMONS_MEDIA_BUCKETS = [
    ("image/png", 5, 40_000, 1_400_000, "images/png"),
    ("image/gif", 3, 30_000, 1_400_000, "images/gif"),
    ("image/webp", 5, 20_000, 1_000_000, "images/webp"),
    ("image/tiff", 1, 100_000, 2_000_000, "images/tiff"),
    ("application/ogg", 2, 150_000, 1_000_000, "audio/ogg"),
    ("audio/flac", 1, 200_000, 1_500_000, "audio/flac"),
    ("audio/wav", 1, 200_000, 1_500_000, "audio/wav"),
    ("audio/mpeg", 1, 200_000, 1_200_000, "audio/mp3"),
    ("video/webm", 1, 400_000, 3_000_000, "video/webm"),
]


@pool("commons-media")
def pool_commons_media():
    """F13 small mixed-media sets a/b/c (tuning/validation/held-out): CC0 files from Wikimedia Commons
    by MIME type; every file in all three sets has a distinct uploader; NASA/LoC/Blender/etc. origins
    excluded (they are independence groups elsewhere)."""
    sets = [("f13-tuning-commons-media-small", "a"), ("f13-validation-commons-media-small", "b"),
            ("f13-heldout-commons-media-small", "c")]
    used_uploaders, out = set(), []
    pools = {}
    for mime, n, lo, hi, d in COMMONS_MEDIA_BUCKETS:
        s = f"filemime:{mime} haswbstatement:P275=Q6938433 filesize:{lo // 1000},{hi // 1000}"
        cand = []
        for p in commons_search(s, want=100 if mime.startswith(("audio", "application")) else 50):
            e = commons_entry(p)
            if e["mime"] != mime or not (lo <= e["api_size"] <= hi) or not lic_ok(e, ("cc0",)):
                continue
            if BLOCK_ORIGIN.search(" ".join([e["license"]["attribution"], e["license"]["credit"], e["name"]])):
                continue
            e["dir"] = d
            cand.append(e)
        pools[mime] = cand
    budget = 15 * MiB
    totals = {iid: 0 for iid, _ in sets}
    for mime, n, lo, hi, d in COMMONS_MEDIA_BUCKETS:
        for iid, _ in sets:
            got = 0
            for e in pools[mime]:
                if got >= n:
                    break
                u = e["uploader"] or e["license"]["attribution"]
                if e.get("alloc") or u in used_uploaders or totals[iid] + e["api_size"] > budget:
                    continue
                e["alloc"] = iid
                used_uploaders.add(u)
                totals[iid] += e["api_size"]
                got += 1
                out.append(e)
            if got == 0:
                raise RuntimeError(f"commons-media: no {mime} file for {iid}")
    for iid, _ in sets:
        log(f"  allocated {iid}: {sum(1 for e in out if e['alloc'] == iid)} files, {totals[iid]} bytes")
    return out


@pool("prelinger-video")
def pool_prelinger():
    """F13 tuning medium (replaces Commons CC0 video, whose original downloads Wikimedia rate-limits): public-domain
    Prelinger Archives films on the Internet Archive; per film the MPEG-4 (h.264 / 512Kb MPEG4) and Ogg Video
    derivatives; films chosen by identifier order among PD-licensed items of 100-700 MB."""
    q = urllib.parse.urlencode([("q", "collection:prelinger AND licenseurl:*publicdomain* AND item_size:[100000000 TO 700000000]"),
                                ("fl[]", "identifier"), ("rows", "40"), ("sort[]", "identifier asc"), ("output", "json")])
    ids = [x["identifier"] for x in get_json("https://archive.org/advancedsearch.php?" + q)["response"]["docs"]]
    out, total = [], 0
    for ident in ids:
        md = get_json(f"https://archive.org/metadata/{ident}")
        lic = md["metadata"].get("licenseurl", "")
        if "publicdomain" not in lic:
            continue
        files = [f for f in md["files"] if f.get("format") in ("h.264", "512Kb MPEG4", "MPEG4", "Ogg Video")
                 and int(f.get("size", 0)) <= 150 * MiB]
        size = sum(int(f["size"]) for f in files)
        if not files or total + size > 420 * MiB:
            continue
        title = str(md["metadata"].get("title", ident))
        for f in sorted(files, key=lambda f: f["name"]):
            out.append({"key": f"ia-{ident}-{hashlib.sha1(f['name'].encode()).hexdigest()[:8]}",
                        "url": f"https://archive.org/download/{ident}/" + urllib.parse.quote(f["name"]),
                        "name": safe_name(f["name"].split("/")[-1]), "dir": safe_name(ident), "api_size": int(f["size"]),
                        "api_md5": f.get("md5"), "api_sha1": f.get("sha1"), "format": f.get("format"),
                        "alloc": "f13-tuning-prelinger-video-medium",
                        "license": {"spdx_or_name": "Public domain (Prelinger Archives; " + lic + ")", "attribution": "Prelinger Archives",
                                    "source_page": f"https://archive.org/details/{ident}", "title": title[:120]}})
        total += size
        if total >= 300 * MiB:
            break
    log(f"  prelinger: {len(out)} files, {total} bytes")
    return out


def ia_pool(identifier, alloc, patterns, dirname, lic):
    md = get_json(f"https://archive.org/metadata/{identifier}")
    out = []
    for f in sorted(md["files"], key=lambda f: f["name"]):
        if not any(re.search(p, f["name"]) for p in patterns):
            continue
        url = f"https://archive.org/download/{identifier}/" + urllib.parse.quote(f["name"])
        out.append({"key": f"ia-{identifier}-{hashlib.sha1(f['name'].encode()).hexdigest()[:10]}", "url": url,
                    "name": safe_name(f["name"].split("/")[-1]), "dir": dirname, "api_size": int(f["size"]),
                    "api_md5": f.get("md5"), "api_sha1": f.get("sha1"), "format": f.get("format"),
                    "license": dict(lic, source_page=f"https://archive.org/details/{identifier}",
                                    licenseurl=md["metadata"].get("licenseurl", "")),
                    "alloc": alloc})
    if not out:
        raise RuntimeError(f"no files matched in {identifier}")
    log(f"  {identifier}: {len(out)} files, {sum(e['api_size'] for e in out)} bytes")
    return out


LIBRIVOX_LIC = {"spdx_or_name": "Public domain (LibriVox recording; PDM 1.0)", "attribution": "LibriVox volunteers"}


@pool("librivox")
def pool_librivox():
    """F13 audiobooks from LibriVox (Internet Archive is LibriVox's distribution host); a different
    book (and reader set) per split."""
    out = []
    out += ia_pool("1900lastpresident_1904_librivox", "f13-tuning-librivox-audiobook-medium",
                   [r"_128kb\.mp3$", r"\.ogg$", r"\.m4b$"], "1900 or The Last President", LIBRIVOX_LIC)
    out += ia_pool("3sfstoriesbywilliamtenn_1910_librivox", "f13-validation-librivox-audiobook-medium",
                   [r"_tenn\.mp3$", r"\.ogg$"], "3 SF Stories by William Tenn", LIBRIVOX_LIC)
    out += ia_pool("4sfstoriesbymackreynolds_2105_librivox", "f13-heldout-librivox-audiobook-medium",
                   [r"_128kb\.mp3$", r"\.ogg$", r"\.m4b$"], "4 SF Stories by Mack Reynolds", LIBRIVOX_LIC)
    return out


@pool("musopen-flac")
def pool_musopen():
    """F13 FLAC: Musopen public-domain recordings (Internet Archive).  Tuning: Kickstarter orchestral DVD
    compilation (PDM); held-out: Chopin complete works piano recordings (CC0)."""
    out = []
    out += ia_pool("musopen-dvd-in-lossless-flac-format", "f13-tuning-musopen-flac-medium",
                   [r"^Borodin - .*\.flac$", r"^Brahms - Symphony No 1.*(II\. Andante|III\. Un poco).*\.flac$"],
                   "Musopen", {"spdx_or_name": "Public domain (PDM 1.0; Musopen Kickstarter recordings)", "attribution": "Musopen / Czech National Symphony Orchestra"})
    out += ia_pool("musopen-chopin-complete-works-flac", "f13-heldout-musopen-chopin-flac-medium",
                   [r"^Albumleaf in E major\.flac$", r"^Andantino 'Spring', B\. 117\.flac$", r"^Berceuse in D-flat major, Op\. 57 \.flac$",
                    r"^Barcarolle Op\. 60\.flac$", r"^Bolero, Op\. 19\.flac$"],
                   "Chopin", {"spdx_or_name": "CC0-1.0", "attribution": "Musopen (Chopin complete works, various pianists)"})
    return out


BLENDER = {
    "f13-tuning-bbb-video-medium": [
        "https://download.blender.org/peach/bigbuckbunny_movies/BigBuckBunny_320x180.mp4.zip",
        "https://download.blender.org/peach/bigbuckbunny_movies/big_buck_bunny_480p_stereo.ogg.zip",
        "https://download.blender.org/peach/bigbuckbunny_movies/big_buck_bunny_480p_stereo.avi.zip",
    ],
    "f13-tuning-bbb-video-large": [
        "https://download.blender.org/peach/bigbuckbunny_movies/big_buck_bunny_720p_h264.mov.zip",
        "https://download.blender.org/peach/bigbuckbunny_movies/BigBuckBunny_640x360.m4v.zip",
        "https://download.blender.org/demo/movies/BBB/bbb_sunflower_1080p_30fps_normal.mp4.zip",
        "https://upload.wikimedia.org/wikipedia/commons/transcoded/c/c0/Big_Buck_Bunny_4K.webm/Big_Buck_Bunny_4K.webm.480p.vp9.webm",
    ],
    "f13-validation-sintel-media-medium": [
        "https://download.blender.org/durian/trailer/sintel_trailer-480p.mp4",
        "https://download.blender.org/durian/trailer/sintel_trailer-720p.mp4",
        "https://download.blender.org/durian/trailer/sintel_trailer-1080p.mp4",
        "https://download.blender.org/durian/trailer/sintel_trailer-480p.ogv",
        "https://download.blender.org/durian/trailer/sintel_trailer-720p.ogv",
        "https://download.blender.org/durian/trailer/Sintel_Trailer.480p.DivX_Plus_HD.mkv",
        "https://download.blender.org/durian/trailer/Sintel_Trailer.720p.DivX_Plus_HD.mkv",
        "https://download.blender.org/durian/movies/sintel-m%2Be-st.flac",
    ],
    "f13-heldout-tos-media-large": [
        "https://download.blender.org/demo/movies/ToS/tears_of_steel_1080p.webm.zip",
        "https://download.blender.org/demo/movies/ToS/TOS_DVDSTEREOMIX.aif",
    ],
    "f13-heldout-elephants-dream-media-medium": [
        "https://download.blender.org/ED/elephantsdream-480-h264-st-aac.mov",
        "https://download.blender.org/ED/elephantsdream-720-h264-st-aac.mov",
        "https://download.blender.org/ED/ED-ME-5.1-DVD.ac3",
    ],
}
BLENDER_LIC = {
    "f13-tuning-bbb-video-medium": ("CC-BY-3.0", "(c) copyright 2008, Blender Foundation / www.bigbuckbunny.org"),
    "f13-tuning-bbb-video-large": ("CC-BY-3.0", "(c) copyright 2008, Blender Foundation / www.bigbuckbunny.org"),
    "f13-validation-sintel-media-medium": ("CC-BY-3.0", "(c) copyright Blender Foundation | durian.blender.org"),
    "f13-heldout-tos-media-large": ("CC-BY-3.0", "(CC) Blender Foundation | mango.blender.org"),
    "f13-heldout-elephants-dream-media-medium": ("CC-BY-2.5", "(c) copyright 2006, Blender Foundation / Netherlands Media Art Institute / www.elephantsdream.org"),
}


@pool("blender")
def pool_blender():
    """F13 Blender Foundation open movies (download.blender.org; one production per split)."""
    out = []
    for iid, urls in BLENDER.items():
        for url in urls:
            st, size, _, _ = head(url)
            if st != 200 or not size:
                raise RuntimeError(f"blender {url}: HTTP {st}")
            name = urllib.parse.unquote(url.rsplit("/", 1)[1])
            spdx, attr = BLENDER_LIC[iid]
            out.append({"key": "blender-" + hashlib.sha1(url.encode()).hexdigest()[:12], "url": url, "name": name,
                        "api_size": size, "alloc": iid,
                        "license": {"spdx_or_name": spdx, "attribution": attr, "source_page": url.rsplit("/", 1)[0] + "/"}})
    return out


@pool("nasa-ntrs-pdf")
def pool_ntrs():
    """F13 tuning PDFs: NASA Technical Reports Server documents whose copyright determination is
    GOV_PUBLIC_USE_PERMITTED (single PDF per record)."""
    out, seen = [], set()
    for sti in ("Technical Memorandum (TM)", "Technical Publication (TP)", "Conference Paper (CP)", "Contractor Report (CR)"):
        body = json.dumps({"page": {"size": 100, "from": 0}, "published": {"gte": "2019-01-01", "lte": "2024-12-31"},
                           "distribution": "PUBLIC", "stiTypeDetails": sti}).encode()
        d = get_json("https://ntrs.nasa.gov/api/citations/search", data=body, headers={"Content-Type": "application/json"})
        for r in d.get("results", []):
            cp = r.get("copyright") or {}
            dls = [x for x in r.get("downloads", []) if (x.get("links") or {}).get("pdf")]
            if cp.get("determinationType") != "GOV_PUBLIC_USE_PERMITTED" or len(dls) != 1 or r["id"] in seen:
                continue
            seen.add(r["id"])
            out.append((sti, r, dls[0]))
        time.sleep(0.5)
    random.Random(SELECTION_SEED + 4).shuffle(out)
    ents = []
    for sti, r, dl in round_robin(out, lambda t: t[0])[:120]:
        url = "https://ntrs.nasa.gov" + dl["links"]["pdf"]
        st, size, _, _ = head(url)
        if st != 200 or not size or size > 3 * MiB:
            continue
        ents.append({"key": f"ntrs-{r['id']}", "url": url, "name": f"{r['id']}.pdf", "dir": sti.split(" (")[0],
                     "api_size": size,
                     "license": {"spdx_or_name": "US Government work (NTRS: GOV_PUBLIC_USE_PERMITTED)", "attribution": "NASA",
                                 "source_page": f"https://ntrs.nasa.gov/citations/{r['id']}"}})
    return allocate(ents, [("f13-tuning-nasa-ntrs-pdf-small", 15 * MiB, 3 * MiB, 8 * MiB)])


@pool("federal-register-pdf")
def pool_fedreg():
    """F13 validation PDFs: Federal Register documents (govinfo.gov PDFs; US Government works)."""
    ents = []
    for day in ("2025-03-03", "2025-06-02", "2025-09-02"):
        q = urllib.parse.urlencode([("per_page", "100"), ("conditions[publication_date][is]", day), ("fields[]", "pdf_url"),
                                    ("fields[]", "type"), ("fields[]", "document_number"), ("fields[]", "page_length"),
                                    ("order", "document_number")])
        d = get_json("https://www.federalregister.gov/api/v1/documents.json?" + q)
        for r in d.get("results", []):
            if r.get("pdf_url"):
                ents.append({"key": f"fr-{r['document_number']}", "url": r["pdf_url"], "name": f"{r['document_number']}.pdf",
                             "dir": f"{day}/{r['type'].replace(' ', '-')}", "type": r["type"], "pages": r.get("page_length"),
                             "license": {"spdx_or_name": "US Government work (Federal Register), public domain in the US",
                                         "attribution": "Office of the Federal Register, NARA",
                                         "source_page": f"https://www.federalregister.gov/d/{r['document_number']}"}})
        time.sleep(0.5)
    random.Random(SELECTION_SEED + 5).shuffle(ents)
    ordered = round_robin(ents, lambda e: e["type"])[:90]
    for e in ordered:
        st, size, _, _ = head(e["url"])
        e["api_size"] = size if st == 200 else None
        time.sleep(0.2)
    return allocate(ordered, [("f13-validation-federal-register-pdf-small", 15 * MiB, 3 * MiB, 8 * MiB)])


@pool("usgs-pdf")
def pool_usgs():
    """F13 held-out PDFs: USGS Publications Warehouse numbered-series reports (USGS-authored, public domain)."""
    ents = []
    for year in (2020, 2021, 2022, 2023):
        q = urllib.parse.urlencode({"page_size": 100, "year": year, "subtypeName": "USGS Numbered Series", "page_number": 1})
        d = get_json("https://pubs.usgs.gov/pubs-services/publication/?" + q)
        for r in d.get("records", []):
            docs = [l for l in r.get("links", []) if (l.get("type") or {}).get("text") == "Document"
                    and str(l.get("url", "")).startswith("https://pubs.usgs.gov/") and str(l["url"]).lower().endswith(".pdf")]
            if len(docs) != 1:
                continue
            ents.append({"key": f"usgs-{r['indexId']}", "url": docs[0]["url"], "name": safe_name(docs[0]["url"].rsplit("/", 1)[1]),
                         "dir": f"{(r.get('seriesTitle') or {}).get('text', 'series')}".replace(" ", "-"),
                         "series": (r.get("seriesTitle") or {}).get("text", ""),
                         "license": {"spdx_or_name": "US Government work (USGS), public domain", "attribution": "U.S. Geological Survey",
                                     "source_page": f"https://pubs.usgs.gov/publication/{r['indexId']}"}})
        time.sleep(0.5)
    random.Random(SELECTION_SEED + 6).shuffle(ents)
    ordered = round_robin(ents, lambda e: e["series"])[:80]
    for e in ordered:
        st, size, _, _ = head(e["url"])
        e["api_size"] = size if st == 200 else None
        time.sleep(0.2)
    return allocate(ordered, [("f13-heldout-usgs-pdf-small", 15 * MiB, 3 * MiB, 8 * MiB)])


# ---------------------------------------------------------------------------
# stages


def sel_path(name):
    return SEL_DIR / f"{name}.json"


def load_sel(name):
    return json.loads(sel_path(name).read_text(encoding="utf-8"))


def save_sel(name, doc):
    cl.write_json_atomic(sel_path(name), doc)


def stage_select(pools, refresh):
    for name in pools:
        if sel_path(name).exists() and name not in refresh:
            log(f"[select] {name}: cached selection kept")
            continue
        log(f"[select] {name}: querying")
        f = POOLS[name]
        entries = f()
        keys = [e["key"] for e in entries]
        if len(keys) != len(set(keys)):
            raise RuntimeError(f"{name}: duplicate keys")
        save_sel(name, {"pool": name, "rules": (f.__doc__ or "").strip(), "selection_seed": SELECTION_SEED,
                        "selected_utc": cl.utc_now(), "entries": entries})
        log(f"[select] {name}: {len(entries)} entries")


class _Args:
    offline = False
    reverify_cache = False


class _Ctx:
    def __init__(self):
        self.layout = cl.Layout()
        self.args = _Args()
        self.iid = "g5-media-select"


def verify_api_digests(e, path):
    if e.get("api_sha1") or e.get("api_md5"):
        h1, h5 = hashlib.sha1(), hashlib.md5()
        with open(path, "rb") as f:
            for b in iter(lambda: f.read(MiB), b""):
                h1.update(b)
                h5.update(b)
        if e.get("api_sha1") and h1.hexdigest() != e["api_sha1"]:
            raise cl.HashMismatch(f"HASH MISMATCH (sha1) {e['url']}: {h1.hexdigest()} != API {e['api_sha1']}")
        if e.get("api_md5") and h5.hexdigest() != e["api_md5"]:
            raise cl.HashMismatch(f"HASH MISMATCH (md5) {e['url']}: {h5.hexdigest()} != API {e['api_md5']}")


def stage_fetch(pools, jobs):
    ctx = _Ctx()
    for name in pools:
        doc = load_sel(name)
        todo = doc["entries"]
        host_sem = {"upload.wikimedia.org": threading.Semaphore(1)}

        def one(e):
            host = urllib.parse.urlparse(e["url"]).hostname
            sem = host_sem.get(host)
            if sem:
                sem.acquire()
            try:
                for attempt in range(1, 41):
                    try:
                        info = provision.fetch_url(ctx, e["url"], e.get("sha256"), e.get("size"))
                        break
                    except cl.HashMismatch:
                        raise
                    except cl.CorpusError as err:
                        if attempt == 40:
                            raise
                        wait = 620 if "429" in str(err) else 60
                        log(f"  retry {attempt} for {e['url'][-80:]} after error ...{str(err)[-60:]}; sleeping {wait}s")
                        time.sleep(wait)
                if sem and not info.get("cached"):
                    time.sleep(4)
            finally:
                if sem:
                    sem.release()
            if e.get("api_size") is not None and info["size"] != e["api_size"]:
                raise cl.HashMismatch(f"SIZE MISMATCH {e['url']}: {info['size']} != API {e['api_size']}")
            if not e.get("sha256"):
                verify_api_digests(e, info["path"])
            return e, info

        n_new = 0
        with ThreadPoolExecutor(max_workers=jobs) as ex:
            futs = [ex.submit(one, e) for e in todo]
            for i, fut in enumerate(as_completed(futs), 1):
                e, info = fut.result()
                if not e.get("sha256"):
                    e["sha256"], e["size"] = info["sha256"], info["size"]
                    e["retrieved_utc"] = info.get("retrieved_utc")
                    e["final_url"] = info.get("final_url")
                    n_new += 1
                    if n_new % 20 == 0:
                        save_sel(name, doc)
                if i % 25 == 0:
                    log(f"[fetch] {name}: {i}/{len(todo)}")
        save_sel(name, doc)
        log(f"[fetch] {name}: done ({len(todo)} entries, {n_new} newly recorded, "
            f"{sum(e['size'] for e in todo)} bytes)")


# ---------------------------------------------------------------------------
# emit


def emit(partial=False):
    import g5_items  # noqa: E402  (item table lives next to this script)
    sel = {p.stem: json.loads(p.read_text(encoding="utf-8")) for p in sorted(SEL_DIR.glob("*.json"))}
    items = g5_items.build_items(sel, partial=partial)
    doc = {"schema": cl.SOURCES_SCHEMA, "notes": g5_items.NOTES, "items": items}
    cl.write_json_atomic(SOURCES_OUT, doc)
    log(f"[emit] wrote {cl.repo_rel(SOURCES_OUT)}: {len(items)} items")


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("stage", choices=("select", "fetch", "emit", "all"))
    ap.add_argument("--pool", action="append", help="restrict to pool(s)")
    ap.add_argument("--refresh", action="append", default=[], help="re-query pool(s) even if a selection exists")
    ap.add_argument("--jobs", type=int, default=4)
    ap.add_argument("--partial", action="store_true", help="emit: skip items whose pools are not fully fetched")
    args = ap.parse_args()
    cl.require_linux()
    os.umask(0o022)
    sys.path.insert(0, str(HERE))
    pools = args.pool or list(POOLS)
    unknown = set(pools) - set(POOLS)
    if unknown:
        raise SystemExit(f"unknown pools {sorted(unknown)}; known: {sorted(POOLS)}")
    SEL_DIR.mkdir(parents=True, exist_ok=True)
    if args.stage in ("select", "all"):
        stage_select(pools, set(args.refresh))
    if args.stage in ("fetch", "all"):
        stage_fetch([p for p in pools if sel_path(p).exists()], args.jobs)
    if args.stage in ("emit", "all"):
        emit(partial=args.partial)


if __name__ == "__main__":
    main()
