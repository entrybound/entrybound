#!/usr/bin/env python3
"""Load a Stack Exchange data-dump 7z archive into a single SQLite database (F08 run step).

Real content: the Stack Exchange Community Data Dump (archive.org mirror) for one site,
distributed as a 7z archive of flat XML files (Posts.xml, Users.xml, Comments.xml, Votes.xml,
Tags.xml, Badges.xml, PostLinks.xml, PostHistory.xml -- not all sites include every file). This
script extracts the archive, streams each `<row .../>` element with a schema-on-read loader (the
SQLite table's columns are the union of XML attribute names seen for that table, discovered
incrementally with ALTER TABLE ADD COLUMN), and writes one table per XML file into a single
SQLite database. This is a real derive (kind=download + run step), not a generator: no synthetic
content is produced, only a lossless container-format change from XML rows to SQLite rows.

ebrc script contract:
    python build_stackexchange_sqlite.py --out <staging dir> --seed <int> --params <canonical JSON>
params:
  input        declared recipe input name holding the 7z archive (default "archive")
  db_name      output SQLite file name (default "stackexchange.sqlite")
  batch_size   rows per executemany batch (default 20000)

The input path comes from EB_INPUT_<NAME> (uppercased, '-' -> '_'), matching the other g3-data
run-step scripts (see osm_pbf_to_xml.sh). Requires the `7z` CLI (Ubuntu package `7zip`, already
used by provision.py's own 7z extraction path) and Python's stdlib xml.etree + sqlite3 only.
"""

import argparse
import json
import os
import shutil
import sqlite3
import subprocess
import sys
import xml.etree.ElementTree as ET
from pathlib import Path


def env_input_path(name: str) -> Path:
    var = "EB_INPUT_" + name.upper().replace("-", "_")
    p = os.environ.get(var)
    if not p or not Path(p).is_file():
        raise SystemExit(f"input {name!r} not available (env {var} = {p!r})")
    return Path(p)


def extract_7z(archive: Path, dest: Path) -> None:
    dest.mkdir(parents=True, exist_ok=True)
    exe = shutil.which("7z") or shutil.which("7za") or shutil.which("7zr")
    if not exe:
        raise SystemExit("no 7z/7za/7zr CLI found on PATH")
    r = subprocess.run([exe, "x", "-y", f"-o{dest}", str(archive)], stdout=subprocess.PIPE,
                       stderr=subprocess.STDOUT)
    if r.returncode != 0:
        raise SystemExit(f"7z extraction failed (exit {r.returncode}):\n{r.stdout.decode('utf-8', 'replace')}")


# XML attribute values that should be stored as SQLite INTEGER rather than TEXT, keyed by
# (table, attribute).  Left as TEXT (the safe default) for anything not listed here; this is a
# convenience for downstream querying, not required for correctness (SQLite is dynamically typed).
INT_HINT_SUFFIXES = ("Id", "Count", "Score", "ViewCount", "AnswerCount", "CommentCount",
                     "FavoriteCount", "Reputation", "UpVotes", "DownVotes", "PostTypeId",
                     "VoteTypeId", "BountyAmount", "ClosedAsOffTopicReasonTypeId")


def sql_type_for(attr: str) -> str:
    return "INTEGER" if attr.endswith(INT_HINT_SUFFIXES) else "TEXT"


def load_xml_table(conn: sqlite3.Connection, xml_path: Path, table: str, batch_size: int) -> int:
    cur = conn.cursor()
    columns: list[str] = []
    have_table = False
    batch = []
    total = 0

    def ensure_columns(attrib_keys):
        nonlocal have_table, columns
        new_cols = [k for k in attrib_keys if k not in columns]
        if not have_table:
            cols_sql = ", ".join(f'"{c}" {sql_type_for(c)}' for c in attrib_keys) or '"_empty" TEXT'
            cur.execute(f'CREATE TABLE "{table}" ({cols_sql})')
            columns = list(attrib_keys)
            have_table = True
        else:
            for c in new_cols:
                cur.execute(f'ALTER TABLE "{table}" ADD COLUMN "{c}" {sql_type_for(c)}')
                columns.append(c)

    def flush():
        nonlocal batch
        if not batch:
            return
        placeholders = ", ".join(f':{c}' for c in columns)
        col_list = ", ".join(f'"{c}"' for c in columns)
        cur.executemany(f'INSERT INTO "{table}" ({col_list}) VALUES ({placeholders})',
                        [{c: r.get(c) for c in columns} for r in batch])
        batch = []

    context = ET.iterparse(str(xml_path), events=("end",))
    for _event, elem in context:
        if elem.tag != "row":
            continue
        attrib = elem.attrib
        if attrib:
            ensure_columns(attrib.keys())
            batch.append(dict(attrib))
            total += 1
            if len(batch) >= batch_size:
                flush()
        elem.clear()
    flush()
    conn.commit()
    if have_table:
        pk = "Id" if "Id" in columns else None
        if pk:
            cur.execute(f'CREATE INDEX IF NOT EXISTS "idx_{table}_{pk}" ON "{table}"("{pk}")')
    conn.commit()
    return total


TABLE_FILES = {
    "posts": "Posts.xml", "users": "Users.xml", "comments": "Comments.xml", "votes": "Votes.xml",
    "tags": "Tags.xml", "badges": "Badges.xml", "postlinks": "PostLinks.xml",
    "posthistory": "PostHistory.xml", "posttypes": "PostTypes.xml",
}


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True)
    ap.add_argument("--seed", type=int, default=0)
    ap.add_argument("--params", default="{}")
    args = ap.parse_args()
    params = json.loads(args.params)
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    scratch = Path(os.environ.get("EB_SCRATCH", "/tmp")) / "stackexchange-extract"
    if scratch.exists():
        shutil.rmtree(scratch)

    archive = env_input_path(params.get("input", "archive"))
    extract_7z(archive, scratch)

    readme = None
    for cand in ("readme.txt", "Readme.txt", "README.txt"):
        p = scratch / cand
        if p.is_file():
            readme = p
            break
    if readme:
        shutil.copyfile(readme, out / "readme.txt")

    db_path = out / params.get("db_name", "stackexchange.sqlite")
    if db_path.exists():
        db_path.unlink()
    conn = sqlite3.connect(str(db_path))
    conn.execute("PRAGMA page_size=4096")
    conn.execute("PRAGMA journal_mode=DELETE")
    conn.execute("PRAGMA synchronous=OFF")

    summary = {}
    batch_size = int(params.get("batch_size", 20000))
    for table, fname in TABLE_FILES.items():
        xml_path = scratch / fname
        if not xml_path.is_file():
            continue
        n = load_xml_table(conn, xml_path, table, batch_size)
        summary[table] = n
        print(f"loaded {table} <- {fname}: {n} rows", file=sys.stderr, flush=True)

    conn.execute("PRAGMA journal_mode=DELETE")
    conn.execute("VACUUM")
    conn.execute("ANALYZE")
    conn.commit()
    conn.close()

    (out / "load_summary.json").write_text(json.dumps({
        "source_archive": str(archive.name), "tables": summary, "sqlite3_version": sqlite3.sqlite_version,
    }, indent=2, sort_keys=True) + "\n", encoding="utf-8")

    shutil.rmtree(scratch, ignore_errors=True)
    if not summary:
        raise SystemExit("no recognized Stack Exchange XML tables found in the archive")
    return 0


if __name__ == "__main__":
    sys.exit(main())
