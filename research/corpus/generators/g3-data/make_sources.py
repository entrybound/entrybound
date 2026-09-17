#!/usr/bin/env python3
"""Emit research/corpus/sources/g3-data.json (F05 logs/text, F06 structured text,
F07 numeric arrays, F08 databases) from the declarative tables below.

    python make_sources.py [--tofu-only] [--check-only]

By default every "TOFU" input whose first retrieval is recorded in
research/corpus/pins/<item_id>.json (same URL) is declared with that SHA-256 and size, so the
committed definitions carry explicit hashes and a fresh machine fails loudly on drift.
--tofu-only   keep all inputs as "TOFU" (bootstrap before the first provisioning run).
--check-only  print the JSON to stdout instead of writing the file.

Output is deterministic (stable key order, indent=2, trailing newline).  Stdlib only; runs
under Windows or WSL Python.
"""

import argparse
import json
import os
import re
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[3]
SOURCES = REPO / "research" / "corpus" / "sources" / "g3-data.json"
PINS = REPO / "research" / "corpus" / "pins"
GEN = "research/corpus/generators/g3-data"

# ---------------------------------------------------------------------------------------
# licenses


def lic(name, redistributable, attribution, notes=""):
    return {"spdx_or_name": name, "redistributable": redistributable, "attribution": attribution, "notes": notes}


LOGHUB = lic("CC-BY-4.0 per Zenodo record 8196385 metadata; upstream README: 'freely available for research or academic work'",
             False, "Loghub, LogPAI (J. Zhu, S. He, P. He, J. Liu, M. R. Lyu, ISSRE 2023; Z. Jiang et al., ISSTA 2024), "
             "https://github.com/logpai/loghub, doi:10.5281/zenodo.8196385",
             "Conflicting signals: Zenodo metadata CC-BY-4.0 vs README research/academic-use wording, and several logs are "
             "production data released by third parties. Treated as not redistributable: commit hashes only.")
SILESIA = lic("Silesia compression corpus terms (freely available for compression research; no explicit redistribution license)",
              False, "Sebastian Deorowicz, Silesian University of Technology, https://sun.aei.polsl.pl/~sdeor/index.php?page=silesia",
              "Files originate from various third parties; not redistributed, hashes only.")
SDRBENCH = lic("SDRBench data terms (cite SDRBench and the per-dataset source; no explicit redistribution license)", False,
               "SDRBench, https://sdrbench.github.io (K. Zhao, S. Di, et al., IWBDR 2020)",
               "Not redistributed; hashes only.")
US_PD = "Public domain (US Government work, 17 U.S.C. 105)"
GHARCHIVE = lic("No explicit data license (GH Archive dataset of GitHub public events; GitHub Terms of Service and third-party rights apply)",
                False, "GH Archive, https://www.gharchive.org (Ilya Grigorik); GitHub, Inc. event data",
                "gharchive.org site content is CC-BY-4.0 but the upstream README states the event archives may be subject "
                "to third-party rights. Treated as not redistributable.")
GUTENBERG = lic("Public domain in the USA (Project Gutenberg eBooks; Project Gutenberg License governs the trademark and header)", True,
                "Project Gutenberg, https://www.gutenberg.org (retrieved from the PGLAF mirror gutenberg.pglaf.org)",
                "Files are redistributed unmodified including the Project Gutenberg header/footer, as the PG license allows.")

# ---------------------------------------------------------------------------------------
# helpers


def dl(name, url, filename=None, notes=None, upstream_digest=None):
    d = {"name": name, "url": url, "sha256": "TOFU"}
    if filename:
        d["filename"] = filename
    if notes:
        d["notes"] = notes
    if upstream_digest:
        d["upstream_digest"] = upstream_digest
    return d


def wikimedia_dumpstatus(url, algo="sha1"):
    """upstream_digest spec for a file under a dumps.wikimedia.org dated dump
    directory: cross-checks the cached blob against the digest the dump directory's
    own dumpstatus.json publishes (independent of, and not shortcut by, our own
    content-addressed cache key -- see task_bb8ca4e8's cache-integrity follow-up in
    PROGRESS.md: the self-consistency check alone cannot catch a blob that was
    corrupted before we ever computed its sha256, e.g. the all-zero pageviews blob).
    Not available for the flat https://dumps.wikimedia.org/other/... trees (pageviews
    etc.), which publish no per-file manifest, only a directory-listing size."""
    manifest_url = url.rsplit("/", 1)[0] + "/dumpstatus.json"
    return {"format": "wikimedia-dumpstatus", "manifest_url": manifest_url, "algo": algo}


def item(item_id, family, split, scale, kind, rog, recipe, license_, group, description, notes=None, tags=None):
    d = {"item_id": item_id, "family": family, "split": split, "scale": scale, "kind": kind,
         "real_or_generated": rog, "recipe": recipe, "license": license_, "independence_group": group,
         "description": description}
    if notes:
        d["notes"] = notes
    if tags:
        d["tags"] = tags
    return d


def gen(script, seed, params, interpreter="python"):
    g = {"script": f"{GEN}/{script}", "seed": seed, "params": params}
    if interpreter != "python":
        g["interpreter"] = interpreter
    return g


ZEN = "https://zenodo.org/records/8196385/files/"
SIL = "https://sun.aei.polsl.pl/~sdeor/corpus/"
SDR = "https://g-d0cd3f.fd635.8443.data.globus.org/raw-data/"
GENERATED_LIC = lic("CC0-1.0 (generated by Entrybound research scripts)", True, "Entrybound research corpus generators",
                    "Synthetic data; statistics reflect the generator's model, not any real system.")

ITEMS = []

# =======================================================================================
# F05 logs/text
# =======================================================================================
ITEMS += [
    item("f05-tuning-loghub-hdfs-v1", "F05", "tuning", "large", "download", "real",
         {"inputs": [dl("hdfs-v1", ZEN + "HDFS_v1.zip")], "steps": [{"op": "extract", "input": "hdfs-v1", "format": "zip"}]},
         LOGHUB, "loghub-hdfs-v1",
         "Loghub HDFS_v1: Hadoop Distributed File System logs from a 200+ node Amazon EC2 cluster (Xu et al., SOSP 2009) "
         "with block-level anomaly labels, extracted from the Loghub Zenodo archive.",
         tags=["public-benchmark", "log-parsing-benchmark"]),
    item("f05-tuning-loghub-openstack", "F05", "tuning", "medium", "download", "real",
         {"inputs": [dl("openstack", ZEN + "OpenStack.tar.gz")], "steps": [{"op": "extract", "input": "openstack"}]},
         LOGHUB, "loghub-openstack",
         "Loghub OpenStack: OpenStack cloud service logs (nova API/compute) with anomaly labels, extracted from the Loghub Zenodo archive.",
         tags=["public-benchmark", "log-parsing-benchmark"]),
    item("f05-tuning-enwik8", "F05", "tuning", "medium", "download", "real",
         {"inputs": [dl("enwik8", "https://mattmahoney.net/dc/enwik8.zip")], "steps": [{"op": "extract", "input": "enwik8", "format": "zip"}]},
         lic("CC-BY-SA-3.0 AND GFDL-1.2 (English Wikipedia text, 2006 dump)", True,
             "Wikipedia contributors; enwik8 prepared by Matt Mahoney for the Hutter Prize / Large Text Compression Benchmark",
             "Redistribution requires attribution and share-alike."),
         "hutter-enwik8",
         "enwik8: first 10^8 bytes of the English Wikipedia XML dump of 2006-03-03 (Hutter Prize text).",
         tags=["public-benchmark"]),
    item("f05-tuning-gutenberg-books", "F05", "tuning", "small", "download", "real",
         {"inputs": [dl("pg145", "https://gutenberg.pglaf.org/cache/epub/145/pg145.txt"),
                     dl("pg2701", "https://gutenberg.pglaf.org/cache/epub/2701/pg2701.txt"),
                     dl("pg17489", "https://gutenberg.pglaf.org/cache/epub/17489/pg17489.txt"),
                     dl("pg2229", "https://gutenberg.pglaf.org/cache/epub/2229/pg2229.txt"),
                     dl("pg23962", "https://gutenberg.pglaf.org/cache/epub/23962/pg23962.txt")]},
         GUTENBERG, "gutenberg-pd-books",
         "Five public-domain Project Gutenberg plain-text eBooks in four languages/scripts: Middlemarch (EN, #145), "
         "Moby Dick (EN, #2701), Les Miserables tome I (FR, #17489), Faust I (DE, #2229), and a Chinese classical novel (#23962).",
         notes="Downloaded from the official PGLAF mirror rather than www.gutenberg.org, whose robot policy disallows automated access."),
    item("f05-tuning-gen-logs-a-small", "F05", "tuning", "small", "generate", "generated",
         {"generator": gen("gen_logs_ab.py", 5101, {"hosts": 12, "start_epoch": 1767225600, "files": [
             {"path": "var/log/syslog", "profile": "syslog", "bytes": 4000000, "rate": 3.0},
             {"path": "var/log/syslog.1", "profile": "syslog", "bytes": 3000000, "rate": 3.0},
             {"path": "srv/checkout/app.jsonl", "profile": "jsonl", "bytes": 5000000, "rate": 8.0}]})},
         GENERATED_LIC, "ebrc-gen-logs-a",
         "Generated log tree (profile A, small): RFC 3164 syslog with rotation plus JSON-lines service logs.",
         tags=["generated"]),
    item("f05-tuning-gen-logs-a-medium", "F05", "tuning", "medium", "generate", "generated",
         {"generator": gen("gen_logs_ab.py", 5102, {"hosts": 40, "start_epoch": 1767312000, "files": [
             {"path": "var/log/syslog", "profile": "syslog", "bytes": 40000000, "rate": 20.0},
             {"path": "var/log/syslog.1", "profile": "syslog", "bytes": 30000000, "rate": 20.0},
             {"path": "srv/gateway/app.jsonl", "profile": "jsonl", "bytes": 70000000, "rate": 60.0},
             {"path": "srv/payments/app.jsonl", "profile": "jsonl", "bytes": 40000000, "rate": 25.0},
             {"path": "srv/orders/application.log", "profile": "java", "bytes": 50000000, "rate": 30.0}]})},
         GENERATED_LIC, "ebrc-gen-logs-a",
         "Generated log tree (profile A, medium): syslog, JSON-lines service logs and log4j-style Java logs with stack traces.",
         tags=["generated"]),
    # validation
    item("f05-validation-loghub-ssh", "F05", "validation", "medium", "download", "real",
         {"inputs": [dl("ssh", ZEN + "SSH.tar.gz")], "steps": [{"op": "extract", "input": "ssh"}]},
         LOGHUB, "loghub-openssh",
         "Loghub SSH: OpenSSH server logs collected from a lab server exposed to the Internet, extracted from the Loghub Zenodo archive.",
         tags=["public-benchmark", "log-parsing-benchmark"]),
    item("f05-validation-rfc-texts", "F05", "validation", "small", "download", "real",
         {"inputs": [dl(f"rfc{n}", f"https://www.rfc-editor.org/rfc/rfc{n}.txt") for n in
                     (791, 793, 2616, 3986, 4180, 5321, 5322, 6749, 7231, 7540, 8259, 8446, 9000, 9001, 9002, 9110, 9111, 9112, 9113, 9114)]},
         lic("IETF Trust Legal Provisions for RFCs (unmodified RFCs may be copied and distributed)", True,
             "IETF Trust and the persons identified as the document authors; RFC Editor, https://www.rfc-editor.org",
             "Redistributed unmodified."),
         "ietf-rfc-texts",
         "Twenty IETF RFC plain-text documents (IP, TCP, HTTP, URI, CSV, SMTP, OAuth, JSON, TLS 1.3, QUIC) in RFC plain-text layout.",
         notes="The RFC Editor no longer publishes bulk tar archives over https (rsync only), so individual RFC text files are listed."),
    item("f05-validation-gen-logs-b", "F05", "validation", "medium", "generate", "generated",
         {"generator": gen("gen_logs_ab.py", 5201, {"hosts": 64, "start_epoch": 1769904000, "files": [
             {"path": "k8s/ingress/access.log", "profile": "access", "bytes": 60000000, "rate": 80.0},
             {"path": "k8s/api/api.logfmt", "profile": "logfmt", "bytes": 50000000, "rate": 40.0},
             {"path": "k8s/billing/billing.log", "profile": "java", "bytes": 40000000, "rate": 15.0}]})},
         GENERATED_LIC, "ebrc-gen-logs-b",
         "Generated log tree (profile B): combined-format HTTP access logs, logfmt key=value service logs and Java logs.",
         notes="Same generator script as ebrc-gen-logs-a but different profiles, seeds and parameters; not independent in "
               "distribution from the tuning generated items (methodology section 3 rule 5).",
         tags=["generated"]),
    # heldout
    item("f05-heldout-secrepo-self-weblogs-2017-01", "F05", "heldout", "medium", "download", "real",
         {"inputs": [dl(f"d{d:02d}", f"https://www.secrepo.com/self.logs/access.log.2017-01-{d:02d}.gz") for d in range(1, 32)],
          "steps": [{"op": "decompress", "input": f"d{d:02d}", "dest": f"access.log.2017-01-{d:02d}", "format": "gz"} for d in range(1, 32)]},
         lic("CC-BY-4.0", True, "Security Repo (secrepo.com) by Mike Sconzo", "Web server logs of secrepo.com itself as published there."),
         "secrepo-self-weblogs",
         "Web server access logs of secrepo.com for January 2017 (31 daily files), published by Security Repo; decompressed from .gz.",
         notes="Held-out item: description is provenance only."),
    item("f05-heldout-maccdc2012-zeek-logs", "F05", "heldout", "medium", "download", "real",
         {"inputs": [dl(n, f"https://www.secrepo.com/maccdc2012/{n}.log.gz") for n in
                     ("dns", "ssl", "weird", "ssh", "ftp", "dhcp", "notice", "smtp", "tunnel", "signatures")],
          "steps": [{"op": "decompress", "input": n, "dest": f"{n}.log", "format": "gz"} for n in
                    ("dns", "ssl", "weird", "ssh", "ftp", "dhcp", "notice", "smtp", "tunnel", "signatures")]},
         lic("CC-BY-4.0 as published on secrepo.com; capture from the 2012 Mid-Atlantic Collegiate Cyber Defense Competition", False,
             "Security Repo (secrepo.com) by Mike Sconzo; MACCDC 2012 / National CyberWatch Center",
             "Upstream capture rights are not documented beyond the secrepo listing; treated as not redistributable (hashes only)."),
         "maccdc2012-zeek-logs",
         "Bro/Zeek network monitor logs (dns, ssl, weird, ssh, ftp, dhcp, notice, smtp, tunnel, signatures) produced from the "
         "MACCDC 2012 packet captures, as published by Security Repo; decompressed from .gz.",
         notes="Held-out item: description is provenance only. conn/http/files logs omitted to stay within the medium tier."),
    item("f05-heldout-congressional-record-2024-01", "F05", "heldout", "small", "download", "real",
         {"inputs": [dl(f"crec{d}", f"https://www.govinfo.gov/content/pkg/CREC-2024-01-{d}/zip/CREC-2024-01-{d}.zip") for d in ("10", "11", "16", "17")],
          "steps": [{"op": "extract", "input": f"crec{d}", "format": "zip"} for d in ("10", "11", "16", "17")]
          + [{"op": "remove", "paths": [f"CREC-2024-01-{d}/pdf" for d in ("10", "11", "16", "17")]}]},
         lic(US_PD, True, "U.S. Government Publishing Office, govinfo.gov, Congressional Record",
             "Congressional Record daily edition; the PDF renditions are removed, the HTML text renditions are kept."),
         "govinfo-congressional-record",
         "U.S. Congressional Record daily editions of 2024-01-10, -11, -16 and -17 from GovInfo package zips, HTML text renditions only.",
         notes="Held-out item: description is provenance only."),
    item("f05-heldout-gen-logs-c", "F05", "heldout", "medium", "generate", "generated",
         {"generator": gen("gen_logs_c.py", 5301, {"start_epoch": 1772323200, "nodes": 48, "jobs": 400000,
                                                   "metrics_every": 30, "max_bytes": 160000000})},
         GENERATED_LIC, "ebrc-gen-logs-c",
         "Generated batch-cluster log tree from a discrete-event simulation (generator C, distinct from generators A/B).",
         notes="Held-out generated item uses a different generator script and model than the tuning/validation generated logs.",
         tags=["generated"]),
]

# =======================================================================================
# F06 JSON/XML/CSV/structured text
# =======================================================================================
GHA = "https://data.gharchive.org/"
UCD_FILES = ['UnicodeData.txt', 'NamesList.txt', 'Scripts.txt', 'ScriptExtensions.txt', 'LineBreak.txt', 'EastAsianWidth.txt', 'DerivedAge.txt', 'PropList.txt', 'Blocks.txt', 'CaseFolding.txt', 'SpecialCasing.txt', 'extracted/DerivedGeneralCategory.txt']
ITEMS += [
    item("f06-tuning-gharchive-2015-01-01-h15-h16", "F06", "tuning", "medium", "download", "real",
         {"inputs": [dl("h15", GHA + "2015-01-01-15.json.gz"), dl("h16", GHA + "2015-01-01-16.json.gz")],
          "steps": [{"op": "decompress", "input": "h15", "dest": "2015-01-01-15.json", "format": "gz"},
                    {"op": "decompress", "input": "h16", "dest": "2015-01-01-16.json", "format": "gz"}]},
         GHARCHIVE, "gharchive-github-events",
         "GH Archive hourly GitHub public event streams (newline-delimited JSON) for 2015-01-01 15:00 and 16:00 UTC, decompressed."),
    item("f06-tuning-gharchive-2024-01-15-h15", "F06", "tuning", "large", "download", "real",
         {"inputs": [dl("h15", GHA + "2024-01-15-15.json.gz")],
          "steps": [{"op": "decompress", "input": "h15", "dest": "2024-01-15-15.json", "format": "gz"}]},
         GHARCHIVE, "gharchive-github-events",
         "GH Archive hourly GitHub public event stream (newline-delimited JSON) for 2024-01-15 15:00 UTC, decompressed."),
    item("f06-tuning-census-popest-2023", "F06", "tuning", "small", "download", "real",
         {"inputs": [dl("county", "https://www2.census.gov/programs-surveys/popest/datasets/2020-2023/counties/totals/co-est2023-alldata.csv"),
                     dl("state", "https://www2.census.gov/programs-surveys/popest/datasets/2020-2023/state/totals/NST-EST2023-ALLDATA.csv")]},
         lic(US_PD, True, "U.S. Census Bureau, Population Estimates Program (Vintage 2023)"),
         "census-popest-v2023",
         "U.S. Census Bureau Vintage 2023 population estimates CSVs: county totals and state totals with components of change.",
         notes="Substitute for the BLS hint: Census CSVs are directly downloadable over https without a registered User-Agent."),
    item("f06-tuning-wikimedia-tnwiki-20260901", "F06", "tuning", "medium", "download", "real",
         {"inputs": [dl("dump", "https://dumps.wikimedia.org/tnwiki/20260901/tnwiki-20260901-pages-meta-current.xml.bz2",
                       upstream_digest=wikimedia_dumpstatus(
                           "https://dumps.wikimedia.org/tnwiki/20260901/tnwiki-20260901-pages-meta-current.xml.bz2"))],
          "steps": [{"op": "decompress", "input": "dump", "dest": "tnwiki-20260901-pages-meta-current.xml", "format": "bz2"}]},
         lic("CC-BY-SA-4.0 AND GFDL-1.3 (Wikipedia text); dump metadata CC0", True,
             "Setswana Wikipedia contributors; Wikimedia Foundation dumps, https://dumps.wikimedia.org",
             "Share-alike; the dated dump directory is removed upstream after some months (cache keeps the bytes)."),
         "wikimedia-tnwiki",
         "Wikimedia XML export of the Setswana Wikipedia (tnwiki), pages-meta-current dump of 2026-09-01, decompressed from bz2."),
    item("f06-tuning-gen-structured-a-small", "F06", "tuning", "small", "generate", "generated",
         {"generator": gen("gen_structured_ab.py", 6100, {"outputs": [
             {"path": "orders/orders.ndjson", "kind": "ndjson-orders", "bytes": 6000000},
             {"path": "catalog/products.json", "kind": "json-array", "bytes": 3000000},
             {"path": "telemetry/sensors.csv", "kind": "csv-sensors", "bytes": 5000000}]})},
         GENERATED_LIC, "ebrc-gen-structured-a",
         "Generated structured-text tree (profile A, small): NDJSON orders, a pretty-printed JSON array, and a wide sensor CSV.",
         tags=["generated"]),
    item("f06-tuning-gen-structured-a-medium", "F06", "tuning", "medium", "generate", "generated",
         {"generator": gen("gen_structured_ab.py", 6101, {"outputs": [
             {"path": "orders/2026-01.ndjson", "kind": "ndjson-orders", "bytes": 30000000},
             {"path": "orders/2026-02.ndjson", "kind": "ndjson-orders", "bytes": 30000000},
             {"path": "catalog/products.json", "kind": "json-array", "bytes": 25000000},
             {"path": "telemetry/sensors-a.csv", "kind": "csv-sensors", "bytes": 30000000},
             {"path": "telemetry/sensors-b.csv", "kind": "csv-sensors", "bytes": 20000000}]})},
         GENERATED_LIC, "ebrc-gen-structured-a",
         "Generated structured-text tree (profile A, medium): NDJSON orders, a pretty-printed JSON array, and wide sensor CSVs.",
         tags=["generated"]),
    # validation
    item("f06-validation-osm-andorra-20250101-xml", "F06", "validation", "medium", "download", "derived-from-real",
         {"inputs": [dl("pbf", "https://download.geofabrik.de/europe/andorra-250101.osm.pbf")],
          "steps": [{"op": "run", "script": f"{GEN}/osm_pbf_to_xml.sh", "interpreter": "bash",
                     "params": {"input": "pbf", "output": "andorra-250101.osm"}}]},
         lic("ODbL-1.0", True, "(c) OpenStreetMap contributors, https://www.openstreetmap.org/copyright; extract by Geofabrik GmbH",
             "Share-alike database license; attribution required."),
         "openstreetmap-planet",
         "OpenStreetMap XML (.osm) for Andorra as of 2025-01-01, converted losslessly from the dated Geofabrik PBF extract with osmium-tool.",
         notes="Geofabrik publishes dated extracts only as PBF; conversion to OSM XML uses osmium-tool 1.16.0 (Ubuntu 24.04 package)."),
    item("f06-validation-osm-monaco-20250101-xml", "F06", "validation", "small", "download", "derived-from-real",
         {"inputs": [dl("pbf", "https://download.geofabrik.de/europe/monaco-250101.osm.pbf")],
          "steps": [{"op": "run", "script": f"{GEN}/osm_pbf_to_xml.sh", "interpreter": "bash",
                     "params": {"input": "pbf", "output": "monaco-250101.osm"}}]},
         lic("ODbL-1.0", True, "(c) OpenStreetMap contributors, https://www.openstreetmap.org/copyright; extract by Geofabrik GmbH",
             "Share-alike database license; attribution required."),
         "openstreetmap-planet",
         "OpenStreetMap XML (.osm) for Monaco as of 2025-01-01, converted losslessly from the dated Geofabrik PBF extract with osmium-tool.",
         notes="Small-scale companion of the Andorra extract (same independence group and split)."),
    item("f06-validation-noaa-storm-events-2016", "F06", "validation", "medium", "download", "real",
         {"inputs": [dl("details", "https://www.ncei.noaa.gov/pub/data/swdi/stormevents/csvfiles/StormEvents_details-ftp_v1.0_d2016_c20260323.csv.gz"),
                     dl("fatalities", "https://www.ncei.noaa.gov/pub/data/swdi/stormevents/csvfiles/StormEvents_fatalities-ftp_v1.0_d2016_c20260323.csv.gz"),
                     dl("locations", "https://www.ncei.noaa.gov/pub/data/swdi/stormevents/csvfiles/StormEvents_locations-ftp_v1.0_d2016_c20260707.csv.gz")],
          "steps": [{"op": "decompress", "input": "details", "dest": "StormEvents_details-ftp_v1.0_d2016_c20260323.csv", "format": "gz"},
                    {"op": "decompress", "input": "fatalities", "dest": "StormEvents_fatalities-ftp_v1.0_d2016_c20260323.csv", "format": "gz"},
                    {"op": "decompress", "input": "locations", "dest": "StormEvents_locations-ftp_v1.0_d2016_c20260707.csv", "format": "gz"}]},
         lic(US_PD, True, "NOAA National Centers for Environmental Information, Storm Events Database"),
         "noaa-storm-events",
         "NOAA NCEI Storm Events Database CSV files for 2016 (details with narrative text, fatalities, locations), decompressed.",
         notes="Chosen over live-updated GHCN-Daily station CSVs because Storm Events file names carry a creation date."),
    item("f06-validation-gen-structured-b", "F06", "validation", "medium", "generate", "generated",
         {"generator": gen("gen_structured_ab.py", 6201, {"outputs": [
             {"path": "library/catalog.xml", "kind": "xml-catalog", "bytes": 25000000},
             {"path": "finance/ledger-2025.csv", "kind": "csv-ledger", "bytes": 15000000},
             {"path": "genome/annotation.tsv", "kind": "tsv-genes", "bytes": 15000000},
             {"path": "api-cache", "kind": "json-tree", "files": 3000}]})},
         GENERATED_LIC, "ebrc-gen-structured-b",
         "Generated structured-text tree (profile B): XML catalog, RFC 4180 CSV ledger with quoted multi-line fields, "
         "GFF-like TSV, and a directory of small JSON API documents.",
         notes="Same generator script as ebrc-gen-structured-a with different kinds, seed and parameters; not independent in distribution.",
         tags=["generated"]),
    # heldout
    item("f06-heldout-unicode-ucd-16-0-0-text", "F06", "heldout", "small", "download", "real",
         {"inputs": [dl(n.split("/")[-1].split(".")[0].lower(), "https://www.unicode.org/Public/16.0.0/ucd/" + n) for n in UCD_FILES]},
         lic("Unicode-3.0", True, "Unicode, Inc., Unicode Character Database 16.0.0, https://www.unicode.org/license.txt"),
         "unicode-ucd",
         "Unicode Character Database 16.0.0 semicolon-delimited property files (twelve files from the versioned ucd/ directory).",
         notes="Held-out item: description is provenance only. Versioned static URLs."),
    item("f06-heldout-federal-register-xml-2024-01", "F06", "heldout", "medium", "download", "real",
         {"inputs": [dl(f"fr{d}", f"https://www.govinfo.gov/bulkdata/FR/2024/01/FR-2024-01-{d}.xml")
                     for d in ("02", "03", "04", "05", "08", "09", "10", "11", "12", "16")]},
         lic(US_PD, True, "Office of the Federal Register / U.S. Government Publishing Office, GovInfo bulk data"),
         "govinfo-federal-register",
         "Federal Register daily issues in GPO bulk-data XML for ten publication days in January 2024.",
         notes="Held-out item: description is provenance only."),
    item("f06-heldout-nvd-cve-2016-json", "F06", "heldout", "medium", "download", "real",
         {"inputs": [dl("nvd2016", "https://nvd.nist.gov/feeds/json/cve/2.0/nvdcve-2.0-2016.json.gz")],
          "steps": [{"op": "decompress", "input": "nvd2016", "dest": "nvdcve-2.0-2016.json", "format": "gz"}]},
         lic("Public domain (NIST NVD, US Government work); CVE records subject to the CVE Program terms of use (permissive)", True,
             "NIST National Vulnerability Database; CVE Program (MITRE)",
             "The yearly feed is regenerated upstream as records change; the TOFU pin and cache fix the retrieved bytes."),
         "nist-nvd-cve",
         "NIST NVD CVE JSON 2.0 yearly data feed for CVE-2016 identifiers, decompressed.",
         notes="Held-out item: description is provenance only."),
    item("f06-heldout-bts-ontime-2024-01-csv", "F06", "heldout", "medium", "download", "real",
         {"inputs": [dl("ontime", "https://transtats.bts.gov/PREZIP/On_Time_Reporting_Carrier_On_Time_Performance_1987_present_2024_1.zip")],
          "steps": [{"op": "extract", "input": "ontime", "format": "zip"}]},
         lic(US_PD, True, "U.S. Department of Transportation, Bureau of Transportation Statistics, TranStats"),
         "bts-ontime-performance",
         "BTS Reporting Carrier On-Time Performance prezipped monthly CSV for January 2024, extracted.",
         notes="Held-out item: description is provenance only."),
]

# =======================================================================================
# F07 scientific/numeric arrays
# =======================================================================================
ITEMS += [
    item(f"f07-tuning-silesia-{name.replace('-', '')}", "F07", "tuning", "small", "download", "real",
         {"inputs": [dl(name.replace("-", ""), SIL + f"{name}.bz2")],
          "steps": [{"op": "decompress", "input": name.replace("-", ""), "dest": name, "format": "bz2"}]},
         SILESIA, "silesia-corpus", desc, tags=["public-benchmark"])
    for name, desc in (("sao", "Silesia corpus 'sao': SAO star catalog binary records (floating-point coordinates and magnitudes), decompressed from the per-file bz2."),
                       ("mr", "Silesia corpus 'mr': medical magnetic resonance image volume (16-bit samples), decompressed from the per-file bz2."),
                       ("x-ray", "Silesia corpus 'x-ray': medical X-ray image (12-bit samples), decompressed from the per-file bz2."))
]
ITEMS += [
    item("f07-tuning-sdrbench-hurricane-isabel", "F07", "tuning", "large", "download", "real",
         {"inputs": [dl("isabel", SDR + "Hurricane-ISABEL/SDRBENCH-Hurricane-ISABEL-100x500x500.tar.gz")],
          "steps": [{"op": "extract", "input": "isabel"}]},
         lic(SDRBENCH["spdx_or_name"], False, SDRBENCH["attribution"] + "; Hurricane Isabel WRF simulation courtesy of NCAR (IEEE Visualization 2004 contest)",
             SDRBENCH["notes"]),
         "sdrbench-hurricane-isabel",
         "SDRBench Hurricane Isabel: 20 single-precision 100x500x500 raw fields at time step 48 from a WRF simulation (13 variables such as "
         "pressure, temperature, wind and moisture species, plus log10 transforms of 7 of them).",
         tags=["public-benchmark"]),
    item("f07-tuning-noaa-ncep-reanalysis-air-sig995-2020", "F07", "tuning", "medium", "download", "real",
         {"inputs": [dl("air", "https://downloads.psl.noaa.gov/Datasets/ncep.reanalysis/surface/air.sig995.2020.nc")]},
         lic("Public domain / no restrictions (US Government data, NOAA PSL); acknowledgement requested", True,
             "NCEP-NCAR Reanalysis 1 data provided by the NOAA PSL, Boulder, Colorado, USA, https://psl.noaa.gov"),
         "noaa-psl-ncep-reanalysis",
         "NCEP/NCAR Reanalysis 1 daily 0.995-sigma air temperature for 2020 on a 2.5-degree global grid, NetCDF.",
         notes="Substitute for the Unidata NetCDF example files, whose former URLs now return 404."),
    item("f07-tuning-gen-numeric-a-small", "F07", "tuning", "small", "generate", "generated",
         {"generator": gen("gen_numeric_ab.py", 7100, {"arrays": [
             {"path": "fields/temperature_1024x1024.f32", "kind": "grf", "shape": [1024, 1024], "dtype": "float32", "beta": 3.0, "scale": 4.0, "offset": 288.0},
             {"path": "fields/elevation_1024x1024_int16.npy", "kind": "grf", "shape": [1024, 1024], "dtype": "int16", "beta": 3.5, "scale": 900.0,
              "offset": 600.0, "fill_fraction": 0.25, "fill_value": -32768, "container": "npy"},
             {"path": "series/adc_8ch_250k.i16", "kind": "timeseries", "shape": [8, 250000], "dtype": "int16", "scale": 2500.0, "noise": 0.05},
             {"path": "series/time_250k.npy", "kind": "timestamps", "shape": [250000], "dtype": "float64", "step": 0.0005, "container": "npy"}]})},
         GENERATED_LIC, "ebrc-gen-numeric-a",
         "Generated numeric arrays (profile A, small): smooth float32 and quantized int16 random fields, int16 ADC channels, float64 timestamps.",
         tags=["generated"]),
    item("f07-tuning-gen-numeric-a-medium", "F07", "tuning", "medium", "generate", "generated",
         {"generator": gen("gen_numeric_ab.py", 7101, {"arrays": [
             {"path": "volumes/density_256x256x256.f32", "kind": "grf", "shape": [256, 256, 256], "dtype": "float32", "beta": 3.5, "scale": 1.0, "offset": 10.0},
             {"path": "volumes/potential_192x192x192.f64", "kind": "grf", "shape": [192, 192, 192], "dtype": "float64", "beta": 2.5},
             {"path": "maps/sst_4096x4096.npy", "kind": "grf", "shape": [4096, 4096], "dtype": "float32", "beta": 3.0, "scale": 3.0, "offset": 290.0,
              "fill_fraction": 0.3, "fill_value": -9999, "container": "npy"},
             {"path": "maps/dem_4096x2048.i16", "kind": "grf", "shape": [2048, 4096], "dtype": "int16", "beta": 3.8, "scale": 1200.0, "offset": 400.0},
             {"path": "series/strain_16ch_2M.f32", "kind": "timeseries", "shape": [16, 2000000], "dtype": "float32", "noise": 0.2, "drift": 0.01},
             {"path": "particles/halo_2M.f32", "kind": "particles", "shape": [2000000, 6], "dtype": "float32"}]})},
         GENERATED_LIC, "ebrc-gen-numeric-a",
         "Generated numeric arrays (profile A, medium): float32/float64 3-D random fields, masked 2-D fields, int16 grids, "
         "float32 multichannel series and clustered particle phase-space records.",
         tags=["generated"]),
    # validation
    item("f07-validation-sdrbench-exafel", "F07", "validation", "medium", "download", "real",
         {"inputs": [dl("exafel", SDR + "EXAFEL/SDRBENCH-EXAFEL-10x32x185x388.tar.gz")],
          "steps": [{"op": "extract", "input": "exafel"}]},
         lic(SDRBENCH["spdx_or_name"], False, SDRBENCH["attribution"] + "; EXAFEL/LCLS detector data (SLAC, ECP ExaFEL)", SDRBENCH["notes"]),
         "sdrbench-exafel",
         "SDRBench EXAFEL: LCLS X-ray free-electron laser detector frames, 10 events x 32 panels x 185x388 pixels, as three raw uint16 "
         "arrays (raw, calibrated, dark).",
         tags=["public-benchmark"]),
    item("f07-validation-nasa-gistemp-1200km", "F07", "validation", "medium", "download", "real",
         {"inputs": [dl("gistemp", "https://data.giss.nasa.gov/pub/gistemp/gistemp1200_GHCNv4_ERSSTv5.nc.gz")],
          "steps": [{"op": "decompress", "input": "gistemp", "dest": "gistemp1200_GHCNv4_ERSSTv5.nc", "format": "gz"}]},
         lic("Public domain (NASA GISS, US Government work); cite GISTEMP Team and Lenssen et al.", True,
             "GISTEMP Team, NASA Goddard Institute for Space Studies, https://data.giss.nasa.gov/gistemp/",
             "The file is updated monthly upstream; the TOFU pin and cache fix the retrieved bytes."),
         "nasa-giss-gistemp",
         "NASA GISTEMP v4 monthly surface temperature anomaly grid (1200 km smoothing, GHCNv4 + ERSSTv5), NetCDF, decompressed.",
         notes="Substitute for Unidata sample NetCDF files (moved/404)."),
    item("f07-validation-gen-numeric-b", "F07", "validation", "medium", "generate", "generated",
         {"generator": gen("gen_numeric_ab.py", 7201, {"arrays": [
             {"path": "cube/vorticity_128x256x256.f32", "kind": "grf", "shape": [128, 256, 256], "dtype": "float32", "beta": 2.0},
             {"path": "grid/geopotential_2048x2048.f64", "kind": "grf", "shape": [2048, 2048], "dtype": "float64", "beta": 4.0, "scale": 150.0, "offset": 5500.0},
             {"path": "grid/counts_2048x2048_uint16.npy", "kind": "grf", "shape": [2048, 2048], "dtype": "uint16", "beta": 1.5, "scale": 5000.0,
              "offset": 30000.0, "container": "npy"},
             {"path": "daq/channels_4x5M.i32", "kind": "timeseries", "shape": [4, 5000000], "dtype": "int32", "scale": 200000.0, "noise": 0.3},
             {"path": "daq/timestamps_2M_int64.npy", "kind": "timestamps", "shape": [2000000], "dtype": "int64", "step": 0.00025, "container": "npy"}]})},
         GENERATED_LIC, "ebrc-gen-numeric-b",
         "Generated numeric arrays (profile B): float32 3-D field, float64 and uint16 2-D fields, int32 DAQ channels and int64 timestamps.",
         notes="Same generator script as ebrc-gen-numeric-a with different kinds, shapes, spectra and seed; not independent in distribution.",
         tags=["generated"]),
    item("f07-validation-gen-numeric-b-small", "F07", "validation", "small", "generate", "generated",
         {"generator": gen("gen_numeric_ab.py", 7200, {"arrays": [
             {"path": "grid/geopotential_1024x1024.f64", "kind": "grf", "shape": [1024, 1024], "dtype": "float64", "beta": 4.0, "scale": 150.0, "offset": 5500.0},
             {"path": "grid/counts_1024x512_uint16.npy", "kind": "grf", "shape": [512, 1024], "dtype": "uint16", "beta": 1.5, "scale": 5000.0,
              "offset": 30000.0, "container": "npy"},
             {"path": "daq/channels_2x500k.i32", "kind": "timeseries", "shape": [2, 500000], "dtype": "int32", "scale": 200000.0, "noise": 0.3}]})},
         GENERATED_LIC, "ebrc-gen-numeric-b",
         "Generated numeric arrays (profile B, small): float64 and uint16 2-D fields and int32 DAQ channels.",
         notes="Same generator script as ebrc-gen-numeric-a with different kinds, shapes, spectra and seed; not independent in distribution.",
         tags=["generated"]),
    # heldout
    item("f07-heldout-noaa-etopo2022-60s-surface", "F07", "heldout", "medium", "download", "real",
         {"inputs": [dl("etopo", "https://www.ngdc.noaa.gov/thredds/fileServer/global/ETOPO2022/60s/60s_surface_elev_netcdf/ETOPO_2022_v1_60s_N90W180_surface.nc")]},
         lic(US_PD, True, "NOAA National Centers for Environmental Information, ETOPO 2022 Global Relief Model, doi:10.25921/fd45-gt74"),
         "noaa-ncei-etopo2022",
         "NOAA NCEI ETOPO 2022 v1 global relief model, 60 arc-second ice-surface elevation grid, NetCDF.",
         notes="Held-out item: description is provenance only."),
    item("f07-heldout-gwosc-gw150914-h1-4khz-4096s", "F07", "heldout", "medium", "download", "real",
         {"inputs": [dl("strain", "https://gwosc.org/eventapi/json/GWTC-1-confident/GW150914/v3/H-H1_GWOSC_4KHZ_R1-1126257415-4096.hdf5")]},
         lic("CC-BY-4.0", True, "LIGO Scientific Collaboration and Virgo Collaboration, Gravitational Wave Open Science Center (GWOSC), https://gwosc.org"),
         "gwosc-strain-data",
         "GWOSC LIGO Hanford (H1) strain time series around GW150914, 4 kHz, 4096 s, HDF5 (GWTC-1 release v3).",
         notes="Held-out item: description is provenance only. Substitute for a generic HDF5 sample file."),
    item("f07-heldout-gen-numeric-c", "F07", "heldout", "medium", "generate", "generated",
         {"generator": gen("gen_numeric_c.py", 7301, {"runs": [
             {"name": "reaction-diffusion", "model": "gray-scott", "n": 512, "steps": 6000, "every": 150, "F": 0.037, "k": 0.06},
             {"name": "tracer", "model": "advection", "n": 512, "steps": 3000, "every": 100},
             {"name": "lorenz96-ensemble", "model": "lorenz96", "k": 40, "members": 64, "steps": 40000, "every": 4},
             {"name": "hydrophone", "model": "ar-audio", "channels": 4, "seconds": 240, "rate": 48000}]})},
         GENERATED_LIC, "ebrc-gen-numeric-c",
         "Generated simulation outputs (generator C): reaction-diffusion and Lorenz-96 NetCDF-3 files, a float64 tracer volume, int16 PCM and int32 counters.",
         notes="Held-out generated item uses a different generator script, models and containers than generators A/B.",
         tags=["generated"]),
]

ITEMS += [
    item("f07-heldout-gen-numeric-c-small", "F07", "heldout", "small", "generate", "generated",
         {"generator": gen("gen_numeric_c.py", 7300, {"runs": [
             {"name": "reaction-diffusion", "model": "gray-scott", "n": 192, "steps": 3000, "every": 250, "F": 0.029, "k": 0.057},
             {"name": "lorenz96-ensemble", "model": "lorenz96", "k": 36, "members": 8, "steps": 10000, "every": 10},
             {"name": "hydrophone", "model": "ar-audio", "channels": 1, "seconds": 20, "rate": 22050}]})},
         GENERATED_LIC, "ebrc-gen-numeric-c",
         "Generated simulation outputs (generator C, small): reaction-diffusion and Lorenz-96 NetCDF-3 files and int16 PCM with int32 counters.",
         notes="Held-out generated item uses a different generator script, models and containers than generators A/B.",
         tags=["generated"]),
]

# =======================================================================================
# F08 databases
# =======================================================================================
PG_IMAGE = "docker.io/library/postgres:17.6@sha256:00bc86618629af00d2937fdc5a5d63db3ff8450acf52f0636ec813c7f4902929"
MARIADB_IMAGE = "docker.io/library/mariadb:11.4.8@sha256:bc474f00629f0123c10f9e1bca193a45d18af15a274cf0656acda64f1086c3b6"
ITEMS += [
    item("f08-tuning-chinook-sqlite", "F08", "tuning", "small", "download", "real",
         {"inputs": [dl("chinook", "https://github.com/lerocha/chinook-database/releases/download/v1.4.5/Chinook_Sqlite.sqlite")]},
         lic("MIT", True, "Chinook Database, Copyright (c) 2008-2024 Luis Rocha, https://github.com/lerocha/chinook-database"),
         "chinook-sample-db",
         "Chinook sample database (digital media store) as a SQLite file, release v1.4.5.",
         notes="Sample database: music catalogue derived from an iTunes library, customer/invoice rows are fictitious."),
    item("f08-tuning-silesia-osdb", "F08", "tuning", "small", "download", "real",
         {"inputs": [dl("osdb", SIL + "osdb.bz2")], "steps": [{"op": "decompress", "input": "osdb", "dest": "osdb", "format": "bz2"}]},
         SILESIA, "silesia-corpus",
         "Silesia corpus 'osdb': MySQL-format sample database file from the Open Source Database Benchmark, decompressed from the per-file bz2.",
         tags=["public-benchmark"]),
    item("f08-tuning-postgres17-pgbench-s40", "F08", "tuning", "large", "build", "generated",
         {"generator": gen("build_postgres_pgbench.sh", 0, {"image": PG_IMAGE, "platform": "linux/amd64", "scale": 40,
                                                            "initdb_args": "--locale=C --encoding=UTF8"}, interpreter="bash"),
          "output_pin": None,
          "notes": "Docker image digest is the multi-arch index digest of postgres:17.6; platform linux/amd64 (native, not emulated)."},
         lic("PostgreSQL License (server software); generated pgbench table data", True,
             "PostgreSQL Global Development Group; Docker official image 'postgres'",
             "The data directory holds only system catalogs and pgbench-generated rows."),
         "postgres-pgbench",
         "PostgreSQL 17.6 data directory (PGDATA) after initdb and pgbench -i at scale factor 40, cleanly shut down, "
         "built in the digest-pinned official postgres image.",
         notes="Not bit-reproducible (system identifier, LSNs, timestamps): output_pin null, logical hash recorded only.",
         tags=["generated", "docker-build"]),
    item("f08-tuning-gen-sqlite-tpch-small", "F08", "tuning", "small", "generate", "generated",
         {"generator": gen("gen_sqlite_tpch.py", 8100, {"sf": 0.008, "filename": "tpch-sf0.008.sqlite", "page_size": 4096})},
         GENERATED_LIC, "ebrc-gen-sqlite-tpch",
         "Generated TPC-H-like SQLite database (not TPC-H) at scale factor 0.008 with secondary indexes and ANALYZE statistics.",
         tags=["generated"]),
    item("f08-tuning-gen-sqlite-tpch-medium", "F08", "tuning", "medium", "generate", "generated",
         {"generator": gen("gen_sqlite_tpch.py", 8101, {"sf": 0.2, "filename": "tpch-sf0.2.sqlite", "page_size": 4096})},
         GENERATED_LIC, "ebrc-gen-sqlite-tpch",
         "Generated TPC-H-like SQLite database (not TPC-H) at scale factor 0.2 with secondary indexes and ANALYZE statistics.",
         tags=["generated"]),
    # validation
    item("f08-validation-sakila-sqlite", "F08", "validation", "small", "download", "real",
         {"inputs": [dl("sakila", "https://raw.githubusercontent.com/bradleygrant/sakila-sqlite3/9394b42d13888c3d3d3d56cd7e9c84fadafb71c7/sakila_master.db")]},
         lic("BSD-3-Clause", True, "Sakila sample database (MySQL AB / Oracle), SQLite3 port by Bradley Grant, https://github.com/bradleygrant/sakila-sqlite3"),
         "sakila-sample-db",
         "Sakila sample database (DVD rental store) SQLite3 port, commit-pinned raw file.",
         notes="Sample database with fictitious rows."),
    item("f08-validation-northwind-sqlite", "F08", "validation", "medium", "download", "real",
         {"inputs": [dl("northwind", "https://raw.githubusercontent.com/jpwhite3/northwind-SQLite3/4f56e7f5906dfd23b25244c5bfe8fb5da6402efd/dist/northwind.db")]},
         lic("MIT (SQLite port); Northwind sample data originally published by Microsoft", True,
             "Northwind SQLite3 port by jpwhite3, https://github.com/jpwhite3/northwind-SQLite3; Microsoft Northwind sample",
             "The port extends the classic Northwind rows with generated orders."),
         "northwind-sample-db",
         "Northwind sample database (trading company) SQLite3 port, commit-pinned raw file.",
         notes="Sample database with fictitious rows."),
    item("f08-validation-mariadb11-test-db-employees", "F08", "validation", "medium", "build", "generated",
         {"git": {"repo": "https://github.com/datacharmer/test_db", "commit": "e324b56193ca506ab7cc1ab143a9153d8c4535d7", "ref": "master"},
          "generator": gen("build_mariadb_test_db.sh", 0, {"image": MARIADB_IMAGE, "platform": "linux/amd64", "sql": "employees.sql"},
                           interpreter="bash"),
          "output_pin": None,
          "notes": "Docker image digest is the multi-arch index digest of mariadb:11.4.8; platform linux/amd64 (native)."},
         lic("CC-BY-SA-3.0 (test_db data and scripts); GPL-2.0 server software", True,
             "Giuseppe Maxia (datacharmer/test_db); employee data by Fusheng Wang and Carlo Zaniolo, Siemens Corporate Research; MariaDB Foundation image",
             "Share-alike applies to the loaded dataset."),
         "mysql-test-db-employees",
         "MariaDB 11.4.8 InnoDB data directory after loading the datacharmer/test_db 'employees' database, slow clean shutdown, "
         "built in the digest-pinned official mariadb image.",
         notes="The employees dataset is synthetic sample data. Not bit-reproducible: output_pin null.",
         tags=["generated", "docker-build"]),
    # heldout
    item("f08-heldout-natural-earth-vector-gpkg", "F08", "heldout", "large", "download", "real",
         {"inputs": [dl("ne", "https://naciscdn.org/naturalearth/packages/natural_earth_vector.gpkg.zip")],
          "steps": [{"op": "extract", "input": "ne", "format": "zip"}]},
         lic("Public domain (Natural Earth terms of use)", True, "Made with Natural Earth, https://www.naturalearthdata.com"),
         "natural-earth-vector",
         "Natural Earth vector data package as an OGC GeoPackage (SQLite) file, extracted from the NACIS CDN zip.",
         notes="Held-out item: description is provenance only. The package URL is unversioned; the TOFU pin fixes the retrieved bytes."),
    item("f08-heldout-lahman-baseball-sqlite-2022", "F08", "heldout", "medium", "download", "real",
         {"inputs": [dl("lahman", "https://github.com/jknecht/baseball-archive-sqlite/releases/download/2022/lahman_1871-2022.sqlite")]},
         lic("CC-BY-SA-3.0", True, "Sean Lahman Baseball Database; SQLite conversion by John Knecht, https://github.com/jknecht/baseball-archive-sqlite"),
         "lahman-baseball-db",
         "Sean Lahman baseball statistics database 1871-2022 as a SQLite file (release 2022).",
         notes="Held-out item: description is provenance only."),
    item("f08-heldout-gen-sqlite-churn", "F08", "heldout", "medium", "generate", "generated",
         {"generator": gen("gen_sqlite_churn.py", 8301, {"databases": [
             {"file": "chat/messages.sqlite", "profile": "messaging", "ops": 400000, "page_size": 8192, "journal_mode": "wal",
              "auto_vacuum": "none", "fts": True, "blob_max": 60000},
             {"file": "cache/kv.sqlite", "profile": "kvstore", "ops": 300000, "page_size": 1024, "journal_mode": "delete",
              "auto_vacuum": "incremental", "blob_max": 16000},
             {"file": "metrics/telemetry.sqlite", "profile": "telemetry", "ops": 1500000, "page_size": 4096, "journal_mode": "delete",
              "auto_vacuum": "none", "series": 400}]})},
         GENERATED_LIC, "ebrc-gen-sqlite-churn",
         "Generated application SQLite databases aged by an OLTP churn workload (generator distinct from the TPC-H-like loader).",
         notes="Held-out generated item uses a different generator script and workload model than the tuning generated databases.",
         tags=["generated"]),
]

ITEMS += [
    item("f08-heldout-gen-sqlite-churn-small", "F08", "heldout", "small", "generate", "generated",
         {"generator": gen("gen_sqlite_churn.py", 8300, {"databases": [
             {"file": "chat/messages.sqlite", "profile": "messaging", "ops": 30000, "page_size": 4096, "journal_mode": "delete",
              "auto_vacuum": "full", "fts": False, "blob_max": 20000},
             {"file": "cache/kv.sqlite", "profile": "kvstore", "ops": 12000, "page_size": 16384, "journal_mode": "wal",
              "auto_vacuum": "none", "blob_max": 4000}]})},
         GENERATED_LIC, "ebrc-gen-sqlite-churn",
         "Generated application SQLite databases aged by an OLTP churn workload (generator distinct from the TPC-H-like loader), small.",
         notes="Held-out generated item uses a different generator script and workload model than the tuning generated databases.",
         tags=["generated"]),
]

# =======================================================================================
# GAP CLOSURE 2026-09-16: F07 ML model/data distribution (BLOCKER), F08 database dumps
# (BLOCKER), F05 telemetry/binary logs (MAJOR), F07 scientific archive formats beyond NetCDF
# (MAJOR), F06 columnar formats (MINOR).  See research/corpus/critique-round1.md G01/G02/G11/
# G15/G20.  Docker Desktop's Linux engine is unavailable this session (non-admin; the
# com.docker.service backend needs an administrator to start), so the Docker-dependent derive
# sub-items from G01/G02 -- pg_dump -Fp/-Fc/-Fd of the existing postgres17-pgbench item,
# Ensembl loaded into MariaDB (real-content InnoDB plus mysqldump), and a mysqldump of the
# existing employees test db -- are NOT included here and remain open; see PROGRESS.md.
# =======================================================================================


def dlx(name, url, sha256, size, filename=None, notes=None):
    """Like dl(), but with an explicit (verified) sha256/size instead of TOFU."""
    d = {"name": name, "url": url, "sha256": sha256, "size": size}
    if filename:
        d["filename"] = filename
    if notes:
        d["notes"] = notes
    return d


def hf_resolve(repo, commit, path, dataset=False):
    prefix = "datasets/" if dataset else ""
    return f"https://huggingface.co/{prefix}{repo}/resolve/{commit}/{path}"


# ---- F07 ML model weights and dataset-shard distribution (critique G01) ---------------

PYTHIA_LIC = lic("Apache-2.0", True, "EleutherAI, https://huggingface.co/EleutherAI/pythia-160m",
                 "Pythia suite model checkpoint.")
QWEN_LIC = lic("Apache-2.0", True, "Qwen Team, Alibaba Cloud, https://huggingface.co/Qwen/Qwen2.5-1.5B", "")
QWEN_GGUF_LIC = lic("Apache-2.0", True,
                    "Qwen Team, Alibaba Cloud (GGUF quantization), "
                    "https://huggingface.co/Qwen/Qwen2.5-0.5B-Instruct-GGUF",
                    "llama.cpp-format Q4_K_M quantized conversion of Qwen2.5-0.5B-Instruct.")
MNIST_LIC = lic("CC-BY-SA-3.0 (MNIST database)", True,
                "Yann LeCun and Corinna Cortes, http://yann.lecun.com/exdb/mnist/; "
                "TensorFlow Keras datasets mirror",
                "Retrieved from the tensorflow.org Keras datasets mirror on storage.googleapis.com.")
SMOLLM2_LIC = lic("Apache-2.0", True, "HuggingFaceTB, https://huggingface.co/HuggingFaceTB/SmolLM2-360M", "")
BERT_LIC = lic("Apache-2.0", True,
               "Google Research; google-bert, https://huggingface.co/google-bert/bert-base-uncased",
               "Identical weights distributed in three container formats (PyTorch pickle, safetensors, "
               "Keras HDF5).")
PHI3_LIC = lic("MIT", True, "Microsoft, https://huggingface.co/microsoft/Phi-3-mini-4k-instruct",
               "Sharded BF16 safetensors checkpoint.")
FINEWEB_LIC = lic("ODC-By-1.0", True,
                  "HuggingFaceFW (G. Penedo et al.), https://huggingface.co/datasets/HuggingFaceFW/fineweb-edu", "")

ITEMS += [
    # tuning: eleutherai-pythia -- a versioned checkpoint pair (same model, two revisions/dtypes)
    item("f07-tuning-pythia-160m-safetensors-main", "F07", "tuning", "medium", "download", "real",
         {"inputs": [dlx("model", hf_resolve("EleutherAI/pythia-160m",
                                             "50f5173d932e8e61f858120bcb800b97af589f46", "model.safetensors"),
                        "29d2e457a664e41c12c735f20a36dc0956a665f614a54ce5db21a32e75965270", 374998696,
                        filename="model.safetensors")]},
         PYTHIA_LIC, "eleutherai-pythia",
         "EleutherAI Pythia-160M model weights (main revision, commit 50f5173d), safetensors, FP16 by file size.",
         notes="REQ-CMP-0149 input: a real BF16/FP16-range model checkpoint for the sign/exponent/mantissa split.",
         tags=["model-weights"]),
    item("f07-tuning-pythia-160m-safetensors-step100000", "F07", "tuning", "large", "download", "real",
         {"inputs": [dlx("model", hf_resolve("EleutherAI/pythia-160m",
                                             "c507d0e63f5a7a833b1b1866116a04cc3e74dc70", "model.safetensors"),
                        "3164be2625f407dd83703a2d218277b7e80a03e28c9d51fc9d40dd5e34c16c14", 649308728,
                        filename="model.safetensors")]},
         PYTHIA_LIC, "eleutherai-pythia",
         "EleutherAI Pythia-160M model weights at training checkpoint revision step100000, safetensors, "
         "FP32 by file size: a same-model, different-revision, different-dtype companion checkpoint.",
         tags=["model-weights"]),
    # tuning: qwen2-5 -- BF16 base model plus a quantized GGUF instruct model
    item("f07-tuning-qwen2-5-1-5b-safetensors-bf16", "F07", "tuning", "large", "download", "real",
         {"inputs": [dlx("model", hf_resolve("Qwen/Qwen2.5-1.5B",
                                             "8faed761d45a263340a0528343f099c05c9a4323", "model.safetensors"),
                        "a961db72e75d52b18e6b0c9d379e51a26973b233385e0e127fdda7d648aec796", 3087467144,
                        filename="model.safetensors")]},
         QWEN_LIC, "qwen2-5", "Qwen2.5-1.5B base model weights, BF16 safetensors.",
         tags=["model-weights"]),
    item("f07-tuning-qwen2-5-0-5b-instruct-gguf-q4km", "F07", "tuning", "medium", "download", "real",
         {"inputs": [dlx("model", hf_resolve("Qwen/Qwen2.5-0.5B-Instruct-GGUF",
                                             "9217f5db79a29953eb74d5343926648285ec7e67",
                                             "qwen2.5-0.5b-instruct-q4_k_m.gguf"),
                        "74a4da8c9fdbcd15bd1f6d01d621410d31c6fc00986f5eb687824e7b93d7a9db", 491400032,
                        filename="qwen2.5-0.5b-instruct-q4_k_m.gguf")]},
         QWEN_GGUF_LIC, "qwen2-5",
         "Qwen2.5-0.5B-Instruct quantized to GGUF Q4_K_M: a near-incompressible, bit-packed quantized weight file.",
         tags=["model-weights", "quantized"]),
    # tuning: mnist -- small real .npz dataset array
    item("f07-tuning-mnist-npz", "F07", "tuning", "small", "download", "real",
         {"inputs": [dl("mnist", "https://storage.googleapis.com/tensorflow/tf-keras-datasets/mnist.npz",
                       filename="mnist.npz")]},
         MNIST_LIC, "mnist",
         "MNIST handwritten-digit dataset as a compressed NumPy .npz archive (train/test images and labels).",
         tags=["dataset-shard"]),
    # validation: huggingfacetb-smollm2 -- two sizes of the same model family
    item("f07-validation-smollm2-360m-safetensors", "F07", "validation", "large", "download", "real",
         {"inputs": [dlx("model", hf_resolve("HuggingFaceTB/SmolLM2-360M",
                                             "f8027fd0eaeea54caa13c31d31b9fdc459c38b49", "model.safetensors"),
                        "7aaff6661428bed033abba9522bec81938678642cca3181fe752b6ca9e1e540f", 723674912,
                        filename="model.safetensors")]},
         SMOLLM2_LIC, "huggingfacetb-smollm2", "SmolLM2-360M model weights, safetensors.",
         tags=["model-weights"]),
    item("f07-validation-smollm2-135m-safetensors", "F07", "validation", "medium", "download", "real",
         {"inputs": [dlx("model", hf_resolve("HuggingFaceTB/SmolLM2-135M",
                                             "93efa2f097d58c2a74874c7e644dbc9b0cee75a2", "model.safetensors"),
                        "80521b40281d6ce74e35c9282c22539e75aa0ac8578892b2a59955ef78d55da1", 269060552,
                        filename="model.safetensors")]},
         SMOLLM2_LIC, "huggingfacetb-smollm2",
         "SmolLM2-135M model weights, safetensors: a smaller sibling checkpoint.",
         tags=["model-weights"]),
    # validation: google-bert -- same weights, three container formats
    item("f07-validation-bert-base-uncased-pytorch-bin", "F07", "validation", "medium", "download", "real",
         {"inputs": [dlx("model", hf_resolve("google-bert/bert-base-uncased",
                                             "86b5e0934494bd15c9632b12f734a8a67f723594", "pytorch_model.bin"),
                        "097417381d6c7230bd9e3557456d726de6e83245ec8b24f529f60198a67b203a", 440473133,
                        filename="pytorch_model.bin")]},
         BERT_LIC, "google-bert", "bert-base-uncased weights as a PyTorch pickle checkpoint (pytorch_model.bin).",
         tags=["model-weights"]),
    item("f07-validation-bert-base-uncased-safetensors", "F07", "validation", "medium", "download", "real",
         {"inputs": [dlx("model", hf_resolve("google-bert/bert-base-uncased",
                                             "86b5e0934494bd15c9632b12f734a8a67f723594", "model.safetensors"),
                        "68d45e234eb4a928074dfd868cead0219ab85354cc53d20e772753c6bb9169d3", 440449768,
                        filename="model.safetensors")]},
         BERT_LIC, "google-bert",
         "bert-base-uncased weights as safetensors: same weights as the pytorch_model.bin item.",
         tags=["model-weights"]),
    item("f07-validation-bert-base-uncased-tf-h5", "F07", "validation", "medium", "download", "real",
         {"inputs": [dlx("model", hf_resolve("google-bert/bert-base-uncased",
                                             "86b5e0934494bd15c9632b12f734a8a67f723594", "tf_model.h5"),
                        "a7a17d6d844b5de815ccab5f42cad6d24496db3850a2a43d8258221018ce87d2", 536063208,
                        filename="tf_model.h5")]},
         BERT_LIC, "google-bert",
         "bert-base-uncased weights as a Keras/TensorFlow HDF5 checkpoint: same weights, third container format.",
         tags=["model-weights"]),
    # held-out: microsoft-phi3 (sharded BF16 instruct model) and huggingfacefw-fineweb (dataset shard)
    item("f07-heldout-phi3-mini-safetensors-p1of2", "F07", "heldout", "large", "download", "real",
         {"inputs": [dlx("model", hf_resolve("microsoft/Phi-3-mini-4k-instruct",
                                             "f39ac1d28e925b323eae81227eaba4464caced4e",
                                             "model-00001-of-00002.safetensors"),
                        "b7492726c01287bf6e13c3d74c65ade3d436d50da1cf5bb6925bc962419d6610", 4972489328,
                        filename="model-00001-of-00002.safetensors")]},
         PHI3_LIC, "microsoft-phi3", "Phi-3-mini-4k-instruct BF16 safetensors, shard 1 of 2.",
         notes="Held-out item: description is provenance only.", tags=["model-weights"]),
    item("f07-heldout-phi3-mini-safetensors-p2of2", "F07", "heldout", "large", "download", "real",
         {"inputs": [dlx("model", hf_resolve("microsoft/Phi-3-mini-4k-instruct",
                                             "f39ac1d28e925b323eae81227eaba4464caced4e",
                                             "model-00002-of-00002.safetensors"),
                        "3f311787aa136e858556caa8543015161edcad85ba81b6a36072443d7fa73c87", 2669692552,
                        filename="model-00002-of-00002.safetensors")]},
         PHI3_LIC, "microsoft-phi3", "Phi-3-mini-4k-instruct BF16 safetensors, shard 2 of 2.",
         notes="Held-out item: description is provenance only.", tags=["model-weights"]),
    item("f07-heldout-fineweb-edu-parquet-sample10bt", "F07", "heldout", "large", "download", "real",
         {"inputs": [dlx("shard", hf_resolve("HuggingFaceFW/fineweb-edu",
                                             "87f09149ef4734204d70ed1d046ddc9ca3f2b8f9",
                                             "sample/10BT/000_00000.parquet", dataset=True),
                        "b1ba7b2ce4cb5ea6ef42dca40263eabb85f37700d01693a68e9b30a31d78e871", 2152819114,
                        filename="000_00000.parquet")]},
         FINEWEB_LIC, "huggingfacefw-fineweb",
         "fineweb-edu 10BT sample, shard 000_00000: one Parquet shard of a pretraining-scale web-text "
         "dataset (also stands in as the F06 large-tier held-out columnar item; see G20 in critique-round1.md).",
         notes="Held-out item: description is provenance only.", tags=["dataset-shard"]),
]

# ---- F08 database dumps (critique G02) -------------------------------------------------

ITEMS += [
    item("f08-tuning-wikimedia-simplewiki-sql-dump", "F08", "tuning", "large", "download", "real",
         {"inputs": [dl("page", "https://dumps.wikimedia.org/simplewiki/20260901/simplewiki-20260901-page.sql.gz",
                       upstream_digest=wikimedia_dumpstatus(
                           "https://dumps.wikimedia.org/simplewiki/20260901/simplewiki-20260901-page.sql.gz")),
                     dl("categorylinks",
                        "https://dumps.wikimedia.org/simplewiki/20260901/simplewiki-20260901-categorylinks.sql.gz",
                        upstream_digest=wikimedia_dumpstatus(
                            "https://dumps.wikimedia.org/simplewiki/20260901/simplewiki-20260901-categorylinks.sql.gz")),
                     dl("pagelinks",
                        "https://dumps.wikimedia.org/simplewiki/20260901/simplewiki-20260901-pagelinks.sql.gz",
                        upstream_digest=wikimedia_dumpstatus(
                            "https://dumps.wikimedia.org/simplewiki/20260901/simplewiki-20260901-pagelinks.sql.gz"))],
          "steps": [{"op": "decompress", "input": "page", "dest": "simplewiki-20260901-page.sql", "format": "gz"},
                    {"op": "decompress", "input": "categorylinks",
                     "dest": "simplewiki-20260901-categorylinks.sql", "format": "gz"},
                    {"op": "decompress", "input": "pagelinks",
                     "dest": "simplewiki-20260901-pagelinks.sql", "format": "gz"}]},
         lic("CC-BY-SA-4.0 AND GFDL-1.3 (Wikipedia text); dump metadata CC0", True,
             "Simple English Wikipedia contributors; Wikimedia Foundation dumps, https://dumps.wikimedia.org",
             "mysqldump-format SQL table dumps; share-alike applies to the underlying wiki text."),
         "wikimedia-simplewiki",
         "Wikimedia Simple English Wikipedia (simplewiki) MySQL table dumps of 2026-09-01: page, "
         "categorylinks and pagelinks tables, mysqldump/MediaWiki export SQL format, decompressed from gz."),
]

ENSEMBL_FILES = [
    "alt_allele.txt.gz", "alt_allele_attrib.txt.gz", "alt_allele_group.txt.gz", "analysis.txt.gz",
    "analysis_description.txt.gz", "assembly.txt.gz", "assembly_exception.txt.gz", "associated_group.txt.gz",
    "associated_xref.txt.gz", "attrib_type.txt.gz", "biotype.txt.gz", "coord_system.txt.gz", "data_file.txt.gz",
    "density_feature.txt.gz", "density_type.txt.gz", "dependent_xref.txt.gz", "ditag.txt.gz",
    "ditag_feature.txt.gz", "dna.txt.gz", "dna_align_feature.txt.gz", "dna_align_feature_attrib.txt.gz",
    "exon.txt.gz", "exon_transcript.txt.gz", "external_db.txt.gz", "external_synonym.txt.gz", "gene.txt.gz",
    "gene_archive.txt.gz", "gene_attrib.txt.gz", "genome_statistics.txt.gz", "identity_xref.txt.gz",
    "interpro.txt.gz", "intron_supporting_evidence.txt.gz", "karyotype.txt.gz", "map.txt.gz",
    "mapping_session.txt.gz", "mapping_set.txt.gz", "marker.txt.gz", "marker_feature.txt.gz",
    "marker_map_location.txt.gz", "marker_synonym.txt.gz", "meta.txt.gz", "meta_coord.txt.gz",
    "misc_attrib.txt.gz", "misc_feature.txt.gz", "misc_feature_misc_set.txt.gz", "misc_set.txt.gz",
    "object_xref.txt.gz", "ontology_xref.txt.gz", "operon.txt.gz", "operon_transcript.txt.gz",
    "operon_transcript_gene.txt.gz", "peptide_archive.txt.gz", "prediction_exon.txt.gz",
    "prediction_transcript.txt.gz", "protein_align_feature.txt.gz", "protein_feature.txt.gz",
    "repeat_consensus.txt.gz", "repeat_feature.txt.gz", "rnaproduct.txt.gz", "rnaproduct_attrib.txt.gz",
    "rnaproduct_type.txt.gz", "saccharomyces_cerevisiae_core_114_4.sql.gz", "seq_region.txt.gz",
    "seq_region_attrib.txt.gz", "seq_region_mapping.txt.gz", "seq_region_synonym.txt.gz",
    "simple_feature.txt.gz", "stable_id_event.txt.gz", "supporting_feature.txt.gz", "transcript.txt.gz",
    "transcript_attrib.txt.gz", "transcript_intron_supporting_evidence.txt.gz",
    "transcript_supporting_feature.txt.gz", "translation.txt.gz", "translation_attrib.txt.gz",
    "unmapped_object.txt.gz", "unmapped_reason.txt.gz", "xref.txt.gz",
]
ENSEMBL_BASE = "https://ftp.ensembl.org/pub/release-114/mysql/saccharomyces_cerevisiae_core_114_4/"


def _ensembl_name(fname):
    return fname[:-7] + "_schema" if fname.endswith(".sql.gz") else fname[:-7]


ITEMS += [
    item("f08-validation-ensembl-scerevisiae-core-sql-dump", "F08", "validation", "medium", "download", "real",
         {"inputs": [dl(_ensembl_name(f), ENSEMBL_BASE + f) for f in ENSEMBL_FILES],
          "steps": [{"op": "decompress", "input": _ensembl_name(f), "dest": f[:-3], "format": "gz"}
                    for f in ENSEMBL_FILES]},
         lic("Ensembl data: no restrictions on use", True,
             "Ensembl (EMBL-EBI), https://www.ensembl.org; release 114, Saccharomyces cerevisiae core database",
             "See https://www.ensembl.org/info/about/legal/disclaimer.html."),
         "ensembl",
         "Ensembl release-114 MySQL table dumps (schema plus tab-delimited data, 78 files) for the "
         "Saccharomyces cerevisiae core database (core_114_4), as published, decompressed from gz."),
]

ITEMS += [
    item("f08-validation-stackexchange-cs-sqlite", "F08", "validation", "large", "download", "derived-from-real",
         {"inputs": [dl("archive", "https://archive.org/download/stackexchange/cs.stackexchange.com.7z",
                       filename="cs.stackexchange.com.7z")],
          "steps": [{"op": "run", "script": f"{GEN}/build_stackexchange_sqlite.py",
                     "params": {"input": "archive", "db_name": "cs.stackexchange.com.sqlite"}}]},
         lic("CC-BY-SA-4.0", True,
             "Stack Exchange, Inc. / cs.stackexchange.com contributors; Internet Archive mirror of the "
             "Stack Exchange Data Dump, https://archive.org/details/stackexchange",
             "Loaded from the published 7z into a single SQLite database by "
             "research/corpus/generators/g3-data/build_stackexchange_sqlite.py (schema-on-read from the "
             "XML row attributes); no content is altered beyond the XML-to-SQL container change."),
         "stackexchange-cs",
         "Computer Science Stack Exchange (cs.stackexchange.com) data dump loaded into a single SQLite "
         "database: posts, users, comments, votes, tags, badges, post links and post history tables."),
]

RFAM_SQL_FILES = [
    "alignment_and_tree.sql", "clan.sql", "clan_database_link.sql", "clan_literature_reference.sql",
    "clan_membership.sql", "database_link.sql", "db_version.sql", "dead_clan.sql", "dead_family.sql",
    "family.sql", "family_literature_reference.sql", "family_ncbi.sql", "features.sql", "full_region.sql",
    "genome.sql", "genseq.sql", "html_alignment.sql", "keywords.sql", "literature_reference.sql",
    "matches_and_fasta.sql", "motif.sql", "motif_database_link.sql", "motif_family_stats.sql",
    "motif_file.sql", "motif_literature.sql", "motif_matches.sql", "motif_pdb.sql", "motif_ss_image.sql",
    "pdb_full_region.sql", "rfamseq.sql", "secondary_structure_image.sql", "seed_region.sql",
    "sunburst.sql", "taxonomy.sql", "taxonomy_websearch.sql", "version.sql", "wikitext.sql",
]
RFAM_TXT_GZ_FILES = [
    "alignment_and_tree.txt.gz", "clan.txt.gz", "clan_database_link.txt.gz",
    "clan_literature_reference.txt.gz", "clan_membership.txt.gz", "database_link.txt.gz",
    "db_version.txt.gz", "dead_clan.txt.gz", "dead_family.txt.gz", "family.txt.gz",
    "family_literature_reference.txt.gz", "family_ncbi.txt.gz", "features.txt.gz", "full_region.txt.gz",
    "genome.txt.gz", "html_alignment.txt.gz", "keywords.txt.gz", "literature_reference.txt.gz",
    "matches_and_fasta.txt.gz", "motif.txt.gz", "motif_database_link.txt.gz", "motif_family_stats.txt.gz",
    "motif_file.txt.gz", "motif_literature.txt.gz", "motif_matches.txt.gz", "motif_pdb.txt.gz",
    "motif_ss_image.txt.gz", "pdb_full_region.txt.gz", "secondary_structure_image.txt.gz",
    "seed_region.txt.gz", "sunburst.txt.gz", "taxonomy.txt.gz", "taxonomy_websearch.txt.gz",
    "version.txt.gz", "wikitext.txt.gz",
]
RFAM_BASE = "https://ftp.ebi.ac.uk/pub/databases/Rfam/15.0/database_files/"

ITEMS += [
    item("f08-heldout-ebi-rfam-15-0-sql-dump", "F08", "heldout", "large", "download", "real",
         {"inputs": [dl(f.replace(".", "_"), RFAM_BASE + f) for f in RFAM_SQL_FILES]
                    + [dl(f.replace(".", "_"), RFAM_BASE + f) for f in RFAM_TXT_GZ_FILES],
          "steps": [{"op": "copy", "input": f.replace(".", "_"), "dest": f} for f in RFAM_SQL_FILES]
                   + [{"op": "decompress", "input": f.replace(".", "_"), "dest": f[:-3], "format": "gz"}
                      for f in RFAM_TXT_GZ_FILES]},
         lic("CC0-1.0", True, "Rfam / EMBL-EBI, https://rfam.org; release 15.0",
             "Full MySQL schema for every table plus tab-delimited data for every table except the two "
             "raw-sequence tables (genseq, rfamseq), excluded to keep the item well under the 8 GiB "
             "decompressed budget; full_region.txt.gz alone is 135,765,791 B compressed."),
         "ebi-rfam",
         "Rfam 15.0 MySQL database dump: schema (.sql) for all 37 tables plus tab-delimited data (.txt.gz) "
         "for 35 of them (excluding genseq and rfamseq), as published, decompressed."),
]

# ---- F05 telemetry, binary logs and logrotate-style rotation (critique G11) ------------

ITEMS += [
    item("f05-tuning-google-clusterdata-2011-2-task-usage", "F05", "tuning", "large", "download", "real",
         {"inputs": [dl(f"part{i}",
                       f"https://storage.googleapis.com/clusterdata-2011-2/task_usage/part-{i:05d}-of-00500.csv.gz")
                     for i in range(5)],
          "steps": [{"op": "decompress", "input": f"part{i}", "dest": f"part-{i:05d}-of-00500.csv", "format": "gz"}
                    for i in range(5)]},
         lic("CC-BY-4.0", True,
             "Google Inc., Google Cluster Usage Traces v2 (2011), https://github.com/google/cluster-data", ""),
         "google-clusterdata-2011-2",
         "Google cluster usage trace 2011-2: five task_usage CSV shards (part-00000 through part-00004 of "
         "500) with per-task five-minute resource-usage measurements (CPU, memory, disk I/O time, page "
         "cache), decompressed."),
    item("f05-validation-wikimedia-pageviews-20260801", "F05", "validation", "large", "download", "real",
         {"inputs": [dl(f"h{h:02d}",
                       f"https://dumps.wikimedia.org/other/pageviews/2026/2026-08/pageviews-20260801-{h:02d}0000.gz")
                     for h in range(24)],
          "steps": [{"op": "decompress", "input": f"h{h:02d}", "dest": f"pageviews-20260801-{h:02d}0000",
                     "format": "gz"} for h in range(24)]},
         lic("CC0-1.0", True,
             "Wikimedia Foundation, pageview complete dumps, https://dumps.wikimedia.org/other/pageviews/", ""),
         "wikimedia-pageviews",
         "Wikimedia hourly pageview count dumps for all 24 hours of 2026-08-01 (one full day), decompressed."),
    item("f05-validation-gen-journal-systemd", "F05", "validation", "medium", "generate", "generated",
         {"generator": gen("build_journal_export.py", 5401,
                          {"boots": 6, "entries_per_boot": 20000, "base_epoch": 1769904000}),
          "output_pin": None,
          "notes": "systemd-journal-remote stamps a random 128-bit file ID per invocation; logical entry "
                   "content is a pure function of seed/params but the journal file bytes are not (same "
                   "pattern as the Docker-built database directories in this file)."},
         GENERATED_LIC, "ebrc-gen-journal",
         "Generated systemd-journald binary journal (native on-disk journal file format, produced by the "
         "real systemd-journal-remote binary from a deterministic synthetic export stream): syslog-style "
         "service, SSH, cron, nginx, dockerd and kernel-style entries across six synthetic boot sessions.",
         tags=["generated"]),
    item("f05-heldout-backblaze-drive-stats-2025-q1", "F05", "heldout", "large", "download", "real",
         {"inputs": [dl("zip", "https://f001.backblazeb2.com/file/Backblaze-Hard-Drive-Data/data_Q1_2025.zip",
                       filename="data_Q1_2025.zip")],
          "steps": [{"op": "run", "script": f"{GEN}/extract_zip_subset.py",
                     "params": {"input": "zip", "prefixes": ["2025-01-", "2025-02-"], "strip_components": 1}}]},
         lic("Backblaze Drive Stats data terms: free to use with attribution; no resale of the dataset", False,
             "Backblaze, Inc., Drive Stats, "
             "https://www.backblaze.com/cloud-storage/resources/hard-drive-test-data",
             "Attribution required; the terms restrict redistribution/resale of the dataset itself, so this "
             "item is treated as not redistributable (hashes only)."),
         "backblaze-drive-stats",
         "Backblaze Drive Stats for Q1 2025: one CSV per day of SMART/health telemetry for every "
         "operational hard drive in Backblaze's data centers; January and February (59 of the 90 quarterly "
         "days) extracted from the quarterly zip, the remaining 31 March days dropped to stay under the "
         "corpus's 8 GiB large-tier budget (the full quarter decompresses to 10,891,076,206 B).",
         notes="Held-out item: description is provenance only."),
    item("f05-heldout-evtx-attack-samples", "F05", "heldout", "medium", "git-archive", "real",
         {"git": {"repo": "https://github.com/sbousseaden/EVTX-ATTACK-SAMPLES.git",
                  "commit": "4ceed2f4706daf601c212a8f91c113dd85349a2c", "ref": "master"}},
         lic("GPL-3.0", True,
             "sbousseaden (Samir Bousseaden), https://github.com/sbousseaden/EVTX-ATTACK-SAMPLES", ""),
         "evtx-attack-samples",
         "sbousseaden/EVTX-ATTACK-SAMPLES: a curated collection of raw Windows Event Log (.evtx) binary "
         "samples reproducing adversary techniques, git-archived at a pinned commit.",
         notes="Held-out item: description is provenance only."),
]

ITEMS += [
    item("f05-tuning-logrotate-from-loghub-hdfs", "F05", "tuning", "small", "derive", "derived-from-real",
         {"from_items": ["f05-tuning-loghub-hdfs-v1"],
          "steps": [{"op": "run", "script": f"{GEN}/logrotate_derive.py",
                     "params": {"from_item": "f05-tuning-loghub-hdfs-v1", "src": "HDFS.log",
                                "max_bytes": 9000000, "segments": 6}}]},
         GENERATED_LIC, "ebrc-gen-logrotate-tuning",
         "Logrotate-style rotated .gz directory (HDFS.log plus HDFS.log.1.gz .. HDFS.log.5.gz), derived by "
         "taking a bounded prefix of the real Loghub HDFS_v1 log (same tuning split), splitting it into six "
         "chronological segments and gzipping all but the newest, matching a `logrotate --compress` layout "
         "with no delaycompress.",
         tags=["derived"]),
    item("f05-validation-logrotate-from-loghub-ssh", "F05", "validation", "small", "derive", "derived-from-real",
         {"from_items": ["f05-validation-loghub-ssh"],
          "steps": [{"op": "run", "script": f"{GEN}/logrotate_derive.py",
                     "params": {"from_item": "f05-validation-loghub-ssh", "src": "SSH.log",
                                "max_bytes": 9000000, "segments": 6}}]},
         GENERATED_LIC, "ebrc-gen-logrotate-validation",
         "Logrotate-style rotated .gz directory derived by taking a bounded prefix of the real Loghub SSH "
         "log (same validation split), splitting it into six chronological segments and gzipping all but "
         "the newest.",
         tags=["derived"]),
    item("f05-heldout-logrotate-from-maccdc2012", "F05", "heldout", "small", "derive", "derived-from-real",
         {"from_items": ["f05-heldout-maccdc2012-zeek-logs"],
          "steps": [{"op": "run", "script": f"{GEN}/logrotate_derive.py",
                     "params": {"from_item": "f05-heldout-maccdc2012-zeek-logs", "src": "dns.log",
                                "max_bytes": 9000000, "segments": 6}}]},
         GENERATED_LIC, "ebrc-gen-logrotate-heldout",
         "Logrotate-style rotated .gz directory derived by taking a bounded prefix of the real MACCDC 2012 "
         "Zeek dns.log (same held-out split), splitting it into six chronological segments and gzipping "
         "all but the newest.",
         notes="Held-out item: description is provenance only.", tags=["derived"]),
]

# ---- F07 scientific archive formats beyond NetCDF/raw arrays (critique G15) -----------

SDSS_BASE = "https://data.sdss.org/sas/dr17/eboss/photoObj/frames/301/756"
SDSS_FRAMES = [
    ("u", 1, "0206"), ("g", 1, "0206"), ("r", 1, "0206"), ("i", 1, "0206"), ("z", 1, "0206"),
    ("g", 2, "0206"), ("r", 2, "0206"),
]

ITEMS += [
    item("f07-tuning-sdss-dr17-frames", "F07", "tuning", "medium", "download", "real",
         {"inputs": [dl(f"frame_{filt}_{camcol}",
                       f"{SDSS_BASE}/{camcol}/frame-{filt}-000756-{camcol}-{field}.fits.bz2")
                     for filt, camcol, field in SDSS_FRAMES],
          "steps": [{"op": "decompress", "input": f"frame_{filt}_{camcol}",
                     "dest": f"frame-{filt}-000756-{camcol}-{field}.fits", "format": "bz2"}
                    for filt, camcol, field in SDSS_FRAMES]},
         lic("SDSS DR17 data usage terms (freely available for research/educational use; please cite SDSS)", True,
             "Sloan Digital Sky Survey (SDSS) DR17, https://www.sdss.org/dr17/, https://data.sdss.org", ""),
         "sdss-dr17",
         "SDSS DR17 imaging frames (FITS), run 756: photometric bands u,g,r,i,z at camcol 1 field 206, plus "
         "g,r at camcol 2 field 206 (several bands and two camcols of the same run), decompressed from bz2."),
    item("f07-tuning-ann-benchmarks-fashion-mnist-hdf5", "F07", "tuning", "medium", "download", "real",
         {"inputs": [dl("hdf5", "https://ann-benchmarks.com/fashion-mnist-784-euclidean.hdf5")]},
         lic("MIT", True, "ann-benchmarks.com (E. Bernhardsson et al.); Fashion-MNIST by Zalando Research", ""),
         "ann-benchmarks-fashion-mnist",
         "ANN-Benchmarks Fashion-MNIST-784-Euclidean HDF5 file: train/test embeddings and ground-truth "
         "nearest-neighbor indices/distances for approximate-nearest-neighbor benchmarking."),
]

CMIP6_BASE = "https://storage.googleapis.com/cmip6/CMIP6/CMIP/NCAR/CESM2/historical/r1i1p1f1/Amon/tas/gn/v20190308"
CMIP6_KEYS = [
    ".zattrs", ".zgroup", ".zmetadata",
    "tas/.zarray", "tas/.zattrs", "tas/0.0.0", "tas/1.0.0", "tas/2.0.0", "tas/3.0.0",
    "lat/.zarray", "lat/.zattrs", "lat/0",
    "lon/.zarray", "lon/.zattrs", "lon/0",
    "time/.zarray", "time/.zattrs", "time/0",
    "lat_bnds/.zarray", "lat_bnds/.zattrs", "lat_bnds/0.0",
    "lon_bnds/.zarray", "lon_bnds/.zattrs", "lon_bnds/0.0",
    "time_bnds/.zarray", "time_bnds/.zattrs", "time_bnds/0.0",
]


def _zarr_name(key):
    return "z_" + re.sub(r"[^a-z0-9]+", "_", key.lower()).strip("_")


ITEMS += [
    item("f07-validation-cmip6-ncar-cesm2-tas-zarr", "F07", "validation", "medium", "download", "real",
         {"inputs": [dl(_zarr_name(k), f"{CMIP6_BASE}/{k}") for k in CMIP6_KEYS],
          "steps": [{"op": "copy", "input": _zarr_name(k), "dest": k} for k in CMIP6_KEYS]},
         lic("CC-BY-4.0", True,
             "NCAR (D. Danabasoglu et al.), CESM2 historical r1i1p1f1, CMIP6; Pangeo/Google Cloud public "
             "CMIP6 Zarr mirror",
             "Full Zarr v2 directory store (consolidated metadata plus every chunk) for the tas variable."),
         "cmip6-ncar-cesm2",
         "CMIP6 CESM2 historical near-surface air temperature (tas), monthly, r1i1p1f1/Amon/tas/gn: the "
         "complete Zarr v2 directory store (consolidated .zmetadata, coordinate arrays, and all four chunks "
         "of the 1980-month tas array), 1850-2014."),
]

ITEMS += [
    item("f07-heldout-1000genomes-chr22-vcf-bgzf", "F07", "heldout", "medium", "download", "real",
         {"inputs": [dl("vcf", "https://ftp.1000genomes.ebi.ac.uk/vol1/ftp/release/20130502/"
                        "ALL.chr22.phase3_shapeit2_mvncall_integrated_v5b.20130502.genotypes.vcf.gz",
                       filename="ALL.chr22.phase3_shapeit2_mvncall_integrated_v5b.20130502.genotypes.vcf.gz")]},
         lic("IGSR/1000 Genomes open-data terms (no restriction on use of the data)", True,
             "International Genome Sample Resource (IGSR), 1000 Genomes Project Phase 3, "
             "https://www.internationalgenome.org", ""),
         "igsr-1000genomes",
         "1000 Genomes Project Phase 3 integrated call set for chromosome 22 (2,504 samples), as-published "
         "BGZF-compressed VCF.",
         notes="Held-out item: description is provenance only."),
    item("f07-heldout-1000genomes-chr22-vcf-region-subset", "F07", "heldout", "large", "download",
         "derived-from-real",
         {"inputs": [dl("vcf", "https://ftp.1000genomes.ebi.ac.uk/vol1/ftp/release/20130502/"
                        "ALL.chr22.phase3_shapeit2_mvncall_integrated_v5b.20130502.genotypes.vcf.gz",
                       filename="ALL.chr22.phase3_shapeit2_mvncall_integrated_v5b.20130502.genotypes.vcf.gz")],
          "steps": [{"op": "run", "script": f"{GEN}/vcf_region_subset.py",
                     "params": {"input": "vcf", "target_bytes": 4000000000,
                                "output": "ALL.chr22.region-subset.vcf"}}]},
         lic("IGSR/1000 Genomes open-data terms (no restriction on use of the data)", True,
             "International Genome Sample Resource (IGSR), 1000 Genomes Project Phase 3, "
             "https://www.internationalgenome.org",
             "Decompressed with a byte-bounded prefix (a real contiguous region from the start of chr22, "
             "kept under the 8 GiB large-tier budget; the full decompression is 11,212,370,718 B)."),
         "igsr-1000genomes",
         "1000 Genomes Project Phase 3 chromosome 22 call set, decompressed and truncated to a ~4 GiB "
         "contiguous region subset starting at the first variant position (full-chromosome decompression "
         "exceeds the corpus's 8 GiB large-tier budget).",
         notes="Held-out item: description is provenance only."),
]

# ---- F06 columnar formats at scale (critique G20) --------------------------------------

ITEMS += [
    item("f06-tuning-nyc-tlc-yellow-tripdata-2024", "F06", "tuning", "medium", "download", "real",
         {"inputs": [dl(f"m{m:02d}",
                       f"https://d37ci6vzurychx.cloudfront.net/trip-data/yellow_tripdata_2024-{m:02d}.parquet")
                     for m in (1, 2, 3)]},
         lic("NYC Open Data Terms of Use (no restriction on reuse)", True,
             "NYC Taxi & Limousine Commission (TLC), distributed via CloudFront", ""),
         "nyc-tlc",
         "NYC TLC Yellow Taxi trip records for January-March 2024, Apache Parquet, as published (three "
         "monthly shards)."),
    item("f06-validation-noaa-ghcn-daily-parquet-2023", "F06", "validation", "small", "download", "real",
         {"inputs": [dl(el.lower(),
                       f"https://noaa-ghcn-pds.s3.amazonaws.com/parquet/by_year/YEAR=2023/ELEMENT={el}/"
                       f"88c0fdff5eb544adbc2c20dbf589c1c3_0.snappy.parquet", filename=f"{el}.snappy.parquet")
                     for el in ("TMAX", "TMIN", "PRCP", "SNOW", "SNWD", "AWND", "TAVG", "WT01")]},
         lic("Public domain (NOAA GHCN-Daily)", True,
             "NOAA National Centers for Environmental Information, Global Historical Climatology "
             "Network daily (GHCN-Daily), https://registry.opendata.aws/noaa-ghcn/", ""),
         "noaa-ghcn-daily",
         "NOAA GHCN-Daily 2023 observations in Parquet, partitioned by element: eight common elements "
         "(max/min temperature, precipitation, snowfall, snow depth, wind, average temperature, fog flag)."),
    item("f06-validation-noaa-ghcn-daily-csv-2023", "F06", "validation", "large", "download", "real",
         {"inputs": [dl("csv", "https://noaa-ghcn-pds.s3.amazonaws.com/csv.gz/by_year/2023.csv.gz")],
          "steps": [{"op": "decompress", "input": "csv", "dest": "2023.csv", "format": "gz"}]},
         lic("Public domain (NOAA GHCN-Daily)", True,
             "NOAA National Centers for Environmental Information, GHCN-Daily, "
             "https://registry.opendata.aws/noaa-ghcn/", ""),
         "noaa-ghcn-daily",
         "NOAA GHCN-Daily complete daily summaries for 2023, single decompressed CSV (large tier; same "
         "source and independence group as the element-partitioned Parquet item)."),
]


DOC_NOTES = (
    "g3-data corpus group: F05 logs/text, F06 JSON/XML/CSV/structured text, F07 scientific/numeric arrays, F08 databases. "
    "Generated by research/corpus/generators/g3-data/make_sources.py; edit the script, not this file. "
    "Host prerequisites: research/corpus/generators/g3-data/setup_prereqs.sh (osmium-tool). "
    "Substitutions: Unidata NetCDF example URLs return 404 -> NOAA PSL NCEP reanalysis, NASA GISTEMP (NetCDF) and GWOSC (HDF5); "
    "RFC bulk tarballs unavailable over https -> individual RFC texts; OSM small-region XML is converted from a dated Geofabrik PBF; "
    "GHCN-Daily station CSVs (live-updated) -> NOAA Storm Events CSVs (dated file names). "
    "Public benchmark items (Silesia, enwik8, SDRBench, Loghub) are confined to tuning/validation and tagged public-benchmark. "
    "Retrieval date of first download: 2026-09-12 (per-input SHA-256, size and UTC time recorded in research/corpus/pins/<item_id>.json). "
    "Gap closure 2026-09-16 (critique-round1.md G01/G02/G11/G15/G20): added ML model weights and a dataset-shard "
    "parquet (F07: EleutherAI Pythia-160M, Qwen2.5-1.5B/GGUF, MNIST npz, SmolLM2, bert-base-uncased x3 containers, "
    "Phi-3-mini shards, fineweb-edu); database dumps (F08: Wikimedia simplewiki mysqldump, Ensembl core db, "
    "Stack Exchange cs.stackexchange.com loaded into SQLite by build_stackexchange_sqlite.py, Rfam 15.0 dump "
    "trimmed to exclude the two raw-sequence tables); telemetry/binary logs (F05: Google cluster-usage trace, "
    "Wikimedia pageviews, a generated systemd journal via build_journal_export.py + real systemd-journal-remote, "
    "Backblaze Drive Stats, EVTX-ATTACK-SAMPLES, and per-split logrotate-style derives via logrotate_derive.py); "
    "scientific archive formats (F07: SDSS DR17 FITS frames, ann-benchmarks Fashion-MNIST HDF5, a CMIP6 Zarr "
    "store, 1000 Genomes chr22 VCF as BGZF plus a region-subset via vcf_region_subset.py); and columnar formats "
    "at scale (F06: NYC TLC and NOAA GHCN-Daily parquet/CSV). HF resolve URLs are pinned to the exact commit "
    "returned in the x-repo-commit header at retrieval time; HF file sha256 values are the tree API's lfs.oid "
    "(equivalently, the resolve response's x-linked-etag), verified against a downloaded copy for one file. "
    "Docker Desktop's Linux engine was unavailable this session (non-admin), so the Docker-dependent parts of "
    "G01/G02 (pg_dump derive of postgres17-pgbench, Ensembl loaded into MariaDB, employees mysqldump) are not "
    "included; see PROGRESS.md."
)


def declare_from_pins(items):
    changed = 0
    for it in items:
        pins_path = PINS / f"{it['item_id']}.json"
        if not pins_path.is_file():
            continue
        pins = json.loads(pins_path.read_text(encoding="utf-8"))
        for inp in it["recipe"].get("inputs", []) or []:
            p = pins.get("inputs", {}).get(inp["name"])
            if inp["sha256"] == "TOFU" and p and p.get("url") == inp["url"]:
                inp["sha256"] = p["sha256"]
                inp["size"] = p["size"]
                changed += 1
    return changed


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--tofu-only", action="store_true")
    ap.add_argument("--check-only", action="store_true")
    args = ap.parse_args()
    items = json.loads(json.dumps(ITEMS))
    if not args.tofu_only:
        n = declare_from_pins(items)
        print(f"declared {n} input hash(es) from pins", flush=True)
    doc = {"schema": "ebrc-sources-v1", "notes": DOC_NOTES, "items": items}
    text = json.dumps(doc, indent=2, ensure_ascii=False) + "\n"
    if args.check_only:
        print(text)
        return
    SOURCES.parent.mkdir(parents=True, exist_ok=True)
    tmp = SOURCES.with_suffix(".json.tmp")
    with open(tmp, "w", encoding="utf-8", newline="\n") as f:
        f.write(text)
    os.replace(tmp, SOURCES)
    print(f"wrote {SOURCES} ({len(items)} items)")


if __name__ == "__main__":
    main()
