#!/usr/bin/env python3
"""TPC-H-like decision-support database in SQLite (F08, generated).  g3-data generator.

Not TPC-H: the schema follows the public TPC-H table layout (region, nation, supplier,
customer, part, partsupp, orders, lineitem) and row counts scale like dbgen, but values come
from this script's own simplified distributions.  Results are not TPC-H results.

ebrc generator contract:
    python gen_sqlite_tpch.py --out <empty dir> --seed <int> --params <canonical JSON>
Pure function of (script bytes, seed, params) for a fixed SQLite library version
(python sqlite3 module of the pinned venv; the version is written into the file header).

params:
  sf           float scale factor (1.0 ~ 6M lineitem rows; default 0.01)
  filename     database file name (default "tpch.sqlite")
  page_size    SQLite page size (default 4096)
  indexes      bool, create secondary indexes (default true)
  analyze      bool, run ANALYZE (default true)
"""

import argparse
import json
import os
import random
import sqlite3

REGIONS = ["AFRICA", "AMERICA", "ASIA", "EUROPE", "MIDDLE EAST"]
NATIONS = [("ALGERIA", 0), ("ARGENTINA", 1), ("BRAZIL", 1), ("CANADA", 1), ("EGYPT", 4), ("ETHIOPIA", 0), ("FRANCE", 3),
           ("GERMANY", 3), ("INDIA", 2), ("INDONESIA", 2), ("IRAN", 4), ("IRAQ", 4), ("JAPAN", 2), ("JORDAN", 4),
           ("KENYA", 0), ("MOROCCO", 0), ("MOZAMBIQUE", 0), ("PERU", 1), ("CHINA", 2), ("ROMANIA", 3),
           ("SAUDI ARABIA", 4), ("VIETNAM", 2), ("RUSSIA", 3), ("UNITED KINGDOM", 3), ("UNITED STATES", 1)]
SEGMENTS = ["AUTOMOBILE", "BUILDING", "FURNITURE", "HOUSEHOLD", "MACHINERY"]
PRIORITIES = ["1-URGENT", "2-HIGH", "3-MEDIUM", "4-NOT SPECIFIED", "5-LOW"]
SHIPMODES = ["REG AIR", "AIR", "RAIL", "SHIP", "TRUCK", "MAIL", "FOB"]
INSTRUCT = ["DELIVER IN PERSON", "COLLECT COD", "NONE", "TAKE BACK RETURN"]
CONTAINERS = [f"{a} {b}" for a in ["SM", "LG", "MED", "JUMBO", "WRAP"] for b in ["CASE", "BOX", "BAG", "JAR", "PKG", "PACK", "CAN", "DRUM"]]
TYPES = [f"{a} {b} {c}" for a in ["STANDARD", "SMALL", "MEDIUM", "LARGE", "ECONOMY", "PROMO"]
         for b in ["ANODIZED", "BURNISHED", "PLATED", "POLISHED", "BRUSHED"] for c in ["TIN", "NICKEL", "BRASS", "STEEL", "COPPER"]]
COLORS = ("almond antique aquamarine azure beige bisque black blanched blue blush brown burlywood burnished chartreuse "
          "chiffon chocolate coral cornflower cornsilk cream cyan dark deep dim dodger drab firebrick floral forest "
          "frosted gainsboro ghost goldenrod green grey honeydew hot indian ivory khaki lace lavender lawn lemon light "
          "lime linen magenta maroon medium metallic midnight mint misty moccasin navajo navy olive orange orchid pale "
          "papaya peach peru pink plum powder puff purple red rose rosy royal saddle salmon sandy seashell sienna sky "
          "slate smoke snow spring steel tan thistle tomato turquoise violet wheat white yellow").split()
TEXT = ("furiously quickly carefully blithely slyly ironic final express regular pending bold special even silent "
        "unusual daring careful ruthless requests deposits packages accounts instructions theodolites pinto beans "
        "foxes ideas dependencies excuses platelets asymptotes courts dolphins multipliers sauternes warthogs frets "
        "dinos attainments somas braids hockey players sleep wake are cajole haggle nag use boost affix detect "
        "integrate maintain nod was lose sublate solve thrash promise engage hinder print x-ray breach eat grow "
        "impress mold poach serve run dazzle snooze doze unwind kindle play hang believe doubt").split()


def comment(rng, lo, hi):
    n = rng.randrange(lo, hi)
    out = []
    size = 0
    while size < n:
        w = rng.choice(TEXT)
        out.append(w)
        size += len(w) + 1
    return " ".join(out)[:n]


def phone(rng, nation):
    return f"{10 + nation}-{rng.randrange(100, 1000)}-{rng.randrange(100, 1000)}-{rng.randrange(1000, 10000)}"


def ymd(day):
    # day offset from 1992-01-01, proleptic calendar via ordinal arithmetic
    import datetime
    return (datetime.date(1992, 1, 1) + datetime.timedelta(days=day)).isoformat()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True)
    ap.add_argument("--seed", type=int, required=True)
    ap.add_argument("--params", default="{}")
    args = ap.parse_args()
    p = json.loads(args.params)
    sf = float(p.get("sf", 0.01))
    rng = random.Random(args.seed)
    path = os.path.join(args.out, p.get("filename", "tpch.sqlite"))
    con = sqlite3.connect(path, isolation_level=None)
    con.execute(f"PRAGMA page_size={int(p.get('page_size', 4096))}")
    con.execute("PRAGMA journal_mode=DELETE")
    con.execute("PRAGMA synchronous=OFF")
    con.executescript("""
    CREATE TABLE region  (r_regionkey INTEGER PRIMARY KEY, r_name TEXT NOT NULL, r_comment TEXT);
    CREATE TABLE nation  (n_nationkey INTEGER PRIMARY KEY, n_name TEXT NOT NULL, n_regionkey INTEGER NOT NULL REFERENCES region, n_comment TEXT);
    CREATE TABLE supplier(s_suppkey INTEGER PRIMARY KEY, s_name TEXT NOT NULL, s_address TEXT NOT NULL, s_nationkey INTEGER NOT NULL REFERENCES nation,
                          s_phone TEXT NOT NULL, s_acctbal NUMERIC NOT NULL, s_comment TEXT);
    CREATE TABLE customer(c_custkey INTEGER PRIMARY KEY, c_name TEXT NOT NULL, c_address TEXT NOT NULL, c_nationkey INTEGER NOT NULL REFERENCES nation,
                          c_phone TEXT NOT NULL, c_acctbal NUMERIC NOT NULL, c_mktsegment TEXT NOT NULL, c_comment TEXT);
    CREATE TABLE part    (p_partkey INTEGER PRIMARY KEY, p_name TEXT NOT NULL, p_mfgr TEXT NOT NULL, p_brand TEXT NOT NULL, p_type TEXT NOT NULL,
                          p_size INTEGER NOT NULL, p_container TEXT NOT NULL, p_retailprice NUMERIC NOT NULL, p_comment TEXT);
    CREATE TABLE partsupp(ps_partkey INTEGER NOT NULL REFERENCES part, ps_suppkey INTEGER NOT NULL REFERENCES supplier, ps_availqty INTEGER NOT NULL,
                          ps_supplycost NUMERIC NOT NULL, ps_comment TEXT, PRIMARY KEY (ps_partkey, ps_suppkey)) WITHOUT ROWID;
    CREATE TABLE orders  (o_orderkey INTEGER PRIMARY KEY, o_custkey INTEGER NOT NULL REFERENCES customer, o_orderstatus TEXT NOT NULL,
                          o_totalprice NUMERIC NOT NULL, o_orderdate TEXT NOT NULL, o_orderpriority TEXT NOT NULL, o_clerk TEXT NOT NULL,
                          o_shippriority INTEGER NOT NULL, o_comment TEXT);
    CREATE TABLE lineitem(l_orderkey INTEGER NOT NULL REFERENCES orders, l_partkey INTEGER NOT NULL, l_suppkey INTEGER NOT NULL,
                          l_linenumber INTEGER NOT NULL, l_quantity NUMERIC NOT NULL, l_extendedprice NUMERIC NOT NULL, l_discount NUMERIC NOT NULL,
                          l_tax NUMERIC NOT NULL, l_returnflag TEXT NOT NULL, l_linestatus TEXT NOT NULL, l_shipdate TEXT NOT NULL,
                          l_commitdate TEXT NOT NULL, l_receiptdate TEXT NOT NULL, l_shipinstruct TEXT NOT NULL, l_shipmode TEXT NOT NULL,
                          l_comment TEXT, PRIMARY KEY (l_orderkey, l_linenumber));
    """)
    n_supp = max(10, int(10000 * sf))
    n_cust = max(150, int(150000 * sf))
    n_part = max(200, int(200000 * sf))
    n_ord = max(1500, int(1500000 * sf))

    con.execute("BEGIN")
    con.executemany("INSERT INTO region VALUES (?,?,?)", [(i, r, comment(rng, 31, 115)) for i, r in enumerate(REGIONS)])
    con.executemany("INSERT INTO nation VALUES (?,?,?,?)", [(i, n, r, comment(rng, 31, 115)) for i, (n, r) in enumerate(NATIONS)])
    con.executemany("INSERT INTO supplier VALUES (?,?,?,?,?,?,?)", (
        (k, f"Supplier#{k:09d}", comment(rng, 10, 40), nk, phone(rng, nk), round(rng.uniform(-999.99, 9999.99), 2), comment(rng, 25, 100))
        for k in range(1, n_supp + 1) for nk in [rng.randrange(25)]))
    con.executemany("INSERT INTO customer VALUES (?,?,?,?,?,?,?,?)", (
        (k, f"Customer#{k:09d}", comment(rng, 10, 40), nk, phone(rng, nk), round(rng.uniform(-999.99, 9999.99), 2),
         rng.choice(SEGMENTS), comment(rng, 29, 116))
        for k in range(1, n_cust + 1) for nk in [rng.randrange(25)]))
    prices = {}

    def parts():
        for k in range(1, n_part + 1):
            m = rng.randrange(1, 6)
            price = round((90000 + ((k // 10) % 20001) + 100 * (k % 1000)) / 100, 2)
            prices[k] = price
            yield (k, " ".join(rng.sample(COLORS, 5)), f"Manufacturer#{m}", f"Brand#{m}{rng.randrange(1, 6)}", rng.choice(TYPES),
                   rng.randrange(1, 51), rng.choice(CONTAINERS), price, comment(rng, 5, 22))
    con.executemany("INSERT INTO part VALUES (?,?,?,?,?,?,?,?,?)", parts())
    con.executemany("INSERT INTO partsupp VALUES (?,?,?,?,?)", (
        (k, (k + i * (n_supp // 4)) % n_supp + 1, rng.randrange(1, 10000), round(rng.uniform(1.0, 1000.0), 2),
         comment(rng, 49, 198))
        for k in range(1, n_part + 1) for i in range(4)))
    con.execute("COMMIT")

    okey = 0
    batch_o, batch_l = [], []
    con.execute("BEGIN")
    for _ in range(n_ord):
        okey += 1 if (okey % 8) < 7 else 25  # sparse keys like dbgen
        cust = rng.randrange(1, n_cust + 1)
        if cust % 3 == 0:
            cust = cust - 1 if cust > 1 else cust + 1
        odate = rng.randrange(0, 2405)
        total = 0.0
        statuses = set()
        for ln in range(1, rng.randrange(2, 8)):
            pk = rng.randrange(1, n_part + 1)
            sk = (pk + rng.randrange(4) * (n_supp // 4)) % n_supp + 1
            qty = rng.randrange(1, 51)
            ext = round(qty * prices[pk], 2)
            disc = rng.randrange(0, 11) / 100
            tax = rng.randrange(0, 9) / 100
            ship = odate + rng.randrange(1, 122)
            commit = odate + rng.randrange(30, 91)
            receipt = ship + rng.randrange(1, 31)
            status = "F" if ship <= 1263 else "O"
            statuses.add(status)
            rflag = (rng.choice("RA") if receipt <= 1263 else "N")
            total += ext * (1 + tax) * (1 - disc)
            batch_l.append((okey, pk, sk, ln, qty, ext, disc, tax, rflag, status, ymd(ship), ymd(commit), ymd(receipt),
                            rng.choice(INSTRUCT), rng.choice(SHIPMODES), comment(rng, 10, 43)))
        ostatus = "F" if statuses == {"F"} else ("O" if statuses == {"O"} else "P")
        batch_o.append((okey, cust, ostatus, round(total, 2), ymd(odate), rng.choice(PRIORITIES), f"Clerk#{rng.randrange(1, max(2, int(1000 * sf)) + 1):09d}",
                        0, comment(rng, 19, 78)))
        if len(batch_o) >= 5000:
            con.executemany("INSERT INTO orders VALUES (?,?,?,?,?,?,?,?,?)", batch_o)
            con.executemany("INSERT INTO lineitem VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)", batch_l)
            batch_o.clear()
            batch_l.clear()
    con.executemany("INSERT INTO orders VALUES (?,?,?,?,?,?,?,?,?)", batch_o)
    con.executemany("INSERT INTO lineitem VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)", batch_l)
    con.execute("COMMIT")
    if p.get("indexes", True):
        con.executescript("""
        CREATE INDEX idx_lineitem_shipdate ON lineitem(l_shipdate);
        CREATE INDEX idx_lineitem_part_supp ON lineitem(l_partkey, l_suppkey);
        CREATE INDEX idx_orders_cust_date ON orders(o_custkey, o_orderdate);
        CREATE INDEX idx_customer_nation ON customer(c_nationkey);
        CREATE INDEX idx_supplier_nation ON supplier(s_nationkey);
        """)
    if p.get("analyze", True):
        con.execute("ANALYZE")
    con.close()
    os.chmod(path, 0o644)


if __name__ == "__main__":
    main()
