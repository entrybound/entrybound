#!/usr/bin/env python3
"""Application-style SQLite databases aged by an OLTP churn workload (F08, generated).
g3-data generator "sqlite churn" -- a different workload model and code path from
gen_sqlite_tpch.py (bulk-loaded analytic tables), used for held-out generated items.

ebrc generator contract:
    python gen_sqlite_churn.py --out <empty dir> --seed <int> --params <canonical JSON>
Pure function of (script bytes, seed, params) for a fixed SQLite library version.
No wall-clock functions are used (timestamps come from a simulated clock).

params:
  databases: list of {
      "file": name, "page_size": int, "auto_vacuum": "none"|"full"|"incremental",
      "journal_mode": "delete"|"wal", "profile": "messaging"|"kvstore"|"telemetry",
      "ops": number of workload operations, "blob_max": max blob bytes (default 16000),
      "fts": bool (messaging only: FTS5 index over message bodies) }

profiles:
  messaging  users, conversations, messages (text + JSON metadata), attachments (blobs that
             spill into overflow pages); inserts dominate, with edits and deletions that
             leave free pages; optional FTS5 table.
  kvstore    a key/value table (WITHOUT ROWID) with versioned values: puts, overwrites of
             varying size, deletes and range scans, like an embedded config/cache store.
  telemetry  append-mostly time-bucketed samples with a retention job deleting old buckets
             and a rollup table updated in place.
WAL databases are checkpointed with TRUNCATE and closed so no -wal/-shm files remain.
"""

import argparse
import json
import os
import random
import sqlite3

FIRST = ["ana", "bruno", "chen", "dara", "emeka", "freya", "gita", "hugo", "ines", "jonas", "kaito", "lena", "mateo", "nia"]
TOPICS = ["deploy", "lunch", "invoice", "roadmap", "bug", "release", "meeting", "travel", "budget", "hiring", "backup"]
VOCAB = ("the a to and of in is it that for on with as this we you be are have at or not but can will just about "
         "please check update ship fix today tomorrow asap thanks ok done later again issue build test merge ticket "
         "customer server latency cache query dashboard alert rollback metric window patch config token").split()


def text(rng, lo, hi):
    return " ".join(rng.choice(VOCAB) for _ in range(rng.randrange(lo, hi)))


def blob(rng, n):
    kind = rng.random()
    if kind < 0.4:   # compressible: repeated structure
        unit = bytes(rng.getrandbits(8) for _ in range(rng.randrange(4, 64)))
        return (unit * (n // len(unit) + 1))[:n]
    if kind < 0.7:   # high entropy (already-compressed media)
        return rng.randbytes(n)
    return (text(rng, 20, 200).encode() * (n // 100 + 1))[:n]


def messaging(rng, con, spec, clock):
    fts = bool(spec.get("fts", False))
    con.executescript("""
    CREATE TABLE users(id INTEGER PRIMARY KEY, handle TEXT UNIQUE NOT NULL, display TEXT, created INTEGER NOT NULL, prefs TEXT);
    CREATE TABLE conversations(id INTEGER PRIMARY KEY, title TEXT, created INTEGER NOT NULL, last_activity INTEGER);
    CREATE TABLE members(conversation INTEGER NOT NULL REFERENCES conversations, user INTEGER NOT NULL REFERENCES users,
                         role TEXT, PRIMARY KEY(conversation, user)) WITHOUT ROWID;
    CREATE TABLE messages(id INTEGER PRIMARY KEY, conversation INTEGER NOT NULL, sender INTEGER NOT NULL, sent INTEGER NOT NULL,
                          body TEXT NOT NULL, edited INTEGER, meta TEXT);
    CREATE INDEX messages_conv_sent ON messages(conversation, sent);
    CREATE TABLE attachments(id INTEGER PRIMARY KEY, message INTEGER NOT NULL REFERENCES messages ON DELETE CASCADE,
                             mime TEXT, name TEXT, data BLOB);
    """)
    if fts:
        con.execute("CREATE VIRTUAL TABLE messages_fts USING fts5(body, content='messages', content_rowid='id')")
    blob_max = int(spec.get("blob_max", 16000))
    users = []
    for i in range(200):
        h = f"{rng.choice(FIRST)}.{i}"
        con.execute("INSERT INTO users(handle, display, created, prefs) VALUES (?,?,?,?)",
                    (h, h.title(), clock[0], json.dumps({"theme": rng.choice(["dark", "light"]), "notify": rng.random() < 0.7})))
        users.append(con.execute("SELECT last_insert_rowid()").fetchone()[0])
    convs = []
    msg_ids = []
    con.execute("BEGIN")
    for op in range(int(spec["ops"])):
        clock[0] += rng.randrange(1, 90)
        r = rng.random()
        if r < 0.02 or not convs:
            cur = con.execute("INSERT INTO conversations(title, created, last_activity) VALUES (?,?,?)",
                              (f"#{rng.choice(TOPICS)}-{op}", clock[0], clock[0]))
            cid = cur.lastrowid
            convs.append(cid)
            for u in rng.sample(users, rng.randrange(2, 12)):
                con.execute("INSERT OR IGNORE INTO members VALUES (?,?,?)", (cid, u, rng.choice(["member", "member", "admin"])))
        elif r < 0.80:
            cid = convs[min(len(convs) - 1, int(rng.paretovariate(1.2)) - 1)] if rng.random() < 0.7 else rng.choice(convs)
            body = text(rng, 1, 60)
            meta = json.dumps({"client": rng.choice(["ios", "android", "web", "desktop"]), "reply_to": rng.choice(msg_ids) if msg_ids and rng.random() < 0.2 else None})
            cur = con.execute("INSERT INTO messages(conversation, sender, sent, body, meta) VALUES (?,?,?,?,?)",
                              (cid, rng.choice(users), clock[0], body, meta))
            mid = cur.lastrowid
            msg_ids.append(mid)
            if fts:
                con.execute("INSERT INTO messages_fts(rowid, body) VALUES (?,?)", (mid, body))
            con.execute("UPDATE conversations SET last_activity=? WHERE id=?", (clock[0], cid))
            if rng.random() < 0.08:
                n = int(min(blob_max, rng.lognormvariate(8, 1.2)))
                con.execute("INSERT INTO attachments(message, mime, name, data) VALUES (?,?,?,?)",
                            (mid, rng.choice(["image/jpeg", "application/pdf", "text/plain"]), f"file{op}.bin", blob(rng, max(1, n))))
        elif r < 0.92 and msg_ids:
            mid = rng.choice(msg_ids)
            row = con.execute("SELECT body FROM messages WHERE id=?", (mid,)).fetchone()
            if row:
                new = row[0] + " " + text(rng, 1, 30)
                if fts:
                    con.execute("INSERT INTO messages_fts(messages_fts, rowid, body) VALUES ('delete', ?, ?)", (mid, row[0]))
                    con.execute("INSERT INTO messages_fts(rowid, body) VALUES (?,?)", (mid, new))
                con.execute("UPDATE messages SET body=?, edited=? WHERE id=?", (new, clock[0], mid))
        elif msg_ids:
            # retention: delete a run of old messages and their attachments
            start = rng.randrange(len(msg_ids))
            victims = msg_ids[start:start + rng.randrange(1, 40)]
            for mid in victims:
                row = con.execute("SELECT body FROM messages WHERE id=?", (mid,)).fetchone()
                if row and fts:
                    con.execute("INSERT INTO messages_fts(messages_fts, rowid, body) VALUES ('delete', ?, ?)", (mid, row[0]))
                con.execute("DELETE FROM attachments WHERE message=?", (mid,))
                con.execute("DELETE FROM messages WHERE id=?", (mid,))
            del msg_ids[start:start + len(victims)]
        if op % 5000 == 4999:
            con.execute("COMMIT")
            con.execute("BEGIN")
    con.execute("COMMIT")


def kvstore(rng, con, spec, clock):
    con.executescript("""
    CREATE TABLE kv(k BLOB PRIMARY KEY, v BLOB NOT NULL, version INTEGER NOT NULL, mtime INTEGER NOT NULL) WITHOUT ROWID;
    CREATE TABLE meta(name TEXT PRIMARY KEY, value TEXT);
    """)
    blob_max = int(spec.get("blob_max", 16000))
    keys = []
    con.execute("BEGIN")
    for op in range(int(spec["ops"])):
        clock[0] += 1
        r = rng.random()
        if r < 0.45 or not keys:
            k = f"{rng.choice(['cfg', 'sess', 'cache', 'lock', 'blob'])}/{rng.getrandbits(40):010x}".encode()
            size = int(min(blob_max, rng.lognormvariate(5, 2)))
            con.execute("INSERT OR REPLACE INTO kv VALUES (?,?,1,?)", (k, blob(rng, max(1, size)), clock[0]))
            keys.append(k)
        elif r < 0.80:
            k = keys[int(rng.random() ** 3 * len(keys))]
            size = int(min(blob_max, rng.lognormvariate(5, 2)))
            con.execute("UPDATE kv SET v=?, version=version+1, mtime=? WHERE k=?", (blob(rng, max(1, size)), clock[0], k))
        elif r < 0.95:
            i = rng.randrange(len(keys))
            con.execute("DELETE FROM kv WHERE k=?", (keys[i],))
            keys[i] = keys[-1]
            keys.pop()
        else:
            prefix = rng.choice([b"cfg/", b"sess/", b"cache/"])
            con.execute("SELECT count(*), sum(length(v)) FROM kv WHERE k >= ? AND k < ?", (prefix, prefix[:-1] + b"0")).fetchone()
        if op % 10000 == 9999:
            con.execute("INSERT OR REPLACE INTO meta VALUES ('last_compaction_op', ?)", (str(op),))
            con.execute("COMMIT")
            if spec.get("auto_vacuum") == "incremental":
                con.execute(f"PRAGMA incremental_vacuum({rng.randrange(10, 200)})")
            con.execute("BEGIN")
    con.execute("COMMIT")


def telemetry(rng, con, spec, clock):
    con.executescript("""
    CREATE TABLE series(id INTEGER PRIMARY KEY, name TEXT UNIQUE NOT NULL, unit TEXT, labels TEXT);
    CREATE TABLE samples(series INTEGER NOT NULL, bucket INTEGER NOT NULL, ts INTEGER NOT NULL, value REAL NOT NULL);
    CREATE INDEX samples_series_ts ON samples(series, ts);
    CREATE INDEX samples_bucket ON samples(bucket);
    CREATE TABLE rollup(series INTEGER NOT NULL, bucket INTEGER NOT NULL, n INTEGER, min REAL, max REAL, sum REAL,
                        PRIMARY KEY(series, bucket));
    """)
    ids = []
    for i in range(int(spec.get("series", 300))):
        cur = con.execute("INSERT INTO series(name, unit, labels) VALUES (?,?,?)",
                          (f"host{i % 40:02d}.{rng.choice(['cpu', 'mem', 'disk', 'net', 'temp'])}.{i}", rng.choice(["%", "bytes", "C", "pkts/s"]),
                           json.dumps({"dc": rng.choice(["fra", "iad", "sin"]), "rack": rng.randrange(40)})))
        ids.append(cur.lastrowid)
    level = {s: rng.uniform(0, 100) for s in ids}
    con.execute("BEGIN")
    bucket_len = 3600
    for op in range(int(spec["ops"])):
        clock[0] += rng.randrange(1, 4)
        s = rng.choice(ids)
        level[s] = max(0.0, level[s] + rng.gauss(0, 1.5))
        b = clock[0] // bucket_len
        con.execute("INSERT INTO samples VALUES (?,?,?,?)", (s, b, clock[0], round(level[s], 3)))
        con.execute("""INSERT INTO rollup VALUES (?,?,1,?,?,?)
                       ON CONFLICT(series, bucket) DO UPDATE SET n=n+1, min=min(min, excluded.min), max=max(max, excluded.max), sum=sum+excluded.sum""",
                    (s, b, level[s], level[s], level[s]))
        if op % 20000 == 19999:
            con.execute("DELETE FROM samples WHERE bucket < ?", (b - 24,))
            con.execute("COMMIT")
            con.execute("BEGIN")
    con.execute("COMMIT")


PROFILES = {"messaging": messaging, "kvstore": kvstore, "telemetry": telemetry}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True)
    ap.add_argument("--seed", type=int, required=True)
    ap.add_argument("--params", default="{}")
    args = ap.parse_args()
    p = json.loads(args.params)
    master = random.Random(f"churn:{args.seed}")
    for spec in p["databases"]:
        rng = random.Random(master.getrandbits(64))
        path = os.path.join(args.out, spec["file"])
        os.makedirs(os.path.dirname(path) or args.out, exist_ok=True)
        con = sqlite3.connect(path, isolation_level=None)
        con.execute(f"PRAGMA page_size={int(spec.get('page_size', 4096))}")
        con.execute(f"PRAGMA auto_vacuum={spec.get('auto_vacuum', 'none').upper()}")
        con.execute("PRAGMA foreign_keys=ON")
        mode = spec.get("journal_mode", "delete")
        con.execute(f"PRAGMA journal_mode={mode.upper()}")
        con.execute("PRAGMA synchronous=OFF")
        clock = [1767225600 + rng.randrange(0, 86400 * 30)]
        PROFILES[spec["profile"]](rng, con, spec, clock)
        if mode == "wal":
            con.execute("PRAGMA wal_checkpoint(TRUNCATE)")
        con.close()
        for suffix in ("-wal", "-shm", "-journal"):
            if os.path.exists(path + suffix):
                os.unlink(path + suffix)
        os.chmod(path, 0o644)


if __name__ == "__main__":
    main()
