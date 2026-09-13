#!/usr/bin/env python3
"""Synthetic structured-text trees: JSON, NDJSON, CSV/TSV and XML (F06, generated).
g3-data generator "structured A/B".

ebrc generator contract:
    python gen_structured_ab.py --out <empty dir> --seed <int> --params <canonical JSON>
Pure function of (script bytes, seed, params).  Stdlib only.

params:
  outputs: list of {"path": rel path, "kind": name, "bytes": target size, ...kind options}
kinds:
  ndjson-orders   one order document per line (nested customer, line items, payments)
  json-array      a single pretty-printed (indent=2) JSON array of product records
  csv-sensors     wide CSV: timestamp, station id, 12 numeric channels, status text
  csv-ledger      CSV with quoted free text, currency amounts, ISO dates (RFC 4180 quoting)
  tsv-genes       TSV annotation-like table with ids, coordinates, attribute strings
  xml-catalog     XML document of catalog records with attributes, namespaces and CDATA
  json-tree       directory of many small pretty JSON API responses (option "files": count)
Generated data: realistic in shape only.
"""

import argparse
import csv
import io
import json
import math
import os
import random
from xml.sax.saxutils import escape, quoteattr

ADJ = "red blue quiet rapid silver ancient hollow bright tiny vast gentle broken golden frozen".split()
NOUN = "river lamp engine garden window signal harbor thread mirror valley rocket anchor lantern".split()
CITIES = ["Lisbon", "Osaka", "Denver", "Nairobi", "Tallinn", "Recife", "Hanoi", "Perth", "Quebec", "Tromsø", "Zürich", "Kraków"]
COUNTRIES = ["PT", "JP", "US", "KE", "EE", "BR", "VN", "AU", "CA", "NO", "CH", "PL"]
CATS = ["tools", "garden", "kitchen", "books", "toys", "audio", "outdoor", "office", "pets", "sports"]


def words(rng, n):
    return " ".join(rng.choice(ADJ) + " " + rng.choice(NOUN) for _ in range(n))


def sentence(rng):
    s = words(rng, rng.randrange(2, 9))
    return s[0].upper() + s[1:] + rng.choice([".", ".", "!", "?", "; see note."])


def date(rng, y0=2019, y1=2026):
    return f"{rng.randrange(y0, y1)}-{rng.randrange(1, 13):02d}-{rng.randrange(1, 29):02d}"


def sized_writer(path, target, make_chunk, header=b"", footer=b""):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    n = len(header)
    with open(path, "wb") as f:
        f.write(header)
        first = True
        while n < target:
            chunk = make_chunk(first)
            first = False
            f.write(chunk)
            n += len(chunk)
        f.write(footer)


def k_ndjson_orders(rng, path, spec):
    oid = [100000]

    def chunk(_):
        lines = []
        for _ in range(200):
            oid[0] += rng.randrange(1, 4)
            items = [{"sku": f"{rng.choice(CATS)[:3].upper()}-{rng.randrange(10000, 99999)}", "qty": rng.randrange(1, 6),
                      "unit_price": round(rng.lognormvariate(3, 1), 2), "title": words(rng, rng.randrange(1, 3))}
                     for _ in range(rng.randrange(1, 8))]
            ci = rng.randrange(len(CITIES))
            doc = {"order_id": oid[0], "created": f"{date(rng, 2024, 2027)}T{rng.randrange(24):02d}:{rng.randrange(60):02d}:{rng.randrange(60):02d}Z",
                   "customer": {"id": rng.randrange(1, 250000), "city": CITIES[ci], "country": COUNTRIES[ci],
                                "segment": rng.choice(["consumer", "smb", "enterprise"])},
                   "items": items, "total": round(sum(i["qty"] * i["unit_price"] for i in items), 2),
                   "payment": {"method": rng.choice(["card", "paypal", "invoice", "voucher"]), "captured": rng.random() < 0.97},
                   "tags": sorted(rng.sample(["gift", "express", "fragile", "b2b", "promo", "repeat"], rng.randrange(0, 3)))}
            if rng.random() < 0.1:
                doc["notes"] = sentence(rng)
            lines.append(json.dumps(doc, ensure_ascii=False, separators=(",", ":")))
        return ("\n".join(lines) + "\n").encode("utf-8")

    sized_writer(path, spec["bytes"], chunk)


def k_json_array(rng, path, spec):
    pid = [0]

    def chunk(first):
        recs = []
        for _ in range(100):
            pid[0] += 1
            recs.append({"id": pid[0], "name": words(rng, 1).title(), "category": rng.choice(CATS),
                         "price": {"amount": round(rng.lognormvariate(3.2, 0.8), 2), "currency": rng.choice(["EUR", "USD", "JPY"])},
                         "dimensions_cm": [round(rng.uniform(1, 120), 1) for _ in range(3)],
                         "in_stock": rng.random() < 0.8, "rating": None if rng.random() < 0.2 else round(rng.uniform(1, 5), 1),
                         "description": " ".join(sentence(rng) for _ in range(rng.randrange(1, 4)))})
        body = ",\n".join(json.dumps(r, ensure_ascii=False, indent=2) for r in recs)
        return (("" if first else ",\n") + body).encode("utf-8")

    sized_writer(path, spec["bytes"], chunk, header=b"[\n", footer=b"\n]\n")


def k_csv_sensors(rng, path, spec):
    stations = [f"ST{rng.randrange(1000, 9999)}" for _ in range(40)]
    state = {s: [rng.gauss(0, 1) for _ in range(12)] for s in stations}
    t = [1735689600]
    hdr = "timestamp,station," + ",".join(f"ch{i:02d}" for i in range(12)) + ",status\n"

    def chunk(_):
        buf = io.StringIO()
        for _ in range(2000):
            t[0] += rng.choice([1, 1, 1, 2, 5])
            s = rng.choice(stations)
            v = state[s]
            for i in range(12):
                v[i] = 0.995 * v[i] + rng.gauss(0, 0.05) + 0.01 * math.sin(t[0] / (3600 * (i + 1)))
            status = "OK" if rng.random() < 0.995 else rng.choice(["SENSOR_FAULT", "CALIBRATING", "LOW_BATTERY"])
            vals = ",".join("" if rng.random() < 0.002 else f"{x * (10 ** (i % 4)):.4f}" for i, x in enumerate(v))
            buf.write(f"{t[0]},{s},{vals},{status}\n")
        return buf.getvalue().encode()

    sized_writer(path, spec["bytes"], chunk, header=hdr.encode())


def k_csv_ledger(rng, path, spec):
    n = [0]

    def chunk(first):
        buf = io.StringIO()
        w = csv.writer(buf, lineterminator="\r\n", quoting=csv.QUOTE_MINIMAL)
        if first:
            w.writerow(["entry_id", "posted_on", "account", "counterparty", "memo", "debit", "credit", "currency", "approved_by"])
        for _ in range(1000):
            n[0] += 1
            amt = f"{rng.lognormvariate(5, 1.5):.2f}"
            debit, credit = (amt, "") if rng.random() < 0.5 else ("", amt)
            memo = sentence(rng)
            if rng.random() < 0.1:
                memo += f' "ref {rng.randrange(10**5)}", split, across\nlines'
            w.writerow([n[0], date(rng), f"{rng.randrange(1000, 9999)}-{rng.randrange(100):02d}", words(rng, 1).title() + " " + rng.choice(["Ltd", "GmbH", "LLC", "S.A.", "K.K."]),
                        memo, debit, credit, rng.choice(["EUR", "USD", "GBP"]), rng.choice(["", "jlee", "mrossi", "akowalski"])])
        return buf.getvalue().encode("utf-8")

    sized_writer(path, spec["bytes"], chunk)


def k_tsv_genes(rng, path, spec):
    g = [0]
    hdr = "#seqid\tsource\ttype\tstart\tend\tscore\tstrand\tphase\tattributes\n"

    def chunk(_):
        lines = []
        for _ in range(1000):
            g[0] += 1
            chrom = f"chr{rng.choice(list(range(1, 23)) + ['X', 'Y'])}"
            start = rng.randrange(1, 240_000_000)
            end = start + rng.randrange(100, 200_000)
            gene = f"GENE{g[0]:06d}"
            lines.append(f"{chrom}\tebrcgen\tgene\t{start}\t{end}\t.\t{rng.choice('+-')}\t.\tID={gene};Name={rng.choice(NOUN).upper()}{rng.randrange(1, 40)};biotype={rng.choice(['protein_coding', 'lncRNA', 'pseudogene'])}")
            pos = start
            for e in range(rng.randrange(1, 12)):
                es = pos + rng.randrange(0, 5000)
                ee = es + rng.randrange(50, 400)
                pos = ee
                lines.append(f"{chrom}\tebrcgen\texon\t{es}\t{ee}\t{rng.random():.3f}\t+\t{rng.randrange(3)}\tParent={gene};exon_number={e + 1}")
        return ("\n".join(lines) + "\n").encode()

    sized_writer(path, spec["bytes"], chunk, header=hdr.encode())


def k_xml_catalog(rng, path, spec):
    rid = [0]
    header = ('<?xml version="1.0" encoding="UTF-8"?>\n'
              '<catalog xmlns="urn:ebrc:catalog:1" xmlns:dc="http://purl.org/dc/elements/1.1/" version="1.4">\n').encode()

    def chunk(_):
        parts = []
        for _ in range(200):
            rid[0] += 1
            ci = rng.randrange(len(CITIES))
            attrs = f'id={quoteattr("R%08d" % rid[0])} status={quoteattr(rng.choice(["active", "retired", "draft"]))}'
            kids = [f"    <dc:title>{escape(words(rng, rng.randrange(1, 4)).title())}</dc:title>",
                    f"    <dc:date>{date(rng, 1950, 2026)}</dc:date>",
                    f"    <location city={quoteattr(CITIES[ci])} country={quoteattr(COUNTRIES[ci])} lat=\"{rng.uniform(-60, 70):.5f}\" lon=\"{rng.uniform(-180, 180):.5f}\"/>"]
            for _ in range(rng.randrange(0, 5)):
                kids.append(f"    <subject scheme=\"lcsh\">{escape(rng.choice(NOUN).title())} -- {escape(rng.choice(ADJ).title())}</subject>")
            if rng.random() < 0.3:
                kids.append(f"    <abstract><![CDATA[{' '.join(sentence(rng) for _ in range(rng.randrange(2, 8)))} <b>&</b>]]></abstract>")
            kids.append(f"    <holdings count=\"{rng.randrange(0, 40)}\" shelf=\"{rng.choice('ABCDEFG')}{rng.randrange(1, 300)}\"/>")
            parts.append(f"  <record {attrs}>\n" + "\n".join(kids) + "\n  </record>\n")
        return "".join(parts).encode("utf-8")

    sized_writer(path, spec["bytes"], chunk, header=header, footer=b"</catalog>\n")


def k_json_tree(rng, path, spec):
    count = int(spec.get("files", 2000))
    for i in range(count):
        kind = rng.choice(["users", "repos", "issues", "invoices"])
        d = os.path.join(path, kind, f"{i % 97:02d}")
        os.makedirs(d, exist_ok=True)
        if kind == "users":
            doc = {"id": i, "login": f"{rng.choice(NOUN)}{rng.randrange(1000)}", "name": words(rng, 1).title(),
                   "created_at": date(rng) + "T00:00:00Z", "followers": rng.randrange(0, 5000), "site_admin": False,
                   "links": {"self": f"https://api.example.test/users/{i}", "repos": f"https://api.example.test/users/{i}/repos"}}
        elif kind == "repos":
            doc = {"id": i, "full_name": f"{rng.choice(NOUN)}/{rng.choice(ADJ)}-{rng.choice(NOUN)}", "private": rng.random() < 0.2,
                   "topics": rng.sample(CATS, rng.randrange(0, 4)), "stars": rng.randrange(0, 100000),
                   "license": rng.choice([None, {"spdx_id": "MIT"}, {"spdx_id": "Apache-2.0"}]), "description": sentence(rng)}
        elif kind == "issues":
            doc = {"number": i, "title": sentence(rng), "state": rng.choice(["open", "closed"]),
                   "labels": [{"name": rng.choice(["bug", "enhancement", "question", "wontfix"]), "color": "%06x" % rng.getrandbits(24)}],
                   "comments": [{"author": rng.choice(NOUN), "body": " ".join(sentence(rng) for _ in range(rng.randrange(1, 6)))}
                                for _ in range(rng.randrange(0, 6))]}
        else:
            lines = [{"desc": words(rng, 2), "qty": rng.randrange(1, 20), "price": round(rng.uniform(1, 500), 2)} for _ in range(rng.randrange(1, 15))]
            doc = {"invoice": f"INV-{i:06d}", "issued": date(rng), "lines": lines,
                   "total": round(sum(x["qty"] * x["price"] for x in lines), 2)}
        with open(os.path.join(d, f"{kind[:-1]}-{i:06d}.json"), "w", encoding="utf-8", newline="\n") as f:
            json.dump(doc, f, ensure_ascii=False, indent=2 if rng.random() < 0.8 else None)
            f.write("\n")


KINDS = {"ndjson-orders": k_ndjson_orders, "json-array": k_json_array, "csv-sensors": k_csv_sensors,
         "csv-ledger": k_csv_ledger, "tsv-genes": k_tsv_genes, "xml-catalog": k_xml_catalog, "json-tree": k_json_tree}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True)
    ap.add_argument("--seed", type=int, required=True)
    ap.add_argument("--params", default="{}")
    args = ap.parse_args()
    p = json.loads(args.params)
    master = random.Random(args.seed)
    for spec in p["outputs"]:
        rng = random.Random(master.getrandbits(64))
        KINDS[spec["kind"]](rng, os.path.join(args.out, spec["path"]), spec)


if __name__ == "__main__":
    main()
