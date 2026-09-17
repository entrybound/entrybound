#!/usr/bin/env python3
"""EXP-CROSSFILE-009 generator: a seeded tree of small, structurally similar JSON documents.

Pre-registered with research/experiments/EXP-CROSSFILE-009/protocol.md. Stdlib only.
Deterministic in (seed, count): identical bytes and names on every host, so it can be packed
on WSL ext4 and on Windows NTFS to exercise the trained-dictionary path of the cross-file
planner (DEC-CMP-021). Names are ASCII; no symlinks, no special files.

usage: make_dictionary_tree.py OUT_DIR [--seed N] [--count N]
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import random

FIELDS = [
    "account_id", "region", "status", "created_at", "updated_at", "plan", "seats", "currency",
    "billing_email", "owner", "tags", "features", "limits", "notes", "last_login", "timezone",
    "events",
]
EVENT_TYPES = ["login", "logout", "invoice.paid", "seat.added", "seat.removed", "export.started",
               "export.finished", "policy.updated"]
REGIONS = ["us-east-1", "us-west-2", "eu-central-1", "ap-southeast-2", "sa-east-1"]
STATUSES = ["active", "suspended", "trial", "closed"]
PLANS = ["free", "team", "business", "enterprise"]
FEATURES = ["sso", "audit-log", "scim", "retention-365", "priority-support", "sandbox", "api"]


def document(rng: random.Random, index: int) -> bytes:
    doc = {
        "account_id": f"acct-{index:07d}",
        "region": rng.choice(REGIONS),
        "status": rng.choice(STATUSES),
        "created_at": f"2025-{rng.randint(1, 12):02d}-{rng.randint(1, 28):02d}T{rng.randint(0, 23):02d}:00:00Z",
        "updated_at": f"2026-{rng.randint(1, 8):02d}-{rng.randint(1, 28):02d}T{rng.randint(0, 23):02d}:30:00Z",
        "plan": rng.choice(PLANS),
        "seats": rng.randint(1, 5000),
        "currency": rng.choice(["USD", "EUR", "AUD", "BRL"]),
        "billing_email": f"billing+{rng.getrandbits(32):08x}@example.org",
        "owner": {"name": f"user{rng.getrandbits(20)}", "role": rng.choice(["admin", "owner"])},
        "tags": sorted(rng.sample(["alpha", "beta", "gamma", "delta", "internal", "partner"], 3)),
        "features": sorted(rng.sample(FEATURES, rng.randint(1, len(FEATURES)))),
        "limits": {k: rng.randint(10, 100000) for k in ("api_calls", "storage_gb", "projects")},
        "notes": " ".join(rng.choice(["renewal", "discount", "migrated", "legacy", "reviewed"]) for _ in range(rng.randint(2, 8))),
        "last_login": f"2026-09-{rng.randint(1, 16):02d}T{rng.randint(0, 23):02d}:{rng.randint(0, 59):02d}:00Z",
        "timezone": rng.choice(["UTC", "Europe/Berlin", "America/New_York", "Australia/Sydney"]),
        "events": [
            {
                "type": rng.choice(EVENT_TYPES),
                "at": f"2026-09-{rng.randint(1, 16):02d}T{rng.randint(0, 23):02d}:{rng.randint(0, 59):02d}:{rng.randint(0, 59):02d}Z",
                "actor": f"user{rng.randint(1, 50)}",
                "source_ip": f"10.{rng.randint(0, 3)}.{rng.randint(0, 255)}.{rng.randint(1, 254)}",
                "result": rng.choice(["ok", "ok", "ok", "denied"]),
            }
            for _ in range(rng.randint(40, 90))
        ],
    }
    return (json.dumps({k: doc[k] for k in FIELDS}, indent=2, sort_keys=False) + "\n").encode("ascii")


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("out_dir")
    ap.add_argument("--seed", type=int, default=20260917091)
    ap.add_argument("--count", type=int, default=4000)
    a = ap.parse_args()
    rng = random.Random(a.seed)
    digest = hashlib.sha256()
    for i in range(a.count):
        sub = os.path.join(a.out_dir, f"shard{i % 16:02d}")
        os.makedirs(sub, exist_ok=True)
        data = document(rng, i)
        name = f"account-{i:07d}.json"
        with open(os.path.join(sub, name), "wb") as fh:
            fh.write(data)
        digest.update(f"shard{i % 16:02d}/{name}\0".encode() + hashlib.sha256(data).digest())
    print(json.dumps({"schema": "ebr.crossfile.generated-tree.v1", "seed": a.seed, "count": a.count,
                      "content_sha256": digest.hexdigest()}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
